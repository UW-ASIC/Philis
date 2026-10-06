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

/// `(net, |I| µA, allowed drop µV)` — the router checks `I·R_route` against
/// the drop, i.e. `R ≤ ΔV/I` — for every supply/ground net that carries current and
/// every signal net carrying at least `policy.ir_high_current_share` of the largest
/// net current. `current_ua` and `headroom_mv` are per net (from the op
/// point); `supply_mv` sizes the fallback. Nets without a current are skipped:
/// no current, no drop.
#[must_use]
pub fn budgets(
    classes: &[NetClassification],
    current_ua: &[Option<i32>],
    headroom_mv: &[Option<f64>],
    supply_mv: f64,
    policy: &crate::policy::Policy,
) -> Vec<(NetId, i32, i64)> {
    let i_max = current_ua.iter().flatten().map(|i| i.unsigned_abs()).max().unwrap_or(0) as f64;
    classes
        .iter()
        .filter_map(|c| {
            let n = c.net.0 as usize;
            let i_ua = current_ua.get(n).copied().flatten()?.saturating_abs();
            let i = f64::from(i_ua);
            let rail = matches!(c.class, NetClass::Supply | NetClass::Ground);
            if i <= 0.0 || !(rail || i >= policy.ir_high_current_share * i_max) {
                return None;
            }
            let dv_mv = headroom_mv
                .get(n)
                .copied()
                .flatten()
                .map_or(policy.ir_rail_share * supply_mv, |h| policy.ir_headroom_share * h);
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
        let b = budgets(&classes, &[Some(1_000), Some(-500), Some(5), None], &[Some(200.0), None, None, None], 1_800.0, &crate::policy::Policy::default());
        assert_eq!(b, [(NetId(0), 1_000, 20_000), (NetId(1), 500, 18_000)]);
        let r_max_ohm = |(_, i, uv): (NetId, i32, i64)| uv as f64 / f64::from(i);
        assert_eq!(b.into_iter().map(r_max_ohm).collect::<Vec<_>>(), [20.0, 36.0], "R_max = ΔV/I");
    }
}
