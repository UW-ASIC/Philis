//! [`Report`] — how well a solution satisfies its requirements.

/// Every stage's return value; `verify` fills it at signoff. The three fields
/// are the tiers of the lexicographic objective `lex-min(V, Θ, PEX)`, compared
/// (never summed) by [`Report::lex`].
#[derive(Default)]
pub struct Report {
    /// Tier V — hard violations. Empty for a legal solution.
    pub hard_violations: Vec<Violation>,
    /// Tier Θ — budget violations; `margin` is the live normalised residual.
    pub budget_violations: Vec<Violation>,
    /// Tier PEX — objective, lower is better.
    pub cost: f32,
}

impl Report {
    /// `(|V|, Σ Θ residual, PEX)`; derived tuple ordering gives tier precedence.
    #[must_use]
    pub fn lex(&self) -> (usize, f64, f32) {
        let theta = self.budget_violations.iter().map(|v| v.margin as f64).sum();
        (self.hard_violations.len(), theta, self.cost)
    }

    /// Both violation tiers empty.
    #[must_use]
    pub fn feasible(&self) -> bool {
        self.hard_violations.is_empty() && self.budget_violations.is_empty()
    }
}

/// One violated requirement.
pub struct Violation {
    /// Rule identity, e.g. `"spacing met1"`.
    pub rule: String,
    /// Severity (nm shortfall, or milli-budgets from [`Violation::from_residual`]).
    /// `0` means boolean fail — and reads as satisfied to [`Report::lex`].
    pub margin: i64,
}

impl Violation {
    /// Prefix of a stage row that restates a whole rule batch; the epoch key
    /// counts those per rule from `metadata`, not per row.
    pub const BATCH: &'static str = "batch:";

    /// `rule` starts with [`Violation::BATCH`].
    #[must_use]
    pub fn is_batch_row(&self) -> bool {
        self.rule.starts_with(Self::BATCH)
    }

    /// From a residual normalised by its own budget (`0.5` = 50% over), stored
    /// in milli-budgets. Every stage must use this so Θ sums in one unit; `ceil`
    /// so a real violation never rounds to `0`.
    #[must_use]
    pub fn from_residual(rule: impl Into<String>, residual: f64) -> Self {
        Self { rule: rule.into(), margin: (residual * 1000.0).ceil() as i64 }
    }
}
