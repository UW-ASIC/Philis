//! pnr-placement — analog placement on top of the pure pnr-engine.
//!
//! `run_placement` takes the frontend's cell/net [`BipartiteHypergraph`], per-
//! cell footprints, and a [`ConstraintRecord`] (category-partitioned views of
//! the pnr-constraints types), runs global (analytical) then detailed
//! (symmetry-preserving SA) placement, closes every consumed
//! `ConstraintContract`, and optionally dumps debug artifacts (cost traces,
//! placement text, contract states) to a directory.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use pnr_cells::netlist::BipartiteHypergraph;
use pnr_cells::DeviceType;
use pnr_constraints::contract::ContractValidation;
pub use pnr_constraints::{ConstraintContract, ConstraintStatus};
#[cfg(test)]
use pnr_constraints::{IsolationConstraint, SymmetryGroup};
use pnr_engine::placement::{self as place, hpwl_pins, total_overlap_eff, Orient};
use pnr_engine::{SplitMix64, Telemetry};

pub mod model;

pub use pnr_constraints::ConstraintRecord;
pub use pnr_engine::placement::{Abutment, DetailedCfg, GlobalCfg, RefinementCfg};

#[derive(Debug, Clone)]
pub struct PlacementConfig {
    pub seed: u64,
    pub utilization: f32,
    pub min_side: i32,
    pub cell_margin: i32,
    /// Reserved routing space between the packed footprint and die boundary.
    /// Zero falls back to `cell_margin`; the integrated flow supplies the
    /// actual routing pitch.
    pub boundary_halo: i32,
    pub grid: i32,
    pub global: GlobalCfg,
    pub detailed: DetailedCfg,
    pub refinement: RefinementCfg,
    pub debug_dir: Option<PathBuf>,
    /// Per-net weight multipliers from routing feedback.
    /// Key = net name, value = multiplier on the base weight (1.0 = no change).
    pub net_weight_overrides: HashMap<String, f64>,
    /// Per-cell variant sizes for the placer to choose from.
    /// `variant_sizes[cell_idx]` = list of `(width, height)` alternatives.
    /// Empty = no variants available (use `sizes` only).
    pub variant_sizes: Vec<Vec<(i32, i32)>>,
    /// Variant to seed for each cell. This is the variant selected by the
    /// previous feedback iteration, not necessarily variant zero.
    pub initial_variants: Vec<usize>,
    /// Per cell/variant pin offsets `(hypergraph_net, dx, dy)` from the cell
    /// center. Detailed placement uses these for variant/orientation-aware HPWL.
    pub variant_pin_offsets: Vec<Vec<Vec<(u32, i32, i32)>>>,
    /// Learned loss per cell/variant from prior routed/signoff outcomes.
    pub variant_penalties: Vec<Vec<f64>>,
    /// Deck-qualified direct-connect transforms.
    pub legal_abutments: Vec<Abutment>,
    /// Extra inter-cluster gap (nm) added by compaction on top of required
    /// gaps — routing slack. Grown by the feedback loop when routing fails.
    pub compaction_slack: f32,
    /// Per-cell soft routing-footprint multipliers. These are deliberately
    /// separate from drawn/variant dimensions so a reshape cannot silently
    /// discard congestion feedback and output geometry remains truthful.
    pub cell_inflation_x: Vec<f64>,
    pub cell_inflation_y: Vec<f64>,
    /// Harness-dictated die (w, h) in nm. Overrides the utilization-derived
    /// canvas AND the post-compaction die shrink: the reported die is exactly
    /// this, with the packed block centered inside it. `None` = adaptive.
    pub fixed_die: Option<(i32, i32)>,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            utilization: 0.4,
            min_side: 12_000,
            // ponytail: 2*halo(430) + 1 track pitch(430) = 1290; rounded to 1300
            cell_margin: 1300,
            boundary_halo: 0,
            grid: 5,
            global: GlobalCfg::default(),
            detailed: DetailedCfg::default(),
            refinement: RefinementCfg::default(),
            debug_dir: None,
            net_weight_overrides: HashMap::new(),
            variant_sizes: Vec::new(),
            initial_variants: Vec::new(),
            variant_pin_offsets: Vec::new(),
            variant_penalties: Vec::new(),
            legal_abutments: Vec::new(),
            compaction_slack: 0.0,
            cell_inflation_x: Vec::new(),
            cell_inflation_y: Vec::new(),
            fixed_die: None,
        }
    }
}

/// Final placement: cell centers in nm on the manufacturing grid.
#[derive(Debug, Clone)]
pub struct Placement {
    pub x: Vec<i32>,
    pub y: Vec<i32>,
    pub sizes: Vec<(i32, i32)>,
    pub die: (i32, i32),
    /// Uniform inter-cell planning margin used to size the placement canvas.
    /// The drawn-area and planning-footprint utilization metrics intentionally
    /// remain distinct.
    pub cell_margin: i32,
    /// Symmetry-axis x per group, nm.
    pub axes: Vec<i32>,
    /// Chosen variant index per cell (indexes into `PlacementConfig::variant_sizes`).
    /// All zeros when the placer has no variant choice.
    pub variant: Vec<usize>,
    /// Per-cell orientation from placement (N for normal, FN for mirror partners).
    pub orient: Vec<Orient>,
}

pub struct PlacementReport {
    pub global: Telemetry,
    pub detailed: Telemetry,
    pub hpwl_initial: f64,
    pub hpwl_final: f64,
    pub overlap_final: f64,
    pub validation: ContractValidation,
    pub contract_lines: String,
    /// Per-constraint contracts from placement, for per-type satisfaction reporting.
    pub contracts: Vec<ConstraintContract>,
}

impl fmt::Display for PlacementReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "placement report")?;
        writeln!(
            f,
            "  global:   {} iters, cost {:.0} -> {:.0}, overflow {:.3}",
            self.global.iters, self.global.initial_cost, self.global.cost, self.global.overflow
        )?;
        writeln!(
            f,
            "  detailed: {} iters, {}/{} moves accepted, cost {:.0} -> {:.0}",
            self.detailed.iters,
            self.detailed.accepted,
            self.detailed.proposed,
            self.detailed.initial_cost,
            self.detailed.cost
        )?;
        writeln!(
            f,
            "  HPWL {:.0} -> {:.0} nm",
            self.hpwl_initial, self.hpwl_final
        )?;
        writeln!(f, "  residual overlap {:.0} nm^2", self.overlap_final)?;
        let v = &self.validation;
        writeln!(
            f,
            "  contracts: {} total | {} satisfied, {} violated, {} waived, {} unconsumed",
            v.total, v.satisfied, v.violated, v.waived, v.emitted
        )?;
        if !v.hard_violations.is_empty() {
            writeln!(f, "  HARD VIOLATIONS: {}", v.hard_violations.join(", "))?;
        }
        Ok(())
    }
}

pub struct PlacementResult {
    pub placement: Placement,
    pub report: PlacementReport,
}

impl PlacementResult {
    /// Write artifacts for this owned result. The flow calls this again after
    /// best-candidate selection so iterative runs cannot leave last-iteration
    /// files beside a best-iteration signoff report.
    pub fn write_debug(
        &self,
        dir: &std::path::Path,
        g: &BipartiteHypergraph,
    ) -> std::io::Result<()> {
        dump_debug(dir, g, &self.placement, &self.report)
    }
}

/// Crude footprint estimator from the SPICE device record.
pub fn estimate_sizes(g: &BipartiteHypergraph) -> Vec<(i32, i32)> {
    g.cells
        .iter()
        .map(|c| match &c.device {
            Some(d) => {
                let nf = i32::from(d.nf.max(1));
                let m = i32::from(d.multiplier.max(1));
                match d.device_type {
                    DeviceType::Res => (d.w + 400, d.l + 800),
                    DeviceType::Cap | DeviceType::Ncap | DeviceType::Pcap => (d.w + 400, d.l + 400),
                    // ponytail: fingers side by side (folding wide devices to
                    // <=5um fingers), multiplier stacks rows
                    _ => {
                        let fw = (d.w / nf).clamp(100, 5000);
                        let nf_eff = (d.w + fw - 1) / fw;
                        (nf_eff * (d.l + 460) + 460, (fw + 700) * m)
                    }
                }
            }
            None => (2000 + 300 * c.pins.len() as i32, 3000),
        })
        .collect()
}

/// The whole placement flow: build views -> global -> detailed -> snap ->
/// reconcile contracts -> report (+ optional debug dump).
pub fn run_placement(
    g: &BipartiteHypergraph,
    sizes: &[(i32, i32)],
    rec: &ConstraintRecord,
    cfg: &PlacementConfig,
    layer_masks: &[u64],
) -> PlacementResult {
    assert_eq!(g.cells.len(), sizes.len(), "one size per hypergraph cell");
    let n = g.cells.len();
    let mut rng = SplitMix64::new(cfg.seed);

    let m = f64::from(cfg.cell_margin.max(0));
    let total: f64 = sizes
        .iter()
        .map(|&(w, h)| (f64::from(w) + m) * (f64::from(h) + m))
        .sum();
    let utilization = f64::from(cfg.utilization.clamp(0.05, 0.95));
    let area_side = (total / utilization).sqrt().ceil() as i32;
    let largest_footprint = sizes
        .iter()
        .map(|&(w, h)| (f64::from(w) + m).max(f64::from(h) + m).ceil() as i32)
        .max()
        .unwrap_or(1000);
    // The old `2 * largest raw device` guard dominated small blocks and made
    // the utilization target ineffective. One inflated footprint is the real
    // geometric lower bound; failed routing probes lower utilization and grow
    // the canvas through the feedback controller.
    let side = area_side.max(largest_footprint).max(cfg.min_side);
    let side = (side + cfg.grid - 1) / cfg.grid * cfg.grid;
    let die = match cfg.fixed_die {
        // Constrained interface: the harness dictates the canvas; never grown.
        Some((w, h)) => (w as f32, h as f32),
        None => (side as f32, side as f32),
    };

    let mut cold = model::build_cold(
        g,
        sizes,
        rec,
        die,
        cfg.grid as f32,
        cfg.cell_margin,
        &cfg.net_weight_overrides,
        layer_masks,
        &cfg.variant_sizes,
        &cfg.initial_variants,
        &cfg.variant_pin_offsets,
        &cfg.variant_penalties,
        &cfg.legal_abutments,
        &cfg.cell_inflation_x,
        &cfg.cell_inflation_y,
    );
    let mut ledger = model::PlaceLedger::build(g, rec);
    let mut hot = place::initial_state(&cold, &mut rng);
    let hpwl_initial = hpwl_pins(&cold, &hot);

    eprintln!("[placement] constraints: {} sym groups, {} CC groups, {} pulls, {} pushes, {} aligns, {} stress, {} DTI zones",
        cold.sym.groups.len(), cold.cc_count(), cold.pulls.len(), cold.pushes.len(),
        cold.aligns.len(), cold.stress.len(), cold.dti_zones.len());

    eprintln!("[placement] stage 1/3: global (analytical descent)");
    let gt = place::run_global(&mut hot, &cold, &mut ledger, &cfg.global, &mut rng);
    eprintln!(
        "[placement] stage 1/3 done: {} iters, cost {:.0} → {:.0}",
        gt.iters, gt.initial_cost, gt.cost
    );

    eprintln!("[placement] stage 2/3: detailed (symmetry-preserving SA)");
    let dt = place::run_detailed(&mut hot, &cold, &mut ledger, &cfg.detailed, &mut rng);
    eprintln!(
        "[placement] stage 2/3 done: {} iters, cost {:.0} → {:.0}",
        dt.iters, dt.initial_cost, dt.cost
    );

    eprintln!("[placement] stage 3/3: refinement (low-temp SA, boosted constraints)");
    let rt = place::run_refinement(&mut hot, &cold, &mut ledger, &cfg.refinement, &mut rng);
    eprintln!(
        "[placement] stage 3/3 done: {} iters, cost {:.0} → {:.0}",
        rt.iters, rt.initial_cost, rt.cost
    );

    // Snap: axes to grid, then cells; mirror partners derived from the snapped
    // axis so symmetry survives quantization exactly.
    let grid = cfg.grid as f32;
    let snap = |v: f32| (v / grid).round() * grid;
    for a in &mut hot.axis {
        *a = snap(*a);
    }
    for i in 0..n {
        hot.x[i] = snap(hot.x[i]);
        hot.y[i] = snap(hot.y[i]);
    }
    for (gi, members) in cold.sym.groups.iter().enumerate() {
        let ax = hot.axis[gi];
        for &m in members {
            let mi = m as usize;
            let p = cold.sym.partner[mi];
            if p == m {
                hot.x[mi] = ax;
            } else if p > m {
                let pi = p as usize;
                hot.x[pi] = 2.0 * ax - hot.x[mi];
                hot.y[pi] = hot.y[mi];
            }
        }
    }
    // Stage 4: constraint-graph compaction — squeeze whitespace, shrink die.
    // Runs BEFORE reconcile so contracts judge the final (compacted) state.
    {
        let before = hot.clone();
        let before_bbox = place::placement_bbox(&before, &cold);
        let before_overlap = total_overlap_eff(&before, &cold);
        let compacted_bbox = place::compact_placement(&mut hot, &cold, cfg.compaction_slack);
        let compacted_overlap = total_overlap_eff(&hot, &cold);
        let bbox_area = |(xmin, ymin, xmax, ymax): (f32, f32, f32, f32)| {
            f64::from((xmax - xmin).max(0.0)) * f64::from((ymax - ymin).max(0.0))
        };
        let before_area = bbox_area(before_bbox);
        let compacted_area = bbox_area(compacted_bbox);
        let expanded = compacted_area > before_area + f64::from(grid * grid);
        let worsened_overlap = compacted_overlap > before_overlap + 0.5;
        let no_material_overlap_gain = compacted_overlap >= before_overlap - 0.5;
        let (minx, miny, maxx, maxy) = if worsened_overlap
            || (expanded && no_material_overlap_gain)
        {
            eprintln!(
                "[placement] compaction rejected: bbox {:.0} -> {:.0} nm², overlap {:.0} -> {:.0} nm²",
                before_area, compacted_area, before_overlap, compacted_overlap
            );
            hot = before;
            before_bbox
        } else {
            compacted_bbox
        };
        // halo: room for routing tracks around the packed block
        let halo = (if cfg.boundary_halo > 0 {
            cfg.boundary_halo
        } else {
            cfg.cell_margin
        }) as f32;
        let halo = halo.max(2.0 * grid);
        let has_fixed_axis = cold.sym.fixed.iter().any(|&f| f);
        // fixed axes are absolute coordinates — no recentering allowed
        let (dx, dy) = if let Some((fw, fh)) = cfg.fixed_die {
            // fixed die: center the packed block, but never inside the halo
            let cx = ((fw as f32 - (maxx - minx)) * 0.5 - minx).max(halo - minx);
            let cy = ((fh as f32 - (maxy - miny)) * 0.5 - miny).max(halo - miny);
            if has_fixed_axis {
                (0.0, snap(cy))
            } else {
                (snap(cx), snap(cy))
            }
        } else if has_fixed_axis {
            (0.0, snap(halo - miny))
        } else {
            (snap(halo - minx), snap(halo - miny))
        };
        for i in 0..n {
            hot.x[i] += dx;
            hot.y[i] += dy;
        }
        for a in &mut hot.axis {
            *a += dx;
        }
        let die_w = snap(if has_fixed_axis {
            maxx + dx + halo
        } else {
            maxx - minx + 2.0 * halo
        });
        let die_h = snap(maxy - miny + 2.0 * halo);
        // The fixed die is a contract, not a fit result: report it verbatim.
        let (die_w, die_h) = match cfg.fixed_die {
            Some((w, h)) => (w as f32, h as f32),
            None => (die_w, die_h),
        };
        eprintln!(
            "[placement] stage 4/4: compaction — die {:.0}x{:.0} → {:.0}x{:.0} nm",
            cold.die.0, cold.die.1, die_w, die_h
        );
        cold.die = (die_w, die_h);
    }
    pnr_engine::Ledger::<place::PlaceDomain>::reconcile(&mut ledger, &hot, &cold);

    {
        let v = ledger.validation();
        eprintln!("[placement] done: HPWL {:.0} → {:.0} nm, overlap {:.0} nm² | contracts {}/{} satisfied, {} violated",
            hpwl_initial, hpwl_pins(&cold, &hot),
            total_overlap_eff(&hot, &cold),
            v.satisfied, v.total, v.violated);
        if !v.hard_violations.is_empty() {
            eprintln!(
                "[placement] HARD VIOLATIONS: {}",
                v.hard_violations.join(", ")
            );
        }
    }

    let report = PlacementReport {
        hpwl_initial,
        hpwl_final: hpwl_pins(&cold, &hot),
        overlap_final: total_overlap_eff(&hot, &cold),
        global: gt,
        detailed: dt,
        validation: ledger.validation(),
        contract_lines: ledger.summary(),
        contracts: std::mem::take(&mut ledger.contracts),
    };

    // Sizes reflect the CHOSEN variant — routing builds obstacles from these;
    // original estimates would mismatch the variant geometry in the GDS.
    let sizes_out: Vec<(i32, i32)> = (0..n)
        .map(|i| {
            let vi = hot.variant_idx.get(i).copied().unwrap_or(0) as usize;
            cfg.variant_sizes
                .get(i)
                .and_then(|vs| vs.get(vi))
                .copied()
                .unwrap_or(sizes[i])
        })
        .collect();
    let placement = Placement {
        x: hot.x.iter().map(|&v| v as i32).collect(),
        y: hot.y.iter().map(|&v| v as i32).collect(),
        sizes: sizes_out,
        die: (cold.die.0 as i32, cold.die.1 as i32),
        cell_margin: cfg.cell_margin,
        axes: hot.axis.iter().map(|&v| v as i32).collect(),
        variant: hot.variant_idx.iter().map(|&v| v as usize).collect(),
        orient: hot.orient.clone(),
    };
    // Routing indexes these arrays in parallel by cell — enforce at the source.
    debug_assert_eq!(placement.x.len(), n);
    debug_assert_eq!(placement.y.len(), n);
    debug_assert_eq!(placement.sizes.len(), n);
    debug_assert_eq!(placement.variant.len(), n);
    debug_assert_eq!(placement.orient.len(), n);

    if let Some(dir) = &cfg.debug_dir {
        if let Err(e) = dump_debug(dir, g, &placement, &report) {
            eprintln!("[placement] debug artifact write failed: {e}");
        }
    }

    PlacementResult { placement, report }
}

fn dump_debug(
    dir: &std::path::Path,
    g: &BipartiteHypergraph,
    p: &Placement,
    r: &PlacementReport,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let w = |name: &str, content: String| -> std::io::Result<()> {
        std::fs::write(dir.join(name), content)
    };
    w("global_trace.csv", r.global.trace_csv())?;
    w("detailed_trace.csv", r.detailed.trace_csv())?;
    w("contracts.txt", r.contract_lines.clone())?;
    w("report.txt", r.to_string())?;
    let mut txt = format!("die {} x {} nm\ncell x y w h\n", p.die.0, p.die.1);
    for (i, c) in g.cells.iter().enumerate() {
        txt.push_str(&format!(
            "{} {} {} {} {}\n",
            c.name, p.x[i], p.y[i], p.sizes[i].0, p.sizes[i].1
        ));
    }
    w("placement.txt", txt)?;
    w("hypergraph.txt", g.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_constraints::{DeviceId, MatchingPair, MatchingTier, MatchingType};

    fn ota_record(g: &BipartiteHypergraph) -> ConstraintRecord {
        let id = |n: &str| DeviceId(g.cell_id(n).unwrap());
        let pair = |a: &str, b: &str, mt| MatchingPair {
            device_a: id(a),
            device_b: id(b),
            matching_type: mt,
            tier: MatchingTier::Exceptional,
            max_dvth_mv: 1.0,
            max_did_pct: 1.0,
            w_ratio: None,
        };
        ConstraintRecord {
            symmetry: vec![SymmetryGroup {
                group_id: "dp".into(),
                axis: None,
                pairs: vec![
                    pair("XM1", "XM2", MatchingType::DiffPair),
                    pair("XM3", "XM4", MatchingType::Mirror),
                ],
                self_symmetric: vec![id("XM5")],
            }],
            ..Default::default()
        }
    }

    fn place_ota() -> (BipartiteHypergraph, PlacementResult) {
        let g = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g);
        let rec = ota_record(&g);
        let cfg = PlacementConfig::default();
        let r = run_placement(&g, &sizes, &rec, &cfg, &[]);
        (g, r)
    }

    #[test]
    fn ota_placement_is_symmetric_legal_and_improves() {
        let (g, r) = place_ota();
        let p = &r.placement;
        let id = |n: &str| g.cell_id(n).unwrap() as usize;
        let ax = p.axes[0];
        let tol = 10;

        for (a, b) in [("XM1", "XM2"), ("XM3", "XM4")] {
            let (a, b) = (id(a), id(b));
            assert!(
                (p.x[a] + p.x[b] - 2 * ax).abs() <= tol,
                "mirror x: {} + {} vs axis {}",
                p.x[a],
                p.x[b],
                ax
            );
            assert!((p.y[a] - p.y[b]).abs() <= tol, "same y");
        }
        assert!((p.x[id("XM5")] - ax).abs() <= tol, "self-symmetric on axis");

        for i in 0..p.x.len() {
            assert!(
                p.x[i] - p.sizes[i].0 / 2 >= -tol && p.x[i] + p.sizes[i].0 / 2 <= p.die.0 + tol
            );
            for j in i + 1..p.x.len() {
                let ox = (p.sizes[i].0 + p.sizes[j].0) / 2 - (p.x[i] - p.x[j]).abs();
                let oy = (p.sizes[i].1 + p.sizes[j].1) / 2 - (p.y[i] - p.y[j]).abs();
                assert!(
                    ox <= tol || oy <= tol,
                    "cells {i} and {j} overlap ({ox} x {oy})"
                );
            }
        }

        let hpwl = |xs: &dyn Fn(usize) -> i64, ys: &dyn Fn(usize) -> i64| -> i64 {
            (0..g.nets.len() as u32)
                .map(|n| {
                    let cells: Vec<usize> =
                        g.pins_on_net(n).iter().map(|&(c, _)| c as usize).collect();
                    if cells.len() < 2 {
                        return 0;
                    }
                    let (mut x0, mut x1, mut y0, mut y1) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
                    for &c in &cells {
                        x0 = x0.min(xs(c));
                        x1 = x1.max(xs(c));
                        y0 = y0.min(ys(c));
                        y1 = y1.max(ys(c));
                    }
                    (x1 - x0) + (y1 - y0)
                })
                .sum()
        };
        let mut row_x = vec![0i64; p.x.len()];
        let mut cursor = 0i64;
        for i in 0..p.x.len() {
            row_x[i] = cursor + i64::from(p.sizes[i].0) / 2;
            cursor += i64::from(p.sizes[i].0);
        }
        let ours = hpwl(&|c| i64::from(p.x[c]), &|c| i64::from(p.y[c]));
        let row = hpwl(&|c| row_x[c], &|_| 0);
        assert!(ours < row, "placed HPWL {ours} should beat naive row {row}");
        assert!(
            r.report.validation.hard_violations.is_empty(),
            "{}",
            r.report.contract_lines
        );
        assert!(r.report.validation.satisfied > 0);
    }

    #[test]
    fn ota_legal_and_compact_across_seeds() {
        // SA + compaction must produce a legal, whitespace-free placement for
        // ANY seed — regressions here were real (hot SA erasing the global
        // solution; compaction leaving overlaps / frozen intra-cluster gaps).
        let g = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g);
        let rec = ota_record(&g);
        let cell_area: i64 = sizes
            .iter()
            .map(|&(w, h)| i64::from(w) * i64::from(h))
            .sum();
        for seed in [1u64, 2, 3, 42, 1337] {
            let cfg = PlacementConfig {
                seed,
                ..Default::default()
            };
            let r = run_placement(&g, &sizes, &rec, &cfg, &[]);
            let p = &r.placement;
            assert!(
                r.report.overlap_final < 1.0,
                "seed {seed}: residual overlap {}",
                r.report.overlap_final
            );
            assert!(
                r.report.validation.hard_violations.is_empty(),
                "seed {seed}: {}",
                r.report.contract_lines
            );
            // die must not balloon: cells + margins fit in a modest envelope
            let die_area = i64::from(p.die.0) * i64::from(p.die.1);
            assert!(
                die_area < 6 * cell_area,
                "seed {seed}: die {}x{} = {die_area} vs cell area {cell_area}",
                p.die.0,
                p.die.1
            );
        }
    }

    #[test]
    fn guarded_quad_never_overlaps() {
        // 4 identical devices, guard-ring spacing to every neighbor: the case
        // where compaction used to shove clusters onto already-placed ones
        // (leading-side-only check) or explode the die (wrong-axis fix).
        let g = pnr_cells::fixtures::quad();
        let sizes = estimate_sizes(&g);
        let mut rec = ConstraintRecord::default();
        for i in 0..4u32 {
            rec.guard_ring.push(pnr_constraints::GuardRingRequirement {
                device_id: DeviceId(i),
                ring_type: pnr_constraints::GuardRingType::PsubRing,
                shareable: false,
                tap_pitch_um: 2.0,
                min_width_um: 0.5,
                max_ring_resistance_ohm: 100.0,
                enclosure_complete: true,
                connection_net: "VSS".into(),
            });
        }
        for seed in [1u64, 2, 3, 42, 1337] {
            let cfg = PlacementConfig {
                seed,
                ..Default::default()
            };
            let r = run_placement(&g, &sizes, &rec, &cfg, &[]);
            let p = &r.placement;
            assert!(
                r.report.overlap_final < 1.0,
                "seed {seed}: residual overlap {}",
                r.report.overlap_final
            );
            // guard gap 1um must hold edge-to-edge for every pair
            for a in 0..4usize {
                for b in a + 1..4 {
                    let gx = (p.x[a] - p.x[b]).abs() - (p.sizes[a].0 + p.sizes[b].0) / 2;
                    let gy = (p.y[a] - p.y[b]).abs() - (p.sizes[a].1 + p.sizes[b].1) / 2;
                    assert!(
                        gx.max(gy) >= 1000 - 10,
                        "seed {seed}: pair {a}/{b} gap {}",
                        gx.max(gy)
                    );
                }
            }
        }
    }

    #[test]
    fn isolation_hard_gap_is_respected() {
        let g = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g);
        let mut rec = ota_record(&g);
        rec.isolation.push(IsolationConstraint {
            device_a: DeviceId(g.cell_id("XM5").unwrap()),
            device_b: DeviceId(g.cell_id("XM3").unwrap()),
            min_distance_um: 3.0,
            requires_guard_ring: false,
            reason: "noisy tail".into(),
        });
        let r = run_placement(&g, &sizes, &rec, &PlacementConfig::default(), &[]);
        let (a, b) = (
            g.cell_id("XM5").unwrap() as usize,
            g.cell_id("XM3").unwrap() as usize,
        );
        let p = &r.placement;
        let gx = (p.x[a] - p.x[b]).abs() - (p.sizes[a].0 + p.sizes[b].0) / 2;
        let gy = (p.y[a] - p.y[b]).abs() - (p.sizes[a].1 + p.sizes[b].1) / 2;
        assert!(gx.max(gy) >= 3000 - 10, "isolation gap {} nm", gx.max(gy));
    }

    #[test]
    fn stress_and_dti_contracts_reconciled() {
        let g = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g);
        let mut rec = ota_record(&g);
        rec.stress.push(pnr_constraints::StressConstraint {
            device_id: DeviceId(g.cell_id("XM5").unwrap()),
            max_centroid_distance_um: 1000.0, // generous: judged, likely satisfied
        });
        rec.dti.push(pnr_constraints::DtiPair {
            device_a: DeviceId(g.cell_id("XM1").unwrap()),
            device_b: DeviceId(g.cell_id("XM3").unwrap()),
            s_max: 0.5,
            d_dti: 2.0,
        });
        let r = run_placement(&g, &sizes, &rec, &PlacementConfig::default(), &[]);
        let judged = r
            .report
            .contracts
            .iter()
            .filter(|c| {
                (c.kind == "stress" || c.kind == "dti")
                    && matches!(
                        c.status(),
                        ConstraintStatus::Satisfied | ConstraintStatus::Violated
                    )
            })
            .count();
        assert_eq!(
            judged, 2,
            "stress + dti contracts must be judged:\n{}",
            r.report.contract_lines
        );
    }

    #[test]
    fn debug_dump_writes_artifacts() {
        let g = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g);
        let rec = ota_record(&g);
        let dir = std::env::temp_dir().join("pnr_placement_debug_test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = PlacementConfig {
            debug_dir: Some(dir.clone()),
            ..Default::default()
        };
        run_placement(&g, &sizes, &rec, &cfg, &[]);
        for f in [
            "global_trace.csv",
            "detailed_trace.csv",
            "placement.txt",
            "contracts.txt",
            "report.txt",
        ] {
            assert!(dir.join(f).exists(), "missing {f}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
