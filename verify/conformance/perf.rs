//! CPU vs GPU DRC baseline on a dense interlocked-bar layout — the case where
//! bbox pruning is structurally useless and the GPU prefilter earns its keep.
//!
//! Runs the same store on Backend::Cpu and (when a device is usable) Backend::Gpu
//! and REQUIRES bit-identical violation reports: the GPU is a prefilter, never an
//! oracle. Prints a wall-clock pair for the benchmark log.
//!
//! Usage: perf [n_bars]   (default 2000; ~2M candidate pairs)

use gdsverify::gpu;
use gdsverify::*;
use std::collections::HashMap;
use std::time::Instant;

fn deck() -> Deck {
    let mut layers = HashMap::new();
    layers.insert("met1".to_string(), (7, 0));
    let rule = schema::DrcRuleSchema {
        id: "met1.spacing".into(),
        kind: "min_spacing".into(),
        enabled: true,
        layer: Some("met1".into()),
        min: Some(100),
        ..Default::default()
    };
    Deck::from_schema(VerifySchema {
        layers,
        drc_rules: vec![rule],
        pex: HashMap::new(),
        erc: params::ErcParams::default(),
        lvs: LvsSchema::default(),
        connectivity: schema::ConnectivitySchema::default(),
        devices: schema::DeviceSchema::default(),
    })
    .expect("deck")
}

/// n horizontal bars sharing one x-range: every pair is an x-sweep candidate.
/// Gaps cycle 85..115nm around the 100nm limit so the near-threshold set is fat.
fn bars(n: usize) -> (GeometryStore, usize) {
    let met1 = 0u16; // only layer in the deck
    let mut st = GeometryStore::new();
    let mut y = 0i32;
    let mut violations = 0usize;
    for i in 0..n {
        st.add_polygon(
            met1,
            &[(0, y), (20_000, y), (20_000, y + 200), (0, y + 200)],
        );
        let gap = 85 + 5 * (i as i32 % 7); // 85,90,95,100,105,110,115
        if i + 1 < n && gap < 100 {
            violations += 1;
        }
        y += 200 + gap;
    }
    (st, violations)
}

/// Two interlocked single-polygon combs, N fingers each, all gaps 150nm (clean).
/// 2 polys → ONE candidate pair with (4N)² edge pairs: bbox pruning is
/// structurally useless, every cycle is edge-pair distance work — the GPU case.
fn combs(n: usize) -> (GeometryStore, usize) {
    let met1 = 0u16;
    let (w, fx_a, fx_b) = (10_000, 9_600, 400); // A fingers 200→9600, B fingers 9800→400
    let mut st = GeometryStore::new();
    let mut a: Vec<(i32, i32)> = vec![(0, 0), (fx_a, 0)];
    let mut b: Vec<(i32, i32)> = vec![(w, 350), (fx_b, 350)];
    for i in 0..n as i32 {
        let (yt_a, yb_next_a) = (i * 700 + 200, (i + 1) * 700);
        let (yt_b, yb_next_b) = (i * 700 + 550, (i + 1) * 700 + 350);
        a.push((fx_a, yt_a));
        b.push((fx_b, yt_b));
        if i < n as i32 - 1 {
            a.extend([(200, yt_a), (200, yb_next_a), (fx_a, yb_next_a)]);
            b.extend([(w - 200, yt_b), (w - 200, yb_next_b), (fx_b, yb_next_b)]);
        } else {
            a.push((0, yt_a));
            b.push((w, yt_b));
        }
    }
    st.add_polygon(met1, &a);
    st.add_polygon(met1, &b);
    (st, 0) // every gap is 150 >= the 100nm rule: clean by construction
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "2000".into());
    let deck = deck();
    let (store, expect) = if let Some(nc) = arg.strip_prefix("combs:") {
        let n = nc.parse().unwrap_or(1000);
        println!(
            "mode=combs fingers={n} (edge-pair work ~{}M)",
            (4 * n) * (4 * n) / 1_000_000
        );
        combs(n)
    } else {
        let n: usize = arg.parse().unwrap_or(2000);
        println!("mode=bars n={n}");
        bars(n)
    };
    println!("expected_violations={expect}");

    let t = Instant::now();
    let cpu = run_drc_backend(&store, &deck, gpu::Backend::Cpu);
    let cpu_ms = t.elapsed().as_millis();
    println!("cpu: {} violations in {cpu_ms} ms", cpu.violations.len());
    assert_eq!(cpu.violations.len(), expect, "CPU count mismatch");

    if gpu::available_backends().contains(&gpu::Backend::Gpu) {
        // warm-up launch compiles the kernels; measure steady state
        let _ = run_drc_backend(&store, &deck, gpu::Backend::Gpu);
        let t = Instant::now();
        let g = run_drc_backend(&store, &deck, gpu::Backend::Gpu);
        let gpu_ms = t.elapsed().as_millis();
        println!(
            "gpu: {} violations in {gpu_ms} ms (steady-state)",
            g.violations.len()
        );
        let cm: Vec<i64> = cpu.violations.iter().map(|v| v.measured).collect();
        let gm: Vec<i64> = g.violations.iter().map(|v| v.measured).collect();
        assert_eq!(cm, gm, "GPU report must be bit-identical to CPU");
        println!(
            "reports identical: yes  speedup: {:.1}x",
            cpu_ms as f64 / gpu_ms.max(1) as f64
        );
    } else {
        println!("gpu: unavailable (no device/driver) — CPU baseline only");
    }
}
