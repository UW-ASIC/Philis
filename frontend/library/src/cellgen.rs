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
use pnr_core::{Device, DeviceGroup, DeviceId, DeviceKind, Macro, NetId, Netlist, Rect};
use verify::{Checker, Checks, Pdk, RefDeviceIn, RefInput, RefKind};

/// The cell table `gp`/`dp` search, and its map to the netlist's devices.
/// A matched group collapses to one cell, so `n_cells <= n_devices`.
pub struct Cells {
    /// Variant space per cell.
    pub spaces: Vec<gp::VariantSpace>,
    /// `cell_of[device]` = its cell.
    pub cell_of: Vec<u16>,
    /// Member devices per cell in draw order: `devices_of[c][k]` is the device
    /// the generator's `d{k}:` pin prefix names.
    pub devices_of: Vec<Vec<DeviceId>>,
}

/// Draw every legal variant of every cell.
///
/// A device with a user macro under its instance name is a one-alternative
/// cell. A multi-device unitization merges into one cell (interleaved stack:
/// same-variant, ratio and common-centroid hold by construction) unless a
/// member is injected, it overlaps an earlier unitization, its MOS members do
/// not share a source net, or every merged pattern draws a short. Declined
/// merges fall back to one cell per device.
#[must_use]
///
/// `merge_distinct_gates = false` keeps members on different gate nets (a
/// differential input) as separate cells: the alternative to a common-centroid
/// merge, which splits one member's drain across the row.
pub fn enumerate(
    netlist: &Netlist,
    macros: &Macros,
    constraints: &Constraints,
    pdk: &Pdk,
    merge_distinct_gates: bool,
) -> Cells {
    let cells: Vec<(Vec<DeviceId>, bool)> = constraints.unitization.iter().map(|u| (u.devices.clone(), u.route_matching_required)).collect();
    enumerate_folded(netlist, macros, constraints, pdk, merge_distinct_gates, &folds(netlist, pdk, &[], &cells), None)
}

/// [`enumerate`] at a given fold table ([`folds`]); the flow computes it once
/// so the cells and every LVS reference agree.
#[must_use]
pub fn enumerate_folded(
    netlist: &Netlist,
    macros: &Macros,
    constraints: &Constraints,
    pdk: &Pdk,
    merge_distinct_gates: bool,
    fold: &[(u16, i32)],
    ground: Option<NetId>,
) -> Cells {
    let sized = with_per_device_sizing(netlist, constraints, fold);
    let n = netlist.devices.len();
    let dev = |d: &DeviceId| &netlist.devices[d.0 as usize];

    // Phase 1: decide and draw the merges.
    let mut unit_of: Vec<Option<usize>> = vec![None; n];
    let mut merged: Vec<Option<(Vec<DeviceId>, Vec<Macro>)>> = Vec::new();
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
            || members.iter().any(|d| unit_of[d.0 as usize].is_some())
        {
            continue;
        }
        // The merged sequence puts every inter-device diffusion boundary on S:
        // differing S nets would be a short DRC cannot see — unless the
        // members are a series stack, drawn as a chain whose junctions are
        // exactly the nets they share (Razavi Fig. 19.12).
        let mut chain: Option<Vec<bool>> = None;
        if matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos) {
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
                let mut alts = draw_variants(kind, &dev(&members[0]).model, &group, &sized, pdk);
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
        let (members, alternatives) = if let Some(ui) = unit_of[i] {
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
            for m in &mut alternatives {
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
    }
}

fn terminal(d: &Device, name: &str) -> Option<NetId> {
    d.terminals.iter().find(|(t, _)| t == name).map(|(_, n)| *n)
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
        if let Some((k, t)) = p.name.strip_prefix('d').and_then(|r| r.split_once(':')).and_then(|(k, t)| Some((k.parse::<usize>().ok()?, t))) {
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
        let Some((n, t)) = pin.name.strip_prefix('d').and_then(|r| r.split_once(':')) else {
            continue;
        };
        if t != "S" && t != "D" {
            continue;
        }
        let Some(slot) = n.parse::<usize>().ok().and_then(|i| span.get_mut(i)) else {
            continue;
        };
        let e = slot.get_or_insert((pin.at.x, pin.at.x));
        e.0 = e.0.min(pin.at.x);
        e.1 = e.1.max(pin.at.x);
    }
    span
}

/// Starting variant per cell, chosen by measuring each alternative in
/// isolation (see [`price`]). Ties break on index, so the seed is deterministic.
/// Also per cell the alternatives [`escalate`] may visit ([`keep`] over
/// `(DRC+ERC, bbox w, bbox h)` from the same prices; `matched[i]` keeps all).
#[must_use]
pub fn seed_assignment(variants: &[gp::VariantSpace], matched: &[bool], pdk: &Pdk) -> (Vec<u16>, Vec<Vec<u16>>) {
    let mut checker = Checker::new(pdk, true).expect("a loaded Pdk re-parses its own deck");
    variants
        .iter()
        .enumerate()
        .map(|(i, space)| {
            let prices: Vec<(usize, i64)> = space.alternatives.iter().map(|m| price(m, &mut checker)).collect();
            let seed = prices.iter().enumerate().min_by(|a, b| a.1.cmp(b.1)).map_or(0, |(v, _)| v);
            let cost: Vec<(usize, i32, i32)> = prices.iter().zip(&space.alternatives).map(|(p, m)| (p.0, m.bbox.w, m.bbox.h)).collect();
            (seed as u16, keep(&cost, seed, matched.get(i).copied().unwrap_or(true)))
        })
        .unzip()
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

/// The geometry an assignment selects. A missing entry means variant 0.
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
/// `allowed[i]` (ascending; [`seed_assignment`]), fastest digit = the cell
/// whose alternatives move pins the most ([`pin_spread`]): never repeats,
/// always terminates, and changes pin geometry first. A cell with an empty
/// or one-entry `allowed` row is a fixed digit.
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
/// every device no unitization covers: a MOS gets `nf·m` fingers of
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
    for bank in dac_banks(netlist, &covered) {
        let dev_nf: Vec<u16> = bank.iter().map(|d| multiplier(&netlist.devices[d.0 as usize])).collect();
        let dev = &netlist.devices[bank[0].0 as usize];
        let nm = |k: &str| dev.params.iter().find(|(n, _)| n == k).map_or(0, |&(_, v)| v.clamp(0, i64::from(i32::MAX)) as i32);
        for d in &bank {
            covered[d.0 as usize] = true;
        }
        unitization.push(Unitization {
            devices: bank,
            device_type: DeviceKind::Capacitor,
            target_ratio: dev_nf.clone(),
            dev_nf,
            unit_w: nm("w"),
            unit_l: nm("l"),
            series_parallel: SeriesParallel::Parallel,
            dummy_required: true,
            route_matching_required: true,
        });
    }
    // Uncovered bipolars of one kind and geometry on one base net are a
    // ratioed set (a bandgap's 1:N): one array cell, units centre-out.
    for group in bjt_groups(netlist, &covered) {
        let dev_nf: Vec<u16> = group.iter().map(|d| multiplier(&netlist.devices[d.0 as usize])).collect();
        let dev = &netlist.devices[group[0].0 as usize];
        let nm = |k: &str| dev.params.iter().find(|(n, _)| n == k).map_or(0, |&(_, v)| v.clamp(0, i64::from(i32::MAX)) as i32);
        for d in &group {
            covered[d.0 as usize] = true;
        }
        unitization.push(Unitization {
            devices: group,
            device_type: dev.kind,
            target_ratio: dev_nf.clone(),
            dev_nf,
            unit_w: nm("w"),
            unit_l: nm("l"),
            series_parallel: SeriesParallel::Parallel,
            dummy_required: false,
            route_matching_required: true,
        });
    }
    // Uncovered MOS devices on the same four nets at the same W/L are one
    // device written as several cards: one cell, one shared diffusion row.
    for group in parallel_groups(netlist, &covered) {
        let dev = &netlist.devices[group[0].0 as usize];
        let nm = |k: &str| dev.params.iter().find(|(n, _)| n == k).map_or(0, |&(_, v)| v.clamp(0, i64::from(i32::MAX)) as i32);
        let dev_nf: Vec<u16> = group.iter().map(|d| mos_fingers(&netlist.devices[d.0 as usize])).collect();
        for d in &group {
            covered[d.0 as usize] = true;
        }
        unitization.push(Unitization {
            devices: group,
            device_type: dev.kind,
            target_ratio: dev_nf.clone(),
            dev_nf,
            unit_w: dev.mos_size().map_or(0, |s| s.w_finger_nm().min(i64::from(i32::MAX)) as i32),
            unit_l: nm("l"),
            series_parallel: SeriesParallel::Parallel,
            dummy_required: false,
            route_matching_required: false,
        });
    }
    for (i, dev) in netlist
        .devices
        .iter()
        .enumerate()
        .filter(|(i, _)| !covered[*i])
    {
        let param = |k: &str| {
            dev.params
                .iter()
                .find(|(n, _)| n == k)
                .map_or(0, |(_, v)| *v)
        };
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
        });
    }
    // Fold every MOS unitization by its width class's factor: `k`× the
    // fingers at `W/k`, ratios kept (every member scales alike).
    for u in &mut unitization {
        let Some(&(k, w)) = u.devices.first().and_then(|d| fold.get(d.0 as usize)) else { continue };
        if k > 1 && matches!(u.device_type, DeviceKind::Nmos | DeviceKind::Pmos) {
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
    let param = |d: &Device, k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let nm = |d: &Device, k: &str| param(d, k).map_or(0, |v| v.clamp(0, i64::from(i32::MAX)) as i32);
    // A MOS's width is its finger's, `W_total/nf`; anything else its written `w`.
    let w_of = |d: &Device| d.mos_size().map_or(nm(d, "w"), |s| s.w_finger_nm().min(i64::from(i32::MAX)) as i32);
    let mos = |d: &Device| matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos);
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

/// [`pnr_core::MosSize::fingers`] as a unitization count; `1` without a size.
fn mos_fingers(d: &Device) -> u16 {
    d.mos_size().map_or(1, |s| s.fingers().min(u32::from(u16::MAX)) as u16)
}

/// Uncovered bipolars sharing kind, W, L and base net, in netlist order;
/// singletons left out.
fn bjt_groups(netlist: &Netlist, covered: &[bool]) -> Vec<Vec<DeviceId>> {
    let param = |d: &Device, k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let mut groups: Vec<(_, Vec<DeviceId>)> = Vec::new();
    for (i, d) in netlist.devices.iter().enumerate() {
        if covered[i] || !matches!(d.kind, DeviceKind::Npn | DeviceKind::Pnp) {
            continue;
        }
        let k = (d.kind, param(d, "w"), param(d, "l"), terminal(d, "B"));
        match groups.iter_mut().find(|(g, _)| *g == k) {
            Some((_, v)) => v.push(DeviceId(i as u16)),
            None => groups.push((k, vec![DeviceId(i as u16)])),
        }
    }
    groups.into_iter().map(|(_, v)| v).filter(|v| v.len() > 1).collect()
}

/// Uncovered MOS devices sharing kind, finger W, L and every terminal net, in
/// netlist order; singletons are left out.
fn parallel_groups(netlist: &Netlist, covered: &[bool]) -> Vec<Vec<DeviceId>> {
    let key = |d: &Device| (d.kind, d.mos_size().map(|s| (s.w_finger_nm(), s.l_nm)), d.terminals.clone());
    let mut groups: Vec<(_, Vec<DeviceId>)> = Vec::new();
    for (i, d) in netlist.devices.iter().enumerate() {
        if covered[i] || !matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            continue;
        }
        let k = key(d);
        match groups.iter_mut().find(|(g, _)| *g == k) {
            Some((_, v)) => v.push(DeviceId(i as u16)),
            None => groups.push((k, vec![DeviceId(i as u16)])),
        }
    }
    groups.into_iter().map(|(_, v)| v).filter(|v| v.len() > 1).collect()
}

fn multiplier(d: &Device) -> u16 {
    d.params.iter().find(|(n, _)| n == "m").map_or(1, |&(_, v)| v.clamp(1, i64::from(u16::MAX)) as u16)
}

/// Uncovered capacitors on one top plate (`P`), one model and one `w`×`l` whose `m` are
/// `[1, 1, 2, …, 2^(N-1)]`: a binary-weighted DAC bank (DACP §II), members in
/// slot order. The electrical dummy (slot 0) is the one-unit cap whose bottom
/// plate is a MOS bulk, i.e. a rail; else the first one-unit cap listed.
fn dac_banks(netlist: &Netlist, covered: &[bool]) -> Vec<Vec<DeviceId>> {
    let bulks: Vec<NetId> = netlist.devices.iter().filter_map(|d| terminal(d, "B")).collect();
    let param = |d: &Device, k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let mut by_plate: Vec<((NetId, String, Option<i64>, Option<i64>), Vec<DeviceId>)> = Vec::new();
    for (i, d) in netlist.devices.iter().enumerate() {
        let Some(p) = terminal(d, "P").filter(|_| d.kind == DeviceKind::Capacitor && !covered[i]) else { continue };
        let key = (p, d.model.clone(), param(d, "w"), param(d, "l"));
        match by_plate.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(DeviceId(i as u16)),
            None => by_plate.push((key, vec![DeviceId(i as u16)])),
        }
    }
    let dev = |d: &DeviceId| &netlist.devices[d.0 as usize];
    by_plate
        .into_iter()
        .filter_map(|(_, mut bank)| {
            bank.sort_by_key(|d| multiplier(dev(d)));
            let counts: Vec<u16> = bank.iter().map(|d| multiplier(dev(d))).collect();
            cells::cap_array::bits(&counts)?;
            let on_rail = |d: &DeviceId| terminal(dev(d), "N").is_some_and(|n| bulks.contains(&n));
            if !on_rail(&bank[0]) && on_rail(&bank[1]) {
                bank.swap(0, 1);
            }
            Some(bank)
        })
        .collect()
}

/// Every enumerated variant of one group, by device kind.
/// Every variant of `group`, a `kind` of schematic `model`. A resistor is
/// drawn to its model's recipe ([`Pdk::recipe`]): the construction the
/// deck's recogniser for that model expects.
fn draw_variants(kind: DeviceKind, model: &str, group: &DeviceGroup, c: &Constraints, pdk: &Pdk) -> Vec<Macro> {
    match kind {
        // A fin process draws its transistors from fins.
        DeviceKind::Nmos | DeviceKind::Pmos if pnr_core::Process::layer(pdk, "fin").is_some() => draw_all::<cells::finfet::FinFet>(group, c, pdk),
        DeviceKind::Nmos | DeviceKind::Pmos => draw_all::<Mosfet>(group, c, pdk),
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

/// `Cell::enumerate` order. Never empty: an empty enumeration yields one empty
/// macro so `variant == 0` always names something.
fn draw_all<G: Cell>(group: &DeviceGroup, c: &Constraints, pdk: &dyn pnr_core::Process) -> Vec<Macro> {
    let drawn: Vec<Macro> = G::enumerate(group, c, pdk)
        .iter()
        .map(|v| v.draw(group, c, pdk))
        .collect();
    if drawn.is_empty() {
        return vec![Macro {
            shapes: Vec::new(),
            pins: Vec::new(),
            bbox: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        }];
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
        let (fingers, params) = if matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
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
            let nf = dev.params.iter().find(|(n, _)| n == "nf").map_or(1, |&(_, v)| v);
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
                Node::Pin(t) => {
                    let name = format!("d{}:{t}", d.owner);
                    let net = m.pins.iter().find(|p| p.name == name).and_then(|p| nets.get(p.net.0 as usize)).cloned();
                    Some(net.unwrap_or_else(|| format!("~{cell}.{}.no-{t}", d.owner)))
                }
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
        let net = |owner: u8, t: &str| {
            let name = format!("d{owner}:{t}");
            m.pins.iter().find(|p| p.name == name).and_then(|p| nets.get(p.net.0 as usize)).cloned()
        };
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
    use pnr_core::{LayerId, Net, Pin, Shape};

    /// The sky130 deck the benchmarks use, compiled in: a sidecar that fails
    /// validation fails the test, never skips it.
    fn pdk() -> Pdk {
        Pdk::builtin("sky130").expect("sky130 loads")
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
        let (a, allowed) = seed_assignment(&cells.spaces, &matched, &pdk);
        let (b, _) = seed_assignment(&cells.spaces, &matched, &pdk);
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
        let cells = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk, true);
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
}
