//! Diode generator. Ported from `backend/cells/src/generators/diode.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::Pattern;

use crate::builder::{layer, req, rule, sizing, Builder, Sizing};
use crate::Cell;

/// One point in the **diode variant space** (junction / ESD clamp). Area set by
/// finger count; guard-ringed for ESD current handling. Theory: AOAL ch05
/// (ESD/reliability); `docs/cells/diode.md`.
///
/// `fingers` is the array unit count; `columns` (the old `DiodeSpec` axis) sets
/// the array aspect and `pattern` toggles interdigitation for matched banks.
#[derive(Clone)]
pub struct Diode {
    pub fingers: u16,
    pub guard_ring: bool,
    pub pattern: Pattern,
    pub columns: u16,
}

impl Cell for Diode {
    fn enumerate(group: &DeviceGroup, _constraints: &Constraints, _process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let patterns = if group.devices.len() > 1 {
            vec![Pattern::Single, Pattern::Interdig]
        } else {
            vec![Pattern::Single]
        };
        let n = group.devices.len().max(1) as u16;
        let mut cols = vec![1, n, (f64::from(n).sqrt().ceil() as u16).max(1)];
        cols.sort_unstable();
        cols.dedup();
        let mut specs = Vec::new();
        for pattern in patterns {
            for &columns in &cols {
                specs.push(Diode { fingers: n, guard_ring: false, pattern, columns });
            }
        }
        specs
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        let s = group_sizing(group, &Constraints::default(), process);
        let w = s.unit_w;
        let l = s.unit_l;
        let n = group.devices.len().max(1) as i32;
        let cols = i32::from(self.columns.max(1)).min(n);
        let rows = (n + cols - 1) / cols;
        let gap = rule(process, "diode_gap", 200);
        (cols * w + (cols - 1) * gap, rows * l + (rows - 1) * gap)
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        // The old generator declared a single shared A/K pair. Keep the per-device
        // A/K pins the geometry actually lands.
        (0..group.devices.len())
            .flat_map(|i| [port(i, "A"), port(i, "K")])
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();

        let w = s.unit_w;
        let l = s.unit_l;
        let diff = req(process, "diff");
        let li = req(process, "li");
        let ct = rule(process, "contact", 170);
        let gap = rule(process, "diode_gap", 200);
        // A contact-sized li pad falls under the deck's li min-area (the rule
        // carries the square's side); round the pad side up to the grid.
        let grid = process.grid().max(1);
        let pad = {
            let side = rule(process, "li_min_area", 236).max(ct);
            (side + grid - 1) / grid * grid
        };

        let ay = l / 4 - pad / 2;
        let ky = 3 * l / 4 - pad / 2;
        let cols = i32::from(self.columns.max(1)).min(n_dev.max(1) as i32);
        // The LVS recognition marker: one `diom` polygon per device, over the
        // device's whole diff body, mirroring how a MOS channel's derived
        // marker names a transistor. The deck's diode recogniser binds the two
        // `li` pads under it as the device's terminals. Optional so a deck
        // without the marker still draws the junction.
        let diom = layer(process, "diom");
        for di in 0..n_dev {
            let ox = (di as i32 % cols) * (w + gap);
            let oy = (di as i32 / cols) * (l + gap);
            let flip = self.pattern == Pattern::Interdig && di % 2 == 1;
            b.rect(diff, Rect { x: ox, y: oy, w, h: l });
            if let Some(diom) = diom {
                b.rect(diom, Rect { x: ox, y: oy, w, h: l });
            }
            for (name, py) in [("A", ay), ("K", ky)] {
                let (px, py2) = my(w / 2 - pad / 2, py, pad, l, flip);
                b.rect(li, Rect { x: ox + px, y: oy + py2, w: pad, h: pad });
                b.pin(pin_at(di, name, ox + px, oy + py2, pad, li));
            }
        }

        b.finish()
    }
}

/// Mirror-Y of a `(x,y,ct,ct)` contact within a cell of height `l` (`R0` when
/// `!flip`; only y flips, matching the old diode's `Orientation::MY`).
fn my(x: i32, y: i32, ct: i32, l: i32, flip: bool) -> (i32, i32) {
    if flip {
        (x, l - y - ct)
    } else {
        (x, y)
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

fn pin_at(i: usize, term: &str, x: i32, y: i32, ct: i32, layer: LayerId) -> Pin {
    Pin {
        name: format!("d{i}:{term}"),
        net: net_of(i, term),
        at: Rect { x, y, w: ct, h: ct },
        layer,
    }
}

fn net_of(i: usize, term: &str) -> NetId {
    let t = if term == "A" { 0 } else { 1 };
    NetId((i as u16).wrapping_mul(4).wrapping_add(t))
}


fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def_w = rule(process, "diode_w", 500);
    let def_l = rule(process, "diode_l", 1000);
    sizing(group, c, def_w, def_l)
}
