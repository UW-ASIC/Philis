use super::*;
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
    Annealer
        .place(coarse, macros, variants, reqs, &[], fixed, &mut gp::Prices::new(), &pnr_core::NullOracle, seed)
        .0
}

// ---- rotation ----

/// Four tall devices; 0 and 1 form one matched group.
fn rotate_bench() -> Layout {
    let mut l = layout(&[(0, 0, 1_000, 8_000), (20_000, 0, 1_000, 8_000), (40_000, 0, 1_000, 8_000), (60_000, 0, 1_000, 8_000)]);
    l.groups = vec![vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2)], vec![DeviceId(3)]];
    l
}

#[test]
fn matched_devices_never_turn() {
    let l = rotate_bench();
    assert!(!rotatable(&l, 0) && !rotatable(&l, 1));
    assert!(rotatable(&l, 2) && rotatable(&l, 3));
}

#[test]
fn extents_stay_in_lockstep_with_orientation() {
    let coarse = rotate_bench();
    let l = run(&coarse, &[], &[], &Requirements::default(), &[false; 4], 42);
    for i in 0..4 {
        let (w, h) = (coarse.hw[i], coarse.hh[i]);
        let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
        assert_eq!((l.hw[i], l.hh[i]), want, "device {i}");
    }
    assert_eq!((l.orient[0], l.orient[1]), (Orient::R0, Orient::R0));
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
    Macro { shapes: Vec::new(), pins: Vec::new(), bbox: Rect { x: 0, y: 0, w, h } }
}

/// Three alternatives per cell, not ordered by similarity.
fn spaces(lock: Option<u16>) -> Vec<VariantSpace> {
    let s = || VariantSpace {
        alternatives: vec![alt(2_000, 16_000), alt(16_000, 2_000), alt(6_000, 6_000)],
        lock,
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
    let variants = spaces(None);
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
    let variants = spaces(None);
    for seed in 0..8u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[true, false], seed);
        assert_eq!(l.variant[0], 0, "seed {seed}");
        assert_eq!((l.hw[0], l.hh[0]), (coarse.hw[0], coarse.hh[0]), "seed {seed}");
    }
}

#[test]
fn a_locked_pair_reshapes_together() {
    let coarse = variant_bench();
    let variants = spaces(Some(0));
    let mut fired = false;
    for seed in 0..8u64 {
        let l = run(&coarse, &drawn(&variants), &variants, &Requirements::default(), &[false; 2], seed);
        assert_eq!(l.variant[0], l.variant[1], "seed {seed}: lock broke");
        fired |= l.variant[0] != coarse.variant[0];
        assert_extents(&l, &variants, seed);
    }
    assert!(fired, "the joint reshape never fired");
}

#[test]
fn an_unlocked_cell_still_reshapes_alone() {
    let coarse = variant_bench();
    let variants = spaces(None);
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
    }
}

/// Equal bboxes, different pin faces: the reshape must be priced on pins.
/// `temp = 0` makes the refusal of the lengthening move unfakeable.
#[test]
fn reshape_is_priced_on_where_the_pins_land() {
    let space = || VariantSpace { alternatives: vec![pin_alt(0), pin_alt(9_900)], lock: None };
    let variants = vec![space(), space()];
    let mut l = layout(&[(0, 0, 5_000, 5_000), (100_000, 0, 5_000, 5_000)]);
    let reqs = Requirements::default();
    let prices = gp::Prices::new();
    let nets = Nets::from_macros(&[pin_alt(0), pin_alt(0)]);
    assert_eq!(nets.count(), 1);
    let mut sa = Sa::new(nets, 2, &reqs, &prices, &[], 0);
    let mut rng = SplitMix64::new(1);
    let free = |c: i32, _half: i32| c;

    let before = hpwl(&sa.nets, &l);
    assert!(try_reshape(&mut sa, &mut l, &mut rng, 0.0, 0, &[0], &variants, &free, &free));
    assert_eq!(l.variant[0], 1);
    let after = hpwl(&sa.nets, &l);
    assert!(after < before, "{before} -> {after}");

    assert!(!try_reshape(&mut sa, &mut l, &mut rng, 0.0, 0, &[0], &variants, &free, &free));
    assert_eq!(l.variant[0], 1);
    assert_eq!(hpwl(&sa.nets, &l), after, "refusal must revert the pin geometry too");
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
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], 0);
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
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], 0);
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
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], 0);
    let mut rng = SplitMix64::new(1);
    assert!(dpex(&sa, &mut l, -20_000) > 0.0);
    assert!(try_move(&mut sa, &mut l, &mut rng, 1e12, 0, -20_000, 0));
    assert_eq!(l.x[0], -20_000);
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
    let mut sa = Sa::new(Nets::from_macros(&[]), 2, &reqs, &prices, &[], 0);
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
    let l = run(&gap_bench(200_000, Vec::new()), &[], &[], &reqs, &[true, true], 9);
    assert_eq!(l.branch, vec![false, false, false, true]);
    let l = run(&gap_bench(200_000, vec![false; 6]), &[], &[], &reqs, &[true, true], 9);
    assert_eq!(l.branch, vec![false, false, false, true, false, false]);
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
fn projection_never_moves_pinned_cells() {
    let (reqs, coarse) = sym_bench();
    for seed in 0..4u64 {
        let l = run(&coarse, &[], &[], &reqs, &[true, false], seed);
        assert_eq!((l.x[0], l.y[0]), (coarse.x[0], coarse.y[0]), "seed {seed}");
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
            let mut sa = Sa::new(Nets::from_macros(&[]), cells.len(), &reqs, &prices, &[], 300);
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
