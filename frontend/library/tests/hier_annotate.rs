//! EXT-27 acceptance (T2): a parsed two-instance fully differential netlist.
//! Each instance keeps its own input pair, the two instances pair up as one
//! template, and their corresponding devices are matched across instances.

use analog::intent::GroupKind;
use annotator::{annotate, hier, size, AnnotationConfig, BlockKind};

const FD: &str = "\
.subckt fd_ota inp inn outp outn vdd vss vb vbp
MN1 outn inp tail vss nfet w=4u l=0.5u
MN2 outp inn tail vss nfet w=4u l=0.5u
MP3 outn vbp vdd vdd pfet w=6u l=0.5u
MP4 outp vbp vdd vdd pfet w=6u l=0.5u
MN5 tail vb vss vss nfet w=8u l=0.5u
.ends
X1 a1 b1 c1 d1 vdd vss vb vbp fd_ota
X2 a2 b2 c2 d2 vdd vss vb vbp fd_ota
.end
";

#[test]
fn two_instance_fd_ota() {
    let nl = library::spice_with(FD, &library::ParseOptions::default()).unwrap();
    let id = |n: &str| pnr_core::ids::DeviceId(nl.devices.iter().position(|d| d.name == n).unwrap_or_else(|| panic!("{n}")) as u16);
    let p = annotate(&nl, &AnnotationConfig::default());
    for x in ["X1", "X2"] {
        let (a, b) = (id(&format!("{x}/MN1")), id(&format!("{x}/MN2")));
        let leaf = annotator::block::leaves(&p.blocks).into_iter().any(|bl| bl.kind == BlockKind::DiffPair && bl.devices.contains(&a) && bl.devices.contains(&b));
        assert!(leaf, "{x}: no DiffPair leaf: {:?}", annotator::block::leaves(&p.blocks).iter().map(|b| (b.kind, b.template, b.devices.iter().map(|d| nl.devices[d.0 as usize].name.as_str()).collect::<Vec<_>>())).collect::<Vec<_>>());
    }
    let mut models = Vec::new();
    let drawn: Vec<_> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
    assert_eq!(hier::same_template(&nl, &drawn), [(0, 1)]);
    // The input devices X1/MN1 and X2/MN1 are matched across instances (EXT-27 MatchBlock):
    // one Matching node's subtree holds both. Shared bias alone (the tails on `vb`) does not.
    let tree = &p.intent.tree;
    fn under(tree: &[analog::intent::GroupNode], g: usize, out: &mut Vec<pnr_core::ids::DeviceId>) {
        out.extend(&tree[g].devices);
        tree[g].children.iter().for_each(|&c| under(tree, c as usize, out));
    }
    let (a, b) = (id("X1/MN1"), id("X2/MN1"));
    let matched = (0..tree.len()).filter(|&g| tree[g].kind == GroupKind::Matching).any(|g| {
        let mut v = Vec::new();
        under(tree, g, &mut v);
        v.contains(&a) && v.contains(&b)
    });
    assert!(matched, "{tree:?}");
}
