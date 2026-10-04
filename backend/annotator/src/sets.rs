//! Matched sets (EXT-15): shared-bias groups, ratio inference and unitization
//! (Hastings MOS rules 1, 11 and eq. 13.49; resistor rule 5; BJT unit
//! emitters; Lampaert: a ratioed group is built of equal units).

use analog::intent::{
    ClassSource, Compound, ConstraintId, Diagnostic, Family, Half, MatchClass, MatchKind, MatchSpec, Member, Origin, ArrayStyle, ReqType,
    UnitGeom,
};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::block::Block;
use crate::graph::Req;
use crate::size::Drawn;

/// Unitization bounds, nm, from the deck (`library::annotation`; sky130 5, 420,
/// 10000, 150, 10000): fabrication grid, cell `min_finger_width` and
/// `max_finger_width`, poly min width, cell `res_min_segment`. `0` = the key is
/// missing, and [`unitize`] answers `unit_deck_incomplete`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnitDeck {
    pub grid_nm: i64,
    pub min_w_nm: i64,
    pub max_w_nm: i64,
    pub min_l_nm: i64,
    pub res_min_segment_nm: i64,
}

fn fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
}

fn bjt(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Npn | DeviceKind::Pnp)
}

fn net(hg: &BipartiteHypergraph, d: usize, t: &str) -> Option<pnr_core::ids::NetId> {
    hg.terminals[d].iter().position(|p| p == t).map(|i| hg.device_nets[d][i])
}

/// Diode-connected: a FET with `D == G`, a BJT with `C == B`.
pub(crate) fn diode(hg: &BipartiteHypergraph, d: usize) -> bool {
    let k = hg.kinds[d];
    (fet(k) && net(hg, d, "D").is_some() && net(hg, d, "D") == net(hg, d, "G"))
        || (bjt(k) && net(hg, d, "C").is_some() && net(hg, d, "C") == net(hg, d, "B"))
}

/// Devices of one kind, model and L sharing gate (FET) or base (BJT) net and
/// source (emitter) net, with a diode-connected member or a gate (base) net that
/// is not Signal; groups of two or more on at least two drains (collectors), the
/// diode reference first, then by id.
#[must_use]
pub fn shared_bias_groups(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<Vec<DeviceId>> {
    let mut groups: Vec<(_, Vec<usize>)> = Vec::new();
    for (d, dr) in drawn.iter().enumerate() {
        let k = hg.kinds[d];
        let (g, s) = if fet(k) { ("G", "S") } else if bjt(k) { ("B", "E") } else { continue };
        let key = (k, dr.model, dr.l_nm, net(hg, d, g), net(hg, d, s));
        match groups.iter_mut().find(|(x, _)| *x == key) {
            Some((_, v)) => v.push(d),
            None => groups.push((key, vec![d])),
        }
    }
    groups
        .into_iter()
        .filter(|(key, v)| {
            let bias = key.3.is_some_and(|n| classes[n.0 as usize].class != NetClass::Signal);
            // All on one drain (collector) is one device written as several cards
            // (cellgen's parallel rule), not a ratio.
            let (dk, d0) = (if fet(key.0) { "D" } else { "C" }, v[0]);
            let one_drain = v.iter().all(|&d| net(hg, d, dk) == net(hg, d0, dk));
            v.len() > 1 && !one_drain && (bias || v.iter().any(|&d| diode(hg, d)))
        })
        .map(|(_, mut v)| {
            v.sort_by_key(|&d| (!diode(hg, d), d));
            v.into_iter().map(|d| DeviceId(d as u16)).collect()
        })
        .collect()
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn err(kind: &'static str, members: &[DeviceId], message: String) -> Diagnostic {
    Diagnostic { kind, devices: members.to_vec(), message }
}

/// The ratio as identical units: `(unit, (parallel, series) per member)`.
/// FET (T = finger W × fingers): equal L gives W_u = the largest divisor of
/// gcd(T) on the grid within `[max(min_w, 1000 nm if ≥ Moderate), max_w]`
/// (Hastings MOS rule 11), parallel = T / W_u; unequal L adds L_u = gcd(L) ≥
/// `max(min_l, 1000 nm if ≥ Moderate)` and series = L / L_u. Resistor: equal W
/// and model, L_u = gcd(L) ≥ `res_min_segment`, series = L / L_u, parallel = `m`.
/// Capacitor: equal W, L and model gives units = `m`; else integer multiples of
/// the smallest area at one W. BJT and diode: equal W, L and model gives units = `m`.
///
/// # Errors
/// `unit_deck_incomplete` (a deck key is 0), `non_integer_ratio`,
/// `resistor_widths_differ` or `unknown_size`; the set then has `unit: None`.
pub fn unitize(members: &[DeviceId], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck) -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic> {
    let d = |m: &DeviceId| drawn[m.0 as usize];
    let deck_keys = [deck.grid_nm, deck.min_w_nm, deck.max_w_nm, deck.min_l_nm, deck.res_min_segment_nm];
    if deck_keys.contains(&0) {
        return Err(err("unit_deck_incomplete", members, format!("deck unit bounds {deck:?}")));
    }
    let model = members.first().map_or(0, |m| d(m).model);
    if members.iter().any(|m| d(m).model != model) {
        return Err(err("non_integer_ratio", members, "models differ".into()));
    }
    let n16 = |v: i64| u16::try_from(v).ok();
    let ratio = |msg: &str| err("non_integer_ratio", members, msg.into());
    let moderate = class >= MatchClass::Moderate;
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => {
            let (Some(ws), Some(ls)) = (members.iter().map(|m| d(m).w_finger_nm.map(|w| w * i64::from(d(m).fingers))).collect::<Option<Vec<i64>>>(), members.iter().map(|m| d(m).l_nm).collect::<Option<Vec<i64>>>()) else {
                return Err(err("unknown_size", members, "W/L unknown".into()));
            };
            let g = ws.iter().fold(0, |a, &w| gcd(a, w));
            let floor = deck.min_w_nm.max(if moderate { 1000 } else { 0 });
            let w_u = (1..=g.min(deck.max_w_nm)).rev().find(|&x| g % x == 0 && x % deck.grid_nm == 0 && x >= floor).ok_or_else(|| ratio("no unit width on the grid"))?;
            let l_u = if ls.iter().all(|&l| l == ls[0]) {
                ls[0]
            } else {
                let l_u = ls.iter().fold(0, |a, &l| gcd(a, l));
                if l_u < deck.min_l_nm.max(if moderate { 1000 } else { 0 }) || l_u % deck.grid_nm != 0 {
                    return Err(ratio("no unit length"));
                }
                l_u
            };
            let units = ws.iter().zip(&ls).map(|(&w, &l)| Some((n16(w / w_u)?, n16(l / l_u)?))).collect::<Option<Vec<_>>>().ok_or_else(|| ratio("too many units"))?;
            Ok((UnitGeom { w_nm: w_u as i32, l_nm: l_u as i32, model }, units))
        }
        DeviceKind::Resistor => {
            let w = d(&members[0]).w_finger_nm;
            if members.iter().any(|m| d(m).w_finger_nm != w) {
                return Err(err("resistor_widths_differ", members, "matched resistors need one width (Hastings R2)".into()));
            }
            let ls = members.iter().map(|m| d(m).l_nm).collect::<Option<Vec<i64>>>().ok_or_else(|| err("unknown_size", members, "L unknown".into()))?;
            let l_u = ls.iter().fold(0, |a, &l| gcd(a, l));
            if l_u < deck.res_min_segment_nm {
                return Err(ratio("segment below res_min_segment"));
            }
            let units = members.iter().zip(&ls).map(|(m, &l)| Some((n16(i64::from(d(m).fingers))?, n16(l / l_u)?))).collect::<Option<Vec<_>>>().ok_or_else(|| ratio("too many units"))?;
            Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l_u as i32, model }, units))
        }
        DeviceKind::Capacitor => {
            let (w, l) = (d(&members[0]).w_finger_nm, d(&members[0]).l_nm);
            let m = |x: &DeviceId| i64::from(d(x).fingers);
            if members.iter().all(|x| (d(x).w_finger_nm, d(x).l_nm) == (w, l)) {
                let units = members.iter().map(|x| Some((n16(m(x))?, 1))).collect::<Option<Vec<_>>>().ok_or_else(|| ratio("too many units"))?;
                return Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l.unwrap_or(0) as i32, model }, units));
            }
            if members.iter().any(|x| d(x).w_finger_nm != w) {
                return Err(ratio("capacitor widths differ"));
            }
            let ls = members.iter().map(|x| d(x).l_nm).collect::<Option<Vec<i64>>>().ok_or_else(|| err("unknown_size", members, "L unknown".into()))?;
            let l_min = *ls.iter().min().unwrap_or(&0);
            if l_min <= 0 || ls.iter().any(|&l| l % l_min != 0) {
                return Err(ratio("areas are not multiples of the smallest"));
            }
            let units = members.iter().zip(&ls).map(|(x, &l)| Some((n16(m(x) * (l / l_min))?, 1))).collect::<Option<Vec<_>>>().ok_or_else(|| ratio("too many units"))?;
            Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l_min as i32, model }, units))
        }
        _ => {
            let (w, l) = (d(&members[0]).w_finger_nm, d(&members[0]).l_nm);
            if members.iter().any(|x| (d(x).w_finger_nm, d(x).l_nm) != (w, l)) {
                return Err(ratio("unit emitters differ"));
            }
            let units = members.iter().map(|x| Some((n16(i64::from(d(x).fingers))?, 1))).collect::<Option<Vec<_>>>().ok_or_else(|| ratio("too many units"))?;
            Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l.unwrap_or(0) as i32, model }, units))
        }
    }
}

/// Matched sets: components of the `MatchSym ∪ MatchBlock` edges (shared-bias and
/// passive groups arrive as `MatchBlock` stars), split by `(kind, model)` with a
/// `mixed_kind_set` diagnostic; singletons and inductors dropped. Ordered by their
/// members' smallest canonical label; members in canonical order. `origin` is
/// `SharedBias` when the set holds a whole shared group, else `PassiveSet` of
/// the first passive set inside it (EXT-19), else the first of `leaves`
/// (`block::leaves`) inside it, else its first compound's seed. A split DAC's
/// bridge is left out of the unit (it gets `(1, 1)`).
/// `kind` is interim (Ratio for R/C, Current otherwise; EXT-16 infers it), class
/// Moderate by Role. Card departure: `leaves` and `canon` are extra arguments
/// (the template and the canonical order need them), and `passive` replaces
/// nothing (its groups also arrive as `reqs` stars) but names their origin.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn matched_sets(
    reqs: &[Req],
    compounds: &[Compound],
    shared: &[Vec<DeviceId>],
    passive: &[crate::passive::PassiveSet],
    leaves: &[&Block],
    canon: &[u64],
    drawn: &[Drawn],
    hg: &BipartiteHypergraph,
    deck: &UnitDeck,
    diags: &mut Vec<Diagnostic>,
) -> Vec<MatchSpec> {
    let n = hg.device_count();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for r in reqs.iter().filter(|r| matches!(r.ty, ReqType::MatchSym | ReqType::MatchBlock)) {
        let (x, y) = (find(&mut parent, r.a.0 as usize), find(&mut parent, r.b.0 as usize));
        parent[x.max(y)] = x.min(y);
    }
    let mut comps: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for d in 0..n {
        let r = find(&mut parent, d);
        comps.entry(r).or_default().push(d);
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for c in comps.into_values().filter(|c| c.len() > 1) {
        let mut split: Vec<((DeviceKind, u16), Vec<usize>)> = Vec::new();
        for d in c {
            let k = (hg.kinds[d], drawn[d].model);
            match split.iter_mut().find(|(x, _)| *x == k) {
                Some((_, v)) => v.push(d),
                None => split.push((k, vec![d])),
            }
        }
        if split.len() > 1 {
            diags.push(Diagnostic {
                kind: "mixed_kind_set",
                devices: split.iter().flat_map(|(_, v)| v.iter().map(|&d| DeviceId(d as u16))).collect(),
                message: "a matched component mixes device kinds or models; split by (kind, model)".into(),
            });
        }
        groups.extend(split.into_iter().map(|(_, v)| v).filter(|v| v.len() > 1 && Family::of(hg.kinds[v[0]]).is_some()));
    }
    for g in &mut groups {
        g.sort_by_key(|&d| (canon[d], d));
    }
    groups.sort_by_key(|g| (canon[g[0]], g[0]));

    let mut half = vec![None; n];
    let mut comp_of = vec![None; n];
    for (ci, c) in compounds.iter().enumerate() {
        for &(a, b) in &c.pairs {
            half[a.0 as usize] = Some(Half::A);
            half[b.0 as usize] = Some(Half::B);
        }
        for d in c.pairs.iter().flat_map(|&(a, b)| [a, b]).chain(c.selfs.iter().copied()) {
            comp_of[d.0 as usize] = Some(ci as u16);
        }
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(i, g)| {
            let kind = hg.kinds[g[0]];
            let ids: Vec<DeviceId> = g.iter().map(|&d| DeviceId(d as u16)).collect();
            let has = |d: &DeviceId| g.contains(&(d.0 as usize));
            let origin = if shared.iter().any(|s| s.iter().all(has)) {
                Origin::SharedBias
            } else if let Some(p) = passive.iter().find(|p| p.devices.iter().all(has)) {
                Origin::PassiveSet { rule: p.rule }
            } else if let Some(b) = leaves.iter().find(|b| b.devices.iter().all(has)) {
                Origin::Pattern { template: b.template }
            } else {
                Origin::Symmetry { seed: ConstraintId(u32::from(g.iter().find_map(|&d| comp_of[d]).unwrap_or(0))) }
            };
            let (unit, units) = match unitize_set(&ids, passive, drawn, kind, MatchClass::Moderate, deck) {
                Ok((u, units)) => (Some(u), units),
                Err(e) => {
                    diags.push(e);
                    (None, g.iter().map(|&d| (drawn[d].fingers.min(u32::from(u16::MAX)) as u16, 1)).collect())
                }
            };
            let diodes: Vec<usize> = (0..g.len()).filter(|&j| fet(kind) && diode(hg, g[j])).collect();
            let bank_ref = passive.iter().filter(|p| p.devices.iter().all(has)).find_map(|p| p.reference).and_then(|r| g.iter().position(|&d| d == r.0 as usize));
            MatchSpec {
                id: ConstraintId(i as u32),
                origin,
                members: g.iter().zip(&units).map(|(&d, &(parallel, series))| Member { device: DeviceId(d as u16), parallel, series, half: half[d] }).collect(),
                reference: bank_ref.or((diodes.len() == 1).then(|| diodes[0])),
                family: Family::of(kind).expect("inductors dropped above"),
                kind: if matches!(kind, DeviceKind::Resistor | DeviceKind::Capacitor) { MatchKind::Ratio } else { MatchKind::Current },
                class: MatchClass::Moderate,
                class_source: ClassSource::Role,
                unit,
                allowance: None,
                weight: None,
                style: ArrayStyle::Any,
                compound: g.iter().find_map(|&d| comp_of[d]),
            }
        })
        .collect()
}

/// [`unitize`] over `members` less any split-DAC bridge, which gets `(1, 1)`.
///
/// # Errors
/// As [`unitize`].
pub fn unitize_set(members: &[DeviceId], passive: &[crate::passive::PassiveSet], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck) -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic> {
    let bridge = |d: &DeviceId| passive.iter().any(|p| p.bridge == Some(*d));
    let core: Vec<DeviceId> = members.iter().copied().filter(|d| !bridge(d)).collect();
    let (u, units) = unitize(&core, drawn, kind, class, deck)?;
    let mut it = units.into_iter();
    Ok((u, members.iter().map(|d| if bridge(d) { (1, 1) } else { it.next().unwrap_or((1, 1)) }).collect()))
}

/// Nested symmetry (EXT-14 step 8): `(i, j)`, `i < j`, on the compound whose
/// pairs map set i's members bijectively onto set j's.
pub fn set_pairs(compounds: &mut [Compound], sets: &[MatchSpec]) {
    for c in compounds.iter_mut() {
        let mate = |d: DeviceId| c.pairs.iter().find_map(|&(a, b)| if a == d { Some(b) } else if b == d { Some(a) } else { None });
        let members = |s: &MatchSpec| {
            let mut v: Vec<u16> = s.members.iter().map(|m| m.device.0).collect();
            v.sort_unstable();
            v
        };
        let mut found = Vec::new();
        for (i, si) in sets.iter().enumerate() {
            let Some(mut img) = si.members.iter().map(|m| mate(m.device).map(|d| d.0)).collect::<Option<Vec<u16>>>() else { continue };
            img.sort_unstable();
            for (j, sj) in sets.iter().enumerate().skip(i + 1) {
                if img == members(sj) {
                    found.push((i as u16, j as u16));
                }
            }
        }
        c.set_pairs = found;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{fet as mk, nets};
    use pnr_core::netlist::{Device, Netlist};

    const DECK: UnitDeck = UnitDeck { grid_nm: 5, min_w_nm: 420, max_w_nm: 10_000, min_l_nm: 150, res_min_segment_nm: 10_000 };

    fn drawn(nl: &Netlist) -> Vec<Drawn> {
        let mut models = Vec::new();
        nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect()
    }

    /// NMOS `(W nm, L nm, nf)` on distinct nets.
    fn mos(sizes: &[(i64, i64, i64)]) -> Netlist {
        let devices = sizes
            .iter()
            .enumerate()
            .map(|(i, &(w, l, nf))| {
                let mut d = mk(&format!("M{i}"), DeviceKind::Nmos, 0, 1, 2, 2, w, l);
                d.params.push(("nf".into(), nf));
                d
            })
            .collect();
        Netlist { devices, nets: nets(&["g", "d", "VSS"]), ..Default::default() }
    }

    fn ids(n: usize) -> Vec<DeviceId> {
        (0..n as u16).map(DeviceId).collect()
    }

    fn par(u: &[(u16, u16)]) -> Vec<u16> {
        u.iter().map(|p| p.0).collect()
    }

    #[test]
    fn mirror_ratio_by_width() {
        let nl = mos(&[(2_000, 1_000, 1), (4_000, 1_000, 1), (8_000, 1_000, 1)]);
        let (u, units) = unitize(&ids(3), &drawn(&nl), DeviceKind::Nmos, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!((u.w_nm, par(&units)), (2000, vec![1, 2, 4]));
    }

    #[test]
    fn three_stage_bias_group() {
        let nl = crate::tests::three_stage();
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let dr = drawn(&nl);
        let groups = shared_bias_groups(&hg, &dr, &p.net_classes);
        let named: Vec<Vec<&str>> = groups
            .iter()
            .map(|g| {
                let mut v: Vec<&str> = g.iter().map(|d| nl.devices[d.0 as usize].name.as_str()).collect();
                v.sort_unstable();
                v
            })
            .collect();
        // Card departure: the plan's rule also groups the diode mirror {M4, M5} (the
        // DP's load, gate n1, M4 diode-connected); the card's "exactly [[M3,M7,M9]]" missed it.
        assert_eq!(named, [vec!["M3", "M7", "M9"], vec!["M4", "M5"]]);
        let mut g = groups[0].clone();
        g.sort_by_key(|d| nl.devices[d.0 as usize].name.clone());
        let (u, units) = unitize(&g, &dr, DeviceKind::Nmos, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!((u.w_nm, par(&units)), (2000, vec![4, 3, 10]));
    }

    #[test]
    fn hastings_unit_example() {
        let nl = mos(&[(100_000, 500, 1), (200_000, 500, 2)]);
        let (u, units) = unitize(&ids(2), &drawn(&nl), DeviceKind::Nmos, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!((u.w_nm, par(&units)), (10_000, vec![10, 20]));
    }

    #[test]
    fn series_parallel_20_to_1() {
        let nl = mos(&[(10_000, 4_000, 1), (50_000, 1_000, 5)]);
        let (u, units) = unitize(&ids(2), &drawn(&nl), DeviceKind::Nmos, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!((u.l_nm, u.w_nm), (1000, 10_000));
        assert_eq!(units, [(1, 4), (5, 1)]);
    }

    #[test]
    fn resistor_divider() {
        let r = |name: &str, p: u16, n: u16, l: i64| Device {
            name: name.into(),
            kind: DeviceKind::Resistor,
            model: "rpoly".into(),
            terminals: vec![("P".into(), pnr_core::ids::NetId(p)), ("N".into(), pnr_core::ids::NetId(n))],
            params: vec![("w".into(), 2_000), ("l".into(), l)],
        };
        let nl = Netlist { devices: vec![r("RA", 0, 1, 10_000), r("RB", 1, 2, 40_000)], nets: nets(&["top", "mid", "VSS"]), ..Default::default() };
        let (u, units) = unitize(&ids(2), &drawn(&nl), DeviceKind::Resistor, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!(u.l_nm, 10_000);
        assert_eq!(units.iter().map(|p| p.1).collect::<Vec<_>>(), [1, 4]);
    }

    #[test]
    fn missing_deck_key() {
        let nl = mos(&[(2_000, 1_000, 1), (4_000, 1_000, 1)]);
        let e = unitize(&ids(2), &drawn(&nl), DeviceKind::Nmos, MatchClass::Moderate, &UnitDeck { max_w_nm: 0, ..DECK }).unwrap_err();
        assert_eq!(e.kind, "unit_deck_incomplete");
    }

    #[test]
    fn non_integer() {
        let nl = mos(&[(2_000, 1_000, 1), (3_000, 1_500, 1)]);
        let e = unitize(&ids(2), &drawn(&nl), DeviceKind::Nmos, MatchClass::Moderate, &DECK).unwrap_err();
        assert_eq!(e.kind, "non_integer_ratio");
    }
}
