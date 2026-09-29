//! Decks and sidecars compiled into the binary: a moved or packaged `philis`
//! loads its PDKs without GPurify's checkout or this repository on disk.
//!
//! The decks are vendored copies of GPurify's (`pdks/decks/*.deck`, line 1
//! names the upstream commit); every Philis edit carries a `# PHILIS:` line
//! above it, so an unmarked rule is upstream text.

/// `(file name a sidecar's "deck" names, deck text)`.
pub(crate) const DECKS: &[(&str, &str)] = &[
    ("sky130.deck", include_str!("../../../pdks/decks/sky130.deck")),
    ("gf180mcu.deck", include_str!("../../../pdks/decks/gf180mcu.deck")),
    ("ihp_sg13g2.deck", include_str!("../../../pdks/decks/ihp_sg13g2.deck")),
    ("generic_finfet.deck", include_str!("../../../pdks/decks/generic_finfet.deck")),
];

/// `(PDK name, sidecar JSON)`, what [`crate::Pdk::builtin`] loads.
pub(crate) const SIDECARS: &[(&str, &str)] = &[
    ("sky130", include_str!("../../../pdks/sky130.json")),
    ("gf180mcu", include_str!("../../../pdks/gf180mcu.json")),
    ("ihp_sg13g2", include_str!("../../../pdks/ihp_sg13g2.json")),
    ("generic_finfet", include_str!("../../../pdks/generic_finfet.json")),
];
