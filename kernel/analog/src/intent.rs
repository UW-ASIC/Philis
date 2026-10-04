//! Batch identity and provenance (EXT-10): which constraint a [`crate::RuleBatch`]
//! is, independent of where it sits in its arm, and where it came from.

/// Dense id within one `annotator::Problem`, assigned in emission order:
/// permutation-invariant because emission order is (EXT-06).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ConstraintId(pub u32);

/// Where a batch came from (EXT-12 widens this).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    /// Emitted for a recognised block of this template.
    Pattern { template: &'static str },
    /// Emitted from net classes, the substrate, or a cross-block rule.
    NetClass,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BatchMeta {
    pub id: ConstraintId,
    pub origin: Origin,
}
