//! Cell generation + the LVS reference — the two places the flow touches `cells`
//! and the schematic side.

use std::collections::HashMap;

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

/// The collapsed cell table: the variant spaces `gp`/`dp` search, plus the two
/// maps that relate the **cell space** they index to the netlist's **device
/// space**.
///
/// After group collapse (PLAN §2) a cell is no longer one device: a matched
/// unitization becomes ONE placeable cell whose alternatives are merged
/// interdigitated stacks, so `n_cells ≤ n_devices` and every consumer that used
/// to assume "cell index == device index" must translate through these maps.
pub struct Cells {
    /// One variant space per **cell**, in cell order.
    pub spaces: Vec<gp::VariantSpace>,
    /// `cell_of[device] = cell index` — the map every device-indexed table
    /// (`fixed`, groups, guard rings, rule targets) is pushed through.
    pub cell_of: Vec<u16>,
    /// `devices_of[cell]` = member devices in **generator draw order**, i.e.
    /// `devices_of[c][di]` is the device the `d{di}:` pin ordinal names (see
    /// `cells::mosfet::net_of` / `pin_at`). Single-device cells hold one entry.
    pub devices_of: Vec<Vec<DeviceId>>,
}

/// Enumerate and draw **every legal variant** of every cell — the search space
/// `gp`/`dp` choose from, and the table `realize` indexes.
///
/// This replaced the old `generate`, which picked the smallest-footprint variant here
/// and returned one `Macro` per device. That single `min_by_key(w*h)` discarded the
/// whole variant degree of freedom before placement ever ran, and footprint area is
/// the wrong criterion anyway: the cost of a variant is where its *pins* end up, which
/// nothing at this point in the flow can know.
///
/// A **user-injected** `macro_master` macro registered under the device's instance
/// name short-circuits enumeration: it becomes a single-alternative space, because
/// a variant the user drew is not ours to reconsider.
///
/// **Group collapse (PLAN §2) happens here.** A multi-member unitization becomes
/// ONE cell drawn through `cells`' merged path (`mosfet`'s ABBA `finger_sequence`),
/// so same-variant, integer-ratio and common-centroid hold **by construction**:
/// one variant index cannot disagree with itself, `dev_nf` decomposes the ratio
/// into identical unit fingers, and the interleaving is centroid-balanced by the
/// pattern. This is also what retires `VariantSpace::lock` — the lock only existed
/// because a differential pair used to be two cells with two independent indices.
///
/// A unitization **declines** to merge (falls back to per-device cells, exactly as
/// before) when:
/// - it has an **injected** member — a `macro_master` macro is the user's geometry
///   and is never redrawn, so it cannot join a merged stack;
/// - it **overlaps** an earlier unitization (first-wins, with a stderr line) — a
///   device can be drawn once;
/// - its MOS members do **not share one source net**: the merged `finger_sequence`
///   puts every inter-device diffusion boundary on S, so differing S nets would be
///   a drawn short that DRC cannot see (the diffusion is one legal rectangle) and
///   only LVS would catch at signoff;
/// - **every** drawn alternative shares a boundary pad between two different nets
///   (the same drawn-short hazard on the pattern axis: `Single`/greedy-centroid
///   sequences can put a D|D boundary between devices whose drains differ). Unsafe
///   alternatives are dropped per-alternative; an empty space declines the merge.
///
/// Everything not merged is a single-device cell **exactly as today** — a netlist
/// with no matched groups produces the identity map and byte-identical spaces.
#[must_use]
pub fn enumerate(netlist: &Netlist, macros: &Macros, constraints: &Constraints, pdk: &Pdk) -> Cells {
    // Every device draws at its real size: matched devices keep the annotator's
    // group-scoped unit geometry (found by subset lookup), and every un-matched
    // device gets a synthesised 1-device unitization from its own netlist w/l/nf.
    // Without this, un-matched devices fall back to PDK-minimum geometry.
    let sized = with_per_device_sizing(netlist, constraints);
    let n = netlist.devices.len();

    // ── Phase 1: decide (and draw) the merges ──────────────────────────────
    // Decided against the post-split unitizations, so each candidate is
    // single-kind by construction — asserted, not trusted, because a mixed-kind
    // merge would draw every member as whichever kind the group is labelled and
    // nothing downstream could detect it.
    let mut unit_of: Vec<Option<usize>> = vec![None; n];
    let mut merged: Vec<Option<(Vec<DeviceId>, Vec<Macro>)>> = Vec::new();
    for u in &sized.unitization {
        let members: Vec<DeviceId> =
            u.devices.iter().copied().filter(|d| (d.0 as usize) < n).collect();
        if members.len() < 2 {
            continue;
        }
        let kinds: Vec<DeviceKind> =
            members.iter().map(|d| netlist.devices[d.0 as usize].kind).collect();
        assert!(
            kinds.windows(2).all(|w| w[0] == w[1]),
            "cellgen: unitization {:?} spans device kinds {kinds:?} — \
             with_per_device_sizing was supposed to have split this",
            u.devices
        );
        // A macro_master macro is the user's geometry; it never joins a merged stack.
        if members.iter().any(|d| macros.get(&netlist.devices[d.0 as usize].name).is_some()) {
            continue;
        }
        // First-wins on overlap: a device can be drawn once. Later claims lose loudly.
        if members.iter().any(|d| unit_of[d.0 as usize].is_some()) {
            eprintln!(
                "[collapse] unitization {:?} overlaps an earlier one — first wins, \
                 this one stays per-device",
                u.devices.iter().map(|d| d.0).collect::<Vec<_>>()
            );
            continue;
        }
        // MOS shared-source pre-check: the ABBA sequence puts every inter-device
        // diffusion boundary on S, so differing S nets would be a drawn short no
        // DRC rule can see. Decline rather than draw it.
        if matches!(kinds[0], DeviceKind::Nmos | DeviceKind::Pmos) {
            let s_net = |d: &DeviceId| {
                netlist.devices[d.0 as usize]
                    .terminals
                    .iter()
                    .find(|(t, _)| t == "S")
                    .map(|(_, n)| *n)
            };
            let first = s_net(&members[0]);
            if first.is_none() || members.iter().any(|d| s_net(d) != first) {
                eprintln!(
                    "[collapse] unitization {:?} has members on different source nets \
                     — merging would draw a short on shared diffusion, staying \
                     per-device",
                    u.devices.iter().map(|d| d.0).collect::<Vec<_>>()
                );
                continue;
            }
        }
        // Draw the merged stack through the same generator path as everything
        // else — `Cell::enumerate` on a multi-device group already IS the joint
        // enumeration — then bind pins by `d{N}:` ordinal against the member list.
        let group = DeviceGroup { devices: members.clone() };
        let mut alternatives = draw_variants(kinds[0], &group, &sized, pdk);
        for m in &mut alternatives {
            bind_pins(m, netlist, &members);
        }
        // Drop any alternative the pattern physically shorts:
        // - a shared boundary pad carrying two nets (Single / greedy-centroid
        //   orders can put a D|D boundary between devices whose drains differ);
        // - a gate strap crossing another member's stubs when their gate nets
        //   differ (interleaved patterns; see `gate_straps_stay_private`).
        // The S-net pre-check covers the ABBA diffusion case; these are the same
        // invariant enforced per-alternative, where the pattern is known.
        alternatives.retain(|m| {
            shared_pads_carry_one_net(m) && gate_straps_stay_private(m, netlist, &members)
        });
        if alternatives.is_empty() {
            eprintln!(
                "[collapse] unitization {:?}: every merged pattern draws a short \
                 (shared diffusion boundary or crossing gate straps) — staying \
                 per-device",
                u.devices.iter().map(|d| d.0).collect::<Vec<_>>()
            );
            continue;
        }
        for d in &members {
            unit_of[d.0 as usize] = Some(merged.len());
        }
        merged.push(Some((members, alternatives)));
    }

    // ── Phase 2: emit cells, ordered by first member device index ─────────
    let mut spaces: Vec<gp::VariantSpace> = Vec::new();
    let mut cell_of = vec![0u16; n];
    let mut devices_of: Vec<Vec<DeviceId>> = Vec::new();
    for (i, dev) in netlist.devices.iter().enumerate() {
        let ci = spaces.len() as u16;
        if let Some(ui) = unit_of[i] {
            // `take` emits the merged cell at its lowest-indexed member and makes
            // every later member a no-op — that is the deterministic cell order.
            let Some((members, alternatives)) = merged[ui].take() else { continue };
            for d in &members {
                cell_of[d.0 as usize] = ci;
            }
            spaces.push(gp::VariantSpace { alternatives, lock: None });
            devices_of.push(members);
            continue;
        }
        cell_of[i] = ci;
        devices_of.push(vec![DeviceId(i as u16)]);
        // A user macro for this instance is a **one-element space**: a variant the
        // user drew is not ours to reconsider. Its ports (`G`/`S`/`D`/…) are still
        // remapped onto THIS instance's terminal nets, or the router cannot connect
        // it into the surrounding circuit.
        if let Some(m) = macros.get(&dev.name) {
            let mut m = m.clone();
            for pin in &mut m.pins {
                if let Some((_, net)) = dev.terminals.iter().find(|(t, _)| *t == pin.name) {
                    pin.net = *net;
                }
            }
            spaces.push(gp::VariantSpace { alternatives: vec![m], lock: None });
            continue;
        }
        let group = DeviceGroup { devices: vec![DeviceId(i as u16)] };
        let mut alternatives = draw_variants(dev.kind, &group, &sized, pdk);
        for m in &mut alternatives {
            bind_pins(m, netlist, &group.devices);
        }
        spaces.push(gp::VariantSpace { alternatives, lock: None });
    }
    Cells { spaces, cell_of, devices_of }
}

/// Every set of pins sharing one drawn pad carries one net — the invariant that
/// makes a merged stack's shared diffusion legal rather than a drawn short.
///
/// Two pins land on the same `at` rect exactly when the generator put two
/// devices' terminals on one boundary diffusion region; after [`bind_pins`] their
/// nets are the schematic's, so a disagreement means the pattern physically
/// connects two different nets. That is invisible to DRC (the diffusion is one
/// legal rectangle) and only surfaces as an LVS mismatch at signoff — so it is
/// checked here, where the alternative can simply be dropped.
fn shared_pads_carry_one_net(m: &Macro) -> bool {
    m.pins.iter().enumerate().all(|(i, a)| {
        m.pins[i + 1..].iter().all(|b| a.at != b.at || a.net == b.net)
    })
}

/// No device's gate strap crosses another member's gate stubs on a different net.
///
/// `mosfet::draw` straps a multi-finger device's gates with one continuous poly
/// rail across that device's finger span, on the same row every finger's stub
/// descends to. Its comment says the per-device span "keeps an interleaved ABBA
/// pair's two gates distinct" — but an *interleaved* span contains the other
/// device's fingers, so the rail runs straight through their stubs: one poly net,
/// two schematic gates. DRC cannot object (poly over poly is legal) and the pad
/// check above cannot see it (a strap is not a pin) — the merged 5T-OTA diff pair
/// extracted as ONE device this way.
///
/// Read off the **S/D region x-spans** (`d{N}:S` / `d{N}:D`, still one pin per
/// diffusion region). A device's fingers — and therefore its strap — sit strictly
/// between its own outermost regions, so two members whose region spans overlap by
/// more than the single shared boundary region are interleaved, and the wider
/// member's strap runs through the narrower one's stubs. Block-ordered (AABB)
/// members abut at exactly one region: overlap of zero width, no crossing.
///
/// ponytail: this mirrors the generator's drawing rule rather than extracting
/// connectivity — brittle if `mosfet.rs` changes its strap. The real fix is a
/// strap that jogs around foreign stubs (kernel/cells, outside this change's
/// scope); once that lands this filter stops firing and interleaved
/// distinct-gate merges come back on their own.
fn gate_straps_stay_private(m: &Macro, netlist: &Netlist, members: &[DeviceId]) -> bool {
    let gate_net = |d: &DeviceId| {
        netlist.devices[d.0 as usize].terminals.iter().find(|(t, _)| t == "G").map(|(_, n)| *n)
    };
    let span = region_spans(m, members.len());
    members.iter().enumerate().all(|(a, da)| {
        members.iter().enumerate().skip(a + 1).all(|(b, db)| {
            gate_net(da) == gate_net(db) // same schematic gate: strap contact is harmless
                || match (span[a], span[b]) {
                    (Some((a0, a1)), Some((b0, b1))) => a1.min(b1) <= a0.max(b0),
                    _ => true,
                }
        })
    })
}

/// Per-member `(min, max)` x of the `d{N}:S`/`d{N}:D` pins — the diffusion-region
/// extent of each device inside a (possibly merged) cell.
fn region_spans(m: &Macro, members: usize) -> Vec<Option<(i32, i32)>> {
    let mut span: Vec<Option<(i32, i32)>> = vec![None; members];
    for pin in &m.pins {
        let Some((n, t)) = pin.name.strip_prefix('d').and_then(|r| r.split_once(':')) else {
            continue;
        };
        if t != "S" && t != "D" {
            continue;
        }
        let Some(slot) = n.parse::<usize>().ok().and_then(|i| span.get_mut(i)) else { continue };
        let e = slot.get_or_insert((pin.at.x, pin.at.x));
        e.0 = e.0.min(pin.at.x);
        e.1 = e.1.max(pin.at.x);
    }
    span
}

/// Choose the starting variant per cell by **pricing** each hypothesis.
///
/// PLAN §2 recommends the hybrid, and both pure strategies lose: pricing alone is
/// myopic (BAG-style generate-then-evaluate cannot know pin-position consequences
/// before placement), while leaving variants purely to annealing moves wastes moves
/// on obviously dominated hypotheses. So price to seed, then let `dp` revise.
///
/// The price has two halves and neither is footprint area — see [`price`].
///
/// Determinism: `min_by` returns the *first* minimum, so ties break on index order.
/// This seeds the whole run, and a hash-ordered tie-break would make two runs of the
/// same inputs diverge from epoch zero.
#[must_use]
pub fn seed_assignment(
    variants: &[gp::VariantSpace],
    layers: &[pnr_core::geom::LayerId],
    pdk: &Pdk,
) -> Vec<u16> {
    let cfg = gr::GlobalCfg::default();
    // One in-loop engine session (density stripped, as this pricing always
    // waived it) reused across every alternative of every cell — the deck is
    // parsed once instead of per hypothesis.
    let mut checker = Checker::new(pdk, true)
        .expect("Checker over a loaded Pdk cannot fail to re-parse its own deck");
    variants
        .iter()
        .enumerate()
        .map(|(i, space)| {
            let best = space
                .alternatives
                .iter()
                .enumerate()
                .map(|(v, m)| (v, price(m, layers, &cfg, &mut checker)))
                .min_by(|a, b| a.1.cmp(&b.1));
            match best {
                // Unreachable sorts last, so this only fires when *every* alternative
                // is unreachable — a variant-space binding visible before the first
                // epoch. Seed it anyway (the loop needs a starting assignment) but say
                // so, because the middle tier will stall and the diagnosis is already
                // known here.
                Some((v, (unreachable, ..))) if unreachable => {
                    eprintln!(
                        "[variant] cell {i}: no alternative's pins are reachable in \
                         isolation — seeding {v} and expecting a variant-space binding"
                    );
                    v as u16
                }
                Some((v, _)) => v as u16,
                None => 0,
            }
        })
        .collect()
}

/// One variant hypothesis's price, ordered **worst-last**: `(unreachable, illegal
/// geometry, congestion, wirelength)`.
///
/// Lexicographic for the same reason `lib.rs::lex_key` is: `reachable == false` is
/// *infeasibility*, not expense (D6) — no placement of such a variant helps — so no
/// amount of wirelength saving may buy past it.
type Price = (bool, usize, i64, i64);

/// Price one drawn hypothesis by **measuring** it. Both halves are measurements;
/// neither is a geometric surrogate, because routability and extracted parasitics are
/// *discontinuous* in the variant index (PLAN §2) and no cheap proxy ranks them.
///
/// ponytail: `variant_signoff` in `kernel/macroMaster` is the same DRC/ERC sweep, but
/// it is a `#[cfg(all(test, feature = "gpurify"))]` module and its `verify` dependency
/// is optional precisely so the crate builds with no GPurify checkout. Calling it from
/// here would force `verify` (and hence `gdsverify`) into `macro_master`'s default
/// dependency set to reuse eight lines that `library` can already call directly. Lift
/// this if the two ever need to agree on more than "count the findings".
///
/// ponytail: one DRC + one ERC call per alternative per cell, once per run. Measured on
/// sky130: a MOSFET enumerates **6** alternatives, drawing all of them costs 0.66 ms for
/// two devices, and pricing them costs **~13 ms per hypothesis** — all of it inside
/// `gdsverify`, which is 99.86% of measured runtime. So the seed pass is roughly
/// `13 ms × n_cells × 6`: negligible at OTA sizes (a 5T OTA is ~0.4 s), about a minute
/// at the benchmark's 800-cell ceiling. Named because this is the one place in the flow
/// where oracle calls were *added*; the upgrade is to price DRC/ERC only for the
/// alternatives that survive the routability half, which needs group collapse first to
/// make that half discriminate at all.
fn price(
    m: &Macro,
    layers: &[pnr_core::geom::LayerId],
    cfg: &gr::GlobalCfg,
    checker: &mut Checker,
) -> Price {
    // Routability: actually route the cell's own nets (D6). For a single-device cell
    // this is near-vacuous — `price_group` finds no net with two distinct terminals
    // inside the group and reports `reachable`, `overflow: 0` — and it only starts
    // discriminating once group collapse makes a cell contain several devices. Called
    // regardless, so the pricing path is the one that lands rather than one written
    // alongside collapse later.
    let p = gr::price_group(std::slice::from_ref(m), layers, cfg);
    // Geometry legality against THIS process, one DRC+ERC engine pass. The session
    // was built density-stripped: `min_density`/`max_density` are windowed *fill*
    // rules, a chip-level property no single cell can satisfy, so they are
    // signoff-only and not a per-cell yardstick. The old engine's
    // `erc_extraction_error` pseudo-violation (stage-inapplicable for a bare cell)
    // is now a `StageStatus`, not a violation row, so no filter is needed. A macro
    // the engine cannot even load prices as maximally illegal — fail closed.
    let geom = match checker
        .run(&m.shapes, &[], Checks { drc: true, erc: true, lvs: false, pex: false })
    {
        Ok(_) => checker.outputs().violations.len(),
        Err(_) => usize::MAX,
    };
    (!p.reachable, geom, p.overflow, p.hpwl)
}

/// The geometry an assignment selects: `variants[i].alternatives[assignment[i]]`.
///
/// Hot — called once per epoch and again whenever `dp` reshapes. Cheap by
/// construction (the alternatives are already drawn; this only selects), which is
/// the point of pre-drawing them.
///
/// ponytail: clones. `gdsverify` is 99.86% of measured runtime, so a per-epoch
/// `Vec<Macro>` copy will not show up; hand out `&Macro` only if it does.
#[must_use]
pub fn realize(variants: &[gp::VariantSpace], assignment: &[u16]) -> Vec<Macro> {
    variants
        .iter()
        .enumerate()
        .map(|(i, space)| {
            // A short or absent table reads as variant 0, which is `Layout::variant`'s
            // own rule: an all-zero table *is* the no-variant-search state, so a
            // missing entry is not an error.
            let v = usize::from(assignment.get(i).copied().unwrap_or(0));
            assert!(
                v < space.alternatives.len(),
                "cell {i} names variant {v} but only {} alternatives were drawn — \
                 whoever wrote `variant[{i}]` wrote it against a different space \
                 (docs/API-WISH.md D7)",
                space.alternatives.len()
            );
            space.alternatives[v].clone()
        })
        .collect()
}

/// The **outer-tier move**: a different joint variant assignment to try, or `None`
/// when the collapsed space is exhausted.
///
/// Called only when the middle tier stalled while still infeasible — PLAN §5's
/// variant-space-binding signature. So this is not "explore a bit more", it is "the
/// current variants cannot be made to work; change the problem".
///
/// Mechanism: a **mixed-radix odometer** over the joint space, with the cells whose
/// alternatives actually relocate pins as the *fastest-moving digits*. That gives both
/// properties the caller needs from one structure:
///
/// - **Never repeats, and terminates.** Each call advances the joint assignment by
///   exactly one in a total order, so no assignment is revisited and the all-digits-max
///   state returns `None`. This function is memoryless — the caller keeps no tried-set
///   — so a successor relation is the only way to guarantee that at all.
/// - **Biased toward pin geometry.** The first escalations move the cells ranked
///   highest by [`pin_spread`], which is what a stalled placement needs: a different
///   pin arrangement, not a marginally different footprint. PLAN §2 flags the capacitor
///   case as the sharpest — the three metal-stack constructions are topologically
///   different devices with different pin faces, and a via cut inside a stacked-plate
///   footprint shorts the device, so a move there changes the LVS-legality of the
///   surrounding routing rather than merely its cost.
#[must_use]
pub fn escalate(variants: &[gp::VariantSpace], current: &[u16], seed: u64) -> Option<Vec<u16>> {
    // `seed` is deliberately unused, and this is the one place we do not obey D7's
    // "sample uniformly". The two requirements are incompatible for a memoryless
    // function: guaranteeing "never return an already-tried assignment" forces a
    // successor on a total order, and a successor has no room for a seed. The caller
    // also hands a *different* seed every outer iteration (`cfg.seed ^ outer`), so it
    // could not index a stable permutation even if one existed.
    //
    // ponytail: uniform-without-repeats needs a seeded bijection over the joint index
    // space (cycle-walking Feistel, since `Π n_i` is not a power of two) plus the index
    // recovered from `current`. Worth it only if a real circuit's space is large enough
    // that scanning from the seeded point misses the good region — with `outer_iters`
    // defaulting to 4, only the first few successors are ever reached.
    let _ = seed;

    // Digit significance: most pin movement first, index order to break ties so the
    // escalation sequence is reproducible. Spread is precomputed because `sort_by_key`
    // may call its key function more than once per element and this one allocates.
    let spread: Vec<usize> = variants.iter().map(pin_spread).collect();
    let mut order: Vec<usize> = (0..variants.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(spread[i]), i));

    let mut next: Vec<u16> =
        (0..variants.len()).map(|i| current.get(i).copied().unwrap_or(0)).collect();
    for &i in &order {
        let n = variants[i].alternatives.len();
        if usize::from(next[i]) + 1 < n {
            next[i] += 1;
            return Some(next);
        }
        // Digit exhausted: reset and carry into the next-fastest cell. The mixed-radix
        // value strictly increases across calls, which is the no-repeat guarantee.
        next[i] = 0;
    }
    // Every digit rolled over: the collapsed space is exhausted. That is a real answer
    // — this circuit cannot be laid out under these constraints — not a failure to
    // converge, and `lib.rs` stops rather than spinning on it.
    None
}

/// How much a cell's alternatives move its **pins**: the number of distinct pin
/// arrangements among them.
///
/// Footprint area is explicitly not this ranking. Routability and extracted parasitics
/// are discontinuous in the variant index *because pins jump* (PLAN §2), so a cell
/// whose 16 alternatives all present the same pin faces is worth escalating last
/// however differently they are shaped.
fn pin_spread(space: &gp::VariantSpace) -> usize {
    let mut sigs: Vec<Vec<(i32, i32, i32, i32)>> = space
        .alternatives
        .iter()
        .map(|m| {
            // Relative to the macro's own bbox origin: a variant that merely translates
            // its pins presents the same arrangement, and the placer positions cells by
            // centre anyway. Sorted so pin *order* is not mistaken for pin movement.
            let mut p: Vec<(i32, i32, i32, i32)> = m
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

/// Return the annotator's constraints augmented with a **1-device unitization for
/// every un-matched device**, built from that device's own netlist `w`/`l`/`nf`
/// (`m` as a fallback multiplier). Matched devices are left to the annotator's
/// group-scoped unitization (found by `cells::builder::unitization`'s subset
/// lookup), which carries the *shared* unit geometry that enforces matching.
///
/// Generators read only `Constraints::unitization`; the other structural fields
/// (dummies/guard rings/…) are consumed elsewhere (`post_cell`, from the
/// annotator's own constraints), so they are intentionally dropped from this
/// generator-facing view.
fn with_per_device_sizing(netlist: &Netlist, annot: &Constraints) -> Constraints {
    // Split any unitization whose members are not all the same device kind.
    //
    // A `Unitization` carries ONE `device_type`, and `cells` reads it to decide
    // what to draw — so a group holding both an NMOS and a PMOS silently draws
    // every member as whichever kind the group is labelled. That is how a 5T OTA
    // ended up with its two PMOS loads drawn as NMOS, and extraction then
    // reported `0P` devices against a reference expecting two.
    //
    // Splitting is also the physically correct answer regardless of the bug: unit
    // geometry is shared to make devices *match*, and devices of opposite
    // polarity never match each other.
    let mut unitization: Vec<Unitization> = Vec::new();
    for u in &annot.unitization {
        let kinds: Vec<DeviceKind> = u
            .devices
            .iter()
            .filter_map(|d| netlist.devices.get(d.0 as usize).map(|dev| dev.kind))
            .collect();
        let mixed = kinds.windows(2).any(|w| w[0] != w[1]);
        if !mixed {
            let mut u = u.clone();
            if let Some(&k) = kinds.first() {
                u.device_type = k; // trust the netlist over the annotator's label
            }
            unitization.push(u);
            continue;
        }
        let mut seen: Vec<DeviceKind> = Vec::new();
        for &k in &kinds {
            if seen.contains(&k) {
                continue;
            }
            seen.push(k);
            let members: Vec<DeviceId> = u
                .devices
                .iter()
                .copied()
                .filter(|d| {
                    netlist.devices.get(d.0 as usize).map(|dev| dev.kind) == Some(k)
                })
                .collect();
            let keep: Vec<usize> = u
                .devices
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    netlist.devices.get(d.0 as usize).map(|dev| dev.kind) == Some(k)
                })
                .map(|(i, _)| i)
                .collect();
            unitization.push(Unitization {
                devices: members,
                device_type: k,
                dev_nf: keep.iter().filter_map(|&i| u.dev_nf.get(i).copied()).collect(),
                target_ratio: keep
                    .iter()
                    .filter_map(|&i| u.target_ratio.get(i).copied())
                    .collect(),
                ..u.clone()
            });
        }
    }
    let covered: HashMap<u16, ()> = annot
        .unitization
        .iter()
        .flat_map(|u| u.devices.iter().map(|d| (d.0, ())))
        .collect();
    for (i, dev) in netlist.devices.iter().enumerate() {
        if covered.contains_key(&(i as u16)) {
            continue;
        }
        let param = |k: &str| dev.params.iter().find(|(n, _)| n == k).map_or(0, |(_, v)| *v);
        let w = param("w").clamp(0, i64::from(i32::MAX)) as i32;
        let l = param("l").clamp(0, i64::from(i32::MAX)) as i32;
        let nf = param("nf").max(param("m")).clamp(1, i64::from(u16::MAX)) as u16;
        unitization.push(Unitization {
            devices: vec![DeviceId(i as u16)],
            device_type: dev.kind,
            dev_nf: vec![nf],
            target_ratio: vec![1],
            unit_w: w,
            unit_l: l,
            series_parallel: match dev.kind {
                DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
                _ => SeriesParallel::Parallel,
            },
            // A lone device is not a matched group.
            same_variant_required: false,
            dummy_required: false,
            route_matching_required: false,
        });
    }
    Constraints { unitization, ..Default::default() }
}

/// Draw **every** enumerated variant of one group, dispatched by kind. The old
/// `candidates::<Spec>` dispatch, re-expressed against the pure `Cell` trait.
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

/// Draw the family's whole variant space, in `Cell::enumerate` order — the ordering
/// `Layout::variant` indexes into, deduped and capped (16) by the generator itself.
///
/// This is where the old `min_by_key(w*h)` lived, and dropping it is the point of
/// [`enumerate`]: smallest-area is not a proxy for anything that matters, because the
/// cost of a variant is where its *pins* land and PLAN §2 shows that is discontinuous
/// in the variant index. Nothing at draw time can rank that; pricing measures it.
///
/// An empty enumeration (only reachable for an empty group) yields **one** empty macro
/// rather than an empty space: `realize` walks cell-for-cell, so a zero-alternative
/// space would shift every subsequent cell index, and `variant[i] == 0` must always
/// name something.
fn draw_all<G: Cell>(group: &DeviceGroup, c: &Constraints, pdk: &Pdk) -> Vec<Macro> {
    let drawn: Vec<Macro> =
        G::enumerate(group, c, pdk).iter().map(|v| v.draw(group, c, pdk)).collect();
    if drawn.is_empty() {
        return vec![Macro {
            shapes: Vec::new(),
            pins: Vec::new(),
            bbox: Rect { x: 0, y: 0, w: 0, h: 0 },
        }];
    }
    drawn
}

/// Build the LVS reference from the parsed schematic for signoff, in
/// `verify`'s [`RefInput`] shape.
///
/// Terminals are **net names in SPICE card order** (see `verify::reference`):
/// MOS `[D, G, S, B]` — bulk always stated, the deck recogniser's arity decides
/// whether it is consumed — BJT `[C, B, E]`, R/C/D `[P, N]`.
///
/// `model` stays `None` throughout: the parsed [`Device`] carries no model
/// string (the old path hardcoded `DeviceFlavor::Standard` for the same
/// reason), so the deck's first recogniser of the right kind/polarity is the
/// correct — and only expressible — choice.
///
/// The old `RefNetlist` path parallel-reduced MOS instances here (W×m summed,
/// extractor-matching key) to keep device counts honest; GPurify reduces
/// **both** LVS sides itself — but a device that declares a parameter never
/// merges, so a sized MOS card goes in **one card per drawn finger**
/// (`max(nf, m)`, the same count `constraints()` hands the generator) with the
/// per-finger `w`/`l` in SI metres. The extractor measures one `w`/`l` per
/// channel marker, so sized fingers pair one to one. Non-MOS kinds carry no
/// params: the extractor measures none for them, and a one-sided name is an
/// `lvs.undeclared_param` mismatch by design.
///
/// ponytail: an `interface.json` unitization that overrides a device's finger
/// count desyncs this expansion from the drawn layout — those annotated
/// fixtures are not LVS-clean today for independent reasons; revisit when one
/// is.
///
/// `ports` is left empty: [`crate::signoff`] fills it with the label names it
/// actually places on the drawn geometry, so the two cannot drift.
pub fn reference(netlist: &Netlist) -> RefInput {
    let net_name = |id: NetId| netlist.nets[id.0 as usize].name.clone();
    let term = |dev: &Device, pin: &str| -> String {
        dev.terminals
            .iter()
            .find(|(p, _)| p == pin)
            .map_or(String::new(), |(_, n)| net_name(*n))
    };
    let terms = |dev: &Device, pins: &[&str]| -> Vec<String> {
        pins.iter().map(|p| term(dev, p)).collect()
    };

    let mut devices: Vec<RefDeviceIn> = Vec::new();
    for dev in &netlist.devices {
        let (kind, terminals) = match dev.kind {
            DeviceKind::Nmos => (RefKind::Nmos, terms(dev, &["D", "G", "S", "B"])),
            DeviceKind::Pmos => (RefKind::Pmos, terms(dev, &["D", "G", "S", "B"])),
            DeviceKind::Resistor => (RefKind::Resistor, terms(dev, &["P", "N"])),
            DeviceKind::Capacitor => (RefKind::Capacitor, terms(dev, &["P", "N"])),
            DeviceKind::Diode => (RefKind::Diode, terms(dev, &["P", "N"])),
            DeviceKind::Npn => (RefKind::Npn, terms(dev, &["C", "B", "E"])),
            DeviceKind::Pnp => (RefKind::Pnp, terms(dev, &["C", "B", "E"])),
            // Inductors have no recogniser in any deck; skip here rather than
            // inflate verify's skipped-device log.
            DeviceKind::Inductor => continue,
        };
        let is_mos = matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos);
        let (fingers, params) = if is_mos {
            // Parsed w/l are nanometres; the reference speaks SI metres. The
            // finger count mirrors `constraints()`'s `nf.max(m)` exactly.
            let param = |k: &str| dev.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
            let fingers = param("nf")
                .unwrap_or(1)
                .max(param("m").unwrap_or(1))
                .clamp(1, i64::from(u16::MAX));
            let mut params = Vec::new();
            for name in ["w", "l"] {
                if let Some(nm) = param(name) {
                    params.push((name.to_string(), nm as f64 * 1e-9));
                }
            }
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
    RefInput { devices, ports: Vec::new() }
}

/// Rebind a drawn macro's pins to the **netlist's** nets, group-aware.
///
/// A generator is a pure function of `(variant, group, constraints, process)` and
/// deliberately has no net table, so it labels pins with synthetic ids
/// (`mosfet::net_of` = `device_ordinal*8 + terminal_ordinal`) that only keep one
/// macro's own terminals distinct. Left unbound, routing sees one giant net per
/// terminal role: it shorts every gate together, every source together, and
/// silently drops the real nets for want of terminals.
///
/// The `d{N}:` prefix on a pin name is the generator's **draw ordinal** — an index
/// into `members`, which arrives in the same order the `DeviceGroup` was drawn
/// (`Cells::devices_of` preserves it). A pin without the prefix is ordinal `0`,
/// which is every pin of a single-device cell. The terminal after the `:` is
/// looked up on *that member's* schematic terminals, so a merged pair's `d1:G`
/// binds to the second device's gate net, not the first's.
fn bind_pins(m: &mut Macro, netlist: &Netlist, members: &[DeviceId]) {
    for pin in &mut m.pins {
        // `d1:G` → ordinal 1, term `G`; `G` alone → ordinal 0, term `G`.
        let (ordinal, term) = match pin.name.split_once(':') {
            Some((d, t)) => {
                (d.strip_prefix('d').and_then(|s| s.parse::<usize>().ok()).unwrap_or(0), t)
            }
            None => (0, pin.name.as_str()),
        };
        let Some(dev) = members.get(ordinal).and_then(|d| netlist.devices.get(d.0 as usize))
        else {
            debug_assert!(false, "pin {:?} names draw ordinal {ordinal} but the cell has \
                 only {} members — generator and devices_of disagree", pin.name, members.len());
            continue;
        };
        if let Some((_, net)) = dev.terminals.iter().find(|(t, _)| t == term) {
            pin.net = *net;
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
            .map(|n| Net { name: (*n).to_string() })
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
            params: vec![("w".to_string(), 1000), ("l".to_string(), 210), ("nf".to_string(), 2)],
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
                            rect: Rect { x: 0, y: 0, w, h: 100 },
                        }],
                        // Distinct pin geometry per alternative, so `pin_spread`
                        // reports the real count rather than collapsing them.
                        pins: vec![Pin {
                            name: "G".to_string(),
                            net: NetId(0),
                            at: Rect { x: w - 10, y: 0, w: 10, h: 10 },
                            layer: LayerId(1),
                        }],
                        bbox: Rect { x: 0, y: 0, w, h: 100 },
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
        assert_eq!(realize(&spaces, &[])[0].bbox, spaces[0].alternatives[0].bbox);
    }

    /// Every cell must offer at least one alternative or `realize` shifts cell
    /// indices, and an injected macro must be a *one*-element space — a variant the
    /// user drew is not ours to reconsider.
    #[test]
    fn enumerate_gives_every_cell_a_space_and_pins_injected_ones() {
        let Some(pdk) = pdk() else { return };
        let netlist = two_devices();

        let auto = enumerate(&netlist, &Macros::default(), &Constraints::default(), &pdk);
        assert_eq!(auto.spaces.len(), netlist.devices.len(), "one cell per device");
        for (i, s) in auto.spaces.iter().enumerate() {
            assert!(!s.alternatives.is_empty(), "cell {i} has an empty variant space");
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
                shapes: vec![Shape { layer: LayerId(1), rect: Rect { x: 0, y: 0, w: 9, h: 9 } }],
                pins: vec![Pin {
                    name: "G".to_string(),
                    net: NetId(0),
                    at: Rect { x: 0, y: 0, w: 9, h: 9 },
                    layer: LayerId(1),
                }],
                bbox: Rect { x: 0, y: 0, w: 9, h: 9 },
            },
        );
        let mixed = enumerate(&netlist, &injected, &Constraints::default(), &pdk);
        assert_eq!(mixed.spaces[0].alternatives.len(), 1, "injected macro must be a single-alternative space");
        assert_eq!(mixed.spaces[0].alternatives[0].bbox.w, 9, "injected geometry was redrawn");
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
            let group = DeviceGroup { devices: vec![DeviceId(i as u16)] };
            let mut expect = draw_variants(dev.kind, &group, &sized, &pdk);
            for m in &mut expect {
                bind_pins(m, &netlist, &group.devices);
            }
            assert_eq!(cells.spaces[i].alternatives, expect, "cell {i} drifted from the per-device path");
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
        while let Some(next) = escalate(&spaces, &cur, 7) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            seen.push(next.clone());
            cur = next;
            assert!(seen.len() <= 6, "escalate exceeded the space size");
        }
        assert_eq!(seen.len(), 6, "escalate stopped before covering the space");
        assert!(escalate(&spaces, &cur, 7).is_none(), "exhaustion must stay exhausted");
        // Nothing to escalate is exhaustion, not a panic.
        assert!(escalate(&[], &[], 7).is_none());
    }

    /// Two matched NMOS on a shared source (tail), distinct gates and drains — the
    /// canonical merge candidate. Net ids: tail 0, g1 1, g2 2, d1 3, d2 4.
    fn matched_pair() -> Netlist {
        let nets = ["tail", "g1", "g2", "d1", "d2"]
            .iter()
            .map(|n| Net { name: (*n).to_string() })
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
            params: vec![("w".to_string(), 1000), ("l".to_string(), 210), ("nf".to_string(), 1)],
        };
        Netlist { devices: vec![dev("M1", 3, 1), dev("M2", 4, 2)], nets }
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
        let cells =
            enumerate(&netlist, &Macros::default(), &matched_unit_nf(&[0, 1], DeviceKind::Nmos, 2), &pdk);

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
        assert!(seen.contains(&"d0") && seen.contains(&"d1"), "both members present");

        // At least one alternative interleaves the two devices (ABBA): one
        // member's diffusion regions then sit *inside* the other's span. A
        // `Single` block layout (AABB) only ever abuts, sharing one region.
        // (Read from S/D pins, not gate pins: a device's fingers are strapped
        // into one node and so surface exactly one gate pin each.)
        let interleaved = cells.spaces[0].alternatives.iter().any(|m| {
            match region_spans(m, 2)[..] {
                [Some((a0, a1)), Some((b0, b1))] => a1.min(b1) > a0.max(b0),
                _ => false,
            }
        });
        assert!(interleaved, "no ABBA alternative in the merged space");
    }

    /// Four matched NMOS mirror legs: one shared gate, one shared source,
    /// distinct drains.
    fn matched_quad() -> Netlist {
        let nets = ["tail", "g", "d1", "d2", "d3", "d4"]
            .iter()
            .map(|n| Net { name: (*n).to_string() })
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
            params: vec![("w".to_string(), 1000), ("l".to_string(), 210), ("nf".to_string(), 4)],
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
                    span.iter().flatten().all(|&(b0, b1)| a1.min(b1) > a0.max(b0))
                })
        });
        assert!(interleaved, "no common-centroid alternative in the merged quad space");
    }

    /// A distinct-gate pair still merges, but only into patterns that do not draw
    /// a short: multi-finger interleaves would run one device's gate strap through
    /// the other's stubs (see `gate_straps_stay_private`), so every surviving
    /// alternative keeps one finger per device — two diffusion regions each.
    #[test]
    fn a_distinct_gate_pair_merges_without_gate_shorting_patterns() {
        let Some(pdk) = pdk() else { return };
        let netlist = matched_pair(); // G nets 1 and 2 — distinct
        let cells =
            enumerate(&netlist, &Macros::default(), &matched_unit(&[0, 1], DeviceKind::Nmos), &pdk);

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
                assert_eq!(n, 2, "alternative {v}: a strapped multi-finger pattern survived");
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
                shapes: vec![Shape { layer: LayerId(1), rect: Rect { x: 0, y: 0, w: 9, h: 9 } }],
                pins: vec![Pin {
                    name: "G".to_string(),
                    net: NetId(0),
                    at: Rect { x: 0, y: 0, w: 9, h: 9 },
                    layer: LayerId(1),
                }],
                bbox: Rect { x: 0, y: 0, w: 9, h: 9 },
            },
        );
        let cells = enumerate(&netlist, &injected, &matched_unit(&[0, 1], DeviceKind::Nmos), &pdk);
        assert_eq!(cells.spaces.len(), 2, "injected member must keep the pair per-device");
        assert_eq!(cells.cell_of, vec![0, 1]);
        assert_eq!(cells.spaces[0].alternatives.len(), 1, "M1 is still the user's macro");
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
        let cells =
            enumerate(&netlist, &Macros::default(), &matched_unit(&[0, 1], DeviceKind::Nmos), &pdk);
        assert_eq!(cells.spaces.len(), 2, "different source nets must not share diffusion");
        assert_eq!(cells.cell_of, vec![0, 1]);
    }

    /// D7's round trip over a *merged* space: `realize` hands back the named merged
    /// alternative, and `escalate` walks the joint space without repeats and
    /// exhausts — a merged cell is one odometer digit like any other.
    #[test]
    fn realize_and_escalate_round_trip_over_a_merged_space() {
        let Some(pdk) = pdk() else { return };
        let netlist = matched_pair();
        let cells =
            enumerate(&netlist, &Macros::default(), &matched_unit(&[0, 1], DeviceKind::Nmos), &pdk);
        let spaces = &cells.spaces;
        assert_eq!(spaces.len(), 1);
        let depth = spaces[0].alternatives.len();
        assert!(depth >= 2, "a merged pair should still have a real variant space");

        for v in 0..depth as u16 {
            let got = realize(spaces, &[v]);
            assert_eq!(got.len(), 1);
            assert_eq!(got[0], spaces[0].alternatives[v as usize]);
        }

        let mut cur = vec![0u16];
        let mut seen = vec![cur.clone()];
        while let Some(next) = escalate(spaces, &cur, 7) {
            assert!(!seen.contains(&next), "escalate repeated {next:?}");
            seen.push(next.clone());
            cur = next;
        }
        assert_eq!(seen.len(), depth, "escalate must cover exactly the merged space");
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

