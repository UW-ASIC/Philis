//! # `emit` — decompile a solved [`Solution`] into a PDK-agnostic generator.
//!
//! The flow's output is nm-coordinates against one deck. This module lifts it
//! into a [`GenIr`]: instances (device family + params), *relational*
//! placement (align chains with rule-attributed gaps), and the net list —
//! the search's **decisions** without its **numbers**. The IR can be
//! interpreted against any [`Process`] ([`elaborate_ir`]) or pretty-printed
//! as macroMaster [`Composition`] source ([`to_rust`]).
//!
//! Scope: per-device instances and equal-leg merged pairs (`MatchedPair`); a
//! ratioed merged group returns [`EmitError::Unsupported`] rather than wrong
//! code. Symmetry axes ride in `Layout::axis` but are not lifted to
//! `place_mirrored`.

use std::collections::HashSet;

use annotator::{annotate};
use macro_master::{build_with, variants, AlignMode, GenError, Macros};
use pnr_core::{DeviceId, DeviceKind, Orient, Process as _};
use verify::Pdk;

use crate::elaborate::{route_built, ElabConfig, Elaborated};
use crate::{cellgen, Config, Solution};

/// One emitted device instance.
#[derive(Debug)]
pub struct IrInst {
    /// Instance name: the schematic device name, `a_b` for a merged pair.
    pub name: String,
    /// Device family: `Nmos`, `Pmos` or `Resistor` (the kinds with a variant).
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
    /// The solved orientation (`layout.orient`), applied about the bbox origin.
    pub orient: Orient,
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
    /// Which edge or side is aligned to the reference.
    pub mode: AlignMode,
    /// Index into [`GenIr::instances`] of the placed reference.
    pub reference: usize,
    /// Offset along the align direction.
    pub gap: IrGap,
}

/// Placement program for one instance: zero aligns = anchor at origin.
#[derive(Debug)]
pub struct IrPlace {
    /// Index into [`GenIr::instances`] of the instance placed.
    pub inst: usize,
    /// Align steps, applied in order.
    pub aligns: Vec<IrAlign>,
}

/// The decompiled generator — decisions only, no coordinates.
#[derive(Debug)]
pub struct GenIr {
    /// Generator (block) name; also the emitted Rust type name.
    pub name: String,
    /// Io port names: the netlist's `.subckt` ports, else every named net.
    pub ports: Vec<String>,
    /// One per layout cell, indexed like the layout.
    pub instances: Vec<IrInst>,
    /// In placement order; references always point at earlier entries.
    pub place: Vec<IrPlace>,
    /// Connect edges: `inst.term` ↔ an io port name or another `inst.term`.
    pub edges: Vec<(String, String)>,
}

/// Why a solution could not be decompiled.
#[derive(Debug)]
pub enum EmitError {
    /// The solution uses a feature the emitter cannot yet express faithfully.
    Unsupported(String),
}

/// Decompiles a solved placement against the deck it was solved on. The
/// layout may come from [`crate::run`] or any other producer — the emitter's
/// contract is `(netlist, layout)`, not the flow: cells are re-enumerated
/// from `netlist` with `cfg`'s annotation, and sizes come from the covering
/// unitization, else the schematic. `pdk` is used only to *attribute* gaps
/// to rule values — nothing PDK-specific survives into the IR except
/// unattributed residues.
///
/// # Errors
/// [`EmitError::Unsupported`] when the layout's cell count differs from the
/// re-enumerated cell table, a device has no W/L, or a cell is outside what
/// the IR expresses (quads, ratioed or mixed-kind pairs, kinds without a
/// macroMaster variant).
pub fn emit(
    netlist: &pnr_core::Netlist,
    layout: &pnr_core::Layout,
    pdk: &Pdk,
    cfg: &Config,
) -> Result<GenIr, EmitError> {
    let problem = annotate(netlist, &crate::annotation(pdk, &cfg.annotation));
    let cells = cellgen::enumerate(netlist, &Macros::default(), &problem.constraints, pdk, true);
    if layout.x.len() != cells.devices_of.len() {
        return Err(EmitError::Unsupported("layout does not match the cell table".into()));
    }
    // Sizes from the covering unitization (cellgen synthesizes one per
    // un-matched device).
    let size = |members: &[DeviceId]| -> Result<(i32, i32, u16), EmitError> {
        let d = &netlist.devices[members[0].0 as usize];
        let u = problem
            .constraints
            .unitization
            .iter()
            .find(|u| members.iter().all(|m| u.devices.contains(m)));
        // Equal legs only: a ratioed mirror (nf 1:2) needs per-leg counts
        // MatchedPair does not model yet.
        if members.len() == 2 && u.is_some_and(|u| u.dev_nf.windows(2).any(|w| w[0] != w[1])) {
            return Err(EmitError::Unsupported("ratioed merged group: MatchedPair models equal legs only".into()));
        }
        Ok(match u {
            Some(u) => {
                let slot = u.devices.iter().position(|x| x == &members[0]);
                let nf = slot
                    .and_then(|s| u.dev_nf.get(s))
                    .copied()
                    .unwrap_or(1)
                    .max(1);
                (u.unit_w.max(1), u.unit_l.max(1), nf)
            }
            // No covering unitization (unmatched device): the schematic size,
            // a MOS as `nf·m` fingers of `W_total/nf`. A missing size is not
            // guessed.
            None => narrow(d.mos_size().map(|s| (s.w_finger_nm(), s.l_nm, s.fingers())), d)?,
        })
    };
    lift(netlist, layout, pdk, &cells.devices_of, size)
}

/// Decompiles a [`crate::run`] result on its own cell table
/// ([`Solution::devices_of`]) and drawn sizes: a MOS at unit width
/// `W_total/nf/k` and `nf·m·k` fingers, `k` from [`Solution::folds`].
///
/// # Errors
/// As [`emit`], less the cell-count check (a `Solution` is consistent).
pub fn emit_solution(sol: &Solution, pdk: &Pdk) -> Result<GenIr, EmitError> {
    let fingers = |m: DeviceId| -> Option<(i64, i64, u32)> {
        let s = sol.netlist.devices[m.0 as usize].mos_size()?;
        Some(match sol.folds.get(m.0 as usize) {
            Some(&(k, wk)) if k > 0 => (i64::from(wk), s.l_nm, s.fingers() * u32::from(k)),
            _ => (s.w_finger_nm(), s.l_nm, s.fingers()),
        })
    };
    let size = |members: &[DeviceId]| -> Result<(i32, i32, u16), EmitError> {
        let d = &sol.netlist.devices[members[0].0 as usize];
        let mos = fingers(members[0]);
        let size = narrow(mos, d)?;
        // A resistor's one segment never equals a second leg's `None`: a
        // resistor pair is refused here too.
        if members.len() == 2 && fingers(members[1]).map(|f| f.2) != Some(mos.map_or(1, |f| f.2)) {
            return Err(EmitError::Unsupported("ratioed merged group: MatchedPair models equal legs only".into()));
        }
        Ok(size)
    };
    lift(&sol.netlist, &sol.layout, pdk, &sol.devices_of, size)
}

/// Returns the drawn `(unit w, l, fingers)` of `d`: `mos` when it is a MOS
/// size, else a resistor's positive `w`/`l` params at one segment, each
/// saturated into the IR's field width.
///
/// # Errors
/// [`EmitError::Unsupported`] when neither gives a size: a missing size is
/// not guessed.
fn narrow(mos: Option<(i64, i64, u32)>, d: &pnr_core::Device) -> Result<(i32, i32, u16), EmitError> {
    let p = |k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v).filter(|&v| v > 0);
    let size = match mos {
        Some(f) => Some(f),
        None if d.kind == DeviceKind::Resistor => p("w").zip(p("l")).map(|(w, l)| (w, l, 1)),
        None => None,
    };
    let Some((w, l, nf)) = size else {
        return Err(EmitError::Unsupported(format!("{}: no W/L in the netlist", d.name)));
    };
    Ok((w.min(i64::from(i32::MAX)) as i32, l.min(i64::from(i32::MAX)) as i32, nf.min(u32::from(u16::MAX)) as u16))
}

/// The IR of `layout`, whose cell `i` draws schematic devices `devices_of[i]`
/// at `size(devices_of[i])` = `(unit w, l, fingers per leg)`. `layout` holds
/// at least `devices_of.len()` cells. Instances are ordered like the cells;
/// placement is lifted bottom-left first into same-row (Bottom + ToTheRight)
/// or new-row (Left + Above) align pairs, with a gap within two grid steps
/// of `device_gap` attributed to that rule.
///
/// # Errors
/// [`EmitError::Unsupported`] for an empty or > 2-member cell, a kind with
/// no macroMaster variant, a mixed-kind pair, or whatever `size` refuses.
fn lift(
    netlist: &pnr_core::Netlist,
    layout: &pnr_core::Layout,
    pdk: &Pdk,
    devices_of: &[Vec<DeviceId>],
    size: impl Fn(&[DeviceId]) -> Result<(i32, i32, u16), EmitError>,
) -> Result<GenIr, EmitError> {
    // A 2-member cell is a merged matched pair → `MatchedPair` (per-leg pins);
    // >2 members (quads) still need a patterned quad variant.
    let mut instances = Vec::with_capacity(devices_of.len());
    for (i, members) in devices_of.iter().enumerate() {
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
        let legs = members.len() as u8;
        if legs == 2 && netlist.devices[members[1].0 as usize].kind != d.kind {
            return Err(EmitError::Unsupported(
                "mixed-kind merged group (implants would merge)".into(),
            ));
        }
        let (w, l, nf) = size(members)?;
        let name = if legs == 2 {
            format!("{}_{}", d.name, netlist.devices[members[1].0 as usize].name)
        } else {
            d.name.clone()
        };
        instances.push(IrInst {
            name,
            kind: d.kind,
            w,
            l,
            nf,
            legs,
            orient: layout.orient.get(i).copied().unwrap_or_default(),
        });
    }

    // Placement lift: order by solved bottom-left corner, then express each
    // instance relative to an earlier neighbour — same row ⇒ Bottom-align +
    // ToTheRight; new row ⇒ Left-align + Above. Gaps that land on the deck's
    // `device_gap` (within a grid step) are attributed to the rule.
    let l_ = layout;
    let n = instances.len();
    let grid = pdk.grid();
    let device_gap = pdk.rule("device_gap", 0);
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
            place.push(IrPlace {
                inst: i,
                aligns: Vec::new(),
            });
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
                IrAlign {
                    mode: AlignMode::Bottom,
                    reference: j,
                    gap: IrGap::Nm((l_.y[i] - l_.hh[i]) - (l_.y[j] - l_.hh[j])),
                },
                IrAlign {
                    mode: AlignMode::ToTheRight,
                    reference: j,
                    gap: attribute(gap),
                },
            ]
        } else {
            // New row: the nearest earlier cell below (any x), else the first anchor.
            let below = order[..k]
                .iter()
                .copied()
                .filter(|&j| l_.y[j] < l_.y[i])
                .max_by_key(|&j| l_.y[j])
                .unwrap_or(order[0]);
            let j = below;
            let gap = (l_.y[i] - l_.hh[i]) - (l_.y[j] + l_.hh[j]);
            vec![
                IrAlign {
                    mode: AlignMode::Left,
                    reference: j,
                    gap: IrGap::Nm((l_.x[i] - l_.hw[i]) - (l_.x[j] - l_.hw[j])),
                },
                IrAlign {
                    mode: AlignMode::Above,
                    reference: j,
                    gap: attribute(gap),
                },
            ]
        };
        place.push(IrPlace { inst: i, aligns });
    }

    // Connectivity: every device terminal to its net — a port by name, an
    // internal net through its first terminal (only ports are names
    // `build_with` knows). For a merged pair the member ordinal becomes the
    // leg suffix (`MatchedPair`'s `g1`/`g2`), except the shared body — one
    // `b` pin serves both legs.
    let net_name = |id: pnr_core::NetId| netlist.nets[id.0 as usize].name.clone();
    let ports: Vec<String> = if netlist.ports.is_empty() {
        netlist.nets.iter().map(|nt| nt.name.clone()).collect()
    } else {
        netlist.ports.iter().map(|&p| net_name(p)).collect()
    };
    let mut first: std::collections::HashMap<pnr_core::NetId, String> = std::collections::HashMap::new();
    let mut edges = Vec::new();
    for (inst, members) in instances.iter().zip(devices_of) {
        for (o, m) in members.iter().enumerate() {
            let d = &netlist.devices[m.0 as usize];
            for (idx, (t, net)) in d.terminals.iter().enumerate() {
                let mut term = terminal_port(inst.kind, t, idx);
                if inst.legs == 2 && term != "b" {
                    term = format!("{term}{}", o + 1);
                }
                let term = format!("{}.{}", inst.name, term);
                let name = net_name(*net);
                if ports.contains(&name) {
                    edges.push((term, name));
                } else if let Some(f) = first.get(net) {
                    edges.push((term, f.clone()));
                } else {
                    first.insert(*net, term);
                }
            }
        }
    }

    Ok(GenIr {
        name: "emitted".into(),
        ports,
        instances,
        place,
        edges,
    })
}

/// A Rust identifier for net `name`, unique within `taken`: lowercased,
/// chars outside `[a-z0-9_]` → `_`, `n_` before an empty or digit-led name,
/// `_` after a keyword, then `_2`, `_3`, … on a collision.
fn ident(name: &str, taken: &mut HashSet<String>) -> String {
    const KEYWORDS: [&str; 52] = [
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
        "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
        "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
    ];
    let mut id: String = name.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if id.is_empty() || id.starts_with(|c: char| c.is_ascii_digit()) {
        id.insert_str(0, "n_");
    }
    if KEYWORDS.contains(&id.as_str()) || id == "_" {
        id.push('_');
    }
    let mut out = id.clone();
    let mut k = 2;
    while !taken.insert(out.clone()) {
        out = format!("{id}_{k}");
        k += 1;
    }
    out
}

/// Returns the macroMaster variant's port name for schematic terminal `t` at
/// `position`: a MOS terminal lowercased, a two-terminal device `a` for the
/// first terminal and `b` for any other.
fn terminal_port(kind: DeviceKind, t: &str, position: usize) -> String {
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => t.to_ascii_lowercase(),
        // Two-terminal devices: variant io is a/b, schematic order decides.
        _ => {
            if position == 0 {
                "a".into()
            } else {
                "b".into()
            }
        }
    }
}

/// Interprets the IR against a process: instantiate, replay the align chains,
/// wire the edges, then route — the same path [`crate::elaborate`] takes.
///
/// # Errors
/// The generator failing against this process.
///
/// # Panics
/// On a malformed IR: a `place` entry or align reference indexing past
/// `instances`, or an align referencing an instance not yet placed.
pub fn elaborate_ir(ir: &GenIr, pdk: &Pdk, cfg: &ElabConfig) -> Result<Elaborated, GenError> {
    let built = build_with(pdk, ir.ports.clone(), |c| {
        let mut placed: Vec<Option<macro_master::Instance>> =
            (0..ir.instances.len()).map(|_| None).collect();
        for p in &ir.place {
            let spec = &ir.instances[p.inst];
            let mut inst = match (spec.kind, spec.legs) {
                (DeviceKind::Resistor, _) => c.instantiate(
                    &spec.name,
                    &variants::Res {
                        w: spec.w,
                        len: spec.l,
                    },
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
            inst.orient(spec.orient);
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
    })?;
    Ok(route_built(built, pdk, cfg))
}

/// Returns the IR pretty-printed as a standalone macroMaster [`Composition`]
/// — the "substrate3 source" a user can read, edit, and re-elaborate on any
/// PDK. Port names become unique snake-case fields; every string
/// the IR carries is written as an escaped Rust literal. Unattributed gaps
/// carry a comment marking them PDK-specific.
///
/// # Panics
/// When a `place` entry indexes past `instances`.
#[must_use]
pub fn to_rust(ir: &GenIr) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "//! Generated by `philis emit` — PDK-agnostic; edit freely."
    );
    let _ = writeln!(
        s,
        "use macro_master::{{variants::{{MatchedPair, Mos, Res}}, AlignMode, Block, CompBuilder,"
    );
    let _ = writeln!(
        s,
        "    Composition, GenError, InOut, Io, PortInfo, Process, Signal}};"
    );
    let _ = writeln!(s, "use pnr_core::{{DeviceKind, Orient}};\n");
    let _ = writeln!(s, "#[derive(Default)]\npub struct EmittedIo {{");
    let mut taken = HashSet::new();
    let fields: Vec<String> = ir.ports.iter().map(|p| ident(p, &mut taken)).collect();
    for f in &fields {
        let _ = writeln!(s, "    pub {f}: InOut<Signal>,");
    }
    let _ = writeln!(s, "}}\nimpl Io for EmittedIo {{");
    let _ = writeln!(s, "    fn ports(&self) -> Vec<PortInfo> {{\n        vec![");
    for (f, p) in fields.iter().zip(&ir.ports) {
        let _ = writeln!(s, "            self.{f}.port({p:?}),");
    }
    let _ = writeln!(s, "        ]\n    }}\n}}\n");
    let _ = writeln!(s, "pub struct {};\nimpl Block for {} {{", ir.name, ir.name);
    let _ = writeln!(s, "    type Io = EmittedIo;");
    let _ = writeln!(
        s,
        "    fn name(&self) -> String {{ \"{}\".into() }}\n}}\n",
        ir.name
    );
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
        let _ = writeln!(
            s,
            "        let mut {} = c.instantiate(\"{}\", &{ctor})?;",
            var(p.inst),
            inst.name
        );
        if inst.orient != Orient::R0 {
            let _ = writeln!(s, "        {}.orient(Orient::{:?});", var(p.inst), inst.orient);
        }
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
        let _ = writeln!(
            s,
            "        let {} = c.place({})?;",
            var(p.inst),
            var(p.inst)
        );
        let _ = writeln!(s, "        let _ = &{};", var(p.inst));
    }
    for (a, b) in &ir.edges {
        let _ = writeln!(s, "        c.connect(\"{a}\", \"{b}\");");
    }
    let _ = writeln!(s, "        Ok(())\n    }}\n}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_compile_shaped() {
        let mut taken = HashSet::new();
        let got: Vec<String> = ["in", "0", "a<1>", "vout-", "VDD", "vdd"].iter().map(|n| ident(n, &mut taken)).collect();
        assert_eq!(got, ["in_", "n_0", "a_1_", "vout_", "vdd", "vdd_2"]);
        for g in &got {
            let mut c = g.chars();
            assert!(c.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_') && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'), "{g}");
            assert!(ident(g, &mut HashSet::new()) == *g, "{g} is not a keyword");
        }
    }
}
