//! Diode generator: one `diom`-marked diff body per device with anode (`P`)
//! and cathode (`N`) li pads, arrayed in `columns`.

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{pin, req, sizing, Builder, Sizing};
use crate::{Cell, Pattern};

/// One diode variant: array aspect `columns`; `Interdig` flips every other
/// device's pads for matched banks.
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

        let (w, l) = (s.unit_w, s.unit_l);
        let diff = req(process, "diff");
        let li = req(process, "li");
        let ct = process.rule("contact", 170);
        let gap = process.rule("diode_gap", 200);
        // li pad side: at least the li min-area square, on grid.
        let grid = process.grid().max(1);
        let pad = (process.rule("li_min_area", 236).max(ct) + grid - 1) / grid * grid;

        let ay = l / 4 - pad / 2;
        let ky = 3 * l / 4 - pad / 2;
        let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
        // LVS marker: the deck's diode recogniser binds the two li pads under it.
        let diom = process.layer("diom");
        for di in 0..n_dev {
            let ox = (di as i32 % cols) * (w + gap);
            let oy = (di as i32 / cols) * (l + gap);
            let flip = self.pattern == Pattern::Interdig && di % 2 == 1;
            b.rect(diff, Rect { x: ox, y: oy, w, h: l });
            if let Some(diom) = diom {
                b.rect(diom, Rect { x: ox, y: oy, w, h: l });
            }
            for (term, py) in [("P", ay), ("N", ky)] {
                let py = if flip { l - py - pad } else { py };
                let at = Rect { x: ox + w / 2 - pad / 2, y: oy + py, w: pad, h: pad };
                b.rect(li, at);
                b.pin(pin(di, term, at, li));
            }
        }

        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, process.rule("diode_w", 500), process.rule("diode_l", 1000))
}
