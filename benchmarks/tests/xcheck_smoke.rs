//! The foundry cross-check (`benchmarks/signoff_xcheck.py`) can see a planted
//! fault: two met1 squares 100 nm apart (sky130A_mr.drc m1.2, 0.14 µm).

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn items(lyrdb: &Path) -> usize {
    std::fs::read_to_string(lyrdb).expect("read lyrdb").matches("<item>").count()
}

#[test]
fn foundry_drc_sees_a_planted_met1_spacing_fault() {
    if !library::tools::tool_or_skip("klayout") {
        return;
    }
    let deck = std::env::var_os("PDK_ROOT")
        .map(|p| PathBuf::from(p).join("sky130A/libs.tech/klayout/drc/sky130A_mr.drc"));
    if !library::tools::present_or_skip(&format!("foundry DRC deck ({deck:?})"), deck.as_ref().is_some_and(|d| d.is_file())) {
        return;
    }
    let deck = deck.unwrap();
    let text = std::fs::read_to_string(root().join("pdks/sky130.json")).expect("read sky130 deck");
    let pdk = verify::Pdk::from_json(&text).expect("sky130 deck is complete");
    let met1 = pdk.layers.iter().find(|(n, _)| n == "met1").expect("met1").1;
    let sq = |x| pnr_core::Shape { layer: met1, rect: pnr_core::Rect { x, y: 0, w: 1000, h: 1000 } };
    let gds = library::gds::emit("TOP", &[sq(0), sq(1100)], &pdk.layer_gds(), &[]).unwrap();
    let dir = std::env::temp_dir().join(format!("philis_xcheck_smoke_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pair.gds");
    std::fs::write(&path, gds).unwrap();

    // (a) The harness, flags and all, reports the fault.
    let out = Command::new("python3").arg(root().join("benchmarks/signoff_xcheck.py")).arg("--drc-only").arg(&path).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "harness failed: {stdout}{}", String::from_utf8_lossy(&out.stderr));
    let n: usize = stdout.trim().strip_prefix("{\"drc\": ").and_then(|s| s.strip_suffix('}')).and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("harness printed {stdout:?}"));
    assert!(n >= 1, "foundry DRC missed a 100 nm met1 gap");

    // (b) The deck without feol/beol checks nothing: why the harness forces them.
    let report = dir.join("bare.lyrdb");
    let st = Command::new("klayout").args(["-b", "-r"]).arg(&deck).arg("-rd").arg(format!("input={}", path.display()))
        .args(["-rd", "top_cell=TOP", "-rd"]).arg(format!("report={}", report.display())).status().unwrap();
    assert!(st.success(), "bare klayout run failed");
    assert_eq!(items(&report), 0, "sky130A_mr.drc without feol/beol reported findings: the flags are no longer mandatory");
    let _ = std::fs::remove_dir_all(&dir);
}
