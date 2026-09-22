//! Capacitor generator. Ported from `backend/cells/src/generators/capacitor.rs`.

use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect};

use crate::Pattern;

use crate::builder::{layer, rule, sizing, Builder, Sizing};
use crate::Cell;

/// How a unit cell builds its capacitance — **the metal stack it occupies**, not
/// a cosmetic tag. Each kind draws genuinely different geometry:
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
/// **No LVS recognition marker.** The comb draws both electrodes as many
/// interdigitated polygons on one metal, and gdsverify's `DeviceRecognition`
/// binds exactly one polygon per terminal position (a surplus refuses the
/// marker), so the comb is not expressible in the current schema; a
/// plate-kinds-only recogniser would make the verdict depend on which variant
/// the placer picked. Capacitors therefore stay a reference-builder skip —
/// the precise gap and the schema extension it needs (per-terminal
/// merged-region binding) are documented in `backend/verify/src/reference.rs`.
///
/// **Inter-plate keepout.** For the two stacked kinds the dielectric between the
/// plates is load-bearing: a via cut or an intervening-metal shape inside the
/// plate footprint shorts the device. This generator honours that for its own
/// geometry (the `met_n+1` bus jumper of the sandwich runs in the bus column,
/// outside the plates). It cannot *enforce* it against the router — `Macro` is
/// shapes + pins + bbox with no blockage concept, and `dr` is not handed the
/// macros at all (see `backend/dr/src/lib.rs` § Contract note). Routing keepout
/// over cell interiors is router-obstacle plumbing, tracked in `TODO.md`.
///
/// Theory: AOAL ch7 (construction ranking), ch8 §8.3.2; `docs/cells/capacitor.md` §1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    VerticalAcrossLayers,
    HorizontalAcrossLayers,
    VerticalInOneLayer,
}

/// One point in the **capacitor variant space** (MOM/MIM array). Theory: AOAL
/// ch08; `docs/cells/capacitor.md`.
///
/// `units_x`/`units_y` tile the device's unit count into a grid and so pick its
/// **aspect ratio** — the axis the placer reshapes over. They do *not* draw as
/// separate unit cells: for a single device, N unit caps in parallel are
/// electrically one plate of N× the area, and drawing them separately would need
/// a bus network to reconnect what the array just split. Units become distinct
/// drawn cells only once they carry *different owners* — the common-centroid
/// assignment for ratioed arrays, which is open research (`TODO.md` § Capacitor).
#[derive(Clone)]
pub struct Capacitor {
    /// Columns of the unit-plate array (`self.columns` in the old spec).
    pub units_x: u16,
    /// Rows, derived from total units / columns.
    pub units_y: u16,
    pub guard_ring: bool,
    pub kind: Kind,
    pub pattern: Pattern,
}

/// Same budget the MOSFET uses — the reshape search must stay bounded.
const MAX_VARIANTS: usize = 16;

impl Cell for Capacitor {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let units = s.total_nf().max(1);
        let columns: Vec<u16> = (1..=units).filter(|c| units % c == 0).collect();
        let kinds = feasible_kinds(process);

        // Column-outer / kind-inner so the `MAX_VARIANTS` truncation below keeps a
        // spread of stacks rather than every column of the first kind.
        let mut specs: Vec<Self> = Vec::new();
        for &cols in &columns {
            for &kind in &kinds {
                specs.push(Capacitor {
                    units_x: cols,
                    units_y: units.div_ceil(cols),
                    guard_ring: false,
                    kind,
                    // ponytail: `draw` lays units out row-major and does not yet
                    // honour a common-centroid assignment, so enumerating `Cc1d`
                    // would offer the placer a variant that draws identically to
                    // `Single`. Unit-owner assignment is open research —
                    // `TODO.md` § Capacitor.
                    pattern: Pattern::Single,
                });
            }
        }

        // Dedup by (estimated footprint, kind) — the same key discipline as
        // `mosfet::enumerate`, so the placer never anneals over two variants it
        // cannot tell apart.
        let mut seen: Vec<((i32, i32), Kind)> = Vec::new();
        specs.retain(|spec| {
            let key = (spec.estimate(group, process), spec.kind);
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        specs.truncate(MAX_VARIANTS);
        specs
    }

    fn estimate(&self, group: &DeviceGroup, process: &dyn Process) -> (i32, i32) {
        // ponytail: the `Cell` trait hands `estimate` no `Constraints`, so this
        // sees the *default* sizing, not the group's unitization — the same hole
        // `mosfet::est_dims` has. Planning area is therefore the un-sized
        // footprint. Fixing it is a trait-signature change across all six
        // families; see the note in `TODO.md`.
        let s = group_sizing(group, &Constraints::default(), process);
        let g = Geom::new(self, &s, process);
        g.footprint(group.devices.len() as i32, &per_device_units(&s))
    }

    fn ports(&self, group: &DeviceGroup) -> Vec<Pin> {
        (0..group.devices.len())
            .flat_map(|i| [port(i, "TOP"), port(i, "BOT")])
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let g = Geom::new(self, &s, process);

        let mut x0 = 0;
        for (di, &n_units) in per_device_units(&s).iter().enumerate() {
            let plate = Rect { x: x0, y: 0, w: g.grid_w(n_units), h: g.grid_h(n_units) };
            let (bot_pin, top_pin) = match self.kind {
                Kind::VerticalInOneLayer => g.comb(&mut b, plate),
                Kind::HorizontalAcrossLayers => g.plates(&mut b, plate, None),
                Kind::VerticalAcrossLayers => g.plates(&mut b, plate, g.third_metal),
            };
            b.pin(pin_at(di, "BOT", bot_pin, g.bot_metal));
            b.pin(pin_at(di, "TOP", top_pin, g.top_metal));
            x0 += g.tile_w(n_units) + g.device_gap;
        }

        b.finish()
    }
}

/// Every dimension and layer the three kinds draw from, resolved once per call so
/// `draw` and `estimate` cannot drift apart.
struct Geom {
    bot_metal: LayerId,
    top_metal: LayerId,
    /// `met_n+2` — present only for the three-metal sandwich.
    third_metal: Option<LayerId>,
    /// The two cut layers strapping `met_n`↔`met_n+2` (`via_n`, `via_n+1`).
    cuts: [LayerId; 2],
    /// Drawn width of each cut — via layers carry exact (min = max) widths, so
    /// each cut is sized from its own layer's rule, not one shared `contact`.
    cut_w: [i32; 2],
    unit_w: i32,
    unit_h: i32,
    unit_gap: i32,
    m_space: i32,
    device_gap: i32,
    inset: i32,
    finger_w: i32,
    finger_space: i32,
    ct: i32,
    via_enc: i32,
    /// Vertical step between cut rows: the worst `size + spacing` over both cut
    /// layers, so neither layer's min-spacing is violated by the shared pitch.
    via_pitch: i32,
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
        let ct = rule(process, "contact", 170);
        let via_enc = rule(process, "via_enclosure", 300);
        let sandwich = spec.kind == Kind::VerticalAcrossLayers;
        let unit_gap = rule(process, "plate_spacing", 200);
        // Per-layer cut dimensions: via1/via2 carry exact widths and their own
        // spacings in the deck (`via{n}_min_width` / `via{n}_min_spacing`); a
        // shared `contact`-sized cut violated `via1_max_width`/`via2_min_width`.
        let via_spacing = rule(process, "via_spacing", 170);
        let cut_w = [rule(process, "via1_min_width", ct), rule(process, "via2_min_width", ct)];
        let via_pitch = (cut_w[0] + rule(process, "via1_min_spacing", via_spacing))
            .max(cut_w[1] + rule(process, "via2_min_spacing", via_spacing));
        // The strap column carries rails on every sandwich metal, so its gap to
        // the plates must satisfy the *tallest* metal's spacing, not met1's.
        let m_space = rule(process, "met1_space", 140)
            .max(rule(process, "met2_min_spacing", 0))
            .max(rule(process, "met3_min_spacing", 0));
        Self {
            bot_metal: met(0),
            // The comb keeps both electrodes on one metal — that *is* the kind.
            top_metal: if spec.kind == Kind::VerticalInOneLayer { met(0) } else { met(1) },
            third_metal: sandwich.then(|| met(2)),
            cuts: [v.first().copied().unwrap_or(LayerId(0)), v.get(1).copied().unwrap_or(LayerId(0))],
            cut_w,
            unit_w: s.unit_w,
            unit_h: s.unit_l,
            unit_gap,
            m_space,
            device_gap: rule(process, "device_gap", 600),
            inset: unit_gap,
            finger_w: rule(process, "mom_finger_width", 200),
            finger_space: rule(process, "mom_finger_space", 200),
            ct,
            via_enc,
            via_pitch,
            max_cols: i32::from(spec.units_x.max(1)),
            strap_w: if sandwich { cut_w[0].max(cut_w[1]) + 2 * via_enc } else { 0 },
        }
    }

    fn cols(&self, n_units: i32) -> i32 {
        self.max_cols.min(n_units).max(1)
    }

    fn rows(&self, n_units: i32) -> i32 {
        let cols = self.cols(n_units);
        (n_units + cols - 1) / cols
    }

    /// Plate width for `n_units` — the unit grid merged into one plate, so the
    /// `units_x × units_y` split only picks the aspect ratio.
    fn grid_w(&self, n_units: i32) -> i32 {
        self.cols(n_units) * (self.unit_w + self.unit_gap) - self.unit_gap
    }

    fn grid_h(&self, n_units: i32) -> i32 {
        self.rows(n_units) * (self.unit_h + self.unit_gap) - self.unit_gap
    }

    /// One device's full footprint: the plate plus, for the sandwich, its strap
    /// column.
    fn tile_w(&self, n_units: i32) -> i32 {
        self.grid_w(n_units) + if self.strap_w > 0 { self.m_space + self.strap_w } else { 0 }
    }

    /// Footprint of the whole group — must agree with what [`Capacitor::draw`]
    /// lays down, or the placer reserves the wrong area.
    fn footprint(&self, n_devices: i32, per_dev: &[i32]) -> (i32, i32) {
        let w: i32 = per_dev.iter().map(|&u| self.tile_w(u)).sum();
        let h = per_dev.iter().map(|&u| self.grid_h(u)).max().unwrap_or(0);
        (w + self.device_gap * (n_devices - 1).max(0), h)
    }

    /// Stacked plates: BOT fills `plate`, TOP is inset so the plate edges never
    /// align (fringe + misalignment tolerance). `third` adds a second BOT level
    /// above TOP, doubling the coupled area, strapped in a side column.
    /// **Nothing is drawn between the plates** — that gap is the dielectric.
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
        let max_cut = self.cut_w[0].max(self.cut_w[1]);
        let mut vy = plate.y + self.via_enc;
        while vy + max_cut + self.via_enc <= plate.y + plate.h {
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
    (1..=5).map_while(|n| layer(process, &format!("met{n}"))).collect()
}

/// Cut layers between consecutive metals (`via1` joins met1↔met2, …).
fn vias(process: &dyn Process) -> Vec<LayerId> {
    (1..=4).map_while(|n| layer(process, &format!("via{n}"))).collect()
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

fn pin_at(i: usize, term: &str, at: Rect, layer: LayerId) -> Pin {
    Pin { name: format!("d{i}:{term}"), net: net_of(i, term), at, layer }
}

fn net_of(i: usize, term: &str) -> NetId {
    let t = if term == "TOP" { 0 } else { 1 };
    NetId((i as u16).wrapping_mul(4).wrapping_add(t))
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default plate: a nominal 2µm square unit cell.
    let def = rule(process, "cap_unit_side", 2000);
    sizing(group, c, def, def)
}

/// Units contributed by each device — the group's `dev_nf`, min 1.
fn per_device_units(s: &Sizing) -> Vec<i32> {
    s.dev_nf.iter().map(|&n| i32::from(n.max(1))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
                same_variant_required: false,
                dummy_required: false,
                route_matching_required: false,
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
        let spec = Capacitor {
            units_x: 2,
            units_y: 2,
            guard_ring: false,
            kind: Kind::VerticalInOneLayer,
            pattern: Pattern::Single,
        };
        let m = spec.draw(&group, &c, &pdk);
        // Every rect is on one metal, so BOT and TOP are distinguished purely by
        // geometry: any contact at all merges the two electrodes.
        let bot_spine = m.pins.iter().find(|p| p.name.ends_with("BOT")).unwrap().at;
        let top_spine = m.pins.iter().find(|p| p.name.ends_with("TOP")).unwrap().at;
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

    /// The three kinds must occupy three different metal stacks; before this they
    /// collapsed to two identical shape sets plus a short.
    #[test]
    fn kinds_draw_distinct_stacks() {
        let pdk = TestPdk::full();
        let (group, c) = one_cap(1, 2000);
        let stack = |kind| {
            let spec = Capacitor {
                units_x: 1,
                units_y: 1,
                guard_ring: false,
                kind,
                pattern: Pattern::Single,
            };
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

    /// `estimate` feeds planning-area sizing before anything is drawn; if it
    /// disagrees with `draw` the placer reserves the wrong footprint. Compared
    /// against the *default* sizing because the trait denies `estimate` the
    /// `Constraints` — see the ponytail note on `Capacitor::estimate`.
    #[test]
    fn estimate_matches_drawn_bbox() {
        let pdk = TestPdk::full();
        let group = DeviceGroup { devices: vec![DeviceId(0)] };
        let c = Constraints::default();
        for kind in feasible_kinds(&pdk) {
            let spec = Capacitor {
                units_x: 2,
                units_y: 2,
                guard_ring: false,
                kind,
                pattern: Pattern::Single,
            };
            let (w, h) = spec.estimate(&group, &pdk);
            let bbox = spec.draw(&group, &c, &pdk).bbox;
            assert_eq!((w, h), (bbox.w, bbox.h), "{kind:?} estimate vs drawn bbox");
        }
    }
}
