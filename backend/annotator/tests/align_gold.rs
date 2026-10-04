//! ALIGN `high_speed_comparator` gold (plan-01 T1), transcribed from
//! `benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.const.json`
//! (a vendored tree that may not be checked out). Only its port declarations
//! are input; its symmetry entries are what the annotator must recover.

mod common;

use annotator::annotate;
use common::{canon_intent, net, sorted, strongarm_cfg, STRONGARM};

/// The device pairs its `SymmetricBlocks` implies (`xdp`, `xccn`, `xccp` and
/// `xinv_n`/`xinv_p` expanded to their instances).
const GOLD_PAIRS: [(&str, &str); 7] = [
    ("mn1", "mn2"),
    ("mn3", "mn4"),
    ("mp5", "mp6"),
    ("mp7", "mp8"),
    ("mp9", "mp10"),
    ("mp11", "mp12"),
    ("mn13", "mn14"),
];
const GOLD_SELF: [&str; 1] = ["mn0"];
const GOLD_NET_PAIRS: [(&str, &str); 3] = [("vin", "vip"), ("vin_d", "vip_d"), ("vin_o", "vip_o")];

/// T1: every gold pair and no other, `mn0` and the 3 net pairs, one axis.
#[test]
fn strongarm_matches_align_gold() {
    let nl = net(STRONGARM);
    let c = canon_intent(&annotate(&nl, &strongarm_cfg()), &nl);
    assert_eq!(c.pairs, GOLD_PAIRS.iter().map(|&(a, b)| sorted(a, b)).collect(), "device pairs");
    assert!(GOLD_SELF.iter().all(|s| c.selfs.contains(*s)), "selfs {:?}", c.selfs);
    assert!(GOLD_NET_PAIRS.iter().all(|&(a, b)| c.net_pairs.contains(&sorted(a, b))), "net pairs {:?}", c.net_pairs);
    assert_eq!(c.axes, 1);
}
