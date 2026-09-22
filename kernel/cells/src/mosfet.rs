//! MOSFET generator: one diffusion row of gates, contacted S/D regions, edge
//! dummies tied to a bulk tap rail, channel implant and (PMOS) nwell.

use std::collections::BTreeMap;

use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, Process, Rect};

use crate::builder::{greedy_centroid, pin, req, sizing, unitization, Builder, Sizing};
use crate::{Cell, Pattern};

/// One MOSFET variant: `nf` fingers per device arranged by `style`, with
/// `dummies_per_edge` dummy gates outside the diffusion on each side.
#[derive(Clone)]
pub struct Mosfet {
    pub nf: u16,
    pub style: Pattern,
    pub dummies_per_edge: u8,
}

/// Dummy counts on offer. `0` is withheld when the unitization sets
/// `dummy_required`; it is last so indices of the others stay stable.
const DUMMY_OPTIONS: [u8; 3] = [1, 2, 0];

impl Cell for Mosfet {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let min_dummies =
            u8::from(unitization(group, constraints).is_some_and(|u| u.dummy_required));
        // `nf` is the schematic's: `draw` gives every finger the full unit
        // width and the LVS reference emits one card per schematic finger, so
        // a refold would draw a different device.
        let nf = s.dev_nf[0];
        let n_dev = group.devices.len();
        // Interdig is never offered: an ABAB boundary between devices lands on
        // a drain region and shorts two drains over shared diffusion.
        let mut styles = vec![Pattern::Single];
        if centroid_sequence(n_dev, usize::from(nf)).is_some() {
            styles.push(Pattern::Cc1d);
        }
        styles
            .into_iter()
            .flat_map(|style| {
                DUMMY_OPTIONS
                    .iter()
                    .filter(move |&&d| d >= min_dummies)
                    .map(move |&d| Mosfet { nf, style, dummies_per_edge: d })
            })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let r = |name: &str, default: i32| process.rule(name, default);

        let diff = req(process, "diff");
        let poly = req(process, "poly");
        let li = req(process, "li");
        let licon = process.layer("licon").unwrap_or(poly);
        let u = unitization(group, constraints);
        // Matched = a multi-device unitization covers the group: draw the LOD
        // moat and WPE well halo.
        let matched = u.is_some_and(|u| u.devices.len() > 1);
        let is_pmos = u.is_some_and(|u| u.device_type == DeviceKind::Pmos);

        let ct = r("contact", 170);
        let poly_ext = r("poly_ext", 130);
        let licon_poly_enc = r("licon_poly_enc", 50);
        let licon_poly_side = r("poly_encloses_licon_one_side", licon_poly_enc).max(licon_poly_enc);
        let polycon_gap = r("polycon_to_diff_spacing", 0);
        // li over a cut: `li_enc` all round, `li_side` on one side per axis.
        let li_enc = r("li_encloses_licon", 0);
        let li_side = r("li_encloses_licon_one_side", 0).max(li_enc);
        let li_space = r("li_min_spacing", 170);
        let diff_enc = r("diff_encloses_licon", 40);
        let gate_l = s.unit_l;
        let finger_w = s.unit_w;
        let m1_pitch = r("mcon_size", 170) + 2 * r("m1_enc", 60) + r("met1_space", 140);
        // S/D width floors: met1 pitch, facing li pads across one gate, and
        // the end cut's diff enclosure.
        let sd_w = r("sd_width", 250)
            .max(m1_pitch - gate_l)
            .max(ct + li_enc + li_side + li_space - gate_l)
            .max(ct + 2 * diff_enc);
        let pitch = (sd_w + gate_l + sd_w).max(m1_pitch);

        let sequence = finger_sequence(n_dev, self.style, self.nf.max(1), &s.dev_nf);

        // LOD moat: extend diff past the outer gates on matched groups.
        let moat_ext = if matched { r("lod_moat_ext_moderate", 3000) } else { 0 };
        let diff_x_start = -moat_ext;
        let diff_x_end = sequence.len() as i32 * pitch + moat_ext;
        // One continuous diff row; extraction splits S from D at each gate
        // (`sd = diff NOT poly`).
        b.rect(diff, Rect { x: diff_x_start, y: 0, w: diff_x_end - diff_x_start, h: finger_w });

        // Gate stub length: the gate pad's met1/li must clear the S/D pad row,
        // and the gate cut must keep `polycon_gap` from the diff.
        let li_floor =
            licon_poly_side - poly_ext + ct + 2 * li_enc + li_space - (finger_w / 2 - ct / 2);
        let stub = 200
            .max(2 * (m1_pitch - finger_w / 2 - poly_ext))
            .max(li_floor)
            .max(polycon_gap + licon_poly_side + ct - poly_ext);
        let pad_y = -(poly_ext + stub);
        // Gate x-extent per device, for the strap below.
        let mut gate_span: BTreeMap<usize, (i32, i32)> = BTreeMap::new();
        for (idx, &di) in sequence.iter().enumerate() {
            let gx = idx as i32 * pitch + sd_w;
            b.rect(poly, Rect { x: gx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
            b.rect(poly, Rect { x: gx, y: pad_y, w: gate_l, h: stub + 30 });
            // Contacted gate pad (the deck requires a cut to reach poly).
            let pad_w = gate_l.max(ct + 2 * licon_poly_side);
            let pad_h = ct + licon_poly_enc + licon_poly_side;
            b.rect(poly, Rect { x: gx + gate_l / 2 - pad_w / 2, y: pad_y, w: pad_w, h: pad_h });
            let cut_x = gx + gate_l / 2 - ct / 2;
            let cut_y = pad_y + licon_poly_side;
            b.rect(licon, Rect { x: cut_x, y: cut_y, w: ct, h: ct });
            // li skirt: `li_side` rightward and downward (free space).
            let li_w = ct + li_enc + li_side;
            b.rect(li, Rect { x: cut_x - li_enc, y: cut_y - li_side, w: li_w, h: li_w });
            // One gate pin per device: the strap joins its fingers.
            if !gate_span.contains_key(&di) {
                b.pin(pin(di, "G", Rect { x: cut_x, y: cut_y, w: ct, h: ct }, li));
            }
            let e = gate_span.entry(di).or_insert((gx, gx + gate_l));
            e.0 = e.0.min(gx);
            e.1 = e.1.max(gx + gate_l);
        }

        // Gate strap: joins one device's fingers into a single gate. Per
        // device, so interleaved devices keep distinct gates.
        let poly_w = r("poly_min_width", 150);
        for &(x0, x1) in gate_span.values() {
            if x1 - x0 > gate_l {
                b.rect(poly, Rect { x: x0, y: pad_y, w: x1 - x0, h: poly_w });
            }
        }

        // S/D regions. Single device: region 0 is S. Multi-device: region 0 is
        // D, so every inter-device boundary (odd region) is a shared source.
        let multi = n_dev > 1;
        let is_s = |region: i32| (region % 2 == 0) != multi;
        let n_fingers = sequence.len() as i32;
        let cy = finger_w / 2 - ct / 2;
        for region in 0..=n_fingers {
            let cx = if region == 0 {
                sd_w / 2
            } else if region == n_fingers {
                (n_fingers - 1) * pitch + sd_w + gate_l + sd_w / 2
            } else {
                (2 * region - 1) * pitch / 2 + sd_w + gate_l / 2
            };
            let px = cx - ct / 2;
            let li_w = ct + li_enc + li_side;
            b.rect(li, Rect { x: px - li_enc, y: cy - li_enc, w: li_w, h: li_w });
            b.rect(licon, Rect { x: px, y: cy, w: ct, h: ct });
            let at = Rect { x: px, y: cy, w: ct, h: ct };
            let term = if is_s(region) { "S" } else { "D" };
            let right = (region < n_fingers).then(|| sequence[region as usize]);
            let left = (region > 0).then(|| sequence[region as usize - 1]);
            if let Some(di) = right {
                b.pin(pin(di, term, at, li));
            }
            if let Some(di) = left.filter(|&d| Some(d) != right) {
                b.pin(pin(di, term, at, li));
            }
        }

        // Dummy gates, outside the diffusion (a dummy over diff is a real,
        // unreferenced transistor), tied up to the bulk rail.
        let rise_l = 30.max(li_enc);
        let rise_r = rise_l.max(li_side);
        let tap_y0 = finger_w + 580;
        let tap_h = ct + 2 * diff_enc;
        let licon_y = finger_w + 240.max(poly_ext - 10 + licon_poly_side);
        // Riser-cut x extents per edge: each edge's dummies share one poly
        // skirt and one li strip (separate ones violate poly/li spacing).
        let mut edge_cuts: [Option<(i32, i32)>; 2] = [None, None];
        let step = gate_l + sd_w;
        for k in 0..i32::from(self.dummies_per_edge) {
            for (edge, dx) in [diff_x_start - (k + 1) * step, diff_x_end + sd_w + k * step]
                .into_iter()
                .enumerate()
            {
                b.rect(poly, Rect { x: dx, y: -poly_ext, w: gate_l, h: finger_w + 2 * poly_ext });
                let cx = dx + gate_l / 2 - ct / 2;
                b.rect(licon, Rect { x: cx, y: licon_y, w: ct, h: ct });
                let e = edge_cuts[edge].get_or_insert((cx, cx));
                e.0 = e.0.min(cx);
                e.1 = e.1.max(cx);
            }
        }
        let stub_top = licon_y + ct + licon_poly_enc;
        let dpad_w = gate_l.max(ct + 2 * licon_poly_side);
        let skirt_y = finger_w + poly_ext - 10;
        // Riser strips lap the rail's li but stay above its cut row (a grazed
        // cut reads as an under-sized contact).
        let rail_li_y = (tap_y0 + diff_enc) - li_enc;
        for (cx0, cx1) in edge_cuts.into_iter().flatten() {
            let x0 = cx0 + ct / 2 - dpad_w / 2;
            b.rect(poly, Rect { x: x0, y: skirt_y, w: (cx1 + ct / 2 + dpad_w / 2) - x0, h: stub_top - skirt_y });
        }
        for (cx0, cx1) in edge_cuts.into_iter().flatten() {
            b.rect(li, Rect {
                x: cx0 - rise_l,
                y: licon_y - rise_l,
                w: (cx1 + ct + rise_r) - (cx0 - rise_l),
                h: (rail_li_y + 40) - (licon_y - rise_l),
            });
        }

        // Bulk tap strip spanning the outer dummies (or the moat): n+ tap for
        // PMOS, p+ for NMOS. Mandatory, or the well/substrate floats.
        let dummy_span = i32::from(self.dummies_per_edge) * step;
        let tap_x0 = diff_x_start - dummy_span;
        let tap_x1 = diff_x_end + dummy_span;
        let tap_w = tap_x1 - tap_x0;
        b.rect(req(process, "tap"), Rect { x: tap_x0, y: tap_y0, w: tap_w, h: tap_h });
        let imp = if is_pmos { req(process, "nsdm") } else { req(process, "psdm") };
        let imp_enc = 125;
        b.rect(imp, Rect {
            x: tap_x0 - imp_enc,
            y: tap_y0 - imp_enc,
            w: tap_w + 2 * imp_enc,
            h: tap_h + 2 * imp_enc,
        });
        // Channel implant of the device's own polarity: the only thing that
        // tells the extractor NMOS from PMOS.
        let chan_imp = process.layer(if is_pmos { "psdm" } else { "nsdm" });
        if let Some(chan_imp) = chan_imp {
            b.rect(chan_imp, Rect {
                x: diff_x_start - imp_enc,
                y: -imp_enc,
                w: (diff_x_end - diff_x_start) + 2 * imp_enc,
                h: finger_w + 2 * imp_enc,
            });
        }
        // Tap cuts, skipping columns under a riser strip.
        let guard_pitch = r("guard_licon_pitch", 340);
        let under_riser = |lx: i32| {
            edge_cuts.iter().flatten().any(|&(x0, x1)| lx <= x1 + ct + rise_r && x0 - rise_l <= lx + ct)
        };
        let mut kept: Option<(i32, i32)> = None;
        let mut lx = tap_x0 + diff_enc;
        while lx + ct + diff_enc <= tap_x1 {
            if !under_riser(lx) {
                b.rect(licon, Rect { x: lx, y: tap_y0 + diff_enc, w: ct, h: ct });
                kept.get_or_insert((lx, lx)).1 = lx;
            }
            lx += guard_pitch;
        }
        // One solid li rail over the cut row, spanning the strip so it merges
        // with the risers.
        let (first_lx, last_lx) = kept.unwrap_or((tap_x0 + diff_enc, tap_x0 + diff_enc));
        let rail_x0 = (first_lx - li_enc).min(tap_x0);
        b.rect(li, Rect {
            x: rail_x0,
            y: rail_li_y,
            w: (last_lx + ct + li_side).max(tap_x1) - rail_x0,
            h: ct + li_enc + li_side,
        });
        // One bulk pin per device on the rail.
        for di in 0..n_dev {
            let xi = (first_lx + di as i32 * guard_pitch).min(last_lx);
            b.pin(pin(di, "B", Rect { x: xi, y: tap_y0 + diff_enc, w: ct, h: ct }, li));
        }

        // PMOS nwell, inflated by the WPE halo on matched groups.
        if is_pmos {
            if let Some(nwell) = process.layer("nwell") {
                let wpe_halo = if matched { r("wpe_clearance_moderate", 3000) } else { 0 };
                let nw_enc = r("nwell_diff_enc", 180) + wpe_halo;
                let nw_min = r("nwell_min_width", 840);
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

        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, process.rule("min_finger_width", 420), process.rule("min_gate_l", 150))
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
}
