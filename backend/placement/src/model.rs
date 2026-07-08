//! Bridge between frontend types and the engine's placement domain.
//! Builds [`PlaceCold`] from the hypergraph + constraints, and provides
//! the [`PlaceLedger`] that maps engine constraint checks to contract updates.

use std::collections::HashMap;

use pnr_constraints::{
    validate_constraint_record, ConstraintContract, ConstraintStatus, ConstraintStrength,
    Contractable, GROUND_NAMES, SUPPLY_NAMES,
};
use pnr_cells::netlist::BipartiteHypergraph;
use pnr_engine::placement::{
    eval_check, CcPattern, Csr, PairRule, PlaceCheck, PlaceCold, PlaceDomain, PlaceHot,
    SymTable, NONE,
};

use crate::ConstraintRecord;

const UM: f32 = 1000.0;

// ---------------------------------------------------------------------------
// Build cold context from the hypergraph + ConstraintRecord
// ---------------------------------------------------------------------------

pub fn build_cold(
    g: &BipartiteHypergraph,
    sizes: &[(i32, i32)],
    rec: &ConstraintRecord,
    die: (f32, f32),
    grid: f32,
    margin: i32,
    net_weight_overrides: &HashMap<String, f64>,
    layer_masks: &[u64],
) -> PlaceCold {
    let n = g.cells.len();
    let m = margin as f32 / 2.0;
    let mut cold = PlaceCold {
        hw: sizes.iter().map(|s| s.0 as f32 / 2.0 + m).collect(),
        hh: sizes.iter().map(|s| s.1 as f32 / 2.0 + m).collect(),
        layer_mask: layer_masks.to_vec(),
        die,
        grid,
        ..Default::default()
    };

    // Nets
    let mut net_rows: Vec<Vec<u32>> = Vec::new();
    let mut cell_rows: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (ni, name) in g.nets.iter().enumerate() {
        let mut cells: Vec<u32> = g.pins_on_net(ni as u32).iter().map(|&(c, _)| c).collect();
        cells.sort_unstable();
        cells.dedup();
        if cells.len() < 2 {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let mut w =
            if SUPPLY_NAMES.contains(&lower.as_str()) || GROUND_NAMES.contains(&lower.as_str()) {
                0.2
            } else {
                1.0
            };
        if let Some(nc) = rec.net_class.iter().find(|c| c.net_name == *name) {
            use pnr_constraints::NetClass::*;
            w = match nc.net_class {
                Sensitive => 3.0,
                Clock => 2.0,
                Supply | Ground | Substrate => 0.2,
                Signal => w,
            };
        }
        if let Some(&mult) = net_weight_overrides.get(name) {
            w *= mult as f32;
        }
        let row = net_rows.len() as u32;
        for &c in &cells {
            cell_rows[c as usize].push(row);
        }
        net_rows.push(cells);
        cold.net_w.push(w);
    }
    cold.nets = to_csr(&net_rows);
    cold.cell_nets = to_csr(&cell_rows);

    // Symmetry table
    let mut sym = SymTable {
        group_of: vec![NONE; n],
        partner: vec![NONE; n],
        groups: Vec::new(),
        fixed: Vec::new(),
        axis0: Vec::new(),
    };
    for sg in &rec.symmetry {
        let gi = sym.groups.len() as u32;
        let mut members = Vec::new();
        for p in &sg.pairs {
            let (a, b) = (p.device_a.0, p.device_b.0);
            if a as usize >= n || b as usize >= n {
                continue;
            }
            sym.group_of[a as usize] = gi;
            sym.group_of[b as usize] = gi;
            sym.partner[a as usize] = b;
            sym.partner[b as usize] = a;
            members.push(a);
            members.push(b);
        }
        for &d in &sg.self_symmetric {
            if (d.0 as usize) < n {
                sym.group_of[d.0 as usize] = gi;
                sym.partner[d.0 as usize] = d.0;
                members.push(d.0);
            }
        }
        sym.groups.push(members);
        sym.fixed.push(sg.axis.is_some());
        sym.axis0.push(sg.axis.map_or(die.0 / 2.0, |a| a as f32 / 10.0));
    }
    cold.sym = sym;

    // Pair rules
    for p in &rec.proximity {
        if let Some(r) = pair(n, p.device_a.0, p.device_b.0, p.min_distance_um) {
            cold.pulls.push(PairRule { weight: 1.0, ..r });
        }
    }
    for t in &rec.thermal {
        if let Some(r) = pair(n, t.device_a.0, t.device_b.0, 0.0) {
            cold.pulls.push(PairRule { weight: 3.0, ..r });
        }
    }
    for i in &rec.isolation {
        if let Some(r) = pair(n, i.device_a.0, i.device_b.0, i.min_distance_um) {
            cold.pushes.push(r);
        }
    }
    for cc in &rec.cc {
        let ga: Vec<u32> =
            cc.group_a.iter().map(|d| d.0).filter(|&d| (d as usize) < n).collect();
        let gb: Vec<u32> =
            cc.group_b.iter().map(|d| d.0).filter(|&d| (d as usize) < n).collect();
        if !ga.is_empty() && !gb.is_empty() {
            cold.cc.push((ga, gb));
            cold.cc_pattern.push(match cc.pattern {
                pnr_constraints::PatternType::Abba => CcPattern::Abba,
                pnr_constraints::PatternType::Abab => CcPattern::Abab,
                pnr_constraints::PatternType::CommonCentroid2d => CcPattern::CommonCentroid2d,
            });
        }
    }
    // Guard ring: treat as isolation push — guarded device needs extra spacing
    // from all other devices equal to min_width_um * 2 (ring on both sides).
    for gr in &rec.guard_ring {
        let d = gr.device_id.0;
        if (d as usize) >= n {
            continue;
        }
        let gap_nm = gr.min_width_um as f32 * 2.0 * UM;
        for other in 0..n as u32 {
            if other != d {
                cold.pushes.push(PairRule { a: d, b: other, gap_nm, weight: 1.0 });
            }
        }
    }
    for s in &rec.straight {
        if let Some(ni) = g.net_id(&s.net) {
            let mut cells: Vec<u32> = g.pins_on_net(ni).iter().map(|&(c, _)| c).collect();
            cells.sort_unstable();
            cells.dedup();
            if cells.len() >= 2 {
                cold.aligns.push((cells, s.vertical));
            }
        }
    }
    cold
}

fn pair(n: usize, a: u32, b: u32, dist_um: f64) -> Option<PairRule> {
    ((a as usize) < n && (b as usize) < n)
        .then_some(PairRule { a, b, gap_nm: dist_um as f32 * UM, weight: 1.0 })
}

fn to_csr(rows: &[Vec<u32>]) -> Csr {
    let mut csr = Csr { start: Vec::with_capacity(rows.len() + 1), items: Vec::new() };
    csr.start.push(0);
    for r in rows {
        csr.items.extend_from_slice(r);
        csr.start.push(csr.items.len() as u32);
    }
    csr
}

// ---------------------------------------------------------------------------
// ConstraintContract ledger
// ---------------------------------------------------------------------------

pub struct PlaceLedger {
    pub contracts: Vec<ConstraintContract>,
    checks: Vec<(usize, PlaceCheck)>,
    open: usize,
}

impl PlaceLedger {
    pub fn build(g: &BipartiteHypergraph, rec: &ConstraintRecord) -> Self {
        let names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        let n = names.len();
        let mut contracts = Vec::new();
        let mut checks = Vec::new();
        let add = |c: ConstraintContract,
                   ch: Option<PlaceCheck>,
                   checks: &mut Vec<(usize, PlaceCheck)>,
                   contracts: &mut Vec<ConstraintContract>| {
            let mut c = c;
            if let Some(ch) = ch {
                c.consume("placement");
                checks.push((contracts.len(), ch));
            }
            contracts.push(c);
        };

        for (gi, sg) in rec.symmetry.iter().enumerate() {
            for p in &sg.pairs {
                let ok = (p.device_a.0 as usize) < n && (p.device_b.0 as usize) < n;
                add(
                    p.to_contract(&names),
                    ok.then_some(PlaceCheck::SymPair {
                        a: p.device_a.0,
                        b: p.device_b.0,
                        g: gi as u32,
                    }),
                    &mut checks,
                    &mut contracts,
                );
            }
            let _ = &sg.self_symmetric;
        }
        for (i, cc) in rec.cc.iter().enumerate() {
            add(cc.to_contract(&names), Some(PlaceCheck::Cc { i }), &mut checks, &mut contracts);
        }
        for iso in &rec.isolation {
            let ok = (iso.device_a.0 as usize) < n && (iso.device_b.0 as usize) < n;
            add(
                iso.to_contract(&names),
                ok.then_some(PlaceCheck::MinGap {
                    a: iso.device_a.0,
                    b: iso.device_b.0,
                    min_nm: iso.min_distance_um as f32 * UM,
                }),
                &mut checks,
                &mut contracts,
            );
        }
        for p in &rec.proximity {
            let ok = (p.device_a.0 as usize) < n && (p.device_b.0 as usize) < n;
            add(
                p.to_contract(&names),
                ok.then_some(PlaceCheck::MaxDist {
                    a: p.device_a.0,
                    b: p.device_b.0,
                    max_nm: p.min_distance_um as f32 * UM,
                }),
                &mut checks,
                &mut contracts,
            );
        }
        // Guard ring: MinGap check against all neighboring devices
        for gr in &rec.guard_ring {
            let d = gr.device_id.0;
            if (d as usize) >= n {
                continue;
            }
            let gap_nm = gr.min_width_um as f32 * 2.0 * UM;
            let mut c = gr.to_contract(&names);
            c.consume("placement");
            for other in 0..n as u32 {
                if other != d {
                    checks.push((
                        contracts.len(),
                        PlaceCheck::MinGap { a: d, b: other, min_nm: gap_nm },
                    ));
                }
            }
            contracts.push(c);
        }
        for t in &rec.thermal {
            let ok = (t.device_a.0 as usize) < n && (t.device_b.0 as usize) < n;
            // ponytail: thermal gradient ∝ separation heuristic — satisfied when
            // the pair sits adjacent; real model at signoff.
            add(
                t.to_contract(&names),
                ok.then_some(PlaceCheck::MaxDist { a: t.device_a.0, b: t.device_b.0, max_nm: 0.0 }),
                &mut checks,
                &mut contracts,
            );
        }
        Self { contracts, checks, open: usize::MAX }
    }

    pub fn validation(&self) -> pnr_constraints::contract::ContractValidation {
        validate_constraint_record(&self.contracts)
    }

    pub fn summary(&self) -> String {
        let mut s = String::new();
        for c in &self.contracts {
            s.push_str(&format!(
                "{:60} {:9} {:?}{}\n",
                c.constraint_id,
                format!("{:?}", c.strength),
                c.status,
                c.violation_metric.map_or(String::new(), |m| format!(
                    " ({m:.1} {})",
                    c.violation_units.as_deref().unwrap_or("")
                )),
            ));
        }
        s
    }
}

impl pnr_engine::Ledger<PlaceDomain> for PlaceLedger {
    fn reconcile(&mut self, hot: &PlaceHot, cold: &PlaceCold) {
        let mut open = 0usize;
        for (ci, ch) in &self.checks {
            let (ok, metric) = eval_check(ch, hot, cold);
            let c = &mut self.contracts[*ci];
            let want = if ok { ConstraintStatus::Satisfied } else { ConstraintStatus::Violated };
            if c.status != want {
                if ok {
                    c.satisfy("placement", &format!("within {:.0} nm tolerance", cold.grid * 2.0));
                } else {
                    c.violate("placement", f64::from(metric) / f64::from(UM), "um");
                }
            }
            if !ok && c.strength == ConstraintStrength::Hard {
                open += 1;
            }
        }
        self.open = open;
    }

    fn open(&self) -> usize {
        self.open
    }
}
