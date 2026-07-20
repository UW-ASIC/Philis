//! Resistor cell generator: body resistors with serpentine folding.

use crate::{CellBuilder, CellError, DeviceType, Orientation, PatternType, PortDef};

use super::{CellSpec, Pdk};
use crate::device::DeviceRecord;

/// One point in the resistor variant space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResistorSpec {
    pub n_segments: i32,
    pub pattern: PatternType,
}

impl CellSpec for ResistorSpec {
    fn enumerate(devices: &[DeviceRecord], pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        let patterns = if devices.len() > 1 {
            vec![PatternType::Single, PatternType::Interdig]
        } else {
            vec![PatternType::Single]
        };
        feasible_segments(&devices[0], pdk)
            .into_iter()
            .flat_map(|n_segments| {
                patterns.iter().map(move |&pattern| ResistorSpec {
                    n_segments,
                    pattern,
                })
            })
            .collect()
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let ref_dev = &devices[0];
        let seg_l = ref_dev.l / self.n_segments.max(1);
        let seg_pitch = ref_dev.w + pdk.res_seg_gap;
        let n = devices.len() as i32;
        // ponytail: interdig lays all segments in one row; single separates per device
        let total_w = match self.pattern {
            PatternType::Interdig => self.n_segments * n * seg_pitch,
            _ => {
                let per_dev = self.n_segments * seg_pitch;
                per_dev * n + pdk.device_gap * (n - 1).max(0)
            }
        };
        let total_h = pdk.res_head + seg_l + pdk.res_head;
        (total_w, total_h)
    }

    fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef> {
        devices
            .iter()
            .flat_map(|d| {
                [
                    PortDef::inout(format!("{}:P", d.name)),
                    PortDef::inout(format!("{}:N", d.name)),
                ]
            })
            .collect()
    }

    fn draw(
        &self,
        devices: &[DeviceRecord],
        pdk: &Pdk,
        b: &mut CellBuilder,
    ) -> Result<(), CellError> {
        b.set_device_type(DeviceType::Res);
        b.set_pattern(self.pattern);

        let ref_dev = &devices[0];
        // ponytail: item 1.12 — compute W from tolerance equation (AOAL ch06 6.3.1 Eq 6.12)
        // dR = sqrt(dRs² + (2*dW/W)²), solve for W >= 2*dW / sqrt(tol² - dRs²)
        let body_w = tolerance_width(ref_dev, pdk).max(ref_dev.w);
        let body_l = ref_dev.l;
        b.set_electrical(body_w, body_l, 1, body_w);

        let ly = &pdk.layers;
        let ct = pdk.contact;
        let head_l = pdk.res_head;
        let n_segments = self.n_segments.max(1);
        let seg_l = body_l / n_segments;
        let seg_pitch = body_w + pdk.res_seg_gap;

        // Build segment sequence: (device_index, segment_index_for_that_device)
        let sequence = res_segment_sequence(devices, self.pattern, n_segments);
        // Track per-device segment counter for P/N pin placement
        let mut dev_seg_placed: Vec<i32> = vec![0; devices.len()];

        for (slot, &(di, _seg_of_dev)) in sequence.iter().enumerate() {
            let dev = &devices[di];
            let seg_idx = dev_seg_placed[di];
            dev_seg_placed[di] += 1;

            let orient = if seg_idx % 2 == 1 {
                Orientation::MY
            } else {
                Orientation::R0
            };
            let sx = slot as i32 * seg_pitch;
            let total_h = head_l + seg_l + head_l;

            let (bx, by, bw, bh) = orient.transform_rect(0, 0, body_w, total_h, body_w, total_h);
            // Body on the non-conducting resistor layer so LVS extraction
            // recognizes it (deck device_recognition.resistor: body rpoly,
            // terminals licon) instead of shorting P to N through poly.
            // ponytail: extraction sees one resistor per body rect, so only
            // n_segments == 1 round-trips through LVS; folded segments were
            // never electrically joined either — fix both when serpentine
            // resistors are actually placed by a flow.
            b.rect(&ly.rpoly, sx + bx, by, bw, bh)?;

            let cy_top = head_l / 2 - ct / 2;
            let cy_bot = head_l + seg_l + head_l / 2 - ct / 2;
            let cx = body_w / 2 - ct / 2;

            let (tx, ty, _, _) = orient.transform_rect(cx, cy_top, ct, ct, body_w, total_h);
            b.rect(&ly.licon, sx + tx, ty, ct, ct)?;
            b.rect(&ly.li, sx + tx, ty, ct, ct)?;

            let (bx2, by2, _, _) = orient.transform_rect(cx, cy_bot, ct, ct, body_w, total_h);
            b.rect(&ly.licon, sx + bx2, by2, ct, ct)?;
            b.rect(&ly.li, sx + bx2, by2, ct, ct)?;

            // Pin sits `ext` off the head contact; the flow drops its own
            // licon at the pin center, so the two cuts must clear licon
            // min-spacing: ext - ct >= 170 (LICON.2). The flow clamps access
            // centers to bbox + ct/2 + grid, shifting the cut one grid step
            // inward at cell-edge pins — add one grid step of margin.
            let ext = 340 + b.grid();
            if seg_idx == 0 {
                b.rect(&ly.li, sx + tx, ty - ext, ct, ext + ct)?;
                b.pin(
                    &format!("{}:P", dev.name),
                    &ly.li,
                    sx + tx,
                    ty - ext,
                    ct,
                    ct,
                )?;
            }
            if seg_idx == n_segments - 1 {
                b.rect(&ly.li, sx + bx2, by2, ct, ext + ct)?;
                b.pin(
                    &format!("{}:N", dev.name),
                    &ly.li,
                    sx + bx2,
                    by2 + ext,
                    ct,
                    ct,
                )?;
            }
        }

        Ok(())
    }
}

/// Build the segment placement sequence for resistor layout.
/// For `Single`, devices are placed sequentially.
/// For `Interdig`, segments are interleaved in centroid-symmetric order
/// (ABBA pattern) to cancel linear process gradients.
/// Returns `(device_index, segment_index_for_that_device)`.
fn res_segment_sequence(
    devices: &[DeviceRecord],
    pattern: PatternType,
    n_segments: i32,
) -> Vec<(usize, i32)> {
    let n_dev = devices.len();
    if n_dev == 0 {
        return vec![];
    }
    match pattern {
        PatternType::Interdig if n_dev >= 2 => {
            // ponytail: greedy centroid interleave — ratio-preserving for
            // unequal segment counts, extends to N>2 devices.
            let counts: Vec<usize> = devices
                .iter()
                .map(|_| n_segments as usize) // each device contributes n_segments
                .collect();
            greedy_centroid_sequence(&counts)
                .into_iter()
                .map(|di| {
                    // segment index within that device will be assigned by
                    // the caller via dev_seg_placed counter
                    (di, 0)
                })
                .collect()
        }
        _ => {
            // Sequential: all segments of device 0, then device 1, etc.
            (0..n_dev)
                .flat_map(|di| (0..n_segments).map(move |seg| (di, seg)))
                .collect()
        }
    }
}

/// Greedy centroid-symmetric interleave over N devices with arbitrary
/// segment counts. Places slots from the outside in, always picking the
/// device with the highest remaining fraction (count/total).
/// Returns device indices in placement order.
fn greedy_centroid_sequence(counts: &[usize]) -> Vec<usize> {
    let total: usize = counts.iter().sum();
    if total == 0 {
        return vec![];
    }
    let mut remaining: Vec<usize> = counts.to_vec();
    let mut seq = vec![0usize; total];
    let mut lo = 0usize;
    let mut hi = total - 1;

    while lo <= hi {
        // Pick device with highest remaining fraction
        let pick = remaining
            .iter()
            .enumerate()
            .filter(|(_, &r)| r > 0)
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(i, _)| i)
            .unwrap_or(0);

        seq[lo] = pick;
        remaining[pick] -= 1;

        if lo < hi {
            // Mirror: place same device on the other end for symmetry
            if remaining[pick] > 0 {
                seq[hi] = pick;
                remaining[pick] -= 1;
            } else {
                // Pick next best for the mirror slot
                let pick2 = remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, &r)| r > 0)
                    .max_by(|(_, a), (_, b)| a.cmp(b))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                seq[hi] = pick2;
                remaining[pick2] = remaining[pick2].saturating_sub(1);
            }
            if hi == 0 {
                break;
            }
            hi -= 1;
        }
        lo += 1;
    }
    seq
}

/// Compute minimum width from tolerance equation (AOAL Eq 6.12).
/// `dR = sqrt(dRs² + (2·dW/W)²)` → `W >= 2·dW / sqrt(tol² - dRs²)`.
/// Returns netlist W if PDK has no tolerance data for this model.
fn tolerance_width(dev: &DeviceRecord, pdk: &Pdk) -> i32 {
    let model = &dev.model_name;
    let drs = match pdk.sheet_tolerance.get(model) {
        Some(&v) if v > 0.0 => v,
        _ => return dev.w,
    };
    let dw_nm = match pdk.linewidth_control_nm.get(model) {
        Some(&v) if v > 0 => v,
        _ => return dev.w,
    };
    // Target tolerance: tier-dependent (5% Moderate, 2% Exceptional, 10% default)
    // ponytail: hardcode reasonable targets; per-net budgets add when constraint pipe delivers them
    let tol = 0.05_f64; // 5% default target
    let radicand = tol * tol - drs * drs;
    if radicand <= 0.0 {
        // Sheet tolerance alone exceeds target — warn and return netlist W
        // ponytail: pelgrom_warning when unreachable, log not panic
        return dev.w;
    }
    let w_min = (2.0 * dw_nm as f64 / radicand.sqrt()).ceil() as i32;
    w_min
}

/// Electrically equivalent fold-count domain, ranked by geometric diversity.
fn feasible_segments(dev: &DeviceRecord, pdk: &Pdk) -> Vec<i32> {
    let max_segments = (dev.l / pdk.res_min_segment.max(1)).clamp(1, 64);
    let mut opts: Vec<i32> = (1..=max_segments)
        .filter(|&n| dev.l % n == 0 && (n == 1 || n % 2 == 0))
        .collect();
    opts.sort_by(|&a, &b| {
        let quality = |n: i32| {
            let w = f64::from(n * (dev.w + pdk.res_seg_gap));
            let h = f64::from(2 * pdk.res_head + dev.l / n);
            (w / h).ln().abs()
        };
        quality(a).total_cmp(&quality(b))
    });
    opts.truncate(8);
    opts.sort_unstable();
    if opts.is_empty() {
        opts.push(1);
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use crate::{CellBuilder, MatchingTier};

    fn test_deck() -> crate::Deck {
        crate::test_util::deck_from_layers(&[
            ("rpoly", 66, 13),
            ("li", 67, 20),
            ("licon", 66, 44),
        ])
    }

    fn res(name: &str, w: i32, l: i32) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Res,
            w,
            l,
            nf: 1,
            multiplier: 1,
            model_name: "rpo_01v8".into(),
            terminals: [("P".into(), 1), ("N".into(), 2)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_short_resistor() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![res("R1", 500, 2000)];
        let specs = ResistorSpec::enumerate(&devices, &pdk);
        assert!(!specs.is_empty());

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pins.len(), 2, "P + N");
    }

    #[test]
    fn serpentine_long_resistor() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![res("R1", 500, 60_000)];
        let specs = ResistorSpec::enumerate(&devices, &pdk);
        assert!(!specs.is_empty());

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.store.poly_count() > 1, "should have multiple segments");
    }

    #[test]
    fn drc_clean_resistor() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![res("R1", 500, 2000)];
        let specs = ResistorSpec::enumerate(&devices, &pdk);
        let cell = SpecCell {
            spec: specs[0],
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
