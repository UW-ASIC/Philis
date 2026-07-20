//! Canonical backend flow: constrain → generate → place → route → verify.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;

use gdsverify::{
    compare, extract_netlist_opts, run_drc, run_pex_by_net_checked, AgingStress, AntennaCollector,
    AntennaConfig, AntennaGate, AntennaMeasurement, AntennaRule, Backend, Bbox, CompareOpts, Deck,
    DensityCmpConfig, DensityCmpRule, DeviceFlavor, DeviceKind, DrcReport, ElectromigrationConfig,
    EsdEdge, EsdLatchupConfig, EsdNode, EsdNodeKind, EsdPathRequirement, ExtractOpts,
    GeometryStore, GuardRingEvidence, IrDropConfig, LatchupSite, LayerId, LvsResult, PexReport,
    PolyId, PowerEdge, PowerEdgeKind, PowerGrid, PowerNode, PowerSignoffConfig, PowerSolveConfig,
    RefDevice, RefNetlist, ReliabilityConfig, Violation, VoltageStress,
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
    /// Harness contract (fixed die + boundary pins). `None` keeps the classic
    /// behavior: adaptive die, unattached IO.
    pub interface: Option<crate::interface::InterfaceSpec>,
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
            interface: None,
        }
    }
}

/// Heuristic ERC violation kinds waived at block level. Every entry must
/// carry a written justification for why the finding cannot be satisfied at
/// block level (same spirit as the min_density waiver). Everything not
/// listed blocks tapeout.
const WAIVED_ERC_KINDS: &[&str] = &[
    // multiple_drivers models DIGITAL output contention: a non-gate net
    // driven by drains whose gates differ. Analog current-mode nodes violate
    // its premise by design — an OTA output is the drain of the mirror load
    // AND the drain of the input device (different gates) summing currents
    // into one high-impedance node, and extraction cannot even orient S vs D.
    // Every correct OTA/mirror in the suite trips it, and no block-level fix
    // exists short of redesigning the circuit, so it is waived for these
    // analog blocks. Real supply contention still surfaces via supply_short.
    "multiple_drivers",
];

pub struct SignoffReport {
    pub drc: DrcReport,
    pub drc_blocking: Vec<Violation>,
    pub drc_waived_density: usize,
    pub lvs: LvsResult,
    pub pex: PexReport,
    pub contracts: Vec<ConstraintContract>,
    /// Geometry-derived heuristic ERC findings (floating_well, missing_tie,
    /// supply_short, floating_gate, ...). Blocking unless the kind is in
    /// [`WAIVED_ERC_KINDS`].
    pub erc_violations: Vec<gdsverify::ErcViolation>,
    pub advanced: gdsverify::SignoffSuiteReport,
    /// True when the design has a recognized VDD-class supply rail (among
    /// routed net names OR schematic port names). Power-family checks
    /// (ir_drop, electromigration, reliability) that are NotRun/Error are only
    /// blocking when a supply rail exists; otherwise there is nothing to
    /// analyze and they are recorded "not applicable".
    pub has_supply_rail: bool,
}

impl SignoffReport {
    /// ERC findings that block tapeout (everything not explicitly waived).
    pub fn blocking_erc_violations(&self) -> impl Iterator<Item = &gdsverify::ErcViolation> {
        self.erc_violations
            .iter()
            .filter(|v| !WAIVED_ERC_KINDS.contains(&v.check.as_str()))
    }

    /// A power-family check that only failed to run because the design has no
    /// supply rail is not applicable — it does not block, and is surfaced
    /// distinctly (not conflated with a genuine NotRun/Error).
    #[must_use]
    pub fn check_not_applicable(&self, check: gdsverify::SignoffCheck) -> bool {
        use gdsverify::{CheckStatus, SignoffCheck};
        if self.has_supply_rail {
            return false;
        }
        let report = match check {
            SignoffCheck::IrDrop => &self.advanced.ir_drop.check,
            SignoffCheck::Electromigration => &self.advanced.electromigration.check,
            SignoffCheck::Reliability => &self.advanced.reliability.check,
            _ => return false,
        };
        matches!(report.status, CheckStatus::NotRun | CheckStatus::Error)
    }

    /// Aggregate the checks that have an explicit pass/fail policy. PEX values
    /// are gated through parasitic-budget contracts when those are supplied.
    /// Missing advanced-analysis inputs remain blocking through `all_clean()`.
    #[must_use]
    pub fn all_required_checks_clean(&self) -> bool {
        self.drc_blocking.is_empty()
            && self.lvs.matched
            && self.pex.is_complete()
            && self.blocking_erc_violations().next().is_none()
            && self
                .advanced
                .checks()
                .iter()
                .all(|c| c.is_clean() || self.check_not_applicable(c.check))
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
                device_mappings: Vec::new(),
                net_mappings: Vec::new(),
                witness: None,
            },
            pex: PexReport { parasitics },
            contracts: Vec::new(),
            erc_violations: Vec::new(),
            advanced,
            // Policy regressions assume a supplied design so NotRun/Error
            // block; applicability is exercised separately below.
            has_supply_rail: true,
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
    fn final_drc_policy_waives_density_only_for_sub_window_layouts() {
        let deck = Deck::from_json(
            r#"{"layers":{"met1":{"layer":68,"datatype":20}},
                "drc":{"min_density":{"layer":"met1","window":700000,"min_frac":0.35}}}"#,
        )
        .expect("test deck");
        let met1 = deck.layers.id("met1").unwrap();
        let violation = Violation {
            rule_id: "min_density".into(),
            kind: "min_density".into(),
            layer: "met1".into(),
            measured: 12,
            limit: 350_000,
            x: 0,
            y: 0,
            hierarchy_path: None,
            source_polygons: Vec::new(),
            marker: None,
        };
        let drc = DrcReport { violations: vec![violation] };

        // Layer extent smaller than the window: waived, not blocking.
        let mut small = GeometryStore::new();
        small.add_rect(met1, 0, 0, 7_000, 7_000);
        let (blocking, waived) = final_drc_policy(&drc, &deck, &small);
        assert!(blocking.is_empty());
        assert_eq!(waived, 1);

        // Layer spanning a full window: density stays a blocking tapeout rule.
        let mut large = GeometryStore::new();
        large.add_rect(met1, 0, 0, 700_000, 700_000);
        let (blocking, waived) = final_drc_policy(&drc, &deck, &large);
        assert_eq!(blocking.len(), 1);
        assert_eq!(waived, 0);

        // One-dimension-long strip (3 windows in x, sub-window in y): real
        // density windows exist along x, so sparse markers must still BLOCK —
        // waiving on a single sub-window extent would hide an empty middle.
        let mut strip = GeometryStore::new();
        strip.add_rect(met1, 0, 0, 2_100_000, 100_000);
        let (blocking, waived) = final_drc_policy(&drc, &deck, &strip);
        assert_eq!(blocking.len(), 1);
        assert_eq!(waived, 0);

        // Empty layer: a deck density rule on a layer with zero drawn shapes
        // is a missing-layer failure, not a fill question — must BLOCK.
        let empty = GeometryStore::new();
        let (blocking, waived) = final_drc_policy(&drc, &deck, &empty);
        assert_eq!(blocking.len(), 1);
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
            hierarchy_path: None,
            source_polygons: Vec::new(),
            marker: None,
        });
        let json = crate::gds::signoff_json(&report);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid sidecar JSON");
        assert_eq!(parsed["drc_violations"][0]["rule"], "deck/\"M1.W\"");
        assert_eq!(parsed["all_required_checks_clean"], false);
    }

    /// Deck shared by the advanced-signoff config-derivation tests.
    fn advanced_deck() -> Deck {
        Deck::from_json(
            r#"{"layers":{
                    "diff":{"layer":65,"datatype":20},
                    "poly":{"layer":66,"datatype":20},
                    "nsdm":{"layer":93,"datatype":44},
                    "li":{"layer":67,"datatype":20},
                    "met1":{"layer":68,"datatype":20}},
                "drc":{"max_density":{"layer":"met1","window":700000,"max_frac":0.7}},
                "connectivity":{"conductors":["diff","poly","li","met1"],"vias":[]},
                "device_recognition":{"mos":[{
                    "name":"nmos","gate_layer":"poly","channel_layer":"diff",
                    "type_implant":"nsdm","device_type":"nmos"}]}}"#,
        )
        .expect("test deck")
    }

    /// Cell-free netlist with the given nets, for config-derivation tests.
    fn bare_graph(nets: &[&str]) -> BipartiteHypergraph {
        BipartiteHypergraph {
            name: "t".into(),
            ports: Vec::new(),
            cells: Vec::new(),
            nets: nets.iter().map(|n| (*n).into()).collect(),
            net_start: vec![0; nets.len() + 1],
            net_pins: Vec::new(),
        }
    }

    fn supply_wire(net: u32, y: i32, len: i32, width: i32) -> pnr_routing::Wire {
        pnr_routing::Wire {
            net,
            layer: 0,
            x0: 0,
            y0: y,
            x1: len,
            y1: y,
            width,
        }
    }

    #[test]
    fn build_signoff_config_populates_all_six_checks() {
        let deck = advanced_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let diff = deck.layers.id("diff").unwrap();
        let li = deck.layers.id("li").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 10_000, 400);
        // Enough drawn junction area that the measured ESD discharge capacity
        // (5 mA/um²) meets the 0.5 A path requirement: 200 um² → 1 A.
        store.add_rect(diff, 0, 20_000, 20_000, 10_000);
        // A drawn tap (li) at die center so the substrate latch-up site has
        // real tap evidence within the deck tie distance.
        store.add_rect(li, 9_000, 14_500, 2_000, 1_000);
        let g = bare_graph(&["VDD", "VSS"]);
        // Wide rails: their measured square-count resistance now feeds the
        // ESD path model, which fails on genuinely resistive rails.
        let cfg = build_signoff_config(
            &store,
            &deck,
            &g,
            &["VDD".into(), "VSS".into()],
            &[supply_wire(0, 0, 10_000, 2_000), supply_wire(1, 5_000, 10_000, 2_000)],
            &[],
            &[],
            &pnr_cells::pdk::Pdk::default(),
            &gdsverify::SignoffConfig::default(),
        );
        assert!(cfg.antenna.is_some(), "antenna config derived from deck");
        assert!(cfg.density_cmp.is_some(), "density rules derived from deck");
        let power = cfg
            .power
            .as_ref()
            .expect("power grid derived from routed rails");
        assert_eq!(power.grid.nodes.len(), 4, "two nodes per supply rail");
        assert_eq!(power.grid.edges.len(), 2, "one edge per supply rail");
        assert!(power.ir_drop.is_some() && power.electromigration.is_some());
        assert!(cfg.reliability.is_some());
        assert!(cfg.esd_latchup.is_some());
        assert!(cfg.layer_order.is_some());
        // The derived evidence must itself be runnable and clean on this
        // trivially healthy input — NotRun/Error are blocking by design.
        let report = gdsverify::run_erc(&store, &deck, &cfg).signoff;
        for check in report.checks() {
            assert!(check.is_clean(), "{:?}: {:?}", check.check, check);
        }
    }

    #[test]
    fn erc_violations_block_aggregate_clean() {
        let mut r = report(suite(CheckStatus::Clean), Vec::new());
        assert!(r.all_required_checks_clean());
        r.erc_violations.push(gdsverify::ErcViolation {
            check: "supply_short".into(),
            detail: "nmos and pmos sources shorted (VDD/VSS)".into(),
            x: 0,
            y: 0,
        });
        assert!(!r.all_required_checks_clean());
        assert_eq!(r.blocking_erc_violations().count(), 1);
        // A waived kind (analog false positive) does not block on its own.
        r.erc_violations.clear();
        r.erc_violations.push(gdsverify::ErcViolation {
            check: "multiple_drivers".into(),
            detail: "net driven by 2 different gate signals".into(),
            x: 0,
            y: 0,
        });
        assert!(r.all_required_checks_clean());
        assert_eq!(r.blocking_erc_violations().count(), 0);
    }

    #[test]
    fn power_family_not_run_is_applicability_gated() {
        // ir_drop NotRun with no supply rail → not applicable, does not block.
        let mut sr = suite(CheckStatus::Clean);
        sr.ir_drop.check = CheckReport::not_run(SignoffCheck::IrDrop, "no power grid");
        sr.electromigration.check =
            CheckReport::not_run(SignoffCheck::Electromigration, "no power grid");
        sr.reliability.check =
            CheckReport::not_run(SignoffCheck::Reliability, "no reliability evidence");
        let mut r = report(sr, Vec::new());
        r.has_supply_rail = false;
        assert!(
            r.all_required_checks_clean(),
            "power-family NotRun on a rail-less design must not block"
        );
        assert!(r.check_not_applicable(SignoffCheck::IrDrop));

        // Same suite, but the design HAS a supply rail: NotRun now blocks.
        r.has_supply_rail = true;
        assert!(
            !r.all_required_checks_clean(),
            "power-family NotRun on a supplied design must block"
        );
        assert!(!r.check_not_applicable(SignoffCheck::IrDrop));

        // A non-power check that Errors always blocks, rail or not.
        let mut ant = suite(CheckStatus::Clean);
        ant.antenna.check = CheckReport::error(SignoffCheck::Antenna, "bad antenna evidence");
        let mut r = report(ant, Vec::new());
        r.has_supply_rail = false;
        assert!(!r.all_required_checks_clean(), "antenna Error always blocks");
        assert!(!r.check_not_applicable(SignoffCheck::Antenna));
    }

    #[test]
    fn advanced_detail_is_serialized_for_non_clean_checks() {
        // A power-family Error on a supplied design serializes its diagnostics;
        // the same on a rail-less design serializes as NotApplicable.
        let mut sr = suite(CheckStatus::Clean);
        sr.ir_drop.check = CheckReport::error(SignoffCheck::IrDrop, "grid singular");
        let mut r = report(sr, Vec::new());
        r.has_supply_rail = true;
        let json = crate::gds::signoff_json(&r);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["advanced"]["ir_drop"]["status"], "Error");
        assert_eq!(
            parsed["advanced"]["ir_drop"]["diagnostics"][0],
            "grid singular"
        );
        assert_eq!(parsed["advanced"]["antenna"]["status"], "Clean");

        r.has_supply_rail = false;
        let json = crate::gds::signoff_json(&r);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["advanced"]["ir_drop"]["status"], "NotApplicable");
    }

    #[test]
    fn esd_path_fails_when_junction_area_is_insufficient() {
        let deck = advanced_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let diff = deck.layers.id("diff").unwrap();
        let li = deck.layers.id("li").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 10_000, 400);
        // Hand-shrunk junction: 1 um² → 5 mA measured capacity, far below the
        // 0.5 A chip-pad HBM requirement. VDD is a DECLARED pad here, so the
        // full HBM discharge path applies and this junction must FAIL — there
        // is no floor that guarantees passing anymore.
        store.add_rect(diff, 0, 20_000, 1_000, 1_000);
        // Tap at die center keeps the latch-up half clean, isolating the
        // ESD-path failure. Die bbox: 0..10000 x 0..21000, center (5000,10500).
        store.add_rect(li, 4_000, 10_000, 2_000, 1_000);
        let g = bare_graph(&["VDD", "VSS"]);
        let cfg = build_signoff_config(
            &store,
            &deck,
            &g,
            &["VDD".into(), "VSS".into()],
            &[supply_wire(0, 0, 10_000, 2_000), supply_wire(1, 5_000, 10_000, 2_000)],
            &[],
            &["VDD".into()],
            &pnr_cells::pdk::Pdk::default(),
            &gdsverify::SignoffConfig::default(),
        );
        let esd = cfg.esd_latchup.expect("esd config derived");
        let report = gdsverify::check_esd_latchup(&esd);
        assert_eq!(report.check.status, CheckStatus::Violations);
        assert!(
            report
                .check
                .violations
                .iter()
                .any(|v| v.rule_id.starts_with("esd.path")),
            "expected an ESD path violation, got {:?}",
            report.check.violations,
        );
    }

    #[test]
    fn latchup_site_fails_without_tap_evidence() {
        let deck = advanced_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let diff = deck.layers.id("diff").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 10_000, 2_000);
        store.add_rect(diff, 0, 20_000, 20_000, 10_000);
        // No li taps drawn at all: the substrate latch-up site has no
        // evidence and must report a real violation.
        let g = bare_graph(&["VDD", "VSS"]);
        let cfg = build_signoff_config(
            &store,
            &deck,
            &g,
            &["VDD".into(), "VSS".into()],
            &[supply_wire(0, 0, 10_000, 2_000), supply_wire(1, 5_000, 10_000, 2_000)],
            &[],
            &[],
            &pnr_cells::pdk::Pdk::default(),
            &gdsverify::SignoffConfig::default(),
        );
        let report = gdsverify::check_esd_latchup(&cfg.esd_latchup.expect("esd config derived"));
        assert_eq!(report.check.status, CheckStatus::Violations);
        assert!(report
            .check
            .violations
            .iter()
            .any(|v| v.rule_id.starts_with("latchup.")));
    }

    #[test]
    fn ir_drop_can_fail_on_thin_long_rail() {
        let deck = advanced_deck();
        // One wide NMOS drawing worst-case Idsat (≈50 mA at W/L = 667)
        // through a 1 mm long, 140 nm wide rail (≈893 ohm): the solved drop
        // dwarfs the 5% limit, so the derived power evidence CAN fail.
        let dev = DeviceRecord {
            name: "M1".into(),
            device_type: DeviceType::Nmos,
            w: 100_000,
            l: 150,
            nf: 1,
            multiplier: 1,
            model_name: "nfet_01v8".into(),
            terminals: HashMap::new(),
            params: HashMap::new(),
        };
        let g = BipartiteHypergraph {
            name: "t".into(),
            ports: Vec::new(),
            cells: vec![pnr_cells::netlist::CellNode {
                name: "XM1".into(),
                model: "nmos".into(),
                pins: vec![("S".into(), 0)],
                device: Some(dev),
                grouped_devices: Vec::new(),
            }],
            nets: vec!["VDD".into()],
            net_start: vec![0, 1],
            net_pins: vec![(0, 0)],
        };
        let cfg = build_signoff_config(
            &GeometryStore::new(),
            &deck,
            &g,
            &["VDD".into()],
            &[supply_wire(0, 0, 1_000_000, 140)],
            &[],
            &[],
            &pnr_cells::pdk::Pdk::default(),
            &gdsverify::SignoffConfig::default(),
        );
        let power = cfg.power.expect("power grid derived from the routed rail");
        let sol = gdsverify::solve_power_grid(&power.grid, &power.solver).expect("solvable grid");
        let ir = gdsverify::analyze_ir_drop(&power.grid, &sol, power.ir_drop.as_ref().unwrap());
        assert_eq!(ir.check.status, CheckStatus::Violations);
    }

    #[test]
    fn reliability_reports_only_measured_stresses() {
        let deck = advanced_deck();
        let g = bare_graph(&["VDD", "VSS"]);
        let pdk = pnr_cells::pdk::Pdk::default();

        // No routed supply: every stress entry would be invented, so the
        // config stays None → the check reports NotRun and blocks honestly.
        let cfg = build_signoff_config(
            &GeometryStore::new(),
            &deck,
            &g,
            &["VDD".into(), "VSS".into()],
            &[],
            &[],
            &[],
            &pdk,
            &gdsverify::SignoffConfig::default(),
        );
        assert!(cfg.power.is_none(), "no fabricated proxy power grid");
        assert!(cfg.reliability.is_none(), "no invented reliability evidence");

        // With a routed rail, the measured value comes from the shared
        // power-grid solve and no thermal entry is invented.
        let cfg = build_signoff_config(
            &GeometryStore::new(),
            &deck,
            &g,
            &["VDD".into(), "VSS".into()],
            &[supply_wire(0, 0, 10_000, 2_000), supply_wire(1, 5_000, 10_000, 2_000)],
            &[],
            &[],
            &pdk,
            &gdsverify::SignoffConfig::default(),
        );
        let rel = cfg.reliability.expect("measured stresses exist");
        assert!(rel.thermal_stresses.is_empty(), "no invented thermal stress");
        assert_eq!(rel.voltage_stresses.len(), 1, "one powered rail");
        let vs = &rel.voltage_stresses[0];
        // Zero attached devices → zero load current → the solved max node
        // voltage equals the source pin: measured from the solve, not typed in.
        assert!((vs.measured_abs_v - 1.8).abs() < 1e-9);
        // Cell-free netlist has no MOS: no aging entry is invented either.
        assert!(rel.aging_stresses.is_empty());
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
        // PEX here is the analytical run_pex summary; the netlist dump uses
        // the quasistatic field-solver path — label so they are not conflated.
        writeln!(
            f,
            "signoff: DRC {} blocking ({} density waived) | LVS {} ({} N / {} P) | PEX(analytical) {} R(met1) {:.1} ohm, C {:.1} fF",
            s.drc_blocking.len(), s.drc_waived_density,
            if s.lvs.matched { "MATCH" } else { "MISMATCH" },
            s.lvs.nmos, s.lvs.pmos,
            if s.pex.is_complete() { "COMPLETE" } else { "INCOMPLETE" },
            s.pex.total_resistance("met1"), s.pex.total_cap() / 1000.0,
        )?;
        let advanced_clean = s.advanced.checks().iter().filter(|r| r.is_clean()).count();
        let advanced_na = s
            .advanced
            .checks()
            .iter()
            .filter(|r| s.check_not_applicable(r.check))
            .count();
        writeln!(
            f,
            "  advanced signoff: {advanced_clean}/6 checks clean ({advanced_na} not applicable)"
        )?;
        // Detail every non-clean advanced check so ESD/IR/EM findings are
        // triageable from signoff.txt, not just a bare status.
        for check in s.advanced.checks() {
            if check.is_clean() {
                continue;
            }
            if s.check_not_applicable(check.check) {
                writeln!(
                    f,
                    "  advanced {:?}: NotApplicable (no supply rails)",
                    check.check
                )?;
                continue;
            }
            writeln!(f, "  advanced {:?}: {:?}", check.check, check.status)?;
            for d in &check.diagnostics {
                writeln!(f, "      diag: {d}")?;
            }
            for v in check.violations.iter().take(10) {
                writeln!(f, "      {}: {}", v.rule_id, v.message)?;
            }
        }
        let erc_blocking = s.blocking_erc_violations().count();
        writeln!(
            f,
            "  ERC: {} blocking violation(s) ({} waived)",
            erc_blocking,
            s.erc_violations.len() - erc_blocking,
        )?;
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
        for v in s.blocking_erc_violations().take(10) {
            writeln!(f, "  ERC {}: {} at ({}, {})", v.check, v.detail, v.x, v.y)?;
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
    /// Pin declared on a layer below li (diff/poly): the contact stack must
    /// add its own li overlay. Pins already on li have cell-drawn li — an
    /// extra overlay bar can bridge adjacent S/D fingers.
    needs_li: bool,
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

/// Boundary pin resolved against the design: net id, routing-metal index, and
/// DRC-legal width. Its position stays symbolic (side/frac or absolute) until
/// a concrete die is known — the adaptive path only fixes the die at the end.
#[derive(Debug)]
struct IfacePin {
    spec: crate::interface::BoundaryPin,
    net: u32,
    /// Index into the PDK routing-metal stack (0 = first metal).
    met_idx: usize,
    /// Resolved deck layer of the pin square. Carried so drawing never
    /// re-derives it by index into a deck-filtered metal list (which shifts
    /// when the deck omits a mid-stack metal).
    layer_id: LayerId,
    width: i32,
    grid: i32,
}

fn resolve_interface_pins(
    spec: Option<&crate::interface::InterfaceSpec>,
    g: &BipartiteHypergraph,
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
    default_width: i32,
    grid: i32,
) -> Result<Vec<IfacePin>, String> {
    let Some(spec) = spec else {
        return Ok(Vec::new());
    };
    spec.validate()?;
    let mut out = Vec::with_capacity(spec.pins.len());
    for p in &spec.pins {
        let net = g.net_id(&p.net).ok_or_else(|| {
            format!(
                "interface pin net `{}` not in netlist; available nets: {}",
                p.net,
                g.nets.join(", ")
            )
        })?;
        if g.pins_on_net(net).is_empty() {
            return Err(format!(
                "interface pin net `{}` has no device pins to route from",
                p.net
            ));
        }
        let layer = p.layer.clone().unwrap_or_else(|| {
            // Default: top routing metal of the PDK.
            cell_pdk
                .layers
                .routing_metals
                .last()
                .cloned()
                .unwrap_or_else(|| cell_pdk.layers.met1.clone())
        });
        let met_idx = cell_pdk
            .layers
            .routing_metals
            .iter()
            .position(|m| *m == layer)
            .ok_or_else(|| {
                format!(
                    "interface pin `{}`: layer `{layer}` is not a routing metal (expected one of {})",
                    p.net,
                    cell_pdk.layers.routing_metals.join(", ")
                )
            })?;
        let lid = deck
            .layers
            .id(&layer)
            .ok_or_else(|| format!("interface pin `{}`: deck missing layer `{layer}`", p.net))?;
        let min_w = rule_min_width(deck, lid).unwrap_or(0);
        // DRC-legal square side — at least the layer's min width AND its
        // min-area square (a met4 pin narrower than sqrt(M4.AREA) is illegal
        // no matter how legal its width is) — rounded up to a 2*grid multiple
        // so the pin center and all four corners stay on the grid.
        let step = (2 * grid).max(1);
        let width = p
            .width
            .unwrap_or(default_width)
            .max(min_w)
            .max(min_area_side(rule_min_area(deck, lid)))
            .max(step);
        let width = (width + step - 1) / step * step;
        out.push(IfacePin {
            spec: p.clone(),
            net,
            met_idx,
            layer_id: lid,
            width,
            grid,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod interface_resolution_tests {
    use super::*;
    use crate::interface::{BoundaryPin, InterfaceSpec, Side};

    fn graph() -> BipartiteHypergraph {
        BipartiteHypergraph {
            name: "t".into(),
            ports: Vec::new(),
            cells: Vec::new(),
            nets: vec!["VDD".into(), "out".into()],
            net_start: vec![0, 0, 0],
            net_pins: Vec::new(),
        }
    }

    fn deck() -> Deck {
        Deck::from_json(
            r#"{"layers":{"met1":{"layer":68,"datatype":20},"met2":{"layer":69,"datatype":20}},
                "drc":{"min_width":{"layer":"met1","min":140}}}"#,
        )
        .expect("test deck")
    }

    fn edge_pin(net: &str) -> BoundaryPin {
        BoundaryPin {
            net: net.into(),
            layer: None,
            side: Some(Side::South),
            frac: Some(0.5),
            at: None,
            width: None,
        }
    }

    #[test]
    fn unknown_net_errors_and_lists_available_nets() {
        let spec = InterfaceSpec {
            die: None,
            pins: vec![edge_pin("vinp")],
        };
        let err = resolve_interface_pins(
            Some(&spec),
            &graph(),
            &deck(),
            &pnr_cells::pdk::Pdk::default(),
            430,
            5,
        )
        .expect_err("unknown net must fail");
        assert!(err.contains("vinp"), "{err}");
        assert!(err.contains("VDD") && err.contains("out"), "{err}");
    }

    #[test]
    fn net_without_device_pins_is_rejected() {
        let spec = InterfaceSpec {
            die: None,
            pins: vec![edge_pin("VDD")],
        };
        let err = resolve_interface_pins(
            Some(&spec),
            &graph(),
            &deck(),
            &pnr_cells::pdk::Pdk::default(),
            430,
            5,
        )
        .expect_err("pinless net must fail");
        assert!(err.contains("VDD") && err.contains("no device pins"), "{err}");
    }

    #[test]
    fn absent_spec_resolves_to_nothing() {
        let pins = resolve_interface_pins(
            None,
            &graph(),
            &deck(),
            &pnr_cells::pdk::Pdk::default(),
            430,
            5,
        )
        .expect("None spec is the classic flow");
        assert!(pins.is_empty());
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
    iface: &[IfacePin],
) -> Result<BuiltBlock, String> {
    let lt = &deck.layers;
    let poly = lt
        .id(&cell_pdk.layers.poly)
        .ok_or_else(|| format!("deck missing gate-layer role `{}`", cell_pdk.layers.poly))?;
    let diff_id = lt
        .id(&cell_pdk.layers.diff)
        .ok_or_else(|| format!("deck missing diff-layer role `{}`", cell_pdk.layers.diff))?;

    let mut pcfg = cfg.placement.clone();
    pcfg.cell_margin = cell_pdk.device_gap;
    pcfg.boundary_halo = cfg.routing.detailed.pitch;
    // Adaptive die: probe around the tightest routable canvas. The controller
    // keeps feasible/failed bounds and bisects once both are known.
    let density = std::cell::RefCell::new(DensitySearch::new(0.85));
    pcfg.min_side = 2_000;
    // Harness-dictated die: pin the placement canvas exactly there. The
    // density probes keep adjusting utilization, but the fixed die makes the
    // derived canvas moot — placement must fit or the flow errors out.
    if let Some(d) = cfg.interface.as_ref().and_then(|i| i.die) {
        pcfg.fixed_die = Some((d.w, d.h));
    }
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
        // Per-layer wire widths, track thinning, and min-area repair, all
        // derived from the deck's DRC rules (+ ERC EM width).
        let dims = route_dims(deck, cell_pdk, cfg.pad)?;
        let mut rc = rcfg.borrow_mut();
        rc.layer_track_steps = dims.track_steps();
        rc.layer_widths = dims.wire_w;
        rc.layer_min_areas = dims.min_area;
        rc.layer_spacings = dims.spacing;
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
                    // Only pins BELOW li (diff/poly) need the licon + li bar
                    // up to the routing stack. Pins already on li keep their
                    // cell-drawn li; pins ABOVE li (e.g. MiM-cap plates on
                    // met3/met4) must not grow a floating li bar underneath.
                    let needs_li = accs[0].0 == diff_id || accs[0].0 == poly;
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
                        needs_li,
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
            // Boundary interface pins are fixed endpoints on the die edge —
            // same seam as the guard-ring rail terminals above: presented to
            // the router before detailed routing. A single-point net gains a
            // second point here and becomes routable.
            for ip in iface {
                let (tx, ty) = ip.spec.center(p.die, ip.width, ip.grid);
                // Terminal on the track lattice: the drawn via stack snaps to
                // the same point, so its cuts merge with the router's vias
                // instead of landing a sub-spacing offset away.
                let pitch = cfg.routing.detailed.pitch;
                let terminal = (
                    snap_to_track(tx, p.die.0, pitch),
                    snap_to_track(ty, p.die.1, pitch),
                );
                'iface_anchor: for (ci, cell) in g.cells.iter().enumerate() {
                    for (pi, &(_, pin_net)) in cell.pins.iter().enumerate() {
                        if pin_net == ip.net {
                            pin_pos[ci][pi].push(terminal);
                            pin_pos[ci][pi].sort_unstable();
                            pin_pos[ci][pi].dedup();
                            break 'iface_anchor;
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
                iface,
            ) {
                Ok((mut net_sample, _rings)) => {
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
                                pin_swaps: Vec::new(),
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
            // Fixed die: a cell outside the harness die is a hard violation,
            // so the candidate selector never returns an overflowing layout
            // when a contained iteration exists. The adaptive flow grows the
            // die instead — fixed_die is Some only with an interface die.
            let containment_violations = if pcfg.fixed_die.is_some() {
                let p = &placed.placement;
                (0..p.x.len())
                    .filter(|&ci| {
                        let (w, h) = p.sizes[ci];
                        p.x[ci] - w / 2 < 0
                            || p.y[ci] - h / 2 < 0
                            || p.x[ci] + w / 2 > p.die.0
                            || p.y[ci] + h / 2 > p.die.1
                    })
                    .count()
            } else {
                0
            };
            if containment_violations > 0 {
                eprintln!(
                    "[placement] blocking: {containment_violations} cell(s) outside the fixed interface die"
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
                    + overlap_violations
                    + containment_violations,
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
//  Per-layer routing dimensions (derived from the deck's DRC rules)
// ═══════════════════════════════════════════════════════════════════════

fn rule_min_width(deck: &Deck, layer: LayerId) -> Option<i32> {
    deck.drc_rules
        .iter()
        .filter_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinWidth { layer: l, min, .. } if *l == layer => {
                Some(*min)
            }
            _ => None,
        })
        .max()
}

fn rule_min_spacing(deck: &Deck, layer: LayerId) -> Option<i32> {
    deck.drc_rules
        .iter()
        .filter_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinSpacing { layer: l, min, .. } if *l == layer => {
                Some(*min)
            }
            _ => None,
        })
        .max()
}

fn rule_min_area(deck: &Deck, layer: LayerId) -> i64 {
    deck.drc_rules
        .iter()
        .filter_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinArea { layer: l, min, .. } if *l == layer => {
                Some(*min)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// Enclosure of `inner` by `outer`, filtered to the exact layer pair. The old
/// code took the FIRST MinEnclosure rule of ANY pair and depended on the
/// deck's rule-key sort order putting the right rule first; this decouples
/// the code from key naming.
fn rule_enclosure(deck: &Deck, outer: LayerId, inner: LayerId) -> Option<i32> {
    deck.drc_rules
        .iter()
        .filter_map(|r| match r {
            gdsverify::params::DrcRuleParam::MinEnclosure {
                outer: o,
                inner: i,
                min,
                ..
            } if *o == outer && *i == inner => Some(*min),
            _ => None,
        })
        .max()
}

/// Smallest square side (nm) whose area meets `min_area`.
fn min_area_side(min_area: i64) -> i32 {
    if min_area <= 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let mut s = (min_area as f64).sqrt().floor() as i64;
    while s * s < min_area {
        s += 1;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        s as i32
    }
}

/// Per-layer routed-wire, via-cut, and landing-pad dimensions, derived from
/// the deck's DRC rules + ERC EM width so widened rules automatically widen
/// geometry. Indexed by routing-metal position (cuts by routing-via position).
struct RouteDims {
    /// Drawn wire width per metal: max(base, layer min width, EM min width).
    wire_w: Vec<i32>,
    /// Metal min spacing per metal.
    spacing: Vec<i32>,
    /// Metal min area per metal (0 = no rule).
    min_area: Vec<i64>,
    /// Via landing-pad square side per metal: covers wire width, enclosure of
    /// the cut below and above, and the metal's min area.
    pad: Vec<i32>,
    /// Cut square side per via (the via layer's min-width rule).
    cut: Vec<i32>,
}

impl RouteDims {
    /// Min same-direction track center-to-center distance per metal: the
    /// worst same-layer feature at a track node (wire or via pad) + spacing.
    fn track_steps(&self) -> Vec<i32> {
        self.pad
            .iter()
            .zip(&self.spacing)
            .map(|(&p, &s)| p + s)
            .collect()
    }
}

fn route_dims(
    deck: &Deck,
    cell_pdk: &pnr_cells::pdk::Pdk,
    base_width: i32,
) -> Result<RouteDims, String> {
    let lid = |name: &str| {
        deck.layers
            .id(name)
            .ok_or_else(|| format!("deck missing layer `{name}`"))
    };
    let mets: Vec<LayerId> = cell_pdk
        .layers
        .routing_metals
        .iter()
        .map(|n| lid(n))
        .collect::<Result<_, _>>()?;
    let vias: Vec<LayerId> = cell_pdk
        .layers
        .routing_vias
        .iter()
        .map(|n| lid(n))
        .collect::<Result<_, _>>()?;
    if mets.is_empty() || vias.len() + 1 != mets.len() {
        return Err(format!(
            "PDK routing stack malformed: {} metals need {} vias, got {}",
            mets.len(),
            mets.len().saturating_sub(1),
            vias.len()
        ));
    }
    let em = deck.erc.em_min_width_nm.max(0);
    // Round up to 10 nm so centered squares keep centers/edges on the 5 nm grid.
    let round10 = |v: i32| (v + 9) / 10 * 10;
    let mut cut = Vec::with_capacity(vias.len());
    for (k, &v) in vias.iter().enumerate() {
        cut.push(rule_min_width(deck, v).ok_or_else(|| {
            format!(
                "PDK missing min-width rule for routing via `{}`",
                cell_pdk.layers.routing_vias[k]
            )
        })?);
    }
    let n = mets.len();
    let mut dims = RouteDims {
        wire_w: Vec::with_capacity(n),
        spacing: Vec::with_capacity(n),
        min_area: Vec::with_capacity(n),
        pad: Vec::with_capacity(n),
        cut,
    };
    for (i, &m) in mets.iter().enumerate() {
        let min_w = rule_min_width(deck, m).unwrap_or(0);
        let space = rule_min_spacing(deck, m).unwrap_or(0);
        let area = rule_min_area(deck, m);
        let wire_w = round10(base_width.max(min_w).max(em));
        let mut pad = wire_w;
        if i > 0 {
            pad = pad.max(dims.cut[i - 1] + 2 * rule_enclosure(deck, m, vias[i - 1]).unwrap_or(0));
        }
        if i < vias.len() {
            pad = pad.max(dims.cut[i] + 2 * rule_enclosure(deck, m, vias[i]).unwrap_or(0));
        }
        pad = pad.max(min_area_side(area));
        dims.wire_w.push(wire_w);
        dims.spacing.push(space);
        dims.min_area.push(area);
        dims.pad.push(round10(pad));
    }
    Ok(dims)
}

/// Nearest track-lattice coordinate (node centers sit at ix*pitch + pitch/2).
/// Interface via stacks and their routed terminals snap here so the stack's
/// cuts either coincide with the router's own vias or sit whole-track
/// multiples away — never at a sub-spacing offset.
fn snap_to_track(c: i32, extent: i32, pitch: i32) -> i32 {
    let pitch = pitch.max(1);
    let n = (extent / pitch).max(2);
    // floor(c/pitch) == round((c - pitch/2)/pitch) for c >= 0.
    let ix = c.max(0) / pitch;
    ix.clamp(0, n - 1) * pitch + pitch / 2
}

/// Fill rect merging two same-potential well rects whose edge gap is under
/// `space` (axis gap, or Euclidean at corners). The fill extends `min_w` into
/// each rect along a gap axis so the merged shape has no sub-min neck.
/// `None` when the rects touch/overlap already or are legally far apart.
fn well_bridge(
    a: (i32, i32, i32, i32),
    b: (i32, i32, i32, i32),
    space: i32,
    min_w: i32,
) -> Option<(i32, i32, i32, i32)> {
    let dx = (b.0 - a.2).max(a.0 - b.2).max(0);
    let dy = (b.1 - a.3).max(a.1 - b.3).max(0);
    if (dx == 0 && dy == 0)
        || i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
            >= i64::from(space) * i64::from(space)
    {
        return None;
    }
    let span = |gap: i32, a_lo: i32, a_hi: i32, b_lo: i32, b_hi: i32| {
        if gap > 0 {
            // Gap axis: cover the gap plus min_w into each rect.
            (a_hi.min(b_hi) - min_w, a_lo.max(b_lo) + min_w)
        } else {
            // Overlap axis: the shared band (inside both rects).
            // ponytail: a band under min_w keeps its size — row-aligned analog
            // cells overlap by full cell height in practice.
            (a_lo.max(b_lo), a_hi.min(b_hi))
        }
    };
    let (x0, x1) = span(dx, a.0, a.2, b.0, b.2);
    let (y0, y1) = span(dy, a.1, a.3, b.1, b.3);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

#[cfg(test)]
mod route_dims_tests {
    use super::*;

    fn sky_deck() -> Deck {
        Deck::from_json(
            r#"{
          "layers": {"met1":{"layer":68,"datatype":20},"via1":{"layer":68,"datatype":44},
                     "met2":{"layer":69,"datatype":20},"via2":{"layer":69,"datatype":44},
                     "met3":{"layer":70,"datatype":20},"via3":{"layer":70,"datatype":44},
                     "met4":{"layer":71,"datatype":20},"via4":{"layer":71,"datatype":44},
                     "met5":{"layer":72,"datatype":20}},
          "drc": {
            "min_width":{"layer":"met1","min":140},
            "min_spacing":{"layer":"met1","min":140},
            "min_area":{"layer":"met1","min":83000},
            "M2.1":{"kind":"min_width","layer":"met2","min":140},
            "M2.2":{"kind":"min_spacing","layer":"met2","min":140},
            "M2.AREA":{"kind":"min_area","layer":"met2","min":67600},
            "M3.1":{"kind":"min_width","layer":"met3","min":300},
            "M3.2":{"kind":"min_spacing","layer":"met3","min":300},
            "M3.AREA":{"kind":"min_area","layer":"met3","min":240000},
            "M4.1":{"kind":"min_width","layer":"met4","min":300},
            "M4.2":{"kind":"min_spacing","layer":"met4","min":300},
            "M4.AREA":{"kind":"min_area","layer":"met4","min":240000},
            "M5.1":{"kind":"min_width","layer":"met5","min":1600},
            "M5.2":{"kind":"min_spacing","layer":"met5","min":1600},
            "M5.AREA":{"kind":"min_area","layer":"met5","min":1600000},
            "VIA.1":{"kind":"min_width","layer":"via1","min":150},
            "ENC.VIA.4A":{"kind":"min_enclosure","outer":"met1","inner":"via1","min":55},
            "ENC.VIA.5A":{"kind":"min_enclosure","outer":"met2","inner":"via1","min":55},
            "VIA2.1":{"kind":"min_width","layer":"via2","min":170},
            "VIA2.ENC.MET2":{"kind":"min_enclosure","outer":"met2","inner":"via2","min":40},
            "VIA2.ENC.MET3":{"kind":"min_enclosure","outer":"met3","inner":"via2","min":65},
            "VIA3.1":{"kind":"min_width","layer":"via3","min":200},
            "VIA3.ENC.MET3":{"kind":"min_enclosure","outer":"met3","inner":"via3","min":60},
            "VIA3.ENC.MET4":{"kind":"min_enclosure","outer":"met4","inner":"via3","min":65},
            "VIA4.1":{"kind":"min_width","layer":"via4","min":800},
            "VIA4.ENC.MET4":{"kind":"min_enclosure","outer":"met4","inner":"via4","min":190},
            "VIA4.ENC.MET5":{"kind":"min_enclosure","outer":"met5","inner":"via4","min":310}
          }}"#,
        )
        .expect("test deck")
    }

    #[test]
    fn per_layer_dims_follow_deck_rules() {
        let deck = sky_deck();
        let dims = route_dims(&deck, &pnr_cells::pdk::Pdk::default(), 300).expect("dims");
        // Wires: >= max(base, layer min width, ERC EM min width).
        assert_eq!(dims.wire_w, vec![300, 300, 300, 300, 1600]);
        assert!(dims.wire_w.iter().all(|&w| w >= deck.erc.em_min_width_nm));
        // Cuts follow the via layers' min-width rules.
        assert_eq!(dims.cut, vec![150, 170, 200, 800]);
        // met4 pad encloses the 800nm via4 cut by 190 per side.
        assert!(dims.pad[3] >= 800 + 2 * 190);
        // met3 pad meets M3.AREA as a square.
        assert!(i64::from(dims.pad[2]) * i64::from(dims.pad[2]) >= 240_000);
        // met5 pad covers the wire width and the 310 via4 enclosure.
        assert!(dims.pad[4] >= 1600 && dims.pad[4] >= 800 + 2 * 310);
        // Track step = worst feature + spacing.
        assert_eq!(dims.track_steps()[4], dims.pad[4] + 1600);
    }

    #[test]
    fn enclosure_lookup_is_layer_pair_filtered() {
        let deck = sky_deck();
        let id = |n: &str| deck.layers.id(n).expect("layer");
        assert_eq!(rule_enclosure(&deck, id("met4"), id("via4")), Some(190));
        assert_eq!(rule_enclosure(&deck, id("met5"), id("via4")), Some(310));
        assert_eq!(rule_enclosure(&deck, id("met2"), id("via1")), Some(55));
        // Reversed pair matches nothing — the lookup is direction-sensitive.
        assert_eq!(rule_enclosure(&deck, id("via4"), id("met4")), None);
    }

    #[test]
    fn well_bridge_fills_sub_spacing_gaps() {
        // Two wells 760nm apart in x (< 1270 spacing): fill spans the gap and
        // extends min-width into each rect.
        let a = (0, 0, 2000, 5000);
        let b = (2760, 0, 5000, 5000);
        let r = well_bridge(a, b, 1270, 840).expect("bridged");
        assert_eq!(r, (2000 - 840, 0, 2760 + 840, 5000));
        // Legal spacing and overlapping wells produce no fill.
        assert_eq!(well_bridge(a, (3270, 0, 5000, 5000), 1270, 840), None);
        assert_eq!(well_bridge(a, (1000, 0, 5000, 5000), 1270, 840), None);
        // Diagonal corner gap under the Euclidean spacing gets a corner fill
        // reaching into both rects.
        let c = (2500, 5500, 5000, 8000);
        let r = well_bridge(a, c, 1270, 840).expect("corner bridged");
        assert!(r.0 < 2500 && r.1 < 5500 && r.2 > 2000 && r.3 > 5000);
    }
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
    iface: &[IfacePin],
) -> Result<(HashMap<u32, PolyId>, Vec<PlannedGuardRing>), String> {
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
    // PMOS well rects + their bulk nets, for same-potential well merging.
    let mut pmos_wells: Vec<((i32, i32, i32, i32), Vec<u32>)> = Vec::new();
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
                let bulk: Vec<u32> = g
                    .cells
                    .get(i)
                    .map(|cell| {
                        cell.pins
                            .iter()
                            .filter(|(name, _)| name == "B" || name.ends_with(".B"))
                            .map(|&(_, net)| net)
                            .collect()
                    })
                    .unwrap_or_default();
                pmos_wells.push(((b.xmin + dx, b.ymin + dy, b.xmax + dx, b.ymax + dy), bulk));
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

    // Same-potential nwell merge (NWELL.2): PMOS wells whose bulk pins share
    // a net are one electrical well — complementary-well spacing is meant for
    // wells at different potentials. Bridge sub-spacing gaps so the merged
    // shape reads as a single well instead of a spacing violation.
    if let Some(l) = nwell {
        let space = rule_min_spacing(deck, l).unwrap_or(0);
        let min_w = rule_min_width(deck, l).unwrap_or(0);
        for i in 0..pmos_wells.len() {
            for j in (i + 1)..pmos_wells.len() {
                let (ra, na) = &pmos_wells[i];
                let (rb, nb) = &pmos_wells[j];
                let shared =
                    (na.is_empty() && nb.is_empty()) || na.iter().any(|n| nb.contains(n));
                if !shared {
                    continue;
                }
                if let Some((x0, y0, x1, y1)) = well_bridge(*ra, *rb, space, min_w) {
                    store.add_rect(l, x0, y0, x1 - x0, y1 - y0);
                }
            }
        }
    }

    let rings = plan_guard_rings(g, gen, placed, trans, cell_pdk);
    emit_guard_rings(store, &rings, deck, cell_pdk)?;

    let mut net_sample: HashMap<u32, PolyId> = HashMap::new();
    let dims = route_dims(deck, cell_pdk, cfg.pad)?;
    let met_at = |layer: u32| mets[(layer as usize).min(mets.len() - 1)];
    let pad_at = |layer: u32| dims.pad[(layer as usize).min(dims.pad.len() - 1)];
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
    // Name every routed net at its sample polygon: named nets are externally
    // observable, so LVS reduction cannot collapse them as internal series
    // nodes (e.g. two mirror PMOS merging through an otherwise-bare VDD).
    for (&net, &p) in &net_sample {
        if let Some(name) = routed.net_names.get(net as usize) {
            store.net_labels.insert(p.0, name.clone());
        }
    }
    for v in &routed.vias {
        let (vx, vy) = (v.x + dx_global, v.y + dy_global);
        let k = (v.layer as usize).min(cuts.len() - 1);
        centered_square(store, cuts[k], vx, vy, dims.cut[k]);
        centered_square(store, met_at(v.layer), vx, vy, pad_at(v.layer));
        centered_square(store, met_at(v.layer + 1), vx, vy, pad_at(v.layer + 1));
    }

    for s in stacks {
        for &(ax, ay) in &s.accesses {
            centered_square(store, licon, ax + dx_global, ay + dy_global, cfg.via_size);
        }
        // li overlay for pins below li: a licon cut without li above is a
        // dangling contact (fails licon:li overlap DRC and breaks the
        // diff/poly -> li -> met1 connectivity chain). Pins already on li
        // keep their cell-drawn li — a blanket overlay bridges S/D fingers.
        if s.needs_li && !s.accesses.is_empty() {
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
    // met2 landing pad: via + 2x process enclosure, NOT full wire pad —
    // smaller footprint means fewer met2 track nodes blocked around the pin.
    // Enclosure is the (met2, via1)-filtered rule, not the first MinEnclosure
    // of any layer pair; the pad also meets met2's min-area square.
    let m1_via = dims.cut.first().copied().unwrap_or(cfg.via_size);
    let m2pad = ((m1_via + 2 * rule_enclosure(deck, met2, via1).unwrap_or(0))
        .max(min_area_side(*dims.min_area.get(1).unwrap_or(&0)))
        + 9)
        / 10
        * 10;
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
            centered_square(store, via1, px, py, m1_via);
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

    // Boundary interface pins: the pin square on its declared layer, flush
    // inside the die edge, plus a via/pad stack down to first metal so the
    // routed wire (which lands on met1/met2) is always physically connected.
    // The net label matches the routed net's label string exactly, so LVS
    // reduction keeps the port observable without a label conflict.
    for ip in iface {
        let (cx0, cy0) = ip.spec.center(placed.placement.die, ip.width, ip.grid);
        // Stack position on the track lattice (same snap as the routing
        // terminal): the stack's cuts coincide with the router's vias or sit
        // whole-track multiples away, never at a sub-spacing offset.
        let die = placed.placement.die;
        let pitch = cfg.routing.detailed.pitch;
        let (sx, sy) = (
            snap_to_track(cx0, die.0, pitch) + dx_global,
            snap_to_track(cy0, die.1, pitch) + dy_global,
        );
        let (cx, cy) = (cx0 + dx_global, cy0 + dy_global);
        let hw = ip.width / 2;
        // The exact resolved layer from resolve_interface_pins — indexing the
        // metal list here could shift on a deck missing a mid-stack metal and
        // panics when it is empty.
        let pin_layer = ip.layer_id;
        let pin_poly = store.add_rect(pin_layer, cx - hw, cy - hw, ip.width, ip.width);
        store.net_labels.insert(pin_poly.0, ip.spec.net.clone());
        // Per-layer via stack: each cut at its own rule size, each metal pad
        // sized for its wire width, both cut enclosures, and its min area —
        // e.g. sky130 via4 (800nm cut, 190/310 enclosures) under a met5 pin.
        // Pin-layer bridge covering both the edge-flush pin square and the
        // snapped stack point: keeps the pin connected to its stack (or, for
        // first-metal pins, to the routed landing) and encloses the top cut.
        if (sx, sy) != (cx, cy) {
            let (x0, y0) = (sx.min(cx) - hw, sy.min(cy) - hw);
            let (x1, y1) = (sx.max(cx) + hw, sy.max(cy) + hw);
            store.add_rect(pin_layer, x0, y0, x1 - x0, y1 - y0);
        }
        for k in 0..ip.met_idx.min(cuts.len()) {
            centered_square(store, cuts[k], sx, sy, dims.cut[k]);
        }
        for k in 0..ip.met_idx.min(mets.len()) {
            centered_square(store, mets[k], sx, sy, dims.pad[k]);
        }
        labels.push(crate::gds::TextLabel {
            x: cx,
            y: cy,
            layer: LABEL_LAYER,
            datatype: LABEL_DATATYPE,
            text: ip.spec.net.clone(),
        });
    }

    // Same-net notch repair: pad-row corner slivers (landing node pads vs pin
    // pad rows) are sub-min gaps inside one merged shape — fill with metal.
    for (layer, g) in gdsverify::drc::same_shape_gap_fills(store, deck)
        .map_err(|error| format!("same-shape gap repair failed: {error}"))?
    {
        store.add_rect(layer, g.xmin, g.ymin, g.xmax - g.xmin, g.ymax - g.ymin);
    }

    Ok((net_sample, rings))
}

// ═══════════════════════════════════════════════════════════════════════
//  Step 5: signoff
// ═══════════════════════════════════════════════════════════════════════

/// Rail classification for advanced signoff: nominal rail voltage for supply
/// nets (power 1.8 V, ground 0 V), `None` for signal nets.
fn rail_nominal_v(name: &str) -> Option<f64> {
    let l = name.to_ascii_lowercase();
    if pnr_constraints::SUPPLY_NAMES.contains(&l.as_str())
        || l.starts_with("vdd")
        || l.starts_with("vcc")
        || l.starts_with("vpwr")
    {
        Some(1.8)
    } else if pnr_constraints::GROUND_NAMES.contains(&l.as_str())
        || l.starts_with("vss")
        || l.starts_with("gnd")
        || l.starts_with("vgnd")
    {
        Some(0.0)
    } else {
        None
    }
}

/// Build a fully-populated advanced-signoff configuration from the real design
/// data available at signoff time: deck device recognition and density rules,
/// routed supply rails, drawn geometry, and planned guard rings. Fields the
/// caller supplied explicitly pass through untouched; only `None` fields are
/// auto-built, so `NotRun` only remains for checks with no derivable evidence
/// (e.g. no routed nets at all for the power family).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn build_signoff_config(
    store: &GeometryStore,
    deck: &Deck,
    g: &BipartiteHypergraph,
    net_names: &[String],
    wires: &[pnr_routing::Wire],
    rings: &[PlannedGuardRing],
    pad_nets: &[String],
    cell_pdk: &pnr_cells::pdk::Pdk,
    user: &gdsverify::SignoffConfig,
) -> gdsverify::SignoffConfig {
    // Conducting layers bottom-to-top: prefer the deck's declared connectivity
    // order, else reconstruct it from the cell PDK's layer roles.
    let conductors: Vec<LayerId> = if deck.connectivity.conductors.is_empty() {
        [
            cell_pdk.layers.diff.clone(),
            cell_pdk.layers.poly.clone(),
            cell_pdk.layers.li.clone(),
        ]
        .into_iter()
        .chain(cell_pdk.layers.routing_metals.iter().cloned())
        .filter_map(|n| deck.layers.id(&n))
        .collect()
    } else {
        deck.connectivity.conductors.clone()
    };
    let mets: Vec<LayerId> = cell_pdk
        .layers
        .routing_metals
        .iter()
        .filter_map(|n| deck.layers.id(n))
        .collect();
    let die = store.poly_bbox.iter().fold(None, |acc: Option<Bbox>, b| {
        Some(acc.map_or(*b, |a| Bbox {
            xmin: a.xmin.min(b.xmin),
            ymin: a.ymin.min(b.ymin),
            xmax: a.xmax.max(b.xmax),
            ymax: a.ymax.max(b.ymax),
        }))
    });
    let supplies: Vec<(usize, f64)> = net_names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| rail_nominal_v(n).map(|v| (i, v)))
        .collect();
    let net_wires = |ni: usize| wires.iter().filter(move |w| w.net == ni as u32);
    let wire_mid = |w: &pnr_routing::Wire| ((w.x0 + w.x1) / 2, (w.y0 + w.y1) / 2);

    // ── 1. Antenna: one rule per MOS (gate, channel) recognition pair; the
    // collectors are every conductor fabricated above the gate layer. ──
    let antenna = user.antenna.clone().or_else(|| {
        let mut pairs: Vec<(LayerId, LayerId)> = deck
            .devices
            .mos_rules
            .iter()
            .map(|r| (r.gate_layer, r.channel_layer))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        let rules: Vec<AntennaRule> = pairs
            .into_iter()
            .map(|(gate, channel)| {
                let above = conductors
                    .iter()
                    .position(|&l| l == gate)
                    .map_or(conductors.len(), |i| i + 1);
                AntennaRule {
                    id: format!(
                        "antenna_{}_{}",
                        deck.layers.name(gate),
                        deck.layers.name(channel)
                    ),
                    gates: vec![AntennaGate {
                        gate_layer: gate,
                        channel_layer: channel,
                    }],
                    collectors: conductors[above..]
                        .iter()
                        .map(|&layer| AntennaCollector {
                            layer,
                            measurement: AntennaMeasurement::Area,
                        })
                        .collect(),
                    max_egar: deck.erc.antenna_ratio,
                    diode: None,
                }
            })
            .filter(|r| !r.collectors.is_empty())
            .collect();
        (!rules.is_empty()).then_some(AntennaConfig {
            rules,
            cut_required: None,
        })
    });

    // ── 2. Density/CMP: one rule per deck density layer over the drawn die. ──
    let density_cmp = user.density_cmp.clone().or_else(|| {
        let die = die?;
        let mut windows: HashMap<LayerId, i32> = HashMap::new();
        let mut max_frac: HashMap<LayerId, f64> = HashMap::new();
        for rule in &deck.drc_rules {
            match rule {
                gdsverify::DrcRuleParam::MinDensity { layer, window, .. } => {
                    windows.entry(*layer).or_insert(*window);
                }
                gdsverify::DrcRuleParam::MaxDensity {
                    layer,
                    window,
                    max_frac: f,
                    ..
                } => {
                    windows.entry(*layer).or_insert(*window);
                    max_frac.insert(*layer, *f);
                }
                _ => {}
            }
        }
        let mut layers: Vec<LayerId> = windows.keys().copied().collect();
        layers.sort_unstable();
        let rules: Vec<DensityCmpRule> = layers
            .into_iter()
            .map(|layer| {
                // ponytail: window clamps to the block extent, so a sub-window
                // block gets one exact-cover window. Blocks larger than the
                // foundry window whose extent is not a step multiple will
                // report a coverage error — pick fill-aware steps at chip
                // assembly, where density is actually closed.
                let ww = windows[&layer].min(die.width()).max(1);
                let wh = windows[&layer].min(die.height()).max(1);
                DensityCmpRule {
                    id: format!("density_{}", deck.layers.name(layer)),
                    layer,
                    window_width: ww,
                    window_height: wh,
                    step_x: (ww / 2).max(1),
                    step_y: (wh / 2).max(1),
                    // Block-level min-density stays waived: filling below the
                    // window scale is chip-assembly fill's job (same rationale
                    // as final_drc_policy's density waiver).
                    min_density: None,
                    max_density: Some(max_frac.get(&layer).copied().unwrap_or(0.9)),
                    max_neighbor_delta: Some(0.35),
                    cmp: None,
                }
            })
            .collect();
        (!rules.is_empty()).then_some(DensityCmpConfig {
            die,
            rules,
            include_partial_windows: false,
        })
    });

    // ── 3. Power: lumped two-node rail model per routed supply net. ──
    // Measured rail geometry shared by the power, reliability, and ESD
    // sections: series square-count resistance, length, minimum width, and
    // endpoint midpoints. ponytail: lumped model; swap in the per-net PEX
    // solve when its seam is threaded to config-build time.
    struct RailModel {
        ni: usize,
        nominal: f64,
        r_ohm: f64,
        len_nm: i64,
        min_w: i32,
        src: (i32, i32),
        load: (i32, i32),
    }
    let sheet_res = |layer_idx: u32| -> f64 {
        mets.get((layer_idx as usize).min(mets.len().saturating_sub(1)))
            .and_then(|l| deck.pex.get(l))
            .map_or(0.125, |p| p.sheet_res_ohm_sq)
    };
    let rails: Vec<RailModel> = supplies
        .iter()
        .filter_map(|&(ni, nominal)| {
            let nw: Vec<&pnr_routing::Wire> = net_wires(ni).collect();
            let (&first, &last) = (nw.first()?, nw.last()?);
            let mut r_ohm = 0.0;
            let mut len_nm: i64 = 0;
            let mut min_w = i32::MAX;
            for w in &nw {
                let l = i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs());
                len_nm += l;
                min_w = min_w.min(w.width.max(1));
                r_ohm += sheet_res(w.layer) * l as f64 / f64::from(w.width.max(1));
            }
            Some(RailModel {
                ni,
                nominal,
                r_ohm,
                len_nm,
                min_w,
                src: wire_mid(first),
                load: wire_mid(last),
            })
        })
        .collect();
    // Worst-case static current per MOS attached to a rail: crude saturation
    // bound Idsat ≈ IDSAT_PER_SQUARE_A * W/L. The coefficient is a documented
    // placeholder for sky130-class 1.8 V devices (≈0.5 mA/um of width at the
    // 150 nm minimum length → ≈75 uA per W/L square). It deliberately
    // overestimates (every device at full drive simultaneously) so IR/EM
    // margins are conservative and falsifiable — unlike the old flat 1 uA/um
    // echo. Replace with simulated per-instance currents when a stimulus seam
    // exists.
    const IDSAT_PER_SQUARE_A: f64 = 75e-6;
    const BJT_EMITTER_A: f64 = 100e-6;
    let rail_load_a = |name: &str| -> f64 {
        g.net_id(name).map_or(0.0, |gnet| {
            let mut seen = HashSet::new();
            let mut total = 0.0;
            for &(cell, _) in g.pins_on_net(gnet) {
                if !seen.insert(cell) {
                    continue;
                }
                let c = &g.cells[cell as usize];
                let devs: Vec<&DeviceRecord> = if c.grouped_devices.is_empty() {
                    c.device.iter().collect()
                } else {
                    c.grouped_devices.iter().collect()
                };
                for d in devs {
                    match d.device_type {
                        DeviceType::Nmos | DeviceType::Pmos => {
                            total += IDSAT_PER_SQUARE_A * f64::from(d.w) / f64::from(d.l.max(1));
                        }
                        // ponytail: flat 100 uA emitter-current bound per BJT.
                        // The extracted device carries no bias point, so this
                        // is a documented placeholder (typical mirror/current-
                        // source leg for a sky130-class NPN); replace with a
                        // simulated per-instance Ic when a stimulus seam exists.
                        DeviceType::Bjt => total += BJT_EMITTER_A,
                        _ => {}
                    }
                }
            }
            total
        })
    };
    // A power grid is only meaningful when a positive-voltage (VDD-class) rail
    // exists: IR-drop percent checking needs a non-zero reference, and a
    // ground-only rail set produces an IR "no checkable nodes" Error. Leave
    // power=None in that case so the power family reports NotRun, which the
    // applicability policy then classifies as "not applicable: no supply rails".
    let has_power_rail = rails.iter().any(|r| r.nominal > 0.0);
    let power = user.power.clone().or_else(|| {
        if !has_power_rail {
            // No recognized routed VDD-class rail. An unmodeled real rail must
            // not read as verified, so IR/EM report NotRun instead of a
            // fabricated proxy grid that trivially passes (or a ground-only
            // grid that Errors).
            return None;
        }
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for rail in &rails {
            let name = &net_names[rail.ni];
            let src = nodes.len();
            nodes.push(PowerNode {
                id: format!("{name}_src"),
                x: rail.src.0,
                y: rail.src.1,
                nominal_voltage_v: rail.nominal,
                fixed_voltage_v: Some(rail.nominal),
                load_current_a: 0.0,
                check_ir_drop: false,
            });
            nodes.push(PowerNode {
                id: format!("{name}_load"),
                x: rail.load.0,
                y: rail.load.1,
                nominal_voltage_v: rail.nominal,
                fixed_voltage_v: None,
                load_current_a: rail_load_a(name),
                // ponytail: percent IR-drop is undefined on a 0V rail; ground
                // bounce is still bounded by the same edge current, just not
                // pct-checked here.
                check_ir_drop: rail.nominal != 0.0,
            });
            edges.push(PowerEdge {
                id: format!("{name}_rail"),
                from: src,
                to: src + 1,
                resistance_ohm: rail.r_ohm.max(1e-3),
                length_um: rail.len_nm as f64 / 1000.0,
                kind: PowerEdgeKind::Metal {
                    width_um: (f64::from(rail.min_w) / 1000.0).max(1e-3),
                    thickness_um: 0.35,
                },
                temperature_c: 25.0,
                max_current_density_a_per_um2: None,
                max_current_per_cut_a: None,
                blech_product_limit_a_per_um: None,
                em_exempt: false,
            });
        }
        Some(PowerSignoffConfig {
            grid: PowerGrid { nodes, edges },
            solver: PowerSolveConfig::default(),
            ir_drop: Some(IrDropConfig {
                max_drop_v: None,
                max_drop_pct: Some(5.0),
                max_overvoltage_v: None,
            }),
            electromigration: Some(ElectromigrationConfig {
                default_max_current_density_a_per_um2: Some(1.0),
                default_max_current_per_cut_a: Some(0.5),
                reference_temperature_c: 25.0,
                activation_energy_ev: 0.7,
                current_exponent: 2.0,
                max_temperature_c: Some(125.0),
            }),
        })
    });

    // One shared solve of the derived grid; reliability's measured voltages
    // come from here (the same solve the IR analysis sees) rather than from a
    // restated nominal.
    let power_solution = power
        .as_ref()
        .and_then(|p| gdsverify::solve_power_grid(&p.grid, &p.solver).ok());

    // ── 4. Reliability: only stresses measurable at block level. ──
    // Voltage stress: max solved node voltage per supply rail from the shared
    // power-grid solve. Aging stress ratio: measured supply over the 1.8 V
    // process nominal the lifetime model is calibrated at. Thermal stress is
    // not derivable from block geometry (there is no thermal solve here), so
    // no entry is invented for it. With no routed, solvable supply at all,
    // every entry would be invented — the config stays `None`, the check
    // reports NotRun, and the aggregate blocks honestly.
    let reliability = user.reliability.clone().or_else(|| {
        let solution = power_solution.as_ref()?;
        let voltage_stresses: Vec<VoltageStress> = rails
            .iter()
            .filter(|r| r.nominal > 0.0)
            .map(|r| {
                let name = &net_names[r.ni];
                let src_id = format!("{name}_src");
                let load_id = format!("{name}_load");
                let measured = solution
                    .node_voltages
                    .iter()
                    .filter(|nv| nv.id == src_id || nv.id == load_id)
                    .map(|nv| nv.voltage_v.abs())
                    .fold(0.0, f64::max);
                VoltageStress {
                    id: format!("{name}_rail"),
                    measured_abs_v: measured,
                    max_abs_v: r.nominal * 1.1,
                    location: None,
                }
            })
            .collect();
        let max_supply_v = rails.iter().map(|r| r.nominal).fold(0.0, f64::max);
        let has = |t: DeviceType| {
            g.cells.iter().any(|c| {
                c.device.as_ref().is_some_and(|d| d.device_type == t)
                    || c.grouped_devices.iter().any(|d| d.device_type == t)
            })
        };
        let aging_stresses: Vec<AgingStress> = if max_supply_v > 0.0 {
            [(DeviceType::Nmos, "HCI"), (DeviceType::Pmos, "NBTI")]
                .into_iter()
                .filter(|&(t, _)| has(t))
                .map(|(_, mech)| AgingStress {
                    id: format!("{mech}_supply"),
                    mechanism: mech.into(),
                    reference_lifetime_hours: 200_000.0,
                    reference_stress: 1.0,
                    // Measured routed supply over the 1.8 V calibration point.
                    applied_stress: max_supply_v / 1.8,
                    stress_exponent: 2.0,
                    reference_temperature_c: 25.0,
                    applied_temperature_c: 25.0,
                    activation_energy_ev: 0.6,
                    duty_cycle: 0.5,
                    location: None,
                })
                .collect()
        } else {
            Vec::new()
        };
        if voltage_stresses.is_empty() && aging_stresses.is_empty() {
            return None;
        }
        Some(ReliabilityConfig {
            required_lifetime_hours: 87_600.0, // 10 years
            voltage_stresses,
            thermal_stresses: Vec::new(),
            aging_stresses,
        })
    });

    // ── 5. ESD/latch-up: measured discharge capacity + per-site evidence. ──
    // An HBM ESD event enters through a package pad. A reusable block only
    // presents a pad when the caller declares one (pad_nets); its diode/clamp
    // ring lives at chip assembly (same tier as the density-fill waiver). So
    // the 500 mA HBM discharge-path requirement is only imposed when a pad net
    // is actually declared and routed. A block with a declared pad but no clamp
    // path still FAILS (actionable); an ordinary block is not spuriously failed
    // for lacking chip-IO-ring protection it is not responsible for. For a
    // padless block the discharge requirement instead models the on-block
    // supply body-diode carrying the block's OWN worst-case rail load, which
    // the junction it draws for those devices can meet and a junction-less
    // block cannot.
    let has_pad = pad_nets
        .iter()
        .any(|p| net_names.iter().any(|n| n == p) && rail_nominal_v(p).is_some());
    let esd_latchup = user.esd_latchup.clone().or_else(|| {
        // No drawn geometry at all: nothing to protect and nothing to
        // measure — honestly NotRun.
        let die = die?;
        let center = ((die.xmin + die.xmax) / 2, (die.ymin + die.ymax) / 2);
        let vdd_rail = rails.iter().find(|r| r.nominal > 0.0);
        let vss_rail = rails.iter().find(|r| r.nominal == 0.0);
        let (vx, vy) = vdd_rail.map_or(center, |r| r.src);
        let (gx, gy) = vss_rail.map_or(center, |r| r.src);
        let nodes = vec![
            EsdNode {
                id: "pad_vdd".into(),
                kind: EsdNodeKind::IoPad,
                x: vx,
                y: vy,
            },
            EsdNode {
                id: "vdd".into(),
                kind: EsdNodeKind::Power,
                x: vx,
                y: vy,
            },
            EsdNode {
                id: "vss".into(),
                kind: EsdNodeKind::Ground,
                x: gx,
                y: gy,
            },
        ];
        // Measured drawn junction (diff) area in um².
        let diff_area_um2: f64 = deck.layers.id(&cell_pdk.layers.diff).map_or(0.0, |diff| {
            store
                .polys_on_layer(diff)
                .map(|p| {
                    let b = store.poly_bbox[p.0 as usize];
                    f64::from(b.width()) * f64::from(b.height()) * 1e-6
                })
                .sum()
        });
        // Body-diode discharge topology (real ESD clamps live at the chip IO
        // ring), but its numbers are measurements, not tuned constants:
        // capacity scales with drawn junction area with NO floor — too little
        // junction area FAILS the path requirement below — and series
        // resistance is the measured supply-rail resistance plus a junction
        // spreading term. Coefficients are documented sky130-class
        // placeholders: STI diode failure ≈5 mA/um², spreading ≈50 ohm·um².
        const ESD_CAPACITY_A_PER_UM2: f64 = 5e-3;
        const ESD_SPREADING_OHM_UM2: f64 = 50.0;
        let capacity_a = ESD_CAPACITY_A_PER_UM2 * diff_area_um2;
        let spreading_ohm = if diff_area_um2 > 0.0 {
            ESD_SPREADING_OHM_UM2 / diff_area_um2
        } else {
            f64::INFINITY
        };
        let mut edges = Vec::new();
        if capacity_a > 0.0 {
            if let Some(vr) = vdd_rail {
                // Pad → VDD → substrate: HBM entry down the powered rail.
                edges.push(EsdEdge {
                    id: "pad_entry".into(),
                    from: 0,
                    to: 1,
                    bidirectional: true,
                    resistance_ohm: vr.r_ohm,
                    current_capacity_a: capacity_a,
                    clamp_voltage_v: 0.0,
                });
                edges.push(EsdEdge {
                    id: "body_diode".into(),
                    from: 1,
                    to: 2,
                    bidirectional: true,
                    resistance_ohm: spreading_ohm + vss_rail.map_or(0.0, |r| r.r_ohm),
                    current_capacity_a: capacity_a,
                    clamp_voltage_v: 1.0,
                });
            } else {
                // Ground-only block: the only discharge topology is the drain
                // junction's substrate body diode straight to the ground rail.
                edges.push(EsdEdge {
                    id: "substrate_diode".into(),
                    from: 0,
                    to: 2,
                    bidirectional: true,
                    resistance_ohm: spreading_ohm + vss_rail.map_or(0.0, |r| r.r_ohm),
                    current_capacity_a: capacity_a,
                    clamp_voltage_v: 1.0,
                });
            }
        }
        // Required discharge current: chip-pad HBM (0.5 A) only for a declared,
        // routed pad; otherwise the block's own worst-case rail load, which the
        // junction it draws for those devices meets and a junction-less block
        // cannot. Zero junction area leaves no discharge edge, so the path
        // reports a real violation rather than being tuned to pass.
        let block_load_a = supplies
            .iter()
            .map(|&(ni, _)| rail_load_a(&net_names[ni]))
            .fold(0.0, f64::max)
            .max(1e-3);
        let required_current_a = if has_pad { 0.5 } else { block_load_a };
        // Path-resistance limit is a bound on discharge overvoltage (I·R): what
        // matters is that the clamped node stays under the ESD budget. Derive it
        // from that budget and the required current, not a fixed chip-pad ohm
        // value — 2.5 V / 0.5 A recovers the classic 5 ohm HBM limit, while a
        // block sinking only its own mA-scale load is correctly allowed the
        // higher series resistance of a small on-block junction. A junction-less
        // path (infinite spreading) still blows any finite budget and FAILS.
        const ESD_OVERVOLTAGE_BUDGET_V: f64 = 2.5;
        let max_path_resistance_ohm = ESD_OVERVOLTAGE_BUDGET_V / required_current_a;
        let esd_paths = vec![EsdPathRequirement {
            id: "pad_vdd_to_ground".into(),
            source: 0,
            explicit_targets: Vec::new(),
            target_kinds: vec![EsdNodeKind::Ground],
            required_current_a,
            max_path_resistance_ohm,
            max_clamp_voltage_v: 2.0,
        }];

        // Latch-up sites: one per drawn nwell region (each PMOS well is a
        // potential injector). Evidence comes from real geometry only: a
        // planned guard ring where one encloses the site, otherwise the
        // nearest drawn tap contact (li polygons — the same tap proxy the
        // missing_tie ERC rule measures against). No taps at all → no
        // evidence → the site genuinely fails.
        let tie_um = f64::from(deck.erc.tie_max_dist_nm) / 1000.0;
        let taps: Vec<Bbox> = deck
            .layers
            .id(&cell_pdk.layers.li)
            .map(|li| {
                store
                    .polys_on_layer(li)
                    .map(|p| store.poly_bbox[p.0 as usize])
                    .collect()
            })
            .unwrap_or_default();
        // (dist_um, min_drawn_dim_um) of the nearest tap to (x, y).
        let nearest_tap = |x: i32, y: i32| -> Option<(f64, f64)> {
            taps.iter()
                .map(|b| {
                    let dx = f64::from(x - (b.xmin + b.xmax) / 2);
                    let dy = f64::from(y - (b.ymin + b.ymax) / 2);
                    (
                        dx.hypot(dy) / 1000.0,
                        f64::from(b.width().min(b.height())) / 1000.0,
                    )
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        };
        let min_ring_w_um = (f64::from(cell_pdk.min_guard_ring_width) / 1000.0).max(1e-3);
        let min_tap_w_um = (f64::from(cell_pdk.contact) / 1000.0).max(1e-3);
        let inside =
            |x: i32, y: i32, b: &Bbox| x >= b.xmin && x <= b.xmax && y >= b.ymin && y <= b.ymax;
        // Tap-based evidence for a site with no enclosing guard ring.
        let tap_evidence = |id: String, x: i32, y: i32| {
            nearest_tap(x, y).map(|(dist_um, w_um)| GuardRingEvidence {
                id,
                bias_node: 2, // substrate taps tie to ground
                // ponytail: a point tap has no ring continuity to fail; the
                // falsifiable measurements here are drawn tap width and
                // distance, both against independent PDK/deck limits.
                continuous: true,
                width_um: w_um,
                aggressor_distance_um: dist_um,
                victim_distance_um: dist_um,
                nearest_tap_distance_um: dist_um,
            })
        };
        // Carrier-collection distance limit. A point tap collects only within
        // the deck's tap-tie distance. A guard RING encloses the well on all
        // sides, so its single-side well-to-ring gap is a DRC spacing, not a
        // point-tap tie — it stays effective out to roughly the substrate
        // lateral diffusion length. ponytail: model that as GUARD_RING_REACH ×
        // the tie distance; a ring drawn far enough that carriers escape (gap
        // beyond that band) still FAILS. Refine to a real diffusion-length
        // solve if a substrate model is ever threaded here.
        const GUARD_RING_REACH: f64 = 4.0;
        let make_site = |id: String,
                         loc: (i32, i32),
                         min_w_um: f64,
                         bias: EsdNodeKind,
                         ring: Option<GuardRingEvidence>| {
            let max_dist = if ring.is_some() {
                tie_um * GUARD_RING_REACH
            } else {
                tie_um
            };
            LatchupSite {
                id,
                location: loc,
                guard_ring: ring,
                allowed_bias_kinds: vec![bias],
                min_guard_ring_width_um: min_w_um,
                max_aggressor_distance_um: max_dist,
                max_victim_distance_um: max_dist,
                max_tap_distance_um: max_dist,
            }
        };
        let mut latchup_sites = Vec::new();
        if let Some(nwell) = deck.layers.id(&cell_pdk.layers.nwell) {
            for (i, p) in store.polys_on_layer(nwell).enumerate() {
                let b = store.poly_bbox[p.0 as usize];
                let (cx, cy) = ((b.xmin + b.xmax) / 2, (b.ymin + b.ymax) / 2);
                // Ring-band nwell (between a planned ring's inner and outer
                // bbox) is the collector itself, not an injector site.
                if rings
                    .iter()
                    .any(|r| inside(cx, cy, &r.outer) && !inside(cx, cy, &r.inner))
                {
                    continue;
                }
                let ring = rings
                    .iter()
                    .enumerate()
                    .find(|(_, r)| inside(cx, cy, &r.inner));
                let (min_w_um, bias, evidence) = match ring {
                    Some((ri, r)) => {
                        // Measured gap from the well edge to the ring's inner
                        // edge (nearest side): the drawn collection distance.
                        let gap_um = f64::from(
                            [
                                b.xmin - r.inner.xmin,
                                r.inner.xmax - b.xmax,
                                b.ymin - r.inner.ymin,
                                r.inner.ymax - b.ymax,
                            ]
                            .into_iter()
                            .min()
                            .unwrap_or(0)
                            .max(0),
                        ) / 1000.0;
                        let ring_is_power =
                            rail_nominal_v(&r.connection_net).is_some_and(|v| v > 0.0);
                        (
                            min_ring_w_um,
                            if ring_is_power {
                                EsdNodeKind::Power
                            } else {
                                EsdNodeKind::Ground
                            },
                            Some(GuardRingEvidence {
                                id: format!("ring_{ri}"),
                                bias_node: if ring_is_power { 1 } else { 2 },
                                // Drawn as a closed ring by emit_guard_rings.
                                continuous: true,
                                width_um: (f64::from(r.width) / 1000.0).max(1e-3),
                                aggressor_distance_um: gap_um,
                                victim_distance_um: gap_um,
                                nearest_tap_distance_um: gap_um,
                            }),
                        )
                    }
                    None => (
                        min_tap_w_um,
                        EsdNodeKind::Ground,
                        tap_evidence(format!("nwell_{i}_tap"), cx, cy),
                    ),
                };
                latchup_sites.push(make_site(
                    format!("nwell_{i}"),
                    (cx, cy),
                    min_w_um,
                    bias,
                    evidence,
                ));
            }
        }
        if latchup_sites.is_empty() {
            // No PMOS well drawn: only the substrate half of the thyristor
            // exists — one site at die center whose evidence is the real
            // nearest drawn tap.
            latchup_sites.push(make_site(
                "substrate".into(),
                center,
                min_tap_w_um,
                EsdNodeKind::Ground,
                tap_evidence("substrate_tap".into(), center.0, center.1),
            ));
        }
        Some(EsdLatchupConfig {
            nodes,
            edges,
            esd_paths,
            latchup_sites,
        })
    });

    gdsverify::SignoffConfig {
        antenna,
        density_cmp,
        power,
        reliability,
        esd_latchup,
        layer_order: user
            .layer_order
            .clone()
            .or_else(|| (!conductors.is_empty()).then_some(conductors)),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_signoff(
    store: &GeometryStore,
    deck: &Deck,
    g: &BipartiteHypergraph,
    routed: &RoutingResult,
    net_sample: &HashMap<u32, PolyId>,
    rec: &ConstraintRecord,
    rings: &[PlannedGuardRing],
    pad_nets: &[String],
    cell_pdk: &pnr_cells::pdk::Pdk,
    advanced_config: &gdsverify::SignoffConfig,
) -> SignoffReport {
    let advanced_config = build_signoff_config(
        store,
        deck,
        g,
        &routed.net_names,
        &routed.wires,
        rings,
        pad_nets,
        cell_pdk,
        advanced_config,
    );
    let erc_report = gdsverify::run_erc(store, deck, &advanced_config);
    let advanced = erc_report.signoff;
    let erc_violations = erc_report.violations;
    // A VDD-class rail among the routed nets OR the schematic ports makes the
    // power family applicable; a ground-only or supply-less design does not.
    let has_supply_rail = routed
        .net_names
        .iter()
        .chain(g.ports.iter())
        .any(|n| rail_nominal_v(n).is_some_and(|v| v > 0.0));
    let drc = run_drc(store, deck);
    // Density is a tapeout rule, not a router-quality hint, but blocks smaller
    // than the density window cannot satisfy it — those markers are waived.
    let (drc_blocking, drc_waived_density) = final_drc_policy(&drc, deck, store);

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
                    device_mappings: Vec::new(),
                    net_mappings: Vec::new(),
                    witness: None,
                },
                pex: PexReport {
                    parasitics: Vec::new(),
                },
                contracts: Vec::new(),
                erc_violations,
                advanced,
                has_supply_rail,
            }
        }
    };
    let cmp_opts = CompareOpts {
        strict: deck.strict,
        w_tolerance: deck.w_tolerance.clone(),
        l_tolerance: deck.l_tolerance.clone(),
        pin_swaps: Vec::new(),
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
            if let Err(err) = &per_net {
                let gdsverify::pex::PexError::UnsupportedGeometry(diagnostics) = err;
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
            } else if routed.report.unrouted.contains(&b.net_name) {
                // The router FAILED this net — zero wires here is a defect,
                // not a legitimately geometry-free net, and must not satisfy
                // the parasitic budget.
                contract.consume("routing");
                contract.violate("routing", 1.0, "net failed routing: no wires to extract");
            } else if g.net_id(&b.net_name).is_some()
                && routed_net.map_or(true, |ni| routed.wires.iter().all(|w| w.net != ni as u32))
            {
                // The net exists in the schematic but was never given wire
                // geometry (single-pin nets are demoted to routing obstacles;
                // zero-wire nets draw nothing), so its routed parasitic
                // contribution is exactly zero and within any budget. Leaving
                // the contract Emitted would block tapeout on a net that has
                // no geometry to extract.
                contract.consume("routing");
                contract.satisfy("routing", "no routed wire geometry: zero routed parasitics");
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
        erc_violations,
        advanced,
        has_supply_rail,
    }
}

fn final_drc_policy(
    drc: &DrcReport,
    deck: &Deck,
    store: &GeometryStore,
) -> (Vec<Violation>, usize) {
    // min_density is a chip-assembly rule: a block smaller than the density
    // window cannot fill it, so those markers are waived (counted, not
    // blocking). The waiver applies only when NEITHER extent reaches a full
    // window — a layer that is a full window long in even one dimension hosts
    // real density windows along it, and waiving those would hide genuinely
    // sparse regions (e.g. a 3-window-long strip with an empty middle). A
    // layer with ZERO drawn shapes is NOT waived: an empty required layer is
    // a missing-layer failure, not a chip-assembly fill question.
    let sub_window = |v: &Violation| {
        v.kind == "min_density"
            && deck.drc_rules.iter().any(|r| match r {
                gdsverify::DrcRuleParam::MinDensity { layer, window, .. } => {
                    deck.layers.name(*layer) == v.layer
                        && layer_extent(store, *layer)
                            .is_some_and(|(w, h)| w < i64::from(*window) && h < i64::from(*window))
                }
                _ => false,
            })
    };
    let (waived, blocking): (Vec<Violation>, Vec<Violation>) =
        drc.violations.iter().cloned().partition(sub_window);
    (blocking, waived.len())
}

/// Drawn extent (width, height) of a layer, `None` when the layer is empty.
fn layer_extent(store: &GeometryStore, layer: LayerId) -> Option<(i64, i64)> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for p in store.polys_on_layer(layer) {
        let b = store.poly_bbox[p.0 as usize];
        bounds = Some(match bounds {
            None => (b.xmin, b.ymin, b.xmax, b.ymax),
            Some((x0, y0, x1, y1)) => {
                (x0.min(b.xmin), y0.min(b.ymin), x1.max(b.xmax), y1.max(b.ymax))
            }
        });
    }
    bounds.map(|(x0, y0, x1, y1)| (i64::from(x1) - i64::from(x0), i64::from(y1) - i64::from(y0)))
}

fn reference_netlist(g: &BipartiteHypergraph) -> RefNetlist {
    let mut devices = Vec::new();
    let mut ref_bjt = Vec::new();
    let mut ref_two_terminal = Vec::new();
    for c in &g.cells {
        let dev_list: Vec<&DeviceRecord> = if !c.grouped_devices.is_empty() {
            c.grouped_devices.iter().collect()
        } else if let Some(d) = &c.device {
            vec![d]
        } else {
            continue;
        };
        for d in dev_list {
            let net = |t: &str| -> String {
                let grouped_key = format!("{}.{t}", d.name);
                c.pins
                    .iter()
                    .find(|(p, _)| p == t || p == &grouped_key)
                    .map_or(String::new(), |(_, n)| g.nets[*n as usize].clone())
            };
            let kind = match d.device_type {
                DeviceType::Nmos => DeviceKind::Nmos,
                DeviceType::Pmos => DeviceKind::Pmos,
                DeviceType::Bjt => {
                    // Compared against extraction via the deck's bjt
                    // recognition rules (marker layers drawn by the BJT
                    // generator). Needs gdsverify >= fix/bjt-extraction
                    // (net-triple dedup + marker-aware gate check).
                    let kind = if d.model_name.to_ascii_lowercase().contains("pnp") {
                        DeviceKind::Pnp
                    } else {
                        DeviceKind::Npn
                    };
                    ref_bjt.push(gdsverify::lvs::RefBjt {
                        kind,
                        name: d.name.clone(),
                        collector: net("C"),
                        base: net("B"),
                        emitter: net("E"),
                    });
                    continue;
                }
                DeviceType::Res => {
                    // Matches extraction: the resistor generator's rpoly body
                    // extracts as one TwoTerminalKind::Resistor between the
                    // licon terminal nets. Compare is topology-driven (kind +
                    // nets only) — name/value/W/L do not participate.
                    ref_two_terminal.push(gdsverify::lvs::RefTwoTerminal {
                        kind: gdsverify::lvs::TwoTerminalKind::Resistor,
                        name: d.name.clone(),
                        terminal_a: net("P"),
                        terminal_b: net("N"),
                    });
                    continue;
                }
                _ => continue,
            };
            // ponytail: emit one reduced device — extractor's reduce_netlist
            // merges parallel fingers/instances, so the reference must match
            let m = i32::from(d.multiplier.max(1));
            // Seed the body from the schematic bulk pin so the parallel-merge
            // key below can distinguish devices with distinct bodies, the way
            // the extractor's key does. Compare itself ignores ref body until
            // well extraction lands, so this only affects merging.
            let body = {
                let b = net("B");
                (!b.is_empty()).then_some(b)
            };
            devices.push(RefDevice {
                kind: kind.clone(),
                gate: net("G"),
                source: net("S"),
                drain: net("D"),
                w: d.w * m,
                l: d.l,
                flavor: DeviceFlavor::Standard,
                body, ad: None, as_: None, pd: None, ps: None,
            });
        }
    }
    // Parallel-reduce separate instances the same way the extractor does
    // (S/D symmetric, same kind/flavor/gate/body/L): identical devices merge,
    // W sums. This key must stay aligned with the extractor's parallel_reduce
    // key (gdsverify crates/lvs/src/extract.rs: kind, flavor, gate, source,
    // drain, body, l, device_class) — a ref-side merge the extractor does not
    // perform (or vice versa) reads as a false LVS count mismatch.
    // device_class is not representable on RefDevice today, so it cannot
    // diverge here.
    type RefMergeKey = (
        DeviceKind,
        DeviceFlavor,
        String,
        String,
        String,
        Option<String>,
        i32,
    );
    let mut merged: Vec<RefDevice> = Vec::new();
    let mut index: HashMap<RefMergeKey, usize> = HashMap::new();
    for d in devices {
        let (a, b) = if d.source <= d.drain {
            (d.source.clone(), d.drain.clone())
        } else {
            (d.drain.clone(), d.source.clone())
        };
        let key = (
            d.kind.clone(),
            d.flavor,
            d.gate.clone(),
            a,
            b,
            d.body.clone(),
            d.l,
        );
        match index.entry(key) {
            std::collections::hash_map::Entry::Occupied(e) => merged[*e.get()].w += d.w,
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(merged.len());
                merged.push(d);
            }
        }
    }
    RefNetlist {
        devices: merged,
        net_seeds: std::collections::HashMap::new(),
        ref_two_terminal,
        ref_bjt,
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
    // Resolve the harness interface against the (grouped) graph before the
    // expensive block build: spec/net/layer errors surface immediately.
    let iface = resolve_interface_pins(
        cfg.interface.as_ref(),
        &graph,
        &full.deck,
        &full.cells,
        cfg.routing.detailed.pitch,
        full.netlist.grid(),
    )?;
    let built = build_block(
        &graph,
        &full.netlist,
        &full.deck,
        &full.cells,
        &block_constraints,
        cfg,
        &iface,
    )?;
    // Fixed die is a hard contract: placement/routing must fit — never grown.
    if let Some(d) = cfg.interface.as_ref().and_then(|i| i.die) {
        let p = &built.placed.placement;
        debug_assert_eq!(p.die, (d.w, d.h), "fixed die must pass through placement");
        let mut fail: Option<String> = None;
        for i in 0..p.x.len() {
            let (w, h) = p.sizes[i];
            if p.x[i] - w / 2 < 0 || p.y[i] - h / 2 < 0 || p.x[i] + w / 2 > d.w || p.y[i] + h / 2 > d.h
            {
                fail = Some(format!(
                    "cell `{}` footprint exceeds the die",
                    graph.cells[i].name
                ));
                break;
            }
        }
        if fail.is_none() && built.placed.report.overlap_final > 0.5 {
            fail = Some(format!(
                "{:.0} nm^2 residual cell overlap",
                built.placed.report.overlap_final
            ));
        }
        if fail.is_none() && !built.routed.report.unrouted.is_empty() {
            fail = Some(format!(
                "{} net(s) unrouted",
                built.routed.report.unrouted.len()
            ));
        }
        if let Some(reason) = fail {
            return Err(format!("interface die {}x{} too small: {reason}", d.w, d.h));
        }
    }

    // Step 4: materialize cell, guard-ring, landing, wire, and via geometry in
    // one flat store, then canonicalize overlapping polygons once.
    let mut store = GeometryStore::new();
    let names: Vec<String> = graph.cells.iter().map(|cell| cell.name.clone()).collect();
    let mut labels: Vec<crate::gds::TextLabel> = Vec::new();
    let (mut net_samples, guard_rings) = merge_block_geometry(
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
        &iface,
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
        &guard_rings,
        &cfg.pad_nets,
        &full.cells,
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
        let _ = std::fs::write(
            dir.join("extracted_pex.spice"),
            dump_extracted_netlist(&result.store, &full.deck),
        );
    }
    Ok(result)
}

/// Layout-extracted netlist with per-net parasitics from the quasistatic BEM
/// field solver (more accurate than the analytical models). Falls back to
/// analytical — and says so in the dump — when the bridge rejects geometry.
fn dump_extracted_netlist(store: &GeometryStore, deck: &Deck) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
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
            let _ = writeln!(out, "* extraction failed: {e}");
            return out;
        }
    };
    let name_of = |net: u32| -> String {
        ext.net_names
            .iter()
            .find(|(id, _)| **id == net)
            .map_or_else(|| format!("n{net}"), |(_, n)| n.clone())
    };

    let _ = writeln!(out, "* layout-extracted netlist (gdsverify)");
    let _ = writeln!(
        out,
        "* {} MOS, {} BJT, {} two-terminal devices",
        ext.devices.len(),
        ext.bjt_devices.len(),
        ext.two_terminal.len(),
    );
    for (i, d) in ext.devices.iter().enumerate() {
        let model = match d.kind {
            DeviceKind::Pmos => "pmos",
            _ => "nmos",
        };
        let _ = writeln!(
            out,
            "M{i} {} {} {} {} {model} W={}n L={}n",
            name_of(d.drain),
            name_of(d.gate),
            name_of(d.source),
            name_of(d.body),
            d.w,
            d.l,
        );
    }
    for (i, d) in ext.bjt_devices.iter().enumerate() {
        let model = if d.kind == DeviceKind::Pnp { "pnp" } else { "npn" };
        let _ = writeln!(
            out,
            "Q{i} {} {} {} {model}",
            name_of(d.collector),
            name_of(d.base),
            name_of(d.emitter),
        );
    }

    // Per-net R/C: quasistatic field solver first, analytical as a labelled
    // fallback so the dump never silently degrades.
    let (per_net, method) = match gdsverify::run_pex_by_net_with_accuracy_checked(
        store,
        deck,
        &ext.net_of_poly,
        gdsverify::Accuracy::Quasistatic,
    ) {
        Ok(p) => (p, "quasistatic field solver"),
        Err(err) => {
            let gdsverify::pex::PexError::UnsupportedGeometry(diags) = &err;
            for d in diags {
                let _ = writeln!(out, "* field-solver diagnostic: {d:?}");
            }
            (
                gdsverify::run_pex_by_net_with_accuracy(
                    store,
                    deck,
                    &ext.net_of_poly,
                    gdsverify::Accuracy::Analytical,
                ),
                "analytical (field-solver fallback)",
            )
        }
    };
    let _ = writeln!(out, "* per-net parasitics ({method}):");
    let mut nets: Vec<_> = per_net.iter().filter(|(&n, _)| n != u32::MAX).collect();
    nets.sort_by_key(|&(&n, _)| n);
    for (&net, p) in nets {
        let _ = writeln!(
            out,
            "* net {:<12} R = {:>9.3} ohm   C = {:>9.3} fF",
            name_of(net),
            p.r_ohm,
            p.cap_af / 1000.0,
        );
    }
    out
}
