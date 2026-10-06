//! EXT-23: substrate aggressor and victim tags (Charbon 2001 ch.2: injection
//! → propagation → reception). What isolation ([`crate::emit::isolation`])
//! and guard rings ([`crate::rings`]) read.

use analog::intent::{Aggressor, Inject, MatchClass, MatchSpec, Victim};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::DeviceId;
use pnr_core::{DeviceKind, Netlist};

/// A net whose voltage swings hard enough to inject: Clock, DigitalSwitching, Noisy.
fn aggressor_net(c: NetClass) -> bool {
    matches!(c, NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy)
}

/// EXT-23 steps 1–3 (GAP-03 adds the minority kinds): aggressors, then
/// victims (non-aggressors), each in device-id order, one tag per device.
/// Switching: a terminal on an aggressor net. Capacitive: a terminal on a net a
/// capacitor couples to an aggressor net. Victims: members of Moderate+ sets
/// and FETs gated by a Bias/Reference net. Weight: the largest `weight` of the
/// device's sets, else the largest class rank over its Moderate+ sets
/// (Moderate 3, Exceptional 10; Philis policy), else 1 for a bias-gated FET.
/// A capacitor needs exactly two terminals to couple.
///
/// # Panics
/// When `classes` is shorter than the nets the devices touch, or a set member
/// is out of bounds of `nl.devices`.
#[must_use]
pub fn tag(nl: &Netlist, classes: &[NetClassification], sets: &[MatchSpec]) -> (Vec<Aggressor>, Vec<Victim>) {
    let class = |n: pnr_core::NetId| classes[n.0 as usize].class;
    let mut coupled = vec![false; classes.len()];
    for d in nl.devices.iter().filter(|d| d.kind == DeviceKind::Capacitor) {
        if let [(_, a), (_, b)] = d.terminals[..] {
            coupled[a.0 as usize] |= aggressor_net(class(b));
            coupled[b.0 as usize] |= aggressor_net(class(a));
        }
    }
    let mut aggressors = Vec::new();
    for (i, d) in nl.devices.iter().enumerate() {
        let device = DeviceId(i as u16);
        if d.terminals.iter().any(|t| aggressor_net(class(t.1))) {
            aggressors.push(Aggressor { device, inject: Inject::Switching, reason: "terminal on a switching net" });
        } else if d.terminals.iter().any(|t| coupled[t.1 .0 as usize]) {
            aggressors.push(Aggressor { device, inject: Inject::Capacitive, reason: "capacitor-coupled to a switching net" });
        }
    }
    let mut is_aggressor = vec![false; nl.devices.len()];
    aggressors.iter().for_each(|a| is_aggressor[a.device.0 as usize] = true);
    let member_ids: Vec<Vec<DeviceId>> = sets.iter().map(|s| s.members.iter().map(|m| m.device).collect()).collect();
    let set_idx = crate::sets::device_index(nl.devices.len(), member_ids.iter().map(Vec::as_slice));
    let victims = (0..nl.devices.len())
        .filter(|&d| !is_aggressor[d])
        .filter_map(|d| {
            let device = DeviceId(d as u16);
            let mine = || set_idx[d].iter().map(|&i| &sets[i]);
            let held: Vec<&MatchSpec> = mine().filter(|s| s.class >= MatchClass::Moderate).collect();
            if !held.is_empty() {
                let weight = mine().filter_map(|s| s.weight).reduce(f32::max);
                let rank = |s: &&MatchSpec| if s.class == MatchClass::Exceptional { 10.0 } else { 3.0 };
                let weight = weight.unwrap_or_else(|| held.iter().map(rank).fold(0.0, f32::max));
                return Some(Victim { device, weight, reason: "matched set" });
            }
            let dev = &nl.devices[d];
            let gate = dev.terminals.iter().find(|t| t.0 == "G").map(|t| class(t.1));
            (matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos) && matches!(gate, Some(NetClass::Bias | NetClass::Reference)))
                .then_some(Victim { device, weight: 1.0, reason: "bias/reference gate" })
        })
        .collect();
    (aggressors, victims)
}

/// GAP-03: devices with a diffusion within `inj_series_ohm` of a pin, plus
/// forward-biased bulks (H14-01). Pins are `nl.ports` that are not rails or
/// substrate; distance is a multi-source Dijkstra over resistors (`r_mohm`).
/// A resistor without a value is an edge of weight 0, conservative: what it
/// reaches carries the reason `"resistance unknown"`. On a reached net NMOS
/// D/S and a diode's N inject electrons, PMOS D/S and a diode's P holes;
/// bipolars stay unclassified (noted in `missing`). With an op point, an NMOS
/// with `vbs > 0` (PMOS `< 0`) gets the same tag. One tag per device, in
/// device order, the first reason winning. "Within" is strict: a net exactly
/// `inj_series_ohm` away is not reached, and `inj_series_ohm ≤ 0` reaches
/// nothing. `missing` gains each note at most once.
///
/// # Panics
/// When `classes` is shorter than a port's net id.
#[must_use]
pub fn injectors(
    nl: &Netlist,
    classes: &[NetClassification],
    op: Option<&crate::evidence::OpFacts>,
    inj_series_ohm: f64,
    missing: &mut Vec<(&'static str, &'static str)>,
) -> Vec<Aggressor> {
    use std::cmp::Reverse;
    let n_nets = nl.nets.len();
    let mut dist: Vec<Option<(u64, bool)>> = vec![None; n_nets];
    let mut heap = std::collections::BinaryHeap::new();
    let pins = nl.ports.iter().filter(|p| !matches!(classes[p.0 as usize].class, NetClass::Supply | NetClass::Ground | NetClass::Substrate));
    for &p in pins {
        heap.push(Reverse((0u64, false, p.0)));
    }
    if nl.ports.is_empty() {
        missing.push(("GuardRing", "no port list: injectors unknown"));
    }
    let mut edges: Vec<Vec<(u16, u64, bool)>> = vec![Vec::new(); n_nets];
    for d in nl.devices.iter().filter(|d| d.kind == DeviceKind::Resistor) {
        let net = |t: &str| d.terminals.iter().find(|x| x.0 == t).map(|x| x.1 .0);
        if let (Some(a), Some(b)) = (net("P"), net("N")) {
            let r = crate::param(d, "r_mohm", -1);
            let (w, unknown) = if r < 0 { (0, true) } else { (r as u64, false) };
            edges[a as usize].push((b, w, unknown));
            edges[b as usize].push((a, w, unknown));
        }
    }
    let limit = (inj_series_ohm * 1000.0) as u64;
    while let Some(Reverse((dd, unknown, n))) = heap.pop() {
        if dd >= limit || dist[n as usize].is_some() {
            continue;
        }
        dist[n as usize] = Some((dd, unknown));
        for &(m, w, u) in &edges[n as usize] {
            if dist[m as usize].is_none() {
                heap.push(Reverse((dd + w, unknown || u, m)));
            }
        }
    }
    let mut out = Vec::new();
    for (i, d) in nl.devices.iter().enumerate() {
        let device = DeviceId(i as u16);
        let reached = |ts: &[&str]| d.terminals.iter().filter(|t| ts.contains(&t.0.as_str())).find_map(|t| dist[t.1 .0 as usize]);
        let tag = match d.kind {
            DeviceKind::Nmos => reached(&["D", "S"]).map(|r| (Inject::MinorityElectron, r.1)),
            DeviceKind::Pmos => reached(&["D", "S"]).map(|r| (Inject::MinorityHole, r.1)),
            DeviceKind::Diode => reached(&["N"]).map(|r| (Inject::MinorityElectron, r.1)).or_else(|| reached(&["P"]).map(|r| (Inject::MinorityHole, r.1))),
            DeviceKind::Npn | DeviceKind::Pnp => {
                if reached(&["C", "B", "E"]).is_some() && !missing.contains(&("GuardRing", "bipolar injectors unclassified")) {
                    missing.push(("GuardRing", "bipolar injectors unclassified"));
                }
                None
            }
            _ => None,
        };
        let tag = tag.map(|(inject, unknown)| (inject, if unknown { "resistance unknown" } else { "pin diffusion" })).or_else(|| {
            let vbs = op?.dev.get(i).copied().flatten()?.vbs_mv?;
            match d.kind {
                DeviceKind::Nmos if vbs > 0.0 => Some((Inject::MinorityElectron, "forward-biased bulk")),
                DeviceKind::Pmos if vbs < 0.0 => Some((Inject::MinorityHole, "forward-biased bulk")),
                _ => None,
            }
        });
        out.extend(tag.map(|(inject, reason)| Aggressor { device, inject, reason }));
    }
    out
}

/// `victim[d]`: what REL-07's `RingInputs.victim` and REL-16's `CellFlags.sensitive` read.
///
/// # Panics
/// When a victim's device is `>= n_devices`.
#[must_use]
pub fn victim_mask(n_devices: usize, v: &[Victim]) -> Vec<bool> {
    let mut out = vec![false; n_devices];
    v.iter().for_each(|v| out[v.device.0 as usize] = true);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(nl: &Netlist, ds: impl Iterator<Item = DeviceId>) -> Vec<String> {
        let mut v: Vec<String> = ds.map(|d| nl.devices[d.0 as usize].name.clone()).collect();
        v.sort();
        v
    }

    /// C1 couples `clk` onto `x`: M1 (D on x) and M2 (G on x) are Capacitive.
    #[test]
    fn capacitor_couples_an_aggressor() {
        use crate::tests::{fet, nets};
        let cap = pnr_core::netlist::Device {
            name: "C1".into(),
            kind: DeviceKind::Capacitor,
            model: String::new(),
            terminals: vec![("P".into(), pnr_core::NetId(0)), ("N".into(), pnr_core::NetId(1))],
            params: vec![("w".into(), 2_000), ("l".into(), 2_000)],
        };
        let nl = Netlist {
            devices: vec![cap, fet("M1", DeviceKind::Nmos, 2, 1, 3, 3, 1_000, 500), fet("M2", DeviceKind::Nmos, 1, 4, 3, 3, 1_000, 500)],
            nets: nets(&["clk", "x", "g", "VSS", "y"]),
            ..Default::default()
        };
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let kind = |n: &str| p.intent.aggressors.iter().find(|a| nl.devices[a.device.0 as usize].name == n).map(|a| a.inject);
        assert_eq!(kind("C1"), Some(Inject::Switching));
        assert_eq!((kind("M1"), kind("M2")), (Some(Inject::Capacitive), Some(Inject::Capacitive)));
        assert_eq!(names(&nl, p.intent.victims.iter().map(|v| v.device)), Vec::<String>::new());
    }

    /// GAP-03 `drv`: `M1` drains on the pin `out`, `M2` through 10 kΩ, `M3`
    /// through 100 kΩ. Nets: 0=in 1=out 2=VDD 3=VSS 4=x 5=y.
    fn drv(r2_mohm: Option<i64>) -> Netlist {
        use crate::tests::{fet, nets};
        let r = |name: &str, p: u16, v: Option<i64>| pnr_core::netlist::Device {
            name: name.into(),
            kind: DeviceKind::Resistor,
            model: String::new(),
            terminals: vec![("P".into(), pnr_core::NetId(p)), ("N".into(), pnr_core::NetId(1))],
            params: v.map(|v| ("r_mohm".to_string(), v)).into_iter().collect(),
        };
        Netlist {
            devices: vec![
                fet("M1", DeviceKind::Nmos, 0, 1, 3, 3, 1_000, 500),
                fet("M2", DeviceKind::Nmos, 0, 4, 3, 3, 1_000, 500),
                fet("M3", DeviceKind::Nmos, 0, 5, 3, 3, 1_000, 500),
                r("R1", 4, Some(10_000_000)),
                r("R2", 5, r2_mohm),
            ],
            nets: nets(&["in", "out", "VDD", "VSS", "x", "y"]),
            ports: [0, 1, 2, 3].map(pnr_core::NetId).to_vec(),
            ..Default::default()
        }
    }

    fn minority(nl: &Netlist) -> (Vec<(String, Inject, &'static str)>, Vec<(&'static str, &'static str)>) {
        let p = crate::annotate(nl, &crate::AnnotationConfig::default());
        let mut missing = Vec::new();
        let tags = injectors(nl, &p.net_classes, None, crate::Policy::default().inj_series_ohm, &mut missing);
        (tags.iter().map(|a| (nl.devices[a.device.0 as usize].name.clone(), a.inject, a.reason)).collect(), missing)
    }

    #[test]
    fn pin_diffusions_are_injectors() {
        let (tags, _) = minority(&drv(Some(100_000_000)));
        assert_eq!(tags, [("M1".into(), Inject::MinorityElectron, "pin diffusion"), ("M2".into(), Inject::MinorityElectron, "pin diffusion")]);
    }

    #[test]
    fn a_pmos_on_a_pin_injects_holes() {
        let mut nl = drv(Some(100_000_000));
        nl.devices.push(crate::tests::fet("M4", DeviceKind::Pmos, 0, 1, 2, 2, 1_000, 500));
        let (tags, _) = minority(&nl);
        assert!(tags.contains(&("M4".into(), Inject::MinorityHole, "pin diffusion")), "{tags:?}");
    }

    #[test]
    fn unknown_resistance_is_conservative() {
        let (tags, _) = minority(&drv(None));
        assert!(tags.contains(&("M3".into(), Inject::MinorityElectron, "resistance unknown")), "{tags:?}");
    }

    #[test]
    fn no_ports_no_minority_tags() {
        let mut nl = drv(Some(100_000_000));
        nl.ports.clear();
        let (tags, missing) = minority(&nl);
        assert!(tags.is_empty(), "{tags:?}");
        assert!(missing.contains(&("GuardRing", "no port list: injectors unknown")));
    }

    #[test]
    fn forward_biased_bulk() {
        let mut nl = drv(Some(100_000_000));
        nl.ports.clear();
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let mut dev = vec![None; nl.devices.len()];
        dev[2] = Some(crate::evidence::DeviceOp { id_ua: 1.0, headroom_mv: 100.0, gm_us: 10.0, power_uw: 0.0, vgs_mv: None, vbs_mv: Some(200.0), vth_mv: None, gmb_us: None, gds_us: None });
        let op = crate::OpFacts { dev, net_mv: vec![None; nl.nets.len()] };
        let tags = injectors(&nl, &p.net_classes, Some(&op), 50_000.0, &mut Vec::new());
        assert_eq!(tags.len(), 1);
        assert_eq!((tags[0].device, tags[0].inject, tags[0].reason), (DeviceId(2), Inject::MinorityElectron, "forward-biased bulk"));
    }

    /// REL T7: every minority-tagged device gets an Injector ring.
    #[test]
    fn drv_injectors_get_rings() {
        let nl = drv(Some(100_000_000));
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let tagged: Vec<DeviceId> = p.intent.aggressors.iter().filter(|a| matches!(a.inject, Inject::MinorityElectron | Inject::MinorityHole)).map(|a| a.device).collect();
        assert_eq!(tagged.len(), 2);
        let rings: Vec<DeviceId> = p.constraints.guard_rings.iter().filter(|r| r.role == analog::cell::RingRole::Injector).map(|r| r.device).collect();
        assert_eq!(rings, tagged);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::evidence::{DeviceOp, OpFacts};
    use crate::tests::{fet, nets};
    use analog::intent::{ArrayStyle, ClassSource, ConstraintId, Family, MatchKind, Member, Origin};
    use pnr_core::netlist::Device;
    use pnr_core::NetId;

    fn cls(cs: &[NetClass]) -> Vec<NetClassification> {
        cs.iter().enumerate().map(|(i, &class)| NetClassification { net: NetId(i as u16), class, c_budget_af: None, max_coupling_af: None }).collect()
    }

    fn set(devs: &[u16], class: MatchClass, weight: Option<f32>) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: devs.iter().map(|&d| Member { device: DeviceId(d), parallel: 1, series: 1, half: None }).collect(),
            reference: None,
            family: Family::Mos,
            kind: MatchKind::Current,
            class,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    fn two(name: &str, kind: DeviceKind, ts: &[(&str, u16)], params: Vec<(String, i64)>) -> Device {
        Device { name: name.into(), kind, model: String::new(), terminals: ts.iter().map(|&(t, n)| (t.into(), NetId(n))).collect(), params }
    }

    #[test]
    fn empty_netlist() {
        let (a, v) = tag(&Netlist::default(), &[], &[]);
        assert!(a.is_empty() && v.is_empty());
        let mut missing = Vec::new();
        assert!(injectors(&Netlist::default(), &[], None, 50_000.0, &mut missing).is_empty());
        assert_eq!(missing, [("GuardRing", "no port list: injectors unknown")]);
        assert!(victim_mask(0, &[]).is_empty());
    }

    /// Nets 0=sig 1=bias 2=vss 3=clk. M0..M3 on signal gates, M4 bias-gated, M5 clocked.
    fn victims_nl() -> Netlist {
        use DeviceKind::Nmos as N;
        Netlist {
            devices: (0..4).map(|i| fet(&format!("M{i}"), N, 0, 0, 2, 2, 1_000, 500)).chain([fet("M4", N, 1, 0, 2, 2, 1_000, 500), fet("M5", N, 3, 0, 2, 2, 1_000, 500)]).collect(),
            nets: nets(&["sig", "bias", "vss", "clk"]),
            ..Default::default()
        }
    }

    #[test]
    fn victim_weights() {
        let nl = victims_nl();
        let c = cls(&[NetClass::Signal, NetClass::Bias, NetClass::Ground, NetClass::Clock]);
        let sets = [set(&[0, 1], MatchClass::Moderate, None), set(&[1, 2], MatchClass::Exceptional, None), set(&[3], MatchClass::Minimal, Some(9.0)), set(&[2], MatchClass::Moderate, Some(4.0)), set(&[5], MatchClass::Exceptional, None)];
        let (a, v) = tag(&nl, &c, &sets);
        assert_eq!(a.iter().map(|x| (x.device.0, x.inject)).collect::<Vec<_>>(), [(5, Inject::Switching)]);
        let got: Vec<(u16, f32, &str)> = v.iter().map(|x| (x.device.0, x.weight, x.reason)).collect();
        // M0: Moderate rank 3. M1: Exceptional 10 over Moderate 3. M2: the set weight 4 wins over a rank.
        // M3: only a Minimal set, signal gate: no victim. M4: bias gate, weight 1. M5: an aggressor, never a victim.
        assert_eq!(got, [(0, 3.0, "matched set"), (1, 10.0, "matched set"), (2, 4.0, "matched set"), (4, 1.0, "bias/reference gate")]);
        assert_eq!(victim_mask(6, &v), [true, true, true, false, true, false]);
    }

    /// A capacitor between two quiet nets couples nothing; one with three terminals is ignored.
    #[test]
    fn capacitor_coupling_needs_an_aggressor() {
        let nl = Netlist {
            devices: vec![two("C0", DeviceKind::Capacitor, &[("P", 0), ("N", 1)], vec![]), two("C1", DeviceKind::Capacitor, &[("P", 2), ("N", 1), ("B", 3)], vec![]), fet("M0", DeviceKind::Nmos, 1, 3, 3, 3, 1_000, 500)],
            nets: nets(&["a", "b", "clk", "vss"]),
            ..Default::default()
        };
        let c = cls(&[NetClass::Signal, NetClass::Signal, NetClass::Clock, NetClass::Ground]);
        let (a, _) = tag(&nl, &c, &[]);
        assert_eq!(a.iter().map(|x| (x.device.0, x.inject)).collect::<Vec<_>>(), [(1, Inject::Switching)]);
    }

    /// Nets 0=pin 1=x 2=vss 3=y; ports {pin, vss}. R0 pin–x (`r`), D0 with N on x, Q0 on pin, P0 PMOS on y.
    fn inj(r: Option<i64>) -> Netlist {
        Netlist {
            devices: vec![
                two("R0", DeviceKind::Resistor, &[("P", 0), ("N", 1)], r.map(|v| ("r_mohm".to_string(), v)).into_iter().collect()),
                two("D0", DeviceKind::Diode, &[("P", 2), ("N", 1)], vec![]),
                two("Q0", DeviceKind::Npn, &[("C", 0), ("B", 3), ("E", 2)], vec![]),
                two("Q1", DeviceKind::Pnp, &[("C", 0), ("B", 3), ("E", 2)], vec![]),
                fet("P0", DeviceKind::Pmos, 3, 3, 3, 3, 1_000, 500),
            ],
            nets: nets(&["pin", "x", "vss", "y"]),
            ports: vec![NetId(0), NetId(2)],
            ..Default::default()
        }
    }

    fn icls() -> Vec<NetClassification> {
        cls(&[NetClass::Signal, NetClass::Signal, NetClass::Ground, NetClass::Signal])
    }

    #[test]
    fn injector_reach_is_strict() {
        // 10 kΩ to x: reached under a 50 kΩ limit, not under exactly 10 kΩ, nothing under 0.
        let tags = |limit: f64| injectors(&inj(Some(10_000_000)), &icls(), None, limit, &mut Vec::new()).iter().map(|a| (a.device.0, a.inject, a.reason)).collect::<Vec<_>>();
        assert_eq!(tags(50_000.0), [(1, Inject::MinorityElectron, "pin diffusion")]);
        assert!(tags(10_000.0).is_empty());
        assert!(tags(0.0).is_empty());
        assert!(tags(-1.0).is_empty());
    }

    #[test]
    fn bipolars_noted_once() {
        let mut missing = vec![("GuardRing", "no port list: injectors unknown")];
        let mut nl = inj(None);
        let tags = injectors(&nl, &icls(), None, 50_000.0, &mut missing);
        assert_eq!(tags.iter().map(|a| (a.device.0, a.reason)).collect::<Vec<_>>(), [(1, "resistance unknown")]);
        assert_eq!(missing, [("GuardRing", "no port list: injectors unknown"), ("GuardRing", "bipolar injectors unclassified")]);
        // Called again with no ports: neither note is repeated.
        nl.ports.clear();
        injectors(&nl, &icls(), None, 50_000.0, &mut missing);
        assert_eq!(missing.len(), 2, "{missing:?}");
    }

    #[test]
    fn diode_p_side_and_pmos_forward_bias() {
        // D0 flipped: P on x injects holes.
        let mut nl = inj(Some(0));
        nl.devices[1] = two("D0", DeviceKind::Diode, &[("P", 1), ("N", 2)], vec![]);
        let dop = |vbs: f64| DeviceOp { id_ua: 1.0, headroom_mv: 100.0, gm_us: 1.0, power_uw: 0.0, vgs_mv: None, vbs_mv: Some(vbs), vth_mv: None, gmb_us: None, gds_us: None };
        let mut dev = vec![None; 5];
        dev[4] = Some(dop(-300.0));
        let op = OpFacts { dev, net_mv: vec![] };
        let tags = injectors(&nl, &icls(), Some(&op), 50_000.0, &mut Vec::new());
        let got: Vec<_> = tags.iter().map(|a| (a.device.0, a.inject, a.reason)).collect();
        assert_eq!(got, [(1, Inject::MinorityHole, "pin diffusion"), (4, Inject::MinorityHole, "forward-biased bulk")]);
        // A reverse-biased PMOS bulk is no injector.
        let op = OpFacts { dev: vec![None, None, None, None, Some(dop(300.0))], net_mv: vec![] };
        assert_eq!(injectors(&nl, &icls(), Some(&op), 50_000.0, &mut Vec::new()).len(), 1);
    }
}
