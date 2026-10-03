//! Shared FET recognisers and terminal positions.

use pnr_core::{BipartiteHypergraph, DeviceKind};

// FET terminal positions in `device_nets` (G, D, S, B).
pub(crate) const G: usize = 0;
pub(crate) const D: usize = 1;
pub(crate) const S: usize = 2;

pub(crate) fn is_fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
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
