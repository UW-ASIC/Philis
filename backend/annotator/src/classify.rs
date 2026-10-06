//! Per-net class + budgets. Name gives rails/clocks ([`crate::netrole`]);
//! structure gives the two classes a name can't: **Sensitive** (feeds a matched
//! device's gate, or gates-only = a bias rail) and **Substrate** (bulk-only).

use analog::intent::{EvidenceLevel, MatchKind, MatchSpec, NetFacts, RcClass};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::NetId;
use pnr_core::BipartiteHypergraph;

use crate::class::SetRole;
use crate::evidence::Evidence;
use crate::netrole::NetRole;

/// Budgets as multiples of the capacitive load `c_load_af` the net drives:
/// `(wire C, total coupling)`, aF. A sensitive net (matched gate, bias rail)
/// may add at most its own load as wire, and again as coupling; a plain signal
/// twice that. Rails and substrate are low-impedance: unbudgeted.
///
/// ponytail: sized so minimum-size gates (rc_filter's 0.2 um2 inverter, whose
/// ring-to-ring wire alone is ~70% of its gate C) stay routable; precision
/// targets (10% wire / 2% coupling) are what a matched stage would want and
/// flag chain4/quad/rc_filter today. Rail aggressors count as coupling here.
/// Aggressor classes (Clock, DigitalSwitching, Noisy, and DigitalStatic) get a
/// wire budget but no coupling budget: an aggressor is not a victim (EXT-18).
pub(crate) fn budgets(class: NetClass, c_load_af: f32) -> (Option<i64>, Option<i64>) {
    let frac = |w: f32, k: f32| (Some((w * c_load_af) as i64), Some((k * c_load_af) as i64));
    match class {
        NetClass::Sensitive | NetClass::Bias | NetClass::Reference => frac(1.0, 1.0),
        NetClass::Signal => frac(2.0, 2.0),
        NetClass::Clock | NetClass::DigitalSwitching | NetClass::Noisy | NetClass::DigitalStatic => {
            (Some((2.0 * c_load_af) as i64), None)
        }
        NetClass::Supply | NetClass::Ground | NetClass::Substrate => (None, None),
    }
}

/// Classify every net: name-derived role, upgraded by structure.
///
/// `sensitive_devices` are the devices whose matching the layout must protect —
/// the annotator's matched pairs. A net feeding one of their gates is the
/// small-signal path whose corruption shows up directly as offset.
///
/// `gate_um2` (per device, `0` for non-FETs) and `loads` (sidecar `Load`, aF)
/// size the budgets ([`net_load_af`]): a net driving no gate and carrying no
/// stated load (a drain-only output) is unbudgeted, as is a net on a capacitor
/// plate: its limit is an array spec (settling, code-dependent error) the
/// netlist does not carry.
///
/// Returns one entry per net of `hg`, `net` = its index.
///
/// # Panics
/// When `roles` is shorter than the net count, or `sensitive_devices` or
/// `gate_um2` shorter than the device count.
#[must_use]
pub fn classify(
    hg: &BipartiteHypergraph,
    roles: &[NetRole],
    sensitive_devices: &[bool],
    gate_um2: &[f32],
    gate_af_per_um2: Option<f32>,
    loads: &[(NetId, f32)],
) -> Vec<NetClassification> {
    let n_nets = hg.net_names.len();
    let mut touches_gate = vec![false; n_nets];
    let mut touches_channel = vec![false; n_nets];
    let mut touches_bulk = vec![false; n_nets];
    let mut gate_of_sensitive = vec![false; n_nets];

    for (d, nets) in hg.device_nets.iter().enumerate() {
        for (t, n) in hg.terminals[d].iter().zip(nets) {
            let i = n.0 as usize;
            match crate::terms::term_role(hg.kinds[d], t) {
                crate::terms::TermRole::FetGate => {
                    touches_gate[i] = true;
                    gate_of_sensitive[i] |= sensitive_devices[d];
                }
                crate::terms::TermRole::Channel
                | crate::terms::TermRole::BjtBase
                | crate::terms::TermRole::Passive => touches_channel[i] = true,
                crate::terms::TermRole::Body => touches_bulk[i] = true,
                crate::terms::TermRole::Plate => {}
            }
        }
    }

    let load_af = net_load_af(hg, gate_um2, gate_af_per_um2, loads);
    (0..n_nets)
        .map(|i| {
            let class = classify_one(
                roles[i],
                gate_of_sensitive[i],
                touches_gate[i],
                touches_channel[i],
                touches_bulk[i],
            );
            let (c_budget_af, max_coupling_af) = load_af[i].map_or((None, None), |c| budgets(class, c));
            NetClassification { net: NetId(i as u16), class, c_budget_af, max_coupling_af }
        })
        .collect()
}

/// Capacitive load per net, aF: the gate area it drives × `gate_af_per_um2`
/// plus its sidecar `Load` (AA-25). `None` for a net with neither (a
/// drain-only output: its load is off-netlist and is never invented), for a
/// net driving gates without a gate-cap number, and on a capacitor plate: a
/// plate net's parasitics trace to array specs (ARR-03 code-dependent error,
/// ARR-05 settling), not a gate load. A `loads` entry naming a net outside
/// `hg` is ignored; several entries on one net add.
pub(crate) fn net_load_af(hg: &BipartiteHypergraph, gate_um2: &[f32], gate_af_per_um2: Option<f32>, loads: &[(NetId, f32)]) -> Vec<Option<f32>> {
    let n_nets = hg.net_names.len();
    let mut load_um2 = vec![0.0f32; n_nets];
    let mut on_plate = vec![false; n_nets];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        for (t, n) in hg.terminals[d].iter().zip(nets) {
            match crate::terms::term_role(hg.kinds[d], t) {
                crate::terms::TermRole::FetGate => load_um2[n.0 as usize] += gate_um2[d],
                crate::terms::TermRole::Plate => on_plate[n.0 as usize] = true,
                _ => {}
            }
        }
    }
    let mut ext: Vec<Option<f32>> = vec![None; n_nets];
    for &(n, af) in loads {
        if let Some(e) = ext.get_mut(n.0 as usize) {
            *e = Some(e.unwrap_or(0.0) + af);
        }
    }
    (0..n_nets)
        .map(|i| {
            let gate = if load_um2[i] > 0.0 { Some(gate_af_per_um2? * load_um2[i]) } else { None };
            let load = match (gate, ext[i]) {
                (Some(g), e) => Some(g + e.unwrap_or(0.0)),
                (None, e) => e,
            };
            load.filter(|_| !on_plate[i])
        })
        .collect()
}

/// What [`refine`] reads: everything that exists only once sets are built.
pub struct RefineCtx<'a> {
    /// The device–net graph the classes index.
    pub hg: &'a BipartiteHypergraph,
    /// Every pattern match, overlapping (`recognize_all`).
    pub matches: &'a [crate::pattern::PatternMatch],
    /// Final matched sets (EXT-15/16).
    pub sets: &'a [MatchSpec],
    /// Role per `sets` entry (`MatchSpec` does not carry it).
    pub set_roles: &'a [SetRole],
    /// Passive, bipolar-core and diode sets (EXT-19).
    pub passive: &'a [crate::passive::PassiveSet],
    /// Per net: named in the caller's config (testbench-derived names excluded).
    pub user: &'a [bool],
    /// Simulation evidence: op point (impedance, DC level) and testbench nets.
    pub ev: &'a Evidence,
    /// Sidecar `NetClass` overrides (EXT-26): first, over every rule, `User` evidence.
    pub user_classes: &'a [(NetId, NetClass)],
    /// Top-level port nets.
    pub ports: &'a [NetId],
    /// Per net capacitive load, aF ([`net_load_af`]).
    pub load_af: &'a [Option<f32>],
}

/// Gate-level logic templates whose `D` follows a switching `G` (EXT-18 step 1).
const LOGIC: [&str; 3] = ["cmos_inverter", "nand_gate", "nor_gate"];

/// [`LOGIC`] plus the transmission gate: templates whose G/D (and a tgate's S)
/// nets carry logic levels (EXT-18 step 3).
const LOGIC_LEVEL: [&str; 4] = ["cmos_inverter", "nand_gate", "nor_gate", "transmission_gate"];

/// A Signal net at or above this impedance is Sensitive (EXT-18; Philis policy).
const HIGH_Z_OHM: f32 = 100_000.0;

/// Nets touched only by FET gates and diode-connected drains: the structural Bias rule of
/// [`refine`] step 7 (rails not excluded), readable before `refine` runs (EXT-27 arrays).
#[must_use]
pub fn bias_lines(hg: &BipartiteHypergraph) -> Vec<bool> {
    let pin = |d: u32, t: &str| crate::pattern::pin_net(hg, d, t);
    let mut only_gates: Vec<Option<bool>> = vec![None; hg.net_names.len()];
    for (d, nets) in hg.device_nets.iter().enumerate() {
        let fet = crate::pattern::fet(hg.kinds[d]);
        let diode = fet && pin(d as u32, "D") == pin(d as u32, "G");
        for (t, n) in hg.terminals[d].iter().zip(nets) {
            let ok = fet && (t == "G" || (t == "D" && diode));
            let g = only_gates[n.0 as usize].get_or_insert(true);
            *g &= ok;
        }
    }
    only_gates.into_iter().map(|g| g == Some(true)).collect()
}

/// EXT-18 step 3: refine the pre-pass classes once sets exist, recompute the
/// budgets, return per-net facts. Rails, Substrate and Clock keep their class.
/// Every other net takes the first rule that applies: DigitalSwitching (a logic
/// match's D when one of its G nets is Clock or DigitalSwitching, to a
/// fixpoint); Sensitive (a Voltage set's gate, ahead of the digital rules so a
/// latch's regenerative nodes stay analog); DigitalStatic (logic G/D, tgate
/// S); Noisy (charge-pump output, `_n` with no `_p` twin: H15-06, where the
/// name cannot be a differential half); Reference (a bandgap core's shared
/// base/gate, a cascoded reference's output, a DAC reference plate); Sensitive
/// (a DAC bank's shared plate, `_s`, or a Signal net of `z ≥ 100 kΩ`); Bias
/// (gates and diode D=G only); else the pre-pass class, so nothing Sensitive
/// is silently demoted. Sidecar overrides (`user_classes`) precede every rule.
///
/// Rewrites `classes` in place (class and budgets) and returns one [`NetFacts`]
/// per net, both indexed by net id.
///
/// # Panics
/// When `classes`, `cx.user` or `cx.load_af` is shorter than the net count.
pub fn refine(classes: &mut [NetClassification], cx: &RefineCtx) -> Vec<NetFacts> {
    use EvidenceLevel as E;
    use NetClass as C;
    let hg = cx.hg;
    let n_nets = hg.net_names.len();
    let pre: Vec<NetClass> = classes.iter().map(|c| c.class).collect();
    let rail = |n: NetId| matches!(pre[n.0 as usize], C::Supply | C::Ground | C::Substrate);
    let fixed = |n: NetId| rail(n) || pre[n.0 as usize] == C::Clock;
    let pin = |d: u32, t: &str| crate::pattern::pin_net(hg, d, t);
    let lower: Vec<String> = hg.net_names.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut got: Vec<Option<(NetClass, EvidenceLevel)>> = vec![None; n_nets];
    let put = |got: &mut Vec<Option<(NetClass, EvidenceLevel)>>, n: Option<NetId>, c: NetClass, e: EvidenceLevel| {
        if let Some(n) = n.filter(|&n| !fixed(n)) {
            got[n.0 as usize].get_or_insert((c, e));
        }
    };
    for &(n, c) in cx.user_classes {
        got[n.0 as usize] = Some((c, E::User));
    }
    let logic = |ts: &[&str]| cx.matches.iter().filter(move |m| ts.contains(&m.template)).collect::<Vec<_>>();
    let nets_of = |m: &crate::pattern::PatternMatch, t: &str| m.instances.iter().filter_map(|&d| pin(d, t)).collect::<Vec<_>>();

    // 1. DigitalSwitching, to a fixpoint (at most #nets rounds: each adds a net).
    let gates = logic(&LOGIC);
    for _ in 0..=n_nets {
        let before = got.iter().flatten().count();
        for m in &gates {
            let switching = |n: &NetId| pre[n.0 as usize] == C::Clock || got[n.0 as usize].is_some_and(|g| g.0 == C::DigitalSwitching);
            if nets_of(m, "G").iter().any(switching) {
                nets_of(m, "D").into_iter().for_each(|n| put(&mut got, Some(n), C::DigitalSwitching, E::Structure));
            }
        }
        if got.iter().flatten().count() == before {
            break;
        }
    }
    // 2. Voltage-set gates.
    for s in cx.sets.iter().filter(|s| s.kind == MatchKind::Voltage) {
        s.members.iter().for_each(|m| put(&mut got, pin(u32::from(m.device.0), "G"), C::Sensitive, E::Structure));
    }
    // 3. Logic levels.
    for m in logic(&LOGIC_LEVEL) {
        let mut ns = [nets_of(m, "G"), nets_of(m, "D")].concat();
        if m.template == "transmission_gate" {
            ns.extend(nets_of(m, "S"));
        }
        ns.into_iter().for_each(|n| put(&mut got, Some(n), C::DigitalStatic, E::Structure));
    }
    // 4. Noisy.
    for m in cx.matches.iter().filter(|m| m.template == "charge_pump_cell") {
        put(&mut got, pin(m.instances[0], "D"), C::Noisy, E::Structure);
    }
    for n in 0..n_nets {
        if let Some(stem) = lower[n].strip_suffix("_n") {
            if !lower.contains(&format!("{stem}_p")) {
                put(&mut got, Some(NetId(n as u16)), C::Noisy, E::Name);
            }
        }
    }
    // 5. Reference. Philis policy: a bandgap core's reference node is its shared base (gate).
    let shared = |devs: &mut dyn Iterator<Item = u32>, t: &str| {
        let ns: Vec<Option<NetId>> = devs.map(|d| pin(d, t)).collect();
        ns.first().copied().flatten().filter(|_| ns.windows(2).all(|w| w[0] == w[1]))
    };
    let set_devs = |s: &MatchSpec| s.members.iter().map(|m| u32::from(m.device.0)).collect::<Vec<_>>();
    let cores = cx.sets.iter().zip(cx.set_roles).filter(|(_, &r)| r == SetRole::BandgapCore).map(|(s, _)| set_devs(s));
    let cores: Vec<Vec<u32>> = cores
        .chain(cx.passive.iter().filter(|p| p.role == SetRole::BandgapCore).map(|p| p.devices.iter().map(|d| u32::from(d.0)).collect()))
        .collect();
    for devs in &cores {
        for t in ["G", "B"] {
            put(&mut got, shared(&mut devs.iter().copied(), t), C::Reference, E::Structure);
        }
    }
    for m in cx.matches.iter().filter(|m| m.template == "cascoded_reference") {
        put(&mut got, pin(m.instances[1], "D"), C::Reference, E::Structure);
    }
    let banks: Vec<&MatchSpec> = cx.sets.iter().zip(cx.set_roles).filter(|(_, &r)| r == SetRole::DacBank).map(|(s, _)| s).collect();
    for s in &banks {
        put(&mut got, s.reference.and_then(|r| pin(u32::from(s.members[r].device.0), "N")), C::Reference, E::Structure);
    }
    // 6. Sensitive: a DAC bank's shared plate, `_s`, high impedance.
    for s in &banks {
        let devs = set_devs(s);
        let plate = hg.device_nets[devs[0] as usize].iter().copied().find(|n| devs.iter().all(|&d| hg.device_nets[d as usize].contains(n)));
        put(&mut got, plate, C::Sensitive, E::Structure);
    }
    let z_ohm = impedance(hg, cx.ev.op.as_ref());
    for n in 0..n_nets {
        if lower[n].ends_with("_s") {
            put(&mut got, Some(NetId(n as u16)), C::Sensitive, E::Name);
        } else if pre[n] == C::Signal && got[n].is_none() && z_ohm[n].is_some_and(|z| z >= HIGH_Z_OHM) {
            put(&mut got, Some(NetId(n as u16)), C::Sensitive, E::OpPoint);
        }
    }
    // 7. Bias: gates and diode-connected drains only.
    for (n, _) in bias_lines(hg).iter().enumerate().filter(|(_, &b)| b) {
        put(&mut got, Some(NetId(n as u16)), C::Bias, E::Structure);
    }

    let tb = |n: NetId| cx.ev.switching_nets.contains(&n) || cx.ev.dc_sources.iter().any(|s| s.0 == n);
    (0..n_nets)
        .map(|i| {
            let n = NetId(i as u16);
            let (class, evidence) = got[i].unwrap_or_else(|| {
                let e = if cx.user[i] {
                    E::User
                } else if fixed(n) && tb(n) {
                    E::Testbench
                } else if crate::netrole::rail_of(&hg.net_names[i]).is_some() || (pre[i] == C::Clock && crate::netrole::is_clock(&lower[i])) {
                    E::Name
                } else if pre[i] == C::Signal {
                    E::Default
                } else {
                    E::Structure
                };
                (pre[i], e)
            });
            set_class(&mut classes[i], class, cx.load_af[i]);
            NetFacts {
                evidence,
                port: cx.ports.contains(&n),
                dc_mv: cx.ev.op.as_ref().and_then(|o| o.net_mv.get(i).copied().flatten()).map(|v| (v as i32, v as i32)),
                rc: RcClass::Unknown,
                z_ohm: z_ohm[i],
                shield_ref: None,
            }
        })
        .collect()
}

/// Set `c`'s class and recompute its budgets from the net's load, aF.
pub(crate) fn set_class(c: &mut NetClassification, class: NetClass, load_af: Option<f32>) {
    c.class = class;
    (c.c_budget_af, c.max_coupling_af) = load_af.map_or((None, None), |l| budgets(class, l));
}

/// Small-signal impedance to AC ground per net, Ω: `1/(Σ gds of FETs with D on
/// the net + Σ gm of diode FETs on it)`, known only when every such gds is.
/// All `None` without an op point; `None` for a net no FET drain touches.
fn impedance(hg: &BipartiteHypergraph, op: Option<&crate::evidence::OpFacts>) -> Vec<Option<f32>> {
    let Some(op) = op else { return vec![None; hg.net_names.len()] };
    let mut g: Vec<Option<f64>> = vec![None; hg.net_names.len()];
    let mut unknown = vec![false; hg.net_names.len()];
    for d in 0..hg.device_nets.len() {
        if !crate::pattern::fet(hg.kinds[d]) {
            continue;
        }
        let Some(dn) = crate::pattern::pin_net(hg, d as u32, "D") else { continue };
        let o = op.dev.get(d).copied().flatten();
        let diode = crate::pattern::pin_net(hg, d as u32, "G") == Some(dn);
        match o.and_then(|o| o.gds_us) {
            Some(gds) => *g[dn.0 as usize].get_or_insert(0.0) += gds + if diode { o.map_or(0.0, |o| o.gm_us) } else { 0.0 },
            None => unknown[dn.0 as usize] = true,
        }
    }
    g.iter().zip(&unknown).map(|(g, &u)| g.filter(|&g| !u && g > 0.0).map(|g| (1e6 / g) as f32)).collect()
}

/// The classification rule itself, isolated so it is testable without a graph.
fn classify_one(
    role: NetRole,
    gate_of_sensitive: bool,
    touches_gate: bool,
    touches_channel: bool,
    touches_bulk: bool,
) -> NetClass {
    match role {
        // A rail's name is authoritative: nothing structural should demote it.
        NetRole::Supply => NetClass::Supply,
        NetRole::Ground => NetClass::Ground,
        NetRole::Clock => NetClass::Clock,
        NetRole::Signal => {
            if touches_bulk && !touches_gate && !touches_channel {
                // Only ever a body connection — substrate, not signal.
                NetClass::Substrate
            } else if gate_of_sensitive || (touches_gate && !touches_channel) {
                // Either it drives a matched device's gate, or it is a pure
                // high-impedance reference (gates only, no DC path) — a bias rail.
                // Both are the small-signal nets coupling budgets protect.
                NetClass::Sensitive
            } else {
                NetClass::Signal
            }
        }
    }
}

/// Count of each class present, for reporting, in order of first appearance;
/// absent classes are left out.
#[must_use]
pub fn census(classes: &[NetClassification]) -> Vec<(NetClass, usize)> {
    let mut out: Vec<(NetClass, usize)> = Vec::new();
    for c in classes {
        match out.iter_mut().find(|(k, _)| *k == c.class) {
            Some((_, n)) => *n += 1,
            None => out.push((c.class, 1)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rails_keep_their_named_role() {
        assert_eq!(classify_one(NetRole::Supply, false, false, true, false), NetClass::Supply);
        assert_eq!(classify_one(NetRole::Ground, false, false, true, false), NetClass::Ground);
        assert_eq!(classify_one(NetRole::Clock, false, true, false, false), NetClass::Clock);
    }

    #[test]
    fn a_gate_only_net_is_a_sensitive_reference() {
        // No DC path — gates only. This is a bias rail, the classic victim.
        assert_eq!(
            classify_one(NetRole::Signal, false, true, false, false),
            NetClass::Sensitive
        );
    }

    #[test]
    fn driving_a_matched_gate_makes_a_net_sensitive_even_with_a_dc_path() {
        // A differential input is driven *and* feeds a matched gate: still the
        // small-signal path whose coupling shows up as offset.
        assert_eq!(
            classify_one(NetRole::Signal, true, true, true, false),
            NetClass::Sensitive
        );
        // The same net without the matched-device connection is ordinary signal.
        assert_eq!(
            classify_one(NetRole::Signal, false, true, true, false),
            NetClass::Signal
        );
    }

    #[test]
    fn bulk_only_nets_are_substrate() {
        assert_eq!(
            classify_one(NetRole::Signal, false, false, false, true),
            NetClass::Substrate
        );
    }

    #[test]
    fn budgets_scale_with_the_load_and_tighten_on_sensitive_nets() {
        let (c_s, k_s) = budgets(NetClass::Sensitive, 100_000.0);
        let (c_g, k_g) = budgets(NetClass::Signal, 100_000.0);
        assert!(k_s < k_g && c_s < c_g, "a reference tolerates less than a signal");
        // 100 fF of gate: a sensitive net gets 100 fF of wire and of coupling.
        assert_eq!((c_s, k_s), (Some(100_000), Some(100_000)));
        assert!(budgets(NetClass::Sensitive, 1_000_000.0).1 > k_s, "a bigger load tolerates more");
        assert_eq!(budgets(NetClass::Supply, 100_000.0), (None, None), "rails are unbudgeted");
    }

    fn classify_netlist(nl: &pnr_core::netlist::Netlist) -> Vec<NetClassification> {
        let hg = pnr_core::BipartiteHypergraph::from_netlist(nl);
        let roles = crate::netrole::classify_nets(&hg, &crate::netrole::AnnotationConfig::default());
        let gates: Vec<f32> = nl.devices.iter().map(crate::gate_um2).collect();
        let sensitive = vec![false; nl.devices.len()];
        classify(&hg, &roles, &sensitive, &gates, None, &[])
    }

    fn class_of(nl: &pnr_core::netlist::Netlist, classes: &[NetClassification], name: &str) -> NetClass {
        let i = nl.nets.iter().position(|n| n.name == name).unwrap();
        classes[i].class
    }

    #[test]
    fn bjt_terminals_are_not_gates() {
        use pnr_core::ids::NetId;
        use pnr_core::netlist::{Device, DeviceKind, Netlist};

        let nl = Netlist {
            devices: vec![
                Device {
                    name: "XQ1".into(),
                    kind: DeviceKind::Npn,
                    model: String::new(),
                    terminals: vec![
                        ("C".into(), NetId(0)),
                        ("B".into(), NetId(1)),
                        ("E".into(), NetId(2)),
                    ],
                    params: vec![],
                },
                Device {
                    name: "XQ2".into(),
                    kind: DeviceKind::Pnp,
                    model: String::new(),
                    terminals: vec![
                        ("C".into(), NetId(3)),
                        ("B".into(), NetId(1)),
                        ("E".into(), NetId(4)),
                    ],
                    params: vec![],
                },
            ],
            nets: crate::tests::nets(&["outn", "in", "VSS", "outp", "VDD"]),
            ..Default::default()
        };
        let classes = classify_netlist(&nl);
        assert_eq!(class_of(&nl, &classes, "outn"), NetClass::Signal);
        assert_eq!(class_of(&nl, &classes, "outp"), NetClass::Signal);
        assert_eq!(class_of(&nl, &classes, "in"), NetClass::Signal);
    }

    #[test]
    fn resistor_ends_are_channels() {
        use pnr_core::ids::NetId;
        use pnr_core::netlist::{Device, DeviceKind, Netlist};

        let r1 = Device {
            name: "R1".into(),
            kind: DeviceKind::Resistor,
            model: String::new(),
            terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(1))],
            params: vec![],
        };
        let m1 = crate::tests::fet("M1", DeviceKind::Nmos, 0, 2, 3, 3, 1_000, 1_000);
        let nl = Netlist {
            devices: vec![r1, m1],
            nets: crate::tests::nets(&["a", "b", "dn", "VSS"]),
            ..Default::default()
        };
        let classes = classify_netlist(&nl);
        assert_eq!(class_of(&nl, &classes, "a"), NetClass::Signal);

        // Control: without R1, `a` has no DC path off the gate and is Sensitive.
        let nl_no_r = Netlist { devices: vec![nl.devices[1].clone()], nets: nl.nets.clone(), ..Default::default() };
        let classes_no_r = classify_netlist(&nl_no_r);
        assert_eq!(class_of(&nl_no_r, &classes_no_r, "a"), NetClass::Sensitive);
    }

    fn named(nl: &pnr_core::netlist::Netlist, p: &crate::Problem, name: &str) -> (NetClass, EvidenceLevel) {
        let i = nl.nets.iter().position(|n| n.name == name).unwrap();
        (p.net_classes[i].class, p.intent.nets[i].evidence)
    }

    #[test]
    fn aggressors_get_no_coupling_budget() {
        let c = 50_000.0;
        assert_eq!(super::budgets(NetClass::Clock, c).1, None, "an aggressor has no coupling budget");
        assert_eq!(super::budgets(NetClass::Bias, c), super::budgets(NetClass::Sensitive, c));
    }

    /// EXT-18: configured clocks keep their class with `User` evidence, and
    /// gate the triode switches.
    #[test]
    fn sc_switches_clocks_are_user_evidence() {
        use crate::evidence::DeviceOp;
        use pnr_core::netlist::{DeviceKind, Netlist};
        let nl = Netlist {
            devices: vec![
                crate::tests::fet("MS1", DeviceKind::Nmos, 1, 0, 2, 3, 1_000, 150),
                crate::tests::fet("MS2", DeviceKind::Nmos, 5, 4, 6, 3, 1_000, 150),
            ],
            nets: crate::tests::nets(&["a1", "p1", "b1", "VSS", "a2", "p2", "b2"]),
            ..Default::default()
        };
        let op = DeviceOp { id_ua: 10.0, headroom_mv: -20.0, gm_us: 50.0, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us: None };
        let ev = crate::Evidence {
            op: Some(crate::OpFacts { dev: vec![Some(op); 2], net_mv: vec![None; 7] }),
            ..Default::default()
        };
        let cfg = crate::AnnotationConfig { clock_nets: vec!["p1".into(), "p2".into()], ..Default::default() };
        let p = crate::annotate_with(&nl, &cfg, &ev);
        for n in ["p1", "p2"] {
            assert_eq!(named(&nl, &p, n), (NetClass::Clock, EvidenceLevel::User), "{n}");
        }
        assert!(p.intent.devices.iter().all(|f| f.role == analog::intent::DeviceRole::Switch));
    }

    #[test]
    fn noisy_suffix_needs_no_p_twin() {
        use pnr_core::netlist::{DeviceKind, Netlist};
        let alone = Netlist {
            devices: vec![crate::tests::fet("M1", DeviceKind::Nmos, 1, 0, 2, 2, 1_000, 500)],
            nets: crate::tests::nets(&["x_n", "g", "VSS"]),
            ..Default::default()
        };
        let p = crate::annotate(&alone, &crate::AnnotationConfig::default());
        assert_eq!(named(&alone, &p, "x_n"), (NetClass::Noisy, EvidenceLevel::Name));
        let twin = Netlist {
            devices: vec![
                crate::tests::fet("M1", DeviceKind::Nmos, 1, 0, 2, 2, 1_000, 500),
                crate::tests::fet("M2", DeviceKind::Nmos, 4, 3, 2, 2, 2_000, 500),
            ],
            nets: crate::tests::nets(&["x_n", "g", "VSS", "x_p", "h"]),
            ..Default::default()
        };
        let p = crate::annotate(&twin, &crate::AnnotationConfig::default());
        assert_ne!(named(&twin, &p, "x_n").0, NetClass::Noisy, "a differential half");
    }

    /// EXT-18 `z_ohm`: inert in the flow (PERF-09 has no gds), so tested on
    /// synthetic evidence. 5 µS is 200 kΩ (Sensitive); 20 µS is 50 kΩ (Signal).
    #[test]
    fn z_ohm_marks_high_impedance_sensitive() {
        use crate::evidence::DeviceOp;
        use pnr_core::netlist::{DeviceKind, Netlist};
        let nl = Netlist {
            devices: vec![crate::tests::fet("M1", DeviceKind::Nmos, 1, 0, 2, 2, 1_000, 500)],
            nets: crate::tests::nets(&["o", "g", "VSS"]),
            ..Default::default()
        };
        let with_gds = |gds: f64| {
            let op = DeviceOp { id_ua: 10.0, headroom_mv: 100.0, gm_us: 50.0, power_uw: 0.0, vgs_mv: None, vbs_mv: None, vth_mv: None, gmb_us: None, gds_us: Some(gds) };
            let ev = crate::Evidence { op: Some(crate::OpFacts { dev: vec![Some(op)], net_mv: vec![None; 3] }), ..Default::default() };
            crate::annotate_with(&nl, &crate::AnnotationConfig::default(), &ev)
        };
        let p = with_gds(5.0);
        assert_eq!(p.intent.nets[0].z_ohm, Some(200_000.0));
        assert_eq!(named(&nl, &p, "o"), (NetClass::Sensitive, EvidenceLevel::OpPoint));
        let p = with_gds(20.0);
        assert_eq!(named(&nl, &p, "o").0, NetClass::Signal);
        assert_eq!(crate::annotate(&nl, &crate::AnnotationConfig::default()).intent.nets[0].z_ohm, None, "no op, no impedance");
    }
}
