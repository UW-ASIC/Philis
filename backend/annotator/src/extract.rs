//! Routing constraints. Structural rules self-extract over the hypergraph; the
//! per-net budgets come from the net's class ([`crate::classify`]).
//!
//! | rule                 | arm    | where                                          |
//! |----------------------|--------|------------------------------------------------|
//! | `Antenna`            | hard   | self-extracted                                 |
//! | `Differential`       | budget | every compound's mirrored non-rail net pair    |
//! | `CrosstalkExclusion` | budget | Voltage-set G×D; victim × aggressor nets       |
//! | `ParasiticBudget`    | budget | every budgeted net, C budget as drawn length   |
//! | `CouplingBudget`     | budget | every budgeted net, from its class and load;   |
//! |                      |        | own shield excluded, quiet rails weigh 0       |
//!
//! | `Shield`             | budget | victim nets, to [`quiet_ground`], only against |
//! |                      |        | an aggressor net                               |
//!
//! One batch per kind per arm: `gp::Prices` keys a budget's (λ, ρ) by kind.
//! EXT-24 also writes the intent's common nodes, stars, Kelvin requests, DAC
//! plate RC classes and shield references.

use analog::intent::{Aggressor, CommonNodeReq, Half, Inject, Intent, KelvinReq, MatchKind, RcClass, StarReq, Term};
use analog::metadata::{NetClass, NetClassification};
use analog::routing::{
    Antenna, CouplingBudget, CrosstalkExclusion, Differential, ParasiticBudget, Shield,
};
use analog::Requirements;
use pnr_core::ids::DeviceId;
use pnr_core::{BipartiteHypergraph, NetId, Routes};

fn victim_net(c: NetClass) -> bool {
    matches!(c, NetClass::Sensitive | NetClass::Bias | NetClass::Reference)
}

fn aggressor_net(c: NetClass) -> bool {
    matches!(c, NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy)
}

fn rail(c: NetClass) -> bool {
    matches!(c, NetClass::Supply | NetClass::Ground | NetClass::Substrate)
}

/// Assemble the routing [`Requirements`] from structure (EXT-24) and fill
/// `intent`'s routing facts. `classes` is indexed by net id; `set_roles` by
/// `intent.sets`.
#[must_use]
pub fn routing(
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    gate_um2: &[f32],
    process: &crate::ProcessNumbers,
    intent: &mut Intent,
    set_roles: &[crate::class::SetRole],
    policy: &crate::policy::Policy,
) -> Requirements<Routes> {
    let margin_pct = |c: NetClass| {
        policy.margin_pct[match c {
            NetClass::Sensitive | NetClass::Bias | NetClass::Reference => 0,
            NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy => 1,
            NetClass::Supply | NetClass::Ground => 2,
            _ => 3,
        }]
    };
    let spacing_multiple = |c: NetClass| {
        policy.spacing_multiple[match c {
            NetClass::Sensitive | NetClass::Bias | NetClass::Reference => 0,
            NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy => 1,
            NetClass::Signal | NetClass::DigitalStatic => 2,
            _ => 3,
        }]
    };
    let mut r = Requirements::<Routes>::default();
    // One Antenna per gate net, over the total gate area it drives.
    let mut gate_nm2 = vec![0i64; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if gate_um2[d] > 0.0 {
            if let Some(t) = hg.terminals[d]
                .iter()
                .position(|t| crate::terms::term_role(hg.kinds[d], t) == crate::terms::TermRole::FetGate)
            {
                gate_nm2[nets[t].0 as usize] += (f64::from(gate_um2[d]) * 1e6) as i64;
            }
        }
    }
    let antenna: Vec<Antenna> = process.antenna_max_ratio
        .into_iter()
        .flat_map(|ratio| {
            gate_nm2.iter().enumerate().filter(|&(_, &a)| a > 0).map(move |(n, &a)| Antenna {
                net: NetId(n as u16),
                max_ratio_x100: (ratio * 100.0) as i32,
                gate_area_nm2: a,
                margin_pct: policy.antenna_margin_pct,
                stack: process.stack,
            })
        })
        .collect();
    r.hard.push(Box::new(antenna));

    let class = |n: NetId| classes[n.0 as usize].class;
    let pin = |d: DeviceId, p: &str| crate::pattern::pin_net(hg, u32::from(d.0), p);
    // Compounds and sets can share nets: one rule per unordered net pair, first
    // orientation kept.
    let mut seen = std::collections::HashSet::new();
    let mut fresh = |x: NetId, y: NetId, kind: u8| seen.insert((kind, x.0.min(y.0), x.0.max(y.0)));
    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this. The victim's own shield is the remedy,
    // not an aggressor, and the quiet rails weigh nothing.
    let weights: &'static [f32] =
        Box::leak(CouplingBudget::default_weights(classes, hg.net_names.len()).into_boxed_slice());
    let mut diff: Vec<Differential> = Vec::new();
    let mut xtalk: Vec<CrosstalkExclusion> = Vec::new();
    for c in &intent.compounds {
        for &(x, y) in &c.net_pairs {
            if x != y && !rail(class(x)) && !rail(class(y)) && fresh(x, y, 0) {
                diff.push(Differential { pos: x, neg: y, max_len_delta_pct10: policy.diff_pct10, same_layer_required: true, stack: process.stack, aggressor_weight: Some(weights) });
            }
        }
    }
    let spacing = |a: NetId, b: NetId| process.route_space_nm * spacing_multiple(class(a)).max(spacing_multiple(class(b)));
    // (a) A Voltage set's gates against its drains: the input-to-output feedback path.
    for s in intent.sets.iter().filter(|s| s.kind == MatchKind::Voltage) {
        let of = |t: &str| s.members.iter().filter_map(|m| pin(m.device, t)).collect::<Vec<_>>();
        for g in of("G") {
            for d in of("D") {
                if g != d && fresh(g, d, 1) {
                    xtalk.push(CrosstalkExclusion { a: g, b: d, min_spacing_nm: spacing(g, d), margin_pct: 25 });
                }
            }
        }
    }
    // (b) Every routed victim net against every routed aggressor net.
    let routed_net = |n: usize| !hg.net_devices[n].is_empty();
    let nets = || (0..classes.len()).filter(|&n| routed_net(n)).map(|n| NetId(n as u16));
    for v in nets().filter(|&n| victim_net(class(n))) {
        for a in nets().filter(|&n| aggressor_net(class(n))) {
            if fresh(v, a, 1) {
                xtalk.push(CrosstalkExclusion { a: v, b: a, min_spacing_nm: spacing(v, a), margin_pct: 25 });
            }
        }
    }
    r.budget.push(Box::new(diff));
    r.budget.push(Box::new(xtalk));

    // Every net a device touches — a one-device net is still routed to its pin.
    let routed = || classes.iter().filter(|c| !hg.net_devices[c.net.0 as usize].is_empty());
    let par: Vec<ParasiticBudget> = routed()
        .filter_map(|c| {
            Some(ParasiticBudget {
                net: c.net,
                max_len_nm: (c.c_budget_af? as f32 * 1_000.0 / process.wire_af_per_um?) as i64,
                max_c_af: c.c_budget_af?,
                margin_pct: margin_pct(c.class),
                stack: process.stack,
            })
        })
        .collect();
    r.budget.push(Box::new(par));

    // Shields only against a real aggressor: with an aggressor net in the
    // design, every routed victim net is shielded by the quiet ground. No
    // aggressor, no shields — blanket shielding only adds load.
    let has_aggressor = routed().any(|c| aggressor_net(c.class));
    let ground = quiet_ground(classes, &hg.net_names, &intent.aggressors, hg).filter(|_| has_aggressor);
    if let Some((g, true)) = ground {
        intent.diagnostics.push(analog::intent::Diagnostic {
            kind: "shared_shield_return",
            devices: intent.aggressors.iter().filter(|a| a.inject == Inject::Switching && hg.device_nets[a.device.0 as usize].contains(&g)).map(|a| a.device).collect(),
            message: format!("shield return {} carries a switching aggressor's current (SUB-30)", hg.net_names[g.0 as usize]),
        });
    }
    let shield_ref = |c: &NetClassification| ground.map(|g| g.0).filter(|_| victim_net(c.class));
    for c in routed() {
        if let (Some(r), Some(f)) = (shield_ref(c), intent.nets.get_mut(c.net.0 as usize)) {
            f.shield_ref = Some(r);
        }
    }

    let coup: Vec<CouplingBudget> = routed()
        .filter_map(|c| {
            Some(CouplingBudget {
                net: c.net,
                max_coupling_af: c.max_coupling_af?,
                margin_pct: margin_pct(c.class),
                stack: process.stack,
                exclude: shield_ref(c),
                aggressor_weight: Some(weights),
            })
        })
        .collect();
    r.budget.push(Box::new(coup));

    let shields: Vec<Shield> = routed()
        .filter_map(|c| {
            Some(Shield {
                victim: c.net,
                reference: shield_ref(c)?,
                min_coverage_pct: policy.shield_coverage_pct,
                max_gap_nm: policy.shield_gap_spaces * process.route_space_nm,
            })
        })
        .collect();
    if ground.is_some() {
        r.budget.push(Box::new(shields));
    }
    common_nodes(hg, classes, intent, set_roles);
    r
}

/// The shield return (EXT-24, SUB-30): an analog ground by name (`avss`,
/// `vssa`, `agnd`), else a Ground net no Switching aggressor touches, else the
/// lowest-id Ground net, `shared` — its noise rides on the shield. Lowest id
/// within each tier; `None` without a Ground net.
#[must_use]
pub fn quiet_ground(classes: &[NetClassification], net_names: &[String], aggressors: &[Aggressor], hg: &BipartiteHypergraph) -> Option<(NetId, bool)> {
    let grounds = || classes.iter().filter(|c| c.class == NetClass::Ground).map(|c| c.net);
    let analog = |n: &NetId| ["avss", "vssa", "agnd"].iter().any(|k| net_names[n.0 as usize].to_ascii_lowercase().contains(k));
    let quiet = |n: &NetId| !aggressors.iter().any(|a| a.inject == Inject::Switching && hg.device_nets[a.device.0 as usize].contains(n));
    grounds().find(analog).or_else(|| grounds().find(quiet)).map(|n| (n, false)).or_else(|| grounds().next().map(|n| (n, true)))
}

/// EXT-24 steps 3–6: common nodes of sets sharing a source (emitter), star
/// points where other devices share that node, Kelvin sensing of a resistor
/// between a Voltage set's gates, and the RC class of DAC plates.
fn common_nodes(hg: &BipartiteHypergraph, classes: &[NetClassification], intent: &mut Intent, set_roles: &[crate::class::SetRole]) {
    let pin = |d: DeviceId, t: &str| crate::pattern::pin_net(hg, u32::from(d.0), t);
    let mut stars: Vec<StarReq> = Vec::new();
    for (si, s) in intent.sets.iter().enumerate() {
        let bjt = matches!(hg.kinds[s.members[0].device.0 as usize], pnr_core::DeviceKind::Npn | pnr_core::DeviceKind::Pnp);
        let fet = matches!(hg.kinds[s.members[0].device.0 as usize], pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos);
        let (t, term) = if bjt { ("E", Term::E) } else { ("S", Term::S) };
        let Some(net) = pin(s.members[0].device, t) else { continue };
        if !(fet || bjt) || s.members.len() < 2 || s.members.iter().any(|m| pin(m.device, t) != Some(net)) {
            continue;
        }
        let ids: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if let Some(k) = s.reference {
            for &m in ids.iter().filter(|&&m| m != ids[k]) {
                intent.common_nodes.push(CommonNodeReq { net, set: si as u16, a: vec![ids[k]], b: vec![m], term });
            }
        } else {
            let (a, b): (Vec<DeviceId>, Vec<DeviceId>) = if s.members.iter().all(|m| m.half.is_some()) {
                let half = |h| s.members.iter().filter(|m| m.half == Some(h)).map(|m| m.device).collect();
                (half(Half::A), half(Half::B))
            } else {
                let mut sorted = ids.clone();
                sorted.sort_unstable_by_key(|d| d.0);
                let k = sorted.len().div_ceil(2);
                (sorted[..k].to_vec(), sorted[k..].to_vec())
            };
            intent.common_nodes.push(CommonNodeReq { net, set: si as u16, a, b, term });
        }
        if stars.iter().any(|st| st.net == net) {
            continue;
        }
        // Other devices on the node: the one feed (a drain/collector) is the
        // star's root; anything else shares the matched return.
        let mut others: Vec<(DeviceId, &str)> = hg.net_devices[net.0 as usize]
            .iter()
            .filter(|d| !ids.contains(d))
            .flat_map(|&d| ["S", "D", "E", "C"].into_iter().filter(move |&t| pin(d, t) == Some(net)).map(move |t| (d, t)))
            .collect();
        others.dedup_by_key(|o| o.0);
        let root = others.iter().position(|o| o.1 == "D" || o.1 == "C").map(|i| others.remove(i)).map(|(d, t)| (d, if t == "D" { Term::D } else { Term::C }));
        if !others.is_empty() {
            stars.push(StarReq { net, root, branches: ids.iter().map(|&d| (d, term)).collect() });
        }
    }
    intent.stars = stars;
    // Kelvin: a resistor whose both ends are gates of one Voltage set.
    for (r, kind) in hg.kinds.iter().enumerate() {
        if *kind != pnr_core::DeviceKind::Resistor {
            continue;
        }
        let r = DeviceId(r as u16);
        let (Some(p), Some(n)) = (pin(r, "P"), pin(r, "N")) else { continue };
        for s in intent.sets.iter().filter(|s| s.kind == MatchKind::Voltage) {
            let sensed = |net: NetId| s.members.iter().filter(|m| pin(m.device, "G") == Some(net)).map(|m| (m.device, Term::G)).collect::<Vec<_>>();
            if p != n && !sensed(p).is_empty() && !sensed(n).is_empty() {
                intent.kelvins.push(KelvinReq { device: r, term: Term::P, sense: sensed(p) });
                intent.kelvins.push(KelvinReq { device: r, term: Term::N, sense: sensed(n) });
            }
        }
    }
    // DAC plates: the shared top plate is capacitive, each bottom plate a driven R.
    for (s, _) in intent.sets.iter().zip(set_roles).filter(|(_, &r)| r == crate::class::SetRole::DacBank) {
        let devs: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        let Some(&plate) = hg.device_nets[devs[0].0 as usize].iter().find(|n| devs.iter().all(|d| hg.device_nets[d.0 as usize].contains(n))) else { continue };
        let mut set_rc = |n: NetId, rc| {
            if let Some(f) = intent.nets.get_mut(n.0 as usize) {
                f.rc = rc;
            }
        };
        set_rc(plate, RcClass::C);
        for d in &devs {
            for &n in hg.device_nets[d.0 as usize].iter().filter(|&&n| n != plate && !rail(classes[n.0 as usize].class)) {
                set_rc(n, RcClass::R);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// An analog-root name wins over a lower id and needs no aggressor check.
    #[test]
    fn quiet_ground_prefers_analog_root() {
        let nl = pnr_core::Netlist {
            devices: vec![crate::tests::fet("M1", pnr_core::DeviceKind::Nmos, 2, 2, 0, 0, 1_000, 500)],
            nets: crate::tests::nets(&["VSS", "AVSS", "g"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let c = |n: u16, class| NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None };
        let classes = [c(0, NetClass::Ground), c(1, NetClass::Ground), c(2, NetClass::Signal)];
        assert_eq!(quiet_ground(&classes, &hg.net_names, &[], &hg), Some((NetId(1), false)));
        let names: Vec<String> = ["VSS", "GND2", "g"].map(String::from).to_vec();
        let agg = [Aggressor { device: DeviceId(0), inject: Inject::Switching, reason: "" }];
        assert_eq!(quiet_ground(&classes, &names, &agg, &hg), Some((NetId(1), false)), "the untouched ground");
        assert_eq!(quiet_ground(&classes[..1], &names, &agg, &hg), Some((NetId(0), true)), "shared");
        assert_eq!(quiet_ground(&classes[2..], &names, &[], &hg), None);
    }
}
