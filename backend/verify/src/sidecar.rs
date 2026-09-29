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
    Bool,
    /// Any number.
    Real,
    /// Array of integers, one per matching tier.
    Tier,
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
    pub name: &'static str,
    pub kind: Kind,
    /// Missing (or null) fails `Pdk::load`.
    pub required: bool,
    /// Process data: a non-null value needs a `<name>_source`.
    pub sourced: bool,
    /// Where it is read, or `unread`.
    pub reader: &'static str,
}

const fn k(name: &'static str, kind: Kind, required: bool, sourced: bool, reader: &'static str) -> Key {
    Key { name, kind, required, sourced, reader }
}

use Kind::{Bool, Count, Layers, List, Nm, Real, Table, Text, Tier};

/// Every key, alphabetical. `<name>_source` of a registered key is implied.
pub const KEYS: &[Key] = &[
    k("antenna_sidewall", Table, false, false, "backend/verify/src/pdk.rs antenna_rule"),
    k("antenna_source", Text, false, false, "provenance of antenna_sidewall and the deck's ANT rules"),
    k("avt_n_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("avt_p_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("bjt_max_emitter_stripe", Nm, false, false, "unread"),
    k("bjt_min_emitter_side", Nm, true, false, "kernel/cells/src/bjt.rs"),
    k("bjt_stripe_gap", Nm, false, false, "unread"),
    k("cap_density_ff_um2", Real, false, true, "benchmarks/src/fixtures.rs PdkPreprocess"),
    k("cap_unit_side", Nm, true, false, "kernel/cells/src/capacitor.rs, cap_array.rs"),
    k("density_source", Text, false, false, "provenance of the deck's density rules"),
    k("device_gap", Nm, true, false, "kernel/cells/src/capacitor.rs; frontend/library/src/emit.rs"),
    k("diode_gap", Nm, true, false, "kernel/cells/src/diode.rs"),
    k("diode_l", Nm, true, false, "kernel/cells/src/diode.rs; frontend/library/src/elaborate.rs"),
    k("diode_w", Nm, true, false, "kernel/cells/src/diode.rs; frontend/library/src/elaborate.rs"),
    k("dti", Nm, false, false, "unread"),
    k("em_current_density_source", Text, false, false, "provenance of the deck's EM rules"),
    k("erc_rules_note", Text, false, false, "documentation"),
    k("finfet_note", Text, false, false, "documentation"),
    k("gate_cap_af_um2", Count, false, true, "frontend/library/src/lib.rs annotation"),
    k("guard_licon_pitch", Nm, true, false, "kernel/cells/src/mosfet.rs, post_cell.rs"),
    k("inapplicable_rules", List, false, false, "backend/verify/src/pdk.rs"),
    k("inapplicable_rules_note", Text, false, false, "documentation"),
    k("ind_min_diameter", Nm, true, false, "kernel/cells/src/inductor.rs"),
    k("ind_min_trace", Nm, true, false, "kernel/cells/src/inductor.rs"),
    k("layers", Layers, true, false, "backend/verify/src/pdk.rs parse_roles"),
    k("li_encloses_licon", Nm, false, false, "kernel/cells/src/mosfet.rs, bjt.rs, diode.rs, resistor.rs"),
    k("li_encloses_licon_one_side", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("licon_poly_enc", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("licon_to_gate_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("linewidth_control_nm", Table, false, false, "unread"),
    k("lod_kvth0_n_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("lod_kvth0_p_mv_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("lod_moat_ext_moderate", Nm, true, false, "kernel/cells/src/mosfet.rs; frontend/library/src/lib.rs"),
    k("lod_moat_ext_nm", Tier, false, false, "unread"),
    k("m1_enc", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("max_finger_width", Nm, true, false, "frontend/library/src/cellgen.rs folds (0 = no limit)"),
    k("min_finger_width", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's); frontend/library/src/cellgen.rs"),
    k("min_guard_ring_width", Nm, true, false, "kernel/cells/src/bjt.rs, post_cell.rs"),
    k("mom_finger_space", Nm, true, false, "kernel/cells/src/capacitor.rs"),
    k("mom_finger_width", Nm, true, false, "kernel/cells/src/capacitor.rs"),
    k("n_well_depth", Nm, false, true, "unread"),
    k("npn_isolation", Count, false, false, "kernel/cells/src/bjt.rs"),
    k("npn_isolation_note", Text, false, false, "documentation"),
    k("p_epi_thickness", Nm, false, true, "unread"),
    k("p_well_depth", Nm, false, true, "unread"),
    k("plate_spacing", Nm, true, false, "kernel/cells/src/capacitor.rs, cap_array.rs"),
    k("polycon_to_diff_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("polycon_to_pdiff_spacing", Nm, false, false, "kernel/cells/src/mosfet.rs"),
    k("res_corner_squares", Real, false, false, "unread"),
    k("res_head", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_min_segment", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_min_width", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_seg_gap", Nm, true, false, "kernel/cells/src/resistor.rs"),
    k("res_serpentine_aspect", Real, false, false, "unread"),
    k("resistors", Table, false, false, "backend/verify/src/pdk.rs recipe"),
    k("retrograde_pwell", Bool, false, true, "unread"),
    k("sd_width", Nm, true, false, "kernel/cells/src/mosfet.rs"),
    k("sheet_tolerance", Table, false, false, "unread"),
    k("svt_uv_per_um", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("tie_max_dist_nm", Nm, false, true, "unread"),
    k("via_enclosure", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("via_spacing", Nm, false, false, "kernel/cells/src/builder.rs dim (raises the deck's)"),
    k("vt_tc_uv_per_k", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("vt_tc_uv_per_k_p", Real, false, true, "frontend/library/src/lib.rs annotation"),
    k("waivers", Table, false, false, "backend/verify/src/checker.rs"),
    k("well_enclosure", Nm, false, false, "tests only (frontend/library/tests/ota_cross_pdk.rs)"),
    k("well_spacing", Nm, false, false, "unread"),
    k("wpe_clearance_moderate", Nm, true, false, "kernel/cells/src/mosfet.rs; frontend/library/src/lib.rs"),
    k("wpe_clearance_nm", Tier, false, false, "unread"),
];

/// The registry row for `name`.
fn key(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.name == name)
}

/// Every problem at once: unknown key (a typo), wrong kind, a required key
/// missing, a sourced key with a non-null value and no non-empty
/// `<name>_source`.
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
            Tier => v.as_array().is_some_and(|a| a.iter().all(Value::is_i64)),
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

/// `<name>_source`, when non-empty text.
pub(crate) fn source<'a>(cell: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a str> {
    cell.get(&format!("{name}_source"))?.as_str().filter(|s| !s.trim().is_empty())
}
