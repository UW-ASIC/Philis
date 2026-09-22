//! Routing constraints. Structural rules self-extract over the hypergraph; the
//! per-net budgets come from the net's class ([`crate::classify`]).
//!
//! | rule                 | arm    | where                                          |
//! |----------------------|--------|------------------------------------------------|
//! | `Antenna`            | hard   | self-extracted                                 |
//! | `Differential`       | hard   | self-extracted diff-pair net pairs             |
//! | `StraightNet`        | cost   | self-extracted                                 |
//! | `CrosstalkExclusion` | budget | self-extracted, spacing raised to victim class |
//! | `ParasiticBudget`    | budget | every net with ≥2 devices, from its class      |
//! | `CouplingBudget`     | budget | every net with ≥2 devices, from its class      |
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
pub fn routing(hg: &BipartiteHypergraph, classes: &[NetClassification]) -> Requirements<Routes> {
    let mut uf = UnionFind::new(hg.device_count()); // routing rules don't group
    let mut r = Requirements::<Routes>::default();
    r.hard.push(Box::new(Antenna::extract(hg, &mut uf)));
    r.hard.push(Box::new(Differential::extract(hg, &mut uf)));
    r.cost.push(Box::new(StraightNet::extract(hg, &mut uf)));

    let class = |n: NetId| classes[n.0 as usize].class;
    let mut xtalk = CrosstalkExclusion::extract(hg, &mut uf);
    for x in &mut xtalk {
        x.min_spacing_nm = x.min_spacing_nm.max(min_spacing_nm(class(x.a)).max(min_spacing_nm(class(x.b))));
    }
    r.budget.push(Box::new(xtalk));

    let routed = || {
        classes.iter().enumerate().filter(|(n, _)| hg.net_devices[*n].len() >= 2).map(|(n, c)| (NetId(n as u16), c))
    };
    let par: Vec<ParasiticBudget> = routed()
        .map(|(net, c)| {
            ParasiticBudget {
                net,
                // ponytail: 100 mΩ/µm sheet-resistance stand-in until the PDK carries it.
                max_len_nm: c.r_budget_mohm.unwrap_or(1_000_000) / 100 * 1_000,
                margin_pct: margin_pct(c.class),
            }
        })
        .collect();
    r.budget.push(Box::new(par));

    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this.
    let coup: Vec<CouplingBudget> = routed()
        .filter_map(|(net, c)| {
            Some(CouplingBudget { net, max_coupling_af: c.max_coupling_af?, margin_pct: margin_pct(c.class) })
        })
        .collect();
    r.budget.push(Box::new(coup));
    r
}

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
