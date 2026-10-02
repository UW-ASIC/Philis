//! Resistor generator: each device a series chain of `segments` sky130
//! precision poly resistors — a continuous poly strip whose body is marked by
//! `rpoly`, with long contacted heads, all under rpm + npc + psdm.

use crate::builder::dim;
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, Macro, Node, Process, Rect};

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
        // No resistor marker, no poly resistor on this process.
        if group.devices.is_empty() || process.layer("rpoly").is_none() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let patterns = if group.devices.len() > 1 {
            vec![Pattern::Single, Pattern::Interdig]
        } else {
            vec![Pattern::Single]
        };
        // Interleaved multi-segment devices jump between their columns on
        // met1, one track per device inside the heads: as many devices as
        // tracks fit.
        let tracks = jumper_tracks(process).len();
        feasible_segments(&s, process)
            .into_iter()
            .flat_map(|segments| {
                patterns.iter().map(move |&pattern| Resistor { segments, pattern })
            })
            .filter(|r| !(matches!(r.pattern, Pattern::Interdig) && r.segments > 1 && group.devices.len() > tracks))
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
        let (mcon, met1) = (req(process, "mcon"), req(process, "met1"));
        let (m_ct, m_enc) = (dim(process, "mcon_size"), dim(process, "m1_enc"));
        let tracks = jumper_tracks(process);
        let lat = cut_lattice(process);

        let ct = dim(process, "contact");
        // Head contact: a square cut, or the deck's precision-resistor slot
        // where it states one (sky130 licon.1b: 190 x 2000 inside rpm).
        let (cw, ch) = slot(process);
        let cut_space = process.space("licon").unwrap_or(ct);
        // Poly past a cut, and li past a cut, on every side.
        let border = r("poly_encloses_licon_one_side", 0)
            .max(r("li_encloses_licon", 0))
            .max(process.enclosure("poly", "licon").unwrap_or(0))
            .max(process.endcap("poly", "licon").unwrap_or(0))
            .max(process.endcap("li", "licon").unwrap_or(0));
        let head = head_len(process);
        let lap = LAP;
        // A cut's inner side keeps the marker's spacing too, where the deck
        // states one.
        let inner = border
            .max(process.space_between("rpoly", "licon").unwrap_or(0))
            .max(process.space_between("rpoly_b", "licon").unwrap_or(0))
            .max(process.space_between("res_block", "licon").unwrap_or(0));
        let seg_gap = seg_gap(process);
        let n_segments = i32::from(self.segments.max(1));
        let body_w = s.unit_w.max(r("res_min_width", 0));
        let seg_l = (s.unit_l / n_segments).max(process.width("rpoly").unwrap_or(0));
        let seg_pitch = body_w + seg_gap;
        let total_h = head + seg_l + head;
        let head_li_h = head - lap;

        // Contact column of a head, outer end first (the top head mirrors it).
        let cut_x = snap_cut(body_w / 2 - cw / 2, lat);
        let cut_ys: Vec<i32> = (0..)
            .map(|k| border + k * (ch + cut_space))
            .take_while(|&y| y + ch + inner <= head_li_h)
            .collect();
        // The outermost cut of a head: where the pins land.
        let end_cut = |sx: i32, top: bool| Rect {
            x: sx + cut_x,
            y: if top { total_h - border - ch } else { border },
            w: cw,
            h: ch,
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
            // The body: its marker(s) and salicide block, as the recipe says.
            let body = Rect { x: sx, y: head - lap, w: body_w, h: seg_l + 2 * lap };
            b.rect(rpoly, body);
            if let Some(l) = process.layer("rpoly_b") {
                b.rect(l, body);
            }
            // The body: current runs up an even segment, down an odd one.
            b.unit(pnr_core::Unit {
                owner: di as u8,
                x: sx + body_w / 2,
                y: total_h / 2,
                weight: i64::from(body_w) * i64::from(seg_l),
                phi: (0, if enters_top { -1 } else { 1 }),
                sa: 0,
                sb: 0,
            });
            for top in [false, true] {
                let y0 = if top { total_h - head_li_h } else { 0 };
                b.rect(li, Rect { x: sx, y: y0, w: body_w, h: head_li_h });
                for &cy in &cut_ys {
                    let y = if top { total_h - cy - ch } else { cy };
                    b.rect(licon, Rect { x: sx + cut_x, y, w: cw, h: ch });
                }
            }
            match prev[di] {
                None => b.pin(pin(di, "P", end_cut(sx, enters_top), li)),
                // Jumper from the previous segment's exit head, which is at
                // this segment's entry end: li to an adjacent column, else
                // met1 on the device's own track (over others' heads).
                Some(px) if sx - px == seg_pitch => {
                    let h = border + ch + border;
                    let y = if enters_top { total_h - h } else { 0 };
                    b.rect(li, Rect { x: px, y, w: sx + body_w - px, h });
                }
                Some(px) => {
                    let t = tracks[di % tracks.len().max(1)];
                    let y = if enters_top { total_h - t - m_ct } else { t };
                    for cx in [px, sx] {
                        b.rect(mcon, Rect { x: cx + cut_x, y, w: m_ct, h: m_ct });
                    }
                    b.rect(met1, Rect { x: px + cut_x - m_enc, y: y - m_enc, w: sx - px + m_ct + 2 * m_enc, h: m_ct + 2 * m_enc });
                }
            }
            // Each segment extracts as its own resistor: the string runs
            // P -> Internal(1) -> ... -> Internal(n-1) -> N.
            let node = |k: i32| match k {
                0 => Node::Pin("P"),
                k if k == n_segments => Node::Pin("N"),
                k => Node::Internal(k as u16),
            };
            b.drawn(Drawn { owner: di as u8, device: None, kind: DrawnKind::Resistor, nodes: [node(seg), node(seg + 1), Node::Unused], w: body_w, l: seg_l });
            prev[di] = Some(sx);
            if seg == n_segments - 1 {
                b.pin(pin(di, "N", end_cut(sx, !enters_top), li));
            }
        }

        // End dummies on a matched array: a poly strip one pitch past each end,
        // contacted like a segment but unmarked (no rpoly: interconnect, not a
        // resistor), tied to ground through a `GND` pin, so the end segments
        // see the same etch neighbours as the inner ones (Hastings §8.3 r7).
        let n_cols = sequence.len() as i32;
        let dummies = n_dev > 1 || crate::builder::unitization(group, constraints).is_some_and(|u| u.dummy_required);
        // Dummies are plain poly: they keep the deck's resistor-body-to-poly
        // spacing (sky130 poly.9), never less than the segment gap.
        let d_pitch = body_w + seg_gap.max(process.space_between("rpoly", "poly").unwrap_or(0));
        // Plain poly takes plain (square) contacts: the precision slot is
        // the resistor body's.
        let sq_x = snap_cut(body_w / 2 - ct / 2, lat);
        let sq_ys: Vec<i32> = (0..).map(|k| border + k * (ct + cut_space)).take_while(|&y| y + ct + inner <= head_li_h).collect();
        let (lo, hi) = if dummies {
            for sx in [-d_pitch, (n_cols - 1) * seg_pitch + d_pitch] {
                b.rect(poly, Rect { x: sx, y: 0, w: body_w, h: total_h });
                for top in [false, true] {
                    let y0 = if top { total_h - head_li_h } else { 0 };
                    b.rect(li, Rect { x: sx, y: y0, w: body_w, h: head_li_h });
                    for &cy in &sq_ys {
                        let y = if top { total_h - cy - ct } else { cy };
                        b.rect(licon, Rect { x: sx + sq_x, y, w: ct, h: ct });
                    }
                }
                let at = Rect { x: sx + sq_x, y: border, w: ct, h: ct };
                b.pin(pnr_core::Pin { name: "GND".into(), net: pnr_core::NetId(u16::MAX), at, layer: li });
            }
            (-d_pitch, (n_cols - 1) * seg_pitch + d_pitch + body_w)
        } else {
            (0, n_cols * seg_pitch - seg_gap)
        };

        // A precision recipe's region over the array: rpm with npc on it
        // (sky130 xhrpoly), and the recipe's implant reaching `res_keepout`
        // past the poly so a neighbour keeps its distance from the body.
        let array_w = hi - lo;
        let rpm_enc = r("rpm_encloses_poly", 0).max(process.enclosure("rpm", "poly").unwrap_or(0));
        let rpm_w = (array_w + 2 * rpm_enc).max(process.width("rpm").unwrap_or(0));
        let rpm_rect = Rect { x: lo + array_w / 2 - rpm_w / 2, y: -rpm_enc, w: rpm_w, h: total_h + 2 * rpm_enc };
        if let Some(rpm) = process.layer("rpm") {
            b.rect(rpm, rpm_rect);
            if let Some(npc) = process.layer("npc") {
                b.rect(npc, rpm_rect);
            }
        }
        if let Some(imp) = process.layer("res_implant") {
            // The implant past the resistor's poly, the deck's.
            let keep = r("res_keepout", 0).max(process.enclosure("res_implant", "rpoly").unwrap_or(0)).max(process.enclosure("res_implant", "poly").unwrap_or(0));
            // ...and past the rpm it dopes (sky130 rpm.4).
            let ri = process.enclosure("res_implant", "rpm").unwrap_or(0);
            let (x0, x1) = ((rpm_rect.x - ri).min(lo - keep), (rpm_rect.x + rpm_w + ri).max(hi + keep));
            let (y0, y1) = ((rpm_rect.y - ri).min(-keep), (rpm_rect.y + rpm_rect.h + ri).max(total_h + keep));
            b.rect(imp, Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 });
        }
        // One salicide block over every body (dummies too: a blocked
        // neighbour is the resistor's own environment), past the poly by the
        // deck's extension, widened to its min area.
        if let Some(blk) = process.layer("res_block") {
            let ext = process.extension("res_block", "poly").unwrap_or(0);
            let (y0, h) = (head - lap, seg_l + 2 * lap);
            let mut x0 = lo - ext;
            let mut w = hi - lo + 2 * ext;
            let need = i32::try_from(process.area("res_block").unwrap_or(0) / i64::from(h.max(1)) + 1).unwrap_or(0);
            if w < need {
                x0 -= (need - w) / 2;
                w = need;
            }
            b.rect(blk, Rect { x: x0, y: y0, w, h });
        }
        // Head contacts on poly outside a precision region still sit in npc.
        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// Heights (from a head's outer end) of the met1 jumper tracks: clear of the
/// pin's cut row by a met1 pad (the router lands one on the pin) and met1
/// spacing, a met1 pitch apart, inside the head.
fn jumper_tracks(process: &dyn Process) -> Vec<i32> {
    let r = |name: &str, default: i32| process.rule(name, default);
    let lat = cut_lattice(process);
    let (ct, m_ct, m_enc) = (dim(process, "contact"), dim(process, "mcon_size"), dim(process, "m1_enc"));
    let border = r("poly_encloses_licon_one_side", 0)
            .max(r("li_encloses_licon", 0))
            .max(process.enclosure("poly", "licon").unwrap_or(0))
            .max(process.endcap("poly", "licon").unwrap_or(0))
            .max(process.endcap("li", "licon").unwrap_or(0));
    let head_li_h = head_len(process) - LAP;
    let pitch = m_ct + 2 * m_enc + dim(process, "met1_space");
    let first = snap_cut(border + ct + (m_ct + 2 * m_enc) + dim(process, "met1_space") + lat - 1, lat);
    (0..).map(|k| first + k * snap_cut(pitch + lat - 1, lat)).take_while(|&y| y + m_ct + m_enc + border <= head_li_h).collect()
}

/// Column gap: the deck's segment gap, and never under the li spacing its
/// full-width heads need (wide-metal spacing included).
fn seg_gap(process: &dyn Process) -> i32 {
    let lat = cut_lattice(process);
    snap_cut(process.rule("res_seg_gap", 0).max(process.space("li").unwrap_or(0)) + lat - 1, lat)
}

/// Head contact `(w, h)`: the sidecar's precision-resistor slot
/// (`res_contact_w`/`res_contact_h`), else a square `contact`.
fn slot(process: &dyn Process) -> (i32, i32) {
    let ct = dim(process, "contact");
    match (process.rule("res_contact_w", 0), process.rule("res_contact_h", 0)) {
        (w, h) if w > 0 && h > 0 => (w, h),
        _ => (ct, ct),
    }
}

/// How far the resistor marker reaches into each head: none, so the body
/// proper is exactly the segment length; the heads' cuts keep the deck's
/// marker spacing on their own (`inner`).
const LAP: i32 = 0;

/// Contacted head length: the deck's `res_head` (sky130's `xpc` must extend
/// the body by >= 2.16 um), and never less than one bordered cut past
/// [`LAP`], or the head carries no contact and the resistor no terminal.
fn head_len(process: &dyn Process) -> i32 {
    let r = |name: &str, default: i32| process.rule(name, default);
    let border = r("poly_encloses_licon_one_side", 0)
            .max(r("li_encloses_licon", 0))
            .max(process.enclosure("poly", "licon").unwrap_or(0))
            .max(process.endcap("poly", "licon").unwrap_or(0))
            .max(process.endcap("li", "licon").unwrap_or(0));
    let inner = border
        .max(process.space_between("rpoly", "licon").unwrap_or(0))
        .max(process.space_between("rpoly_b", "licon").unwrap_or(0))
        .max(process.space_between("res_block", "licon").unwrap_or(0));
    let lat = cut_lattice(process);
    snap_cut(r("res_head", 0).max(LAP + border + slot(process).1 + inner) + lat - 1, lat)
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default body: min segment width and a nominal 10µm body (`res_min_segment`).
    let def_w = process.rule("res_min_width", 0);
    let def_l = process.rule("res_min_segment", 0);
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
    let seg_gap = seg_gap(process);
    let head = head_len(process);
    let min_seg = process.rule("res_min_segment", 0).max(1);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, drawn alone, is DRC- and ERC-clean.
    #[test]
    fn every_variant_is_drc_and_erc_clean() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let mut dirty = Vec::new();
        for n in [1, 2] {
            dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, n, 1, 500, 10_000, &pdk));
            // Long enough for several segments: interleaved met1 jumpers.
            dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, n, 1, 500, 40_000, &pdk));
        }
        dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, 3, 1, 500, 40_000, &pdk));
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }
}
