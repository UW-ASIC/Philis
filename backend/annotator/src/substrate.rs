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
    let victims = (0..nl.devices.len())
        .filter(|&d| !is_aggressor[d])
        .filter_map(|d| {
            let device = DeviceId(d as u16);
            let held: Vec<&MatchSpec> = sets.iter().filter(|s| s.class >= MatchClass::Moderate && s.members.iter().any(|m| m.device == device)).collect();
            if !held.is_empty() {
                let weight = sets.iter().filter(|s| s.members.iter().any(|m| m.device == device)).filter_map(|s| s.weight).reduce(f32::max);
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

/// `victim[d]`: what REL-07's `RingInputs.victim` and REL-16's `CellFlags.sensitive` read.
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
}
