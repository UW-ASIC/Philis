//! Net role by name (supply / ground / clock / signal) + user recognition
//! overrides. Gates recognition: a diff pair's gates must be signals.

use std::collections::HashSet;

use pnr_core::BipartiteHypergraph;

/// Electrical role of a net, keyed off its name. Index the returned `Vec` by
/// [`pnr_core::ids::NetId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRole {
    Signal,
    Supply,
    Ground,
    Clock,
}

const SUPPLY_NAMES: &[&str] = &["vdd", "vcc", "vpwr", "vddio", "avdd", "dvdd", "vdda", "vddd"];
const GROUND_NAMES: &[&str] = &["vss", "gnd", "vgnd", "vssio", "avss", "dvss", "vssa", "vssd"];
/// Substrings that mark a clock anywhere in the name.
const CLK_SUBSTR: &[&str] = &["clk", "clock"];
/// Clock prefixes that must be followed by digits/`_`/`b`/end (`phi1`, `ck_b`),
/// so `back`/`stack`/`phase` stay signals.
const CLK_PREFIX: &[&str] = &["phi", "ck"];

/// Classify every net by name. Config-supplied names win over the built-in lists.
#[must_use]
pub fn classify_nets(hg: &BipartiteHypergraph, cfg: &AnnotationConfig) -> Vec<NetRole> {
    hg.net_names
        .iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            let named = |set: &[String]| set.iter().any(|s| s.eq_ignore_ascii_case(name));
            let matches_rail = |pats: &[&str]| {
                pats.iter().any(|p| lower == *p || lower.starts_with(&format!("{p}_")))
            };
            if named(&cfg.supply_nets) || matches_rail(SUPPLY_NAMES) {
                NetRole::Supply
            } else if named(&cfg.ground_nets) || matches_rail(GROUND_NAMES) {
                NetRole::Ground
            } else if named(&cfg.clock_nets) || is_clock(&lower) {
                NetRole::Clock
            } else {
                NetRole::Signal
            }
        })
        .collect()
}

fn is_clock(lower: &str) -> bool {
    CLK_SUBSTR.iter().any(|p| lower.contains(p))
        || CLK_PREFIX.iter().any(|p| {
            lower.strip_prefix(p).is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit() || c == '_' || c == 'b'))
        })
}

/// User overrides on recognition — the library's channel to override our
/// decisions (ALIGN `DoNotIdentify` / `SameTemplate` semantics).
#[derive(Debug, Clone, Default)]
pub struct AnnotationConfig {
    /// Device ids the recogniser must never place in any block (ALIGN
    /// `DoNotIdentify`). The library resolves user device names to ids.
    pub do_not_identify: HashSet<u32>,
    /// Pattern template names to skip entirely.
    pub do_not_use: HashSet<String>,
    /// Extra nets to force to each role (name match, case-insensitive).
    pub supply_nets: Vec<String>,
    pub ground_nets: Vec<String>,
    pub clock_nets: Vec<String>,
    /// Process numbers from the deck; `library::annotation` fills them.
    pub process: ProcessNumbers,
    /// 1σ input-referred offset a matched pair may spend, mV (from the
    /// circuit's spec). Sets how much of it placement gradients may take.
    pub offset_sigma_mv: Option<f32>,
}

/// Every process number the annotator uses. A `None` means the deck does not
/// carry it, and the constraints that need it are not emitted.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessNumbers {
    /// Tightest antenna ratio (metal / gate area) on routed metal.
    pub antenna_max_ratio: Option<f32>,
    /// Gate capacitance per gate area, aF/µm²: the load a net drives.
    pub gate_af_per_um2: Option<f32>,
    /// Ground capacitance of a minimum-width lowest routing wire, aF/µm.
    pub wire_af_per_um: Option<f32>,
    /// Lowest routing metal's min spacing, nm; crosstalk spacings are multiples.
    pub route_space_nm: i32,
    /// Deep-trench isolation: (max spacing sharing one trench, trench width), nm.
    pub dti: Option<(i32, i32)>,
    /// Pelgrom `A_VT` (ΔVT of a pair), mV·µm, `[nmos, pmos]`.
    pub avt_mv_um: [Option<f32>; 2],
    /// Pelgrom distance coefficient `S_VT`, µV/µm. Process-specific and rarely
    /// published: absent leaves the matching distance check unknown.
    pub svt_uv_per_um: Option<f32>,
    /// |dVT/dT|, µV/K, `[nmos, pmos]`: turns a matched pair's offset allowance
    /// into a ΔT limit.
    pub vt_tc_uv_per_k: [Option<f32>; 2],
    /// BSIM4 LOD `KVTH0` (ΔVT per unit `Δ(1/SA + 1/SB)`), mV·µm, `[nmos,
    /// pmos]`: prices LOD imbalance across a matched array.
    pub lod_kvth0_mv_um: [Option<f32>; 2],
    /// What the active area sits on; only `EpiOnLowRes` gives isolation a
    /// calibrated distance.
    pub substrate: pnr_core::SubstrateKind,
    /// Epitaxial layer thickness, nm: on `EpiOnLowRes` substrate isolation
    /// saturates at a few times it. Unread on any other kind.
    pub epi_nm: Option<i32>,
    /// The routing stack's per-layer parasitics and antenna stages; `None`
    /// leaves the routing budgets on drawn length and the cumulative antenna.
    pub stack: Option<&'static analog::routing::Stack>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn clock_names() {
        for n in ["clk", "clk_in", "phi1", "phi_2b", "ck", "ckb", "sysclock"] {
            assert!(super::is_clock(n), "{n}");
        }
        for n in ["back", "stack", "phase", "lock", "vbias"] {
            assert!(!super::is_clock(n), "{n}");
        }
    }
}
