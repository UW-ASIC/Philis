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
