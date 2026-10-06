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
    let covered = (0..nx * ny).filter(|&k| drawn.iter().filter(on).any(|s| overlaps(&tile(k % nx, k / nx), &s.rect))).count();
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

/// The widest spacing `layer` asks of any shape.
fn spacing(pdk: &Pdk, layer: LayerId) -> i32 {
    pdk.wide_spacing(layer.0).iter().map(|w| w.1).fold(pdk.min_spacing(layer.0).unwrap_or(0), i32::max)
}

/// Returns the deck's minimum-density rules on routing metals whose window
/// the block `b` covers, one per layer, each with the layer's tightest
/// ceiling (1 without one); wider windows are chip-level and left out.
fn floors(pdk: &Pdk, b: Rect) -> Vec<Floor> {
    let rules = pdk.density_rules();
    let mut out: Vec<Floor> = rules
        .iter()
        .filter(|r| !r.3 && pdk.routing_metals.contains(&r.0) && r.1 <= i64::from(b.w.min(b.h)))
        .map(|&(layer, _, min, _)| {
            let max = rules.iter().filter(|m| m.3 && m.0 == layer).map(|m| m.2).fold(1.0, f64::min);
            Floor { layer, min, max }
        })
        .collect();
    out.dedup_by_key(|f| f.layer);
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
}
