//! REL-10: operating-point voltage checks. Each FET's |V_GS| and |V_DS|
//! against the deck's `gate_oxide` / `drain_source` ratings
//! ([`verify::Pdk::fet_voltage_limits`]), forward-biased bulk junctions, and
//! the bias asymmetry of recognised matched pairs (reported, not judged).

use crate::oppoint::OpPoint;
use pnr_core::report::Violation;
use pnr_core::{DeviceId, DeviceKind, Netlist};

/// Bulk junction forward-bias tolerance, V: below solver noise, far below a
/// diode turn-on, so only a real wrong-bulk tie trips it.
const FORWARD_TOL_V: f64 = 1e-3;

/// A recognised matched pair's bias asymmetry, mV, each `|a − b|` (report
/// only: no source gives a threshold, H05-33/35).
#[derive(Clone, Debug, PartialEq)]
pub struct PairAging {
    /// First member, as the pair was given.
    pub a: DeviceId,
    /// Second member.
    pub b: DeviceId,
    /// `|V_DS,a − V_DS,b|`, mV.
    pub dvds_mv: f64,
    /// `|V_GS,a − V_GS,b|`, mV.
    pub dvgs_mv: f64,
    /// `|V_BS,a − V_BS,b|`, mV.
    pub dvbs_mv: f64,
}

/// Hard rows `rel/vgs:{name}` / `rel/vds:{name}` (margin = overshoot mV,
/// rounded up) for a FET over its model's rating, and `rel/bulk_forward:{name}`
/// for a bulk junction forward-biased past [`FORWARD_TOL_V`]; each `pairs`
/// entry whose members both have all three voltages as a [`PairAging`]; and
/// the count of FETs whose rating could not be checked (a voltage unresolved,
/// or no limit names its model). A limit's `None` field is not checked and not
/// unknown: the deck states no such rule. The bulk check needs no limit.
/// With `probe` (the op came from a synthesised mid-rail probe, not a
/// testbench) every row's rule ends ` (probe)`, so a signoff report never
/// passes probe voltages off as a testbench finding.
///
/// A voltage that is not finite (NaN, ±∞) reads as unresolved: the FET counts
/// as unknown and no row or pair entry is built from it. A device index past
/// `op`'s columns is unresolved too; a pair naming one is left out.
#[must_use]
pub fn voltage_findings(
    netlist: &Netlist,
    op: &OpPoint,
    limits: &[verify::FetLimit],
    pairs: &[(DeviceId, DeviceId)],
    probe: bool,
) -> (Vec<Violation>, Vec<PairAging>, usize) {
    let src = if probe { " (probe)" } else { "" };
    // `(V_GS, V_DS, V_BS)` of device `i`, V, when all three are resolved.
    let v3 = |i: usize| Some((*op.vgs_v.get(i)?.as_ref()?, *op.vds_v.get(i)?.as_ref()?, *op.vbs_v.get(i)?.as_ref()?));
    let (mut rows, mut unknown) = (Vec::new(), 0);
    for (i, d) in netlist.devices.iter().enumerate() {
        if !matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            continue;
        }
        let Some((vgs, vds, vbs)) = v3(i) else {
            unknown += 1;
            continue;
        };
        match limits.iter().find(|l| l.model == d.model) {
            None => unknown += 1,
            Some(l) => {
                for (tag, v, max) in [("vgs", vgs, l.vgs_max_mv), ("vds", vds, l.vds_max_mv)] {
                    let over = v.abs() * 1e3 - max.map_or(f64::INFINITY, f64::from);
                    if over > 0.0 {
                        rows.push(Violation { rule: format!("rel/{tag}:{}{src}", d.name), margin: over.ceil() as i64 });
                    }
                }
            }
        }
        // Body→source and body→drain (V_BD = V_BS − V_DS). ngspice prints a
        // PMOS's voltages polarity-normalised (V_DS > 0 in saturation, see
        // `oppoint::tests::show_reads_vgs_and_vbs`), so positive is forward
        // for both polarities.
        let fwd = vbs.max(vbs - vds);
        if fwd > FORWARD_TOL_V {
            rows.push(Violation { rule: format!("rel/bulk_forward:{}{src}", d.name), margin: ((fwd * 1e3).ceil() as i64).max(1) });
        }
    }
    let aging = pairs
        .iter()
        .filter_map(|&(a, b)| {
            let ((ga, da, ba), (gb, db, bb)) = (v3(a.0 as usize)?, v3(b.0 as usize)?);
            Some(PairAging { a, b, dvds_mv: (da - db).abs() * 1e3, dvgs_mv: (ga - gb).abs() * 1e3, dvbs_mv: (ba - bb).abs() * 1e3 })
        })
        .collect();
    (rows, aging, unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::Device;

    fn nfet(name: &str, model: &str) -> Device {
        Device { name: name.into(), kind: DeviceKind::Nmos, model: model.into(), terminals: Vec::new(), params: Vec::new() }
    }

    fn pfet(name: &str) -> Device {
        Device { kind: DeviceKind::Pmos, ..nfet(name, "sky130_fd_pr__pfet_01v8") }
    }

    fn run(devs: Vec<Device>, v: &[(f64, f64, f64)], pairs: &[(DeviceId, DeviceId)]) -> (Vec<Violation>, Vec<PairAging>, usize) {
        let netlist = Netlist { devices: devs, ..Default::default() };
        let op = OpPoint {
            vgs_v: v.iter().map(|t| Some(t.0)).collect(),
            vds_v: v.iter().map(|t| Some(t.1)).collect(),
            vbs_v: v.iter().map(|t| Some(t.2)).collect(),
            ..Default::default()
        };
        let lim = verify::FetLimit { model: "sky130_fd_pr__nfet_01v8".into(), vgs_max_mv: Some(1950.0), vds_max_mv: Some(1950.0) };
        voltage_findings(&netlist, &op, &[lim], pairs, false)
    }

    #[test]
    fn a_gate_over_its_rating_is_a_violation() {
        let (rows, _, unknown) = run(vec![nfet("M1", "sky130_fd_pr__nfet_01v8")], &[(2.5, 0.9, 0.0)], &[]);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].rule.starts_with("rel/vgs:"), "{}", rows[0].rule);
        assert_eq!(rows[0].margin, 550);
        assert_eq!(unknown, 0);
    }

    #[test]
    fn a_forward_biased_nmos_bulk_is_a_violation() {
        let m = "sky130_fd_pr__nfet_01v8";
        let (rows, ..) = run(vec![nfet("M1", m)], &[(0.8, 0.9, 0.2)], &[]);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].rule.starts_with("rel/bulk_forward:"), "{}", rows[0].rule);
        assert_eq!(rows[0].margin, 200);
        assert!(run(vec![nfet("M1", m)], &[(0.8, 0.9, 0.0)], &[]).0.is_empty());
        let (rows, _, unknown) = run(vec![nfet("M1", "no_such_fet")], &[(2.5, 0.9, 0.2)], &[]);
        assert_eq!(unknown, 1);
        assert!(!rows.iter().any(|r| r.rule.starts_with("rel/vgs:")));
        assert!(rows.iter().any(|r| r.rule.starts_with("rel/bulk_forward:")), "the bulk check needs no limit");
    }

    /// A PMOS load with B = S = VDD, as ngspice prints it (normalised:
    /// V_DS = +1.79 V, V_BS = 0, `oppoint::tests::show_reads_vgs_and_vbs`), is
    /// not forward; a bulk 0.2 V below its source (normalised V_BS = +0.2) is.
    #[test]
    fn a_pmos_reads_polarity_normalised_voltages() {
        assert!(run(vec![pfet("M3")], &[(1.05, 1.788, 0.0)], &[]).0.is_empty());
        assert!(run(vec![pfet("M3")], &[(1.05, 1.788, -0.3)], &[]).0.is_empty());
        let (rows, ..) = run(vec![pfet("M3")], &[(1.05, 1.788, 0.2)], &[]);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].rule.starts_with("rel/bulk_forward:"), "{}", rows[0].rule);
    }

    /// A probe op's rows carry the probe provenance; a testbench op's do not.
    #[test]
    fn a_probe_op_tags_its_rows() {
        let netlist = Netlist { devices: vec![nfet("M1", "sky130_fd_pr__nfet_01v8")], ..Default::default() };
        let op = OpPoint { vgs_v: vec![Some(2.5)], vds_v: vec![Some(2.5)], vbs_v: vec![Some(0.2)], ..Default::default() };
        let lim = verify::FetLimit { model: "sky130_fd_pr__nfet_01v8".into(), vgs_max_mv: Some(1950.0), vds_max_mv: Some(1950.0) };
        let rules = |probe| voltage_findings(&netlist, &op, std::slice::from_ref(&lim), &[], probe).0.into_iter().map(|r| r.rule).collect::<Vec<_>>();
        assert_eq!(rules(true), ["rel/vgs:M1 (probe)", "rel/vds:M1 (probe)", "rel/bulk_forward:M1 (probe)"]);
        assert_eq!(rules(false), ["rel/vgs:M1", "rel/vds:M1", "rel/bulk_forward:M1"]);
    }

    #[test]
    fn pair_aging_reports_the_vds_difference() {
        let m = "sky130_fd_pr__nfet_01v8";
        let (_, aging, _) = run(vec![nfet("M1", m), nfet("M2", m)], &[(0.8, 0.9, 0.0), (0.8, 0.6, 0.0)], &[(DeviceId(0), DeviceId(1))]);
        assert_eq!(aging.len(), 1);
        assert!((aging[0].dvds_mv - 300.0).abs() < 1e-9, "{}", aging[0].dvds_mv);
        assert_eq!(aging[0].dvbs_mv, 0.0);
        assert_eq!(aging[0].dvgs_mv, 0.0);
    }
}

/// Step-2 coverage of [`voltage_findings`]: every branch, boundary and the
/// unresolved / non-finite paths. Oracles are the doc contract and the
/// physics of a forward-biased junction.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::Device;

    const NFET: &str = "sky130_fd_pr__nfet_01v8";

    fn dev(name: &str, kind: DeviceKind, model: &str) -> Device {
        Device { name: name.into(), kind, model: model.into(), terminals: Vec::new(), params: Vec::new() }
    }

    fn op(v: &[Option<(f64, f64, f64)>]) -> OpPoint {
        OpPoint {
            vgs_v: v.iter().map(|t| t.map(|t| t.0)).collect(),
            vds_v: v.iter().map(|t| t.map(|t| t.1)).collect(),
            vbs_v: v.iter().map(|t| t.map(|t| t.2)).collect(),
            ..Default::default()
        }
    }

    fn lim(vgs: Option<f32>, vds: Option<f32>) -> verify::FetLimit {
        verify::FetLimit { model: NFET.into(), vgs_max_mv: vgs, vds_max_mv: vds }
    }

    fn find(devs: Vec<Device>, v: &[Option<(f64, f64, f64)>], l: &[verify::FetLimit], pairs: &[(DeviceId, DeviceId)]) -> (Vec<Violation>, Vec<PairAging>, usize) {
        let nl = Netlist { devices: devs, ..Default::default() };
        voltage_findings(&nl, &op(v), l, pairs, false)
    }

    #[test]
    fn an_empty_netlist_has_no_findings() {
        let (rows, aging, unknown) = find(Vec::new(), &[], &[], &[]);
        assert!(rows.is_empty() && aging.is_empty());
        assert_eq!(unknown, 0);
    }

    #[test]
    fn non_fets_are_neither_checked_nor_unknown() {
        let devs = vec![dev("R1", DeviceKind::Resistor, ""), dev("C1", DeviceKind::Capacitor, ""), dev("Q1", DeviceKind::Npn, "")];
        let (rows, _, unknown) = find(devs, &[Some((9.0, 9.0, 9.0)); 3], &[lim(Some(1.0), Some(1.0))], &[]);
        assert!(rows.is_empty(), "{rows:?}");
        assert_eq!(unknown, 0);
    }

    #[test]
    fn an_unresolved_or_out_of_range_fet_is_unknown() {
        let devs = vec![dev("M1", DeviceKind::Nmos, NFET), dev("M2", DeviceKind::Nmos, NFET)];
        // M1 unresolved, M2 past op's columns.
        let (rows, _, unknown) = find(devs, &[None], &[lim(Some(1950.0), Some(1950.0))], &[]);
        assert!(rows.is_empty());
        assert_eq!(unknown, 2);
    }

    #[test]
    fn a_partially_resolved_fet_is_unknown() {
        let nl = Netlist { devices: vec![dev("M1", DeviceKind::Nmos, NFET)], ..Default::default() };
        let o = OpPoint { vgs_v: vec![Some(3.0)], vds_v: vec![Some(3.0)], vbs_v: vec![None], ..Default::default() };
        let (rows, _, unknown) = voltage_findings(&nl, &o, &[lim(Some(1950.0), Some(1950.0))], &[], false);
        assert!(rows.is_empty());
        assert_eq!(unknown, 1);
    }

    #[test]
    fn a_none_limit_field_is_not_checked_and_not_unknown() {
        let (rows, _, unknown) = find(vec![dev("M1", DeviceKind::Nmos, NFET)], &[Some((5.0, 5.0, 0.0))], &[lim(None, Some(1950.0))], &[]);
        let rules: Vec<_> = rows.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(rules, ["rel/vds:M1"]);
        assert_eq!(rows[0].margin, 3050);
        assert_eq!(unknown, 0);
    }

    #[test]
    fn exactly_at_the_rating_passes_and_a_hair_over_fails_by_one_mv() {
        let m = |v: f64| find(vec![dev("M1", DeviceKind::Nmos, NFET)], &[Some((v, 0.0, 0.0))], &[lim(Some(1950.0), Some(1950.0))], &[]).0;
        assert!(m(1.95).is_empty());
        let rows = m(1.9501);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].margin, 1, "margin rounds up");
    }

    #[test]
    fn the_rating_is_on_the_magnitude() {
        let rows = find(vec![dev("M1", DeviceKind::Nmos, NFET)], &[Some((-2.5, 0.0, 0.0))], &[lim(Some(1950.0), Some(1950.0))], &[]).0;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rule, "rel/vgs:M1");
        assert_eq!(rows[0].margin, 550);
    }

    /// V_BD = V_BS − V_DS: an NMOS with its drain 0.2 V below a grounded bulk
    /// forward-biases the drain junction.
    #[test]
    fn a_forward_drain_junction_is_a_violation() {
        let rows = find(vec![dev("M1", DeviceKind::Nmos, "x")], &[Some((0.5, -0.2, 0.0))], &[], &[]).0;
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].rule.as_str(), rows[0].margin), ("rel/bulk_forward:M1", 200));
    }

    #[test]
    fn the_forward_tolerance_is_a_strict_threshold() {
        let rows = |vbs: f64| find(vec![dev("M1", DeviceKind::Nmos, "x")], &[Some((0.5, 0.5, vbs))], &[], &[]).0;
        assert!(rows(FORWARD_TOL_V).is_empty());
        let r = rows(0.0015);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].margin, 2, "1.5 mV rounds up to 2");
    }

    /// NaN is not a voltage: the FET is unresolved, never silently passing.
    #[test]
    fn a_non_finite_voltage_is_unresolved() {
        for bad in [f64::NAN, f64::INFINITY] {
            let (rows, aging, unknown) = find(
                vec![dev("M1", DeviceKind::Nmos, NFET), dev("M2", DeviceKind::Nmos, NFET)],
                &[Some((bad, 0.5, 0.0)), Some((0.5, 0.5, 0.0))],
                &[lim(Some(1950.0), Some(1950.0))],
                &[(DeviceId(0), DeviceId(1))],
            );
            assert!(rows.is_empty(), "{rows:?}");
            assert_eq!(unknown, 1, "{bad}");
            assert!(aging.is_empty(), "a non-finite member has no asymmetry");
        }
    }

    #[test]
    fn pair_aging_skips_unresolved_and_out_of_range_members() {
        let devs = vec![dev("M1", DeviceKind::Nmos, NFET), dev("M2", DeviceKind::Nmos, NFET), dev("M3", DeviceKind::Nmos, NFET)];
        let v = [Some((0.8, 0.9, -0.1)), Some((0.7, 0.9, 0.0)), None];
        let pairs = [(DeviceId(0), DeviceId(1)), (DeviceId(0), DeviceId(2)), (DeviceId(0), DeviceId(9))];
        let (_, aging, _) = find(devs, &v, &[], &pairs);
        assert_eq!(aging.len(), 1);
        let a = &aging[0];
        assert_eq!((a.a, a.b), (DeviceId(0), DeviceId(1)));
        assert!((a.dvgs_mv - 100.0).abs() < 1e-9 && a.dvds_mv == 0.0 && (a.dvbs_mv - 100.0).abs() < 1e-9, "{a:?}");
    }

    #[test]
    fn the_first_matching_limit_names_the_model() {
        let l = [verify::FetLimit { model: "other".into(), vgs_max_mv: Some(1.0), vds_max_mv: Some(1.0) }, lim(Some(1950.0), Some(1950.0))];
        let (rows, _, unknown) = find(vec![dev("M1", DeviceKind::Nmos, NFET)], &[Some((1.0, 1.0, 0.0))], &l, &[]);
        assert!(rows.is_empty(), "the other model's limit must not apply: {rows:?}");
        assert_eq!(unknown, 0);
    }
}
