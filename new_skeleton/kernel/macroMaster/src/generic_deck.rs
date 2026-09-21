//! The **Generic PDK**, hardcoded -- deliberately *not* a reference to any file
//! in `pdks/`, so a Device's validation yardstick travels with the crate.
//!
//! It is the **super-strict exhaustive envelope** of the target PDKs
//! (`pdks/sky130.json` + `pdks/generic_finfet.json`): for every shared
//! scalar-bounded DRC rule it keeps the *hardest* value (`min_*`/`notch`/
//! `overlap`/enclosure/extension -> MAX over sources, `max_width` -> MIN,
//! `angle.allowed` -> intersection, `off_grid` pitch -> LCM), and it keeps only
//! the **universal** layers (the intersection across both PDKs -- sky130-only
//! layers like hvtp/lvtn/rpm/npn/pnp are dropped, and any rule that referenced
//! them with it). Passing DRC here therefore *guarantees* passing every real
//! target PDK: the deck is an upper bound on strictness, so a clean Device is
//! portable by construction.
//!
//! Its job is to *bite*: a Device drawn with under-sized gate geometry fails
//! here (poly min-width 150nm, diff 150nm, ...), which is exactly what forces
//! authors toward real, portable dimensions.
//!
//! The document is in the gdsverify (GPurify rewrite) deck schema —
//! `layers`/`rules`/`connectivity` consumed by
//! `gdsverify::ingest::deck::parse_deck`, plus the Philis-owned `cell` section
//! (layer roles, routing stack, construction scalars) that gdsverify tolerates
//! and ignores and `verify::Pdk::from_json` re-reads. Kinds outside geometric
//! DRC (antenna/EM/reliability/ERC) are deliberately absent, as they were from
//! the old deck: the Device tier runs geometry checks only. The deck likewise
//! declares no `derived`/`device_recognition`/`pex` sections — it recognises no
//! devices and extracts nothing; those tiers belong to the real target PDKs.
//!
//! Consumed under the `gpurify` feature by [`crate::GenericPdk`]
//! (`verify::Pdk::from_json`), and without it by [`DeckView`] below. The
//! strictness-envelope property is enforced by the `gpurify`-gated tests at the
//! bottom of this file; edit the sources, re-run those, and resolve any failure
//! toward strictness.

/// The hardcoded Generic PDK deck (strict envelope of the target PDKs), as a
/// `gdsverify` deck JSON string.
///
/// Rule ids follow the target decks' `<layer>_<kind>` spelling, with one
/// deliberate exception: met1's width/spacing rules keep the legacy ids
/// `"min_width"` / `"min_spacing"`, because generator call sites query the
/// [`crate::Process::rule`] seam by those names (see `examples/`), and both
/// builds (`DeckView` here, `verify::Pdk::from_json` under `gpurify`) answer
/// `rule()` from the deck's rule ids.
pub const GENERIC_DECK_JSON: &str = r##"{
  "layers": {
    "diff": [65, 20],
    "li": [67, 20],
    "licon": [66, 44],
    "mcon": [67, 44],
    "met1": [68, 20],
    "met2": [69, 20],
    "met3": [70, 20],
    "met4": [71, 20],
    "met5": [72, 20],
    "nsdm": [93, 44],
    "nwell": [64, 20],
    "poly": [66, 20],
    "psdm": [94, 20],
    "rpoly": [66, 13],
    "tap": [65, 44],
    "via1": [68, 44],
    "via2": [69, 44],
    "via3": [70, 44],
    "via4": [71, 44]
  },
  "rules": {
    "nwell_min_width": { "kind": "min_width", "layers": ["nwell"], "params": { "limit": { "nm": 840 } } },
    "diff_min_width": { "kind": "min_width", "layers": ["diff"], "params": { "limit": { "nm": 150 } } },
    "tap_min_width": { "kind": "min_width", "layers": ["tap"], "params": { "limit": { "nm": 150 } } },
    "poly_min_width": { "kind": "min_width", "layers": ["poly"], "params": { "limit": { "nm": 150 } } },
    "rpoly_min_width": { "kind": "min_width", "layers": ["rpoly"], "params": { "limit": { "nm": 150 } } },
    "licon_min_width": { "kind": "min_width", "layers": ["licon"], "params": { "limit": { "nm": 170 } } },
    "li_min_width": { "kind": "min_width", "layers": ["li"], "params": { "limit": { "nm": 170 } } },
    "mcon_min_width": { "kind": "min_width", "layers": ["mcon"], "params": { "limit": { "nm": 170 } } },
    "min_width": { "kind": "min_width", "layers": ["met1"], "params": { "limit": { "nm": 140 } } },
    "via1_min_width": { "kind": "min_width", "layers": ["via1"], "params": { "limit": { "nm": 150 } } },
    "met2_min_width": { "kind": "min_width", "layers": ["met2"], "params": { "limit": { "nm": 140 } } },
    "via2_min_width": { "kind": "min_width", "layers": ["via2"], "params": { "limit": { "nm": 200 } } },
    "met3_min_width": { "kind": "min_width", "layers": ["met3"], "params": { "limit": { "nm": 300 } } },
    "via3_min_width": { "kind": "min_width", "layers": ["via3"], "params": { "limit": { "nm": 200 } } },
    "met4_min_width": { "kind": "min_width", "layers": ["met4"], "params": { "limit": { "nm": 300 } } },
    "via4_min_width": { "kind": "min_width", "layers": ["via4"], "params": { "limit": { "nm": 800 } } },
    "met5_min_width": { "kind": "min_width", "layers": ["met5"], "params": { "limit": { "nm": 1600 } } },
    "nsdm_min_width": { "kind": "min_width", "layers": ["nsdm"], "params": { "limit": { "nm": 380 } } },
    "psdm_min_width": { "kind": "min_width", "layers": ["psdm"], "params": { "limit": { "nm": 380 } } },
    "licon_max_width": { "kind": "max_width", "layers": ["licon"], "params": { "limit": { "nm": 170 } } },
    "mcon_max_width": { "kind": "max_width", "layers": ["mcon"], "params": { "limit": { "nm": 170 } } },
    "via1_max_width": { "kind": "max_width", "layers": ["via1"], "params": { "limit": { "nm": 150 } } },
    "via2_max_width": { "kind": "max_width", "layers": ["via2"], "params": { "limit": { "nm": 200 } } },
    "via3_max_width": { "kind": "max_width", "layers": ["via3"], "params": { "limit": { "nm": 200 } } },
    "via4_max_width": { "kind": "max_width", "layers": ["via4"], "params": { "limit": { "nm": 800 } } },
    "met5_max_width": { "kind": "max_width", "layers": ["met5"], "params": { "limit": { "nm": 30000 } } },
    "nwell_min_spacing": { "kind": "min_spacing", "layers": ["nwell"], "params": { "limit": { "nm": 1270 } } },
    "diff_min_spacing": { "kind": "min_spacing", "layers": ["diff"], "params": { "limit": { "nm": 270 } } },
    "tap_min_spacing": { "kind": "min_spacing", "layers": ["tap"], "params": { "limit": { "nm": 270 } } },
    "poly_min_spacing": { "kind": "min_spacing", "layers": ["poly"], "params": { "limit": { "nm": 210 } } },
    "rpoly_min_spacing": { "kind": "min_spacing", "layers": ["rpoly"], "params": { "limit": { "nm": 210 } } },
    "licon_min_spacing": { "kind": "min_spacing", "layers": ["licon"], "params": { "limit": { "nm": 170 } } },
    "li_min_spacing": { "kind": "min_spacing", "layers": ["li"], "params": { "limit": { "nm": 170 } } },
    "mcon_min_spacing": { "kind": "min_spacing", "layers": ["mcon"], "params": { "limit": { "nm": 190 } } },
    "min_spacing": { "kind": "min_spacing", "layers": ["met1"], "params": { "limit": { "nm": 140 } } },
    "via1_min_spacing": { "kind": "min_spacing", "layers": ["via1"], "params": { "limit": { "nm": 170 } } },
    "met2_min_spacing": { "kind": "min_spacing", "layers": ["met2"], "params": { "limit": { "nm": 140 } } },
    "via2_min_spacing": { "kind": "min_spacing", "layers": ["via2"], "params": { "limit": { "nm": 200 } } },
    "met3_min_spacing": { "kind": "min_spacing", "layers": ["met3"], "params": { "limit": { "nm": 300 } } },
    "via3_min_spacing": { "kind": "min_spacing", "layers": ["via3"], "params": { "limit": { "nm": 200 } } },
    "met4_min_spacing": { "kind": "min_spacing", "layers": ["met4"], "params": { "limit": { "nm": 300 } } },
    "via4_min_spacing": { "kind": "min_spacing", "layers": ["via4"], "params": { "limit": { "nm": 800 } } },
    "met5_min_spacing": { "kind": "min_spacing", "layers": ["met5"], "params": { "limit": { "nm": 1600 } } },
    "nsdm_min_spacing": { "kind": "min_spacing", "layers": ["nsdm"], "params": { "limit": { "nm": 380 } } },
    "psdm_min_spacing": { "kind": "min_spacing", "layers": ["psdm"], "params": { "limit": { "nm": 380 } } },
    "nwell_notch": { "kind": "notch", "layers": ["nwell"], "params": { "limit": { "nm": 1270 } } },
    "diff_notch": { "kind": "notch", "layers": ["diff"], "params": { "limit": { "nm": 270 } } },
    "tap_notch": { "kind": "notch", "layers": ["tap"], "params": { "limit": { "nm": 270 } } },
    "poly_notch": { "kind": "notch", "layers": ["poly"], "params": { "limit": { "nm": 210 } } },
    "li_notch": { "kind": "notch", "layers": ["li"], "params": { "limit": { "nm": 170 } } },
    "met1_notch": { "kind": "notch", "layers": ["met1"], "params": { "limit": { "nm": 140 } } },
    "met2_notch": { "kind": "notch", "layers": ["met2"], "params": { "limit": { "nm": 140 } } },
    "met3_notch": { "kind": "notch", "layers": ["met3"], "params": { "limit": { "nm": 300 } } },
    "met4_notch": { "kind": "notch", "layers": ["met4"], "params": { "limit": { "nm": 300 } } },
    "met5_notch": { "kind": "notch", "layers": ["met5"], "params": { "limit": { "nm": 1600 } } },
    "nsdm_notch": { "kind": "notch", "layers": ["nsdm"], "params": { "limit": { "nm": 380 } } },
    "psdm_notch": { "kind": "notch", "layers": ["psdm"], "params": { "limit": { "nm": 380 } } },
    "poly_to_tap_spacing": { "kind": "min_spacing_diff", "layers": ["poly", "tap"], "params": { "limit": { "nm": 55 } } },
    "diff_min_area": { "kind": "min_area", "layers": ["diff"], "params": { "limit": { "nm": 265 } } },
    "li_min_area": { "kind": "min_area", "layers": ["li"], "params": { "limit": { "nm": 236 } } },
    "met1_min_area": { "kind": "min_area", "layers": ["met1"], "params": { "limit": { "nm": 288 } } },
    "met2_min_area": { "kind": "min_area", "layers": ["met2"], "params": { "limit": { "nm": 285 } } },
    "met3_min_area": { "kind": "min_area", "layers": ["met3"], "params": { "limit": { "nm": 489 } } },
    "met4_min_area": { "kind": "min_area", "layers": ["met4"], "params": { "limit": { "nm": 489 } } },
    "met5_min_area": { "kind": "min_area", "layers": ["met5"], "params": { "limit": { "nm": 2000 } } },
    "nsdm_min_area": { "kind": "min_area", "layers": ["nsdm"], "params": { "limit": { "nm": 514 } } },
    "psdm_min_area": { "kind": "min_area", "layers": ["psdm"], "params": { "limit": { "nm": 504 } } },
    "met1_encloses_mcon": { "kind": "min_enclosure", "layers": ["met1", "mcon"], "params": { "limit": { "nm": 30 } } },
    "met1_encloses_via1": { "kind": "min_enclosure", "layers": ["met1", "via1"], "params": { "limit": { "nm": 55 } } },
    "met2_encloses_via1": { "kind": "min_enclosure", "layers": ["met2", "via1"], "params": { "limit": { "nm": 55 } } },
    "met2_encloses_via2": { "kind": "min_enclosure", "layers": ["met2", "via2"], "params": { "limit": { "nm": 40 } } },
    "met3_encloses_via2": { "kind": "min_enclosure", "layers": ["met3", "via2"], "params": { "limit": { "nm": 65 } } },
    "met3_encloses_via3": { "kind": "min_enclosure", "layers": ["met3", "via3"], "params": { "limit": { "nm": 60 } } },
    "met4_encloses_via3": { "kind": "min_enclosure", "layers": ["met4", "via3"], "params": { "limit": { "nm": 65 } } },
    "met4_encloses_via4": { "kind": "min_enclosure", "layers": ["met4", "via4"], "params": { "limit": { "nm": 190 } } },
    "met5_encloses_via4": { "kind": "min_enclosure", "layers": ["met5", "via4"], "params": { "limit": { "nm": 310 } } },
    "li_encloses_licon": { "kind": "min_enclosure", "layers": ["li", "licon"], "params": { "limit": { "nm": 80 } } },
    "poly_endcap_over_diff": { "kind": "min_extension", "layers": ["poly", "diff"], "params": { "limit": { "nm": 130 } } },
    "diff_overhang_of_poly": { "kind": "min_extension", "layers": ["diff", "poly"], "params": { "limit": { "nm": 250 } } },
    "li_covers_licon": { "kind": "overlap", "layers": ["li", "licon"], "params": { "limit": { "nm": 170 } } },
    "li_covers_mcon": { "kind": "overlap", "layers": ["li", "mcon"], "params": { "limit": { "nm": 170 } } },
    "met1_corner_to_corner": { "kind": "corner_to_corner", "layers": ["met1"], "params": { "limit": { "nm": 140 } } },
    "met1_min_density": { "kind": "density", "layers": ["met1"], "params": { "window": { "nm": 700000 }, "step": { "nm": 70000 }, "limit": { "ratio": 0.35 }, "maximum": false } },
    "met1_max_density": { "kind": "density", "layers": ["met1"], "params": { "window": { "nm": 700000 }, "step": { "nm": 70000 }, "limit": { "ratio": 0.7 }, "maximum": true } },
    "manufacturing_grid": { "kind": "off_grid", "layers": [], "params": { "pitch": { "nm": 5 } } },
    "allowed_angles": { "kind": "angle", "layers": [], "params": { "angle": { "count": 0 }, "angle": { "count": 90 } } }
  },
  "connectivity": {
    "conductors": ["diff", "poly", "li", "met1", "met2", "met3", "met4", "met5", "tap", "rpoly"],
    "intra_layer_touch": true,
    "vias": [
      { "layer": "licon", "connects": ["diff", "li"] },
      { "layer": "licon", "connects": ["poly", "li"] },
      { "layer": "licon", "connects": ["tap", "li"] },
      { "layer": "licon", "connects": ["rpoly", "li"] },
      { "layer": "mcon", "connects": ["li", "met1"] },
      { "layer": "via1", "connects": ["met1", "met2"] },
      { "layer": "via2", "connects": ["met2", "met3"] },
      { "layer": "via3", "connects": ["met3", "met4"] },
      { "layer": "via4", "connects": ["met4", "met5"] }
    ],
    "labels": [
      { "layer": "li", "names": "li" },
      { "layer": "met1", "names": "met1" },
      { "layer": "met2", "names": "met2" },
      { "layer": "met3", "names": "met3" },
      { "layer": "met4", "names": "met4" },
      { "layer": "met5", "names": "met5" }
    ]
  },
  "cell": {
    "layers": {
      "diff": "diff",
      "poly": "poly",
      "rpoly": "rpoly",
      "li": "li",
      "licon": "licon",
      "mcon": "mcon",
      "met1": "met1",
      "nwell": "nwell",
      "nsdm": "nsdm",
      "psdm": "psdm",
      "tap": "tap",
      "routing_metals": ["li", "met1", "met2", "met3", "met4", "met5"],
      "routing_vias": ["mcon", "via1", "via2", "via3", "via4"]
    },
    "contact": 170,
    "sd_width": 250,
    "min_finger_width": 420,
    "max_finger_width": 10000,
    "poly_ext": 130,
    "via_enclosure": 300,
    "plate_spacing": 200,
    "res_head": 300,
    "well_enclosure": 300,
    "via_spacing": 170,
    "mcon_size": 170,
    "m1_enc": 65,
    "met1_space": 140,
    "device_gap": 600,
    "well_spacing": 1270,
    "res_min_segment": 10000,
    "res_seg_gap": 400,
    "bjt_max_emitter_stripe": 25000,
    "bjt_min_emitter_side": 420,
    "bjt_base_frac_permille": 300,
    "bjt_collector_frac_permille": 500,
    "bjt_stripe_gap": 200,
    "diode_gap": 270,
    "diode_l": 1000,
    "diode_w": 500,
    "ind_min_trace": 1000,
    "ind_min_diameter": 10000,
    "nwell_diff_enc": 180,
    "min_guard_ring_width": 420,
    "guard_licon_pitch": 360,
    "licon_poly_enc": 50,
    "p_epi_thickness": 3000,
    "p_well_depth": 1500,
    "n_well_depth": 2000,
    "retrograde_pwell": false,
    "cap_unit_side": 2000,
    "lod_moat_ext_moderate": 3000,
    "min_gate_l": 150,
    "mom_finger_space": 200,
    "mom_finger_width": 200,
    "nwell_min_width": 840,
    "poly_min_width": 150,
    "res_min_width": 500,
    "tie_max_dist_nm": 3000,
    "wpe_clearance_moderate": 3000
  }
}"##;

/// A read-only view of a deck JSON, for the build that does **not** link
/// `verify`.
///
/// [`crate::GenericPdk`] needs exactly three things from a deck — resolve a role
/// to a layer, look up a scalar rule, and know the grid. Under the `gpurify`
/// feature `verify::Pdk` supplies them; without it this does, from the same
/// document, so the two builds cannot disagree about what the process is.
///
/// Layer ids are positions in the deck's own (sorted) layer table. They need not
/// match the ids gdsverify assigns — no engine runs on this path — only be
/// self-consistent, which is what a generator's `Shape`s require.
#[cfg(not(feature = "gpurify"))]
pub struct DeckView {
    layers: Vec<String>,
    /// role → layer name, from `cell.layers`.
    roles: Vec<(String, String)>,
    /// `<rule id>` → value, for the scalar minima generators query.
    rules: Vec<(String, i32)>,
    grid: i32,
}

#[cfg(not(feature = "gpurify"))]
impl DeckView {
    /// Parse the deck. Panics rather than returning an error: the only caller
    /// passes [`GENERIC_DECK_JSON`], a compile-time constant in this crate, so a
    /// failure is a build-time bug and never depends on user input.
    ///
    /// # Panics
    /// If the embedded deck is not valid JSON or omits `layers` / `cell.layers`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let v: serde_json::Value =
            serde_json::from_str(text).expect("embedded generic deck is valid JSON");
        let layers: Vec<String> = v["layers"]
            .as_object()
            .expect("generic deck has a `layers` table")
            .keys()
            .cloned()
            .collect();
        let cell = v["cell"]["layers"]
            .as_object()
            .expect("generic deck has a `cell.layers` section declaring its roles");
        let roles = cell
            .iter()
            .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();
        // Mirrors `verify::pdk::scalar_rules`: the generators ask for a rule by
        // its deck id and get the single scalar length bound it carries
        // (`params.limit.nm`). Ratio/count/flag parameters (density, angle…)
        // are not a length the generators query — skipped, as before.
        let mut rules = Vec::new();
        let mut grid = 1;
        if let Some(declared) = v["rules"].as_object() {
            for (id, body) in declared {
                if body["kind"] == "off_grid" {
                    if let Some(g) = body["params"]["pitch"]["nm"].as_i64() {
                        grid = g as i32;
                    }
                }
                if let Some(n) = body["params"]["limit"]["nm"].as_i64() {
                    rules.push((id.clone(), n as i32));
                }
            }
        }
        Self { layers, roles, rules, grid }
    }

    pub(crate) fn layer(&self, role: &str) -> Option<pnr_core::LayerId> {
        let name = self
            .roles
            .iter()
            .find(|(r, _)| r == role)
            .map_or(role, |(_, l)| l.as_str());
        self.layers
            .iter()
            .position(|n| n == name)
            .map(|i| pnr_core::LayerId(i as u16))
    }

    pub(crate) fn rule(&self, name: &str, default: i32) -> i32 {
        self.rules.iter().find(|(n, _)| n == name).map_or(default, |(_, v)| *v)
    }

    pub(crate) fn grid(&self) -> i32 {
        self.grid
    }
}

#[cfg(all(test, not(feature = "gpurify")))]
mod view_tests {
    use super::DeckView;

    /// Every role a cell generator resolves unconditionally must be present. This
    /// is the non-`verify` mirror of `verify::pdk::REQUIRED_ROLES`; the two builds
    /// draw against the same deck, so a gap here is a gap there.
    #[test]
    fn generic_deck_declares_every_mandatory_role() {
        let d = DeckView::parse(super::GENERIC_DECK_JSON);
        for role in [
            "diff", "poly", "li", "licon", "mcon", "met1", "nwell", "nsdm", "psdm", "tap",
        ] {
            assert!(d.layer(role).is_some(), "generic deck has no layer for role {role:?}");
        }
        assert!(d.grid() > 0, "generic deck declares no fabrication grid");
    }

    /// The scalar rules the generator examples query by name must answer from
    /// the deck, not from the call sites' compiled-in defaults — that is the
    /// whole point of embedding the document.
    #[test]
    fn queried_rule_ids_answer_from_the_deck() {
        let d = DeckView::parse(super::GENERIC_DECK_JSON);
        assert_eq!(d.rule("min_width", -1), 140, "met1 min_width lost its legacy id");
        assert_eq!(d.rule("min_spacing", -1), 140, "met1 min_spacing lost its legacy id");
        assert_eq!(d.grid(), 5, "manufacturing grid is 5 nm");
    }
}

#[cfg(all(test, feature = "gpurify"))]
mod tests {
    use gdsverify::ingest::deck::{parse_deck, Deck, ParamValue};
    use gdsverify::ingest::StrTable;
    use std::collections::HashMap;

    // The two source PDKs, embedded so the test is hermetic. Paths are relative
    // to this file (`kernel/macroMaster/src/`) up to `new_skeleton/pdks/`.
    const SKY130: &str = include_str!("../../../pdks/sky130.json");
    const FINFET: &str = include_str!("../../../pdks/generic_finfet.json");

    /// The kinds merged by scalar strictness: a single `limit` length whose
    /// direction is known (`max_width` is the one upper bound). Density, angle,
    /// off-grid, antenna/EM/ERC kinds carry no such scalar and are outside the
    /// envelope comparison.
    const MERGED: &[&str] = &[
        "min_width", "min_spacing", "min_spacing_diff", "min_enclosure", "min_extension",
        "min_area", "notch", "corner_to_corner", "overlap", "max_width",
    ];

    fn parse(label: &str, src: &str) -> (Deck, StrTable) {
        let mut strings = StrTable::default();
        let deck = parse_deck(src, verify::pdk::nm_grid(), &mut strings)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        (deck, strings)
    }

    /// A rule's *layer signature* (`kind|layer|layer…`, names resolved through
    /// the deck's own table so signatures compare across decks) → whether it is
    /// an upper bound, and its `limit` in nm. One entry per merged-kind rule.
    fn bounds(deck: &Deck, strings: &StrTable) -> HashMap<String, (bool, i64)> {
        let Some(limit) = strings.get("limit") else { return HashMap::new() };
        deck.rules
            .spec
            .iter()
            .filter_map(|s| {
                let kind = strings.resolve(s.kind);
                if !MERGED.contains(&kind) {
                    return None;
                }
                let names: Vec<&str> = deck
                    .rules
                    .layers_of(s)
                    .iter()
                    .map(|&l| strings.resolve(deck.layers.name(l)))
                    .collect();
                let Some(ParamValue::Length(d)) = deck.rules.param(s, limit) else {
                    return None;
                };
                Some((format!("{kind}|{}", names.join("|")), (kind == "max_width", d.raw())))
            })
            .collect()
    }

    /// Every layer name a deck declares (base and derived).
    fn layer_names(deck: &Deck, strings: &StrTable) -> Vec<String> {
        (0..deck.layers.len())
            .map(|i| {
                strings
                    .resolve(deck.layers.name(gdsverify::geom::LayerId(i as u16)))
                    .to_string()
            })
            .collect()
    }

    // The deck parses through the raw gdsverify path *and* through the exact
    // path GenericPdk uses (`verify::Pdk::from_json`, which also validates the
    // Philis `cell` section: mandatory roles, construction scalars, routing
    // stack cross-checked against connectivity).
    #[test]
    fn generic_deck_parses() {
        let (deck, strings) = parse("generic", super::GENERIC_DECK_JSON);
        assert_eq!(deck.layers.len(), 19, "the universal layer set has 19 layers");
        assert_eq!(
            super::GENERIC_DECK_JSON.matches("\"kind\"").count(),
            deck.rules.spec.len(),
            "every declared rule parsed into the rule table"
        );
        let grid = strings.get("off_grid").map(|kind| {
            deck.rules.spec.iter().find(|s| s.kind == kind).expect("an off_grid rule")
        });
        assert!(grid.is_some(), "generic deck declares no fabrication grid");
        verify::Pdk::from_json(super::GENERIC_DECK_JSON)
            .expect("generic deck is a complete Philis PDK");
    }

    // Finfet-topology rules the generic deck deliberately does NOT carry, even
    // though their layers are universal. The engine's semantics make them
    // unsatisfiable by the *planar* cell style this deck's `cell` section
    // prescribes: `min_spacing_diff` counts an overlapping pair as zero spacing
    // (so poly-over-diff — every planar MOS gate — and diff-inside-nwell — every
    // planar PMOS — always violate), and `min_enclosure` counts an unhosted
    // inner shape as zero enclosure (so a planar NMOS diff, which sits in *no*
    // nwell by design, always violates). In the finfet source these layers name
    // raised-S/D topology where the pairs genuinely never interact; on planar
    // geometry the rules contradict the deck itself (`nwell_encloses_diff` +
    // `nwell_to_diff_min_spacing` cannot both hold for any diff/nwell pair).
    const PLANAR_INAPPLICABLE: &[&str] = &[
        "min_spacing_diff|poly|diff",
        "min_spacing_diff|nwell|diff",
        "min_enclosure|nwell|diff",
    ];

    // The generic deck must be at least as strict as EACH source PDK on every
    // shared rule over universal layers: min-kinds >= source, max_width <=
    // source. Rules naming a layer the generic deck dropped (sky130-only
    // hvtp/lvtn/rpm/npn/pnp) are dropped with the layer, and the
    // `PLANAR_INAPPLICABLE` finfet-topology rules are dropped by semantics (see
    // above). Guards a future divergence in the source PDKs from silently
    // loosening the envelope.
    #[test]
    fn generic_is_at_least_as_strict_as_each_source() {
        let (generic, g_strings) = parse("generic", super::GENERIC_DECK_JSON);
        let g = bounds(&generic, &g_strings);
        let g_layers = layer_names(&generic, &g_strings);
        for (label, src_json) in [("sky130", SKY130), ("generic_finfet", FINFET)] {
            let (src, s_strings) = parse(label, src_json);
            for (sig, (is_max, sv)) in bounds(&src, &s_strings) {
                // `sig` is `kind|layer|layer…`; skip rules on non-universal layers.
                if sig.split('|').skip(1).any(|l| !g_layers.iter().any(|n| n == l)) {
                    continue;
                }
                if PLANAR_INAPPLICABLE.contains(&sig.as_str()) {
                    continue;
                }
                let (_, gv) = g
                    .get(&sig)
                    .unwrap_or_else(|| panic!("{label}: generic deck missing shared rule `{sig}`"));
                if is_max {
                    assert!(*gv <= sv, "{label}: `{sig}` generic max {gv} looser than source {sv}");
                } else {
                    assert!(*gv >= sv, "{label}: `{sig}` generic min {gv} weaker than source {sv}");
                }
            }
        }
    }
}
