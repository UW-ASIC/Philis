//! Routing constraints. Structural rules self-extract over the hypergraph; the
//! per-net budgets come from the net's class ([`crate::classify`]).
//!
//! | rule                 | arm    | where                                          |
//! |----------------------|--------|------------------------------------------------|
//! | `Antenna`            | hard   | self-extracted                                 |
//! | `Differential`       | hard   | self-extracted diff-pair net pairs             |
//! | `StraightNet`        | cost   | self-extracted                                 |
//! | `CrosstalkExclusion` | budget | self-extracted, spacing raised to victim class |
//! | `ParasiticBudget`    | budget | every budgeted net, C budget as drawn length   |
//! | `CouplingBudget`     | budget | every budgeted net, from its class and load    |
//!
//! One batch per kind per arm: `gp::Prices` keys a budget's (λ, ρ) by kind.

use analog::metadata::{NetClass, NetClassification};
use analog::routing::{
    Antenna, CouplingBudget, CrosstalkExclusion, Differential, ParasiticBudget, StraightNet,
};
use analog::rule::Rule;
use analog::Requirements;
use pnr_core::{BipartiteHypergraph, NetId, Routes, UnionFind};

/// Assemble the routing [`Requirements`]. `classes` is indexed by net id.
#[must_use]
pub fn routing(
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    gate_um2: &[f32],
    antenna_max_ratio: Option<f32>,
) -> Requirements<Routes> {
    let mut uf = UnionFind::new(hg.device_count()); // routing rules don't group
    let mut r = Requirements::<Routes>::default();
    // One Antenna per gate net, over the total gate area it drives.
    let mut gate_nm2 = vec![0i64; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if gate_um2[d] > 0.0 {
            gate_nm2[nets[1].0 as usize] += (f64::from(gate_um2[d]) * 1e6) as i64;
        }
    }
    let antenna: Vec<Antenna> = antenna_max_ratio
        .into_iter()
        .flat_map(|ratio| {
            gate_nm2.iter().enumerate().filter(|&(_, &a)| a > 0).map(move |(n, &a)| Antenna {
                net: NetId(n as u16),
                max_ratio_x100: (ratio * 100.0) as i32,
                gate_area_nm2: a,
                margin_pct: 20,
            })
        })
        .collect();
    r.hard.push(Box::new(antenna));
    r.hard.push(Box::new(Differential::extract(hg, &mut uf)));
    r.cost.push(Box::new(StraightNet::extract(hg, &mut uf)));

    let class = |n: NetId| classes[n.0 as usize].class;
    let mut xtalk = CrosstalkExclusion::extract(hg, &mut uf);
    for x in &mut xtalk {
        x.min_spacing_nm = x.min_spacing_nm.max(min_spacing_nm(class(x.a)).max(min_spacing_nm(class(x.b))));
    }
    r.budget.push(Box::new(xtalk));

    // Every net a device touches — a one-device net is still routed to its pin.
    let routed = || classes.iter().filter(|c| !hg.net_devices[c.net.0 as usize].is_empty());
    let par: Vec<ParasiticBudget> = routed()
        .filter_map(|c| {
            Some(ParasiticBudget {
                net: c.net,
                max_len_nm: c.c_budget_af? * 1_000 / WIRE_AF_PER_UM,
                margin_pct: margin_pct(c.class),
            })
        })
        .collect();
    r.budget.push(Box::new(par));

    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this.
    let coup: Vec<CouplingBudget> = routed()
        .filter_map(|c| {
            Some(CouplingBudget { net: c.net, max_coupling_af: c.max_coupling_af?, margin_pct: margin_pct(c.class) })
        })
        .collect();
    r.budget.push(Box::new(coup));
    r
}

/// Ground capacitance of a minimum-width lower-metal wire, aF/µm — lowers a
/// net's C budget to the drawn-length cap the router checks.
///
/// ponytail: ≈0.1 fF/µm holds within ~2× across nodes (sky130 met1 ≈ 85); read
/// the per-layer area/fringe from the PDK's `pex` section when routing gets it.
const WIRE_AF_PER_UM: i64 = 100;

/// Safety margin held back from a class's budgets, percent.
fn margin_pct(class: NetClass) -> u8 {
    match class {
        NetClass::Sensitive => 35,
        NetClass::Clock => 30,
        NetClass::Supply | NetClass::Ground => 25,
        _ => 20,
    }
}

/// Minimum run-adjacent spacing a class demands, nm.
fn min_spacing_nm(class: NetClass) -> i32 {
    match class {
        NetClass::Sensitive => 1_200,
        NetClass::Clock => 1_000,
        NetClass::Signal => 400,
        _ => 200,
    }
}
