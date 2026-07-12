//! Canonical backend flow: constrain → generate → place → route → verify.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

use gdsverify::{
    compare, extract_netlist_opts, run_drc, run_pex_by_net_checked, Backend, Bbox, CompareOpts,
    Deck, DeviceFlavor, DeviceKind, DrcReport, ExtractOpts, GeometryStore, LayerId, LvsResult,
    PexReport, PolyId, RefDevice, RefNetlist, Violation,
};
use pnr_cells::device::DeviceRecord;
use pnr_cells::generators::{
    bjt::BjtSpec,
    candidates,
    capacitor::CapacitorSpec,
    diode::DiodeSpec,
    guard_ring::{draw_guard_ring, guard_ring_outer_bbox, ring_width_from_depth, RingType},
    inductor::InductorSpec,
    mosfet::MosfetSpec,
    resistor::ResistorSpec,
};
use pnr_cells::netlist::BipartiteHypergraph;
use pnr_cells::{
    snap_to_grid, CellBuilder, CellContext, CellOutput, DeviceType, MatchingTier, Orientation,
};
use pnr_constraints::{
    ConstraintContract, ConstraintRecord, ConstraintStatus, Contractable, DeviceId,
};
use pnr_engine::block::IterationSummary;
use pnr_placement::{run_placement, PlacementConfig, PlacementResult};
use pnr_routing::{
    extract_feedback_with_prior, run_routing_at, wire_rect, RoutingConfig, RoutingResult,
};

use crate::pdk::{CellBase, Pdk};
use crate::FlowInput;

// ═══════════════════════════════════════════════════════════════════════
//  Config / result
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct FlowConfig {
    pub placement: PlacementConfig,
    pub routing: RoutingConfig,
    /// Cut square in nm. Zero selects the PDK contact size.
    pub via_size: i32,
    /// Landing-pad width in nm. Zero derives contact + metal enclosure.
    pub pad: i32,
    /// Local-interconnect strap width in nm. Zero selects the PDK contact size.
    pub li_width: i32,
    pub debug_dir: Option<PathBuf>,
    pub max_feedback_iters: u32,
    pub feedback_weight_cap: f64,
    pub feedback_threshold: f64,
    pub feedback_inflation_cap: f64,
    /// Top-level nets that are actual package/pad connections. Subcircuit
    /// ports are not pads by default; conflating them creates private guard
    /// rings around every device in ordinary reusable blocks.
    pub pad_nets: Vec<String>,
    /// Electrical/reliability tapeout inputs. Checks without required inputs
    /// report NOT_RUN; they are never silently treated as clean.
    pub signoff_checks: gdsverify::SignoffConfig,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            placement: PlacementConfig::default(),
            routing: RoutingConfig::default(),
            via_size: 0,
            pad: 0,
            li_width: 0,
            debug_dir: None,
            max_feedback_iters: 100,
            feedback_weight_cap: 5.0,
            feedback_threshold: 1.2,
            feedback_inflation_cap: 1.5,
            pad_nets: Vec::new(),
            signoff_checks: gdsverify::SignoffConfig::default(),
        }
    }
}

pub struct SignoffReport {
    pub drc: DrcReport,
    pub drc_blocking: Vec<Violation>,
    pub drc_waived_density: usize,
    pub lvs: LvsResult,
    pub pex: PexReport,
    pub contracts: Vec<ConstraintContract>,
    pub advanced: gdsverify::SignoffSuiteReport,
}

impl SignoffReport {
    /// Aggregate the checks that have an explicit pass/fail policy. PEX values
    /// are gated through parasitic-budget contracts when those are supplied.
    /// Missing advanced-analysis inputs remain blocking through `all_clean()`.
    #[must_use]
    pub fn all_required_checks_clean(&self) -> bool {
        self.drc_blocking.is_empty()
            && self.lvs.matched
            && self.pex.is_complete()
            && self.advanced.all_clean()
            && self.contracts.iter().all(|c| {
                matches!(
                    c.status(),
                    ConstraintStatus::Satisfied | ConstraintStatus::Waived
                )
            })
    }
}

#[cfg(test)]
mod signoff_policy_tests {
    use super::*;
    use gdsverify::{
        AntennaReport, CheckReport, CheckStatus, DensityCmpReport, ElectromigrationReport,
        EsdLatchupReport, IrDropReport, ReliabilityReport, SignoffCheck, SignoffSuiteReport,
    };

    fn suite(antenna_status: CheckStatus) -> SignoffSuiteReport {
        let report = |check, status| match status {
            CheckStatus::Clean => CheckReport::clean(check),
            CheckStatus::NotRun => CheckReport::not_run(check, "missing evidence"),
            CheckStatus::Error => CheckReport::error(check, "invalid evidence"),
            CheckStatus::Violations => unreachable!("not needed by this policy regression"),
        };
        SignoffSuiteReport {
            antenna: AntennaReport {
                check: report(SignoffCheck::Antenna, antenna_status),
                nets: Vec::new(),
            },
            density_cmp: DensityCmpReport {
                check: CheckReport::clean(SignoffCheck::DensityCmp),
                windows: Vec::new(),
            },
            ir_drop: IrDropReport {
                check: CheckReport::clean(SignoffCheck::IrDrop),
                nodes: Vec::new(),
                branches: Vec::new(),
            },
            electromigration: ElectromigrationReport {
                check: CheckReport::clean(SignoffCheck::Electromigration),
                branches: Vec::new(),
            },
            reliability: ReliabilityReport {
                check: CheckReport::clean(SignoffCheck::Reliability),
                aging: Vec::new(),
            },
            esd_latchup: EsdLatchupReport {
                check: CheckReport::clean(SignoffCheck::EsdLatchup),
                paths: Vec::new(),
            },
        }
    }

    fn report(advanced: SignoffSuiteReport, parasitics: Vec<gdsverify::Parasitic>) -> SignoffReport {
        SignoffReport {
            drc: DrcReport {
                violations: Vec::new(),
            },
            drc_blocking: Vec::new(),
            drc_waived_density: 0,
            lvs: LvsResult {
                matched: true,
                reason: "match".into(),
                mismatches: Vec::new(),
                extracted_devices: 0,
                nmos: 0,
                pmos: 0,
                ambiguous_classes: 0,
                label_conflicts: Vec::new(),
                floating_nets: Vec::new(),
            },
            pex: PexReport { parasitics },
            contracts: Vec::new(),
            advanced,
        }
    }

    #[test]
    fn missing_advanced_evidence_and_pex_diagnostics_block_aggregate_clean() {
        assert!(report(suite(CheckStatus::Clean), Vec::new()).all_required_checks_clean());
        assert!(!report(suite(CheckStatus::NotRun), Vec::new()).all_required_checks_clean());
        assert!(!report(
            suite(CheckStatus::Clean),
            vec![gdsverify::Parasitic::ExtractionDiagnostic {
                layer: "met1".into(),
                polygon: 7,
                model: "sheet_resistance".into(),
                message: "unsupported geometry".into(),
            }],
        )
        .all_required_checks_clean());
    }

    #[test]
    fn final_drc_policy_never_waives_density() {
        let drc = DrcReport {
            violations: vec![Violation {
                rule_id: "M1.DENS.MIN".into(),
                kind: "min_density".into(),
                layer: "met1".into(),
                measured: 100_000,
                limit: 300_000,
                x: 0,
                y: 0,
            }],
        };
        let (blocking, waived) = final_drc_policy(&drc);
        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].rule_id, "M1.DENS.MIN");
        assert_eq!(waived, 0);
    }

    #[test]
    fn signoff_sidecar_is_valid_json_for_arbitrary_stable_ids() {
        let mut report = report(suite(CheckStatus::Clean), Vec::new());
        report.drc_blocking.push(Violation {
            rule_id: "deck/\"M1.W\"".into(),
            kind: "min_width".into(),
            layer: "met1".into(),
            measured: 90,
            limit: 100,
            x: 1,
            y: 2,
        });
        let json = crate::gds::signoff_json(&report);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid sidecar JSON");
        assert_eq!(parsed["drc_violations"][0]["rule"], "deck/\"M1.W\"");
        assert_eq!(parsed["all_required_checks_clean"], false);
    }
}

pub struct FlowResult {
    pub graph: BipartiteHypergraph,
    pub placement: PlacementResult,
    pub routing: RoutingResult,
    pub store: GeometryStore,
    pub signoff: SignoffReport,
    pub gds_path: Option<PathBuf>,
    /// Number of feedback iterations executed and the zero-based iteration
    /// whose owned placement/routing candidate was returned.
    pub iterations: u32,
    pub best_iteration: u32,
    pub converged: bool,
    pub feedback_trace: Vec<IterationSummary>,
}

impl fmt::Display for FlowResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "feedback: best iteration {}/{} | {}",
            self.best_iteration + 1,
            self.iterations,
            if self.converged {
                "converged"
            } else {
                "budget exhausted"
            },
        )?;
        write!(f, "{}", self.placement.report)?;
        write!(f, "{}", self.routing.report)?;
        let s = &self.signoff;
        writeln!(
            f,
            "signoff: DRC {} blocking ({} density waived) | LVS {} ({} N / {} P) | PEX {} R(met1) {:.1} ohm, C {:.1} fF",
            s.drc_blocking.len(), s.drc_waived_density,
            if s.lvs.matched { "MATCH" } else { "MISMATCH" },
            s.lvs.nmos, s.lvs.pmos,
            if s.pex.is_complete() { "COMPLETE" } else { "INCOMPLETE" },
            s.pex.total_resistance("met1"), s.pex.total_cap() / 1000.0,
        )?;
        let advanced_clean = s.advanced.checks().iter().filter(|r| r.is_clean()).count();
        writeln!(f, "  advanced signoff: {advanced_clean}/6 checks clean")?;
        writeln!(
            f,
            "  aggregate tapeout policy: {}",
            if s.all_required_checks_clean() {
                "CLEAN"
            } else {
                "BLOCKED"
            }
        )?;
        if !s.lvs.matched {
            writeln!(f, "  LVS: {}", s.lvs.reason)?;
        }
        for v in s.drc_blocking.iter().take(10) {
            writeln!(
                f,
                "  DRC {}: {} measured {} < {} at ({}, {})",
                v.rule_id, v.kind, v.measured, v.limit, v.x, v.y
            )?;
        }
        for c in &s.contracts {
            writeln!(
                f,
                "  budget {}: {:?}{}",
                c.constraint_id,
                c.status(),
                c.violation_metric().map_or(String::new(), |m| format!(
                    " ({m:.2} {})",
                    c.violation_units().unwrap_or("")
                ))
            )?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Cell generation helpers
// ═══════════════════════════════════════════════════════════════════════

struct GenCell {
    variants: Vec<CellOutput>,
    device_type: Option<DeviceType>,
    guard_rings: Vec<pnr_constraints::GuardRingRequirement>,
}

fn cell_tier(rec: &ConstraintRecord, cell_idx: u32) -> MatchingTier {
    let mut best = MatchingTier::None;
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            if mp.device_a.0 == cell_idx || mp.device_b.0 == cell_idx {
                let t = match mp.tier {
                    pnr_constraints::MatchingTier::None => MatchingTier::None,
                    pnr_constraints::MatchingTier::Minimal => MatchingTier::Minimal,
                    pnr_constraints::MatchingTier::Moderate => MatchingTier::Moderate,
                    pnr_constraints::MatchingTier::Exceptional => MatchingTier::Exceptional,
                };
                if t > best {
                    best = t;
                }
            }
        }
    }
    best
}

fn generate_cells(
    g: &BipartiteHypergraph,
    pdk: &Pdk,
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
    rec: &ConstraintRecord,
) -> Result<Vec<GenCell>, String> {
    let mut cells = Vec::with_capacity(g.cells.len());
    for (i, c) in g.cells.iter().enumerate() {
        let devices: Vec<&DeviceRecord> = if !c.grouped_devices.is_empty() {
            c.grouped_devices.iter().collect()
        } else if let Some(dev) = &c.device {
            vec![dev]
        } else {
            return Err(format!(
                "cell `{}` is a macro; macro geometry not wired yet",
                c.name
            ));
        };
        let ref_dev = devices[0];
        let def = pdk
            .device(&c.model)
            .ok_or_else(|| format!("no PDK device def for model `{}`", c.model))?;
        let tier = cell_tier(rec, i as u32);
        let owned: Vec<DeviceRecord> = devices.iter().map(|d| (*d).clone()).collect();
        let gens = match def.cell {
            CellBase::Mosfet => candidates::<MosfetSpec>(&owned, cell_pdk),
            CellBase::Resistor => candidates::<ResistorSpec>(&owned, cell_pdk),
            CellBase::Capacitor => candidates::<CapacitorSpec>(&owned, cell_pdk),
            CellBase::Diode => candidates::<DiodeSpec>(&owned, cell_pdk),
            CellBase::Bjt => candidates::<BjtSpec>(&owned, cell_pdk),
            CellBase::Inductor => candidates::<InductorSpec>(&owned, cell_pdk),
        };
        if gens.is_empty() {
            return Err(format!("no feasible variants for `{}`", c.name));
        }
        let mut variants = Vec::with_capacity(gens.len());
        for gen in &gens {
            let mut b = CellBuilder::with_context(CellContext::new(deck, cell_pdk), tier, i as u32);
            gen.generate(&mut b)
                .map_err(|e| format!("generate `{}`: {e:?}", c.name))?;
            variants.push(b.finish());
        }
        let guard_rings = rec
            .guard_ring
            .iter()
            .filter(|r| r.device_id.0 == i as u32)
            .cloned()
            .collect();
        cells.push(GenCell {
            variants,
            device_type: Some(ref_dev.device_type),
            guard_rings,
        });
    }
    Ok(cells)
}

fn pin_accesses(out: &CellOutput, cell_name: &str, pin_name: &str) -> Vec<(LayerId, Bbox)> {
    let want = format!("{cell_name}:{pin_name}");
    // Grouped cells: pin_name is "XM1.S" but cell generator names it "XM1:S"
    let alt = pin_name.replacen('.', ":", 1);
    let mut hits: Vec<(LayerId, Bbox)> = out
        .pins
        .iter()
        .filter(|p| p.name == want || p.name == alt)
        .map(|p| {
            (
                p.layer,
                Bbox {
                    xmin: p.x,
                    ymin: p.y,
                    xmax: p.x + p.w,
                    ymax: p.y + p.h,
                },
            )
        })
        .collect();
    hits.sort_by_key(|(_, b)| (b.xmin, b.ymin));
    hits.dedup_by_key(|(_, b)| (b.xmin, b.ymin));
    hits
}

// ═══════════════════════════════════════════════════════════════════════
//  Geometry helpers
// ═══════════════════════════════════════════════════════════════════════

fn translate_into(
    dst: &mut GeometryStore,
    src: &GeometryStore,
    dx: i32,
    dy: i32,
    orient: Orientation,
    bbox: &Bbox,
) {
    for p in 0..src.poly_count() as u32 {
        let (start, end) = src.poly_range(PolyId(p));
        let pts: Vec<(i32, i32)> = (0..end - start)
            .map(|i| {
                let (x, y) = src.poly_vertex(start, i);
                let (x, y) =
                    orient.transform(x - bbox.xmin, y - bbox.ymin, bbox.width(), bbox.height());
                (x + bbox.xmin + dx, y + bbox.ymin + dy)
            })
            .collect();
        dst.add_polygon(src.poly_layer[p as usize], &pts);
    }
}

struct GeometryCoalesceReport {
    input_polygons: usize,
    output_polygons: usize,
    merged_components: usize,
    removed_overlap_area: i64,
    old_to_new: Vec<PolyId>,
}

impl GeometryCoalesceReport {
    fn remap(&self, old: PolyId) -> PolyId {
        self.old_to_new.get(old.0 as usize).copied().unwrap_or(old)
    }
}

/// Normalize generated masks before verification. This is deliberately a
/// frontend data transform: verification remains a read-only consumer.
/// Device layers get exact single-contour unions when the union has no hole;
/// PEX layers retain rectangular decomposition but collapse duplicates and
/// any pair whose union is itself a rectangle.
fn coalesce_geometry(store: &mut GeometryStore, deck: &Deck) -> GeometryCoalesceReport {
    let old = std::mem::take(store);
    let input_polygons = old.poly_count();
    let mut out = GeometryStore::new();
    let mut old_to_new = vec![PolyId(u32::MAX); input_polygons];
    let mut merged_components = 0usize;
    let mut removed_overlap_area = 0i64;
    let layers: std::collections::BTreeSet<LayerId> = old.poly_layer.iter().copied().collect();

    for layer in layers {
        let mut rects = Vec::new();
        for p in old.polys_on_layer(layer) {
            if is_axis_aligned_rect(&old, p) {
                rects.push(p);
            } else {
                let q = copy_geometry_polygon(&old, &mut out, p);
                old_to_new[p.0 as usize] = q;
            }
        }
        rects.sort_unstable_by_key(|p| {
            let b = old.poly_bbox[p.0 as usize];
            (b.xmin, b.ymin, b.xmax, b.ymax)
        });
        let mut parent: Vec<usize> = (0..rects.len()).collect();
        let mut rank = vec![0u8; rects.len()];
        for i in 0..rects.len() {
            let a = old.poly_bbox[rects[i].0 as usize];
            for j in i + 1..rects.len() {
                let b = old.poly_bbox[rects[j].0 as usize];
                if b.xmin > a.xmax {
                    break;
                }
                if a.ymin <= b.ymax && b.ymin <= a.ymax {
                    union_indices_local(&mut parent, &mut rank, i, j);
                }
            }
        }
        let mut components: std::collections::BTreeMap<usize, Vec<PolyId>> =
            std::collections::BTreeMap::new();
        for (i, &p) in rects.iter().enumerate() {
            let root = find_index_local(&mut parent, i);
            components.entry(root).or_default().push(p);
        }

        for component in components.into_values() {
            let mut label: Option<String> = None;
            let mut conflict = false;
            for p in &component {
                if let Some(l) = old.net_labels.get(&p.0) {
                    match &label {
                        Some(existing) if existing != l => conflict = true,
                        None => label = Some(l.clone()),
                        _ => {}
                    }
                }
            }
            if conflict {
                for p in component {
                    let q = copy_geometry_polygon(&old, &mut out, p);
                    old_to_new[p.0 as usize] = q;
                }
                continue;
            }

            let boxes: Vec<Bbox> = component
                .iter()
                .map(|p| old.poly_bbox[p.0 as usize])
                .collect();
            let source_area: i64 = boxes
                .iter()
                .map(|b| i64::from(b.width()) * i64::from(b.height()))
                .sum();
            let is_pex_layer = deck.pex.contains_key(&layer);
            let contour = if is_pex_layer {
                None
            } else {
                rectangle_union_contour(&boxes)
            };
            let mut produced = Vec::new();
            let union_area;
            if let Some(points) = contour {
                let q = out.add_polygon(layer, &points);
                if let Some(l) = &label {
                    out.net_labels.insert(q.0, l.clone());
                }
                union_area = out.area(q);
                produced.push(q);
            } else {
                let boxes = coalesce_rectangles(boxes);
                union_area = boxes
                    .iter()
                    .map(|b| i64::from(b.width()) * i64::from(b.height()))
                    .sum();
                for b in boxes {
                    let q = out.add_rect(layer, b.xmin, b.ymin, b.width(), b.height());
                    if let Some(l) = &label {
                        out.net_labels.insert(q.0, l.clone());
                    }
                    produced.push(q);
                }
            }
            let representative = produced[0];
            for p in &component {
                old_to_new[p.0 as usize] = representative;
            }
            let removed = (source_area - union_area).max(0);
            removed_overlap_area += removed;
            if produced.len() < component.len() || removed > 0 {
                merged_components += 1;
            }
        }
    }

    out.text_x = old.text_x;
    out.text_y = old.text_y;
    out.text_layer = old.text_layer;
    out.text_datatype = old.text_datatype;
    out.text_string = old.text_string;
    let output_polygons = out.poly_count();
    *store = out;
    GeometryCoalesceReport {
        input_polygons,
        output_polygons,
        merged_components,
        removed_overlap_area,
        old_to_new,
    }
}

fn copy_geometry_polygon(src: &GeometryStore, dst: &mut GeometryStore, p: PolyId) -> PolyId {
    let (s, e) = src.poly_range(p);
    let points: Vec<(i32, i32)> = (0..e - s).map(|i| src.poly_vertex(s, i)).collect();
    let q = dst.add_polygon(src.poly_layer[p.0 as usize], &points);
    if let Some(label) = src.net_labels.get(&p.0) {
        dst.net_labels.insert(q.0, label.clone());
    }
    q
}

fn is_axis_aligned_rect(store: &GeometryStore, p: PolyId) -> bool {
    let (s, e) = store.poly_range(p);
    if e - s != 4 {
        return false;
    }
    let b = store.poly_bbox[p.0 as usize];
    b.width() > 0
        && b.height() > 0
        && (0..4).all(|i| {
            let (x, y) = store.poly_vertex(s, i);
            (x == b.xmin || x == b.xmax) && (y == b.ymin || y == b.ymax)
        })
        && store.area(p) == i64::from(b.width()) * i64::from(b.height())
}

fn coalesce_rectangles(mut boxes: Vec<Bbox>) -> Vec<Bbox> {
    loop {
        let mut merged = false;
        'pairs: for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let bound = Bbox {
                    xmin: a.xmin.min(b.xmin),
                    ymin: a.ymin.min(b.ymin),
                    xmax: a.xmax.max(b.xmax),
                    ymax: a.ymax.max(b.ymax),
                };
                let ox = (a.xmax.min(b.xmax) - a.xmin.max(b.xmin)).max(0);
                let oy = (a.ymax.min(b.ymax) - a.ymin.max(b.ymin)).max(0);
                let union_area = i64::from(a.width()) * i64::from(a.height())
                    + i64::from(b.width()) * i64::from(b.height())
                    - i64::from(ox) * i64::from(oy);
                let bound_area = i64::from(bound.width()) * i64::from(bound.height());
                if union_area == bound_area {
                    boxes[i] = bound;
                    boxes.swap_remove(j);
                    merged = true;
                    break 'pairs;
                }
            }
        }
        if !merged {
            break;
        }
    }
    boxes
}

/// Return one simple boundary for the exact rectangle union. Components with
/// holes or corner junctions return None and retain a rectangular encoding.
fn rectangle_union_contour(boxes: &[Bbox]) -> Option<Vec<(i32, i32)>> {
    let mut xs: Vec<i32> = boxes.iter().flat_map(|b| [b.xmin, b.xmax]).collect();
    let mut ys: Vec<i32> = boxes.iter().flat_map(|b| [b.ymin, b.ymax]).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    if xs.len() < 2 || ys.len() < 2 {
        return None;
    }
    let nx = xs.len() - 1;
    let ny = ys.len() - 1;
    let mut filled = vec![false; nx * ny];
    for ix in 0..nx {
        for iy in 0..ny {
            filled[ix * ny + iy] = boxes.iter().any(|b| {
                b.xmin <= xs[ix] && b.xmax >= xs[ix + 1] && b.ymin <= ys[iy] && b.ymax >= ys[iy + 1]
            });
        }
    }
    let at = |x: isize, y: isize| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < nx
            && (y as usize) < ny
            && filled[x as usize * ny + y as usize]
    };
    let mut edges = Vec::new();
    for ix in 0..nx {
        for iy in 0..ny {
            if !filled[ix * ny + iy] {
                continue;
            }
            let (x0, x1, y0, y1) = (xs[ix], xs[ix + 1], ys[iy], ys[iy + 1]);
            if !at(ix as isize, iy as isize - 1) {
                edges.push(((x0, y0), (x1, y0)));
            }
            if !at(ix as isize + 1, iy as isize) {
                edges.push(((x1, y0), (x1, y1)));
            }
            if !at(ix as isize, iy as isize + 1) {
                edges.push(((x1, y1), (x0, y1)));
            }
            if !at(ix as isize - 1, iy as isize) {
                edges.push(((x0, y1), (x0, y0)));
            }
        }
    }
    let mut next = std::collections::HashMap::new();
    let mut indegree = std::collections::HashMap::new();
    for (a, b) in edges {
        if next.insert(a, b).is_some() {
            return None;
        }
        *indegree.entry(b).or_insert(0usize) += 1;
    }
    if indegree.values().any(|&n| n != 1) {
        return None;
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = next.iter().next() {
        let mut points = Vec::new();
        let mut current = start;
        loop {
            points.push(current);
            let end = next.remove(&current)?;
            current = end;
            if current == start {
                break;
            }
            if points.len() > 4 * (nx + ny) {
                return None;
            }
        }
        loops.push(points);
    }
    if loops.len() != 1 {
        return None;
    }
    let mut points = loops.pop().unwrap();
    loop {
        let n = points.len();
        if n <= 4 {
            break;
        }
        let mut keep = vec![true; n];
        for i in 0..n {
            let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            if (a.0 == b.0 && b.0 == c.0) || (a.1 == b.1 && b.1 == c.1) {
                keep[i] = false;
            }
        }
        if keep.iter().all(|&k| k) {
            break;
        }
        points = points
            .into_iter()
            .zip(keep)
            .filter_map(|(p, k)| k.then_some(p))
            .collect();
    }
    (points.len() >= 4).then_some(points)
}

fn placed_orientation(o: pnr_engine::placement::Orient) -> Orientation {
    match o {
        pnr_engine::placement::Orient::N => Orientation::R0,
        pnr_engine::placement::Orient::S => Orientation::R180,
        pnr_engine::placement::Orient::FN => Orientation::MX,
        pnr_engine::placement::Orient::FS => Orientation::MY,
    }
}

fn hint_severity(reason: &pnr_engine::block::HintReason) -> f64 {
    use pnr_engine::block::HintReason;
    match reason {
        HintReason::HighParasiticR {
            estimated_ohm,
            budget_ohm,
            ..
        } => {
            if *budget_ohm > 0.0 {
                (estimated_ohm / budget_ohm - 1.0).max(0.25)
            } else {
                1.0
            }
        }
        HintReason::HighParasiticC {
            estimated_ff,
            budget_ff,
            ..
        } => {
            if *budget_ff > 0.0 {
                (estimated_ff / budget_ff - 1.0).max(0.25)
            } else {
                1.0
            }
        }
        HintReason::MatchedMismatchR { delta_pct, .. }
        | HintReason::MatchedMismatchC { delta_pct, .. } => (delta_pct / 5.0).max(0.25),
        HintReason::PinAccessFailed { .. } => 2.0,
        HintReason::Congested => 1.0,
    }
}

fn variant_preference(
    v: &CellOutput,
    reason: &pnr_engine::block::HintReason,
    max_area: f64,
) -> f64 {
    use pnr_engine::block::HintReason;
    let w = v.bbox.width().max(1) as f64;
    let h = v.bbox.height().max(1) as f64;
    let area = w * h / max_area.max(1.0);
    let aspect = (w.max(h) / w.min(h) - 1.0).min(10.0) / 10.0;
    let edge = if v.pins.is_empty() {
        1.0
    } else {
        v.pins
            .iter()
            .map(|p| {
                let x = p.x + p.w / 2;
                let y = p.y + p.h / 2;
                let d = (x - v.bbox.xmin)
                    .min(v.bbox.xmax - x)
                    .min((y - v.bbox.ymin).min(v.bbox.ymax - y))
                    .max(0);
                f64::from(d) / w.min(h)
            })
            .sum::<f64>()
            / v.pins.len() as f64
    };
    match reason {
        HintReason::HighParasiticR { .. } | HintReason::MatchedMismatchR { .. } => {
            0.55 * edge + 0.30 * area + 0.15 * aspect
        }
        HintReason::HighParasiticC { .. } | HintReason::MatchedMismatchC { .. } => {
            0.65 * area + 0.25 * edge + 0.10 * aspect
        }
        HintReason::PinAccessFailed { .. } | HintReason::Congested => {
            0.55 * edge + 0.30 * aspect + 0.15 * area
        }
    }
}

fn placement_pin_offsets(
    g: &BipartiteHypergraph,
    gen: &[GenCell],
) -> Vec<Vec<Vec<(u32, i32, i32)>>> {
    gen.iter()
        .enumerate()
        .map(|(ci, cell)| {
            cell.variants
                .iter()
                .map(|v| {
                    let cx = (v.bbox.xmin + v.bbox.xmax) / 2;
                    let cy = (v.bbox.ymin + v.bbox.ymax) / 2;
                    g.cells[ci]
                        .pins
                        .iter()
                        .filter_map(|entry| {
                            let (pin, net) = entry;
                            let accesses = pin_accesses(v, &g.cells[ci].name, pin);
                            if accesses.is_empty() {
                                return None;
                            }
                            let n = accesses.len() as i64;
                            let x = accesses
                                .iter()
                                .map(|(_, b)| i64::from((b.xmin + b.xmax) / 2))
                                .sum::<i64>()
                                / n;
                            let y = accesses
                                .iter()
                                .map(|(_, b)| i64::from((b.ymin + b.ymax) / 2))
                                .sum::<i64>()
                                / n;
                            Some((*net, x as i32 - cx, y as i32 - cy))
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// Discover direct-connect transforms by touching same-net pin rectangles,
/// then accept only transforms whose combined cell geometry is DRC-clean and
/// preserves the generated cells' effective MOS channel geometry.
type MosGeometrySignature = std::collections::BTreeMap<(u8, u8, i32, Option<String>), i64>;

fn mos_geometry_signature(store: &GeometryStore, deck: &Deck) -> Option<MosGeometrySignature> {
    let mut normalized = store.clone();
    coalesce_geometry(&mut normalized, deck);
    let ext = extract_netlist_opts(
        &normalized,
        deck,
        &ExtractOpts {
            cut_required: deck.lvs_cut_required,
            ..Default::default()
        },
        Backend::Cpu,
    )
    .ok()?;
    let mut signature = MosGeometrySignature::new();
    for device in ext.devices {
        let kind = match device.kind {
            DeviceKind::Nmos => 0,
            DeviceKind::Pmos => 1,
            _ => 2,
        };
        let flavor = match device.flavor {
            DeviceFlavor::Standard => 0,
            DeviceFlavor::Hvt => 1,
            DeviceFlavor::Lvt => 2,
        };
        *signature
            .entry((kind, flavor, device.l, device.device_class))
            .or_default() += i64::from(device.w);
    }
    Some(signature)
}

fn combined_mos_signature(
    a: &MosGeometrySignature,
    b: &MosGeometrySignature,
) -> MosGeometrySignature {
    let mut combined = a.clone();
    for (key, width) in b {
        *combined.entry(key.clone()).or_default() += width;
    }
    combined
}

fn discover_legal_abutments(
    g: &BipartiteHypergraph,
    gen: &[GenCell],
    deck: &Deck,
    grid: i32,
) -> Vec<pnr_placement::Abutment> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let signatures: Vec<Vec<Option<MosGeometrySignature>>> = gen
        .iter()
        .map(|cell| {
            cell.variants
                .iter()
                .map(|variant| mos_geometry_signature(&variant.store, deck))
                .collect()
        })
        .collect();
    const MAX_PER_PAIR: usize = 64;
    for a in 0..g.cells.len() {
        for b in a + 1..g.cells.len() {
            let mut pair_count = 0usize;
            for (va, ca) in gen[a].variants.iter().enumerate() {
                for (vb, cb) in gen[b].variants.iter().enumerate() {
                    let (acx, acy) = (
                        (ca.bbox.xmin + ca.bbox.xmax) / 2,
                        (ca.bbox.ymin + ca.bbox.ymax) / 2,
                    );
                    let (bcx, bcy) = (
                        (cb.bbox.xmin + cb.bbox.xmax) / 2,
                        (cb.bbox.ymin + cb.bbox.ymax) / 2,
                    );
                    'pins: for (pa, na) in &g.cells[a].pins {
                        for (pb, nb) in &g.cells[b].pins {
                            if na != nb {
                                continue;
                            }
                            for (la, ba) in pin_accesses(ca, &g.cells[a].name, pa) {
                                for (lb, bb) in pin_accesses(cb, &g.cells[b].name, pb) {
                                    if la != lb {
                                        continue;
                                    }
                                    let ay = (ba.ymin + ba.ymax) / 2;
                                    let by = (bb.ymin + bb.ymax) / 2;
                                    let ax = (ba.xmin + ba.xmax) / 2;
                                    let bx = (bb.xmin + bb.xmax) / 2;
                                    let candidates = [
                                        (ba.xmax - acx - (bb.xmin - bcx), ay - acy - (by - bcy)),
                                        (ba.xmin - acx - (bb.xmax - bcx), ay - acy - (by - bcy)),
                                        (ax - acx - (bx - bcx), ba.ymax - acy - (bb.ymin - bcy)),
                                        (ax - acx - (bx - bcx), ba.ymin - acy - (bb.ymax - bcy)),
                                    ];
                                    for (dx0, dy0) in candidates {
                                        let (dx, dy) =
                                            (snap_to_grid(dx0, grid), snap_to_grid(dy0, grid));
                                        let key =
                                            (a as u32, b as u32, va as u16, vb as u16, dx, dy);
                                        if !seen.insert(key) || (dx.abs() < 5 && dy.abs() < 5) {
                                            continue;
                                        }
                                        let ox = ((ca.bbox.width() + cb.bbox.width()) / 2
                                            - dx.abs())
                                        .max(0)
                                            as i64;
                                        let oy = ((ca.bbox.height() + cb.bbox.height()) / 2
                                            - dy.abs())
                                        .max(0)
                                            as i64;
                                        let overlap = ox * oy;
                                        let smaller = (i64::from(ca.bbox.width())
                                            * i64::from(ca.bbox.height()))
                                        .min(
                                            i64::from(cb.bbox.width())
                                                * i64::from(cb.bbox.height()),
                                        );
                                        if overlap * 2 > smaller {
                                            continue;
                                        }
                                        let mut store = GeometryStore::new();
                                        translate_into(
                                            &mut store,
                                            &ca.store,
                                            -acx,
                                            -acy,
                                            Orientation::R0,
                                            &ca.bbox,
                                        );
                                        translate_into(
                                            &mut store,
                                            &cb.store,
                                            dx - bcx,
                                            dy - bcy,
                                            Orientation::R0,
                                            &cb.bbox,
                                        );
                                        let preserves_devices = signatures[a][va]
                                            .as_ref()
                                            .zip(signatures[b][vb].as_ref())
                                            .is_some_and(|(sa, sb)| {
                                                mos_geometry_signature(&store, deck).is_some_and(
                                                    |actual| {
                                                        actual == combined_mos_signature(sa, sb)
                                                    },
                                                )
                                            });
                                        if preserves_devices
                                            && gdsverify::run_drc_no_density(&store, deck)
                                                .violations
                                                .is_empty()
                                        {
                                            out.push(pnr_placement::Abutment {
                                                a: a as u32,
                                                b: b as u32,
                                                variant_a: va as u16,
                                                variant_b: vb as u16,
                                                dx: dx as f32,
                                                dy: dy as f32,
                                            });
                                            pair_count += 1;
                                            if pair_count >= MAX_PER_PAIR {
                                                break 'pins;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "[placement] qualified {} DRC/LVS-clean direct-connect transforms",
        out.len()
    );
    out
}

fn centered_square(store: &mut GeometryStore, layer: LayerId, x: i32, y: i32, side: i32) {
    store.add_rect(layer, x - side / 2, y - side / 2, side, side);
}

struct PinStack {
    terms: Vec<(i32, i32)>,
    accesses: Vec<(i32, i32)>,
    on_poly: bool,
}

#[derive(Clone)]
struct PlannedGuardRing {
    members: Vec<usize>,
    inner: Bbox,
    outer: Bbox,
    ring_type: RingType,
    width: i32,
    connection_net: String,
}

fn plan_guard_rings(
    g: &BipartiteHypergraph,
    gen: &[GenCell],
    placed: &PlacementResult,
    trans: &[(i32, i32)],
    pdk: &pnr_cells::pdk::Pdk,
) -> Vec<PlannedGuardRing> {
    struct Candidate {
        cell: usize,
        inner: Bbox,
        ring_type: RingType,
        width: i32,
        connection_net: String,
        shareable: bool,
    }

    let p = &placed.placement;
    let mut candidates = Vec::new();
    for (cell, generated) in gen.iter().enumerate() {
        let b = generated.variants[p.variant[cell]].bbox;
        let (dx, dy) = trans[cell];
        let inner = Bbox {
            xmin: b.xmin + dx,
            ymin: b.ymin + dy,
            xmax: b.xmax + dx,
            ymax: b.ymax + dy,
        };
        for req in &generated.guard_rings {
            let ring_type = match req.ring_type {
                pnr_constraints::GuardRingType::PsubRing => RingType::PsubRing,
                pnr_constraints::GuardRingType::NwellRing => RingType::NwellRing,
                pnr_constraints::GuardRingType::DoubleRing => {
                    eprintln!("[guard-ring] double ring for cell {cell} needs two explicit rail requirements; keeping it unmaterialized");
                    continue;
                }
            };
            if g.net_id(&req.connection_net).is_none() {
                eprintln!(
                    "[guard-ring] connection net `{}` is absent; skipping cell {cell}",
                    req.connection_net
                );
                continue;
            }
            let width = ring_width_from_depth(pdk, ring_type)
                .max((req.min_width_um * 1000.0).ceil() as i32);
            candidates.push(Candidate {
                cell,
                inner,
                ring_type,
                width,
                connection_net: req.connection_net.clone(),
                shareable: req.shareable,
            });
        }
    }
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    let mut rank = vec![0u8; candidates.len()];
    for i in 0..candidates.len() {
        for j in i + 1..candidates.len() {
            let (a, b) = (&candidates[i], &candidates[j]);
            if !a.shareable
                || !b.shareable
                || a.ring_type != b.ring_type
                || a.connection_net != b.connection_net
            {
                continue;
            }
            let ao = guard_ring_outer_bbox(&a.inner, a.width);
            let bo = guard_ring_outer_bbox(&b.inner, b.width);
            if ao.overlaps(&bo) {
                union_indices_local(&mut parent, &mut rank, i, j);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for i in 0..candidates.len() {
        let root = find_index_local(&mut parent, i);
        groups.entry(root).or_default().push(i);
    }

    let mut planned = Vec::new();
    for ids in groups.into_values() {
        let first = &candidates[ids[0]];
        let mut inner = first.inner;
        let mut width = first.width;
        let mut members = Vec::new();
        for &id in &ids {
            let c = &candidates[id];
            inner.xmin = inner.xmin.min(c.inner.xmin);
            inner.ymin = inner.ymin.min(c.inner.ymin);
            inner.xmax = inner.xmax.max(c.inner.xmax);
            inner.ymax = inner.ymax.max(c.inner.ymax);
            width = width.max(c.width);
            members.push(c.cell);
        }
        members.sort_unstable();
        members.dedup();
        let outer = guard_ring_outer_bbox(&inner, width);
        planned.push(PlannedGuardRing {
            members,
            inner,
            outer,
            ring_type: first.ring_type,
            width,
            connection_net: first.connection_net.clone(),
        });
    }
    planned
}

fn find_index_local(parent: &mut [usize], x: usize) -> usize {
    if parent[x] != x {
        parent[x] = find_index_local(parent, parent[x]);
    }
    parent[x]
}

fn union_indices_local(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
    let (ra, rb) = (find_index_local(parent, a), find_index_local(parent, b));
    if ra == rb {
        return;
    }
    if rank[ra] < rank[rb] {
        parent[ra] = rb;
    } else if rank[ra] > rank[rb] {
        parent[rb] = ra;
    } else {
        parent[rb] = ra;
        rank[ra] += 1;
    }
}

fn emit_guard_rings(
    store: &mut GeometryStore,
    rings: &[PlannedGuardRing],
    deck: &Deck,
    pdk: &pnr_cells::pdk::Pdk,
) -> Result<(), String> {
    let met1 = deck
        .layers
        .id(&pdk.layers.met1)
        .ok_or_else(|| format!("deck missing first-metal role `{}`", pdk.layers.met1))?;
    for (i, ring) in rings.iter().enumerate() {
        let mut builder = CellBuilder::with_context(
            CellContext::new(deck, pdk),
            MatchingTier::None,
            u32::MAX - i as u32,
        );
        draw_guard_ring(
            &mut builder,
            pdk,
            &ring.inner,
            ring.ring_type,
            ring.width,
            &ring.connection_net,
        )
        .map_err(|e| format!("guard-ring generation: {e:?}"))?;
        let generated = builder.finish();
        let first = store.poly_count();
        translate_into(
            store,
            &generated.store,
            0,
            0,
            Orientation::R0,
            &generated.bbox,
        );
        for p in first..store.poly_count() {
            if store.poly_layer[p] == met1 {
                store
                    .net_labels
                    .insert(p as u32, ring.connection_net.clone());
            }
        }
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Interdigitation grouping
// ═══════════════════════════════════════════════════════════════════════

fn group_matched_pairs(
    g: &mut BipartiteHypergraph,
    rec: &ConstraintRecord,
) -> Result<(ConstraintRecord, Vec<u32>), String> {
    let mut groups: Vec<(u32, u32)> = Vec::new();
    let mut already: HashSet<u32> = HashSet::new();
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            if mp.tier < pnr_constraints::MatchingTier::Moderate {
                continue;
            }
            let (a, b) = (mp.device_a.0, mp.device_b.0);
            if already.contains(&a) || already.contains(&b) {
                continue;
            }
            if g.cells[a as usize].model != g.cells[b as usize].model {
                continue;
            }
            if g.cells[a as usize].device.is_none() {
                continue;
            }
            groups.push((a, b));
            already.insert(a);
            already.insert(b);
        }
    }
    if groups.is_empty() {
        return Ok((rec.clone(), (0..g.cells.len() as u32).collect()));
    }
    let n_orig = g.cells.len();
    let mut remap: Vec<u32> = (0..n_orig as u32).collect();
    for &(orig_a, orig_b) in &groups {
        let cur_a = remap[orig_a as usize];
        let cur_b = remap[orig_b as usize];
        let name_a = g.cells[cur_a as usize].name.clone();
        let name_b = g.cells[cur_b as usize].name.clone();
        let model = g.cells[cur_a as usize].model.clone();
        let new_id = g.group(&format!("{name_a}_{name_b}"), &model, &[name_a, name_b])?;
        let removed = [cur_a, cur_b];
        for r in remap.iter_mut() {
            if removed.contains(r) {
                *r = new_id;
            } else {
                *r -= removed.iter().filter(|&&x| x < *r).count() as u32;
            }
        }
    }
    let mut rec2 = rec.clone();
    let remap_id = |id: DeviceId| -> DeviceId { DeviceId(remap[id.0 as usize]) };
    for sg in &mut rec2.symmetry {
        sg.pairs
            .retain(|mp| remap_id(mp.device_a).0 != remap_id(mp.device_b).0);
        for mp in &mut sg.pairs {
            mp.device_a = remap_id(mp.device_a);
            mp.device_b = remap_id(mp.device_b);
        }
        sg.self_symmetric = sg.self_symmetric.iter().map(|&d| remap_id(d)).collect();
        sg.self_symmetric.sort_unstable_by_key(|d| d.0);
        sg.self_symmetric.dedup_by_key(|d| d.0);
    }
    for cc in &mut rec2.cc {
        cc.group_a = cc.group_a.iter().map(|&d| remap_id(d)).collect();
        cc.group_b = cc.group_b.iter().map(|&d| remap_id(d)).collect();
    }
    for p in &mut rec2.proximity {
        p.device_a = remap_id(p.device_a);
        p.device_b = remap_id(p.device_b);
    }
    for iso in &mut rec2.isolation {
        iso.device_a = remap_id(iso.device_a);
        iso.device_b = remap_id(iso.device_b);
    }
    Ok((rec2, remap))
}

// ═══════════════════════════════════════════════════════════════════════
//  Step 3: build one block (cells → place → route feedback loop)
// ═══════════════════════════════════════════════════════════════════════

struct BuiltBlock {
    gen: Vec<GenCell>,
    placed: PlacementResult,
    routed: RoutingResult,
    trans: Vec<(i32, i32)>,
    stacks: Vec<PinStack>,
    iterations: u32,
    best_iteration: u32,
    converged: bool,
    feedback_trace: Vec<IterationSummary>,
}

fn feedback_trace_jsonl(trace: &[IterationSummary], best_iteration: u32) -> Result<String, String> {
    let mut output = String::new();
    for row in trace {
        let value = serde_json::json!({
            "iteration": row.iteration,
            "selected": row.iteration == best_iteration,
            "goal_reached": row.goal_reached,
            "new_best": row.new_best,
            "max_weight": row.max_weight,
            "routing_clean": row.routing_clean,
            "parasitic_clean": row.parasitic_clean,
            "drc_blocking": row.drc_blocking,
            "lvs_matched": row.lvs_matched,
            "die_area_nm2": row.die_area,
            "hard_violations": row.hard_violations,
            "total_r_ohm": row.total_r_ohm,
            "total_c_ff": row.total_c_ff,
            "wirelength_nm": row.wirelength_nm,
            "via_count": row.via_count,
            "physical_score": row.physical_score,
        });
        output.push_str(&serde_json::to_string(&value).map_err(|e| e.to_string())?);
        output.push('\n');
    }
    Ok(output)
}

/// Small monotone search around the tightest routable placement canvas.
/// Higher utilization is tighter. A failed point is an upper bound; a clean
/// point is a lower bound. Before both bounds exist, move gradually instead of
/// destroying density with repeated 2x area expansions.
#[derive(Debug, Clone, Copy)]
struct DensitySearch {
    target: f32,
    tightest_feasible: Option<f32>,
    loosest_failed: Option<f32>,
}

impl DensitySearch {
    const MIN: f32 = 0.20;
    const MAX: f32 = 0.92;
    const RESOLUTION: f32 = 0.02;

    fn new(target: f32) -> Self {
        Self {
            target: target.clamp(Self::MIN, Self::MAX),
            tightest_feasible: None,
            loosest_failed: None,
        }
    }

    fn observe(&mut self, routable: bool) -> Option<(f32, f32)> {
        let old = self.target;
        if routable {
            self.tightest_feasible =
                Some(self.tightest_feasible.map_or(old, |prior| prior.max(old)));
        } else {
            self.loosest_failed = Some(self.loosest_failed.map_or(old, |prior| prior.min(old)));
        }

        let next = match (self.tightest_feasible, self.loosest_failed) {
            (Some(feasible), Some(failed)) if failed - feasible > Self::RESOLUTION => {
                (feasible + failed) * 0.5
            }
            (Some(_), Some(_)) => old,
            (Some(_), None) => (old + 0.03).min(Self::MAX),
            (None, Some(_)) => (old * 0.80).max(Self::MIN),
            (None, None) => old,
        };
        self.target = next;
        ((next - old).abs() > f32::EPSILON).then_some((old, next))
    }
}

#[cfg(test)]
mod density_search_tests {
    use super::DensitySearch;

    #[test]
    fn clean_placement_probes_a_little_tighter() {
        let mut search = DensitySearch::new(0.85);
        assert_eq!(search.observe(true), Some((0.85, 0.88)));
    }

    #[test]
    fn first_routing_failure_backs_off_without_halving_density() {
        let mut search = DensitySearch::new(0.85);
        let (_, next) = search.observe(false).expect("density changes");
        assert!((next - 0.68).abs() < 1e-6);
        assert!(next >= DensitySearch::MIN);
    }

    #[test]
    fn routable_and_failed_points_bracket_the_boundary() {
        let mut search = DensitySearch::new(0.85);
        search.observe(false);
        let (_, midpoint) = search.observe(true).expect("bracket midpoint");
        assert!((midpoint - 0.765).abs() < 1e-6);
        assert!(search.tightest_feasible.unwrap() < search.loosest_failed.unwrap());
    }
}

/// Build a snapshot GeometryStore for the probe from mid-iteration state.
#[cfg(feature = "visualizer")]
fn probe_snapshot(
    gen: &[GenCell],
    placed: &PlacementResult,
    routed: &RoutingResult,
    trans: &[(i32, i32)],
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
) -> GeometryStore {
    let lt = &deck.layers;
    let mets: Vec<LayerId> = cell_pdk
        .layers
        .routing_metals
        .iter()
        .filter_map(|name| lt.id(name))
        .collect();
    let mut store = GeometryStore::new();
    let vi = &placed.placement.variant;
    for (i, c) in gen.iter().enumerate() {
        let chosen = &c.variants[vi[i]];
        let (dx, dy) = trans[i];
        let b = &chosen.bbox;
        translate_into(
            &mut store,
            &chosen.store,
            dx,
            dy,
            placed_orientation(placed.placement.orient[i]),
            b,
        );
    }
    for w in &routed.wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        let layer = mets[(w.layer as usize).min(mets.len() - 1)];
        store.add_rect(layer, x0, y0, x1 - x0, y1 - y0);
    }
    store
}

fn build_block(
    g: &BipartiteHypergraph,
    pdk: &Pdk,
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
    rec: &ConstraintRecord,
    cfg: &FlowConfig,
) -> Result<BuiltBlock, String> {
    let lt = &deck.layers;
    let poly = lt
        .id(&cell_pdk.layers.poly)
        .ok_or_else(|| format!("deck missing gate-layer role `{}`", cell_pdk.layers.poly))?;

    let mut pcfg = cfg.placement.clone();
    pcfg.cell_margin = cell_pdk.device_gap;
    pcfg.boundary_halo = cfg.routing.detailed.pitch;
    // Adaptive die: probe around the tightest routable canvas. The controller
    // keeps feasible/failed bounds and bisects once both are known.
    let density = std::cell::RefCell::new(DensitySearch::new(0.85));
    pcfg.min_side = 2_000;
    if pcfg.debug_dir.is_none() {
        pcfg.debug_dir.clone_from(&cfg.debug_dir);
    }
    let mut rcfg_init = cfg.routing.clone();
    if rcfg_init.debug_dir.is_none() {
        rcfg_init.debug_dir.clone_from(&cfg.debug_dir);
    }
    // ponytail: RefCell lets extract closure write net_priority_overrides for next route iteration
    let rcfg = std::cell::RefCell::new(rcfg_init);

    #[cfg(feature = "visualizer")]
    let vis_frame = std::cell::Cell::new(0u32);

    let block_cfg = pnr_engine::block::BlockConfig {
        max_iters: cfg.max_feedback_iters,
        feedback_threshold: cfg.feedback_threshold,
        circuit_name: Some(g.name.clone()),
        ..Default::default()
    };

    // Build WireParasiticParams from the PDK-declared routing stack.
    let wire_params = {
        let mut layers = Vec::with_capacity(cell_pdk.layers.routing_metals.len());
        for name in &cell_pdk.layers.routing_metals {
            let layer = lt
                .id(name)
                .ok_or_else(|| format!("deck missing routing conductor `{name}`"))?;
            layers.push(
                deck.pex
                    .get(&layer)
                    .ok_or_else(|| format!("PDK missing PEX parameters for `{name}`"))?,
            );
        }
        pnr_constraints::WireParasiticParams {
            sheet_r: layers.iter().map(|p| p.sheet_res_ohm_sq).collect(),
            area_cap: layers.iter().map(|p| p.area_cap_af_um2).collect(),
            fringe_cap: layers.iter().map(|p| p.fringe_cap_af_um).collect(),
            via_r: layers.first().map_or(0.0, |p| p.via_res_ohm),
        }
    };
    // Same params drive post-route contract closure inside run_routing.
    rcfg.borrow_mut().wire_params = Some(wire_params.clone());
    {
        // Landing-claim clearance from in-cell met1 pads: obstacle-pad
        // half-width + stub half-width + met1 spacing, all deck-derived.
        let met1 = lt.id(&cell_pdk.layers.met1);
        let m1_space = deck
            .drc_rules
            .iter()
            .find_map(|r| match r {
                gdsverify::params::DrcRuleParam::MinSpacing { layer, min, .. }
                    if Some(*layer) == met1 =>
                {
                    Some(*min)
                }
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "PDK missing minimum-spacing rule for `{}`",
                    cell_pdk.layers.met1,
                )
            })?;
        let m1_width = deck
            .drc_rules
            .iter()
            .find_map(|r| match r {
                gdsverify::params::DrcRuleParam::MinWidth { layer, min, .. }
                    if Some(*layer) == met1 =>
                {
                    Some(*min)
                }
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "PDK missing minimum-width rule for `{}`",
                    cell_pdk.layers.met1,
                )
            })?;
        rcfg.borrow_mut().detailed.obstacle_clearance = cfg.pad / 2 + m1_width / 2 + m1_space;
        rcfg.borrow_mut().detailed.n_layers =
            u32::try_from(cell_pdk.layers.routing_metals.len())
                .map_err(|_| "PDK routing stack exceeds the router's u32 index space")?;
    }

    let device_names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();

    // Loop-invariant hoist: the reference netlist never changes across iterations.
    let ref_netlist = reference_netlist(g);

    // ponytail: RefCell tracks chosen variant per cell across iterations
    let chosen_variant = std::cell::RefCell::new(vec![0usize; g.cells.len()]);
    let variant_penalties = std::cell::RefCell::new(Vec::<Vec<f64>>::new());
    let abutment_cache = std::cell::RefCell::new(None::<Vec<pnr_placement::Abutment>>);
    // Last iteration's net weights — EMA prior for feedback extraction.
    let prev_weights = std::cell::RefCell::new(HashMap::<String, f64>::new());

    // Engine drives: cells(+hints) → place(+weights) → route → feedback
    let result = pnr_engine::block::run_block(
        &block_cfg,
        // cells: generate cell variants, select based on routing feedback hints
        |_iter, hints| {
            let gen = generate_cells(g, pdk, deck, cell_pdk, rec).unwrap();

            // Causal variant selection: penalize the variant that produced a
            // measured failure, then explore the lowest-loss alternative.
            let mut cv = chosen_variant.borrow_mut();
            let mut penalties = variant_penalties.borrow_mut();
            if penalties.len() != gen.len() {
                *penalties = gen.iter().map(|c| vec![0.0; c.variants.len()]).collect();
            }
            // Decay stale evidence. A variant that caused trouble several
            // placements ago must become eligible again after the topology
            // and neighboring variants change.
            for row in penalties.iter_mut() {
                for loss in row {
                    *loss *= 0.85;
                }
            }
            for hint in hints {
                let ci = hint.cell_idx as usize;
                if ci >= gen.len() || gen[ci].variants.len() <= 1 {
                    continue;
                }
                let variants = &gen[ci].variants;
                let current = cv[ci].min(variants.len() - 1);
                penalties[ci][current] += 5_000.0 * hint_severity(&hint.reason);
                let max_area = variants
                    .iter()
                    .map(|v| f64::from(v.bbox.width()) * f64::from(v.bbox.height()))
                    .fold(1.0f64, f64::max);
                let pick = variants
                    .iter()
                    .enumerate()
                    .min_by(|(ia, a), (ib, b)| {
                        let sa = penalties[ci][*ia] / 5_000.0
                            + variant_preference(a, &hint.reason, max_area);
                        let sb = penalties[ci][*ib] / 5_000.0
                            + variant_preference(b, &hint.reason, max_area);
                        sa.total_cmp(&sb)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(current);
                cv[ci] = pick;
            }

            let sizes: Vec<(i32, i32)> = gen
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let vi = cv.get(i).copied().unwrap_or(0).min(c.variants.len() - 1);
                    let b = &c.variants[vi].bbox;
                    (b.width(), b.height())
                })
                .collect();
            let variant_sizes: Vec<Vec<(i32, i32)>> = gen
                .iter()
                .map(|c| {
                    c.variants
                        .iter()
                        .map(|v| (v.bbox.width(), v.bbox.height()))
                        .collect()
                })
                .collect();
            let lmasks: Vec<u64> = gen
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let vi = cv.get(i).copied().unwrap_or(0).min(c.variants.len() - 1);
                    c.variants[vi].layer_mask()
                })
                .collect();
            let pin_offsets = placement_pin_offsets(g, &gen);
            let cached_abutments = abutment_cache.borrow().clone();
            let abutments = if let Some(cached) = cached_abutments {
                cached
            } else {
                let found = discover_legal_abutments(g, &gen, deck, pdk.grid());
                *abutment_cache.borrow_mut() = Some(found.clone());
                found
            };
            (gen, sizes, variant_sizes, lmasks, pin_offsets, abutments)
        },
        // place: run placement with feedback weights and directional inflation
        |iter, cells_out, net_weights, cell_inflation_x, cell_inflation_y, constraint_adj| {
            let (
                ref _gen,
                ref sizes,
                ref variant_sizes,
                ref lmasks,
                ref pin_offsets,
                ref abutments,
            ) = *cells_out;
            let mut pc = pcfg.clone();
            // Per-iteration seed: a fixed seed makes SA deterministic, so
            // identical feedback reproduces identical layouts and the loop
            // cycles through the same few states instead of exploring.
            pc.seed = pcfg.seed ^ (u64::from(iter)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            pc.utilization = density.borrow().target;
            pc.variant_sizes = variant_sizes.clone();
            pc.initial_variants = chosen_variant.borrow().clone();
            pc.variant_pin_offsets = pin_offsets.clone();
            pc.variant_penalties = variant_penalties.borrow().clone();
            pc.legal_abutments = abutments.clone();
            pc.cell_inflation_x = cell_inflation_x.to_vec();
            pc.cell_inflation_y = cell_inflation_y.to_vec();
            for &(ref name, w) in net_weights {
                pc.net_weight_overrides.insert(name.clone(), w);
            }
            // ponytail: inject isolation constraints from crosstalk violations
            let mut rec_iter = rec.clone();
            for (dev_a, dev_b, gap_nm) in constraint_adj {
                let id_a = g.cell_id(dev_a);
                let id_b = g.cell_id(dev_b);
                if let (Some(a), Some(b)) = (id_a, id_b) {
                    let gap_um = *gap_nm / 1000.0;
                    if let Some(iso) = rec_iter.isolation.iter_mut().find(|iso| {
                        (iso.device_a.0 == a && iso.device_b.0 == b)
                            || (iso.device_a.0 == b && iso.device_b.0 == a)
                    }) {
                        iso.min_distance_um = iso.min_distance_um.max(iso.min_distance_um + gap_um);
                    } else {
                        rec_iter
                            .isolation
                            .push(pnr_constraints::IsolationConstraint {
                                device_a: pnr_constraints::DeviceId(a),
                                device_b: pnr_constraints::DeviceId(b),
                                min_distance_um: gap_um,
                                requires_guard_ring: false,
                                reason: "crosstalk violation feedback".into(),
                            });
                    }
                }
            }
            run_placement(g, sizes, &rec_iter, &pc, lmasks)
        },
        // route: compute pin positions from placement + cells, run routing
        |cells_out, placed| {
            let (ref gen, _, _, _, _, _) = *cells_out;
            let p = &placed.placement;
            let vi = &p.variant;
            let trans: Vec<(i32, i32)> = (0..g.cells.len())
                .map(|i| {
                    let b = &gen[i].variants[vi[i]].bbox;
                    (
                        snap_to_grid(p.x[i] - (b.xmin + b.xmax) / 2, pdk.grid()),
                        snap_to_grid(p.y[i] - (b.ymin + b.ymax) / 2, pdk.grid()),
                    )
                })
                .collect();
            let mut stacks: Vec<PinStack> = Vec::new();
            let mut pin_pos: Vec<Vec<Vec<(i32, i32)>>> = Vec::with_capacity(g.cells.len());
            for (i, c) in g.cells.iter().enumerate() {
                let (dx, dy) = trans[i];
                let chosen = &gen[i].variants[vi[i]];
                let orient = placed_orientation(p.orient[i]);
                let mut row = Vec::with_capacity(c.pins.len());
                for (pn, _) in &c.pins {
                    let accs = pin_accesses(chosen, &c.name, pn);
                    if accs.is_empty() {
                        row.push(Vec::new());
                        continue;
                    }
                    let on_poly = accs[0].0 == poly;
                    let bb = &chosen.bbox;
                    let hv = cell_pdk.contact / 2 + pdk.grid();
                    let centers: Vec<(i32, i32)> = accs
                        .iter()
                        .map(|(_, b)| {
                            let x = ((b.xmin + b.xmax) / 2).clamp(bb.xmin + hv, bb.xmax - hv);
                            let y = ((b.ymin + b.ymax) / 2).clamp(bb.ymin + hv, bb.ymax - hv);
                            let (x, y) =
                                orient.transform(x - bb.xmin, y - bb.ymin, bb.width(), bb.height());
                            (
                                snap_to_grid(x + bb.xmin + dx, pdk.grid()),
                                snap_to_grid(y + bb.ymin + dy, pdk.grid()),
                            )
                        })
                        .collect();
                    let terms = if on_poly {
                        vec![centers[centers.len() / 2]]
                    } else {
                        centers.clone()
                    };
                    row.push(terms.clone());
                    stacks.push(PinStack {
                        terms,
                        accesses: centers,
                        on_poly,
                    });
                }
                pin_pos.push(row);
            }
            // Rings are a post-placement construct, but their contacted M1
            // terminal is a real endpoint of the declared rail and must be
            // presented to the router before detailed routing.
            for ring in plan_guard_rings(g, gen, placed, &trans, cell_pdk) {
                let Some(net_id) = g.net_id(&ring.connection_net) else {
                    continue;
                };
                let terminal = (
                    snap_to_grid((ring.outer.xmin + ring.outer.xmax) / 2, pdk.grid()),
                    snap_to_grid(ring.outer.ymin + ring.width / 2, pdk.grid()),
                );
                'anchor: for &ci in &ring.members {
                    for (pi, &(_, pin_net)) in g.cells[ci].pins.iter().enumerate() {
                        if pin_net == net_id {
                            pin_pos[ci][pi].push(terminal);
                            pin_pos[ci][pi].sort_unstable();
                            pin_pos[ci][pi].dedup();
                            break 'anchor;
                        }
                    }
                }
            }
            let routed = run_routing_at(g, p, Some(&pin_pos), rec, &rcfg.borrow());
            (routed, trans, stacks)
        },
        // extract: pull feedback from completed routing + visualizer probe
        |_cells_out, placed, route_out| {
            let (ref routed, ref _trans, ref stacks) = *route_out;
            let (ref gen, _, _, _, _, _) = *_cells_out;
            *chosen_variant.borrow_mut() = placed.placement.variant.clone();

            #[cfg(feature = "visualizer")]
            if let Some(ref dir) = cfg.debug_dir {
                let (ref gen, _, _, _, _, _) = *_cells_out;
                let snap = probe_snapshot(gen, placed, routed, _trans, deck, cell_pdk);
                let frame = vis_frame.get();
                vis_frame.set(frame + 1);
                let path = dir.join("dump.txt");
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .unwrap();
                use std::io::Write;
                let _ = writeln!(
                    f,
                    "# frame {} wl={:.1} overuse={}",
                    frame,
                    routed.report.wirelength_nm as f64 / 1000.0,
                    routed.report.overuse
                );
                for i in 0..snap.poly_count() {
                    let start = snap.poly_vert_start[i] as usize;
                    let len = snap.poly_vert_len[i] as usize;
                    let _ = write!(f, "{}", snap.poly_layer[i]);
                    for j in 0..len {
                        let _ = write!(
                            f,
                            " {},{}",
                            snap.verts_x[start + j],
                            snap.verts_y[start + j]
                        );
                    }
                    let _ = writeln!(f);
                }
            }

            // Closure: map (device_idx, pin_name) → routing net index
            let net_of_device = |dev: u32, pin: &str| -> Option<usize> {
                let cell = g.cells.get(dev as usize)?;
                let (_, net_id) = cell.pins.iter().find(|(n, _)| n == pin)?;
                let net_name = g.nets.get(*net_id as usize)?;
                routed.net_names.iter().position(|n| n == net_name)
            };

            let fb = {
                // EMA prior: last iteration's weights, so resolved nets relax
                // gradually instead of the loop recomputing from scratch
                // (research doc §1 — this was passed as None, dead code).
                let pw = prev_weights.borrow();
                extract_feedback_with_prior(
                    routed,
                    &placed.placement,
                    &rec.parasitic,
                    cfg.feedback_weight_cap,
                    cfg.feedback_inflation_cap,
                    if pw.is_empty() { None } else { Some(&*pw) },
                    Some(&wire_params),
                    &rec.symmetry,
                    &device_names,
                    &net_of_device,
                )
            };
            *prev_weights.borrow_mut() = routed
                .net_names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.clone(), fb.net_weights[i]))
                .collect();

            // ponytail: feed net ordering back into routing config for next iteration
            {
                let mut next = rcfg.borrow_mut();
                next.net_priority_overrides = fb.net_order_priority.iter().cloned().collect();
                next.global_history = routed.global_history.clone();
                next.detailed_history = routed.detailed_history.clone();
            }

            // Build cell hints from parasitic budget violations
            let mut cell_hints: Vec<pnr_engine::block::CellHint> = Vec::new();
            for budget in &rec.parasitic {
                if let Some(idx) = routed.net_names.iter().position(|n| n == &budget.net_name) {
                    let r_over = budget.max_r > 0.0 && fb.net_r_ohm[idx] > budget.max_r;
                    let c_over = budget.max_c > 0.0 && fb.net_c_ff[idx] > budget.max_c;
                    if !r_over && !c_over {
                        continue;
                    }
                    // Find cells on this net
                    if let Some(net_id) = g.net_id(&budget.net_name) {
                        for &(cell_idx, _) in g.pins_on_net(net_id) {
                            let reason = if r_over {
                                pnr_engine::block::HintReason::HighParasiticR {
                                    net: budget.net_name.clone(),
                                    estimated_ohm: fb.net_r_ohm[idx],
                                    budget_ohm: budget.max_r,
                                }
                            } else {
                                pnr_engine::block::HintReason::HighParasiticC {
                                    net: budget.net_name.clone(),
                                    estimated_ff: fb.net_c_ff[idx],
                                    budget_ff: budget.max_c,
                                }
                            };
                            cell_hints.push(pnr_engine::block::CellHint { cell_idx, reason });
                        }
                    }
                }
            }

            // Add hints from matched-pair parasitic mismatches
            let match_tol = pnr_constraints::RouteMatchingTolerance::default();
            for md in &fb.matched_deltas {
                if md.r_delta_pct > match_tol.max_r_delta_pct {
                    if let (Some(a), Some(b)) = (g.cell_id(&md.device_a), g.cell_id(&md.device_b)) {
                        for ci in [a, b] {
                            cell_hints.push(pnr_engine::block::CellHint {
                                cell_idx: ci,
                                reason: pnr_engine::block::HintReason::MatchedMismatchR {
                                    net_a: md.net_a.clone(),
                                    net_b: md.net_b.clone(),
                                    delta_pct: md.r_delta_pct,
                                },
                            });
                        }
                    }
                }
                if md.c_delta_pct > match_tol.max_c_delta_pct {
                    if let (Some(a), Some(b)) = (g.cell_id(&md.device_a), g.cell_id(&md.device_b)) {
                        for ci in [a, b] {
                            cell_hints.push(pnr_engine::block::CellHint {
                                cell_idx: ci,
                                reason: pnr_engine::block::HintReason::MatchedMismatchC {
                                    net_a: md.net_a.clone(),
                                    net_b: md.net_b.clone(),
                                    delta_pct: md.c_delta_pct,
                                },
                            });
                        }
                    }
                }
            }

            // Pin-access failures directly target the closest cell. They do
            // not imply that the whole placement canvas is too dense.
            for failure in &routed.landing_failures {
                let nearest = (0..placed.placement.x.len()).min_by_key(|&ci| {
                    let p = &placed.placement;
                    let (w, h) = p.sizes[ci];
                    let dx = (failure.pin.0 - p.x[ci]).abs().saturating_sub(w / 2).max(0);
                    let dy = (failure.pin.1 - p.y[ci]).abs().saturating_sub(h / 2).max(0);
                    i64::from(dx).pow(2) + i64::from(dy).pow(2)
                });
                if let Some(cell_idx) = nearest {
                    let net = routed
                        .net_names
                        .get(failure.net as usize)
                        .cloned()
                        .unwrap_or_default();
                    cell_hints.push(pnr_engine::block::CellHint {
                        cell_idx: cell_idx as u32,
                        reason: pnr_engine::block::HintReason::PinAccessFailed {
                            net,
                            displacement_nm: routed.track_pitch * 8,
                        },
                    });
                }
            }
            for ci in 0..placed.placement.x.len() {
                let pressure = fb
                    .cell_inflation_x
                    .get(ci)
                    .copied()
                    .unwrap_or(1.0)
                    .max(fb.cell_inflation_y.get(ci).copied().unwrap_or(1.0));
                if !fb.clean
                    && pressure > 1.08
                    && !cell_hints.iter().any(|h| h.cell_idx == ci as u32)
                {
                    cell_hints.push(pnr_engine::block::CellHint {
                        cell_idx: ci as u32,
                        reason: pnr_engine::block::HintReason::Congested,
                    });
                }
            }

            // Parasitic cleanness: budgets from routing estimate (PEX
            // overrides below when extraction succeeds) + matched deltas.
            let mut budget_clean = rec.parasitic.iter().all(|b| {
                routed
                    .net_names
                    .iter()
                    .position(|n| n == &b.net_name)
                    .map_or(true, |i| {
                        (b.max_r <= 0.0 || fb.net_r_ohm[i] <= b.max_r)
                            && (b.max_c <= 0.0 || fb.net_c_ff[i] <= b.max_c)
                    })
            });
            let matched_clean = fb.matched_deltas.iter().all(|md| {
                md.r_delta_pct <= match_tol.max_r_delta_pct
                    && md.c_delta_pct <= match_tol.max_c_delta_pct
            });

            // ── In-loop signoff: assemble this iteration's geometry, run
            // DRC + LVS (+ per-net PEX) so convergence gates on the real
            // goal and violations become placement pressure. ──
            let mut constraint_adjustments = routed.crosstalk_violations.clone();
            let mut vstore = GeometryStore::new();
            let mut vlabels = Vec::new();
            let (drc_blocking, lvs_matched) = match merge_block_geometry(
                &mut vstore,
                g,
                gen,
                placed,
                routed,
                _trans,
                stacks,
                cfg,
                deck,
                cell_pdk,
                0,
                0,
                &mut vlabels,
                &device_names,
            ) {
                Ok(mut net_sample) => {
                    let merged = coalesce_geometry(&mut vstore, deck);
                    for sample in net_sample.values_mut() {
                        *sample = merged.remap(*sample);
                    }
                    // In-loop signoff always waives density — skip those rules
                    // instead of computing window clips and discarding them.
                    let drc = gdsverify::run_drc_no_density(&vstore, deck);
                    let blocking: Vec<&Violation> = drc.violations.iter().collect();
                    // Spacing-class violations on DEVICE layers → isolation
                    // pressure on the two devices nearest the violation (same
                    // channel the crosstalk feedback uses). Metal/via layers
                    // are the ROUTER's problem — pushing devices apart for a
                    // wire-to-wire spacing violation just inflates the die
                    // without fixing anything.
                    let p = &placed.placement;
                    for v in blocking.iter().filter(|v| {
                        v.kind.contains("spacing")
                            && !v.layer.starts_with("met")
                            && !v.layer.starts_with("via")
                    }) {
                        let mut near: Vec<(i64, usize)> = (0..p.x.len())
                            .map(|i| {
                                let dx = i64::from(p.x[i] - v.x);
                                let dy = i64::from(p.y[i] - v.y);
                                (dx * dx + dy * dy, i)
                            })
                            .collect();
                        near.sort_unstable();
                        if near.len() >= 2 {
                            let shortfall = (v.limit - v.measured).max(0) as f64;
                            constraint_adjustments.push((
                                g.cells[near[0].1].name.clone(),
                                g.cells[near[1].1].name.clone(),
                                shortfall,
                            ));
                        }
                    }
                    let lvs_ok = match extract_netlist_opts(
                        &vstore,
                        deck,
                        &ExtractOpts {
                            cut_required: deck.lvs_cut_required,
                            ..Default::default()
                        },
                        Backend::Cpu,
                    ) {
                        Ok(ext) => {
                            if std::env::var("PNR_DEBUG_LVS").is_ok() {
                                for d in &ext.devices {
                                    eprintln!(
                                        "[lvs-dbg] {:?} g={} s={} d={} b={} w={} l={} class={:?}",
                                        d.kind,
                                        d.gate,
                                        d.source,
                                        d.drain,
                                        d.body,
                                        d.w,
                                        d.l,
                                        d.device_class
                                    );
                                }
                                for lname in ["met2", "via1", "met1", "li", "licon", "mcon"] {
                                    if let Some(lid) = deck.layers.id(lname) {
                                        for p in vstore.polys_on_layer(lid) {
                                            let bb = vstore.poly_bbox[p.0 as usize];
                                            eprintln!(
                                                "[lvs-dbg] {lname} net={} ({},{})-({},{})",
                                                ext.net_of_poly[p.0 as usize],
                                                bb.xmin,
                                                bb.ymin,
                                                bb.xmax,
                                                bb.ymax
                                            );
                                        }
                                    }
                                }
                            }
                            // Per-net PEX replaces the routing estimate for
                            // budget cleanness — judged on extracted geometry.
                            if !rec.parasitic.is_empty() {
                                budget_clean = run_pex_by_net_checked(
                                    &vstore,
                                    deck,
                                    &ext.net_of_poly,
                                )
                                .is_ok_and(|per_net| {
                                    rec.parasitic.iter().all(|b| {
                                        routed
                                            .net_names
                                            .iter()
                                            .position(|n| n == &b.net_name)
                                            .and_then(|ni| net_sample.get(&(ni as u32)))
                                            .map(|&pp| ext.net_of_poly[pp.0 as usize])
                                            .and_then(|net| per_net.get(&net))
                                            .is_some_and(|np| {
                                                (b.max_r <= 0.0 || np.r_ohm <= b.max_r)
                                                    && (b.max_c <= 0.0
                                                        || np.cap_af / 1000.0 <= b.max_c)
                                            })
                                    })
                                });
                            }
                            let cmp_opts = CompareOpts {
                                strict: false,
                                w_tolerance: deck.w_tolerance.clone(),
                                l_tolerance: deck.l_tolerance.clone(),
                            };
                            compare(&ext, &ref_netlist, &cmp_opts).matched
                        }
                        Err(e) => {
                            eprintln!("[signoff] in-loop extraction failed: {e}");
                            false
                        }
                    };
                    eprintln!(
                        "[signoff] in-loop: DRC {} blocking | LVS {} | parasitic budgets {}",
                        blocking.len(),
                        if lvs_ok { "MATCH" } else { "MISMATCH" },
                        if budget_clean { "ok" } else { "over" }
                    );
                    (blocking.len(), lvs_ok)
                }
                Err(e) => {
                    eprintln!("[signoff] in-loop assembly failed: {e}");
                    (usize::MAX, false)
                }
            };
            let parasitic_clean = budget_clean && matched_clean;
            let overlap_violations = usize::from(placed.report.overlap_final > 0.5);
            if overlap_violations > 0 {
                eprintln!(
                    "[placement] blocking residual footprint overlap: {:.0} nm^2",
                    placed.report.overlap_final
                );
            }

            // Dedup adjustments: one entry per unordered pair, keeping the
            // largest gap. Each DRC violation instance produced one entry —
            // the same pair N times would escalate the isolation constraint
            // N-fold per iteration (runaway die growth).
            let mut pair_max: HashMap<(String, String), f64> = HashMap::new();
            for (a, b, gap) in constraint_adjustments.drain(..) {
                let key = if a <= b { (a, b) } else { (b, a) };
                let e = pair_max.entry(key).or_insert(0.0);
                if gap > *e {
                    *e = gap;
                }
            }
            let mut constraint_adjustments: Vec<(String, String, f64)> = pair_max
                .into_iter()
                .map(|((a, b), gap)| (a, b, gap))
                .collect();
            constraint_adjustments.sort_by(|x, y| (&x.0, &x.1).cmp(&(&y.0, &y.1)));

            // Bracket the legal-and-routable density boundary. Residual
            // footprint overlap needs more placement room; missing pin
            // landings are local cell/variant failures and deliberately do
            // not change density.
            let density_observation = if overlap_violations > 0 || fb.spread_required {
                Some(false)
            } else if fb.clean {
                Some(true)
            } else {
                None
            };
            if let Some(routable) = density_observation {
                if let Some((old, next)) = density.borrow_mut().observe(routable) {
                    eprintln!(
                        "[engine] routing {} at util {:.2} — next density probe {:.2}",
                        if routable { "clean" } else { "failed" },
                        old,
                        next
                    );
                }
            }

            pnr_engine::block::Feedback {
                net_weights: routed
                    .net_names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.clone(), fb.net_weights[i]))
                    .collect(),
                cell_inflation_x: fb.cell_inflation_x.clone(),
                cell_inflation_y: fb.cell_inflation_y.clone(),
                cell_hints,
                max_weight: fb.max_weight,
                clean: fb.clean,
                parasitic_clean,
                constraint_adjustments,
                drc_blocking,
                lvs_matched,
                die_area: placed.placement.die.0 as u64 * placed.placement.die.1 as u64,
                hard_violations: placed.report.validation.hard_violations.len()
                    + routed.report.validation.hard_violations.len()
                    + overlap_violations,
                total_r_ohm: fb.net_r_ohm.iter().sum(),
                total_c_ff: fb.net_c_ff.iter().sum(),
                wirelength_nm: routed.report.wirelength_nm.max(0) as u64,
                via_count: routed.report.via_count,
            }
        },
    );

    let pnr_engine::block::BlockResult {
        cells,
        placement: placed,
        routing,
        iterations,
        best_iteration,
        converged,
        trace: feedback_trace,
    } = result;
    let (gen, _, _, _, _, _) = cells;
    let (routed, trans, stacks) = routing;
    if let Some(dir) = &cfg.debug_dir {
        if let Err(e) = placed.write_debug(dir, g) {
            eprintln!("[engine] best placement artifact write failed: {e}");
        }
        if let Err(e) = routed.write_debug(dir) {
            eprintln!("[engine] best routing artifact write failed: {e}");
        }
    }
    Ok(BuiltBlock {
        gen,
        placed,
        routed,
        trans,
        stacks,
        iterations,
        best_iteration,
        converged,
        feedback_trace,
    })
}

// ═══════════════════════════════════════════════════════════════════════
//  Step 4: merge one block's geometry into a store
// ═══════════════════════════════════════════════════════════════════════

/// Component-wise so the in-loop signoff can call it on borrowed mid-iteration
/// state (no owned `BuiltBlock` needed).
#[allow(clippy::too_many_arguments)]
fn merge_block_geometry(
    store: &mut GeometryStore,
    g: &BipartiteHypergraph,
    gen: &[GenCell],
    placed: &PlacementResult,
    routed: &RoutingResult,
    trans: &[(i32, i32)],
    stacks: &[PinStack],
    cfg: &FlowConfig,
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
    dx_global: i32,
    dy_global: i32,
    labels: &mut Vec<crate::gds::TextLabel>,
    cell_names: &[String],
) -> Result<HashMap<u32, PolyId>, String> {
    let lt = &deck.layers;
    let lid = |name: &str| {
        lt.id(name)
            .ok_or_else(|| format!("deck missing layer `{name}`"))
    };
    let mcon = lid(&cell_pdk.layers.mcon)?;
    let licon = lid(&cell_pdk.layers.licon)?;
    let li = lid(&cell_pdk.layers.li)?;
    let mets: Vec<LayerId> = cell_pdk
        .layers
        .routing_metals
        .iter()
        .map(|name| lid(name))
        .collect::<Result<_, _>>()?;
    let cuts: Vec<LayerId> = cell_pdk
        .layers
        .routing_vias
        .iter()
        .map(|name| lid(name))
        .collect::<Result<_, _>>()?;
    let (met1, met2, via1) = (mets[0], mets[1], cuts[0]);
    let nsdm = lt.id(&cell_pdk.layers.nsdm);
    let psdm = lt.id(&cell_pdk.layers.psdm);
    let nwell = lt.id(&cell_pdk.layers.nwell);

    // ponytail: 236/0 is a common annotation layer, won't collide with sky130 physical layers
    const LABEL_LAYER: i16 = 236;
    const LABEL_DATATYPE: i16 = 0;

    let vi = &placed.placement.variant;
    for (i, c) in gen.iter().enumerate() {
        let chosen = &c.variants[vi[i]];
        let (dx, dy) = (trans[i].0 + dx_global, trans[i].1 + dy_global);
        let b = &chosen.bbox;
        translate_into(
            store,
            &chosen.store,
            dx,
            dy,
            placed_orientation(placed.placement.orient[i]),
            b,
        );
        let implant = match c.device_type {
            Some(DeviceType::Nmos) => nsdm,
            Some(DeviceType::Pmos) => psdm,
            _ => None,
        };
        if let Some(l) = implant {
            store.add_rect(l, b.xmin + dx, b.ymin + dy, b.width(), b.height());
        }
        if c.device_type == Some(DeviceType::Pmos) {
            if let Some(l) = nwell {
                store.add_rect(l, b.xmin + dx, b.ymin + dy, b.width(), b.height());
            }
        }
        if i < cell_names.len() {
            labels.push(crate::gds::TextLabel {
                x: b.xmin + dx + b.width() / 2,
                y: b.ymin + dy + b.height() / 2,
                layer: LABEL_LAYER,
                datatype: LABEL_DATATYPE,
                text: cell_names[i].clone(),
            });
        }
    }

    let rings = plan_guard_rings(g, gen, placed, trans, cell_pdk);
    emit_guard_rings(store, &rings, deck, cell_pdk)?;

    let mut net_sample: HashMap<u32, PolyId> = HashMap::new();
    let met_at = |layer: u32| mets[(layer as usize).min(mets.len() - 1)];
    for w in &routed.wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        let p = store.add_rect(
            met_at(w.layer),
            x0 + dx_global,
            y0 + dy_global,
            x1 - x0,
            y1 - y0,
        );
        net_sample.entry(w.net).or_insert(p);
    }
    for v in &routed.vias {
        let (vx, vy) = (v.x + dx_global, v.y + dy_global);
        let cut = cuts[(v.layer as usize).min(cuts.len() - 1)];
        centered_square(store, cut, vx, vy, cfg.via_size);
        centered_square(store, met_at(v.layer), vx, vy, cfg.pad);
        centered_square(store, met_at(v.layer + 1), vx, vy, cfg.pad);
    }

    for s in stacks {
        for &(ax, ay) in &s.accesses {
            centered_square(store, licon, ax + dx_global, ay + dy_global, cfg.via_size);
        }
        if s.on_poly {
            let hw = cfg.li_width / 2;
            let x0 = s.accesses.iter().map(|a| a.0).min().unwrap() - hw + dx_global;
            let x1 = s.accesses.iter().map(|a| a.0).max().unwrap() + hw + dx_global;
            let y0 = s.accesses.iter().map(|a| a.1).min().unwrap() - hw + dy_global;
            let y1 = s.accesses.iter().map(|a| a.1).max().unwrap() + hw + dy_global;
            store.add_rect(li, x0, y0, x1 - x0, y1 - y0);
        }
        for &(tx, ty) in &s.terms {
            centered_square(store, mcon, tx + dx_global, ty + dy_global, cfg.via_size);
        }
        let mut terms: Vec<(i32, i32)> = s
            .terms
            .iter()
            .map(|&(x, y)| (x + dx_global, y + dy_global))
            .collect();
        terms.sort_unstable_by_key(|&(x, y)| (y, x));
        let mut k = 0;
        while k < terms.len() {
            let (y, x0) = (terms[k].1, terms[k].0);
            let mut x1 = x0;
            while k + 1 < terms.len() && terms[k + 1].1 == y && terms[k + 1].0 - x1 < cfg.pad + 140
            {
                k += 1;
                x1 = terms[k].0;
            }
            store.add_rect(
                met1,
                x0 - cfg.pad / 2,
                y - cfg.pad / 2,
                x1 - x0 + cfg.pad,
                cfg.pad,
            );
            k += 1;
        }
    }

    // Stubs at met1 min WIDTH (from deck), not pad width: pads carry mcon
    // enclosure and min-area; a pad-wide stub next to a neighbor pin's stub
    // (pins can sit closer than a track pitch across cells) violates met1
    // spacing where a min-width one clears. Stub merges with both pads, so
    // min-area is collective.
    let stub_w = deck
        .drc_rules
        .iter()
        .find_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinWidth { layer, min, .. } if *layer == met1 => {
                Some(*min)
            }
            _ => None,
        })
        .unwrap_or(cfg.pad);
    // met2 landing pad: via + 2x process enclosure (deck), NOT full wire pad —
    // smaller footprint means fewer met2 track nodes blocked around the pin.
    let enc = deck
        .drc_rules
        .iter()
        .find_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinEnclosure { min, .. } => Some(*min),
            _ => None,
        })
        .unwrap_or(30);
    let m2pad = cfg.via_size + 2 * enc;
    for l in &routed.landings {
        let (px, py) = (l.pin.0 + dx_global, l.pin.1 + dy_global);
        let (nx, ny) = (l.node.0 + dx_global, l.node.1 + dy_global);
        // Long legs stay at min width (foreign geometry can sit alongside);
        // short legs go full pad width — nothing fits next to them anyway,
        // and the width step against the pads would leave same-net notches.
        let leg = (px - nx).abs().max((py - ny).abs());
        let stub_w = if leg < cfg.pad + stub_w {
            cfg.pad
        } else {
            stub_w
        };
        let hw = stub_w / 2;
        if l.layer == 1 {
            // met2 landing: via at pin, pad at pin, L-shaped wire to node.
            // The pad + L live outside the router's model — overlapping a
            // foreign met2 wire is a short. Pick the L orientation that
            // avoids foreign wires (met2 has no spacing rule, only overlap).
            let foreign = |x0: i32, y0: i32, w: i32, h: i32| -> usize {
                routed
                    .wires
                    .iter()
                    .filter(|wr| {
                        if wr.layer == 0 || wr.net == l.net {
                            return false;
                        }
                        let (wx0, wy0, wx1, wy1) = wire_rect(wr);
                        x0 < wx1 + dx_global
                            && wx0 + dx_global < x0 + w
                            && y0 < wy1 + dy_global
                            && wy0 + dy_global < y0 + h
                    })
                    .count()
            };
            centered_square(store, via1, px, py, cfg.via_size);
            centered_square(store, met2, px, py, m2pad);
            if px == nx && py == ny {
                // coincident — pad already covers it
            } else if px == nx || py == ny {
                // axis-aligned — single segment
                store.add_rect(
                    met2,
                    px.min(nx) - hw,
                    py.min(ny) - hw,
                    (px - nx).abs() + stub_w,
                    (py - ny).abs() + stub_w,
                );
            } else {
                // vertical-first legs
                let va = (px - hw, py.min(ny) - hw, stub_w, (py - ny).abs() + stub_w);
                let ha = (px.min(nx) - hw, ny - hw, (px - nx).abs() + stub_w, stub_w);
                // horizontal-first legs
                let hb = (px.min(nx) - hw, py - hw, (px - nx).abs() + stub_w, stub_w);
                let vb = (nx - hw, py.min(ny) - hw, stub_w, (py - ny).abs() + stub_w);
                let hits_a = foreign(va.0, va.1, va.2, va.3) + foreign(ha.0, ha.1, ha.2, ha.3);
                let hits_b = foreign(hb.0, hb.1, hb.2, hb.3) + foreign(vb.0, vb.1, vb.2, vb.3);
                let (r1, r2) = if hits_b < hits_a { (hb, vb) } else { (va, ha) };
                store.add_rect(met2, r1.0, r1.1, r1.2, r1.3);
                store.add_rect(met2, r2.0, r2.1, r2.2, r2.3);
            }
        } else {
            // met1 landing: pad at node, L-shaped wire to node.
            centered_square(store, met1, nx, ny, cfg.pad);
            if (px - nx).abs() >= stub_w || (py - ny).abs() >= stub_w {
                if px == nx || py == ny {
                    // axis-aligned — single segment
                    store.add_rect(
                        met1,
                        px.min(nx) - hw,
                        py.min(ny) - hw,
                        (px - nx).abs() + stub_w,
                        (py - ny).abs() + stub_w,
                    );
                } else {
                    // L-shape: met1 horizontal-preferred — horizontal from
                    // pin to node's x, then vertical to node's y.
                    store.add_rect(
                        met1,
                        px.min(nx) - hw,
                        py - hw,
                        (px - nx).abs() + stub_w,
                        stub_w,
                    );
                    store.add_rect(
                        met1,
                        nx - hw,
                        py.min(ny) - hw,
                        stub_w,
                        (py - ny).abs() + stub_w,
                    );
                }
            }
        }
    }

    // Same-net notch repair: pad-row corner slivers (landing node pads vs pin
    // pad rows) are sub-min gaps inside one merged shape — fill with metal.
    for (layer, g) in gdsverify::same_shape_gap_fills(store, deck) {
        store.add_rect(layer, g.xmin, g.ymin, g.xmax - g.xmin, g.ymax - g.ymin);
    }

    Ok(net_sample)
}

// ═══════════════════════════════════════════════════════════════════════
//  Step 5: signoff
// ═══════════════════════════════════════════════════════════════════════

fn run_signoff(
    store: &GeometryStore,
    deck: &Deck,
    g: &BipartiteHypergraph,
    routed: &RoutingResult,
    net_sample: &HashMap<u32, PolyId>,
    rec: &ConstraintRecord,
    advanced_config: &gdsverify::SignoffConfig,
) -> SignoffReport {
    let advanced = gdsverify::run_signoff_suite(store, deck, advanced_config);
    let drc = run_drc(store, deck);
    // Density is a tapeout rule, not a router-quality hint.  Keep the legacy
    // field for API compatibility, but no final-signoff density marker is
    // waived.  The calibrated density/CMP suite is an additional gate.
    let (drc_blocking, drc_waived_density) = final_drc_policy(&drc);

    let reference = reference_netlist(g);
    let ext = match extract_netlist_opts(
        store,
        deck,
        &ExtractOpts {
            cut_required: deck.lvs_cut_required,
            ..Default::default()
        },
        Backend::Cpu,
    ) {
        Ok(e) => e,
        Err(e) => {
            return SignoffReport {
                drc,
                drc_blocking,
                drc_waived_density,
                lvs: LvsResult {
                    matched: false,
                    reason: format!("extraction failed: {e}"),
                    mismatches: Vec::new(),
                    extracted_devices: 0,
                    nmos: 0,
                    pmos: 0,
                    ambiguous_classes: 0,
                    label_conflicts: Vec::new(),
                    floating_nets: Vec::new(),
                },
                pex: PexReport {
                    parasitics: Vec::new(),
                },
                contracts: Vec::new(),
                advanced,
            }
        }
    };
    let cmp_opts = CompareOpts {
        strict: deck.strict,
        w_tolerance: deck.w_tolerance.clone(),
        l_tolerance: deck.l_tolerance.clone(),
    };
    let mut lvs: LvsResult = compare(&ext, &reference, &cmp_opts);
    if deck.fail_on_floating && !ext.floating_nets.is_empty() {
        lvs.matched = false;
        lvs.reason = format!("{} floating extracted net(s)", ext.floating_nets.len());
    }
    let pex: PexReport = gdsverify::run_pex(store, deck);

    let mut contracts = Vec::new();
    if !rec.parasitic.is_empty() {
        let per_net = run_pex_by_net_checked(store, deck, &ext.net_of_poly);
        let names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        for b in &rec.parasitic {
            let mut contract = b.to_contract(&names);
            if let Err(diagnostics) = &per_net {
                contract.consume("signoff");
                contract.violate(
                    "signoff",
                    diagnostics.len() as f64,
                    "PEX extraction diagnostics",
                );
                contracts.push(contract);
                continue;
            }
            let per_net = per_net.as_ref().expect("checked above");
            let routed_net = routed.net_names.iter().position(|n| n == &b.net_name);
            let parasitics = routed_net
                .and_then(|ni| net_sample.get(&(ni as u32)))
                .map(|&p| ext.net_of_poly[p.0 as usize])
                .and_then(|net| per_net.get(&net));
            if let Some(np) = parasitics {
                contract.consume("routing");
                let cap_ff = np.cap_af / 1000.0;
                if np.r_ohm <= b.max_r && cap_ff <= b.max_c {
                    contract.satisfy(
                        "routing",
                        &format!(
                            "R {:.2} ohm <= {:.2}, C {:.2} fF <= {:.2}",
                            np.r_ohm, b.max_r, cap_ff, b.max_c
                        ),
                    );
                } else if np.r_ohm > b.max_r {
                    contract.violate("routing", np.r_ohm - b.max_r, "ohm over budget");
                } else {
                    contract.violate("routing", cap_ff - b.max_c, "fF over budget");
                }
            }
            contracts.push(contract);
        }
    }

    SignoffReport {
        drc,
        drc_blocking,
        drc_waived_density,
        lvs,
        pex,
        contracts,
        advanced,
    }
}

fn final_drc_policy(drc: &DrcReport) -> (Vec<Violation>, usize) {
    (drc.violations.clone(), 0)
}

fn reference_netlist(g: &BipartiteHypergraph) -> RefNetlist {
    let mut devices = Vec::new();
    for c in &g.cells {
        let dev_list: Vec<&DeviceRecord> = if !c.grouped_devices.is_empty() {
            c.grouped_devices.iter().collect()
        } else if let Some(d) = &c.device {
            vec![d]
        } else {
            continue;
        };
        for d in dev_list {
            let kind = match d.device_type {
                DeviceType::Nmos => DeviceKind::Nmos,
                DeviceType::Pmos => DeviceKind::Pmos,
                _ => continue,
            };
            let net = |t: &str| -> String {
                let grouped_key = format!("{}.{t}", d.name);
                c.pins
                    .iter()
                    .find(|(p, _)| p == t || p == &grouped_key)
                    .map_or(String::new(), |(_, n)| g.nets[*n as usize].clone())
            };
            // ponytail: emit one reduced device — extractor's reduce_netlist
            // merges parallel fingers/instances, so the reference must match
            let m = i32::from(d.multiplier.max(1));
            devices.push(RefDevice {
                kind: kind.clone(),
                gate: net("G"),
                source: net("S"),
                drain: net("D"),
                w: d.w * m,
                l: d.l,
                flavor: DeviceFlavor::Standard,
            });
        }
    }
    RefNetlist {
        devices,
        net_seeds: std::collections::HashMap::new(),
        ref_two_terminal: Vec::new(),
        ref_bjt: Vec::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Circuit-agnostic constraint extraction
// ═══════════════════════════════════════════════════════════════════════

fn auto_constraints(
    g: &BipartiteHypergraph,
    roles: &[pnr_constraints::NetClass],
    pad_names: &[String],
    cell_pdk: &pnr_cells::pdk::Pdk,
) -> ConstraintRecord {
    let n = g.cells.len();

    let pin_net = |cell: usize, pin: &str| -> Option<u32> {
        g.cells[cell]
            .pins
            .iter()
            .find(|(p, _)| p == pin)
            .map(|(_, n)| *n)
    };
    let is_signal = |net: u32| -> bool {
        matches!(
            roles.get(net as usize),
            Some(pnr_constraints::NetClass::Signal)
        )
    };
    let is_fet = |cell: usize| -> bool {
        g.cells[cell]
            .device
            .as_ref()
            .is_some_and(|d| matches!(d.device_type, DeviceType::Nmos | DeviceType::Pmos))
    };
    let is_complement = |a: usize, b: usize| -> bool {
        let da = g.cells[a].device.as_ref().map(|d| d.device_type);
        let db = g.cells[b].device.as_ref().map(|d| d.device_type);
        matches!(
            (da, db),
            (Some(DeviceType::Nmos), Some(DeviceType::Pmos))
                | (Some(DeviceType::Pmos), Some(DeviceType::Nmos))
        )
    };

    let mut rec = ConstraintRecord::default();
    let mut used: HashSet<u32> = HashSet::new();
    let mut gc = 0u32;

    // ── 1. Symmetry: signature-matched FET pairs by terminal connectivity ──
    // Priority: cross-coupled > differential (shared S, different signal G) > mirror (shared G+S)

    // Pass 1a: cross-coupled — gate_a == drain_b AND gate_b == drain_a
    for i in 0..n {
        if !is_fet(i) || used.contains(&(i as u32)) {
            continue;
        }
        let di = g.cells[i].device.as_ref().unwrap();
        for j in (i + 1)..n {
            if !is_fet(j) || used.contains(&(j as u32)) {
                continue;
            }
            let dj = g.cells[j].device.as_ref().unwrap();
            if di.device_type != dj.device_type || di.w != dj.w || di.l != dj.l {
                continue;
            }
            let (gi, gj) = (pin_net(i, "G"), pin_net(j, "G"));
            let (d_i, d_j) = (pin_net(i, "D"), pin_net(j, "D"));
            if gi.is_none() || d_i.is_none() {
                continue;
            }
            if gi == d_j && gj == d_i {
                rec.symmetry.push(pnr_constraints::SymmetryGroup {
                    group_id: format!("xc_{gc}"),
                    axis: None,
                    pairs: vec![pnr_constraints::MatchingPair {
                        device_a: DeviceId(i as u32),
                        device_b: DeviceId(j as u32),
                        matching_type: pnr_constraints::MatchingType::DiffPair,
                        tier: pnr_constraints::MatchingTier::None,
                        max_dvth_mv: 1.0,
                        max_did_pct: 1.0,
                        w_ratio: None,
                    }],
                    self_symmetric: Vec::new(),
                });
                gc += 1;
                used.insert(i as u32);
                used.insert(j as u32);
                break;
            }
        }
    }

    // Pass 1b: differential — shared S, different signal G, different D
    for i in 0..n {
        if !is_fet(i) || used.contains(&(i as u32)) {
            continue;
        }
        let di = g.cells[i].device.as_ref().unwrap();
        for j in (i + 1)..n {
            if !is_fet(j) || used.contains(&(j as u32)) {
                continue;
            }
            let dj = g.cells[j].device.as_ref().unwrap();
            if di.device_type != dj.device_type || di.w != dj.w || di.l != dj.l {
                continue;
            }
            let (si, sj) = (pin_net(i, "S"), pin_net(j, "S"));
            if si.is_none() || si != sj {
                continue;
            }
            let (gi, gj) = (pin_net(i, "G"), pin_net(j, "G"));
            if gi == gj {
                continue;
            }
            if gi.map_or(true, |n| !is_signal(n)) || gj.map_or(true, |n| !is_signal(n)) {
                continue;
            }
            let (d_i, d_j) = (pin_net(i, "D"), pin_net(j, "D"));
            if d_i == d_j {
                continue;
            }
            if gi == d_j || gj == d_i {
                continue;
            } // exclude cross-coupled

            let mut sg = pnr_constraints::SymmetryGroup {
                group_id: format!("dp_{gc}"),
                axis: None,
                pairs: vec![pnr_constraints::MatchingPair {
                    device_a: DeviceId(i as u32),
                    device_b: DeviceId(j as u32),
                    matching_type: pnr_constraints::MatchingType::DiffPair,
                    tier: pnr_constraints::MatchingTier::None,
                    max_dvth_mv: 1.0,
                    max_did_pct: 1.0,
                    w_ratio: None,
                }],
                self_symmetric: Vec::new(),
            };

            // Tail: same-type device whose drain feeds the shared source
            let shared_src = si.unwrap();
            for k in 0..n {
                if k == i || k == j || !is_fet(k) || used.contains(&(k as u32)) {
                    continue;
                }
                let dk = g.cells[k].device.as_ref().unwrap();
                if dk.device_type != di.device_type {
                    continue;
                }
                if pin_net(k, "D") == Some(shared_src) {
                    sg.self_symmetric.push(DeviceId(k as u32));
                    used.insert(k as u32);
                    break;
                }
            }
            gc += 1;
            rec.symmetry.push(sg);
            used.insert(i as u32);
            used.insert(j as u32);
            break;
        }
    }

    // Pass 1c: mirror — shared G + S, different D, same L (W may differ)
    for i in 0..n {
        if !is_fet(i) || used.contains(&(i as u32)) {
            continue;
        }
        let di = g.cells[i].device.as_ref().unwrap();
        for j in (i + 1)..n {
            if !is_fet(j) || used.contains(&(j as u32)) {
                continue;
            }
            let dj = g.cells[j].device.as_ref().unwrap();
            if di.device_type != dj.device_type || di.l != dj.l {
                continue;
            }
            if di.model_name != dj.model_name {
                continue;
            }
            let (gi, gj) = (pin_net(i, "G"), pin_net(j, "G"));
            if gi.is_none() || gi != gj {
                continue;
            }
            let (si, sj) = (pin_net(i, "S"), pin_net(j, "S"));
            if si.is_none() || si != sj {
                continue;
            }
            if pin_net(i, "D") == pin_net(j, "D") {
                continue;
            }

            let w_ratio = if di.w != dj.w {
                pnr_constraints::width_ratio(di.w, dj.w, 8, 0.01)
            } else {
                None
            };

            rec.symmetry.push(pnr_constraints::SymmetryGroup {
                group_id: format!("mir_{gc}"),
                axis: None,
                pairs: vec![pnr_constraints::MatchingPair {
                    device_a: DeviceId(i as u32),
                    device_b: DeviceId(j as u32),
                    matching_type: pnr_constraints::MatchingType::Mirror,
                    tier: pnr_constraints::MatchingTier::None,
                    max_dvth_mv: 5.0,
                    max_did_pct: 2.0,
                    w_ratio,
                }],
                self_symmetric: Vec::new(),
            });
            gc += 1;
            used.insert(i as u32);
            used.insert(j as u32);
            break;
        }
    }

    // ── 2. CC: symmetry pairs with W mismatch → common centroid ──
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            let (ai, bi) = (mp.device_a.0 as usize, mp.device_b.0 as usize);
            let da = g.cells[ai].device.as_ref().unwrap();
            let db = g.cells[bi].device.as_ref().unwrap();
            if da.w != db.w {
                if let Some((ra, rb)) = pnr_constraints::width_ratio(da.w, db.w, 8, 0.01) {
                    let pat = if ra + rb <= 4 {
                        pnr_constraints::PatternType::Abba
                    } else {
                        pnr_constraints::PatternType::CommonCentroid2d
                    };
                    rec.cc.push(pnr_constraints::CcGroup {
                        group_a: vec![mp.device_a; ra as usize],
                        group_b: vec![mp.device_b; rb as usize],
                        pattern: pat,
                    });
                }
            }
        }
    }

    // ── 3. Proximity: every symmetry pair → soft pull (distance = 0) ──
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            rec.proximity.push(pnr_constraints::ProximityRule {
                device_a: mp.device_a,
                device_b: mp.device_b,
                min_distance_um: 0.0,
            });
        }
    }

    // ── 4. Isolation: PDK-defined complementary-well separation ──
    let well_spacing_um = f64::from(cell_pdk.well_spacing.max(0)) / 1000.0;
    for i in 0..n {
        if !is_fet(i) {
            continue;
        }
        for j in (i + 1)..n {
            if !is_fet(j) {
                continue;
            }
            if is_complement(i, j) {
                rec.isolation.push(pnr_constraints::IsolationConstraint {
                    device_a: DeviceId(i as u32),
                    device_b: DeviceId(j as u32),
                    min_distance_um: well_spacing_um,
                    requires_guard_ring: false,
                    reason: "well-type isolation".into(),
                });
            }
        }
    }

    // ── 5. Thermal: every symmetry pair → gradient constraint ──
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            rec.thermal
                .push(pnr_constraints::ThermalGradientConstraint {
                    device_a: mp.device_a,
                    device_b: mp.device_b,
                    max_delta_c: 0.1,
                    estimated_gradient_c: 0.0,
                });
        }
    }

    // ── 6. Stress: all devices in symmetry groups → center pull ──
    for sg in &rec.symmetry {
        for dev in sg.all_devices() {
            rec.stress.push(pnr_constraints::StressConstraint {
                device_id: dev,
                max_centroid_distance_um: 50.0,
            });
        }
    }

    // ── 7. Net classification: name-based + sensitivity upgrade ──
    let mut sensitive_nets: HashSet<u32> = HashSet::new();
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            let (ai, bi) = (mp.device_a.0 as usize, mp.device_b.0 as usize);
            for pin in &["D", "G", "S"] {
                let na = pin_net(ai, pin);
                let nb = pin_net(bi, pin);
                if na != nb {
                    if let Some(n) = na {
                        sensitive_nets.insert(n);
                    }
                    if let Some(n) = nb {
                        sensitive_nets.insert(n);
                    }
                }
            }
        }
    }
    for (ni, name) in g.nets.iter().enumerate() {
        let role = &roles[ni];
        let net_class = if sensitive_nets.contains(&(ni as u32)) {
            pnr_constraints::NetClass::Sensitive
        } else {
            *role
        };
        rec.net_class.push(pnr_constraints::NetClassification {
            net_name: name.clone(),
            net_class,
            voltage_domain: None,
            shielding_required: net_class == pnr_constraints::NetClass::Sensitive,
            parasitic_c_budget_ff: None,
            parasitic_r_budget_ohm: None,
            preferred_layers: Vec::new(),
            max_coupling_ff: None,
        });
    }

    // ── 8. Straight net: shared terminals of symmetry pairs ──
    // A vertical mirror axis places partners left/right at the same y, so the
    // natural direct segment is horizontal. Marking these nets vertical
    // pulled both devices onto the axis and contradicted symmetry legality.
    let mut straight_added: HashSet<u32> = HashSet::new();
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            let (ai, bi) = (mp.device_a.0 as usize, mp.device_b.0 as usize);
            for pin in &["D", "G", "S"] {
                let na = pin_net(ai, pin);
                let nb = pin_net(bi, pin);
                if na == nb {
                    if let Some(net_id) = na {
                        if !straight_added.contains(&net_id) {
                            let name = &g.nets[net_id as usize];
                            let lower = name.to_ascii_lowercase();
                            let is_supply =
                                pnr_constraints::SUPPLY_NAMES.iter().any(|p| lower == *p)
                                    || pnr_constraints::GROUND_NAMES.iter().any(|p| lower == *p);
                            if !is_supply {
                                rec.straight.push(pnr_constraints::StraightNet {
                                    net: name.clone(),
                                    vertical: false,
                                });
                                straight_added.insert(net_id);
                            }
                        }
                    }
                }
            }
        }
    }

    // ── 9. Crosstalk: clock × sensitive exclusion ──
    let clock_nets: Vec<String> = rec
        .net_class
        .iter()
        .filter(|nc| nc.net_class == pnr_constraints::NetClass::Clock)
        .map(|nc| nc.net_name.clone())
        .collect();
    let sens_nets: Vec<String> = rec
        .net_class
        .iter()
        .filter(|nc| nc.net_class == pnr_constraints::NetClass::Sensitive)
        .map(|nc| nc.net_name.clone())
        .collect();
    for c in &clock_nets {
        for s in &sens_nets {
            rec.crosstalk.push(pnr_constraints::CrosstalkExclusion {
                net_a: c.clone(),
                net_b: s.clone(),
                min_spacing_um: 2.0,
            });
        }
    }

    // ── 10. Differential routing: differentiated gate nets of diff pairs ──
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            if mp.matching_type != pnr_constraints::MatchingType::DiffPair
                || mp.tier < pnr_constraints::MatchingTier::Moderate
            {
                continue;
            }
            let (ai, bi) = (mp.device_a.0 as usize, mp.device_b.0 as usize);
            let ga = pin_net(ai, "G");
            let gb = pin_net(bi, "G");
            if let (Some(na), Some(nb)) = (ga, gb) {
                if na != nb {
                    rec.differential.push(pnr_constraints::DifferentialPair {
                        net_pos: g.nets[na as usize].clone(),
                        net_neg: g.nets[nb as usize].clone(),
                        ..Default::default()
                    });
                }
            }
            // Also matched drain nets
            let da = pin_net(ai, "D");
            let db = pin_net(bi, "D");
            if let (Some(na), Some(nb)) = (da, db) {
                if na != nb {
                    rec.differential.push(pnr_constraints::DifferentialPair {
                        net_pos: g.nets[na as usize].clone(),
                        net_neg: g.nets[nb as usize].clone(),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // ── 11. Current flow: all FETs get left-to-right default ──
    for i in 0..n {
        if is_fet(i) {
            rec.current_flow.push(pnr_constraints::CurrentFlowTag {
                device_id: DeviceId(i as u32),
                direction: pnr_constraints::CurrentFlowDir::LeftToRight,
            });
        }
    }

    // ── 12. Guard ring: only explicitly declared package-pad nets feed
    // injector detection. A reusable subcircuit port is not itself a pad. ──
    let pad_nets: HashSet<u32> = pad_names.iter().filter_map(|p| g.net_id(p)).collect();
    if !pad_nets.is_empty() {
        // Collect resistors from the netlist
        let mut resistors: Vec<(u32, u32, f64)> = Vec::new();
        for c in &g.cells {
            if let Some(d) = &c.device {
                if d.device_type == DeviceType::Res {
                    let nets: Vec<u32> = c.pins.iter().map(|(_, n)| *n).collect();
                    if nets.len() >= 2 {
                        let r_ohm = d.params.get("r").copied().unwrap_or(1e6);
                        resistors.push((nets[0], nets[1], r_ohm));
                    }
                }
            }
        }
        let device_nets: HashMap<DeviceId, Vec<u32>> = g
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.device
                    .as_ref()
                    .is_some_and(|d| matches!(d.device_type, DeviceType::Nmos | DeviceType::Pmos))
            })
            .map(|(i, c)| (DeviceId(i as u32), c.pins.iter().map(|(_, n)| *n).collect()))
            .collect();
        let injectors =
            pnr_constraints::detect_injectors(&pad_nets, &resistors, &device_nets, 1000.0);
        for inj in &injectors {
            if inj.is_injector {
                rec.guard_ring.push(pnr_constraints::GuardRingRequirement {
                    device_id: inj.device_id,
                    ring_type: match g.cells[inj.device_id.0 as usize]
                        .device
                        .as_ref()
                        .map(|d| d.device_type)
                    {
                        Some(DeviceType::Pmos) => pnr_constraints::GuardRingType::NwellRing,
                        _ => pnr_constraints::GuardRingType::PsubRing,
                    },
                    shareable: false,
                    tap_pitch_um: 2.0,
                    min_width_um: 0.5,
                    max_ring_resistance_ohm: 100.0,
                    enclosure_complete: true,
                    connection_net: "VSS".into(),
                });
            }
        }
    }

    // ── 13. LDE bounds: every symmetry pair gets WPE/LOD budget ──
    for sg in &rec.symmetry {
        for mp in &sg.pairs {
            let ai = mp.device_a.0 as usize;
            let da = g.cells[ai].device.as_ref().unwrap();
            // SA/SB from finger geometry: outer = poly pitch, inner = half pitch
            let nf = da.nf.max(1) as f64;
            let poly_pitch_um = (da.w as f64 / 1000.0) / nf + 0.2;
            rec.lde.push(pnr_constraints::LdeBound {
                pair: (mp.device_a, mp.device_b),
                min_well_edge_distance_um: 1.0,
                max_wpe_dvth_mv: 2.0,
                max_lod_did_pct: 2.0,
                max_sa_sb_mismatch_um: 0.1,
                outer_sa_um: poly_pitch_um,
                outer_sb_um: poly_pitch_um,
                inner_sa_um: poly_pitch_um / 2.0,
                inner_sb_um: poly_pitch_um / 2.0,
                sti_dvth_mv: 0.0,
                same_width: true,
                same_orientation: true,
            });
        }
    }

    // ── 14. Dummy: all devices in symmetry groups need edge dummies ──
    for sg in &rec.symmetry {
        for dev in sg.all_devices() {
            rec.dummy.push(pnr_constraints::DummyConstraint {
                device_id: dev,
                dummy_type: pnr_constraints::DummyType::Full,
                moat_ext_um: 0.2,
                min_poly_clearance_um: 0.1,
            });
        }
    }

    // ── 15. Unitization: CC groups with ratio > 1:1 ──
    for cc in &rec.cc {
        let all = cc.all_devices();
        if all.is_empty() {
            continue;
        }
        let dev0 = g.cells[all[0].0 as usize].device.as_ref().unwrap();
        let mut unit_counts = std::collections::HashMap::new();
        unit_counts.insert(
            g.cells[cc.group_a[0].0 as usize].name.clone(),
            cc.group_a.len() as u32,
        );
        unit_counts.insert(
            g.cells[cc.group_b[0].0 as usize].name.clone(),
            cc.group_b.len() as u32,
        );
        let mut target_ratio = std::collections::HashMap::new();
        target_ratio.insert(
            g.cells[cc.group_a[0].0 as usize].name.clone(),
            cc.group_a.len() as u32,
        );
        target_ratio.insert(
            g.cells[cc.group_b[0].0 as usize].name.clone(),
            cc.group_b.len() as u32,
        );
        rec.unitization
            .push(pnr_constraints::UnitizationConstraint {
                group_id: format!(
                    "unit_{}",
                    all.iter()
                        .map(|d| d.0.to_string())
                        .collect::<Vec<_>>()
                        .join("_")
                ),
                device_type: match dev0.device_type {
                    DeviceType::Nmos => pnr_constraints::DeviceType::Nmos,
                    DeviceType::Pmos => pnr_constraints::DeviceType::Pmos,
                    DeviceType::Res => pnr_constraints::DeviceType::Resistor,
                    DeviceType::Cap | DeviceType::Ncap | DeviceType::Pcap => {
                        pnr_constraints::DeviceType::Capacitor
                    }
                    DeviceType::Bjt => pnr_constraints::DeviceType::Bjt,
                    _ => pnr_constraints::DeviceType::Nmos,
                },
                unit_geometry: std::collections::HashMap::new(),
                instance_unit_counts: unit_counts,
                target_ratio,
                series_parallel_allowed: pnr_constraints::SeriesParallel::Parallel,
                required_pattern: cc.pattern,
                same_variant_required: true,
                dummy_required: true,
                route_matching_required: true,
                devices: all,
            });
    }

    // ── 16. Aging: tag NMOS for HCI, PMOS for NBTI ──
    for i in 0..n {
        if let Some(d) = &g.cells[i].device {
            let mechanism = match d.device_type {
                DeviceType::Nmos => pnr_constraints::AgingMechanism::Hci,
                DeviceType::Pmos => pnr_constraints::AgingMechanism::Nbti,
                _ => continue,
            };
            rec.aging.push(pnr_constraints::AgingConstraint {
                device_id: DeviceId(i as u32),
                mechanism,
                severity: pnr_constraints::Severity::Warning,
                description: format!("{:?} susceptible device", mechanism),
            });
        }
    }

    // ── 17. Parasitic: sensitive nets get conservative budgets ──
    for nc in &rec.net_class {
        if nc.net_class == pnr_constraints::NetClass::Sensitive {
            rec.parasitic.push(pnr_constraints::ParasiticBudget {
                net_name: nc.net_name.clone(),
                max_r: 10_000.0,
                max_c: 500.0,
            });
        }
    }

    // ── 18. Antenna: all signal/sensitive nets get default ratio limit ──
    for nc in &rec.net_class {
        if matches!(
            nc.net_class,
            pnr_constraints::NetClass::Signal | pnr_constraints::NetClass::Sensitive
        ) {
            let matched = sensitive_nets.contains(&(g.net_id(&nc.net_name).unwrap_or(u32::MAX)));
            rec.antenna.push(pnr_constraints::AntennaConstraint {
                net_name: nc.net_name.clone(),
                max_ratio: 400.0,
                matched_symmetric_repair: matched,
            });
        }
    }

    // ── 19. ESD: subcircuit ports get default protection requirement ──
    for port in &g.ports {
        let lower = port.to_ascii_lowercase();
        let is_power = pnr_constraints::SUPPLY_NAMES.iter().any(|p| lower == *p)
            || pnr_constraints::GROUND_NAMES.iter().any(|p| lower == *p);
        if !is_power {
            rec.esd.push(pnr_constraints::EsdConstraint {
                pad_name: port.clone(),
                protection_type: pnr_constraints::EsdProtectionType::Primary,
                primary_clamp_required: true,
                secondary_cdm_required: false,
                max_bus_resistance_ohm: 10.0,
                ecgr_required: false,
                silicide_block_required: false,
            });
        }
    }

    // ── 20. DTI: process-gated complement pairs near symmetry groups ──
    // DTI is a technology capability, not a generic analog constraint. Keep
    // unordered pairs canonical so overlapping symmetry groups cannot emit the
    // same forbidden band twice in opposite directions.
    if let Some(dti) = cell_pdk.dti {
        let mut dti_added = HashSet::<(u32, u32)>::new();
        for sg in &rec.symmetry {
            let sym_devs: HashSet<u32> = sg.all_devices().iter().map(|d| d.0).collect();
            for &dev_id in &sym_devs {
                let di = g.cells[dev_id as usize].device.as_ref().unwrap();
                for j in 0..n {
                    if sym_devs.contains(&(j as u32)) || !is_fet(j) {
                        continue;
                    }
                    let dj = g.cells[j].device.as_ref().unwrap();
                    if di.device_type == dj.device_type {
                        continue;
                    }
                    let pair = (dev_id.min(j as u32), dev_id.max(j as u32));
                    if dti_added.insert(pair) {
                        rec.dti.push(pnr_constraints::DtiPair {
                            device_a: DeviceId(pair.0),
                            device_b: DeviceId(pair.1),
                            s_max: f64::from(dti.shared_max_gap) / 1000.0,
                            d_dti: f64::from(dti.separated_min_gap) / 1000.0,
                        });
                    }
                }
            }
        }
    }

    // ── 21. Environment: WPE four-edge for all symmetry devices ──
    for sg in &rec.symmetry {
        let devs = sg.all_devices();
        if !devs.is_empty() {
            rec.environment
                .push(pnr_constraints::EnvironmentalConstraint {
                    constraint_id: format!("wpe_{}", sg.group_id),
                    kind: pnr_constraints::EnvironmentalKind::WpeFourEdge,
                    scope: devs.clone(),
                    strength: pnr_constraints::ConstraintStrength::Hard,
                    threshold: 1.0,
                    units: pnr_constraints::ThresholdUnit::Um,
                });
            rec.environment
                .push(pnr_constraints::EnvironmentalConstraint {
                    constraint_id: format!("lod_{}", sg.group_id),
                    kind: pnr_constraints::EnvironmentalKind::LodSaSb,
                    scope: devs,
                    strength: pnr_constraints::ConstraintStrength::Soft,
                    threshold: 0.1,
                    units: pnr_constraints::ThresholdUnit::Um,
                });
        }
    }

    // ── 22. Bias current: tag from SPICE params if available ──
    for i in 0..n {
        if let Some(d) = &g.cells[i].device {
            if !matches!(d.device_type, DeviceType::Nmos | DeviceType::Pmos) {
                continue;
            }
            let id_ma = d
                .params
                .get("id")
                .or(d.params.get("ids"))
                .copied()
                .unwrap_or(0.0);
            let vgs_vth = d.params.get("vgs_minus_vth").copied().unwrap_or(0.0);
            if id_ma > 0.0 || vgs_vth > 0.0 {
                rec.bias_current.push(pnr_constraints::BiasCurrentTag {
                    device_id: DeviceId(i as u32),
                    id_ma,
                    vgs_minus_vth_mv: vgs_vth,
                });
            }
        }
    }

    rec
}

// ═══════════════════════════════════════════════════════════════════════
//  Canonical template method
// ═══════════════════════════════════════════════════════════════════════

pub(crate) fn run(
    input: FlowInput,
    full: &crate::pdk::FullPdk,
    rec: &ConstraintRecord,
    cfg: &FlowConfig,
) -> Result<FlowResult, String> {
    // Step 1: take ownership of the validated frontend payload. From here on,
    // every operation is physical-design work owned by the backend. Resolve
    // process-sized defaults here so downstream algorithms see only numbers.
    let FlowInput {
        graph: g_pre,
        net_classes,
    } = input;
    let mut resolved = cfg.clone();
    if resolved.via_size <= 0 {
        resolved.via_size = full.cells.mcon_size;
    }
    if resolved.pad <= 0 {
        resolved.pad = full.cells.mcon_size + 2 * full.cells.m1_enc;
    }
    if resolved.li_width <= 0 {
        resolved.li_width = full.cells.contact;
    }
    resolved.placement.grid = full.netlist.grid();
    let met1 = full
        .deck
        .layers
        .id(&full.cells.layers.met1)
        .ok_or_else(|| format!("deck missing first-metal role `{}`", full.cells.layers.met1))?;
    let met1_spacing = full
        .deck
        .drc_rules
        .iter()
        .find_map(|rule| match rule {
            gdsverify::params::DrcRuleParam::MinSpacing { layer, min, .. } if *layer == met1 => {
                Some(*min)
            }
            _ => None,
        })
        .ok_or_else(|| {
            format!(
                "PDK missing minimum-spacing rule for `{}`",
                full.cells.layers.met1,
            )
        })?;
    resolved.routing.detailed.wire_width = resolved.pad;
    resolved.routing.detailed.pitch = resolved.pad + met1_spacing;
    let cfg = &resolved;

    // Step 2: initialize optional debug output and derive the canonical analog
    // constraint record when the caller did not provide one.
    #[cfg(feature = "visualizer")]
    if let Some(ref dir) = cfg.debug_dir {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join("dump.txt"), "");
    }
    let auto_rec;
    let rec = if rec.symmetry.is_empty() && rec.cc.is_empty() && rec.differential.is_empty() {
        auto_rec = auto_constraints(&g_pre, &net_classes, &cfg.pad_nets, &full.cells);
        &auto_rec
    } else {
        rec
    };

    // Step 3: generate matched cells, place them, route them, and close the
    // feedback loop. Hierarchical partitioning remains disabled until an
    // inter-block router can preserve connectivity by construction.
    let mut graph = g_pre.clone();
    let (block_constraints, _) = group_matched_pairs(&mut graph, rec)?;
    let built = build_block(
        &graph,
        &full.netlist,
        &full.deck,
        &full.cells,
        &block_constraints,
        cfg,
    )?;

    // Step 4: materialize cell, guard-ring, landing, wire, and via geometry in
    // one flat store, then canonicalize overlapping polygons once.
    let mut store = GeometryStore::new();
    let names: Vec<String> = graph.cells.iter().map(|cell| cell.name.clone()).collect();
    let mut labels: Vec<crate::gds::TextLabel> = Vec::new();
    let mut net_samples = merge_block_geometry(
        &mut store,
        &graph,
        &built.gen,
        &built.placed,
        &built.routed,
        &built.trans,
        &built.stacks,
        cfg,
        &full.deck,
        &full.cells,
        0,
        0,
        &mut labels,
        &names,
    )?;
    let merged = coalesce_geometry(&mut store, &full.deck);
    for sample in net_samples.values_mut() {
        *sample = merged.remap(*sample);
    }
    eprintln!(
        "[geometry] union {} -> {} polygons, {} merged components, {} nm^2 duplicate area removed",
        merged.input_polygons,
        merged.output_polygons,
        merged.merged_components,
        merged.removed_overlap_area,
    );

    // Step 5: compute the final signoff evidence. DRC, LVS, PEX, advanced
    // checks, and hard-constraint status all observe the same returned geometry;
    // callers can enforce their tapeout policy from the typed report.
    let signoff = run_signoff(
        &store,
        &full.deck,
        &g_pre,
        &built.routed,
        &net_samples,
        rec,
        &cfg.signoff_checks,
    );

    // Step 6: emit deterministic artifacts only after signoff and assemble the
    // owned result. No frontend code participates in physical output creation.
    let mut gds_path = None;
    if let Some(dir) = &cfg.debug_dir {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!("{}.gds", g_pre.name));
        crate::gds::write_gds(&store, &full.deck.layers, &g_pre.name, &path, &labels)
            .map_err(|e| e.to_string())?;
        gds_path = Some(path);
    }

    let result = FlowResult {
        placement: built.placed,
        routing: built.routed,
        store,
        signoff,
        gds_path,
        graph: g_pre,
        iterations: built.iterations,
        best_iteration: built.best_iteration,
        converged: built.converged,
        feedback_trace: built.feedback_trace,
    };
    if let Some(dir) = &cfg.debug_dir {
        let trace = feedback_trace_jsonl(&result.feedback_trace, result.best_iteration)?;
        std::fs::write(dir.join("feedback.jsonl"), trace)
            .map_err(|e| format!("feedback trace write failed: {e}"))?;
        let _ = std::fs::write(dir.join("signoff.txt"), result.to_string());
        let _ = std::fs::write(
            dir.join("signoff.json"),
            crate::gds::signoff_json(&result.signoff),
        );
    }
    Ok(result)
}
