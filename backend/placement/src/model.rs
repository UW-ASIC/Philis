//! Bridge between frontend types and the engine's placement domain.
//! Builds [`PlaceCold`] from the hypergraph + constraints, and provides
//! the [`PlaceLedger`] that maps engine constraint checks to contract updates.

use std::collections::HashMap;

use pnr_cells::netlist::BipartiteHypergraph;
use pnr_constraints::{
    validate_constraint_record, ConstraintContract, ConstraintStatus, ConstraintStrength,
    Contractable, GROUND_NAMES, SUPPLY_NAMES,
};
use pnr_engine::placement::{
    eval_check, Abutment, CcPattern, Csr, PairRule, PlaceCheck, PlaceCold, PlaceDomain, PlaceHot,
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
    variant_sizes: &[Vec<(i32, i32)>],
    initial_variants: &[usize],
    variant_pin_offsets: &[Vec<Vec<(u32, i32, i32)>>],
    variant_penalties: &[Vec<f64>],
    legal_abutments: &[Abutment],
    cell_inflation_x: &[f64],
    cell_inflation_y: &[f64],
) -> PlaceCold {
    let n = g.cells.len();
    let m = margin as f32 / 2.0;
    let inflate = |cell: usize, value: i32, factors: &[f64]| {
        let factor = factors.get(cell).copied().unwrap_or(1.0).max(1.0) as f32;
        value as f32 * factor / 2.0 + m
    };
    let variants: Vec<Vec<(f32, f32)>> = (0..n)
        .map(|i| {
            if i < variant_sizes.len() && variant_sizes[i].len() > 1 {
                variant_sizes[i]
                    .iter()
                    .map(|&(w, h)| {
                        (
                            inflate(i, w, cell_inflation_x),
                            inflate(i, h, cell_inflation_y),
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            }
        })
        .collect();
    let mut cold = PlaceCold {
        hw: sizes
            .iter()
            .enumerate()
            .map(|(i, s)| inflate(i, s.0, cell_inflation_x))
            .collect(),
        hh: sizes
            .iter()
            .enumerate()
            .map(|(i, s)| inflate(i, s.1, cell_inflation_y))
            .collect(),
        layer_mask: layer_masks.to_vec(),
        die,
        grid,
        cell_margin: margin as f32,
        variants,
        ..Default::default()
    };

    // Nets
    let mut net_rows: Vec<Vec<u32>> = Vec::new();
    let mut net_row_of = vec![NONE; g.nets.len()];
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
        net_row_of[ni] = row;
        for &c in &cells {
            cell_rows[c as usize].push(row);
        }
        net_rows.push(cells);
        cold.net_w.push(w);
    }
    cold.nets = to_csr(&net_rows);
    cold.cell_nets = to_csr(&cell_rows);

    // Variant-aware physical pin locations. Hypergraph net ids are remapped
    // to the compact placement-net rows built above.
    cold.variant_pins = (0..n)
        .map(|ci| {
            variant_pin_offsets.get(ci).map_or_else(Vec::new, |vars| {
                vars.iter()
                    .map(|pins| {
                        pins.iter()
                            .filter_map(|&(net, dx, dy)| {
                                let row = net_row_of.get(net as usize).copied().unwrap_or(NONE);
                                (row != NONE).then_some((row, dx as f32, dy as f32))
                            })
                            .collect()
                    })
                    .collect()
            })
        })
        .collect();
    for ci in 0..n {
        if let Some(base) = cold.variant_pins.get(ci).and_then(|v| v.first()) {
            for &(net, dx, dy) in base {
                cold.pin_cell.push(ci as u32);
                cold.pin_net.push(net);
                cold.pin_dx.push(dx);
                cold.pin_dy.push(dy);
            }
        }
    }
    cold.initial_variant = (0..n)
        .map(|ci| {
            initial_variants
                .get(ci)
                .copied()
                .unwrap_or(0)
                .min(variant_sizes.get(ci).map_or(1, Vec::len).saturating_sub(1)) as u16
        })
        .collect();
    cold.variant_penalty = variant_penalties.to_vec();
    cold.abutments = legal_abutments.to_vec();

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
        sym.axis0
            .push(sg.axis.map_or(die.0 / 2.0, |a| a as f32 / 10.0));
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
    // Stress: centroid distance from die center
    for s in &rec.stress {
        let d = s.device_id.0;
        if (d as usize) < n {
            cold.stress
                .push((d, s.max_centroid_distance_um as f32 * UM));
        }
    }
    // The engine operates on footprints inflated by `margin / 2` per edge.
    // Convert raw-bbox constraint distances once at this boundary; ledger
    // checks add the margin back through `gap()` and therefore keep reporting
    // the user/PDK-facing raw distance.
    let effective_min_gap = |raw_nm: f32| (raw_nm - margin as f32).max(0.0);
    let effective_band_edge = |raw_nm: f32| raw_nm - margin as f32;

    // DTI: deep trench isolation pair zones
    for d in &rec.dti {
        let (a, b) = (d.device_a.0, d.device_b.0);
        if (a as usize) < n && (b as usize) < n {
            cold.dti_zones.push((
                a,
                b,
                effective_band_edge(d.s_max as f32 * UM),
                effective_band_edge(d.d_dti as f32 * UM),
            ));
        }
    }
    for i in &rec.isolation {
        let effective_um = f64::from(effective_min_gap(i.min_distance_um as f32 * UM) / UM);
        if let Some(r) = pair(n, i.device_a.0, i.device_b.0, effective_um) {
            cold.pushes.push(r);
        }
    }
    // CC groups → flat CSR (side A = row 2i, side B = row 2i+1)
    let mut cc_rows: Vec<Vec<u32>> = Vec::new();
    for cc in &rec.cc {
        let ga: Vec<u32> = cc
            .group_a
            .iter()
            .map(|d| d.0)
            .filter(|&d| (d as usize) < n)
            .collect();
        let gb: Vec<u32> = cc
            .group_b
            .iter()
            .map(|d| d.0)
            .filter(|&d| (d as usize) < n)
            .collect();
        if !ga.is_empty() && !gb.is_empty() {
            cc_rows.push(ga);
            cc_rows.push(gb);
            cold.cc_pattern.push(match cc.pattern {
                pnr_constraints::PatternType::Abba => CcPattern::Abba,
                pnr_constraints::PatternType::Abab => CcPattern::Abab,
                pnr_constraints::PatternType::CommonCentroid2d => CcPattern::CommonCentroid2d,
            });
        }
    }
    cold.cc = to_csr(&cc_rows);
    // Guard ring: treat as isolation push — guarded device needs extra spacing
    // from all other devices equal to min_width_um * 2 (ring on both sides).
    for gr in &rec.guard_ring {
        let d = gr.device_id.0;
        if (d as usize) >= n {
            continue;
        }
        let gap_nm = effective_min_gap(gr.min_width_um as f32 * 2.0 * UM);
        for other in 0..n as u32 {
            if other != d {
                cold.pushes.push(PairRule {
                    a: d,
                    b: other,
                    gap_nm,
                    weight: 1.0,
                });
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

    // Boundary validation: constraint device refs outside the cell arena are
    // silently skipped by the loops above — surface the count once.
    let oob = |a: u32| (a as usize) >= n;
    let dropped = rec
        .proximity
        .iter()
        .filter(|p| oob(p.device_a.0) || oob(p.device_b.0))
        .count()
        + rec
            .thermal
            .iter()
            .filter(|t| oob(t.device_a.0) || oob(t.device_b.0))
            .count()
        + rec.stress.iter().filter(|s| oob(s.device_id.0)).count()
        + rec
            .dti
            .iter()
            .filter(|d| oob(d.device_a.0) || oob(d.device_b.0))
            .count()
        + rec
            .isolation
            .iter()
            .filter(|i| oob(i.device_a.0) || oob(i.device_b.0))
            .count()
        + rec.guard_ring.iter().filter(|g| oob(g.device_id.0)).count()
        + rec
            .symmetry
            .iter()
            .flat_map(|sg| &sg.pairs)
            .filter(|p| oob(p.device_a.0) || oob(p.device_b.0))
            .count();
    if dropped > 0 {
        eprintln!("[placement] WARNING: {dropped} constraints reference devices outside the {n}-cell arena; dropped");
    }

    // -- 2.2 + 2.3 + 3.3: per-pair min-distance + class-pair spacing table --
    // Populate pair_min_dist from isolation constraints
    for i in &rec.isolation {
        let (a, b) = (i.device_a.0, i.device_b.0);
        if (a as usize) < n && (b as usize) < n {
            cold
                .pair_min_dist
                .push((a, b, effective_min_gap(i.min_distance_um as f32 * UM)));
        }
    }
    cold.normalize_pair_min_dist();

    // 2.3: assign device class index from DeviceType
    // ponytail: 5 classes (nmos=0, pmos=1, res=2, cap=3, other=4) — extend when PDK needs finer
    cold.class_count = 5;
    cold.class_idx = g
        .cells
        .iter()
        .map(|c| match &c.device {
            Some(d) => match d.device_type {
                pnr_cells::DeviceType::Nmos => 0,
                pnr_cells::DeviceType::Pmos => 1,
                pnr_cells::DeviceType::Res => 2,
                pnr_cells::DeviceType::Cap
                | pnr_cells::DeviceType::Ncap
                | pnr_cells::DeviceType::Pcap => 3,
                _ => 4,
            },
            None => 4,
        })
        .collect();

    // Dense class-pair spacing table (5x5). The inflated footprint already
    // provides the base `device_gap`; process-specific well/isolation rules
    // arrive as explicit pair constraints and must not be guessed again here.
    let nc = cold.class_count as usize;
    cold.class_spacing = vec![0.0; nc * nc];

    // Parse-don't-validate: `build_cold` is the single boundary where a
    // PlaceCold is born — downstream hot loops index unchecked, so the
    // parallel-array invariants must hold HERE.
    debug_assert_eq!(cold.hw.len(), n);
    debug_assert_eq!(cold.hh.len(), n);
    debug_assert_eq!(cold.sym.group_of.len(), n);
    debug_assert_eq!(cold.sym.partner.len(), n);
    debug_assert_eq!(cold.sym.groups.len(), cold.sym.fixed.len());
    debug_assert_eq!(cold.sym.groups.len(), cold.sym.axis0.len());
    debug_assert_eq!(cold.cc_count(), cold.cc_pattern.len());
    debug_assert_eq!(cold.class_idx.len(), n);
    debug_assert_eq!(cold.initial_variant.len(), n);
    debug_assert_eq!(cold.class_spacing.len(), (cold.class_count as usize).pow(2));
    debug_assert_eq!(cold.nets.start.len().saturating_sub(1), cold.net_w.len());
    debug_assert!(cold
        .pair_min_dist
        .windows(2)
        .all(|w| (w[0].0, w[0].1) < (w[1].0, w[1].1)));

    cold
}

fn pair(n: usize, a: u32, b: u32, dist_um: f64) -> Option<PairRule> {
    ((a as usize) < n && (b as usize) < n).then_some(PairRule {
        a,
        b,
        gap_nm: dist_um as f32 * UM,
        weight: 1.0,
    })
}

fn to_csr(rows: &[Vec<u32>]) -> Csr {
    let mut csr = Csr {
        start: Vec::with_capacity(rows.len() + 1),
        items: Vec::new(),
    };
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
            add(
                cc.to_contract(&names),
                Some(PlaceCheck::Cc { i }),
                &mut checks,
                &mut contracts,
            );
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
                        PlaceCheck::MinGap {
                            a: d,
                            b: other,
                            min_nm: gap_nm,
                        },
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
                ok.then_some(PlaceCheck::MaxDist {
                    a: t.device_a.0,
                    b: t.device_b.0,
                    max_nm: 0.0,
                }),
                &mut checks,
                &mut contracts,
            );
        }
        for s in &rec.stress {
            let ok = (s.device_id.0 as usize) < n;
            add(
                s.to_contract(&names),
                ok.then_some(PlaceCheck::CentroidDist {
                    d: s.device_id.0,
                    max_nm: s.max_centroid_distance_um as f32 * UM,
                }),
                &mut checks,
                &mut contracts,
            );
        }
        for d in &rec.dti {
            let ok = (d.device_a.0 as usize) < n && (d.device_b.0 as usize) < n;
            add(
                d.to_contract(&names),
                ok.then_some(PlaceCheck::OutsideBand {
                    a: d.device_a.0,
                    b: d.device_b.0,
                    min_nm: d.s_max as f32 * UM,
                    max_nm: d.d_dti as f32 * UM,
                }),
                &mut checks,
                &mut contracts,
            );
        }
        Self {
            contracts,
            checks,
            open: usize::MAX,
        }
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
                c.status(),
                c.violation_metric().map_or(String::new(), |m| format!(
                    " ({m:.1} {})",
                    c.violation_units().unwrap_or("")
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
            let want = if ok {
                ConstraintStatus::Satisfied
            } else {
                ConstraintStatus::Violated
            };
            if c.status() != want {
                if ok {
                    c.satisfy(
                        "placement",
                        &format!("within {:.0} nm tolerance", cold.grid * 2.0),
                    );
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
