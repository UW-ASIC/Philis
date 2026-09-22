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
