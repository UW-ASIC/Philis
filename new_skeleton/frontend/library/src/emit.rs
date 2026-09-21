//! # `emit` — decompile a solved [`Solution`] into a PDK-agnostic generator.
//!
//! The flow's output is nm-coordinates against one deck. This module lifts it
//! into a [`GenIr`]: instances (device family + params), *relational*
//! placement (align chains with rule-attributed gaps), and the net list —
//! the search's **decisions** without its **numbers**. The IR can be
//! interpreted against any [`Process`] ([`elaborate_ir`]) or pretty-printed
//! as macroMaster [`Composition`] source ([`to_rust`]).
//!
//! v1 scope (deliberate): per-device instances only — a merged matched group
//! (group collapse) draws interdigitated legs one `variants::Mos` cannot yet
//! express (M5: patterned library). Symmetry axes ride in `Layout::axis` but
//! are not yet lifted to `place_mirrored` (P2). Both return
//! [`EmitError::Unsupported`] rather than emitting wrong code.

use annotator::{annotate, NoInference};
use macro_master::{build_with, variants, AlignMode, GenError, Macros};
use pnr_core::{DeviceKind, Process as _};
use verify::Pdk;

use crate::elaborate::{route_built, ElabConfig, Elaborated};
use crate::{cellgen, Config, Solution};

/// One emitted device instance.
#[derive(Debug)]
pub struct IrInst {
    pub name: String,
    pub kind: DeviceKind,
    /// Unit finger/segment width, nm (a device *parameter*, not a layout nm).
    pub w: i32,
    /// Gate/body length, nm.
    pub l: i32,
    /// Fingers — total for a single device, per-leg for a pair.
    pub nf: u16,
    /// `1` = one device (`variants::Mos`/`Res`); `2` = a merged matched pair
    /// drawn as one interdigitated macro (`variants::MatchedPair`, per-leg
    /// pins `g1…s2`). This is how group collapse survives decompilation.
    pub legs: u8,
}

/// A gap in a placement op: attributed to a named process rule when the
/// solved value matches one, else a raw nm residue (PDK-specific — the
/// attribution failing is information, not a fallback to hide).
#[derive(Debug, Clone)]
pub enum IrGap {
    /// `process.rule(name, default)` at elaboration.
    Rule(String, i32),
    /// Literal nm (lossy across PDKs; kept honest in the emitted source).
    Nm(i32),
}

/// One align step against an already-placed instance.
#[derive(Debug)]
pub struct IrAlign {
    pub mode: AlignMode,
    /// Index into [`GenIr::instances`] of the placed reference.
    pub reference: usize,
    pub gap: IrGap,
}

/// Placement program for one instance: zero aligns = anchor at origin.
#[derive(Debug)]
pub struct IrPlace {
    pub inst: usize,
    pub aligns: Vec<IrAlign>,
}

/// The decompiled generator — decisions only, no coordinates.
#[derive(Debug)]
pub struct GenIr {
    pub name: String,
    /// Io port names (v1: every named net).
    pub ports: Vec<String>,
    pub instances: Vec<IrInst>,
    /// In placement order; references always point at earlier entries.
    pub place: Vec<IrPlace>,
    /// Connect edges: `inst.term` ↔ net/port name.
    pub edges: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum EmitError {
    /// The solution uses a feature the emitter cannot yet express faithfully.
    Unsupported(String),
}

/// Decompile a flow [`Solution`] — thin wrapper over [`emit`].
pub fn emit_solution(sol: &Solution, pdk: &Pdk, cfg: &Config) -> Result<GenIr, EmitError> {
    emit(&sol.netlist, &sol.layout, pdk, cfg)
}

/// Decompile a solved placement against the deck it was solved on. The layout
/// may come from [`crate::run`] or any other producer — the emitter's contract
/// is `(netlist, layout)`, not the flow. `pdk` is used only to *attribute*
/// gaps to rule values — nothing PDK-specific survives into the IR except
/// unattributed residues.
pub fn emit(
    netlist: &pnr_core::Netlist,
    layout: &pnr_core::Layout,
    pdk: &Pdk,
    cfg: &Config,
) -> Result<GenIr, EmitError> {
    let problem = annotate(netlist, &NoInference, &cfg.annotation);
    let cells = cellgen::enumerate(netlist, &Macros::default(), &problem.constraints, pdk);

    // Instances: device family + electrical params from the covering
    // unitization (cellgen synthesizes one per un-matched device). A 2-member
    // cell is a merged matched pair → `MatchedPair` (per-leg pins); >2 members
    // (quads) still need a patterned quad variant.
    let mut instances = Vec::with_capacity(cells.devices_of.len());
    for members in &cells.devices_of {
        if members.len() > 2 {
            return Err(EmitError::Unsupported(format!(
                "merged matched group of {} devices: no quad variant yet",
                members.len()
            )));
        }
        let d = &netlist.devices[members[0].0 as usize];
        match d.kind {
            DeviceKind::Nmos | DeviceKind::Pmos | DeviceKind::Resistor => {}
            k => {
                return Err(EmitError::Unsupported(format!(
                    "device kind {k:?} has no macroMaster variant yet (M5)"
                )))
            }
        }
        let u = problem
            .constraints
            .unitization
            .iter()
            .find(|u| members.iter().all(|m| u.devices.contains(m)));
        let (w, l, nf) = match u {
            Some(u) => {
                let slot = u.devices.iter().position(|x| x == &members[0]);
                let nf = slot.and_then(|s| u.dev_nf.get(s)).copied().unwrap_or(1).max(1);
                (u.unit_w.max(1), u.unit_l.max(1), nf)
            }
            // No covering unitization (unmatched device): schematic params
            // verbatim. The parser stores lowercase keys, nm units.
            None => {
                let p = |k: &str, d_: i64| {
                    d.params.iter().find(|(n, _)| n == k).map_or(d_, |(_, v)| *v)
                };
                (p("w", 420) as i32, p("l", 150) as i32, p("nf", 1).max(1) as u16)
            }
        };
        let legs = members.len() as u8;
        if legs == 2 {
            let d2 = &netlist.devices[members[1].0 as usize];
            if d2.kind != d.kind {
                return Err(EmitError::Unsupported(
                    "mixed-kind merged group (implants would merge)".into(),
                ));
            }
            // Equal legs only: a ratioed mirror (nf 1:2) needs per-leg counts
            // MatchedPair does not model yet.
            if let Some(u) = u {
                if u.dev_nf.windows(2).any(|w| w[0] != w[1]) {
                    return Err(EmitError::Unsupported(
                        "ratioed merged group: MatchedPair models equal legs only".into(),
                    ));
                }
            }
        }
        let name = if legs == 2 {
            format!("{}_{}", d.name, netlist.devices[members[1].0 as usize].name)
        } else {
            d.name.clone()
        };
        instances.push(IrInst { name, kind: d.kind, w, l, nf, legs });
    }

    // Placement lift: order by solved bottom-left corner, then express each
    // instance relative to an earlier neighbour — same row ⇒ Bottom-align +
    // ToTheRight; new row ⇒ Left-align + Above. Gaps that land on the deck's
    // `device_gap` (within a grid step) are attributed to the rule.
    let l_ = layout;
    let n = instances.len();
    let grid = pdk_grid(pdk);
    let device_gap = process_rule(pdk, "device_gap", 600);
    let corner = |i: usize| (l_.y[i] - l_.hh[i], l_.x[i] - l_.hw[i]);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| corner(i));

    let attribute = |gap: i32| -> IrGap {
        if (gap - device_gap).abs() <= 2 * grid {
            IrGap::Rule("device_gap".into(), device_gap)
        } else {
            IrGap::Nm(gap.max(0))
        }
    };

    let mut place: Vec<IrPlace> = Vec::with_capacity(n);
    for (k, &i) in order.iter().enumerate() {
        if k == 0 {
            place.push(IrPlace { inst: i, aligns: Vec::new() });
            continue;
        }
        // Same row: a placed cell whose y-range overlaps and that sits left.
        let row_mate = order[..k]
            .iter()
            .copied()
            .filter(|&j| {
                (l_.y[j] - l_.hh[j]) < (l_.y[i] + l_.hh[i])
                    && (l_.y[i] - l_.hh[i]) < (l_.y[j] + l_.hh[j])
                    && l_.x[j] < l_.x[i]
            })
            .max_by_key(|&j| l_.x[j]);
        let aligns = if let Some(j) = row_mate {
            let gap = (l_.x[i] - l_.hw[i]) - (l_.x[j] + l_.hw[j]);
            vec![
                IrAlign { mode: AlignMode::Bottom, reference: j, gap: IrGap::Nm(
                    (l_.y[i] - l_.hh[i]) - (l_.y[j] - l_.hh[j]),
                ) },
                IrAlign { mode: AlignMode::ToTheRight, reference: j, gap: attribute(gap) },
            ]
        } else {
            // New row: nearest below with x-overlap, else the previous anchor.
            let below = order[..k]
                .iter()
                .copied()
                .filter(|&j| l_.y[j] < l_.y[i])
                .max_by_key(|&j| l_.y[j])
                .unwrap_or(order[0]);
            let j = below;
            let gap = (l_.y[i] - l_.hh[i]) - (l_.y[j] + l_.hh[j]);
            vec![
                IrAlign { mode: AlignMode::Left, reference: j, gap: IrGap::Nm(
                    (l_.x[i] - l_.hw[i]) - (l_.x[j] - l_.hw[j]),
                ) },
                IrAlign { mode: AlignMode::Above, reference: j, gap: attribute(gap) },
            ]
        };
        place.push(IrPlace { inst: i, aligns });
    }

    // Connectivity: every device terminal to its net name. For a merged pair
    // the member ordinal becomes the leg suffix (`MatchedPair`'s `g1`/`g2`),
    // except the shared body — one `b` pin serves both legs.
    let net_name = |id: pnr_core::NetId| netlist.nets[id.0 as usize].name.clone();
    let mut edges = Vec::new();
    for (inst, members) in instances.iter().zip(&cells.devices_of) {
        for (o, m) in members.iter().enumerate() {
            let d = &netlist.devices[m.0 as usize];
            for (idx, (t, net)) in d.terminals.iter().enumerate() {
                let mut term = terminal_port(inst.kind, t, idx);
                if inst.legs == 2 && term != "b" {
                    term = format!("{term}{}", o + 1);
                }
                edges.push((format!("{}.{}", inst.name, term), net_name(*net)));
            }
        }
    }
    let ports: Vec<String> = netlist.nets.iter().map(|nt| nt.name.clone()).collect();

    Ok(GenIr { name: "emitted".into(), ports, instances, place, edges })
}

/// Map a schematic terminal to the macroMaster variant's port name.
fn terminal_port(kind: DeviceKind, t: &str, position: usize) -> String {
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => t.to_ascii_lowercase(),
        // Two-terminal devices: variant io is a/b, schematic order decides.
        _ => if position == 0 { "a".into() } else { "b".into() },
    }
}

/// Interpret the IR against a process: instantiate, replay the align chains,
/// wire the edges, then route — the same path [`crate::elaborate`] takes.
///
/// # Errors
/// Generator failures surface as [`crate::elaborate::ElabError`].
pub fn elaborate_ir(
    ir: &GenIr,
    pdk: &Pdk,
    cfg: &ElabConfig,
) -> Result<Elaborated, crate::elaborate::ElabError> {
    use crate::elaborate::ElabError;
    let built = build_with(pdk, ir.ports.clone(), |c| {
        let mut placed: Vec<Option<macro_master::Instance>> = (0..ir.instances.len())
            .map(|_| None)
            .collect();
        for p in &ir.place {
            let spec = &ir.instances[p.inst];
            let mut inst = match (spec.kind, spec.legs) {
                (DeviceKind::Resistor, _) => c.instantiate(
                    &spec.name,
                    &variants::Res { w: spec.w, len: spec.l },
                )?,
                (kind, 2) => c.instantiate(
                    &spec.name,
                    &variants::MatchedPair {
                        kind,
                        w: spec.w,
                        l: spec.l,
                        nf_each: spec.nf,
                        pattern: None,
                    },
                )?,
                (kind, _) => c.instantiate(
                    &spec.name,
                    &variants::Mos::new(kind, spec.w, spec.l, spec.nf),
                )?,
            };
            for a in &p.aligns {
                let gap = match &a.gap {
                    IrGap::Rule(name, default) => c.process().rule(name, *default),
                    IrGap::Nm(v) => *v,
                };
                let reference = placed[a.reference]
                    .as_ref()
                    .expect("IR references earlier instances only");
                inst.align(a.mode, reference, gap);
            }
            placed[p.inst] = Some(c.place(inst)?);
        }
        for (a, b) in &ir.edges {
            c.connect(a, b);
        }
        Ok::<(), GenError>(())
    })
    .map_err(ElabError::Gen)?;
    route_built(built, pdk, cfg)
}

/// Pretty-print the IR as a standalone macroMaster [`Composition`] — the
/// "substrate3 source" a user can read, edit, and re-elaborate on any PDK.
#[must_use]
pub fn to_rust(ir: &GenIr) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "//! Generated by `philis emit` — PDK-agnostic; edit freely.");
    let _ = writeln!(s, "use macro_master::{{variants::{{MatchedPair, Mos, Res}}, AlignMode, Block, CompBuilder,");
    let _ = writeln!(s, "    Composition, GenError, InOut, Io, PortInfo, Process, Signal}};");
    let _ = writeln!(s, "use pnr_core::DeviceKind;\n");
    let _ = writeln!(s, "#[derive(Default)]\npub struct EmittedIo {{");
    for p in &ir.ports {
        let _ = writeln!(s, "    pub {p}: InOut<Signal>,");
    }
    let _ = writeln!(s, "}}\nimpl Io for EmittedIo {{");
    let _ = writeln!(s, "    fn ports(&self) -> Vec<PortInfo> {{\n        vec![");
    for p in &ir.ports {
        let _ = writeln!(s, "            self.{p}.port(\"{p}\"),");
    }
    let _ = writeln!(s, "        ]\n    }}\n}}\n");
    let _ = writeln!(s, "pub struct {};\nimpl Block for {} {{", ir.name, ir.name);
    let _ = writeln!(s, "    type Io = EmittedIo;");
    let _ = writeln!(s, "    fn name(&self) -> String {{ \"{}\".into() }}\n}}\n", ir.name);
    let _ = writeln!(s, "impl Composition for {} {{", ir.name);
    let _ = writeln!(
        s,
        "    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {{"
    );
    let var = |i: usize| format!("i{i}");
    for p in &ir.place {
        let inst = &ir.instances[p.inst];
        let ctor = match (inst.kind, inst.legs) {
            (DeviceKind::Resistor, _) => format!("Res {{ w: {}, len: {} }}", inst.w, inst.l),
            (k, 2) => format!(
                "MatchedPair {{ kind: DeviceKind::{k:?}, w: {}, l: {}, nf_each: {}, pattern: None }}",
                inst.w, inst.l, inst.nf
            ),
            (k, _) => format!(
                "Mos::new(DeviceKind::{k:?}, {}, {}, {})",
                inst.w, inst.l, inst.nf
            ),
        };
        let _ = writeln!(s, "        let mut {} = c.instantiate(\"{}\", &{ctor})?;", var(p.inst), inst.name);
        for a in &p.aligns {
            let gap = match &a.gap {
                IrGap::Rule(name, d) => format!("c.process().rule(\"{name}\", {d})"),
                IrGap::Nm(v) => format!("{v} /* unattributed: PDK-specific residue */"),
            };
            let _ = writeln!(
                s,
                "        {}.align(AlignMode::{:?}, &{}, {gap});",
                var(p.inst),
                a.mode,
                var(a.reference)
            );
        }
        let _ = writeln!(s, "        let {} = c.place({})?;", var(p.inst), var(p.inst));
        let _ = writeln!(s, "        let _ = &{};", var(p.inst));
    }
    for (a, b) in &ir.edges {
        let _ = writeln!(s, "        c.connect(\"{a}\", \"{b}\");");
    }
    let _ = writeln!(s, "        Ok(())\n    }}\n}}");
    s
}

fn pdk_grid(pdk: &Pdk) -> i32 {
    use pnr_core::Process as _;
    pdk.grid()
}

fn process_rule(pdk: &Pdk, name: &str, default: i32) -> i32 {
    use pnr_core::Process as _;
    pdk.rule(name, default)
}
