//! BJT cell generator: NPN and PNP bipolar junction transistors.

use crate::{CellBuilder, CellError, DeviceType, PatternType, PortDef};

use super::{CellSpec, Pdk};
use crate::device::DeviceRecord;

/// BJT group aspect variant. Individual emitter geometry is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BjtSpec {
    pub columns: u16,
}

impl CellSpec for BjtSpec {
    fn enumerate(devices: &[DeviceRecord], _pdk: &Pdk) -> Vec<Self> {
        if devices.is_empty() {
            return vec![];
        }
        let n = devices.len().max(1) as u16;
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        cols.into_iter()
            .map(|columns| BjtSpec { columns })
            .collect()
    }

    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) {
        let ref_dev = &devices[0];
        let emitter_w = ref_dev.w.max(pdk.bjt_min_emitter_side);
        let emitter_h = ref_dev.l.max(pdk.bjt_min_emitter_side);
        #[allow(clippy::cast_possible_truncation)]
        let base_w = ((emitter_w as f64 * pdk.bjt_base_frac).round() as i32).max(pdk.contact);
        #[allow(clippy::cast_possible_truncation)]
        let collector_w =
            ((emitter_w as f64 * pdk.bjt_collector_frac).round() as i32).max(2 * pdk.contact);
        let per_dev = emitter_w + 2 * base_w + 2 * collector_w;
        let n = devices.len() as i32;
        let cols = i32::from(self.columns.max(1)).min(n.max(1));
        let rows = (n + cols - 1) / cols;
        let total_w = per_dev * cols + pdk.device_gap * (cols - 1).max(0);
        let cell_h = emitter_h + 2 * base_w + 2 * collector_w;
        let total_h = cell_h * rows + pdk.device_gap * (rows - 1).max(0);
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

        // Terminals must extract as three distinct nets: collector ring and
        // emitter block are separate diff shapes (gap = base_w), the base is
        // a poly ring overlapping both by `ct` — never covering the pin
        // landings, so routed contact stacks cannot short B to E or C.
        let collector_w = collector_w.max(2 * ct);
        let base_w = base_w.max(ct);
        for (di, dev) in devices.iter().enumerate() {
            let coll_w = emitter_w + 2 * base_w + 2 * collector_w;
            let coll_h = emitter_h + 2 * base_w + 2 * collector_w;
            let cols = usize::from(self.columns.max(1)).min(devices.len().max(1));
            let dev_x = (di % cols) as i32 * (coll_w + pdk.device_gap);
            let dev_y = (di / cols) as i32 * (coll_h + pdk.device_gap);

            // Collector: diff ring around the perimeter. Side bands run full
            // height so they area-overlap the top/bottom bands — edge-touching
            // rects do not merge into one extracted net.
            let (cx, cy) = (dev_x, dev_y);
            b.rect(&ly.diff, cx, cy, coll_w, collector_w)?;
            b.rect(&ly.diff, cx, cy + coll_h - collector_w, coll_w, collector_w)?;
            b.rect(&ly.diff, cx, cy, collector_w, coll_h)?;
            b.rect(&ly.diff, cx + coll_w - collector_w, cy, collector_w, coll_h)?;
            // C pin near the bottom-right corner, far from the B bar landing,
            // inset by the licon.5 diff-past-cut margin so the routed contact
            // stack keeps 40nm diff enclosure on every side.
            let cut_enc = 40;
            b.pin(
                &format!("{}:C", dev.name),
                &ly.diff,
                cx + coll_w - ct - cut_enc,
                cy + cut_enc,
                ct,
                ct,
            )?;

            // Emitter: diff stripes tied by a same-layer bridge bar (one net,
            // no in-cell cuts needed).
            let emitter_x = dev_x + collector_w + base_w;
            let emitter_y = dev_y + collector_w + base_w;
            let stripe_pitch = stripe_w + pdk.bjt_stripe_gap.min(stripe_w / 4);
            let emitter_span = (n_stripes - 1) * stripe_pitch + stripe_w;
            for s in 0..n_stripes {
                b.rect(&ly.diff, emitter_x + s * stripe_pitch, emitter_y, stripe_w, emitter_h)?;
            }
            if n_stripes > 1 {
                b.rect(
                    &ly.diff,
                    emitter_x,
                    emitter_y + emitter_h / 2 - ct / 2,
                    emitter_span,
                    ct,
                )?;
            }
            // E pin at the right end of the block, clear of the base bar,
            // inset by the licon.5 margin from the diff edge.
            b.pin(
                &format!("{}:E", dev.name),
                &ly.diff,
                emitter_x + emitter_span - ct - cut_enc,
                emitter_y + emitter_h / 2 - ct / 2,
                ct,
                ct,
            )?;

            // Base: one vertical poly bar crossing the full cell. The deck's
            // min_extension (gate endcap) demands poly fully cross any diff it
            // touches, so a partial-overlap ring is illegal — a through-bar
            // over the leftmost emitter stripe crosses every diff band with
            // >= `ext` protrusion and overlaps both emitter and collector for
            // recognition. B pin lands on the poly-only bottom protrusion.
            let ext = ct.max(130);
            b.rect(&ly.poly, emitter_x, cy - ext, ct, coll_h + 2 * ext)?;
            b.pin(&format!("{}:B", dev.name), &ly.poly, emitter_x, cy - ext, ct, ct)?;

            // Recognition marker over the emitter block: the deck's
            // `device_recognition.bjt` rule requires `type_marker` to cover
            // the emitter/base overlap. PDKs without a marker layer skip it
            // (no recognition — matches their empty bjt rule set).
            let marker = if is_pnp(devices) { &ly.pnp } else { &ly.npn };
            let _ = b.rect(marker, emitter_x, emitter_y, emitter_span, emitter_h);

            if !is_pnp(devices) {
                let well_enc = pdk.well_enclosure;
                b.rect(
                    &ly.nwell,
                    cx - well_enc,
                    cy - well_enc,
                    coll_w + 2 * well_enc,
                    coll_h + 2 * well_enc,
                )?;
                // N+ tap marker over the left collector band: the collector
                // ring IS the nwell tap (ERC floating_well wants diff+nsdm
                // inside the well). Left band only — the implant must never
                // reach the base poly bar, or MOS recognition would see
                // implant∩gate∩diff and extract the base crossing as a MOS.
                let e = 20;
                let _ = b.rect(
                    &ly.nsdm,
                    cx - e,
                    cy - e,
                    (collector_w + 2 * e).max(380),
                    coll_h + 2 * e,
                );
            }

            // ── ERC missing_tie: li chunk chains along large diffusions ──
            // The check wants an li bbox-center within tie_max of every
            // corner of any diff bigger than tie_max². Chains run on the
            // short-axis centerline and merge with the flow's pin li where
            // they meet it (same terminal net); elsewhere they are passive
            // li straps over the diff (no cuts drawn here).
            let tie_max = b.deck().erc.tie_max_dist_nm.max(1);
            let tie2 = i64::from(tie_max) * i64::from(tie_max);
            let chunk = (tie_max / 2).max(ct);
            let mut regions: Vec<(i32, i32, i32, i32)> = vec![
                (cx, cy, coll_w, collector_w),
                (cx, cy + coll_h - collector_w, coll_w, collector_w),
                (cx, cy, collector_w, coll_h),
                (cx + coll_w - collector_w, cy, collector_w, coll_h),
            ];
            for s in 0..n_stripes {
                regions.push((
                    emitter_x + s * stripe_pitch,
                    emitter_y,
                    stripe_w,
                    emitter_h,
                ));
            }
            for (rx, ry, rw, rh) in regions {
                if i64::from(rw) * i64::from(rh) < tie2 {
                    continue;
                }
                if rw >= rh {
                    super::li_chain(b, &ly.li, rx, ry + rh / 2 - ct / 2, rw, ct, chunk)?;
                } else {
                    super::li_chain(b, &ly.li, rx + rw / 2 - ct / 2, ry, ct, rh, chunk)?;
                }
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
    use crate::{CellBuilder, MatchingTier};

    fn test_deck() -> crate::Deck {
        crate::test_util::deck_from_layers(&[
            ("diff", 65, 20),
            ("poly", 66, 20),
            ("li", 67, 20),
            ("nwell", 64, 20),
            ("npn", 82, 20),
            ("pnp", 82, 44),
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
        let npn_id = deck.layers.id("npn").unwrap();
        let pnp_id = deck.layers.id("pnp").unwrap();
        let on = |l| (0..out.store.poly_count()).any(|i| out.store.poly_layer[i] == l);
        assert!(on(npn_id), "NPN draws its recognition marker");
        assert!(!on(pnp_id), "NPN must not draw the PNP marker");
    }

    #[test]
    fn drc_clean_npn() {
        let deck = crate::test_util::sky130_deck();
        let pdk = crate::test_util::sky130_pdk();
        let devices = vec![npn("Q1")];
        let cell = SpecCell {
            spec: BjtSpec { columns: 1 },
            devices,
            pdk,
        };
        crate::test_util::generate_and_verify(&cell, &deck, MatchingTier::None);
    }
}
