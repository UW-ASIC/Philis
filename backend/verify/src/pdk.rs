//! [`Pdk`] — process data every crate reads: layers, rule values, grid, the
//! compiled gdsverify deck and its source text.
//!
//! Units: the deck is parsed against [`nm_grid`], so 1 dbu = 1 nm and any
//! `Dbu::raw()` in this crate is nanometres. `Pdk::grid` is the manufacturing
//! grid (sky130: 5 nm), from the deck's `off_grid` rule.

use gdsverify::ingest::deck::{Deck, ParamValue, RuleSpec};
use gdsverify::ingest::StrTable;
use gdsverify::geom::Grid;
use pnr_core::{LayerId, Process};

/// The database grid every length in this crate is expressed against:
/// 1000 dbu/µm, so **1 dbu = 1 nm**.
#[must_use]
pub fn nm_grid() -> Grid {
    Grid::new(1000).expect("1000 dbu/um is a valid grid resolution")
}

/// gdsverify's layer id type (`u16` newtype).
pub use gdsverify::geom::LayerId as GvLayerId;

/// One layer's DC electromigration limits at the deck's rating temperature.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EmLimit {
    /// Current per µm of width, µA/µm (`max_density`); `0` = none.
    pub ua_per_um: f32,
    /// Current per cut, µA (`max_current_per_cut`); `0` = none.
    pub ua_per_cut: f32,
    /// Blech product, (µA/µm)·µm (`blech_limit`); `0` = none.
    pub blech: f32,
    /// `(T_ref K, Ea eV, n)` when the deck gives the rating temperature and
    /// Black's-law parameters.
    pub derating: Option<(f32, f32, f32)>,
}

/// Process design kit: layers, design-rule values, grid, and the gdsverify deck.
pub struct Pdk {
    /// Layer name → [`LayerId`]; row `i` is `LayerId(i)` is deck layer `i`
    /// (checked at load), so a Philis layer id feeds the engine unchanged.
    pub layers: Vec<(String, LayerId)>,
    /// Design-rule values by name, nm. Later entries override earlier ones.
    pub rules: Vec<(String, i32)>,
    /// Manufacturing grid, nm.
    pub grid: i32,
    pub deck: Deck,
    /// The table `deck`'s `StrId`s resolve against.
    pub strings: StrTable,
    /// The deck text (GPurify's deck language); each [`crate::Checker`]
    /// re-parses it into its own table.
    pub source: String,
    /// The Philis sidecar's `cell` section (roles, generator dimensions,
    /// process coefficients): what the checker's deck does not carry.
    pub cell: serde_json::Value,
    /// Layer role → deck layer name, from `cell.layers` (gdsverify ignores
    /// the `cell` section).
    pub roles: Vec<(String, String)>,
    /// The routable metal stack, bottom-up.
    pub routing_metals: Vec<LayerId>,
    /// The cuts joining consecutive `routing_metals` (one fewer).
    pub routing_cuts: Vec<LayerId>,
}

impl Pdk {
    /// Load a Philis PDK sidecar: `{"deck": <name>, "cell": {…}}`. The deck is
    /// GPurify's deck language (one source of truth for every rule, layer,
    /// device and PEX value), resolved by [`Pdk::deck_text`]. The sidecar adds
    /// only what the checker does not read: layer roles, generator
    /// dimensions, process coefficients (registry: [`crate::sidecar`]).
    ///
    /// # Errors
    /// An unreadable sidecar or deck, or anything [`Pdk::load`] rejects.
    pub fn from_json(sidecar: &str) -> Result<Self, String> {
        Self::load(&Self::deck_text(sidecar)?, sidecar)
    }

    /// A PDK compiled into the binary (`sky130`, `gf180mcu`, `ihp_sg13g2`,
    /// `generic_finfet`): its sidecar and vendored deck, no filesystem read.
    ///
    /// # Errors
    /// An unknown name, or anything [`Pdk::load`] rejects.
    pub fn builtin(name: &str) -> Result<Self, String> {
        let (_, sidecar) = crate::decks::SIDECARS.iter().find(|(n, _)| *n == name).ok_or_else(|| {
            let names: Vec<_> = crate::decks::SIDECARS.iter().map(|(n, _)| *n).collect();
            format!("no built-in PDK {name:?} (built in: {})", names.join(", "))
        })?;
        Self::from_json(sidecar)
    }

    /// The deck text a sidecar names: a vendored deck (`pdks/decks/`,
    /// compiled in) by file name first, then an absolute path, then a file of
    /// GPurify's `pdks/` found at build time (build.rs) for a deck not vendored.
    ///
    /// # Errors
    /// An unreadable sidecar or deck file.
    pub fn deck_text(sidecar: &str) -> Result<String, String> {
        let v: serde_json::Value =
            serde_json::from_str(sidecar).map_err(|e| format!("sidecar is not valid JSON: {e}"))?;
        let name = v.get("deck").and_then(serde_json::Value::as_str).ok_or("sidecar names no `deck` file")?;
        if let Some((_, text)) = crate::decks::DECKS.iter().find(|(n, _)| *n == name) {
            return Ok((*text).to_string());
        }
        let path = std::path::Path::new(name);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            let pdks = option_env!("GPURIFY_PDKS")
                .ok_or_else(|| format!("deck {name:?} is not vendored and GPurify's pdks/ was not found at build time; set GPURIFY_DIR"))?;
            std::path::Path::new(pdks).join(name)
        };
        std::fs::read_to_string(&path).map_err(|e| format!("deck {}: {e}", path.display()))
    }

    /// Parse `deck` (GPurify deck text) with the sidecar JSON `sidecar`, and
    /// validate that the pair is complete, so downstream code may assume every
    /// value it reads is real process data.
    ///
    /// # Errors
    /// gdsverify's parse error, or a list of every way the deck is incomplete.
    pub fn load(text: &str, sidecar: &str) -> Result<Self, String> {
        let mut strings = StrTable::default();
        let deck = gdsverify::ingest::deck::parse_deck(text, nm_grid(), &mut strings)
            .map_err(|e| format!("deck rejected: {e}"))?;
        let roles = parse_roles(sidecar)?;
        let cell = serde_json::from_str::<serde_json::Value>(sidecar)
            .ok()
            .and_then(|v| v.get("cell").cloned())
            .unwrap_or_default();

        let layers: Vec<(String, LayerId)> = (0..deck.layers.len())
            .map(|id| {
                let gv = GvLayerId(id as u16);
                (strings.resolve(deck.layers.name(gv)).to_string(), LayerId(id as u16))
            })
            .collect();
        let find = |name: &str| {
            layers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, id)| *id)
                .ok_or_else(|| format!("cell.layers names layer {name:?}, absent from the deck"))
        };
        let routing_metals = roles
            .routing_metals
            .iter()
            .map(|n| find(n))
            .collect::<Result<Vec<_>, _>>()?;
        let routing_cuts = roles
            .routing_vias
            .iter()
            .map(|n| find(n))
            .collect::<Result<Vec<_>, _>>()?;

        // Narrowest source last so it wins: deck rule ids, then synthesised
        // `<layer>_min_width`/`_min_spacing`, then explicit `cell.*` values.
        let mut rules = scalar_rules(&deck, &strings);
        let grid = deck_grid(&deck, &strings);

        let mut pdk = Self {
            layers,
            rules: Vec::new(),
            grid,
            deck,
            strings,
            source: text.to_string(),
            cell,
            roles: roles.map,
            routing_metals,
            routing_cuts,
        };
        for (name, id) in pdk.layers.clone() {
            if let Some(w) = pdk.min_width(id.0) {
                rules.push((format!("{name}_min_width"), w));
            }
            if let Some(s) = pdk.min_spacing(id.0) {
                rules.push((format!("{name}_min_spacing"), s));
            }
        }
        // The same values under role names (`li_min_spacing` on a deck whose li
        // role is `metal1`), so generators asking by role get this deck's number.
        for (role, name) in pdk.roles.clone() {
            if let Some(&(_, id)) = pdk.layers.iter().find(|(n, _)| *n == name) {
                if let Some(w) = pdk.min_width(id.0) {
                    rules.push((format!("{role}_min_width"), w));
                }
                if let Some(s) = pdk.min_spacing(id.0) {
                    rules.push((format!("{role}_min_spacing"), s));
                }
                // The side of the minimum-area square.
                if let Some(a) = pdk.min_area(id.0) {
                    rules.push((format!("{role}_min_area"), (a as f64).sqrt().ceil() as i32));
                }
            }
        }
        rules.extend(roles.scalars);
        pdk.rules = rules;
        pdk.validate()?;
        Ok(pdk)
    }

    /// Every way the deck is too incomplete to build legal geometry from,
    /// reported at once. Each absence used to produce silent wrong geometry.
    fn validate(&self) -> Result<(), String> {
        let mut bad = Vec::new();
        let named = |id: LayerId| {
            self.layers
                .iter()
                .find(|(_, l)| *l == id)
                .map_or_else(|| format!("{id:?}"), |(n, _)| n.clone())
        };

        for (role, layer) in &self.roles {
            if !self.layers.iter().any(|(n, _)| n == layer) {
                bad.push(format!(
                    "role {role:?} maps to layer {layer:?}, which the deck does not define"
                ));
            }
        }

        // The identity map `gv_layer` relies on: row i of `layers` is LayerId(i)
        // is deck layer i. Built that way above, but a future constructor could
        // break it silently, so it is validated rather than assumed.
        for (i, (name, id)) in self.layers.iter().enumerate() {
            let deck_says = self
                .deck
                .layers
                .id(&self.strings, name)
                .map_or(u16::MAX, |gv| gv.0);
            if id.0 as usize != i || deck_says != id.0 {
                bad.push(format!(
                    "layer {name:?}: Philis LayerId {} / table row {i} / deck id {deck_says} \
                     disagree — gv_layer's identity map is broken",
                    id.0
                ));
            }
        }

        // Roles no cell generator can draw a correct device without. Marker layers
        // for optional device families (`npn`/`pnp`/`hvtp`/`lvtn`) are deliberately
        // absent: a deck with no BJTs legitimately has no BJT marker, and those
        // sites still use `layer()` + `if let Some(..)`.
        for role in REQUIRED_ROLES {
            if <Self as Process>::layer(self, role).is_none() {
                bad.push(format!(
                    "no layer for mandatory role {role:?}: cells cannot be drawn against \
                     this deck (add it to cell.layers)"
                ));
            }
        }
        // Unknown, mistyped, missing-required and unsourced `cell.*` keys.
        bad.extend(crate::sidecar::validate(&self.cell));

        if self.routing_metals.is_empty() {
            bad.push("cell.layers.routing_metals is empty: nowhere legal to route".into());
        }
        if self.routing_cuts.len() + 1 != self.routing_metals.len() {
            bad.push(format!(
                "{} routing metals need {} cuts to join them, cell.layers.routing_vias gives {}",
                self.routing_metals.len(),
                self.routing_metals.len().saturating_sub(1),
                self.routing_cuts.len()
            ));
        }

        // A declared cut must actually join the pair it sits between, per the
        // deck's own connectivity — otherwise the stack is not electrically
        // continuous and cut-required connectivity leaves isolated islands.
        let conn = &self.deck.connectivity;
        for (i, cut) in self.routing_cuts.iter().enumerate() {
            let (Some(a), Some(b)) = (self.routing_metals.get(i), self.routing_metals.get(i + 1))
            else {
                continue;
            };
            // A deck joins extracted conductors through extracted cuts, each
            // the drawn layer or computed from it (`li_c = li not li_rs`,
            // `via3_m3 = via3 not capm`).
            let is = |x: GvLayerId, drawn: &LayerId| self.reaches(x, drawn.0);
            let joins = conn.via_cut.iter().zip(&conn.via_connects).any(|(&c, &(x, y))| {
                is(c, cut) && ((is(x, a) && is(y, b)) || (is(x, b) && is(y, a)))
            });
            if !joins {
                bad.push(format!(
                    "cut {} is declared between {} and {}, but connectivity.vias does not join them",
                    named(*cut),
                    named(*a),
                    named(*b)
                ));
            }
            if self.routing_metals.contains(cut) {
                bad.push(format!("{} is both routing metal and a via cut", named(*cut)));
            }
        }

        // Geometry on a layer needs that layer's own numbers; a default would be
        // a guess about a physical process.
        for l in &self.routing_metals {
            for (what, present) in [
                ("min_width", self.min_width(l.0).is_some()),
                ("min_spacing", self.min_spacing(l.0).is_some()),
            ] {
                if !present {
                    bad.push(format!("routing layer {} has no {what} rule", named(*l)));
                }
            }
        }
        for c in &self.routing_cuts {
            if self.min_width(c.0).is_none() {
                bad.push(format!(
                    "cut layer {} has no min_width, so its drawn size is unknown",
                    named(*c)
                ));
            }
        }

        if bad.is_empty() {
            Ok(())
        } else {
            Err(format!("PDK is incomplete:\n  - {}", bad.join("\n  - ")))
        }
    }

    /// gdsverify layer id for a Philis [`LayerId`] (the identity map).
    #[must_use]
    pub fn gv_layer(&self, layer: LayerId) -> GvLayerId {
        GvLayerId(layer.0)
    }

    /// gdsverify layer id for a named layer, or `None` if the deck lacks it.
    #[must_use]
    pub fn gv_layer_by_name(&self, name: &str) -> Option<GvLayerId> {
        self.deck.layers.id(&self.strings, name)
    }

    /// GDS `(layer, datatype)` that names the net drawn on `drawn` in a
    /// stream other tools read: the deck's dedicated text layer for that
    /// conductor (sky130 `met1_label` 68/5, gf180 `metal1_label`), never the
    /// drawing layer itself, which `connect label met1 names met1` also allows
    /// but magic and klayout ignore for text. `None` when no label names it.
    #[must_use]
    pub fn label_gds(&self, drawn: u16) -> Option<(u16, u16)> {
        let c = &self.deck.connectivity;
        let d = GvLayerId(drawn);
        (0..c.label_layer.len())
            .filter(|&r| c.label_names[r] == d || self.deck.layers.operands(c.label_names[r]).contains(&d))
            .map(|r| c.label_layer[r])
            .find(|&t| t != d)
            .map(|t| self.deck.layers.stream_of(t))
    }

    /// `reference` as one SPICE `.subckt top <ports>`: the netlist signoff's
    /// LVS compares the drawing against, dummies and one card per drawn
    /// finger included, so an external LVS (netgen) can check the same claim.
    /// `ports` is the cell's interface (empty: every port signoff labelled).
    /// Lengths are unitless µm, the convention magic extracts and sky130
    /// schematics use; a device with no model (a dummy) takes the deck's first
    /// recogniser of its kind and polarity.
    #[must_use]
    pub fn reference_spice(&self, reference: &crate::reference::RefInput, top: &str, ports: &[String]) -> String {
        use crate::reference::RefKind;
        use gdsverify::ingest::deck::DeviceKind;
        let d = &self.deck.devices;
        let first = |kind: DeviceKind, p: Option<bool>| {
            (0..d.kind.len())
                .find(|&r| d.kind[r] == kind && p.is_none_or(|p| self.strings.resolve(self.deck.layers.name(d.marker[r])).starts_with('p') == p))
                .map_or("?", |r| self.strings.resolve(d.model[r]))
        };
        let ports = if ports.is_empty() { &reference.ports } else { ports };
        let mut sp = format!(".subckt {top} {}\n", ports.join(" "));
        for (i, dev) in reference.devices.iter().enumerate() {
            let default = match dev.kind {
                RefKind::Nmos => first(DeviceKind::Mos, Some(false)),
                RefKind::Pmos => first(DeviceKind::Mos, Some(true)),
                RefKind::Npn | RefKind::Pnp => first(DeviceKind::Bjt, None),
                RefKind::Resistor => first(DeviceKind::Resistor, None),
                RefKind::Capacitor => first(DeviceKind::Capacitor, None),
                RefKind::Diode => first(DeviceKind::Diode, None),
                // No deck recognises an inductor: listed, commented out.
                RefKind::Inductor => "?",
            };
            let model = dev.model.as_deref().map_or_else(|| default.to_string(), |m| self.deck_model(m).unwrap_or_else(|| m.to_string()));
            let comment = if model == "?" { "* unrecognised: " } else { "" };
            sp.push_str(&format!("{comment}X{i} {} {model}", dev.terminals.join(" ")));
            for (name, v) in &dev.params {
                // SI metres → µm; other params (m, nf, counts) as given.
                if matches!(name.as_str(), "w" | "l") {
                    // Rounded to the nm the geometry is drawn on: no float noise.
                    sp.push_str(&format!(" {name}={}", (v * 1e9).round() / 1e3));
                } else {
                    sp.push_str(&format!(" {name}={v}"));
                }
            }
            sp.push('\n');
        }
        sp.push_str(".ends\n");
        sp
    }

    /// `table[LayerId.0] = (gds_layer, gds_datatype)` for the GDS writer;
    /// `(0, 0)` for a layer the deck computes rather than draws.
    #[must_use]
    pub fn layer_gds(&self) -> Vec<(u16, u16)> {
        let max_id = self.layers.iter().map(|(_, id)| id.0 as usize).max().unwrap_or(0);
        let mut table = vec![(0u16, 0u16); max_id + 1];

        for (_, id) in &self.layers {
            let gv = GvLayerId(id.0);
            if !self.deck.layers.is_derived(gv) {
                table[id.0 as usize] = self.deck.layers.stream_of(gv);
            }
        }
        table
    }

    /// The routable stack, bottom-up (`cell.layers.routing_metals`); `gr`/`dr`
    /// index it by their internal layer index.
    #[must_use]
    pub fn routing_layers(&self) -> Vec<LayerId> {
        self.routing_metals.clone()
    }

    /// `(cut layer, cut size, pad below, pad above)` joining
    /// `routing_layers()[i]` to `[i + 1]`, nm. Pads clear their metal's
    /// `min_width`/`min_area` and sit on the manufacturing grid.
    #[must_use]
    pub fn routing_vias(&self) -> Vec<(LayerId, i32, i32, i32)> {
        let stack = self.routing_layers();
        let mut cuts = Vec::with_capacity(stack.len().saturating_sub(1));
        for (i, pair) in stack.windows(2).enumerate() {
            let (a, b) = (pair[0].0, pair[1].0);
            let cut = self.routing_cuts[i].0;
            let size = self
                .min_width(cut)
                .expect("validate() guarantees every cut layer has a min_width");
            // A centred square's sides are equal, so it clears an asymmetric
            // `min_one_side` only by granting it on every side.
            let enclose = self
                .max_enclosure_of(cut, "min_enclosure", "limit")
                .max(self.max_enclosure_of(cut, "asymmetric_enclosure", "min_one_side"));
            let enclosed = size + 2 * enclose;
            cuts.push((
                LayerId(cut),
                size,
                self.pad_for(a, enclosed),
                self.pad_for(b, enclosed),
            ));
        }
        cuts
    }

    /// Largest length `param` of any `kind` rule whose inner layer (index 1 of
    /// `[outer, inner]`) is `inner`; `0` when none. The binding enclosure,
    /// since the same pad is drawn above and below the cut.
    fn max_enclosure_of(&self, inner: u16, kind: &str, param: &str) -> i32 {
        let (Some(kind), Some(param)) = (self.strings.get(kind), self.strings.get(param)) else {
            return 0;
        };
        self.deck
            .rules
            .spec
            .iter()
            .filter(|s| s.kind == kind)
            .filter(|s| self.deck.rules.layers_of(s).get(1).map(|l| l.0) == Some(inner))
            .filter_map(|s| match self.deck.rules.param(s, param) {
                Some(ParamValue::Length(d)) => Some(d.raw() as i32),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// Smallest square enclosing the cut that is also legal alone on `metal`
    /// (`min_width`, `min_area`), rounded up to the manufacturing grid.
    fn pad_for(&self, metal: u16, enclosed: i32) -> i32 {
        let side_for_area = self.min_area(metal).map_or(0, |a| {
            let mut s = (a as f64).sqrt() as i32;
            while i64::from(s) * i64::from(s) < a {
                s += 1;
            }
            s
        });
        let side = enclosed
            .max(self.min_width(metal).unwrap_or(0))
            .max(side_for_area);
        let g = self.grid.max(1);
        side.div_euclid(g) * g + if side.rem_euclid(g) == 0 { 0 } else { g }
    }

    /// The deck's `min_area` for a layer in nm².
    #[must_use]
    pub fn min_area(&self, layer: u16) -> Option<i64> {
        let (kind, limit) = (self.strings.get("min_area")?, self.strings.get("limit")?);
        self.deck.rules.spec.iter().find_map(|s| {
            (s.kind == kind && self.deck.rules.layers_of(s).first().map(|l| l.0) == Some(layer)).then_some(())?;
            match self.deck.rules.param(s, limit) {
                Some(ParamValue::Area(a)) => Some(a.raw() as i64),
                // A side length (an older deck's spelling).
                Some(ParamValue::Length(d)) => Some(d.raw() * d.raw()),
                _ => None,
            }
        })
    }

    /// `wire_width + max(min_spacing)` over `stack`: the one lattice pitch
    /// legal on every layer actually routed on.
    ///
    /// ponytail: one pitch for the whole stack, so the upper layers are routed
    /// more coarsely than they need. Per-layer pitch means a per-layer track
    /// lattice in `gr::TrackGrid`; do that if upper-layer density ever matters.
    #[must_use]
    pub fn routing_pitch(&self, wire_width: i32, stack: &[u16]) -> i32 {
        let worst = stack.iter().filter_map(|&l| self.route_spacing(l)).max().unwrap_or(0);
        wire_width + worst
    }

    /// Spacing a routed wire keeps on `layer`, nm: its `min_spacing`, and its
    /// end-of-line spacing (a wire end is a line end; the lattice cannot
    /// tell ends from sides, so every track keeps the larger).
    #[must_use]
    pub fn route_spacing(&self, layer: u16) -> Option<i32> {
        let eol = self.widest_on("eol_spacing", "limit", &[layer]).map(|v| v as i32);
        self.min_spacing(layer).max(eol)
    }

    /// `(width threshold, spacing)` of every `wide_dependent_spacing` rule on
    /// `layer`, nm, ascending by threshold: a shape at least that wide needs
    /// that spacing (the binding one is the largest threshold it reaches).
    #[must_use]
    pub fn wide_spacing(&self, layer: u16) -> Vec<(i32, i32)> {
        let len = |s, p| match self.deck.rules.param(s, p) {
            Some(ParamValue::Length(d)) => Some(d.raw() as i32),
            _ => None,
        };
        let mut steps: Vec<(i32, i32)> = match (
            self.strings.get("wide_dependent_spacing"),
            self.strings.get("width_threshold"),
            self.strings.get("limit"),
        ) {
            (Some(kind), Some(thr), Some(lim)) => self
                .deck
                .rules
                .spec
                .iter()
                .filter(|s| s.kind == kind && self.deck.rules.layers_of(s).first().map(|l| l.0) == Some(layer))
                .filter_map(|s| Some((len(s, thr)?, len(s, lim)?)))
                .collect(),
            _ => Vec::new(),
        };
        // A spacing table (LEF SPACINGTABLE: a row per width threshold, a
        // column per parallel run length): each row's widest entry, any run.
        if let (Some(table), Some(w), Some(sp), Some(prl)) =
            (self.strings.get("spacing_table"), self.strings.get("width"), self.strings.get("space"), self.strings.get("prl"))
        {
            for s in self.deck.rules.spec.iter().filter(|s| s.kind == table && self.deck.rules.layers_of(s).first().map(|l| l.0) == Some(layer)) {
                let ps = self.deck.rules.params_of(s);
                let of = |name| ps.iter().filter(move |(n, _)| *n == name).filter_map(|(_, v)| match v {
                    ParamValue::Length(d) => Some(d.raw() as i32),
                    _ => None,
                });
                let (widths, spaces): (Vec<i32>, Vec<i32>) = (of(w).collect(), of(sp).collect());
                let cols = ps.iter().filter(|(n, _)| *n == prl).count().max(1);
                for (i, &wi) in widths.iter().enumerate() {
                    if let Some(m) = spaces.get(i * cols..(i + 1) * cols).and_then(|r| r.iter().copied().max()) {
                        steps.push((wi, m));
                    }
                }
            }
        }
        steps.sort_unstable();
        steps
    }

    /// `(cut count, spacing)` of the deck's `via_array_spacing` rule on cut
    /// `layer`: an array of at least that many cuts needs that spacing.
    #[must_use]
    pub fn via_array_spacing(&self, layer: u16) -> Option<(i32, i32)> {
        let (kind, thr, lim) = (
            self.strings.get("via_array_spacing")?,
            self.strings.get("array_threshold")?,
            self.strings.get("limit")?,
        );
        self.deck
            .rules
            .spec
            .iter()
            .filter(|s| s.kind == kind && self.deck.rules.layers_of(s).first().map(|l| l.0) == Some(layer))
            .find_map(|s| match (self.deck.rules.param(s, thr), self.deck.rules.param(s, lim)) {
                (Some(ParamValue::Count(n)), Some(ParamValue::Length(d))) => Some((n as i32, d.raw() as i32)),
                _ => None,
            })
    }

    /// A non-length number from the deck's `cell` section (e.g. a mismatch
    /// coefficient); `None` when absent or not a number.
    #[must_use]
    pub fn cell_f32(&self, key: &str) -> Option<f32> {
        self.cell.get(key)?.as_f64().map(|x| x as f32)
    }

    /// Where a `cell.*` value comes from (`<key>_source`); `None` when the
    /// sidecar gives no source text.
    #[must_use]
    pub fn provenance(&self, key: &str) -> Option<&str> {
        crate::sidecar::source(self.cell.as_object()?, key)
    }

    /// Process-data keys this sidecar states on an `UNVERIFIED` source and
    /// some code reads (registry `reader` not `unread`): used, and reported
    /// as assumed rather than known. An unread key is assumed by nothing.
    #[must_use]
    pub fn unverified(&self) -> Vec<&str> {
        crate::sidecar::KEYS
            .iter()
            .filter(|k| k.sourced && k.reader != "unread" && self.provenance(k.name).is_some_and(|s| s.starts_with("UNVERIFIED")))
            .map(|k| k.name)
            .collect()
    }

    /// DC electromigration limits of `layer` from the deck's own EM rules:
    /// an `electromigration` rule (a deck that splits DC from peak) before an
    /// `em_current_density` one. A cut no rule names takes the tighter
    /// `max_current_per_cut` of the two routing metals it joins. `None` when
    /// no rule limits the layer.
    #[must_use]
    pub fn em_limit(&self, layer: LayerId) -> Option<EmLimit> {
        let ratio = |s: &RuleSpec, key: &str| match self.deck.rules.param(s, self.strings.get(key)?) {
            Some(ParamValue::Ratio(r)) if r > 0.0 => Some(r as f32),
            _ => None,
        };
        let rule_on = |l: LayerId| {
            ["electromigration", "em_current_density"].iter().find_map(|kind| {
                let kind = self.strings.get(kind)?;
                // A rule on the layer, or on the conductor extracted from it
                // (gf180 `metal1_con`).
                self.deck.rules.spec.iter().find(|s| s.kind == kind && self.deck.rules.layers_of(s).iter().any(|&g| self.reaches(g, l.0)))
            })
        };
        let of = |s: &RuleSpec| EmLimit {
            ua_per_um: ratio(s, "max_density").unwrap_or(0.0),
            ua_per_cut: ratio(s, "max_current_per_cut").unwrap_or(0.0),
            blech: ratio(s, "blech_limit").unwrap_or(0.0),
            derating: (|| Some((ratio(s, "reference_temperature")?, ratio(s, "activation_energy_ev")?, ratio(s, "current_exponent")?)))(),
        };
        // A cut has a per-cut limit only: a rule pairing it with metals states
        // their per-µm density too, which is not the cut's.
        let is_cut = self.routing_cuts.contains(&layer);
        if let Some(s) = rule_on(layer) {
            let e = of(s);
            return Some(if is_cut { EmLimit { ua_per_um: 0.0, blech: 0.0, ..e } } else { e });
        }
        // A cut: the tighter per-cut limit of the metals it joins.
        let i = self.routing_cuts.iter().position(|&c| c == layer)?;
        [self.routing_metals.get(i)?, self.routing_metals.get(i + 1)?]
            .into_iter()
            .filter_map(|&m| rule_on(m).map(of))
            .filter(|e| e.ua_per_cut > 0.0)
            .reduce(|a, b| if a.ua_per_cut <= b.ua_per_cut { a } else { b })
            .map(|e| EmLimit { ua_per_um: 0.0, blech: 0.0, ..e })
    }

    /// The deck's point-to-point resistance limit, Ω (`p2p_resistance`):
    /// the most any conductor path may drop end to end. `None` = unchecked.
    #[must_use]
    pub fn p2p_max_ohm(&self) -> Option<f32> {
        let (kind, key) = (self.strings.get("p2p_resistance")?, self.strings.get("max_resistance")?);
        self.deck.rules.spec.iter().filter(|s| s.kind == kind).find_map(|s| match self.deck.rules.param(s, key) {
            Some(ParamValue::Ratio(r)) if r > 0.0 => Some(r as f32),
            _ => None,
        })
    }

    /// The tightest antenna ratio of any routing metal's etch stage
    /// ([`Pdk::antenna_rule`]); `None` when no routed metal has one.
    #[must_use]
    pub fn antenna_max_ratio(&self) -> Option<f32> {
        self.routing_metals.iter().filter_map(|&l| self.antenna_rule(l)).map(|r| r.0).reduce(f32::min)
    }

    /// The antenna rule of `layer`'s etch stage (the rule's last layer):
    /// `(max ratio, sidewall thickness nm or 0 for areal, cumulative)`. An areal
    /// rule wins over a sidewall one on the same layer; `cumulative` when the
    /// rule is named so (`antenna_cumulative_*`: layers summed up to the stage).
    /// `None` when the deck states no antenna rule for the stage. `antenna`
    /// and `antenna_electrical` rows alike: the latter is the same per-stage
    /// ratio (areal) with a diode credit ([`Pdk::antenna_diode_credit`]).
    #[must_use]
    pub fn antenna_rule(&self, layer: LayerId) -> Option<(f32, f32, bool)> {
        let kinds = [self.strings.get("antenna"), self.strings.get("antenna_electrical")];
        let ratio = self.strings.get("max_ratio")?;
        let side = self.strings.get("sidewall_thickness");
        self.deck
            .rules
            .spec
            .iter()
            .filter(|s| kinds.contains(&Some(s.kind)) && self.deck.rules.layers_of(s).last().is_some_and(|&l| self.reaches(l, layer.0)))
            .filter_map(|s| {
                let Some(ParamValue::Ratio(r)) = self.deck.rules.param(s, ratio) else { return None };
                let t = match side.and_then(|k| self.deck.rules.param(s, k)) {
                    Some(ParamValue::Length(d)) => d.raw() as f32,
                    _ => 0.0,
                };
                // Cumulative: the rule states several routing metals, summed
                // up to this stage (ihp `antenna(gate, metal1, metal2)`).
                let metals = self.deck.rules.layers_of(s).iter().skip(1).filter(|&&l| self.routing_metals.iter().any(|m| self.reaches(l, m.0))).count();
                Some((r as f32, t, metals > 1))
            })
            .min_by(|a, b| (a.1 > 0.0).cmp(&(b.1 > 0.0)))
    }

    /// The diode layer and `diode_bonus` of an `antenna_electrical` rule with
    /// `diode_credit == 0`: the only diode credit signoff grants (GPurify
    /// refuses a row whose verdict hangs on `credit · diode area`, its unit
    /// unstated). `None` when the deck credits no diode — a drawn diode then
    /// fixes nothing at signoff.
    #[must_use]
    pub fn antenna_diode_credit(&self) -> Option<(LayerId, f32)> {
        let r = &self.deck.rules;
        let kind = self.strings.get("antenna_electrical")?;
        let (diode, credit, bonus) = (self.strings.get("diode_layer")?, self.strings.get("diode_credit")?, self.strings.get("diode_bonus")?);
        r.spec.iter().filter(|s| s.kind == kind).find_map(|s| match (r.param(s, diode)?, r.param(s, credit)?, r.param(s, bonus)?) {
            (ParamValue::Layer(l), ParamValue::Ratio(0.0), ParamValue::Ratio(b)) => Some((LayerId(l.0), b as f32)),
            _ => None,
        })
    }

    /// Every `density` and `density_cmp` rule: `(layer, window nm, limit
    /// fraction, is a maximum)`, one row per bound a `density_cmp` states
    /// (`min_density`, `max_density`; its window is `window_x`); a rule
    /// missing its window or limit is left out.
    #[must_use]
    pub fn density_rules(&self) -> Vec<(LayerId, i64, f64, bool)> {
        let s = |k: &str| self.strings.get(k);
        let (r, mut out) = (&self.deck.rules, Vec::new());
        let ratio = |spec, key: Option<_>| match key.and_then(|k| r.param(spec, k)) {
            Some(ParamValue::Ratio(l)) => Some(l),
            _ => None,
        };
        for spec in &r.spec {
            let Some(layer) = r.layers_of(spec).first().map(|l| LayerId(l.0)) else { continue };
            if Some(spec.kind) == s("density") {
                let Some(ParamValue::Length(w)) = s("window").and_then(|k| r.param(spec, k)) else { continue };
                let max = matches!(s("maximum").and_then(|m| r.param(spec, m)), Some(ParamValue::Flag(true)));
                out.extend(ratio(spec, s("limit")).map(|l| (layer, w.raw(), l, max)));
            } else if Some(spec.kind) == s("density_cmp") {
                let Some(ParamValue::Length(w)) = s("window_x").and_then(|k| r.param(spec, k)) else { continue };
                out.extend(ratio(spec, s("min_density")).map(|l| (layer, w.raw(), l, false)));
                out.extend(ratio(spec, s("max_density")).map(|l| (layer, w.raw(), l, true)));
            }
        }
        out
    }

    /// The marker layer of the deck's diode recogniser, when LVS can extract
    /// a diode here — every terminal layer a conductor, so both of its nets
    /// resolve. `None` otherwise (gf180's `pn_3p3` names `nwell`, not one).
    #[must_use]
    pub fn diode_marker(&self) -> Option<LayerId> {
        use gdsverify::ingest::deck::DeviceKind;
        // The drawn marker (`diom` role), where a diode recogniser is built
        // from it and binds extractable terminals (conductors, or a global
        // net such as sky130's substrate).
        let drawn = pnr_core::Process::layer(self, "diom")?;
        let c = &self.deck.connectivity;
        let d = &self.deck.devices;
        (0..d.kind.len()).filter(|&r| d.kind[r] == DeviceKind::Diode).find_map(|r| {
            let terms = &d.terminal[d.terminal_start[r] as usize..d.terminal_start[r + 1] as usize];
            let ok = terms.iter().all(|t| c.conductors.contains(t) || c.global.contains(t));
            (ok && self.reaches(d.marker[r], drawn.0)).then_some(drawn)
        })
    }

    /// Deck layer `i` has a `pex` row (a thickness or a dielectric).
    fn pex_declared(&self, i: usize) -> bool {
        let st = &self.deck.stack;
        st.thickness_nm.get(i).is_some_and(|&t| t > 0.0) || st.dielectric_k.get(i).is_some_and(|&k| k > 0.0)
    }

    /// The deck's `pex` row for `layer`: its own, else that of the one layer
    /// with a row that is a part of it ([`Pdk::reaches`]: the deck states PEX
    /// per extracted conductor). A declared conductor beats a cut or device
    /// body carved from it (sky130 `poly` answers with `poly_c`, not
    /// `licon_po` or `rbody_po`). Several candidates left → `None`, never an
    /// arbitrary pick: sky130 `licon` (five cut rows, 152–585 Ω) and `poly_rs`
    /// (only resistor bodies keep its area; `poly_c` is poly *not* poly_rs).
    fn pex_row(&self, layer: LayerId) -> Option<usize> {
        if self.pex_declared(layer.0 as usize) {
            return Some(layer.0 as usize);
        }
        let cands: Vec<GvLayerId> = (0..self.deck.layers.len())
            .map(|i| GvLayerId(i as u16))
            .filter(|&d| d.0 != layer.0 && self.pex_declared(d.0 as usize) && self.reaches(d, layer.0))
            .collect();
        let conductors: Vec<GvLayerId> = cands.iter().copied().filter(|d| self.deck.connectivity.conductors.contains(d)).collect();
        let pool = if conductors.is_empty() { cands } else { conductors };
        (pool.len() == 1).then(|| pool[0].0 as usize)
    }

    /// A number from the deck's `pex` row for `layer` ([`Pdk::pex_row`]):
    /// `thickness_nm`, `height_nm`, `sheet_res_ohm_sq`, `area_cap_af_um2`,
    /// `fringe_cap_af_um` or `dielectric_k`; `None` when absent.
    #[must_use]
    pub fn pex_f32(&self, layer: LayerId, key: &str) -> Option<f32> {
        self.pex_col(self.pex_row(layer)?, key)
    }

    /// [`Pdk::pex_f32`] of the deck layer `name`'s own row (sky130
    /// `licon_po`, the gate cut), never one of a layer carved from it.
    #[must_use]
    pub fn pex_f32_named(&self, name: &str, key: &str) -> Option<f32> {
        let i = self.gv_layer_by_name(name)?.0 as usize;
        self.pex_declared(i).then(|| self.pex_col(i, key))?
    }

    /// Body sheet resistance of the deck's device `model` (`deck_model`
    /// spelling), Ω/□: its recogniser's marker layer's own `pex` row (sky130
    /// `rbody_high_po`, 317.3885). `None` when the deck states none.
    #[must_use]
    pub fn device_sheet_ohm(&self, model: &str) -> Option<f32> {
        let name = self.deck_model(model)?;
        let d = &self.deck.devices;
        let r = (0..d.model.len()).find(|&r| self.strings.resolve(d.model[r]) == name)?;
        self.pex_f32_named(self.strings.resolve(self.deck.layers.name(d.marker[r])), "sheet_res_ohm_sq")
    }

    fn pex_col(&self, i: usize, key: &str) -> Option<f32> {
        let st = &self.deck.stack;
        let col = match key {
            "thickness_nm" => &st.thickness_nm,
            "height_nm" => &st.height_nm,
            "sheet_res_ohm_sq" => &st.sheet_res_ohm_sq,
            "area_cap_af_um2" => &st.area_cap_af_um2,
            "fringe_cap_af_um" => &st.fringe_cap_af_um,
            "dielectric_k" => &st.dielectric_k,
            _ => return None,
        };
        col.get(i).map(|&v| v as f32)
    }

    /// Ground capacitance of a `width_nm` wire on `layer`, aF/µm, from the
    /// deck's `pex` area and fringe terms; `None` when the deck has neither.
    #[must_use]
    pub fn wire_af_per_um(&self, layer: LayerId, width_nm: i32) -> Option<f32> {
        let (area, fringe) = (self.pex_f32(layer, "area_cap_af_um2")?, self.pex_f32(layer, "fringe_cap_af_um")?);
        if area <= 0.0 && fringe <= 0.0 {
            return None;
        }
        Some(area * width_nm as f32 / 1e3 + 2.0 * fringe)
    }

    /// Lateral (sidewall) capacitance between two parallel `layer` wires
    /// `gap_nm` apart, aF per µm of run: `ε0·k·t/gap` from the deck's `pex`
    /// thickness and dielectric (TOPO eq. 4.3, `C_2D = l·C/d`); `None` when the
    /// deck lacks either.
    ///
    /// ponytail: the parallel-plate sidewall term only; the fringe share of
    /// the coupling (it grows as the gap shrinks below the height) is not split
    /// out of the deck's ground fringe.
    #[must_use]
    pub fn lateral_af_per_um(&self, layer: LayerId, gap_nm: i32) -> Option<f32> {
        const EPS0_AF_PER_UM: f32 = 8.854;
        let (t, k) = (self.pex_f32(layer, "thickness_nm")?, self.pex_f32(layer, "dielectric_k")?);
        (gap_nm > 0 && t > 0.0 && k > 0.0).then(|| EPS0_AF_PER_UM * k * t / gap_nm as f32)
    }

    /// The deck's `min_spacing` for a layer in nm, if it declares one.
    #[must_use]
    pub fn min_spacing(&self, layer: u16) -> Option<i32> {
        self.rule_limit_nm("min_spacing", layer).map(|v| v as i32)
    }

    /// The deck's `min_width` for a layer in nm, if it declares one; for a
    /// cut, its exact `size` (the shorter side).
    #[must_use]
    pub fn min_width(&self, layer: u16) -> Option<i32> {
        self.rule_limit_nm("min_width", layer).or_else(|| {
            let (kind, w, h) = (self.strings.get("cut_size")?, self.strings.get("width")?, self.strings.get("height")?);
            // A cut sized on a part of the layer (sky130 `licon_std`: licon
            // outside the precision resistors) sizes the plain cut; the
            // smallest of several is the ordinary one, not a slot.
            self.deck.rules.spec.iter().filter_map(|s| {
                (s.kind == kind && self.deck.rules.layers_of(s).first().is_some_and(|&l| self.drawn_from(l, layer))).then_some(())?;
                match (self.deck.rules.param(s, w), self.deck.rules.param(s, h)) {
                    (Some(ParamValue::Length(a)), Some(ParamValue::Length(b))) => Some(a.raw().min(b.raw())),
                    (Some(ParamValue::Length(a)), _) => Some(a.raw()),
                    _ => None,
                }
            }).min()
        }).map(|v| v as i32)
    }

    /// Whether deck layer `x` is drawing layer `drawn` or computed from it
    /// (through any chain of derived layers) out of layers Philis draws only:
    /// a rule on a seal-ring, high-voltage or deep-well layer never fires on
    /// Philis geometry, so it must not set a generator dimension.
    #[must_use]
    pub fn drawn_from(&self, x: GvLayerId, drawn: u16) -> bool {
        self.philis_drawn(x) && self.reaches(x, drawn)
    }

    /// `x` is `drawn` or a part of it: through the operands whose area it
    /// keeps (all of an `and`/`or`, the first of a `not` or a selection).
    #[must_use]
    pub fn reaches(&self, x: GvLayerId, drawn: u16) -> bool {
        use gdsverify::ingest::deck::DerivedOp as Op;
        if x.0 == drawn {
            return true;
        }
        let ops = self.deck.layers.operands(x);
        let kept: &[GvLayerId] = match self.deck.layers.op(x) {
            None => &[],
            Some(Op::And | Op::Or) => ops,
            Some(_) => &ops[..ops.len().min(1)],
        };
        kept.iter().any(|&o| self.reaches(o, drawn))
    }

    /// `x` can hold area in Philis geometry: a drawn layer, or one computed
    /// so that it is non-empty given only drawn layers (`X and hvi` is empty,
    /// `X not thickgateox` is `X`).
    fn philis_drawn(&self, x: GvLayerId) -> bool {
        use gdsverify::ingest::deck::DerivedOp as Op;
        let ops = self.deck.layers.operands(x);
        let can = |i: usize| ops.get(i).is_some_and(|&o| self.philis_drawn(o));
        match self.deck.layers.op(x) {
            None => {
                DRAWN_ROLES.iter().any(|r| pnr_core::Process::layer(self, r).is_some_and(|l| l.0 == x.0))
                    || self.routing_metals.iter().chain(&self.routing_cuts).any(|l| l.0 == x.0)
                    || self.recipe_layers().iter().any(|n| self.layers.iter().any(|(m, l)| m == n && l.0 == x.0))
            }
            Some(Op::And) => (0..ops.len()).all(can),
            Some(Op::Or) => (0..ops.len()).any(can),
            Some(Op::Interacting(true) | Op::Inside) => can(0) && can(1),
            Some(_) => can(0),
        }
    }

    /// The widest length `param` of any `kind` rule on `layers`, nm: each
    /// rule layer the drawn layer or a part of it Philis geometry can produce
    /// ([`Pdk::drawn_from`]; a deck states contact rules on `poly_licon`,
    /// `licon_diff`…). Rules the sidecar lists in `cell.inapplicable_rules`
    /// (structures no Philis generator draws, e.g. sky130's varactor) do not
    /// set a dimension; the checker still runs them.
    pub(crate) fn widest_on(&self, kind: &str, param: &str, layers: &[u16]) -> Option<i64> {
        let (kind, param) = (self.strings.get(kind)?, self.strings.get(param)?);
        let skip: Vec<&str> = self.cell.get("inapplicable_rules").and_then(|v| v.as_array()).map_or(Vec::new(), |a| a.iter().filter_map(|x| x.as_str()).collect());
        self.deck
            .rules
            .spec
            .iter()
            .filter(|s| {
                let l = self.deck.rules.layers_of(s);
                s.kind == kind
                    && !skip.contains(&self.strings.resolve(s.id))
                    && l.len() == layers.len()
                    && l.iter().zip(layers).all(|(&x, &d)| self.drawn_from(x, d))
            })
            .filter_map(|s| match self.deck.rules.param(s, param) {
                Some(ParamValue::Length(d)) => Some(d.raw()),
                _ => None,
            })
            .max()
    }


    /// The shortest legal channel `(l, w)`, nm, of a MOS of `model` (`pmos`
    /// polarity): poly's and diffusion's min width, the sidecar's
    /// `min_gate_l`, any width rule on the gate (a layer drawn from both,
    /// gf180 `DF.2a`) that the recogniser LVS pairs it with is carved from,
    /// and, for `l`, any edge-length rule on that gate's edges (gf180 `PL.2`).
    /// ponytail: a gate-edge length rule is read as L, as the one on COMP's
    /// edges is; one on poly's edges (W) would need the edge's side.
    /// ponytail: carved-from is by derivation, so a rule on a sibling layer of
    /// the same area (sky130's 350 nm `lvtn_pfet`) binds no device; an area
    /// test would.
    #[must_use]
    pub fn min_channel(&self, pmos: bool, model: &str) -> (i32, i32) {
        let (Some(poly), Some(diff)) = (Process::layer(self, "poly"), Process::layer(self, "diff")) else { return (0, 0) };
        let d = &self.deck.devices;
        let rows: Vec<usize> = (0..d.kind.len())
            .filter(|&r| d.kind[r] == gdsverify::ingest::deck::DeviceKind::Mos)
            .filter(|&r| self.strings.resolve(self.deck.layers.name(d.marker[r])).starts_with('p') == pmos)
            .collect();
        let named = self.deck_model(model);
        let marker = rows.iter().find(|&&r| named.as_deref() == Some(self.strings.resolve(d.model[r]))).or(rows.first()).map(|&r| d.marker[r]);
        // The largest `kind` limit on a layer whose `area` is the gate.
        let bound = |kind: &str, area: &dyn Fn(GvLayerId) -> Option<GvLayerId>| {
            self.strings.get(kind).zip(self.strings.get("limit")).zip(marker).and_then(|((kind, limit), marker)| {
                self.deck.rules.spec.iter()
                    .filter(|s| s.kind == kind && matches!(self.deck.rules.layers_of(s), &[x] if area(x).is_some_and(|x| self.reaches(marker, x.0) && self.drawn_from(x, poly.0) && self.drawn_from(x, diff.0))))
                    .filter_map(|s| match self.deck.rules.param(s, limit) {
                        Some(ParamValue::Length(d)) => Some(d.raw() as i32),
                        _ => None,
                    })
                    .max()
            }).unwrap_or(0)
        };
        let gate = bound("min_width", &Some);
        let length = bound("edge_min_length", &|x| self.edge_source(x));
        let chosen = self.rules.iter().rev().find(|(n, _)| n == "min_gate_l").map_or(0, |(_, v)| *v);
        let w = |role| Process::width(self, role).unwrap_or(0);
        (w("poly").max(gate).max(length).max(chosen), w("diff").max(gate))
    }

    /// The area layer an edge layer's edges were taken from (through its
    /// first operands); `None` for an area layer.
    fn edge_source(&self, x: GvLayerId) -> Option<GvLayerId> {
        use gdsverify::ingest::deck::DerivedOp as Op;
        let ops = self.deck.layers.operands(x);
        match self.deck.layers.op(x)? {
            Op::Edges => ops.first().copied(),
            _ => self.edge_source(*ops.first()?),
        }
    }

    /// Widest `min_spacing` of a layer sized from drawn layer `l`, seen on
    /// `l`: the limit plus the growth on both sides.
    fn sized_spacing(&self, l: u16) -> Option<i32> {
        let (kind, limit) = (self.strings.get("min_spacing")?, self.strings.get("limit")?);
        self.deck
            .rules
            .spec
            .iter()
            .filter(|s| s.kind == kind)
            .filter_map(|s| {
                let &[x] = self.deck.rules.layers_of(s) else { return None };
                (x.0 != l && self.philis_drawn(x)).then_some(())?;
                let g = self.growth(x, l)?;
                match self.deck.rules.param(s, limit) {
                    Some(ParamValue::Length(d)) => Some(d.raw() as i32 + 2 * g),
                    _ => None,
                }
            })
            .max()
    }

    /// Net growth of `x` from drawn layer `l` through sizing (and the first
    /// operand of any other op); `None` if `x` is not built from `l`.
    fn growth(&self, x: GvLayerId, l: u16) -> Option<i32> {
        use gdsverify::ingest::deck::DerivedOp as Op;
        if x.0 == l {
            return Some(0);
        }
        let ops = self.deck.layers.operands(x);
        match self.deck.layers.op(x)? {
            Op::Sized(d) => Some(self.growth(ops[0], l)? + d.raw() as i32),
            Op::Or => ops.iter().find_map(|&o| self.growth(o, l)),
            _ => self.growth(*ops.first()?, l),
        }
    }

    /// The `limit` (nm) of the first rule of `kind` whose first layer is `layer`.
    fn rule_limit_nm(&self, kind: &str, layer: u16) -> Option<i64> {
        let kind = self.strings.get(kind)?;
        let limit = self.strings.get("limit")?;
        self.deck.rules.spec.iter().find_map(|s: &RuleSpec| {
            if s.kind != kind {
                return None;
            }
            if self.deck.rules.layers_of(s).first().map(|l| l.0) != Some(layer) {
                return None;
            }
            match self.deck.rules.param(s, limit) {
                Some(ParamValue::Length(d)) => Some(d.raw()),
                _ => None,
            }
        })
    }
}

/// `Pdk` is the concrete [`Process`] the whole pipeline binds against: `cells`
/// generators (`&dyn Process`) and `macroMaster` resolve every layer role and
/// rule through it, so they stay PDK-agnostic while this maps to the real deck.
impl Process for Pdk {
    /// Resolve a role through the deck's `cell.layers` map first; then a
    /// stack position, `met{n}` / `via{n}` (the `n`-th metal above li and the
    /// cut below it, as sky130 names them: `routing_metals[n]`,
    /// `routing_cuts[n]`), so a deck whose layers are named otherwise (ihp's
    /// `via1` is li's cut) still answers by position; then a layer of that
    /// literal name.
    ///
    /// The indirection is the point: a role is what a generator wants (`"tap"`),
    /// a layer is what the process provides.
    fn layer(&self, role: &str) -> Option<LayerId> {
        if let Some((_, l)) = self.roles.iter().find(|(r, _)| r == role) {
            return self.layers.iter().find(|(n, _)| n == l).map(|(_, id)| *id);
        }
        let at = |prefix: &str, stack: &[LayerId]| {
            role.strip_prefix(prefix)?.parse::<usize>().ok().and_then(|n| stack.get(n).copied())
        };
        at("met", &self.routing_metals)
            .or_else(|| at("via", &self.routing_cuts))
            .or_else(|| self.layers.iter().find(|(n, _)| n == role).map(|(_, id)| *id))
    }
    /// Last match wins: `rules` is built widest-source-first, so an explicit
    /// `cell.*` dimension overrides a value synthesised from a DRC rule.
    /// No `min_gate_l` chosen: the deck's shortest legal gate, either polarity.
    fn rule(&self, name: &str, default: i32) -> i32 {
        let deck = || (name == "min_gate_l").then(|| self.min_channel(false, "").0.max(self.min_channel(true, "").0)).filter(|&l| l > 0);
        self.rules
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
            .or_else(deck)
            .unwrap_or(default)
    }
    fn grid(&self) -> i32 {
        self.grid
    }
    fn sheet_ohm(&self, role: &str) -> Option<f32> {
        self.pex_f32(pnr_core::Process::layer(self, role)?, "sheet_res_ohm_sq")
    }
    /// The cut's own `pex` row, else the largest of the cut rows carved from
    /// both `cut` and `onto` (conservative: sky130 licon on tap is `licon_nt`
    /// 185 or `licon_pt` 585 → 585).
    fn cut_ohm(&self, cut: &str, onto: &str) -> Option<f32> {
        let (c, o) = (pnr_core::Process::layer(self, cut)?.0, pnr_core::Process::layer(self, onto)?.0);
        if self.pex_declared(c as usize) {
            return self.pex_col(c as usize, "sheet_res_ohm_sq");
        }
        (0..self.deck.layers.len())
            .map(|i| GvLayerId(i as u16))
            .filter(|&d| self.pex_declared(d.0 as usize) && self.reaches(d, c) && self.reaches(d, o))
            .filter_map(|d| self.pex_col(d.0 as usize, "sheet_res_ohm_sq"))
            .reduce(f32::max)
    }
    fn space(&self, role: &str) -> Option<i32> {
        let l = pnr_core::Process::layer(self, role)?.0;
        let plain = self.min_spacing(l);
        let array = self.via_array_spacing(l).map(|(_, s)| s);
        // Wide-metal steps a cell's features can reach; wider-only spacings
        // bind the router's straps, which read `wide_spacing` per width.
        let wide = self.wide_spacing(l).iter().filter(|&&(t, _)| t <= MAX_CELL_FEATURE_NM).map(|&(_, s)| s).max();
        // A layer the deck grows from this one (ihp's buried layer: n-well
        // opened, then grown 1 um) spaces its growth apart too.
        let grown = self.sized_spacing(l);
        plain.max(array).max(wide).max(grown)
    }
    fn eol_space(&self, role: &str) -> Option<i32> {
        let l = pnr_core::Process::layer(self, role)?.0;
        self.widest_on("eol_spacing", "limit", &[l]).map(|v| v as i32)
    }
    fn width(&self, role: &str) -> Option<i32> {
        let l = pnr_core::Process::layer(self, role)?.0;
        self.min_width(l).or_else(|| self.widest_on("min_width", "limit", &[l]).map(|v| v as i32))
    }
    fn enclosure(&self, outer: &str, inner: &str) -> Option<i32> {
        let l = [pnr_core::Process::layer(self, outer)?.0, pnr_core::Process::layer(self, inner)?.0];
        self.widest_on("min_enclosure", "limit", &l).map(|v| v as i32)
    }
    fn endcap(&self, outer: &str, inner: &str) -> Option<i32> {
        let l = [pnr_core::Process::layer(self, outer)?.0, pnr_core::Process::layer(self, inner)?.0];
        self.widest_on("asymmetric_enclosure", "min_one_side", &l).map(|v| v as i32)
    }
    fn area(&self, role: &str) -> Option<i64> {
        self.min_area(pnr_core::Process::layer(self, role)?.0)
    }
    fn extension(&self, outer: &str, inner: &str) -> Option<i32> {
        let l = [pnr_core::Process::layer(self, outer)?.0, pnr_core::Process::layer(self, inner)?.0];
        self.widest_on("min_extension", "limit", &l).map(|v| v as i32)
    }
    fn space_between(&self, a: &str, b: &str) -> Option<i32> {
        let (la, lb) = (pnr_core::Process::layer(self, a)?.0, pnr_core::Process::layer(self, b)?.0);
        let one = |x, y| self.widest_on("min_spacing_diff", "limit", &[x, y]);
        one(la, lb).max(one(lb, la)).map(|v| v as i32)
    }
}

/// Pull every rule's scalar length parameter (nm) out of the deck into
/// `(rule_id, value_nm)` pairs for the generator-facing `rules` view.
/// `limit` and `min_one_side` (asymmetric enclosure) are the two single-length
/// spellings; ratio/count/flag parameters (density, antenna, angle…) aren't a
/// single scalar length the generators query — skipped from this view.
fn scalar_rules(deck: &Deck, strings: &StrTable) -> Vec<(String, i32)> {
    let keys: Vec<_> = ["limit", "min_one_side"].iter().filter_map(|k| strings.get(k)).collect();
    deck.rules
        .spec
        .iter()
        .filter_map(|s| {
            keys.iter().find_map(|&k| match deck.rules.param(s, k) {
                Some(ParamValue::Length(d)) => {
                    Some((strings.resolve(s.id).to_string(), d.raw() as i32))
                }
                _ => None,
            })
        })
        .collect()
}

/// Layer roles every cell generator resolves unconditionally (`cells::req`).
const REQUIRED_ROLES: &[&str] = &[
    "diff", "poly", "li", "licon", "mcon", "met1", "nwell", "nsdm", "psdm", "tap",
];

/// A device construction from the sidecar's `cell.<kind>s` table: which
/// layers a generator draws for one schematic model and the deck model the
/// extractor then recognises (sky130 `res_high_po`: poly_rs + rpm + npc +
/// psdm with slot contacts; `res_generic_po`: poly_rs alone).
#[derive(Clone, Debug, Default)]
pub struct Recipe {
    /// The deck's model name for what this draws (its LVS recogniser).
    pub model: String,
    /// Role → layer; a controlled role absent here draws nothing.
    pub layers: Vec<(String, String)>,
    /// Dimension keys overriding the sidecar's.
    pub rules: Vec<(String, i32)>,
}

/// Roles a resistor recipe controls: unset in the recipe = not drawn.
const RESISTOR_ROLES: &[&str] = &["rpoly", "rpoly_b", "res_block", "rpm", "npc", "res_implant"];

/// Roles a capacitor recipe controls (MIM: `bottom` plate, `plate` the
/// insulator/top-plate marker, `top_contact` cuts onto it, `top` and `strap`
/// metal, `bottom_contact` the cut off the bottom plate's stub).
const CAPACITOR_ROLES: &[&str] = &["bottom", "plate", "top_contact", "top", "strap", "bottom_contact"];

impl Pdk {
    /// How far `outer` must pass cut `inner` on every side, nm: the deck's
    /// all-round enclosure and its two-opposite-sides end-cap, the larger
    /// (a centred square pad clears both). `0` when the deck sets none.
    #[must_use]
    pub fn cut_enclosure(&self, outer: LayerId, inner: LayerId) -> i32 {
        let l = [outer.0, inner.0];
        let e = self.widest_on("min_enclosure", "limit", &l).unwrap_or(0);
        let c = self.widest_on("asymmetric_enclosure", "min_one_side", &l).unwrap_or(0);
        e.max(c) as i32
    }

    /// Every layer some recipe draws.
    fn recipe_layers(&self) -> Vec<String> {
        let mut out = Vec::new();
        for kind in ["resistors", "capacitors"] {
            let Some(rs) = self.cell.get(kind).and_then(|t| t.get("recipes")).and_then(|r| r.as_object()) else { continue };
            for r in rs.values() {
                if let Some(l) = r.get("layers").and_then(|l| l.as_object()) {
                    out.extend(l.values().filter_map(|v| v.as_str().map(String::from)));
                }
            }
        }
        out
    }

    /// The deck's name for schematic `model` (sky130 `nfet_01v8` →
    /// `sky130_fd_pr__nfet_01v8`): equal, or one is the other behind a vendor
    /// `__` prefix. `None` when the deck names no such model.
    #[must_use]
    pub fn deck_model(&self, model: &str) -> Option<String> {
        let hit = |n: &str| !model.is_empty() && (n == model || n.ends_with(&format!("__{model}")) || model.ends_with(&format!("__{n}")));
        self.deck.devices.model.iter().map(|&m| self.strings.resolve(m)).find(|n| hit(n)).map(str::to_string)
    }

    /// The recipe for a `kind` (`"resistor"`) of schematic `model`: the one
    /// naming it (its deck model or an alias, a vendor prefix ignored), else
    /// the table's `default`. `None` when the sidecar has no table.
    #[must_use]
    pub fn recipe(&self, kind: &str, model: &str) -> Option<Recipe> {
        let table = self.cell.get(format!("{kind}s"))?;
        let recipes = table.get("recipes")?.as_object()?;
        let names = |r: &serde_json::Value| -> Vec<String> {
            let mut v: Vec<String> = r.get("aliases").and_then(|a| a.as_array()).map_or(Vec::new(), |a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect());
            v.extend(r.get("model").and_then(|m| m.as_str()).map(String::from));
            v
        };
        let hit = |r: &serde_json::Value| !model.is_empty() && names(r).iter().any(|n| n == model || n.ends_with(&format!("__{model}")) || model.ends_with(&format!("__{n}")));
        let r = recipes.values().find(|r| hit(r)).or_else(|| recipes.get(table.get("default")?.as_str()?))?;
        Some(Recipe {
            model: r.get("model").and_then(|m| m.as_str()).unwrap_or_default().to_string(),
            layers: r.get("layers").and_then(|l| l.as_object()).map_or(Vec::new(), |l| l.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()),
            rules: r.as_object().map_or(Vec::new(), |o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_i64()? as i32))).collect()),
        })
    }
}

/// `pdk` as one recipe sees it: the recipe's roles and keys first; a role it
/// controls but leaves unset resolves to no layer.
pub struct Overlay<'a> {
    pub pdk: &'a Pdk,
    pub recipe: Recipe,
}

impl Process for Overlay<'_> {
    fn layer(&self, role: &str) -> Option<LayerId> {
        match self.recipe.layers.iter().find(|(r, _)| r == role) {
            Some((_, l)) => self.pdk.layers.iter().find(|(n, _)| n == l).map(|(_, id)| *id),
            None if RESISTOR_ROLES.contains(&role) || CAPACITOR_ROLES.contains(&role) => None,
            None => self.pdk.layer(role),
        }
    }
    fn rule(&self, name: &str, default: i32) -> i32 {
        self.recipe.rules.iter().find(|(n, _)| n == name).map_or_else(|| self.pdk.rule(name, default), |&(_, v)| v)
    }
    fn grid(&self) -> i32 {
        self.pdk.grid()
    }
    /// A resistor role is the recipe's device body ([`Pdk::device_sheet_ohm`]),
    /// not the layer it is drawn on.
    fn sheet_ohm(&self, role: &str) -> Option<f32> {
        if RESISTOR_ROLES.contains(&role) {
            return self.pdk.device_sheet_ohm(&self.recipe.model);
        }
        self.pdk.sheet_ohm(role)
    }
    fn cut_ohm(&self, cut: &str, onto: &str) -> Option<f32> {
        self.pdk.cut_ohm(cut, onto)
    }
    fn space(&self, role: &str) -> Option<i32> {
        // By the overlay's layer, so a recipe role (`res_block`) resolves.
        let name = &self.pdk.layers.iter().find(|(_, l)| Some(*l) == self.layer(role))?.0;
        self.pdk.space(name)
    }
    fn width(&self, role: &str) -> Option<i32> {
        self.pdk.min_width(self.layer(role)?.0)
    }
    // By the overlay's layers, so a recipe role (`res_implant`) resolves.
    fn enclosure(&self, outer: &str, inner: &str) -> Option<i32> {
        self.pdk.widest_on("min_enclosure", "limit", &[self.layer(outer)?.0, self.layer(inner)?.0]).map(|v| v as i32)
    }
    fn endcap(&self, outer: &str, inner: &str) -> Option<i32> {
        self.pdk.widest_on("asymmetric_enclosure", "min_one_side", &[self.layer(outer)?.0, self.layer(inner)?.0]).map(|v| v as i32)
    }
    fn area(&self, role: &str) -> Option<i64> {
        self.pdk.min_area(self.layer(role)?.0)
    }
    fn space_between(&self, a: &str, b: &str) -> Option<i32> {
        let (la, lb) = (self.layer(a)?.0, self.layer(b)?.0);
        let one = |x, y| self.pdk.widest_on("min_spacing_diff", "limit", &[x, y]);
        one(la, lb).max(one(lb, la)).map(|v| v as i32)
    }
    fn extension(&self, outer: &str, inner: &str) -> Option<i32> {
        let l = [self.layer(outer)?.0, self.layer(inner)?.0];
        self.pdk.widest_on("min_extension", "limit", &l).map(|v| v as i32)
    }
    fn eol_space(&self, role: &str) -> Option<i32> {
        self.pdk.widest_on("eol_spacing", "limit", &[self.layer(role)?.0]).map(|v| v as i32)
    }
}

/// The widest single feature a cell generator draws, nm (a cap plate or a
/// resistor body; li and metal runs inside cells are far narrower). A
/// design bound, not process data: wide-metal spacings that only start past
/// it are for the router's straps.
const MAX_CELL_FEATURE_NM: i32 = 5_000;

/// Every role a Philis generator draws on (optional ones resolve to nothing
/// on a deck without them).
const DRAWN_ROLES: &[&str] =
    &["diff", "tap", "poly", "rpoly", "li", "licon", "mcon", "met1", "nwell", "dnwell", "nsdm", "psdm", "npc", "rpm", "pnp", "npn", "hvtp", "lvtn", "fin", "sdt", "lisd", "lig", "gcut"];

/// The deck's `cell` section, which gdsverify's reader ignores.
#[derive(Default)]
struct Roles {
    /// role → deck layer name (every string-valued `cell.layers` key).
    map: Vec<(String, String)>,
    routing_metals: Vec<String>,
    routing_vias: Vec<String>,
    /// Integer (or boolean → 0/1) `cell.*` dimensions: `contact`, `sd_width`, …
    scalars: Vec<(String, i32)>,
}

fn parse_roles(text: &str) -> Result<Roles, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("deck is not valid JSON: {e}"))?;
    let Some(cell) = v.get("cell").and_then(|c| c.as_object()) else {
        return Err("deck has no `cell` section: it declares no layer roles, no routing \
                    stack and no construction dimensions, so generators cannot resolve a \
                    layer and the router has no legal set of layers to use"
            .into());
    };
    let Some(obj) = cell.get("layers").and_then(|l| l.as_object()) else {
        return Err("deck has no `cell.layers` section: it declares no layer roles and no \
                    routing stack"
            .into());
    };
    let mut roles = Roles::default();
    let list = |key: &str| -> Result<Vec<String>, String> {
        obj.get(key)
            .and_then(|a| a.as_array())
            .ok_or_else(|| format!("cell.layers.{key} is missing or not an array"))?
            .iter()
            .map(|e| {
                e.as_str()
                    .map(String::from)
                    .ok_or_else(|| format!("cell.layers.{key} contains a non-string entry"))
            })
            .collect()
    };
    roles.routing_metals = list("routing_metals")?;
    roles.routing_vias = list("routing_vias")?;
    for (k, val) in obj {
        if let Some(s) = val.as_str() {
            roles.map.push((k.clone(), s.to_string()));
        }
    }
    for (k, val) in cell {
        if k == "layers" {
            continue;
        }
        let n = val.as_i64().or_else(|| val.as_bool().map(i64::from));
        if let Some(n) = n {
            roles.scalars.push((k.clone(), n as i32));
        }
    }
    Ok(roles)
}

/// Manufacturing grid from the deck's `off_grid` rule (`pitch` param), else `1`.
fn deck_grid(deck: &Deck, strings: &StrTable) -> i32 {
    let (Some(kind), Some(pitch)) = (strings.get("off_grid"), strings.get("pitch")) else {
        return 1;
    };
    deck.rules
        .spec
        .iter()
        .filter(|s| s.kind == kind)
        .find_map(|s| match deck.rules.param(s, pitch) {
            Some(ParamValue::Length(d)) => Some(d.raw() as i32),
            _ => None,
        })
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(deck: &str) -> Pdk {
        let path = format!("{}/../../pdks/{deck}.json", env!("CARGO_MANIFEST_DIR"));
        Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// gf180's gate width rule (`DF.2a`, 220 nm) is wider than its poly's
    /// (`PL.1`, 180 nm): the channel minimum reads the gate, not the poly.
    #[test]
    fn label_gds_is_the_text_layer_not_the_drawing_layer() {
        let sky = load("sky130");
        let met1 = Process::layer(&sky, "met1").expect("sky130 met1").0;
        // sky130 also lets text on met1 drawing (68/20) name the net; other
        // tools read only 68/5, so that is what export writes.
        assert_eq!(sky.label_gds(met1), Some((68, 5)));
        let gf = load("gf180mcu");
        let m1 = Process::layer(&gf, "met1").expect("gf180 met1 role").0;
        let label = gf.label_gds(m1).expect("gf180 labels metal1 through metal1_label");
        assert_ne!(label, gf.layer_gds()[m1 as usize], "a text layer, not the drawing layer");
    }

    #[test]
    fn reference_spice_writes_ports_dummy_models_and_micron_lengths() {
        use crate::reference::{RefDeviceIn, RefInput, RefKind};
        let sky = load("sky130");
        let r = RefInput {
            devices: vec![
                RefDeviceIn { kind: RefKind::Nmos, model: Some("nfet_01v8".into()), terminals: vec!["d".into(), "g".into(), "vss".into(), "vss".into()], params: vec![("w".into(), 0.42e-6), ("l".into(), 0.15e-6)] },
                RefDeviceIn { kind: RefKind::Pmos, model: None, terminals: vec!["vdd".into(); 4], params: vec![("w".into(), 1.15e-6), ("l".into(), 0.15e-6)] },
            ],
            ports: vec!["d".into(), "g".into(), "vss".into(), "vdd".into(), "internal".into()],
            ..RefInput::default()
        };
        let sp = sky.reference_spice(&r, "cell", &["d".into(), "g".into(), "vss".into(), "vdd".into()]);
        assert_eq!(
            sp,
            ".subckt cell d g vss vdd\n\
             X0 d g vss vss sky130_fd_pr__nfet_01v8 w=0.42 l=0.15\n\
             X1 vdd vdd vdd vdd sky130_fd_pr__pfet_01v8 w=1.15 l=0.15\n\
             .ends\n",
            "short model canonicalised, dummy gets the deck's pfet, nm-rounded µm, interface ports only"
        );
    }

    #[test]
    fn min_channel_reads_the_gate_width_rule() {
        let gf = load("gf180mcu");
        let (l, w) = gf.min_channel(false, "nfet_01v8");
        assert_eq!((l, w), (280, 220), "gf180 3.3 V: PL.2 gate length, DF.2a width");
        assert_eq!(Process::rule(&gf, "min_gate_l", 0), l, "no sidecar gate length: the deck's");
        let sky = load("sky130");
        assert_eq!(sky.min_channel(true, "pfet_01v8").0, 150, "a plain PMOS is not held to the LVT one's 350 nm");
    }
    /// Every shipped PDK loads from the binary alone: sidecar and deck are
    /// the compiled-in copies, so a moved or packaged `philis` has its PDKs.
    #[test]
    fn every_builtin_pdk_loads() {
        for (name, _) in crate::decks::SIDECARS {
            let pdk = Pdk::builtin(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            let (_, deck) = crate::decks::DECKS.iter().find(|(d, _)| *d == format!("{name}.deck")).unwrap();
            assert_eq!(pdk.source, *deck, "{name}: deck text is not the vendored one");
        }
        assert!(Pdk::builtin("sky13").is_err());
    }

    /// Line 1 of each vendored deck names the GPurify commit it was copied
    /// from, which must be the rev `Cargo.lock` pins (build.rs
    /// `GPURIFY_REV`): a lockfile bump without re-vendoring the decks fails.
    #[test]
    fn vendored_decks_name_their_origin() {
        let rev = env!("GPURIFY_REV");
        for (name, text) in crate::decks::DECKS {
            let first = text.lines().next().unwrap_or_default();
            assert!(first.starts_with(&format!("# vendored from GPurify {rev}")), "{name}: line 1 is {first:?}, Cargo.lock pins {rev}");
        }
    }

    /// The capacitor table adds capm/met4 to the layers cell rules are read
    /// on (`recipe_layers`): routing metal rules must not move, and the MIM
    /// recipe's plate enclosure (capm.3) must resolve through it.
    #[test]
    fn capacitor_recipes_do_not_move_routing_rules() {
        let with = sky130_with(|_| {}).unwrap();
        let without = sky130_with(|c| {
            c.remove("capacitors");
        })
        .unwrap();
        assert!(with.cell.get("capacitors").is_some() && without.cell.get("capacitors").is_none());
        let q = |p: &Pdk| (Process::space(p, "met3"), Process::width(p, "met3"), Process::space(p, "met4"), Process::enclosure(p, "met3", "via3"));
        assert_eq!(q(&with), q(&without));
        let ov = Overlay { pdk: &with, recipe: with.recipe("capacitor", "sky130_fd_pr__cap_mim_m3_1").unwrap() };
        assert_eq!(ov.enclosure("bottom", "plate"), Some(140), "capm.3");
    }

    fn sky130_with(edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>)) -> Result<Pdk, String> {
        let text = crate::decks::SIDECARS[0].1;
        let mut v: serde_json::Value = serde_json::from_str(text).unwrap();
        edit(v["cell"].as_object_mut().unwrap());
        Pdk::from_json(&v.to_string())
    }

    /// A misspelt key is an error, not a value silently ignored while the
    /// generator falls back to its default.
    #[test]
    fn sidecar_rejects_a_misspelt_key() {
        let err = sky130_with(|c| {
            c.insert("wpe_clearence_nm".into(), serde_json::json!([2000, 3000, 5000]));
        })
        .err()
        .expect("a misspelt key must not load");
        assert!(err.contains("wpe_clearence_nm"), "{err}");
    }

    /// A required key that is missing, null or of the wrong kind does not
    /// load (`sd_width` is read by every MOSFET with a compiled default).
    #[test]
    fn a_required_key_must_be_present_and_well_kinded() {
        for (edit, why) in [(None, "required"), (Some(serde_json::Value::Null), "required"), (Some(serde_json::json!(280.5)), "expected Nm")] {
            let err = sky130_with(|c| match edit {
                None => drop(c.remove("sd_width")),
                Some(v) => drop(c.insert("sd_width".into(), v)),
            })
            .err()
            .unwrap_or_else(|| panic!("sd_width {why}: loaded"));
            assert!(err.contains("cell.sd_width") && err.contains(why), "{err}");
        }
    }

    /// A process number without its source does not load; one on an
    /// `UNVERIFIED` source loads and is listed as assumed.
    #[test]
    fn a_process_number_needs_a_source() {
        let err = sky130_with(|c| {
            c.remove("avt_n_mv_um_source");
        })
        .err()
        .expect("an unsourced process number must not load");
        assert!(err.contains("avt_n_mv_um"), "{err}");
        let sky = Pdk::builtin("sky130").unwrap();
        assert!(sky.provenance("avt_n_mv_um").is_some_and(|s| s.starts_with("Monte Carlo")));
        let assumed = sky.unverified();
        assert!(assumed.contains(&"gate_cap_af_um2") && !assumed.contains(&"avt_n_mv_um"), "{assumed:?}");
        assert!(!assumed.contains(&"n_well_depth"), "UNVERIFIED but read by no code, so not assumed: {assumed:?}");
        assert!(!assumed.contains(&"tie_max_dist_nm"), "LU.2/LU.2.1/LU.3 source it: {assumed:?}");
    }

    const DECKS: [&str; 3] = ["sky130", "gf180mcu", "ihp_sg13g2"];
    fn id(p: &Pdk, n: &str) -> LayerId {
        p.layers.iter().find(|(l, _)| l == n).unwrap().1
    }

    /// A router indexes `routing_layers()` by its internal layer index, so the
    /// stack must hold metal only: a device-formation or mask layer in it gets
    /// wires drawn on it (~330 nwell/diff DRC violations and a broken LVS once).
    #[test]
    fn routing_stack_is_metal_only() {
        for deck in DECKS {
            let pdk = load(deck);
            assert!(!pdk.routing_layers().is_empty(), "{deck}: empty routing stack");
            let device: Vec<LayerId> = ["diff", "tap", "poly", "nwell", "nsdm", "psdm", "licon"].iter().filter_map(|r| pnr_core::Process::layer(&pdk, r)).collect();
            for l in pdk.routing_layers() {
                assert!(!device.contains(&l), "{deck}: {l:?} is a device layer in the routing stack");
                assert!(pdk.deck.connectivity.conductors.iter().any(|&c| pdk.reaches(c, l.0)), "{deck}: {l:?} conducts nothing");
            }
        }
    }

    /// The via stack lines up with the metal stack one-for-one, and each cut
    /// is drawable: no wider than the pads enclosing it, not itself routing
    /// metal, pads on the fab grid. The same conditions `library::run`
    /// asserts, caught in `cargo test` rather than deep in a flow.
    #[test]
    fn via_stack_matches_metal_stack() {
        for deck in DECKS {
            let pdk = load(deck);
            let layers = pdk.routing_layers();
            let cuts = pdk.routing_vias();
            assert_eq!(cuts.len(), layers.len() - 1, "{deck}: {} metals need {} cuts", layers.len(), layers.len() - 1);
            for &(cut, size, below, above) in &cuts {
                assert!(size > 0, "{deck}: cut {cut:?} has no size");
                assert!(size <= below && size <= above, "{deck}: cut {cut:?} {size} nm exceeds its pads {below}/{above}");
                assert!(!layers.contains(&cut), "{deck}: {cut:?} is both routing metal and a via cut");
                for pad in [below, above] {
                    assert_eq!(pad % pdk.grid, 0, "{deck}: pad {pad} for cut {cut:?} is off the {} nm grid", pdk.grid);
                }
            }
        }
    }

    /// `routing_pitch` leaves at least `min_spacing` between adjacent tracks
    /// on every layer of the stack it is given: one global pitch, so the worst
    /// layer in that stack sets it.
    #[test]
    fn routing_pitch_clears_every_layer_of_its_stack() {
        for deck in DECKS {
            let pdk = load(deck);
            let w = 290;
            let stack: Vec<_> = pdk.routing_layers().into_iter().take_while(|l| pdk.min_width(l.0).is_none_or(|mw| mw <= w)).map(|l| l.0).collect();
            let pitch = pdk.routing_pitch(w, &stack);
            for &l in &stack {
                let need = pdk.min_spacing(l).unwrap_or(0);
                assert!(pitch - w >= need, "{deck}: pitch {pitch} leaves {} nm, layer {l:?} needs {need}", pitch - w);
            }
        }
    }

    /// A layer the router cannot reach must not set the pitch for the layers
    /// it can (sky130 met5's spacing once gave a 1890 nm lattice on a 170 nm pin).
    #[test]
    fn an_unreachable_layer_does_not_set_the_pitch() {
        let pdk = load("sky130");
        let all: Vec<_> = pdk.routing_layers().into_iter().map(|l| l.0).collect();
        let reachable: Vec<_> = pdk.routing_layers().into_iter().take_while(|l| pdk.min_width(l.0).is_none_or(|mw| mw <= 290)).map(|l| l.0).collect();
        assert!(pdk.routing_pitch(290, &reachable) < pdk.routing_pitch(290, &all));
    }

    /// No diode is inserted where LVS cannot extract one: gf180's deck has no
    /// diode recogniser on conductors.
    #[test]
    fn a_diode_marker_only_where_lvs_can_extract_it() {
        assert_eq!(load("gf180mcu").diode_marker(), None);
    }

    /// A generator asking by role gets this deck's minimum-area side: gf180's
    /// li role is metal1, whose M1.3 area is 0.1444 um2 (a 380 nm square).
    #[test]
    fn role_min_area_is_the_decks_side() {
        assert_eq!(pnr_core::Process::rule(&load("gf180mcu"), "li_min_area", 0), 380);
    }

    /// Antenna stages per layer, from the deck's own rules: sky130 metal1 a
    /// sidewall ratio, ihp summing metals cumulatively, a cut its own ratio.
    #[test]
    fn antenna_rules_are_read_per_stage() {
        let sky = load("sky130");
        assert_eq!(sky.antenna_rule(id(&sky, "met1")), Some((400.0, 350.0, false)), "ar.met1.1");
        assert_eq!(sky.antenna_rule(id(&sky, "mcon")), Some((3.0, 0.0, false)), "ar.mcon.1");
        let ihp = load("ihp_sg13g2");
        assert_eq!(ihp.antenna_rule(id(&ihp, "metal2")), Some((200.0, 0.0, true)), "Ant.b_Metal2 sums metal1+metal2");
        assert_eq!(ihp.antenna_rule(id(&ihp, "metal1")), Some((200.0, 0.0, false)));
    }

    /// EM limits come from the deck's own rules, per layer, through the
    /// conductor extracted from it (gf180 `metal1_con`); a cut a metal's rule
    /// names takes its per-cut limit, never the per-um density; a deck
    /// without EM rules is unknown.
    #[test]
    fn em_limits_are_read_per_layer_from_the_deck_rules() {
        let ihp = load("ihp_sg13g2");
        let m2 = ihp.em_limit(id(&ihp, "metal2")).expect("EM.Metal2");
        assert_eq!((m2.ua_per_um, m2.ua_per_cut), (2_000.0, 400.0));
        let v1 = ihp.em_limit(id(&ihp, "via1")).expect("EM.Metal2 names via1");
        assert_eq!((v1.ua_per_um, v1.ua_per_cut), (0.0, 400.0));
        let gf = load("gf180mcu");
        let m1 = gf.em_limit(id(&gf, "metal1")).expect("EM.14.3_Mn on metal1_con");
        assert_eq!((m1.ua_per_um, m1.ua_per_cut), (2_090.0, 580.0));
        // sky130's tech-LEF limits, met1 with its mcon.
        let sky = load("sky130");
        let m1 = sky.em_limit(id(&sky, "met1")).expect("EM.met1_mcon");
        assert_eq!((m1.ua_per_um, m1.ua_per_cut), (2_800.0, 360.0));
        // A deck without EM rules is unknown.
        let sidecar = std::fs::read_to_string(format!("{}/../../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let text: String = Pdk::deck_text(&sidecar).unwrap().lines().filter(|l| !l.contains("em_current_density")).collect::<Vec<_>>().join("\n");
        let bare = Pdk::load(&text, &sidecar).unwrap();
        assert!(bare.routing_metals.iter().all(|&l| bare.em_limit(l).is_none()), "no EM rules: unknown");
    }

    /// sky130 LU.2: an NMOS diffusion (n+ diff outside any well) 20 µm from
    /// the nearest p-tap is a finding of that rule; 10 µm away it is not.
    #[test]
    fn latch_up_rules_find_a_far_tap() {
        let sky = load("sky130");
        let lu2 = |gap: i32| {
            let rect = |n: &str, x: i32| pnr_core::Shape { layer: id(&sky, n), rect: pnr_core::Rect { x, y: 0, w: 1_000, h: 1_000 } };
            let far = 1_000 + gap;
            let shapes = [rect("diff", 0), rect("nsdm", 0), rect("tap", far), rect("psdm", far)];
            crate::drc(&shapes, &[], &sky).iter().filter(|f| f.rule == "LU.2").count()
        };
        assert!(lu2(20_000) > 0, "20 um from its p-tap: LU.2 (< 15 um)");
        assert_eq!(lu2(10_000), 0, "10 um from its p-tap is within LU.2");
    }

    /// Each routing metal's antenna sidewall thickness is the metal's `pex`
    /// thickness within 5 % [policy] (sky130 met1/met2 350 vs 360 nm;
    /// ar.met3.1 once said 2000 against 845).
    #[test]
    fn antenna_sidewall_thickness_matches_pex() {
        let sky = load("sky130");
        let mut checked = 0;
        for &m in &sky.routing_metals {
            let Some((_, side, _)) = sky.antenna_rule(m).filter(|r| r.1 > 0.0) else { continue };
            let t = sky.pex_f32(m, "thickness_nm").unwrap();
            assert!((side - t).abs() / t <= 0.05, "{m:?}: sidewall {side} nm against pex thickness {t} nm");
            checked += 1;
        }
        assert_eq!(checked, sky.routing_metals.len(), "every sky130 routing metal has a sidewall rule");
    }

    /// A resistor's sheet is its body's row, never the conductor it is cut
    /// from; a role with several candidate rows answers `None`.
    #[test]
    fn resistor_body_sheet_is_the_body() {
        let sky = load("sky130");
        assert_eq!(sky.device_sheet_ohm("sky130_fd_pr__res_generic_po"), Some(48.2));
        let high = sky.device_sheet_ohm("sky130_fd_pr__res_high_po").unwrap();
        assert!((high - 317.3885).abs() <= 1e-3, "{high}");
        assert_eq!(Process::sheet_ohm(&sky, "licon"), None, "five licon cut rows: ambiguous");
        assert_eq!(Process::sheet_ohm(&sky, "rpoly"), None, "poly_rs: only resistor bodies keep it");
        // The overlay answers a resistor role with its recipe's body.
        let overlay = Overlay { pdk: &sky, recipe: sky.recipe("resistor", "res_high_po").unwrap() };
        assert_eq!(overlay.sheet_ohm("rpoly"), Some(high));
    }

    /// sky130 `poly` answers with its declared conductor `poly_c`, not the
    /// gate cut or resistor bodies carved from it; a cut's resistance is read
    /// per landing.
    #[test]
    fn poly_answers_with_its_conductor() {
        let sky = load("sky130");
        assert_eq!(Process::sheet_ohm(&sky, "poly"), Some(48.2));
        assert_eq!(sky.cut_ohm("licon", "poly"), Some(152.0), "licon_po");
        assert_eq!(sky.cut_ohm("licon", "tap"), Some(585.0), "licon_pt, the larger of licon_nt 185 and licon_pt 585");
        assert_eq!(sky.pex_f32_named("licon_po", "sheet_res_ohm_sq"), Some(152.0));
    }

    /// Every routing metal and cut keeps a `pex` row (its own or its one
    /// conductor's): the router's R and C come from it.
    #[test]
    fn routing_layers_keep_their_pex_rows() {
        for deck in DECKS {
            let pdk = load(deck);
            for &l in pdk.routing_metals.iter().chain(&pdk.routing_cuts) {
                assert!(pdk.pex_f32(l, "thickness_nm").is_some(), "{deck}: {l:?} has no pex row");
            }
        }
    }

    /// sky130 states metal density as `density_cmp` (max 0.7 in 700 um
    /// windows): read as a maximum.
    #[test]
    fn density_cmp_is_read() {
        let sky = load("sky130");
        assert!(sky.density_rules().contains(&(id(&sky, "met1"), 700_000, 0.7, true)), "{:?}", sky.density_rules());
    }
}
