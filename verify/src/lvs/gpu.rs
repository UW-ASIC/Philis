//! LVS GPU kernel. Returns `None` without the `gpu` feature / a device;
//! callers fall back to the exact CPU path.

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

#[cfg(feature = "gpu")]
mod cube_impl {
    use crate::backend::cube::{client, contain, to_u32, CUBE_DIM};
    use cubecl::bytes::Bytes;
    use cubecl::cuda::CudaRuntime;
    use cubecl::prelude::*;

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
}
