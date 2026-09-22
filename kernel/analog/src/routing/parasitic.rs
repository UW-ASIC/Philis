//! Per-net parasitic budget (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/parasitic.rs`.

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::rule::Rule;

/// **Parasitic budget.** Wire R/C degrades speed (`RC delay ∝ L²`) and burns
/// power. Per-net `max_r`/`max_c` budgets drive layer assignment (high-impedance
/// nets on top layers; power on thick metal) and net splitting. Fringe cap
/// dominates narrow leads (~60% of total); coupling cap compounds the load.
///
/// - **Enforcement:** [`crate::Mode::Cost`] (soft, priority 60).
/// - **Arity:** Net↔Route.
/// - **Books:** AOAL ch01/1.3.2, ch02/2.7.1, ch07/7.3, ch13/13.1 (#58);
///   FOLD 7.3 (#20–21); ALS 4.4/4.5 (#14/#24); PNR_ANALOG 00/3.1–3.4 (#16–17).
// `net` was a String net name in the old code; `max_r` (ohm f64) / `max_c` (fF
// f64) are `max_r_mohm` / `max_c_af` here (integer sub-units).
#[derive(Clone, Copy)]
pub struct ParasiticBudget {
    pub net: NetId,
    /// Max wire resistance, milli-ohm.
    pub max_r_mohm: i64,
    /// Max wire capacitance, atto-farad.
    pub max_c_af: i64,
    /// Drawn length, nm, at which this net exhausts its R/C budget — the
    /// electrical budget **lowered into a routing resource**.
    ///
    /// Turning a spec into a geometric limit the router can check locally is what
    /// ALIGN (max wire length / parallel tracks / via count) and AIDA-L (EM-aware
    /// topology from current densities) do upstream of P&R; see
    /// `backend/TODO.md` §5a. It is what lets a budget be scored during search
    /// instead of only after extraction.
    pub max_len_nm: i64,
    /// Safety margin held back from `max_len_nm`, percent. The raw length is the
    /// terminal hard floor; the optimiser targets `max_len_nm·(1 − margin)`.
    pub margin_pct: u8,
}

impl Rule for ParasiticBudget {
    type On = Routes;
    /// Squared overshoot of extracted R/C beyond the budget — the backend's
    /// `worst = max(rn/max_r - 1, cn/max_c - 1)`, squared to penalise the tail.
    fn cost(self, r: &Routes) -> f32 {
        // Proxy: extracted R and C both grow with drawn wire length, so penalise
        // length² (monotone in the R·C the budget bounds). Real extraction needs
        // per-layer `WireParasiticParams` (sheet_r/area_cap/fringe_cap/via_r) — see
        // backend `estimate_net_rc` — which live in the PDK, not in `Routes`; until
        // they're threaded, `max_r_mohm`/`max_c_af` stay on the rule but the score
        // is length-driven.
        //
        // ponytail: `1e-6` is the same species of fudge as `CouplingBudget::cost`'s
        // deleted `× 1e-3` — a hand-tuned scale that only exists because `len²` in nm²
        // would dwarf every other routing cost. The normalised form is `(len/max_len)²`,
        // which is dimensionless and needs no constant. Not changed here because unlike
        // the coupling one this term is *read*: `dr::score` blends `reqs.cost` costs and
        // `gr::score` sums them, so rescaling it silently re-weights the PEX tier of a
        // sibling stage. Ceiling: the constant is a per-net weight in disguise. Upgrade:
        // divide by `max_len_nm²` here and re-baseline the routing cost fixtures in the
        // same commit.
        let len = r.length(self.net) as f32;
        len * len * 1e-6
    }

    /// Drawn length within the lowered budget. A net that is not routed at all
    /// has zero length and trivially passes — an unrouted net is a connectivity
    /// failure, not a parasitic one.
    fn satisfied(self, r: &Routes) -> bool {
        r.length(self.net) <= self.max_len_nm
    }

    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }

    /// Fraction of the lowered length budget still unspent.
    fn headroom(self, r: &Routes) -> f32 {
        let budget = self.max_len_nm.max(1) as f32;
        1.0 - (r.length(self.net) as f32 / budget)
    }

    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }

    /// Drawn length **past** the lowered budget, as a fraction of that budget.
    ///
    /// One of the two batches that can answer this today without new data (D15): the
    /// rule already carries the spec (`max_len_nm`, itself the lowering of
    /// `max_r_mohm`/`max_c_af` into a routing resource) *and* the measured quantity
    /// (`Routes::length`), so the ratio needs nothing threaded through.
    ///
    /// Measured against the **raw** `max_len_nm`, not the derated `·(1 − margin)`
    /// target. The margin is what [`headroom`](Rule::headroom)/`criticality` use to
    /// start applying pressure *early*; Θ is "how far past the spec", and a net inside
    /// the raw spec but inside its margin is tight, not violated. Deriving Θ from the
    /// derated value would report every converged run as infeasible.
    ///
    /// Deliberately not `-headroom`: [`Rule::headroom`]'s contract clamps to `[0, 1]`,
    /// so its negation saturates at `0` exactly where a residual has to start counting.
    /// That this impl happens to leave it unclamped is not something to build on.
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_len_nm as f32;
        crate::rule::over(r.length(self.net) as f32 - budget, budget)
    }

    /// **Recognition.** A parasitic budget is per-net and applies to every
    /// **routed** net — one with at least two incident device terminals, i.e. an
    /// actual wire between pins (a net touched only once carries no route to
    /// budget). Sweep `net_devices`, emit one `ParasiticBudget` per such net.
    ///
    /// Only finds *where* the rule applies; the `max_r`/`max_c` are placeholders
    /// and `cost` stays blocked on per-layer `WireParasiticParams` the PDK holds
    /// and `Routes` does not carry.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self> {
        let _ = uf;
        let mut out = Vec::new();
        for (n, devs) in hg.net_devices.iter().enumerate() {
            if devs.len() >= 2 {
                out.push(ParasiticBudget {
                    net: NetId(n as u16),
                    // Loose default budgets (real values come from the net's class
                    // + PDK; the hypergraph carries neither). ponytail: placeholder.
                    max_r_mohm: 1_000_000, // 1 kΩ
                    max_c_af: 100_000_000, // 100 fF
                    // ponytail: 1 mm of drawn metal as the lowered length cap —
                    // generous for an analog block, so it bites only on a badly
                    // detoured net. The real lowering is `max_r_mohm / sheet_r`
                    // at the net's assigned layer, which needs the PDK's
                    // `WireParasiticParams`; thread those and this becomes exact.
                    max_len_nm: 1_000_000,
                    margin_pct: 20,
                });
            }
        }
        out
    }
}
