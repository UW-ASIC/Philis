//! Post-placement composition: guard rings drawn around **placed** geometry.
//!
//! A guard ring is not a device variant — it encloses a placed group bbox that
//! only exists after placement, and whether two requesters share one ring
//! depends on placed adjacency. So it is applied *after* placement and *before*
//! routing (a ring is both an obstacle and a net target the router must see). See
//! ADR 0006. It lives in `cells` (not the pipeline crate) so `macroMaster`, which
//! depends on `cells`, can reach it too — but it is a **pass**, not a `Cell`.
//!
//! Grouping is **annotator-intent × placed-adjacency**: the annotator's
//! [`GuardRingRequirement`] carries the class (`connection_net`, `ring_type` ⇒
//! well/implant, `shareable`); this pass merges same-class requesters that land
//! *adjacent*, and never merges across a class or rings a non-shareable
//! (injector) device in. The physics forces the split — purpose/net/well is
//! annotator-known, adjacency is post-placement-known.
//!
//! A merge is also refused when its **combined hull would hit a non-member**:
//! only each requester's own bbox was halo-reserved during placement, so the
//! union hull of two requesters spans ground nobody reserved, and a band drawn
//! around that hull crosses whatever the placer parked there (measured: ota's
//! nine `poly_to_tap_spacing` findings were ring bands across a 40 µm cell).
//! Such a cluster splits back to singletons — always correct, because each
//! requester alone fits its own reservation by construction. The check needs
//! no extra margin: placed bboxes already carry [`ring_halo`], so every drawn
//! band lies inside the hull, at least `RING_OUTER_CLEAR` deep — a foreigner
//! outside the hull is exactly as safe as one parked flush against a singleton
//! (inflating the hull again by the halo double-counts, and measurably split
//! harmless merges: ota shipped 77 findings against 4).
//!
//! The **ring geometry** lives here too (it was `cells::guard_ring`): it is drawn
//! through [`crate::Builder`], the shared geometry surface — a ring is geometry,
//! not a device variant, so it never becomes a `Cell`/`DeviceGen`.

use analog::cell::{GuardRingRequirement, GuardRingType};
use analog::Constraints;
use pnr_core::{DeviceId, Layout, Macro, NetId, Pin, Process, Rect, Target};

use crate::builder::{layer, req, rule};
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
            // Representative: the widest-ring requirement dominates (same class,
            // so ring_type/net agree; the largest min_width wins).
            let rep = members
                .iter()
                .map(|&i| reqs[i])
                .max_by_key(|r| r.min_width_nm)
                .expect("cluster is non-empty");
            // The placed bbox already carries the ring's halo (the variant
            // table was inflated by `ring_halo`, so the placer reserved this
            // ground). Shrink back to the device geometry so the drawn band
            // lands INSIDE the reservation — its outer edge returns exactly to
            // the halo edge — instead of outside it, on the neighbours.
            let ext = ring_halo(rep, process);
            let inner = Rect {
                x: inner.x + ext,
                y: inner.y + ext,
                w: (inner.w - 2 * ext).max(0),
                h: (inner.h - 2 * ext).max(0),
            };
            GuardRing::from_req(rep, process).draw(inner, process)
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
    // Placement-aware hull check: refuse any merge whose combined hull hits a
    // cell that is not a cluster member. Splitting to singletons (members stay
    // in ascending order, so the output is deterministic) is the monotone,
    // always-correct fallback; each singleton was reserved by construction.
    // ponytail: splits to singletons, not maximal safe sub-merges — revisit if
    // shared rings become an area target.
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

// ───────────────────────────────────────────────────────────────────────────
//  Ring geometry (was `cells::guard_ring`). Drawn through `crate::Builder`.
// ───────────────────────────────────────────────────────────────────────────

/// Clearance from the enclosed device geometry to the collecting diffusion.
/// Was 270 (sky130 difftap.3, diff-to-diff): the binding rule is actually the
/// ring's own implant against the cell's tap implant — psdm/nsdm min_spacing
/// is 380, and a psdm-class ring 270 from a cell whose psdm tap reaches its
/// bbox edge ships three `psdm_min_spacing` findings per ring (measured on
/// rc_filter). The gap must clear the widest of the two rule families.
const GUARD_RING_GAP: i32 = 380;

/// A concrete guard ring: a tapped diffusion ring sized to a resistance target
/// with a given tap pitch, tied to `connection_net`. Theory: AOAL ch14 (latchup).
struct GuardRing {
    width_nm: i32,
    tap_pitch_nm: i32,
    ring_type: GuardRingType,
    connection_net: NetId,
}

impl GuardRing {
    /// Build from an annotator [`GuardRingRequirement`], sizing width from
    /// well/epi depth (clamped to `min_width_nm`) and tap pitch from the
    /// requirement or the PDK default.
    fn from_req(r: &GuardRingRequirement, process: &dyn Process) -> Self {
        GuardRing {
            width_nm: width_from_depth(process, r.ring_type).max(r.min_width_nm),
            tap_pitch_nm: if r.tap_pitch_nm > 0 {
                r.tap_pitch_nm
            } else {
                rule(process, "guard_licon_pitch", 340)
            },
            ring_type: r.ring_type,
            connection_net: r.connection_net,
        }
    }

    /// Draw the four contacted bands around the **real placed** `inner` bbox.
    fn draw(&self, inner: Rect, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        draw_ring(&mut b, process, self, inner);
        b.finish()
    }
}

/// Draw the four contacted bands around `inner`. Ported from `draw_guard_ring`.
fn draw_ring(b: &mut Builder, process: &dyn Process, g: &GuardRing, inner: Rect) {
    let ct = rule(process, "contact", 170);
    let licon_pitch = g.tap_pitch_nm.max(1);
    let ring_width = g.width_nm;
    let gap = GUARD_RING_GAP;
    let m1_enc = rule(process, "m1_enc", 60);

    let ix0 = inner.x - gap;
    let iy0 = inner.y - gap;
    let ix1 = inner.x + inner.w + gap;
    let iy1 = inner.y + inner.h + gap;
    let ox0 = ix0 - ring_width;
    let oy0 = iy0 - ring_width;
    let ox1 = ix1 + ring_width;
    let oy1 = iy1 + ring_width;

    // Implant: N+ (nsdm) for the electron ring, P+ (psdm) for the hole ring. A
    // ring with no implant and no tap is just a metal loop — it collects
    // nothing, so none of these are optional.
    let implant = req(process, implant_name(g.ring_type));
    let tap = req(process, "tap");
    let li = req(process, "li");
    // These defaulted to `li`, which drew a *contact* as interconnect: the cut
    // vanished and the two layers were never actually tied.
    let licon = req(process, "licon");
    let _ = m1_enc;

    // NO met1 band, and the pin sits on li — deliberately. The routing stack is
    // met1-and-up with li reserved for cells (dr's pin-access split), and the
    // stack's horizontal layer is met1: any net entering or leaving the ring
    // must cross a vertical band on met1, so a met1 band loop is a drawn short
    // with every crossing route — measured on rc_filter, where two rings fused
    // the whole design into ONE extracted net. On li the bands are
    // uncrossable-by-construction (no route is ever drawn there), and the
    // router reaches the li pin the same way it reaches every cell pin: an
    // mcon stitch from met1 down.
    //
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
        b.rect(tap, Rect { x: bx, y: by, w: bw, h: bh });
        b.rect(implant, Rect { x: bx, y: by, w: bw, h: bh });
        b.rect(li, Rect { x: bx, y: by, w: bw, h: bh });

        let margin = (ring_width - ct) / 2;
        if bw >= bh {
            let cy = by + bh / 2 - ct / 2;
            let mut cx = bx + margin;
            while cx + ct <= bx + bw - margin {
                b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                cx += licon_pitch;
            }
        } else {
            let cx = bx + bw / 2 - ct / 2;
            let mut cy = by + margin;
            while cy + ct <= by + bh - margin {
                b.rect(licon, Rect { x: cx, y: cy, w: ct, h: ct });
                cy += licon_pitch;
            }
        }

        b.pin(Pin {
            name: "ring".into(),
            net: g.connection_net,
            layer: li,
            at: Rect {
                x: bx + bw / 2 - ct / 2,
                y: by + bh / 2 - ct / 2,
                w: ct,
                h: ct,
            },
        });
    }

    // N-well under the whole ring — only for the p+ ring, which lives *in* the
    // well (see `in_nwell`).
    if in_nwell(g.ring_type) {
        if let Some(nwell) = layer(process, "nwell") {
            let enc = rule(process, "nwell_diff_enc", 180);
            b.rect(nwell, Rect {
                x: ox0 - enc,
                y: oy0 - enc,
                w: (ox1 - ox0) + 2 * enc,
                h: (oy1 - oy0) + 2 * enc,
            });
        }
    }
}

/// How far a ring around `r`'s device extends past the device bbox: band width
/// plus the diff-spacing gap. The **placement halo**: `frontend/library`
/// inflates the requester's variant bboxes by this much, so the placer reserves
/// the ring's ground by construction and a drawn band can never land on a
/// neighbouring cell. Without the halo, rc_filter's two rings (3 µm bands from
/// the epi-depth sizing) stamped tap/implant/li straight across the other
/// cells: a ghost `pgate` from ring-psdm over the NMOS row, and one merged
/// extracted net across the whole design.
#[must_use]
pub fn ring_halo(r: &GuardRingRequirement, process: &dyn Process) -> i32 {
    width_from_depth(process, r.ring_type).max(r.min_width_nm)
        + GUARD_RING_GAP
        + outer_clear(r.ring_type, process)
}

/// Clearance from the ring band's OUTER edge to the halo boundary — i.e. to
/// whatever neighbour the placer parks flush against it. The band is tap +
/// implant, so the binding rules are implant spacing (380) and tap spacing
/// (270); without this the band sat exactly on the halo edge and every
/// abutting cell shipped `tap_min_spacing`/`poly_to_tap` findings (chain4,
/// 24 of them, margins 55–135).
const RING_OUTER_CLEAR: i32 = 380;

/// The ring band's implant: N+ for the electron ring, P+ for the hole ring.
fn implant_name(ring_type: GuardRingType) -> &'static str {
    match ring_type {
        GuardRingType::Ecgr | GuardRingType::Ebgr => "nsdm",
        GuardRingType::Hcgr | GuardRingType::Hbgr => "psdm",
    }
}

/// Does this ring's diffusion sit **inside an n-well**? The one fact three
/// sites key off — the drawn well, the depth that sizes the band, and the
/// outer clearance — so it is stated once.
///
/// `Hcgr`/`Hbgr` are p+ (psdm) and a p+ diffusion only exists in an n-well;
/// `Ecgr`/`Ebgr` are n+ (nsdm) collecting in the p-substrate and carry **no**
/// well. This was inverted: every Ecgr drew an n-well over the NMOS it rings,
/// which is a channel in the wrong body — magic extracted **1** nfet from
/// chain4's four (3 of 4 `diff` rects sat fully inside an n-well). The deck has
/// no channel-vs-well polarity rule, so signoff read clean throughout.
fn in_nwell(ring_type: GuardRingType) -> bool {
    matches!(ring_type, GuardRingType::Hcgr | GuardRingType::Hbgr)
}

/// The per-class outer clearance. The two concerns this used to conflate:
///
/// 1. *This* ring carries a well: it reaches `nwell_diff_enc` past its band and
///    owes `nwell_min_spacing` to whatever well a flush neighbour brings — so
///    1450, and the neighbour's well is then automatically cleared too.
///    Measured on ota: two split in-well rings 360 apart shipped two
///    `nwell_min_spacing` findings (margin 910 against the 1270 rule).
/// 2. This ring carries *no* well but the neighbour might (a PMOS cell, whose
///    own nwell reaches its bbox edge). The binding rule is then well-to-
///    outside-diffusion, not well-to-well: sky130 wants 340, and
///    `RING_OUTER_CLEAR` (380, from the implant/tap spacing that binds against
///    a well-less neighbour) already dominates it. Holding the well-to-well
///    1450 here would be reserving 1 µm of ground per side for a well this ring
///    does not have — and a PMOS neighbour parks its *own* 1450 stand-off
///    anyway, so the pair is 1830 apart regardless.
fn outer_clear(ring_type: GuardRingType, process: &dyn Process) -> i32 {
    if in_nwell(ring_type) {
        rule(process, "nwell_min_spacing", 1270) + rule(process, "nwell_diff_enc", 180)
    } else {
        RING_OUTER_CLEAR
    }
}

/// Ring width from well/epi depth (AOAL ch14 14.2.3/14.2.4 — the collecting ring
/// must be at least as wide as the well/epi it intercepts). The depth is that of
/// the region the ring is diffused *into*: an in-well ring intercepts carriers
/// in the n-well, a substrate ring intercepts them in the p-well (retrograde) or
/// the full p-epi.
fn width_from_depth(process: &dyn Process, ring_type: GuardRingType) -> i32 {
    let depth_width = if in_nwell(ring_type) {
        rule(process, "n_well_depth", 2000)
    } else if rule(process, "retrograde_pwell", 0) != 0 {
        rule(process, "p_well_depth", 1500)
    } else {
        rule(process, "p_epi_thickness", 3000)
    };
    rule(process, "min_guard_ring_width", 420).max(depth_width)
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
            connection_net: NetId(net),
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

    /// The well must follow the implant: p+ only exists in an n-well, n+
    /// collecting in the p-substrate never carries one. These two were
    /// inverted — every NMOS's Ecgr drew an n-well over it and magic extracted
    /// 1 nfet from chain4's four, with signoff clean (no polarity rule in the
    /// deck). Cross-checks the two independent sites, not one against itself.
    #[test]
    fn well_follows_implant_polarity() {
        use GuardRingType::{Ebgr, Ecgr, Hbgr, Hcgr};
        for t in [Ecgr, Ebgr, Hcgr, Hbgr] {
            assert_eq!(in_nwell(t), implant_name(t) == "psdm", "{t:?}");
        }
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
