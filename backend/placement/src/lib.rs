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

use pnr_constraints::contract::ContractValidation;
use pnr_constraints::{
    CcGroup, CrosstalkExclusion, GuardRingRequirement, IsolationConstraint, NetClassification,
    ParasiticBudget, ProximityRule, StraightNet, SymmetryGroup, ThermalGradientConstraint,
};
use pnr_cells::netlist::BipartiteHypergraph;
use pnr_cells::DeviceType;
use pnr_engine::placement::{self as place, hpwl, total_overlap};
use pnr_engine::{SplitMix64, Telemetry};

pub mod model;

pub use pnr_engine::placement::{DetailedCfg, GlobalCfg};

/// Category-partitioned constraint views — each field is consumed by exactly
/// one slot family. Populate what you have; `Default` is an unconstrained design.
#[derive(Clone, Default)]
pub struct ConstraintRecord {
    pub symmetry: Vec<SymmetryGroup>,
    pub cc: Vec<CcGroup>,
    pub proximity: Vec<ProximityRule>,
    pub isolation: Vec<IsolationConstraint>,
    pub thermal: Vec<ThermalGradientConstraint>,
    pub net_class: Vec<NetClassification>,
    pub crosstalk: Vec<CrosstalkExclusion>,
    pub straight: Vec<StraightNet>,
    pub parasitic: Vec<ParasiticBudget>,
    pub guard_ring: Vec<GuardRingRequirement>,
}

#[derive(Debug, Clone)]
pub struct PlacementConfig {
    pub seed: u64,
    pub utilization: f32,
    pub min_side: i32,
    pub cell_margin: i32,
    pub grid: i32,
    pub global: GlobalCfg,
    pub detailed: DetailedCfg,
    pub debug_dir: Option<PathBuf>,
    /// Per-net weight multipliers from routing feedback.
    /// Key = net name, value = multiplier on the base weight (1.0 = no change).
    pub net_weight_overrides: HashMap<String, f64>,
    /// Per-cell variant sizes for the placer to choose from.
    /// `variant_sizes[cell_idx]` = list of `(width, height)` alternatives.
    /// Empty = no variants available (use `sizes` only).
    pub variant_sizes: Vec<Vec<(i32, i32)>>,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            utilization: 0.4,
            min_side: 12_000,
            // ponytail: 2*halo(430) + 1 track pitch(430) = 1290; rounded to 1300
            cell_margin: 1300,
            grid: 5,
            global: GlobalCfg::default(),
            detailed: DetailedCfg::default(),
            debug_dir: None,
            net_weight_overrides: HashMap::new(),
            variant_sizes: Vec::new(),
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
    /// Symmetry-axis x per group, nm.
    pub axes: Vec<i32>,
    /// Chosen variant index per cell (indexes into `PlacementConfig::variant_sizes`).
    /// All zeros when the placer has no variant choice.
    pub variant: Vec<usize>,
}

pub struct PlacementReport {
    pub global: Telemetry,
    pub detailed: Telemetry,
    pub hpwl_initial: f64,
    pub hpwl_final: f64,
    pub overlap_final: f64,
    pub validation: ContractValidation,
    pub contract_lines: String,
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
        writeln!(f, "  HPWL {:.0} -> {:.0} nm", self.hpwl_initial, self.hpwl_final)?;
        writeln!(f, "  residual overlap {:.0} nm^2", self.overlap_final)?;
        let v = &self.validation;
        writeln!(
            f,
            "  contracts: {} total | {} satisfied, {} violated, {} waived, {} unconsumed",
            v.total,
            v.satisfied,
            v.violated,
            v.waived,
            v.emitted
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
                    DeviceType::Cap | DeviceType::Ncap | DeviceType::Pcap => {
                        (d.w + 400, d.l + 400)
                    }
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

    let m = f64::from(cfg.cell_margin);
    let total: f64 = sizes
        .iter()
        .map(|&(w, h)| (f64::from(w) + m) * (f64::from(h) + m))
        .sum();
    let side = ((total / f64::from(cfg.utilization)).sqrt() as i32)
        .max(sizes.iter().map(|&(w, h)| w.max(h)).max().unwrap_or(1000) * 2)
        .max(cfg.min_side);
    let side = (side + cfg.grid - 1) / cfg.grid * cfg.grid;
    let die = (side as f32, side as f32);

    let cold = model::build_cold(g, sizes, rec, die, cfg.grid as f32, cfg.cell_margin, &cfg.net_weight_overrides, layer_masks);
    let mut ledger = model::PlaceLedger::build(g, rec);
    let mut hot = place::initial_state(&cold, &mut rng);
    let hpwl_initial = hpwl(&cold, &hot.x, &hot.y);

    let gt = place::run_global(&mut hot, &cold, &mut ledger, &cfg.global, &mut rng);
    let dt = place::run_detailed(&mut hot, &cold, &mut ledger, &cfg.detailed, &mut rng);

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
    pnr_engine::Ledger::<place::PlaceDomain>::reconcile(&mut ledger, &hot, &cold);

    let report = PlacementReport {
        hpwl_initial,
        hpwl_final: hpwl(&cold, &hot.x, &hot.y),
        overlap_final: total_overlap(&cold, &hot.x, &hot.y),
        global: gt,
        detailed: dt,
        validation: ledger.validation(),
        contract_lines: ledger.summary(),
    };

    let placement = Placement {
        x: hot.x.iter().map(|&v| v as i32).collect(),
        y: hot.y.iter().map(|&v| v as i32).collect(),
        sizes: sizes.to_vec(),
        die: (side, side),
        axes: hot.axis.iter().map(|&v| v as i32).collect(),
        // ponytail: variant selection during SA is the upgrade path
        variant: vec![0; n],
    };

    if let Some(dir) = &cfg.debug_dir {
        dump_debug(dir, g, &placement, &report);
    }

    PlacementResult { placement, report }
}

fn dump_debug(
    dir: &std::path::Path,
    g: &BipartiteHypergraph,
    p: &Placement,
    r: &PlacementReport,
) {
    let _ = std::fs::create_dir_all(dir);
    let w = |name: &str, content: String| {
        let _ = std::fs::write(dir.join(name), content);
    };
    w("global_trace.csv", r.global.trace_csv());
    w("detailed_trace.csv", r.detailed.trace_csv());
    w("contracts.txt", r.contract_lines.clone());
    w("report.txt", r.to_string());
    let mut txt = format!("die {} x {} nm\ncell x y w h\n", p.die.0, p.die.1);
    for (i, c) in g.cells.iter().enumerate() {
        txt.push_str(&format!(
            "{} {} {} {} {}\n",
            c.name, p.x[i], p.y[i], p.sizes[i].0, p.sizes[i].1
        ));
    }
    w("placement.txt", txt);
    w("hypergraph.txt", g.to_string());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_constraints::{DeviceId, MatchingPair, MatchingTier, MatchingType};
    use pnr_core::frontend::{parse_spice, Pdk};
    use std::collections::HashSet;

    const OTA: &str = "\
.subckt ota vinp vinm vout1 vout2 VDD VSS
XM1 vout1 vinp vtail VSS nfet_01v8 W=10u L=1u nf=2
XM2 vout2 vinm vtail VSS nfet_01v8 W=10u L=1u nf=2
XM3 vout1 vbias VDD VDD pfet_01v8 W=20u L=1u
XM4 vout2 vbias VDD VDD pfet_01v8 W=20u L=1u
XM5 vtail vbn VSS VSS nfet_01v8 W=40u L=2u m=4
.ends ota
";

    fn pdk() -> Pdk {
        Pdk::from_json(
            r#"{
            "drc": {"off_grid": {"grid": 5}},
            "devices": {
                "nfet_01v8": {"type": "nmos", "cell": "mosfet", "min_w": 420, "min_l": 150},
                "pfet_01v8": {"type": "pmos", "cell": "mosfet", "min_w": 420, "min_l": 150}
            }}"#,
        )
        .unwrap()
    }

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
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
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
                assert!(ox <= tol || oy <= tol, "cells {i} and {j} overlap ({ox} x {oy})");
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
                    let (mut x0, mut x1, mut y0, mut y1) =
                        (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
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
        assert!(r.report.validation.hard_violations.is_empty(), "{}", r.report.contract_lines);
        assert!(r.report.validation.satisfied > 0);
    }

    #[test]
    fn isolation_hard_gap_is_respected() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
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
        let (a, b) = (g.cell_id("XM5").unwrap() as usize, g.cell_id("XM3").unwrap() as usize);
        let p = &r.placement;
        let gx = (p.x[a] - p.x[b]).abs() - (p.sizes[a].0 + p.sizes[b].0) / 2;
        let gy = (p.y[a] - p.y[b]).abs() - (p.sizes[a].1 + p.sizes[b].1) / 2;
        assert!(gx.max(gy) >= 3000 - 10, "isolation gap {} nm", gx.max(gy));
    }

    #[test]
    fn debug_dump_writes_artifacts() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let sizes = estimate_sizes(&g);
        let rec = ota_record(&g);
        let dir = std::env::temp_dir().join("pnr_placement_debug_test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = PlacementConfig { debug_dir: Some(dir.clone()), ..Default::default() };
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
