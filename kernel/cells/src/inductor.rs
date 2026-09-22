//! Inductor generator. Ported from `backend/cells/src/generators/inductor.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::builder::{req, rule, sizing, Builder, Sizing};
use crate::Cell;

/// One point in the **inductor variant space**: an `n`-turn spiral on thick top
/// metal with a patterned ground shield and a keep-out halo (no devices under the
/// coil). Geometry sets `L` and `Q`. Theory: AOAL ch06/6.4; `docs/cells/inductor.md`.
#[derive(Clone)]
pub struct Inductor {
    pub turns: u16,
    pub ground_shield: bool,
}

impl Cell for Inductor {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        // Single variant per group, just like the old `InductorSpec` — turn count
        // rides in on the unit count.
        let s = group_sizing(group, constraints, process);
        vec![Inductor { turns: s.total_nf(), ground_shield: false }]
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        let s = group_sizing(group, &Constraints::default(), process);
        let outer_d = s.unit_l.max(rule(process, "ind_min_diameter", 10_000));
        (outer_d, outer_d)
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        (0..group.devices.len())
            .flat_map(|i| [port(i, "P"), port(i, "N")])
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);

        let trace_w = s.unit_w.max(rule(process, "ind_min_trace", 1000));
        let outer_d = s.unit_l.max(rule(process, "ind_min_diameter", 10_000));
        let n_turns = i32::from(self.turns.max(1));
        let spacing = trace_w;

        let met1 = req(process, "met1");
        let li = req(process, "li");
        let ct = rule(process, "contact", 170);

        // ponytail: rectangular approximation of an octagonal spiral — DRC-clean
        // and placement-correct; upgrade to a polygon path for EM accuracy.
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
                    b.rect(met1, Rect {
                        x: ring_outer - trace_w,
                        y: trace_w,
                        w: trace_w,
                        h: right_h - gap,
                    });
                }
            } else {
                b.rect(met1, Rect { x: ring_outer - trace_w, y: trace_w, w: trace_w, h: right_h });
            }

            let inset = trace_w + spacing;
            ring_outer -= 2 * inset;
        }

        let center = outer_d / 2;
        b.rect(li, Rect {
            x: center - trace_w / 2,
            y: center - trace_w / 2,
            w: trace_w,
            h: outer_d / 2,
        });

        b.pin(Pin {
            name: "d0:P".into(),
            net: net_of(0, "P"),
            at: Rect { x: outer_d - trace_w, y: trace_w, w: ct, h: ct },
            layer: met1,
        });
        b.pin(Pin {
            name: "d0:N".into(),
            net: net_of(0, "N"),
            at: Rect { x: center - ct / 2, y: center - ct / 2, w: ct, h: ct },
            layer: li,
        });

        b.finish()
    }
}

fn port(i: usize, term: &str) -> Pin {
    Pin {
        name: format!("d{i}:{term}"),
        net: net_of(i, term),
        at: Rect { x: 0, y: 0, w: 0, h: 0 },
        // Enumeration placeholder: a 0x0 rect is never routed to, so the layer
        // is not a claim about geometry. `ports()` has no `Process` to ask.
        layer: LayerId(0),
    }
}

fn net_of(i: usize, term: &str) -> NetId {
    let t = if term == "P" { 0 } else { 1 };
    NetId((i as u16).wrapping_mul(4).wrapping_add(t))
}


fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def_w = rule(process, "ind_min_trace", 1000);
    let def_l = rule(process, "ind_min_diameter", 10_000);
    sizing(group, c, def_w, def_l)
}
