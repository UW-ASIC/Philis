//! The drawing surface shared by every generator (and by `macro_master`), plus
//! the sizing/pin helpers the generators have in common.
//!
//! All coordinates are nanometres in the cell's local frame.

use analog::cell::Unitization;
use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect, Shape};

/// Accumulates grid-snapped rectangles and pins into a [`Macro`].
///
/// Shapes, pins and keep-outs are snapped on entry; units, dummies and drawn
/// records are stored verbatim (they are centres and LVS cards, not
/// geometry). [`Builder::finish`] drops exact duplicate shapes and sizes the
/// bbox.
pub struct Builder {
    /// Manufacturing grid, nm; `<= 0` disables snapping.
    grid: i32,
    shapes: Vec<Shape>,
    pins: Vec<Pin>,
    units: Vec<pnr_core::Unit>,
    dummies: Vec<pnr_core::Dummy>,
    drawn: Vec<pnr_core::Drawn>,
    keepouts: Vec<pnr_core::Keepout>,
}

impl Builder {
    /// An empty builder snapping to `grid` nm (`<= 0`: no snapping, extents
    /// still clamp up to 1 nm).
    #[must_use]
    pub fn new(grid: i32) -> Self {
        Self { grid, shapes: Vec::new(), pins: Vec::new(), units: Vec::new(), dummies: Vec::new(), drawn: Vec::new(), keepouts: Vec::new() }
    }

    /// Draw `r` on `layer`, snapped to grid. Width/height clamp up to one grid
    /// step so a snapped-to-zero rect never trips a min-width rule.
    pub fn rect(&mut self, layer: LayerId, r: Rect) {
        let rect = self.snap(r);
        self.shapes.push(Shape { layer, rect });
    }

    /// Record one active unit (local frame, unsnapped: a centre, not a shape).
    pub fn unit(&mut self, u: pnr_core::Unit) {
        self.units.push(u);
    }

    /// Record one dummy gate drawn on a member's diffusion.
    pub fn dummy(&mut self, d: pnr_core::Dummy) {
        self.dummies.push(d);
    }

    /// Record one device drawn for LVS (`Macro::drawn`).
    pub fn drawn(&mut self, d: pnr_core::Drawn) {
        self.drawn.push(d);
    }

    /// Record a keep-out region, snapped like [`Builder::rect`].
    pub fn keepout(&mut self, r: Rect, why: pnr_core::KeepWhy) {
        let rect = self.snap(r);
        self.keepouts.push(pnr_core::Keepout { rect, why });
    }

    /// Register a pin, its `at` snapped like [`Builder::rect`].
    pub fn pin(&mut self, mut pin: Pin) {
        pin.at = self.snap(pin.at);
        self.pins.push(pin);
    }

    /// `r` with every coordinate on the grid and each extent at least one
    /// grid step (1 nm with snapping off).
    fn snap(&self, r: Rect) -> Rect {
        let g = self.grid.max(1);
        Rect {
            x: snap_to_grid(r.x, self.grid),
            y: snap_to_grid(r.y, self.grid),
            w: snap_to_grid(r.w, self.grid).max(g),
            h: snap_to_grid(r.h, self.grid).max(g),
        }
    }

    /// Cover every contact on poly with the deck's poly-contact mask (`npc`
    /// role; sky130: a poly cut must sit inside nitride poly cut, 100 nm
    /// clear): one strip per cut row, rows closer than the mask's spacing
    /// joined. Nothing on a deck without the role.
    ///
    /// ponytail: a row's strip spans all its cuts; a row that must skip a
    /// gap (a foreign contact between) would need splitting.
    pub fn cover_poly_cuts(&mut self, process: &dyn Process) {
        let (Some(npc), Some(poly), Some(licon)) = (process.layer("npc"), process.layer("poly"), process.layer("licon")) else {
            return;
        };
        if npc == poly {
            return;
        }
        let enc = process.enclosure("npc", "licon").unwrap_or(0);
        let space = process.space("npc").unwrap_or(0);
        let wmin = process.width("npc").unwrap_or(0);
        let polys: Vec<Rect> = self.shapes.iter().filter(|s| s.layer == poly).map(|s| s.rect).collect();
        let mut rows: Vec<Rect> = Vec::new();
        for c in self.shapes.iter().filter(|s| s.layer == licon && polys.iter().any(|p| contains(p, &s.rect))) {
            let r = Rect { x: c.rect.x - enc, y: c.rect.y - enc, w: c.rect.w + 2 * enc, h: c.rect.h + 2 * enc };
            match rows.iter_mut().find(|q| q.y == r.y && q.h == r.h) {
                Some(q) => *q = hull(*q, r),
                None => rows.push(r),
            }
        }
        // Join strips closer than the mask spacing (one merged figure).
        let near = |a: &Rect, b: &Rect| {
            let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
            let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
            dx.max(dy) < space
        };
        'again: loop {
            for i in 0..rows.len() {
                for j in i + 1..rows.len() {
                    if near(&rows[i], &rows[j]) {
                        rows[i] = hull(rows[i], rows[j]);
                        rows.swap_remove(j);
                        continue 'again;
                    }
                }
            }
            break;
        }
        for mut r in rows {
            if r.h < wmin {
                r.y -= (wmin - r.h) / 2;
                r.h = wmin;
            }
            if r.w < wmin {
                r.x -= (wmin - r.w) / 2;
                r.w = wmin;
            }
            self.rect(npc, r);
        }
    }

    /// The finished macro. Exact duplicate shapes (same layer and rect) are
    /// kept once. The bbox encloses every shape; its corner is a multiple of
    /// two grid steps (the cut lattice) and its extents of four, so the
    /// half-extent the placer stamps at (`centre - bbox.w / 2`) is on the cut
    /// lattice too (H01-26). No shapes: a zero-size bbox at the origin.
    #[must_use]
    pub fn finish(mut self) -> Macro {
        // Exact duplicates (rings sharing a band draw its cuts twice) are one
        // shape; a checker would read two coincident cuts as zero spacing.
        let mut seen = std::collections::HashSet::new();
        self.shapes.retain(|s| seen.insert((s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h)));
        let tight = bbox_of(&self.shapes);
        let step = 2 * self.grid.max(1);
        let ext = 2 * step;
        let x = tight.x.div_euclid(step) * step;
        let y = tight.y.div_euclid(step) * step;
        let bbox = Rect {
            x,
            y,
            w: (tight.x + tight.w - x + ext - 1) / ext * ext,
            h: (tight.y + tight.h - y + ext - 1) / ext * ext,
        };
        Macro { shapes: self.shapes, pins: self.pins, bbox, units: self.units, dummies: self.dummies, drawn: self.drawn, keepouts: self.keepouts, ..Default::default() }
    }
}

/// The lattice cut positions snap to: two grid steps. magic grows a cut that
/// is off this lattice by a grid step, which then reads as missing enclosure.
#[must_use]
pub fn cut_lattice(process: &dyn Process) -> i32 {
    2 * process.grid().max(1)
}

/// Round `v` down (toward −∞) onto the cut lattice `lat`.
///
/// # Panics
/// If `lat == 0`.
#[must_use]
pub fn snap_cut(v: i32, lat: i32) -> i32 {
    v.div_euclid(lat) * lat
}

/// The tight bounding box of `shapes`; a zero rect at the origin when empty.
fn bbox_of(shapes: &[Shape]) -> Rect {
    let Some(first) = shapes.first() else {
        return Rect::default();
    };
    let (mut x0, mut y0) = (first.rect.x, first.rect.y);
    let (mut x1, mut y1) = (x0 + first.rect.w, y0 + first.rect.h);
    for s in shapes {
        x0 = x0.min(s.rect.x);
        y0 = y0.min(s.rect.y);
        x1 = x1.max(s.rect.x + s.rect.w);
        y1 = y1.max(s.rect.y + s.rect.h);
    }
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// Round half away from zero to a multiple of `grid` (`grid <= 0`: identity).
pub(crate) fn snap_to_grid(value: i32, grid: i32) -> i32 {
    if grid <= 0 {
        return value;
    }
    // `down` (toward zero) never overflows; rounding away saturates to it
    // at the i32 extremes. `grid - grid / 2` is `ceil(grid / 2)` without the
    // `grid + 1` overflow.
    let rem = value % grid;
    let down = value - rem;
    if rem.abs() >= grid - grid / 2 {
        down.checked_add(grid * value.signum()).unwrap_or(down)
    } else {
        down
    }
}

/// Resolve a layer role the cell cannot be correct without.
///
/// # Panics
/// If the deck does not declare `role` (`Pdk::validate` rejects such decks, so
/// reaching here is a generator/deck bug, never a process variation).
#[must_use]
pub fn req(process: &dyn Process, role: &str) -> LayerId {
    process.layer(role).unwrap_or_else(|| {
        panic!("PDK declares no layer for mandatory role {role:?}; add it to the deck's cell.layers section")
    })
}

/// A cell dimension the deck states: its value under the legacy sidecar
/// `key`, never under what the deck's rules require (a sidecar key can only
/// raise it). 0 when neither states it.
#[must_use]
pub fn dim(process: &dyn Process, key: &str) -> i32 {
    let q = |v: Option<i32>| v.unwrap_or(0);
    let deck = match key {
        "contact" => q(process.width("licon")),
        "mcon_size" => q(process.width("mcon")),
        "m1_enc" => q(process.enclosure("met1", "mcon")).max(q(process.endcap("met1", "mcon"))),
        "met1_space" => q(process.space("met1")),
        "poly_ext" => q(process.extension("poly", "diff")),
        "poly_min_width" | "min_gate_l" => q(process.width("poly")),
        "min_finger_width" => q(process.width("diff")),
        "nwell_diff_enc" => q(process.enclosure("nwell", "diff")),
        "nwell_min_width" => q(process.width("nwell")),
        "via_spacing" => q(process.space("via1")),
        "via_enclosure" => q(process.enclosure("met1", "via1")).max(q(process.enclosure("met2", "via1"))),
        _ => 0,
    };
    process.rule(key, 0).max(deck)
}

/// A pin `d{di}:{term}` over `at`. The net is a placeholder, distinct per
/// `(di, term)` for `di < 8192` (eight terminal slots per member in a `u16`),
/// that the caller rebinds by pin name. A `term` outside `G S D B P N C`
/// shares slot 7.
#[must_use]
pub fn pin(di: usize, term: &str, at: Rect, layer: LayerId) -> Pin {
    const TERMS: [&str; 7] = ["G", "S", "D", "B", "P", "N", "C"];
    let t = TERMS.iter().position(|&x| x == term).unwrap_or(TERMS.len());
    Pin { name: format!("d{di}:{term}"), net: NetId((di * 8 + t) as u16), at, layer }
}

/// Resolved sizing for the devices in a group.
///
/// Invariant: `dev_nf` holds one entry per group member (one entry for an
/// empty group), each `>= 1`.
pub struct Sizing {
    /// Unit finger/segment width, nm.
    pub unit_w: i32,
    /// Unit finger/segment length, nm.
    pub unit_l: i32,
    /// Finger/segment count per group member.
    pub dev_nf: Vec<u16>,
}

/// The unitization covering every device of `group` (subset match), if any.
#[must_use]
pub fn unitization<'a>(group: &DeviceGroup, c: &'a Constraints) -> Option<&'a Unitization> {
    if group.devices.is_empty() {
        return None;
    }
    c.unitization.iter().find(|u| group.devices.iter().all(|d| u.devices.contains(d)))
}

/// Group sizing from the covering unitization, else one finger at
/// `def_w`×`def_l` per device. A unitization's non-positive `unit_w`/`unit_l`
/// falls back to the default, a missing or zero finger count to 1.
#[must_use]
pub fn sizing(group: &DeviceGroup, c: &Constraints, def_w: i32, def_l: i32) -> Sizing {
    let Some(u) = unitization(group, c) else {
        return Sizing { unit_w: def_w, unit_l: def_l, dev_nf: vec![1; group.devices.len().max(1)] };
    };
    // Each member reads its own slot of the (possibly larger) unitization.
    // `unitization` only covers a non-empty group, so `dev_nf` is non-empty.
    let dev_nf: Vec<u16> = group
        .devices
        .iter()
        .map(|d| {
            let slot = u.devices.iter().position(|x| x == d);
            slot.and_then(|i| u.dev_nf.get(i)).copied().unwrap_or(1).max(1)
        })
        .collect();
    Sizing {
        unit_w: if u.unit_w > 0 { u.unit_w } else { def_w },
        unit_l: if u.unit_l > 0 { u.unit_l } else { def_l },
        dev_nf,
    }
}

/// The smallest rect enclosing both `a` and `b`.
pub(crate) fn hull(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    Rect { x: x0, y: y0, w: (a.x + a.w).max(b.x + b.w) - x0, h: (a.y + a.h).max(b.y + b.h) - y0 }
}

/// Whether `inner` lies inside `outer`, edges included.
pub(crate) fn contains(outer: &Rect, inner: &Rect) -> bool {
    inner.x >= outer.x && inner.y >= outer.y && inner.x + inner.w <= outer.x + outer.w && inner.y + inner.h <= outer.y + outer.h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_rounds_half_away_from_zero() {
        assert_eq!(snap_to_grid(7, 5), 5);
        assert_eq!(snap_to_grid(8, 5), 10);
        assert_eq!(snap_to_grid(-8, 5), -10);
        assert_eq!(snap_to_grid(-7, 5), -5);
        assert_eq!(snap_to_grid(10, 5), 10);
    }

    #[test]
    fn extents_are_twice_the_cut_lattice() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: 3, y: 7, w: 1235, h: 41 });
        let m = b.finish();
        assert_eq!(m.bbox.x % 10, 0);
        assert_eq!(m.bbox.y % 10, 0);
        assert_eq!(m.bbox.w % 20, 0);
        assert_eq!(m.bbox.h % 20, 0);
        let s = m.shapes[0].rect;
        assert!(m.bbox.x <= s.x && m.bbox.y <= s.y);
        assert!(m.bbox.x + m.bbox.w >= s.x + s.w && m.bbox.y + m.bbox.h >= s.y + s.h);
    }
}

/// A hand-built [`Process`] for unit tests that must not depend on a real
/// deck: every number is stated by the test, so an expected value is
/// derivable by hand.
#[cfg(test)]
pub(crate) mod fake {
    use pnr_core::{LayerId, Process};

    /// Layers by role (`LayerId` = position unless aliased) and numbers by
    /// key: a bare rule name for [`Process::rule`], else `w:role`, `s:role`,
    /// `eol:role`, `a:role`, `enc:outer:inner`, `cap:outer:inner`,
    /// `ext:outer:inner`, `sb:a:b`; ohms under `sheet:role` / `cut:cut:onto`.
    pub struct Deck {
        pub grid: i32,
        pub roles: Vec<(&'static str, u16)>,
        pub nums: Vec<(String, i32)>,
        pub ohms: Vec<(String, f32)>,
    }

    impl Deck {
        pub fn new(grid: i32, roles: &[&'static str]) -> Self {
            Deck { grid, roles: roles.iter().enumerate().map(|(i, &r)| (r, i as u16)).collect(), nums: Vec::new(), ohms: Vec::new() }
        }
        pub fn with(mut self, key: &str, v: i32) -> Self {
            self.nums.push((key.to_string(), v));
            self
        }
        pub fn ohm(mut self, key: &str, v: f32) -> Self {
            self.ohms.push((key.to_string(), v));
            self
        }
        /// `role` resolves to `id` (two roles on one layer).
        pub fn alias(mut self, role: &'static str, id: u16) -> Self {
            self.roles.push((role, id));
            self
        }
        fn get(&self, key: &str) -> Option<i32> {
            self.nums.iter().rev().find(|(k, _)| k == key).map(|&(_, v)| v)
        }
    }

    impl Process for Deck {
        fn layer(&self, role: &str) -> Option<LayerId> {
            self.roles.iter().find(|(r, _)| *r == role).map(|&(_, i)| LayerId(i))
        }
        fn rule(&self, name: &str, default: i32) -> i32 {
            self.get(name).unwrap_or(default)
        }
        fn grid(&self) -> i32 {
            self.grid
        }
        fn sheet_ohm(&self, role: &str) -> Option<f32> {
            let k = format!("sheet:{role}");
            self.ohms.iter().find(|(x, _)| *x == k).map(|&(_, v)| v)
        }
        fn cut_ohm(&self, cut: &str, onto: &str) -> Option<f32> {
            let k = format!("cut:{cut}:{onto}");
            self.ohms.iter().find(|(x, _)| *x == k).map(|&(_, v)| v)
        }
        fn space(&self, role: &str) -> Option<i32> {
            self.get(&format!("s:{role}"))
        }
        fn eol_space(&self, role: &str) -> Option<i32> {
            self.get(&format!("eol:{role}"))
        }
        fn width(&self, role: &str) -> Option<i32> {
            self.get(&format!("w:{role}"))
        }
        fn enclosure(&self, outer: &str, inner: &str) -> Option<i32> {
            self.get(&format!("enc:{outer}:{inner}"))
        }
        fn endcap(&self, outer: &str, inner: &str) -> Option<i32> {
            self.get(&format!("cap:{outer}:{inner}"))
        }
        fn extension(&self, outer: &str, inner: &str) -> Option<i32> {
            self.get(&format!("ext:{outer}:{inner}"))
        }
        fn area(&self, role: &str) -> Option<i64> {
            self.get(&format!("a:{role}")).map(i64::from)
        }
        fn space_between(&self, a: &str, b: &str) -> Option<i32> {
            self.get(&format!("sb:{a}:{b}"))
        }
    }
}

/// Corner cases for every builder function (cleanup step 2). Oracles: the
/// doc comments and hand-derived values on [`fake::Deck`].
#[cfg(test)]
mod cleanup_tests {
    use super::fake::Deck;
    use super::*;
    use analog::cell::{SeriesParallel, Unitization};
    use pnr_core::{DeviceId, DeviceKind, Dummy, KeepWhy, Unit};

    const R0: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

    fn uz(devices: &[u16], dev_nf: &[u16], w: i32, l: i32) -> Unitization {
        Unitization {
            devices: devices.iter().map(|&d| DeviceId(d)).collect(),
            device_type: DeviceKind::Nmos,
            dev_nf: dev_nf.to_vec(),
            target_ratio: vec![1; dev_nf.len()],
            unit_w: w,
            unit_l: l,
            series_parallel: SeriesParallel::Parallel,
            dummy_required: false,
            route_matching_required: false,
            class: None,
            kind: None,
            series: Vec::new(),
            style: None,
        }
    }

    fn group(devices: &[u16]) -> DeviceGroup {
        DeviceGroup { devices: devices.iter().map(|&d| DeviceId(d)).collect() }
    }

    fn constraints(us: Vec<Unitization>) -> Constraints {
        let mut c = Constraints::default();
        c.unitization = us;
        c
    }

    #[test]
    fn snap_to_grid_is_identity_without_a_grid() {
        assert_eq!(snap_to_grid(7, 0), 7);
        assert_eq!(snap_to_grid(-7, -3), -7);
        assert_eq!(snap_to_grid(i32::MIN, 0), i32::MIN);
        assert_eq!(snap_to_grid(123, 1), 123);
    }

    #[test]
    fn snap_to_grid_rounds_half_away_from_zero_at_even_and_odd_grids() {
        for (v, g, want) in [(0, 5, 0), (10, 10, 10), (5, 10, 10), (-5, 10, -10), (4, 10, 0), (-4, 10, 0), (3, 5, 5), (2, 5, 0), (-3, 5, -5), (-2, 5, 0)] {
            assert_eq!(snap_to_grid(v, g), want, "snap({v}, {g})");
        }
    }

    /// The nearest representable multiple, never an overflow.
    #[test]
    fn snap_to_grid_saturates_at_the_i32_extremes() {
        assert_eq!(snap_to_grid(i32::MAX, 10), 2_147_483_640);
        assert_eq!(snap_to_grid(i32::MIN, 10), -2_147_483_640);
    }

    #[test]
    fn rect_snaps_and_clamps_extents_to_one_step() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: 3, y: -3, w: 0, h: 2 });
        assert_eq!(b.finish().shapes[0].rect, Rect { x: 5, y: -5, w: 5, h: 5 });
        let mut b = Builder::new(0);
        b.rect(LayerId(0), Rect { x: 3, y: 4, w: 0, h: -2 });
        assert_eq!(b.finish().shapes[0].rect, Rect { x: 3, y: 4, w: 1, h: 1 });
    }

    #[test]
    fn pins_and_keepouts_snap_units_and_dummies_do_not() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: 0, y: 0, w: 100, h: 100 });
        b.pin(Pin { name: "d0:G".into(), net: NetId(0), at: Rect { x: 7, y: 7, w: 7, h: 7 }, layer: LayerId(1) });
        b.keepout(Rect { x: 7, y: 7, w: 1, h: 1 }, KeepWhy::Gate { owner: 0 });
        let u = Unit { owner: 0, x: 7, y: 9, weight: 3, phi: (1, 0), sa_sb: Unit::diffusion(1, 2) };
        b.unit(u);
        let d = Dummy { owner: 0, pmos: false, edge: "S", w: 7, l: 3 };
        b.dummy(d);
        let m = b.finish();
        assert_eq!(m.pins[0].at, Rect { x: 5, y: 5, w: 5, h: 5 });
        assert_eq!(m.pins[0].name, "d0:G");
        assert_eq!(m.keepouts[0].rect, Rect { x: 5, y: 5, w: 5, h: 5 });
        assert_eq!(m.units, vec![u]);
        assert_eq!(m.dummies, vec![d]);
    }

    #[test]
    fn finish_keeps_one_of_each_exact_duplicate_only() {
        let mut b = Builder::new(5);
        let r = Rect { x: 0, y: 0, w: 10, h: 10 };
        b.rect(LayerId(0), r);
        b.rect(LayerId(0), r);
        b.rect(LayerId(1), r);
        b.rect(LayerId(0), Rect { x: 5, ..r });
        let m = b.finish();
        assert_eq!(m.shapes.len(), 3);
        assert_eq!(m.shapes[0].rect, r, "first occurrence kept, order stable");
    }

    #[test]
    fn finish_of_nothing_is_a_zero_bbox() {
        let m = Builder::new(5).finish();
        assert!(m.shapes.is_empty());
        assert_eq!(m.bbox, R0);
    }

    #[test]
    fn finish_bbox_encloses_on_the_lattice_for_negative_coordinates_and_no_grid() {
        for grid in [5, 0, -2] {
            let mut b = Builder::new(grid);
            b.rect(LayerId(0), Rect { x: -13, y: -27, w: 41, h: 9 });
            b.rect(LayerId(1), Rect { x: 40, y: 3, w: 11, h: 30 });
            let m = b.finish();
            let step = 2 * grid.max(1);
            assert_eq!(m.bbox.x.rem_euclid(step), 0, "grid {grid}");
            assert_eq!(m.bbox.y.rem_euclid(step), 0, "grid {grid}");
            assert_eq!(m.bbox.w % (2 * step), 0, "grid {grid}");
            assert_eq!(m.bbox.h % (2 * step), 0, "grid {grid}");
            for s in &m.shapes {
                assert!(contains(&m.bbox, &s.rect), "grid {grid}: {:?} outside {:?}", s.rect, m.bbox);
            }
        }
    }

    #[test]
    fn cut_lattice_is_two_grid_steps_and_never_below_two() {
        assert_eq!(cut_lattice(&Deck::new(5, &[])), 10);
        assert_eq!(cut_lattice(&Deck::new(0, &[])), 2);
        assert_eq!(cut_lattice(&Deck::new(-3, &[])), 2);
    }

    #[test]
    fn snap_cut_rounds_toward_negative_infinity() {
        for (v, lat, want) in [(15, 10, 10), (10, 10, 10), (0, 10, 0), (-1, 10, -10), (-10, 10, -10), (-11, 10, -20), (7, 1, 7)] {
            assert_eq!(snap_cut(v, lat), want, "snap_cut({v}, {lat})");
        }
    }

    #[test]
    #[should_panic]
    fn snap_cut_panics_on_a_zero_lattice() {
        let _ = snap_cut(5, 0);
    }

    #[test]
    fn bbox_of_covers_every_shape() {
        assert_eq!(bbox_of(&[]), R0);
        let s = |x, y, w, h| Shape { layer: LayerId(0), rect: Rect { x, y, w, h } };
        assert_eq!(bbox_of(&[s(1, 2, 3, 4)]), Rect { x: 1, y: 2, w: 3, h: 4 });
        assert_eq!(bbox_of(&[s(1, 2, 3, 4), s(-5, 10, 2, 2), s(0, 0, 0, 0)]), Rect { x: -5, y: 0, w: 9, h: 12 });
    }

    #[test]
    fn hull_and_contains_agree() {
        let a = Rect { x: 0, y: 0, w: 10, h: 10 };
        let b = Rect { x: 20, y: -5, w: 5, h: 5 };
        let h = hull(a, b);
        assert_eq!(h, Rect { x: 0, y: -5, w: 25, h: 15 });
        assert_eq!(hull(a, a), a);
        assert_eq!(hull(a, b), hull(b, a));
        assert!(contains(&h, &a) && contains(&h, &b));
        assert!(contains(&a, &a), "edges included");
        assert!(!contains(&a, &Rect { x: 1, y: 0, w: 10, h: 10 }));
        assert!(!contains(&a, &Rect { x: -1, y: 0, w: 1, h: 1 }));
    }

    #[test]
    fn req_resolves_a_declared_role() {
        assert_eq!(req(&Deck::new(5, &["diff", "poly"]), "poly"), LayerId(1));
    }

    #[test]
    #[should_panic(expected = "mandatory role")]
    fn req_panics_on_a_missing_role() {
        let _ = req(&Deck::new(5, &["diff"]), "poly");
    }

    #[test]
    fn dim_takes_the_larger_of_sidecar_and_deck() {
        let p = Deck::new(5, &[]);
        assert_eq!(dim(&p, "contact"), 0);
        assert_eq!(dim(&p, "no_such_key"), 0);
        let p = Deck::new(5, &[]).with("w:licon", 170);
        assert_eq!(dim(&p, "contact"), 170);
        assert_eq!(dim(&p.with("contact", 200), "contact"), 200, "sidecar raises");
        let p = Deck::new(5, &[]).with("w:licon", 170).with("contact", 100);
        assert_eq!(dim(&p, "contact"), 170, "sidecar never lowers");
        let p = Deck::new(5, &[]).with("enc:met1:mcon", 30).with("cap:met1:mcon", 60);
        assert_eq!(dim(&p, "m1_enc"), 60);
        assert_eq!(dim(&Deck::new(5, &[]).with("w:poly", 150), "min_gate_l"), 150);
        assert_eq!(dim(&Deck::new(5, &[]).with("custom", 42), "custom"), 42, "an unknown key reads the sidecar");
    }

    #[test]
    fn pin_names_and_placeholder_nets_are_distinct_per_member_and_terminal() {
        let at = Rect { x: 1, y: 2, w: 3, h: 4 };
        let p = pin(3, "G", at, LayerId(9));
        assert_eq!((p.name.as_str(), p.at, p.layer), ("d3:G", at, LayerId(9)));
        let mut nets = Vec::new();
        for di in 0..64 {
            for t in ["G", "S", "D", "B", "P", "N", "C", "X"] {
                nets.push(pin(di, t, at, LayerId(0)).net);
            }
        }
        let n = nets.len();
        nets.sort_by_key(|n| n.0);
        nets.dedup();
        assert_eq!(nets.len(), n);
        assert_eq!(pin(5, "X", at, LayerId(0)).net, pin(5, "Y", at, LayerId(0)).net, "unknown terminals share slot 7");
    }

    #[test]
    fn unitization_needs_a_cover_of_every_member() {
        let c = constraints(vec![uz(&[0, 1], &[1, 1], 0, 0), uz(&[1, 2, 3], &[1, 1, 1], 0, 0), uz(&[2, 3], &[1, 1], 0, 0)]);
        assert!(unitization(&group(&[]), &c).is_none());
        assert_eq!(unitization(&group(&[1, 0]), &c).map(|u| u.devices.len()), Some(2));
        assert_eq!(unitization(&group(&[3]), &c).map(|u| u.devices.len()), Some(3), "first covering wins");
        assert!(unitization(&group(&[0, 2]), &c).is_none());
        assert!(unitization(&group(&[0]), &Constraints::default()).is_none());
    }

    #[test]
    fn sizing_defaults_without_a_unitization() {
        let s = sizing(&group(&[0, 1, 2]), &Constraints::default(), 420, 150);
        assert_eq!((s.unit_w, s.unit_l, s.dev_nf), (420, 150, vec![1, 1, 1]));
        let s = sizing(&group(&[]), &Constraints::default(), 420, 150);
        assert_eq!(s.dev_nf, vec![1], "one entry for an empty group");
    }

    #[test]
    fn sizing_reads_each_member_s_own_slot() {
        let c = constraints(vec![uz(&[5, 6, 7], &[2, 0, 4], 1000, 0)]);
        let s = sizing(&group(&[7, 5, 6]), &c, 420, 150);
        assert_eq!(s.dev_nf, vec![4, 2, 1], "own slot, zero reads 1");
        assert_eq!((s.unit_w, s.unit_l), (1000, 150), "non-positive l falls back");
        // A short `dev_nf` column: the missing slot reads 1.
        let c = constraints(vec![uz(&[5, 6], &[3], -1, 200)]);
        let s = sizing(&group(&[5, 6]), &c, 420, 150);
        assert_eq!((s.unit_w, s.unit_l, s.dev_nf), (420, 200, vec![3, 1]));
    }

    fn poly_deck() -> Deck {
        Deck::new(5, &["poly", "licon", "npc"]).with("enc:npc:licon", 50)
    }

    fn npc_rects(m: &Macro) -> Vec<Rect> {
        m.shapes.iter().filter(|s| s.layer == LayerId(2)).map(|s| s.rect).collect()
    }

    #[test]
    fn cover_poly_cuts_does_nothing_without_the_role_or_when_it_is_poly() {
        for p in [Deck::new(5, &["poly", "licon"]), Deck::new(5, &["poly", "licon"]).alias("npc", 0)] {
            let mut b = Builder::new(5);
            b.rect(LayerId(0), Rect { x: -100, y: -100, w: 400, h: 400 });
            b.rect(LayerId(1), Rect { x: 0, y: 0, w: 170, h: 170 });
            b.cover_poly_cuts(&p);
            assert_eq!(b.finish().shapes.len(), 2);
        }
    }

    #[test]
    fn cover_poly_cuts_encloses_only_cuts_on_poly() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: -100, y: -100, w: 400, h: 400 });
        b.rect(LayerId(1), Rect { x: 0, y: 0, w: 170, h: 170 });
        b.rect(LayerId(1), Rect { x: 1000, y: 0, w: 170, h: 170 }); // on diffusion
        b.cover_poly_cuts(&poly_deck());
        assert_eq!(npc_rects(&b.finish()), vec![Rect { x: -50, y: -50, w: 270, h: 270 }]);
    }

    #[test]
    fn cover_poly_cuts_joins_a_row_and_near_rows() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: -100, y: -100, w: 2000, h: 2000 });
        b.rect(LayerId(1), Rect { x: 0, y: 0, w: 170, h: 170 });
        b.rect(LayerId(1), Rect { x: 1000, y: 0, w: 170, h: 170 });
        b.rect(LayerId(1), Rect { x: 0, y: 1000, w: 170, h: 170 });
        let mut far = Builder::new(5);
        far.shapes = b.shapes.clone();
        // No spacing: one strip per row, two rows apart.
        b.cover_poly_cuts(&poly_deck());
        let mut rows = npc_rects(&b.finish());
        rows.sort_by_key(|r| r.y);
        assert_eq!(rows, vec![Rect { x: -50, y: -50, w: 1270, h: 270 }, Rect { x: -50, y: 950, w: 270, h: 270 }]);
        // Rows closer than the mask spacing merge into one figure.
        far.cover_poly_cuts(&poly_deck().with("s:npc", 1000));
        assert_eq!(npc_rects(&far.finish()), vec![Rect { x: -50, y: -50, w: 1270, h: 1270 }]);
    }

    #[test]
    fn cover_poly_cuts_grows_a_strip_to_the_mask_width_about_its_centre() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: -100, y: -100, w: 400, h: 400 });
        b.rect(LayerId(1), Rect { x: 0, y: 0, w: 170, h: 170 });
        b.cover_poly_cuts(&poly_deck().with("w:npc", 400));
        assert_eq!(npc_rects(&b.finish()), vec![Rect { x: -115, y: -115, w: 400, h: 400 }]);
    }
}
