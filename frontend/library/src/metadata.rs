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
    /// Rule kind, path-trimmed (`ThermalGradient`, `CrosstalkExclusion`, …).
    pub kind: String,
    /// Which arm the family was registered in.
    pub arm: Arm,
    /// Rules of this kind in the circuit.
    pub total: usize,
    /// How many are satisfied against the **raw** spec.
    pub satisfied: usize,
    /// How many lack their inputs ([`analog::Rule::known`]); counted in
    /// `satisfied` too, since search cannot act on them.
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
    /// Sidecar process numbers used on an `UNVERIFIED` source
    /// ([`verify::Pdk::unverified`]). Reported, not blocking [`Self::certified`].
    pub assumed: Vec<String>,
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

    /// |V| from rule batches: violated hard-arm rules (`total − satisfied`)
    /// over both tiers, counted per rule, not per batch.
    #[must_use]
    pub fn hard_violated(&self) -> usize {
        self.placement
            .iter()
            .chain(&self.routing)
            .filter(|b| b.arm == Arm::Hard)
            .map(|b| b.total - b.satisfied)
            .sum()
    }

    /// Every family met with all its inputs present, and none left
    /// uninstantiated. A feasible search result is not a certificate without it.
    #[must_use]
    pub fn certified(&self) -> bool {
        self.missing.is_empty()
            && self.placement.iter().chain(&self.routing).all(|b| b.met() && b.unknown == 0)
            && self.performance.iter().all(|p| p.4 <= 0.0)
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
        let unknown = b.unknown(state) as usize;
        let criticality = b.criticality(state);
        let residual = b.residual(state);
        let usage = b.worst_usage(state);
        // One row per family: batches of the same kind merge, since the annotator
        // emits one batch per recognised structure. Residuals *sum* — each is
        // normalised by its own budget, so the family total stays a real measure
        // of "how many budgets' worth over" (linear, per D17; a max would hide
        // every violation but the worst).
        if let Some(e) = out.iter_mut().find(|e| e.kind == kind) {
            e.total += total;
            e.satisfied += satisfied;
            e.unknown += unknown;
            e.criticality = e.criticality.max(criticality);
            e.residual += residual;
            e.usage = match (e.usage, usage) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
        } else {
            out.push(BudgetStatus {
                kind,
                arm,
                total,
                satisfied,
                unknown,
                criticality,
                residual,
                usage,
            });
        }
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
    MetadataReport {
        placement: p,
        routing: r,
        bias,
        net_classes: census,
        missing: missing.to_vec(),
        performance: Vec::new(),
        assumed: assumed.iter().map(|s| (*s).to_string()).collect(),
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
            writeln!(f)?;
        }
        for (kind, input) in &self.missing {
            writeln!(f, "  {kind:<22} {:>6} {:>5} {:>5}  {:>9}  {:>9}  UNKNOWN (no {input})", "-", "-", "-", "-", "-")?;
        }
        if !self.assumed.is_empty() {
            writeln!(f, "\n  assumed (UNVERIFIED sidecar values): {}", self.assumed.join(", "))?;
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
        assert!(MetadataReport::default().certified());
    }

    /// Values on an `UNVERIFIED` source are reported, not blocking.
    #[test]
    fn assumed_values_are_listed_not_blocking() {
        let r = MetadataReport { assumed: vec!["tie_max_dist_nm".into()], ..MetadataReport::default() };
        assert!(r.certified());
        assert!(r.to_string().contains("assumed (UNVERIFIED sidecar values): tie_max_dist_nm"), "{r}");
    }
}
