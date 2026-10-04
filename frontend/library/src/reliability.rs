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
    pub a: DeviceId,
    pub b: DeviceId,
    pub dvds_mv: f64,
    pub dvgs_mv: f64,
    pub dvbs_mv: f64,
}

/// Hard rows `rel/vgs:{name}` / `rel/vds:{name}` (margin = overshoot mV,
/// rounded up) for a FET over its model's rating, and `rel/bulk_forward:{name}`
/// for a bulk junction forward-biased past [`FORWARD_TOL_V`]; each `pairs`
/// entry whose members both have all three voltages as a [`PairAging`]; and
/// the count of FETs whose rating could not be checked (a voltage unresolved,
/// or no limit names its model). A limit's `None` field is not checked and not
/// unknown: the deck states no such rule. The bulk check needs no limit.
#[must_use]
pub fn voltage_findings(
    netlist: &Netlist,
    op: &OpPoint,
    limits: &[verify::FetLimit],
    pairs: &[(DeviceId, DeviceId)],
) -> (Vec<Violation>, Vec<PairAging>, usize) {
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
                        rows.push(Violation { rule: format!("rel/{tag}:{}", d.name), margin: over.ceil() as i64 });
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
            rows.push(Violation { rule: format!("rel/bulk_forward:{}", d.name), margin: ((fwd * 1e3).ceil() as i64).max(1) });
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
        voltage_findings(&netlist, &op, &[lim], pairs)
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
