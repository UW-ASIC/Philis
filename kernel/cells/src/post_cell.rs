//! Guard rings, drawn around **placed** devices after placement and before
//! routing (a ring is both an obstacle and a net target). Same-class
//! shareable requesters placed within `guard_ring_merge_gap_nm` share one
//! ring, unless the merged hull would cross a non-member cell: only each
//! requester's own bbox (inflated by [`ring_halo`]) was reserved.

use crate::builder::dim;
use analog::cell::{GuardRingRequirement, GuardRingType};
use analog::Constraints;
use pnr_core::{DeviceId, Layout, Macro, Pin, Process, Rect, Target};

use crate::builder::{cut_lattice, req, snap_cut};
use crate::Builder;

/// Rings of same-class cells placed within this gap merge into one: a
/// design choice (fewer rings, shared taps), overridable per process.
const RING_MERGE_GAP_NM: i32 = 2_000;

/// Draw every guard ring the placed `layout` calls for. One [`Macro`] per ring
/// (carrying its `ring` pin), to be folded into the layout geometry before
/// routing.
///
/// `cut_ohm` is one tap contact's resistance (the deck's `pex` value); it
/// sizes each ring's contact rows against `max_ring_resistance_mohm`.
/// Requirements naming an unplaced device are skipped.
///
/// # Panics
/// When the deck has no `li`, `tap`, `licon` or ring-implant layer.
///
/// # Time complexity
/// *O*(r² + r·d) for `r` requirements and `d` placed devices (clustering).
#[must_use]
pub fn guard_rings(layout: &Layout, c: &Constraints, process: &dyn Process, cut_ohm: f32) -> Vec<Macro> {
    let n_dev = layout.x.len();
    // Only requirements whose device is actually placed.
    let reqs: Vec<&GuardRingRequirement> = c
        .guard_rings
        .iter()
        .filter(|r| (r.device.0 as usize) < n_dev)
        .collect();
    if reqs.is_empty() {
        return Vec::new();
    }

    // Design policy, not process data: rings of cells this close share one.
    let merge_gap = process.rule("guard_ring_merge_gap_nm", RING_MERGE_GAP_NM) as f32;

    clusters(&reqs, layout, merge_gap)
        .into_iter()
        .map(|members| {
            // Inner bbox = union of the cluster's placed device boxes.
            let inner = members
                .iter()
                .map(|&i| dev_rect(layout, reqs[i].device))
                .reduce(union_rect)
                .expect("cluster is non-empty");
            // Same class, so the widest requirement speaks for the cluster.
            let rep = members
                .iter()
                .map(|&i| reqs[i])
                .max_by_key(|r| r.min_width_nm)
                .expect("cluster is non-empty");
            // Placed bboxes carry the halo; shrink back to the device so the
            // band lands inside the reservation.
            let ext = ring_halo(rep, process, cut_ohm);
            let inner = Rect {
                x: inner.x + ext,
                y: inner.y + ext,
                w: (inner.w - 2 * ext).max(0),
                h: (inner.h - 2 * ext).max(0),
            };
            let mut b = Builder::new(process.grid());
            draw_ring(&mut b, process, rep, inner, cut_ohm);
            b.finish()
        })
        .collect()
}

/// Partition requirements into rings: same-class (`connection_net`, `ring_type`)
/// **shareable** requesters that are placed within `merge_gap` merge; every
/// non-shareable requirement is its own ring. A merged cluster whose hull
/// would contain or intersect a placed cell outside the cluster splits into
/// the largest sub-clusters that do not — the hull covers unreserved ground,
/// see the module doc.
/// **Pure** (no PDK) so the grouping policy is table-testable.
fn clusters(reqs: &[&GuardRingRequirement], layout: &Layout, merge_gap: f32) -> Vec<Vec<usize>> {
    let n = reqs.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (reqs[i], reqs[j]);
            let same_class = a.connection_net == b.connection_net && a.ring_type == b.ring_type && a.role == b.role;
            let adjacent = layout
                .edge_gap(Target::Device(a.device), Target::Device(b.device))
                <= merge_gap;
            if a.shareable && b.shareable && same_class && adjacent {
                union(&mut parent, i, j);
            }
        }
    }
    // Group indices by their union-find root, preserving first-seen order.
    let mut roots: Vec<usize> = Vec::new();
    let mut out: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        match roots.iter().position(|&x| x == r) {
            Some(p) => out[p].push(i),
            None => {
                roots.push(r);
                out.push(vec![i]);
            }
        }
    }
    // A cluster whose hull hits a non-member splits into sub-clusters, each
    // grown greedily (left to right) while its hull stays clear of foreign
    // cells and the newcomer is adjacent to a member.
    // ponytail: greedy, not the fewest sub-clusters; exact is set partition.
    let mut safe: Vec<Vec<usize>> = Vec::with_capacity(out.len());
    for mut members in out {
        if members.len() < 2 || !hull_hits_foreign(&members, reqs, layout) {
            safe.push(members);
            continue;
        }
        members.sort_by_key(|&i| (layout.x[reqs[i].device.0 as usize], layout.y[reqs[i].device.0 as usize]));
        let mut subs: Vec<Vec<usize>> = Vec::new();
        for i in members {
            let near = |j: &usize| {
                layout.edge_gap(Target::Device(reqs[i].device), Target::Device(reqs[*j].device)) <= merge_gap
            };
            let home = subs.iter_mut().find(|sub| {
                sub.iter().any(near) && {
                    let trial: Vec<usize> = sub.iter().copied().chain([i]).collect();
                    !hull_hits_foreign(&trial, reqs, layout)
                }
            });
            match home {
                Some(sub) => sub.push(i),
                None => subs.push(vec![i]),
            }
        }
        safe.extend(subs);
    }
    safe
}

/// Would a ring drawn around `members`' union hull contain or intersect a
/// placed cell outside the cluster? The hull is of **placed** bboxes, which
/// already carry each member's [`ring_halo`], so all drawn ring geometry lies
/// strictly inside it; overlap is strict, because flush abutment at the hull
/// edge is the stand-off a singleton ring already tolerates (the per-class
/// [`outer_clear`]).
fn hull_hits_foreign(members: &[usize], reqs: &[&GuardRingRequirement], layout: &Layout) -> bool {
    let hull = members
        .iter()
        .map(|&i| dev_rect(layout, reqs[i].device))
        .reduce(union_rect)
        .expect("cluster is non-empty");
    let (x0, y0) = (hull.x, hull.y);
    let (x1, y1) = (hull.x + hull.w, hull.y + hull.h);
    (0..layout.x.len()).any(|d| {
        if members.iter().any(|&i| reqs[i].device.0 as usize == d) {
            return false;
        }
        layout.x[d] - layout.hw[d] < x1
            && layout.x[d] + layout.hw[d] > x0
            && layout.y[d] - layout.hh[d] < y1
            && layout.y[d] + layout.hh[d] > y0
    })
}

/// Union-find root of `i`, halving the path on the way.
fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]]; // path-halving
        i = parent[i];
    }
    i
}

/// Joins `i`'s and `j`'s union-find sets.
fn union(parent: &mut [usize], i: usize, j: usize) {
    let (ri, rj) = (find(parent, i), find(parent, j));
    if ri != rj {
        parent[ri] = rj;
    }
}

/// Placed bbox of device `d` (halo included) as a corner rect.
fn dev_rect(l: &Layout, d: DeviceId) -> Rect {
    let (cx, cy, hw, hh) = l.bbox(Target::Device(d));
    Rect { x: cx - hw, y: cy - hh, w: 2 * hw, h: 2 * hh }
}

/// Smallest rect covering `a` and `b`.
fn union_rect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// Device-to-band gap, the deck's: the widest spacing among the implants,
/// tap and diffusion, so the band clears a cell's diffusion and its implant
/// of either type (sky130 380, gf180 400).
fn ring_clear(process: &dyn Process) -> i32 {
    ["psdm", "nsdm", "tap", "diff"].iter().filter_map(|r| process.space(r)).max().unwrap_or(0)
}

/// Draw the four contacted tap/implant/li bands of `r`'s ring around `inner`.
///
/// No met1 band: routing is met1-and-up, so a met1 loop would short every
/// route crossing it. The `ring` pins sit on li; the router stitches down.
fn draw_ring(b: &mut Builder, process: &dyn Process, r: &GuardRingRequirement, inner: Rect, cut_ohm: f32) {
    let perimeter = 2 * i64::from(inner.w + inner.h + 4 * ring_clear(process));
    let (width, rows) = band(process, r, perimeter, cut_ohm);
    let gap = ring_gap(process, r, width);
    let pin = Pin { name: "ring".into(), net: r.connection_net, layer: req(process, "li"), at: Rect { x: 0, y: 0, w: 0, h: 0 } };
    let o = tap_ring(b, process, implant_name(r.ring_type), well_shape(r.ring_type), inner, gap, (width, rows), &pin);
    // A tub: its n-well band, and the deep n-well under the band's hole and
    // past it by nwell.6 (the band well passes the deep well by nwell.5).
    if let (GuardRingType::Tub { .. }, Some(nwell), Some(dnwell)) = (r.ring_type, process.layer("nwell"), process.layer("dnwell")) {
        let g = tub_well_ext(process, width);
        let tap_in = Rect { x: o.x + width, y: o.y + width, w: o.w - 2 * width, h: o.h - 2 * width };
        band_well(b, nwell, o, tap_in, g);
        let e = process.enclosure("dnwell", "nwell").unwrap_or(0) - g;
        b.rect(dnwell, Rect { x: tap_in.x - e, y: tap_in.y - e, w: tap_in.w + 2 * e, h: tap_in.h + 2 * e });
    }
}

/// An n-well band around a tap band (outer edge `o`, inner edge `inner`),
/// each side grown by `g`, full length: they overlap at the corners, so the
/// union is one ring with the interior left out.
fn band_well(b: &mut Builder, nwell: pnr_core::LayerId, o: Rect, inner: Rect, g: i32) {
    let (fw, fh, bw) = (o.w + 2 * g, o.h + 2 * g, inner.x - o.x + 2 * g);
    for r in [
        Rect { x: o.x - g, y: o.y - g, w: fw, h: bw },
        Rect { x: o.x - g, y: inner.y + inner.h - g, w: fw, h: bw },
        Rect { x: o.x - g, y: o.y - g, w: bw, h: fh },
        Rect { x: inner.x + inner.w - g, y: o.y - g, w: bw, h: fh },
    ] {
        b.rect(nwell, r);
    }
}

/// A contacted tap ring `gap` outside `inner`: four bands of tap under
/// `implant` and li, `rows` rows of cuts along each, a copy of `pin` on
/// each band, and its `well` (see [`WellShape`]). Returns the band's outer
/// edge. A band of non-positive extent is skipped.
///
/// # Panics
/// When the deck has no `implant`, `tap`, `li` or `licon` layer.
pub(crate) fn tap_ring(b: &mut Builder, process: &dyn Process, implant: &str, well: WellShape, inner: Rect, gap: i32, (ring_width, rows): (i32, i32), pin: &Pin) -> Rect {
    let ct = dim(process, "contact");
    let lat = cut_lattice(process);
    let pitch = cut_pitch(process);
    let (ix0, iy0) = (inner.x - gap, inner.y - gap);
    let (ix1, iy1) = (inner.x + inner.w + gap, inner.y + inner.h + gap);
    let (ox0, oy0, ox1, oy1) = (ix0 - ring_width, iy0 - ring_width, ix1 + ring_width, iy1 + ring_width);
    // The implant passes the tap by the deck's enclosure, never under its
    // own width.
    let imp_enc = process
        .enclosure(implant, "tap")
        .unwrap_or(0)
        .max((process.width(implant).unwrap_or(0) - ring_width + 1) / 2);
    let implant = req(process, implant);
    let tap = req(process, "tap");
    let li = req(process, "li");
    let licon = req(process, "licon");

    // bottom, top, left, right.
    let bands: [(i32, i32, i32, i32); 4] = [
        (ox0, oy0, ox1 - ox0, ring_width),
        (ox0, iy1, ox1 - ox0, ring_width),
        (ox0, oy0 + ring_width, ring_width, iy1 - iy0),
        (ix1, oy0 + ring_width, ring_width, iy1 - iy0),
    ];
    for &(bx, by, bw, bh) in &bands {
        if bw <= 0 || bh <= 0 {
            continue;
        }
        let band = Rect { x: bx, y: by, w: bw, h: bh };
        b.rect(implant, Rect { x: bx - imp_enc, y: by - imp_enc, w: bw + 2 * imp_enc, h: bh + 2 * imp_enc });
        b.rect(li, band);
        b.rect(tap, band);
        // `rows` lines of cuts along the band, centred across it.
        let margin = (ring_width - ct - (rows - 1) * pitch) / 2;
        for row in 0..rows {
            let off = margin + row * pitch;
            // Along the band, cuts sit on one pitch grid from the cell
            // origin: two rings sharing a band draw the very same cuts.
            let on_grid = |v: i32| v.div_euclid(pitch) * pitch + if v.rem_euclid(pitch) == 0 { 0 } else { pitch };
            if bw >= bh {
                let cy = snap_cut(by + off, lat);
                let mut cx = on_grid(bx + margin);
                while cx + ct <= bx + bw - margin {
                    b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                    cx += pitch;
                }
            } else {
                // A pitch clear of the horizontal bands' cut rows at either
                // end (the corners are theirs).
                let cx = snap_cut(bx + off, lat);
                let mut cy = on_grid(by - ring_width + margin + rows * pitch);
                while cy + pitch <= by + bh + margin {
                    b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                    cy += pitch;
                }
            }
        }
        b.pin(Pin { at: Rect { x: bx + bw / 2 - ct / 2, y: by + bh / 2 - ct / 2, w: ct, h: ct }, layer: li, ..pin.clone() });
    }
    // An n+ ring lives in an n-well; a p+ ring in the substrate has none.
    if let Some(nwell) = process.layer("nwell") {
        match well {
            WellShape::None => {}
            WellShape::Filled => {
                let enc = dim(process, "nwell_diff_enc");
                b.rect(nwell, Rect { x: ox0 - enc, y: oy0 - enc, w: (ox1 - ox0) + 2 * enc, h: (oy1 - oy0) + 2 * enc });
            }
            WellShape::Band => band_well(
                b,
                nwell,
                Rect { x: ox0, y: oy0, w: ox1 - ox0, h: oy1 - oy0 },
                Rect { x: ix0, y: iy0, w: ix1 - ix0, h: iy1 - iy0 },
                band_well_ext(process, ring_width),
            ),
        }
    }
    Rect { x: ox0, y: oy0, w: ox1 - ox0, h: oy1 - oy0 }
}

/// The placement halo: how far `r`'s ring (gap + band + outer clearance)
/// extends past its device bbox. The caller inflates the requester's variant
/// bboxes by this so the placer reserves the ring's ground.
/// Sized for the smallest ring (a point device), which needs the most rows.
#[must_use]
pub fn ring_halo(r: &GuardRingRequirement, process: &dyn Process, cut_ohm: f32) -> i32 {
    let width = band(process, r, 8 * i64::from(ring_clear(process)), cut_ohm).0;
    width + ring_gap(process, r, width) + outer_clear(r.ring_type, process, width) + ring_implant_enc(process, r.ring_type)
}

/// Device-to-band gap: the deck's diffusion/implant clearance past the
/// band's own implant growth (the ring's well merges with a ringed cell's
/// own well into one figure). An `Ecgr`'s gap is measured from its band
/// well's inner edge, not the tap band: the well grows [`band_well_ext`]
/// inward, then clears the ringed device's diffusion (difftap.9 on sky130)
/// and leaves a hole at least the well spacing wide even around a point
/// device (half per side).
fn ring_gap(process: &dyn Process, r: &GuardRingRequirement, width: i32) -> i32 {
    let ext = match r.ring_type {
        GuardRingType::Ecgr => Some(band_well_ext(process, width)),
        GuardRingType::Tub { .. } => Some(tub_well_ext(process, width)),
        _ => None,
    };
    if let Some(ext) = ext {
        // ponytail: conservative for large devices (sky130 845 vs 550); size the
        // hole from `inner` if the area matters.
        let g = ext + process.space_between("nwell", "diff").unwrap_or(0).max((nwell_space(process) + 1) / 2);
        return on_grid_up(process, g);
    }
    ring_clear(process) + ring_implant_enc(process, r.ring_type)
}

/// The deck's n-well to n-well spacing (nwell.2a, sky130 1270).
fn nwell_space(process: &dyn Process) -> i32 {
    process.rule("nwell_min_spacing", 0).max(process.space("nwell").unwrap_or(0))
}

/// How far an ECGR's band well grows past its tap band of `width`: the deck's
/// well-over-diff/tap enclosure, and enough that the band well is
/// `width("nwell")` wide (sky130 840 > 420 + 2·180), snapped up to the grid.
fn band_well_ext(process: &dyn Process, width: i32) -> i32 {
    let e = dim(process, "nwell_diff_enc").max(process.enclosure("nwell", "tap").unwrap_or(0)).max((dim(process, "nwell_min_width") - width + 1) / 2);
    on_grid_up(process, e)
}

/// A tub's band well: [`band_well_ext`], and wide enough to hold the deep
/// n-well's reach past the hole (nwell.6) plus the well's past the deep well
/// (nwell.5): sky130 width 420 → 505.
fn tub_well_ext(process: &dyn Process, width: i32) -> i32 {
    let reach = process.enclosure("dnwell", "nwell").unwrap_or(0) + process.enclosure("nwell", "dnwell").unwrap_or(0);
    band_well_ext(process, width).max(on_grid_up(process, (reach - width + 1) / 2))
}

/// `v` rounded up (toward +∞) to the manufacturing grid; negative `v`
/// included (a reach that a band already covers).
fn on_grid_up(process: &dyn Process, v: i32) -> i32 {
    let g = process.grid().max(1);
    (v + g - 1).div_euclid(g) * g
}

/// How far a ring's implant grows past its tap (see [`tap_ring`]).
fn ring_implant_enc(process: &dyn Process, ring_type: GuardRingType) -> i32 {
    process.enclosure(implant_name(ring_type), "tap").unwrap_or(0)
}

/// The ring band's implant. A `Tap` ring is its device's bulk tap: n+ in the
/// n-well for a PMOS (`in_well`), p+ in the substrate for an NMOS. `Ecgr` is
/// n+ in its own well, drawn as a band ([`WellShape::Band`]); `Hcgr` is p+.
fn implant_name(ring_type: GuardRingType) -> &'static str {
    use GuardRingType::{Ecgr, Hcgr, Tap, Tub};
    match ring_type {
        Tap { in_well: true } | Ecgr | Tub { .. } => "nsdm",
        Tap { in_well: false } | Hcgr => "psdm",
    }
}

/// Only a `Tap { in_well: true }` (a PMOS's bulk tap) sits in a filled n-well;
/// `Ecgr`'s n-well is its own band, drawn as a band ([`WellShape::Band`]), and
/// `Hcgr` needs a retrograde/isolated well, not a plain n-well.
fn in_nwell(ring_type: GuardRingType) -> bool {
    matches!(ring_type, GuardRingType::Tap { in_well: true })
}

/// The n-well a tap ring sits in: none (p+ in substrate), one rect over the
/// ring and its interior (a PMOS's bulk tap, the PMOS shares it), or a band
/// around the tap band only (an ECGR, whose interior is an NMOS).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WellShape {
    /// No n-well (p+ ring in the substrate).
    None,
    /// One n-well rect over the ring and its interior.
    Filled,
    /// An n-well band around the tap band only.
    Band,
}

/// The [`WellShape`] [`tap_ring`] draws for `t`; a `Tub`'s well is drawn by
/// [`draw_ring`] itself.
fn well_shape(t: GuardRingType) -> WellShape {
    match t {
        GuardRingType::Tap { in_well: true } => WellShape::Filled,
        // A tub's band is drawn by `draw_ring`, with its deep n-well.
        GuardRingType::Tap { in_well: false } | GuardRingType::Hcgr | GuardRingType::Tub { .. } => WellShape::None,
        GuardRingType::Ecgr => WellShape::Band,
    }
}

/// Whether this deck can draw a `t` ring as named. `Tap`: always. `Hcgr`: only
/// with a retrograde/isolated p-well the deck declares (`retrograde_pwell`;
/// sky130 false). `Tub`: with `dnwell`, `nwell`, `nsdm` and the deck's
/// hole-to-deep-well enclosure.
#[must_use]
pub fn drawable(t: GuardRingType, p: &dyn Process) -> bool {
    match t {
        GuardRingType::Tap { .. } => true,
        GuardRingType::Ecgr => p.layer("nwell").is_some() && p.layer("nsdm").is_some(),
        GuardRingType::Hcgr => p.rule("retrograde_pwell", 0) != 0,
        GuardRingType::Tub { .. } => ["dnwell", "nwell", "nsdm"].iter().all(|l| p.layer(l).is_some()) && p.enclosure("dnwell", "nwell").is_some(),
    }
}

/// Outer clearance: an in-well ring owes a neighbour's well
/// `nwell_min_spacing` beyond its own well enclosure; a well-less ring only
/// [`ring_clear`] (which already beats well-to-outside-diff, 340). An
/// `Ecgr`'s band well grows [`band_well_ext`] past a band of `width`; a
/// `Tub`'s deep n-well owes any other deep n-well the full dnwell spacing.
fn outer_clear(ring_type: GuardRingType, process: &dyn Process, width: i32) -> i32 {
    if in_nwell(ring_type) {
        nwell_space(process) + dim(process, "nwell_diff_enc").max(process.enclosure("nwell", "tap").unwrap_or(0))
    } else if ring_type == GuardRingType::Ecgr {
        nwell_space(process) + band_well_ext(process, width)
    } else if let GuardRingType::Tub { .. } = ring_type {
        // ponytail: full dnwell.3 per side (sky130 ≈ 6.4 µm halo); halve when every dnwell owner reserves its half.
        let g = tub_well_ext(process, width);
        (nwell_space(process) + g).max(g - process.enclosure("nwell", "dnwell").unwrap_or(0) + process.space("dnwell").unwrap_or(0))
    } else {
        ring_clear(process)
    }
}

/// The deck's densest legal tap-contact pitch: more cuts, lower ring R.
pub(crate) fn cut_pitch(process: &dyn Process) -> i32 {
    let lat = cut_lattice(process);
    // At least a cut plus the deck's spacing for cuts in an array (a ring
    // row is one).
    let ct = dim(process, "contact");
    let floor = ct + process.space("licon").unwrap_or(0);
    snap_cut(process.rule("guard_licon_pitch", 0).max(floor) + lat - 1, lat).max(lat)
}

/// `(band width, cut rows)` for a ring whose inner loop is `perimeter` nm.
/// Thin: the deck's minimum ring width, one row of cuts at the densest pitch
/// (Charbon §8.6.2, PDF pp.146–147: minimum-width rings with a low-impedance
/// contact beat wide ones). Rows are added only while the cuts in parallel,
/// `cut_ohm / cuts`, exceed `max_ring_resistance_mohm`.
///
/// ponytail: ring R is the parallel cut resistance only; tap spreading and
/// the li run to the tie point are left out. Extract the ring if a budget is
/// ever tight.
fn band(process: &dyn Process, r: &GuardRingRequirement, perimeter: i64, cut_ohm: f32) -> (i32, i32) {
    let ct = dim(process, "contact");
    let pitch = cut_pitch(process);
    // Wide enough to enclose its cut row in diffusion on both sides.
    let floor = process.rule("min_guard_ring_width", 0).max(r.min_width_nm).max(ct + 2 * process.rule("diff_encloses_licon", 0));
    let per_row = (perimeter / i64::from(pitch)).max(1) as f32;
    let budget = r.max_ring_resistance_mohm as f32 / 1000.0;
    let mut rows = 1;
    while rows < 4 && budget > 0.0 && cut_ohm / (per_row * rows as f32) > budget {
        rows += 1;
    }
    (floor.max(ct) + (rows - 1) * pitch, rows)
}

/// A drawn ring's resistance to its net: its `licon` cuts in parallel, ohms.
/// A ring with no cut (or a deck without `licon`) counts as one cut.
#[must_use]
pub fn ring_ohm(ring: &Macro, process: &dyn Process, cut_ohm: f32) -> f32 {
    let cuts = process.layer("licon").map_or(0, |l| ring.shapes.iter().filter(|s| s.layer == l).count());
    cut_ohm / cuts.max(1) as f32
}

/// Substrate tags of one placed cell, OR over its members (EXT-23): `injector` = a
/// `MinorityElectron`/`MinorityHole` aggressor, `noisy` = any aggressor, `sensitive` = a victim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellFlags {
    /// A member injects minority carriers.
    pub injector: bool,
    /// A member is any substrate aggressor.
    pub noisy: bool,
    /// A member is a substrate victim.
    pub sensitive: bool,
}

/// Hastings §14.1.5 (L43595–43612): two cells may share a well only if both inject or neither does,
/// and neither is noisy beside a sensitive one. §14.1.4: output (injecting) devices get their own.
/// The predicate only; CELL-22 applies it in [`well_bridges`].
#[must_use]
pub fn may_share_well(a: CellFlags, b: CellFlags) -> bool {
    a.injector == b.injector && !((a.noisy && b.sensitive) || (b.noisy && a.sensitive))
}

/// One well for neighbouring PMOS cells on the same bulk: where two placed
/// cells' nwells face each other across a gap up to twice the well spacing,
/// with overlapping spans (at least the well width) on the other axis, the gap
/// is filled. The fill spans both wells (their hull on that axis) when that
/// stays clear of foreign active and of other wells by the well spacing, else
/// only the overlap; for identical spans the two are one rectangle. The caller
/// merges the touching rects for a checker that reads a well per rect. Fewer
/// well edges near the devices (WPE); matched PMOS in a common well (Hastings
/// §13.3 r8). A fill that would come within the deck's enclosure/spacing of
/// anything p-type or of foreign active (another cell's or a ring's diff, poly
/// or p-implant) is not drawn.
///
/// `placed`: the cells, placed (absolute). `rings`: this layout's rings.
/// `can_share(i, j)`: REL-16's legality for placed cells `i`, `j`
/// ([`may_share_well`] over their tags). The check is applied per merged
/// well: a pair is bridged only if every cell already in `i`'s well may share
/// with every cell in `j`'s, so A–B and B–C bridges never join a forbidden A–C.
///
/// Returns at most one macro holding every fill; empty without an `nwell`
/// layer or a positive `nwell_min_spacing`.
///
/// # Time complexity
/// *O*(n² · S) for `n` cells and `S` shapes scanned per clearance check,
/// plus *O*(n²) per newly merged well.
#[must_use]
pub fn well_bridges(placed: &[Macro], rings: &[Macro], process: &dyn Process, can_share: &dyn Fn(usize, usize) -> bool) -> Vec<Macro> {
    let Some(nwell) = process.layer("nwell") else { return Vec::new() };
    // All from the deck; without a well spacing there is nothing to bridge.
    let space = process.rule("nwell_min_spacing", 0);
    let min_w = dim(process, "nwell_min_width");
    let keep = dim(process, "nwell_diff_enc");
    if space <= 0 {
        return Vec::new();
    }
    let blocking: Vec<pnr_core::LayerId> = ["diff", "poly", "psdm"].iter().filter_map(|r| process.layer(r)).collect();
    // Per cell: its well's bbox and its bulk net.
    let wells: Vec<Option<(Rect, pnr_core::NetId)>> = placed
        .iter()
        .map(|m| {
            let well = m.shapes.iter().filter(|s| s.layer == nwell).map(|s| s.rect).reduce(union_rect)?;
            let bulk = m.pins.iter().find(|p| p.name.ends_with(":B"))?.net;
            Some((well, bulk))
        })
        .collect();
    // Any shape on `layers` of a cell outside `skip`, or of a ring, within `margin` of `r`.
    let near = |r: Rect, skip: [usize; 2], margin: i32, layers: &[pnr_core::LayerId]| {
        let grown = Rect { x: r.x - margin, y: r.y - margin, w: r.w + 2 * margin, h: r.h + 2 * margin };
        let meets = |a: &Rect| a.x < grown.x + grown.w && grown.x < a.x + a.w && a.y < grown.y + grown.h && grown.y < a.y + a.h;
        placed.iter().enumerate().filter(|(k, _)| !skip.contains(k)).map(|(_, m)| m).chain(rings).flat_map(|m| &m.shapes).any(|s| layers.contains(&s.layer) && meets(&s.rect))
    };
    let n = placed.len();
    // Merged wells so far (union-find over cells).
    let mut comp: Vec<usize> = (0..n).collect();
    let mut b = Builder::new(process.grid());
    let mut any = false;
    for i in 0..n {
        for j in i + 1..n {
            let (Some((a, na)), Some((c, nc))) = (wells[i], wells[j]) else { continue };
            if na != nc {
                continue;
            }
            // Facing across x with overlapping y spans, or across y with overlapping x spans:
            // (overlap, hull) fills.
            let (lo, hi) = if a.x <= c.x { (a, c) } else { (c, a) };
            let (lo_y, hi_y) = if a.y <= c.y { (a, c) } else { (c, a) };
            let gap_x = hi.x - (lo.x + lo.w);
            let gap_y = hi_y.y - (lo_y.y + lo_y.h);
            let (y0, y1) = (a.y.max(c.y), (a.y + a.h).min(c.y + c.h));
            let (x0, x1) = (a.x.max(c.x), (a.x + a.w).min(c.x + c.w));
            let fills = if gap_x > 0 && gap_x <= 2 * space && y1 - y0 >= min_w {
                let (h0, h1) = (a.y.min(c.y), (a.y + a.h).max(c.y + c.h));
                Some((Rect { x: lo.x + lo.w, y: y0, w: gap_x, h: y1 - y0 }, Rect { x: lo.x + lo.w, y: h0, w: gap_x, h: h1 - h0 }))
            } else if gap_y > 0 && gap_y <= 2 * space && x1 - x0 >= min_w {
                let (h0, h1) = (a.x.min(c.x), (a.x + a.w).max(c.x + c.w));
                Some((Rect { x: x0, y: lo_y.y + lo_y.h, w: x1 - x0, h: gap_y }, Rect { x: h0, y: lo_y.y + lo_y.h, w: h1 - h0, h: gap_y }))
            } else {
                None
            };
            let Some((overlap, hull)) = fills else { continue };
            // Only the hull leaves the facing wells' common span, so only it can approach a third well.
            let bridge = if !near(hull, [i, j], keep, &blocking) && !near(hull, [i, j], space, &[nwell]) {
                hull
            } else if !near(overlap, [i, j], keep, &blocking) {
                overlap
            } else {
                continue;
            };
            let (ri, rj) = (find(&mut comp, i), find(&mut comp, j));
            if ri != rj {
                // ponytail: O(n²) per pair; wells per layout are few.
                let roots: Vec<usize> = (0..n).map(|p| find(&mut comp, p)).collect();
                let legal = (0..n).filter(|&p| roots[p] == ri).all(|p| (0..n).filter(|&q| roots[q] == rj).all(|q| can_share(p, q)));
                if !legal {
                    continue;
                }
                comp[ri] = rj;
            }
            b.rect(nwell, bridge);
            any = true;
        }
    }
    if any { vec![b.finish()] } else { Vec::new() }
}

/// Same-type implants of neighbouring cells (and rings) closer than the
/// implant's spacing merge into one: the gap between two facing implant
/// rects is filled, the usual cure for an implant spacing a cell cannot see
/// alone. A fill narrower than the implant's width, or overlapping the
/// opposite implant (it would re-dope a diffusion), is not drawn. Only
/// pairs sharing the exact span on the other axis are bridged.
///
/// Returns at most one macro holding every fill; empty when the deck lacks
/// `nsdm` or `psdm`.
///
/// # Time complexity
/// *O*(k²) pairs for `k` implant rects of one type, each candidate checked
/// against every rect.
#[must_use]
pub fn implant_bridges(all: &[Macro], process: &dyn Process) -> Vec<Macro> {
    let (Some(n), Some(p)) = (process.layer("nsdm"), process.layer("psdm")) else { return Vec::new() };
    let mut b = Builder::new(process.grid());
    let mut any = false;
    for (role, own, other) in [("nsdm", n, p), ("psdm", p, n)] {
        let (Some(space), wmin) = (process.space(role), process.width(role).unwrap_or(0)) else { continue };
        let of = |l| all.iter().flat_map(|m| &m.shapes).filter(|s| s.layer == l).map(|s| s.rect).collect::<Vec<_>>();
        let (mine, theirs) = (of(own), of(other));
        let overlaps = |a: &Rect, c: &Rect| a.x < c.x + c.w && c.x < a.x + a.w && a.y < c.y + c.h && c.y < a.y + a.h;
        for i in 0..mine.len() {
            for j in i + 1..mine.len() {
                let (a, c) = (mine[i], mine[j]);
                let gap_x = (c.x - (a.x + a.w)).max(a.x - (c.x + c.w));
                let gap_y = (c.y - (a.y + a.h)).max(a.y - (c.y + c.h));
                let (y0, y1) = (a.y.max(c.y), (a.y + a.h).min(c.y + c.h));
                let (x0, x1) = (a.x.max(c.x), (a.x + a.w).min(c.x + c.w));
                // Only where the two share a span, so the union is one clean
                // rectangle (a partial span leaves notches).
                let bridge = if gap_x > 0 && gap_x < space && (a.y, a.h) == (c.y, c.h) && y1 - y0 >= wmin {
                    Some(Rect { x: a.x.min(c.x) + if a.x < c.x { a.w } else { c.w }, y: y0, w: gap_x, h: y1 - y0 })
                } else if gap_y > 0 && gap_y < space && (a.x, a.w) == (c.x, c.w) && x1 - x0 >= wmin {
                    Some(Rect { x: x0, y: a.y.min(c.y) + if a.y < c.y { a.h } else { c.h }, w: x1 - x0, h: gap_y })
                } else {
                    None
                };
                let inside = |r: &Rect| mine.iter().any(|m| m.x <= r.x && m.y <= r.y && m.x + m.w >= r.x + r.w && m.y + m.h >= r.y + r.h);
                if let Some(r) = bridge.filter(|r| !theirs.iter().any(|t| overlaps(r, t)) && !inside(r)) {
                    b.rect(own, r);
                    any = true;
                }
            }
        }
    }
    if any { vec![b.finish()] } else { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two PMOS cells on one bulk, their wells 1.5 µm apart (under the
    /// spacing): bridged into one well, DRC/ERC clean. On different bulks, or
    /// with p-active in the gap, no bridge.
    #[test]
    fn same_bulk_pmos_cells_share_a_well() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let nwell = pdk.layer("nwell").unwrap();
        let (cell, well) = pmos(&pdk);
        let dx = well.w + 1_500;
        let pair = [shift(&cell, 0, 0, 1), shift(&cell, dx, 0, 1)];
        let bridges = well_bridges(&pair, &[], &pdk, &|_, _| true);
        assert_eq!(bridges.len(), 1);
        // Merged as the flow's geometry does: the three rects are one well.
        let mut shapes: Vec<pnr_core::Shape> = pair.iter().chain(&bridges).flat_map(|m| m.shapes.clone()).collect();
        let (mut wells, rest): (Vec<pnr_core::Shape>, Vec<pnr_core::Shape>) = shapes.drain(..).partition(|s| s.layer == nwell);
        let r = wells.iter().map(|s| s.rect).reduce(union_rect).unwrap();
        assert_eq!(wells.iter().map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum::<i64>(), i64::from(r.w) * i64::from(r.h), "the rects tile one rectangle");
        wells = vec![pnr_core::Shape { layer: nwell, rect: r }];
        shapes = rest.into_iter().chain(wells).collect();
        let dirty = findings_of(&pair, shapes, &pdk);
        assert!(dirty.is_empty(), "{dirty:?}");
        assert!(well_bridges(&[shift(&cell, 0, 0, 1), shift(&cell, dx, 0, 2)], &[], &pdk, &|_, _| true).is_empty(), "different bulks");
        let mut blocker = shift(&cell, 0, 0, 3);
        blocker.shapes.retain(|s| Some(s.layer) == pdk.layer("diff"));
        for s in &mut blocker.shapes {
            s.rect = Rect { x: well.x + well.w + 500, y: well.y + well.h / 2, w: 400, h: 400 };
        }
        assert!(well_bridges(&[pair[0].clone(), pair[1].clone(), blocker], &[], &pdk, &|_, _| true).is_empty(), "active in the gap");
    }

    /// One drawn PMOS cell and its well's bbox.
    fn pmos(pdk: &verify::Pdk) -> (Macro, Rect) {
        use crate::{testkit, Cell};
        let nwell = pdk.layer("nwell").unwrap();
        let (g, c) = testkit::group_of(pnr_core::DeviceKind::Pmos, 1, 2, 1680, 150);
        let cell = crate::mosfet::Mosfet::enumerate(&g, &c, pdk)[0].draw(&g, &c, pdk);
        let well = cell.shapes.iter().filter(|s| s.layer == nwell).map(|s| s.rect).reduce(union_rect).unwrap();
        (cell, well)
    }

    /// `m` moved by (`dx`, `dy`), its bulk on net `bulk` and its other pins on nets of their own.
    fn shift(m: &Macro, dx: i32, dy: i32, bulk: u16) -> Macro {
        use pnr_core::NetId;
        let mut m = m.clone();
        for s in &mut m.shapes {
            s.rect.x += dx;
            s.rect.y += dy;
        }
        for p in &mut m.pins {
            p.at.x += dx;
            p.at.y += dy;
            p.net = if p.name.ends_with(":B") { NetId(bulk) } else { NetId(10 + bulk + p.net.0) };
        }
        m.bbox.x += dx;
        m.bbox.y += dy;
        m
    }

    /// DRC/ERC findings on `shapes`, labelled by `cells`' pins (bulks as `B`, the rest per cell).
    fn findings_of(cells: &[Macro], shapes: Vec<pnr_core::Shape>, pdk: &verify::Pdk) -> Vec<String> {
        let labels: Vec<verify::LabeledPin> = cells
            .iter()
            .enumerate()
            .flat_map(|(k, m)| m.pins.iter().map(move |p| (k, p)))
            .map(|(k, p)| verify::LabeledPin { name: if p.name.ends_with(":B") { "B".into() } else { format!("c{k}_{}", p.name.replace(':', "_")) }, layer: p.layer.0, x: p.at.x + p.at.w / 2, y: p.at.y + p.at.h / 2 })
            .fold(Vec::new(), |mut v: Vec<verify::LabeledPin>, l| {
                if !v.iter().any(|o| (o.x, o.y) == (l.x, l.y)) {
                    v.push(l);
                }
                v
            });
        crate::testkit::findings(&shapes, &labels, pdk)
    }

    /// CELL-22: wells offset by a quarter of their height bridge over their
    /// hull when the gap is clear, over the overlap when p-active sits beside
    /// the overlap or the hull would come within the well spacing of a third
    /// well; both DRC/ERC clean with the touching rects left unmerged.
    #[test]
    fn offset_wells_bridge_over_their_overlap() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let nwell = pdk.layer("nwell").unwrap();
        let (cell, well) = pmos(&pdk);
        // On the grid: the builder snaps what it draws.
        let (dx, dy) = (well.w + 600, well.h / 4 / pdk.grid() * pdk.grid());
        let pair = [shift(&cell, 0, 0, 1), shift(&cell, dx, dy, 1)];
        let bridges = well_bridges(&pair, &[], &pdk, &|_, _| true);
        assert_eq!(bridges.len(), 1);
        let fill = rects_on(&bridges[0], Some(nwell));
        assert_eq!(fill, vec![Rect { x: well.x + well.w, y: well.y, w: 600, h: well.h + dy }], "the hull");
        let shapes: Vec<pnr_core::Shape> = pair.iter().chain(&bridges).flat_map(|m| m.shapes.clone()).collect();
        let dirty = findings_of(&pair, shapes, &pdk);
        assert!(dirty.is_empty(), "hull: {dirty:?}");

        let mut blocker = shift(&cell, 0, 0, 3);
        blocker.shapes.retain(|s| Some(s.layer) == pdk.layer("diff"));
        blocker.shapes.truncate(1);
        blocker.shapes[0].rect = Rect { x: well.x + well.w + 100, y: well.y + well.h + dy - 300, w: 400, h: 300 };
        let three = [pair[0].clone(), pair[1].clone(), blocker];
        let bridges = well_bridges(&three, &[], &pdk, &|_, _| true);
        assert_eq!(bridges.len(), 1);
        assert_eq!(rects_on(&bridges[0], Some(nwell)), vec![Rect { x: well.x + well.w, y: well.y + dy, w: 600, h: well.h - dy }], "the overlap");
        let shapes: Vec<pnr_core::Shape> = pair.iter().chain(&bridges).flat_map(|m| m.shapes.clone()).collect();
        let dirty = findings_of(&pair, shapes, &pdk);
        assert!(dirty.is_empty(), "overlap: {dirty:?}");

        // A third well (another bulk) above the gap, within the well spacing of
        // the hull's top but not of the overlap's: the fill falls back to the overlap.
        let space = pdk.rule("nwell_min_spacing", 0);
        let mut other = shift(&cell, 0, 0, 2);
        other.shapes.retain(|s| s.layer == nwell);
        other.shapes.truncate(1);
        other.shapes[0].rect = Rect { x: well.x + well.w, y: well.y + well.h + dy + space - pdk.grid(), w: 600, h: well.w };
        let three = [pair[0].clone(), pair[1].clone(), other];
        let bridges = well_bridges(&three, &[], &pdk, &|_, _| true);
        assert_eq!(bridges.len(), 1);
        assert_eq!(rects_on(&bridges[0], Some(nwell)), vec![Rect { x: well.x + well.w, y: well.y + dy, w: 600, h: well.h - dy }], "near a third well: the overlap");
    }

    /// CELL-22 (REL-16's acceptance): an injector's well is never bridged to a
    /// non-injector's, directly or through a third cell's well.
    #[test]
    fn a_forbidden_pair_is_not_bridged() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let nwell = pdk.layer("nwell").unwrap();
        let (cell, well) = pmos(&pdk);
        let dx = well.w + 1_500;
        let pair = [shift(&cell, 0, 0, 1), shift(&cell, dx, 0, 1)];
        let flags = [CellFlags { injector: true, ..Default::default() }, CellFlags::default()];
        assert!(well_bridges(&pair, &[], &pdk, &|i, j| may_share_well(flags[i], flags[j])).is_empty());
        let flags = [CellFlags::default(); 2];
        assert_eq!(well_bridges(&pair, &[], &pdk, &|i, j| may_share_well(flags[i], flags[j])).len(), 1);
        let row = [shift(&cell, 0, 0, 1), shift(&cell, dx, 0, 1), shift(&cell, 2 * dx, 0, 1)];
        let bridges = well_bridges(&row, &[], &pdk, &|i, j| !matches!((i, j), (0, 2) | (2, 0)));
        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].shapes.iter().filter(|s| s.layer == nwell).count(), 1, "A–B bridged, B–C would join A to C");
        let all = well_bridges(&row, &[], &pdk, &|_, _| true);
        assert_eq!(all[0].shapes.iter().filter(|s| s.layer == nwell).count(), 2, "control: both bridged when legal");
    }

    fn req(dev: u16, net: u16, ty: GuardRingType, shareable: bool) -> GuardRingRequirement {
        GuardRingRequirement {
            device: DeviceId(dev),
            ring_type: ty,
            shareable,
            min_width_nm: 420,
            max_ring_resistance_mohm: 10_000,
            connection_net: pnr_core::NetId(net),
            role: analog::cell::RingRole::Aggressor,
        }
    }

    /// A layout of unit 100nm-half-extent devices at the given centres.
    fn layout_at(centres: &[(i32, i32)]) -> Layout {
        Layout {
            x: centres.iter().map(|c| c.0).collect(),
            y: centres.iter().map(|c| c.1).collect(),
            hw: vec![100; centres.len()],
            hh: vec![100; centres.len()],
            // `0` = "the first alternative", the correct no-variant-search state; guard
            // rings do not read it, but the `hw`/`hh` invariant wants one entry per device.
            variant: vec![0; centres.len()],
            // No disjunctive pair is emitted here, so the branch table is empty rather
            // than device-length — it is indexed by `BranchId`, not `DeviceId`.
            branch: Vec::new(),
            axis: Vec::new(),
            groups: Vec::new(),
            orient: vec![pnr_core::Orient::default(); centres.len()],
            power_uw: vec![0; centres.len()],
            temp_mc: vec![0; centres.len()],
            units: Default::default(),
        }
    }

    /// Every annotator ring still draws byte-identically: `draw_ring`/
    /// `ring_halo` read the type only through these two and `ring_gap`, whose
    /// `Ecgr` arm no annotator ring hits (the annotator only ever emits
    /// `Tap`, the old `Hcgr`/`Ecgr` values).
    #[test]
    fn tap_rings_draw_as_before() {
        use GuardRingType::{Ecgr, Hcgr, Tap};
        assert_eq!(implant_name(Tap { in_well: true }), "nsdm");
        assert!(in_nwell(Tap { in_well: true }));
        assert_eq!(implant_name(Tap { in_well: false }), "psdm");
        assert!(!in_nwell(Tap { in_well: false }));
        assert_eq!(implant_name(Ecgr), "nsdm");
        assert!(!in_nwell(Ecgr));
        assert_eq!(implant_name(Hcgr), "psdm");
    }

    #[test]
    fn hcgr_is_not_drawable_on_sky130() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        assert!(!drawable(GuardRingType::Hcgr, &pdk));
        assert!(drawable(GuardRingType::Ecgr, &pdk));
        assert!(drawable(GuardRingType::Tap { in_well: true }, &pdk));
    }

    #[test]
    fn ecgr_ring_gap_clears_the_well() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let Some(space) = pdk.space_between("nwell", "diff") else {
            panic!(
                "pdk.space_between(\"nwell\", \"diff\") is None — the deck states difftap.9 \
                 on derived `ndiff`, and `widest_on` may not reach it"
            );
        };
        assert_eq!(space, 340);
        assert!(ring_gap(&pdk, &req(0, 5, GuardRingType::Ecgr, true), 0) >= 340);
        // sky130's base gap already clears 340, so the well term is checked
        // on a deck whose clearance (100 + 50) is under the 400 well spacing.
        struct Narrow;
        impl Process for Narrow {
            fn layer(&self, _: &str) -> Option<pnr_core::LayerId> {
                None
            }
            fn rule(&self, _: &str, default: i32) -> i32 {
                default
            }
            fn grid(&self) -> i32 {
                5
            }
            fn space(&self, _: &str) -> Option<i32> {
                Some(100)
            }
            fn enclosure(&self, _: &str, _: &str) -> Option<i32> {
                Some(50)
            }
            fn space_between(&self, _: &str, _: &str) -> Option<i32> {
                Some(400)
            }
        }
        assert_eq!(ring_gap(&Narrow, &req(0, 5, GuardRingType::Tap { in_well: false }, true), 0), 150);
        // The gap runs from the band well's inner edge: g 50 + max(400, 50).
        assert_eq!(ring_gap(&Narrow, &req(0, 5, GuardRingType::Ecgr, true), 0), 450);
        assert!(ring_gap(&pdk, &req(0, 5, GuardRingType::Ecgr, true), 420) - band_well_ext(&pdk, 420) >= 340);
    }

    /// An NMOS (2 µm × 2 µm-ish) inside an `Ecgr`: the cell, the ring macro
    /// and the ring's interior.
    fn ecgr_around_nmos(pdk: &verify::Pdk) -> (Macro, Macro, Rect) {
        use crate::{testkit, Cell};
        let (g, c) = testkit::group_of(pnr_core::DeviceKind::Nmos, 1, 2, 1000, 150);
        let cell = crate::mosfet::Mosfet::enumerate(&g, &c, pdk)[0].draw(&g, &c, pdk);
        let inner = cell.bbox;
        let mut b = Builder::new(pdk.grid());
        draw_ring(&mut b, pdk, &req(0, 9, GuardRingType::Ecgr, false), inner, 15.0);
        (cell, b.finish(), inner)
    }

    fn rects_on(m: &Macro, layer: Option<pnr_core::LayerId>) -> Vec<Rect> {
        m.shapes.iter().filter(|s| Some(s.layer) == layer).map(|s| s.rect).collect()
    }

    fn inside(a: Rect, b: Rect) -> bool {
        a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h
    }

    /// CELL-17: the ECGR's well is a band, clear of the NMOS's diffusion by
    /// difftap.9, at least the well width wide, and the whole is DRC/ERC clean.
    #[test]
    fn an_ecgr_well_never_covers_its_interior() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let (cell, ring, inner) = ecgr_around_nmos(&pdk);
        let s = pdk.space_between("nwell", "diff").unwrap();
        let keep_out = Rect { x: inner.x - s, y: inner.y - s, w: inner.w + 2 * s, h: inner.h + 2 * s };
        let wells = rects_on(&ring, pdk.layer("nwell"));
        assert_eq!(wells.len(), 4);
        for w in &wells {
            let hits = w.x < keep_out.x + keep_out.w && keep_out.x < w.x + w.w && w.y < keep_out.y + keep_out.h && keep_out.y < w.y + w.h;
            assert!(!hits, "{w:?} reaches within {s} of {inner:?}");
            assert!(w.w.min(w.h) >= dim(&pdk, "nwell_min_width"), "{w:?}");
        }
        let shapes: Vec<pnr_core::Shape> = cell.shapes.iter().chain(&ring.shapes).cloned().collect();
        let mut labels = testkit::ports_with(&cell, &["G", "S", "B"]);
        labels.extend(ring.pins.iter().map(|p| verify::LabeledPin { name: "VDD".into(), layer: p.layer.0, x: p.at.x + p.at.w / 2, y: p.at.y + p.at.h / 2 }));
        let dirty = testkit::findings(&shapes, &labels, &pdk);
        assert!(dirty.is_empty(), "{dirty:?}");
    }

    /// GAP-05's spec: an ECGR is n+ tap in its own n-well, no p+.
    #[test]
    fn ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let (_, ring, _) = ecgr_around_nmos(&pdk);
        assert!(drawable(GuardRingType::Ecgr, &pdk));
        let (nsdm, nwell) = (rects_on(&ring, pdk.layer("nsdm")), rects_on(&ring, pdk.layer("nwell")));
        let taps = rects_on(&ring, pdk.layer("tap"));
        assert_eq!(taps.len(), 4);
        for t in taps {
            assert!(nsdm.iter().any(|&r| inside(t, r)), "{t:?} outside nsdm");
            assert!(nwell.iter().any(|&r| inside(t, r)), "{t:?} outside nwell");
        }
        assert!(rects_on(&ring, pdk.layer("psdm")).is_empty());
    }

    /// An NMOS inside a GAP-14 tub (`ecgr_around_nmos` with `Tub { id: 0 }`) or a substrate tap ring:
    /// the cell, the ring macro, the ring's interior, and the labels (G/S/B, the ring as `VDDQ`).
    fn ringed_nmos(pdk: &verify::Pdk, ty: GuardRingType) -> (Macro, Macro, Rect, Vec<verify::LabeledPin>) {
        use crate::{testkit, Cell};
        let (g, c) = testkit::group_of(pnr_core::DeviceKind::Nmos, 1, 2, 1000, 150);
        let cell = crate::mosfet::Mosfet::enumerate(&g, &c, pdk)[0].draw(&g, &c, pdk);
        let inner = cell.bbox;
        let mut b = Builder::new(pdk.grid());
        draw_ring(&mut b, pdk, &req(0, 9, ty, true), inner, 15.0);
        let ring = b.finish();
        let mut labels = testkit::ports_with(&cell, &["G", "S", "B"]);
        labels.extend(ring.pins.iter().map(|p| verify::LabeledPin { name: "VDDQ".into(), layer: p.layer.0, x: p.at.x + p.at.w / 2, y: p.at.y + p.at.h / 2 }));
        (cell, ring, inner, labels)
    }

    /// GAP-14: one deep n-well strictly around the cell, a four-rect band
    /// well clear of the NMOS's diffusion, DRC/ERC clean.
    #[test]
    fn a_tub_is_drc_clean_on_sky130() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        assert!(drawable(GuardRingType::Tub { id: 0 }, &pdk));
        let (cell, ring, inner, labels) = ringed_nmos(&pdk, GuardRingType::Tub { id: 0 });
        let dn = rects_on(&ring, pdk.layer("dnwell"));
        assert_eq!(dn.len(), 1);
        let d = dn[0];
        assert!(d.x < inner.x && d.y < inner.y && d.x + d.w > inner.x + inner.w && d.y + d.h > inner.y + inner.h, "{d:?} vs {inner:?}");
        let s = pdk.space_between("nwell", "diff").unwrap();
        let keep_out = Rect { x: inner.x - s, y: inner.y - s, w: inner.w + 2 * s, h: inner.h + 2 * s };
        let wells = rects_on(&ring, pdk.layer("nwell"));
        assert_eq!(wells.len(), 4);
        for w in &wells {
            let hits = w.x < keep_out.x + keep_out.w && keep_out.x < w.x + w.w && w.y < keep_out.y + keep_out.h && keep_out.y < w.y + w.h;
            assert!(!hits, "{w:?} reaches within {s} of {inner:?}");
        }
        let shapes: Vec<pnr_core::Shape> = cell.shapes.iter().chain(&ring.shapes).cloned().collect();
        let dirty = testkit::findings(&shapes, &labels, &pdk);
        assert!(dirty.is_empty(), "{dirty:?}");
    }

    /// GAP-14: inside a tub the nfet's bulk is the isolated p-well, its own
    /// net: beside a plain NMOS 30 µm away whose bulk (`SUB`) is the
    /// substrate, the two nfets extract on `B` and `SUB`. Were the tub's
    /// p-well the substrate, `B` and `SUB` would be one net (a label short).
    #[test]
    fn the_tub_bulk_is_not_the_substrate() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        // Only the bulks are labelled: a 2-finger cell's S pads join only
        // through routing, so labelling them names two nets alike.
        let (cell, ring, _, labels) = ringed_nmos(&pdk, GuardRingType::Tub { id: 0 });
        let plain = shift(&cell, 30_000, 0, 0);
        let mut labels: Vec<_> = labels.into_iter().filter(|l| l.name == "B" || l.name == "VDDQ").collect();
        let subs: Vec<_> = labels.iter().filter(|l| l.name == "B").map(|l| verify::LabeledPin { name: "SUB".into(), x: l.x + 30_000, ..l.clone() }).collect();
        labels.extend(subs);
        let shapes: Vec<pnr_core::Shape> = cell.shapes.iter().chain(&ring.shapes).chain(&plain.shapes).cloned().collect();
        let spice = verify::extract_spice(&shapes, &labels, &pdk, verify::Detail::Schematic).expect("extracts");
        let mut bulks: Vec<&str> = spice.lines().filter(|l| l.to_ascii_lowercase().contains("nfet")).filter_map(|l| l.split_whitespace().nth(4)).collect();
        bulks.sort_unstable();
        bulks.dedup();
        assert_eq!(bulks, ["B", "SUB"], "{spice}");
    }

    /// GAP-14: the halo keeps a foreign deep n-well dnwell.3 off the tub's,
    /// and two declared tubs never share a ring.
    #[test]
    fn a_tub_halo_clears_dnwell_spacing() {
        use crate::testkit;
        let Some(pdk) = testkit::pdk() else { return };
        let r = req(0, 9, GuardRingType::Tub { id: 0 }, true);
        let width = band(&pdk, &r, 8 * i64::from(ring_clear(&pdk)), 15.0).0;
        let g = tub_well_ext(&pdk, width);
        let dn_past_inner = ring_gap(&pdk, &r, width) + width + g - pdk.enclosure("nwell", "dnwell").unwrap();
        assert!(ring_halo(&r, &pdk, 15.0) - dn_past_inner >= pdk.space("dnwell").unwrap());
        let l = layout_at(&[(0, 0), (300, 0)]);
        let reqs = [r, req(1, 9, GuardRingType::Tub { id: 1 }, true)];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        assert_eq!(clusters(&refs, &l, 2000.0).len(), 2);
        let same = [r, req(1, 9, GuardRingType::Tub { id: 0 }, true)];
        let refs: Vec<&GuardRingRequirement> = same.iter().collect();
        assert_eq!(clusters(&refs, &l, 2000.0).len(), 1, "control: one tub's members merge");
    }

    #[test]
    fn merges_adjacent_shareable_same_class() {
        use GuardRingType::Tap;
        let hcgr = Tap { in_well: true };
        let l = layout_at(&[(0, 0), (300, 0), (100_000, 0)]);
        let reqs = [
            req(0, 5, hcgr, true),
            req(1, 5, hcgr, true), // edge-gap 100nm from dev0 → merges
            req(2, 5, hcgr, true), // ~100µm away → own ring
        ];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let mut c = clusters(&refs, &l, 2000.0);
        c.sort_by_key(|m| m.len());
        assert_eq!(c.len(), 2, "adjacent pair merges, far one stays");
        assert_eq!(c[1], vec![0, 1]);
    }

    #[test]
    fn never_merges_across_class_or_private() {
        use GuardRingType::Tap;
        let (hcgr, ecgr) = (Tap { in_well: true }, Tap { in_well: false });
        let l = layout_at(&[(0, 0), (300, 0), (600, 0), (900, 0)]);
        let reqs = [
            req(0, 5, hcgr, true),
            req(1, 6, hcgr, true),  // different net → no merge
            req(2, 5, ecgr, true),  // different ring_type → no merge
            req(3, 5, hcgr, false), // private (injector) → own ring
        ];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let c = clusters(&refs, &l, 2000.0);
        assert_eq!(c.len(), 4, "class/net/private all block the merge");
    }

    /// REL-07: a victim ring never shares an aggressor's return, even on the
    /// same net and type, adjacent and shareable.
    #[test]
    fn rings_of_different_roles_never_merge() {
        let tap = GuardRingType::Tap { in_well: false };
        let l = layout_at(&[(0, 0), (300, 0)]);
        let reqs = [req(0, 5, tap, true), GuardRingRequirement { role: analog::cell::RingRole::Victim, ..req(1, 5, tap, true) }];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        assert_eq!(clusters(&refs, &l, 2000.0).len(), 2);
        let same = [req(0, 5, tap, true), req(1, 5, tap, true)];
        assert_eq!(clusters(&same.iter().collect::<Vec<_>>(), &l, 2000.0).len(), 1, "same role merges");
    }

    #[test]
    fn never_merges_across_a_foreign_cell() {
        let hcgr = GuardRingType::Tap { in_well: true };
        // Three cells in a row; only the OUTER two request rings. Their edge
        // gap (1800 nm) is within merge_gap, but the merged hull would swallow
        // the non-member middle cell — bands would be drawn straight across it.
        let l = layout_at(&[(0, 0), (1000, 0), (2000, 0)]);
        let reqs = [req(0, 5, hcgr, true), req(2, 5, hcgr, true)];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let c = clusters(&refs, &l, 2000.0);
        assert_eq!(c.len(), 2, "hull would cross the middle cell: split to singletons");
        assert_eq!(c, vec![vec![0], vec![1]]);

        // Control: the same pair with no middle cell merges.
        let l2 = layout_at(&[(0, 0), (2000, 0)]);
        let reqs2 = [req(0, 5, hcgr, true), req(1, 5, hcgr, true)];
        let refs2: Vec<&GuardRingRequirement> = reqs2.iter().collect();
        let c2 = clusters(&refs2, &l2, 2000.0);
        assert_eq!(c2, vec![vec![0, 1]], "no foreigner in the hull: pair merges");
    }

    #[test]
    fn may_share_well_table() {
        let f = |i, n, s| CellFlags { injector: i, noisy: n, sensitive: s };
        assert!(!may_share_well(f(true, false, false), f(false, false, false)), "injector beside a non-injector");
        assert!(!may_share_well(f(false, true, false), f(false, false, true)), "noisy beside sensitive");
        assert!(!may_share_well(f(false, false, true), f(false, true, false)), "mirrored");
        assert!(may_share_well(f(false, true, false), f(false, true, false)));
        assert!(may_share_well(f(true, false, false), f(true, false, false)));
        assert!(may_share_well(f(false, false, false), f(false, false, false)));
    }

    /// A deck of named layers (ids by position), rules, and per-role width,
    /// spacing and enclosure tables; grid 5.
    struct Fake {
        layers: &'static [&'static str],
        rules: &'static [(&'static str, i32)],
        widths: &'static [(&'static str, i32)],
        spaces: &'static [(&'static str, i32)],
        encs: &'static [((&'static str, &'static str), i32)],
    }

    const EMPTY: Fake = Fake { layers: &[], rules: &[], widths: &[], spaces: &[], encs: &[] };

    impl Process for Fake {
        fn layer(&self, role: &str) -> Option<pnr_core::LayerId> {
            self.layers.iter().position(|l| *l == role).map(|i| pnr_core::LayerId(i as u16 + 1))
        }
        fn rule(&self, name: &str, d: i32) -> i32 {
            self.rules.iter().find(|(k, _)| *k == name).map_or(d, |r| r.1)
        }
        fn grid(&self) -> i32 {
            5
        }
        fn width(&self, role: &str) -> Option<i32> {
            self.widths.iter().find(|(k, _)| *k == role).map(|r| r.1)
        }
        fn space(&self, role: &str) -> Option<i32> {
            self.spaces.iter().find(|(k, _)| *k == role).map(|r| r.1)
        }
        fn enclosure(&self, o: &str, i: &str) -> Option<i32> {
            self.encs.iter().find(|(k, _)| *k == (o, i)).map(|r| r.1)
        }
    }

    /// A licon-tapped ring deck: cut 170, spacing 170 (pitch 340).
    const TAPS: Fake = Fake {
        layers: &["li", "tap", "licon", "nsdm", "psdm", "nwell"],
        rules: &[],
        widths: &[("licon", 170)],
        spaces: &[("licon", 170)],
        encs: &[],
    };

    #[test]
    fn on_grid_up_is_a_ceiling() {
        for (v, want) in [(0, 0), (1, 5), (5, 5), (6, 10), (-1, 0), (-5, -5), (-7, -5)] {
            assert_eq!(on_grid_up(&EMPTY, v), want, "on_grid_up({v})");
        }
    }

    #[test]
    fn union_rect_covers_both() {
        let a = Rect { x: 0, y: 0, w: 10, h: 10 };
        let b = Rect { x: 20, y: -5, w: 5, h: 5 };
        assert_eq!(union_rect(a, b), Rect { x: 0, y: -5, w: 25, h: 15 });
        assert_eq!(union_rect(a, a), a);
    }

    #[test]
    fn union_find_joins_transitively() {
        let mut p: Vec<usize> = (0..5).collect();
        union(&mut p, 0, 1);
        union(&mut p, 3, 4);
        union(&mut p, 1, 4);
        let r = find(&mut p, 0);
        assert!([1, 3, 4].iter().all(|&i| find(&mut p, i) == r));
        assert_ne!(find(&mut p, 2), r);
        union(&mut p, 2, 2);
        assert_eq!(find(&mut p, 2), 2, "self-union is a no-op");
    }

    #[test]
    fn clusters_of_nothing_and_of_one() {
        let l = layout_at(&[(0, 0)]);
        assert!(clusters(&[], &l, 2000.0).is_empty());
        let r = req(0, 5, GuardRingType::Tap { in_well: false }, true);
        assert_eq!(clusters(&[&r], &l, 2000.0), vec![vec![0]]);
    }

    /// Edge gap exactly `merge_gap` merges (inclusive), one nm more does not.
    #[test]
    fn the_merge_gap_is_inclusive() {
        let tap = GuardRingType::Tap { in_well: false };
        let reqs = [req(0, 5, tap, true), req(1, 5, tap, true)];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        // Half-extents 100: centres 2200 apart leave a 2000 edge gap.
        assert_eq!(clusters(&refs, &layout_at(&[(0, 0), (2200, 0)]), 2000.0).len(), 1);
        assert_eq!(clusters(&refs, &layout_at(&[(0, 0), (2201, 0)]), 2000.0).len(), 2);
    }

    /// A requirement on an unplaced device draws nothing and does not panic.
    #[test]
    fn rings_of_unplaced_devices_are_skipped() {
        let l = layout_at(&[(0, 0)]);
        let c = Constraints { guard_rings: vec![req(3, 5, GuardRingType::Tap { in_well: false }, true)], ..Default::default() };
        assert!(guard_rings(&l, &c, &EMPTY, 15.0).is_empty());
        assert!(guard_rings(&l, &Constraints::default(), &EMPTY, 15.0).is_empty());
    }

    /// One ring per requirement: four tap bands, each with a `ring` pin on
    /// `li` and at least one cut, around the device.
    #[test]
    fn a_tap_ring_has_four_contacted_bands() {
        let mut l = layout_at(&[(0, 0)]);
        (l.hw[0], l.hh[0]) = (5000, 5000);
        let c = Constraints { guard_rings: vec![req(0, 5, GuardRingType::Tap { in_well: false }, true)], ..Default::default() };
        let rings = guard_rings(&l, &c, &TAPS, 15.0);
        assert_eq!(rings.len(), 1);
        let m = &rings[0];
        let on = |r: &str| m.shapes.iter().filter(|s| Some(s.layer) == TAPS.layer(r)).count();
        assert_eq!(on("tap"), 4);
        assert_eq!(on("li"), 4);
        assert_eq!(m.pins.len(), 4);
        assert!(m.pins.iter().all(|p| p.name == "ring" && Some(p.layer) == TAPS.layer("li") && p.net == pnr_core::NetId(5)));
        assert!(on("licon") >= 4);
        assert_eq!(on("nwell"), 0, "a substrate tap has no well");
    }

    /// Rows grow only while the parallel cut resistance misses the budget,
    /// never past four, and a zero budget means one row.
    #[test]
    fn band_rows_follow_the_resistance_budget() {
        let r = |mohm: i64| GuardRingRequirement { max_ring_resistance_mohm: mohm, ..req(0, 5, GuardRingType::Tap { in_well: false }, true) };
        // Perimeter 3400 at pitch 340: 10 cuts a row, 15 Ω each → 1.5 Ω a row.
        assert_eq!(cut_pitch(&TAPS), 340);
        assert_eq!(band(&TAPS, &r(10_000), 3400, 15.0), (420, 1));
        assert_eq!(band(&TAPS, &r(500), 3400, 15.0), (420 + 2 * 340, 3), "1.5/3 = 0.5 meets 0.5");
        assert_eq!(band(&TAPS, &r(1), 3400, 15.0).1, 4, "capped");
        assert_eq!(band(&TAPS, &r(0), 3400, 15.0).1, 1, "no budget");
        assert_eq!(band(&TAPS, &r(10_000), 0, 15.0).1, 2, "a point perimeter still counts one cut a row: 15 Ω > 10 Ω");
    }

    #[test]
    fn ring_ohm_is_the_cuts_in_parallel() {
        let mut b = Builder::new(5);
        let licon = TAPS.layer("licon").unwrap();
        for i in 0..4 {
            b.rect(licon, Rect { x: i * 400, y: 0, w: 170, h: 170 });
        }
        let m = b.finish();
        assert!((ring_ohm(&m, &TAPS, 20.0) - 5.0).abs() < 1e-6);
        assert!((ring_ohm(&m, &EMPTY, 20.0) - 20.0).abs() < 1e-6, "no licon layer: one cut");
    }

    #[test]
    fn drawable_needs_the_named_layers() {
        assert!(drawable(GuardRingType::Tap { in_well: true }, &EMPTY));
        assert!(!drawable(GuardRingType::Ecgr, &EMPTY));
        assert!(drawable(GuardRingType::Ecgr, &TAPS));
        assert!(!drawable(GuardRingType::Hcgr, &TAPS));
        assert!(drawable(GuardRingType::Hcgr, &Fake { rules: &[("retrograde_pwell", 1)], ..EMPTY }));
        let tub = Fake { layers: &["dnwell", "nwell", "nsdm"], ..EMPTY };
        assert!(!drawable(GuardRingType::Tub { id: 0 }, &tub), "no hole enclosure");
        assert!(drawable(GuardRingType::Tub { id: 0 }, &Fake { encs: &[(("dnwell", "nwell"), 400)], ..tub }));
    }

    /// Sharing is symmetric over every flag combination.
    #[test]
    fn may_share_well_is_symmetric() {
        let f = |b: u8| CellFlags { injector: b & 1 != 0, noisy: b & 2 != 0, sensitive: b & 4 != 0 };
        for a in 0..8 {
            assert!(may_share_well(f(a), f(a)) || (f(a).noisy && f(a).sensitive), "{a}: a cell shares with itself");
            for b in 0..8 {
                assert_eq!(may_share_well(f(a), f(b)), may_share_well(f(b), f(a)), "{a} {b}");
            }
        }
    }

    /// A well deck: nwell spacing 1270, width 840.
    const WELLS: Fake = Fake {
        layers: &["nwell", "diff", "poly", "psdm"],
        rules: &[("nwell_min_spacing", 1270)],
        widths: &[("nwell", 840)],
        spaces: &[],
        encs: &[],
    };

    /// A cell holding one n-well rect and a bulk pin on `bulk`.
    fn well_cell(r: Rect, bulk: u16) -> Macro {
        let nwell = WELLS.layer("nwell").unwrap();
        let mut b = Builder::new(5);
        b.rect(nwell, r);
        b.pin(Pin { name: "d0:B".into(), net: pnr_core::NetId(bulk), at: Rect { x: r.x, y: r.y, w: 10, h: 10 }, layer: nwell });
        b.finish()
    }

    #[test]
    fn well_bridges_need_a_well_layer_and_spacing() {
        let cells = [well_cell(Rect { x: 0, y: 0, w: 2000, h: 2000 }, 1), well_cell(Rect { x: 3000, y: 0, w: 2000, h: 2000 }, 1)];
        assert!(well_bridges(&cells, &[], &EMPTY, &|_, _| true).is_empty(), "no nwell");
        assert!(well_bridges(&cells, &[], &Fake { rules: &[], ..WELLS }, &|_, _| true).is_empty(), "no spacing");
        assert!(well_bridges(&[], &[], &WELLS, &|_, _| true).is_empty(), "no cells");
    }

    /// Facing wells with equal spans: the bridge is exactly the gap, in x and
    /// in y; a gap of twice the spacing still bridges, one step more does not.
    #[test]
    fn well_bridges_fill_exactly_the_gap() {
        let nwell = WELLS.layer("nwell").unwrap();
        let fill = |a: Rect, b: Rect| -> Vec<Rect> {
            well_bridges(&[well_cell(a, 1), well_cell(b, 1)], &[], &WELLS, &|_, _| true)
                .iter()
                .flat_map(|m| m.shapes.iter().filter(move |s| s.layer == nwell).map(|s| s.rect))
                .collect()
        };
        let a = Rect { x: 0, y: 0, w: 2000, h: 2000 };
        assert_eq!(fill(a, Rect { x: 3000, ..a }), vec![Rect { x: 2000, y: 0, w: 1000, h: 2000 }]);
        assert_eq!(fill(Rect { x: 3000, ..a }, a), vec![Rect { x: 2000, y: 0, w: 1000, h: 2000 }], "order-free");
        assert_eq!(fill(a, Rect { y: 3000, ..a }), vec![Rect { x: 0, y: 2000, w: 2000, h: 1000 }]);
        assert_eq!(fill(a, Rect { x: 2000 + 2540, ..a }).len(), 1, "gap 2·space");
        assert!(fill(a, Rect { x: 2000 + 2545, ..a }).is_empty(), "past 2·space");
        assert!(fill(a, Rect { x: 1000, ..a }).is_empty(), "overlapping wells");
        assert!(fill(a, Rect { x: 3000, y: 1500, ..a }).is_empty(), "shared span under the well width");
        let other_bulk = well_bridges(&[well_cell(a, 1), well_cell(Rect { x: 3000, ..a }, 2)], &[], &WELLS, &|_, _| true);
        assert!(other_bulk.is_empty(), "different bulks");
    }

    /// An implant deck: nsdm/psdm spacing 380, width 380.
    const IMPLANTS: Fake = Fake {
        layers: &["nsdm", "psdm"],
        rules: &[],
        widths: &[("nsdm", 380), ("psdm", 380)],
        spaces: &[("nsdm", 380), ("psdm", 380)],
        encs: &[],
    };

    fn implants(rects: &[(&str, Rect)]) -> Macro {
        let mut b = Builder::new(5);
        for (l, r) in rects {
            b.rect(IMPLANTS.layer(l).unwrap(), *r);
        }
        b.finish()
    }

    #[test]
    fn implant_bridges_fill_only_clean_gaps() {
        let n = IMPLANTS.layer("nsdm").unwrap();
        let a = Rect { x: 0, y: 0, w: 1000, h: 1000 };
        let fills = |m: Macro| -> Vec<Rect> { implant_bridges(&[m], &IMPLANTS).iter().flat_map(|m| m.shapes.iter().filter(move |s| s.layer == n).map(|s| s.rect)).collect() };
        assert!(implant_bridges(&[implants(&[("nsdm", a)])], &EMPTY).is_empty(), "no implant layers");
        assert!(implant_bridges(&[], &IMPLANTS).is_empty());
        assert_eq!(fills(implants(&[("nsdm", a), ("nsdm", Rect { x: 1200, ..a })])), vec![Rect { x: 1000, y: 0, w: 200, h: 1000 }]);
        assert_eq!(fills(implants(&[("nsdm", Rect { x: 1200, ..a }), ("nsdm", a)])), vec![Rect { x: 1000, y: 0, w: 200, h: 1000 }], "order-free");
        assert_eq!(fills(implants(&[("nsdm", a), ("nsdm", Rect { y: 1200, ..a })])), vec![Rect { x: 0, y: 1000, w: 1000, h: 200 }]);
        assert!(fills(implants(&[("nsdm", a), ("nsdm", Rect { x: 1380, ..a })])).is_empty(), "gap == spacing is legal");
        assert!(fills(implants(&[("nsdm", a), ("nsdm", Rect { x: 1200, y: 100, ..a })])).is_empty(), "partial span");
        assert!(fills(implants(&[("nsdm", a), ("nsdm", Rect { x: 1200, ..a }), ("psdm", Rect { x: 1050, y: 400, w: 100, h: 100 })])).is_empty(), "opposite implant in the gap");
        assert!(fills(implants(&[("nsdm", a), ("nsdm", Rect { x: 1200, ..a }), ("nsdm", Rect { x: 0, y: 0, w: 2200, h: 1000 })])).is_empty(), "already covered");
    }
}
