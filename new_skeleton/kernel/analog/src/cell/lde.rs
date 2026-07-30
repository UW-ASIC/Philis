//! Layout-dependent effects (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/lde.rs`.

use pnr_core::ids::DeviceId;

/// **Layout-dependent effects (LDE).** Combines the Well-Proximity Effect (a
/// 4-edge exponential model of `ΔVth` vs distance to the well junction) and
/// LOD/STI stress (`ΔVth_STI ∝ 1/SA_eff − 1/SB_eff`). A matched pair must meet a
/// well-edge distance budget and an `SA`/`SB` mismatch cap to keep total `ΔVth`
/// inside the Pelgrom limit. Tier-set well distances (min 2 µm … exceptional ≥5 µm).
///
/// - **Role:** structural — well-edge distance feeds placement; `SA`/`SB` symmetry
///   feeds `cells`. Cold.
/// - **Books:** AOAL ch02/2.3.3–2.4.2, ch06/6.2, ch08/8.2.3 (#52); FOLD 6.6/6.6.3 (#6);
///   PNR_ANALOG 00/2.1–2.2 (#9–10), 04/2.5 (#63).
#[derive(Clone, Copy)]
pub struct LdeBound {
    pub pair: (DeviceId, DeviceId),
    /// Min distance to the well edge for WPE control, `nm`.
    // was: min_well_edge_distance_um: f64.
    pub min_well_edge_distance_nm: i32,
    /// Max WPE-induced `ΔVth`, µV.
    // was: max_wpe_dvth_mv: f64.
    pub max_wpe_dvth_uv: i32,
    /// Max LOD-induced drain-current mismatch, hundredths of a percent.
    // was: max_lod_did_pct: f64 (%).
    pub max_lod_did_cpct: i32,
    /// Max `SA`/`SB` mismatch, `nm`.
    // was: max_sa_sb_mismatch_um: f64.
    pub max_sa_sb_mismatch_nm: i32,
    /// `SA`/`SB` OD extension for the outer fingers, `nm`.
    // was: outer_sa_um / outer_sb_um: f64.
    pub outer_sa_nm: i32,
    pub outer_sb_nm: i32,
    /// `SA`/`SB` for inner fingers (half poly-to-poly spacing), `nm`.
    // was: inner_sa_um / inner_sb_um: f64.
    pub inner_sa_nm: i32,
    pub inner_sb_nm: i32,
    /// Full STI `ΔVth` from the `k1·(1/SA_eff − 1/SB_eff)` model, µV.
    // was: sti_dvth_mv: f64.
    pub sti_dvth_uv: i32,
    /// Enforce identical OD width (passives).
    pub same_width: bool,
    /// Enforce identical orientation (passives).
    pub same_orientation: bool,
}
