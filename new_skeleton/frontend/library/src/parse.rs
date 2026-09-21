//! SPICE front-end: `.sp` text → [`pnr_core::Netlist`].
//!
//! A real reader for the SPICE subset the fixtures use (`temporary/circuits/*.sp`):
//! a single `.subckt`/`.ends` block of primitive instance lines (MOSFET, R, C,
//! diode, BJT), values in engineering notation / SI suffixes. Migrated from
//! `frontend/core/src/netlist.rs` (`parse_value`/`to_nm`/terminal ordering),
//! trimmed to what this subset needs — no subckt flattening, no PDK device table,
//! no `.param` expression evaluation (the fixtures carry none).

use std::collections::HashMap;

use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};

/// Parse a SPICE netlist into the internal [`Netlist`].
///
/// The whole pipeline's input boundary: after this, everything is the internal
/// SoA model (parse-don't-validate). Nets are interned to [`NetId`]s in
/// first-seen order; each [`Device`] carries its terminals in G,D,S,B order (for
/// FETs) and its numeric params (W/L in `nm`, `nf`/`stack`/`m` as plain counts).
pub fn spice(text: &str) -> Result<Netlist, String> {
    let mut nets: Vec<Net> = Vec::new();
    let mut net_index: HashMap<String, NetId> = HashMap::new();
    let mut devices: Vec<Device> = Vec::new();

    // Intern a net name → NetId, deduping (api boundary: nets deduped).
    let mut intern = |name: &str, nets: &mut Vec<Net>| -> NetId {
        if let Some(id) = net_index.get(name) {
            return *id;
        }
        let id = NetId(nets.len() as u16);
        nets.push(Net { name: name.to_string() });
        net_index.insert(name.to_string(), id);
        id
    };

    for raw in logical_lines(text) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('*') || line.starts_with('.') {
            // Comments, blank lines, and dot-directives (`.subckt`/`.ends`/…) are
            // structural for this subset — the fixtures are a single flat subckt.
            continue;
        }

        // `Mname n1 n2 … model param=val …`. Split into positional tokens (up to
        // and including the model) and `key=value` params.
        let mut positional: Vec<&str> = Vec::new();
        let mut params: Vec<(String, String)> = Vec::new();
        for tok in line.split_whitespace() {
            match tok.split_once('=') {
                Some((k, v)) => {
                    params.push((k.to_ascii_lowercase(), v.trim_end_matches(',').to_string()))
                }
                None => positional.push(tok),
            }
        }
        if positional.len() < 2 {
            return Err(format!("device `{line}`: too few tokens"));
        }

        let name = positional[0].to_string();
        let model = positional[positional.len() - 1];
        let nodes = &positional[1..positional.len() - 1];

        let kind = device_kind(&name, model)
            .ok_or_else(|| format!("device `{name}`: unknown model `{model}`"))?;

        let (term_names, min_nodes) = terminal_spec(kind);
        if nodes.len() < min_nodes {
            return Err(format!(
                "device `{name}` ({model}): {} nodes, expected >= {min_nodes}",
                nodes.len()
            ));
        }

        // SPICE lists MOS nodes as D G S [B]; emit terminals in G,D,S,B order (the
        // order `cells`/LVS expect). Non-MOS keep their listed order. A 3-terminal
        // MOS reuses the source net as bulk.
        let terminals: Vec<(String, NetId)> = match kind {
            DeviceKind::Nmos | DeviceKind::Pmos => {
                let d = intern(nodes[0], &mut nets);
                let g = intern(nodes[1], &mut nets);
                let s = intern(nodes[2], &mut nets);
                let b = intern(nodes.get(3).copied().unwrap_or(nodes[2]), &mut nets);
                vec![
                    ("G".to_string(), g),
                    ("D".to_string(), d),
                    ("S".to_string(), s),
                    ("B".to_string(), b),
                ]
            }
            _ => term_names
                .iter()
                .zip(nodes.iter())
                .map(|(t, n)| ((*t).to_string(), intern(n, &mut nets)))
                .collect(),
        };

        // Numeric params. Lengths (w/l) → nm; counts (nf/stack/m) → plain ints.
        let mut out_params: Vec<(String, i64)> = Vec::new();
        for (k, v) in &params {
            let val = match k.as_str() {
                "w" | "l" => i64::from(to_nm(parse_value(v))),
                "nf" | "nfin" | "stack" | "m" | "multi" => parse_value(v).round() as i64,
                _ => continue,
            };
            out_params.push((k.clone(), val));
        }

        devices.push(Device { name, kind, terminals, params: out_params });
    }

    if devices.is_empty() {
        return Err("no devices parsed".to_string());
    }
    Ok(Netlist { devices, nets })
}

/// Fold SPICE continuation lines (`+ …`) into their parent, so each yielded line
/// is one logical statement.
fn logical_lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(cont) = t.strip_prefix('+') {
            if let Some(last) = out.last_mut() {
                last.push(' ');
                last.push_str(cont.trim());
                continue;
            }
        }
        out.push(line.to_string());
    }
    out
}

/// Classify a device by its SPICE instance prefix + model name.
///
/// The instance letter is the primary key (`M`→MOS, `R`→res, …); the model name
/// disambiguates N/P polarity for MOS (`pfet`/`pmos`) and NPN/PNP for BJT.
fn device_kind(name: &str, model: &str) -> Option<DeviceKind> {
    let m = model.to_ascii_lowercase();
    let is_p = m.contains("pfet") || m.contains("pmos") || m.contains("pnp");
    match name.chars().next()?.to_ascii_lowercase() {
        // `X` is a *subcircuit call*, so the instance letter says nothing about
        // what the device is — and modern PDKs ship every primitive as a
        // subcircuit, so `X` is the common case, not an exotic one. The model
        // name is the only signal, and it must be consulted for the device
        // *family* before falling back to MOS.
        //
        // Getting this wrong is silent and expensive: `XQ1 … npn_05v5` became an
        // NMOS (drawn as a MOSFET, extracted as the wrong device), and
        // `XR1 … res_generic_po` became an NMOS demanding ≥3 nodes, so a
        // perfectly ordinary 2-terminal resistor failed to parse at all.
        'x' => Some(subckt_kind(&m).unwrap_or(if is_p {
            DeviceKind::Pmos
        } else {
            DeviceKind::Nmos
        })),
        'm' => Some(if is_p { DeviceKind::Pmos } else { DeviceKind::Nmos }),
        'r' => Some(DeviceKind::Resistor),
        'c' => Some(DeviceKind::Capacitor),
        'd' => Some(DeviceKind::Diode),
        // `is_p` already tests `pnp`; a bare `Q` card with an unrecognised model
        // keeps the historical NPN default.
        'q' => Some(if is_p { DeviceKind::Pnp } else { DeviceKind::Npn }),
        'l' => Some(DeviceKind::Inductor),
        _ => None,
    }
}

/// Device family implied by a **subcircuit model name**, for `X` instances.
///
/// Ordered most-specific first: `pnp`/`npn` before any substring that might also
/// appear elsewhere, and the FET check last so `nfet`/`pfet` fall through to the
/// caller's polarity logic. `None` means "not recognisably a non-MOS primitive",
/// which the caller reads as MOS — the right default for a modern PDK where the
/// overwhelming majority of `X` instances are transistors.
fn subckt_kind(model_lower: &str) -> Option<DeviceKind> {
    let has = |p: &str| model_lower.contains(p);
    if has("pnp") {
        Some(DeviceKind::Pnp)
    } else if has("npn") {
        Some(DeviceKind::Npn)
    } else if has("res") || has("resistor") {
        Some(DeviceKind::Resistor)
    } else if has("cap") || has("capacitor") {
        Some(DeviceKind::Capacitor)
    } else if has("diode") || has("_diode") {
        Some(DeviceKind::Diode)
    } else if has("ind") && !has("individual") {
        Some(DeviceKind::Inductor)
    } else {
        None
    }
}

/// Terminal names + minimum node count per kind (ported from
/// `netlist.rs::terminal_spec`). MOS listed as D G S B; a 3-terminal MOS is
/// accepted (bulk defaults to the source).
fn terminal_spec(kind: DeviceKind) -> (&'static [&'static str], usize) {
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => (&["D", "G", "S", "B"], 3),
        DeviceKind::Resistor | DeviceKind::Capacitor | DeviceKind::Diode => (&["P", "N"], 2),
        DeviceKind::Npn | DeviceKind::Pnp => (&["C", "B", "E"], 3),
        DeviceKind::Inductor => (&["P", "N"], 2),
    }
}

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

/// Parse a SPICE value with optional SI suffix into base SI units (ported from
/// `netlist.rs::parse_value`). `"10u"` → 10e-6, `"270e-9"` → 270e-9,
/// `"3.3meg"` → 3.3e6.
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

/// Base-SI value → nm (ported from `netlist.rs::to_nm`). Values >= 0.01 are bare
/// micrometers (`"W=2"`); smaller are meters (`"W=2u"`, already scaled).
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
        // Nets: id, vss, vout, net8, vdd → 5 distinct.
        assert_eq!(nl.nets.len(), 5);
    }
}

#[cfg(test)]
mod x_instance_tests {
    use super::*;

    #[test]
    fn x_instances_are_classified_by_model_not_by_letter() {
        // A subcircuit call carries no device information in its letter, and in a
        // modern PDK almost everything is a subcircuit.
        assert_eq!(device_kind("XM1", "nfet_01v8"), Some(DeviceKind::Nmos));
        assert_eq!(device_kind("XM3", "pfet_01v8"), Some(DeviceKind::Pmos));
        // Regressions that used to silently become MOSFETs:
        // Polarity, not just family: collapsing these two onto one `Bjt` variant
        // drew, extracted and referenced the PNP as an NPN.
        assert_eq!(device_kind("XQ1", "npn_05v5_1x1"), Some(DeviceKind::Npn));
        assert_eq!(device_kind("XQ2", "pnp_05v5_W3p40L3p40"), Some(DeviceKind::Pnp));
        assert_eq!(device_kind("XR1", "res_generic_po"), Some(DeviceKind::Resistor));
        assert_eq!(device_kind("XC1", "cap_mim_m3_1"), Some(DeviceKind::Capacitor));
    }

    #[test]
    fn explicit_primitive_letters_still_win() {
        assert_eq!(device_kind("M1", "nfet_01v8"), Some(DeviceKind::Nmos));
        assert_eq!(device_kind("Q1", "npn"), Some(DeviceKind::Npn));
        assert_eq!(device_kind("Q2", "pnp"), Some(DeviceKind::Pnp));
        assert_eq!(device_kind("R1", "res"), Some(DeviceKind::Resistor));
    }

    #[test]
    fn a_two_terminal_resistor_parses() {
        // The exact `rc_filter` failure: classified NMOS, which demands >=3
        // nodes, so an ordinary 2-terminal resistor was rejected outright.
        let kind = device_kind("XR1", "res_generic_po").unwrap();
        let (names, min_nodes) = terminal_spec(kind);
        assert_eq!(min_nodes, 2, "a resistor takes two nodes");
        assert_eq!(names, &["P", "N"]);
    }
}
