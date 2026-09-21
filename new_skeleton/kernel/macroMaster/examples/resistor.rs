//! Reference: **authoring a new Device** (the rare, opt-in raw-geometry path).
//!
//! A serpentine resistor isn't in the [`variants`](macro_master::variants)
//! library, so we write it as a [`DeviceGen`]: the *only* tier handed a
//! [`DeviceBuilder`], the *only* surface with `draw`. It resolves its layer by
//! role and its dimensions from rules (never a hardcoded number), then
//! [`check_device`] validates it against the [`GenericPdk`] — the synthetic
//! yardstick that forces PDK-agnostic authoring. Composition (see `inverter`)
//! never draws; it only assembles Devices like this one.
//!
//! Run: `cargo run -p macro_master --example resistor`.

use macro_master::{
    check_device, Block, DeviceBuilder, DeviceGen, GenError, GenericPdk, InOut, Io, PortInfo,
    Process, Signal,
};
use pnr_core::Rect;

/// Resistor IO: two terminals, both bidirectional (a resistor has no direction).
#[derive(Default)]
pub struct ResIo {
    pub a: InOut<Signal>,
    pub b: InOut<Signal>,
}
impl Io for ResIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![self.a.port("a"), self.b.port("b")]
    }
}

/// A serpentine resistor of `segments` unit bars — one Device, drawn raw.
pub struct SerpentineRes {
    pub segments: u16,
}

impl Block for SerpentineRes {
    type Io = ResIo;
    fn name(&self) -> String {
        format!("res_{}seg", self.segments)
    }
}

impl DeviceGen for SerpentineRes {
    fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
        let body = cell
            .process()
            .layer("rpoly")
            .or_else(|| cell.process().layer("poly")) // optional-layer fallback, substrate2-style
            .ok_or(GenError::IllegalLayer)?;
        let w = cell.process().rule("min_width", 60);
        let space = cell.process().rule("min_spacing", 80);
        let seg_len = w * 8; // unit bar length, a rule-derived multiple of min width
        let pitch = w + space;

        // Bars abut side-by-side; every coordinate is a rule-derived multiple, so
        // the same source draws on-grid on any PDK.
        for i in 0..i32::from(self.segments.max(1)) {
            cell.draw(body, Rect { x: i * pitch, y: 0, w, h: seg_len })?;
        }

        cell.connect("a", "b"); // both terminals live on the one serpentine body
        Ok(())
    }
}

fn main() {
    let res = SerpentineRes { segments: 6 };
    let report = check_device(&res, &GenericPdk::default());
    println!(
        "resistor device '{}': {} hard violations, cost {}",
        res.name(),
        report.hard_violations.len(),
        report.cost
    );
    assert!(report.hard_violations.is_empty(), "reference resistor device checks clean");
}
