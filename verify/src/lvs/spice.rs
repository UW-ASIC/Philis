//! SPICE netlist export from extracted layout + PEX parasitics.
//!
//! Combines LVS extraction (devices + connectivity) with PEX (per-net R/C) into a
//! simulatable SPICE subcircuit. Two modes:
//!   - **Schematic-only**: just the extracted devices (MOS, R, C, diodes).
//!   - **With parasitics**: devices + lumped per-net parasitic R and C elements.

use super::types::*;
use crate::pex::NetParasitics;
use std::collections::HashMap;
use std::fmt::Write;

/// Which nets are ports (externally visible pins). Maps net_id → port name.
pub type PortMap = HashMap<u32, String>;

pub struct SpiceOpts {
    pub cell_name: String,
    pub ports: PortMap,
    /// When true, include parasitic R/C from PEX. When false, schematic-only.
    pub include_parasitics: bool,
}

/// Generate a SPICE subcircuit string from extracted netlist + optional parasitics.
pub fn to_spice(
    ext: &ExtractedNetlist,
    parasitics: Option<&HashMap<u32, NetParasitics>>,
    opts: &SpiceOpts,
) -> String {
    let mut out = String::new();

    let net_name = |id: u32| -> String {
        if let Some(name) = opts.ports.get(&id) {
            name.clone()
        } else {
            format!("n{}", id)
        }
    };

    // Header
    let port_list: String = {
        let mut ports: Vec<(&u32, &String)> = opts.ports.iter().collect();
        ports.sort_by_key(|(id, _)| *id);
        ports.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>().join(" ")
    };
    let _ = writeln!(out, ".subckt {} {}", opts.cell_name, port_list);

    // MOS devices
    let mut mi = 0u32;
    for d in &ext.devices {
        let prefix = match d.kind { DeviceKind::Nmos => "Mn", DeviceKind::Pmos => "Mp", DeviceKind::Npn | DeviceKind::Pnp => continue };
        let model = match (&d.kind, &d.flavor) {
            (DeviceKind::Nmos, DeviceFlavor::Standard) => "nmos",
            (DeviceKind::Nmos, DeviceFlavor::Lvt) => "nmos_lvt",
            (DeviceKind::Nmos, DeviceFlavor::Hvt) => "nmos_hvt",
            (DeviceKind::Pmos, DeviceFlavor::Standard) => "pmos",
            (DeviceKind::Pmos, DeviceFlavor::Lvt) => "pmos_lvt",
            (DeviceKind::Pmos, DeviceFlavor::Hvt) => "pmos_hvt",
            (DeviceKind::Npn, _) | (DeviceKind::Pnp, _) => continue,
        };
        let _ = writeln!(out, "{}{} {} {} {} {} {} w={}n l={}n",
            prefix, mi,
            net_name(d.drain), net_name(d.gate), net_name(d.source), net_name(d.body),
            model, d.w, d.l,
        );
        mi += 1;
    }

    // Two-terminal devices
    let mut ri = 0u32;
    let mut ci = 0u32;
    let mut di = 0u32;
    for d in &ext.two_terminal {
        match d.kind {
            TwoTerminalKind::Resistor => {
                let _ = writeln!(out, "R{} {} {} {:.6}", ri, net_name(d.terminal_a), net_name(d.terminal_b), d.value);
                ri += 1;
            }
            TwoTerminalKind::Capacitor => {
                let _ = writeln!(out, "C{} {} {} {:.6}f", ci, net_name(d.terminal_a), net_name(d.terminal_b), d.value);
                ci += 1;
            }
            TwoTerminalKind::Diode => {
                let _ = writeln!(out, "D{} {} {} diode", di, net_name(d.terminal_a), net_name(d.terminal_b));
                di += 1;
            }
        }
    }

    // Parasitic R/C (lumped per net)
    if opts.include_parasitics {
        if let Some(pex) = parasitics {
            let mut pex_sorted: Vec<(&u32, &NetParasitics)> = pex.iter().collect();
            pex_sorted.sort_by_key(|(id, _)| *id);
            let mut pri = 0u32;
            let mut pci = 0u32;
            for (&net_id, np) in pex_sorted {
                if net_id == u32::MAX { continue; }
                let nn = net_name(net_id);
                if np.r_ohm > 0.0 {
                    // ponytail: lumped R inserted as net → net_par node. Distributed RC
                    // ladder when accuracy demands it.
                    let par_node = format!("{}_par", nn);
                    let _ = writeln!(out, "Rpar{} {} {} {:.6}", pri, nn, par_node, np.r_ohm);
                    pri += 1;
                }
                if np.cap_af > 0.0 {
                    let _ = writeln!(out, "Cpar{} {} 0 {:.6}f", pci, nn, np.cap_af);
                    pci += 1;
                }
            }
        }
    }

    let _ = writeln!(out, ".ends {}", opts.cell_name);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spice_inverter() {
        let ext = ExtractedNetlist {
            devices: vec![
                Device { kind: DeviceKind::Pmos, gate: 0, source: 1, drain: 2, body: 1,
                         flavor: DeviceFlavor::Standard, w: 1000, l: 180, device_class: None },
                Device { kind: DeviceKind::Nmos, gate: 0, source: 3, drain: 2, body: 3,
                         flavor: DeviceFlavor::Standard, w: 500, l: 180, device_class: None },
            ],
            net_count: 4, used_nets: 4, net_of_poly: Vec::new(),
            label_conflicts: Vec::new(), two_terminal: Vec::new(),
            bjt_devices: Vec::new(), floating_nets: Vec::new(),
        };
        let mut ports = PortMap::new();
        ports.insert(0, "A".into());
        ports.insert(1, "VDD".into());
        ports.insert(2, "Y".into());
        ports.insert(3, "VSS".into());

        let spice = to_spice(&ext, None, &SpiceOpts {
            cell_name: "inv".into(), ports: ports.clone(), include_parasitics: false,
        });
        assert!(spice.contains(".subckt inv"));
        assert!(spice.contains("Mp0 Y A VDD VDD pmos w=1000n l=180n"));
        assert!(spice.contains("Mn1 Y A VSS VSS nmos w=500n l=180n"));
        assert!(spice.contains(".ends inv"));

        // with parasitics
        let mut pex = HashMap::new();
        pex.insert(2, NetParasitics { r_ohm: 0.5, cap_af: 100.0 });
        let spice = to_spice(&ext, Some(&pex), &SpiceOpts {
            cell_name: "inv".into(), ports, include_parasitics: true,
        });
        assert!(spice.contains("Rpar0 Y Y_par 0.5"));
        assert!(spice.contains("Cpar0 Y 0 100.0"));
    }
}
