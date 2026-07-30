//! The parsed circuit: devices, nets, and the groups the annotator recovers.

use crate::ids::{DeviceId, NetId};

/// A device's electrical kind — routes it to the right `cells` generator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    Nmos,
    Pmos,
    Resistor,
    Capacitor,
    Bjt,
    Diode,
    Inductor,
}

/// One device instance from the netlist.
pub struct Device {
    pub name: String,
    pub kind: DeviceKind,
    /// Terminal name → net it connects to (e.g. `"G" -> NetId`).
    pub terminals: Vec<(String, NetId)>,
    /// Parameters (W, L, multiplier…), name → value in `nm`/PDK units.
    pub params: Vec<(String, i64)>,
}

/// One net (a wire connecting terminals).
pub struct Net {
    pub name: String,
}

/// A group of devices the annotator has decided belong together (a diff pair,
/// current mirror, cascode). The unit that `cells` draws and the placer places.
pub struct DeviceGroup {
    pub devices: Vec<DeviceId>,
}

/// The whole circuit as flat SoA tables. IDs index these vectors.
pub struct Netlist {
    pub devices: Vec<Device>,
    pub nets: Vec<Net>,
}
