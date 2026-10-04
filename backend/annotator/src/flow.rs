//! Signal-flow and current-flow ordering (EXT-28): device steps a placer lines
//! up, inputs to outputs (horizontal) and ground to supply (vertical).

use std::collections::VecDeque;

use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind as K;
use pnr_core::BipartiteHypergraph;

use crate::evidence::OpFacts;
use crate::pattern::pin_net;

fn rail(c: NetClass) -> bool {
    matches!(c, NetClass::Supply | NetClass::Ground | NetClass::Substrate)
}

fn sort_steps(steps: &mut [Vec<DeviceId>], canon: &[u64]) {
    steps.iter_mut().for_each(|s| s.sort_by_key(|d| (canon[d.0 as usize], d.0)));
}

/// Signal stages: BFS from input nets over net edges control→drain (FET G→D, BJT B→C) and
/// source→drain (S→D, E→C), rails never traversed. Inputs: nets touched only by G/B terminals,
/// class Signal or Sensitive, ∩ `ports` when `ports` is non-empty. Step k = devices whose D/C net
/// is at level k ≥ 1, sorted by (canon, id).
#[must_use]
pub fn stage_order(hg: &BipartiteHypergraph, classes: &[NetClassification], ports: &[NetId], canon: &[u64]) -> Vec<Vec<DeviceId>> {
    let n = hg.net_names.len();
    let mut control_only = vec![true; n];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        for (t, net) in hg.terminals[d].iter().zip(nets) {
            control_only[net.0 as usize] &= t == "G" || t == "B" && matches!(hg.kinds[d], K::Npn | K::Pnp);
        }
    }
    let mut level: Vec<Option<usize>> = vec![None; n];
    let mut q = VecDeque::new();
    for i in 0..n {
        let input = control_only[i] && !hg.net_devices[i].is_empty() && matches!(classes[i].class, NetClass::Signal | NetClass::Sensitive);
        if input && (ports.is_empty() || ports.contains(&NetId(i as u16))) {
            level[i] = Some(0);
            q.push_back(i);
        }
    }
    let out = |d: usize| pin_net(hg, d as u32, "D").or_else(|| pin_net(hg, d as u32, "C"));
    while let Some(u) = q.pop_front() {
        for &d in &hg.net_devices[u] {
            let d = d.0 as usize;
            let feeds = ["G", "S", "B", "E"].iter().any(|t| pin_net(hg, d as u32, t) == Some(NetId(u as u16)) && (*t != "B" || matches!(hg.kinds[d], K::Npn | K::Pnp)));
            if let Some(v) = out(d).filter(|v| feeds && level[v.0 as usize].is_none() && !rail(classes[v.0 as usize].class)) {
                level[v.0 as usize] = Some(level[u].unwrap() + 1);
                q.push_back(v.0 as usize);
            }
        }
    }
    let mut steps: Vec<Vec<DeviceId>> = Vec::new();
    for d in 0..hg.kinds.len() {
        if let Some(k) = out(d).and_then(|v| level[v.0 as usize]).filter(|&k| k >= 1) {
            if steps.len() < k {
                steps.resize(k, Vec::new());
            }
            steps[k - 1].push(DeviceId(d as u16));
        }
    }
    sort_steps(&mut steps, canon);
    steps
}

/// Per supply-to-ground conduction chain, the steps from ground up and the chain current (µA, 0 without op).
/// Channel edges run low→high (NMOS S→D, PMOS D→S, NPN E→C, PNP C→E); with an op point a device under
/// 1 % of the largest |Id| is idle and dropped (**Philis threshold**). `depth` = longest path from a
/// Ground net (Kahn order; a net on or past a cycle has none), `height` = longest path to a Supply net.
/// Per component of non-rail nets, L = the deepest supply-side device; step k holds the devices of depth
/// k on a longest path (depth + height − 1 = L). In a step with a non-Clock-gated device the Clock-gated
/// ones (precharge switches beside a load) drop out; an all-clocked step (a clocked tail) stays
/// (**Philis policy**). Chains in order of their first device (canon, id).
#[must_use]
pub fn current_paths(hg: &BipartiteHypergraph, op: Option<&OpFacts>, classes: &[NetClassification], canon: &[u64]) -> Vec<(Vec<Vec<DeviceId>>, f64)> {
    let n = hg.net_names.len();
    let class = |x: NetId| classes[x.0 as usize].class;
    let id_ua = |d: usize| op.and_then(|o| o.dev.get(d).copied().flatten()).map(|o| o.id_ua.abs());
    let i_max = (0..hg.kinds.len()).filter_map(id_ua).fold(0.0, f64::max);
    // (device, low net, high net)
    let edges: Vec<(usize, NetId, NetId)> = (0..hg.kinds.len())
        .filter(|&d| id_ua(d).is_none_or(|i| i >= 0.01 * i_max))
        .filter_map(|d| {
            let p = |t: &str| pin_net(hg, d as u32, t);
            let (lo, hi) = match hg.kinds[d] {
                K::Nmos => (p("S")?, p("D")?),
                K::Pmos => (p("D")?, p("S")?),
                K::Npn => (p("E")?, p("C")?),
                K::Pnp => (p("C")?, p("E")?),
                _ => return None,
            };
            (lo != hi && class(lo) != NetClass::Supply && class(hi) != NetClass::Ground).then_some((d, lo, hi))
        })
        .collect();
    // Kahn order over the net DAG.
    let mut indeg = vec![0usize; n];
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(_, lo, hi) in &edges {
        succ[lo.0 as usize].push(hi.0 as usize);
        indeg[hi.0 as usize] += 1;
    }
    let mut order: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut k = 0;
    while k < order.len() {
        for &v in &succ[order[k]] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                order.push(v);
            }
        }
        k += 1;
    }
    let (mut depth, mut height): (Vec<Option<usize>>, Vec<Option<usize>>) = (vec![None; n], vec![None; n]);
    for &u in &order {
        if class(NetId(u as u16)) == NetClass::Ground {
            depth[u] = Some(0);
        }
        if let Some(du) = depth[u] {
            succ[u].iter().for_each(|&v| depth[v] = depth[v].max(Some(du + 1)));
        }
    }
    for &u in order.iter().rev() {
        height[u] = if class(NetId(u as u16)) == NetClass::Supply { Some(0) } else { succ[u].iter().filter_map(|&v| height[v]).max().map(|h| h + 1) };
    }
    // Components: union-find of the non-rail nets a device joins.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for &(_, lo, hi) in &edges {
        if !rail(class(lo)) && !rail(class(hi)) {
            let (a, b) = (find(&mut parent, lo.0 as usize), find(&mut parent, hi.0 as usize));
            parent[a] = b;
        }
    }
    // (component, device, depth, total, high net is Supply)
    let mut devs: Vec<(usize, usize, usize, usize, bool)> = Vec::new();
    for &(d, lo, hi) in &edges {
        let Some(c) = [lo, hi].into_iter().find(|x| !rail(class(*x))) else { continue };
        if let (Some(dl), Some(hh)) = (depth[lo.0 as usize], height[hi.0 as usize]) {
            devs.push((find(&mut parent, c.0 as usize), d, dl + 1, dl + 1 + hh, class(hi) == NetClass::Supply));
        }
    }
    devs.sort_unstable();
    let clocked = |d: usize| ["G", "B"].iter().filter_map(|t| pin_net(hg, d as u32, t)).any(|g| class(g) == NetClass::Clock);
    let mut out = Vec::new();
    for in_c in devs.chunk_by(|x, y| x.0 == y.0) {
        let Some(l) = in_c.iter().filter(|x| x.4).map(|x| x.2).max() else { continue };
        let mut steps: Vec<Vec<DeviceId>> = vec![Vec::new(); l];
        for x in in_c.iter().filter(|x| x.3 == l) {
            steps[x.2 - 1].push(DeviceId(x.1 as u16));
        }
        for s in &mut steps {
            if s.iter().any(|d| !clocked(d.0 as usize)) {
                s.retain(|d| !clocked(d.0 as usize));
            }
        }
        sort_steps(&mut steps, canon);
        let i = steps[0].iter().filter_map(|d| id_ua(d.0 as usize)).sum();
        out.push((steps, i));
    }
    out.sort_by_key(|(s, _)| (canon[s[0][0].0 as usize], s[0][0].0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::DeviceOp;
    use crate::{annotate, pattern, size, AnnotationConfig};
    use pnr_core::Netlist;

    fn setup(nl: &Netlist) -> (BipartiteHypergraph, Vec<NetClassification>, Vec<u64>) {
        let cfg = AnnotationConfig::default();
        let hg = BipartiteHypergraph::from_netlist(nl);
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        let roles = crate::netrole::classify_nets(&hg, &cfg);
        let canon = pattern::canonical_labels(&hg, &drawn, &models, &roles);
        (hg, annotate(nl, &cfg).net_classes, canon)
    }

    fn names(nl: &Netlist, steps: &[Vec<DeviceId>]) -> Vec<Vec<String>> {
        steps
            .iter()
            .map(|s| {
                let mut v: Vec<String> = s.iter().map(|d| nl.devices[d.0 as usize].name.clone()).collect();
                v.sort();
                v
            })
            .collect()
    }

    #[test]
    fn three_stage_stage_order() {
        let nl = crate::tests::three_stage();
        let (hg, classes, canon) = setup(&nl);
        assert_eq!(names(&nl, &stage_order(&hg, &classes, &[], &canon)), [vec!["M1", "M2", "M4", "M5"], vec!["M6", "M7"], vec!["M8", "M9"]]);
    }

    #[test]
    fn op_drops_idle_branch() {
        let nl = crate::tests::three_stage();
        let (hg, classes, canon) = setup(&nl);
        let dop = |i: f64| Some(DeviceOp { id_ua: i, headroom_mv: 100.0, gm_us: 100.0, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us: None });
        let op = OpFacts { dev: nl.devices.iter().map(|d| dop(match d.name.as_str() {
            "M8" | "M9" => 0.001,
            "M3" => 20.0,
            "M6" | "M7" => 40.0,
            _ => 10.0,
        })).collect(), net_mv: vec![None; nl.nets.len()] };
        let (m8, m9) = (DeviceId(7), DeviceId(8));
        let has = |c: &[(Vec<Vec<DeviceId>>, f64)]| c.iter().any(|(s, _)| s.iter().flatten().any(|&d| d == m8 || d == m9));
        assert!(has(&current_paths(&hg, None, &classes, &canon)), "without op the output stage is a chain");
        assert!(!has(&current_paths(&hg, Some(&op), &classes, &canon)));
        // lib.rs weights each V order by its chain current over the largest.
        let ev = crate::Evidence { op: Some(op), ..Default::default() };
        let w: Vec<f32> = crate::annotate_with(&nl, &AnnotationConfig::default(), &ev).intent.order.iter().filter(|o| o.dir == analog::intent::AxisDir::V).map(|o| o.weight).collect();
        assert!(w.contains(&1.0) && w.iter().any(|&x| x < 1.0), "{w:?}");
    }
}
