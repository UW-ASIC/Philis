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

/// Pelgrom matching, `σ²(ΔVth) = A²/(W·L) + S²·D²`. `cost` is the placement
/// (gradient, `D²`) term; `satisfied`/`residual` check the area (random) term
/// against the tier budget.
#[derive(Clone, Copy)]
pub struct MatchingPair {
    pub a: Target,
    pub b: Target,
    /// ΔVth budget, mV·10.
    pub max_dvth_mv10: i32,
    /// Width ratio `(num, den)`. Read by nothing: integer ratios are enforced
    /// by `cell::Unitization`.
    pub w_ratio: (u16, u16),
    /// Pelgrom `A_Vth`, µV·µm.
    pub avt_uv_um: i32,
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
        let (sigma_uv, budget_uv) = self.mismatch_uv(l);
        sigma_uv <= budget_uv
    }
    fn residual(self, l: &Layout) -> f32 {
        let (sigma_uv, budget_uv) = self.mismatch_uv(l);
        crate::rule::over(sigma_uv - budget_uv, budget_uv)
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}

impl MatchingPair {
    /// `(A_Vth/√(W·L), budget)` in µV, with `W·L` from `a`'s drawn extents.
    fn mismatch_uv(self, l: &Layout) -> (f32, f32) {
        let (hw, hh) = l.extent(self.a);
        let w_um = (2 * hw) as f32 / 1000.0;
        let l_um = (2 * hh) as f32 / 1000.0;
        let area_um2 = (w_um * l_um).max(1e-6);
        (self.avt_uv_um as f32 / area_um2.sqrt(), self.max_dvth_mv10 as f32 * 100.0)
    }
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
