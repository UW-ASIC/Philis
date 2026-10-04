//! Diode generator: n+ diffusion (cathode `N`) in the p-substrate, with a
//! p+ substrate tap beside it as the anode contact (`P`), under `diom`;
//! devices arrayed in `columns`. A deck whose diode marker is `diode_mk`
//! (gf180 `pn_3p3`: the p+ comp in an nwell) gets the well form instead: p+
//! anode `P` under the marker, an n+ tap as the nwell's cathode contact `N`,
//! both in one nwell per device.

use crate::builder::dim;
use analog::matching::pattern::{self, Fill};
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, Macro, Node, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, Builder, Sizing};
use crate::Cell;

/// ponytail: the gf180 well form is drawn but unusable. Its recogniser's
/// `nwell` terminal is not a conductor in the deck, so extraction finds 0
/// devices (gpurify: terminals must be conductors; wells-as-nets is coming
/// upstream), and the li pad fails M1.3 until verify exposes `li_min_area`.
///
/// One diode variant: `rows × cols` grid; units of member `d` are
/// `dev_nf[d]`, owners from [`pattern::centro_assign`]`(Balanced)`.
#[derive(Clone)]
pub struct Diode {
    pub rows: u16,
    pub cols: u16,
}

impl Cell for Diode {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let counts: Vec<u16> = s.dev_nf.iter().map(|&n| n.max(1)).collect();
        let n: u16 = counts.iter().sum();
        if counts.len() > 1 {
            return pattern::grids(&counts, 3.0).into_iter().map(|(r, c)| Diode { rows: r as u16, cols: c as u16 }).collect();
        }
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        cols.into_iter().map(|cols| Diode { rows: n.div_ceil(cols), cols }).collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let counts: Vec<u16> = s.dev_nf.iter().map(|&n| n.max(1)).collect();
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
        // Anode tap: a licon.7-legal pad, a cut end-cap on both sides.
        let tap_cap = r("tap_encloses_licon_one_side", 0).max(process.endcap("tap", "licon").unwrap_or(0));
        let tap_side = ct + 2 * tap_cap.max(diff_enc);
        // Tall enough to pass the cut by the end-cap above and below (the
        // deck's end-cap holds on both ends of one axis).
        let tap_h = l.max(tap_side).max(ct + 2 * tap_cap);
        // Opposite implants may not overlap; the tap keeps its diff spacing.
        let g = up((2 * imp).max(process.space_between("tap", "diff").unwrap_or(0)).max(process.space("diff").unwrap_or(0)));
        // Between devices: same-type implants keep their spacing.
        let gap = up(r("diode_gap", 0).max(2 * imp + process.space("nsdm").unwrap_or(0).max(process.space("psdm").unwrap_or(0))));
        let dev_w = w + g + tap_side;
        let rows = usize::from(self.rows);
        let cols = usize::from(self.cols);
        let diom = process.layer("diom");
        // The well form swaps which pad is which terminal and what dopes it.
        let well = process.layer("diode_mk").map(|mk| (mk, req(process, "nwell"), up(dim(process, "nwell_diff_enc"))));
        let (k_imp, a_imp, k_term, a_term) = if well.is_some() { (psdm, nsdm, "P", "N") } else { (nsdm, psdm, "N", "P") };
        // Neighbouring devices' wells keep their spacing.
        let gap = well.map_or(gap, |(_, _, enc)| gap.max(2 * enc + r("nwell_min_spacing", 0).max(process.space("nwell").unwrap_or(0))));
        // The li pad also clears the li min area (the deck states its side).
        let pad_enc = li_enc.max(up((r("li_min_area", 0) - ct + 1) / 2));
        // Contact array pitch and the cathode's cut counts (bjt.rs:166-178).
        let pitch = up(ct + process.space("licon").unwrap_or(ct));
        let fit = |len: i32, inset: i32| ((len - 2 * inset - ct) / pitch + 1).max(1);
        let (nx, ny) = (fit(w, diff_cap), fit(l, diff_cap));
        // The anode's column of cuts, centred in x (tap_side is symmetric).
        let ny_tap = fit(tap_h, tap_cap);
        let ax_cut = tap_side / 2 - ct / 2;

        let (owners, _) = pattern::centro_assign(&counts, rows, cols, Fill::Balanced);
        for (slot, d) in owners.iter().enumerate().filter_map(|(i, d)| Some((i, (*d)?))) {
            let di = usize::from(d);
            let ox = (slot % cols) as i32 * (dev_w + gap);
            let oy = (slot / cols) as i32 * (tap_h + gap);
            let (kx, ax) = (ox, ox + w + g);

            let k = Rect { x: kx, y: oy, w, h: l };
            b.rect(diff, k);
            b.rect(k_imp, Rect { x: k.x - imp, y: k.y - imp, w: w + 2 * imp, h: l + 2 * imp });
            let (x0, y0) = (snap_cut(kx + (w - (nx - 1) * pitch - ct) / 2, lat), snap_cut(oy + (l - (ny - 1) * pitch - ct) / 2, lat));
            for i in 0..nx {
                for j in 0..ny {
                    b.rect(licon, Rect { x: x0 + i * pitch, y: y0 + j * pitch, w: ct, h: ct });
                }
            }
            let plate = Rect { x: x0 - pad_enc, y: y0 - pad_enc, w: (nx - 1) * pitch + ct + 2 * pad_enc, h: (ny - 1) * pitch + ct + 2 * pad_enc };
            b.rect(li, plate);
            b.pin(pin(di, k_term, Rect { x: x0 + (nx / 2) * pitch, y: y0 + (ny / 2) * pitch, w: ct, h: ct }, li));

            let a = Rect { x: ax, y: oy, w: tap_side, h: tap_h };
            b.rect(tap, a);
            b.rect(a_imp, Rect { x: a.x - imp, y: a.y - imp, w: a.w + 2 * imp, h: a.h + 2 * imp });
            let ay0 = snap_cut(oy + (tap_h - (ny_tap - 1) * pitch - ct) / 2, lat);
            for j in 0..ny_tap {
                b.rect(licon, Rect { x: ax + ax_cut, y: ay0 + j * pitch, w: ct, h: ct });
            }
            b.rect(
                li,
                Rect { x: ax + ax_cut - pad_enc, y: ay0 - pad_enc, w: ct + 2 * pad_enc, h: (ny_tap - 1) * pitch + ct + 2 * pad_enc },
            );
            b.pin(pin(di, a_term, Rect { x: ax + ax_cut, y: ay0 + (ny_tap / 2) * pitch, w: ct, h: ct }, li));

            if let Some((mk, nwell, enc)) = well {
                // Marker on the anode alone: the tap's comp under it would be
                // a second anode.
                b.rect(mk, k);
                b.rect(nwell, Rect { x: ox - enc, y: oy - enc, w: dev_w + 2 * enc, h: tap_h + 2 * enc });
            } else if let Some(diom) = diom {
                // LVS marker: the recogniser binds the two li pads under it.
                b.rect(diom, Rect { x: ox, y: oy, w: dev_w, h: tap_h });
            }

            b.unit(pnr_core::Unit { owner: d, x: k.x + w / 2, y: k.y + l / 2, weight: i64::from(w) * i64::from(l), phi: (0, 0), sa: 0, sb: 0 });
            b.drawn(Drawn { owner: d, device: None, kind: DrawnKind::Diode, nodes: [Node::Pin("P"), Node::Pin("N"), Node::Unused], w, l });
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
        for counts in [vec![4u16], vec![2, 2]] {
            let (g, mut c) = testkit::group_of(DeviceKind::Diode, counts.len(), 1, 500, 1000);
            c.unitization[0].dev_nf = counts.clone();
            dirty.extend(testkit::dirty_group::<Diode>(&g, &c, &pdk).into_iter().map(|d| format!("{counts:?} {d}")));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    /// A member's `dev_nf` units each draw a cathode and anode, with the
    /// member's total diff (cathode) area unchanged since 500×1000 are
    /// lattice multiples (no snap).
    #[test]
    fn a_multiplied_diode_draws_its_area() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let diff = pdk.layer("diff").unwrap();
        let (g, c) = testkit::group_of(DeviceKind::Diode, 1, 4, 500, 1000);
        for v in Diode::enumerate(&g, &c, &pdk) {
            let m = v.draw(&g, &c, &pdk);
            let at = format!("{}x{}", v.rows, v.cols);
            assert_eq!(m.units.len(), 4, "{at}");
            assert_eq!(m.drawn.len(), 4, "{at}");
            let area: i64 = m.shapes.iter().filter(|s| s.layer == diff).map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum();
            assert_eq!(area, 4 * 500_000, "{at}");
        }
    }

    /// Every anode tap pad is at least the licon.7-legal width (410 nm on
    /// sky130).
    #[test]
    fn the_anode_tap_meets_licon_7() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let tap = pdk.layer("tap").unwrap();
        let (g, c) = testkit::group_of(DeviceKind::Diode, 2, 1, 500, 1000);
        for v in Diode::enumerate(&g, &c, &pdk) {
            let m = v.draw(&g, &c, &pdk);
            for s in m.shapes.iter().filter(|s| s.layer == tap) {
                assert!(s.rect.w >= 410, "{}x{}: tap w={}", v.rows, v.cols, s.rect.w);
            }
        }
    }

    /// A matched bank's members share a centroid: `n_B·Σ_{u∈A} u.x ==
    /// n_A·Σ_{u∈B} u.x` (and y), exactly (i64, equal-area units).
    #[test]
    fn matched_diodes_share_a_centroid() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        for counts in [[2u16, 2], [1, 4]] {
            let (g, mut c) = testkit::group_of(DeviceKind::Diode, 2, 1, 500, 1000);
            c.unitization[0].dev_nf = counts.to_vec();
            for v in Diode::enumerate(&g, &c, &pdk) {
                let m = v.draw(&g, &c, &pdk);
                let at = format!("{counts:?} on {}x{}", v.rows, v.cols);
                let sum = |owner: u8| {
                    m.units.iter().filter(|u| u.owner == owner).fold((0i64, 0i64), |(sx, sy), u| (sx + i64::from(u.x), sy + i64::from(u.y)))
                };
                let (n_a, n_b) = (i64::from(counts[0]), i64::from(counts[1]));
                let (sxa, sya) = sum(0);
                let (sxb, syb) = sum(1);
                assert_eq!(n_b * sxa, n_a * sxb, "{at}");
                assert_eq!(n_b * sya, n_a * syb, "{at}");
            }
        }
    }
}
