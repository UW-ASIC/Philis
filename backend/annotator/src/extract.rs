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

/// A coupling victim: Sensitive, Bias or Reference.
fn victim_net(c: NetClass) -> bool {
    matches!(c, NetClass::Sensitive | NetClass::Bias | NetClass::Reference)
}

/// A coupling aggressor: Clock, DigitalSwitching or Noisy.
fn aggressor_net(c: NetClass) -> bool {
    matches!(c, NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy)
}

/// A Supply, Ground or Substrate rail: carries no mirror and no differential rule.
pub(crate) fn rail(c: NetClass) -> bool {
    matches!(c, NetClass::Supply | NetClass::Ground | NetClass::Substrate)
}

/// Row of a 4-row policy table (`Policy::margin_pct`, `spacing_multiple`):
/// victims 0, aggressors 1, `third` 2, anything else 3.
fn policy_row(c: NetClass, third: bool) -> usize {
    if victim_net(c) {
        0
    } else if aggressor_net(c) {
        1
    } else if third {
        2
    } else {
        3
    }
}

/// Assemble the routing [`Requirements`] from structure (EXT-24) and fill
/// `intent`'s routing facts (net shield references and RC classes, common
/// nodes, stars, Kelvin requests, diagnostics). `classes` is indexed by net
/// id; `gate_um2` (µm²) by device; `set_roles` by `intent.sets`. `ports` and
/// `op` decide star points (EXT-24 step 4). The `Shield` batch is present only
/// when a shield return exists (see [`quiet_ground`]).
///
/// # Panics
/// If `classes` is shorter than `hg`'s nets, `gate_um2` than its devices,
/// `set_roles` than `intent.sets`, or a set is empty.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn routing(
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    gate_um2: &[f32],
    process: &crate::ProcessNumbers,
    intent: &mut Intent,
    set_roles: &[crate::class::SetRole],
    policy: &crate::policy::Policy,
    ports: &[NetId],
    op: Option<&crate::evidence::OpFacts>,
) -> Requirements<Routes> {
    let margin_pct = |c: NetClass| policy.margin_pct[policy_row(c, matches!(c, NetClass::Supply | NetClass::Ground))];
    let spacing_multiple = |c: NetClass| policy.spacing_multiple[policy_row(c, matches!(c, NetClass::Signal | NetClass::DigitalStatic))];
    let mut r = Requirements::<Routes>::default();
    r.hard.push(Box::new(antennas(hg, gate_um2, process, policy)));

    let class = |n: NetId| classes[n.0 as usize].class;
    let pin = |d: DeviceId, p: &str| crate::pattern::pin_net(hg, u32::from(d.0), p);
    // Compounds and sets can share nets: one rule per unordered net pair, first
    // orientation kept.
    let mut seen = std::collections::HashSet::new();
    let mut fresh = |x: NetId, y: NetId, kind: u8| seen.insert((kind, x.0.min(y.0), x.0.max(y.0)));
    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this. The victim's own shield is the remedy,
    // not an aggressor, and the quiet rails weigh nothing.
    // ponytail: leaked once per call, the rule types take `&'static [f32]`;
    // an owned or shared weight table on the rule types removes the leak.
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
    common_nodes(hg, classes, intent, set_roles, ports, op);
    r
}

/// One `Antenna` rule per gate net, over the total FET gate area (nm²) it
/// drives; none when the process has no antenna ratio.
fn antennas(hg: &BipartiteHypergraph, gate_um2: &[f32], process: &crate::ProcessNumbers, policy: &crate::policy::Policy) -> Vec<Antenna> {
    let Some(ratio) = process.antenna_max_ratio else { return Vec::new() };
    let mut gate_nm2 = vec![0i64; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if gate_um2[d] <= 0.0 {
            continue;
        }
        if let Some(t) = hg.terminals[d].iter().position(|t| crate::terms::term_role(hg.kinds[d], t) == crate::terms::TermRole::FetGate) {
            gate_nm2[nets[t].0 as usize] += (f64::from(gate_um2[d]) * 1e6) as i64;
        }
    }
    gate_nm2
        .iter()
        .enumerate()
        .filter(|&(_, &a)| a > 0)
        .map(|(n, &a)| Antenna { net: NetId(n as u16), max_ratio_x100: (ratio * 100.0) as i32, gate_area_nm2: a, margin_pct: policy.antenna_margin_pct, stack: process.stack })
        .collect()
}

/// The shield return (EXT-24, SUB-30): an analog ground by name (`avss`,
/// `vssa`, `agnd`), else a Ground net no Switching aggressor touches, else the
/// lowest-id Ground net, `shared` — its noise rides on the shield. Lowest id
/// within each tier; `None` without a Ground net. The flag is `true` for the
/// shared fallback.
///
/// # Panics
/// If a Ground class names a net past `net_names`, or an aggressor a device past `hg`.
#[must_use]
pub fn quiet_ground(classes: &[NetClassification], net_names: &[String], aggressors: &[Aggressor], hg: &BipartiteHypergraph) -> Option<(NetId, bool)> {
    let grounds = || classes.iter().filter(|c| c.class == NetClass::Ground).map(|c| c.net);
    let analog = |n: &NetId| ["avss", "vssa", "agnd"].iter().any(|k| net_names[n.0 as usize].to_ascii_lowercase().contains(k));
    let quiet = |n: &NetId| !aggressors.iter().any(|a| a.inject == Inject::Switching && hg.device_nets[a.device.0 as usize].contains(n));
    grounds().find(analog).or_else(|| grounds().find(quiet)).map(|n| (n, false)).or_else(|| grounds().next().map(|n| (n, true)))
}

/// Star threshold (EXT-24 step 4, **Philis threshold**): a shared node is a
/// star when non-member current exceeds this share of the members' current.
const STAR_CURRENT_SHARE: f64 = 0.01;

/// EXT-24 steps 3–6: common nodes of sets sharing a source (emitter), star
/// points where other devices share that node, Kelvin sensing of a resistor
/// between a Voltage set's gates, and the RC class of DAC plates. The node's
/// one feed is the port when it is one (no root), else a drain/collector.
/// With an op point covering every device on the node, a star needs
/// `Σ|I_others| > STAR_CURRENT_SHARE·Σ|I_members|`; else any other
/// S/D/E/C terminal makes one. Only FET and BJT sets of two or more whose
/// members all share the node count. Replaces `intent.stars`; appends to
/// `intent.common_nodes` and `intent.kelvins`.
fn common_nodes(
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    intent: &mut Intent,
    set_roles: &[crate::class::SetRole],
    ports: &[NetId],
    op: Option<&crate::evidence::OpFacts>,
) {
    let pin = |d: DeviceId, t: &str| crate::pattern::pin_net(hg, u32::from(d.0), t);
    let mut stars: Vec<StarReq> = Vec::new();
    for (si, s) in intent.sets.iter().enumerate() {
        let Some(m0) = s.members.first() else { continue };
        let kind = hg.kinds[m0.device.0 as usize];
        let (bjt, fet) = (crate::sets::bjt(kind), crate::sets::fet(kind));
        let (t, term) = if bjt { ("E", Term::E) } else { ("S", Term::S) };
        let Some(net) = pin(m0.device, t) else { continue };
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
        // Other devices on the node: the one feed (the port, else a
        // drain/collector) is the star's root; anything else shares the matched return.
        let mut others: Vec<(DeviceId, &str)> = hg.net_devices[net.0 as usize]
            .iter()
            .filter(|d| !ids.contains(d))
            .flat_map(|&d| ["S", "D", "E", "C"].into_iter().filter(move |&t| pin(d, t) == Some(net)).map(move |t| (d, t)))
            .collect();
        others.dedup_by_key(|o| o.0);
        let root = if ports.contains(&net) {
            None
        } else {
            others.iter().position(|o| o.1 == "D" || o.1 == "C").map(|i| others.remove(i)).map(|(d, t)| (d, if t == "D" { Term::D } else { Term::C }))
        };
        let amps = |ds: &mut dyn Iterator<Item = DeviceId>| -> Option<f64> { ds.map(|d| op?.dev.get(d.0 as usize).copied().flatten().map(|o| o.id_ua.abs())).sum() };
        let carries = match (amps(&mut others.iter().map(|o| o.0)), amps(&mut ids.iter().copied())) {
            (Some(i_others), Some(i_members)) => i_others > STAR_CURRENT_SHARE * i_members,
            _ => !others.is_empty(),
        };
        if carries {
            stars.push(StarReq { net, root, branches: ids.iter().map(|&d| (d, term)).collect() });
        }
    }
    intent.stars = stars;
    kelvins(hg, intent);
    dac_plates(hg, classes, intent, set_roles);
}

/// Kelvin requests (EXT-24 step 5): a resistor whose two ends, on distinct
/// nets, are both gates of one Voltage set senses each end at those gates.
fn kelvins(hg: &BipartiteHypergraph, intent: &mut Intent) {
    let pin = |d: DeviceId, t: &str| crate::pattern::pin_net(hg, u32::from(d.0), t);
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
}

/// DAC plate RC classes (EXT-24 step 6): per DacBank set, the net every member
/// shares is the capacitive top plate, each other non-rail member net a driven R.
fn dac_plates(hg: &BipartiteHypergraph, classes: &[NetClassification], intent: &mut Intent, set_roles: &[crate::class::SetRole]) {
    for (s, _) in intent.sets.iter().zip(set_roles).filter(|(_, &r)| r == crate::class::SetRole::DacBank) {
        let devs: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        let Some(d0) = devs.first() else { continue };
        let Some(&plate) = hg.device_nets[d0.0 as usize].iter().find(|n| devs.iter().all(|d| hg.device_nets[d.0 as usize].contains(n))) else { continue };
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

    // ---- cleanup(annotator-sets) step 2 ----

    use analog::intent::{ArrayStyle, ClassSource, ConstraintId, MatchClass, MatchSpec, Member, Origin};
    use pnr_core::netlist::{Device, DeviceKind, Netlist};

    fn spec(devs: &[u16], kind: MatchKind) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: devs.iter().map(|&d| Member { device: DeviceId(d), parallel: 1, series: 1, half: None }).collect(),
            reference: None,
            family: analog::intent::Family::of(DeviceKind::Nmos).unwrap(),
            kind,
            class: MatchClass::Moderate,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn signal_classes(n: usize, ground: &[u16]) -> Vec<NetClassification> {
        (0..n as u16).map(|i| NetClassification { net: NetId(i), class: if ground.contains(&i) { NetClass::Ground } else { NetClass::Signal }, c_budget_af: None, max_coupling_af: None }).collect()
    }

    #[test]
    fn class_predicates_and_policy_rows() {
        for c in [NetClass::Supply, NetClass::Ground, NetClass::Substrate] {
            assert!(rail(c) && !victim_net(c) && !aggressor_net(c), "{c:?}");
        }
        for c in [NetClass::Sensitive, NetClass::Bias, NetClass::Reference] {
            assert!(victim_net(c) && !rail(c) && !aggressor_net(c));
            assert_eq!(policy_row(c, true), 0, "a victim before the third row");
        }
        for c in [NetClass::Clock, NetClass::DigitalSwitching, NetClass::Noisy] {
            assert!(aggressor_net(c) && !rail(c));
            assert_eq!(policy_row(c, true), 1);
        }
        assert_eq!(policy_row(NetClass::Signal, true), 2);
        assert_eq!(policy_row(NetClass::Signal, false), 3);
    }

    /// One Antenna per gate net over the summed gate area; none without a ratio.
    #[test]
    fn antennas_sum_gate_area_per_net() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let policy = crate::policy::Policy::default();
        let gate_um2 = [1.0, 0.0, 0.5, 0.5, -1.0];
        assert!(antennas(&hg, &gate_um2, &crate::ProcessNumbers::default(), &policy).is_empty());
        let process = crate::ProcessNumbers { antenna_max_ratio: Some(4.0), ..Default::default() };
        let a = antennas(&hg, &gate_um2, &process, &policy);
        let got: Vec<(NetId, i32, i64)> = a.iter().map(|a| (a.net, a.max_ratio_x100, a.gate_area_nm2)).collect();
        assert_eq!(got, [(NetId(1), 400, 1_000_000), (NetId(6), 400, 1_000_000)]);
        assert!(a.iter().all(|a| a.margin_pct == policy.antenna_margin_pct));
    }

    /// The OTA's input pair shares vtail: one common node, halves by id; the
    /// tail's drain is the feed, so no star unless vtail is a port.
    #[test]
    fn common_node_of_the_input_pair() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let classes = signal_classes(nl.nets.len(), &[3]);
        for (ports, star) in [(vec![], false), (vec![NetId(2)], true)] {
            let mut intent = Intent { sets: vec![spec(&[1, 0], MatchKind::Current)], ..Default::default() };
            common_nodes(&hg, &classes, &mut intent, &[], &ports, None);
            let c: Vec<(NetId, u16, Vec<DeviceId>, Vec<DeviceId>, Term)> = intent.common_nodes.iter().map(|c| (c.net, c.set, c.a.clone(), c.b.clone(), c.term)).collect();
            assert_eq!(c, [(NetId(2), 0, vec![DeviceId(0)], vec![DeviceId(1)], Term::S)]);
            assert_eq!(intent.stars.len(), usize::from(star), "ports {ports:?}");
            if star {
                assert!(intent.stars[0].root.is_none());
            }
        }
    }

    /// A reference member makes one common node per other member.
    #[test]
    fn common_nodes_from_a_reference() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut s = spec(&[0, 1], MatchKind::Current);
        s.reference = Some(1);
        let mut intent = Intent { sets: vec![s], ..Default::default() };
        common_nodes(&hg, &signal_classes(nl.nets.len(), &[3]), &mut intent, &[], &[], None);
        let c: Vec<(Vec<DeviceId>, Vec<DeviceId>)> = intent.common_nodes.iter().map(|c| (c.a.clone(), c.b.clone())).collect();
        assert_eq!(c, [(vec![DeviceId(1)], vec![DeviceId(0)])]);
    }

    /// Members on different sources, or a lone member, share no node.
    #[test]
    fn no_common_node_without_a_shared_source() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut intent = Intent { sets: vec![spec(&[0, 4], MatchKind::Current), spec(&[0], MatchKind::Current)], ..Default::default() };
        common_nodes(&hg, &signal_classes(nl.nets.len(), &[3]), &mut intent, &[], &[], None);
        assert!(intent.common_nodes.is_empty() && intent.stars.is_empty());
    }

    /// An empty set (no members) is skipped, never indexed.
    #[test]
    fn empty_sets_are_skipped() {
        let nl = crate::tests::ota();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut intent = Intent { sets: vec![spec(&[], MatchKind::Voltage)], ..Default::default() };
        common_nodes(&hg, &signal_classes(nl.nets.len(), &[3]), &mut intent, &[crate::class::SetRole::DacBank], &[], None);
        assert!(intent.common_nodes.is_empty() && intent.kelvins.is_empty());
    }

    /// A resistor between the two gates of a Voltage set is sensed at both ends.
    #[test]
    fn kelvin_across_a_voltage_pair() {
        let mut nl: Netlist = crate::tests::ota();
        nl.devices.push(Device {
            name: "R".into(),
            kind: DeviceKind::Resistor,
            model: "r".into(),
            terminals: vec![("P".into(), NetId(1)), ("N".into(), NetId(5))],
            params: vec![],
        });
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut intent = Intent { sets: vec![spec(&[0, 1], MatchKind::Voltage)], ..Default::default() };
        kelvins(&hg, &mut intent);
        let k: Vec<(DeviceId, Term, Vec<(DeviceId, Term)>)> = intent.kelvins.iter().map(|k| (k.device, k.term, k.sense.clone())).collect();
        assert_eq!(k, [(DeviceId(5), Term::P, vec![(DeviceId(0), Term::G)]), (DeviceId(5), Term::N, vec![(DeviceId(1), Term::G)])]);
        // A Current set is not sensed.
        let mut intent = Intent { sets: vec![spec(&[0, 1], MatchKind::Current)], ..Default::default() };
        kelvins(&hg, &mut intent);
        assert!(intent.kelvins.is_empty());
    }
}
