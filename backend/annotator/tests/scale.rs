//! T8 (Philis policy): `annotate` on 12,500 devices within 2.0 s, release.
//! Fails today. M0: no return after 13.5 min. After EXT-06's matcher (pin table,
//! static slot order, smallest-net candidates, hashed dedupe): 242 s release (loaded machine).
//! The matcher is no longer the limit; the catalog is. Per-pattern release times
//! with both patterns below skipped via `do_not_use`: 4.26 s total.
//! - `diff_pair_with_degen` joins slots 0 and 1 by `Diff` links only, and
//!   `diff_pair_with_split_cascodes` joins them by a plain `Same` source (a rail is
//!   fine). Their match *counts* grow with N² (at 1,250 devices: 249,000 and
//!   124,500), so no enumerator can meet the bound.
//! - `diff_pair_with_reference` (1.85 s, 0 matches): slot 1 meets slot 0 only on
//!   `S`, and slot 0's `S` is VDD; `source_follower_with_mirror` (1.39 s): slot 1
//!   has no link to slot 0 and slot 2 waits on its `SameTypeAs(1)` reference.
//! Fix: catalog links (`eq_sig` sources, a link joining the pair), not the matcher.
//! Run with `cargo test --release -p annotator --test scale -- --ignored`.

mod common;

use std::time::{Duration, Instant};

use annotator::{annotate, AnnotationConfig};
use common::net;

/// `ota5t` of `tests/corpus.rs`, `{k}` marking what each copy suffixes.
const OTA5T: &str = "XM1_{k} vout1_{k} vinp_{k} vtail_{k} VSS nfet w=10u l=1u
    XM2_{k} vout2_{k} vinm_{k} vtail_{k} VSS nfet w=10u l=1u
    XM3_{k} vout1_{k} vbias_{k} VDD VDD pfet w=20u l=1u
    XM4_{k} vout2_{k} vbias_{k} VDD VDD pfet w=20u l=1u
    XM5_{k} vtail_{k} vbn_{k} VSS VSS nfet w=40u l=2u";

/// 2,500 disjoint copies sharing only VDD/VSS: 12,500 devices, 17,502 nets
/// (below the `u16` id limit).
#[test]
#[ignore = "T8: 242 s after EXT-06 (module doc); needs catalog link fixes"]
fn twelve_thousand_devices() {
    let src: String = (0..2500).map(|k| OTA5T.replace("{k}", &k.to_string()) + "\n").collect();
    let nl = net(&src);
    assert_eq!((nl.devices.len(), nl.nets.len()), (12_500, 17_502));
    let t = Instant::now();
    let p = annotate(&nl, &AnnotationConfig::default());
    let wall = t.elapsed();
    println!("annotate: {} devices in {wall:?}", nl.devices.len());
    assert_eq!(p.groups.iter().map(Vec::len).sum::<usize>(), 12_500, "every device in one group");
    assert!(wall <= Duration::from_secs(2), "{wall:?} > 2.0 s");
}
