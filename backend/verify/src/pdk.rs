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
    /// The deck JSON; each [`crate::Checker`] re-parses it into its own table.
    pub source: String,
    /// Layer role → deck layer name, from `cell.layers` (gdsverify ignores
    /// the `cell` section).
    pub roles: Vec<(String, String)>,
    /// The routable metal stack, bottom-up.
    pub routing_metals: Vec<LayerId>,
    /// The cuts joining consecutive `routing_metals` (one fewer).
    pub routing_cuts: Vec<LayerId>,
}

impl Pdk {
    /// Read a deck JSON and validate that it is complete, so downstream code
    /// may assume every value it reads is real process data.
    ///
    /// # Errors
    /// gdsverify's parse error, or a list of every way the deck is incomplete.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut strings = StrTable::default();
        let deck = gdsverify::ingest::deck::parse_deck(text, nm_grid(), &mut strings)
            .map_err(|e| format!("deck rejected: {e}"))?;
        let roles = parse_roles(text)?;

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
        for name in REQUIRED_RULES {
            if !self.rules.iter().any(|(n, _)| n == name) {
                bad.push(format!(
                    "no value for construction dimension {name:?}: a generator would fall \
                     back to a number compiled into Philis instead of this process's \
                     (add it to the deck's cell section)"
                ));
            }
        }

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
            let joins = conn.via_cut.iter().zip(&conn.via_connects).any(|(c, &(x, y))| {
                c.0 == cut.0
                    && ((x.0 == a.0 && y.0 == b.0) || (x.0 == b.0 && y.0 == a.0))
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

    /// `table[LayerId.0] = (gds_layer, gds_datatype)` for the GDS writer;
    /// `(0, 0)` for a layer the deck does not number. Reads both
    /// `[layer, datatype]` and `{"layer", "datatype"}` spellings — knowing only
    /// one once emitted every shape on `(0, 0)`.
    #[must_use]
    pub fn layer_gds(&self) -> Vec<(u16, u16)> {
        let max_id = self.layers.iter().map(|(_, id)| id.0 as usize).max().unwrap_or(0);
        let mut table = vec![(0u16, 0u16); max_id + 1];

        let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.source) else {
            return table;
        };
        let Some(layers) = v.get("layers").and_then(|l| l.as_object()) else {
            return table;
        };

        for (name, id) in &self.layers {
            let Some(val) = layers.get(name) else { continue };
            let pair = match (val.as_array(), val.get("layer"), val.get("datatype")) {
                (Some(a), ..) if a.len() >= 2 => a[0].as_i64().zip(a[1].as_i64()),
                (_, Some(l), Some(d)) => l.as_i64().zip(d.as_i64()),
                _ => None,
            };
            if let Some((l, d)) = pair {
                table[id.0 as usize] = (l as u16, d as u16);
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

    /// The deck's `min_area` for a layer in nm² (the deck states the side of
    /// the square; squared here).
    #[must_use]
    pub fn min_area(&self, layer: u16) -> Option<i64> {
        self.rule_limit_nm("min_area", layer).map(|side| side * side).map(|a| a as i64)
    }

    /// `wire_width + max(min_spacing)` over `stack`: the one lattice pitch
    /// legal on every layer actually routed on.
    ///
    /// ponytail: one pitch for the whole stack, so the upper layers are routed
    /// more coarsely than they need. Per-layer pitch means a per-layer track
    /// lattice in `gr::TrackGrid`; do that if upper-layer density ever matters.
    #[must_use]
    pub fn routing_pitch(&self, wire_width: i32, stack: &[u16]) -> i32 {
        let worst = stack.iter().filter_map(|&l| self.min_spacing(l)).max().unwrap_or(0);
        wire_width + worst
    }

    /// `(width threshold, spacing)` of every `wide_dependent_spacing` rule on
    /// `layer`, nm, ascending by threshold: a shape at least that wide needs
    /// that spacing (the binding one is the largest threshold it reaches).
    #[must_use]
    pub fn wide_spacing(&self, layer: u16) -> Vec<(i32, i32)> {
        let (Some(kind), Some(thr), Some(lim)) = (
            self.strings.get("wide_dependent_spacing"),
            self.strings.get("width_threshold"),
            self.strings.get("limit"),
        ) else {
            return Vec::new();
        };
        let len = |s, p| match self.deck.rules.param(s, p) {
            Some(ParamValue::Length(d)) => Some(d.raw() as i32),
            _ => None,
        };
        let mut steps: Vec<(i32, i32)> = self
            .deck
            .rules
            .spec
            .iter()
            .filter(|s| s.kind == kind && self.deck.rules.layers_of(s).first().map(|l| l.0) == Some(layer))
            .filter_map(|s| Some((len(s, thr)?, len(s, lim)?)))
            .collect();
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

    /// The deck's `min_spacing` for a layer in nm, if it declares one.
    #[must_use]
    pub fn min_spacing(&self, layer: u16) -> Option<i32> {
        self.rule_limit_nm("min_spacing", layer).map(|v| v as i32)
    }

    /// The deck's `min_width` for a layer in nm, if it declares one.
    #[must_use]
    pub fn min_width(&self, layer: u16) -> Option<i32> {
        self.rule_limit_nm("min_width", layer).map(|v| v as i32)
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
    /// Resolve a role through the deck's `cell.layers` map first, then fall back
    /// to a layer of that literal name.
    ///
    /// The indirection is the point: a role is what a generator wants (`"tap"`),
    /// a layer is what the process provides.
    fn layer(&self, role: &str) -> Option<LayerId> {
        let name = self
            .roles
            .iter()
            .find(|(r, _)| r == role)
            .map_or(role, |(_, l)| l.as_str());
        self.layers.iter().find(|(n, _)| n == name).map(|(_, id)| *id)
    }
    /// Last match wins: `rules` is built widest-source-first, so an explicit
    /// `cell.*` dimension overrides a value synthesised from a DRC rule.
    fn rule(&self, name: &str, default: i32) -> i32 {
        self.rules
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map_or(default, |(_, v)| *v)
    }
    fn grid(&self) -> i32 {
        self.grid
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

/// Every construction dimension a cell generator reads (`cells::rule`). Each
/// call site takes a default, so a missing one would silently build to a
/// number compiled into Philis. Keep in sync with those call sites.
const REQUIRED_RULES: &[&str] = &[
    "bjt_base_frac_permille",
    "bjt_collector_frac_permille",
    "bjt_max_emitter_stripe",
    "bjt_min_emitter_side",
    "bjt_stripe_gap",
    "cap_unit_side",
    "contact",
    "device_gap",
    "diode_gap",
    "diode_l",
    "diode_w",
    "guard_licon_pitch",
    "ind_min_diameter",
    "ind_min_trace",
    "lod_moat_ext_moderate",
    "m1_enc",
    "max_finger_width",
    "mcon_size",
    "met1_space",
    "min_finger_width",
    "min_gate_l",
    "min_guard_ring_width",
    "mom_finger_space",
    "mom_finger_width",
    "n_well_depth",
    "nwell_diff_enc",
    "nwell_min_width",
    "p_epi_thickness",
    "p_well_depth",
    "plate_spacing",
    "poly_ext",
    "poly_min_width",
    "res_head",
    "res_min_segment",
    "res_min_width",
    "res_seg_gap",
    "retrograde_pwell",
    "sd_width",
    "tie_max_dist_nm",
    "via_enclosure",
    "via_spacing",
    "well_enclosure",
    "wpe_clearance_moderate",
];

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

    /// A router indexes `routing_layers()` by its internal layer index, so the
    /// slice must contain metal only. If a device-formation or mask layer leaks
    /// in, wires get drawn on it — that is how ~330 nwell/diff DRC violations and
    /// a broken LVS extraction happened.
    #[test]
    fn routing_stack_is_metal_only() {
        for pdk_file in ["sky130.json", "generic_finfet.json"] {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/");
            let text = std::fs::read_to_string(format!("{path}{pdk_file}")).unwrap();
            let pdk = Pdk::from_json(&text).unwrap();
            let names: Vec<&str> = pdk
                .routing_layers()
                .iter()
                .map(|id| {
                    pdk.layers
                        .iter()
                        .find(|(_, l)| l == id)
                        .map_or("?", |(n, _)| n.as_str())
                })
                .collect();
            assert!(!names.is_empty(), "{pdk_file}: empty routing stack");
            for n in &names {
                assert!(
                    n.starts_with("met") || *n == "li",
                    "{pdk_file}: {n} is not routable metal (stack {names:?})"
                );
            }
        }
    }

    /// The via stack has to line up with the metal stack one-for-one, and each cut
    /// has to be drawable: a cut no wider than the pads that must enclose it, on a
    /// layer that is not itself routing metal, snapped to the fab grid.
    ///
    /// These are the same conditions `library::run` asserts before routing; having
    /// them here means a PDK edit that breaks them fails in `cargo test` rather
    /// than 200 iterations into a flow.
    #[test]
    fn via_stack_matches_metal_stack() {
        for pdk_file in ["sky130.json", "generic_finfet.json"] {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/");
            let text = std::fs::read_to_string(format!("{path}{pdk_file}")).unwrap();
            let pdk = Pdk::from_json(&text).unwrap();
            let layers = pdk.routing_layers();
            let cuts = pdk.routing_vias();
            assert_eq!(
                cuts.len(),
                layers.len() - 1,
                "{pdk_file}: {} metal layers need {} cuts, deck names {}",
                layers.len(),
                layers.len() - 1,
                cuts.len()
            );
            for &(cut, size, below, above) in &cuts {
                assert!(size > 0, "{pdk_file}: cut {cut:?} has no size");
                assert!(
                    size <= below && size <= above,
                    "{pdk_file}: cut {cut:?} {size} nm exceeds its pads {below}/{above}"
                );
                assert!(
                    !layers.contains(&cut),
                    "{pdk_file}: {cut:?} is both routing metal and a via cut"
                );
                for pad in [below, above] {
                    assert_eq!(
                        pad % pdk.grid,
                        0,
                        "{pdk_file}: pad {pad} for cut {cut:?} is off the {} nm grid",
                        pdk.grid
                    );
                }
            }
        }
    }

    /// `routing_pitch` must leave at least `min_spacing` between adjacent tracks on
    /// **every layer of the stack it is given** — the lattice has one global pitch,
    /// so the worst layer *in that stack* sets it. Sized for met1 alone, every pair
    /// of adjacent li tracks is a violation nothing downstream can fix.
    #[test]
    fn routing_pitch_clears_every_layer_of_its_stack() {
        for pdk_file in ["sky130.json", "generic_finfet.json"] {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/");
            let text = std::fs::read_to_string(format!("{path}{pdk_file}")).unwrap();
            let pdk = Pdk::from_json(&text).unwrap();
            let w = 290;
            // The stack the flow actually builds: layers whose min_width fits `w`.
            let stack: Vec<_> = pdk
                .routing_layers()
                .into_iter()
                .take_while(|l| pdk.min_width(l.0).is_none_or(|mw| mw <= w))
                .map(|l| l.0)
                .collect();
            let pitch = pdk.routing_pitch(w, &stack);
            for &l in &stack {
                let need = pdk.min_spacing(l).unwrap_or(0);
                assert!(
                    pitch - w >= need,
                    "{pdk_file}: pitch {pitch} leaves {} nm between {w} nm wires, \
                     layer {l:?} needs {need}",
                    pitch - w
                );
            }
        }
    }

    /// A layer the router cannot reach must not set the pitch for the layers it
    /// can. sky130's met5 wants 1600 nm spacing; li wants 170. Counting met5 gave
    /// a 1890 nm lattice on a 170 nm pin, and pin access became impossible.
    #[test]
    fn an_unreachable_layer_does_not_set_the_pitch() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
        let pdk = Pdk::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
        let all: Vec<_> = pdk.routing_layers().into_iter().map(|l| l.0).collect();
        let reachable: Vec<_> = pdk
            .routing_layers()
            .into_iter()
            .take_while(|l| pdk.min_width(l.0).is_none_or(|mw| mw <= 290))
            .map(|l| l.0)
            .collect();
        assert!(
            pdk.routing_pitch(290, &reachable) < pdk.routing_pitch(290, &all),
            "the reachable stack must give a tighter pitch than the whole deck"
        );
    }
}
