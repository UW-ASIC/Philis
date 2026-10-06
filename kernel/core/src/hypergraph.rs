//! The netlist as a bipartite device↔net incidence graph — what each analog
//! rule's `extract` walks.
//!
//! Terminal order follows [`crate::Device::terminals`]; for FETs that is
//! **G, D, S, B = 0, 1, 2, 3**, and recognisers key off those positions
//! (diode-connected: `device_nets[d][G] == device_nets[d][D]`).

use crate::ids::{DeviceId, NetId};
use crate::netlist::{DeviceKind, Netlist};

/// Device↔net incidence in both directions, plus the per-device and per-net
/// data recognisers key off. Built once per netlist by
/// [`BipartiteHypergraph::from_netlist`]; read-only afterwards.
///
/// Invariant: `device_nets`, `kinds` and `terminals` are device-length;
/// `net_devices` and `net_names` are net-length; `terminals[d]` is parallel to
/// `device_nets[d]`.
pub struct BipartiteHypergraph {
    /// Nets per device, in terminal order, by [`DeviceId`].
    pub device_nets: Vec<Vec<NetId>>,
    /// Devices per net, by [`NetId`], ascending. A device appears once per
    /// terminal on the net (a diode-connected FET lists twice on its gate net).
    pub net_devices: Vec<Vec<DeviceId>>,
    /// Kind per device, parallel to `device_nets`.
    pub kinds: Vec<DeviceKind>,
    /// Pin names parallel to each `device_nets` row.
    pub terminals: Vec<Vec<String>>,
    /// Net name by [`NetId`] (supply/rail classification).
    pub net_names: Vec<String>,
}

impl BipartiteHypergraph {
    /// Builds both incidence directions from `nl`. O(devices + terminals + nets).
    ///
    /// # Panics
    /// If a terminal names a net past `nl.nets`.
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

    /// Number of devices (rows of `device_nets`).
    #[must_use]
    pub fn device_count(&self) -> usize {
        self.device_nets.len()
    }
}
