//! Layout I/O: format-agnostic reading and the PDK rule schema.
//!
//! * [`read`] — GDS/OASIS decoding into one lossless record database, with a
//!   format-agnostic wrapper ([`read::read_layout`]) and checked flattening.
//! * [`schema`] — PDK rules (input schema + resolved deck) consumed by the
//!   rule engines through their contexts.
pub mod read;
pub mod schema;
