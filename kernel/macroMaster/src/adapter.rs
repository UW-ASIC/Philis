//! Run a `cells` generator for a Device: synthesise the one-group
//! unitization it reads its sizing from, draw the variants `pick` accepts
//! (all of them if none is accepted), keep the smallest.

use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::Cell;
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, Macro, Process, Rect};

/// Draw `dev_nf.len()` devices of `kind` at unit `w`×`l`, `dev_nf[i]`
/// fingers/units each, as one macro with `d{i}:` pins; `dummies` asks for the
/// generator's end dummies.
pub(crate) fn draw<G: Cell>(
    kind: DeviceKind,
    w: i32,
    l: i32,
    dev_nf: &[u16],
    dummies: bool,
    process: &dyn Process,
    pick: impl Fn(&G) -> bool,
) -> Macro {
    let n = dev_nf.len().max(1);
    let group = DeviceGroup { devices: (0..n).map(|i| DeviceId(i as u16)).collect() };
    let series_parallel = match kind {
        DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
        _ => SeriesParallel::Parallel,
    };
    let c = Constraints {
        unitization: vec![Unitization {
            devices: group.devices.clone(),
            device_type: kind,
            dev_nf: dev_nf.iter().map(|&f| f.max(1)).collect(),
            target_ratio: vec![1; n],
            unit_w: w,
            unit_l: l,
            series_parallel,
            same_variant_required: true,
            dummy_required: dummies,
            route_matching_required: false,
        }],
        ..Constraints::default()
    };
    let all = G::enumerate(&group, &c, process);
    let picked: Vec<&G> = all.iter().filter(|v| pick(v)).collect();
    let pool = if picked.is_empty() { all.iter().collect() } else { picked };
    pool.into_iter()
        .map(|v| v.draw(&group, &c, process))
        .min_by_key(|m| i64::from(m.bbox.w) * i64::from(m.bbox.h))
        .unwrap_or(Macro { shapes: Vec::new(), pins: Vec::new(), bbox: Rect { x: 0, y: 0, w: 0, h: 0 }, units: Vec::new(), dummies: Vec::new() })
}
