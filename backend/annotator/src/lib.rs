//! `annotator`: netlist → recognised analog blocks → constraints.
//!
//! [`annotate`] runs the [`pattern`] catalog over the netlist, turns the
//! non-overlapping matches into blocks (one group each, plus a trailing glue
//! block for the unclaimed devices), gives each composite its declared children
//! ([`catalog::roles_of`]), and emits placement ([`emit`]), routing ([`extract`]), net-class
//! ([`classify`]) and cell-tier ([`constraints`]) constraints from them, then
//! reports conflicts ([`conflict`]).

pub mod allocate;
pub mod audit;
pub mod block;
pub mod budget;
pub mod catalog;
pub mod class;
pub mod classify;
pub mod conflict;
pub mod constraints;
pub mod emit;
pub mod evidence;
pub mod extract;
pub mod flow;
pub mod graph;
pub mod hier;
pub mod ir;
pub mod netrole;
pub mod passive;
pub mod pattern;
pub mod policy;
pub mod rings;
pub mod sets;
pub mod sidecar;
pub mod size;
pub mod substrate;
pub mod symmetry;
pub mod terms;

#[cfg(test)]
mod tests;

pub use block::{Block, BlockKind};
pub use evidence::{Evidence, OpFacts};
pub use netrole::{rail_of, AnnotationConfig, NetRole, ProcessNumbers};
pub use policy::Policy;

use pnr_core::ids::{DeviceId, NetId};
use pnr_core::Netlist;

/// Everything the annotator hands the generators, placer and router.
pub struct Problem {
    /// Recognised blocks, then the glue block (always last, empty when every
    /// device is claimed). Index = [`pnr_core::GroupId`] and the stage's symmetry
    /// [`pnr_core::ids::AxisId`]; `blocks[i].devices` is [`pnr_core::Layout::groups`]`[i]`
    /// (composites may mix polarities). The annotator emits no group-targeted
    /// rule, so an empty glue never reaches [`pnr_core::Layout::bbox`]'s panic.
    pub blocks: Vec<Block>,
    /// Cell-tier directives (unitization, dummies, guard rings), read by `cells`.
    pub constraints: analog::Constraints,
    /// Placement rule batches, each wrapped in [`analog::rule::Tagged`] with a
    /// unique id (dense from 0 across placement then routing, in emission order).
    pub placement: analog::Requirements<pnr_core::Layout>,
    /// Routing rule batches, tagged like `placement` (ids continue after it),
    /// origin always net-class.
    pub routing: analog::Requirements<pnr_core::Routes>,
    /// Per-net class + budgets, indexed by [`pnr_core::NetId`].
    pub net_classes: Vec<analog::metadata::NetClassification>,
    /// `(rule kind, missing input)` for every family left unemitted because the
    /// deck lacks a number: **unknown**, never a pass.
    pub missing: Vec<(&'static str, &'static str)>,
    /// Every device, by id, with how recognition covers it (T7).
    pub coverage: Vec<(DeviceId, Coverage)>,
    /// Extraction's contract (EXT-12); filled from EXT-13 on.
    pub intent: analog::intent::Intent,
    /// Symmetry axes the placement emits: one per compound, at least one (EXT-20).
    pub axis_count: usize,
}

/// How a device is covered: the one report that it got a constraint or why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Touched by a placement batch.
    Constrained,
    /// In a recognised block of this template whose roles emit nothing for it.
    Grouped(&'static str),
    /// In no constraint: `"do_not_identify"`, `"unknown size"` or `"no pattern"`.
    Unconstrained(&'static str),
}

/// What the netlist needs, so `missing` lists only families that would apply.
#[derive(Clone, Copy, Debug, Default)]
struct Needs {
    /// A matched leaf (DiffPair, CurrentMirror, Load, CascodePair) exists.
    matched: bool,
    /// A FET exists: gate nets an antenna rule would check.
    gate_nets: bool,
    /// A device has gate area: `classify` would budget nets (its own precondition).
    budgeted_nets: bool,
}

/// Rule families [`ProcessNumbers`] cannot instantiate, among those `needs`
/// says apply. DTI is absent from the list: a process without trenches makes
/// `DtiBand` inapplicable, not unknown.
fn missing(p: &ProcessNumbers, needs: &Needs) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    if needs.matched && p.svt_uv_per_um.is_none() && p.svt_fit.is_none() {
        out.push(("MatchedSet", "deck svt_uv_per_um — distance term unknown"));
    }
    if needs.matched && p.avt_mv_um.iter().any(Option::is_none) {
        out.push(("MatchedSet", "deck avt_n_mv_um/avt_p_mv_um"));
    }
    if needs.gate_nets && p.antenna_max_ratio.is_none() {
        out.push(("Antenna", "deck antenna ratio"));
    }
    if needs.budgeted_nets && p.gate_af_per_um2.is_none() {
        out.push(("ParasiticBudget", "deck gate_cap_af_um2"));
        out.push(("CouplingBudget", "deck gate_cap_af_um2"));
    } else if needs.budgeted_nets && p.wire_af_per_um.is_none() {
        out.push(("ParasiticBudget", "deck wire capacitance"));
    }
    out
}

/// Assemble the [`Problem`] from structure alone: [`annotate_with`] and no evidence.
///
/// # Panics
/// When the netlist has more than 65535 devices or nets: ids are `u16` (AA-35).
#[must_use]
pub fn annotate(netlist: &Netlist, cfg: &AnnotationConfig) -> Problem {
    annotate_with(netlist, cfg, &Evidence::default())
}

/// Assemble the [`Problem`], reading simulation evidence (EXT-17): testbench
/// rails and clocks join the name pre-pass, the op point sets device regions
/// and roles. Deterministic.
///
/// # Panics
/// When the netlist has more than 65535 devices or nets: ids are `u16` (AA-35).
#[must_use]
pub fn annotate_with(netlist: &Netlist, cfg: &AnnotationConfig, ev: &Evidence) -> Problem {
    for n in [netlist.devices.len(), netlist.nets.len()] {
        assert!(n <= usize::from(u16::MAX), "annotator: {n} devices/nets exceed the u16 id space (65535)");
    }
    let n = netlist.devices.len();
    let hg = pnr_core::BipartiteHypergraph::from_netlist(netlist);
    let mut models = Vec::new();
    let drawn: Vec<size::Drawn> = netlist.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
    let named = |n: &String| [&cfg.supply_nets, &cfg.ground_nets, &cfg.clock_nets].iter().any(|l| l.iter().any(|s| s.eq_ignore_ascii_case(n)));
    let user: Vec<bool> = hg.net_names.iter().map(named).collect();
    let cfg = &with_testbench(netlist, cfg, ev);
    let roles = netrole::classify_nets(&hg, cfg);

    // Recognised blocks in selection order, each composite with its primitive children.
    let canon = pattern::canonical_labels(&hg, &drawn, &models, &roles);
    let names: Vec<&str> = netlist.devices.iter().map(|d| d.name.as_str()).collect();
    let all = pattern::recognize_all(&hg, &drawn, &roles, cfg, &canon, &names);
    let mut claimed = vec![false; netlist.devices.len()];
    let mut blocks: Vec<Block> = pattern::select_disjoint(&all, &canon, &names)
        .into_iter()
        .map(|m| {
            for &d in &m.instances {
                claimed[d as usize] = true;
            }
            Block::from_match(&m)
        })
        .collect();
    // A source-degenerated diff pair no joined-source pattern sees (EXT-19 step 5).
    for (a, b) in passive::degenerated_pairs(&hg, &drawn, &roles) {
        if !claimed[a.0 as usize] && !claimed[b.0 as usize] {
            (claimed[a.0 as usize], claimed[b.0 as usize]) = (true, true);
            blocks.push(Block { kind: BlockKind::DiffPair, template: "diff_pair_with_degen", devices: vec![a, b], injected: false, sub_blocks: Vec::new(), selfs: Vec::new() });
        }
    }
    let glue = (0..netlist.devices.len() as u16).filter(|&d| !claimed[d as usize]).map(DeviceId);
    blocks.push(Block { kind: BlockKind::Glue, template: "glue", devices: glue.collect(), injected: false, sub_blocks: Vec::new(), selfs: Vec::new() });

    // A net feeding a matched device's gate is the small-signal path whose coupling
    // shows up as offset: classified Sensitive, tight budgets.
    let sensitive = device_mask(n, block::leaves(&blocks).into_iter().filter(|b| b.kind.is_sensitive()).flat_map(|b| &b.devices));
    let gates: Vec<f32> = netlist.devices.iter().map(gate_um2).collect();
    let mut net_classes = classify::classify(&hg, &roles, &sensitive, &gates, cfg.process.gate_af_per_um2, &cfg.loads);

    let fet = |k: pnr_core::DeviceKind| matches!(k, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos);
    let needs = Needs {
        matched: block::leaves(&blocks)
            .iter()
            .any(|b| matches!(b.kind, BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load | BlockKind::CascodePair)),
        gate_nets: netlist.devices.iter().any(|d| fet(d.kind)),
        budgeted_nets: gates.iter().any(|&g| g > 0.0),
    };
    let mut missing = missing(&cfg.process, &needs);
    // The matcher pairs FETs only: an unsized FET is a pair nobody could check.
    if netlist.devices.iter().zip(&drawn).any(|(d, s)| fet(d.kind) && size::unknown_size(d.kind, s)) {
        missing.push(("MatchedSet", "device W/L"));
    }
    if netlist.devices.iter().any(|d| d.kind == pnr_core::DeviceKind::Capacitor) {
        missing.push(("ParasiticBudget", "capacitor-plate nets: settling / code-error spec (ARR-03, ARR-05)"));
    }
    let mut intent = analog::intent::Intent::default();
    // Symmetry seeds: the disjoint DiffPair/Load/CascodePair leaves (never contradictory),
    // each couple and the list in canonical order, names breaking exact ties.
    let ck = |d: DeviceId| (canon[d.0 as usize], names[d.0 as usize]);
    let mut seeds: Vec<(DeviceId, DeviceId, usize)> = block::leaves(&blocks)
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b.kind, BlockKind::DiffPair | BlockKind::Load | BlockKind::CascodePair))
        .map(|(i, b)| if ck(b.devices[0]) <= ck(b.devices[1]) { (b.devices[0], b.devices[1], i) } else { (b.devices[1], b.devices[0], i) })
        .collect();
    seeds.sort_by_key(|&(a, b, _)| (ck(a), ck(b)));
    // User seeds first (EXT-26).
    let mut seeds: Vec<symmetry::Seed> = cfg.seeds.iter().copied().chain(seeds.into_iter().map(|(a, b, i)| symmetry::Seed::Devices(a, b, analog::intent::ConstraintId(i as u32)))).collect();
    // EXT-27: couples of identical instance pairs whose ports pair up, after the leaf seeds;
    // a device already seeded (leaf symmetry inside an instance) keeps its own couple.
    let inst_pairs = hier::same_template(netlist, &drawn);
    let hier_couples: Vec<(DeviceId, DeviceId)> = inst_pairs.iter().flat_map(|&(a, b)| hier::corresponding(netlist, &drawn, a, b).unwrap_or_default()).collect();
    let seeded: std::collections::HashSet<DeviceId> = seeds.iter().flat_map(|s| match *s {
        symmetry::Seed::Devices(a, b, _) => vec![a, b],
        symmetry::Seed::SelfDevice(a, _) => vec![a],
        symmetry::Seed::Nets(..) => vec![],
    }).collect();
    let mut k = 0;
    for &(a, b) in &inst_pairs {
        if hier::ports_pair(netlist, a, b) {
            for (x, y) in hier::corresponding(netlist, &drawn, a, b).unwrap_or_default() {
                if !seeded.contains(&x) && !seeded.contains(&y) {
                    seeds.push(symmetry::Seed::Devices(x, y, analog::intent::ConstraintId(HIER_SEED_ID_BASE + k)));
                    k += 1;
                }
            }
        }
    }
    let (compounds, diags) = symmetry::analyze(&hg, &drawn, &net_classes, &seeds, &canon);
    intent.compounds = compounds;
    if let (Some(dir), [c]) = (cfg.symmetry_dir, intent.compounds.as_mut_slice()) {
        c.dir = dir;
    }
    intent.diagnostics.extend(cfg.sidecar_diags.iter().cloned());
    intent.diagnostics.extend(diags);
    let shared = sets::shared_bias_groups(&hg, &drawn, &net_classes);
    // EXT-19: passive, bipolar-core and diode sets.
    let pair_of = |m: &pattern::PatternMatch| (DeviceId(m.instances[0] as u16), DeviceId(m.instances[1] as u16));
    let bjt_ratioed: Vec<(DeviceId, DeviceId)> = all.iter().filter(|m| m.template.starts_with("bjt_ratioed_pair")).map(pair_of).collect();
    let mut pairs: Vec<(DeviceId, DeviceId)> =
        block::leaves(&blocks).iter().filter(|b| b.kind == BlockKind::DiffPair).map(|b| (b.devices[0], b.devices[1])).collect();
    pairs.extend(all.iter().filter(|m| m.template.starts_with("bjt_")).map(pair_of));
    // Degeneration first: its role (the pair's) wins over a plain symmetric couple.
    let mut passive_sets = passive::degeneration(&hg, &drawn, &pairs, &mut intent.diagnostics);
    passive_sets.extend(passive::resistor_sets(&hg, &drawn, &net_classes, &intent.compounds));
    passive_sets.extend(passive::bandgap_cores(&hg, &drawn, &bjt_ratioed));
    passive_sets.extend(passive::capacitor_sets(&hg, &drawn, &net_classes, &mut intent.diagnostics));
    passive_sets.extend(passive::diode_sets(&hg, &drawn));
    let passive_groups: Vec<Vec<DeviceId>> = passive_sets.iter().map(|p| p.devices.clone()).collect();
    // EXT-27 arrays: one ProxBlock group and one horizontal, reversible order step per instance.
    let ck2 = |d: &DeviceId| (canon[d.0 as usize], d.0);
    let array_steps: Vec<Vec<Vec<DeviceId>>> = hier::arrays(netlist, &drawn, &{
        use analog::metadata::NetClass as C;
        let fixed = |c: C| matches!(c, C::Supply | C::Ground | C::Substrate | C::Clock);
        classify::bias_lines(&hg).into_iter().zip(&net_classes).map(|(b, c)| b && !fixed(c.class)).collect::<Vec<bool>>()
    })
        .into_iter()
        .map(|a| {
            a.into_iter()
                .map(|i| {
                    let mut v: Vec<DeviceId> = hier::devices(netlist, i).into_iter().map(|(d, _)| d).collect();
                    v.sort_by_key(ck2);
                    v
                })
                .collect()
        })
        .collect();
    let array_devs: Vec<Vec<DeviceId>> = array_steps.iter().map(|a| a.concat()).collect();
    let array_orders: Vec<analog::intent::Order> =
        array_steps.into_iter().map(|steps| analog::intent::Order { steps, dir: analog::intent::AxisDir::H, reversible: true, weight: 1.0 }).collect();
    let reqs = graph::requirements(&all, &intent.compounds, &shared, &passive_groups, &cfg.groups, &hier_couples, &array_devs, &netlist.device_inst, &hg, &net_classes, &canon, &cfg.policy);
    intent.tree = graph::hsmpg(netlist.devices.len(), &reqs, &canon);
    intent.sets = sets::matched_sets(&reqs, &intent.compounds, &shared, &passive_sets, &block::leaves(&blocks), &canon, &drawn, &hg, &cfg.process.unit, &mut intent.diagnostics);
    // EXT-16: kind, class and style per set; the unit floors depend on the class.
    let leaves = block::leaves(&blocks);
    let leaf_idx = sets::device_index(n, leaves.iter().map(|b| b.devices.as_slice()));
    let passive_idx = sets::device_index(n, passive_sets.iter().map(|p| p.devices.as_slice()));
    let mut roles = Vec::new();
    for s in &intent.sets {
        let has = |d: DeviceId| s.members.iter().any(|m| m.device == d);
        let pair = |b: usize| leaves[b].devices.len() == 2 && leaves[b].devices.iter().all(|&d| has(d));
        let kinds: Vec<BlockKind> = sets::inside(&leaf_idx, s.members.iter().map(|m| m.device.0 as usize), pair).into_iter().map(|b| leaves[b].kind).collect();
        roles.push(kinds);
    }
    // The sidecar entry whose devices cover a set's members (EXT-26).
    let covering = |ds: &[DeviceId], s: &analog::intent::MatchSpec| s.members.iter().all(|m| ds.contains(&m.device));
    for (s, kinds) in intent.sets.iter_mut().zip(&roles) {
        let user_kind = cfg.classes.iter().find(|c| covering(&c.0, s)).and_then(|c| c.2);
        s.kind = user_kind.unwrap_or_else(|| class::kind_of(s, kinds, netlist.devices[s.members[0].device.0 as usize].kind));
    }
    // EXT-21: allowances split from spec sensitivities over the MOS sets a sidecar
    // OffsetBudget does not cover (a user budget wins over a computed one).
    if let Some(sens) = &ev.sens {
        let (ix, ins): (Vec<usize>, Vec<allocate::SetIn>) = (0..intent.sets.len())
            .filter(|&i| !cfg.offset_budgets.iter().any(|b| covering(&b.0, &intent.sets[i])))
            .filter_map(|i| allocate::set_in(&intent.sets[i], netlist, cfg.process.avt_mv_um).map(|s| (i, s)))
            .unzip();
        let (out, diags) = allocate::allocate(&ins, sens, cfg.policy.beta_target, cfg.policy.max_eta);
        for (i, (a, w)) in ix.into_iter().zip(out) {
            (intent.sets[i].allowance, intent.sets[i].weight) = (a, w);
        }
        intent.diagnostics.extend(diags);
    }
    let input = |i: usize| roles[i].contains(&BlockKind::DiffPair);
    let mut set_roles = Vec::with_capacity(intent.sets.len());
    let input_compounds: std::collections::HashSet<u16> = (0..intent.sets.len()).filter(|&j| input(j)).filter_map(|j| intent.sets[j].compound).collect();
    for i in 0..intent.sets.len() {
        let s = &intent.sets[i];
        let role = if input(i) {
            class::SetRole::InputPair
        } else if roles[i].iter().any(|k| matches!(k, BlockKind::Load | BlockKind::CascodePair))
            && s.compound.is_some_and(|c| input_compounds.contains(&c))
        {
            class::SetRole::LoadOfPair
        } else if let Some(&p) = sets::inside(&passive_idx, s.members.iter().map(|m| m.device.0 as usize), |p| {
            passive_sets[p].devices.iter().all(|d| s.members.iter().any(|m| m.device == *d))
        })
        .first()
        {
            passive_sets[p].role
        } else if matches!(s.origin, analog::intent::Origin::Pattern { template } if template.starts_with("bjt_ratioed_pair")) {
            // A ratioed pair's ΔV_BE is a bandgap core (H09-01).
            class::SetRole::BandgapCore
        } else if roles[i].contains(&BlockKind::CurrentMirror) || s.origin == analog::intent::Origin::SharedBias {
            class::SetRole::BiasMirror
        } else {
            class::SetRole::Other
        };
        set_roles.push(role);
        let user = cfg.classes.iter().find(|c| covering(&c.0, s)).map(|c| c.1);
        let sigma = cfg.offset_budgets.iter().find(|b| covering(&b.0, s)).map(|b| b.1).or(cfg.offset_sigma_mv);
        // class_of reads only mV limits, so an allocated allowance sets a Voltage set's 6σ target.
        let alloc = s.allowance.filter(|_| s.kind == analog::intent::MatchKind::Voltage);
        let mut ctx = class::ClassCtx { user, spec_6sigma: sigma.map(|v| 6.0 * v).or(alloc.map(|a| 6.0 * a)), role, diags: &mut intent.diagnostics };
        let (mut c, mut src) = class::class_of(s, &mut ctx);
        // EXT-21: a set explaining under `minor_weight` of every spec's variance is Minimal.
        if src != analog::intent::ClassSource::User && s.weight.is_some_and(|w| w < cfg.policy.minor_weight) {
            (c, src) = (analog::intent::MatchClass::Minimal, analog::intent::ClassSource::Spec);
        }
        let source = |d: DeviceId| {
            let i = d.0 as usize;
            hg.terminals[i].iter().position(|t| t == "S" || t == "E").map(|k| hg.device_nets[i][k])
        };
        let shares_source = s.members.iter().all(|m| source(m.device).is_some() && source(m.device) == source(s.members[0].device));
        let s = &mut intent.sets[i];
        (s.class, s.class_source, s.style) = (c, src, class::style_of(c, s.kind, shares_source));
        let ids: Vec<DeviceId> = s.members.iter().map(|m| m.device).collect();
        if let Ok((u, units)) = sets::unitize_set(&ids, &passive_sets, &drawn, netlist.devices[ids[0].0 as usize].kind, c, &cfg.process.unit) {
            s.unit = Some(u);
            for (m, (p, ser)) in s.members.iter_mut().zip(units) {
                (m.parallel, m.series) = (p, ser);
            }
            if c == analog::intent::MatchClass::Exceptional && s.members.iter().any(|m| m.series > 1) {
                intent.diagnostics.push(analog::intent::Diagnostic {
                    kind: "series_units_exceptional",
                    devices: ids,
                    message: "series units carry inherent mismatch (Hastings H13-22)".into(),
                });
            }
        }
    }
    // EXT-14 step 9 (card D-d): a compound holding an Exceptional Voltage set is Perfect.
    let mut perfect = vec![false; intent.compounds.len()];
    for s in intent.sets.iter().filter(|s| s.kind == analog::intent::MatchKind::Voltage && s.class == analog::intent::MatchClass::Exceptional) {
        s.compound.and_then(|c| perfect.get_mut(c as usize)).into_iter().for_each(|p| *p = true);
    }
    for (c, &perfect) in intent.compounds.iter_mut().zip(&perfect) {
        c.kind = if perfect { analog::intent::SymKind::Perfect } else { analog::intent::SymKind::Mirror };
    }
    sets::set_pairs(&mut intent.compounds, &intent.sets);
    let load_leaf = device_mask(n, leaves.iter().copied().filter(|b| b.kind == BlockKind::Load).flat_map(|b| &b.devices));
    // EXT-18: classes that need sets, then device roles, then current-source gates.
    let load_af = classify::net_load_af(&hg, &gates, cfg.process.gate_af_per_um2, &cfg.loads);
    // AA-25: a budgeted-class net on a channel and no plate whose load is off-netlist.
    if cfg.process.gate_af_per_um2.is_some() {
        let (mut channel, mut plate) = (vec![false; load_af.len()], vec![false; load_af.len()]);
        for (d, nets) in hg.device_nets.iter().enumerate() {
            for (t, net) in hg.terminals[d].iter().zip(nets) {
                match terms::term_role(hg.kinds[d], t) {
                    terms::TermRole::Channel => channel[net.0 as usize] = true,
                    terms::TermRole::Plate => plate[net.0 as usize] = true,
                    _ => {}
                }
            }
        }
        let rail = |i: usize| matches!(net_classes[i].class, analog::metadata::NetClass::Supply | analog::metadata::NetClass::Ground | analog::metadata::NetClass::Substrate);
        if (0..load_af.len()).any(|i| channel[i] && !plate[i] && load_af[i].is_none() && !rail(i)) {
            missing.push(("ParasiticBudget", "external load of drain-only nets: sidecar Load (AA-25)"));
        }
    }
    intent.nets = classify::refine(
        &mut net_classes,
        &classify::RefineCtx {
            hg: &hg,
            matches: &all,
            sets: &intent.sets,
            set_roles: &set_roles,
            passive: &passive_sets,
            user: &user,
            ev,
            user_classes: &cfg.net_classes,
            ports: &netlist.ports,
            load_af: &load_af,
        },
    );
    intent.devices = evidence::device_facts(netlist, ev.op.as_ref(), &net_classes, &shared, &load_leaf);
    // EXT-28: user orders, then current chains a user order does not already give (either sense),
    // then the signal stages, then the EXT-27 arrays.
    intent.order.extend(cfg.order.iter().cloned());
    let norm = |steps: &[Vec<DeviceId>]| -> Vec<Vec<u16>> {
        steps.iter().map(|s| {
            let mut v: Vec<u16> = s.iter().map(|d| d.0).collect();
            v.sort_unstable();
            v
        }).collect()
    };
    let user_steps: Vec<Vec<Vec<u16>>> = cfg.order.iter().map(|o| norm(&o.steps)).collect();
    let chains = flow::current_paths(&hg, ev.op.as_ref(), &net_classes, &canon);
    let i_max = chains.iter().map(|c| c.1).fold(0.0, f64::max);
    for (steps, i) in chains {
        let (fwd, mut rev) = (norm(&steps), norm(&steps));
        rev.reverse();
        if !user_steps.iter().any(|u| *u == fwd || *u == rev) {
            let weight = if i_max > 0.0 { (i / i_max) as f32 } else { 1.0 };
            intent.order.push(analog::intent::Order { steps, dir: analog::intent::AxisDir::V, reversible: true, weight });
        }
    }
    let stages = flow::stage_order(&hg, &net_classes, &netlist.ports, &canon);
    if stages.len() >= 2 {
        intent.order.push(analog::intent::Order { steps: stages, dir: analog::intent::AxisDir::H, reversible: true, weight: 1.0 });
    }
    intent.order.extend(array_orders);
    // EXT-29: bias and structure audit over the final sets; cascode mirrors deduplicated.
    let cascodes: std::collections::BTreeSet<[u16; 4]> = all
        .iter()
        .filter(|m| matches!(m.template, "cascode_mirror" | "wide_swing_cascode_mirror" | "low_voltage_cascode_mirror"))
        .map(|m| std::array::from_fn(|k| m.instances[k] as u16))
        .collect();
    let cascodes: Vec<[DeviceId; 4]> = cascodes.into_iter().map(|c| c.map(DeviceId)).collect();
    intent.diagnostics.extend(audit::audit(&intent, netlist, &cascodes, ev.op.as_ref()));
    // The gate of every CurrentSource is a bias line. Cascode/CurrentSource-by-class
    // gates already are, so one pass is a fixpoint. A sidecar class (User) wins (EXT-26 step 4).
    for (d, f) in intent.devices.iter().enumerate() {
        let g = (f.role == analog::intent::DeviceRole::CurrentSource).then(|| pattern::pin_net(&hg, d as u32, "G")).flatten();
        let g = g.filter(|g| intent.nets[g.0 as usize].evidence != analog::intent::EvidenceLevel::User);
        if let Some(g) = g.filter(|g| matches!(net_classes[g.0 as usize].class, analog::metadata::NetClass::Signal | analog::metadata::NetClass::Sensitive)) {
            classify::set_class(&mut net_classes[g.0 as usize], analog::metadata::NetClass::Bias, load_af[g.0 as usize]);
            intent.nets[g.0 as usize].evidence = analog::intent::EvidenceLevel::OpPoint;
        }
    }
    // EXT-23: substrate tags; an aggressor is never a victim.
    (intent.aggressors, intent.victims) = substrate::tag(netlist, &net_classes, &intent.sets);
    intent.aggressors.extend(substrate::injectors(netlist, &net_classes, ev.op.as_ref(), cfg.policy.inj_series_ohm, &mut missing));
    let victim = substrate::victim_mask(n, &intent.victims);
    let aggressor = device_mask(
        n,
        intent.aggressors.iter().filter(|a| matches!(a.inject, analog::intent::Inject::Switching | analog::intent::Inject::Capacitive)).map(|a| &a.device),
    );
    let mut routing = extract::routing(
        &hg,
        &net_classes,
        &gates,
        &cfg.process,
        &mut intent,
        &set_roles,
        &cfg.policy,
        &netlist.ports,
        ev.op.as_ref(),
    );
    // EXT-25: R/C classes from the spec sensitivities (the rows themselves are the
    // library's, which owns the post-layout loop); evidence wins only where it is not Unknown.
    match (&ev.sens, cfg.process.wire_af_per_um) {
        (Some(sens), Some(af_per_um)) => {
            let (_, rc, diags) = budget::rows(sens, af_per_um / 1000.0, cfg.process.wire_ohm_per_um, &cfg.policy);
            for (n, c) in rc {
                if let Some(f) = intent.nets.get_mut(n.0 as usize) {
                    f.rc = c;
                }
            }
            intent.diagnostics.extend(diags);
        }
        (Some(_), None) => missing.push(("PerformanceBudget", "deck wire C")),
        _ => {}
    }
    intent.kelvins.extend(cfg.kelvins.iter().cloned());
    if ev.op.is_some() && ev.probe_bias {
        intent.diagnostics.push(analog::intent::Diagnostic {
            kind: "probe_bias",
            devices: vec![],
            message: "device regions from a synthesised probe bench".into(),
        });
    }
    let mut placement = emit::placement(&intent, &blocks, &cfg.groups, netlist, &drawn, &cfg.process, cfg.offset_sigma_mv, &cfg.policy);
    // Same entry of `blocks`, glue excluded: glue is no stage.
    let mut block_of = vec![usize::MAX; netlist.devices.len()];
    for (bi, b) in blocks.iter().enumerate().filter(|(_, b)| b.kind != BlockKind::Glue) {
        b.devices.iter().for_each(|d| block_of[d.0 as usize] = bi);
    }
    // Related devices sit together by design (AA-13): same block, same matched
    // set, or same symmetry compound (EXT-23 step 4: strongarm's `mn0` on the axis).
    let set_devs: Vec<Vec<DeviceId>> = intent.sets.iter().map(|s| s.members.iter().map(|m| m.device).collect()).collect();
    let compound_devs: Vec<Vec<DeviceId>> =
        intent.compounds.iter().map(|c| c.pairs.iter().flat_map(|&(a, b)| [a, b]).chain(c.selfs.iter().copied()).collect()).collect();
    let in_set = sets::device_index(n, set_devs.iter().map(Vec::as_slice));
    let in_compound = sets::device_index(n, compound_devs.iter().map(Vec::as_slice));
    let shares = |idx: &[Vec<usize>], a: usize, v: usize| idx[a].iter().any(|i| idx[v].contains(i));
    let related = |a: usize, v: usize| (block_of[a] != usize::MAX && block_of[a] == block_of[v]) || shares(&in_set, a, v) || shares(&in_compound, a, v);
    let p = &cfg.process;
    // GAP-09 (b): a sidecar GroupBlocks pull (≤ proximity_nm) on an aggressor–victim
    // pair that Isolation would push ≥ isolation_min_nm apart: the user pull wins.
    let iso_nm = emit::isolation_min_nm(p.substrate, p.epi_nm);
    let clash: Vec<(usize, usize, u32)> = cfg
        .groups
        .iter()
        .flat_map(|(e, g)| g.iter().skip(1).map(move |m| (g[0].0 as usize, m.0 as usize, *e)))
        .filter(|&(x, y, _)| ((aggressor[x] && victim[y]) || (aggressor[y] && victim[x])) && !related(x, y) && cfg.policy.proximity_nm < iso_nm)
        .collect();
    let related = |a: usize, v: usize| related(a, v) || clash.iter().any(|&(x, y, _)| (x, y) == (a, v) || (y, x) == (a, v));
    if let Some(why) = emit::isolation(&aggressor, &victim, &related, p.substrate, p.epi_nm, &mut placement) {
        missing.push(("Isolation", why));
    }
    emit::substrate_balance(&aggressor, &blocks, &block_of, &mut placement);

    // Stable ids in emission order (permutation-invariant since EXT-06). A
    // pre-tagged batch (sidecar `GroupBlocks`) keeps its origin; else a
    // placement batch whose first touched device is in a recognised block came
    // from that block's pattern; Isolation and SubstrateBalance are cross-block,
    // and the rest are net-class.
    let mut next = 0u32;
    let mut id = |origin| {
        next += 1;
        analog::intent::BatchMeta { id: analog::intent::ConstraintId(next - 1), origin }
    };
    let mut touched = vec![false; netlist.devices.len()];
    for arm in [&mut placement.hard, &mut placement.budget, &mut placement.cost] {
        *arm = std::mem::take(arm)
            .into_iter()
            .map(|inner| -> Box<dyn analog::RuleBatch<pnr_core::Layout>> {
                let mut ids = Vec::new();
                inner.touched(&mut ids);
                ids.iter().for_each(|&d| touched[d as usize] = true);
                let bi = ids.first().map_or(usize::MAX, |&d| block_of[d as usize]);
                let origin = if let Some(m) = inner.meta() {
                    m.origin
                } else if bi == usize::MAX || inner.kind().ends_with("::Isolation") || inner.kind().ends_with("::SubstrateBalance") {
                    analog::intent::Origin::NetClass
                } else {
                    analog::intent::Origin::Pattern { template: blocks[bi].template }
                };
                Box::new(analog::rule::Tagged { meta: id(origin), inner })
            })
            .collect();
    }
    for arm in [&mut routing.hard, &mut routing.budget, &mut routing.cost] {
        *arm = std::mem::take(arm)
            .into_iter()
            .map(|inner| -> Box<dyn analog::RuleBatch<pnr_core::Routes>> {
                Box::new(analog::rule::Tagged { meta: id(analog::intent::Origin::NetClass), inner })
            })
            .collect();
    }

    // GAP-09 (b) report: the user pull's id, then the Isolation batch's (left
    // out when the dropped pairs were its only ones, so no batch was emitted).
    let first_id = |f: &dyn Fn(&dyn analog::RuleBatch<pnr_core::Layout>) -> bool| {
        placement.budget.iter().chain(&placement.cost).find(|b| f(b.as_ref())).and_then(|b| b.meta()).map(|m| m.id)
    };
    let iso_id = first_id(&|b| b.kind().ends_with("::Isolation"));
    for &(x, y, e) in &clash {
        let user_id = first_id(&|b| b.meta().is_some_and(|m| m.origin == analog::intent::Origin::User { index: e }));
        let ids: Vec<_> = user_id.into_iter().chain(iso_id).collect();
        let what = format!("GroupBlocks Proximity ≤ {} nm vs Isolation ≥ {iso_nm} nm; Isolation dropped", cfg.policy.proximity_nm);
        intent.diagnostics.push(conflict::diag(&ids, vec![DeviceId(x as u16), DeviceId(y as u16)], what));
    }

    let coverage = coverage(netlist, &drawn, &touched, &block_of, &blocks, &cfg.do_not_identify);

    // REL-07: guard rings by role, from the EXT-23 tags.
    let mut constraints = constraints::assemble(netlist, &drawn, &blocks, &intent.sets);
    {
        use analog::metadata::NetClass;
        let of_class = |c: NetClass| net_classes.iter().filter(move |k| k.class == c).map(|k| k.net);
        let mut injector = vec![None; netlist.devices.len()];
        for a in &intent.aggressors {
            match a.inject {
                analog::intent::Inject::MinorityElectron => injector[a.device.0 as usize] = Some(rings::Carrier::Electrons),
                analog::intent::Inject::MinorityHole => injector[a.device.0 as usize] = Some(rings::Carrier::Holes),
                _ => {}
            }
        }
        let touched_by_aggressor = |n: NetId| netlist.devices.iter().zip(&aggressor).any(|(d, &a)| a && d.terminals.iter().any(|t| t.1 == n));
        let name = |n: NetId| netlist.nets[n.0 as usize].name.to_lowercase();
        let quiet_ring_net = match &cfg.quiet_ring_net {
            Some(q) => (0..netlist.nets.len()).map(|n| NetId(n as u16)).find(|&n| name(n) == q.to_lowercase()),
            None => of_class(NetClass::Ground)
                .filter(|&n| ["avss", "vssa", "agnd"].iter().any(|k| name(n).contains(k)))
                .find(|&n| !touched_by_aggressor(n)),
        };
        let p = &cfg.process;
        // GAP-14: the tie is a quiet supply; any other net is drawn as asked and reported.
        if cfg.tubs.iter().any(|(_, t)| !of_class(NetClass::Supply).chain(of_class(NetClass::Ground)).any(|n| n == *t)) {
            missing.push(("IsolatedTub", "tie is not a supply net"));
        }
        let (rings, notes) = rings::plan(&rings::RingInputs {
            netlist,
            aggressor: &aggressor,
            victim: &victim,
            injector: &injector,
            substrate: p.substrate,
            quiet_ring_net,
            // The Supply net at the highest known DC level; the lowest id
            // when the op point gives none.
            highest_supply: of_class(NetClass::Supply).max_by_key(|n| (intent.nets[n.0 as usize].dc_mv.map(|v| v.0), std::cmp::Reverse(n.0))),
            ground: of_class(NetClass::Ground).min_by_key(|n| n.0),
            min_ring_width_nm: p.min_ring_width_nm,
            ecgr_min_width_nm: p.ecgr_min_width_nm,
            ecgr_drawable: p.ecgr_drawable,
            hcgr_drawable: p.hcgr_drawable,
            tubs: &cfg.tubs,
            tub_drawable: p.tub_drawable,
        });
        constraints.guard_rings.extend(rings);
        missing.extend(notes);
    }

    let mut out = Problem {
        // One axis per compound (`Compound.axis`); a spare one when there is none.
        axis_count: intent.compounds.len().max(1),
        intent,
        placement,
        routing,
        coverage,
        constraints,
        net_classes,
        blocks,
        missing,
    };
    let d = conflict::check(&out, netlist);
    out.intent.diagnostics.extend(d);
    out
}

/// Base of the symmetry-seed ids given to EXT-27 instance couples: far above
/// any leaf-seed id (a leaf index), so the two ranges never collide.
const HIER_SEED_ID_BASE: u32 = u32::MAX / 2;

/// A per-device mask, `true` at every listed device.
///
/// # Panics
/// When a listed device id is `≥ n`.
fn device_mask<'a>(n: usize, devices: impl IntoIterator<Item = &'a DeviceId>) -> Vec<bool> {
    let mut mask = vec![false; n];
    for d in devices {
        mask[d.0 as usize] = true;
    }
    mask
}

/// [`Coverage`] of every device, by id: constrained when a placement batch
/// touched it, else grouped when it sits in a recognised (non-glue) block
/// (`block_of[d] != usize::MAX`), else the reason it is unconstrained, checked
/// in the order `do_not_identify`, unknown size, no pattern.
fn coverage(netlist: &Netlist, drawn: &[size::Drawn], touched: &[bool], block_of: &[usize], blocks: &[Block], do_not_identify: &std::collections::HashSet<u32>) -> Vec<(DeviceId, Coverage)> {
    (0..netlist.devices.len())
        .map(|d| {
            let c = if touched[d] {
                Coverage::Constrained
            } else if block_of[d] != usize::MAX {
                Coverage::Grouped(blocks[block_of[d]].template)
            } else if do_not_identify.contains(&(d as u32)) {
                Coverage::Unconstrained("do_not_identify")
            } else if size::unknown_size(netlist.devices[d].kind, &drawn[d]) {
                Coverage::Unconstrained("unknown size")
            } else {
                Coverage::Unconstrained("no pattern")
            };
            (DeviceId(d as u16), c)
        })
        .collect()
}

/// `cfg` plus the testbench's rails and clocks (EXT-17): with ≥ 2 distinct DC
/// source levels, the nets at the highest (lowest) level that [`rail_of`] gives
/// no role join `supply_nets` (`ground_nets`); switching-source nets join
/// `clock_nets`. Names, so [`netrole::classify_nets`] runs unchanged.
fn with_testbench(netlist: &Netlist, cfg: &AnnotationConfig, ev: &Evidence) -> AnnotationConfig {
    let mut cfg = cfg.clone();
    let name = |n: NetId| netlist.nets[n.0 as usize].name.clone();
    let (lo, hi) = ev.dc_sources.iter().map(|&(_, v)| v).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
    if lo < hi {
        for &(n, v) in ev.dc_sources.iter().filter(|(n, _)| rail_of(&netlist.nets[n.0 as usize].name).is_none()) {
            if v == hi {
                cfg.supply_nets.push(name(n));
            } else if v == lo {
                cfg.ground_nets.push(name(n));
            }
        }
    }
    cfg.clock_nets.extend(ev.switching_nets.iter().map(|&n| name(n)));
    cfg
}

/// Gate area `W_total·L·m` of a FET, µm²; `0` for anything else or when the
/// netlist omits W/L.
pub(crate) fn gate_um2(dev: &pnr_core::netlist::Device) -> f32 {
    dev.gate_area_um2() as f32
}

/// SPICE param `key` of `dev`, or `default`.
pub(crate) fn param(dev: &pnr_core::netlist::Device, key: &str, default: i64) -> i64 {
    dev.params.iter().find(|(k, _)| k == key).map_or(default, |&(_, v)| v)
}

/// Step-2 coverage: the assembly helpers and the [`annotate`] contract
/// (shape, ids, determinism, the u16 limit).
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::tests::{fet, nets, ota};
    use analog::RuleBatch;
    use pnr_core::netlist::{Device, DeviceKind};

    fn resistor(name: &str, a: u16, b: u16, params: Vec<(String, i64)>) -> Device {
        Device { name: name.into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("P".into(), NetId(a)), ("N".into(), NetId(b))], params }
    }

    #[test]
    fn missing_lists_only_applicable_families() {
        let p = ProcessNumbers::default();
        assert!(missing(&p, &Needs::default()).is_empty(), "nothing applies, nothing missing");
        let all = Needs { matched: true, gate_nets: true, budgeted_nets: true };
        assert_eq!(
            missing(&p, &all),
            [
                ("MatchedSet", "deck svt_uv_per_um — distance term unknown"),
                ("MatchedSet", "deck avt_n_mv_um/avt_p_mv_um"),
                ("Antenna", "deck antenna ratio"),
                ("ParasiticBudget", "deck gate_cap_af_um2"),
                ("CouplingBudget", "deck gate_cap_af_um2"),
            ]
        );
        let p = ProcessNumbers {
            svt_fit: Some((1.0, 1.0)),
            avt_mv_um: [Some(1.0), None],
            antenna_max_ratio: Some(400.0),
            gate_af_per_um2: Some(8_000.0),
            ..Default::default()
        };
        assert_eq!(missing(&p, &all), [("MatchedSet", "deck avt_n_mv_um/avt_p_mv_um"), ("ParasiticBudget", "deck wire capacitance")], "a fit stands in for S_VT; one polarity missing is missing");
        let p = ProcessNumbers { svt_uv_per_um: Some(1.0), avt_mv_um: [Some(1.0); 2], antenna_max_ratio: Some(1.0), gate_af_per_um2: Some(1.0), wire_af_per_um: Some(1.0), ..Default::default() };
        assert!(missing(&p, &all).is_empty());
    }

    #[test]
    fn device_mask_marks_listed_devices() {
        assert!(device_mask(0, []).is_empty());
        assert_eq!(device_mask(4, &[DeviceId(3), DeviceId(1), DeviceId(3)]), [false, true, false, true]);
    }

    #[test]
    fn coverage_reason_order() {
        let nl = pnr_core::Netlist {
            devices: vec![
                fet("M0", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000),
                fet("M1", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000),
                fet("M2", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000),
                fet("M3", DeviceKind::Nmos, 0, 1, 2, 2, 0, 1_000),
                fet("M4", DeviceKind::Nmos, 0, 1, 2, 2, 1_000, 1_000),
            ],
            nets: nets(&["a", "b", "c"]),
            ..Default::default()
        };
        let mut models = Vec::new();
        let drawn: Vec<size::Drawn> = nl.devices.iter().map(|d| size::drawn(d, &mut models)).collect();
        let blocks = [Block { kind: BlockKind::DiffPair, template: "dp", devices: vec![DeviceId(1)], injected: false, sub_blocks: Vec::new(), selfs: Vec::new() }];
        let touched = [true, true, false, false, false];
        let block_of = [0, 0, usize::MAX, usize::MAX, usize::MAX];
        let dni: std::collections::HashSet<u32> = [0, 2, 3].into_iter().collect();
        let c = coverage(&nl, &drawn, &touched, &block_of, &blocks, &dni);
        let want = [
            Coverage::Constrained,
            Coverage::Constrained,
            Coverage::Unconstrained("do_not_identify"),
            Coverage::Unconstrained("do_not_identify"),
            Coverage::Unconstrained("no pattern"),
        ];
        assert_eq!(c.iter().map(|x| x.1).collect::<Vec<_>>(), want);
        assert!(c.iter().enumerate().all(|(i, x)| x.0 == DeviceId(i as u16)));
        let c = coverage(&nl, &drawn, &[false; 5], &block_of, &blocks, &Default::default());
        assert_eq!(c[0].1, Coverage::Grouped("dp"));
        assert_eq!(c[3].1, Coverage::Unconstrained("unknown size"), "W = 0");
    }

    #[test]
    fn testbench_rails_and_clocks_join_by_name() {
        let nl = pnr_core::Netlist { nets: nets(&["hi", "lo", "mid", "VDD", "ck"]), ..Default::default() };
        let base = AnnotationConfig::default();
        let ev = Evidence {
            dc_sources: vec![(NetId(0), 1_800.0), (NetId(1), 0.0), (NetId(2), 900.0), (NetId(3), 1_800.0)],
            switching_nets: vec![NetId(4)],
            ..Default::default()
        };
        let c = with_testbench(&nl, &base, &ev);
        assert_eq!(c.supply_nets, ["hi"], "a named rail keeps its own role");
        assert_eq!(c.ground_nets, ["lo"]);
        assert_eq!(c.clock_nets, ["ck"]);
        let one_level = Evidence { dc_sources: vec![(NetId(0), 900.0), (NetId(2), 900.0)], ..Default::default() };
        let c = with_testbench(&nl, &base, &one_level);
        assert!(c.supply_nets.is_empty() && c.ground_nets.is_empty(), "one DC level names no rail");
        let c = with_testbench(&nl, &base, &Evidence::default());
        assert!(c.supply_nets.is_empty() && c.ground_nets.is_empty() && c.clock_nets.is_empty());
    }

    #[test]
    fn device_param_helpers() {
        let r = resistor("R", 0, 1, vec![("r".into(), 5), ("w".into(), 7)]);
        assert_eq!(param(&r, "w", -1), 7);
        assert_eq!(param(&r, "l", -1), -1);
        assert_eq!(gate_um2(&r), 0.0, "not a FET");
        assert_eq!(gate_um2(&fet("M", DeviceKind::Pmos, 0, 0, 0, 0, 3_000, 2_000)), 6.0);
        assert_eq!(gate_um2(&fet("M", DeviceKind::Nmos, 0, 0, 0, 0, 3_000, 0)), 0.0, "missing L");
    }

    #[test]
    fn empty_netlist_is_one_empty_glue_block() {
        let p = annotate(&pnr_core::Netlist::default(), &AnnotationConfig::default());
        assert_eq!(p.blocks.len(), 1);
        assert_eq!((p.blocks[0].kind, p.blocks[0].template), (BlockKind::Glue, "glue"));
        assert!(p.blocks[0].devices.is_empty() && p.coverage.is_empty() && p.net_classes.is_empty());
        assert_eq!(p.axis_count, 1, "a spare axis when there is no compound");
    }

    #[test]
    #[should_panic(expected = "u16 id space")]
    fn more_than_u16_devices_panics() {
        let devices = vec![resistor("R", 0, 0, vec![]); usize::from(u16::MAX) + 1];
        let nl = pnr_core::Netlist { devices, nets: nets(&["a"]), ..Default::default() };
        let _ = annotate(&nl, &AnnotationConfig::default());
    }

    /// Batch ids of three arms, in arm then emission order.
    fn ids<On>(arms: [&Vec<Box<dyn RuleBatch<On>>>; 3]) -> Vec<u32> {
        arms.iter().flat_map(|a| a.iter()).map(|b| b.meta().expect("tagged").id.0).collect()
    }

    /// Shape invariants: one coverage row per device in id order, glue last,
    /// every device in exactly one block, every non-glue block non-empty, and
    /// batch ids dense from 0 (placement, then routing).
    #[test]
    fn problem_shape_and_ids() {
        for nl in [ota(), crate::tests::three_stage()] {
            let p = annotate(&nl, &AnnotationConfig::default());
            assert_eq!(p.coverage.len(), nl.devices.len());
            assert!(p.coverage.iter().enumerate().all(|(i, c)| c.0 == DeviceId(i as u16)));
            assert_eq!(p.blocks.last().map(|b| b.kind), Some(BlockKind::Glue));
            assert!(p.blocks[..p.blocks.len() - 1].iter().all(|b| !b.devices.is_empty()), "only glue may be empty");
            let mut seen = vec![false; nl.devices.len()];
            for d in p.blocks.iter().flat_map(|b| &b.devices) {
                assert!(!std::mem::replace(&mut seen[d.0 as usize], true), "device {d:?} in two groups");
            }
            assert!(seen.iter().all(|&s| s), "every device in exactly one group");
            let place = ids([&p.placement.hard, &p.placement.budget, &p.placement.cost]);
            let route = ids([&p.routing.hard, &p.routing.budget, &p.routing.cost]);
            assert_eq!(place, (0..place.len() as u32).collect::<Vec<_>>(), "dense in emission order");
            assert_eq!(route, (place.len() as u32..(place.len() + route.len()) as u32).collect::<Vec<_>>());
            assert!(p.routing.hard.iter().chain(&p.routing.budget).chain(&p.routing.cost).all(|b| b.meta().unwrap().origin == analog::intent::Origin::NetClass));
            assert_eq!(p.axis_count, p.intent.compounds.len().max(1));
        }
    }

    /// Determinism: the same input gives the same blocks, coverage, missing
    /// list and batch kinds.
    #[test]
    fn annotate_is_deterministic() {
        let nl = crate::tests::three_stage();
        let cfg = AnnotationConfig::default();
        let shape = |p: &Problem| {
            let kinds: Vec<&str> = p.placement.hard.iter().chain(&p.placement.budget).chain(&p.placement.cost).map(|b| b.kind()).collect();
            (p.blocks.iter().map(|b| (b.template, b.devices.clone())).collect::<Vec<_>>(), p.coverage.clone(), p.missing.clone(), kinds)
        };
        assert_eq!(shape(&annotate(&nl, &cfg)), shape(&annotate(&nl, &cfg)));
    }
}
