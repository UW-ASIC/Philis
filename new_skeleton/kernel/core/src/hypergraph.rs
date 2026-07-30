//! The netlist as a **bipartite device↔net hypergraph** — the structure each
//! analog rule's `extract` walks to decide where it applies.
//!
//! # What the graph is
//!
//! Bipartite incidence: partition A = devices, partition B = nets. Each device
//! *terminal* is one edge `(device, net)`. Both adjacency directions are
//! materialized as parallel SoA arrays so an `extract` sweeps them linearly:
//!
//! * device → nets: [`BipartiteHypergraph::device_nets`], indexed by [`DeviceId`],
//!   inner `Vec` in **terminal order** (see below);
//! * net → devices: [`BipartiteHypergraph::net_devices`], indexed by [`NetId`].
//!
//! [`BipartiteHypergraph::kinds`] is parallel to `device_nets` so a rule filters
//! by device type, and [`BipartiteHypergraph::terminals`] carries the pin *names*
//! parallel to each `device_nets` row — the connectivity the real backend graph
//! (`backend/cells/src/netlist.rs`, `CellNode::pins`) keeps but which positional
//! recognition alone can't recover.
//!
//! # Terminal-order convention
//!
//! Terminal nets keep the order they appear in [`Device::terminals`], which the
//! `cells` MOSFET generator fixes as **G, D, S, B = indices 0, 1, 2, 3**. Every
//! analog recogniser keys off this position (`matching_pair::{G,D,S,B}`): e.g. a
//! diode-connected FET is `device_nets[d][G] == device_nets[d][D]`, a mirror pair
//! shares `device_nets[a][G] == device_nets[b][G]`. Only FETs carry this ordering;
//! two-terminal devices (R/C/L, diode) just list their two nets in netlist order.
//!
//! # How a rule's `extract` walks it
//!
//! A [`crate::BipartiteHypergraph`] plus a `UnionFind` is all a rule gets. Typical
//! sweep: iterate `device_nets.iter().enumerate()`, gate on `kinds[d]`, read the
//! terminal nets by the G/D/S/B indices, then hop to co-incident devices via
//! `net_devices[net.0 as usize]`. `terminals[d]` recovers a pin's name when a rule
//! needs more than position (e.g. distinguishing bulk taps).
//!
//! Migrated from `backend/cells/src/netlist.rs` (`BipartiteHypergraph`): same
//! device↔net incidence and ordered-pin semantics, kept std-only for `pnr_core`.

use crate::ids::{DeviceId, NetId};
use crate::netlist::{DeviceKind, Netlist};

/// Device↔net incidence, in CSR-ish parallel arrays so an `extract` can sweep it
/// linearly. See the [module docs](self) for the terminal-order convention.
pub struct BipartiteHypergraph {
    /// Nets each device connects to, in terminal order (G,D,S,B for FETs),
    /// indexed by [`DeviceId`].
    pub device_nets: Vec<Vec<NetId>>,
    /// Devices incident on each net, indexed by [`NetId`].
    pub net_devices: Vec<Vec<DeviceId>>,
    /// Device kind, parallel to `device_nets` — lets a rule filter by type.
    pub kinds: Vec<DeviceKind>,
    /// Pin names parallel to each `device_nets` row (`terminals[d][t]` names
    /// `device_nets[d][t]`). The real backend graph keeps these on `CellNode::pins`;
    /// positional recognition drops them, so they're carried here for rules that
    /// need to classify a terminal by name rather than index.
    pub terminals: Vec<Vec<String>>,
    /// Net name by [`NetId`] — lets a rule classify a net (supply rail, ground,
    /// well domain) by name rather than only by connectivity. The structural
    /// graph otherwise drops names; carried here for the same reason as
    /// [`Self::terminals`].
    pub net_names: Vec<String>,
}

/// Build the device↔net bipartite hypergraph from a [`Netlist`].
///
/// The frontend calls `netlist.into_bipartite_hypergraph()`. It's a trait (not an
/// inherent method) so the graph type in `pnr_core` doesn't have to know every
/// producer, and so callers can `use` the conversion where they need it.
pub trait IntoBipartiteHypergraph {
    /// Build the hypergraph. Pure; borrows the netlist.
    fn into_bipartite_hypergraph(&self) -> BipartiteHypergraph;
}

impl IntoBipartiteHypergraph for Netlist {
    fn into_bipartite_hypergraph(&self) -> BipartiteHypergraph {
        let mut device_nets = Vec::with_capacity(self.devices.len());
        let mut terminals = Vec::with_capacity(self.devices.len());
        let mut kinds = Vec::with_capacity(self.devices.len());
        let mut net_devices = vec![Vec::new(); self.nets.len()];
        for (di, dev) in self.devices.iter().enumerate() {
            let did = DeviceId(di as u16);
            let mut nets = Vec::with_capacity(dev.terminals.len());
            let mut names = Vec::with_capacity(dev.terminals.len());
            // Preserve `Device::terminals` order — this is the G,D,S,B convention.
            for (pin, net) in &dev.terminals {
                net_devices[net.0 as usize].push(did);
                nets.push(*net);
                names.push(pin.clone());
            }
            device_nets.push(nets);
            terminals.push(names);
            kinds.push(dev.kind);
        }
        let net_names = self.nets.iter().map(|n| n.name.clone()).collect();
        BipartiteHypergraph { device_nets, net_devices, kinds, terminals, net_names }
    }
}

impl BipartiteHypergraph {
    /// Build the hypergraph from a [`Netlist`]. Pure. Kept for callers that
    /// don't want to import [`IntoBipartiteHypergraph`]; delegates to it.
    #[must_use]
    pub fn from_netlist(nl: &Netlist) -> Self {
        nl.into_bipartite_hypergraph()
    }

    /// Number of devices.
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.device_nets.len()
    }
}
