//! Inductor generator: a rectangular met1 spiral. Inductors have no LVS
//! recogniser, so this only has to be DRC-clean and placeable.

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process, Rect};

use crate::builder::{pin, req, sizing, Builder, Sizing};
use crate::Cell;

/// The one inductor variant: `turns` rectangular turns.
#[derive(Clone)]
pub struct Inductor {
    pub turns: u16,
}

impl Cell for Inductor {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        vec![Inductor { turns: s.dev_nf.iter().sum::<u16>().max(1) }]
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);

        let trace_w = s.unit_w.max(process.rule("ind_min_trace", 1000));
        let outer_d = s.unit_l.max(process.rule("ind_min_diameter", 10_000));
        let n_turns = i32::from(self.turns.max(1));
        let spacing = trace_w;

        let met1 = req(process, "met1");
        let li = req(process, "li");
        let ct = process.rule("contact", 170);

        // ponytail: rectangular turns, not an octagonal path — DRC-clean and
        // placeable; draw polygons if EM accuracy ever matters.
        let mut ring_outer = outer_d;
        for turn in 0..n_turns {
            if ring_outer <= 2 * trace_w {
                break;
            }
            b.rect(met1, Rect { x: 0, y: 0, w: ring_outer, h: trace_w });
            b.rect(met1, Rect { x: 0, y: ring_outer - trace_w, w: ring_outer, h: trace_w });
            b.rect(met1, Rect { x: 0, y: trace_w, w: trace_w, h: ring_outer - 2 * trace_w });

            let right_h = ring_outer - 2 * trace_w;
            if turn == 0 {
                let gap = trace_w + spacing;
                if right_h > gap {
                    b.rect(met1, Rect { x: ring_outer - trace_w, y: trace_w, w: trace_w, h: right_h - gap });
                }
            } else {
                b.rect(met1, Rect { x: ring_outer - trace_w, y: trace_w, w: trace_w, h: right_h });
            }
            ring_outer -= 2 * (trace_w + spacing);
        }

        let center = outer_d / 2;
        b.rect(li, Rect { x: center - trace_w / 2, y: center - trace_w / 2, w: trace_w, h: outer_d / 2 });
        b.pin(pin(0, "P", Rect { x: outer_d - trace_w, y: trace_w, w: ct, h: ct }, met1));
        b.pin(pin(0, "N", Rect { x: center - ct / 2, y: center - ct / 2, w: ct, h: ct }, li));

        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    sizing(group, c, process.rule("ind_min_trace", 1000), process.rule("ind_min_diameter", 10_000))
}
