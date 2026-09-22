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
    let result = library::run(
        "\n.subckt chain2 in mid out vss\nM1 mid in vss vss nfet w=0.42u l=0.15u\nM2 out mid vss vss nfet w=0.84u l=0.15u\n.ends\n",
        &pdk,
        &library::Macros::default(),
        &Config { feedback_iters: 3, outer_iters: 1, ..Default::default() },
    );
    // Ok or a clean FlowError both prove the point — the epoch loop survived.
    let sol = match result {
        Ok(sol) => sol,
        Err(e) => {
            eprintln!("flow returned a clean error (accepted): {e:?}");
            return;
        }
    };

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
    let report = library::signoff(&sol, &pdk);
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
