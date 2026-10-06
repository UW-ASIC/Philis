//! Precision class and match kind per matched set (EXT-16; Hastings §13.3,
//! §10.3, §8.3 class tables via `analog::matching::class::limit`).

use analog::intent::{ArrayStyle, ClassSource, Diagnostic, MatchClass, MatchKind, MatchSpec};
use analog::matching::class::{limit, ClassLimit};
use pnr_core::netlist::DeviceKind;

use crate::block::BlockKind;

/// What a set does in the circuit: the evidence of last resort for its class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetRole {
    /// The input differential pair (role default Moderate).
    InputPair,
    /// The load of an input pair (Moderate).
    LoadOfPair,
    /// A bias current mirror (Minimal).
    BiasMirror,
    /// A bandgap core's ratioed devices or resistors (Moderate).
    BandgapCore,
    /// A binary-weighted DAC bank (Exceptional).
    DacBank,
    /// A feedback divider or capacitor ratio (Moderate).
    FeedbackRatio,
    /// Anything else (Minimal).
    Other,
}

/// Inputs to [`class_of`]; the family is the set's own.
pub struct ClassCtx<'a> {
    /// Sidecar class (EXT-26, M3); `None` until then.
    pub user: Option<MatchClass>,
    /// 6σ target in the unit of `limit(family, kind, _)`: `6·offset_sigma_mv` today.
    pub spec_6sigma: Option<f32>,
    /// The set's circuit role, the fallback when neither user nor spec decides.
    pub role: SetRole,
    /// Receives `beyond_exceptional_trim` when the spec needs trim.
    pub diags: &'a mut Vec<Diagnostic>,
}

/// Voltage for a diff pair or cross-coupled couple (a `DiffPair` leaf) and a
/// BJT ratioed pair (ΔV_BE); Ratio for resistors and capacitors; Current
/// otherwise (mirror, load, cascode pair, shared bias). `leaf_kinds` are the
/// kinds of the leaves whose both devices are in the set.
#[must_use]
pub fn kind_of(set: &MatchSpec, leaf_kinds: &[BlockKind], dk: DeviceKind) -> MatchKind {
    let bjt_ratio = matches!(set.origin, analog::intent::Origin::Pattern { template } if template.starts_with("bjt_ratioed_pair") || template.starts_with("bjt_diff_pair"));
    if matches!(dk, DeviceKind::Resistor | DeviceKind::Capacitor) {
        MatchKind::Ratio
    } else if leaf_kinds.contains(&BlockKind::DiffPair) || bjt_ratio {
        MatchKind::Voltage
    } else {
        MatchKind::Current
    }
}

/// First rule that applies: User; Spec, only where `limit` is in mV (card D-i:
/// a % limit needs EXT-21's allowance), Minimal at `X ≥ lim(Minimal)`, Moderate
/// at `lim(Moderate) ≤ X`, else Exceptional, with `beyond_exceptional_trim`
/// below `lim(Exceptional)` (Hastings: that needs trim); Role defaults
/// (InputPair, LoadOfPair, BandgapCore, FeedbackRatio Moderate; DacBank
/// Exceptional; BiasMirror, Other Minimal). A NaN spec counts as no spec.
pub fn class_of(set: &MatchSpec, ctx: &mut ClassCtx) -> (MatchClass, ClassSource) {
    if let Some(c) = ctx.user {
        return (c, ClassSource::User);
    }
    let mv = |c| match limit(set.family, set.kind, c) {
        Some(ClassLimit::Mv(v)) => Some(v),
        _ => None,
    };
    if let (Some(x), Some(min), Some(moderate), Some(exc)) = (ctx.spec_6sigma, mv(MatchClass::Minimal), mv(MatchClass::Moderate), mv(MatchClass::Exceptional)) {
        let c = if x >= min {
            MatchClass::Minimal
        } else if x >= moderate {
            MatchClass::Moderate
        } else {
            MatchClass::Exceptional
        };
        if x < exc {
            ctx.diags.push(Diagnostic {
                kind: "beyond_exceptional_trim",
                devices: set.members.iter().map(|m| m.device).collect(),
                message: format!("6σ target {x} mV is below the exceptional class's {exc} mV: needs trim"),
            });
        }
        return (c, ClassSource::Spec);
    }
    let c = match ctx.role {
        SetRole::InputPair | SetRole::LoadOfPair | SetRole::BandgapCore | SetRole::FeedbackRatio => MatchClass::Moderate,
        SetRole::DacBank => MatchClass::Exceptional,
        SetRole::BiasMirror | SetRole::Other => MatchClass::Minimal,
    };
    (c, ClassSource::Role)
}

/// Hastings rule 8: Minimal adjacent; Moderate interdigitated for a Current set
/// sharing a source, common-centroid in one row otherwise; Exceptional
/// common-centroid in two dimensions. CELL may override by metric (CC-40).
#[must_use]
pub fn style_of(class: MatchClass, kind: MatchKind, shares_source: bool) -> ArrayStyle {
    match class {
        MatchClass::Minimal => ArrayStyle::Adjacent,
        MatchClass::Moderate if kind == MatchKind::Current && shares_source => ArrayStyle::Interdigitated,
        MatchClass::Moderate => ArrayStyle::CommonCentroid1d,
        MatchClass::Exceptional => ArrayStyle::CommonCentroid2d,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::intent::{ConstraintId, Family, Member, Origin};
    use pnr_core::ids::DeviceId;

    fn set(family: Family, kind: MatchKind) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: vec![Member { device: DeviceId(0), parallel: 1, series: 1, half: None }],
            reference: None,
            family,
            kind,
            class: MatchClass::Moderate,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn run(s: &MatchSpec, user: Option<MatchClass>, spec: Option<f32>, role: SetRole) -> (MatchClass, ClassSource, Vec<&'static str>) {
        let mut diags = Vec::new();
        let (c, src) = class_of(s, &mut ClassCtx { user, spec_6sigma: spec, role, diags: &mut diags });
        (c, src, diags.iter().map(|d| d.kind).collect())
    }

    #[test]
    fn spec_maps_to_class() {
        use MatchClass::{Exceptional, Minimal, Moderate};
        let s = set(Family::Mos, MatchKind::Voltage);
        assert_eq!(run(&s, None, Some(12.0), SetRole::Other), (Minimal, ClassSource::Spec, vec![]));
        assert_eq!(run(&s, None, Some(5.0), SetRole::Other), (Moderate, ClassSource::Spec, vec![]));
        assert_eq!(run(&s, None, Some(2.0), SetRole::Other), (Exceptional, ClassSource::Spec, vec![]));
        assert_eq!(run(&s, None, Some(0.5), SetRole::Other), (Exceptional, ClassSource::Spec, vec!["beyond_exceptional_trim"]));
    }

    #[test]
    fn user_wins_over_spec_and_role() {
        let s = set(Family::Mos, MatchKind::Voltage);
        assert_eq!(run(&s, Some(MatchClass::Minimal), Some(2.0), SetRole::InputPair), (MatchClass::Minimal, ClassSource::User, vec![]));
    }

    #[test]
    fn current_kind_ignores_mv_spec() {
        let s = set(Family::Mos, MatchKind::Current);
        assert_eq!(run(&s, None, Some(2.0), SetRole::BiasMirror), (MatchClass::Minimal, ClassSource::Role, vec![]));
    }

    #[test]
    fn style_by_class() {
        assert_eq!(style_of(MatchClass::Minimal, MatchKind::Current, true), ArrayStyle::Adjacent);
        assert_eq!(style_of(MatchClass::Moderate, MatchKind::Current, true), ArrayStyle::Interdigitated);
        assert_eq!(style_of(MatchClass::Moderate, MatchKind::Current, false), ArrayStyle::CommonCentroid1d);
        assert_eq!(style_of(MatchClass::Exceptional, MatchKind::Voltage, false), ArrayStyle::CommonCentroid2d);
    }
}

/// Step-2 coverage: every kind/class/style rule and its boundaries.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use analog::intent::{ConstraintId, Family, Member, Origin};
    use pnr_core::ids::DeviceId;
    use MatchClass::{Exceptional, Minimal, Moderate};

    fn set(origin: Origin, family: Family, kind: MatchKind, members: &[u16]) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin,
            members: members.iter().map(|&d| Member { device: DeviceId(d), parallel: 1, series: 1, half: None }).collect(),
            reference: None,
            family,
            kind,
            class: Moderate,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn mos_v() -> MatchSpec {
        set(Origin::SharedBias, Family::Mos, MatchKind::Voltage, &[3, 8])
    }

    fn run(s: &MatchSpec, user: Option<MatchClass>, spec: Option<f32>, role: SetRole) -> (MatchClass, ClassSource, Vec<Diagnostic>) {
        let mut diags = Vec::new();
        let (c, src) = class_of(s, &mut ClassCtx { user, spec_6sigma: spec, role, diags: &mut diags });
        (c, src, diags)
    }

    #[test]
    fn passives_are_ratio_whatever_their_leaves() {
        let s = mos_v();
        for dk in [DeviceKind::Resistor, DeviceKind::Capacitor] {
            assert_eq!(kind_of(&s, &[BlockKind::DiffPair], dk), MatchKind::Ratio);
        }
    }

    #[test]
    fn a_diff_pair_leaf_or_bjt_ratio_template_is_voltage() {
        let s = mos_v();
        assert_eq!(kind_of(&s, &[BlockKind::Load, BlockKind::DiffPair], DeviceKind::Nmos), MatchKind::Voltage);
        for t in ["bjt_ratioed_pair", "bjt_ratioed_pair_8x", "bjt_diff_pair"] {
            let b = set(Origin::Pattern { template: t }, Family::Bipolar, MatchKind::Current, &[0, 1]);
            assert_eq!(kind_of(&b, &[], DeviceKind::Pnp), MatchKind::Voltage, "{t}");
        }
        // The template must lead with the prefix.
        let b = set(Origin::Pattern { template: "x_bjt_diff_pair" }, Family::Bipolar, MatchKind::Current, &[0, 1]);
        assert_eq!(kind_of(&b, &[], DeviceKind::Npn), MatchKind::Current);
    }

    #[test]
    fn everything_else_is_current() {
        let s = mos_v();
        for leaves in [&[][..], &[BlockKind::CurrentMirror], &[BlockKind::Load, BlockKind::CascodePair, BlockKind::Stack]] {
            assert_eq!(kind_of(&s, leaves, DeviceKind::Pmos), MatchKind::Current);
        }
        assert_eq!(kind_of(&s, &[], DeviceKind::Npn), MatchKind::Current);
    }

    /// MOS voltage limits are 10 / 3 / 1 mV: each threshold belongs to the looser class.
    #[test]
    fn spec_thresholds_are_inclusive() {
        let s = mos_v();
        let class = |x: f32| {
            let (c, src, d) = run(&s, None, Some(x), SetRole::DacBank);
            assert_eq!(src, ClassSource::Spec);
            (c, d.len())
        };
        assert_eq!(class(f32::INFINITY), (Minimal, 0));
        assert_eq!(class(10.0), (Minimal, 0));
        assert_eq!(class(9.99), (Moderate, 0));
        assert_eq!(class(3.0), (Moderate, 0));
        assert_eq!(class(2.99), (Exceptional, 0));
        assert_eq!(class(1.0), (Exceptional, 0), "at the exceptional limit: no trim");
        assert_eq!(class(0.99), (Exceptional, 1));
        assert_eq!(class(0.0), (Exceptional, 1));
    }

    #[test]
    fn the_trim_diagnostic_names_every_member() {
        let (_, _, d) = run(&mos_v(), None, Some(0.1), SetRole::Other);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, "beyond_exceptional_trim");
        assert_eq!(d[0].devices, [DeviceId(3), DeviceId(8)]);
    }

    /// A NaN spec is no evidence: the role decides.
    #[test]
    fn a_nan_spec_falls_back_to_the_role() {
        let (c, src, d) = run(&mos_v(), None, Some(f32::NAN), SetRole::InputPair);
        assert_eq!((c, src, d.len()), (Moderate, ClassSource::Role, 0));
    }

    /// Bipolar voltage limits are 2 / 0.5 / 0.1 mV (Hastings §9).
    #[test]
    fn bipolar_voltage_uses_its_own_table() {
        let b = set(Origin::SharedBias, Family::Bipolar, MatchKind::Voltage, &[0, 1]);
        assert_eq!(run(&b, None, Some(2.0), SetRole::Other).0, Minimal);
        assert_eq!(run(&b, None, Some(0.5), SetRole::Other).0, Moderate);
        let (c, _, d) = run(&b, None, Some(0.05), SetRole::Other);
        assert_eq!((c, d.len()), (Exceptional, 1));
    }

    /// A % table (or no table) ignores the mV spec.
    #[test]
    fn a_spec_without_an_mv_table_is_ignored() {
        for s in [
            set(Origin::SharedBias, Family::Resistor, MatchKind::Ratio, &[0, 1]),
            set(Origin::SharedBias, Family::Diode, MatchKind::Current, &[0, 1]),
            set(Origin::SharedBias, Family::Mos, MatchKind::Ratio, &[0, 1]),
        ] {
            let (c, src, d) = run(&s, None, Some(0.01), SetRole::BiasMirror);
            assert_eq!((c, src, d.len()), (Minimal, ClassSource::Role, 0));
        }
    }

    #[test]
    fn role_defaults() {
        use SetRole::*;
        for (r, c) in [(InputPair, Moderate), (LoadOfPair, Moderate), (BandgapCore, Moderate), (FeedbackRatio, Moderate), (DacBank, Exceptional), (BiasMirror, Minimal), (Other, Minimal)] {
            assert_eq!(run(&mos_v(), None, None, r).0, c, "{r:?}");
            assert_eq!(run(&mos_v(), None, None, r).1, ClassSource::Role);
        }
    }

    /// The user's class wins and suppresses the trim diagnostic.
    #[test]
    fn user_class_reports_nothing() {
        let (c, src, d) = run(&mos_v(), Some(Exceptional), Some(0.01), SetRole::Other);
        assert_eq!((c, src, d.len()), (Exceptional, ClassSource::User, 0));
    }

    /// Hastings rule 8 as a table.
    #[test]
    fn style_table() {
        use ArrayStyle::*;
        use MatchKind::{Current, Ratio, Voltage};
        let cases = [
            (Minimal, Current, true, Adjacent),
            (Minimal, Voltage, false, Adjacent),
            (Moderate, Current, true, Interdigitated),
            (Moderate, Current, false, CommonCentroid1d),
            (Moderate, Voltage, true, CommonCentroid1d),
            (Moderate, Ratio, true, CommonCentroid1d),
            (Exceptional, Current, true, CommonCentroid2d),
            (Exceptional, Ratio, false, CommonCentroid2d),
        ];
        for (c, k, shares, want) in cases {
            assert_eq!(style_of(c, k, shares), want, "{c:?} {k:?} {shares}");
        }
    }
}
