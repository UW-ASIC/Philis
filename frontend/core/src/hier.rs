//! Hierarchical place-and-route driver.
//!
//! Splits a SPICE design into its subcircuits, solves the instantiation
//! dependency DAG, and P&Rs each subckt exactly once — leaves first,
//! independent nodes in parallel. Each finished block becomes a fixed macro
//! cell its parents route to, so:
//!   * a subckt instantiated N times is placed once and reused (arrays/macros);
//!   * subckts with no dependency between them run concurrently (parallel blocks).
//!
//! The top-level (last-defined) subckt is P&R'd last, with every reachable
//! child already a macro, and its result is the finished chip.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use pnr_backend::{block_to_macro, ConstraintRecord};
use pnr_cells::CellOutput;
use rayon::prelude::*;

use crate::orchestrator::{run_on_process, FlowConfig, FlowResult};

struct Subckt {
    name: String,
    ports: Vec<String>,
    text: String,          // full `.subckt` … `.ends` block, verbatim
    devices: usize,        // instance lines (rough size)
    children: Vec<String>, // direct subckt instances (deduped)
}

/// The last positional token of an instance line = its master (subckt/device).
fn instance_master(line: &str) -> Option<&str> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.len() < 3 {
        return None;
    }
    let eq = toks[1..]
        .iter()
        .position(|t| t.contains('='))
        .map_or(toks.len(), |p| p + 1);
    toks[1..eq].last().copied()
}

/// Parse `.subckt`/`.ends` blocks: ports, raw text, instantiated subckts.
fn split_subckts(spice: &str) -> Vec<Subckt> {
    // Pass 1: every subckt name (so instances can be classified).
    let names: HashSet<String> = spice
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.to_ascii_lowercase()
                .starts_with(".subckt")
                .then(|| l.split_whitespace().nth(1).map(str::to_string))
                .flatten()
        })
        .collect();

    // Pass 2: blocks.
    let mut out = Vec::new();
    let mut cur: Option<Subckt> = None;
    for line in spice.lines() {
        let trimmed = line.trim();
        let low = trimmed.to_ascii_lowercase();

        if low.starts_with(".subckt") {
            let toks: Vec<&str> = trimmed.split_whitespace().collect();
            cur = Some(Subckt {
                name: toks.get(1).copied().unwrap_or_default().to_string(),
                ports: toks[2.min(toks.len())..]
                    .iter()
                    .filter(|t| !t.contains('='))
                    .map(|s| (*s).to_string())
                    .collect(),
                text: String::new(),
                devices: 0,
                children: Vec::new(),
            });
        }

        if let Some(s) = cur.as_mut() {
            s.text.push_str(line);
            s.text.push('\n');
            if !low.starts_with('.') && !trimmed.is_empty() {
                s.devices += 1;
                if let Some(m) = instance_master(trimmed) {
                    if names.contains(m) && !s.children.iter().any(|c| c == m) {
                        s.children.push(m.to_string());
                    }
                }
            }
        }

        if low.starts_with(".ends") {
            if let Some(s) = cur.take() {
                out.push(s);
            }
        }
    }
    out
}

/// Longest child-chain depth per node (leaves = 0). Used to band the DAG into
/// dependency levels processed low→high.
fn levels(subs: &[Subckt], idx_of: &HashMap<&str, usize>) -> Vec<usize> {
    let mut memo = vec![usize::MAX; subs.len()];
    fn depth(
        i: usize,
        subs: &[Subckt],
        idx_of: &HashMap<&str, usize>,
        memo: &mut [usize],
    ) -> usize {
        if memo[i] != usize::MAX {
            return memo[i];
        }
        memo[i] = 0; // guard against cycles (netlists are acyclic)
        let d = subs[i]
            .children
            .iter()
            .filter_map(|c| idx_of.get(c.as_str()).copied())
            .map(|ci| 1 + depth(ci, subs, idx_of, memo))
            .max()
            .unwrap_or(0);
        memo[i] = d;
        d
    }
    for i in 0..subs.len() {
        memo[i] = usize::MAX;
    }
    (0..subs.len())
        .map(|i| depth(i, subs, idx_of, &mut memo))
        .collect()
}

/// Reachable set from `top` over the child edges.
fn reachable(top: usize, subs: &[Subckt], idx_of: &HashMap<&str, usize>) -> HashSet<usize> {
    let mut seen = HashSet::new();
    let mut stack = vec![top];
    while let Some(i) = stack.pop() {
        if !seen.insert(i) {
            continue;
        }
        for c in &subs[i].children {
            if let Some(&ci) = idx_of.get(c.as_str()) {
                stack.push(ci);
            }
        }
    }
    seen
}

/// Build a standalone netlist that makes subckt `idx` the top: every other
/// block first (their defs supply macro port lists), `idx`'s block last so the
/// parser treats it as top-level.
fn netlist_for(idx: usize, subs: &[Subckt]) -> String {
    let mut s = String::new();
    for (j, sub) in subs.iter().enumerate() {
        if j != idx {
            s.push_str(&sub.text);
            s.push('\n');
        }
    }
    s.push_str(&subs[idx].text);
    s
}

/// Hierarchical P&R. Returns the finished top-level [`FlowResult`].
///
/// # Errors
/// Propagates PDK-load, parse, or per-block P&R failures.
pub fn run_flow_hier(
    spice: &str,
    deck_json: &str,
    constraints: &ConstraintRecord,
    config: &FlowConfig,
) -> Result<FlowResult, String> {
    let process = pnr_backend::load_pdk(deck_json)?;
    let subs = split_subckts(spice);
    if subs.is_empty() {
        return Err("no .subckt blocks found; use a flat run for a single circuit".into());
    }

    let idx_of: HashMap<&str, usize> =
        subs.iter().enumerate().map(|(i, s)| (s.name.as_str(), i)).collect();
    let lvl = levels(&subs, &idx_of);
    let top = subs.len() - 1; // last-defined subckt = design top (matches the flat parser)
    let reach = reachable(top, &subs, &idx_of);

    // ── Verbose: blocks found ─────────────────────────────────────────────
    eprintln!("[hier] {} subckt(s) parsed:", subs.len());
    for (i, s) in subs.iter().enumerate() {
        let mark = if i == top {
            " (TOP)"
        } else if reach.contains(&i) {
            ""
        } else {
            " (unused)"
        };
        eprintln!(
            "[hier]   {:<22} ports={:<3} devices={:<4} lvl={} children=[{}]{}",
            s.name,
            s.ports.len(),
            s.devices,
            lvl[i],
            s.children.join(", "),
            mark,
        );
    }

    // ── Verbose: dependency DAG, banded leaves→top ────────────────────────
    let max_lvl = reach.iter().map(|&i| lvl[i]).max().unwrap_or(0);
    let mut bands: Vec<Vec<usize>> = vec![Vec::new(); max_lvl + 1];
    for &i in &reach {
        bands[lvl[i]].push(i);
    }
    eprintln!("[hier] dependency DAG ({} reachable, leaves first):", reach.len());
    for (l, band) in bands.iter().enumerate() {
        let names: Vec<&str> = band.iter().map(|&i| subs[i].name.as_str()).collect();
        eprintln!("[hier]   level {l}: [{}]  (parallel)", names.join(", "));
    }

    // ── Build every reachable non-top block once, leaves first, level in
    //    parallel; each result becomes a macro cell for its parents. ───────
    let mut registry: HashMap<String, Arc<CellOutput>> = HashMap::new();
    for band in &bands {
        let jobs: Vec<usize> = band.iter().copied().filter(|&i| i != top).collect();
        if jobs.is_empty() {
            continue;
        }
        let built: Vec<Result<(String, Arc<CellOutput>), String>> = jobs
            .par_iter()
            .map(|&i| {
                let s = &subs[i];
                eprintln!(
                    "[hier]   P&R block '{}' (lvl {}, {} devices, {} macro child(ren))",
                    s.name,
                    lvl[i],
                    s.devices,
                    s.children.len()
                );
                let nl = netlist_for(i, &subs);
                let res = run_on_process(&nl, &process, constraints, config, registry.clone())
                    .map_err(|e| format!("block '{}': {e}", s.name))?;
                let macro_cell = block_to_macro(&res, &s.ports, &process.deck, &process.cells);
                eprintln!(
                    "[hier]   done  '{}': {}x{} nm, {} boundary pins, DRC {} | LVS {}",
                    s.name,
                    macro_cell.bbox.xmax - macro_cell.bbox.xmin,
                    macro_cell.bbox.ymax - macro_cell.bbox.ymin,
                    macro_cell.pins.len(),
                    res.signoff.drc_blocking.len(),
                    if res.signoff.lvs.matched { "match" } else { "MISMATCH" },
                );
                Ok((s.name.clone(), Arc::new(macro_cell)))
            })
            .collect();
        for r in built {
            let (name, cell) = r?;
            registry.insert(name, cell);
        }
    }

    // ── Top level: children are all macros now. This run is the chip. ─────
    eprintln!(
        "[hier] assembling TOP '{}' with {} macro block(s)",
        subs[top].name,
        registry.len()
    );
    let nl = netlist_for(top, &subs);
    run_on_process(&nl, &process, constraints, config, registry)
}
