//! Debug-build smoke test: a mid-search epoch may produce an *open* net (the
//! router is still negotiating), and an open epoch must be scored and
//! discarded, not treated as fatal. Only the winning solution has to be
//! connected. This used to panic at `routes.rs: net 0 is open` on the very
//! first epoch.

use library::Config;

#[test]
fn chain2_open_epoch_is_scored_not_fatal() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let sol = library::run(
        "\n.subckt chain2 in mid out vss\nM1 mid in vss vss nfet w=0.42u l=0.15u\nM2 out mid vss vss nfet w=0.84u l=0.15u\n.ends\n",
        &pdk,
        &library::Macros::default(),
        &Config { feedback_iters: 3, outer_iters: 1, ..Default::default() },
    )
    // An `Err` here would pass the old "no panic" bar while proving nothing
    // about the signoff plumbing below, so the flow must deliver a layout.
    .expect("the flow returns a solution");

    // Signoff smoke over the same solution: the pins helper must derive labels
    // the engine can bind (verify fails closed on a label with no geometry
    // under it), and the reference must compile against the deck — either
    // failure comes back as an `engine/…` hard violation. Real DRC/LVS
    // findings are fine here — clean-layout gating belongs to ota_cross_pdk —
    // and so is ONE specific engine fault: a 3-epoch layout may carry a drawn
    // short, which puts two nets' labels on one extracted net and aborts
    // extraction (verify's fail-closed answer to a short, observed here). What
    // must never appear is a label that failed to bind or a reference the deck
    // rejected — those would be bugs in this crate's signoff plumbing.
    let report = library::signoff(&sol, &pdk).report;
    let plumbing_faults: Vec<&str> = report
        .hard_violations
        .iter()
        .filter(|v| v.rule.starts_with("engine/"))
        .filter(|v| !v.rule.contains("two different labels"))
        .map(|v| v.rule.as_str())
        .collect();
    assert!(
        plumbing_faults.is_empty(),
        "signoff plumbing failed (mislanded pin label / bad reference?): {plumbing_faults:?}"
    );
}

/// MAT-13 / MAT-08: the report carries one ledger row per matched pair, and
/// a 1 mV offset budget under the input pair's σ_rand (9.5/√10 ≈ 3.0 mV)
/// leaves the layout no allowance: the row is sizing-limited and its set is
/// violated whenever placement spends anything.
#[test]
fn matched_sets_are_reported() {
    let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
    let cfg = Config {
        feedback_iters: 1,
        outer_iters: 1,
        annotation: annotator::AnnotationConfig { offset_sigma_mv: Some(1.0), ..Default::default() },
        ..Default::default()
    };
    let sol = library::run(include_str!("../../../benchmarks/fixtures/ota.spice"), &pdk, &library::Macros::default(), &cfg)
        .expect("the flow returns a solution");
    let rows = &sol.metadata.matched;
    assert!(!rows.is_empty());
    for r in rows {
        let f = [r.sigma_rand, r.sigma_layout, r.mu_thermal, r.mu_lod, r.allowance, r.usage, r.second_order_nm];
        assert!(f.iter().all(|v| v.is_finite()), "{r:?}");
        assert!(!r.known || r.sigma_rand > 0.0, "{r:?}");
    }
    let pair = rows.iter().find(|r| r.members == (0, 1)).expect("XM1/XM2 row");
    assert!(pair.sigma_rand >= 1.0, "{pair:?}");
    for r in rows.iter().filter(|r| r.sigma_rand >= 1.0) {
        assert!(r.allowance == 0.0 && r.sizing_limited, "{r:?}");
        assert!(r.usage > 1.0 || r.sigma_layout + r.mu_thermal + r.mu_lod == 0.0, "{r:?}");
    }
    // GAP-02: the same shortfall is named as a sizing note, once.
    assert_eq!(sol.metadata.sizing.iter().filter(|n| n.members == (0, 1) && n.kind == "budget_area").count(), 1);
}
