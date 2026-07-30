//! LVS — layout-versus-schematic over drawn geometry via gdsverify.
//!
//! Extract a netlist from the drawn geometry and [`compare`](gdsverify::compare)
//! it against a reference. Unlike DRC/ERC/PEX, LVS needs the schematic
//! ([`RefNetlist`]) as a second input, so it is not a pure `(&geometry, &Pdk)`
//! call — the caller threads the reference through.

use pnr_core::Shape;

use crate::geom::store_from_shapes;
use crate::pdk::Pdk;

pub use gdsverify::{LvsResult, RefNetlist};

/// Extract-and-compare drawn geometry against `reference`.
#[must_use]
pub fn run_lvs(shapes: &[Shape], pdk: &Pdk, reference: &RefNetlist) -> LvsResult {
    let store = store_from_shapes(shapes, pdk);
    gdsverify::run_lvs(&store, &pdk.deck, reference)
}

/// **Extract only** — how many distinct devices the extractor sees in `shapes`,
/// with no reference netlist to compare against.
///
/// `None` means extraction itself failed, and the interesting failure is exactly
/// the one PLAN §3c names: a channel polygon that "ambiguously matches MOS
/// rules" because a placement move merged two devices' implants — gdsverify
/// aborts extraction on that, so the abort *is* the ambiguity signal. The
/// count sums all three device tables (MOS, BJT, two-terminal) because a merge
/// can also make one of them vanish into another.
///
/// This is [`crate::LiveOracle::merge_ambiguous`]'s whole implementation:
/// `None`-or-mismatch ⇒ ambiguous. gdsverify exposes `extract_netlist`
/// directly, so no comparison against a dummy reference is needed.
#[must_use]
pub fn extract_device_count(shapes: &[Shape], pdk: &Pdk) -> Option<usize> {
    let store = store_from_shapes(shapes, pdk);
    gdsverify::extract_netlist(&store, &pdk.deck)
        .ok()
        .map(|e| e.devices.len() + e.bjt_devices.len() + e.two_terminal.len())
}
