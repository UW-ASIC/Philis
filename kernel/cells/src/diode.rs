//! Diode generator: n+ diffusion (cathode `N`) in the p-substrate, with a
//! p+ substrate tap beside it as the anode contact (`P`), under `diom`;
//! devices arrayed in `columns`.

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, Builder, Sizing};
use crate::{Cell, Pattern};

/// One diode variant: array aspect `columns`; `Interdig` flips every other
/// device (anode tap on the left) for matched banks.
#[derive(Clone)]
pub struct Diode {
    pub pattern: Pattern,
    pub columns: u16,
}

impl Cell for Diode {
    fn enumerate(group: &DeviceGroup, _constraints: &Constraints, _process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let n = group.devices.len() as u16;
        let patterns: &[Pattern] =
            if n > 1 { &[Pattern::Single, Pattern::Interdig] } else { &[Pattern::Single] };
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        patterns
            .iter()
            .flat_map(|&pattern| cols.iter().map(move |&columns| Diode { pattern, columns }))
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let r = |name: &str, default: i32| process.rule(name, default);
        let lat = cut_lattice(process);
        let up = |v: i32| snap_cut(v + lat - 1, lat);

        let (diff, tap, li, licon) = (req(process, "diff"), req(process, "tap"), req(process, "li"), req(process, "licon"));
        let (nsdm, psdm) = (req(process, "nsdm"), req(process, "psdm"));
        let ct = r("contact", 170);
        let diff_enc = r("diff_encloses_licon", 40);
        let li_enc = r("li_encloses_licon", 80);
        let imp = 125; // implant past the diffusion it dopes
        // Cathode diff: room for a cut with the one-direction diff enclosure.
        let (w, l) = (up(s.unit_w.max(ct + 2 * 60)), up(s.unit_l.max(ct + 2 * 60)));
        // Anode tap: a cut with `diff_enc` on one side, the tap one-side
        // enclosure on the other, in both axes.
        let tap_side = ct + diff_enc + r("tap_encloses_licon_one_side", 120);
        let tap_h = l.max(tap_side);
        // Opposite implants may not overlap.
        let g = up(2 * imp + 50);
        // Between devices: same-type implants keep their spacing.
        let gap = up(r("diode_gap", 200).max(2 * imp + r("nsdm_min_spacing", 380).max(r("psdm_min_spacing", 380))));
        let dev_w = w + g + tap_side;
        let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
        let diom = process.layer("diom");
        let contact = |b: &mut Builder, cut: Rect| {
            b.rect(licon, cut);
            b.rect(li, Rect { x: cut.x - li_enc, y: cut.y - li_enc, w: cut.w + 2 * li_enc, h: cut.h + 2 * li_enc });
        };
        for di in 0..n_dev {
            let ox = (di as i32 % cols) * (dev_w + gap);
            let oy = (di as i32 / cols) * (tap_h + gap);
            // Interdig mirrors every other device: anode tap on the left.
            let flip = self.pattern == Pattern::Interdig && di % 2 == 1;
            let (kx, ax) = if flip { (ox + tap_side + g, ox) } else { (ox, ox + w + g) };

            let k = Rect { x: kx, y: oy, w, h: l };
            b.rect(diff, k);
            b.rect(nsdm, Rect { x: k.x - imp, y: k.y - imp, w: w + 2 * imp, h: l + 2 * imp });
            let at = Rect { x: snap_cut(kx + w / 2 - ct / 2, lat), y: snap_cut(oy + l / 2 - ct / 2, lat), w: ct, h: ct };
            contact(&mut b, at);
            b.pin(pin(di, "N", at, li));

            let a = Rect { x: ax, y: oy, w: tap_side, h: tap_h };
            b.rect(tap, a);
            b.rect(psdm, Rect { x: a.x - imp, y: a.y - imp, w: a.w + 2 * imp, h: a.h + 2 * imp });
            let at = Rect { x: ax + if flip { tap_side - diff_enc - ct } else { diff_enc }, y: oy + diff_enc, w: ct, h: ct };
            contact(&mut b, at);
            b.pin(pin(di, "P", at, li));

            // LVS marker: the recogniser binds the two li pads under it.
            if let Some(diom) = diom {
                b.rect(diom, Rect { x: ox, y: oy, w: dev_w, h: tap_h });
            }
        }

        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, process.rule("diode_w", 500), process.rule("diode_l", 1000))
}
