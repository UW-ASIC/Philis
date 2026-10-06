//! Post-route metal fill (MFG-01), inside the block only.
//!
//! A density rule is a window sweep (gdsverify `check_density`: one window per
//! step over the layer's extent, always divided by the full window). A rule
//! whose window is wider than the block is chip-level: the block cannot decide
//! it, so it is left to integration and nothing is filled for it. A rule the
//! block covers is answered with fill inside the block's bbox, to the floor
//! and never above the ceiling.
//!
//! Tied, not floating: the extractor makes each floating piece a device-less
//! net (LVS `floating_net`/`unpaired_net`, and an ERC finding on decks that
//! flag unconnected metal). Fill is grown tile by tile out of routed ground
//! wires, so every piece touches ground; a layer whose fill makes any DRC or
//! ERC rule worse, label-free (a ground wire can be a dangling fragment, a
//! tile can break a max-width rule), is dropped.
//!
//! Keep-outs: every foreign shape by the layer's widest spacing, sensitive-net
//! wires by [`KEEP_OUT`] times it, matched cells entirely (Hastings 3e §13.3:
//! fill over matched devices must be symmetric or absent; absent is simple).

use std::collections::VecDeque;

use pnr_core::{LayerId, Macro, Rect, Shape};
use verify::Pdk;

use crate::geometry::{bbox, rect_gap as gap};

/// Clearance from sensitive-net wires, in the layer's widest spacing.
/// ponytail: one multiple for every sensitive net; per-net budgets from the
/// extracted coupling if a net ever needs a tighter bound.
const KEEP_OUT: i32 = 3;
/// Aim this far above the floor and below any ceiling.
const MARGIN: f64 = 0.02;
/// Smallest fill tile, nm: bounds the shape count on a large block.
/// ponytail: one size for every deck.
const TILE: i32 = 2_000;

/// One density rule the block covers.
#[derive(Debug, PartialEq)]
struct Floor {
    /// The routing metal the floor applies to.
    layer: LayerId,
    /// Minimum density, a fraction in `[0, 1]`.
    min: f64,
    /// The layer's density ceiling, 1 when the deck has none.
    max: f64,
}

/// Returns in-block fill for every routing metal with a minimum-density rule
/// whose window fits in the block (the bbox of `drawn`), grown out of
/// `ground` (routed ground-class wires, so every tile is tied) and clear of
/// `sensitive` wires and the `avoid` cells; `None` when nothing is filled.
/// A layer whose tiles add any DRC or ERC finding is dropped whole (logged to
/// stderr). The block average lands at the floor plus a margin, never above
/// the ceiling minus it.
///
/// Cost: one label-free DRC + ERC run on `drawn`, plus one per layer filled.
#[must_use]
pub fn fill(drawn: &[Shape], ground: &[Shape], sensitive: &[Shape], avoid: &[Rect], pdk: &Pdk) -> Option<Macro> {
    // ponytail: the floor is met on the block average, not per window; a
    // sparse corner can still fail a window. Per-window targets are the upgrade.
    let b = bbox(drawn)?;
    let floors = floors(pdk, b);
    if floors.is_empty() {
        return None;
    }
    let base = counts(drawn, pdk);
    let mut out: Vec<Shape> = Vec::new();
    for f in floors {
        let tiles = layer_fill(drawn, ground, sensitive, avoid, pdk, b, &f);
        if tiles.is_empty() {
            continue;
        }
        let all: Vec<Shape> = drawn.iter().chain(&out).chain(&tiles).copied().collect();
        // Per rule, not totals: fill removes the density findings it answers,
        // which would hide a new spacing or width finding in a total.
        if let Some((rule, _)) = counts(&all, pdk).into_iter().find(|(r, n)| *n > base.get(r).copied().unwrap_or(0)) {
            eprintln!("fill: layer {:?} dropped, it adds {rule} findings", f.layer);
            continue;
        }
        out.extend(tiles);
    }
    let bbox = bbox(&out)?;
    Some(Macro { shapes: out, pins: Vec::new(), bbox, units: Vec::new(), dummies: Vec::new(), ..Default::default() })
}

/// Findings per rule id, DRC then ERC, label-free.
fn counts(shapes: &[Shape], pdk: &Pdk) -> std::collections::BTreeMap<String, usize> {
    let mut out = std::collections::BTreeMap::new();
    for f in verify::drc(shapes, &[], pdk).into_iter().chain(verify::erc(shapes, &[], pdk)) {
        *out.entry(f.rule).or_insert(0) += 1;
    }
    out
}

/// Returns tiles on `f.layer`, flooded breadth-first out of the tiles that
/// touch a ground wire, through free tiles, until the block `b`'s average
/// density reaches the target. Tiles are square, lattice-snapped, at least
/// [`TILE`], one spacing and one min width wide; the grid starts at `b`'s
/// corner and drops any partial last column or row.
fn layer_fill(drawn: &[Shape], ground: &[Shape], sensitive: &[Shape], avoid: &[Rect], pdk: &Pdk, b: Rect, f: &Floor) -> Vec<Shape> {
    let lat = 2 * pdk.grid.max(1);
    let clear = spacing(pdk, f.layer);
    // At least a spacing wide, so tiles one apart are legal.
    let t = ((TILE.max(clear).max(pdk.min_width(f.layer.0).unwrap_or(0)) + lat - 1) / lat) * lat;
    let (nx, ny) = ((b.w / t) as usize, (b.h / t) as usize);
    let tile = |i: usize, j: usize| Rect { x: b.x + i as i32 * t, y: b.y + j as i32 * t, w: t, h: t };
    let on = |s: &&Shape| s.layer == f.layer;
    let is_ground = |s: &Shape| ground.iter().any(|g| g.layer == s.layer && g.rect.touches(&s.rect));
    let foreign: Vec<Rect> = drawn.iter().filter(on).filter(|s| !is_ground(s)).map(|s| s.rect).collect();
    let grounds: Vec<Rect> = ground.iter().filter(on).map(|s| s.rect).collect();
    let hot: Vec<Rect> = sensitive.iter().filter(on).map(|s| s.rect).collect();
    // Free: clear of foreign metal, sensitive wires and matched cells, and
    // either touching ground or a spacing clear of it (never a same-net sliver).
    let free = |r: &Rect| {
        foreign.iter().all(|o| gap(r, o) >= clear)
            && hot.iter().all(|o| gap(r, o) >= KEEP_OUT * clear)
            && avoid.iter().all(|o| !overlaps(r, o))
            && grounds.iter().all(|g| r.touches(g) || gap(r, g) >= clear)
    };
    let area = |r: &Rect| f64::from(r.w) * f64::from(r.h);
    // Covered tiles approximate the drawn area without double-counting overlaps.
    let covered = covered_tiles(drawn.iter().filter(on).map(|s| s.rect), b, t, nx, ny);
    let target = (f.min + MARGIN).min(f.max - MARGIN) * area(&b);
    let mut need = target - covered as f64 * area(&tile(0, 0));

    let mut filled = vec![false; nx * ny];
    let mut queue: VecDeque<usize> =
        (0..nx * ny).filter(|&k| grounds.iter().any(|g| tile(k % nx, k / nx).touches(g))).collect();
    let mut seen = vec![false; nx * ny];
    for &k in &queue {
        seen[k] = true;
    }
    let mut out = Vec::new();
    while let Some(k) = queue.pop_front() {
        if need <= 0.0 {
            break;
        }
        let (i, j) = (k % nx, k / nx);
        let r = tile(i, j);
        // No corner-only contact with an earlier tile: it would read as a
        // zero-width neck.
        let at = |di: i64, dj: i64| {
            let (a, c) = (i as i64 + di, j as i64 + dj);
            (0..nx as i64).contains(&a) && (0..ny as i64).contains(&c) && filled[c as usize * nx + a as usize]
        };
        let pinch = [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(di, dj)| at(di, dj) && !at(di, 0) && !at(0, dj));
        // One-tile fingers on every other column (plus the tiles on ground),
        // so no fill blob outgrows a max-width rule.
        let finger = i % 2 == 0 || grounds.iter().any(|g| r.touches(g));
        if !free(&r) || pinch || !finger {
            continue;
        }
        filled[k] = true;
        need -= area(&r);
        out.push(Shape { layer: f.layer, rect: r });
        for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
            let (a, c) = (i as i64 + di, j as i64 + dj);
            if (0..nx as i64).contains(&a) && (0..ny as i64).contains(&c) {
                let n = c as usize * nx + a as usize;
                if !seen[n] {
                    seen[n] = true;
                    queue.push_back(n);
                }
            }
        }
    }
    out
}

/// Returns how many tiles of the `nx` × `ny` grid of side `t` from `b`'s
/// corner share interior area with any of `rects`. O(Σ tiles each rect
/// spans), not O(tiles × rects).
fn covered_tiles(rects: impl Iterator<Item = Rect>, b: Rect, t: i32, nx: usize, ny: usize) -> usize {
    let mut hit = vec![false; nx * ny];
    // Tiles `[lo, hi)` along one axis whose open span meets `(p, p + len)`.
    let span = |p: i32, len: i32, origin: i32, n: usize| {
        let (p0, p1) = (i64::from(p) - i64::from(origin), i64::from(p) + i64::from(len) - i64::from(origin));
        let t = i64::from(t);
        let lo = p0.div_euclid(t).clamp(0, n as i64) as usize;
        let hi = (p1 + t - 1).div_euclid(t).clamp(0, n as i64) as usize;
        lo..hi.max(lo)
    };
    for r in rects {
        for j in span(r.y, r.h, b.y, ny) {
            for i in span(r.x, r.w, b.x, nx) {
                hit[j * nx + i] = true;
            }
        }
    }
    hit.iter().filter(|&&h| h).count()
}

/// The widest spacing `layer` asks of any shape.
fn spacing(pdk: &Pdk, layer: LayerId) -> i32 {
    pdk.wide_spacing(layer.0).iter().map(|w| w.1).fold(pdk.min_spacing(layer.0).unwrap_or(0), i32::max)
}

/// Returns the deck's minimum-density rules on routing metals whose window
/// the block `b` covers, one per layer, each with the layer's tightest
/// ceiling (1 without one); wider windows are chip-level and left out.
fn floors(pdk: &Pdk, b: Rect) -> Vec<Floor> {
    let rules = pdk.density_rules();
    let covers = |r: &&(LayerId, i64, f64, bool)| pdk.routing_metals.contains(&r.0) && r.1 <= i64::from(b.w.min(b.h));
    let mut out: Vec<Floor> = Vec::new();
    for &(layer, _, min, _) in rules.iter().filter(covers).filter(|r| !r.3) {
        match out.iter_mut().find(|f| f.layer == layer) {
            Some(f) => f.min = f.min.max(min),
            None => {
                // Every ceiling counts, chip-level ones too: fill must not push
                // the block over a ceiling integration then checks.
                let max = rules.iter().filter(|m| m.3 && m.0 == layer).map(|m| m.2).fold(1.0, f64::min);
                out.push(Floor { layer, min, max });
            }
        }
    }
    out
}

/// Returns whether `a` and `b` share interior area (open intervals).
fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// gf180's real deck plus a windowed metal1 floor (min 30% per 200 um,
    /// step 100 um): the shipped decks state metal density whole-die only.
    fn deck(name: &str, extra: &str) -> Option<Pdk> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sidecar = std::fs::read_to_string(root.join(format!("pdks/{name}.json"))).ok()?;
        let text = Pdk::deck_text(&sidecar).ok()?;
        let text = format!("{text}\nrule TEST.m1_density density(metal1; window: 200um, step: 100um) >= 30%\n{extra}");
        Pdk::load(&text, &sidecar).ok()
    }

    /// A block smaller than the window: the rule is chip-level, nothing filled.
    #[test]
    fn a_block_narrower_than_the_window_gets_no_fill() {
        let Some(pdk) = deck("gf180mcu", "") else { return };
        let m1 = pdk.routing_metals[0];
        let rail = Shape { layer: m1, rect: Rect { x: 0, y: 0, w: 20_000, h: 1_000 } };
        assert!(fill(&[rail], &[rail], &[], &[], &pdk).is_none());
    }

    /// gf180 (min 30% per 200 µm) over a 200 µm block: a ground rail grows fill
    /// to the floor inside the bbox, every tile clear of the signal wire and
    /// out of the matched cell, and the empty metal2 stays empty.
    #[test]
    fn a_block_wider_than_the_window_is_filled_inside_from_ground() {
        let Some(pdk) = deck("gf180mcu", "") else { return };
        let m1 = pdk.routing_metals[0];
        let side = 200_000;
        let rail = Shape { layer: m1, rect: Rect { x: 0, y: 0, w: side, h: 1_000 } };
        let signal = Shape { layer: m1, rect: Rect { x: 0, y: side - 1_000, w: side, h: 1_000 } };
        let cell = Rect { x: 50_000, y: 50_000, w: 20_000, h: 20_000 };
        let m = fill(&[rail, signal], &[rail], &[signal], &[cell], &pdk).expect("metal1 is below its floor");
        assert!(m.shapes.iter().all(|s| s.layer == m1));
        let f = floors(&pdk, Rect { x: 0, y: 0, w: side, h: side }).into_iter().find(|f| f.layer == m1).unwrap();
        let area: f64 = m.shapes.iter().chain([&rail, &signal]).map(|s| f64::from(s.rect.w) * f64::from(s.rect.h)).sum();
        let d = area / f64::from(side).powi(2);
        assert!(d >= f.min && d <= f.max, "density {d}");
        let clear = spacing(&pdk, m1);
        assert!(m.shapes.iter().all(|s| gap(&s.rect, &signal.rect) >= KEEP_OUT * clear && !overlaps(&s.rect, &cell)));
        assert!(m.bbox.x >= 0 && m.bbox.y >= 0 && m.bbox.x + m.bbox.w <= side, "inside the block");
    }

    /// The filled-from-ground geometry plus a metal1 max width of 1.5 µm: the
    /// 1 µm wires meet it, every ≥ 2 µm tile breaks it, so metal1 is dropped
    /// and nothing is filled (the ERC-only gate kept it).
    #[test]
    fn fill_that_adds_a_drc_finding_is_dropped() {
        let Some(pdk) = deck("gf180mcu", "rule TEST.m1_maxw width(metal1) <= 1.5um\n") else { return };
        let m1 = pdk.routing_metals[0];
        let side = 200_000;
        let rail = Shape { layer: m1, rect: Rect { x: 0, y: 0, w: side, h: 1_000 } };
        let signal = Shape { layer: m1, rect: Rect { x: 0, y: side - 1_000, w: side, h: 1_000 } };
        let cell = Rect { x: 50_000, y: 50_000, w: 20_000, h: 20_000 };
        assert!(fill(&[rail, signal], &[rail], &[signal], &[cell], &pdk).is_none());
    }

    /// gf180 plus `extra` deck lines; `None` only when the sidecar is not in
    /// the checkout, a load failure fails the test.
    fn deck_strict(extra: &str) -> Option<Pdk> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let sidecar = std::fs::read_to_string(root.join("pdks/gf180mcu.json")).ok()?;
        let text = Pdk::deck_text(&sidecar).expect("deck text");
        Some(Pdk::load(&format!("{text}\n{extra}"), &sidecar).expect("deck loads"))
    }

    #[test]
    fn overlaps_is_open() {
        let a = Rect { x: 0, y: 0, w: 10, h: 10 };
        assert!(overlaps(&a, &Rect { x: 9, y: 9, w: 5, h: 5 }));
        assert!(!overlaps(&a, &Rect { x: 10, y: 0, w: 5, h: 5 }), "abutting");
        assert!(!overlaps(&a, &Rect { x: 10, y: 10, w: 5, h: 5 }), "corner");
        assert!(overlaps(&a, &Rect { x: 2, y: 2, w: 1, h: 1 }), "contained");
    }

    /// Several floors on one layer fold into one: the strictest minimum of
    /// the windows the block covers, and the tightest ceiling.
    #[test]
    fn floors_fold_per_layer_and_respect_the_window() {
        let Some(pdk) = deck_strict(
            "rule TEST.a density(metal1; window: 200um, step: 100um) >= 30%\n\
             rule TEST.b density(metal1; window: 100um, step: 50um) >= 40%\n\
             rule TEST.c density(metal1; window: 100um, step: 50um) <= 70%\n",
        ) else {
            return;
        };
        let m1 = pdk.routing_metals[0];
        let block = |side: i32| Rect { x: 0, y: 0, w: side, h: side };
        let m1_floors = |b: Rect| floors(&pdk, b).into_iter().filter(|f| f.layer == m1).collect::<Vec<_>>();
        let f = m1_floors(block(200_000));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!((f[0].min - 0.4).abs() < 1e-9 && (f[0].max - 0.7).abs() < 1e-9, "{f:?}");
        // 150 µm covers only the 100 µm window; 99.999 µm covers none.
        assert!((m1_floors(block(150_000))[0].min - 0.4).abs() < 1e-9);
        assert!(m1_floors(block(99_999)).is_empty());
        // The window must fit on the narrow side.
        assert!(m1_floors(Rect { x: 0, y: 0, w: 500_000, h: 99_999 }).is_empty());
    }

    #[test]
    fn nothing_drawn_is_nothing_filled() {
        let Some(pdk) = deck_strict("") else { return };
        assert!(fill(&[], &[], &[], &[], &pdk).is_none());
    }

    /// Without a ground wire on the layer there is nowhere to tie fill: none.
    #[test]
    fn no_ground_no_fill() {
        let Some(pdk) = deck("gf180mcu", "") else { return };
        let m1 = pdk.routing_metals[0];
        let side = 200_000;
        let signal = Shape { layer: m1, rect: Rect { x: 0, y: 0, w: side, h: 1_000 } };
        let corner = Shape { layer: m1, rect: Rect { x: side - 1_000, y: side - 1_000, w: 1_000, h: 1_000 } };
        assert!(fill(&[signal, corner], &[], &[], &[], &pdk).is_none());
    }

    /// The span-marking count equals the brute-force tile × rect overlap
    /// count, edge-aligned, zero-extent, outside and straddling rects included.
    #[test]
    fn covered_tiles_matches_brute_force() {
        let b = Rect { x: -100, y: 50, w: 1_000, h: 700 };
        let (t, nx, ny) = (100, 10, 7);
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = |m: u64| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % m) as i32
        };
        for _ in 0..300 {
            let k = next(6) as usize;
            let rects: Vec<Rect> = (0..k)
                .map(|_| {
                    // Snap half the corners to the tile lattice to hit edges exactly.
                    let snap = |v: i32, on: bool| if on { v - v.rem_euclid(t) } else { v };
                    let (px, py) = (snap(next(1_400) - 300, next(2) == 0), snap(next(1_000) - 100, next(2) == 0));
                    Rect { x: px, y: py, w: next(400), h: next(400) }
                })
                .collect();
            let tile = |i: usize, j: usize| Rect { x: b.x + i as i32 * t, y: b.y + j as i32 * t, w: t, h: t };
            let brute = (0..nx * ny).filter(|&q| rects.iter().any(|r| overlaps(&tile(q % nx, q / nx), r))).count();
            assert_eq!(covered_tiles(rects.iter().copied(), b, t, nx, ny), brute, "{rects:?}");
        }
        assert_eq!(covered_tiles(std::iter::empty(), b, t, nx, ny), 0);
        assert_eq!(covered_tiles(std::iter::once(b), b, t, 0, 0), 0, "no tiles");
    }
}
