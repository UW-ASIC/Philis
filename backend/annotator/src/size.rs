//! What a device draws, as the matcher and the unitization classes compare it:
//! size, model flavour and bulk net. The one reader of the size convention
//! ([`pnr_core::MosSize`], FLOW-01) inside the annotator's matching.

use pnr_core::ids::NetId;
use pnr_core::netlist::{Device, DeviceKind};

use crate::netrole::NetRole;
use crate::param;

/// A device's drawn identity. An unknown size is `None`, never `0`: two
/// devices missing W/L are not "identical" (AA-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawn {
    /// Width of one finger, nm (`W_total / nf` for a MOS, the `w` param otherwise).
    pub w_finger_nm: Option<i64>,
    /// Drawn length, nm.
    pub l_nm: Option<i64>,
    /// Drawn units: `nf·m` for a MOS, `m` (at least 1) otherwise.
    pub fingers: u32,
    /// Index into the `models` table [`drawn`] interns into (case-insensitive).
    pub model: u16,
    /// The FET `B` net; `None` for anything else (a bipolar's `B` is its base).
    pub bulk: Option<NetId>,
}

/// MOS: from `dev.mos_size()` (FLOW-01: w_finger = W_total/nf, fingers = nf·m); `None` size → both `None`, fingers 1.
/// Other kinds: `w`, `l` params as written (`None` when absent), fingers = `m` (default 1).
/// `model` interns `dev.model` (case-insensitive) into `models`, appending a
/// new lowercase entry on first sight (linear in `models.len()`); `bulk` = the FET `B` net, else `None`.
///
/// The index is a `u16`: the netlist's `u16` device id space bounds the number
/// of distinct models, so it never wraps.
#[must_use]
pub fn drawn(dev: &Device, models: &mut Vec<String>) -> Drawn {
    let model = dev.model.to_ascii_lowercase();
    let model = models.iter().position(|m| *m == model).unwrap_or_else(|| {
        models.push(model);
        models.len() - 1
    }) as u16;
    let fet = matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos);
    let bulk = fet.then(|| dev.terminals.iter().find(|(t, _)| t == "B").map(|&(_, n)| n)).flatten();
    let p = |k: &str| dev.params.iter().find(|(n, _)| n == k).map(|&(_, v)| v);
    let (w_finger_nm, l_nm, fingers) = if fet {
        dev.mos_size().map_or((None, None, 1), |s| (Some(s.w_finger_nm()), Some(s.l_nm), s.fingers()))
    } else {
        (p("w"), p("l"), param(dev, "m", 1).clamp(1, i64::from(u32::MAX)) as u32)
    };
    Drawn { w_finger_nm, l_nm, fingers, model, bulk }
}

/// A bipolar or diode written without W/L: a fixed-geometry PDK device whose
/// size is its model (`bgr_core`'s `pnp_05v5_W3p40L3p40`).
fn fixed_geometry(kind: DeviceKind, d: &Drawn) -> bool {
    matches!(kind, DeviceKind::Npn | DeviceKind::Pnp | DeviceKind::Diode) && d.w_finger_nm.is_none() && d.l_nm.is_none()
}

/// The size is unknown: never `ExactAs`, and reported as missing `device W/L`.
/// A fixed-geometry bipolar or diode (no W and no L) is known by its model.
#[must_use]
pub fn unknown_size(kind: DeviceKind, d: &Drawn) -> bool {
    !fixed_geometry(kind, d) && (d.w_finger_nm.is_none() || d.l_nm.is_none())
}

/// `SizeMatch::ExactAs`: same known W/L (or both fixed-geometry), same model,
/// and the same bulk net or both bulks on rails. A bipolar compares its written
/// W, L and model as they are (EXT-19): its size is the model's emitter, and a
/// flow may write only a default `l`.
///
/// Panics when a bulk net id is outside `roles`.
pub(crate) fn exact_as(kind: DeviceKind, a: &Drawn, b: &Drawn, roles: &[NetRole]) -> bool {
    if matches!(kind, DeviceKind::Npn | DeviceKind::Pnp) {
        return (a.w_finger_nm, a.l_nm, a.model) == (b.w_finger_nm, b.l_nm, b.model);
    }
    let rail = |n: NetId| matches!(roles[n.0 as usize], NetRole::Supply | NetRole::Ground);
    let size = if fixed_geometry(kind, a) {
        b.w_finger_nm.is_none() && b.l_nm.is_none()
    } else {
        !unknown_size(kind, a) && (a.w_finger_nm, a.l_nm) == (b.w_finger_nm, b.l_nm)
    };
    let bulk = a.bulk == b.bulk || a.bulk.zip(b.bulk).is_some_and(|(x, y)| rail(x) && rail(y));
    size && a.model == b.model && bulk
}

/// `SizeMatch::SameLAs`: same known L, same model.
pub(crate) fn same_l_as(a: &Drawn, b: &Drawn) -> bool {
    a.l_nm.is_some() && a.l_nm == b.l_nm && a.model == b.model
}

#[cfg(test)]
mod tests {
    use crate::tests::{fet, nets};
    use crate::{annotate, block, constraints, AnnotationConfig, Block, BlockKind};
    use pnr_core::ids::DeviceId;
    use pnr_core::netlist::{DeviceKind, Netlist};

    /// Two NMOS on a shared source (`tail`), distinct gates and drains, W/L
    /// 10 µm/1 µm (`w = None` drops W). Nets: 0=inp 1=inm 2=outp 3=outm 4=tail
    /// 5=VSS 6=VDD 7=nb1 8=nb2; each device is `(model, bulk net)`.
    fn pair(devs: [(&str, u16); 2], w: Option<i64>) -> Netlist {
        let mut nl = Netlist {
            devices: vec![
                fet("XM1", DeviceKind::Nmos, 0, 2, 4, devs[0].1, 10_000, 1_000),
                fet("XM2", DeviceKind::Nmos, 1, 3, 4, devs[1].1, 10_000, 1_000),
            ],
            nets: nets(&["inp", "inm", "outp", "outm", "tail", "VSS", "VDD", "nb1", "nb2"]),
            ..Default::default()
        };
        for (d, (model, _)) in nl.devices.iter_mut().zip(devs) {
            d.model = model.into();
            if w.is_none() {
                d.params.retain(|(k, _)| k != "w");
            }
        }
        nl
    }

    fn diff_pairs(nl: &Netlist) -> usize {
        let p = annotate(nl, &AnnotationConfig::default());
        block::leaves(&p.blocks).iter().filter(|b| b.kind == BlockKind::DiffPair).count()
    }

    #[test]
    fn flavours_do_not_match() {
        let nl = pair([("nfet_01v8", 5), ("nfet_01v8_lvt", 5)], Some(10_000));
        assert_eq!(diff_pairs(&nl), 0);
        // Grouped by hand (no pattern groups them now): one drawn unit per flavour.
        let mut models = Vec::new();
        let drawn: Vec<_> = nl.devices.iter().map(|d| super::drawn(d, &mut models)).collect();
        let both = Block { kind: BlockKind::Group, template: "test", devices: vec![DeviceId(0), DeviceId(1)], injected: false, sub_blocks: Vec::new(), selfs: Vec::new() };
        assert_eq!(constraints::assemble(&nl, &drawn, &[both], &[]).unitization.len(), 2, "two unitization classes");
        // Control: one flavour, written in either case, is a pair.
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 5), ("NFET_01V8", 5)], Some(10_000))), 1);
    }

    #[test]
    fn unknown_size_never_matches() {
        let nl = pair([("nfet_01v8", 5); 2], None);
        assert_eq!(diff_pairs(&nl), 0);
        let p = annotate(&nl, &AnnotationConfig::default());
        assert!(p.coverage.iter().all(|&(_, c)| c == crate::Coverage::Unconstrained("unknown size")), "{:?}", p.coverage);
        let missing = |nl: &Netlist| annotate(nl, &AnnotationConfig::default()).missing.contains(&("MatchedSet", "device W/L"));
        assert!(missing(&nl));
        assert!(!missing(&pair([("nfet_01v8", 5); 2], Some(10_000))));
    }

    #[test]
    fn split_bulk_does_not_match() {
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 7), ("nfet_01v8", 8)], Some(10_000))), 0);
        // Control: bulks on two rails (VSS, VDD) still match.
        assert_eq!(diff_pairs(&pair([("nfet_01v8", 5), ("nfet_01v8", 6)], Some(10_000))), 1);
    }
}

/// Step-2 coverage: `drawn` per kind, interning, and the size predicates.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::fet;

    fn dev(kind: DeviceKind, model: &str, params: &[(&str, i64)]) -> Device {
        Device {
            name: "D".into(),
            kind,
            model: model.into(),
            terminals: vec![("C".into(), NetId(0)), ("B".into(), NetId(1)), ("E".into(), NetId(2))],
            params: params.iter().map(|&(k, v)| (k.into(), v)).collect(),
        }
    }

    fn d(w: Option<i64>, l: Option<i64>, model: u16, bulk: Option<u16>) -> Drawn {
        Drawn { w_finger_nm: w, l_nm: l, fingers: 1, model, bulk: bulk.map(NetId) }
    }

    /// Nets 0..4 signal, 4 supply, 5 ground.
    fn roles() -> Vec<NetRole> {
        let mut r = vec![NetRole::Signal; 4];
        r.extend([NetRole::Supply, NetRole::Ground]);
        r
    }

    #[test]
    fn a_mos_draws_per_finger_width_and_nf_times_m() {
        let mut m = fet("M", DeviceKind::Nmos, 0, 1, 2, 3, 9_000, 150);
        m.params.extend([("nf".into(), 3), ("m".into(), 2)]);
        let mut models = Vec::new();
        assert_eq!(drawn(&m, &mut models), Drawn { w_finger_nm: Some(3_000), l_nm: Some(150), fingers: 6, model: 0, bulk: Some(NetId(3)) });
        m.params.retain(|(k, _)| k != "l");
        let u = drawn(&m, &mut models);
        assert_eq!((u.w_finger_nm, u.l_nm, u.fingers), (None, None, 1), "no size: neither dimension, one finger");
    }

    #[test]
    fn other_kinds_draw_their_params_as_written() {
        let mut models = Vec::new();
        let r = drawn(&dev(DeviceKind::Resistor, "", &[("w", 500), ("l", 4_000), ("m", 3)]), &mut models);
        assert_eq!((r.w_finger_nm, r.l_nm, r.fingers, r.bulk), (Some(500), Some(4_000), 3, None));
        // A bipolar's `B` is its base, not a bulk; `m` below 1 reads as 1.
        let q = drawn(&dev(DeviceKind::Npn, "", &[("m", 0)]), &mut models);
        assert_eq!((q.w_finger_nm, q.l_nm, q.fingers, q.bulk), (None, None, 1, None));
        let big = drawn(&dev(DeviceKind::Capacitor, "", &[("m", i64::MAX)]), &mut models);
        assert_eq!(big.fingers, u32::MAX);
    }

    #[test]
    fn models_intern_case_insensitively() {
        let mut models = Vec::new();
        let ids: Vec<u16> = ["nfet", "NFET", "pfet", "", "Pfet"].iter().map(|m| drawn(&dev(DeviceKind::Resistor, m, &[]), &mut models).model).collect();
        assert_eq!(ids, [0, 0, 1, 2, 1]);
        assert_eq!(models, ["nfet", "pfet", ""]);
    }

    #[test]
    fn unknown_size_spares_fixed_geometry() {
        assert!(unknown_size(DeviceKind::Nmos, &d(None, None, 0, None)));
        assert!(unknown_size(DeviceKind::Nmos, &d(Some(1), None, 0, None)));
        assert!(unknown_size(DeviceKind::Resistor, &d(None, Some(1), 0, None)));
        assert!(!unknown_size(DeviceKind::Resistor, &d(Some(1), Some(1), 0, None)));
        for k in [DeviceKind::Npn, DeviceKind::Pnp, DeviceKind::Diode] {
            assert!(!unknown_size(k, &d(None, None, 0, None)), "{k:?}");
        }
        assert!(!unknown_size(DeviceKind::Diode, &d(Some(1), Some(1), 0, None)));
        assert!(unknown_size(DeviceKind::Diode, &d(Some(1), None, 0, None)), "half a size is not fixed geometry");
    }

    #[test]
    fn exact_as_needs_size_model_and_bulk() {
        let k = DeviceKind::Nmos;
        let r = roles();
        let a = d(Some(1_000), Some(150), 0, Some(0));
        assert!(exact_as(k, &a, &a, &r));
        assert!(!exact_as(k, &a, &d(Some(1_000), Some(160), 0, Some(0)), &r));
        assert!(!exact_as(k, &a, &d(Some(1_000), Some(150), 1, Some(0)), &r), "model");
        assert!(!exact_as(k, &a, &d(Some(1_000), Some(150), 0, Some(1)), &r), "split signal bulks");
        // Bulks on two rails match; a rail and a signal do not.
        let on = |n| d(Some(1_000), Some(150), 0, Some(n));
        assert!(exact_as(k, &on(4), &on(5), &r));
        assert!(!exact_as(k, &on(4), &on(0), &r));
        assert!(!exact_as(k, &on(4), &d(Some(1_000), Some(150), 0, None), &r));
        // Two unknown sizes are never identical.
        let u = d(None, None, 0, Some(0));
        assert!(!exact_as(k, &u, &u, &r));
    }

    #[test]
    fn fixed_geometry_matches_only_fixed_geometry() {
        let r = roles();
        let f = d(None, None, 3, None);
        assert!(exact_as(DeviceKind::Diode, &f, &f, &r));
        assert!(!exact_as(DeviceKind::Diode, &f, &d(Some(1), Some(1), 3, None), &r));
        assert!(!exact_as(DeviceKind::Diode, &d(Some(1), Some(1), 3, None), &f, &r));
        assert!(!exact_as(DeviceKind::Diode, &f, &d(None, None, 4, None), &r), "model is the size");
    }

    /// A bipolar compares W, L and model as written, bulk ignored.
    #[test]
    fn bipolars_compare_as_written() {
        let r = roles();
        let a = d(None, Some(1), 0, None);
        assert!(exact_as(DeviceKind::Npn, &a, &a, &r));
        assert!(exact_as(DeviceKind::Pnp, &a, &d(None, Some(1), 0, Some(1)), &r));
        assert!(!exact_as(DeviceKind::Pnp, &a, &d(None, Some(2), 0, None), &r));
        assert!(!exact_as(DeviceKind::Pnp, &a, &d(None, Some(1), 1, None), &r));
    }

    #[test]
    fn same_l_as_needs_a_known_l_and_one_model() {
        let a = d(Some(1), Some(150), 0, None);
        assert!(same_l_as(&a, &d(Some(9), Some(150), 0, Some(3))));
        assert!(!same_l_as(&a, &d(Some(1), Some(150), 1, None)));
        assert!(!same_l_as(&a, &d(Some(1), Some(151), 0, None)));
        let none = d(Some(1), None, 0, None);
        assert!(!same_l_as(&none, &none));
    }
}
