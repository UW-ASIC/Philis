//! The parsed circuit: devices, nets, and the groups the annotator recovers.

use crate::ids::{DeviceId, NetId};

/// A device's electrical kind — routes it to the right `cells` generator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    Nmos,
    Pmos,
    Resistor,
    Capacitor,
    /// BJTs split by polarity: marker, well and LVS device class all differ.
    Npn,
    Pnp,
    Diode,
    Inductor,
}

/// One device instance from the netlist.
#[derive(Clone)]
pub struct Device {
    pub name: String,
    pub kind: DeviceKind,
    /// The SPICE model name as written (`sky130_fd_pr__res_high_po`, …):
    /// selects the drawn construction and the LVS recogniser. Empty = none.
    pub model: String,
    /// Terminal name → net it connects to (e.g. `"G" -> NetId`).
    pub terminals: Vec<(String, NetId)>,
    /// Parameters (W, L, multiplier…), name → value in `nm`/PDK units. A MOS
    /// `w` is the SPICE instance total over its `nf` fingers ([`MosSize`]).
    pub params: Vec<(String, i64)>,
}

/// A MOS instance's size in SPICE/BSIM4 semantics: `w_total_nm` is one
/// instance's gate width, split over `nf` fingers; `m` instances in parallel.
/// Layout, the LVS reference, the annotator and the simulator card all read
/// this one record, so the drawn channel is the simulated one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MosSize {
    pub w_total_nm: i64,
    pub l_nm: i64,
    pub nf: u32,
    pub m: u32,
}

impl MosSize {
    /// Per-finger width, nm (floor; the < nf nm residue is inside LVS's 2 % tolerance).
    #[must_use]
    pub fn w_finger_nm(self) -> i64 {
        self.w_total_nm / i64::from(self.nf)
    }

    /// Drawn fingers = nf·m.
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
    pub name: String,
}

/// A group of devices the annotator has decided belong together (a diff pair,
/// current mirror, cascode). The unit that `cells` draws and the placer places.
pub struct DeviceGroup {
    pub devices: Vec<DeviceId>,
}

/// The whole circuit as flat SoA tables. IDs index these vectors.
#[derive(Clone)]
pub struct Netlist {
    pub devices: Vec<Device>,
    pub nets: Vec<Net>,
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
