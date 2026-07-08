//! Block-level feedback-loop orchestrator.
//!
//! The engine owns the full backend iteration cycle:
//!
//! ```text
//! iter 0: cells()           → place()           → route() → feedback
//! iter 1: cells(+feedback)  → place(+feedback)   → route() → feedback
//! iter N: cells(+feedback)  → place(+feedback)   → route() → feedback (converged)
//! ```
//!
//! Each stage is a caller-supplied closure — engine sequences them and
//! drives convergence without depending on pnr-placement/pnr-routing
//! (avoids Cargo cycle).

/// Feedback from routing — drives next iteration's cell selection AND placement.
pub struct Feedback {
    /// Per-net weight multipliers for placement cost.
    pub net_weights: Vec<(String, f64)>,
    /// Per-cell virtual x-size inflation for placement (horizontal congestion).
    pub cell_inflation_x: Vec<f64>,
    /// Per-cell virtual y-size inflation for placement (vertical congestion).
    pub cell_inflation_y: Vec<f64>,
    /// Cell-selection recommendations from routing (e.g. "cell 3 needs
    /// a wider variant"). Opaque to engine — passed back to `cells` closure.
    pub cell_hints: Vec<CellHint>,
    /// Largest net weight (convergence signal).
    pub max_weight: f64,
    /// All nets routed, no track overuse.
    pub clean: bool,
    /// Constraint-violation pushes from routing: (device_a, device_b, gap_nm).
    pub constraint_adjustments: Vec<(String, String, f64)>,
}

/// One recommendation for a cell to try a different variant.
pub struct CellHint {
    pub cell_idx: u32,
    pub reason: String,
}

pub struct BlockConfig {
    pub max_iters: u32,
    pub feedback_threshold: f64,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self { max_iters: 1, feedback_threshold: 1.2 }
    }
}

pub struct BlockResult<C, P, R> {
    pub cells: C,
    pub placement: P,
    pub routing: R,
    pub iterations: u32,
    pub converged: bool,
}

/// Run the full backend feedback loop for one block.
///
/// Closures (caller supplies, engine sequences):
/// - `cells`: generate/select cell variants. Receives feedback hints
///   from previous iteration (empty on iter 0).
/// - `place`: run placement. Receives cell output + feedback weights.
/// - `route`: run routing. Receives cell output + placement output.
/// - `extract`: pull feedback from the completed routing.
pub fn run_block<C, P, R>(
    cfg: &BlockConfig,
    mut cells: impl FnMut(u32, &[CellHint]) -> C,
    mut place: impl FnMut(u32, &C, &[(String, f64)], &[f64], &[f64], &[(String, String, f64)]) -> P,
    mut route: impl FnMut(&C, &P) -> R,
    mut extract: impl FnMut(&C, &P, &R) -> Feedback,
) -> BlockResult<C, P, R> {
    let max = cfg.max_iters.max(1);
    let mut net_weights: Vec<(String, f64)> = Vec::new();
    let mut cell_inflation_x: Vec<f64> = Vec::new();
    let mut cell_inflation_y: Vec<f64> = Vec::new();
    let mut cell_hints: Vec<CellHint> = Vec::new();
    let mut constraint_adjustments: Vec<(String, String, f64)> = Vec::new();

    for iter in 0..max {
        eprintln!("[engine] iter {}/{max} — cells ({} hints)", iter, cell_hints.len());
        let c = cells(iter, &cell_hints);

        eprintln!("[engine] iter {}/{max} — place ({} net weights, {} inflation)",
            iter, net_weights.len(), cell_inflation_x.len());
        let p = place(iter, &c, &net_weights, &cell_inflation_x, &cell_inflation_y, &constraint_adjustments);

        eprintln!("[engine] iter {}/{max} — route", iter);
        let r = route(&c, &p);

        if iter + 1 >= max {
            eprintln!("[engine] iter {}/{max} — max iters reached, returning", iter);
            return BlockResult {
                cells: c, placement: p, routing: r,
                iterations: iter + 1, converged: false,
            };
        }

        eprintln!("[engine] iter {}/{max} — extracting feedback", iter);
        let fb = extract(&c, &p, &r);
        eprintln!("[engine] iter {}/{max} — feedback: max_weight={:.2}, clean={}, {} hints",
            iter, fb.max_weight, fb.clean, fb.cell_hints.len());

        if fb.clean && fb.max_weight < cfg.feedback_threshold {
            eprintln!("[engine] iter {}/{max} — converged (max_weight {:.2} < threshold {:.2})",
                iter, fb.max_weight, cfg.feedback_threshold);
            return BlockResult {
                cells: c, placement: p, routing: r,
                iterations: iter + 1, converged: true,
            };
        }

        net_weights = fb.net_weights;
        cell_inflation_x = fb.cell_inflation_x;
        cell_inflation_y = fb.cell_inflation_y;
        cell_hints = fb.cell_hints;
        constraint_adjustments = fb.constraint_adjustments;
    }
    unreachable!()
}
