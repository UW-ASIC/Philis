//! Flatten a placed and routed solution into the shape list signoff and the
//! GDS writer consume.

use pnr_core::{Layout, Macro, Routes, Shape};

/// Every macro stamped at its placement (the one transform, shared with the
/// router via `gr::place_macros`), then every routed wire.
#[must_use]
pub fn collect(macros: &[Macro], layout: &Layout, routes: &Routes) -> Vec<Shape> {
    let mut out = Vec::new();
    for m in gr::place_macros(macros, layout) {
        out.extend(m.shapes);
    }
    for net in &routes.wires {
        out.extend_from_slice(net);
    }
    out
}


/// Merge `layer`'s rects wherever two overlap or abut into exactly one
/// rectangle (same span on one axis, touching on the other), until none do.
/// The drawn area is unchanged; the checker, which reads a well per rect,
/// then sees a bridged well as one.
///
/// ponytail: O(n²) per pass over the layer's rects; wells are few.
pub fn merge_rects(shapes: &mut Vec<Shape>, layer: pnr_core::LayerId) {
    let (mut on, rest): (Vec<Shape>, Vec<Shape>) = shapes.drain(..).partition(|s| s.layer == layer);
    use pnr_core::Rect;
    let joined = |a: Rect, b: Rect| -> Option<Rect> {
        let (x0, x1) = (a.x.min(b.x), (a.x + a.w).max(b.x + b.w));
        let (y0, y1) = (a.y.min(b.y), (a.y + a.h).max(b.y + b.h));
        let rows = a.y == b.y && a.h == b.h && a.x <= b.x + b.w && b.x <= a.x + a.w;
        let cols = a.x == b.x && a.w == b.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
        let inside = |p: Rect, q: Rect| p.x >= q.x && p.y >= q.y && p.x + p.w <= q.x + q.w && p.y + p.h <= q.y + q.h;
        (rows || cols || inside(a, b) || inside(b, a)).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    };
    'again: loop {
        for i in 0..on.len() {
            for j in i + 1..on.len() {
                if let Some(r) = joined(on[i].rect, on[j].rect) {
                    on[i].rect = r;
                    on.swap_remove(j);
                    continue 'again;
                }
            }
        }
        break;
    }
    shapes.extend(rest);
    shapes.extend(on);
}

/// Debug-only: every pin of a net touches that net's routed geometry (xy
/// overlap, touching counts). `Routes::debug_check` only proves the wires are
/// self-connected, which a net can satisfy while missing its pins entirely.
///
/// ponytail: xy-only — ignores whether the touch has a via stack; O(k²) per net.
pub fn debug_check_connected(macros: &[Macro], layout: &Layout, routes: &Routes) {
    if !cfg!(debug_assertions) {
        return;
    }
    let placed = gr::place_macros(macros, layout);
    let n_nets = routes.wires.len().max(
        placed
            .iter()
            .flat_map(|m| &m.pins)
            .map(|p| p.net.0 as usize + 1)
            .max()
            .unwrap_or(0),
    );
    for net in 0..n_nets {
        let pins: Vec<pnr_core::Rect> = placed
            .iter()
            .flat_map(|m| &m.pins)
            .filter(|p| p.net.0 as usize == net)
            .map(|p| p.at)
            .collect();
        // One pin is nothing to connect; zero means the net has no terminals here.
        if pins.len() < 2 {
            continue;
        }
        let wires = routes.wires.get(net).map_or(&[][..], Vec::as_slice);
        assert!(
            !wires.is_empty(),
            "routing: net {net} has {} pins and no routed geometry at all — it was \
             dropped, not routed (see gr::build_nets' obstacle-only path)",
            pins.len()
        );
        // Flood over pins ∪ wires, starting from the first pin.
        let mut boxes: Vec<pnr_core::Rect> = pins.clone();
        boxes.extend(wires.iter().map(|s| s.rect));
        let mut seen = vec![false; boxes.len()];
        let mut stack = vec![0usize];
        seen[0] = true;
        while let Some(a) = stack.pop() {
            for b in 0..boxes.len() {
                if seen[b] {
                    continue;
                }
                let (ra, rb) = (boxes[a], boxes[b]);
                if ra.x <= rb.x + rb.w
                    && rb.x <= ra.x + ra.w
                    && ra.y <= rb.y + rb.h
                    && rb.y <= ra.y + ra.h
                {
                    seen[b] = true;
                    stack.push(b);
                }
            }
        }
        let unreached: Vec<usize> = (0..pins.len()).filter(|&i| !seen[i]).collect();
        if unreached.is_empty() {
            continue;
        }
        // Nearest wire per orphaned pin: a few nm off is a track-snap problem, a
        // few µm off means the router aimed somewhere else entirely.
        let gap = |r: pnr_core::Rect, s: pnr_core::Rect| -> i32 {
            let dx = (s.x - (r.x + r.w)).max(r.x - (s.x + s.w)).max(0);
            let dy = (s.y - (r.y + r.h)).max(r.y - (s.y + s.h)).max(0);
            dx.max(dy)
        };
        let detail: Vec<String> = unreached
            .iter()
            .map(|&i| {
                let p = pins[i];
                let near = wires.iter().min_by_key(|s| gap(p, s.rect));
                match near {
                    Some(s) => format!(
                        "pin[{i}] {p:?} — nearest wire {:?} on layer {:?}, {} nm away",
                        s.rect,
                        s.layer,
                        gap(p, s.rect)
                    ),
                    None => format!("pin[{i}] {p:?} — no wires"),
                }
            })
            .collect();
        panic!(
            "routing: net {net} is open — {} of {} pins unreached by its {} wires:\n  {}",
            unreached.len(),
            pins.len(),
            wires.len(),
            detail.join("\n  "),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Orient, Rect};

    /// An L of two rects, so a turn is detectable (a single rect would look the
    /// same under R180).
    fn ell() -> Macro {
        let shapes = vec![
            Shape {
                layer: LayerId(1),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 400,
                },
            },
            Shape {
                layer: LayerId(1),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 300,
                    h: 100,
                },
            },
        ];
        Macro {
            shapes,
            pins: vec![],
            bbox: Rect {
                x: 0,
                y: 0,
                w: 300,
                h: 400,
            },
            units: Vec::new(),
            dummies: Vec::new(),
        }
    }

    fn layout_of(o: Orient, x: i32, y: i32) -> Layout {
        Layout {
            x: vec![x],
            y: vec![y],
            hw: vec![0],
            hh: vec![0],
            axis: vec![],
            groups: vec![],
            orient: vec![o],
            variant: vec![0],
            branch: vec![],
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        }
    }

    fn bbox_of(shapes: &[Shape]) -> Rect {
        let (mut x0, mut y0) = (i32::MAX, i32::MAX);
        let (mut x1, mut y1) = (i32::MIN, i32::MIN);
        for s in shapes {
            x0 = x0.min(s.rect.x);
            y0 = y0.min(s.rect.y);
            x1 = x1.max(s.rect.x + s.rect.w);
            y1 = y1.max(s.rect.y + s.rect.h);
        }
        Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        }
    }

    /// The invariant the whole flow depends on: drawn geometry must land where
    /// the placer legalised it. `Layout` is centre + half-extents, so the stamped
    /// bbox must be centred on `(x, y)` with those extents — not merely
    /// translated by them. Getting this wrong put guard rings through devices and
    /// shorted supply nets, and it is invisible whenever `hw`/`hh` are zero, so
    /// the extents here are deliberately non-zero.
    #[test]
    fn drawn_bbox_is_centred_on_the_placed_centre() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let m = ell();
        let (hw, hh) = (m.bbox.w / 2, m.bbox.h / 2);
        let mut l = layout_of(Orient::R0, 7_000, 3_000);
        l.hw = vec![hw];
        l.hh = vec![hh];
        let got = bbox_of(&collect(&[m], &l, &routes));
        assert_eq!((got.x, got.y), (7_000 - hw, 3_000 - hh), "bbox not centred");
        assert_eq!((got.w, got.h), (2 * hw, 2 * hh));
    }

    /// A macro whose local bbox does not start at the origin must still centre.
    /// This is the case that actually broke: cells are built around their own
    /// origin, so `bbox.x`/`bbox.y` are negative and were silently dropped.
    #[test]
    fn offset_local_bbox_still_centres() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let shifted = Macro {
            shapes: ell()
                .shapes
                .iter()
                .map(|s| Shape {
                    layer: s.layer,
                    rect: Rect {
                        x: s.rect.x - 2_000,
                        y: s.rect.y - 500,
                        ..s.rect
                    },
                })
                .collect(),
            pins: vec![],
            bbox: Rect {
                x: -2_000,
                y: -500,
                w: 300,
                h: 400,
            },
            units: Vec::new(),
            dummies: Vec::new(),
        };
        let (hw, hh) = (150, 200);
        let mut l = layout_of(Orient::R0, 40_000, 60_000);
        l.hw = vec![hw];
        l.hh = vec![hh];
        let got = bbox_of(&collect(&[shifted], &l, &routes));
        assert_eq!((got.x, got.y), (40_000 - hw, 60_000 - hh));
    }

    /// A turned macro keeps its anchor and transposes its extents — the contract
    /// `dp::try_rotate` assumes when it swaps `hw`/`hh`.
    #[test]
    fn r90_transposes_and_keeps_the_anchor() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let flat = bbox_of(&collect(
            &[ell()],
            &layout_of(Orient::R0, 500, 900),
            &routes,
        ));
        let turned = bbox_of(&collect(
            &[ell()],
            &layout_of(Orient::R90, 500, 900),
            &routes,
        ));
        assert_eq!((turned.x, turned.y), (flat.x, flat.y), "anchor moved");
        assert_eq!(
            (turned.w, turned.h),
            (flat.h, flat.w),
            "extents not transposed"
        );
    }

    /// Four quarter-turns return the exact original geometry — no drift creeps in
    /// through the re-anchoring arithmetic.
    #[test]
    fn four_quarter_turns_are_the_identity() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let mut m = ell();
        for _ in 0..4 {
            let shapes = collect(&[m.clone()], &layout_of(Orient::R90, 0, 0), &routes);
            m = Macro {
                bbox: bbox_of(&shapes),
                shapes,
                pins: vec![],
                units: Vec::new(),
                dummies: Vec::new(),
            };
        }
        let orig = ell();
        assert_eq!(m.bbox, orig.bbox);
        for (a, b) in m.shapes.iter().zip(&orig.shapes) {
            assert_eq!(a.rect, b.rect);
        }
    }
}
