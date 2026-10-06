//! Passive, bipolar and diode matched sets (EXT-19): procedural rules, since a
//! two-terminal device has no fixed orientation for the pattern DSL (Hastings
//! H08-61 matched passives from topology, H08-56 one material, H09-05 bandgap
//! cores, H09-26 emitter degeneration; DACP eq. 1 split DAC).

use analog::intent::{Compound, Diagnostic};
use analog::metadata::NetClassification;
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::class::SetRole;
use crate::sets::{bjt, fet, find, net, union};
use crate::size::Drawn;

/// One procedural set, found by a rule of this module.
#[derive(Clone, Debug, PartialEq)]
pub struct PassiveSet {
    /// Members, the reference first where there is one; at least two.
    pub devices: Vec<DeviceId>,
    /// The rule that found the set (`Origin::PassiveSet { rule }`, the
    /// identifier consumers match on), e.g. `"divider"`, `"split_dac"`.
    pub rule: &'static str,
    /// The set's role for `class::class_of`.
    pub role: SetRole,
    /// A bank's terminating unit (`MatchSpec::reference`), one of `devices`.
    pub reference: Option<DeviceId>,
    /// A split DAC's bridge capacitor, one of `devices`; it is left out of the unit.
    pub bridge: Option<DeviceId>,
}

/// `n` is a Supply, Ground or Substrate net.
fn rail(classes: &[NetClassification], n: NetId) -> bool {
    crate::extract::rail(classes[n.0 as usize].class)
}

fn set(devices: Vec<usize>, rule: &'static str, role: SetRole) -> PassiveSet {
    PassiveSet { devices: devices.into_iter().map(|d| DeviceId(d as u16)).collect(), rule, role, reference: None, bridge: None }
}

/// Same model and width as the first of `g` (H08-56: another material is not matched).
fn one_material(drawn: &[Drawn], g: &[usize]) -> bool {
    g.iter().all(|&d| (drawn[d].model, drawn[d].w_finger_nm) == (drawn[g[0]].model, drawn[g[0]].w_finger_nm))
}

/// The one resistor on net `s` and its far end: `None` when `s` carries no
/// resistor or more than one (a resistor with both ends on `s` counts twice).
fn lone_resistor(hg: &BipartiteHypergraph, s: NetId) -> Option<(usize, NetId)> {
    let mut rs = hg.net_devices[s.0 as usize].iter().map(|x| x.0 as usize).filter(|&x| hg.kinds[x] == DeviceKind::Resistor);
    let (Some(r), None) = (rs.next(), rs.next()) else { return None };
    hg.device_nets[r].iter().copied().find(|&n| n != s).map(|far| (r, far))
}

/// Resistor sets of one model and width: (a) dividers and ladders, the
/// resistors joined through internal nodes that reach only resistors and FET
/// gates (no rail), role FeedbackRatio, rule `"divider"`; (b) each compound
/// pair of resistors, role Other, rule `"symmetric_r"`. Dividers come first,
/// by lowest member id, members ascending.
#[must_use]
pub fn resistor_sets(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], compounds: &[Compound]) -> Vec<PassiveSet> {
    let res = |d: usize| hg.kinds[d] == DeviceKind::Resistor;
    let mut parent: Vec<usize> = (0..hg.device_count()).collect();
    for (n, devs) in hg.net_devices.iter().enumerate() {
        let n = NetId(n as u16);
        if rail(classes, n) {
            continue;
        }
        let gate_only = |i: usize| fet(hg.kinds[i]) && net(hg, i, "G") == Some(n) && net(hg, i, "D") != Some(n) && net(hg, i, "S") != Some(n);
        if !devs.iter().all(|d| res(d.0 as usize) || gate_only(d.0 as usize)) {
            continue;
        }
        let rs: Vec<usize> = devs.iter().map(|d| d.0 as usize).filter(|&d| res(d)).collect();
        for w in rs.windows(2) {
            union(&mut parent, w[0], w[1]);
        }
    }
    let mut comps: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for d in (0..hg.device_count()).filter(|&d| res(d)) {
        let r = find(&mut parent, d);
        comps.entry(r).or_default().push(d);
    }
    let mut out: Vec<PassiveSet> = comps.into_values().filter(|g| g.len() > 1 && one_material(drawn, g)).map(|g| set(g, "divider", SetRole::FeedbackRatio)).collect();
    for c in compounds {
        for &(a, b) in &c.pairs {
            let g = vec![a.0 as usize, b.0 as usize];
            if res(g[0]) && res(g[1]) && one_material(drawn, &g) {
                out.push(set(g, "symmetric_r", SetRole::Other));
            }
        }
    }
    out
}

/// The resistors on a BJT ratioed pair's emitter nets (Brokaw R1/R2), one set
/// per pair when there are two or more and they share model and width: role
/// BandgapCore, rule `"bandgap_r"`, members ascending.
#[must_use]
pub fn bandgap_cores(hg: &BipartiteHypergraph, drawn: &[Drawn], bjt_pairs: &[(DeviceId, DeviceId)]) -> Vec<PassiveSet> {
    bjt_pairs
        .iter()
        .filter_map(|&(a, b)| {
            let em = [net(hg, a.0 as usize, "E"), net(hg, b.0 as usize, "E")];
            let mut rs: Vec<usize> = em.iter().flatten().flat_map(|n| hg.net_devices[n.0 as usize].iter().map(|d| d.0 as usize)).filter(|&d| hg.kinds[d] == DeviceKind::Resistor).collect();
            rs.sort_unstable();
            rs.dedup();
            (rs.len() > 1 && one_material(drawn, &rs)).then(|| set(rs, "bandgap_r", SetRole::BandgapCore))
        })
        .collect()
}

/// `true` when `counts`, sorted, are `[1, 1, 2, …, 2^(N-1)]`, N ≥ 2: a
/// terminated binary-weighted bank (card D-h: `cells::cap_array::bits` is out
/// of the annotator's reach).
fn binary(counts: &[u32]) -> bool {
    let mut c = counts.to_vec();
    c.sort_unstable();
    c.len() >= 3 && c[0] == 1 && c[1..].iter().enumerate().all(|(i, &u)| u == 1 << i)
}

/// Bridge mismatch tolerance, relative to the drawn bridge area (**Philis
/// tolerance**).
const BRIDGE_TOLERANCE: f64 = 0.01;

/// Capacitors sharing a top plate `P` with one W, L and model: one set each,
/// the one-unit cap whose other plate is a rail first (the bank's termination,
/// cellgen's old `dac_banks` rule), role DacBank (rule `"dac_bank"`) when the
/// counts are binary, else FeedbackRatio (`"cap_ratio"`). Two banks joined by
/// exactly one other capacitor are one `"split_dac"` set holding that bridge;
/// `bridge_cap_value` when its area is off `(C_T^LSB / C_T^MSB)·C_u` by more
/// than 1 % of the bridge area, the LSB bank being the terminated one. Split
/// DACs come first, then the remaining banks in order of their lowest id.
#[must_use]
pub fn capacitor_sets(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], diags: &mut Vec<Diagnostic>) -> Vec<PassiveSet> {
    let cap = |d: usize| hg.kinds[d] == DeviceKind::Capacitor;
    // (top plate, model, W, L) → members.
    type Key = (NetId, u16, Option<i64>, Option<i64>);
    let mut banks: Vec<(Key, Vec<usize>)> = crate::sets::group(
        (0..hg.device_count()).filter(|&d| cap(d)).filter_map(|d| Some(((net(hg, d, "P")?, drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm), d))),
    );
    banks.retain(|(_, v)| v.len() > 1);
    let terminated = |d: usize| drawn[d].fingers == 1 && net(hg, d, "N").is_some_and(|n| rail(classes, n));
    for (_, v) in &mut banks {
        v.sort_by_key(|&d| (!terminated(d), d));
    }
    let reference = |v: &[usize]| v.first().filter(|&&d| terminated(d)).map(|&d| DeviceId(d as u16));
    let mut out = Vec::new();
    let mut used = vec![false; banks.len()];
    for i in 0..banks.len() {
        for j in i + 1..banks.len() {
            if used[i] || used[j] {
                continue;
            }
            let Some(ca) = lone_bridge(hg, banks[i].0 .0, banks[j].0 .0, &banks) else { continue };
            (used[i], used[j]) = (true, true);
            let (lsb, msb) = if banks[i].1.iter().any(|&d| terminated(d)) { (&banks[i].1, &banks[j].1) } else { (&banks[j].1, &banks[i].1) };
            check_bridge(drawn, lsb, msb, ca, diags);
            let devices: Vec<usize> = lsb.iter().chain(msb).copied().chain([ca]).collect();
            out.push(PassiveSet { bridge: Some(DeviceId(ca as u16)), reference: reference(lsb), ..set(devices, "split_dac", SetRole::DacBank) });
        }
    }
    for (_, v) in banks.iter().zip(&used).filter(|(_, &u)| !u).map(|(b, _)| b) {
        let counts: Vec<u32> = v.iter().map(|&d| drawn[d].fingers).collect();
        let (rule, role) = if binary(&counts) { ("dac_bank", SetRole::DacBank) } else { ("cap_ratio", SetRole::FeedbackRatio) };
        out.push(PassiveSet { reference: reference(v), ..set(v.clone(), rule, role) });
    }
    out
}

/// The one capacitor in no bank with its plates on `ti` and `tj` (either way
/// round), `None` when there is none or several.
fn lone_bridge<K>(hg: &BipartiteHypergraph, ti: NetId, tj: NetId, banks: &[(K, Vec<usize>)]) -> Option<usize> {
    let mut bridges = (0..hg.device_count()).filter(|&d| {
        let (p, n) = (net(hg, d, "P"), net(hg, d, "N"));
        hg.kinds[d] == DeviceKind::Capacitor
            && ((p, n) == (Some(ti), Some(tj)) || (p, n) == (Some(tj), Some(ti)))
            && !banks.iter().any(|(_, v)| v.contains(&d))
    });
    match (bridges.next(), bridges.next()) {
        (Some(ca), None) => Some(ca),
        _ => None,
    }
}

/// Pushes `bridge_cap_value` when bridge `ca`'s area is off
/// `(C_T^LSB / C_T^MSB)·C_u` by more than [`BRIDGE_TOLERANCE`] of its own area.
fn check_bridge(drawn: &[Drawn], lsb: &[usize], msb: &[usize], ca: usize, diags: &mut Vec<Diagnostic>) {
    let units = |v: &[usize]| v.iter().map(|&d| f64::from(drawn[d].fingers)).sum::<f64>();
    let area = |d: usize| (drawn[d].w_finger_nm.unwrap_or(0) * drawn[d].l_nm.unwrap_or(0)) as f64;
    let want = units(lsb) / units(msb) * area(lsb[0]);
    let got = area(ca) * f64::from(drawn[ca].fingers);
    if (got - want).abs() > BRIDGE_TOLERANCE * got {
        diags.push(Diagnostic {
            kind: "bridge_cap_value",
            devices: vec![DeviceId(ca as u16)],
            message: format!("split-DAC bridge area {got} nm², want (C_T^LSB/C_T^MSB)·C_u = {want} nm²"),
        });
    }
}

/// Per pair (FET sources or BJT emitters), the two resistors through which the
/// halves reach one common node, one each: a Ratio set whose ratio is the
/// inverse of the pair's (H09-26: 1X 4 kΩ / 2X 2 kΩ), rule `"degeneration"`,
/// role InputPair, `[resistor of a, resistor of b]`. `"degeneration_ratio"`
/// when the drawn resistor lengths are not that inverse.
#[must_use]
pub fn degeneration(hg: &BipartiteHypergraph, drawn: &[Drawn], pairs: &[(DeviceId, DeviceId)], diags: &mut Vec<Diagnostic>) -> Vec<PassiveSet> {
    let leg = |d: DeviceId| {
        let i = d.0 as usize;
        lone_resistor(hg, net(hg, i, if bjt(hg.kinds[i]) { "E" } else { "S" })?)
    };
    let mut out = Vec::new();
    for &(a, b) in pairs {
        let (Some((ra, ta)), Some((rb, tb))) = (leg(a), leg(b)) else { continue };
        if ta != tb || ra == rb || !one_material(drawn, &[ra, rb]) {
            continue;
        }
        let (ua, ub) = (i64::from(drawn[a.0 as usize].fingers), i64::from(drawn[b.0 as usize].fingers));
        let (la, lb) = (drawn[ra].l_nm.unwrap_or(0), drawn[rb].l_nm.unwrap_or(0));
        if la * ua != lb * ub {
            diags.push(Diagnostic {
                kind: "degeneration_ratio",
                devices: vec![DeviceId(ra as u16), DeviceId(rb as u16)],
                message: format!("degeneration lengths {la}:{lb} nm are not the inverse of the pair's units {ua}:{ub}"),
            });
        }
        out.push(set(vec![ra, rb], "degeneration", SetRole::InputPair));
    }
    out
}

/// FET couples of one signature (kind, model, finger W, L, fingers; known L)
/// with distinct gates whose sources are distinct non-rail nets each reaching
/// one common node through exactly one resistor: the degenerated diff pair no
/// joined-source pattern sees (M1 regression note). Lower id first, sorted.
#[must_use]
pub fn degenerated_pairs(hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[crate::NetRole]) -> Vec<(DeviceId, DeviceId)> {
    let tail = |d: usize| {
        let s = net(hg, d, "S")?;
        if matches!(roles[s.0 as usize], crate::NetRole::Supply | crate::NetRole::Ground) {
            return None;
        }
        lone_resistor(hg, s).map(|(_, far)| far)
    };
    let sig = |d: usize| (hg.kinds[d] as u8, drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm, drawn[d].fingers);
    let keyed = (0..hg.device_count()).filter(|&d| fet(hg.kinds[d]) && drawn[d].l_nm.is_some()).filter_map(|d| Some(((sig(d), tail(d)?), d)));
    let mut out = Vec::new();
    for (_, g) in crate::sets::group(keyed) {
        for (i, &a) in g.iter().enumerate() {
            for &b in &g[i + 1..] {
                if net(hg, a, "G") != net(hg, b, "G") && net(hg, a, "S") != net(hg, b, "S") {
                    out.push((DeviceId(a as u16), DeviceId(b as u16)));
                }
            }
        }
    }
    out.sort_by_key(|&(a, b)| (a.0, b.0));
    out
}

/// Diodes of one model and area on a shared anode (`P`) or cathode (`N`) net
/// (H09-43): rule `"diode_set"`, role Other, members ascending. One set per
/// shared net; a group already found on the other terminal is not repeated,
/// and a diode without that terminal joins nothing on it.
#[must_use]
pub fn diode_sets(hg: &BipartiteHypergraph, drawn: &[Drawn]) -> Vec<PassiveSet> {
    let keyed = (0..hg.device_count())
        .filter(|&d| hg.kinds[d] == DeviceKind::Diode)
        .flat_map(|d| ["P", "N"].map(|t| ((t, net(hg, d, t), drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm), d)));
    crate::sets::group(keyed).into_iter().filter(|(_, v)| v.len() > 1).map(|(_, v)| set(v, "diode_set", SetRole::Other)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::nets;
    use pnr_core::netlist::{Device, Netlist};

    fn two(name: &str, kind: DeviceKind, model: &str, p: u16, n: u16, params: &[(&str, i64)]) -> Device {
        Device {
            name: name.into(),
            kind,
            model: model.into(),
            terminals: vec![("P".into(), NetId(p)), ("N".into(), NetId(n))],
            params: params.iter().map(|&(k, v)| (k.into(), v)).collect(),
        }
    }

    fn bjt(name: &str, c: u16, b: u16, e: u16, m: i64) -> Device {
        Device {
            name: name.into(),
            kind: DeviceKind::Npn,
            model: "npn".into(),
            terminals: vec![("C".into(), NetId(c)), ("B".into(), NetId(b)), ("E".into(), NetId(e))],
            params: vec![("m".into(), m)],
        }
    }

    #[test]
    fn divider_chain_is_one_set() {
        // rdiv: RA top-mid 10 µm, RB mid-VSS 40 µm.
        let nl = Netlist {
            devices: vec![
                two("RA", DeviceKind::Resistor, "rpoly", 0, 1, &[("w", 2_000), ("l", 10_000)]),
                two("RB", DeviceKind::Resistor, "rpoly", 1, 2, &[("w", 2_000), ("l", 40_000)]),
            ],
            nets: nets(&["top", "mid", "VSS"]),
            ..Default::default()
        };
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect();
        let sets = resistor_sets(&hg, &drawn, &p.net_classes, &[]);
        assert_eq!(sets, [set(vec![0, 1], "divider", SetRole::FeedbackRatio)]);
    }

    /// Hastings 10.2.2: a 1X/2X pair degenerated for equal drops has resistors
    /// 2:1, so 20 µm against 10 µm is the inverse; equal lengths are flagged.
    #[test]
    fn degeneration_inverse_ratio() {
        // Nets: 0=c1 1=vb 2=e1 3=c2 4=e2 5=t.
        let nl = |lb: i64| Netlist {
            devices: vec![
                bjt("Q1", 0, 1, 2, 1),
                bjt("Q2", 3, 1, 4, 2),
                two("RA", DeviceKind::Resistor, "rpoly", 2, 5, &[("w", 2_000), ("l", 20_000)]),
                two("RB", DeviceKind::Resistor, "rpoly", 4, 5, &[("w", 2_000), ("l", lb)]),
            ],
            nets: nets(&["c1", "vb", "e1", "c2", "e2", "t"]),
            ..Default::default()
        };
        let run = |lb: i64| {
            let nl = nl(lb);
            let hg = BipartiteHypergraph::from_netlist(&nl);
            let mut models = Vec::new();
            let drawn: Vec<_> = nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect();
            let mut diags = Vec::new();
            let s = degeneration(&hg, &drawn, &[(DeviceId(0), DeviceId(1))], &mut diags);
            (s, diags.iter().map(|d| d.kind).collect::<Vec<_>>())
        };
        assert_eq!(run(10_000), (vec![set(vec![2, 3], "degeneration", SetRole::InputPair)], vec![]));
        assert_eq!(run(20_000).1, ["degeneration_ratio"]);
        // Through annotate: the ratioed pair is found, and the set's units are 2:1 series.
        let mut c = crate::AnnotationConfig::default();
        c.process.unit = crate::sets::UnitDeck { grid_nm: 5, min_w_nm: 420, max_w_nm: 10_000, min_l_nm: 150, res_min_segment_nm: 10_000 };
        let p = crate::annotate(&nl(10_000), &c);
        let r = p.intent.sets.iter().find(|s| s.members.iter().any(|m| m.device == DeviceId(2))).expect("degeneration set");
        let mut series: Vec<(u16, u16)> = r.members.iter().map(|m| (m.device.0, m.series)).collect();
        series.sort_unstable();
        assert_eq!(series, [(2, 2), (3, 1)]);
    }

    // ---- cleanup(annotator-sets) step 2 ----

    fn drawn_of(nl: &Netlist) -> Vec<Drawn> {
        let mut models = Vec::new();
        nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect()
    }

    /// Every net Signal except `rails`, which are Ground.
    fn classes(nl: &Netlist, rails: &[u16]) -> Vec<NetClassification> {
        use analog::metadata::NetClass;
        (0..nl.nets.len() as u16)
            .map(|n| NetClassification { net: NetId(n), class: if rails.contains(&n) { NetClass::Ground } else { NetClass::Signal }, c_budget_af: None, max_coupling_af: None })
            .collect()
    }

    fn cap(name: &str, p: u16, n: u16, w: i64, l: i64, m: i64) -> Device {
        two(name, DeviceKind::Capacitor, "mim", p, n, &[("w", w), ("l", l), ("m", m)])
    }

    fn res(name: &str, p: u16, n: u16, w: i64, l: i64) -> Device {
        two(name, DeviceKind::Resistor, "rpoly", p, n, &[("w", w), ("l", l)])
    }

    #[test]
    fn binary_banks() {
        assert!(binary(&[1, 1, 2, 4]));
        assert!(binary(&[4, 1, 2, 1]), "order does not matter");
        assert!(binary(&[1, 1, 2]));
        assert!(!binary(&[1, 2]), "N >= 2");
        assert!(!binary(&[]));
        assert!(!binary(&[1, 1, 3]));
        assert!(!binary(&[2, 2, 4]));
        assert!(!binary(&[1, 1, 1, 2]));
    }

    /// A 33-bit-and-more bank is no binary bank of `u32` counts, and asking
    /// never overflows the shift.
    #[test]
    fn binary_wide_bank_does_not_overflow() {
        let mut c: Vec<u32> = vec![1];
        c.extend((0..32).map(|i| 1u32 << i));
        c.push(u32::MAX);
        assert!(!binary(&c));
        c.pop();
        assert!(binary(&c), "the full 32-bit bank");
    }

    #[test]
    fn lone_resistor_cases() {
        // Nets: 0=s 1=far 2=t. RA s-far; RB t-t (both ends on t); RC, RD on 2 too.
        let nl = Netlist { devices: vec![res("RA", 0, 1, 1_000, 1_000), res("RB", 2, 2, 1_000, 1_000)], nets: nets(&["s", "far", "t"]), ..Default::default() };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        assert_eq!(lone_resistor(&hg, NetId(0)), Some((0, NetId(1))));
        assert_eq!(lone_resistor(&hg, NetId(2)), None, "a resistor shorted on the net");
        let nl = Netlist { devices: vec![res("RA", 0, 1, 1_000, 1_000), res("RB", 0, 2, 1_000, 1_000)], nets: nets(&["s", "a", "b"]), ..Default::default() };
        assert_eq!(lone_resistor(&BipartiteHypergraph::from_netlist(&nl), NetId(0)), None, "two resistors");
        assert_eq!(lone_resistor(&BipartiteHypergraph::from_netlist(&nl), NetId(1)), Some((0, NetId(0))));
    }

    #[test]
    fn resistors_joined_only_through_a_rail_or_a_drain_are_no_divider() {
        // Nets: 0=a 1=VSS 2=b 3=m 4=g.
        let nl = Netlist {
            devices: vec![res("RA", 0, 1, 2_000, 10_000), res("RB", 1, 2, 2_000, 10_000), res("RC", 2, 3, 2_000, 10_000), crate::tests::fet("M", DeviceKind::Nmos, 4, 3, 1, 1, 1_000, 500), res("RD", 3, 0, 2_000, 10_000)],
            nets: nets(&["a", "VSS", "b", "m", "g"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let sets = resistor_sets(&hg, &drawn_of(&nl), &classes(&nl, &[1]), &[]);
        // RA-RD through `a`, RB-RC through `b`; `m` reaches a drain, VSS is a rail.
        assert_eq!(sets, [set(vec![0, 4], "divider", SetRole::FeedbackRatio), set(vec![1, 2], "divider", SetRole::FeedbackRatio)]);
    }

    #[test]
    fn divider_through_a_gate_and_material() {
        // Nets: 0=top 1=mid 2=VSS 3=d. The mid node also drives a gate.
        let base = |w_b: i64| Netlist {
            devices: vec![res("RA", 0, 1, 2_000, 10_000), res("RB", 1, 2, w_b, 10_000), crate::tests::fet("M", DeviceKind::Nmos, 1, 3, 2, 2, 1_000, 500)],
            nets: nets(&["top", "mid", "VSS", "d"]),
            ..Default::default()
        };
        let run = |nl: &Netlist| resistor_sets(&BipartiteHypergraph::from_netlist(nl), &drawn_of(nl), &classes(nl, &[2]), &[]);
        assert_eq!(run(&base(2_000)), [set(vec![0, 1], "divider", SetRole::FeedbackRatio)]);
        assert!(run(&base(3_000)).is_empty(), "two widths are two materials");
    }

    #[test]
    fn symmetric_resistor_pairs() {
        let nl = Netlist { devices: vec![res("RA", 0, 1, 2_000, 10_000), res("RB", 2, 3, 2_000, 10_000)], nets: nets(&["a", "VSS", "b", "VDD"]), ..Default::default() };
        let c = Compound {
            id: analog::intent::ConstraintId(0),
            axis: pnr_core::ids::AxisId(0),
            dir: analog::intent::AxisDir::V,
            kind: analog::intent::SymKind::Mirror,
            pairs: vec![(DeviceId(0), DeviceId(1))],
            selfs: vec![],
            net_pairs: vec![],
            self_nets: vec![],
            set_pairs: vec![],
        };
        let sets = resistor_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl), &classes(&nl, &[1, 3]), &[c]);
        assert_eq!(sets, [set(vec![0, 1], "symmetric_r", SetRole::Other)]);
    }

    #[test]
    fn bandgap_core_resistors() {
        // Nets: 0=c1 1=b 2=e1 3=c2 4=e2 5=VSS.
        let nl = |w2: i64| Netlist {
            devices: vec![bjt("Q1", 0, 1, 2, 1), bjt("Q2", 3, 1, 4, 8), res("R1", 2, 5, 2_000, 10_000), res("R2", 4, 2, w2, 20_000)],
            nets: nets(&["c1", "b", "e1", "c2", "e2", "VSS"]),
            ..Default::default()
        };
        let run = |nl: &Netlist| bandgap_cores(&BipartiteHypergraph::from_netlist(nl), &drawn_of(nl), &[(DeviceId(0), DeviceId(1))]);
        assert_eq!(run(&nl(2_000)), [set(vec![2, 3], "bandgap_r", SetRole::BandgapCore)]);
        assert!(run(&nl(3_000)).is_empty(), "one material");
        assert!(bandgap_cores(&BipartiteHypergraph::from_netlist(&nl(2_000)), &drawn_of(&nl(2_000)), &[]).is_empty());
    }

    /// Nets: 0=T 1=VSS 2=b0 3=b1 4=b2.
    fn bank(counts_m: &[i64]) -> Netlist {
        let mut devices = vec![cap("CT", 0, 1, 10_000, 10_000, 1)];
        devices.extend(counts_m.iter().enumerate().map(|(i, &m)| cap(&format!("C{i}"), 0, 2 + (i as u16 % 3), 10_000, 10_000, m)));
        Netlist { devices, nets: nets(&["T", "VSS", "b0", "b1", "b2"]), ..Default::default() }
    }

    #[test]
    fn binary_bank_is_a_dac_bank_with_its_terminator_first() {
        let nl = bank(&[4, 1, 2]);
        let mut diags = Vec::new();
        let sets = capacitor_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl), &classes(&nl, &[1]), &mut diags);
        assert!(diags.is_empty());
        assert_eq!(sets, [PassiveSet { reference: Some(DeviceId(0)), ..set(vec![0, 1, 2, 3], "dac_bank", SetRole::DacBank) }]);
    }

    #[test]
    fn ratio_bank_and_unterminated_bank() {
        let nl = bank(&[3, 5]);
        let mut diags = Vec::new();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let sets = capacitor_sets(&hg, &drawn_of(&nl), &classes(&nl, &[1]), &mut diags);
        assert_eq!(sets, [PassiveSet { reference: Some(DeviceId(0)), ..set(vec![0, 1, 2], "cap_ratio", SetRole::FeedbackRatio) }]);
        // No rail: no terminator, members by id.
        let sets = capacitor_sets(&hg, &drawn_of(&nl), &classes(&nl, &[]), &mut diags);
        assert_eq!(sets, [set(vec![0, 1, 2], "cap_ratio", SetRole::FeedbackRatio)]);
        // A lone capacitor is no bank.
        let one = Netlist { devices: vec![cap("C", 0, 1, 1_000, 1_000, 1)], nets: nets(&["T", "VSS"]), ..Default::default() };
        assert!(capacitor_sets(&BipartiteHypergraph::from_netlist(&one), &drawn_of(&one), &classes(&one, &[1]), &mut diags).is_empty());
    }

    /// DACP eq. 1: LSB bank {CT, C1, C2} (4 units, terminated) and MSB bank
    /// {C3, C4} (4 units) joined by CB: C_B = (4 / 4)·C_u = 1e8 nm².
    fn split_dac(bridge_l: i64, bridge_m: i64) -> Netlist {
        // Nets: 0=T1 1=VSS 2=b0 3=b1 4=T2 5=b2 6=b3.
        Netlist {
            devices: vec![
                cap("CT", 0, 1, 10_000, 10_000, 1),
                cap("C1", 0, 2, 10_000, 10_000, 1),
                cap("C2", 0, 3, 10_000, 10_000, 2),
                cap("C3", 4, 5, 10_000, 10_000, 2),
                cap("C4", 4, 6, 10_000, 10_000, 2),
                cap("CB", 0, 4, 10_000, bridge_l, bridge_m),
            ],
            nets: nets(&["T1", "VSS", "b0", "b1", "T2", "b2", "b3"]),
            ..Default::default()
        }
    }

    #[test]
    fn split_dac_holds_its_bridge() {
        for (l, m, flagged) in [(5_000, 2, false), (20_000, 1, true)] {
            let nl = split_dac(l, m);
            let mut diags = Vec::new();
            let sets = capacitor_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl), &classes(&nl, &[1]), &mut diags);
            assert_eq!(
                sets,
                [PassiveSet { bridge: Some(DeviceId(5)), reference: Some(DeviceId(0)), ..set(vec![0, 1, 2, 3, 4, 5], "split_dac", SetRole::DacBank) }],
                "bridge {l}x{m}"
            );
            assert_eq!(diags.iter().map(|d| d.kind).collect::<Vec<_>>(), if flagged { vec!["bridge_cap_value"] } else { vec![] });
        }
    }

    #[test]
    fn two_bridges_make_no_split_dac() {
        let mut nl = split_dac(5_000, 2);
        nl.devices.push(cap("CB2", 4, 0, 3_000, 3_000, 1));
        let mut diags = Vec::new();
        let sets = capacitor_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl), &classes(&nl, &[1]), &mut diags);
        assert_eq!(sets.iter().map(|s| s.rule).collect::<Vec<_>>(), ["dac_bank", "cap_ratio"]);
    }

    /// Degeneration needs one resistor per leg to one common node.
    #[test]
    fn degeneration_skips_open_legs() {
        // Nets: 0=c1 1=vb 2=e1 3=c2 4=e2 5=t 6=u.
        let nl = Netlist {
            devices: vec![bjt("Q1", 0, 1, 2, 1), bjt("Q2", 3, 1, 4, 1), res("RA", 2, 5, 2_000, 10_000), res("RB", 4, 6, 2_000, 10_000)],
            nets: nets(&["c1", "vb", "e1", "c2", "e2", "t", "u"]),
            ..Default::default()
        };
        let mut diags = Vec::new();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        assert!(degeneration(&hg, &drawn_of(&nl), &[(DeviceId(0), DeviceId(1))], &mut diags).is_empty(), "two far nodes");
        assert!(degeneration(&hg, &drawn_of(&nl), &[], &mut diags).is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn degenerated_pair_found() {
        let n = DeviceKind::Nmos;
        // Nets: 0=inp 1=a 2=s1 3=inn 4=b 5=s2 6=t 7=VSS.
        let nl = |g2: u16| Netlist {
            devices: vec![
                crate::tests::fet("M1", n, 0, 1, 2, 7, 2_000, 500),
                crate::tests::fet("M2", n, g2, 4, 5, 7, 2_000, 500),
                res("RA", 2, 6, 2_000, 10_000),
                res("RB", 5, 6, 2_000, 10_000),
            ],
            nets: nets(&["inp", "a", "s1", "inn", "b", "s2", "t", "VSS"]),
            ..Default::default()
        };
        let roles = vec![crate::NetRole::Signal; 8];
        let run = |nl: &Netlist| degenerated_pairs(&BipartiteHypergraph::from_netlist(nl), &drawn_of(nl), &roles);
        assert_eq!(run(&nl(3)), [(DeviceId(0), DeviceId(1))]);
        assert!(run(&nl(0)).is_empty(), "one gate");
        let mut ground = roles.clone();
        ground[2] = crate::NetRole::Ground;
        assert!(degenerated_pairs(&BipartiteHypergraph::from_netlist(&nl(3)), &drawn_of(&nl(3)), &ground).is_empty(), "a source on a rail");
    }

    #[test]
    fn diode_sets_per_shared_terminal() {
        let d = |name: &str, p: u16, n: u16| two(name, DeviceKind::Diode, "dio", p, n, &[("w", 1_000), ("l", 1_000)]);
        // Nets: 0=a 1=k1 2=k2. D0, D1 share the anode only.
        let nl = Netlist { devices: vec![d("D0", 0, 1), d("D1", 0, 2)], nets: nets(&["a", "k1", "k2"]), ..Default::default() };
        assert_eq!(diode_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl)), [set(vec![0, 1], "diode_set", SetRole::Other)]);
        // Different area: no set.
        let mut odd = nl.clone();
        odd.devices[1].params[0].1 = 2_000;
        assert!(diode_sets(&BipartiteHypergraph::from_netlist(&odd), &drawn_of(&odd)).is_empty());
    }

    /// Diodes in parallel share both terminals: one set, not the same set twice.
    #[test]
    fn parallel_diodes_are_one_set() {
        let d = |name: &str| two(name, DeviceKind::Diode, "dio", 0, 1, &[("w", 1_000), ("l", 1_000)]);
        let nl = Netlist { devices: vec![d("D0"), d("D1")], nets: nets(&["a", "k"]), ..Default::default() };
        assert_eq!(diode_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl)), [set(vec![0, 1], "diode_set", SetRole::Other)]);
    }

    /// A diode written without `P`/`N` pins shares no net with anything.
    #[test]
    fn diodes_without_pins_join_nothing() {
        let d = |name: &str, a: u16, k: u16| Device {
            name: name.into(),
            kind: DeviceKind::Diode,
            model: "dio".into(),
            terminals: vec![("A".into(), NetId(a)), ("K".into(), NetId(k))],
            params: vec![],
        };
        let nl = Netlist { devices: vec![d("D0", 0, 1), d("D1", 2, 3)], nets: nets(&["a0", "k0", "a1", "k1"]), ..Default::default() };
        assert!(diode_sets(&BipartiteHypergraph::from_netlist(&nl), &drawn_of(&nl)).is_empty());
    }
}
