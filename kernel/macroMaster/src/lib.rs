//! # `macroMaster` — Substrate2, plus Philis
//!
//! The crate root re-exports [Substrate2](https://github.com/ucb-substrate/substrate2)
//! (`block`, `types`, `schematic`, `layout`, `simulation`, `context`, …) and
//! its IRs ([`layir`], [`scir`], [`spice`]). Write generators the Substrate
//! way; this crate adds what Philis brings on top of it:
//!
//! - [`signoff`]: in-process DRC/ERC/LVS/PEX of a Substrate block through
//!   GPurify, for any PDK `verify` has a deck for.
//!
//! Substrate's derives (`#[derive(Io, Block)]`) name the crate `substrate`
//! in the code they expand to, so a crate that uses them also depends on
//! `substrate` directly (`substrate = { workspace = true }`).
//!
//! [`legacy`] is the pre-Substrate API (`DeviceGen`/`Composition`), kept
//! until its callers move over (plan-09 P7).

pub use substrate::*;

pub use layir;
pub use scir;
pub use spice;

pub mod legacy;
pub mod signoff;
