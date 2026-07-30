//! Net classification — the front of the routing-constraint chain.
//!
//! [`netrole`](crate::netrole) answers "is this a rail or a clock" from the net's
//! *name*, which is what recognition needs. Routing needs more: a budget is only
//! as good as the class it is keyed to, and the two classes that matter most for
//! analog are invisible to a name.
//!
//! * **Sensitive** — a small-signal reference: a bias rail, a bandgap output, a
//!   differential input. These are the *victims* coupling budgets and shielding
//!   exist to protect. Structurally they are gate-facing nets, often with no DC
//!   path at all, feeding matched devices.
//! * **Substrate** — a net that only ever reaches bulk terminals.
//!
//! Neither can be recovered from `vbias` vs `vfoo`, so both are recognised from
//! the hypergraph. The result carries the per-net budgets the routing tier scores
//! against, which is what finally dissolves the `metadata` staging tier into real
//! rules (`backend/TODO.md` §5).

use analog::metadata::{NetClass, NetClassification, VoltDomain};
use pnr_core::ids::NetId;
use pnr_core::BipartiteHypergraph;

use crate::netrole::NetRole;

/// Terminal index of a FET gate / drain / source in the hypergraph's per-device
/// net list (mirrors `analog::placement::matching_pair`).
const G: usize = 0;
const D: usize = 1;
const S: usize = 2;
const B: usize = 3;

/// Budget defaults per class.
///
/// A sensitive reference gets an order more spacing and a tighter coupling cap
/// than a plain signal, because that is the whole point of classifying it; a
/// supply gets a loose coupling budget but a tight resistance one (IR drop), which
/// is the opposite trade.
///
/// ponytail: one table, not a PDK read. Real values are process- and
/// tier-dependent; when the PDK grows a routing section, source them from it and
/// keep this as the fallback.
fn budgets(class: NetClass) -> (i64, i64, i64, bool) {
    // (max_r_mohm, max_c_af, max_coupling_af, shield)
    match class {
        // Tight on everything, and shielded: this is the net whose noise budget
        // sets the circuit's precision.
        NetClass::Sensitive => (200_000, 20_000_000, 2_000_000, true),
        // Clocks are aggressors, not victims: bound what they inject.
        NetClass::Clock => (500_000, 50_000_000, 5_000_000, true),
        // IR drop dominates; coupling barely matters on a low-impedance rail.
        NetClass::Supply | NetClass::Ground => (50_000, 1_000_000_000, 100_000_000, false),
        NetClass::Substrate => (100_000, 1_000_000_000, 100_000_000, false),
        NetClass::Signal => (1_000_000, 100_000_000, 20_000_000, false),
    }
}

/// Classify every net: name-derived role, upgraded by structure.
///
/// `sensitive_devices` are the devices whose matching the layout must protect —
/// the annotator's matched pairs. A net feeding one of their gates is the
/// small-signal path whose corruption shows up directly as offset.
#[must_use]
pub fn classify(
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    sensitive_devices: &[bool],
) -> Vec<NetClassification> {
    let n_nets = hg.net_names.len();
    let mut touches_gate = vec![false; n_nets];
    let mut touches_channel = vec![false; n_nets];
    let mut touches_bulk = vec![false; n_nets];
    let mut gate_of_sensitive = vec![false; n_nets];

    for (d, nets) in hg.device_nets.iter().enumerate() {
        let sensitive = sensitive_devices.get(d).copied().unwrap_or(false);
        if let Some(&g) = nets.get(G) {
            if let Some(slot) = touches_gate.get_mut(g.0 as usize) {
                *slot = true;
            }
            if sensitive {
                if let Some(slot) = gate_of_sensitive.get_mut(g.0 as usize) {
                    *slot = true;
                }
            }
        }
        for idx in [D, S] {
            if let Some(&c) = nets.get(idx) {
                if let Some(slot) = touches_channel.get_mut(c.0 as usize) {
                    *slot = true;
                }
            }
        }
        if let Some(&b) = nets.get(B) {
            if let Some(slot) = touches_bulk.get_mut(b.0 as usize) {
                *slot = true;
            }
        }
    }

    (0..n_nets)
        .map(|i| {
            let role = roles.get(i).copied().unwrap_or(NetRole::Signal);
            let class = classify_one(
                role,
                gate_of_sensitive[i],
                touches_gate[i],
                touches_channel[i],
                touches_bulk[i],
            );
            let (r, c, coup, shield) = budgets(class);
            NetClassification {
                net: NetId(i as u16),
                class,
                voltage_domain: Some(VoltDomain::Analog),
                shielding_required: shield,
                c_budget_af: Some(c),
                r_budget_mohm: Some(r),
                max_coupling_af: Some(coup),
                preferred_layers: Vec::new(),
            }
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
    fn sensitive_nets_get_the_tightest_budgets() {
        let (r_s, c_s, coup_s, shield_s) = budgets(NetClass::Sensitive);
        let (r_g, c_g, coup_g, shield_g) = budgets(NetClass::Signal);
        assert!(coup_s < coup_g, "a reference must tolerate less coupling than a signal");
        assert!(c_s < c_g && r_s < r_g);
        assert!(shield_s && !shield_g, "only the sensitive net is shielded");

        // Supplies invert the trade: loose coupling, tight resistance (IR drop).
        let (r_v, _, coup_v, _) = budgets(NetClass::Supply);
        assert!(r_v < r_s, "a rail is the tightest on resistance");
        assert!(coup_v > coup_g, "but the most tolerant of coupling");
    }
}
