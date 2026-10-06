//! Diode generator: n+ diffusion (cathode `N`) in the p-substrate, with a
//! p+ substrate tap beside it as the anode contact (`P`), under `diom`;
//! devices arrayed in `columns`. A deck whose diode marker is `diode_mk`
//! (gf180 `pn_3p3`: the p+ comp in an nwell) gets the well form instead: p+
//! anode `P` under the marker, an n+ tap as the nwell's cathode contact `N`,
//! both in one nwell per device.

use analog::matching::pattern::{self, Fill};
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, Macro, Node, Process, Rect};

use crate::builder::{cut_lattice, dim, pin, req, sizing, snap_cut, Builder, Sizing};
use crate::{unit_grids, Cell};

// ponytail: the gf180 well form is drawn but unusable. Its recogniser's
// `nwell` terminal is not a conductor in the deck, so extraction finds 0
// devices (gpurify: terminals must be conductors; wells-as-nets is coming
// upstream), and the li pad fails M1.3 until verify exposes `li_min_area`.

/// One diode variant: a `rows × cols` grid of units, each a cathode pad and
/// an anode pad side by side; member `d` owns `dev_nf[d]` units, dealt by
/// [`pattern::centro_assign`]`(Balanced)`. Empty cells stay empty.
///
/// Invariant (from [`Cell::enumerate`]): `rows · cols` is at least the
/// group's total unit count.
#[derive(Clone)]
pub struct Diode {
    /// Unit rows, bottom to top.
    pub rows: u16,
    /// Unit columns, left to right.
    pub cols: u16,
}

impl Cell for Diode {
    /// Empty for an empty group.
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        unit_grids(&s.dev_nf).into_iter().map(|(rows, cols)| Diode { rows, cols }).collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
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
        // Contact array pitch and the cathode's cut counts (as the BJT emitter's).
        // At least a lattice step: a deck stating no cut size still draws.
        let pitch = up(ct + process.space("licon").unwrap_or(ct)).max(lat);
        let fit = |len: i32, inset: i32| ((len - 2 * inset - ct) / pitch + 1).max(1);
        let (nx, ny) = (fit(w, diff_cap), fit(l, diff_cap));
        // The anode's column of cuts, centred in x (tap_side is symmetric).
        let ny_tap = fit(tap_h, tap_cap);
        let ax_cut = tap_side / 2 - ct / 2;

        let (owners, _) = pattern::centro_assign(&s.dev_nf, rows, cols, Fill::Balanced);
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

            b.unit(pnr_core::Unit { owner: d, x: k.x + w / 2, y: k.y + l / 2, weight: i64::from(w) * i64::from(l), phi: (0, 0), sa_sb: None });
            b.drawn(Drawn { owner: d, device: None, kind: DrawnKind::Diode, nodes: [Node::Pin("P"), Node::Pin("N"), Node::Unused], w, l });
        }

        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// The group's sizing, a unit defaulting to `diode_w` × `diode_l`.
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

/// Corner cases for the diode generator on a hand-built deck (cleanup step
/// 2). Oracles: the doc comments and positions derived by hand.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::builder::fake::Deck;
    use crate::testkit::group_of;
    use pnr_core::DeviceKind;

    /// Cuts 170 on a 340 pitch, the tap 270 from the cathode, devices 400
    /// apart; no implant or contact enclosure. A 500×1000 unit then draws
    /// its cathode at (0, 0, 500, 1000), its 170-wide anode tap at x = 770,
    /// and one device spans 940.
    fn deck() -> Deck {
        Deck::new(1, &["diff", "tap", "li", "licon", "nsdm", "psdm", "diom"])
            .with("w:licon", 170)
            .with("s:licon", 170)
            .with("s:diff", 270)
            .with("diode_gap", 400)
    }

    /// `deck` with the gf180 well form: a `diode_mk` marker and an n-well
    /// 100 past the diffusion.
    fn well_deck() -> Deck {
        let mut d = deck().with("enc:nwell:diff", 100);
        d.roles.push(("diode_mk", 50));
        d.roles.push(("nwell", 51));
        d
    }

    fn rects(m: &Macro, d: &Deck, role: &str) -> Vec<Rect> {
        let l = d.layer(role).unwrap();
        m.shapes.iter().filter(|s| s.layer == l).map(|s| s.rect).collect()
    }

    fn inside(outer: Rect, inner: Rect) -> bool {
        outer.x <= inner.x && outer.y <= inner.y && outer.x + outer.w >= inner.x + inner.w && outer.y + outer.h >= inner.y + inner.h
    }

    #[test]
    fn an_empty_group_has_no_variants() {
        let g = DeviceGroup { devices: vec![] };
        assert!(Diode::enumerate(&g, &Constraints::default(), &deck()).is_empty());
    }

    #[test]
    fn a_lone_member_offers_row_column_and_square() {
        let (g, c) = group_of(DeviceKind::Diode, 1, 4, 500, 1000);
        let v: Vec<(u16, u16)> = Diode::enumerate(&g, &c, &deck()).iter().map(|d| (d.rows, d.cols)).collect();
        assert_eq!(v, [(4, 1), (2, 2), (1, 4)]);
    }

    #[test]
    fn the_sizing_defaults_to_the_deck_s_diode() {
        let d = deck().with("diode_w", 700).with("diode_l", 900);
        let s = group_sizing(&DeviceGroup { devices: vec![pnr_core::DeviceId(0)] }, &Constraints::default(), &d);
        assert_eq!((s.unit_w, s.unit_l, s.dev_nf), (700, 900, vec![1]));
    }

    #[test]
    fn a_unit_draws_cathode_anode_and_marker_where_derived() {
        let d = deck();
        let (g, c) = group_of(DeviceKind::Diode, 1, 1, 500, 1000);
        let m = Diode { rows: 1, cols: 1 }.draw(&g, &c, &d);
        let k = Rect { x: 0, y: 0, w: 500, h: 1000 };
        let a = Rect { x: 770, y: 0, w: 170, h: 1000 };
        assert_eq!(rects(&m, &d, "diff"), [k]);
        assert_eq!(rects(&m, &d, "tap"), [a]);
        assert_eq!(rects(&m, &d, "diom"), [Rect { x: 0, y: 0, w: 940, h: 1000 }]);
        // One column of three cuts in each pad.
        assert_eq!(rects(&m, &d, "licon").len(), 6);
        let pin = |n: &str| m.pins.iter().find(|p| p.name == n).unwrap_or_else(|| panic!("{n}")).at;
        assert!(inside(k, pin("d0:N")));
        assert!(inside(a, pin("d0:P")));
        assert_eq!(m.units.len(), 1);
        assert_eq!(m.units[0].weight, 500 * 1000);
        assert_eq!((m.units[0].x, m.units[0].y), (250, 500));
        assert_eq!(m.drawn.len(), 1);
        assert_eq!(m.drawn[0].kind, DrawnKind::Diode);
        assert_eq!(m.drawn[0].nodes, [Node::Pin("P"), Node::Pin("N"), Node::Unused]);
        assert_eq!((m.drawn[0].w, m.drawn[0].l), (500, 1000));
    }

    #[test]
    fn units_step_one_device_plus_the_gap() {
        let d = deck();
        let (g, c) = group_of(DeviceKind::Diode, 1, 2, 500, 1000);
        let m = Diode { rows: 1, cols: 2 }.draw(&g, &c, &d);
        let xs: Vec<i32> = rects(&m, &d, "diff").iter().map(|r| r.x).collect();
        assert_eq!(xs, [0, 940 + 400]);
        let m = Diode { rows: 2, cols: 1 }.draw(&g, &c, &d);
        let ys: Vec<i32> = rects(&m, &d, "diff").iter().map(|r| r.y).collect();
        assert_eq!(ys, [0, 1000 + 400]);
    }

    /// The well form swaps the terminals: the diff pad is the p+ anode
    /// under `diode_mk`, the tap the n-well's cathode contact, both in one
    /// n-well; no `diom`.
    #[test]
    fn the_well_form_swaps_terminals_and_wraps_a_well() {
        let d = well_deck();
        let (g, c) = group_of(DeviceKind::Diode, 1, 1, 500, 1000);
        let m = Diode { rows: 1, cols: 1 }.draw(&g, &c, &d);
        let k = Rect { x: 0, y: 0, w: 500, h: 1000 };
        let a = Rect { x: 770, y: 0, w: 170, h: 1000 };
        assert_eq!(rects(&m, &d, "diode_mk"), [k]);
        assert_eq!(rects(&m, &d, "nwell"), [Rect { x: -100, y: -100, w: 1140, h: 1200 }]);
        assert!(rects(&m, &d, "diom").is_empty());
        let pin = |n: &str| m.pins.iter().find(|p| p.name == n).unwrap_or_else(|| panic!("{n}")).at;
        assert!(inside(k, pin("d0:P")));
        assert!(inside(a, pin("d0:N")));
    }

    #[test]
    fn matched_members_share_a_centroid_on_the_fake_deck() {
        let d = deck();
        let (g, mut c) = group_of(DeviceKind::Diode, 2, 1, 500, 1000);
        c.unitization[0].dev_nf = vec![2, 2];
        for v in Diode::enumerate(&g, &c, &d) {
            let m = v.draw(&g, &c, &d);
            let sum = |o: u8| m.units.iter().filter(|u| u.owner == o).fold((0i64, 0i64), |(x, y), u| (x + i64::from(u.x), y + i64::from(u.y)));
            assert_eq!(sum(0), sum(1), "{}x{}", v.rows, v.cols);
            assert_eq!(m.units.len(), 4);
        }
    }

    /// A deck that states no contact size or spacing still draws.
    #[test]
    fn a_deck_without_contact_rules_draws() {
        let d = Deck::new(1, &["diff", "tap", "li", "licon", "nsdm", "psdm"]);
        let (g, c) = group_of(DeviceKind::Diode, 1, 1, 500, 1000);
        let m = Diode { rows: 1, cols: 1 }.draw(&g, &c, &d);
        assert_eq!(m.drawn.len(), 1);
    }
}
