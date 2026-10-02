//! T8 (Philis policy): `annotate` on 12,500 devices within 2.0 s, release.
//! Fails today: measured at M0 (release), `annotate` had not returned after
//! 13.5 min wall, ~8 min CPU, >400x the bound. EXT-06 owns the fix.
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
#[ignore = "T8: passes after EXT-06; today annotate does not finish in >13 min at 12,500 devices"]
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
