//! Net role by name (supply / ground / clock / signal), bulk-inferred rails + user recognition
//! overrides. Gates recognition: a diff pair's gates must be signals.

use std::collections::HashSet;

use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

/// Electrical role of a net, keyed off its name (bulk connectivity when no rail is named). Index the returned `Vec` by
/// [`pnr_core::ids::NetId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRole {
    /// Anything not recognised as a rail or clock.
    Signal,
    /// Positive supply rail.
    Supply,
    /// Ground or the most negative rail.
    Ground,
    /// Clock or phase line.
    Clock,
}

/// Supply rail roots (**Philis policy**, not a source's list); `vpb` is the
/// sky130 PMOS body pin.
const SUPPLY_ROOTS: &[&str] = &["vdd", "vcc", "vpwr", "vddio", "avdd", "dvdd", "vdda", "vddd", "vpb"];
/// Ground rail roots; `vnb` is the sky130 NMOS body pin, `vee` the most
/// negative rail of a bipolar circuit (low impedance, hence ground-class).
const GROUND_ROOTS: &[&str] =
    &["vss", "gnd", "vgnd", "vssio", "avss", "dvss", "vssa", "vssd", "vnb", "vee", "agnd", "dgnd"];
/// Substrings that mark a clock anywhere in the name.
const CLK_SUBSTR: &[&str] = &["clk", "clock"];
/// Clock prefixes that must be followed by digits/`_`/`b`/end (`phi1`, `ck_b`),
/// so `back`/`stack`/`phase` stay signals.
const CLK_PREFIX: &[&str] = &["phi", "ck"];

/// Rail role from a net name alone; the one source of truth for every crate.
/// Case-insensitive; a trailing `!` (SPICE global) is stripped; `0` is ground;
/// a root matches when followed by nothing, `_…`, or only [0-9pv] (vdd1, avdd3v3, vdd1p8).
#[must_use]
pub fn rail_of(name: &str) -> Option<NetRole> {
    let s = name.trim_end_matches('!').to_ascii_lowercase();
    if s == "0" {
        return Some(NetRole::Ground);
    }
    [(SUPPLY_ROOTS, NetRole::Supply), (GROUND_ROOTS, NetRole::Ground)].into_iter().find_map(|(roots, role)| {
        roots
            .iter()
            .any(|r| {
                s.strip_prefix(r).is_some_and(|rest| {
                    rest.is_empty()
                        || rest.starts_with('_')
                        || rest.bytes().all(|c| c.is_ascii_digit() || c == b'p' || c == b'v')
                })
            })
            .then_some(role)
    })
}

/// Classify every net by name: config names win, then [`rail_of`], then clock
/// names. Bulk inference runs per rail role only when no net got that role by
/// name or config: the net that is the `B` terminal of the most PMOS (NMOS)
/// devices (ties to the lowest net id) becomes Supply (Ground) if it is a
/// Signal on no FET gate; otherwise nothing is inferred. A named rail
/// therefore always disables the inference. One role per net, in net order.
#[must_use]
pub fn classify_nets(hg: &BipartiteHypergraph, cfg: &AnnotationConfig) -> Vec<NetRole> {
    let mut roles: Vec<NetRole> = hg
        .net_names
        .iter()
        .map(|name| {
            let named = |set: &[String]| set.iter().any(|s| s.eq_ignore_ascii_case(name));
            if named(&cfg.supply_nets) {
                NetRole::Supply
            } else if named(&cfg.ground_nets) {
                NetRole::Ground
            } else if let Some(rail) = rail_of(name) {
                rail
            } else if named(&cfg.clock_nets) || is_clock(&name.to_ascii_lowercase()) {
                NetRole::Clock
            } else {
                NetRole::Signal
            }
        })
        .collect();
    // ponytail: evidence (`Structure`) is not recorded until EXT-18 adds `Intent.nets[n].evidence`.
    let fet = |k: DeviceKind| matches!(k, DeviceKind::Nmos | DeviceKind::Pmos);
    let mut on_gate = vec![false; roles.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        if fet(hg.kinds[d]) {
            for (t, n) in hg.terminals[d].iter().zip(nets) {
                on_gate[n.0 as usize] |= t == "G";
            }
        }
    }
    for (kind, role) in [(DeviceKind::Pmos, NetRole::Supply), (DeviceKind::Nmos, NetRole::Ground)] {
        if roles.contains(&role) {
            continue;
        }
        let mut bulk = vec![0usize; roles.len()];
        for (d, nets) in hg.device_nets.iter().enumerate() {
            if hg.kinds[d] == kind {
                if let Some((_, n)) = hg.terminals[d].iter().zip(nets).find(|(t, _)| *t == "B") {
                    bulk[n.0 as usize] += 1;
                }
            }
        }
        // Max first, then the checks: a top-bulk rail that also drives a gate (a G=S=B dummy) must
        // not hand the role to the runner-up, typically a source-tied bulk (diff-pair tail).
        let best = (0..roles.len()).filter(|&n| bulk[n] > 0).max_by_key(|&n| (bulk[n], std::cmp::Reverse(n)));
        if let Some(n) = best.filter(|&n| !on_gate[n] && roles[n] == NetRole::Signal) {
            roles[n] = role;
        }
    }
    roles
}

/// A clock name: contains a [`CLK_SUBSTR`], or is a [`CLK_PREFIX`] followed only
/// by digits, `_` and `b`. `lower` must already be lowercase.
pub(crate) fn is_clock(lower: &str) -> bool {
    CLK_SUBSTR.iter().any(|p| lower.contains(p))
        || CLK_PREFIX.iter().any(|p| {
            lower.strip_prefix(p).is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit() || c == '_' || c == 'b'))
        })
}

/// User overrides on recognition — the library's channel to override our
/// decisions (ALIGN `DoNotIdentify` / `SameTemplate` semantics).
#[derive(Debug, Clone, Default)]
pub struct AnnotationConfig {
    /// Device ids the recogniser must never place in any block (ALIGN
    /// `DoNotIdentify`). The library resolves user device names to ids.
    pub do_not_identify: HashSet<u32>,
    /// Pattern template names to skip entirely.
    pub do_not_use: HashSet<String>,
    /// Extra nets forced to Supply (name match, case-insensitive); wins over every other rule.
    pub supply_nets: Vec<String>,
    /// Extra nets forced to Ground; loses only to `supply_nets`.
    pub ground_nets: Vec<String>,
    /// Extra nets forced to Clock; loses to the rail rules ([`rail_of`] and the two lists above).
    pub clock_nets: Vec<String>,
    /// Process numbers from the deck; `library::annotation` fills them.
    pub process: ProcessNumbers,
    /// 1σ input-referred offset a matched pair may spend, mV (from the
    /// circuit's spec). Sets how much of it placement gradients may take.
    pub offset_sigma_mv: Option<f32>,
    /// Emission tuning numbers ([`crate::policy::Policy`]).
    pub policy: crate::policy::Policy,
    /// Victim guard rings' return net (case-insensitive name). `None`: a
    /// Ground-class net named like `avss`/`vssa`/`agnd` that no aggressor
    /// touches, else no victim rings ([`crate::rings`]).
    pub quiet_ring_net: Option<String>,
    /// Sidecar symmetry seeds (EXT-26), ahead of the recognised ones; ids
    /// `u32::MAX − entry`, so they never collide with leaf-index seeds.
    pub seeds: Vec<crate::symmetry::Seed>,
    /// Sidecar `SymmetricBlocks` direction: the axis of a single compound.
    pub symmetry_dir: Option<analog::intent::AxisDir>,
    /// Sidecar `GroupBlocks`: (entry index, members) kept together (ProxBlock,
    /// Proximity); the batches carry `Origin::User { index }`.
    pub groups: Vec<(u32, Vec<pnr_core::ids::DeviceId>)>,
    /// Sidecar `Match`: the class (and optionally kind) of the set holding these devices.
    pub classes: Vec<(Vec<pnr_core::ids::DeviceId>, analog::intent::MatchClass, Option<analog::intent::MatchKind>)>,
    /// Sidecar `NetClass`: overrides with `User` evidence.
    pub net_classes: Vec<(pnr_core::ids::NetId, analog::metadata::NetClass)>,
    /// Sidecar `OffsetBudget`: 1σ offset, mV, of the set holding these devices.
    pub offset_budgets: Vec<(Vec<pnr_core::ids::DeviceId>, f32)>,
    /// Sidecar `Load`: external load per net, aF, added to its gate load (AA-25).
    pub loads: Vec<(pnr_core::ids::NetId, f32)>,
    /// Sidecar `IsolatedTub` (GAP-14): NMOS members drawn in one deep-n-well tub, its ring tied to the net.
    /// User-declared only: the sources give no automatic threshold for when a tub pays.
    pub tubs: Vec<(Vec<pnr_core::ids::DeviceId>, pnr_core::ids::NetId)>,
    /// Sidecar `Kelvin` requests, appended to the extracted ones.
    pub kelvins: Vec<analog::intent::KelvinReq>,
    /// Sidecar `Order` (EXT-28): placed ahead of the extracted orders, `reversible: false`.
    pub order: Vec<analog::intent::Order>,
    /// The sidecar parse's diagnostics, carried into `Intent.diagnostics`.
    pub sidecar_diags: Vec<analog::intent::Diagnostic>,
}

/// Every process number the annotator uses. A `None` means the deck does not
/// carry it, and the constraints that need it are not emitted.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessNumbers {
    /// Tightest antenna ratio (metal / gate area) on routed metal.
    pub antenna_max_ratio: Option<f32>,
    /// Gate capacitance per gate area, aF/µm²: the load a net drives.
    pub gate_af_per_um2: Option<f32>,
    /// Ground capacitance of a minimum-width lowest routing wire, aF/µm.
    pub wire_af_per_um: Option<f32>,
    /// Series resistance of a minimum-width lowest routing wire, Ω/µm (sheet
    /// ohms / width; EXT-25's R class reference).
    pub wire_ohm_per_um: Option<f32>,
    /// Lowest routing metal's min spacing, nm; crosstalk spacings are multiples.
    pub route_space_nm: i32,
    /// Deep-trench isolation: (max spacing sharing one trench, trench width), nm.
    pub dti: Option<(i32, i32)>,
    /// Pelgrom `A_VT` (ΔVT of a pair), mV·µm, `[nmos, pmos]`.
    pub avt_mv_um: [Option<f32>; 2],
    /// Current-factor mismatch `A_β` (Δβ/β of a pair), %·µm, `[nmos, pmos]`:
    /// with a mirror's `g_m/I` it puts the ledger in % (MAT-09).
    pub abeta_pct_um: [Option<f32>; 2],
    /// Pelgrom distance coefficient `S_VT`, µV/µm. Process-specific and rarely
    /// published: absent leaves the matching distance check unknown.
    pub svt_uv_per_um: Option<f32>,
    /// `S_VT² = a + b/L²` fit `(a µV²/µm², b µV²)`: with a set's gate L it
    /// replaces `svt_uv_per_um` (MAT-16).
    pub svt_fit: Option<(f32, f32)>,
    /// |dVT/dT|, µV/K, `[nmos, pmos]`: turns a matched pair's offset allowance
    /// into a ΔT limit.
    pub vt_tc_uv_per_k: [Option<f32>; 2],
    /// BSIM4 LOD `KVTH0` (ΔVT per unit `Δ(1/SA + 1/SB)`), mV·µm, `[nmos,
    /// pmos]`: prices LOD imbalance across a matched array.
    pub lod_kvth0_mv_um: [Option<f32>; 2],
    /// Deck `bjt_ka_pct_um`: bipolar/diode area constant `k_A` (ΔI_S/I_S of a
    /// pair), %·µm (MAT-10).
    pub bjt_ka_pct_um: Option<f32>,
    /// Deck `vbe_tc_uv_per_k`: bipolar/diode `|dV_BE/dT|`, µV/K (MAT-10).
    pub vbe_tc_uv_per_k: Option<f32>,
    /// Cut lattice, nm (coincidence tolerance is half of it); 0 = unknown.
    pub lattice_nm: i32,
    /// What the active area sits on; only `EpiOnLowRes` gives isolation a
    /// calibrated distance.
    pub substrate: pnr_core::SubstrateKind,
    /// Epitaxial layer thickness, nm: on `EpiOnLowRes` substrate isolation
    /// saturates at a few times it. Unread on any other kind.
    pub epi_nm: Option<i32>,
    /// The routing stack's per-layer parasitics and antenna stages; `None`
    /// leaves the routing budgets on drawn length and the cumulative antenna.
    pub stack: Option<&'static analog::routing::Stack>,
    /// Deck `min_guard_ring_width`, nm; `0` = none stated.
    pub min_ring_width_nm: i32,
    /// Sidecar `ecgr_min_width_nm`: an electron-collecting ring's width for
    /// a stated collection efficiency; `None` on every shipped deck.
    pub ecgr_min_width_nm: Option<i32>,
    /// The process can draw an `Ecgr` (`cells::post_cell::drawable`).
    pub ecgr_drawable: bool,
    /// The process can draw an `Hcgr` (`cells::post_cell::drawable`).
    pub hcgr_drawable: bool,
    /// The process can draw a `Tub` (deep n-well; `cells::post_cell::drawable`).
    pub tub_drawable: bool,
    /// `Config.op` temperature, K (not a deck key): a mirror's mobility term (MAT-14).
    pub die_temp_k: Option<f32>,
    /// Unitization bounds (EXT-15); 0 = deck key missing.
    pub unit: crate::sets::UnitDeck,
}

#[cfg(test)]
mod tests {
    use super::{classify_nets, rail_of, AnnotationConfig, NetRole};
    use pnr_core::ids::NetId;
    use pnr_core::netlist::{Device, DeviceKind, Net, Netlist};
    use pnr_core::BipartiteHypergraph;

    #[test]
    fn rail_names() {
        use NetRole::{Ground as G, Supply as S};
        let names = ["0", "gnd!", "vdd!", "VSS!", "avdd3v3", "vdd1", "vdd1p8", "VPWR", "vgnd", "VPB", "VNB", "vee", "vbias", "vdd_half"];
        let want = [Some(G), Some(G), Some(S), Some(G), Some(S), Some(S), Some(S), Some(S), Some(G), Some(S), Some(G), Some(G), None, Some(S)];
        for (n, w) in names.iter().zip(want) {
            assert_eq!(rail_of(n), w, "{n}");
        }
    }

    /// Roles of nets `names` under FETs given as `(kind, [g, d, s, b])` net ids.
    fn roles(names: &[&str], fets: &[(DeviceKind, [u16; 4])]) -> Vec<NetRole> {
        let devices = fets
            .iter()
            .enumerate()
            .map(|(i, &(kind, t))| Device {
                name: format!("M{i}"),
                kind,
                model: String::new(),
                terminals: ["G", "D", "S", "B"].iter().zip(t).map(|(n, id)| ((*n).into(), NetId(id))).collect(),
                params: vec![],
            })
            .collect();
        let nl = Netlist { devices, nets: names.iter().map(|n| Net { name: (*n).into() }).collect(), ..Default::default() };
        classify_nets(&BipartiteHypergraph::from_netlist(&nl), &AnnotationConfig::default())
    }

    #[test]
    fn bulk_inference() {
        use DeviceKind::{Nmos as N, Pmos as P};
        use NetRole::{Ground as G, Signal as Sig, Supply as S};
        // 2 PMOS bulk on `a`, 2 NMOS bulk on `c`, gates on `b`.
        let fets = [(P, [1, 1, 0, 0]), (P, [1, 1, 0, 0]), (N, [1, 1, 2, 2]), (N, [1, 1, 2, 2])];
        assert_eq!(roles(&["a", "b", "c"], &fets), [S, Sig, G]);
        assert_eq!(roles(&["a", "b", "c", "VDD"], &fets), [Sig, Sig, G, S]);
        // Top PMOS bulk `a` (2) also drives a dummy's gate (G=S=B=a): no inference,
        // and the runner-up `d` (B=S, 1) must not be promoted.
        let gated = [(P, [0, 1, 0, 0]), (P, [1, 2, 0, 0]), (P, [1, 2, 3, 3])];
        assert_eq!(roles(&["a", "b", "c", "d"], &gated), [Sig; 4]);
        // Equal bulk counts on `a` and `d`: the lowest net id wins.
        let tie = [(P, [1, 2, 0, 0]), (P, [1, 2, 3, 3])];
        assert_eq!(roles(&["a", "b", "c", "d"], &tie), [S, Sig, Sig, Sig]);
    }

    #[test]
    fn clock_names() {
        for n in ["clk", "clk_in", "phi1", "phi_2b", "ck", "ckb", "sysclock"] {
            assert!(super::is_clock(n), "{n}");
        }
        for n in ["back", "stack", "phase", "lock", "vbias"] {
            assert!(!super::is_clock(n), "{n}");
        }
    }
}
