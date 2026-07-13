//! PEX GPU kernel. Returns `None` without the `gpu` feature / a device;
//! callers fall back to the exact CPU path.

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

#[cfg(feature = "gpu")]
mod cube_impl {
    use crate::backend::cube::{client, contain, to_f32, CUBE_DIM};
    use cubecl::bytes::Bytes;
    use cubecl::cuda::CudaRuntime;
    use cubecl::prelude::*;

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
