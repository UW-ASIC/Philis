//! The budget report: per constraint family, met, met without margin, or
//! violated — how close to the edge a legal layout sits.

use analog::{Requirements, RuleBatch};

/// Which `Requirements` arm a row was measured from. Only budget rows count
/// toward [`MetadataReport::theta`]; hard rows are legality (the V tier).
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
    /// Rule kind, path-trimmed (`MatchedSet`, `CrosstalkExclusion`, …).
    pub kind: String,
    /// Which arm the family was registered in.
    pub arm: Arm,
    /// Rules of this kind in the circuit.
    pub total: usize,
    /// How many are known and satisfied against the **raw** spec: `total −
    /// violations − unknown` (master §6.1: unknown is never pass).
    pub satisfied: usize,
    /// How many are violated ([`analog::RuleBatch::violations`]); a hard
    /// row's count is |V|.
    pub violations: usize,
    /// How many lack their inputs ([`analog::Rule::known`]): neither
    /// satisfied nor, unless the batch also counts them, violated.
    pub unknown: usize,
    /// Tightest rule's criticality, `0.0` (slack to spare) … `1.0` (at or past
    /// the spec). Derived from headroom against the family's safety margin.
    pub criticality: f32,
    /// Σ `RuleBatch::residual` over the family — each normalised by its own
    /// budget, so summable across families. `0.0` = inside spec.
    pub residual: f64,
    /// Largest spent fraction of a rule's budget (`1.0` = at the spec);
    /// `None` when no rule reports one.
    pub usage: Option<f32>,
    /// Ids the violated rules touch, sorted, deduplicated (net ids for
    /// routing, cell ids for placement), each with the largest residual of a
    /// violated rule touching it (`NaN` from a batch that has no per-rule
    /// residual, [`analog::RuleBatch::violating_residuals`]).
    pub violated: Vec<(u32, f32)>,
}

impl BudgetStatus {
    /// No rule violated against the raw spec (unknowns allowed: see
    /// [`Self::verdict`] and [`MetadataReport::certified`]).
    #[must_use]
    pub fn met(&self) -> bool {
        self.violations == 0
    }

    /// Satisfied *and* still inside the safety margin — the state a converged
    /// run is supposed to reach, not merely "not yet illegal".
    #[must_use]
    pub fn met_with_margin(&self) -> bool {
        self.met() && self.criticality <= 0.0
    }

    /// One-word verdict for a report line; unknowns are named beside a
    /// violation, never hidden by it.
    #[must_use]
    pub fn verdict(&self) -> &'static str {
        if !self.met() {
            if self.unknown > 0 { "VIOLATED + UNKNOWN" } else { "VIOLATED" }
        } else if self.unknown > 0 {
            "UNKNOWN"
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
    /// The operating point used; `None` means no simulation, so thermal
    /// results are vacuous and the report says so.
    pub bias: Option<BiasSummary>,
    /// Net count per class (every routing budget is keyed to a class).
    pub net_classes: Vec<(String, usize)>,
    /// `(rule kind, missing input)`: families never instantiated.
    pub missing: Vec<(&'static str, &'static str)>,
    /// Post-layout specs: `(metric, measured, min, max, normalised miss)`.
    /// Empty when performance scoring is off.
    pub performance: Vec<(String, Option<f64>, Option<f64>, Option<f64>, f64)>,
    /// Per declared spec bound, its routing budget row or why it has none
    /// (`"ugf:min: row (3 nets)"`, `"…: do-not-worsen row …"`, `"…: no row (reason)"`).
    /// Empty from [`build`]; the flow fills it.
    pub budget_rows: Vec<String>,
    /// Post-layout simulations that could not run ([`crate::RunStats::sim_failures`]).
    pub sim_failures: u32,
    /// One ledger row per matched pair on the placed layout, from the
    /// placement budget arm (each set is also in cost; reading both would
    /// duplicate it).
    pub matched: Vec<analog::matching::mismatch::LedgerRow>,
    /// Sidecar process numbers used on an `UNVERIFIED` source
    /// ([`verify::Pdk::unverified`]). Reported, not blocking [`Self::certified`].
    pub assumed: Vec<String>,
    /// Budget kinds whose price sat at its cap with the batch still violated
    /// on the run's **last** epoch ([`gp::Prices::saturated`]): why the search
    /// stopped, not a verdict on the drawn winner, which can be an earlier
    /// epoch where that batch was met (its rows above say). Empty from
    /// [`build`]; the flow fills it at the end of the run.
    pub binding: Vec<String>,
    /// What the winning epoch's signoff did not check (LVS-unverified
    /// devices block [`Self::certified`]; skipped rules are listed). Empty
    /// from [`build`]; the flow fills it from the winner.
    pub coverage: verify::Coverage,
    /// `(template, recognised non-glue blocks)`, by template name. Empty from
    /// [`build`]; the flow fills it from `annotator::Problem::blocks`.
    pub recognition: Vec<(&'static str, usize)>,
    /// `(device, reason)` for every `annotator::Coverage::Unconstrained` device.
    /// Empty from [`build`]; the flow fills it.
    pub unconstrained: Vec<(String, &'static str)>,
}

impl MetadataReport {
    /// Θ, the middle tier of the search key: Σ budget-arm residuals in
    /// milli-budgets (× 1000, the stage reports' scale). The only Θ source for
    /// rule batches: the key drops the stage reports' `batch:` rows, which
    /// restate these residuals. Criticality is deliberately excluded: a
    /// satisfied but tight budget must not read as violated.
    #[must_use]
    pub fn theta(&self) -> f64 {
        self.placement
            .iter()
            .chain(&self.routing)
            .filter(|b| b.arm == Arm::Budget)
            .map(|b| b.residual * 1000.0)
            .sum()
    }

    /// |V| from rule batches: violated hard-arm rules over both tiers,
    /// counted per rule, not per batch.
    #[must_use]
    pub fn hard_violated(&self) -> usize {
        self.placement
            .iter()
            .chain(&self.routing)
            .filter(|b| b.arm == Arm::Hard)
            .map(|b| b.violations)
            .sum()
    }

    /// Every family met with all its inputs present, none left
    /// uninstantiated, and every schematic device compared by LVS. A feasible
    /// search result is not a certificate without it.
    #[must_use]
    pub fn certified(&self) -> bool {
        self.missing.is_empty()
            && self.coverage.unverified.is_empty()
            && self.placement.iter().chain(&self.routing).all(|b| b.met() && b.unknown == 0)
            && self.performance.iter().all(|p| p.4 <= 0.0)
            && self.bias.as_ref().map_or(true, |b| !b.probe)
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
    /// Synthesised mid-rail probe, not a testbench: never a sign-off bias.
    pub probe: bool,
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
        let violations = b.violations(state) as usize;
        let unknown = b.unknown(state) as usize;
        // ponytail: a batch may count a rule both violated and unknown
        // (a batch that counts both); saturating keeps it out of both.
        let satisfied = (total - violations).saturating_sub(unknown);
        let criticality = b.criticality(state);
        let residual = b.residual(state);
        let usage = b.worst_usage(state);
        let mut violated = Vec::new();
        b.violating_residuals(state, &mut violated);
        // One row per family: batches of the same kind merge, since the annotator
        // emits one batch per recognised structure. Residuals *sum* — each is
        // normalised by its own budget, so the family total stays a real measure
        // of "how many budgets' worth over" (linear, per D17; a max would hide
        // every violation but the worst).
        if let Some(e) = out.iter_mut().find(|e| e.kind == kind) {
            e.total += total;
            e.satisfied += satisfied;
            e.violations += violations;
            e.unknown += unknown;
            e.criticality = e.criticality.max(criticality);
            e.residual += residual;
            e.usage = match (e.usage, usage) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            e.violated.extend(violated);
        } else {
            out.push(BudgetStatus {
                kind,
                arm,
                total,
                satisfied,
                violations,
                unknown,
                criticality,
                residual,
                usage,
                violated,
            });
        }
    }
    for e in &mut out {
        e.violated.sort_unstable_by_key(|v| v.0);
        e.violated.dedup_by(|a, b| a.0 == b.0 && {
            b.1 = b.1.max(a.1);
            true
        });
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind));
    out
}

/// Status of every hard and budget batch of both tiers against `layout` /
/// `routes`. Cost-arm batches have no spec, so no status.
#[must_use]
pub fn build(
    placement: &Requirements<pnr_core::Layout>,
    layout: &pnr_core::Layout,
    routing: &Requirements<pnr_core::Routes>,
    routes: &pnr_core::Routes,
    bias: Option<BiasSummary>,
    net_classes: &[analog::metadata::NetClassification],
    missing: &[(&'static str, &'static str)],
    assumed: &[&str],
) -> MetadataReport {
    let census = annotator::classify::census(net_classes)
        .into_iter()
        .map(|(c, n)| (format!("{c:?}"), n))
        .collect();
    let mut p = statuses(&placement.hard, layout, Arm::Hard);
    p.extend(statuses(&placement.budget, layout, Arm::Budget));
    let mut r = statuses(&routing.hard, routes, Arm::Hard);
    r.extend(statuses(&routing.budget, routes, Arm::Budget));
    let mut matched = Vec::new();
    placement.budget.iter().for_each(|b| b.ledger_rows(layout, &mut matched));
    MetadataReport {
        placement: p,
        routing: r,
        bias,
        net_classes: census,
        missing: missing.to_vec(),
        performance: Vec::new(),
        budget_rows: Vec::new(),
        sim_failures: 0,
        matched,
        assumed: assumed.iter().map(|s| (*s).to_string()).collect(),
        binding: Vec::new(),
        coverage: verify::Coverage::default(),
        recognition: Vec::new(),
        unconstrained: Vec::new(),
    }
}

impl MetadataReport {
    /// Fold in routing batches built after the fact (on placed pins).
    pub fn add_routing(&mut self, reqs: &[Box<dyn RuleBatch<pnr_core::Routes>>], routes: &pnr_core::Routes) {
        self.routing.extend(statuses(reqs, routes, Arm::Budget));
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
            let census: Vec<String> = self
                .net_classes
                .iter()
                .map(|(c, n)| format!("{n} {c}"))
                .collect();
            writeln!(f, "  nets: {}", census.join(", "))?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "  {:<22} {:>6} {:>5} {:>5} {:>5} {:>5}  {:>9}  {:>9}  {}",
            "constraint", "arm", "total", "sat", "viol", "unk", "critical", "residual", "verdict"
        )?;
        writeln!(f, "  {}", "-".repeat(87))?;
        for s in self.placement.iter().chain(self.routing.iter()) {
            writeln!(
                f,
                "  {:<22} {:>6} {:>5} {:>5} {:>5} {:>5}  {:>9.2}  {:>9.3}  {}",
                s.kind,
                match s.arm {
                    Arm::Hard => "hard",
                    Arm::Budget => "budget",
                },
                s.total,
                s.satisfied,
                s.violations,
                s.unknown,
                s.criticality,
                s.residual,
                s.verdict()
            )?;
        }
        if !self.performance.is_empty() {
            writeln!(f, "\n  {:<22} {:>12} {:>12} {:>12}  verdict", "spec (post-layout)", "measured", "min", "max")?;
            writeln!(f, "  {}", "-".repeat(75))?;
            let num = |v: Option<f64>| v.map_or_else(|| "-".to_string(), |v| format!("{v:.4e}"));
            for (m, v, lo, hi, miss) in &self.performance {
                let verdict = match (v, *miss > 0.0) {
                    (None, _) => "UNKNOWN (not measured)".to_string(),
                    (Some(_), true) => format!("VIOLATED ({:.1}% short)", miss * 100.0),
                    (Some(_), false) => "met".to_string(),
                };
                writeln!(f, "  {m:<22} {:>12} {:>12} {:>12}  {verdict}", num(*v), num(*lo), num(*hi))?;
            }
            writeln!(f, "  simulations failed: {}", self.sim_failures)?;
        }
        for b in &self.budget_rows {
            writeln!(f, "  budget {b}")?;
        }
        if !self.performance.is_empty() || !self.budget_rows.is_empty() {
            writeln!(f)?;
        }
        for (kind, input) in &self.missing {
            writeln!(f, "  {kind:<22} {:>6} {:>5} {:>5} {:>5} {:>5}  {:>9}  {:>9}  UNKNOWN (no {input})", "-", "-", "-", "-", "-", "-", "-")?;
        }
        if !self.recognition.is_empty() {
            let r: Vec<String> = self.recognition.iter().map(|(t, n)| format!("{t} ×{n}")).collect();
            writeln!(f, "  RECOGNITION: {}", r.join(", "))?;
        }
        if !self.unconstrained.is_empty() {
            let u: Vec<String> = self.unconstrained.iter().map(|(d, why)| format!("{d} ({why})")).collect();
            writeln!(f, "  UNCONSTRAINED: {}", u.join(", "))?;
        }
        if !self.assumed.is_empty() {
            writeln!(f, "\n  assumed (UNVERIFIED sidecar values): {}", self.assumed.join(", "))?;
        }
        if !self.binding.is_empty() {
            writeln!(f, "\n  price at cap on the last epoch (search stopped binding): {}", self.binding.join(", "))?;
        }
        if !self.coverage.unverified.is_empty() || !self.coverage.skipped_rules.is_empty() {
            write!(f, "\n{}", self.coverage)?;
        }
        writeln!(f, "\n  certificate: {}", if self.certified() { "all families met, all inputs present" } else { "NOT CERTIFIED" })?;
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
        /// A negative `used` stands for a rule missing its input.
        fn known(self, _: &Routes) -> bool {
            self.used >= 0.0
        }
    }

    fn reqs(used: &[f32]) -> Requirements<Routes> {
        let mut r = Requirements::<Routes>::default();
        r.hard.push(Box::new(
            used.iter()
                .map(|&u| Budgeted { used: u })
                .collect::<Vec<_>>(),
        ));
        r
    }

    fn empty_routes() -> Routes {
        Routes { wires: Vec::new(), ..Default::default()  }
    }

    #[test]
    fn distinguishes_comfortable_from_barely_legal() {
        let s = &statuses(&reqs(&[0.3]).hard, &empty_routes(), Arm::Hard)[0];
        assert!(
            s.met() && s.met_with_margin(),
            "slack-rich budget is fully met"
        );
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

    /// Unknown is never pass (master §6.1): of 5 rules, 1 violated and 1
    /// unknown leave 3 satisfied; |V| is the 1 violation, and the verdict
    /// names the unknown beside it.
    #[test]
    fn an_unknown_is_not_satisfied() {
        let s = &statuses(&reqs(&[0.1, 0.1, 0.1, 1.4, -1.0]).hard, &empty_routes(), Arm::Hard)[0];
        assert_eq!((s.total, s.satisfied, s.violations, s.unknown), (5, 3, 1, 1));
        assert_eq!(s.verdict(), "VIOLATED + UNKNOWN");
        let report = MetadataReport { routing: vec![s.clone()], ..Default::default() };
        assert_eq!(report.hard_violated(), 1);
        let s = &statuses(&reqs(&[0.1, -1.0]).hard, &empty_routes(), Arm::Hard)[0];
        assert!(s.met() && s.satisfied == 1, "{s:?}");
        assert_eq!(s.verdict(), "UNKNOWN");
    }

    #[test]
    fn recognition_is_printed() {
        let report = MetadataReport {
            recognition: vec![("five_transistor_ota", 1)],
            unconstrained: vec![("R1".into(), "no pattern")],
            ..Default::default()
        };
        let out = report.to_string();
        assert!(out.contains("  RECOGNITION: five_transistor_ota ×1\n"), "{out}");
        assert!(out.contains("  UNCONSTRAINED: R1 (no pattern)\n"), "{out}");
    }

    #[test]
    fn family_row_follows_its_worst_member() {
        let s = &statuses(&reqs(&[0.1, 0.1, 0.95]).hard, &empty_routes(), Arm::Hard)[0];
        assert_eq!(s.total, 3);
        assert_eq!(s.satisfied, 3, "all legal");
        assert!(
            !s.met_with_margin(),
            "one member in the margin taints the family"
        );
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
        assert_eq!(
            hard_only.theta(),
            0.0,
            "hard residuals are Φ's business, not Θ's"
        );
    }

    #[test]
    fn unsimulated_bias_is_stated_not_implied() {
        let r = MetadataReport::default();
        let text = format!("{r}");
        assert!(
            text.contains("NOT SIMULATED"),
            "must not imply a thermal pass: {text}"
        );
        assert!(text.contains("vacuous"));
    }

    /// A failed boolean budget (no measurable overshoot) still moves Θ and
    /// blocks the certificate: fail is never read as pass (EVD-07).
    #[test]
    fn a_boolean_fail_costs_theta() {
        #[derive(Clone, Copy)]
        struct Fails;
        impl Rule for Fails {
            type On = Routes;
            fn cost(self, _: &Routes) -> f32 {
                0.0
            }
            fn satisfied(self, _: &Routes) -> bool {
                false
            }
            fn residual(self, _: &Routes) -> f32 {
                0.0
            }
        }
        let mut rq = Requirements::<Routes>::default();
        rq.budget.push(Box::new(vec![Fails]));
        let r = MetadataReport { routing: statuses(&rq.budget, &empty_routes(), Arm::Budget), ..MetadataReport::default() };
        assert!(r.theta() > 0.0);
        assert!(!r.certified());
    }

    /// Unknown is neither pass nor fail: a met family with missing inputs, or a
    /// family the deck could not instantiate, blocks the certificate.
    #[test]
    fn unknown_inputs_block_the_certificate() {
        #[derive(Clone, Copy)]
        struct Unrouted;
        impl Rule for Unrouted {
            type On = Routes;
            fn cost(self, _: &Routes) -> f32 {
                0.0
            }
            fn known(self, _: &Routes) -> bool {
                false
            }
        }
        let mut rq = Requirements::<Routes>::default();
        rq.budget.push(Box::new(vec![Unrouted]));
        let r = MetadataReport {
            routing: statuses(&rq.budget, &empty_routes(), Arm::Budget),
            ..MetadataReport::default()
        };
        assert!(r.routing[0].met(), "search sees it as satisfied");
        assert_eq!(r.routing[0].verdict(), "UNKNOWN");
        assert!(!r.certified());

        let clean = MetadataReport { missing: vec![("Antenna", "deck antenna ratio")], ..MetadataReport::default() };
        assert!(!clean.certified(), "an uninstantiated family is not a pass");
        let coverage = verify::Coverage { unverified: vec![(verify::RefKind::Npn, None, 2)], ..Default::default() };
        let unverified = MetadataReport { coverage, ..MetadataReport::default() };
        assert!(!unverified.certified(), "a device LVS did not compare is not a pass");
        assert!(unverified.to_string().contains("LVS unverified: 2 × Npn"), "{unverified}");
        assert!(MetadataReport::default().certified());
    }

    /// Values on an `UNVERIFIED` source are reported, not blocking.
    #[test]
    fn assumed_values_are_listed_not_blocking() {
        let r = MetadataReport { assumed: vec!["tie_max_dist_nm".into()], ..MetadataReport::default() };
        assert!(r.certified());
        assert!(r.to_string().contains("assumed (UNVERIFIED sidecar values): tie_max_dist_nm"), "{r}");
    }

    /// A probe bias (no user testbench) never certifies, whatever else is
    /// met: the bench's rows are all probes, so none of them can be a
    /// sign-off certificate.
    #[test]
    fn a_probe_bias_never_certifies() {
        let probe = BiasSummary { provenance: String::new(), resolved: 0, devices: 0, total_power_uw: 0, hottest: None, probe: true };
        let real = BiasSummary { probe: false, ..probe.clone() };
        assert!(!MetadataReport { bias: Some(probe), ..MetadataReport::default() }.certified());
        assert!(MetadataReport { bias: Some(real), ..MetadataReport::default() }.certified());
    }

    /// A saturated price is printed as the search's last-epoch state, never as
    /// a claim about the drawn winner, and only when there is one.
    #[test]
    fn binding_kinds_are_printed_as_last_epoch_state() {
        assert!(!MetadataReport::default().to_string().contains("price at cap"));
        let r = MetadataReport { binding: vec!["WireLength".into()], ..MetadataReport::default() };
        assert!(r.to_string().contains("price at cap on the last epoch (search stopped binding): WireLength"), "{r}");
    }
}
