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

use std::io::Write;

fn term_height() -> u16 {
    std::process::Command::new("tput")
        .arg("lines")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(40)
}

fn setup_pinned_bar(rows: u16) {
    // Set scroll region to all lines except the last, then park the cursor at
    // the BOTTOM of the region so output keeps scrolling there. (Parking at
    // the top overprints whatever is already on screen — garbled lines.)
    eprint!("\x1b[1;{}r\x1b[{};1H", rows - 1, rows - 1);
    let _ = std::io::stderr().flush();
}

fn update_pinned_bar(rows: u16, msg: &str) {
    // Save cursor, jump to last line, clear it, print, restore cursor
    eprint!("\x1b7\x1b[{rows};1H\x1b[2K\x1b[36;1m{msg}\x1b[0m\x1b8");
    let _ = std::io::stderr().flush();
}

fn teardown_pinned_bar(rows: u16) {
    // Reset scroll region, move to bottom, clear status line
    eprint!("\x1b[r\x1b[{rows};1H\x1b[2K");
    let _ = std::io::stderr().flush();
}

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
    /// All parasitic budgets met and matched-pair deltas within tolerance.
    pub parasitic_clean: bool,
    /// Constraint-violation pushes from routing: (device_a, device_b, gap_nm).
    pub constraint_adjustments: Vec<(String, String, f64)>,
    /// Blocking DRC violations from in-loop signoff (0 when signoff not run).
    pub drc_blocking: usize,
    /// LVS netlist match from in-loop signoff (true when signoff not run).
    pub lvs_matched: bool,
    /// Die area in nm² (0 when unknown). Best-iteration tiebreak: among
    /// equally-clean iterations the smallest die wins.
    pub die_area: u64,
    /// Hard constraint-contract violations (placement + routing). A small die
    /// that breaks hard contracts must never beat a legal one.
    pub hard_violations: usize,
    /// Aggregate routed resistance and capacitance. These remain optimization
    /// objectives after all explicit parasitic budgets have been satisfied.
    pub total_r_ohm: f64,
    pub total_c_ff: f64,
    /// Routed geometry objectives used alongside die area.
    pub wirelength_nm: u64,
    pub via_count: usize,
}

/// Backend-neutral metrics captured for one feedback iteration.
///
/// Keeping the trace beside best-candidate selection makes benchmark and debug
/// artifacts describe the candidate that was actually returned, rather than
/// whichever iteration happened to run last.
#[derive(Debug, Clone)]
pub struct IterationSummary {
    pub iteration: u32,
    pub goal_reached: bool,
    pub new_best: bool,
    pub max_weight: f64,
    pub routing_clean: bool,
    pub parasitic_clean: bool,
    pub drc_blocking: usize,
    pub lvs_matched: bool,
    pub die_area: u64,
    pub hard_violations: usize,
    pub total_r_ohm: f64,
    pub total_c_ff: f64,
    pub wirelength_nm: u64,
    pub via_count: usize,
    pub physical_score: f64,
}

/// Why routing recommends a cell variant change.
#[derive(Debug, Clone)]
pub enum HintReason {
    HighParasiticR {
        net: String,
        estimated_ohm: f64,
        budget_ohm: f64,
    },
    HighParasiticC {
        net: String,
        estimated_ff: f64,
        budget_ff: f64,
    },
    MatchedMismatchR {
        net_a: String,
        net_b: String,
        delta_pct: f64,
    },
    MatchedMismatchC {
        net_a: String,
        net_b: String,
        delta_pct: f64,
    },
    PinAccessFailed {
        net: String,
        displacement_nm: i32,
    },
    Congested,
}

/// One recommendation for a cell to try a different variant.
pub struct CellHint {
    pub cell_idx: u32,
    pub reason: HintReason,
}

pub struct BlockConfig {
    pub max_iters: u32,
    pub feedback_threshold: f64,
    pub circuit_name: Option<String>,
    /// Stop early after this many iterations without a new best — the
    /// feedback loop has plateaued and further iterations only repeat states.
    pub stall_iters: u32,
    /// Relative physical-objective improvement required to reset the plateau
    /// counter. Tiny coordinate/area noise still updates the returned best,
    /// but does not keep the search alive indefinitely.
    pub min_improvement: f64,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            max_iters: 1,
            feedback_threshold: 1.2,
            circuit_name: None,
            stall_iters: 12,
            min_improvement: 0.005,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Feasibility {
    lvs_mismatch: u8,
    violations: u64,
    routing_dirty: u8,
    parasitic_dirty: u8,
}

#[derive(Clone, Copy)]
struct Quality {
    feasibility: Feasibility,
    physical: f64,
}

#[derive(Clone, Copy)]
struct ObjectiveReference {
    area: f64,
    wirelength: f64,
    resistance: f64,
    capacitance: f64,
    vias: f64,
}

impl ObjectiveReference {
    fn from_feedback(fb: &Feedback) -> Self {
        Self {
            area: (fb.die_area as f64).max(1.0),
            wirelength: (fb.wirelength_nm as f64).max(1.0),
            resistance: fb.total_r_ohm.max(1e-12),
            capacitance: fb.total_c_ff.max(1e-12),
            vias: (fb.via_count as f64).max(1.0),
        }
    }

    fn physical(self, fb: &Feedback) -> f64 {
        // Fixed first-iteration normalization makes unlike units comparable.
        // Feasibility is compared separately and always dominates this value.
        0.45 * fb.die_area as f64 / self.area
            + 0.20 * fb.wirelength_nm as f64 / self.wirelength
            + 0.12 * fb.total_r_ohm / self.resistance
            + 0.13 * fb.total_c_ff / self.capacitance
            + 0.10 * fb.via_count as f64 / self.vias
    }
}

pub struct BlockResult<C, P, R> {
    pub cells: C,
    pub placement: P,
    pub routing: R,
    pub iterations: u32,
    pub best_iteration: u32,
    pub converged: bool,
    pub trace: Vec<IterationSummary>,
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
    // Step 1: initialize the feedback vectors and terminal progress display
    // once. Each vector is reused and replaced between iterations.
    let max = cfg.max_iters.max(1);
    let mut net_weights: Vec<(String, f64)> = Vec::new();
    let mut cell_inflation_x: Vec<f64> = Vec::new();
    let mut cell_inflation_y: Vec<f64> = Vec::new();
    let mut cell_hints: Vec<CellHint> = Vec::new();
    let mut constraint_adjustments: Vec<(String, String, f64)> = Vec::new();
    let block_start = std::time::Instant::now();
    let rows = term_height();
    // Set up the scroll region FIRST, then print the banner inside it —
    // otherwise the banner lands below the pinned status line.
    setup_pinned_bar(rows);
    if let Some(name) = &cfg.circuit_name {
        eprintln!("\n══════════════════════════════════════════════════════════════");
        eprintln!("  {name}");
        eprintln!("══════════════════════════════════════════════════════════════");
    }

    // Step 2: define the lexicographic feasibility gate and initialize
    // best-candidate tracking. Within one feasibility class, optimize a
    // normalized physical objective instead of treating die area as the only
    // quality signal. This prevents a smaller but RC-heavy layout from hiding
    // a materially better routed solution.
    let feasibility_of = |fb: &Feedback| Feasibility {
        lvs_mismatch: u8::from(!fb.lvs_matched),
        violations: (fb.drc_blocking + fb.hard_violations) as u64,
        routing_dirty: u8::from(!fb.clean),
        parasitic_dirty: u8::from(!fb.parasitic_clean),
    };
    let mut objective_reference: Option<ObjectiveReference> = None;
    let mut best: Option<(Quality, u32, C, P, R)> = None;
    let mut stall = 0u32;
    let mut trace = Vec::with_capacity(max as usize);
    let mut seen_feasible = false;

    for iter in 0..max {
        // Step 3: execute the fixed cells -> placement -> routing template for
        // this iteration, then extract one backend-neutral feedback record.
        let elapsed = block_start.elapsed().as_secs_f64();
        let per_iter = if iter > 0 { elapsed / iter as f64 } else { 0.0 };
        let remaining = per_iter * (max - iter) as f64;
        if iter > 0 {
            update_pinned_bar(rows, &format!(
                "[engine] iter {iter}/{max} | {per_iter:.1}s/iter | ETA {remaining:.0}s | elapsed {elapsed:.0}s"
            ));
        } else {
            update_pinned_bar(rows, &format!("[engine] iter 0/{max} | starting..."));
        }
        // Per-step timing: each phase is timed so slow steps are visible and
        // the flow ETA reflects real per-phase cost, not just iteration count.
        eprintln!(
            "[engine] iter {}/{max} — cells ({} hints)",
            iter,
            cell_hints.len()
        );
        let t_cells = std::time::Instant::now();
        let c = cells(iter, &cell_hints);
        let d_cells = t_cells.elapsed();

        eprintln!(
            "[engine] iter {}/{max} — place ({} net weights, {} inflation)",
            iter,
            net_weights.len(),
            cell_inflation_x.len()
        );
        let t_place = std::time::Instant::now();
        let p = place(
            iter,
            &c,
            &net_weights,
            &cell_inflation_x,
            &cell_inflation_y,
            &constraint_adjustments,
        );
        let d_place = t_place.elapsed();

        eprintln!("[engine] iter {}/{max} — route", iter);
        let t_route = std::time::Instant::now();
        let r = route(&c, &p);
        let d_route = t_route.elapsed();

        eprintln!("[engine] iter {}/{max} — extracting feedback", iter);
        let t_extract = std::time::Instant::now();
        let fb = extract(&c, &p, &r);
        let d_extract = t_extract.elapsed();

        let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
        let iter_ms = ms(d_cells) + ms(d_place) + ms(d_route) + ms(d_extract);
        // Flow ETA from wall-clock over completed iterations (captures all
        // overhead, not just the four timed phases).
        let done = iter + 1;
        let flow_elapsed = block_start.elapsed().as_secs_f64();
        let flow_eta = flow_elapsed / done as f64 * (max - done) as f64;
        eprintln!(
            "[engine] iter {iter}/{max} — timing: cells {:.0}ms | place {:.0}ms | route {:.0}ms | extract {:.0}ms | step {:.0}ms | flow ETA {flow_eta:.0}s",
            ms(d_cells), ms(d_place), ms(d_route), ms(d_extract), iter_ms,
        );
        eprintln!("[engine] iter {}/{max} — feedback: max_weight={:.2}, clean={}, parasitic_clean={}, drc_blocking={}, lvs={}, {} hints, {} adjustments",
            iter, fb.max_weight, fb.clean, fb.parasitic_clean, fb.drc_blocking,
            if fb.lvs_matched { "MATCH" } else { "MISMATCH" },
            fb.cell_hints.len(), fb.constraint_adjustments.len());
        for (a, b, gap) in &fb.constraint_adjustments {
            eprintln!("[engine]   adjustment: {a} ↔ {b} gap={gap:.0}nm");
        }
        eprintln!("[engine]");

        // Step 4: score feasibility before physical quality so an illegal or
        // LVS-broken layout can never beat a larger clean layout.
        let feasible = fb.clean
            && fb.parasitic_clean
            && fb.drc_blocking == 0
            && fb.hard_violations == 0
            && fb.lvs_matched;
        let goal = feasible && fb.max_weight < cfg.feedback_threshold;
        seen_feasible |= feasible;
        let reference =
            *objective_reference.get_or_insert_with(|| ObjectiveReference::from_feedback(&fb));
        let quality = Quality {
            feasibility: feasibility_of(&fb),
            physical: reference.physical(&fb),
        };
        let (better, material) = best.as_ref().map_or((true, true), |(bq, ..)| {
            if quality.feasibility < bq.feasibility {
                (true, true)
            } else if quality.feasibility > bq.feasibility {
                (false, false)
            } else {
                let better = quality.physical < bq.physical;
                let material =
                    quality.physical < bq.physical * (1.0 - cfg.min_improvement.clamp(0.0, 0.5));
                (better, material)
            }
        });

        trace.push(IterationSummary {
            iteration: iter,
            goal_reached: goal,
            new_best: better,
            max_weight: fb.max_weight,
            routing_clean: fb.clean,
            parasitic_clean: fb.parasitic_clean,
            drc_blocking: fb.drc_blocking,
            lvs_matched: fb.lvs_matched,
            die_area: fb.die_area,
            hard_violations: fb.hard_violations,
            total_r_ohm: fb.total_r_ohm,
            total_c_ff: fb.total_c_ff,
            wirelength_nm: fb.wirelength_nm,
            via_count: fb.via_count,
            physical_score: quality.physical,
        });

        // Step 5: retain the best owned stage outputs and update the material-
        // improvement plateau counter without cloning large layout buffers.
        if better {
            eprintln!(
                "[engine] iter {iter}/{max} — new best (lvs={}, drc={}, clean={}, physical={:.4})",
                fb.lvs_matched, fb.drc_blocking, fb.clean, quality.physical
            );
            best = Some((quality, iter, c, p, r));
        }
        if material {
            stall = 0;
        } else {
            stall += 1;
        }

        // Step 6: once a feasible state has been observed, continue until
        // physical quality plateaus. The routing-weight threshold remains a
        // recorded optimization signal, not a signoff rule, and therefore
        // cannot keep a legal block spinning until the iteration budget.
        let feasible_plateau =
            cfg.stall_iters > 0 && stall >= cfg.stall_iters && seen_feasible;
        if (cfg.stall_iters == 0 && feasible) || feasible_plateau {
            eprintln!("[engine] iter {iter}/{max} — quality plateau ({stall} iterations without material improvement), stopping");
            teardown_pinned_bar(rows);
            let (_, bi, c, p, r) = best.unwrap();
            eprintln!("[engine] returning optimized feasible iteration {bi}");
            return BlockResult {
                cells: c,
                placement: p,
                routing: r,
                iterations: iter + 1,
                best_iteration: bi,
                converged: true,
                trace,
            };
        }

        // Step 7: move feedback into the next iteration's input buffers.
        net_weights = fb.net_weights;
        cell_inflation_x = fb.cell_inflation_x;
        cell_inflation_y = fb.cell_inflation_y;
        cell_hints = fb.cell_hints;
        constraint_adjustments = fb.constraint_adjustments;
    }

    // Step 8: on budget exhaustion, tear down terminal state and return the
    // best candidate observed rather than merely the final iteration.
    teardown_pinned_bar(rows);
    let (_, bi, c, p, r) = best.unwrap();
    eprintln!("[engine] max iters reached — returning best iteration {bi}/{max}");
    BlockResult {
        cells: c,
        placement: p,
        routing: r,
        iterations: max,
        best_iteration: bi,
        converged: seen_feasible,
        trace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feedback(die_area: u64, max_weight: f64) -> Feedback {
        Feedback {
            net_weights: Vec::new(),
            cell_inflation_x: Vec::new(),
            cell_inflation_y: Vec::new(),
            cell_hints: Vec::new(),
            max_weight,
            clean: true,
            parasitic_clean: true,
            constraint_adjustments: Vec::new(),
            drc_blocking: 0,
            lvs_matched: true,
            die_area,
            hard_violations: 0,
            total_r_ohm: 10.0,
            total_c_ff: 10.0,
            wirelength_nm: 100,
            via_count: 1,
        }
    }

    #[test]
    fn result_and_trace_identify_the_owned_best_iteration() {
        let cfg = BlockConfig {
            max_iters: 3,
            stall_iters: 99,
            ..Default::default()
        };
        let areas = [100, 80, 90];
        let max_weights = [1.1, 2.0, 2.0];
        let mut next = 0usize;
        let result = run_block(
            &cfg,
            |iteration, _| iteration,
            |_, &cells, _, _, _, _| cells,
            |_, &placement| placement,
            |_, _, _| {
                let value = feedback(areas[next], max_weights[next]);
                next += 1;
                value
            },
        );

        assert_eq!(result.iterations, 3);
        assert_eq!(result.best_iteration, 1);
        assert_eq!(result.placement, 1);
        assert!(result.converged, "a convergence signal was observed");
        assert_eq!(result.trace.len(), 3);
        assert_eq!(
            result.trace.iter().map(|row| row.new_best).collect::<Vec<_>>(),
            vec![true, true, false]
        );
    }
}
