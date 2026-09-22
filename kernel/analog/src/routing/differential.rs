//! Differential-pair route matching (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/differential.rs`.

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D};
use crate::rule::Rule;

/// **Differential pair.** The `+`/`−` paths must match in length, R, C (and often
/// layer and via count) to preserve common-mode rejection and limit EMI.
/// Mismatch introduces differential delay, impedance mismatch, and asymmetric
/// crosstalk. Tight same-layer coupling and via parity further improve symmetry.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality, priority 80).
/// - **Arity:** Net↔Net (directional pos/neg pair).
/// - **Books:** FOLD 5.3/5.3.3 (#43); ALS 4.4/4.4.2 (#12); PNR_ANALOG 05/8.A (#72).
// `pos`/`neg` were `net_pos`/`net_neg` String net names in the old code.
// Old code also carried `max_r_delta_pct`/`max_c_delta_pct` (f64); R/C matching
// needs extracted parasitics (PDK wire params) which `Routes` doesn't hold, so
// only the pure-geometry length + layer-parity checks live here (see `cost`).
#[derive(Clone, Copy)]
pub struct Differential {
    pub pos: NetId,
    pub neg: NetId,
    /// Max length mismatch, percent ×10.
    pub max_len_delta_pct10: i32,
    pub same_layer_required: bool,
}

impl Differential {
    /// `|a-b|/avg·100` — the symmetric percent delta the backend uses
    /// (`reconcile_geometry`'s `delta_pct`). 0 when both sides are empty.
    fn len_delta_pct(self, r: &Routes) -> f32 {
        let a = r.length(self.pos) as f32;
        let b = r.length(self.neg) as f32;
        let avg = (a + b) / 2.0;
        if avg > 0.0 {
            (a - b).abs() / avg * 100.0
        } else {
            0.0
        }
    }

    /// True when the two nets draw on the same set of layers. The backend
    /// compares sorted-deduped layer lists; a set-equality check is the same
    /// predicate without allocating.
    fn layers_match(self, r: &Routes) -> bool {
        let layers = |n: NetId| {
            let mut v: Vec<u16> = r.shapes(n).iter().map(|s| s.layer.0).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        layers(self.pos) == layers(self.neg)
    }
}

impl Rule for Differential {
    type On = Routes;
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.pos.0));
        out.push(u32::from(self.neg.0));
    }

    /// Percent length delta beyond the budget (0 within budget). Pure geometry;
    /// R/C deltas are omitted — no extracted parasitics in `Routes`.
    fn cost(self, r: &Routes) -> f32 {
        let budget = self.max_len_delta_pct10 as f32 / 10.0;
        (self.len_delta_pct(r) - budget).max(0.0)
    }
    /// Length delta within budget and, if required, matched layer sets.
    fn satisfied(self, r: &Routes) -> bool {
        let budget = self.max_len_delta_pct10 as f32 / 10.0;
        self.len_delta_pct(r) <= budget && (!self.same_layer_required || self.layers_match(r))
    }

    /// Length mismatch past budget as a fraction of budget — or a full `1.0` when the
    /// layer sets disagree.
    ///
    /// The two halves of [`satisfied`](Rule::satisfied) are different kinds of condition
    /// and the residual has to say so. Length delta is a magnitude and normalises
    /// cleanly. `same_layer_required` is a **Boolean** with no spec to be a fraction of:
    /// the sets either match or they do not, and there is no "20% wrong layer". So it
    /// contributes the [`Rule::residual`] default's `1.0`, which is the honest reading —
    /// one full budget's worth, never understated — and the two are combined by `max`
    /// rather than summed, because they are two views of one pair, not two violations.
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_len_delta_pct10 as f32 / 10.0;
        let len = crate::rule::over(self.len_delta_pct(r) - budget, budget);
        let layer = f32::from(self.same_layer_required && !self.layers_match(r));
        len.max(layer)
    }

    // AUDIT (`Rule::project`, `docs/API-WISH.md`): no `project`, and the `same_layer_required`
    // half is a genuine gap rather than a deliberate omission.
    //
    // `Differential` is registered `Hard` (`annotator::extract::routing`). The length half
    // is an inequality with a gradient, so the optimiser is the right tool for it. The
    // layer half is a **set equality** on layer assignment: `cost` above scores only the
    // length term, so a layer mismatch is invisible to the objective *and* has no
    // projection — a route pair on mismatched layers is a hard violation nothing in the
    // flow can move toward fixing, and `dr` will report it every epoch forever.
    //
    // It is not projectable from `Routes` either: re-assigning a net's layer means
    // rerouting it, which is `dr`'s job, not a coordinate nudge. The actionable form is
    // `Rule::touches` (already implemented — it names both nets), so the honest repair path
    // is targeted rip-up of the pair on a layer mismatch, in `dr`.

    /// **Recognition.** The `+`/`−` paths of a differential pair are its two
    /// **drain** nets — the balanced outputs that must match in length/layer. For
    /// each recognised diff pair (shared source, distinct gates+drains), emit the
    /// two drain nets as `pos`/`neg` (Net↔Net). The gate inputs are the pair's
    /// other differential net; drains are chosen because they carry the matched
    /// output currents the old `differential_extractor` balanced.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self> {
        let _ = uf;
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if is_diff_pair(hg, a, b) {
                    out.push(Differential {
                        pos: hg.device_nets[a][D],
                        neg: hg.device_nets[b][D],
                        max_len_delta_pct10: 50, // 5.0% (old DifferentialPair default)
                        same_layer_required: true,
                    });
                }
            }
        }
        out
    }
}
