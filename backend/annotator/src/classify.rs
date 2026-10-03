//! Per-net class + budgets. Name gives rails/clocks ([`crate::netrole`]);
//! structure gives the two classes a name can't: **Sensitive** (feeds a matched
//! device's gate, or gates-only = a bias rail) and **Substrate** (bulk-only).

use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::NetId;
use pnr_core::BipartiteHypergraph;

use crate::netrole::NetRole;

/// Budgets as multiples of the capacitive load `c_load_af` the net drives:
/// `(wire C, total coupling)`, aF. A sensitive net (matched gate, bias rail)
/// may add at most its own load as wire, and again as coupling; a plain signal
/// twice that. Rails and substrate are low-impedance: unbudgeted.
///
/// ponytail: sized so minimum-size gates (rc_filter's 0.2 um2 inverter, whose
/// ring-to-ring wire alone is ~70% of its gate C) stay routable; precision
/// targets (10% wire / 2% coupling) are what a matched stage would want and
/// flag chain4/quad/rc_filter today. Rail aggressors count as coupling here.
fn budgets(class: NetClass, c_load_af: f32) -> (Option<i64>, Option<i64>) {
    let frac = |w: f32, k: f32| (Some((w * c_load_af) as i64), Some((k * c_load_af) as i64));
    match class {
        NetClass::Sensitive => frac(1.0, 1.0),
        NetClass::Signal | NetClass::Clock => frac(2.0, 2.0),
        NetClass::Supply | NetClass::Ground | NetClass::Substrate => (None, None),
    }
}

/// Classify every net: name-derived role, upgraded by structure.
///
/// `sensitive_devices` are the devices whose matching the layout must protect —
/// the annotator's matched pairs. A net feeding one of their gates is the
/// small-signal path whose corruption shows up directly as offset.
///
/// `gate_um2` (per device, `0` for non-FETs) sizes the budgets: a net's load is
/// the gate area it drives; a net driving no gate (a drain, an output) is held
/// to the circuit's smallest gate load. No FET gates at all: unbudgeted. A net
/// on a capacitor plate is unbudgeted too: its limit is an array spec
/// (settling, code-dependent error) the netlist does not carry.
#[must_use]
pub fn classify(
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    sensitive_devices: &[bool],
    gate_um2: &[f32],
    gate_af_per_um2: Option<f32>,
) -> Vec<NetClassification> {
    let n_nets = hg.net_names.len();
    let mut touches_gate = vec![false; n_nets];
    let mut touches_channel = vec![false; n_nets];
    let mut touches_bulk = vec![false; n_nets];
    let mut gate_of_sensitive = vec![false; n_nets];
    let mut load_um2 = vec![0.0f32; n_nets];
    let mut on_plate = vec![false; n_nets];

    for (d, nets) in hg.device_nets.iter().enumerate() {
        for (t, n) in hg.terminals[d].iter().zip(nets) {
            let i = n.0 as usize;
            match crate::terms::term_role(hg.kinds[d], t) {
                crate::terms::TermRole::FetGate => {
                    touches_gate[i] = true;
                    load_um2[i] += gate_um2[d];
                    gate_of_sensitive[i] |= sensitive_devices[d];
                }
                crate::terms::TermRole::Channel
                | crate::terms::TermRole::BjtBase
                | crate::terms::TermRole::Passive => touches_channel[i] = true,
                crate::terms::TermRole::Body => touches_bulk[i] = true,
                crate::terms::TermRole::Plate => on_plate[i] = true,
            }
        }
    }

    let smallest = load_um2.iter().copied().filter(|&a| a > 0.0).reduce(f32::min);
    (0..n_nets)
        .map(|i| {
            let class = classify_one(
                roles[i],
                gate_of_sensitive[i],
                touches_gate[i],
                touches_channel[i],
                touches_bulk[i],
            );
            let load = if load_um2[i] > 0.0 { Some(load_um2[i]) } else { smallest };
            // A plate net's parasitics trace to array specs (ARR-03 code-
            // dependent error, ARR-05 settling), not a gate load: without
            // them the budget is unknown, never invented.
            let load = load.filter(|_| !on_plate[i]);
            let (c_budget_af, max_coupling_af) = load
                .zip(gate_af_per_um2)
                .map_or((None, None), |(a, cox)| budgets(class, a * cox));
            NetClassification { net: NetId(i as u16), class, c_budget_af, max_coupling_af }
        })
        .collect()
}

/// The classification rule itself, isolated so it is testable without a graph.
fn classify_one(
    role: NetRole,
    gate_of_sensitive: bool,
    touches_gate: bool,
    touches_channel: bool,
    touches_bulk: bool,
) -> NetClass {
    match role {
        // A rail's name is authoritative: nothing structural should demote it.
        NetRole::Supply => NetClass::Supply,
        NetRole::Ground => NetClass::Ground,
        NetRole::Clock => NetClass::Clock,
        NetRole::Signal => {
            if touches_bulk && !touches_gate && !touches_channel {
                // Only ever a body connection — substrate, not signal.
                NetClass::Substrate
            } else if gate_of_sensitive || (touches_gate && !touches_channel) {
                // Either it drives a matched device's gate, or it is a pure
                // high-impedance reference (gates only, no DC path) — a bias rail.
                // Both are the small-signal nets coupling budgets protect.
                NetClass::Sensitive
            } else {
                NetClass::Signal
            }
        }
    }
}

/// Count of each class, for reporting.
#[must_use]
pub fn census(classes: &[NetClassification]) -> Vec<(NetClass, usize)> {
    let mut out: Vec<(NetClass, usize)> = Vec::new();
    for c in classes {
        match out.iter_mut().find(|(k, _)| *k == c.class) {
            Some((_, n)) => *n += 1,
            None => out.push((c.class, 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rails_keep_their_named_role() {
        assert_eq!(classify_one(NetRole::Supply, false, false, true, false), NetClass::Supply);
        assert_eq!(classify_one(NetRole::Ground, false, false, true, false), NetClass::Ground);
        assert_eq!(classify_one(NetRole::Clock, false, true, false, false), NetClass::Clock);
    }

    #[test]
    fn a_gate_only_net_is_a_sensitive_reference() {
        // No DC path — gates only. This is a bias rail, the classic victim.
        assert_eq!(
            classify_one(NetRole::Signal, false, true, false, false),
            NetClass::Sensitive
        );
    }

    #[test]
    fn driving_a_matched_gate_makes_a_net_sensitive_even_with_a_dc_path() {
        // A differential input is driven *and* feeds a matched gate: still the
        // small-signal path whose coupling shows up as offset.
        assert_eq!(
            classify_one(NetRole::Signal, true, true, true, false),
            NetClass::Sensitive
        );
        // The same net without the matched-device connection is ordinary signal.
        assert_eq!(
            classify_one(NetRole::Signal, false, true, true, false),
            NetClass::Signal
        );
    }

    #[test]
    fn bulk_only_nets_are_substrate() {
        assert_eq!(
            classify_one(NetRole::Signal, false, false, false, true),
            NetClass::Substrate
        );
    }

    #[test]
    fn budgets_scale_with_the_load_and_tighten_on_sensitive_nets() {
        let (c_s, k_s) = budgets(NetClass::Sensitive, 100_000.0);
        let (c_g, k_g) = budgets(NetClass::Signal, 100_000.0);
        assert!(k_s < k_g && c_s < c_g, "a reference tolerates less than a signal");
        // 100 fF of gate: a sensitive net gets 100 fF of wire and of coupling.
        assert_eq!((c_s, k_s), (Some(100_000), Some(100_000)));
        assert!(budgets(NetClass::Sensitive, 1_000_000.0).1 > k_s, "a bigger load tolerates more");
        assert_eq!(budgets(NetClass::Supply, 100_000.0), (None, None), "rails are unbudgeted");
    }

    fn classify_netlist(nl: &pnr_core::netlist::Netlist) -> Vec<NetClassification> {
        let hg = pnr_core::BipartiteHypergraph::from_netlist(nl);
        let roles = crate::netrole::classify_nets(&hg, &crate::netrole::AnnotationConfig::default());
        let gates: Vec<f32> = nl.devices.iter().map(crate::gate_um2).collect();
        let sensitive = vec![false; nl.devices.len()];
        classify(&hg, &roles, &sensitive, &gates, None)
    }

    fn class_of(nl: &pnr_core::netlist::Netlist, classes: &[NetClassification], name: &str) -> NetClass {
        let i = nl.nets.iter().position(|n| n.name == name).unwrap();
        classes[i].class
    }

    #[test]
    fn bjt_terminals_are_not_gates() {
        use pnr_core::ids::NetId;
        use pnr_core::netlist::{Device, DeviceKind, Netlist};

        let nl = Netlist {
            devices: vec![
                Device {
                    name: "XQ1".into(),
                    kind: DeviceKind::Npn,
                    model: String::new(),
                    terminals: vec![
                        ("C".into(), NetId(0)),
                        ("B".into(), NetId(1)),
                        ("E".into(), NetId(2)),
                    ],
                    params: vec![],
                },
                Device {
                    name: "XQ2".into(),
                    kind: DeviceKind::Pnp,
                    model: String::new(),
                    terminals: vec![
                        ("C".into(), NetId(3)),
                        ("B".into(), NetId(1)),
                        ("E".into(), NetId(4)),
                    ],
                    params: vec![],
                },
            ],
            nets: crate::tests::nets(&["outn", "in", "VSS", "outp", "VDD"]),
        };
        let classes = classify_netlist(&nl);
        assert_eq!(class_of(&nl, &classes, "outn"), NetClass::Signal);
        assert_eq!(class_of(&nl, &classes, "outp"), NetClass::Signal);
        assert_eq!(class_of(&nl, &classes, "in"), NetClass::Signal);
    }

    #[test]
    fn resistor_ends_are_channels() {
        use pnr_core::ids::NetId;
        use pnr_core::netlist::{Device, DeviceKind, Netlist};

        let r1 = Device {
            name: "R1".into(),
            kind: DeviceKind::Resistor,
            model: String::new(),
            terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(1))],
            params: vec![],
        };
        let m1 = crate::tests::fet("M1", DeviceKind::Nmos, 0, 2, 3, 3, 1_000, 1_000);
        let nl = Netlist {
            devices: vec![r1, m1],
            nets: crate::tests::nets(&["a", "b", "dn", "VSS"]),
        };
        let classes = classify_netlist(&nl);
        assert_eq!(class_of(&nl, &classes, "a"), NetClass::Signal);

        // Control: without R1, `a` has no DC path off the gate and is Sensitive.
        let nl_no_r = Netlist { devices: vec![nl.devices[1].clone()], nets: nl.nets.clone() };
        let classes_no_r = classify_netlist(&nl_no_r);
        assert_eq!(class_of(&nl_no_r, &classes_no_r, "a"), NetClass::Sensitive);
    }
}
