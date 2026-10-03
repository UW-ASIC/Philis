//! Recognition corpus (EXT-01, plan-01 T2/T3/T4/T6/T7): 17 circuits plus 3
//! negative-only ones, each with the leaves and [`Canon`] the annotator gives **today**. The
//! expectations characterise, they do not endorse: each later EXT item edits the
//! rows it changes in the same commit, so its diff shows exactly what moved.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use annotator::{annotate, AnnotationConfig, BlockKind};
use common::{canon, canon_leaves, Canon, id_pairs, net, permute, strongarm_cfg, STRONGARM};

/// `(name, netlist)`: repository fixtures transcribed without `.subckt`, then
/// constructed circuits (exact text of plan-01 EXT-01).
pub const CIRCUITS: [(&str, &str); 17] = [
    // backend/annotator/src/tests.rs `ota()`.
    ("ota5t", "XM1 vout1 vinp vtail VSS nfet w=10u l=1u | XM2 vout2 vinm vtail VSS nfet w=10u l=1u
               XM3 vout1 vbias VDD VDD pfet w=20u l=1u | XM4 vout2 vbias VDD VDD pfet w=20u l=1u
               XM5 vtail vbn VSS VSS nfet w=40u l=2u"),
    // examples/three_stage_opamp/src/main.rs.
    ("three_stage", "M1 n1 vin_p tail vss nfet w=4u l=0.5u | M2 n2 vin_n tail vss nfet w=4u l=0.5u
                     M3 tail vbias vss vss nfet w=8u l=0.5u
                     M4 n1 n1 vdd vdd pfet w=6u l=0.5u | M5 n2 n1 vdd vdd pfet w=6u l=0.5u
                     M6 n3 n2 vdd vdd pfet w=12u l=0.5u | M7 n3 vbias vss vss nfet w=6u l=0.5u
                     M8 vout n3 vdd vdd pfet w=40u l=0.5u | M9 vout vbias vss vss nfet w=20u l=0.5u"),
    // benchmarks/fixtures/*.spice.
    ("dac4", "XC0 top VSS cap_generic_m1m2 W=2u L=2u m=1
              XC1 top b0 cap_generic_m1m2 W=2u L=2u m=1
              XC2 top b1 cap_generic_m1m2 W=2u L=2u m=2
              XC3 top b2 cap_generic_m1m2 W=2u L=2u m=4
              XC4 top b3 cap_generic_m1m2 W=2u L=2u m=8
              XMP0 b0 d0 VDD VDD pfet_01v8 W=1u L=0.15u | XMN0 b0 d0 VSS VSS nfet_01v8 W=0.5u L=0.15u
              XMP1 b1 d1 VDD VDD pfet_01v8 W=1u L=0.15u | XMN1 b1 d1 VSS VSS nfet_01v8 W=0.5u L=0.15u
              XMP2 b2 d2 VDD VDD pfet_01v8 W=1u L=0.15u | XMN2 b2 d2 VSS VSS nfet_01v8 W=0.5u L=0.15u
              XMP3 b3 d3 VDD VDD pfet_01v8 W=1u L=0.15u | XMN3 b3 d3 VSS VSS nfet_01v8 W=0.5u L=0.15u
              XMC cmp top VSS VSS nfet_01v8 W=1u L=0.15u"),
    ("bgr_core", "XQ1 VSS VSS e1 pnp_05v5_W3p40L3p40 m=1 | XQ2 VSS VSS e2 pnp_05v5_W3p40L3p40 m=8"),
    ("bjt_mirror", "XQ1 outn in VSS npn_05v5_1x1 W=1u L=1u | XQ2 outp in VDD pnp_05v5_W3p40L3p40"),
    ("chain4", "XM1 a g b VSS nfet_01v8 W=2u L=0.5u | XM2 b g c VSS nfet_01v8 W=2u L=0.5u
                XM3 c g d VSS nfet_01v8 W=2u L=0.5u | XM4 d g e VSS nfet_01v8 W=2u L=0.5u"),
    ("pair", "XM1 d g VSS VSS nfet_01v8 W=2u L=0.5u | XM2 d g VSS VSS nfet_01v8 W=2u L=0.5u"),
    ("quad", "XM1 n g VSS VSS nfet_01v8 W=2u L=0.5u | XM2 n g VSS VSS nfet_01v8 W=2u L=0.5u
              XM3 n g VSS VSS nfet_01v8 W=2u L=0.5u | XM4 n g VSS VSS nfet_01v8 W=2u L=0.5u"),
    ("folded", "M1 x1 vinp tail VSS nfet w=10u l=1u | M2 x2 vinn tail VSS nfet w=10u l=1u | M0 tail vbn VSS VSS nfet w=20u l=1u
                M3 x1 vbp1 VDD VDD pfet w=20u l=1u | M4 x2 vbp1 VDD VDD pfet w=20u l=1u
                M5 o1 vbp2 x1 VDD pfet w=10u l=1u | M6 out vbp2 x2 VDD pfet w=10u l=1u
                M7 o1 vbn2 y1 VSS nfet w=5u l=1u | M8 out vbn2 y2 VSS nfet w=5u l=1u
                M9 y1 o1 VSS VSS nfet w=5u l=1u | M10 y2 o1 VSS VSS nfet w=5u l=1u"),
    ("gilbert", "M1 x1 rfp tail VSS nfet w=8u l=0.5u | M2 x2 rfn tail VSS nfet w=8u l=0.5u | M0 tail vb VSS VSS nfet w=16u l=0.5u
                 M3 outp lop x1 VSS nfet w=4u l=0.5u | M4 outn lon x1 VSS nfet w=4u l=0.5u
                 M5 outp lon x2 VSS nfet w=4u l=0.5u | M6 outn lop x2 VSS nfet w=4u l=0.5u"),
    ("rail2rail", "MN1 xn1 vinp tn VSS nfet w=4u l=0.5u | MP1 xp1 vinp tp VDD pfet w=8u l=0.5u
                   MN2 xn2 vinn tn VSS nfet w=4u l=0.5u | MP2 xp2 vinn tp VDD pfet w=8u l=0.5u
                   MN0 tn vbn VSS VSS nfet w=8u l=0.5u | MP0 tp vbp VDD VDD pfet w=16u l=0.5u"),
    ("latch", "MN1 q qb VSS VSS nfet w=2u l=0.15u | MP1 q qb VDD VDD pfet w=4u l=0.15u
               MN2 qb q VSS VSS nfet w=2u l=0.15u | MP2 qb q VDD VDD pfet w=4u l=0.15u"),
    ("mirror6", "MR ref ref VSS VSS nfet w=2u l=1u | MO1 o1 ref VSS VSS nfet w=2u l=1u | MO2 o2 ref VSS VSS nfet w=4u l=1u
                 MO3 o3 ref VSS VSS nfet w=2u l=1u | MO4 o4 ref VSS VSS nfet w=8u l=1u | MO5 o5 ref VSS VSS nfet w=2u l=1u"),
    ("brokaw", "Q1 c1 vb x npn m=1 | Q2 c2 vb y npn m=8 | R2 y x rpoly w=2u l=20u | R1 x VSS rpoly w=2u l=80u
                MP1 c1 c1 VDD VDD pfet w=4u l=1u | MP2 c2 c1 VDD VDD pfet w=4u l=1u"),
    ("rdiv", "RA top mid rpoly w=2u l=10u | RB mid VSS rpoly w=2u l=40u"),
    ("splitdac", "C0 tl VSS cap w=3u l=3u m=1 | C1 tl b0 cap w=3u l=3u m=1 | C2 tl b1 cap w=3u l=3u m=2
                  C3 tm b2 cap w=3u l=3u m=1 | C4 tm b3 cap w=3u l=3u m=2 | CA tl tm cap w=3u l=4u m=1"),
    ("strongarm", STRONGARM),
];

/// Negative circuits: nothing in them should be matched (T3). `bjt_mirror` is
/// the fourth, shared with [`CIRCUITS`].
const NEGATIVE: [(&str, &str); 3] = [
    ("sc_switches", "MS1 a1 p1 b1 VSS nfet w=1u l=0.15u | MS2 a2 p2 b2 VSS nfet w=1u l=0.15u"),
    ("equal_fets", "MA da ga VSS VSS nfet w=2u l=0.5u | MB db gb VSS VSS nfet w=2u l=0.5u"),
    ("inv_chain", "MN1 a in VSS VSS nfet w=1u l=0.15u | MP1 a in VDD VDD pfet w=2u l=0.15u | MN2 b a VSS VSS nfet w=1u l=0.15u | MP2 b a VDD VDD pfet w=2u l=0.15u | MN3 c b VSS VSS nfet w=1u l=0.15u | MP3 c b VDD VDD pfet w=2u l=0.15u | MN4 out c VSS VSS nfet w=1u l=0.15u | MP4 out c VDD VDD pfet w=2u l=0.15u"),
];

fn cfg(name: &str) -> AnnotationConfig {
    if name == "strongarm" { strongarm_cfg() } else { AnnotationConfig::default() }
}

fn all() -> impl Iterator<Item = (&'static str, &'static str)> {
    CIRCUITS.into_iter().chain(NEGATIVE)
}

/// Today's decisions per circuit: leaves `(kind, sorted member names)`, then the
/// [`Canon`] fields `pairs`, `selfs`, `net_pairs`, `axes` (`sets` is empty until EXT-12).
type Row = (&'static str, &'static [(&'static str, &'static [&'static str])], &'static [(&'static str, &'static str)], &'static [&'static str], &'static [(&'static str, &'static str)], usize);
const EXPECTED: [Row; 17] = [
    // EXT-05: declared roles: five_transistor_ota's slots 2,3 are a Load.
    ("ota5t", &[("Load", &["XM3", "XM4"]), ("DiffPair", &["XM1", "XM2"])], &[("XM1", "XM2"), ("XM3", "XM4")], &["XM5"], &[("vout1", "vout2")], 1),
    // EXT-05: declared roles: diff_pair_with_mirror_load's slots 2,3 are a Load.
    ("three_stage", &[("DiffPair", &["M1", "M2"]), ("Group", &["M6", "M8", "M9"]), ("Load", &["M4", "M5"])], &[("M1", "M2"), ("M4", "M5")], &["M3"], &[("n1", "n2")], 1),
    // EXT-05 review: `cmos_inverter`'s declared prox makes each switch inverter a Stack leaf.
    ("dac4", &[("Stack", &["XMN0", "XMP0"]), ("Stack", &["XMN1", "XMP1"]), ("Stack", &["XMN2", "XMP2"]), ("Stack", &["XMN3", "XMP3"])], &[], &[], &[], 0),
    ("bgr_core", &[], &[], &[], &[], 0),
    ("bjt_mirror", &[], &[], &[], &[], 0),
    // EXT-05: series_stack_4 declares no roles, so no re-searched Stack children.
    ("chain4", &[("Group", &["XM1", "XM2", "XM3", "XM4"])], &[], &[], &[], 0),
    ("pair", &[], &[], &[], &[], 0),
    ("quad", &[], &[], &[], &[], 0),
    // EXT-04: DIFF_PAIR_SPLIT_SOURCE (any-pins-differ, no shared net) was a dead
    // pattern that falsely matched M7/M9 and M10/M8 as DiffPair; deleted, so
    // they fall back to their real Stack structure.
    // EXT-05: folded_cascode_core's declared pairs and prox; the M7-M10 composite
    // declares no roles.
    ("folded", &[("CascodePair", &["M5", "M6"]), ("DiffPair", &["M1", "M2"]), ("Group", &["M10", "M7", "M8", "M9"]), ("Load", &["M3", "M4"]), ("Stack", &["M3", "M5"]), ("Stack", &["M4", "M6"])], &[("M1", "M2"), ("M3", "M4"), ("M5", "M6")], &[], &[("x1", "x2")], 1),
    // EXT-04: GILBERT_CELL's link fix makes it match the whole 6-device cell
    // (today's child re-search, max_slots=2, decomposes it into 3 DiffPair legs).
    // EXT-05: declared pairs are the mirror images (M3,M6), (M4,M5).
    ("gilbert", &[("DiffPair", &["M1", "M2"]), ("DiffPair", &["M3", "M6"]), ("DiffPair", &["M4", "M5"])], &[("M1", "M2"), ("M3", "M6"), ("M4", "M5")], &[], &[("outn", "outp"), ("x1", "x2")], 1),
    // EXT-05: complementary_diff_pair declares both polarities' DiffPairs.
    ("rail2rail", &[("DiffPair", &["MN1", "MN2"]), ("DiffPair", &["MP1", "MP2"])], &[("MN1", "MN2"), ("MP1", "MP2")], &[], &[("xn1", "xn2"), ("xp1", "xp2")], 1),
    // EXT-05 (AA-03): cross_coupled_inverters' declared pairs and prox, not inverters.
    // EXT-09: `net_pairs` moves — Differential now comes from the DiffPair leaves, and
    // the cross-coupled MN1/MN2 pair's drains (q, qb) differ, so it yields one.
    ("latch", &[("DiffPair", &["MN1", "MN2"]), ("DiffPair", &["MP1", "MP2"]), ("Stack", &["MN1", "MP1"]), ("Stack", &["MN2", "MP2"])], &[("MN1", "MN2"), ("MP1", "MP2")], &[], &[("q", "qb")], 1),
    // EXT-05: current_mirror_4 declares (ref, k) per output; only the first pair
    // holding the shared reference gets a Symmetry.
    // EXT-06: equal-priority current_mirror_4 matches tie on canonical labels, not
    // device ids: MR takes the three identical 2 µm outputs, MO2/MO4 pair up.
    ("mirror6", &[("CurrentMirror", &["MO1", "MR"]), ("CurrentMirror", &["MO3", "MR"]), ("CurrentMirror", &["MO5", "MR"]), ("CurrentMirror", &["MO2", "MO4"])], &[("MO1", "MR"), ("MO2", "MO4")], &[], &[], 2),
    ("brokaw", &[("CurrentMirror", &["MP1", "MP2"])], &[("MP1", "MP2")], &[], &[], 1),
    ("rdiv", &[], &[], &[], &[], 0),
    ("splitdac", &[], &[], &[], &[], 0),
    // EXT-05: diff_pair_cross_coupled_load's declared tail mn0 (not mp8) and Load;
    // cross_coupled_inverters' pairs; complementary_diff_pair's sources must be a
    // signal, so the output inverters no longer match it and mp9/mp10 join
    // undeclared 3-device groups (their CurrentMirror was a misrecognition).
    // EXT-05 review: the out-of-plan `diff_pair_cross_coupled_load` entry is gone; the
    // row is unchanged, as `five_transistor_ota` (EXT-06 selection) claims mn1/mn2/mp7/mp8/mn0.
    // EXT-09: `net_pairs` gains (vin_o, vip_o) — the second DiffPair leaf (mn3, mn4)
    // now also yields a Differential; the old device-pair scan missed it.
    ("strongarm", &[("DiffPair", &["mn1", "mn2"]), ("DiffPair", &["mn3", "mn4"]), ("DiffPair", &["mp5", "mp6"]), ("Group", &["mn13", "mp10", "mp11"]), ("Group", &["mn14", "mp12", "mp9"]), ("Load", &["mp7", "mp8"]), ("Stack", &["mn3", "mp5"]), ("Stack", &["mn4", "mp6"])], &[("mn1", "mn2"), ("mn3", "mn4"), ("mp5", "mp6"), ("mp7", "mp8")], &["mn0"], &[("vin_d", "vip_d"), ("vin_o", "vip_o")], 2),
];

#[test]
fn corpus_expectations() {
    assert_eq!(CIRCUITS.map(|c| c.0), EXPECTED.map(|r| r.0), "one row per circuit, same order");
    let owned = |v: &[(&str, &str)]| v.iter().map(|(a, b)| ((*a).to_string(), (*b).to_string())).collect();
    let mut bad = Vec::new();
    for ((name, src), (_, leaves, pairs, selfs, net_pairs, axes)) in CIRCUITS.into_iter().zip(EXPECTED) {
        let nl = net(src);
        let p = annotate(&nl, &cfg(name));
        let got = (canon(&p, &nl), canon_leaves(&p, &nl));
        let exp = (
            Canon { pairs: owned(pairs), selfs: selfs.iter().map(|s| (*s).to_string()).collect(), net_pairs: owned(net_pairs), axes, ..Canon::default() },
            leaves.iter().map(|(k, m)| ((*k).to_string(), m.iter().map(|s| (*s).to_string()).collect())).collect::<BTreeSet<(String, Vec<String>)>>(),
        );
        if got != exp {
            bad.push(format!("{name}:\n  got  {got:?}\n  want {exp:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// T3 at M0: no symmetry pair, no matched set, and no matched leaf (DiffPair,
/// CurrentMirror, Load: today's matched sets, before EXT-12's `MatchSpec`).
fn assert_unmatched(name: &str, src: &str) {
    let nl = net(src);
    let p = annotate(&nl, &cfg(name));
    let c = canon(&p, &nl);
    assert!(c.pairs.is_empty() && c.sets.is_empty(), "{name}: {c:?}");
    let matched: Vec<_> = annotator::block::leaves(&p.blocks)
        .into_iter()
        .filter(|b| matches!(b.kind, BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load))
        .map(|b| (b.kind, b.devices.clone()))
        .collect();
    assert!(matched.is_empty(), "{name}: matched leaves {matched:?}");
}

/// The netlist named `name` in [`CIRCUITS`] or [`NEGATIVE`].
fn src(name: &str) -> &'static str {
    all().find(|c| c.0 == name).unwrap_or_else(|| panic!("no circuit {name}")).1
}

#[test]
fn negative_corpus() {
    for name in ["bjt_mirror", "inv_chain"] {
        assert_unmatched(name, src(name));
    }
}

#[test]
fn negative_corpus_sc_switches_and_equal_fets() {
    for name in ["sc_switches", "equal_fets"] {
        assert_unmatched(name, src(name));
    }
}

#[test]
fn permutation_invariance() {
    for (name, src) in all() {
        let nl = net(src);
        let p = annotate(&nl, &cfg(name));
        let base = (canon(&p, &nl), canon_leaves(&p, &nl));
        for seed in 1..=20 {
            let q = permute(&nl, seed);
            let pq = annotate(&q, &cfg(name));
            assert_eq!((canon(&pq, &q), canon_leaves(&pq, &q)), base, "{name} seed {seed}");
        }
    }
}

/// T6 over the emitted placement arms of `name`: the unordered device pairs
/// carrying both a `Proximity` and an `Isolation`, and the devices in more
/// than one hard `Symmetry` entry (two pairs, or two axes). Both empty is T6.
fn conflicts(name: &str, src: &str) -> (Vec<(String, String)>, Vec<String>) {
    let nl = net(src);
    let p = annotate(&nl, &cfg(name));
    let arms = || p.placement.hard.iter().chain(&p.placement.budget).chain(&p.placement.cost);
    let pairs = |kind: &str| -> BTreeSet<(u32, u32)> {
        arms().filter(|b| b.kind().ends_with(kind)).flat_map(|b| id_pairs(b.as_ref())).map(|(a, b)| (a.min(b), a.max(b))).collect()
    };
    let dev = |d: u32| nl.devices[d as usize].name.clone();
    let both = pairs("::Proximity").intersection(&pairs("::Isolation")).map(|&(a, b)| (dev(a), dev(b))).collect();
    let mut mirror = Vec::new();
    p.placement.hard.iter().for_each(|b| b.mirror_pairs(&mut mirror));
    let mut seen = BTreeMap::new();
    for &(a, b, _) in &mirror {
        for d in if a == b { vec![a] } else { vec![a, b] } {
            *seen.entry(d).or_insert(0) += 1;
        }
    }
    let twice = seen.into_iter().filter(|&(_, n)| n > 1).map(|(d, _)| dev(d)).collect();
    (both, twice)
}

fn assert_no_conflicts(name: &str, src: &str) {
    let (both, twice) = conflicts(name, src);
    assert!(both.is_empty(), "{name}: Proximity and Isolation on {both:?}");
    assert!(twice.is_empty(), "{name}: devices in two symmetry entries: {twice:?}");
}

/// T6 on every corpus circuit but strongarm, which has its own test below.
#[test]
fn no_emitted_conflicts() {
    for (name, src) in all().filter(|c| c.0 != "strongarm") {
        assert_no_conflicts(name, src);
    }
}

/// AA-13: the clocked tail `mn0` sits in the input pair's stage, so it gets
/// Proximity ≤ 5 µm to `mn1`/`mn2`; Isolation
/// from them would contradict it. GAP-04 exempts same-block pairs.
#[test]
fn no_emitted_conflicts_strongarm() {
    assert_no_conflicts("strongarm", STRONGARM);
}

/// EXT-09: `Differential` comes only from recognised `DiffPair` leaves, never a
/// positional/O(N²) device-pair scan, and lands in `routing.budget`, not `.hard`.
#[test]
fn differential_comes_from_recognized_pairs() {
    use analog::RuleBatch;
    let count = |arm: &[Box<dyn RuleBatch<pnr_core::Routes>>]| -> usize {
        arm.iter().filter(|b| b.kind().ends_with("::Differential")).map(|b| b.count()).sum()
    };
    // dac4 has no recognised pair (it is a DAC capacitor bank): 0 Differential
    // anywhere (today's positional scan finds 10 on its NMOS/PMOS switch pairs).
    let nl = net(&src("dac4").replace("VSS", "0"));
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(count(&p.routing.hard) + count(&p.routing.budget) + count(&p.routing.cost), 0, "dac4");
    assert!(xtalk(&nl, &p).is_empty(), "dac4 crosstalk");

    // ota5t has exactly one DiffPair leaf: exactly 1 Differential, in budget only,
    // and a CrosstalkExclusion from each input gate to each output drain.
    let nl = net(src("ota5t"));
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(count(&p.routing.hard), 0, "ota5t hard");
    assert_eq!(count(&p.routing.budget), 1, "ota5t budget");
    let want: BTreeSet<_> = [("vinm", "vout1"), ("vinm", "vout2"), ("vinp", "vout1"), ("vinp", "vout2")].map(|(a, b)| (a.to_string(), b.to_string())).into();
    let got = xtalk(&nl, &p);
    assert_eq!((got.len(), got.into_iter().collect::<BTreeSet<_>>()), (4, want), "ota5t crosstalk");

    // latch: its two DiffPair leaves share drains (q, qb); one rule per net pair.
    let nl = net(src("latch"));
    let p = annotate(&nl, &AnnotationConfig::default());
    assert_eq!(count(&p.routing.budget), 1, "latch Differential");
    assert_eq!(xtalk(&nl, &p), [("q".to_string(), "qb".to_string())], "latch crosstalk");
}

/// `CrosstalkExclusion` net-name pairs (each sorted), in emission order.
fn xtalk(nl: &pnr_core::Netlist, p: &annotator::Problem) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for b in p.routing.budget.iter().filter(|b| b.kind().ends_with("::CrosstalkExclusion")) {
        let mut nets = Vec::new();
        b.touched(&mut nets);
        for c in nets.chunks(2) {
            let (a, z) = (nl.nets[c[0] as usize].name.clone(), nl.nets[c[1] as usize].name.clone());
            v.push(if a < z { (a, z) } else { (z, a) });
        }
    }
    v
}

/// T7: every device is in a requirement or reported `Unconstrained(reason)`.
#[test]
fn coverage_is_total() {
    use annotator::Coverage::{Constrained, Grouped, Unconstrained};
    for (name, src) in all() {
        let nl = net(src);
        let p = annotate(&nl, &cfg(name));
        let ids: Vec<u16> = p.coverage.iter().map(|(d, _)| d.0).collect();
        assert_eq!(ids, (0..nl.devices.len() as u16).collect::<Vec<_>>(), "{name}: one entry per device, in id order");
        for (d, c) in &p.coverage {
            if let Unconstrained(why) = c {
                assert!(["do_not_identify", "unknown size", "no pattern"].contains(why), "{name} {d:?}: {why}");
            }
        }
        match name {
            "ota5t" => assert!(p.coverage.iter().all(|c| c.1 == Constrained), "{name}: {:?}", p.coverage),
            // A role-less composite: recognised, nothing emitted on it.
            "chain4" => assert!(p.coverage.iter().all(|c| c.1 == Grouped("series_stack_4")), "{name}: {:?}", p.coverage),
            "rdiv" => assert!(p.coverage.iter().all(|c| c.1 == Unconstrained("no pattern")), "{name}: {:?}", p.coverage),
            _ => {}
        }
    }
}

/// EXT-10: a batch's id survives a netlist permutation (ids follow emission
/// order, which follows EXT-06's canonical block order).
#[test]
fn ids_survive_permutation() {
    let id_of = |nl: &pnr_core::Netlist| {
        let p = annotate(nl, &AnnotationConfig::default());
        let want: BTreeSet<&str> = ["XM1", "XM2"].into();
        let b = p.placement.cost.iter().find(|b| {
            let mut ids = Vec::new();
            b.touched(&mut ids);
            b.kind().ends_with("::MatchingPair") && ids.iter().map(|&d| nl.devices[d as usize].name.as_str()).collect::<BTreeSet<_>>() == want
        });
        b.expect("XM1/XM2 MatchingPair cost batch").meta().expect("tagged").id
    };
    let nl = net(src("ota5t"));
    let base = id_of(&nl);
    for s in 1..=5 {
        assert_eq!(id_of(&permute(&nl, s)), base, "seed {s}");
    }
}
