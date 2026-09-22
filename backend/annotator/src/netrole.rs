//! Net-role classification + recognition config.
//!
//! Ported from `frontend/annotator/src/lib.rs`. A net's role (supply / ground /
//! clock / signal) gates recognition: the DSL's `gate_is_signal` slot flag keys
//! off it, and it is the basis for MAGICAL's virtual-ground disambiguation
//! (a shared-source pair is a differential pair only when the shared source is a
//! signal node, not a power/ground rail — research §4.2.1).
//!
//! The frontend pulled the rail/clock name lists from `pnr_constraints`; here they
//! are inlined so the annotator carries its own heuristic and needs no PDK handle.

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
const CLK_PATTERNS: &[&str] = &["clk", "clock", "phi", "ck"];

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
            } else if named(&cfg.clock_nets) || CLK_PATTERNS.iter().any(|p| lower.contains(p)) {
                NetRole::Clock
            } else {
                NetRole::Signal
            }
        })
        .collect()
}

/// Whether a net is a power/ground rail (not a signal node). The virtual-ground
/// gate: a differential pair's shared source must **not** be a rail.
#[must_use]
pub fn is_rail(role: NetRole) -> bool {
    matches!(role, NetRole::Supply | NetRole::Ground)
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
}
