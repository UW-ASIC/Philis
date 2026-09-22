//! The PDK seam generators are written against. Layers are looked up by role
//! (`"poly"`, `"metal1"`) and rules by name, so no generator hard-codes a PDK
//! number. The real impl is `verify::Pdk`.

use crate::LayerId;

pub trait Process {
    /// Layer for a role; `None` when this process omits that (optional) layer.
    fn layer(&self, role: &str) -> Option<LayerId>;

    /// Rule value in nm, or `default` when the process does not specify it.
    fn rule(&self, name: &str, default: i32) -> i32;

    /// Fabrication grid, nm. Every emitted coordinate snaps to it.
    fn grid(&self) -> i32;
}
