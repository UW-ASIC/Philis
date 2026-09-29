//! DC IR-drop budgets (PWR-02): a resistance allowance per net that carries
//! real current, from the operating point.
//!
//! `ΔV = I·R` along a path (Lienig & Thiele 2018 §3.3, eqs 3.1–3.7), so a net
//! carrying `I` may have at most `R_max = ΔV_allowed / I` of wire and vias;
//! the router turns that into length per width through `R = R□·L/W`
//! (Lampaert 1999 §2.5.3.2 eq.(2.33)). `ΔV_allowed` is spent from the
//! saturation headroom `V_DS − V_DSsat` of the devices the net feeds: a drop
//! past it pushes a device into triode.

use analog::metadata::{NetClass, NetClassification};
use pnr_core::NetId;

/// Share of a net's saturation headroom its wiring may drop.
///
/// ponytail: a fixed 10%; the circuit's own error budget (ΔI ≈ g_m·ΔV) should
/// set it once specs carry one.
const HEADROOM_SHARE: f64 = 0.1;
/// Drop allowed when no saturated device reports headroom, as a share of the
/// supply.
///
/// ponytail: 1% of the rail, a common analog rail-drop target, not derived.
const RAIL_SHARE: f64 = 0.01;
/// A signal net is "high-current" at this share of the busiest net's current.
///
/// ponytail: relative, so a µA bias net is not budgeted like a mA branch.
const HIGH_CURRENT_SHARE: f64 = 0.1;

/// `(net, |I| µA, allowed drop µV)` — the router checks `I·R_route` against
/// the drop, i.e. `R ≤ ΔV/I` — for every supply/ground net that carries current and
/// every signal net carrying at least [`HIGH_CURRENT_SHARE`] of the largest
/// net current. `current_ua` and `headroom_mv` are per net (from the op
/// point); `supply_mv` sizes the fallback. Nets without a current are skipped:
/// no current, no drop.
#[must_use]
pub fn budgets(
    classes: &[NetClassification],
    current_ua: &[Option<i32>],
    headroom_mv: &[Option<f64>],
    supply_mv: f64,
) -> Vec<(NetId, i32, i64)> {
    let i_max = current_ua.iter().flatten().map(|i| i.unsigned_abs()).max().unwrap_or(0) as f64;
    classes
        .iter()
        .filter_map(|c| {
            let n = c.net.0 as usize;
            let i_ua = current_ua.get(n).copied().flatten()?.saturating_abs();
            let i = f64::from(i_ua);
            let rail = matches!(c.class, NetClass::Supply | NetClass::Ground);
            if i <= 0.0 || !(rail || i >= HIGH_CURRENT_SHARE * i_max) {
                return None;
            }
            let dv_mv = headroom_mv
                .get(n)
                .copied()
                .flatten()
                .map_or(RAIL_SHARE * supply_mv, |h| HEADROOM_SHARE * h);
            Some((c.net, i_ua, (dv_mv * 1e3) as i64))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(net: u16, class: NetClass) -> NetClassification {
        NetClassification { net: NetId(net), class, c_budget_af: None, max_coupling_af: None }
    }

    #[test]
    fn resistance_budget_is_the_allowed_drop_over_the_current() {
        let classes = [
            class(0, NetClass::Supply),
            class(1, NetClass::Signal),
            class(2, NetClass::Signal),
            class(3, NetClass::Ground),
        ];
        // VDD: 1 mA, a 200 mV headroom device → 20 mV (R ≤ 20 Ω).
        // Net 1: 500 µA, no saturated device → 1% of 1.8 V = 18 mV (R ≤ 36 Ω).
        // Net 2: 5 µA, under 10% of the busiest: no budget. Net 3: no current.
        let b = budgets(&classes, &[Some(1_000), Some(-500), Some(5), None], &[Some(200.0), None, None, None], 1_800.0);
        assert_eq!(b, [(NetId(0), 1_000, 20_000), (NetId(1), 500, 18_000)]);
        let r_max_ohm = |(_, i, uv): (NetId, i32, i64)| uv as f64 / f64::from(i);
        assert_eq!(b.into_iter().map(r_max_ohm).collect::<Vec<_>>(), [20.0, 36.0], "R_max = ΔV/I");
    }
}
