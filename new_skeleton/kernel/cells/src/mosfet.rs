//! MOSFET generator. Ported from `backend/cells/src/generators/mosfet.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::Pattern;

use crate::builder::{layer, req, rule, sizing, Builder, Sizing};
use crate::Cell;

/// One point in the **MOSFET variant space** — pure geometry identity: `nf`
/// fingers arranged by `style`, with `dummies_per_edge` dummy poly for
/// etch/stress matching (WPE/LOD). What's *feasible* and how fingers decompose is
/// derived by [`Cell::enumerate`]/[`Cell::draw`] from the group's
/// [`Constraints`] (matching axis, unitization, pattern), never stored here.
///
/// Theory: ALS §1.1 (device-level placement); AOAL ch08 (matching);
/// `docs/cells/mosfet.md`.
#[derive(Clone)]
pub struct Mosfet {
    pub nf: u16,
    pub style: Pattern,
    pub dummies_per_edge: u8,
}

const MAX_VARIANTS: usize = 16;
/// Dummy counts the placer may choose from. `0` is a real point in the space:
/// "are the dummies worth their area here?" is only an askable question if the
/// generator can draw the device without them. Withheld from any group whose
/// [`analog::cell::Unitization`] sets `dummy_required` — LOD/WPE matching is a
/// hard constraint there, not a trade. Ordered last so the existing variants
/// keep their position under the `MAX_VARIANTS` truncation.
const DUMMY_OPTIONS: [u8; 3] = [1, 2, 0];

impl Cell for Mosfet {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        // A group that declares `dummy_required` does not get to trade its
        // dummies away; everyone else sees the zero option too.
        let min_dummies = u8::from(
            crate::builder::unitization(group, constraints).is_some_and(|u| u.dummy_required),
        );

        let n_dev = group.devices.len();
        let mut specs: Vec<Self> = feasible_styles(n_dev)
            .into_iter()
            .flat_map(|style| {
                feasible_nf(&s, process)
                    .into_iter()
                    // A centroid style is only offered at a finger count that can
                    // actually form one; otherwise `finger_sequence` falls through
                    // to the block order and the variant is a duplicate of
                    // `Single` wearing a misleading label.
                    .filter(move |&nf| {
                        style == Pattern::Single
                            || centroid_sequence(n_dev, usize::from(nf)).is_some()
                    })
                    .flat_map(move |nf| {
                        DUMMY_OPTIONS.iter().filter(move |&&d| d >= min_dummies).map(
                            move |&d| Mosfet { nf, style, dummies_per_edge: d },
                        )
                    })
            })
            .collect();

        // Dedup by (estimated footprint, pattern) — the same key the placer would
        // otherwise anneal over redundantly.
        let mut seen: Vec<((i32, i32), Pattern)> = Vec::new();
        specs.retain(|spec| {
            let key = (est_dims(spec, group, constraints, process), spec.style);
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        specs.truncate(MAX_VARIANTS);
        specs
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        est_dims(self, group, &Constraints::default(), process)
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        // The pure trait has no net table; ports become the routing entry points
        // per device terminal (G/S/D). Positions are filled by `draw`; here we
        // expose the terminal set the placer must route to.
        let mut pins = Vec::new();
        for (i, _d) in group.devices.iter().enumerate() {
            for (t, term) in ["G", "S", "D", "B"].iter().enumerate() {
                pins.push(Pin {
                    name: format!("d{i}:{term}"),
                    net: net_of(i, t),
                    at: Rect { x: 0, y: 0, w: 0, h: 0 },
                    // Enumeration placeholder: 0x0, never routed to.
                    layer: LayerId(0),
                });
            }
        }
        pins
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        // Layer roles. diff/poly/li are mandatory; tap/implant/well are optional
        // (geometry-only decks omit them) — draw with `if let Some`.
        let diff = req(process, "diff");
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = layer(process, "licon").unwrap_or(poly);
        let matched = matched(group, constraints);
        let is_pmos = matches!(device_kind(group, constraints), Some(pnr_core::DeviceKind::Pmos));

        let ct = rule(process, "contact", 170);
        let poly_ext = rule(process, "poly_ext", 130);
        // Poly enclosure of the contact cut (sky130 licon.5c), from the deck.
        let licon_poly_enc = rule(process, "licon_poly_enc", 50);
        // One side of each axis must give more (deck `poly_encloses_licon_one_side`),
        // and the cut must keep its distance from any diff (`polycon_to_diff_spacing`).
        let licon_poly_side =
            rule(process, "poly_encloses_licon_one_side", licon_poly_enc).max(licon_poly_enc);
        let polycon_gap = rule(process, "polycon_to_diff_spacing", 0);
        // li enclosure of the contact cut, straight off the deck's rule ids: a
        // symmetric `min_enclosure` demands `li_enc` on every side, an
        // `asymmetric_enclosure` demands `li_side` on one side of each axis.
        // Absent rule = 0 = the old cut-sized pads. Every li-over-licon pad
        // below extends `li_enc` all round plus `li_side` on the two sides with
        // free space.
        let li_enc = rule(process, "li_encloses_licon", 0);
        let li_side = rule(process, "li_encloses_licon_one_side", 0).max(li_enc);
        let li_space = rule(process, "li_min_spacing", 170);
        // Diff/tap enclosure of a cut landing on it (sky130 licon.5). This used
        // to be a bare 40 in four places below, which is sky130's number: on a
        // deck demanding 70 the tap rail and the end S/D cuts came out short by
        // exactly the difference, on a *device* layer, which is the class of
        // silent wrong geometry `REQUIRED_RULES` exists to stop.
        let diff_enc = rule(process, "diff_encloses_licon", 40);
        let gate_l = s.unit_l;
        // Pitch floor from met1.2 spacing — kills residual m1.2 DRC by construction.
        let m1_pitch = rule(process, "mcon_size", 170)
            + 2 * rule(process, "m1_enc", 60)
            + rule(process, "met1_space", 140);
        // Second floor on sd_w: adjacent S/D pads sit `sd_w + gate_l` apart
        // centre-to-centre, and their enclosure-sized li pads (ct + li_enc +
        // li_side wide) must clear li min-spacing — binding for nf=1, where the
        // two pads face each other across a single gate.
        // Fourth floor: the outermost S/D cut sits at `sd_w / 2` from the diff
        // edge, so the row must be wide enough to give it `diff_enc` there too.
        let sd_w = rule(process, "sd_width", 250)
            .max(m1_pitch - gate_l)
            .max(ct + li_enc + li_side + li_space - gate_l)
            .max(ct + 2 * diff_enc);
        let pitch = (sd_w + gate_l + sd_w).max(m1_pitch);

        let nf = self.nf.max(1);
        let finger_w = s.unit_w;

        let sequence = finger_sequence(n_dev, self.style, nf, &s.dev_nf);

        // LOD moat extension: extend diff past the outer gates on matched groups
        // so SA/SB matches emitted geometry (AOAL ch13 Rule 12).
        // ponytail: the old tier gate (Moderate/Exceptional) isn't reachable — the
        // pure trait carries no MatchingTier — so `matched` picks the moderate moat.
        // Measured both ways on this stack: without the moat ota (93% util)
        // loses its only routing whitespace and overflow explodes ×1700; with
        // it chain4's pins sit deep in blocked interior. The moat stays until
        // placement reserves routing space explicitly — the lesser regression.
        let moat_ext = if matched { rule(process, "lod_moat_ext_moderate", 3000) } else { 0 };
        let diff_x_start = -moat_ext;
        let diff_x_end = sequence.len() as i32 * pitch + moat_ext;
        // One continuous diff row. Extraction still separates S from D: the
        // deck's conducting layer is `sd = diff NOT poly`, so every gate
        // crossing subtracts its channel and splits the row into per-region
        // conductor pieces — the engine's own contract (its channel guard
        // refuses a deck whose MOS marker overlaps live S/D conductor area).
        b.rect(diff, Rect { x: diff_x_start, y: 0, w: diff_x_end - diff_x_start, h: finger_w });

        // Gate x-extent per device, so each device's fingers can be strapped
        // together afterwards. Keyed by device index: in a multi-device ABBA
        // pattern the fingers interleave, and strapping across devices would
        // short two different gates.
        let mut gate_span: std::collections::BTreeMap<usize, (i32, i32)> =
            std::collections::BTreeMap::new();
        // Gate met1 pad row must clear the S/D pad row by met1_space, else
        // small-finger_w cells get diagonal met1 gaps < min_spacing. Independent
        // of which finger we are drawing, so hoisted out of the loop — the gate
        // strap below needs it too.
        let m1_clear = rule(process, "mcon_size", 170)
            + 2 * rule(process, "m1_enc", 60)
            + rule(process, "met1_space", 140);
        // Third floor: with the deck-demanded li enclosure the gate pad's li
        // grows toward the S/D pad row; push the gate cut down until the two li
        // rows clear `li_space`. cy is the S/D cut row (redeclared below).
        let li_floor =
            licon_poly_side - poly_ext + ct + 2 * li_enc + li_space - (finger_w / 2 - ct / 2);
        let stub = 200
            .max(2 * (m1_clear - finger_w / 2 - poly_ext))
            .max(li_floor)
            // polycon_to_diff_spacing: the gate cut's top edge (pad_y +
            // licon_poly_side + ct) must sit `polycon_gap` below the diff row.
            .max(polycon_gap + licon_poly_side + ct - poly_ext);
        for (idx, dev) in sequence.iter().enumerate() {
            let gx = idx as i32 * pitch + sd_w;
            b.rect(poly, Rect { x: gx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
            let Slot::Dev(di) = *dev;

            b.rect(poly, Rect { x: gx, y: -(poly_ext + stub), w: gate_l, h: stub + 30 });
            // Contact stack on the gate stub. Without a cut the gate is a
            // poly-only island: the deck sets `lvs.cut_required`, so a route
            // arriving on li/met1 cannot reach it, and signoff reports
            // `erc/floating_gate` plus an `unconnected_pin` per landing wire.
            // The pad widens to fit the cut when the gate is narrower than
            // contact + 2·one-side enclosure (a min-length device is 150 nm
            // across, the cut is 170; both horizontal sides get the one-side
            // 80 so either satisfies `poly_encloses_licon_one_side`).
            let pad_w = gate_l.max(ct + 2 * licon_poly_side);
            let pad_x = gx + gate_l / 2 - pad_w / 2;
            let pad_y = -(poly_ext + stub);
            // Vertically: `licon_poly_side` below the cut (free space), the
            // symmetric `licon_poly_enc` above it.
            let pad_h = ct + licon_poly_enc + licon_poly_side;
            b.rect(poly, Rect { x: pad_x, y: pad_y, w: pad_w, h: pad_h });
            let cut_x = gx + gate_l / 2 - ct / 2;
            let cut_y = pad_y + licon_poly_side;
            b.rect(licon, Rect { x: cut_x, y: cut_y, w: ct, h: ct });
            // li over the gate cut, to the deck's enclosure demands: `li_enc`
            // all round, `li_side` taken rightward and *downward* — below the
            // poly pad is free space, while upward closes on the S/D li row
            // (that residual gap is what `li_floor` above keeps ≥ li_space).
            b.rect(li, Rect {
                x: cut_x - li_enc,
                y: cut_y - li_side,
                w: ct + li_enc + li_side,
                h: ct + li_enc + li_side,
            });
            // The pin stays cut-sized: the router lands on the cut, not the skirt.
            let gate_pad = Rect { x: cut_x, y: cut_y, w: ct, h: ct };
            // ONE gate pin per *device*, not per finger. The strap below joins
            // every finger of a device into a single poly node, so the extra
            // pins are not extra terminals — they are the same net asked for
            // `nf` times, and the router obliges: on a 5-device nf=2 OTA that is
            // 10 gate landings instead of 5, each wanting its own via stack and
            // access jog in the same die area. The first finger carries it.
            if !gate_span.contains_key(&di) {
                // The pin lands on the li pad, which is what the router can reach.
                b.pin(Pin { name: format!("d{di}:G"), net: net_of(di, 0), at: gate_pad, layer: li });
            }
            let e = gate_span.entry(di).or_insert((gx, gx + gate_l));
            e.0 = e.0.min(gx);
            e.1 = e.1.max(gx + gate_l);
        }

        // **Gate strap.** Every finger above draws its own poly gate and stub;
        // without a rail joining them each finger is an electrically separate
        // gate, which means an `nf=2` transistor is not one device but two
        // independent ones. Extraction sees exactly that — one device per gate
        // crossing, unmergeable because parallel reduction keys on the gate net —
        // so a 5-device OTA extracted as 12.
        //
        // The rail runs along the bottom of the stub row, spanning only the
        // fingers of a single device so an interleaved ABBA pair keeps its two
        // gates distinct.
        let poly_w = rule(process, "poly_min_width", 150);
        for (_di, (x0, x1)) in &gate_span {
            if x1 - x0 <= gate_l {
                continue; // single finger: the stub is already the whole gate
            }
            b.rect(poly, Rect { x: *x0, y: -(poly_ext + stub), w: x1 - x0, h: poly_w });
        }

        // One contact pad per S/D diffusion region. ABBA multi-device layouts put
        // D on even fingers so shared boundaries between different devices are
        // always sources (same net).
        let multi = n_dev > 1;
        let term_of = |idx: i32| -> &'static str {
            match (multi, idx % 2 == 0) {
                (false, true) | (true, false) => "S",
                _ => "D",
            }
        };
        let n_fingers = sequence.len() as i32;
        let cy = finger_w / 2 - ct / 2;
        let region_cx = |r: i32| -> i32 {
            if r == 0 {
                sd_w / 2
            } else if r == n_fingers {
                (n_fingers - 1) * pitch + sd_w + gate_l + sd_w / 2
            } else {
                (2 * r - 1) * pitch / 2 + sd_w + gate_l / 2
            }
        };
        for r in 0..=n_fingers {
            let left = (r > 0).then(|| sequence[(r - 1) as usize]).and_then(Slot::dev);
            let right = (r < n_fingers).then(|| sequence[r as usize]).and_then(Slot::dev);
            if left.is_none() && right.is_none() {
                continue;
            }
            let cx = region_cx(r);
            let px = cx - ct / 2;
            // li over the S/D cut: `li_enc` all round, `li_side` up (inside the
            // diff finger) and right (over the gate poly — a different plane,
            // and the gate's own li row sits far below).
            b.rect(li, Rect {
                x: px - li_enc,
                y: cy - li_enc,
                w: ct + li_enc + li_side,
                h: ct + li_enc + li_side,
            });
            // The cut under the pad. The deck sets `lvs.cut_required`, so without
            // it this diffusion is electrically open: every finger extracts as a
            // separate series device instead of merging in parallel, and every
            // route landing on the li pad reads as `erc/unconnected_pin`.
            b.rect(licon, Rect { x: px, y: cy, w: ct, h: ct });
            let right_pin = right.map(|di| (di, term_of(r).to_string()));
            let left_pin = left.map(|di| {
                let t = if term_of(r - 1) == "S" { "D" } else { "S" };
                (di, t.to_string())
            });
            if let Some((di, t)) = &right_pin {
                b.pin(pin_at(*di, t, px, cy, ct, li));
            }
            if let Some((di, t)) = &left_pin {
                if right_pin.as_ref() != Some(&(*di, t.clone())) {
                    b.pin(pin_at(*di, t, px, cy, ct, li));
                }
            }
        }

        // (The old big-diff li_chain chunks are gone: they fed the retired
        // engine's centre-sampled missing_tie check, this deck declares no such
        // rule, and cut-less li over diff is exactly what the new engine's
        // floating_interconnect ERC flags — 12 of them on `pair` alone.)

        // Dummy gates (AOAL ch13 Rule 12: dummies sit in cutoff, tied to the bulk
        // rail below via a top poly stub + licon riser).
        // Riser margins: 30 was the historic skirt; the deck's enclosure floors it.
        let rise_l = 30.max(li_enc); // left + bottom
        let rise_r = rise_l.max(li_side); // one horizontal side must make `li_side`
        let tap_y0 = finger_w + 580; // DIFF.3 (270) + dummy-cut clearance
        let tap_h = ct + 2 * diff_enc; // LICON.5: diff extends past each cut edge
        // The riser cut's poly skirt below it: the stub base sits at
        // `finger_w + poly_ext - 10`, so a deck with a long endcap and a wide
        // poly-over-cut enclosure pushes the cut up rather than overhanging.
        let licon_y = finger_w + 240.max(poly_ext - 10 + licon_poly_side);
        // Riser-cut x extents per edge: the edge's dummies share ONE li strip.
        // Separate per-column risers sat `gate_l + sd_w` apart — with the deck's
        // enclosure skirt that gap falls under li min-spacing, and since both
        // columns join the same bulk rail it reads as an li *notch*.
        let mut edge_cuts: [Option<(i32, i32)>; 2] = [None, None];
        for k in 0..i32::from(self.dummies_per_edge) {
            // Outside the diffusion moat, not inside it. A dummy column that
            // lands between `diff_x_start` and `diff_x_end` is poly over diff —
            // i.e. a real gate crossing — so extraction reports one extra
            // transistor per dummy with no schematic counterpart (8 of them on
            // the ota fixture). On a matched group the moat runs `moat_ext`
            // (3000 nm) past the outer gate, which swallowed every dummy at the
            // old `(k + 1) * (gate_l + sd_w)` offset. `sd_w` of clearance keeps
            // the pair symmetric about the moat edges.
            let step = gate_l + sd_w;
            let dummy_positions = [diff_x_start - (k + 1) * step, diff_x_end + sd_w + k * step];
            for (edge, dx) in dummy_positions.into_iter().enumerate() {
                b.rect(poly, Rect { x: dx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
                let cx = dx + gate_l / 2 - ct / 2;
                b.rect(licon, Rect { x: cx, y: licon_y, w: ct, h: ct });
                let e = edge_cuts[edge].get_or_insert((cx, cx));
                e.0 = e.0.min(cx);
                e.1 = e.1.max(cx);
            }
        }
        // ONE poly skirt per edge, spanning that edge's cuts — the same merge the
        // li strip below already does, and for the same reason. A per-column pad
        // has to be `ct + 2 * licon_poly_side` (330 nm on sky130) to enclose its
        // cut, while consecutive columns are only `gate_l + sd_w` (430 nm) apart:
        // the pads ended up 100 nm apart against a 210 nm `poly_min_spacing`. So
        // *every* `dummies_per_edge = 2` variant of every device was DRC-dirty,
        // on both edges, and the placer could pick one — the old self-check drew
        // `variants.first()` only, so it never saw it. Merging is free: both
        // columns tie to the same bulk rail, so the joined poly is not a new
        // connection, and at one dummy per edge it draws the identical rect.
        let stub_top = licon_y + ct + licon_poly_enc;
        let dpad_w = gate_l.max(ct + 2 * licon_poly_side);
        for (cx0, cx1) in edge_cuts.into_iter().flatten() {
            // Cuts are centred on their column, so growing the span by half a pad
            // either side reproduces the per-column skirt at its two ends.
            let x0 = cx0 + ct / 2 - dpad_w / 2;
            b.rect(poly, Rect {
                x: x0,
                y: finger_w + poly_ext - 10,
                w: (cx1 + ct / 2 + dpad_w / 2) - x0,
                h: stub_top - (finger_w + poly_ext - 10),
            });
        }
        // The li riser strips. Bottom edge laps the bulk rail's li by 40 nm but
        // stays ABOVE the rail's cut row: a strip that dips into it overlaps a
        // tap cut by a sliver, which the `li_covers_licon` overlap rule reads as
        // an under-sized (non-conducting) contact.
        let rail_li_y = (tap_y0 + diff_enc) - li_enc;
        for (x0, x1) in edge_cuts.into_iter().flatten() {
            b.rect(li, Rect {
                x: x0 - rise_l,
                y: licon_y - rise_l,
                w: (x1 + ct + rise_r) - (x0 - rise_l),
                h: (rail_li_y + 40) - (licon_y - rise_l),
            });
        }

        // Bulk tap strip: n+ tap in the nwell for PMOS, p+ substrate tie for NMOS.
        // Span the dummies at their new outside-the-moat positions, or the moat
        // itself when there are none.
        // `dpe * (gate_l + sd_w)` on each side is exactly the outer dummy column's
        // far edge (left: `-(k+1)*step` at `k = dpe-1`; right: `+sd_w + k*step +
        // gate_l` at the same `k`), and it is the same expression `est_dims` uses,
        // so the reserved and drawn footprints cannot drift. The old right-hand
        // form spelled it out with a `(dpe - 1).max(0)` clamp that guarded the
        // *multiplier* but left the `+ sd_w + gate_l` unconditional: identical for
        // `dpe >= 1`, but at `dpe = 0` it hung the tap (and, for a PMOS, the nwell
        // derived from it) 600 nm past the right moat edge and flush with the left
        // — an asymmetry on the one axis a matched device is drawn to keep.
        let dummy_span = i32::from(self.dummies_per_edge) * (gate_l + sd_w);
        let tap_x0 = diff_x_start - dummy_span;
        let tap_x1 = diff_x_end + dummy_span;
        let tap_w = tap_x1 - tap_x0;
        // The body/well tap is not optional: without it the well floats, which is
        // a latch-up path, and `erc/floating_well` reports it. This was an
        // `if let Some(..)` and sky130's deck has no layer literally named `tap`
        // (its role maps onto `diff`), so every cell silently drew no tap at all.
        b.rect(req(process, "tap"), Rect { x: tap_x0, y: tap_y0, w: tap_w, h: tap_h });
        let imp = if is_pmos { req(process, "nsdm") } else { req(process, "psdm") };
        // Deck `nsdm_encloses_ndiff`/`psdm_encloses_pdiff`: implant ≥125 past the
        // diff it dopes. NSDM.1/PSDM.1 min width 380 still holds: tap_h(250) + 2*125.
        let imp_enc = 125;
        b.rect(imp, Rect {
            x: tap_x0 - imp_enc,
            y: tap_y0 - imp_enc,
            w: tap_w + 2 * imp_enc,
            h: tap_h + 2 * imp_enc,
        });

        // **Channel implant.** The strip above dopes the bulk *tap*, and carries
        // the opposite polarity to the device by design (an n+ tap in the nwell
        // of a PMOS). The device's own source/drain diffusion needs the matching
        // polarity — psdm over a PMOS channel, nsdm over an NMOS one.
        //
        // This is not cosmetic: it is the only thing that tells an extractor
        // which *kind* of transistor a gate-over-diffusion crossing is. Without
        // it LVS aborts with "gate polygon crossing channel polygon has no
        // matching MOS type implant" before comparing anything, which is why
        // every circuit in the suite reported an LVS mismatch identically.
        // `cells/tests/cell_selfcheck.rs` pins the invariant on the generator.
        let chan_imp = if is_pmos { layer(process, "psdm") } else { layer(process, "nsdm") };
        if let Some(chan_imp) = chan_imp {
            b.rect(chan_imp, Rect {
                x: diff_x_start - imp_enc,
                y: -imp_enc,
                w: (diff_x_end - diff_x_start) + 2 * imp_enc,
                h: finger_w + 2 * imp_enc,
            });
        }
        let mut lx = tap_x0 + diff_enc;
        let guard_pitch = rule(process, "guard_licon_pitch", 340);
        let mut kept: Option<(i32, i32)> = None;
        // The riser strips lap the rail (they must merge into one conductor),
        // and on a deck with no symmetric enclosure the rail's bottom edge *is*
        // the cut row — so a cut under a riser strip is grazed by it, which the
        // li-covers-licon overlap rule reads as an under-sized contact. Skip
        // those columns; the strip's own dummy cut ties that stretch already.
        let riser_span = |cx0: i32, cx1: i32| (cx0 - rise_l, cx1 + ct + rise_r);
        while lx + ct + diff_enc <= tap_x1 {
            let under_riser = edge_cuts
                .iter()
                .flatten()
                .map(|&(x0, x1)| riser_span(x0, x1))
                .any(|(r0, r1)| lx <= r1 && r0 <= lx + ct);
            if !under_riser {
                b.rect(licon, Rect { x: lx, y: tap_y0 + diff_enc, w: ct, h: ct });
                let k = kept.get_or_insert((lx, lx));
                k.1 = lx;
            }
            lx += guard_pitch;
        }
        // One solid li rail over the cut row. li_chain's chunking fed the old
        // engine's centre-sampled tie check; the rewrite measures exact
        // geometry, and a chunk seam under a cut clips its host, which
        // `li_encloses_licon*` reads as zero enclosure. Sides meet the deck:
        // `li_enc` all round every kept cut, `li_side` above the row (free
        // space over the tap strip); the span runs the whole strip so the rail
        // overlaps — and merges with — the riser strips at both ends.
        let (first_lx, last_lx) = kept.unwrap_or((tap_x0 + diff_enc, tap_x0 + diff_enc));
        b.rect(li, Rect {
            x: (first_lx - li_enc).min(tap_x0),
            y: (tap_y0 + diff_enc) - li_enc,
            w: (last_lx + ct + li_side).max(tap_x1) - (first_lx - li_enc).min(tap_x0),
            h: ct + li_enc + li_side,
        });
        // Bulk pins: one per device, on the rail li over the cut row. The rail
        // used to be a passive island — no pin, so no route ever reached it, and
        // the whole tap/dummy/riser assembly extracted as one floating
        // conductor (`erc/floating_interconnect`, 4+ components per cell) with
        // the bulk unbiased. A `d{i}:B` pin gives `rebind` the handle to put the
        // schematic bulk net (VSS/VDD) on it, and the router does the rest.
        for di in 0..n_dev {
            let xi = (first_lx + di as i32 * guard_pitch).min(last_lx);
            b.pin(pin_at(di, "B", xi, tap_y0 + diff_enc, ct, li));
        }

        // WPE clearance: inflate nwell (and, historically, the bbox) by the
        // tier-keyed gate-to-well-edge clearance (AOAL ch13 Rule 19). ponytail: no
        // MatchingTier in the pure trait — `matched` ⇒ moderate halo, else none.
        let wpe_halo = if matched { rule(process, "wpe_clearance_moderate", 3000) } else { 0 };
        if is_pmos {
            if let Some(nwell) = layer(process, "nwell") {
                let nw_enc = rule(process, "nwell_diff_enc", 180) + wpe_halo;
                let nw_min = rule(process, "nwell_min_width", 840);
                let mut w = tap_w + 2 * nw_enc;
                let mut h = (tap_y0 + tap_h) + 2 * nw_enc;
                let mut x = tap_x0 - nw_enc;
                let mut y = -nw_enc;
                if w < nw_min {
                    x -= (nw_min - w) / 2;
                    w = nw_min;
                }
                if h < nw_min {
                    y -= (nw_min - h) / 2;
                    h = nw_min;
                }
                b.rect(nwell, Rect { x, y, w, h });
            }
        }
        // The old builder padded the bbox by `wpe_halo` without emitting a rect (a
        // marker on a real layer trips its min-width rule). The new Builder derives
        // the bbox purely from shapes. ponytail: add a Builder::pad_bbox knob if the
        // planner needs the WPE halo reflected in the bbox for non-PMOS cells too.

        b.finish()
    }
}

/// A finger slot in the active row: a real device index. Edge dummies are drawn
/// separately (they never enter `sequence`), so a slot is always a device — the
/// wrapper keeps the port faithful to the old `Vec<String>` shape and leaves room
/// for a future dummy-in-row variant.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Dev(usize),
}

impl Slot {
    fn dev(self) -> Option<usize> {
        let Slot::Dev(i) = self;
        Some(i)
    }
}

/// Deterministic net id for device `i`'s terminal ordinal `t` (0=G,1=S,2=D).
/// The pure trait has no net table; a stable synthetic id keeps distinct
/// terminals distinct for downstream routing.
fn net_of(i: usize, t: usize) -> NetId {
    NetId((i as u16).wrapping_mul(8).wrapping_add(t as u16))
}

fn pin_at(di: usize, term: &str, x: i32, y: i32, ct: i32, layer: LayerId) -> Pin {
    let t = match term {
        "S" => 1,
        "D" => 2,
        "B" => 3,
        _ => 0,
    };
    Pin {
        name: format!("d{di}:{term}"),
        net: net_of(di, t),
        at: Rect { x, y, w: ct, h: ct },
        layer,
    }
}


fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def_w = rule(process, "min_finger_width", 420);
    // Nominal channel L default: the min contacted gate. ponytail: exact L rides
    // in on the unitization constraint; this only backstops an un-matched device.
    let def_l = rule(process, "min_gate_l", 150);
    sizing(group, c, def_w, def_l)
}

fn matched(group: &DeviceGroup, c: &Constraints) -> bool {
    // A group is "matched" when the annotator attached a [`Unitization`] covering
    // it — that constraint is what now carries the matched device set (the old
    // `Constraints.groups`/`GroupConstraint` model is gone). A multi-device unit
    // ⇒ a real matched group ⇒ moderate LOD moat / WPE halo below.
    crate::builder::unitization(group, c).is_some_and(|u| u.devices.len() > 1)
}

fn device_kind(group: &DeviceGroup, c: &Constraints) -> Option<pnr_core::DeviceKind> {
    crate::builder::unitization(group, c).map(|u| u.device_type)
}

fn feasible_styles(n_devices: usize) -> Vec<Pattern> {
    let mut styles = vec![Pattern::Single];
    if n_devices >= 2 {
        // Interdig (ABAB) shorts different drain nets at B-A boundaries on shared
        // diffusion; only the centroid orders are safe. Whether a given `nf` can
        // actually form one is `centroid_sequence`'s call, and `enumerate` filters
        // on it — a style with no feasible finger count never reaches the placer.
        styles.push(Pattern::Cc1d);
        styles.push(Pattern::Cc2d);
    }
    styles
}

/// The **common-centroid finger order** for `n_dev` devices at `nf` fingers each,
/// or `None` when that shape has none.
///
/// Two hard requirements, and they fight each other:
///
/// 1. *Common centroid.* Every device's fingers must average to the same
///    position, so the order has to be mirror-symmetric about the array centre.
/// 2. *No drawn short.* A boundary between two **different** devices must land on
///    a source region. `draw`'s `term_of` makes region `r` a drain when `r` is
///    even and a source when it is odd, and region `r` sits between fingers
///    `r - 1` and `r`, so the device may only change at odd `r` — that is,
///    `seq[1] == seq[2]`, `seq[3] == seq[4]`, and so on. A pattern that breaks
///    this ties two drains together on one diffusion; DRC sees one legal
///    rectangle and says nothing, which is the failure mode the old
///    `feasible_styles` comment named for `Interdig`.
///
/// Fingers therefore come in fixed pairs, with the two array *ends* left over as
/// singletons. Mirror symmetry maps the pair structure onto itself and forces the
/// two ends onto the same device, so that device gets `2 + 4k` fingers and every
/// other device gets `4k` — whence the arithmetic below:
///
/// * `n_dev == 2` — the classic `ABBA` repeated; works at any even `nf`, because
///   B can take the single self-symmetric middle pair.
/// * `n_dev > 2` — only one device can own the middle pair, so all the others
///   need whole mirror-orbits of two pairs: `nf` must be a multiple of 4. The
///   unit is `A BB CC .. A A .. CC BB A`, four fingers per device, repeated
///   `nf / 4` times (the repeat is safe: the unit ends and starts on A, so the
///   seam pair matches, and concatenating centroid-symmetric blocks keeps every
///   centroid at the joint centre).
///
/// A quad thus needs `nf >= 4`; at `nf = 1` or `2` there is simply no
/// centroid-symmetric order that also keeps drains apart, and `Single` is the
/// honest answer rather than a pattern that draws a short.
fn centroid_sequence(n_dev: usize, nf: usize) -> Option<Vec<usize>> {
    if n_dev < 2 || nf == 0 || nf % 2 != 0 {
        return None;
    }
    if n_dev == 2 {
        return Some([0, 1, 1, 0].into_iter().cycle().take(2 * nf).collect());
    }
    if nf % 4 != 0 {
        return None;
    }
    let mut unit = vec![0usize];
    for d in 1..n_dev {
        unit.extend([d, d]);
    }
    unit.extend([0, 0]);
    for d in (1..n_dev).rev() {
        unit.extend([d, d]);
    }
    unit.push(0);
    Some(unit.into_iter().cycle().take(n_dev * nf).collect())
}

/// The finger count(s) this group may be drawn with: **the schematic's**, and
/// only that.
///
/// This used to also offer refolds — the same total width redrawn as 2 / 4 / 8
/// narrower fingers — and that is a genuinely useful axis, but the flow cannot
/// honour it end to end and the halves disagreed in two places at once:
///
/// * `draw` took `finger_w = s.unit_w` per finger, so it read `nf` as a
///   *multiplier*, not a split: an `nf = 4` variant of a `W = 2 µm` device drew
///   four 2 µm fingers, an 8 µm transistor.
/// * `cellgen::reference` emits one reference card per **schematic** finger, so
///   even a correctly refolded layout is compared finger-for-finger against a
///   card count that never moves. Fixing `draw` alone still leaves 4 drawn
///   fingers facing 1 card.
///
/// Either way signoff reports `lvs.unpaired_device` — `Q&A.md` records `chain4`
/// doing exactly that at seed 1, and notes that seed 42 keeping `nf = 1` "is the
/// only reason that fixture is clean". A variant space whose soundness depends
/// on the placer never picking most of it is not a variant space; it is a
/// landmine. So the refolds come out until `reference` can be told the drawn
/// finger count (it needs the chosen variant, which `VariantSpace` discards when
/// it stores drawn `Macro`s — the real fix, and a bigger one than this).
///
/// `process` stays in the signature: the PDK finger-width window is what a
/// restored refold has to consult.
fn feasible_nf(s: &Sizing, _process: &dyn Process) -> Vec<u16> {
    vec![s.dev_nf.first().copied().unwrap_or(1).max(1)]
}

fn est_dims(spec: &Mosfet, group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> (i32, i32) {
    let s = group_sizing(group, c, process);
    let gate_l = s.unit_l;
    let m1_pitch =
        rule(process, "mcon_size", 170) + 2 * rule(process, "m1_enc", 60) + rule(process, "met1_space", 140);
    // Mirror of `draw`'s sd_w, li-pad floor included — est and drawn pitch must
    // not drift apart or the placer reserves the wrong footprint.
    let ct = rule(process, "contact", 170);
    let li_enc = rule(process, "li_encloses_licon", 0);
    let li_side = rule(process, "li_encloses_licon_one_side", 0).max(li_enc);
    let li_space = rule(process, "li_min_spacing", 170);
    let sd_w = rule(process, "sd_width", 250)
        .max(m1_pitch - gate_l)
        .max(ct + li_enc + li_side + li_space - gate_l)
        .max(ct + 2 * rule(process, "diff_encloses_licon", 40));
    let pitch = (2 * sd_w + gate_l).max(m1_pitch);
    let per_dev = i32::from(spec.nf.max(1));
    let seq_len = per_dev * group.devices.len().max(1) as i32;
    let dummy_span = i32::from(spec.dummies_per_edge) * (gate_l + sd_w);
    let moat = rule(process, "lod_moat_ext_moderate", 3000);
    let w = seq_len * pitch + 2 * dummy_span + 2 * moat;
    let finger_w = s.unit_w;
    // +900: bulk tap strip zone above the fingers (580 gap + 250 tap + enc).
    (w, finger_w + 2 * rule(process, "poly_ext", 130) + 900)
}

/// Finger sequence: device slots (with dummies filtered by the caller), matching
/// the old `finger_sequence_ext`. Unequal per-device counts route through a
/// greedy centroid interleave.
fn finger_sequence(n_dev: usize, style: Pattern, nf_per_device: u16, dev_nf: &[u16]) -> Vec<Slot> {
    if dev_nf.len() == n_dev && dev_nf.iter().any(|&x| x != dev_nf[0]) {
        let counts: Vec<usize> = dev_nf.iter().map(|&x| x as usize).collect();
        return greedy_centroid(&counts);
    }
    let nf = nf_per_device as usize;
    match (style, n_dev) {
        (Pattern::Single, _) => (0..n_dev)
            .flat_map(|d| std::iter::repeat_n(Slot::Dev(d), nf))
            .collect(),
        (Pattern::Interdig, 2) => (0..nf).flat_map(|_| [Slot::Dev(0), Slot::Dev(1)]).collect(),
        (Pattern::Cc1d | Pattern::Cc2d, n) if centroid_sequence(n, nf).is_some() => {
            centroid_sequence(n, nf).unwrap_or_default().into_iter().map(Slot::Dev).collect()
        }
        _ => {
            let counts: Vec<usize> = (0..n_dev).map(|_| nf).collect();
            let total: usize = counts.iter().sum();
            let mut seq = Vec::with_capacity(total);
            let mut c = counts;
            while seq.len() < total {
                if let Some(i) = c.iter().position(|&x| x > 0) {
                    seq.push(Slot::Dev(i));
                    c[i] -= 1;
                } else {
                    break;
                }
            }
            seq
        }
    }
}

/// Greedy centroid interleave for unequal finger counts (verbatim algorithm from
/// `greedy_centroid_mosfet`): fill outside-in, always picking the device with the
/// most remaining fingers so the group stays centroid-symmetric.
fn greedy_centroid(counts: &[usize]) -> Vec<Slot> {
    let total: usize = counts.iter().sum();
    if total == 0 {
        return vec![];
    }
    let mut remaining = counts.to_vec();
    let mut seq = vec![0usize; total];
    let mut lo = 0usize;
    let mut hi = total - 1;
    while lo <= hi {
        let pick = remaining
            .iter()
            .enumerate()
            .filter(|(_, &r)| r > 0)
            .max_by_key(|(_, &r)| r)
            .map(|(i, _)| i)
            .unwrap_or(0);
        seq[lo] = pick;
        remaining[pick] -= 1;
        if lo < hi {
            if remaining[pick] > 0 {
                seq[hi] = pick;
                remaining[pick] -= 1;
            } else {
                let pick2 = remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, &r)| r > 0)
                    .max_by_key(|(_, &r)| r)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                seq[hi] = pick2;
                remaining[pick2] = remaining[pick2].saturating_sub(1);
            }
            if hi == 0 {
                break;
            }
            hi -= 1;
        }
        lo += 1;
    }
    seq.into_iter().map(Slot::Dev).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two properties [`centroid_sequence`] exists to guarantee, checked over
    /// every shape it admits. The second one is the load-bearing half: a pattern
    /// that puts two different devices either side of an even (drain) region
    /// draws a short across shared diffusion that DRC cannot see and only LVS
    /// finds, five stages downstream.
    #[test]
    fn a_centroid_order_is_symmetric_and_never_abuts_two_drains() {
        for n_dev in 2..=6usize {
            for nf in 1..=12usize {
                let Some(seq) = centroid_sequence(n_dev, nf) else { continue };
                assert_eq!(seq.len(), n_dev * nf, "n={n_dev} nf={nf}: wrong finger count");

                for d in 0..n_dev {
                    assert_eq!(
                        seq.iter().filter(|&&x| x == d).count(),
                        nf,
                        "n={n_dev} nf={nf}: device {d} got the wrong share of fingers"
                    );
                    // Centroid: sum of positions, compared as `2 * sum` so the
                    // half-integer centre stays in integers.
                    let sum: usize = seq.iter().enumerate().filter(|(_, &x)| x == d).map(|(i, _)| i).sum();
                    assert_eq!(
                        2 * sum,
                        nf * (seq.len() - 1),
                        "n={n_dev} nf={nf}: device {d}'s centroid is off the array centre"
                    );
                }

                // Region `r` is a drain when `r` is even, and sits between
                // fingers `r - 1` and `r`.
                for r in (2..seq.len()).step_by(2) {
                    assert_eq!(
                        seq[r - 1], seq[r],
                        "n={n_dev} nf={nf}: fingers {} and {r} are different devices across a \
                         DRAIN region — that is a drawn short: {seq:?}",
                        r - 1
                    );
                }
            }
        }
    }

    /// A quad has no centroid order below four fingers a side, and `Single` is
    /// the honest answer there — pinned so a future "generalisation" that quietly
    /// returns a non-centroid order for `nf = 2` fails here rather than in LVS.
    #[test]
    fn a_quad_needs_four_fingers_before_a_centroid_exists() {
        assert!(centroid_sequence(4, 1).is_none());
        assert!(centroid_sequence(4, 2).is_none());
        assert!(centroid_sequence(4, 4).is_some());
        // A pair is the exception: B can take the single self-symmetric middle.
        assert!(centroid_sequence(2, 2).is_some());
        assert!(centroid_sequence(2, 1).is_none());
    }
}
