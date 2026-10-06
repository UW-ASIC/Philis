//! PLC-08 property tests over random symmetric-feasible codes (run in release:
//! 10,000 trees).

use dp::sp::{decode, verify, Geo, Kid, Out, Scratch, Tree};
use gp::mechanics::SplitMix64;
use gp::spacing::SpacingTable;
use pnr_core::DeviceId;

struct Case {
    t: Tree,
    w: Vec<i32>,
    h: Vec<i32>,
    table: SpacingTable,
    /// Self kids an odd number of 10 nm lattice steps wide: no axis on a
    /// `(420, 420)` track centreline can centre them on the lattice.
    odd_self: bool,
}

/// One symmetry node (p ∈ [1, 4] pairs, s ∈ [0, 2] selfs) and up to 8 loose
/// cells; dims multiples of 10 nm in [100, 5000] (partners equal; self widths
/// all even or all odd multiples, the only realisable mixes); one
/// uniform gap in [0, 1270]; with `block`, a proximity node over a random
/// subset (the symmetry node included half the time). Every code random,
/// symmetry nodes then made S-F.
fn case(rng: &mut SplitMix64, block: bool) -> Case {
    let (p, s, loose) = (1 + rng.below(4), rng.below(3), rng.below(9));
    let n = 2 * p + s + loose;
    let odd = rng.below(2) as i32;
    let mut dim = || 100 + 10 * rng.below(491) as i32;
    let (mut w, mut h) = (vec![0; n], vec![0; n]);
    for c in 0..n {
        (w[c], h[c]) = if c < 2 * p && c % 2 == 1 { (w[c - 1], h[c - 1]) } else { (dim(), dim()) };
        if (2 * p..2 * p + s).contains(&c) {
            w[c] = w[c] / 20 * 20 + 10 * odd;
        }
    }
    let mut pairs: Vec<(u32, u32, u16)> = (0..p as u32).map(|i| (2 * i, 2 * i + 1, 0)).collect();
    pairs.extend((2 * p..2 * p + s).map(|c| (c as u32, c as u32, 0)));
    let mut blocks = Vec::new();
    if block {
        let with_sym = rng.below(2) == 0;
        let blk: Vec<DeviceId> =
            (0..n).filter(|&c| if c < 2 * p + s { with_sym } else { rng.below(2) == 0 }).map(|c| DeviceId(c as u16)).collect();
        blocks = vec![blk, Vec::new()];
    }
    let (mut t, errs) = Tree::build(n, &pairs, &blocks);
    assert!(errs.is_empty());
    for ni in 0..t.nodes.len() {
        let nd = &mut t.nodes[ni];
        for seq in [&mut nd.alpha, &mut nd.beta] {
            for i in (1..seq.len()).rev() {
                seq.swap(i, rng.below(i + 1));
            }
        }
        t.make_sf(ni as u16);
    }
    let table = SpacingTable::uniform(10 * rng.below(128) as i32, 10);
    Case { t, w, h, table, odd_self: s > 0 && odd == 1 }
}

fn run(c: &Case) -> (Result<(), dp::sp::Fail>, Out, bool) {
    let prof = vec![None; c.w.len()];
    let g = Geo { w: &c.w, h: &c.h, prof: &prof, halo: &[], table: &c.table, lattice: 10, axis_grid: None };
    let mut out = Out::default();
    let r = decode(&c.t, &g, &mut Scratch::default(), &mut out);
    let ok = r.is_ok() && verify(&c.t, &g, &out);
    (r, out, ok)
}

#[test]
fn random_sf_codes_decode_exactly() {
    let mut rng = SplitMix64::new(7);
    let (mut fails, mut fixed, mut fixes) = (0, 0, 0);
    for i in 0..10_000 {
        let c = case(&mut rng, false);
        assert!(c.t.is_sf(0));
        let (r, out, ok) = run(&c);
        if r.is_err() {
            fails += 1;
            continue;
        }
        assert!(ok, "tree {i}: decoded but verify failed: {:?}", c.t);
        fixed += u32::from(out.fixes > 0);
        fixes += out.fixes;
    }
    eprintln!("random_sf_codes_decode_exactly: {fixed} of 10000 trees ran fix_monotone ({fixes} passes)");
    assert_eq!(fails, 0);
}

/// Cells under node `ni`.
fn cells_under(t: &Tree, ni: usize, out: &mut Vec<usize>) {
    for k in &t.nodes[ni].kids {
        match *k {
            Kid::Cell(c) => out.push(usize::from(c)),
            Kid::Node(m) => cells_under(t, usize::from(m), out),
        }
    }
}

#[test]
fn contiguous_node_has_exclusive_bbox() {
    let mut rng = SplitMix64::new(7);
    let mut checked = 0;
    for _ in 0..2_000 {
        let c = case(&mut rng, true);
        let (r, out, _) = run(&c);
        assert_eq!(r, Ok(()));
        for ni in 0..c.t.nodes.len() - 1 {
            let mut inside = Vec::new();
            cells_under(&c.t, ni, &mut inside);
            let x0 = inside.iter().map(|&i| out.x0[i]).min().unwrap();
            let y0 = inside.iter().map(|&i| out.y0[i]).min().unwrap();
            let x1 = inside.iter().map(|&i| out.x0[i] + c.w[i]).max().unwrap();
            let y1 = inside.iter().map(|&i| out.y0[i] + c.h[i]).max().unwrap();
            for o in (0..c.w.len()).filter(|o| !inside.contains(o)) {
                let hit = out.x0[o] < x1 && x0 < out.x0[o] + c.w[o] && out.y0[o] < y1 && y0 < out.y0[o] + c.h[o];
                assert!(!hit, "cell {o} enters node {ni}'s bbox");
            }
            checked += 1;
        }
    }
    assert!(checked > 1_000, "only {checked} nodes checked");
}

#[test]
fn pairwise_gaps_hold_with_shadows() {
    let mut rng = SplitMix64::new(7);
    for _ in 0..2_000 {
        let b = rng.below(2) == 0;
        let c = case(&mut rng, b);
        let (r, out, _) = run(&c);
        assert_eq!(r, Ok(()));
        let g = c.table.fallback;
        let apart = |p: &[i32], s: &[i32], a: usize, b: usize| {
            let (lo, hi) = if p[a] <= p[b] { (a, b) } else { (b, a) };
            p[hi] - (p[lo] + s[lo]) >= g
        };
        for a in 0..c.w.len() {
            for b in a + 1..c.w.len() {
                // Overlapping y-projections leave only x (and vice versa); a
                // diagonal pair keeps the gap on at least one axis.
                assert!(apart(&out.x0, &c.w, a, b) || apart(&out.y0, &c.h, a, b), "cells {a}, {b} closer than {g}");
            }
        }
    }
}

#[test]
fn every_coordinate_is_on_the_lattice() {
    let mut rng = SplitMix64::new(7);
    for _ in 0..2_000 {
        let b = rng.below(2) == 0;
        let c = case(&mut rng, b);
        let (r, out, _) = run(&c);
        assert_eq!(r, Ok(()));
        assert!(out.x0.iter().chain(&out.y0).all(|v| v % 10 == 0));
        assert!(out.axis.iter().all(|a| (2 * a.1) % 10 == 0));
    }
}

fn run_grid(c: &Case, grid: Option<(i32, i32)>) -> (Result<(), dp::sp::Fail>, Out) {
    let prof = vec![None; c.w.len()];
    let g = Geo { w: &c.w, h: &c.h, prof: &prof, halo: &[], table: &c.table, lattice: 10, axis_grid: grid };
    let mut out = Out::default();
    let r = decode(&c.t, &g, &mut Scratch::default(), &mut out);
    assert!(r.is_err() || verify(&c.t, &g, &out), "decoded but verify failed");
    (r, out)
}

/// PLC-28: with sky130's `(p0, P) = (420, 420)` every axis lands on a track
/// centreline (`210 mod 420`), and every code decodes unless a self kid is an
/// odd number of lattice steps wide (its corner would sit 5 nm off-lattice).
#[test]
fn axes_land_on_track_centrelines() {
    let mut rng = SplitMix64::new(7);
    let mut odd = 0;
    for _ in 0..10_000 {
        let b = rng.below(2) == 0;
        let c = case(&mut rng, b);
        let (r, out) = run_grid(&c, Some((420, 420)));
        if c.odd_self {
            assert!(matches!(r, Err(dp::sp::Fail::SymX(_))), "{r:?}");
            odd += 1;
            continue;
        }
        assert_eq!(r, Ok(()));
        assert!(!out.axis.is_empty() && out.axis.iter().all(|a| a.1.rem_euclid(420) == 210), "{:?}", out.axis);
    }
    assert!(odd > 1_000, "only {odd} odd-self codes");
}

/// Snapping raises a symmetry node's width by less than `2·P`.
#[test]
fn axis_snapping_costs_at_most_one_period_per_level() {
    let mut rng = SplitMix64::new(7);
    let mut hist = [0u32; 9];
    for _ in 0..2_000 {
        let c = case(&mut rng, false);
        if c.odd_self {
            continue;
        }
        let width = |out: &Out| {
            let mut cells = Vec::new();
            cells_under(&c.t, 0, &mut cells);
            let x0 = cells.iter().map(|&i| out.x0[i]).min().unwrap();
            cells.iter().map(|&i| out.x0[i] + c.w[i]).max().unwrap() - x0
        };
        let (free, snapped) = (run_grid(&c, None).1, run_grid(&c, Some((420, 420))).1);
        let grow = width(&snapped) - width(&free);
        assert!((0..2 * 420).contains(&grow), "symmetry node grew {grow} nm");
        hist[(grow / 100) as usize] += 1;
    }
    eprintln!("axis_snapping_costs_at_most_one_period_per_level: width growth by 100 nm bin {hist:?}");
}

/// A self kid with a 40 nm right halo owes its right partner more than its left
/// one, so the first `2·axis` estimate is short: raising the self kid onto it
/// pushes the right member past its mirror and `fix_monotone` must re-snap
/// (twice: the error halves per pass). A 2000 nm halo needs more than the
/// `2p + 2` passes and fails at the cap.
#[test]
fn asymmetric_push_runs_fix_monotone() {
    let (w, h) = (vec![100, 100, 1000], vec![100, 100, 100]);
    let prof = vec![None; 3];
    let table = SpacingTable::uniform(0, 10);
    let (mut t, errs) = Tree::build(3, &[(0, 1, 0), (2, 2, 0)], &[]);
    assert!(errs.is_empty());
    // One row: A (cell 0), self (cell 2), B (cell 1).
    let pos = |c: u16| t.nodes[0].kids.iter().position(|k| matches!(*k, Kid::Cell(x) if x == c)).unwrap() as u16;
    let row = vec![pos(0), pos(2), pos(1)];
    (t.nodes[0].alpha, t.nodes[0].beta) = (row.clone(), row);
    assert!(t.is_sf(0));
    for (halo, want) in [(40, Ok(2)), (2000, Err(dp::sp::Fail::SymX(0)))] {
        let halos = [[0; 4], [0; 4], [0, 0, halo, 0]];
        let g = Geo { w: &w, h: &h, prof: &prof, halo: &halos, table: &table, lattice: 10, axis_grid: None };
        let mut out = Out::default();
        let r = decode(&t, &g, &mut Scratch::default(), &mut out).map(|()| out.fixes);
        assert_eq!(r, want, "halo {halo}");
        if r.is_ok() {
            assert!(verify(&t, &g, &out));
            assert_eq!(out.x0[1] - (out.x0[2] + 1000), 40, "self-B gap");
        }
    }
}
