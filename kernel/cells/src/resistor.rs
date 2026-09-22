//! Resistor generator. Ported from `backend/cells/src/generators/resistor.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::Pattern;

use crate::builder::{layer, req, rule, sizing, Builder, Sizing};
use crate::Cell;

/// One point in the **resistor variant space**: a serpentine of `segments` unit
/// bars in the PDK resistor material, flanked by dummies so end bars match
/// interior ones. `R = ρ·L/W`; the generator folds to hit target `R` while
/// keeping segments unit-matched. Theory: AOAL ch08 (unit matching);
/// `docs/cells/resistor.md`.
#[derive(Clone)]
pub struct Resistor {
    pub segments: u16,
    pub dummies_per_edge: u8,
    /// Single vs interdigitated (multi-device matched banks).
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
                patterns.iter().map(move |&pattern| Resistor {
                    segments,
                    dummies_per_edge: 1,
                    pattern,
                })
            })
            // Interdig interleaves devices' segments across columns, but a
            // multi-segment device chains its segments with li jumpers drawn
            // straight across the neighbouring column — a short on any
            // interleaved layout. ponytail: allow again with routed jumpers.
            .filter(|r| !(matches!(r.pattern, Pattern::Interdig) && r.segments > 1))
            .collect()
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        let s = group_sizing(group, &Constraints::default(), process);
        let seg_l = s.unit_l / i32::from(self.segments.max(1));
        let seg_gap = rule(process, "res_seg_gap", 400);
        let head = rule(process, "res_head", 300);
        let seg_pitch = s.unit_w + seg_gap;
        let n = group.devices.len() as i32;
        let device_gap = rule(process, "device_gap", 600);
        let total_w = match self.pattern {
            Pattern::Interdig => i32::from(self.segments) * n * seg_pitch,
            _ => {
                let per_dev = i32::from(self.segments) * seg_pitch;
                per_dev * n + device_gap * (n - 1).max(0)
            }
        };
        (total_w, head + seg_l + head)
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        (0..group.devices.len())
            .flat_map(|i| {
                [
                    port(i, "P"),
                    port(i, "N"),
                ]
            })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        // The body is a GAP between two poly end pads under an `rpm` marker —
        // the extractor's resistor abstraction (recogniser: rpm over [poly,
        // poly]). Drawing the body as a continuous conductor bar (the old rpoly
        // rect) extracts P≡N as one net: rpoly is in the deck's conductor list,
        // so the "resistor" was a wire and both labels landed on one component.
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = layer(process, "licon").unwrap_or(poly);
        let rpm = layer(process, "rpm");

        let ct = rule(process, "contact", 170);
        // li must enclose the cut on every side (sky130 licon.5-class rule); a
        // cut-sized li pad reads as zero enclosure.
        let li_enc = rule(process, "li_encloses_licon", 80);
        // Head pad long enough that the centred cut gets the deck's one-side
        // poly enclosure (80) above AND below — 65/65 failed
        // `poly_encloses_licon_one_side`.
        let head_l = rule(process, "res_head", 300)
            .max(ct + 2 * rule(process, "poly_encloses_licon_one_side", 80));
        let seg_gap = rule(process, "res_seg_gap", 400);
        let n_segments = i32::from(self.segments.max(1));
        let body_w = s.unit_w;
        let body_l = s.unit_l;
        // The drawn gap is the electrical length; it floors at poly spacing so
        // the two pads never violate poly_min_spacing (nothing measures L).
        let seg_l = (body_l / n_segments).max(rule(process, "poly_min_spacing", 210));
        let seg_pitch = body_w + seg_gap;
        let total_h = head_l + seg_l + head_l;
        let cut_enc = 40;

        let sequence = res_segment_sequence(n_dev, self.pattern, n_segments);
        let mut dev_seg_placed = vec![0i32; n_dev];
        // Per (device, segment): the P-side and N-side contact positions, for
        // the inter-segment jumpers and the end pins.
        let mut p_at = vec![vec![(0i32, 0i32); n_segments as usize]; n_dev];
        let mut n_at = vec![vec![(0i32, 0i32); n_segments as usize]; n_dev];

        for &(di, _) in &sequence {
            let slot = dev_seg_placed.iter().sum::<i32>();
            let seg_idx = dev_seg_placed[di];
            dev_seg_placed[di] += 1;

            let flip = seg_idx % 2 == 1; // Orientation::MY on odd segments
            let sx = slot * seg_pitch;

            // Two poly end pads; the gap between them is the resistive body.
            b.rect(poly, Rect { x: sx, y: 0, w: body_w, h: head_l });
            b.rect(poly, Rect { x: sx, y: head_l + seg_l, w: body_w, h: head_l });
            // Marker over the gap, lapping `cut_enc` into each pad: exactly two
            // poly polygons under one marker, the recogniser's arity.
            //
            // Width: rpm carries a real min_width (1270 on sky130), wider than
            // any body, so a single-segment marker widens symmetrically — pure
            // marker, no conductor, so the overhang costs nothing. Multi-segment
            // markers must NOT widen past the column: at seg_gap pitch they
            // would overlap, merge into one marker polygon spanning columns,
            // and the recogniser would refuse it (>2 poly pads under one
            // marker). ponytail: multi-segment bodies narrower than
            // rpm_min_width keep the violation; interleave-safe widening needs
            // per-column staggered markers.
            if let Some(rpm) = rpm {
                let rpm_min = rule(process, "rpm_min_width", 1270);
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

            let (tx, ty) = my(cx, cy_top, ct, ct, total_h, flip);
            b.rect(licon, Rect { x: sx + tx, y: ty, w: ct, h: ct });
            b.rect(li, Rect { x: sx + tx - li_enc, y: ty - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });

            let (bx2, by2) = my(cx, cy_bot, ct, ct, total_h, flip);
            b.rect(licon, Rect { x: sx + bx2, y: by2, w: ct, h: ct });
            b.rect(li, Rect { x: sx + bx2 - li_enc, y: by2 - li_enc, w: ct + 2 * li_enc, h: ct + 2 * li_enc });

            // Under MY the pad that was drawn at the top is still the P side of
            // an even segment; odd segments enter at the bottom, exit at the top.
            p_at[di][seg_idx as usize] = (sx + tx, ty);
            n_at[di][seg_idx as usize] = (sx + bx2, by2);

            // Pin sits `ext` off the head contact; the flow drops its own licon at
            // the pin center, so the two cuts must clear licon min-spacing.
            let ext = 340 + process.grid();
            if seg_idx == 0 {
                b.rect(li, Rect { x: sx + tx, y: ty - ext, w: ct, h: ext + ct });
                b.pin(pin_at(di, "P", sx + tx, ty - ext, ct, li));
            }
            if seg_idx == n_segments - 1 {
                b.rect(li, Rect { x: sx + bx2, y: by2, w: ct, h: ext + ct });
                b.pin(pin_at(di, "N", sx + bx2, by2 + ext, ct, li));
            }
        }

        // li jumpers chaining each device's segments in series: exit (N side) of
        // segment k to entry (P side) of k+1. The MY flip puts both on one row,
        // and `Single` keeps a device's segments in adjacent columns, so the
        // jumper is one horizontal li bar merging with both contact pads.
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

/// Mirror-Y transform of a `(x,y,ct,ct)` contact within a cell of height `h`.
/// `R0` when `!flip`. Only the y flips (the resistor never mirrors left-right).
fn my(x: i32, y: i32, w: i32, h_box: i32, cell_h: i32, flip: bool) -> (i32, i32) {
    let _ = w;
    if flip {
        (x, cell_h - y - h_box)
    } else {
        (x, y)
    }
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
    let t = if term == "P" { 0 } else { 1 };
    NetId((i as u16).wrapping_mul(4).wrapping_add(t))
}


fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default body: min segment width and a nominal 10µm body (`res_min_segment`).
    let def_w = rule(process, "res_min_width", 500);
    let def_l = rule(process, "res_min_segment", 10_000);
    sizing(group, c, def_w, def_l)
}

/// Segment placement sequence. `Single` lays each device's segments in order;
/// `Interdig` interleaves centroid-symmetrically (greedy) across devices.
/// Returns `(device_index, _)`; the per-device segment index is assigned by the
/// caller's `dev_seg_placed` counter.
fn res_segment_sequence(n_dev: usize, pattern: Pattern, n_segments: i32) -> Vec<(usize, i32)> {
    if n_dev == 0 {
        return vec![];
    }
    match pattern {
        Pattern::Interdig if n_dev >= 2 => {
            let counts: Vec<usize> = (0..n_dev).map(|_| n_segments as usize).collect();
            greedy_centroid(&counts).into_iter().map(|di| (di, 0)).collect()
        }
        _ => (0..n_dev)
            .flat_map(|di| (0..n_segments).map(move |seg| (di, seg)))
            .collect(),
    }
}

/// Greedy centroid-symmetric interleave over N devices, verbatim from the old
/// `greedy_centroid_sequence`.
fn greedy_centroid(counts: &[usize]) -> Vec<usize> {
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
    seq
}

/// Electrically equivalent fold-count domain, ranked by geometric diversity
/// (squarest aspect first), capped at 8 — ported from `feasible_segments`.
fn feasible_segments(s: &Sizing, process: &dyn Process) -> Vec<u16> {
    let body_l = s.unit_l;
    let body_w = s.unit_w;
    let seg_gap = rule(process, "res_seg_gap", 400);
    let head = rule(process, "res_head", 300);
    let min_seg = rule(process, "res_min_segment", 10_000).max(1);
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
