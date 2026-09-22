//! Resistor generator: each device a series chain of `segments` poly
//! resistors (gap between two contacted poly heads under an `rpm` marker).

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{greedy_centroid, pin, req, sizing, Builder, Sizing};
use crate::{Cell, Pattern};

/// One resistor variant: body split into `segments` series segments, devices
/// laid out `Single` (each device's segments adjacent) or `Interdig`.
#[derive(Clone)]
pub struct Resistor {
    pub segments: u16,
    pub pattern: Pattern,
}

impl Cell for Resistor {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let patterns = if group.devices.len() > 1 {
            vec![Pattern::Single, Pattern::Interdig]
        } else {
            vec![Pattern::Single]
        };
        feasible_segments(&s, process)
            .into_iter()
            .flat_map(|segments| {
                patterns.iter().map(move |&pattern| Resistor { segments, pattern })
            })
            // Interleaved multi-segment devices would chain their segments
            // with li jumpers across a neighbour's column: a short.
            .filter(|r| !(matches!(r.pattern, Pattern::Interdig) && r.segments > 1))
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        // The body is a gap between two poly heads under an `rpm` marker (the
        // recogniser: rpm over [poly, poly]); a continuous bar would be a wire.
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = process.layer("licon").unwrap_or(poly);
        let rpm = process.layer("rpm");

        let ct = process.rule("contact", 170);
        let li_enc = process.rule("li_encloses_licon", 80);
        // Head long enough for the one-side poly enclosure above and below.
        let head_l = process.rule("res_head", 300)
            .max(ct + 2 * process.rule("poly_encloses_licon_one_side", 80));
        let seg_gap = process.rule("res_seg_gap", 400);
        let n_segments = i32::from(self.segments.max(1));
        let body_w = s.unit_w;
        let body_l = s.unit_l;
        // The gap floors at poly spacing (nothing measures L).
        let seg_l = (body_l / n_segments).max(process.rule("poly_min_spacing", 210));
        let seg_pitch = body_w + seg_gap;
        let total_h = head_l + seg_l + head_l;
        let cut_enc = 40;

        let sequence = res_segment_sequence(n_dev, self.pattern, n_segments);
        let mut dev_seg_placed = vec![0i32; n_dev];
        let mut p_at = vec![vec![(0i32, 0i32); n_segments as usize]; n_dev];
        let mut n_at = vec![vec![(0i32, 0i32); n_segments as usize]; n_dev];

        for &di in &sequence {
            let slot = dev_seg_placed.iter().sum::<i32>();
            let seg_idx = dev_seg_placed[di];
            dev_seg_placed[di] += 1;

            let flip = seg_idx % 2 == 1;
            let sx = slot * seg_pitch;

            // Two poly end pads; the gap between them is the resistive body.
            b.rect(poly, Rect { x: sx, y: 0, w: body_w, h: head_l });
            b.rect(poly, Rect { x: sx, y: head_l + seg_l, w: body_w, h: head_l });
            // Marker over the gap, lapping into both heads. A single segment
            // widens it to `rpm_min_width`; multi-segment markers would merge
            // across columns if widened. ponytail: so multi-segment bodies
            // narrower than rpm_min_width still violate it.
            if let Some(rpm) = rpm {
                let rpm_min = process.rule("rpm_min_width", 1270);
                let w = if n_segments == 1 { body_w.max(rpm_min) } else { body_w };
                b.rect(rpm, Rect {
                    x: sx - (w - body_w) / 2,
                    y: head_l - cut_enc,
                    w,
                    h: seg_l + 2 * cut_enc,
                });
            }

            let cy_top = head_l / 2 - ct / 2;
            let cy_bot = head_l + seg_l + head_l / 2 - ct / 2;
            let cx = body_w / 2 - ct / 2;

            let (tx, ty) = my(cx, cy_top, ct, total_h, flip);
            b.rect(licon, Rect { x: sx + tx, y: ty, w: ct, h: ct });
            b.rect(li, Rect { x: sx + tx - li_enc, y: ty - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });

            let (bx2, by2) = my(cx, cy_bot, ct, total_h, flip);
            b.rect(licon, Rect { x: sx + bx2, y: by2, w: ct, h: ct });
            b.rect(li, Rect { x: sx + bx2 - li_enc, y: by2 - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });

            // Odd segments are mirrored: they enter at the bottom.
            p_at[di][seg_idx as usize] = (sx + tx, ty);
            n_at[di][seg_idx as usize] = (sx + bx2, by2);

            // Pin `ext` off the head cut: the flow drops a cut at the pin.
            let ext = 340 + process.grid();
            if seg_idx == 0 {
                b.rect(li, Rect { x: sx + tx, y: ty - ext, w: ct, h: ext + ct });
                b.pin(pin(di, "P", Rect { x: sx + tx, y: ty - ext, w: ct, h: ct }, li));
            }
            if seg_idx == n_segments - 1 {
                b.rect(li, Rect { x: sx + bx2, y: by2, w: ct, h: ext + ct });
                b.pin(pin(di, "N", Rect { x: sx + bx2, y: by2 + ext, w: ct, h: ct }, li));
            }
        }

        // li jumpers chaining segment k's exit to k+1's entry (same row, adjacent
        // columns).
        for di in 0..n_dev {
            for k in 0..(n_segments as usize).saturating_sub(1) {
                let (x0, y0) = n_at[di][k];
                let (x1, y1) = p_at[di][k + 1];
                debug_assert_eq!(y0, y1, "the MY flip must land chained contacts on one row");
                let (xl, xr) = (x0.min(x1), x0.max(x1));
                b.rect(li, Rect { x: xl, y: y0, w: xr - xl + ct, h: ct });
            }
        }

        b.finish()
    }
}

/// Mirror a `h_box`-tall box at `y` about the middle of a `cell_h`-tall cell.
fn my(x: i32, y: i32, h_box: i32, cell_h: i32, flip: bool) -> (i32, i32) {
    if flip {
        (x, cell_h - y - h_box)
    } else {
        (x, y)
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default body: min segment width and a nominal 10µm body (`res_min_segment`).
    let def_w = process.rule("res_min_width", 500);
    let def_l = process.rule("res_min_segment", 10_000);
    sizing(group, c, def_w, def_l)
}

/// Device index per segment column: `Interdig` interleaves devices (greedy
/// centroid), anything else keeps each device's segments adjacent.
fn res_segment_sequence(n_dev: usize, pattern: Pattern, n_segments: i32) -> Vec<usize> {
    let per_dev = n_segments as usize;
    if pattern == Pattern::Interdig && n_dev >= 2 {
        return greedy_centroid(&vec![per_dev; n_dev]);
    }
    (0..n_dev).flat_map(|di| std::iter::repeat_n(di, per_dev)).collect()
}

/// Segment counts that divide the body evenly (1 or even), the 8 squarest.
fn feasible_segments(s: &Sizing, process: &dyn Process) -> Vec<u16> {
    let body_l = s.unit_l;
    let body_w = s.unit_w;
    let seg_gap = process.rule("res_seg_gap", 400);
    let head = process.rule("res_head", 300);
    let min_seg = process.rule("res_min_segment", 10_000).max(1);
    let max_segments = (body_l / min_seg).clamp(1, 64);
    let mut opts: Vec<i32> = (1..=max_segments)
        .filter(|&n| body_l % n == 0 && (n == 1 || n % 2 == 0))
        .collect();
    opts.sort_by(|&a, &b| {
        let quality = |n: i32| {
            let w = f64::from(n * (body_w + seg_gap));
            let h = f64::from(2 * head + body_l / n);
            (w / h).ln().abs()
        };
        quality(a).total_cmp(&quality(b))
    });
    opts.truncate(8);
    opts.sort_unstable();
    if opts.is_empty() {
        opts.push(1);
    }
    opts.into_iter().map(|n| n as u16).collect()
}
