//! ALIGN `high_speed_comparator` gold (plan-01 T1), transcribed from
//! `benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.const.json`
//! (a vendored tree that may not be checked out). Only its port declarations
//! are input; its symmetry entries are what the annotator must recover.

mod common;

use annotator::annotate;
use common::{canon, canon_intent, net, sorted, strongarm_cfg, STRONGARM};

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
    let p = annotate(&nl, &strongarm_cfg());
    let c = canon(&p, &nl);
    assert_eq!(c.pairs, GOLD_PAIRS.iter().map(|&(a, b)| sorted(a, b)).collect(), "device pairs");
    assert!(GOLD_SELF.iter().all(|s| c.selfs.contains(*s)), "selfs {:?}", c.selfs);
    // The compound's mirrored nets (routing's Differential since EXT-24) carry
    // the gold's (vin, vip).
    let np = canon_intent(&p, &nl).net_pairs;
    assert!(GOLD_NET_PAIRS.iter().all(|&(a, b)| np.contains(&sorted(a, b))), "net pairs {np:?}");
    assert_eq!(c.axes, 1);
}

/// The gold `.const.json` verbatim (minified), so the test never reads the vendored tree.
const GOLD_JSON: &str = r#"[{"constraint":"PowerPorts","ports":["VCC"]},{"constraint":"GroundPorts","ports":["VSS"]},{"constraint":"ClockPorts","ports":["clk"]},{"constraint":"HorizontalDistance","abs_distance":0},{"constraint":"VerticalDistance","abs_distance":0},{"constraint":"GroupBlocks","instances":["mn1","mn2"],"instance_name":"xdp"},{"constraint":"GroupBlocks","instances":["mn3","mn4"],"instance_name":"xccn"},{"constraint":"GroupBlocks","instances":["mp5","mp6"],"instance_name":"xccp"},{"constraint":"GroupBlocks","instances":["mp11","mn13"],"instance_name":"xinv_n"},{"constraint":"GroupBlocks","instances":["mp12","mn14"],"instance_name":"xinv_p"},{"constraint":"SymmetricBlocks","direction":"V","pairs":[["mn0"],["xdp"],["xccn"],["xccp"],["mp7","mp8"],["mp9","mp10"],["xinv_n","xinv_p"]]},{"constraint":"Order","direction":"top_to_bottom","instances":["mn0","xdp","xccn","xccp"]},{"constraint":"Align","line":"h_bottom","instances":["mp9","mp7","xdp","mp8","mp10"]},{"constraint":"Align","line":"h_bottom","instances":["xinv_n","xccp","xinv_p"]},{"constraint":"SymmetricNets","direction":"V","net1":"vin","pins1":["mn1/G"],"net2":"vip","pins2":["mn2/G"]},{"constraint":"SymmetricNets","direction":"V","net1":"vin_d","pins1":["mn1/D","mn3/S","mp7/D"],"net2":"vip_d","pins2":["mn2/D","mn4/S","mp8/D"]},{"constraint":"SymmetricNets","direction":"V","net1":"vin_o","pins1":["mn3/D","mn4/G","mp5/D","mp6/G","mp9/D","mp12/G","mn14/G"],"net2":"vip_o","pins2":["mn3/G","mn4/D","mp5/G","mp6/D","mp10/D","mp11/G","mn13/G"]}]"#;

/// EXT-26: the gold's port entries name the rails and the clock.
#[test]
fn align_power_ground_clock() {
    use analog::metadata::NetClass;
    let nl = net(STRONGARM);
    let first3 = serde_json::to_string(&serde_json::from_str::<Vec<serde_json::Value>>(GOLD_JSON).unwrap()[..3]).unwrap();
    let (c, d) = annotator::AnnotationConfig::from_json(&first3, &nl).unwrap();
    assert!(d.is_empty(), "{d:?}");
    let p = annotate(&nl, &c);
    let class = |n: &str| p.net_classes[nl.nets.iter().position(|x| x.name == n).unwrap()].class;
    assert_eq!((class("vcc"), class("vss"), class("clk")), (NetClass::Supply, NetClass::Ground, NetClass::Clock));
}

/// EXT-26 T1 through the sidecar: the full gold alone (no `strongarm_cfg`
/// names) gives the gold pairs exactly, `mn0` on the one axis; its layout-only
/// entries are diagnosed, never dropped silently.
#[test]
fn symmetric_blocks_become_seeds() {
    let nl = net(STRONGARM);
    let (mut c, d) = annotator::AnnotationConfig::from_json(GOLD_JSON, &nl).unwrap();
    c.process = common::cfg().process;
    let count = |k: &str| d.iter().filter(|x| x.kind == k).count();
    assert_eq!((count("sidecar_unsupported"), count("sidecar_unconsumed"), d.len()), (4, 1, 5), "{d:?}");
    c.sidecar_diags = d;
    let p = annotate(&nl, &c);
    let got = canon(&p, &nl);
    assert_eq!(got.pairs, GOLD_PAIRS.iter().map(|&(a, b)| sorted(a, b)).collect());
    assert!(got.selfs.contains("mn0"), "{:?}", got.selfs);
    assert_eq!(got.axes, 1);
    assert_eq!(p.intent.diagnostics.iter().filter(|x| x.kind.starts_with("sidecar_")).count(), 5);
}
