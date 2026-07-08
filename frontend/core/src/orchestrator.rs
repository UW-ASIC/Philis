//! Orchestrator: parse → annotate → build blocks → merge → signoff.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

use gdsverify::{
    compare, extract_netlist_opts, run_drc, run_pex_by_net, Backend, Bbox, CompareOpts, Deck,
    DeviceFlavor, DeviceKind, DrcReport, ExtractOpts, GeometryStore, LayerId, LvsResult,
    PexReport, PolyId, RefDevice, RefNetlist, Violation,
};
use pnr_constraints::{ConstraintContract, Contractable, DeviceId};
use pnr_cells::generators::{
    bjt::BjtSpec, capacitor::CapacitorSpec, diode::DiodeSpec,
    inductor::InductorSpec, mosfet::MosfetSpec, resistor::ResistorSpec,
    candidates,
};
use pnr_cells::netlist::BipartiteHypergraph;
use pnr_cells::device::DeviceRecord;
use pnr_cells::{CellBuilder, CellOutput, DeviceType, MatchingTier, Orientation};
use pnr_placement::{run_placement, ConstraintRecord, PlacementConfig, PlacementResult};
use pnr_routing::{extract_feedback, run_routing_at, wire_rect, RoutingConfig, RoutingResult};

use crate::pdk::{CellBase, Pdk};

// ═══════════════════════════════════════════════════════════════════════
//  Config / result
// ═══════════════════════════════════════════════════════════════════════

pub struct FlowConfig {
    pub placement: PlacementConfig,
    pub routing: RoutingConfig,
    pub via_size: i32,
    pub pad: i32,
    pub li_width: i32,
    pub debug_dir: Option<PathBuf>,
    pub max_feedback_iters: u32,
    pub feedback_weight_cap: f64,
    pub feedback_threshold: f64,
    pub feedback_inflation_cap: f64,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            placement: PlacementConfig::default(),
            routing: RoutingConfig::default(),
            via_size: 170,
            pad: 290,
            li_width: 170,
            debug_dir: None,
            max_feedback_iters: 10,
            feedback_weight_cap: 5.0,
            feedback_threshold: 1.2,
            feedback_inflation_cap: 1.5,
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
}

pub struct FlowResult {
    pub graph: BipartiteHypergraph,
    pub placement: PlacementResult,
    pub routing: RoutingResult,
    pub store: GeometryStore,
    pub signoff: SignoffReport,
    pub gds_path: Option<PathBuf>,
}

impl fmt::Display for FlowResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.placement.report)?;
        write!(f, "{}", self.routing.report)?;
        let s = &self.signoff;
        writeln!(
            f,
            "signoff: DRC {} blocking ({} density waived) | LVS {} ({} N / {} P) | PEX R(met1) {:.1} ohm, C {:.1} fF",
            s.drc_blocking.len(), s.drc_waived_density,
            if s.lvs.matched { "MATCH" } else { "MISMATCH" },
            s.lvs.nmos, s.lvs.pmos,
            s.pex.total_resistance("met1"), s.pex.total_cap() / 1000.0,
        )?;
        if !s.lvs.matched { writeln!(f, "  LVS: {}", s.lvs.reason)?; }
        for v in s.drc_blocking.iter().take(10) {
            writeln!(f, "  DRC {}: {} measured {} < {} at ({}, {})",
                v.rule_id, v.kind, v.measured, v.limit, v.x, v.y)?;
        }
        for c in &s.contracts {
            writeln!(f, "  budget {}: {:?}{}",
                c.constraint_id, c.status,
                c.violation_metric.map_or(String::new(), |m|
                    format!(" ({m:.2} {})", c.violation_units.as_deref().unwrap_or(""))))?;
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
                if t > best { best = t; }
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
            return Err(format!("cell `{}` is a macro; macro geometry not wired yet", c.name));
        };
        let ref_dev = devices[0];
        let def = pdk.device(&c.model)
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
            let mut b = CellBuilder::new(deck, tier, i as u32);
            gen.generate(&mut b).map_err(|e| format!("generate `{}`: {e:?}", c.name))?;
            variants.push(b.finish());
        }
        cells.push(GenCell { variants, device_type: Some(ref_dev.device_type) });
    }
    Ok(cells)
}

fn pin_accesses(out: &CellOutput, cell_name: &str, pin_name: &str) -> Vec<(LayerId, Bbox)> {
    let want = format!("{cell_name}:{pin_name}");
    // Grouped cells: pin_name is "XM1.S" but cell generator names it "XM1:S"
    let alt = pin_name.replacen('.', ":", 1);
    let mut hits: Vec<(LayerId, Bbox)> = out.pins.iter()
        .filter(|p| p.name == want || p.name == alt)
        .map(|p| (p.layer, Bbox { xmin: p.x, ymin: p.y, xmax: p.x + p.w, ymax: p.y + p.h }))
        .collect();
    hits.sort_by_key(|(_, b)| (b.xmin, b.ymin));
    hits.dedup_by_key(|(_, b)| (b.xmin, b.ymin));
    hits
}

// ═══════════════════════════════════════════════════════════════════════
//  Geometry helpers
// ═══════════════════════════════════════════════════════════════════════

fn snap5(v: i32) -> i32 { (v / 5) * 5 }

fn translate_into(
    dst: &mut GeometryStore, src: &GeometryStore,
    dx: i32, dy: i32, orient: Orientation, cw: i32, ch: i32,
) {
    for p in 0..src.poly_count() as u32 {
        let (start, end) = src.poly_range(PolyId(p));
        let pts: Vec<(i32, i32)> = (0..end - start).map(|i| {
            let (x, y) = src.poly_vertex(start, i);
            let (x, y) = orient.transform(x, y, cw, ch);
            (x + dx, y + dy)
        }).collect();
        dst.add_polygon(src.poly_layer[p as usize], &pts);
    }
}

fn centered_square(store: &mut GeometryStore, layer: LayerId, x: i32, y: i32, side: i32) {
    store.add_rect(layer, x - side / 2, y - side / 2, side, side);
}

struct PinStack {
    terms: Vec<(i32, i32)>,
    accesses: Vec<(i32, i32)>,
    on_poly: bool,
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
            if mp.tier < pnr_constraints::MatchingTier::Moderate { continue; }
            let (a, b) = (mp.device_a.0, mp.device_b.0);
            if already.contains(&a) || already.contains(&b) { continue; }
            if g.cells[a as usize].model != g.cells[b as usize].model { continue; }
            if g.cells[a as usize].device.is_none() { continue; }
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
        sg.pairs.retain(|mp| remap_id(mp.device_a).0 != remap_id(mp.device_b).0);
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
    for p in &mut rec2.proximity { p.device_a = remap_id(p.device_a); p.device_b = remap_id(p.device_b); }
    for iso in &mut rec2.isolation { iso.device_a = remap_id(iso.device_a); iso.device_b = remap_id(iso.device_b); }
    Ok((rec2, remap))
}

// ═══════════════════════════════════════════════════════════════════════
//  Constraint partitioning for sub-blocks
// ═══════════════════════════════════════════════════════════════════════

fn partition_constraints(
    rec: &ConstraintRecord,
    cell_indices: &[u32],
) -> ConstraintRecord {
    let set: HashSet<u32> = cell_indices.iter().copied().collect();
    let idx_map: HashMap<u32, u32> = cell_indices.iter().enumerate()
        .map(|(new, &old)| (old, new as u32)).collect();
    let remap = |id: DeviceId| -> Option<DeviceId> {
        idx_map.get(&id.0).map(|&new| DeviceId(new))
    };

    let mut out = ConstraintRecord::default();

    for sg in &rec.symmetry {
        let pairs: Vec<_> = sg.pairs.iter().filter_map(|mp| {
            let a = remap(mp.device_a)?;
            let b = remap(mp.device_b)?;
            Some(pnr_constraints::MatchingPair { device_a: a, device_b: b, ..mp.clone() })
        }).collect();
        let self_sym: Vec<_> = sg.self_symmetric.iter().filter_map(|&d| remap(d)).collect();
        if !pairs.is_empty() || !self_sym.is_empty() {
            out.symmetry.push(pnr_constraints::SymmetryGroup {
                group_id: sg.group_id.clone(),
                axis: sg.axis,
                pairs,
                self_symmetric: self_sym,
            });
        }
    }
    for cc in &rec.cc {
        let ga: Vec<_> = cc.group_a.iter().filter_map(|&d| remap(d)).collect();
        let gb: Vec<_> = cc.group_b.iter().filter_map(|&d| remap(d)).collect();
        if !ga.is_empty() && !gb.is_empty() {
            out.cc.push(pnr_constraints::CcGroup { group_a: ga, group_b: gb, ..cc.clone() });
        }
    }
    for p in &rec.proximity {
        if let (Some(a), Some(b)) = (remap(p.device_a), remap(p.device_b)) {
            out.proximity.push(pnr_constraints::ProximityRule { device_a: a, device_b: b, ..p.clone() });
        }
    }
    for iso in &rec.isolation {
        if let (Some(a), Some(b)) = (remap(iso.device_a), remap(iso.device_b)) {
            out.isolation.push(pnr_constraints::IsolationConstraint { device_a: a, device_b: b, ..iso.clone() });
        }
    }
    // Net-level constraints (thermal, net_class, crosstalk, straight, parasitic) pass through
    out.thermal = rec.thermal.clone();
    out.net_class = rec.net_class.clone();
    out.crosstalk = rec.crosstalk.clone();
    out.straight = rec.straight.clone();
    out.parasitic = rec.parasitic.clone();
    out
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
}

/// Build a snapshot GeometryStore for the probe from mid-iteration state.
#[cfg(feature = "visualizer")]
fn probe_snapshot(
    gen: &[GenCell],
    placed: &PlacementResult,
    routed: &RoutingResult,
    trans: &[(i32, i32)],
    deck: &Deck,
) -> GeometryStore {
    let lt = &deck.layers;
    let met1 = lt.id("met1").unwrap_or(0);
    let met2 = lt.id("met2").unwrap_or(1);
    let mut store = GeometryStore::new();
    let vi = &placed.placement.variant;
    for (i, c) in gen.iter().enumerate() {
        let chosen = &c.variants[vi[i]];
        let (dx, dy) = trans[i];
        let b = &chosen.bbox;
        translate_into(&mut store, &chosen.store, dx, dy, Orientation::default(), b.width(), b.height());
    }
    for w in &routed.wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        let layer = if w.layer == 0 { met1 } else { met2 };
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
    let poly = lt.id("poly").ok_or("deck missing layer `poly`")?;

    let mut pcfg = cfg.placement.clone();
    if pcfg.debug_dir.is_none() { pcfg.debug_dir.clone_from(&cfg.debug_dir); }
    let mut rcfg_init = cfg.routing.clone();
    if rcfg_init.debug_dir.is_none() { rcfg_init.debug_dir.clone_from(&cfg.debug_dir); }
    // ponytail: RefCell lets extract closure write net_priority_overrides for next route iteration
    let rcfg = std::cell::RefCell::new(rcfg_init);

    #[cfg(feature = "visualizer")]
    let vis_frame = std::cell::Cell::new(0u32);

    let block_cfg = pnr_engine::block::BlockConfig {
        max_iters: cfg.max_feedback_iters,
        feedback_threshold: cfg.feedback_threshold,
    };

    // Engine drives: cells(+hints) → place(+weights) → route → feedback
    let result = pnr_engine::block::run_block(
        &block_cfg,
        // cells: generate/select cell variants (hints ignored until cells/ refactor)
        |_iter, _hints| {
            let gen = generate_cells(g, pdk, deck, cell_pdk, rec).unwrap();
            let sizes: Vec<(i32, i32)> = gen.iter()
                .map(|c| { let b = &c.variants[0].bbox; (b.width(), b.height()) }).collect();
            let variant_sizes: Vec<Vec<(i32, i32)>> = gen.iter()
                .map(|c| c.variants.iter().map(|v| (v.bbox.width(), v.bbox.height())).collect()).collect();
            let lmasks: Vec<u64> = gen.iter()
                .map(|c| c.variants[0].layer_mask()).collect();
            (gen, sizes, variant_sizes, lmasks)
        },
        // place: run placement with feedback weights and directional inflation
        |_iter, cells_out, net_weights, cell_inflation_x, cell_inflation_y, constraint_adj| {
            let (ref gen, ref sizes, ref variant_sizes, ref lmasks) = *cells_out;
            let mut pc = pcfg.clone();
            pc.variant_sizes = variant_sizes.clone();
            for &(ref name, w) in net_weights { pc.net_weight_overrides.insert(name.clone(), w); }
            let iter_sizes: Vec<(i32, i32)> = if cell_inflation_x.is_empty() && cell_inflation_y.is_empty() {
                sizes.clone()
            } else {
                sizes.iter().enumerate().map(|(i, &(w, h))| {
                    let fx = cell_inflation_x.get(i).copied().unwrap_or(1.0);
                    let fy = cell_inflation_y.get(i).copied().unwrap_or(1.0);
                    ((w as f64 * fx) as i32, (h as f64 * fy) as i32)
                }).collect()
            };
            // ponytail: inject isolation constraints from crosstalk violations
            let mut rec_iter = rec.clone();
            for (dev_a, dev_b, gap_nm) in constraint_adj {
                let id_a = g.cell_id(dev_a);
                let id_b = g.cell_id(dev_b);
                if let (Some(a), Some(b)) = (id_a, id_b) {
                    let gap_um = *gap_nm / 1000.0;
                    // increase existing isolation or add new one
                    if let Some(iso) = rec_iter.isolation.iter_mut().find(|iso|
                        (iso.device_a.0 == a && iso.device_b.0 == b) ||
                        (iso.device_a.0 == b && iso.device_b.0 == a))
                    {
                        iso.min_distance_um = iso.min_distance_um.max(iso.min_distance_um + gap_um);
                    } else {
                        rec_iter.isolation.push(pnr_constraints::IsolationConstraint {
                            device_a: pnr_constraints::DeviceId(a),
                            device_b: pnr_constraints::DeviceId(b),
                            min_distance_um: gap_um,
                            requires_guard_ring: false,
                            reason: "crosstalk violation feedback".into(),
                        });
                    }
                }
            }
            run_placement(g, &iter_sizes, &rec_iter, &pc, lmasks)
        },
        // route: compute pin positions from placement + cells, run routing
        |cells_out, placed| {
            let (ref gen, _, _, _) = *cells_out;
            let p = &placed.placement;
            let vi = &p.variant;
            let trans: Vec<(i32, i32)> = (0..g.cells.len()).map(|i| {
                let b = &gen[i].variants[vi[i]].bbox;
                (snap5(p.x[i] - (b.xmin + b.xmax) / 2), snap5(p.y[i] - (b.ymin + b.ymax) / 2))
            }).collect();
            let mut stacks: Vec<PinStack> = Vec::new();
            let mut pin_pos: Vec<Vec<Vec<(i32, i32)>>> = Vec::with_capacity(g.cells.len());
            for (i, c) in g.cells.iter().enumerate() {
                let (dx, dy) = trans[i];
                let chosen = &gen[i].variants[vi[i]];
                let mut row = Vec::with_capacity(c.pins.len());
                for (pn, _) in &c.pins {
                    let accs = pin_accesses(chosen, &c.name, pn);
                    if accs.is_empty() { row.push(Vec::new()); continue; }
                    let on_poly = accs[0].0 == poly;
                    let bb = &chosen.bbox;
                    let hv = 170 / 2 + 5;
                    let centers: Vec<(i32, i32)> = accs.iter().map(|(_, b)| {
                        (snap5(((b.xmin + b.xmax) / 2).clamp(bb.xmin + hv, bb.xmax - hv) + dx),
                         snap5(((b.ymin + b.ymax) / 2).clamp(bb.ymin + hv, bb.ymax - hv) + dy))
                    }).collect();
                    let terms = if on_poly { vec![centers[centers.len() / 2]] } else { centers.clone() };
                    row.push(terms.clone());
                    stacks.push(PinStack { terms, accesses: centers, on_poly });
                }
                pin_pos.push(row);
            }
            let routed = run_routing_at(g, p, Some(&pin_pos), rec, &rcfg.borrow());
            (routed, trans, stacks)
        },
        // extract: pull feedback from completed routing + visualizer probe
        |_cells_out, placed, route_out| {
            let (ref routed, ref _trans, _) = *route_out;

            #[cfg(feature = "visualizer")]
            if let Some(ref dir) = cfg.debug_dir {
                let (ref gen, _, _, _) = *_cells_out;
                let snap = probe_snapshot(gen, placed, routed, _trans, deck);
                let frame = vis_frame.get();
                vis_frame.set(frame + 1);
                let path = dir.join("dump.txt");
                let mut f = std::fs::OpenOptions::new()
                    .create(true).append(true).open(&path).unwrap();
                use std::io::Write;
                let _ = writeln!(f, "# frame {} wl={:.1} overuse={}",
                    frame, routed.report.wirelength_nm as f64 / 1000.0, routed.report.overuse);
                for i in 0..snap.poly_count() {
                    let start = snap.poly_vert_start[i] as usize;
                    let len = snap.poly_vert_len[i] as usize;
                    let _ = write!(f, "{}", snap.poly_layer[i]);
                    for j in 0..len {
                        let _ = write!(f, " {},{}", snap.verts_x[start + j], snap.verts_y[start + j]);
                    }
                    let _ = writeln!(f);
                }
            }

            let fb = extract_feedback(routed, &placed.placement, &rec.parasitic,
                cfg.feedback_weight_cap, cfg.feedback_inflation_cap);
            // ponytail: feed net ordering back into routing config for next iteration
            rcfg.borrow_mut().net_priority_overrides = fb.net_order_priority.iter()
                .cloned().collect();
            pnr_engine::block::Feedback {
                net_weights: routed.net_names.iter().enumerate()
                    .map(|(i, n)| (n.clone(), fb.net_weights[i])).collect(),
                cell_inflation_x: fb.cell_inflation_x.clone(),
                cell_inflation_y: fb.cell_inflation_y.clone(),
                cell_hints: Vec::new(),
                max_weight: fb.max_weight, clean: fb.clean,
                // ponytail: crosstalk violations → placement isolation pressure
                constraint_adjustments: routed.crosstalk_violations.clone(),
            }
        },
    );

    let (gen, _, _, _) = result.cells;
    let placed = result.placement;
    let (routed, trans, stacks) = result.routing;
    Ok(BuiltBlock { gen, placed, routed, trans, stacks })
}

// ═══════════════════════════════════════════════════════════════════════
//  Step 4: merge one block's geometry into a store
// ═══════════════════════════════════════════════════════════════════════

fn merge_block_geometry(
    store: &mut GeometryStore,
    block: &BuiltBlock,
    cfg: &FlowConfig,
    deck: &Deck,
    dx_global: i32,
    dy_global: i32,
    labels: &mut Vec<crate::gds::TextLabel>,
    cell_names: &[String],
) -> Result<HashMap<u32, PolyId>, String> {
    let lt = &deck.layers;
    let lid = |name: &str| lt.id(name).ok_or_else(|| format!("deck missing layer `{name}`"));
    let (met1, met2, via1, mcon, licon) =
        (lid("met1")?, lid("met2")?, lid("via1")?, lid("mcon")?, lid("licon")?);
    let li = lid("li")?;
    let nsdm = lt.id("nsdm");
    let psdm = lt.id("psdm");
    let nwell = lt.id("nwell");

    // ponytail: 236/0 is a common annotation layer, won't collide with sky130 physical layers
    const LABEL_LAYER: i16 = 236;
    const LABEL_DATATYPE: i16 = 0;

    let vi = &block.placed.placement.variant;
    for (i, c) in block.gen.iter().enumerate() {
        let chosen = &c.variants[vi[i]];
        let (dx, dy) = (block.trans[i].0 + dx_global, block.trans[i].1 + dy_global);
        let b = &chosen.bbox;
        translate_into(store, &chosen.store, dx, dy, Orientation::default(), b.width(), b.height());
        let implant = match c.device_type { Some(DeviceType::Nmos) => nsdm, Some(DeviceType::Pmos) => psdm, _ => None };
        if let Some(l) = implant { store.add_rect(l, b.xmin + dx, b.ymin + dy, b.width(), b.height()); }
        if c.device_type == Some(DeviceType::Pmos) {
            if let Some(l) = nwell { store.add_rect(l, b.xmin + dx, b.ymin + dy, b.width(), b.height()); }
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

    let mut net_sample: HashMap<u32, PolyId> = HashMap::new();
    for w in &block.routed.wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        let layer = if w.layer == 0 { met1 } else { met2 };
        let p = store.add_rect(layer, x0 + dx_global, y0 + dy_global, x1 - x0, y1 - y0);
        net_sample.entry(w.net).or_insert(p);
    }
    for v in &block.routed.vias {
        let (vx, vy) = (v.x + dx_global, v.y + dy_global);
        centered_square(store, via1, vx, vy, cfg.via_size);
        centered_square(store, met1, vx, vy, cfg.pad);
        centered_square(store, met2, vx, vy, cfg.pad);
    }

    for s in &block.stacks {
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
        let mut terms: Vec<(i32, i32)> = s.terms.iter().map(|&(x, y)| (x + dx_global, y + dy_global)).collect();
        terms.sort_unstable_by_key(|&(x, y)| (y, x));
        let mut k = 0;
        while k < terms.len() {
            let (y, x0) = (terms[k].1, terms[k].0);
            let mut x1 = x0;
            while k + 1 < terms.len() && terms[k + 1].1 == y && terms[k + 1].0 - x1 < cfg.pad + 140 {
                k += 1; x1 = terms[k].0;
            }
            store.add_rect(met1, x0 - cfg.pad / 2, y - cfg.pad / 2, x1 - x0 + cfg.pad, cfg.pad);
            k += 1;
        }
    }

    for l in &block.routed.landings {
        let (px, py) = (l.pin.0 + dx_global, l.pin.1 + dy_global);
        let (nx, ny) = (l.node.0 + dx_global, l.node.1 + dy_global);
        let hw = cfg.pad / 2;
        if l.layer == 1 {
            // met2 landing: via at pin, pad at pin, L-shaped wire to node.
            centered_square(store, via1, px, py, cfg.via_size);
            centered_square(store, met2, px, py, cfg.pad);
            if px == nx && py == ny {
                // coincident — pad already covers it
            } else if px == nx || py == ny {
                // axis-aligned — single segment
                store.add_rect(met2, px.min(nx) - hw, py.min(ny) - hw,
                    (px - nx).abs() + cfg.pad, (py - ny).abs() + cfg.pad);
            } else {
                // L-shape: met2 vertical-preferred — vertical from pin to
                // node's y, then horizontal to node's x.
                store.add_rect(met2, px - hw, py.min(ny) - hw,
                    cfg.pad, (py - ny).abs() + cfg.pad);
                store.add_rect(met2, px.min(nx) - hw, ny - hw,
                    (px - nx).abs() + cfg.pad, cfg.pad);
            }
        } else {
            // met1 landing: pad at node, L-shaped wire to node.
            centered_square(store, met1, nx, ny, cfg.pad);
            if (px - nx).abs() >= cfg.pad || (py - ny).abs() >= cfg.pad {
                if px == nx || py == ny {
                    // axis-aligned — single segment
                    store.add_rect(met1, px.min(nx) - hw, py.min(ny) - hw,
                        (px - nx).abs() + cfg.pad, (py - ny).abs() + cfg.pad);
                } else {
                    // L-shape: met1 horizontal-preferred — horizontal from
                    // pin to node's x, then vertical to node's y.
                    store.add_rect(met1, px.min(nx) - hw, py - hw,
                        (px - nx).abs() + cfg.pad, cfg.pad);
                    store.add_rect(met1, nx - hw, py.min(ny) - hw,
                        cfg.pad, (py - ny).abs() + cfg.pad);
                }
            }
        }
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
) -> SignoffReport {
    let drc = run_drc(store, deck);
    let (density, blocking): (Vec<&Violation>, Vec<&Violation>) = drc.violations.iter()
        .partition(|v| v.kind == "min_density" || v.kind == "max_density");
    let drc_blocking: Vec<Violation> = blocking.into_iter().cloned().collect();
    let drc_waived_density = density.len();

    let reference = reference_netlist(g);
    let ext = match extract_netlist_opts(store, deck,
        &ExtractOpts { cut_required: deck.lvs_cut_required, ..Default::default() }, Backend::Cpu)
    {
        Ok(e) => e,
        Err(e) => return SignoffReport {
            drc, drc_blocking, drc_waived_density,
            lvs: LvsResult { matched: false, reason: format!("extraction failed: {e}"),
                mismatches: Vec::new(), extracted_devices: 0, nmos: 0, pmos: 0,
                ambiguous_classes: 0, label_conflicts: Vec::new(), floating_nets: Vec::new() },
            pex: PexReport { parasitics: Vec::new() },
            contracts: Vec::new(),
        },
    };
    let cmp_opts = CompareOpts {
        strict: false,
        w_tolerance: deck.w_tolerance.clone(),
        l_tolerance: deck.l_tolerance.clone(),
    };
    let lvs: LvsResult = compare(&ext, &reference, &cmp_opts);
    let pex: PexReport = gdsverify::run_pex(store, deck);

    let mut contracts = Vec::new();
    if !rec.parasitic.is_empty() {
        let per_net = run_pex_by_net(store, deck, &ext.net_of_poly);
        let names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        for b in &rec.parasitic {
            let mut contract = b.to_contract(&names);
            let routed_net = routed.net_names.iter().position(|n| n == &b.net_name);
            let parasitics = routed_net
                .and_then(|ni| net_sample.get(&(ni as u32)))
                .map(|&p| ext.net_of_poly[p.0 as usize])
                .and_then(|net| per_net.get(&net));
            if let Some(np) = parasitics {
                contract.consume("routing");
                let cap_ff = np.cap_af / 1000.0;
                if np.r_ohm <= b.max_r && cap_ff <= b.max_c {
                    contract.satisfy("routing",
                        &format!("R {:.2} ohm <= {:.2}, C {:.2} fF <= {:.2}", np.r_ohm, b.max_r, cap_ff, b.max_c));
                } else if np.r_ohm > b.max_r {
                    contract.violate("routing", np.r_ohm - b.max_r, "ohm over budget");
                } else {
                    contract.violate("routing", cap_ff - b.max_c, "fF over budget");
                }
            }
            contracts.push(contract);
        }
    }

    SignoffReport { drc, drc_blocking, drc_waived_density, lvs, pex, contracts }
}

fn reference_netlist(g: &BipartiteHypergraph) -> RefNetlist {
    let mut devices = Vec::new();
    for c in &g.cells {
        let dev_list: Vec<&DeviceRecord> = if !c.grouped_devices.is_empty() {
            c.grouped_devices.iter().collect()
        } else if let Some(d) = &c.device { vec![d] }
        else { continue; };
        for d in dev_list {
            let kind = match d.device_type {
                DeviceType::Nmos => DeviceKind::Nmos, DeviceType::Pmos => DeviceKind::Pmos,
                _ => continue,
            };
            let net = |t: &str| -> String {
                let grouped_key = format!("{}.{t}", d.name);
                c.pins.iter().find(|(p, _)| p == t || p == &grouped_key)
                    .map_or(String::new(), |(_, n)| g.nets[*n as usize].clone())
            };
            // ponytail: emit one reduced device — extractor's reduce_netlist
            // merges parallel fingers/instances, so the reference must match
            let m = i32::from(d.multiplier.max(1));
            devices.push(RefDevice { kind: kind.clone(), gate: net("G"),
                source: net("S"), drain: net("D"), w: d.w * m, l: d.l,
                flavor: DeviceFlavor::Standard });
        }
    }
    RefNetlist { devices, net_seeds: std::collections::HashMap::new(), ref_two_terminal: Vec::new(), ref_bjt: Vec::new() }
}

// ═══════════════════════════════════════════════════════════════════════
//  run_flow — the 5-step orchestrator
// ═══════════════════════════════════════════════════════════════════════

#[allow(clippy::missing_errors_doc)]
pub fn run_flow(
    spice: &str,
    deck_json: &str,
    rec: &ConstraintRecord,
    cfg: &FlowConfig,
) -> Result<FlowResult, String> {
    // ── 1. Parse ──
    let full = crate::pdk::load_pdk(deck_json)?;
    let mut g = crate::netlist::parse_spice(spice, &full.netlist, &HashSet::new())
        .map_err(|e| e.to_string())?;
    if g.cells.is_empty() {
        return Err("no PDK devices resolved from netlist".into());
    }

    // ── 2. Annotate ──
    let g_pre = g.clone();
    let ann = pnr_annotator::annotate(&mut g, &pnr_annotator::AnnotationConfig::default())?;

    // ── 2b. Truncate visualizer dump ──
    #[cfg(feature = "visualizer")]
    if let Some(ref dir) = cfg.debug_dir {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join("dump.txt"), "");
    }

    // ── 3. Build blocks ──
    let blocks: Vec<(BuiltBlock, BipartiteHypergraph)>;

    // ponytail: multi-block disabled — no inter-block routing yet, causes LVS net splits
    if true || ann.chain.blocks.is_empty() || ann.chain.blocks.iter().all(|b| b.cell_indices.len() >= g_pre.cells.len()) {
        // No sub-blocks recognized (or single block spans everything) → single-block path
        let mut g_single = g_pre.clone();
        let (rec_final, _) = group_matched_pairs(&mut g_single, rec)?;
        let built = build_block(&g_single, &full.netlist, &full.deck, &full.cells, &rec_final, cfg)?;
        blocks = vec![(built, g_single)];
    } else {
        // Multi-block: build each recognized block + unclaimed cells
        let mut v = Vec::new();

        for block_node in &ann.chain.blocks {
            let (mut sub_hg, _) = g_pre.extract_subgraph(&block_node.cell_indices);
            let sub_rec = partition_constraints(rec, &block_node.cell_indices);
            let (sub_rec, _) = group_matched_pairs(&mut sub_hg, &sub_rec)?;
            let built = build_block(&sub_hg, &full.netlist, &full.deck, &full.cells, &sub_rec, cfg)?;
            v.push((built, sub_hg));
        }

        if !ann.chain.unclaimed.is_empty() {
            let (mut sub_hg, _) = g_pre.extract_subgraph(&ann.chain.unclaimed);
            let sub_rec = partition_constraints(rec, &ann.chain.unclaimed);
            let (sub_rec, _) = group_matched_pairs(&mut sub_hg, &sub_rec)?;
            let built = build_block(&sub_hg, &full.netlist, &full.deck, &full.cells, &sub_rec, cfg)?;
            v.push((built, sub_hg));
        }

        blocks = v;
    }

    // ── 4. Merge ──
    let mut store = GeometryStore::new();
    let mut all_net_samples: HashMap<u32, PolyId> = HashMap::new();
    let mut labels: Vec<crate::gds::TextLabel> = Vec::new();

    // ponytail: 236/0 annotation layer for block boundary boxes
    const BLOCK_LAYER: i16 = 236;
    const BLOCK_DATATYPE: i16 = 1;

    if blocks.len() > 1 {
        let mut y_cursor: i32 = 0;
        let gap = 2000;
        for (idx, (ref block, ref sub_hg)) in blocks.iter().enumerate() {
            let names: Vec<String> = sub_hg.cells.iter().map(|c| c.name.clone()).collect();
            let ns = merge_block_geometry(&mut store, block, cfg, &full.deck, 0, y_cursor, &mut labels, &names)?;
            for (k, v) in ns { all_net_samples.entry(k).or_insert(v); }
            let die = &block.placed.placement.die;
            let block_name = if idx < ann.chain.blocks.len() {
                format!("{} ({})", ann.chain.blocks[idx].template, ann.chain.blocks[idx].instance_names.join(", "))
            } else {
                "unclaimed".to_string()
            };
            labels.push(crate::gds::TextLabel {
                x: die.0 / 2, y: y_cursor + die.1 / 2,
                layer: BLOCK_LAYER, datatype: BLOCK_DATATYPE,
                text: block_name,
            });
            y_cursor += die.1 + gap;
        }
    } else {
        let names: Vec<String> = blocks[0].1.cells.iter().map(|c| c.name.clone()).collect();
        let ns = merge_block_geometry(&mut store, &blocks[0].0, cfg, &full.deck, 0, 0, &mut labels, &names)?;
        all_net_samples = ns;
    }

    // Destructure to take ownership of primary block's placement/routing
    let (primary_block, _) = blocks.into_iter().next().unwrap();

    // ── 5. Signoff ──
    let signoff = run_signoff(&store, &full.deck, &g_pre, &primary_block.routed, &all_net_samples, rec);

    let mut gds_path = None;
    if let Some(dir) = &cfg.debug_dir {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!("{}.gds", g_pre.name));
        crate::gds::write_gds(&store, &full.deck.layers, &g_pre.name, &path, &labels)
            .map_err(|e| e.to_string())?;
        gds_path = Some(path);
    }

    let result = FlowResult {
        placement: primary_block.placed,
        routing: primary_block.routed,
        store,
        signoff,
        gds_path,
        graph: g_pre,
    };
    if let Some(dir) = &cfg.debug_dir {
        let _ = std::fs::write(dir.join("signoff.txt"), result.to_string());
        let _ = std::fs::write(dir.join("signoff.json"), crate::gds::signoff_json(&result.signoff));
    }
    Ok(result)
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_constraints::{
        ConstraintStatus, DeviceId, MatchingPair, MatchingTier, MatchingType, ParasiticBudget,
        SymmetryGroup,
    };

    const OTA: &str = "\
.subckt ota vinp vinm vout1 vout2 VDD VSS
XM1 vout1 vinp vtail VSS nfet_01v8 W=10u L=1u nf=2
XM2 vout2 vinm vtail VSS nfet_01v8 W=10u L=1u nf=2
XM3 vout1 vbias VDD VDD pfet_01v8 W=20u L=1u
XM4 vout2 vbias VDD VDD pfet_01v8 W=20u L=1u
XM5 vtail vbn VSS VSS nfet_01v8 W=40u L=2u m=4
.ends ota
";

    fn deck_json() -> String {
        std::fs::read_to_string(format!("{}/../../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    fn ota_rec(g: &BipartiteHypergraph) -> ConstraintRecord {
        let id = |n: &str| DeviceId(g.cell_id(n).unwrap());
        ConstraintRecord {
            symmetry: vec![SymmetryGroup {
                group_id: "dp".into(), axis: None,
                pairs: vec![MatchingPair {
                    device_a: id("XM1"), device_b: id("XM2"),
                    matching_type: MatchingType::DiffPair,
                    tier: MatchingTier::Exceptional,
                    max_dvth_mv: 1.0, max_did_pct: 1.0, w_ratio: None,
                }],
                self_symmetric: vec![id("XM5")],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn min_size_devices_sign_off() {
        const MINV: &str = "\
.subckt minv in out mid VDD VSS
XM1 mid in VSS VSS nfet_01v8 W=0.42u L=0.15u
XM2 mid in VDD VDD pfet_01v8 W=0.42u L=0.15u
XR1 mid out res_generic_po W=0.33u L=0.1u
.ends minv
";
        let deck = deck_json();
        let r = run_flow(MINV, &deck, &ConstraintRecord::default(), &FlowConfig::default()).expect("flow");
        println!("{r}");
        assert!(r.routing.report.unrouted.is_empty(), "{:?}", r.routing.report.unrouted);
        assert_eq!(r.routing.report.overuse, 0);
        assert_eq!(r.signoff.drc_blocking.len(), 0,
            "blocking DRC at min size:\n{}", r.signoff.drc_blocking.iter().take(10)
                .map(|v| format!("{} {} measured {} < {} at ({},{})", v.kind, v.layer, v.measured, v.limit, v.x, v.y))
                .collect::<Vec<_>>().join("\n"));
        assert!(r.signoff.lvs.matched, "LVS: {}", r.signoff.lvs.reason);
        assert_eq!((r.signoff.lvs.nmos, r.signoff.lvs.pmos), (1, 1));
    }

    #[test]
    fn ota_full_flow_signs_off() {
        let deck = deck_json();
        let pdk = Pdk::from_json(&deck).unwrap();
        let g = crate::netlist::parse_spice(OTA, &pdk, &HashSet::new()).unwrap();
        let mut rec = ota_rec(&g);
        rec.parasitic.push(ParasiticBudget { net_name: "vtail".into(), max_r: 5_000.0, max_c: 500.0 });

        let dir = std::env::temp_dir().join("pnr_flow_test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = FlowConfig { debug_dir: Some(dir.clone()), ..Default::default() };
        let r = run_flow(OTA, &deck, &rec, &cfg).expect("flow");
        println!("{r}");
        assert!(r.routing.report.unrouted.is_empty(), "{:?}", r.routing.report.unrouted);
        assert_eq!(r.routing.report.overuse, 0, "routing shorts");
        assert!(r.signoff.lvs.matched, "LVS: {}", r.signoff.lvs.reason);
        assert_eq!((r.signoff.lvs.nmos, r.signoff.lvs.pmos), (3, 2));
        assert_eq!(r.signoff.drc_blocking.len(), 0,
            "blocking DRC:\n{}", r.signoff.drc_blocking.iter().take(20)
                .map(|v| format!("{} {} {} measured {} limit {} at ({},{})", v.rule_id, v.kind, v.layer, v.measured, v.limit, v.x, v.y))
                .collect::<Vec<_>>().join("\n"));
        assert!(r.signoff.pex.total_cap() > 0.0);
        assert_eq!(r.signoff.contracts.len(), 1);
        assert_eq!(r.signoff.contracts[0].status, ConstraintStatus::Satisfied,
            "budget contract: {:?} {:?}", r.signoff.contracts[0].status, r.signoff.contracts[0].violation_metric);
        let path = r.gds_path.as_ref().expect("gds written");
        let bytes = std::fs::read(path).unwrap();
        let deck2 = Deck::from_json(&deck).unwrap();
        let layout = gdsverify::read_gds(&bytes, &deck2.layers).expect("gds parses");
        assert_eq!(layout.cells[&r.graph.name].poly_count(), r.store.poly_count());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
