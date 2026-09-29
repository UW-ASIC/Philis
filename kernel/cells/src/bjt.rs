//! BJT generator: the vertical bipolar of a CMOS process, concentric per
//! unit. PNP: a p+ emitter, an n-tap base ring whose n-well is the base, and
//! a p-tap collector ring on the substrate (the collector), under the `pnp`
//! marker. NPN: an n+ emitter, a p-tap base ring, both in the p-region an
//! n-well ring and a deep n-well isolate, the collector tapped on that ring.
//! Every gap and width is the deck's.

use crate::builder::dim;
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, NetId, Pin, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
use crate::post_cell::tap_ring;
use crate::Cell;

/// One BJT variant: the group's unit devices (each member's `dev_nf` units)
/// arrayed in `columns`, filled centre-out with the smallest member first: a
/// 1:8 pair lands as the classic 3×3, the one unit centred in the eight
/// (Hastings §10.2, ratioed bipolars). Area ratio comes from unit count,
/// never emitter scaling. Neighbouring units share their collector band.
#[derive(Clone)]
pub struct Bjt {
    pub columns: u16,
}

impl Cell for Bjt {
    fn enumerate(group: &DeviceGroup, _constraints: &Constraints, _process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        // An NPN needs an isolated p-base: a deep n-well under an n-well
        // ring. Only where the process declares that construction
        // (`npn_isolation`), and has the deep well.
        if !device_is_pnp(group, _constraints) && (_process.layer("dnwell").is_none() || _process.rule("npn_isolation", 0) == 0) {
            return vec![];
        }
        let s = group_sizing(group, _constraints, _process);
        let n = s.dev_nf.iter().map(|&u| u.max(1)).sum::<u16>().max(1);
        let square = (f64::from(n).sqrt().ceil() as u16).max(1);
        // A matched set only as the square: a line centres it in one axis
        // only and strings its routing out.
        let mut cols = if group.devices.len() > 1 { vec![square] } else { vec![1, n, square] };
        cols.sort_unstable();
        cols.dedup();
        cols.into_iter()
            .map(|columns| Bjt { columns })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let pnp = device_is_pnp(group, constraints);
        let u = Unit::new(&s, pnp, process);
        let owners = unit_order(&s.dev_nf, i32::from(self.columns.max(1)));
        let total = owners.iter().flatten().count();
        let cols = i32::from(self.columns.max(1)).min(total.max(1) as i32);
        // PNP units abut on a shared collector band (the substrate); NPN
        // units each keep their own isolation, the deck's spacings apart.
        let lat = cut_lattice(process);
        let (px, py) = (snap_cut(u.pitch.0 + lat - 1, lat), snap_cut(u.pitch.1 + lat - 1, lat));
        for (slot, di) in owners.iter().enumerate().filter_map(|(i, d)| Some((i, (*d)?))) {
            let (ox, oy) = ((slot as i32 % cols) * px, (slot as i32 / cols) * py);
            u.draw(&mut b, process, di, ox, oy);
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
        let (emit_imp, base_imp, coll_imp) = if pnp { ("psdm", "nsdm", "psdm") } else { ("nsdm", "psdm", "nsdm") };
        let min_side = r("bjt_min_emitter_side", 0);
        let emitter = Rect { x: 0, y: 0, w: s.unit_w.max(min_side), h: s.unit_l.max(min_side) };
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
        let grow = |x: Rect, d: i32| Rect { x: x.x - d, y: x.y - d, w: x.w + 2 * d, h: x.h + 2 * d };
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
            (hole, full, dn)
        });
        let pitch = match iso {
            None => (outer.w - ring_w, outer.h - ring_w),
            Some((_, full, dn)) => {
                let (sw, sd) = (process.space("nwell").unwrap_or(0), process.space("dnwell").unwrap_or(0));
                ((full.w + sw).max(dn.w + sd), (full.h + sw).max(dn.h + sd))
            }
        };
        Self { pnp, emitter, base_gap, base_outer, coll_gap, ring_w, iso, pitch }
    }

    fn draw(&self, b: &mut Builder, process: &dyn Process, di: usize, ox: i32, oy: i32) {
        let at = |x: Rect| Rect { x: x.x + ox, y: x.y + oy, ..x };
        let r = |name: &str, d: i32| process.rule(name, d);
        let enc = |o: &str, i: &str| process.enclosure(o, i).unwrap_or(0);
        let cap = |o: &str, i: &str| process.endcap(o, i).unwrap_or(0);
        let (emit_imp, base_imp, coll_imp) = if self.pnp { ("psdm", "nsdm", "psdm") } else { ("nsdm", "psdm", "nsdm") };
        let (diff, li, licon) = (req(process, "diff"), req(process, "li"), req(process, "licon"));
        let lat = cut_lattice(process);
        let ct = dim(process, "contact");
        let e = at(self.emitter);

        // Emitter: diffusion, its implant, a contact array under one li
        // plate (cuts `max(enclosure, end-cap)` inside the diffusion, li the
        // end-cap past the outer cuts on every side).
        b.rect(diff, e);
        let ei = enc(emit_imp, "diff");
        b.rect(req(process, emit_imp), Rect { x: e.x - ei, y: e.y - ei, w: e.w + 2 * ei, h: e.h + 2 * ei });
        let inset = r("diff_encloses_licon", 0).max(enc("diff", "licon")).max(cap("diff", "licon"));
        let pitch = ct + process.space("licon").unwrap_or(ct);
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
        tap_ring(b, process, base_imp, self.pnp, e, self.base_gap, (self.ring_w, 1), &base);
        // Collector band on the substrate (PNP) or the isolating n-well ring.
        let coll = Pin { name: format!("d{di}:C"), net, layer: li, at: e };
        let bo = at(self.base_outer);
        let co = tap_ring(b, process, coll_imp, false, bo, self.coll_gap, (self.ring_w, 1), &coll);
        if let (Some((hole, full, dn)), Some(nwell), Some(dnwell)) = (self.iso, process.layer("nwell"), process.layer("dnwell")) {
            let (hole, full) = (at(hole), at(full));
            for band in [
                Rect { x: full.x, y: full.y, w: full.w, h: hole.y - full.y },
                Rect { x: full.x, y: hole.y + hole.h, w: full.w, h: full.y + full.h - hole.y - hole.h },
                Rect { x: full.x, y: hole.y, w: hole.x - full.x, h: hole.h },
                Rect { x: hole.x + hole.w, y: hole.y, w: full.x + full.w - hole.x - hole.w, h: hole.h },
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

/// Owner per slot of a `cols`-wide grid holding every member's units: slots
/// ordered by distance from the grid centre (then angle), members smallest
/// first, so a lone unit takes the centre and a large member surrounds it.
fn unit_order(dev_nf: &[u16], cols: i32) -> Vec<Option<usize>> {
    let total: usize = dev_nf.iter().map(|&u| usize::from(u.max(1))).sum();
    let cols = (cols.max(1) as usize).min(total.max(1));
    let rows = total.div_ceil(cols);
    let key = |i: usize| {
        let (dr, dc) = (2 * (i / cols) as i64 - rows as i64 + 1, 2 * (i % cols) as i64 - cols as i64 + 1);
        (dr * dr + dc * dc, (dr as f64).atan2(dc as f64))
    };
    let mut order: Vec<usize> = (0..rows * cols).collect();
    order.sort_by(|&a, &b| key(a).0.cmp(&key(b).0).then(key(a).1.total_cmp(&key(b).1)));
    let mut members: Vec<usize> = (0..dev_nf.len()).collect();
    members.sort_by_key(|&d| (dev_nf[d], d));
    let mut owner = vec![None; rows * cols];
    let mut at = order.into_iter();
    for d in members {
        for _ in 0..dev_nf[d].max(1) {
            if let Some(i) = at.next() {
                owner[i] = Some(d);
            }
        }
    }
    // Row-major; a short grid leaves its outermost cells empty.
    owner
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
        // A 1:8 bandgap pair: nine units.
        for kind in [DeviceKind::Npn, DeviceKind::Pnp] {
            let (g, mut c) = testkit::group_of(kind, 2, 1, 1000, 1000);
            c.unitization[0].dev_nf = vec![1, 8];
            dirty.extend(testkit::dirty_group::<Bjt>(&g, &c, &pdk).into_iter().map(|d| format!("{kind:?} 1:8 {d}")));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    /// 1:8 on a 3×3: the one unit at the centre, the eight around it, both
    /// centroids on the middle cell.
    #[test]
    fn a_one_to_eight_pair_centres_the_single_unit() {
        let o = unit_order(&[1, 8], 3);
        assert_eq!(o.len(), 9);
        assert_eq!(o[4], Some(0), "{o:?}");
        assert_eq!(o.iter().filter(|&&d| d == Some(1)).count(), 8);
        // Device 1's slots are point-symmetric about the centre.
        assert!((0..9).all(|i| o[i] == o[8 - i]));
    }
}
