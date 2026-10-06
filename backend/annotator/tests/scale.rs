//! T8 (Philis policy): `annotate` on 12,500 devices within 2.0 s, release.
//! M0: no return after 13.5 min. EXT-06: 0.21-0.25 s on a loaded machine, once the
//! catalog stopped pairing devices across copies (`diff_pair_with_degen` deleted,
//! `eq_sig` sources: N² genuine matches before) and candidates came from a
//! (net, pin, polarity) index, so a rail's devices are never scanned for a drain.
//! EXT-14..19 review fixes 1: set origins, roles and `set_pairs` look leaves, passive
//! and shared groups up through a per-device index rather than scanning every set and
//! leaf (debug 5.0 s → 1.0 s; release 437 ms before the fix).

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
fn twelve_thousand_devices() {
    let src: String = (0..2500).map(|k| OTA5T.replace("{k}", &k.to_string()) + "\n").collect();
    let nl = net(&src);
    assert_eq!((nl.devices.len(), nl.nets.len()), (12_500, 17_502));
    let t = Instant::now();
    let p = annotate(&nl, &AnnotationConfig::default());
    let wall = t.elapsed();
    println!("annotate: {} devices in {wall:?}", nl.devices.len());
    assert_eq!(p.blocks.iter().map(|b| b.devices.len()).sum::<usize>(), 12_500, "every device in one group");
    assert!(wall <= Duration::from_secs(2), "{wall:?} > 2.0 s");
}
