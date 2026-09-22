//! The shared drawing surface. `cells` generators and `macroMaster` user
//! generators both build geometry through it, so both emit identical [`Macro`]s.
//!
//! Ported from `backend/cells/src/api.rs` (`CellBuilder`, `GeometryStore`,
//! `snap_to_grid`). The old builder resolved named layers through a `Deck` and
//! carried DRC/pin metadata; here layers arrive pre-resolved as [`LayerId`] and
//! the surface is a flat SoA of [`Shape`]s plus [`Pin`]s, so `.finish()` is a
//! pure fold into a [`Macro`].

use analog::cell::Unitization;
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceId, LayerId, Macro, Pin, Process, Rect, Shape};

/// Accumulates rectangles on named layers, tracks pins, snaps to the PDK grid,
/// and yields a [`Macro`] via [`Builder::finish`].
pub struct Builder {
    grid: i32,
    shapes: Vec<Shape>,
    pins: Vec<Pin>,
}

impl Builder {
    /// New builder snapping to `grid` nm.
    #[must_use]
    pub fn new(grid: i32) -> Self {
        Self { grid, shapes: Vec::new(), pins: Vec::new() }
    }

    /// Draw a rectangle on `layer`, snapped to grid. `w`/`h` clamp up to one
    /// grid step so a snapped-to-zero rect never trips the layer's min-width
    /// rule (api.rs `rect_id`).
    pub fn rect(&mut self, layer: LayerId, r: Rect) {
        let g = self.grid.max(1);
        self.shapes.push(Shape {
            layer,
            rect: Rect {
                x: snap_to_grid(r.x, self.grid),
                y: snap_to_grid(r.y, self.grid),
                w: snap_to_grid(r.w, self.grid).max(g),
                h: snap_to_grid(r.h, self.grid).max(g),
            },
        });
    }

    /// Register a pin (net attach point). Snaps like [`Builder::rect`] so pins
    /// stay coincident with the geometry they were computed from.
    pub fn pin(&mut self, mut pin: Pin) {
        let g = self.grid.max(1);
        pin.at = Rect {
            x: snap_to_grid(pin.at.x, self.grid),
            y: snap_to_grid(pin.at.y, self.grid),
            w: snap_to_grid(pin.at.w, self.grid).max(g),
            h: snap_to_grid(pin.at.h, self.grid).max(g),
        };
        self.pins.push(pin);
    }

    /// Consume the builder into a [`Macro`] (computes the bbox from shapes).
    ///
    /// Each extent is rounded up to an **even** number of grid steps.
    /// `gp::mechanics::half_extents` takes the placer's half-extent as
    /// `bbox.w / 2`, and `gr::place_macros` stamps drawn geometry at
    /// `centre - half_extent`; that offset lands on the fabrication grid only if
    /// the half-extent itself does. Without this, a device an odd number of grid
    /// steps across stamps its whole cell half a step off, and signoff comes back
    /// with hundreds of `off_grid` violations. A bbox may be looser than the
    /// tight hull of its shapes, and one grid step of slack per axis costs
    /// nothing against the spacing rules.
    #[must_use]
    pub fn finish(self) -> Macro {
        let mut bbox = bbox_of(&self.shapes);
        let step = 2 * self.grid.max(1);
        let round_up = |v: i32| ((v + step - 1) / step) * step;
        bbox.w = round_up(bbox.w);
        bbox.h = round_up(bbox.h);
        Macro { shapes: self.shapes, pins: self.pins, bbox }
    }
}

/// Bounding box over all shapes, `(0,0,0,0)` when empty (api.rs `compute_bbox`).
fn bbox_of(shapes: &[Shape]) -> Rect {
    let mut it = shapes.iter();
    let Some(first) = it.next() else {
        return Rect { x: 0, y: 0, w: 0, h: 0 };
    };
    let mut xmin = first.rect.x;
    let mut ymin = first.rect.y;
    let mut xmax = first.rect.x + first.rect.w;
    let mut ymax = first.rect.y + first.rect.h;
    for s in it {
        xmin = xmin.min(s.rect.x);
        ymin = ymin.min(s.rect.y);
        xmax = xmax.max(s.rect.x + s.rect.w);
        ymax = ymax.max(s.rect.y + s.rect.h);
    }
    Rect { x: xmin, y: ymin, w: xmax - xmin, h: ymax - ymin }
}

/// Snap `value` to the manufacturing `grid`. Round-half-away-from-zero, keeping
/// sign — an exact port of api.rs `snap_to_grid`.
#[must_use]
pub fn snap_to_grid(value: i32, grid: i32) -> i32 {
    if grid <= 0 {
        return value;
    }
    let rem = value % grid;
    if rem == 0 {
        return value;
    }
    let abs_rem = rem.abs();
    let half = (grid + 1) / 2;
    if abs_rem >= half {
        value + (grid - abs_rem) * value.signum()
    } else {
        value - rem
    }
}

/// Handle to a placed sub-instance, for composing a macro from primitives.
pub struct Instance {
    pub bbox: Rect,
}

// ---------------------------------------------------------------------------
//  PDK access shims — thin wrappers over the abstract [`Process`] seam
// ---------------------------------------------------------------------------
//
// Generators never touch a concrete PDK. They resolve every layer role and rule
// value through the PDK-agnostic [`pnr_core::Process`] trait, whose one concrete
// impl (`verify::Pdk`) lives outside this crate. These two helpers just forward
// to it so the generators keep their terse `layer(process, "poly")` /
// `rule(process, "contact", 170)` call sites.

/// Resolve a layer *role* to its [`LayerId`]. Missing layers (optional implant/
/// well/marker layers on geometry-only decks) return `None`; callers draw them
/// with `if let Some(..)`.
///
/// Use this **only** where absence is a legitimate process variation. For a layer
/// the cell cannot be correct without, use [`req`]: an `if let Some(..)` on a
/// mandatory role silently emits nothing, which is how every sky130 cell ended up
/// with no well or body tap at all.
#[must_use]
pub fn layer(process: &dyn Process, role: &str) -> Option<LayerId> {
    process.layer(role)
}

/// Resolve a layer role the cell **cannot be correct without**.
///
/// # Panics
/// If the role does not resolve. `Pdk::validate` rejects such a deck at load, so
/// reaching here means a generator asked for a role the PDK never declared — a
/// bug in one of the two, not a process variation.
///
/// This was `unwrap_or(LayerId(0))`, duplicated in six generators. `LayerId(0)`
/// is the deck's *first* layer — `nwell` in sky130 — so an unresolvable mandatory
/// role quietly drew its geometry onto the well.
#[must_use]
pub fn req(process: &dyn Process, role: &str) -> LayerId {
    process.layer(role).unwrap_or_else(|| {
        panic!(
            "PDK declares no layer for mandatory role {role:?}; add it to the \
             deck's cell.layers section"
        )
    })
}

/// Look up a construction value by `name`, falling back to `default` when the
/// bound process does not specify it.
#[must_use]
pub fn rule(process: &dyn Process, name: &str, default: i32) -> i32 {
    process.rule(name, default)
}

// ---------------------------------------------------------------------------
//  Per-device electrical sizing
// ---------------------------------------------------------------------------
//
// The old generators read `ref_dev.w / .l / .nf / .multiplier` straight off the
// `DeviceRecord`. The pure `Cell` trait is handed only a `DeviceGroup` (device
// *ids*, no W/L) and the group's `Constraints`. The intended data channel for
// per-instance geometry is the [`Unitization`] constraint the annotator recovers
// for the group: it carries `unit_w`/`unit_l` (unit finger/segment geometry) and
// per-instance `dev_nf` — exactly the `unit_w`/`unit_nf` the old `MosfetSpec`
// threaded. [`Sizing`] resolves that, falling back to PDK-derived defaults when
// no unitization was attached (single un-matched device).

/// Resolved electrical sizing for the devices in a group.
pub struct Sizing {
    /// Unit finger/segment width, `nm`.
    pub unit_w: i32,
    /// Unit finger/segment length (== channel/body L), `nm`.
    pub unit_l: i32,
    /// Per-instance finger/segment counts (len == group device count).
    pub dev_nf: Vec<u16>,
}

impl Sizing {
    /// Total finger/segment count over the whole group.
    #[must_use]
    pub fn total_nf(&self) -> u16 {
        self.dev_nf.iter().copied().sum::<u16>().max(1)
    }
}

/// Find the [`Unitization`] that **covers** this group — a *subset* match: the
/// group's devices are all members of `u`. So a per-device (single-device) group
/// still resolves the group-scoped unitization that contains it, and
/// `u.devices.len() > 1` on the result still flags the device as *matched* (drives
/// LOD/WPE). `cellgen` synthesises a 1-device unitization for every un-matched
/// device, so every drawn device finds exactly one covering unitization.
#[must_use]
pub fn unitization<'a>(group: &DeviceGroup, c: &'a Constraints) -> Option<&'a Unitization> {
    if group.devices.is_empty() {
        return None;
    }
    c.unitization
        .iter()
        .find(|u| group.devices.iter().all(|d| u.devices.contains(d)))
}

/// Resolve group sizing from the covering unitization, or fall back to a
/// single-finger device at the PDK's minimum finger width. `def_w`/`def_l` are
/// the family's default unit geometry (`min_finger_width`, a nominal L).
#[must_use]
pub fn sizing(group: &DeviceGroup, c: &Constraints, def_w: i32, def_l: i32) -> Sizing {
    let n = group.devices.len().max(1);
    match unitization(group, c) {
        Some(u) => {
            // Map each group device to its slot in the (possibly larger,
            // group-scoped) unitization and read *that instance's* finger count —
            // so a single-device group gets exactly its own `dev_nf`, and a ratioed
            // mirror leg gets its own `N`.
            let dev_nf: Vec<u16> = group
                .devices
                .iter()
                .map(|d| {
                    u.devices
                        .iter()
                        .position(|x| x == d)
                        .and_then(|i| u.dev_nf.get(i))
                        .copied()
                        .unwrap_or(1)
                        .max(1)
                })
                .collect();
            Sizing {
                unit_w: if u.unit_w > 0 { u.unit_w } else { def_w },
                unit_l: if u.unit_l > 0 { u.unit_l } else { def_l },
                dev_nf: if dev_nf.is_empty() { vec![1; n] } else { dev_nf },
            }
        }
        // No covering unitization ⇒ one unit finger at the family default geometry.
        None => Sizing { unit_w: def_w, unit_l: def_l, dev_nf: vec![1; n] },
    }
}

/// Device ids of the group as a slice (readability helper).
#[must_use]
pub fn ids(group: &DeviceGroup) -> &[DeviceId] {
    &group.devices
}

/// Draw one merged conductor as a chain of overlapping chunks along its long
/// axis. Ported verbatim from `generators/mod.rs::li_chain`.
///
/// ERC `missing_tie` measures diff-corner → li *bbox-center* distance, so a
/// single long strip reads as one far-away contact. Overlapping, size-staggered
/// chunks are electrically one shape (extraction merges on area overlap) but give
/// the check a contact center every `chunk` nm.
pub fn li_chain(b: &mut Builder, layer: LayerId, x: i32, y: i32, w: i32, h: i32, chunk: i32) {
    let horiz = w >= h;
    let span = if horiz { w } else { h };
    let step = chunk.max(if horiz { h } else { w }).max(1);
    let full = step.min(span);
    let mut s = 0;
    let mut odd = false;
    loop {
        // Clamp so every chunk is full-size: a sliver tail rect would trip the
        // layer's min-width rule even though the merged shape is wide.
        let cs = s.min(span - full);
        // Stagger the transverse size by +20nm so the pre-signoff coalesce never
        // unions a pair back into one bbox — every chunk survives as its own
        // contact center while staying one conductor.
        let t = if odd { 20 } else { 0 };
        if horiz {
            b.rect(layer, Rect { x: x + cs, y, w: full, h: h + t });
        } else {
            b.rect(layer, Rect { x, y: y + cs, w: w + t, h: full });
        }
        if cs + full >= span {
            return;
        }
        s += step - 20; // 20nm overlap keeps chunks one extracted conductor
        odd = !odd;
    }
}
