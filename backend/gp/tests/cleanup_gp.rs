//! Corner-case tests for gp's public surface: `mechanics` (RNG, nets, HPWL,
//! overlap, objectives, report, canvas helpers), `spacing` (role table, gaps,
//! halos, profiles, orients) and the crate root (`Prices`, `net_weights`,
//! `PlaceRules`, `place`). Oracles are the doc contracts, hand-worked values
//! and metamorphic relations (symmetry, translation, determinism).

use analog::metadata::{NetClass, NetClassification};
use analog::{Requirements, RuleBatch};
use gp::mechanics::{
    analog_cost, analog_phi, analog_theta, analog_violations, augmented_cost, canvas_side, choose_variants,
    clamp_to_die, encroach, encroachment, half_extents, hpwl, initial_layout, pex, report, snap, variant_extents,
    Nets, SplitMix64,
};
use gp::spacing::{
    self, oriented, oriented_faces, profile, Edge, Face, Gap, Profile, Profiles, SpacingTable, Src, DECK_ROLE, N,
    ORIENTS, ROLES,
};
use gp::{net_weights, place, GpInput, PlaceRules, Prices, VariantSpace, RAIL_MIN, RAIL_UNKNOWN};
use pnr_core::{LayerId, Layout, Macro, MatchClass, NetId, Orient, Pin, Process, Rect, Shape};

// ---------------------------------------------------------------- fixtures

fn lay(x: &[i32], y: &[i32], hw: &[i32], hh: &[i32]) -> Layout {
    let n = x.len();
    Layout {
        x: x.to_vec(),
        y: y.to_vec(),
        hw: hw.to_vec(),
        hh: hh.to_vec(),
        variant: vec![0; n],
        axis: vec![0],
        branch: vec![false; n],
        groups: Vec::new(),
        orient: vec![Orient::R0; n],
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
        units: Default::default(),
    }
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

fn pin(net: u16, at: Rect) -> Pin {
    Pin { name: String::new(), net: NetId(net), at, layer: LayerId(0) }
}

/// A 100 × 100 macro at the origin with point pins `(net, x, y)`.
fn cell(pins: &[(u16, i32, i32)]) -> Macro {
    Macro { bbox: rect(0, 0, 100, 100), pins: pins.iter().map(|&(n, x, y)| pin(n, rect(x, y, 0, 0))).collect(), ..Default::default() }
}

type F32 = fn(&Layout) -> f32;
type F64 = fn(&Layout) -> f64;

/// A rule batch whose every answer is a plain function of the layout.
struct Batch {
    kind: &'static str,
    cost: F32,
    viol: fn(&Layout) -> u32,
    res: F64,
    crit: F32,
    usage: fn(&Layout) -> Option<f32>,
}

impl Batch {
    fn new(kind: &'static str) -> Self {
        Batch { kind, cost: |_| 0.0, viol: |_| 0, res: |_| 0.0, crit: |_| 1.0, usage: |_| None }
    }
}

impl RuleBatch<Layout> for Batch {
    fn cost(&self, l: &Layout) -> f32 {
        (self.cost)(l)
    }
    fn violations(&self, l: &Layout) -> u32 {
        (self.viol)(l)
    }
    fn residual(&self, l: &Layout) -> f64 {
        (self.res)(l)
    }
    fn kind(&self) -> &'static str {
        self.kind
    }
    fn criticality(&self, l: &Layout) -> f32 {
        (self.crit)(l)
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        (self.usage)(l)
    }
}

fn reqs(hard: Vec<Batch>, budget: Vec<Batch>, cost: Vec<Batch>) -> Requirements<Layout> {
    let boxed = |v: Vec<Batch>| -> Vec<Box<dyn RuleBatch<Layout>>> {
        v.into_iter().map(|b| Box::new(b) as Box<dyn RuleBatch<Layout>>).collect()
    };
    Requirements { hard: boxed(hard), budget: boxed(budget), cost: boxed(cost) }
}

fn role(name: &str) -> usize {
    ROLES.iter().position(|&r| r == name).unwrap()
}

// ---------------------------------------------------------------- SplitMix64

#[test]
fn splitmix_matches_the_reference_sequence() {
    // Vigna's splitmix64.c from state 0.
    let mut r = SplitMix64::new(0);
    assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn splitmix_is_deterministic_and_unit_ranges_hold() {
    let (mut a, mut b) = (SplitMix64::new(42), SplitMix64::new(42));
    for _ in 0..1_000 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
    let mut r = SplitMix64::new(7);
    for _ in 0..10_000 {
        let f = r.f64();
        assert!((0.0..1.0).contains(&f), "{f}");
        let g = r.f32();
        assert!((0.0..1.0).contains(&g), "{g}");
        let c = r.centered(3.0);
        assert!((-3.0..3.0).contains(&c), "{c}");
        assert!(r.below(7) < 7);
        assert_eq!(r.below(1), 0);
    }
    assert_eq!(r.centered(0.0), 0.0);
}

#[test]
fn below_zero_is_zero_and_still_consumes_a_draw() {
    let (mut a, mut b) = (SplitMix64::new(3), SplitMix64::new(3));
    assert_eq!(a.below(0), 0);
    b.next_u64();
    assert_eq!(a.next_u64(), b.next_u64());
}

// ---------------------------------------------------------------- Nets

#[test]
fn nets_from_no_macros_or_no_pins_are_empty() {
    let n = Nets::from_macros(&[]);
    assert_eq!(n.count(), 0);
    assert_eq!(hpwl(&n, &lay(&[], &[], &[], &[])), 0.0);
    let n = Nets::from_macros(&[cell(&[]), cell(&[])]);
    assert_eq!(n.count(), 0);
    assert!(n.cell_nets(2).iter().all(Vec::is_empty));
}

#[test]
fn single_device_nets_are_dropped_and_rows_follow_net_order() {
    // net 0: one device only (two pins); net 3: devices 1 and 0; net 5: all three.
    let macros = [
        cell(&[(0, 10, 10), (0, 20, 20), (3, 50, 50), (5, 0, 0)]),
        cell(&[(3, 50, 50), (5, 100, 100)]),
        cell(&[(5, 50, 0)]),
    ];
    let n = Nets::from_macros(&macros);
    assert_eq!(n.count(), 2);
    assert_eq!(n.row(0), [0, 1]);
    assert_eq!(n.row(1), [0, 1, 2]);
    assert_eq!(n.span(0).len(), 2);
    assert_eq!(n.cell_nets(3), vec![vec![0, 1], vec![0, 1], vec![1]]);
    assert_eq!((n.weight(0), n.weight(1)), (1.0, 1.0));
}

#[test]
fn several_pins_of_one_device_on_one_net_give_their_centroid() {
    let macros = [cell(&[(1, 0, 0), (1, 100, 50)]), cell(&[(1, 50, 50)])];
    let n = Nets::from_macros(&macros);
    let l = lay(&[1_000, 2_000], &[0, 0], &[50, 50], &[50, 50]);
    // Centroid (50, 25) is offset (0, -25) from the centre (50, 50).
    assert_eq!(n.pin(n.span(0).start, &l), (1_000, -25));
    assert_eq!(n.pin(n.span(0).start + 1, &l), (2_000, 0));
}

#[test]
fn pin_turns_its_offset_by_orient_and_short_orient_reads_r0() {
    let macros = [cell(&[(0, 100, 50)]), cell(&[(0, 50, 50)])];
    let n = Nets::from_macros(&macros);
    let mut l = lay(&[0, 0], &[0, 0], &[50, 50], &[50, 50]);
    let k = n.span(0).start;
    assert_eq!(n.pin(k, &l), (50, 0));
    l.orient[0] = Orient::R90;
    assert_eq!(n.pin(k, &l), (0, 50));
    l.orient[0] = Orient::Mx180;
    assert_eq!(n.pin(k, &l), (-50, 0));
    l.orient.clear();
    assert_eq!(n.pin(k, &l), (50, 0));
}

#[test]
fn weigh_reads_by_net_id_and_missing_is_one() {
    let macros = [cell(&[(2, 0, 0), (4, 0, 0)]), cell(&[(2, 0, 0), (4, 0, 0)])];
    let n = Nets::from_macros(&macros).weigh(&[9.0, 9.0, 3.0]);
    assert_eq!((n.weight(0), n.weight(1)), (3.0, 1.0));
    let l = lay(&[0, 10], &[0, 20], &[50, 50], &[50, 50]);
    assert_eq!(hpwl(&n, &l), 3.0 * 30.0 + 30.0);
}

#[test]
fn pin_bbox_and_hpwl_agree() {
    let macros = [cell(&[(0, 50, 50)]), cell(&[(0, 50, 50)]), cell(&[(0, 50, 50)])];
    let n = Nets::from_macros(&macros);
    let l = lay(&[0, 300, -100], &[50, -20, 70], &[50; 3], &[50; 3]);
    assert_eq!(n.pin_bbox(0, &l), (-100, 300, -20, 70));
    assert_eq!(hpwl(&n, &l), 400.0 + 90.0);
}

#[test]
fn hpwl_is_translation_invariant() {
    let macros = [cell(&[(0, 0, 0), (1, 100, 100)]), cell(&[(0, 30, 70), (1, 0, 0)])];
    let n = Nets::from_macros(&macros);
    let a = lay(&[0, 500], &[0, -300], &[50, 50], &[50, 50]);
    let b = lay(&[7_000, 7_500], &[-9_000, -9_300], &[50, 50], &[50, 50]);
    assert_eq!(hpwl(&n, &a), hpwl(&n, &b));
}

#[test]
fn hpwl_is_exact_for_extreme_coordinates() {
    let macros = [cell(&[(0, 50, 50)]), cell(&[(0, 50, 50)])];
    let n = Nets::from_macros(&macros);
    let l = lay(&[-2_000_000_000, 2_000_000_000], &[0, 0], &[50, 50], &[50, 50]);
    assert_eq!(hpwl(&n, &l), 4_000_000_000.0);
}

#[test]
fn reshape_cell_repoints_offsets_and_skips_foreign_rows() {
    let macros = [cell(&[(0, 50, 50), (1, 50, 50)]), cell(&[(0, 50, 50)]), cell(&[(1, 50, 50)])];
    let mut n = Nets::from_macros(&macros);
    let l = lay(&[0, 0, 0], &[0, 0, 0], &[50; 3], &[50; 3]);
    let wide = Macro { bbox: rect(0, 0, 200, 100), pins: vec![pin(0, rect(200, 50, 0, 0)), pin(1, rect(0, 50, 0, 0))], ..Default::default() };
    // Row 1 is net 1 (cells 0 and 2): passing it for cell 1 must be a no-op.
    n.reshape_cell(1, &[1], &wide);
    assert_eq!(n.pin(n.span(1).start, &l), (0, 0));
    n.reshape_cell(0, &[0, 1], &wide);
    assert_eq!(n.pin(n.span(0).start, &l), (100, 0));
    assert_eq!(n.pin(n.span(1).start, &l), (-100, 0));
}

// ---------------------------------------------------------------- overlap

#[test]
fn encroach_cases() {
    let l = lay(&[0, 100, 300], &[0, 0, 0], &[50, 50, 50], &[50, 50, 50]);
    assert_eq!(encroach(&l, 0, 1, 0), 0.0, "abutting is not overlap");
    assert_eq!(encroach(&l, 0, 1, 10), 10.0 * 110.0);
    assert_eq!(encroach(&l, 0, 2, 0), 0.0);
    let o = lay(&[0, 60], &[0, 30], &[50, 50], &[50, 50]);
    assert_eq!(encroach(&o, 0, 1, 0), 40.0 * 70.0);
    assert_eq!(encroach(&o, 0, 1, 0), encroach(&o, 1, 0, 0), "symmetric");
    assert_eq!(encroachment(&o, 0), 2_800.0);
}

#[test]
fn encroachment_of_fewer_than_two_cells_is_positive_zero() {
    for l in [lay(&[], &[], &[], &[]), lay(&[0], &[0], &[5], &[5])] {
        let e = encroachment(&l, 100);
        assert!(e == 0.0 && e.is_sign_positive());
    }
}

// ---------------------------------------------------------------- objectives

#[test]
fn empty_requirements_score_zero() {
    let r = Requirements::<Layout>::default();
    let l = lay(&[0], &[0], &[1], &[1]);
    let p = Prices::new();
    assert_eq!(analog_cost(&r, &l, &p), 0.0);
    assert_eq!(augmented_cost(&r, &l, &p), 0.0);
    assert_eq!(analog_phi(&r, &l), (0, 0.0));
    assert_eq!(analog_theta(&r, &l), 0.0);
    assert_eq!(analog_violations(&r, &l), 0);
}

#[test]
fn cost_terms_weight_criticality_and_price() {
    let mut c = Batch::new("c");
    c.cost = |_| 3.0;
    c.crit = |_| 0.5;
    let mut b = Batch::new("b");
    b.res = |_| 2.0;
    let r = reqs(Vec::new(), vec![b], vec![c]);
    let l = lay(&[0], &[0], &[1], &[1]);
    let mut p = Prices::new();
    p.bind(&r);
    assert_eq!(analog_cost(&r, &l, &p), 1.5, "an unpriced budget adds nothing");
    assert_eq!(augmented_cost(&r, &l, &p), 1.5);
    p.settle(&r, &l); // λ = −ρ·g = −0.25·2
    let (w, rho) = (p.weight_of(0), p.rho_of(0));
    assert_eq!((w, rho), (0.5, 0.25));
    assert_eq!(analog_cost(&r, &l, &p), 1.5 + 0.5 * 2.0);
    assert_eq!(augmented_cost(&r, &l, &p), 1.5 + 0.5 * 2.0 + 0.5 * 0.25 * 4.0);
    assert_eq!(analog_theta(&r, &l), 2.0);
}

#[test]
fn augmented_cost_clamps_a_negative_residual_but_analog_cost_does_not() {
    let mut b = Batch::new("b");
    b.res = |l| f64::from(l.x[0]);
    let r = reqs(Vec::new(), vec![b], Vec::new());
    let mut l = lay(&[1], &[0], &[1], &[1]);
    let mut p = Prices::new();
    p.settle(&r, &l);
    assert!(p.weight_of(0) > 0.0);
    l.x[0] = -4;
    assert_eq!(augmented_cost(&r, &l, &p), 0.0);
    assert!(analog_cost(&r, &l, &p) < 0.0);
}

#[test]
fn phi_counts_only_violating_hard_batches() {
    let mut v = Batch::new("v");
    v.viol = |_| 2;
    v.res = |_| 0.75;
    let mut ok = Batch::new("ok");
    ok.res = |_| 9.0; // residual without a violation is not Φ
    let r = reqs(vec![v, ok], Vec::new(), Vec::new());
    let l = lay(&[0], &[0], &[1], &[1]);
    assert_eq!(analog_phi(&r, &l), (1, 0.75));
    assert_eq!(analog_violations(&r, &l), 2);
}

#[test]
fn pex_is_hpwl_over_l_ref_plus_analog_cost() {
    let macros = [cell(&[(0, 50, 50)]), cell(&[(0, 50, 50)])];
    let n = Nets::from_macros(&macros);
    let l = lay(&[0, 400], &[0, 0], &[50, 50], &[50, 50]);
    let mut c = Batch::new("c");
    c.cost = |_| 2.0;
    let r = reqs(Vec::new(), Vec::new(), vec![c]);
    let want = 400.0 / f64::from(l.l_ref()) + 2.0;
    assert!((pex(&n, &r, &l, &Prices::new()) - want).abs() < 1e-9);
}

#[test]
fn report_lists_hard_budget_overlap_and_clearance() {
    let mut h = Batch::new("h");
    h.viol = |_| 1;
    h.res = |_| 0.5;
    let mut b = Batch::new("b");
    b.res = |_| 0.25;
    let mut slack = Batch::new("s");
    slack.res = |_| -1.0;
    let r = reqs(vec![h], vec![b, slack], Vec::new());
    // Cells overlap by 20 × 100; a clearance of 30 inflates that to 50 × 130.
    let l = lay(&[0, 80], &[0, 0], &[50, 50], &[50, 50]);
    let rules = PlaceRules::uniform(5, 30);
    let n = Nets::from_macros(&[]);
    let rep = report(&n, &r, &l, &Prices::new(), &rules);
    let names: Vec<&str> = rep.hard_violations.iter().map(|v| v.rule.as_str()).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names[0].contains("analog hard 0"));
    assert_eq!((names[1], rep.hard_violations[1].margin), ("device overlap", 2_000));
    assert_eq!((names[2], rep.hard_violations[2].margin), ("clearance encroachment", 50 * 130 - 2_000));
    assert_eq!(rep.budget_violations.len(), 1, "only the positive residual");
    assert!(rep.budget_violations[0].rule.contains("analog budget 0"));

    let clean = lay(&[0, 500], &[0, 0], &[50, 50], &[50, 50]);
    let rep = report(&n, &Requirements::default(), &clean, &Prices::new(), &rules);
    assert!(rep.hard_violations.is_empty() && rep.budget_violations.is_empty());
    assert_eq!(rep.cost, 0.0);
}

// ---------------------------------------------------------------- canvas helpers

#[test]
fn extents_truncate_odd_sizes() {
    let m = Macro { bbox: rect(3, 4, 101, 7), ..Default::default() };
    assert_eq!(variant_extents(&m), (50, 3));
    assert_eq!(half_extents(&[m.clone(), Macro::default()]), (vec![50, 0], vec![3, 0]));
    assert_eq!(half_extents(&[]), (vec![], vec![]));
}

#[test]
fn choose_variants_falls_back_on_every_short_table() {
    let base = |w| Macro { bbox: rect(0, 0, w, 10), ..Default::default() };
    let macros = [base(10), base(20), base(30)];
    let spaces = [VariantSpace { alternatives: vec![base(11), base(12)] }, VariantSpace { alternatives: vec![] }];
    let w = |v: Vec<Macro>| v.iter().map(|m| m.bbox.w).collect::<Vec<_>>();
    assert_eq!(w(choose_variants(&macros, &spaces, &[1, 0, 0])), [12, 20, 30]);
    assert_eq!(w(choose_variants(&macros, &spaces, &[7, 3, 0])), [10, 20, 30], "out-of-range alternative");
    assert_eq!(w(choose_variants(&macros, &spaces, &[])), [10, 20, 30], "short assignment");
    assert!(choose_variants(&[], &spaces, &[1]).is_empty());
}

#[test]
fn canvas_side_cases() {
    assert_eq!(canvas_side(&[], &[], 0.4, 10), 1_000, "no cells: 1000 nm");
    assert_eq!(canvas_side(&[], &[], 0.4, 0), 1_000, "grid 0 reads 1");
    assert_eq!(canvas_side(&[], &[], 0.4, 300), 1_200);
    // One 100 × 100 cell at 100% → clamp 0.95 → ceil(√(10000/0.95)) = 103 > 100.
    assert_eq!(canvas_side(&[50], &[50], 1.0, 1), 103);
    // At 0.01 → clamp 0.05 → √200000 = 447.2 → 448.
    assert_eq!(canvas_side(&[50], &[50], 0.01, 1), 448);
    // A long thin cell: the largest side dominates the area term.
    assert_eq!(canvas_side(&[5_000], &[1], 0.5, 10), 10_000);
    // Zero-area cells still give a die of at least 1.
    assert_eq!(canvas_side(&[0], &[0], 0.4, 1), 1);
    // Wide extents: 2·hw near i32::MAX must not overflow.
    assert_eq!(canvas_side(&[1_000_000_000], &[1], 0.95, 10), 2_000_000_000);
    let s = canvas_side(&[37, 81], &[12, 400], 0.4, 7);
    assert_eq!(s % 7, 0);
}

#[test]
fn initial_layout_is_a_jittered_pile() {
    let macros = vec![Macro { bbox: rect(0, 0, 40, 60), ..Default::default() }; 5];
    let side = 10_000;
    let a = initial_layout(&macros, vec![1; 5], side, 0, &mut SplitMix64::new(9));
    let b = initial_layout(&macros, vec![1; 5], side, 0, &mut SplitMix64::new(9));
    assert_eq!((a.x.clone(), a.y.clone()), (b.x, b.y), "seed-deterministic");
    assert_eq!(a.axis, vec![5_000], "n_axes 0 still gets one axis");
    assert_eq!((a.hw[0], a.hh[0]), (20, 30));
    assert_eq!(a.variant, vec![1; 5]);
    assert_eq!(a.groups.len(), 5);
    assert!(a.groups.iter().enumerate().all(|(i, g)| g == &[pnr_core::DeviceId(i as u16)]));
    for i in 0..5 {
        assert!((a.x[i] - 5_000).abs() <= 1_500 && (a.y[i] - 5_000).abs() <= 1_500);
    }
    assert!(a.branch.iter().all(|&b| !b) && a.power_uw.iter().all(|&p| p == 0));
    let e = initial_layout(&[], Vec::new(), side, 3, &mut SplitMix64::new(1));
    assert!(e.x.is_empty() && e.axis.len() == 3);
}

#[test]
fn clamp_to_die_cases() {
    assert_eq!(clamp_to_die(500, 50, 1_000), 500);
    assert_eq!(clamp_to_die(10, 50, 1_000), 50);
    assert_eq!(clamp_to_die(990, 50, 1_000), 950);
    assert_eq!(clamp_to_die(-7, 600, 1_000), 600, "too wide sits at half");
    assert_eq!(clamp_to_die(9_999, 600, 1_000), 600);
}

#[test]
fn snap_rounds_to_nearest_ties_away_from_zero() {
    assert_eq!(snap(14, 10), 10);
    assert_eq!(snap(15, 10), 20);
    assert_eq!(snap(-14, 10), -10);
    assert_eq!(snap(-15, 10), -20);
    assert_eq!(snap(0, 10), 0);
    assert_eq!(snap(17, 0), 17, "grid 0 reads 1");
    assert_eq!(snap(17, -5), 17, "negative grid reads 1");
}

#[test]
fn snap_is_exact_past_f32_precision() {
    assert_eq!(snap(16_777_217, 1), 16_777_217);
    assert_eq!(snap(2_000_000_005, 10), 2_000_000_010);
    assert_eq!(snap(-2_000_000_004, 10), -2_000_000_000);
    let top = snap(i32::MAX, 10);
    assert!(i64::from(i32::MAX) - i64::from(top) < 10, "saturates near the top: {top}");
    let bottom = snap(i32::MIN, 10);
    assert!(i64::from(bottom) - i64::from(i32::MIN) < 10, "{bottom}");
}

// ---------------------------------------------------------------- spacing: basics

#[test]
fn faces_and_orients() {
    for f in [Face::L, Face::B, Face::R, Face::T] {
        assert_eq!(f.opposite().opposite(), f);
        assert_ne!(f.opposite(), f);
    }
    assert_eq!(Face::L.opposite(), Face::R);
    assert_eq!(Face::B.opposite(), Face::T);
    for (i, o) in ORIENTS.iter().enumerate() {
        assert_eq!(*o as usize, i);
    }
    assert_eq!(ROLES.len(), N);
    assert_eq!(DECK_ROLE.len(), N);
    assert_eq!(ROLES[N - 1], "other");
}

#[test]
fn edge_put_keeps_the_minimum_and_clamps_at_zero() {
    let mut e = Edge::default();
    assert_eq!(e.present, 0);
    assert!(e.inset.iter().all(|&i| i == i32::MAX));
    e.put(3, 50);
    e.put(3, 80);
    assert_eq!((e.inset[3], e.present), (50, 1 << 3));
    e.put(3, -20);
    assert_eq!(e.inset[3], 0);
    e.put(N - 1, 7);
    assert_eq!(e.present, (1 << 3) | (1 << (N - 1)));
}

/// A deck with n-well, diff, tap and poly layers and a few spacings.
struct Deck;
impl Process for Deck {
    fn layer(&self, role: &str) -> Option<LayerId> {
        match role {
            "nwell" => Some(LayerId(1)),
            "diff" => Some(LayerId(2)),
            "tap" => Some(LayerId(3)),
            "poly" => Some(LayerId(4)),
            _ => None,
        }
    }
    fn rule(&self, _: &str, default: i32) -> i32 {
        default
    }
    fn grid(&self) -> i32 {
        5
    }
    fn space(&self, role: &str) -> Option<i32> {
        match role {
            "nwell" => Some(1_270),
            "poly" => Some(210),
            "diff" => Some(270),
            _ => None,
        }
    }
    fn space_between(&self, a: &str, b: &str) -> Option<i32> {
        match (a, b) {
            ("diff", "nwell") => Some(340),
            ("poly", "diff") => Some(75),
            _ => None,
        }
    }
}

#[test]
fn spacing_table_reads_deck_markers_fallback_and_other() {
    let t = SpacingTable::new(&Deck, &[], 999, 5);
    let at = |a: &str, b: &str| (t.rule[role(a)][role(b)], t.src[role(a)][role(b)]);
    assert_eq!(at("nwell", "nwell"), (1_270, Src::Deck));
    assert_eq!(at("diff_in", "diff_out"), (270, Src::Deck), "one deck role, both sides of the well");
    assert_eq!(at("diff_out", "nwell"), (340, Src::Deck));
    assert_eq!(at("nwell", "diff_in"), (340, Src::Deck), "either order");
    assert_eq!(at("diff_in", "poly"), (75, Src::Deck));
    assert_eq!(at("tap_in", "tap_out"), (999, Src::Fallback), "same role, no deck value");
    assert_eq!(at("diom", "diom"), (0, Src::NoRule), "marker");
    assert_eq!(at("pnp", "pnp"), (0, Src::NoRule), "marker");
    assert_eq!(at("li", "met1"), (0, Src::NoRule), "cross-role, no deck value");
    assert_eq!(at("other", "other"), (999, Src::Fallback));
    assert_eq!(at("poly", "other"), (999, Src::Fallback));
    for i in 0..N {
        for j in 0..N {
            assert_eq!(t.rule[i][j], t.rule[j][i], "symmetric {i},{j}");
        }
    }
    assert_eq!((t.wpe, t.foreign_poly), ([0; 3], [0; 3]));
}

#[test]
fn sidecar_raises_never_lowers_and_expands_diff_and_tap() {
    let side = [("diff".to_string(), "poly".to_string(), 500), ("nwell".into(), "nwell".into(), 10), ("tap".into(), "li".into(), 33)];
    let t = SpacingTable::new(&Deck, &side, 999, 5);
    let at = |a: &str, b: &str| (t.rule[role(a)][role(b)], t.src[role(a)][role(b)]);
    assert_eq!(at("diff_in", "poly"), (500, Src::Sidecar));
    assert_eq!(at("poly", "diff_out"), (500, Src::Sidecar));
    assert_eq!(at("nwell", "nwell"), (1_270, Src::Deck), "smaller sidecar loses");
    assert_eq!(at("tap_out", "li"), (33, Src::Sidecar));
    assert_eq!(at("li", "tap_in"), (33, Src::Sidecar));
}

#[test]
#[should_panic(expected = "unknown role")]
fn sidecar_typo_panics() {
    let _ = SpacingTable::new(&Deck, &[("polly".into(), "poly".into(), 1)], 0, 5);
}

#[test]
fn uniform_table_and_max_gap() {
    let t = SpacingTable::uniform(1_271, 5);
    assert!(t.rule.iter().flatten().all(|&v| v == 1_271));
    assert!(t.src.iter().flatten().all(|&s| s == Src::Fallback));
    assert_eq!(t.max_gap(), 1_275, "lattice-rounded");
    assert_eq!(SpacingTable::uniform(0, 5).max_gap(), 0);
    assert_eq!(SpacingTable::uniform(7, 0).max_gap(), 7, "lattice 0 reads 1");
    let k = SpacingTable { wpe: [0, 0, 4_001], ..SpacingTable::uniform(100, 10) };
    assert_eq!(k.max_gap(), 4_010, "keep-outs count");
    // A fallback at the top of the range must not overflow the rounding.
    let big = SpacingTable::uniform(i32::MAX, 10).max_gap();
    assert!(big >= i32::MAX - 9 && big % 10 == 0 || big == i32::MAX, "{big}");
}

// ---------------------------------------------------------------- spacing: gaps

fn face(f: Face, roles: &[(usize, i32)]) -> Profile {
    let mut p = Profile::default();
    for &(r, i) in roles {
        p.edge[f as usize].put(r, i);
    }
    p
}

fn table(rules: &[(&str, &str, i32)], lattice: i32) -> SpacingTable {
    let mut t = SpacingTable { lattice, ..SpacingTable::uniform(0, lattice) };
    for &(a, b, v) in rules {
        t.rule[role(a)][role(b)] = v;
        t.rule[role(b)][role(a)] = v;
    }
    t
}

#[test]
fn gap_of_empty_faces_is_zero_and_abuts() {
    let t = table(&[("poly", "poly", 200)], 5);
    let g = t.gap(&Profile::default(), Face::R, &Profile::default());
    assert_eq!(g, Gap { abut: true, min: 0 });
    // Roles on the non-facing faces do not count.
    let a = face(Face::L, &[(role("poly"), 0)]);
    let b = face(Face::R, &[(role("poly"), 0)]);
    assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: true, min: 0 });
}

#[test]
fn gap_rounds_up_to_the_lattice_and_floors_at_zero() {
    let t = table(&[("poly", "poly", 200)], 5);
    let a = face(Face::R, &[(role("poly"), 29)]);
    let b = face(Face::L, &[(role("poly"), 30)]);
    assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: false, min: 145 });
    let far = face(Face::L, &[(role("poly"), 500)]);
    assert_eq!(t.gap(&a, Face::R, &far), Gap { abut: true, min: 0 }, "insets already cover the rule");
    let t0 = table(&[("poly", "poly", 200)], 0);
    assert_eq!(t0.gap(&a, Face::R, &b).min, 141, "lattice 0 reads 1");
}

#[test]
fn gap_is_symmetric_across_the_pair() {
    let t = table(&[("poly", "diff_out", 300), ("nsdm", "nsdm", 380), ("met1", "met1", 140)], 5);
    let a = Profile {
        edge: [
            face(Face::L, &[(role("met1"), 3)]).edge[0],
            Edge::default(),
            face(Face::R, &[(role("poly"), 40), (role("nsdm"), 0)]).edge[2],
            Edge::default(),
        ],
        ..Profile::default()
    };
    let b = Profile {
        edge: [
            face(Face::L, &[(role("diff_out"), 12), (role("nsdm"), 0)]).edge[0],
            Edge::default(),
            face(Face::R, &[(role("met1"), 1)]).edge[2],
            Edge::default(),
        ],
        ..Profile::default()
    };
    assert_eq!(t.gap(&a, Face::R, &b), t.gap(&b, Face::L, &a));
    assert_eq!(t.gap(&a, Face::L, &b), t.gap(&b, Face::R, &a));
    assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: false, min: 380 });
    assert_eq!(t.gap(&b, Face::R, &a), Gap { abut: false, min: 140 });
}

#[test]
fn gap_with_huge_insets_does_not_overflow() {
    let t = table(&[("poly", "poly", 200)], 5);
    let a = face(Face::R, &[(role("poly"), i32::MAX - 1)]);
    let b = face(Face::L, &[(role("poly"), i32::MAX - 1)]);
    assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: true, min: 0 });
    let k = SpacingTable { wpe: [5_000; 3], ..table(&[], 5) };
    let m = Profile { matched: Some(MatchClass::Minimal), ..face(Face::R, &[(role("diff_out"), i32::MAX)]) };
    let w = face(Face::L, &[(role("nwell"), i32::MAX)]);
    assert_eq!(k.gap(&m, Face::R, &w), Gap { abut: true, min: 0 });
}

#[test]
fn nwell_merges_only_on_one_named_bulk() {
    let t = table(&[("nwell", "nwell", 1_270)], 5);
    let w = |net: Option<u16>| Profile { well_net: net.map(NetId), ..face(Face::R, &[(role("nwell"), 0)]) };
    let wl = |net: Option<u16>| Profile { well_net: net.map(NetId), ..face(Face::L, &[(role("nwell"), 0)]) };
    assert!(t.gap(&w(Some(4)), Face::R, &wl(Some(4))).abut);
    assert!(!t.gap(&w(Some(4)), Face::R, &wl(Some(5))).abut);
    assert!(!t.gap(&w(None), Face::R, &wl(None)).abut, "two unknown bulks never merge");
    // An implant merges only when both sit on the face.
    let t = table(&[("psdm", "psdm", 380)], 5);
    let a = face(Face::R, &[(role("psdm"), 0)]);
    assert!(t.gap(&a, Face::R, &face(Face::L, &[(role("psdm"), 0)])).abut);
    assert!(!t.gap(&a, Face::R, &face(Face::L, &[(role("psdm"), 5)])).abut);
}

#[test]
fn keep_outs_apply_both_ways_with_the_matched_class() {
    let k = SpacingTable { wpe: [1_000, 2_000, 3_000], foreign_poly: spacing::FOREIGN_POLY_NM, ..table(&[], 5) };
    let m = |c| Profile { matched: Some(c), ..face(Face::R, &[(role("diff_out"), 100)]) };
    let well = face(Face::L, &[(role("nwell"), 0)]);
    assert_eq!(k.gap(&m(MatchClass::Minimal), Face::R, &well).min, 900);
    assert_eq!(k.gap(&m(MatchClass::Exceptional), Face::R, &well).min, 2_900);
    // Mirror: the unmatched cell on the left of the matched one.
    let mr = |c| Profile { matched: Some(c), ..face(Face::L, &[(role("diff_out"), 100)]) };
    let wr = face(Face::R, &[(role("nwell"), 0)]);
    assert_eq!(k.gap(&wr, Face::R, &mr(MatchClass::Exceptional)).min, 2_900);
    // Minimal has no foreign-poly keep-out.
    let poly = face(Face::L, &[(role("poly"), 0)]);
    assert_eq!(k.gap(&m(MatchClass::Minimal), Face::R, &poly), Gap { abut: true, min: 0 });
    assert_eq!(k.gap(&m(MatchClass::Exceptional), Face::R, &poly).min, 4_900);
}

#[test]
fn halo_cases() {
    let t = table(&[("poly", "poly", 200), ("poly", "met1", 600)], 5);
    assert_eq!(t.halo(&Profile::default()), 0);
    assert_eq!(t.halo(&face(Face::T, &[(role("poly"), 50)])), 550);
    assert_eq!(t.halo(&face(Face::T, &[(role("poly"), 900)])), 0, "deep inset owes nothing");
    let o = SpacingTable { fallback: 1_000, ..t.clone() };
    assert_eq!(o.halo(&face(Face::B, &[(role("other"), 100)])), 900);
    let k = SpacingTable { wpe: [0, 0, 3_000], foreign_poly: [0, 0, 5_000], ..t };
    let m = Profile { matched: Some(MatchClass::Exceptional), ..face(Face::L, &[(role("diff_in"), 0)]) };
    assert_eq!(k.halo(&m), 5_000);
    let m = Profile { matched: Some(MatchClass::Exceptional), ..face(Face::L, &[(role("diff_out"), 10)]) };
    assert_eq!(k.halo(&m), 4_990);
}

// ---------------------------------------------------------------- spacing: profiles

#[test]
fn profile_reads_roles_wells_and_insets() {
    let sh = |l: u16, x, y, w, h| Shape { layer: LayerId(l), rect: rect(x, y, w, h) };
    let m = Macro {
        bbox: rect(0, 0, 1_000, 1_000),
        shapes: vec![sh(1, 0, 0, 500, 1_000), sh(2, 100, 100, 200, 200), sh(2, 400, 600, 200, 200), sh(99, 900, 900, 200, 200), sh(3, 495, 0, 10, 10)],
        ..Default::default()
    };
    let p = profile(&m, &Deck, Some(NetId(7)));
    assert_eq!((p.well_net, p.matched, p.set), (Some(NetId(7)), None, None));
    let e = |f: Face, r: &str| p.edge[f as usize].inset[role(r)];
    assert_eq!((e(Face::L, "diff_in"), e(Face::B, "diff_in"), e(Face::R, "diff_in"), e(Face::T, "diff_in")), (100, 100, 700, 700));
    assert_eq!((e(Face::L, "diff_out"), e(Face::R, "diff_out")), (400, 400), "straddles the well");
    assert_eq!((e(Face::R, "other"), e(Face::T, "other"), e(Face::L, "other")), (0, 0, 900), "pokes out: inset 0");
    assert_eq!(e(Face::B, "tap_out"), 0, "a tap crossing the well edge is outside");
    assert_eq!(e(Face::L, "nwell"), 0);
    assert_eq!(e(Face::R, "nwell"), 500);
    assert_eq!(p.edge[0].present & (1 << role("poly")), 0);
}

#[test]
fn profile_of_an_empty_macro_is_empty() {
    let p = profile(&Macro::default(), &Deck, None);
    assert_eq!(p, Profile::default());
}

#[test]
fn orients_permute_faces_as_a_group() {
    let mut p = Profile::default();
    for (k, f) in [Face::L, Face::B, Face::R, Face::T].into_iter().enumerate() {
        p.edge[f as usize].put(k, 10 * k as i32 + 1);
    }
    let all = Profiles::orients(&p);
    for (i, o) in ORIENTS.into_iter().enumerate() {
        assert_eq!(all[i], oriented(&p, o));
        // A bijection on faces: the four edges reappear, each once.
        let mut seen: Vec<_> = all[i].edge.iter().map(|e| e.present).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![1, 2, 4, 8], "{o:?}");
        let h = oriented_faces([1, 11, 21, 31], o);
        for f in 0..4 {
            let k = all[i].edge[f].present.trailing_zeros() as usize;
            assert_eq!(h[f], 10 * k as i32 + 1, "oriented_faces agrees with oriented, {o:?}");
        }
    }
    assert_eq!(oriented(&oriented(&p, Orient::R90), Orient::R90), oriented(&p, Orient::R180));
    assert_eq!(oriented(&oriented(&p, Orient::Mx), Orient::Mx), p);
    assert_eq!(oriented_faces([1, 2, 3, 4], Orient::R180), [3, 4, 1, 2]);
    assert_eq!(oriented_faces([1, 2, 3, 4], Orient::R0), [1, 2, 3, 4]);
}

// ---------------------------------------------------------------- Prices

#[test]
fn fresh_prices() {
    let p = Prices::new();
    assert!(p.drift().is_infinite());
    assert_eq!(p.steps(), 0);
    assert!(p.saturated().is_empty());
    assert_eq!((p.weight_of(0), p.rho_of(0), p.weight_of(usize::MAX)), (0.0, 0.0, 0.0));
}

#[test]
fn settle_on_no_budget_is_a_stationary_step() {
    let mut p = Prices::new();
    let r = Requirements::<Layout>::default();
    p.settle(&r, &lay(&[], &[], &[], &[]));
    assert_eq!((p.drift(), p.steps()), (0.0, 1));
}

#[test]
fn first_violated_step_prices_at_the_rho_floor() {
    let mut b = Batch::new("b");
    b.res = |_| 1.0;
    let r = reqs(Vec::new(), vec![b], Vec::new());
    let mut p = Prices::new();
    p.settle(&r, &lay(&[0], &[0], &[1], &[1]));
    assert_eq!((p.weight_of(0), p.rho_of(0)), (0.25, 0.25));
    assert!((p.drift() - 0.25).abs() < 1e-9);
}

#[test]
fn slack_from_criticality_relaxes_the_price() {
    let mut b = Batch::new("b");
    b.res = |l| f64::from(l.x[0]);
    b.crit = |_| 0.5;
    let r = reqs(Vec::new(), vec![b], Vec::new());
    let mut l = lay(&[1], &[0], &[1], &[1]);
    let mut p = Prices::new();
    p.settle(&r, &l);
    p.settle(&r, &l);
    let paid = p.weight_of(0);
    l.x[0] = 0; // satisfied, criticality 0.5 ⇒ g = −0.5
    p.settle(&r, &l);
    assert!(p.weight_of(0) < paid);
    assert!(p.rho_of(0) >= 0.25, "ρ never under its floor");
}

#[test]
fn nan_inputs_never_poison_the_price() {
    let mut b = Batch::new("nan");
    b.crit = |_| f32::NAN;
    let mut u = Batch::new("u");
    u.usage = |_| Some(f32::NAN);
    let mut r = Batch::new("r");
    r.res = |_| f64::NAN;
    let req = reqs(Vec::new(), vec![b, u, r], Vec::new());
    let mut p = Prices::new();
    for _ in 0..3 {
        p.settle(&req, &lay(&[0], &[0], &[1], &[1]));
    }
    for bi in 0..3 {
        assert_eq!(p.weight_of(bi), 0.0, "batch {bi}");
        assert!(p.rho_of(bi).is_finite());
    }
    assert!(p.drift().is_finite());
}

#[test]
fn untagged_ordinals_ignore_tagged_batches_of_the_same_kind() {
    let tagged = |inner: Batch| -> Box<dyn RuleBatch<Layout>> {
        Box::new(analog::rule::Tagged {
            meta: analog::intent::BatchMeta { id: analog::intent::ConstraintId(9), origin: analog::intent::Origin::NetClass },
            inner: Box::new(inner),
        })
    };
    let mut hot = Batch::new("k");
    hot.res = |_| 1.0;
    let mut cold = Batch::new("k");
    cold.res = |_| 0.5;
    let r = Requirements { hard: Vec::new(), budget: vec![tagged(hot), Box::new(cold) as Box<dyn RuleBatch<Layout>>], cost: Vec::new() };
    let mut p = Prices::new();
    p.settle(&r, &lay(&[0], &[0], &[1], &[1]));
    let (w_tag, w_ord) = (p.weight_of(0), p.weight_of(1));
    assert_ne!(w_tag, w_ord);
    // Drop the tagged batch: the untagged one is still ordinal 0 of kind "k".
    let mut only = Batch::new("k");
    only.res = |_| 0.5;
    let r2 = reqs(Vec::new(), vec![only], Vec::new());
    p.bind(&r2);
    assert_eq!(p.weight_of(0), w_ord);
}

// ---------------------------------------------------------------- net_weights

fn sig(net: u16, budget: Option<i64>) -> NetClassification {
    NetClassification { net: NetId(net), class: NetClass::Signal, c_budget_af: budget, max_coupling_af: None }
}

#[test]
fn net_weights_of_nothing_is_empty() {
    assert!(net_weights(&[], &[], &[]).is_empty());
    assert!(net_weights(&[], &[(NetId(3), 1.0)], &[Some(5)]).is_empty());
}

#[test]
fn nonpositive_budgets_and_bad_sensitivities_are_ignored() {
    let classes = [sig(0, Some(0)), sig(1, Some(-5)), sig(2, Some(1_000))];
    let w = net_weights(&classes, &[(NetId(9), 4.0), (NetId(0), f32::NAN), (NetId(1), -3.0)], &[]);
    assert_eq!(w, vec![1.0, 1.0, 1.0]);
}

#[test]
fn every_weight_is_finite_and_nonnegative() {
    let classes = [sig(0, Some(1_000)), sig(1, Some(2_000)), sig(2, None)];
    for s in [f32::INFINITY, f32::MAX, 1e30] {
        let w = net_weights(&classes, &[(NetId(0), s)], &[]);
        assert!(w.iter().all(|x| x.is_finite() && *x >= 0.0), "{s}: {w:?}");
    }
}

#[test]
fn weighted_signal_nets_have_mean_one() {
    let classes = [sig(0, Some(500)), sig(1, Some(2_000)), sig(2, Some(8_000)), sig(3, None)];
    let w = net_weights(&classes, &[(NetId(1), 0.01)], &[]);
    assert!(((w[0] + w[1] + w[2]) / 3.0 - 1.0).abs() < 1e-5, "{w:?}");
    assert_eq!(w[3], 1.0);
    assert!(w[1] > w[0], "a sensitivity beats a class budget: {w:?}");
}

#[test]
fn rail_weights_clamp_and_default() {
    let rail = |n, class| NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None };
    let classes = [rail(0, NetClass::Supply), rail(1, NetClass::Ground), rail(2, NetClass::Supply)];
    let w = net_weights(&classes, &[], &[Some(-50), Some(0), None]);
    assert_eq!(w, vec![RAIL_MIN, RAIL_MIN, RAIL_UNKNOWN], "no positive current: every known rail at the floor");
    let w = net_weights(&classes, &[(NetId(0), 5.0)], &[Some(400), Some(800)]);
    assert_eq!(w[0], 0.5, "a sensitivity does not override a rail's current weight");
    assert_eq!((w[1], w[2]), (1.0, RAIL_UNKNOWN));
}

// ---------------------------------------------------------------- PlaceRules

/// Cell 0: `diff_out` on its R face; cell 1: n-well on its L face; 340 nm rule.
fn asym_rules() -> PlaceRules {
    let t = table(&[("diff_out", "nwell", 340)], 5);
    let a = face(Face::R, &[(role("diff_out"), 0)]);
    let b = face(Face::L, &[(role("nwell"), 0)]);
    PlaceRules::new(5, t, Profiles { of: vec![vec![Profiles::orients(&a)], vec![Profiles::orients(&b)]] })
}

#[test]
fn gaps_follow_centre_order_and_ties_take_the_max() {
    let r = asym_rules();
    let mut l = lay(&[0, 1_000], &[0, 0], &[100, 100], &[100, 100]);
    assert_eq!(r.gaps(&l, 0, 1), (340, 0));
    assert_eq!(r.gaps(&l, 1, 0), (340, 0), "argument order does not matter");
    l.x = vec![1_000, 0];
    assert_eq!(r.gaps(&l, 0, 1), (0, 0), "a's R face no longer faces b");
    l.x = vec![0, 0];
    assert_eq!(r.gaps(&l, 0, 1), (340, 0), "tie: both orders' max");
    l.y = vec![0, 0];
    assert_eq!(r.gaps(&l, 0, 1).1, 0);
}

#[test]
fn rules_without_a_profile_use_the_fallback() {
    let r = PlaceRules::uniform(10, 270);
    let l = lay(&[0, 0], &[0, 0], &[1, 1], &[1, 1]);
    assert_eq!(r.gaps(&l, 0, 1), (270, 270));
    assert_eq!(r.halo(&l, 0), 270);
    assert!(r.profile_of(0, 0, Orient::R0).is_none());
    assert_eq!(r.axis_grid, None);
    let p = asym_rules();
    assert!(p.profile_of(0, 0, Orient::R0).is_some());
    assert!(p.profile_of(0, 1, Orient::R0).is_none(), "missing variant");
    assert!(p.profile_of(5, 0, Orient::R0).is_none(), "missing cell");
    // A layout with fewer variant entries than cells reads as no profile.
    let mut short = lay(&[0, 0], &[0, 0], &[1, 1], &[1, 1]);
    short.variant.clear();
    assert_eq!(p.gaps(&short, 0, 1), (0, 0));
    assert_eq!(p.halo(&short, 0), 0);
}

#[test]
fn rules_encroach_matches_its_formula_and_the_far_cut() {
    let r = asym_rules();
    for dx in [-1_000, -540, -539, -200, 0, 200, 539, 540, 541, 1_000] {
        let l = lay(&[0, dx], &[0, 37], &[100, 100], &[100, 100]);
        let (gx, gy) = r.gaps(&l, 0, 1);
        let ox = (200 + gx - dx.abs()).max(0);
        let oy = (200 + gy - 37).max(0);
        assert_eq!(r.encroach(&l, 0, 1), f64::from(ox) * f64::from(oy), "dx {dx}");
        assert_eq!(r.encroach(&l, 0, 1), r.encroach(&l, 1, 0));
    }
    let one = lay(&[0], &[0], &[1], &[1]);
    let e = r.encroachment(&one);
    assert!(e == 0.0 && e.is_sign_positive());
}

// ---------------------------------------------------------------- place

fn squares(n: usize) -> Vec<Macro> {
    vec![Macro { bbox: rect(0, 0, 1_000, 1_000), ..Default::default() }; n]
}

fn input<'a>(macros: &'a [Macro], reqs: &'a Requirements<Layout>, rules: &'a PlaceRules, power: &'a [i32]) -> GpInput<'a> {
    GpInput {
        macros,
        variants: &[],
        assignment: &[],
        reqs,
        rules,
        net_weight: &[],
        n_axes: 1,
        power_uw: power,
        units: Default::default(),
        iterate: true,
    }
}

#[test]
fn place_of_nothing_is_an_empty_clean_layout() {
    let r = Requirements::default();
    let rules = PlaceRules::uniform(10, 0);
    let (l, rep) = place(&input(&[], &r, &rules, &[]), &mut Prices::new(), 1);
    assert!(l.x.is_empty() && l.axis.len() == 1);
    assert!(rep.hard_violations.is_empty() && rep.budget_violations.is_empty());
    assert_eq!(rep.cost, 0.0);
}

#[test]
fn place_is_seed_deterministic_and_stays_on_the_die() {
    let macros = squares(9);
    let r = Requirements::default();
    let rules = PlaceRules::uniform(10, 100);
    let inp = input(&macros, &r, &rules, &[]);
    let (a, _) = place(&inp, &mut Prices::new(), 5);
    let (b, _) = place(&inp, &mut Prices::new(), 5);
    assert_eq!((&a.x, &a.y), (&b.x, &b.y));
    let side = canvas_side(&a.hw, &a.hh, 0.4, 10);
    for i in 0..9 {
        assert!(a.x[i] >= a.hw[i] && a.x[i] <= side - a.hw[i], "x {}", a.x[i]);
        assert!(a.y[i] >= a.hh[i] && a.y[i] <= side - a.hh[i], "y {}", a.y[i]);
    }
}

#[test]
fn place_never_raises_phi_over_the_pile() {
    // Hard: cell 0 left of cell 1, by how far it is not.
    let mut h = Batch::new("order");
    h.viol = |l| u32::from(l.x[0] > l.x[1]);
    h.res = |l| f64::from((l.x[0] - l.x[1]).max(0)) / 1_000.0;
    let r = reqs(vec![h], Vec::new(), Vec::new());
    let rules = PlaceRules::uniform(10, 0);
    let macros = squares(6);
    for seed in 1..=6u64 {
        let inp = input(&macros, &r, &rules, &[]);
        let (pile, _) = place(&GpInput { iterate: false, ..input(&macros, &r, &rules, &[]) }, &mut Prices::new(), seed);
        let (l, _) = place(&inp, &mut Prices::new(), seed);
        let (a, b) = (analog_phi(&r, &pile), analog_phi(&r, &l));
        assert!(b.0 < a.0 || (b.0 == a.0 && b.1 <= a.1), "seed {seed}: {b:?} > {a:?}");
    }
}

#[test]
fn place_ignores_power_of_the_wrong_length() {
    let macros = squares(3);
    let r = Requirements::default();
    let rules = PlaceRules::uniform(10, 0);
    let (l, _) = place(&GpInput { iterate: false, ..input(&macros, &r, &rules, &[5, 5]) }, &mut Prices::new(), 1);
    assert_eq!(l.power_uw, vec![0, 0, 0]);
    let (l, _) = place(&GpInput { iterate: false, ..input(&macros, &r, &rules, &[5, 6, 7]) }, &mut Prices::new(), 1);
    assert_eq!(l.power_uw, vec![5, 6, 7]);
}

#[test]
fn place_binds_but_never_steps_prices() {
    let mut b = Batch::new("b");
    b.res = |_| 1.0;
    let r = reqs(Vec::new(), vec![b], Vec::new());
    let mut p = Prices::new();
    p.settle(&r, &lay(&[0], &[0], &[1], &[1]));
    let (w, steps) = (p.weight_of(0), p.steps());
    let macros = squares(2);
    let rules = PlaceRules::uniform(10, 0);
    let _ = place(&input(&macros, &r, &rules, &[]), &mut p, 3);
    assert_eq!((p.weight_of(0), p.steps()), (w, steps));
}

#[test]
fn place_keeps_the_assigned_variant() {
    let macros = squares(2);
    let spaces = [VariantSpace { alternatives: vec![squares(1)[0].clone(), Macro { bbox: rect(0, 0, 2_000, 400), ..Default::default() }] }];
    let r = Requirements::default();
    let rules = PlaceRules::uniform(10, 0);
    let inp = GpInput { variants: &spaces, assignment: &[1], iterate: false, ..input(&macros, &r, &rules, &[]) };
    let (l, _) = place(&inp, &mut Prices::new(), 1);
    assert_eq!(l.variant, vec![1, 0]);
    assert_eq!((l.hw[0], l.hh[0]), (1_000, 200));
    assert_eq!((l.hw[1], l.hh[1]), (500, 500));
}
