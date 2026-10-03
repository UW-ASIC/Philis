//! Recognition regression net — ported from `frontend/annotator`'s tests onto
//! `pnr_core::Netlist`. Proves the DSL catalog recognises the canonical analog
//! primitives after the port, and that the `do_not_identify` override suppresses a
//! block.

use crate::{annotate, AnnotationConfig, Block, BlockKind};
use analog::RuleBatch;
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::{Device, DeviceKind, Net, Netlist};

/// A FET with G,D,S,B terminals on the given net ids, W/L in nm.
pub(crate) fn fet(name: &str, kind: DeviceKind, g: u16, d: u16, s: u16, b: u16, w: i64, l: i64) -> Device {
    Device {
        name: name.into(),
        kind, model: String::new(),
        terminals: vec![
            ("G".into(), NetId(g)),
            ("D".into(), NetId(d)),
            ("S".into(), NetId(s)),
            ("B".into(), NetId(b)),
        ],
        params: vec![("w".into(), w), ("l".into(), l)],
    }
}

pub(crate) fn nets(names: &[&str]) -> Vec<Net> {
    names.iter().map(|n| Net { name: (*n).into() }).collect()
}

/// Block (non-glue) containing device id `d`, if any.
fn block_of(blocks: &[Block], d: u16) -> Option<&Block> {
    blocks
        .iter()
        .find(|b| !matches!(b.kind, BlockKind::Glue) && b.devices.contains(&DeviceId(d)))
}

/// 5-transistor OTA. Nets: 0=vout1 1=vinp 2=vtail 3=VSS 4=vout2 5=vinm 6=vbias
/// 7=VDD 8=vbn.
fn ota() -> Netlist {
    Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Nmos, 1, 0, 2, 3, 10_000, 1_000),
            fet("XM2", DeviceKind::Nmos, 5, 4, 2, 3, 10_000, 1_000),
            fet("XM3", DeviceKind::Pmos, 6, 0, 7, 7, 20_000, 1_000),
            fet("XM4", DeviceKind::Pmos, 6, 4, 7, 7, 20_000, 1_000),
            fet("XM5", DeviceKind::Nmos, 8, 2, 3, 3, 40_000, 2_000),
        ],
        nets: nets(&["vout1", "vinp", "vtail", "VSS", "vout2", "vinm", "vbias", "VDD", "vbn"]),
    }
}

#[test]
fn diff_pair_halves_share_a_block() {
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    // XM1 (0) and XM2 (1) must land in the same recognised block — whether the
    // winner is a bare diff_pair or a composite (five_transistor_ota / DP+load).
    let b1 = block_of(&p.blocks, 0).expect("XM1 recognised");
    let b2 = block_of(&p.blocks, 1).expect("XM2 recognised");
    assert!(std::ptr::eq(b1, b2), "diff-pair halves split across blocks");
}

#[test]
fn current_mirror_recognised() {
    // XM1 diode-connected (D=G=vref net 0), XM2 output (D=iout net 1, G=vref).
    let nl = Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Pmos, 0, 0, 2, 2, 5_000, 1_000),
            fet("XM2", DeviceKind::Pmos, 0, 1, 2, 2, 5_000, 1_000),
        ],
        nets: nets(&["vref", "iout", "VDD"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    let b = block_of(&p.blocks, 0).expect("mirror recognised");
    assert_eq!(b.kind, BlockKind::CurrentMirror);
    assert_eq!(b.devices, [DeviceId(0), DeviceId(1)], "slot 0 is the diode reference");
}

#[test]
fn cross_coupled_recognised() {
    // XM1: G=outn(1) D=outp(0); XM2: G=outp(0) D=outn(1).
    let nl = Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Pmos, 1, 0, 2, 2, 2_000, 500),
            fet("XM2", DeviceKind::Pmos, 0, 1, 2, 2, 2_000, 500),
        ],
        nets: nets(&["outp", "outn", "VDD", "VSS"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    let b = block_of(&p.blocks, 0).expect("cross-coupled recognised");
    assert_eq!(b.kind, BlockKind::DiffPair, "cross-coupled pairs are symmetric matched pairs");
}

#[test]
fn do_not_identify_suppresses_block() {
    let nl = ota();
    let cfg = AnnotationConfig { do_not_identify: [0u32].into_iter().collect(), ..Default::default() };
    let p = annotate(&nl, &cfg);
    // XM1 (device 0) is blocked → it cannot appear in any recognised block; it
    // falls through to glue.
    assert!(block_of(&p.blocks, 0).is_none(), "XM1 should be suppressed from recognition");
    let glue = p.blocks.iter().find(|b| matches!(b.kind, BlockKind::Glue)).expect("glue");
    assert!(glue.devices.contains(&DeviceId(0)));
}

/// Find a leaf (top block or nested sub_block) of `kind` covering exactly the
/// given device ids, searching the hierarchy.
fn has_leaf(blocks: &[Block], kind: BlockKind, ids: &[u16]) -> bool {
    fn walk(b: &Block, kind: BlockKind, want: &[u16]) -> bool {
        let here = b.kind == kind && {
            let mut got: Vec<u16> = b.devices.iter().map(|d| d.0).collect();
            got.sort_unstable();
            let mut w = want.to_vec();
            w.sort_unstable();
            got == w
        };
        here || b.sub_blocks.iter().any(|c| walk(c, kind, want))
    }
    blocks.iter().any(|b| walk(b, kind, ids))
}

/// Total individual rules across a hard/cost batch list.
fn count(batches: &[Box<dyn RuleBatch<pnr_core::Layout>>]) -> usize {
    batches.iter().map(|b| b.count()).sum()
}

#[test]
fn diff_pair_lives_in_the_hierarchy() {
    // Whether the OTA matches as a composite (diff pair swallowed → recovered as a
    // sub_block) or as a standalone diff_pair, a DiffPair leaf covering {XM1,XM2}
    // must exist somewhere in the hierarchy.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    assert!(has_leaf(&p.blocks, BlockKind::DiffPair, &[0, 1]), "no DiffPair leaf for XM1/XM2");
}

#[test]
fn diff_pair_emits_its_constraints() {
    // A recognised diff pair emits Symmetry (hard), ThermalGradient (budget — a
    // spec plus a margin on a derived field, PLAN §4d) and MatchingPair +
    // CommonCentroid (cost) — the BlockKind mapping, driven by recognition.
    // MatchingPair is Cost, not Hard: its `satisfied` bounds device AREA
    // (Pelgrom `σ²_u = A²/(W·L)`), which no placement move can change, so
    // gating SA on it is inert. See `backend/TODO.md` §2.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    assert!(count(&p.placement.hard) >= 1, "expected >=1 hard placement rule (sym), got {}", count(&p.placement.hard));
    assert!(count(&p.placement.budget) >= 1, "expected >=1 budget placement rule (thermal), got {}", count(&p.placement.budget));
    assert!(count(&p.placement.cost) >= 2, "expected >=2 cost placement rules (match/CC/prox), got {}", count(&p.placement.cost));

    // The partition itself is the contract: nothing in `hard` may be a rule the
    // placer cannot act on.
    let hard_kinds: Vec<&str> = p.placement.hard.iter().map(|b| b.kind()).collect();
    assert!(
        !hard_kinds.iter().any(|k| k.contains("MatchingPair")),
        "MatchingPair must not gate placement moves: {hard_kinds:?}"
    );
}

#[test]
fn budget_rules_land_in_exactly_one_partition() {
    // D15's invariant, and the one this partition exists to establish: the two-arm
    // code simulated the missing middle tier by registering a batch in *both* `hard`
    // and `cost` via `clone`, and a batch in two arms is counted twice by
    // `Report::lex`, priced twice by `gp::Prices`, and sits both above and below the
    // feasibility frontier. Double-registration is a `clone` away, so it is asserted.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    let arms: [(&str, &Vec<Box<dyn RuleBatch<pnr_core::Routes>>>); 3] =
        [("hard", &p.routing.hard), ("budget", &p.routing.budget), ("cost", &p.routing.cost)];
    for kind in ["CrosstalkExclusion", "ParasiticBudget", "CouplingBudget"] {
        let hits: Vec<&str> = arms
            .iter()
            .filter(|(_, a)| a.iter().any(|b| b.kind().ends_with(kind)))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(hits, ["budget"], "{kind} must be registered once, in `budget`");
    }

    // The trap, from the other side: `SymmetryGroup` matches the double-registration
    // shape that *locates* a budget and is emphatically not one — an exact equality is
    // not tradeable at any price, and pricing it is PLAN §4a's park-one-grid-unit-off
    // failure reported as convergence.
    // (`SymmetryGroup` reports itself as `"Symmetry"` — the rule it enforces, not the
    // container.)
    let sym = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| {
        a.iter().any(|b| b.kind() == "Symmetry")
    };
    assert!(sym(&p.placement.hard), "SymmetryGroup must stay Hard");
    assert!(!sym(&p.placement.budget), "SymmetryGroup must never be priced as a budget");

    // And the placement-tier budget, from the other other side: `ThermalGradient`
    // carries a spec *and* a margin on a derived field, is tradeable while
    // converging, and PLAN §4d calls it a budget outright. Its cost copy (the
    // per-move isotherm pull over the epoch-frozen field — see
    // `emit::budget_and_cost`) is a gradient, not a classification; the classified
    // copy must be priced, never gated.
    let therm = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| {
        a.iter().any(|b| b.kind().ends_with("ThermalGradient"))
    };
    assert!(therm(&p.placement.budget), "ThermalGradient must be a priced budget");
    assert!(!therm(&p.placement.hard), "ThermalGradient must never gate legality as Hard");
}

#[test]
fn parasitic_and_coupling_report_an_overshoot_not_a_count() {
    // D17: Θ sums residuals, and a residual is only summable normalised by its own
    // budget. These two batches carry a spec *and* a measured quantity, so a budget
    // blown many times over must read as many times worse — the property a count cannot
    // express, and the one that lets the search sit on a huge violation forever.
    use pnr_core::geom::{LayerId, Rect, Shape};
    let wire = |y: i32| Shape { layer: LayerId(0), rect: Rect { x: 0, y, w: 100_000_000, h: 1 } };
    // 100 mm of metal per net — past every class's length budget by ~10×; the two runs
    // are parallel 1 nm apart, so the coupling sum is past its budget too.
    let routes = pnr_core::routes::Routes { wires: vec![vec![wire(0)], vec![wire(2)]], ..Default::default()  };

    // Budgets exist only when the deck gives Cox and wire C (sky130-like numbers).
    let mut cfg = AnnotationConfig::default();
    cfg.process.gate_af_per_um2 = Some(8_325.0);
    cfg.process.wire_af_per_um = Some(50.0);
    let p = annotate(&ota(), &cfg);
    for kind in ["ParasiticBudget", "CouplingBudget"] {
        let b = p
            .routing
            .budget
            .iter()
            .find(|b| b.kind().ends_with(kind))
            .expect("budget batch");
        let (v, res) = (f64::from(b.violations(&routes)), b.residual(&routes));
        assert!(v > 0.0, "{kind}: fixture must actually violate");
        assert!(res > v, "{kind}: residual {res} degraded to a count of {v}");
    }
}

#[test]
fn dti_bands_are_one_hard_batch_with_dense_ids_seeded_share() {
    // The producer half of PLAN §4b: every matched pair in the OTA (diff pair,
    // mirror legs, load) gets a `DtiBand`, all in ONE batch registered hard+cost —
    // the hard copy is legality (the full disjunction), the cost copy is what makes
    // `dp`'s branch flip priceable, and the budget arm never sees it (a disjunction
    // is not tradeable, and `Prices::bind` asserts hard ∩ budget = ∅).
    // Bands exist only on a trench process: the deck supplies the rule.
    let mut cfg = AnnotationConfig::default();
    cfg.process.dti = Some((2_000, 1_000));
    let p = annotate(&ota(), &cfg);
    let is_dti = |b: &Box<dyn RuleBatch<pnr_core::Layout>>| b.kind().ends_with("DtiBand");
    assert_eq!(
        p.placement.hard.iter().filter(|b| is_dti(b)).count(),
        1,
        "one DtiBand batch, registered once at the end of placement()"
    );
    assert!(p.placement.cost.iter().any(is_dti), "the cost copy prices the flip");
    assert!(!p.placement.budget.iter().any(is_dti), "a disjunction is never a budget");

    // Dense, distinct, allocation order = emission order — the id contract that
    // keeps one pair's commitment from silently transferring to another.
    let mut seeds = Vec::new();
    let dti = p.placement.hard.iter().find(|b| is_dti(b)).unwrap();
    dti.branches(&mut seeds);
    assert!(seeds.len() >= 2, "the OTA has more than one matched pair: {seeds:?}");
    for (i, &(id, isolate)) in seeds.iter().enumerate() {
        assert_eq!(usize::from(id.0), i, "ids must be dense in emission order");
        assert!(!isolate, "matched structures seed `share` — one trench by design");
    }
}


#[test]
fn abutment_excludes_mixed_polarity_groups() {
    // D16: `groups` is the recognition table and deliberately contains composites
    // spanning both polarities; `abutment` is diffusion-sharing permission. Granting
    // abutment on a mixed group lets an NMOS and a PMOS merge implants — DRC-clean,
    // LVS-fatal, and unrepairable downstream because nothing is illegal.
    let nl = ota(); // XM1/XM2/XM5 Nmos, XM3/XM4 Pmos
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.abutment.len(), p.groups.len(), "abutment is indexed by GroupId too");
    for (gi, (grp, ab)) in p.groups.iter().zip(&p.abutment).enumerate() {
        let mut kinds = grp.iter().filter_map(|d| nl.devices.get(d.0 as usize)).map(|d| d.kind);
        let first = kinds.next();
        let same = kinds.all(|k| Some(k) == first);
        // A mixed group is truncated to one member (which grants no abutment, since
        // `group_index` ignores groups below two members) rather than emptied — an
        // empty group panics `Layout::bbox`.
        assert_eq!(ab.len(), if same { grp.len() } else { 1 }, "group {gi}: {grp:?} -> {ab:?}");
    }
}

#[test]
fn glue_only_netlist_emits_no_placement() {
    // Two unrelated resistors → nothing recognised → no placement constraints.
    let nl = Netlist {
        devices: vec![
            Device { name: "R1".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(0)), ("B".into(), NetId(1))], params: vec![] },
            Device { name: "R2".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(2)), ("B".into(), NetId(3))], params: vec![] },
        ],
        nets: nets(&["a", "b", "c", "d"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(count(&p.placement.hard), 0);
    assert_eq!(count(&p.placement.cost), 0);
}








#[test]
fn every_device_accounted_for() {
    // No device is lost: recognised ∪ glue == all devices, disjoint.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    let mut seen = vec![false; nl.devices.len()];
    for b in &p.blocks {
        for d in &b.devices {
            assert!(!seen[d.0 as usize], "device {} in two blocks", d.0);
            seen[d.0 as usize] = true;
        }
    }
    assert!(seen.into_iter().all(|s| s), "some device unaccounted for");
}


#[test]
fn a_unitization_never_mixes_kinds_or_sizes() {
    // `unit_w`/`unit_l`/`device_type` are one-per-group scalars, so every member
    // must genuinely carry them. When they did not (the block-level W read off
    // `devices[0]`), `cells::builder::sizing` drew the odd member at the wrong
    // size and LVS reported `lvs.parameter_mismatch`. The OTA mixes N/P and
    // three widths inside its recognised blocks, so it is the case that bites.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    let param = |d: DeviceId, k: &str| {
        nl.devices[d.0 as usize].params.iter().find(|(n, _)| n == k).map(|&(_, v)| v)
    };
    for u in &p.constraints.unitization {
        for &d in &u.devices {
            assert_eq!(nl.devices[d.0 as usize].kind, u.device_type, "unitization {:?} mixes kinds", u.devices);
            let w_finger = nl.devices[d.0 as usize].mos_size().map(|s| s.w_finger_nm());
            assert_eq!(w_finger, Some(i64::from(u.unit_w)), "unitization {:?} mixes W", u.devices);
            assert_eq!(param(d, "l"), Some(i64::from(u.unit_l)), "unitization {:?} mixes L", u.devices);
        }
    }
}

#[test]
fn guard_rings_tie_to_the_guarded_device_s_bulk() {
    // A ring is a substrate/well tap: it must land on the bulk rail, never on
    // whatever net the netlist numbered first. `connection_net` used to be a
    // hardcoded `NetId(0)` — here `vout1` — and `dr` folds ring pins in as real
    // routing terminals, so the router wired every guard ring in the design to
    // that signal net.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    assert!(!p.constraints.guard_rings.is_empty(), "matched FETs get rings");
    for r in &p.constraints.guard_rings {
        let dev = &nl.devices[r.device.0 as usize];
        let bulk = dev.terminals.iter().find(|(t, _)| t == "B").expect("a FET states a bulk").1;
        assert_eq!(
            r.connection_net, bulk,
            "{}'s ring ties to net {} instead of its bulk {}",
            dev.name, r.connection_net.0, bulk.0
        );
    }
}

#[test]
fn a_differential_stage_is_symmetric_about_one_axis() {
    // 5T OTA with a biased (non-diode) load: one five_transistor_ota stage. The
    // diff pair and the load mirror about the stage axis; the tail sits on it.
    let p = annotate(&ota(), &AnnotationConfig::default());
    assert_eq!(p.blocks[0].devices.len(), 5, "tail and biased load belong to the OTA stage");
    let sym: usize = p.placement.hard.iter().filter(|b| b.kind() == "Symmetry").map(|b| b.count()).sum();
    assert_eq!(sym, 3, "diff pair + load pair + self-symmetric tail");
    let tail = p.constraints.unitization.iter().find(|u| u.devices == [DeviceId(4)]);
    assert!(tail.is_some(), "the tail gets a sizing directive");
    assert!(p.constraints.guard_rings.iter().any(|g| g.device == DeviceId(4)), "and a ring");
}

#[test]
fn a_shared_gate_chain_is_a_series_stack_not_a_cascode() {
    // chain4: four same-size nfets in series on one gate net.
    let nl = Netlist {
        devices: (0..4u16)
            .map(|i| fet(&format!("M{i}"), DeviceKind::Nmos, 0, i + 1, i + 2, 6, 2_000, 500))
            .collect(),
        nets: nets(&["g", "a", "b", "c", "d", "e", "VSS"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.blocks[0].devices.len(), 4, "one series_stack_4 group");
    assert!(p.blocks[0].sub_blocks.iter().all(|b| b.kind == BlockKind::Stack), "stack pairs, not a cascode");
}

#[test]
fn a_cascode_stack_is_adjacent_not_matched() {
    // M1 (bottom) drain feeds M2 (top) source: a stack, not a matched pair.
    let nl = Netlist {
        devices: vec![
            fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 4_000, 500),
            fet("M2", DeviceKind::Nmos, 3, 4, 1, 2, 8_000, 500),
        ],
        nets: nets(&["vin", "x", "VSS", "vcas", "out"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.blocks[0].kind, BlockKind::Stack);
    let kinds: Vec<&str> = p.placement.cost.iter().map(|b| b.kind()).collect();
    assert!(kinds.iter().any(|k| k.ends_with("Proximity")), "{kinds:?}");
    assert!(!kinds.iter().any(|k| k.ends_with("MatchingPair") || k.ends_with("ThermalGradient")), "{kinds:?}");
    assert!(p.placement.hard.is_empty());
}

#[test]
fn antenna_gate_area_lands_on_the_gate_net_not_the_drain() {
    // G ≠ D on purpose: a diode-connected device would hide an index slip.
    let nl = Netlist {
        devices: vec![fet("XM1", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000)],
        nets: nets(&["g", "d", "VSS"]),
    };
    let mut cfg = AnnotationConfig::default();
    cfg.process.antenna_max_ratio = Some(400.0);
    let p = annotate(&nl, &cfg);
    let ant = p.routing.hard.iter().find(|b| b.kind().ends_with("Antenna")).expect("antenna batch");
    let mut nets_hit = Vec::new();
    ant.touched(&mut nets_hit);
    assert_eq!(nets_hit, vec![0], "gate area charged to net {nets_hit:?}, want the gate (0)");
}

#[test]
fn shields_are_requested_only_against_a_clock() {
    let is_shield = |b: &Box<dyn RuleBatch<pnr_core::Routes>>| b.kind().ends_with("Shield");
    let quiet = annotate(&ota(), &AnnotationConfig::default());
    assert!(!quiet.routing.budget.iter().any(is_shield), "no aggressor, no shield");

    // The same OTA plus a clocked switch on its own nets.
    let mut nl = ota();
    nl.nets.push(Net { name: "clk".into() });
    nl.nets.push(Net { name: "sw".into() });
    let (clk, sw) = (nl.nets.len() as u16 - 2, nl.nets.len() as u16 - 1);
    nl.devices.push(fet("XS", DeviceKind::Nmos, clk, sw, 3, 3, 1_000, 150));
    nl.devices.push(fet("XC", DeviceKind::Nmos, clk, 0, 3, 3, 1_000, 150));
    let clocked = annotate(&nl, &AnnotationConfig::default());
    let b = clocked.routing.budget.iter().find(|b| is_shield(b)).expect("shield batch");
    let mut pairs = Vec::new();
    b.shield_pairs(&mut pairs);
    assert!(!pairs.is_empty());
    let vss = 3u32;
    assert!(pairs.iter().all(|&(_, r)| r == vss), "shielded by ground: {pairs:?}");
}

#[test]
fn a_shielded_victim_books_no_coupling_to_its_shield() {
    // RTE-02: the ground shield the clocked OTA asks for is not an aggressor.
    use pnr_core::geom::{LayerId, Rect, Shape};
    let mut nl = ota();
    nl.nets.push(Net { name: "clk".into() });
    nl.nets.push(Net { name: "sw".into() });
    let (clk, sw) = (nl.nets.len() as u16 - 2, nl.nets.len() as u16 - 1);
    nl.devices.push(fet("XS", DeviceKind::Nmos, clk, sw, 3, 3, 1_000, 150));
    nl.devices.push(fet("XC", DeviceKind::Nmos, clk, 0, 3, 3, 1_000, 150));
    let mut cfg = AnnotationConfig::default();
    cfg.process.gate_af_per_um2 = Some(8_325.0);
    cfg.process.wire_af_per_um = Some(50.0);
    let p = annotate(&nl, &cfg);
    let mut pairs = Vec::new();
    p.routing.budget.iter().find(|b| b.kind().ends_with("Shield")).expect("shield batch").shield_pairs(&mut pairs);
    let (victim, vss) = pairs[0];
    let coup = p.routing.budget.iter().find(|b| b.kind().ends_with("CouplingBudget")).expect("coupling batch");
    // 100 mm of victim between its shield tracks 1 nm away: far past any budget.
    let wire = |y: i32| Shape { layer: LayerId(0), rect: Rect { x: 0, y, w: 100_000_000, h: 1 } };
    let mut wires = vec![Vec::new(); nl.nets.len()];
    wires[victim as usize] = vec![wire(0)];
    wires[vss as usize] = vec![wire(-2), wire(2)];
    let shielded = pnr_core::routes::Routes { wires: wires.clone(), ..Default::default() };
    assert_eq!(coup.violations(&shielded), 0, "its own shield is not an aggressor");
    // The same track held by the clock is.
    wires[vss as usize].clear();
    wires[clk as usize] = vec![wire(2)];
    let clocked = pnr_core::routes::Routes { wires, ..Default::default() };
    assert!(coup.violations(&clocked) > 0, "a clock beside the victim still counts");
}

#[test]
fn a_lone_mirror_stage_is_symmetric_too() {
    // No diff pair in the stage: the mirror pair still shares the stage axis,
    // hard (the equality) and cost (the pull toward it).
    let nl = Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Pmos, 0, 0, 2, 2, 5_000, 1_000),
            fet("XM2", DeviceKind::Pmos, 0, 1, 2, 2, 5_000, 1_000),
        ],
        nets: nets(&["vref", "iout", "VDD"]),
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    let sym = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| {
        a.iter().filter(|b| b.kind() == "Symmetry").map(|b| b.count()).sum::<usize>()
    };
    assert_eq!(sym(&p.placement.hard), 1);
    assert_eq!(sym(&p.placement.cost), 1);
    let mut pairs = Vec::new();
    p.placement.hard.iter().for_each(|b| b.mirror_pairs(&mut pairs));
    assert_eq!(pairs, [(0, 1, 0)], "the reference and output mirror about stage 0's axis");
}

#[test]
fn matching_is_budgeted_only_with_the_deck_s_mismatch_data() {
    let is_mp = |b: &Box<dyn RuleBatch<pnr_core::Layout>>| b.kind().ends_with("MatchingPair");
    let bare = annotate(&ota(), &AnnotationConfig::default());
    assert!(!bare.placement.budget.iter().any(is_mp), "no S_VT: a pull, not a budget");
    assert!(bare.missing.iter().any(|m| m.0 == "MatchingPair"), "and listed unknown");
    assert!(
        bare.placement.budget.iter().any(|b| b.kind() == "CommonCentroid"),
        "coincidence needs no deck data"
    );

    let mut cfg = AnnotationConfig::default();
    cfg.process.avt_mv_um = [Some(5.0), Some(6.0)];
    cfg.process.svt_uv_per_um = Some(4.0);
    let full = annotate(&ota(), &cfg);
    assert!(full.placement.budget.iter().any(is_mp));
    assert!(!full.missing.iter().any(|m| m.0 == "MatchingPair"));
}

#[test]
fn clocked_devices_are_kept_away_from_matched_ones() {
    let is_iso = |b: &Box<dyn RuleBatch<pnr_core::Layout>>| b.kind().ends_with("Isolation");
    let quiet = annotate(&ota(), &AnnotationConfig::default());
    assert!(!quiet.placement.cost.iter().any(is_iso), "no aggressor, no isolation");

    let mut nl = ota();
    nl.nets.push(Net { name: "clk".into() });
    nl.nets.push(Net { name: "sw".into() });
    let (clk, sw) = (nl.nets.len() as u16 - 2, nl.nets.len() as u16 - 1);
    nl.devices.push(fet("XS", DeviceKind::Nmos, clk, sw, 3, 3, 1_000, 150));
    let p = annotate(&nl, &AnnotationConfig::default());
    let b = p.placement.cost.iter().find(|b| is_iso(b)).expect("isolation pull");
    assert_eq!(b.count(), 4, "XS against each of the four matched devices");
    assert!(!p.placement.budget.iter().any(is_iso), "uncalibrated: a pull, not a budget");
    assert!(p.missing.iter().any(|m| m.0 == "Isolation"), "and reported unknown");

    let mut cfg = AnnotationConfig::default();
    cfg.process.epi_nm = Some(3_000);
    let p = annotate(&nl, &cfg);
    assert!(p.placement.budget.iter().any(is_iso));
    assert!(!p.missing.iter().any(|m| m.0 == "Isolation"));
}

#[test]
fn proximity_is_a_priced_budget_not_just_a_pull() {
    // MAT-07: the tail's distance to the input pair is an allowance the search
    // must pay for exceeding, not a report nobody enforces.
    let p = annotate(&ota(), &AnnotationConfig::default());
    let prox = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| {
        a.iter().filter(|b| b.kind().ends_with("Proximity")).map(|b| b.count()).sum::<usize>()
    };
    assert!(prox(&p.placement.budget) > 0);
    assert_eq!(prox(&p.placement.budget), prox(&p.placement.cost), "every budget keeps its pull");
    assert!(!p.placement.hard.iter().any(|b| b.kind().ends_with("Proximity")));
}

#[test]
fn capacitor_plate_nets_get_no_invented_budget() {
    // An inverter drives a DAC bit plate; the top plate feeds a comparator
    // gate. A gate-load budget on either would be invented: the limits are
    // array specs (ARR-03/05), so both read unknown and say so.
    let cap = |name: &str, p: u16, n: u16| Device {
        name: name.into(),
        kind: DeviceKind::Capacitor, model: String::new(),
        terminals: vec![("P".into(), NetId(p)), ("N".into(), NetId(n))],
        params: vec![],
    };
    // nets: 0 top, 1 b0, 2 d0, 3 VDD, 4 VSS, 5 cmp
    let nl = Netlist {
        devices: vec![
            cap("XC1", 0, 1),
            fet("XMP", DeviceKind::Pmos, 2, 1, 3, 3, 1_000, 150),
            fet("XMN", DeviceKind::Nmos, 2, 1, 4, 4, 500, 150),
            fet("XMC", DeviceKind::Nmos, 0, 5, 4, 4, 1_000, 150),
        ],
        nets: nets(&["top", "b0", "d0", "VDD", "VSS", "cmp"]),
    };
    let mut cfg = AnnotationConfig::default();
    cfg.process.gate_af_per_um2 = Some(8_325.0);
    let p = annotate(&nl, &cfg);
    assert_eq!(p.net_classes[0].c_budget_af, None, "top plate");
    assert_eq!(p.net_classes[1].c_budget_af, None, "bit plate");
    assert!(p.net_classes[2].c_budget_af.is_some(), "the inverter input keeps its gate-load budget");
    assert!(p.missing.iter().any(|m| m.0 == "ParasiticBudget" && m.1.contains("ARR-05")));
}

#[test]
#[should_panic(expected = "annotator: 65536 devices/nets exceed the u16 id space (65535)")]
fn more_nets_than_u16_ids_is_refused_not_wrapped() {
    // `NetId(n as u16)` would alias net 65536 onto net 0 (AA-35).
    let nl = Netlist { devices: Vec::new(), nets: vec![Net { name: "n".into() }; 65_536] };
    let _ = annotate(&nl, &AnnotationConfig::default());
}
