//! Netlist comparison via probabilistic partition refinement (netgen-style).
//!
//! Both extracted and reference graphs are loaded into a shared class space.
//! Each refinement round computes hash values for devices and nets using
//! XOR+SUM of random magic numbers, then fractures classes where hashes differ.
//! After convergence, automorphisms are broken by forcing unique IDs on one
//! ambiguous pair at a time.
//!
//! Two-terminal devices (R, C, diode) are full graph participants — not count-only.

use super::types::*;
use crate::schema::PropertyTolerance;
use std::collections::{BTreeMap, HashMap, HashSet};

// --- public helpers used by extract.rs reduce_netlist ---

pub fn kind_tag(k: &DeviceKind) -> u32 {
    match k { DeviceKind::Nmos => 0, DeviceKind::Pmos => 1, DeviceKind::Npn | DeviceKind::Pnp => 0 }
}
pub fn flavor_tag(f: &DeviceFlavor) -> u32 {
    match f { DeviceFlavor::Standard => 0, DeviceFlavor::Lvt => 1, DeviceFlavor::Hvt => 2 }
}
#[allow(dead_code)]
fn kind_name(tag: u32) -> &'static str {
    if tag == 0 { "Nmos" } else { "Pmos" }
}
#[allow(dead_code)]
fn flavor_name(tag: u32) -> &'static str {
    match tag { 1 => "Lvt", 2 => "Hvt", _ => "Std" }
}

fn bjt_kind_tag(k: &DeviceKind) -> u32 {
    match k { DeviceKind::Npn => 50, DeviceKind::Pnp => 51, _ => 50 }
}

fn two_term_kind_tag(k: &TwoTerminalKind) -> u32 {
    match k { TwoTerminalKind::Resistor => 0, TwoTerminalKind::Diode => 1, TwoTerminalKind::Capacitor => 2 }
}

#[allow(dead_code)]
fn two_term_kind_label(tag: u32) -> &'static str {
    match tag { 0 => "Resistor", 1 => "Diode", _ => "Capacitor" }
}

// --- PRNG (xorshift64, deterministic) ---

struct Rng { state: u64 }
impl Rng {
    fn new(seed: u64) -> Self { Rng { state: seed } }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

// --- graph representation ---

const MAX_PINS: usize = 4;

// Pin roles — permutable pins share the same role index so they get the
// same pin_magic value, making the hash invariant to their ordering.
const ROLE_GATE: u8 = 0;
const ROLE_SD: u8 = 1;       // source+drain share this when non-strict
const ROLE_SOURCE: u8 = 1;   // strict: source gets its own
const ROLE_DRAIN: u8 = 2;    // strict: drain distinct from source
#[allow(dead_code)]
const ROLE_BODY: u8 = 3;
const ROLE_TERM: u8 = 4;     // symmetric two-terminal (R, C): pins permutable
const ROLE_COLLECTOR: u8 = 5; // BJT collector
const ROLE_BASE: u8 = 6;     // BJT base
const ROLE_EMITTER: u8 = 7;  // BJT emitter
const ROLE_ANODE: u8 = 8;    // diode: polarity is never permutable
const ROLE_CATHODE: u8 = 9;
const NUM_ROLES: usize = 10;

/// Two-terminal pin roles: diodes are polar (terminal_a = anode by extraction
/// convention), resistors and capacitors are symmetric.
fn two_term_roles(k: &TwoTerminalKind) -> [u8; 2] {
    match k {
        TwoTerminalKind::Diode => [ROLE_ANODE, ROLE_CATHODE],
        _ => [ROLE_TERM, ROLE_TERM],
    }
}

struct GraphDev {
    seed: u32,
    pin_count: u8,
    nets: [u32; MAX_PINS],
    roles: [u8; MAX_PINS],
    /// Index into original device arrays for parametric checks.
    /// For MOS: index into ext.devices / reference.devices.
    /// For two-terminal: u32::MAX (no parametric check on them yet).
    orig_idx: u32,
    is_mos: bool,
}

struct TopoGraph {
    devs: Vec<GraphDev>,
    net_count: usize,
}

fn graph_from_extracted(ext: &ExtractedNetlist, strict: bool) -> TopoGraph {
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut local = |n: u32| -> u32 {
        let next = remap.len() as u32;
        *remap.entry(n).or_insert(next)
    };
    let mut devs = Vec::new();

    // ponytail: body pin omitted until Phase 3 implements well extraction.
    // body=0 placeholder would collide with real net IDs in the remap.
    for (i, d) in ext.devices.iter().enumerate() {
        let g = local(d.gate);
        let s = local(d.source);
        let dr = local(d.drain);
        let mut seed = kind_tag(&d.kind) * 3 + flavor_tag(&d.flavor);
        // DMOS gets a distinct seed offset from regular MOS
        if d.device_class.as_deref() == Some("dmos") {
            seed += 20;
        }
        let (sr, dr_r) = if strict { (ROLE_SOURCE, ROLE_DRAIN) } else { (ROLE_SD, ROLE_SD) };
        devs.push(GraphDev {
            seed, pin_count: 3, nets: [g, s, dr, 0], roles: [ROLE_GATE, sr, dr_r, 0],
            orig_idx: i as u32, is_mos: true,
        });
    }

    // BJT devices (3-pin: collector, base, emitter)
    for d in &ext.bjt_devices {
        let c = local(d.collector);
        let b = local(d.base);
        let e = local(d.emitter);
        let seed = bjt_kind_tag(&d.kind);
        devs.push(GraphDev {
            seed, pin_count: 3,
            nets: [c, b, e, 0],
            roles: [ROLE_COLLECTOR, ROLE_BASE, ROLE_EMITTER, 0],
            orig_idx: u32::MAX, is_mos: false,
        });
    }

    for d in &ext.two_terminal {
        let a = local(d.terminal_a);
        let b = local(d.terminal_b);
        // seed by KIND only: matching is topology-driven, not name-driven —
        // auto-generated layout names must never have to line up with the schematic.
        let seed = 100 + two_term_kind_tag(&d.kind) * 10;
        let [ra, rb] = two_term_roles(&d.kind);
        devs.push(GraphDev {
            seed, pin_count: 2, nets: [a, b, 0, 0], roles: [ra, rb, 0, 0],
            orig_idx: u32::MAX, is_mos: false,
        });
    }

    TopoGraph { devs, net_count: remap.len() }
}

fn graph_from_reference(reference: &RefNetlist, strict: bool) -> (TopoGraph, HashMap<String, u32>) {
    let mut remap: HashMap<String, u32> = HashMap::new();
    let mut local = |n: &str| -> u32 {
        if let Some(&v) = remap.get(n) { return v; }
        let next = remap.len() as u32;
        remap.insert(n.to_string(), next);
        next
    };
    let mut devs = Vec::new();

    for (i, d) in reference.devices.iter().enumerate() {
        let g = local(&d.gate);
        let s = local(&d.source);
        let dr = local(&d.drain);
        let seed = kind_tag(&d.kind) * 3 + flavor_tag(&d.flavor);
        let (sr, dr_r) = if strict { (ROLE_SOURCE, ROLE_DRAIN) } else { (ROLE_SD, ROLE_SD) };
        devs.push(GraphDev {
            seed, pin_count: 3, nets: [g, s, dr, 0], roles: [ROLE_GATE, sr, dr_r, 0],
            orig_idx: i as u32, is_mos: true,
        });
    }

    // BJT devices (3-pin: collector, base, emitter)
    for d in &reference.ref_bjt {
        let c = local(&d.collector);
        let b = local(&d.base);
        let e = local(&d.emitter);
        let seed = bjt_kind_tag(&d.kind);
        devs.push(GraphDev {
            seed, pin_count: 3,
            nets: [c, b, e, 0],
            roles: [ROLE_COLLECTOR, ROLE_BASE, ROLE_EMITTER, 0],
            orig_idx: u32::MAX, is_mos: false,
        });
    }

    for d in &reference.ref_two_terminal {
        let a = local(&d.terminal_a);
        let b = local(&d.terminal_b);
        let seed = 100 + two_term_kind_tag(&d.kind) * 10;
        let [ra, rb] = two_term_roles(&d.kind);
        devs.push(GraphDev {
            seed, pin_count: 2, nets: [a, b, 0, 0], roles: [ra, rb, 0, 0],
            orig_idx: u32::MAX, is_mos: false,
        });
    }

    let net_count = remap.len();
    (TopoGraph { devs, net_count }, remap)
}

// --- probabilistic partition refinement ---

fn build_pin_magic(rng: &mut Rng) -> [u64; NUM_ROLES] {
    let mut m = [0u64; NUM_ROLES];
    for i in 0..NUM_ROLES { m[i] = rng.next(); }
    m
}

/// Compute hash for each device: SUM over pins of (pin_magic[role] XOR class_magic[net_class]).
fn compute_dev_hashes(
    g: &TopoGraph, net_class: &[u32], pin_magic: &[u64; NUM_ROLES], class_magic: &[u64],
) -> Vec<u64> {
    g.devs.iter().map(|d| {
        let mut h: u64 = 0;
        for i in 0..d.pin_count as usize {
            let nc = net_class[d.nets[i] as usize];
            let cm = class_magic.get(nc as usize).copied().unwrap_or(0);
            h = h.wrapping_add(pin_magic[d.roles[i] as usize] ^ cm);
        }
        h
    }).collect()
}

/// Compute hash for each net: SUM over incident device terminals of (pin_magic[role] XOR class_magic[dev_class]).
fn compute_net_hashes(
    g: &TopoGraph, dev_class: &[u32], pin_magic: &[u64; NUM_ROLES], class_magic: &[u64],
    net_count: usize,
) -> Vec<u64> {
    let mut hashes = vec![0u64; net_count];
    let mut has_term = vec![false; net_count];
    for (di, d) in g.devs.iter().enumerate() {
        let dc = dev_class[di];
        let cm = class_magic.get(dc as usize).copied().unwrap_or(0);
        for i in 0..d.pin_count as usize {
            let net = d.nets[i] as usize;
            if net < net_count {
                hashes[net] = hashes[net].wrapping_add(pin_magic[d.roles[i] as usize] ^ cm);
                has_term[net] = true;
            }
        }
    }
    // Nets with no terminals get a sentinel hash
    for i in 0..net_count {
        if !has_term[i] { hashes[i] = u64::MAX; }
    }
    hashes
}

/// Fracture device classes: within each existing class, elements with different hashes
/// get split into new classes. Both graphs are fractured jointly (shared class space).
fn fracture_devs(
    cls_a: &mut [u32], hash_a: &[u64],
    cls_b: &mut [u32], hash_b: &[u64],
    class_magic: &mut Vec<u64>, rng: &mut Rng,
) -> bool {
    let mut groups: HashMap<u32, HashMap<u64, (Vec<usize>, Vec<usize>)>> = HashMap::new();
    for (i, (&c, &h)) in cls_a.iter().zip(hash_a).enumerate() {
        groups.entry(c).or_default().entry(h).or_default().0.push(i);
    }
    for (i, (&c, &h)) in cls_b.iter().zip(hash_b).enumerate() {
        groups.entry(c).or_default().entry(h).or_default().1.push(i);
    }
    let mut changed = false;
    for (_, hash_groups) in &groups {
        if hash_groups.len() <= 1 { continue; }
        changed = true;
        // First hash group keeps original class; others get new classes.
        let mut first = true;
        for (_, (ma, mb)) in hash_groups {
            if first { first = false; continue; }
            let new_cls = class_magic.len() as u32;
            class_magic.push(rng.next());
            for &m in ma { cls_a[m] = new_cls; }
            for &m in mb { cls_b[m] = new_cls; }
        }
    }
    changed
}

/// Fracture net classes — same algorithm as device fracturing.
fn fracture_nets(
    cls_a: &mut [u32], hash_a: &[u64],
    cls_b: &mut [u32], hash_b: &[u64],
    class_magic: &mut Vec<u64>, rng: &mut Rng,
) -> bool {
    fracture_devs(cls_a, hash_a, cls_b, hash_b, class_magic, rng)
}

/// Run refinement until no class splits occur.
fn iterate_to_fixpoint(
    ga: &TopoGraph, gb: &TopoGraph,
    dev_cls_a: &mut Vec<u32>, dev_cls_b: &mut Vec<u32>,
    net_cls_a: &mut Vec<u32>, net_cls_b: &mut Vec<u32>,
    class_magic: &mut Vec<u64>, pin_magic: &[u64; NUM_ROLES],
    rng: &mut Rng,
) {
    for _ in 0..1000 {
        let dh_a = compute_dev_hashes(ga, net_cls_a, pin_magic, class_magic);
        let dh_b = compute_dev_hashes(gb, net_cls_b, pin_magic, class_magic);
        let d_changed = fracture_devs(dev_cls_a, &dh_a, dev_cls_b, &dh_b, class_magic, rng);

        let nh_a = compute_net_hashes(ga, dev_cls_a, pin_magic, class_magic, ga.net_count);
        let nh_b = compute_net_hashes(gb, dev_cls_b, pin_magic, class_magic, gb.net_count);
        let n_changed = fracture_nets(net_cls_a, &nh_a, net_cls_b, &nh_b, class_magic, rng);

        if !d_changed && !n_changed { break; }
    }
}

/// Count ambiguous classes (>1 member in BOTH graphs).
/// Device and net classes are checked separately since their ID spaces may overlap
/// at initial seeds.
fn count_ambiguous(
    dev_cls_a: &[u32], dev_cls_b: &[u32],
    net_cls_a: &[u32], net_cls_b: &[u32],
) -> usize {
    let mut count = 0;
    // Device classes
    let mut da: HashMap<u32, usize> = HashMap::new();
    let mut db: HashMap<u32, usize> = HashMap::new();
    for &c in dev_cls_a { *da.entry(c).or_default() += 1; }
    for &c in dev_cls_b { *db.entry(c).or_default() += 1; }
    for (&c, &ca) in &da {
        if ca > 1 { if db.get(&c).copied().unwrap_or(0) > 1 { count += 1; } }
    }
    // Net classes
    let mut na: HashMap<u32, usize> = HashMap::new();
    let mut nb: HashMap<u32, usize> = HashMap::new();
    for &c in net_cls_a { if c != u32::MAX { *na.entry(c).or_default() += 1; } }
    for &c in net_cls_b { if c != u32::MAX { *nb.entry(c).or_default() += 1; } }
    for (&c, &ca) in &na {
        if ca > 1 { if nb.get(&c).copied().unwrap_or(0) > 1 { count += 1; } }
    }
    count
}

/// Break one automorphism: find smallest ambiguous class, force-assign unique classes
/// to one element from each graph. Returns true if an automorphism was broken.
fn resolve_one_automorphism(
    dev_cls_a: &mut [u32], dev_cls_b: &mut [u32],
    net_cls_a: &mut [u32], net_cls_b: &mut [u32],
    class_magic: &mut Vec<u64>, rng: &mut Rng,
) -> bool {
    // Check device classes for ambiguity
    let mut cls_members_a: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut cls_members_b: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, &c) in dev_cls_a.iter().enumerate() { cls_members_a.entry(c).or_default().push(i); }
    for (i, &c) in dev_cls_b.iter().enumerate() { cls_members_b.entry(c).or_default().push(i); }

    // Find smallest ambiguous device class
    let mut best: Option<(usize, u32)> = None;
    for (&c, ma) in &cls_members_a {
        if ma.len() <= 1 { continue; }
        if let Some(mb) = cls_members_b.get(&c) {
            if mb.len() <= 1 { continue; }
            let size = ma.len() + mb.len();
            if best.map_or(true, |(s, _)| size < s) { best = Some((size, c)); }
        }
    }

    // Also check net classes
    let mut net_ma: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut net_mb: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, &c) in net_cls_a.iter().enumerate() { if c != u32::MAX { net_ma.entry(c).or_default().push(i); } }
    for (i, &c) in net_cls_b.iter().enumerate() { if c != u32::MAX { net_mb.entry(c).or_default().push(i); } }
    for (&c, ma) in &net_ma {
        if ma.len() <= 1 { continue; }
        if let Some(mb) = net_mb.get(&c) {
            if mb.len() <= 1 { continue; }
            let size = ma.len() + mb.len();
            if best.map_or(true, |(s, _)| size < s) { best = Some((size, c)); }
        }
    }

    let Some((_, cls)) = best else { return false; };

    // Break it: assign unique class to one member from each graph
    if let (Some(ma), Some(mb)) = (cls_members_a.get(&cls), cls_members_b.get(&cls)) {
        if ma.len() > 1 && mb.len() > 1 {
            let new_cls = class_magic.len() as u32;
            class_magic.push(rng.next());
            dev_cls_a[ma[0]] = new_cls;
            dev_cls_b[mb[0]] = new_cls;
            return true;
        }
    }
    if let (Some(ma), Some(mb)) = (net_ma.get(&cls), net_mb.get(&cls)) {
        if ma.len() > 1 && mb.len() > 1 {
            let new_cls = class_magic.len() as u32;
            class_magic.push(rng.next());
            net_cls_a[ma[0]] = new_cls;
            net_cls_b[mb[0]] = new_cls;
            return true;
        }
    }
    false
}

// --- public API ---

pub struct CompareOpts {
    pub strict: bool,
    pub w_tolerance: PropertyTolerance,
    pub l_tolerance: PropertyTolerance,
}

impl Default for CompareOpts {
    fn default() -> Self {
        CompareOpts {
            strict: false,
            w_tolerance: PropertyTolerance::default(),
            l_tolerance: PropertyTolerance::default(),
        }
    }
}

pub fn compare(ext: &ExtractedNetlist, reference: &RefNetlist, opts: &CompareOpts) -> LvsResult {
    let ext_n = ext.devices.iter().filter(|d| d.kind == DeviceKind::Nmos).count();
    let ext_p = ext.devices.iter().filter(|d| d.kind == DeviceKind::Pmos).count();
    let ref_n = reference.devices.iter().filter(|d| d.kind == DeviceKind::Nmos).count();
    let ref_p = reference.devices.iter().filter(|d| d.kind == DeviceKind::Pmos).count();

    let ext_npn = ext.bjt_devices.iter().filter(|d| d.kind == DeviceKind::Npn).count();
    let ext_pnp = ext.bjt_devices.iter().filter(|d| d.kind == DeviceKind::Pnp).count();
    let ref_npn = reference.ref_bjt.iter().filter(|d| d.kind == DeviceKind::Npn).count();
    let ref_pnp = reference.ref_bjt.iter().filter(|d| d.kind == DeviceKind::Pnp).count();

    let mut mismatches: Vec<Mismatch> = Vec::new();

    // Collect floating nets from extracted netlist
    let floating_nets: Vec<FloatingNet> = ext.floating_nets.clone();

    // Record floating nets as mismatches
    for fnet in &floating_nets {
        mismatches.push(Mismatch::FloatingNet {
            net_id: fnet.net_id,
            label: fnet.label.clone(),
        });
    }

    // Record label conflicts as mismatches
    for conflict in &ext.label_conflicts {
        mismatches.push(Mismatch::LabelConflict {
            net_id: 0,
            labels: vec![conflict.clone()],
        });
    }

    let make_result = |matched, reason: String, ambiguous: usize, mismatches: Vec<Mismatch>| LvsResult {
        matched, reason, mismatches,
        extracted_devices: ext.devices.len(), nmos: ext_n, pmos: ext_p,
        ambiguous_classes: ambiguous, label_conflicts: ext.label_conflicts.clone(),
        floating_nets: floating_nets.clone(),
    };

    // Fast-fail: MOS device count check
    if ext_n != ref_n || ext_p != ref_p {
        if ext_n != ref_n {
            mismatches.push(Mismatch::DeviceCount {
                kind: "Nmos".into(), extracted: ext_n, reference: ref_n,
            });
        }
        if ext_p != ref_p {
            mismatches.push(Mismatch::DeviceCount {
                kind: "Pmos".into(), extracted: ext_p, reference: ref_p,
            });
        }
        let reason = format!(
            "device count mismatch (ext {}N/{}P vs ref {}N/{}P)", ext_n, ext_p, ref_n, ref_p);
        return make_result(false, reason, 0, mismatches);
    }

    // Fast-fail: BJT device count check
    if ext_npn != ref_npn || ext_pnp != ref_pnp {
        if ext_npn != ref_npn {
            mismatches.push(Mismatch::DeviceCount {
                kind: "Npn".into(), extracted: ext_npn, reference: ref_npn,
            });
        }
        if ext_pnp != ref_pnp {
            mismatches.push(Mismatch::DeviceCount {
                kind: "Pnp".into(), extracted: ext_pnp, reference: ref_pnp,
            });
        }
        let reason = format!(
            "device count mismatch (ext {}NPN/{}PNP vs ref {}NPN/{}PNP)",
            ext_npn, ext_pnp, ref_npn, ref_pnp);
        return make_result(false, reason, 0, mismatches);
    }

    let ga = graph_from_extracted(ext, opts.strict);
    let (gb, ref_net_remap) = graph_from_reference(reference, opts.strict);

    // Initialize PRNG and magic tables
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let pin_magic = build_pin_magic(&mut rng);

    // Initial class_magic: one entry per initial seed value
    let max_seed = ga.devs.iter().chain(gb.devs.iter()).map(|d| d.seed).max().unwrap_or(0) as usize + 1;
    let mut class_magic: Vec<u64> = (0..max_seed).map(|_| rng.next()).collect();

    // Seed device classes from device kind
    let mut dev_cls_a: Vec<u32> = ga.devs.iter().map(|d| d.seed).collect();
    let mut dev_cls_b: Vec<u32> = gb.devs.iter().map(|d| d.seed).collect();

    // Ensure class_magic covers all seed values (name_seed can produce large values)
    let max_dev_cls = dev_cls_a.iter().chain(dev_cls_b.iter()).copied().max().unwrap_or(0);
    while class_magic.len() <= max_dev_cls as usize {
        class_magic.push(rng.next());
    }

    // Seed net classes: all nets start in class 0 (already covered by class_magic)
    let mut net_cls_a = vec![0u32; ga.net_count];
    let mut net_cls_b = vec![0u32; gb.net_count];

    // --- Iterative refinement with automorphism breaking ---
    iterate_to_fixpoint(
        &ga, &gb, &mut dev_cls_a, &mut dev_cls_b, &mut net_cls_a, &mut net_cls_b,
        &mut class_magic, &pin_magic, &mut rng,
    );

    // Break automorphisms until fully resolved
    for _ in 0..1000 {
        if !resolve_one_automorphism(
            &mut dev_cls_a, &mut dev_cls_b, &mut net_cls_a, &mut net_cls_b,
            &mut class_magic, &mut rng,
        ) { break; }
        iterate_to_fixpoint(
            &ga, &gb, &mut dev_cls_a, &mut dev_cls_b, &mut net_cls_a, &mut net_cls_b,
            &mut class_magic, &pin_magic, &mut rng,
        );
    }

    let ambiguous = count_ambiguous(&dev_cls_a, &dev_cls_b, &net_cls_a, &net_cls_b);

    // --- Topology validation ---

    // Device class multiset comparison
    let mut dev_buckets_a: HashMap<u32, usize> = HashMap::new();
    let mut dev_buckets_b: HashMap<u32, usize> = HashMap::new();
    for &c in &dev_cls_a { *dev_buckets_a.entry(c).or_default() += 1; }
    for &c in &dev_cls_b { *dev_buckets_b.entry(c).or_default() += 1; }
    let all_dev_cls: HashSet<u32> = dev_buckets_a.keys().chain(dev_buckets_b.keys()).copied().collect();
    for &c in &all_dev_cls {
        let ca = dev_buckets_a.get(&c).copied().unwrap_or(0);
        let cb = dev_buckets_b.get(&c).copied().unwrap_or(0);
        if ca != cb {
            let desc = format!(
                "device class {} has {} in layout vs {} in reference", c, ca, cb);
            mismatches.push(Mismatch::TopologyMismatch { description: desc.clone() });
            return make_result(false, format!("topology mismatch: {}", desc), ambiguous, mismatches);
        }
    }

    // Net class multiset comparison (skip floating nets)
    let mut net_buckets_a: HashMap<u32, usize> = HashMap::new();
    let mut net_buckets_b: HashMap<u32, usize> = HashMap::new();
    for &c in &net_cls_a { if c != u32::MAX { *net_buckets_a.entry(c).or_default() += 1; } }
    for &c in &net_cls_b { if c != u32::MAX { *net_buckets_b.entry(c).or_default() += 1; } }
    let all_net_cls: HashSet<u32> = net_buckets_a.keys().chain(net_buckets_b.keys()).copied().collect();
    for &c in &all_net_cls {
        let ca = net_buckets_a.get(&c).copied().unwrap_or(0);
        let cb = net_buckets_b.get(&c).copied().unwrap_or(0);
        if ca != cb {
            let desc = format!(
                "net class {} has {} nets in layout vs {} in reference", c, ca, cb);
            mismatches.push(Mismatch::TopologyMismatch { description: desc.clone() });
            return make_result(false, format!("topology mismatch: {}", desc), ambiguous, mismatches);
        }
    }

    // Net seed conflict detection (VDD/VSS swap)
    if !reference.net_seeds.is_empty() {
        let mut seed_class_to_names: HashMap<u32, Vec<&str>> = HashMap::new();
        for (net_name, _) in &reference.net_seeds {
            if let Some(&local_id) = ref_net_remap.get(net_name) {
                let c = net_cls_b[local_id as usize];
                if c != u32::MAX {
                    seed_class_to_names.entry(c).or_default().push(net_name.as_str());
                }
            }
        }
        for (_, names) in &seed_class_to_names {
            if names.len() > 1 {
                let nets: Vec<String> = names.iter().map(|s| s.to_string()).collect();
                let reason = format!("net seed conflict: {} are isomorphic", names.join(" and "));
                mismatches.push(Mismatch::NetSeedConflict { nets });
                return make_result(false, reason, ambiguous, mismatches);
            }
        }
    }

    // --- Parametric pass (W/L on MOS devices) ---
    let enforce_parametric = reference.devices.iter().all(|d| d.w > 0 && d.l > 0);
    if enforce_parametric {
        // Bucket MOS devices by their final class
        let mut ext_wl: BTreeMap<u32, Vec<(i32, i32)>> = BTreeMap::new();
        let mut ref_wl: BTreeMap<u32, Vec<(i32, i32)>> = BTreeMap::new();
        for (i, d) in ga.devs.iter().enumerate() {
            if !d.is_mos { continue; }
            let oi = d.orig_idx as usize;
            ext_wl.entry(dev_cls_a[i]).or_default().push((ext.devices[oi].w, ext.devices[oi].l));
        }
        for (i, d) in gb.devs.iter().enumerate() {
            if !d.is_mos { continue; }
            let oi = d.orig_idx as usize;
            ref_wl.entry(dev_cls_b[i]).or_default().push((reference.devices[oi].w, reference.devices[oi].l));
        }

        let within_w = |got: i32, expect: i32| -> bool {
            ((got - expect).abs() as f64) <= (opts.w_tolerance.abs_nm as f64).max(opts.w_tolerance.rel_pct * expect as f64)
        };
        let within_l = |got: i32, expect: i32| -> bool {
            ((got - expect).abs() as f64) <= (opts.l_tolerance.abs_nm as f64).max(opts.l_tolerance.rel_pct * expect as f64)
        };
        for (cls, exts) in ext_wl.iter_mut() {
            if let Some(refs) = ref_wl.get_mut(cls) {
                exts.sort_unstable();
                refs.sort_unstable();
                for (&(gw, gl), &(rw, rl)) in exts.iter().zip(refs.iter()) {
                    if !within_w(gw, rw) || !within_l(gl, rl) {
                        let reason = format!(
                            "parametric mismatch: expected W/L {}/{} got {}/{}",
                            rw, rl, gw, gl);
                        mismatches.push(Mismatch::ParametricMismatch {
                            property: "W/L".into(),
                            got: gw as f64,
                            expected: rw as f64,
                            tolerance: opts.w_tolerance.abs_nm as f64,
                        });
                        return make_result(false, reason, ambiguous, mismatches);
                    }
                }
            }
        }
    }

    make_result(true, "match".into(), ambiguous, mismatches)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_diode_ext(anti_parallel: bool) -> ExtractedNetlist {
        // nets 0 and 1; diode2 flips direction when anti_parallel
        let d1 = TwoTerminalDevice {
            kind: TwoTerminalKind::Diode, name: "d1".into(),
            terminal_a: 0, terminal_b: 1, value: 0.0,
        };
        let d2 = TwoTerminalDevice {
            kind: TwoTerminalKind::Diode, name: "d2".into(),
            terminal_a: if anti_parallel { 1 } else { 0 },
            terminal_b: if anti_parallel { 0 } else { 1 },
            value: 0.0,
        };
        ExtractedNetlist {
            devices: Vec::new(), device_sources: Vec::new(), bjt_devices: Vec::new(), net_count: 2, used_nets: 2,
            net_of_poly: Vec::new(), label_conflicts: Vec::new(),
            two_terminal: vec![d1, d2], floating_nets: Vec::new(),
        }
    }

    fn two_diode_ref() -> RefNetlist {
        let d = |a: &str, b: &str| RefTwoTerminal {
            kind: TwoTerminalKind::Diode, name: String::new(),
            terminal_a: a.into(), terminal_b: b.into(),
        };
        RefNetlist {
            devices: Vec::new(), net_seeds: std::collections::HashMap::new(),
            ref_two_terminal: vec![d("X", "Y"), d("X", "Y")], ref_bjt: Vec::new(),
        }
    }

    /// Diode polarity is topology, not a permutable label: parallel pair matches
    /// the parallel reference, the anti-parallel pair must NOT.
    #[test]
    fn diode_polarity_not_permutable() {
        let opts = CompareOpts::default();
        assert!(compare(&two_diode_ext(false), &two_diode_ref(), &opts).matched,
            "parallel diode pair should match");
        assert!(!compare(&two_diode_ext(true), &two_diode_ref(), &opts).matched,
            "anti-parallel diode pair must mismatch a parallel reference");
    }
}
