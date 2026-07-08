//! Built-in cell generators: netlist devices → physical layout.
//!
//! Each device family defines a spec type implementing [`CellSpec`]:
//! enumerate feasible variants, estimate their bbox, and draw pure
//! geometry. The engine drives candidate selection — generators don't
//! rank or pick winners.

pub mod mosfet;
pub mod resistor;
pub mod capacitor;
pub mod bjt;
pub mod diode;
pub mod inductor;

use substrate3::{CellBuilder, CellError, CellGenerator, PortDef};

pub use crate::pdk::Pdk;
use crate::device::DeviceRecord;

/// One point in a device family's layout design space.
///
/// Implement this to add a new cell type:
///
/// ```ignore
/// #[derive(Debug, Clone, Copy)]
/// pub struct MySpec { /* variant axes */ }
///
/// impl CellSpec for MySpec {
///     fn enumerate(devices: &[DeviceRecord], pdk: &Pdk) -> Vec<Self> {
///         vec![MySpec { /* ... */ }]
///     }
///     fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32) { (100, 100) }
///     fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef> { vec![] }
///     fn draw(&self, devices: &[DeviceRecord], pdk: &Pdk, b: &mut CellBuilder) -> Result<(), CellError> {
///         Ok(())
///     }
/// }
/// ```
///
/// [`SpecCell`] bridges to substrate3's [`CellGenerator`] automatically.
pub trait CellSpec: Clone + Send + Sync + 'static {
    /// All feasible layout variants. **Unranked** — the engine picks
    /// based on placement/routing feedback, not generator heuristics.
    fn enumerate(devices: &[DeviceRecord], pdk: &Pdk) -> Vec<Self>
    where
        Self: Sized;

    /// Bbox estimate (width, height) in nm without drawing geometry.
    fn estimate(&self, devices: &[DeviceRecord], pdk: &Pdk) -> (i32, i32);

    /// Port definitions for this variant.
    fn ports(&self, devices: &[DeviceRecord]) -> Vec<PortDef>;

    /// Pure geometry emission. No ranking, no heuristics.
    fn draw(
        &self,
        devices: &[DeviceRecord],
        pdk: &Pdk,
        b: &mut CellBuilder,
    ) -> Result<(), CellError>;
}

/// Bridges any [`CellSpec`] to substrate3's [`CellGenerator`].
pub struct SpecCell<S: CellSpec> {
    pub spec: S,
    pub devices: Vec<DeviceRecord>,
    pub pdk: Pdk,
}

impl<S: CellSpec> CellGenerator for SpecCell<S> {
    fn ports(&self) -> Vec<PortDef> {
        self.spec.ports(&self.devices)
    }

    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        self.spec.draw(&self.devices, &self.pdk, b)
    }
}

/// Enumerate specs and wrap as `Box<dyn CellGenerator>` for dyn dispatch.
pub fn candidates<S: CellSpec>(
    devices: &[DeviceRecord],
    pdk: &Pdk,
) -> Vec<Box<dyn CellGenerator>> {
    S::enumerate(devices, pdk)
        .into_iter()
        .map(|spec| -> Box<dyn CellGenerator> {
            Box::new(SpecCell {
                spec,
                devices: devices.to_vec(),
                pdk: pdk.clone(),
            })
        })
        .collect()
}

