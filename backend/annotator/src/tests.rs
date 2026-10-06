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

/// Nets named in order: `names[i]` is `NetId(i)`.
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
pub(crate) fn ota() -> Netlist {
    Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Nmos, 1, 0, 2, 3, 10_000, 1_000),
            fet("XM2", DeviceKind::Nmos, 5, 4, 2, 3, 10_000, 1_000),
            fet("XM3", DeviceKind::Pmos, 6, 0, 7, 7, 20_000, 1_000),
            fet("XM4", DeviceKind::Pmos, 6, 4, 7, 7, 20_000, 1_000),
            fet("XM5", DeviceKind::Nmos, 8, 2, 3, 3, 40_000, 2_000),
        ],
        nets: nets(&["vout1", "vinp", "vtail", "VSS", "vout2", "vinm", "vbias", "VDD", "vbn"]),
        ..Default::default()
    }
}

/// The three-stage op-amp (`examples/three_stage_opamp`, corpus `three_stage`).
pub(crate) fn three_stage() -> Netlist {
    // Nets: 0=n1 1=vin_p 2=tail 3=vss 4=n2 5=vin_n 6=vbias 7=vdd 8=n3 9=vout.
    let (n, p) = (DeviceKind::Nmos, DeviceKind::Pmos);
    Netlist {
        devices: vec![
            fet("M1", n, 1, 0, 2, 3, 4_000, 500),
            fet("M2", n, 5, 4, 2, 3, 4_000, 500),
            fet("M3", n, 6, 2, 3, 3, 8_000, 500),
            fet("M4", p, 0, 0, 7, 7, 6_000, 500),
            fet("M5", p, 0, 4, 7, 7, 6_000, 500),
            fet("M6", p, 4, 8, 7, 7, 12_000, 500),
            fet("M7", n, 6, 8, 3, 3, 6_000, 500),
            fet("M8", p, 8, 9, 7, 7, 40_000, 500),
            fet("M9", n, 6, 9, 3, 3, 20_000, 500),
        ],
        nets: nets(&["n1", "vin_p", "tail", "vss", "n2", "vin_n", "vbias", "vdd", "n3", "vout"]),
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
    // A recognised diff pair emits Symmetry (hard) and MatchedSet (budget +
    // cost: one pair's gradient, thermal and LOD ledger, PLAN §4d) — the
    // BlockKind mapping, driven by recognition. MatchedSet is never Hard: its
    // allowance follows device AREA (Pelgrom `σ²_u = A²/(W·L)`), which no
    // placement move can change, so gating SA on it is inert. See
    // `backend/TODO.md` §2.
    let nl = ota();
    let p = annotate(&nl, &AnnotationConfig::default());
    assert!(count(&p.placement.hard) >= 1, "expected >=1 hard placement rule (sym), got {}", count(&p.placement.hard));
    assert!(count(&p.placement.budget) >= 1, "expected >=1 budget placement rule (thermal), got {}", count(&p.placement.budget));
    assert!(count(&p.placement.cost) >= 2, "expected >=2 cost placement rules (matched set/prox), got {}", count(&p.placement.cost));

    // The partition itself is the contract: nothing in `hard` may be a rule the
    // placer cannot act on.
    let hard_kinds: Vec<&str> = p.placement.hard.iter().map(|b| b.kind()).collect();
    assert!(
        !hard_kinds.iter().any(|k| k.contains("MatchedSet")),
        "MatchedSet must not gate placement moves: {hard_kinds:?}"
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
    for kind in ["Differential", "CrosstalkExclusion", "ParasiticBudget", "CouplingBudget"] {
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

    // And the placement-tier budget, from the other other side: `MatchedSet`
    // carries an allowance on derived fields (gradient, live thermal, LOD), is
    // tradeable while converging, and PLAN §4d calls it a budget outright. Its
    // cost copy is a gradient, not a classification; the classified copy must
    // be priced, never gated.
    let set = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| {
        a.iter().any(|b| b.kind().ends_with("MatchedSet"))
    };
    assert!(set(&p.placement.budget), "MatchedSet must be a priced budget");
    assert!(!set(&p.placement.hard), "MatchedSet must never gate legality as Hard");
}

#[test]
fn matched_pairs_get_orientation_rules() {
    // MAT-05: channel axes are a hard placement rule (a quarter-turned partner is
    // illegal); Φ is budget only (a discrete flip, no gradient for a cost copy).
    let p = annotate(&ota(), &AnnotationConfig::default());
    let has = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>| a.iter().any(|b| b.kind() == "Orientation");
    assert!(has(&p.placement.hard), "Orientation axis must be Hard");
    assert!(has(&p.placement.budget), "Orientation Φ must be a budget");
    assert!(!has(&p.placement.cost), "Orientation has no cost copy");
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
fn glue_only_netlist_emits_no_placement() {
    // Two unrelated resistors → nothing recognised → no placement constraints.
    let nl = Netlist {
        devices: vec![
            Device { name: "R1".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(0)), ("B".into(), NetId(1))], params: vec![] },
            Device { name: "R2".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(2)), ("B".into(), NetId(3))], params: vec![] },
        ],
        nets: nets(&["a", "b", "c", "d"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(count(&p.placement.hard), 0);
    assert_eq!(count(&p.placement.cost), 0);
}








#[test]
fn missing_is_relevant() {
    // Two resistors: no matched leaf and no gate, so no MatchedSet or Antenna entry.
    let nl = Netlist {
        devices: vec![
            Device { name: "R1".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(0)), ("B".into(), NetId(1))], params: vec![] },
            Device { name: "R2".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("A".into(), NetId(2)), ("B".into(), NetId(3))], params: vec![] },
        ],
        nets: nets(&["a", "b", "c", "d"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    // ... and no gate area, so no net budget either.
    assert!(!p.missing.iter().any(|m| ["MatchedSet", "Antenna", "ParasiticBudget", "CouplingBudget"].contains(&m.0)), "{:?}", p.missing);
    let p = annotate(&ota(), &AnnotationConfig::default());
    assert!(p.missing.contains(&("MatchedSet", "deck svt_uv_per_um — distance term unknown")), "{:?}", p.missing);
    assert!(p.missing.contains(&("ParasiticBudget", "deck gate_cap_af_um2")), "{:?}", p.missing);
}

#[test]
fn every_batch_is_tagged() {
    let mut clocked = ota();
    clocked.nets.push(Net { name: "clk".into() });
    clocked.nets.push(Net { name: "sw".into() });
    let (clk, sw) = (clocked.nets.len() as u16 - 2, clocked.nets.len() as u16 - 1);
    clocked.devices.push(fet("XS", DeviceKind::Nmos, clk, sw, 3, 3, 1_000, 150));
    for nl in [ota(), clocked] {
        let p = annotate(&nl, &AnnotationConfig::default());
        let (pl, ro) = (&p.placement, &p.routing);
        let mut ids: Vec<u32> = (pl.hard.iter().chain(&pl.budget).chain(&pl.cost))
            .map(|b| b.meta().expect("placement batch tagged").id.0)
            .chain(ro.hard.iter().chain(&ro.budget).chain(&ro.cost).map(|b| b.meta().expect("routing batch tagged").id.0))
            .collect();
        let total = ids.len() as u32;
        assert!(total > 0);
        assert_eq!(ids, (0..total).collect::<Vec<_>>(), "dense, in arm order");
        ids.dedup();
        assert_eq!(ids.len() as u32, total);

        // Origin: the XM1/XM2 pair came from its pattern; Isolation and routing from net classes.
        let net_class = Some(analog::intent::Origin::NetClass);
        let origin = |b: &dyn analog::RuleBatch<pnr_core::Layout>| b.meta().map(|m| m.origin);
        let pair = (pl.cost.iter()).find(|b| {
            let mut t = Vec::new();
            b.touched(&mut t);
            t.sort_unstable();
            b.kind() == "MatchedSet" && t == [0, 1]
        });
        assert_eq!(origin(pair.expect("XM1/XM2 MatchedSet").as_ref()), Some(analog::intent::Origin::Pattern { template: "five_transistor_ota" }));
        let iso = (pl.hard.iter().chain(&pl.budget).chain(&pl.cost)).filter(|b| b.kind().ends_with("::Isolation"));
        assert!(iso.map(|b| origin(b.as_ref())).all(|o| o == net_class));
        assert!(ro.hard.iter().chain(&ro.budget).chain(&ro.cost).all(|b| b.meta().map(|m| m.origin) == net_class));
    }
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
    // REL-07: only an aggressor gets a ring, so M1 is clocked; its bulk
    // (net 3) is not net 0.
    let nl = Netlist {
        devices: vec![fet("M1", DeviceKind::Nmos, 1, 2, 3, 3, 1_000, 150), fet("M2", DeviceKind::Pmos, 1, 2, 4, 4, 1_000, 150)],
        nets: nets(&["out", "clk", "x", "vss", "vdd"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig { clock_nets: vec!["clk".into()], ..AnnotationConfig::default() });
    assert!(!p.constraints.guard_rings.is_empty(), "clocked FETs get rings");
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
    assert!(p.constraints.guard_rings.is_empty(), "no aggressor: no ring (T8)");
}

/// AA-23: only declared selfs go on the axis. `telescopic_ota_full` declares
/// the tail (slot 6) and no role for the load bias (slot 7, a diode PMOS on the
/// loads' source). Nets: 0=inp 1=x1 2=tail 3=VSS 4=inn 5=x2 6=vbn2 7=o1 8=o2
/// 9=vbp 10=VDD 11=vbn.
#[test]
fn telescopic_slot7_is_not_self_symmetric() {
    let (n, p) = (DeviceKind::Nmos, DeviceKind::Pmos);
    let nl = Netlist {
        devices: vec![
            fet("M0", n, 0, 1, 2, 3, 10_000, 1_000),
            fet("M1", n, 4, 5, 2, 3, 10_000, 1_000),
            fet("M2", n, 6, 7, 1, 3, 10_000, 1_000),
            fet("M3", n, 6, 8, 5, 3, 10_000, 1_000),
            fet("M4", p, 9, 7, 10, 10, 20_000, 1_000),
            fet("M5", p, 9, 8, 10, 10, 20_000, 1_000),
            fet("M6", n, 11, 2, 3, 3, 40_000, 1_000),
            fet("M7", p, 9, 9, 10, 10, 5_000, 1_000),
        ],
        nets: nets(&["inp", "x1", "tail", "VSS", "inn", "x2", "vbn2", "o1", "o2", "vbp", "VDD", "vbn"]),
        ..Default::default()
    };
    let pr = annotate(&nl, &AnnotationConfig::default());
    assert_eq!((pr.blocks[0].template, pr.blocks[0].devices.len()), ("telescopic_ota_full", 8));
    let mut m = Vec::new();
    pr.placement.hard.iter().for_each(|b| b.mirror_pairs(&mut m));
    let selfs: Vec<u32> = m.iter().filter(|p| p.0 == p.1).map(|p| p.0).collect();
    assert_eq!(selfs, [6], "the tail only, not slot 7: {m:?}");
}

#[test]
fn a_shared_gate_chain_is_a_series_stack_not_a_cascode() {
    // chain4: four same-size nfets in series on one gate net.
    let nl = Netlist {
        devices: (0..4u16)
            .map(|i| fet(&format!("M{i}"), DeviceKind::Nmos, 0, i + 1, i + 2, 6, 2_000, 500))
            .collect(),
        nets: nets(&["g", "a", "b", "c", "d", "e", "VSS"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.blocks[0].devices.len(), 4, "one series_stack_4 group");
    assert!(p.blocks[0].sub_blocks.iter().all(|b| b.kind == BlockKind::Stack), "stack pairs, not a cascode");
    // The shared gate net (gates only) is a bias line (EXT-18: was `Sensitive`).
    assert_eq!(p.net_classes[0].class, analog::metadata::NetClass::Bias);
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
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.blocks[0].kind, BlockKind::Stack);
    let kinds: Vec<&str> = p.placement.cost.iter().map(|b| b.kind()).collect();
    assert!(kinds.iter().any(|k| k.ends_with("Proximity")), "{kinds:?}");
    assert!(!kinds.iter().any(|k| k.ends_with("MatchedSet")), "{kinds:?}");
    assert!(p.placement.hard.is_empty());
    // `vcas` (gates M2 only, no DC path) is a bias line (EXT-18: was `Sensitive`).
    assert_eq!(p.net_classes[3].class, analog::metadata::NetClass::Bias);
}

/// EXT-07: a `Stack` is adjacent/symmetric, not a gate reference, so it is no
/// longer an isolation victim. A clocked switch elsewhere used to be an
/// aggressor that forced isolation onto the stack's two devices. Since EXT-23
/// a Bias-gated FET is a victim on its own, so `R1`/`R2` give both gates a DC
/// path (Signal, not Bias): only the stack membership is under test.
#[test]
fn a_cascode_stack_is_not_an_isolation_victim() {
    let r = |name: &str, p: u16| Device {
        name: name.into(),
        kind: DeviceKind::Resistor,
        model: String::new(),
        terminals: vec![("P".into(), pnr_core::NetId(p)), ("N".into(), pnr_core::NetId(2))],
        params: vec![],
    };
    let nl = Netlist {
        devices: vec![
            fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 4_000, 500),
            fet("M2", DeviceKind::Nmos, 3, 4, 1, 2, 8_000, 500),
            fet("XS", DeviceKind::Nmos, 5, 6, 2, 2, 1_000, 150),
            r("R1", 0),
            r("R2", 3),
        ],
        nets: nets(&["vin", "x", "VSS", "vcas", "out", "clk", "sw"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    let mut touched = Vec::new();
    for b in &p.placement.cost {
        if b.kind().ends_with("Isolation") {
            b.touched(&mut touched);
        }
    }
    assert!(
        !touched.contains(&0) && !touched.contains(&1),
        "stack devices are no longer isolation victims: {touched:?}"
    );
}

/// EXT-07 variant: once `vcas` also touches a channel (not gates only), its
/// class is ordinary `Signal`, not `Sensitive`: with `Stack` no longer a gate
/// reference in `is_sensitive`, a `vcas` that touches a channel is `Signal`.
#[test]
fn a_cascode_gate_net_with_a_channel_use_is_signal() {
    let nl = Netlist {
        devices: vec![
            fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 4_000, 500),
            fet("M2", DeviceKind::Nmos, 3, 4, 1, 2, 8_000, 500),
            fet("MD", DeviceKind::Nmos, 0, 3, 2, 2, 4_000, 500),
        ],
        nets: nets(&["vin", "x", "VSS", "vcas", "out"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(p.net_classes[3].class, analog::metadata::NetClass::Signal);
}

#[test]
fn antenna_gate_area_lands_on_the_gate_net_not_the_drain() {
    // G ≠ D on purpose: a diode-connected device would hide an index slip.
    let nl = Netlist {
        devices: vec![fet("XM1", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000)],
        nets: nets(&["g", "d", "VSS"]),
        ..Default::default()
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
    assert!(pairs.iter().all(|&(_, r, _)| r == vss), "shielded by ground: {pairs:?}");
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
    let (victim, vss, _) = pairs[0];
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

/// EXT-20 (AA-23): a mirror with no compound is matched, not mirrored: one
/// `MatchedSet` on both devices and its Axis Orientation, no Symmetry.
#[test]
fn a_lone_mirror_is_matched_not_mirrored() {
    let nl = Netlist {
        devices: vec![
            fet("XM1", DeviceKind::Pmos, 0, 0, 2, 2, 5_000, 1_000),
            fet("XM2", DeviceKind::Pmos, 0, 1, 2, 2, 5_000, 1_000),
        ],
        nets: nets(&["vref", "iout", "VDD"]),
        ..Default::default()
    };
    let p = annotate(&nl, &AnnotationConfig::default());
    let count = |a: &Vec<Box<dyn RuleBatch<pnr_core::Layout>>>, k: &str| a.iter().filter(|b| b.kind() == k).map(|b| b.count()).sum::<usize>();
    assert_eq!(count(&p.placement.hard, "Symmetry") + count(&p.placement.cost, "Symmetry"), 0);
    let sets: Vec<Vec<u32>> = (p.placement.budget.iter().filter(|b| b.kind() == "MatchedSet"))
        .map(|b| {
            let mut t = Vec::new();
            b.touched(&mut t);
            t.sort_unstable();
            t
        })
        .collect();
    assert_eq!(sets, [vec![0, 1]]);
    assert_eq!(p.placement.hard.iter().filter(|b| b.kind() == "Orientation").count(), 1);
}

#[test]
fn matched_sets_are_budgeted_and_missing_deck_terms_are_listed() {
    let is_set = |b: &Box<dyn RuleBatch<pnr_core::Layout>>| b.kind() == "MatchedSet";
    let bare = annotate(&ota(), &AnnotationConfig::default());
    assert!(bare.placement.budget.iter().any(is_set), "coincidence needs no deck data");
    assert!(bare.missing.iter().any(|m| m.0 == "MatchedSet"), "the deck terms are listed unknown");

    let mut cfg = AnnotationConfig::default();
    cfg.process.avt_mv_um = [Some(5.0), Some(6.0)];
    cfg.process.svt_uv_per_um = Some(4.0);
    let full = annotate(&ota(), &cfg);
    assert!(full.placement.budget.iter().any(is_set));
    assert!(!full.missing.iter().any(|m| m.0 == "MatchedSet"));
}

/// MAT-16: with the deck's S(L) fit the XM1/XM2 pair's gradient term reads
/// S at L = 0.48 µm (0.580 µV/µm), 0.356 of the worst-case 1.63.
#[test]
fn svt_fit_sets_s_of_l() {
    let mut nl = ota();
    for d in &mut nl.devices[..2] {
        d.params.iter_mut().filter(|(k, _)| k == "l").for_each(|(_, v)| *v = 480);
    }
    // Two point cells 1 mm apart, no units.
    let l = pnr_core::Layout {
        x: vec![0, 1_000_000],
        y: vec![0; 2],
        hw: vec![0; 2],
        hh: vec![0; 2],
        axis: vec![0; 8],
        groups: vec![],
        orient: vec![pnr_core::Orient::default(); 2],
        variant: vec![0; 2],
        branch: Vec::new(),
        power_uw: vec![0; 2],
        temp_mc: vec![0; 2],
        units: Default::default(),
    };
    let ratio = |fit| {
        let mut cfg = AnnotationConfig::default();
        cfg.process.avt_mv_um = [Some(5.0), Some(6.0)];
        cfg.process.svt_fit = fit;
        cfg.process.svt_uv_per_um = Some(1.63);
        let p = annotate(&nl, &cfg);
        let set = (p.placement.budget.iter())
            .find(|b| {
                let mut t = Vec::new();
                b.touched(&mut t);
                t.sort_unstable();
                b.kind() == "MatchedSet" && t == [0, 1]
            })
            .expect("XM1/XM2 MatchedSet");
        let mut rows = Vec::new();
        set.ledger_rows(&l, &mut rows);
        rows[0].sigma_layout / 1.63
    };
    let r = ratio(Some((0.1835, 0.03533)));
    assert!((r - 0.356).abs() < 0.005, "{r}");
    let r = ratio(None);
    assert!((r - 1.0).abs() < 1e-4, "{r}");
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
    assert_eq!(b.count(), 5, "XS against the four matched devices and the bias-gated tail (EXT-23)");
    assert!(!p.placement.budget.iter().any(is_iso), "uncalibrated: a pull, not a budget");
    assert!(p.missing.iter().any(|m| m.0 == "Isolation"), "and reported unknown");

    let mut cfg = AnnotationConfig::default();
    cfg.process.substrate = pnr_core::SubstrateKind::EpiOnLowRes;
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
        ..Default::default()
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
    let nl = Netlist { devices: Vec::new(), nets: vec![Net { name: "n".into() }; 65_536], ..Default::default() };
    let _ = annotate(&nl, &AnnotationConfig::default());
}

// ── REL-07: guard rings by role ──────────────────────────────────────────

mod rings {
    use super::*;
    use crate::rings::{plan, Carrier, RingInputs};
    use analog::cell::{GuardRingType, RingRole};
    use pnr_core::SubstrateKind;

    const SUB30: (&str, &str) = ("GuardRing", "victim rings: no quiet ring return (SUB-30)");

    /// M1 (NMOS, gate `clk`), M2/M3 (NMOS victims), M4 (PMOS). Nets: 0=clk
    /// 1=x 2=vss 3=vssq 4=vdd.
    fn nl() -> Netlist {
        Netlist {
            devices: vec![
                fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 150),
                fet("M2", DeviceKind::Nmos, 1, 1, 2, 2, 1_000, 150),
                fet("M3", DeviceKind::Nmos, 1, 1, 2, 2, 1_000, 150),
                fet("M4", DeviceKind::Pmos, 1, 1, 4, 4, 1_000, 150),
            ],
            nets: nets(&["clk", "x", "vss", "vssq", "vdd"]),
            ..Default::default()
        }
    }

    fn inputs<'a>(nl: &'a Netlist, aggressor: &'a [bool], victim: &'a [bool], injector: &'a [Option<Carrier>]) -> RingInputs<'a> {
        RingInputs {
            netlist: nl,
            aggressor,
            victim,
            injector,
            substrate: SubstrateKind::Bulk,
            quiet_ring_net: None,
            highest_supply: Some(NetId(4)),
            ground: Some(NetId(2)),
            min_ring_width_nm: 420,
            ecgr_min_width_nm: None,
            ecgr_drawable: false,
            hcgr_drawable: false,
            tubs: &[],
            tub_drawable: false,
        }
    }

    /// GAP-14: tub members get the tub's ring on its tie; the rest keep their rows.
    #[test]
    fn tub_members_get_a_tub_ring() {
        let nl = nl();
        let tubs = [(vec![DeviceId(1), DeviceId(2)], NetId(4))];
        let i = RingInputs { tubs: &tubs, tub_drawable: true, ..inputs(&nl, &[true, false, false, false], &[false; 4], &[None; 4]) };
        let (rings, missing) = plan(&i);
        let tub: Vec<_> = rings.iter().filter(|r| r.ring_type == GuardRingType::Tub { id: 0 }).map(|r| (r.device, r.connection_net, r.shareable)).collect();
        assert_eq!(tub, [(DeviceId(1), NetId(4), true), (DeviceId(2), NetId(4), true)]);
        let other: Vec<_> = rings.iter().filter(|r| !matches!(r.ring_type, GuardRingType::Tub { .. })).map(|r| (r.device, r.ring_type)).collect();
        assert_eq!(other, [(DeviceId(0), GuardRingType::Tap { in_well: false })]);
        assert!(!missing.iter().any(|m| m.0 == "IsolatedTub"));
    }

    #[test]
    fn no_dnwell_falls_back_and_says_so() {
        let nl = nl();
        let tubs = [(vec![DeviceId(1), DeviceId(2)], NetId(4))];
        let (rings, missing) = plan(&RingInputs { tubs: &tubs, ..inputs(&nl, &[true, false, false, false], &[false; 4], &[None; 4]) });
        assert!(rings.iter().all(|r| !matches!(r.ring_type, GuardRingType::Tub { .. })));
        assert!(missing.contains(&("IsolatedTub", "deck has no deep n-well: tub drawn as an ordinary ring")), "{missing:?}");
    }

    #[test]
    fn an_ota_without_clocks_or_ports_gets_no_rings() {
        assert!(annotate(&ota(), &AnnotationConfig::default()).constraints.guard_rings.is_empty());
    }

    #[test]
    fn clocked_devices_get_aggressor_tap_rings() {
        let nl = nl();
        let (rings, missing) = plan(&inputs(&nl, &[true, false, false, false], &[false, true, false, false], &[None; 4]));
        assert_eq!(rings.len(), 1);
        let r = &rings[0];
        assert_eq!((r.device, r.ring_type, r.role, r.connection_net), (DeviceId(0), GuardRingType::Tap { in_well: false }, RingRole::Aggressor, NetId(2)));
        assert!(missing.contains(&SUB30), "{missing:?}");
    }

    #[test]
    fn a_quiet_net_turns_on_victim_rings() {
        let nl = nl();
        let i = RingInputs { quiet_ring_net: Some(NetId(3)), ..inputs(&nl, &[true, false, false, false], &[false, true, true, false], &[None; 4]) };
        let (rings, missing) = plan(&i);
        let v: Vec<_> = rings.iter().filter(|r| r.role == RingRole::Victim).collect();
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|r| r.connection_net == NetId(3) && r.shareable));
        assert!(!missing.contains(&SUB30));
    }

    #[test]
    fn an_electron_injector_gets_a_supply_tied_ecgr() {
        let nl = nl();
        let inj = [Some(Carrier::Electrons), None, None, None];
        let (rings, missing) = plan(&RingInputs { ecgr_drawable: true, ..inputs(&nl, &[false; 4], &[false; 4], &inj) });
        assert_eq!(rings.len(), 1);
        let r = &rings[0];
        assert_eq!((r.ring_type, r.role, r.connection_net, r.min_width_nm, r.shareable), (GuardRingType::Ecgr, RingRole::Injector, NetId(4), 420, false));
        assert!(missing.contains(&("GuardRing", "ECGR width rule not given: collection efficiency unknown")), "{missing:?}");
    }

    #[test]
    fn a_hole_injector_falls_back_to_a_well_tap() {
        let nl = nl();
        let inj = [None, None, None, Some(Carrier::Holes)];
        let (rings, _) = plan(&inputs(&nl, &[false; 4], &[false; 4], &inj));
        assert_eq!(rings.len(), 1);
        let r = &rings[0];
        assert_eq!((r.device, r.ring_type, r.role, r.connection_net), (DeviceId(3), GuardRingType::Tap { in_well: true }, RingRole::Injector, NetId(4)));
    }

    #[test]
    fn epi_substrate_draws_no_aggressor_or_victim_rings() {
        let nl = nl();
        let i = RingInputs {
            substrate: SubstrateKind::EpiOnLowRes,
            quiet_ring_net: Some(NetId(3)),
            ..inputs(&nl, &[true, false, false, false], &[false, true, true, false], &[None; 4])
        };
        assert!(plan(&i).0.is_empty());
    }
}

#[test]
fn intent_axes_per_compound() {
    let p = annotate(&ota(), &AnnotationConfig::default());
    // EXT-14 fills compounds (one here), EXT-15 sets (DP and load; without a unit
    // deck they have no unit); EXT-20: one axis per compound.
    assert!(p.intent.sets.len() == 2 && p.intent.compounds.len() == 1);
    assert_eq!(p.axis_count, 1);
    assert!(p.blocks.len() > 1, "not per block");
}

/// EXT-21: offset f0 0, hi 5 (M 5) touches the DP (S 1) and the load (S 0.2);
/// K 2 gives the DP 5/(2·1) = 2.5 mV (its cap 3·5/√10 = 4.74 does not bind)
/// and the load min(12.5, 3·5/√20 = 3.35).
#[test]
fn ext21_ota_dp_allowance_below_load() {
    use crate::evidence::{Evidence, Sensitivities, SpecSens};
    let mut cfg = AnnotationConfig::default();
    cfg.process.avt_mv_um = [Some(5.0), Some(5.0)];
    let d_vt = vec![(DeviceId(0), 1.0), (DeviceId(1), -1.0), (DeviceId(2), 0.2), (DeviceId(3), -0.2)];
    let s = SpecSens { metric: "offset".into(), f0: 0.0, lo: None, hi: Some(5.0), proc: None, sigma_f: None, d_c: vec![], d_r: vec![], d_vt, d_t: vec![], d_cc: vec![] };
    let ev = Evidence { sens: Some(Sensitivities { specs: vec![s] }), ..Evidence::default() };
    let p = crate::annotate_with(&ota(), &cfg, &ev);
    let allowance = |d: u16| p.intent.sets.iter().find(|s| s.members.iter().any(|m| m.device == DeviceId(d))).and_then(|s| s.allowance);
    let (dp, load) = (allowance(0).expect("DP allowance"), allowance(2).expect("load allowance"));
    assert!((dp - 2.5).abs() < 1e-4, "{dp}");
    assert!(dp < load, "{dp} vs {load}");
    let bare = annotate(&ota(), &cfg);
    assert!(bare.intent.sets.iter().all(|s| s.allowance.is_none()));
}

/// EXT-21 class rules: σ_f = 4 mV puts the load (σ ≈ 1.58 mV, S = 0.2) at weight
/// ≈ 0.006 < `minor_weight` and the DP (σ ≈ 2.24 mV, S = 1) at ≈ 0.31, so D5 makes
/// only the load (Minimal, Spec); the ceiling 2 + β·σ_f leaves a 2 mV margin, so
/// the DP's allowance is 1 mV and its 6 mV target makes it (Moderate, Spec), apart
/// from both the role default (Moderate, Role) and D5. (At the 2.5 mV of
/// `ext21_ota_dp_allowance_below_load` the 15 mV target is itself Minimal.)
#[test]
fn ext21_minor_weight_and_allowance_set_class() {
    use crate::evidence::{Evidence, Sensitivities, SpecSens};
    use analog::intent::{ClassSource, MatchClass};
    let mut cfg = AnnotationConfig::default();
    cfg.process.avt_mv_um = [Some(5.0), Some(5.0)];
    let d_vt = vec![(DeviceId(0), 1.0), (DeviceId(1), -1.0), (DeviceId(2), 0.2), (DeviceId(3), -0.2)];
    let sf = 4.0;
    let s = SpecSens { metric: "offset".into(), f0: 0.0, lo: None, hi: Some(2.0 + cfg.policy.beta_target * sf), proc: None, sigma_f: Some(sf), d_c: vec![], d_r: vec![], d_vt, d_t: vec![], d_cc: vec![] };
    let ev = Evidence { sens: Some(Sensitivities { specs: vec![s] }), ..Evidence::default() };
    let p = crate::annotate_with(&ota(), &cfg, &ev);
    let set = |d: u16| p.intent.sets.iter().find(|s| s.members.iter().any(|m| m.device == DeviceId(d))).expect("set");
    let (dp, load) = (set(0), set(2));
    assert!(load.weight.is_some_and(|w| w < cfg.policy.minor_weight), "{:?}", load.weight);
    assert!(dp.weight.is_some_and(|w| w > cfg.policy.minor_weight), "{:?}", dp.weight);
    assert_eq!((load.class, load.class_source), (MatchClass::Minimal, ClassSource::Spec));
    assert_ne!((dp.class, dp.class_source), (MatchClass::Minimal, ClassSource::Spec));
    assert_eq!(dp.kind, analog::intent::MatchKind::Voltage);
    assert!((dp.allowance.expect("DP allowance") - 1.0).abs() < 1e-4, "{:?}", dp.allowance);
    let mut diags = Vec::new();
    let mut ctx = crate::class::ClassCtx { user: None, spec_6sigma: Some(6.0 * 1.0), role: crate::class::SetRole::InputPair, diags: &mut diags };
    assert_eq!((dp.class, dp.class_source), crate::class::class_of(dp, &mut ctx));
    assert_eq!((dp.class, dp.class_source), (MatchClass::Moderate, ClassSource::Spec));
    // Without σ_f there is no weight, so the load keeps its role class.
    let bare = annotate(&ota(), &cfg);
    let load = bare.intent.sets.iter().find(|s| s.members.iter().any(|m| m.device == DeviceId(2))).expect("set");
    assert_ne!(load.class, MatchClass::Minimal);
}

/// EXT-25 (AA-25): a drain-only net's load is off-netlist, so it is unbudgeted
/// and listed missing until a sidecar `Load` states it; a gate-driving net
/// (vbias) keeps its gate-load budget.
#[test]
fn drain_only_net_without_load_is_unknown() {
    let mut cfg = AnnotationConfig::default();
    cfg.process.gate_af_per_um2 = Some(8325.0);
    cfg.process.wire_af_per_um = Some(50.0);
    let budget = |p: &crate::Problem, n: u16| p.net_classes[n as usize].c_budget_af;
    let missing = |p: &crate::Problem| p.missing.iter().any(|m| m.0 == "ParasiticBudget" && m.1.contains("AA-25"));
    let p = annotate(&ota(), &cfg);
    assert_eq!((budget(&p, 4), budget(&p, 2)), (None, None), "vout2, vtail");
    assert!(missing(&p));
    assert!(budget(&p, 6).is_some(), "vbias drives gates");
    cfg.loads = vec![(NetId(4), 1e6)];
    let p = annotate(&ota(), &cfg);
    assert!(budget(&p, 4).is_some());
    assert!(budget(&p, 6).is_some());
}
