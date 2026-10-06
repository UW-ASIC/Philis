//! The registry of every `cell.*` key a Philis sidecar may carry: its kind,
//! whether loading requires it, whether it is process data that must name its
//! source, and who reads it.
//!
//! `required` is the set generators read through `Process::rule` with a
//! compiled default, less keys whose reader only raises a deck-derived value
//! and the `npn_isolation` flag (kernel/cells/tests/deck_keys.rs checks both
//! directions against a recording run; `max_finger_width`, read by the
//! library's cellgen, by name), so a shipped deck never silently builds to a
//! number compiled into Philis. `sourced` keys are measurements of the process
//! (mismatch, temperature, capacitance, well geometry): each non-null value
//! needs a non-empty `<key>_source`; one starting `UNVERIFIED` is accepted
//! and listed by [`crate::Pdk::unverified`]. Generator construction
//! dimensions are Philis choices bounded below by deck rules
//! (kernel/cells/src/builder.rs `dim`), so they are not sourced.

use serde_json::Value;

/// The JSON shape a key's value must have. `null` is accepted for every kind
/// (the process does not state it) except on a `required` key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Integer length, nm.
    Nm,
    /// Non-negative integer (a count or 0/1 flag).
    Count,
    /// `true` or `false`.
    Bool,
    /// Any number.
    Real,
    /// Array of exactly 3 (MIN, MOD, EXC; index = `MatchClass as usize`), each a
    /// non-negative integer or `null` (the process does not state that tier).
    Tier,
    /// A string.
    Text,
    /// Array of strings.
    List,
    /// Object whose shape its reader checks.
    Table,
    /// `cell.layers`: role → layer name plus the routing stack, checked by
    /// `parse_roles`.
    Layers,
}

/// One registry row.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    /// The key under `cell`, without the `cell.` prefix.
    pub name: &'static str,
    /// The JSON shape its value must have.
    pub kind: Kind,
    /// Missing (or null) fails `Pdk::load`.
    pub required: bool,
    /// Process data: a non-null value needs a `<name>_source`.
    pub sourced: bool,
    /// Where it is read, or `unread`.
    pub reader: &'static str,
}

/// Shorthand row constructor that keeps [`KEYS`] one line per key.
const fn k(name: &'static str, kind: Kind, required: bool, sourced: bool, reader: &'static str) -> Key {
    Key { name, kind, required, sourced, reader }
}

use Kind::{Bool, Count, Layers, List, Nm, Real, Table, Text, Tier};

/// Every key, sorted by name (ASCII order; [`validate`] binary-searches it).
/// `<name>_source` of a registered key is implied.
pub const KEYS: &[Key] = &[
    k("abeta_n_pct_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("abeta_p_pct_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("antenna_source", Text, false, false, "provenance of the deck's antenna rules"),
    k("avt_n_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("avt_p_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("bjt_ka_pct_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("bjt_max_emitter_stripe", Nm, false, false, "unread"),
    k("bjt_min_emitter_side", Nm, true, false, "kernel/cells/src/bjt.rs"),
    k("bjt_stripe_gap", Nm, false, false, "unread"),
    k("bjts", Table, false, false, "backend/verify/src/pdk.rs recipe"),
    k("cap_density_ff_um2", Real, false, true, "benchmarks/src/fixtures.rs PdkPreprocess"),
    k("cap_unit_side", Nm, true, false, "kernel/cells/src/capacitor.rs, cap_array.rs"),
    k("capacitors", Table, false, false, "backend/verify/src/pdk.rs recipe"),
    k("density_source", Text, false, false, "provenance of the deck's density rules"),
    k("device_gap", Nm, true, false, "kernel/cells/src/capacitor.rs; frontend/library/src/emit.rs"),
    k("diode_gap", Nm, true, false, "kernel/cells/src/diode.rs"),
    k("diode_l", Nm, true, false, "kernel/cells/src/diode.rs; frontend/library/src/elaborate.rs"),
    k("diode_w", Nm, true, false, "kernel/cells/src/diode.rs; frontend/library/src/elaborate.rs"),
    k("dti", Nm, false, false, "unread"),
    k("dummy_gates_per_end", Count, true, false, "kernel/cells/src/mosfet.rs (dummy gates each end of a matched row)"),
    k("dummy_max_l_nm", Nm, false, true, "kernel/cells/src/mosfet.rs (dummy gate length cap)"),
    k("dummy_reach_nm", Tier, false, true, "kernel/analog/src/matching/class.rs"),
    k("em_current_density_source", Text, false, false, "provenance of the deck's EM rules"),
    k("em_derating", Table, false, true, "backend/verify/src/pdk.rs em_limit (when the deck rule states no Black parameters)"),
    k("em_front_row_cuts", Bool, false, true, "frontend/library/src/lib.rs em_rules; backend/dr DetailedCfg (REL-12)"),
    k("epi_thickness_nm", Nm, false, true, "frontend/library/src/lib.rs annotation (isolation distance on epi_on_pplus)"),
    k("erc_rules_note", Text, false, false, "documentation"),
    k("finfet_note", Text, false, false, "documentation"),
    k("gate_cap_af_um2", Count, false, true, "frontend/library/src/lib.rs annotation"),
    k("gate_ext_extra_nm", Tier, false, true, "kernel/analog/src/matching/class.rs"),
    k("guard_licon_pitch", Nm, true, false, "kernel/cells/src/mosfet.rs, post_cell.rs"),
    k("inapplicable_rules", List, false, false, "backend/verify/src/pdk.rs"),
    k("inapplicable_rules_note", Text, false, false, "documentation"),
    k("latent_merge_nm", Nm, false, true, "frontend/library/src/elaborate.rs stack (GAP-13; default the layer's min spacing)"),
    k("layers", Layers, true, false, "backend/verify/src/pdk.rs parse_roles"),
    k("li_encloses_licon", Nm, false, false, "kernel/cells/src/mosfet.rs, bjt.rs, diode.rs, resistor.rs"),
    k("li_encloses_licon_one_side", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("licon_poly_enc", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("licon_to_gate_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("linewidth_control_nm", Table, false, false, "unread"),
    k("lod_kvth0_n_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("lod_kvth0_p_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("lod_moat_ext_nm", Tier, false, true, "kernel/cells/src/mosfet.rs; frontend/library/src/lib.rs; kernel/analog/src/matching/class.rs"),
    k("m1_enc", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("max_finger_width", Nm, true, false, "frontend/library/src/cellgen.rs folds (0 = no limit)"),
    k("metal_family", Text, false, true, "kernel/analog/src/routing/em.rs metal_family (REL-17 ESD width; \"al\" | \"cu\")"),
    k("min_finger_width", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's); frontend/library/src/cellgen.rs"),
    k("min_guard_ring_width", Nm, true, false, "kernel/cells/src/bjt.rs, post_cell.rs"),
    k("mom_finger_space", Nm, true, false, "kernel/cells/src/capacitor.rs"),
    k("mom_finger_width", Nm, true, false, "kernel/cells/src/capacitor.rs"),
    k("n_well_depth", Nm, false, true, "unread"),
    k("npn_isolation", Count, false, false, "kernel/cells/src/bjt.rs"),
    k("npn_isolation_note", Text, false, false, "documentation"),
    k("p_epi_thickness", Nm, false, true, "unread"),
    k("p_well_depth", Nm, false, true, "unread"),
    k("placement_space", Table, false, false, "frontend/library/src/lib.rs place_rules"),
    k("plate_spacing", Nm, true, false, "kernel/cells/src/capacitor.rs, cap_array.rs"),
    k("polycon_to_diff_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("polycon_to_pdiff_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("res_corner_squares", Real, false, false, "unread"),
    k("res_dummy_span_nm", Tier, false, true, "kernel/analog/src/matching/class.rs"),
    k("res_head", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_length_floor_x", Tier, false, true, "kernel/analog/src/matching/class.rs"),
    k("res_min_segment", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_min_width", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_seg_gap", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_self_heat_dt_k", Real, false, true, "kernel/cells/src/resistor.rs self_heating_min_width_nm (CELL-23; default 5 K, Hastings eq. 5.8)"),
    k("res_serpentine_aspect", Real, false, false, "unread"),
    k("res_tox_nm", Real, false, true, "kernel/cells/src/resistor.rs self_heating_min_width_nm (CELL-23; else pex height_nm of the body layer)"),
    k("res_value_tol_ppm", Count, true, false, "kernel/cells/src/resistor.rs"),
    k("res_width_floor_permille", Tier, false, true, "kernel/analog/src/matching/class.rs"),
    k("resistors", Table, false, false, "backend/verify/src/pdk.rs recipe"),
    k("retrograde_pwell", Bool, false, true, "kernel/cells/src/post_cell.rs drawable"),
    k("sd_width", Nm, true, false, "kernel/cells/src/mosfet.rs"),
    k("sheet_tolerance", Table, false, false, "unread"),
    k("substrate_kind", Text, false, true, "frontend/library/src/lib.rs annotation (\"bulk\" | \"epi_on_pplus\" | null)"),
    k("svt_a_uv2_per_um2", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("svt_b_uv2", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("svt_uv_per_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("tie_max_dist_nm", Nm, false, true, "kernel/cells/src/mosfet.rs taps_in_reach, via frontend/library/src/cellgen.rs draw_variants (CELL-13)"),
    k("vbe_tc_uv_per_k", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("via_enclosure", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("via_spacing", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("vt_tc_uv_per_k", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("vt_tc_uv_per_k_p", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("waivers", Table, false, false, "backend/verify/src/checker.rs"),
    k("well_enclosure", Nm, false, false, "tests only (frontend/library/tests/ota_cross_pdk.rs)"),
    k("well_spacing", Nm, false, false, "unread"),
    k("wpe_clearance_nm", Tier, false, true, "kernel/cells/src/mosfet.rs; frontend/library/src/lib.rs; kernel/analog/src/matching/class.rs"),
];

/// Returns the registry row for `name`.
fn key(name: &str) -> Option<&'static Key> {
    KEYS.binary_search_by(|k| k.name.cmp(name)).ok().map(|i| &KEYS[i])
}

/// Returns every problem with a sidecar's `cell` object at once, one
/// message per problem, empty when it is valid: not an object, an unknown
/// key (a typo), a value of the wrong kind, a `<name>_source` that is not
/// text, a required key missing or null, a sourced key with a non-null
/// value and no non-empty `<name>_source`.
#[must_use]
pub fn validate(cell: &Value) -> Vec<String> {
    let Some(obj) = cell.as_object() else { return vec!["the sidecar's `cell` is not an object".into()] };
    let mut bad = Vec::new();
    for (name, v) in obj {
        if name.strip_suffix("_source").is_some_and(|b| key(b).is_some()) {
            if !v.is_string() {
                bad.push(format!("cell.{name}: a source is text, got {v}"));
            }
            continue;
        }
        let Some(row) = key(name) else {
            bad.push(format!("cell.{name}: unknown key (a typo? register it in backend/verify/src/sidecar.rs)"));
            continue;
        };
        let ok = match row.kind {
            _ if v.is_null() => true,
            Nm => v.is_i64(),
            Count => v.is_u64(),
            Bool => v.is_boolean(),
            Real => v.is_number(),
            Tier => v.as_array().is_some_and(|a| a.len() == 3 && a.iter().all(|e| e.is_null() || e.is_u64())),
            Text => v.is_string(),
            List => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
            Table | Layers => v.is_object(),
        };
        if !ok {
            bad.push(format!("cell.{name}: expected {:?}, got {v}", row.kind));
        }
    }
    for row in KEYS {
        let v = obj.get(row.name).filter(|v| !v.is_null());
        if row.required && v.is_none() {
            bad.push(format!(
                "cell.{}: required ({} reads it with a compiled default; add this process's value)",
                row.name, row.reader
            ));
        }
        if row.sourced && v.is_some() && source(obj, row.name).is_none() {
            bad.push(format!("cell.{}: process data with no `{}_source`", row.name, row.name));
        }
    }
    bad
}

/// Returns `<name>_source`, when it is text that is not all whitespace.
pub(crate) fn source<'a>(cell: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a str> {
    cell.get(&format!("{name}_source"))?.as_str().filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use serde_json::json;

    /// A value of `kind` that `validate` must accept.
    fn valid(kind: Kind) -> Value {
        match kind {
            Nm => json!(120),
            Count => json!(2),
            Bool => json!(true),
            Real => json!(1.5),
            Tier => json!([1, null, 3]),
            Text => json!("x"),
            List => json!(["a", "b"]),
            Table | Layers => json!({}),
        }
    }

    /// Every required key with a valid value, and a source for each sourced one.
    fn minimal() -> serde_json::Map<String, Value> {
        let mut m = serde_json::Map::new();
        for row in KEYS.iter().filter(|k| k.required) {
            m.insert(row.name.into(), valid(row.kind));
            if row.sourced {
                m.insert(format!("{}_source", row.name), json!("datasheet"));
            }
        }
        m
    }

    fn problems(m: &serde_json::Map<String, Value>) -> Vec<String> {
        validate(&Value::Object(m.clone()))
    }

    #[test]
    fn keys_are_sorted_and_unique() {
        assert!(KEYS.windows(2).all(|w| w[0].name < w[1].name), "KEYS must stay strictly sorted");
        assert!(KEYS.iter().all(|k| key(k.name).is_some_and(|r| r.name == k.name)));
        assert!(key("").is_none() && key("zzz").is_none() && key("a").is_none());
    }

    #[test]
    fn a_non_object_is_one_problem() {
        for v in [json!(null), json!(1), json!("cell"), json!([])] {
            assert_eq!(validate(&v), ["the sidecar's `cell` is not an object"]);
        }
    }

    #[test]
    fn the_minimal_cell_is_valid() {
        assert_eq!(problems(&minimal()), Vec::<String>::new());
    }

    #[test]
    fn every_required_key_missing_is_reported() {
        let p = validate(&json!({}));
        let required = KEYS.iter().filter(|k| k.required).count();
        assert!(required > 0);
        assert_eq!(p.len(), required, "{p:?}");
        assert!(p.iter().all(|m| m.contains(": required (")));
    }

    #[test]
    fn a_null_required_key_is_missing() {
        let mut m = minimal();
        m.insert("sd_width".into(), Value::Null);
        assert_eq!(problems(&m), ["cell.sd_width: required (kernel/cells/src/mosfet.rs reads it with a compiled default; add this process's value)"]);
    }

    #[test]
    fn null_is_accepted_for_an_optional_key_of_any_kind() {
        for row in KEYS.iter().filter(|k| !k.required) {
            let mut m = minimal();
            m.insert(row.name.into(), Value::Null);
            assert_eq!(problems(&m), Vec::<String>::new(), "{}", row.name);
        }
    }

    #[test]
    fn every_key_accepts_its_kind() {
        for row in KEYS {
            let mut m = minimal();
            m.insert(row.name.into(), valid(row.kind));
            if row.sourced {
                m.insert(format!("{}_source", row.name), json!("UNVERIFIED: estimate"));
            }
            assert_eq!(problems(&m), Vec::<String>::new(), "{}", row.name);
        }
    }

    #[test]
    fn wrong_kinds_are_rejected() {
        let cases: [(Kind, Value); 14] = [
            (Nm, json!(1.5)),
            (Nm, json!("10")),
            (Count, json!(-1)),
            (Count, json!(1.0)),
            (Bool, json!(1)),
            (Real, json!("1.0")),
            (Tier, json!([1, 2])),
            (Tier, json!([1, 2, 3, 4])),
            (Tier, json!([1, -2, 3])),
            (Tier, json!({})),
            (Text, json!(1)),
            (List, json!(["a", 1])),
            (Table, json!([])),
            (Layers, json!("met1")),
        ];
        for (kind, v) in cases {
            let row = KEYS.iter().find(|k| k.kind == kind && !k.sourced).or_else(|| KEYS.iter().find(|k| k.kind == kind)).unwrap();
            let mut m = minimal();
            m.insert(row.name.into(), v.clone());
            m.insert(format!("{}_source", row.name), json!("x"));
            let p = problems(&m);
            assert_eq!(p, [format!("cell.{}: expected {kind:?}, got {v}", row.name)], "{kind:?} {v}");
        }
    }

    #[test]
    fn an_unknown_key_is_a_typo() {
        let mut m = minimal();
        m.insert("sd_widht".into(), json!(1));
        assert_eq!(problems(&m), ["cell.sd_widht: unknown key (a typo? register it in backend/verify/src/sidecar.rs)"]);
        // `_source` of an unknown key is itself unknown.
        let mut m = minimal();
        m.insert("nope_source".into(), json!("x"));
        assert_eq!(problems(&m).len(), 1);
    }

    #[test]
    fn a_sourced_value_needs_a_non_blank_source() {
        let (name, kind) = ("avt_n_mv_um", Real);
        assert!(key(name).is_some_and(|k| k.sourced && k.kind == kind));
        let want = [format!("cell.{name}: process data with no `{name}_source`")];
        for source in [None, Some(json!("")), Some(json!("  \t"))] {
            let mut m = minimal();
            m.insert(name.into(), json!(3.5));
            if let Some(s) = source {
                m.insert(format!("{name}_source"), s);
            }
            assert_eq!(problems(&m), want);
        }
        // A null value needs no source; a source alone is fine.
        let mut m = minimal();
        m.insert(name.into(), Value::Null);
        m.insert(format!("{name}_source"), json!("paper"));
        assert_eq!(problems(&m), Vec::<String>::new());
    }

    #[test]
    fn a_source_must_be_text() {
        let mut m = minimal();
        m.insert("avt_n_mv_um".into(), json!(3.5));
        m.insert("avt_n_mv_um_source".into(), json!(7));
        let p = problems(&m);
        assert!(p.contains(&"cell.avt_n_mv_um_source: a source is text, got 7".to_string()), "{p:?}");
        assert!(p.iter().any(|m| m.contains("process data with no")), "{p:?}");
    }

    // `antenna_source` is a key of its own (no `antenna` key), checked as Text.
    #[test]
    fn a_registered_key_ending_in_source_is_its_own_key() {
        let mut m = minimal();
        m.insert("antenna_source".into(), json!(1));
        assert_eq!(problems(&m), ["cell.antenna_source: expected Text, got 1"]);
    }

    #[test]
    fn source_trims_blank_text() {
        let m = json!({ "a_source": " x ", "b_source": " ", "c_source": 1 });
        let m = m.as_object().unwrap();
        assert_eq!(source(m, "a"), Some(" x "));
        assert_eq!(source(m, "b"), None);
        assert_eq!(source(m, "c"), None);
        assert_eq!(source(m, "d"), None);
    }
}
