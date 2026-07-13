//! Tapeout-oriented signoff checks beyond geometric DRC/LVS/PEX.
//!
//! A signoff check has four states.  In particular, [`CheckStatus::NotRun`] and
//! [`CheckStatus::Error`] are not clean.  Electrical analyses need stimuli and
//! foundry-qualified limits; silently substituting defaults would create a
//! dangerous false-clean result.

mod antenna;
mod density_cmp;
mod esd_latchup;
mod power;
mod reliability;

pub use antenna::{
    check_antenna, check_antenna_from_deck, AntennaCollector, AntennaConfig, AntennaDiode,
    AntennaGate, AntennaMeasurement, AntennaNetResult, AntennaReport, AntennaRule,
};
pub use density_cmp::{
    check_density_cmp, CmpModel, DensityCmpConfig, DensityCmpReport, DensityCmpRule,
    DensityWindowResult,
};
pub use esd_latchup::{
    check_esd_latchup, EsdEdge, EsdLatchupConfig, EsdLatchupReport, EsdNode, EsdNodeKind,
    EsdPathRequirement, EsdPathResult, GuardRingEvidence, LatchupSite,
};
pub use power::{
    analyze_electromigration, analyze_ir_drop, solve_power_grid, BranchCurrent,
    ElectromigrationConfig, ElectromigrationReport, EmBranchResult, IrDropConfig, IrDropReport,
    NodeVoltage, PowerEdge, PowerEdgeKind, PowerGrid, PowerNode, PowerSignoffConfig, PowerSolution,
    PowerSolveConfig,
};
pub use reliability::{
    check_reliability, AgingStress, AgingStressResult, ReliabilityConfig, ReliabilityReport,
    ThermalStress, VoltageStress,
};

use crate::geometry::{Bbox, GeometryStore, PolyId};
use crate::params::Deck;

/// Stable identity for every tapeout check in this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignoffCheck {
    Antenna,
    DensityCmp,
    IrDrop,
    Electromigration,
    Reliability,
    EsdLatchup,
}

/// A check is clean only when it ran successfully and found no violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Clean,
    Violations,
    NotRun,
    Error,
}

/// Common violation record used by all signoff analyses.
#[derive(Debug, Clone, PartialEq)]
pub struct SignoffViolation {
    pub check: SignoffCheck,
    pub rule_id: String,
    pub message: String,
    pub location: Option<(i32, i32)>,
    pub measured: Option<f64>,
    pub limit: Option<f64>,
    pub units: String,
}

/// Status and diagnostics common to all typed check reports.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    pub check: SignoffCheck,
    pub status: CheckStatus,
    pub violations: Vec<SignoffViolation>,
    pub diagnostics: Vec<String>,
}

impl CheckReport {
    pub fn clean(check: SignoffCheck) -> Self {
        Self {
            check,
            status: CheckStatus::Clean,
            violations: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn not_run(check: SignoffCheck, reason: impl Into<String>) -> Self {
        Self {
            check,
            status: CheckStatus::NotRun,
            violations: Vec::new(),
            diagnostics: vec![reason.into()],
        }
    }

    pub fn error(check: SignoffCheck, reason: impl Into<String>) -> Self {
        Self {
            check,
            status: CheckStatus::Error,
            violations: Vec::new(),
            diagnostics: vec![reason.into()],
        }
    }

    pub(crate) fn from_violations(
        check: SignoffCheck,
        violations: Vec<SignoffViolation>,
        diagnostics: Vec<String>,
    ) -> Self {
        let status = if violations.is_empty() {
            CheckStatus::Clean
        } else {
            CheckStatus::Violations
        };
        Self {
            check,
            status,
            violations,
            diagnostics,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.status == CheckStatus::Clean
    }

    pub fn is_blocking(&self) -> bool {
        !self.is_clean()
    }
}

/// Inputs for all six requested signoff families.
#[derive(Debug, Clone, Default)]
pub struct SignoffConfig {
    pub antenna: Option<AntennaConfig>,
    pub density_cmp: Option<DensityCmpConfig>,
    pub power: Option<PowerSignoffConfig>,
    pub reliability: Option<ReliabilityConfig>,
    pub esd_latchup: Option<EsdLatchupConfig>,
}

/// One report object for a tapeout gate.  `all_clean()` requires all six checks
/// to have actually run; `NOT_RUN` is therefore blocking by construction.
#[derive(Debug, Clone)]
pub struct SignoffSuiteReport {
    pub antenna: AntennaReport,
    pub density_cmp: DensityCmpReport,
    pub ir_drop: IrDropReport,
    pub electromigration: ElectromigrationReport,
    pub reliability: ReliabilityReport,
    pub esd_latchup: EsdLatchupReport,
}

impl SignoffSuiteReport {
    pub fn checks(&self) -> [&CheckReport; 6] {
        [
            &self.antenna.check,
            &self.density_cmp.check,
            &self.ir_drop.check,
            &self.electromigration.check,
            &self.reliability.check,
            &self.esd_latchup.check,
        ]
    }

    pub fn all_clean(&self) -> bool {
        self.checks().iter().all(|r| r.is_clean())
    }

    pub fn blocking_checks(&self) -> Vec<SignoffCheck> {
        self.checks()
            .iter()
            .filter(|r| r.is_blocking())
            .map(|r| r.check)
            .collect()
    }
}

/// Run the requested signoff suite.  Missing input is reported as `NOT_RUN`.
pub fn run_signoff_suite(
    store: &GeometryStore,
    deck: &Deck,
    config: &SignoffConfig,
) -> SignoffSuiteReport {
    let antenna = config.antenna.as_ref().map_or_else(
        || check_antenna_from_deck(store, deck),
        |c| check_antenna(store, deck, c),
    );
    let density_cmp = config.density_cmp.as_ref().map_or_else(
        || DensityCmpReport::not_run("die boundary and density/CMP rules were not supplied"),
        |c| {
            if let Some(rule) = c
                .rules
                .iter()
                .find(|rule| rule.layer as usize >= deck.layers.id_to_name.len())
            {
                DensityCmpReport {
                    check: CheckReport::error(
                        SignoffCheck::DensityCmp,
                        format!(
                            "density/CMP rule '{}' references unknown layer id {}",
                            rule.id, rule.layer,
                        ),
                    ),
                    windows: Vec::new(),
                }
            } else {
                check_density_cmp(store, c)
            }
        },
    );

    let (ir_drop, electromigration) = match &config.power {
        None => (
            IrDropReport::not_run("power-grid topology and load currents were not supplied"),
            ElectromigrationReport::not_run(
                "power-grid topology and load currents were not supplied",
            ),
        ),
        Some(p) => match solve_power_grid(&p.grid, &p.solver) {
            Err(e) => (
                IrDropReport::error(format!("power-grid solve failed: {e}")),
                ElectromigrationReport::error(format!("power-grid solve failed: {e}")),
            ),
            Ok(solution) => {
                let ir = p.ir_drop.as_ref().map_or_else(
                    || IrDropReport::not_run("IR-drop limits were not supplied"),
                    |c| analyze_ir_drop(&p.grid, &solution, c),
                );
                let em = p.electromigration.as_ref().map_or_else(
                    || ElectromigrationReport::not_run("electromigration limits were not supplied"),
                    |c| analyze_electromigration(&p.grid, &solution, c),
                );
                (ir, em)
            }
        },
    };

    let reliability = config.reliability.as_ref().map_or_else(
        || ReliabilityReport::not_run("reliability stress observations/models were not supplied"),
        check_reliability,
    );
    let esd_latchup = config.esd_latchup.as_ref().map_or_else(
        || EsdLatchupReport::not_run("ESD network and latch-up evidence were not supplied"),
        check_esd_latchup,
    );

    SignoffSuiteReport {
        antenna,
        density_cmp,
        ir_drop,
        electromigration,
        reliability,
        esd_latchup,
    }
}

/// Return a bbox only when `p` is exactly an axis-aligned rectangle.
pub(crate) fn polygon_rect(store: &GeometryStore, p: PolyId) -> Option<Bbox> {
    let (s, e) = store.poly_range(p);
    if e - s != 4 {
        return None;
    }
    let bb = store.poly_bbox[p.0 as usize];
    if bb.width() <= 0 || bb.height() <= 0 {
        return None;
    }
    let mut corners = std::collections::BTreeSet::new();
    for edge in store.edges_of(p) {
        let pt = (edge.x0, edge.y0);
        if pt.0 != bb.xmin && pt.0 != bb.xmax {
            return None;
        }
        if pt.1 != bb.ymin && pt.1 != bb.ymax {
            return None;
        }
        corners.insert(pt);

        // A proper rectangle walks one horizontal or vertical side at a time.
        // Merely seeing the four bbox corners would also accept a self-crossing
        // bow-tie ordering, whose bbox area is not its polygon area.
        let next = (edge.x1, edge.y1);
        if (pt.0 == next.0) == (pt.1 == next.1) {
            return None;
        }
    }
    let bbox_area = i64::from(bb.width()) * i64::from(bb.height());
    if corners.len() == 4 && store.area(p) == bbox_area {
        Some(bb)
    } else {
        None
    }
}

/// Exact union area and perimeter for axis-aligned rectangles.
pub(crate) fn rect_union_metrics(rects: &[Bbox]) -> (f64, f64) {
    if rects.is_empty() {
        return (0.0, 0.0);
    }
    let mut xs: Vec<i32> = rects.iter().flat_map(|r| [r.xmin, r.xmax]).collect();
    let mut ys: Vec<i32> = rects.iter().flat_map(|r| [r.ymin, r.ymax]).collect();
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    if xs.len() < 2 || ys.len() < 2 {
        return (0.0, 0.0);
    }

    let nx = xs.len() - 1;
    let ny = ys.len() - 1;
    let mut covered = vec![false; nx * ny];
    for ix in 0..nx {
        for iy in 0..ny {
            let x0 = xs[ix];
            let x1 = xs[ix + 1];
            let y0 = ys[iy];
            let y1 = ys[iy + 1];
            covered[ix * ny + iy] = rects
                .iter()
                .any(|r| r.xmin <= x0 && r.xmax >= x1 && r.ymin <= y0 && r.ymax >= y1);
        }
    }

    let mut area = 0.0;
    let mut perimeter = 0.0;
    let is_covered = |ix: isize, iy: isize| -> bool {
        ix >= 0
            && iy >= 0
            && (ix as usize) < nx
            && (iy as usize) < ny
            && covered[ix as usize * ny + iy as usize]
    };
    for ix in 0..nx {
        for iy in 0..ny {
            if !covered[ix * ny + iy] {
                continue;
            }
            let dx = f64::from(xs[ix + 1] - xs[ix]);
            let dy = f64::from(ys[iy + 1] - ys[iy]);
            area += dx * dy;
            if !is_covered(ix as isize - 1, iy as isize) {
                perimeter += dy;
            }
            if !is_covered(ix as isize + 1, iy as isize) {
                perimeter += dy;
            }
            if !is_covered(ix as isize, iy as isize - 1) {
                perimeter += dx;
            }
            if !is_covered(ix as isize, iy as isize + 1) {
                perimeter += dx;
            }
        }
    }
    (area, perimeter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CheckStatus, Deck, GeometryStore};

    fn antenna_deck() -> Deck {
        Deck::from_json(
            r#"{
            "layers": {
                "diff": {"layer": 1, "datatype": 0},
                "poly": {"layer": 2, "datatype": 0},
                "nsdm": {"layer": 3, "datatype": 0},
                "met1": {"layer": 4, "datatype": 0},
                "diode": {"layer": 5, "datatype": 0}
            },
            "drc": {},
            "pex": {},
            "connectivity": {
                "conductors": ["diff", "poly", "met1"],
                "intra_layer_touch": true
            },
            "device_recognition": {
                "mos": [{
                    "name": "nmos",
                    "gate_layer": "poly",
                    "channel_layer": "diff",
                    "type_implant": "nsdm",
                    "device_type": "nmos"
                }]
            }
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn signoff_missing_inputs_are_not_clean() {
        let deck = antenna_deck();
        let report = run_signoff_suite(&GeometryStore::new(), &deck, &SignoffConfig::default());
        assert!(!report.all_clean());
        assert_eq!(report.blocking_checks().len(), 6);
        assert!(report
            .checks()
            .iter()
            .all(|r| r.status == CheckStatus::NotRun));

        let invalid_density = run_signoff_suite(
            &GeometryStore::new(),
            &deck,
            &SignoffConfig {
                density_cmp: Some(DensityCmpConfig {
                    die: Bbox {
                        xmin: 0,
                        ymin: 0,
                        xmax: 100,
                        ymax: 100,
                    },
                    include_partial_windows: true,
                    rules: vec![DensityCmpRule {
                        id: "unknown.layer".into(),
                        layer: u16::MAX,
                        window_width: 100,
                        window_height: 100,
                        step_x: 100,
                        step_y: 100,
                        min_density: Some(0.0),
                        max_density: Some(1.0),
                        max_neighbor_delta: None,
                        cmp: None,
                    }],
                }),
                ..Default::default()
            },
        );
        assert_eq!(invalid_density.density_cmp.check.status, CheckStatus::Error);
    }

    #[test]
    fn antenna_ratio_is_checked_per_connected_gate_net() {
        let deck = antenna_deck();
        let diff = deck.layers.id("diff").unwrap();
        let poly = deck.layers.id("poly").unwrap();
        let nsdm = deck.layers.id("nsdm").unwrap();
        let met1 = deck.layers.id("met1").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(diff, 0, 0, 100, 100);
        store.add_rect(poly, 40, -20, 20, 140);
        store.add_rect(nsdm, -10, -10, 120, 120);
        store.add_rect(met1, 40, -20, 1000, 1000);
        let report = check_antenna(
            &store,
            &deck,
            &AntennaConfig {
                cut_required: Some(false),
                rules: vec![AntennaRule {
                    id: "ant.m1".into(),
                    gates: vec![AntennaGate {
                        gate_layer: poly,
                        channel_layer: diff,
                    }],
                    collectors: vec![AntennaCollector {
                        layer: met1,
                        measurement: AntennaMeasurement::Area,
                    }],
                    max_egar: 100.0,
                    diode: None,
                }],
            },
        );
        assert_eq!(report.check.status, CheckStatus::Violations);
        assert_eq!(report.check.violations.len(), 1);
        assert!(report.nets[0].egar > 100.0);
    }

    #[test]
    fn antenna_diode_marker_is_attributed_to_its_conductor_net() {
        let deck = antenna_deck();
        let diff = deck.layers.id("diff").unwrap();
        let poly = deck.layers.id("poly").unwrap();
        let nsdm = deck.layers.id("nsdm").unwrap();
        let met1 = deck.layers.id("met1").unwrap();
        let diode = deck.layers.id("diode").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(diff, 0, 0, 100, 100);
        store.add_rect(poly, 40, -20, 20, 140);
        store.add_rect(nsdm, -10, -10, 120, 120);
        store.add_rect(met1, 40, -20, 1000, 1000);
        // Recognition marker overlaps met1, but is deliberately absent from
        // `connectivity.conductors` in antenna_deck().
        store.add_rect(diode, 100, 100, 100, 100);

        let report = check_antenna(
            &store,
            &deck,
            &AntennaConfig {
                cut_required: Some(false),
                rules: vec![AntennaRule {
                    id: "ant.m1.diode".into(),
                    gates: vec![AntennaGate {
                        gate_layer: poly,
                        channel_layer: diff,
                    }],
                    collectors: vec![AntennaCollector {
                        layer: met1,
                        measurement: AntennaMeasurement::Area,
                    }],
                    max_egar: 100.0,
                    diode: Some(AntennaDiode {
                        layer: diode,
                        area_credit_per_um2: 0.0,
                        fixed_bonus: 0.0,
                        full_waiver: true,
                    }),
                }],
            },
        );
        assert_eq!(report.check.status, CheckStatus::Clean);
        assert_eq!(report.nets.len(), 1);
        assert!(report.nets[0].waived);
        assert!(report.nets[0].diode_area_um2 > 0.0);
    }

    #[test]
    fn antenna_diode_marker_cannot_bridge_distinct_nets() {
        let deck = antenna_deck();
        let diff = deck.layers.id("diff").unwrap();
        let poly = deck.layers.id("poly").unwrap();
        let met1 = deck.layers.id("met1").unwrap();
        let diode = deck.layers.id("diode").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 100, 100);
        store.add_rect(met1, 200, 0, 100, 100);
        store.add_rect(diode, 50, 0, 200, 100);

        let report = check_antenna(
            &store,
            &deck,
            &AntennaConfig {
                cut_required: Some(false),
                rules: vec![AntennaRule {
                    id: "ant.ambiguous_diode".into(),
                    gates: vec![AntennaGate {
                        gate_layer: poly,
                        channel_layer: diff,
                    }],
                    collectors: vec![AntennaCollector {
                        layer: met1,
                        measurement: AntennaMeasurement::Area,
                    }],
                    max_egar: 100.0,
                    diode: Some(AntennaDiode {
                        layer: diode,
                        area_credit_per_um2: 0.0,
                        fixed_bonus: 0.0,
                        full_waiver: true,
                    }),
                }],
            },
        );
        assert_eq!(report.check.status, CheckStatus::Error);
        assert!(report.check.diagnostics[0].contains("multiple extracted nets"));
    }

    #[test]
    fn density_checks_empty_die_windows_and_gradient() {
        let mut store = GeometryStore::new();
        store.add_rect(0, 0, 0, 50, 100);
        let report = check_density_cmp(
            &store,
            &DensityCmpConfig {
                die: Bbox {
                    xmin: 0,
                    ymin: 0,
                    xmax: 100,
                    ymax: 100,
                },
                include_partial_windows: false,
                rules: vec![DensityCmpRule {
                    id: "m1".into(),
                    layer: 0,
                    window_width: 50,
                    window_height: 100,
                    step_x: 50,
                    step_y: 100,
                    min_density: Some(0.4),
                    max_density: Some(1.0),
                    max_neighbor_delta: Some(0.5),
                    cmp: None,
                }],
            },
        );
        assert_eq!(report.windows.len(), 2);
        assert_eq!(report.windows[0].density, 1.0);
        assert_eq!(report.windows[1].density, 0.0);
        assert_eq!(report.check.status, CheckStatus::Violations);
        assert!(report
            .check
            .violations
            .iter()
            .any(|v| v.rule_id.ends_with("min_density")));
        assert!(report
            .check
            .violations
            .iter()
            .any(|v| v.rule_id.ends_with("density_gradient")));
    }

    #[test]
    fn density_rejects_steps_that_leave_unchecked_gaps() {
        let report = check_density_cmp(
            &GeometryStore::new(),
            &DensityCmpConfig {
                die: Bbox {
                    xmin: 0,
                    ymin: 0,
                    xmax: 100,
                    ymax: 100,
                },
                include_partial_windows: true,
                rules: vec![DensityCmpRule {
                    id: "m1".into(),
                    layer: 0,
                    window_width: 20,
                    window_height: 20,
                    step_x: 21,
                    step_y: 20,
                    min_density: Some(0.0),
                    max_density: None,
                    max_neighbor_delta: None,
                    cmp: None,
                }],
            },
        );
        assert_eq!(report.check.status, CheckStatus::Error);

        let trailing_edge = check_density_cmp(
            &GeometryStore::new(),
            &DensityCmpConfig {
                die: Bbox {
                    xmin: 0,
                    ymin: 0,
                    xmax: 250,
                    ymax: 100,
                },
                include_partial_windows: false,
                rules: vec![DensityCmpRule {
                    id: "m1.trailing".into(),
                    layer: 0,
                    window_width: 100,
                    window_height: 100,
                    step_x: 100,
                    step_y: 100,
                    min_density: Some(0.0),
                    max_density: Some(1.0),
                    max_neighbor_delta: None,
                    cmp: None,
                }],
            },
        );
        assert_eq!(trailing_edge.check.status, CheckStatus::Error);
        assert!(trailing_edge.check.diagnostics[0].contains("unchecked die-edge strip"));
    }

    #[test]
    fn power_solver_resolves_sub_ampere_loads_and_ir_needs_scope() {
        let grid = PowerGrid {
            nodes: vec![
                PowerNode {
                    id: "supply".into(),
                    x: 0,
                    y: 0,
                    nominal_voltage_v: 1.0,
                    fixed_voltage_v: Some(1.0),
                    load_current_a: 0.0,
                    check_ir_drop: false,
                },
                PowerNode {
                    id: "load".into(),
                    x: 1,
                    y: 0,
                    nominal_voltage_v: 1.0,
                    fixed_voltage_v: None,
                    load_current_a: 1.0e-12,
                    check_ir_drop: true,
                },
            ],
            edges: vec![PowerEdge {
                id: "r".into(),
                from: 0,
                to: 1,
                resistance_ohm: 1.0e6,
                length_um: 1.0,
                kind: PowerEdgeKind::Metal {
                    width_um: 1.0,
                    thickness_um: 1.0,
                },
                temperature_c: 25.0,
                max_current_density_a_per_um2: Some(1.0),
                max_current_per_cut_a: None,
                blech_product_limit_a_per_um: None,
                em_exempt: false,
            }],
        };
        let solution = solve_power_grid(&grid, &PowerSolveConfig::default()).unwrap();
        assert!((solution.node_voltages[1].drop_v - 1.0e-6).abs() < 1.0e-12);

        let mut no_scope = grid.clone();
        no_scope.nodes[1].check_ir_drop = false;
        let report = analyze_ir_drop(
            &no_scope,
            &solution,
            &IrDropConfig {
                max_drop_v: Some(1.0),
                max_drop_pct: None,
                max_overvoltage_v: None,
            },
        );
        assert_eq!(report.check.status, CheckStatus::Error);
    }

    fn simple_power() -> (PowerGrid, PowerSolution) {
        let grid = PowerGrid {
            nodes: vec![
                PowerNode {
                    id: "VDD".into(),
                    x: 0,
                    y: 0,
                    nominal_voltage_v: 1.0,
                    fixed_voltage_v: Some(1.0),
                    load_current_a: 0.0,
                    check_ir_drop: true,
                },
                PowerNode {
                    id: "LOAD".into(),
                    x: 10,
                    y: 0,
                    nominal_voltage_v: 1.0,
                    fixed_voltage_v: None,
                    load_current_a: 0.1,
                    check_ir_drop: true,
                },
            ],
            edges: vec![PowerEdge {
                id: "M1.0".into(),
                from: 0,
                to: 1,
                resistance_ohm: 1.0,
                length_um: 10.0,
                kind: PowerEdgeKind::Metal {
                    width_um: 1.0,
                    thickness_um: 1.0,
                },
                temperature_c: 25.0,
                max_current_density_a_per_um2: Some(0.05),
                max_current_per_cut_a: None,
                blech_product_limit_a_per_um: None,
                em_exempt: false,
            }],
        };
        let solution = solve_power_grid(&grid, &PowerSolveConfig::default()).unwrap();
        (grid, solution)
    }

    #[test]
    fn dc_grid_drives_ir_and_em_checks() {
        let (grid, solution) = simple_power();
        assert!((solution.node_voltages[1].voltage_v - 0.9).abs() < 1e-9);
        assert!((solution.branch_currents[0].current_a - 0.1).abs() < 1e-9);
        let ir = analyze_ir_drop(
            &grid,
            &solution,
            &IrDropConfig {
                max_drop_v: Some(0.05),
                max_drop_pct: Some(5.0),
                max_overvoltage_v: None,
            },
        );
        assert_eq!(ir.check.status, CheckStatus::Violations);
        let em = analyze_electromigration(
            &grid,
            &solution,
            &ElectromigrationConfig {
                default_max_current_density_a_per_um2: None,
                default_max_current_per_cut_a: None,
                reference_temperature_c: 25.0,
                activation_energy_ev: 0.7,
                current_exponent: 2.0,
                max_temperature_c: Some(125.0),
            },
        );
        assert_eq!(em.check.status, CheckStatus::Violations);
        assert!((em.branches[0].current_density_a_per_um2.unwrap() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn power_grid_rejects_duplicate_edge_identity() {
        let (mut grid, _) = simple_power();
        grid.edges.push(grid.edges[0].clone());
        assert!(solve_power_grid(&grid, &PowerSolveConfig::default()).is_err());
    }

    #[test]
    fn reliability_models_voltage_thermal_and_lifetime() {
        let report = check_reliability(&ReliabilityConfig {
            required_lifetime_hours: 100_000.0,
            voltage_stresses: vec![VoltageStress {
                id: "oxide".into(),
                measured_abs_v: 2.0,
                max_abs_v: 1.8,
                location: None,
            }],
            thermal_stresses: vec![ThermalStress {
                id: "junction".into(),
                measured_c: 100.0,
                max_c: 125.0,
                location: None,
            }],
            aging_stresses: vec![AgingStress {
                id: "bti".into(),
                mechanism: "BTI".into(),
                reference_lifetime_hours: 200_000.0,
                reference_stress: 1.0,
                applied_stress: 2.0,
                stress_exponent: 2.0,
                reference_temperature_c: 25.0,
                applied_temperature_c: 25.0,
                activation_energy_ev: 0.0,
                duty_cycle: 1.0,
                location: None,
            }],
        });
        assert_eq!(report.check.status, CheckStatus::Violations);
        assert_eq!(report.aging[0].predicted_lifetime_hours, 50_000.0);
        assert_eq!(report.check.violations.len(), 2);
    }

    #[test]
    fn esd_path_and_latchup_evidence_can_be_clean() {
        let config = EsdLatchupConfig {
            nodes: vec![
                EsdNode {
                    id: "PAD".into(),
                    kind: EsdNodeKind::IoPad,
                    x: 0,
                    y: 0,
                },
                EsdNode {
                    id: "VSS".into(),
                    kind: EsdNodeKind::Ground,
                    x: 10,
                    y: 0,
                },
            ],
            edges: vec![EsdEdge {
                id: "clamp".into(),
                from: 0,
                to: 1,
                bidirectional: false,
                resistance_ohm: 1.0,
                current_capacity_a: 2.0,
                clamp_voltage_v: 4.0,
            }],
            esd_paths: vec![EsdPathRequirement {
                id: "pad_to_vss".into(),
                source: 0,
                explicit_targets: vec![],
                target_kinds: vec![EsdNodeKind::Ground],
                required_current_a: 1.0,
                max_path_resistance_ohm: 2.0,
                max_clamp_voltage_v: 5.0,
            }],
            latchup_sites: vec![LatchupSite {
                id: "io_site".into(),
                location: (0, 0),
                guard_ring: Some(GuardRingEvidence {
                    id: "gr0".into(),
                    bias_node: 1,
                    continuous: true,
                    width_um: 2.0,
                    aggressor_distance_um: 1.0,
                    victim_distance_um: 1.0,
                    nearest_tap_distance_um: 5.0,
                }),
                allowed_bias_kinds: vec![EsdNodeKind::Ground],
                min_guard_ring_width_um: 1.0,
                max_aggressor_distance_um: 2.0,
                max_victim_distance_um: 2.0,
                max_tap_distance_um: 10.0,
            }],
        };
        let report = check_esd_latchup(&config);
        assert_eq!(report.check.status, CheckStatus::Clean);
        assert_eq!(report.paths.len(), 1);

        let mut missing_latchup = config.clone();
        missing_latchup.latchup_sites.clear();
        assert_eq!(
            check_esd_latchup(&missing_latchup).check.status,
            CheckStatus::Error
        );

        let mut missing_esd = config;
        missing_esd.esd_paths.clear();
        assert_eq!(
            check_esd_latchup(&missing_esd).check.status,
            CheckStatus::Error
        );
    }

    #[test]
    fn rectangle_union_does_not_double_count_overlap() {
        let (area, perimeter) = rect_union_metrics(&[
            Bbox {
                xmin: 0,
                ymin: 0,
                xmax: 10,
                ymax: 10,
            },
            Bbox {
                xmin: 5,
                ymin: 0,
                xmax: 15,
                ymax: 10,
            },
        ]);
        assert_eq!(area, 150.0);
        assert_eq!(perimeter, 50.0);
    }

    #[test]
    fn polygon_rect_rejects_bow_tie_corner_order() {
        let mut store = GeometryStore::new();
        let p = store.add_polygon(0, &[(0, 0), (10, 10), (10, 0), (0, 10)]);
        assert_eq!(polygon_rect(&store, p), None);
    }
}
