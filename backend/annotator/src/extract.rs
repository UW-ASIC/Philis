//! Routing constraints. Structural rules self-extract over the hypergraph; the
//! per-net budgets come from the net's class ([`crate::classify`]).
//!
//! | rule                 | arm    | where                                          |
//! |----------------------|--------|------------------------------------------------|
//! | `Antenna`            | hard   | self-extracted                                 |
//! | `Differential`       | budget | from the DiffPair leaves                       |
//! | `CrosstalkExclusion` | budget | from the DiffPair leaves, spacing by class      |
//! | `ParasiticBudget`    | budget | every budgeted net, C budget as drawn length   |
//! | `CouplingBudget`     | budget | every budgeted net, from its class and load;   |
//! |                      |        | own shield excluded, quiet rails weigh 0       |
//!
//! One batch per kind per arm: `gp::Prices` keys a budget's (λ, ρ) by kind.

use analog::metadata::{NetClass, NetClassification};
use analog::routing::{
    Antenna, CouplingBudget, CrosstalkExclusion, Differential, ParasiticBudget, Shield,
};
use analog::Requirements;
use pnr_core::ids::DeviceId;
use pnr_core::{BipartiteHypergraph, NetId, Routes};

/// Assemble the routing [`Requirements`]. `classes` is indexed by net id.
/// `pairs` are the recognised `DiffPair` leaves (2 devices each), the source of
/// `Differential` and `CrosstalkExclusion`.
#[must_use]
pub fn routing(
    hg: &BipartiteHypergraph,
    classes: &[NetClassification],
    gate_um2: &[f32],
    process: &crate::ProcessNumbers,
    pairs: &[(DeviceId, DeviceId)],
    policy: &crate::policy::Policy,
) -> Requirements<Routes> {
    let margin_pct = |c: NetClass| {
        policy.margin_pct[match c {
            NetClass::Sensitive => 0,
            NetClass::Clock => 1,
            NetClass::Supply | NetClass::Ground => 2,
            _ => 3,
        }]
    };
    let spacing_multiple = |c: NetClass| {
        policy.spacing_multiple[match c {
            NetClass::Sensitive => 0,
            NetClass::Clock => 1,
            NetClass::Signal => 2,
            _ => 3,
        }]
    };
    let mut r = Requirements::<Routes>::default();
    // One Antenna per gate net, over the total gate area it drives.
    let mut gate_nm2 = vec![0i64; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if gate_um2[d] > 0.0 {
            if let Some(t) = hg.terminals[d]
                .iter()
                .position(|t| crate::terms::term_role(hg.kinds[d], t) == crate::terms::TermRole::FetGate)
            {
                gate_nm2[nets[t].0 as usize] += (f64::from(gate_um2[d]) * 1e6) as i64;
            }
        }
    }
    let antenna: Vec<Antenna> = process.antenna_max_ratio
        .into_iter()
        .flat_map(|ratio| {
            gate_nm2.iter().enumerate().filter(|&(_, &a)| a > 0).map(move |(n, &a)| Antenna {
                net: NetId(n as u16),
                max_ratio_x100: (ratio * 100.0) as i32,
                gate_area_nm2: a,
                margin_pct: policy.antenna_margin_pct,
                stack: process.stack,
            })
        })
        .collect();
    r.hard.push(Box::new(antenna));

    // Differential and crosstalk-exclusion both come from the recognised DiffPair
    // leaves (EXT-09): no positional-pin or O(N²) device-pair scan.
    let class = |n: NetId| classes[n.0 as usize].class;
    let pin = |d: DeviceId, p: &str| crate::pattern::pin_net(hg, u32::from(d.0), p);
    let mut diff: Vec<Differential> = Vec::new();
    let mut xtalk: Vec<CrosstalkExclusion> = Vec::new();
    for &(a, b) in pairs {
        if let (Some(da), Some(db)) = (pin(a, "D"), pin(b, "D")) {
            if da != db {
                diff.push(Differential { pos: da, neg: db, max_len_delta_pct10: policy.diff_pct10, same_layer_required: true, stack: process.stack });
            }
        }
        for g in [pin(a, "G"), pin(b, "G")].into_iter().flatten() {
            for d in [pin(a, "D"), pin(b, "D")].into_iter().flatten() {
                if g != d {
                    let min_spacing_nm = process.route_space_nm * spacing_multiple(class(g)).max(spacing_multiple(class(d)));
                    xtalk.push(CrosstalkExclusion { a: g, b: d, min_spacing_nm, margin_pct: 25 });
                }
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

    // Shields only against a real aggressor: with a clock in the design, every
    // routed sensitive net is shielded by the ground net (quiet and low
    // impedance). No clock, no shields — blanket shielding only adds load.
    let has_clock = routed().any(|c| c.class == NetClass::Clock);
    let ground = routed().find(|c| c.class == NetClass::Ground).map(|c| c.net);
    let shield_ref = |c: &NetClassification| ground.filter(|_| has_clock && c.class == NetClass::Sensitive);

    // Σ coupling per victim: several minimum-spaced aggressors pass every pairwise
    // crosstalk rule and still blow this. The victim's own shield is the remedy,
    // not an aggressor, and the quiet rails weigh nothing.
    let weights: &'static [f32] =
        Box::leak(CouplingBudget::default_weights(classes, hg.net_names.len()).into_boxed_slice());
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
    if has_clock && ground.is_some() {
        r.budget.push(Box::new(shields));
    }
    r
}

