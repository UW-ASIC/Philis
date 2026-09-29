//! DC operating point via ngspice: per-device power for the thermal rules.
//!
//! The bias comes from [`OpConfig::testbench`] when given; otherwise a mid-rail
//! probe bench is synthesised and [`OpPoint::provenance`] says so — a probe is
//! not a sign-off condition.

use std::path::PathBuf;
use std::process::Command;

use pnr_core::Netlist;

/// Per-device facts from a DC operating point.
pub struct OpPoint {
    /// Dissipation `|Id·Vds|` per device, µW.
    pub power_uw: Vec<i32>,
    /// Drain current per device, µA, signed as ngspice reports it (into the
    /// drain); `None` for a device the simulation did not resolve.
    pub id_ua: Vec<Option<f64>>,
    /// Saturation headroom `|V_DS| − |V_DSsat|` per device, mV: what a series
    /// IR drop may eat before the device leaves saturation. Negative = already
    /// in triode; `None` when unresolved.
    pub headroom_mv: Vec<Option<f64>>,
    /// Transconductance per device, µS; `None` when unresolved.
    pub gm_us: Vec<Option<f64>>,
    /// How the bias was obtained, so a probe bench is never passed off as real.
    pub provenance: String,
    /// Devices the simulation reported.
    pub resolved: usize,
}

impl OpPoint {
    /// DC current each device terminal draws from its net, µA, per device:
    /// FET `D` draws `+Id` (ngspice's drain current flows in), `S` `−Id`, and
    /// the gate and bulk none. A capacitor draws no DC current (every
    /// terminal 0). `None` for a FET the simulation did not resolve and for
    /// every other device (resistor, diode, BJT, inductor: not simulated
    /// per terminal yet — unknown, never zero). A net's terminal currents
    /// sum to what its port supplies (zero without one) — the input the
    /// router's per-branch sums (Lienig & Thiele 2018 eqs. 3.5–3.7) need.
    #[must_use]
    pub fn terminal_ua(&self, netlist: &Netlist) -> Vec<Option<Vec<(String, f64)>>> {
        netlist
            .devices
            .iter()
            .enumerate()
            .map(|(i, dev)| {
                let id = match dev.kind {
                    pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos => self.id_ua.get(i).copied().flatten()?,
                    pnr_core::DeviceKind::Capacitor => 0.0,
                    pnr_core::DeviceKind::Resistor | pnr_core::DeviceKind::Diode | pnr_core::DeviceKind::Npn | pnr_core::DeviceKind::Pnp | pnr_core::DeviceKind::Inductor => return None,
                };
                let draw = |t: &str| match t {
                    "D" => id,
                    "S" => -id,
                    _ => 0.0,
                };
                Some(dev.terminals.iter().map(|(t, _)| (t.clone(), draw(t))).collect())
            })
            .collect()
    }

    /// Per net, the smallest positive saturation headroom among devices whose
    /// source or drain sits on it, mV: the drop that net's wiring may spend
    /// without pushing a device out of saturation. `None` when no saturated
    /// device touches the net.
    #[must_use]
    pub fn net_headroom_mv(&self, netlist: &Netlist) -> Vec<Option<f64>> {
        let mut out: Vec<Option<f64>> = vec![None; netlist.nets.len()];
        for (dev, h) in netlist.devices.iter().zip(&self.headroom_mv) {
            let Some(h) = h.filter(|&h| h > 0.0) else { continue };
            for (t, n) in &dev.terminals {
                if let (true, Some(o)) = (t == "S" || t == "D", out.get_mut(n.0 as usize)) {
                    *o = Some(o.map_or(h, |x: f64| x.min(h)));
                }
            }
        }
        out
    }

    /// Total dissipation, µW.
    #[must_use]
    pub fn total_power_uw(&self) -> i64 {
        self.power_uw.iter().map(|&p| i64::from(p)).sum()
    }
}

/// DC current each net carries, µA, from per-device terminal draws
/// ([`OpPoint::terminal_ua`]): the larger of what its terminals draw and what
/// they supply (a rail's port makes up the difference). `None` when a device
/// on the net is unresolved — unknown, never zero.
#[must_use]
pub fn net_current_ua(netlist: &Netlist, draws: &[Option<Vec<(String, f64)>>]) -> Vec<Option<i32>> {
    let mut acc: Vec<Option<(f64, f64)>> = vec![Some((0.0, 0.0)); netlist.nets.len()];
    for (dev, d) in netlist.devices.iter().zip(draws) {
        for (t, net) in &dev.terminals {
            let Some(slot) = acc.get_mut(net.0 as usize) else { continue };
            let i = d.as_ref().map(|ts| ts.iter().find(|(x, _)| x == t).map_or(0.0, |&(_, i)| i));
            *slot = slot.zip(i).map(|((draw, supply), i)| if i >= 0.0 { (draw + i, supply) } else { (draw, supply - i) });
        }
    }
    acc.into_iter().map(|a| a.map(|(d, s)| d.max(s).round() as i32)).collect()
}

/// How to build and run the operating-point deck.
#[derive(Clone, Debug)]
pub struct OpConfig {
    /// SPICE model library, included as `.lib <path> <corner>`.
    pub model_lib: Option<PathBuf>,
    /// Corner inside `model_lib` (sky130: `tt`, `ff`, `ss`, …).
    pub corner: String,
    /// Testbench appended to the circuit; `None` synthesises a mid-rail probe.
    pub testbench: Option<String>,
    /// Supply voltage of the synthesised bench, volts.
    pub vdd: f64,
    /// NMOS / PMOS library model names overriding the schematic's own
    /// (empty: each device simulates as the model it was drawn as).
    pub nmos_model: String,
    pub pmos_model: String,
    /// ngspice binary.
    pub ngspice: String,
    /// Simulation temperature, °C: the bias is solved at it, and EM limits
    /// rated at a hotter reference are derated to it (never credited cooler).
    pub temp_c: f64,
}

impl Default for OpConfig {
    fn default() -> Self {
        Self {
            model_lib: None,
            corner: "tt".into(),
            testbench: None,
            vdd: 1.8,
            nmos_model: String::new(),
            pmos_model: String::new(),
            ngspice: "ngspice".into(),
            temp_c: 27.0,
        }
    }
}

impl OpConfig {
    /// Library model for a device. Only FETs are simulated.
    fn model_for<'a>(&'a self, dev: &'a pnr_core::Device) -> Option<&'a str> {
        let forced = match dev.kind {
            pnr_core::DeviceKind::Nmos => &self.nmos_model,
            pnr_core::DeviceKind::Pmos => &self.pmos_model,
            _ => return None,
        };
        [forced, &dev.model].into_iter().find(|m| !m.is_empty()).map(String::as_str)
    }
}

/// Solve `netlist`'s DC operating point and map it onto `netlist.devices`.
///
/// # Errors
/// ngspice missing, the deck unwritable, or no device data in the output —
/// all of which the caller treats as "bias unknown".
pub fn extract(netlist: &Netlist, cfg: &OpConfig) -> Result<OpPoint, String> {
    let (deck, provenance) = build_deck(netlist, cfg);
    let dir = std::env::temp_dir().join(format!("philis_op_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("deck io: {e}"))?;
    let path = dir.join("op.spice");
    std::fs::write(&path, &deck).map_err(|e| format!("deck io: {e}"))?;
    let out = Command::new(&cfg.ngspice)
        .arg("-b")
        .arg(&path)
        .output()
        .map_err(|e| format!("ngspice unavailable: {e}"))?;

    let table = parse_show(&String::from_utf8_lossy(&out.stdout));
    if table.is_empty() {
        let tail: String = String::from_utf8_lossy(&out.stderr)
            .chars()
            .take(400)
            .collect();
        return Err(format!("no device operating points: {tail}"));
    }
    let mut power_uw = vec![0i32; netlist.devices.len()];
    let mut id_ua = vec![None; netlist.devices.len()];
    let mut headroom_mv = vec![None; netlist.devices.len()];
    let mut gm_us = vec![None; netlist.devices.len()];
    let mut resolved = 0usize;
    for (i, dev) in netlist.devices.iter().enumerate() {
        let Some(op) = table.get(instance_name(dev).to_ascii_lowercase().as_str()) else {
            continue;
        };
        power_uw[i] = ((op.id * op.vds).abs() * 1e6).round() as i32;
        id_ua[i] = Some(op.id * 1e6);
        headroom_mv[i] = Some((op.vds.abs() - op.vdsat.abs()) * 1e3);
        gm_us[i] = Some(op.gm.abs() * 1e6);
        resolved += 1;
    }
    Ok(OpPoint {
        power_uw,
        id_ua,
        headroom_mv,
        gm_us,
        provenance,
        resolved,
    })
}

/// One device's operating point as `show` reports it.
#[derive(Default, Clone, Copy)]
struct DevOp {
    id: f64,
    vds: f64,
    vdsat: f64,
    gm: f64,
}

/// Assemble the deck: model library, the circuit, a bias bench, and a control
/// block that dumps every MOSFET's operating point at once.
///
/// The circuit is emitted **flat, from the parsed netlist**, not by reusing the
/// input text. A `.subckt` cannot be biased from outside: its internal bias nodes
/// (a mirror's gate rail, a tail bias) are invisible at the top level, so a
/// wrapper testbench leaves them floating and the DC solve has no solution.
/// Flattening puts every node in one scope where it can be driven.
fn build_deck(netlist: &Netlist, cfg: &OpConfig) -> (String, String) {
    let (bench, provenance) = match &cfg.testbench {
        Some(tb) => (tb.clone(), "user testbench".to_string()),
        None => probe_bench(netlist, cfg),
    };
    let body = flat_circuit(netlist, cfg);

    let lib = cfg.model_lib.as_ref().map_or(String::new(), |p| {
        format!(".lib {} {}\n", p.display(), cfg.corner)
    });

    let temp = cfg.temp_c;
    let deck = format!(
        "* Philis operating-point probe (generated)\n\
         {lib}{body}\n\
         {bench}\n\
         .temp {temp}\n\
         .control\n\
         set ngbehavior=hsa\n\
         op\n\
         echo @@PHILIS_OP\n\
         show m : id,vds,vdsat,gm\n\
         echo @@PHILIS_END\n\
         .endc\n\
         .end\n"
    );
    (deck, provenance)
}

/// Emit every device flat, one line each, against the library's model names.
pub(crate) fn flat_circuit(netlist: &Netlist, cfg: &OpConfig) -> String {
    flat_circuit_with(netlist, cfg, |_, _, n| n, |_| String::new())
}

/// [`flat_circuit`] with each terminal's node renamed by `node(device index,
/// terminal, net node)` and `extra(device index)` appended to the card (a
/// post-layout deck's branch resistors and stress parameters).
pub(crate) fn flat_circuit_with(
    netlist: &Netlist,
    cfg: &OpConfig,
    node: impl Fn(usize, &str, String) -> String,
    extra: impl Fn(usize) -> String,
) -> String {
    let mut s = String::new();
    for (di, dev) in netlist.devices.iter().enumerate() {
        let Some(model) = cfg.model_for(dev) else {
            continue; // not a simulatable primitive here (R/C/L handled by their own cards)
        };
        let net = |t: &str| {
            let n = dev.terminals.iter().find(|(n, _)| n == t).map(|(_, id)| node_name(netlist, *id)).unwrap_or_else(|| "0".into());
            node(di, t, n)
        };
        // sky130 primitives are subcircuits: D G S B, then W/L in microns.
        let (w_um, l_um) = (param_um(dev, "w"), param_um(dev, "l"));
        let nf = dev
            .params
            .iter()
            .find(|(k, _)| k == "nf")
            .map_or(1, |(_, v)| *v)
            .max(1);
        s.push_str(&format!(
            "{} {} {} {} {} {} W={w_um} L={l_um} nf={nf}{}\n",
            instance_name(dev),
            net("D"),
            net("G"),
            net("S"),
            net("B"),
            model,
            extra(di)
        ));
    }
    s
}

/// SPICE instance name for a device.
///
/// A subcircuit instance must start with `X`, but netlist names usually already
/// do (`XM1`). Blindly prefixing yields `XXM1`, whose operating point then fails
/// to map back to the device — so prefix only when needed.
fn instance_name(dev: &pnr_core::Device) -> String {
    if dev.name.starts_with('X') || dev.name.starts_with('x') {
        dev.name.clone()
    } else {
        format!("X{}", dev.name)
    }
}

/// Net name, sanitised for SPICE and mapped to node 0 for grounds.
pub(crate) fn node_name(netlist: &Netlist, id: pnr_core::NetId) -> String {
    let raw = netlist
        .nets
        .get(id.0 as usize)
        .map_or_else(|| format!("n{}", id.0), |n| n.name.clone());
    let lower = raw.to_ascii_lowercase();
    if is_ground(&lower) {
        "0".into()
    } else {
        lower.replace(['/', '.', '<', '>'], "_")
    }
}

/// A `w`/`l` parameter in microns (the netlist stores nm).
fn param_um(dev: &pnr_core::Device, key: &str) -> f64 {
    let nm = dev
        .params
        .iter()
        .find(|(k, _)| k == key)
        .map_or(0, |(_, v)| *v);
    if nm <= 0 {
        // A missing dimension would make the device degenerate; fall back to a
        // minimum-ish geometry so the solve still yields a usable bias.
        return if key == "l" { 0.15 } else { 1.0 };
    }
    nm as f64 / 1000.0
}

/// Synthesise a mid-rail probe bench.
///
/// Two classes of node need a source, and both are invisible from a `.subckt`
/// wrapper — which is why the circuit is flattened first:
///
/// * **supplies/grounds**, tied to the rails; and
/// * **gate-only nets** — nets that reach transistor gates but no source or
///   drain. A MOSFET gate draws no DC current, so such a net has no DC path to
///   anything and the solve is singular. These are exactly the circuit's inputs
///   and bias rails (`vinp`, `vbias`, `vbn`), the nodes a real testbench would
///   drive.
///
/// It is a probe, not a sign-off condition, and the returned note says so.
fn probe_bench(netlist: &Netlist, cfg: &OpConfig) -> (String, String) {
    let n_nets = netlist.nets.len();
    let (mut touches_gate, mut touches_channel) = (vec![false; n_nets], vec![false; n_nets]);
    for dev in &netlist.devices {
        for (term, id) in &dev.terminals {
            let i = id.0 as usize;
            if i >= n_nets {
                continue;
            }
            match term.as_str() {
                "G" => touches_gate[i] = true,
                "D" | "S" | "C" | "E" | "A" | "B2" => touches_channel[i] = true,
                _ => {}
            }
        }
    }

    let mut lines = String::new();
    let mut driven = 0usize;
    for (i, net) in netlist.nets.iter().enumerate() {
        let lower = net.name.to_ascii_lowercase();
        if is_ground(&lower) {
            continue; // node 0
        }
        let name = node_name(netlist, pnr_core::NetId(i as u16));
        let v = if is_supply(&lower) {
            cfg.vdd
        } else if touches_gate[i] && !touches_channel[i] {
            // Input or bias rail: no DC path, must be driven.
            cfg.vdd / 2.0
        } else {
            continue; // the circuit drives it
        };
        lines.push_str(&format!("V{name} {name} 0 {v}\n"));
        driven += 1;
    }
    (
        lines,
        format!(
            "SYNTHESISED probe bench — {driven} nodes forced (rails at {}V, gate-only nets at {}V); NOT a sign-off bias",
            cfg.vdd,
            cfg.vdd / 2.0
        ),
    )
}

fn is_ground(n: &str) -> bool {
    matches!(n, "vss" | "gnd" | "vgnd" | "0" | "vssd" | "vssa")
}

fn is_supply(n: &str) -> bool {
    matches!(n, "vdd" | "vcc" | "vpwr" | "vdda" | "vddd")
}

/// Parse ngspice `show m : id,vds` column blocks into `device -> DevOp`.
///
/// The output is column-oriented and one block per model type:
/// ```text
///      device m.xm5.msky130_fd_pr__ m.xm4.msky130_fd_pr__
///          id           8.21938e-06           4.10968e-06
///         vds            0.00628286               1.78826
/// ```
/// Instance names are truncated to a fixed width, which is why the *device*
/// name is taken from the `m.<name>.` prefix rather than the tail.
fn parse_show(text: &str) -> std::collections::HashMap<String, DevOp> {
    let mut out: std::collections::HashMap<String, DevOp> = std::collections::HashMap::new();
    let mut cols: Vec<String> = Vec::new();
    let mut inside = false;

    for line in text.lines() {
        if line.contains("@@PHILIS_OP") {
            inside = true;
            continue;
        }
        if line.contains("@@PHILIS_END") {
            break;
        }
        if !inside {
            continue;
        }
        let t = line.trim();
        let mut it = t.split_whitespace();
        let Some(head) = it.next() else {
            continue;
        };
        let rest: Vec<&str> = it.collect();

        match head {
            "device" => {
                cols = rest.iter().filter_map(|c| instance_device(c)).collect();
            }
            "id" | "vds" | "vdsat" | "gm" => {
                for (ci, raw) in rest.iter().enumerate() {
                    let Some(dev) = cols.get(ci) else { continue };
                    let Ok(v) = raw.parse::<f64>() else { continue };
                    let e = out.entry(dev.clone()).or_default();
                    match head {
                        "id" => e.id = v,
                        "vds" => e.vds = v,
                        "gm" => e.gm = v,
                        _ => e.vdsat = v,
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// `m.xm5.msky130_fd_pr__` → `xm5`.
fn instance_device(col: &str) -> Option<String> {
    let s = col.strip_prefix("m.")?;
    let end = s.find('.')?;
    Some(s[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOW: &str = "\
@@PHILIS_OP
 BSIM4v5: Berkeley Short Channel IGFET Model-4
     device m.xm5.msky130_fd_pr__ m.xm4.msky130_fd_pr__ m.xm3.msky130_fd_pr__
      model xm5:sky130_fd_pr__nfe xm4:sky130_fd_pr__pfe xm3:sky130_fd_pr__pfe
         id           8.21938e-06           4.10968e-06           4.10968e-06
        vds            0.00628286               1.78826               1.78826
        vgs                   0.8                  1.05                  1.05
      vdsat              0.297885             0.0485318             0.0485318
@@PHILIS_END
";

    #[test]
    fn parses_the_column_layout_ngspice_actually_emits() {
        let t = parse_show(SHOW);
        assert_eq!(t.len(), 3, "one entry per device column");
        let m5 = t.get("xm5").expect("xm5 present");
        assert!((m5.id - 8.21938e-06).abs() < 1e-12);
        assert!((m5.vds - 0.00628286).abs() < 1e-9);
        let m3 = t.get("xm3").expect("xm3 present");
        assert!((m3.vds - 1.78826).abs() < 1e-5);
    }

    #[test]
    fn power_separates_the_hot_device_from_the_cold_one() {
        // The physical point: equal currents, wildly unequal dissipation. A
        // triode device carries current at millivolts and heats nothing; the
        // load across the rail is the heat source the placer must work around.
        let t = parse_show(SHOW);
        let p = |d: &str| {
            let o = t[d];
            (o.id * o.vds).abs() * 1e6
        };
        assert!(
            p("xm3") > 7.0 && p("xm3") < 8.0,
            "load ≈7.35µW, got {}",
            p("xm3")
        );
        assert!(p("xm5") < 0.1, "tail in triode ≈0.05µW, got {}", p("xm5"));
        assert!(p("xm3") > 100.0 * p("xm5"), "hot device must dominate");
    }

    #[test]
    fn instance_names_survive_truncation() {
        assert_eq!(
            instance_device("m.xm5.msky130_fd_pr__").as_deref(),
            Some("xm5")
        );
        assert_eq!(instance_device("m.xdut.xm1.mnfet").as_deref(), Some("xdut"));
        assert_eq!(instance_device("notadevice"), None);
    }

    /// Mirror-ish stub: `vbias` reaches only gates (no DC path), `vout` is
    /// driven by a drain, `VDD`/`VSS` are rails.
    fn stub_netlist() -> pnr_core::Netlist {
        use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};
        let nets = ["vdd", "vss", "vbias", "vout"]
            .iter()
            .map(|n| Net {
                name: (*n).to_string(),
            })
            .collect();
        let dev = |name: &str, d: u16, g: u16, s: u16, b: u16| Device {
            name: name.to_string(),
            kind: DeviceKind::Nmos, model: "sky130_fd_pr__nfet_01v8".into(),
            terminals: vec![
                ("D".into(), NetId(d)),
                ("G".into(), NetId(g)),
                ("S".into(), NetId(s)),
                ("B".into(), NetId(b)),
            ],
            params: vec![("w".into(), 10_000), ("l".into(), 1_000)],
        };
        Netlist {
            devices: vec![dev("XM1", 3, 2, 1, 1)],
            nets,
        }
    }

    #[test]
    fn probe_bench_drives_rails_and_gate_only_nets_only() {
        let (bench, note) = probe_bench(&stub_netlist(), &OpConfig::default());
        assert!(
            note.contains("NOT a sign-off bias"),
            "provenance must flag the guess: {note}"
        );
        assert!(
            bench.contains("Vvdd vdd 0 1.8"),
            "supply at the rail: {bench}"
        );
        // `vbias` reaches only a gate — no DC path — so the solve is singular
        // unless it is driven. This is the case a subckt wrapper cannot reach.
        assert!(
            bench.contains("Vvbias vbias 0 0.9"),
            "gate-only bias must be driven: {bench}"
        );
        // `vout` is held by a drain; forcing it would destroy the operating point.
        assert!(
            !bench.contains("Vvout"),
            "circuit-driven node must float: {bench}"
        );
        assert!(!bench.contains("Vvss"), "ground is node 0: {bench}");
    }

    #[test]
    fn flat_circuit_emits_devices_against_library_models() {
        let cfg = OpConfig::default();
        let deck = flat_circuit(&stub_netlist(), &cfg);
        assert!(
            deck.contains("sky130_fd_pr__nfet_01v8"),
            "library model name: {deck}"
        );
        assert!(deck.contains("W=10 L=1"), "nm converted to microns: {deck}");
        assert!(
            deck.starts_with("XM1 vout vbias 0 0"),
            "D G S B order, vss→0: {deck}"
        );
        assert!(
            !deck.starts_with("XXM1"),
            "must not double-prefix an X-name: {deck}"
        );
    }

    /// A diff pair on one tail: each drain draws its half, the tail device's
    /// drain the whole; the terminal currents on the tail net sum to zero
    /// (KCL, no port), and an unresolved device is unknown, never zero.
    #[test]
    fn terminal_currents_balance_on_an_internal_net() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let fet = |name: &str, d: u16, g: u16, s: u16| Device {
            name: name.into(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: vec![("D".into(), NetId(d)), ("G".into(), NetId(g)), ("S".into(), NetId(s)), ("B".into(), NetId(4))],
            params: vec![],
        };
        // nets: 0 outp, 1 outn, 2 tail, 3 in, 4 vss
        let nl = Netlist {
            devices: vec![fet("M1", 0, 3, 2), fet("M2", 1, 3, 2), fet("M5", 2, 3, 4)],
            nets: ["outp", "outn", "tail", "in", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
        };
        let op = |ids: [Option<f64>; 3]| OpPoint { power_uw: vec![0; 3], id_ua: ids.to_vec(), headroom_mv: vec![None; 3], gm_us: vec![None; 3], provenance: String::new(), resolved: 3 };
        let t = op([Some(10.0), Some(10.0), Some(20.0)]).terminal_ua(&nl);
        let on = |net: u16| -> f64 {
            nl.devices
                .iter()
                .zip(&t)
                .flat_map(|(d, c)| d.terminals.iter().zip(c.as_ref().unwrap()).filter(move |((_, n), _)| n.0 == net).map(|(_, (_, i))| *i))
                .sum()
        };
        assert_eq!(on(2), 0.0, "tail: two sources out, the tail drain in");
        assert_eq!(t[2].as_ref().unwrap()[0], ("D".to_string(), 20.0));
        assert_eq!(on(3), 0.0, "a gate net draws no DC current");
        assert!(op([Some(10.0), None, Some(20.0)]).terminal_ua(&nl)[1].is_none(), "unknown, never zero");
    }

    /// A resistor's DC current is not simulated per terminal yet: unknown, and
    /// its net's current with it. A capacitor carries no DC current: every
    /// terminal a known 0, and its net stays known.
    #[test]
    fn a_resistor_current_is_unknown_and_a_capacitor_is_zero() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let two = |name: &str, kind, a: u16, b: u16| Device {
            name: name.into(),
            kind,
            model: String::new(),
            terminals: vec![("P".into(), NetId(a)), ("N".into(), NetId(b))],
            params: vec![],
        };
        // nets: 0 x, 1 y, 2 vss
        let nl = Netlist {
            devices: vec![two("R1", DeviceKind::Resistor, 0, 2), two("C1", DeviceKind::Capacitor, 1, 2)],
            nets: ["x", "y", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
        };
        let op = OpPoint { power_uw: vec![0; 2], id_ua: vec![None; 2], headroom_mv: vec![None; 2], gm_us: vec![None; 2], provenance: String::new(), resolved: 0 };
        let t = op.terminal_ua(&nl);
        assert!(t[0].is_none(), "resistor: unknown, never zero");
        assert_eq!(t[1], Some(vec![("P".to_string(), 0.0), ("N".to_string(), 0.0)]));
        let i = net_current_ua(&nl, &t);
        assert_eq!((i[0], i[1]), (None, Some(0)), "the resistor's net unknown, the capacitor's known");
    }

    /// Headroom is `|V_DS| − V_DSsat`: a triode tail has none to spend, and a
    /// net takes the tightest saturated device on it.
    #[test]
    fn headroom_reads_vdsat_and_a_net_takes_its_tightest_device() {
        let t = parse_show(SHOW);
        assert!((t["xm3"].vdsat - 0.0485318).abs() < 1e-9);
        let nl = stub_netlist(); // XM1: D=vout(3) S=vss(1)
        let op = |h: Option<f64>| OpPoint { power_uw: vec![0], id_ua: vec![Some(1.0)], headroom_mv: vec![h], gm_us: vec![None], provenance: String::new(), resolved: 1 };
        let hr = op(Some(250.0)).net_headroom_mv(&nl);
        assert_eq!((hr[3], hr[1], hr[2]), (Some(250.0), Some(250.0), None), "drain and source nets, not the gate");
        assert_eq!(op(Some(-3.0)).net_headroom_mv(&nl)[3], None, "a triode device has no headroom to give");
    }

    /// A tail net: two sources supply 10 µA each, the tail drain draws 20 µA —
    /// the net carries 20; an unresolved device leaves its nets unknown.
    #[test]
    fn a_net_carries_the_larger_of_its_draw_and_supply() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let fet = |name: &str, d: u16, s: u16| Device {
            name: name.into(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: vec![("D".into(), NetId(d)), ("G".into(), NetId(3)), ("S".into(), NetId(s)), ("B".into(), NetId(4))],
            params: vec![],
        };
        let nl = Netlist {
            devices: vec![fet("M1", 0, 2), fet("M2", 1, 2), fet("M5", 2, 4)],
            nets: ["outp", "outn", "tail", "in", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
        };
        let op = |ids: [Option<f64>; 3]| OpPoint { power_uw: vec![0; 3], id_ua: ids.to_vec(), headroom_mv: vec![None; 3], gm_us: vec![None; 3], provenance: String::new(), resolved: 3 };
        let i = net_current_ua(&nl, &op([Some(10.0), Some(10.0), Some(20.0)]).terminal_ua(&nl));
        assert_eq!((i[2], i[4], i[3]), (Some(20), Some(20), Some(0)), "tail, the ground return, a gate net");
        let u = net_current_ua(&nl, &op([Some(10.0), None, Some(20.0)]).terminal_ua(&nl));
        assert_eq!((u[1], u[2]), (None, None), "M2 unresolved: its nets are unknown");
    }
}
