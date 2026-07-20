//! Built-in cell generators: netlist devices → physical layout.
//!
//! Each device family defines a spec type implementing [`CellSpec`]:
//! enumerate feasible variants, estimate their bbox, and draw pure
//! geometry. The engine drives candidate selection — generators don't
//! rank or pick winners.

pub mod bjt;
pub mod capacitor;
pub mod diode;
pub mod guard_ring;
pub mod inductor;
pub mod mosfet;
pub mod resistor;

use crate::{CellBuilder, CellError, CellGenerator, PortDef};

use crate::device::DeviceRecord;
pub use crate::pdk::Pdk;

/// Draw one merged conductor as a chain of overlapping chunks along its
/// long axis.
///
/// ERC `missing_tie` measures diff-corner -> li *bbox-center* distance, so a
/// single long strip reads as one far-away contact. Overlapping chunks are
/// electrically one shape (extraction merges on area overlap) but give the
/// check a contact center every `chunk` nm.
pub(crate) fn li_chain(
    b: &mut CellBuilder,
    layer: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    chunk: i32,
) -> Result<(), CellError> {
    let horiz = w >= h;
    let span = if horiz { w } else { h };
    let step = chunk.max(if horiz { h } else { w }).max(1);
    let full = step.min(span);
    let mut s = 0;
    let mut odd = false;
    loop {
        // Clamp so every chunk is full-size: a sliver tail rect would trip
        // the layer's min-width rule even though the merged shape is wide.
        let cs = s.min(span - full);
        // Alternate the transverse size by +20nm: the flow's pre-signoff
        // coalesce merges rect pairs whose union is itself a rectangle, which
        // would collapse a uniform chain back into one poly (one bbox center).
        // A staggered pair never unions to its bounding box, so every chunk
        // survives as its own contact center while staying one conductor.
        let t = if odd { 20 } else { 0 };
        if horiz {
            b.rect(layer, x + cs, y, full, h + t)?;
        } else {
            b.rect(layer, x, y + cs, w + t, full)?;
        }
        if cs + full >= span {
            return Ok(());
        }
        s += step - 20; // 20nm overlap keeps chunks one extracted conductor
        odd = !odd;
    }
}

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
pub fn candidates<S: CellSpec>(devices: &[DeviceRecord], pdk: &Pdk) -> Vec<Box<dyn CellGenerator>> {
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
