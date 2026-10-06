//! The parsed circuit: devices, nets, and the groups the annotator recovers.

use crate::ids::{DeviceId, NetId};

/// A device's electrical kind — routes it to the right `cells` generator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    /// n-channel MOSFET; terminals G, D, S, B.
    Nmos,
    /// p-channel MOSFET; terminals G, D, S, B.
    Pmos,
    /// Two-terminal resistor.
    Resistor,
    /// Two-terminal capacitor.
    Capacitor,
    /// BJTs split by polarity: marker, well and LVS device class all differ.
    Npn,
    /// PNP bipolar transistor (see [`DeviceKind::Npn`]).
    Pnp,
    /// Junction diode.
    Diode,
    /// Inductor.
    Inductor,
}

/// One device instance from the netlist.
#[derive(Clone)]
pub struct Device {
    /// Hierarchical instance name (`X1/X3/M2`).
    pub name: String,
    /// Electrical kind.
    pub kind: DeviceKind,
    /// The SPICE model name as written (`sky130_fd_pr__res_high_po`, …):
    /// selects the drawn construction and the LVS recogniser. Empty = none.
    pub model: String,
    /// Terminal name → net it connects to (e.g. `"G" -> NetId`), in the
    /// kind's terminal order (FETs: G, D, S, B).
    pub terminals: Vec<(String, NetId)>,
    /// Parameters (W, L, multiplier…), name → value in `nm`/PDK units. A MOS
    /// `w` is the SPICE instance total over its `nf` fingers ([`MosSize`]).
    /// An `R`/`C`/`L` letter card's value is `r_mohm` (mΩ), `c_af` (aF) or
    /// `ind_ph` (pH).
    pub params: Vec<(String, i64)>,
}

/// A MOS instance's size in SPICE/BSIM4 semantics: `w_total_nm` is one
/// instance's gate width, split over `nf` fingers; `m` instances in parallel.
/// Layout, the LVS reference, the annotator and the simulator card all read
/// this one record, so the drawn channel is the simulated one.
///
/// Invariant (guaranteed by [`Device::mos_size`]): `w_total_nm`, `l_nm` > 0
/// and `nf`, `m` ≥ 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MosSize {
    /// Gate width of one instance, summed over its fingers, nm.
    pub w_total_nm: i64,
    /// Drawn gate length, nm.
    pub l_nm: i64,
    /// Fingers per instance (≥ 1).
    pub nf: u32,
    /// Parallel instances (≥ 1).
    pub m: u32,
}

impl MosSize {
    /// Per-finger width, nm (floor; the < nf nm residue is inside LVS's 2 % tolerance).
    ///
    /// # Panics
    /// If `nf == 0` (outside the invariant).
    #[must_use]
    pub fn w_finger_nm(self) -> i64 {
        self.w_total_nm / i64::from(self.nf)
    }

    /// Drawn fingers = nf·m, saturating at `u32::MAX`.
    #[must_use]
    pub fn fingers(self) -> u32 {
        self.nf.saturating_mul(self.m)
    }

    /// W_total·L·m, µm².
    #[must_use]
    pub fn gate_area_um2(self) -> f64 {
        (self.w_total_nm * self.l_nm) as f64 * 1e-6 * f64::from(self.m)
    }
}

impl Device {
    /// `None` for a non-MOS or when `w`/`l` is missing or ≤ 0 (never a silent
    /// 0 µm²). `nf` and `m` default to 1 and read as at least 1; `multi` (the
    /// HSPICE-style alias) multiplies into `m`.
    #[must_use]
    pub fn mos_size(&self) -> Option<MosSize> {
        if !matches!(self.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            return None;
        }
        let p = |k: &str| self.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
        let count = |k: &str| p(k).map_or(1, |v| v.clamp(1, i64::from(u32::MAX)) as u32);
        Some(MosSize {
            w_total_nm: p("w").filter(|&v| v > 0)?,
            l_nm: p("l").filter(|&v| v > 0)?,
            nf: count("nf"),
            m: count("m").saturating_mul(count("multi")),
        })
    }

    /// [`MosSize::gate_area_um2`], µm²; `0` when [`Device::mos_size`] is `None`.
    #[must_use]
    pub fn gate_area_um2(&self) -> f64 {
        self.mos_size().map_or(0.0, MosSize::gate_area_um2)
    }
}

/// One net (a wire connecting terminals).
#[derive(Clone)]
pub struct Net {
    /// Hierarchical net name (`X1/X3/out`; top-level nets bare).
    pub name: String,
}

/// A group of devices the annotator has decided belong together (a diff pair,
/// current mirror, cascode). The unit that `cells` draws and the placer places.
pub struct DeviceGroup {
    /// Member devices.
    pub devices: Vec<DeviceId>,
}

/// The whole circuit as flat SoA tables. IDs index these vectors.
///
/// Hierarchy is flattened by the parser: a device inside instance `X1/X3`
/// is named `X1/X3/<name>` and its internal nets `X1/X3/<net>`. `ports`,
/// `insts`, `device_inst` and `sources` are filled by the SPICE front end
/// only; a netlist built elsewhere leaves them empty (every device top level).
#[derive(Clone, Default)]
pub struct Netlist {
    /// Every device, by [`DeviceId`].
    pub devices: Vec<Device>,
    /// Every net, by [`NetId`].
    pub nets: Vec<Net>,
    /// Top sub-circuit ports in declaration order (empty: no .subckt around the top).
    pub ports: Vec<NetId>,
    /// Every flattened sub-circuit instance, parents before children.
    pub insts: Vec<SubcktInst>,
    /// Innermost instance of each device; `None` = top level. Parallel to
    /// `devices` when the parser built it, else empty.
    pub device_inst: Vec<Option<u32>>,
    /// Independent/controlled sources and couplings: evidence, not devices.
    pub sources: Vec<SourceCard>,
}

/// One flattened `.subckt` instance.
#[derive(Clone, Debug)]
pub struct SubcktInst {
    /// Hierarchical instance path, `"X1/X3"`.
    pub path: String,
    /// The sub-circuit's name as declared.
    pub subckt: String,
    /// Index of the enclosing instance in [`Netlist::insts`]; `None` = top.
    pub parent: Option<u32>,
    /// Actual nets, in the sub-circuit's formal-port order.
    pub ports: Vec<NetId>,
}

/// A `V I E F G H B K` card: what drives or couples the circuit, kept as
/// evidence (rails, inputs, clocks) rather than drawn.
#[derive(Clone, Debug)]
pub struct SourceCard {
    /// Hierarchical card name (`V1`, `X1/Vb`).
    pub name: String,
    /// Card letter, upper case: `V I E F G H B K`.
    pub kind: char,
    /// Nodes in card order: 2 for `V I F H B`, 4 for `E G`, none for `K`.
    pub nodes: Vec<NetId>,
    /// DC value of a `V`/`I` card (after `dc`, else its first value), base SI.
    pub dc: Option<f64>,
    /// A `PULSE`/`PWL`/`SIN`/`EXP` waveform is present.
    pub waveform: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mos_size_follows_spice_semantics() {
        let mos = |params: &[(&str, i64)]| Device {
            name: "M1".into(),
            kind: DeviceKind::Nmos,
            model: String::new(),
            terminals: Vec::new(),
            params: params.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
        };
        let s = mos(&[("w", 10_000), ("l", 1_000), ("nf", 2)]).mos_size().unwrap();
        assert_eq!(s.w_finger_nm(), 5_000);
        assert_eq!(s.fingers(), 2);
        assert!((s.gate_area_um2() - 10.0).abs() < 1e-9, "{}", s.gate_area_um2());
        let s = mos(&[("w", 10_000), ("l", 1_000), ("nf", 2), ("m", 4)]).mos_size().unwrap();
        assert_eq!(s.fingers(), 8);
        assert!((s.gate_area_um2() - 40.0).abs() < 1e-9, "{}", s.gate_area_um2());
        assert_eq!(mos(&[("w", 0), ("l", 1_000)]).mos_size(), None);
    }
}
