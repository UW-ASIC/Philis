//! DC electromigration (routing tier, hard), plus the per-layer limit model
//! the detailed router sizes segments and via arrays with.

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;

/// Boltzmann's constant, eV/K.
const K_EV: f32 = 8.617e-5;

/// Allowed-current factor for a limit rated at `t_ref_k`, at conductor
/// temperature `t_k`: `F = exp[(Ea/(n·k))·(1/T − 1/T_ref)]`, `< 1` when hotter.
/// Hastings eqs. 15.24–15.25; Lienig & Thiele 2018 eq. 3.11 (from Black, eq. 3.8).
/// Never credits a conductor cooler than the rating: `F ≤ 1`.
#[must_use]
pub fn derate(t_k: f32, t_ref_k: f32, ea_ev: f32, n: f32) -> f32 {
    if t_k <= t_ref_k || n <= 0.0 {
        return 1.0;
    }
    ((ea_ev / (n * K_EV)) * (1.0 / t_k - 1.0 / t_ref_k)).exp()
}

/// One layer's deck EM limits, already derated to the conductor temperature.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Limit {
    /// DC current per µm of width, µA/µm (the PDK's unit carries thickness);
    /// `0` = unknown.
    pub ua_per_um: f32,
    /// DC current per cut, µA; `0` = unknown.
    pub ua_per_cut: f32,
    /// Blech product `(jL)_B` in (µA/µm)·µm; `0` = the deck has none.
    pub blech: f32,
}

impl Limit {
    /// Scale both current limits by a [`derate`] factor.
    #[must_use]
    pub fn derated(self, f: f32) -> Self {
        Self { ua_per_um: self.ua_per_um * f, ua_per_cut: self.ua_per_cut * f, ..self }
    }

    /// Width a segment carrying `i_ua` needs, nm: `w ≥ I/J` (Lienig & Thiele
    /// 2018 eq. 3.21; Hastings eq. 15.24) — unless it is Blech-immortal,
    /// `(I/w)·L < (jL)_B` (Lienig eq. 4.1), i.e. `w > I·L/(jL)_B`. `domain_nm`
    /// must bound the diffusion domain (the net's whole run on the layer), not
    /// one drawn rect. `0` when the limit is unknown.
    #[must_use]
    pub fn width_nm(self, i_ua: f32, domain_nm: f32) -> f32 {
        if self.ua_per_um <= 0.0 {
            return 0.0;
        }
        let density = i_ua.abs() * 1_000.0 / self.ua_per_um;
        if self.blech > 0.0 {
            density.min(i_ua.abs() * domain_nm / self.blech)
        } else {
            density
        }
    }

    /// Cuts a via carrying `i_ua` needs: `n = ⌈I / I_cut⌉` (Lienig & Thiele 2018
    /// eq. 3.25; its temperature factor f(T) is divided out here, via
    /// [`Limit::derated`] — the book prints it multiplied, which would let a
    /// hotter via have fewer cuts). `1` when the limit is unknown.
    #[must_use]
    pub fn cuts(self, i_ua: f32) -> u32 {
        if self.ua_per_cut <= 0.0 {
            return 1;
        }
        ((i_ua.abs() / self.ua_per_cut).ceil() as u32).max(1)
    }
}

/// Most routing metals a rule carries limits for.
pub const MAX_METALS: usize = 8;

/// The largest current a single terminal of `net` draws must fit through
/// some drawn segment at its layer's DC limit.
///
/// This is the tree-independent half of the check: the rule sees only shapes,
/// not which terminal each segment feeds, so it checks what every routed tree
/// must satisfy (a terminal's current passes through the segments that reach
/// it). Per-segment currents (Lienig & Thiele 2018 eqs. 3.5–3.7: each segment
/// carries the sum of the terminal currents on one side) need the tree, and
/// `dr` sizes and reports them (Θ `em underwidth` / `em cuts`).
///
/// Unknown (not passed) without an operating-point current, without a deck
/// limit on any routed metal, or before the net is routed. A known **zero**
/// current is a real pass.
///
/// ponytail: DC average only; RMS/peak need waveforms, and cuts are `dr`'s.
#[derive(Clone, Copy)]
pub struct Electromigration {
    pub net: NetId,
    /// Largest single-terminal current, µA; negative = unknown.
    pub current_ua: i32,
    /// `(layer id, limit)` per routing metal the deck limits; unused slots
    /// have layer `u16::MAX`.
    pub limits: [(u16, Limit); MAX_METALS],
}

impl Electromigration {
    fn limit(self, layer: u16) -> Option<Limit> {
        self.limits.iter().find(|(l, lim)| *l == layer && lim.ua_per_um > 0.0).map(|&(_, lim)| lim)
    }

    /// `(width, need)` of the segment with the most headroom, nm; `None` when
    /// no drawn segment is on a limited metal. Blech domain: the net's whole
    /// run on that layer.
    fn best(self, r: &Routes) -> Option<(f32, f32)> {
        let shapes = r.shapes(self.net);
        shapes
            .iter()
            .filter_map(|s| {
                let lim = self.limit(s.layer.0)?;
                let run: i64 = shapes.iter().filter(|t| t.layer == s.layer).map(|t| i64::from(t.rect.w.max(t.rect.h))).sum();
                let need = lim.width_nm(self.current_ua.max(0) as f32, run as f32);
                Some((s.rect.w.min(s.rect.h) as f32, need))
            })
            .max_by(|a, b| (a.0 - a.1).total_cmp(&(b.0 - b.1)))
    }
}

impl Rule for Electromigration {
    type On = Routes;
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        !self.known(r) || self.best(r).is_some_and(|(w, need)| w >= need)
    }
    fn known(self, r: &Routes) -> bool {
        self.current_ua >= 0 && self.best(r).is_some()
    }
    fn residual(self, r: &Routes) -> f32 {
        match (self.known(r), self.best(r)) {
            (true, Some((w, need))) => crate::rule::over(need - w, need),
            _ => 0.0,
        }
    }
    /// Unused fraction of the best segment; an unknown puts no pressure.
    fn headroom(self, r: &Routes) -> f32 {
        match (self.known(r), self.best(r)) {
            (true, Some((w, need))) if w > 0.0 => 1.0 - need / w,
            _ => 1.0,
        }
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Rect, Shape};

    /// Net 0: a 200 nm-wide wire on layer 1 and a 100 nm cut (layer 2).
    fn routes() -> Routes {
        Routes {
            wires: vec![vec![
                Shape { layer: LayerId(1), rect: Rect { x: 0, y: 0, w: 10_000, h: 200 } },
                Shape { layer: LayerId(2), rect: Rect { x: 0, y: 0, w: 100, h: 100 } },
            ]],
            ..Default::default()
        }
    }

    fn lim(ua_per_um: f32) -> Limit {
        Limit { ua_per_um, ua_per_cut: 100.0, blech: 0.0 }
    }

    fn em(current_ua: i32, ua_per_um: f32) -> Electromigration {
        let mut limits = [(u16::MAX, Limit::default()); MAX_METALS];
        limits[0] = (1, lim(ua_per_um));
        Electromigration { net: NetId(0), current_ua, limits }
    }

    #[test]
    fn width_must_carry_the_current_and_cuts_are_not_segments() {
        let r = routes();
        // 150 µA at 1 mA/µm needs 150 nm: the 200 nm wire carries it (the
        // 100 nm cut has no metal limit, so it is not a segment).
        assert!(em(150, 1_000.0).satisfied(&r));
        // 400 µA needs 400 nm: violated, residual = (400−200)/400.
        let e = em(400, 1_000.0);
        assert!(!e.satisfied(&r));
        assert!((e.residual(&r) - 0.5).abs() < 1e-6);
        // A known zero current is a real pass.
        assert!(em(0, 1_000.0).known(&r) && em(0, 1_000.0).satisfied(&r));
    }

    #[test]
    fn the_limit_is_per_layer() {
        let r = routes();
        // The same 400 µA passes at layer 1's own 2 mA/µm, and a limit listed
        // for another layer does not apply to layer 1's segment.
        assert!(em(400, 2_000.0).satisfied(&r));
        let mut other = em(400, 2_000.0);
        other.limits[0].0 = 3;
        assert!(!other.known(&r), "no limit on a routed metal: unknown");
    }

    #[test]
    fn missing_current_limit_or_route_is_unknown() {
        let r = routes();
        assert!(!em(-1, 1_000.0).known(&r), "no operating point");
        assert!(!em(400, 0.0).known(&r), "no deck current density");
        assert!(!em(400, 1_000.0).known(&Routes { wires: vec![vec![]], ..Default::default()  }), "unrouted");
        assert!(em(-1, 1_000.0).satisfied(&r), "search cannot act on an unknown");
    }

    /// Hastings' worked derating: 398 K on a 378 K rating with Ea 0.7 eV, n 2
    /// gives F ≈ 0.58 ("25 mA safe at ref → 15 mA"). Hotter never needs fewer
    /// cuts or less width; cooler is never credited.
    #[test]
    fn hotter_never_needs_fewer_cuts_or_less_width() {
        let f = derate(398.0, 378.0, 0.7, 2.0);
        assert!((f - 0.58).abs() < 0.02, "Hastings eq. 15.25 example: {f}");
        assert_eq!(derate(300.0, 378.0, 0.7, 2.0), 1.0, "cooler than the rating is not credited");
        let base = Limit { ua_per_um: 1_000.0, ua_per_cut: 100.0, blech: 0.0 };
        let mut prev = (0u32, 0.0f32);
        for t in [378.0, 398.0, 423.0, 448.0] {
            let hot = base.derated(derate(t, 378.0, 0.9, 2.0));
            let now = (hot.cuts(250.0), hot.width_nm(250.0, 0.0));
            assert!(now.0 >= prev.0 && now.1 >= prev.1, "T {t}: {now:?} after {prev:?}");
            prev = now;
        }
        assert_eq!(base.cuts(250.0), 3, "⌈250/100⌉ at the rating");
        assert!(prev.0 > 3, "hot enough to need more cuts: {prev:?}");
    }

    /// A short enough run is Blech-immortal and needs less than `I/J`; without
    /// a deck Blech product the density limit always applies.
    #[test]
    fn a_short_run_below_the_blech_product_needs_less_width() {
        let l = Limit { ua_per_um: 1_000.0, ua_per_cut: 0.0, blech: 54_000.0 };
        // 500 µA: I/J = 500 nm. Over 20 µm, I·L/B = 500·20_000/54_000 ≈ 185 nm.
        assert!((l.width_nm(500.0, 20_000.0) - 185.2).abs() < 1.0);
        // A long run is bounded by density again.
        assert!((l.width_nm(500.0, 1_000_000.0) - 500.0).abs() < 1e-3);
        assert!((Limit { blech: 0.0, ..l }.width_nm(500.0, 20_000.0) - 500.0).abs() < 1e-3);
    }
}
