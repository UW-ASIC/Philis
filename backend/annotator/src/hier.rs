//! Hierarchy (EXT-27): identical sub-circuit instances become matched couples,
//! ≥ 3 identical instances on a shared bias net an array. Reads the parser's
//! `Netlist.insts` / `device_inst` (FLOW-07); a netlist without them has no
//! hierarchy and every function returns empty.

use std::collections::HashMap;

use pnr_core::ids::{DeviceId, NetId};
use pnr_core::Netlist;

use crate::size::Drawn;
use crate::symmetry::sig;

/// Devices of instance `i` (its own and its sub-instances'), with their names relative to
/// its path, in device order. A device whose name lacks the `path/` prefix is left out.
/// Cost: O(devices × hierarchy depth).
///
/// # Panics
/// When `i` is out of bounds of `nl.insts`.
#[must_use]
pub fn devices(nl: &Netlist, i: u32) -> Vec<(DeviceId, &str)> {
    let prefix = format!("{}/", nl.insts[i as usize].path);
    let under = |mut k: Option<u32>| {
        while let Some(j) = k {
            if j == i {
                return true;
            }
            k = nl.insts[j as usize].parent;
        }
        false
    };
    (0..nl.devices.len())
        .filter(|&d| under(nl.device_inst.get(d).copied().flatten()))
        .filter_map(|d| nl.devices[d].name.strip_prefix(&prefix).map(|r| (DeviceId(d as u16), r)))
        .collect()
}

/// Device couples of instances `a`, `b` by name below the instance path (`X1/M3` ↔ `X2/M3`);
/// `None` when the device lists or any couple's (kind, model, finger W, L, fingers) differ.
/// Drawn signatures, not canonical labels: those include outside connections. Instances with no
/// devices never correspond. Couples come in order of the relative name.
///
/// # Panics
/// When `a` or `b` is out of bounds of `nl.insts`, or `drawn` is shorter than `nl.devices`.
#[must_use]
pub fn corresponding(nl: &Netlist, drawn: &[Drawn], a: u32, b: u32) -> Option<Vec<(DeviceId, DeviceId)>> {
    let (mut da, mut db) = (devices(nl, a), devices(nl, b));
    if da.is_empty() || da.len() != db.len() {
        return None;
    }
    da.sort_by_key(|&(_, n)| n);
    db.sort_by_key(|&(_, n)| n);
    let s = |d: DeviceId| sig(nl.devices[d.0 as usize].kind, &drawn[d.0 as usize]);
    da.iter().zip(&db).map(|(&(x, nx), &(y, ny))| (nx == ny && s(x) == s(y)).then_some((x, y))).collect()
}

/// Instances grouped by (subckt, parent): groups in order of their first instance, members in
/// instance order. Cost: O(instances × groups).
fn siblings(nl: &Netlist) -> Vec<Vec<u32>> {
    let mut g: Vec<Vec<u32>> = Vec::new();
    for (i, x) in nl.insts.iter().enumerate() {
        match g.iter_mut().find(|v| nl.insts[v[0] as usize].subckt == x.subckt && nl.insts[v[0] as usize].parent == x.parent) {
            Some(v) => v.push(i as u32),
            None => g.push(vec![i as u32]),
        }
    }
    g
}

/// Instance pairs of one subckt under one parent, exactly two such instances, device-wise identical
/// (`corresponding` is `Some`); a subckt with ≥ 3 instances is an array candidate, never paired (**Philis policy**).
#[must_use]
pub fn same_template(nl: &Netlist, drawn: &[Drawn]) -> Vec<(u32, u32)> {
    siblings(nl).into_iter().filter(|g| g.len() == 2 && corresponding(nl, drawn, g[0], g[1]).is_some()).map(|g| (g[0], g[1])).collect()
}

/// ≥ 3 identical instances of one subckt under one parent with one bias port net in common
/// (`bias[net]`: the structural Bias rule, [`crate::classify::bias_lines`], since net classes are
/// refined only after the requirement graph); per bias net the instances that carry it (and
/// match its first), largest set first, instance order; a set two bias nets both give is listed once.
///
/// # Panics
/// When `bias` is shorter than the net count of an instance port.
#[must_use]
pub fn arrays(nl: &Netlist, drawn: &[Drawn], bias: &[bool]) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = Vec::new();
    for g in siblings(nl).into_iter().filter(|g| g.len() >= 3) {
        let mut bias: Vec<NetId> = g.iter().flat_map(|&i| nl.insts[i as usize].ports.iter().copied()).filter(|n| bias[n.0 as usize]).collect();
        bias.sort_unstable_by_key(|n| n.0);
        bias.dedup();
        for b in bias {
            let on: Vec<u32> = g.iter().copied().filter(|&i| nl.insts[i as usize].ports.contains(&b)).collect();
            let set: Vec<u32> = on.iter().copied().filter(|&i| i == on[0] || corresponding(nl, drawn, on[0], i).is_some()).collect();
            if set.len() >= 3 && !out.contains(&set) {
                out.push(set);
            }
        }
    }
    out.sort_by(|x, y| y.len().cmp(&x.len()).then_with(|| x.cmp(y)));
    out
}

/// Port nets of `a` and `b` pair up: per formal port k, equal nets (shared) or a couple, and no net in two couples.
/// Port lists of different lengths never pair; two empty lists do.
///
/// # Panics
/// When `a` or `b` is out of bounds of `nl.insts`.
#[must_use]
pub fn ports_pair(nl: &Netlist, a: u32, b: u32) -> bool {
    let (pa, pb) = (&nl.insts[a as usize].ports, &nl.insts[b as usize].ports);
    let mut mate: HashMap<NetId, NetId> = HashMap::new();
    let mut bind = |x: NetId, y: NetId| *mate.entry(x).or_insert(y) == y;
    pa.len() == pb.len() && pa.iter().zip(pb).all(|(&x, &y)| bind(x, y) && bind(y, x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{annotate, size, AnnotationConfig};
    use analog::intent::GroupKind;
    use pnr_core::netlist::{DeviceKind, Net, SubcktInst};
    use pnr_core::BipartiteHypergraph;

    /// `(name, kind, g, d, s, b, w nm, l nm)` over formal or internal net names.
    type Dev = (&'static str, DeviceKind, &'static str, &'static str, &'static str, &'static str, i64, i64);
    const N: DeviceKind = DeviceKind::Nmos;
    const P: DeviceKind = DeviceKind::Pmos;
    const OTA_PORTS: [&str; 6] = ["inp", "inn", "out", "vdd", "vss", "vb"];
    const OTA: [Dev; 5] = [
        ("MN1", N, "inp", "x", "tail", "vss", 4_000, 500),
        ("MN2", N, "inn", "out", "tail", "vss", 4_000, 500),
        ("MP3", P, "x", "x", "vdd", "vdd", 6_000, 500),
        ("MP4", P, "x", "out", "vdd", "vdd", 6_000, 500),
        ("MN5", N, "vb", "tail", "vss", "vss", 8_000, 500),
    ];

    fn net(nl: &mut Netlist, name: &str) -> NetId {
        let i = nl.nets.iter().position(|n| n.name == name).unwrap_or_else(|| {
            nl.nets.push(Net { name: name.into() });
            nl.nets.len() - 1
        });
        NetId(i as u16)
    }

    /// Flattens one top-level instance the way the parser does: `path/<dev>`, `path/<net>`.
    fn inst(nl: &mut Netlist, path: &str, subckt: &str, formals: &[&str], actuals: &[&str], devs: &[Dev]) {
        let i = nl.insts.len() as u32;
        let ports = actuals.iter().map(|a| net(nl, a)).collect();
        nl.insts.push(SubcktInst { path: path.into(), subckt: subckt.into(), parent: None, ports });
        let map = |nl: &mut Netlist, f: &str| match formals.iter().position(|&x| x == f) {
            Some(k) => net(nl, actuals[k]),
            None => net(nl, &format!("{path}/{f}")),
        };
        for &(name, kind, g, d, s, b, w, l) in devs {
            let (g, d, s, b) = (map(nl, g), map(nl, d), map(nl, s), map(nl, b));
            nl.devices.push(crate::tests::fet(&format!("{path}/{name}"), kind, g.0, d.0, s.0, b.0, w, l));
            nl.device_inst.push(Some(i));
        }
    }

    /// Some `kind` node's subtree holds every device of `devs`.
    fn grouped(p: &crate::Problem, kind: GroupKind, devs: &[DeviceId]) -> bool {
        fn under(t: &[analog::intent::GroupNode], g: usize, out: &mut Vec<DeviceId>) {
            out.extend(&t[g].devices);
            t[g].children.iter().for_each(|&c| under(t, c as usize, out));
        }
        let t = &p.intent.tree;
        (0..t.len()).filter(|&g| t[g].kind == kind).any(|g| {
            let mut v = Vec::new();
            under(t, g, &mut v);
            devs.iter().all(|d| v.contains(d))
        })
    }

    fn drawn(nl: &Netlist) -> Vec<Drawn> {
        let mut m = Vec::new();
        nl.devices.iter().map(|d| size::drawn(d, &mut m)).collect()
    }

    fn bias(nl: &Netlist) -> Vec<bool> {
        crate::classify::bias_lines(&BipartiteHypergraph::from_netlist(nl))
    }

    fn id(nl: &Netlist, n: &str) -> DeviceId {
        DeviceId(nl.devices.iter().position(|d| d.name == n).unwrap() as u16)
    }

    fn two_otas() -> Netlist {
        let mut nl = Netlist::default();
        inst(&mut nl, "X1", "ota", &OTA_PORTS, &["inp1", "inn1", "out1", "vdd", "vss", "vb"], &OTA);
        inst(&mut nl, "X2", "ota", &OTA_PORTS, &["inp2", "inn2", "out2", "vdd", "vss", "vb"], &OTA);
        nl
    }

    #[test]
    fn two_identical_otas_pair_up() {
        let nl = two_otas();
        let dr = drawn(&nl);
        assert_eq!(same_template(&nl, &dr), [(0, 1)]);
        let p = annotate(&nl, &AnnotationConfig::default());
        // lib.rs passes the EXT-27 couples through: one Matching subtree holds X1/MN1 and X2/MN1
        // (shared bias alone matches only the tails on `vb`).
        assert!(grouped(&p, GroupKind::Matching, &[id(&nl, "X1/MN1"), id(&nl, "X2/MN1")]), "{:?}", p.intent.tree);
        // Each OTA keeps its own input pair.
        let pairs: Vec<(DeviceId, DeviceId)> = p.intent.compounds.iter().flat_map(|c| c.pairs.iter().copied()).collect();
        let has = |x: &str, y: &str| pairs.iter().any(|&(a, b)| (a, b) == (id(&nl, x), id(&nl, y)) || (b, a) == (id(&nl, x), id(&nl, y)));
        assert!(has("X1/MN1", "X1/MN2") && has("X2/MN1", "X2/MN2"), "{pairs:?}");
        assert!(!has("X1/MN1", "X2/MN1"), "{pairs:?}");
    }

    #[test]
    fn identical_inverters_mirror() {
        let inv: [Dev; 2] = [("MN", N, "i", "o", "vss", "vss", 1_000, 150), ("MP", P, "i", "o", "vdd", "vdd", 2_000, 150)];
        let f = ["i", "o", "vdd", "vss"];
        let mut nl = Netlist::default();
        inst(&mut nl, "X1", "inv", &f, &["a", "b", "vdd", "vss"], &inv);
        inst(&mut nl, "X2", "inv", &f, &["c", "d", "vdd", "vss"], &inv);
        assert!(ports_pair(&nl, 0, 1));
        // Crossed ports: `a` pairs with both `a` and `b`, so no couple seeds.
        let mut crossed = nl.clone();
        crossed.insts[1].ports = ["a", "a", "vdd", "vss"].map(|n| net(&mut crossed, n)).to_vec();
        assert!(!ports_pair(&crossed, 0, 1));
        let p = annotate(&nl, &AnnotationConfig::default());
        let pairs: Vec<(DeviceId, DeviceId)> = p.intent.compounds.iter().flat_map(|c| c.pairs.iter().copied()).collect();
        for (x, y) in [("X1/MN", "X2/MN"), ("X1/MP", "X2/MP")] {
            let (x, y) = (id(&nl, x), id(&nl, y));
            assert!(pairs.contains(&(x, y)) || pairs.contains(&(y, x)), "{pairs:?}");
        }
    }

    fn cells(k: usize) -> Netlist {
        let cell: [Dev; 2] = [("MC", N, "vb", "o", "vss", "vss", 2_000, 1_000), ("MK", N, "vc", "d", "vss", "vss", 2_000, 500)];
        let mut nl = Netlist::default();
        let (vb, vss) = (net(&mut nl, "vb"), net(&mut nl, "vss"));
        nl.devices.push(crate::tests::fet("MREF", N, vb.0, vb.0, vss.0, vss.0, 2_000, 1_000));
        nl.device_inst.push(None);
        for i in 0..k {
            let (o, d) = (format!("out{i}"), format!("o{i}"));
            inst(&mut nl, &format!("X{i}"), "cell", &["o", "d", "vb", "vc", "vss"], &[&o, &d, "vb", "vc", "vss"], &cell);
        }
        nl
    }

    #[test]
    fn bias_shared_array() {
        let nl = cells(4);
        let p = annotate(&nl, &AnnotationConfig::default());
        assert_eq!(arrays(&nl, &drawn(&nl), &bias(&nl)), [vec![0, 1, 2, 3]]);
        let arr: Vec<_> = p.intent.order.iter().filter(|o| o.dir == analog::intent::AxisDir::H && o.steps.len() == 4).collect();
        assert_eq!(arr.len(), 1, "{:?}", p.intent.order);
        assert!(arr[0].reversible && arr[0].steps.iter().all(|s| s.len() == 2), "{arr:?}");
        // lib.rs passes the array devices through: one Proximity subtree holds every MK (MK shares
        // no ProxNet with its MC, so only the EXT-27 ProxBlock star joins them).
        let mk: Vec<DeviceId> = (0..4).map(|i| id(&nl, &format!("X{i}/MK"))).collect();
        assert!(grouped(&p, GroupKind::Proximity, &mk), "{:?}", p.intent.tree);
        // Three or more siblings are array candidates, never paired; exactly two pair.
        assert!(same_template(&nl, &drawn(&nl)).is_empty());
        let nl = cells(2);
        assert_eq!(arrays(&nl, &drawn(&nl), &bias(&nl)), Vec::<Vec<u32>>::new());
        assert_eq!(same_template(&nl, &drawn(&nl)), [(0, 1)]);
    }

    /// EXT-27: ProxNet stays inside an instance, one star per instance on a shared net. Both
    /// OTAs drive `out1`: each instance keeps its MN2–MP4 edge, no edge crosses X1/X2.
    #[test]
    fn proxnet_per_instance() {
        let mut nl = Netlist::default();
        inst(&mut nl, "X1", "ota", &OTA_PORTS, &["inp1", "inn1", "out1", "vdd", "vss", "vb"], &OTA);
        inst(&mut nl, "X2", "ota", &OTA_PORTS, &["inp2", "inn2", "out1", "vdd", "vss", "vb"], &OTA);
        let cfg = AnnotationConfig::default();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        let canon = crate::pattern::canonical_labels(&hg, &drawn, &models, &crate::netrole::classify_nets(&hg, &cfg));
        let classes = annotate(&nl, &cfg).net_classes;
        let reqs = crate::graph::requirements(&[], &[], &[], &[], &[], &[], &[], &nl.device_inst, &hg, &classes, &canon, &cfg.policy);
        let out1 = net(&mut nl, "out1");
        let pn: Vec<(u16, u16)> = reqs.iter().filter(|r| r.ty == analog::intent::ReqType::ProxNet && r.source.0 == u32::from(out1.0)).map(|r| (r.a.0.min(r.b.0), r.a.0.max(r.b.0))).collect();
        let e = |x: &str, y: &str| (id(&nl, x).0.min(id(&nl, y).0), id(&nl, x).0.max(id(&nl, y).0));
        assert_eq!(pn.len(), 2, "{pn:?}");
        assert!(pn.contains(&e("X1/MN2", "X1/MP4")) && pn.contains(&e("X2/MN2", "X2/MP4")), "{pn:?}");
    }

    #[test]
    fn flat_netlist_unchanged() {
        let nl = crate::tests::three_stage();
        assert!(same_template(&nl, &drawn(&nl)).is_empty() && arrays(&nl, &drawn(&nl), &bias(&nl)).is_empty());
    }
}
