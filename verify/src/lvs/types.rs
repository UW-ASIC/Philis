//! LVS type definitions: devices, netlists, results.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceKind { Nmos, Pmos, Npn, Pnp }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceFlavor { Standard, Lvt, Hvt }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwoTerminalKind { Resistor, Diode, Capacitor }

#[derive(Debug, Clone)]
pub struct TwoTerminalDevice {
    pub kind: TwoTerminalKind,
    pub name: String,
    pub terminal_a: u32,
    pub terminal_b: u32,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub kind: DeviceKind,
    pub gate: u32,
    pub source: u32,
    pub drain: u32,
    pub body: u32,
    pub flavor: DeviceFlavor,
    pub w: i32,
    pub l: i32,
    /// Device class tag for comparison (e.g., "mos", "dmos").
    pub device_class: Option<String>,
}

/// Exact source polygons used to recognize one MOS before any legacy reduction.
/// Polygon indices address the input [`GeometryStore`](crate::geometry::GeometryStore).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRecognitionSource {
    pub gate_polygon: u32,
    pub channel_polygon: u32,
    pub well_polygon: Option<u32>,
    pub rule_id: String,
}

/// BJT device extracted from layout.
#[derive(Debug, Clone)]
pub struct BjtDevice {
    pub kind: DeviceKind, // Npn or Pnp
    pub collector: u32,
    pub base: u32,
    pub emitter: u32,
    pub name: String,
}

/// A net with polygons but no device terminal connections.
#[derive(Debug, Clone)]
pub struct FloatingNet {
    pub net_id: u32,
    pub label: Option<String>,
    pub polygon_count: usize,
}

pub struct ExtractedNetlist {
    pub devices: Vec<Device>,
    /// One entry per `devices` item when recognition provenance is available.
    /// Legacy/manual netlists and reduced devices deliberately leave this empty.
    pub device_sources: Vec<DeviceRecognitionSource>,
    pub bjt_devices: Vec<BjtDevice>,
    pub net_count: usize,
    pub used_nets: usize,
    pub net_of_poly: Vec<u32>,
    pub label_conflicts: Vec<String>,
    pub two_terminal: Vec<TwoTerminalDevice>,
    pub floating_nets: Vec<FloatingNet>,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractOpts {
    pub cut_required: bool,
    pub hierarchical: bool,
    pub black_box_cells: Vec<String>,
    pub lvs_strict: bool,
}

// --- reference netlist ---

#[derive(Debug, Clone)]
pub struct RefDevice {
    pub kind: DeviceKind,
    pub gate: String,
    pub source: String,
    pub drain: String,
    pub w: i32,
    pub l: i32,
    pub flavor: DeviceFlavor,
}

#[derive(Debug, Clone)]
pub struct RefTwoTerminal {
    pub kind: TwoTerminalKind,
    pub name: String,
    pub terminal_a: String,
    pub terminal_b: String,
}

#[derive(Debug, Clone)]
pub struct RefBjt {
    pub kind: DeviceKind,
    pub name: String,
    pub collector: String,
    pub base: String,
    pub emitter: String,
}

#[derive(Debug, Clone)]
pub struct RefNetlist {
    pub devices: Vec<RefDevice>,
    pub net_seeds: HashMap<String, String>,
    pub ref_two_terminal: Vec<RefTwoTerminal>,
    pub ref_bjt: Vec<RefBjt>,
}

// --- results ---

#[derive(Debug)]
pub enum Mismatch {
    DeviceCount { kind: String, extracted: usize, reference: usize },
    TopologyMismatch { description: String },
    ParametricMismatch { property: String, got: f64, expected: f64, tolerance: f64 },
    FloatingNet { net_id: u32, label: Option<String> },
    LabelConflict { net_id: u32, labels: Vec<String> },
    NetSeedConflict { nets: Vec<String> },
}

#[derive(Debug)]
pub struct LvsResult {
    pub matched: bool,
    pub reason: String,
    pub mismatches: Vec<Mismatch>,
    pub extracted_devices: usize,
    pub nmos: usize,
    pub pmos: usize,
    pub ambiguous_classes: usize,
    pub label_conflicts: Vec<String>,
    pub floating_nets: Vec<FloatingNet>,
}
