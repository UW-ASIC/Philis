//! BJT generator: the vertical bipolar of a CMOS process, concentric per
//! unit. PNP: a p+ emitter, an n-tap base ring whose n-well is the base, and
//! a p-tap collector ring on the substrate (the collector), under the `pnp`
//! marker. NPN: an n+ emitter, a p-tap base ring, both in the p-region an
//! n-well ring and a deep n-well isolate, the collector tapped on that ring.
//! Every gap and width is the deck's.

use crate::builder::dim;
use analog::matching::pattern::{self, Fill};
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, NetId, Pin, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
use crate::post_cell::{tap_ring, WellShape};
use crate::Cell;

/// One BJT variant: the group's unit devices (each member's `dev_nf` units)
/// on a `rows × columns` grid, owners from
/// [`pattern::centro_assign`]`(Balanced)`: every member point-symmetric about
/// the grid centre when at most one count is odd, so a 1:8 pair is the
/// classic 3×3 with the one unit centred (Hastings §10.2, ratioed bipolars).
/// Empty cells stay empty. Area ratio comes from unit count, never emitter
/// scaling. Neighbouring units share their collector band.
#[derive(Clone)]
pub struct Bjt {
    pub rows: u16,
    pub columns: u16,
}

impl Cell for Bjt {
    fn enumerate(
        group: &DeviceGroup,
        _constraints: &Constraints,
        _process: &dyn Process,
    ) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        // An NPN needs an isolated p-base: a deep n-well under an n-well
        // ring. Only where the process declares that construction
        // (`npn_isolation`), and has the deep well.
        if !device_is_pnp(group, _constraints)
            && (_process.layer("dnwell").is_none() || _process.rule("npn_isolation", 0) == 0)
        {
            return vec![];
        }
        let s = group_sizing(group, _constraints, _process);
        let counts = unit_counts(&s);
        let n = counts.iter().sum::<u16>();
        if group.devices.len() > 1 {
            return pattern::grids(&counts, 3.0)
                .into_iter()
                .map(|(r, c)| Bjt {
                    rows: r as u16,
                    columns: c as u16,
                })
                .collect();
        }
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        cols.into_iter()
            .map(|columns| Bjt {
                rows: n.div_ceil(columns),
                columns,
            })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let pnp = device_is_pnp(group, constraints);
        let u = Unit::new(&s, pnp, process);
        let cols = i32::from(self.columns);
        let (owners, _) = pattern::centro_assign(
            &unit_counts(&s),
            usize::from(self.rows),
            usize::from(self.columns),
            Fill::Balanced,
        );
        // PNP units abut on a shared collector band (the substrate); NPN
        // units each keep their own isolation, the deck's spacings apart.
        let lat = cut_lattice(process);
        let (px, py) = (
            snap_cut(u.pitch.0 + lat - 1, lat),
            snap_cut(u.pitch.1 + lat - 1, lat),
        );
        for (slot, di) in owners
            .iter()
            .enumerate()
            .filter_map(|(i, d)| Some((i, (*d)?)))
        {
            let (ox, oy) = ((slot as i32 % cols) * px, (slot as i32 / cols) * py);
            u.draw(&mut b, process, usize::from(di), ox, oy);
        }
        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// One unit's geometry at the origin: the emitter, the base band's outer
/// edge, the collector band's outer edge.
struct Unit {
    pnp: bool,
    emitter: Rect,
    base_gap: i32,
    base_outer: Rect,
    coll_gap: i32,
    ring_w: i32,
    /// NPN isolation: the n-well ring's hole and outer edge, the deep well.
    iso: Option<(Rect, Rect, Rect)>,
    /// Unit-to-unit step.
    pitch: (i32, i32),
}

impl Unit {
    fn new(s: &Sizing, pnp: bool, process: &dyn Process) -> Self {
        let r = |name: &str, d: i32| process.rule(name, d);
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let (emit_imp, base_imp, coll_imp) = if pnp {
            ("psdm", "nsdm", "psdm")
        } else {
            ("nsdm", "psdm", "nsdm")
        };
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
        let emitter = Rect {
            x: 0,
            y: 0,
            w: ew,
            h: el,
        };
        let ct = dim(process, "contact");
        let ring_w = r("min_guard_ring_width", 0)
            .max(ct + 2 * enc("tap", "licon"))
            .max(ct + 2 * r("diff_encloses_licon", 0));
        let clear = ["psdm", "nsdm", "tap", "diff"]
            .iter()
            .filter_map(|x| process.space(x))
            .max()
            .unwrap_or(0);
        // Emitter to base band: diffusion clearance, and the two implants
        // meeting at most edge to edge.
        // An implant past its diffusion keeps the deck's implant-to-opposite
        // diffusion and contact spacings from the next band.
        let beyond = |imp: &str| {
            process
                .space_between(imp, "tap")
                .unwrap_or(0)
                .max(process.space_between(imp, "licon").unwrap_or(0))
        };
        let base_gap = clear
            .max(process.space_between("tap", "diff").unwrap_or(0))
            .max(enc(emit_imp, "diff") + enc(base_imp, "tap"))
            .max(enc(emit_imp, "diff") + beyond(emit_imp))
            .max(enc(base_imp, "tap") + process.space_between(base_imp, "diff").unwrap_or(0));
        let grow = |x: Rect, d: i32| Rect {
            x: x.x - d,
            y: x.y - d,
            w: x.w + 2 * d,
            h: x.h + 2 * d,
        };
        let base_outer = grow(emitter, base_gap + ring_w);
        // Base band to collector band: the base implant against the
        // collector's, and the well (PNP: the base n-well; NPN: the isolating
        // n-well ring, drawn past the collector band) to the tap.
        let nw = dim(process, "nwell_diff_enc");
        // Neighbouring units' base wells keep the well spacing across the
        // shared collector band.
        let well_gap = nw
            + (r("nwell_min_spacing", 0).max(process.space("nwell").unwrap_or(0)) - ring_w + 1) / 2;
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
        let coll_gap = if pnp {
            coll_gap
        } else {
            coll_gap.max(sp_nt + nw)
        };
        let outer = grow(base_outer, coll_gap + ring_w);
        let iso = (!pnp).then(|| {
            let hole = grow(base_outer, sp_nt);
            let dn = grow(hole, process.enclosure("dnwell", "nwell").unwrap_or(0));
            let past = process.enclosure("nwell", "dnwell").unwrap_or(0);
            let reach = (dn.x - hole.x).abs() + past;
            let full = grow(hole, reach.max(hole.x - outer.x + nw));
            (hole, full, dn)
        });
        let pitch = match iso {
            None => (outer.w - ring_w, outer.h - ring_w),
            Some((_, full, dn)) => {
                let (sw, sd) = (
                    process.space("nwell").unwrap_or(0),
                    process.space("dnwell").unwrap_or(0),
                );
                ((full.w + sw).max(dn.w + sd), (full.h + sw).max(dn.h + sd))
            }
        };
        Self {
            pnp,
            emitter,
            base_gap,
            base_outer,
            coll_gap,
            ring_w,
            iso,
            pitch,
        }
    }

    fn draw(&self, b: &mut Builder, process: &dyn Process, di: usize, ox: i32, oy: i32) {
        let at = |x: Rect| Rect {
            x: x.x + ox,
            y: x.y + oy,
            ..x
        };
        let r = |name: &str, d: i32| process.rule(name, d);
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let cap = |o: &str, i: &str| process.endcap(o, i).unwrap_or(0);
        let (emit_imp, base_imp, coll_imp) = if self.pnp {
            ("psdm", "nsdm", "psdm")
        } else {
            ("nsdm", "psdm", "nsdm")
        };
        let (diff, li, licon) = (
            req(process, "diff"),
            req(process, "li"),
            req(process, "licon"),
        );
        let lat = cut_lattice(process);
        let ct = dim(process, "contact");
        let e = at(self.emitter);
        b.unit(pnr_core::Unit {
            owner: di as u8,
            x: e.x + e.w / 2,
            y: e.y + e.h / 2,
            weight: i64::from(e.w) * i64::from(e.h),
            phi: (0, 0),
            sa: 0,
            sb: 0,
        });
        // Pin order matches `cellgen::BJT_PINS`.
        b.drawn(pnr_core::Drawn {
            owner: di as u8,
            device: None,
            kind: if self.pnp {
                pnr_core::DrawnKind::Pnp
            } else {
                pnr_core::DrawnKind::Npn
            },
            nodes: [
                pnr_core::Node::Pin("C"),
                pnr_core::Node::Pin("B"),
                pnr_core::Node::Pin("E"),
            ],
            w: self.emitter.w,
            l: self.emitter.h,
        });

        // Emitter: diffusion, its implant, a contact array under one li
        // plate (cuts `max(enclosure, end-cap)` inside the diffusion, li the
        // end-cap past the outer cuts on every side).
        b.rect(diff, e);
        let ei = enc(emit_imp, "diff");
        b.rect(
            req(process, emit_imp),
            Rect {
                x: e.x - ei,
                y: e.y - ei,
                w: e.w + 2 * ei,
                h: e.h + 2 * ei,
            },
        );
        let inset = r("diff_encloses_licon", 0)
            .max(enc("diff", "licon"))
            .max(cap("diff", "licon"));
        let pitch = ct + process.space("licon").unwrap_or(ct);
        let fit = |len: i32| ((len - 2 * inset - ct) / pitch + 1).max(1);
        let (nx, ny) = (fit(e.w), fit(e.h));
        let (x0, y0) = (
            e.x + (e.w - (nx - 1) * pitch - ct) / 2,
            e.y + (e.h - (ny - 1) * pitch - ct) / 2,
        );
        let (x0, y0) = (snap_cut(x0, lat), snap_cut(y0, lat));
        for i in 0..nx {
            for j in 0..ny {
                b.rect(
                    licon,
                    Rect {
                        x: x0 + i * pitch,
                        y: y0 + j * pitch,
                        w: ct,
                        h: ct,
                    },
                );
            }
        }
        let ls = r("li_encloses_licon", 0)
            .max(enc("li", "licon"))
            .max(cap("li", "licon"));
        let plate = Rect {
            x: x0 - ls,
            y: y0 - ls,
            w: (nx - 1) * pitch + ct + 2 * ls,
            h: (ny - 1) * pitch + ct + 2 * ls,
        };
        b.rect(li, plate);
        b.pin(pin(
            di,
            "E",
            Rect {
                x: x0 + (nx / 2) * pitch,
                y: y0 + (ny / 2) * pitch,
                w: ct,
                h: ct,
            },
            li,
        ));

        // Base band (PNP: n-tap with its n-well, the base; NPN: p-tap).
        let net = NetId(u16::MAX);
        let base = Pin {
            name: format!("d{di}:B"),
            net,
            layer: li,
            at: e,
        };
        tap_ring(
            b,
            process,
            base_imp,
            if self.pnp {
                WellShape::Filled
            } else {
                WellShape::None
            },
            e,
            self.base_gap,
            (self.ring_w, 1),
            &base,
        );
        // Collector band on the substrate (PNP) or the isolating n-well ring.
        let coll = Pin {
            name: format!("d{di}:C"),
            net,
            layer: li,
            at: e,
        };
        let bo = at(self.base_outer);
        let co = tap_ring(
            b,
            process,
            coll_imp,
            WellShape::None,
            bo,
            self.coll_gap,
            (self.ring_w, 1),
            &coll,
        );
        if let (Some((hole, full, dn)), Some(nwell), Some(dnwell)) =
            (self.iso, process.layer("nwell"), process.layer("dnwell"))
        {
            let (hole, full) = (at(hole), at(full));
            for band in [
                Rect {
                    x: full.x,
                    y: full.y,
                    w: full.w,
                    h: hole.y - full.y,
                },
                Rect {
                    x: full.x,
                    y: hole.y + hole.h,
                    w: full.w,
                    h: full.y + full.h - hole.y - hole.h,
                },
                Rect {
                    x: full.x,
                    y: hole.y,
                    w: hole.x - full.x,
                    h: hole.h,
                },
                Rect {
                    x: hole.x + hole.w,
                    y: hole.y,
                    w: full.x + full.w - hole.x - hole.w,
                    h: hole.h,
                },
            ] {
                b.rect(nwell, band);
            }
            b.rect(dnwell, at(dn));
        }
        // The device marker over the unit.
        if let Some(m) = process.layer(if self.pnp { "pnp" } else { "npn" }) {
            b.rect(m, co);
        }
    }
}

/// Units per member: a member without units draws one.
fn unit_counts(s: &Sizing) -> Vec<u16> {
    s.dev_nf.iter().map(|&u| u.max(1)).collect()
}

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
                dirty.extend(
                    testkit::dirty_group::<Bjt>(&g, &c, &pdk)
                        .into_iter()
                        .map(|d| format!("{kind:?} {counts:?} {d}")),
                );
            }
        }
        // The [1, 8] PNP block once more through the `pnp_3p40` fixed-geometry
        // recipe overlay.
        let overlay = verify::pdk::Overlay {
            pdk: &pdk,
            recipe: pdk
                .recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40")
                .unwrap(),
        };
        let (g, mut c) = testkit::group_of(DeviceKind::Pnp, 2, 1, 1000, 1000);
        c.unitization[0].dev_nf = vec![1, 8];
        for v in Bjt::enumerate(&g, &c, &overlay) {
            let m = v.draw(&g, &c, &overlay);
            let rules = testkit::findings(
                &m.shapes,
                &testkit::ports_with(&m, &["G", "S", "B", "C"]),
                &pdk,
            );
            if !rules.is_empty() {
                dirty.push(format!(
                    "overlay pnp_3p40 [1, 8] {}×{}: {rules:?}",
                    v.rows, v.columns
                ));
            }
        }
        assert!(
            dirty.is_empty(),
            "DRC/ERC-dirty variants:\n{}",
            dirty.join("\n")
        );
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
        let overlay = verify::pdk::Overlay {
            pdk: &pdk,
            recipe: pdk
                .recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40")
                .unwrap(),
        };
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
        let overlay = verify::pdk::Overlay {
            pdk: &pdk,
            recipe: pdk
                .recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40")
                .unwrap(),
        };
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
                        let (w, x, y) = m.units.iter().filter(|u| u.owner == d).fold(
                            (0i64, 0i64, 0i64),
                            |(w, x, y), u| {
                                (
                                    w + u.weight,
                                    x + u.weight * i64::from(u.x),
                                    y + u.weight * i64::from(u.y),
                                )
                            },
                        );
                        (x as f64 / w as f64, y as f64 / w as f64)
                    };
                    let ((x0, y0), (x1, y1)) = (centre(0), centre(1));
                    assert!(
                        (x0 - x1).abs() <= 1.0 && (y0 - y1).abs() <= 1.0,
                        "{at}: ({x0}, {y0}) vs ({x1}, {y1})"
                    );
                }
            }
        }
    }
}
