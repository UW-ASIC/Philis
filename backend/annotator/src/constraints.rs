//! Cell-tier [`analog::Constraints`], read by `cells` when drawing devices.
//!
//! One **Unitization** per matched set (EXT-15) with one finger W and L over
//! its members (deck-independent: the inferred unit is not drawn): members, ratio reduced by the gcd, dummies from the
//! class (MOS, R and C only, card D-g). Drawn in schematic fingers (`dev_nf` =
//! `nf·m`, `unit_w` = W_f), not the set's inferred unit (`MatchSpec::unit`,
//! `Member::parallel`/`series`): the LVS reference expands `nf·m` fingers of
//! W_f, so a set drawn in `W_u ≠ W_f` units is `lvs.unpaired_device` (measured
//! on `mirror_ratio` and `ota`). Adopting the unit is a CELL/FLOW item. Devices no such set
//! covers keep today's rules (card D-f), below, and then identical parallel MOS
//! (all terminals equal, one size and model) still uncovered share one
//! Unitization with neither dummies nor route matching (cellgen's rule).
//!
//! For every device of every recognised block:
//! - **Unitization**, one per (kind, model, bulk, finger W, L) class of the block — the only channel
//!   by which W/L/nf reach `cells`. One class per unitization: `unit_w`/`unit_l`
//!   are group scalars, and a mixed group draws members at the wrong size (LVS
//!   `parameter_mismatch`). Model and bulk split classes too: two flavours or two
//!   wells are never one drawn unit (AA-20).
//!
//! Guard rings are not emitted here: [`crate::rings::plan`] places them by role (REL-07).

use analog::cell::{SeriesParallel, Unitization};
use analog::intent::{MatchClass, MatchKind, MatchSpec, Origin};
use analog::Constraints;
use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;
use pnr_core::Netlist;

use crate::block::{Block, BlockKind};
use crate::size::Drawn;

/// Drawn fingers `nf·m` ([`pnr_core::MosSize::fingers`]; cells and the LVS
/// reference both expand that many unit devices); `1` without a MOS size.
pub(crate) fn fingers(dev: &pnr_core::netlist::Device) -> u16 {
    dev.mos_size().map_or(1, |s| s.fingers().min(u32::from(u16::MAX)) as u16)
}

/// How a block's units of `kind` combine: a resistor or capacitor drawn
/// outside a matched set is a series string of units, anything else parallel.
fn series_parallel(kind: DeviceKind) -> SeriesParallel {
    match kind {
        DeviceKind::Resistor | DeviceKind::Capacitor => SeriesParallel::Series,
        _ => SeriesParallel::Parallel,
    }
}

/// Dummies by class (Hastings MOS rule 12: minimal need none, moderate and
/// exceptional do), MOS, R and C only: bipolar sets keep none (card D-g).
pub(crate) fn dummy_required(kind: DeviceKind, class: MatchClass) -> bool {
    !matches!(kind, DeviceKind::Npn | DeviceKind::Pnp) && class >= MatchClass::Moderate
}

/// The member of `s` whose `P` and `N` nets are both `P` nets of other members
/// (a split DAC's bridge), if exactly one is.
fn split_bridge(netlist: &Netlist, s: &MatchSpec) -> Option<DeviceId> {
    let net = |d: DeviceId, t: &str| netlist.devices[d.0 as usize].terminals.iter().find(|(n, _)| n == t).map(|x| x.1);
    let tops = |d: DeviceId| s.members.iter().filter(|m| m.device != d).filter_map(|m| net(m.device, "P")).collect::<Vec<_>>();
    let mut it = s.members.iter().map(|m| m.device).filter(|&d| {
        let t = tops(d);
        [net(d, "P"), net(d, "N")].iter().all(|n| n.is_some_and(|n| t.contains(&n)))
    });
    let b = it.next();
    it.next().is_none().then_some(b).flatten()
}

/// A Unitization outside any matched set (no class, kind, style or series):
/// one target ratio per device equal to its drawn fingers ([`fingers`]).
/// `matched` sets both `dummy_required` and `route_matching_required`.
fn plain_unit(netlist: &Netlist, devices: Vec<DeviceId>, kind: DeviceKind, unit_w: i32, unit_l: i32, series_parallel: SeriesParallel, matched: bool) -> Unitization {
    let dev_nf: Vec<u16> = devices.iter().map(|&d| fingers(&netlist.devices[d.0 as usize])).collect();
    Unitization {
        devices,
        device_type: kind,
        target_ratio: dev_nf.clone(),
        dev_nf,
        unit_w,
        unit_l,
        series_parallel,
        dummy_required: matched,
        route_matching_required: matched,
        class: None,
        kind: None,
        series: Vec::new(),
        style: None,
    }
}

/// The cell-tier constraints of the module doc, in three passes: one
/// Unitization per matched set in `sets` whose banks share one finger W and L
/// (a set that does not is skipped and falls through), then per non-glue
/// block one per drawn class of its still-uncovered devices, then one per
/// group of identical parallel MOS left over. Every device lands in at most
/// one Unitization. Sizes are clamped to `0..=i32::MAX` nm, an unknown size
/// drawing as 0.
///
/// `drawn` is indexed by device id and must cover every device of
/// `netlist`; panics otherwise, or when a set or block names a device past it.
#[must_use]
pub fn assemble(netlist: &Netlist, drawn: &[Drawn], blocks: &[Block], sets: &[MatchSpec]) -> Constraints {
    let mut c = Constraints::default();
    let mut covered = vec![false; netlist.devices.len()];
    // An unknown size draws as 0 (and is reported missing by `annotate`).
    let clamp = |v: Option<i64>| v.unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32;
    for s in sets {
        // Drawn in schematic fingers (see the module doc): one common finger W and L.
        // A split DAC's bridge is drawn off the unit (`CapArray::split`), so only the
        // banks must share it; the bridge is the member whose `P` and `N` are both
        // other members' `P` (CELL-21).
        let bridge = if matches!(s.origin, Origin::PassiveSet { rule: "split_dac" }) { split_bridge(netlist, s) } else { None };
        let banks: Vec<DeviceId> = s.members.iter().map(|m| m.device).filter(|&d| Some(d) != bridge).collect();
        let d0 = drawn[banks[0].0 as usize];
        let same = banks.iter().all(|d| (drawn[d.0 as usize].w_finger_nm, drawn[d.0 as usize].l_nm) == (d0.w_finger_nm, d0.l_nm));
        if !same {
            continue;
        }
        let kind = netlist.devices[s.members[0].device.0 as usize].kind;
        let nf = |d: DeviceId| drawn[d.0 as usize].fingers.min(u32::from(u16::MAX)) as u16;
        // A bank or bipolar array lists its members as cellgen did: by count, the
        // reference (a terminating unit) first among equals, then by id.
        let mut members: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if !matches!(kind, DeviceKind::Nmos | DeviceKind::Pmos) {
            let r = s.reference.map(|i| s.members[i].device);
            members.sort_by_key(|&d| (nf(d), Some(d) != r, d.0));
        }
        // `nf·m` for a MOS, `m` otherwise (`Drawn::fingers`).
        let dev_nf: Vec<u16> = members.iter().map(|&d| nf(d)).collect();
        let g = dev_nf.iter().fold(0, |a, &b| gcd(a, b)).max(1);
        members.iter().for_each(|d| covered[d.0 as usize] = true);
        c.unitization.push(Unitization {
            devices: members,
            device_type: kind,
            target_ratio: dev_nf.iter().map(|&n| n / g).collect(),
            dev_nf,
            unit_w: clamp(d0.w_finger_nm),
            unit_l: clamp(d0.l_nm),
            // A set's units are in parallel (`m` capacitors are `m` plates); a
            // resistor set with series units falls back above (one finger L).
            series_parallel: SeriesParallel::Parallel,
            dummy_required: dummy_required(kind, s.class),
            route_matching_required: s.kind != MatchKind::Ratio || s.class >= MatchClass::Moderate,
            class: Some(s.class),
            kind: Some(s.kind),
            series: vec![1; s.members.len()],
            style: Some(s.style),
        });
    }
    let class_of = |d: DeviceId| {
        let s = &drawn[d.0 as usize];
        (netlist.devices[d.0 as usize].kind, s.model, s.bulk, clamp(s.w_finger_nm), clamp(s.l_nm))
    };

    for b in blocks.iter().filter(|b| b.kind != BlockKind::Glue) {
        let mut classes = Vec::new();
        for &d in b.devices.iter().filter(|d| !covered[d.0 as usize]) {
            if !classes.contains(&class_of(d)) {
                classes.push(class_of(d));
            }
        }
        for class in classes {
            let (kind, _, _, unit_w, unit_l) = class;
            let devices: Vec<DeviceId> = b.devices.iter().copied().filter(|&d| !covered[d.0 as usize] && class_of(d) == class).collect();
            c.unitization.push(plain_unit(netlist, devices, kind, unit_w, unit_l, series_parallel(kind), true));
        }
        // A device shared by two blocks is drawn once, by the first.
        b.devices.iter().for_each(|d| covered[d.0 as usize] = true);
    }
    let keyed = netlist.devices.iter().enumerate().filter_map(|(i, d)| {
        if covered[i] || !matches!(d.kind, DeviceKind::Nmos | DeviceKind::Pmos) || drawn[i].w_finger_nm.is_none() {
            return None;
        }
        let mut t: Vec<(&str, u16)> = d.terminals.iter().map(|(n, x)| (n.as_str(), x.0)).collect();
        t.sort_unstable();
        Some(((d.kind as u8, drawn[i].model, drawn[i].w_finger_nm, drawn[i].l_nm, t), DeviceId(i as u16)))
    });
    for (key, devices) in crate::sets::group(keyed).into_iter().filter(|(_, v)| v.len() > 1) {
        let kind = netlist.devices[devices[0].0 as usize].kind;
        c.unitization.push(plain_unit(netlist, devices, kind, clamp(key.2), clamp(key.3), SeriesParallel::Parallel, false));
    }
    c
}

/// Greatest common divisor; `gcd(0, 0) = 0`.
fn gcd(a: u16, b: u16) -> u16 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Step-2 coverage: helpers and every `assemble` pass.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use analog::intent::{ArrayStyle, ClassSource, ConstraintId, Family, Member};
    use pnr_core::ids::NetId;
    use pnr_core::netlist::{Device, Net};

    use crate::tests::fet;

    fn two(name: &str, kind: DeviceKind, p: u16, n: u16, params: &[(&str, i64)]) -> Device {
        Device {
            name: name.into(),
            kind,
            model: String::new(),
            terminals: vec![("P".into(), NetId(p)), ("N".into(), NetId(n))],
            params: params.iter().map(|&(k, v)| (k.into(), v)).collect(),
        }
    }

    fn netlist(devices: Vec<Device>) -> Netlist {
        Netlist { devices, nets: (0..10).map(|i| Net { name: format!("n{i}") }).collect(), ..Default::default() }
    }

    fn drawn(nl: &Netlist) -> Vec<Drawn> {
        let mut models = Vec::new();
        nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect()
    }

    fn spec(origin: Origin, family: Family, kind: MatchKind, class: MatchClass, members: &[u16]) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin,
            members: members.iter().map(|&d| Member { device: DeviceId(d), parallel: 1, series: 1, half: None }).collect(),
            reference: None,
            family,
            kind,
            class,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Adjacent,
            compound: None,
        }
    }

    fn group(devices: &[u16]) -> Block {
        Block { kind: BlockKind::Group, template: "t", devices: devices.iter().map(|&d| DeviceId(d)).collect(), injected: false, sub_blocks: Vec::new(), selfs: Vec::new() }
    }

    fn devs(u: &Unitization) -> Vec<u16> {
        u.devices.iter().map(|d| d.0).collect()
    }

    #[test]
    fn gcd_corners() {
        assert_eq!((gcd(0, 0), gcd(0, 5), gcd(5, 0), gcd(12, 18), gcd(7, 13), gcd(u16::MAX, u16::MAX)), (0, 5, 5, 6, 1, u16::MAX));
    }

    #[test]
    fn dummies_by_class_and_kind() {
        for k in [DeviceKind::Npn, DeviceKind::Pnp] {
            for c in [MatchClass::Minimal, MatchClass::Moderate, MatchClass::Exceptional] {
                assert!(!dummy_required(k, c));
            }
        }
        for k in [DeviceKind::Nmos, DeviceKind::Pmos, DeviceKind::Resistor, DeviceKind::Capacitor] {
            assert_eq!([MatchClass::Minimal, MatchClass::Moderate, MatchClass::Exceptional].map(|c| dummy_required(k, c)), [false, true, true], "{k:?}");
        }
    }

    #[test]
    fn fingers_are_nf_times_m_saturating() {
        let mut d = fet("M", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500);
        assert_eq!(fingers(&d), 1);
        d.params.extend([("nf".into(), 2), ("m".into(), 3)]);
        assert_eq!(fingers(&d), 6);
        d.params.push(("multi".into(), 100_000));
        assert_eq!(fingers(&d), u16::MAX);
        assert_eq!(fingers(&two("R", DeviceKind::Resistor, 0, 1, &[("m", 4)])), 1, "not a MOS");
        d.params.retain(|(k, _)| k != "w");
        assert_eq!(fingers(&d), 1, "unknown size");
    }

    /// Banks on `a` and `b`, bridge from `a` to `b`.
    fn split_dac() -> Netlist {
        netlist(vec![
            two("C0", DeviceKind::Capacitor, 1, 0, &[("m", 1)]),
            two("C1", DeviceKind::Capacitor, 2, 0, &[("m", 2)]),
            two("CB", DeviceKind::Capacitor, 1, 2, &[("m", 1)]),
        ])
    }

    #[test]
    fn split_bridge_is_the_unique_member_between_two_tops() {
        let nl = split_dac();
        let s = spec(Origin::PassiveSet { rule: "split_dac" }, Family::Capacitor, MatchKind::Ratio, MatchClass::Moderate, &[0, 1, 2]);
        assert_eq!(split_bridge(&nl, &s), Some(DeviceId(2)));
        // Without the bridge no member qualifies.
        let s2 = spec(Origin::PassiveSet { rule: "split_dac" }, Family::Capacitor, MatchKind::Ratio, MatchClass::Moderate, &[0, 1]);
        assert_eq!(split_bridge(&nl, &s2), None);
        // Two bridges (both between a and b): ambiguous.
        let mut nl2 = split_dac();
        nl2.devices.push(two("CB2", DeviceKind::Capacitor, 2, 1, &[]));
        let s3 = spec(Origin::PassiveSet { rule: "split_dac" }, Family::Capacitor, MatchKind::Ratio, MatchClass::Moderate, &[0, 1, 2, 3]);
        assert_eq!(split_bridge(&nl2, &s3), None);
    }

    #[test]
    fn nothing_in_nothing_out() {
        let nl = netlist(vec![]);
        assert!(assemble(&nl, &[], &[], &[]).unitization.is_empty());
    }

    /// A ratioed set: target ratio reduced by the gcd, sizes from the bank,
    /// flags from the class, members covered once.
    #[test]
    fn a_matched_set_is_one_unitization() {
        let mut a = fet("M0", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500);
        a.params.push(("nf".into(), 2));
        let mut b = fet("M1", DeviceKind::Nmos, 0, 4, 2, 3, 8_000, 500);
        b.params.push(("nf".into(), 4));
        let nl = netlist(vec![a, b]);
        let s = spec(Origin::SharedBias, Family::Mos, MatchKind::Current, MatchClass::Moderate, &[0, 1]);
        // The same devices in a block are not drawn twice.
        let c = assemble(&nl, &drawn(&nl), &[group(&[0, 1])], &[s]);
        assert_eq!(c.unitization.len(), 1);
        let u = &c.unitization[0];
        assert_eq!((devs(u), u.dev_nf.clone(), u.target_ratio.clone(), u.unit_w, u.unit_l), (vec![0, 1], vec![2, 4], vec![1, 2], 2_000, 500));
        assert_eq!((u.series_parallel, u.dummy_required, u.route_matching_required), (SeriesParallel::Parallel, true, true));
        assert_eq!((u.class, u.kind, u.style, u.series.clone()), (Some(MatchClass::Moderate), Some(MatchKind::Current), Some(ArrayStyle::Adjacent), vec![1, 1]));
    }

    /// A Minimal Ratio set needs no route matching; any other set does.
    #[test]
    fn route_matching_by_kind_and_class() {
        let nl = netlist(vec![two("R0", DeviceKind::Resistor, 1, 0, &[("w", 1_000), ("l", 9_000)]), two("R1", DeviceKind::Resistor, 2, 0, &[("w", 1_000), ("l", 9_000)])]);
        let rm = |kind, class| assemble(&nl, &drawn(&nl), &[], &[spec(Origin::SharedBias, Family::Resistor, kind, class, &[0, 1])]).unitization[0].route_matching_required;
        assert!(!rm(MatchKind::Ratio, MatchClass::Minimal));
        assert!(rm(MatchKind::Ratio, MatchClass::Moderate));
        assert!(rm(MatchKind::Current, MatchClass::Minimal));
    }

    /// A non-MOS set lists members by count, reference first among equals, then id.
    #[test]
    fn a_passive_set_is_sorted_by_count_then_reference() {
        let nl = netlist(vec![
            two("C0", DeviceKind::Capacitor, 1, 0, &[("m", 2)]),
            two("C1", DeviceKind::Capacitor, 2, 0, &[("m", 1)]),
            two("C2", DeviceKind::Capacitor, 3, 0, &[("m", 1)]),
        ]);
        let mut s = spec(Origin::SharedBias, Family::Capacitor, MatchKind::Ratio, MatchClass::Moderate, &[0, 1, 2]);
        s.reference = Some(2);
        let c = assemble(&nl, &drawn(&nl), &[], &[s]);
        assert_eq!(devs(&c.unitization[0]), [2, 1, 0]);
        assert_eq!(c.unitization[0].target_ratio, [1, 1, 2]);
    }

    /// A split DAC's bridge may differ in size: only the banks must agree.
    #[test]
    fn a_split_dac_bridge_is_exempt_from_the_common_unit() {
        let mut nl = split_dac();
        for d in &mut nl.devices[..2] {
            d.params.extend([("w".into(), 5_000), ("l".into(), 5_000)]);
        }
        nl.devices[2].params.extend([("w".into(), 7_000), ("l".into(), 7_000)]);
        let s = spec(Origin::PassiveSet { rule: "split_dac" }, Family::Capacitor, MatchKind::Ratio, MatchClass::Moderate, &[0, 1, 2]);
        let c = assemble(&nl, &drawn(&nl), &[], &[s.clone()]);
        assert_eq!(c.unitization.len(), 1);
        assert_eq!((c.unitization[0].unit_w, c.unitization[0].devices.len()), (5_000, 3));
        // Under any other rule the bridge's size splits the set: it is skipped.
        let other = MatchSpec { origin: Origin::PassiveSet { rule: "cap_ratio" }, ..s };
        assert!(assemble(&nl, &drawn(&nl), &[], &[other]).unitization.is_empty());
    }

    /// A set whose members differ in size falls through to its block's classes.
    #[test]
    fn a_mixed_size_set_falls_back_to_block_classes() {
        let nl = netlist(vec![
            fet("M0", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500),
            fet("M1", DeviceKind::Nmos, 0, 4, 2, 3, 4_000, 600),
            fet("M2", DeviceKind::Nmos, 5, 6, 2, 3, 4_000, 500),
        ]);
        let s = spec(Origin::SharedBias, Family::Mos, MatchKind::Current, MatchClass::Moderate, &[0, 1]);
        let c = assemble(&nl, &drawn(&nl), &[group(&[0, 1, 2])], &[s]);
        let got: Vec<(Vec<u16>, i32, bool, Option<MatchClass>)> = c.unitization.iter().map(|u| (devs(u), u.unit_l, u.dummy_required, u.class)).collect();
        assert_eq!(got, [(vec![0, 2], 500, true, None), (vec![1], 600, true, None)]);
    }

    /// An empty set is skipped, never indexed.
    #[test]
    fn an_empty_set_is_skipped() {
        let nl = netlist(vec![fet("M0", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500)]);
        let s = spec(Origin::SharedBias, Family::Mos, MatchKind::Current, MatchClass::Moderate, &[]);
        assert!(assemble(&nl, &drawn(&nl), &[], &[s]).unitization.is_empty());
    }

    /// Block classes split on kind, model and bulk; a resistor class is a series string.
    #[test]
    fn block_classes_split_by_kind_model_and_bulk() {
        let mut lvt = fet("M1", DeviceKind::Nmos, 0, 4, 2, 3, 4_000, 500);
        lvt.model = "lvt".into();
        let nl = netlist(vec![
            fet("M0", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500),
            lvt,
            fet("M2", DeviceKind::Nmos, 0, 5, 2, 9, 4_000, 500),
            two("R", DeviceKind::Resistor, 1, 6, &[("w", 1_000), ("l", 2_000)]),
        ]);
        let c = assemble(&nl, &drawn(&nl), &[group(&[0, 1, 2, 3])], &[]);
        let got: Vec<(Vec<u16>, SeriesParallel)> = c.unitization.iter().map(|u| (devs(u), u.series_parallel)).collect();
        assert_eq!(got, [(vec![0], SeriesParallel::Parallel), (vec![1], SeriesParallel::Parallel), (vec![2], SeriesParallel::Parallel), (vec![3], SeriesParallel::Series)]);
    }

    /// Glue draws nothing itself; a device two blocks share is drawn once.
    #[test]
    fn glue_is_skipped_and_shared_devices_draw_once() {
        let nl = netlist(vec![fet("M0", DeviceKind::Nmos, 0, 1, 2, 3, 4_000, 500), fet("M1", DeviceKind::Nmos, 7, 8, 2, 3, 4_000, 500)]);
        let glue = Block { kind: BlockKind::Glue, ..group(&[0, 1]) };
        assert!(assemble(&nl, &drawn(&nl), &[glue], &[]).unitization.is_empty(), "distinct terminals: no parallel group either");
        let c = assemble(&nl, &drawn(&nl), &[group(&[0, 1]), group(&[1])], &[]);
        assert_eq!(c.unitization.iter().map(devs).collect::<Vec<_>>(), [vec![0, 1]]);
    }

    /// Identical parallel MOS left uncovered share one unit without dummies or
    /// route matching; an unknown size or a lone device gets none.
    #[test]
    fn identical_parallel_mos_share_a_unit() {
        let p = |n| fet(n, DeviceKind::Pmos, 0, 1, 2, 2, 6_000, 500);
        let nl = netlist(vec![p("A"), p("B"), p("C"), fet("D", DeviceKind::Pmos, 0, 1, 3, 2, 6_000, 500)]);
        let c = assemble(&nl, &drawn(&nl), &[], &[]);
        assert_eq!(c.unitization.len(), 1);
        let u = &c.unitization[0];
        assert_eq!((devs(u), u.unit_w, u.dummy_required, u.route_matching_required, u.device_type), (vec![0, 1, 2], 6_000, false, false, DeviceKind::Pmos));
        let mut nl = netlist(vec![p("A"), p("B")]);
        nl.devices.iter_mut().for_each(|d| d.params.retain(|(k, _)| k != "w"));
        assert!(assemble(&nl, &drawn(&nl), &[], &[]).unitization.is_empty());
    }

    /// Sizes past `i32::MAX` nm clamp, never wrap.
    #[test]
    fn huge_sizes_clamp() {
        let nl = netlist(vec![fet("A", DeviceKind::Nmos, 0, 1, 2, 3, i64::MAX, i64::MAX)]);
        let c = assemble(&nl, &drawn(&nl), &[group(&[0])], &[]);
        assert_eq!((c.unitization[0].unit_w, c.unitization[0].unit_l), (i32::MAX, i32::MAX));
    }
}
