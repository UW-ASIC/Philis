//! BJT generator: emitter block inside a collector diff ring, base poly bar
//! into the emitter, footprint marker; the PNP also gets its n-well base and
//! well tie.

use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, LayerId, Macro, Process, Rect};

use crate::builder::{pin, req, sizing, unitization, Builder, Sizing};
use crate::Cell;

/// One BJT variant: the group's unit devices arrayed in `columns`. Area ratio
/// comes from unit count, never emitter scaling.
#[derive(Clone)]
pub struct Bjt {
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
            .map(|columns| Bjt { columns })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        let diff = req(process, "diff");
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = process.layer("licon").unwrap_or(diff);
        let ct = process.rule("contact", 170);
        let li_enc = process.rule("li_encloses_licon", 80);

        let (emitter_w, emitter_h, base_w0, collector_w0) = dims(&s, process);
        let max_stripe = process.rule("bjt_max_emitter_stripe", 25_000);
        let stripe_gap = process.rule("bjt_stripe_gap", 200);
        let n_stripes = ((emitter_w + max_stripe - 1) / max_stripe).max(1);
        let stripe_w = emitter_w / n_stripes;

        // Terminals must extract as three distinct nets: keep collector ring and
        // emitter block separated by base_w, base poly overlapping both by `ct`.
        let collector_w = collector_w0.max(2 * ct);
        // `base_w` is the ring-to-emitter diff gap: it must clear diff spacing.
        let diff_space = process.rule("DIFF.3", 270);
        let base_w = base_w0.max(ct).max(diff_space);
        let is_pnp = device_is_pnp(group, constraints);
        let coll_w = emitter_w + 2 * base_w + 2 * collector_w;
        let coll_h = emitter_h + 2 * base_w + 2 * collector_w;
        let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
        // The base pad hangs `ext` below the ring; its cut must clear the ring
        // diff by `polycon_to_diff_spacing` or it lands on the collector.
        let ext = ct + process.rule("polycon_to_diff_spacing", 190);
        let well_enc = process.rule("well_enclosure", 300);
        // A PNP's n-well reaches `well_enc + 160` left, `well_enc` right and
        // above, `ext + well_enc` below its ring. Neighbouring wells must stay
        // `nwell_min_spacing` apart: merged wells would short the bases.
        let device_gap = process.rule("device_gap", 600);
        let (gap_x, gap_y) = if is_pnp {
            let nw_space = process.rule("nwell_min_spacing", 1270);
            (
                device_gap.max(2 * well_enc + 160 + nw_space),
                device_gap.max(ext + 2 * well_enc + nw_space),
            )
        } else {
            (device_gap, device_gap)
        };
        let cut_enc = 40; // licon.5 diff-past-cut margin

        for di in 0..n_dev {
            let dev_x = (di as i32 % cols) * (coll_w + gap_x);
            let dev_y = (di as i32 / cols) * (coll_h + gap_y);
            let (cx, cy) = (dev_x, dev_y);

            // Emitter FIRST: the recogniser is [poly, diff, diff] = B, E, C and
            // fills the diff slots in drawing order.
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
            b.pin(pin(di, "E", Rect { x: e_x, y: e_y, w: ct, h: ct }, li));

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
            b.pin(pin(di, "C", Rect { x: c_x, y: c_y, w: ct, h: ct }, li));
            // One cut only: a left-end cut would crowd the PNP well tie.
            b.rect(li, Rect { x: cx + cut_enc, y: c_y, w: coll_w - 2 * cut_enc, h: ct });

            // Base: a poly bar from the B pad below the ring up into the
            // emitter block — the marker's one poly terminal.
            let bar_top = emitter_y + emitter_h / 2 - ct / 2 - cut_enc;
            b.rect(poly, Rect { x: emitter_x, y: cy - ext, w: ct, h: bar_top - (cy - ext) });
            // Poly skirt enclosing the B cut (one-side rule on the sides and
            // below, symmetric rule above).
            let pol_enc = process.rule("poly_encloses_licon", 50);
            let pol_side = process.rule("poly_encloses_licon_one_side", 80).max(pol_enc);
            b.rect(poly, Rect {
                x: emitter_x - pol_side,
                y: cy - ext - pol_side,
                w: ct + 2 * pol_side,
                h: ct + pol_side + pol_enc,
            });
            contact(&mut b, li, licon, emitter_x, cy - ext, ct, li_enc);
            b.pin(pin(di, "B", Rect { x: emitter_x, y: cy - ext, w: ct, h: ct }, li));

            // Marker over the whole footprint: magic reads NPNID/PNPID as
            // regions, and this deck's recogniser still sees exactly the base
            // bar, emitter and collector ring under it.
            let marker = if is_pnp { process.layer("pnp") } else { process.layer("npn") };
            if let Some(marker) = marker {
                b.rect(marker, Rect { x: cx, y: cy, w: coll_w, h: coll_h });
            }

            // Only the PNP gets an n-well (its base). sky130's NPN needs a deep
            // n-well this deck lacks, so the NPN here is a lateral device in the
            // p-substrate; a plain n-well would put it in the wrong body.
            if is_pnp {
                let (wy0, wy1) = (cy - ext - well_enc, cy + coll_h + well_enc);
                if let Some(nwell) = process.layer("nwell") {
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
                // Well tie: a tap strip left of the ring, bridged on li to the
                // B pad (the well is the base) along the pad row, clear of the
                // ring.
                if let Some(tap) = process.layer("tap") {
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
                // N+ implant over the tap strip only (over the ring it would
                // slice a diff polygon and stamp a phantom gate under the base
                // bar). 20 short of the ring, 400 wide for NSDM.1.
                if let Some(nsdm) = process.layer("nsdm") {
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

/// `(emitter_w, emitter_h, base_w, collector_w)` from the unit geometry.
fn dims(s: &Sizing, process: &dyn Process) -> (i32, i32, i32, i32) {
    let min_side = process.rule("bjt_min_emitter_side", 420);
    let ct = process.rule("contact", 170);
    let emitter_w = s.unit_w.max(min_side);
    let emitter_h = s.unit_l.max(min_side);
    // base/collector fractions were floats (0.3 / 0.5); read as per-mille rules.
    let base_frac = process.rule("bjt_base_frac_permille", 300);
    let coll_frac = process.rule("bjt_collector_frac_permille", 500);
    let base_w = ((emitter_w as i64 * i64::from(base_frac) / 1000) as i32).max(ct);
    let collector_w = ((emitter_w as i64 * i64::from(coll_frac) / 1000) as i32).max(2 * ct);
    (emitter_w, emitter_h, base_w, collector_w)
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def = process.rule("bjt_min_emitter_side", 420);
    sizing(group, c, def, def)
}

/// A cut plus its enclosing li pad: the deck joins conductors only through
/// drawn cuts, so every terminal needs one.
fn contact(b: &mut Builder, li: LayerId, licon: LayerId, x: i32, y: i32, ct: i32, li_enc: i32) {
    b.rect(li, Rect { x: x - li_enc, y: y - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });
    b.rect(licon, Rect { x, y, w: ct, h: ct });
}

/// PNP vs NPN from the unitization's `device_type` (no unitization: NPN).
fn device_is_pnp(group: &DeviceGroup, c: &Constraints) -> bool {
    unitization(group, c).is_some_and(|u| u.device_type == DeviceKind::Pnp)
}
