//! The bipartite cell/net hypergraph — the data structure the whole flow
//! passes around. Lives here (below both frontend parsing and the backend)
//! so pnr-placement/pnr-routing can consume it without depending on the
//! SPICE parser in pnr-core, and pnr-core can re-export the backend.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::device::DeviceRecord;

/// Net index into [`BipartiteHypergraph::nets`].
pub type NetId = u32;

/// One cell node: a leaf PDK device or a substrate3 macro.
#[derive(Debug, Clone)]
pub struct CellNode {
    /// Instance name (e.g. "XM1", "XDP1").
    pub name: String,
    /// SPICE model / subckt name.
    pub model: String,
    /// Ordered (pin name, net) — terminal order for devices, port order for macros.
    pub pins: Vec<(String, NetId)>,
    /// `Some` = leaf device (input to a built-in generator); `None` = macro.
    pub device: Option<DeviceRecord>,
    /// All member device records when this node is a matched group
    /// (interdigitated pair, CC group). Empty for ungrouped cells.
    pub grouped_devices: Vec<DeviceRecord>,
}

/// Bipartite hypergraph (incidence form): node partition A = cells,
/// partition B = nets, and each pin is one edge `(cell, net)`. Both
/// adjacency directions are materialized:
///
/// * cell -> nets: `cells[c].pins` (ordered pin list);
/// * net -> cells: CSR — pins of net `n` are
///   `net_pins[net_start[n] as usize .. net_start[n+1] as usize]`,
///   each entry `(cell index, pin index into cells[cell].pins)`.
#[derive(Debug, Clone, Default)]
pub struct BipartiteHypergraph {
    /// Top subcircuit name.
    pub name: String,
    /// Top subcircuit ports.
    pub ports: Vec<String>,
    pub cells: Vec<CellNode>,
    /// Net id -> name.
    pub nets: Vec<String>,
    pub net_start: Vec<u32>,
    pub net_pins: Vec<(u32, u16)>,
}

impl BipartiteHypergraph {
    pub fn net_id(&self, name: &str) -> Option<NetId> {
        self.nets.iter().position(|n| n == name).map(|i| i as u32)
    }

    pub fn cell_id(&self, name: &str) -> Option<u32> {
        self.cells
            .iter()
            .position(|c| c.name == name)
            .map(|i| i as u32)
    }

    /// Pins on a net: `(cell index, pin index)`.
    pub fn pins_on_net(&self, net: NetId) -> &[(u32, u16)] {
        let (s, e) = (
            self.net_start[net as usize],
            self.net_start[net as usize + 1],
        );
        &self.net_pins[s as usize..e as usize]
    }

    /// Collapse named leaf instances into one macro node (a substrate3
    /// custom cell claiming devices not wrapped in a subckt). Group pins
    /// keep exact connectivity, renamed `{member}.{pin}`; nets internal to
    /// the group stay in the net partition (all their edges now reach the
    /// one node — zero wirelength for placement).
    pub fn group(&mut self, name: &str, model: &str, instances: &[String]) -> Result<u32, String> {
        let mut member_ids = Vec::with_capacity(instances.len());
        for inst in instances {
            let id = self
                .cell_id(inst)
                .ok_or_else(|| format!("group `{name}`: unknown instance `{inst}`"))?;
            member_ids.push(id);
        }

        let mut pins = Vec::new();
        for &id in &member_ids {
            let c = &self.cells[id as usize];
            for (pin, net) in &c.pins {
                pins.push((format!("{}.{pin}", c.name), *net));
            }
        }

        let grouped_devices: Vec<DeviceRecord> = member_ids
            .iter()
            .filter_map(|&id| self.cells[id as usize].device.clone())
            .collect();

        let member_set: HashSet<u32> = member_ids.iter().copied().collect();
        let mut i = 0u32;
        self.cells.retain(|_| {
            let keep = !member_set.contains(&i);
            i += 1;
            keep
        });
        self.cells.push(CellNode {
            name: name.to_string(),
            model: model.to_string(),
            pins,
            device: None,
            grouped_devices,
        });
        self.build_csr();
        Ok(self.cells.len() as u32 - 1)
    }

    /// Apply a substrate3 [`CellRegistry`](crate::CellRegistry): each
    /// custom cell that consumes SPICE instances groups them into one node.
    #[allow(clippy::missing_errors_doc)]
    pub fn group_registry(&mut self, reg: &crate::CellRegistry) -> Result<(), String> {
        for e in reg.entries() {
            if e.instances.is_empty() {
                continue;
            }
            self.group(&e.name, &e.name, &e.instances)?;
        }
        Ok(())
    }

    /// Extract a sub-hypergraph containing only the given cell indices.
    /// Nets that connect to cells outside the subset become ports of the sub-graph.
    /// Returns the sub-hypergraph and a map from old cell indices to new ones.
    pub fn extract_subgraph(&self, cell_indices: &[u32]) -> (BipartiteHypergraph, Vec<(u32, u32)>) {
        let cell_set: HashSet<u32> = cell_indices.iter().copied().collect();

        // Build old-to-new cell index map.
        let mut old_to_new: Vec<(u32, u32)> = Vec::with_capacity(cell_indices.len());
        let mut old_to_new_map: HashMap<u32, u32> = HashMap::new();
        let mut new_cells = Vec::with_capacity(cell_indices.len());
        for (new_idx, &old_idx) in cell_indices.iter().enumerate() {
            old_to_new.push((old_idx, new_idx as u32));
            old_to_new_map.insert(old_idx, new_idx as u32);
            new_cells.push(self.cells[old_idx as usize].clone());
        }

        // Collect all nets referenced by the selected cells.
        let mut referenced_nets: HashSet<u32> = HashSet::new();
        for &old_idx in cell_indices {
            for &(_, net) in &self.cells[old_idx as usize].pins {
                referenced_nets.insert(net);
            }
        }

        // Build old-to-new net index map and identify ports.
        let mut net_old_to_new: HashMap<u32, u32> = HashMap::new();
        let mut new_nets = Vec::new();
        let mut new_ports = Vec::new();
        for &old_net in &referenced_nets {
            let new_net = new_nets.len() as u32;
            net_old_to_new.insert(old_net, new_net);
            new_nets.push(self.nets[old_net as usize].clone());

            // A net becomes a port if it connects to any cell outside the subset.
            let pins = self.pins_on_net(old_net);
            let is_port = pins.iter().any(|&(ci, _)| !cell_set.contains(&ci));
            if is_port {
                new_ports.push(self.nets[old_net as usize].clone());
            }
        }

        // Remap pin net indices in the new cells.
        for cell in &mut new_cells {
            for pin in &mut cell.pins {
                pin.1 = net_old_to_new[&pin.1];
            }
        }

        let mut sub_hg = BipartiteHypergraph {
            name: format!("{}_sub", self.name),
            ports: new_ports,
            cells: new_cells,
            nets: new_nets,
            ..Default::default()
        };
        sub_hg.build_csr();

        (sub_hg, old_to_new)
    }

    /// (Re)build the net -> pins CSR from `cells[*].pins`.
    pub fn build_csr(&mut self) {
        let mut count = vec![0u32; self.nets.len()];
        for c in &self.cells {
            for &(_, n) in &c.pins {
                count[n as usize] += 1;
            }
        }
        self.net_start = Vec::with_capacity(self.nets.len() + 1);
        let mut total = 0u32;
        self.net_start.push(0);
        for c in &count {
            total += c;
            self.net_start.push(total);
        }
        let mut cursor: Vec<u32> = self.net_start[..self.nets.len()].to_vec();
        self.net_pins = vec![(0, 0); total as usize];
        for (ci, c) in self.cells.iter().enumerate() {
            for (pi, &(_, n)) in c.pins.iter().enumerate() {
                self.net_pins[cursor[n as usize] as usize] = (ci as u32, pi as u16);
                cursor[n as usize] += 1;
            }
        }
    }
}

/// Debug dump: every cell with its pins, every net with its members.
impl fmt::Display for BipartiteHypergraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "hypergraph `{}`: {} cells, {} nets, ports [{}]",
            self.name,
            self.cells.len(),
            self.nets.len(),
            self.ports.join(" "),
        )?;
        writeln!(f, "cells:")?;
        for (i, c) in self.cells.iter().enumerate() {
            write!(f, "  [{i}] {} {}", c.name, c.model)?;
            match &c.device {
                Some(d) => write!(
                    f,
                    " {:?} w={} l={} nf={} m={}",
                    d.device_type, d.w, d.l, d.nf, d.multiplier
                )?,
                None => write!(f, " macro")?,
            }
            write!(f, " |")?;
            for (pin, net) in &c.pins {
                write!(f, " {pin}={}", self.nets[*net as usize])?;
            }
            writeln!(f)?;
        }
        writeln!(f, "nets:")?;
        for (n, name) in self.nets.iter().enumerate() {
            write!(f, "  [{n}] {name}:")?;
            for &(ci, pi) in self.pins_on_net(n as u32) {
                let c = &self.cells[ci as usize];
                write!(f, " {}.{}", c.name, c.pins[pi as usize].0)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
