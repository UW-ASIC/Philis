//! Routing constraints. Structural rules self-extract over the hypergraph; the
//! per-net budgets come from the net's class ([`crate::classify`]).
//!
//! | rule                 | arm    | where                                          |
//! |----------------------|--------|------------------------------------------------|
//! | `Antenna`            | hard   | self-extracted                                 |
//! | `Differential`       | hard   | self-extracted diff-pair net pairs             |
//! | `CrosstalkExclusion` | budget | self-extracted, spacing raised to victim class |
//! | `ParasiticBudget`    | budget | every budgeted net, C budget as drawn length   |
//! | `CouplingBudget`     | budget | every budgeted net, from its class and load    |
//!
//! One batch per kind per arm: `gp::Prices` keys a budget's (λ, ρ) by kind.

use analog::metadata::{NetClass, NetClassification};
use analog::routing::{
    Antenna, CouplingBudget, CrosstalkExclusion, Differential, ParasiticBudget, Shield,
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
    process: &crate::ProcessNumbers,
) -> Requirements<Routes> {
    let mut uf = UnionFind::new(hg.device_count()); // routing rules don't group
    let mut r = Requirements::<Routes>::default();
    // One Antenna per gate net, over the total gate area it drives.
    let mut gate_nm2 = vec![0i64; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if gate_um2[d] > 0.0 {
            // Terminal 0 is G (G,D,S,B). A diode-connected FET hides a wrong index.
            gate_nm2[nets[0].0 as usize] += (f64::from(gate_um2[d]) * 1e6) as i64;
        }
    }
    let antenna: Vec<Antenna> = process.antenna_max_ratio
        .into_iter()
        .flat_map(|ratio| {
            gate_nm2.iter().enumerate().filter(|&(_, &a)| a > 0).map(move |(n, &a)| Antenna {
                net: NetId(n as u16),
                max_ratio_x100: (ratio * 100.0) as i32,
                gate_area_nm2: a,
                margin_pct: 20,
                stack: process.stack,
            })
        })
        .collect();
    r.hard.push(Box::new(antenna));
    let diff: Vec<Differential> = Differential::extract(hg, &mut uf).into_iter().map(|d| Differential { stack: process.stack, ..d }).collect();
    r.hard.push(Box::new(diff));

    let class = |n: NetId| classes[n.0 as usize].class;
    let mut xtalk = CrosstalkExclusion::extract(hg, &mut uf);
    for x in &mut xtalk {
        x.min_spacing_nm = process.route_space_nm * spacing_multiple(class(x.a)).max(spacing_multiple(class(x.b)));
    }
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

    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this.
    let coup: Vec<CouplingBudget> = routed()
        .filter_map(|c| {
            Some(CouplingBudget { net: c.net, max_coupling_af: c.max_coupling_af?, margin_pct: margin_pct(c.class), stack: process.stack })
        })
        .collect();
    r.budget.push(Box::new(coup));

    // Shields only against a real aggressor: with a clock in the design, every
    // routed sensitive net is shielded by the ground net (quiet and low
    // impedance). No clock, no shields — blanket shielding only adds load.
    let has_clock = routed().any(|c| c.class == NetClass::Clock);
    let ground = routed().find(|c| c.class == NetClass::Ground).map(|c| c.net);
    if let (true, Some(reference)) = (has_clock, ground) {
        let shields: Vec<Shield> = routed()
            .filter(|c| c.class == NetClass::Sensitive)
            .map(|c| Shield {
                victim: c.net,
                reference,
                min_coverage_pct: 80,
                // The adjacent track: one routing space, with a spacing of slack.
                max_gap_nm: 2 * process.route_space_nm,
            })
            .collect();
        r.budget.push(Box::new(shields));
    }
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
fn spacing_multiple(class: NetClass) -> i32 {
    match class {
        NetClass::Sensitive => 8,
        NetClass::Clock => 7,
        NetClass::Signal => 3,
        _ => 1,
    }
}
