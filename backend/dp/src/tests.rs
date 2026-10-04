use super::*;

/// sky130-like placement rules the fixed-geometry tests are written against.
const RULES: gp::Rules = gp::Rules { grid: 5, clearance: 2000 };
const CLEARANCE_NM: i32 = RULES.clearance;
use gp::mechanics::{analog_violations, encroachment};
use analog::placement::symmetry::{Symmetry, SymmetryGroup};
use analog::placement::DtiBand;
use analog::Rule;
use gp::VariantSpace;
use pnr_core::geom::Rect;
use pnr_core::ids::{AxisId, Target};
use pnr_core::DeviceId;

/// Layout of devices `(x, y, hw, hh)`, each in its own group.
fn layout(cells: &[(i32, i32, i32, i32)]) -> Layout {
    let n = cells.len();
    Layout {
        x: cells.iter().map(|c| c.0).collect(),
        y: cells.iter().map(|c| c.1).collect(),
        hw: cells.iter().map(|c| c.2).collect(),
        hh: cells.iter().map(|c| c.3).collect(),
        variant: vec![0; n],
        axis: vec![0; n],
        branch: vec![false; n],
        groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
        orient: vec![Orient::default(); n],
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
        units: Default::default(),
    }
}

fn run(
    coarse: &Layout,
    macros: &[Macro],
    variants: &[VariantSpace],
    reqs: &Requirements<Layout>,
    fixed: &[bool],
    seed: u64,
) -> Layout {
    let locks = locks::locks(reqs, coarse.x.len(), variants);
    place(coarse, macros, variants, reqs, fixed, &locks, &mut gp::Prices::new(), RULES, &[], seed, Schedule::cold()).0
}

fn sym(a: u16, b: u16) -> Symmetry {
    Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0) }
}

// ---- rotation ----

/// Four tall devices; 0 and 1 form one matched group (`groups`, which dp ignores).
fn rotate_bench() -> Layout {
    let mut l = layout(&[(0, 0, 1_000, 8_000), (20_000, 0, 1_000, 8_000), (40_000, 0, 1_000, 8_000), (60_000, 0, 1_000, 8_000)]);
    l.groups = vec![vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2)], vec![DeviceId(3)]];
    l
}

#[test]
fn matched_cells_share_one_orient_set() {
    let reqs = Requirements { hard: vec![Box::new(SymmetryGroup(vec![sym(0, 1)]))], ..Default::default() };
    let k = locks::locks(&reqs, 3, &[]);
    assert_eq!(k.orient_of[0], k.orient_of[1]);
    assert!(k.orient_of[0].is_some());
    assert_eq!(k.orient_of[2], None);
}

/// A matched set that is no hard rule and has no units: the lock can only come
/// from `matched_pairs`, and it must actually turn.
#[test]
fn mixed_polarity_composite_halves_turn_together() {
    use analog::placement::{OrientCheck, OrientationSet};
    let coarse = rotate_bench();
    let reqs = Requirements {
        budget: vec![Box::new(OrientationSet { members: vec![DeviceId(0), DeviceId(1)], check: OrientCheck::Phi, cell_of: vec![] })],
        ..Default::default()
    };
    let mut turned = false;
    for seed in 1..=20u64 {
        let l = run(&coarse, &[], &[], &reqs, &[false; 4], seed);
        assert_eq!(l.orient[0], l.orient[1], "seed {seed}");
        turned |= l.orient[0] != Orient::R0;
    }
    assert!(turned, "the locked set never turned");
}

#[test]
fn extents_stay_in_lockstep_with_orientation() {
    let coarse = rotate_bench();
    let reqs = Requirements { hard: vec![Box::new(SymmetryGroup(vec![sym(0, 1)]))], ..Default::default() };
    let l = run(&coarse, &[], &[], &reqs, &[false; 4], 42);
    for i in 0..4 {
        let (w, h) = (coarse.hw[i], coarse.hh[i]);
        let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
        assert_eq!((l.hw[i], l.hh[i]), want, "device {i}");
    }
    assert_eq!(l.orient[0], l.orient[1]);
}

#[test]
fn placement_is_deterministic_for_a_seed() {
    let coarse = rotate_bench();
    let go = || {
        let l = run(&coarse, &[], &[], &Requirements::default(), &[false; 4], 7);
        (l.x, l.y, l.orient)
    };
    assert_eq!(go(), go());
}

#[test]
fn rotation_actually_happens() {
    let l = run(&rotate_bench(), &[], &[], &Requirements::default(), &[false; 4], 0);
    assert!(l.orient[2..].iter().any(|&o| o != Orient::R0), "{:?}", l.orient);
}

// ---- variant reshape ----

fn alt(w: i32, h: i32) -> Macro {
    Macro { shapes: Vec::new(), pins: Vec::new(), bbox: Rect { x: 0, y: 0, w, h }, units: Vec::new(), dummies: Vec::new(), ..Default::default() }
}

/// Three alternatives per cell, not ordered by similarity.
fn spaces() -> Vec<VariantSpace> {
    let s = || VariantSpace {
        alternatives: vec![alt(2_000, 16_000), alt(16_000, 2_000), alt(6_000, 6_000)],
    };
    vec![s(), s()]
}

fn drawn(variants: &[VariantSpace]) -> Vec<Macro> {
    variants.iter().map(|v| v.alternatives[0].clone()).collect()
}

fn variant_bench() -> Layout {
    layout(&[(0, 0, 1_000, 8_000), (200_000, 0, 1_000, 8_000)])
}

fn assert_extents(l: &Layout, variants: &[VariantSpace], seed: u64) {
    for i in 0..2 {
        let (w, h) = gp::mechanics::variant_extents(&variants[i].alternatives[l.variant[i] as usize]);
        let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
        assert_eq!((l.hw[i], l.hh[i]), want, "seed {seed}: cell {i} sized off-variant");
    }
}

#[test]
fn reshape_changes_the_variant_and_keeps_the_extent_invariant() {
    let coarse = variant_bench();
    let variants = spaces();
    let mut fired = false;
    for seed in 0..8u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[false; 2], seed);
        fired |= l.variant != coarse.variant;
        assert_extents(&l, &variants, seed);
    }
    assert!(fired, "the reshape move never fired");
}

#[test]
fn a_fixed_cell_never_reshapes() {
    let coarse = variant_bench();
    let variants = spaces();
    for seed in 0..8u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[true, false], seed);
        assert_eq!(l.variant[0], 0, "seed {seed}");
        assert_eq!((l.hw[0], l.hh[0]), (coarse.hw[0], coarse.hh[0]), "seed {seed}");
    }
}

#[test]
fn mirror_partners_reshape_together() {
    let s = || VariantSpace { alternatives: vec![alt(1_000, 4_000), alt(2_000, 2_000), alt(4_000, 1_000)] };
    let variants = vec![s(), s()];
    let coarse = layout(&[(-20_000, 0, 500, 2_000), (20_000, 0, 500, 2_000)]);
    let reqs = Requirements { hard: vec![Box::new(SymmetryGroup(vec![sym(0, 1)]))], ..Default::default() };
    let mut reshaped = false;
    for seed in 1..=20u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &reqs, &[false; 2], seed);
        assert_eq!(l.variant[0], l.variant[1], "seed {seed}");
        reshaped |= l.variant[0] != 0;
    }
    assert!(reshaped, "the locked pair never reshaped");
}

/// Fixed means drawn as given, not pinned: two stacked fixed cells separate.
#[test]
fn fixed_cells_separate_but_keep_shape() {
    let variants = spaces();
    let coarse = layout(&[(0, 0, 1_000, 8_000), (0, 0, 1_000, 8_000)]);
    for seed in 0..4u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[true, true], seed);
        assert_eq!(encroachment(&l, 0), 0.0, "seed {seed}");
        assert_eq!(l.variant, vec![0, 0], "seed {seed}");
        assert_eq!(l.orient, vec![Orient::R0, Orient::R0], "seed {seed}");
        assert_eq!((&l.hw, &l.hh), (&coarse.hw, &coarse.hh), "seed {seed}");
    }
}

#[test]
fn unmatched_cells_reshape_independently() {
    let coarse = variant_bench();
    let variants = spaces();
    assert!((0..16u64).any(|seed| {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[false; 2], seed);
        l.variant[0] != l.variant[1]
    }));
}

/// Same bbox, one pin on the face at `x`.
fn pin_alt(x: i32) -> Macro {
    Macro {
        shapes: Vec::new(),
        pins: vec![pnr_core::Pin {
            name: "G".to_string(),
            net: pnr_core::NetId(0),
            at: Rect { x, y: 4_950, w: 100, h: 100 },
            layer: pnr_core::LayerId(0),
        }],
        bbox: Rect { x: 0, y: 0, w: 10_000, h: 10_000 },
        units: Vec::new(),
        dummies: Vec::new(),
        ..Default::default()
    }
}

/// Equal bboxes, different pin faces: the reshape must be priced on pins.
/// `temp = 0` makes the refusal of the lengthening move unfakeable.
#[test]
fn reshape_is_priced_on_where_the_pins_land() {
    let space = || VariantSpace { alternatives: vec![pin_alt(0), pin_alt(9_900)] };
    let variants = vec![space(), space()];
    let mut l = layout(&[(0, 0, 5_000, 5_000), (100_000, 0, 5_000, 5_000)]);
    let reqs = Requirements::default();
    let prices = gp::Prices::new();
    let nets = Nets::from_macros(&[pin_alt(0), pin_alt(0)]);
    assert_eq!(nets.count(), 1);
    let mut sa = Sa::new(nets, 2, &reqs, &prices, &[], gp::Rules { clearance: 0, ..RULES });
    let mut rng = SplitMix64::new(1);
    let free = |c: i32, _half: i32| c;

    let before = hpwl(&sa.nets, &l);
    assert!(try_reshape(&mut sa, &mut l, &mut rng, 0.0, &[0], &variants, &free, &free));
    assert_eq!(l.variant[0], 1);
    let after = hpwl(&sa.nets, &l);
    assert!(after < before, "{before} -> {after}");

    assert!(!try_reshape(&mut sa, &mut l, &mut rng, 0.0, &[0], &variants, &free, &free));
    assert_eq!(l.variant[0], 1);
    assert_eq!(hpwl(&sa.nets, &l), after, "refusal must revert the pin geometry too");
}

// ---- prices ----

/// T6: the dual step is the flow's, once per epoch on the scored layout; gp
/// and dp only bind, so a violated budget leaves a fresh `Prices` unstepped.
#[test]
fn place_does_not_settle() {
    #[derive(Clone, Copy)]
    struct Over;
    impl Rule for Over {
        type On = Layout;
        fn cost(self, _: &Layout) -> f32 {
            0.0
        }
        fn satisfied(self, _: &Layout) -> bool {
            false
        }
    }
    let variants = spaces();
    let macros = drawn(&variants);
    let reqs = Requirements { hard: Vec::new(), budget: vec![Box::new(vec![Over])], cost: Vec::new() };
    let mut prices = gp::Prices::new();
    let inp = gp::GpInput {
        macros: &macros,
        variants: &variants,
        assignment: &[0, 0],
        reqs: &reqs,
        rules: RULES,
        net_weight: &[],
        n_axes: 1,
        power_uw: &[],
        units: Default::default(),
        iterate: true,
    };
    let (coarse, _) = gp::place(&inp, &mut prices, 3);
    let locks = locks::locks(&reqs, coarse.x.len(), &variants);
    place(&coarse, &macros, &variants, &reqs, &[false; 2], &locks, &mut prices, RULES, &[], 3, Schedule::cold());
    assert_eq!(prices.drift(), f64::INFINITY, "no dual step inside place");
    assert_eq!(prices.steps(), 0);
}

// ---- lexicographic acceptance ----

/// PEX change for moving device 0 to `(x, 0)`.
fn dpex(sa: &Sa, l: &mut Layout, x: i32) -> f64 {
    let before = sa.pex(l);
    let ox = l.x[0];
    l.x[0] = x;
    let d = sa.pex(l) - before;
    l.x[0] = ox;
    d
}

/// Cost pulls devices 0 and 1 together with no floor.
#[derive(Clone, Copy)]
struct Attract;
impl Rule for Attract {
    type On = Layout;
    fn cost(self, l: &Layout) -> f32 {
        ((l.x[0] - l.x[1]).abs() + (l.y[0] - l.y[1]).abs()) as f32
    }
}

fn attract_bench() -> (Requirements<Layout>, Layout) {
    let reqs = Requirements { hard: Vec::new(), budget: Vec::new(), cost: vec![Box::new(vec![Attract])] };
    (reqs, layout(&[(0, 0, 1_000, 1_000), (4_000, 0, 1_000, 1_000)]))
}

/// At `temp = 1e12` Metropolis takes anything, so a refusal is the Φ tier's.
#[test]
fn phi_rejects_a_cost_lowering_move_that_stacks_geometry() {
    let (reqs, mut l) = attract_bench();
    let prices = gp::Prices::new();
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], gp::Rules { clearance: 0, ..RULES });
    let mut rng = SplitMix64::new(1);
    assert!(dpex(&sa, &mut l, 4_000) < 0.0);
    assert!(!try_move(&mut sa, &mut l, &mut rng, 1e12, 0, 4_000, 0));
    assert_eq!(l.x[0], 0);
    assert_eq!(encroachment(&l, 0), 0.0);
}

#[test]
fn theta_outranks_pex_so_a_budget_is_never_traded_for_parasitics() {
    #[derive(Clone, Copy)]
    struct Separation;
    impl Rule for Separation {
        type On = Layout;
        fn cost(self, _: &Layout) -> f32 {
            0.0
        }
        fn residual(self, l: &Layout) -> f32 {
            ((2_000 - (l.x[0] - l.x[1]).abs()).max(0) as f32) / 2_000.0
        }
    }
    let (mut reqs, mut l) = attract_bench();
    reqs.budget = vec![Box::new(vec![Separation])];
    let prices = gp::Prices::new();
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], gp::Rules { clearance: 0, ..RULES });
    let mut rng = SplitMix64::new(1);
    assert_eq!(analog_theta(&reqs, &l), 0.0);
    assert!(dpex(&sa, &mut l, 3_000) < 0.0);
    assert!(!try_move(&mut sa, &mut l, &mut rng, 1e12, 0, 3_000, 0));
    assert_eq!(l.x[0], 0);
}

#[test]
fn metropolis_still_accepts_an_uphill_move_inside_the_pex_tier() {
    let (reqs, mut l) = attract_bench();
    let prices = gp::Prices::new();
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], gp::Rules { clearance: 0, ..RULES });
    let mut rng = SplitMix64::new(1);
    assert!(dpex(&sa, &mut l, -20_000) > 0.0);
    assert!(try_move(&mut sa, &mut l, &mut rng, 1e12, 0, -20_000, 0));
    assert_eq!(l.x[0], -20_000);
}

/// At the same hard-violation count, raising clearance encroachment must never
/// win against a cheaper PEX, even at an extreme temperature.
#[test]
fn gate_never_trades_a_broken_pair_for_overlap() {
    let e = 1_000.0;
    let mut rng = SplitMix64::new(1);
    for draw in 0..100 {
        assert!(
            !accept((1, 0.002, e, 0.0), (1, 0.003, e - 2.0, 0.0), -1.0, 1e12, &mut rng),
            "draw {draw}"
        );
    }
}

// ---- DTI branch flip ----

fn band(id: u16, seed_isolate: bool) -> Requirements<Layout> {
    let b = || {
        vec![DtiBand {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            s_max_nm: 200,
            d_dti_nm: 2_000,
            branch: BranchId(id),
            seed_isolate,
        }]
    };
    Requirements { hard: vec![Box::new(b())], budget: Vec::new(), cost: vec![Box::new(b())] }
}

fn gap_bench(gap: i32, branch: Vec<bool>) -> Layout {
    let mut l = layout(&[(0, 0, 1_000, 1_000), (2_000 + gap, 0, 1_000, 1_000)]);
    l.branch = branch;
    l
}

/// Abutting pair committed to `isolate`: the flip to `share` strictly lowers
/// PEX and is taken; the reverse is refused.
#[test]
fn branch_flip_fires_and_is_priced() {
    let reqs = band(0, true);
    let mut l = gap_bench(100, vec![true]);
    let prices = gp::Prices::new();
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], gp::Rules { clearance: 0, ..RULES });
    let mut rng = SplitMix64::new(1);
    assert!(try_branch(&mut sa, &mut l, &mut rng, 0.0, 0));
    assert!(!l.branch[0]);
    assert!(!try_branch(&mut sa, &mut l, &mut rng, 0.0, 0));
    assert!(!l.branch[0]);
}

/// Both cells pinned, so the returned branch table is exactly what seeding wrote.
#[test]
fn seeds_are_written_and_table_resized() {
    let reqs = band(3, true);
    let mut b = Vec::new();
    assert_eq!(seed_branches(&reqs, &mut b), vec![BranchId(3)]);
    assert_eq!(b, vec![false, false, false, true]);
    let mut b = vec![false; 6];
    seed_branches(&reqs, &mut b);
    assert_eq!(b, vec![false, false, false, true, false, false]);
}

// ---- exact projection ----

fn sym_bench() -> (Requirements<Layout>, Layout) {
    let reqs = Requirements {
        hard: vec![Box::new(SymmetryGroup(vec![Symmetry {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            axis: AxisId(0),
        }]))],
        budget: Vec::new(),
        cost: Vec::new(),
    };
    (reqs, layout(&[(-30_000, 0, 1_000, 1_000), (30_500, 3_500, 1_000, 1_000)]))
}

#[test]
fn symmetry_holds_at_exit_with_room_to_move() {
    let (reqs, coarse) = sym_bench();
    let l = run(&coarse, &[], &[], &reqs, &[false; 2], 11);
    assert_eq!(analog_violations(&reqs, &l), 0, "x = {:?}, y = {:?}, axis = {:?}", l.x, l.y, l.axis);
}

#[test]
fn projection_moves_fixed_cells_but_not_their_shape() {
    let (reqs, coarse) = sym_bench();
    for seed in 0..4u64 {
        let l = run(&coarse, &[], &[], &reqs, &[true, false], seed);
        assert_eq!(analog_violations(&reqs, &l), 0, "seed {seed}");
        assert_eq!((l.hw[0], l.hh[0], l.orient[0]), (coarse.hw[0], coarse.hh[0], Orient::R0), "seed {seed}");
    }
}

/// A mirror pair stacked on its own axis satisfies `Symmetry` with zero
/// separation; single-cell moves must still pull it apart (the partner follows
/// by projection) until the pair is clearance-legal and still mirrored.
#[test]
fn a_stacked_symmetric_pair_separates_and_stays_mirrored() {
    let (reqs, _) = sym_bench();
    let coarse = layout(&[(0, 0, 1_000, 1_000), (0, 0, 1_000, 1_000), (5_000, 0, 1_000, 1_000)]);
    let l = run(&coarse, &[], &[], &reqs, &[false; 3], 3);
    assert_eq!(analog_violations(&reqs, &l), 0, "x = {:?}, y = {:?}", l.x, l.y);
    assert_eq!(encroachment(&l, CLEARANCE_NM), 0.0, "x = {:?}, y = {:?}", l.x, l.y);
}

/// A diff stage: input pair and load pair mirrored about ONE shared axis, plus
/// a self-symmetric tail cell, all starting piled on top of each other.
#[test]
fn two_pairs_and_a_tail_share_one_axis_legally() {
    let sym = |a: u16, b: u16| Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0) };
    let group = || SymmetryGroup(vec![sym(0, 1), sym(2, 3), sym(4, 4)]);
    let reqs = Requirements { hard: vec![Box::new(group())], budget: Vec::new(), cost: vec![Box::new(group())] };
    for seed in 0..4 {
        let coarse = layout(&[
            (0, 0, 3_000, 2_000),
            (100, 50, 3_000, 2_000),
            (0, 0, 4_000, 1_500),
            (-50, 20, 4_000, 1_500),
            (30, 30, 2_500, 2_500),
        ]);
        let l = run(&coarse, &[], &[], &reqs, &[false; 5], seed);
        assert_eq!(analog_violations(&reqs, &l), 0, "seed {seed}: x = {:?}, y = {:?}", l.x, l.y);
        assert_eq!(encroachment(&l, CLEARANCE_NM), 0.0, "seed {seed}: x = {:?}, y = {:?}", l.x, l.y);
    }
}

#[test]
fn projection_is_deterministic_for_a_seed() {
    let go = || {
        let (reqs, coarse) = sym_bench();
        let l = run(&coarse, &[], &[], &reqs, &[false; 2], 7);
        (l.x, l.y, l.axis)
    };
    assert_eq!(go(), go());
}

/// `encroach_moved`'s before/after difference must equal the full scan's for
/// every one- and two-cell move, or the gate compares the wrong numbers.
#[test]
fn incident_encroachment_difference_matches_full_scan() {
    let cells = [(0, 0, 100, 100), (150, 0, 100, 100), (60, 60, 100, 100), (900, 900, 50, 50), (120, 30, 80, 40)];
    let reqs = Requirements::default();
    let prices = gp::Prices::new();
    for c in 0..cells.len() {
        for o in 0..cells.len() {
            let mut sa = Sa::new(Nets::from_macros(&[]), cells.len(), &reqs, &prices, &[], gp::Rules { clearance: 300, ..RULES });
            sa.moved = if c == o { vec![c] } else { vec![c, o] };
            let mut l = layout(&cells);
            let (full0, inc0) = (encroachment(&l, 300), sa.encroach_moved(&l));
            for &m in &sa.moved {
                l.x[m] += 37;
                l.y[m] -= 11;
            }
            let full = encroachment(&l, 300) - full0;
            let inc = sa.encroach_moved(&l) - inc0;
            assert!((full - inc).abs() < 1e-6, "moved {:?}: {full} vs {inc}", sa.moved);
        }
    }
}

/// Each compound move keeps every mirror equation of a stage exactly, with no
/// projection: Φ never rises, so `trial` never reaches for `project`.
#[test]
fn compound_moves_keep_a_mirrored_stage_mirrored() {
    let sym = |a: u16, b: u16| Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0) };
    let reqs = Requirements {
        hard: vec![Box::new(SymmetryGroup(vec![sym(0, 1), sym(2, 3), sym(4, 4)]))],
        budget: Vec::new(),
        cost: Vec::new(),
    };
    let mut l = layout(&[
        (-6_000, 0, 1_000, 1_000),
        (6_000, 0, 1_000, 1_000),
        (-6_000, 8_000, 1_000, 1_000),
        (6_000, 8_000, 1_000, 1_000),
        (0, -8_000, 1_000, 1_000),
    ]);
    l.axis = vec![0];
    assert_eq!(analog_violations(&reqs, &l), 0);
    let groups = sym_groups(&reqs, 5);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].members.len(), 5);

    let prices = gp::Prices::new();
    let mut sa = Sa::new(Nets::from_macros(&[]), 5, &reqs, &prices, &[], RULES);
    let mut rng = SplitMix64::new(5);
    let free = |c: i32, _: i32| c;
    // Metropolis at huge temperature takes every tie, so each move applies.
    assert!(try_group_shift(&mut sa, &mut l, &mut rng, 1e12, &groups[0], 2_000, -1_000, &free, &free));
    assert_eq!((l.axis[0], l.x[4]), (2_000, 2_000));
    assert!(try_pair_expand(&mut sa, &mut l, &mut rng, 1e12, 0, 1, 1_000, &free));
    assert_eq!((l.x[0], l.x[1]), (-5_000, 9_000));
    assert!(try_pair_swap(&mut sa, &mut l, &mut rng, 1e12, 2, 3));
    assert_eq!(analog_violations(&reqs, &l), 0, "x = {:?}, y = {:?}, axis = {:?}", l.x, l.y, l.axis);
}

// ---- report ----

#[test]
fn report_counts_clearance_only_residue() {
    let l = layout(&[(0, 0, 500, 500), (1_100, 0, 500, 500)]);
    let rep = report(&Nets::from_macros(&[]), &Requirements::default(), &l, &gp::Prices::new(), 270);
    let rules: Vec<&str> = rep.hard_violations.iter().map(|v| v.rule.as_str()).collect();
    let clearance_rows: Vec<_> = rep.hard_violations.iter().filter(|v| v.rule == "clearance encroachment").collect();
    assert_eq!(clearance_rows.len(), 1, "{rules:?}");
    assert_eq!(clearance_rows[0].margin, 215_900);
    assert!(!rules.contains(&"device overlap"), "{rules:?}");
}

/// PLC-10 step 0: `Schedule::cold()` is exactly the constants it replaced.
#[test]
fn cold_schedule_reproduces_todays_layout() {
    let l = run(&rotate_bench(), &[], &[], &Requirements::default(), &[false; 4], 7);
    assert_eq!(l.x, vec![26615, 8990, 44550, 53905]);
    assert_eq!(l.y, vec![-975, -2545, 2190, 265]);
    assert_eq!(l.orient, vec![Orient::R270, Orient::R0, Orient::R180, Orient::R0]);
}
