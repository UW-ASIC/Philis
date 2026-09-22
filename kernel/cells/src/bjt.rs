//! BJT generator. Ported from `backend/cells/src/generators/bjt.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::builder::{layer, req, rule, sizing, unitization, Builder, Sizing};
use crate::Cell;

/// One point in the **BJT variant space**: concentric emitter/base/collector
/// rings, optionally arrayed for area ratio (bandgap `Q1:Q8`). Ratio comes from
/// unit-device count, not emitter scaling, to keep matching. Theory: AOAL ch08;
/// `docs/cells/bjt.md`.
///
/// `unit_count` is the number of unit devices in the array; `columns` (the old
/// `BjtSpec` axis) is the array aspect and derives the row count.
#[derive(Clone)]
pub struct Bjt {
    pub unit_count: u16,
    pub guard_ring: bool,
    pub columns: u16,
}

impl Cell for Bjt {
    fn enumerate(group: &DeviceGroup, _constraints: &Constraints, _process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let n = group.devices.len().max(1) as u16;
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        cols.into_iter()
            .map(|columns| Bjt { unit_count: n, guard_ring: false, columns })
            .collect()
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        let s = group_sizing(group, &Constraints::default(), process);
        let (emitter_w, emitter_h, base_w, collector_w) = dims(&s, process);
        let ct = rule(process, "contact", 170);
        let base_w = base_w.max(ct);
        let collector_w = collector_w.max(2 * ct);
        let per_dev = emitter_w + 2 * base_w + 2 * collector_w;
        let n = group.devices.len() as i32;
        let cols = i32::from(self.columns.max(1)).min(n.max(1));
        let rows = (n + cols - 1) / cols;
        let device_gap = rule(process, "device_gap", 600);
        let total_w = per_dev * cols + device_gap * (cols - 1).max(0);
        let cell_h = emitter_h + 2 * base_w + 2 * collector_w;
        let total_h = cell_h * rows + device_gap * (rows - 1).max(0);
        (total_w, total_h)
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        (0..group.devices.len())
            .flat_map(|i| [port(i, "E"), port(i, "B"), port(i, "C")])
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        let diff = req(process, "diff");
        let poly = req(process, "poly");
        let li = req(process, "li");
        // Same fallback as `mosfet.rs`: a deck without a licon role draws the cut on
        // the host layer rather than silently omitting it.
        let licon = layer(process, "licon").unwrap_or(diff);
        let ct = rule(process, "contact", 170);
        let li_enc = rule(process, "li_encloses_licon", 80);

        let (emitter_w, emitter_h, base_w0, collector_w0) = dims(&s, process);
        let max_stripe = rule(process, "bjt_max_emitter_stripe", 25_000);
        let stripe_gap = rule(process, "bjt_stripe_gap", 200);
        let n_stripes = ((emitter_w + max_stripe - 1) / max_stripe).max(1);
        let stripe_w = emitter_w / n_stripes;

        // Terminals must extract as three distinct nets: keep collector ring and
        // emitter block separated by base_w, base poly overlapping both by `ct`.
        let collector_w = collector_w0.max(2 * ct);
        // `base_w` is the collector-ring-to-emitter gap, and both sides are diff, so
        // it has to clear diff-to-diff spacing — not just fit a contact. At `ct`
        // (170) every device shipped four DIFF.3 violations, one per emitter side.
        let diff_space = rule(process, "DIFF.3", 270);
        let base_w = base_w0.max(ct).max(diff_space);
        let device_gap = rule(process, "device_gap", 600);
        let is_pnp = device_is_pnp(group, constraints);

        for di in 0..n_dev {
            let coll_w = emitter_w + 2 * base_w + 2 * collector_w;
            let coll_h = emitter_h + 2 * base_w + 2 * collector_w;
            let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
            let dev_x = (di as i32 % cols) * (coll_w + device_gap);
            let dev_y = (di as i32 / cols) * (coll_h + device_gap);
            let (cx, cy) = (dev_x, dev_y);
            let cut_enc = 40; // licon.5 diff-past-cut margin

            // Emitter FIRST, collector ring second — deliberately. The deck's BJT
            // recogniser is [poly, diff, diff] = Base, Emitter, Collector, and the
            // extractor fills the two diff slots in ascending polygon order, which
            // is drawing order. Emitter drawn first is what makes slot 1 the
            // emitter; swapping these two blocks silently swaps E and C on every
            // extracted BJT.
            let emitter_x = dev_x + collector_w + base_w;
            let emitter_y = dev_y + collector_w + base_w;
            let stripe_pitch = stripe_w + stripe_gap.min(stripe_w / 4);
            let emitter_span = (n_stripes - 1) * stripe_pitch + stripe_w;
            for st in 0..n_stripes {
                b.rect(diff, Rect {
                    x: emitter_x + st * stripe_pitch,
                    y: emitter_y,
                    w: stripe_w,
                    h: emitter_h,
                });
            }
            if n_stripes > 1 {
                b.rect(diff, Rect {
                    x: emitter_x,
                    y: emitter_y + emitter_h / 2 - ct / 2,
                    w: emitter_span,
                    h: ct,
                });
            }
            let (e_x, e_y) =
                (emitter_x + emitter_span - ct - cut_enc, emitter_y + emitter_h / 2 - ct / 2);
            contact(&mut b, li, licon, e_x, e_y, ct, li_enc);
            b.pin(pin_at(di, "E", e_x, e_y, ct, li));

            // Collector: diff ring; side bands run full height so they area-overlap
            // top/bottom (edge-touching rects don't merge into one net).
            b.rect(diff, Rect { x: cx, y: cy, w: coll_w, h: collector_w });
            b.rect(diff, Rect { x: cx, y: cy + coll_h - collector_w, w: coll_w, h: collector_w });
            b.rect(diff, Rect { x: cx, y: cy, w: collector_w, h: coll_h });
            b.rect(diff, Rect { x: cx + coll_w - collector_w, y: cy, w: collector_w, h: coll_h });
            // Collector contacts + strap, both on the TOP band, away from the base
            // stub at the bottom.
            let c_y = cy + coll_h - cut_enc - ct;
            let c_x = cx + coll_w - ct - cut_enc;
            contact(&mut b, li, licon, c_x, c_y, ct, li_enc);
            b.pin(pin_at(di, "C", c_x, c_y, ct, li));
            // One contact at the right end only. A second cut at the left end
            // sat 110 nm from the well-tap's licon (licon min_spacing 170) and
            // grazed its li bridge (li_covers_licon reads a grazed cut as an
            // under-sized contact); the strap conducts fine with one cut.
            b.rect(li, Rect { x: cx + cut_enc, y: c_y, w: coll_w - 2 * cut_enc, h: ct });

            // Base: a vertical poly bar from the B pad below the device up INTO
            // the emitter block, crossing only the bottom collector band. The bar
            // exists to give the marker its one poly terminal, not to span the
            // ring, so it stops just under the emitter's mid-height bridge.
            //
            // The B contact sits `ct + 190` below the ring so its licon CLEARS the
            // bottom band: a licon is a diff↔li via, and the extractor counts a
            // cut *touching* a conductor as landing on it — at the old `ext = ct`
            // the cut's top edge lay exactly on the band edge, which connected the
            // base pad to the collector ring and shorted B to C on every device
            // (the bjt_mirror "three labels on one net" failure). 190 is the
            // deck's `polycon_to_diff_spacing`: a poly cut keeps that much from
            // any diff.
            let ext = ct + rule(process, "polycon_to_diff_spacing", 190);
            let bar_top = emitter_y + emitter_h / 2 - ct / 2 - cut_enc;
            b.rect(poly, Rect { x: emitter_x, y: cy - ext, w: ct, h: bar_top - (cy - ext) });
            // Poly skirt around the B cut: the bar is cut-sized, so alone it
            // gives zero `poly_encloses_licon` enclosure. 80 on both horizontal
            // sides and below (`poly_encloses_licon_one_side`), the symmetric
            // 50 above.
            let pol_enc = rule(process, "poly_encloses_licon", 50);
            let pol_side = rule(process, "poly_encloses_licon_one_side", 80).max(pol_enc);
            b.rect(poly, Rect {
                x: emitter_x - pol_side,
                y: cy - ext - pol_side,
                w: ct + 2 * pol_side,
                h: ct + pol_side + pol_enc,
            });
            contact(&mut b, li, licon, emitter_x, cy - ext, ct, li_enc);
            b.pin(pin_at(di, "B", emitter_x, cy - ext, ct, li));

            // Recognition marker: the device footprint. It has to cover the
            // whole thing, not just the terminals, because NPNID/PNPID are
            // *region* identifiers, not stencil hints: magic reads the PNP's
            // base as `NWELL and PNPID` (sky130A.tech cifinput), so a marker
            // that only hugs the base bar intersects the well in poly-covered
            // area, leaves no bare `pdiff` inside the region, and no bipolar is
            // recognised at all. Covering the footprint still gives this deck's
            // own `[poly, sd, sd]` recogniser exactly three conductors — the
            // base bar, the emitter block, and the collector ring (`sd = diff
            // NOT poly`; cutting a ring at one point still leaves one piece) —
            // with the emitter in slot 1 because it is drawn first.
            let marker = if is_pnp { layer(process, "pnp") } else { layer(process, "npn") };
            if let Some(marker) = marker {
                b.rect(marker, Rect { x: cx, y: cy, w: coll_w, h: coll_h });
            }

            // The n-well goes on the **PNP**, never the NPN. In a p-substrate
            // process the only bipolar an n-well buys you is the p-type one: p+
            // emitter and p+ collector ring diffused into the well, the well
            // itself the base. sky130's own recogniser says so — magic's
            // cifinput builds the `pnp` (nbase) type as `NWELL and PNPID`
            // (sky130A.tech "layer pnp NWELL,... / and PNPID") and extracts
            // `sky130_fd_pr__pnp_05v5` from `pnp *pdiff pwell,space/w`.
            //
            // The NPN gets no well at all. sky130's NPN is a *deep*-n-well
            // device: `layer npn DNWELL / and-not NWELL / and NPNID`, extracted
            // as `npn *ndiff dnwell space/w`. A plain n-well is not a weaker
            // version of that — the recogniser subtracts it — and this deck has
            // no `dnwell` layer at all (`pdks/sky130.json` "layers"), so the
            // NPN here is a lateral device in the p-substrate and its base is
            // the substrate.
            //
            // This was inverted: the NPN drew a well over its whole ring while
            // the PNP drew none, so all five of bjt_mirror's in-well `diff`
            // rects belonged to the one device that must not have a well, and
            // the device that needs one had its emitter sitting in bare
            // substrate. Same shape as the `Ecgr`/`Hcgr` guard-ring inversion
            // in `post_cell::in_nwell`, and invisible for the same reason: the
            // deck has no channel-vs-well polarity rule.
            if is_pnp {
                let well_enc = rule(process, "well_enclosure", 300);
                // The well has to reach past the base tie, which hangs `ext`
                // below the ring to meet the B pad.
                let (wy0, wy1) = (cy - ext - well_enc, cy + coll_h + well_enc);
                if let Some(nwell) = layer(process, "nwell") {
                    // 160 extra on the left: the tap strip sits at
                    // `cx - well_enc + 20`, and `nwell_encloses_ntap` wants the
                    // well 180 past it.
                    b.rect(nwell, Rect {
                        x: cx - well_enc - 160,
                        y: wy0,
                        w: coll_w + 2 * well_enc + 160,
                        h: wy1 - wy0,
                    });
                }
                // Base tie: a tap strip inside the well, left of the ring,
                // licon'd to an li bridge that runs *under* the ring to the B
                // pad. The well IS the base, so it rides at the base potential;
                // bridging it to the collector strap (which is what this did
                // while the well was on the NPN and stood in for its collector)
                // shorts B to C through the silicon. The bridge sits at the B
                // pad's row, `ext` below the ring, so it passes clear of every
                // diff band. Without a tap inside the well `erc/floating_nwell`
                // flags it; without the bridge the tap is a floating conductor.
                if let Some(tap) = layer(process, "tap") {
                    let tap_w = ct + 2 * cut_enc;
                    let tap_x = cx - well_enc + 20;
                    let tie_y = cy - ext;
                    b.rect(tap, Rect { x: tap_x, y: tie_y, w: tap_w, h: coll_h + ext });
                    b.rect(licon, Rect { x: tap_x + cut_enc, y: tie_y, w: ct, h: ct });
                    b.rect(li, Rect {
                        x: tap_x + cut_enc - li_enc,
                        y: tie_y - li_enc,
                        w: (emitter_x + ct + li_enc) - (tap_x + cut_enc - li_enc),
                        h: ct + 2 * li_enc,
                    });
                }
                // N+ implant over the tap strip ONLY — never onto the collector
                // ring: an implant slicing partway across a diff polygon is
                // exactly what `nsdm_encloses_ndiff` rejects (the sliced band
                // fragment has zero enclosure at the cut line), covering the
                // whole ring would stamp `pgate` where the base poly crosses it
                // (a phantom MOS), and this deck's ERC never reads
                // implant-typed ties. 20 nm short of the ring, 400 wide for
                // NSDM.1 (380).
                if let Some(nsdm) = layer(process, "nsdm") {
                    let e = 20;
                    b.rect(nsdm, Rect {
                        x: cx - e - 400,
                        y: cy - ext - e,
                        w: 400,
                        h: coll_h + ext + 2 * e,
                    });
                }
            }
        }

        b.finish()
    }
}

/// Emitter/base/collector nominal dimensions from unit geometry (`dims` folds the
/// old `emitter_w = w.max(min_side)` etc.).
fn dims(s: &Sizing, process: &dyn Process) -> (i32, i32, i32, i32) {
    let min_side = rule(process, "bjt_min_emitter_side", 420);
    let ct = rule(process, "contact", 170);
    let emitter_w = s.unit_w.max(min_side);
    let emitter_h = s.unit_l.max(min_side);
    // base/collector fractions were floats (0.3 / 0.5); read as per-mille rules.
    let base_frac = rule(process, "bjt_base_frac_permille", 300);
    let coll_frac = rule(process, "bjt_collector_frac_permille", 500);
    let base_w = ((emitter_w as i64 * i64::from(base_frac) / 1000) as i32).max(ct);
    let collector_w = ((emitter_w as i64 * i64::from(coll_frac) / 1000) as i32).max(2 * ct);
    (emitter_w, emitter_h, base_w, collector_w)
}

fn port(i: usize, term: &str) -> Pin {
    Pin {
        name: format!("d{i}:{term}"),
        net: net_of(i, term),
        at: Rect { x: 0, y: 0, w: 0, h: 0 },
        // Enumeration placeholder: a 0x0 rect is never routed to, so the layer
        // is not a claim about geometry. `ports()` has no `Process` to ask.
        layer: LayerId(0),
    }
}

fn pin_at(i: usize, term: &str, x: i32, y: i32, ct: i32, layer: LayerId) -> Pin {
    Pin {
        name: format!("d{i}:{term}"),
        net: net_of(i, term),
        at: Rect { x, y, w: ct, h: ct },
        layer,
    }
}

fn net_of(i: usize, term: &str) -> NetId {
    let t = match term {
        "E" => 0,
        "B" => 1,
        _ => 2,
    };
    NetId((i as u16).wrapping_mul(4).wrapping_add(t))
}


fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def = rule(process, "bjt_min_emitter_side", 420);
    sizing(group, c, def, def)
}

/// An li pad and the cut under it, co-located with a terminal.
///
/// The deck sets `lvs.cut_required`, so a terminal drawn on bare diff/poly is
/// electrically open no matter how much metal lands on it: extraction joins two
/// conductors only through a drawn cut. Without this the three BJT terminals
/// extracted as floating diff nets, every route landing on them read as
/// `erc/unconnected_pin` (31 of them on `bjt_mirror` alone), and LVS could not
/// match a device whose terminal nets were all wrong. `mosfet.rs` has carried the
/// same pad+cut pair, and the same comment, since it was written.
fn contact(b: &mut Builder, li: LayerId, licon: LayerId, x: i32, y: i32, ct: i32, li_enc: i32) {
    // li must enclose the cut on every side (`li_encloses_licon`); a cut-sized
    // pad reads as zero enclosure and also falls under li min-area.
    b.rect(li, Rect { x: x - li_enc, y: y - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });
    b.rect(licon, Rect { x, y, w: ct, h: ct });
}

/// PNP vs NPN, read from the group's unitization `device_type`.
///
/// The old generator sniffed `model_name.contains("pnp")`, which the pure trait
/// can't reach. It doesn't need to: the annotator already stamps the polarity on
/// `Unitization::device_type`, which is the same channel `sizing` reads W/L from.
/// A group with no unitization keeps the historical NPN default.
fn device_is_pnp(group: &DeviceGroup, c: &Constraints) -> bool {
    unitization(group, c)
        .is_some_and(|u| u.device_type == pnr_core::netlist::DeviceKind::Pnp)
}
