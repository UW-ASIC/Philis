//! Format boundary: everything that parses or serializes external inputs.
//!
//! GDS (legacy DRC-compat reader and the lossless strict reader/writer),
//! OASIS, the JSON verify schema, and deck/parameter resolution. Engines
//! consume the parsed, validated outputs; they never touch bytes.
pub mod gds;
pub mod gds_lossless;
pub mod oasis;
pub mod params;
pub mod schema;
