//! # `macroMaster` — Substrate2, plus Philis
//!
//! The crate root re-exports [Substrate2](https://github.com/ucb-substrate/substrate2)
//! (`block`, `types`, `schematic`, `layout`, `simulation`, `context`, …) and
//! its IRs ([`layir`], [`scir`], [`spice`]). Write generators the Substrate
//! way; this crate adds what Philis brings on top of it:
//!
//! - [`pdk`]: typed PDKs for every Philis deck (gf180mcu, ihp_sg13g2, asap7
//!   generated from their decks; sky130 is upstream's), the [`pdk::Mos`]
//!   primitive on any of them, and the [`pdk::MosTile`] layout on the
//!   generated ones.
//! - [`signoff`]: in-process DRC/ERC/LVS/PEX of a Substrate block through
//!   GPurify, for any PDK `verify` has a deck for.
//!
//! Substrate's derives (`#[derive(Io, Block)]`) name the crate `substrate`
//! in the code they expand to, so a crate that uses them also depends on
//! `substrate` directly (`substrate = { workspace = true }`).
//!
//! [`legacy`] is the pre-Substrate API (`DeviceGen`/`Composition`) Philis's
//! flow builds on; moving Philis off it is Philis's business.

pub use substrate::*;

pub use layir;
pub use scir;
pub use spice;

pub mod legacy;
pub mod pdk;
pub mod signoff;
