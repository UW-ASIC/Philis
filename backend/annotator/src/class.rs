//! Precision class and match kind per matched set (EXT-16; Hastings §13.3,
//! §10.3, §8.3 class tables via `analog::matching::class::limit`).

use analog::intent::{ArrayStyle, ClassSource, Diagnostic, MatchClass, MatchKind, MatchSpec};
use analog::matching::class::{limit, ClassLimit};
use pnr_core::netlist::DeviceKind;

use crate::block::BlockKind;

/// What a set does in the circuit: the evidence of last resort for its class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetRole {
    InputPair,
    LoadOfPair,
    BiasMirror,
    BandgapCore,
    DacBank,
    FeedbackRatio,
    Other,
}

/// Inputs to [`class_of`]; the family is the set's own.
pub struct ClassCtx<'a> {
    /// Sidecar class (EXT-26, M3); `None` until then.
    pub user: Option<MatchClass>,
    /// 6σ target in the unit of `limit(family, kind, _)`: `6·offset_sigma_mv` today.
    pub spec_6sigma: Option<f32>,
    pub role: SetRole,
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
/// Exceptional; BiasMirror, Other Minimal).
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
