//! Passive, bipolar and diode matched sets (EXT-19): procedural rules, since a
//! two-terminal device has no fixed orientation for the pattern DSL (Hastings
//! H08-61 matched passives from topology, H08-56 one material, H09-05 bandgap
//! cores, H09-26 emitter degeneration; DACP eq. 1 split DAC).

use analog::intent::{Compound, Diagnostic};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::class::SetRole;
use crate::size::Drawn;

/// One procedural set: its devices (reference first where there is one), the
/// rule that found it (`Origin::PassiveSet { rule }`, the identifier consumers
/// match on), its role for `class::class_of`, a bank's terminating unit
/// (`MatchSpec::reference`) and a split DAC's bridge.
#[derive(Clone, Debug, PartialEq)]
pub struct PassiveSet {
    pub devices: Vec<DeviceId>,
    pub rule: &'static str,
    pub role: SetRole,
    pub reference: Option<DeviceId>,
    pub bridge: Option<DeviceId>,
}

fn net(hg: &BipartiteHypergraph, d: usize, t: &str) -> Option<NetId> {
    hg.terminals[d].iter().position(|p| p == t).map(|i| hg.device_nets[d][i])
}

fn rail(classes: &[NetClassification], n: NetId) -> bool {
    matches!(classes[n.0 as usize].class, NetClass::Supply | NetClass::Ground | NetClass::Substrate)
}

fn set(devices: Vec<usize>, rule: &'static str, role: SetRole) -> PassiveSet {
    PassiveSet { devices: devices.into_iter().map(|d| DeviceId(d as u16)).collect(), rule, role, reference: None, bridge: None }
}

/// Same model and width (H08-56: another material is not matched).
fn one_material(drawn: &[Drawn], g: &[usize]) -> bool {
    g.iter().all(|&d| (drawn[d].model, drawn[d].w_finger_nm) == (drawn[g[0]].model, drawn[g[0]].w_finger_nm))
}

/// Resistor sets of one model and width: (a) dividers and ladders, the
/// resistors joined through internal nodes that reach only resistors and FET
/// gates (no rail), role FeedbackRatio, rule `"divider"`; (b) each compound
/// pair of resistors, role Other, rule `"symmetric_r"`.
#[must_use]
pub fn resistor_sets(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], compounds: &[Compound]) -> Vec<PassiveSet> {
    let res = |d: usize| hg.kinds[d] == DeviceKind::Resistor;
    let mut parent: Vec<usize> = (0..hg.device_count()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for (n, devs) in hg.net_devices.iter().enumerate() {
        if rail(classes, NetId(n as u16)) {
            continue;
        }
        let internal = devs.iter().all(|d| {
            let i = d.0 as usize;
            res(i) || (matches!(hg.kinds[i], DeviceKind::Nmos | DeviceKind::Pmos) && net(hg, i, "G") == Some(NetId(n as u16)) && [net(hg, i, "D"), net(hg, i, "S")].iter().all(|&x| x != Some(NetId(n as u16))))
        });
        let rs: Vec<usize> = devs.iter().map(|d| d.0 as usize).filter(|&d| res(d)).collect();
        if internal && rs.len() >= 2 {
            for w in rs.windows(2) {
                let (x, y) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
                parent[x.max(y)] = x.min(y);
            }
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
/// per pair when they share model and width: role BandgapCore, rule `"bandgap_r"`.
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

/// `true` when `counts`, sorted, are `[1, 1, 2, …, 2^(N-1)]`, N ≥ 2 (card D-h:
/// `cells::cap_array::bits` is out of the annotator's reach).
fn binary(counts: &[u32]) -> bool {
    let mut c = counts.to_vec();
    c.sort_unstable();
    c.len() >= 3 && c[0] == 1 && c[1..].iter().enumerate().all(|(i, &u)| u == 1 << i)
}

/// Capacitors sharing a top plate `P` with one W, L and model: one set each,
/// the one-unit cap whose other plate is a rail first (the bank's termination,
/// cellgen's old `dac_banks` rule), role DacBank (rule `"dac_bank"`) when the
/// counts are binary, else FeedbackRatio (`"cap_ratio"`). Two banks joined by
/// exactly one other capacitor are one `"split_dac"` set holding that bridge;
/// `bridge_cap_value` when its area is off `(C_T^LSB / C_T^MSB)·C_u` by more
/// than 1 % (Philis tolerance), the LSB bank being the terminated one.
#[must_use]
pub fn capacitor_sets(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], diags: &mut Vec<Diagnostic>) -> Vec<PassiveSet> {
    let cap = |d: usize| hg.kinds[d] == DeviceKind::Capacitor;
    // (top plate, model, W, L) → members.
    type Key = (NetId, u16, Option<i64>, Option<i64>);
    let mut banks: Vec<(Key, Vec<usize>)> = Vec::new();
    for d in (0..hg.device_count()).filter(|&d| cap(d)) {
        let Some(p) = net(hg, d, "P") else { continue };
        let key = (p, drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm);
        match banks.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(d),
            None => banks.push((key, vec![d])),
        }
    }
    banks.retain(|(_, v)| v.len() > 1);
    let terminated = |d: usize| drawn[d].fingers == 1 && net(hg, d, "N").is_some_and(|n| rail(classes, n));
    for (_, v) in &mut banks {
        v.sort_by_key(|&d| (!terminated(d), d));
    }
    let units = |v: &[usize]| v.iter().map(|&d| drawn[d].fingers).sum::<u32>();
    let in_bank = |d: usize, banks: &[(_, Vec<usize>)]| banks.iter().any(|(_, v): &(_, Vec<usize>)| v.contains(&d));
    let mut out = Vec::new();
    let mut used = vec![false; banks.len()];
    for i in 0..banks.len() {
        for j in i + 1..banks.len() {
            let (ti, tj) = (banks[i].0 .0, banks[j].0 .0);
            let bridges: Vec<usize> = (0..hg.device_count())
                .filter(|&d| cap(d) && !in_bank(d, &banks))
                .filter(|&d| {
                    let (p, n) = (net(hg, d, "P"), net(hg, d, "N"));
                    (p, n) == (Some(ti), Some(tj)) || (p, n) == (Some(tj), Some(ti))
                })
                .collect();
            if used[i] || used[j] || bridges.len() != 1 {
                continue;
            }
            (used[i], used[j]) = (true, true);
            let ca = bridges[0];
            let (lsb, msb) = if banks[i].1.iter().any(|&d| terminated(d)) { (i, j) } else { (j, i) };
            let area = |d: usize| (drawn[d].w_finger_nm.unwrap_or(0) * drawn[d].l_nm.unwrap_or(0)) as f64;
            let want = f64::from(units(&banks[lsb].1)) / f64::from(units(&banks[msb].1)) * area(banks[lsb].1[0]);
            let got = area(ca) * f64::from(drawn[ca].fingers);
            if (got - want).abs() > 0.01 * got {
                diags.push(Diagnostic {
                    kind: "bridge_cap_value",
                    devices: vec![DeviceId(ca as u16)],
                    message: format!("split-DAC bridge area {got} nm², want (C_T^LSB/C_T^MSB)·C_u = {want} nm²"),
                });
            }
            let devices: Vec<usize> = banks[lsb].1.iter().chain(&banks[msb].1).copied().chain([ca]).collect();
            let reference = banks[lsb].1.first().filter(|&&d| terminated(d)).map(|&d| DeviceId(d as u16));
            out.push(PassiveSet { bridge: Some(DeviceId(ca as u16)), reference, ..set(devices, "split_dac", SetRole::DacBank) });
        }
    }
    for (_, (_, v)) in banks.iter().enumerate().filter(|(k, _)| !used[*k]) {
        let counts: Vec<u32> = v.iter().map(|&d| drawn[d].fingers).collect();
        let (rule, role) = if binary(&counts) { ("dac_bank", SetRole::DacBank) } else { ("cap_ratio", SetRole::FeedbackRatio) };
        let reference = v.first().filter(|&&d| terminated(d)).map(|&d| DeviceId(d as u16));
        out.push(PassiveSet { reference, ..set(v.clone(), rule, role) });
    }
    out
}

/// Per pair (FET sources or BJT emitters), the two resistors through which the
/// halves reach one common node, one each: a Ratio set whose ratio is the
/// inverse of the pair's (H09-26: 1X 4 kΩ / 2X 2 kΩ). `"degeneration_ratio"`
/// when the drawn resistor lengths are not that inverse.
#[must_use]
pub fn degeneration(hg: &BipartiteHypergraph, drawn: &[Drawn], pairs: &[(DeviceId, DeviceId)], diags: &mut Vec<Diagnostic>) -> Vec<PassiveSet> {
    let mut out = Vec::new();
    for &(a, b) in pairs {
        let leg = |d: DeviceId| {
            let i = d.0 as usize;
            let s = net(hg, i, if matches!(hg.kinds[i], DeviceKind::Npn | DeviceKind::Pnp) { "E" } else { "S" })?;
            let rs: Vec<usize> = hg.net_devices[s.0 as usize].iter().map(|x| x.0 as usize).filter(|&x| hg.kinds[x] == DeviceKind::Resistor).collect();
            let [r] = rs[..] else { return None };
            let far = hg.device_nets[r].iter().copied().find(|&n| n != s)?;
            Some((r, far))
        };
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

/// FET couples of one signature (kind, model, finger W, L, fingers) with
/// distinct gates whose sources are distinct non-rail nets each reaching one
/// common node through exactly one resistor: the degenerated diff pair no
/// joined-source pattern sees (M1 regression note).
#[must_use]
pub fn degenerated_pairs(hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[crate::NetRole]) -> Vec<(DeviceId, DeviceId)> {
    let fets: Vec<usize> = (0..hg.device_count()).filter(|&d| matches!(hg.kinds[d], DeviceKind::Nmos | DeviceKind::Pmos)).collect();
    let tail = |d: usize| {
        let s = net(hg, d, "S")?;
        if matches!(roles[s.0 as usize], crate::NetRole::Supply | crate::NetRole::Ground) {
            return None;
        }
        let rs: Vec<usize> = hg.net_devices[s.0 as usize].iter().map(|x| x.0 as usize).filter(|&x| hg.kinds[x] == DeviceKind::Resistor).collect();
        let [r] = rs[..] else { return None };
        hg.device_nets[r].iter().copied().find(|&n| n != s)
    };
    let sig = |d: usize| (hg.kinds[d] as u8, drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm, drawn[d].fingers);
    let mut groups: std::collections::HashMap<_, Vec<usize>> = std::collections::HashMap::new();
    for &d in fets.iter().filter(|&&d| drawn[d].l_nm.is_some()) {
        if let Some(t) = tail(d) {
            groups.entry((sig(d), t.0)).or_default().push(d);
        }
    }
    let mut out = Vec::new();
    for g in groups.values() {
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
/// (H09-43): rule `"diode_set"`, role Other.
#[must_use]
pub fn diode_sets(hg: &BipartiteHypergraph, drawn: &[Drawn]) -> Vec<PassiveSet> {
    let mut groups: Vec<(_, Vec<usize>)> = Vec::new();
    for d in (0..hg.device_count()).filter(|&d| hg.kinds[d] == DeviceKind::Diode) {
        for t in ["P", "N"] {
            let key = (t, net(hg, d, t), drawn[d].model, drawn[d].w_finger_nm, drawn[d].l_nm);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, v)) => v.push(d),
                None => groups.push((key, vec![d])),
            }
        }
    }
    groups.into_iter().filter(|(_, v)| v.len() > 1).map(|(_, v)| set(v, "diode_set", SetRole::Other)).collect()
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
}
