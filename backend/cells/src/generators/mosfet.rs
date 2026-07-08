//! MOSFET cell generator: finger decomposition, interdigitation, contacts.

use substrate3::{CellBuilder, CellError, Direction, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

/// One point in the MOSFET variant space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MosfetSpec {
    pub nf: u16,
    pub style: PatternType,
    pub dummies_per_edge: u8,
}

const MAX_VARIANTS: usize = 16;
const DUMMY_OPTIONS: [u8; 2] = [1, 2];

impl CellSpec for MosfetSpec {
    fn enumerate(devices: &[DeviceRecord], pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        let ref_dev = &devices[0];

        let mut specs: Vec<Self> = feasible_styles(devices.len())
            .into_iter()
            .flat_map(|style| {
                feasible_nf(ref_dev, pdk)
                    .into_iter()
                    .flat_map(move |nf| {
                        DUMMY_OPTIONS.iter().map(move |&d| MosfetSpec {
                            nf,
                            style,
                            dummies_per_edge: d,
                        })
                    })
            })
            .collect();

        let mut seen: Vec<((i32, i32), PatternType)> = Vec::new();
        specs.retain(|s| {
            let key = (est_dims(s, devices, pdk), s.style);
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        specs.truncate(MAX_VARIANTS);
        specs
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        est_dims(self, devices, pdk)
    }

    fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef> {
        let mut ports = Vec::new();
        for dev in devices {
            for role in dev.terminals.keys() {
                let dir = if role == "G" || role == "gate" {
                    Direction::In
                } else {
                    Direction::InOut
                };
                ports.push(PortDef {
                    name: format!("{}:{role}", dev.name),
                    direction: dir,
                });
            }
        }
        ports
    }

    fn draw(
        &self,
        devices: &[DeviceRecord],
        pdk: &Pdk,
        b: &mut CellBuilder,
    ) -> Result<(), CellError> {
        let ref_dev = &devices[0];
        b.set_device_type(ref_dev.device_type);
        b.set_pattern(self.style);

        let nf = self.nf.max(1);
        let finger_w = ref_dev.w / i32::from(nf);
        b.set_electrical(ref_dev.w, ref_dev.l, nf, finger_w);
        let m = ref_dev.multiplier.max(1);
        let drawn_fingers = nf * m;

        let ly = &pdk.layers;
        let ct = pdk.contact;
        let poly_ext = pdk.poly_ext;
        let gate_l = ref_dev.l;
        let sd_w = pdk.sd_width.max(430 - gate_l);
        let pitch = sd_w + gate_l + sd_w;

        let sequence = finger_sequence(devices, self.style, drawn_fingers);

        b.rect(&ly.diff, 0, 0, sequence.len() as i32 * pitch, finger_w)?;

        for (idx, dev_name) in sequence.iter().enumerate() {
            let gx = idx as i32 * pitch + sd_w;

            b.rect(&ly.poly, gx, -poly_ext, gate_l, finger_w + 2 * poly_ext)?;

            if *dev_name == "dummy" {
                continue;
            }

            let stub = 200;
            b.rect(&ly.poly, gx, -(poly_ext + stub), gate_l, stub + 30)?;
            b.pin(
                &format!("{dev_name}:G"),
                &ly.poly,
                gx,
                -(poly_ext + stub),
                gate_l,
                stub,
            )?;

            let cy = finger_w / 2 - ct / 2;
            // ponytail: multi-device ABBA needs D on even idx so shared
            // boundaries between different devices are always sources (same net)
            let multi = devices.len() > 1;
            let terminal = match (multi, idx % 2 == 0) {
                (false, true) | (true, false) => "S",
                _ => "D",
            };

            let sx = idx as i32 * pitch + sd_w / 2 - ct / 2;
            b.rect(&ly.li, sx, cy, ct, ct)?;
            b.pin(&format!("{dev_name}:{terminal}"), &ly.li, sx, cy, ct, ct)?;

            let other = if terminal == "S" { "D" } else { "S" };
            let dx = gx + gate_l + sd_w / 2 - ct / 2;
            b.rect(&ly.li, dx, cy, ct, ct)?;
            b.pin(&format!("{dev_name}:{other}"), &ly.li, dx, cy, ct, ct)?;
        }

        for k in 0..i32::from(self.dummies_per_edge) {
            let off = (k + 1) * (gate_l + sd_w);
            let dummy_positions = [
                -off,
                sequence.len() as i32 * pitch + sd_w + k * (gate_l + sd_w),
            ];
            for dx in dummy_positions {
                b.rect(&ly.poly, dx, -poly_ext, gate_l, finger_w + 2 * poly_ext)?;
            }
        }

        Ok(())
    }
}

fn feasible_styles(n_devices: usize) -> Vec<PatternType> {
    let mut styles = vec![PatternType::Single];
    if n_devices == 2 {
        // ponytail: Interdig (ABAB) shorts different drain nets at B-A
        // boundaries on shared diffusion. Only ABBA patterns are safe.
        styles.push(PatternType::Cc1d);
        styles.push(PatternType::Cc2d);
    }
    styles
}

fn feasible_nf(dev: &DeviceRecord, pdk: &Pdk) -> Vec<u16> {
    let mut nfs = vec![dev.nf.max(1)];
    for nf in [1u16, 2, 4, 6, 8, 12, 16] {
        let w_f = dev.w / i32::from(nf);
        if dev.w % i32::from(nf) == 0
            && w_f >= pdk.min_finger_width
            && w_f <= pdk.max_finger_width
        {
            nfs.push(nf);
        }
    }
    nfs.sort_unstable();
    nfs.dedup();
    nfs
}

fn est_dims(spec: &MosfetSpec, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
    let ref_dev = &devices[0];
    let gate_l = ref_dev.l;
    let sd_w = pdk.sd_width.max(430 - gate_l);
    let pitch = 2 * sd_w + gate_l;
    let per_dev = i32::from(spec.nf.max(1)) * i32::from(ref_dev.multiplier.max(1));
    let seq_len = per_dev * devices.len() as i32;
    let dummy_span = i32::from(spec.dummies_per_edge) * (gate_l + sd_w);
    let w = seq_len * pitch + 2 * dummy_span;
    let finger_w = ref_dev.w / i32::from(spec.nf.max(1));
    (w, finger_w + 2 * pdk.poly_ext)
}

fn finger_sequence(
    devices: &[DeviceRecord],
    style: PatternType,
    nf_per_device: u16,
) -> Vec<String> {
    let names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
    let nf = nf_per_device as usize;

    match (style, names.len()) {
        (PatternType::Single, _) => names
            .iter()
            .flat_map(|n| std::iter::repeat_n(n.to_string(), nf))
            .collect(),
        (PatternType::Interdig, 2) => (0..nf)
            .flat_map(|_| [names[0].to_string(), names[1].to_string()])
            .collect(),
        (PatternType::Cc1d | PatternType::Cc2d, 2) => {
            let unit: Vec<String> = vec![
                names[0].into(),
                names[1].into(),
                names[1].into(),
                names[0].into(),
            ];
            unit.into_iter().cycle().take(nf * 2).collect()
        }
        _ => {
            let total: usize = devices.iter().map(|d| d.nf as usize).sum();
            let mut seq = Vec::with_capacity(total);
            let mut counts: Vec<usize> = devices.iter().map(|d| d.nf as usize).collect();
            while seq.len() < total {
                if let Some(i) = counts.iter().position(|&c| c > 0) {
                    seq.push(names[i].to_string());
                    counts[i] -= 1;
                } else {
                    break;
                }
            }
            seq
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::SpecCell;
    use substrate3::{CellBuilder, DeviceType, MatchingTier};

    fn test_deck() -> substrate3::Deck {
        crate::test_util::deck_from_layers(&[
            ("diff", 65, 20),
            ("poly", 66, 20),
            ("li", 67, 20),
        ])
    }

    fn nmos(name: &str, w: i32, l: i32, nf: u16) -> DeviceRecord {
        DeviceRecord {
            name: name.into(),
            device_type: DeviceType::Nmos,
            w,
            l,
            nf,
            multiplier: 1,
            model_name: "nfet_01v8".into(),
            terminals: [("G".into(), 1), ("D".into(), 2), ("S".into(), 3)].into(),
            params: Default::default(),
        }
    }

    #[test]
    fn single_device_single_finger() {
        let deck = test_deck();
        let pdk = Pdk::default();
        let devices = vec![nmos("M1", 420, 150, 1)];
        let specs = MosfetSpec::enumerate(&devices, &pdk);
        assert!(!specs.is_empty());

        let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
        specs[0].draw(&devices, &pdk, &mut b).expect("gen");
        let out = b.finish();
        assert!(out.bbox.width() > 0);
        assert!(!out.pins.is_empty());
    }

    #[test]
    fn diff_pair_has_cc1d() {
        let devices = vec![nmos("MA", 420, 150, 2), nmos("MB", 420, 150, 2)];
        let specs = MosfetSpec::enumerate(&devices, &Pdk::default());
        assert!(
            specs.iter().any(|s| s.style == PatternType::Cc1d),
            "Cc1d variant should exist"
        );
    }

    #[test]
    fn refolds_enumerated() {
        let pdk = Pdk::default();
        let deck = test_deck();
        let devices = vec![nmos("M1", 1680, 150, 1)];
        let specs = MosfetSpec::enumerate(&devices, &pdk);

        let finger_counts: Vec<u16> = specs
            .iter()
            .map(|s| {
                let mut b = CellBuilder::new(&deck, MatchingTier::None, 0);
                s.draw(&devices, &pdk, &mut b).expect("gen");
                b.finish().meta.finger_count
            })
            .collect();
        assert!(finger_counts.contains(&1), "netlist nf variant present");
        assert!(
            finger_counts.contains(&2) && finger_counts.contains(&4),
            "refold variants offered: {finger_counts:?}"
        );
    }

    #[test]
    fn pair_enumerates_multiple_variants() {
        let devices = vec![nmos("MA", 1260, 150, 3), nmos("MB", 1260, 150, 3)];
        let specs = MosfetSpec::enumerate(&devices, &Pdk::default());
        assert!(specs.len() > 1, "should enumerate multiple variants");
    }

    #[test]
    fn finger_sequence_abba() {
        let devices = vec![nmos("A", 420, 150, 2), nmos("B", 420, 150, 2)];
        let seq = finger_sequence(&devices, PatternType::Cc1d, 2);
        assert_eq!(seq, vec!["A", "B", "B", "A"]);
    }

    #[test]
    fn drc_clean_single_nmos() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![nmos("M1", 420, 150, 1)];
        let specs = MosfetSpec::enumerate(&devices, &pdk);
        let cell = SpecCell {
            spec: specs[0],
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
