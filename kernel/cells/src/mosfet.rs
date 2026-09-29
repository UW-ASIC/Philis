//! MOSFET generator: one diffusion row of gates, contacted S/D regions, edge
//! dummies tied to a bulk tap rail, channel implant and (PMOS) nwell.

use crate::builder::dim;
use std::collections::BTreeMap;

use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, Process, Rect};

use crate::builder::{cut_lattice, greedy_centroid, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
use crate::{Cell, Pattern};

/// One MOSFET variant: `nf` fingers per device arranged by `style`, with
/// `dummies_per_edge` dummy gates on each end of the diffusion (Razavi Fig.
/// 19.21b): the edge fingers then see gate, not trench, on both sides.
///
/// `split_gates` (pairs only): device 1's gates leave the row **above** the
/// diffusion with their own strap, device 0's below. An interleave whose gate
/// spans overlap (ABBA) then keeps two private gates — the construction a
/// common-centroid differential pair needs.
///
/// `mirror_pins` (pairs): fingers pair up on shared drains and the
/// pair order is swap-reverse symmetric (see [`mirror_sequence`]), so device
/// 1's drain and gate pins are device 0's mirrored about the cell's centre —
/// the matched interconnect a differential route copies (MAT-11; Karmokar et
/// al. ASP-DAC 2022 §V-A, unequal access skews a matched pair). The centroids
/// coincide only where such an order exists; otherwise they sit one drain
/// pair apart, which `CentroidGroup` prices, so the search trades the two.
///
/// `double_gate`: see the field.
///
/// `rows` (1 or 2): two rows share one tap strip, the second mirrored about
/// it and holding the first's order relabelled (a pair) or reversed: a
/// blocked pair becomes the cross-coupled quad of Razavi Fig. 19.19, a
/// centroid row a 2-D centroid, a wide device the stacked rows of Fig. 19.11.
#[derive(Clone)]
pub struct Mosfet {
    pub nf: u16,
    pub style: Pattern,
    pub dummies_per_edge: u8,
    pub split_gates: bool,
    pub mirror_pins: bool,
    pub rows: u16,
    /// Every finger contacted at both ends, a strap on each (unsplit rows):
    /// the distributed gate R drops fourfold, `R□·W/(12·L)` against `/3`
    /// (Razavi §19.2.1; Hastings rule 22).
    pub double_gate: bool,
}

/// Dummy gates per end when the unitization asks for them: the deck's
/// `dummy_gates_per_end`, else one. Not a variant: a matched set drawn as
/// separate cells must see one environment (Razavi Fig. 19.16), so the count
/// follows the constraint, never the search.
fn dummies_per_end(process: &dyn Process) -> u8 {
    process.rule("dummy_gates_per_end", 1).clamp(0, 4) as u8
}

/// Whether a finger's own distributed poly resistance, `R□·W/(3·L)`, exceeds
/// its gate contact's: then a contact at the far end too pays for itself.
/// `false` when the deck characterises neither.
fn two_ended_gate_pays(process: &dyn Process, w: i32, l: i32) -> bool {
    match (process.sheet_ohm("poly"), process.sheet_ohm("licon")) {
        (Some(poly), Some(cut)) if l > 0 => poly * w as f32 / (3.0 * l as f32) > cut,
        _ => false,
    }
}

impl Cell for Mosfet {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let dummies = if unitization(group, constraints).is_some_and(|u| u.dummy_required) { dummies_per_end(process) } else { 0 };
        // `nf` is the schematic's: `draw` gives every finger the full unit
        // width and the LVS reference emits one card per schematic finger, so
        // a refold would draw a different device.
        let nf = s.dev_nf[0];
        let n_dev = group.devices.len();
        // A series stack (the caller orders the members) is one blocked row.
        if n_dev > 1 && unitization(group, constraints).is_some_and(|u| u.series_parallel == analog::cell::SeriesParallel::Series) {
            if s.dev_nf.iter().any(|&n| n % 2 == 0) {
                return Vec::new();
            }
            return vec![Mosfet { nf, style: Pattern::Chain, dummies_per_edge: dummies, split_gates: false, mirror_pins: false, rows: 1, double_gate: false }];
        }
        // Interdig is never offered: an ABAB boundary between devices lands on
        // a drain region and shorts two drains over shared diffusion.
        let mut styles = vec![(Pattern::Single, false, false)];
        if centroid_sequence(n_dev, usize::from(nf)).is_some() {
            styles.push((Pattern::Cc1d, false, false));
            // Split gates, then mirror pins, last: indices of the existing
            // variants stay stable.
            if n_dev == 2 {
                styles.push((Pattern::Cc1d, true, false));
                styles.push((Pattern::Cc1d, true, true));
                // Mirror pins on one shared gate strap: a mirror or load pair
                // whose drains then route as mirror images.
                styles.push((Pattern::Cc1d, false, true));
                // Route-matched pairs keep only the mirror orders: their drains
                // must be mirror images for the wiring to match (Razavi
                // §19.2.2, symmetry includes the interconnect).
                if unitization(group, constraints).is_some_and(|u| u.route_matching_required) {
                    styles.retain(|&(_, _, mirror)| mirror);
                }
            }
        }
        // Two rows when every member splits evenly and each half still has
        // its order (a centroid order at nf/2, mirror pairs at nf/2).
        let half = nf / 2;
        let two_rows = |style: Pattern, mirror: bool| {
            nf % 2 == 0
                && s.dev_nf.iter().all(|&n| n % 2 == 0)
                && s.dev_nf.iter().map(|&n| u32::from(n)).sum::<u32>() >= 4
                && match (style, mirror) {
                    (_, true) => half % 2 == 0,
                    (Pattern::Cc1d, false) => centroid_sequence(n_dev, usize::from(half)).is_some(),
                    _ => true,
                }
        };
        // Two-ended gates where a finger's poly R outweighs its contact's.
        let wide = two_ended_gate_pays(process, s.unit_w, s.unit_l);
        styles
            .into_iter()
            .flat_map(|(style, split_gates, mirror_pins)| {
                let rows: &[u16] = if two_rows(style, mirror_pins) { &[1, 2] } else { &[1] };
                let ends: &[bool] = if wide && !split_gates { &[false, true] } else { &[false] };
                rows.iter().flat_map(move |&rows| {
                    ends.iter().map(move |&double_gate| Mosfet { nf, style, dummies_per_edge: dummies, split_gates, mirror_pins, rows, double_gate })
                })
            })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let rows = self.rows.clamp(1, 2);
        let nf = (self.nf / rows).max(1);
        let dev_nf: Vec<u16> = s.dev_nf.iter().map(|&n| (n / rows).max(1)).collect();
        let first = if self.mirror_pins {
            mirror_sequence(usize::from(nf))
        } else if self.style == Pattern::Chain {
            dev_nf.iter().enumerate().flat_map(|(d, &n)| std::iter::repeat_n(d, usize::from(n))).collect()
        } else {
            finger_sequence(n_dev, self.style, nf, &dev_nf)
        };
        let row0 = self.draw_row(group, constraints, process, &first);
        if rows == 1 {
            return row0;
        }
        let second: Vec<usize> = if n_dev == 2 {
            first.iter().map(|&d| 1 - d).collect()
        } else {
            first.iter().rev().copied().collect()
        };
        stack_on_tap(&row0, &self.draw_row(group, constraints, process, &second), process)
    }
}

/// `b` mirrored about `a`'s tap strip and merged with `a`: the two rows share
/// that strip (identical in both, so it overlaps itself).
fn stack_on_tap(a: &Macro, b: &Macro, process: &dyn Process) -> Macro {
    let tap = req(process, "tap");
    // The strip is the topmost shape on the tap layer (a deck may draw tap
    // and diffusion on one layer).
    let t = a.shapes.iter().filter(|s| s.layer == tap).max_by_key(|s| s.rect.y).map_or(Rect { x: 0, y: 0, w: 0, h: 0 }, |s| s.rect);
    let y2 = 2 * t.y + t.h;
    let flip = |r: Rect| Rect { y: y2 - r.y - r.h, ..r };
    let mut out = Builder::new(process.grid());
    // One well over both rows: two overlapping well rects read as two wells,
    // the second untapped.
    let nwell = process.layer("nwell");
    let mut well: Option<Rect> = None;
    let mut grow = |r: Rect| {
        well = Some(well.map_or(r, |w: Rect| {
            let (x0, y0) = (w.x.min(r.x), w.y.min(r.y));
            Rect { x: x0, y: y0, w: (w.x + w.w).max(r.x + r.w) - x0, h: (w.y + w.h).max(r.y + r.h) - y0 }
        }));
    };
    for s in &a.shapes {
        if Some(s.layer) == nwell {
            grow(s.rect);
            continue;
        }
        out.rect(s.layer, s.rect);
    }
    // The strip keeps `a`'s cut row: `b`'s, mirrored, sits off it by the
    // strip's asymmetric enclosure and would merge into oversized cuts.
    let licon = process.layer("licon");
    let inside = |r: Rect| r.x >= t.x && r.x + r.w <= t.x + t.w && r.y >= t.y && r.y + r.h <= t.y + t.h;
    for s in &b.shapes {
        let r = flip(s.rect);
        if Some(s.layer) == licon && inside(r) {
            continue;
        }
        if Some(s.layer) == nwell {
            grow(r);
            continue;
        }
        out.rect(s.layer, r);
    }
    if let (Some(l), Some(r)) = (nwell, well) {
        out.rect(l, r);
    }
    for p in &a.pins {
        out.pin(p.clone());
    }
    for p in &b.pins {
        out.pin(pnr_core::Pin { at: flip(p.at), ..p.clone() });
    }
    for &u in &a.units {
        out.unit(u);
    }
    for &u in &b.units {
        out.unit(pnr_core::Unit { y: y2 - u.y, ..u });
    }
    for &d in a.dummies.iter().chain(&b.dummies) {
        out.dummy(d);
    }
    out.finish()
}

impl Mosfet {
    /// One row of fingers in `sequence` order (device index per finger).
    fn draw_row(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process, sequence: &[usize]) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let r = |name: &str, default: i32| process.rule(name, default);

        let diff = req(process, "diff");
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = process.layer("licon").unwrap_or(poly);
        let u = unitization(group, constraints);
        // Matched = a multi-device unitization covers the group: draw the WPE
        // well halo.
        let matched = u.is_some_and(|u| u.devices.len() > 1);
        let is_pmos = u.is_some_and(|u| u.device_type == DeviceKind::Pmos);

        let ct = dim(process, "contact");
        let poly_ext = dim(process, "poly_ext");
        // Every enclosure: the sidecar's key, never under the deck's own rule.
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let cap = |o: &str, i: &str| process.endcap(o, i).unwrap_or(0);
        let licon_poly_enc = r("licon_poly_enc", 0).max(enc("poly", "licon"));
        let licon_poly_side = r("poly_encloses_licon_one_side", licon_poly_enc).max(licon_poly_enc).max(cap("poly", "licon"));
        // Poly cut to any diff/tap: `polycon_to_diff_spacing` (n), 235 to p+.
        let polycon_gap =
            r("polycon_to_diff_spacing", 0).max(r("polycon_to_pdiff_spacing", 0)).max(process.space_between("licon", "diff").unwrap_or(0));
        // li over a cut: `li_enc` all round, `li_side` on one side per axis.
        let li_enc = r("li_encloses_licon", 0).max(enc("li", "licon"));
        let li_side = r("li_encloses_licon_one_side", 0).max(li_enc).max(cap("li", "licon"));
        let li_space = r("li_min_spacing", 0).max(process.space("li").unwrap_or(0));
        let diff_enc = r("diff_encloses_licon", 0).max(enc("diff", "licon"));
        let gate_space = r("licon_to_gate_spacing", 0).max(process.space_between("licon", "poly").unwrap_or(0));
        let tap_enc = r("tap_encloses_licon_one_side", 0).max(cap("tap", "licon")).max(enc("tap", "licon"));
        let gate_l = s.unit_l;
        let finger_w = s.unit_w;
        let m1_pitch = dim(process, "mcon_size") + 2 * dim(process, "m1_enc") + dim(process, "met1_space");
        let (sd_w, pitch) = sd_and_pitch(process, gate_l);
        // End regions hold one cut between the diff edge and a gate: it needs
        // `gate_space` to the gate, plus a lattice step of snap slack.
        let lat = cut_lattice(process);
        let sd_end = sd_w.max(ct + 2 * (gate_space + lat / 2));

        let n_fingers = sequence.len() as i32;

        // Dummy gates sit on the diffusion, one `sd_edge` region outside the
        // outer gates: wide enough for the edge contact, and for the gate pads
        // beside a dummy to keep poly spacing from it and its riser skirt.
        let nd = i32::from(self.dummies_per_edge);
        let pad_over = (gate_l.max(ct + 2 * licon_poly_side + lat) - gate_l + 1) / 2;
        let skirt_over = (gate_l.max(ct + 2 * licon_poly_side) - gate_l + 1) / 2;
        // With dummies, at least the inner gate-to-gate gap: every finger then
        // sees gates on both sides at one pitch (poly spacing effect, PSE;
        // Hastings §13.3 r9: end dummies at the array's pitch).
        let sd_edge = if nd > 0 {
            let clear = r("poly_min_spacing", 0).max(process.space("poly").unwrap_or(0)) + pad_over + if self.split_gates || self.double_gate { skirt_over } else { 0 };
            snap_cut(sd_end.max(clear).max(pitch - gate_l) + lat - 1, lat)
        } else {
            sd_end
        };
        let d_step = gate_l + sd_end;
        let gates_end = sd_edge + (n_fingers - 1) * pitch + gate_l;
        // LOD moat where devices share a row: their fingers sit at different
        // distances from the diffusion ends (ABBA: A owns both ends), so the
        // diffusion runs on past the outer dummy (bulk-tied, no signal
        // junction grows) until SA/SB are long enough that the stress term
        // fades (Hastings §13.3 r9). A lone device, a parallel group and a
        // series stack have no such mismatch.
        let moat = if n_dev > 1 && nd > 0 && self.style != Pattern::Chain { r("lod_moat_ext_moderate", 0) } else { 0 };
        let diff_x_start = -nd * d_step - moat;
        let diff_x_end = gates_end + sd_edge + nd * d_step + moat;
        // One continuous diff row; extraction splits S from D at each gate
        // (`sd = diff NOT poly`).
        b.rect(diff, Rect { x: diff_x_start, y: 0, w: diff_x_end - diff_x_start, h: finger_w });

        // Gate stub length: the gate pad's met1/li must clear the S/D pad row,
        // and the gate cut must keep `polycon_gap` from the diff.
        let cy = snap_cut(finger_w / 2 - ct / 2, lat);
        // S/D contacts: a column at the cut pitch filling the finger (one cut
        // per region leaves contact R and current crowding set by a single
        // cut, Hastings §5.1), centred; `cy` stays the pin's cut.
        let cut_pitch = snap_cut(ct + process.space("licon").unwrap_or(ct) + lat - 1, lat);
        let n_cuts = ((finger_w - 2 * diff_enc - ct) / cut_pitch + 1).max(1);
        let col_lo = snap_cut((finger_w - (n_cuts - 1) * cut_pitch - ct) / 2, lat).min(cy);
        let cuts_y: Vec<i32> = (0..n_cuts).map(|k| col_lo + k * cut_pitch).collect();
        let pin_y = cuts_y[cuts_y.len() / 2];
        // The gate pads clear the column's li by `li_space`: the pad's li
        // tops out `li_side` past its cut, the column's bottoms `li_side`
        // below its first cut, or at the diff edge if grown for area.
        let li_floor = licon_poly_side - poly_ext + ct + li_side + li_space - (col_lo - li_side).min(0);
        let stub = (2 * (m1_pitch - finger_w / 2 - poly_ext))
            .max(li_floor)
            .max(polycon_gap + licon_poly_side + ct - poly_ext)
            // Split rows leave a neighbour's gate end beside each pad: the pad
            // must sit a poly spacing below it.
            .max(if self.split_gates { ct + licon_poly_enc + licon_poly_side + r("poly_min_spacing", 0) } else { 0 });
        // Snapped down (away from the diff) so the cut sits on the lattice.
        let pad_y = snap_cut(-(poly_ext + stub), lat);
        let stub = -pad_y - poly_ext;
        // S/D regions. Single device: region 0 is S. Multi-device: region 0 is
        // D, so every inter-device boundary (odd region) is a shared source.
        // Mirror pins pair fingers on drains instead: region 0 is S again.
        let multi = n_dev > 1 && !self.mirror_pins;
        let is_s = |region: i32| (region % 2 == 0) != multi;
        // Terminal of the finger at `idx` on its left (`right_side = false`)
        // or right side. A chain restarts at drain with each member.
        let chain = self.style == Pattern::Chain;
        let start: Vec<i32> = (0..sequence.len()).map(|i| sequence[..i].iter().rposition(|&d| d != sequence[i]).map_or(0, |p| p as i32 + 1)).collect();
        let term = |idx: i32, right_side: bool| -> &'static str {
            if chain {
                let local = idx - start[idx as usize] + i32::from(right_side);
                if local % 2 == 0 { "D" } else { "S" }
            } else if is_s(idx + i32::from(right_side)) {
                "S"
            } else {
                "D"
            }
        };
        // Bottom gate pad row (every device, unless split).
        let pad_w = gate_l.max(ct + 2 * licon_poly_side + lat);
        let pad_h = ct + licon_poly_enc + licon_poly_side;
        let li_w = ct + li_enc + li_side;
        // Gate pad li grows away from the diff to the deck's min area (a side
        // length; the li role may be a real metal with a sizeable one).
        let area_side = r("li_min_area", 0);
        // Tall enough to pass the cut by `li_side` above and below (the deck's
        // end-cap holds on both ends of one axis).
        let pad_li_h = snap_cut(
            li_w.max(ct + 2 * li_side).max((i64::from(area_side) * i64::from(area_side)).div_euclid(i64::from(li_w.max(1))) as i32 + 1) + lat - 1,
            lat,
        );
        let bot_cut_y = pad_y + licon_poly_side;
        // Top gate pad row: the bottom one mirrored about the channel, cut
        // snapped up (away from the diff). `top_pad_top` is what the tap strip
        // must clear.
        let top_cut_y = snap_cut(finger_w - bot_cut_y - ct + lat - 1, lat);
        let top_pad_y = top_cut_y - licon_poly_enc;
        let top_pad_top = top_pad_y + pad_h;
        let top_li_top = top_cut_y - li_side + pad_li_h;
        let up = |di: usize| self.split_gates && di == 1;
        // Which ends of a device's fingers carry a pad.
        let top_end = |di: usize| up(di) || self.double_gate;
        let bottom_end = |di: usize| !up(di);
        // Gate x-extent per device, for the strap below.
        let mut gate_span: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
        for (idx, &di) in sequence.iter().enumerate() {
            let gx = idx as i32 * pitch + sd_edge;
            b.rect(poly, Rect { x: gx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
            // Stub and gate overlap one lattice step (they join).
            let stub_top = finger_w + poly_ext - lat;
            if top_end(di) {
                b.rect(poly, Rect { x: gx, y: stub_top, w: gate_l, h: top_pad_top - stub_top });
            }
            if bottom_end(di) {
                b.rect(poly, Rect { x: gx, y: pad_y, w: gate_l, h: stub + lat });
            }
            // The channel: gate ∩ diff. S on the left means S→D runs +x.
            b.unit(pnr_core::Unit {
                owner: di as u8,
                x: gx + gate_l / 2,
                y: finger_w / 2,
                weight: i64::from(gate_l) * i64::from(finger_w),
                phi: (if term(idx as i32, false) == "S" { 1 } else { -1 }, 0),
                sa: gx + gate_l / 2 - diff_x_start,
                sb: diff_x_end - (gx + gate_l / 2),
            });
            // Contacted gate pad (the deck requires a cut to reach poly),
            // centred on the lattice-snapped cut.
            let cut_x = snap_cut(gx + gate_l / 2 - ct / 2, lat);
            // li `li_side` past the cut toward the diff, the rest away from it.
            let top_pad = (top_cut_y, top_pad_y, top_cut_y - li_side);
            let bottom_pad = (bot_cut_y, pad_y, bot_cut_y + ct + li_side - pad_li_h);
            let pads = [top_end(di).then_some(top_pad), bottom_end(di).then_some(bottom_pad)];
            for (cy_, py_, ly_) in pads.into_iter().flatten() {
                b.rect(poly, Rect { x: cut_x + ct / 2 - pad_w / 2, y: py_, w: pad_w, h: pad_h });
                b.rect(licon, Rect { x: cut_x, y: cy_, w: ct, h: ct });
                b.rect(li, Rect { x: cut_x - li_enc, y: ly_, w: li_w, h: pad_li_h });
            }
            // The pin sits on the end the device's gate leaves by.
            let cut_y = if up(di) { top_cut_y } else { bot_cut_y };
            // One gate pin per device: the strap joins its fingers. Mirror
            // pins put device 1's on its last finger, device 0's image.
            let last = sequence.iter().rposition(|&d| d == di) == Some(idx);
            let pin_here = if self.mirror_pins && di == 1 { last } else { !gate_span.contains_key(&di) };
            if pin_here {
                b.pin(pin(di, "G", Rect { x: cut_x, y: cut_y, w: ct, h: ct }, li));
            }
            let e = gate_span.entry(di).or_insert((gx, gx + gate_l));
            e.0 = e.0.min(gx);
            e.1 = e.1.max(gx + gate_l);
        }

        // Gate strap: joins one device's fingers into a single gate. Per
        // device, so interleaved devices keep distinct gates; unsplit mirror
        // pins run one strap under the whole row (a shared-gate pair).
        let poly_w = dim(process, "poly_min_width");
        if self.mirror_pins && !self.split_gates {
            let (x0, x1) = gate_span.values().fold((i32::MAX, i32::MIN), |(a, b), &(x0, x1)| (a.min(x0), b.max(x1)));
            gate_span = BTreeMap::from([(0, (x0, x1))]);
        }
        for (&di, &(x0, x1)) in &gate_span {
            let (c0, c1) = (snap_cut(x0 + gate_l / 2 - ct / 2, lat), snap_cut(x1 - gate_l / 2 - ct / 2, lat));
            let strap = |y: i32| Rect { x: c0 - li_enc, y, w: c1 + ct + li_side - (c0 - li_enc), h: pad_li_h };
            if x1 - x0 > gate_l {
                let y = if up(di) { top_pad_top - poly_w } else { pad_y };
                b.rect(poly, Rect { x: x0, y, w: x1 - x0, h: poly_w });
                // An li strap over the finger cuts too (Hastings rule 22,
                // metal straps): the fingers join through metal, not only
                // through the poly strap's resistance.
                let ly = if up(di) { top_cut_y - li_side } else { bot_cut_y + ct + li_side - pad_li_h };
                b.rect(li, strap(ly));
            }
            // The second end's strap is li (a second poly strap would close the
            // fingers into a ring around the diffusion), with its own gate pin:
            // the router ties both ends in metal.
            if self.double_gate {
                b.rect(li, strap(top_cut_y - li_side));
                b.pin(pin(di, "G", Rect { x: c1, y: top_cut_y, w: ct, h: ct }, li));
            }
        }

        // S/D li column span: its cuts, grown to the deck's li min area —
        // within the diffusion first (no pads there), then above it (the
        // dummy row below keeps its spacing from this top).
        let (col_bot, col_top) = {
            let lo = col_lo - li_side;
            let hi = cuts_y[cuts_y.len() - 1] + ct + li_side;
            let w = i64::from(ct + li_enc + li_side);
            let need = i32::try_from((process.area("li").unwrap_or(0) + w - 1) / w).unwrap_or(0);
            let hi = hi.max((lo + need).min(finger_w));
            let lo = snap_cut(lo.min((hi - need).max(0)), lat);
            (lo, snap_cut(hi.max(lo + need) + lat - 1, lat))
        };
        for region in 0..=n_fingers {
            let cx = if region == 0 {
                sd_edge / 2
            } else if region == n_fingers {
                gates_end + sd_edge / 2
            } else {
                (2 * region - 1) * pitch / 2 + sd_edge + gate_l / 2
            };
            let px = snap_cut(cx - ct / 2, lat);
            // A column of cuts encloses each one on its inner side; only a lone
            // cut needs `li_side` beyond it (then above, away from the pads).
            b.rect(li, Rect { x: px - li_enc, y: col_bot, w: ct + li_enc + li_side, h: col_top - col_bot });
            for &y in &cuts_y {
                b.rect(licon, Rect { x: px, y, w: ct, h: ct });
            }
            let at = Rect { x: px, y: pin_y, w: ct, h: ct };
            let right = (region < n_fingers).then(|| sequence[region as usize]);
            let left = (region > 0).then(|| sequence[region as usize - 1]);
            if let Some(di) = right {
                b.pin(pin(di, term(region, false), at, li));
            }
            if let Some(di) = left.filter(|&d| Some(d) != right) {
                b.pin(pin(di, term(region - 1, true), at, li));
            }
        }

        // Dummy gates on the diffusion ends, gate and outer S/D tied up to the
        // bulk rail: off transistors the LVS reference lists (`Macro::dummies`).
        let rise_l = li_side.max(li_enc).max(lat);
        let rise_r = rise_l.max(li_side);
        // Cut `diff_enc` above the strip bottom and `tap_enc` below its top: the
        // deck wants `tap_enc` on one side of each axis.
        let tap_h = ct + diff_enc + tap_enc;
        // Dummy cut row: above the gate's poly end, and its riser strip
        // `li_space` above the edge regions' contact column.
        let licon_y = snap_cut((finger_w + polycon_gap.max(poly_ext - lat + licon_poly_side)).max(col_top + li_space + rise_l) + lat - 1, lat);
        // The tap row clears the dummy cuts by the poly-cut-to-diff spacing.
        let tap_diff = process.space_between("tap", "diff").or(process.space("diff")).unwrap_or(0);
        // The tap's implant, past the strip, keeps its gate spacing (ihp pSD.j).
        let tap_imp = if is_pmos { "nsdm" } else { "psdm" };
        let imp_gate = enc(tap_imp, "tap").max(enc(tap_imp, "diff")) + process.space_between(tap_imp, "poly").unwrap_or(0);
        let mut tap_y0 = snap_cut((finger_w + tap_diff.max(imp_gate)).max(licon_y + ct + polycon_gap) + lat - 1, lat);
        if self.split_gates || self.double_gate {
            // Clear the top gate row: poly-cut-to-diff below the tap, li
            // spacing below the rail (`rail_li_y = tap_y0 + diff_enc − li_side`).
            let floor = (top_pad_top + polycon_gap).max(top_li_top + li_space - diff_enc + li_side);
            tap_y0 = tap_y0.max(snap_cut(floor + lat - 1, lat));
        }
        // Riser x extents per edge: each edge's dummy cuts and outer S/D cuts
        // share one poly skirt / li strip (separate ones violate spacing).
        let mut edge_cuts: [Option<(i32, i32)>; 2] = [None, None];
        let mut outer_cuts: Vec<i32> = Vec::new();
        let ends = [(sequence[0], term(0, false)), (sequence[sequence.len() - 1], term(n_fingers - 1, true))];
        for k in 0..nd {
            // Gate `k` out from each end, and the S/D region beyond it.
            let left = -(k + 1) * gate_l - k * sd_end;
            let right = gates_end + sd_edge + k * d_step;
            for (edge, (dx, rx)) in [(left, left - sd_end), (right, right + gate_l)].into_iter().enumerate() {
                b.rect(poly, Rect { x: dx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
                let cx = snap_cut(dx + gate_l / 2 - ct / 2, lat);
                b.rect(licon, Rect { x: cx, y: licon_y, w: ct, h: ct });
                let px = snap_cut(rx + sd_end / 2 - ct / 2, lat);
                b.rect(licon, Rect { x: px, y: cy, w: ct, h: ct });
                outer_cuts.push(px);
                let e = edge_cuts[edge].get_or_insert((cx, cx));
                e.0 = e.0.min(cx).min(px);
                e.1 = e.1.max(cx).max(px);
                let (owner, near) = ends[edge];
                b.dummy(pnr_core::Dummy {
                    owner: owner as u8,
                    pmos: is_pmos,
                    edge: if k == 0 { near } else { "B" },
                    w: finger_w,
                    l: gate_l,
                });
            }
        }
        let stub_top = licon_y + ct + licon_poly_enc;
        let dpad_w = gate_l.max(ct + 2 * licon_poly_side);
        let skirt_y = finger_w + poly_ext - 10;
        // Riser strips lap the rail's li but stay above its cut row (a grazed
        // cut reads as an under-sized contact).
        // Symmetric about its cuts, so a row mirrored about the strip puts
        // its rail exactly on this one.
        let rail_li_y = (tap_y0 + diff_enc) - li_side;
        for (e, (cx0, cx1)) in edge_cuts.into_iter().enumerate().filter_map(|(e, c)| Some((e, c?))) {
            // The skirt joins this edge's dummy gates only (outer S/D cuts sit
            // on diff, not poly): its x span is the dummy cuts'.
            let gates: Vec<i32> = (0..nd)
                .map(|k| if e == 0 { -(k + 1) * gate_l - k * sd_end } else { gates_end + sd_edge + k * d_step })
                .map(|dx| snap_cut(dx + gate_l / 2 - ct / 2, lat))
                .collect();
            let (g0, g1) = (gates.iter().copied().min().unwrap_or(cx0), gates.iter().copied().max().unwrap_or(cx1));
            let x0 = g0 + ct / 2 - dpad_w / 2;
            b.rect(poly, Rect { x: x0, y: skirt_y, w: (g1 + ct / 2 + dpad_w / 2) - x0, h: stub_top - skirt_y });
            b.rect(li, Rect {
                x: cx0 - rise_l,
                y: licon_y - rise_l,
                w: (cx1 + ct + rise_r) - (cx0 - rise_l),
                h: (rail_li_y + lat) - (licon_y - rise_l),
            });
        }
        // Each outer S/D cut: an li column lapping 40 nm into the riser strip,
        // short of its cut row (an edge on a cut's corner reads as uncovered).
        for &px in &outer_cuts {
            let top = licon_y - rise_l + lat;
            b.rect(li, Rect { x: px - li_enc, y: cy - li_side, w: ct + li_enc + li_side, h: top - (cy - li_side) });
        }

        // Bulk tap strip spanning the diffusion, dummies included: n+ tap for
        // PMOS, p+ for NMOS. Mandatory, or the well/substrate floats.
        let tap_x0 = diff_x_start;
        let tap_x1 = diff_x_end;
        let tap_w = tap_x1 - tap_x0;
        b.rect(req(process, "tap"), Rect { x: tap_x0, y: tap_y0, w: tap_w, h: tap_h });
        let (tap_role, chan_role) = if is_pmos { ("nsdm", "psdm") } else { ("psdm", "nsdm") };
        let imp = req(process, tap_role);
        // Implant past what it types, the deck's: the tap strip, and the
        // channel (its diffusion and the gates on it).
        let imp_enc = enc(tap_role, "tap").max(enc(tap_role, "diff"));
        let chan_enc = enc(chan_role, "diff").max(enc(chan_role, "poly"));
        b.rect(imp, Rect {
            x: tap_x0 - imp_enc,
            y: tap_y0 - imp_enc,
            w: tap_w + 2 * imp_enc,
            h: tap_h + 2 * imp_enc,
        });
        // Channel implant of the device's own polarity: the only thing that
        // tells the extractor NMOS from PMOS.
        if let Some(chan_imp) = process.layer(chan_role) {
            b.rect(chan_imp, Rect {
                x: diff_x_start - chan_enc,
                y: -chan_enc,
                w: (diff_x_end - diff_x_start) + 2 * chan_enc,
                h: finger_w + 2 * chan_enc,
            });
        }
        // Tap cuts, skipping columns under a riser strip.
        let guard_pitch = snap_cut(r("guard_licon_pitch", 0).max(ct + process.space("licon").unwrap_or(0)) + lat - 1, lat);
        let under_riser = |lx: i32| {
            edge_cuts.iter().flatten().any(|&(x0, x1)| lx <= x1 + ct + rise_r && x0 - rise_l <= lx + ct)
        };
        let mut kept: Option<(i32, i32)> = None;
        // End cuts keep `tap_enc` of tap along the strip (licon.7).
        let mut lx = snap_cut(tap_x0 + tap_enc + lat - 1, lat);
        while lx + ct + tap_enc <= tap_x1 {
            if !under_riser(lx) {
                b.rect(licon, Rect { x: lx, y: tap_y0 + diff_enc, w: ct, h: ct });
                kept.get_or_insert((lx, lx)).1 = lx;
            }
            lx += guard_pitch;
        }
        // One solid li rail over the cut row, spanning the strip so it merges
        // with the risers.
        let (first_lx, last_lx) = kept.unwrap_or((tap_x0 + tap_enc, tap_x0 + tap_enc));
        let rail_x0 = (first_lx - li_enc).min(tap_x0);
        b.rect(li, Rect {
            x: rail_x0,
            y: rail_li_y,
            w: (last_lx + ct + li_side).max(tap_x1) - rail_x0,
            h: ct + 2 * li_side,
        });
        // One bulk pin per device on the rail.
        for di in 0..n_dev {
            let xi = (first_lx + di as i32 * guard_pitch).min(last_lx);
            b.pin(pin(di, "B", Rect { x: xi, y: tap_y0 + diff_enc, w: ct, h: ct }, li));
        }

        // PMOS nwell, inflated by the WPE halo on matched groups.
        if is_pmos {
            if let Some(nwell) = process.layer("nwell") {
                let wpe_halo = if matched { r("wpe_clearance_moderate", 0) } else { 0 };
                let nw_enc = dim(process, "nwell_diff_enc") + wpe_halo;
                let nw_min = dim(process, "nwell_min_width");
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

        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// Inner S/D region width and gate-to-gate pitch of a row at gate length
/// `gate_l`, nm. S/D floors: met1 pitch, facing li pads across one gate, and
/// the end cut's diff enclosure.
#[must_use]
pub fn sd_and_pitch(process: &dyn Process, gate_l: i32) -> (i32, i32) {
    let r = |name: &str, default: i32| process.rule(name, default);
    let ct = dim(process, "contact");
    let li_enc = r("li_encloses_licon", 0).max(process.enclosure("li", "licon").unwrap_or(0));
    let li_side = r("li_encloses_licon_one_side", 0).max(li_enc).max(process.endcap("li", "licon").unwrap_or(0));
    let m1_pitch = dim(process, "mcon_size") + 2 * dim(process, "m1_enc") + dim(process, "met1_space");
    let diff_enc = r("diff_encloses_licon", 0).max(process.enclosure("diff", "licon").unwrap_or(0));
    let sd_w = r("sd_width", 0)
        .max(m1_pitch - gate_l)
        .max(ct + li_enc + li_side + r("li_min_spacing", 0).max(process.space("li").unwrap_or(0)) - gate_l)
        .max(ct + 2 * diff_enc);
    (sd_w, (sd_w + gate_l + sd_w).max(m1_pitch))
}

/// Which poly island each member's gate pin (`d{i}:G`) sits on, by member
/// index: equal ids mean the drawn poly joins those gates. `None` for a member
/// without a gate pin on poly. Exact for the one layer gates share through, so
/// a caller can reject a variant that shorts two distinct gates.
#[must_use]
pub fn gate_islands(m: &Macro, poly: pnr_core::LayerId, members: usize) -> Vec<Option<usize>> {
    let at: Vec<Option<(i32, i32)>> = (0..members)
        .map(|i| {
            let p = m.pins.iter().find(|p| p.name == format!("d{i}:G"))?;
            Some((p.at.x + p.at.w / 2, p.at.y + p.at.h / 2))
        })
        .collect();
    let pts: Vec<(i32, i32)> = at.iter().flatten().copied().collect();
    let mut ids = poly_islands_at(m, poly, &pts).into_iter();
    at.iter().map(|p| p.and_then(|_| ids.next().flatten())).collect()
}

/// The poly island under each point (`None` off poly); equal ids = joined.
#[must_use]
pub fn poly_islands_at(m: &Macro, poly: pnr_core::LayerId, points: &[(i32, i32)]) -> Vec<Option<usize>> {
    let rects: Vec<Rect> = m.shapes.iter().filter(|s| s.layer == poly).map(|s| s.rect).collect();
    let mut parent: Vec<usize> = (0..rects.len()).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let touch = |a: &Rect, b: &Rect| a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
    for i in 0..rects.len() {
        for j in i + 1..rects.len() {
            if touch(&rects[i], &rects[j]) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                parent[ri] = rj;
            }
        }
    }
    points
        .iter()
        .map(|&(x, y)| {
            let k = rects.iter().position(|r| (r.x..=r.x + r.w).contains(&x) && (r.y..=r.y + r.h).contains(&y))?;
            Some(root(&mut parent, k))
        })
        .collect()
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, dim(process, "min_finger_width"), dim(process, "min_gate_l"))
}

/// The common-centroid finger order for `n_dev` devices at `nf` fingers each,
/// or `None` when none exists.
///
/// A boundary between two devices must land on a source (odd) region, so
/// fingers pair up (`seq[1] == seq[2]`, …) and mirror symmetry forces both
/// array ends onto one device. A pair (`ABBA`…) works at any even `nf`; with
/// more devices only one owns the middle pair, so `nf` must be a multiple of 4
/// (unit `A BB CC .. AA .. CC BB A`, repeated `nf / 4` times).
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

/// Finger order for a pair at `nf` fingers each whose pin sets mirror onto
/// each other: fingers pair up on a shared drain (`nf` pairs), and the pair
/// order is `H ++ swap(reverse(H))`, so reflecting the row swaps the devices.
/// `H` is chosen to bring the centroids closest (exact where one exists, e.g.
/// `nf = 8`: A BB A B AA B); ties go to the lexicographically first.
///
/// ponytail: brute force over `2^(nf/2)` halves, capped at `nf = 32`; larger
/// arrays fall back to alternating pairs (one pair of centroid offset).
fn mirror_sequence(nf: usize) -> Vec<usize> {
    let half = nf / 2;
    let build = |h: u32| -> Vec<usize> {
        let first: Vec<usize> = (0..half).map(|i| ((h >> (half - 1 - i)) & 1) as usize).collect();
        first.iter().copied().chain(first.iter().rev().map(|&d| 1 - d)).collect()
    };
    // Twice device 0's pair-index sum minus the balanced one: 0 = coincident.
    let offset = |pairs: &[usize]| {
        let sum: i64 = pairs.iter().enumerate().filter(|&(_, &d)| d == 0).map(|(i, _)| i as i64).sum();
        (2 * sum - (half as i64) * (nf as i64 - 1)).abs()
    };
    let pairs = if half == 0 {
        Vec::new()
    } else if half <= 16 {
        (0..1u32 << half).map(build).min_by_key(|p| offset(p)).unwrap_or_default()
    } else {
        build(0b0101_0101_0101_0101 & ((1 << 16) - 1))
    };
    pairs.into_iter().flat_map(|d| [d, d]).collect()
}

/// Device index per finger slot. Unequal per-device counts (ratioed mirror)
/// use the greedy centroid interleave; `Cc1d` its centroid order when one
/// exists; everything else each device's fingers in a block.
fn finger_sequence(n_dev: usize, style: Pattern, nf: u16, dev_nf: &[u16]) -> Vec<usize> {
    if dev_nf.len() == n_dev && dev_nf.iter().any(|&x| x != dev_nf[0]) {
        let counts: Vec<usize> = dev_nf.iter().map(|&x| usize::from(x)).collect();
        return greedy_centroid(&counts);
    }
    let nf = usize::from(nf);
    if style == Pattern::Cc1d {
        if let Some(seq) = centroid_sequence(n_dev, nf) {
            return seq;
        }
    }
    (0..n_dev).flat_map(|d| std::iter::repeat_n(d, nf)).collect()
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
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            // Centroid styles need two fingers a side for a pair, four for a quad.
            for (n, nf) in [(1usize, 1u16), (2, 1), (2, 2), (4, 4)] {
                dirty.extend(testkit::dirty::<Mosfet>(kind, n, nf, 1680, 150, &pdk));
            }
            dirty.extend(testkit::dirty::<Mosfet>(kind, 1, 1, 420, 150, &pdk));
            // A wide, long finger: a tall contact column, long gates.
            dirty.extend(testkit::dirty::<Mosfet>(kind, 2, 2, 5000, 1000, &pdk));
            // Wide and short: its poly R outweighs a contact, so two-ended.
            dirty.extend(testkit::dirty::<Mosfet>(kind, 2, 2, 10_000, 150, &pdk));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    /// Every variant extracts exactly its fingers plus its dummies as MOS
    /// devices: nothing merged away (a holed gate), nothing extra.
    #[test]
    fn every_variant_extracts_its_fingers_and_dummies() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let mut wrong = Vec::new();
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            for (n, nf, w, l) in [(1usize, 1u16, 1680, 150), (2, 2, 1680, 150), (2, 2, 5000, 1000), (2, 2, 10_000, 150), (4, 4, 1680, 150)] {
                for dummies in [false, true] {
                    let (g, mut c) = testkit::group_of(kind, n, nf, w, l);
                    c.unitization[0].dummy_required = dummies;
                    for (i, v) in Mosfet::enumerate(&g, &c, &pdk).iter().enumerate() {
                        let m = v.draw(&g, &c, &pdk);
                        let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).unwrap_or_default();
                        let got = spice.lines().filter(|l| l.starts_with('M')).count();
                        if got != m.units.len() + m.dummies.len() {
                            wrong.push(format!("{kind:?} n={n} nf={nf} w={w} dummies={dummies} #{i}: {got} vs {}", m.units.len() + m.dummies.len()));
                        }
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// Every admitted centroid order is balanced, centred, and never puts two
    /// different devices across a drain (even) region — a short only LVS sees.
    #[test]
    fn a_centroid_order_is_symmetric_and_never_abuts_two_drains() {
        for n_dev in 2..=6usize {
            for nf in 1..=12usize {
                let Some(seq) = centroid_sequence(n_dev, nf) else { continue };
                assert_eq!(seq.len(), n_dev * nf);
                for d in 0..n_dev {
                    assert_eq!(seq.iter().filter(|&&x| x == d).count(), nf, "n={n_dev} nf={nf} d={d}");
                    let sum: usize = seq.iter().enumerate().filter(|(_, &x)| x == d).map(|(i, _)| i).sum();
                    assert_eq!(2 * sum, nf * (seq.len() - 1), "n={n_dev} nf={nf}: {d} off-centre");
                }
                for r in (2..seq.len()).step_by(2) {
                    assert_eq!(seq[r - 1], seq[r], "n={n_dev} nf={nf}: drain short at {r}: {seq:?}");
                }
            }
        }
    }

    #[test]
    fn a_quad_needs_four_fingers_before_a_centroid_exists() {
        assert!(centroid_sequence(4, 1).is_none());
        assert!(centroid_sequence(4, 2).is_none());
        assert!(centroid_sequence(4, 4).is_some());
        assert!(centroid_sequence(2, 2).is_some());
        assert!(centroid_sequence(2, 1).is_none());
    }

    /// Each finger is one unit on its channel, with signed S→D direction: a
    /// merged ABBA pair balances per device, a merged single-finger pair
    /// (`D A S B D`) runs its currents opposite ways.
    #[test]
    fn units_carry_each_finger_s_signed_direction() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let phi_sum = |m: &pnr_core::Macro, d: u8| m.units.iter().filter(|u| u.owner == d).map(|u| i32::from(u.phi.0)).sum::<i32>();

        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 1, 1680, 150);
        let m = Mosfet { nf: 1, style: Pattern::Single, dummies_per_edge: 1, split_gates: false, mirror_pins: false, rows: 1, double_gate: false }.draw(&g, &c, &pdk);
        assert_eq!(m.units.len(), 2);
        assert_eq!((phi_sum(&m, 0), phi_sum(&m, 1)), (-1, 1), "shared-source pair mirrors its currents");
        assert!(m.units.iter().all(|u| u.weight == 1680 * 150));

        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        let m = Mosfet { nf: 2, style: Pattern::Cc1d, dummies_per_edge: 1, split_gates: false, mirror_pins: false, rows: 1, double_gate: false }.draw(&g, &c, &pdk);
        assert_eq!(m.units.len(), 4);
        assert_eq!((phi_sum(&m, 0), phi_sum(&m, 1)), (0, 0), "ABBA balances each device");
        // Equal electrical moments: both devices centre on the same x.
        let mean_x = |d: u8| m.units.iter().filter(|u| u.owner == d).map(|u| u.x).sum::<i32>();
        assert_eq!(mean_x(0), mean_x(1));
    }

    /// A series stack draws as one row, each junction between members one
    /// shared pad carrying the upper member's S and the next one's D, and is
    /// DRC/ERC clean with or without dummies.
    #[test]
    fn a_chain_shares_each_junction_and_is_clean() {
        use crate::testkit;
        use analog::cell::SeriesParallel;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        for (n, nf) in [(2usize, 1u16), (3, 1), (2, 3)] {
            for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
                for dummies in [false, true] {
                    let (g, mut c) = testkit::group_of(kind, n, nf, 1680, 150);
                    c.unitization[0].series_parallel = SeriesParallel::Series;
                    c.unitization[0].dummy_required = dummies;
                    let v = Mosfet::enumerate(&g, &c, &pdk);
                    assert_eq!(v.len(), 1, "one chain variant");
                    let m = v[0].draw(&g, &c, &pdk);
                    for k in 0..n - 1 {
                        let s = m.pins.iter().find(|p| p.name == format!("d{k}:S")).map(|p| p.at);
                        let d = m.pins.iter().filter(|p| p.name == format!("d{}:D", k + 1)).map(|p| p.at).next();
                        // The shared junction: member k's last S is k+1's first D.
                        let last_s = m.pins.iter().filter(|p| p.name == format!("d{k}:S")).map(|p| p.at.x).max();
                        assert_eq!(last_s, d.map(|r| r.x), "n={n} nf={nf}: junction {k} not shared ({s:?} vs {d:?})");
                    }
                    // Labels: one per pad, named by the first pin on it.
                    let mut labels: Vec<verify::LabeledPin> = Vec::new();
                    for p in &m.pins {
                        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
                        if !labels.iter().any(|l| (l.x, l.y) == (x, y)) {
                            let name = if p.name.ends_with(":B") { "B".to_string() } else { p.name.replace(':', "_") };
                            labels.push(verify::LabeledPin { name, layer: p.layer.0, x, y });
                        }
                    }
                    let dirty = crate::testkit::findings(&m.shapes, &labels, &pdk);
                    assert!(dirty.is_empty(), "{kind:?} n={n} nf={nf} dummies={dummies}: {dirty:?}");
                }
            }
        }
    }

    /// A two-ended gate still extracts one device per finger: the second
    /// strap is li, so the poly never closes a ring around the diffusion (a
    /// holed gate polygon extracts nothing, silently to DRC/ERC).
    #[test]
    fn a_two_ended_gate_extracts_every_finger() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        // 10 µm / 150 nm: ~1.1 kΩ of poly per finger against a 152 Ω poly
        // contact; 1 µm: ~107 Ω, under it.
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 4, 10_000, 150);
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.double_gate).expect("offered where it pays");
        let narrow = testkit::group_of(DeviceKind::Nmos, 1, 4, 1000, 150);
        assert!(Mosfet::enumerate(&narrow.0, &narrow.1, &pdk).iter().all(|v| !v.double_gate), "not where a contact outweighs the finger");
        let m = v.draw(&g, &c, &pdk);
        let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).unwrap();
        assert_eq!(spice.lines().filter(|l| l.starts_with('M')).count(), 4, "{spice}");
    }

    /// With dummies, the poly pitch is uniform: each dummy sits one gate
    /// pitch from its neighbouring finger, as the fingers do from each other.
    #[test]
    fn dummies_keep_the_finger_pitch() {
        use crate::testkit;
        use pnr_core::{DeviceKind, Process};
        let Some(pdk) = testkit::pdk() else { return };
        let (g, mut c) = testkit::group_of(DeviceKind::Nmos, 1, 4, 1680, 150);
        c.unitization[0].dummy_required = true;
        let m = Mosfet::enumerate(&g, &c, &pdk)[0].draw(&g, &c, &pdk);
        let poly = pdk.layer("poly").unwrap();
        // Gate stripes: poly crossing the whole finger width.
        let mut xs: Vec<i32> = m.shapes.iter().filter(|s| s.layer == poly && s.rect.h >= 1680).map(|s| s.rect.x).collect();
        xs.sort_unstable();
        xs.dedup();
        let steps: Vec<i32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        assert_eq!(steps.len(), 5, "4 fingers + 2 dummies: {xs:?}");
        let lat = crate::builder::cut_lattice(&pdk);
        assert!(steps.iter().all(|&d| (d - steps[1]).abs() <= lat), "non-uniform poly pitch {steps:?}");
    }

    /// Two rows turn a blocked pair (`AABB`) into the cross-coupled quad of
    /// Razavi Fig. 19.19: both devices' centroids coincide in x and y.
    #[test]
    fn two_rows_cross_couple_a_blocked_pair() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        let m = Mosfet { nf: 2, style: Pattern::Single, dummies_per_edge: 1, split_gates: false, mirror_pins: false, rows: 2, double_gate: false }.draw(&g, &c, &pdk);
        assert_eq!(m.units.len(), 4);
        let sum = |d: u8| m.units.iter().filter(|u| u.owner == d).fold((0i64, 0i64), |(x, y), u| (x + i64::from(u.x), y + i64::from(u.y)));
        assert_eq!(sum(0), sum(1), "centroids coincide");
        let ys = |d: u8| { let mut v: Vec<i32> = m.units.iter().filter(|u| u.owner == d).map(|u| u.y).collect(); v.sort_unstable(); v.dedup(); v.len() };
        assert_eq!((ys(0), ys(1)), (2, 2), "each device has a finger in both rows");
    }

    /// Split gates give an ABBA pair two private gates, each joining its own
    /// fingers; the shared-strap ABBA joins them (a mirror's single gate).
    #[test]
    fn split_gates_keep_an_abba_pair_s_gates_private() {
        use crate::testkit;
        use pnr_core::{DeviceKind, Process};
        let Some(pdk) = testkit::pdk() else { return };
        let poly = pdk.layer("poly").unwrap();
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        let draw = |split_gates| Mosfet { nf: 2, style: Pattern::Cc1d, dummies_per_edge: 1, split_gates, mirror_pins: false, rows: 1, double_gate: false }.draw(&g, &c, &pdk);

        let split = gate_islands(&draw(true), poly, 2);
        assert!(split.iter().all(Option::is_some));
        assert_ne!(split[0], split[1], "split ABBA must not join the two gates");
        // Each device still one gate: every finger of it on its pin's island.
        let shared = gate_islands(&draw(false), poly, 2);
        assert_eq!(shared[0], shared[1], "one strap row runs A's strap across B's stubs");
        // Every finger sits on its own device's gate island: no open finger.
        let m = draw(true);
        let fingers: Vec<(i32, i32)> = m.units.iter().map(|u| (u.x, u.y)).collect();
        for (u, isl) in m.units.iter().zip(poly_islands_at(&m, poly, &fingers)) {
            assert_eq!(isl, split[usize::from(u.owner)], "finger at x={} is not on d{}'s gate", u.x, u.owner);
        }
        // Balanced moments and currents survive the split.
        let phi = |d: u8| m.units.iter().filter(|u| u.owner == d).map(|u| i32::from(u.phi.0)).sum::<i32>();
        assert_eq!((phi(0), phi(1)), (0, 0));
    }

    /// A mirror-pin order: drains never short, reflection swaps the devices,
    /// and the centroids coincide where the size allows (nf = 8).
    #[test]
    fn mirror_order_swaps_on_reflection_and_centres_when_it_can() {
        for nf in [2usize, 4, 6, 8, 12] {
            let seq = mirror_sequence(nf);
            assert_eq!(seq.len(), 2 * nf);
            let n = seq.len();
            for i in 0..n {
                assert_eq!(seq[i], 1 - seq[n - 1 - i], "nf={nf}: {seq:?} is not swap-reverse symmetric");
            }
            for p in (0..n).step_by(2) {
                assert_eq!(seq[p], seq[p + 1], "nf={nf}: drain at {p} shared by two devices");
            }
        }
        let seq = mirror_sequence(8);
        let mean2 = |d: usize| seq.iter().enumerate().filter(|&(_, &x)| x == d).map(|(i, _)| i).sum::<usize>();
        assert_eq!(mean2(0), mean2(1), "nf=8 admits an exact centroid: {seq:?}");
    }

    /// The drawn cell: device 1's drain and gate pins are device 0's reflected
    /// about the cell centre, same count, and the variant is DRC/ERC clean.
    #[test]
    fn mirror_pins_reflect_one_device_onto_the_other() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        for nf in [2u16, 4] {
            let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, nf, 1680, 150);
            let m = Mosfet { nf, style: Pattern::Cc1d, dummies_per_edge: 1, split_gates: true, mirror_pins: true, rows: 1, double_gate: false }.draw(&g, &c, &pdk);
            // The row's centre: the channels are symmetric about it.
            let xs: Vec<i32> = m.units.iter().map(|u| u.x).collect();
            let axis2 = xs.iter().min().unwrap() + xs.iter().max().unwrap();
            let centres = |name: &str| {
                let mut v: Vec<i32> = m.pins.iter().filter(|p| p.name == name).map(|p| 2 * p.at.x + p.at.w).collect();
                v.sort_unstable();
                v
            };
            for t in ["D", "G"] {
                let (a, b) = (centres(&format!("d0:{t}")), centres(&format!("d1:{t}")));
                assert_eq!(a.len(), b.len(), "nf={nf} {t}: {a:?} vs {b:?}");
                let mut img: Vec<i32> = a.iter().map(|&x| 2 * axis2 - x).collect();
                img.sort_unstable();
                // Pads snap to the cut lattice: allow one step either way.
                let lat = 2 * crate::builder::cut_lattice(&pdk);
                assert!(img.iter().zip(&b).all(|(i, j)| (i - j).abs() <= lat), "nf={nf} {t}: mirror of {a:?} is {img:?}, got {b:?}");
            }
        }
        let mut dirty = Vec::new();
        for nf in [2u16, 4] {
            dirty.extend(testkit::dirty::<Mosfet>(DeviceKind::Nmos, 2, nf, 1680, 150, &pdk));
            dirty.extend(testkit::dirty::<Mosfet>(DeviceKind::Pmos, 2, nf, 1680, 150, &pdk));
        }
        assert!(dirty.is_empty(), "{}", dirty.join("\n"));
    }
}
