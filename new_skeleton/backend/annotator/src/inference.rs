//! Inference recognition — heuristic pattern matching over what strict left
//! unclaimed. Pluggable and **stubbed for now**; the real inference lands later
//! (see the cell-variants / recognition research ADR). Strict carries the load
//! until then.

use crate::block::Block;
use pnr_core::Netlist;

/// A pluggable block-inference strategy — drop-in, like the placer/router, so a
/// future recogniser slots in without touching the rest.
pub trait BlockInference {
    /// Infer additional blocks from the devices `claimed` (strict matches) left
    /// unassigned.
    fn infer(&self, netlist: &Netlist, claimed: &[Block]) -> Vec<Block>;
}

/// The default: no inference yet. Recognises nothing, so only strict-matched
/// structure is annotated. Swap in a real strategy when inference is implemented.
#[derive(Default)]
pub struct NoInference;

impl BlockInference for NoInference {
    fn infer(&self, netlist: &Netlist, claimed: &[Block]) -> Vec<Block> {
        let _ = (netlist, claimed);
        Vec::new()
    }
}
