//! Resistor cell generator: body resistors with serpentine folding.

use substrate3::{
    CellBuilder, CellError, DeviceType, Orientation, PatternType, PortDef,
};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

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
                patterns.iter().map(move |&pattern| ResistorSpec { n_segments, pattern })
            })
            .collect()
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let ref_dev = &devices[0];
        let seg_l = ref_dev.l / self.n_segments.max(1);
        let seg_pitch = ref_dev.w + pdk.res_seg_gap;
        let per_dev = self.n_segments * seg_pitch;
        let n = devices.len() as i32;
        let total_w = per_dev * n + pdk.device_gap * (n - 1).max(0);
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
        let body_w = ref_dev.w;
        let body_l = ref_dev.l;
        b.set_electrical(body_w, body_l, 1, body_w);

        let ly = &pdk.layers;
        let ct = pdk.contact;
        let head_l = pdk.res_head;
        let n_segments = self.n_segments.max(1);
        let seg_l = body_l / n_segments;
        let seg_pitch = body_w + pdk.res_seg_gap;

        for (di, dev) in devices.iter().enumerate() {
            let dev_x_off = di as i32 * (n_segments as i32 * seg_pitch + pdk.device_gap);

            for seg in 0..n_segments {
                let orient = if seg % 2 == 1 {
                    Orientation::MY
                } else {
                    Orientation::R0
                };
                let sx = dev_x_off + seg as i32 * seg_pitch;
                let total_h = head_l + seg_l + head_l;

                let (bx, by, bw, bh) =
                    orient.transform_rect(0, 0, body_w, total_h, body_w, total_h);
                b.rect(&ly.poly, sx + bx, by, bw, bh)?;

                let cy_top = head_l / 2 - ct / 2;
                let cy_bot = head_l + seg_l + head_l / 2 - ct / 2;
                let cx = body_w / 2 - ct / 2;

                let (tx, ty, _, _) = orient.transform_rect(cx, cy_top, ct, ct, body_w, total_h);
                b.rect(&ly.licon, sx + tx, ty, ct, ct)?;
                b.rect(&ly.li, sx + tx, ty, ct, ct)?;

                let (bx2, by2, _, _) = orient.transform_rect(cx, cy_bot, ct, ct, body_w, total_h);
                b.rect(&ly.licon, sx + bx2, by2, ct, ct)?;
                b.rect(&ly.li, sx + bx2, by2, ct, ct)?;

                let ext = 250;
                if seg == 0 {
                    b.rect(&ly.li, sx + tx, ty - ext, ct, ext + ct)?;
                    b.pin(&format!("{}:P", dev.name), &ly.li, sx + tx, ty - ext, ct, ct)?;
                }
                if seg == n_segments - 1 {
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
        }

        Ok(())
    }
}

/// Fold-count domain: aspect-driven default, then a shallower even fold.
fn feasible_segments(dev: &DeviceRecord, pdk: &Pdk) -> Vec<i32> {
    let aspect = f64::from(dev.l) / f64::from(dev.w.max(1));
    if aspect <= pdk.res_serpentine_aspect {
        return vec![1];
    }
    #[allow(clippy::cast_possible_truncation)]
    let base = ((f64::from(dev.l) / f64::from(pdk.res_min_segment)).ceil() as i32).max(1);
    let evenize = |n: i32| if n <= 1 { 2 } else { n + n % 2 };
    let mut opts = vec![evenize(base)];
    let half = evenize(base / 2);
    if half < opts[0] && half >= 2 {
        opts.push(half);
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use substrate3::{CellBuilder, MatchingTier};

    fn test_deck() -> substrate3::Deck {
        crate::test_util::deck_from_layers(&[
            ("poly", 66, 20),
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
