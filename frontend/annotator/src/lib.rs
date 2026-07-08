//! Netlist annotation: flat hypergraph → hierarchical block structure.
//!
//! PLAN.md stages 2, 4, 5. Pure structural recognition — no constraint
//! extraction. Constraints are derived downstream by pnr-constraints
//! running on each hierarchy level.

pub mod catalog;
mod pattern;

use std::collections::HashSet;

use pnr_cells::netlist::BipartiteHypergraph;

pub use pattern::PatternMatch;

// ── Net classification ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRole {
    Signal,
    Supply,
    Ground,
    Clock,
}

pub fn classify_nets(hg: &BipartiteHypergraph, cfg: &AnnotationConfig) -> Vec<NetRole> {
    hg.nets
        .iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            if cfg.supply_nets.iter().any(|s| s.eq_ignore_ascii_case(name))
                || pnr_constraints::SUPPLY_NAMES
                    .iter()
                    .any(|p| lower == *p || lower.starts_with(&format!("{p}_")))
            {
                NetRole::Supply
            } else if cfg.ground_nets.iter().any(|s| s.eq_ignore_ascii_case(name))
                || pnr_constraints::GROUND_NAMES
                    .iter()
                    .any(|p| lower == *p || lower.starts_with(&format!("{p}_")))
            {
                NetRole::Ground
            } else if cfg.clock_nets.iter().any(|s| s.eq_ignore_ascii_case(name))
                || pnr_constraints::CLK_PATTERNS
                    .iter()
                    .any(|p| lower.contains(p))
            {
                NetRole::Clock
            } else {
                NetRole::Signal
            }
        })
        .collect()
}

// ── Config ──

#[derive(Debug, Clone, Default)]
pub struct AnnotationConfig {
    pub do_not_identify: HashSet<String>,
    pub do_not_use: HashSet<String>,
    pub supply_nets: Vec<String>,
    pub ground_nets: Vec<String>,
    pub clock_nets: Vec<String>,
}

// ── Result ──

#[derive(Debug, Clone)]
pub struct RecognizedBlock {
    pub template: String,
    pub instances: Vec<String>,
    pub group_cell_id: u32,
}

/// One block in the dependency graph.
#[derive(Debug, Clone)]
pub struct BlockNode {
    /// Block index.
    pub id: u32,
    /// Pattern template name (e.g. "diff_pair").
    pub template: String,
    /// Cell indices in the ORIGINAL (pre-grouping) hypergraph.
    pub cell_indices: Vec<u32>,
    /// Instance names.
    pub instance_names: Vec<String>,
    /// Blocks that must be built before this one (by id).
    pub depends_on: Vec<u32>,
}

/// Dependency chain: blocks in topological order (leaves first).
#[derive(Debug, Clone, Default)]
pub struct DependencyChain {
    pub blocks: Vec<BlockNode>,
    /// Cells NOT claimed by any block (the "top-level glue").
    pub unclaimed: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct AnnotationResult {
    pub blocks: Vec<RecognizedBlock>,
    pub net_roles: Vec<NetRole>,
    pub chain: DependencyChain,
}

// ── Merge parallel devices (Stage 2) ──

fn merge_parallel(hg: &mut BipartiteHypergraph) {
    let n = hg.cells.len();
    let mut remove = HashSet::new();
    for i in 0..n {
        if remove.contains(&i) {
            continue;
        }
        let Some(di) = &hg.cells[i].device else { continue };
        let key_i = (di.device_type, di.model_name.clone(), di.w, di.l, di.nf);
        for j in (i + 1)..n {
            if remove.contains(&j) {
                continue;
            }
            let Some(dj) = &hg.cells[j].device else { continue };
            if key_i != (dj.device_type, dj.model_name.clone(), dj.w, dj.l, dj.nf) {
                continue;
            }
            if hg.cells[i].pins.len() != hg.cells[j].pins.len() {
                continue;
            }
            let same_nets = hg.cells[i]
                .pins
                .iter()
                .zip(hg.cells[j].pins.iter())
                .all(|((pi, ni), (pj, nj))| pi == pj && ni == nj);
            if same_nets {
                let mj = dj.multiplier;
                hg.cells[i].device.as_mut().unwrap().multiplier += mj;
                remove.insert(j);
            }
        }
    }
    if !remove.is_empty() {
        let mut indices: Vec<usize> = remove.into_iter().collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in indices {
            hg.cells.remove(idx);
        }
        hg.build_csr();
    }
}

// ── Dependency chain ──

/// Build a dependency chain from annotation results.
/// Blocks with no inter-block net dependencies come first.
/// The `unclaimed` cells form the implicit top-level block (depends on all others).
pub fn build_dependency_chain(
    hg: &BipartiteHypergraph,
    matches: &[PatternMatch],
) -> DependencyChain {
    let num_cells = hg.cells.len() as u32;

    // Collect all cells claimed by any pattern match.
    let mut claimed: HashSet<u32> = HashSet::new();
    let mut blocks = Vec::with_capacity(matches.len());

    for (i, m) in matches.iter().enumerate() {
        for &ci in &m.instances {
            claimed.insert(ci);
        }
        blocks.push(BlockNode {
            id: i as u32,
            template: m.template.clone(),
            cell_indices: m.instances.clone(),
            instance_names: m.instance_names.clone(),
            depends_on: Vec::new(),
        });
    }

    // Cells not claimed by any recognized block.
    let unclaimed: Vec<u32> = (0..num_cells).filter(|c| !claimed.contains(c)).collect();

    // For now, all recognized blocks at the same hierarchy level are
    // independent of each other (diff pair doesn't depend on current
    // mirror, etc.). They all come first in topological order;
    // the unclaimed "top-level glue" cells implicitly depend on all
    // recognized blocks.
    //
    // Future: detect inter-block net dependencies and add edges.

    DependencyChain { blocks, unclaimed }
}

// ── Entry point ──

pub fn annotate(
    hg: &mut BipartiteHypergraph,
    cfg: &AnnotationConfig,
) -> Result<AnnotationResult, String> {
    merge_parallel(hg);
    let net_roles = classify_nets(hg, cfg);
    let matches = pattern::recognize(hg, &net_roles, cfg);

    // Build the dependency chain BEFORE grouping (which mutates cell indices).
    let chain = build_dependency_chain(hg, &matches);

    let mut blocks = Vec::new();
    let mut counter = 0u32;
    for m in &matches {
        let name = format!("{}_{counter}", m.template);
        counter += 1;
        let gid = hg.group(&name, &m.template, &m.instance_names)?;
        blocks.push(RecognizedBlock {
            template: m.template.clone(),
            instances: m.instance_names.clone(),
            group_cell_id: gid,
        });
    }

    Ok(AnnotationResult { blocks, net_roles, chain })
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_cells::device::DeviceRecord;
    use pnr_cells::netlist::{BipartiteHypergraph, CellNode};
    use pnr_cells::DeviceType;
    use std::collections::HashMap;

    fn nfet(name: &str, d: u32, g: u32, s: u32, b: u32, w: i32, l: i32) -> CellNode {
        CellNode {
            name: name.into(),
            model: "nfet".into(),
            pins: vec![
                ("D".into(), d),
                ("G".into(), g),
                ("S".into(), s),
                ("B".into(), b),
            ],
            device: Some(DeviceRecord {
                name: name.into(),
                device_type: DeviceType::Nmos,
                w,
                l,
                nf: 1,
                multiplier: 1,
                model_name: "nfet".into(),
                terminals: HashMap::new(),
                params: HashMap::new(),
            }),
            grouped_devices: Vec::new(),
        }
    }

    fn pfet(name: &str, d: u32, g: u32, s: u32, b: u32, w: i32, l: i32) -> CellNode {
        CellNode {
            name: name.into(),
            model: "pfet".into(),
            pins: vec![
                ("D".into(), d),
                ("G".into(), g),
                ("S".into(), s),
                ("B".into(), b),
            ],
            device: Some(DeviceRecord {
                name: name.into(),
                device_type: DeviceType::Pmos,
                w,
                l,
                nf: 1,
                multiplier: 1,
                model_name: "pfet".into(),
                terminals: HashMap::new(),
                params: HashMap::new(),
            }),
            grouped_devices: Vec::new(),
        }
    }

    /// Build 5T OTA hypergraph directly.
    /// Nets: 0=vout1, 1=vinp, 2=vtail, 3=VSS, 4=vout2, 5=vinm, 6=vbias, 7=VDD, 8=vbn
    fn ota_hg() -> BipartiteHypergraph {
        let mut hg = BipartiteHypergraph {
            name: "ota".into(),
            ports: vec!["vinp".into(), "vinm".into(), "vout1".into(), "vout2".into(), "VDD".into(), "VSS".into()],
            cells: vec![
                nfet("XM1", 0, 1, 2, 3, 10_000, 1_000), // D=vout1 G=vinp S=vtail B=VSS
                nfet("XM2", 4, 5, 2, 3, 10_000, 1_000), // D=vout2 G=vinm S=vtail B=VSS
                pfet("XM3", 0, 6, 7, 7, 20_000, 1_000), // D=vout1 G=vbias S=VDD B=VDD
                pfet("XM4", 4, 6, 7, 7, 20_000, 1_000), // D=vout2 G=vbias S=VDD B=VDD
                nfet("XM5", 2, 8, 3, 3, 40_000, 2_000), // D=vtail G=vbn S=VSS B=VSS
            ],
            nets: vec![
                "vout1".into(), "vinp".into(), "vtail".into(), "VSS".into(),
                "vout2".into(), "vinm".into(), "vbias".into(), "VDD".into(), "vbn".into(),
            ],
            ..Default::default()
        };
        hg.build_csr();
        hg
    }

    #[test]
    fn annotate_ota() {
        let mut hg = ota_hg();
        assert_eq!(hg.cells.len(), 5);

        let result = annotate(&mut hg, &AnnotationConfig::default()).expect("annotate");

        // The catalog may recognize this as a composite 4-device pattern
        // (diff_pair_with_active_load) or as separate diff_pair + active_load.
        // Both are valid annotations; the composite is preferred when available.
        let composite: Vec<_> = result.blocks.iter()
            .filter(|b| b.template == "diff_pair_with_active_load")
            .collect();
        if !composite.is_empty() {
            assert_eq!(composite.len(), 1, "got {:?}", result.blocks);
            assert!(composite[0].instances.contains(&"XM1".to_string()));
            assert!(composite[0].instances.contains(&"XM2".to_string()));
            assert!(composite[0].instances.contains(&"XM3".to_string()));
            assert!(composite[0].instances.contains(&"XM4".to_string()));
        } else {
            let dp: Vec<_> = result.blocks.iter().filter(|b| b.template == "diff_pair").collect();
            assert_eq!(dp.len(), 1, "got {:?}", result.blocks);
            assert!(dp[0].instances.contains(&"XM1".to_string()));
            assert!(dp[0].instances.contains(&"XM2".to_string()));

            let loads: Vec<_> = result.blocks.iter().filter(|b| b.template == "active_load").collect();
            assert_eq!(loads.len(), 1, "got {:?}", result.blocks);
            assert!(loads[0].instances.contains(&"XM3".to_string()));
            assert!(loads[0].instances.contains(&"XM4".to_string()));
        }

        // Grouped — cell count reduced
        assert!(hg.cells.len() < 5, "cells after grouping: {}", hg.cells.len());
    }

    #[test]
    fn net_classification() {
        let hg = ota_hg();
        let roles = classify_nets(&hg, &AnnotationConfig::default());
        assert_eq!(roles[hg.net_id("VDD").unwrap() as usize], NetRole::Supply);
        assert_eq!(roles[hg.net_id("VSS").unwrap() as usize], NetRole::Ground);
        assert_eq!(roles[hg.net_id("vinp").unwrap() as usize], NetRole::Signal);
    }

    #[test]
    fn do_not_identify() {
        let mut hg = ota_hg();
        let cfg = AnnotationConfig {
            do_not_identify: ["XM1".into()].into(),
            ..Default::default()
        };
        let result = annotate(&mut hg, &cfg).expect("annotate");
        assert!(
            result.blocks.iter().all(|b| b.template != "diff_pair"),
            "diff pair should be suppressed"
        );
    }

    #[test]
    fn annotate_current_mirror() {
        // XM1: diode-connected (D=G=vref), XM2: output (D=iout, G=vref)
        let mut hg = BipartiteHypergraph {
            name: "mirror".into(),
            ports: vec!["vref".into(), "iout".into(), "VDD".into()],
            cells: vec![
                pfet("XM1", 0, 0, 2, 2, 5_000, 1_000), // D=vref G=vref S=VDD B=VDD
                pfet("XM2", 1, 0, 2, 2, 5_000, 1_000), // D=iout G=vref S=VDD B=VDD
            ],
            nets: vec!["vref".into(), "iout".into(), "VDD".into()],
            ..Default::default()
        };
        hg.build_csr();
        let result = annotate(&mut hg, &AnnotationConfig::default()).expect("annotate");
        assert_eq!(
            result.blocks.iter().filter(|b| b.template == "current_mirror").count(),
            1,
            "got {:?}", result.blocks
        );
    }

    #[test]
    fn annotate_cross_coupled() {
        // XM1: G=outn D=outp, XM2: G=outp D=outn
        let mut hg = BipartiteHypergraph {
            name: "latch".into(),
            ports: vec!["outp".into(), "outn".into(), "VDD".into(), "VSS".into()],
            cells: vec![
                pfet("XM1", 0, 1, 2, 2, 2_000, 500), // D=outp G=outn S=VDD B=VDD
                pfet("XM2", 1, 0, 2, 2, 2_000, 500), // D=outn G=outp S=VDD B=VDD
            ],
            nets: vec!["outp".into(), "outn".into(), "VDD".into(), "VSS".into()],
            ..Default::default()
        };
        hg.build_csr();
        let result = annotate(&mut hg, &AnnotationConfig::default()).expect("annotate");
        assert_eq!(
            result.blocks.iter().filter(|b| b.template == "cross_coupled").count(),
            1,
            "got {:?}", result.blocks
        );
    }

    #[test]
    fn dependency_chain_ota() {
        let mut hg = ota_hg();
        let result = annotate(&mut hg, &AnnotationConfig::default()).expect("annotate");

        let chain = &result.chain;

        // All recognized blocks should appear in the chain.
        assert!(!chain.blocks.is_empty(), "expected at least one block in chain");

        // Check every block template matches a recognized block.
        for node in &chain.blocks {
            assert!(
                result.blocks.iter().any(|b| b.template == node.template),
                "chain block '{}' not in recognized blocks",
                node.template,
            );
        }

        // The tail transistor (XM5) should be unclaimed — it's not part of
        // the diff_pair or active_load / diff_pair_with_active_load patterns.
        // It is the "top-level glue" that depends on all recognized blocks.
        let composite = result.blocks.iter()
            .any(|b| b.template == "diff_pair_with_active_load");
        if composite {
            // Composite matched 4 devices → XM5 is unclaimed.
            assert_eq!(chain.unclaimed.len(), 1, "expected 1 unclaimed cell (XM5), got {:?}", chain.unclaimed);
        } else {
            // Separate diff_pair + active_load → XM5 is still unclaimed.
            assert_eq!(chain.unclaimed.len(), 1, "expected 1 unclaimed cell (XM5), got {:?}", chain.unclaimed);
        }

        // Recognized blocks come before unclaimed (the implicit top-level).
        // Since all blocks have no inter-block dependencies, they have
        // empty depends_on.
        for node in &chain.blocks {
            assert!(
                node.depends_on.is_empty(),
                "block '{}' should have no dependencies at same hierarchy level",
                node.template,
            );
        }
    }

    #[test]
    fn merge_parallel_devices() {
        // Two identical nfets on the same nets → should merge
        let mut hg = BipartiteHypergraph {
            name: "test".into(),
            ports: vec![],
            cells: vec![
                nfet("M1", 0, 1, 2, 2, 1_000, 500),
                nfet("M2", 0, 1, 2, 2, 1_000, 500),
            ],
            nets: vec!["d".into(), "g".into(), "s".into()],
            ..Default::default()
        };
        hg.build_csr();
        merge_parallel(&mut hg);
        assert_eq!(hg.cells.len(), 1);
        assert_eq!(hg.cells[0].device.as_ref().unwrap().multiplier, 2);
    }
}
