//! Capacitor cell generator: MIM, MOM, and single-layer capacitors.

use substrate3::{CellBuilder, CellError, DeviceType, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacitorKind {
    VerticalAcrossLayers,
    HorizontalAcrossLayers,
    VerticalInOneLayer,
}

/// One point in the capacitor variant space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacitorSpec {
    pub kind: CapacitorKind,
    pub pattern: PatternType,
}

impl CellSpec for CapacitorSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        let patterns = if devices.len() > 1 {
            vec![PatternType::Single, PatternType::Cc1d]
        } else {
            vec![PatternType::Single]
        };
        [
            CapacitorKind::VerticalAcrossLayers,
            CapacitorKind::HorizontalAcrossLayers,
            CapacitorKind::VerticalInOneLayer,
        ]
        .into_iter()
        .flat_map(|kind| patterns.iter().map(move |&pattern| CapacitorSpec { kind, pattern }))
        .collect()
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let ref_dev = &devices[0];
        let n_units = ref_dev.nf.max(1) as i32;
        let unit_pitch = ref_dev.w + pdk.plate_spacing;
        let per_dev = n_units * unit_pitch;
        let n = devices.len() as i32;
        let total_w = per_dev * n + pdk.device_gap * (n - 1).max(0);
        (total_w, ref_dev.l)
    }

    fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef> {
        devices
            .iter()
            .flat_map(|d| {
                [
                    PortDef::inout(format!("{}:TOP", d.name)),
                    PortDef::inout(format!("{}:BOT", d.name)),
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
        b.set_device_type(DeviceType::Cap);
        b.set_pattern(self.pattern);

        let ref_dev = &devices[0];
        let plate_w = ref_dev.w;
        let plate_h = ref_dev.l;
        b.set_electrical(plate_w, plate_h, 1, plate_w);

        let ly = &pdk.layers;
        let ct = pdk.contact;
        let plate_spacing = pdk.plate_spacing;
        let enclosure = pdk.via_enclosure;

        let bot_layer: &str = &ly.met1;
        let top_layer: &str = match self.kind {
            CapacitorKind::VerticalInOneLayer => &ly.met1,
            _ => &ly.li,
        };

        for (di, dev) in devices.iter().enumerate() {
            let n_units = dev.nf.max(1) as i32;
            let unit_pitch = plate_w + plate_spacing;

            for u in 0..n_units {
                let ux =
                    di as i32 * (n_units * unit_pitch + pdk.device_gap) + u * unit_pitch;

                b.rect(bot_layer, ux, 0, plate_w, plate_h)?;

                let top_inset = enclosure;
                if plate_w > 2 * top_inset && plate_h > 2 * top_inset {
                    b.rect(
                        top_layer,
                        ux + top_inset,
                        top_inset,
                        plate_w - 2 * top_inset,
                        plate_h - 2 * top_inset,
                    )?;
                }

                let via_pitch = ct + pdk.via_spacing;
                let n_via_x = ((plate_w - 2 * enclosure - ct) / via_pitch + 1).max(1);
                let n_via_y = ((plate_h - 2 * enclosure - ct) / via_pitch + 1).max(1);
                for vx in 0..n_via_x {
                    for vy in 0..n_via_y {
                        let x = ux + enclosure + vx * via_pitch;
                        let y = enclosure + vy * via_pitch;
                        b.rect(&ly.li, x, y, ct, ct)?;
                    }
                }

                if u == 0 {
                    b.pin(
                        &format!("{}:BOT", dev.name),
                        bot_layer,
                        ux,
                        0,
                        plate_w,
                        ct,
                    )?;
                }
                if u == n_units - 1 {
                    b.pin(
                        &format!("{}:TOP", dev.name),
                        top_layer,
                        ux + top_inset,
                        top_inset,
                        (plate_w - 2 * top_inset).max(ct),
                        ct,
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use substrate3::{CellBuilder, MatchingTier};

    fn test_deck() -> substrate3::Deck {
        crate::test_util::deck_from_layers(&[
            ("met1", 68, 20),
            ("li", 67, 20),
        ])
    }

    fn cap(name: &str, w: i32, l: i32, nf: u16) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Cap,
            w,
            l,
            nf,
            multiplier: 1,
            model_name: "mim_cap".into(),
            terminals: [("TOP".into(), 1), ("BOT".into(), 2)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_mim_cap() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![cap("C1", 5000, 5000, 1)];
        let spec = CapacitorSpec {
            kind: CapacitorKind::VerticalAcrossLayers,
            pattern: PatternType::Single,
        };

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        spec.draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pins.len(), 2, "TOP + BOT");
    }

    #[test]
    fn multi_unit_cap() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![cap("C1", 2000, 2000, 4)];
        let spec = CapacitorSpec {
            kind: CapacitorKind::VerticalAcrossLayers,
            pattern: PatternType::Single,
        };

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        spec.draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.store.poly_count() > 8);
    }

    #[test]
    fn drc_clean_mim_cap() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![cap("C1", 5000, 5000, 1)];
        let cell = SpecCell {
            spec: CapacitorSpec {
                kind: CapacitorKind::VerticalAcrossLayers,
                pattern: PatternType::Single,
            },
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
