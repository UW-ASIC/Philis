//! Cell-construction PDK parameters.
//!
//! DRC rules live in the deck (`Deck::from_json`); these are the *construction*
//! values generators need (contact size, S/D width, enclosures). Loaded from
//! the `"cell"` section of the same `pdks/<name>.json` file.

use serde::Deserialize;

/// Optional deep-trench-isolation construction rules, in nm.
///
/// Absence means that the process does not expose DTI to automatic placement.
/// Keeping this process capability in the PDK prevents circuit recognition from
/// inventing a technology-specific 5 um keep-out on every complementary pair.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct DtiRules {
    /// Maximum raw edge gap for two devices to share a trench.
    pub shared_max_gap: i32,
    /// Minimum raw edge gap when the devices do not share a trench.
    pub separated_min_gap: i32,
}

/// Layer roles the generators draw on, mapped to this PDK's layer names.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Layers {
    pub diff: String,
    pub poly: String,
    /// Resistor-body layer: drawn poly that is NOT a conductor in the deck's
    /// connectivity, so LVS extracts it as a resistor body instead of a wire.
    pub rpoly: String,
    pub li: String,
    pub licon: String,
    pub mcon: String,
    pub met1: String,
    pub nwell: String,
    /// N-select (nsdm) implant layer for N+ taps.
    pub nsdm: String,
    /// P-select (psdm) implant layer for P+ taps.
    pub psdm: String,
    /// Tap diffusion layer (if separate from diff).
    pub tap: String,
    /// NPN recognition marker layer (deck `device_recognition.bjt` type_marker).
    pub npn: String,
    /// PNP recognition marker layer (deck `device_recognition.bjt` type_marker).
    pub pnp: String,
    /// Ordered routing conductors, bottom to top.
    pub routing_metals: Vec<String>,
    /// Ordered cuts between adjacent entries in `routing_metals`.
    pub routing_vias: Vec<String>,
}

impl Default for Layers {
    fn default() -> Self {
        Self {
            diff: "diff".into(),
            poly: "poly".into(),
            rpoly: "rpoly".into(),
            li: "li".into(),
            licon: "licon".into(),
            mcon: "mcon".into(),
            met1: "met1".into(),
            nwell: "nwell".into(),
            nsdm: "nsdm".into(),
            psdm: "psdm".into(),
            tap: "tap".into(),
            npn: "npn".into(),
            pnp: "pnp".into(),
            routing_metals: vec![
                "met1".into(),
                "met2".into(),
                "met3".into(),
                "met4".into(),
                "met5".into(),
            ],
            routing_vias: vec!["via1".into(), "via2".into(), "via3".into(), "via4".into()],
        }
    }
}

/// Construction parameters for cell generation, in nm.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Pdk {
    pub layers: Layers,
    pub contact: i32,
    pub sd_width: i32,
    pub min_finger_width: i32,
    pub max_finger_width: i32,
    pub poly_ext: i32,
    pub via_enclosure: i32,
    pub plate_spacing: i32,
    pub res_head: i32,
    pub well_enclosure: i32,
    pub via_spacing: i32,
    pub mcon_size: i32,
    pub m1_enc: i32,
    pub met1_space: i32,
    pub device_gap: i32,
    /// Raw NMOS/PMOS edge spacing used by automatic well separation.
    pub well_spacing: i32,
    /// Process-specific DTI rules. `None` disables automatic DTI extraction.
    pub dti: Option<DtiRules>,
    pub res_corner_squares: f64,
    pub res_serpentine_aspect: f64,
    pub res_min_segment: i32,
    pub res_seg_gap: i32,
    pub bjt_max_emitter_stripe: i32,
    pub bjt_min_emitter_side: i32,
    pub bjt_base_frac: f64,
    pub bjt_collector_frac: f64,
    pub bjt_stripe_gap: i32,
    pub diode_gap: i32,
    pub ind_min_trace: i32,
    pub ind_min_diameter: i32,

    // ── WPE clearance (item 1.9) ──
    /// Gate-to-well-edge clearance per tier: [Minimal, Moderate, Exceptional] in nm.
    /// AOAL ch13 Rule 19 / FOLD 6.6.3.
    pub wpe_clearance_nm: [i32; 3],
    /// difftap.8 enclosure for nwell over diffusion (nm).
    pub nwell_diff_enc: i32,

    // ── Guard ring (items 1.4, 1.6) ──
    pub min_guard_ring_width: i32,
    /// Tap contact pitch in nm.
    pub guard_licon_pitch: i32,
    /// P-epi thickness (nm) — collecting ring width floor for PsubRing.
    pub p_epi_thickness: i32,
    /// P-well depth (nm) — retrograde process floor for PsubRing.
    pub p_well_depth: i32,
    /// N-well depth (nm) — collecting ring width floor for NwellRing.
    pub n_well_depth: i32,
    /// Whether the process uses a retrograde p-well.
    pub retrograde_pwell: bool,

    // ── LOD moat extension (item 1.10) ──
    /// Diffusion moat extension past outer gates per tier: [Moderate, Exceptional] in nm.
    pub lod_moat_ext_nm: [i32; 2],

    // ── Resistor tolerance (item 1.12) ──
    /// Sheet resistance tolerance (fractional, e.g. 0.20 = 20%) per resistor model.
    pub sheet_tolerance: std::collections::HashMap<String, f64>,
    /// Linewidth control (nm) — 3-sigma CD variation per resistor model.
    pub linewidth_control_nm: std::collections::HashMap<String, i32>,
}

// These defaults support isolated generator tests. The backend's production
// PDK factory rejects omitted construction fields, so no real process silently
// inherits these values.
impl Default for Pdk {
    fn default() -> Self {
        Self {
            layers: Layers::default(),
            contact: 170,
            sd_width: 250,
            min_finger_width: 420,
            max_finger_width: 10_000,
            poly_ext: 130,
            via_enclosure: 300,
            plate_spacing: 200,
            res_head: 300,
            well_enclosure: 300,
            via_spacing: 170,
            mcon_size: 170,
            m1_enc: 60,
            met1_space: 140,
            device_gap: 600,
            well_spacing: 600,
            dti: None,
            res_corner_squares: 0.56,
            res_serpentine_aspect: 10.0,
            res_min_segment: 10_000,
            res_seg_gap: 400,
            bjt_max_emitter_stripe: 25_000,
            bjt_min_emitter_side: 420,
            bjt_base_frac: 0.3,
            bjt_collector_frac: 0.5,
            bjt_stripe_gap: 200,
            diode_gap: 200,
            ind_min_trace: 1000,
            ind_min_diameter: 10_000,
            // WPE clearance: [Minimal >=2um, Moderate >=3um, Exceptional >=5um]
            wpe_clearance_nm: [2000, 3000, 5000],
            nwell_diff_enc: 180,
            // Guard ring
            min_guard_ring_width: 420,
            guard_licon_pitch: 340,
            p_epi_thickness: 3000,
            p_well_depth: 1500,
            n_well_depth: 2000,
            retrograde_pwell: false,
            // LOD moat: [Moderate 3um, Exceptional 5um]
            lod_moat_ext_nm: [3000, 5000],
            // Resistor tolerance — populated per-PDK; empty = use tier multiplier fallback
            sheet_tolerance: std::collections::HashMap::new(),
            linewidth_control_nm: std::collections::HashMap::new(),
        }
    }
}

#[derive(Deserialize)]
struct Doc {
    #[serde(default)]
    cell: Pdk,
}

impl Pdk {
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str::<Doc>(text)
            .map(|d| d.cell)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_partial_section_and_defaults() {
        let p = Pdk::from_json(r#"{"layers": {}, "cell": {"contact": 200}}"#).unwrap();
        assert_eq!(p.contact, 200);
        assert_eq!(p.sd_width, Pdk::default().sd_width);

        let p = Pdk::from_json(r#"{"layers": {}}"#).unwrap();
        assert_eq!(p.contact, 170);
        assert_eq!(p.layers.diff, "diff");
        assert_eq!(p.well_spacing, 600);
        assert!(p.dti.is_none());
    }

    #[test]
    fn parses_optional_dti_rules() {
        let p = Pdk::from_json(
            r#"{"cell":{"dti":{"shared_max_gap":500,"separated_min_gap":5000}}}"#,
        )
        .unwrap();
        let dti = p.dti.expect("DTI rules");
        assert_eq!(dti.shared_max_gap, 500);
        assert_eq!(dti.separated_min_gap, 5000);
    }

    #[test]
    fn layer_roles_remap() {
        let p =
            Pdk::from_json(r#"{"cell": {"layers": {"diff": "Activ", "met1": "Metal1"}}}"#).unwrap();
        assert_eq!(p.layers.diff, "Activ");
        assert_eq!(p.layers.met1, "Metal1");
        assert_eq!(p.layers.poly, "poly", "unspecified role keeps default");
    }
}
