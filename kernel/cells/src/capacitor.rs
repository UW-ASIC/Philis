//! Capacitor generator: one merged plate (or comb) per device, sized by its
//! unit count; the variant axes are array aspect and metal-stack kind.

use crate::builder::dim;
use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, Process, Rect};

use crate::builder::{pin, sizing, Builder, Sizing};
use crate::Cell;

/// The metal stack a capacitor occupies:
///
/// - [`Kind::VerticalInOneLayer`] — lateral MOM comb on **one** metal.
///   Interdigitated A/B fingers; the flux is sidewall-to-sidewall, so the two
///   electrodes share a layer and must never overlap.
/// - [`Kind::HorizontalAcrossLayers`] — parallel plate across **two** metals
///   (`met_n` bottom, `met_n+1` top). Vertical flux through the ILD.
/// - [`Kind::VerticalAcrossLayers`] — sandwich across **three** metals: BOT on
///   `met_n` *and* `met_n+2`, TOP on `met_n+1` between them, so both faces of the
///   top plate couple (≈2× the two-metal C for the same plan area). The two BOT
///   levels are strapped in the bus column, never through the plate stack.
///
/// No LVS marker is drawn: the comb's many same-layer polygons cannot bind to
/// one terminal slot, so capacitors are an LVS reference skip. Nothing may be
/// drawn between stacked plates (a cut there shorts the device); the router is
/// not told this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Three-metal sandwich (`met_n`/`met_n+1`/`met_n+2`); needs three metals
    /// and two cut layers.
    VerticalAcrossLayers,
    /// Two-metal parallel plate (`met_n`/`met_n+1`); needs two metals.
    HorizontalAcrossLayers,
    /// Single-metal lateral comb; needs one metal.
    VerticalInOneLayer,
}

/// One capacitor variant. `units_x` is the column count of the unit grid and
/// only picks the aspect ratio: a device's units are drawn as one plate.
///
/// Pins: `P` is the top plate, `N` the bottom plate.
#[derive(Clone)]
pub struct Capacitor {
    /// Unit-grid columns per device; a device with fewer units uses its unit
    /// count. `0` draws as `1`.
    pub units_x: u16,
    /// The metal stack drawn.
    pub kind: Kind,
}

/// Bound on the reshape search: a highly composite unit count would otherwise
/// offer `divisors × kinds` variants for the placer to try.
const MAX_VARIANTS: usize = 16;

impl Cell for Capacitor {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let kinds = feasible_kinds(process);
        if kinds.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        // u32: members' unit counts may sum past u16::MAX.
        let units = s.dev_nf.iter().map(|&n| u32::from(n)).sum::<u32>().max(1);
        let columns = (1..=units.min(u32::from(u16::MAX))).filter(|c| units % c == 0).map(|c| c as u16);

        // Column-outer / kind-inner so the `MAX_VARIANTS` cut keeps a spread of
        // stacks rather than every column of the first kind.
        columns
            .flat_map(|units_x| kinds.iter().map(move |&kind| Capacitor { units_x, kind }))
            .take(MAX_VARIANTS)
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let g = Geom::new(self, &s, process);

        let mut x0 = 0;
        for (di, &nf) in s.dev_nf.iter().enumerate() {
            let n_units = i32::from(nf.max(1));
            let plate = Rect { x: x0, y: 0, w: g.grid_w(n_units), h: g.grid_h(n_units) };
            let (bot_pin, top_pin) = match self.kind {
                Kind::VerticalInOneLayer => g.comb(&mut b, plate),
                Kind::HorizontalAcrossLayers => g.plates(&mut b, plate, None),
                Kind::VerticalAcrossLayers => g.plates(&mut b, plate, g.third_metal),
            };
            b.pin(pin(di, "N", bot_pin, g.bot_metal));
            b.pin(pin(di, "P", top_pin, g.top_metal));
            // Never abut two devices' electrodes: a deck without `device_gap`
            // still gets a metal spacing between tiles.
            x0 += g.tile_w(n_units) + g.device_gap.max(g.m_space);
        }

        b.finish()
    }
}

/// Every dimension (nm) and layer the three kinds draw from, resolved once
/// per [`Cell::draw`] from the deck.
struct Geom {
    /// BOT electrode metal (`met_n`), also the comb's only metal.
    bot_metal: LayerId,
    /// TOP electrode metal: `met_n+1`, or `met_n` for the comb.
    top_metal: LayerId,
    /// `met_n+2` — present only for the three-metal sandwich.
    third_metal: Option<LayerId>,
    /// The two cut layers strapping `met_n`↔`met_n+2` (`via_n`, `via_n+1`).
    cuts: [LayerId; 2],
    /// Drawn width of each cut — via layers carry exact (min = max) widths, so
    /// each cut is sized from its own layer's rule, not one shared `contact`.
    cut_w: [i32; 2],
    /// One unit's plate width and height.
    unit_w: i32,
    unit_h: i32,
    /// Worst same-metal spacing over `met1..=met3` (plate to strap column).
    m_space: i32,
    /// Gap between consecutive devices' tiles.
    device_gap: i32,
    /// TOP plate inset from the BOT edge (`plate_spacing`).
    inset: i32,
    /// Comb finger width and finger-to-finger space.
    finger_w: i32,
    finger_space: i32,
    /// Metal enclosure of each strap cut.
    via_enc: i32,
    /// Vertical step between cut rows: the worst `size + spacing` over both cut
    /// layers, so neither layer's min-spacing is violated by the shared pitch.
    via_pitch: i32,
    /// Column cap from [`Capacitor::units_x`], `>= 1`.
    max_cols: i32,
    /// Width of the `met_n`↔`met_n+2` strap column, `0` for the non-sandwich
    /// kinds. It sits **beside** the plates: a cut inside the stack would pierce
    /// the dielectric and short the device.
    strap_w: i32,
}

impl Geom {
    fn new(spec: &Capacitor, s: &Sizing, process: &dyn Process) -> Self {
        let m = metals(process);
        let v = vias(process);
        let met = |i: usize| m.get(i).copied().unwrap_or(LayerId(0));
        let ct = dim(process, "contact");
        let via_enc = dim(process, "via_enclosure");
        let sandwich = spec.kind == Kind::VerticalAcrossLayers;
        let unit_gap = process.rule("plate_spacing", 0);
        // Per-layer cut dimensions, the deck's: each cut layer's exact width
        // and its own (array) spacing.
        let via_spacing = dim(process, "via_spacing");
        let cut_w = [process.width("via1").unwrap_or(ct), process.width("via2").unwrap_or(ct)];
        let via_pitch = (cut_w[0] + process.space("via1").unwrap_or(via_spacing))
            .max(cut_w[1] + process.space("via2").unwrap_or(via_spacing));
        // The strap column carries rails on every sandwich metal beside a plate
        // that is usually wide: clear the worst (wide-)spacing of all three.
        let m_space = (1..=3)
            .filter_map(|n| process.space(&format!("met{n}")))
            .fold(dim(process, "met1_space"), i32::max);
        // Fingers: the deck's MOM pitch, never under the comb metal's own
        // width and spacing (the MOM keys may be another metal's).
        let comb = ["met1", "met2"];
        // At least one grid step each: a deck stating neither must not leave
        // the comb a zero pitch.
        let step = process.grid().max(1);
        let finger_w = comb.iter().filter_map(|m| process.width(m)).fold(process.rule("mom_finger_width", 0), i32::max).max(step);
        let finger_space = comb.iter().filter_map(|m| process.space(m)).fold(process.rule("mom_finger_space", 0), i32::max).max(step);
        // The smallest unit every kind draws legally: two spines and two
        // fingers of the comb, which also exceeds every plate metal's width.
        let min_side = 2 * finger_w + 2 * finger_space;
        Self {
            bot_metal: met(0),
            // The comb keeps both electrodes on one metal — that *is* the kind.
            top_metal: if spec.kind == Kind::VerticalInOneLayer { met(0) } else { met(1) },
            third_metal: sandwich.then(|| met(2)),
            cuts: [v.first().copied().unwrap_or(LayerId(0)), v.get(1).copied().unwrap_or(LayerId(0))],
            cut_w,
            unit_w: s.unit_w.max(min_side),
            unit_h: s.unit_l.max(min_side),
            m_space,
            device_gap: process.rule("device_gap", 0),
            inset: unit_gap,
            finger_w,
            finger_space,
            via_enc,
            via_pitch,
            max_cols: i32::from(spec.units_x.max(1)),
            strap_w: if sandwich { cut_w[0].max(cut_w[1]) + 2 * via_enc } else { 0 },
        }
    }

    /// Columns of an `n_units` device's grid: `units_x`, capped by the units.
    fn cols(&self, n_units: i32) -> i32 {
        self.max_cols.min(n_units).max(1)
    }

    /// Rows holding `n_units` at [`Self::cols`] per row (last row may be partial).
    fn rows(&self, n_units: i32) -> i32 {
        let cols = self.cols(n_units);
        (n_units + cols - 1) / cols
    }

    /// Plate width for `n_units` — the unit grid merged into one plate, so the
    /// `units_x × units_y` split only picks the aspect ratio. No gap terms: a
    /// full grid's plate is exactly n·W·L (CELL-08).
    fn grid_w(&self, n_units: i32) -> i32 {
        self.cols(n_units) * self.unit_w
    }

    /// Plate height for `n_units`; see [`Self::grid_w`].
    fn grid_h(&self, n_units: i32) -> i32 {
        self.rows(n_units) * self.unit_h
    }

    /// One device's full footprint: the plate plus, for the sandwich, its strap
    /// column.
    fn tile_w(&self, n_units: i32) -> i32 {
        self.grid_w(n_units) + if self.strap_w > 0 { self.m_space + self.strap_w } else { 0 }
    }

    /// Stacked plates: BOT fills `plate`, TOP is inset so the plate edges never
    /// align (fringe + misalignment tolerance); a plate too small to inset
    /// keeps TOP flush with BOT. `third` adds a second BOT level above TOP,
    /// doubling the coupled area, strapped in a side column.
    /// **Nothing is drawn between the plates** — that gap is the dielectric.
    ///
    /// Returns `(BOT pin, TOP pin)`: BOT's exposed bottom border (or, for the
    /// sandwich, its strap rail) and the TOP plate.
    fn plates(&self, b: &mut Builder, plate: Rect, third: Option<LayerId>) -> (Rect, Rect) {
        b.rect(self.bot_metal, plate);
        let i = self.inset;
        let top = if plate.w > 2 * i && plate.h > 2 * i {
            Rect { x: plate.x + i, y: plate.y + i, w: plate.w - 2 * i, h: plate.h - 2 * i }
        } else {
            plate
        };
        b.rect(self.top_metal, top);

        let Some(third) = third else {
            // BOT's exposed border is the only place a router can land on it.
            return (Rect { x: plate.x, y: plate.y, w: plate.w, h: i.max(1) }, top);
        };
        b.rect(third, plate);
        // Strap column, clear of the plates by `m_space`: met_n and met_n+2 rails
        // joined by a met_n+1 jumper and two via stacks. Running this through the
        // plate stack instead would short TOP to BOT.
        let sx = plate.x + plate.w + self.m_space;
        let rail = Rect { x: sx, y: plate.y, w: self.strap_w, h: plate.h };
        for l in [self.bot_metal, self.top_metal, third] {
            b.rect(l, rail);
        }
        // Bridge the gap on the BOT levels only: without it both BOT plates float
        // (ERC `floating_interconnect`). The met_n+1 jumper keeps its gap to TOP.
        let bridge = Rect { x: plate.x + plate.w, y: plate.y, w: self.m_space, h: plate.h };
        for l in [self.bot_metal, third] {
            b.rect(l, bridge);
        }
        let max_cut = self.cut_w[0].max(self.cut_w[1]);
        let mut vy = plate.y + self.via_enc;
        // A deck stating no cut size or spacing gets no cuts, not a hang.
        while self.via_pitch > 0 && vy + max_cut + self.via_enc <= plate.y + plate.h {
            for (cut, w) in self.cuts.into_iter().zip(self.cut_w) {
                b.rect(cut, Rect { x: sx + self.via_enc, y: vy, w, h: w });
            }
            vy += self.via_pitch;
        }
        (rail, top)
    }

    /// Lateral MOM comb on one metal: horizontal spines at the bottom (BOT) and
    /// top (TOP) of `plate`, with alternating vertical fingers between them. The
    /// two electrodes **share a layer**, so no BOT rect may touch a TOP rect —
    /// that separation is what makes this a capacitor rather than a short.
    /// A plate too small for two spines and two fingers degrades to two
    /// side-by-side plates `finger_space` apart.
    ///
    /// Returns `(BOT spine, TOP spine)`.
    fn comb(&self, b: &mut Builder, plate: Rect) -> (Rect, Rect) {
        let (x, y, w, h) = (plate.x, plate.y, plate.w, plate.h);
        let (mut fw, fs) = (self.finger_w, self.finger_space);
        let m = self.bot_metal;

        // Too small to seat two spines and a finger: degrade to a two-plate
        // lateral cap. Still real, still non-overlapping, just no fingers.
        if w < 2 * fw + fs || h < 2 * fw + 2 * fs {
            let half = ((w - fs) / 2).max(1);
            let (bot, top) = (
                Rect { x, y, w: half, h },
                Rect { x: x + half + fs, y, w: w - half - fs, h },
            );
            b.rect(m, bot);
            b.rect(m, top);
            return (bot, top);
        }

        let pitch = fw + fs;
        // Even finger count ⇒ equal A/B counts. Fingers must tile inside the
        // plate: shrink the finger, never the pitch, when only two fit.
        let mut n = (w + fs) / pitch;
        n -= i32::from(n % 2 == 1);
        if n < 2 {
            n = 2;
            fw = ((w - fs) / 2).max(1);
        }

        let bot = Rect { x, y, w, h: fw };
        let top = Rect { x, y: y + h - fw, w, h: fw };
        b.rect(m, bot);
        b.rect(m, top);

        // Each finger runs from inside its own spine to `fs` short of the other,
        // so it is one conductor with its spine and never reaches its partner.
        let reach = h - fw - fs;
        for i in 0..n {
            let fx = x + i * pitch;
            let fy = if i % 2 == 0 { y } else { y + fw + fs };
            b.rect(m, Rect { x: fx, y: fy, w: fw, h: reach });
        }
        (bot, top)
    }
}

/// The PDK's metal stack bottom-up, as far as it is populated. `map_while` stops
/// at the first absent level, so a deck with only `met1` yields one entry.
fn metals(process: &dyn Process) -> Vec<LayerId> {
    (1..=5).map_while(|n| process.layer(&format!("met{n}"))).collect()
}

/// Cut layers between consecutive metals (`via1` joins met1↔met2, …).
fn vias(process: &dyn Process) -> Vec<LayerId> {
    (1..=4).map_while(|n| process.layer(&format!("via{n}"))).collect()
}

/// Kinds this process can actually build — a deck without `met2` cannot stack a
/// plate, and without two cut layers cannot strap a sandwich.
fn feasible_kinds(process: &dyn Process) -> Vec<Kind> {
    let (m, v) = (metals(process).len(), vias(process).len());
    let mut kinds = Vec::new();
    if m >= 1 {
        kinds.push(Kind::VerticalInOneLayer);
    }
    if m >= 2 {
        kinds.push(Kind::HorizontalAcrossLayers);
    }
    if m >= 3 && v >= 2 {
        kinds.push(Kind::VerticalAcrossLayers);
    }
    kinds
}

/// The group's capacitor sizing: its unitization, else one square unit per
/// device of the deck's `cap_unit_side` (0 when the deck states none). Shared
/// with [`crate::cap_array`].
pub(crate) fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def = process.rule("cap_unit_side", 0);
    sizing(group, c, def, def)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, drawn alone, is DRC- and ERC-clean.
    #[test]
    fn every_variant_is_drc_and_erc_clean() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let mut dirty = Vec::new();
        for n in [1, 2] {
            dirty.extend(testkit::dirty::<Capacitor>(DeviceKind::Capacitor, n, 4, 2000, 2000, &pdk));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }
    use analog::cell::{SeriesParallel, Unitization};
    use pnr_core::{DeviceId, DeviceKind, Shape};

    /// A sky130-shaped deck: five metals, four vias, default rule values.
    struct TestPdk {
        metals: usize,
        vias: usize,
    }

    impl TestPdk {
        fn full() -> Self {
            Self { metals: 5, vias: 4 }
        }
    }

    impl Process for TestPdk {
        fn layer(&self, role: &str) -> Option<LayerId> {
            let idx = |p: &str, n: usize| {
                role.strip_prefix(p).and_then(|d| d.parse::<usize>().ok()).filter(|&i| i <= n)
            };
            idx("met", self.metals)
                .map(|i| LayerId(i as u16))
                .or_else(|| idx("via", self.vias).map(|i| LayerId(100 + i as u16)))
        }
        fn rule(&self, _name: &str, default: i32) -> i32 {
            default
        }
        fn grid(&self) -> i32 {
            5
        }
        fn width(&self, role: &str) -> Option<i32> {
            self.layer(role).map(|_| 200)
        }
        fn space(&self, role: &str) -> Option<i32> {
            self.layer(role).map(|_| 200)
        }
    }

    fn one_cap(units: u16, side: i32) -> (DeviceGroup, Constraints) {
        let group = DeviceGroup { devices: vec![DeviceId(0)] };
        let c = Constraints {
            unitization: vec![Unitization {
                devices: vec![DeviceId(0)],
                device_type: DeviceKind::Capacitor,
                dev_nf: vec![units],
                target_ratio: vec![1],
                unit_w: side,
                unit_l: side,
                series_parallel: SeriesParallel::Series,
                dummy_required: false,
                route_matching_required: false,
                class: None, kind: None, series: Vec::new(), style: None,
            }],
            ..Default::default()
        };
        (group, c)
    }

    fn overlaps(a: &Shape, b: &Shape) -> bool {
        a.layer == b.layer
            && a.rect.x < b.rect.x + b.rect.w
            && b.rect.x < a.rect.x + a.rect.w
            && a.rect.y < b.rect.y + b.rect.h
            && b.rect.y < a.rect.y + a.rect.h
    }

    /// A same-layer comb whose electrodes touch is a short, not a capacitor —
    /// the exact defect the old `top_layer = met1` fallthrough produced.
    #[test]
    fn comb_electrodes_never_short() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(4, 2000);
        let spec = Capacitor { units_x: 2, kind: Kind::VerticalInOneLayer };
        let m = spec.draw(&group, &c, &pdk);
        // Every rect is on one metal, so BOT and TOP are distinguished purely by
        // geometry: any contact at all merges the two electrodes.
        let bot_spine = m.pins.iter().find(|p| p.name.ends_with(":N")).unwrap().at;
        let top_spine = m.pins.iter().find(|p| p.name.ends_with(":P")).unwrap().at;
        assert_ne!(bot_spine, top_spine);

        // Flood-fill from the BOT spine; the TOP spine must stay unreached.
        let mut reached: Vec<bool> = m.shapes.iter().map(|s| s.rect == bot_spine).collect();
        loop {
            let mut grew = false;
            for i in 0..m.shapes.len() {
                if reached[i] {
                    continue;
                }
                if (0..m.shapes.len())
                    .any(|j| reached[j] && overlaps(&m.shapes[i], &m.shapes[j]))
                {
                    reached[i] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        for (i, s) in m.shapes.iter().enumerate() {
            assert!(!(reached[i] && s.rect == top_spine), "TOP spine reachable from BOT — shorted");
        }
        assert!(reached.iter().filter(|r| **r).count() > 1, "BOT spine must reach its fingers");
    }

    /// CELL-08: four 2 µm units merge into one 16 µm² plate in every aspect.
    #[test]
    fn a_merged_plate_is_the_units_area() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(4, 2000);
        for units_x in [1, 2, 4] {
            let m = Capacitor { units_x, kind: Kind::HorizontalAcrossLayers }.draw(&group, &c, &pdk);
            let plate = m.shapes.iter().filter(|s| s.layer.0 == 1).map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).max();
            assert_eq!(plate, Some(16_000_000), "units_x={units_x}");
        }
    }

    /// The three kinds must occupy three different metal stacks; before this they
    /// collapsed to two identical shape sets plus a short.
    #[test]
    fn kinds_draw_distinct_stacks() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(1, 2000);
        let stack = |kind| {
            let spec = Capacitor { units_x: 1, kind };
            let mut ls: Vec<u16> =
                spec.draw(&group, &c, &pdk).shapes.iter().map(|s| s.layer.0).collect();
            ls.sort_unstable();
            ls.dedup();
            ls
        };
        assert_eq!(stack(Kind::VerticalInOneLayer), vec![1], "comb is single-metal");
        assert_eq!(stack(Kind::HorizontalAcrossLayers), vec![1, 2], "plate is met1+met2");
        // met1 + met2 + met3 + via1 + via2.
        assert_eq!(stack(Kind::VerticalAcrossLayers), vec![1, 2, 3, 101, 102]);
    }

    /// `enumerate` used to be unbounded: 3 kinds × 2 patterns × every divisor.
    #[test]
    fn enumerate_is_bounded_and_covers_the_stacks() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(64, 2000);
        let specs = Capacitor::enumerate(&group, &c, &pdk);
        assert!(specs.len() <= MAX_VARIANTS, "got {} variants", specs.len());
        for kind in feasible_kinds(&pdk) {
            assert!(specs.iter().any(|s| s.kind == kind), "{kind:?} missing from the space");
        }
    }

    /// A deck with one metal can only build the comb — and must not panic
    /// reaching for a `met2` that does not exist.
    #[test]
    fn single_metal_deck_degrades_to_the_comb() {
        let pdk = TestPdk { metals: 1, vias: 0 };
        let (group, c) = one_cap(2, 2000);
        assert_eq!(feasible_kinds(&pdk), vec![Kind::VerticalInOneLayer]);
        let specs = Capacitor::enumerate(&group, &c, &pdk);
        assert!(!specs.is_empty());
        assert!(specs.iter().all(|s| s.kind == Kind::VerticalInOneLayer));
    }

    /// A configurable deck: `metals`/`vias` populated levels, named rule
    /// overrides, and (when `sized`) a 200 nm width and spacing on every layer.
    struct Deck {
        metals: usize,
        vias: usize,
        rules: &'static [(&'static str, i32)],
        sized: bool,
    }

    impl Process for Deck {
        fn layer(&self, role: &str) -> Option<LayerId> {
            TestPdk { metals: self.metals, vias: self.vias }.layer(role)
        }
        fn rule(&self, name: &str, default: i32) -> i32 {
            self.rules.iter().find(|(k, _)| *k == name).map_or(default, |r| r.1)
        }
        fn grid(&self) -> i32 {
            5
        }
        fn width(&self, role: &str) -> Option<i32> {
            self.layer(role).filter(|_| self.sized).map(|_| 200)
        }
        fn space(&self, role: &str) -> Option<i32> {
            self.layer(role).filter(|_| self.sized).map(|_| 200)
        }
    }

    /// One unitized group of `counts.len()` capacitors, `side`-nm units.
    fn caps(counts: &[u16], side: i32) -> (DeviceGroup, Constraints) {
        let devices: Vec<DeviceId> = (0..counts.len() as u16).map(DeviceId).collect();
        let (_, mut c) = one_cap(1, side);
        c.unitization[0].devices = devices.clone();
        c.unitization[0].dev_nf = counts.to_vec();
        c.unitization[0].target_ratio = counts.to_vec();
        (DeviceGroup { devices }, c)
    }

    const KINDS: [Kind; 3] = [Kind::VerticalInOneLayer, Kind::HorizontalAcrossLayers, Kind::VerticalAcrossLayers];

    #[test]
    fn an_empty_group_has_no_variants() {
        let group = DeviceGroup { devices: vec![] };
        assert!(Capacitor::enumerate(&group, &Constraints::default(), &TestPdk::full()).is_empty());
    }

    #[test]
    fn a_metal_less_deck_has_no_variants() {
        let (group, c) = one_cap(4, 2000);
        assert!(Capacitor::enumerate(&group, &c, &TestPdk { metals: 0, vias: 0 }).is_empty());
    }

    /// Feasibility per stack, boundaries included: the sandwich needs three
    /// metals *and* two cuts.
    #[test]
    fn feasible_kinds_follow_the_stack() {
        use Kind::*;
        let k = |metals, vias| feasible_kinds(&TestPdk { metals, vias });
        assert_eq!(k(0, 0), vec![]);
        assert_eq!(k(1, 4), vec![VerticalInOneLayer]);
        assert_eq!(k(2, 0), vec![VerticalInOneLayer, HorizontalAcrossLayers]);
        assert_eq!(k(3, 1), vec![VerticalInOneLayer, HorizontalAcrossLayers]);
        assert_eq!(k(3, 2), vec![VerticalInOneLayer, HorizontalAcrossLayers, VerticalAcrossLayers]);
    }

    /// Six units: every divisor (1, 2, 3, 6) in every kind, column-outer.
    #[test]
    fn enumerate_offers_every_divisor_in_every_kind() {
        let (group, c) = one_cap(6, 2000);
        let specs = Capacitor::enumerate(&group, &c, &TestPdk::full());
        let got: Vec<(u16, Kind)> = specs.iter().map(|s| (s.units_x, s.kind)).collect();
        let want: Vec<(u16, Kind)> = [1, 2, 3, 6].iter().flat_map(|&c| KINDS.iter().map(move |&k| (c, k))).collect();
        assert_eq!(got, want);
    }

    /// Units summing past `u16::MAX` must not overflow the column search, and
    /// every offered column count divides the true total.
    #[test]
    fn a_unit_total_past_u16_does_not_overflow() {
        let (group, c) = caps(&[40_000, 40_000], 2000);
        let specs = Capacitor::enumerate(&group, &c, &TestPdk::full());
        assert!(!specs.is_empty() && specs.len() <= MAX_VARIANTS);
        assert!(specs.iter().all(|s| 80_000 % u32::from(s.units_x) == 0), "{:?}", specs.iter().map(|s| s.units_x).collect::<Vec<_>>());
    }

    /// `cols` caps at the units and never drops under 1; `rows` rounds up.
    #[test]
    fn grid_shape_rounds_up_and_caps() {
        let (_, c) = one_cap(1, 1000);
        let s = group_sizing(&DeviceGroup { devices: vec![DeviceId(0)] }, &c, &TestPdk::full());
        let g = |units_x| Geom::new(&Capacitor { units_x, kind: Kind::HorizontalAcrossLayers }, &s, &TestPdk::full());
        assert_eq!((g(0).cols(4), g(0).rows(4)), (1, 4), "units_x 0 draws as 1");
        assert_eq!((g(3).cols(4), g(3).rows(4)), (3, 2), "partial last row");
        assert_eq!((g(3).cols(2), g(3).rows(2)), (2, 1), "capped by the units");
        assert_eq!(g(3).grid_w(4), 3000);
        assert_eq!(g(3).grid_h(4), 2000);
    }

    /// Every device of every kind gets exactly one `N` on BOT's metal and one
    /// `P` on TOP's.
    #[test]
    fn every_device_has_one_n_and_one_p_pin() {
        let pdk = TestPdk::full();
        let (group, c) = caps(&[1, 2, 3], 2000);
        for kind in KINDS {
            let m = Capacitor { units_x: 2, kind }.draw(&group, &c, &pdk);
            let top = if kind == Kind::VerticalInOneLayer { 1 } else { 2 };
            for d in 0..3 {
                let n: Vec<_> = m.pins.iter().filter(|p| p.name == format!("d{d}:N")).collect();
                let p: Vec<_> = m.pins.iter().filter(|p| p.name == format!("d{d}:P")).collect();
                assert_eq!((n.len(), p.len()), (1, 1), "{kind:?} d{d}");
                assert_eq!((n[0].layer.0, p[0].layer.0), (1, top), "{kind:?} d{d}");
            }
        }
    }

    /// Two devices side by side: device 0's geometry stays a metal spacing
    /// clear of device 1's, even on a deck that states no `device_gap`
    /// (abutting electrodes of two devices are a short).
    #[test]
    fn neighbouring_devices_are_a_spacing_apart() {
        let pdk = TestPdk::full();
        let (group, c) = caps(&[2, 3], 2000);
        for kind in KINDS {
            let m = Capacitor { units_x: 2, kind }.draw(&group, &c, &pdk);
            let split = m.pins.iter().find(|p| p.name == "d1:P").unwrap().at.x;
            let end0 = m.shapes.iter().filter(|s| s.rect.x < split).map(|s| s.rect.x + s.rect.w).max().unwrap();
            assert!(end0 + 200 <= split, "{kind:?}: device 0 ends at {end0}, device 1 starts at {split}");
        }
    }

    /// A unit too small to draw (here the deck states no `cap_unit_side` and
    /// the group no unitization: side 0) still draws every metal at least its
    /// min width, and the comb's electrodes stay apart.
    #[test]
    fn a_zero_unit_still_draws_legal_metal() {
        let pdk = TestPdk::full();
        let group = DeviceGroup { devices: vec![DeviceId(0)] };
        let c = Constraints::default();
        for kind in KINDS {
            let m = Capacitor { units_x: 1, kind }.draw(&group, &c, &pdk);
            for s in m.shapes.iter().filter(|s| s.layer.0 <= 5) {
                assert!(s.rect.w >= 200 && s.rect.h >= 200, "{kind:?}: {:?} under the 200 nm min width", s.rect);
            }
        }
        let m = Capacitor { units_x: 1, kind: Kind::VerticalInOneLayer }.draw(&group, &c, &pdk);
        let (n, p) = (m.pins.iter().find(|p| p.name.ends_with(":N")).unwrap().at, m.pins.iter().find(|p| p.name.ends_with(":P")).unwrap().at);
        let gap = (p.x - (n.x + n.w)).max(n.x - (p.x + p.w)).max(p.y - (n.y + n.h)).max(n.y - (p.y + p.h));
        assert!(gap >= 200, "comb electrodes {n:?} / {p:?} closer than finger space");
    }

    /// A deck with no MOM keys and no metal widths must not divide by a zero
    /// finger pitch; the comb still separates its electrodes.
    #[test]
    fn a_comb_without_finger_rules_does_not_panic() {
        let pdk = Deck { metals: 1, vias: 0, rules: &[], sized: false };
        let (group, c) = one_cap(2, 2000);
        let m = Capacitor { units_x: 1, kind: Kind::VerticalInOneLayer }.draw(&group, &c, &pdk);
        let n = m.pins.iter().find(|p| p.name.ends_with(":N")).unwrap().at;
        let p = m.pins.iter().find(|p| p.name.ends_with(":P")).unwrap().at;
        assert_ne!(n, p);
        assert!(!m.shapes.is_empty());
    }

    /// TOP is inset by `plate_spacing` on every side when the plate has room,
    /// and stays flush with BOT when it does not.
    #[test]
    fn top_plate_insets_only_when_it_fits() {
        let pdk = Deck { metals: 5, vias: 4, rules: &[("plate_spacing", 600)], sized: true };
        let top = |side| {
            let (group, c) = one_cap(1, side);
            let m = Capacitor { units_x: 1, kind: Kind::HorizontalAcrossLayers }.draw(&group, &c, &pdk);
            m.shapes.iter().find(|s| s.layer.0 == 2).unwrap().rect
        };
        assert_eq!(top(2000), Rect { x: 600, y: 600, w: 800, h: 800 });
        assert_eq!(top(1200), Rect { x: 0, y: 0, w: 1200, h: 1200 }, "w == 2·inset: flush");
    }

    /// The sandwich's strap column: one via1 and one via2 per cut row, rows a
    /// pitch apart, every cut inside the rail, none under the plates.
    #[test]
    fn sandwich_cuts_sit_in_the_strap_column_only() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(1, 2000);
        let m = Capacitor { units_x: 1, kind: Kind::VerticalAcrossLayers }.draw(&group, &c, &pdk);
        let cuts = |l: u16| m.shapes.iter().filter(|s| s.layer.0 == l).map(|s| s.rect).collect::<Vec<_>>();
        let (v1, v2) = (cuts(101), cuts(102));
        // Cut 200, spacing 200, no enclosure: rows at 0, 400, …, 1600 in 2000.
        assert_eq!(v1.len(), 5);
        assert_eq!(v1.len(), v2.len());
        let rail = m.pins.iter().find(|p| p.name.ends_with(":N")).unwrap().at;
        for r in v1.iter().chain(&v2) {
            assert!(r.x >= rail.x && r.x + r.w <= rail.x + rail.w && r.y >= rail.y && r.y + r.h <= rail.y + rail.h, "{r:?} outside rail {rail:?}");
            assert!(r.x >= 2000, "{r:?} pierces the plate stack");
        }
    }

    /// A deck with no cut size or spacing draws a sandwich with no cuts
    /// instead of looping forever on a zero pitch.
    #[test]
    fn a_zero_via_pitch_draws_no_cuts() {
        let pdk = Deck { metals: 3, vias: 2, rules: &[], sized: false };
        let (group, c) = one_cap(1, 2000);
        let m = Capacitor { units_x: 1, kind: Kind::VerticalAcrossLayers }.draw(&group, &c, &pdk);
        assert!(m.shapes.iter().all(|s| s.layer.0 < 100));
    }
}
