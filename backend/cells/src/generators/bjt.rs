//! BJT cell generator: NPN and PNP bipolar junction transistors.

use substrate3::{CellBuilder, CellError, DeviceType, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

/// BJT spec — single variant per device group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BjtSpec;

impl CellSpec for BjtSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        vec![BjtSpec]
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let ref_dev = &devices[0];
        let emitter_w = ref_dev.w.max(pdk.bjt_min_emitter_side);
        let emitter_h = ref_dev.l.max(pdk.bjt_min_emitter_side);
        #[allow(clippy::cast_possible_truncation)]
        let base_w = (emitter_w as f64 * pdk.bjt_base_frac).round() as i32;
        #[allow(clippy::cast_possible_truncation)]
        let collector_w = (emitter_w as f64 * pdk.bjt_collector_frac).round() as i32;
        let per_dev = emitter_w + 2 * base_w + 2 * collector_w;
        let n = devices.len() as i32;
        let total_w = per_dev * n + pdk.device_gap * (n - 1).max(0);
        let total_h = emitter_h + 2 * base_w + 2 * collector_w;
        (total_w, total_h)
    }

    fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef> {
        devices
            .iter()
            .flat_map(|d| {
                [
                    PortDef::inout(format!("{}:E", d.name)),
                    PortDef::inout(format!("{}:B", d.name)),
                    PortDef::inout(format!("{}:C", d.name)),
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
        b.set_device_type(DeviceType::Bjt);
        b.set_pattern(PatternType::Single);

        let ref_dev = &devices[0];
        let emitter_w = ref_dev.w.max(pdk.bjt_min_emitter_side);
        let emitter_h = ref_dev.l.max(pdk.bjt_min_emitter_side);

        let max_stripe = pdk.bjt_max_emitter_stripe;
        let n_stripes = ((emitter_w + max_stripe - 1) / max_stripe).max(1);
        let stripe_w = emitter_w / n_stripes;

        #[allow(clippy::cast_possible_truncation)]
        let base_w = (emitter_w as f64 * pdk.bjt_base_frac).round() as i32;
        #[allow(clippy::cast_possible_truncation)]
        let collector_w = (emitter_w as f64 * pdk.bjt_collector_frac).round() as i32;
        let ly = &pdk.layers;
        let ct = pdk.contact;

        b.set_electrical(emitter_w, emitter_h, n_stripes as u16, stripe_w);

        for (di, dev) in devices.iter().enumerate() {
            let dev_off =
                di as i32 * (emitter_w + 2 * base_w + 2 * collector_w + pdk.device_gap);

            let coll_x = dev_off;
            let coll_y = 0;
            let coll_w = emitter_w + 2 * base_w + 2 * collector_w;
            let coll_h = emitter_h + 2 * base_w + 2 * collector_w;
            b.rect(&ly.diff, coll_x, coll_y, coll_w, coll_h)?;
            b.pin(&format!("{}:C", dev.name), &ly.diff, coll_x, coll_y, ct, ct)?;

            let poly_ext = pdk.poly_ext;
            b.rect(
                &ly.poly,
                coll_x - poly_ext,
                coll_y - poly_ext,
                coll_w + 2 * poly_ext,
                coll_h + 2 * poly_ext,
            )?;
            b.pin(&format!("{}:B", dev.name), &ly.poly, coll_x, coll_y, ct, ct)?;

            let emitter_x = dev_off + collector_w + base_w;
            let emitter_y = collector_w + base_w;
            let stripe_pitch = stripe_w + pdk.bjt_stripe_gap.min(stripe_w / 4);
            for s in 0..n_stripes {
                let sx = emitter_x + s * stripe_pitch;
                b.rect(&ly.diff, sx, emitter_y, stripe_w, emitter_h)?;
                b.rect(
                    &ly.li,
                    sx + stripe_w / 2 - ct / 2,
                    emitter_y + emitter_h / 2 - ct / 2,
                    ct,
                    ct,
                )?;
            }
            b.pin(
                &format!("{}:E", dev.name),
                &ly.li,
                emitter_x + stripe_w / 2 - ct / 2,
                emitter_y + emitter_h / 2 - ct / 2,
                ct,
                ct,
            )?;

            if !is_pnp(devices) {
                let well_enc = pdk.well_enclosure;
                b.rect(
                    &ly.nwell,
                    coll_x - well_enc,
                    coll_y - well_enc,
                    coll_w + 2 * well_enc,
                    coll_h + 2 * well_enc,
                )?;
            }
        }

        Ok(())
    }
}

fn is_pnp(devices: &[DeviceRecord]) -> bool {
    devices[0].model_name.to_ascii_lowercase().contains("pnp")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use substrate3::{CellBuilder, MatchingTier};

    fn test_deck() -> substrate3::Deck {
        crate::test_util::deck_from_layers(&[
            ("diff", 65, 20),
            ("poly", 66, 20),
            ("li", 67, 20),
            ("nwell", 64, 20),
        ])
    }

    fn npn(name: &str) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Bjt,
            w: 1000,
            l: 1000,
            nf: 1,
            multiplier: 1,
            model_name: "npn_01v8".into(),
            terminals: [("E".into(), 1), ("B".into(), 2), ("C".into(), 3)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_npn() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![npn("Q1")];
        let specs = BjtSpec::enumerate(&devices, &pdk);
        assert_eq!(specs.len(), 1);

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pins.len(), 3, "E + B + C");
        let nwell_id = deck.layers.id("nwell").unwrap();
        let has_nwell = (0..out.store.poly_count()).any(|i| out.store.poly_layer[i] == nwell_id);
        assert!(has_nwell, "NPN needs nwell");
    }

    #[test]
    fn drc_clean_npn() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![npn("Q1")];
        let cell = SpecCell {
            spec: BjtSpec,
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
