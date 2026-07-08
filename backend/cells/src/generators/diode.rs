//! Diode cell generator: junction diodes with optional interdigitation.

use substrate3::{CellBuilder, CellError, DeviceType, Orientation, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

/// One point in the diode variant space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiodeSpec {
    pub pattern: PatternType,
}

impl CellSpec for DiodeSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        if devices.len() > 1 {
            vec![
                DiodeSpec { pattern: PatternType::Single },
                DiodeSpec { pattern: PatternType::Interdig },
            ]
        } else {
            vec![DiodeSpec { pattern: PatternType::Single }]
        }
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let w = devices[0].w;
        let l = devices[0].l;
        if self.pattern == PatternType::Interdig && devices.len() > 1 {
            let n = devices.len() as i32;
            (w + (n - 1) * (w + pdk.diode_gap), l)
        } else {
            (w, l)
        }
    }

    fn ports(&self, _devices: &[DeviceRecord]) -> Vec<PortDef> {
        vec![PortDef::inout("A"), PortDef::inout("K")]
    }

    fn draw(
        &self,
        devices: &[DeviceRecord],
        pdk: &Pdk,
        b: &mut CellBuilder,
    ) -> Result<(), CellError> {
        let w = devices[0].w;
        let l = devices[0].l;

        b.set_device_type(DeviceType::Diode);
        b.set_pattern(self.pattern);
        b.set_electrical(w, l, 1, w);

        let ly = &pdk.layers;
        let ct = pdk.contact;

        b.rect(&ly.diff, 0, 0, w, l)?;

        let ay = l / 4 - ct / 2;
        b.rect(&ly.li, w / 2 - ct / 2, ay, ct, ct)?;
        b.pin("A", &ly.li, w / 2 - ct / 2, ay, ct, ct)?;

        let ky = 3 * l / 4 - ct / 2;
        b.rect(&ly.li, w / 2 - ct / 2, ky, ct, ct)?;
        b.pin("K", &ly.li, w / 2 - ct / 2, ky, ct, ct)?;

        if self.pattern == PatternType::Interdig && devices.len() > 1 {
            let pitch = w + pdk.diode_gap;
            for i in 1..devices.len() {
                let orient = if i % 2 == 1 {
                    Orientation::MX
                } else {
                    Orientation::R0
                };
                b.rect(&ly.diff, i as i32 * pitch, 0, w, l)?;
                let (ax, _) = orient.transform(w / 2 - ct / 2, ay, w, l);
                b.rect(&ly.li, i as i32 * pitch + ax, ay, ct, ct)?;
                b.pin("A", &ly.li, i as i32 * pitch + ax, ay, ct, ct)?;
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
            ("diff", 65, 20),
            ("li", 67, 20),
        ])
    }

    fn diode_dev(name: &str) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Diode,
            w: 500,
            l: 1000,
            nf: 1,
            multiplier: 1,
            model_name: "diode_01v8".into(),
            terminals: [("A".into(), 1), ("K".into(), 2)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_diode() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![diode_dev("D1")];
        let specs = DiodeSpec::enumerate(&devices, &pdk);
        assert_eq!(specs.len(), 1);

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).expect("gen");
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pins.len(), 2, "anode + cathode");
    }

    #[test]
    fn drc_clean_diode() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![diode_dev("D1")];
        let specs = DiodeSpec::enumerate(&devices, &pdk);
        let cell = SpecCell {
            spec: specs[0],
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
