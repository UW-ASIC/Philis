//! Adapter: expose a `cells::Cell` device generator as a macroMaster `DeviceGen`.
//!
//! macroMaster does **not** rebuild the device generators — the real,
//! signoff-grade ones live in `cells` (`mosfet`, `resistor`, …). This bridge
//! synthesises the one-device [`DeviceGroup`] + [`Unitization`] (`w`/`l`/`nf`) a
//! `cells` generator expects, runs it, and returns the drawn [`Macro`] for the
//! Device tier to replay through [`crate::DeviceBuilder::draw`]. See macroMaster
//! `TODO.md` #1 / ADR 0004.
//!
//! The dependency direction is `macroMaster → cells` (never the reverse): a
//! device is drawn by `cells` via `cells::Builder`; macroMaster only *exposes* it.

use analog::cell::{SeriesParallel, Unitization};
use analog::Constraints;
use cells::Cell;
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, Macro, Process, Rect};

/// Run the `cells` generator `G` for a single device of `kind` sized `w`/`l`/`nf`
/// and return its drawn geometry. The variant is picked by smallest footprint —
/// a lone Device has no placer to reshape it, so the smallest legal one is the
/// natural choice for the portability (Generic-PDK) check.
#[must_use]
pub fn draw_cell<G: Cell>(
    kind: DeviceKind,
    w: i32,
    l: i32,
    nf: u16,
    process: &dyn Process,
) -> Macro {
    draw_cell_where::<G>(kind, w, l, nf, process, |_| true)
}

/// [`draw_cell`] with a variant *filter* — how a Device exposes the generator's
/// topological alternatives (finger pattern, dummy count, stack construction)
/// as parameters. Falls back to the unfiltered set when nothing matches, so a
/// pattern the enumeration cannot produce degrades to the smallest legal
/// drawing rather than to empty geometry (the structural checks would report
/// that only as an opaque floating-port error).
#[must_use]
pub fn draw_cell_where<G: Cell>(
    kind: DeviceKind,
    w: i32,
    l: i32,
    nf: u16,
    process: &dyn Process,
    pick: impl Fn(&G) -> bool,
) -> Macro {
    draw_group_where::<G>(kind, w, l, &[nf.max(1)], process, pick)
}

/// Group form of [`draw_cell_where`]: `dev_nf` gives one finger/segment count
/// per member device, so a matched pair/quad draws as ONE macro with per-member
/// pins (`d0:G`, `d1:G`, …) — the group-collapse geometry, requestable from the
/// Device tier.
#[must_use]
pub fn draw_group_where<G: Cell>(
    kind: DeviceKind,
    w: i32,
    l: i32,
    dev_nf: &[u16],
    process: &dyn Process,
    pick: impl Fn(&G) -> bool,
) -> Macro {
    let n = dev_nf.len().max(1);
    let group = DeviceGroup {
        devices: (0..n).map(|i| DeviceId(i as u16)).collect(),
    };
    // FETs parallel their fingers; passives (R/C) stack in series — the same
    // split `annotator::constraints::assemble` makes.
    let series_parallel = match kind {
        DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
        _ => SeriesParallel::Parallel,
    };
    let mut c = Constraints::default();
    c.unitization.push(Unitization {
        devices: group.devices.clone(),
        device_type: kind,
        dev_nf: dev_nf.iter().map(|&f| f.max(1)).collect(),
        target_ratio: vec![1; n],
        unit_w: w,
        unit_l: l,
        series_parallel,
        same_variant_required: true,
        // Dummies are the caller's choice via `pick`, not forced here.
        dummy_required: false,
        route_matching_required: false,
    });
    let (mut matching, mut rest) = (Vec::new(), Vec::new());
    for v in G::enumerate(&group, &c, process) {
        if pick(&v) { matching.push(v) } else { rest.push(v) }
    }
    let pool = if matching.is_empty() { rest } else { matching };
    let best = pool.into_iter().min_by_key(|v| {
        let (vw, vh) = v.estimate(&group, process);
        i64::from(vw) * i64::from(vh)
    });
    match best {
        Some(v) => v.draw(&group, &c, process),
        None => Macro { shapes: Vec::new(), pins: Vec::new(), bbox: Rect { x: 0, y: 0, w: 0, h: 0 } },
    }
}
