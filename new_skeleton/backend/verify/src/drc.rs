//! DRC — design-rule check over drawn geometry via gdsverify.
//!
//! Two entry points, one engine:
//! - [`run_drc`] — **signoff**: the full deck including density windows.
//! - [`run_drc_inloop`] — **feedback**: density waived (meaningless mid-iteration)
//!   and routed through the **GPU backend** when the `gpu` feature is on, so the
//!   P&R loop can DRC every iteration cheaply. Verdicts are identical to CPU;
//!   the GPU path is an exact-result spacing prefilter (see gdsverify).

use pnr_core::Shape;

use crate::geom::store_from_shapes;
use crate::pdk::Pdk;

pub use gdsverify::{DrcReport, Violation};

/// Full signoff DRC over drawn geometry. Includes density rules.
#[must_use]
pub fn run_drc(shapes: &[Shape], pdk: &Pdk) -> DrcReport {
    let store = store_from_shapes(shapes, pdk);
    gdsverify::run_drc(&store, &pdk.deck)
}

/// In-loop DRC: density waived, GPU backend when available. This is the path the
/// orchestrator runs each iteration to feed [`crate::drc_feedback`].
#[must_use]
pub fn run_drc_inloop(shapes: &[Shape], pdk: &Pdk) -> DrcReport {
    let store = store_from_shapes(shapes, pdk);
    gdsverify::run_drc_no_density_backend(&store, &pdk.deck, inloop_backend())
}

/// GPU when built with `gpu` (devshell + CUDA/Vulkan), else CPU. Memory: this is
/// the in-loop GPU DRC backend the signoff-hardening note refers to.
#[inline]
#[must_use]
pub fn inloop_backend() -> gdsverify::Backend {
    #[cfg(feature = "gpu")]
    {
        gdsverify::Backend::Gpu
    }
    #[cfg(not(feature = "gpu"))]
    {
        gdsverify::Backend::Cpu
    }
}
