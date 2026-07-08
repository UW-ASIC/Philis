//! Connectivity + device extraction from layout geometry.

use crate::geometry::*;
use crate::params::Deck;
use crate::traits::Backend;
use super::types::*;
use std::collections::{HashMap, HashSet};

// --- union-find ---

struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
}
impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind { parent: (0..n as u32).collect(), rank: vec![0; n] }
    }
    fn find(&mut self, x: u32) -> u32 {
        let mut r = x;
        while self.parent[r as usize] != r {
            self.parent[r as usize] = self.parent[self.parent[r as usize] as usize];
            r = self.parent[r as usize];
        }
        r
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb { return; }
        if self.rank[ra as usize] < self.rank[rb as usize] {
            self.parent[ra as usize] = rb;
        } else if self.rank[ra as usize] > self.rank[rb as usize] {
            self.parent[rb as usize] = ra;
        } else {
            self.parent[rb as usize] = ra;
            self.rank[ra as usize] += 1;
        }
    }
}

// --- internal types ---

struct Node {
    bbox: Bbox,
    layer: LayerId,
    is_diff_seg: bool,
}

struct GateSpan {
    gate: u32,
    lo: i32,
    hi: i32,
}

struct DiffSplit {
    diff: u32,
    axis_x: bool,
    seg_nodes: Vec<u32>,
    seg_spans: Vec<(i32, i32)>,
    gates: Vec<GateSpan>,
}

// --- helpers ---

#[inline]
fn overlap_pos(a: Bbox, b: Bbox) -> bool {
    a.xmax.min(b.xmax) > a.xmin.max(b.xmin) && a.ymax.min(b.ymax) > a.ymin.max(b.ymin)
}

/// Touching-edge overlap: polygons that share an edge (>=) connect when intra_layer_touch is on.
#[inline]
fn overlap_touch(a: Bbox, b: Bbox) -> bool {
    a.xmax.min(b.xmax) >= a.xmin.max(b.xmin) && a.ymax.min(b.ymax) >= a.ymin.max(b.ymin)
}

fn intersect_bbox(a: Bbox, b: Bbox) -> Bbox {
    Bbox {
        xmin: a.xmin.max(b.xmin),
        ymin: a.ymin.max(b.ymin),
        xmax: a.xmax.min(b.xmax),
        ymax: a.ymax.min(b.ymax),
    }
}

fn poly_poly_overlap(store: &GeometryStore, a: PolyId, b: PolyId) -> bool {
    let ba = store.poly_bbox[a.0 as usize];
    let bb = store.poly_bbox[b.0 as usize];
    if !ba.overlaps(&bb) { return false; }
    let ix = ba.xmax.min(bb.xmax) - ba.xmin.max(bb.xmin);
    let iy = ba.ymax.min(bb.ymax) - ba.ymin.max(bb.ymin);
    ix > 0 && iy > 0
}

/// Resolve connectivity from PDK config. Returns error if no conductors configured.
fn resolve_connectivity(deck: &Deck) -> Result<(Vec<LayerId>, Vec<LayerId>), String> {
    if deck.connectivity.conductors.is_empty() {
        return Err("no conductors configured in connectivity schema".into());
    }
    let vias = deck.connectivity.vias.iter().map(|(v, _)| *v).collect();
    Ok((deck.connectivity.conductors.clone(), vias))
}

// --- device type resolution ---

/// Result of matching a MOS rule: device kind, flavor, and the index of the matched rule.
struct MosMatch {
    kind: DeviceKind,
    flavor: DeviceFlavor,
    rule_idx: usize,
}

fn pick_type_and_flavor(
    store: &GeometryStore, deck: &Deck, gate_bb: Bbox,
) -> Result<MosMatch, String> {
    if deck.devices.mos_rules.is_empty() {
        return Err("found gate crossing but no MOS rules to classify device".into());
    }
    for (ri, rule) in deck.devices.mos_rules.iter().enumerate() {
        let implant_overlaps = store.polys_on_layer(rule.type_implant)
            .into_iter().any(|p| store.poly_bbox[p.0 as usize].overlaps(&gate_bb));
        if !implant_overlaps { continue; }
        let kind = match rule.device_type.as_str() {
            "pmos" => DeviceKind::Pmos,
            _ => DeviceKind::Nmos,
        };
        let mut flavor = DeviceFlavor::Standard;
        for &(marker_layer, ref flavor_name) in &rule.flavor_markers {
            let marker_overlaps = store.polys_on_layer(marker_layer)
                .into_iter().any(|p| store.poly_bbox[p.0 as usize].overlaps(&gate_bb));
            if marker_overlaps {
                flavor = match flavor_name.as_str() {
                    "hvt" | "Hvt" | "HVT" => DeviceFlavor::Hvt,
                    "lvt" | "Lvt" | "LVT" => DeviceFlavor::Lvt,
                    _ => DeviceFlavor::Standard,
                };
                break;
            }
        }
        return Ok(MosMatch { kind, flavor, rule_idx: ri });
    }
    Ok(MosMatch { kind: DeviceKind::Nmos, flavor: DeviceFlavor::Standard, rule_idx: 0 })
}

// --- two-terminal device extraction ---

fn extract_two_terminal_devices(
    store: &GeometryStore, deck: &Deck, net_of_poly: &[u32],
) -> Vec<TwoTerminalDevice> {
    let mut out = Vec::new();

    for rule in &deck.devices.resistor_rules {
        let gate_layers: Vec<LayerId> = deck.devices.mos_rules.iter()
            .map(|r| r.gate_layer).collect();
        for body in store.polys_on_layer(rule.body_layer) {
            let bb = store.poly_bbox[body.0 as usize];
            let has_marker = store.polys_on_layer(rule.marker_layer)
                .iter().any(|&m| store.poly_bbox[m.0 as usize].overlaps(&bb));
            if !has_marker { continue; }
            let is_gate = gate_layers.iter().any(|&gl| {
                store.polys_on_layer(gl).iter().any(|&g| {
                    let gb = store.poly_bbox[g.0 as usize];
                    let ix = bb.xmax.min(gb.xmax) - bb.xmin.max(gb.xmin);
                    let iy = bb.ymax.min(gb.ymax) - bb.ymin.max(gb.ymin);
                    ix > 0 && iy > 0
                })
            });
            if is_gate { continue; }
            let mut terminals: Vec<(u32, i32)> = Vec::new();
            let long_axis_x = bb.width() >= bb.height();
            for tc in store.polys_on_layer(rule.terminal_layer) {
                let tb = store.poly_bbox[tc.0 as usize];
                if !tb.overlaps(&bb) { continue; }
                let net = net_of_poly[tc.0 as usize];
                if net == u32::MAX { continue; }
                let pos = if long_axis_x { (tb.xmin + tb.xmax) / 2 } else { (tb.ymin + tb.ymax) / 2 };
                terminals.push((net, pos));
            }
            if terminals.len() < 2 { continue; }
            terminals.sort_by_key(|&(_, p)| p);
            let ta = terminals.first().unwrap().0;
            let tb_net = terminals.last().unwrap().0;
            let value = deck.pex.get(&rule.body_layer).map_or(0.0, |p| {
                let (l, w) = if long_axis_x {
                    (bb.width() as f64, bb.height() as f64)
                } else {
                    (bb.height() as f64, bb.width() as f64)
                };
                if w > 0.0 { p.sheet_res_ohm_sq * l / w } else { 0.0 }
            });
            out.push(TwoTerminalDevice {
                kind: TwoTerminalKind::Resistor,
                name: rule.name.clone(),
                terminal_a: ta, terminal_b: tb_net, value,
            });
        }
    }

    for rule in &deck.devices.diode_rules {
        for anode in store.polys_on_layer(rule.anode_layer) {
            let ab = store.poly_bbox[anode.0 as usize];
            for cathode in store.polys_on_layer(rule.cathode_layer) {
                let cb = store.poly_bbox[cathode.0 as usize];
                let ix = ab.xmax.min(cb.xmax) - ab.xmin.max(cb.xmin);
                let iy = ab.ymax.min(cb.ymax) - ab.ymin.max(cb.ymin);
                if ix <= 0 || iy <= 0 { continue; }
                let overlap_bb = Bbox {
                    xmin: ab.xmin.max(cb.xmin), ymin: ab.ymin.max(cb.ymin),
                    xmax: ab.xmax.min(cb.xmax), ymax: ab.ymax.min(cb.ymax),
                };
                let has_implant = store.polys_on_layer(rule.implant_layer)
                    .iter().any(|&imp| store.poly_bbox[imp.0 as usize].overlaps(&overlap_bb));
                if !has_implant { continue; }
                let a_net = net_of_poly[anode.0 as usize];
                let c_net = net_of_poly[cathode.0 as usize];
                if a_net == u32::MAX || c_net == u32::MAX { continue; }
                out.push(TwoTerminalDevice {
                    kind: TwoTerminalKind::Diode,
                    name: rule.name.clone(),
                    terminal_a: a_net, terminal_b: c_net, value: 0.0,
                });
            }
        }
    }

    for rule in &deck.devices.cap_rules {
        for top in store.polys_on_layer(rule.top_layer) {
            let tb = store.poly_bbox[top.0 as usize];
            for bot in store.polys_on_layer(rule.bottom_layer) {
                let bb = store.poly_bbox[bot.0 as usize];
                let ix = tb.xmax.min(bb.xmax) - tb.xmin.max(bb.xmin);
                let iy = tb.ymax.min(bb.ymax) - tb.ymin.max(bb.ymin);
                if ix <= 0 || iy <= 0 { continue; }
                if let Some(marker_l) = rule.marker_layer {
                    let overlap_bb = Bbox {
                        xmin: tb.xmin.max(bb.xmin), ymin: tb.ymin.max(bb.ymin),
                        xmax: tb.xmax.min(bb.xmax), ymax: tb.ymax.min(bb.ymax),
                    };
                    let has_marker = store.polys_on_layer(marker_l)
                        .iter().any(|&m| store.poly_bbox[m.0 as usize].overlaps(&overlap_bb));
                    if !has_marker { continue; }
                }
                let t_net = net_of_poly[top.0 as usize];
                let b_net = net_of_poly[bot.0 as usize];
                if t_net == u32::MAX || b_net == u32::MAX { continue; }
                let area = (ix as f64) * (iy as f64);
                let cap_per_area = deck.pex.get(&rule.top_layer)
                    .map_or(0.0, |p| p.interlayer_cap_af_um2);
                let value = area * cap_per_area;
                out.push(TwoTerminalDevice {
                    kind: TwoTerminalKind::Capacitor,
                    name: rule.name.clone(),
                    terminal_a: t_net, terminal_b: b_net, value,
                });
            }
        }
    }

    out
}

// --- BJT device extraction ---

fn extract_bjt_devices(
    store: &GeometryStore, deck: &Deck, net_of_poly: &[u32],
) -> Vec<BjtDevice> {
    let mut out = Vec::new();
    for rule in &deck.devices.bjt_rules {
        let kind = match rule.device_type.as_str() {
            "pnp" => DeviceKind::Pnp,
            _ => DeviceKind::Npn,
        };
        for emitter in store.polys_on_layer(rule.emitter_layer) {
            let eb = store.poly_bbox[emitter.0 as usize];
            for base in store.polys_on_layer(rule.base_layer) {
                let bb = store.poly_bbox[base.0 as usize];
                let ix_eb = eb.xmax.min(bb.xmax) - eb.xmin.max(bb.xmin);
                let iy_eb = eb.ymax.min(bb.ymax) - eb.ymin.max(bb.ymin);
                if ix_eb <= 0 || iy_eb <= 0 { continue; }
                for collector in store.polys_on_layer(rule.collector_layer) {
                    let cb = store.poly_bbox[collector.0 as usize];
                    let ix_bc = bb.xmax.min(cb.xmax) - bb.xmin.max(cb.xmin);
                    let iy_bc = bb.ymax.min(cb.ymax) - bb.ymin.max(cb.ymin);
                    if ix_bc <= 0 || iy_bc <= 0 { continue; }
                    // Check type marker overlap at the emitter-base intersection
                    let marker_bb = Bbox {
                        xmin: eb.xmin.max(bb.xmin), ymin: eb.ymin.max(bb.ymin),
                        xmax: eb.xmax.min(bb.xmax), ymax: eb.ymax.min(bb.ymax),
                    };
                    let has_marker = store.polys_on_layer(rule.type_marker)
                        .iter().any(|&m| store.poly_bbox[m.0 as usize].overlaps(&marker_bb));
                    if !has_marker { continue; }
                    let e_net = net_of_poly[emitter.0 as usize];
                    let b_net = net_of_poly[base.0 as usize];
                    let c_net = net_of_poly[collector.0 as usize];
                    if e_net == u32::MAX || b_net == u32::MAX || c_net == u32::MAX { continue; }
                    out.push(BjtDevice {
                        kind: kind.clone(),
                        collector: c_net,
                        base: b_net,
                        emitter: e_net,
                        name: rule.name.clone(),
                    });
                }
            }
        }
    }
    out
}

// --- netlist reduction ---

/// Hash a device_class Option<String> to a u64 for use in merge keys.
fn class_hash(dc: &Option<String>) -> u64 {
    match dc {
        None => 0,
        Some(s) => {
            let mut h: u64 = 0xcafe_babe;
            for b in s.bytes() { h = h.wrapping_mul(31).wrapping_add(b as u64); }
            h
        }
    }
}

pub fn reduce_netlist(ext: &mut ExtractedNetlist) {
    loop {
        let before = ext.devices.len();

        // parallel reduction — S/D symmetric (MOS terminals are interchangeable),
        // normalize by sorting so interleaved-finger devices merge correctly
        let mut par_map: HashMap<(u32, u32, u32, u32, u32, u64), usize> = HashMap::new();
        let mut merged_par: Vec<Device> = Vec::new();
        for mut d in ext.devices.drain(..) {
            let kt = super::compare::kind_tag(&d.kind);
            let ft = super::compare::flavor_tag(&d.flavor);
            let ch = class_hash(&d.device_class);
            if d.source > d.drain { std::mem::swap(&mut d.source, &mut d.drain); }
            let key = (kt, ft, d.gate, d.source, d.drain, ch);
            if let Some(&idx) = par_map.get(&key) {
                merged_par[idx].w += d.w;
            } else {
                par_map.insert(key, merged_par.len());
                merged_par.push(d);
            }
        }
        ext.devices = merged_par;

        // series reduction — only combine same-class devices
        let mut merged_any_series = false;
        let mut used = vec![false; ext.devices.len()];
        let mut merged_ser: Vec<Device> = Vec::new();

        let mut src_index: HashMap<(u32, u32, u32, u32, u64), Vec<usize>> = HashMap::new();
        for (i, d) in ext.devices.iter().enumerate() {
            let kt = super::compare::kind_tag(&d.kind);
            let ft = super::compare::flavor_tag(&d.flavor);
            let ch = class_hash(&d.device_class);
            src_index.entry((kt, ft, d.gate, d.source, ch)).or_default().push(i);
        }

        for i in 0..ext.devices.len() {
            if used[i] { continue; }
            let d = &ext.devices[i];
            let kt = super::compare::kind_tag(&d.kind);
            let ft = super::compare::flavor_tag(&d.flavor);
            let ch = class_hash(&d.device_class);
            let key = (kt, ft, d.gate, d.drain, ch);
            let mut found = None;
            if let Some(candidates) = src_index.get(&key) {
                for &j in candidates {
                    if j != i && !used[j] {
                        found = Some(j);
                        break;
                    }
                }
            }
            if let Some(j) = found {
                let d2 = &ext.devices[j];
                merged_ser.push(Device {
                    kind: d.kind.clone(), gate: d.gate, source: d.source, drain: d2.drain,
                    body: d.body, flavor: d.flavor, w: d.w.min(d2.w), l: d.l + d2.l,
                    device_class: d.device_class.clone(),
                });
                used[i] = true;
                used[j] = true;
                merged_any_series = true;
            } else {
                used[i] = true;
                merged_ser.push(d.clone());
            }
        }
        ext.devices = merged_ser;

        let mut used_set = HashSet::new();
        for d in &ext.devices {
            used_set.insert(d.gate); used_set.insert(d.source); used_set.insert(d.drain);
        }
        ext.used_nets = used_set.len();

        if ext.devices.len() == before && !merged_any_series { break; }
    }
}

// --- main extraction pipeline ---

pub fn extract_netlist(store: &GeometryStore, deck: &Deck) -> Result<ExtractedNetlist, String> {
    extract_netlist_opts(store, deck, &ExtractOpts::default(), Backend::Cpu)
}

pub fn extract_netlist_opts(
    store: &GeometryStore, deck: &Deck, opts: &ExtractOpts, backend: Backend,
) -> Result<ExtractedNetlist, String> {
    let n = store.poly_count();
    let (conductors, vias) = resolve_connectivity(deck)?;
    let is_conn = |l: LayerId| conductors.contains(&l) || vias.contains(&l);

    // Derive gate layers and channel layers from MOS rules (no hardcoded "poly"/"diff")
    let mut gate_layers: Vec<LayerId> = deck.devices.mos_rules.iter()
        .map(|r| r.gate_layer).collect();
    gate_layers.sort_unstable();
    gate_layers.dedup();
    let mut channel_layers: Vec<LayerId> = deck.devices.mos_rules.iter()
        .map(|r| r.channel_layer).collect();
    channel_layers.sort_unstable();
    channel_layers.dedup();

    // Build connectivity nodes
    let mut nodes: Vec<Node> = Vec::new();
    let mut node_of_poly: Vec<u32> = vec![u32::MAX; n];
    let mut first_seg_of_poly: Vec<u32> = vec![u32::MAX; n];
    let mut splits: Vec<DiffSplit> = Vec::new();

    for i in 0..n as u32 {
        let li = store.poly_layer[i as usize];
        if channel_layers.contains(&li) { continue; }
        if !is_conn(li) { continue; }
        node_of_poly[i as usize] = nodes.len() as u32;
        nodes.push(Node { bbox: store.poly_bbox[i as usize], layer: li, is_diff_seg: false });
    }

    // Split each channel-layer polygon at its gate crossings
    for &ch_l in &channel_layers {
        for d in store.polys_on_layer(ch_l) {
            let db = store.poly_bbox[d.0 as usize];
            let gates_all: Vec<u32> = gate_layers.iter().flat_map(|&gl| {
                store.polys_on_layer(gl).into_iter()
                    .filter(|&g| poly_poly_overlap(store, g, d))
                    .map(|g| g.0)
            }).collect();

            let crosses_vertically = |g: u32| {
                let gb = store.poly_bbox[g as usize];
                (gb.ymin <= db.ymin && gb.ymax >= db.ymax) || gb.height() >= db.height()
            };
            let n_vert = gates_all.iter().filter(|&&g| crosses_vertically(g)).count();
            let axis_x = n_vert * 2 >= gates_all.len();
            let (d_lo, d_hi) = if axis_x { (db.xmin, db.xmax) } else { (db.ymin, db.ymax) };
            let mut gspans: Vec<GateSpan> = gates_all.iter().copied()
                .filter(|&g| crosses_vertically(g) == axis_x)
                .map(|g| {
                    let gb = store.poly_bbox[g as usize];
                    let (lo, hi) = if axis_x {
                        (gb.xmin.max(d_lo), gb.xmax.min(d_hi))
                    } else {
                        (gb.ymin.max(d_lo), gb.ymax.min(d_hi))
                    };
                    GateSpan { gate: g, lo, hi }
                })
                .collect();
            gspans.sort_by_key(|s| (s.lo, s.hi));

            let mut seg_spans: Vec<(i32, i32)> = Vec::new();
            let mut cursor = d_lo;
            for s in &gspans {
                if s.lo > cursor { seg_spans.push((cursor, s.lo)); }
                cursor = cursor.max(s.hi);
            }
            if d_hi > cursor { seg_spans.push((cursor, d_hi)); }

            let mut seg_nodes = Vec::with_capacity(seg_spans.len());
            for &(lo, hi) in &seg_spans {
                let bb = if axis_x {
                    Bbox { xmin: lo, xmax: hi, ymin: db.ymin, ymax: db.ymax }
                } else {
                    Bbox { xmin: db.xmin, xmax: db.xmax, ymin: lo, ymax: hi }
                };
                seg_nodes.push(nodes.len() as u32);
                nodes.push(Node { bbox: bb, layer: ch_l, is_diff_seg: true });
            }
            if let Some(&f) = seg_nodes.first() {
                first_seg_of_poly[d.0 as usize] = f;
            }
            splits.push(DiffSplit { diff: d.0, axis_x, seg_nodes, seg_spans, gates: gspans });
        }
    }

    // Union-find over nodes
    let intra_touch = deck.intra_layer_touch;
    let can_union = |a: &Node, b: &Node| -> bool {
        let gate_pair = (gate_layers.contains(&a.layer) && b.is_diff_seg)
            || (gate_layers.contains(&b.layer) && a.is_diff_seg);
        if gate_pair { return false; }
        if !opts.cut_required { return true; }
        if a.layer == b.layer { return true; }
        let bridges = |via: &Node, other: &Node| -> bool {
            for &(vid, ref connects) in &deck.connectivity.vias {
                if via.layer == vid {
                    return connects.contains(&other.layer) || (other.is_diff_seg && connects.iter().any(|&c| {
                        channel_layers.contains(&c) || c == other.layer
                    }));
                }
            }
            false
        };
        bridges(a, b) || bridges(b, a)
    };

    let m = nodes.len();
    let mut uf = UnionFind::new(m);

    let n_pairs = m * (m.saturating_sub(1)) / 2;
    let gpu_flags = if backend == Backend::Gpu && n_pairs >= (1 << 18) {
        let xmins: Vec<i32> = nodes.iter().map(|n| n.bbox.xmin).collect();
        let ymins: Vec<i32> = nodes.iter().map(|n| n.bbox.ymin).collect();
        let xmaxs: Vec<i32> = nodes.iter().map(|n| n.bbox.xmax).collect();
        let ymaxs: Vec<i32> = nodes.iter().map(|n| n.bbox.ymax).collect();
        let mut pa = Vec::with_capacity(n_pairs);
        let mut pb = Vec::with_capacity(n_pairs);
        for i in 0..m { for j in (i + 1)..m { pa.push(i as u32); pb.push(j as u32); } }
        crate::traits::bbox_overlap_flags(&xmins, &ymins, &xmaxs, &ymaxs, &pa, &pb)
    } else {
        None
    };

    let mut pair_idx = 0usize;
    for i in 0..m {
        for j in (i + 1)..m {
            let overlaps = match &gpu_flags {
                Some(flags) => { let r = flags[pair_idx] != 0; pair_idx += 1; r }
                None => {
                    // Same-layer touching edges connect when intra_layer_touch is on
                    if intra_touch && nodes[i].layer == nodes[j].layer {
                        overlap_touch(nodes[i].bbox, nodes[j].bbox)
                    } else {
                        overlap_pos(nodes[i].bbox, nodes[j].bbox)
                    }
                }
            };
            if !overlaps { continue; }
            if !can_union(&nodes[i], &nodes[j]) { continue; }
            uf.union(i as u32, j as u32);
        }
    }

    // Assign compact net ids
    let mut root_to_net: HashMap<u32, u32> = HashMap::new();
    let mut net_of_node = vec![u32::MAX; m];
    for i in 0..m as u32 {
        let r = uf.find(i);
        let next = root_to_net.len() as u32;
        net_of_node[i as usize] = *root_to_net.entry(r).or_insert(next);
    }
    let net_count = root_to_net.len();

    let mut net_of_poly = vec![u32::MAX; n];
    for i in 0..n {
        if node_of_poly[i] != u32::MAX {
            net_of_poly[i] = net_of_node[node_of_poly[i] as usize];
        } else if first_seg_of_poly[i] != u32::MAX {
            net_of_poly[i] = net_of_node[first_seg_of_poly[i] as usize];
        }
    }

    // Device extraction
    let mut devices = Vec::new();
    for sp in &splits {
        let db = store.poly_bbox[sp.diff as usize];
        for gs in &sp.gates {
            let gate_net = net_of_poly[gs.gate as usize];
            if gate_net == u32::MAX { continue; }
            let mut src: Option<u32> = None;
            let mut drn: Option<u32> = None;
            for (k, &(s0, s1)) in sp.seg_spans.iter().enumerate() {
                if s1 <= gs.lo { src = Some(sp.seg_nodes[k]); }
                if drn.is_none() && s0 >= gs.hi { drn = Some(sp.seg_nodes[k]); }
            }
            if let (Some(s), Some(dd)) = (src, drn) {
                let gb = store.poly_bbox[gs.gate as usize];
                let channel = intersect_bbox(gb, db);
                let mos_match = pick_type_and_flavor(store, deck, channel)?;
                let matched_rule = &deck.devices.mos_rules[mos_match.rule_idx];
                let l = gs.hi - gs.lo;
                let w = if sp.axis_x {
                    db.ymax.min(gb.ymax) - db.ymin.max(gb.ymin)
                } else {
                    db.xmax.min(gb.xmax) - db.xmin.max(gb.xmin)
                };
                // Phase 3A: body/well extraction
                let body = if let Some(well_layer) = matched_rule.well_layer {
                    let chan_bb = channel;
                    store.polys_on_layer(well_layer).into_iter()
                        .find(|&wp| store.poly_bbox[wp.0 as usize].overlaps(&chan_bb))
                        .map(|wp| net_of_poly[wp.0 as usize])
                        .unwrap_or(0)
                } else {
                    0
                };
                // Phase 3C: DMOS class tag
                let device_class = matched_rule.device_class.clone();
                devices.push(Device {
                    kind: mos_match.kind, gate: gate_net,
                    source: net_of_node[s as usize], drain: net_of_node[dd as usize],
                    body, flavor: mos_match.flavor, w, l, device_class,
                });
            }
        }
    }

    // Phase 4B: Global net merging — force all polygons labelled with a global net name
    // to share a single net ID.
    if !deck.global_nets.is_empty() && !store.net_labels.is_empty() {
        for global_name in &deck.global_nets {
            let mut matching_nets: Vec<u32> = Vec::new();
            for (poly_idx, label) in &store.net_labels {
                if label == global_name {
                    let net = net_of_poly[*poly_idx as usize];
                    if net != u32::MAX {
                        matching_nets.push(net);
                    }
                }
            }
            if matching_nets.len() > 1 {
                let canonical = matching_nets[0];
                for &other in &matching_nets[1..] {
                    if other != canonical {
                        // Remap all occurrences of `other` to `canonical`
                        for np in net_of_poly.iter_mut() {
                            if *np == other { *np = canonical; }
                        }
                    }
                }
            }
        }
    }

    let two_terminal = extract_two_terminal_devices(store, deck, &net_of_poly);

    // Phase 3B: BJT device extraction
    let bjt_devices = extract_bjt_devices(store, deck, &net_of_poly);

    let mut used = HashSet::new();
    for d in &devices {
        used.insert(d.gate); used.insert(d.source); used.insert(d.drain);
    }

    // Label conflict detection
    let mut label_conflicts: Vec<String> = Vec::new();
    if !store.net_labels.is_empty() {
        let mut net_to_label: HashMap<u32, &str> = HashMap::new();
        for (poly_idx, label) in &store.net_labels {
            let net = net_of_poly[*poly_idx as usize];
            if net == u32::MAX { continue; }
            if let Some(existing) = net_to_label.get(&net) {
                if *existing != label.as_str() {
                    label_conflicts.push(format!(
                        "net {} has conflicting labels: '{}' vs '{}'", net, existing, label
                    ));
                }
            } else {
                net_to_label.insert(net, label.as_str());
            }
        }
    }

    let mut ext = ExtractedNetlist {
        devices, net_count, used_nets: used.len(), net_of_poly, label_conflicts, two_terminal,
        bjt_devices, floating_nets: Vec::new(),
    };
    reduce_netlist(&mut ext);

    // Phase 4C: Floating net detection — nets with polygons but no device terminal connections
    {
        let mut terminal_nets: HashSet<u32> = HashSet::new();
        for d in &ext.devices {
            terminal_nets.insert(d.gate);
            terminal_nets.insert(d.source);
            terminal_nets.insert(d.drain);
            if d.body != 0 { terminal_nets.insert(d.body); }
        }
        for d in &ext.two_terminal {
            terminal_nets.insert(d.terminal_a);
            terminal_nets.insert(d.terminal_b);
        }
        for d in &ext.bjt_devices {
            terminal_nets.insert(d.collector);
            terminal_nets.insert(d.base);
            terminal_nets.insert(d.emitter);
        }
        // Count polygons per net and find label for each net
        let mut net_poly_count: HashMap<u32, usize> = HashMap::new();
        let mut net_label: HashMap<u32, String> = HashMap::new();
        for (i, &net) in ext.net_of_poly.iter().enumerate() {
            if net == u32::MAX { continue; }
            *net_poly_count.entry(net).or_default() += 1;
            if let Some(label) = store.net_labels.get(&(i as u32)) {
                net_label.entry(net).or_insert_with(|| label.clone());
            }
        }
        for (&net, &count) in &net_poly_count {
            if !terminal_nets.contains(&net) {
                ext.floating_nets.push(FloatingNet {
                    net_id: net,
                    label: net_label.get(&net).cloned(),
                    polygon_count: count,
                });
            }
        }
    }

    Ok(ext)
}
