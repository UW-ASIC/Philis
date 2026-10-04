//! The `philis` binary end to end: flags, outputs, exit codes.

use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/fixtures").join(name)
}

/// STRING payloads of the GDS TEXT elements, trailing NULs trimmed.
fn gds_texts(bytes: &[u8]) -> Vec<String> {
    let (mut i, mut in_text, mut out) = (0, false, Vec::new());
    while i + 4 <= bytes.len() {
        let len = usize::from(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
        let ty = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]);
        assert!(len >= 4, "bad record length at {i}");
        match ty {
            0x0C00 => in_text = true,
            0x1906 if in_text => {
                out.push(String::from_utf8_lossy(&bytes[i + 4..i + len]).trim_end_matches('\0').to_string());
                in_text = false;
            }
            _ => {}
        }
        i += len;
    }
    out
}

#[test]
fn run_writes_a_labelled_gds_and_a_report() {
    let dir = std::env::temp_dir().join(format!("philis-cli-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_philis"))
        .arg(fixture("pair.spice"))
        .args(["--pdk", "sky130", "--iters", "2", "--starts", "1", "--outer", "1", "--out"])
        .arg(&dir)
        .env("PDK_ROOT", std::env::var_os("PDK_ROOT").unwrap_or_default())
        .output()
        .expect("philis runs");
    let code = out.status.code();
    assert!(matches!(code, Some(0 | 1)), "exit {code:?}: {}", String::from_utf8_lossy(&out.stderr));
    let texts = gds_texts(&std::fs::read(dir.join("pair.gds")).expect("pair.gds written"));
    for port in ["d", "g", "VSS"] {
        assert!(texts.iter().any(|t| t == port), "label {port} missing from {texts:?}");
    }
    let report = std::fs::read_to_string(dir.join("report.txt")).expect("report.txt written");
    assert!(report.contains("stage_ms"), "{report}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// EXT-26 (replaces `constraints_flag_exits_2`, which pinned the refusal
/// "until EXT-26"): an unreadable file exits 2 naming the flag; a sidecar is
/// read and the flow runs.
#[test]
fn constraints_flag_reads_the_file() {
    let out = Command::new(env!("CARGO_BIN_EXE_philis"))
        .arg(fixture("pair.spice"))
        .args(["--pdk", "sky130", "--constraints", "no_such_constraints.json"])
        .output()
        .expect("philis runs");
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("constraints"));

    let dir = std::env::temp_dir().join(format!("philis_cli_constraints_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let json = dir.join("pair.const.json");
    std::fs::write(&json, r#"[{"constraint":"GroundPorts","ports":["VSS"]},{"constraint":"Align","instances":["XM1","XM2"]}]"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_philis"))
        .arg(fixture("pair.spice"))
        .args(["--pdk", "sky130", "--starts", "1", "--constraints"])
        .arg(&json)
        .args(["-o"])
        .arg(&dir)
        .output()
        .expect("philis runs");
    let code = out.status.code();
    assert!(matches!(code, Some(0 | 1)), "exit {code:?}: {}", String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}
