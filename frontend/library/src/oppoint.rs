//! DC operating point via ngspice: per-device power for the thermal rules.
//!
//! The bias comes from [`OpConfig::testbench`] when given; otherwise a mid-rail
//! probe bench is synthesised and [`OpPoint::provenance`] says so — a probe is
//! not a sign-off condition.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use annotator::NetRole;
use pnr_core::Netlist;

/// Per-device facts from a DC operating point.
#[derive(Clone, Default)]
pub struct OpPoint {
    /// Dissipation `|Id·Vds|` per device, µW.
    pub power_uw: Vec<i32>,
    /// Per device, µA: FET: drain current into the drain (NMOS as ngspice
    /// reports it, PMOS negated); resistor: current `P`→`N`; `None` otherwise or
    /// unresolved.
    pub id_ua: Vec<Option<f64>>,
    /// BJT `[ic, ib, ie]`, µA, each into its terminal as ngspice reports it; `None` for any other device or unresolved.
    pub bjt_ua: Vec<Option<[f64; 3]>>,
    /// Saturation headroom `|V_DS| − |V_DSsat|` per device, mV: what a series
    /// IR drop may eat before the device leaves saturation. Negative = already
    /// in triode; `None` when unresolved.
    pub headroom_mv: Vec<Option<f64>>,
    /// Transconductance per device, µS; `None` when unresolved.
    pub gm_us: Vec<Option<f64>>,
    /// Gate-source voltage per device, V, signed as ngspice prints it; `None`
    /// for a non-FET or unresolved.
    pub vgs_v: Vec<Option<f64>>,
    /// Drain-source voltage per device, V; `None` for a non-FET or unresolved.
    pub vds_v: Vec<Option<f64>>,
    /// Bulk-source voltage per device, V; `None` for a non-FET or unresolved.
    pub vbs_v: Vec<Option<f64>>,
    /// DC voltage per net, V, indexed by `NetId`; ground nets read 0.0. `None`
    /// for a net the simulated circuit does not reach (only capacitors, which
    /// the flat deck drops) or that ngspice did not print.
    pub net_v: Vec<Option<f64>>,
    /// How the bias was obtained, so a probe bench is never passed off as real.
    pub provenance: String,
    /// Devices the simulation reported.
    pub resolved: usize,
    /// Devices the simulation did not report, by netlist name.
    pub unresolved: Vec<String>,
}

impl OpPoint {
    /// DC current each device terminal draws from its net, µA, per device:
    /// FET `D` draws `+Id` (`id_ua`, into the drain), `S` `−Id`, and
    /// the gate and bulk none; a resistor `P` draws `+I`, `N` `−I`; a BJT
    /// `C`/`B`/`E` draw `ic`/`ib`/`ie` and the substrate none. A capacitor
    /// draws no DC current (every terminal 0). `None` for a FET, resistor or
    /// BJT the simulation did not resolve and for a diode or inductor (not
    /// simulated per terminal — unknown, never zero). A net's terminal currents
    /// sum to what its port supplies (zero without one) — the input the
    /// router's per-branch sums (Lienig & Thiele 2018 eqs. 3.5–3.7) need.
    #[must_use]
    pub fn terminal_ua(&self, netlist: &Netlist) -> Vec<Option<Vec<(String, f64)>>> {
        netlist
            .devices
            .iter()
            .enumerate()
            .map(|(i, dev)| {
                use pnr_core::DeviceKind as K;
                let id = || self.id_ua.get(i).copied().flatten();
                let known: Vec<(&str, f64)> = match dev.kind {
                    K::Nmos | K::Pmos => id().map(|i| vec![("D", i), ("S", -i)])?,
                    K::Resistor => id().map(|i| vec![("P", i), ("N", -i)])?,
                    K::Npn | K::Pnp => self.bjt_ua.get(i).copied().flatten().map(|[c, b, e]| vec![("C", c), ("B", b), ("E", e)])?,
                    K::Capacitor => Vec::new(),
                    K::Diode | K::Inductor => return None,
                };
                let draw = |t: &str| known.iter().find(|(n, _)| *n == t).map_or(0.0, |&(_, i)| i);
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
    /// `.param` lines emitted right after the model library: values a library reads but does not define
    /// in the chosen corner (sky130's NPN, [`SKY130_NPN_NOMINAL`]).
    pub params: Vec<(String, f64)>,
    /// Testbench appended to the circuit; `None` synthesises a mid-rail probe.
    pub testbench: Option<String>,
    /// Explicit DC voltage per net (case-insensitive name) for the probe bench;
    /// wins over the supply default.
    pub rails_v: Vec<(String, f64)>,
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
            params: Vec::new(),
            testbench: None,
            rails_v: Vec::new(),
            vdd: 1.8,
            nmos_model: String::new(),
            pmos_model: String::new(),
            ngspice: "ngspice".into(),
            temp_c: 27.0,
        }
    }
}

/// sky130 `npn_05v5_W1p00L1p00` reads these, which only the `mc` corner defines
/// (libs.tech/ngspice/parameters/montecarlo.spice:348, :353); here at the statistical
/// variable's nominal 0 (parameters/critical.spice:62).
/// ponytail: two parameters of one model; a deck-side table if another library needs more.
pub const SKY130_NPN_NOMINAL: [(&str, f64); 2] = [("dkisnpn1x1", 0.87913), ("dkbfnpn1x1", 0.98501)];

impl OpConfig {
    /// The model library include and [`OpConfig::params`], as deck lines.
    pub(crate) fn lib_lines(&self) -> String {
        let lib = self.model_lib.as_ref().map_or(String::new(), |p| format!(".lib {} {}\n", p.display(), self.corner));
        lib + &self.params.iter().map(|(k, v)| format!(".param {k}={v}\n")).collect::<String>()
    }
}

/// The last `n` characters of `s`: ngspice puts the fatal line last.
fn tail(s: &[u8], n: usize) -> String {
    let s = String::from_utf8_lossy(s);
    let k = s.chars().count().saturating_sub(n);
    s.chars().skip(k).collect()
}

/// A fresh, unique scratch directory under the OS temp dir, tagged `tag`
/// (`"op"`, `"perf"`): per-call, not per-process, so concurrent extracts in one
/// process never share a deck or a `bsim4v5.out` and the ngspice run's cwd
/// never leaks into the caller's.
pub(crate) fn scratch_dir(tag: &str) -> std::io::Result<PathBuf> {
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("philis_{tag}_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

/// `deck` under `ngspice -b` in its own [`scratch_dir`], removed on every path
/// once it exists unless `PHILIS_KEEP_DECKS` is set.
///
/// # Errors
/// The deck unwritable ("deck io"), ngspice not started ("ngspice unavailable"),
/// or [`check_exit`].
pub(crate) fn run_deck(ngspice: &str, tag: &str, deck: &str) -> Result<std::process::Output, String> {
    let dir = scratch_dir(tag).map_err(|e| format!("deck io: {e}"))?;
    let file = format!("{tag}.spice");
    let out = std::fs::write(dir.join(&file), deck)
        .map_err(|e| format!("deck io: {e}"))
        .and_then(|()| Command::new(ngspice).current_dir(&dir).arg("-b").arg(&file).output().map_err(|e| format!("ngspice unavailable: {e}")));
    if std::env::var_os("PHILIS_KEEP_DECKS").is_none() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let out = out?;
    check_exit(&out)?;
    Ok(out)
}

/// `Err("cannot simulate: ngspice exit …")` when ngspice failed: an unknown
/// subckt or a pin-count mismatch is fatal in batch mode (exit 1).
pub(crate) fn check_exit(out: &std::process::Output) -> Result<(), String> {
    if out.status.success() {
        return Ok(());
    }
    Err(format!("cannot simulate: ngspice {}: {}", out.status, tail(&out.stderr, 400)))
}

/// Solve `netlist`'s DC operating point and map it onto `netlist.devices`.
///
/// # Errors
/// ngspice missing, the deck unwritable, a device the deck cannot express or
/// ngspice rejects ("cannot simulate: …"), or no device data in the output —
/// all of which the caller treats as "bias unknown".
pub fn extract(netlist: &Netlist, cfg: &OpConfig) -> Result<OpPoint, String> {
    let (deck, provenance) = build_deck(netlist, cfg).map_err(|e| format!("cannot simulate: {e}"))?;
    let out = run_deck(&cfg.ngspice, "op", &deck)?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let table = parse_show(&stdout);
    let nodes = parse_nodes(&stdout);
    if table.is_empty() {
        let tail: String = String::from_utf8_lossy(&out.stderr)
            .chars()
            .take(400)
            .collect();
        return Err(format!("no device operating points: {tail}"));
    }
    let n = netlist.devices.len();
    let mut o = OpPoint {
        power_uw: vec![0; n],
        id_ua: vec![None; n],
        bjt_ua: vec![None; n],
        headroom_mv: vec![None; n],
        gm_us: vec![None; n],
        vgs_v: vec![None; n],
        vds_v: vec![None; n],
        vbs_v: vec![None; n],
        net_v: (0..netlist.nets.len())
            .map(|i| {
                let n = node_name(netlist, pnr_core::NetId(i as u16));
                if n == "0" { Some(0.0) } else { nodes.get(&n).copied() }
            })
            .collect(),
        provenance,
        resolved: 0,
        unresolved: Vec::new(),
    };
    for (i, dev) in netlist.devices.iter().enumerate() {
        use pnr_core::DeviceKind as K;
        // Diodes and capacitors are simulated but not looked up: unresolved.
        if !matches!(dev.kind, K::Nmos | K::Pmos | K::Resistor | K::Npn | K::Pnp) {
            o.unresolved.push(dev.name.clone());
            continue;
        }
        let Some(op) = table.get(instance_name(dev).to_ascii_lowercase().as_str()) else {
            o.unresolved.push(dev.name.clone());
            continue;
        };
        match dev.kind {
            K::Nmos | K::Pmos => {
                o.power_uw[i] = ((op.id * op.vds).abs() * 1e6).round() as i32;
                // ngspice reports a PMOS `id` positive out of the drain (measured:
                // sky130 pfet sourcing 156 µA into a 0.9 V drain source reads +156 µA).
                let into_drain = if dev.kind == K::Pmos { -op.id } else { op.id };
                o.id_ua[i] = Some(into_drain * 1e6);
                o.headroom_mv[i] = Some((op.vds.abs() - op.vdsat.abs()) * 1e3);
                o.gm_us[i] = Some(op.gm.abs() * 1e6);
                o.vgs_v[i] = Some(op.vgs);
                o.vds_v[i] = Some(op.vds);
                o.vbs_v[i] = Some(op.vbs);
            }
            K::Resistor => {
                o.power_uw[i] = (op.p.abs() * 1e6).round() as i32;
                o.id_ua[i] = Some(op.i * 1e6);
            }
            _ => {
                o.power_uw[i] = (op.p.abs() * 1e6).round() as i32;
                o.bjt_ua[i] = Some([op.ic, op.ib, op.ie].map(|a| a * 1e6));
            }
        }
        o.resolved += 1;
    }
    Ok(o)
}

/// One device's operating point as `show` reports it.
#[derive(Default, Clone, Copy)]
struct DevOp {
    id: f64,
    vds: f64,
    vdsat: f64,
    gm: f64,
    vgs: f64,
    vbs: f64,
    ic: f64,
    ib: f64,
    ie: f64,
    i: f64,
    p: f64,
}

/// Assemble the deck: model library, the circuit, a bias bench, and a control
/// block that dumps every MOSFET's operating point and every node voltage at once.
///
/// The circuit is emitted **flat, from the parsed netlist**, not by reusing the
/// input text. A `.subckt` cannot be biased from outside: its internal bias nodes
/// (a mirror's gate rail, a tail bias) are invisible at the top level, so a
/// wrapper testbench leaves them floating and the DC solve has no solution.
/// Flattening puts every node in one scope where it can be driven.
fn build_deck(netlist: &Netlist, cfg: &OpConfig) -> Result<(String, String), String> {
    let (bench, provenance) = match &cfg.testbench {
        Some(tb) => (tb.clone(), "user testbench".to_string()),
        None => probe_bench(netlist, cfg),
    };
    let body = flat_circuit(netlist, cfg)?;
    let lib = cfg.lib_lines();

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
         show m : id,vgs,vds,vbs,vdsat,gm\n\
         show q : ic,ib,ie,p\n\
         show r : i,p\n\
         echo @@PHILIS_END\n\
         echo @@PHILIS_NV\n\
         print all\n\
         echo @@PHILIS_NV_END\n\
         .endc\n\
         .end\n"
    );
    Ok((deck, provenance))
}

/// Emit every device flat, one line each, against the library's model names.
/// Capacitors are left out: a DC operating point sees them open, so dropping
/// them is exact (and a library without the drawn capacitor model still
/// solves).
pub(crate) fn flat_circuit(netlist: &Netlist, cfg: &OpConfig) -> Result<String, String> {
    flat_circuit_with(netlist, cfg, |_, _, n| n, |_| String::new(), false)
}

/// [`flat_circuit`] with each terminal's node renamed by `node(device index,
/// terminal, net node)`, `extra(device index)` appended to each FET card (a
/// post-layout deck's branch resistors and stress parameters), and capacitor
/// cards when `emit_caps`.
///
/// FET: `X… D G S B model W= L= nf= m=` (W the instance total, µm: the size
/// layout draws, `pnr_core::MosSize`). A modelled resistor is an `R` element
/// (`R<inst> P N model w= l= m=`), an unmodelled one `R<inst> P N ohms`;
/// diode `D<inst> P N model [area=]`; BJT `X… C B E [S] model m=`; modelled
/// capacitor `X… P N model w= l= m=`, else `C<inst> P N <aF>a`.
///
/// # Errors
/// A device the deck cannot express: a FET or modelled R/C without W/L, an
/// R/C with neither model nor value, an inductor.
pub(crate) fn flat_circuit_with(
    netlist: &Netlist,
    cfg: &OpConfig,
    node: impl Fn(usize, &str, String) -> String,
    extra: impl Fn(usize) -> String,
    emit_caps: bool,
) -> Result<String, String> {
    use pnr_core::DeviceKind as K;
    let mut s = String::new();
    for (di, dev) in netlist.devices.iter().enumerate() {
        let inst = instance_name(dev);
        let n = |t: &str| {
            let n = dev.terminals.iter().find(|(n, _)| n == t).map(|(_, id)| node_name(netlist, *id)).unwrap_or_else(|| "0".into());
            node(di, t, n)
        };
        let param = |k: &str| dev.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
        let m = param("m").unwrap_or(1);
        let um = |k: &str| param(k).map(|v| v as f64 / 1000.0);
        let wl = || um("w").zip(um("l")).ok_or_else(|| format!("{inst}: no W/L"));
        let model = &dev.model;
        let card = match dev.kind {
            K::Nmos | K::Pmos => {
                let forced = if dev.kind == K::Nmos { &cfg.nmos_model } else { &cfg.pmos_model };
                let model = if forced.is_empty() { model } else { forced };
                let sz = dev.mos_size().ok_or_else(|| format!("{inst}: no W/L"))?;
                format!(
                    "{inst} {} {} {} {} {model} W={} L={} nf={} m={}{}",
                    n("D"),
                    n("G"),
                    n("S"),
                    n("B"),
                    sz.w_total_nm as f64 / 1000.0,
                    sz.l_nm as f64 / 1000.0,
                    sz.nf,
                    sz.m,
                    extra(di)
                )
            }
            // ponytail: an R element for every modelled resistor; a library shipping one as a .subckt (sky130
            // res_high_po, libs.ref only) fails in ngspice and reads "cannot simulate".
            K::Resistor if !model.is_empty() => {
                let (w, l) = wl()?;
                format!("R{inst} {} {} {model} w={w} l={l} m={m}", n("P"), n("N"))
            }
            K::Resistor => {
                let r = param("r_mohm").ok_or_else(|| format!("{inst}: resistor has no model or value"))?;
                format!("R{inst} {} {} {}", n("P"), n("N"), r as f64 / 1000.0)
            }
            K::Diode => {
                let area = um("w").zip(um("l")).map_or(String::new(), |(w, l)| format!(" area={}", w * l));
                format!("D{inst} {} {} {model}{area}", n("P"), n("N"))
            }
            K::Npn => format!("{inst} {} {} {} {} {model} m={m}", n("C"), n("B"), n("E"), n("S")),
            K::Pnp => format!("{inst} {} {} {} {model} m={m}", n("C"), n("B"), n("E")),
            K::Capacitor if !emit_caps => continue,
            K::Capacitor if !model.is_empty() => {
                let (w, l) = wl()?;
                format!("{inst} {} {} {model} w={w} l={l} m={m}", n("P"), n("N"))
            }
            K::Capacitor => {
                let c = param("c_af").ok_or_else(|| format!("{inst}: capacitor has no model or value"))?;
                format!("C{inst} {} {} {c}a", n("P"), n("N"))
            }
            K::Inductor => return Err(format!("{inst}: inductors are not simulated")),
        };
        s.push_str(&card);
        s.push('\n');
    }
    Ok(s)
}

/// SPICE instance name for a device.
///
/// A subcircuit instance must start with `X`, but netlist names usually already
/// do (`XM1`). Blindly prefixing yields `XXM1`, whose operating point then fails
/// to map back to the device — so prefix only when needed. A flattened
/// hierarchical name's `/` becomes `__` (`X1/XM2` → `X1__XM2`), so
/// `parse_show`'s `m.<name>.` split still finds one token; the parser rejects
/// two devices that collide here.
pub(crate) fn instance_name(dev: &pnr_core::Device) -> String {
    let name = dev.name.replace('/', "__");
    if name.starts_with('X') || name.starts_with('x') {
        name
    } else {
        format!("X{name}")
    }
}

/// Net name, sanitised for SPICE and mapped to node 0 for grounds.
///
/// ponytail: rails are what the name classifier says ([`annotator::rail_of`]);
/// a bulk-inferred rail (`annotator::classify_nets`) is not grounded here —
/// same ceiling as the name list it replaced.
pub(crate) fn node_name(netlist: &Netlist, id: pnr_core::NetId) -> String {
    let raw = netlist
        .nets
        .get(id.0 as usize)
        .map_or_else(|| format!("n{}", id.0), |n| n.name.clone());
    if annotator::rail_of(&raw) == Some(NetRole::Ground) {
        "0".into()
    } else {
        raw.to_ascii_lowercase().replace(['/', '.', '<', '>'], "_")
    }
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
        let rail = annotator::rail_of(&net.name);
        if rail == Some(NetRole::Ground) {
            continue; // node 0
        }
        let name = node_name(netlist, pnr_core::NetId(i as u16));
        let v = if let Some(&(_, v)) = cfg.rails_v.iter().find(|(n, _)| n.eq_ignore_ascii_case(&net.name)) {
            v
        } else if rail == Some(NetRole::Supply) {
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
    let mut cols: Vec<Option<String>> = Vec::new();
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
                // `.map`, not `filter_map`: an unparseable column must still
                // occupy its slot so later columns are not shifted left.
                cols = rest.iter().map(|c| instance_device(c)).collect();
            }
            "id" | "vgs" | "vds" | "vbs" | "vdsat" | "gm" | "ic" | "ib" | "ie" | "i" | "p" => {
                for (ci, raw) in rest.iter().enumerate() {
                    let Some(Some(dev)) = cols.get(ci) else { continue };
                    let Ok(v) = raw.parse::<f64>() else { continue };
                    let e = out.entry(dev.clone()).or_default();
                    match head {
                        "id" => e.id = v,
                        "vgs" => e.vgs = v,
                        "vds" => e.vds = v,
                        "vbs" => e.vbs = v,
                        "gm" => e.gm = v,
                        "vdsat" => e.vdsat = v,
                        "ic" => e.ic = v,
                        "ib" => e.ib = v,
                        "ie" => e.ie = v,
                        "i" => e.i = v,
                        _ => e.p = v,
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Node voltages from `print all` between `@@PHILIS_NV` and `@@PHILIS_NV_END`:
/// one `name = value` line per node (ground is not printed), keyed lowercase.
/// `v1#branch = …` source currents are skipped.
fn parse_nodes(text: &str) -> std::collections::HashMap<String, f64> {
    text.lines()
        .skip_while(|l| !l.contains("@@PHILIS_NV"))
        .skip(1)
        .take_while(|l| !l.contains("@@PHILIS_NV_END"))
        .filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            [name, "=", v] if !name.contains('#') => {
                Some((name.to_ascii_lowercase(), v.parse::<f64>().ok()?))
            }
            _ => None,
        })
        .collect()
}

/// `m.xm5.msky130_fd_pr__` → `xm5`, `q.xq1.qsky130_fd_pr__` → `xq1`; a
/// top-level `R` element's column is its own name (`rxr1` → `xr1`).
fn instance_device(col: &str) -> Option<String> {
    if let Some(s) = col.strip_prefix("m.").or_else(|| col.strip_prefix("q.")) {
        let end = s.find('.')?;
        return Some(s[..end].to_string());
    }
    col.strip_prefix('r').map(str::to_string)
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
            ..Default::default()
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
        let deck = flat_circuit(&stub_netlist(), &cfg).unwrap();
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
            ..Default::default()
        };
        let op = |ids: [Option<f64>; 3]| OpPoint { power_uw: vec![0; 3], id_ua: ids.to_vec(), headroom_mv: vec![None; 3], gm_us: vec![None; 3], provenance: String::new(), resolved: 3, ..Default::default() };
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

    /// An unresolved resistor's DC current is unknown, and its net's current
    /// with it. A capacitor carries no DC current: every terminal a known 0,
    /// and its net stays known.
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
            ..Default::default()
        };
        let op = OpPoint { power_uw: vec![0; 2], id_ua: vec![None; 2], headroom_mv: vec![None; 2], gm_us: vec![None; 2], provenance: String::new(), resolved: 0, ..Default::default() };
        let t = op.terminal_ua(&nl);
        assert!(t[0].is_none(), "resistor: unknown, never zero");
        assert_eq!(t[1], Some(vec![("P".to_string(), 0.0), ("N".to_string(), 0.0)]));
        let i = net_current_ua(&nl, &t);
        assert_eq!((i[0], i[1]), (None, Some(0)), "the resistor's net unknown, the capacitor's known");
    }

    /// A modelled resistor is an `R` element against its `.model … r`
    /// (sky130 `res_generic_*`); a bare one carries its value in ohms.
    #[test]
    fn a_resistor_gets_an_r_card_with_its_model() {
        let nl = crate::parse("XR1 a b sky130_fd_pr__res_generic_po w=0.5u l=2u\nXM1 a g 0 0 nfet_01v8 W=1u L=1u\n").unwrap();
        let c = flat_circuit(&nl, &OpConfig::default()).unwrap();
        assert!(c.lines().any(|l| l == "RXR1 a b sky130_fd_pr__res_generic_po w=0.5 l=2 m=1"), "{c}");
        let c = flat_circuit(&crate::parse("R1 a b 10k\n").unwrap(), &OpConfig::default()).unwrap();
        assert!(c.lines().any(|l| l == "RXR1 a b 10000"), "{c}");
    }

    /// sky130 BJTs are `.subckt c b e [s]`: collector first, a missing
    /// substrate on ground.
    #[test]
    fn a_bjt_card_is_collector_first() {
        let nl = crate::parse(include_str!("../../../benchmarks/fixtures/bjt_mirror.spice")).unwrap();
        let c = flat_circuit(&nl, &OpConfig::default()).unwrap();
        assert!(c.lines().any(|l| l == "XQ1 outn in 0 0 sky130_fd_pr__npn_05v5_W1p00L1p00 m=1"), "{c}");
        assert!(c.lines().any(|l| l == "XQ2 outp in vdd sky130_fd_pr__pnp_05v5_W3p40L3p40 m=1"), "{c}");
    }

    #[test]
    fn an_inductor_refuses_to_simulate() {
        let e = flat_circuit(&crate::parse("L1 a b 1n\n").unwrap(), &OpConfig::default()).unwrap_err();
        assert!(e.contains("inductors are not simulated"), "{e}");
    }

    /// No invented geometry: a FET without `L` is an error, not a 0.15 µm guess.
    #[test]
    fn a_missing_length_is_an_error() {
        let e = flat_circuit(&crate::parse("XM1 d g 0 0 nfet_01v8 W=1u\n").unwrap(), &OpConfig::default()).unwrap_err();
        assert!(e.contains("no W/L"), "{e}");
    }

    /// Measured ngspice output for `show q : ic,ib,ie,p` and `show r : i,p`.
    #[test]
    fn parse_show_reads_bjt_and_resistor_columns() {
        let t = parse_show(
            "\
@@PHILIS_OP
 BJT: Bipolar Junction Transistor
     device q.xq2.qsky130_fd_pr__ q.xq1.qsky130_fd_pr__
      model xq2:sky130_fd_pr__pnp xq1:sky130_fd_pr__npn
         ic          -7.76701e-05           5.14061e-07
         ib          -6.20301e-06           1.42865e-08
         ie           8.38731e-05          -5.28349e-07
          p           0.000144768           5.24063e-07
 Resistor: Simple linear resistor
     device                  rxr1
      model sky130_fd_pr__res_gen
          i            0.00520059
          p            0.00520059
@@PHILIS_END
",
        );
        assert_eq!(t["xq2"].ic, -7.76701e-05);
        assert_eq!(t["xq1"].ib, 1.42865e-08);
        assert_eq!(t["xr1"].i, 0.00520059);
    }

    /// A resolved resistor draws `+I` at `P` and returns it at `N`.
    #[test]
    fn a_resolved_resistor_draws_its_current() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let nl = Netlist {
            devices: vec![Device { name: "R1".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(1))], params: vec![] }],
            nets: ["x", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let op = OpPoint { id_ua: vec![Some(5.0)], ..Default::default() };
        let t = op.terminal_ua(&nl);
        assert_eq!(t[0], Some(vec![("P".to_string(), 5.0), ("N".to_string(), -5.0)]));
        assert_eq!(net_current_ua(&nl, &t)[0], Some(5));
    }

    /// Headroom is `|V_DS| − V_DSsat`: a triode tail has none to spend, and a
    /// net takes the tightest saturated device on it.
    #[test]
    fn headroom_reads_vdsat_and_a_net_takes_its_tightest_device() {
        let t = parse_show(SHOW);
        assert!((t["xm3"].vdsat - 0.0485318).abs() < 1e-9);
        let nl = stub_netlist(); // XM1: D=vout(3) S=vss(1)
        let op = |h: Option<f64>| OpPoint { power_uw: vec![0], id_ua: vec![Some(1.0)], headroom_mv: vec![h], gm_us: vec![None], provenance: String::new(), resolved: 1, ..Default::default() };
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
            ..Default::default()
        };
        let op = |ids: [Option<f64>; 3]| OpPoint { power_uw: vec![0; 3], id_ua: ids.to_vec(), headroom_mv: vec![None; 3], gm_us: vec![None; 3], provenance: String::new(), resolved: 3, ..Default::default() };
        let i = net_current_ua(&nl, &op([Some(10.0), Some(10.0), Some(20.0)]).terminal_ua(&nl));
        assert_eq!((i[2], i[4], i[3]), (Some(20), Some(20), Some(0)), "tail, the ground return, a gate net");
        let u = net_current_ua(&nl, &op([Some(10.0), None, Some(20.0)]).terminal_ua(&nl));
        assert_eq!((u[1], u[2]), (None, None), "M2 unresolved: its nets are unknown");
    }

    /// An unparseable device column (no `m.`/`q.`/`r` prefix) must still take
    /// its slot in `cols`, so a later column is not shifted onto the wrong
    /// device name.
    #[test]
    fn a_truncated_header_does_not_shift_later_devices() {
        // The first column: a shift would hand xm4 xm5's distinct values.
        let truncated = SHOW.replace("m.xm5.msky130_fd_pr__", "xm5:truncated");
        let t = parse_show(&truncated);
        let keys: Vec<_> = t.keys().collect();
        assert!(!t.contains_key("xm5"), "the truncated column must not parse as xm5: {keys:?}");
        assert!((t["xm4"].id - 4.10968e-06).abs() < 1e-12, "xm4 keeps its own id, not xm5's: {}", t["xm4"].id);
        assert!((t["xm4"].vds - 1.78826).abs() < 1e-9, "xm4 keeps its own vds, not xm5's: {}", t["xm4"].vds);
    }

    /// A failed run leaves no scratch dir behind.
    #[test]
    fn a_missing_ngspice_leaks_no_scratch_dir() {
        let e = run_deck("/nonexistent/ngspice", "leakcheck", "* empty\n.end\n").unwrap_err();
        assert!(e.starts_with("ngspice unavailable"), "{e}");
        let prefix = format!("philis_leakcheck_{}_", std::process::id());
        let left = std::fs::read_dir(std::env::temp_dir()).unwrap().flatten().any(|d| d.file_name().to_string_lossy().starts_with(&prefix));
        assert!(!left, "{prefix}* left in the temp dir");
    }

    #[test]
    fn avdd_is_driven_when_classified_supply() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        // nets: 0 avdd (supply, by name only), 1 vss (ground), 2 vin (gate-only)
        let nl = Netlist {
            devices: vec![Device {
                name: "XM1".into(),
                kind: DeviceKind::Nmos,
                model: "sky130_fd_pr__nfet_01v8".into(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(2)), ("S".into(), NetId(1)), ("B".into(), NetId(1))],
                params: vec![("w".into(), 1_000), ("l".into(), 150)],
            }],
            nets: ["avdd", "vss", "vin"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let (bench, _) = probe_bench(&nl, &OpConfig::default());
        assert!(bench.contains("Vavdd avdd 0 1.8"), "supply by name, not by topology: {bench}");
        assert!(!bench.contains("Vvss"), "ground is node 0: {bench}");
        assert!(bench.contains("Vvin vin 0 0.9"), "gate-only net driven: {bench}");

        let cfg = OpConfig { rails_v: vec![("AVDD".into(), 3.3)], ..Default::default() };
        let (bench, _) = probe_bench(&nl, &cfg);
        assert!(bench.contains("Vavdd avdd 0 3.3"), "rails_v wins over the supply default: {bench}");
    }

    #[test]
    fn node_zero_is_ground() {
        let nl = Netlist {
            nets: ["0", "VSS", "vssa", "gnd!", "vout"].iter().map(|n| pnr_core::Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        for (i, want) in [(0, "0"), (1, "0"), (2, "0"), (3, "0"), (4, "vout")] {
            assert_eq!(node_name(&nl, pnr_core::NetId(i)), want, "{}", nl.nets[i as usize].name);
        }
    }

    #[test]
    fn print_all_reads_node_voltages() {
        let nv = "@@PHILIS_NV\nvdd = 1.800000e+00\nx1_mid = 9.000000e-01\nv1#branch = -9.00000e-04\n@@PHILIS_NV_END\n";
        let text = format!("{SHOW}{nv}");
        let m = parse_nodes(&text);
        assert_eq!(m.len(), 2);
        assert_eq!(m["vdd"], 1.8);
        assert_eq!(m["x1_mid"], 0.9);
        assert!(m.keys().all(|k| !k.contains('#')));
        let key = |t: &std::collections::HashMap<String, DevOp>| {
            let mut v: Vec<_> = t.iter().map(|(k, d)| (k.clone(), d.id.to_bits(), d.vds.to_bits(), d.gm.to_bits())).collect();
            v.sort();
            v
        };
        assert_eq!(key(&parse_show(&text)), key(&parse_show(SHOW)));
    }

    #[test]
    fn show_reads_vgs_and_vbs() {
        let text = "\
@@PHILIS_OP
 BSIM4v5: Berkeley Short Channel IGFET Model-4
     device m.xm5.msky130_fd_pr__ m.xm4.msky130_fd_pr__ m.xm3.msky130_fd_pr__
      model xm5:sky130_fd_pr__nfe xm4:sky130_fd_pr__pfe xm3:sky130_fd_pr__pfe
         id           8.21938e-06           4.10968e-06           4.10968e-06
        vgs                   0.8                  1.05                  1.05
        vds            0.00628286               1.78826               1.78826
        vbs                     0                  -0.3                  -0.3
      vdsat              0.297885             0.0485318             0.0485318
@@PHILIS_END
";
        let t = parse_show(text);
        assert_eq!(t["xm5"].vgs, 0.8);
        assert_eq!(t["xm4"].vbs, -0.3);
    }

    /// A 1-NMOS netlist against the sky130 library: two threads must not
    /// share a deck directory (the old PID-only dir did).
    #[test]
    fn concurrent_extracts_use_distinct_decks() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let Some(lib) = crate::tools::sky130_models() else { return };
        let nmos = |w: i64| Netlist {
            devices: vec![Device {
                name: "XM1".into(),
                kind: DeviceKind::Nmos,
                model: "sky130_fd_pr__nfet_01v8".into(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), w), ("l".into(), 150)],
            }],
            nets: ["vdd", "vg", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let cfg = OpConfig { model_lib: Some(lib), ..Default::default() };
        let (a, b) = std::thread::scope(|s| {
            let ha = s.spawn(|| extract(&nmos(1_000), &cfg));
            let hb = s.spawn(|| extract(&nmos(4_000), &cfg));
            (ha.join().unwrap(), hb.join().unwrap())
        });
        let a = a.expect("1u extract");
        let b = b.expect("4u extract");
        assert_eq!(a.resolved, 1);
        assert_eq!(b.resolved, 1);
        assert!(b.id_ua[0].unwrap() > a.id_ua[0].unwrap(), "wider device draws more current: {:?} vs {:?}", b.id_ua, a.id_ua);
    }

    /// ngspice's `bsim4v5.out` lands in its own cwd; `extract` must run it in
    /// a scratch dir, not the caller's, and clean that dir up afterward.
    #[test]
    fn extract_leaves_nothing_in_the_cwd() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let Some(lib) = crate::tools::sky130_models() else { return };
        let marker = std::env::current_dir().unwrap().join("bsim4v5.out");
        let _ = std::fs::remove_file(&marker);
        let nl = Netlist {
            devices: vec![Device {
                name: "XM1".into(),
                kind: DeviceKind::Nmos,
                model: "sky130_fd_pr__nfet_01v8".into(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 1_000), ("l".into(), 150)],
            }],
            nets: ["vdd", "vg", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let cfg = OpConfig { model_lib: Some(lib), ..Default::default() };
        extract(&nl, &cfg).expect("extract");
        assert!(!marker.exists(), "extract must not leave bsim4v5.out in the cwd");
    }
}
