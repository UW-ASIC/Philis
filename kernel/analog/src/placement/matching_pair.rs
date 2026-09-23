//! Device matching (Pelgrom) plus the shared FET recognisers.

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use pnr_core::{BipartiteHypergraph, DeviceKind};
use crate::rule::Rule;

// FET terminal positions in `device_nets` (G, D, S, B).
pub(crate) const G: usize = 0;
pub(crate) const D: usize = 1;
pub(crate) const S: usize = 2;

pub(crate) fn is_fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
}

/// Vth gradient over Pelgrom `A_Vth`, 1/µm². Both scale with oxide thickness,
/// so the ratio carries across processes (≈1 µV/µm over ≈4 mV·µm).
pub const GRADIENT_PER_AVT_UM2: f32 = 2.5e-4;

/// Pelgrom matching, `σ²(ΔVth) = A²/(W·L) + S²·D²`. `cost` pulls the pair
/// together (the `D²` term). The check is placement's share: the gradient term
/// `S·D` must stay within `gradient_share` of the random term `A/√(W·L)` the
/// sizing bought, so `A` cancels and only `S/A` ([`GRADIENT_PER_AVT_UM2`]) is
/// needed.
#[derive(Clone, Copy)]
pub struct MatchingPair {
    pub a: Target,
    pub b: Target,
    /// Gate area `W·L·fingers` of one device, µm² (from the netlist).
    pub gate_um2: f32,
    /// Allowed `σ_gradient / σ_random`.
    pub gradient_share: f32,
    pub matching: Matching,
}

/// Sizing axis of a matched pair.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Matching {
    /// L-dominated (current mirrors).
    Mirror,
    /// W·L (differential pairs).
    Cross,
}

impl Rule for MatchingPair {
    type On = Layout;
    /// Squared centre distance `· 1e-3`.
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        (dx * dx + dy * dy) * 1e-3
    }
    fn satisfied(self, l: &Layout) -> bool {
        self.gradient_over_random(l) <= self.gradient_share
    }
    fn residual(self, l: &Layout) -> f32 {
        crate::rule::over(self.gradient_over_random(l) - self.gradient_share, self.gradient_share)
    }
    fn usage(self, l: &Layout) -> Option<f32> {
        Some(self.gradient_over_random(l) / self.gradient_share.max(f32::EPSILON))
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}

impl MatchingPair {
    fn gradient_over_random(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        gradient_over_random(((ax - bx) as f32).hypot((ay - by) as f32), self.gate_um2)
    }
}

/// `σ_gradient / σ_random = (S/A)·D·√(W·L)` for devices `d_nm` apart.
pub(crate) fn gradient_over_random(d_nm: f32, gate_um2: f32) -> f32 {
    GRADIENT_PER_AVT_UM2 * (d_nm / 1000.0) * gate_um2.max(0.0).sqrt()
}

/// `a`, `b` form a differential pair: same FET kind, shared non-rail source,
/// distinct gates and drains, not cross-coupled.
pub(crate) fn is_diff_pair(hg: &BipartiteHypergraph, a: usize, b: usize) -> bool {
    if !is_fet(hg.kinds[a]) || hg.kinds[a] != hg.kinds[b] {
        return false;
    }
    let (na, nb) = (&hg.device_nets[a], &hg.device_nets[b]);
    na.len() > S
        && nb.len() > S
        && na[S] == nb[S]
        && na[G] != nb[G]
        && na[D] != nb[D]
        && na[G] != nb[D]
        && nb[G] != na[D]
        && !is_supply(hg, na[S])
}

/// Supply rail by name: case-insensitive prefix match on the usual roots
/// (sky130 `VPWR/VGND/VPB/VNB`, `vdd/vss…`), or `gnd` anywhere.
pub(crate) fn is_supply(hg: &BipartiteHypergraph, net: pnr_core::NetId) -> bool {
    let Some(name) = hg.net_names.get(net.0 as usize) else {
        return false;
    };
    let n = name.to_ascii_lowercase();
    const ROOTS: &[&str] = &["vdd", "vss", "vcc", "vee", "vpwr", "vgnd", "vpb", "vnb", "avdd", "avss", "dvdd", "dvss"];
    ROOTS.iter().any(|r| n.starts_with(r)) || n.contains("gnd")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    fn at(xb: i32) -> Layout {
        Layout {
            x: vec![0, xb],
            y: vec![0, 0],
            hw: vec![500; 2],
            hh: vec![500; 2],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        }
    }

    #[test]
    fn distance_and_device_area_both_drive_the_check() {
        let p = |gate_um2| MatchingPair {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            gate_um2,
            gradient_share: 0.3,
            matching: Matching::Cross,
        };
        // 20 µm² at 20 µm: ≈0.09 of σ_random — well inside.
        assert!(p(20.0).satisfied(&at(20_000)));
        // The same pair 1 mm apart has spent its share.
        assert!(!p(20.0).satisfied(&at(1_000_000)));
        // A bigger device buys less random mismatch, so the same distance costs more.
        assert!(p(2_000.0).usage(&at(20_000)) > p(20.0).usage(&at(20_000)));
    }
}
