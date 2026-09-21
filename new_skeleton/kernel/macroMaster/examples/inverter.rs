//! Reference: a **Composition** — a CMOS inverter assembled from two library
//! [`Mos`](macro_master::variants::Mos) Devices.
//!
//! No raw geometry: [`Composition`] gets a [`CompBuilder`], which has
//! `instantiate` + `place` + `connect` but **no** `draw`. The two transistors are
//! placed with *relative* alignment ([`AlignMode::Above`]) — `place` rejects any
//! overlap at call time — and wired by nets. [`check_lvs`] runs the structural
//! LVS (declared connect-graph vs [`Schematic`]). Real geometric DRC of the
//! routed result is downstream signoff, not here.
//!
//! Run: `cargo run -p macro_master --example inverter`.

use macro_master::{
    check_lvs, variants::Mos, AlignMode, Block, CompBuilder, Composition, GenError, GenericPdk,
    InOut, Input, Io, Output, PortInfo, Process, Schematic, Signal,
};
use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};

/// Inverter IO: signal in, signal out, power rails. Directions are types, so
/// wiring `vdd` where `din` is expected is a compile error.
#[derive(Default)]
pub struct InverterIo {
    pub din: Input<Signal>,
    pub dout: Output<Signal>,
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
}
impl Io for InverterIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![self.din.port("din"), self.dout.port("dout"), self.vdd.port("vdd"), self.vss.port("vss")]
    }
}

/// A CMOS inverter parameterised by device widths (`nm`).
pub struct Inverter {
    /// PMOS finger width.
    pub pw: i32,
    /// NMOS finger width.
    pub nw: i32,
}

impl Block for Inverter {
    type Io = InverterIo;
    fn name(&self) -> String {
        format!("inv_p{}_n{}", self.pw, self.nw)
    }
}

impl Composition for Inverter {
    fn build<P: Process>(&self, cell: &mut CompBuilder<P>) -> Result<(), GenError> {
        let gap = cell.process().rule("min_spacing", 80);

        // Two Devices from the variant library — shared diffusion (fingers) lives
        // inside each Mos, never between them.
        let nmos = cell.instantiate("mn", &Mos::new(DeviceKind::Nmos, self.nw, 60, 1))?;
        let n = cell.place(nmos)?;
        // PMOS placed *above* the NMOS by a rule-derived gap — relative, no
        // absolute y, and `place` guarantees they don't overlap.
        let pmos = cell.instantiate("mp", &Mos::new(DeviceKind::Pmos, self.pw, 60, 1))?;
        let p = cell.place_by(pmos, AlignMode::Above, &n, gap)?;

        // Declared connectivity for the structural LVS / floating-port check.
        cell.connect(&n.term("g"), "din");
        cell.connect(&p.term("g"), "din");
        cell.connect(&n.term("d"), "dout");
        cell.connect(&p.term("d"), "dout");
        cell.connect(&p.term("s"), "vdd");
        cell.connect(&p.term("b"), "vdd");
        cell.connect(&n.term("s"), "vss");
        cell.connect(&n.term("b"), "vss");
        Ok(())
    }
}

impl Schematic for Inverter {
    fn schematic(&self) -> Netlist {
        let nets = ["vdd", "vss", "din", "dout"].iter().map(|n| Net { name: (*n).into() }).collect();
        let (vdd, vss, din, dout) = (NetId(0), NetId(1), NetId(2), NetId(3));
        Netlist {
            nets,
            devices: vec![
                Device {
                    name: "MP".into(),
                    kind: DeviceKind::Pmos,
                    terminals: vec![("D".into(), dout), ("G".into(), din), ("S".into(), vdd)],
                    params: vec![("W".into(), i64::from(self.pw))],
                },
                Device {
                    name: "MN".into(),
                    kind: DeviceKind::Nmos,
                    terminals: vec![("D".into(), dout), ("G".into(), din), ("S".into(), vss)],
                    params: vec![("W".into(), i64::from(self.nw))],
                },
            ],
        }
    }
}

fn main() {
    let inv = Inverter { pw: 240, nw: 120 };
    let report = check_lvs(&inv, &GenericPdk::default());
    println!(
        "inverter '{}': {} hard violations, cost {}",
        inv.name(),
        report.hard_violations.len(),
        report.cost
    );
    assert!(report.hard_violations.is_empty(), "reference inverter composes clean");
}
