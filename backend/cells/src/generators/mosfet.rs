//! MOSFET cell generator: finger decomposition, interdigitation, contacts.

use substrate3::{CellBuilder, CellError, DeviceType, Direction, MatchingTier, MatchingType, PatternType, PortDef};

use crate::device::DeviceRecord;
use super::{CellSpec, Pdk};

/// One point in the MOSFET variant space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MosfetSpec {
    pub nf: u16,
    pub style: PatternType,
    pub dummies_per_edge: u8,
    /// Sizing axis: Mirror (L-dominated, current mirrors) vs Cross (W·L, diff pairs).
    /// ponytail: was hardcoded Cross; now threaded from constraints.
    pub match_kind: Option<MatchingType>,
    /// Per-device finger counts from unitization constraint (item 1.7).
    /// When set, overrides uniform nf: devices get `dev_nf[i]` fingers
    /// of width `unit_w` each, preserving target ratios.
    pub unit_nf: Option<Vec<u16>>,
    /// Unit finger width in nm (from unitization constraint).
    pub unit_w: Option<i32>,
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
                            match_kind: None,
                            unit_nf: None,
                            unit_w: None,
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

        // ponytail: item 1.7 — unit finger from unitization constraint.
        // When unit_w is set, all devices share the same finger width;
        // per-device finger counts come from unit_nf (ratio-preserving).
        let nf = self.nf.max(1);
        let finger_w = self.unit_w.unwrap_or(ref_dev.w / i32::from(nf));
        b.set_electrical(ref_dev.w, ref_dev.l, nf, finger_w);
        let m = ref_dev.multiplier.max(1);
        let drawn_fingers = nf * m;

        let ly = &pdk.layers;
        let ct = pdk.contact;
        let poly_ext = pdk.poly_ext;
        let gate_l = ref_dev.l;
        let sd_w = pdk.sd_width.max(430 - gate_l);
        // ponytail: pitch floor from met1.2 spacing — mcon + 2*m1_enc + met1_space
        // kills residual m1.2 DRC violations by construction
        let m1_pitch = pdk.mcon_size + 2 * pdk.m1_enc + pdk.met1_space;
        let pitch = (sd_w + gate_l + sd_w).max(m1_pitch);

        let sequence = finger_sequence(devices, self.style, drawn_fingers);

        // ponytail: LOD moat extension — extend diff past outer gates by tier-keyed
        // distance so SA/SB diagnostics == emitted geometry (AOAL ch13 13.2.2 Rule 12)
        let moat_ext = match b.tier() {
            MatchingTier::Exceptional => pdk.lod_moat_ext_nm[1],
            MatchingTier::Moderate => pdk.lod_moat_ext_nm[0],
            _ => 0,
        };
        let diff_x_start = -moat_ext;
        let diff_x_end = sequence.len() as i32 * pitch + moat_ext;
        b.rect(&ly.diff, diff_x_start, 0, diff_x_end - diff_x_start, finger_w)?;

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

        // ponytail: dummy gates tied to supply — GND for NMOS, VDD for PMOS
        // (AOAL ch13 13.2.2 Rule 12: dummy must sit in cutoff)
        let supply_net = match ref_dev.device_type {
            DeviceType::Nmos | DeviceType::Ncap => "GND",
            _ => "VDD",
        };
        for k in 0..i32::from(self.dummies_per_edge) {
            let off = (k + 1) * (gate_l + sd_w);
            let dummy_positions = [
                -off,
                sequence.len() as i32 * pitch + sd_w + k * (gate_l + sd_w),
            ];
            for dx in dummy_positions {
                b.rect(&ly.poly, dx, -poly_ext, gate_l, finger_w + 2 * poly_ext)?;
                // Contact stack: licon → li → mcon → met1 on dummy poly endcap
                let cx = dx + gate_l / 2 - ct / 2;
                let cy = -(poly_ext / 2) - ct / 2;
                b.rect(&ly.licon, cx, cy, ct, ct)?;
                // li encloses mcon (sky130: 30nm enclosure)
                let li_enc = 30;
                let li_x = cx - li_enc;
                let li_y = cy - li_enc;
                let li_sz = ct + 2 * li_enc;
                b.rect(&ly.li, li_x, li_y, li_sz, li_sz)?;
                b.rect(&ly.mcon, cx, cy, ct, ct)?;
                // met1 strap — sized to meet min_area (sky130: 83000 nm²)
                let m1_w = ct + 2 * pdk.m1_enc;
                let m1_min_area = 83_000;
                let m1_h = (m1_min_area / m1_w).max(ct + 2 * pdk.m1_enc);
                let m1_x = cx - pdk.m1_enc;
                let m1_y = cy - (m1_h - ct) / 2;
                b.rect(&ly.met1, m1_x, m1_y, m1_w, m1_h)?;
                b.pin(
                    &format!("dummy:{supply_net}"),
                    &ly.met1,
                    m1_x, m1_y, m1_w, m1_h,
                )?;
            }
        }

        // ── WPE clearance: nwell + bbox inflation (item 1.9) ──
        // AOAL ch13 Rule 19 / FOLD 6.6.3 — inflate nwell rect and cell bbox
        // by tier-keyed gate-to-well-edge clearance.
        let wpe_halo = match b.tier() {
            MatchingTier::Exceptional => pdk.wpe_clearance_nm[2],
            MatchingTier::Moderate => pdk.wpe_clearance_nm[1],
            MatchingTier::Minimal => pdk.wpe_clearance_nm[0],
            MatchingTier::None => 0,
        };
        let is_pmos = matches!(ref_dev.device_type, DeviceType::Pmos | DeviceType::Pcap);
        if is_pmos {
            // Emit nwell enclosing diffusion + WPE halo
            let nw_enc = pdk.nwell_diff_enc + wpe_halo;
            b.rect(
                &ly.nwell,
                diff_x_start - nw_enc,
                -nw_enc,
                (diff_x_end - diff_x_start) + 2 * nw_enc,
                finger_w + 2 * nw_enc,
            )?;
        }
        // For both NMOS and PMOS: inflate bbox by WPE halo so placement
        // accounts for well-edge clearance requirement.
        if wpe_halo > 0 {
            let bb = b.compute_bbox();
            let pad = wpe_halo;
            // ponytail: transparent rect on diff to expand bbox — placement
            // reads bbox, not layer-specific bounds
            let _ = b.rect(&ly.diff, bb.xmin - pad, bb.ymin - pad, 0, 0);
            let _ = b.rect(&ly.diff, bb.xmax + pad, bb.ymax + pad, 0, 0);
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
    let m1_pitch = pdk.mcon_size + 2 * pdk.m1_enc + pdk.met1_space;
    let pitch = (2 * sd_w + gate_l).max(m1_pitch);
    let per_dev = i32::from(spec.nf.max(1)) * i32::from(ref_dev.multiplier.max(1));
    let seq_len = per_dev * devices.len() as i32;
    let dummy_span = i32::from(spec.dummies_per_edge) * (gate_l + sd_w);
    // ponytail: moat extension (item 1.10) — not tier-aware in estimate,
    // use moderate as conservative default for area ranking
    let moat = pdk.lod_moat_ext_nm[0];
    let w = seq_len * pitch + 2 * dummy_span + 2 * moat;
    let finger_w = ref_dev.w / i32::from(spec.nf.max(1));
    (w, finger_w + 2 * pdk.poly_ext)
}

fn finger_sequence(
    devices: &[DeviceRecord],
    style: PatternType,
    nf_per_device: u16,
) -> Vec<String> {
    finger_sequence_ext(devices, style, nf_per_device, None)
}

/// Extended finger sequence supporting per-device finger counts from unitization.
fn finger_sequence_ext(
    devices: &[DeviceRecord],
    style: PatternType,
    nf_per_device: u16,
    unit_nf: Option<&[u16]>,
) -> Vec<String> {
    let names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();

    // ponytail: item 1.7 — when unit_nf is set, devices have different
    // finger counts. Route through greedy_centroid for ratio-preserving layout.
    if let Some(per_dev) = unit_nf {
        if per_dev.len() == names.len() && per_dev.iter().any(|&n| n != per_dev[0]) {
            let counts: Vec<usize> = per_dev.iter().map(|&n| n as usize).collect();
            return greedy_centroid_mosfet(&names, &counts);
        }
    }

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

/// Greedy centroid interleave for MOSFET fingers with unequal counts.
/// Same algorithm as resistor.rs greedy_centroid_sequence but returns device names.
fn greedy_centroid_mosfet(names: &[&str], counts: &[usize]) -> Vec<String> {
    let total: usize = counts.iter().sum();
    if total == 0 {
        return vec![];
    }
    let mut remaining: Vec<usize> = counts.to_vec();
    let mut seq = vec![0usize; total];
    let mut lo = 0usize;
    let mut hi = total - 1;

    while lo <= hi {
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
            if remaining[pick] > 0 {
                seq[hi] = pick;
                remaining[pick] -= 1;
            } else {
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
            if hi == 0 { break; }
            hi -= 1;
        }
        lo += 1;
    }
    seq.into_iter().map(|i| names[i].to_string()).collect()
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
            ("licon", 66, 44),
            ("mcon", 67, 44),
            ("met1", 68, 20),
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
            spec: specs[0].clone(),
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
