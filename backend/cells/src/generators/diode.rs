//! Diode cell generator: junction diodes with optional interdigitation.

use crate::{CellBuilder, CellError, DeviceType, Orientation, PatternType, PortDef};

use super::{CellSpec, Pdk};
use crate::device::DeviceRecord;

/// One point in the diode variant space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiodeSpec {
    pub pattern: PatternType,
    pub columns: u16,
}

impl CellSpec for DiodeSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        let patterns = if devices.len() > 1 {
            vec![PatternType::Single, PatternType::Interdig]
        } else {
            vec![PatternType::Single]
        };
        let n = devices.len().max(1) as u16;
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        let mut specs = Vec::new();
        for pattern in patterns {
            for &columns in &cols {
                specs.push(DiodeSpec { pattern, columns });
            }
        }
        specs
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let w = devices[0].w;
        let l = devices[0].l;
        let n = devices.len().max(1) as i32;
        let cols = i32::from(self.columns.max(1)).min(n);
        let rows = (n + cols - 1) / cols;
        (
            cols * w + (cols - 1) * pdk.diode_gap,
            rows * l + (rows - 1) * pdk.diode_gap,
        )
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

        let ay = l / 4 - ct / 2;
        let ky = 3 * l / 4 - ct / 2;
        let cols = usize::from(self.columns.max(1)).min(devices.len().max(1));
        for (i, dev) in devices.iter().enumerate() {
            let ox = (i % cols) as i32 * (w + pdk.diode_gap);
            let oy = (i / cols) as i32 * (l + pdk.diode_gap);
            let orient = if self.pattern == PatternType::Interdig && i % 2 == 1 {
                Orientation::MY
            } else {
                Orientation::R0
            };
            b.rect(&ly.diff, ox, oy, w, l)?;
            for (name, py) in [("A", ay), ("K", ky)] {
                let (px, py, _, _) = orient.transform_rect(w / 2 - ct / 2, py, ct, ct, w, l);
                b.rect(&ly.li, ox + px, oy + py, ct, ct)?;
                b.pin(
                    &format!("{}:{name}", dev.name),
                    &ly.li,
                    ox + px,
                    oy + py,
                    ct,
                    ct,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use crate::{CellBuilder, MatchingTier};

    fn test_deck() -> crate::Deck {
        crate::test_util::deck_from_layers(&[("diff", 65, 20), ("li", 67, 20)])
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
