//! EXT-17: simulation evidence the annotator may read (operating point,
//! testbench sources, spec sensitivities), and the per-device region and role
//! derived from it. Every field is optional: [`Evidence::default`] is the
//! structure-only annotation [`crate::annotate`] runs.

use analog::intent::{DeviceFacts, DeviceRole, Region};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::{DeviceId, NetId};
use pnr_core::{DeviceKind, Netlist};

/// What the flow knows beyond the netlist. Ports are `Netlist.ports` (FLOW-07).
#[derive(Clone, Debug, Default)]
pub struct Evidence {
    pub op: Option<OpFacts>,
    /// Spec sensitivities; PERF-12 fills them (M4); EXT-21 allocates set allowances
    /// from `d_vt`, EXT-25 parasitic budgets from `d_c`/`d_r`/`d_cc`.
    pub sens: Option<Sensitivities>,
    /// Nets driven by PULSE/PWL/SIN testbench sources.
    pub switching_nets: Vec<NetId>,
    /// Nets held by DC testbench sources, mV.
    pub dc_sources: Vec<(NetId, f64)>,
    /// The op point came from the synthesised probe bench (not a sign-off bias).
    pub probe_bias: bool,
}

/// DC operating point, indexed by device and by net.
#[derive(Clone, Debug)]
pub struct OpFacts {
    pub dev: Vec<Option<DeviceOp>>,
    pub net_mv: Vec<Option<f64>>,
}

/// One device's bias. `headroom_mv` = `|V_DS| − |V_DSsat|` (`library::OpPoint`
/// folds vdsat into it); `vth`/`gmb`/`gds` are `None` until PERF-09 produces them.
#[derive(Clone, Copy, Debug)]
pub struct DeviceOp {
    pub id_ua: f64,
    pub headroom_mv: f64,
    pub gm_us: f64,
    pub power_uw: f64,
    pub vgs_mv: Option<f64>,
    pub vbs_mv: Option<f64>,
    pub vth_mv: Option<f64>,
    pub gmb_us: Option<f64>,
    pub gds_us: Option<f64>,
}

/// Per-spec sensitivities (Lampaert eqs. 2.2–2.7); PERF-12 produces them.
#[derive(Clone, Debug, Default)]
pub struct Sensitivities {
    pub specs: Vec<SpecSens>,
}

#[derive(Clone, Debug)]
pub struct SpecSens {
    pub metric: String,
    pub f0: f64,
    pub lo: Option<f64>,
    pub hi: Option<f64>,
    /// Metric range over the process corners; `None` = nominal only.
    pub proc: Option<(f64, f64)>,
    pub sigma_f: Option<f64>,
    /// Per aF of ground capacitance.
    pub d_c: Vec<(NetId, f64)>,
    /// Per Ω of series resistance.
    pub d_r: Vec<(NetId, f64)>,
    /// Per mV of threshold shift.
    pub d_vt: Vec<(DeviceId, f64)>,
    /// Per K; PERF-12 leaves it empty.
    pub d_t: Vec<(DeviceId, f64)>,
    /// Coupling sensitivity per aF between two nets (LAMP-39 ½·S_Cki).
    pub d_cc: Vec<(NetId, NetId, f64)>,
}

/// Operating region, first match wins: `|Id| < 1e-3·max_id` Off; `|Vgs| <
/// |Vth|` when both are known, else `gm/|Id| ≥ 20 /V`, Subthreshold (H12-08;
/// 20 /V is Philis policy, near the weak-inversion limit 1/(n·U_T)); negative
/// headroom Triode; otherwise Saturation.
#[must_use]
pub fn region(op: &DeviceOp, max_id_ua: f64) -> Region {
    let id = op.id_ua.abs();
    if id < 1e-3 * max_id_ua {
        Region::Off
    } else if match (op.vgs_mv, op.vth_mv) {
        (Some(vgs), Some(vth)) => vgs.abs() < vth.abs(),
        _ => op.gm_us / id >= 20.0,
    } {
        Region::Subthreshold
    } else if op.headroom_mv < 0.0 {
        Region::Triode
    } else {
        Region::Saturation
    }
}

/// Region and role per device. Roles, first match: Switch (Triode, digital
/// gate), Diode (D = G), Cascode (Saturation, non-rail S, Bias gate; before
/// CurrentSource, which it would otherwise always match), CurrentSource
/// (Saturation, Bias/Reference gate or a `shared_bias` member), Amplifier
/// (Saturation, Signal/Sensitive gate), Load (in a Load leaf). Without an op
/// only Diode, Load and Passive (R/C/L) are assigned.
#[must_use]
pub fn device_facts(
    nl: &Netlist,
    op: Option<&OpFacts>,
    classes: &[NetClassification],
    shared_bias: &[Vec<DeviceId>],
    load_leaf: &[bool],
) -> Vec<DeviceFacts> {
    let fet = |k: DeviceKind| matches!(k, DeviceKind::Nmos | DeviceKind::Pmos);
    let dev_op = |d: usize| op.and_then(|o| o.dev.get(d).copied().flatten());
    let max_id = (0..nl.devices.len()).filter(|&d| fet(nl.devices[d].kind)).filter_map(dev_op).map(|o| o.id_ua.abs()).fold(0.0, f64::max);
    let mut in_shared = vec![false; nl.devices.len()];
    shared_bias.iter().flatten().for_each(|d| in_shared[d.0 as usize] = true);
    nl.devices
        .iter()
        .enumerate()
        .map(|(d, dev)| {
            if !fet(dev.kind) {
                let role = if matches!(dev.kind, DeviceKind::Resistor | DeviceKind::Capacitor | DeviceKind::Inductor) {
                    DeviceRole::Passive
                } else {
                    DeviceRole::Unknown
                };
                return DeviceFacts { region: Region::Unknown, role };
            }
            let net = |t: &str| dev.terminals.iter().find(|(n, _)| n == t).map(|&(_, n)| n);
            let class = |t: &str| net(t).map(|n| classes[n.0 as usize].class);
            let gate = class("G");
            let region = dev_op(d).map_or(Region::Unknown, |o| region(&o, max_id));
            let sat = region == Region::Saturation;
            let gate_in = |cs: &[NetClass]| gate.is_some_and(|g| cs.contains(&g));
            let role = if region == Region::Triode
                && gate_in(&[NetClass::Clock, NetClass::DigitalSwitching, NetClass::DigitalStatic])
            {
                DeviceRole::Switch
            } else if net("D").is_some() && net("D") == net("G") {
                DeviceRole::Diode
            } else if sat
                && gate_in(&[NetClass::Bias])
                && class("S").is_some_and(|c| !matches!(c, NetClass::Supply | NetClass::Ground | NetClass::Substrate))
            {
                DeviceRole::Cascode
            } else if sat && (gate_in(&[NetClass::Bias, NetClass::Reference]) || in_shared[d]) {
                DeviceRole::CurrentSource
            } else if sat && gate_in(&[NetClass::Signal, NetClass::Sensitive]) {
                DeviceRole::Amplifier
            } else if load_leaf[d] {
                DeviceRole::Load
            } else {
                DeviceRole::Unknown
            };
            DeviceFacts { region, role }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn op(id_ua: f64, gm_us: f64, headroom_mv: f64) -> DeviceOp {
        DeviceOp { id_ua, headroom_mv, gm_us, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us: None }
    }

    #[test]
    fn regions_from_headroom_and_gm() {
        assert_eq!(region(&op(0.05, 1.0, 100.0), 100.0), Region::Off);
        assert_eq!(region(&op(10.0, 250.0, 100.0), 100.0), Region::Subthreshold);
        assert_eq!(region(&op(10.0, 50.0, -20.0), 100.0), Region::Triode);
        assert_eq!(region(&op(10.0, 50.0, 100.0), 100.0), Region::Saturation);
        let vt = DeviceOp { vgs_mv: Some(300.0), vth_mv: Some(400.0), ..op(10.0, 50.0, 100.0) };
        assert_eq!(region(&vt, 100.0), Region::Subthreshold);
    }

    /// Every device biased as `o`; nets unknown.
    pub(crate) fn all_ops(nl: &Netlist, o: DeviceOp) -> Evidence {
        Evidence {
            op: Some(OpFacts { dev: vec![Some(o); nl.devices.len()], net_mv: vec![None; nl.nets.len()] }),
            ..Default::default()
        }
    }

    fn sc_switches() -> Netlist {
        use crate::tests::{fet, nets};
        Netlist {
            devices: vec![fet("MS1", DeviceKind::Nmos, 1, 0, 2, 3, 1_000, 150), fet("MS2", DeviceKind::Nmos, 5, 4, 6, 3, 1_000, 150)],
            nets: nets(&["a1", "p1", "b1", "VSS", "a2", "p2", "b2"]),
            ..Default::default()
        }
    }

    #[test]
    fn switch_needs_digital_gate() {
        let nl = sc_switches();
        let ev = all_ops(&nl, op(10.0, 50.0, -20.0));
        let cfg = crate::AnnotationConfig { clock_nets: vec!["p1".into(), "p2".into()], ..Default::default() };
        let p = crate::annotate_with(&nl, &cfg, &ev);
        assert!(p.intent.devices.iter().all(|f| f.role == DeviceRole::Switch), "{:?}", p.intent.devices);
        let p = crate::annotate_with(&nl, &crate::AnnotationConfig::default(), &ev);
        assert!(p.intent.devices.iter().all(|f| f.region == Region::Triode && f.role != DeviceRole::Switch), "{:?}", p.intent.devices);
    }

    /// The folded cascode of the corpus (`tests/corpus.rs`).
    pub(crate) fn folded() -> Netlist {
        use crate::tests::{fet, nets};
        use DeviceKind::{Nmos, Pmos};
        Netlist {
            devices: vec![
                fet("M1", Nmos, 1, 0, 2, 3, 10_000, 1_000),
                fet("M2", Nmos, 5, 4, 2, 3, 10_000, 1_000),
                fet("M0", Nmos, 6, 2, 3, 3, 20_000, 1_000),
                fet("M3", Pmos, 7, 0, 8, 8, 20_000, 1_000),
                fet("M4", Pmos, 7, 4, 8, 8, 20_000, 1_000),
                fet("M5", Pmos, 10, 9, 0, 8, 10_000, 1_000),
                fet("M6", Pmos, 10, 11, 4, 8, 10_000, 1_000),
                fet("M7", Nmos, 12, 9, 13, 3, 5_000, 1_000),
                fet("M8", Nmos, 12, 11, 14, 3, 5_000, 1_000),
                fet("M9", Nmos, 9, 13, 3, 3, 5_000, 1_000),
                fet("M10", Nmos, 9, 14, 3, 3, 5_000, 1_000),
            ],
            nets: nets(&["x1", "vinp", "tail", "VSS", "x2", "vinn", "vbn", "vbp1", "VDD", "o1", "vbp2", "out", "vbn2", "y1", "y2"]),
            ..Default::default()
        }
    }

    /// EXT-17/18: a Bias-gated FET with its source off the rails is a cascode,
    /// not a current source.
    #[test]
    fn cascode_before_current_source() {
        let nl = folded();
        let p = crate::annotate_with(&nl, &crate::AnnotationConfig::default(), &all_ops(&nl, op(10.0, 50.0, 100.0)));
        let role = |n: &str| p.intent.devices[nl.devices.iter().position(|d| d.name == n).unwrap()].role;
        assert_eq!((role("M5"), role("M6")), (DeviceRole::Cascode, DeviceRole::Cascode));
        assert_eq!(role("M0"), DeviceRole::CurrentSource);
    }
}
