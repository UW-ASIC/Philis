//! [`Pdk`] — process data, shared by everyone. Plain data, not theory.
//!
//! Philis's own PDK schema. It carries the numbers a generator/checker reads
//! (layers, rule values, grid) *and* the compiled [`gdsverify::Deck`] that the
//! real verification engine consumes — the deck is what `run_drc`/`run_erc`/
//! `run_pex`/LVS actually run against.
//!
//! The frontend builds a `Pdk` once from a deck JSON (via [`Pdk::from_json`],
//! which is gdsverify's own reader) and hands it to cells/stages/verify. This
//! is the single public PDK type the whole pipeline passes around.

use pnr_core::{LayerId, Process};

/// Process design kit: layers, design-rule values, grid, and the gdsverify deck.
pub struct Pdk {
    /// Named layers → their Philis [`LayerId`]. The index (`LayerId.0`) is the
    /// position in this table; the name is what gdsverify's deck keys on.
    pub layers: Vec<(String, LayerId)>,
    /// Design-rule values by name (min spacing, min width…), in `nm`.
    pub rules: Vec<(String, i32)>,
    /// Fabrication grid, `nm`. All geometry snaps to this.
    pub grid: i32,
    /// The compiled verification deck — layer table, DRC rules, PEX models, LVS
    /// tolerances. This is the object `gdsverify::run_*` consume; verify builds
    /// its `GeometryStore` against `deck.layers` before every check.
    pub deck: gdsverify::Deck,
    /// Layer **role** → deck layer name, from the deck's `cell.layers` section.
    ///
    /// Generators ask for roles (`"tap"`, `"poly"`), not physical layers, and a
    /// deck is free to map a role onto whatever layer implements it — sky130 has
    /// no distinct `tap` layer, so its deck maps `"tap" -> "diff"`. gdsverify's
    /// `Deck` has no field for this section and silently drops it, which is why
    /// it is re-read here rather than taken from `deck`.
    pub roles: Vec<(String, String)>,
    /// The routable metal stack, bottom-up, from `cell.layers.routing_metals`.
    pub routing_metals: Vec<LayerId>,
    /// The cut layers joining consecutive `routing_metals`, from
    /// `cell.layers.routing_vias`. Always `routing_metals.len() - 1` long.
    pub routing_cuts: Vec<LayerId>,
}

impl Pdk {
    /// Read a PDK from a deck JSON string and **validate that it is complete**.
    ///
    /// This is gdsverify's reader for the verification half, plus a re-read of
    /// the Philis-only `cell` section (layer roles + routing stack) that
    /// gdsverify discards, plus [`Pdk::validate`].
    ///
    /// Loading is the only place that can tell "the PDK does not say" apart from
    /// "the PDK says zero", so it is the only place allowed to reject. Everything
    /// downstream may then assume what it reads is real — no `unwrap_or(default)`
    /// standing in for absent process data.
    ///
    /// # Errors
    /// gdsverify's parse error, or a list of every way the deck is incomplete.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let deck = gdsverify::Deck::from_json(text)?;
        let roles = parse_roles(text)?;
        let pdk = Self::from_deck_and_roles(deck, roles)?;
        pdk.validate()?;
        Ok(pdk)
    }

    /// Mirror a compiled [`gdsverify::Deck`] into a `Pdk` with an explicit role
    /// map and routing stack.
    ///
    /// # Errors
    /// If `cell.layers` names a layer the deck's layer table does not contain, or
    /// omits the routing stack.
    pub fn from_deck_and_roles(
        deck: gdsverify::Deck,
        roles: Roles,
    ) -> Result<Self, String> {
        // Deck layer ids are gdsverify's u16 = position in `id_to_name`; reuse
        // them directly so a Philis Shape's LayerId round-trips to the exact
        // deck layer.
        let layers: Vec<(String, LayerId)> = deck
            .layers
            .id_to_name
            .iter()
            .enumerate()
            .map(|(id, name)| (name.clone(), LayerId(id as u16)))
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
        // Three sources, narrowest last so it wins: the deck's DRC ids, then
        // `<layer>_min_width` / `<layer>_min_spacing` synthesised from those same
        // rules (the spelling generators use), then the explicit `cell.*`
        // dimensions.
        let mut rules = scalar_rules(&deck);
        for (name, id) in &layers {
            if let Some(w) = min_width_of(&deck, id.0) {
                rules.push((format!("{name}_min_width"), w));
            }
            if let Some(s) = min_spacing_of(&deck, id.0) {
                rules.push((format!("{name}_min_spacing"), s));
            }
        }
        rules.extend(roles.scalars);
        let grid = deck_grid(&deck);
        Ok(Self {
            layers,
            rules,
            grid,
            deck,
            roles: roles.map,
            routing_metals,
            routing_cuts,
        })
    }

    /// Every way this deck could be too incomplete to build legal geometry from,
    /// reported at once rather than one reload at a time.
    ///
    /// Each check exists because its absence previously produced *silent* wrong
    /// geometry, not an error: an unresolvable role fell back to `LayerId(0)`
    /// (which is `nwell`, so mandatory layers were drawn on the well), and a
    /// missing routing stack let the router index the raw deck table.
    ///
    /// # Errors
    /// A newline-separated list of every problem found.
    pub fn validate(&self) -> Result<(), String> {
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
        // continuous and `lvs_cut_required` leaves isolated islands.
        for (i, cut) in self.routing_cuts.iter().enumerate() {
            let (Some(a), Some(b)) = (self.routing_metals.get(i), self.routing_metals.get(i + 1))
            else {
                continue;
            };
            let joins = self
                .deck
                .connectivity
                .vias
                .iter()
                .any(|(c, js)| *c == cut.0 && js.contains(&a.0) && js.contains(&b.0));
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

    /// gdsverify layer id for a Philis [`LayerId`] — the identity map, since
    /// `from_deck` seeds `LayerId` from the deck's own ids. Returns the raw
    /// `u16` gdsverify uses in a `GeometryStore`.
    #[must_use]
    pub fn gv_layer(&self, layer: LayerId) -> gdsverify::LayerId {
        layer.0
    }

    /// gdsverify layer id for a named layer, or `None` if the deck lacks it.
    #[must_use]
    pub fn gv_layer_by_name(&self, name: &str) -> Option<gdsverify::LayerId> {
        self.deck.layers.id(name)
    }

    /// The full set of PDK [`LayerId`]s, in deck order — every layer in the deck,
    /// masks included. This is **not** a routing stack; see [`Pdk::routing_layers`].
    #[must_use]
    pub fn layers(&self) -> Vec<LayerId> {
        self.layers.iter().map(|(_, id)| *id).collect()
    }

    /// The routable stack, bottom-up, as the deck declares it in
    /// `cell.layers.routing_metals`. `gr`/`dr` index this slice by their internal
    /// layer index, so its contents decide what metal a wire lands on.
    ///
    /// Handing them [`Pdk::layers`] instead put wires on the first entries of the
    /// deck table — `nwell`, `diff`, `rpoly` — which produced ~330 of the OTA's
    /// 341 DRC violations (`min_width:nwell` measured 290 nm against an 840 nm
    /// limit: a wire width, not a well) and fabricated gate-over-diff crossings
    /// that broke LVS extraction.
    ///
    /// Read, not inferred: the deck is the one thing that knows which of its
    /// conductors are meant to carry routing. [`Pdk::validate`] cross-checks the
    /// declaration against `connectivity`/`device_recognition` at load, so a
    /// typo here fails immediately instead of becoming DRC noise.
    #[must_use]
    pub fn routing_layers(&self) -> Vec<LayerId> {
        self.routing_metals.clone()
    }

    /// The cut (via) layer joining each adjacent pair of [`Pdk::routing_layers`]
    /// **and its exact drawn size in nm**, so `routing_vias()[i]` connects
    /// `routing_layers()[i]` to `[i + 1]`. Length is one less than the stack — or
    /// empty if the deck cannot name every cut, since a partial table would
    /// silently mislabel every layer above the gap.
    ///
    /// The deck sets `lvs_cut_required`: two conductors are connected **only**
    /// where an explicit cut shape exists. A router that emits no cuts therefore
    /// produces electrically isolated per-layer islands, which reads downstream as
    /// unconnected pins, floating gates and a wrong LVS device count — never as
    /// anything that looks like a via problem.
    ///
    /// The size matters as much as the layer: a cut is a **fixed-size** contact,
    /// not a wire. sky130 pairs `MCON.1 min_width 170` with `MCON.1.EXACT
    /// max_width 170`, so drawing a cut at the wire width violates `max_width`
    /// *and* eats the metal enclosure the cut needs on both sides.
    ///
    /// Each entry is `(cut layer, cut size, pad below, pad above)`. The pads are
    /// the squares of metal the cut needs on the layers under and over it. A
    /// router that changes two layers at once emits no wire on the layer it passes
    /// through, leaving its cuts with literally zero enclosing metal, so the pad
    /// has to be drawn explicitly rather than assumed from the wire.
    ///
    /// A pad is sized per metal layer, because a square big enough to enclose the
    /// cut can still be an illegal piece of that metal on its own: it must also
    /// clear the layer's `min_width` and `min_area`. `met3` is wider than `met1`,
    /// so one global pad size cannot satisfy both.
    ///
    /// # Panics
    /// Never in practice: [`Pdk::validate`] rejects at load any deck whose cuts
    /// lack a `min_width`, so by the time this runs the data is known present.
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
            let enclosed = size + 2 * self.enclosure_of(cut);
            cuts.push((
                LayerId(cut),
                size,
                self.pad_for(a, enclosed),
                self.pad_for(b, enclosed),
            ));
        }
        cuts
    }

    /// Smallest square that both encloses the cut (`enclosed`) and is a legal
    /// standalone piece of `metal` — clearing that layer's `min_width` and, since
    /// a lone pad may be the only metal there, its `min_area`.
    ///
    /// Rounded up to the fabrication grid: `min_area` rarely has an integral
    /// square root (sky130 met1 wants 83000 nm², so 289 nm), and a pad off the
    /// 5 nm grid is an `off_grid` violation on every edge it has.
    fn pad_for(&self, metal: gdsverify::LayerId, enclosed: i32) -> i32 {
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

    /// The deck's `min_area` for a layer, if it declares one.
    #[must_use]
    pub fn min_area(&self, layer: gdsverify::LayerId) -> Option<i64> {
        self.deck.drc_rules.iter().find_map(|r| match r {
            gdsverify::DrcRuleParam::MinArea { layer: l, min, .. } if *l == layer => Some(*min),
            _ => None,
        })
    }

    /// The tightest track pitch that keeps a `wire_width` wire legal on **every**
    /// routing layer: `wire_width + max(min_spacing)` over the stack.
    ///
    /// The track lattice has one global pitch, so it must satisfy the worst layer
    /// or that layer is illegal by construction — sky130 `li` wants 170 nm of
    /// spacing where `met1` wants 140, so a pitch sized for met1 makes every pair
    /// of adjacent li tracks a violation no amount of rerouting can fix.
    ///
    /// ponytail: one pitch for the whole stack, so the upper layers are routed
    /// more coarsely than they need. Per-layer pitch means a per-layer track
    /// lattice in `gr::TrackGrid`; do that if upper-layer density ever matters.
    #[must_use]
    pub fn routing_pitch(&self, wire_width: i32, stack: &[gdsverify::LayerId]) -> i32 {
        // Over the layers actually routed on, NOT every conductor the deck
        // declares. Scanning the whole deck let sky130's met5 (1600 nm spacing)
        // set the pitch for a router that only reaches li/met1/met2: 1890 nm
        // instead of 460, which is 4x coarser than li needs and far coarser than a
        // 170 nm pin, so no track node ever landed on a pin.
        let worst = stack.iter().filter_map(|&l| self.min_spacing(l)).max().unwrap_or(0);
        wire_width + worst
    }

    /// The deck's `min_spacing` for a layer, if it declares one.
    #[must_use]
    pub fn min_spacing(&self, layer: gdsverify::LayerId) -> Option<i32> {
        self.deck.drc_rules.iter().find_map(|r| match r {
            gdsverify::DrcRuleParam::MinSpacing { layer: l, min, .. } if *l == layer => Some(*min),
            _ => None,
        })
    }

    /// The deck's `min_width` for a layer, if it declares one.
    #[must_use]
    pub fn min_width(&self, layer: gdsverify::LayerId) -> Option<i32> {
        self.deck.drc_rules.iter().find_map(|r| match r {
            gdsverify::DrcRuleParam::MinWidth { layer: l, min, .. } if *l == layer => Some(*min),
            _ => None,
        })
    }

    /// Largest enclosure any layer must give `inner` — the binding one, since the
    /// same pad is drawn above and below the cut.
    #[must_use]
    fn enclosure_of(&self, inner: gdsverify::LayerId) -> i32 {
        self.deck
            .drc_rules
            .iter()
            .filter_map(|r| match r {
                gdsverify::DrcRuleParam::MinEnclosure { inner: i, min, .. } if *i == inner => {
                    Some(*min)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0)
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
    /// a layer is what the process provides (sky130 has no `tap`, so its deck
    /// maps the role onto `diff`). Before this map was parsed, `layer("tap")`
    /// returned `None` and every caller was an `if let Some(..)` that silently
    /// drew nothing — so no well or body tap existed in any cell.
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

/// Pull the scalar DRC minima (min_width / min_spacing / …) out of the deck into
/// `(rule_id, value_nm)` pairs for the generator-facing `rules` view.
fn scalar_rules(deck: &gdsverify::Deck) -> Vec<(String, i32)> {
    use gdsverify::DrcRuleParam as P;
    deck.drc_rules
        .iter()
        .filter_map(|r| {
            let v = match r {
                P::MinWidth { min, .. }
                | P::MinSpacing { min, .. }
                | P::MinEnclosure { min, .. }
                | P::MinExtension { min, .. }
                | P::Notch { min, .. }
                | P::MinEdgeLength { min, .. }
                | P::CornerToCorner { min, .. }
                | P::Overlap { min, .. }
                | P::MinSpacingDiff { min, .. } => *min,
                P::MaxWidth { max, .. } => *max,
                // Density/antenna/area/multi-patterning etc. aren't a single
                // scalar minimum the generators query — skipped from this view.
                _ => return None,
            };
            Some((r.id().to_string(), v))
        })
        .collect()
}

/// Layer roles every cell generator resolves unconditionally (`cells::req`).
/// A deck missing any of these cannot produce a correct device, so loading it
/// fails rather than letting the generator skip the geometry.
pub const REQUIRED_ROLES: &[&str] = &[
    "diff", "poly", "li", "licon", "mcon", "met1", "nwell", "nsdm", "psdm", "tap",
];

/// Every construction dimension a cell generator reads (`cells::rule`).
///
/// These all take a `default` at the call site, which made a missing one
/// invisible: the generator built to a number compiled into Philis rather than
/// one describing the target process, and the layout looked plausible. Requiring
/// them here turns "this deck does not describe its own devices" into a load
/// error. Keep in sync with the `rule(process, "…")` call sites.
pub const REQUIRED_RULES: &[&str] = &[
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

/// The deck's `cell` section: layer roles, the routing stack, and the scalar
/// construction dimensions the generators build geometry from.
#[derive(Default)]
pub struct Roles {
    /// role → deck layer name.
    pub map: Vec<(String, String)>,
    /// Routable metals, bottom-up.
    pub routing_metals: Vec<String>,
    /// Cuts joining consecutive `routing_metals`.
    pub routing_vias: Vec<String>,
    /// Named integer dimensions from `cell.*` (`contact`, `sd_width`, …).
    pub scalars: Vec<(String, i32)>,
}

/// Read the `cell.layers` section gdsverify's `Deck` drops.
///
/// `routing_metals` / `routing_vias` are lists, every other key is a role→layer
/// string pair; they share one object in the deck format.
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
    // `cell.*` siblings of `layers` are the construction dimensions (contact
    // size, S/D width, poly endcap, …). These were declared by every deck and
    // read by none: `scalar_rules` only walked `drc_rules`, whose keys are rule
    // *ids* (`MCON.1`), so `rule(process, "contact", 170)` never matched and
    // every generator silently built to its hardcoded fallback instead of the
    // process it was pointed at.
    for (k, val) in cell {
        if k == "layers" {
            continue;
        }
        // Flags are written as JSON booleans (`retrograde_pwell: false`) but read
        // through the same integer `rule()` seam, so fold them to 0/1.
        let n = val.as_i64().or_else(|| val.as_bool().map(i64::from));
        if let Some(n) = n {
            roles.scalars.push((k.clone(), n as i32));
        }
    }
    Ok(roles)
}

/// `min_width` for a layer, straight off the deck's rule list.
fn min_width_of(deck: &gdsverify::Deck, layer: gdsverify::LayerId) -> Option<i32> {
    deck.drc_rules.iter().find_map(|r| match r {
        gdsverify::DrcRuleParam::MinWidth { layer: l, min, .. } if *l == layer => Some(*min),
        _ => None,
    })
}

/// `min_spacing` for a layer, straight off the deck's rule list.
fn min_spacing_of(deck: &gdsverify::Deck, layer: gdsverify::LayerId) -> Option<i32> {
    deck.drc_rules.iter().find_map(|r| match r {
        gdsverify::DrcRuleParam::MinSpacing { layer: l, min, .. } if *l == layer => Some(*min),
        _ => None,
    })
}

/// Manufacturing grid from the deck's `off_grid` rule, else `1`.
fn deck_grid(deck: &gdsverify::Deck) -> i32 {
    deck.drc_rules
        .iter()
        .find_map(|r| match r {
            gdsverify::DrcRuleParam::OffGrid { grid, .. } => Some(*grid),
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
