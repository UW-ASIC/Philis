//! Reference: a **Composition** — a differential pair, the *correct* way.
//!
//! The two matched transistors are separate multi-finger [`Mos`] Devices placed
//! side-by-side and wired by nets (their common `tail` is a **net**, not a merged
//! diffusion). This is the substrate2 model: shared diffusion is internal to a
//! Device (the fingers of one `Mos`); matching between Devices is by identical
//! parameters + net connectivity, never by overlapping instances. `place` would
//! reject an overlap, so "share diffusion by abutting two transistors" is simply
//! not expressible here — by design.
//!
//! Run: `cargo run -p macro_master --example diff_pair`.

use macro_master::{
    check_lvs, variants::Mos, AlignMode, Block, CompBuilder, Composition, GenError, GenericPdk,
    InOut, Input, Io, Output, PortInfo, Process, Schematic, Signal,
};
use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};

/// The differential IO: two matched inputs, two matched outputs, a tail node, and
/// a body rail. Directions are typed.
#[derive(Default)]
pub struct DiffPairIo {
    pub inp: Input<Signal>,
    pub inn: Input<Signal>,
    pub outp: Output<Signal>,
    pub outn: Output<Signal>,
    pub tail: InOut<Signal>,
    pub body: InOut<Signal>,
}
impl Io for DiffPairIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![
            self.inp.port("inp"),
            self.inn.port("inn"),
            self.outp.port("outp"),
            self.outn.port("outn"),
            self.tail.port("tail"),
            self.body.port("body"),
        ]
    }
}

/// An NMOS differential pair, each side a `w`-wide, `nf`-finger transistor.
pub struct DiffPair {
    pub w: i32,
    pub nf: u16,
}

impl Block for DiffPair {
    type Io = DiffPairIo;
    fn name(&self) -> String {
        format!("diffpair_w{}_nf{}", self.w, self.nf)
    }
}

impl Composition for DiffPair {
    fn build<P: Process>(&self, cell: &mut CompBuilder<P>) -> Result<(), GenError> {
        let sep = cell.process().rule("min_spacing", 80);

        // Two identical Devices — matched by construction (same params) — placed
        // side by side and connected by the tail net. No overlap, no shared
        // diffusion between them.
        let mos = || Mos::new(DeviceKind::Nmos, self.w, 60, self.nf);
        let li = cell.instantiate("mp", &mos())?;
        let left = cell.place(li)?;
        let ri = cell.instantiate("mn", &mos())?;
        let right = cell.place_by(ri, AlignMode::ToTheRight, &left, sep)?;

        cell.connect(&left.term("g"), "inp");
        cell.connect(&right.term("g"), "inn");
        cell.connect(&left.term("d"), "outp");
        cell.connect(&right.term("d"), "outn");
        cell.connect(&left.term("s"), "tail");
        cell.connect(&right.term("s"), "tail");
        cell.connect(&left.term("b"), "body");
        cell.connect(&right.term("b"), "body");
        Ok(())
    }
}

impl Schematic for DiffPair {
    fn schematic(&self) -> Netlist {
        let nets = ["inp", "inn", "outp", "outn", "tail", "body"]
            .iter()
            .map(|n| Net { name: (*n).into() })
            .collect();
        let (inp, inn, outp, outn, tail, body) =
            (NetId(0), NetId(1), NetId(2), NetId(3), NetId(4), NetId(5));
        let mos = |name: &str, g, d| Device {
            name: name.into(),
            kind: DeviceKind::Nmos,
            terminals: vec![("G".into(), g), ("D".into(), d), ("S".into(), tail), ("B".into(), body)],
            params: vec![("W".into(), i64::from(self.w))],
        };
        Netlist { nets, devices: vec![mos("MP", inp, outp), mos("MN", inn, outn)] }
    }
}

fn main() {
    let dp = DiffPair { w: 200, nf: 2 };
    let report = check_lvs(&dp, &GenericPdk::default());
    println!(
        "diff pair '{}': {} hard violations, cost {}",
        dp.name(),
        report.hard_violations.len(),
        report.cost
    );
    assert!(report.hard_violations.is_empty(), "reference diff pair composes clean");
}
