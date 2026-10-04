//! Requirement graph and HSMPG group tree (EXT-13; survey §3.2.1 types and
//! importance order, Algorithm 3.1). Overlapping requirements (a tail in a
//! compound and in a shared-bias group, AA-01) live here, not in the disjoint
//! `Problem::blocks`.

use analog::intent::{Compound, ConstraintId, GroupKind, GroupNode, ReqType};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::DeviceId;
use pnr_core::BipartiteHypergraph;

use crate::catalog::roles_of;
use crate::pattern::PatternMatch;
use crate::policy::Policy;

/// One requirement edge, stored with `canon[a] <= canon[b]`. `source` indexes
/// the match, compound, group or net it came from in its input slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Req {
    pub a: DeviceId,
    pub b: DeviceId,
    pub ty: ReqType,
    pub source: ConstraintId,
}

/// Every requirement edge: `MatchSym` per compound pair; `MatchBlock` per declared
/// pair of every match (all of them, not only the disjoint ones: AA-01) not already
/// `MatchSym`, and a star from `g[0]` of each shared-bias and passive group (callers
/// put a reference first); `ProxBlock` per declared prox couple and per self to its
/// pattern's first pair; `Sym` as a path over each compound's members; `ProxNet` a
/// star from the canonically first device of each net that is not Supply, Ground,
/// Substrate or Clock (card D-j) with at most `policy.pn_max_degree` devices.
#[allow(clippy::too_many_arguments)] // the plan's signature: one slice per evidence source
#[must_use]
pub fn requirements(
    matches: &[PatternMatch],
    compounds: &[Compound],
    shared_bias: &[Vec<DeviceId>],
    passive: &[Vec<DeviceId>],
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    canon: &[u64],
    policy: &Policy,
) -> Vec<Req> {
    let mut out = Vec::new();
    let mut push = |a: DeviceId, b: DeviceId, ty, src: usize| {
        if a != b {
            let (a, b) = if (canon[a.0 as usize], a.0) <= (canon[b.0 as usize], b.0) { (a, b) } else { (b, a) };
            out.push(Req { a, b, ty, source: ConstraintId(src as u32) });
        }
    };
    let mut sym_pair = std::collections::HashSet::new();
    for (ci, c) in compounds.iter().enumerate() {
        for &(a, b) in &c.pairs {
            push(a, b, ReqType::MatchSym, ci);
            sym_pair.insert((a.0.min(b.0), a.0.max(b.0)));
        }
        let mut members: Vec<DeviceId> = c.pairs.iter().flat_map(|&(a, b)| [a, b]).chain(c.selfs.iter().copied()).collect();
        members.sort_by_key(|d| (canon[d.0 as usize], d.0));
        members.windows(2).for_each(|w| push(w[0], w[1], ReqType::Sym, ci));
    }
    for (mi, m) in matches.iter().enumerate() {
        let dev = |s: u8| DeviceId(m.instances[s as usize] as u16);
        let r = roles_of(m.pattern);
        for &(x, y, _) in r.pairs {
            let (a, b) = (dev(x), dev(y));
            if !sym_pair.contains(&(a.0.min(b.0), a.0.max(b.0))) {
                push(a, b, ReqType::MatchBlock, mi);
            }
        }
        for &(x, y) in r.prox {
            push(dev(x), dev(y), ReqType::ProxBlock, mi);
        }
        if let Some(&(p0, _, _)) = r.pairs.first() {
            r.selfs.iter().for_each(|&s| push(dev(s), dev(p0), ReqType::ProxBlock, mi));
        }
    }
    for (gi, g) in shared_bias.iter().chain(passive).enumerate() {
        let src = if gi < shared_bias.len() { gi } else { gi - shared_bias.len() };
        g.iter().skip(1).for_each(|&d| push(g[0], d, ReqType::MatchBlock, src));
    }
    for (n, devs) in hg.net_devices.iter().enumerate() {
        if matches!(classes[n].class, NetClass::Supply | NetClass::Ground | NetClass::Substrate | NetClass::Clock) {
            continue;
        }
        let mut ds = devs.clone();
        ds.sort_by_key(|d| (canon[d.0 as usize], d.0));
        ds.dedup();
        if ds.len() <= policy.pn_max_degree {
            ds.iter().skip(1).for_each(|&d| push(ds[0], d, ReqType::ProxNet, n));
        }
    }
    out
}

/// Algorithm 3.1: per type in importance order, union-find over the current
/// super-nodes on that type's edges; every class of more than one becomes a node
/// (`Matching` for MatchSym/MatchBlock, `Symmetry` for Sym, `Proximity`
/// otherwise) and is contracted. A final `Root` holds what remains and is last.
/// Classes become nodes, and children sit within a node, in order of
/// `(min canon of the subtree, device count)`; names and ids never decide
/// anything but exact label ties.
#[must_use]
pub fn hsmpg(n_devices: usize, reqs: &[Req], canon: &[u64]) -> Vec<GroupNode> {
    let mut edges = reqs.to_vec();
    edges.sort_by_key(|r| (r.ty, canon[r.a.0 as usize], canon[r.b.0 as usize], r.a.0, r.b.0));
    // Super-node ids: devices `0..n`, group node `k` is `n + k`.
    let mut top: Vec<usize> = (0..n_devices).collect();
    let mut key: Vec<(u64, usize)> = (0..n_devices).map(|d| (canon[d], 1)).collect();
    let mut tree: Vec<GroupNode> = Vec::new();
    let node_of = |items: &mut Vec<usize>, kind, key: &[(u64, usize)], tree: &mut Vec<GroupNode>| {
        items.sort_by_key(|&s| (key[s], s));
        let devices = items.iter().filter(|&&s| s < n_devices).map(|&s| DeviceId(s as u16)).collect();
        let children = items.iter().filter(|&&s| s >= n_devices).map(|&s| (s - n_devices) as u32).collect();
        tree.push(GroupNode { kind, devices, children });
    };
    for ty in [ReqType::MatchSym, ReqType::MatchBlock, ReqType::ProxBlock, ReqType::Sym, ReqType::ProxNet] {
        let mut parent: Vec<u32> = (0..(n_devices + tree.len()) as u32).collect();
        let find = |p: &mut Vec<u32>, mut x: u32| {
            while p[x as usize] != x {
                p[x as usize] = p[p[x as usize] as usize];
                x = p[x as usize];
            }
            x
        };
        for r in edges.iter().filter(|r| r.ty == ty) {
            let (x, y) = (find(&mut parent, top[r.a.0 as usize] as u32), find(&mut parent, top[r.b.0 as usize] as u32));
            if x != y {
                parent[x.max(y) as usize] = x.min(y);
            }
        }
        let mut alive: Vec<usize> = top.clone();
        alive.sort_unstable();
        alive.dedup();
        let mut classes: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
        for s in alive {
            classes.entry(find(&mut parent, s as u32)).or_default().push(s);
        }
        let mut classes: Vec<Vec<usize>> = classes.into_values().filter(|c| c.len() > 1).collect();
        let sub = |key: &[(u64, usize)], c: &[usize]| (c.iter().map(|&s| key[s].0).min().unwrap(), c.iter().map(|&s| key[s].1).sum::<usize>());
        classes.sort_by_cached_key(|c| (sub(&key, c), c.clone()));
        let kind = match ty {
            ReqType::MatchSym | ReqType::MatchBlock => GroupKind::Matching,
            ReqType::Sym => GroupKind::Symmetry,
            _ => GroupKind::Proximity,
        };
        let mut remap: Vec<usize> = (0..n_devices + tree.len()).collect();
        for mut c in classes {
            let id = n_devices + tree.len();
            key.push(sub(&key, &c));
            c.iter().for_each(|&s| remap[s] = id);
            node_of(&mut c, kind, &key, &mut tree);
        }
        top.iter_mut().for_each(|t| *t = remap[*t]);
    }
    let mut rest: Vec<usize> = top;
    rest.sort_unstable();
    rest.dedup();
    node_of(&mut rest, GroupKind::Root, &key, &mut tree);
    tree
}

/// The tree under its `Root` as one string, members and children sorted by text
/// (`Kind{a,b,Kind{..}}`): an id-free view for tests and reports.
#[must_use]
pub fn render(tree: &[GroupNode], names: &[&str]) -> String {
    fn show(tree: &[GroupNode], i: usize, names: &[&str]) -> String {
        let n = &tree[i];
        let mut items: Vec<String> = n.devices.iter().map(|d| names[d.0 as usize].to_string()).collect();
        items.extend(n.children.iter().map(|&c| show(tree, c as usize, names)));
        items.sort();
        format!("{:?}{{{}}}", n.kind, items.join(","))
    }
    tree.len().checked_sub(1).map_or_else(String::new, |r| show(tree, r, names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{netrole, pattern, size, AnnotationConfig};
    use analog::intent::{AxisDir, SymKind};
    use pnr_core::ids::AxisId;
    use pnr_core::Netlist;

    fn compound(pairs: &[(u16, u16)], selfs: &[u16]) -> Compound {
        Compound {
            id: ConstraintId(0),
            axis: AxisId(0),
            dir: AxisDir::V,
            kind: SymKind::Mirror,
            pairs: pairs.iter().map(|&(a, b)| (DeviceId(a), DeviceId(b))).collect(),
            selfs: selfs.iter().map(|&s| DeviceId(s)).collect(),
            net_pairs: Vec::new(),
            self_nets: Vec::new(),
            set_pairs: Vec::new(),
        }
    }

    /// `(reqs, rendered tree)` of `nl` with hand-built compounds and shared groups.
    fn run(nl: &Netlist, c: &[Compound], shared: &[Vec<DeviceId>]) -> (Vec<Req>, String) {
        let cfg = AnnotationConfig::default();
        let hg = BipartiteHypergraph::from_netlist(nl);
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        let roles = netrole::classify_nets(&hg, &cfg);
        let canon = pattern::canonical_labels(&hg, &drawn, &models, &roles);
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        let all = pattern::recognize_all(&hg, &drawn, &roles, &cfg, &canon, &names);
        let p = crate::annotate(nl, &cfg);
        let reqs = requirements(&all, c, shared, &[], &hg, &p.net_classes, &canon, &cfg.policy);
        let tree = hsmpg(nl.devices.len(), &reqs, &canon);
        (reqs, render(&tree, &names))
    }

    #[test]
    fn ota5t_tree() {
        // XM1..XM5 = ids 0..4.
        let (_, t) = run(&crate::tests::ota(), &[compound(&[(0, 1), (2, 3)], &[4])], &[]);
        assert_eq!(t, "Root{Symmetry{Matching{XM3,XM4},Proximity{Matching{XM1,XM2},XM5}}}");
    }

    #[test]
    fn tail_in_two_requirements() {
        let nl = crate::tests::three_stage();
        let id = |n: &str| DeviceId(nl.devices.iter().position(|d| d.name == n).unwrap() as u16);
        let (m1, m2, m3, m4, m5, m7, m9) = (id("M1"), id("M2"), id("M3"), id("M4"), id("M5"), id("M7"), id("M9"));
        let c = Compound { pairs: vec![(m1, m2), (m4, m5)], selfs: vec![m3], ..compound(&[], &[]) };
        let (reqs, t) = run(&nl, &[c], &[vec![m3, m7, m9]]);
        let has = |x: DeviceId, y: DeviceId, ty| reqs.iter().any(|r| r.ty == ty && ((r.a, r.b) == (x, y) || (r.a, r.b) == (y, x)));
        assert!(has(m3, m7, ReqType::MatchBlock) && has(m3, m9, ReqType::MatchBlock), "{reqs:?}");
        assert!(reqs.iter().any(|r| r.ty == ReqType::Sym && (r.a == m3 || r.b == m3)), "{reqs:?}");
        // One Symmetry node, and its subtree holds M3, M7 and M9.
        assert_eq!(t.matches("Symmetry{").count(), 1, "{t}");
        let sym = &t[t.find("Symmetry{").unwrap()..];
        let mut depth = 0;
        let end = sym.char_indices().find(|&(_, ch)| {
            depth += i32::from(ch == '{') - i32::from(ch == '}');
            ch == '}' && depth == 0
        });
        let sym = &sym[..end.unwrap().0];
        for n in ["M3", "M7", "M9"] {
            assert!(sym.split(|c: char| !c.is_alphanumeric()).any(|w| w == n), "{n} not under Symmetry: {t}");
        }
    }
}
