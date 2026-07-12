//! Guard ring generator: contacted, implanted, welled bands around a cell bbox.
//!
//! AOAL ch14 14.2.2 — un-contacted ring = no collection, no latch-up protection.
//! Emits: diffusion band + licon array + li overlay + tap implant + well layer.
//! Mirrors tap-column recipe from mosfet.rs bulk-tap emission.

use crate::{Bbox, CellBuilder, CellError, DeviceType};

use super::Pdk;

/// Clearance from the enclosed device geometry to the collecting diffusion.
pub const GUARD_RING_GAP: i32 = 200;

/// Guard ring type — determines implant polarity and well layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingType {
    /// P+ ring in p-substrate (NMOS guard) — connects to VSS.
    PsubRing,
    /// N+ ring in n-well (PMOS guard) — connects to VDD.
    NwellRing,
}

/// Draw a complete guard ring around `inner` bbox.
///
/// Emits four bands (top/bottom/left/right) of:
///   diff + licon array + li overlay + tap implant (nsdm/psdm) + well (nwell for NwellRing)
///
/// Returns the outer bbox of the ring.
pub fn draw_guard_ring(
    b: &mut CellBuilder,
    pdk: &Pdk,
    inner: &Bbox,
    ring_type: RingType,
    ring_width: i32,
    net_name: &str,
) -> Result<Bbox, CellError> {
    let ly = &pdk.layers;
    let ct = pdk.contact;
    let licon_pitch = pdk.guard_licon_pitch;
    let gap = GUARD_RING_GAP;

    let ix0 = inner.xmin - gap;
    let iy0 = inner.ymin - gap;
    let ix1 = inner.xmax + gap;
    let iy1 = inner.ymax + gap;

    let ox0 = ix0 - ring_width;
    let oy0 = iy0 - ring_width;
    let ox1 = ix1 + ring_width;
    let oy1 = iy1 + ring_width;

    // Implant layer: P+ taps use psdm, N+ taps use nsdm
    let implant_layer = match ring_type {
        RingType::PsubRing => &ly.psdm,
        RingType::NwellRing => &ly.nsdm,
    };
    // Tap diffusion layer
    let tap_layer = &ly.tap;

    // Four bands: bottom, top, left, right
    let bands: [(i32, i32, i32, i32); 4] = [
        (ox0, oy0, ox1 - ox0, ring_width),              // bottom
        (ox0, iy1, ox1 - ox0, ring_width),              // top
        (ox0, oy0 + ring_width, ring_width, iy1 - iy0), // left
        (ix1, oy0 + ring_width, ring_width, iy1 - iy0), // right
    ];

    for &(bx, by, bw, bh) in &bands {
        if bw <= 0 || bh <= 0 {
            continue;
        }
        // Tap diffusion
        let _ = b.rect(tap_layer, bx, by, bw, bh);

        // Implant (same extent as tap)
        let _ = b.rect(implant_layer, bx, by, bw, bh);

        // Li overlay (same extent)
        let _ = b.rect(&ly.li, bx, by, bw, bh);

        // Licon array: fill the band with contacts at licon_pitch
        let margin = (ring_width - ct) / 2;
        if bw >= bh {
            // Horizontal band: contacts along x
            let cy = by + bh / 2 - ct / 2;
            let mut cx = bx + margin;
            while cx + ct <= bx + bw - margin {
                let _ = b.rect(&ly.licon, cx, cy, ct, ct);
                // licon connects tap->li; mcon is separately required for
                // li->met1. Without it the visually complete ring is floating.
                let _ = b.rect(&ly.mcon, cx, cy, ct, ct);
                cx += licon_pitch;
            }
        } else {
            // Vertical band: contacts along y
            let cx = bx + bw / 2 - ct / 2;
            let mut cy = by + margin;
            while cy + ct <= by + bh - margin {
                let _ = b.rect(&ly.licon, cx, cy, ct, ct);
                let _ = b.rect(&ly.mcon, cx, cy, ct, ct);
                cy += licon_pitch;
            }
        }

        // Met1 strap for connection
        let _ = b.rect(&ly.met1, bx, by, bw, bh);
        // Pin on met1 for the ring net
        let _ = b.pin(
            &format!("ring:{net_name}"),
            &ly.met1,
            bx,
            by,
            bw.min(ct + 2 * pdk.m1_enc),
            bh.min(ct + 2 * pdk.m1_enc),
        );
    }

    // N-well for NwellRing: enclose the entire ring
    if ring_type == RingType::NwellRing {
        let enc = pdk.nwell_diff_enc;
        b.rect(
            &ly.nwell,
            ox0 - enc,
            oy0 - enc,
            (ox1 - ox0) + 2 * enc,
            (oy1 - oy0) + 2 * enc,
        )?;
    }

    Ok(Bbox {
        xmin: ox0,
        ymin: oy0,
        xmax: ox1,
        ymax: oy1,
    })
}

/// Outer mask bounds produced by [`draw_guard_ring`]. Kept separate so the
/// post-placement planner and the geometry emitter use identical dimensions.
pub fn guard_ring_outer_bbox(inner: &Bbox, ring_width: i32) -> Bbox {
    Bbox {
        xmin: inner.xmin - GUARD_RING_GAP - ring_width,
        ymin: inner.ymin - GUARD_RING_GAP - ring_width,
        xmax: inner.xmax + GUARD_RING_GAP + ring_width,
        ymax: inner.ymax + GUARD_RING_GAP + ring_width,
    }
}

/// Compute guard ring width from well/epi depth (item 1.4).
/// AOAL ch14 14.2.3/14.2.4 — collecting ring width must >= well/epi depth
/// to intercept minority carriers.
pub fn ring_width_from_depth(pdk: &Pdk, ring_type: RingType) -> i32 {
    let depth_width = match ring_type {
        RingType::PsubRing => {
            if pdk.retrograde_pwell {
                pdk.p_well_depth
            } else {
                pdk.p_epi_thickness
            }
        }
        RingType::NwellRing => pdk.n_well_depth,
    };
    pdk.min_guard_ring_width.max(depth_width)
}

/// Whether a device type needs a PsubRing or NwellRing.
pub fn ring_type_for_device(dt: DeviceType) -> Option<RingType> {
    match dt {
        DeviceType::Nmos | DeviceType::Ncap => Some(RingType::PsubRing),
        DeviceType::Pmos | DeviceType::Pcap => Some(RingType::NwellRing),
        _ => None,
    }
}
