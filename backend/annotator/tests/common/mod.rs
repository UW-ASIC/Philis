//! Shared harness for the recognition corpus (EXT-01): a one-line netlist
//! format, a seeded permutation, and the id-free views the tests compare.
#![allow(dead_code)] // each test binary uses a different subset

use std::collections::{BTreeMap, BTreeSet};

use analog::RuleBatch;
use pnr_core::ids::NetId;
use pnr_core::netlist::{Device, DeviceKind, Net, Netlist};

/// Devices separated by newlines or ` | `: `NAME n1 … nk MODEL [w=4u] [l=0.5u] [m=8] [nf=2]`,
/// SPICE order (the model is the last bare token, as in every corpus line).
/// Kind from the model's prefix: `nfet`→Nmos, `pfet`→Pmos, `npn`, `pnp`, `r`→Resistor,
/// `cap`→Capacitor, `d`→Diode; `Device::model` = the whole token (so `nfet_01v8_lvt` works).
/// Terminals: FET nodes D G S B stored as G,D,S,B (as `library::parse` does), BJT C B E,
/// two-terminal P N. `w`, `l`: suffix `u` → ×1000 nm, `n` → ×1 nm, bare → nm. `m`, `nf`:
/// plain integers. Keys are case-insensitive (the fixtures write `W=`); any other key
/// panics rather than being dropped. Net ids in first-appearance order. The annotator
/// cannot dev-depend on `library` (cycle), hence this parser.
pub fn net(src: &str) -> Netlist {
    let mut nets: Vec<Net> = Vec::new();
    let mut ids: BTreeMap<String, u16> = BTreeMap::new();
    let mut intern = |n: &str| {
        NetId(*ids.entry(n.to_string()).or_insert_with(|| {
            nets.push(Net { name: n.to_string() });
            (nets.len() - 1) as u16
        }))
    };
    let mut devices = Vec::new();
    for line in src.split(['\n', '|']).map(str::trim).filter(|l| !l.is_empty()) {
        let (params, bare): (Vec<&str>, Vec<&str>) = line.split_whitespace().partition(|t| t.contains('='));
        let (name, model, nodes) = (bare[0], bare[bare.len() - 1], &bare[1..bare.len() - 1]);
        let kind = [
            ("nfet", DeviceKind::Nmos),
            ("pfet", DeviceKind::Pmos),
            ("npn", DeviceKind::Npn),
            ("pnp", DeviceKind::Pnp),
            ("cap", DeviceKind::Capacitor),
            ("r", DeviceKind::Resistor),
            ("d", DeviceKind::Diode),
        ]
        .into_iter()
        .find(|(p, _)| model.starts_with(p))
        .unwrap_or_else(|| panic!("{name}: unknown model {model}"))
        .1;
        let (order, terms): (&[usize], &[&str]) = match kind {
            DeviceKind::Nmos | DeviceKind::Pmos => (&[1, 0, 2, 3], &["G", "D", "S", "B"]),
            DeviceKind::Npn | DeviceKind::Pnp => (&[0, 1, 2], &["C", "B", "E"]),
            _ => (&[0, 1], &["P", "N"]),
        };
        assert_eq!(nodes.len(), order.len(), "{name}: node count");
        // Intern in SPICE order so net ids follow first appearance.
        let listed: Vec<NetId> = nodes.iter().map(|n| intern(n)).collect();
        let terminals = order.iter().zip(terms).map(|(&i, t)| ((*t).to_string(), listed[i])).collect();
        let params = params
            .iter()
            .map(|p| {
                let (k, v) = p.split_once('=').unwrap();
                let k = k.to_ascii_lowercase();
                let v = match k.as_str() {
                    "w" | "l" => match v.as_bytes()[v.len() - 1] {
                        b'u' => (v[..v.len() - 1].parse::<f64>().unwrap() * 1000.0).round() as i64,
                        b'n' => v[..v.len() - 1].parse::<f64>().unwrap().round() as i64,
                        _ => v.parse::<f64>().unwrap().round() as i64,
                    },
                    "m" | "nf" => v.parse().unwrap(),
                    _ => panic!("{name}: unsupported param {k}"),
                };
                (k, v)
            })
            .collect();
        devices.push(Device { name: name.into(), kind, model: model.into(), terminals, params });
    }
    Netlist { devices, nets }
}

/// `benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.sp`:
/// n→nfet, p→pfet, l=14e-9→14n, nfin/nf dropped, m kept, and `w=1u` on every device, a
/// Philis placeholder (the .sp carries no w; once EXT-11 makes a missing w an unknown size,
/// none would leave no `ExactAs` pattern, hence no seed pair, matching).
/// Instance names are the ALIGN gold's.
pub const STRONGARM: &str = "mn0 vcom clk vss vss nfet w=1u l=14n m=8
    mn1 vin_d vin vcom vss nfet w=1u l=14n m=16 | mn2 vip_d vip vcom vss nfet w=1u l=14n m=16
    mn3 vin_o vip_o vin_d vss nfet w=1u l=14n m=8 | mn4 vip_o vin_o vip_d vss nfet w=1u l=14n m=8
    mp5 vin_o vip_o vcc vcc pfet w=1u l=14n m=4 | mp6 vip_o vin_o vcc vcc pfet w=1u l=14n m=4
    mp7 vin_d clk vcc vcc pfet w=1u l=14n m=1 | mp8 vip_d clk vcc vcc pfet w=1u l=14n m=1
    mp9 vin_o clk vcc vcc pfet w=1u l=14n m=1 | mp10 vip_o clk vcc vcc pfet w=1u l=14n m=1
    mp11 vop vip_o vcc vcc pfet w=1u l=14n m=1 | mn13 vop vip_o vss vss nfet w=1u l=14n m=1
    mp12 von vin_o vcc vcc pfet w=1u l=14n m=1 | mn14 von vin_o vss vss nfet w=1u l=14n m=1";

/// The gold's `PowerPorts`, `GroundPorts` and `ClockPorts`; never its symmetry entries.
pub fn strongarm_cfg() -> annotator::AnnotationConfig {
    annotator::AnnotationConfig {
        supply_nets: vec!["VCC".into()],
        ground_nets: vec!["VSS".into()],
        clock_nets: vec!["clk".into()],
        ..annotator::AnnotationConfig::default()
    }
}

/// Seeded shuffle (xorshift64) of devices, nets and each device's terminal list.
pub fn permute(nl: &Netlist, seed: u64) -> Netlist {
    let mut s = seed.max(1);
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut shuffle = |n: usize| {
        let mut p: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            p.swap(i, (next() % (i as u64 + 1)) as usize);
        }
        p
    };
    // New position of old net i.
    let net_order = shuffle(nl.nets.len());
    let mut new_id = vec![0u16; nl.nets.len()];
    for (new, &old) in net_order.iter().enumerate() {
        new_id[old] = new as u16;
    }
    let nets = net_order.iter().map(|&i| nl.nets[i].clone()).collect();
    let devices = shuffle(nl.devices.len())
        .into_iter()
        .map(|i| {
            let d = &nl.devices[i];
            let terminals = shuffle(d.terminals.len())
                .into_iter()
                .map(|t| (d.terminals[t].0.clone(), NetId(new_id[d.terminals[t].1 .0 as usize])))
                .collect();
            Device { terminals, ..d.clone() }
        })
        .collect();
    Netlist { devices, nets }
}

/// Name-level, id-free view of what the annotator decided.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Canon {
    /// Matched sets `(members (name, parallel, series), kind, class)`: empty until
    /// EXT-12 adds `MatchSpec`; the annotator has no matched-set output today.
    pub sets: BTreeSet<(Vec<(String, u16, u16)>, String /*kind*/, String /*class*/)>,
    /// Hard `Symmetry` pairs of two distinct devices, names sorted.
    pub pairs: BTreeSet<(String, String)>,
    /// Hard self-symmetric `Symmetry` entries (`a == b`).
    pub selfs: BTreeSet<String>,
    /// Hard `Differential` routing pairs, names sorted.
    pub net_pairs: BTreeSet<(String, String)>,
    /// Distinct symmetry axes among the hard pairs.
    pub axes: usize,
}

/// `(a, b)` with the smaller name first.
pub fn sorted(a: &str, b: &str) -> (String, String) {
    if a <= b { (a.into(), b.into()) } else { (b.into(), a.into()) }
}

pub fn canon(p: &annotator::Problem, nl: &Netlist) -> Canon {
    let dev = |i: u32| nl.devices[i as usize].name.as_str();
    let mut mirror = Vec::new();
    p.placement.hard.iter().for_each(|b| b.mirror_pairs(&mut mirror));
    let mut c = Canon { axes: mirror.iter().map(|m| m.2).collect::<BTreeSet<_>>().len(), ..Canon::default() };
    for &(a, b, _) in &mirror {
        if a == b {
            c.selfs.insert(dev(a).into());
        } else {
            c.pairs.insert(sorted(dev(a), dev(b)));
        }
    }
    for b in p.routing.hard.iter().filter(|b| b.kind().ends_with("::Differential")) {
        for (x, y) in id_pairs(b.as_ref()) {
            c.net_pairs.insert(sorted(&nl.nets[x as usize].name, &nl.nets[y as usize].name));
        }
    }
    c
}

/// The `(a, b)` ids of a batch whose every rule touches exactly two ids.
pub fn id_pairs<On>(b: &dyn RuleBatch<On>) -> Vec<(u32, u32)> {
    let mut ids = Vec::new();
    b.touched(&mut ids);
    assert_eq!(ids.len(), 2 * b.count(), "{}: every rule must touch two ids", b.kind());
    ids.chunks(2).map(|p| (p[0], p[1])).collect()
}

/// Interim form used until EXT-12 lands: (leaf kind, sorted member names).
pub fn canon_leaves(p: &annotator::Problem, nl: &Netlist) -> BTreeSet<(String, Vec<String>)> {
    annotator::block::leaves(&p.blocks)
        .into_iter()
        .map(|b| {
            let mut names: Vec<String> = b.devices.iter().map(|d| nl.devices[d.0 as usize].name.clone()).collect();
            names.sort();
            (format!("{:?}", b.kind), names)
        })
        .collect()
}
