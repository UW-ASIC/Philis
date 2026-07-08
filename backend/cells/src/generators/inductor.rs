//! Inductor cell generator: rectangular-approximated spiral.

use substrate3::{CellBuilder, CellError, DeviceType, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

/// Inductor spec — single variant per device group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InductorSpec;

impl CellSpec for InductorSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        vec![InductorSpec]
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let outer_d = devices[0].l.max(pdk.ind_min_diameter);
        (outer_d, outer_d)
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
        b.set_device_type(DeviceType::Bjt); // ponytail: no Inductor in DeviceType yet
        b.set_pattern(PatternType::Single);

        let ref_dev = &devices[0];
        let trace_w = ref_dev.w.max(pdk.ind_min_trace);
        let outer_d = ref_dev.l.max(pdk.ind_min_diameter);
        let n_turns = ref_dev.nf.max(1) as i32;
        let spacing = trace_w;

        b.set_electrical(trace_w, outer_d, n_turns as u16, trace_w);

        // ponytail: rectangular approximation of octagonal spiral;
        // DRC-clean and placement-correct, upgrade to polygon() for EM accuracy
        let ly = &pdk.layers;
        let metal: &str = &ly.met1;
        let ct = pdk.contact;

        let mut ring_outer = outer_d;
        for turn in 0..n_turns {
            if ring_outer <= 2 * trace_w {
                break;
            }
            b.rect(metal, 0, 0, ring_outer, trace_w)?;
            b.rect(metal, 0, ring_outer - trace_w, ring_outer, trace_w)?;
            b.rect(metal, 0, trace_w, trace_w, ring_outer - 2 * trace_w)?;

            let right_h = ring_outer - 2 * trace_w;
            if turn == 0 {
                let gap = trace_w + spacing;
                if right_h > gap {
                    b.rect(metal, ring_outer - trace_w, trace_w, trace_w, right_h - gap)?;
                }
            } else {
                b.rect(metal, ring_outer - trace_w, trace_w, trace_w, right_h)?;
            }

            let inset = trace_w + spacing;
            ring_outer -= 2 * inset;
        }

        let center = outer_d / 2;
        b.rect(
            &ly.li,
            center - trace_w / 2,
            center - trace_w / 2,
            trace_w,
            outer_d / 2,
        )?;

        b.pin(
            &format!("{}:P", ref_dev.name),
            metal,
            outer_d - trace_w,
            trace_w,
            ct,
            ct,
        )?;
        b.pin(
            &format!("{}:N", ref_dev.name),
            &ly.li,
            center - ct / 2,
            center - ct / 2,
            ct,
            ct,
        )?;

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

    fn inductor(name: &str, trace_w: i32, outer_d: i32, turns: u16) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Bjt,
            w: trace_w,
            l: outer_d,
            nf: turns,
            multiplier: 1,
            model_name: "ind_01v8".into(),
            terminals: [("P".into(), 1), ("N".into(), 2)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_turn_spiral() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![inductor("L1", 2000, 50_000, 1)];
        let specs = InductorSpec::enumerate(&devices, &pdk);
        assert_eq!(specs.len(), 1);

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert_eq!(out.pins.len(), 2, "P + N");
    }

    #[test]
    fn multi_turn_spiral() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![inductor("L1", 2000, 100_000, 3)];
        let specs = InductorSpec::enumerate(&devices, &pdk);

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).unwrap();
        let out = b.finish();
        assert!(out.store.poly_count() > 10);
    }

    #[test]
    fn drc_clean_inductor() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![inductor("L1", 2000, 50_000, 1)];
        let cell = SpecCell {
            spec: InductorSpec,
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
