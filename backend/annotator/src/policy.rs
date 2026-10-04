//! Every tuning number the annotator emits with, in one table (EXT-10). Each is
//! Philis policy unless its doc names a source; none is derived from the deck.

/// The annotator's emission policy; [`Default`] is today's values.
#[derive(Clone, Debug)]
pub struct Policy {
    /// Philis policy: the largest distance, nm, a tail or cascode may sit from
    /// the devices it serves (`Proximity`).
    pub proximity_nm: i32,
    /// Philis policy: crosstalk spacing in routing spaces by class, for
    /// [Sensitive, Clock, Signal, other].
    pub spacing_multiple: [i32; 4],
    /// Philis policy: margin held back from a net's budgets, percent, for
    /// [Sensitive, Clock, Supply|Ground, other].
    pub margin_pct: [u8; 4],
    /// Philis policy: share of a sensitive net's run a shield must cover, percent.
    pub shield_coverage_pct: u8,
    /// Philis policy: largest shield gap, in routing spaces (the adjacent track
    /// with a spacing of slack).
    pub shield_gap_spaces: i32,
    /// Philis policy: margin held back from the deck's antenna ratio, percent.
    pub antenna_margin_pct: u8,
    /// Philis policy: allowed length mismatch of a differential pair, 0.1 %.
    pub diff_pct10: i32,
    /// Philis policy: share of a net's saturation headroom its wiring may drop.
    /// The circuit's own error budget (ΔI ≈ g_m·ΔV) should set it once specs carry one.
    pub ir_headroom_share: f64,
    /// Common analog rail-drop target, not derived: drop allowed, as a share of
    /// the supply, when no saturated device reports headroom.
    pub ir_rail_share: f64,
    /// Philis policy: a signal net is "high-current" at this share of the
    /// busiest net's current (relative, so a µA bias net is not budgeted like a
    /// mA branch).
    pub ir_high_current_share: f64,
    /// ProxNet star cap (EXT-13, Philis policy: the survey does not say whether rails are excluded;
    /// BAL1-49).
    pub pn_max_degree: usize,
    /// Series resistance below which a diffusion on a pin net is a
    /// minority-carrier injector, Ω (GAP-03): Hastings §14.2 L43629–43638
    /// (below about 50 kΩ); H05-47 (10 kΩ usual, 50–100 kΩ conservative).
    pub inj_series_ohm: f64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            proximity_nm: 5_000,
            spacing_multiple: [8, 7, 3, 1],
            margin_pct: [35, 30, 25, 20],
            shield_coverage_pct: 80,
            shield_gap_spaces: 2,
            antenna_margin_pct: 20,
            diff_pct10: 50,
            ir_headroom_share: 0.1,
            ir_rail_share: 0.01,
            ir_high_current_share: 0.1,
            pn_max_degree: 8,
            inj_series_ohm: 50_000.0,
        }
    }
}
