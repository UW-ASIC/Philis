//! BJT generator: the vertical bipolar of a CMOS process, concentric per
//! unit. PNP: a p+ emitter, an n-tap base ring whose n-well is the base, and
//! a p-tap collector ring on the substrate (the collector), under the `pnp`
//! marker. NPN: an n+ emitter, a p-tap base ring, both in the p-region an
//! n-well ring and a deep n-well isolate, the collector tapped on that ring.
//! Every gap and width is the deck's.

use analog::matching::pattern::{self, Fill};
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, NetId, Pin, Process, Rect};

use crate::builder::{cut_lattice, dim, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
use crate::post_cell::{tap_ring, WellShape};
use crate::{unit_grids, Cell};

/// One BJT variant: the group's unit devices (each member's `dev_nf` units)
/// on a `rows × columns` grid, owners from
/// [`pattern::centro_assign`]`(Balanced)`: every member point-symmetric about
/// the grid centre when at most one count is odd, so a 1:8 pair is the
/// classic 3×3 with the one unit centred (Hastings §10.2, ratioed bipolars).
/// Empty cells stay empty. Area ratio comes from unit count, never emitter
/// scaling. Neighbouring units share their collector band.
///
/// Invariant (from [`Cell::enumerate`]): `rows · columns` is at least the
/// group's total unit count.
#[derive(Clone)]
pub struct Bjt {
    /// Unit rows, bottom to top.
    pub rows: u16,
    /// Unit columns, left to right.
    pub columns: u16,
}

impl Cell for Bjt {
    /// Empty for an empty group, and for an NPN where the process lacks the
    /// isolated p-base (a `dnwell` layer and a non-zero `npn_isolation`).
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        if !device_is_pnp(group, constraints) && (process.layer("dnwell").is_none() || process.rule("npn_isolation", 0) == 0) {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        unit_grids(&s.dev_nf).into_iter().map(|(rows, columns)| Bjt { rows, columns }).collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let pnp = device_is_pnp(group, constraints);
        let u = Unit::new(&s, pnp, process);
        let cols = i32::from(self.columns);
        let (owners, _) = pattern::centro_assign(&s.dev_nf, usize::from(self.rows), usize::from(self.columns), Fill::Balanced);
        // PNP units abut on a shared collector band (the substrate); NPN
        // units each keep their own isolation, the deck's spacings apart.
        let lat = cut_lattice(process);
        let (px, py) = (snap_cut(u.pitch.0 + lat - 1, lat), snap_cut(u.pitch.1 + lat - 1, lat));
        for (slot, di) in owners.iter().enumerate().filter_map(|(i, d)| Some((i, (*d)?))) {
            let (ox, oy) = ((slot as i32 % cols) * px, (slot as i32 / cols) * py);
            u.draw(&mut b, process, usize::from(di), ox, oy);
        }
        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// An NPN's isolation, unit frame: an n-well ring (`hole` inside `outer`)
/// over a deep n-well `dnwell`.
#[derive(Clone, Copy)]
struct Isolation {
    hole: Rect,
    outer: Rect,
    dnwell: Rect,
}

/// One unit's geometry with its emitter's lower-left corner at the origin,
/// nm. All derived from the deck once per draw.
struct Unit {
    pnp: bool,
    emitter: Rect,
    /// Emitter edge to the base band's inner edge.
    base_gap: i32,
    /// The base band's outer edge.
    base_outer: Rect,
    /// Base band's outer edge to the collector band's inner edge.
    coll_gap: i32,
    /// Width of the base and collector bands.
    ring_w: i32,
    /// `Some` exactly for an NPN.
    iso: Option<Isolation>,
    /// Unit-to-unit step `(x, y)` before lattice snapping.
    pitch: (i32, i32),
}

/// Implant roles `(emitter, base, collector)`: PNP p+/n+/p+, NPN n+/p+/n+.
fn implants(pnp: bool) -> (&'static str, &'static str, &'static str) {
    if pnp {
        ("psdm", "nsdm", "psdm")
    } else {
        ("nsdm", "psdm", "nsdm")
    }
}

/// `r` grown by `d` on every side.
fn grow(r: Rect, d: i32) -> Rect {
    Rect { x: r.x - d, y: r.y - d, w: r.w + 2 * d, h: r.h + 2 * d }
}

impl Unit {
    fn new(s: &Sizing, pnp: bool, process: &dyn Process) -> Self {
        let r = |name: &str, d: i32| process.rule(name, d);
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let (emit_imp, base_imp, coll_imp) = implants(pnp);
        let min_side = r("bjt_min_emitter_side", 0);
        // A fixed-geometry model's recipe slot overrides the netlist's size
        // (sky130 BJTs are fixed devices: any other drawn size simulates a
        // different one).
        let ew = match r("bjt_emitter_w", 0) {
            0 => s.unit_w.max(min_side),
            v => v,
        };
        let el = match r("bjt_emitter_l", 0) {
            0 => s.unit_l.max(min_side),
            v => v,
        };
        let emitter = Rect { x: 0, y: 0, w: ew, h: el };
        let ct = dim(process, "contact");
        let ring_w = r("min_guard_ring_width", 0).max(ct + 2 * enc("tap", "licon")).max(ct + 2 * r("diff_encloses_licon", 0));
        let clear = ["psdm", "nsdm", "tap", "diff"].iter().filter_map(|x| process.space(x)).max().unwrap_or(0);
        // Emitter to base band: diffusion clearance, and the two implants
        // meeting at most edge to edge.
        // An implant past its diffusion keeps the deck's implant-to-opposite
        // diffusion and contact spacings from the next band.
        let beyond = |imp: &str| process.space_between(imp, "tap").unwrap_or(0).max(process.space_between(imp, "licon").unwrap_or(0));
        let base_gap = clear
            .max(process.space_between("tap", "diff").unwrap_or(0))
            .max(enc(emit_imp, "diff") + enc(base_imp, "tap"))
            .max(enc(emit_imp, "diff") + beyond(emit_imp))
            .max(enc(base_imp, "tap") + process.space_between(base_imp, "diff").unwrap_or(0));
        let base_outer = grow(emitter, base_gap + ring_w);
        // Base band to collector band: the base implant against the
        // collector's, and the well (PNP: the base n-well; NPN: the isolating
        // n-well ring, drawn past the collector band) to the tap.
        let nw = dim(process, "nwell_diff_enc");
        // Neighbouring units' base wells keep the well spacing across the
        // shared collector band.
        let well_gap = nw + (r("nwell_min_spacing", 0).max(process.space("nwell").unwrap_or(0)) - ring_w + 1) / 2;
        let coll_gap = clear
            .max(enc(base_imp, "tap") + enc(coll_imp, "tap"))
            .max(enc(coll_imp, "tap") + beyond(coll_imp))
            .max(enc(base_imp, "tap") + beyond(base_imp))
            .max(nw + process.space_between("nwell", "tap").unwrap_or(0))
            .max(if pnp { well_gap } else { 0 });
        // NPN: the n-well ring's hole a well-to-p-tap spacing past the base
        // band, the deep well the deck's enclosure past the hole, the ring
        // past the deep well and around the collector band.
        let sp_nt = process.space_between("nwell", "tap").unwrap_or(0);
        let coll_gap = if pnp { coll_gap } else { coll_gap.max(sp_nt + nw) };
        let outer = grow(base_outer, coll_gap + ring_w);
        let iso = (!pnp).then(|| {
            let hole = grow(base_outer, sp_nt);
            let dn = grow(hole, process.enclosure("dnwell", "nwell").unwrap_or(0));
            let past = process.enclosure("nwell", "dnwell").unwrap_or(0);
            let reach = (dn.x - hole.x).abs() + past;
            let full = grow(hole, reach.max(hole.x - outer.x + nw));
            Isolation { hole, outer: full, dnwell: dn }
        });
        let pitch = match iso {
            None => (outer.w - ring_w, outer.h - ring_w),
            Some(Isolation { outer: full, dnwell: dn, .. }) => {
                let (sw, sd) = (process.space("nwell").unwrap_or(0), process.space("dnwell").unwrap_or(0));
                ((full.w + sw).max(dn.w + sd), (full.h + sw).max(dn.h + sd))
            }
        };
        Self { pnp, emitter, base_gap, base_outer, coll_gap, ring_w, iso, pitch }
    }

    /// Draws member `di`'s unit with its emitter corner at `(ox, oy)`.
    fn draw(&self, b: &mut Builder, process: &dyn Process, di: usize, ox: i32, oy: i32) {
        let at = |x: Rect| Rect { x: x.x + ox, y: x.y + oy, ..x };
        let r = |name: &str, d: i32| process.rule(name, d);
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let cap = |o: &str, i: &str| process.endcap(o, i).unwrap_or(0);
        let (emit_imp, base_imp, coll_imp) = implants(self.pnp);
        let (diff, li, licon) = (req(process, "diff"), req(process, "li"), req(process, "licon"));
        let lat = cut_lattice(process);
        let ct = dim(process, "contact");
        let e = at(self.emitter);
        b.unit(pnr_core::Unit { owner: di as u8, x: e.x + e.w / 2, y: e.y + e.h / 2, weight: i64::from(e.w) * i64::from(e.h), phi: (0, 0), sa_sb: None });
        // Pin order matches `cellgen::BJT_PINS`.
        b.drawn(pnr_core::Drawn {
            owner: di as u8,
            device: None,
            kind: if self.pnp { pnr_core::DrawnKind::Pnp } else { pnr_core::DrawnKind::Npn },
            nodes: [pnr_core::Node::Pin("C"), pnr_core::Node::Pin("B"), pnr_core::Node::Pin("E")],
            w: self.emitter.w,
            l: self.emitter.h,
        });

        // Emitter: diffusion, its implant, a contact array under one li
        // plate (cuts `max(enclosure, end-cap)` inside the diffusion, li the
        // end-cap past the outer cuts on every side).
        b.rect(diff, e);
        let ei = enc(emit_imp, "diff");
        b.rect(req(process, emit_imp), grow(e, ei));
        let inset = r("diff_encloses_licon", 0).max(enc("diff", "licon")).max(cap("diff", "licon"));
        // At least a lattice step: a deck stating no cut size still draws.
        let pitch = (ct + process.space("licon").unwrap_or(ct)).max(lat);
        let fit = |len: i32| ((len - 2 * inset - ct) / pitch + 1).max(1);
        let (nx, ny) = (fit(e.w), fit(e.h));
        let (x0, y0) = (e.x + (e.w - (nx - 1) * pitch - ct) / 2, e.y + (e.h - (ny - 1) * pitch - ct) / 2);
        let (x0, y0) = (snap_cut(x0, lat), snap_cut(y0, lat));
        for i in 0..nx {
            for j in 0..ny {
                b.rect(licon, Rect { x: x0 + i * pitch, y: y0 + j * pitch, w: ct, h: ct });
            }
        }
        let ls = r("li_encloses_licon", 0).max(enc("li", "licon")).max(cap("li", "licon"));
        let plate = Rect { x: x0 - ls, y: y0 - ls, w: (nx - 1) * pitch + ct + 2 * ls, h: (ny - 1) * pitch + ct + 2 * ls };
        b.rect(li, plate);
        b.pin(pin(di, "E", Rect { x: x0 + (nx / 2) * pitch, y: y0 + (ny / 2) * pitch, w: ct, h: ct }, li));

        // Base band (PNP: n-tap with its n-well, the base; NPN: p-tap).
        let net = NetId(u16::MAX);
        let base = Pin { name: format!("d{di}:B"), net, layer: li, at: e };
        tap_ring(b, process, base_imp, if self.pnp { WellShape::Filled } else { WellShape::None }, e, self.base_gap, (self.ring_w, 1), &base);
        // Collector band on the substrate (PNP) or the isolating n-well ring.
        let coll = Pin { name: format!("d{di}:C"), net, layer: li, at: e };
        let bo = at(self.base_outer);
        let co = tap_ring(b, process, coll_imp, WellShape::None, bo, self.coll_gap, (self.ring_w, 1), &coll);
        if let (Some(iso), Some(nwell), Some(dnwell)) = (self.iso, process.layer("nwell"), process.layer("dnwell")) {
            let (hole, full) = (at(iso.hole), at(iso.outer));
            for band in [
                Rect { x: full.x, y: full.y, w: full.w, h: hole.y - full.y },
                Rect { x: full.x, y: hole.y + hole.h, w: full.w, h: full.y + full.h - hole.y - hole.h },
                Rect { x: full.x, y: hole.y, w: hole.x - full.x, h: hole.h },
                Rect { x: hole.x + hole.w, y: hole.y, w: full.x + full.w - hole.x - hole.w, h: hole.h },
            ] {
                b.rect(nwell, band);
            }
            b.rect(dnwell, at(iso.dnwell));
        }
        // The device marker over the unit.
        if let Some(m) = process.layer(if self.pnp { "pnp" } else { "npn" }) {
            b.rect(m, co);
        }
    }
}

/// The group's sizing, emitter defaulting to `bjt_min_emitter_side` square.
fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def = process.rule("bjt_min_emitter_side", 0);
    sizing(group, c, def, def)
}

/// PNP vs NPN from the unitization's `device_type` (no unitization: NPN).
fn device_is_pnp(group: &DeviceGroup, c: &Constraints) -> bool {
    unitization(group, c).is_some_and(|u| u.device_type == DeviceKind::Pnp)
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
            for kind in [DeviceKind::Npn, DeviceKind::Pnp] {
                dirty.extend(testkit::dirty::<Bjt>(kind, n, 1, 1000, 1000, &pdk));
            }
        }
        // A 1:8 bandgap pair (nine units), an equal pair, and 1:4, whose 3×3
        // "+" cross leaves the four corners empty.
        for counts in [[1, 8], [2, 2], [1, 4]] {
            for kind in [DeviceKind::Npn, DeviceKind::Pnp] {
                let (g, mut c) = testkit::group_of(kind, 2, 1, 1000, 1000);
                c.unitization[0].dev_nf = counts.to_vec();
                dirty.extend(testkit::dirty_group::<Bjt>(&g, &c, &pdk).into_iter().map(|d| format!("{kind:?} {counts:?} {d}")));
            }
        }
        // The [1, 8] PNP block once more through the `pnp_3p40` fixed-geometry
        // recipe overlay.
        let overlay = verify::pdk::Overlay { pdk: &pdk, recipe: pdk.recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40").unwrap() };
        let (g, mut c) = testkit::group_of(DeviceKind::Pnp, 2, 1, 1000, 1000);
        c.unitization[0].dev_nf = vec![1, 8];
        for v in Bjt::enumerate(&g, &c, &overlay) {
            let m = v.draw(&g, &c, &overlay);
            let rules = testkit::findings(&m.shapes, &testkit::ports_with(&m, &["G", "S", "B", "C"]), &pdk);
            if !rules.is_empty() {
                dirty.push(format!("overlay pnp_3p40 [1, 8] {}×{}: {rules:?}", v.rows, v.columns));
            }
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    /// A fixed-geometry recipe's emitter overrides the netlist's w/l: every
    /// drawn card and unit weight is the model's 3400×3400.
    #[test]
    fn a_fixed_geometry_model_sets_the_emitter() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let overlay = verify::pdk::Overlay { pdk: &pdk, recipe: pdk.recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40").unwrap() };
        let (g, c) = testkit::group_of(DeviceKind::Pnp, 1, 1, 150, 150);
        for v in Bjt::enumerate(&g, &c, &overlay) {
            let m = v.draw(&g, &c, &overlay);
            let at = format!("{}×{}", v.rows, v.columns);
            assert!(!m.drawn.is_empty(), "{at}");
            for d in &m.drawn {
                assert_eq!((d.w, d.l), (3400, 3400), "{at}");
            }
            for u in &m.units {
                assert_eq!(u.weight, 3400i64 * 3400, "{at}");
            }
        }
    }

    /// `dev_nf = [1, 8]` draws 9 units, each a `Pnp` drawn card at the
    /// model's fixed emitter size.
    #[test]
    fn every_emitter_is_a_drawn_card() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let overlay = verify::pdk::Overlay { pdk: &pdk, recipe: pdk.recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40").unwrap() };
        let (g, mut c) = testkit::group_of(DeviceKind::Pnp, 2, 1, 150, 150);
        c.unitization[0].dev_nf = vec![1, 8];
        for v in Bjt::enumerate(&g, &c, &overlay) {
            let m = v.draw(&g, &c, &overlay);
            let at = format!("{}×{}", v.rows, v.columns);
            assert_eq!(m.drawn.len(), 9, "{at}");
            for d in &m.drawn {
                assert_eq!(d.kind, pnr_core::DrawnKind::Pnp, "{at}");
                assert_eq!((d.w, d.l), (3400, 3400), "{at}");
            }
        }
    }

    /// Every PNP a:b (a ≤ 2, b ≤ 16, at most one odd) on every grid variant:
    /// each member's area-weighted emitter centroid on the others' within
    /// 1 nm, one recorded unit per emitter.
    #[test]
    fn every_bjt_ratio_is_common_centroid() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        for a in 1..=2u16 {
            for b in 1..=16u16 {
                if (a % 2 + b % 2) > 1 {
                    continue;
                }
                let (g, mut c) = testkit::group_of(DeviceKind::Pnp, 2, 1, 1000, 1000);
                c.unitization[0].dev_nf = vec![a, b];
                let variants = Bjt::enumerate(&g, &c, &pdk);
                assert!(!variants.is_empty(), "{a}:{b}");
                for v in variants {
                    let m = v.draw(&g, &c, &pdk);
                    let at = format!("{a}:{b} on {}×{}", v.rows, v.columns);
                    assert_eq!(m.units.len(), usize::from(a + b), "{at}");
                    let centre = |d: u8| {
                        let (w, x, y) = m.units.iter().filter(|u| u.owner == d).fold((0i64, 0i64, 0i64), |(w, x, y), u| {
                            (w + u.weight, x + u.weight * i64::from(u.x), y + u.weight * i64::from(u.y))
                        });
                        (x as f64 / w as f64, y as f64 / w as f64)
                    };
                    let ((x0, y0), (x1, y1)) = (centre(0), centre(1));
                    assert!((x0 - x1).abs() <= 1.0 && (y0 - y1).abs() <= 1.0, "{at}: ({x0}, {y0}) vs ({x1}, {y1})");
                }
            }
        }
    }
}

/// Corner cases for every BJT helper on a hand-built deck (cleanup step 2).
/// Oracles: the doc comments and values derived by hand from the deck.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::builder::fake::Deck;
    use crate::testkit::group_of;
    use pnr_core::{DrawnKind, Node};

    /// Ring 300 (the guard-ring floor beats 170 + 2·50), cuts 170 on a 340
    /// pitch, implants 100 apart: every other rule unstated.
    fn deck() -> Deck {
        Deck::new(1, &["diff", "tap", "li", "licon", "psdm", "nsdm", "nwell", "pnp", "npn"])
            .with("min_guard_ring_width", 300)
            .with("w:licon", 170)
            .with("s:licon", 170)
            .with("enc:tap:licon", 50)
            .with("s:psdm", 100)
    }

    /// `deck` with the NPN's isolation: a deep n-well and `npn_isolation`.
    fn npn_deck() -> Deck {
        let mut d = deck().with("npn_isolation", 1).with("enc:dnwell:nwell", 400).with("enc:nwell:dnwell", 200);
        d = d.with("sb:nwell:tap", 300).with("s:nwell", 1000).with("s:dnwell", 2000);
        d.roles.push(("dnwell", 99));
        d
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        outer.x <= inner.x && outer.y <= inner.y && outer.x + outer.w >= inner.x + inner.w && outer.y + outer.h >= inner.y + inner.h
    }

    fn sizing_of(w: i32, l: i32) -> Sizing {
        Sizing { unit_w: w, unit_l: l, dev_nf: vec![1] }
    }

    #[test]
    fn implants_are_opposite_for_the_two_polarities() {
        assert_eq!(implants(true), ("psdm", "nsdm", "psdm"));
        assert_eq!(implants(false), ("nsdm", "psdm", "nsdm"));
    }

    #[test]
    fn grow_moves_every_edge_out() {
        let r = Rect { x: 1, y: 2, w: 3, h: 4 };
        assert_eq!(grow(r, 2), Rect { x: -1, y: 0, w: 7, h: 8 });
        assert_eq!(grow(r, 0), r);
        assert_eq!(grow(grow(r, 5), -5), r);
    }

    #[test]
    fn an_empty_group_has_no_variants() {
        let g = DeviceGroup { devices: vec![] };
        assert!(Bjt::enumerate(&g, &Constraints::default(), &npn_deck()).is_empty());
    }

    /// An NPN needs both the deep n-well and `npn_isolation`; a PNP neither.
    #[test]
    fn an_npn_needs_the_isolated_base() {
        let (g, c) = group_of(DeviceKind::Npn, 1, 1, 1000, 1000);
        assert!(Bjt::enumerate(&g, &c, &deck()).is_empty(), "no dnwell");
        let mut no_rule = deck();
        no_rule.roles.push(("dnwell", 99));
        assert!(Bjt::enumerate(&g, &c, &no_rule).is_empty(), "no npn_isolation");
        assert!(!Bjt::enumerate(&g, &c, &npn_deck()).is_empty());
        let (g, c) = group_of(DeviceKind::Pnp, 1, 1, 1000, 1000);
        assert!(!Bjt::enumerate(&g, &c, &deck()).is_empty());
    }

    /// No unitization reads as an NPN.
    #[test]
    fn polarity_defaults_to_npn() {
        let (g, c) = group_of(DeviceKind::Pnp, 1, 1, 1000, 1000);
        assert!(device_is_pnp(&g, &c));
        assert!(!device_is_pnp(&g, &Constraints::default()));
        let (g, c) = group_of(DeviceKind::Npn, 1, 1, 1000, 1000);
        assert!(!device_is_pnp(&g, &c));
    }

    #[test]
    fn a_lone_member_offers_row_column_and_square() {
        let (g, c) = group_of(DeviceKind::Pnp, 1, 4, 1000, 1000);
        let v: Vec<(u16, u16)> = Bjt::enumerate(&g, &c, &deck()).iter().map(|b| (b.rows, b.columns)).collect();
        assert_eq!(v, [(4, 1), (2, 2), (1, 4)]);
    }

    /// Hand-derived PNP unit: ring 300, base gap = implant spacing 100,
    /// collector gap 100 (the negative well term loses), so the outer edge
    /// is 1000 + 2·(100 + 300)·2 = 2600 and units step 2600 − 300.
    #[test]
    fn a_pnp_unit_takes_the_deck_s_gaps() {
        let u = Unit::new(&sizing_of(1000, 1000), true, &deck());
        assert_eq!(u.emitter, Rect { x: 0, y: 0, w: 1000, h: 1000 });
        assert_eq!((u.ring_w, u.base_gap, u.coll_gap), (300, 100, 100));
        assert_eq!(u.base_outer, Rect { x: -400, y: -400, w: 1800, h: 1800 });
        assert!(u.iso.is_none());
        assert_eq!(u.pitch, (2300, 2300));
    }

    #[test]
    fn the_emitter_never_undercuts_the_minimum_side() {
        let d = deck().with("bjt_min_emitter_side", 1500);
        let u = Unit::new(&sizing_of(1000, 2000), true, &d);
        assert_eq!((u.emitter.w, u.emitter.h), (1500, 2000));
    }

    #[test]
    fn a_fixed_geometry_recipe_overrides_the_netlist_size() {
        let d = deck().with("bjt_emitter_w", 3400).with("bjt_emitter_l", 2000);
        let u = Unit::new(&sizing_of(150, 150), true, &d);
        assert_eq!((u.emitter.w, u.emitter.h), (3400, 2000));
    }

    /// NPN isolation nests: base band inside the ring's hole (a well-to-tap
    /// spacing past it), the hole inside the deep well, the ring's outer
    /// edge past the deep well and the collector band; units step at least
    /// the well and deep-well spacings apart.
    #[test]
    fn an_npn_unit_nests_its_isolation() {
        let d = npn_deck();
        let u = Unit::new(&sizing_of(1000, 1000), false, &d);
        let iso = u.iso.expect("an NPN is isolated");
        assert_eq!(iso.hole, grow(u.base_outer, 300));
        assert_eq!(iso.dnwell, grow(iso.hole, 400));
        assert!(contains(iso.outer, grow(iso.dnwell, 200)), "{:?}", (iso.outer, iso.dnwell));
        assert!(contains(iso.outer, grow(u.base_outer, u.coll_gap + u.ring_w)));
        assert!(u.pitch.0 >= iso.outer.w + 1000 && u.pitch.0 >= iso.dnwell.w + 2000);
        assert!(u.pitch.1 >= iso.outer.h + 1000 && u.pitch.1 >= iso.dnwell.h + 2000);
    }

    #[test]
    fn a_pnp_unit_draws_one_card_one_unit_and_three_terminals() {
        let d = deck();
        let (g, c) = group_of(DeviceKind::Pnp, 1, 1, 1000, 1000);
        let m = Bjt { rows: 1, columns: 1 }.draw(&g, &c, &d);
        assert_eq!(m.units.len(), 1);
        assert_eq!(m.units[0].weight, 1_000_000);
        assert_eq!(m.drawn.len(), 1);
        let card = m.drawn[0];
        assert_eq!(card.kind, DrawnKind::Pnp);
        assert_eq!(card.nodes, [Node::Pin("C"), Node::Pin("B"), Node::Pin("E")]);
        assert_eq!((card.w, card.l), (1000, 1000));
        for t in ["d0:E", "d0:B", "d0:C"] {
            assert!(m.pins.iter().any(|p| p.name == t), "{t}");
        }
        // The `pnp` marker covers the whole unit.
        let marker = d.layer("pnp").unwrap();
        assert!(m.shapes.iter().any(|s| s.layer == marker));
    }

    #[test]
    fn an_npn_unit_draws_its_ring_and_deep_well() {
        let d = npn_deck();
        let (g, c) = group_of(DeviceKind::Npn, 1, 1, 1000, 1000);
        let m = Bjt { rows: 1, columns: 1 }.draw(&g, &c, &d);
        assert_eq!(m.drawn[0].kind, DrawnKind::Npn);
        let (nwell, dnwell) = (d.layer("nwell").unwrap(), d.layer("dnwell").unwrap());
        assert_eq!(m.shapes.iter().filter(|s| s.layer == nwell).count(), 4, "four ring bands");
        assert_eq!(m.shapes.iter().filter(|s| s.layer == dnwell).count(), 1);
    }

    /// 1:8 on 3×3: the lone unit sits exactly on the eight's centroid.
    #[test]
    fn a_one_to_eight_pair_centres_the_one() {
        let (g, mut c) = group_of(DeviceKind::Pnp, 2, 1, 1000, 1000);
        c.unitization[0].dev_nf = vec![1, 8];
        let m = Bjt { rows: 3, columns: 3 }.draw(&g, &c, &deck());
        assert_eq!(m.units.len(), 9);
        let one: Vec<_> = m.units.iter().filter(|u| u.owner == 0).collect();
        assert_eq!(one.len(), 1);
        let eight: Vec<_> = m.units.iter().filter(|u| u.owner == 1).collect();
        let sx: i64 = eight.iter().map(|u| i64::from(u.x)).sum();
        let sy: i64 = eight.iter().map(|u| i64::from(u.y)).sum();
        assert_eq!((sx, sy), (8 * i64::from(one[0].x), 8 * i64::from(one[0].y)));
    }

    #[test]
    fn drawing_is_deterministic() {
        let (g, mut c) = group_of(DeviceKind::Pnp, 2, 1, 1000, 1000);
        c.unitization[0].dev_nf = vec![2, 2];
        let d = deck();
        for v in Bjt::enumerate(&g, &c, &d) {
            assert_eq!(v.draw(&g, &c, &d), v.draw(&g, &c, &d));
        }
    }

    /// A deck that states no contact size or spacing still draws (a
    /// degenerate deck is reported by signoff, never a panic here).
    #[test]
    fn a_deck_without_contact_rules_draws() {
        let d = Deck::new(1, &["diff", "tap", "li", "licon", "psdm", "nsdm"]);
        let (g, c) = group_of(DeviceKind::Pnp, 1, 1, 1000, 1000);
        let m = Bjt { rows: 1, columns: 1 }.draw(&g, &c, &d);
        assert_eq!(m.drawn.len(), 1);
    }
}
