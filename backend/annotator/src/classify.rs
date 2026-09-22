//! Per-net class + budgets. Name gives rails/clocks ([`crate::netrole`]);
//! structure gives the two classes a name can't: **Sensitive** (feeds a matched
//! device's gate, or gates-only = a bias rail) and **Substrate** (bulk-only).

use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::NetId;
use pnr_core::BipartiteHypergraph;

use crate::netrole::NetRole;

/// FET terminal order in `hg.device_nets`: G, D, S, B.
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
        let mark = |slot: usize, v: &mut [bool]| {
            if let Some(n) = nets.get(slot) {
                v[n.0 as usize] = true;
            }
        };
        mark(G, &mut touches_gate);
        if sensitive_devices[d] {
            mark(G, &mut gate_of_sensitive);
        }
        mark(D, &mut touches_channel);
        mark(S, &mut touches_channel);
        mark(B, &mut touches_bulk);
    }

    (0..n_nets)
        .map(|i| {
            let class = classify_one(
                roles[i],
                gate_of_sensitive[i],
                touches_gate[i],
                touches_channel[i],
                touches_bulk[i],
            );
            let (r, c, coup, shield) = budgets(class);
            NetClassification {
                net: NetId(i as u16),
                class,
                shielding_required: shield,
                c_budget_af: Some(c),
                r_budget_mohm: Some(r),
                max_coupling_af: Some(coup),
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
