//! Execution backend selection and the shared CubeCL runtime plumbing.
//!
//! Engine-specific GPU kernels live with their engines (`drc::gpu`, `erc::gpu`,
//! `lvs::gpu`, `pex::gpu`); this module only owns the backend enum and the
//! runtime pieces every engine shares: the memoized CUDA client, panic
//! containment, and readback helpers. Kernels are plain Rust `#[cube]`
//! functions compiled at runtime for the selected backend (CUDA; flip
//! cubecl's cargo feature for ROCm/WGPU/Metal). No GPU, no `gpu` feature, no
//! driver => engine wrappers return `None` and callers silently run the full
//! CPU path.

/// Where the per-element kernels run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Cpu,
    /// CubeCL (CUDA runtime). Requires the `gpu` feature + a device; falls back to CPU.
    Gpu,
}

/// Is a CUDA device actually usable right now?
pub fn gpu_ready() -> bool {
    #[cfg(feature = "gpu")] { return cube::ready(); }
    #[cfg(not(feature = "gpu"))] false
}

/// Report which backends are usable in this build and on this machine.
pub fn available_backends() -> Vec<Backend> {
    let mut v = vec![Backend::Cpu];
    if gpu_ready() { v.push(Backend::Gpu); }
    v
}

/// Shared CubeCL runtime infrastructure for the engine kernel modules.
#[cfg(feature = "gpu")]
pub(crate) mod cube {
    use cubecl::cuda::{CudaDevice, CudaRuntime};
    use cubecl::prelude::*;
    use std::sync::OnceLock;

    pub(crate) const CUBE_DIM: u32 = 256;

    pub(crate) type Client = ComputeClient<CudaRuntime>;

    pub(crate) fn client() -> Option<&'static Client> {
        static C: OnceLock<Option<Client>> = OnceLock::new();
        C.get_or_init(|| {
            std::panic::catch_unwind(|| CudaRuntime::client(&CudaDevice::new(0))).ok()
        })
        .as_ref()
    }

    pub(crate) fn ready() -> bool { client().is_some() }

    /// Contain kernel/driver panics: a GPU failure must degrade to the CPU
    /// path (`None`), never abort a verification run.
    pub(crate) fn contain<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
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

    pub(crate) fn log_time(what: &str, t0: std::time::Instant) {
        if std::env::var_os("GDSVERIFY_GPU_LOG").is_some() {
            eprintln!("gdsverify gpu: {what}: {:?}", t0.elapsed());
        }
    }

    pub(crate) fn to_f32(b: &[u8], n: usize) -> Vec<f32> {
        b.chunks_exact(4).take(n).map(|c| f32::from_ne_bytes(c.try_into().unwrap())).collect()
    }

    pub(crate) fn to_u32(b: &[u8], n: usize) -> Vec<u32> {
        b.chunks_exact(4).take(n).map(|c| u32::from_ne_bytes(c.try_into().unwrap())).collect()
    }
}
