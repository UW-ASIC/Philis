//! Corner-case coverage for `pnr_core`'s public API. Oracles are hand-worked
//! values, the doc-comment contracts, and metamorphic relations (symmetry,
//! translation invariance, group laws) — never a re-run of the code under test.

use pnr_core::lanes::{self, Box4, Boxes};
use pnr_core::routes::{conductor_layers_meet, open_components, Join};
use pnr_core::thermal::{rise_bound_mc, rises_mc, self_rise_mc, K_SI_W_PER_M_K};
use pnr_core::units::{Unit, UnitLib};
use pnr_core::{
    pin_shares, place_macro, place_macros, place_rect, AxisId, BipartiteHypergraph, Device, DeviceId, DeviceKind,
    GatePin, GroupId, LayerId, Layout, Macro, MatchClass, MosSize, Net, NetId, Netlist, Orient, Pin, Rect, Report,
    Routes, Shape, SubstrateKind, Target, Terminal, UnionFind, Violation,
};

// ---------------------------------------------------------------- fixtures

fn layout(x: Vec<i32>, y: Vec<i32>, hw: Vec<i32>, hh: Vec<i32>) -> Layout {
    let n = x.len();
    Layout {
        x,
        y,
        hw,
        hh,
        orient: vec![Orient::R0; n],
        variant: vec![0; n],
        axis: Vec::new(),
        branch: Vec::new(),
        groups: Vec::new(),
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
        units: Default::default(),
    }
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

fn shape(layer: u16, r: Rect) -> Shape {
    Shape { layer: LayerId(layer), rect: r }
}

fn device(kind: DeviceKind, params: &[(&str, i64)], terminals: &[(&str, u16)]) -> Device {
    Device {
        name: "M1".into(),
        kind,
        model: String::new(),
        terminals: terminals.iter().map(|&(t, n)| (t.to_string(), NetId(n))).collect(),
        params: params.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
    }
}

// ---------------------------------------------------------------- geom

#[test]
fn touches_is_closed_and_symmetric() {
    let a = rect(0, 0, 10, 10);
    let corner = rect(10, 10, 5, 5);
    let gap_y = rect(0, 11, 5, 5);
    let inside = rect(2, 2, 1, 1);
    let point = rect(10, 5, 0, 0);
    for (b, want) in [(corner, true), (gap_y, false), (inside, true), (point, true)] {
        assert_eq!(a.touches(&b), want, "{b:?}");
        assert_eq!(b.touches(&a), want, "symmetric {b:?}");
    }
    assert!(rect(-20, -20, 5, 5).touches(&rect(-15, -15, 1, 1)));
}

#[test]
fn orient_known_images_of_unit_x() {
    let want = [
        (Orient::R0, (1, 0)),
        (Orient::R90, (0, 1)),
        (Orient::R180, (-1, 0)),
        (Orient::R270, (0, -1)),
        (Orient::Mx, (1, 0)),
        (Orient::Mx90, (0, 1)),
        (Orient::Mx180, (-1, 0)),
        (Orient::Mx270, (0, -1)),
    ];
    for (o, p) in want {
        assert_eq!(o.apply(1, 0), p, "{o:?}");
    }
    assert_eq!(Orient::Mx.apply(0, 1), (0, -1));
    assert_eq!(Orient::Mx180.apply(3, 4), (-3, 4));
    assert_eq!(Orient::default(), Orient::R0);
}

#[test]
fn orient_is_a_group() {
    for a in Orient::ALL {
        assert_eq!(Orient::R0.then(a), a, "left identity");
        assert_eq!(a.then(Orient::R0), a, "right identity");
        assert_eq!(a.inverse().then(a), Orient::R0, "two-sided inverse {a:?}");
        assert_eq!(a.inverse().inverse(), a);
        for b in Orient::ALL {
            for c in Orient::ALL {
                assert_eq!(a.then(b).then(c), a.then(b.then(c)), "associative");
            }
        }
    }
    assert_eq!(Orient::ALL.iter().filter(|o| o.swaps_axes()).count(), 4);
}

#[test]
fn apply_rect_keeps_area_and_maps_corners() {
    let r = rect(-30, 15, 70, 20);
    for o in Orient::ALL {
        let t = o.apply_rect(r);
        assert!(t.w >= 0 && t.h >= 0);
        assert_eq!(i64::from(t.w) * i64::from(t.h), 1400, "{o:?}");
        // Every corner of `r` lands on a corner of `t`.
        for (x, y) in [(r.x, r.y), (r.x + r.w, r.y), (r.x, r.y + r.h), (r.x + r.w, r.y + r.h)] {
            let (px, py) = o.apply(x, y);
            assert!(px == t.x || px == t.x + t.w, "{o:?}");
            assert!(py == t.y || py == t.y + t.h, "{o:?}");
        }
    }
    // A degenerate rect stays degenerate.
    assert_eq!(Orient::R90.apply_rect(rect(5, 0, 10, 0)), rect(0, 5, 0, 10));
}

// ---------------------------------------------------------------- ids

#[test]
fn retarget_maps_devices_and_passes_the_rest() {
    let cell_of = [3u16, 3, 7];
    assert_eq!(Target::Device(DeviceId(1)).retarget(&cell_of), Target::Device(DeviceId(3)));
    assert_eq!(Target::Device(DeviceId(2)).retarget(&cell_of), Target::Device(DeviceId(7)));
    assert_eq!(Target::Device(DeviceId(3)).retarget(&cell_of), Target::Device(DeviceId(3)), "past the end");
    assert_eq!(Target::Device(DeviceId(0)).retarget(&[]), Target::Device(DeviceId(0)), "empty map");
    assert_eq!(Target::Group(GroupId(0)).retarget(&cell_of), Target::Group(GroupId(0)));
}

// ---------------------------------------------------------------- unionfind

#[test]
fn union_find_empty_and_singletons() {
    assert!(UnionFind::new(0).groups().is_empty());
    let mut uf = UnionFind::new(4);
    for i in 0..4 {
        assert_eq!(uf.find(i), i);
    }
    uf.union(2, 2);
    assert!(uf.groups().is_empty(), "self-union leaves a singleton");
}

#[test]
fn union_find_is_transitive_and_idempotent() {
    let mut uf = UnionFind::new(6);
    uf.union(0, 1);
    uf.union(1, 2);
    uf.union(2, 0);
    uf.union(4, 5);
    assert_eq!(uf.find(0), uf.find(2));
    assert_ne!(uf.find(0), uf.find(4));
    assert_ne!(uf.find(3), uf.find(0));
    let r = uf.find(1);
    assert_eq!(uf.find(1), r, "find is stable");
    assert_eq!(uf.groups(), vec![vec![0, 1, 2], vec![4, 5]]);
}

/// The doc promises groups ordered by smallest member, so the annotator's
/// group table is identical run to run.
#[test]
fn union_find_groups_are_deterministic() {
    for _ in 0..8 {
        let mut uf = UnionFind::new(40);
        for (a, b) in [(30, 31), (5, 39), (0, 20), (12, 13), (20, 7)] {
            uf.union(a, b);
        }
        assert_eq!(uf.groups(), vec![vec![0, 7, 20], vec![5, 39], vec![12, 13], vec![30, 31]]);
    }
}

#[test]
#[should_panic]
fn union_find_out_of_range_panics() {
    UnionFind::new(2).find(2);
}

// ---------------------------------------------------------------- hypergraph

#[test]
fn hypergraph_of_empty_netlist_is_empty() {
    let hg = BipartiteHypergraph::from_netlist(&Netlist::default());
    assert_eq!(hg.device_count(), 0);
    assert!(hg.net_devices.is_empty() && hg.net_names.is_empty());
}

#[test]
fn hypergraph_lists_both_directions() {
    let nl = Netlist {
        devices: vec![
            // Diode-connected: G and D on net 0.
            device(DeviceKind::Nmos, &[], &[("G", 0), ("D", 0), ("S", 1), ("B", 1)]),
            device(DeviceKind::Resistor, &[], &[("P", 0), ("N", 2)]),
        ],
        nets: ["a", "gnd", "b", "unused"].iter().map(|n| Net { name: (*n).into() }).collect(),
        ..Netlist::default()
    };
    let hg = BipartiteHypergraph::from_netlist(&nl);
    assert_eq!(hg.device_count(), 2);
    assert_eq!(hg.device_nets[0], vec![NetId(0), NetId(0), NetId(1), NetId(1)]);
    assert_eq!(hg.terminals[1], vec!["P".to_string(), "N".to_string()]);
    assert_eq!(hg.kinds, vec![DeviceKind::Nmos, DeviceKind::Resistor]);
    assert_eq!(hg.net_devices[0], vec![DeviceId(0), DeviceId(0), DeviceId(1)], "once per terminal, ascending");
    assert_eq!(hg.net_devices[2], vec![DeviceId(1)]);
    assert!(hg.net_devices[3].is_empty());
    assert_eq!(hg.net_names[1], "gnd");
}

#[test]
#[should_panic]
fn hypergraph_panics_on_dangling_net() {
    let nl = Netlist { devices: vec![device(DeviceKind::Resistor, &[], &[("P", 5)])], ..Netlist::default() };
    let _ = BipartiteHypergraph::from_netlist(&nl);
}

// ---------------------------------------------------------------- netlist

#[test]
fn mos_size_rejects_non_mos_and_bad_sizes() {
    assert_eq!(device(DeviceKind::Resistor, &[("w", 1), ("l", 1)], &[]).mos_size(), None);
    assert_eq!(device(DeviceKind::Pmos, &[("w", 1_000)], &[]).mos_size(), None, "missing l");
    assert_eq!(device(DeviceKind::Pmos, &[("w", -5), ("l", 100)], &[]).mos_size(), None, "negative w");
    assert_eq!(device(DeviceKind::Capacitor, &[], &[]).gate_area_um2(), 0.0);
}

#[test]
fn mos_size_counts_default_and_clamp() {
    let s = device(DeviceKind::Pmos, &[("w", 3_000), ("l", 150), ("nf", 0), ("m", -2)], &[]).mos_size().unwrap();
    assert_eq!((s.nf, s.m), (1, 1), "nf/m read as at least 1");
    assert_eq!(s.w_finger_nm(), 3_000);
    let s = device(DeviceKind::Nmos, &[("w", 3_000), ("l", 150), ("m", 2), ("multi", 3)], &[]).mos_size().unwrap();
    assert_eq!(s.m, 6, "multi multiplies into m");
    let s = device(DeviceKind::Nmos, &[("w", 1), ("l", 1), ("nf", i64::MAX)], &[]).mos_size().unwrap();
    assert_eq!(s.nf, u32::MAX, "clamped to u32");
    let s = MosSize { w_total_nm: 1, l_nm: 1, nf: u32::MAX, m: 2 };
    assert_eq!(s.fingers(), u32::MAX, "saturates");
    // Floor: 1001 nm over 2 fingers.
    assert_eq!(MosSize { w_total_nm: 1_001, l_nm: 1, nf: 2, m: 1 }.w_finger_nm(), 500);
}

/// 1e10 nm × 1e10 nm = 1e8 µm × 1e8 µm... the product of the nm values is past
/// i64; the area must still come out as 1e14 µm², not overflow.
#[test]
fn gate_area_does_not_overflow() {
    let s = MosSize { w_total_nm: 10_000_000_000, l_nm: 10_000_000_000, nf: 1, m: 1 };
    let a = s.gate_area_um2();
    assert!((a - 1e14).abs() / 1e14 < 1e-9, "{a}");
}

// ---------------------------------------------------------------- process

#[test]
fn substrate_kind_parses_only_known_keys() {
    assert_eq!(SubstrateKind::from_key(Some("bulk")), SubstrateKind::Bulk);
    assert_eq!(SubstrateKind::from_key(Some("epi_on_pplus")), SubstrateKind::EpiOnLowRes);
    assert_eq!(SubstrateKind::from_key(Some("Bulk")), SubstrateKind::Unknown);
    assert_eq!(SubstrateKind::from_key(None), SubstrateKind::Unknown);
    assert_eq!(SubstrateKind::default(), SubstrateKind::Unknown);
    assert_eq!([MatchClass::Minimal as usize, MatchClass::Moderate as usize, MatchClass::Exceptional as usize], [0, 1, 2]);
    assert!(MatchClass::Minimal < MatchClass::Exceptional);
}

// ---------------------------------------------------------------- layout

#[test]
fn footprint_of_empty_single_and_pair() {
    assert_eq!(layout(vec![], vec![], vec![], vec![]).footprint_nm2(), 0.0);
    assert_eq!(layout(vec![7], vec![-3], vec![5], vec![2]).footprint_nm2(), 40.0);
    // Boxes [-1,1]×[-1,1] and [9,11]×[4,6] → 12 × 7.
    assert_eq!(layout(vec![0, 10], vec![0, 5], vec![1, 1], vec![1, 1]).footprint_nm2(), 84.0);
}

#[test]
fn group_bbox_encloses_every_member() {
    // Members [-1, 1] and [2, 2] on x: span [-1, 2] is odd.
    let mut l = layout(vec![0, 2], vec![0, 0], vec![1, 0], vec![1, 1]);
    l.groups = vec![vec![DeviceId(0), DeviceId(1)]];
    let (cx, cy, hw, hh) = l.bbox(Target::Group(GroupId(0)));
    assert!(cx - hw <= -1 && cx + hw >= 2, "x span not enclosed: {cx}±{hw}");
    assert!(hw <= 2, "rounded out by more than 1 nm: {hw}");
    assert_eq!((cy, hh), (0, 1));
    // Negative odd span [-3, 0].
    let mut l = layout(vec![-2, 0], vec![0, 0], vec![1, 0], vec![0, 0]);
    l.groups = vec![vec![DeviceId(0), DeviceId(1)]];
    let (cx, _, hw, _) = l.bbox(Target::Group(GroupId(0)));
    assert!(cx - hw <= -3 && cx + hw >= 0, "{cx}±{hw}");
    // Even span is exact.
    let mut l = layout(vec![0, 10], vec![0, 0], vec![2, 2], vec![1, 1]);
    l.groups = vec![vec![DeviceId(1), DeviceId(0)]];
    assert_eq!(l.bbox(Target::Group(GroupId(0))), (5, 0, 7, 1));
    assert_eq!(l.centre(Target::Group(GroupId(0))), (5, 0));
    assert_eq!(l.extent(Target::Group(GroupId(0))), (7, 1));
    assert_eq!(l.bbox(Target::Device(DeviceId(1))), (10, 0, 2, 1));
}

#[test]
#[should_panic(expected = "no members")]
fn empty_group_bbox_panics() {
    let mut l = layout(vec![0], vec![0], vec![1], vec![1]);
    l.groups = vec![vec![]];
    let _ = l.bbox(Target::Group(GroupId(0)));
}

#[test]
fn axis_x_falls_back_to_mean_centre() {
    let mut l = layout(vec![-3, 0], vec![0, 0], vec![1, 1], vec![1, 1]);
    assert_eq!(l.centre_x_estimate(), -1, "truncates toward zero");
    assert_eq!(l.axis_x(AxisId(0)), -1);
    l.axis = vec![42];
    assert_eq!(l.axis_x(AxisId(0)), 42);
    assert_eq!(l.axis_x(AxisId(1)), -1);
    assert_eq!(layout(vec![], vec![], vec![], vec![]).centre_x_estimate(), 0);
    // i32 extremes sum in i64.
    let l = layout(vec![i32::MAX, i32::MAX], vec![0, 0], vec![0, 0], vec![0, 0]);
    assert_eq!(l.centre_x_estimate(), i32::MAX);
}

#[test]
fn edge_gap_is_euclidean_and_symmetric() {
    // Box A [-1,1]², box B centre (6, 7) half 2 → gaps 3 and 4.
    let l = layout(vec![0, 6, 2], vec![0, 7, 0], vec![1, 2, 1], vec![1, 2, 1]);
    let (a, b, c) = (Target::Device(DeviceId(0)), Target::Device(DeviceId(1)), Target::Device(DeviceId(2)));
    assert_eq!(l.edge_gap(a, b), 5.0);
    assert_eq!(l.edge_gap(b, a), 5.0);
    assert_eq!(l.edge_gap(a, c), 0.0, "abutting");
    assert_eq!(l.edge_gap(a, a), 0.0, "self");
}

#[test]
fn l_ref_single_cell() {
    assert_eq!(layout(vec![0], vec![0], vec![2], vec![2]).l_ref(), 4.0);
    // Zero-area cells floor at 1 nm.
    assert_eq!(layout(vec![0], vec![0], vec![0], vec![5]).l_ref(), 1.0);
}

#[test]
fn debug_check_placed_allows_abutment() {
    let l = layout(vec![0, 2], vec![0, 0], vec![1, 1], vec![1, 1]);
    l.debug_check_placed("abut");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "overlap")]
fn debug_check_placed_rejects_overlap() {
    layout(vec![0, 1], vec![0, 0], vec![1, 1], vec![1, 1]).debug_check_placed("t");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Layout.y")]
fn debug_check_rejects_ragged_columns() {
    layout(vec![0, 1], vec![0], vec![1, 1], vec![1, 1]).debug_check("t");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "negative extent")]
fn debug_check_rejects_negative_extent() {
    layout(vec![0], vec![0], vec![-1], vec![1]).debug_check("t");
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "group 0")]
fn debug_check_rejects_dangling_group_member() {
    let mut l = layout(vec![0], vec![0], vec![1], vec![1]);
    l.groups = vec![vec![DeviceId(3)]];
    l.debug_check("t");
}

// ---------------------------------------------------------------- macro

fn pin(name: &str, at: Rect) -> Pin {
    Pin { name: name.into(), net: NetId(0), at, layer: LayerId(0) }
}

#[test]
fn align_bbox_lattice_below_one_reads_as_one() {
    let mut m = Macro { bbox: rect(5, 3, 95, 41), ..Default::default() };
    m.align_bbox(0);
    assert_eq!(m.bbox, rect(5, 3, 96, 42));
    let mut m = Macro { bbox: rect(5, 3, 95, 41), ..Default::default() };
    m.align_bbox(-7);
    assert_eq!(m.bbox, rect(5, 3, 96, 42));
}

#[test]
fn align_bbox_negative_corner_contains_original() {
    let mut m = Macro { bbox: rect(-15, -5, 10, 10), ..Default::default() };
    m.align_bbox(10);
    assert_eq!(m.bbox, rect(-20, -10, 20, 20));
    let mut z = Macro::default();
    z.align_bbox(10);
    assert_eq!(z.bbox, rect(0, 0, 0, 0), "empty stays empty");
}

#[test]
fn place_macro_past_the_layout_is_identity() {
    let m = Macro { bbox: rect(1, 2, 3, 4), shapes: vec![shape(1, rect(1, 2, 3, 4))], ..Default::default() };
    let l = layout(vec![100], vec![100], vec![2], vec![2]);
    assert_eq!(place_macro(&m, &l, 1), m);
    let placed = place_macros(&[m.clone(), m.clone()], &l);
    assert_eq!(placed.len(), 2);
    assert_eq!(placed[1], m, "guard-ring index unchanged");
    assert_eq!(placed[0].bbox, rect(98, 98, 3, 4));
}

#[test]
fn place_rect_without_orient_column_reads_r0() {
    let mut l = layout(vec![100], vec![50], vec![10], vec![5]);
    l.orient.clear();
    assert_eq!(place_rect(rect(0, 0, 20, 10), rect(5, 5, 2, 2), &l, 0), rect(95, 50, 2, 2));
}

#[test]
fn pin_shares_degenerate_inputs() {
    assert!(pin_shares(&Macro::default()).is_empty());
    let one = Macro { pins: vec![pin("d0:G", rect(0, 0, 10, 10))], ..Default::default() };
    assert_eq!(pin_shares(&one), vec![1.0], "a lone pin carries everything");
    // Units with phi = (0, 0) count for nothing: fallback 2/n.
    let m = Macro {
        pins: vec![pin("d0:D", rect(0, 0, 10, 10)), pin("d0:D", rect(100, 0, 10, 10))],
        units: vec![Unit { owner: 0, x: 50, y: 5, weight: 1, phi: (0, 0), sa: 0, sb: 0 }],
        ..Default::default()
    };
    assert_eq!(pin_shares(&m), vec![1.0, 1.0]);
}

/// Shares of one terminal's pins sum to 1 whenever a finger reaches them.
#[test]
fn pin_shares_of_a_terminal_sum_to_one() {
    let m = Macro {
        pins: vec![pin("d1:D", rect(-5, -5, 10, 10)), pin("d1:S", rect(195, -5, 10, 10)), pin("d1:D", rect(395, -5, 10, 10))],
        units: vec![
            Unit { owner: 1, x: 100, y: 0, weight: 1, phi: (-1, 0), sa: 0, sb: 0 },
            Unit { owner: 1, x: 300, y: 0, weight: 1, phi: (1, 0), sa: 0, sb: 0 },
        ],
        ..Default::default()
    };
    let s = pin_shares(&m);
    assert_eq!(s, vec![0.5, 1.0, 0.5]);
}

// ---------------------------------------------------------------- report

#[test]
fn report_lex_and_feasibility() {
    let r = Report::default();
    assert_eq!(r.lex(), (0, 0.0, 0.0));
    assert!(r.feasible());
    let r = Report {
        hard_violations: vec![],
        budget_violations: vec![Violation::from_residual("b", 0.5), Violation::from_residual("c", 0.25)],
        cost: 3.0,
    };
    assert_eq!(r.lex(), (0, 750.0, 3.0));
    assert!(!r.feasible());
    let hard = Report { hard_violations: vec![Violation { rule: "x".into(), margin: 0 }], ..Report::default() };
    assert!(hard.lex() > r.lex(), "one hard violation outranks any Θ");
}

#[test]
fn from_residual_units_and_rounding() {
    assert_eq!(Violation::from_residual("r", 0.5).margin, 500);
    assert_eq!(Violation::from_residual("r", 1e-9).margin, 1, "ceil: never rounds to 0");
    assert_eq!(Violation::from_residual("r", 0.0).margin, 0);
    assert_eq!(Violation::from_residual("r", -0.3).margin, 0, "within budget is not negative Θ");
    assert_eq!(Violation::from_residual("r", f64::NAN).margin, i64::MAX, "unknown is never a pass");
    assert_eq!(Violation::from_residual("r", f64::INFINITY).margin, i64::MAX);
    assert_eq!(Violation::from_residual(String::from("r"), 2.0).rule, "r");
}

#[test]
fn batch_rows_are_recognised_by_prefix() {
    assert!(Violation { rule: format!("{}analog", Violation::BATCH), margin: 1 }.is_batch_row());
    assert!(!Violation { rule: "analog batch:".into(), margin: 1 }.is_batch_row());
    assert!(!Violation { rule: String::new(), margin: 1 }.is_batch_row());
}

// ---------------------------------------------------------------- routes

const M1: u16 = 1;
const V1: u16 = 2;
const M2: u16 = 3;
const JOINS: [Join; 1] = [(LayerId(V1), LayerId(M1), LayerId(M2))];

#[test]
fn layers_meet_only_through_listed_cuts() {
    let r = rect(0, 0, 10, 10);
    let (m1, v1, m2) = (shape(M1, r), shape(V1, r), shape(M2, r));
    assert!(conductor_layers_meet(&m1, &m1, &JOINS));
    assert!(conductor_layers_meet(&m1, &v1, &JOINS) && conductor_layers_meet(&v1, &m1, &JOINS));
    assert!(conductor_layers_meet(&v1, &m2, &JOINS));
    assert!(!conductor_layers_meet(&m1, &m2, &JOINS), "metals do not meet without a cut");
    assert!(!conductor_layers_meet(&m1, &v1, &[]), "no joins, no meeting");
}

#[test]
fn open_components_counts_extra_pieces() {
    assert_eq!(open_components(&[], &JOINS), 0);
    assert_eq!(open_components(&[shape(M1, rect(0, 0, 5, 5))], &JOINS), 0);
    let apart = [shape(M1, rect(0, 0, 5, 5)), shape(M1, rect(6, 0, 5, 5))];
    assert_eq!(open_components(&apart, &JOINS), 1);
    let stacked = [shape(M1, rect(0, 0, 5, 5)), shape(M2, rect(0, 0, 5, 5))];
    assert_eq!(open_components(&stacked, &JOINS), 1, "overlap without a via is open");
    let via = [shape(M1, rect(0, 0, 5, 5)), shape(M2, rect(0, 0, 5, 5)), shape(V1, rect(1, 1, 2, 2))];
    assert_eq!(open_components(&via, &JOINS), 0);
    // Chain reached only through a later shape: a—c—b.
    let chain = [shape(M1, rect(0, 0, 5, 1)), shape(M1, rect(20, 0, 5, 1)), shape(M1, rect(5, 0, 15, 1))];
    assert_eq!(open_components(&chain, &JOINS), 0);
}

#[test]
fn routes_accessors_read_missing_rows_as_empty() {
    let r = Routes {
        wires: vec![vec![shape(M1, rect(0, 0, 100, 10)), shape(M1, rect(0, 0, 10, 300))]],
        cell: vec![vec![shape(M1, rect(0, 0, 1, 1))]],
        gates: vec![vec![GatePin { at: rect(0, 0, 1, 1), dev: 0, nm2: 9 }]],
        terms: vec![vec![Terminal { at: rect(0, 0, 1, 1), ua: Some(2.0) }]],
    };
    assert_eq!(r.length(NetId(0)), 400);
    assert_eq!(r.length(NetId(9)), 0);
    assert_eq!(r.shapes(NetId(1)).len(), 0);
    assert_eq!(r.cell_metal(NetId(0)).len(), 1);
    assert_eq!(r.cell_metal(NetId(1)).len(), 0);
    assert_eq!(r.gate_pins(NetId(0))[0].nm2, 9);
    assert!(r.gate_pins(NetId(4)).is_empty());
    assert_eq!(r.terminals(NetId(0))[0].ua, Some(2.0));
    assert!(r.terminals(NetId(u16::MAX)).is_empty());
    r.debug_check_joined("ok", &JOINS);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "degenerate")]
fn debug_check_joined_rejects_degenerate_shapes() {
    let r = Routes { wires: vec![vec![shape(M1, rect(0, 0, 0, 10))]], ..Routes::default() };
    r.debug_check_joined("t", &JOINS);
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "open")]
fn debug_check_joined_rejects_open_nets() {
    let r = Routes { wires: vec![vec![shape(M1, rect(0, 0, 5, 5)), shape(M1, rect(50, 0, 5, 5))]], ..Routes::default() };
    r.debug_check_joined("t", &JOINS);
}

// ---------------------------------------------------------------- thermal

#[test]
fn self_rise_is_linear_symmetric_and_finite() {
    assert_eq!(self_rise_mc(0, 1_000, 2_000, K_SI_W_PER_M_K), 0.0);
    let a = self_rise_mc(1_000, 1_000, 4_000, K_SI_W_PER_M_K);
    assert_eq!(a, self_rise_mc(1_000, 4_000, 1_000, K_SI_W_PER_M_K), "side order");
    assert!((self_rise_mc(3_000, 1_000, 4_000, K_SI_W_PER_M_K) - 3.0 * a).abs() <= 1e-3 * a);
    assert!(self_rise_mc(1_000, 0, 0, K_SI_W_PER_M_K).is_finite(), "sides floored at 1 nm");
    assert_eq!(rise_bound_mc(&[], &[], K_SI_W_PER_M_K), 0.0);
}

#[test]
fn rises_handle_empty_and_mismatched_power() {
    let l = layout(vec![], vec![], vec![], vec![]);
    assert!(rises_mc(&l, &[5]).is_empty());
    let l = layout(vec![0, 10_000], vec![0, 0], vec![500, 500], vec![500, 500]);
    let short = rises_mc(&l, &[1_000]);
    let padded = rises_mc(&l, &[1_000, 0, 77_777]);
    assert_eq!(short, padded, "missing = 0, extras ignored");
    assert!(short[0] > short[1] && short[1] > 0);
}

/// Translating every device leaves every rise unchanged.
#[test]
fn rises_are_translation_invariant() {
    let mut l = layout(vec![0, 7_000, -3_000], vec![0, 2_000, 9_000], vec![400, 300, 900], vec![200, 300, 100]);
    let p = [5_000, 2_000, 0];
    let t0 = rises_mc(&l, &p);
    for x in &mut l.x {
        *x += 1_234_567;
    }
    for y in &mut l.y {
        *y -= 765_432;
    }
    assert_eq!(rises_mc(&l, &p), t0);
}

/// Devices 4e9 nm apart: the difference does not fit i32, the rise is still
/// the point-source value, not a panic.
#[test]
fn far_apart_devices_do_not_overflow() {
    let l = layout(vec![-2_000_000_000, 2_000_000_000], vec![0, 0], vec![1, 1], vec![1, 1]);
    let t = rises_mc(&l, &[1_000_000, 0]);
    // 1 W / (2π·148·4 m) ≈ 0.27 mK → rounds to 0.
    assert_eq!(t[1], 0);
    let r = l.rise_at_point_mc(2_000_000_000, 0);
    assert!((r - 0.2689).abs() < 0.01, "{r}");
}

// ---------------------------------------------------------------- units

fn unit(owner: u8, x: i32, phi: (i8, i8), sa: i32, sb: i32) -> Unit {
    Unit { owner, x, y: 0, weight: 3, phi, sa, sb }
}

#[test]
fn unit_lib_empty_and_dropped_owners() {
    let lib = UnitLib::build(Vec::new(), &[], std::iter::empty());
    assert!(lib.is_empty());
    let l = layout(vec![0], vec![0], vec![1], vec![1]);
    assert_eq!(lib.placed(&l, 0).count(), 0);
    assert_eq!(lib.of_device(&l, DeviceId(0)).count(), 0);

    // Owner 5 is not a member: dropped.
    let units = [unit(5, 0, (1, 0), 0, 0)];
    let alts = [(rect(0, 0, 2, 2), &units[..])];
    let lib = UnitLib::build(vec![0], &[vec![DeviceId(0)]], std::iter::once(&alts[..]));
    assert!(lib.is_empty());
}

#[test]
fn unit_lib_selects_variant_turns_phi_and_computes_lod() {
    let v0 = [unit(0, 10, (1, 0), 1_000, 1_000)];
    let v1 = [unit(0, 30, (1, 0), 0, 500), unit(0, 50, (0, 1), 500, 250)];
    let alts = [(rect(0, 0, 100, 40), &v0[..]), (rect(0, 0, 100, 40), &v1[..])];
    let lib = UnitLib::build(vec![0, 0], &[vec![DeviceId(1)]], std::iter::once(&alts[..]));
    let mut l = layout(vec![1_000], vec![0], vec![50], vec![20]);
    let u: Vec<_> = lib.placed(&l, 0).collect();
    assert_eq!(u.len(), 1);
    assert_eq!((u[0].owner, u[0].x, u[0].y, u[0].weight), (DeviceId(1), 960, -20, 3));
    assert_eq!(u[0].lod, 2.0, "1e3/1000 + 1e3/1000 per µm");

    l.variant[0] = 1;
    let u: Vec<_> = lib.of_device(&l, DeviceId(1)).collect();
    assert_eq!(u.len(), 2);
    assert!(u[0].lod.is_nan(), "sa = 0: unknown");
    assert_eq!(u[1].lod, 6.0);
    assert!(lib.of_device(&l, DeviceId(0)).next().is_none(), "maps to cell 0 but owns nothing");
    assert!(lib.of_device(&l, DeviceId(9)).next().is_none(), "no cell");

    // Quarter turn: phi (1, 0) → (0, 1), and the variant column may be missing.
    l.variant.clear();
    l.orient[0] = Orient::R90;
    l.hw[0] = 20;
    l.hh[0] = 50;
    let u: Vec<_> = lib.placed(&l, 0).collect();
    assert_eq!(u[0].phi, (0, 1));
    // Local (10, 0) → (0, 10); the turned bbox's lower-left (−40, 0) lands on
    // (1000 − 20, 0 − 50), a shift of (1020, −50).
    assert_eq!((u[0].x, u[0].y), (1020, -40));
}

// ---------------------------------------------------------------- lanes

#[test]
fn min_max_edges() {
    assert_eq!(lanes::min_max(&[]), None);
    assert_eq!(lanes::min_max(&[-4]), Some((-4, -4)));
    assert_eq!(lanes::min_max(&[i32::MAX, i32::MIN, 0]), Some((i32::MIN, i32::MAX)));
    // Tail-only values must win over padding zeros.
    let v: Vec<i32> = (1..=17).collect();
    assert_eq!(lanes::min_max(&v), Some((1, 17)));
    let v: Vec<i32> = (-17..=-1).collect();
    assert_eq!(lanes::min_max(&v), Some((-17, -1)));
}

#[test]
fn overlap_area_excludes_touching_and_honours_clearance() {
    let (x, y, hw, hh) = ([10], [0], [5], [5]);
    let b = Boxes { x: &x, y: &y, hw: &hw, hh: &hh };
    let q = Box4 { x: 0, y: 0, hw: 5, hh: 5 };
    assert_eq!(lanes::overlap_area(b, q, 0), 0, "abutting boxes do not overlap");
    // Inflated by 2: ox = 7 + 5 − 10 = 2, oy = 7 + 5 − 0 = 12.
    assert_eq!(lanes::overlap_area(b, q, 2), 2 * 12);
    assert_eq!(lanes::overlap_area(b, q, -1), 0);
    let e = Boxes { x: &[], y: &[], hw: &[], hh: &[] };
    assert_eq!(lanes::overlap_area(e, q, 100), 0);
}

#[test]
fn changed_reports_nothing_for_equal_tables() {
    let v: Vec<i32> = (0..37).collect();
    let b = Boxes { x: &v, y: &v, hw: &v, hh: &v };
    let mut out = vec![99];
    lanes::changed(b, b, &mut out);
    assert_eq!(out, vec![99], "appends only");
    let mut hh = v.clone();
    hh[36] = -1;
    lanes::changed(Boxes { hh: &hh, ..b }, b, &mut out);
    assert_eq!(out, vec![99, 36]);
}

#[test]
#[should_panic(expected = "ragged")]
fn changed_panics_on_ragged_columns() {
    let b = Boxes { x: &[1, 2], y: &[1], hw: &[1, 2], hh: &[1, 2] };
    lanes::changed(b, b, &mut Vec::new());
}
