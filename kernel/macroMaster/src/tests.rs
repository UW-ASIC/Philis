use super::*;
use pnr_core::DeviceKind;

fn pdk() -> verify::Pdk {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json");
    verify::Pdk::from_json(&std::fs::read_to_string(path).expect("sky130 deck")).expect("deck parses")
}

#[derive(Default)]
struct PairIo {
    a: InOut<Signal>,
    b: InOut<Signal>,
    tail: InOut<Signal>,
    body: InOut<Signal>,
}
impl Io for PairIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![self.a.port("a"), self.b.port("b"), self.tail.port("tail"), self.body.port("body")]
    }
}

/// Two Mos side by side, wired to the four ports.
struct GoodPair;
impl Block for GoodPair {
    type Io = PairIo;
    fn name(&self) -> String {
        "goodpair".into()
    }
}
impl Composition for GoodPair {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        let mos = || variants::Mos::new(DeviceKind::Nmos, 420, 150, 2);
        let li = c.instantiate("m1", &mos())?;
        let left = c.place(li)?;
        let ri = c.instantiate("m2", &mos())?;
        let right = c.place_by(ri, AlignMode::ToTheRight, &left, 80)?;
        wire(c, &left, &right);
        Ok(())
    }
}

fn wire<P: Process>(c: &mut CompBuilder<P>, left: &Instance, right: &Instance) {
    c.connect(&left.term("g"), "a");
    c.connect(&right.term("g"), "b");
    c.connect(&left.term("s"), "tail");
    c.connect(&right.term("s"), "tail");
    c.connect(&left.term("b"), "body");
    c.connect(&right.term("b"), "body");
}

#[test]
fn placing_overlapping_devices_is_rejected() {
    struct Overlap;
    impl Block for Overlap {
        type Io = PairIo;
        fn name(&self) -> String {
            "overlap".into()
        }
    }
    impl Composition for Overlap {
        fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
            let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 2))?;
            let b = c.instantiate("m2", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 2))?;
            c.place(a)?;
            c.place(b)?; // also at the origin
            Ok(())
        }
    }
    assert_eq!(build_composition(&Overlap, &pdk()).err(), Some(GenError::Overlap));
}

#[test]
fn overlaps_treats_flush_abut_as_legal() {
    let a = Rect { x: 0, y: 0, w: 100, h: 50 };
    assert!(!overlaps(a, Rect { x: 100, y: 0, w: 100, h: 50 }));
    assert!(overlaps(a, Rect { x: 90, y: 0, w: 100, h: 50 }));
}

#[test]
fn nets_resolve_to_port_named_classes() {
    let edges = vec![
        ("m1.s".to_string(), "tail".to_string()),
        ("m2.s".to_string(), "m1.s".to_string()),
        ("m1.d".to_string(), "m2.g".to_string()),
    ];
    let (names, binding) = resolve_nets(&edges, &["tail".to_string()]);
    assert_eq!(names.len(), 2);
    assert_eq!(names[binding["m2.s"]], "tail");
    assert!(names[binding["m1.d"]].starts_with("net"));
}

/// Flat and per-instance views agree on NetIds, every Device pin survives
/// qualified, unwired pins stay separate nets, and the netlist is declared.
#[test]
fn build_composition_binds_every_pin() {
    let built = build_composition(&GoodPair, &pdk()).expect("builds");
    let net = |pin: &str| {
        let p = built.flat.pins.iter().find(|p| p.name == pin).unwrap_or_else(|| panic!("missing {pin}"));
        built.nets[p.net.0 as usize].clone()
    };
    for (_, mac) in &built.instances {
        for p in &mac.pins {
            assert_eq!(built.nets[p.net.0 as usize], net(&p.name), "{}", p.name);
        }
    }
    assert_eq!(net("m1.g"), "a");
    assert_eq!(net("m2.g"), "b");
    assert_eq!(net("m1.s"), "tail");
    assert_eq!(net("m2.b"), "body");
    assert_ne!(net("m1.d"), net("m2.d"), "unwired drains are separate nets");
    let (m1, m2) = (&built.instances[0].1, &built.instances[1].1);
    assert!(m2.bbox.x >= m1.bbox.x + m1.bbox.w, "m2 sits right of m1");
    let nl = built.netlist.as_ref().expect("all devices declared");
    assert_eq!(nl.devices.len(), 2);
    assert!(nl.nets.iter().zip(&built.nets).all(|(a, b)| &a.name == b));
}

#[test]
fn place_mirrored_is_exactly_symmetric() {
    struct MirrorPair;
    impl Block for MirrorPair {
        type Io = PairIo;
        fn name(&self) -> String {
            "mirrorpair".into()
        }
    }
    impl Composition for MirrorPair {
        fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
            let mos = || variants::Mos::new(DeviceKind::Nmos, 420, 150, 2);
            let li = c.instantiate("m1", &mos())?;
            let left = c.place(li)?;
            let ri = c.instantiate("m2", &mos())?;
            let right = c.place_mirrored(ri, &left, 100)?;
            wire(c, &left, &right);
            Ok(())
        }
    }
    let built = build_composition(&MirrorPair, &pdk()).expect("builds");
    let (m1, m2) = (&built.instances[0].1, &built.instances[1].1);
    // Twice the axis: left edge of m1 + right edge of m2.
    let two_axis = m1.bbox.x + m2.bbox.x + m2.bbox.w;
    assert_eq!((m2.bbox.w, m2.bbox.h), (m1.bbox.w, m1.bbox.h));
    for p1 in &m1.pins {
        let local = p1.name.split_once('.').unwrap().1;
        assert!(
            m2.pins.iter().any(|p| p.name == format!("m2.{local}")
                && p.at.y == p1.at.y
                && p.at.x == two_axis - (p1.at.x + p1.at.w)),
            "no mirrored partner for {}",
            p1.name
        );
    }
}

/// Hierarchy: child port pins surface as `x1.<port>`, child internals stay
/// private, and the netlist counts transistors through the hierarchy.
#[test]
fn hierarchical_composition_keeps_child_nets_whole() {
    struct TwoPairs;
    impl Block for TwoPairs {
        type Io = PairIo;
        fn name(&self) -> String {
            "twopairs".into()
        }
    }
    impl Composition for TwoPairs {
        fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
            let x1 = c.instantiate_comp("x1", &GoodPair)?;
            let x1 = c.place(x1)?;
            let x2 = c.instantiate_comp("x2", &GoodPair)?;
            let x2 = c.place_by(x2, AlignMode::Above, &x1, 200)?;
            for port in ["a", "b", "tail", "body"] {
                c.connect(&x1.term(port), port);
                c.connect(&x2.term(port), port);
            }
            Ok(())
        }
    }
    let built = build_composition(&TwoPairs, &pdk()).expect("builds");
    let net = |pin: &str| {
        let p = built.flat.pins.iter().find(|p| p.name == pin).unwrap_or_else(|| panic!("missing {pin}"));
        built.nets[p.net.0 as usize].clone()
    };
    assert_eq!(net("x1.tail"), "tail");
    assert_eq!(net("x2.tail"), "tail");
    assert_eq!(net("x2.b"), "b");
    assert_ne!(net("x1.m1.d"), net("x2.m1.d"), "internals do not merge across instances");
    let names: Vec<&str> =
        built.netlist.as_ref().expect("netlist").devices.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["x1.m1", "x1.m2", "x2.m1", "x2.m2"]);
}

#[test]
fn matched_pair_exposes_per_leg_pins() {
    struct One(variants::MatchedPair);
    impl Block for One {
        type Io = variants::MatchedPairIo;
        fn name(&self) -> String {
            "one".into()
        }
    }
    impl Composition for One {
        fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
            let i = c.instantiate("p", &self.0)?;
            c.place(i)?;
            Ok(())
        }
    }
    let pair = variants::MatchedPair { kind: DeviceKind::Nmos, w: 420, l: 150, nf_each: 2, pattern: None };
    let built = build_composition(&One(pair), &pdk()).expect("builds");
    for leg in ["g1", "d1", "s1", "g2", "d2", "s2", "b"] {
        assert!(built.flat.pins.iter().any(|p| p.name == format!("p.{leg}")), "missing {leg}");
    }
}

#[test]
fn mos_dummies_parameter_selects_variant() {
    let g = pdk();
    let draw = |dummies| {
        let mut m = variants::Mos::new(DeviceKind::Nmos, 420, 150, 4);
        m.dummies_per_edge = dummies;
        let mut b = cells::Builder::new(g.grid());
        m.layout(&mut DeviceBuilder { builder: &mut b, process: &g }).expect("draws");
        b.finish()
    };
    assert_ne!(draw(Some(0)).bbox, draw(Some(1)).bbox);
}

/// `Mos::w` is one finger's width; the schematic device carries the SPICE
/// total `w·nf` (`pnr_core::MosSize`), so LVS compares `W_total/nf` with the
/// drawn finger, not half of it.
#[test]
fn mos_device_card_carries_total_width() {
    let devs = variants::Mos::new(DeviceKind::Nmos, 420, 150, 2).devices().expect("Mos is transparent");
    assert_eq!(devs[0].params, vec![("w".into(), 840), ("l".into(), 150), ("nf".into(), 2)]);
}

/// A one-off build over the pair's ports.
fn build_pair(f: impl FnOnce(&mut CompBuilder<verify::Pdk>) -> Result<(), GenError>) -> Result<BuiltComp, GenError> {
    let ports = ["a", "b", "tail", "body"].map(String::from).to_vec();
    build_with(&pdk(), ports, f)
}

#[test]
fn duplicate_names_are_rejected() {
    let r = build_pair(|c| {
        let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 2))?;
        let a = c.place(a)?;
        let b = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 2))?;
        c.place_by(b, AlignMode::ToTheRight, &a, 1000)?;
        Ok(())
    });
    assert_eq!(r.err(), Some(GenError::DuplicateName("m1".into())));
}

#[test]
fn an_unknown_terminal_is_rejected() {
    let r = build_pair(|c| {
        let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 2))?;
        let a = c.place(a)?;
        c.connect(&a.term("d"), "a");
        c.connect("m9.d", "vout");
        Ok(())
    });
    assert_eq!(r.err(), Some(GenError::UnknownTerminal("m9.d".into())));
}

/// One finger's current reverses under a mirror, so the pair would not match.
#[test]
fn mirroring_an_odd_finger_device_is_refused() {
    let r = build_pair(|c| {
        let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 1))?;
        assert!(!a.mac.units.is_empty(), "the Mos instance carries its units");
        let a = c.place(a)?;
        let b = c.instantiate("m2", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 1))?;
        c.place_mirrored(b, &a, 200)?;
        Ok(())
    });
    assert!(matches!(r.err(), Some(GenError::Orientation(n)) if n == "m2"));
}

#[test]
fn copy_keeps_orientation() {
    let gap = 200;
    build_pair(|c| {
        let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 1))?;
        let a = c.place(a)?;
        let b = c.instantiate("m2", &variants::Mos::new(DeviceKind::Nmos, 420, 150, 1))?;
        let b = c.place_copy(b, &a, gap)?;
        assert_eq!(b.phi_sum(), a.phi_sum());
        assert_eq!(b.bbox().x, a.bbox().x + a.bbox().w + gap);
        assert_eq!(b.bbox().y, a.bbox().y);
        Ok(())
    })
    .expect("a copy places");
}

// ── Pure helpers ──

#[test]
fn align_delta_matches_every_mode() {
    let a = Rect { x: 0, y: 0, w: 10, h: 20 };
    let b = Rect { x: 100, y: 200, w: 40, h: 60 };
    let cases = [
        (AlignMode::Left, (100, 5)),
        (AlignMode::Right, (130, 5)),
        (AlignMode::Bottom, (5, 200)),
        (AlignMode::Top, (5, 240)),
        (AlignMode::CenterHorizontal, (5, 220)),
        (AlignMode::CenterVertical, (115, 5)),
        (AlignMode::ToTheRight, (145, 0)),
        (AlignMode::ToTheLeft, (85, 0)),
        (AlignMode::Above, (0, 265)),
        (AlignMode::Beneath, (0, 175)),
    ];
    for (mode, want) in cases {
        assert_eq!(align_delta(a, b, mode, 5), want, "{mode:?}");
    }
    // Abut modes at offset 0 touch without overlapping.
    for mode in [AlignMode::ToTheRight, AlignMode::ToTheLeft, AlignMode::Above, AlignMode::Beneath] {
        let (dx, dy) = align_delta(a, b, mode, 0);
        let moved = Rect { x: a.x + dx, y: a.y + dy, ..a };
        assert!(!overlaps(moved, b), "{mode:?}");
    }
}

#[test]
fn overlaps_is_symmetric_and_needs_positive_area() {
    let a = Rect { x: 0, y: 0, w: 100, h: 100 };
    for (b, want) in [
        (Rect { x: 100, y: 100, w: 10, h: 10 }, false), // corner touch
        (Rect { x: 0, y: 100, w: 100, h: 10 }, false),  // edge touch above
        (Rect { x: 10, y: 10, w: 10, h: 10 }, true),    // contained
        (Rect { x: -50, y: -50, w: 300, h: 300 }, true), // containing
        (Rect { x: 99, y: 99, w: 10, h: 10 }, true),    // one-unit overlap
        (Rect { x: 500, y: 0, w: 10, h: 10 }, false),
    ] {
        assert_eq!(overlaps(a, b), want, "{b:?}");
        assert_eq!(overlaps(b, a), want, "symmetric {b:?}");
    }
}

#[test]
fn check_grid_tests_every_corner_and_disables_on_nonpositive_grid() {
    assert_eq!(check_grid(Rect { x: -10, y: 5, w: 15, h: 5 }, 5), Ok(()));
    assert_eq!(check_grid(Rect { x: 0, y: 0, w: 7, h: 5 }, 5), Err(GenError::OffGrid));
    assert_eq!(check_grid(Rect { x: 0, y: -3, w: 5, h: 5 }, 5), Err(GenError::OffGrid));
    for g in [0, -5] {
        assert_eq!(check_grid(Rect { x: 1, y: 2, w: 3, h: 4 }, g), Ok(()));
    }
}

#[test]
fn resolve_nets_corner_cases() {
    let (names, binding) = resolve_nets(&[], &["a".to_string()]);
    assert!(names.is_empty() && binding.is_empty(), "no edges, no nets; an unwired port has none");

    // Two ports in one class: the first in `ports` order names it.
    let e = |a: &str, b: &str| (a.to_string(), b.to_string());
    let edges = [e("a", "m1.d"), e("m1.d", "b")];
    let (names, binding) = resolve_nets(&edges, &["b".to_string(), "a".to_string()]);
    assert_eq!(names, ["b"]);
    assert!(["a", "b", "m1.d"].iter().all(|t| binding[*t] == 0));

    // A self-edge is one singleton class.
    let (names, binding) = resolve_nets(&[e("x", "x")], &[]);
    assert_eq!(names, ["net0"]);
    assert_eq!(binding["x"], 0);

    // Long chains merge transitively, in any edge order.
    let chain: Vec<_> = (0..50).rev().map(|i| e(&format!("t{i}"), &format!("t{}", i + 1))).collect();
    let (names, binding) = resolve_nets(&chain, &["t25".to_string()]);
    assert_eq!(names, ["t25"]);
    assert_eq!(binding.len(), 51);
}

#[test]
fn macros_register_replaces_and_get_misses_cleanly() {
    let mut m = Macros::default();
    assert!(m.get("x").is_none());
    let a = Macro { bbox: Rect { x: 0, y: 0, w: 1, h: 1 }, ..Default::default() };
    let b = Macro { bbox: Rect { x: 0, y: 0, w: 2, h: 2 }, ..Default::default() };
    m.register("x", a);
    m.register("y", b.clone());
    m.register("x", b);
    assert_eq!(m.get("x").map(|m| m.bbox.w), Some(2));
    assert_eq!(m.get("y").map(|m| m.bbox.w), Some(2));
    assert!(m.get("z").is_none());
}

// ── Devices ──

fn mos(nf: u16) -> variants::Mos {
    variants::Mos::new(DeviceKind::Nmos, 420, 150, nf)
}

#[test]
fn mos_dummy_cards_tie_to_the_end_regions() {
    let even = variants::Mos { dummies_per_edge: Some(1), ..mos(2) }.devices().unwrap();
    assert_eq!(even.len(), 3);
    let drain = |d: &GenDevice| d.terminals.iter().find(|(t, _)| t == "D").unwrap().1.clone();
    assert_eq!((drain(&even[1]), drain(&even[2])), ("s".to_string(), "s".to_string()), "even nf: source both ends");
    let odd = variants::Mos { dummies_per_edge: Some(2), ..mos(3) }.devices().unwrap();
    assert_eq!((drain(&odd[1]), drain(&odd[2])), ("s".to_string(), "d".to_string()), "odd nf: drain on the right");
    for d in &even[1..] {
        assert_eq!(d.params, vec![("w".into(), 420), ("l".into(), 150), ("nf".into(), 1)]);
        assert!(d.terminals.iter().filter(|(t, _)| t != "D").all(|(_, p)| p == "b"));
    }
    assert_eq!(variants::Mos { dummies_per_edge: Some(0), ..mos(2) }.devices().unwrap().len(), 1);
}

#[test]
fn zero_fingers_read_as_one() {
    let d = mos(0).devices().unwrap();
    assert_eq!(d[0].params, vec![("w".into(), 420), ("l".into(), 150), ("nf".into(), 1)]);
    let pair = variants::MatchedPair { kind: DeviceKind::Pmos, w: 500, l: 150, nf_each: 0, pattern: None };
    assert!(pair.devices().unwrap().iter().all(|d| d.params[2] == ("nf".into(), 1)));
}

#[test]
fn matched_pair_and_res_cards() {
    let pair = variants::MatchedPair { kind: DeviceKind::Nmos, w: 500, l: 150, nf_each: 3, pattern: None };
    let ds = pair.devices().unwrap();
    assert_eq!(ds.len(), 2);
    assert_eq!(ds[1].terminals, vec![
        ("D".into(), "d2".into()),
        ("G".into(), "g2".into()),
        ("S".into(), "s2".into()),
        ("B".into(), "b".into()),
    ]);
    assert_eq!(ds[0].params[0], ("w".into(), 1500), "SPICE total width");
    let r = variants::Res { w: 1000, len: 5000 }.devices().unwrap();
    assert_eq!(r[0].terminals, vec![("P".into(), "a".into()), ("N".into(), "b".into())]);
    assert_eq!(r[0].params, vec![("w".into(), 1000), ("l".into(), 5000)]);
    assert_eq!(variants::Res { w: 1000, len: 5000 }.name(), "res_w1000_l5000");
    assert_eq!(mos(2).name(), "mos_Nmos_w420_l150_nf2");
}

#[test]
fn adapter_draw_treats_missing_and_zero_counts_as_one() {
    let g = pdk();
    let draw = |nf: &[u16], pick: bool| {
        adapter::draw::<cells::mosfet::Mosfet>(DeviceKind::Nmos, 420, 150, nf, false, &g, move |_| pick)
    };
    let one = draw(&[1], true);
    assert!(!one.shapes.is_empty());
    assert_eq!(draw(&[0], true).bbox, one.bbox);
    assert_eq!(draw(&[], true).bbox, one.bbox, "an empty count list draws one device");
    assert_eq!(draw(&[1], false).bbox, one.bbox, "no accepted variant falls back to the smallest of all");
}

/// A device with geometry but no schematic card.
struct Blob {
    off_grid: bool,
}
impl Block for Blob {
    type Io = variants::ResIo;
    fn name(&self) -> String {
        "blob".into()
    }
}
impl DeviceGen for Blob {
    fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
        let g = cell.process().grid().max(1);
        let nudge = i32::from(self.off_grid);
        cell.draw(LayerId(0), Rect { x: 0, y: 0, w: 10 * g + nudge, h: 10 * g })?;
        cell.pin("a", LayerId(0), Rect { x: 0, y: 0, w: g, h: g })
    }
}

#[test]
fn an_opaque_device_drops_the_netlist_but_keeps_nets() {
    let built = build_pair(|c| {
        let b = c.instantiate("x", &Blob { off_grid: false })?;
        c.place(b)?;
        c.connect("x.a", "a");
        Ok(())
    })
    .expect("builds");
    assert!(built.netlist.is_none());
    let p = built.flat.pins.iter().find(|p| p.name == "x.a").expect("qualified pin");
    assert_eq!(built.nets[p.net.0 as usize], "a");
}

#[test]
fn off_grid_drawing_is_rejected() {
    if pdk().grid() <= 1 {
        return; // every integer is on a 1 nm grid
    }
    let r = build_pair(|c| {
        c.instantiate("x", &Blob { off_grid: true })?;
        Ok(())
    });
    assert_eq!(r.err(), Some(GenError::OffGrid));
}

#[test]
fn an_empty_build_has_no_nets_and_an_empty_netlist() {
    let built = build_pair(|_| Ok(())).expect("builds");
    assert!(built.instances.is_empty() && built.nets.is_empty() && built.edges.is_empty());
    assert!(built.flat.shapes.is_empty());
    assert_eq!(built.ports, ["a", "b", "tail", "body"]);
    assert!(built.netlist.expect("no opaque instance").devices.is_empty());
}

#[test]
fn multi_device_generators_suffix_ordinals_and_share_nets() {
    let built = build_pair(|c| {
        let m = c.instantiate("m1", &variants::Mos { dummies_per_edge: Some(1), ..mos(2) })?;
        c.place(m)?;
        Ok(())
    })
    .expect("builds");
    let nl = built.netlist.expect("netlist");
    let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["m1.0", "m1.1", "m1.2"]);
    let net = |d: usize, t: &str| nl.devices[d].terminals.iter().find(|(n, _)| n == t).unwrap().1;
    assert_eq!(net(1, "D"), net(0, "S"), "left dummy ties to the source");
    assert_eq!(net(1, "G"), net(0, "B"));
}

// ── Placement ──

#[test]
fn a_rejected_place_commits_nothing() {
    let built = build_pair(|c| {
        let a = c.instantiate("m1", &mos(2))?;
        let a = c.place(a)?;
        assert_eq!((a.name(), a.term("d").as_str()), ("m1", "m1.d"));
        let b = c.instantiate("m2", &mos(2))?;
        assert_eq!(c.place(b.clone()).err(), Some(GenError::Overlap));
        c.place_by(b, AlignMode::ToTheRight, &a, 0)?; // flush abut is legal
        Ok(())
    })
    .expect("builds");
    assert_eq!(built.instances.len(), 2);
}

/// The mirror axis is `gap / 2` right of the reference, snapped up (toward
/// +∞): when that is already on the grid the partner sits exactly `gap` away,
/// also left of the origin.
#[test]
fn mirror_axis_snaps_up_for_negative_coordinates() {
    let g = pdk().grid().max(1);
    build_pair(|c| {
        let a = c.instantiate("m1", &mos(2))?;
        let a = c.place(a)?;
        let left = c.instantiate("m2", &mos(2))?;
        let left = c.place_by(left, AlignMode::ToTheLeft, &a, 20_000)?;
        let right_edge = left.bbox().x + left.bbox().w;
        assert!(right_edge < 0);
        let m = c.instantiate("m3", &mos(2))?;
        let m = c.place_mirrored(m, &left, 2 * g)?;
        assert_eq!(m.bbox().x - right_edge, 2 * g);
        assert_eq!(m.bbox().y, left.bbox().y);
        Ok(())
    })
    .expect("builds");
}

#[test]
fn orient_keeps_the_lower_left_corner_and_turns_units() {
    build_pair(|c| {
        let before = c.instantiate("m1", &mos(2))?;
        let b = before.bbox();
        let mut m = before.clone();
        m.orient(Orient::R180);
        assert_eq!(m.bbox(), b);
        for (u, v) in m.mac.units.iter().zip(&before.mac.units) {
            assert_eq!(u.phi, (-v.phi.0, -v.phi.1));
        }
        for (p, q) in m.mac.pins.iter().zip(&before.mac.pins) {
            assert_eq!(p.at.x, 2 * b.x + b.w - (q.at.x + q.at.w), "{}", p.name);
            assert_eq!(p.at.y, 2 * b.y + b.h - (q.at.y + q.at.h), "{}", p.name);
        }
        m.orient(Orient::R90);
        assert_eq!(m.bbox(), Rect { x: b.x, y: b.y, w: b.h, h: b.w });
        Ok(())
    })
    .expect("builds");
}

#[test]
fn resistor_keepouts_survive_and_move_with_the_instance() {
    build_pair(|c| {
        let r = variants::Res { w: 1000, len: 5000 };
        let a = c.instantiate("r1", &r)?;
        assert!(!a.mac.keepouts.is_empty(), "the body keepout is replayed");
        let names: Vec<&str> = a.mac.pins.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"a") && names.contains(&"b"), "{names:?}");
        let a = c.place(a)?;
        let b = c.instantiate("r2", &r)?;
        let b = c.place_by(b, AlignMode::Above, &a, 1000)?;
        let dy = b.bbox().y - a.bbox().y;
        assert_eq!(a.mac.keepouts.len(), b.mac.keepouts.len());
        for (ka, kb) in a.mac.keepouts.iter().zip(&b.mac.keepouts) {
            assert_eq!((kb.rect.x, kb.rect.y - ka.rect.y, kb.why), (ka.rect.x, dy, ka.why));
        }
        Ok(())
    })
    .expect("builds");
}
