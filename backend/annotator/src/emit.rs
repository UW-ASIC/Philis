//! Placement constraints from the extraction intent (EXT-20): one emission per
//! symmetry compound and one per matched set (`a`, `b` = a compound couple).
//!
//! | source                      | emits                                          |
//! |-----------------------------|------------------------------------------------|
//! | compound                    | one SymmetryGroup on `c.axis`: each couple that |
//! |                             | is an equal couple of one set, plus every       |
//! |                             | self; a DtiBand per couple                     |
//! | set                         | MatchedSet (slot 0 = reference), Orientation   |
//! |                             | (MOS/bipolar, ≥ 2 members), Proximity (Minimal) |
//! | Stack leaf                  | Proximity                                      |
//! | stage with a DiffPair leaf  | Proximity of each declared self to the pair    |
//!
//! A set reaches Symmetry only through a compound: a lone mirror (no compound)
//! or a ratioed couple (unequal units) is matched, not mirrored.
//!
//! One `MatchedSet` batch per set (per-batch criticality weights each set by
//! its own urgency; one merged batch regressed the OTA). Arms: `SymmetryGroup`
//! (one per compound) and `DtiBand` are hard + cost — the cost copy is the
//! gradient toward the hard set (and what prices dp's DTI branch flip).
//! `MatchedSet` (the set's gradient, thermal and LOD terms against one
//! allowance, plus coincidence when its unit counts admit a centroid row) and
//! `Proximity` (MAT-07, a distance allowance) are budget + cost; a set without
//! deck data or units reads unknown and only pulls. Placement owns the
//! systematic terms of Pelgrom; area is the cell generator's. `Orientation`
//! (MAT-05) is Axis hard (a quarter-turned partner is illegal) and Φ by class
//! ([`phi_arm`]): Exceptional hard, Moderate budget only (Φ changes by a
//! discrete flip, so a cost copy has no gradient to give; Θ prices it),
//! Minimal none.
//!
//! Matching and thermal budgets come from the netlist's areas and the deck's
//! mismatch data; without them the documented fallbacks apply.

use analog::intent::{ClassSource, Intent, MatchSpec};
use analog::matching::class::{self, phi_arm, Family, MatchClass};
use analog::matching::mismatch::{self, Budget, Coeffs};
use analog::placement::symmetry::SymmetryGroup;
use analog::placement::{DtiBand, Isolation, MatchedSet, OrientCheck, OrientationSet, Proximity, SubstrateBalance, Symmetry};
use analog::Requirements;
use pnr_core::ids::{BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{DeviceKind, Netlist};

use crate::block::{leaves, Block, BlockKind};
use crate::size::Drawn;
use crate::ProcessNumbers;

/// A set's budget: an allocated MOS allowance as itself (C13), else MAT-08's rule, with the
/// class limit only for a User/Spec class.
fn set_budget(s: &MatchSpec, offset_sigma_mv: Option<f32>) -> Budget {
    match s.allowance {
        // EXT-21 allocates MOS sets only, in mV ΔV_T: the ledger converts it to % for a
        // Current set (`MatchedSet::budget_in`); any other family's ledger would misread it.
        Some(a) if s.family == Family::Mos => Budget::Allowance(a),
        _ => {
            let limit = (s.class_source != ClassSource::Role).then(|| class::limit(s.family, s.kind, s.class)).flatten();
            mismatch::choose(offset_sigma_mv, None, limit, s.kind)
        }
    }
}

/// `d`'s entry of a deck `[nmos, pmos]` pair; `None` for a non-FET or a
/// missing entry.
pub(crate) fn by_polarity(nl: &Netlist, d: DeviceId, v: [Option<f32>; 2]) -> Option<f32> {
    match nl.devices[d.0 as usize].kind {
        DeviceKind::Nmos => v[0],
        DeviceKind::Pmos => v[1],
        _ => None,
    }
}

/// `S_VT` at `d`'s gate length: the deck's S(L) fit when it has one and `d`
/// a length, else its single `svt_uv_per_um`.
fn svt(nl: &Netlist, p: &ProcessNumbers, d: DeviceId) -> Option<f32> {
    let l = crate::param(&nl.devices[d.0 as usize], "l", 0);
    match p.svt_fit {
        Some((a, b)) if l > 0 => Some(mismatch::svt_of_l(a, b, l as f32 / 1000.0)),
        _ => p.svt_uv_per_um,
    }
}

/// Deck coefficients of a set whose slot 0 is `d`. R/C sets stay unknown:
/// their `k_A` is per recipe (FLOW-06) and has no [`ProcessNumbers`] slot.
fn coeffs(nl: &Netlist, p: &ProcessNumbers, d: DeviceId, family: Family) -> Coeffs {
    match family {
        Family::Mos => Coeffs {
            avt_mv_um: by_polarity(nl, d, p.avt_mv_um),
            svt_uv_per_um: svt(nl, p, d),
            kvth0_mv_um: by_polarity(nl, d, p.lod_kvth0_mv_um),
            tc_uv_per_k: by_polarity(nl, d, p.vt_tc_uv_per_k),
            abeta_pct_um: by_polarity(nl, d, p.abeta_pct_um),
            mobility_exp: by_polarity(nl, d, [Some(1.7), Some(1.5)]),
            die_temp_k: p.die_temp_k,
            ..Coeffs::default()
        },
        Family::Bipolar | Family::Diode => {
            Coeffs { ka_pct_um: p.bjt_ka_pct_um, vbe_tc_uv_per_k: p.vbe_tc_uv_per_k, die_temp_k: p.die_temp_k, ..Coeffs::default() }
        }
        Family::Resistor | Family::Capacitor => Coeffs::default(),
    }
}

/// Netlist area of `d`, µm²: a FET's gate area, else drawn `w·l·fingers`
/// (0 when either is unknown).
fn area_um2(nl: &Netlist, drawn: &[Drawn], d: DeviceId) -> f32 {
    let dev = &nl.devices[d.0 as usize];
    if matches!(dev.kind, DeviceKind::Nmos | DeviceKind::Pmos) {
        return crate::gate_um2(dev);
    }
    let s = &drawn[d.0 as usize];
    match (s.w_finger_nm, s.l_nm) {
        (Some(w), Some(l)) => (w as f64 * l as f64 * f64::from(s.fingers) * 1e-6) as f32,
        _ => 0.0,
    }
}

/// Build the placement [`Requirements`] from the intent's compounds and sets
/// plus the blocks' Stack leaves and declared selfs.
///
/// A `MatchedSet` is priced against its allowance when the deck carries its
/// family's mismatch constant; the terms whose coefficient is missing spend
/// nothing. The budget is [`set_budget`]: `offset_sigma_mv` (1σ input-referred
/// offset a pair may spend) when given, else [`mismatch::GRADIENT_SHARE`]`·σ_rand`.
#[must_use]
pub fn placement(
    intent: &Intent,
    blocks: &[Block],
    user_groups: &[(u32, Vec<DeviceId>)],
    nl: &Netlist,
    drawn: &[Drawn],
    p: &ProcessNumbers,
    offset_sigma_mv: Option<f32>,
    policy: &crate::policy::Policy,
) -> Requirements<Layout> {
    let mut r = Requirements::<Layout>::default();
    let mut dti = Vec::new();
    let td = Target::Device;
    let prox = |a: DeviceId, b: DeviceId| Proximity { a: td(a), b: td(b), max_distance_nm: policy.proximity_nm };

    // Symmetry per compound (MAT-06; Lampaert 1999 §4.6–4.7 symmetry groups):
    // only an equal couple of one set mirrors (a ratioed couple is a centroid
    // array, not a mirror image). Equal = the same unit counts; a set that did
    // not unitize (finger counts, not units) compares drawn geometry instead.
    let geom = |d: DeviceId| {
        let g = &drawn[d.0 as usize];
        (g.w_finger_nm, g.l_nm, g.fingers, g.model)
    };
    // (set, member) of each device: sets are disjoint (EXT-15 components).
    let mut set_of: Vec<Option<(usize, usize)>> = vec![None; drawn.len()];
    for (i, s) in intent.sets.iter().enumerate() {
        for (j, m) in s.members.iter().enumerate() {
            set_of[m.device.0 as usize] = Some((i, j));
        }
    }
    let equal = |a: DeviceId, b: DeviceId| match (set_of[a.0 as usize], set_of[b.0 as usize]) {
        (Some((sa, ia)), Some((sb, ib))) if sa == sb => {
            let s = &intent.sets[sa];
            let m = |i: usize| (s.members[i].parallel, s.members[i].series);
            if s.unit.is_some() { m(ia) == m(ib) } else { geom(a) == geom(b) }
        }
        _ => false,
    };
    for c in &intent.compounds {
        let mut syms: Vec<Symmetry> =
            c.pairs.iter().filter(|&&(a, b)| equal(a, b)).map(|&(a, b)| Symmetry { a: td(a), b: td(b), axis: c.axis }).collect();
        syms.extend(c.selfs.iter().map(|&d| Symmetry { a: td(d), b: td(d), axis: c.axis }));
        if !syms.is_empty() {
            r.cost.push(Box::new(SymmetryGroup(syms.clone())));
            r.hard.push(Box::new(SymmetryGroup(syms)));
        }
        // Mirrored devices share one trench. Ids are dense in emission order,
        // which is deterministic, so a branch never transfers between couples.
        if let Some((s_max_nm, d_dti_nm)) = p.dti {
            for &(a, b) in &c.pairs {
                dti.push(DtiBand { a: td(a), b: td(b), s_max_nm, d_dti_nm, branch: BranchId(dti.len() as u16), seed_isolate: false });
            }
        }
    }

    for s in &intent.sets {
        let mut members: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if let Some(i) = s.reference {
            members.swap(0, i);
        }
        let areas = members.iter().map(|&d| area_um2(nl, drawn, d)).collect();
        let tol_nm = p.lattice_nm.max(1) as f32 / 2.0;
        let set = MatchedSet::for_family(members.clone(), s.family, s.kind, s.class, coeffs(nl, p, members[0], s.family), set_budget(s, offset_sigma_mv), areas, tol_nm);
        r.budget.push(Box::new(set.clone()));
        r.cost.push(Box::new(set));
        if matches!(s.family, Family::Mos | Family::Bipolar) && members.len() >= 2 {
            let orient = |check| OrientationSet { members: members.clone(), check, cell_of: Vec::new() };
            r.hard.push(Box::new(orient(OrientCheck::Axis)));
            // ponytail: resistor PhiZero is MAT-12 step 3.
            match phi_arm(s.class) {
                Some(true) => r.hard.push(Box::new(orient(OrientCheck::Phi))),
                Some(false) => r.budget.push(Box::new(orient(OrientCheck::Phi))),
                None => {}
            }
        }
        if s.class == MatchClass::Minimal && members.len() >= 2 {
            let pull: Vec<Proximity> = members[1..].iter().map(|&m| prox(members[0], m)).collect();
            r.budget.push(Box::new(pull.clone()));
            r.cost.push(Box::new(pull));
        }
    }

    for l in leaves(blocks).iter().filter(|l| l.kind == BlockKind::Stack && l.devices.len() == 2) {
        let pull = vec![prox(l.devices[0], l.devices[1])];
        r.budget.push(Box::new(pull.clone()));
        r.cost.push(Box::new(pull));
    }
    // A differential stage pulls each declared self (tail, shared bias) to its input pair.
    for stage in blocks {
        let Some(dp) = leaves(std::slice::from_ref(stage)).into_iter().find(|l| l.kind == BlockKind::DiffPair && l.devices.len() == 2) else { continue };
        for &d in &stage.selfs {
            let tail = vec![prox(d, dp.devices[0]), prox(d, dp.devices[1])];
            r.budget.push(Box::new(tail.clone()));
            r.cost.push(Box::new(tail));
        }
    }

    // Sidecar `GroupBlocks` (EXT-26): each member pulled to the first, tagged
    // with its entry (the annotator keeps a pre-set origin, renumbering the id).
    for (index, g) in user_groups.iter().filter(|g| g.1.len() >= 2) {
        let pull: Vec<Proximity> = g[1..].iter().map(|&m| prox(g[0], m)).collect();
        let meta = analog::intent::BatchMeta { id: analog::intent::ConstraintId(0), origin: analog::intent::Origin::User { index: *index } };
        r.budget.push(Box::new(analog::rule::Tagged { meta, inner: Box::new(pull.clone()) }));
        r.cost.push(Box::new(analog::rule::Tagged { meta, inner: Box::new(pull) }));
    }
    if !dti.is_empty() {
        r.cost.push(Box::new(dti.clone()));
        r.hard.push(Box::new(dti));
    }
    r
}

/// Isolation saturates beyond this multiple of the epi thickness (Charbon et
/// al. 2001 ch.8, PDF p.127: 2.5–5×; Su et al. 4×): farther buys nothing.
const ISOLATION_EPI_MULTIPLE: i32 = 4;
/// ponytail: nominal epi when the distance is uncalibrated, so the pull still
/// acts; the check then reads unknown. The decks' `p_epi_thickness` is not it:
/// a guard-ring depth default (3000 nm in all four decks, finfet included),
/// and sky130 is bulk p-substrate, not epi on p+, where Charbon's plateau does
/// not hold (isolation keeps improving with distance; SUB-32).
const NOMINAL_EPI_NM: i32 = 2_500;

/// The epi thickness `isolation` scales, or the `missing` input that leaves it unknown.
fn calibration(kind: pnr_core::SubstrateKind, epi_nm: Option<i32>) -> Result<i32, &'static str> {
    use pnr_core::SubstrateKind;
    match (kind, epi_nm) {
        (SubstrateKind::EpiOnLowRes, Some(epi)) => Ok(epi),
        (SubstrateKind::EpiOnLowRes, None) => Err("deck epi_thickness_nm"),
        (SubstrateKind::Bulk, _) => Err("bulk substrate: no plateau distance"),
        (SubstrateKind::Unknown, _) => Err("substrate kind unknown"),
    }
}

/// The edge-to-edge distance [`isolation`] asks: `4·t_epi`, at
/// [`NOMINAL_EPI_NM`] when uncalibrated.
#[must_use]
pub fn isolation_min_nm(kind: pnr_core::SubstrateKind, epi_nm: Option<i32>) -> i32 {
    ISOLATION_EPI_MULTIPLE * calibration(kind, epi_nm).unwrap_or(NOMINAL_EPI_NM)
}

/// Substrate isolation (ENV-04; Charbon 2001 ch.2 injection → propagation →
/// reception): every EXT-23 `aggressor` (Switching or Capacitive) is kept
/// `ISOLATION_EPI_MULTIPLE·t_epi` edge-to-edge from every `victim`, except
/// where `related(aggressor, victim)`: a stage's own clocked tail sits by its
/// pair by design (AA-13), so isolating them contradicts the stage's Proximity.
///
/// Budget + cost only on `EpiOnLowRes` with `epi_nm` known (the plateau
/// distance). Otherwise a cost-only pull at [`NOMINAL_EPI_NM`], and the
/// returned `Some(why)` is the `missing` input that leaves it unknown: bulk has
/// no plateau distance. `None` when calibrated or nothing was emitted.
pub fn isolation(
    aggressor: &[bool],
    victim: &[bool],
    related: &dyn Fn(usize, usize) -> bool,
    kind: pnr_core::SubstrateKind,
    epi_nm: Option<i32>,
    r: &mut Requirements<Layout>,
) -> Option<&'static str> {
    let n = aggressor.len();
    let calibrated = calibration(kind, epi_nm);
    let min_distance_nm = isolation_min_nm(kind, epi_nm);
    let dev = |d: usize| Target::Device(DeviceId(d as u16));
    let rules: Vec<Isolation> = (0..n)
        .filter(|&a| aggressor[a])
        .flat_map(|a| {
            (0..n).filter(move |&v| victim[v] && !related(a, v)).map(move |v| Isolation { a: dev(a), b: dev(v), min_distance_nm })
        })
        .collect();
    if rules.is_empty() {
        return None;
    }
    if calibrated.is_ok() {
        r.budget.push(Box::new(rules.clone()));
    }
    r.cost.push(Box::new(rules));
    calibrated.err()
}

/// Substrate balance (REL-15, Charbon §8.3.1): every EXT-23 `aggressor` outside
/// a two-device DiffPair's stage (`block_of`, entries of `blocks`) pulled onto
/// the pair's bisector. Cost only: the source states no threshold.
pub fn substrate_balance(aggressor: &[bool], blocks: &[Block], block_of: &[usize], r: &mut Requirements<Layout>) {
    let dev = |d: usize| Target::Device(DeviceId(d as u16));
    let rules: Vec<SubstrateBalance> = leaves(blocks)
        .iter()
        .filter(|l| l.kind == BlockKind::DiffPair && l.devices.len() == 2)
        .flat_map(|l| {
            let (a, b) = (l.devices[0].0 as usize, l.devices[1].0 as usize);
            (0..aggressor.len())
                .filter(move |&g| aggressor[g] && block_of[g] != block_of[a])
                .map(move |g| SubstrateBalance { aggressor: dev(g), a: dev(a), b: dev(b) })
        })
        .collect();
    if !rules.is_empty() {
        r.cost.push(Box::new(rules));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::matching::mismatch::MatchKind;

    /// StrongARM-like stage: input pair `mn1`/`mn2` on a tail node, `mn0` the
    /// tail on `clk`, PMOS loads on `clk` (the StrongARM's precharge pair); plus `XS`, a lone clocked
    /// switch outside every block. Nets: 0=outp 1=inp 2=tail 3=VSS 4=outn
    /// 5=inn 6=clk 7=VDD 8=sw 9=sw2.
    fn strongarm_like() -> (Netlist, crate::AnnotationConfig) {
        use crate::tests::{fet, nets};
        let nl = Netlist {
            devices: vec![
                fet("mn1", DeviceKind::Nmos, 1, 0, 2, 3, 10_000, 1_000),
                fet("mn2", DeviceKind::Nmos, 5, 4, 2, 3, 10_000, 1_000),
                fet("mp3", DeviceKind::Pmos, 6, 0, 7, 7, 20_000, 1_000),
                fet("mp4", DeviceKind::Pmos, 6, 4, 7, 7, 20_000, 1_000),
                fet("mn0", DeviceKind::Nmos, 6, 2, 3, 3, 40_000, 1_000),
                fet("XS", DeviceKind::Nmos, 6, 8, 9, 3, 1_000, 150),
            ],
            nets: nets(&["outp", "inp", "tail", "VSS", "outn", "inn", "clk", "VDD", "sw", "sw2"]),
            ..Default::default()
        };
        let cfg = crate::AnnotationConfig {
            supply_nets: vec!["VDD".into()],
            ground_nets: vec!["VSS".into()],
            clock_nets: vec!["clk".into()],
            ..crate::AnnotationConfig::default()
        };
        (nl, cfg)
    }

    type Arm = Vec<Box<dyn analog::RuleBatch<Layout>>>;

    /// Device id pairs of every `Isolation` batch in `arm`.
    fn isolated(arm: &Arm) -> Vec<(u32, u32)> {
        let mut ids = Vec::new();
        arm.iter().filter(|b| b.kind().ends_with("::Isolation")).for_each(|b| b.touched(&mut ids));
        ids.chunks(2).map(|p| (p[0], p[1])).collect()
    }

    /// Point devices all at the origin except `XS` (id 5) at `x`: every edge
    /// gap from `XS` is `x`.
    fn xs_at(x: i32) -> Layout {
        let n = 6;
        Layout {
            x: (0..n).map(|d| if d == 5 { x } else { 0 }).collect(),
            y: vec![0; n],
            hw: vec![0; n],
            hh: vec![0; n],
            axis: vec![0; 8],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    /// The smallest `XS` distance (nm, 1 nm resolution) at which `arm`'s
    /// Isolation batches are all satisfied.
    fn isolation_distance(arm: &Arm) -> i32 {
        let ok = |x| arm.iter().filter(|b| b.kind().ends_with("::Isolation")).all(|b| b.violations(&xs_at(x)) == 0);
        let (mut lo, mut hi) = (0, 1_000_000);
        assert!(ok(hi) && !ok(lo));
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if ok(mid) { hi = mid } else { lo = mid }
        }
        hi
    }

    /// MAT-07 / EXT-20: each MOS/bipolar set of ≥ 2 keeps Axis hard; Φ is a
    /// budget for a Moderate set and absent for a Minimal one.
    #[test]
    fn default_class_is_moderate_mos() {
        let nl = crate::tests::ota();
        let cfg = crate::AnnotationConfig::default();
        let p = crate::annotate(&nl, &cfg);
        let r = &p.placement;
        let count = |arm: &Arm, k: &str| arm.iter().filter(|b| b.kind() == k).count();
        let oriented: Vec<_> = p.intent.sets.iter().filter(|s| matches!(s.family, Family::Mos | Family::Bipolar) && s.members.len() >= 2).collect();
        assert!(!oriented.is_empty());
        assert_eq!(count(&r.hard, "Orientation"), oriented.len(), "Axis hard");
        assert_eq!(count(&r.budget, "Orientation"), oriented.iter().filter(|s| s.class == MatchClass::Moderate).count(), "Phi budget");
    }

    fn spec(kind: MatchKind, source: ClassSource, allowance: Option<f32>) -> MatchSpec {
        MatchSpec {
            id: analog::intent::ConstraintId(0),
            origin: analog::intent::Origin::NetClass,
            members: Vec::new(),
            reference: None,
            family: Family::Mos,
            kind,
            class: MatchClass::Moderate,
            class_source: source,
            unit: None,
            allowance,
            weight: None,
            style: analog::intent::ArrayStyle::Any,
            compound: None,
        }
    }

    /// MAT-08 + C13: the class limit only for an explicit class; an allocated
    /// Voltage allowance is the budget itself.
    #[test]
    fn set_budget_follows_source() {
        use MatchKind::{Current, Voltage};
        let eta = Budget::Eta(mismatch::GRADIENT_SHARE);
        assert_eq!(set_budget(&spec(Voltage, ClassSource::Role, None), None), eta);
        assert_eq!(set_budget(&spec(Voltage, ClassSource::User, None), None), Budget::Sigma1Mv(0.5));
        assert_eq!(set_budget(&spec(Voltage, ClassSource::Role, None), Some(0.4)), Budget::Sigma1Mv(0.4));
        assert_eq!(set_budget(&spec(Voltage, ClassSource::Role, Some(0.4)), None), Budget::Allowance(0.4));
        assert_eq!(set_budget(&spec(Current, ClassSource::Role, Some(0.4)), None), Budget::Allowance(0.4));
        let mut r = spec(Current, ClassSource::Role, Some(0.4));
        r.family = Family::Resistor;
        assert_eq!(set_budget(&r, None), eta);
    }

    /// C13: an allocated allowance reaches the ledger unchanged.
    #[test]
    fn allocated_allowance_round_trips() {
        let nl = crate::tests::ota();
        let mut cfg = crate::AnnotationConfig::default();
        cfg.process.avt_mv_um = [Some(5.0), Some(6.0)];
        let mut p = crate::annotate(&nl, &cfg);
        let i = p.intent.sets.iter().position(|s| s.kind == MatchKind::Voltage).expect("the input pair");
        p.intent.sets[i].allowance = Some(0.4);
        let mut models = Vec::new();
        let drawn: Vec<Drawn> = nl.devices.iter().map(|d| crate::size::drawn(d, &mut models)).collect();
        let r = placement(&p.intent, &p.blocks, &[], &nl, &drawn, &cfg.process, None, &cfg.policy);
        let set = r.budget.iter().filter(|b| b.kind() == "MatchedSet").nth(i).expect("set i's batch");
        let l = Layout {
            x: vec![0, 1_000_000],
            y: vec![0; 2],
            hw: vec![0; 2],
            hh: vec![0; 2],
            axis: vec![0; 8],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        };
        let mut rows = Vec::new();
        set.ledger_rows(&l, &mut rows);
        assert_eq!(rows[0].allowance, 0.4);
        assert!(rows[0].sigma_rand > 0.0, "{}", rows[0].sigma_rand);
    }

    /// REL C3 / AA-13: the stage's clocked tail sits by its pair (Proximity);
    /// isolating it from the pair would contradict that. A clocked device
    /// outside the stage is still isolated from the pair.
    #[test]
    fn a_clocked_tail_is_not_isolated_from_its_own_pair() {
        let (nl, cfg) = strongarm_like();
        let p = crate::annotate(&nl, &cfg);
        let (mn1, mn2, mn0, xs) = (0u32, 1, 4, 5);
        let block = |d: u32| p.blocks.iter().position(|b| b.kind != BlockKind::Glue && b.devices.contains(&DeviceId(d as u16)));
        assert!(block(mn0).is_some() && block(mn0) == block(mn1), "the tail is in the pair's stage");
        let iso = isolated(&p.placement.cost);
        assert!(!iso.iter().any(|&(a, v)| a == mn0 && (v == mn1 || v == mn2)), "tail isolated from its pair: {iso:?}");
        assert!(iso.contains(&(xs, mn1)) && iso.contains(&(xs, mn2)), "an outside aggressor still is: {iso:?}");
    }

    /// GAP-09 (b): a sidecar group holding aggressor `XS` and victim `mn1`
    /// pulls them ≤ 5 µm while Isolation pushes ≥ 10 µm: the user pull wins,
    /// the Isolation pair is dropped and both ids are reported.
    #[test]
    fn proximity_below_isolation_is_a_conflict() {
        let (nl, mut cfg) = strongarm_like();
        let (mn1, mn2, xs) = (0u32, 1, 5);
        cfg.groups = vec![(0, vec![DeviceId(xs as u16), DeviceId(mn1 as u16)])];
        let p = crate::annotate(&nl, &cfg);
        let iso = isolated(&p.placement.cost);
        assert!(!iso.contains(&(xs, mn1)) && !iso.contains(&(mn1, xs)), "{iso:?}");
        assert!(iso.contains(&(xs, mn2)), "{iso:?}");
        let id = |f: &dyn Fn(&dyn analog::RuleBatch<Layout>) -> bool| {
            p.placement.budget.iter().chain(&p.placement.cost).find(|b| f(b.as_ref())).and_then(|b| b.meta()).map(|m| m.id.0).unwrap()
        };
        let user_id = id(&|b| b.meta().is_some_and(|m| m.origin == analog::intent::Origin::User { index: 0 }));
        let iso_id = id(&|b| b.kind().ends_with("::Isolation"));
        let c: Vec<_> = p.intent.diagnostics.iter().filter(|d| d.kind == "conflict").collect();
        assert_eq!(c.len(), 1, "{:?}", p.intent.diagnostics);
        let mut devs: Vec<u16> = c[0].devices.iter().map(|d| d.0).collect();
        devs.sort_unstable();
        assert_eq!(devs, [mn1 as u16, xs as u16]);
        assert!(c[0].message.starts_with(&format!("ids {user_id},{iso_id}:")), "{}", c[0].message);
    }

    /// REL-15: the stage's own clocked tail is no imbalance source for its
    /// pair; the lone `XS` gets one rule per pair, tagged net-class.
    #[test]
    fn substrate_balance_skips_the_aggressors_own_block() {
        let (nl, cfg) = strongarm_like();
        let p = crate::annotate(&nl, &cfg);
        let (mn1, mn2, mn0, xs) = (0u32, 1, 4, 5);
        let batches: Vec<_> = p.placement.cost.iter().filter(|b| b.kind().ends_with("::SubstrateBalance")).collect();
        assert_eq!(batches.len(), 1);
        assert!(batches[0].meta().is_some_and(|m| m.origin == analog::intent::Origin::NetClass));
        let mut ids = Vec::new();
        batches[0].touched(&mut ids);
        let triples: Vec<&[u32]> = ids.chunks(3).collect();
        assert!(!triples.iter().any(|t| t[0] == mn0), "own stage: {triples:?}");
        let pairs = leaves(&p.blocks).iter().filter(|l| l.kind == BlockKind::DiffPair && l.devices.len() == 2).count();
        assert!(pairs >= 1);
        assert_eq!(triples.iter().filter(|t| t[0] == xs).count(), pairs, "{triples:?}");
        assert!(triples.contains(&&[xs, mn1, mn2][..]) || triples.contains(&&[xs, mn2, mn1][..]), "{triples:?}");
        assert!(p.placement.budget.iter().chain(&p.placement.hard).all(|b| !b.kind().ends_with("::SubstrateBalance")), "cost only");
    }

    /// REL C3: epi on p+ saturates at 4·t_epi (Su), a budget the search pays.
    #[test]
    fn epi_decks_get_a_plateau_budget() {
        let (nl, mut cfg) = strongarm_like();
        cfg.process.substrate = pnr_core::SubstrateKind::EpiOnLowRes;
        cfg.process.epi_nm = Some(10_000);
        let p = crate::annotate(&nl, &cfg);
        assert!(!isolated(&p.placement.budget).is_empty(), "in the budget arm");
        assert_eq!(isolation_distance(&p.placement.budget), 40_000, "min_distance_nm");
        assert!(!p.missing.iter().any(|m| m.0 == "Isolation"), "{:?}", p.missing);
    }

    /// Bulk (sky130's `substrate_kind`) has no plateau distance: a cost-only
    /// pull at 4·2500 nm, reported unknown, whatever epi the deck states.
    #[test]
    fn bulk_is_cost_only_and_unknown() {
        let (nl, mut cfg) = strongarm_like();
        assert_eq!(pnr_core::SubstrateKind::from_key(Some("epi_on_pplus")), pnr_core::SubstrateKind::EpiOnLowRes);
        cfg.process.substrate = pnr_core::SubstrateKind::from_key(Some("bulk"));
        cfg.process.epi_nm = Some(10_000);
        let p = crate::annotate(&nl, &cfg);
        assert!(isolated(&p.placement.budget).is_empty(), "not a budget");
        assert!(!isolated(&p.placement.cost).is_empty(), "a pull");
        assert_eq!(isolation_distance(&p.placement.cost), 4 * NOMINAL_EPI_NM);
        assert!(p.missing.contains(&("Isolation", "bulk substrate: no plateau distance")), "{:?}", p.missing);

        cfg.process.substrate = pnr_core::SubstrateKind::Unknown;
        let p = crate::annotate(&nl, &cfg);
        assert!(p.missing.contains(&("Isolation", "substrate kind unknown")));
        assert!(isolated(&p.placement.budget).is_empty());
    }
}
