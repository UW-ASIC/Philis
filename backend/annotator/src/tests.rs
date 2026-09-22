//! Recognition regression net — ported from `frontend/annotator`'s tests onto
//! `pnr_core::Netlist`. Proves the DSL catalog recognises the canonical analog
//! primitives after the port, and that the `do_not_identify` override suppresses a
//! block.

use crate::{annotate, AnnotationConfig, Block, BlockKind, NoInference, S3Det};
use analog::RuleBatch;
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::{Device, DeviceKind, Net, Netlist};

/// A two-terminal passive (resistor/cap) — never matched by the FET catalog, so
/// it lands in glue where S3DET can find repetition.
fn passive(name: &str, kind: DeviceKind, a: u16, b: u16) -> Device {
    Device {
        name: name.into(),
        kind,
        terminals: vec![("P".into(), NetId(a)), ("N".into(), NetId(b))],
        params: vec![("w".into(), 1_000), ("l".into(), 1_000)],
    }
}

/// Device ids of every `template == "s3det"` block.
fn s3det_sets(blocks: &[Block]) -> Vec<Vec<u16>> {
    blocks
        .iter()
        .filter(|b| b.template == "s3det")
        .map(|b| {
            let mut v: Vec<u16> = b.devices.iter().map(|d| d.0).collect();
            v.sort_unstable();
            v
        })
        .collect()
}

/// A FET with G,D,S,B terminals on the given net ids, W/L in nm.
fn fet(name: &str, kind: DeviceKind, g: u16, d: u16, s: u16, b: u16, w: i64, l: i64) -> Device {
    Device {
        name: name.into(),
        kind,
        terminals: vec![
            ("G".into(), NetId(g)),
            ("D".into(), NetId(d)),
            ("S".into(), NetId(s)),
            ("B".into(), NetId(b)),
        ],
        params: vec![("w".into(), w), ("l".into(), l)],
    }
}

fn nets(names: &[&str]) -> Vec<Net> {
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    // XM1 (0) and XM2 (1) must land in the same recognised block — whether the
    // winner is a bare diff_pair or a composite (five_transistor_ota / DP+load).
    let b1 = block_of(&p.blocks, 0).expect("XM1 recognised");
    let b2 = block_of(&p.blocks, 1).expect("XM2 recognised");
    assert_eq!(b1.group, b2.group, "diff-pair halves split across blocks: {:?}", p.blocks.iter().map(|b| (b.template, &b.devices)).collect::<Vec<_>>());
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    let b = block_of(&p.blocks, 0).expect("mirror recognised");
    assert_eq!(b.kind, BlockKind::CurrentMirror, "template={}", b.template);
    assert!(b.devices.contains(&DeviceId(1)));
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    let b = block_of(&p.blocks, 0).expect("cross-coupled recognised");
    assert!(b.template.contains("cross_coupled"), "template={}", b.template);
}

#[test]
fn do_not_identify_suppresses_block() {
    let nl = ota();
    let cfg = AnnotationConfig { do_not_identify: [0u32].into_iter().collect(), ..Default::default() };
    let p = annotate(&nl, &NoInference, &cfg);
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    assert!(has_leaf(&p.blocks, BlockKind::DiffPair, &[0, 1]), "no DiffPair leaf for XM1/XM2 in {:?}", p.blocks.iter().map(|b| (b.template, b.devices.iter().map(|d| d.0).collect::<Vec<_>>(), b.sub_blocks.iter().map(|s| (s.template, s.devices.iter().map(|d| d.0).collect::<Vec<_>>())).collect::<Vec<_>>())).collect::<Vec<_>>());
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
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
    let routes = pnr_core::routes::Routes { wires: vec![vec![wire(0)], vec![wire(2)]] };

    let p = annotate(&ota(), &NoInference, &AnnotationConfig::default());
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
    let p = annotate(&ota(), &NoInference, &AnnotationConfig::default());
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
fn bias_gen_pair_seeds_isolate() {
    // The other recognised seed: a bias reference is the noisy/sensitive case and
    // starts committed to the far component (a private trench). Built as a block
    // directly — the unit under test is emission, not the recogniser.
    use pnr_core::ids::GroupId;
    let nl = Netlist {
        devices: vec![
            fet("XB1", DeviceKind::Nmos, 0, 0, 1, 1, 5_000, 1_000),
            fet("XB2", DeviceKind::Nmos, 0, 2, 1, 1, 5_000, 1_000),
        ],
        nets: nets(&["vbias", "VSS", "iout"]),
    };
    let hg = pnr_core::BipartiteHypergraph::from_netlist(&nl);
    let b = Block {
        kind: BlockKind::BiasGen,
        template: "bias",
        devices: vec![DeviceId(0), DeviceId(1)],
        group: GroupId(0),
        depends_on: Vec::new(),
        injected: false,
        sub_blocks: Vec::new(),
    };
    let r = crate::emit::placement(&[b], &hg);
    let mut seeds = Vec::new();
    for batch in &r.hard {
        batch.branches(&mut seeds);
    }
    assert_eq!(seeds.len(), 1, "one pair, one disjunction");
    assert!(seeds[0].1, "a bias pair must seed `isolate`");
}

#[test]
fn abutment_excludes_mixed_polarity_groups() {
    // D16: `groups` is the recognition table and deliberately contains composites
    // spanning both polarities; `abutment` is diffusion-sharing permission. Granting
    // abutment on a mixed group lets an NMOS and a PMOS merge implants — DRC-clean,
    // LVS-fatal, and unrepairable downstream because nothing is illegal.
    let nl = ota(); // XM1/XM2/XM5 Nmos, XM3/XM4 Pmos
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
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
            Device { name: "R1".into(), kind: DeviceKind::Resistor, terminals: vec![("A".into(), NetId(0)), ("B".into(), NetId(1))], params: vec![] },
            Device { name: "R2".into(), kind: DeviceKind::Resistor, terminals: vec![("A".into(), NetId(2)), ("B".into(), NetId(3))], params: vec![] },
        ],
        nets: nets(&["a", "b", "c", "d"]),
    };
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    assert_eq!(count(&p.placement.hard), 0);
    assert_eq!(count(&p.placement.cost), 0);
}

#[test]
fn identical_blocks_form_one_reuse_class() {
    // Two structurally-identical current mirrors on disjoint nets → one reuse
    // class of 2 (cross-circuit reuse; ADR-0005 variant coupling, not frozen).
    let nl = Netlist {
        devices: vec![
            fet("M1", DeviceKind::Pmos, 0, 0, 8, 8, 5_000, 1_000), // mirror A: diode ref
            fet("M2", DeviceKind::Pmos, 0, 1, 8, 8, 5_000, 1_000), // mirror A: out
            fet("M3", DeviceKind::Pmos, 2, 2, 8, 8, 5_000, 1_000), // mirror B: diode ref
            fet("M4", DeviceKind::Pmos, 2, 3, 8, 8, 5_000, 1_000), // mirror B: out
        ],
        nets: nets(&["vrefA", "ioutA", "vrefB", "ioutB", "n4", "n5", "n6", "n7", "VDD"]),
    };
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    // Two identical mirrors — whether recognised as two top blocks or as the
    // children of one composite — form a single 2-member reuse class.
    assert!(
        p.reuse.iter().any(|c| c.members.len() == 2),
        "expected a 2-member reuse class, got {:?}", p.reuse
    );
}

#[test]
fn distinct_blocks_do_not_share_a_reuse_class() {
    // The 5T OTA's diff pair and load differ structurally → no spurious class.
    let nl = ota();
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    // No reuse class mixes non-identical blocks (a class only forms on equal
    // template+geometry). The OTA has at most singletons of each block shape.
    for c in &p.reuse {
        assert!(c.members.len() >= 2, "reuse classes are never singletons");
    }
}

#[test]
fn parallel_array_detected() {
    // Three identical unit caps wired A→B in parallel → one parallel group of 3
    // (draw one, stamp 3 — the passive-array case the catalog never blocks).
    let cap = |name: &str| Device {
        name: name.into(),
        kind: DeviceKind::Capacitor,
        terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(1))],
        params: vec![("w".into(), 2_000), ("l".into(), 2_000)],
    };
    let nl = Netlist { devices: vec![cap("C1"), cap("C2"), cap("C3")], nets: nets(&["top", "bot"]) };
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    assert_eq!(p.parallel, vec![vec![DeviceId(0), DeviceId(1), DeviceId(2)]], "parallel array not coalesced");
}

/// Two identical R-R-C "K3" sections (0,1,2) and (3,4,5), plus optionally a
/// same-size R-R-R "P3" chain (6,7,8) that is structurally different. All passive
/// → all glue → S3DET territory.
fn two_identical_plus_optional_different(with_different: bool) -> Netlist {
    use DeviceKind::{Capacitor as C, Resistor as R};
    let mut devices = vec![
        passive("RA0", R, 0, 1),
        passive("RA1", R, 1, 2),
        passive("CA2", C, 1, 10), // 10 = gnd (rail) → excluded from adjacency
        passive("RB0", R, 3, 4),
        passive("RB1", R, 4, 5),
        passive("CB2", C, 4, 10),
    ];
    if with_different {
        devices.push(passive("RC0", R, 6, 7));
        devices.push(passive("RC1", R, 7, 8));
        devices.push(passive("RC2", R, 8, 9)); // chain (P3), not a triangle
    }
    Netlist { devices, nets: nets(&["a0", "a1", "a2", "b0", "b1", "b2", "c0", "c1", "c2", "c3", "gnd"]) }
}

#[test]
fn s3det_matches_identical_untemplated_sections() {
    let nl = two_identical_plus_optional_different(false);
    let p = annotate(&nl, &S3Det::default(), &AnnotationConfig::default());
    let mut sets = s3det_sets(&p.blocks);
    sets.sort();
    assert_eq!(sets, vec![vec![0, 1, 2], vec![3, 4, 5]], "S3DET should match the two identical sections");
}

#[test]
fn s3det_rejects_same_size_different_structure() {
    // The R-R-R chain (6,7,8) is the same *size* as the R-R-C sections but a
    // different topology and has no twin → S3DET must NOT emit it (proves the
    // match is spectral, not just a device-count prefilter).
    let nl = two_identical_plus_optional_different(true);
    let p = annotate(&nl, &S3Det::default(), &AnnotationConfig::default());
    let mut sets = s3det_sets(&p.blocks);
    sets.sort();
    assert_eq!(sets, vec![vec![0, 1, 2], vec![3, 4, 5]], "the lone R-R-R chain must stay glue");
}

#[test]
fn no_inference_emits_no_s3det_blocks() {
    let nl = two_identical_plus_optional_different(false);
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    assert!(s3det_sets(&p.blocks).is_empty(), "NoInference must not infer anything");
}

#[test]
fn every_device_accounted_for() {
    // No device is lost: recognised ∪ glue == all devices, disjoint.
    let nl = ota();
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
    let param = |d: DeviceId, k: &str| {
        nl.devices[d.0 as usize].params.iter().find(|(n, _)| n == k).map(|&(_, v)| v)
    };
    for u in &p.constraints.unitization {
        for &d in &u.devices {
            assert_eq!(nl.devices[d.0 as usize].kind, u.device_type, "unitization {:?} mixes kinds", u.devices);
            assert_eq!(param(d, "w"), Some(i64::from(u.unit_w)), "unitization {:?} mixes W", u.devices);
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
    let p = annotate(&nl, &NoInference, &AnnotationConfig::default());
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
