//! Cells: draw every legal variant of every cell (collapsing matched groups
//! into one cell), seed and escalate the variant assignment, and build the LVS
//! reference from the schematic.

use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::bjt::Bjt;
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
pub fn enumerate(
    netlist: &Netlist,
    macros: &Macros,
    constraints: &Constraints,
    pdk: &Pdk,
) -> Cells {
    let sized = with_per_device_sizing(netlist, constraints);
    let n = netlist.devices.len();
    let dev = |d: &DeviceId| &netlist.devices[d.0 as usize];

    // Phase 1: decide and draw the merges.
    let mut unit_of: Vec<Option<usize>> = vec![None; n];
    let mut merged: Vec<Option<(Vec<DeviceId>, Vec<Macro>)>> = Vec::new();
    for u in &sized.unitization {
        let members: Vec<DeviceId> = u
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
        // differing S nets would be a short DRC cannot see.
        if matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            let s = |d: &DeviceId| terminal(dev(d), "S");
            if s(&members[0]).is_none() || members.iter().any(|d| s(d) != s(&members[0])) {
                continue;
            }
        }
        let group = DeviceGroup {
            devices: members.clone(),
        };
        let mut alternatives = draw_variants(kind, &group, &sized, pdk);
        for m in &mut alternatives {
            bind_pins(m, netlist, &members);
        }
        alternatives.retain(|m| {
            shared_pads_carry_one_net(m) && gate_straps_stay_private(m, netlist, &members)
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
            let mut alternatives = draw_variants(d.kind, &group, &sized, pdk);
            for m in &mut alternatives {
                bind_pins(m, netlist, &group.devices);
            }
            (group.devices, alternatives)
        };
        for d in &members {
            cell_of[d.0 as usize] = ci;
        }
        devices_of.push(members);
        spaces.push(gp::VariantSpace {
            alternatives,
            lock: None,
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

/// Pins drawn on one pad carry one net — otherwise a shared diffusion region
/// shorts two nets (invisible to DRC, fatal to LVS).
fn shared_pads_carry_one_net(m: &Macro) -> bool {
    m.pins.iter().enumerate().all(|(i, a)| {
        m.pins[i + 1..]
            .iter()
            .all(|b| a.at != b.at || a.net == b.net)
    })
}

/// No member's gate strap crosses another member's stubs on a different gate
/// net. A strap spans its device's fingers, so two members whose S/D region
/// spans overlap by more than one shared region are interleaved and the strap
/// shorts the gates.
///
/// ponytail: mirrors `mosfet::draw`'s strap rule rather than extracting; drop
/// once the generator jogs straps around foreign stubs.
fn gate_straps_stay_private(m: &Macro, netlist: &Netlist, members: &[DeviceId]) -> bool {
    let gate = |d: &DeviceId| terminal(&netlist.devices[d.0 as usize], "G");
    let span = region_spans(m, members.len());
    members.iter().enumerate().all(|(a, da)| {
        members.iter().enumerate().skip(a + 1).all(|(b, db)| {
            gate(da) == gate(db)
                || match (span[a], span[b]) {
                    (Some((a0, a1)), Some((b0, b1))) => a1.min(b1) <= a0.max(b0),
                    _ => true,
                }
        })
    })
}

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
#[must_use]
pub fn seed_assignment(
    variants: &[gp::VariantSpace],
    layers: &[pnr_core::LayerId],
    pdk: &Pdk,
) -> Vec<u16> {
    let cfg = gr::GlobalCfg::default();
    let mut checker = Checker::new(pdk, true).expect("a loaded Pdk re-parses its own deck");
    variants
        .iter()
        .map(|space| {
            space
                .alternatives
                .iter()
                .map(|m| price(m, layers, &cfg, &mut checker))
                .enumerate()
                .min_by(|a, b| a.1.cmp(&b.1))
                .map_or(0, |(v, _)| v as u16)
        })
        .collect()
}

/// `(unreachable, DRC+ERC findings, congestion, wirelength)`, worst-last and
/// compared lexicographically: unreachable pins are infeasible, not expensive.
/// DRC runs density-stripped (fill rules are chip-level); a macro the engine
/// cannot load prices as maximally illegal.
///
/// ponytail: one DRC+ERC pass per alternative per run (~13 ms each on sky130).
fn price(
    m: &Macro,
    layers: &[pnr_core::LayerId],
    cfg: &gr::GlobalCfg,
    checker: &mut Checker,
) -> (bool, usize, i64, i64) {
    let p = gr::price_group(std::slice::from_ref(m), layers, cfg);
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
    (!p.reachable, geom, p.overflow, p.hpwl)
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
/// A mixed-radix odometer over the cells, fastest digit = the cell whose
/// alternatives move pins the most ([`pin_spread`]): never repeats, always
/// terminates, and changes pin geometry first.
#[must_use]
pub fn escalate(variants: &[gp::VariantSpace], current: &[u16]) -> Option<Vec<u16>> {
    let spread: Vec<usize> = variants.iter().map(pin_spread).collect();
    let mut order: Vec<usize> = (0..variants.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(spread[i]), i));
    let mut next: Vec<u16> = (0..variants.len())
        .map(|i| current.get(i).copied().unwrap_or(0))
        .collect();
    for &i in &order {
        if usize::from(next[i]) + 1 < variants[i].alternatives.len() {
            next[i] += 1;
            return Some(next);
        }
        next[i] = 0; // carry
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
/// opposite polarities never match anyway), plus a 1-device unitization from
/// netlist `w`/`l`/`nf` (or `m`) for every device no unitization covers.
fn with_per_device_sizing(netlist: &Netlist, annot: &Constraints) -> Constraints {
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
            dev_nf: vec![param("nf").max(param("m")).clamp(1, i64::from(u16::MAX)) as u16],
            target_ratio: vec![1],
            unit_w: param("w").clamp(0, i64::from(i32::MAX)) as i32,
            unit_l: param("l").clamp(0, i64::from(i32::MAX)) as i32,
            series_parallel: match dev.kind {
                DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
                _ => SeriesParallel::Parallel,
            },
            same_variant_required: false,
            dummy_required: false,
            route_matching_required: false,
        });
    }
    Constraints {
        unitization,
        ..Default::default()
    }
}

/// Every enumerated variant of one group, by device kind.
fn draw_variants(kind: DeviceKind, group: &DeviceGroup, c: &Constraints, pdk: &Pdk) -> Vec<Macro> {
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => draw_all::<Mosfet>(group, c, pdk),
        DeviceKind::Resistor => draw_all::<Resistor>(group, c, pdk),
        DeviceKind::Capacitor => draw_all::<Capacitor>(group, c, pdk),
        DeviceKind::Diode => draw_all::<Diode>(group, c, pdk),
        DeviceKind::Npn | DeviceKind::Pnp => draw_all::<Bjt>(group, c, pdk),
        DeviceKind::Inductor => draw_all::<Inductor>(group, c, pdk),
    }
}

/// `Cell::enumerate` order. Never empty: an empty enumeration yields one empty
/// macro so `variant == 0` always names something.
fn draw_all<G: Cell>(group: &DeviceGroup, c: &Constraints, pdk: &Pdk) -> Vec<Macro> {
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
        }];
    }
    drawn
}

/// The LVS reference from the schematic, terminals as net names in SPICE card
/// order (MOS `D G S B`, BJT `C B E`, two-terminal `P N`).
///
/// A sized MOS goes in as `max(nf, m)` cards of per-finger `w`/`l` (SI metres):
/// the extractor measures one device per channel and a parametrised device never
/// parallel-merges. Inductors have no recogniser and are skipped. `ports` is left
/// empty for the caller to fill with the labels it actually places.
///
/// ponytail: a unitization that overrides a device's finger count desyncs this
/// expansion from the drawn layout.
pub fn reference(netlist: &Netlist) -> RefInput {
    let mut devices: Vec<RefDeviceIn> = Vec::new();
    for dev in &netlist.devices {
        let (kind, pins): (RefKind, &[&str]) = match dev.kind {
            DeviceKind::Nmos => (RefKind::Nmos, &["D", "G", "S", "B"]),
            DeviceKind::Pmos => (RefKind::Pmos, &["D", "G", "S", "B"]),
            DeviceKind::Resistor => (RefKind::Resistor, &["P", "N"]),
            DeviceKind::Capacitor => (RefKind::Capacitor, &["P", "N"]),
            DeviceKind::Diode => (RefKind::Diode, &["P", "N"]),
            DeviceKind::Npn => (RefKind::Npn, &["C", "B", "E"]),
            DeviceKind::Pnp => (RefKind::Pnp, &["C", "B", "E"]),
            DeviceKind::Inductor => continue,
        };
        let terminals: Vec<String> = pins
            .iter()
            .map(|p| {
                terminal(dev, p).map_or(String::new(), |n| netlist.nets[n.0 as usize].name.clone())
            })
            .collect();
        let param = |k: &str| dev.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
        let (fingers, params) = if matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            let fingers = param("nf")
                .unwrap_or(1)
                .max(param("m").unwrap_or(1))
                .clamp(1, i64::from(u16::MAX));
            let params = ["w", "l"]
                .into_iter()
                .filter_map(|k| param(k).map(|nm| (k.to_string(), nm as f64 * 1e-9)))
                .collect();
            (fingers, params)
        } else {
            (1, Vec::new())
        };
        for _ in 0..fingers {
            devices.push(RefDeviceIn {
                kind,
                model: None,
                terminals: terminals.clone(),
                params: params.clone(),
            });
        }
    }
    RefInput {
        devices,
        ports: Vec::new(),
    }
}

/// Rebind a drawn macro's synthetic pin nets to the schematic's. A pin named
/// `d{k}:T` is terminal `T` of `members[k]`; a bare `T` is member 0.
fn bind_pins(m: &mut Macro, netlist: &Netlist, members: &[DeviceId]) {
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
        if let Some(net) = dev.and_then(|d| terminal(d, term)) {
            pin.net = net;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Net, Pin, Shape};

    /// Load the sky130 deck the benchmarks use. Skips (rather than fails) when the
    /// PDK is absent, so the suite still runs outside the dev shell — same policy as
    /// `cells`' own self-check.
    fn pdk() -> Option<Pdk> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let json = std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?;
        Pdk::from_json(&json).ok()
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
            kind,
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
                    }
                })
                .collect(),
            lock: None,
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
        let Some(pdk) = pdk() else { return };
        let netlist = two_devices();

        let auto = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk);
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
            },
        );
        let mixed = enumerate(&netlist, &injected, &Constraints::default(), &pdk);
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
        let Some(pdk) = pdk() else { return };
        let netlist = two_devices();
        let cells = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk);

        assert_eq!(cells.cell_of, vec![0, 1], "identity map");
        assert_eq!(
            cells.devices_of,
            vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            "singleton members"
        );

        // Byte-identical to drawing each device as its own group, with no collapse
        // machinery (no pad filter, no re-ordering) in the way.
        let sized = with_per_device_sizing(&netlist, &Constraints::default());
        for (i, dev) in netlist.devices.iter().enumerate() {
            let group = DeviceGroup {
                devices: vec![DeviceId(i as u16)],
            };
            let mut expect = draw_variants(dev.kind, &group, &sized, &pdk);
            for m in &mut expect {
                bind_pins(m, &netlist, &group.devices);
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
        let mut seen = vec![vec![0u16, 0, 0]];
        let mut cur = vec![0u16, 0, 0];
        while let Some(next) = escalate(&spaces, &cur) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            seen.push(next.clone());
            cur = next;
            assert!(seen.len() <= 6, "escalate exceeded the space size");
        }
        assert_eq!(seen.len(), 6, "escalate stopped before covering the space");
        assert!(
            escalate(&spaces, &cur).is_none(),
            "exhaustion must stay exhausted"
        );
        // Nothing to escalate is exhaustion, not a panic.
        assert!(escalate(&[], &[]).is_none());
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
            kind: DeviceKind::Nmos,
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
                same_variant_required: true,
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
        let Some(pdk) = pdk() else { return };
        let netlist = matched_mirror();
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&[0, 1], DeviceKind::Nmos, 2),
            &pdk,
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
            kind: DeviceKind::Nmos,
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
        let Some(pdk) = pdk() else { return };
        let netlist = matched_quad();
        let members = [0u16, 1, 2, 3];
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit_nf(&members, DeviceKind::Nmos, 4),
            &pdk,
        );

        assert_eq!(cells.spaces.len(), 1, "the quad merges into one cell");
        for (v, m) in cells.spaces[0].alternatives.iter().enumerate() {
            assert!(
                shared_pads_carry_one_net(m),
                "alternative {v} shorts two nets on one boundary pad"
            );
            assert!(
                gate_straps_stay_private(m, &netlist, &cells.devices_of[0]),
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

    /// A distinct-gate pair still merges, but only into patterns that do not draw
    /// a short: multi-finger interleaves would run one device's gate strap through
    /// the other's stubs (see `gate_straps_stay_private`), so every surviving
    /// alternative keeps one finger per device — two diffusion regions each.
    #[test]
    fn a_distinct_gate_pair_merges_without_gate_shorting_patterns() {
        let Some(pdk) = pdk() else { return };
        let netlist = matched_pair(); // G nets 1 and 2 — distinct
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
        );

        assert_eq!(cells.spaces.len(), 1, "the pair still merges");
        assert!(!cells.spaces[0].alternatives.is_empty());
        for (v, m) in cells.spaces[0].alternatives.iter().enumerate() {
            assert!(
                gate_straps_stay_private(m, &netlist, &cells.devices_of[0]),
                "alternative {v} straps across a foreign gate"
            );
            // One finger per device ⇒ two S/D regions per device. (Gate pins no
            // longer count fingers — there is one per device by construction.)
            for (d, s) in region_spans(m, 2).iter().enumerate() {
                let n = m
                    .pins
                    .iter()
                    .filter(|p| {
                        p.name.starts_with(&format!("d{d}:"))
                            && (p.name.ends_with(":S") || p.name.ends_with(":D"))
                    })
                    .count();
                assert!(s.is_some(), "alternative {v}: member d{d} has no S/D pin");
                assert_eq!(
                    n, 2,
                    "alternative {v}: a strapped multi-finger pattern survived"
                );
            }
        }
    }

    /// A `macro_master` macro is the user's geometry: it is never redrawn, so it can
    /// never join a merged stack — the whole unitization stays per-device.
    #[test]
    fn an_injected_member_declines_the_merge() {
        let Some(pdk) = pdk() else { return };
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
            },
        );
        let cells = enumerate(
            &netlist,
            &injected,
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
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
        let Some(pdk) = pdk() else { return };
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
        );
        assert_eq!(
            cells.spaces.len(),
            2,
            "different source nets must not share diffusion"
        );
        assert_eq!(cells.cell_of, vec![0, 1]);
    }

    /// D7's round trip over a *merged* space: `realize` hands back the named merged
    /// alternative, and `escalate` walks the joint space without repeats and
    /// exhausts — a merged cell is one odometer digit like any other.
    #[test]
    fn realize_and_escalate_round_trip_over_a_merged_space() {
        let Some(pdk) = pdk() else { return };
        let netlist = matched_pair();
        let cells = enumerate(
            &netlist,
            &Macros::default(),
            &matched_unit(&[0, 1], DeviceKind::Nmos),
            &pdk,
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
        while let Some(next) = escalate(spaces, &cur) {
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
        let Some(pdk) = pdk() else { return };
        let netlist = two_devices();
        let cells = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk);
        let layers = pdk.routing_layers();
        let a = seed_assignment(&cells.spaces, &layers, &pdk);
        let b = seed_assignment(&cells.spaces, &layers, &pdk);
        assert_eq!(a, b);
        assert_eq!(a.len(), cells.spaces.len());
        for (i, &v) in a.iter().enumerate() {
            assert!(
                usize::from(v) < cells.spaces[i].alternatives.len(),
                "cell {i} seeded out of range"
            );
        }
    }
}
