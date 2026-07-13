//! LVS: extract a netlist from layout, then compare it to a reference schematic netlist.
//!
//! Pipeline:
//!   1. Connectivity extraction — union-find over nodes; channel polygons are split at gate
//!      crossings so that source and drain extract as distinct nets.
//!   2. Device extraction — gate-over-channel recognized via PDK rules; type from implant.
//!   3. Comparison — iterative partition refinement over both extracted and reference graphs.

pub mod types;
pub mod extract;
pub mod compare;
pub mod gpu_compare;
pub mod spice;
pub mod derived;
pub mod hierarchical;

pub use types::*;
pub use extract::{extract_netlist, extract_netlist_opts, reduce_netlist};
pub use compare::{compare, CompareOpts};
pub use spice::{to_spice, SpiceOpts, PortMap};
pub use derived::evaluate_derived_layers;
pub use hierarchical::{compare_hierarchical, HierCell, HierLvsResult, RefHierarchy};

use crate::geometry::GeometryStore;
use crate::params::Deck;
use crate::traits::{Backend, VerifyCheck};

/// LVS check implementing VerifyCheck.
pub struct LvsCheck {
    pub reference: RefNetlist,
}

impl VerifyCheck for LvsCheck {
    type Output = LvsResult;
    fn id(&self) -> &str { "lvs" }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> LvsResult {
        let opts = ExtractOpts { cut_required: deck.lvs_cut_required, ..Default::default() };
        let ext = match extract_netlist_opts(store, deck, &opts, backend) {
            Ok(e) => e,
            Err(e) => return LvsResult {
                matched: false, reason: format!("extraction failed: {}", e),
                extracted_devices: 0, nmos: 0, pmos: 0,
                ambiguous_classes: 0, label_conflicts: Vec::new(),
                mismatches: Vec::new(), floating_nets: Vec::new(),
            },
        };
        let cmp_opts = CompareOpts {
            strict: false,
            w_tolerance: deck.w_tolerance.clone(),
            l_tolerance: deck.l_tolerance.clone(),
        };
        compare(&ext, &self.reference, &cmp_opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{LayerDef, LayerTable, ConnectivityConfig, DeviceConfig, MosRule};
    use crate::geometry::GeometryStore;
    use std::collections::HashMap;

    fn layer_table() -> LayerTable {
        let names = ["nwell", "diff", "poly", "licon", "li", "mcon", "met1", "via1",
                     "met2", "nsdm", "psdm"];
        let mut defs: HashMap<String, LayerDef> = HashMap::new();
        for (i, n) in names.iter().enumerate() {
            defs.insert(n.to_string(), LayerDef { layer: i as i32 + 1, datatype: 0 });
        }
        LayerTable::from_defs(&defs)
    }

    fn test_deck() -> Deck {
        let lt = layer_table();
        let conductors = ["diff", "poly", "li", "met1", "met2"].iter()
            .filter_map(|n| lt.id(n)).collect();
        let vias = vec![
            (lt.id("licon").unwrap(), vec![lt.id("diff").unwrap(), lt.id("poly").unwrap(), lt.id("li").unwrap()]),
            (lt.id("mcon").unwrap(), vec![lt.id("li").unwrap(), lt.id("met1").unwrap()]),
            (lt.id("via1").unwrap(), vec![lt.id("met1").unwrap(), lt.id("met2").unwrap()]),
        ];
        let mos_rules = vec![
            MosRule {
                name: "nmos".into(),
                gate_layer: lt.id("poly").unwrap(),
                channel_layer: lt.id("diff").unwrap(),
                type_implant: lt.id("nsdm").unwrap(),
                device_type: "nmos".into(),
                flavor_markers: Vec::new(),
                well_layer: None,
                device_class: None,
            },
            MosRule {
                name: "pmos".into(),
                gate_layer: lt.id("poly").unwrap(),
                channel_layer: lt.id("diff").unwrap(),
                type_implant: lt.id("psdm").unwrap(),
                device_type: "pmos".into(),
                flavor_markers: Vec::new(),
                well_layer: None,
                device_class: None,
            },
        ];
        Deck {
            layers: lt,
            drc_rules: Vec::new(),
            pex: HashMap::new(),
            dbu_nm: 1.0,
            lvs_cut_required: false,
            strict: false,
            connectivity: ConnectivityConfig { conductors, vias },
            devices: DeviceConfig {
                mos_rules, bjt_rules: Vec::new(),
                resistor_rules: Vec::new(), diode_rules: Vec::new(), cap_rules: Vec::new(),
            },
            w_tolerance: crate::schema::PropertyTolerance::default(),
            l_tolerance: crate::schema::PropertyTolerance::default(),
            fail_on_floating: false,
            intra_layer_touch: false,
            global_nets: Vec::new(),
        }
    }

    #[test]
    fn gate_split_two_fingers() {
        let deck = test_deck();
        let lt = &deck.layers;
        let diff = lt.id("diff").unwrap();
        let poly = lt.id("poly").unwrap();
        let li = lt.id("li").unwrap();
        let nsdm = lt.id("nsdm").unwrap();

        let mut st = GeometryStore::new();
        let d = st.add_rect(diff, 0, 0, 1000, 200);
        st.add_rect(poly, 200, -50, 60, 300);
        st.add_rect(poly, 600, -50, 60, 300);
        st.add_rect(li, 50, 50, 100, 100);
        st.add_rect(li, 350, 50, 100, 100);
        st.add_rect(li, 800, 50, 100, 100);
        let imp = st.add_rect(nsdm, -100, -100, 1200, 400);

        let ext = extract_netlist(&st, &deck).unwrap();
        assert_eq!(ext.devices.len(), 2);
        let (d0, d1) = (&ext.devices[0], &ext.devices[1]);
        assert_eq!(d0.kind, DeviceKind::Nmos);
        assert_eq!(d1.kind, DeviceKind::Nmos);
        assert_ne!(d0.gate, d1.gate);
        assert_ne!(d0.source, d0.drain);
        assert_ne!(d1.source, d1.drain);
        assert_eq!(d0.drain, d1.source);
        assert_ne!(d0.source, d1.drain);
        assert_eq!((d0.w, d0.l), (200, 60));
        assert_eq!((d1.w, d1.l), (200, 60));
        assert_eq!(ext.net_count, 5);
        assert_eq!(ext.used_nets, 5);
        assert_eq!(ext.net_of_poly.len(), st.poly_count());
        assert_eq!(ext.net_of_poly[d.0 as usize], d0.source);
        assert_eq!(ext.net_of_poly[imp.0 as usize], u32::MAX);
    }

    fn two_device_reference() -> RefNetlist {
        RefNetlist {
            devices: vec![
                RefDevice { kind: DeviceKind::Pmos, gate: "A".into(),
                            source: "VDD".into(), drain: "Y".into(), w: 0, l: 0,
                            flavor: DeviceFlavor::Standard },
                RefDevice { kind: DeviceKind::Nmos, gate: "A".into(),
                            source: "VSS".into(), drain: "Y".into(), w: 0, l: 0,
                            flavor: DeviceFlavor::Standard },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        }
    }

    #[test]
    fn compare_detects_swapped_gate() {
        let reference = two_device_reference();
        let cmp_opts = CompareOpts::default();

        let good = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Pmos, gate: 0, source: 1, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
                Device { kind: DeviceKind::Nmos, gate: 0, source: 3, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
            ],
            net_count: 4, used_nets: 4, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(), two_terminal: Vec::new(),
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let r = compare(&good, &reference, &cmp_opts);
        assert!(r.matched, "good wiring should match: {}", r.reason);

        let bad = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Pmos, gate: 0, source: 1, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
                Device { kind: DeviceKind::Nmos, gate: 4, source: 3, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
            ],
            net_count: 5, used_nets: 5, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(), two_terminal: Vec::new(),
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let r = compare(&bad, &reference, &cmp_opts);
        assert!(!r.matched);
        assert!(r.reason.starts_with("topology mismatch"), "reason: {}", r.reason);
    }

    fn inverter_extracted(nmos_w: i32) -> ExtractedNetlist {
        ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Pmos, gate: 0, source: 1, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 5000, l: 1000, device_class: None },
                Device { kind: DeviceKind::Nmos, gate: 0, source: 3, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: nmos_w, l: 1000, device_class: None },
            ],
            net_count: 4, used_nets: 4, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(), two_terminal: Vec::new(),
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        }
    }

    fn inverter_reference(w: (i32, i32), l: (i32, i32)) -> RefNetlist {
        RefNetlist {
            devices: vec![
                RefDevice { kind: DeviceKind::Pmos, gate: "A".into(),
                            source: "VDD".into(), drain: "Y".into(), w: w.0, l: l.0,
                            flavor: DeviceFlavor::Standard },
                RefDevice { kind: DeviceKind::Nmos, gate: "A".into(),
                            source: "VSS".into(), drain: "Y".into(), w: w.1, l: l.1,
                            flavor: DeviceFlavor::Standard },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        }
    }

    #[test]
    fn parametric_wrong_width_mismatch() {
        let reference = inverter_reference((5000, 2000), (1000, 1000));
        let cmp_opts = CompareOpts::default();

        let r = compare(&inverter_extracted(1500), &reference, &cmp_opts);
        assert!(!r.matched);
        assert!(r.reason.starts_with("parametric mismatch"), "reason: {}", r.reason);

        let r = compare(&inverter_extracted(2000), &reference, &cmp_opts);
        assert!(r.matched, "exact W/L should match: {}", r.reason);

        let r = compare(&inverter_extracted(2030), &reference, &cmp_opts);
        assert!(r.matched, "within-tolerance W should match: {}", r.reason);
    }

    #[test]
    fn parametric_two_fingers() {
        let deck = test_deck();
        let lt = &deck.layers;
        let diff = lt.id("diff").unwrap();
        let poly = lt.id("poly").unwrap();
        let li = lt.id("li").unwrap();
        let nsdm = lt.id("nsdm").unwrap();

        let mut st = GeometryStore::new();
        st.add_rect(diff, 0, 0, 1000, 5000);
        st.add_rect(poly, 200, -50, 60, 5100);
        st.add_rect(poly, 600, -50, 60, 5100);
        st.add_rect(li, 50, 2000, 100, 100);
        st.add_rect(li, 350, 2000, 100, 100);
        st.add_rect(li, 800, 2000, 100, 100);
        st.add_rect(nsdm, -100, -100, 1200, 5200);

        let ext = extract_netlist(&st, &deck).unwrap();
        assert_eq!(ext.devices.len(), 2);
        assert!(ext.devices.iter().all(|d| d.w == 5000 && d.l == 60));

        let make_ref = |w: i32| RefNetlist {
            devices: vec![
                RefDevice { kind: DeviceKind::Nmos, gate: "A1".into(),
                            source: "S".into(), drain: "M".into(), w, l: 60,
                            flavor: DeviceFlavor::Standard },
                RefDevice { kind: DeviceKind::Nmos, gate: "A2".into(),
                            source: "M".into(), drain: "D".into(), w, l: 60,
                            flavor: DeviceFlavor::Standard },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        };

        let cmp_opts = CompareOpts::default();
        let r = compare(&ext, &make_ref(5000), &cmp_opts);
        assert!(r.matched, "per-finger W=5000 should match: {}", r.reason);

        let r = compare(&ext, &make_ref(4000), &cmp_opts);
        assert!(!r.matched);
        assert!(r.reason.starts_with("parametric mismatch"), "reason: {}", r.reason);
    }

    #[test]
    fn parametric_legacy_mode_skips() {
        let cmp_opts = CompareOpts::default();
        let reference = inverter_reference((0, 2000), (1000, 1000));
        let r = compare(&inverter_extracted(999), &reference, &cmp_opts);
        assert!(r.matched, "legacy mode must skip parametric: {}", r.reason);

        let reference = inverter_reference((5000, 2000), (1000, 0));
        let r = compare(&inverter_extracted(999), &reference, &cmp_opts);
        assert!(r.matched, "l == 0 must also skip parametric: {}", r.reason);
    }

    #[test]
    fn cut_required_connectivity() {
        let deck = test_deck();
        let lt = &deck.layers;
        let diff = lt.id("diff").unwrap();
        let li = lt.id("li").unwrap();
        let licon = lt.id("licon").unwrap();
        let met1 = lt.id("met1").unwrap();
        let mcon = lt.id("mcon").unwrap();

        let mut st = GeometryStore::new();
        let d = st.add_rect(diff, 0, 0, 400, 200);
        let l = st.add_rect(li, 100, 50, 100, 100);

        let opts = ExtractOpts { cut_required: true, ..Default::default() };
        let ext = extract_netlist_opts(&st, &deck, &opts, Backend::Cpu).unwrap();
        assert_eq!(ext.net_count, 2, "li over diff without licon must not connect");
        assert_ne!(ext.net_of_poly[d.0 as usize], ext.net_of_poly[l.0 as usize]);

        let ext_default = extract_netlist(&st, &deck).unwrap();
        assert_eq!(ext_default.net_count, 1);

        st.add_rect(licon, 120, 70, 60, 60);
        let ext = extract_netlist_opts(&st, &deck, &opts, Backend::Cpu).unwrap();
        assert_eq!(ext.net_count, 1, "licon must bridge diff <-> li");
        assert_eq!(ext.net_of_poly[d.0 as usize], ext.net_of_poly[l.0 as usize]);

        let m = st.add_rect(met1, 100, 50, 100, 100);
        let ext = extract_netlist_opts(&st, &deck, &opts, Backend::Cpu).unwrap();
        assert_eq!(ext.net_count, 2, "met1 over li without mcon must not connect");
        assert_ne!(ext.net_of_poly[m.0 as usize], ext.net_of_poly[l.0 as usize]);
        st.add_rect(mcon, 130, 80, 40, 40);
        let ext = extract_netlist_opts(&st, &deck, &opts, Backend::Cpu).unwrap();
        assert_eq!(ext.net_count, 1, "mcon must bridge li <-> met1");
    }

    /// Symmetric diff pair: two NMOS with shared source, mirrored drain/gate.
    /// This is an automorphism — the old comparator would stall; the new one
    /// must break it and still match.
    #[test]
    fn automorphism_diff_pair() {
        let reference = RefNetlist {
            devices: vec![
                RefDevice { kind: DeviceKind::Nmos, gate: "INP".into(),
                            source: "TAIL".into(), drain: "OUTN".into(), w: 0, l: 0,
                            flavor: DeviceFlavor::Standard },
                RefDevice { kind: DeviceKind::Nmos, gate: "INN".into(),
                            source: "TAIL".into(), drain: "OUTP".into(), w: 0, l: 0,
                            flavor: DeviceFlavor::Standard },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        };

        // Extracted: same topology, different net numbering
        let good = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Nmos, gate: 10, source: 20, drain: 30, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
                Device { kind: DeviceKind::Nmos, gate: 11, source: 20, drain: 31, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
            ],
            net_count: 5, used_nets: 5, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(), two_terminal: Vec::new(),
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let r = compare(&good, &reference, &CompareOpts::default());
        assert!(r.matched, "symmetric diff pair should match: {}", r.reason);
        // Automorphism should be resolved (0 ambiguous after breaking)
        assert_eq!(r.ambiguous_classes, 0, "automorphisms should be fully resolved");
    }

    /// Two-terminal device on wrong net must be caught (not just count-checked).
    #[test]
    fn two_terminal_wrong_net() {
        let reference = RefNetlist {
            devices: vec![
                RefDevice { kind: DeviceKind::Nmos, gate: "A".into(),
                            source: "S".into(), drain: "D".into(), w: 0, l: 0,
                            flavor: DeviceFlavor::Standard },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: vec![
                RefTwoTerminal { kind: TwoTerminalKind::Resistor, name: "r1".into(),
                                 terminal_a: "D".into(), terminal_b: "VDD".into() },
            ],
            ref_bjt: Vec::new(),
        };

        // Correct: resistor between drain and VDD
        let good = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Nmos, gate: 0, source: 1, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
            ],
            net_count: 4, used_nets: 4, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(),
            two_terminal: vec![
                TwoTerminalDevice { kind: TwoTerminalKind::Resistor, name: "r1".into(),
                                    terminal_a: 2, terminal_b: 3, value: 100.0 },
            ],
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let r = compare(&good, &reference, &CompareOpts::default());
        assert!(r.matched, "correct resistor placement should match: {}", r.reason);

        // Wrong: resistor between gate and VDD (should be drain/source and VDD).
        // Gate is not S/D-permutable, so this is a genuine topology mismatch.
        let bad = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Nmos, gate: 0, source: 1, drain: 2, body: 0,
                         flavor: DeviceFlavor::Standard, w: 0, l: 0, device_class: None },
            ],
            net_count: 4, used_nets: 4, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(),
            two_terminal: vec![
                TwoTerminalDevice { kind: TwoTerminalKind::Resistor, name: "r1".into(),
                                    terminal_a: 0, terminal_b: 3, value: 100.0 },
            ],
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let r = compare(&bad, &reference, &CompareOpts::default());
        assert!(!r.matched, "resistor on gate (not S/D) should mismatch");
    }
}
