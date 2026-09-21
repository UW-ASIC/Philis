//! Track B / P0: (netlist, solved layout) → emit → interpret round-trip.
//! The layout here is a hand-solved placement (the emitter's contract is
//! `(netlist, layout)`, not the flow — `run()` is just one producer; its
//! chain2 debug fragility is task #10). Checks the emitted generator carries
//! the *decisions*: instances, params, relational placement with
//! rule-attributed gaps, connectivity — and that the printed source is real
//! macroMaster code.

use library::emit::{elaborate_ir, emit, to_rust, IrGap};
use library::{Config, ElabConfig};
use pnr_core::{Layout, Orient};

// Two unmatched NMOS in a chain — deliberately no symmetry/matching so no
// group collapse (v1 emitter scope); different widths keep them distinct.
const CHAIN2: &str = r"
.subckt chain2 in mid out vss
M1 mid in vss vss nfet w=0.42u l=0.15u
M2 out mid vss vss nfet w=0.84u l=0.15u
.ends
";

// A matched diff pair — identical W/L, shared source — which the annotator
// collapses into ONE cell. emit v2 must express it as a `MatchedPair` with
// per-leg connectivity, not reject it.
const DIFFPAIR: &str = r"
.subckt diffpair inp inn outp outn tail vss
M1 outp inp tail vss nfet w=0.84u l=0.15u
M2 outn inn tail vss nfet w=0.84u l=0.15u
.ends
";

#[test]
fn matched_pair_emits_as_one_interdigitated_instance() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let cfg = Config::default();
    let netlist = library::parse(DIFFPAIR).expect("diffpair parses");

    // Hand layout sized for either outcome (1 collapsed cell or 2).
    let layout = Layout {
        x: vec![800, 3000],
        y: vec![600, 600],
        hw: vec![800, 800],
        hh: vec![600, 600],
        axis: vec![0],
        groups: Vec::new(),
        orient: vec![Orient::default(); 2],
        variant: vec![0; 2],
        branch: Vec::new(),
        power_uw: vec![0; 2],
        temp_mc: vec![0; 2],
    };

    let ir = emit(&netlist, &layout, &pdk, &cfg).expect("matched pair is in emit v2 scope");
    assert_eq!(ir.instances.len(), 1, "matched pair collapses to one instance: {ir:?}");
    let pair = &ir.instances[0];
    assert_eq!(pair.legs, 2);
    assert_eq!(pair.name, "M1_M2");
    // Per-leg connectivity: leg 1 gate → inp, leg 2 gate → inn, sources → tail.
    let has = |t: &str, n: &str| ir.edges.iter().any(|(a, b)| a == &format!("M1_M2.{t}") && b == n);
    assert!(has("g1", "inp") && has("g2", "inn"), "leg gates wired: {:?}", ir.edges);
    assert!(has("s1", "tail") && has("s2", "tail"), "shared tail: {:?}", ir.edges);
    assert!(has("d1", "outp") && has("d2", "outn"), "leg drains wired: {:?}", ir.edges);

    // Interprets + routes; printed source names MatchedPair.
    let re = elaborate_ir(&ir, &pdk, &ElabConfig::default()).expect("IR elaborates");
    assert_eq!(re.macros.len(), 1, "one interdigitated macro");
    let tail = re.nets.iter().position(|n| n == "tail").expect("tail net");
    assert!(!re.routes.shapes(pnr_core::NetId(tail as u16)).is_empty(), "tail routed");
    assert!(to_rust(&ir).contains("MatchedPair { kind: DeviceKind::Nmos"), "source uses MatchedPair");
}

#[test]
fn chain2_roundtrips_through_ir() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let cfg = Config::default();
    let netlist = library::parse(CHAIN2).expect("chain2 parses");

    // A hand-solved placement: one row, edge gap exactly the deck's
    // device_gap (600) so attribution fires.
    let layout = Layout {
        x: vec![500, 2100],
        y: vec![400, 400],
        hw: vec![500, 500],
        hh: vec![400, 400],
        axis: vec![0],
        groups: Vec::new(),
        orient: vec![Orient::default(); 2],
        variant: vec![0; 2],
        branch: Vec::new(),
        power_uw: vec![0; 2],
        temp_mc: vec![0; 2],
    };

    // ── decompile ──
    let ir = emit(&netlist, &layout, &pdk, &cfg).expect("chain2 is within v1 emitter scope");
    assert_eq!(ir.instances.len(), 2);
    let m1 = ir.instances.iter().find(|i| i.name == "M1").expect("M1 emitted");
    let m2 = ir.instances.iter().find(|i| i.name == "M2").expect("M2 emitted");
    // Unitization decomposes total width into unit fingers: 420×1 and 420×2.
    assert_eq!((m1.w * i32::from(m1.nf), m1.l), (420, 150));
    assert_eq!((m2.w * i32::from(m2.nf), m2.l), (840, 150));
    // Both gates wired: M1.g → in, M2.g → mid.
    assert!(ir.edges.iter().any(|(a, b)| a == "M1.g" && b == "in"));
    assert!(ir.edges.iter().any(|(a, b)| a == "M2.g" && b == "mid"));
    // The second placement is relational and its row gap is rule-attributed.
    let aligns = &ir.place[1].aligns;
    assert!(!aligns.is_empty(), "non-anchor placement must be relative");
    assert!(
        aligns.iter().any(|a| matches!(&a.gap, IrGap::Rule(name, _) if name == "device_gap")),
        "600nm row gap attributes to device_gap, got {aligns:?}"
    );

    // ── re-elaborate the IR on the same deck ──
    let re = elaborate_ir(&ir, &pdk, &ElabConfig::default()).expect("IR elaborates");
    assert_eq!(re.macros.len(), 2);
    // Relative order preserved: M2 to the right of M1.
    assert!(re.macros[1].bbox.x > re.macros[0].bbox.x, "row order survives the round trip");
    // The shared net `mid` (M1 drain ↔ M2 gate) must be routed.
    let mid = re.nets.iter().position(|n| n == "mid").expect("mid net exists");
    assert!(
        !re.routes.shapes(pnr_core::NetId(mid as u16)).is_empty(),
        "mid net routed in the re-elaboration"
    );

    // ── P4: the same IR retargets to a second deck ──
    let deck2 = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/generic_finfet.json"
    ))
    .expect("generic_finfet deck present");
    let pdk2 = verify::Pdk::from_json(&deck2).expect("deck2 parses");
    let re2 = elaborate_ir(&ir, &pdk2, &ElabConfig::default()).expect("IR retargets");
    assert_eq!(re2.macros.len(), 2);
    assert!(re2.macros[1].bbox.x > re2.macros[0].bbox.x);
    let mid2 = re2.nets.iter().position(|n| n == "mid").expect("mid on deck2");
    assert!(!re2.routes.shapes(pnr_core::NetId(mid2 as u16)).is_empty(), "mid routed on deck2");

    // ── printed source is real macroMaster code ──
    let src = to_rust(&ir);
    for needle in [
        "impl Composition for emitted",
        "c.instantiate(\"M1\"",
        "c.instantiate(\"M2\"",
        "AlignMode::",
        "c.process().rule(\"device_gap\"",
        "c.connect(\"M1.g\", \"in\");",
        "Ok(())",
    ] {
        assert!(src.contains(needle), "emitted source missing `{needle}`:\n{src}");
    }
}

