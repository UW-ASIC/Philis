//! Guard rings, drawn around **placed** devices after placement and before
//! routing (a ring is both an obstacle and a net target). Same-class
//! shareable requesters placed within `guard_ring_merge_gap_nm` share one
//! ring, unless the merged hull would cross a non-member cell: only each
//! requester's own bbox (inflated by [`ring_halo`]) was reserved.

use analog::cell::{GuardRingRequirement, GuardRingType};
use analog::Constraints;
use pnr_core::{DeviceId, Layout, Macro, Pin, Process, Rect, Target};

use crate::builder::{cut_lattice, req, snap_cut};
use crate::Builder;

/// Draw every guard ring the placed `layout` calls for. One [`Macro`] per ring
/// (carrying its `ring` pin), to be folded into the layout geometry before
/// routing.
#[must_use]
pub fn guard_rings(layout: &Layout, c: &Constraints, process: &dyn Process) -> Vec<Macro> {
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

    let merge_gap = process.rule("guard_ring_merge_gap_nm", 2000) as f32;

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
            let ext = ring_halo(rep, process);
            let inner = Rect {
                x: inner.x + ext,
                y: inner.y + ext,
                w: (inner.w - 2 * ext).max(0),
                h: (inner.h - 2 * ext).max(0),
            };
            let mut b = Builder::new(process.grid());
            draw_ring(&mut b, process, rep, inner);
            b.finish()
        })
        .collect()
}

/// Partition requirements into rings: same-class (`connection_net`, `ring_type`)
/// **shareable** requesters that are placed within `merge_gap` merge; every
/// non-shareable requirement is its own ring. A merged cluster whose hull
/// would contain or intersect a placed cell outside the cluster splits back
/// to singletons — the hull covers unreserved ground, see the module doc.
/// **Pure** (no PDK) so the grouping policy is table-testable.
fn clusters(reqs: &[&GuardRingRequirement], layout: &Layout, merge_gap: f32) -> Vec<Vec<usize>> {
    let n = reqs.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (reqs[i], reqs[j]);
            let same_class = a.connection_net == b.connection_net && a.ring_type == b.ring_type;
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
    // A cluster whose hull hits a non-member splits to singletons (each was
    // reserved by construction). ponytail: not maximal safe sub-merges.
    let mut safe: Vec<Vec<usize>> = Vec::with_capacity(out.len());
    for members in out {
        if members.len() > 1 && hull_hits_foreign(&members, reqs, layout) {
            safe.extend(members.into_iter().map(|i| vec![i]));
        } else {
            safe.push(members);
        }
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

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]]; // path-halving
        i = parent[i];
    }
    i
}

fn union(parent: &mut [usize], i: usize, j: usize) {
    let (ri, rj) = (find(parent, i), find(parent, j));
    if ri != rj {
        parent[ri] = rj;
    }
}

fn dev_rect(l: &Layout, d: DeviceId) -> Rect {
    let (cx, cy, hw, hh) = l.bbox(Target::Device(d));
    Rect { x: cx - hw, y: cy - hh, w: 2 * hw, h: 2 * hh }
}

fn union_rect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}


/// Device-to-band gap: clears both diff spacing (270) and the ring implant
/// against a cell's tap implant (380).
const GUARD_RING_GAP: i32 = 380;

/// Draw the four contacted tap/implant/li bands of `r`'s ring around `inner`.
///
/// No met1 band: routing is met1-and-up, so a met1 loop would short every
/// route crossing it. The `ring` pins sit on li; the router stitches down.
fn draw_ring(b: &mut Builder, process: &dyn Process, r: &GuardRingRequirement, inner: Rect) {
    let ct = process.rule("contact", 170);
    let lat = cut_lattice(process);
    let pitch = snap_cut(if r.tap_pitch_nm > 0 { r.tap_pitch_nm } else { process.rule("guard_licon_pitch", 340) } + lat - 1, lat).max(lat);
    let ring_width = width_from_depth(process, r.ring_type).max(r.min_width_nm);
    let gap = GUARD_RING_GAP;

    let ix0 = inner.x - gap;
    let iy0 = inner.y - gap;
    let ix1 = inner.x + inner.w + gap;
    let iy1 = inner.y + inner.h + gap;
    let ox0 = ix0 - ring_width;
    let oy0 = iy0 - ring_width;
    let ox1 = ix1 + ring_width;
    let oy1 = iy1 + ring_width;

    let implant = req(process, implant_name(r.ring_type));
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
        b.rect(tap, band);
        b.rect(implant, band);
        b.rect(li, band);

        let margin = (ring_width - ct) / 2;
        if bw >= bh {
            let cy = snap_cut(by + bh / 2 - ct / 2, lat);
            let mut cx = snap_cut(bx + margin + lat - 1, lat);
            while cx + ct <= bx + bw - margin {
                b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                cx += pitch;
            }
        } else {
            let cx = snap_cut(bx + bw / 2 - ct / 2, lat);
            let mut cy = snap_cut(by + margin + lat - 1, lat);
            while cy + ct <= by + bh - margin {
                b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                cy += pitch;
            }
        }
        b.pin(Pin {
            name: "ring".into(),
            net: r.connection_net,
            layer: li,
            at: Rect { x: bx + bw / 2 - ct / 2, y: by + bh / 2 - ct / 2, w: ct, h: ct },
        });
    }

    // The p+ ring lives in an n-well; the n+ ring in the substrate has none.
    if in_nwell(r.ring_type) {
        if let Some(nwell) = process.layer("nwell") {
            let enc = process.rule("nwell_diff_enc", 180);
            b.rect(nwell, Rect {
                x: ox0 - enc,
                y: oy0 - enc,
                w: (ox1 - ox0) + 2 * enc,
                h: (oy1 - oy0) + 2 * enc,
            });
        }
    }
}

/// The placement halo: how far `r`'s ring (gap + band + outer clearance)
/// extends past its device bbox. The caller inflates the requester's variant
/// bboxes by this so the placer reserves the ring's ground.
#[must_use]
pub fn ring_halo(r: &GuardRingRequirement, process: &dyn Process) -> i32 {
    width_from_depth(process, r.ring_type).max(r.min_width_nm)
        + GUARD_RING_GAP
        + outer_clear(r.ring_type, process)
}

/// Band outer edge to halo edge for a well-less ring: implant spacing (380)
/// and tap spacing (270) against a flush neighbour.
const RING_OUTER_CLEAR: i32 = 380;

/// The ring band's implant. A ring is tied to its device's bulk, so it is
/// drawn as that bulk's tap: `Hcgr`/`Hbgr` ring a PMOS and are an n+ tap in
/// the n-well, `Ecgr`/`Ebgr` ring an NMOS and are a p+ substrate tap. (The
/// other way round is a p+ "tap" in a well and an n+ "tap" in bare
/// substrate, which magic rejects as diff/tap.10 and diff/tap.11.)
fn implant_name(ring_type: GuardRingType) -> &'static str {
    if in_nwell(ring_type) { "nsdm" } else { "psdm" }
}

/// `Hcgr`/`Hbgr` (PMOS rings) sit in an n-well; `Ecgr`/`Ebgr` (NMOS rings)
/// in the substrate. A well over an NMOS ring would bury the NMOS.
fn in_nwell(ring_type: GuardRingType) -> bool {
    matches!(ring_type, GuardRingType::Hcgr | GuardRingType::Hbgr)
}

/// Outer clearance: an in-well ring owes a neighbour's well
/// `nwell_min_spacing` beyond its own well enclosure; a well-less ring only
/// [`RING_OUTER_CLEAR`] (which already beats well-to-outside-diff, 340).
fn outer_clear(ring_type: GuardRingType, process: &dyn Process) -> i32 {
    if in_nwell(ring_type) {
        process.rule("nwell_min_spacing", 1270) + process.rule("nwell_diff_enc", 180)
    } else {
        RING_OUTER_CLEAR
    }
}

/// Ring width: at least the depth of the region it collects from (n-well,
/// retrograde p-well, or p-epi).
fn width_from_depth(process: &dyn Process, ring_type: GuardRingType) -> i32 {
    let depth_width = if in_nwell(ring_type) {
        process.rule("n_well_depth", 2000)
    } else if process.rule("retrograde_pwell", 0) != 0 {
        process.rule("p_well_depth", 1500)
    } else {
        process.rule("p_epi_thickness", 3000)
    };
    process.rule("min_guard_ring_width", 420).max(depth_width)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn req(dev: u16, net: u16, ty: GuardRingType, shareable: bool) -> GuardRingRequirement {
        GuardRingRequirement {
            device: DeviceId(dev),
            ring_type: ty,
            shareable,
            tap_pitch_nm: 0,
            min_width_nm: 420,
            max_ring_resistance_mohm: 10_000,
            enclosure_complete: true,
            connection_net: pnr_core::NetId(net),
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
        }
    }

    /// A tap ring is n+ exactly when it sits in a well, and only PMOS rings
    /// do (the annotator gives a PMOS `Hcgr`, an NMOS `Ecgr`).
    #[test]
    fn well_follows_implant_polarity() {
        use GuardRingType::{Ebgr, Ecgr, Hbgr, Hcgr};
        for t in [Ecgr, Ebgr, Hcgr, Hbgr] {
            assert_eq!(in_nwell(t), implant_name(t) == "nsdm", "{t:?}");
        }
        assert!(in_nwell(Hcgr) && !in_nwell(Ecgr));
    }

    #[test]
    fn merges_adjacent_shareable_same_class() {
        use GuardRingType::Hcgr;
        let l = layout_at(&[(0, 0), (300, 0), (100_000, 0)]);
        let reqs = [
            req(0, 5, Hcgr, true),
            req(1, 5, Hcgr, true), // edge-gap 100nm from dev0 → merges
            req(2, 5, Hcgr, true), // ~100µm away → own ring
        ];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let mut c = clusters(&refs, &l, 2000.0);
        c.sort_by_key(|m| m.len());
        assert_eq!(c.len(), 2, "adjacent pair merges, far one stays");
        assert_eq!(c[1], vec![0, 1]);
    }

    #[test]
    fn never_merges_across_class_or_private() {
        use GuardRingType::{Ecgr, Hcgr};
        let l = layout_at(&[(0, 0), (300, 0), (600, 0), (900, 0)]);
        let reqs = [
            req(0, 5, Hcgr, true),
            req(1, 6, Hcgr, true),  // different net → no merge
            req(2, 5, Ecgr, true),  // different ring_type → no merge
            req(3, 5, Hcgr, false), // private (injector) → own ring
        ];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let c = clusters(&refs, &l, 2000.0);
        assert_eq!(c.len(), 4, "class/net/private all block the merge");
    }

    #[test]
    fn never_merges_across_a_foreign_cell() {
        use GuardRingType::Hcgr;
        // Three cells in a row; only the OUTER two request rings. Their edge
        // gap (1800 nm) is within merge_gap, but the merged hull would swallow
        // the non-member middle cell — bands would be drawn straight across it.
        let l = layout_at(&[(0, 0), (1000, 0), (2000, 0)]);
        let reqs = [req(0, 5, Hcgr, true), req(2, 5, Hcgr, true)];
        let refs: Vec<&GuardRingRequirement> = reqs.iter().collect();
        let c = clusters(&refs, &l, 2000.0);
        assert_eq!(c.len(), 2, "hull would cross the middle cell: split to singletons");
        assert_eq!(c, vec![vec![0], vec![1]]);

        // Control: the same pair with no middle cell merges.
        let l2 = layout_at(&[(0, 0), (2000, 0)]);
        let reqs2 = [req(0, 5, Hcgr, true), req(1, 5, Hcgr, true)];
        let refs2: Vec<&GuardRingRequirement> = reqs2.iter().collect();
        let c2 = clusters(&refs2, &l2, 2000.0);
        assert_eq!(c2, vec![vec![0, 1]], "no foreigner in the hull: pair merges");
    }
}
