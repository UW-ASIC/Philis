//! Cells: draw every legal variant of every cell (collapsing matched groups
//! into one cell), seed and escalate the variant assignment, and build the LVS
//! reference from the schematic and what the cells drew (`Macro::drawn`).

use cells::builder::dim;
use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::bjt::Bjt;
use cells::cap_array::CapArray;
use cells::capacitor::Capacitor;
use cells::diode::Diode;
use cells::inductor::Inductor;
use cells::mosfet::Mosfet;
use cells::resistor::Resistor;
use cells::Cell;
use macro_master::Macros;
use pnr_core::{Device, DeviceGroup, DeviceId, DeviceKind, Macro, NetId, Netlist};
use verify::{Checker, Checks, Pdk, RefDeviceIn, RefInput, RefKind};

/// The cell table `gp`/`dp` search, and its map to the netlist's devices.
/// A matched group collapses to one cell, so `n_cells <= n_devices`.
///
/// Invariants: `spaces` and `devices_of` are parallel (one entry per cell, in
/// order of each cell's lowest member device); every space holds at least one
/// alternative; `cell_of` has one entry per netlist device and
/// `devices_of[cell_of[d]]` contains `d`.
pub struct Cells {
    /// Variant space per cell; never empty, so variant 0 always exists.
    pub spaces: Vec<gp::VariantSpace>,
    /// `cell_of[device]` = its cell.
    pub cell_of: Vec<u16>,
    /// Member devices per cell in draw order: `devices_of[c][k]` is the device
    /// the generator's `d{k}:` pin prefix names.
    pub devices_of: Vec<Vec<DeviceId>>,
    /// Matched cells where no variant met the GAP-11 aspect limit (the squarest was kept).
    pub aspect_missed: usize,
}

/// Draw every legal variant of every cell.
///
/// A device with a user macro under its instance name is a one-alternative
/// cell. A multi-device unitization merges into one cell (interleaved stack:
/// same-variant, ratio and common-centroid hold by construction) unless a
/// member is injected, it overlaps an earlier unitization, its MOS members do
/// not share a source net, or every merged pattern draws a short. Declined
/// merges fall back to one cell per device.
///
/// `merge_distinct_gates = false` keeps members on different gate nets (a
/// differential input) as separate cells: the alternative to a common-centroid
/// merge, which splits one member's drain across the row.
///
/// Folds at [`folds`] with no transconductance, the annotator's unitizations
/// as cells, and no net classes or blocks. Every cell gets at least one
/// alternative.
///
/// # Panics
/// If a unitization spans more than one device kind (an annotator bug).
#[must_use]
pub fn enumerate(
    netlist: &Netlist,
    macros: &Macros,
    constraints: &Constraints,
    pdk: &Pdk,
    merge_distinct_gates: bool,
) -> Cells {
    let cells: Vec<(Vec<DeviceId>, bool)> = constraints.unitization.iter().map(|u| (u.devices.clone(), u.route_matching_required)).collect();
    enumerate_folded(netlist, macros, constraints, pdk, merge_distinct_gates, &folds(netlist, pdk, &[], &cells), &[], &[])
}

/// [`enumerate`] at a given fold table ([`folds`]); the flow computes it once
/// so the cells and every LVS reference agree. `net_classes` names the rails:
/// a dummy tie goes to Ground, and a single capacitor binds its bottom plate
/// to the lower-impedance net (CELL-18, [`plate_rank`]). Each of `blocks`
/// (FLOW-11: member devices, the child's geometry) is one cell of one
/// alternative at its lowest member, drawn in the child, never here; a
/// unitization touching a block member is declined.
///
/// `fold` is indexed by device; devices past its end draw unfolded. Block
/// members and unitization members past `netlist.devices` are ignored.
///
/// # Panics
/// If a unitization spans more than one device kind (an annotator bug), or
/// the netlist holds more than `u16::MAX` cells.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn enumerate_folded(
    netlist: &Netlist,
    macros: &Macros,
    constraints: &Constraints,
    pdk: &Pdk,
    merge_distinct_gates: bool,
    fold: &[(u16, i32)],
    net_classes: &[analog::metadata::NetClassification],
    blocks: &[(Vec<DeviceId>, Macro)],
) -> Cells {
    use analog::metadata::NetClass;
    let ground = net_classes.iter().find(|c| c.class == NetClass::Ground).map(|c| c.net);
    let rails: Vec<NetId> = net_classes.iter().filter(|c| matches!(c.class, NetClass::Supply | NetClass::Ground)).map(|c| c.net).collect();
    let sized = with_per_device_sizing(netlist, constraints, fold);
    let n = netlist.devices.len();
    let dev = |d: &DeviceId| &netlist.devices[d.0 as usize];

    // Phase 1: decide and draw the merges.
    let mut unit_of: Vec<Option<usize>> = vec![None; n];
    let mut block_of: Vec<Option<usize>> = vec![None; n];
    for (b, (members, _)) in blocks.iter().enumerate() {
        members.iter().filter(|d| (d.0 as usize) < n).for_each(|d| block_of[d.0 as usize] = Some(b));
    }
    let mut block_emitted = vec![false; blocks.len()];
    let mut merged: Vec<Option<(Vec<DeviceId>, Vec<Macro>)>> = Vec::new();
    let mut aspect_missed = 0usize;
    for u in &sized.unitization {
        let mut members: Vec<DeviceId> = u
            .devices
            .iter()
            .copied()
            .filter(|d| (d.0 as usize) < n)
            .collect();
        if members.len() < 2 {
            continue;
        }
        let kind = dev(&members[0]).kind;
        assert!(
            members.iter().all(|d| dev(d).kind == kind),
            "cellgen: unitization {:?} spans device kinds",
            u.devices
        );
        if members.iter().any(|d| macros.get(&dev(d).name).is_some())
            || members.iter().any(|d| unit_of[d.0 as usize].is_some() || block_of[d.0 as usize].is_some())
        {
            continue;
        }
        // The merged sequence puts every inter-device diffusion boundary on S:
        // differing S nets would be a short DRC cannot see — unless the
        // members are a series stack, drawn as a chain whose junctions are
        // exactly the nets they share (Razavi Fig. 19.12).
        let mut chain: Option<Vec<bool>> = None;
        if is_mos(kind) {
            let s = |d: &DeviceId| terminal(dev(d), "S");
            if s(&members[0]).is_none() || members.iter().any(|d| s(d) != s(&members[0])) {
                let Some(order) = series_order(netlist, &members) else { continue };
                members = order.iter().map(|&(d, _)| d).collect();
                chain = Some(order.iter().map(|&(_, flip)| flip).collect());
            } else {
                let g = |d: &DeviceId| terminal(dev(d), "G");
                if !merge_distinct_gates && members.iter().any(|d| g(d) != g(&members[0])) {
                    continue;
                }
            }
        }
        // A split DAC (CELL-21) is drawn as one array with a top per bank.
        let mut split = None;
        if kind == DeviceKind::Capacitor {
            let nf = |d: DeviceId| u.devices.iter().position(|&x| x == d).map_or(1, |i| u.dev_nf[i]);
            if let Some((order, lsb, top_lsb)) = split_dac(netlist, &rails, &members, nf) {
                members = order;
                split = Some((lsb, top_lsb));
            }
        }
        let group = DeviceGroup {
            devices: members.clone(),
        };
        let mut alternatives = match &chain {
            Some(flips) => {
                let c = Constraints {
                    unitization: vec![Unitization { series_parallel: SeriesParallel::Series, ..u.clone() }],
                    ..Default::default()
                };
                let mut alts = draw_variants(kind, &dev(&members[0]).model, &group, &c, pdk);
                alts.retain(|m| !m.shapes.is_empty());
                for m in &mut alts {
                    flip_members(m, flips);
                }
                alts
            }
            // No legal merged row (`draw_all`'s empty placeholder): declined.
            None => {
                let model = &dev(&members[0]).model;
                let split = split.and_then(|(lsb, top_lsb)| {
                    let ov = verify::pdk::Overlay { pdk, recipe: pdk.recipe("capacitor", model)? };
                    CapArray::split(&group, &sized, &ov, lsb, top_lsb).map(|a| a.draw(&group, &sized, &ov))
                });
                let mut alts = split.map_or_else(|| draw_variants(kind, model, &group, &sized, pdk), |m| vec![m]);
                alts.retain(|m| !m.shapes.is_empty());
                alts
            }
        };
        for m in &mut alternatives {
            bind_pins(m, netlist, &members, ground);
        }
        // Parallel members are one device drawn as several: a multi-finger
        // device alternates S→D by construction, so direction is not a match,
        // and each member's own gate pin is routed like any same-net pin.
        let parallel = chain.is_some() || members.iter().all(|d| dev(d).terminals == dev(&members[0]).terminals);
        alternatives.retain(|m| {
            shared_pads_carry_one_net(m)
                && (parallel || gate_straps_stay_private(m, netlist, &members, pdk) && currents_run_alike(m, members.len()))
        });
        if alternatives.is_empty() {
            continue;
        }
        if let Some(lim) = aspect_limit(u.class, u.kind) {
            if !keep_compact(&mut alternatives, lim) {
                aspect_missed += 1;
            }
        }
        for d in &members {
            unit_of[d.0 as usize] = Some(merged.len());
        }
        merged.push(Some((members, alternatives)));
    }

    // Phase 2: emit cells in order of their first member device.
    let mut spaces: Vec<gp::VariantSpace> = Vec::new();
    let mut cell_of = vec![0u16; n];
    let mut devices_of: Vec<Vec<DeviceId>> = Vec::new();
    for (i, d) in netlist.devices.iter().enumerate() {
        let ci = spaces.len() as u16;
        let (members, alternatives) = if let Some(b) = block_of[i] {
            if std::mem::replace(&mut block_emitted[b], true) {
                continue;
            }
            (blocks[b].0.clone(), vec![blocks[b].1.clone()])
        } else if let Some(ui) = unit_of[i] {
            // Emitted at its lowest member; later members are no-ops.
            let Some(m) = merged[ui].take() else { continue };
            m
        } else if let Some(m) = macros.get(&d.name) {
            // The user's geometry, with its ports bound to this instance's nets.
            let mut m = m.clone();
            for pin in &mut m.pins {
                if let Some(net) = terminal(d, &pin.name) {
                    pin.net = net;
                }
            }
            (vec![DeviceId(i as u16)], vec![m])
        } else {
            let group = DeviceGroup {
                devices: vec![DeviceId(i as u16)],
            };
            let mut alternatives = draw_variants(d.kind, &d.model, &group, &sized, pdk);
            let flip = d.kind == DeviceKind::Capacitor && bottom_on_p(netlist, &rails, d);
            for m in &mut alternatives {
                if flip {
                    swap_plates(m);
                }
                bind_pins(m, netlist, &group.devices, ground);
            }
            (group.devices, alternatives)
        };
        for d in &members {
            cell_of[d.0 as usize] = ci;
        }
        devices_of.push(members);
        spaces.push(gp::VariantSpace {
            alternatives,
        });
    }
    Cells {
        spaces,
        cell_of,
        devices_of,
        aspect_missed,
    }
}

/// A split DAC's members as `Pattern::Split` orders them, with its LSB bit
/// count and whether the bridge's `P` is the LSB top: the bridge is the one
/// member whose `P` and `N` are both other members' `P` (as the annotator's
/// `assemble` finds it), the LSB the bank on a bridge plate holding a unit
/// whose `N` is a rail (the termination, passive.rs's `terminated`), each bank
/// by `(count, !terminated, id)`. `None` unless the members are exactly two
/// banks on the bridge's plates, one of them terminated.
fn split_dac(netlist: &Netlist, rails: &[NetId], members: &[DeviceId], dev_nf: impl Fn(DeviceId) -> u16) -> Option<(Vec<DeviceId>, u8, bool)> {
    let t = |d: DeviceId, name: &str| terminal(&netlist.devices[d.0 as usize], name);
    let bridges: Vec<DeviceId> = members
        .iter()
        .copied()
        .filter(|&d| [t(d, "P"), t(d, "N")].iter().all(|x| x.is_some() && members.iter().any(|&o| o != d && t(o, "P") == *x)))
        .collect();
    let [bridge] = bridges[..] else { return None };
    let terminated = |d: DeviceId| dev_nf(d) == 1 && t(d, "N").is_some_and(|x| rails.contains(&x));
    let bank = |top: Option<NetId>| {
        let mut v: Vec<DeviceId> = members.iter().copied().filter(|&d| d != bridge && t(d, "P") == top).collect();
        v.sort_by_key(|&d| (dev_nf(d), !terminated(d), d.0));
        v
    };
    let (a, b) = (bank(t(bridge, "P")), bank(t(bridge, "N")));
    let a_lsb = a.iter().any(|&d| terminated(d));
    if a.len() + b.len() + 1 != members.len() || a_lsb == b.iter().any(|&d| terminated(d)) {
        return None;
    }
    let (lsb, msb) = if a_lsb { (a, b) } else { (b, a) };
    let l = u8::try_from(lsb.len().checked_sub(1)?).ok()?;
    Some((lsb.into_iter().chain(msb).chain([bridge]).collect(), l, a_lsb))
}

/// The net on `d`'s terminal `name` (`"D"`, `"G"`, `"P"`, …), if it has one.
fn terminal(d: &Device, name: &str) -> Option<NetId> {
    d.terminals.iter().find(|(t, _)| t == name).map(|(_, n)| *n)
}

/// `d`'s parameter `name` as written (nm or PDK units), if present.
fn param(d: &Device, name: &str) -> Option<i64> {
    d.params.iter().find(|(n, _)| n == name).map(|&(_, v)| v)
}

/// Whether `kind` is a MOSFET (drawn by the MOS or FinFET generator).
fn is_mos(kind: DeviceKind) -> bool {
    matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos)
}

/// Splits a generated pin name `d{k}:{T}` into member ordinal `k` and
/// terminal `T`; `None` for a bare name or a non-numeric ordinal.
fn member_pin(name: &str) -> Option<(usize, &str)> {
    let (k, t) = name.strip_prefix('d')?.split_once(':')?;
    Some((k.parse().ok()?, t))
}

/// The net name bound to member `owner`'s pin `t` in `m`; `None` when `m`
/// has no such pin or its net is outside `nets`.
fn pin_net<'n>(m: &Macro, owner: usize, t: &str, nets: &'n [String]) -> Option<&'n String> {
    m.pins.iter().find(|p| member_pin(&p.name) == Some((owner, t))).and_then(|p| nets.get(p.net.0 as usize))
}

/// A plate net's impedance rank (H06-40, H08-25): 0 a rail (Supply/Ground),
/// 2 a net that only reaches MOS gates besides capacitors (high-Z), 1
/// anything else. ponytail: class-only rank; EXT-18's net classes refine it.
fn plate_rank(netlist: &Netlist, rails: &[NetId], net: NetId) -> u8 {
    if rails.contains(&net) {
        return 0;
    }
    let gate_only = netlist.devices.iter().all(|d| {
        d.terminals.iter().filter(|(_, n)| *n == net).all(|(t, _)| d.kind == DeviceKind::Capacitor || matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) && t == "G")
    });
    if gate_only { 2 } else { 1 }
}

/// Whether capacitor `d`'s bottom plate belongs on its `P` net: `P` ranks
/// strictly lower ([`plate_rank`]) than `N`. Equal ranks keep the drawn order.
fn bottom_on_p(netlist: &Netlist, rails: &[NetId], d: &Device) -> bool {
    match (terminal(d, "P"), terminal(d, "N")) {
        (Some(p), Some(n)) => plate_rank(netlist, rails, p) < plate_rank(netlist, rails, n),
        _ => false,
    }
}

/// Put the bottom (drawn `N`, high-parasitic) plate on terminal `P`: rename
/// every `P` pin to `N` and back, and swap the capacitor cards' `P`/`N`
/// nodes, before [`bind_pins`] — so a pin's name stays its terminal
/// everywhere downstream (parasitics, currents) and the LVS card, which
/// lists top plate then bottom, follows the geometry.
fn swap_plates(m: &mut Macro) {
    use pnr_core::{DrawnKind, Node};
    let other = |t: &str| match t {
        "P" => Some("N"),
        "N" => Some("P"),
        _ => None,
    };
    for pin in &mut m.pins {
        let renamed = match pin.name.rsplit_once(':') {
            Some((head, t)) => other(t).map(|o| format!("{head}:{o}")),
            None => other(&pin.name).map(str::to_string),
        };
        if let Some(name) = renamed {
            pin.name = name;
        }
    }
    for d in m.drawn.iter_mut().filter(|d| d.kind == DrawnKind::Capacitor) {
        for n in &mut d.nodes {
            if let Node::Pin(t) = *n {
                if let Some(o) = other(t) {
                    *n = Node::Pin(o);
                }
            }
        }
    }
}

/// `members` as one series stack, in stack order, each with whether it is
/// drawn flipped (its source on the left): consecutive members share one S/D
/// net, no S/D net serves more than two members, one bulk throughout. `None`
/// when they are not a simple path.
fn series_order(netlist: &Netlist, members: &[DeviceId]) -> Option<Vec<(DeviceId, bool)>> {
    let dev = |d: &DeviceId| &netlist.devices[d.0 as usize];
    let bulk = terminal(dev(members.first()?), "B");
    if members.len() < 2 || members.iter().any(|d| terminal(dev(d), "B") != bulk) {
        return None;
    }
    let ends: Vec<(NetId, NetId)> =
        members.iter().map(|d| Some((terminal(dev(d), "D")?, terminal(dev(d), "S")?))).collect::<Option<_>>()?;
    let count = |n: NetId| ends.iter().map(|&(d, s)| usize::from(d == n) + usize::from(s == n)).sum::<usize>();
    if ends.iter().any(|&(d, s)| d == s || count(d) > 2 || count(s) > 2) {
        return None;
    }
    let mut i = (0..ends.len()).find(|&i| count(ends[i].0) == 1 || count(ends[i].1) == 1)?;
    let mut left = if count(ends[i].0) == 1 { ends[i].0 } else { ends[i].1 };
    let mut used = vec![false; ends.len()];
    let mut order = Vec::new();
    loop {
        used[i] = true;
        let (d, s) = ends[i];
        let flip = left == s;
        order.push((members[i], flip));
        let right = if flip { d } else { s };
        match (0..ends.len()).find(|&j| !used[j] && (ends[j].0 == right || ends[j].1 == right)) {
            Some(j) => (left, i) = (right, j),
            None => break,
        }
    }
    (order.len() == members.len()).then_some(order)
}

/// Swap S and D on each flipped member of a drawn chain: its pins, its
/// dummies' near side, its fingers' current direction.
fn flip_members(m: &mut Macro, flips: &[bool]) {
    let flipped = |k: usize| flips.get(k).copied().unwrap_or(false);
    for p in &mut m.pins {
        if let Some((k, t)) = member_pin(&p.name) {
            if flipped(k) && (t == "S" || t == "D") {
                p.name = format!("d{k}:{}", if t == "S" { "D" } else { "S" });
            }
        }
    }
    for d in &mut m.dummies {
        if flipped(usize::from(d.owner)) {
            d.edge = match d.edge { "S" => "D", "D" => "S", e => e };
        }
    }
    for u in &mut m.units {
        if flipped(usize::from(u.owner)) {
            u.phi = (-u.phi.0, -u.phi.1);
        }
    }
}

/// Every member's mean signed S→D direction is the same (Hastings' Φ): equal
/// magnitude is not enough, since a mirrored pair (`D A S B D`) runs its two
/// currents opposite ways and picks up orientation-dependent mismatch.
/// Compared exactly as `Σφ_a·n_b == Σφ_b·n_a`. A macro without units has
/// nothing to compare and passes.
fn currents_run_alike(m: &Macro, n_members: usize) -> bool {
    analog::matching::moments::phi_equal_all(&m.units, n_members)
}

/// Pins drawn on one pad carry one net — otherwise a shared diffusion region
/// shorts two nets (invisible to DRC, fatal to LVS).
fn shared_pads_carry_one_net(m: &Macro) -> bool {
    m.pins.iter().enumerate().all(|(i, a)| {
        m.pins[i + 1..]
            .iter()
            .all(|b| a.at != b.at || a.net == b.net)
    })
}

/// Members on different gate nets sit on different poly islands (else the
/// merge draws a short). Members on one gate net may sit on separate islands:
/// each has its own gate pin, which the router joins like any same-net pins.
/// Exact: it walks the drawn poly, so a generator that routes one device's
/// gates out the other side of the row passes. Non-MOS cells (no `d{i}:G`
/// pins) pass.
fn gate_straps_stay_private(m: &Macro, netlist: &Netlist, members: &[DeviceId], pdk: &Pdk) -> bool {
    use pnr_core::Process;
    let Some(poly) = pdk.layer("poly") else { return true };
    let island = cells::mosfet::gate_islands(m, poly, members.len());
    let gate = |d: &DeviceId| terminal(&netlist.devices[d.0 as usize], "G");
    members.iter().enumerate().all(|(a, da)| {
        members.iter().enumerate().skip(a + 1).all(|(b, db)| match (island[a], island[b]) {
            (Some(ia), Some(ib)) => gate(da) == gate(db) || ia != ib,
            _ => true,
        })
    })
}

#[cfg(test)]
/// Per-member `(min, max)` x of its `d{N}:S`/`d{N}:D` pins.
fn region_spans(m: &Macro, members: usize) -> Vec<Option<(i32, i32)>> {
    let mut span: Vec<Option<(i32, i32)>> = vec![None; members];
    for pin in &m.pins {
        let Some((n, t)) = member_pin(&pin.name) else { continue };
        if t != "S" && t != "D" {
            continue;
        }
        let Some(slot) = span.get_mut(n) else { continue };
        let e = slot.get_or_insert((pin.at.x, pin.at.x));
        e.0 = e.0.min(pin.at.x);
        e.1 = e.1.max(pin.at.x);
    }
    span
}

/// Starting variant per cell, chosen by measuring each alternative in
/// isolation (see [`price`], [`seed_of`]). Ties break on index, so the seed is deterministic.
/// Also per cell the alternatives [`escalate`] may visit ([`keep`] over
/// `(DRC+ERC, bbox w, bbox h)` from the same prices; `matched[i]` keeps all).
/// `ranked[i]` (missing = false) marks a cell whose generator lists its
/// alternatives best-matching first (GAP-18).
///
/// Cost: one density-stripped DRC+ERC run per alternative of every cell.
///
/// # Panics
/// If `pdk`'s own deck no longer parses into a [`Checker`].
#[must_use]
pub fn seed_assignment(variants: &[gp::VariantSpace], matched: &[bool], ranked: &[bool], pdk: &Pdk) -> (Vec<u16>, Vec<Vec<u16>>) {
    let mut checker = Checker::new(pdk, true).expect("a loaded Pdk re-parses its own deck");
    variants
        .iter()
        .enumerate()
        .map(|(i, space)| {
            let prices: Vec<(usize, i64)> = space.alternatives.iter().map(|m| price(m, &mut checker)).collect();
            let seed = seed_of(&prices, ranked.get(i).copied().unwrap_or(false));
            let cost: Vec<(usize, i32, i32)> = prices.iter().zip(&space.alternatives).map(|(p, m)| (p.0, m.bbox.w, m.bbox.h)).collect();
            (seed as u16, keep(&cost, seed, matched.get(i).copied().unwrap_or(true)))
        })
        .unzip()
}

/// Seed index from `(DRC+ERC, HPWL)` prices: the cheapest, ties → index; a `ranked` cell (its generator lists
/// alternatives best-first, GAP-18) seeds at 0. Not "first DRC+ERC-minimal": [`price`] passes no ports, so each
/// conductor is an x.22 finding and the count tracks shape count (dac4: 178/176/178), which would seed the
/// fewest-shapes variant, not the best-matching one; every CapArray variant is proven DRC/ERC-clean
/// (`cap_array::tests::every_variant_is_drc_and_erc_clean`).
fn seed_of(prices: &[(usize, i64)], ranked: bool) -> usize {
    if ranked {
        return 0;
    }
    prices.iter().enumerate().min_by(|a, b| a.1.cmp(b.1)).map_or(0, |(v, _)| v)
}

/// The alternatives of one cell worth escalating to, ascending: all when
/// `matched` (a matched cell's alternatives differ in pattern, which its own
/// rules price, not the bbox); else every one not dominated in `(DRC+ERC,
/// w, h)` — some other is no worse in all three and better in one (BAL2-39,
/// dominated-variant pruning). `seed` is always kept, so the start digit is
/// in its own set even when a smaller, higher-HPWL alternative dominates it.
/// ponytail: O(n²) per cell, n ≤ a few dozen alternatives.
fn keep(cost: &[(usize, i32, i32)], seed: usize, matched: bool) -> Vec<u16> {
    let dominated = |v: usize| {
        let c = cost[v];
        cost.iter().any(|u| u.0 <= c.0 && u.1 <= c.1 && u.2 <= c.2 && *u != c)
    };
    (0..cost.len()).filter(|&v| matched || v == seed || !dominated(v)).map(|v| v as u16).collect()
}

#[cfg(test)]
thread_local!(static PRICE_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) });

/// [`price`] calls on this thread so far (tests: pricing happens once per run).
#[cfg(test)]
pub(crate) fn price_calls() -> u32 {
    PRICE_CALLS.with(std::cell::Cell::get)
}

/// `(DRC+ERC findings, pin HPWL)`, compared lexicographically. DRC runs
/// density-stripped (fill rules are chip-level); a macro the engine cannot
/// load prices as maximally illegal.
fn price(m: &Macro, checker: &mut Checker) -> (usize, i64) {
    #[cfg(test)]
    PRICE_CALLS.with(|c| c.set(c.get() + 1));
    let geom = match checker.run(
        &m.shapes,
        &[],
        Checks {
            drc: true,
            erc: true,
            lvs: false,
            pex: false,
        },
    ) {
        Ok(_) => checker.outputs().violations.len(),
        Err(_) => usize::MAX,
    };
    (geom, gr::group_hpwl(std::slice::from_ref(m)))
}

/// The geometry an assignment selects, one cloned macro per cell. A missing
/// entry means variant 0; entries past `variants` are ignored.
///
/// # Panics
/// If an entry names a variant its cell does not have (including any variant
/// of an empty space).
#[must_use]
pub fn realize(variants: &[gp::VariantSpace], assignment: &[u16]) -> Vec<Macro> {
    variants
        .iter()
        .enumerate()
        .map(|(i, space)| {
            let v = usize::from(assignment.get(i).copied().unwrap_or(0));
            assert!(
                v < space.alternatives.len(),
                "cell {i} names variant {v} of {}",
                space.alternatives.len()
            );
            space.alternatives[v].clone()
        })
        .collect()
}

/// The next joint assignment to try, or `None` when the space is exhausted.
///
/// A mixed-radix odometer over the cells, digit `i` ranging over
/// `allowed[i]` (ascending = best-matching first for a ranked cell; [`seed_assignment`]), fastest digit = the cell
/// whose alternatives move pins the most ([`pin_spread`]): never repeats,
/// always terminates, and changes pin geometry first. A cell with an empty
/// or one-entry `allowed` row is a fixed digit. Each `allowed` row must be
/// ascending; a missing `current` entry reads as 0.
#[must_use]
pub fn escalate(variants: &[gp::VariantSpace], allowed: &[Vec<u16>], current: &[u16]) -> Option<Vec<u16>> {
    let spread: Vec<usize> = variants.iter().map(pin_spread).collect();
    let mut order: Vec<usize> = (0..variants.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(spread[i]), i));
    let mut next: Vec<u16> = (0..variants.len())
        .map(|i| current.get(i).copied().unwrap_or(0))
        .collect();
    for &i in &order {
        let row = allowed.get(i).map_or(&[][..], Vec::as_slice);
        if let Some(&v) = row.iter().find(|&&v| v > next[i]) {
            next[i] = v;
            return Some(next);
        }
        if let Some(&v) = row.first() {
            next[i] = v; // carry
        }
    }
    None
}

/// Next assignment on an infeasible stall: the most-blamed cell with an
/// untried higher `allowed` entry moves first (ties: larger pin spread, then
/// lower index); else the first untried assignment in [`escalate`] order,
/// walked from the odometer's origin (so a blamed jump strands nothing
/// behind it). Never returns `current` or an assignment in `tried`; `None`
/// when the space is exhausted.
#[must_use]
pub fn escalate_blamed(variants: &[gp::VariantSpace], allowed: &[Vec<u16>], current: &[u16], blame: &[u32], tried: &std::collections::BTreeSet<Vec<u16>>) -> Option<Vec<u16>> {
    use std::cmp::Reverse;
    let n = variants.len();
    let cur: Vec<u16> = (0..n).map(|i| current.get(i).copied().unwrap_or(0)).collect();
    let fresh = |a: &Vec<u16>| *a != cur && !tried.contains(a);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (Reverse(blame.get(i).copied().unwrap_or(0)), Reverse(pin_spread(&variants[i])), i));
    for &i in &order {
        for &a in allowed.get(i).map_or(&[][..], Vec::as_slice).iter().filter(|&&a| a > cur[i]) {
            let mut next = cur.clone();
            next[i] = a;
            if fresh(&next) {
                return Some(next);
            }
        }
    }
    // ponytail: a linear odometer walk per call, O(space); the space is the
    // pruned product of `allowed`, small after GAP-16.
    let mut a: Vec<u16> = (0..n).map(|i| allowed.get(i).and_then(|r| r.first()).copied().unwrap_or(cur[i])).collect();
    loop {
        if fresh(&a) {
            return Some(a);
        }
        a = escalate(variants, allowed, &a)?;
    }
}

/// Distinct pin arrangements (bbox-relative, order-free) among a cell's
/// alternatives.
fn pin_spread(space: &gp::VariantSpace) -> usize {
    let mut sigs: Vec<Vec<(i32, i32, i32, i32)>> = space
        .alternatives
        .iter()
        .map(|m| {
            let mut p: Vec<_> = m
                .pins
                .iter()
                .map(|p| (p.at.x - m.bbox.x, p.at.y - m.bbox.y, p.at.w, p.at.h))
                .collect();
            p.sort_unstable();
            p
        })
        .collect();
    sigs.sort();
    sigs.dedup();
    sigs.len()
}

/// The generator-facing constraints: the annotator's unitizations split by
/// device kind (a unitization draws every member as its one `device_type`, and
/// opposite polarities never match anyway), plus a 1-device unitization for
/// every device no unitization covers (banks, bipolar arrays and identical
/// parallel MOS are the annotator's matched-set unitizations, EXT-19): a MOS gets `nf·m` fingers of
/// `W_total/nf` ([`pnr_core::MosSize`]), a bipolar `m` units, anything else
/// its written `w` and `nf`/`m` count.
fn with_per_device_sizing(netlist: &Netlist, annot: &Constraints, fold: &[(u16, i32)]) -> Constraints {
    let kind_of = |d: &DeviceId| netlist.devices.get(d.0 as usize).map(|dev| dev.kind);
    let mut unitization: Vec<Unitization> = Vec::new();
    for u in &annot.unitization {
        let mut kinds: Vec<DeviceKind> = Vec::new();
        for k in u.devices.iter().filter_map(kind_of) {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
        }
        if kinds.len() <= 1 {
            let mut u = u.clone();
            if let Some(&k) = kinds.first() {
                u.device_type = k; // trust the netlist over the annotator's label
            }
            unitization.push(u);
            continue;
        }
        for k in kinds {
            let keep: Vec<usize> = (0..u.devices.len())
                .filter(|&i| kind_of(&u.devices[i]) == Some(k))
                .collect();
            unitization.push(Unitization {
                devices: keep.iter().map(|&i| u.devices[i]).collect(),
                device_type: k,
                dev_nf: keep
                    .iter()
                    .filter_map(|&i| u.dev_nf.get(i).copied())
                    .collect(),
                target_ratio: keep
                    .iter()
                    .filter_map(|&i| u.target_ratio.get(i).copied())
                    .collect(),
                ..u.clone()
            });
        }
    }
    let mut covered = vec![false; netlist.devices.len()];
    for d in annot.unitization.iter().flat_map(|u| &u.devices) {
        if let Some(c) = covered.get_mut(d.0 as usize) {
            *c = true;
        }
    }
    // Banks, bipolar arrays and identical parallel MOS are the annotator's
    // matched-set Unitizations (EXT-19 moved cellgen's recognizers there).
    for (i, dev) in netlist
        .devices
        .iter()
        .enumerate()
        .filter(|(i, _)| !covered[*i])
    {
        let param = |k: &str| param(dev, k).unwrap_or(0);
        unitization.push(Unitization {
            devices: vec![DeviceId(i as u16)],
            device_type: dev.kind,
            // A MOS draws `nf·m` fingers of `W_total/nf`, a bipolar `m` units
            // (the reference's card count), anything else its `nf`/`m` count
            // at the written `w`.
            dev_nf: vec![match (dev.mos_size(), dev.kind) {
                (Some(s), _) => i64::from(s.fingers()),
                (None, DeviceKind::Npn | DeviceKind::Pnp) => i64::from(multiplier(dev)),
                (None, _) => param("nf").max(param("m")),
            }
            .clamp(1, i64::from(u16::MAX)) as u16],
            target_ratio: vec![1],
            unit_w: dev.mos_size().map_or(param("w"), |s| s.w_finger_nm()).clamp(0, i64::from(i32::MAX)) as i32,
            unit_l: param("l").clamp(0, i64::from(i32::MAX)) as i32,
            // SPICE `m` is parallel: a resistor draws `m` strings. A
            // capacitor's composition has no reader yet (CELL-08).
            series_parallel: match dev.kind {
                DeviceKind::Capacitor => SeriesParallel::Series,
                _ => SeriesParallel::Parallel,
            },
            dummy_required: false,
            route_matching_required: false,
            class: None, kind: None, series: Vec::new(), style: None,
        });
    }
    // Fold every MOS unitization by its width class's factor: `k`× the
    // fingers at `W/k`, ratios kept (every member scales alike).
    for u in &mut unitization {
        let Some(&(k, w)) = u.devices.first().and_then(|d| fold.get(d.0 as usize)) else { continue };
        if k > 1 && is_mos(u.device_type) {
            u.dev_nf = u.dev_nf.iter().map(|&n| n.saturating_mul(k)).collect();
            u.unit_w = w;
        }
    }
    Constraints {
        unitization,
        ..Default::default()
    }
}

/// Share of the deck's point-to-point R limit one finger's poly may take:
/// between the farthest attach points of a drawn row (fingers, end dummies,
/// their stubs and straps) the checker sees up to ~1.6× a finger (measured on
/// generic_finfet's 1 kΩ rule: rows fail from `R□·W_f/L` ≈ 0.63·limit).
const P2P_SHARE: f32 = 0.55;

/// Per device, `(k, W_f/k)`: its schematic fingers (`nf·m`, each
/// `W_f = W_total/nf` wide, [`pnr_core::MosSize`]) are drawn as `k`× as many
/// at width `W_f/k` (grid-snapped). One `k` per class: the MOS of equal
/// (kind, W_f, L) in the first `cells` entry (a unitization's devices, drawn as
/// one cell) holding the device, else the device alone, so matched devices
/// (one entry by construction) fold alike and an unrelated same-size device
/// folds for its own row (FLOW-16). A class of several non-parallel, non-stack
/// members keeps only the `k` whose per-member counts have a centroid-exact
/// row ([`cells::mosfet::cc_row_exists`], route-matched as the entry's flag)
/// when any `k` does; else the choice below stands. `k` brings the class's
/// row (all its fingers side by side, at the generator's pitch) closest to
/// square, fingers within the deck's `min_finger_width`/`max_finger_width`
/// (no cap when the deck gives none) and its point-to-point resistance limit
/// along one finger. Parity
/// first: a series stack wants every member odd (a chain row), any other
/// class of several non-parallel devices every member even (an ABBA row).
///
/// `gm_us` (per device, µS; empty or `None` = unknown) sets a floor on the
/// finger count: the distributed gate resistance of `N` fingers contacted at
/// one end, `R□·W/(3·L·N²)`, stays under a fifth of `1/gm` (Razavi Ex. 19.1:
/// gate noise a fifth of the channel's), so `N ≥ √(5·gm·R□·W / 3L)`, `W`
/// read as the per-finger `W_f`. ponytail: known inconsistency, the `N²` form
/// holds for the total `W`; with `W_f` the `N` parallel fingers give
/// `R□·W_f/(3·L·N)`, linear in `N`. The floor is kept as plan-08 FLOW-01
/// specifies it; the gate-R floor's owner settles which form is meant.
/// Non-MOS devices and devices without W/L get `(1, W)`.
#[must_use]
pub fn folds(netlist: &Netlist, pdk: &Pdk, gm_us: &[Option<f64>], cells: &[(Vec<DeviceId>, bool)]) -> Vec<(u16, i32)> {
    use pnr_core::Process;
    let nm = |d: &Device, k: &str| param(d, k).map_or(0, |v| v.clamp(0, i64::from(i32::MAX)) as i32);
    // A MOS's width is its finger's, `W_total/nf`; anything else its written `w`.
    let w_of = |d: &Device| d.mos_size().map_or(nm(d, "w"), |s| s.w_finger_nm().min(i64::from(i32::MAX)) as i32);
    let mos = |d: &Device| is_mos(d.kind);
    // A channel below the deck's shortest legal one is drawn as asked (DRC
    // reports it), never resized behind the netlist's back: said here, by name.
    for d in netlist.devices.iter().filter(|d| mos(d)) {
        let (l_legal, w_legal) = pdk.min_channel(d.kind == DeviceKind::Pmos, &d.model);
        let (w, l) = (w_of(d), nm(d, "l"));
        if (l > 0 && l < l_legal) || (w > 0 && w < w_legal) {
            eprintln!("cellgen: {} asks W/L {w}/{l} nm, below the deck's shortest legal channel {w_legal}/{l_legal} nm; drawn as asked", d.name);
        }
    }
    let grid = pdk.grid().max(1);
    // A finger holds an enclosed S/D contact (and the deck's own minimum).
    let w_min = pdk
        .rule("min_finger_width", 0)
        .max(dim(pdk, "contact") + 2 * pdk.rule("diff_encloses_licon", 0).max(pdk.enclosure("diff", "licon").unwrap_or(0)))
        .max(pdk.width("diff").unwrap_or(0));
    let w_max = pdk.rule("max_finger_width", 0);
    // A finger also keeps its far diffusion corner within the deck's
    // latch-up tap reach of the strip above it (CELL-13), per gate length,
    // never below the smallest finger. A fin deck draws `FinFet`, which the
    // planar probe does not describe: no reach cap there.
    let fin = pnr_core::Process::layer(pdk, "fin").is_some();
    let mut tap_cap = std::collections::BTreeMap::new();
    // The deck's point-to-point R limit bounds a finger too: a finger's poly,
    // `R□·W_f/L`, within [`P2P_SHARE`] of it.
    let poly_sq = pdk.sheet_ohm("poly").filter(|&sq| sq > 0.0);
    let p2p = pdk.p2p_max_ohm().zip(poly_sq);
    let poly_sq = poly_sq.map_or(0.0, f64::from);
    let mut out: Vec<(u16, i32)> = netlist.devices.iter().map(|d| (1, w_of(d))).collect();
    let mut done = vec![false; netlist.devices.len()];
    for (i, d) in netlist.devices.iter().enumerate() {
        let (w, l) = (w_of(d), nm(d, "l"));
        if done[i] || !mos(d) || w <= 0 || l <= 0 {
            continue;
        }
        let entry = cells.iter().find(|(c, _)| c.contains(&DeviceId(i as u16)));
        let route = entry.is_some_and(|e| e.1);
        let class: Vec<usize> = match entry {
            Some((members, _)) => members
                .iter()
                .map(|m| m.0 as usize)
                .filter(|&j| {
                    let Some(e) = netlist.devices.get(j) else { return false };
                    !done[j] && e.kind == d.kind && w_of(e) == w && nm(e, "l") == l
                })
                .collect(),
            None => vec![i],
        };
        let fingers: Vec<u32> = class.iter().map(|&j| netlist.devices[j].mos_size().map_or(1, |s| s.fingers())).collect();
        let row: u32 = fingers.iter().sum();
        let parallel = class.iter().all(|&j| netlist.devices[j].terminals == d.terminals);
        // A series stack needs odd fingers per member (each starts on D and
        // ends on S); any other multi-device class wants even (ABBA).
        let ids: Vec<DeviceId> = class.iter().map(|&j| DeviceId(j as u16)).collect();
        let s_of = |j: usize| terminal(&netlist.devices[j], "S");
        let stack = class.len() > 1 && class.iter().any(|&j| s_of(j) != s_of(class[0])) && series_order(netlist, &ids).is_some();
        let (_, pitch) = cells::mosfet::sd_and_pitch(pdk, l);
        let w_max = match (w_max, *tap_cap.entry(l).or_insert_with(|| if fin { i32::MAX } else { cells::mosfet::max_finger_for_taps(pdk, l).max(w_min) })) {
            (0, t) if t < i32::MAX => t,
            (m, t) => m.min(t),
        };
        // Smallest k whose finger count keeps every member's gate R below
        // 1/(5·gm).
        let k_gate = class
            .iter()
            .zip(&fingers)
            .filter_map(|(&j, &f)| {
                let gm = gm_us.get(j).copied().flatten()? * 1e-6;
                let n = (5.0 * gm * poly_sq * f64::from(w) / (3.0 * f64::from(l))).sqrt();
                Some((n / f64::from(f)).ceil() as u32)
            })
            .max()
            .unwrap_or(1)
            .max(1);
        let snap = |v: i32| (v + grid / 2) / grid * grid;
        // (wrong member finger parity, |log aspect|) — lexicographic.
        let score = |k: u32| {
            let fw = snap(w / k as i32);
            let bad_parity = if stack {
                fingers.iter().any(|&f| (f * k) % 2 == 0)
            } else {
                !parallel && class.len() > 1 && fingers.iter().any(|&f| (f * k) % 2 == 1)
            };
            let aspect = (f64::from(row * k) * f64::from(pitch) / f64::from(fw)).ln().abs();
            (bad_parity, aspect)
        };
        let mut ks: Vec<u32> = (1u32..=64).filter(|&k| k == 1 || (snap(w / k as i32) >= w_min && row * k <= u32::from(u16::MAX))).collect();
        if class.len() > 1 && !stack && !parallel {
            let cc = |k: &u32| cells::mosfet::cc_row_exists(&fingers.iter().map(|&f| (f * k).min(u32::from(u16::MAX)) as u16).collect::<Vec<_>>(), route);
            if ks.iter().any(cc) {
                ks.retain(cc);
            }
        }
        let best = ks
            .into_iter()
            .min_by(|&a, &b| {
                let (sa, sb) = (score(a), score(b));
                // A finger over the cap, or too few fingers for the gate R,
                // loses to any fold that respects both.
                let over = |k: u32| {
                    let fw = snap(w / k as i32);
                    (w_max > 0 && fw > w_max) || k < k_gate || p2p.is_some_and(|(r, sq)| sq * fw as f32 / l as f32 > P2P_SHARE * r)
                };
                (over(a), sa.0).cmp(&(over(b), sb.0)).then(sa.1.total_cmp(&sb.1))
            })
            .unwrap_or(1);
        for &j in &class {
            done[j] = true;
            out[j] = (best as u16, if best == 1 { w } else { snap(w / best as i32) });
        }
    }
    out
}

/// `d`'s SPICE `m` (parallel count), clamped to `1..=u16::MAX`; 1 when absent.
fn multiplier(d: &Device) -> u16 {
    param(d, "m").map_or(1, |v| v.clamp(1, i64::from(u16::MAX)) as u16)
}

/// The longest-to-shortest side a matched member's subarray may have (GAP-11, Hastings rule 9, H13-46
/// L42504–42516, PDF 714): a current-matched set drawn long and thin sees the gradient along its long side.
/// `class` is `unwrap_or(Moderate)` (C16). Current: Minimal 10 / Moderate 3 / Exceptional 1.5; Voltage:
/// 3 / 1.5 / 1.5 (the 1.5 for "square or nearly square" is **[policy]**). Ratio sets and an unknown kind:
/// `None` (the rule is for transistors; R/C ratio sets have their own pattern rules).
fn aspect_limit(class: Option<analog::intent::MatchClass>, kind: Option<analog::intent::MatchKind>) -> Option<f64> {
    use analog::intent::{MatchClass as C, MatchKind as K};
    let c = class.unwrap_or(C::Moderate);
    match kind? {
        K::Current => Some(match c { C::Minimal => 10.0, C::Moderate => 3.0, C::Exceptional => 1.5 }),
        K::Voltage => Some(if c == C::Minimal { 3.0 } else { 1.5 }),
        K::Ratio => None,
    }
}

/// The worst member's subarray aspect (≥ 1) on the macro's unit grid: pitch = bbox extent / distinct unit
/// centres per axis, member footprint = its own units' span + one pitch (H13-46: only the subarray counts).
/// `1.0` with no units or a zero extent.
fn member_aspect(m: &Macro) -> f64 {
    let pitch = |key: fn(&pnr_core::units::Unit) -> i32, extent: i32| {
        let mut v: Vec<i32> = m.units.iter().map(key).collect();
        v.sort_unstable();
        v.dedup();
        extent as f64 / v.len().max(1) as f64
    };
    let (px, py) = (pitch(|u| u.x, m.bbox.w), pitch(|u| u.y, m.bbox.h));
    let mut worst = 1.0f64;
    for o in m.units.iter().map(|u| u.owner).collect::<std::collections::BTreeSet<_>>() {
        let us = m.units.iter().filter(|u| u.owner == o);
        let (x0, x1) = us.clone().fold((i32::MAX, i32::MIN), |(a, b), u| (a.min(u.x), b.max(u.x)));
        let (y0, y1) = us.fold((i32::MAX, i32::MIN), |(a, b), u| (a.min(u.y), b.max(u.y)));
        let (w, h) = ((x1 - x0) as f64 + px, (y1 - y0) as f64 + py);
        if w > 0.0 && h > 0.0 {
            worst = worst.max(w.max(h) / w.min(h));
        }
    }
    worst
}

/// Keep the variants within `limit` (GAP-11) and return `true`; when none is, keep only the squarest (first
/// on ties) and return `false` (the caller reports it).
fn keep_compact(alts: &mut Vec<Macro>, limit: f64) -> bool {
    if alts.iter().any(|m| member_aspect(m) <= limit) {
        alts.retain(|m| member_aspect(m) <= limit);
        return true;
    }
    let best = (0..alts.len()).min_by(|&a, &b| member_aspect(&alts[a]).total_cmp(&member_aspect(&alts[b])));
    if let Some(i) = best {
        alts.swap(0, i);
        alts.truncate(1);
    }
    false
}

/// `model`'s recogniser markers (GAP-07) that the planar generator does not already draw, as `gate_marker{i}`
/// roles over `pdk`; `None` when there are none (plain `nfet_01v8`). Forbidden layers are ignored: the generator never
/// draws them.
fn mos_overlay<'a>(pdk: &'a Pdk, model: &str) -> Option<verify::pdk::Overlay<'a>> {
    let (need, _) = pdk.model_markers(model)?;
    let drawn: Vec<_> = ["diff", "tap", "poly", "licon", "li", "mcon", "met1", "nwell", "nsdm", "psdm", "npc"]
        .iter()
        .filter_map(|r| pnr_core::Process::layer(pdk, r))
        .collect();
    let name = |l: &pnr_core::LayerId| pdk.layers.iter().find(|(_, id)| id == l).map(|(n, _)| n.clone());
    let mut layers: Vec<(String, String)> =
        need.iter().filter(|l| !drawn.contains(l)).filter_map(name).enumerate().map(|(i, n)| (format!("gate_marker{i}"), n)).collect();
    if layers.is_empty() {
        return None;
    }
    // `npc` is a recipe-controlled role on an overlay: the gate contacts keep the base deck's.
    layers.extend(pnr_core::Process::layer(pdk, "npc").as_ref().and_then(name).map(|n| ("npc".to_string(), n)));
    Some(verify::pdk::Overlay { pdk, recipe: verify::pdk::Recipe { model: model.into(), layers, rules: vec![] } })
}

/// Every variant of `group`, a `kind` of schematic `model`. A resistor is
/// drawn to its model's recipe ([`Pdk::recipe`]): the construction the
/// deck's recogniser for that model expects. Never empty.
fn draw_variants(kind: DeviceKind, model: &str, group: &DeviceGroup, c: &Constraints, pdk: &Pdk) -> Vec<Macro> {
    match kind {
        // A fin process draws its transistors from fins.
        DeviceKind::Nmos | DeviceKind::Pmos if pnr_core::Process::layer(pdk, "fin").is_some() => draw_all::<cells::finfet::FinFet>(group, c, pdk),
        // A variant whose diffusion lies beyond the deck's latch-up tap reach
        // is dropped (CELL-13); none left keeps the empty placeholder.
        DeviceKind::Nmos | DeviceKind::Pmos => {
            // A flavoured model (lvt, hvt) draws its recogniser's markers.
            let ov = mos_overlay(pdk, model);
            let mut v = draw_all::<Mosfet>(group, c, ov.as_ref().map_or(pdk as &dyn pnr_core::Process, |o| o));
            v.retain(|m| cells::mosfet::taps_in_reach(m, pdk));
            if v.is_empty() {
                v.push(Macro::default());
            }
            v
        }
        DeviceKind::Resistor => match pdk.recipe("resistor", model) {
            Some(recipe) => draw_all::<Resistor>(group, c, &verify::pdk::Overlay { pdk, recipe }),
            None => draw_all::<Resistor>(group, c, pdk),
        },
        // A binary bank is drawn only as a common-centroid array: a merged
        // plate per bit implements no pattern (ARR-01/02). The model's
        // capacitor recipe picks the stack (sky130 MIM, else MOM; CELL-08).
        DeviceKind::Capacitor => {
            let ov = pdk.recipe("capacitor", model).map(|recipe| verify::pdk::Overlay { pdk, recipe });
            let p: &dyn pnr_core::Process = ov.as_ref().map_or(pdk as &dyn pnr_core::Process, |o| o);
            match CapArray::enumerate(group, c, p) {
                v if v.is_empty() => draw_all::<Capacitor>(group, c, p),
                v => v.iter().map(|a| a.draw(group, c, p)).collect(),
            }
        }
        DeviceKind::Diode => draw_all::<Diode>(group, c, pdk),
        DeviceKind::Npn | DeviceKind::Pnp => match pdk.recipe("bjt", model) {
            Some(recipe) => draw_all::<Bjt>(group, c, &verify::pdk::Overlay { pdk, recipe }),
            None => draw_all::<Bjt>(group, c, pdk),
        },
        DeviceKind::Inductor => draw_all::<Inductor>(group, c, pdk),
    }
}

/// Every variant generator `G` enumerates for `group`, drawn, in
/// `Cell::enumerate` order. Never empty: an empty enumeration yields one empty
/// macro (no shapes) so `variant == 0` always names something; callers that
/// must not keep it filter on `shapes.is_empty()`.
fn draw_all<G: Cell>(group: &DeviceGroup, c: &Constraints, pdk: &dyn pnr_core::Process) -> Vec<Macro> {
    let mut drawn: Vec<Macro> = G::enumerate(group, c, pdk).iter().map(|v| v.draw(group, c, pdk)).collect();
    if drawn.is_empty() {
        drawn.push(Macro::default());
    }
    drawn
}

/// The LVS reference from the schematic, terminals as net names in SPICE card
/// order (MOS `D G S B`, two-terminal `P N`), BJTs in the order the extractor
/// reports them, `E B C`: its emitter/collector symmetry does not hold for a
/// diode-connected device (B = C), which the `C B E` order then fails.
///
/// A sized MOS goes in as `nf·m` cards of per-finger `W_total/nf` and `l` (SI
/// metres, [`pnr_core::MosSize`]):
/// the extractor measures one device per channel and a parametrised device never
/// parallel-merges. Capacitors go in as one card per unit (`max(nf, m)`),
/// inductors as one card, both `P N`: a MOM (every capacitor generator here
/// draws one) and an inductor have no deck recogniser, and `verify` matches a
/// capacitor card only to a deck row its model names (never a model-less one
/// to the first MIM), so it counts them LVS-unverified (`lvs-coverage/…`)
/// rather than this module dropping them.
/// `ports` is left empty for the caller to fill with the labels it actually
/// places.
///
/// Each MOS is folded as [`folds`] says (the same factor the cells use).
///
/// ponytail: a ratioed unitization (unit W below a member's W) desyncs this
/// expansion from the drawn layout.
///
/// `fold`: the flow's [`folds`] table, which the cards follow; `None` for
/// geometry drawn at the schematic's own fingers (the manual path).
///
/// `skip`: devices whose cards come from [`drawn_cards`] instead.
///
/// A terminal that is absent reads as the empty net name `""`.
///
/// # Panics
/// If a device terminal names a net outside `netlist.nets`.
#[must_use]
pub fn reference(netlist: &Netlist, fold: Option<&[(u16, i32)]>, skip: &[DeviceId]) -> RefInput {
    let mut devices: Vec<RefDeviceIn> = Vec::new();
    for (i, dev) in netlist.devices.iter().enumerate() {
        if skip.contains(&DeviceId(i as u16)) {
            continue;
        }
        // Devices past the table (inserted later: antenna diodes) are unfolded.
        let (k, fw) = fold.and_then(|f| f.get(i)).copied().unwrap_or((1, 0));
        let (kind, pins): (RefKind, &[&str]) = match dev.kind {
            DeviceKind::Nmos => (RefKind::Nmos, &["D", "G", "S", "B"]),
            DeviceKind::Pmos => (RefKind::Pmos, &["D", "G", "S", "B"]),
            DeviceKind::Resistor => (RefKind::Resistor, &["P", "N"]),
            DeviceKind::Diode => (RefKind::Diode, &["P", "N"]),
            DeviceKind::Npn => (RefKind::Npn, &BJT_PINS),
            DeviceKind::Pnp => (RefKind::Pnp, &BJT_PINS),
            DeviceKind::Capacitor => (RefKind::Capacitor, &["P", "N"]),
            DeviceKind::Inductor => (RefKind::Inductor, &["P", "N"]),
        };
        let terminals: Vec<String> = pins
            .iter()
            .map(|p| {
                terminal(dev, p).map_or(String::new(), |n| netlist.nets[n.0 as usize].name.clone())
            })
            .collect();
        let (fingers, params) = if is_mos(dev.kind) {
            // No size (missing or non-positive w/l): one card without params,
            // so LVS reports the device rather than a made-up size.
            match dev.mos_size() {
                Some(s) => {
                    let w = if k > 1 { i64::from(fw) } else { s.w_finger_nm() };
                    (i64::from(s.fingers()) * i64::from(k), vec![("w".to_string(), w as f64 * 1e-9), ("l".to_string(), s.l_nm as f64 * 1e-9)])
                }
                None => (1, Vec::new()),
            }
        } else if matches!(dev.kind, DeviceKind::Npn | DeviceKind::Pnp) {
            // One card per drawn unit (ratios are unit counts: `m`).
            (i64::from(multiplier(dev)), Vec::new())
        } else if dev.kind == DeviceKind::Capacitor {
            // One card per drawn unit (dac4's XC4 `m=8` is 8 unit caps).
            let nf = param(dev, "nf").unwrap_or(1);
            (nf.max(i64::from(multiplier(dev))).clamp(1, i64::from(u16::MAX)), Vec::new())
        } else {
            (1, Vec::new())
        };
        for _ in 0..fingers {
            devices.push(RefDeviceIn {
                kind,
                model: (!dev.model.is_empty()).then(|| dev.model.clone()),
                terminals: terminals.clone(),
                params: params.clone(),
            });
        }
    }
    RefInput {
        devices,
        ports: Vec::new(),
        external_ports: None,
    }
}

/// BJT terminals in LVS card order, collector first (GPurify `lvs/graph.rs`
/// reads card position 0 as Collector): [`reference`]'s and `Macro::drawn`'s.
pub(crate) const BJT_PINS: [&str; 3] = ["C", "B", "E"];

/// LVS cards for everything the placed cells drew as `Macro::drawn`, and the
/// schematic devices they replace (sorted, distinct), whose own cards
/// [`reference`] must skip. `Node::Pin(t)` is the net of the macro's bound pin
/// `d{owner}:{t}`; `Node::Internal(k)` is the non-port net `~{cell}.{owner}.{k}`,
/// `cell` the macro's index in `placed`. A pin that is missing reads as the
/// non-port net `~{cell}.{owner}.no-{t}`, which nothing drawn reaches: the
/// card stays, so the device is an LVS mismatch where the deck extracts its
/// kind and an `lvs-coverage/` unit where it does not, never unaccounted.
///
/// No params: GPurify measures `w`/`l` for MOS only (and `area`, which the
/// reference never interns), and a param on one side only is a mismatch.
pub fn drawn_cards(placed: &[Macro], nets: &[String], schematic: &Netlist, pdk: &Pdk) -> (Vec<RefDeviceIn>, Vec<DeviceId>) {
    use pnr_core::{DrawnKind, Node};
    let mut cards = Vec::new();
    let mut replaced: Vec<DeviceId> = Vec::new();
    for (cell, m) in placed.iter().enumerate() {
        for d in &m.drawn {
            let Some(id) = d.device else { continue };
            // A stale id (its schematic device since removed, e.g. lifted
            // out for re-adoption) replaces nothing: the card would self-
            // reference the macro's own geometry and always match.
            let Some(schem_dev) = schematic.devices.get(id.0 as usize) else { continue };
            replaced.push(id);
            let (kind, recipe) = match d.kind {
                DrawnKind::Resistor => (RefKind::Resistor, "resistor"),
                DrawnKind::Capacitor => (RefKind::Capacitor, "capacitor"),
                DrawnKind::Diode => (RefKind::Diode, "diode"),
                DrawnKind::Npn => (RefKind::Npn, "bjt"),
                DrawnKind::Pnp => (RefKind::Pnp, "bjt"),
            };
            let node = |n: &Node| match *n {
                Node::Unused => None,
                Node::Pin(t) => Some(pin_net(m, usize::from(d.owner), t, nets).cloned().unwrap_or_else(|| format!("~{cell}.{}.no-{t}", d.owner))),
                Node::Internal(k) => Some(format!("~{cell}.{}.{k}", d.owner)),
            };
            let terminals: Vec<String> = d.nodes.iter().filter_map(node).collect();
            let model = schem_dev.model.as_str();
            let model = pdk.recipe(recipe, model).map(|r| r.model).filter(|m| !m.is_empty()).or_else(|| (!model.is_empty()).then(|| model.to_string()));
            cards.push(RefDeviceIn { kind, model, terminals, params: Vec::new() });
        }
    }
    replaced.sort_unstable_by_key(|d| d.0);
    replaced.dedup();
    (cards, replaced)
}

/// One LVS card per dummy gate a placed macro drew (`Macro::dummies`): nets
/// read off the owner's own bound pins (`d{k}:S|D` near side, `d{k}:B` gate,
/// far side and body), params mirroring the schematic's MOS cards (a param
/// on one side only is an LVS mismatch). A dummy whose pins are missing is
/// skipped: its absence then shows as an extracted extra device.
pub fn dummy_cards(placed: &[Macro], nets: &[String], schematic_cards: &[RefDeviceIn]) -> Vec<RefDeviceIn> {
    let sized = schematic_cards.iter().any(|c| matches!(c.kind, RefKind::Nmos | RefKind::Pmos) && !c.params.is_empty());
    let mut out = Vec::new();
    for m in placed {
        let net = |owner: u8, t: &str| pin_net(m, usize::from(owner), t, nets).cloned();
        for d in &m.dummies {
            let (Some(near), Some(bulk)) = (net(d.owner, d.edge), net(d.owner, "B")) else { continue };
            out.push(RefDeviceIn {
                kind: if d.pmos { RefKind::Pmos } else { RefKind::Nmos },
                model: None,
                terminals: vec![near, bulk.clone(), bulk.clone(), bulk],
                params: if sized { vec![("w".into(), f64::from(d.w) * 1e-9), ("l".into(), f64::from(d.l) * 1e-9)] } else { Vec::new() },
            });
        }
    }
    out
}

/// Rebind a drawn macro's synthetic pin nets to the schematic's. A pin named
/// `d{k}:T` is terminal `T` of `members[k]`; a bare `T` is member 0. A bare
/// `GND` (a generator's dummy tie) goes to `ground` (the classified ground
/// net), else to member 0's `N`, else `S`.
fn bind_pins(m: &mut Macro, netlist: &Netlist, members: &[DeviceId], ground: Option<NetId>) {
    for d in &mut m.drawn {
        d.device = members.get(usize::from(d.owner)).copied();
    }
    for pin in &mut m.pins {
        let (ordinal, term) = match pin.name.split_once(':') {
            Some((d, t)) => (
                d.strip_prefix('d')
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0),
                t,
            ),
            None => (0, pin.name.as_str()),
        };
        let dev = members
            .get(ordinal)
            .and_then(|d| netlist.devices.get(d.0 as usize));
        debug_assert!(
            dev.is_some(),
            "pin {:?} names member {ordinal} of {}",
            pin.name,
            members.len()
        );
        let net = if pin.name == "GND" {
            ground.or_else(|| dev.and_then(|d| terminal(d, "N").or_else(|| terminal(d, "S"))))
        } else {
            dev.and_then(|d| terminal(d, term))
        };
        if let Some(net) = net {
            pin.net = net;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Net, Pin, Rect, Shape};

    /// The sky130 deck the benchmarks use, compiled in: a sidecar that fails
    /// validation fails the test, never skips it.
    fn pdk() -> Pdk {
        Pdk::builtin("sky130").expect("sky130 loads")
    }

    /// A synthetic matched macro: `(owner, x)` units on one row at y = 500, bbox `w`×1000 (GAP-11).
    fn row(units: &[(u8, i32)], w: i32) -> Macro {
        let units = units
            .iter()
            .map(|&(owner, x)| pnr_core::Unit { owner, x, y: 500, weight: 1, phi: (1, 0), sa: 0, sb: 0 })
            .collect();
        Macro { bbox: Rect { x: 0, y: 0, w, h: 1000 }, units, ..Default::default() }
    }

    #[test]
    fn current_mod_filters_4_to_1() {
        use analog::intent::{MatchClass, MatchKind};
        let a = row(&[(0, 500), (0, 1500), (0, 2500), (0, 3500)], 4000);
        let b = row(&[(0, 500), (0, 1500)], 2000);
        assert_eq!(member_aspect(&a), 4.0);
        assert_eq!(member_aspect(&b), 2.0);
        let lim = aspect_limit(Some(MatchClass::Moderate), Some(MatchKind::Current)).unwrap();
        assert_eq!(lim, 3.0);
        let mut v = vec![a, b];
        assert!(keep_compact(&mut v, lim));
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].bbox.w, 2000);
    }

    #[test]
    fn last_variant_is_kept_and_reported() {
        use analog::intent::{MatchClass, MatchKind};
        let a = row(&[(0, 500), (0, 1500), (0, 2500), (0, 3500)], 4000);
        let b = row(&[(0, 500), (0, 1500)], 2000);
        let mut v = vec![a.clone()];
        assert!(!keep_compact(&mut v, aspect_limit(Some(MatchClass::Exceptional), Some(MatchKind::Voltage)).unwrap()));
        assert_eq!(v.len(), 1);
        // With none in reach the squarest survives, wherever it sits.
        let mut v = vec![a, b];
        assert!(!keep_compact(&mut v, 1.5));
        assert_eq!((v.len(), v[0].bbox.w), (1, 2000));
        assert_eq!(aspect_limit(Some(MatchClass::Moderate), Some(MatchKind::Ratio)), None);
        assert_eq!(aspect_limit(None, Some(MatchKind::Current)), Some(3.0));
        assert_eq!(aspect_limit(Some(MatchClass::Exceptional), None), None);
    }

    #[test]
    fn an_interdigitated_member_spans_the_row() {
        // ABAB: each member's subarray is its own span (2 pitches + 1 = 3000 × 1000), not one unit (1.0).
        let m = row(&[(0, 500), (1, 1500), (0, 2500), (1, 3500)], 4000);
        assert_eq!(member_aspect(&m), 3.0);
    }

    /// Two MOSFETs on three nets — the smallest circuit with a real variant space.
    fn two_devices() -> Netlist {
        let nets = ["vdd", "vss", "g", "out"]
            .iter()
            .map(|n| Net {
                name: (*n).to_string(),
            })
            .collect();
        let dev = |name: &str, kind, d, g, s| Device {
            name: name.to_string(),
            kind, model: String::new(),
            terminals: vec![
                ("D".to_string(), NetId(d)),
                ("G".to_string(), NetId(g)),
                ("S".to_string(), NetId(s)),
                ("B".to_string(), NetId(s)),
            ],
            params: vec![
                ("w".to_string(), 1000),
                ("l".to_string(), 210),
                ("nf".to_string(), 2),
            ],
        };
        Netlist {
            devices: vec![
                dev("M1", DeviceKind::Nmos, 3, 2, 1),
                dev("M2", DeviceKind::Pmos, 3, 2, 0),
            ],
            nets,
            ..Default::default()
        }
    }

    /// CELL-18: a rail ranks 0, a drain net 1, a gate-only net 2 (a
    /// capacitor on it keeps it so); the bottom plate moves to `P` only when
    /// `P` ranks strictly lower; `swap_plates` renames only `P`/`N` pins and
    /// capacitor nodes.
    #[test]
    fn plate_rank_orders_rail_signal_gate() {
        use pnr_core::{Drawn, DrawnKind, Node};
        let mut nl = two_devices();
        let rails = [NetId(0), NetId(1)];
        assert_eq!([0, 3, 2].map(|n| plate_rank(&nl, &rails, NetId(n))), [0, 1, 2]);
        let cap = |p: u16, n: u16| Device {
            name: "C1".into(),
            kind: DeviceKind::Capacitor, model: String::new(),
            terminals: vec![("P".into(), NetId(p)), ("N".into(), NetId(n))],
            params: Vec::new(),
        };
        nl.devices.push(cap(2, 3));
        assert_eq!(plate_rank(&nl, &rails, NetId(2)), 2, "a cap on a gate net keeps it high-Z");
        assert!(!bottom_on_p(&nl, &rails, &cap(0, 1)), "equal rank (two rails): no swap");
        assert!(!bottom_on_p(&nl, &rails, &cap(3, 3)), "equal rank (one signal net): no swap");
        assert!(!bottom_on_p(&nl, &rails, &cap(2, 0)), "rail on N: no swap");
        assert!(bottom_on_p(&nl, &rails, &cap(0, 2)), "rail on P: swap");
        let mut m = Macro::default();
        for name in ["d0:P", "N", "GND"] {
            m.pins.push(Pin { name: name.into(), net: NetId(0), layer: LayerId(1), at: Rect { x: 0, y: 0, w: 1, h: 1 } });
        }
        let card = |kind| Drawn { owner: 0, device: None, kind, nodes: [Node::Pin("P"), Node::Pin("N"), Node::Unused], w: 1, l: 1 };
        m.drawn = vec![card(DrawnKind::Capacitor), card(DrawnKind::Diode)];
        swap_plates(&mut m);
        assert_eq!(m.pins.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["d0:N", "P", "GND"]);
        assert_eq!(m.drawn.iter().map(|d| d.nodes).collect::<Vec<_>>(), [[Node::Pin("N"), Node::Pin("P"), Node::Unused], [Node::Pin("P"), Node::Pin("N"), Node::Unused]]);
    }

    /// A hand-built space, so `realize`/`escalate` can be exercised with no PDK.
    fn space(n: usize) -> gp::VariantSpace {
        gp::VariantSpace {
            alternatives: (0..n)
                .map(|k| {
                    let w = 100 * (k as i32 + 1);
                    Macro {
                        shapes: vec![Shape {
                            layer: LayerId(1),
                            rect: Rect {
                                x: 0,
                                y: 0,
                                w,
                                h: 100,
                            },
                        }],
                        // Distinct pin geometry per alternative, so `pin_spread`
                        // reports the real count rather than collapsing them.
                        pins: vec![Pin {
                            name: "G".to_string(),
                            net: NetId(0),
                            at: Rect {
                                x: w - 10,
                                y: 0,
                                w: 10,
                                h: 10,
                            },
                            layer: LayerId(1),
                        }],
                        bbox: Rect {
                            x: 0,
                            y: 0,
                            w,
                            h: 100,
                        },
                        units: Vec::new(),
                        dummies: Vec::new(),
                        ..Default::default()
                    }
                })
                .collect(),
        }
    }

    /// D7's round trip: an assignment names an alternative, and `realize` must hand
    /// back *that* one. Getting this wrong is silent — every downstream stage would
    /// score geometry the placer did not choose.
    #[test]
    fn realize_selects_the_named_alternative() {
        let spaces = vec![space(3), space(2)];
        for (a, b) in [(0u16, 0u16), (2, 1), (1, 0)] {
            let got = realize(&spaces, &[a, b]);
            assert_eq!(got.len(), 2);
            assert_eq!(got[0].bbox, spaces[0].alternatives[a as usize].bbox);
            assert_eq!(got[1].bbox, spaces[1].alternatives[b as usize].bbox);
        }
        // A short table reads as variant 0 — `Layout::variant`'s own rule, and what
        // a placer that has not written it yet leaves behind.
        assert_eq!(
            realize(&spaces, &[])[0].bbox,
            spaces[0].alternatives[0].bbox
        );
    }

    /// Every cell must offer at least one alternative or `realize` shifts cell
    /// indices, and an injected macro must be a *one*-element space — a variant the
    /// user drew is not ours to reconsider.
    #[test]
    fn enumerate_gives_every_cell_a_space_and_pins_injected_ones() {
        let pdk = pdk();
        let netlist = two_devices();

        let auto = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk, true);
        assert_eq!(
            auto.spaces.len(),
            netlist.devices.len(),
            "one cell per device"
        );
        for (i, s) in auto.spaces.iter().enumerate() {
            assert!(
                !s.alternatives.is_empty(),
                "cell {i} has an empty variant space"
            );
            // Pins bound to the netlist's nets, not the generator's synthetic ids:
            // unbound, routing shorts every gate together.
            let nets: Vec<u16> = s.alternatives[0].pins.iter().map(|p| p.net.0).collect();
            assert!(
                nets.iter().all(|&n| usize::from(n) < netlist.nets.len()),
                "cell {i} kept synthetic pin nets: {nets:?}"
            );
        }

        let mut injected = Macros::default();
        injected.register(
            "M1",
            Macro {
                shapes: vec![Shape {
                    layer: LayerId(1),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 9,
                        h: 9,
                    },
                }],
                pins: vec![Pin {
                    name: "G".to_string(),
                    net: NetId(0),
                    at: Rect {
                        x: 0,
                        y: 0,
                        w: 9,
                        h: 9,
                    },
                    layer: LayerId(1),
                }],
                bbox: Rect {
                    x: 0,
                    y: 0,
                    w: 9,
                    h: 9,
                },
                units: Vec::new(),
                dummies: Vec::new(),
                ..Default::default()
            },
        );
        let mixed = enumerate(&netlist, &injected, &Constraints::default(), &pdk, true);
        assert_eq!(
            mixed.spaces[0].alternatives.len(),
            1,
            "injected macro must be a single-alternative space"
        );
        assert_eq!(
            mixed.spaces[0].alternatives[0].bbox.w, 9,
            "injected geometry was redrawn"
        );
        // Its port was remapped onto M1's own `G` net (2), not left at the registry's 0.
        assert_eq!(mixed.spaces[0].alternatives[0].pins[0].net, NetId(2));
        assert_eq!(
            mixed.spaces[1].alternatives.len(),
            auto.spaces[1].alternatives.len(),
            "M2 still auto-drawn"
        );
    }

    /// The regression anchor for the collapse: a netlist with **no matched groups**
    /// must come out exactly as before — identity map, one cell per device, and
    /// spaces byte-identical to the per-device draw path.
    #[test]
    fn no_matched_groups_is_the_identity_map_with_identical_spaces() {
        let pdk = pdk();
        let netlist = two_devices();
        let cells = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk, true);

        assert_eq!(cells.cell_of, vec![0, 1], "identity map");
        assert_eq!(
            cells.devices_of,
            vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            "singleton members"
        );

        // Byte-identical to drawing each device as its own group, with no collapse
        // machinery (no pad filter, no re-ordering) in the way.
        let sized = with_per_device_sizing(&netlist, &Constraints::default(), &folds(&netlist, &pdk, &[], &[]));
        for (i, dev) in netlist.devices.iter().enumerate() {
            let group = DeviceGroup {
                devices: vec![DeviceId(i as u16)],
            };
            let mut expect = draw_variants(dev.kind, &dev.model, &group, &sized, &pdk);
            for m in &mut expect {
                bind_pins(m, &netlist, &group.devices, None);
            }
            assert_eq!(
                cells.spaces[i].alternatives, expect,
                "cell {i} drifted from the per-device path"
            );
        }
    }

    /// CELL-13: a deck whose tap reach no variant meets leaves only the empty
    /// placeholder; sky130's own reach keeps every drawn variant.
    #[test]
    fn a_tap_out_of_reach_drops_the_variant() {
        let mut pdk = pdk();
        let mut netlist = two_devices();
        netlist.devices[0].params[0].1 = 5000;
        let sized = with_per_device_sizing(&netlist, &Constraints::default(), &folds(&netlist, &pdk, &[], &[]));
        let group = DeviceGroup { devices: vec![DeviceId(0)] };
        let all = draw_variants(DeviceKind::Nmos, "", &group, &sized, &pdk);
        assert!(!all.is_empty() && all.iter().all(|m| !m.shapes.is_empty()), "sky130 keeps every variant");
        pdk.rules.push(("tie_max_dist_nm".into(), 1000));
        assert_eq!(draw_variants(DeviceKind::Nmos, "", &group, &sized, &pdk), vec![Macro::default()]);
    }

    /// ihp_sg13g2 allows 100 um fingers but 20 um tap reach. A 40-finger nmos
    /// of 60 um fingers folds squarest at 2 x 30 um, under the raw cap: the
    /// reach cap must fold it further.
    #[test]
    fn folds_caps_the_finger_at_the_tap_reach() {
        use pnr_core::Process;
        let pdk = Pdk::builtin("ihp_sg13g2").expect("ihp_sg13g2 loads");
        let l = pdk.min_channel(false, "").0;
        let mut netlist = two_devices();
        netlist.devices.truncate(1);
        netlist.devices[0].params = vec![("w".into(), 2_400_000), ("l".into(), i64::from(l)), ("nf".into(), 40)];
        let cap = cells::mosfet::max_finger_for_taps(&pdk, l);
        assert!(cap < pdk.rule("max_finger_width", 0), "reach {cap} does not bind on ihp_sg13g2");
        let (k, fw) = folds(&netlist, &pdk, &[], &[])[0];
        assert!(k > 1 && fw <= cap, "folded to {k} x {fw} nm, reach cap {cap} nm");
    }

    /// The property the outer loop depends on: escalation must make progress. A
    /// repeat would spin the loop while `variant_escalations` reports otherwise.
    #[test]
    fn escalate_never_repeats_and_exhausts() {
        // 3 × 2 × 1 = 6 joint assignments; the single-alternative cell is a fixed
        // digit and must not stall the odometer.
        let spaces = vec![space(3), space(2), space(1)];
        let all = full(&spaces);
        let mut seen = vec![vec![0u16, 0, 0]];
        let mut cur = vec![0u16, 0, 0];
        while let Some(next) = escalate(&spaces, &all, &cur) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            seen.push(next.clone());
            cur = next;
            assert!(seen.len() <= 6, "escalate exceeded the space size");
        }
        assert_eq!(seen.len(), 6, "escalate stopped before covering the space");
        assert!(
            escalate(&spaces, &all, &cur).is_none(),
            "exhaustion must stay exhausted"
        );
        // Nothing to escalate is exhaustion, not a panic.
        assert!(escalate(&[], &[], &[]).is_none());
    }

    /// FLOW-08: blame picks the cell; equal pin spread, so blame alone decides.
    #[test]
    fn escalate_blamed_moves_the_most_blamed_cell_first() {
        let spaces = vec![space(2), space(2), space(2)];
        let all = full(&spaces);
        let tried = std::collections::BTreeSet::from([vec![0u16, 0, 0]]);
        assert_eq!(escalate_blamed(&spaces, &all, &[0, 0, 0], &[0, 5, 1], &tried), Some(vec![0, 1, 0]));
    }

    /// FLOW-08: from the origin, every other assignment once, then `None`.
    #[test]
    fn escalate_blamed_never_repeats() {
        let spaces = vec![space(2), space(2), space(2)];
        let all = full(&spaces);
        let mut tried = std::collections::BTreeSet::from([vec![0u16, 0, 0]]);
        let (mut cur, mut out) = (vec![0u16, 0, 0], Vec::new());
        while let Some(next) = escalate_blamed(&spaces, &all, &cur, &[0, 0, 0], &tried) {
            assert!(tried.insert(next.clone()), "repeated {next:?}");
            out.push(next.clone());
            cur = next;
            assert!(out.len() <= 7, "exceeded the space");
        }
        assert_eq!(out.len(), 7);
        assert!(!out.contains(&vec![0, 0, 0]));
    }

    /// Every alternative allowed: the unpruned odometer.
    fn full(spaces: &[gp::VariantSpace]) -> Vec<Vec<u16>> {
        spaces.iter().map(|s| (0..s.alternatives.len() as u16).collect()).collect()
    }

    /// GAP-16: the pruned odometer visits exactly the product of the allowed
    /// sets, never a digit outside them, and stays exhausted.
    #[test]
    fn escalate_still_terminates() {
        let spaces = vec![space(3), space(2), space(1)];
        let allowed = vec![vec![0u16, 2], vec![0, 1], vec![0]];
        let mut seen = vec![vec![0u16, 0, 0]];
        let mut cur = vec![0u16, 0, 0];
        while let Some(next) = escalate(&spaces, &allowed, &cur) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            assert!(next.iter().zip(&allowed).all(|(v, a)| a.contains(v)), "{next:?} outside {allowed:?}");
            seen.push(next.clone());
            cur = next;
            assert!(seen.len() <= 4, "escalate exceeded the allowed space");
        }
        assert_eq!(seen.len(), 4);
        assert!(escalate(&spaces, &allowed, &cur).is_none());
    }

    /// GAP-16: an alternative no better in DRC, w and h than another, and
    /// worse in one, is dropped; an incomparable one stays.
    #[test]
    fn a_dominated_alternative_is_pruned() {
        assert_eq!(keep(&[(0, 100, 100), (0, 200, 100), (1, 100, 100), (0, 50, 300)], 0, false), [0, 3]);
    }

    /// GAP-16: matched cells keep every alternative, and the seed survives
    /// even when dominated.
    #[test]
    fn matched_cells_are_never_pruned() {
        assert_eq!(keep(&[(0, 100, 100), (0, 200, 100), (1, 100, 100), (0, 50, 300)], 0, true), [0, 1, 2, 3]);
        assert_eq!(keep(&[(0, 50, 50), (0, 100, 100)], 1, false), [0, 1]);
    }

    /// Two matched NMOS on a shared source (tail), distinct gates and drains — the
    /// canonical merge candidate. Net ids: tail 0, g1 1, g2 2, d1 3, d2 4.
    fn matched_pair() -> Netlist {
        let nets = ["tail", "g1", "g2", "d1", "d2"]
            .iter()
            .map(|n| Net {
                name: (*n).to_string(),
            })
            .collect();
        let dev = |name: &str, d: u16, g: u16| Device {
            name: name.to_string(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: vec![
                ("D".to_string(), NetId(d)),
                ("G".to_string(), NetId(g)),
                ("S".to_string(), NetId(0)),
                ("B".to_string(), NetId(0)),
            ],
            params: vec![
                ("w".to_string(), 1000),
                ("l".to_string(), 210),
                ("nf".to_string(), 1),
            ],
        };
        Netlist {
            devices: vec![dev("M1", 3, 1), dev("M2", 4, 2)],
            nets,
            ..Default::default()
        }
    }

    /// A matched unitization over `devices`, the shape the annotator emits for a
    /// recognised pair.
    fn matched_unit(devices: &[u16], kind: DeviceKind) -> Constraints {
        matched_unit_nf(devices, kind, 1)
    }

    /// [`matched_unit`] with an explicit per-member finger count. A device drawn
    /// with ONE finger cannot interleave with anything — ABBA needs two fingers
    /// per member — so any test about interleaving has to say so.
    fn matched_unit_nf(devices: &[u16], kind: DeviceKind, nf: u16) -> Constraints {
        Constraints {
            unitization: vec![Unitization {
                devices: devices.iter().map(|&d| DeviceId(d)).collect(),
                device_type: kind,
                dev_nf: vec![nf; devices.len()],
                target_ratio: vec![1; devices.len()],
                unit_w: 1000,
                unit_l: 210,
                series_parallel: SeriesParallel::Parallel,
                dummy_required: false,
                route_matching_required: false,
                class: None, kind: None, series: Vec::new(), style: None,
            }],
            ..Default::default()
        }
    }

    /// Two matched NMOS mirror legs: shared gate and source, distinct drains — the
    /// topology where every merged pattern (including interleaved ones) is
    /// electrically safe, so the full variant space survives.
    fn matched_mirror() -> Netlist {
        let mut nl = matched_pair(); // nets: tail 0, g1 1, g2 2, d1 3, d2 4
        for t in &mut nl.devices[1].terminals {
            if t.0 == "G" {
                t.1 = NetId(1); // both legs on g1
            }
        }
        nl
    }

    /// GAP-11 through `enumerate`: an Exceptional current mirror (1.5:1) keeps only the variants that meet
    /// the limit (4 fingers: some do), or the squarest alone counted in `aspect_missed` (1 finger: none
    /// does). The same mirror with no kind is unfiltered and has variants past the limit in both cases.
    #[test]
    fn an_exceptional_mirror_keeps_compact_variants() {
        use analog::intent::{MatchClass, MatchKind};
        let (pdk, nl) = (pdk(), matched_mirror());
        let lim = aspect_limit(Some(MatchClass::Exceptional), Some(MatchKind::Current)).unwrap();
        for (nf, missed) in [(4, false), (1, true)] {
            let free = enumerate(&nl, &Macros::default(), &matched_unit_nf(&[0, 1], DeviceKind::Nmos, nf), &pdk, true);
            let mut c = matched_unit_nf(&[0, 1], DeviceKind::Nmos, nf);
            c.unitization[0].class = Some(MatchClass::Exceptional);
            c.unitization[0].kind = Some(MatchKind::Current);
            let cells = enumerate(&nl, &Macros::default(), &c, &pdk, true);
            assert_eq!((free.spaces.len(), cells.spaces.len()), (1, 1), "nf={nf}: the mirror is one cell");
            let all: Vec<f64> = free.spaces[0].alternatives.iter().map(member_aspect).collect();
            let kept: Vec<f64> = cells.spaces[0].alternatives.iter().map(member_aspect).collect();
            assert_eq!(free.aspect_missed, 0);
            assert!(all.iter().any(|&a| a > lim), "nf={nf}: unfiltered space has a variant past {lim}: {all:?}");
            assert_eq!(all.iter().all(|&a| a > lim), missed, "nf={nf}: {all:?}");
            if missed {
                let best = all.iter().copied().fold(f64::INFINITY, f64::min);
                assert_eq!((kept, cells.aspect_missed), (vec![best], 1), "nf={nf}: the squarest is kept and the miss counted");
            } else {
                assert!(!kept.is_empty() && kept.iter().all(|&a| a <= lim), "nf={nf}: {kept:?}");
                assert_eq!(kept.len(), all.iter().filter(|&&a| a <= lim).count(), "nf={nf}: every compact variant survives");
                assert_eq!(cells.aspect_missed, 0);
            }
        }
    }

    /// PLAN §2's collapse, end to end: a matched unitization becomes ONE cell whose
    /// alternatives are merged stacks with every pin bound to a real net — and at
    /// least one alternative is genuinely interleaved (ABBA), which is what makes
    /// same-variant and common-centroid hold by construction. A mirror (shared
    /// gate) keeps its ABBA patterns; see the test below for why a distinct-gate
    /// pair currently does not.
    #[test]
    fn a_matched_unitization_collapses_to_one_cell() {
        let pdk = pdk();
        let netlist = matched_mirror();
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&[0, 1], DeviceKind::Nmos, 2),
            &pdk,
            true,
        );

        assert_eq!(cells.spaces.len(), 1, "the pair is one placeable cell");
        assert_eq!(cells.cell_of, vec![0, 0]);
        assert_eq!(cells.devices_of, vec![vec![DeviceId(0), DeviceId(1)]]);

        // Every pin of every alternative bound to the member's schematic net:
        // d0 → M1 (G=1, D=3, S=0), d1 → M2 (G=1, D=4, S=0).
        let expect = |name: &str| -> Option<NetId> {
            match name {
                "d0:G" | "d1:G" => Some(NetId(1)),
                "d0:D" => Some(NetId(3)),
                "d1:D" => Some(NetId(4)),
                "d0:S" | "d1:S" => Some(NetId(0)),
                // The generator's `d{i}:B` bulk pin, bound to the schematic
                // bulk net (tail, same as S in this fixture).
                "d0:B" | "d1:B" => Some(NetId(0)),
                _ => None,
            }
        };
        let mut seen: Vec<&str> = Vec::new();
        for (v, m) in cells.spaces[0].alternatives.iter().enumerate() {
            for p in &m.pins {
                let e = expect(&p.name).unwrap_or_else(|| panic!("unexpected pin {:?}", p.name));
                assert_eq!(p.net, e, "alternative {v} pin {:?} mis-bound", p.name);
                seen.push(if p.name.starts_with("d1") { "d1" } else { "d0" });
            }
            assert!(
                shared_pads_carry_one_net(m),
                "alternative {v} shorts two nets on one boundary pad"
            );
        }
        assert!(
            seen.contains(&"d0") && seen.contains(&"d1"),
            "both members present"
        );

        // At least one alternative interleaves the two devices (ABBA): one
        // member's diffusion regions then sit *inside* the other's span. A
        // `Single` block layout (AABB) only ever abuts, sharing one region.
        // (Read from S/D pins, not gate pins: a device's fingers are strapped
        // into one node and so surface exactly one gate pin each.)
        let interleaved =
            cells.spaces[0]
                .alternatives
                .iter()
                .any(|m| match region_spans(m, 2)[..] {
                    [Some((a0, a1)), Some((b0, b1))] => a1.min(b1) > a0.max(b0),
                    _ => false,
                });
        assert!(interleaved, "no ABBA alternative in the merged space");
    }

    /// Four matched NMOS mirror legs: one shared gate, one shared source,
    /// distinct drains.
    fn matched_quad() -> Netlist {
        let nets = ["tail", "g", "d1", "d2", "d3", "d4"]
            .iter()
            .map(|n| Net {
                name: (*n).to_string(),
            })
            .collect();
        let dev = |name: &str, d: u16| Device {
            name: name.to_string(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: vec![
                ("D".to_string(), NetId(d)),
                ("G".to_string(), NetId(1)),
                ("S".to_string(), NetId(0)),
                ("B".to_string(), NetId(0)),
            ],
            params: vec![
                ("w".to_string(), 1000),
                ("l".to_string(), 210),
                ("nf".to_string(), 4),
            ],
        };
        Netlist {
            devices: vec![dev("M1", 2), dev("M2", 3), dev("M3", 4), dev("M4", 5)],
            nets,
            ..Default::default()
        }
    }

    /// A matched **quad** reaches the merged space as a real common centroid, not
    /// just as the block order.
    ///
    /// Two things had to hold for this and both are easy to get wrong. The
    /// generator has to offer a centroid finger order for more than two devices
    /// at all (it only ever did pairs), and the order it offers has to survive
    /// the two shorting guards below — a pattern that is quietly dropped here is
    /// indistinguishable, from the placer's side, from one that was never
    /// enumerated.
    #[test]
    fn a_matched_quad_merges_into_a_common_centroid() {
        let pdk = pdk();
        let netlist = matched_quad();
        let members = [0u16, 1, 2, 3];
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&members, DeviceKind::Nmos, 4),
            &pdk,
            true,
        );

        assert_eq!(cells.spaces.len(), 1, "the quad merges into one cell");
        for (v, m) in cells.spaces[0].alternatives.iter().enumerate() {
            assert!(
                shared_pads_carry_one_net(m),
                "alternative {v} shorts two nets on one boundary pad"
            );
            assert!(
                gate_straps_stay_private(m, &netlist, &cells.devices_of[0], &pdk),
                "alternative {v} straps across a foreign gate"
            );
        }

        // Interleaved: with four members, a centroid order puts every member's
        // diffusion regions inside every other's span. The block order (AABB…)
        // only ever abuts, so this is what tells the two apart.
        let interleaved = cells.spaces[0].alternatives.iter().any(|m| {
            let span = region_spans(m, 4);
            span.iter().all(Option::is_some)
                && span.iter().flatten().all(|&(a0, a1)| {
                    span.iter()
                        .flatten()
                        .all(|&(b0, b1)| a1.min(b1) > a0.max(b0))
                })
        });
        assert!(
            interleaved,
            "no common-centroid alternative in the merged quad space"
        );
    }

    /// A single-finger distinct-gate pair too narrow to fold has no merge
    /// left: `D A S B D` runs the two currents opposite ways
    /// (`currents_run_alike`), and ABBA needs two fingers a side. It stays two
    /// cells, matched by placement.
    #[test]
    fn a_distinct_gate_pair_declines_every_merge() {
        let pdk = pdk();
        let mut netlist = matched_pair(); // G nets 1 and 2 — distinct
        // Half of 800 nm is under the deck's minimum finger: no fold.
        for d in &mut netlist.devices {
            d.params[0].1 = 800;
        }
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
            true,
        );
        assert_eq!(cells.spaces.len(), 2, "the pair must not merge");
        // Apart, both are drawn alike, so their currents agree.
        let phi = |c: usize| cells.spaces[c].alternatives[0].units[0].phi;
        assert_eq!(phi(0), phi(1));
    }

    /// With two fingers a side, a distinct-gate pair (a differential input)
    /// merges into common-centroid ABBA: only the split-gate variants survive,
    /// every one balanced in current direction and with private gates.
    #[test]
    fn a_two_finger_diff_pair_merges_as_split_gate_abba() {
        let pdk = pdk();
        let netlist = matched_pair(); // G nets 1 and 2 — distinct
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&[0, 1], DeviceKind::Nmos, 2),
            &pdk,
            true,
        );
        assert_eq!(cells.spaces.len(), 1, "the diff pair merges");
        let alts = &cells.spaces[0].alternatives;
        assert!(!alts.is_empty());
        for m in alts {
            assert!(gate_straps_stay_private(m, &netlist, &cells.devices_of[0], &pdk));
            assert!(currents_run_alike(m, 2));
            // Common centroid (equal first moments per device), or the
            // mirror-pin variant: each drain pad reflects onto the other's.
            let mx = |d: u8| m.units.iter().filter(|u| u.owner == d).map(|u| i64::from(u.x)).sum::<i64>();
            let xs: Vec<i32> = m.units.iter().map(|u| u.x).collect();
            let axis2 = xs.iter().min().unwrap() + xs.iter().max().unwrap();
            let drains = |d: &str| {
                let mut v: Vec<i32> = m.pins.iter().filter(|p| p.name == d).map(|p| 2 * p.at.x + p.at.w).collect();
                v.sort_unstable();
                v
            };
            let mut img: Vec<i32> = drains("d0:D").iter().map(|&x| 2 * axis2 - x).collect();
            img.sort_unstable();
            let mirrored = img.len() == drains("d1:D").len() && img.iter().zip(drains("d1:D")).all(|(a, b)| (a - b).abs() <= 40);
            assert!(mx(0) == mx(1) || mirrored, "neither centroid nor mirror-pin construction");
        }
        assert!(alts.iter().any(|m| {
            let mx = |d: u8| m.units.iter().filter(|u| u.owner == d).map(|u| i64::from(u.x)).sum::<i64>();
            mx(0) == mx(1)
        }), "a common-centroid alternative stays on offer");
    }

    /// A `macro_master` macro is the user's geometry: it is never redrawn, so it can
    /// never join a merged stack — the whole unitization stays per-device.
    #[test]
    fn an_injected_member_declines_the_merge() {
        let pdk = pdk();
        let netlist = matched_pair();
        let mut injected = Macros::default();
        injected.register(
            "M1",
            Macro {
                shapes: vec![Shape {
                    layer: LayerId(1),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 9,
                        h: 9,
                    },
                }],
                pins: vec![Pin {
                    name: "G".to_string(),
                    net: NetId(0),
                    at: Rect {
                        x: 0,
                        y: 0,
                        w: 9,
                        h: 9,
                    },
                    layer: LayerId(1),
                }],
                bbox: Rect {
                    x: 0,
                    y: 0,
                    w: 9,
                    h: 9,
                },
                units: Vec::new(),
                dummies: Vec::new(),
                ..Default::default()
            },
        );
        let cells = enumerate(
            &netlist,
            &injected,
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
            true,
        );
        assert_eq!(
            cells.spaces.len(),
            2,
            "injected member must keep the pair per-device"
        );
        assert_eq!(cells.cell_of, vec![0, 1]);
        assert_eq!(
            cells.spaces[0].alternatives.len(),
            1,
            "M1 is still the user's macro"
        );
    }

    /// The drawn-short guard: the merged `finger_sequence` puts inter-device
    /// diffusion boundaries on S, so members on different source nets would be
    /// physically shorted by geometry no DRC rule can object to. Decline, honestly.
    #[test]
    fn mismatched_source_nets_decline_the_merge() {
        let pdk = pdk();
        let mut netlist = matched_pair();
        // Move M2's source off the shared tail.
        for t in &mut netlist.devices[1].terminals {
            if t.0 == "S" {
                t.1 = NetId(4);
            }
        }
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
            true,
        );
        assert_eq!(
            cells.spaces.len(),
            2,
            "different source nets must not share diffusion"
        );
        assert_eq!(cells.cell_of, vec![0, 1]);
    }

    /// FLOW-16: a fold class is one cell's members, so an unrelated
    /// same-size device folds for its own row, and the mirror as it would alone.
    #[test]
    fn unrelated_same_size_devices_fold_independently() {
        let pdk = pdk();
        let (a, b) = ("XA da g s s nfet_01v8 W=2u L=0.5u\n", "XB g g s s nfet_01v8 W=2u L=0.5u\n");
        // XC: 2 µm fingers too, so a netlist-wide (kind, finger W, L) class would take it in.
        let c = "XC dc gc sc sc nfet_01v8 W=32u L=0.5u nf=16\n";
        let nl = |body: &str| crate::parse(&format!("{body}.end\n")).expect("parses");
        let ab = [(vec![DeviceId(0), DeviceId(1)], false)];
        let all = folds(&nl(&format!("{a}{b}{c}")), &pdk, &[], &ab);
        assert_eq!(all[0], folds(&nl(&format!("{a}{b}")), &pdk, &[], &ab)[0]);
        assert_eq!(all[2], folds(&nl(c), &pdk, &[], &[])[0]);
    }

    /// FLOW-16 step 4: the OTA input pair, route-matched, folds to the first
    /// `k` with an exact mirror order (nf = 8 per device, 1.25 µm fingers).
    #[test]
    fn a_route_matched_pair_folds_to_a_centroid_row() {
        let pdk = pdk();
        let netlist = crate::parse("XM1 vout1 vinp vtail VSS nfet_01v8 W=10u L=1u nf=2\nXM2 vout2 vinm vtail VSS nfet_01v8 W=10u L=1u nf=2\n.end\n").expect("parses");
        let f = folds(&netlist, &pdk, &[], &[(vec![DeviceId(0), DeviceId(1)], true)]);
        assert_eq!(f[0].0, 4, "{f:?}");
        assert_eq!(f[0], f[1]);
        assert!(cells::mosfet::cc_row_exists(&[8, 8], true));
    }

    /// A high-gm device folds into enough fingers that its gate R stays under
    /// 1/(5·gm): 20 µm / 150 nm at 10 mS needs N ≥ √(5·gm·R□·W/3L) ≈ 10.4.
    #[test]
    fn transconductance_sets_a_finger_floor() {
        let pdk = pdk();
        let mut netlist = two_devices();
        netlist.devices.truncate(1);
        netlist.devices[0].params = vec![("w".into(), 20_000), ("l".into(), 150)];
        let (k0, _) = folds(&netlist, &pdk, &[], &[])[0];
        let (k, fw) = folds(&netlist, &pdk, &[Some(10_000.0)], &[])[0];
        assert!(k >= 11 && k > k0, "k = {k} (without gm {k0})");
        assert!(fw >= 420);
    }

    /// D7's round trip over a *merged* space: `realize` hands back the named merged
    /// alternative, and `escalate` walks the joint space without repeats and
    /// exhausts — a merged cell is one odometer digit like any other.
    #[test]
    fn realize_and_escalate_round_trip_over_a_merged_space() {
        let pdk = pdk();
        // A mirror pair at four fingers a side: orders × one or two rows.
        let netlist = matched_mirror();
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&[0, 1], DeviceKind::Nmos, 4),
            &pdk,
            true,
        );
        let spaces = &cells.spaces;
        assert_eq!(spaces.len(), 1);
        let depth = spaces[0].alternatives.len();
        assert!(
            depth >= 2,
            "a merged pair should still have a real variant space"
        );

        for v in 0..depth as u16 {
            let got = realize(spaces, &[v]);
            assert_eq!(got.len(), 1);
            assert_eq!(got[0], spaces[0].alternatives[v as usize]);
        }

        let mut cur = vec![0u16];
        let mut seen = vec![cur.clone()];
        while let Some(next) = escalate(spaces, &full(spaces), &cur) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            seen.push(next.clone());
            cur = next;
        }
        assert_eq!(
            seen.len(),
            depth,
            "escalate must cover exactly the merged space"
        );
    }

    /// This seeds the whole run, so it must be a function of its inputs alone — a
    /// hash-ordered tie-break would make two runs of the same netlist diverge from
    /// epoch zero.
    #[test]
    fn seed_assignment_is_deterministic() {
        let pdk = pdk();
        let netlist = two_devices();
        let cells = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk, true);
        let matched = vec![false; cells.spaces.len()];
        let (a, allowed) = seed_assignment(&cells.spaces, &matched, &[], &pdk);
        let (b, _) = seed_assignment(&cells.spaces, &matched, &[], &pdk);
        assert!(a.iter().zip(&allowed).all(|(v, row)| row.contains(v)), "the seed is always allowed");
        assert_eq!(a, b);
        assert_eq!(a.len(), cells.spaces.len());
        for (i, &v) in a.iter().enumerate() {
            assert!(
                usize::from(v) < cells.spaces[i].alternatives.len(),
                "cell {i} seeded out of range"
            );
        }
    }

    /// A binary bank on one top plate is one cell, drawn only as the array, with
    /// the rail-tied one-unit cap first (slot 0) wherever the netlist lists it.
    #[test]
    fn a_binary_cap_bank_becomes_one_array_cell() {
        let pdk = pdk();
        let nets = ["top", "vss", "b0", "b1", "b2"].iter().map(|n| Net { name: (*n).to_string() }).collect();
        let cap = |name: &str, bot: u16, m: i64| Device {
            name: name.to_string(),
            kind: DeviceKind::Capacitor, model: String::new(),
            terminals: vec![("P".to_string(), NetId(0)), ("N".to_string(), NetId(bot))],
            params: vec![("w".to_string(), 2000), ("l".to_string(), 2000), ("m".to_string(), m)],
        };
        let sw = Device {
            name: "M1".to_string(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: [("D", 2), ("G", 2), ("S", 1), ("B", 1)].iter().map(|&(t, n)| (t.to_string(), NetId(n))).collect(),
            params: vec![("w".to_string(), 1000), ("l".to_string(), 150)],
        };
        let netlist = Netlist {
            devices: vec![cap("C1", 2, 1), cap("C2", 3, 2), cap("C3", 4, 4), cap("C0", 1, 1), sw],
            nets,
            ..Default::default()
        };
        // The bank is the annotator's matched set since EXT-19 (cellgen's `dac_banks` moved there).
        let annot = annotator::annotate(&netlist, &crate::annotation(&pdk, &Default::default())).constraints;
        let cells = enumerate(&netlist, &Macros::default(), &annot, &pdk, true);
        let bank = &cells.devices_of[cells.cell_of[0] as usize];
        assert_eq!(bank, &[DeviceId(3), DeviceId(0), DeviceId(1), DeviceId(2)], "dummy first, then by weight");
        let alts = &cells.spaces[cells.cell_of[0] as usize].alternatives;
        assert!(!alts.is_empty() && alts.iter().all(|m| m.units.len() == 8), "every alternative is the 2^3 array");
    }

    /// A lone bipolar draws `m` units (`cells::bjt` draws its unitization's
    /// `dev_nf`) and its LVS reference holds `m` cards; `nf` counts neither (it
    /// once drew `max(nf, m)` against `m` cards).
    #[test]
    fn lone_bjt_draws_as_many_units_as_reference_cards() {
        let pdk = pdk();
        let netlist = crate::parse("XQ1 c b e sky130_fd_pr__pnp_05v5_W3p40L3p40 nf=3 m=2\n.end\n").expect("parses");
        let sized = with_per_device_sizing(&netlist, &Constraints::default(), &folds(&netlist, &pdk, &[], &[]));
        let u = sized.unitization.iter().find(|u| u.devices == [DeviceId(0)]).expect("a 1-device unitization");
        assert_eq!(u.dev_nf, vec![2], "drawn units");
        assert_eq!(reference(&netlist, None, &[]).devices.len(), 2, "one reference card per drawn unit");
    }

    /// BJT cards are collector first, schematic and drawn alike (GPurify reads
    /// card position 0 as Collector).
    #[test]
    fn a_bjt_reference_is_collector_first() {
        let pdk = pdk();
        let mut netlist = crate::parse(include_str!("../../../benchmarks/fixtures/bjt_mirror.spice")).expect("parses");
        crate::deck_models(&mut netlist, &pdk);
        let r = reference(&netlist, None, &[]);
        assert_eq!(r.devices[0].terminals, ["outn", "in", "VSS"]);
        let q2 = netlist.devices.iter().position(|d| d.name == "XQ2").unwrap();
        assert_eq!(netlist.devices[q2].kind, DeviceKind::Pnp);
        let group = DeviceGroup { devices: vec![DeviceId(q2 as u16)] };
        let sized = with_per_device_sizing(&netlist, &Constraints::default(), &folds(&netlist, &pdk, &[], &[]));
        let mut m = draw_variants(DeviceKind::Pnp, &netlist.devices[q2].model, &group, &sized, &pdk)
            .into_iter()
            .next()
            .expect("a PNP variant");
        bind_pins(&mut m, &netlist, &group.devices, None);
        let names: Vec<String> = netlist.nets.iter().map(|n| n.name.clone()).collect();
        let (cards, _) = drawn_cards(&[m], &names, &netlist, &pdk);
        assert_eq!(cards.len(), 1, "{cards:?}");
        assert_eq!(cards[0].terminals, ["outp", "in", "VDD"]);
    }

    /// A 2-segment resistor's drawn cards: one per segment, joined by the
    /// macro-internal node, no params (GPurify extracts none for a resistor).
    #[test]
    fn drawn_cards_carry_no_params_for_passives() {
        let pdk = pdk();
        let netlist = crate::parse("XR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u\n.end\n").expect("parses");
        let group = DeviceGroup { devices: vec![DeviceId(0)] };
        let sized = with_per_device_sizing(&netlist, &Constraints::default(), &folds(&netlist, &pdk, &[], &[]));
        let mut m = draw_variants(DeviceKind::Resistor, &netlist.devices[0].model, &group, &sized, &pdk)
            .into_iter()
            .find(|m| m.drawn.len() == 2)
            .expect("a 2-segment variant at L = 40 um");
        bind_pins(&mut m, &netlist, &group.devices, None);
        let names: Vec<String> = netlist.nets.iter().map(|n| n.name.clone()).collect();
        let (cards, replaced) = drawn_cards(&[m.clone()], &names, &netlist, &pdk);
        assert_eq!(replaced, vec![DeviceId(0)]);
        assert_eq!(cards.len(), 2, "{cards:?}");
        for c in &cards {
            assert_eq!(c.kind, RefKind::Resistor);
            assert!(c.params.is_empty(), "{c:?}");
            assert!(c.terminals.iter().any(|t| t == "~0.0.1"), "{c:?}");
        }
        let ends: Vec<&str> = cards.iter().flat_map(|c| &c.terminals).map(String::as_str).filter(|t| !t.starts_with('~')).collect();
        assert_eq!(ends, ["a", "b"], "the string runs a -> ~0.0.1 -> b");

        // A lost pin keeps the card, on a net nothing drawn reaches.
        m.pins.retain(|p| p.name != "d0:P");
        let (cards, _) = drawn_cards(&[m], &names, &netlist, &pdk);
        assert_eq!(cards.len(), 2, "{cards:?}");
        assert!(cards.iter().any(|c| c.terminals.iter().any(|t| t == "~0.0.no-P")), "{cards:?}");
    }

    /// GAP-18: a ranked cell seeds at alternative 0 whatever its prices; unranked keeps `(DRC+ERC, HPWL)` with
    /// index ties.
    #[test]
    fn ranked_cells_seed_at_their_first_alternative() {
        assert_eq!(seed_of(&[(1, 5), (0, 90), (0, 10)], false), 2);
        assert_eq!(seed_of(&[(1, 5), (0, 90), (0, 10)], true), 0);
        assert_eq!(seed_of(&[(9, 9), (0, 1)], true), 0);
    }

    /// GAP-18 acceptance: dac4's bank, annotated Exceptional, draws its lowest-M_sys variant as alternative 0
    /// and the flow seeds it (`seed_assignment(...).0[ci] == 0`; non-vacuous: the unranked seed differs).
    #[test]
    fn exceptional_dac4_seeds_the_lowest_msys() {
        let pdk = pdk();
        let mut nl = crate::parse(include_str!("../../../benchmarks/fixtures/dac4.spice")).expect("parses");
        crate::deck_models(&mut nl, &pdk);
        let id = |n: &str| DeviceId(nl.devices.iter().position(|d| d.name == n).unwrap_or_else(|| panic!("{n}")) as u16);
        let devices: Vec<DeviceId> = ["XC0", "XC1", "XC2", "XC3", "XC4"].into_iter().map(id).collect();
        let c = Constraints {
            unitization: vec![Unitization {
                devices: devices.clone(),
                device_type: DeviceKind::Capacitor,
                dev_nf: vec![1, 1, 2, 4, 8],
                target_ratio: vec![1, 1, 2, 4, 8],
                unit_w: 2000,
                unit_l: 2000,
                series_parallel: SeriesParallel::Parallel,
                dummy_required: true,
                route_matching_required: true,
                class: Some(pnr_core::MatchClass::Exceptional), kind: None, series: Vec::new(), style: None,
            }],
            ..Default::default()
        };
        let cells = enumerate(&nl, &Macros::default(), &c, &pdk, false);
        let ci = usize::from(cells.cell_of[devices[0].0 as usize]);
        assert_eq!(cells.devices_of[ci], devices, "the bank is one cell");
        let group = DeviceGroup { devices };
        let model = &nl.devices[group.devices[0].0 as usize].model;
        let recipe = pdk.recipe("capacitor", model).expect("cap_generic_m1m2 has a capacitor recipe");
        let p = verify::pdk::Overlay { pdk: &pdk, recipe };
        let variants = CapArray::enumerate(&group, &c, &p);
        let ms: Vec<f64> = variants.iter().map(|v| v.metrics(&group, &c, &p, cells::cap_array::RANK_G_PER_UM).msys).collect();
        assert!(ms.iter().any(|&m| m > ms[0]), "vacuous: every variant has M_sys {ms:?}");
        assert!(ms.iter().all(|&m| ms[0] <= m), "variant 0 is not the lowest M_sys: {ms:?}");
        assert!(cells.spaces[ci].alternatives[0].shapes == variants[0].draw(&group, &c, &p).shapes, "alternative 0 is not the ranked first variant");
        let mut ranked = vec![false; cells.spaces.len()];
        ranked[ci] = true;
        let matched = vec![true; cells.spaces.len()];
        assert_eq!(seed_assignment(&cells.spaces, &matched, &ranked, &pdk).0[ci], 0, "the ranked bank seeds at its best-matching variant");
        // Non-vacuous: unranked, the same bank seeds elsewhere (fewer x.22 findings on variant 1).
        assert_ne!(seed_assignment(&cells.spaces, &matched, &[], &pdk).0[ci], 0, "ranking does not change the seed here");
    }
}
