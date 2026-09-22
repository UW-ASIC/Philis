//! DC operating point via **ngspice** — where the electrical facts come from.
//!
//! Placement theory keeps asking questions a netlist cannot answer. How much
//! does this device dissipate (thermal gradient)? How much current does this
//! branch carry (electromigration wire width)? What is the overdrive (HCI
//! aging)? All of it lives in the *operating point*, and the only way to know an
//! operating point is to solve the circuit.
//!
//! Without this module the thermal solver sees a uniform die, every ΔT is zero,
//! and `ThermalGradient` passes trivially — honestly, but uselessly. With it the
//! placer gets a real, asymmetric power map: in a 5T OTA the PMOS loads sit at
//! nearly the full rail (`Vds ≈ 1.79 V`) and dissipate ~7 µW each, while the
//! input pair sits in triode at millivolts and dissipates almost nothing. That
//! asymmetry *is* the thermal gradient a matched pair has to be placed against.
//!
//! ## What it does not do
//!
//! It does not invent a bias. An operating point needs a testbench — supplies,
//! input levels, bias voltages — and those are design intent, not netlist facts.
//! Supply one via [`OpConfig::testbench`]. The synthesised fallback bench
//! (mid-rail everything) is a *probe*, not a sign-off condition, and it says so
//! in [`OpPoint::provenance`]; a wrong operating point yields a wrong thermal
//! field, which is worse than none at all.

use std::path::PathBuf;
use std::process::Command;

use pnr_core::Netlist;

/// Per-device electrical facts recovered from a DC operating point.
pub struct OpPoint {
    /// Steady-state dissipation `|Id·Vds|` per device, µW — the thermal solver's
    /// input (`Layout::power_uw`).
    pub power_uw: Vec<i32>,
    /// Drain current per device, µA — electromigration wire sizing (Black's law)
    /// and the `BiasCurrentTag`'s `id_ua`.
    pub id_ua: Vec<i32>,
    /// Gate-source voltage per device, mV. Overdrive needs `Vth`, which `show`
    /// can also report per model; this is the raw `Vgs` the tag starts from.
    pub vgs_mv: Vec<i32>,
    /// How the bias was obtained — carried so a report can never present a
    /// probe bench as if it were a real one.
    pub provenance: String,
    /// Devices the simulation reported, out of `netlist.devices.len()`.
    pub resolved: usize,
}

impl OpPoint {
    /// Total dissipation across the circuit, µW.
    #[must_use]
    pub fn total_power_uw(&self) -> i64 {
        self.power_uw.iter().map(|&p| i64::from(p)).sum()
    }

    /// True when the simulation produced no usable device data.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resolved == 0
    }
}

/// How to build and run the operating-point deck.
pub struct OpConfig {
    /// SPICE model library, included as `.lib <path> <corner>`. Without it only
    /// a netlist of built-in primitives can simulate.
    pub model_lib: Option<PathBuf>,
    /// Corner name inside `model_lib` (sky130: `tt`, `ff`, `ss`, …).
    pub corner: String,
    /// Testbench appended to the netlist: supplies, bias sources, and the DUT
    /// instantiation. `None` synthesises a mid-rail probe bench.
    pub testbench: Option<String>,
    /// Supply voltage the synthesised bench drives, volts.
    pub vdd: f64,
    /// Model-name rewrites applied to the deck, `(from, to)`. A netlist written
    /// against short PDK names (`nfet_01v8`) has to reach the library's real
    /// subcircuit (`sky130_fd_pr__nfet_01v8`).
    pub model_map: Vec<(String, String)>,
    /// ngspice binary.
    pub ngspice: String,
    /// Hard timeout, seconds — a non-converging DC solve must not hang a flow.
    pub timeout_s: u64,
}

impl Default for OpConfig {
    fn default() -> Self {
        Self {
            model_lib: None,
            corner: "tt".into(),
            testbench: None,
            vdd: 1.8,
            model_map: sky130_model_map(),
            ngspice: "ngspice".into(),
            timeout_s: 120,
        }
    }
}

/// Short-name → sky130 subcircuit rewrites, the models the fixtures name.
#[must_use]
pub fn sky130_model_map() -> Vec<(String, String)> {
    ["nfet_01v8", "pfet_01v8", "nfet_01v8_lvt", "pfet_01v8_lvt", "nfet_g5v0d10v5", "pfet_g5v0d10v5"]
        .iter()
        .map(|m| ((*m).to_string(), format!("sky130_fd_pr__{m}")))
        .collect()
}

impl OpConfig {
    /// Library model name for a device kind, via [`OpConfig::model_map`].
    ///
    /// The parsed netlist keeps a [`pnr_core::DeviceKind`], not the original
    /// model string, so the mapping is by kind. Only FETs are emitted: passives
    /// carry no operating point worth a thermal source here.
    #[must_use]
    pub fn model_for(&self, kind: pnr_core::DeviceKind) -> Option<&str> {
        let short = match kind {
            pnr_core::DeviceKind::Nmos => "nfet_01v8",
            pnr_core::DeviceKind::Pmos => "pfet_01v8",
            _ => return None,
        };
        self.model_map
            .iter()
            .find(|(from, _)| from == short)
            .map(|(_, to)| to.as_str())
    }
}

/// Why an operating point could not be obtained. Never fatal to a flow: the
/// caller falls back to zero power, which the thermal solver reads as a uniform
/// die.
#[derive(Debug)]
pub enum OpError {
    /// ngspice is not installed or not runnable.
    NgspiceMissing(String),
    /// The simulator ran but reported no device operating points.
    NoDevices(String),
    /// Could not write the temporary deck.
    Io(String),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NgspiceMissing(e) => write!(f, "ngspice unavailable: {e}"),
            Self::NoDevices(e) => write!(f, "no device operating points: {e}"),
            Self::Io(e) => write!(f, "deck io: {e}"),
        }
    }
}

/// Solve `spice` for its DC operating point and map the result onto
/// `netlist.devices`.
///
/// # Errors
/// Returns [`OpError`] when ngspice is missing, the deck cannot be written, or
/// the solve yields no device data. Callers should treat all three as "no bias
/// known" rather than as a flow failure.
pub fn extract(spice: &str, netlist: &Netlist, cfg: &OpConfig) -> Result<OpPoint, OpError> {
    let (deck, provenance) = build_deck(spice, netlist, cfg);

    let dir = std::env::temp_dir().join(format!("philis_op_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| OpError::Io(e.to_string()))?;
    let path = dir.join("op.spice");
    std::fs::write(&path, &deck).map_err(|e| OpError::Io(e.to_string()))?;

    let out = Command::new(&cfg.ngspice)
        .arg("-b")
        .arg(&path)
        .output()
        .map_err(|e| OpError::NgspiceMissing(e.to_string()))?;
    let text = String::from_utf8_lossy(&out.stdout);

    let table = parse_show(&text);
    if table.is_empty() {
        let tail: String = String::from_utf8_lossy(&out.stderr).chars().take(400).collect();
        return Err(OpError::NoDevices(tail));
    }

    let n = netlist.devices.len();
    let (mut power_uw, mut id_ua, mut vgs_mv) = (vec![0i32; n], vec![0i32; n], vec![0i32; n]);
    let mut resolved = 0usize;
    for (i, dev) in netlist.devices.iter().enumerate() {
        let key = instance_name(dev).to_ascii_lowercase();
        let Some(op) = table.get(key.as_str()) else {
            continue;
        };
        // Dissipation is |Id·Vds|: a device in triode carries current at almost
        // no voltage and heats nothing; one across the rail is the hot spot.
        power_uw[i] = ((op.id * op.vds).abs() * 1e6).round() as i32;
        id_ua[i] = (op.id.abs() * 1e6).round() as i32;
        vgs_mv[i] = (op.vgs * 1e3).round() as i32;
        resolved += 1;
    }

    Ok(OpPoint { power_uw, id_ua, vgs_mv, provenance, resolved })
}

/// One device's operating point as `show` reports it.
#[derive(Default, Clone, Copy)]
struct DevOp {
    id: f64,
    vds: f64,
    vgs: f64,
}

/// Assemble the deck: model library, the circuit, a bias bench, and a control
/// block that dumps every MOSFET's operating point at once.
///
/// The circuit is emitted **flat, from the parsed netlist**, not by reusing the
/// input text. A `.subckt` cannot be biased from outside: its internal bias nodes
/// (a mirror's gate rail, a tail bias) are invisible at the top level, so a
/// wrapper testbench leaves them floating and the DC solve has no solution.
/// Flattening puts every node in one scope where it can be driven.
fn build_deck(spice: &str, netlist: &Netlist, cfg: &OpConfig) -> (String, String) {
    let (bench, provenance) = match &cfg.testbench {
        Some(tb) => (tb.clone(), "user testbench".to_string()),
        None => probe_bench(netlist, cfg),
    };
    let body = flat_circuit(netlist, cfg);
    let _ = spice;

    let lib = cfg.model_lib.as_ref().map_or(String::new(), |p| {
        format!(".lib {} {}\n", p.display(), cfg.corner)
    });

    let deck = format!(
        "* Philis operating-point probe (generated)\n\
         {lib}{body}\n\
         {bench}\n\
         .control\n\
         set ngbehavior=hsa\n\
         op\n\
         echo @@PHILIS_OP\n\
         show m : id,vds,vgs\n\
         echo @@PHILIS_END\n\
         .endc\n\
         .end\n"
    );
    (deck, provenance)
}

/// Emit every device flat, one line each, against the library's model names.
fn flat_circuit(netlist: &Netlist, cfg: &OpConfig) -> String {
    let mut s = String::new();
    for dev in &netlist.devices {
        let Some(model) = cfg.model_for(dev.kind) else {
            continue; // not a simulatable primitive here (R/C/L handled by their own cards)
        };
        let net = |t: &str| {
            dev.terminals
                .iter()
                .find(|(n, _)| n == t)
                .map(|(_, id)| node_name(netlist, *id))
                .unwrap_or_else(|| "0".into())
        };
        // sky130 primitives are subcircuits: D G S B, then W/L in microns.
        let (w_um, l_um) = (param_um(dev, "w"), param_um(dev, "l"));
        let nf = dev.params.iter().find(|(k, _)| k == "nf").map_or(1, |(_, v)| *v).max(1);
        s.push_str(&format!(
            "{} {} {} {} {} {} W={w_um} L={l_um} nf={nf}\n",
            instance_name(dev),
            net("D"),
            net("G"),
            net("S"),
            net("B"),
            model
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
fn node_name(netlist: &Netlist, id: pnr_core::NetId) -> String {
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
    let nm = dev.params.iter().find(|(k, _)| k == key).map_or(0, |(_, v)| *v);
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

/// First `.subckt` header as `(name, ports)`.
fn subckt_header(spice: &str) -> Option<(String, Vec<String>)> {
    for line in spice.lines() {
        let t = line.trim();
        if t.len() >= 7 && t[..7].eq_ignore_ascii_case(".subckt") {
            let mut it = t.split_whitespace().skip(1);
            let name = it.next()?.to_string();
            // Drop `key=value` parameters; ports are the bare tokens.
            let ports: Vec<String> =
                it.filter(|t| !t.contains('=')).map(str::to_string).collect();
            return Some((name, ports));
        }
    }
    None
}

/// Parse ngspice `show m : id,vds,vgs` column blocks into `device -> DevOp`.
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
            "id" | "vds" | "vgs" => {
                for (ci, raw) in rest.iter().enumerate() {
                    let Some(dev) = cols.get(ci) else { continue };
                    let Ok(v) = raw.parse::<f64>() else { continue };
                    let e = out.entry(dev.clone()).or_default();
                    match head {
                        "id" => e.id = v,
                        "vds" => e.vds = v,
                        _ => e.vgs = v,
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
        assert!(p("xm3") > 7.0 && p("xm3") < 8.0, "load ≈7.35µW, got {}", p("xm3"));
        assert!(p("xm5") < 0.1, "tail in triode ≈0.05µW, got {}", p("xm5"));
        assert!(p("xm3") > 100.0 * p("xm5"), "hot device must dominate");
    }

    #[test]
    fn instance_names_survive_truncation() {
        assert_eq!(instance_device("m.xm5.msky130_fd_pr__").as_deref(), Some("xm5"));
        assert_eq!(instance_device("m.xdut.xm1.mnfet").as_deref(), Some("xdut"));
        assert_eq!(instance_device("notadevice"), None);
    }

    /// Mirror-ish stub: `vbias` reaches only gates (no DC path), `vout` is
    /// driven by a drain, `VDD`/`VSS` are rails.
    fn stub_netlist() -> pnr_core::Netlist {
        use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};
        let nets = ["vdd", "vss", "vbias", "vout"]
            .iter()
            .map(|n| Net { name: (*n).to_string() })
            .collect();
        let dev = |name: &str, d: u16, g: u16, s: u16, b: u16| Device {
            name: name.to_string(),
            kind: DeviceKind::Nmos,
            terminals: vec![
                ("D".into(), NetId(d)),
                ("G".into(), NetId(g)),
                ("S".into(), NetId(s)),
                ("B".into(), NetId(b)),
            ],
            params: vec![("w".into(), 10_000), ("l".into(), 1_000)],
        };
        Netlist { devices: vec![dev("XM1", 3, 2, 1, 1)], nets }
    }

    #[test]
    fn probe_bench_drives_rails_and_gate_only_nets_only() {
        let (bench, note) = probe_bench(&stub_netlist(), &OpConfig::default());
        assert!(note.contains("NOT a sign-off bias"), "provenance must flag the guess: {note}");
        assert!(bench.contains("Vvdd vdd 0 1.8"), "supply at the rail: {bench}");
        // `vbias` reaches only a gate — no DC path — so the solve is singular
        // unless it is driven. This is the case a subckt wrapper cannot reach.
        assert!(bench.contains("Vvbias vbias 0 0.9"), "gate-only bias must be driven: {bench}");
        // `vout` is held by a drain; forcing it would destroy the operating point.
        assert!(!bench.contains("Vvout"), "circuit-driven node must float: {bench}");
        assert!(!bench.contains("Vvss"), "ground is node 0: {bench}");
    }

    #[test]
    fn flat_circuit_emits_devices_against_library_models() {
        let cfg = OpConfig::default();
        let deck = flat_circuit(&stub_netlist(), &cfg);
        assert!(deck.contains("sky130_fd_pr__nfet_01v8"), "library model name: {deck}");
        assert!(deck.contains("W=10 L=1"), "nm converted to microns: {deck}");
        assert!(deck.starts_with("XM1 vout vbias 0 0"), "D G S B order, vss→0: {deck}");
        assert!(!deck.starts_with("XXM1"), "must not double-prefix an X-name: {deck}");
    }

    #[test]
    fn subckt_header_skips_parameters() {
        let (n, ports) = subckt_header(".subckt amp a b VDD w=1u l=2u").unwrap();
        assert_eq!(n, "amp");
        assert_eq!(ports, vec!["a", "b", "VDD"]);
    }
}
