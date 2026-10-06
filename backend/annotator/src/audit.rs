//! Structure and bias audit (EXT-29): Hastings' checks on a matched set's bias
//! and geometry, reported as diagnostics. Never alters a constraint.

use analog::intent::{Diagnostic, Family, Intent, MatchClass, MatchKind, MatchSpec};
use analog::matching::class::{limit, ClassLimit};
use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

use crate::evidence::OpFacts;

/// Every kind this module emits (the metadata report filters on it).
pub const KINDS: [&str; 7] = ["vgst_low", "clm_mismatch", "cascode_ratio", "cascode_bulk", "bjt_ratio", "vce_unequal", "audit_not_checked"];

/// H13-32: a current-matched FET below this overdrive, mV (Hastings: V_GT ≥ 0.1 V for Moderate+).
const VGST_MIN_MV: f64 = 100.0;
/// H09-23: unequal V_CE across a matched bipolar set, mV (**Philis threshold**).
const VCE_MAX_MV: f64 = 10.0;

/// A Moderate+ MOS current-matched set: the H13-32/33 checks apply.
fn mos_current(s: &MatchSpec) -> bool {
    s.kind == MatchKind::Current && s.class >= MatchClass::Moderate && s.family == Family::Mos
}

/// A Moderate+ bipolar set: the H09-04/23 checks apply.
fn bjt(s: &MatchSpec) -> bool {
    s.family == Family::Bipolar && s.class >= MatchClass::Moderate
}

/// H13-32/33/34, H09-04/23 checks. `cascodes`: slot order 0 bottom ref, 1 bottom out, 2 top ref, 3 top out.
/// `vgst_low`: a Moderate+ MOS Current set's members under 100 mV of `|V_GS|−|V_th|` (else `2·Id/gm`);
/// `clm_mismatch`: `gds_ref·|ΔV_DS|/Id_ref` over half the class's % limit, per member against the
/// reference; `cascode_ratio`: bottom W/L ratio ≠ top ratio (> 1 %); `cascode_bulk`: a top device's B ≠ S;
/// `bjt_ratio`: a Moderate+ bipolar ratio over 16 or odd (no common centroid); `vce_unequal`: its V_CE spread
/// over 10 mV; `audit_not_checked`: the op-dependent checks a set qualified for whose data (op point,
/// a member's dev entry, `gds_us`, or a pin's `net_mv`) is absent, at most one, last, naming each check once.
/// The reference is slot `reference` (clamped to the last member), default 0; a set without members is
/// skipped. A cascode whose four devices do not all have a W/L skips `cascode_ratio`.
///
/// # Panics
/// When a member or cascode device id is out of bounds of `nl.devices`.
#[must_use]
pub fn audit(intent: &Intent, nl: &Netlist, cascodes: &[[DeviceId; 4]], op: Option<&OpFacts>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let name = |d: DeviceId| nl.devices[d.0 as usize].name.as_str();
    let pin = |d: DeviceId, t: &str| nl.devices[d.0 as usize].terminals.iter().find(|(x, _)| x == t).map(|&(_, n)| n);
    let mv = |d: DeviceId, a: &str, b: &str| {
        let o = op?;
        let v = |t: &str| o.net_mv.get(pin(d, t)?.0 as usize).copied().flatten();
        Some(v(a)? - v(b)?)
    };
    let dop = |d: DeviceId| op.and_then(|o| o.dev.get(d.0 as usize).copied().flatten());
    let mut skipped: Vec<&str> = Vec::new();
    for s in &intent.sets {
        let ids: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if mos_current(s) {
            let vgst = |d: DeviceId| {
                let o = dop(d)?;
                match (o.vgs_mv, o.vth_mv) {
                    (Some(g), Some(t)) => Some(g.abs() - t.abs()),
                    _ => (o.gm_us > 0.0).then(|| 2.0 * o.id_ua.abs() / o.gm_us * 1000.0),
                }
            };
            if ids.iter().any(|&d| vgst(d).is_none()) {
                skipped.push("vgst_low");
            }
            let low: Vec<DeviceId> = ids.iter().copied().filter(|&d| vgst(d).is_some_and(|v| v < VGST_MIN_MV)).collect();
            if !low.is_empty() {
                let msg = format!("V_GS−V_th < {VGST_MIN_MV} mV on {:?}: current matching degrades (Hastings H13-32)", low.iter().map(|&d| name(d)).collect::<Vec<_>>());
                out.push(Diagnostic { kind: "vgst_low", devices: low, message: msg });
            }
            let r = ids[s.reference.unwrap_or(0).min(ids.len() - 1)];
            let qualifies = matches!(limit(s.family, MatchKind::Current, s.class), Some(ClassLimit::Pct(_)));
            if qualifies && (op.is_none() || dop(r).and_then(|o| o.gds_us).is_none() || ids.iter().any(|&d| mv(d, "D", "S").is_none())) {
                skipped.push("clm_mismatch");
            }
            if let (Some(o), Some(ClassLimit::Pct(x)), Some(vr)) = (dop(r), limit(s.family, MatchKind::Current, s.class), mv(r, "D", "S")) {
                for &d in ids.iter().filter(|&&d| d != r) {
                    let (Some(gds), Some(vd)) = (o.gds_us, mv(d, "D", "S")) else { continue };
                    let pct = 100.0 * gds * (vd - vr).abs() * 1e-3 / o.id_ua.abs();
                    if o.id_ua != 0.0 && pct > f64::from(x) / 2.0 {
                        let msg = format!("ΔV_DS {:.0} mV gives {pct:.2} % current error > {:.2} % (half the class limit, H13-33)", (vd - vr).abs(), x / 2.0);
                        out.push(Diagnostic { kind: "clm_mismatch", devices: vec![r, d], message: msg });
                    }
                }
            }
        }
        if bjt(s) {
            let units: Vec<u32> = s.members.iter().map(|m| u32::from(m.parallel) * u32::from(m.series)).collect();
            let min = units.iter().copied().min().unwrap_or(1).max(1);
            if units.iter().any(|&u| u / min > 16 || (u / min > 1 && (u / min) % 2 == 1)) {
                out.push(Diagnostic { kind: "bjt_ratio", devices: ids.clone(), message: format!("unit ratio {units:?}: over 16 or odd, no common centroid (Hastings H09-04)") });
            }
            let vce: Option<Vec<f64>> = ids.iter().map(|&d| mv(d, "C", "E")).collect();
            if vce.is_none() {
                skipped.push("vce_unequal");
            }
            if let Some(v) = vce.filter(|v| !v.is_empty()) {
                let spread = v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min);
                if spread > VCE_MAX_MV {
                    out.push(Diagnostic { kind: "vce_unequal", devices: ids.clone(), message: format!("V_CE spread {spread:.0} mV > {VCE_MAX_MV} mV (Early effect, H09-23)") });
                }
            }
        }
    }
    let ratio = |d: DeviceId| nl.devices[d.0 as usize].mos_size().map(|m| m.w_total_nm as f64 * f64::from(m.m) / m.l_nm as f64);
    for c in cascodes {
        if let [Some(r0), Some(r1), Some(r2), Some(r3)] = c.map(ratio) {
            let (bottom, top) = (r0 / r1, r2 / r3);
            if (bottom - top).abs() > 0.01 * top {
                out.push(Diagnostic { kind: "cascode_ratio", devices: c.to_vec(), message: format!("bottom W/L ratio {bottom:.3} ≠ top {top:.3} (Hastings H13-34)") });
            }
        }
        for &d in &c[2..] {
            if pin(d, "B") != pin(d, "S") {
                out.push(Diagnostic { kind: "cascode_bulk", devices: vec![d], message: format!("{}: cascode bulk not tied to its source (body effect, H13-34)", name(d)) });
            }
        }
    }
    skipped.sort_unstable();
    skipped.dedup();
    if !skipped.is_empty() {
        out.push(Diagnostic { kind: "audit_not_checked", devices: vec![], message: format!("no op data: {} not checked", skipped.join(", ")) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::DeviceOp;
    use crate::tests::fet;
    use analog::intent::{ArrayStyle, ClassSource, ConstraintId, Member, Origin};
    use pnr_core::netlist::{Device, DeviceKind};

    fn set(family: Family, class: MatchClass, units: &[u16]) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: units.iter().enumerate().map(|(i, &p)| Member { device: DeviceId(i as u16), parallel: p, series: 1, half: None }).collect(),
            reference: None,
            family,
            kind: MatchKind::Current,
            class,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn intent(s: MatchSpec) -> Intent {
        Intent { sets: vec![s], ..Default::default() }
    }

    /// Two NMOS on nets 0=g 1=d0 2=d1 3=vss.
    fn pair() -> Netlist {
        Netlist { devices: vec![fet("M0", DeviceKind::Nmos, 0, 1, 3, 3, 2_000, 1_000), fet("M1", DeviceKind::Nmos, 0, 2, 3, 3, 2_000, 1_000)], nets: crate::tests::nets(&["g", "d0", "d1", "vss"]), ..Default::default() }
    }

    fn op(id_ua: f64, gm_us: f64, gds_us: Option<f64>, net_mv: Vec<Option<f64>>) -> OpFacts {
        let d = DeviceOp { id_ua, headroom_mv: 100.0, gm_us, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us };
        OpFacts { dev: vec![Some(d); 2], net_mv }
    }

    fn kinds(d: &[Diagnostic]) -> Vec<&str> {
        d.iter().map(|x| x.kind).collect()
    }

    #[test]
    fn vgst_floor() {
        let (nl, i) = (pair(), intent(set(Family::Mos, MatchClass::Moderate, &[1, 1])));
        let d = audit(&i, &nl, &[], Some(&op(0.6, 20.0, Some(1.0), vec![Some(0.0); 4])));
        assert_eq!(kinds(&d), ["vgst_low"]);
        assert_eq!(d[0].devices, [DeviceId(0), DeviceId(1)]);
        assert!(audit(&i, &nl, &[], Some(&op(0.6, 10.0, Some(1.0), vec![Some(0.0); 4]))).is_empty());
    }

    #[test]
    fn bjt_ratio_odd() {
        let nl = pair(); // the ratio reads units only; no C/E pins, so V_CE is never checked
        let k = |c, u: &[u16]| kinds(&audit(&intent(set(Family::Bipolar, c, u)), &nl, &[], Some(&op(1.0, 1.0, None, vec![])))).join(" ");
        assert_eq!(k(MatchClass::Moderate, &[1, 7]), "bjt_ratio audit_not_checked");
        assert_eq!(k(MatchClass::Moderate, &[1, 8]), "audit_not_checked");
        assert_eq!(k(MatchClass::Minimal, &[1, 7]), "");
    }

    #[test]
    fn cascode_ratio_mismatch() {
        let n = DeviceKind::Nmos;
        let mk = |w0: i64, w1: i64| -> Vec<Device> {
            vec![fet("B0", n, 0, 0, 3, 3, w0, 1_000), fet("B1", n, 0, 1, 3, 3, w1, 1_000), fet("T0", n, 2, 2, 0, 0, 2_000, 500), fet("T1", n, 2, 4, 1, 1, 2_000, 500)]
        };
        let nl = |w0, w1| Netlist { devices: mk(w0, w1), nets: crate::tests::nets(&["a", "b", "c", "vss", "o"]), ..Default::default() };
        let c = [[DeviceId(0), DeviceId(1), DeviceId(2), DeviceId(3)]];
        assert_eq!(kinds(&audit(&Intent::default(), &nl(2_000, 4_000), &c, None)), ["cascode_ratio"]);
        assert!(audit(&Intent::default(), &nl(2_000, 2_000), &c, None).is_empty());
        let mut tied = nl(2_000, 2_000);
        tied.devices[3] = fet("T1", n, 2, 4, 1, 3, 2_000, 500); // top bulk on vss, source on b
        assert_eq!(kinds(&audit(&Intent::default(), &tied, &c, None)), ["cascode_bulk"]);
    }

    #[test]
    fn clm_from_vds() {
        let (nl, i) = (pair(), intent(set(Family::Mos, MatchClass::Moderate, &[1, 1])));
        // gm 1 µS: V_GT 20 V, no vgst_low. ΔV_DS 200 mV · 1 µS / 10 µA = 2 % > 1.5 %.
        let vds = |d1: f64| vec![Some(500.0), Some(500.0), Some(d1), Some(0.0)];
        assert_eq!(kinds(&audit(&i, &nl, &[], Some(&op(10.0, 1.0, Some(1.0), vds(700.0))))), ["clm_mismatch"]);
        assert!(audit(&i, &nl, &[], Some(&op(10.0, 1.0, Some(1.0), vds(600.0)))).is_empty());
    }

    #[test]
    fn no_op_not_checked() {
        let (nl, i) = (pair(), intent(set(Family::Mos, MatchClass::Moderate, &[1, 1])));
        assert_eq!(kinds(&audit(&i, &nl, &[], None)), ["audit_not_checked"]);
    }

    /// An op point without `gds_us` still reports `clm_mismatch` as not checked.
    #[test]
    fn missing_gds_not_checked() {
        let (nl, i) = (pair(), intent(set(Family::Mos, MatchClass::Moderate, &[1, 1])));
        let d = audit(&i, &nl, &[], Some(&op(10.0, 1.0, None, vec![Some(0.0); 4])));
        assert_eq!(kinds(&d), ["audit_not_checked"]);
        assert!(d[0].message.contains("clm_mismatch") && !d[0].message.contains("vgst_low"), "{}", d[0].message);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::evidence::DeviceOp;
    use crate::tests::{fet, nets};
    use analog::intent::{ArrayStyle, ClassSource, ConstraintId, Member, Origin};
    use pnr_core::ids::NetId;
    use pnr_core::netlist::{Device, DeviceKind};

    fn set(family: Family, class: MatchClass, units: &[u16], reference: Option<usize>) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: units.iter().enumerate().map(|(i, &p)| Member { device: DeviceId(i as u16), parallel: p, series: 1, half: None }).collect(),
            reference,
            family,
            kind: MatchKind::Current,
            class,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn intent(s: MatchSpec) -> Intent {
        Intent { sets: vec![s], ..Default::default() }
    }

    fn dop(id_ua: f64, gm_us: f64, gds_us: Option<f64>) -> DeviceOp {
        DeviceOp { id_ua, headroom_mv: 100.0, gm_us, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us }
    }

    fn kinds(d: &[Diagnostic]) -> Vec<&str> {
        d.iter().map(|x| x.kind).collect()
    }

    /// NPNs Q0 (C on 0), Q1 (C on 1), common B 2 and E 3.
    fn bjts() -> Netlist {
        let q = |name: &str, c: u16| Device {
            name: name.into(),
            kind: DeviceKind::Npn,
            model: String::new(),
            terminals: vec![("C".into(), NetId(c)), ("B".into(), NetId(2)), ("E".into(), NetId(3))],
            params: vec![],
        };
        Netlist { devices: vec![q("Q0", 0), q("Q1", 1)], nets: nets(&["c0", "c1", "b", "e"]), ..Default::default() }
    }

    #[test]
    fn nothing_to_audit() {
        assert!(audit(&Intent::default(), &Netlist::default(), &[], None).is_empty());
    }

    /// An empty set has nothing to compare: skipped, never a panic.
    #[test]
    fn empty_set_is_skipped() {
        let nl = bjts();
        assert!(audit(&intent(set(Family::Mos, MatchClass::Moderate, &[], None)), &nl, &[], None).is_empty());
        assert!(audit(&intent(set(Family::Bipolar, MatchClass::Moderate, &[], None)), &nl, &[], None).is_empty());
    }

    #[test]
    fn minimal_and_voltage_sets_are_not_audited() {
        let nl = Netlist { devices: vec![fet("M0", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000), fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000)], nets: nets(&["g", "d", "s"]), ..Default::default() };
        assert!(audit(&intent(set(Family::Mos, MatchClass::Minimal, &[1, 1], None)), &nl, &[], None).is_empty());
        let v = MatchSpec { kind: MatchKind::Voltage, ..set(Family::Mos, MatchClass::Moderate, &[1, 1], None) };
        assert!(audit(&intent(v), &nl, &[], None).is_empty());
    }

    #[test]
    fn vgst_from_vgs_and_vth() {
        let nl = Netlist { devices: vec![fet("M0", DeviceKind::Pmos, 0, 1, 2, 2, 1_000, 1_000), fet("M1", DeviceKind::Pmos, 0, 1, 2, 2, 1_000, 1_000)], nets: nets(&["g", "d", "s"]), ..Default::default() };
        // |−450| − |−400| = 50 mV < 100 mV on both (signs ignored); gm alone would say 2 V.
        let low = DeviceOp { vgs_mv: Some(-450.0), vth_mv: Some(-400.0), ..dop(10.0, 10.0, Some(0.0)) };
        let op = OpFacts { dev: vec![Some(low); 2], net_mv: vec![Some(0.0); 3] };
        assert_eq!(kinds(&audit(&intent(set(Family::Mos, MatchClass::Moderate, &[1, 1], None)), &nl, &[], Some(&op))), ["vgst_low"]);
        // Exactly 100 mV is not low.
        let edge = DeviceOp { vgs_mv: Some(500.0), vth_mv: Some(400.0), ..low };
        let op = OpFacts { dev: vec![Some(edge); 2], net_mv: vec![Some(0.0); 3] };
        assert!(audit(&intent(set(Family::Mos, MatchClass::Moderate, &[1, 1], None)), &nl, &[], Some(&op)).is_empty());
    }

    #[test]
    fn reference_is_clamped_to_the_last_member() {
        let nl = Netlist { devices: vec![fet("M0", DeviceKind::Nmos, 0, 1, 3, 3, 1_000, 1_000), fet("M1", DeviceKind::Nmos, 0, 2, 3, 3, 1_000, 1_000)], nets: nets(&["g", "d0", "d1", "s"]), ..Default::default() };
        let op = OpFacts { dev: vec![Some(dop(10.0, 1.0, Some(1.0))); 2], net_mv: vec![Some(0.0), Some(500.0), Some(700.0), Some(0.0)] };
        let d = audit(&intent(set(Family::Mos, MatchClass::Moderate, &[1, 1], Some(7))), &nl, &[], Some(&op));
        assert_eq!(kinds(&d), ["clm_mismatch"]);
        assert_eq!(d[0].devices, [DeviceId(1), DeviceId(0)]);
    }

    #[test]
    fn bjt_ratio_bounds() {
        let nl = bjts();
        let op = OpFacts { dev: vec![], net_mv: vec![Some(500.0), Some(500.0), Some(0.0), Some(0.0)] };
        let k = |u: &[u16]| kinds(&audit(&intent(set(Family::Bipolar, MatchClass::Moderate, u, None)), &nl, &[], Some(&op))).join(" ");
        assert_eq!(k(&[1, 1]), "");
        assert_eq!(k(&[1, 16]), "");
        assert_eq!(k(&[1, 32]), "bjt_ratio");
        assert_eq!(k(&[2, 6]), "bjt_ratio", "ratio 3 is odd");
        assert_eq!(k(&[0, 4]), "", "a zero unit reads as 1: ratio 4");
    }

    #[test]
    fn vce_spread() {
        let nl = bjts();
        let k = |c1: f64| {
            let op = OpFacts { dev: vec![], net_mv: vec![Some(500.0), Some(c1), Some(0.0), Some(0.0)] };
            kinds(&audit(&intent(set(Family::Bipolar, MatchClass::Moderate, &[1, 1], None)), &nl, &[], Some(&op))).join(" ")
        };
        assert_eq!(k(520.0), "vce_unequal");
        assert_eq!(k(510.0), "", "10 mV is the limit, not over it");
        assert_eq!(kinds(&audit(&intent(set(Family::Bipolar, MatchClass::Moderate, &[1, 1], None)), &nl, &[], None)), ["audit_not_checked"]);
    }

    #[test]
    fn cascode_without_size_skips_ratio_but_checks_bulk() {
        let n = DeviceKind::Nmos;
        let mut bare = fet("T1", n, 2, 4, 1, 3, 2_000, 500);
        bare.params.clear();
        let nl = Netlist { devices: vec![fet("B0", n, 0, 0, 3, 3, 2_000, 1_000), fet("B1", n, 0, 1, 3, 3, 9_000, 1_000), fet("T0", n, 2, 2, 0, 0, 2_000, 500), bare], nets: nets(&["a", "b", "c", "vss", "o"]), ..Default::default() };
        let d = audit(&Intent::default(), &nl, &[[DeviceId(0), DeviceId(1), DeviceId(2), DeviceId(3)]], None);
        assert_eq!(kinds(&d), ["cascode_bulk"]);
        assert_eq!(d[0].devices, [DeviceId(3)]);
    }

    #[test]
    fn not_checked_is_one_last_sorted_diagnostic() {
        let mut i = intent(set(Family::Mos, MatchClass::Moderate, &[1, 1], None));
        i.sets.push(set(Family::Bipolar, MatchClass::Moderate, &[1, 7], None));
        i.sets.push(set(Family::Mos, MatchClass::Moderate, &[1, 1], None));
        let d = audit(&i, &bjts(), &[], None);
        assert_eq!(kinds(&d), ["bjt_ratio", "audit_not_checked"]);
        assert_eq!(d[1].message, "no op data: clm_mismatch, vce_unequal, vgst_low not checked");
    }
}
