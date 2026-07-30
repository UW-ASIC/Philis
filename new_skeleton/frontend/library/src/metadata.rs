//! What the constraint system actually achieved — the **budget report**.
//!
//! `Report.hard_violations` answers "is it legal", which is a yes/no. It cannot
//! say *how close to the edge* a legal solution sits, and for analog that is the
//! interesting question: a net one nanometre inside its coupling limit and a net
//! with 60% headroom are both "legal" and are not remotely the same layout.
//!
//! Every budget rule already computes its own headroom against its safety margin
//! (`analog::Rule::headroom`/`margin`, used by the slack-blended objective). This
//! module simply surfaces that, so a run reports **met, met-with-margin, or
//! violated** per constraint family instead of a bare violation count. See
//! `backend/TODO.md` §5b.

use analog::{Requirements, RuleBatch};

/// Which `Requirements` arm a status row was measured from.
///
/// Carried on the row because [`MetadataReport::theta`] must sum **budget-arm**
/// residuals only: a hard batch's residual is Φ's business (`Report::phi`), and
/// folding it into Θ would count legality twice and let a budget trade against a
/// hard rule inside one tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// `reqs.hard` — legality; a violation here is V-tier, not Θ.
    Hard,
    /// `reqs.budget` — priced allowances; residuals here *are* Θ.
    Budget,
}

/// How one constraint family came out.
#[derive(Clone, Debug)]
pub struct BudgetStatus {
    /// Rule kind, path-trimmed (`ThermalGradient`, `CrosstalkExclusion`, …).
    pub kind: String,
    /// Which arm the family was registered in.
    pub arm: Arm,
    /// Rules of this kind in the circuit.
    pub total: usize,
    /// How many are satisfied against the **raw** spec.
    pub satisfied: usize,
    /// Tightest rule's criticality, `0.0` (slack to spare) … `1.0` (at or past
    /// the spec). Derived from headroom against the family's safety margin.
    pub criticality: f32,
    /// The family's **measured** overshoot: `RuleBatch::residual`, summed over
    /// same-kind batches — each batch's residual is already normalised by its own
    /// budget, so the sum is dimensionless and summable across families (D17).
    /// `0.0` means every member is inside spec; `0.5` means half a budget's worth
    /// of overshoot across the family.
    pub residual: f64,
}

impl BudgetStatus {
    /// Every rule satisfied against the raw spec.
    #[must_use]
    pub fn met(&self) -> bool {
        self.satisfied == self.total
    }

    /// Satisfied *and* still inside the safety margin — the state a converged
    /// run is supposed to reach, not merely "not yet illegal".
    #[must_use]
    pub fn met_with_margin(&self) -> bool {
        self.met() && self.criticality <= 0.0
    }

    /// One-word verdict for a report line.
    #[must_use]
    pub fn verdict(&self) -> &'static str {
        if !self.met() {
            "VIOLATED"
        } else if self.met_with_margin() {
            "met"
        } else {
            "met (no margin)"
        }
    }
}

/// Budget status for every family, plus the electrical bias it was judged under.
#[derive(Clone, Debug, Default)]
pub struct MetadataReport {
    pub placement: Vec<BudgetStatus>,
    pub routing: Vec<BudgetStatus>,
    /// Where the operating point came from, and what it showed. `None` when no
    /// simulation ran — in which case any thermal result is vacuous and this
    /// report says so rather than implying a pass.
    pub bias: Option<BiasSummary>,
    /// Net census by class — how many nets the classifier judged Sensitive,
    /// Supply, Signal, … Every routing budget is keyed to one of these, so this
    /// is the provenance of the numbers in the table above.
    pub net_classes: Vec<(String, usize)>,
}

impl MetadataReport {
    /// **Θ** — the analog budget residual, PLAN §3b's middle lexicographic tier.
    ///
    /// A **measured sum**, not a count: Σ of every budget-arm family's
    /// [`BudgetStatus::residual`], in **milli-budgets** (× 1000.0 — the
    /// `Violation::from_residual` scale), so it is commensurate with the `pt`/`rt`
    /// terms `library::lex_key` adds it to. A 1 nm miss and a 1 µm miss finally
    /// weigh differently here, which is what D2 wanted and a count could not say.
    /// Hard-arm rows are excluded: their violations are V-tier.
    ///
    /// Known double-weighing, deliberate: the stage reports already carry these
    /// same budget residuals (`gp::mechanics::report` and `gr::analog_tiers` fill
    /// `Report::budget_violations` from the identical `residual` calls), so
    /// `lex_key`'s `pt + rt + theta()` counts each budget-arm residual ~twice.
    /// Monotone-safe — both copies are the same measurement of the same state, so
    /// every comparison the search makes orders identically — and the cleanup
    /// (drop one source) is deferred until something reads Θ as an absolute
    /// quantity rather than a ranking key.
    ///
    /// `criticality` is deliberately **not** folded in. It is the *promotion* signal
    /// (how close a budget is to binding) and belongs in the ρ ratchet; adding it to Θ
    /// would make a satisfied-but-tight budget read as violated and the run would never
    /// terminate (D14).
    #[must_use]
    pub fn theta(&self) -> f64 {
        self.placement
            .iter()
            .chain(&self.routing)
            .filter(|b| b.arm == Arm::Budget)
            .map(|b| b.residual * 1000.0)
            .sum()
    }
}

/// The electrical conditions the layout was evaluated under.
#[derive(Clone, Debug)]
pub struct BiasSummary {
    /// How the bias was obtained (user testbench vs synthesised probe).
    pub provenance: String,
    /// Devices the simulator resolved, of the total in the netlist.
    pub resolved: usize,
    pub devices: usize,
    /// Total circuit dissipation, µW.
    pub total_power_uw: i64,
    /// Hottest device: `(name, µW)`.
    pub hottest: Option<(String, i32)>,
}

/// Collect budget status for one requirement arm. `arm` tags every row, because
/// [`MetadataReport::theta`] sums residuals from the budget arm alone.
fn statuses<S>(reqs: &[Box<dyn RuleBatch<S>>], state: &S, arm: Arm) -> Vec<BudgetStatus> {
    let mut out: Vec<BudgetStatus> = Vec::new();
    for b in reqs {
        if b.count() == 0 {
            continue;
        }
        let kind = b.kind().rsplit("::").next().unwrap_or(b.kind()).to_string();
        let total = b.count();
        let satisfied = total - b.violations(state) as usize;
        let criticality = b.criticality(state);
        let residual = b.residual(state);
        // One row per family: batches of the same kind merge, since the annotator
        // emits one batch per recognised structure. Residuals *sum* — each is
        // normalised by its own budget, so the family total stays a real measure
        // of "how many budgets' worth over" (linear, per D17; a max would hide
        // every violation but the worst).
        if let Some(e) = out.iter_mut().find(|e| e.kind == kind) {
            e.total += total;
            e.satisfied += satisfied;
            e.criticality = e.criticality.max(criticality);
            e.residual += residual;
        } else {
            out.push(BudgetStatus { kind, arm, total, satisfied, criticality, residual });
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind));
    out
}

/// Build the report from the winning iteration's state.
///
/// Scans the **hard and budget** arms of both tiers. The hard rows keep the old
/// "is it legal, and how close to the edge" view; the budget rows are what
/// [`MetadataReport::theta`] measures — and for the routing tier they make the
/// declared budgets (`ParasiticBudget`, `CouplingBudget`, `CrosstalkExclusion`)
/// visible in the report for the first time, where previously only the hard arm
/// was tabulated. `cost`-arm batches are shaping terms with no spec to violate,
/// so they have no status to report.
#[must_use]
pub fn build(
    placement: &Requirements<pnr_core::Layout>,
    layout: &pnr_core::Layout,
    routing: &Requirements<pnr_core::Routes>,
    routes: &pnr_core::Routes,
    bias: Option<BiasSummary>,
    net_classes: &[analog::metadata::NetClassification],
) -> MetadataReport {
    let census = annotator::classify::census(net_classes)
        .into_iter()
        .map(|(c, n)| (format!("{c:?}"), n))
        .collect();
    let mut p = statuses(&placement.hard, layout, Arm::Hard);
    p.extend(statuses(&placement.budget, layout, Arm::Budget));
    let mut r = statuses(&routing.hard, routes, Arm::Hard);
    r.extend(statuses(&routing.budget, routes, Arm::Budget));
    MetadataReport {
        placement: p,
        routing: r,
        bias,
        net_classes: census,
    }
}

impl std::fmt::Display for MetadataReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "── Constraint budgets ──\n")?;
        match &self.bias {
            Some(b) => {
                writeln!(f, "  bias: {}", b.provenance)?;
                writeln!(
                    f,
                    "  {} of {} devices solved · {} µW total{}",
                    b.resolved,
                    b.devices,
                    b.total_power_uw,
                    b.hottest
                        .as_ref()
                        .map(|(n, p)| format!(" · hottest {n} at {p} µW"))
                        .unwrap_or_default()
                )?;
            }
            None => writeln!(
                f,
                "  bias: NOT SIMULATED — thermal results below are vacuous (uniform die)"
            )?,
        }
        if !self.net_classes.is_empty() {
            let census: Vec<String> =
                self.net_classes.iter().map(|(c, n)| format!("{n} {c}")).collect();
            writeln!(f, "  nets: {}", census.join(", "))?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "  {:<22} {:>6} {:>5} {:>5}  {:>9}  {:>9}  {}",
            "constraint", "arm", "total", "sat", "critical", "residual", "verdict"
        )?;
        writeln!(f, "  {}", "-".repeat(75))?;
        for s in self.placement.iter().chain(self.routing.iter()) {
            writeln!(
                f,
                "  {:<22} {:>6} {:>5} {:>5}  {:>9.2}  {:>9.3}  {}",
                s.kind,
                match s.arm {
                    Arm::Hard => "hard",
                    Arm::Budget => "budget",
                },
                s.total,
                s.satisfied,
                s.criticality,
                s.residual,
                s.verdict()
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::Rule;
    use pnr_core::Routes;

    #[derive(Clone, Copy)]
    struct Budgeted {
        used: f32,
    }
    impl Rule for Budgeted {
        type On = Routes;
        fn cost(self, _: &Routes) -> f32 {
            self.used
        }
        fn satisfied(self, _: &Routes) -> bool {
            self.used <= 1.0
        }
        fn headroom(self, _: &Routes) -> f32 {
            (1.0 - self.used).clamp(0.0, 1.0)
        }
        fn margin(self) -> f32 {
            0.2
        }
        /// Overshoot past the spec, as a fraction of it — the measured quantity
        /// `theta()` sums (a real budget rule normalises by its own budget the
        /// same way).
        fn residual(self, _: &Routes) -> f32 {
            (self.used - 1.0).max(0.0)
        }
    }

    fn reqs(used: &[f32]) -> Requirements<Routes> {
        let mut r = Requirements::<Routes>::default();
        r.hard.push(Box::new(used.iter().map(|&u| Budgeted { used: u }).collect::<Vec<_>>()));
        r
    }

    fn empty_routes() -> Routes {
        Routes { wires: Vec::new() }
    }

    #[test]
    fn distinguishes_comfortable_from_barely_legal() {
        let s = &statuses(&reqs(&[0.3]).hard, &empty_routes(), Arm::Hard)[0];
        assert!(s.met() && s.met_with_margin(), "slack-rich budget is fully met");
        assert_eq!(s.verdict(), "met");

        // Legal, but inside the 20% margin — the distinction a violation count
        // cannot express.
        let s = &statuses(&reqs(&[0.9]).hard, &empty_routes(), Arm::Hard)[0];
        assert!(s.met(), "still within raw spec");
        assert!(!s.met_with_margin(), "but has eaten into the margin");
        assert_eq!(s.verdict(), "met (no margin)");

        let s = &statuses(&reqs(&[1.4]).hard, &empty_routes(), Arm::Hard)[0];
        assert!(!s.met());
        assert_eq!(s.verdict(), "VIOLATED");
    }

    #[test]
    fn family_row_follows_its_worst_member() {
        let s = &statuses(&reqs(&[0.1, 0.1, 0.95]).hard, &empty_routes(), Arm::Hard)[0];
        assert_eq!(s.total, 3);
        assert_eq!(s.satisfied, 3, "all legal");
        assert!(!s.met_with_margin(), "one member in the margin taints the family");
    }

    /// Θ is a measured milli-budget sum over the **budget** arm, not a count of
    /// unmet rules. One budget 10% over and one 200% over must read `2100.0`
    /// (`(0.1 + 2.0) × 1000`); the old count body would have said `2`, weighing a
    /// 1 nm miss the same as a 1 µm one — exactly what D2 rejects.
    #[test]
    fn theta_sums_residuals_not_counts() {
        let mut rq = Requirements::<Routes>::default();
        rq.budget.push(Box::new(vec![Budgeted { used: 1.1 }])); // 10% over ⇒ 0.1
        rq.budget.push(Box::new(vec![Budgeted { used: 3.0 }])); // 200% over ⇒ 2.0
        let report = MetadataReport {
            routing: statuses(&rq.budget, &empty_routes(), Arm::Budget),
            ..MetadataReport::default()
        };
        assert!(
            (report.theta() - 2_100.0).abs() < 0.01,
            "theta must be the milli-scaled residual sum, got {}",
            report.theta()
        );

        // Hard-arm rows never contribute: their violations are V-tier, and adding
        // them here would count legality twice.
        let hard_only = MetadataReport {
            routing: statuses(&reqs(&[3.0]).hard, &empty_routes(), Arm::Hard),
            ..MetadataReport::default()
        };
        assert_eq!(hard_only.theta(), 0.0, "hard residuals are Φ's business, not Θ's");
    }

    #[test]
    fn unsimulated_bias_is_stated_not_implied() {
        let r = MetadataReport::default();
        let text = format!("{r}");
        assert!(text.contains("NOT SIMULATED"), "must not imply a thermal pass: {text}");
        assert!(text.contains("vacuous"));
    }
}
