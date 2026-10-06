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

/// One requirement edge between two distinct devices, stored with
/// `(canon[a], a) <= (canon[b], b)` so the same pair always reads the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Req {
    /// The canonically first end.
    pub a: DeviceId,
    /// The canonically second end; never equal to `a`.
    pub b: DeviceId,
    /// Requirement type; [`hsmpg`] contracts types in importance order.
    pub ty: ReqType,
    /// Index of the match, compound, group, couple or net the edge came from,
    /// within the input slice its `ty` names (see [`requirements`]).
    pub source: ConstraintId,
}

/// `(min, max)` of two device ids: the key of an unordered pair.
fn unordered(a: DeviceId, b: DeviceId) -> (u16, u16) {
    (a.0.min(b.0), a.0.max(b.0))
}

/// Root of `x` in the union-find forest `p`, halving the path on the way.
///
/// # Panics
/// When `x` or any parent link is out of bounds of `p`.
pub(crate) fn uf_find(p: &mut [usize], mut x: usize) -> usize {
    while p[x] != x {
        p[x] = p[p[x]];
        x = p[x];
    }
    x
}

/// Every requirement edge: `MatchSym` per compound pair; `MatchBlock` per declared
/// pair of every match (all of them, not only the disjoint ones: AA-01) not already
/// `MatchSym`, and a star from `g[0]` of each shared-bias and passive group (callers
/// put a reference first); `ProxBlock` per declared prox couple and per self to its
/// pattern's first pair, and a star from `g[0]` of each sidecar group; `Sym` as a path over each compound's members; `ProxNet` a
/// star from the canonically first device of each net that is not Supply, Ground,
/// Substrate or Clock (card D-j) with at most `policy.pn_max_degree` devices.
/// EXT-27: `MatchBlock` per couple of identical instances (`hier_pairs`) not already
/// `MatchSym`; `ProxBlock` a star from `g[0]` of each instance array; `ProxNet` does not
/// cross an instance boundary (**Philis policy**): an edge joins devices of one innermost
/// instance (`inst`, `netlist.device_inst`; empty = all top level).
///
/// Self-edges are dropped. `source` indexes `compounds` (MatchSym, Sym), `matches`
/// (MatchBlock/ProxBlock from patterns), `shared_bias` and `passive` each from 0,
/// `user_groups`, `hier_pairs`, `arrays`, or the net id (ProxNet).
///
/// # Panics
/// When a device id is out of bounds of `canon`, or `classes` is shorter than
/// `hg.net_devices`.
#[allow(clippy::too_many_arguments)] // the plan's signature: one slice per evidence source
#[must_use]
pub fn requirements(
    matches: &[PatternMatch],
    compounds: &[Compound],
    shared_bias: &[Vec<DeviceId>],
    passive: &[Vec<DeviceId>],
    user_groups: &[(u32, Vec<DeviceId>)],
    hier_pairs: &[(DeviceId, DeviceId)],
    arrays: &[Vec<DeviceId>],
    inst: &[Option<u32>],
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
            sym_pair.insert(unordered(a, b));
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
            if !sym_pair.contains(&unordered(a, b)) {
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
    // Stars from `g[0]`; an empty group pushes nothing (`skip(1)` never reads `g[0]`).
    let mut star = |g: &[DeviceId], ty: ReqType, src: usize| g.iter().skip(1).for_each(|&d| push(g[0], d, ty, src));
    for (gi, g) in shared_bias.iter().enumerate() {
        star(g, ReqType::MatchBlock, gi);
    }
    for (gi, g) in passive.iter().enumerate() {
        star(g, ReqType::MatchBlock, gi);
    }
    // Sidecar `GroupBlocks` (EXT-26).
    for (gi, (_, g)) in user_groups.iter().enumerate() {
        star(g, ReqType::ProxBlock, gi);
    }
    for (gi, g) in arrays.iter().enumerate() {
        star(g, ReqType::ProxBlock, gi);
    }
    for (i, &(a, b)) in hier_pairs.iter().enumerate() {
        if !sym_pair.contains(&unordered(a, b)) {
            push(a, b, ReqType::MatchBlock, i);
        }
    }
    let inst_of = |d: DeviceId| inst.get(d.0 as usize).copied().flatten();
    let mut ds: Vec<DeviceId> = Vec::new();
    for (n, devs) in hg.net_devices.iter().enumerate() {
        if matches!(classes[n].class, NetClass::Supply | NetClass::Ground | NetClass::Substrate | NetClass::Clock) {
            continue;
        }
        ds.clear();
        ds.extend_from_slice(devs);
        ds.sort_by_key(|d| (canon[d.0 as usize], d.0));
        ds.dedup();
        if ds.len() <= policy.pn_max_degree {
            // One star per instance (first device of the group by canon), so
            // no edge crosses an instance boundary yet none inside one is lost.
            for (i, &d) in ds.iter().enumerate() {
                if let Some(&h) = ds[..i].iter().find(|&&h| inst_of(h) == inst_of(d)) {
                    push(h, d, ReqType::ProxNet, n);
                }
            }
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
///
/// Node `k` of the result has id `k`; children always precede their parent,
/// and the `Root` is the last node (present even for `n_devices == 0`).
///
/// # Panics
/// When a requirement's device is `>= n_devices` or `canon` is shorter than
/// `n_devices`.
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
        let mut parent: Vec<usize> = (0..n_devices + tree.len()).collect();
        for r in edges.iter().filter(|r| r.ty == ty) {
            let (x, y) = (uf_find(&mut parent, top[r.a.0 as usize]), uf_find(&mut parent, top[r.b.0 as usize]));
            if x != y {
                parent[x.max(y)] = x.min(y);
            }
        }
        let mut alive: Vec<usize> = top.clone();
        alive.sort_unstable();
        alive.dedup();
        let mut classes: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
        for s in alive {
            classes.entry(uf_find(&mut parent, s)).or_default().push(s);
        }
        let mut classes: Vec<Vec<usize>> = classes.into_values().filter(|c| c.len() > 1).collect();
        let sub = |key: &[(u64, usize)], c: &[usize]| (c.iter().map(|&s| key[s].0).min().unwrap(), c.iter().map(|&s| key[s].1).sum::<usize>());
        classes.sort_by_cached_key(|c| (sub(&key, c), c.clone()));
        let kind = match ty {
            ReqType::MatchSym | ReqType::MatchBlock => GroupKind::Matching,
            ReqType::Sym => GroupKind::Symmetry,
            ReqType::ProxBlock | ReqType::ProxNet => GroupKind::Proximity,
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
/// (`Kind{a,b,Kind{..}}`): an id-free view for tests and reports. An empty
/// tree renders as `""`.
///
/// # Panics
/// When a device id is out of bounds of `names`.
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
        let reqs = requirements(&all, c, shared, &[], &[], &[], &[], &[], &hg, &p.net_classes, &canon, &cfg.policy);
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

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::{fet, nets};
    use pnr_core::ids::NetId;
    use pnr_core::netlist::{DeviceKind, Netlist};

    fn req(a: u16, b: u16, ty: ReqType) -> Req {
        Req { a: DeviceId(a), b: DeviceId(b), ty, source: ConstraintId(0) }
    }

    fn classes(cs: &[NetClass]) -> Vec<NetClassification> {
        cs.iter().enumerate().map(|(i, &class)| NetClassification { net: NetId(i as u16), class, c_budget_af: None, max_coupling_af: None }).collect()
    }

    /// Nets 0=a 1=b 2=vss. M0, M1: G on a, D on b. M2: G on b, D on a.
    fn three() -> (Netlist, BipartiteHypergraph, Vec<NetClassification>) {
        let n = DeviceKind::Nmos;
        let nl = Netlist { devices: vec![fet("M0", n, 0, 1, 2, 2, 1_000, 500), fet("M1", n, 0, 1, 2, 2, 1_000, 500), fet("M2", n, 1, 0, 2, 2, 1_000, 500)], nets: nets(&["a", "b", "vss"]), ..Default::default() };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        (nl, hg, classes(&[NetClass::Signal, NetClass::Signal, NetClass::Ground]))
    }

    fn compound(pairs: &[(u16, u16)], selfs: &[u16]) -> Compound {
        Compound {
            id: ConstraintId(0),
            axis: pnr_core::ids::AxisId(0),
            dir: analog::intent::AxisDir::V,
            kind: analog::intent::SymKind::Mirror,
            pairs: pairs.iter().map(|&(a, b)| (DeviceId(a), DeviceId(b))).collect(),
            selfs: selfs.iter().map(|&s| DeviceId(s)).collect(),
            net_pairs: Vec::new(),
            self_nets: Vec::new(),
            set_pairs: Vec::new(),
        }
    }

    #[test]
    fn uf_find_halves_paths() {
        let mut p = vec![0, 0, 1, 2];
        assert_eq!(uf_find(&mut p, 3), 0);
        assert_eq!(uf_find(&mut p, 0), 0);
        // Every node on the walked path now points closer to the root.
        assert!(p[3] <= 1, "{p:?}");
        assert_eq!(unordered(DeviceId(5), DeviceId(2)), (2, 5));
        assert_eq!(unordered(DeviceId(2), DeviceId(5)), (2, 5));
    }

    #[test]
    fn empty_inputs_give_no_requirements() {
        let nl = Netlist::default();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let r = requirements(&[], &[], &[], &[], &[], &[], &[], &[], &hg, &[], &[], &Policy::default());
        assert!(r.is_empty());
    }

    #[test]
    fn edges_are_canonically_oriented_and_self_edges_dropped() {
        let (_, hg, cls) = three();
        let canon = [2, 1, 0];
        // Compound (0, 1): MatchSym and one Sym edge, both read (1, 0) by canon.
        let r = requirements(&[], &[compound(&[(0, 1)], &[])], &[vec![DeviceId(0), DeviceId(0)], vec![]], &[], &[], &[(DeviceId(1), DeviceId(0))], &[], &[], &hg, &cls, &canon, &Policy { pn_max_degree: 0, ..Policy::default() });
        assert_eq!(r, [req(1, 0, ReqType::MatchSym), req(1, 0, ReqType::Sym)]);
    }

    #[test]
    fn stars_and_sources() {
        let (_, hg, cls) = three();
        let canon = [0, 1, 2];
        let g = vec![DeviceId(2), DeviceId(0), DeviceId(1)];
        let pol = Policy { pn_max_degree: 0, ..Policy::default() };
        let r = requirements(&[], &[], &[vec![], g.clone()], &[g.clone()], &[(7, g.clone())], &[(DeviceId(0), DeviceId(1))], &[g], &[], &hg, &cls, &canon, &pol);
        let of = |ty: ReqType| r.iter().filter(|x| x.ty == ty).map(|x| (x.a.0, x.b.0, x.source.0)).collect::<Vec<_>>();
        // shared_bias group 1 and passive group 0 (each from 0), then the hier couple 0.
        assert_eq!(of(ReqType::MatchBlock), [(0, 2, 1), (1, 2, 1), (0, 2, 0), (1, 2, 0), (0, 1, 0)]);
        // user group index 0 (not the entry index 7), then array 0.
        assert_eq!(of(ReqType::ProxBlock), [(0, 2, 0), (1, 2, 0), (0, 2, 0), (1, 2, 0)]);
    }

    #[test]
    fn proxnet_skips_rails_and_wide_nets_and_splits_instances() {
        let (_, hg, cls) = three();
        let canon = [2, 1, 0];
        let pn = |r: &[Req]| r.iter().filter(|x| x.ty == ReqType::ProxNet).map(|x| (x.a.0, x.b.0, x.source.0)).collect::<Vec<_>>();
        let all = requirements(&[], &[], &[], &[], &[], &[], &[], &[], &hg, &cls, &canon, &Policy::default());
        // A star from M2 (lowest canon) on each Signal net; nothing on vss.
        assert_eq!(pn(&all), [(2, 1, 0), (2, 0, 0), (2, 1, 1), (2, 0, 1)]);
        // Degree 3 over a cap of 2: no star.
        assert!(pn(&requirements(&[], &[], &[], &[], &[], &[], &[], &[], &hg, &cls, &canon, &Policy { pn_max_degree: 2, ..Policy::default() })).is_empty());
        // M1 alone in instance 1: no edge reaches it.
        let inst = [Some(0), Some(1), Some(0)];
        assert_eq!(pn(&requirements(&[], &[], &[], &[], &[], &[], &[], &inst, &hg, &cls, &canon, &Policy::default())), [(2, 0, 0), (2, 0, 1)]);
        // A Clock net is skipped like a rail.
        let clk = classes(&[NetClass::Clock, NetClass::Signal, NetClass::Ground]);
        assert_eq!(pn(&requirements(&[], &[], &[], &[], &[], &[], &[], &[], &hg, &clk, &canon, &Policy::default())), [(2, 1, 1), (2, 0, 1)]);
    }

    #[test]
    fn sym_pair_suppresses_duplicate_match_block() {
        let (_, hg, cls) = three();
        let r = requirements(&[], &[compound(&[(0, 1)], &[2])], &[], &[], &[], &[(DeviceId(1), DeviceId(0))], &[], &[], &hg, &cls, &[0, 1, 2], &Policy { pn_max_degree: 0, ..Policy::default() });
        assert!(r.iter().all(|x| x.ty != ReqType::MatchBlock), "{r:?}");
        // Sym is a path over the members in canon order: 0-1, 1-2.
        let sym: Vec<_> = r.iter().filter(|x| x.ty == ReqType::Sym).map(|x| (x.a.0, x.b.0)).collect();
        assert_eq!(sym, [(0, 1), (1, 2)]);
    }

    #[test]
    fn hsmpg_empty_is_a_lone_root() {
        let t = hsmpg(0, &[], &[]);
        assert_eq!(t.len(), 1);
        assert!(t[0].kind == GroupKind::Root && t[0].devices.is_empty() && t[0].children.is_empty());
        assert_eq!(render(&t, &[]), "Root{}");
        assert_eq!(render(&[], &[]), "");
    }

    #[test]
    fn hsmpg_contracts_in_importance_order() {
        let canon = [0, 1, 2, 3];
        let names = ["a", "b", "c", "d"];
        // MatchSym first: a later ProxNet on the same pair adds no node.
        let t = hsmpg(4, &[req(0, 1, ReqType::ProxNet), req(0, 1, ReqType::MatchSym)], &canon);
        assert_eq!(render(&t, &names), "Root{Matching{a,b},c,d}");
        // A chain of MatchBlocks is one class; ProxBlock then joins d to it; Sym groups nothing new.
        let t = hsmpg(4, &[req(0, 1, ReqType::MatchBlock), req(1, 2, ReqType::MatchBlock), req(2, 3, ReqType::ProxBlock), req(0, 3, ReqType::Sym)], &canon);
        assert_eq!(render(&t, &names), "Root{Proximity{Matching{a,b,c},d}}");
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn hsmpg_tree_invariants() {
        let canon = [5, 3, 9, 1, 7, 2];
        let reqs = [req(0, 1, ReqType::MatchSym), req(2, 3, ReqType::MatchBlock), req(1, 2, ReqType::Sym), req(4, 5, ReqType::ProxNet), req(0, 4, ReqType::ProxNet)];
        let t = hsmpg(6, &reqs, &canon);
        assert_eq!(t.last().unwrap().kind, GroupKind::Root);
        let mut seen = [0u8; 6];
        let mut parents = vec![0u8; t.len()];
        for (i, n) in t.iter().enumerate() {
            n.devices.iter().for_each(|d| seen[d.0 as usize] += 1);
            for &c in &n.children {
                assert!((c as usize) < i, "child {c} after parent {i}");
                parents[c as usize] += 1;
            }
            if n.kind != GroupKind::Root {
                assert!(n.devices.len() + n.children.len() >= 2, "{n:?}");
            }
        }
        assert_eq!(seen, [1; 6], "every device in exactly one node");
        assert!(parents[..t.len() - 1].iter().all(|&p| p == 1) && parents[t.len() - 1] == 0);
        // Metamorphic: the input order of the requirements does not matter.
        let mut rev = reqs;
        rev.reverse();
        let names = ["a", "b", "c", "d", "e", "f"];
        assert_eq!(render(&hsmpg(6, &rev, &canon), &names), render(&t, &names));
    }
}
