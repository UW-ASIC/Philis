//! SPICE netlist reader -> bipartite cell/net hypergraph.
//!
//! Reads any standard SPICE/Spectre file. Output is a
//! [`BipartiteHypergraph`]: two node partitions — cells and nets — with
//! pins as the edges between them. Two kinds of cell:
//!
//! * leaf PDK device -> [`DeviceRecord`], drawn later by the built-in
//!   generator its [`crate::pdk::DeviceDef`] names;
//! * subckt with a substrate3 `CellGenerator` impl -> kept whole as one
//!   macro node (never flattened). Caller passes those subckt names in
//!   `macros`; grouping n devices into one cell happens *only* this way.
//!
//! Everything else (subckts without an impl) is flattened to leaves.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use pnr_cells::device::DeviceRecord;
use pnr_cells::DeviceType;

use crate::pdk::Pdk;

// The hypergraph itself lives in pnr-cells (below the backend crates);
// this module is the SPICE parser that builds it.
pub use pnr_cells::netlist::{BipartiteHypergraph, CellNode, NetId};

const MAX_FLATTEN_DEPTH: usize = 50;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Syntax { line: usize, message: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Syntax { line, message } => write!(f, "line {line}: {message}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

fn syntax_err(line: usize, message: impl Into<String>) -> ParseError {
    ParseError::Syntax { line, message: message.into() }
}

// ---------------------------------------------------------------------------
// Value parsing
// ---------------------------------------------------------------------------

/// SPICE SI suffix table, ordered longest-first so "meg" matches before "m".
const SUFFIXES: &[(&str, f64)] = &[
    ("meg", 1e6),
    ("t", 1e12),
    ("g", 1e9),
    ("k", 1e3),
    ("m", 1e-3),
    ("u", 1e-6),
    ("n", 1e-9),
    ("p", 1e-12),
    ("f", 1e-15),
];

/// Parse a SPICE value with optional SI suffix into base SI units.
/// "10u" -> 10e-6, "270e-9" -> 270e-9, "3.3meg" -> 3.3e6.
fn parse_value(s: &str) -> f64 {
    let s = s.trim().trim_end_matches(',');
    let s_low = s.to_ascii_lowercase();

    // Engineering notation ("270e-9") — check before suffix stripping.
    if s_low.contains('e') && s_low.as_bytes().last().is_some_and(u8::is_ascii_digit) {
        if let Ok(v) = s_low.parse::<f64>() {
            return v;
        }
    }

    for &(suffix, mult) in SUFFIXES {
        if let Some(numeric) = s_low.strip_suffix(suffix) {
            if let Ok(v) = numeric.parse::<f64>() {
                return v * mult;
            }
        }
    }

    s_low.parse::<f64>().unwrap_or(0.0)
}

/// Base-SI value -> nm. Values >= 0.01 are bare micrometers (common in
/// PDK netlists: "W=2"); smaller ones are meters ("W=2u" already scaled).
fn to_nm(v: f64) -> i32 {
    if v.abs() >= 0.01 {
        (v * 1e3).round() as i32
    } else {
        (v * 1e9).round() as i32
    }
}

/// Terminal names per device type + minimum node count.
fn terminal_spec(dt: DeviceType) -> (&'static [&'static str], usize) {
    match dt {
        DeviceType::Nmos | DeviceType::Pmos | DeviceType::Ncap | DeviceType::Pcap => {
            // 3-terminal MOS accepted (no explicit bulk).
            (&["D", "G", "S", "B"], 3)
        }
        DeviceType::Res | DeviceType::Cap | DeviceType::Diode => (&["P", "N"], 2),
        DeviceType::Bjt => (&["C", "B", "E"], 3),
    }
}

// ---------------------------------------------------------------------------
// .param collection + expression evaluation
// ---------------------------------------------------------------------------

fn collect_params(lines: &[(usize, String)]) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for (_, line) in lines {
        if !line.trim().to_ascii_lowercase().starts_with(".param") {
            continue;
        }
        for tok in line.split_whitespace().skip(1) {
            if let Some((k, v)) = tok.split_once('=') {
                params.insert(k.to_ascii_lowercase(), v.trim_end_matches(',').to_string());
            }
        }
    }
    params
}

fn eval_spice_value(s: &str, params: &HashMap<String, String>) -> f64 {
    SpiceExpr::eval(s, params).unwrap_or_else(|| parse_value(s))
}

const MAX_EXPR_DEPTH: usize = 20;

/// Recursive-descent evaluator for SPICE `.param` expressions.
///
/// expr = term (('+'|'-') term)* ; term = power (('*'|'/') power)* ;
/// power = unary ('**' power)? ; primary = '(' expr ')' | FUNC(args) | NUMBER | IDENT
struct SpiceExpr<'a> {
    src: &'a [u8],
    pos: usize,
    params: &'a HashMap<String, String>,
    depth: usize,
}

impl SpiceExpr<'_> {
    fn eval(src: &str, params: &HashMap<String, String>) -> Option<f64> {
        Self::eval_at_depth(src, params, 0)
    }

    fn eval_at_depth(src: &str, params: &HashMap<String, String>, depth: usize) -> Option<f64> {
        if depth > MAX_EXPR_DEPTH {
            return None;
        }
        let s = src.trim().trim_matches(|c: char| c == '{' || c == '}').trim();
        if s.is_empty() {
            return None;
        }
        let mut e = SpiceExpr { src: s.as_bytes(), pos: 0, params, depth };
        let v = e.expr()?;
        e.ws();
        (e.pos >= e.src.len()).then_some(v)
    }

    fn ws(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos] == b' ' {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.src.get(self.pos).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some(b'+') => { self.pos += 1; v += self.term()?; }
                Some(b'-') => { self.pos += 1; v -= self.term()?; }
                _ => return Some(v),
            }
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        loop {
            self.ws();
            if self.at_starstar() {
                return Some(v);
            }
            match self.peek() {
                Some(b'*') => { self.pos += 1; v *= self.power()?; }
                Some(b'/') => { self.pos += 1; v /= self.power()?; }
                _ => return Some(v),
            }
        }
    }

    fn at_starstar(&self) -> bool {
        self.pos + 1 < self.src.len()
            && self.src[self.pos] == b'*'
            && self.src[self.pos + 1] == b'*'
    }

    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        self.ws();
        if self.at_starstar() {
            self.pos += 2;
            Some(base.powf(self.power()?))
        } else {
            Some(base)
        }
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek() {
            Some(b'-') => { self.pos += 1; Some(-self.primary()?) }
            Some(b'+') => { self.pos += 1; self.primary() }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<f64> {
        if self.eat(b'(') {
            let v = self.expr()?;
            self.eat(b')');
            return Some(v);
        }

        self.ws();
        let start = self.pos;

        if self.pos < self.src.len()
            && (self.src[self.pos].is_ascii_alphabetic() || self.src[self.pos] == b'_')
        {
            while self.pos < self.src.len()
                && (self.src[self.pos].is_ascii_alphanumeric() || self.src[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let name = std::str::from_utf8(&self.src[start..self.pos]).ok()?;

            if self.eat(b'(') {
                let a = self.expr()?;
                let v = match name.to_ascii_lowercase().as_str() {
                    "sqrt" => a.sqrt(),
                    "abs" => a.abs(),
                    "log" | "ln" => a.ln(),
                    "log10" => a.log10(),
                    "exp" => a.exp(),
                    "int" | "floor" => a.floor(),
                    "ceil" => a.ceil(),
                    "min" => { self.eat(b','); a.min(self.expr()?) }
                    "max" => { self.eat(b','); a.max(self.expr()?) }
                    "pow" => { self.eat(b','); a.powf(self.expr()?) }
                    _ => return None,
                };
                self.eat(b')');
                return Some(v);
            }

            let key = name.to_ascii_lowercase();
            return self
                .params
                .get(&key)
                .and_then(|v| SpiceExpr::eval_at_depth(v, self.params, self.depth + 1));
        }

        self.number()
    }

    fn number(&mut self) -> Option<f64> {
        self.ws();
        let start = self.pos;
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c.is_ascii_digit() || c == b'.' {
                self.pos += 1;
            } else if (c == b'e' || c == b'E') && self.pos > start {
                self.pos += 1;
                if self.pos < self.src.len()
                    && (self.src[self.pos] == b'-' || self.src[self.pos] == b'+')
                {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        // Greedily consume SI suffix attached to the number.
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        let tok = std::str::from_utf8(&self.src[start..self.pos]).ok()?;
        Some(parse_value(tok))
    }
}

// ---------------------------------------------------------------------------
// Subcircuit flattening (macro-aware)
// ---------------------------------------------------------------------------

struct SubcktDef {
    ports: Vec<String>,
    body: Vec<(usize, String)>,
}

/// Extract `.subckt`/`.ends` blocks; flatten non-macro instances to leaves.
/// Returns (flat lines, top name, top ports, subckt port lists for macros).
fn flatten_subcircuits(
    lines: &[(usize, String)],
    macros: &HashSet<String>,
) -> (Vec<(usize, String)>, String, Vec<String>, HashMap<String, Vec<String>>) {
    let mut subcircuits: HashMap<String, SubcktDef> = HashMap::new();
    let mut outside: Vec<(usize, String)> = Vec::new();
    let mut current: Option<(String, Vec<String>, Vec<(usize, String)>)> = None;
    let mut last_name = String::new();

    for (ln, line) in lines {
        let lower = line.trim().to_ascii_lowercase();

        if lower.starts_with(".subckt")
            || (lower.starts_with("subckt ") && !lower.starts_with("subckt_"))
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(&name) = parts.get(1) {
                let ports: Vec<String> = parts[2..]
                    .iter()
                    .filter(|p| !p.contains('='))
                    .map(std::string::ToString::to_string)
                    .collect();
                last_name = name.to_string();
                current = Some((name.to_string(), ports, Vec::new()));
            }
            continue;
        }

        if lower.starts_with(".ends") || lower == "ends" || lower.starts_with("ends ") {
            if let Some((name, ports, body)) = current.take() {
                subcircuits.insert(name, SubcktDef { ports, body });
            }
            continue;
        }

        if let Some((_, _, ref mut body)) = current {
            body.push((*ln, line.clone()));
        } else {
            outside.push((*ln, line.clone()));
        }
    }

    // Last subcircuit = top-level; everything else available for inlining.
    let (top_body, top_ports) = subcircuits
        .remove(&last_name)
        .map_or_else(|| (Vec::new(), Vec::new()), |s| (s.body, s.ports));
    let mut result = outside;
    result.extend(expand_instances(&top_body, &subcircuits, macros, 0));
    let port_map = subcircuits
        .into_iter()
        .map(|(k, v)| (k, v.ports))
        .collect();
    (result, last_name, top_ports, port_map)
}

fn expand_instances(
    lines: &[(usize, String)],
    subcircuits: &HashMap<String, SubcktDef>,
    macros: &HashSet<String>,
    depth: usize,
) -> Vec<(usize, String)> {
    if depth > MAX_FLATTEN_DEPTH {
        return lines.to_vec();
    }

    let mut result = Vec::new();
    let mut any_expanded = false;

    for (ln, line) in lines {
        let clean = line.replace(['(', ')'], " ");
        let tokens: Vec<&str> = clean.split_whitespace().collect();
        if tokens.is_empty() || tokens[0].starts_with('.') {
            result.push((*ln, line.clone()));
            continue;
        }

        let eq_start = tokens[1..]
            .iter()
            .position(|t| t.contains('='))
            .unwrap_or(tokens.len() - 1);
        let positional = &tokens[1..=eq_start];
        let params = &tokens[1 + eq_start..];

        if positional.is_empty() {
            result.push((*ln, line.clone()));
            continue;
        }

        let model = positional[positional.len() - 1];
        let nets = &positional[..positional.len() - 1];

        // substrate3-implemented subckt: keep whole, never flatten.
        if macros.contains(model) {
            result.push((*ln, line.clone()));
            continue;
        }

        let Some(subckt) = subcircuits.get(model) else {
            result.push((*ln, line.clone()));
            continue;
        };

        any_expanded = true;
        let inst = tokens[0];

        let net_map: HashMap<&str, &str> = subckt
            .ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| nets.get(i).map(|&n| (port.as_str(), n)))
            .collect();

        for (bln, bline) in &subckt.body {
            let bclean = bline.replace(['(', ')'], " ");
            let bt: Vec<&str> = bclean.split_whitespace().collect();
            if bt.is_empty() || bt[0].starts_with('.') {
                continue;
            }

            let beq = bt[1..]
                .iter()
                .position(|t| t.contains('='))
                .unwrap_or(bt.len() - 1);
            let bpos = &bt[1..=beq];
            let bpar = &bt[1 + beq..];

            let mut out = vec![format!("{inst}/{}", bt[0])];

            for (i, &net) in bpos.iter().enumerate() {
                if i == bpos.len() - 1 {
                    out.push(net.to_string());
                } else if let Some(&mapped) = net_map.get(net) {
                    out.push(mapped.to_string());
                } else {
                    out.push(format!("{inst}/{net}"));
                }
            }

            // Params from body line, then override with instance params.
            for &p in bpar {
                out.push(p.to_string());
            }
            for &p in params {
                out.push(p.to_string());
            }

            result.push((*bln, out.join(" ")));
        }
    }

    if any_expanded {
        expand_instances(&result, subcircuits, macros, depth + 1)
    } else {
        result
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Read a SPICE netlist file, resolving `.include`/`.lib` relative to the
/// file's directory. `macros` = subckt names with a substrate3 impl.
#[allow(clippy::missing_errors_doc)]
pub fn read_netlist(
    path: &Path,
    pdk: &Pdk,
    macros: &HashSet<String>,
) -> Result<BipartiteHypergraph, ParseError> {
    let text = resolve_includes(path, &mut HashSet::new())?;
    parse_spice(&text, pdk, macros)
}

fn resolve_includes(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<String, ParseError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical) {
        return Ok(String::new());
    }
    let text = std::fs::read_to_string(path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut out = String::with_capacity(text.len());

    for line in text.lines() {
        let lower = line.trim().to_ascii_lowercase();
        if lower.starts_with(".include") || lower.starts_with(".lib") {
            if let Some((_, rest)) = line.trim().split_once(char::is_whitespace) {
                let inc = rest.trim().trim_matches('"').trim_matches('\'');
                // Strip optional section name (".lib file.lib tt").
                let inc = inc.split_whitespace().next().unwrap_or(inc);
                let full = base.join(inc);
                if full.exists() {
                    out.push_str(&resolve_includes(&full, visited)?);
                }
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

/// Parse a SPICE netlist string into a [`BipartiteHypergraph`].
///
/// `macros`: subckt names implemented as substrate3 cells — kept as single
/// nodes. Every other subckt is flattened; leaf devices resolve through the
/// PDK device table. Unknown models are skipped (unresolved library refs).
#[allow(clippy::missing_errors_doc)]
pub fn parse_spice(
    text: &str,
    pdk: &Pdk,
    macros: &HashSet<String>,
) -> Result<BipartiteHypergraph, ParseError> {
    // Phase 1: preprocess — strip comments, join continuation lines.
    let mut cleaned: Vec<(usize, String)> = Vec::new();
    for (line_no, raw) in text.lines().enumerate() {
        let line_no = line_no + 1;
        let raw = raw.split("//").next().unwrap_or("").trim();
        if raw.is_empty() || raw.starts_with('*') {
            continue;
        }
        if let Some(stripped) = raw.strip_prefix('+') {
            let Some((_, last)) = cleaned.last_mut() else {
                return Err(syntax_err(line_no, "continuation line without previous line"));
            };
            last.push(' ');
            last.push_str(stripped.trim());
            continue;
        }
        cleaned.push((line_no, raw.to_string()));
    }

    let spice_params = collect_params(&cleaned);

    // Phase 2: flatten hierarchy, keeping macro instances whole.
    let (flattened, top_name, top_ports, subckt_ports) =
        flatten_subcircuits(&cleaned, macros);

    let mut hg = BipartiteHypergraph {
        name: top_name,
        ports: top_ports,
        ..Default::default()
    };
    let mut net_ids: HashMap<String, NetId> = HashMap::new();
    let mut net = |name: &str, nets: &mut Vec<String>| -> NetId {
        *net_ids.entry(name.to_string()).or_insert_with(|| {
            nets.push(name.to_string());
            nets.len() as u32 - 1
        })
    };

    // Phase 3: each remaining line is a macro instance or a leaf device.
    let mut in_control_block = false;
    for (line_no, line) in flattened {
        let lower = line.trim().to_ascii_lowercase();

        if lower.starts_with(".control") {
            in_control_block = true;
            continue;
        }
        if lower.starts_with(".endc") {
            in_control_block = false;
            continue;
        }
        if in_control_block || lower.starts_with('.') || lower.starts_with("ends") {
            continue;
        }

        let line = line.replace(['(', ')'], " ");
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let name = tokens[0];

        let params: HashMap<String, String> = tokens
            .iter()
            .filter_map(|t| {
                let (key, value) = t.split_once('=')?;
                Some((key.to_ascii_lowercase(), value.trim_end_matches(',').to_string()))
            })
            .collect();

        let positional: Vec<&str> = tokens[1..]
            .iter()
            .copied()
            .take_while(|t| !t.contains('='))
            .collect();
        if positional.is_empty() {
            return Err(syntax_err(
                line_no,
                format!("instance `{name}`: no positional tokens (expected nodes + model)"),
            ));
        }
        let model = positional[positional.len() - 1];
        let nodes = &positional[..positional.len() - 1];

        // --- substrate3 macro: one node, pins = subckt ports ---
        if macros.contains(model) {
            let pins: Vec<(String, NetId)> = match subckt_ports.get(model) {
                Some(ports) => {
                    if ports.len() != nodes.len() {
                        return Err(syntax_err(
                            line_no,
                            format!(
                                "macro `{name}` ({model}): {} nets for {} ports",
                                nodes.len(),
                                ports.len()
                            ),
                        ));
                    }
                    ports
                        .iter()
                        .zip(nodes)
                        .map(|(p, n)| (p.clone(), net(n, &mut hg.nets)))
                        .collect()
                }
                // No .subckt def in file (impl-only macro): positional pin names.
                None => nodes
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (format!("p{i}"), net(n, &mut hg.nets)))
                    .collect(),
            };
            hg.cells.push(CellNode {
                name: name.to_string(),
                model: model.to_string(),
                pins,
                device: None,
                grouped_devices: Vec::new(),
            });
            continue;
        }

        // --- leaf device via PDK table ---
        let Some(def) = pdk.device(model) else {
            return Err(syntax_err(
                line_no,
                format!("device `{name}`: model `{model}` not found in PDK device table"),
            ));
        };
        let dt = def.device_type;
        let (terminals, min_nodes) = terminal_spec(dt);
        if nodes.len() < min_nodes {
            return Err(syntax_err(
                line_no,
                format!(
                    "device `{name}` ({dt:?}): expected at least {min_nodes} nodes, got {}",
                    nodes.len()
                ),
            ));
        }

        let w = params
            .get("w")
            .map(|s| to_nm(eval_spice_value(s, &spice_params)))
            .or(def.default_w);
        let l = params
            .get("l")
            .map(|s| to_nm(eval_spice_value(s, &spice_params)))
            .or(def.default_l);
        let (Some(w), Some(l)) = (w, l) else {
            return Err(syntax_err(
                line_no,
                format!("device `{name}` ({model}): missing W/L and no PDK defaults"),
            ));
        };
        // Legalize: clamp up to PDK DRC minima, snap up to grid — FinFET
        // netlists carry sub-planar dims (l=20e-9) no planar process draws.
        let grid = pdk.grid();
        let snap_up = |v: i32| ((v + grid - 1) / grid) * grid;
        let w = def.min_w.map_or(w, |m| snap_up(w.max(m)));
        let l = def.min_l.map_or(l, |m| snap_up(l.max(m)));

        let nf = params
            .get("nf")
            .or_else(|| params.get("nfin"))
            .map(|s| eval_spice_value(s, &spice_params) as u16)
            .map_or(1, |v| v.max(1));
        let multiplier = params
            .get("m")
            .or_else(|| params.get("multi"))
            .map(|s| eval_spice_value(s, &spice_params) as u16)
            .map_or(1, |v| v.max(1));

        let pins: Vec<(String, NetId)> = terminals
            .iter()
            .zip(nodes)
            .map(|(&pin, n)| (pin.to_string(), net(n, &mut hg.nets)))
            .collect();
        let term_map: HashMap<String, u32> = pins.iter().cloned().collect();

        let extra_params: HashMap<String, f64> = params
            .iter()
            .filter(|(k, _)| !matches!(k.as_str(), "w" | "l" | "nf" | "nfin" | "m" | "multi"))
            .map(|(k, v)| (k.clone(), eval_spice_value(v, &spice_params)))
            .collect();

        hg.cells.push(CellNode {
            name: name.to_string(),
            model: model.to_string(),
            pins,
            device: Some(DeviceRecord {
                name: name.to_string(),
                device_type: dt,
                w,
                l,
                nf,
                multiplier,
                model_name: model.to_string(),
                terminals: term_map,
                params: extra_params,
            }),
            grouped_devices: Vec::new(),
        });
    }

    hg.build_csr();
    Ok(hg)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pdk() -> Pdk {
        Pdk::from_json(
            r#"{
            "drc": {"off_grid": {"grid": 5}},
            "devices": {
                "nfet_01v8": {"type": "nmos", "cell": "mosfet", "min_w": 420, "min_l": 150},
                "pfet_01v8": {"type": "pmos", "cell": "mosfet", "min_w": 420, "min_l": 150},
                "res_generic_po": {"type": "res", "cell": "resistor", "default_w": 1410, "min_w": 330}
            }}"#,
        )
        .unwrap()
    }

    fn no_macros() -> HashSet<String> {
        HashSet::new()
    }

    // -- value parsing ------------------------------------------------------

    #[test]
    fn parse_value_forms() {
        assert!((parse_value("10u") - 10e-6).abs() < 1e-15);
        assert!((parse_value("3.3meg") - 3.3e6).abs() < 1.0);
        assert!((parse_value("270e-9") - 270e-9).abs() < 1e-18);
        assert!((parse_value("42") - 42.0).abs() < 1e-15);
        assert!((parse_value("10u,") - 10e-6).abs() < 1e-15);
    }

    #[test]
    fn to_nm_units() {
        assert_eq!(to_nm(10e-6), 10_000, "10u meters -> nm");
        assert_eq!(to_nm(2.0), 2_000, "bare 2 = 2um -> nm");
    }

    #[test]
    fn expr_eval() {
        let p = HashMap::new();
        assert_eq!(SpiceExpr::eval("(1+2)*3", &p), Some(9.0));
        assert_eq!(SpiceExpr::eval("2**3", &p), Some(8.0));
        assert_eq!(SpiceExpr::eval("min(1, 2)", &p), Some(1.0));
    }

    // -- flat parse -----------------------------------------------------------

    const OTA: &str = "\
.subckt ota vinp vinm vout1 vout2 VDD VSS
XM1 vout1 vinp vtail VSS nfet_01v8 W=10u L=1u nf=2
XM2 vout2 vinm vtail VSS nfet_01v8 W=10u L=1u nf=2
XM3 vout1 vbias VDD VDD pfet_01v8 W=20u L=1u
XM4 vout2 vbias VDD VDD pfet_01v8 W=20u L=1u
XM5 vtail vbn VSS VSS nfet_01v8 W=40u L=2u m=4
.ends ota
";

    #[test]
    fn parse_ota_5t() {
        let hg = parse_spice(OTA, &test_pdk(), &no_macros()).expect("parse");
        assert_eq!(hg.name, "ota");
        assert_eq!(hg.ports.len(), 6);
        assert_eq!(hg.cells.len(), 5);
        assert_eq!(hg.nets.len(), 9);

        let m1 = &hg.cells[hg.cell_id("XM1").unwrap() as usize];
        let d = m1.device.as_ref().unwrap();
        assert_eq!(d.w, 10_000, "10u = 10_000 nm");
        assert_eq!(d.l, 1_000);
        assert_eq!(d.nf, 2);
        assert_eq!(d.device_type, DeviceType::Nmos);
        assert_eq!(m1.pins[0].0, "D");

        let m5 = &hg.cells[hg.cell_id("XM5").unwrap() as usize];
        assert_eq!(m5.device.as_ref().unwrap().multiplier, 4);

        // vtail hyperedge: XM1.S, XM2.S, XM5.D.
        let vtail = hg.net_id("vtail").unwrap();
        let pins = hg.pins_on_net(vtail);
        assert_eq!(pins.len(), 3);
        let members: Vec<String> = pins
            .iter()
            .map(|&(c, p)| {
                let cell = &hg.cells[c as usize];
                format!("{}.{}", cell.name, cell.pins[p as usize].0)
            })
            .collect();
        assert!(members.contains(&"XM1.S".to_string()));
        assert!(members.contains(&"XM2.S".to_string()));
        assert!(members.contains(&"XM5.D".to_string()));
    }

    #[test]
    fn continuation_comments_params() {
        let spice = "\
* header comment
.param wp=5u
.subckt test VDD VSS
XM1 vout vin VSS VSS nfet_01v8 // inline comment
+ W={2*wp} L=1u nf=4
.ends test
";
        let hg = parse_spice(spice, &test_pdk(), &no_macros()).expect("parse");
        let d = hg.cells[0].device.as_ref().unwrap();
        assert_eq!(d.w, 10_000, "2*5u = 10u");
        assert_eq!(d.nf, 4);
    }

    #[test]
    fn default_w_from_pdk() {
        let spice = "\
.subckt test A B
XR1 A B res_generic_po L=10u
.ends test
";
        let hg = parse_spice(spice, &test_pdk(), &no_macros()).expect("parse");
        let d = hg.cells[0].device.as_ref().unwrap();
        assert_eq!(d.w, 1_410, "PDK default");
        assert_eq!(d.l, 10_000);
        assert_eq!(hg.cells[0].pins[0].0, "P");
    }

    #[test]
    fn unknown_model_errors() {
        let unknown = "\
.subckt test VDD VSS
X5 VDD VSS SOME_TAPCELL
XM1 d g VSS VSS nfet_01v8 W=1u L=1u
.ends test
";
        let err = parse_spice(unknown, &test_pdk(), &no_macros()).unwrap_err();
        assert!(err.to_string().contains("not found in PDK"), "{err}");

        let missing_wl = "\
.subckt test VDD VSS
XM1 d g s b nfet_01v8
.ends test
";
        assert!(parse_spice(missing_wl, &test_pdk(), &no_macros()).is_err());
    }

    #[test]
    fn legalize_clamps_and_snaps() {
        // FinFET-style dims: l=20e-9 is below any planar poly width.
        let spice = "\
.subckt t VDD VSS
XM1 d g s b nfet_01v8 W=0.422u L=20e-9
.ends t
";
        let hg = parse_spice(spice, &test_pdk(), &no_macros()).expect("parse");
        let d = hg.cells[0].device.as_ref().unwrap();
        assert_eq!(d.l, 150, "20nm clamped up to min_l");
        assert_eq!(d.w, 425, "422nm >= min_w, snapped up to grid 5");
    }

    // -- instance-level grouping (registry-consumed devices) -------------------

    #[test]
    fn group_instances_collapses() {
        let mut hg = parse_spice(OTA, &test_pdk(), &no_macros()).expect("parse");
        let gid = hg
            .group("XDP", "diff_pair", &["XM1".into(), "XM2".into()])
            .expect("group");
        assert_eq!(hg.cells.len(), 4, "5 devices - 2 members + 1 group");

        let g = &hg.cells[gid as usize];
        assert!(g.device.is_none(), "group is a macro node");
        assert_eq!(g.model, "diff_pair");
        assert_eq!(g.pins.len(), 8, "2 members x 4 terminals");

        // vinp now reaches the group through pin `XM1.G`.
        let vinp = hg.net_id("vinp").unwrap();
        let pins = hg.pins_on_net(vinp);
        assert_eq!(pins.len(), 1);
        let (c, p) = pins[0];
        assert_eq!(hg.cells[c as usize].name, "XDP");
        assert_eq!(hg.cells[c as usize].pins[p as usize].0, "XM1.G");

        // vtail: two group pins (XM1.S, XM2.S) + XM5.D — connectivity exact.
        let vtail = hg.net_id("vtail").unwrap();
        assert_eq!(hg.pins_on_net(vtail).len(), 3);

        // Unknown member errors.
        assert!(hg.group("X", "m", &["nope".into()]).is_err());
    }

    #[test]
    fn group_registry_applies() {
        use pnr_cells::{CellBuilder, CellError, CellRegistry, CustomCellEntry, MatchingTier};

        let mut hg = parse_spice(OTA, &test_pdk(), &no_macros()).expect("parse");

        let gen = |b: &mut CellBuilder| -> Result<(), CellError> {
            b.rect("met1", 0, 0, 100, 100)?;
            Ok(())
        };
        let mut reg = CellRegistry::new();
        reg.add_custom(CustomCellEntry {
            name: "diff_pair".into(),
            generator: Box::new(gen),
            instances: vec!["XM1".into(), "XM2".into()],
            device_type: DeviceType::Nmos,
            matching_tier: MatchingTier::Exceptional,
            matching_group: Some(0),
        });

        hg.group_registry(&reg).expect("group_registry");
        assert_eq!(hg.cells.len(), 4);
        let g = hg.cell_id("diff_pair").expect("group node named after entry");
        assert!(hg.cells[g as usize].device.is_none());
    }

    // -- flattening vs macro grouping -----------------------------------------

    const INV_TOP: &str = "\
.subckt INV A Y VDD VSS
XM1 Y A VSS VSS nfet_01v8 W=1u L=0.18u
XM2 Y A VDD VDD pfet_01v8 W=2u L=0.18u
.ends INV

.subckt TOP in out VDD VSS
XINV1 in mid VDD VSS INV
XINV2 mid out VDD VSS INV
.ends TOP
";

    #[test]
    fn no_impl_flattens() {
        let hg = parse_spice(INV_TOP, &test_pdk(), &no_macros()).expect("parse");
        assert_eq!(hg.cells.len(), 4, "2 inverters x 2 FETs");
        assert!(hg.cell_id("XINV1/XM1").is_some());
        let mid = hg.net_id("mid").unwrap();
        assert_eq!(hg.pins_on_net(mid).len(), 4);
    }

    #[test]
    fn substrate3_impl_groups() {
        let macros: HashSet<String> = ["INV".to_string()].into();
        let hg = parse_spice(INV_TOP, &test_pdk(), &macros).expect("parse");
        assert_eq!(hg.cells.len(), 2, "each INV = one macro node");
        let inv1 = &hg.cells[hg.cell_id("XINV1").unwrap() as usize];
        assert!(inv1.device.is_none());
        assert_eq!(inv1.model, "INV");
        // Pins named after subckt ports, wired to instance nets.
        assert_eq!(inv1.pins[0].0, "A");
        assert_eq!(hg.nets[inv1.pins[0].1 as usize], "in");
        // Shared net `mid` connects XINV1.Y to XINV2.A.
        let mid = hg.net_id("mid").unwrap();
        assert_eq!(hg.pins_on_net(mid).len(), 2);
    }

    #[test]
    fn impl_only_macro_positional_pins() {
        // diff_pair has a substrate3 impl but no .subckt in the file.
        let spice = "\
.subckt top inp inm tail op om VDD VSS
XDP inp inm tail op om diff_pair
.ends top
";
        let macros: HashSet<String> = ["diff_pair".to_string()].into();
        let hg = parse_spice(spice, &test_pdk(), &macros).expect("parse");
        assert_eq!(hg.cells.len(), 1);
        assert_eq!(hg.cells[0].pins.len(), 5);
        assert_eq!(hg.cells[0].pins[0].0, "p0");
    }

    #[test]
    fn macro_port_count_mismatch_errors() {
        let spice = "\
.subckt INV A Y VDD VSS
XM1 Y A VSS VSS nfet_01v8 W=1u L=0.18u
.ends INV
.subckt TOP in out VDD VSS
XINV1 in mid VDD INV
.ends TOP
";
        let macros: HashSet<String> = ["INV".to_string()].into();
        assert!(parse_spice(spice, &test_pdk(), &macros).is_err());
    }

    // -- dump -------------------------------------------------------------------

    #[test]
    fn dump_format() {
        let macros: HashSet<String> = ["INV".to_string()].into();
        let hg = parse_spice(INV_TOP, &test_pdk(), &macros).expect("parse");
        let dump = hg.to_string();
        assert!(dump.contains("hypergraph `TOP`: 2 cells, 5 nets"), "{dump}");
        assert!(dump.contains("XINV1 INV macro | A=in Y=mid"), "{dump}");
        assert!(dump.contains("mid: XINV1.Y XINV2.A"), "{dump}");
    }

    // -- .include ---------------------------------------------------------------

    #[test]
    fn include_resolves() {
        let dir = std::env::temp_dir().join("pnr_upgraded_test_include");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("inv.lib"),
            "\
.subckt INV A Y VDD VSS
XM1 Y A VSS VSS nfet_01v8 W=1u L=0.18u
XM2 Y A VDD VDD pfet_01v8 W=2u L=0.18u
.ends INV
",
        )
        .unwrap();
        std::fs::write(
            dir.join("top.spice"),
            "\
.include \"inv.lib\"
.subckt top in out VDD VSS
XINV1 in out VDD VSS INV
.ends top
",
        )
        .unwrap();

        let hg = read_netlist(&dir.join("top.spice"), &test_pdk(), &no_macros()).expect("parse");
        assert_eq!(hg.name, "top");
        assert_eq!(hg.cells.len(), 2, "INV flattened to 2 FETs");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
