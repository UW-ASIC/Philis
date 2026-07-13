//! ERC GPU kernel. Returns `None` without the `gpu` feature / a device;
//! callers fall back to the exact CPU path.

/// Per-query-point minimum squared distance to any target point (f32).
/// One thread per query; scans all targets. Used by missing_tie.
pub fn nearest_contact_dist2(
    px: &[i32], py: &[i32], cx: &[i32], cy: &[i32],
) -> Option<Vec<f32>> {
    #[cfg(feature = "gpu")] { return cube_impl::nearest_dist2(px, py, cx, cy); }
    #[cfg(not(feature = "gpu"))] { let _ = (px, py, cx, cy); None }
}

#[cfg(feature = "gpu")]
mod cube_impl {
    use crate::backend::cube::{client, contain, to_f32, CUBE_DIM};
    use cubecl::bytes::Bytes;
    use cubecl::cuda::CudaRuntime;
    use cubecl::prelude::*;

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
}
