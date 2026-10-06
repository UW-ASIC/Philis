//! SPICE front end → [`pnr_core::Netlist`] (FLOW-07): `.subckt` hierarchy
//! flattened, `.param` expressions evaluated, R/C/L values kept, `V I E F G H
//! B K` cards kept as evidence, and each `X` target classified by the file's
//! sub-circuits, then the deck's model table, then a model-token rule — never
//! by a default. [`retarget`] then fills in what a generic netlist leaves to
//! the PDK (ideal R/C, `nfin`, a FET's missing W/L).

use std::collections::{HashMap, HashSet};

use pnr_core::{Device, DeviceKind, Net, NetId, Netlist, SourceCard, SubcktInst};

/// What a MOS card's `W` means. W/nf/m semantics are PDK- and tool-specific,
/// so the convention is an input and is normalised once, here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeConvention {
    /// SPICE/BSIM4: `W` is the instance's total width over its `nf` fingers.
    #[default]
    Spice,
    /// `W` is one finger's width; stored as `W·nf`.
    PerFinger,
}

/// How [`spice_with`] reads a netlist.
#[derive(Clone, Debug, Default)]
pub struct ParseOptions {
    /// How a MOS card's `w` relates to its `nf` fingers; normalised to the
    /// SPICE total before the [`Device`] is stored.
    pub size: SizeConvention,
    /// Line 1 is a title (a simulator deck), not a card.
    pub title_line: bool,
    /// The top sub-circuit (case-insensitive). When set it wins over the
    /// file-level cards, which are then dropped (a simulator deck's
    /// testbench around its DUT); needed when the file has no file-level cards
    /// and several uninstantiated `.subckt`s.
    pub top: Option<String>,
    /// Model → kind, from the deck ([`crate::model_table`]): an `X` target
    /// naming one (exactly or behind a vendor `__` prefix) is that device.
    pub models: Vec<(String, DeviceKind)>,
}

/// What a parse read but did not act on.
#[derive(Clone, Debug, Default)]
pub struct ParseReport {
    /// `.include`, `.lib`, `.model`, `.option(s)`, `.temp`, `.end`, analysis
    /// and `.meas` lines, as written: not errors, not circuit.
    pub ignored: Vec<String>,
}

/// Parse a SPICE netlist into the internal [`Netlist`], `W` read as
/// [`SizeConvention::Spice`], no deck model table.
pub fn spice(text: &str) -> Result<Netlist, String> {
    spice_with(text, &ParseOptions::default())
}

/// [`spice_report`] without the report.
pub fn spice_with(text: &str, opts: &ParseOptions) -> Result<Netlist, String> {
    spice_report(text, opts).map(|(nl, _)| nl)
}

/// Parse a SPICE netlist into the internal [`Netlist`].
///
/// The whole pipeline's input boundary: after this, everything is the internal
/// SoA model (parse-don't-validate). Nets are interned to [`NetId`]s in
/// first-seen card order, keyed case-insensitively and displayed as first
/// spelled; each [`Device`] carries its terminals in G,D,S,B order (for FETs)
/// and its numeric params (W/L in `nm`, `nf`/`stack`/`m` as plain counts, R/C/L
/// letter-card values as `r_mohm`/`c_af`/`ind_ph`). A MOS `w` is stored as the
/// SPICE instance total (`pnr_core::MosSize`), so under
/// [`SizeConvention::PerFinger`] it is the written `w` times `nf`.
///
/// The top is `opts.top` if set (file-level cards and sources dropped), else
/// the file-level cards if any, else the unique uninstantiated `.subckt` (a
/// `.subckt` top's formals become [`Netlist::ports`]). Below
/// it, instance `X1`'s device `XM2` is `X1/XM2` and its internal net `n1` is
/// `X1/n1`; `.global` nets and `0` are never prefixed; instance `m=k`
/// multiplies every child's `m`.
///
/// # Errors
/// An unresolved parameter (named), an `X` target that is neither a `.subckt`
/// here nor a known model, a port-count mismatch, nested or unclosed
/// `.subckt`, a MOS `w`/`l` ≤ 0, more than 65 535 nets or devices, or two
/// devices whose simulator instance names collide.
pub fn spice_report(text: &str, opts: &ParseOptions) -> Result<(Netlist, ParseReport), String> {
    let mut report = ParseReport::default();
    let mut file_params = Scope::new();
    let mut globals: HashSet<String> = HashSet::from(["0".to_string()]);
    let mut subckts: Vec<Subckt> = Vec::new();
    let mut top_cards: Vec<Vec<String>> = Vec::new();
    let mut open: Option<Subckt> = None;
    for stmt in statements(text, opts.title_line) {
        let toks = tokens(&stmt);
        let Some(head) = toks.first() else { continue };
        let key = head.to_ascii_lowercase();
        if !key.starts_with('.') {
            open.as_mut().map_or(&mut top_cards, |s| &mut s.cards).push(toks);
            continue;
        }
        let (pos, kv) = split(&toks[1..]);
        match key.as_str() {
            ".subckt" => {
                if open.is_some() {
                    return Err("nested .subckt is not supported".into());
                }
                let (name, ports) = pos.split_first().ok_or(".subckt without a name")?;
                let kv = kv.into_iter().map(|(k, v)| (k, v.to_string())).collect();
                open = Some(Subckt { name: (*name).to_string(), ports: ports.iter().map(|p| (*p).to_string()).collect(), defaults: kv, cards: Vec::new() });
            }
            ".ends" => subckts.push(open.take().ok_or(".ends without .subckt")?),
            ".param" => {
                for (k, v) in kv {
                    match &mut open {
                        Some(s) => s.defaults.push((k, v.to_string())),
                        None => {
                            let x = value(v, &file_params)?;
                            file_params.insert(k, x);
                        }
                    }
                }
            }
            ".global" => globals.extend(pos.iter().map(|n| n.to_ascii_lowercase())),
            _ => report.ignored.push(stmt.clone()),
        }
    }
    if let Some(s) = open {
        return Err(format!(".subckt {} has no .ends", s.name));
    }

    let mut by_name = HashMap::new();
    for (i, s) in subckts.iter().enumerate() {
        if by_name.insert(s.name.to_ascii_lowercase(), i).is_some() {
            return Err(format!("duplicate .subckt {}", s.name));
        }
    }
    let top = match &opts.top {
        Some(t) => Some(*by_name.get(&t.to_ascii_lowercase()).ok_or_else(|| format!("top sub-circuit {t} not found"))?),
        None if !top_cards.is_empty() => None,
        None => {
            let called: HashSet<String> = subckts.iter().flat_map(|s| &s.cards).filter(|c| c[0].starts_with(['x', 'X'])).filter_map(|c| split(&c[1..]).0.last().map(|t| t.to_ascii_lowercase())).collect();
            let roots: Vec<usize> = (0..subckts.len()).filter(|&i| !called.contains(&subckts[i].name.to_ascii_lowercase())).collect();
            match roots[..] {
                [] => return Err("no devices parsed".into()),
                [r] => Some(r),
                _ => {
                    let names: Vec<&str> = roots.iter().map(|&r| subckts[r].name.as_str()).collect();
                    return Err(format!("several top-level sub-circuits ({}): choose one with ParseOptions::top", names.join(", ")));
                }
            }
        }
    };

    let mut flat = Flat { opts, subckts: &subckts, by_name, globals, file_params, nl: Netlist::default(), net_index: HashMap::new() };
    let mut scope = flat.file_params.clone();
    if let Some(t) = top {
        for (k, v) in &subckts[t].defaults {
            let x = value(v, &scope)?;
            scope.insert(k.to_ascii_lowercase(), x);
        }
    }
    let frame = Frame { path: String::new(), inst: None, ports: HashMap::new(), scope, m: 1 };
    let cards = top.map_or(&top_cards, |t| &subckts[t].cards);
    flat.expand(cards, &frame, &mut top.into_iter().collect())?;
    if let Some(t) = top {
        flat.nl.ports = subckts[t].ports.iter().map(|p| flat.net(p.clone())).collect::<Result<_, _>>()?;
    }

    let nl = flat.nl;
    if nl.devices.is_empty() {
        return Err("no devices parsed".to_string());
    }
    // Device indices are `u16` downstream (`annotator` ranges over
    // `0..len as u16`): 65 536 would wrap to an empty range.
    if nl.devices.len() > usize::from(u16::MAX) {
        return Err(format!("{} devices: more than 65 535 is not supported", nl.devices.len()));
    }
    // The op deck names each device `instance_name` (`/` → `__`); ngspice
    // matches case-insensitively, so two that agree there are one row.
    let mut seen = HashSet::new();
    for d in &nl.devices {
        if !seen.insert(crate::oppoint::instance_name(d).to_ascii_lowercase()) {
            return Err(format!("device {}: its simulator name {} collides with another device's", d.name, crate::oppoint::instance_name(d)));
        }
    }
    Ok((nl, report))
}

/// Parameter name (lower case) → value, base SI.
type Scope = HashMap<String, f64>;

struct Subckt {
    name: String,
    ports: Vec<String>,
    /// `params:` defaults and in-body `.param`s, unevaluated, in order.
    defaults: Vec<(String, String)>,
    cards: Vec<Vec<String>>,
}

/// One level of the expansion.
struct Frame {
    /// `""` at the top, else the instance path (`"X1/X3"`).
    path: String,
    inst: Option<u32>,
    /// Formal port (lower case) → actual net.
    ports: HashMap<String, NetId>,
    scope: Scope,
    /// Product of the enclosing instances' `m`.
    m: i64,
}

struct Flat<'a> {
    opts: &'a ParseOptions,
    subckts: &'a [Subckt],
    by_name: HashMap<String, usize>,
    globals: HashSet<String>,
    file_params: Scope,
    nl: Netlist,
    net_index: HashMap<String, NetId>,
}

impl Flat<'_> {
    /// Intern a net by its case-insensitive key; the first spelling displays.
    fn net(&mut self, display: String) -> Result<NetId, String> {
        let key = display.to_ascii_lowercase();
        if let Some(&id) = self.net_index.get(&key) {
            return Ok(id);
        }
        let id = u16::try_from(self.nl.nets.len()).ok().filter(|&i| i < u16::MAX).ok_or("more than 65 535 nets is not supported")?;
        let id = NetId(id);
        self.nl.nets.push(Net { name: display });
        self.net_index.insert(key, id);
        Ok(id)
    }

    /// A node token inside frame `f`: a formal port's actual, a global or a
    /// top-level net by name, else the instance's internal `path/net`.
    fn node(&mut self, f: &Frame, n: &str) -> Result<NetId, String> {
        let k = n.to_ascii_lowercase();
        if let Some(&id) = f.ports.get(&k) {
            return Ok(id);
        }
        if f.path.is_empty() || self.globals.contains(&k) {
            return self.net(n.to_string());
        }
        self.net(format!("{}/{n}", f.path))
    }

    fn expand(&mut self, cards: &[Vec<String>], f: &Frame, stack: &mut Vec<usize>) -> Result<(), String> {
        for card in cards {
            let name = if f.path.is_empty() { card[0].clone() } else { format!("{}/{}", f.path, card[0]) };
            let (pos, kv) = split(&card[1..]);
            let letter = card[0].chars().next().unwrap_or(' ').to_ascii_lowercase();
            match letter {
                'v' | 'i' | 'e' | 'f' | 'g' | 'h' | 'b' | 'k' => self.source(f, name, letter, &pos, &kv)?,
                'x' | 'm' | 'q' => {
                    let Some((&target, nodes)) = pos.split_last() else { return Err(format!("device `{name}`: too few tokens")) };
                    if letter == 'x' {
                        if let Some(&si) = self.by_name.get(&target.to_ascii_lowercase()) {
                            self.instantiate(f, name, si, nodes, &kv, stack)?;
                            continue;
                        }
                    }
                    let lower = target.to_ascii_lowercase();
                    let table = self.table(&lower);
                    let kind = match letter {
                        'm' => table.filter(|k| matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)).unwrap_or(if lower.contains("pfet") || lower.contains("pmos") { DeviceKind::Pmos } else { DeviceKind::Nmos }),
                        'q' => table.filter(|k| matches!(k, DeviceKind::Npn | DeviceKind::Pnp)).unwrap_or(if lower.contains("pnp") { DeviceKind::Pnp } else { DeviceKind::Npn }),
                        _ => table.or_else(|| token_kind(&lower)).ok_or_else(|| format!("device {name}: unknown model {target}: not a .subckt here and not a deck model"))?,
                    };
                    self.device(f, name, kind, target, nodes, &kv, None)?;
                }
                'r' | 'c' | 'l' | 'd' => {
                    if pos.len() < 2 {
                        return Err(format!("device `{name}`: too few tokens"));
                    }
                    // After the two nodes: the first value (R/C/L) and the
                    // identifiers; of two identifiers the first is a bulk node.
                    // A diode's trailing number is its SPICE3 area, dropped.
                    let mut rest = &pos[2..];
                    if letter == 'd' && rest.len() >= 2 && rest.last().is_some_and(|t| value(t, &f.scope).is_ok()) {
                        rest = &rest[..rest.len() - 1];
                    }
                    let (mut val, mut idents) = (None, Vec::new());
                    for &t in rest {
                        if val.is_none() && letter != 'd' {
                            if let Ok(v) = value(t, &f.scope) {
                                val = Some(v);
                                continue;
                            }
                        }
                        idents.push(t);
                    }
                    // `R1 a b resistor r=1k`: the value as a keyword.
                    if val.is_none() {
                        if let Some((_, v)) = kv.iter().find(|(k, _)| matches!((letter, k.as_str()), ('r', "r") | ('c', "c"))) {
                            val = Some(value(v, &f.scope)?);
                        }
                    }
                    let mut nodes = pos[..2].to_vec();
                    if idents.len() >= 2 {
                        nodes.push(idents[0]);
                    }
                    let model = idents.last().copied().unwrap_or("");
                    let (kind, key, scale) = match letter {
                        'r' => (DeviceKind::Resistor, "r_mohm", 1e3),
                        'c' => (DeviceKind::Capacitor, "c_af", 1e18),
                        'l' => (DeviceKind::Inductor, "ind_ph", 1e12),
                        _ => (DeviceKind::Diode, "", 0.0),
                    };
                    self.device(f, name, kind, model, &nodes, &kv, val.map(|v| (key, (v * scale).round() as i64)))?;
                }
                _ => return Err(format!("device `{name}`: unsupported card")),
            }
        }
        Ok(())
    }

    /// The deck table's kind for lower-case `model`: exact, or one is the
    /// other behind a vendor `__` prefix (as `Pdk::deck_model`).
    fn table(&self, model: &str) -> Option<DeviceKind> {
        let hit = |n: &str| n == model || n.ends_with(&format!("__{model}")) || model.ends_with(&format!("__{n}"));
        self.opts.models.iter().find(|(n, _)| hit(&n.to_ascii_lowercase())).map(|&(_, k)| k)
    }

    fn instantiate(&mut self, f: &Frame, name: String, si: usize, nodes: &[&str], kv: &[(String, &str)], stack: &mut Vec<usize>) -> Result<(), String> {
        let subckts = self.subckts;
        let sc = &subckts[si];
        if stack.contains(&si) {
            return Err(format!("instance {name}: .subckt {} instantiates itself", sc.name));
        }
        if nodes.len() != sc.ports.len() {
            return Err(format!("instance {name}: {} nodes, .subckt {} has {} ports", nodes.len(), sc.name, sc.ports.len()));
        }
        let actuals = nodes.iter().map(|n| self.node(f, n)).collect::<Result<Vec<_>, _>>()?;
        let inst = u32::try_from(self.nl.insts.len()).map_err(|_| "too many instances")?;
        self.nl.insts.push(SubcktInst { path: name.clone(), subckt: sc.name.clone(), parent: f.inst, ports: actuals.clone() });
        // Scope: instance overrides (in the caller's scope) → the .subckt's
        // defaults → file `.param`.
        let mut scope = self.file_params.clone();
        let mut m = f.m;
        let mut overridden = HashSet::new();
        for (k, v) in kv {
            let x = value(v, &f.scope)?;
            if k == "m" {
                m = m.saturating_mul(x.round() as i64);
            } else {
                scope.insert(k.clone(), x);
                overridden.insert(k.clone());
            }
        }
        for (k, v) in &sc.defaults {
            let k = k.to_ascii_lowercase();
            if !overridden.contains(&k) {
                let x = value(v, &scope)?;
                scope.insert(k, x);
            }
        }
        let ports = sc.ports.iter().map(|p| p.to_ascii_lowercase()).zip(actuals).collect();
        let child = Frame { path: name, inst: Some(inst), ports, scope, m };
        stack.push(si);
        self.expand(&sc.cards, &child, stack)?;
        stack.pop();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn device(&mut self, f: &Frame, name: String, kind: DeviceKind, model: &str, nodes: &[&str], kv: &[(String, &str)], val: Option<(&str, i64)>) -> Result<(), String> {
        let (term_names, min_nodes) = terminal_spec(kind);
        if nodes.len() < min_nodes {
            return Err(format!("device `{name}` ({model}): {} nodes, expected >= {min_nodes}", nodes.len()));
        }
        let mos = matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos);
        // SPICE lists MOS nodes as D G S [B]; emit terminals in G,D,S,B order
        // (the order `cells`/LVS expect). Others keep their listed order. A
        // 3-terminal MOS reuses the source net as bulk.
        let terminals: Vec<(String, NetId)> = if mos {
            let order = [(1, "G"), (0, "D"), (2, "S"), (if nodes.len() > 3 { 3 } else { 2 }, "B")];
            order.iter().map(|&(i, t)| Ok((t.to_string(), self.node(f, nodes[i])?))).collect::<Result<_, String>>()?
        } else {
            term_names.iter().zip(nodes).map(|(t, n)| Ok(((*t).to_string(), self.node(f, n)?))).collect::<Result<_, String>>()?
        };

        // Lengths (w/l) → nm; counts (nf/stack/m) → plain ints.
        let mut params: Vec<(String, i64)> = Vec::new();
        for (k, v) in kv {
            let x = match k.as_str() {
                "w" | "l" => i64::from(to_nm(value(v, &f.scope)?)),
                "nf" | "nfin" | "stack" | "m" | "multi" => value(v, &f.scope)?.round() as i64,
                _ => continue,
            };
            params.push((k.clone(), x));
        }
        if f.m != 1 {
            match params.iter_mut().find(|(k, _)| k == "m") {
                Some((_, m)) => *m = m.saturating_mul(f.m),
                None => params.push(("m".into(), f.m)),
            }
        }
        let p = |params: &[(String, i64)], k: &str| params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
        if mos {
            if let Some(bad) = ["w", "l"].into_iter().find(|&k| p(&params, k).is_some_and(|v| v <= 0)) {
                return Err(format!("device {name}: {bad} must be > 0"));
            }
            if self.opts.size == SizeConvention::PerFinger {
                let nf = p(&params, "nf").map_or(1, |v| v.max(1));
                if let Some((_, w)) = params.iter_mut().find(|(k, _)| k == "w") {
                    *w *= nf;
                }
            }
        }
        params.extend(val.map(|(k, v)| (k.to_string(), v)));
        self.nl.devices.push(Device { name, kind, model: model.to_string(), terminals, params });
        self.nl.device_inst.push(f.inst);
        Ok(())
    }

    fn source(&mut self, f: &Frame, name: String, letter: char, pos: &[&str], kv: &[(String, &str)]) -> Result<(), String> {
        // `K` couples inductors by name; `E`/`G` sense a second node pair,
        // unless behavioural (`value=`/`vol=`/`cur=`) or `poly(n)`, whose
        // controlling nodes are not interned.
        let behavioural = kv.iter().any(|(k, _)| matches!(k.as_str(), "value" | "vol" | "cur")) || pos.get(2).is_some_and(|t| t.to_ascii_lowercase().starts_with("poly("));
        let n = match letter {
            'k' => 0,
            'e' | 'g' if !behavioural => 4,
            _ => 2,
        };
        if pos.len() < n {
            return Err(format!("source `{name}`: too few tokens"));
        }
        let nodes = pos[..n].iter().map(|t| self.node(f, t)).collect::<Result<_, _>>()?;
        let rest = &pos[n..];
        let waveform = rest.iter().any(|t| matches!(t.split('(').next().unwrap_or("").to_ascii_lowercase().as_str(), "pulse" | "pwl" | "sin" | "exp"));
        let dc = match (letter, rest.iter().position(|t| t.eq_ignore_ascii_case("dc"))) {
            ('v' | 'i', Some(i)) => rest.get(i + 1).map(|t| value(t, &f.scope)).transpose()?,
            ('v' | 'i', None) => rest.first().and_then(|t| value(t, &f.scope).ok()),
            _ => None,
        };
        self.nl.sources.push(SourceCard { name, kind: letter.to_ascii_uppercase(), nodes, dc, waveform });
        Ok(())
    }
}

/// What a generic netlist leaves to the PDK, filled in from `pdk` ([`crate::run`]
/// calls it after parsing): an ideal `R`/`C` card (a value and no deck model, or
/// the bare word `resistor`/`res`/`capacitor`/`cap`) becomes the deck's resistor or
/// capacitor sized from its value; a MOS card without `w` takes `nfin` fins per
/// finger on a fin deck, else the deck's minimum width per finger, and one without
/// `l` the deck's minimum length. One note per change, one per card left ideal,
/// and one per source card (testbench evidence, never drawn).
pub fn retarget(nl: &mut Netlist, pdk: &verify::Pdk) -> Vec<String> {
    use pnr_core::Process as _;
    let get = |d: &Device, k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let fin_pitch = pdk.width("fin").zip(pdk.space("fin")).map(|(w, s)| i64::from(w + s));
    let ideal = |m: &str| matches!(m.to_ascii_lowercase().as_str(), "" | "resistor" | "res" | "capacitor" | "cap") && pdk.deck_model(m).is_none();
    let mut notes = Vec::new();
    for d in &mut nl.devices {
        let sized = get(d, "w").is_some() && get(d, "l").is_some();
        match d.kind {
            DeviceKind::Nmos | DeviceKind::Pmos => {
                let (l_min, w_min) = pdk.min_channel(d.kind == DeviceKind::Pmos, &d.model);
                // A finger as narrow as the generator draws by default (sky130 420 nm, not diff's 150).
                let w_min = w_min.max(cells::builder::dim(pdk, "min_finger_width"));
                let nf = get(d, "nf").unwrap_or(1).max(1);
                if get(d, "w").is_none() {
                    let (w, why) = match (fin_pitch, get(d, "nfin")) {
                        (Some(p), n) => (n.unwrap_or(1).max(1) * nf * p, format!("{} fin(s) × {nf} finger(s) at the {p} nm fin pitch", n.unwrap_or(1).max(1))),
                        (None, _) => (nf * i64::from(w_min), format!("the deck's minimum finger {w_min} nm × {nf} finger(s)")),
                    };
                    d.params.push(("w".into(), w));
                    notes.push(format!("{}: no w, drawn at w = {w} nm ({why})", d.name));
                }
                if get(d, "l").is_none() {
                    d.params.push(("l".into(), i64::from(l_min)));
                    notes.push(format!("{}: no l, drawn at the deck's minimum l = {l_min} nm", d.name));
                }
            }
            DeviceKind::Resistor if ideal(&d.model) => {
                let Some(recipe) = pdk.recipe("resistor", "") else {
                    notes.push(format!("{}: ideal resistor left as written: this PDK has no resistor", d.name));
                    continue;
                };
                let ov = verify::pdk::Overlay { pdk, recipe };
                let w = ov.rule("res_min_width", 0).max(ov.width("rpoly").unwrap_or(0));
                let ohm = get(d, "r_mohm").map(|v| v as f64 / 1e3).filter(|&r| r > 0.0);
                // The recipe's own model (body plus heads), else the deck's body sheet.
                let l = ohm.filter(|_| !sized && w > 0).and_then(|r| match cells::resistor::ResModel::of(&ov) {
                    Some(m) => m.seg_len(w, r, 1, 0, 1, i32::MAX),
                    None => ov.sheet_ohm("rpoly").filter(|&s| s > 0.0).map(|s| ((r * f64::from(w) / f64::from(s)).round() as i32).max(w)),
                });
                match (sized, l) {
                    (true, _) => notes.push(format!("{}: drawn as {}", d.name, ov.recipe.model)),
                    (false, Some(l)) => {
                        d.params.extend([("w".to_string(), i64::from(w)), ("l".to_string(), i64::from(l))]);
                        notes.push(format!("{}: ideal {} Ω drawn as {} w = {w} nm, l = {l} nm", d.name, ohm.unwrap_or(0.0), ov.recipe.model));
                    }
                    (false, None) => {
                        notes.push(format!("{}: ideal resistor left as written: no value, or none {} can be sized to", d.name, ov.recipe.model));
                        continue;
                    }
                }
                d.model = ov.recipe.model;
            }
            DeviceKind::Capacitor if ideal(&d.model) => {
                // The first recipe the deck's LVS recognises, the default first.
                let recipes = pdk.cell.get("capacitors").and_then(|t| t.get("recipes")).and_then(|r| r.as_object());
                let models = recipes.into_iter().flat_map(|r| r.values()).filter_map(|r| r.get("model")?.as_str());
                let recipe = std::iter::once("").chain(models).filter_map(|m| pdk.recipe("capacitor", m)).find(|r| pdk.deck_model(&r.model).is_some());
                let Some(recipe) = recipe else {
                    notes.push(format!("{}: ideal capacitor left as written: no capacitor this PDK's LVS recognises", d.name));
                    continue;
                };
                let rule = |k: &str| recipe.rules.iter().find(|(n, _)| n == k).map_or(0.0, |&(_, v)| f64::from(v));
                let (area, perim, dw) = (rule("c_area_af_um2"), rule("c_perim_af_um"), rule("c_dw_nm"));
                let ov = verify::pdk::Overlay { pdk, recipe };
                // A square plate: C = area·s² + 4·perim·s, s the drawn side plus `c_dw`.
                let side = get(d, "c_af").map(|c| c as f64).filter(|&c| c > 0.0 && area > 0.0 && !sized).map(|c| {
                    let s_um = (-4.0 * perim + (16.0 * perim * perim + 4.0 * area * c).sqrt()) / (2.0 * area);
                    ((s_um * 1e3 - dw).round() as i32).max(ov.width("plate").unwrap_or(0))
                });
                match (sized, side) {
                    (true, _) => notes.push(format!("{}: drawn as {}", d.name, ov.recipe.model)),
                    (false, Some(s)) => {
                        d.params.extend([("w".to_string(), i64::from(s)), ("l".to_string(), i64::from(s))]);
                        notes.push(format!("{}: ideal {} fF drawn as {} w = l = {s} nm", d.name, get(d, "c_af").unwrap_or(0) as f64 / 1e3, ov.recipe.model));
                    }
                    (false, None) => {
                        notes.push(format!("{}: ideal capacitor left as written: no value, or {} states no area capacitance", d.name, ov.recipe.model));
                        continue;
                    }
                }
                d.model = ov.recipe.model;
            }
            _ => {}
        }
    }
    notes.extend(nl.sources.iter().map(|s| format!("{}: {} card, testbench evidence only: not drawn", s.name, s.kind)));
    notes
}

/// Logical statements: `*` comment lines dropped, inline `;` and ` $ `
/// comments stripped, `.control … .endc` skipped, `+` continuations and
/// trailing-`\` joins folded into one line; line 1 skipped iff `title_line`.
fn statements(text: &str, title_line: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let (mut joined, mut control) = (false, false);
    for (i, raw) in text.lines().enumerate() {
        if i == 0 && title_line {
            continue;
        }
        let mut line = raw.trim();
        if line.starts_with('*') {
            continue;
        }
        if let Some(p) = line.find(';') {
            line = &line[..p];
        }
        if let Some(p) = line.find(" $ ").or_else(|| line.strip_suffix(" $").map(str::len)) {
            line = &line[..p];
        }
        let lower = line.trim().to_ascii_lowercase();
        if control {
            control = !lower.starts_with(".endc");
            continue;
        }
        if lower.starts_with(".control") {
            control = true;
            continue;
        }
        let (body, cont) = match line.trim().strip_suffix('\\') {
            Some(b) => (b.trim(), true),
            None => (line.trim(), false),
        };
        let more = body.strip_prefix('+').map(str::trim).or(joined.then_some(body));
        match (more, out.last_mut()) {
            (Some(m), Some(last)) => {
                last.push(' ');
                last.push_str(m);
            }
            _ if !body.is_empty() => out.push(body.to_string()),
            _ => {}
        }
        joined = cont;
    }
    out
}

/// Whitespace-split tokens, keeping `{…}`, `(…)` and `'…'` whole and gluing
/// `k = v` into `k=v`; a comma outside them separates like a space.
fn tokens(stmt: &str) -> Vec<String> {
    let mut raw: Vec<String> = Vec::new();
    let (mut cur, mut depth, mut quote) = (String::new(), 0i32, false);
    for c in stmt.chars() {
        match c {
            '\'' => quote = !quote,
            '{' | '(' if !quote => depth += 1,
            '}' | ')' if !quote => depth -= 1,
            _ => {}
        }
        if (c.is_whitespace() || c == ',') && depth <= 0 && !quote {
            if !cur.is_empty() {
                raw.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        raw.push(cur);
    }
    let mut out: Vec<String> = Vec::new();
    for t in raw {
        match out.last_mut() {
            Some(last) if t.starts_with('=') || last.ends_with('=') => last.push_str(&t),
            _ => out.push(t),
        }
    }
    out
}

/// Positional tokens and `key=value` pairs (keys lower case); a `params:`
/// keyword is dropped.
fn split(toks: &[String]) -> (Vec<&str>, Vec<(String, &str)>) {
    let (mut pos, mut kv) = (Vec::new(), Vec::new());
    for t in toks {
        match t.split_once('=') {
            Some((k, v)) => kv.push((k.to_ascii_lowercase(), v)),
            None if t.eq_ignore_ascii_case("params:") => {}
            None => pos.push(t.as_str()),
        }
    }
    (pos, kv)
}

/// Device family named by the `_`-separated tokens of a lower-case model
/// (whole tokens, so `reset_logic` and `window_cmp` match nothing).
/// `nfet`/`nmos`/`pfet`/`pmos` name a MOS, so a sky130-style netlist parses
/// without a deck table.
fn token_kind(model: &str) -> Option<DeviceKind> {
    model.split('_').find_map(|t| match t {
        "npn" => Some(DeviceKind::Npn),
        "pnp" => Some(DeviceKind::Pnp),
        "res" => Some(DeviceKind::Resistor),
        "cap" | "mim" => Some(DeviceKind::Capacitor),
        "diode" => Some(DeviceKind::Diode),
        "ind" => Some(DeviceKind::Inductor),
        "nfet" | "nmos" => Some(DeviceKind::Nmos),
        "pfet" | "pmos" => Some(DeviceKind::Pmos),
        _ => None,
    })
}

/// Terminal names + minimum node count per kind. MOS listed as D G S B (a
/// 3-terminal MOS takes the source as bulk); a BJT's optional 4th node is the
/// substrate `S`; a two-terminal device's optional 3rd is its body `B`.
fn terminal_spec(kind: DeviceKind) -> (&'static [&'static str], usize) {
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => (&["D", "G", "S", "B"], 3),
        DeviceKind::Resistor | DeviceKind::Capacitor | DeviceKind::Diode | DeviceKind::Inductor => (&["P", "N", "B"], 2),
        DeviceKind::Npn | DeviceKind::Pnp => (&["C", "B", "E", "S"], 3),
    }
}

/// A SPICE value in base SI: a number with SI suffix, `{expr}`, `'expr'`, or
/// a bare expression/identifier, resolved in `scope`. An unresolved
/// identifier is an error naming it, never 0.
fn value(tok: &str, scope: &Scope) -> Result<f64, String> {
    let t = tok.trim();
    let inner = t.strip_prefix('{').and_then(|s| s.strip_suffix('}')).or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))).unwrap_or(t);
    let mut e = Expr { s: inner.as_bytes(), i: 0, scope };
    let v = e.expr()?;
    e.ws();
    if e.i < e.s.len() {
        return Err(format!("value `{tok}`: unexpected `{}`", &inner[e.i..]));
    }
    Ok(v)
}

/// Recursive descent over `expr := term (('+'|'-') term)*`, `term := factor
/// (('*'|'/') factor)*`, `factor := unary (('**'|'^') factor)?`, `unary := '-'
/// unary | primary`, `primary := number | ident | ident '(' args ')' | '('
/// expr ')'`; functions `sqrt abs min max`.
struct Expr<'a> {
    s: &'a [u8],
    i: usize,
    scope: &'a Scope,
}

impl Expr<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, tok: &[u8]) -> bool {
        self.ws();
        let hit = self.s[self.i..].starts_with(tok);
        self.i += if hit { tok.len() } else { 0 };
        hit
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut v = self.term()?;
        loop {
            if self.eat(b"+") {
                v += self.term()?;
            } else if self.eat(b"-") {
                v -= self.term()?;
            } else {
                return Ok(v);
            }
        }
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut v = self.factor()?;
        loop {
            self.ws();
            if self.s[self.i..].starts_with(b"*") && !self.s[self.i..].starts_with(b"**") {
                self.i += 1;
                v *= self.factor()?;
            } else if self.eat(b"/") {
                v /= self.factor()?;
            } else {
                return Ok(v);
            }
        }
    }

    fn factor(&mut self) -> Result<f64, String> {
        let b = self.unary()?;
        if self.eat(b"**") || self.eat(b"^") {
            return Ok(b.powf(self.factor()?));
        }
        Ok(b)
    }

    fn unary(&mut self) -> Result<f64, String> {
        if self.eat(b"-") {
            return Ok(-self.unary()?);
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<f64, String> {
        self.ws();
        if self.eat(b"(") {
            let v = self.expr()?;
            return if self.eat(b")") { Ok(v) } else { Err("missing `)`".into()) };
        }
        let start = self.i;
        let c = self.s.get(start).copied().unwrap_or(0);
        if c.is_ascii_digit() || c == b'.' {
            return Ok(self.number());
        }
        if !(c.is_ascii_alphabetic() || c == b'_') {
            return Err(format!("expected a value at `{}`", String::from_utf8_lossy(&self.s[start..])));
        }
        while self.s.get(self.i).is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.') {
            self.i += 1;
        }
        let name = String::from_utf8_lossy(&self.s[start..self.i]).to_ascii_lowercase();
        if self.eat(b"(") {
            let mut args = vec![self.expr()?];
            while self.eat(b",") {
                args.push(self.expr()?);
            }
            if !self.eat(b")") {
                return Err(format!("{name}(: missing `)`"));
            }
            return match (name.as_str(), &args[..]) {
                ("sqrt", [x]) => Ok(x.sqrt()),
                ("abs", [x]) => Ok(x.abs()),
                ("min", [a, b]) => Ok(a.min(*b)),
                ("max", [a, b]) => Ok(a.max(*b)),
                _ => Err(format!("unknown function {name} of {} arguments", args.len())),
            };
        }
        self.scope.get(&name).copied().ok_or_else(|| format!("undefined parameter {name}"))
    }

    /// Digits, fraction, exponent, then an SI suffix (`meg t g k m u n p f
    /// a`); any further letters are a unit and ignored (`1pF`, `1.8V`).
    fn number(&mut self) -> f64 {
        let start = self.i;
        let digit = |s: &Self, i: usize| s.s.get(i).is_some_and(u8::is_ascii_digit);
        while digit(self, self.i) || self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
        }
        if matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(self.s.get(self.i + 1), Some(b'+' | b'-')));
            if digit(self, self.i + 1 + sign) {
                self.i += 1 + sign;
                while digit(self, self.i) {
                    self.i += 1;
                }
            }
        }
        let v: f64 = std::str::from_utf8(&self.s[start..self.i]).ok().and_then(|n| n.parse().ok()).unwrap_or(f64::NAN);
        let letters = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
            self.i += 1;
        }
        let suffix = self.s[letters..self.i].to_ascii_lowercase();
        // Dividing by the inverse keeps `10u` exactly the double nearest 1e-5.
        if suffix.starts_with(b"meg") {
            return v * 1e6;
        }
        match suffix.first() {
            Some(b't') => v * 1e12,
            Some(b'g') => v * 1e9,
            Some(b'k') => v * 1e3,
            Some(b'm') => v / 1e3,
            Some(b'u') => v / 1e6,
            Some(b'n') => v / 1e9,
            Some(b'p') => v / 1e12,
            Some(b'f') => v / 1e15,
            Some(b'a') => v / 1e18,
            _ => v,
        }
    }
}

/// Base-SI value → nm. Values >= 0.01 are bare micrometers (`"W=2"`);
/// smaller are meters (`"W=2u"`, already scaled).
fn to_nm(v: f64) -> i32 {
    if v.abs() >= 0.01 {
        (v * 1e3).round() as i32
    } else {
        (v * 1e9).round() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ponytail: one self-check over a real fixture shape — covers the parser's
    // load-bearing logic (net dedup, G,D,S,B reorder, e-notation → nm).
    #[test]
    fn five_transistor_ota_shape() {
        let deck = "\
.subckt five_transistor_ota vss vdd vout vinn vinp id
M5 id id vss vss nfet_01v8 L=150e-9 w=10.5e-7 nf=10
M2 vout net8 vdd vdd pfet_01v8 L=150e-9 w=10.5e-7 nf=20
.ends five_transistor_ota
";
        let nl = spice(deck).unwrap();
        assert_eq!(nl.devices.len(), 2);
        assert_eq!(nl.devices[0].kind, DeviceKind::Nmos);
        assert_eq!(nl.devices[1].kind, DeviceKind::Pmos);
        // G,D,S,B order: M5 gate = "id", drain = "id", source/bulk = "vss".
        let m5 = &nl.devices[0];
        assert_eq!(m5.terminals[0].0, "G");
        assert_eq!(m5.terminals[1].0, "D");
        let name_of = |id: NetId| nl.nets[id.0 as usize].name.clone();
        assert_eq!(name_of(m5.terminals[0].1), "id"); // G
        assert_eq!(name_of(m5.terminals[1].1), "id"); // D
        assert_eq!(name_of(m5.terminals[2].1), "vss"); // S
        assert_eq!(name_of(m5.terminals[3].1), "vss"); // B
        // W: 10.5e-7 m = 1050 nm; L: 150e-9 m = 150 nm; nf plain.
        let p = |d: &Device, k: &str| d.params.iter().find(|(n, _)| n == k).map(|(_, v)| *v);
        assert_eq!(p(m5, "w"), Some(1050));
        assert_eq!(p(m5, "l"), Some(150));
        assert_eq!(p(m5, "nf"), Some(10));
        // Nets: id, vss, vout, net8, vdd from the cards, then the unused
        // ports vinn, vinp → 7 distinct.
        assert_eq!(nl.nets.len(), 7);
        assert_eq!(nl.ports.len(), 6);
    }

    fn param(d: &Device, k: &str) -> Option<i64> {
        d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v)
    }

    #[test]
    fn subckt_ports_and_hierarchy() {
        let deck = "\
.subckt inv a y vdd vss
XM1 y a n1 vss nfet_01v8 W=1u L=0.15u
XM2 y a vdd vdd pfet_01v8 W=2u L=0.15u
R1 n1 vss 1k
.ends
.subckt buf in out VDD VSS
X1 in mid VDD VSS inv
X2 mid out VDD VSS inv m=2
.ends buf
";
        let nl = spice(deck).unwrap();
        let names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["X1/XM1", "X1/XM2", "X1/R1", "X2/XM1", "X2/XM2", "X2/R1"]);
        let net = |n: &str| nl.nets.iter().position(|x| x.name == n).map(|i| NetId(i as u16));
        assert!(net("X1/n1").is_some() && net("X2/n1").is_some(), "internal nets are per instance");
        assert_eq!(net("n1"), None);
        let ports: Vec<&str> = nl.ports.iter().map(|p| nl.nets[p.0 as usize].name.as_str()).collect();
        assert_eq!(ports, ["in", "out", "VDD", "VSS"]);
        assert_eq!(nl.insts.len(), 2);
        assert_eq!(nl.insts[0].path, "X1");
        assert_eq!(nl.insts[0].subckt, "inv");
        assert_eq!(nl.insts[0].parent, None);
        assert_eq!(nl.insts[1].ports, [net("mid").unwrap(), net("out").unwrap(), net("VDD").unwrap(), net("VSS").unwrap()]);
        assert_eq!(nl.device_inst, [Some(0), Some(0), Some(0), Some(1), Some(1), Some(1)]);
        // Formals map to actuals: X1/XM1's gate is `in`, its source the internal net.
        let g = |d: &Device, t: &str| d.terminals.iter().find(|(n, _)| n == t).map(|&(_, id)| id);
        assert_eq!(g(&nl.devices[0], "G"), net("in"));
        assert_eq!(g(&nl.devices[0], "S"), net("X1/n1"));
        // Instance `m=2` multiplies every child.
        assert_eq!(param(&nl.devices[3], "m"), Some(2));
        assert_eq!(param(&nl.devices[0], "m"), None);
        // Port-count mismatch is an error.
        let bad = deck.replace("X2 mid out VDD VSS inv", "X2 mid out VDD inv");
        assert!(spice(&bad).err().unwrap().contains("ports"), "{:?}", spice(&bad).err());
        assert_eq!(spice(".subckt a x\n.subckt b y\n.ends\n.ends\n").err().unwrap(), "nested .subckt is not supported");
    }

    #[test]
    fn params_and_expressions() {
        let deck = ".param wu=0.5u\n.subckt t d g s\nXM1 d g s s nfet_01v8 W={2*wu} L='wu/2+0.1u'\n.ends\n";
        let nl = spice(deck).unwrap();
        assert_eq!(param(&nl.devices[0], "w"), Some(1000));
        assert_eq!(param(&nl.devices[0], "l"), Some(350));
        let err = spice(".subckt t d g s\nXM1 d g s s nfet_01v8 W=wx L=1u\n.ends\n").err().unwrap();
        assert!(err.contains("wx"), "{err}");
        // Scope: instance override → .subckt default → file .param.
        let deck = "\
.param wu=1u lu=2u
.subckt cell d g s params: wu=3u
XM1 d g s s nfet_01v8 W={wu} L={lu}
.ends
.subckt top a b c
X1 a b c cell
X2 a b c cell wu={2*lu}
.ends
";
        let nl = spice(deck).unwrap();
        assert_eq!(param(&nl.devices[0], "w"), Some(3000), "default beats file .param");
        assert_eq!(param(&nl.devices[0], "l"), Some(2000));
        assert_eq!(param(&nl.devices[1], "w"), Some(4000), "override beats default");
        // Grammar: power, unary minus, functions.
        let s = Scope::from([("a".to_string(), 4.0)]);
        assert_eq!(value("{sqrt(a)*2**3 - -1}", &s), Ok(17.0));
        assert_eq!(value("{max(a, 1meg)/min(2, abs(-4))}", &s), Ok(5e5));
        assert_eq!(value("10pF", &s), Ok(10e-12));
        // w ≤ 0 after evaluation is an error for a MOS.
        assert!(spice(".subckt t d g s\nXM1 d g s s nfet_01v8 W={1u-1u} L=1u\n.ends\n").err().unwrap().contains("w must be > 0"));
    }

    #[test]
    fn rc_values() {
        let nl = spice("R1 a b 10k\nC1 a b 1p\nL1 a c 2n\nR2 a b sub rmod\n").unwrap();
        assert_eq!(param(&nl.devices[0], "r_mohm"), Some(10_000_000));
        assert_eq!(param(&nl.devices[1], "c_af"), Some(1_000_000));
        assert_eq!(param(&nl.devices[2], "ind_ph"), Some(2_000));
        assert_eq!(nl.devices[0].kind, DeviceKind::Resistor);
        assert_eq!(nl.devices[2].kind, DeviceKind::Inductor);
        // No value, two identifiers: a body node, then the model.
        assert_eq!(nl.devices[3].model, "rmod");
        assert_eq!(nl.devices[3].terminals.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(), ["P", "N", "B"]);
        assert_eq!(param(&nl.devices[3], "r_mohm"), None);
    }

    #[test]
    fn nets_are_case_insensitive() {
        let nl = spice("M1 VDD g s s nfet_01v8 W=1u L=1u\nM2 vdd g s s nfet_01v8 W=1u L=1u\n").unwrap();
        assert_eq!(nl.devices[0].terminals[1].1, nl.devices[1].terminals[1].1);
        assert_eq!(nl.nets.iter().filter(|n| n.name.eq_ignore_ascii_case("vdd")).count(), 1);
        assert_eq!(nl.nets[nl.devices[1].terminals[1].1 .0 as usize].name, "VDD");
    }

    #[test]
    fn sources_are_evidence_not_devices() {
        let base = "M1 d g 0 0 nfet_01v8 W=1u L=1u\n";
        let nl = spice(&format!("{base}V1 vdd 0 1.8\nVclk clk 0 PULSE(0 1.8 0 1n 1n 5n 10n)\nI1 0 d dc 10u\n")).unwrap();
        assert_eq!(nl.devices.len(), spice(base).unwrap().devices.len());
        assert_eq!(nl.sources.len(), 3);
        assert_eq!(nl.sources[0].dc, Some(1.8));
        assert!(!nl.sources[0].waveform);
        assert!(nl.sources[1].waveform);
        assert_eq!(nl.sources[1].dc, None);
        assert_eq!(nl.sources[2].kind, 'I');
        assert_eq!(nl.sources[2].dc, Some(10e-6));
        // Behavioural and `poly` E/G cards have one node pair; linear ones two.
        let deck = format!("{base}E1 o1 0 value={{v(d)*2}}\nE2 o2 0 vol='v(g)'\nG1 o3 0 cur='1m'\nE3 o4 0 poly(1) d 0 0 1\nG2 o5 0 d g 1m\n");
        let nl = spice(&deck).unwrap();
        let nodes = |i: usize| nl.sources[i].nodes.iter().map(|n| nl.nets[n.0 as usize].name.as_str()).collect::<Vec<_>>();
        assert_eq!([nodes(0), nodes(1), nodes(2), nodes(3)], [vec!["o1", "0"], vec!["o2", "0"], vec!["o3", "0"], vec!["o4", "0"]]);
        assert_eq!(nodes(4), ["o5", "0", "d", "g"]);
        assert!(!nl.nets.iter().any(|n| n.name.to_ascii_lowercase().starts_with("poly")));
        assert!(spice(&format!("{base}E1 o1 0 d\n")).err().unwrap().contains("too few tokens"));
    }

    /// Field report: a FET card without W/L, an ideal R/C and a source must
    /// not fail the flow. On sky130 the R/C become the deck's resistor and the
    /// capacitor its LVS recognises, sized from the value.
    #[test]
    fn retarget_fills_what_a_generic_netlist_leaves_to_sky130() {
        use pnr_core::Process as _;
        let pdk = verify::Pdk::builtin("sky130").unwrap();
        let mut nl = spice("XM1 d g 0 0 nfet_01v8 nf=2\nR1 d out 10k\nC1 d 0 100f\nR2 a b resistor r=1k\nV1 vdd 0 1.8\n").unwrap();
        let notes = retarget(&mut nl, &pdk);
        let (l_min, _) = pdk.min_channel(false, "nfet_01v8");
        let d = &nl.devices;
        assert_eq!((param(&d[0], "w"), param(&d[0], "l")), (Some(2 * 420), Some(i64::from(l_min))));
        assert_eq!(d[1].model, "sky130_fd_pr__res_high_po");
        let ov = verify::pdk::Overlay { pdk: &pdk, recipe: pdk.recipe("resistor", &d[1].model).unwrap() };
        let ohm = cells::resistor::ResModel::of(&ov).unwrap().ohm(param(&d[1], "w").unwrap() as i32, param(&d[1], "l").unwrap() as i32);
        assert!((ohm - 10e3).abs() < 10.0, "R1 drawn at {ohm} Ω");
        assert_eq!(d[2].model, "sky130_fd_pr__cap_mim_m3_1");
        assert!(param(&d[2], "w").is_some_and(|w| w >= i64::from(ov.pdk.width("capm").unwrap_or(1))) && param(&d[2], "w") == param(&d[2], "l"));
        assert_eq!(param(&d[3], "r_mohm"), Some(1_000_000), "r= is R2's value");
        assert_eq!(d[3].model, "sky130_fd_pr__res_high_po");
        assert!(notes.iter().any(|n| n.starts_with("V1:")), "{notes:?}");
        assert_eq!(notes.len(), 6, "{notes:?}");
    }

    /// On ASAP7 a FET's width is its fins (`nfin`, else one per finger) and
    /// its length the deck's gate; an ideal R stays ideal, with a note.
    #[test]
    fn retarget_on_asap7_counts_fins_and_keeps_an_unbuildable_r_ideal() {
        let pdk = verify::Pdk::builtin("asap7").unwrap();
        let mut nl = spice("M1 d g 0 0 nmos_rvt nfin=4 l=20n\nM2 d g vdd vdd pmos_lvt nf=2\nR1 d out 10k\nC1 d 0 1f\n").unwrap();
        let notes = retarget(&mut nl, &pdk);
        let d = &nl.devices;
        assert_eq!((param(&d[0], "w"), param(&d[0], "l")), (Some(4 * 27), Some(20)));
        assert_eq!((param(&d[1], "w"), param(&d[1], "l")), (Some(2 * 27), Some(20)));
        assert_eq!((d[2].model.as_str(), param(&d[2], "w")), ("", None));
        assert!(notes.iter().any(|n| n.starts_with("R1: ideal resistor left as written")), "{notes:?}");
        assert!(notes.iter().any(|n| n.starts_with("C1: ideal capacitor left as written")), "{notes:?}");
    }

    #[test]
    fn diode_area_is_not_a_model() {
        let nl = spice("D1 a b dmod 2\nD2 a b sub dmod\n").unwrap();
        assert_eq!(nl.devices[0].model, "dmod");
        assert_eq!(nl.devices[0].terminals.len(), 2);
        assert_eq!(nl.devices[1].model, "dmod");
        assert_eq!(nl.devices[1].terminals.len(), 3);
    }

    #[test]
    fn more_than_u16_max_devices_or_nets_is_an_error() {
        // `n` resistors sharing two nets; `n` resistors to ground, `n + 1` nets.
        let shared = |n: usize| (0..n).map(|i| format!("R{i} a b 1\n")).collect::<String>();
        let fanout = |n: usize| (0..n).map(|i| format!("R{i} n{i} 0 1\n")).collect::<String>();
        let max = usize::from(u16::MAX);
        assert_eq!(spice(&shared(max)).unwrap().devices.len(), max);
        assert!(spice(&shared(max + 1)).err().unwrap().contains("65 535"));
        assert_eq!(spice(&fanout(max - 1)).unwrap().nets.len(), max);
        assert!(spice(&fanout(max)).err().unwrap().contains("65 535 nets"));
    }

    #[test]
    fn unknown_model_is_an_error() {
        let err = spice("X1 a b c myamp\n").err().unwrap();
        assert!(err.contains("unknown model"), "{err}");
    }

    #[test]
    fn token_rule_has_no_substring_false_positives() {
        assert_eq!(token_kind("reset_logic"), None);
        assert_eq!(token_kind("window_cmp"), None);
        assert!(spice("X1 a b reset_logic\n").err().unwrap().contains("unknown model"));
        assert!(spice("X1 a b window_cmp\n").err().unwrap().contains("unknown model"));
    }

    #[test]
    fn bjt_substrate_node_is_kept() {
        let nl = spice("Q1 c b e s pnp_05v5\n").unwrap();
        assert_eq!(nl.devices[0].kind, DeviceKind::Pnp);
        let t: Vec<&str> = nl.devices[0].terminals.iter().map(|t| t.0.as_str()).collect();
        assert_eq!(t, ["C", "B", "E", "S"]);
    }

    #[test]
    fn lexing_joins_comments_and_directives() {
        let deck = "title line\n.include models.lib\n.control\nrun\n.endc\nXM1 d g \\\n s b nfet_01v8 ; trailing\n+ W=1u $ unit\n+ L = 1u\n* comment\n.tran 1n 10n\n.end\n";
        let (nl, report) = spice_report(deck, &ParseOptions { title_line: true, ..Default::default() }).unwrap();
        assert_eq!(nl.devices.len(), 1);
        assert_eq!(param(&nl.devices[0], "w"), Some(1000));
        assert_eq!(param(&nl.devices[0], "l"), Some(1000));
        assert_eq!(report.ignored, [".include models.lib", ".tran 1n 10n", ".end"]);
        // Without `title_line`, line 1 is a card and an unknown one.
        assert!(spice(deck).is_err());
    }

    #[test]
    fn several_tops_need_a_choice() {
        let deck = ".subckt a x\nM1 x x 0 0 nfet_01v8 W=1u L=1u\n.ends\n.subckt b y\nM1 y y 0 0 pfet_01v8 W=1u L=1u\n.ends\n";
        assert!(spice(deck).err().unwrap().contains("a, b"));
        let nl = spice_with(deck, &ParseOptions { top: Some("B".into()), ..Default::default() }).unwrap();
        assert_eq!(nl.devices[0].kind, DeviceKind::Pmos);
        // An explicit top wins over a testbench's file-level cards.
        let bench = format!("{deck}X1 n a\nV1 n 0 1\n");
        assert_eq!(spice(&bench).unwrap().devices[0].name, "X1/M1");
        let nl = spice_with(&bench, &ParseOptions { top: Some("b".into()), ..Default::default() }).unwrap();
        assert_eq!((nl.devices[0].name.as_str(), nl.sources.len()), ("M1", 0));
    }

    #[test]
    fn simulator_name_collisions_are_errors() {
        let deck = ".subckt c a\nM1 a a 0 0 nfet_01v8 W=1u L=1u\n.ends\nX1 n c\nX1__M1 n n 0 0 nfet_01v8 W=1u L=1u\n";
        assert!(spice(deck).err().unwrap().contains("collides"));
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn per_finger_convention_stores_total_width() {
        let card = "M1 d g s b nfet_01v8 W=1u L=0.15u nf=4\n";
        let w = |size| {
            let nl = spice_with(card, &ParseOptions { size, ..Default::default() }).unwrap();
            nl.devices[0].params.iter().find(|(k, _)| k == "w").map(|&(_, v)| v)
        };
        assert_eq!(w(SizeConvention::PerFinger), Some(4000));
        assert_eq!(w(SizeConvention::Spice), Some(1000));
    }
}

#[cfg(test)]
mod x_instance_tests {
    use super::*;

    /// The kind and terminal names one `X` card parses to.
    fn x(card: &str) -> (DeviceKind, Vec<String>) {
        let nl = spice(card).unwrap();
        (nl.devices[0].kind, nl.devices[0].terminals.iter().map(|t| t.0.clone()).collect())
    }

    #[test]
    fn x_instances_are_classified_by_model_not_by_letter() {
        // A subcircuit call carries no device information in its letter, and in a
        // modern PDK almost everything is a subcircuit.
        assert_eq!(x("XM1 d g s b nfet_01v8").0, DeviceKind::Nmos);
        assert_eq!(x("XM3 d g s b pfet_01v8").0, DeviceKind::Pmos);
        // Regressions that used to silently become MOSFETs:
        // Polarity, not just family: collapsing these two onto one `Bjt` variant
        // drew, extracted and referenced the PNP as an NPN.
        assert_eq!(x("XQ1 c b e npn_05v5_1x1").0, DeviceKind::Npn);
        assert_eq!(x("XQ2 c b e pnp_05v5_W3p40L3p40").0, DeviceKind::Pnp);
        assert_eq!(x("XR1 a b res_generic_po").0, DeviceKind::Resistor);
        assert_eq!(x("XC1 a b cap_mim_m3_1").0, DeviceKind::Capacitor);
    }

    #[test]
    fn explicit_primitive_letters_still_win() {
        assert_eq!(x("M1 d g s b nfet_01v8").0, DeviceKind::Nmos);
        assert_eq!(x("Q1 c b e npn").0, DeviceKind::Npn);
        assert_eq!(x("Q2 c b e pnp").0, DeviceKind::Pnp);
        assert_eq!(x("R1 a b res").0, DeviceKind::Resistor);
    }

    #[test]
    fn a_two_terminal_resistor_parses() {
        // The exact `rc_filter` failure: classified NMOS, which demands >=3
        // nodes, so an ordinary 2-terminal resistor was rejected outright.
        assert_eq!(x("XR1 a b res_generic_po"), (DeviceKind::Resistor, vec!["P".to_string(), "N".to_string()]));
    }
}
