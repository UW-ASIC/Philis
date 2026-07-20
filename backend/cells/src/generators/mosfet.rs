//! MOSFET cell generator: finger decomposition, interdigitation, contacts.

use crate::{
    CellBuilder, CellError, DeviceType, Direction, MatchingTier, MatchingType, PatternType, PortDef,
};

use super::{CellSpec, Pdk};
use crate::device::DeviceRecord;

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
                feasible_nf(ref_dev, pdk).into_iter().flat_map(move |nf| {
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
        // ponytail: pitch floor from met1.2 spacing — mcon + 2*m1_enc + met1_space
        // kills residual m1.2 DRC violations by construction. sd_w floor keeps
        // the within-finger S->D pad centers (sd_w + gate_l apart) at that
        // same met1 pad pitch.
        let m1_pitch = pdk.mcon_size + 2 * pdk.m1_enc + pdk.met1_space;
        let sd_w = pdk.sd_width.max(m1_pitch - gate_l);
        let pitch = (sd_w + gate_l + sd_w).max(m1_pitch);

        let sequence =
            finger_sequence_ext(devices, self.style, drawn_fingers, self.unit_nf.as_deref());

        // ponytail: LOD moat extension — extend diff past outer gates by tier-keyed
        // distance so SA/SB diagnostics == emitted geometry (AOAL ch13 13.2.2 Rule 12)
        let moat_ext = match b.tier() {
            MatchingTier::Exceptional => pdk.lod_moat_ext_nm[1],
            MatchingTier::Moderate => pdk.lod_moat_ext_nm[0],
            _ => 0,
        };
        let diff_x_start = -moat_ext;
        let diff_x_end = sequence.len() as i32 * pitch + moat_ext;
        b.rect(
            &ly.diff,
            diff_x_start,
            0,
            diff_x_end - diff_x_start,
            finger_w,
        )?;

        for (idx, dev_name) in sequence.iter().enumerate() {
            let gx = idx as i32 * pitch + sd_w;

            b.rect(&ly.poly, gx, -poly_ext, gate_l, finger_w + 2 * poly_ext)?;

            if *dev_name == "dummy" {
                continue;
            }

            // Gate met1 pad row (drawn downstream, mcon + 2*m1_enc wide, centered
            // on the stub) must clear the S/D pad row at cy by met1_space, or
            // small-finger_w cells get diagonal met1 gaps < min_spacing.
            let m1_clear = pdk.mcon_size + 2 * pdk.m1_enc + pdk.met1_space;
            let stub = 200.max(2 * (m1_clear - finger_w / 2 - poly_ext));
            b.rect(&ly.poly, gx, -(poly_ext + stub), gate_l, stub + 30)?;
            b.pin(
                &format!("{dev_name}:G"),
                &ly.poly,
                gx,
                -(poly_ext + stub),
                gate_l,
                stub,
            )?;
        }

        // One contact pad per S/D diffusion region. The old per-finger
        // left+right pads drew TWO pads in every shared interior region,
        // only sd_w - ct (80nm) apart — li/licon/mcon min-spacing
        // violations by construction and duplicated cuts.
        // ponytail: multi-device ABBA needs D on even fingers so shared
        // boundaries between different devices are always sources (same net).
        let multi = devices.len() > 1;
        let term_of = |idx: i32| -> &'static str {
            match (multi, idx % 2 == 0) {
                (false, true) | (true, false) => "S",
                _ => "D",
            }
        };
        let n_fingers = sequence.len() as i32;
        let cy = finger_w / 2 - ct / 2;
        // center x of S/D region r (0 = left of first gate, n_fingers = right
        // of last); shared between contact pads and the bulk-tap strap.
        let region_cx = |r: i32| -> i32 {
            if r == 0 {
                sd_w / 2
            } else if r == n_fingers {
                (n_fingers - 1) * pitch + sd_w + gate_l + sd_w / 2
            } else {
                // center of the shared region between gates r-1 and r
                (2 * r - 1) * pitch / 2 + sd_w + gate_l / 2
            }
        };
        for r in 0..=n_fingers {
            let left = (r > 0)
                .then(|| sequence[(r - 1) as usize].as_str())
                .filter(|d| *d != "dummy");
            let right = (r < n_fingers)
                .then(|| sequence[r as usize].as_str())
                .filter(|d| *d != "dummy");
            if left.is_none() && right.is_none() {
                continue;
            }
            let cx = region_cx(r);
            let px = cx - ct / 2;
            b.rect(&ly.li, px, cy, ct, ct)?;
            let right_pin = right.map(|d| format!("{d}:{}", term_of(r)));
            let left_pin = left.map(|d| {
                let t = if term_of(r - 1) == "S" { "D" } else { "S" };
                format!("{d}:{t}")
            });
            if let Some(name) = &right_pin {
                b.pin(name, &ly.li, px, cy, ct, ct)?;
            }
            // Same device + same terminal on both sides (interior region of
            // one device) is one electrical pin — register it once.
            if let Some(name) = &left_pin {
                if right_pin.as_deref() != Some(name.as_str()) {
                    b.pin(name, &ly.li, px, cy, ct, ct)?;
                }
            }
        }

        // ── ERC missing_tie coverage: li chunk chains near big-diff corners ──
        // The check measures diff-corner -> li bbox-center distance
        // (deck.erc.tie_max_dist_nm) for diffusions larger than tie_max², so
        // long/tall bars need contact centers along their end columns (and
        // along LOD-moat edges), not just one mid-height pad.
        let tie_max = b.deck().erc.tie_max_dist_nm.max(1);
        let chunk = (tie_max / 2).max(ct);
        let big_diff = i64::from(diff_x_end - diff_x_start) * i64::from(finger_w)
            >= i64::from(tie_max) * i64::from(tie_max);
        let end_cols = [
            sd_w / 2,
            (n_fingers - 1) * pitch + sd_w + gate_l + sd_w / 2,
        ];
        if big_diff {
            // Full-height chains on the two end S/D columns; they overlap the
            // existing pads, so everything stays one conductor per region.
            for ecx in end_cols {
                super::li_chain(b, &ly.li, ecx - ct / 2, 0, ct, finger_w, chunk)?;
            }
            // Moat corners sit past the outer gates: run edge strips out to
            // the diff ends, merged into the end columns (same S/D net).
            if moat_ext > 0 {
                for ey in [0, finger_w - ct] {
                    super::li_chain(
                        b,
                        &ly.li,
                        diff_x_start,
                        ey,
                        end_cols[0] - diff_x_start + ct,
                        ct,
                        chunk,
                    )?;
                    super::li_chain(
                        b,
                        &ly.li,
                        end_cols[1] - ct / 2,
                        ey,
                        diff_x_end - end_cols[1] + ct / 2,
                        ct,
                        chunk,
                    )?;
                }
            }
        }

        // ── Dummy gates (AOAL ch13 13.2.2 Rule 12: dummies sit in cutoff) ──
        // The tie-off rises from a top poly stub into the bulk tap rail below,
        // putting dummy gates at the bulk potential (GND for NMOS, VDD for
        // PMOS). The old bottom-side licon+li pad was a floating li island —
        // under the flow's blanket nwell it tripped ERC soft_connection on
        // every PMOS cell.
        let li_enc = 30;
        let tap_y0 = finger_w + 580; // diff->tap: DIFF.3 (270) + dummy-cut clearance
        let tap_h = ct + 80; // LICON.5: diff extends 40 past each cut edge
        for k in 0..i32::from(self.dummies_per_edge) {
            let off = (k + 1) * (gate_l + sd_w);
            let dummy_positions = [
                -off,
                sequence.len() as i32 * pitch + sd_w + k * (gate_l + sd_w),
            ];
            for dx in dummy_positions {
                b.rect(&ly.poly, dx, -poly_ext, gate_l, finger_w + 2 * poly_ext)?;
                // Top stub hosting the tie-off cut; it stops short of the tap
                // diff so the cut never clips diff (LICON.5) and the stub
                // never crosses it (no phantom gate).
                let licon_y = finger_w + 240;
                let stub_top = licon_y + ct + 40;
                // overlap the gate bar by 10nm so extraction merges the rects
                b.rect(
                    &ly.poly,
                    dx,
                    finger_w + poly_ext - 10,
                    gate_l,
                    stub_top - (finger_w + poly_ext - 10),
                )?;
                let cx = dx + gate_l / 2 - ct / 2;
                b.rect(&ly.licon, cx, licon_y, ct, ct)?;
                // li riser: encloses the cut (30nm) and overlaps the tap rail
                b.rect(
                    &ly.li,
                    cx - li_enc,
                    licon_y - li_enc,
                    ct + 2 * li_enc,
                    (tap_y0 + 60) - (licon_y - li_enc),
                )?;
            }
        }

        // ── Bulk tap strip: n+ tap in the nwell for PMOS (ERC floating_well),
        // p+ substrate tie for NMOS. The rail li ties into a same-net S/D
        // column when the schematic bulk equals a local terminal net (the
        // B==rail==source case in every fixture); routing it as a pin instead
        // would draw met1 on bulk-only nets (chain4: VSS has *no* S/D/G pins),
        // which ERC unconnected_pin/esd_missing cannot associate with a device.
        let is_pmos = matches!(ref_dev.device_type, DeviceType::Pmos | DeviceType::Pcap);
        let dpe = i32::from(self.dummies_per_edge);
        let tap_x0 = diff_x_start.min(-(dpe * (gate_l + sd_w)));
        let tap_x1 =
            diff_x_end.max(n_fingers * pitch + sd_w + (dpe - 1) * (gate_l + sd_w) + gate_l);
        let tap_w = tap_x1 - tap_x0;
        // Tap diff + implant are optional layers (geometry-only test decks
        // omit them); the rail li + pins are always drawn.
        let _ = b.rect(&ly.tap, tap_x0, tap_y0, tap_w, tap_h);
        let imp = if is_pmos { &ly.nsdm } else { &ly.psdm };
        let imp_enc = 65; // NSDM.1/PSDM.1 min width 380 = tap_h(250) + 2*65
        let _ = b.rect(
            imp,
            tap_x0 - imp_enc,
            tap_y0 - imp_enc,
            tap_w + 2 * imp_enc,
            tap_h + 2 * imp_enc,
        );
        // licon array (guard-ring recipe): safe now that no routed pin lands
        // on the rail — the flow's per-pin licon cuts stay at S/D and gate
        // pads, LICON.2 clear of this row.
        let mut lx = tap_x0 + 40;
        while lx + ct + 40 <= tap_x1 {
            let _ = b.rect(&ly.licon, lx, tap_y0 + 40, ct, ct);
            lx += pdk.guard_licon_pitch;
        }
        // Rail li as a chunk chain: doubles as missing_tie contact centers
        // for the tap diff itself on very wide cells.
        super::li_chain(b, &ly.li, tap_x0, tap_y0, tap_w, tap_h, chunk)?;
        // Strap the rail into the first S/D region whose terminal net equals
        // the cell bulk net: the tap then joins a real device net (what ERC
        // soft_connection / unconnected_pin key on) without any routing.
        // ponytail: cells whose bulk matches no local terminal keep a
        // floating tap (harmless for NMOS; a PMOS cell with an isolated bulk
        // net would need flow-side bulk routing to fully tie its well).
        let cell_bulk = devices[0].terminals.get("B").copied();
        let same_bulk = devices
            .iter()
            .all(|d| d.terminals.get("B").copied() == cell_bulk);
        let mut strap_x: Option<i32> = None;
        if let (Some(bulk), true) = (cell_bulk, same_bulk) {
            'find: for r in 0..=n_fingers {
                let right = (r < n_fingers)
                    .then(|| (sequence[r as usize].as_str(), term_of(r)));
                let left = (r > 0).then(|| {
                    let t = if term_of(r - 1) == "S" { "D" } else { "S" };
                    (sequence[(r - 1) as usize].as_str(), t)
                });
                for (name, term) in [right, left].into_iter().flatten() {
                    if name == "dummy" {
                        continue;
                    }
                    let net = devices
                        .iter()
                        .find(|d| d.name == name)
                        .and_then(|d| d.terminals.get(term));
                    if net == Some(&bulk) {
                        strap_x = Some(region_cx(r));
                        break 'find;
                    }
                }
            }
        }
        if let Some(sx) = strap_x {
            // vertical li strap: S/D pad (device net) up into the rail li
            b.rect(&ly.li, sx - ct / 2, cy, ct, (tap_y0 + 60) - cy)?;
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
        if is_pmos {
            // Emit nwell enclosing diffusion + tap strip + WPE halo, clamped
            // up to the deck's nwell min-width rule (NWELL.1): a
            // minimum-height finger plus 2x180 enclosure is only 780nm,
            // under the 840nm rule.
            let nw_enc = pdk.nwell_diff_enc + wpe_halo;
            let nwell_id = b.resolve(&ly.nwell)?;
            let nw_min = b
                .deck()
                .drc_rules
                .iter()
                .find_map(|r| match r {
                    gdsverify::DrcRuleParam::MinWidth { layer, min, .. }
                        if *layer == nwell_id =>
                    {
                        Some(*min)
                    }
                    _ => None,
                })
                .unwrap_or(0);
            let mut w = tap_w + 2 * nw_enc;
            let mut h = (tap_y0 + tap_h) + 2 * nw_enc;
            let mut x = tap_x0 - nw_enc;
            let mut y = -nw_enc;
            if w < nw_min {
                x -= (nw_min - w) / 2;
                w = nw_min;
            }
            if h < nw_min {
                y -= (nw_min - h) / 2;
                h = nw_min;
            }
            b.rect(&ly.nwell, x, y, w, h)?;
        }
        // For both NMOS and PMOS: inflate bbox by WPE halo so placement
        // accounts for well-edge clearance requirement. Bbox-only pad: marker
        // rects on a real layer would trip that layer's min-width rule.
        b.pad_bbox(wpe_halo);

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
        if dev.w % i32::from(nf) == 0 && w_f >= pdk.min_finger_width && w_f <= pdk.max_finger_width
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
    let m1_pitch = pdk.mcon_size + 2 * pdk.m1_enc + pdk.met1_space;
    let sd_w = pdk.sd_width.max(m1_pitch - gate_l);
    let pitch = (2 * sd_w + gate_l).max(m1_pitch);
    let per_dev = i32::from(spec.nf.max(1)) * i32::from(ref_dev.multiplier.max(1));
    let seq_len = per_dev * devices.len() as i32;
    let dummy_span = i32::from(spec.dummies_per_edge) * (gate_l + sd_w);
    // ponytail: moat extension (item 1.10) — not tier-aware in estimate,
    // use moderate as conservative default for area ranking
    let moat = pdk.lod_moat_ext_nm[0];
    let w = seq_len * pitch + 2 * dummy_span + 2 * moat;
    let finger_w = ref_dev.w / i32::from(spec.nf.max(1));
    // +900: bulk tap strip zone above the fingers (580 gap + 250 tap + enc)
    (w, finger_w + 2 * pdk.poly_ext + 900)
}

#[cfg(test)]
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
            if hi == 0 {
                break;
            }
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
    use crate::{CellBuilder, DeviceType, MatchingTier};

    fn test_deck() -> crate::Deck {
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
