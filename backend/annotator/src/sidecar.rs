//! EXT-26: the user constraint sidecar, an ALIGN-style JSON array of
//! `{"constraint": <kind>, ...}` entries, parsed into [`AnnotationConfig`]
//! fields. Names resolve case-insensitively; an unknown name skips its entry
//! with a `sidecar_unknown_name` diagnostic. Only kinds with a reader today
//! are applied: `Load` (EXT-25) and `Order` (EXT-28) are `sidecar_unconsumed`,
//! anything else (`Align`, `HorizontalDistance`, …) `sidecar_unsupported`.
//!
//! | kind                                     | fields                                  |
//! |------------------------------------------|-----------------------------------------|
//! | `PowerPorts`, `GroundPorts`, `ClockPorts` | `ports: [net]`                          |
//! | `DoNotIdentify`                          | `instances`                             |
//! | `GroupBlocks`                            | `instances`, `instance_name` (an alias) |
//! | `SymmetricBlocks`                        | `direction` (`V`/`H`), `pairs: [[name]]`|
//! | `SymmetricNets`                          | `net1`, `net2`                          |
//! | `Match`                                  | `instances`, `class`, optional `kind`   |
//! | `NetClass`                               | `nets`, `class`                         |
//! | `OffsetBudget`                           | `instances`, `sigma_mv`                 |
//! | `Kelvin`                                 | `pin: "R/P"`, `sense: ["M/G"]`          |
//! | `IsolatedTub`                            | `instances` (NMOS, one bulk net), `tie` |

use std::collections::HashMap;

use analog::intent::{ConstraintId, Diagnostic, KelvinReq, MatchClass, MatchKind, Term};
use analog::metadata::NetClass;
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::Netlist;
use serde_json::Value;

use crate::symmetry::Seed;
use crate::AnnotationConfig;

/// Parse `json` against `nl`. `Err` only for text that is not JSON or not a
/// top-level array; every entry-level problem is a diagnostic.
pub fn parse(json: &str, nl: &Netlist) -> Result<(AnnotationConfig, Vec<Diagnostic>), String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("constraints: {e}"))?;
    let Value::Array(entries) = v else { return Err("constraints: top level is not an array".into()) };
    let mut cfg = AnnotationConfig::default();
    let mut diags = Vec::new();
    let mut alias: HashMap<String, Vec<DeviceId>> = HashMap::new();
    let device = |n: &str| nl.devices.iter().position(|d| d.name.eq_ignore_ascii_case(n)).map(|i| DeviceId(i as u16));
    let net = |n: &str| nl.nets.iter().position(|x| x.name.eq_ignore_ascii_case(n)).map(|i| NetId(i as u16));
    for (i, e) in entries.iter().enumerate() {
        let id = ConstraintId(u32::MAX - i as u32);
        let kind = e.get("constraint").and_then(Value::as_str).unwrap_or("");
        let strs = |k: &str| -> Vec<String> { e.get(k).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(String::from).collect() };
        macro_rules! unknown {
            ($what:expr) => {
                diags.push(Diagnostic { kind: "sidecar_unknown_name", devices: vec![], message: format!("entry {i} ({kind}): unknown {}", $what) })
            };
        }
        // All named devices, or `Err` naming the first unknown one.
        let devices = |names: &[String]| -> Result<Vec<DeviceId>, String> { names.iter().map(|n| device(n).ok_or_else(|| format!("instance {n}"))).collect() };
        match kind {
            "PowerPorts" | "GroundPorts" | "ClockPorts" => {
                let ports = strs("ports");
                if let Some(bad) = ports.iter().find(|p| net(p).is_none()) {
                    unknown!(format!("net {bad}"));
                    continue;
                }
                let list = match kind {
                    "PowerPorts" => &mut cfg.supply_nets,
                    "GroundPorts" => &mut cfg.ground_nets,
                    _ => &mut cfg.clock_nets,
                };
                list.extend(ports);
            }
            "DoNotIdentify" => {
                match devices(&strs("instances")) {
                    Ok(ds) => cfg.do_not_identify.extend(ds.iter().map(|d| u32::from(d.0))),
                    Err(n) => unknown!(n),
                }
            }
            "GroupBlocks" => {
                match devices(&strs("instances")) {
                    Ok(ds) => {
                        if let Some(name) = e.get("instance_name").and_then(Value::as_str) {
                            alias.insert(name.to_ascii_lowercase(), ds.clone());
                        }
                        cfg.groups.push((i as u32, ds));
                    }
                    Err(n) => unknown!(n),
                }
            }
            "SymmetricBlocks" => {
                cfg.symmetry_dir = Some(if e.get("direction").and_then(Value::as_str) == Some("H") { analog::intent::AxisDir::H } else { analog::intent::AxisDir::V });
                for pair in e.get("pairs").and_then(Value::as_array).into_iter().flatten() {
                    let names: Vec<&str> = pair.as_array().into_iter().flatten().filter_map(Value::as_str).collect();
                    let resolve = |n: &str| alias.get(&n.to_ascii_lowercase()).cloned().or_else(|| device(n).map(|d| vec![d]));
                    let got: Option<Vec<Vec<DeviceId>>> = names.iter().map(|n| resolve(n)).collect();
                    let Some(got) = got else {
                        unknown!(format!("instance in {names:?}"));
                        continue;
                    };
                    match got.as_slice() {
                        [one] if one.len() == 1 => cfg.seeds.push(Seed::SelfDevice(one[0], id)),
                        [one] if one.len() == 2 => cfg.seeds.push(Seed::Devices(one[0], one[1], id)),
                        [a, b] if a.len() == b.len() => cfg.seeds.extend(a.iter().zip(b).map(|(&x, &y)| Seed::Devices(x, y, id))),
                        _ => diags.push(Diagnostic { kind: "sidecar_unsupported", devices: got.concat(), message: format!("entry {i}: symmetric group {names:?}") }),
                    }
                }
            }
            "SymmetricNets" => {
                let (a, b) = (e.get("net1").and_then(Value::as_str).unwrap_or(""), e.get("net2").and_then(Value::as_str).unwrap_or(""));
                match (net(a), net(b)) {
                    (Some(x), Some(y)) => cfg.seeds.push(Seed::Nets(x, y, id)),
                    _ => unknown!(format!("net {a}/{b}")),
                }
            }
            "Match" => {
                let class = match e.get("class").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() {
                    Some("minimal") => MatchClass::Minimal,
                    Some("moderate") => MatchClass::Moderate,
                    Some("exceptional") => MatchClass::Exceptional,
                    _ => {
                        diags.push(Diagnostic { kind: "sidecar_unsupported", devices: vec![], message: format!("entry {i}: Match class") });
                        continue;
                    }
                };
                let mk = match e.get("kind").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() {
                    Some("voltage") => Some(MatchKind::Voltage),
                    Some("current") => Some(MatchKind::Current),
                    Some("ratio") => Some(MatchKind::Ratio),
                    _ => None,
                };
                match devices(&strs("instances")) {
                    Ok(ds) => {
                        // GAP-09 (c): kind, model and L must agree (W may differ: a ratioed set).
                        let mut models = Vec::new();
                        let key = |d: DeviceId, models: &mut Vec<String>| {
                            let dev = &nl.devices[d.0 as usize];
                            let s = crate::size::drawn(dev, models);
                            (dev.kind, s.model, s.l_nm)
                        };
                        let first = ds.first().map(|&d| key(d, &mut models));
                        if ds.iter().any(|&d| Some(key(d, &mut models)) != first) {
                            diags.push(crate::conflict::diag(&[id], ds, "Match on unequal kind/model/L; entry dropped"));
                            continue;
                        }
                        cfg.classes.push((ds, class, mk));
                    }
                    Err(n) => unknown!(n),
                }
            }
            "NetClass" => {
                let class = match e.get("class").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() {
                    Some("signal") => NetClass::Signal,
                    Some("clock") => NetClass::Clock,
                    Some("supply") => NetClass::Supply,
                    Some("ground") => NetClass::Ground,
                    Some("sensitive") => NetClass::Sensitive,
                    Some("substrate") => NetClass::Substrate,
                    Some("bias") => NetClass::Bias,
                    Some("reference") => NetClass::Reference,
                    // Conservative: undeclared logic may toggle, so it is an aggressor.
                    Some("digital") => NetClass::DigitalSwitching,
                    Some("digital_static") => NetClass::DigitalStatic,
                    Some("noisy") => NetClass::Noisy,
                    _ => {
                        diags.push(Diagnostic { kind: "sidecar_unsupported", devices: vec![], message: format!("entry {i}: NetClass class") });
                        continue;
                    }
                };
                for n in strs("nets") {
                    match net(&n) {
                        Some(x) => cfg.net_classes.push((x, class)),
                        None => unknown!(format!("net {n}")),
                    }
                }
            }
            "OffsetBudget" => {
                match (devices(&strs("instances")), e.get("sigma_mv").and_then(Value::as_f64)) {
                    (Ok(ds), Some(s)) => cfg.offset_budgets.push((ds, s as f32)),
                    (Err(n), _) => unknown!(n),
                    (Ok(_), None) => diags.push(Diagnostic { kind: "sidecar_unsupported", devices: vec![], message: format!("entry {i}: OffsetBudget without sigma_mv") }),
                }
            }
            "Kelvin" => {
                let pin = |s: &str| -> Option<(DeviceId, Term)> {
                    let (d, t) = s.split_once('/')?;
                    let t = match t.to_ascii_uppercase().as_str() {
                        "G" => Term::G,
                        "D" => Term::D,
                        "S" => Term::S,
                        "B" => Term::B,
                        "C" => Term::C,
                        "E" => Term::E,
                        "P" => Term::P,
                        "N" => Term::N,
                        _ => return None,
                    };
                    Some((device(d)?, t))
                };
                let at = e.get("pin").and_then(Value::as_str).unwrap_or("");
                let sense: Option<Vec<(DeviceId, Term)>> = strs("sense").iter().map(|s| pin(s)).collect();
                match (pin(at), sense) {
                    (Some((device, term)), Some(sense)) => cfg.kelvins.push(KelvinReq { device, term, sense }),
                    _ => unknown!(format!("pin in {at}")),
                }
            }
            "IsolatedTub" => {
                let ds = match devices(&strs("instances")) {
                    Ok(ds) => ds,
                    Err(n) => {
                        unknown!(n);
                        continue;
                    }
                };
                let tie_name = e.get("tie").and_then(Value::as_str).unwrap_or("");
                let Some(tie) = net(tie_name) else {
                    unknown!(format!("net {tie_name}"));
                    continue;
                };
                let bulk = |d: &DeviceId| {
                    let dev = &nl.devices[d.0 as usize];
                    (dev.kind == pnr_core::DeviceKind::Nmos).then(|| dev.terminals.iter().find(|(t, _)| t == "B").map(|t| t.1)).flatten()
                };
                if ds.is_empty() || ds.iter().any(|d| bulk(d).is_none() || bulk(d) != bulk(&ds[0])) {
                    diags.push(Diagnostic { kind: "sidecar_unsupported", devices: ds, message: format!("entry {i}: IsolatedTub members must be NMOS on one bulk net") });
                    continue;
                }
                cfg.tubs.push((ds, tie));
            }
            "Load" | "Order" => diags.push(Diagnostic {
                kind: "sidecar_unconsumed",
                devices: vec![],
                message: format!("entry {i} ({kind}): read by {}", if kind == "Load" { "EXT-25" } else { "EXT-28" }),
            }),
            _ => diags.push(Diagnostic { kind: "sidecar_unsupported", devices: vec![], message: format!("entry {i} ({kind})") }),
        }
    }
    Ok((cfg, diags))
}

impl AnnotationConfig {
    /// [`parse`]: the sidecar's config and its diagnostics.
    pub fn from_json(json: &str, nl: &Netlist) -> Result<(Self, Vec<Diagnostic>), String> {
        parse(json, nl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_constraint_is_diagnosed() {
        let nl = crate::tests::ota();
        let (_, d) = parse(r#"[{"constraint":"Align","instances":["XM1"]}]"#, &nl).unwrap();
        assert_eq!(d.iter().map(|d| d.kind).collect::<Vec<_>>(), ["sidecar_unsupported"]);
    }

    /// EXT-26: a `GroupBlocks` pull carries its entry as `Origin::User`, not
    /// the pattern or net class of its first device.
    #[test]
    fn group_blocks_batches_have_user_origin() {
        let nl = crate::tests::ota();
        let (cfg, _) = parse(r#"[{"constraint":"Align","instances":["XM1"]},{"constraint":"GroupBlocks","instances":["XM1","XM5"]}]"#, &nl).unwrap();
        let p = crate::annotate(&nl, &cfg);
        let user = |arm: &[Box<dyn analog::RuleBatch<pnr_core::Layout>>]| {
            arm.iter().filter(|b| b.meta().map(|m| m.origin) == Some(analog::intent::Origin::User { index: 1 })).map(|b| b.kind().ends_with("Proximity")).collect::<Vec<_>>()
        };
        assert_eq!((user(&p.placement.budget), user(&p.placement.cost)), (vec![true], vec![true]));
    }

    /// GAP-14: two NMOS on one bulk, tied to a supply.
    #[test]
    fn an_isolated_tub_parses() {
        let nl = crate::tests::ota();
        let (cfg, d) = parse(r#"[{"constraint":"IsolatedTub","instances":["XM1","XM2"],"tie":"vdd"}]"#, &nl).unwrap();
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(cfg.tubs, [(vec![DeviceId(0), DeviceId(1)], NetId(7))]);
    }

    #[test]
    fn a_tub_with_a_pmos_is_refused() {
        let nl = crate::tests::ota();
        let (cfg, d) = parse(r#"[{"constraint":"IsolatedTub","instances":["XM1","XM3"],"tie":"vdd"}]"#, &nl).unwrap();
        assert_eq!(d.iter().map(|d| d.kind).collect::<Vec<_>>(), ["sidecar_unsupported"]);
        assert!(cfg.tubs.is_empty());
    }

    #[test]
    fn unknown_name_is_diagnosed() {
        let nl = crate::tests::ota();
        let (c, d) = parse(r#"[{"constraint":"PowerPorts","ports":["nope"]}]"#, &nl).unwrap();
        assert_eq!(d.iter().map(|d| d.kind).collect::<Vec<_>>(), ["sidecar_unknown_name"]);
        assert!(c.supply_nets.is_empty());
    }

    #[test]
    fn not_json_is_an_error() {
        assert!(parse("{", &crate::tests::ota()).is_err());
        assert!(parse("{}", &crate::tests::ota()).is_err(), "not an array");
    }

    /// A user class wins over the role, and an Exceptional Voltage set makes
    /// its compound Perfect.
    #[test]
    fn match_class_overrides_role() {
        let nl = crate::tests::ota();
        let (cfg, _) = parse(r#"[{"constraint":"Match","instances":["XM1","XM2"],"class":"exceptional"}]"#, &nl).unwrap();
        let p = crate::annotate(&nl, &cfg);
        let s = p.intent.sets.iter().find(|s| s.kind == MatchKind::Voltage).expect("the input pair");
        assert_eq!((s.class, s.class_source), (MatchClass::Exceptional, analog::intent::ClassSource::User));
        assert_eq!(p.intent.compounds[s.compound.unwrap() as usize].kind, analog::intent::SymKind::Perfect);
    }
}
