//! The netlist as a bipartite device↔net incidence graph — what each analog
//! rule's `extract` walks.
//!
//! Terminal order follows [`crate::Device::terminals`]; for FETs that is
//! **G, D, S, B = 0, 1, 2, 3**, and recognisers key off those positions
//! (diode-connected: `device_nets[d][G] == device_nets[d][D]`).

use crate::ids::{DeviceId, NetId};
use crate::netlist::{DeviceKind, Netlist};

pub struct BipartiteHypergraph {
    /// Nets per device, in terminal order, by [`DeviceId`].
    pub device_nets: Vec<Vec<NetId>>,
    /// Devices per net, by [`NetId`].
    pub net_devices: Vec<Vec<DeviceId>>,
    /// Kind per device, parallel to `device_nets`.
    pub kinds: Vec<DeviceKind>,
    /// Pin names parallel to each `device_nets` row.
    pub terminals: Vec<Vec<String>>,
    /// Net name by [`NetId`] (supply/rail classification).
    pub net_names: Vec<String>,
}

/// `netlist.into_bipartite_hypergraph()`; same as [`BipartiteHypergraph::from_netlist`].
pub trait IntoBipartiteHypergraph {
    fn into_bipartite_hypergraph(&self) -> BipartiteHypergraph;
}

impl IntoBipartiteHypergraph for Netlist {
    fn into_bipartite_hypergraph(&self) -> BipartiteHypergraph {
        BipartiteHypergraph::from_netlist(self)
    }
}

impl BipartiteHypergraph {
    #[must_use]
    pub fn from_netlist(nl: &Netlist) -> Self {
        let mut net_devices = vec![Vec::new(); nl.nets.len()];
        for (di, dev) in nl.devices.iter().enumerate() {
            for (_, net) in &dev.terminals {
                net_devices[net.0 as usize].push(DeviceId(di as u16));
            }
        }
        BipartiteHypergraph {
            device_nets: nl.devices.iter().map(|d| d.terminals.iter().map(|t| t.1).collect()).collect(),
            terminals: nl.devices.iter().map(|d| d.terminals.iter().map(|t| t.0.clone()).collect()).collect(),
            kinds: nl.devices.iter().map(|d| d.kind).collect(),
            net_devices,
            net_names: nl.nets.iter().map(|n| n.name.clone()).collect(),
        }
    }

    #[must_use]
    pub fn device_count(&self) -> usize {
        self.device_nets.len()
    }
}
