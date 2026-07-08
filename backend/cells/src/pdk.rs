//! Cell-construction PDK parameters.
//!
//! DRC rules live in the deck (`Deck::from_json`); these are the *construction*
//! values generators need (contact size, S/D width, enclosures). Loaded from
//! the `"cell"` section of the same `pdks/<name>.json` file.

use serde::Deserialize;

/// Layer roles the generators draw on, mapped to this PDK's layer names.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Layers {
    pub diff: String,
    pub poly: String,
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
}

impl Default for Layers {
    fn default() -> Self {
        Self {
            diff: "diff".into(),
            poly: "poly".into(),
            li: "li".into(),
            licon: "licon".into(),
            mcon: "mcon".into(),
            met1: "met1".into(),
            nwell: "nwell".into(),
            nsdm: "nsdm".into(),
            psdm: "psdm".into(),
            tap: "tap".into(),
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

// ponytail: defaults are sky130 values so decks without a "cell" section
// still load; add real per-PDK sections as they're characterized.
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
    }

    #[test]
    fn layer_roles_remap() {
        let p = Pdk::from_json(
            r#"{"cell": {"layers": {"diff": "Activ", "met1": "Metal1"}}}"#,
        )
        .unwrap();
        assert_eq!(p.layers.diff, "Activ");
        assert_eq!(p.layers.met1, "Metal1");
        assert_eq!(p.layers.poly, "poly", "unspecified role keeps default");
    }
}
