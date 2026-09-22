//! M2 acceptance: a hand-written Composition elaborates against a real deck —
//! build, net-bind, route — with no SPICE and no placer. The PDK is a
//! parameter, which is the substrate3 "swap on the fly" contract.

use library::{elaborate, ElabConfig};
use macro_master::{
    variants::Mos, AlignMode, Block, CompBuilder, Composition, GenError, InOut, Io, PortInfo,
    Process, Signal,
};
use pnr_core::DeviceKind;

#[derive(Default)]
struct PairIo {
    a: InOut<Signal>,
    b: InOut<Signal>,
    tail: InOut<Signal>,
}
impl Io for PairIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![self.a.port("a"), self.b.port("b"), self.tail.port("tail")]
    }
}

struct Pair;
impl Block for Pair {
    type Io = PairIo;
    fn name(&self) -> String {
        "pair".into()
    }
}
impl Composition for Pair {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        let sep = c.process().rule("min_spacing", 80).max(c.process().rule("diff_spacing", 270));
        let mos = || Mos::new(DeviceKind::Nmos, 420, 150, 2);
        let i1 = c.instantiate("m1", &mos())?;
        let m1 = c.place(i1)?;
        let i2 = c.instantiate("m2", &mos())?;
        let m2 = c.place_by(i2, AlignMode::ToTheRight, &m1, sep)?;
        c.connect(&m1.term("g"), "a");
        c.connect(&m2.term("g"), "b");
        c.connect(&m1.term("s"), "tail");
        c.connect(&m2.term("s"), "tail");
        Ok(())
    }
}

#[test]
fn pair_elaborates_and_routes_on_sky130() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");

    let sol = elaborate(&Pair, &pdk, &ElabConfig::default()).expect("elaboration succeeds");

    assert_eq!(sol.macros.len(), 2, "two placed instances");
    // The tail net joins m1.s and m2.s across the two instances — it must have
    // drawn wires.
    let tail = sol.nets.iter().position(|n| n == "tail").expect("tail net exists");
    assert!(
        !sol.routes.shapes(pnr_core::NetId(tail as u16)).is_empty(),
        "tail net is routed (report: {} hard violations)",
        sol.report.hard_violations.len()
    );
    // Geometry is non-degenerate and includes both device shapes and wires.
    assert!(sol.geometry().len() > sol.macros.iter().map(|m| m.shapes.len()).sum::<usize>());
}

