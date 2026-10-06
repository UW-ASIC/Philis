//! GAP-09 (BAL2-48, NOTES-06): constraints that cannot all hold are reported
//! by rule id and the lower-priority one dropped, instead of letting λ
//! saturate at LAMBDA_MAX. (a) symmetry seeds: `symmetry::analyze`; (b) a
//! sidecar pull vs Isolation: `annotate_with`; (c) a sidecar Match on unequal
//! kind/model/L: `sidecar::parse`; (d) here.

use analog::intent::{ConstraintId, Diagnostic};
use analog::metadata::NetClass;
use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

use crate::Problem;

/// A `"conflict"` naming `ids` as `"ids a,b: what"` (decimal `ConstraintId.0`).
pub(crate) fn diag(ids: &[ConstraintId], devices: Vec<DeviceId>, what: impl std::fmt::Display) -> Diagnostic {
    let ids: Vec<String> = ids.iter().map(|i| i.0.to_string()).collect();
    Diagnostic { kind: "conflict", devices, message: format!("ids {}: {what}", ids.join(",")) }
}

/// (d): every Differential net pair (a compound's non-rail `net_pairs`, x ≠ y)
/// whose nets are touched by different device counts: `"asymmetric_net_pair"`,
/// message names the Differential batch id ("ids <id>: <x>/<y> <nx> vs <ny> devices").
/// Reported only, nothing dropped: it is a netlist fact, not a rule clash.
/// Empty when `p` has no Differential batch.
///
/// Panics when a terminal's net is outside `nl.nets` or a paired net outside
/// `p.net_classes` (both hold for a `Problem` annotated from `nl`).
#[must_use]
pub fn check(p: &Problem, nl: &Netlist) -> Vec<Diagnostic> {
    let Some(id) = p.routing.budget.iter().find(|b| b.kind().ends_with("::Differential")).and_then(|b| b.meta()).map(|m| m.id) else { return Vec::new() };
    let rail = |n: pnr_core::ids::NetId| matches!(p.net_classes[n.0 as usize].class, NetClass::Supply | NetClass::Ground | NetClass::Substrate);
    // Distinct devices per net, one pass.
    // `last[n]` is the last device counted on net `n`, so a device on one net
    // through several terminals counts once without a per-device allocation.
    let mut on = vec![0usize; nl.nets.len()];
    let mut last = vec![usize::MAX; nl.nets.len()];
    for (i, d) in nl.devices.iter().enumerate() {
        for &(_, n) in &d.terminals {
            let n = n.0 as usize;
            on[n] += usize::from(last[n] != i);
            last[n] = i;
        }
    }
    let count = |n: pnr_core::ids::NetId| on[n.0 as usize];
    let mut out = Vec::new();
    for c in &p.intent.compounds {
        for &(x, y) in c.net_pairs.iter().filter(|&&(x, y)| x != y && !rail(x) && !rail(y)) {
            let (nx, ny) = (count(x), count(y));
            if nx != ny {
                let name = |n: pnr_core::ids::NetId| &nl.nets[n.0 as usize].name;
                out.push(Diagnostic { kind: "asymmetric_net_pair", devices: vec![], message: format!("ids {}: {}/{} {nx} vs {ny} devices", id.0, name(x), name(y)) });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::tests::{fet, nets, ota};
    use pnr_core::netlist::{DeviceKind, Netlist};

    /// `ota()` plus `XM6`, XM1's signature on a new net `x6`.
    fn ota6() -> Netlist {
        let mut nl = ota();
        nl.nets.extend(nets(&["x6"]));
        nl.devices.push(fet("XM6", DeviceKind::Nmos, 9, 9, 3, 3, 10_000, 1_000));
        nl
    }

    fn annotate(nl: &Netlist, json: &str) -> (crate::Problem, crate::AnnotationConfig) {
        let (mut cfg, d) = crate::AnnotationConfig::from_json(json, nl).unwrap();
        cfg.sidecar_diags = d;
        (crate::annotate(nl, &cfg), cfg)
    }

    fn conflicts(p: &crate::Problem) -> Vec<&analog::intent::Diagnostic> {
        p.intent.diagnostics.iter().filter(|d| d.kind == "conflict").collect()
    }

    fn double_pairing(json: &str, prefix: &str) {
        let (p, _) = annotate(&ota6(), json);
        let c = conflicts(&p);
        assert_eq!(c.len(), 1, "{:?}", p.intent.diagnostics);
        assert_eq!(c[0].devices.iter().map(|d| d.0).collect::<Vec<_>>(), [0, 5]);
        assert!(c[0].message.starts_with(prefix), "{}", c[0].message);
        let pairs: Vec<(u16, u16)> = p.intent.compounds.iter().flat_map(|c| &c.pairs).map(|&(a, b)| (a.0.min(b.0), a.0.max(b.0))).collect();
        assert!(!pairs.iter().any(|&(a, b)| a == 5 || b == 5), "{pairs:?}");
        assert!(pairs.contains(&(0, 1)), "{pairs:?}");
    }

    /// (a): one entry pairs XM1 twice; the second pair is dropped, naming the entry.
    #[test]
    fn sidecar_double_pairing_is_a_conflict() {
        double_pairing(r#"[{"constraint":"SymmetricBlocks","direction":"V","pairs":[["XM1","XM2"],["XM1","XM6"]]}]"#, "ids 4294967295,4294967295:");
    }

    /// (a): split over two entries, both ids are named, the later one first.
    #[test]
    fn two_entries_name_both_ids() {
        double_pairing(
            r#"[{"constraint":"SymmetricBlocks","direction":"V","pairs":[["XM1","XM2"]]},{"constraint":"SymmetricBlocks","direction":"V","pairs":[["XM1","XM6"]]}]"#,
            "ids 4294967294,4294967295:",
        );
    }

    /// (c): XM1 (L 1 µm) and XM5 (L 2 µm) cannot match; the entry is dropped.
    #[test]
    fn match_on_unequal_l_is_a_conflict() {
        let (p, cfg) = annotate(&ota(), r#"[{"constraint":"Match","instances":["XM1","XM5"],"class":"moderate"}]"#);
        assert!(cfg.classes.is_empty(), "{:?}", cfg.classes);
        let c = conflicts(&p);
        assert_eq!(c.len(), 1, "{:?}", p.intent.diagnostics);
        assert_eq!(c[0].devices.iter().map(|d| d.0).collect::<Vec<_>>(), [0, 4]);
        assert!(c[0].message.starts_with("ids 4294967295:"), "{}", c[0].message);
    }

    /// (d): an extra gate on `vout1` only makes the mated outputs unequal.
    #[test]
    fn asymmetric_differential_is_reported() {
        let asym = |nl: &Netlist| {
            let p = crate::annotate(nl, &crate::AnnotationConfig::default());
            let id = p.routing.budget.iter().find(|b| b.kind().ends_with("::Differential")).and_then(|b| b.meta()).map(|m| m.id.0);
            (p.intent.diagnostics.into_iter().filter(|d| d.kind == "asymmetric_net_pair").map(|d| d.message).collect::<Vec<_>>(), id)
        };
        let mut nl = ota();
        nl.nets.extend(nets(&["y"]));
        nl.devices.push(fet("XM6", DeviceKind::Nmos, 0, 9, 3, 3, 10_000, 1_000));
        let (msgs, id) = asym(&nl);
        let id = id.expect("a Differential batch");
        assert!(!msgs.is_empty());
        for m in &msgs {
            assert!(m.starts_with(&format!("ids {id}:")), "{m}");
        }
        assert!(msgs.iter().any(|m| m.contains("vout1/vout2 3 vs 2 devices") || m.contains("vout2/vout1 2 vs 3 devices")), "{msgs:?}");
        // Plain `ota()`: vout1 = XM1/XM3, vout2 = XM2/XM4 (XM4's gate is vbias), equal.
        assert!(asym(&ota()).0.is_empty());
    }

    /// SelfDevice: a self-symmetric tail agreeing with propagation is no conflict; a self seed on a paired device is.
    #[test]
    fn self_device_conflicts_only_on_a_paired_device() {
        let two = |second: &str| annotate(&ota(), &format!(r#"[{{"constraint":"SymmetricBlocks","direction":"V","pairs":[["XM1","XM2"]]}},{{"constraint":"SymmetricBlocks","direction":"V","pairs":[[{second}]]}}]"#)).0;
        let p = two(r#""XM5""#);
        assert!(conflicts(&p).is_empty(), "{:?}", p.intent.diagnostics);
        let p = two(r#""XM1""#);
        let c = conflicts(&p);
        assert_eq!(c.len(), 1, "{:?}", p.intent.diagnostics);
        assert_eq!(c[0].devices.iter().map(|d| d.0).collect::<Vec<_>>(), [0]);
        assert!(c[0].message.starts_with("ids 4294967294,4294967295:"), "{}", c[0].message);
    }
}

/// Step-2 coverage: message format and `check`'s counting rules on a fixed
/// `Problem` (the netlist varies, the annotation does not).
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::{fet, ota};
    use pnr_core::ids::NetId;
    use pnr_core::netlist::DeviceKind;

    #[test]
    fn diag_lists_ids_in_order() {
        let d = diag(&[ConstraintId(7), ConstraintId(2)], vec![DeviceId(1)], "pull vs isolation");
        assert_eq!((d.kind, d.message.as_str(), d.devices.as_slice()), ("conflict", "ids 7,2: pull vs isolation", &[DeviceId(1)][..]));
        assert_eq!(diag(&[], vec![], 3).message, "ids : 3");
    }

    fn messages(p: &Problem, nl: &Netlist) -> Vec<String> {
        check(p, nl).into_iter().map(|d| {
            assert_eq!(d.kind, "asymmetric_net_pair");
            assert!(d.devices.is_empty());
            d.message
        }).collect()
    }

    /// `ota()` annotated: one Differential batch mating vout1/vout2.
    fn problem() -> Problem {
        crate::annotate(&ota(), &crate::AnnotationConfig::default())
    }

    #[test]
    fn a_balanced_ota_reports_nothing() {
        assert!(messages(&problem(), &ota()).is_empty());
    }

    /// A device on both of its terminals' same net counts once.
    #[test]
    fn devices_count_once_per_net() {
        let p = problem();
        let mut nl = ota();
        // vout1 gains a gate (3 devices); vout2 a device whose D and S both sit on it (3, not 4).
        nl.devices.push(fet("XG", DeviceKind::Nmos, 0, 8, 3, 3, 1_000, 1_000));
        nl.devices.push(fet("XDS", DeviceKind::Nmos, 8, 4, 4, 3, 1_000, 1_000));
        let vout = |m: Vec<String>| m.into_iter().filter(|m| m.contains("vout")).collect::<Vec<_>>();
        assert!(vout(messages(&p, &nl)).is_empty(), "{:?}", messages(&p, &nl));
        nl.devices.pop();
        let m = vout(messages(&p, &nl));
        assert!(m.iter().any(|m| m.contains("vout1/vout2 3 vs 2 devices") || m.contains("vout2/vout1 2 vs 3 devices")), "{m:?}");
    }

    /// Rails and self-pairs are never compared.
    #[test]
    fn rails_and_self_pairs_are_skipped() {
        let mut p = problem();
        let (vout1, vss, vdd) = (NetId(0), NetId(3), NetId(7));
        assert!(!p.intent.compounds.is_empty(), "ota has a compound");
        for c in &mut p.intent.compounds {
            c.net_pairs = vec![(vout1, vout1), (vss, vdd), (vout1, vdd)];
        }
        let mut nl = ota();
        nl.devices.push(fet("XG", DeviceKind::Nmos, 0, 8, 3, 3, 1_000, 1_000));
        assert!(messages(&p, &nl).is_empty());
    }

    /// Without a Differential batch there is no id to name: nothing is reported.
    #[test]
    fn no_differential_batch_no_report() {
        let mut nl = ota();
        nl.devices.truncate(1);
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        assert!(p.routing.budget.iter().all(|b| !b.kind().ends_with("::Differential")));
        assert!(check(&p, &nl).is_empty());
    }
}
