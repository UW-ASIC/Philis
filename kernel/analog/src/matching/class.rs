//! The matching class's one home (GAP-01): the device family a matched set
//! belongs to, the environment each class asks of the layout (read from the
//! sidecar's `[MIN, MOD, EXC]` tier arrays) and Hastings' class limit table.

pub use crate::matching::mismatch::MatchKind;
pub use pnr_core::MatchClass;

use pnr_core::{DeviceKind, Process};

/// The device family a class limit is tabulated for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    Mos,
    Bipolar,
    Diode,
    Resistor,
    Capacitor,
}

impl Family {
    /// `None` for an inductor: no class table, never matched.
    #[must_use]
    pub fn of(k: DeviceKind) -> Option<Family> {
        match k {
            DeviceKind::Nmos | DeviceKind::Pmos => Some(Family::Mos),
            DeviceKind::Npn | DeviceKind::Pnp => Some(Family::Bipolar),
            DeviceKind::Diode => Some(Family::Diode),
            DeviceKind::Resistor => Some(Family::Resistor),
            DeviceKind::Capacitor => Some(Family::Capacitor),
            DeviceKind::Inductor => None,
        }
    }
}

/// How matched gates are strapped: MIN a poly bar, MOD a poly bar ≥ 1 µm
/// from the gates (CELL-11 step 4), EXC each gate contacted in metal with
/// no shared poly (Hastings §13.3 rule 22, hastings.txt L42638–42647).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GateStrap {
    #[default]
    PolyBar,
    PolyBarFar,
    MetalIsolated,
}

/// What a MOS matched set of one class asks of its surroundings, nm.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MosEnv {
    /// Outer dummy poly edge to the nearest active gate (rule 12).
    pub dummy_reach_nm: i32,
    /// Diffusion run past the outer dummy (LOD moat, §13.2.2 and rule 12).
    pub moat_nm: i32,
    /// Well edge to the matched gates (rule 19).
    pub wpe_nm: i32,
    /// Gate poly extension beyond the deck rule (rule 21).
    pub gate_ext_extra_nm: i32,
    pub gate_strap: GateStrap,
}

/// `c`'s MOS environment from `p`'s tiers; a missing tier reads 0 (and
/// [`missing_tiers`] names it).
#[must_use]
pub fn mos_env(c: MatchClass, p: &dyn Process) -> MosEnv {
    let t = |k| p.tier(k, c).unwrap_or(0);
    MosEnv {
        dummy_reach_nm: t("dummy_reach_nm"),
        moat_nm: t("lod_moat_ext_nm"),
        wpe_nm: t("wpe_clearance_nm"),
        gate_ext_extra_nm: t("gate_ext_extra_nm"),
        gate_strap: match c {
            MatchClass::Minimal => GateStrap::PolyBar,
            MatchClass::Moderate => GateStrap::PolyBarFar,
            MatchClass::Exceptional => GateStrap::MetalIsolated,
        },
    }
}

/// What a matched resistor array of one class asks (Hastings §8, H08-13,
/// H08-41).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PassiveEnv {
    /// Dummies per array end: one on every class; EXC also spans
    /// `dummy_span_nm`, so its count is `ceil(span / pitch)` (the caller's).
    pub min_dummies: u8,
    pub dummy_span_nm: i32,
    /// Segment width floor, ‰ of the deck's minimum width.
    pub width_floor_permille: i32,
    /// Segment length floor, multiples of the deck's minimum length.
    pub length_floor_x: i32,
}

/// `c`'s resistor environment from `p`'s tiers; a missing tier reads 0.
#[must_use]
pub fn resistor_env(c: MatchClass, p: &dyn Process) -> PassiveEnv {
    let t = |k| p.tier(k, c).unwrap_or(0);
    PassiveEnv {
        min_dummies: 1,
        dummy_span_nm: t("res_dummy_span_nm"),
        width_floor_permille: t("res_width_floor_permille"),
        length_floor_x: t("res_length_floor_x"),
    }
}

/// Every tier key the environments read, with its `Problem.missing` text.
const TIERS: [(&str, &str); 7] = [
    ("dummy_reach_nm", "dummy_reach_nm tier missing"),
    ("gate_ext_extra_nm", "gate_ext_extra_nm tier missing"),
    ("lod_moat_ext_nm", "lod_moat_ext_nm tier missing"),
    ("res_dummy_span_nm", "res_dummy_span_nm tier missing"),
    ("res_length_floor_x", "res_length_floor_x tier missing"),
    ("res_width_floor_permille", "res_width_floor_permille tier missing"),
    ("wpe_clearance_nm", "wpe_clearance_nm tier missing"),
];

/// `"<key> tier missing"` for each tier key `p` lacks on any class.
pub fn missing_tiers(p: &dyn Process) -> impl Iterator<Item = &'static str> + '_ {
    let classes = [MatchClass::Minimal, MatchClass::Moderate, MatchClass::Exceptional];
    TIERS.iter().filter(move |(k, _)| classes.iter().any(|&c| p.tier(k, c).is_none())).map(|(_, m)| *m)
}

/// A class limit: an offset in mV or a mismatch in %, both 6σ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClassLimit {
    Mv(f32),
    Pct(f32),
}

/// Hastings' class limit for `(f, k)` at `c` (C2): MOS §13.3 (hastings.txt
/// L42327–42350), bipolar §9 (L31244–31275), R/C ratio §8 (L25121–25133,
/// the tight end of each range). `None` where the book gives none (a diode
/// has ideality limits, not offset limits) or the pair is meaningless.
#[must_use]
pub fn limit(f: Family, k: MatchKind, c: MatchClass) -> Option<ClassLimit> {
    use ClassLimit::{Mv, Pct};
    let i = c as usize;
    Some(match (f, k) {
        (Family::Mos, MatchKind::Voltage) => Mv([10.0, 3.0, 1.0][i]),
        (Family::Mos, MatchKind::Current) => Pct([10.0, 3.0, 1.0][i]),
        (Family::Bipolar, MatchKind::Voltage) => Mv([2.0, 0.5, 0.1][i]),
        (Family::Bipolar, MatchKind::Current) => Pct([8.0, 2.0, 0.5][i]),
        (Family::Resistor | Family::Capacitor, MatchKind::Ratio) => Pct([1.0, 0.1, 0.01][i]),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ClassLimit::{Mv, Pct};
    use MatchClass::{Exceptional as Exc, Minimal as Min, Moderate as Mod};

    /// GAP-01's tier table, less `dummy_reach_nm`.
    struct NoReach;
    impl Process for NoReach {
        fn layer(&self, _: &str) -> Option<pnr_core::LayerId> {
            None
        }
        fn rule(&self, _: &str, default: i32) -> i32 {
            default
        }
        fn grid(&self) -> i32 {
            1
        }
        fn tier(&self, key: &str, c: MatchClass) -> Option<i32> {
            let a = match key {
                "wpe_clearance_nm" => [2000, 3000, 5000],
                "lod_moat_ext_nm" => [3000, 5000, 10000],
                "gate_ext_extra_nm" => [0, 1000, 1000],
                "res_dummy_span_nm" => [0, 0, 10000],
                "res_width_floor_permille" => [1500, 2000, 4000],
                "res_length_floor_x" => [3, 5, 10],
                _ => return None,
            };
            Some(a[c as usize])
        }
    }

    #[test]
    fn missing_tier_reads_zero_and_is_reported() {
        let e = mos_env(Mod, &NoReach);
        assert_eq!(e, MosEnv { dummy_reach_nm: 0, moat_nm: 5000, wpe_nm: 3000, gate_ext_extra_nm: 1000, gate_strap: GateStrap::PolyBarFar });
        assert_eq!(missing_tiers(&NoReach).collect::<Vec<_>>(), ["dummy_reach_nm tier missing"]);
    }

    #[test]
    fn limits_match_hastings() {
        let cases = [
            (Family::Mos, MatchKind::Voltage, [Mv(10.0), Mv(3.0), Mv(1.0)]),
            (Family::Mos, MatchKind::Current, [Pct(10.0), Pct(3.0), Pct(1.0)]),
            (Family::Bipolar, MatchKind::Voltage, [Mv(2.0), Mv(0.5), Mv(0.1)]),
            (Family::Bipolar, MatchKind::Current, [Pct(8.0), Pct(2.0), Pct(0.5)]),
            (Family::Resistor, MatchKind::Ratio, [Pct(1.0), Pct(0.1), Pct(0.01)]),
            (Family::Capacitor, MatchKind::Ratio, [Pct(1.0), Pct(0.1), Pct(0.01)]),
        ];
        for (f, k, want) in cases {
            for (c, w) in [Min, Mod, Exc].into_iter().zip(want) {
                assert_eq!(limit(f, k, c), Some(w), "{f:?} {k:?} {c:?}");
            }
        }
        for c in [Min, Mod, Exc] {
            assert_eq!(limit(Family::Diode, MatchKind::Voltage, c), None);
            assert_eq!(limit(Family::Mos, MatchKind::Ratio, c), None);
        }
    }

    #[test]
    fn family_of_kind() {
        assert_eq!(Family::of(DeviceKind::Nmos), Some(Family::Mos));
        assert_eq!(Family::of(DeviceKind::Pnp), Some(Family::Bipolar));
        assert_eq!(Family::of(DeviceKind::Inductor), None);
    }
}
