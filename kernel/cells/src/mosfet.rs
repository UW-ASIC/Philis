//! MOSFET generator: one diffusion row of gates, contacted S/D regions, edge
//! dummies tied to a bulk tap rail, channel implant and (PMOS) nwell.

use crate::builder::dim;
use std::collections::BTreeMap;

use analog::matching::pattern::{self, Outer};
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, MatchClass, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
use crate::{Cell, Pattern};

/// One MOSFET variant: `nf` fingers per device arranged by `style`, with
/// `dummies_per_edge` dummy gates on each end of the diffusion (Razavi Fig.
/// 19.21b): the edge fingers then see gate, not trench, on both sides.
///
/// Gates leave the row on a continuous poly bar per device (Hastings eqs
/// 13.7–13.9: no per-finger pad sets the pitch, so it is the deck's minimum
/// contacted pitch, [`sd_and_pitch`]).
///
/// `split_gates`: odd-indexed devices' gates leave the row **above** the
/// diffusion on their own bar, even-indexed ones below. An interleave whose
/// gate spans overlap (ABBA) then keeps two private gates — the construction
/// a common-centroid differential pair needs — and blocks whose boundary
/// fingers are too few to drop a cut ([`bar_cuts`]) still draw.
///
/// `mirror_pins` (pairs): fingers pair up on shared drains and the
/// pair order is swap-reverse symmetric (see [`mirror_sequence`]), so device
/// 1's drain and gate pins are device 0's mirrored about the cell's centre —
/// the matched interconnect a differential route copies (MAT-11; Karmokar et
/// al. ASP-DAC 2022 §V-A, unequal access skews a matched pair). The centroids
/// coincide only where such an order exists; otherwise they sit one drain
/// pair apart, which `MatchedSet` prices, so the search trades the two.
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
/// its gate contact's (a licon landing on poly): then a contact at the far end
/// too pays for itself. `false` when the deck characterises either not.
fn two_ended_gate_pays(process: &dyn Process, w: i32, l: i32) -> bool {
    match (process.sheet_ohm("poly"), process.cut_ohm("licon", "poly")) {
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
            // Split when one-finger members leave no room for two bars in a
            // row: the stack stays one merged cell.
            let chain = |split_gates| Mosfet { nf, style: Pattern::Chain, dummies_per_edge: dummies, split_gates, mirror_pins: false, rows: 1, double_gate: false };
            return vec![Some(chain(false)).filter(|v| v.bars_fit(&s.dev_nf, n_dev)).unwrap_or_else(|| chain(true))];
        }
        // Interdig is never offered: an ABAB boundary between devices lands on
        // a drain region and shorts two drains over shared diffusion. Blocks
        // only where every boundary can sit on a source ([`legal_row`]): all
        // even counts from S, two odd ones from D; mixed parity is not offered.
        let blocks_legal = |dev_nf: &[u16]| {
            let blocks: Vec<usize> = dev_nf.iter().enumerate().flat_map(|(d, &n)| std::iter::repeat_n(d, usize::from(n))).collect();
            legal_row(&blocks, true) || legal_row(&blocks, false)
        };
        let mut styles = if blocks_legal(&s.dev_nf) { vec![(Pattern::Single, false, false)] } else { Vec::new() };
        if pattern::diffusion_cc_row(&s.dev_nf, Outer::Drain).is_some() {
            styles.push((Pattern::Cc1d, false, false));
            // Split gates, then mirror pins, last: indices of the existing
            // variants stay stable. Equal pairs only: mirror pins swap two
            // equal devices by reflection.
            if n_dev == 2 && s.dev_nf[0] == s.dev_nf[1] {
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
        // Blocks with split gates too: the generator cannot see gate nets, so
        // private gates (bars in alternate rows) and shared ones are both
        // offered; cellgen's gate privacy filter and the search choose.
        if n_dev > 1 && styles.contains(&(Pattern::Single, false, false)) {
            styles.push((Pattern::Single, true, false));
        }
        // Two rows when every member splits evenly and each half still has
        // its order (a centroid order at nf/2, mirror pairs at nf/2, blocks
        // legal at nf/2: three 2-finger members halve to `0 1 2`, whose two
        // boundaries cannot both sit on a source).
        let half = nf / 2;
        let two_rows = |style: Pattern, mirror: bool| {
            nf % 2 == 0
                && s.dev_nf.iter().all(|&n| n % 2 == 0)
                && s.dev_nf.iter().map(|&n| u32::from(n)).sum::<u32>() >= 4
                && match (style, mirror) {
                    (_, true) => half % 2 == 0,
                    (Pattern::Cc1d, false) => pattern::diffusion_cc_row(&s.dev_nf.iter().map(|&n| n / 2).collect::<Vec<_>>(), Outer::Drain).is_some(),
                    _ => blocks_legal(&s.dev_nf.iter().map(|&n| n / 2).collect::<Vec<_>>()),
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
            .filter(|v| v.bars_fit(&s.dev_nf, n_dev))
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let s = group_sizing(group, constraints, process);
        // A row whose bars cannot drop a boundary cut (never enumerated)
        // draws split, as `enumerate` would offer it.
        if !self.split_gates && !self.bars_fit(&s.dev_nf, group.devices.len()) {
            return Mosfet { split_gates: true, double_gate: false, ..self.clone() }.draw(group, constraints, process);
        }
        let orders = self.row_orders(&s.dev_nf, group.devices.len());
        let row0 = self.draw_row(group, constraints, process, &orders[0]);
        let mut m = match orders.get(1) {
            Some(second) => stack_on_tap(&row0, &self.draw_row(group, constraints, process, second), process),
            None => row0,
        };
        // Gate resistance per owner (CELL-19): a finger's distributed poly,
        // `R□·W/(k·L)` (k = 3 one-ended, 12 two-ended; Razavi §19.2.1), plus
        // its gate cut, over the owner's `N_f` fingers in parallel.
        if let (Some(sq), Some(cut)) = (process.sheet_ohm("poly"), process.cut_ohm("licon", "poly")) {
            let k = if self.double_gate { 12.0 } else { 3.0 };
            let finger = sq * s.unit_w as f32 / (k * s.unit_l.max(1) as f32) + cut;
            m.figures.gate_ohm = (0..group.devices.len())
                .map(|di| (di as u8, finger / orders.iter().flatten().filter(|&&d| d == di).count().max(1) as f32))
                .collect();
        }
        m
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
    let mut sd: BTreeMap<(u8, &'static str), (i64, i64)> = BTreeMap::new();
    for &(o, t, ar, p) in a.figures.sd.iter().chain(&b.figures.sd) {
        let e = sd.entry((o, t)).or_default();
        *e = (e.0 + ar, e.1 + p);
    }
    let mut m = out.finish();
    m.figures.sd = sd.into_iter().map(|((o, t), (a, p))| (o, t, a, p)).collect();
    m
}

impl Mosfet {
    /// The finger order (device index per finger) of each drawn row: the
    /// first, then for two rows the first relabelled (a pair) or reversed.
    fn row_orders(&self, dev_nf: &[u16], n_dev: usize) -> Vec<Vec<usize>> {
        let rows = self.rows.clamp(1, 2);
        let nf = (self.nf / rows).max(1);
        let dev_nf: Vec<u16> = dev_nf.iter().map(|&n| (n / rows).max(1)).collect();
        let first = if self.mirror_pins {
            mirror_sequence(usize::from(nf))
        } else if self.style == Pattern::Chain {
            dev_nf.iter().enumerate().flat_map(|(d, &n)| std::iter::repeat_n(d, usize::from(n))).collect()
        } else {
            finger_sequence(self.style, &dev_nf)
        };
        if rows == 1 {
            return vec![first];
        }
        let second = if n_dev == 2 { first.iter().map(|&d| 1 - d).collect() } else { first.iter().rev().copied().collect() };
        vec![first, second]
    }

    /// Whether every row's gate bars fit ([`bar_cuts`]): split rows put
    /// odd-indexed devices on the top bar row.
    fn bars_fit(&self, dev_nf: &[u16], n_dev: usize) -> bool {
        let same_row = |a: usize, b: usize| !self.split_gates || a % 2 == b % 2;
        self.row_orders(dev_nf, n_dev).iter().all(|seq| bar_cuts(seq, same_row, self.mirror_pins && !self.split_gates).is_some())
    }

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
        // Dummy L = min(active L, microloading reach): H13-26, a dummy wider
        // than the process's local-oxide-thinning reach draws no benefit, so a
        // long active gate keeps its dummies short rather than growing them to
        // match.
        let dummy_l = match r("dummy_max_l_nm", 0) {
            cap if cap > 0 && gate_l > cap => cap,
            _ => gate_l,
        };
        let finger_w = s.unit_w;
        let m1_pitch = m1_land(process);
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
        // A gate bar's worst-case overhang past an end gate: a lattice-snapped
        // cut plus its poly enclosure.
        let bar_over = ((ct + 2 * licon_poly_enc + lat - gate_l + 1) / 2).max(0);
        // The skirt's: its cuts enclosed `licon_poly_enc` on x (the deck's
        // opposite-sides enclosure goes on y, where the skirt is tall).
        let skirt_over = ((ct + 2 * licon_poly_enc + lat - dummy_l + 1) / 2).max(0);
        // With dummies, at least the inner gate-to-gate gap: every finger then
        // sees gates on both sides at one pitch (poly spacing effect, PSE;
        // Hastings §13.3 r9: end dummies at the array's pitch).
        // Gate 0 sits where the inner S/D cuts, centred between gates at the
        // lattice pitch, land exactly on the cut lattice (gates then sit off
        // it: the pitch has no snap slack). An end region's cut is centred to
        // the nearest lattice step ([`edge_cut`]) and must keep `gate_space`
        // from a dummy, so a dummied end grows by lattice steps until it does.
        let on_lattice = |v: i32| v + (-(v + gate_l + (sd_w - ct) / 2)).rem_euclid(lat);
        let edge_cut = |start: i32, w: i32| snap_cut(start + w / 2 - ct / 2 + lat / 2, lat);
        let holds = |start: i32, w: i32| {
            let p = edge_cut(start, w);
            p - start >= gate_space && start + w - p - ct >= gate_space
        };
        let sd_edge = if nd > 0 {
            let clear = r("poly_min_spacing", 0).max(process.space("poly").unwrap_or(0)) + if self.split_gates || self.double_gate { bar_over + skirt_over } else { bar_over.max(skirt_over) };
            let mut e = on_lattice(clear.max(pitch - gate_l).max(ct + 2 * gate_space));
            while !(holds(0, e) && holds(e + (n_fingers - 1) * pitch + gate_l, e)) {
                e += lat;
            }
            e
        } else {
            on_lattice(sd_end)
        };
        let d_step = dummy_l + sd_end;
        let gates_end = sd_edge + (n_fingers - 1) * pitch + gate_l;
        // LOD moat where devices share a row: their fingers sit at different
        // distances from the diffusion ends (ABBA: A owns both ends), so the
        // diffusion runs on past the outer dummy (bulk-tied, no signal
        // junction grows) until SA/SB are long enough that the stress term
        // fades (Hastings §13.3 r9). A lone device, a parallel group and a
        // series stack have no such mismatch.
        // ponytail: today's 3 µm; CELL-12 reads mos_env(class)
        let moat = if n_dev > 1 && nd > 0 && self.style != Pattern::Chain { process.tier("lod_moat_ext_nm", MatchClass::Minimal).unwrap_or(0) } else { 0 };
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
            .max(if self.split_gates { ct + 2 * licon_poly_side + r("poly_min_spacing", 0) } else { 0 });
        // Snapped down (away from the diff) so the cut sits on the lattice.
        // The bar's distance below the stub's minimum (CELL-12 sets it by
        // matching class).
        let bar_gap = 0;
        let pad_y = snap_cut(-(poly_ext + stub + bar_gap), lat);
        let stub = -pad_y - poly_ext;
        // S/D regions: region 0 is S iff `s0`, chosen so every inter-device
        // boundary is a shared source ([`legal_row`]): a single device or an
        // even-count block row from S, a centroid row from D. Mirror pins pair
        // fingers on drains: region 0 is S. A chain restarts per member.
        let chain = self.style == Pattern::Chain;
        let s0 = if self.mirror_pins { true } else { legal_row(sequence, true) };
        let is_s = |region: i32| (region % 2 == 0) == s0;
        debug_assert!(chain || self.mirror_pins || legal_row(sequence, s0), "{sequence:?}");
        // Terminal of the finger at `idx` on its left (`right_side = false`)
        // or right side.
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
        // Gate bar rows: a horizontal poly bar per device and row, its cuts
        // `licon_poly_side` inside both long edges (the deck's opposite-sides
        // enclosure, on y), `licon_poly_enc` past the end cuts on x.
        let pad_h = ct + 2 * licon_poly_side;
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
        let top_pad_y = top_cut_y - licon_poly_side;
        let top_pad_top = top_pad_y + pad_h;
        let top_li_top = top_cut_y - li_side + pad_li_h;
        let up = |di: usize| self.split_gates && di % 2 == 1;
        // Which ends of a device's fingers carry a bar.
        let top_end = |di: usize| up(di) || self.double_gate;
        let bottom_end = |di: usize| !up(di);
        // Unsplit mirror pins run one bar under the whole row (a shared-gate
        // pair).
        let shared = self.mirror_pins && !self.split_gates;
        let cut = bar_cuts(sequence, |a, b| up(a) == up(b), shared).expect("enumerate filters on bars_fit");
        // Per (top row, device) bar — device 0 for a shared one: the gates'
        // x span and the cuts' x span.
        let mut bars: BTreeMap<(bool, usize), ((i32, i32), Option<(i32, i32)>)> = BTreeMap::new();
        let mut pinned = vec![false; n_dev];
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
            // Gate cut (the deck requires a cut to reach poly), centred on the
            // finger at the lattice.
            let cut_x = snap_cut(gx + gate_l / 2 - ct / 2, lat);
            for (top, cy_) in [(true, top_cut_y), (false, bot_cut_y)] {
                if !(if top { top_end(di) } else { bottom_end(di) }) {
                    continue;
                }
                if cut[idx] {
                    b.rect(licon, Rect { x: cut_x, y: cy_, w: ct, h: ct });
                }
                let e = bars.entry((top, if shared { 0 } else { di })).or_insert(((gx, gx + gate_l), None));
                e.0 = (e.0 .0.min(gx), e.0 .1.max(gx + gate_l));
                if cut[idx] {
                    e.1 = Some(e.1.map_or((cut_x, cut_x), |(c0, c1)| (c0.min(cut_x), c1.max(cut_x))));
                }
            }
            // One gate pin per device, on the end its gate leaves by: its
            // first cut finger; mirror pins put device 1's on its last, device
            // 0's image.
            let last = sequence.iter().zip(&cut).rposition(|(&d, &c)| d == di && c) == Some(idx);
            let pin_here = cut[idx] && if self.mirror_pins && di == 1 { last } else { !pinned[di] };
            if pin_here {
                pinned[di] = true;
                let cut_y = if up(di) { top_cut_y } else { bot_cut_y };
                b.pin(pin(di, "G", Rect { x: cut_x, y: cut_y, w: ct, h: ct }, li));
            }
        }

        // Each bar joins its fingers into one gate in poly, and an li strap
        // over its cuts joins them in metal too (Hastings rule 22). A
        // two-ended gate's top strap carries a second pin: the router ties
        // both ends in metal.
        for (&(top, di), &((x0, x1), cuts)) in &bars {
            let (c0, c1) = cuts.expect("bar_cuts keeps a cut per device");
            let bx0 = x0.min(c0 - licon_poly_enc);
            let bx1 = x1.max(c1 + ct + licon_poly_enc);
            b.rect(poly, Rect { x: bx0, y: if top { top_pad_y } else { pad_y }, w: bx1 - bx0, h: pad_h });
            // li `li_side` past the cuts toward the diff, the rest away from it.
            let ly = if top { top_cut_y - li_side } else { bot_cut_y + ct + li_side - pad_li_h };
            b.rect(li, Rect { x: c0 - li_enc, y: ly, w: c1 + ct + li_side - (c0 - li_enc), h: pad_li_h });
            if top && self.double_gate {
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
        // Junction figures per (owner, terminal) (CELL-19): the regions between
        // the outer active gates only.
        let mut sd: BTreeMap<(u8, &'static str), (i64, i64)> = BTreeMap::new();
        for region in 0..=n_fingers {
            let end = region == 0 || region == n_fingers;
            let rw = i64::from(if end { sd_edge } else { pitch - gate_l });
            let (area, perim) = (rw * i64::from(finger_w), 2 * rw + if end && nd == 0 { i64::from(finger_w) } else { 0 });
            let sides: Vec<(u8, &'static str)> = [
                (region > 0).then(|| (sequence[region as usize - 1] as u8, term(region - 1, true))),
                (region < n_fingers).then(|| (sequence[region as usize] as u8, term(region, false))),
            ]
            .into_iter()
            .flatten()
            .collect();
            // Two different sides share the region half and half.
            let n = if sides.first() == sides.last() { 1 } else { 2 };
            for &k in &sides[..n] {
                let e = sd.entry(k).or_default();
                *e = (e.0 + area / n as i64, e.1 + perim / n as i64);
            }
            let px = if region == 0 {
                edge_cut(0, sd_edge)
            } else if region == n_fingers {
                edge_cut(gates_end, sd_edge)
            } else {
                snap_cut((2 * region - 1) * pitch / 2 + sd_edge + gate_l / 2 - ct / 2, lat)
            };
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
            let left = -(k + 1) * dummy_l - k * sd_end;
            let right = gates_end + sd_edge + k * d_step;
            for (edge, (dx, rx)) in [(left, left - sd_end), (right, right + dummy_l)].into_iter().enumerate() {
                b.rect(poly, Rect { x: dx, y: -poly_ext, w: dummy_l, h: finger_w + 2 * poly_ext });
                let cx = snap_cut(dx + dummy_l / 2 - ct / 2, lat);
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
                    l: dummy_l,
                });
            }
        }
        let stub_top = licon_y + ct + licon_poly_side;
        let dpad_w = dummy_l.max(ct + 2 * licon_poly_enc);
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
                .map(|k| if e == 0 { -(k + 1) * dummy_l - k * sd_end } else { gates_end + sd_edge + k * d_step })
                .map(|dx| snap_cut(dx + dummy_l / 2 - ct / 2, lat))
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
                // ponytail: today's 3 µm; CELL-12 reads mos_env(class)
                let wpe_halo = if matched { process.tier("wpe_clearance_nm", MatchClass::Moderate).unwrap_or(0) } else { 0 };
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
        let mut m = b.finish();
        m.figures.sd = sd.into_iter().map(|((o, t), (a, p))| (o, t, a, p)).collect();
        m
    }
}

/// Inner S/D region width and gate-to-gate pitch of a row at gate length
/// `gate_l`, nm: the contacted pitch at the deck's minimum (Hastings eqs
/// 13.7–13.9). `sd_w = max(sd_width, ct + 2·gate_space, ct + 2·diff_enc,
/// li_col + li_space − gate_l, m1_land − gate_l)` with `li_col = ct + li_enc
/// + li_side`: a cut clearing both gates, the end cut's diff enclosure, facing
/// li columns across one gate and met1 landings ([`m1_land`]) across one gate.
/// The pitch is `sd_w + gate_l`, grown to a cut-lattice multiple; gate cuts
/// sit off the diffusion on a bar.
#[must_use]
pub fn sd_and_pitch(process: &dyn Process, gate_l: i32) -> (i32, i32) {
    let r = |name: &str, default: i32| process.rule(name, default);
    let ct = dim(process, "contact");
    let li_enc = r("li_encloses_licon", 0).max(process.enclosure("li", "licon").unwrap_or(0));
    let li_side = r("li_encloses_licon_one_side", 0).max(li_enc).max(process.endcap("li", "licon").unwrap_or(0));
    let diff_enc = r("diff_encloses_licon", 0).max(process.enclosure("diff", "licon").unwrap_or(0));
    let gate_space = r("licon_to_gate_spacing", 0).max(process.space_between("licon", "poly").unwrap_or(0));
    let sd_w = r("sd_width", 0)
        .max(ct + 2 * gate_space)
        .max(ct + 2 * diff_enc)
        .max(ct + li_enc + li_side + r("li_min_spacing", 0).max(process.space("li").unwrap_or(0)) - gate_l)
        .max(m1_land(process) - gate_l);
    // A lattice-multiple pitch: every inner cut then lands on the lattice.
    let sd_w = sd_w + (-(sd_w + gate_l)).rem_euclid(cut_lattice(process));
    (sd_w, sd_w + gate_l)
}

/// Worst diffusion-to-own-tap distance of a drawn MOS macro, nm: the max over
/// every diffusion rect's corners (distance to a rect is convex, so a corner is
/// the worst point) of the Euclidean distance to the nearest tap rect, where a
/// tap rect is a `tap`-layer shape holding a `B` pin. `None` when no tap holds
/// a `B` pin. LU.2/LU.3 measure the same thing on the final GDS.
#[must_use]
pub fn tap_reach(m: &Macro, process: &dyn Process) -> Option<i32> {
    let (tap, diff) = (req(process, "tap"), req(process, "diff"));
    let holds_b = |r: &Rect| m.pins.iter().any(|p| p.name.ends_with(":B") && r.x <= p.at.x && r.y <= p.at.y && p.at.x + p.at.w <= r.x + r.w && p.at.y + p.at.h <= r.y + r.h);
    let taps: Vec<Rect> = m.shapes.iter().filter(|s| s.layer == tap && holds_b(&s.rect)).map(|s| s.rect).collect();
    if taps.is_empty() {
        return None;
    }
    // Squared distance from a point to a rect, i64.
    let d2 = |x: i32, y: i32, r: &Rect| {
        let dx = i64::from((r.x - x).max(x - (r.x + r.w)).max(0));
        let dy = i64::from((r.y - y).max(y - (r.y + r.h)).max(0));
        dx * dx + dy * dy
    };
    let worst = m
        .shapes
        .iter()
        .filter(|s| s.layer == diff && !taps.contains(&s.rect))
        .flat_map(|s| {
            let r = s.rect;
            [(r.x, r.y), (r.x + r.w, r.y), (r.x, r.y + r.h), (r.x + r.w, r.y + r.h)]
        })
        .map(|(x, y)| taps.iter().map(|t| d2(x, y, t)).min().unwrap_or(0))
        .max()
        .unwrap_or(0);
    Some((worst as f64).sqrt().ceil() as i32)
}

/// [`tap_reach`] ≤ `process.rule("tie_max_dist_nm", i32::MAX)`; a macro with no
/// diffusion (the empty placeholder) passes.
#[must_use]
pub fn taps_in_reach(m: &Macro, process: &dyn Process) -> bool {
    let diff = req(process, "diff");
    if !m.shapes.iter().any(|s| s.layer == diff) {
        return true;
    }
    tap_reach(m, process).is_some_and(|r| r <= process.rule("tie_max_dist_nm", i32::MAX))
}

/// Widest finger, nm (grid-snapped down), whose drawn rows keep every
/// diffusion point within `tie_max_dist_nm` of the tap strip; `i32::MAX` when
/// the deck states no reach. The strip sits a fixed pad stack above the
/// finger, so the reach is `W + offset`: the offset is measured on every
/// variant of probe groups (N/P, one device one finger and a two-device
/// two-finger pair, dummies off and on) at `W = tie_max/2` and gate length
/// `l`, and the worst one kept. ponytail: a probe sweep, not
/// an analytic `tap_y0` (it is a max over ~20 `draw_row` locals); a variant the
/// probe misses still meets [`taps_in_reach`] in cellgen's filter.
#[must_use]
pub fn max_finger_for_taps(process: &dyn Process, l: i32) -> i32 {
    use analog::cell::{SeriesParallel, Unitization};
    let tie_max = process.rule("tie_max_dist_nm", i32::MAX);
    if tie_max == i32::MAX {
        return i32::MAX;
    }
    let grid = process.grid().max(1);
    let w = tie_max / 2 / grid * grid;
    let mut offset = 0;
    for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
        for (n, nf) in [(1u16, 1u16), (2, 2)] {
            for dummy_required in [false, true] {
                let group = DeviceGroup { devices: (0..n).map(pnr_core::DeviceId).collect() };
                let mut c = Constraints::default();
                c.unitization.push(Unitization {
                    devices: group.devices.clone(),
                    device_type: kind,
                    dev_nf: vec![nf; usize::from(n)],
                    target_ratio: vec![1; usize::from(n)],
                    unit_w: w,
                    unit_l: l,
                    series_parallel: SeriesParallel::Parallel,
                    dummy_required,
                    route_matching_required: false,
                    class: None,
                    series: Vec::new(),
                    style: None,
                });
                for v in Mosfet::enumerate(&group, &c, process) {
                    if let Some(r) = tap_reach(&v.draw(&group, &c, process), process) {
                        offset = offset.max(r - w);
                    }
                }
            }
        }
    }
    (tie_max - offset).div_euclid(grid) * grid
}

/// Centre-to-centre pitch of two met1 landings on mcon, from deck values
/// only: `mcon + 2·max(enc, endcap) + met1 space` (Hastings eq. 13.9), the
/// plain spacing: two landings are far under any wide-metal threshold.
fn m1_land(process: &dyn Process) -> i32 {
    process.width("mcon").unwrap_or(0)
        + 2 * process.enclosure("met1", "mcon").unwrap_or(0).max(process.endcap("met1", "mcon").unwrap_or(0))
        + process.min_space("met1").unwrap_or(0)
}

/// Which fingers of `seq` (device index per finger) get a gate cut on a
/// row's bar. All of them when one bar is `shared` by the row. Otherwise two
/// devices abutting in one row (`same_row`) with disjoint finger spans have
/// bars a gate gap apart, too close for poly spacing with a cut overhanging
/// both boundary gates: the right finger drops its cut if its device keeps
/// another, else the left one; `None` when neither does. Overlapping spans
/// (an interleave) merge into one poly island anyway and keep every cut.
fn bar_cuts(seq: &[usize], same_row: impl Fn(usize, usize) -> bool, shared: bool) -> Option<Vec<bool>> {
    let mut cut = vec![true; seq.len()];
    if shared {
        return Some(cut);
    }
    let span = |d: usize| (seq.iter().position(|&x| x == d).unwrap_or(0), seq.iter().rposition(|&x| x == d).unwrap_or(0));
    let keeps_other = |cut: &[bool], i: usize| seq.iter().zip(cut).enumerate().any(|(j, (&d, &c))| j != i && d == seq[i] && c);
    for i in 1..seq.len() {
        let (a, b) = (seq[i - 1], seq[i]);
        let ((a0, a1), (b0, b1)) = (span(a), span(b));
        if a == b || !same_row(a, b) || !cut[i - 1] || (a0 <= b1 && b0 <= a1) {
            continue;
        }
        if keeps_other(&cut, i) {
            cut[i] = false;
        } else if keeps_other(&cut, i - 1) {
            cut[i - 1] = false;
        } else {
            return None;
        }
    }
    Some(cut)
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

/// Every inter-device boundary of `seq` (device index per finger) lies on a
/// source region, with region 0 a source iff `s0`. Region i sits left of
/// finger i; region `seq.len()` is the right end.
pub(crate) fn legal_row(seq: &[usize], s0: bool) -> bool {
    (1..seq.len()).all(|i| seq[i] == seq[i - 1] || ((i % 2 == 0) == s0))
}

/// Whether `enumerate` offers a centroid-exact row for these per-member finger
/// counts: a route-matched equal pair draws only mirror orders
/// ([`mirror_sequence`] exact), anything else a [`pattern::diffusion_cc_row`].
#[must_use]
pub fn cc_row_exists(dev_nf: &[u16], route_matched: bool) -> bool {
    let cc = pattern::diffusion_cc_row(dev_nf, Outer::Drain).is_some();
    if dev_nf.len() == 2 && dev_nf[0] == dev_nf[1] && route_matched {
        // `cc` excludes odd nf, where the mirror order is empty and its
        // offset a vacuous 0.
        cc && mirror_offset(usize::from(dev_nf[0])) == 0
    } else {
        cc
    }
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
    mirror_pairs(nf).into_iter().flat_map(|d| [d, d]).collect()
}

/// [`mirror_sequence`]'s order, one entry per drain pair.
fn mirror_pairs(nf: usize) -> Vec<usize> {
    let half = nf / 2;
    let build = |h: u32| -> Vec<usize> {
        let first: Vec<usize> = (0..half).map(|i| ((h >> (half - 1 - i)) & 1) as usize).collect();
        first.iter().copied().chain(first.iter().rev().map(|&d| 1 - d)).collect()
    };
    if half == 0 {
        Vec::new()
    } else if half <= 16 {
        (0..1u32 << half).map(build).min_by_key(|p| pair_offset(p)).unwrap_or_default()
    } else {
        build(0b0101_0101_0101_0101 & ((1 << 16) - 1))
    }
}

/// Twice device 0's pair-index sum minus the balanced one: 0 = coincident.
fn pair_offset(pairs: &[usize]) -> i64 {
    let (nf, half) = (pairs.len() as i64, pairs.len() as i64 / 2);
    let sum: i64 = pairs.iter().enumerate().filter(|&(_, &d)| d == 0).map(|(i, _)| i as i64).sum();
    (2 * sum - half * (nf - 1)).abs()
}

/// The centroid offset of the order [`mirror_sequence`] draws at `nf`.
fn mirror_offset(nf: usize) -> i64 {
    pair_offset(&mirror_pairs(nf))
}

/// Device index per finger slot: `Cc1d` the diffusion-legal centroid row
/// ([`pattern::diffusion_cc_row`], ratioed counts too) when one exists;
/// everything else each device's fingers in a block.
fn finger_sequence(style: Pattern, dev_nf: &[u16]) -> Vec<usize> {
    if style == Pattern::Cc1d {
        if let Some(seq) = pattern::diffusion_cc_row(dev_nf, Outer::Drain) {
            return seq;
        }
    }
    dev_nf.iter().enumerate().flat_map(|(d, &n)| std::iter::repeat_n(d, usize::from(n))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sky130: 48.2·10000/(3·150) = 1071 Ω of finger poly against a 152 Ω
    /// gate cut (`licon_po`), so a second gate contact pays.
    #[test]
    fn two_ended_gate_still_pays_on_sky130() {
        let pdk = verify::Pdk::builtin("sky130").unwrap();
        assert!(two_ended_gate_pays(&pdk, 10_000, 150));
    }

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
            // CELL-30: gate L on both sides of the dummy_max_l_nm cap (3000).
            for (w, l) in [(420, 1000), (420, 16_200), (420, 64_800), (2160, 9600)] {
                dirty.extend(testkit::dirty::<Mosfet>(kind, 1, 1, w, l, &pdk));
            }
            // CELL-10: ratioed mirrors.
            for counts in RATIOED {
                let (g, mut c) = testkit::group_of(kind, 2, counts[0], 1680, 150);
                c.unitization[0].dev_nf = counts.to_vec();
                for dummies in [false, true] {
                    c.unitization[0].dummy_required = dummies;
                    dirty.extend(testkit::dirty_group::<Mosfet>(&g, &c, &pdk).into_iter().map(|d| format!("{kind:?} {counts:?} dummies={dummies} {d}")));
                }
            }
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
            for (n, nf, w, l) in [
                (1usize, 1u16, 1680, 150),
                (2, 2, 1680, 150),
                (2, 2, 5000, 1000),
                (2, 2, 10_000, 150),
                (4, 4, 1680, 150),
                // CELL-30: gate L on both sides of the dummy_max_l_nm cap (3000).
                (1, 1, 420, 150),
                (1, 1, 420, 1000),
                (1, 1, 420, 16_200),
                (1, 1, 420, 64_800),
                (1, 1, 2160, 9600),
            ] {
                for dummies in [false, true] {
                    let (g, mut c) = testkit::group_of(kind, n, nf, w, l);
                    c.unitization[0].dummy_required = dummies;
                    wrong.extend(miscounted(&g, &c, &pdk).into_iter().map(|e| format!("{kind:?} n={n} nf={nf} w={w} dummies={dummies} {e}")));
                }
            }
            for counts in RATIOED {
                for dummies in [false, true] {
                    let (g, mut c) = testkit::group_of(kind, 2, counts[0], 1680, 150);
                    c.unitization[0].dev_nf = counts.to_vec();
                    c.unitization[0].dummy_required = dummies;
                    wrong.extend(miscounted(&g, &c, &pdk).into_iter().map(|e| format!("{kind:?} {counts:?} dummies={dummies} {e}")));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// CELL-10 sweep members: 1:2, 1:3, 1:2 at four fingers a unit.
    const RATIOED: [&[u16]; 3] = [&[2, 4], &[2, 6], &[4, 8]];

    /// Variants whose extracted MOS count is not their fingers plus dummies.
    fn miscounted(g: &DeviceGroup, c: &Constraints, pdk: &verify::Pdk) -> Vec<String> {
        let mut out = Vec::new();
        for (i, v) in Mosfet::enumerate(g, c, pdk).iter().enumerate() {
            let m = v.draw(g, c, pdk);
            let spice = verify::extract_spice(&m.shapes, &[], pdk, verify::Detail::Schematic).unwrap_or_default();
            let got = spice.lines().filter(|l| l.starts_with('M')).count();
            if got != m.units.len() + m.dummies.len() {
                out.push(format!("#{i}: {got} vs {}", m.units.len() + m.dummies.len()));
            }
        }
        out
    }

    /// Every admitted centroid order is balanced, centred, and never puts two
    /// different devices across a drain (even) region — a short only LVS sees.
    #[test]
    fn a_centroid_order_is_symmetric_and_never_abuts_two_drains() {
        for n_dev in 2..=6usize {
            for nf in 1..=12usize {
                let Some(seq) = pattern::diffusion_cc_row(&vec![nf as u16; n_dev], Outer::Drain) else { continue };
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
        let row = |c: &[u16]| pattern::diffusion_cc_row(c, Outer::Drain);
        assert!(row(&[1; 4]).is_none());
        assert!(row(&[2; 4]).is_none());
        assert!(row(&[4; 4]).is_some());
        assert!(row(&[2, 2]).is_some());
        assert!(row(&[1, 1]).is_none());
    }

    /// CELL-10: the region rule, by hand.
    #[test]
    fn legal_row_matches_the_region_rule() {
        assert!(legal_row(&[0, 1, 1, 0], false) && !legal_row(&[0, 1, 1, 0], true));
        assert!(legal_row(&[0, 0, 1, 1], true) && !legal_row(&[0, 0, 1, 1], false));
        assert!(legal_row(&[0, 1], false));
    }

    /// CELL-10: `[2, 2]` blocked starts on S, so the two drains never share a pad.
    #[test]
    fn a_blocked_even_row_puts_boundaries_on_sources() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Single).expect("a blocked [2, 2] is offered");
        let m = v.draw(&g, &c, &pdk);
        let d = |i: usize| m.pins.iter().filter(move |p| p.name == format!("d{i}:D")).map(|p| p.at).collect::<Vec<_>>();
        assert!(d(0).iter().all(|a| !d(1).contains(a)), "a pad carries both drains");
    }

    /// CELL-10: a 1:2 mirror `[2, 4]` merges as A BBBB A with no shared drain
    /// pad, DRC/ERC clean.
    #[test]
    fn a_ratioed_mirror_merges() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, mut c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        c.unitization[0].dev_nf = vec![2, 4];
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Cc1d && v.rows == 1).expect("a centroid [2, 4] is offered");
        assert_eq!(finger_sequence(v.style, &[2, 4]), [0, 1, 1, 1, 1, 0]);
        let m = v.draw(&g, &c, &pdk);
        let d = |i: usize| m.pins.iter().filter(move |p| p.name == format!("d{i}:D")).map(|p| p.at).collect::<Vec<_>>();
        assert!(d(0).iter().all(|a| !d(1).contains(a)), "a pad carries both drains");
        let dirty = testkit::dirty_group::<Mosfet>(&g, &c, &pdk);
        assert!(dirty.is_empty(), "{dirty:?}");
    }

    /// CELL-10: the predicate FLOW-16 folds by agrees with what `enumerate` offers.
    #[test]
    fn cc_row_exists_follows_the_offered_variants() {
        assert!(!cc_row_exists(&[2, 2], true), "nf=2 mirror orders are one pair off");
        assert!(cc_row_exists(&[8, 8], true));
        assert!(!cc_row_exists(&[1, 1], true));
        assert!(cc_row_exists(&[2, 2], false));
        assert!(!cc_row_exists(&[1, 2], false));
    }

    /// A long active gate (well past the microloading reach) keeps its dummy
    /// gates capped at `dummy_max_l_nm`, so the dummy ring's footprint stops
    /// growing with the active L (H13-26).
    #[test]
    fn a_long_gate_keeps_short_dummies() {
        use crate::testkit;
        use pnr_core::DeviceKind::Pmos;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, mut c) = testkit::group_of(Pmos, 1, 1, 420, 64_800);
        c.unitization[0].dummy_required = false;
        let without: Vec<Macro> = Mosfet::enumerate(&g, &c, &pdk).iter().map(|v| v.draw(&g, &c, &pdk)).collect();
        c.unitization[0].dummy_required = true;
        let with: Vec<Macro> = Mosfet::enumerate(&g, &c, &pdk).iter().map(|v| v.draw(&g, &c, &pdk)).collect();
        assert_eq!(without.len(), with.len(), "dummy_required must not change the variant count");
        let mut findings = Vec::new();
        for (i, m) in with.iter().enumerate() {
            for dm in &m.dummies {
                assert_eq!(dm.l, 3000, "variant #{i}: dummy gate l {} not capped", dm.l);
            }
            let w_nd = without[i].bbox.w;
            assert!(m.bbox.w <= w_nd + 2 * (3_000 + 500), "variant #{i}: bbox.w {} grew past the capped dummy footprint (no-dummy {w_nd})", m.bbox.w);
            assert!(m.bbox.w <= 73_000, "variant #{i}: bbox.w {} exceeds FR-5", m.bbox.w);
            let labels = testkit::ports_with(m, &["G", "S", "B"]);
            findings.extend(testkit::findings(&m.shapes, &labels, &pdk));
            let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).unwrap_or_default();
            let got = spice.lines().filter(|l| l.starts_with('M')).count();
            assert_eq!(got, m.units.len() + m.dummies.len(), "variant #{i}: extracted {got} vs {} units+dummies", m.units.len() + m.dummies.len());
        }
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// A short active gate (below `dummy_max_l_nm`) draws full-length dummies,
    /// unchanged from before the cap existed.
    #[test]
    fn a_short_gate_keeps_full_dummies() {
        use crate::testkit;
        use pnr_core::DeviceKind::Nmos;
        let Some(pdk) = testkit::pdk() else { return };
        for l in [150, 1000] {
            let (g, mut c) = testkit::group_of(Nmos, 1, 1, 420, l);
            c.unitization[0].dummy_required = true;
            for v in Mosfet::enumerate(&g, &c, &pdk) {
                let m = v.draw(&g, &c, &pdk);
                for dm in &m.dummies {
                    assert_eq!(dm.l, l, "gate_l={l}: dummy gate l {} not left at the active L", dm.l);
                }
            }
        }
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

    /// Every split-gate variant, labelled with a private gate per member
    /// (only S and B common): ERC sees the two gates as two nets, so a poly
    /// or strap joining them is a short. The shared-strap ABBA under the same
    /// labels must short, or the labels cannot see one.
    #[test]
    fn split_gate_variants_keep_private_gates_under_erc() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let (mut dirty, mut checked, mut controls) = (Vec::new(), 0, 0);
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            for nf in [2u16, 4] {
                for dummies in [false, true] {
                    let (g, mut c) = testkit::group_of(kind, 2, nf, 1680, 150);
                    c.unitization[0].dummy_required = dummies;
                    for (i, v) in Mosfet::enumerate(&g, &c, &pdk).into_iter().enumerate() {
                        let m = v.draw(&g, &c, &pdk);
                        let rules = testkit::findings(&m.shapes, &testkit::ports_with(&m, &["S", "B"]), &pdk);
                        if v.split_gates {
                            checked += 1;
                            if !rules.is_empty() {
                                dirty.push(format!("{kind:?} nf={nf} dummies={dummies} #{i} rows={} mirror_pins={}: {rules:?}", v.rows, v.mirror_pins));
                            }
                        } else if v.style == Pattern::Cc1d && !v.mirror_pins && v.rows == 1 && !v.double_gate {
                            controls += 1;
                            assert!(!rules.is_empty(), "{kind:?} nf={nf} #{i}: the shared-strap ABBA joins both gates, yet private gate labels read clean");
                        }
                    }
                }
            }
        }
        assert!(checked > 0, "no split-gate variant enumerated");
        assert!(controls > 0, "no shared-strap ABBA enumerated to show the labels can see a short");
        assert!(dirty.is_empty(), "{}", dirty.join("\n"));
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

    /// CELL-11 / T5: sky130 at L = 150 contacts at 280 + 150 = 430 nm (was
    /// 730, set by a met1 pitch the row never draws).
    #[test]
    fn the_contacted_pitch_is_the_deck_minimum() {
        let pdk = verify::Pdk::builtin("sky130").unwrap();
        assert_eq!(sd_and_pitch(&pdk, 150), (280, 430));
    }

    /// Every gate-cut row's poly is whole bars: along the line through its
    /// cuts, each poly run is one rect holding a cut, one run per device
    /// leaving by that row (one for a shared bar) — no pads joined by a
    /// narrower strap.
    #[test]
    fn a_gate_bar_has_no_notch() {
        use crate::testkit;
        use pnr_core::{DeviceKind, Process};
        let Some(pdk) = testkit::pdk() else { return };
        let (poly, licon) = (pdk.layer("poly").unwrap(), pdk.layer("licon").unwrap());
        let ct = dim(&pdk, "contact");
        let w = 1680;
        let mut checked = 0;
        for (n, nf) in [(2usize, 2u16), (2, 8), (1, 4), (3, 2)] {
            let (g, c) = testkit::group_of(DeviceKind::Nmos, n, nf, w, 150);
            for (i, v) in Mosfet::enumerate(&g, &c, &pdk).into_iter().enumerate() {
                let m = v.draw(&g, &c, &pdk);
                let polys: Vec<Rect> = m.shapes.iter().filter(|s| s.layer == poly).map(|s| s.rect).collect();
                let on_poly = |r: &Rect| polys.iter().any(|p| p.x <= r.x && p.x + p.w >= r.x + r.w && p.y <= r.y && p.y + p.h >= r.y + r.h);
                let cuts: Vec<Rect> = m.shapes.iter().filter(|s| s.layer == licon && (s.rect.y + ct <= 0 || s.rect.y >= w) && on_poly(&s.rect)).map(|s| s.rect).collect();
                let mut ys: Vec<i32> = cuts.iter().map(|r| r.y).collect();
                ys.sort_unstable();
                ys.dedup();
                for y in ys {
                    let line = y + ct / 2;
                    let mut spans: Vec<(i32, i32)> = polys.iter().filter(|p| p.y < line && line < p.y + p.h).map(|p| (p.x, p.x + p.w)).collect();
                    spans.sort_unstable();
                    let mut runs: Vec<(i32, i32)> = Vec::new();
                    for (a, b) in spans {
                        match runs.last_mut() {
                            Some(r) if a <= r.1 => r.1 = r.1.max(b),
                            _ => runs.push((a, b)),
                        }
                    }
                    let tag = format!("n={n} nf={nf} #{i} y={y}");
                    for run in &runs {
                        assert!(polys.iter().any(|p| p.y < line && line < p.y + p.h && p.x <= run.0 && p.x + p.w >= run.1), "{tag}: run {run:?} is not one bar");
                        assert!(cuts.iter().any(|c| c.y == y && c.x >= run.0 && c.x + ct <= run.1), "{tag}: run {run:?} holds no cut");
                    }
                    let mut leaving: Vec<&str> = m.pins.iter().filter(|p| p.name.ends_with(":G") && p.at.y == y).map(|p| p.name.as_str()).collect();
                    leaving.sort_unstable();
                    leaving.dedup();
                    // A shared bar, or an unsplit centroid row whose bars overlap
                    // (one poly island, as the ABBA mirror's strap was).
                    let want = if !v.split_gates && (v.mirror_pins || v.style == Pattern::Cc1d) { 1 } else { leaving.len() };
                    assert_eq!(runs.len(), want, "{tag}: runs {runs:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 0);
    }

    /// At the minimum pitch, a dummy-free eight-finger centroid pair steps
    /// its gates exactly 430 nm.
    #[test]
    fn a_row_is_shorter() {
        use crate::testkit;
        use pnr_core::{DeviceKind, Process};
        let Some(pdk) = testkit::pdk() else { return };
        let (g, mut c) = testkit::group_of(DeviceKind::Nmos, 2, 8, 1680, 150);
        c.unitization[0].dummy_required = false;
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Cc1d && v.rows == 1).expect("a one-row centroid");
        let m = v.draw(&g, &c, &pdk);
        let poly = pdk.layer("poly").unwrap();
        let mut xs: Vec<i32> = m.shapes.iter().filter(|s| s.layer == poly && s.rect.h >= 1680).map(|s| s.rect.x).collect();
        xs.sort_unstable();
        xs.dedup();
        assert_eq!(xs.len(), 16);
        assert!(xs.windows(2).all(|w| w[1] - w[0] == 430), "{xs:?}");
    }

    #[test]
    fn bar_cuts_drops_one_boundary_cut() {
        let all = |_: usize, _: usize| true;
        assert_eq!(bar_cuts(&[0, 0, 1, 1], all, false), Some(vec![true, true, false, true]));
        assert_eq!(bar_cuts(&[0, 0, 1], all, false), Some(vec![true, false, true]));
        assert_eq!(bar_cuts(&[0, 1], all, false), None);
        assert_eq!(bar_cuts(&[0, 1, 1, 0], all, false), Some(vec![true; 4]));
        assert_eq!(bar_cuts(&[0, 1], all, true), Some(vec![true; 2]));
        // Different bar rows never constrain each other.
        assert_eq!(bar_cuts(&[0, 1], |a, b| a == b, false), Some(vec![true; 2]));
    }

    /// One finger each leaves no cut to drop: only split bars draw it, and
    /// they are clean.
    #[test]
    fn a_one_finger_pair_draws_split() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 1, 1680, 150);
        let vs = Mosfet::enumerate(&g, &c, &pdk);
        assert!(!vs.is_empty() && vs.iter().all(|v| v.split_gates));
        let dirty = testkit::dirty_group::<Mosfet>(&g, &c, &pdk);
        assert!(dirty.is_empty(), "{}", dirty.join("\n"));
    }

    /// The tap strip spans the diffusion's x range above it, so the worst
    /// diffusion point is a bottom corner (y = 0) and the reach is the strip's y.
    #[test]
    fn tap_reach_is_the_far_corner() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let tap = req(&pdk, "tap");
        let strip = |m: &Macro| {
            let b = m.pins.iter().find(|p| p.name.ends_with(":B")).expect("a B pin").at;
            m.shapes.iter().find(|s| s.layer == tap && s.rect.x <= b.x && s.rect.y <= b.y && b.x + b.w <= s.rect.x + s.rect.w && b.y + b.h <= s.rect.y + s.rect.h).expect("a strip holds B").rect
        };
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 1, 1680, 150);
        for v in Mosfet::enumerate(&g, &c, &pdk) {
            let m = v.draw(&g, &c, &pdk);
            assert_eq!(tap_reach(&m, &pdk), Some(strip(&m).y));
        }
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 4, 1680, 150);
        let vs = Mosfet::enumerate(&g, &c, &pdk);
        let reach = |rows: u16| vs.iter().find(|v| v.rows == rows && !v.double_gate).map(|v| tap_reach(&v.draw(&g, &c, &pdk), &pdk));
        assert!(reach(2).is_some(), "a two-row variant exists");
        assert_eq!(reach(2), reach(1), "the mirrored row sits as far from the shared strip");
    }

    /// T8: on every planar built-in deck, every enumerated variant keeps each
    /// diffusion point within `tie_max_dist_nm` of its own tap, up to the
    /// widest finger `folds` draws (gf180/ihp's 100 µm `max_finger_width` is
    /// clamped by [`max_finger_for_taps`]).
    #[test]
    fn every_diffusion_point_reaches_its_tap() {
        use crate::testkit;
        let mut bad = Vec::new();
        for deck in ["sky130", "gf180mcu", "ihp_sg13g2"] {
            let pdk = verify::Pdk::builtin(deck).unwrap();
            let tie_max = pdk.rule("tie_max_dist_nm", i32::MAX);
            assert!(tie_max < i32::MAX, "{deck}: no tie_max_dist_nm");
            let l = pdk.min_channel(true, "").0;
            // The widest finger `folds` draws: the deck's cap, clamped to the reach.
            let w_max = pdk.rule("max_finger_width", 0);
            assert!(w_max > 0, "{deck}: no max_finger_width");
            let w_max = w_max.min(max_finger_for_taps(&pdk, l));
            assert!(w_max >= 5000, "{deck}: reach clamps fingers to {w_max} nm");
            for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
                for w in [420, 1680, 5000, w_max] {
                    for (n, nf) in [(1usize, 1u16), (2, 1), (2, 2), (4, 4)] {
                        for dummies in [false, true] {
                            let (g, mut c) = testkit::group_of(kind, n, nf, w, l);
                            c.unitization[0].dummy_required = dummies;
                            for (i, v) in Mosfet::enumerate(&g, &c, &pdk).iter().enumerate() {
                                let m = v.draw(&g, &c, &pdk);
                                if !taps_in_reach(&m, &pdk) {
                                    bad.push(format!("{deck} {kind:?} n={n} nf={nf} W={w} dummies={dummies} variant {i}: reach {:?} > {tie_max}", tap_reach(&m, &pdk)));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// (`e`, `pitch`) of a drawn row: the end region's width (the first
    /// gate's left edge) and the unit-to-unit pitch of one row.
    fn e_and_pitch(m: &Macro, l: i32) -> (i64, i64) {
        let mut xs: Vec<i32> = m.units.iter().map(|u| u.x).collect();
        xs.sort_unstable();
        xs.dedup();
        (i64::from(xs[0] - l / 2), xs.get(1).map_or(0, |&x| i64::from(x - xs[0])))
    }

    /// CELL-19: every S/D region's area lands on some (owner, terminal).
    #[test]
    fn sd_figures_add_up() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let area = |m: &Macro| m.figures.sd.iter().map(|f| f.2).sum::<i64>();
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 1, 1680, 150);
        let m = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Single).unwrap().draw(&g, &c, &pdk);
        let (e, _) = e_and_pitch(&m, 150);
        assert_eq!(m.figures.sd, vec![(0, "D", e * 1680, 2 * e + 1680), (0, "S", e * 1680, 2 * e + 1680)]);

        let (g, mut c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        c.unitization[0].dummy_required = true;
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Cc1d && !v.split_gates && !v.mirror_pins && v.rows == 1).unwrap();
        assert!(v.dummies_per_edge > 0);
        let m = v.draw(&g, &c, &pdk);
        let (e, pitch) = e_and_pitch(&m, 150);
        assert_eq!(area(&m), 1680 * (2 * e + 3 * (pitch - 150)));

        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 4, 1680, 150);
        let m = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.rows == 2).unwrap().draw(&g, &c, &pdk);
        let (e, pitch) = e_and_pitch(&m, 150);
        assert_eq!(area(&m), 2 * 1680 * (2 * e + (pitch - 150)));
    }

    /// S D S: the inner drain is all the device's; both end sources add up.
    #[test]
    fn a_shared_drain_halves_the_area() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 1, 2, 1680, 150);
        let m = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Single && v.rows == 1).unwrap().draw(&g, &c, &pdk);
        let (e, pitch) = e_and_pitch(&m, 150);
        let get = |t: &str| m.figures.sd.iter().find(|f| f.1 == t).map(|f| (f.2, f.3)).unwrap();
        assert_eq!(get("D"), ((pitch - 150) * 1680, 2 * (pitch - 150)));
        assert_eq!(get("S").0, 2 * e * 1680);
    }

    /// Two owners on one region take half each, area and perimeter.
    #[test]
    fn two_owners_split_a_shared_region() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        // A B: the one inner S region, half to each.
        let (g, c) = testkit::group_of(DeviceKind::Nmos, 2, 1, 1680, 150);
        let m = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Single && v.rows == 1).unwrap().draw(&g, &c, &pdk);
        let (_, pitch) = e_and_pitch(&m, 150);
        let half = ((pitch - 150) * 1680 / 2, pitch - 150);
        let get = |m: &Macro, o: u8, t: &str| m.figures.sd.iter().find(|f| f.0 == o && f.1 == t).map(|f| (f.2, f.3));
        assert_eq!((get(&m, 0, "S"), get(&m, 1, "S")), (Some(half), Some(half)), "{:?}", m.figures.sd);
        // A B B A, dummies on: each A|B region is half A's, half B's.
        let (g, mut c) = testkit::group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        c.unitization[0].dummy_required = true;
        let v = Mosfet::enumerate(&g, &c, &pdk).into_iter().find(|v| v.style == Pattern::Cc1d && !v.split_gates && !v.mirror_pins && v.rows == 1).unwrap();
        let m = v.draw(&g, &c, &pdk);
        let (_, pitch) = e_and_pitch(&m, 150);
        let two_halves = ((pitch - 150) * 1680, 2 * (pitch - 150));
        assert_eq!((get(&m, 0, "S"), get(&m, 1, "S")), (Some(two_halves), Some(two_halves)), "{:?}", m.figures.sd);
    }

    /// sky130 poly 48.2 Ω/□, licon_po 152 Ω: (48.2·10000/(k·150) + 152)/4.
    #[test]
    fn gate_ohm_matches_the_formula() {
        let pdk = verify::Pdk::builtin("sky130").unwrap();
        let (g, c) = crate::testkit::group_of(DeviceKind::Nmos, 1, 4, 10_000, 150);
        let vs = Mosfet::enumerate(&g, &c, &pdk);
        for (double, want) in [(false, 305.8f32), (true, 104.9)] {
            let v = vs.iter().find(|v| v.double_gate == double && v.rows == 1).unwrap();
            let got = v.draw(&g, &c, &pdk).figures.gate_ohm;
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].0, 0);
            assert!((got[0].1 - want).abs() / want < 0.01, "double_gate={double}: {} Ω, want {want}", got[0].1);
        }
    }
}
