//! Corner-case tests for every public `gr` item: history buckets, route
//! order, the track lattice and its `RGraph` impl, routing state, the search,
//! PathFinder and geometry extraction. Expected values come from the doc
//! comments, worked by hand on small lattices.

use analog::Requirements;
use gr::*;
use pnr_core::geom::{LayerId, Pin, Rect};
use pnr_core::ids::NetId;
use pnr_core::{Macro, Routes};

const ONE: [u8; MAX_LAYERS] = [1; MAX_LAYERS];
const ZERO: [u8; MAX_LAYERS] = [0; MAX_LAYERS];

fn grid(nx: i32, ny: i32, layers: u32) -> TrackGrid {
    TrackGrid::with_layers((nx * 100, ny * 100), 100, 4.0, layers)
}

fn spec(stride: u32, halo_wire: u8, halo_via: u8) -> LayerSpec {
    LayerSpec { stride, halo_wire, halo_via, ..LayerSpec::default() }
}

fn query<'a>(terms: &'a [u32]) -> NetSearch<'a> {
    NetSearch { net: 0, owner: 0, terms, k: ONE, guard: ZERO, term_k: &[], own: &[], own_halo: &[], penalty: &[], keepout: (&[], &[]), blocked: &[], sep: &[], foot: &[], terms_of: &[], mirror: None, elec: Elec::default() }
}

fn mac(pins: &[(u16, i32, i32)]) -> Macro {
    let pin = |&(net, x, y): &(u16, i32, i32)| Pin { name: format!("p{net}"), net: NetId(net), at: Rect { x, y, w: 0, h: 0 }, layer: LayerId(0) };
    Macro { pins: pins.iter().map(pin).collect(), ..Default::default() }
}

// --- Negotiation -----------------------------------------------------------

#[test]
fn a_new_negotiation_has_no_pressure() {
    assert_eq!(Negotiation::new().pressure(), 0.0);
}

#[test]
fn accumulate_keeps_the_max_per_bucket_and_ignores_non_positive() {
    let mut neg = Negotiation::new();
    neg.accumulate(&[2.0, 1.0, 0.0, -3.0], |_| (10, 10, 0));
    assert_eq!(neg.pressure(), 2.0);
    neg.accumulate(&[5.0], |_| (499, 0, 0));
    assert_eq!(neg.pressure(), 5.0, "same 500 nm bucket");
    neg.accumulate(&[1.0], |_| (10, 10, 1));
    assert_eq!(neg.pressure(), 6.0, "another layer is another bucket");
}

#[test]
fn seed_writes_zero_where_no_history_is_stored() {
    let mut neg = Negotiation::new();
    neg.accumulate(&[4.0], |_| (0, 0, 0));
    let mut hist = [9.0, 9.0];
    neg.seed(&mut hist, |n| (n as i32 * 1_000, 0, 0));
    assert_eq!(hist, [4.0, 0.0]);
}

/// The doc promises every entry is overwritten, also with nothing stored.
#[test]
fn seed_from_an_empty_negotiation_clears_stale_history() {
    let mut hist = [9.0, 1.5];
    Negotiation::new().seed(&mut hist, |n| (n as i32, 0, 0));
    assert_eq!(hist, [0.0, 0.0]);
}

#[test]
fn negative_coordinates_bucket_by_floor() {
    let mut neg = Negotiation::new();
    neg.accumulate(&[3.0], |_| (-1, 0, 0));
    let mut h = [0.0, 0.0];
    neg.seed(&mut h, |n| ([-500, 0][n as usize], 0, 0));
    assert_eq!(h, [3.0, 0.0], "-1 and -500 share bucket -1, 0 does not");
}

// --- group_hpwl ------------------------------------------------------------

#[test]
fn group_hpwl_of_nothing_or_single_pins_is_zero() {
    assert_eq!(group_hpwl(&[]), 0);
    assert_eq!(group_hpwl(&[mac(&[(0, 5, 5), (1, 900, 900)])]), 0);
}

#[test]
fn group_hpwl_sums_per_net_half_perimeters_across_macros() {
    let ms = [mac(&[(0, 0, 0), (1, 10, 0)]), mac(&[(0, 30, 40), (1, 10, 7)])];
    assert_eq!(group_hpwl(&ms), 70 + 7);
}

#[test]
fn group_hpwl_uses_pin_centres() {
    let mut m = mac(&[(0, 0, 0), (0, 100, 0)]);
    m.pins[0].at.w = 40;
    assert_eq!(group_hpwl(&[m]), 80);
}

/// A span wider than `i32::MAX` is measured in i64, not wrapped.
#[test]
fn group_hpwl_does_not_overflow_on_a_huge_span() {
    let m = mac(&[(0, -2_000_000_000, 0), (0, 2_000_000_000, 0)]);
    assert_eq!(group_hpwl(&[m]), 4_000_000_000);
}

// --- order_by_priority / symmetric_nets / analog_tiers ----------------------

#[test]
fn order_of_no_nets_is_empty() {
    assert!(order_by_priority(&[], &[], &Requirements::<Routes>::default(), &[], &[]).is_empty());
}

#[test]
fn free_nets_order_by_weight_then_terminals_then_index() {
    let reqs = Requirements::<Routes>::default();
    assert_eq!(order_by_priority(&[3, 2, 2, 2], &[0, 1, 2, 3], &reqs, &[0.5, 0.5, 0.9], &[]), vec![2, 1, 0, 3]);
}

/// `-0.0` is no weight at all: it must not outrank a positive weight.
#[test]
fn negative_zero_weight_does_not_outrank_a_positive_one() {
    let reqs = Requirements::<Routes>::default();
    assert_eq!(order_by_priority(&[2, 2], &[0, 1], &reqs, &[-0.0, 0.5], &[]), vec![1, 0]);
}

#[test]
fn symmetric_nets_lists_differential_members() {
    use analog::routing::Differential;
    let mut reqs = Requirements::<Routes>::default();
    assert!(symmetric_nets(&reqs).is_empty());
    reqs.hard.push(Box::new(vec![Differential { pos: NetId(3), neg: NetId(4), max_len_delta_pct10: 50, same_layer_required: true, stack: None, aggressor_weight: None }]));
    let mut s = symmetric_nets(&reqs);
    s.sort_unstable();
    assert_eq!(s, vec![3, 4]);
}

#[test]
fn analog_tiers_without_rules_is_clean() {
    let (hard, budget) = analog_tiers(&Routes::default(), &Requirements::default());
    assert!(hard.is_empty() && budget.is_empty());
}

// --- LatticeMap / TrackGrid ------------------------------------------------

#[test]
fn lattice_map_inverse() {
    assert_eq!(LatticeMap::Shift { dx: 2, dy: -3 }.inverse(), LatticeMap::Shift { dx: -2, dy: 3 });
    assert_eq!(LatticeMap::MirrorX { k: 7 }.inverse(), LatticeMap::MirrorX { k: 7 });
}

#[test]
fn new_grid_dimensions_and_coarsening() {
    let g = TrackGrid::new((1_000, 1_000), 100, Vec::new(), 4.0);
    assert_eq!((g.nx, g.ny, g.pitch, g.n_layers, g.coarsened), (10, 10, 100, 1, false));
    let g = TrackGrid::new((2_400_000, 100), 100, vec![spec(1, 0, 0); 3], 4.0);
    assert_eq!((g.nx, g.ny, g.pitch, g.n_layers, g.coarsened), (1_200, 2, 2_000, 3, true));
    let g = TrackGrid::new((0, 0), 0, Vec::new(), 4.0);
    assert_eq!((g.nx, g.ny, g.pitch), (2, 2, 1), "degenerate die: 2×2 nodes, pitch 1");
}

#[test]
fn with_layers_has_at_least_one_layer() {
    assert_eq!(grid(4, 4, 0).n_layers, 1);
    assert_eq!(grid(4, 4, 3).nodes(), 48);
}

#[test]
fn stride_defaults_to_one_and_floors_zero() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(0, 0, 0), spec(3, 0, 0)], 4.0);
    assert_eq!((g.stride(0), g.stride(1), g.stride(7)), (1, 3, 1));
}

#[test]
fn node_and_ixy_round_trip() {
    let g = grid(7, 5, 3);
    for n in 0..g.nodes() as u32 {
        let (x, y, l) = g.ixy(n);
        assert_eq!(g.node(x, y, l), n);
    }
    assert_eq!(g.layer_size(), 35);
}

#[test]
fn bins_clamp_to_the_grid() {
    let g = grid(10, 10, 1);
    assert_eq!((g.bin_x(-5_000), g.bin_x(0), g.bin_x(99), g.bin_x(100), g.bin_x(1_000_000)), (0, 0, 0, 1, 9));
    assert_eq!((g.bin_y(-1), g.bin_y(950)), (0, 9));
}

#[test]
fn lattice_map_checks_every_layer() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(2, 0, 0), spec(3, 0, 0)], 4.0);
    assert!(!g.lattice_map(LatticeMap::MirrorX { k: -3 }), "negative k");
    assert!(g.lattice_map(LatticeMap::MirrorX { k: 6 }));
    assert!(!g.lattice_map(LatticeMap::MirrorX { k: 4 }), "4 is not a multiple of the vertical stride 3");
    assert!(g.lattice_map(LatticeMap::Shift { dx: -3, dy: -2 }));
    assert!(!g.lattice_map(LatticeMap::Shift { dx: 3, dy: 1 }));
}

#[test]
fn on_track_follows_stride_across_the_direction() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(2, 0, 0), spec(2, 0, 0)], 4.0);
    assert!(g.on_track(g.node(3, 4, 0)) && !g.on_track(g.node(4, 3, 0)), "horizontal: rows");
    assert!(g.on_track(g.node(4, 3, 1)) && !g.on_track(g.node(3, 4, 1)), "vertical: columns");
}

// --- candidates ------------------------------------------------------------

#[test]
fn candidates_walk_rings_then_layers() {
    let g = grid(10, 10, 2);
    let claimed = vec![false; g.nodes()];
    let c = g.candidates(550, 550, &claimed, |_| true, 1, 100);
    assert_eq!(c.len(), 18);
    assert_eq!(c[0], g.node(5, 5, 0));
    assert_eq!(c[1], g.node(4, 4, 0), "ring 1, row-major");
    assert_eq!(c[9], g.node(5, 5, 1));
    assert_eq!(g.candidates(550, 550, &claimed, |_| true, 1, 1), vec![g.node(5, 5, 0)]);
}

#[test]
fn candidates_with_k_zero_is_empty() {
    let g = grid(10, 10, 2);
    assert!(g.candidates(550, 550, &vec![false; g.nodes()], |_| true, 3, 0).is_empty());
}

#[test]
fn candidates_with_negative_rmax_is_empty() {
    let g = grid(10, 10, 2);
    assert!(g.candidates(550, 550, &vec![false; g.nodes()], |_| true, -1, 5).is_empty());
}

#[test]
fn candidates_skip_claimed_rejected_and_stacked() {
    let g = grid(10, 10, 2);
    let mut claimed = vec![false; g.nodes()];
    claimed[g.node(5, 5, 0) as usize] = true;
    claimed[g.node(5, 3, 1) as usize] = true;
    let c = g.candidates(550, 550, &claimed, |n| n != g.node(4, 4, 0), 1, 100);
    assert!(!c.contains(&g.node(5, 5, 0)) && !c.contains(&g.node(4, 4, 0)));
    assert!(!c.contains(&g.node(5, 5, 1)) && !c.contains(&g.node(5, 4, 1)), "within two rows of a claimed via column");
    assert!(c.contains(&g.node(5, 6, 1)) && c.contains(&g.node(4, 5, 1)));
}

#[test]
fn candidates_clamp_a_far_pin_into_the_corner() {
    let g = grid(10, 10, 2);
    let c = g.candidates(-5_000, 99_999, &vec![false; g.nodes()], |_| true, 0, 10);
    assert_eq!(c, vec![g.node(0, 9, 0), g.node(0, 9, 1)]);
}

#[test]
fn candidates_skip_off_track_nodes() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(2, 0, 0)], 4.0);
    let c = g.candidates(550, 550, &vec![false; g.nodes()], |_| true, 1, 100);
    assert!(c.iter().all(|&n| g.ixy(n).1 % 2 == 0), "{c:?}");
    assert_eq!(c.len(), 6, "rows 4 and 6, three columns each");
}

// --- RGraph for TrackGrid --------------------------------------------------

fn edges(g: &TrackGrid, n: u32) -> Vec<(u32, f32)> {
    let mut buf = [(0u32, 0.0f32); 6];
    let k = g.neighbors(n, &mut buf);
    buf[..k].to_vec()
}

#[test]
fn neighbor_counts_at_corner_interior_and_middle_layer() {
    let g = grid(10, 10, 3);
    let corner = edges(&g, g.node(0, 0, 0));
    assert_eq!(corner, vec![(g.node(1, 0, 0), 1.0), (g.node(0, 1, 0), WRONG_WAY_COST), (g.node(0, 0, 1), 4.0)]);
    assert_eq!(edges(&g, g.node(5, 5, 0)).len(), 5);
    assert_eq!(edges(&g, g.node(5, 5, 1)).len(), 6);
    assert_eq!(edges(&g, g.node(9, 9, 2)).len(), 3);
}

#[test]
fn vertical_layer_runs_along_y() {
    let g = grid(10, 10, 2);
    let e = edges(&g, g.node(5, 5, 1));
    assert!(e.contains(&(g.node(5, 4, 1), 1.0)) && e.contains(&(g.node(5, 6, 1), 1.0)));
    assert!(e.contains(&(g.node(4, 5, 1), WRONG_WAY_COST)));
}

#[test]
fn pos_is_the_bin_centre() {
    let g = grid(10, 10, 2);
    assert_eq!(g.pos(g.node(0, 0, 0)), (50, 50, 0));
    assert_eq!(g.pos(g.node(9, 3, 1)), (950, 350, 1));
}

#[test]
fn beside_and_stacked_at_edges() {
    let g = grid(10, 10, 3);
    let mut out = [0u32; 2];
    let n = g.node(4, 0, 0);
    assert_eq!(g.beside(n, &mut out), 1);
    assert_eq!(out[0], g.node(4, 1, 0));
    let n = g.node(9, 4, 1);
    assert_eq!(g.beside(n, &mut out), 1);
    assert_eq!(out[0], g.node(8, 4, 1));
    assert_eq!(g.stacked(g.node(1, 1, 1), &mut out), 2);
    assert_eq!(out, [g.node(1, 1, 0), g.node(1, 1, 2)]);
    assert_eq!(grid(4, 4, 1).stacked(0, &mut out), 0);
}

#[test]
fn map_leaves_the_grid_as_none() {
    let g = grid(10, 10, 2);
    assert_eq!(g.map(LatticeMap::MirrorX { k: 9 }, g.node(0, 3, 1)), Some(g.node(9, 3, 1)));
    assert_eq!(g.map(LatticeMap::Shift { dx: 1, dy: 0 }, g.node(9, 3, 0)), None);
    assert_eq!(g.map(LatticeMap::Shift { dx: 0, dy: -1 }, g.node(2, 0, 0)), None);
    assert_eq!(g.map(LatticeMap::Shift { dx: i32::MAX, dy: 0 }, g.node(2, 0, 0)), None, "no overflow");
}

#[test]
fn within_counts_the_clipped_box() {
    let g = grid(10, 10, 1);
    let count = |n, t| {
        let mut v = Vec::new();
        g.within(n, t, &mut v);
        v
    };
    assert!(count(g.node(5, 5, 0), 0).is_empty());
    assert_eq!(count(g.node(5, 5, 0), 1).len(), 8);
    assert_eq!(count(g.node(0, 0, 0), 1).len(), 3);
    assert_eq!(count(g.node(0, 0, 0), 1_000).len(), 99, "a huge t clips to the whole layer");
}

#[test]
fn within_appends() {
    let g = grid(10, 10, 1);
    let mut v = vec![7];
    g.within(g.node(0, 0, 0), 1, &mut v);
    assert_eq!(v[0], 7);
    assert_eq!(v.len(), 4);
}

#[test]
fn lower_bound_is_manhattan_plus_vias() {
    let g = grid(10, 10, 2);
    assert_eq!(g.lower_bound(g.node(0, 0, 0), g.node(3, 4, 1)), 11.0);
    assert_eq!(g.lower_bound(g.node(3, 4, 1), g.node(3, 4, 1)), 0.0);
}

#[test]
fn footprint_spans_k_tracks_plus_guards_in_bounds() {
    let g = grid(10, 10, 2);
    let mut out = vec![99];
    assert!(g.footprint(g.node(5, 8, 0), 2, 1, &mut out));
    assert_eq!(out, vec![g.node(5, 7, 0), g.node(5, 8, 0), g.node(5, 9, 0)], "the guard past the edge is dropped, out cleared");
    assert!(!g.footprint(g.node(5, 8, 0), 3, 0, &mut out), "a drawn track off the graph");
    assert!(g.footprint(g.node(5, 8, 0), 0, 0, &mut out));
    assert_eq!(out, vec![g.node(5, 8, 0)], "k = 0 is one track");
    assert!(g.footprint(g.node(8, 5, 1), 2, 0, &mut out));
    assert_eq!(out, vec![g.node(8, 5, 1), g.node(9, 5, 1)], "vertical layers step x");
}

#[test]
fn footprint_steps_by_stride() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(2, 0, 0)], 4.0);
    let mut out = Vec::new();
    assert!(g.footprint(g.node(5, 2, 0), 2, 0, &mut out));
    assert_eq!(out, vec![g.node(5, 2, 0), g.node(5, 4, 0)]);
}

#[test]
fn via_block_covers_the_corner_on_both_layers() {
    let g = grid(10, 10, 2);
    let (a, b) = (g.node(3, 3, 0), g.node(3, 3, 1));
    let mut out = Vec::new();
    assert!(g.via_block(a, b, 1, 1, &mut out));
    assert_eq!(out, vec![a, b]);
    out.clear();
    assert!(g.via_block(a, b, 2, 2, &mut out));
    let mut want: Vec<u32> = [0, 1].iter().flat_map(|&l| [g.node(3, 3, l), g.node(4, 3, l), g.node(3, 4, l), g.node(4, 4, l)]).collect();
    want.sort_unstable();
    out.sort_unstable();
    assert_eq!(out, want);
    assert!(!g.via_block(g.node(9, 9, 0), g.node(9, 9, 1), 2, 1, &mut Vec::new()));
}

#[test]
fn via_halo_clips_at_the_edge() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(1, 0, 1), spec(1, 0, 1)], 4.0);
    let mut out = Vec::new();
    g.via_halo(g.node(0, 5, 0), g.node(0, 5, 1), &mut out);
    out.sort_unstable();
    assert_eq!(out, vec![g.node(1, 5, 0), g.node(0, 4, 1), g.node(0, 6, 1)]);
}

#[test]
fn halo_nodes_without_specs_is_empty() {
    let g = grid(10, 10, 2);
    let mut out = Vec::new();
    g.halo_nodes(&[vec![g.node(1, 1, 0), g.node(2, 1, 0), g.node(2, 1, 1)]], &mut out);
    assert!(out.is_empty());
}

#[test]
fn halo_nodes_marks_both_run_ends() {
    let g = TrackGrid::new((1_000, 1_000), 100, vec![spec(1, 2, 0)], 4.0);
    let mut out = Vec::new();
    g.halo_nodes(&[vec![g.node(5, 5, 0), g.node(4, 5, 0), g.node(3, 5, 0)], vec![g.node(7, 7, 0)]], &mut out);
    out.sort_unstable();
    assert_eq!(out, vec![g.node(1, 5, 0), g.node(2, 5, 0), g.node(6, 5, 0), g.node(7, 5, 0)], "a reversed run, and a single node casts none");
}

// --- RouteHot --------------------------------------------------------------

#[test]
fn commit_moves_usage_and_dedups_the_footprint() {
    let mut hot = RouteHot::new(10, 2);
    hot.commit(0, vec![vec![1, 2], vec![2, 3]], Vec::new(), Vec::new());
    assert_eq!(hot.tree_nodes(0), vec![1, 2, 3]);
    assert_eq!(&hot.usage[..5], &[0, 1, 1, 1, 0]);
    hot.commit(0, vec![vec![4]], Vec::new(), vec![5, 6]);
    assert_eq!(&hot.usage[..5], &[0, 0, 0, 0, 1]);
    assert_eq!((hot.halo[5], hot.halo[6]), (1, 1));
    hot.commit(0, vec![vec![4]], Vec::new(), vec![6]);
    assert_eq!((hot.halo[5], hot.halo[6]), (0, 1));
    hot.commit(0, Vec::new(), Vec::new(), Vec::new());
    assert!(hot.usage.iter().all(|&u| u == 0) && hot.halo.iter().all(|&h| h == 0));
}

#[test]
fn commit_with_an_explicit_footprint_counts_it() {
    let mut hot = RouteHot::new(10, 1);
    hot.commit(0, vec![vec![1, 2]], vec![1, 2, 7], Vec::new());
    assert_eq!(hot.usage[7], 1);
    assert_eq!(hot.trees[0], vec![vec![1, 2]]);
}

#[test]
fn set_weights_sums_per_node_and_commit_keeps_it() {
    let mut hot = RouteHot::new(10, 2);
    hot.commit(0, vec![vec![1, 2]], Vec::new(), Vec::new());
    hot.commit(1, vec![vec![2]], Vec::new(), Vec::new());
    hot.set_weights(vec![0.5, 0.25], vec![2.0]);
    assert_eq!((hot.sens[1], hot.sens[2]), (0.5, 0.75));
    assert_eq!((hot.agg[1], hot.agg[2]), (2.0, 3.0), "a missing aggressor weight is 1.0");
    hot.commit(1, vec![vec![3]], Vec::new(), Vec::new());
    assert_eq!((hot.sens[2], hot.sens[3]), (0.5, 0.25));
    assert_eq!((hot.agg[2], hot.agg[3]), (2.0, 1.0));
    hot.set_weights(vec![1.0], Vec::new());
    assert!(hot.agg.is_empty());
    assert_eq!(hot.sens[3], 0.0, "a missing weight is 0");
}

#[test]
fn over_counts_halos_only_under_metal() {
    let mut hot = RouteHot::new(3, 1);
    hot.halo = vec![5, 1, 0];
    hot.usage = vec![0, 1, 3];
    assert_eq!((hot.over(0, 1), hot.over(1, 1), hot.over(2, 1)), (0, 1, 2));
}

#[test]
fn over_does_not_overflow() {
    let mut hot = RouteHot::new(1, 1);
    hot.usage[0] = u16::MAX;
    hot.halo[0] = 2;
    assert_eq!(hot.over(0, 1), u16::MAX);
}

// --- RouteCtx --------------------------------------------------------------

#[test]
fn branch_k_reads_the_terminal_reached() {
    let g = grid(10, 10, 1);
    let (a, b) = (g.node(0, 0, 0), g.node(5, 0, 0));
    let mut cold = RouteCtx::new(g, vec![vec![a, b], vec![a]], vec![0, 1]);
    cold.k = vec![[3; MAX_LAYERS]];
    cold.term_k = vec![vec![ONE, [2; MAX_LAYERS]]];
    assert_eq!(cold.branch_k(0, &[a, b]), [2; MAX_LAYERS]);
    assert_eq!(cold.branch_k(0, &[b, a]), ONE);
    assert_eq!(cold.branch_k(0, &[7]), [3; MAX_LAYERS], "not a terminal: the net's k");
    assert_eq!(cold.branch_k(0, &[]), [3; MAX_LAYERS]);
    assert_eq!(cold.branch_k(1, &[a]), ONE, "past the k table: one track");
}

#[test]
fn map_tree_fails_when_a_node_leaves() {
    let g = grid(10, 10, 1);
    let tree = vec![vec![g.node(1, 1, 0), g.node(2, 1, 0)]];
    let cold = RouteCtx::new(g, vec![], vec![]);
    let up = cold.map_tree(&tree, LatticeMap::Shift { dx: 0, dy: 2 }).unwrap();
    assert_eq!(up, vec![vec![cold.graph.node(1, 3, 0), cold.graph.node(2, 3, 0)]]);
    assert!(cold.map_tree(&tree, LatticeMap::Shift { dx: 0, dy: 9 }).is_none());
    assert_eq!(cold.map_tree(&[], LatticeMap::Shift { dx: 0, dy: 9 }), Some(Vec::new()));
}

#[test]
fn committing_one_side_of_a_pair_commits_the_image() {
    let g = grid(10, 10, 1);
    let tree = vec![vec![g.node(1, 1, 0), g.node(2, 1, 0)]];
    let image = vec![vec![g.node(1, 3, 0), g.node(2, 3, 0)]];
    let mut cold = RouteCtx::new(g, vec![vec![], vec![]], vec![0, 1]);
    let up = LatticeMap::Shift { dx: 0, dy: 2 };
    cold.mirror = vec![Some((1, up, true)), Some((0, up.inverse(), false))];
    let mut hot = RouteHot::new(cold.graph.nodes(), 2);
    cold.commit(&mut hot, 0, tree.clone());
    assert_eq!((&hot.trees[0], &hot.trees[1]), (&tree, &image));
    cold.commit(&mut hot, 1, vec![vec![cold.graph.node(5, 1, 0)]]);
    assert!(hot.trees[0].is_empty(), "the image falls off the graph: the partner is cleared");
}

#[test]
fn a_wide_commit_claims_every_track() {
    let g = grid(10, 10, 1);
    let tree = vec![vec![g.node(1, 1, 0), g.node(2, 1, 0)]];
    let mut cold = RouteCtx::new(g, vec![vec![]], vec![0]);
    cold.k = vec![[2; MAX_LAYERS]];
    let mut hot = RouteHot::new(cold.graph.nodes(), 1);
    cold.commit(&mut hot, 0, tree);
    let n = |x, y| cold.graph.node(x, y, 0);
    assert_eq!(hot.foot[0], vec![n(1, 1), n(2, 1), n(1, 2), n(2, 2)]);
}

// --- route_net -------------------------------------------------------------

#[test]
fn route_net_degenerate_terminal_sets() {
    let g = grid(5, 2, 1);
    let hot = RouteHot::new(g.nodes(), 1);
    let mut dij = Dij::new(g.nodes());
    assert_eq!(route_net(&g, &hot, &[], &query(&[]), 1.0, &mut dij), Some(Vec::new()));
    assert_eq!(route_net(&g, &hot, &[], &query(&[3]), 1.0, &mut dij), Some(vec![vec![3]]));
    let tree = route_net(&g, &hot, &[], &query(&[0, 1, 1, 0]), 1.0, &mut dij).unwrap();
    assert_eq!(tree, vec![vec![0], vec![0, 1]], "repeated terminals add no branch");
}

#[test]
fn route_net_branches_start_on_the_tree_and_end_at_the_target() {
    let g = grid(10, 10, 2);
    let terms = [g.node(0, 0, 0), g.node(9, 0, 0), g.node(9, 9, 0)];
    let hot = RouteHot::new(g.nodes(), 1);
    let tree = route_net(&g, &hot, &[], &query(&terms), 1.0, &mut Dij::new(g.nodes())).unwrap();
    assert_eq!(tree.len(), 3);
    let mut seen = vec![terms[0]];
    for b in &tree[1..] {
        assert!(seen.contains(&b[0]), "a branch starts on the tree");
        assert!(terms.contains(b.last().unwrap()));
        seen.extend_from_slice(b);
    }
    assert_eq!(tree[1].last(), Some(&terms[1]), "the nearer target first");
}

#[test]
fn reserved_and_blocked_nodes_wall_a_net_off() {
    let g = grid(5, 2, 1);
    let terms = [g.node(0, 0, 0), g.node(4, 0, 0)];
    let hot = RouteHot::new(g.nodes(), 1);
    let mut dij = Dij::new(g.nodes());
    let mut reserved = vec![NONE; g.nodes()];
    reserved[g.node(2, 0, 0) as usize] = 7;
    reserved[g.node(2, 1, 0) as usize] = 7;
    assert!(route_net(&g, &hot, &reserved, &query(&terms), 1.0, &mut dij).is_none());
    let q = NetSearch { owner: 7, ..query(&terms) };
    assert!(route_net(&g, &hot, &reserved, &q, 1.0, &mut dij).is_some(), "reserved for its owner id");
    let wall = [g.node(2, 0, 0), g.node(2, 1, 0)];
    let q = NetSearch { blocked: &wall, ..query(&terms) };
    assert!(route_net(&g, &hot, &[], &q, 1.0, &mut dij).is_none());
}

/// Repeated searches on one scratch give the same tree as a fresh scratch.
#[test]
fn reused_scratch_matches_a_fresh_one() {
    let g = grid(12, 12, 2);
    let hot = RouteHot::new(g.nodes(), 1);
    let terms = [g.node(1, 1, 0), g.node(10, 7, 0), g.node(3, 11, 0)];
    let fresh = route_net(&g, &hot, &[], &query(&terms), 1.0, &mut Dij::new(g.nodes()));
    let mut dij = Dij::new(g.nodes());
    for _ in 0..5 {
        assert_eq!(route_net(&g, &hot, &[], &query(&terms), 1.0, &mut dij), fresh);
    }
    assert!(dij.pops > 0);
}

#[test]
fn reroute_of_a_lone_net_is_route_net() {
    let g = grid(10, 10, 2);
    let terms = vec![g.node(0, 0, 0), g.node(6, 4, 0)];
    let cold = RouteCtx::new(g, vec![terms.clone()], vec![0]);
    let hot = RouteHot::new(cold.graph.nodes(), 1);
    let a = cold.reroute(&hot, 0, 1.0, &[], &mut Dij::new(cold.graph.nodes()));
    let b = route_net(&cold.graph, &hot, &[], &query(&terms), 1.0, &mut Dij::new(cold.graph.nodes()));
    assert_eq!(a, b);
}

// --- run_pathfinder --------------------------------------------------------

#[test]
fn zero_iterations_report_the_current_overflow() {
    let g = grid(5, 2, 1);
    let cold = RouteCtx::new(g, vec![vec![0, 1], vec![1, 2]], vec![0, 1]);
    let mut hot = RouteHot::new(cold.graph.nodes(), 2);
    hot.commit(0, vec![vec![0, 1]], Vec::new(), Vec::new());
    hot.commit(1, vec![vec![1, 2]], Vec::new(), Vec::new());
    assert_eq!(run_pathfinder(&mut hot, &cold, 1.0, 0.5, 0, &mut Dij::new(cold.graph.nodes())), (1.0, 0));
}

#[test]
fn a_clean_state_stops_after_one_iteration() {
    let g = grid(5, 2, 1);
    let cold = RouteCtx::new(g, vec![vec![0, 1]], vec![0]);
    let mut hot = RouteHot::new(cold.graph.nodes(), 1);
    hot.commit(0, vec![vec![0], vec![0, 1]], Vec::new(), Vec::new());
    assert_eq!(run_pathfinder(&mut hot, &cold, 1.0, 0.5, 9, &mut Dij::new(cold.graph.nodes())), (0.0, 1));
    assert!(hot.hist.iter().all(|&h| h == 0.0));
}

#[test]
fn nets_without_terminals_are_skipped() {
    let g = grid(5, 2, 1);
    let cold = RouteCtx::new(g, vec![vec![]], vec![0]);
    let mut hot = RouteHot::new(cold.graph.nodes(), 1);
    assert_eq!(run_pathfinder(&mut hot, &cold, 1.0, 0.5, 9, &mut Dij::new(cold.graph.nodes())), (0.0, 1));
    assert!(hot.trees[0].is_empty());
}

/// A net whose footprint (not its tree) is over capacity is dirty: wide
/// guard tracks claim nodes the branch nodes do not list.
#[test]
fn an_overused_footprint_node_makes_its_net_dirty() {
    let g = grid(5, 2, 1);
    let cold = RouteCtx::new(g, vec![vec![0, 1], vec![]], vec![0]);
    let mut hot = RouteHot::new(cold.graph.nodes(), 2);
    hot.commit(0, vec![vec![0], vec![0, 1]], vec![0, 1, 7], Vec::new());
    hot.commit(1, vec![vec![7]], Vec::new(), Vec::new());
    let (over, _) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 1, &mut Dij::new(cold.graph.nodes()));
    assert_eq!(over, 0.0, "net 0 rerouted off node 7");
}

#[test]
fn history_grows_only_on_overused_nodes() {
    let g = grid(5, 1, 1);
    let cold = RouteCtx::new(g, vec![vec![0, 2], vec![2, 4]], vec![0, 1]);
    let mut hot = RouteHot::new(cold.graph.nodes(), 2);
    run_pathfinder(&mut hot, &cold, 1.0, 0.5, 3, &mut Dij::new(cold.graph.nodes()));
    assert!(hot.hist[2] > 0.0);
    assert!(hot.hist.iter().enumerate().all(|(i, &h)| i == 2 || h == 0.0), "{:?}", hot.hist);
}

// --- extract_geometry / to_shapes -------------------------------------------

fn one_track(_: usize, _: &[u32]) -> [u8; MAX_LAYERS] {
    ONE
}

fn hot_with(g: &TrackGrid, trees: Vec<Vec<Vec<u32>>>) -> RouteHot {
    let mut hot = RouteHot::new(g.nodes(), trees.len());
    for (net, t) in trees.into_iter().enumerate() {
        hot.commit(net, t, Vec::new(), Vec::new());
    }
    hot
}

#[test]
fn a_single_node_tree_draws_nothing() {
    let g = grid(10, 10, 2);
    let hot = hot_with(&g, vec![vec![vec![g.node(3, 3, 0)]], vec![]]);
    let (w, v) = extract_geometry(&hot, &g, &[10, 10], &one_track, &[]);
    assert!(w.is_empty() && v.is_empty());
}

#[test]
fn a_straight_run_is_one_centre_line_wire() {
    let g = grid(10, 10, 2);
    let hot = hot_with(&g, vec![vec![vec![g.node(1, 1, 0), g.node(2, 1, 0), g.node(3, 1, 0)]]]);
    let (w, v) = extract_geometry(&hot, &g, &[30], &one_track, &[]);
    assert!(v.is_empty());
    assert_eq!(w.len(), 1);
    assert_eq!((w[0].net, w[0].layer, w[0].x0, w[0].y0, w[0].x1, w[0].y1, w[0].width, w[0].across), (0, 0, 150, 150, 350, 150, 30, 0));
}

#[test]
fn widths_fall_back_to_the_last_entry_then_zero() {
    let g = grid(10, 10, 2);
    let hot = hot_with(&g, vec![vec![vec![g.node(1, 1, 1), g.node(1, 2, 1)]]]);
    assert_eq!(extract_geometry(&hot, &g, &[7], &one_track, &[]).0[0].width, 7);
    assert_eq!(extract_geometry(&hot, &g, &[], &one_track, &[]).0[0].width, 0);
}

#[test]
fn a_layer_change_draws_one_via_at_the_lower_width() {
    let g = grid(10, 10, 2);
    let tree = vec![vec![g.node(0, 1, 0), g.node(1, 1, 0), g.node(1, 1, 1), g.node(1, 2, 1)]];
    let hot = hot_with(&g, vec![tree]);
    let (w, v) = extract_geometry(&hot, &g, &[20, 40], &one_track, &[]);
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].x, v[0].y, v[0].layer, v[0].size), (150, 150, 0, 20));
    assert_eq!(w.len(), 2);
    assert_eq!((w[1].layer, w[1].x0, w[1].y0, w[1].x1, w[1].y1, w[1].width), (1, 150, 150, 150, 250, 40));
}

#[test]
fn vias_dedupe_by_position_and_layer() {
    let g = grid(10, 10, 2);
    let via = vec![vec![g.node(1, 1, 0), g.node(1, 1, 1)]];
    let hot = hot_with(&g, vec![via.clone(), via.clone(), via]);
    let (_, v) = extract_geometry(&hot, &g, &[10], &one_track, &[]);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].net, 0, "the lowest net is kept");
}

#[test]
fn a_two_track_run_merges_below_the_wide_threshold_and_splits_at_it() {
    let g = grid(10, 10, 2);
    let hot = hot_with(&g, vec![vec![vec![g.node(1, 1, 0), g.node(4, 1, 0)]]]);
    let two = |_: usize, _: &[u32]| [2; MAX_LAYERS];
    let (w, _) = extract_geometry(&hot, &g, &[30], &two, &[131]);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].across, 100);
    let (w, _) = extract_geometry(&hot, &g, &[30], &two, &[130]);
    assert_eq!(w.len(), 4, "two tracks plus an end cap at each end");
    assert!(w.iter().all(|x| x.across == 0));
    assert!(w.iter().any(|x| x.y0 == 250 && x.y1 == 250), "the second track one pitch up");
}

#[test]
fn to_shapes_widens_centre_lines_and_centres_vias() {
    let wires = [Wire { net: 0, layer: 0, x0: 100, y0: 0, x1: 0, y1: 0, width: 10, across: 20 }, Wire { net: 1, layer: 1, x0: 0, y0: 0, x1: 0, y1: 100, width: 10, across: 20 }];
    let vias = [Via { net: 1, x: 50, y: 50, size: 10, layer: 0 }];
    let s = to_shapes(3, &wires, &vias);
    assert_eq!(s.len(), 3);
    let r = s[0][0].rect;
    assert_eq!((r.x, r.y, r.w, r.h), (-5, -5, 110, 30));
    let r = s[1][0].rect;
    assert_eq!((r.x, r.y, r.w, r.h), (-5, -5, 30, 110));
    let r = s[1][1].rect;
    assert_eq!((s[1][1].layer, r.x, r.y, r.w, r.h), (LayerId(0), 45, 45, 10, 10));
    assert!(s[2].is_empty());
    assert!(to_shapes(0, &[], &[]).is_empty());
}
