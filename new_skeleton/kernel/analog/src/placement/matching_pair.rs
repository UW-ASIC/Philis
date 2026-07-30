//! Device matching (placement tier) — the Pelgrom-law core.
//!
//! Ported from `backend/constraints/src/placement_level/symmetry.rs`
//! (`MatchingPair`).

use pnr_core::ids::{GroupId, Target};
use pnr_core::layout::Layout;
use pnr_core::{BipartiteHypergraph, DeviceKind, UnionFind};
use crate::rule::Rule;

// MOSFET terminal order in `device_nets` is G, D, S, B (see the `cells` MOSFET
// generator / `macroMaster` diff_pair example). These index a device's terminal
// nets; the hypergraph drops the pin *names*, so recognition keys off position.
pub(crate) const G: usize = 0;
pub(crate) const D: usize = 1;
pub(crate) const S: usize = 2;

/// A FET (only FETs carry the G/D/S/B ordering the recognisers rely on).
pub(crate) fn is_fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
}

/// **Matching pair.** Nominally identical devices suffer random mismatch by
/// Pelgrom's law: `σ²(ΔP) = A_P²/(W·L) + S_P²·D²`. Common-centroid/mirror
/// placement minimises separation `D` and cancels gradients; the tier sets how
/// hard. Ratioed devices (current mirrors, BJT area) carry a `w_ratio`.
///
/// - **Enforcement:** [`crate::Mode::Hard`] for Moderate+ tiers, [`crate::Mode::Cost`]
///   for Minimal (priority 95/80/50/30 by tier). The canonical example of a rule
///   whose mode is chosen at application.
/// - **Arity:** Device↔Device.
/// - **Books:** AOAL ch08/8.1 (#55), 8.2.7 (#62), ch13/13.2.1 (#42); ALS 2.2.2 (#34);
///   PNR_ANALOG 00/1.1, 1.3–1.7 (#3/#39).
#[derive(Clone, Copy)]
pub struct MatchingPair {
    pub a: Target,
    pub b: Target,
    /// Max threshold-voltage mismatch budget, mV·10 (tier-derived).
    pub max_dvth_mv10: i32,
    /// Integer width ratio `(num, den)` for ratioed devices; `(1, 1)` if identical.
    ///
    /// AUDIT (`docs/API-WISH.md`): **read by nothing.** Neither `cost`, `satisfied` nor
    /// `residual` touches it, and every emitter passes `(1, 1)`
    /// (`annotator::emit::emit_leaf`), so PLAN §4a's integer-ratio equality — "a 1:8
    /// bandgap is eight unit devices, never emitter scaling" — is currently unenforced at
    /// this tier. That is defensible only because it is enforced *elsewhere*: the ratio is
    /// exact by construction once [`crate::cell::Unitization`] decomposes both devices into
    /// identical units (`target_ratio` / `dev_nf`), which is the "unitization" half of
    /// §4a and lives in the cell tier where it belongs. The gap is that nothing checks the
    /// two agree, so a `w_ratio` that contradicts the emitted `Unitization` is silent.
    pub w_ratio: (u16, u16),
    /// Pelgrom `A_Vth` area coefficient, µV·µm (a process constant from the PDK).
    pub avt_uv_um: i32,
    /// Sizing axis. `Mirror` matches on `L` (current mirrors), `Cross` on `W·L`
    /// (differential pairs) — the old `MatchingPair.matching_type`. It lives on the
    /// matching constraint, not as free-floating vocabulary.
    pub matching: Matching,
}

/// Sizing axis for a matched pair — an attribute of the matching constraint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Matching {
    /// L-dominated (current mirrors).
    Mirror,
    /// W·L (differential pairs).
    Cross,
}

impl Rule for MatchingPair {
    type On = Layout;
    /// Pelgrom **gradient** term `S_P²·D²`: mismatch grows with the square of
    /// separation `D`, so the placement-tunable part is squared centre distance
    /// (`· 1e-3`, the engine's analog-pull form).
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        (dx * dx + dy * dy) * 1e-3
    }
    /// Pelgrom **random** term `σ(ΔVth) = A_Vth / √(W·L)` within the tier budget.
    /// Device area `W·L` comes from the drawn half-extents (`W ≈ 2·hw`, `L ≈ 2·hh`,
    /// nm → µm), the reason this needs [`Layout::extent`].
    fn satisfied(self, l: &Layout) -> bool {
        let (sigma_uv, budget_uv) = self.mismatch_uv(l);
        sigma_uv <= budget_uv
    }

    /// Pelgrom σ(ΔVth) past the tier's mismatch budget, as a fraction of that budget.
    ///
    /// Normalises the *random* term, matching [`satisfied`](Rule::satisfied) — the half
    /// set by device area, i.e. by the variant/unitization choice. The gradient term
    /// `S_P²·D²` that `cost` scores is the placement-tunable half and has no declared
    /// spec on this rule, so it stays purely objective; folding a separation into this
    /// residual would let a well-matched-by-area pair report a Θ violation for being far
    /// apart, which is a cost, not a budget miss.
    fn residual(self, l: &Layout) -> f32 {
        let (sigma_uv, budget_uv) = self.mismatch_uv(l);
        crate::rule::over(sigma_uv - budget_uv, budget_uv)
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}

impl MatchingPair {
    /// `(σ(ΔVth), budget)` in µV — the one place the Pelgrom arithmetic lives, so
    /// [`Rule::satisfied`] and [`Rule::residual`] cannot disagree about where the spec is.
    fn mismatch_uv(self, l: &Layout) -> (f32, f32) {
        let (hw, hh) = l.extent(self.a);
        let w_um = (2 * hw) as f32 / 1000.0;
        let l_um = (2 * hh) as f32 / 1000.0;
        let area_um2 = (w_um * l_um).max(1e-6);
        (
            self.avt_uv_um as f32 / area_um2.sqrt(),
            self.max_dvth_mv10 as f32 * 100.0, // mV·10 → µV
        )
    }
}

/// Devices `a`, `b` are a differential pair: shared source, distinct gates+drains,
/// not cross-coupled, and the shared source is **not a supply rail** (see
/// [`is_supply`]). The rail exclusion is what separates a true diff pair (common
/// source on a tail, e.g. M1/M2 on `tail`) from two devices that merely share
/// `vdd`/`vss` (M4/M6) — sharing a rail is not differential.
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

/// A net is a **supply rail** (power or ground) by its name — the standard,
/// deterministic way tools identify supplies. A differential pair's common source
/// must not be one (a rail is a low-impedance supply, not a current-source tail).
/// Prefix-matches the conventional roots across the target PDKs (sky130
/// `VPWR/VGND/VPB/VNB`, generic `vdd/vss`), catching numbered/analog variants
/// (`vdd1`, `vssa`, `vddio`); `gnd` is matched anywhere (`dgnd`, `agnd`).
/// Case-insensitive. (Reads [`BipartiteHypergraph::net_names`] — the same net
/// classification `isolation`/`dti` can build their well/injection tests on.)
pub(crate) fn is_supply(hg: &BipartiteHypergraph, net: pnr_core::NetId) -> bool {
    let Some(name) = hg.net_names.get(net.0 as usize) else {
        return false;
    };
    let n = name.to_ascii_lowercase();
    const ROOTS: &[&str] = &[
        "vdd", "vss", "vcc", "vee", "vpwr", "vgnd", "vpb", "vnb", "avdd", "avss",
        "dvdd", "dvss",
    ];
    ROOTS.iter().any(|r| n.starts_with(r)) || n.contains("gnd")
}

/// The union-find root of device `d` as a [`GroupId`] — the group handle the
/// annotator reads back after all `extract`s have run their unions.
pub(crate) fn group_of(uf: &mut UnionFind, d: usize) -> GroupId {
    GroupId(uf.find(d as u32) as u16)
}
