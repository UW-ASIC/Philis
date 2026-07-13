//! Core verification traits and GPU acceleration.
//!
//! Every DRC, ERC, LVS, and PEX rule implements [`VerifyCheck`], which gives it a
//! uniform interface for CPU and GPU execution.
//!
//! ## GPU acceleration
//!
//! Written in CubeCL: kernels are plain Rust functions annotated `#[cube]`, compiled
//! at runtime for the selected backend (CUDA; flip cubecl's cargo feature for
//! ROCm/WGPU/Metal). The GPU only prunes work the exact path would have rejected
//! anyway (DRC) or computes the same values in parallel (ERC/LVS/PEX). No GPU, no
//! `gpu` feature, no driver => callers get `None` and silently run the full CPU path.

use crate::geometry::{Edge, GeometryStore};
use crate::params::Deck;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Where the per-element kernels run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Cpu,
    /// CubeCL (CUDA runtime). Requires the `gpu` feature + a device; falls back to CPU.
    Gpu,
}

/// A verification check that can run on CPU or GPU.
pub trait VerifyCheck {
    type Output;
    fn id(&self) -> &str;
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Self::Output;
}

// ---------------------------------------------------------------------------
// DRC GPU prefilters
// ---------------------------------------------------------------------------

/// Polygon-pair prefilter: per pair, 1 iff any edge pair is within thr2 (squared).
pub fn pair_near_flags(
    edges: &[Edge], descs: &[(u32, u32, u32, u32)], thr2: f32,
) -> Option<Vec<u32>> {
    #[cfg(feature = "gpu")] { return cube_impl::pair_near(edges, descs, thr2); }
    #[cfg(not(feature = "gpu"))] { let _ = (edges, descs, thr2); None }
}

/// Same-polygon facing-gap prefilter: per polygon, 1 iff any facing gap < thr.
pub fn poly_near_flags(edges: &[Edge], descs: &[(u32, u32)], thr: f32) -> Option<Vec<u32>> {
    #[cfg(feature = "gpu")] { return cube_impl::poly_near(edges, descs, thr); }
    #[cfg(not(feature = "gpu"))] { let _ = (edges, descs, thr); None }
}

/// Squared length per edge (f32 prefilter for min_edge_length).
pub fn approx_edge_len2(edges: &[Edge]) -> Option<Vec<f32>> {
    #[cfg(feature = "gpu")] { return cube_impl::edge_len2(edges); }
    #[cfg(not(feature = "gpu"))] { let _ = edges; None }
}

/// Per-edge angular deviation prefilter.
pub fn approx_angle_dev(edges: &[Edge], allowed_deg: &[i32]) -> Option<Vec<f32>> {
    #[cfg(feature = "gpu")] { return cube_impl::angle_dev(edges, allowed_deg); }
    #[cfg(not(feature = "gpu"))] { let _ = (edges, allowed_deg); None }
}

/// Exact per-vertex off-grid flags (integer remainder).
pub fn offgrid_flags(xs: &[i32], ys: &[i32], grid: i32) -> Option<Vec<u32>> {
    #[cfg(feature = "gpu")] { return cube_impl::offgrid(xs, ys, grid); }
    #[cfg(not(feature = "gpu"))] { let _ = (xs, ys, grid); None }
}

// ---------------------------------------------------------------------------
// ERC GPU kernel
// ---------------------------------------------------------------------------

/// Per-query-point minimum squared distance to any target point (f32).
/// One thread per query; scans all targets. Used by missing_tie.
pub fn nearest_contact_dist2(
    px: &[i32], py: &[i32], cx: &[i32], cy: &[i32],
) -> Option<Vec<f32>> {
    #[cfg(feature = "gpu")] { return cube_impl::nearest_dist2(px, py, cx, cy); }
    #[cfg(not(feature = "gpu"))] { let _ = (px, py, cx, cy); None }
}

// ---------------------------------------------------------------------------
// LVS GPU kernel
// ---------------------------------------------------------------------------

/// Per-pair positive-area bbox overlap flags. 1 = overlaps, 0 = disjoint.
pub fn bbox_overlap_flags(
    xmins: &[i32], ymins: &[i32], xmaxs: &[i32], ymaxs: &[i32],
    pairs_a: &[u32], pairs_b: &[u32],
) -> Option<Vec<u32>> {
    #[cfg(feature = "gpu")] {
        return cube_impl::bbox_overlap(xmins, ymins, xmaxs, ymaxs, pairs_a, pairs_b);
    }
    #[cfg(not(feature = "gpu"))] { let _ = (xmins, ymins, xmaxs, ymaxs, pairs_a, pairs_b); None }
}

// ---------------------------------------------------------------------------
// PEX GPU kernel
// ---------------------------------------------------------------------------

/// Per-pair parallel run length (nm) and gap (nm) for coupling cap.
/// gap < 0 means the pair is not facing (overlapping or non-parallel).
pub fn coupling_scan_gpu(
    xmins: &[f32], ymins: &[f32], xmaxs: &[f32], ymaxs: &[f32],
    pairs_a: &[u32], pairs_b: &[u32],
) -> Option<(Vec<f32>, Vec<f32>)> {
    #[cfg(feature = "gpu")] {
        return cube_impl::coupling_scan(xmins, ymins, xmaxs, ymaxs, pairs_a, pairs_b);
    }
    #[cfg(not(feature = "gpu"))] { let _ = (xmins, ymins, xmaxs, ymaxs, pairs_a, pairs_b); None }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Is a CUDA device actually usable right now?
pub fn gpu_ready() -> bool {
    #[cfg(feature = "gpu")] { return cube_impl::ready(); }
    #[cfg(not(feature = "gpu"))] false
}

/// Report which backends are usable in this build and on this machine.
pub fn available_backends() -> Vec<Backend> {
    let mut v = vec![Backend::Cpu];
    if gpu_ready() { v.push(Backend::Gpu); }
    v
}

// ---------------------------------------------------------------------------
// CubeCL implementation (feature = "gpu")
// ---------------------------------------------------------------------------

#[cfg(feature = "gpu")]
mod cube_impl {
    use crate::geometry::Edge;
    use cubecl::bytes::Bytes;
    use cubecl::cuda::{CudaDevice, CudaRuntime};
    use cubecl::prelude::*;
    use std::sync::OnceLock;

    // --- shared helpers ----------------------------------------------------

    #[cube]
    fn pt_seg_d2(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
        let vx = x1 - x0;
        let vy = y1 - y0;
        let wx = px - x0;
        let wy = py - y0;
        let c1 = vx * wx + vy * wy;
        let c2 = vx * vx + vy * vy;
        let mut r = wx * wx + wy * wy;
        if c1 > 0.0 {
            if c2 <= c1 {
                let dx = px - x1;
                let dy = py - y1;
                r = dx * dx + dy * dy;
            } else {
                let t = c1 / c2;
                let dx = wx - t * vx;
                let dy = wy - t * vy;
                r = dx * dx + dy * dy;
            }
        }
        r
    }

    #[cube]
    fn owner_of(off: &Array<u32>, n: u32, i: u32) -> u32 {
        let mut lo = 0u32;
        let mut hi = n - 1;
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            if off[mid as usize] <= i { lo = mid; } else { hi = mid - 1; }
        }
        lo
    }

    // --- DRC kernels -------------------------------------------------------

    #[cube(launch_unchecked)]
    fn pair_near_kernel(
        ex0: &Array<f32>, ey0: &Array<f32>, ex1: &Array<f32>, ey1: &Array<f32>,
        a0: &Array<u32>, b0: &Array<u32>, blen: &Array<u32>, off: &Array<u32>,
        n_pairs: u32, total: u32, thr2: f32,
        flags: &mut Array<u32>,
    ) {
        // 1:1 thread:eval. Tiling (8 evals/thread) was measured SLOWER on a 4060
        // (16ms vs 10ms on 256M evals) — the unrolled body raises register
        // pressure and drops occupancy; the div/mod it saves is cheaper.
        let i = ABSOLUTE_POS as u32;
        if i < total {
            let k = owner_of(off, n_pairs, i);
            let local = i - off[k as usize];
            let bl = blen[k as usize];
            let a = (a0[k as usize] + local / bl) as usize;
            let b = (b0[k as usize] + local % bl) as usize;
            let d0 = pt_seg_d2(ex0[a], ey0[a], ex0[b], ey0[b], ex1[b], ey1[b]);
            let d1 = pt_seg_d2(ex1[a], ey1[a], ex0[b], ey0[b], ex1[b], ey1[b]);
            let d2 = pt_seg_d2(ex0[b], ey0[b], ex0[a], ey0[a], ex1[a], ey1[a]);
            let d3 = pt_seg_d2(ex1[b], ey1[b], ex0[a], ey0[a], ex1[a], ey1[a]);
            let d = f32::min(f32::min(d0, d1), f32::min(d2, d3));
            if d < thr2 { flags[k as usize] = 1u32; }
        }
    }

    #[cube(launch_unchecked)]
    fn poly_near_kernel(
        ex0: &Array<f32>, ey0: &Array<f32>, ex1: &Array<f32>, ey1: &Array<f32>,
        estart: &Array<u32>, elen: &Array<u32>, off: &Array<u32>,
        n_polys: u32, total: u32, thr: f32,
        flags: &mut Array<u32>,
    ) {
        let i = ABSOLUTE_POS as u32;
        if i < total {
            let k = owner_of(off, n_polys, i);
            let local = i - off[k as usize];
            let el = elen[k as usize];
            let ur = local / el;
            let vr = local % el;
            if vr > ur {
                let a = (estart[k as usize] + ur) as usize;
                let b = (estart[k as usize] + vr) as usize;
                let mut d = 1e30f32;
                if ex0[a] == ex1[a] && ex0[b] == ex1[b] {
                    let lo = f32::max(f32::min(ey0[a], ey1[a]), f32::min(ey0[b], ey1[b]));
                    let hi = f32::min(f32::max(ey0[a], ey1[a]), f32::max(ey0[b], ey1[b]));
                    if lo < hi { d = f32::abs(ex0[a] - ex0[b]); }
                }
                if ey0[a] == ey1[a] && ey0[b] == ey1[b] {
                    let lo = f32::max(f32::min(ex0[a], ex1[a]), f32::min(ex0[b], ex1[b]));
                    let hi = f32::min(f32::max(ex0[a], ex1[a]), f32::max(ex0[b], ex1[b]));
                    if lo < hi { d = f32::abs(ey0[a] - ey0[b]); }
                }
                if d > 0.0 && d < thr { flags[k as usize] = 1u32; }
            }
        }
    }

    #[cube(launch_unchecked)]
    fn edge_len2_kernel(
        ex0: &Array<f32>, ey0: &Array<f32>, ex1: &Array<f32>, ey1: &Array<f32>,
        out: &mut Array<f32>,
    ) {
        if ABSOLUTE_POS < out.len() {
            let dx = ex1[ABSOLUTE_POS] - ex0[ABSOLUTE_POS];
            let dy = ey1[ABSOLUTE_POS] - ey0[ABSOLUTE_POS];
            out[ABSOLUTE_POS] = dx * dx + dy * dy;
        }
    }

    #[cube(launch_unchecked)]
    fn angle_dev_kernel(
        ex0: &Array<f32>, ey0: &Array<f32>, ex1: &Array<f32>, ey1: &Array<f32>,
        sin_a: &Array<f32>, cos_a: &Array<f32>, n_allowed: u32,
        out: &mut Array<f32>,
    ) {
        if ABSOLUTE_POS < out.len() {
            let dx = ex1[ABSOLUTE_POS] - ex0[ABSOLUTE_POS];
            let dy = ey1[ABSOLUTE_POS] - ey0[ABSOLUTE_POS];
            let len = f32::sqrt(dx * dx + dy * dy);
            let mut best = 1e30f32;
            if len > 0.0 {
                for i in 0..n_allowed {
                    let dev = f32::abs(dx * sin_a[i as usize] - dy * cos_a[i as usize]) / len;
                    best = f32::min(best, dev);
                }
            } else {
                best = 0.0;
            }
            out[ABSOLUTE_POS] = best;
        }
    }

    #[cube(launch_unchecked)]
    fn offgrid_kernel(xs: &Array<i32>, ys: &Array<i32>, grid: i32, out: &mut Array<u32>) {
        if ABSOLUTE_POS < out.len() {
            let mut f = 0u32;
            if xs[ABSOLUTE_POS] % grid != 0 || ys[ABSOLUTE_POS] % grid != 0 { f = 1u32; }
            out[ABSOLUTE_POS] = f;
        }
    }

    // --- ERC kernel --------------------------------------------------------

    #[cube(launch_unchecked)]
    fn nearest_dist2_kernel(
        px: &Array<f32>, py: &Array<f32>,
        cx: &Array<f32>, cy: &Array<f32>,
        n_targets: u32,
        out: &mut Array<f32>,
    ) {
        if ABSOLUTE_POS < out.len() {
            let qx = px[ABSOLUTE_POS];
            let qy = py[ABSOLUTE_POS];
            let mut best = 1e30f32;
            for t in 0..n_targets {
                let dx = qx - cx[t as usize];
                let dy = qy - cy[t as usize];
                let d2 = dx * dx + dy * dy;
                best = f32::min(best, d2);
            }
            out[ABSOLUTE_POS] = best;
        }
    }

    // --- LVS kernel --------------------------------------------------------

    #[cube(launch_unchecked)]
    fn overlap_kernel(
        xmin: &Array<i32>, ymin: &Array<i32>, xmax: &Array<i32>, ymax: &Array<i32>,
        pa: &Array<u32>, pb: &Array<u32>,
        flags: &mut Array<u32>,
    ) {
        if ABSOLUTE_POS < flags.len() {
            let a = pa[ABSOLUTE_POS] as usize;
            let b = pb[ABSOLUTE_POS] as usize;
            let mut x0 = xmin[a]; if xmin[b] > x0 { x0 = xmin[b]; }
            let mut x1 = xmax[a]; if xmax[b] < x1 { x1 = xmax[b]; }
            let mut y0 = ymin[a]; if ymin[b] > y0 { y0 = ymin[b]; }
            let mut y1 = ymax[a]; if ymax[b] < y1 { y1 = ymax[b]; }
            let mut f = 0u32;
            if x1 > x0 && y1 > y0 { f = 1u32; }
            flags[ABSOLUTE_POS] = f;
        }
    }

    // --- PEX kernel --------------------------------------------------------

    #[cube(launch_unchecked)]
    fn coupling_kernel(
        bxmin: &Array<f32>, bymin: &Array<f32>, bxmax: &Array<f32>, bymax: &Array<f32>,
        pa: &Array<u32>, pb: &Array<u32>,
        run_out: &mut Array<f32>, gap_out: &mut Array<f32>,
    ) {
        if ABSOLUTE_POS < run_out.len() {
            let a = pa[ABSOLUTE_POS] as usize;
            let b = pb[ABSOLUTE_POS] as usize;
            // cubecl 0.10's cube macro cannot expand const-const float arithmetic
            // (`-1.0f32` / `0.0 - 1.0`), so "not facing" is encoded as gap = 0.0 —
            // callers only accept gap > 0.0, and a facing pair with zero gap is
            // touching, which they reject too. Locals + f32::max-wrapped mut
            // assignments dodge the same macro's From<NativeExpand<f32>> gap.
            let axmin = bxmin[a]; let oxmin = bxmin[b];
            let axmax = bxmax[a]; let oxmax = bxmax[b];
            let aymin = bymin[a]; let oymin = bymin[b];
            let aymax = bymax[a]; let oymax = bymax[b];
            let mut run = 0.0f32;
            let mut gap = 0.0f32;
            let x_overlap = f32::min(axmax, oxmax) - f32::max(axmin, oxmin);
            if x_overlap > 0.0 {
                if aymax <= oymin {
                    run = f32::max(x_overlap, 0.0);
                    gap = f32::max(oymin - aymax, 0.0);
                } else if oymax <= aymin {
                    run = f32::max(x_overlap, 0.0);
                    gap = f32::max(aymin - oymax, 0.0);
                }
            }
            if gap <= 0.0 {
                let y_overlap = f32::min(aymax, oymax) - f32::max(aymin, oymin);
                if y_overlap > 0.0 {
                    if axmax <= oxmin {
                        run = f32::max(y_overlap, 0.0);
                        gap = f32::max(oxmin - axmax, 0.0);
                    } else if oxmax <= axmin {
                        run = f32::max(y_overlap, 0.0);
                        gap = f32::max(axmin - oxmax, 0.0);
                    }
                }
            }
            run_out[ABSOLUTE_POS] = run;
            gap_out[ABSOLUTE_POS] = gap;
        }
    }

    // --- infrastructure ----------------------------------------------------

    const CHUNK: usize = 16 << 20;
    const CUBE_DIM: u32 = 256;

    type Client = ComputeClient<CudaRuntime>;

    fn client() -> Option<&'static Client> {
        static C: OnceLock<Option<Client>> = OnceLock::new();
        C.get_or_init(|| {
            std::panic::catch_unwind(|| CudaRuntime::client(&CudaDevice::new(0))).ok()
        })
        .as_ref()
    }

    pub fn ready() -> bool { client().is_some() }

    fn contain<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(v) => {
                if v.is_none() && std::env::var_os("GDSVERIFY_GPU_LOG").is_some() {
                    eprintln!("gdsverify gpu: call returned None (non-panic failure)");
                }
                v
            }
            Err(p) => {
                if std::env::var_os("GDSVERIFY_GPU_LOG").is_some() {
                    let msg = p.downcast_ref::<String>().map(|s| s.as_str())
                        .or_else(|| p.downcast_ref::<&str>().copied())
                        .unwrap_or("<non-string panic>");
                    eprintln!("gdsverify gpu: panic: {msg}");
                }
                None
            }
        }
    }

    /// Upload an edge pool, memoized by CONTENT hash: the same layer's edge set
    /// is uploaded by several rules per run (spacing, width/notch mask, c2c, eol)
    /// and by every run in a steady-state loop. Full-content hashing (not sampling)
    /// keeps this sound — a false hit is impossible without a 64-bit collision on
    /// the exact coordinate stream. Bounded to 16 pools, evicting oldest.
    fn upload_edges(client: &Client, edges: &[Edge]) -> [cubecl::server::Handle; 4] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::sync::Mutex;
        type Pool = (u64, [cubecl::server::Handle; 4]);
        static CACHE: Mutex<Vec<Pool>> = Mutex::new(Vec::new());

        let mut h = DefaultHasher::new();
        edges.len().hash(&mut h);
        for e in edges {
            (e.x0, e.y0, e.x1, e.y1).hash(&mut h);
        }
        let key = h.finish();

        let mut cache = CACHE.lock().unwrap();
        if let Some((_, hs)) = cache.iter().find(|(k, _)| *k == key) {
            let t0 = std::time::Instant::now();
            let hs = hs.clone();
            log_time("edge-pool cache hit", t0);
            return hs;
        }
        let t0 = std::time::Instant::now();
        let up = |get: fn(&Edge) -> i32| {
            client.create(Bytes::from_elems(
                edges.iter().map(|e| get(e) as f32).collect::<Vec<f32>>()))
        };
        let hs = [up(|e| e.x0), up(|e| e.y0), up(|e| e.x1), up(|e| e.y1)];
        log_time(&format!("edge-pool upload ({} edges)", edges.len()), t0);
        if cache.len() >= 16 { cache.remove(0); }
        cache.push((key, hs.clone()));
        hs
    }

    fn log_time(what: &str, t0: std::time::Instant) {
        if std::env::var_os("GDSVERIFY_GPU_LOG").is_some() {
            eprintln!("gdsverify gpu: {what}: {:?}", t0.elapsed());
        }
    }

    fn to_f32(b: &[u8], n: usize) -> Vec<f32> {
        b.chunks_exact(4).take(n).map(|c| f32::from_ne_bytes(c.try_into().unwrap())).collect()
    }

    fn to_u32(b: &[u8], n: usize) -> Vec<u32> {
        b.chunks_exact(4).take(n).map(|c| u32::from_ne_bytes(c.try_into().unwrap())).collect()
    }

    // --- DRC host launchers ------------------------------------------------

    pub fn pair_near(
        edges: &[Edge], descs: &[(u32, u32, u32, u32)], thr2: f32,
    ) -> Option<Vec<u32>> {
        let client = client()?;
        if descs.is_empty() { return Some(Vec::new()); }
        let run = || -> Option<Vec<u32>> {
            let ne = edges.len();
            let [ex0, ey0, ex1, ey1] = upload_edges(client, edges);
            // two-phase: queue every chunk's kernel first, read results after —
            // interleaving launch/read serializes the device on chunked scans.
            let mut pending: Vec<(cubecl::server::Handle, usize)> = Vec::new();
            let mut idx = 0;
            while idx < descs.len() {
                let (mut a0s, mut b0s, mut blens, mut offs) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                let mut total: u64 = 0;
                while idx < descs.len() && (total as usize) < CHUNK {
                    let (a0, a1, b0, b1) = descs[idx];
                    offs.push(total as u32);
                    a0s.push(a0); b0s.push(b0); blens.push(b1 - b0);
                    total += ((a1 - a0) as u64) * ((b1 - b0) as u64);
                    idx += 1;
                }
                let m = offs.len();
                let ha = client.create(Bytes::from_elems(a0s));
                let hb = client.create(Bytes::from_elems(b0s));
                let hl = client.create(Bytes::from_elems(blens));
                let ho = client.create(Bytes::from_elems(offs));
                let hf = client.create(Bytes::from_elems(vec![0u32; m]));
                unsafe {
                    pair_near_kernel::launch_unchecked::<CudaRuntime>(
                        client,
                        CubeCount::Static((total as u32).div_ceil(CUBE_DIM), 1, 1),
                        CubeDim::new_1d(CUBE_DIM),
                        ArrayArg::from_raw_parts(ex0.clone(), ne),
                        ArrayArg::from_raw_parts(ey0.clone(), ne),
                        ArrayArg::from_raw_parts(ex1.clone(), ne),
                        ArrayArg::from_raw_parts(ey1.clone(), ne),
                        ArrayArg::from_raw_parts(ha, m),
                        ArrayArg::from_raw_parts(hb, m),
                        ArrayArg::from_raw_parts(hl, m),
                        ArrayArg::from_raw_parts(ho, m),
                        m as u32, total as u32, thr2,
                        ArrayArg::from_raw_parts(hf.clone(), m),
                    );
                }
                pending.push((hf, m));
            }
            let t0 = std::time::Instant::now();
            let mut flags = Vec::with_capacity(descs.len());
            for (hf, m) in pending {
                flags.extend(to_u32(&client.read_one(hf).ok()?, m));
            }
            log_time("pair_near read-back", t0);
            Some(flags)
        };
        contain(run)
    }

    pub fn poly_near(edges: &[Edge], descs: &[(u32, u32)], thr: f32) -> Option<Vec<u32>> {
        let client = client()?;
        if descs.is_empty() { return Some(Vec::new()); }
        let run = || -> Option<Vec<u32>> {
            let ne = edges.len();
            let [ex0, ey0, ex1, ey1] = upload_edges(client, edges);
            // two-phase: queue every chunk's kernel first, read results after —
            // interleaving launch/read serializes the device on chunked scans.
            let mut pending: Vec<(cubecl::server::Handle, usize)> = Vec::new();
            let mut idx = 0;
            while idx < descs.len() {
                let (mut starts, mut elens, mut offs) = (Vec::new(), Vec::new(), Vec::new());
                let mut total: u64 = 0;
                while idx < descs.len() && (total as usize) < CHUNK {
                    let (s, e) = descs[idx];
                    offs.push(total as u32);
                    starts.push(s); elens.push(e - s);
                    total += ((e - s) as u64) * ((e - s) as u64);
                    idx += 1;
                }
                let m = offs.len();
                let hs = client.create(Bytes::from_elems(starts));
                let hl = client.create(Bytes::from_elems(elens));
                let ho = client.create(Bytes::from_elems(offs));
                let hf = client.create(Bytes::from_elems(vec![0u32; m]));
                unsafe {
                    poly_near_kernel::launch_unchecked::<CudaRuntime>(
                        client,
                        CubeCount::Static((total as u32).div_ceil(CUBE_DIM), 1, 1),
                        CubeDim::new_1d(CUBE_DIM),
                        ArrayArg::from_raw_parts(ex0.clone(), ne),
                        ArrayArg::from_raw_parts(ey0.clone(), ne),
                        ArrayArg::from_raw_parts(ex1.clone(), ne),
                        ArrayArg::from_raw_parts(ey1.clone(), ne),
                        ArrayArg::from_raw_parts(hs, m),
                        ArrayArg::from_raw_parts(hl, m),
                        ArrayArg::from_raw_parts(ho, m),
                        m as u32, total as u32, thr,
                        ArrayArg::from_raw_parts(hf.clone(), m),
                    );
                }
                pending.push((hf, m));
            }
            let mut flags = Vec::with_capacity(descs.len());
            for (hf, m) in pending {
                flags.extend(to_u32(&client.read_one(hf).ok()?, m));
            }
            Some(flags)
        };
        contain(run)
    }

    pub fn edge_len2(edges: &[Edge]) -> Option<Vec<f32>> {
        let client = client()?;
        if edges.is_empty() { return Some(Vec::new()); }
        let run = || -> Option<Vec<f32>> {
            let ne = edges.len();
            let [ex0, ey0, ex1, ey1] = upload_edges(client, edges);
            let out = client.empty(ne * core::mem::size_of::<f32>());
            unsafe {
                edge_len2_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((ne as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(ex0, ne), ArrayArg::from_raw_parts(ey0, ne),
                    ArrayArg::from_raw_parts(ex1, ne), ArrayArg::from_raw_parts(ey1, ne),
                    ArrayArg::from_raw_parts(out.clone(), ne),
                );
            }
            Some(to_f32(&client.read_one(out).ok()?, ne))
        };
        contain(run)
    }

    pub fn angle_dev(edges: &[Edge], allowed_deg: &[i32]) -> Option<Vec<f32>> {
        let client = client()?;
        if edges.is_empty() || allowed_deg.is_empty() { return Some(vec![0.0; edges.len()]); }
        let run = || -> Option<Vec<f32>> {
            let ne = edges.len();
            let na = allowed_deg.len();
            let [ex0, ey0, ex1, ey1] = upload_edges(client, edges);
            let sins: Vec<f32> = allowed_deg.iter().map(|&a| (a as f32).to_radians().sin()).collect();
            let coss: Vec<f32> = allowed_deg.iter().map(|&a| (a as f32).to_radians().cos()).collect();
            let hs = client.create(Bytes::from_elems(sins));
            let hc = client.create(Bytes::from_elems(coss));
            let out = client.empty(ne * core::mem::size_of::<f32>());
            unsafe {
                angle_dev_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((ne as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(ex0, ne), ArrayArg::from_raw_parts(ey0, ne),
                    ArrayArg::from_raw_parts(ex1, ne), ArrayArg::from_raw_parts(ey1, ne),
                    ArrayArg::from_raw_parts(hs, na), ArrayArg::from_raw_parts(hc, na),
                    na as u32,
                    ArrayArg::from_raw_parts(out.clone(), ne),
                );
            }
            Some(to_f32(&client.read_one(out).ok()?, ne))
        };
        contain(run)
    }

    pub fn offgrid(xs: &[i32], ys: &[i32], grid: i32) -> Option<Vec<u32>> {
        let client = client()?;
        if xs.is_empty() { return Some(Vec::new()); }
        let run = || -> Option<Vec<u32>> {
            let n = xs.len();
            let hx = client.create(Bytes::from_elems(xs.to_vec()));
            let hy = client.create(Bytes::from_elems(ys.to_vec()));
            let out = client.empty(n * core::mem::size_of::<u32>());
            unsafe {
                offgrid_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((n as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(hx, n), ArrayArg::from_raw_parts(hy, n),
                    grid,
                    ArrayArg::from_raw_parts(out.clone(), n),
                );
            }
            Some(to_u32(&client.read_one(out).ok()?, n))
        };
        contain(run)
    }

    // --- ERC host launcher -------------------------------------------------

    pub fn nearest_dist2(
        px: &[i32], py: &[i32], cx: &[i32], cy: &[i32],
    ) -> Option<Vec<f32>> {
        let client = client()?;
        let n_q = px.len();
        let n_t = cx.len();
        if n_q == 0 { return Some(Vec::new()); }
        if n_t == 0 { return Some(vec![f32::MAX; n_q]); }
        let run = || -> Option<Vec<f32>> {
            let hpx = client.create(Bytes::from_elems(px.iter().map(|&x| x as f32).collect::<Vec<f32>>()));
            let hpy = client.create(Bytes::from_elems(py.iter().map(|&y| y as f32).collect::<Vec<f32>>()));
            let hcx = client.create(Bytes::from_elems(cx.iter().map(|&x| x as f32).collect::<Vec<f32>>()));
            let hcy = client.create(Bytes::from_elems(cy.iter().map(|&y| y as f32).collect::<Vec<f32>>()));
            let out = client.empty(n_q * core::mem::size_of::<f32>());
            unsafe {
                nearest_dist2_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((n_q as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(hpx, n_q), ArrayArg::from_raw_parts(hpy, n_q),
                    ArrayArg::from_raw_parts(hcx, n_t), ArrayArg::from_raw_parts(hcy, n_t),
                    n_t as u32,
                    ArrayArg::from_raw_parts(out.clone(), n_q),
                );
            }
            Some(to_f32(&client.read_one(out).ok()?, n_q))
        };
        contain(run)
    }

    // --- LVS host launcher -------------------------------------------------

    pub fn bbox_overlap(
        xmins: &[i32], ymins: &[i32], xmaxs: &[i32], ymaxs: &[i32],
        pairs_a: &[u32], pairs_b: &[u32],
    ) -> Option<Vec<u32>> {
        let client = client()?;
        let m = pairs_a.len();
        if m == 0 { return Some(Vec::new()); }
        let run = || -> Option<Vec<u32>> {
            let n = xmins.len();
            let hxn = client.create(Bytes::from_elems(xmins.to_vec()));
            let hyn = client.create(Bytes::from_elems(ymins.to_vec()));
            let hxx = client.create(Bytes::from_elems(xmaxs.to_vec()));
            let hyx = client.create(Bytes::from_elems(ymaxs.to_vec()));
            let hpa = client.create(Bytes::from_elems(pairs_a.to_vec()));
            let hpb = client.create(Bytes::from_elems(pairs_b.to_vec()));
            let flags = client.create(Bytes::from_elems(vec![0u32; m]));
            unsafe {
                overlap_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((m as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(hxn, n), ArrayArg::from_raw_parts(hyn, n),
                    ArrayArg::from_raw_parts(hxx, n), ArrayArg::from_raw_parts(hyx, n),
                    ArrayArg::from_raw_parts(hpa, m), ArrayArg::from_raw_parts(hpb, m),
                    ArrayArg::from_raw_parts(flags.clone(), m),
                );
            }
            Some(to_u32(&client.read_one(flags).ok()?, m))
        };
        contain(run)
    }

    // --- PEX host launcher -------------------------------------------------

    pub fn coupling_scan(
        xmins: &[f32], ymins: &[f32], xmaxs: &[f32], ymaxs: &[f32],
        pairs_a: &[u32], pairs_b: &[u32],
    ) -> Option<(Vec<f32>, Vec<f32>)> {
        let client = client()?;
        let m = pairs_a.len();
        if m == 0 { return Some((Vec::new(), Vec::new())); }
        let run = || -> Option<(Vec<f32>, Vec<f32>)> {
            let n = xmins.len();
            let hxn = client.create(Bytes::from_elems(xmins.to_vec()));
            let hyn = client.create(Bytes::from_elems(ymins.to_vec()));
            let hxx = client.create(Bytes::from_elems(xmaxs.to_vec()));
            let hyx = client.create(Bytes::from_elems(ymaxs.to_vec()));
            let hpa = client.create(Bytes::from_elems(pairs_a.to_vec()));
            let hpb = client.create(Bytes::from_elems(pairs_b.to_vec()));
            let run_out = client.empty(m * core::mem::size_of::<f32>());
            let gap_out = client.empty(m * core::mem::size_of::<f32>());
            unsafe {
                coupling_kernel::launch_unchecked::<CudaRuntime>(
                    client,
                    CubeCount::Static((m as u32).div_ceil(CUBE_DIM), 1, 1),
                    CubeDim::new_1d(CUBE_DIM),
                    ArrayArg::from_raw_parts(hxn, n), ArrayArg::from_raw_parts(hyn, n),
                    ArrayArg::from_raw_parts(hxx, n), ArrayArg::from_raw_parts(hyx, n),
                    ArrayArg::from_raw_parts(hpa, m), ArrayArg::from_raw_parts(hpb, m),
                    ArrayArg::from_raw_parts(run_out.clone(), m),
                    ArrayArg::from_raw_parts(gap_out.clone(), m),
                );
            }
            let runs = to_f32(&client.read_one(run_out).ok()?, m);
            let gaps = to_f32(&client.read_one(gap_out).ok()?, m);
            Some((runs, gaps))
        };
        contain(run)
    }
}
