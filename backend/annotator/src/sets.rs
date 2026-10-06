//! Matched sets (EXT-15): shared-bias groups, ratio inference and unitization
//! (Hastings MOS rules 1, 11 and eq. 13.49; resistor rule 5; BJT unit
//! emitters; Lampaert: a ratioed group is built of equal units).
//!
//! Also home of the crate's small grouping kernels ([`group`], [`device_index`],
//! [`inside`], [`find`], [`union`]) that the other set and symmetry passes share.

use analog::intent::{
    ArrayStyle, ClassSource, Compound, ConstraintId, Diagnostic, Family, Half, MatchClass, MatchKind, MatchSpec, Member, Origin, ReqType,
    UnitGeom,
};
use analog::metadata::{NetClass, NetClassification};
use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;
use pnr_core::BipartiteHypergraph;

use crate::block::Block;
use crate::graph::Req;
use crate::pattern::pin_net;
use crate::passive::PassiveSet;
use crate::size::Drawn;

/// Unitization bounds, nm, from the deck (`library::annotation`; sky130 5, 420,
/// 10000, 150, 10000). A field of `0` means the deck key is missing, and
/// [`unitize`] answers `unit_deck_incomplete`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnitDeck {
    /// Fabrication grid, nm: every unit width and split length is a multiple.
    pub grid_nm: i64,
    /// Cell `min_finger_width`, nm: the narrowest unit width.
    pub min_w_nm: i64,
    /// Cell `max_finger_width`, nm: the widest unit width.
    pub max_w_nm: i64,
    /// Poly minimum width, nm: the shortest unit length when lengths differ.
    pub min_l_nm: i64,
    /// Cell `res_min_segment`, nm: the shortest resistor unit segment.
    pub res_min_segment_nm: i64,
}

/// Unit width and split-length floor, nm, for a Moderate or tighter class
/// (Hastings MOS rule 11: matched devices at least 1 µm wide and long).
const MATCHED_MIN_NM: i64 = 1000;

/// NMOS or PMOS.
pub(crate) fn fet(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Nmos | DeviceKind::Pmos)
}

/// NPN or PNP.
pub(crate) fn bjt(k: DeviceKind) -> bool {
    matches!(k, DeviceKind::Npn | DeviceKind::Pnp)
}

/// Net on device `d`'s terminal `t`, `None` when `d` has no such terminal.
pub(crate) fn net(hg: &BipartiteHypergraph, d: usize, t: &str) -> Option<pnr_core::ids::NetId> {
    pin_net(hg, d as u32, t)
}

/// Diode-connected: a FET with `D == G`, a BJT with `C == B` (both terminals present).
pub(crate) fn diode(hg: &BipartiteHypergraph, d: usize) -> bool {
    let k = hg.kinds[d];
    let (out, ctl) = if fet(k) {
        ("D", "G")
    } else if bjt(k) {
        ("C", "B")
    } else {
        return false;
    };
    net(hg, d, out).is_some_and(|n| net(hg, d, ctl) == Some(n))
}

/// Devices of one kind, model and L sharing gate (FET) or base (BJT) net and
/// source (emitter) net, with a diode-connected member or a gate (base) net that
/// is not Signal; groups of two or more on at least two drains (collectors), the
/// diode reference first, then by id. Groups come in order of their lowest id.
///
/// # Panics
/// If `drawn` is longer than `hg`'s devices or `classes` misses a gate net.
#[must_use]
pub fn shared_bias_groups(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<Vec<DeviceId>> {
    let keyed = drawn.iter().enumerate().filter_map(|(d, dr)| {
        let k = hg.kinds[d];
        let (g, s) = if fet(k) {
            ("G", "S")
        } else if bjt(k) {
            ("B", "E")
        } else {
            return None;
        };
        Some(((k as u8, dr.model, dr.l_nm, net(hg, d, g), net(hg, d, s)), d))
    });
    group(keyed)
        .into_iter()
        .filter(|(key, v)| {
            let bias = key.3.is_some_and(|n| classes[n.0 as usize].class != NetClass::Signal);
            // All on one drain (collector) is one device written as several cards
            // (cellgen's parallel rule), not a ratio.
            let dk = if fet(hg.kinds[v[0]]) { "D" } else { "C" };
            let one_drain = v.iter().all(|&d| net(hg, d, dk) == net(hg, v[0], dk));
            v.len() > 1 && !one_drain && (bias || v.iter().any(|&d| diode(hg, d)))
        })
        .map(|(_, mut v)| {
            v.sort_by_key(|&d| (!diode(hg, d), d));
            v.into_iter().map(|d| DeviceId(d as u16)).collect()
        })
        .collect()
}

/// `(key, item)` pairs grouped by key: groups in first-seen order, items in
/// input order. O(n) expected (hashed: a linear key search over 12 k devices
/// is quadratic).
pub(crate) fn group<K: std::hash::Hash + Eq + Clone, T>(items: impl IntoIterator<Item = (K, T)>) -> Vec<(K, Vec<T>)> {
    let mut at: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    let mut out: Vec<(K, Vec<T>)> = Vec::new();
    for (k, t) in items {
        match at.get(&k) {
            Some(&i) => out[i].1.push(t),
            None => {
                at.insert(k.clone(), out.len());
                out.push((k, vec![t]));
            }
        }
    }
    out
}

/// Per device id below `n`, the positions of the `items` holding it, ascending.
///
/// # Panics
/// If an item holds a device id `>= n`.
pub(crate) fn device_index<'a>(n: usize, items: impl IntoIterator<Item = &'a [DeviceId]>) -> Vec<Vec<usize>> {
    let mut idx = vec![Vec::new(); n];
    for (i, it) in items.into_iter().enumerate() {
        for d in it {
            idx[d.0 as usize].push(i);
        }
    }
    idx
}

/// Ascending, distinct positions (from [`device_index`]) of the items touching
/// a device of `g` that `all_in` accepts. Only items touching `g` are tested,
/// not every item (12 k-device scale).
///
/// # Panics
/// If `g` yields a device past `idx`.
pub(crate) fn inside(idx: &[Vec<usize>], g: impl IntoIterator<Item = usize>, all_in: impl Fn(usize) -> bool) -> Vec<usize> {
    let mut c: Vec<usize> = g.into_iter().flat_map(|d| idx[d].iter().copied()).collect();
    c.sort_unstable();
    c.dedup();
    c.retain(|&i| all_in(i));
    c
}

/// Union-find root of `x`, halving the path on the way.
pub(crate) fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Joins the sets of `a` and `b`; the lower root id becomes the root, so a
/// component's root is its smallest member.
pub(crate) fn union(parent: &mut [usize], a: usize, b: usize) {
    let (x, y) = (find(parent, a), find(parent, b));
    parent[x.max(y)] = x.min(y);
}

/// Greatest common divisor, non-negative; `gcd(0, 0) == 0`.
fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.abs()
}

/// GCD of every value, `0` for none.
fn gcd_all(values: impl IntoIterator<Item = i64>) -> i64 {
    values.into_iter().fold(0, gcd)
}

fn err(kind: &'static str, members: &[DeviceId], message: String) -> Diagnostic {
    Diagnostic { kind, devices: members.to_vec(), message }
}

fn ratio(members: &[DeviceId], msg: &str) -> Diagnostic {
    err("non_integer_ratio", members, msg.into())
}

/// Unitization result: the shared unit and `(parallel, series)` per member.
type Units = Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic>;

/// `(parallel, series)` counts narrowed to `u16`, or `non_integer_ratio`
/// ("too many units") when one does not fit.
fn unit_counts(members: &[DeviceId], counts: impl Iterator<Item = (i64, i64)>) -> Result<Vec<(u16, u16)>, Diagnostic> {
    counts
        .map(|(p, s)| Some((u16::try_from(p).ok()?, u16::try_from(s).ok()?)))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ratio(members, "too many units"))
}

/// Every member's drawn L, or `unknown_size` when one is unknown.
fn lengths(members: &[DeviceId], drawn: &[Drawn]) -> Result<Vec<i64>, Diagnostic> {
    members.iter().map(|m| drawn[m.0 as usize].l_nm).collect::<Option<Vec<i64>>>().ok_or_else(|| err("unknown_size", members, "L unknown".into()))
}

/// The ratio as identical units: `(unit, (parallel, series) per member)`.
/// FET (T = finger W × fingers): equal L gives W_u = the largest divisor of
/// gcd(T) on the grid within `[max(min_w, 1000 nm if ≥ Moderate), max_w]`
/// (Hastings MOS rule 11), parallel = T / W_u; unequal L adds L_u = gcd(L) ≥
/// `max(min_l, 1000 nm if ≥ Moderate)` on the grid and series = L / L_u.
/// Resistor: equal W and model, L_u = gcd(L) ≥ `res_min_segment`, series =
/// L / L_u, parallel = `m`. Capacitor: equal W, L and model gives units = `m`;
/// else integer multiples of the smallest area at one W. BJT, diode and any
/// other kind: equal W, L and model gives units = `m`.
///
/// # Errors
/// `unit_deck_incomplete` (a deck key is 0), `non_integer_ratio` (models
/// differ, no unit fits, a count passes `u16::MAX`, or `members` is empty),
/// `resistor_widths_differ` or `unknown_size`; the set then has `unit: None`.
///
/// # Panics
/// If a member id is past `drawn`.
pub fn unitize(members: &[DeviceId], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck) -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic> {
    let deck_keys = [deck.grid_nm, deck.min_w_nm, deck.max_w_nm, deck.min_l_nm, deck.res_min_segment_nm];
    if deck_keys.contains(&0) {
        return Err(err("unit_deck_incomplete", members, format!("deck unit bounds {deck:?}")));
    }
    let model = members.first().map_or(0, |m| drawn[m.0 as usize].model);
    if members.iter().any(|m| drawn[m.0 as usize].model != model) {
        return Err(ratio(members, "models differ"));
    }
    let floor_nm = if class >= MatchClass::Moderate { MATCHED_MIN_NM } else { 0 };
    match kind {
        DeviceKind::Nmos | DeviceKind::Pmos => unit_fet(members, drawn, deck, floor_nm, model),
        DeviceKind::Resistor => unit_resistor(members, drawn, deck, model),
        DeviceKind::Capacitor => unit_capacitor(members, drawn, model),
        _ => unit_identical(members, drawn, model),
    }
}

/// [`unitize`]'s FET rule; `floor_nm` is the class's width and split-length floor.
fn unit_fet(members: &[DeviceId], drawn: &[Drawn], deck: &UnitDeck, floor_nm: i64, model: u16) -> Units {
    let sizes = members
        .iter()
        .map(|m| {
            let d = drawn[m.0 as usize];
            Some((d.w_finger_nm? * i64::from(d.fingers), d.l_nm?))
        })
        .collect::<Option<Vec<(i64, i64)>>>()
        .ok_or_else(|| err("unknown_size", members, "W/L unknown".into()))?;
    let g = gcd_all(sizes.iter().map(|s| s.0));
    let w_min = deck.min_w_nm.max(floor_nm);
    let w_u = (1..=g.min(deck.max_w_nm))
        .rev()
        .find(|&x| g % x == 0 && x % deck.grid_nm == 0 && x >= w_min)
        .ok_or_else(|| ratio(members, "no unit width on the grid"))?;
    // A unit width was found, so `sizes` is not empty.
    let l0 = sizes[0].1;
    let l_u = if sizes.iter().all(|s| s.1 == l0) {
        l0
    } else {
        let l_u = gcd_all(sizes.iter().map(|s| s.1));
        if l_u < deck.min_l_nm.max(floor_nm) || l_u % deck.grid_nm != 0 {
            return Err(ratio(members, "no unit length"));
        }
        l_u
    };
    let units = unit_counts(members, sizes.iter().map(|&(w, l)| (w / w_u, l / l_u)))?;
    Ok((UnitGeom { w_nm: w_u as i32, l_nm: l_u as i32, model }, units))
}

/// [`unitize`]'s resistor rule (Hastings R2: one width).
fn unit_resistor(members: &[DeviceId], drawn: &[Drawn], deck: &UnitDeck, model: u16) -> Units {
    let w = drawn[members[0].0 as usize].w_finger_nm;
    if members.iter().any(|m| drawn[m.0 as usize].w_finger_nm != w) {
        return Err(err("resistor_widths_differ", members, "matched resistors need one width (Hastings R2)".into()));
    }
    let ls = lengths(members, drawn)?;
    let l_u = gcd_all(ls.iter().copied());
    if l_u < deck.res_min_segment_nm {
        return Err(ratio(members, "segment below res_min_segment"));
    }
    let units = unit_counts(members, members.iter().zip(&ls).map(|(m, &l)| (i64::from(drawn[m.0 as usize].fingers), l / l_u)))?;
    Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l_u as i32, model }, units))
}

/// [`unitize`]'s capacitor rule: identical units, else multiples of the
/// smallest length at one width.
fn unit_capacitor(members: &[DeviceId], drawn: &[Drawn], model: u16) -> Units {
    let first = drawn[members[0].0 as usize];
    let (w, l) = (first.w_finger_nm, first.l_nm);
    let m = |x: &DeviceId| i64::from(drawn[x.0 as usize].fingers);
    if members.iter().all(|x| (drawn[x.0 as usize].w_finger_nm, drawn[x.0 as usize].l_nm) == (w, l)) {
        let units = unit_counts(members, members.iter().map(|x| (m(x), 1)))?;
        return Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l.unwrap_or(0) as i32, model }, units));
    }
    if members.iter().any(|x| drawn[x.0 as usize].w_finger_nm != w) {
        return Err(ratio(members, "capacitor widths differ"));
    }
    let ls = lengths(members, drawn)?;
    let l_min = ls.iter().copied().min().unwrap_or(0);
    if l_min <= 0 || ls.iter().any(|&l| l % l_min != 0) {
        return Err(ratio(members, "areas are not multiples of the smallest"));
    }
    let units = unit_counts(members, members.iter().zip(&ls).map(|(x, &l)| (m(x) * (l / l_min), 1)))?;
    Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l_min as i32, model }, units))
}

/// [`unitize`]'s rule for BJTs, diodes and any other kind: one unit geometry,
/// `m` units each.
fn unit_identical(members: &[DeviceId], drawn: &[Drawn], model: u16) -> Units {
    let first = drawn[members[0].0 as usize];
    let (w, l) = (first.w_finger_nm, first.l_nm);
    if members.iter().any(|x| (drawn[x.0 as usize].w_finger_nm, drawn[x.0 as usize].l_nm) != (w, l)) {
        return Err(ratio(members, "unit emitters differ"));
    }
    let units = unit_counts(members, members.iter().map(|x| (i64::from(drawn[x.0 as usize].fingers), 1)))?;
    Ok((UnitGeom { w_nm: w.unwrap_or(0) as i32, l_nm: l.unwrap_or(0) as i32, model }, units))
}

/// Matched sets: components of the `MatchSym ∪ MatchBlock` edges (shared-bias and
/// passive groups arrive as `MatchBlock` stars), split by `(kind, model)` with a
/// `mixed_kind_set` diagnostic; singletons and inductors dropped. Ordered by their
/// members' smallest canonical label; members in canonical order. `origin` is
/// `SharedBias` when the set holds a whole shared group, else `PassiveSet` of
/// the first passive set inside it (EXT-19), else the first of `leaves`
/// (`block::leaves`) inside it, else its first compound's seed. A split DAC's
/// bridge is left out of the unit (it gets `(1, 1)`). A set that cannot be
/// unitized gets `unit: None`, `(m, 1)` per member and its diagnostic.
/// `kind` is interim (Ratio for R/C, Current otherwise; EXT-16 infers it), class
/// Moderate by Role. Card departure: `leaves` and `canon` are extra arguments
/// (the template and the canonical order need them), and `passive` replaces
/// nothing (its groups also arrive as `reqs` stars) but names their origin.
///
/// # Panics
/// If `canon` or `drawn` is shorter than `hg`'s devices, or a request or
/// group names a device past them.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn matched_sets(
    reqs: &[Req],
    compounds: &[Compound],
    shared: &[Vec<DeviceId>],
    passive: &[PassiveSet],
    leaves: &[&Block],
    canon: &[u64],
    drawn: &[Drawn],
    hg: &BipartiteHypergraph,
    deck: &UnitDeck,
    diags: &mut Vec<Diagnostic>,
) -> Vec<MatchSpec> {
    let n = hg.device_count();
    let groups = match_groups(reqs, canon, drawn, hg, diags);

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
    let shared_idx = device_index(n, shared.iter().map(Vec::as_slice));
    let passive_idx = device_index(n, passive.iter().map(|p| p.devices.as_slice()));
    let leaf_idx = device_index(n, leaves.iter().map(|b| b.devices.as_slice()));
    groups
        .into_iter()
        .enumerate()
        .map(|(i, g)| {
            let kind = hg.kinds[g[0]];
            let ids: Vec<DeviceId> = g.iter().map(|&d| DeviceId(d as u16)).collect();
            let has = |d: &DeviceId| g.contains(&(d.0 as usize));
            let ps = inside(&passive_idx, g.iter().copied(), |p| passive[p].devices.iter().all(has));
            let origin = if !inside(&shared_idx, g.iter().copied(), |s| shared[s].iter().all(has)).is_empty() {
                Origin::SharedBias
            } else if let Some(&p) = ps.first() {
                Origin::PassiveSet { rule: passive[p].rule }
            } else if let Some(&b) = inside(&leaf_idx, g.iter().copied(), |b| leaves[b].devices.iter().all(has)).first() {
                Origin::Pattern { template: leaves[b].template }
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
            let bank_ref = ps.iter().find_map(|&p| passive[p].reference).and_then(|r| g.iter().position(|&d| d == r.0 as usize));
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

/// The device groups behind [`matched_sets`]: components of the match edges,
/// split by `(kind, model)` (pushing `mixed_kind_set`), groups of two or more
/// devices with a [`Family`]. Members in canonical order; groups by their
/// first member.
fn match_groups(reqs: &[Req], canon: &[u64], drawn: &[Drawn], hg: &BipartiteHypergraph, diags: &mut Vec<Diagnostic>) -> Vec<Vec<usize>> {
    let n = hg.device_count();
    let mut parent: Vec<usize> = (0..n).collect();
    for r in reqs.iter().filter(|r| matches!(r.ty, ReqType::MatchSym | ReqType::MatchBlock)) {
        union(&mut parent, r.a.0 as usize, r.b.0 as usize);
    }
    let mut comps: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for d in 0..n {
        let r = find(&mut parent, d);
        comps.entry(r).or_default().push(d);
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for c in comps.into_values().filter(|c| c.len() > 1) {
        // ponytail: linear kind search, a component holds a handful of (kind, model)s.
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
    groups
}

/// [`unitize`] over `members` less any split-DAC bridge, which gets `(1, 1)`;
/// the rest keep their order.
///
/// # Errors
/// As [`unitize`].
pub fn unitize_set(members: &[DeviceId], passive: &[PassiveSet], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck) -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic> {
    let bridge = |d: &DeviceId| passive.iter().any(|p| p.bridge == Some(*d));
    let core: Vec<DeviceId> = members.iter().copied().filter(|d| !bridge(d)).collect();
    let (u, units) = unitize(&core, drawn, kind, class, deck)?;
    let mut it = units.into_iter();
    Ok((u, members.iter().map(|d| if bridge(d) { (1, 1) } else { it.next().unwrap_or((1, 1)) }).collect()))
}

/// Nested symmetry (EXT-14 step 8): sets each compound's `set_pairs` to every
/// `(i, j)`, `i < j` indices into `sets`, whose members the compound's pairs
/// map bijectively onto each other. Overwrites any earlier `set_pairs`; a
/// device in two of a compound's pairs keeps its first mate.
pub fn set_pairs(compounds: &mut [Compound], sets: &[MatchSpec]) {
    let n = sets.iter().flat_map(|s| s.members.iter().map(|m| m.device.0 as usize + 1)).max().unwrap_or(0);
    // Each set's members sorted once: every image is compared against these.
    let sorted: Vec<Vec<DeviceId>> = sets
        .iter()
        .map(|s| {
            let mut v: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
            v.sort_unstable_by_key(|d| d.0);
            v
        })
        .collect();
    let idx = device_index(n, sorted.iter().map(Vec::as_slice));
    for c in compounds.iter_mut() {
        let mut mates = std::collections::HashMap::new();
        for &(a, b) in &c.pairs {
            mates.entry(a).or_insert(b);
            mates.entry(b).or_insert(a);
        }
        // Only sets wholly inside the pairs can map onto one another.
        let near = inside(&idx, mates.keys().map(|d| d.0 as usize).filter(|&d| d < n), |s| sorted[s].iter().all(|d| mates.contains_key(d)));
        let mut found = Vec::new();
        for (k, &i) in near.iter().enumerate() {
            let Some(mut img) = sorted[i].iter().map(|d| mates.get(d).copied()).collect::<Option<Vec<DeviceId>>>() else { continue };
            img.sort_unstable_by_key(|d| d.0);
            found.extend(near[k + 1..].iter().filter(|&&j| img == sorted[j]).map(|&j| (i as u16, j as u16)));
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

    /// EXT-14 step 8: a compound whose couples carry one set's members onto
    /// another's records `(i, j)`; three_stage's own compound maps each set onto
    /// itself, so it records none.
    #[test]
    fn set_pairs_maps_set_onto_set() {
        let nl = crate::tests::three_stage();
        let p = crate::annotate(&nl, &crate::AnnotationConfig::default());
        let id = |n: &str| DeviceId(nl.devices.iter().position(|d| d.name == n).unwrap() as u16);
        let set = |n: &str| p.intent.sets.iter().position(|s| s.members.iter().any(|m| m.device == id(n))).unwrap() as u16;
        let (i, j) = (set("M1"), set("M4"));
        assert!(p.intent.compounds.iter().all(|c| c.set_pairs.is_empty()));
        let mut c = p.intent.compounds[0].clone();
        c.pairs = vec![(id("M1"), id("M4")), (id("M2"), id("M5"))];
        let mut cs = vec![c];
        set_pairs(&mut cs, &p.intent.sets);
        assert_eq!(cs[0].set_pairs, [(i.min(j), i.max(j))]);
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

    // ---- cleanup(annotator-sets) step 2: kernels and every unitize branch ----

    /// A drawn device straight from its numbers.
    fn dr(w: Option<i64>, l: Option<i64>, fingers: u32, model: u16) -> Drawn {
        Drawn { w_finger_nm: w, l_nm: l, fingers, model, bulk: None }
    }

    fn unit(members: usize, d: &[Drawn], kind: DeviceKind, class: MatchClass) -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic> {
        unitize(&ids(members), d, kind, class, &DECK)
    }

    #[test]
    fn gcd_corners() {
        assert_eq!(gcd(0, 0), 0);
        assert_eq!(gcd(12, 18), 6);
        assert_eq!(gcd(18, 12), 6);
        assert_eq!(gcd(-4, 6), 2);
        assert_eq!(gcd(7, 0), 7);
        assert_eq!(gcd_all([]), 0);
        assert_eq!(gcd_all([7]), 7);
        assert_eq!(gcd_all([4, 6, 9]), 1);
    }

    #[test]
    fn group_keeps_first_seen_and_input_order() {
        assert!(group(Vec::<(u8, u8)>::new()).is_empty());
        let g = group([('b', 1), ('a', 2), ('b', 3)]);
        assert_eq!(g, [('b', vec![1, 3]), ('a', vec![2])]);
    }

    #[test]
    fn device_index_and_inside() {
        let items = [vec![DeviceId(0), DeviceId(2)], vec![DeviceId(2)]];
        let idx = device_index(3, items.iter().map(Vec::as_slice));
        assert_eq!(idx, [vec![0], vec![], vec![0, 1]]);
        assert!(device_index(0, std::iter::empty::<&[DeviceId]>()).is_empty());
        assert_eq!(inside(&idx, [0, 2], |_| true), [0, 1], "ascending, distinct");
        assert_eq!(inside(&idx, [2], |i| i == 1), [1]);
        assert!(inside(&idx, [1], |_| true).is_empty(), "an untouched device");
        assert!(inside(&idx, std::iter::empty::<usize>(), |_| true).is_empty());
    }

    #[test]
    fn union_roots_at_the_smallest_member() {
        let mut p: Vec<usize> = (0..5).collect();
        union(&mut p, 3, 1);
        union(&mut p, 4, 3);
        union(&mut p, 2, 2);
        assert_eq!([find(&mut p, 1), find(&mut p, 3), find(&mut p, 4)], [1, 1, 1]);
        assert_eq!([find(&mut p, 0), find(&mut p, 2)], [0, 2]);
    }

    #[test]
    fn diode_connection() {
        let n = DeviceKind::Nmos;
        let nl = Netlist {
            devices: vec![mk("D", n, 0, 0, 1, 1, 1_000, 500), mk("M", n, 0, 2, 1, 1, 1_000, 500)],
            nets: nets(&["g", "VSS", "d"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        assert!(diode(&hg, 0));
        assert!(!diode(&hg, 1));
    }

    #[test]
    fn every_missing_deck_key_is_incomplete() {
        let d = [dr(Some(2_000), Some(1_000), 1, 0); 2];
        let decks = [
            UnitDeck { grid_nm: 0, ..DECK },
            UnitDeck { min_w_nm: 0, ..DECK },
            UnitDeck { max_w_nm: 0, ..DECK },
            UnitDeck { min_l_nm: 0, ..DECK },
            UnitDeck { res_min_segment_nm: 0, ..DECK },
            UnitDeck::default(),
        ];
        for deck in decks {
            let e = unitize(&ids(2), &d, DeviceKind::Nmos, MatchClass::Moderate, &deck).unwrap_err();
            assert_eq!((e.kind, e.devices.len()), ("unit_deck_incomplete", 2), "{deck:?}");
        }
    }

    /// No members is no ratio, whatever the kind: an error, never a panic.
    #[test]
    fn empty_members_is_an_error_for_every_kind() {
        for kind in [DeviceKind::Nmos, DeviceKind::Resistor, DeviceKind::Capacitor, DeviceKind::Npn, DeviceKind::Diode] {
            let e = unitize(&[], &[], kind, MatchClass::Moderate, &DECK).unwrap_err();
            assert_eq!(e.kind, "non_integer_ratio", "{kind:?}");
        }
    }

    #[test]
    fn models_differ() {
        let d = [dr(Some(2_000), Some(1_000), 1, 0), dr(Some(2_000), Some(1_000), 1, 1)];
        assert_eq!(unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
    }

    #[test]
    fn fet_unknown_size() {
        for d in [[dr(None, Some(1_000), 1, 0), dr(Some(2_000), Some(1_000), 1, 0)], [dr(Some(2_000), None, 1, 0), dr(Some(2_000), Some(1_000), 1, 0)]] {
            assert_eq!(unit(2, &d, DeviceKind::Pmos, MatchClass::Moderate).unwrap_err().kind, "unknown_size");
        }
    }

    /// Hastings MOS rule 11's 1 µm floor binds only from Moderate up.
    #[test]
    fn fet_floor_follows_the_class() {
        let d = [dr(Some(500), Some(1_000), 1, 0), dr(Some(1_000), Some(1_000), 1, 0)];
        let (u, units) = unit(2, &d, DeviceKind::Nmos, MatchClass::Minimal).unwrap();
        assert_eq!((u.w_nm, u.l_nm, units), (500, 1_000, vec![(1, 1), (2, 1)]));
        assert_eq!(unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
    }

    /// The unit width is capped at `max_w`: the largest on-grid divisor below it.
    #[test]
    fn fet_unit_width_capped_at_max() {
        let d = [dr(Some(20_000), Some(1_000), 1, 0), dr(Some(40_000), Some(1_000), 1, 0)];
        let (u, units) = unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap();
        assert_eq!((u.w_nm, par(&units)), (10_000, vec![2, 4]));
    }

    /// Fingers multiply the finger width into the total.
    #[test]
    fn fet_fingers_count_into_the_total() {
        let d = [dr(Some(2_000), Some(1_000), 3, 0), dr(Some(2_000), Some(1_000), 1, 0)];
        let (u, units) = unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap();
        assert_eq!((u.w_nm, par(&units)), (2_000, vec![3, 1]));
    }

    #[test]
    fn fet_off_grid_width_has_no_unit() {
        // gcd 2003 is prime: neither 1 nor 2003 is a 5 nm multiple at or above the floor.
        let d = [dr(Some(2_003), Some(1_000), 1, 0), dr(Some(4_006), Some(1_000), 1, 0)];
        assert_eq!(unit(2, &d, DeviceKind::Nmos, MatchClass::Minimal).unwrap_err().kind, "non_integer_ratio");
    }

    #[test]
    fn fet_unequal_lengths_split_on_the_floor() {
        let d = [dr(Some(2_000), Some(1_500), 1, 0), dr(Some(2_000), Some(2_000), 1, 0)];
        // gcd 500: below the 1 µm Moderate floor, above min_l for Minimal.
        assert_eq!(unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
        let (u, units) = unit(2, &d, DeviceKind::Nmos, MatchClass::Minimal).unwrap();
        assert_eq!((u.l_nm, units), (500, vec![(1, 3), (1, 4)]));
        // Off-grid split length.
        let d = [dr(Some(2_000), Some(1_502), 1, 0), dr(Some(2_000), Some(3_004), 1, 0)];
        assert_eq!(unit(2, &d, DeviceKind::Nmos, MatchClass::Minimal).unwrap_err().kind, "non_integer_ratio");
    }

    #[test]
    fn fet_too_many_units() {
        let d = [dr(Some(10_000), Some(1_000), 1, 0), dr(Some(700_000_000), Some(1_000), 1, 0)];
        let e = unit(2, &d, DeviceKind::Nmos, MatchClass::Moderate).unwrap_err();
        assert_eq!((e.kind, e.message.as_str()), ("non_integer_ratio", "too many units"));
    }

    #[test]
    fn fet_single_member_is_one_unit() {
        let (u, units) = unit(1, &[dr(Some(4_000), Some(1_000), 1, 0)], DeviceKind::Nmos, MatchClass::Moderate).unwrap();
        assert_eq!((u.w_nm, u.l_nm, units), (4_000, 1_000, vec![(1, 1)]));
    }

    #[test]
    fn resistor_rules() {
        let r = DeviceKind::Resistor;
        let d = [dr(Some(2_000), Some(10_000), 2, 0), dr(Some(2_000), Some(30_000), 1, 0)];
        let (u, units) = unit(2, &d, r, MatchClass::Moderate).unwrap();
        assert_eq!((u.w_nm, u.l_nm, units), (2_000, 10_000, vec![(2, 1), (1, 3)]), "parallel = m, series = L / L_u");
        let widths = [dr(Some(2_000), Some(10_000), 1, 0), dr(Some(3_000), Some(10_000), 1, 0)];
        assert_eq!(unit(2, &widths, r, MatchClass::Moderate).unwrap_err().kind, "resistor_widths_differ");
        let unknown = [dr(Some(2_000), None, 1, 0), dr(Some(2_000), Some(10_000), 1, 0)];
        assert_eq!(unit(2, &unknown, r, MatchClass::Moderate).unwrap_err().kind, "unknown_size");
        let short = [dr(Some(2_000), Some(5_000), 1, 0), dr(Some(2_000), Some(10_000), 1, 0)];
        assert_eq!(unit(2, &short, r, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
    }

    #[test]
    fn capacitor_rules() {
        let c = DeviceKind::Capacitor;
        let same = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(10_000), Some(10_000), 2, 0), dr(Some(10_000), Some(10_000), 4, 0)];
        let (u, units) = unit(3, &same, c, MatchClass::Moderate).unwrap();
        assert_eq!((u.w_nm, u.l_nm, units), (10_000, 10_000, vec![(1, 1), (2, 1), (4, 1)]));
        let multiple = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(10_000), Some(30_000), 2, 0)];
        let (u, units) = unit(2, &multiple, c, MatchClass::Moderate).unwrap();
        assert_eq!((u.l_nm, units), (10_000, vec![(1, 1), (6, 1)]));
        let widths = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(20_000), Some(20_000), 1, 0)];
        assert_eq!(unit(2, &widths, c, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
        let off = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(10_000), Some(15_000), 1, 0)];
        assert_eq!(unit(2, &off, c, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio");
        let unknown = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(10_000), None, 1, 0)];
        assert_eq!(unit(2, &unknown, c, MatchClass::Moderate).unwrap_err().kind, "unknown_size");
    }

    #[test]
    fn identical_unit_kinds() {
        for k in [DeviceKind::Npn, DeviceKind::Pnp, DeviceKind::Diode] {
            let d = [dr(Some(5_000), Some(5_000), 1, 0), dr(Some(5_000), Some(5_000), 8, 0)];
            let (u, units) = unit(2, &d, k, MatchClass::Moderate).unwrap();
            assert_eq!((u.w_nm, u.l_nm, units), (5_000, 5_000, vec![(1, 1), (8, 1)]), "{k:?}");
            let differ = [dr(Some(5_000), Some(5_000), 1, 0), dr(Some(10_000), Some(10_000), 1, 0)];
            assert_eq!(unit(2, &differ, k, MatchClass::Moderate).unwrap_err().kind, "non_integer_ratio", "{k:?}");
        }
    }

    fn bridged(bridge: u16) -> PassiveSet {
        PassiveSet { devices: ids(3), rule: "split_dac", role: crate::class::SetRole::DacBank, reference: None, bridge: Some(DeviceId(bridge)) }
    }

    #[test]
    fn unitize_set_skips_the_bridge() {
        let d = [dr(Some(10_000), Some(10_000), 1, 0), dr(Some(10_000), Some(10_000), 2, 0), dr(Some(3_000), Some(7_000), 1, 0)];
        let (u, units) = unitize_set(&ids(3), &[bridged(2)], &d, DeviceKind::Capacitor, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!((u.w_nm, units), (10_000, vec![(1, 1), (2, 1), (1, 1)]));
        // The bridge first: the others keep their order.
        let order = [DeviceId(2), DeviceId(0), DeviceId(1)];
        let (_, units) = unitize_set(&order, &[bridged(2)], &d, DeviceKind::Capacitor, MatchClass::Moderate, &DECK).unwrap();
        assert_eq!(units, [(1, 1), (1, 1), (2, 1)]);
        // Without the bridge entry, the odd capacitor breaks the ratio.
        assert!(unitize_set(&ids(3), &[], &d, DeviceKind::Capacitor, MatchClass::Moderate, &DECK).is_err());
    }

    /// Only bridges: nothing to unitize, an error and no panic.
    #[test]
    fn unitize_set_of_only_a_bridge() {
        let d = [dr(Some(10_000), Some(10_000), 1, 0)];
        let e = unitize_set(&[DeviceId(0)], &[bridged(0)], &d, DeviceKind::Capacitor, MatchClass::Moderate, &DECK).unwrap_err();
        assert_eq!(e.kind, "non_integer_ratio");
    }

    fn classes_of(nl: &Netlist, class: impl Fn(usize) -> NetClass) -> Vec<NetClassification> {
        (0..nl.nets.len()).map(|n| NetClassification { net: pnr_core::ids::NetId(n as u16), class: class(n), c_budget_af: None, max_coupling_af: None }).collect()
    }

    /// Bias gate, two drains: one group; one drain: one device written twice;
    /// Signal gate without a diode: no bias relation.
    #[test]
    fn shared_bias_rules() {
        let n = DeviceKind::Nmos;
        // Nets: 0=vb 1=VSS 2=d1 3=d2.
        let nl = Netlist {
            devices: vec![mk("A", n, 0, 2, 1, 1, 2_000, 500), mk("B", n, 0, 3, 1, 1, 4_000, 500), mk("C", n, 0, 2, 1, 1, 2_000, 1_000), mk("D", n, 0, 2, 1, 1, 2_000, 1_000)],
            nets: nets(&["vb", "VSS", "d1", "d2"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let d = drawn(&nl);
        let bias = classes_of(&nl, |n| if n == 0 { NetClass::Bias } else { NetClass::Signal });
        assert_eq!(shared_bias_groups(&hg, &d, &bias), [vec![DeviceId(0), DeviceId(1)]], "C and D share one drain");
        let signal = classes_of(&nl, |_| NetClass::Signal);
        assert!(shared_bias_groups(&hg, &d, &signal).is_empty());
        assert!(shared_bias_groups(&hg, &[], &bias).is_empty(), "no devices");
    }

    /// A diode-connected member makes a group on a Signal gate, and leads it.
    #[test]
    fn shared_bias_diode_leads() {
        let n = DeviceKind::Nmos;
        // Nets: 0=g 1=VSS 2=out. Device 1 is the diode (D = G).
        let nl = Netlist { devices: vec![mk("O", n, 0, 2, 1, 1, 2_000, 500), mk("R", n, 0, 0, 1, 1, 2_000, 500)], nets: nets(&["g", "VSS", "out"]), ..Default::default() };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let signal = classes_of(&nl, |_| NetClass::Signal);
        assert_eq!(shared_bias_groups(&hg, &drawn(&nl), &signal), [vec![DeviceId(1), DeviceId(0)]]);
    }

    /// Components split by kind with `mixed_kind_set`; members in canonical
    /// order; no compound, so the origin is the default symmetry seed.
    #[test]
    fn matched_sets_split_mixed_kinds() {
        let (n, p) = (DeviceKind::Nmos, DeviceKind::Pmos);
        // Nets: 0=a 1=b 2=c 3=VSS.
        let nl = Netlist {
            devices: vec![mk("N0", n, 0, 1, 3, 3, 2_000, 1_000), mk("P1", p, 0, 2, 3, 3, 2_000, 1_000), mk("N2", n, 1, 2, 3, 3, 2_000, 1_000)],
            nets: nets(&["a", "b", "c", "VSS"]),
            ..Default::default()
        };
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let req = |a: u16, b: u16| Req { a: DeviceId(a), b: DeviceId(b), ty: ReqType::MatchBlock, source: ConstraintId(0) };
        let mut diags = Vec::new();
        let sets = matched_sets(&[req(0, 1), req(1, 2)], &[], &[], &[], &[], &[5, 0, 1], &drawn(&nl), &hg, &DECK, &mut diags);
        assert_eq!(diags.iter().map(|d| d.kind).collect::<Vec<_>>(), ["mixed_kind_set"]);
        assert_eq!(sets.len(), 1);
        let s = &sets[0];
        assert_eq!(s.members.iter().map(|m| m.device).collect::<Vec<_>>(), [DeviceId(2), DeviceId(0)], "canonical order");
        assert_eq!(s.origin, Origin::Symmetry { seed: ConstraintId(0) });
        assert_eq!((s.kind, s.class, s.reference, s.compound), (MatchKind::Current, MatchClass::Moderate, None, None));
        assert_eq!(s.family, Family::of(n).unwrap());
        assert!(s.unit.is_some());
        // ProxBlock edges join nothing.
        let prox = Req { ty: ReqType::ProxBlock, ..req(0, 2) };
        assert!(matched_sets(&[prox], &[], &[], &[], &[], &[0, 1, 2], &drawn(&nl), &hg, &DECK, &mut diags).is_empty());
    }

    /// A set that cannot be unitized keeps `(m, 1)` per member and reports why.
    #[test]
    fn matched_sets_fall_back_to_fingers() {
        let nl = mos(&[(2_000, 1_000, 1), (2_000, 1_000, 1)]);
        let hg = BipartiteHypergraph::from_netlist(&nl);
        let req = Req { a: DeviceId(0), b: DeviceId(1), ty: ReqType::MatchSym, source: ConstraintId(0) };
        let mut diags = Vec::new();
        let sets = matched_sets(&[req], &[], &[], &[], &[], &[0, 1], &drawn(&nl), &hg, &UnitDeck::default(), &mut diags);
        assert_eq!(diags.iter().map(|d| d.kind).collect::<Vec<_>>(), ["unit_deck_incomplete"]);
        assert!(sets[0].unit.is_none());
        assert_eq!(sets[0].members.iter().map(|m| (m.parallel, m.series)).collect::<Vec<_>>(), [(1, 1), (1, 1)]);
    }

    fn compound(pairs: &[(u16, u16)]) -> Compound {
        Compound {
            id: ConstraintId(0),
            axis: pnr_core::ids::AxisId(0),
            dir: analog::intent::AxisDir::V,
            kind: analog::intent::SymKind::Mirror,
            pairs: pairs.iter().map(|&(a, b)| (DeviceId(a), DeviceId(b))).collect(),
            selfs: Vec::new(),
            net_pairs: Vec::new(),
            self_nets: Vec::new(),
            set_pairs: vec![(9, 9)],
        }
    }

    fn spec(devs: &[u16]) -> MatchSpec {
        MatchSpec {
            id: ConstraintId(0),
            origin: Origin::SharedBias,
            members: devs.iter().map(|&d| Member { device: DeviceId(d), parallel: 1, series: 1, half: None }).collect(),
            reference: None,
            family: Family::of(DeviceKind::Nmos).unwrap(),
            kind: MatchKind::Current,
            class: MatchClass::Moderate,
            class_source: ClassSource::Role,
            unit: None,
            allowance: None,
            weight: None,
            style: ArrayStyle::Any,
            compound: None,
        }
    }

    #[test]
    fn set_pairs_corners() {
        // No sets: stale pairs are cleared.
        let mut cs = vec![compound(&[(0, 1)])];
        set_pairs(&mut cs, &[]);
        assert!(cs[0].set_pairs.is_empty());
        // {0, 2} maps onto {1, 3} whatever the member order; {4} is outside the pairs.
        let sets = [spec(&[2, 0]), spec(&[4]), spec(&[3, 1])];
        let mut cs = vec![compound(&[(0, 1), (2, 3)]), compound(&[])];
        set_pairs(&mut cs, &sets);
        assert_eq!(cs[0].set_pairs, [(0, 2)]);
        assert!(cs[1].set_pairs.is_empty());
        // A partial image is no pair.
        let mut cs = vec![compound(&[(0, 1)])];
        set_pairs(&mut cs, &sets);
        assert!(cs[0].set_pairs.is_empty());
    }
}
