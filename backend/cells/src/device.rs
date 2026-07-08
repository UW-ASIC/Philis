//! Device record from SPICE netlist — input to built-in generators.

use std::collections::HashMap;

use substrate3::DeviceType;

/// One device instance from the SPICE netlist.
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    /// Instance name (e.g. "M1").
    pub name: String,
    /// Electrical classification.
    pub device_type: DeviceType,
    /// Total width in nm.
    pub w: i32,
    /// Channel / body length in nm.
    pub l: i32,
    /// Number of fingers.
    pub nf: u16,
    /// Instance multiplier (m=).
    pub multiplier: u16,
    /// PDK model name (e.g. "nfet_01v8").
    pub model_name: String,
    /// Pin-name -> net-id connectivity.
    pub terminals: HashMap<String, u32>,
    /// Arbitrary SPICE parameters.
    pub params: HashMap<String, f64>,
}
