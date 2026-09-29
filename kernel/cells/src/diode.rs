//! Diode generator: n+ diffusion (cathode `N`) in the p-substrate, with a
//! p+ substrate tap beside it as the anode contact (`P`), under `diom`;
//! devices arrayed in `columns`. A deck whose diode marker is `diode_mk`
//! (gf180 `pn_3p3`: the p+ comp in an nwell) gets the well form instead: p+
//! anode `P` under the marker, an n+ tap as the nwell's cathode contact `N`,
//! both in one nwell per device.

use crate::builder::dim;
use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, Builder, Sizing};
use crate::{Cell, Pattern};

/// ponytail: the gf180 well form is drawn but unusable. Its recogniser's
/// `nwell` terminal is not a conductor in the deck, so extraction finds 0
/// devices (gpurify: terminals must be conductors; wells-as-nets is coming
/// upstream), and the li pad fails M1.3 until verify exposes `li_min_area`.
///
/// One diode variant: array aspect `columns`; `Interdig` flips every other
/// device (anode tap on the left) for matched banks.
#[derive(Clone)]
pub struct Diode {
    pub pattern: Pattern,
    pub columns: u16,
}

impl Cell for Diode {
    fn enumerate(group: &DeviceGroup, _constraints: &Constraints, _process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let n = group.devices.len() as u16;
        let patterns: &[Pattern] =
            if n > 1 { &[Pattern::Single, Pattern::Interdig] } else { &[Pattern::Single] };
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        patterns
            .iter()
            .flat_map(|&pattern| cols.iter().map(move |&columns| Diode { pattern, columns }))
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let r = |name: &str, default: i32| process.rule(name, default);
        let lat = cut_lattice(process);
        let up = |v: i32| snap_cut(v + lat - 1, lat);

        let (diff, tap, li, licon) = (req(process, "diff"), req(process, "tap"), req(process, "li"), req(process, "licon"));
        let (nsdm, psdm) = (req(process, "nsdm"), req(process, "psdm"));
        let ct = dim(process, "contact");
        let diff_enc = r("diff_encloses_licon", 0).max(process.enclosure("diff", "licon").unwrap_or(0));
        let li_enc = r("li_encloses_licon", 0).max(process.enclosure("li", "licon").unwrap_or(0)).max(process.endcap("li", "licon").unwrap_or(0));
        // Implant past the diffusion it dopes, the deck's.
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let imp = up(enc("nsdm", "diff").max(enc("psdm", "diff")).max(enc("nsdm", "tap")).max(enc("psdm", "tap")));
        // Cathode diff: room for a cut with the diff end-cap on both ends.
        let diff_cap = diff_enc.max(process.endcap("diff", "licon").unwrap_or(0));
        let (w, l) = (up(s.unit_w.max(ct + 2 * diff_cap)), up(s.unit_l.max(ct + 2 * diff_cap)));
        // Anode tap: a cut with `diff_enc` on one side, the tap one-side
        // enclosure on the other, in both axes.
        let tap_cap = r("tap_encloses_licon_one_side", 0).max(process.endcap("tap", "licon").unwrap_or(0));
        let tap_side = ct + diff_enc + tap_cap;
        // Tall enough to pass the cut by the end-cap above and below (the
        // deck's end-cap holds on both ends of one axis).
        let tap_h = l.max(tap_side).max(ct + 2 * tap_cap);
        // Opposite implants may not overlap; the tap keeps its diff spacing.
        let g = up((2 * imp).max(process.space_between("tap", "diff").unwrap_or(0)).max(process.space("diff").unwrap_or(0)));
        // Between devices: same-type implants keep their spacing.
        let gap = up(r("diode_gap", 0).max(2 * imp + process.space("nsdm").unwrap_or(0).max(process.space("psdm").unwrap_or(0))));
        let dev_w = w + g + tap_side;
        let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
        let diom = process.layer("diom");
        // The well form swaps which pad is which terminal and what dopes it.
        let well = process.layer("diode_mk").map(|mk| (mk, req(process, "nwell"), up(dim(process, "nwell_diff_enc"))));
        let (k_imp, a_imp, k_term, a_term) = if well.is_some() { (psdm, nsdm, "P", "N") } else { (nsdm, psdm, "N", "P") };
        // Neighbouring devices' wells keep their spacing.
        let gap = well.map_or(gap, |(_, _, enc)| gap.max(2 * enc + r("nwell_min_spacing", 0).max(process.space("nwell").unwrap_or(0))));
        // The li pad also clears the li min area (the deck states its side).
        let pad_enc = li_enc.max(up((r("li_min_area", 0) - ct + 1) / 2));
        let contact = |b: &mut Builder, cut: Rect| {
            b.rect(licon, cut);
            b.rect(li, Rect { x: cut.x - pad_enc, y: cut.y - pad_enc, w: cut.w + 2 * pad_enc, h: cut.h + 2 * pad_enc });
        };
        for di in 0..n_dev {
            let ox = (di as i32 % cols) * (dev_w + gap);
            let oy = (di as i32 / cols) * (tap_h + gap);
            // Interdig mirrors every other device: anode tap on the left.
            let flip = self.pattern == Pattern::Interdig && di % 2 == 1;
            let (kx, ax) = if flip { (ox + tap_side + g, ox) } else { (ox, ox + w + g) };

            let k = Rect { x: kx, y: oy, w, h: l };
            b.rect(diff, k);
            b.rect(k_imp, Rect { x: k.x - imp, y: k.y - imp, w: w + 2 * imp, h: l + 2 * imp });
            let at = Rect { x: snap_cut(kx + w / 2 - ct / 2, lat), y: snap_cut(oy + l / 2 - ct / 2, lat), w: ct, h: ct };
            contact(&mut b, at);
            b.pin(pin(di, k_term, at, li));

            let a = Rect { x: ax, y: oy, w: tap_side, h: tap_h };
            b.rect(tap, a);
            b.rect(a_imp, Rect { x: a.x - imp, y: a.y - imp, w: a.w + 2 * imp, h: a.h + 2 * imp });
            let at = Rect { x: ax + if flip { tap_side - diff_enc - ct } else { diff_enc }, y: snap_cut(oy + (tap_h - ct) / 2, cut_lattice(process)), w: ct, h: ct };
            contact(&mut b, at);
            b.pin(pin(di, a_term, at, li));

            if let Some((mk, nwell, enc)) = well {
                // Marker on the anode alone: the tap's comp under it would be
                // a second anode.
                b.rect(mk, k);
                b.rect(nwell, Rect { x: ox - enc, y: oy - enc, w: dev_w + 2 * enc, h: tap_h + 2 * enc });
            } else if let Some(diom) = diom {
                // LVS marker: the recogniser binds the two li pads under it.
                b.rect(diom, Rect { x: ox, y: oy, w: dev_w, h: tap_h });
            }
        }

        b.cover_poly_cuts(process);
        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, process.rule("diode_w", 0), process.rule("diode_l", 0))
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
            dirty.extend(testkit::dirty::<Diode>(DeviceKind::Diode, n, 1, 500, 1000, &pdk));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }
}
