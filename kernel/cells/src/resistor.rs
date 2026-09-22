//! Resistor generator: each device a series chain of `segments` sky130
//! precision poly resistors — a continuous poly strip whose body is marked by
//! `rpoly`, with long contacted heads, all under rpm + npc + psdm.

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{cut_lattice, greedy_centroid, pin, req, sizing, snap_cut, Builder, Sizing};
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
        let r = |name: &str, default: i32| process.rule(name, default);

        let poly = req(process, "poly");
        let rpoly = req(process, "rpoly");
        let li = req(process, "li");
        let licon = req(process, "licon");
        let lat = cut_lattice(process);

        let ct = r("contact", 170);
        let cut_space = r("licon_min_spacing", 170);
        // Poly past a cut, and li past a cut, on every side.
        let border = r("poly_encloses_licon_one_side", 80).max(r("li_encloses_licon", 80));
        // Contacted head: magic's `xpc` must extend the body by >= 2.16 um.
        let head = r("res_head", 2160);
        // rpoly reaches this far into each head (magic's POLYRES convention),
        // so the body proper is exactly `seg_l`.
        let lap = 60;
        let seg_gap = r("res_seg_gap", 400);
        let n_segments = i32::from(self.segments.max(1));
        let body_w = s.unit_w.max(r("res_min_width", 350));
        let seg_l = (s.unit_l / n_segments).max(r("rpoly_min_width", 150));
        let seg_pitch = body_w + seg_gap;
        let total_h = head + seg_l + head;
        let head_li_h = head - lap;

        // Contact column of a head, outer end first (the top head mirrors it).
        let cut_x = snap_cut(body_w / 2 - ct / 2, lat);
        let cut_ys: Vec<i32> = (0..)
            .map(|k| border + k * (ct + cut_space))
            .take_while(|&y| y + ct + border <= head_li_h)
            .collect();
        // The outermost cut of a head: where the pins land.
        let end_cut = |sx: i32, top: bool| Rect {
            x: sx + cut_x,
            y: if top { total_h - border - ct } else { border },
            w: ct,
            h: ct,
        };

        let sequence = res_segment_sequence(n_dev, self.pattern, n_segments);
        let mut seg_of = vec![0i32; n_dev];
        // Per device, the column x of its previous segment.
        let mut prev: Vec<Option<i32>> = vec![None; n_dev];
        for (slot, &di) in sequence.iter().enumerate() {
            let seg = seg_of[di];
            seg_of[di] += 1;
            let sx = slot as i32 * seg_pitch;
            // Even segments run bottom -> top, odd ones top -> bottom.
            let enters_top = seg % 2 == 1;

            b.rect(poly, Rect { x: sx, y: 0, w: body_w, h: total_h });
            b.rect(rpoly, Rect { x: sx, y: head - lap, w: body_w, h: seg_l + 2 * lap });
            for top in [false, true] {
                let y0 = if top { total_h - head_li_h } else { 0 };
                b.rect(li, Rect { x: sx, y: y0, w: body_w, h: head_li_h });
                for &cy in &cut_ys {
                    let y = if top { total_h - cy - ct } else { cy };
                    b.rect(licon, Rect { x: sx + cut_x, y, w: ct, h: ct });
                }
            }
            match prev[di] {
                None => b.pin(pin(di, "P", end_cut(sx, enters_top), li)),
                // li jumper from the previous segment's exit head, which is
                // at this segment's entry end.
                Some(px) => {
                    let h = border + ct + border;
                    let y = if enters_top { total_h - h } else { 0 };
                    b.rect(li, Rect { x: px, y, w: sx + body_w - px, h });
                }
            }
            prev[di] = Some(sx);
            if seg == n_segments - 1 {
                b.pin(pin(di, "N", end_cut(sx, !enters_top), li));
            }
        }

        // One precision-resistor region over the array: rpm, npc and psdm
        // (magic's xhrpoly needs all three). psdm reaches `res_keepout` past
        // the poly so a neighbour's poly/N+ keeps its distance from the body.
        let array_w = sequence.len() as i32 * seg_pitch - seg_gap;
        let rpm_enc = r("rpm_encloses_poly", 200);
        let rpm_w = (array_w + 2 * rpm_enc).max(r("rpm_min_width", 1270));
        let rpm_rect = Rect { x: array_w / 2 - rpm_w / 2, y: -rpm_enc, w: rpm_w, h: total_h + 2 * rpm_enc };
        for role in ["rpm", "npc"] {
            if let Some(l) = process.layer(role) {
                b.rect(l, rpm_rect);
            }
        }
        let keep = r("res_keepout", 420);
        let x0 = rpm_rect.x.min(-keep);
        let x1 = (rpm_rect.x + rpm_w).max(array_w + keep);
        b.rect(req(process, "psdm"), Rect { x: x0, y: -keep, w: x1 - x0, h: total_h + 2 * keep });

        b.finish()
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
