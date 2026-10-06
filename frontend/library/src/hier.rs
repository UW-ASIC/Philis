//! Bottom-up hierarchy (FLOW-11): each eligible sub-circuit definition is
//! solved once, from its first instance, as its own netlist; every outermost
//! instance then enters the parent solve as one fixed-geometry cell whose LVS
//! cards are the child's, renamed per instance.

use pnr_core::{DeviceId, Macro, NetId, Netlist, Pin, Rect, SubcktInst};
use verify::Pdk;

use crate::{metadata, Bias, Config, FlowError, Hierarchy, Macros, RunStats};

/// One sub-circuit definition solved once (FLOW-11).
pub(crate) struct Block {
    pub subckt: String,
    /// Child geometry (cells, rings, routes; no fill) translated so its bbox
    /// corner is the origin; one pin `p{k}` per routed formal port k; `units`,
    /// `dummies`, `drawn` empty; child `keepouts` translated.
    pub mac: Macro,
    /// The child's LVS reference cards (`signoff_inputs(&child, pdk).2`):
    /// child net names, which are the first instance's parent names.
    pub ref_cards: Vec<verify::RefDeviceIn>,
    /// First instance's path and actual port names: the name map's source side.
    pub i0_path: String,
    pub i0_ports: Vec<String>,
    pub metadata: metadata::MetadataReport,
    #[allow(dead_code)] // kept for the report; `RunStats::block_solves` counts blocks
    pub stats: RunStats,
}

/// The LVS side of placed blocks: cards renamed per instance, and the
/// schematic devices they replace.
#[derive(Clone, Default)]
pub(crate) struct BlockRef {
    pub cards: Vec<verify::RefDeviceIn>,
    pub members: Vec<DeviceId>,
}

/// What a parent solve consumes: one cell per outermost block instance, pins
/// bound to its actual nets.
#[derive(Default)]
pub(crate) struct Placed {
    pub cells: Vec<(Vec<DeviceId>, Macro)>,
    pub refs: BlockRef,
}

fn member_ids(nl: &Netlist, i: u32) -> Vec<DeviceId> {
    annotator::hier::devices(nl, i).into_iter().map(|(d, _)| d).collect()
}

/// Definitions to solve, post-order (children first: insts list parents
/// first, so the reverse lists every child before its parent). Eligible:
/// ≥ `min_devices` devices (`annotator::hier::devices`), and every instance
/// has pairwise-distinct port nets and devices touching only its ports or
/// `path/` nets (a shorted port or a global would make one layout wrong for
/// another instance). The top is no instance, so never listed.
pub(crate) fn eligible(nl: &Netlist, min_devices: usize) -> Vec<String> {
    let ok = |j: usize| {
        let x = &nl.insts[j];
        let prefix = format!("{}/", x.path);
        let distinct = x.ports.iter().enumerate().all(|(k, p)| !x.ports[..k].contains(p));
        let ids = member_ids(nl, j as u32);
        let local = ids.iter().flat_map(|d| &nl.devices[d.0 as usize].terminals).all(|(_, n)| x.ports.contains(n) || nl.nets[n.0 as usize].name.starts_with(&prefix));
        distinct && local && ids.len() >= min_devices
    };
    let mut out: Vec<String> = Vec::new();
    for j in (0..nl.insts.len()).rev() {
        let s = &nl.insts[j].subckt;
        if !out.contains(s) && nl.insts.iter().enumerate().filter(|(_, x)| &x.subckt == s).all(|(k, _)| ok(k)) {
            out.push(s.clone());
        }
    }
    out
}

/// Instance `i`'s devices as their own netlist, keeping parent names; ports =
/// `insts[i].ports`; nested insts re-parented (i's children → `None`).
/// Returns (netlist, parent device per child device, parent net per child net).
pub(crate) fn local(nl: &Netlist, i: u32) -> (Netlist, Vec<DeviceId>, Vec<NetId>) {
    let x = &nl.insts[i as usize];
    let devs = member_ids(nl, i);
    let mut nets: Vec<NetId> = Vec::new();
    let map = |n: NetId, nets: &mut Vec<NetId>| {
        NetId(nets.iter().position(|&m| m == n).unwrap_or_else(|| {
            nets.push(n);
            nets.len() - 1
        }) as u16)
    };
    let ports: Vec<NetId> = x.ports.iter().map(|&n| map(n, &mut nets)).collect();
    let devices = devs
        .iter()
        .map(|d| {
            let mut dev = nl.devices[d.0 as usize].clone();
            dev.terminals.iter_mut().for_each(|t| t.1 = map(t.1, &mut nets));
            dev
        })
        .collect();
    // Strict descendants of `i`, in order (parents first stays true).
    let under = |mut k: Option<u32>| {
        while let Some(j) = k {
            if j == i {
                return true;
            }
            k = nl.insts[j as usize].parent;
        }
        false
    };
    let kept: Vec<u32> = (0..nl.insts.len() as u32).filter(|&j| j != i && under(Some(j))).collect();
    let new_of = |j: u32| kept.iter().position(|&k| k == j).map(|p| p as u32);
    let insts = kept
        .iter()
        .map(|&j| {
            let y = &nl.insts[j as usize];
            SubcktInst { path: y.path.clone(), subckt: y.subckt.clone(), parent: y.parent.and_then(new_of), ports: y.ports.iter().map(|&n| map(n, &mut nets)).collect() }
        })
        .collect();
    let device_inst = devs.iter().map(|d| nl.device_inst.get(d.0 as usize).copied().flatten().and_then(new_of)).collect();
    let out = Netlist {
        devices,
        nets: nets.iter().map(|n| nl.nets[n.0 as usize].clone()).collect(),
        ports,
        insts,
        device_inst,
        sources: Vec::new(),
    };
    (out, devs, nets)
}

/// Outermost instances (no ancestor instance also a block) of every
/// definition in `blocks`: one cell each, pin `p{k}` on actual port k, the
/// child's cards renamed to this instance.
pub(crate) fn instantiate(nl: &Netlist, blocks: &[Block]) -> Placed {
    let block = |j: u32| blocks.iter().find(|b| b.subckt == nl.insts[j as usize].subckt);
    let mut out = Placed::default();
    for j in 0..nl.insts.len() as u32 {
        let Some(b) = block(j) else { continue };
        let mut up = nl.insts[j as usize].parent;
        while let Some(a) = up.filter(|&a| block(a).is_none()) {
            up = nl.insts[a as usize].parent;
        }
        if up.is_some() {
            continue;
        }
        let x = &nl.insts[j as usize];
        let members = member_ids(nl, j);
        let mut mac = b.mac.clone();
        for p in &mut mac.pins {
            let k: usize = p.name[1..].parse().expect("block pins are `p{k}`");
            p.net = x.ports[k];
        }
        let ports: Vec<String> = x.ports.iter().map(|n| nl.nets[n.0 as usize].name.clone()).collect();
        out.refs.cards.extend(b.ref_cards.iter().map(|c| {
            let mut c = c.clone();
            c.terminals.iter_mut().for_each(|t| *t = rename(t, b, &x.path, &ports));
            c
        }));
        out.refs.members.extend_from_slice(&members);
        out.cells.push((members, mac));
    }
    out
}

/// Instance `j`'s name for child net `name`: port k → j's actual k;
/// `i0_path/x` → `j.path/x`; else `j.path/name` (synthetic `~c.o.k` nets of
/// drawn cards stay unique per instance).
fn rename(name: &str, b: &Block, j_path: &str, j_ports: &[String]) -> String {
    if let Some(k) = b.i0_ports.iter().position(|p| p == name) {
        return j_ports[k].clone();
    }
    let rest = name.strip_prefix(&b.i0_path).and_then(|r| r.strip_prefix('/')).unwrap_or(name);
    format!("{j_path}/{rest}")
}

impl Bias {
    /// The bias of a child netlist from [`local`]: per-device tables by
    /// `devs`, per-net by `nets`; no summary or op point (the parent's).
    fn restrict(&self, devs: &[DeviceId], nets: &[NetId]) -> Bias {
        fn by<T: Clone>(v: &[T], ids: impl Iterator<Item = usize>) -> Vec<T> {
            if v.is_empty() {
                return Vec::new();
            }
            ids.filter_map(|i| v.get(i).cloned()).collect()
        }
        let d = || devs.iter().map(|d| d.0 as usize);
        Bias {
            power: by(&self.power, d()),
            summary: None,
            currents: self.currents.as_ref().map(|c| by(c, d())),
            net_headroom_mv: self.net_headroom_mv.as_ref().map(|h| by(h, nets.iter().map(|n| n.0 as usize))),
            gm_us: by(&self.gm_us, d()),
            op: None,
        }
    }
}

/// A child solve's config: the search knobs and models of `cfg`; nothing
/// that names the top (interface, constraints, ESD, performance specs).
fn child_config(cfg: &Config) -> Config {
    Config {
        seed: cfg.seed,
        feedback_iters: cfg.feedback_iters,
        outer_iters: cfg.outer_iters,
        annotation: cfg.annotation.clone(),
        device_power_uw: Vec::new(),
        op: cfg.op.clone(),
        performance: None,
        starts: cfg.starts,
        min_utilization: cfg.min_utilization,
        heat_source_uw: cfg.heat_source_uw,
        size_convention: cfg.size_convention,
        gp_mode: cfg.gp_mode,
        interface: None,
        top: None,
        constraints: None,
        esd: None,
        cold_every: cfg.cold_every,
        warm: cfg.warm,
        max_wall: cfg.max_wall,
        hierarchy: Hierarchy::Flat,
    }
}

/// Solve every [`eligible`] definition once, children first, each from its
/// first instance with its nested blocks already placed.
pub(crate) fn solve_blocks(nl: &Netlist, pdk: &Pdk, cfg: &Config, bias: &Bias, min_devices: usize) -> Result<Vec<Block>, FlowError> {
    let cc = child_config(cfg);
    let mut blocks: Vec<Block> = Vec::new();
    for subckt in eligible(nl, min_devices) {
        let i0 = nl.insts.iter().position(|x| x.subckt == subckt).expect("eligible names an instance") as u32;
        let (n_s, devs, nets) = local(nl, i0);
        let placed = instantiate(&n_s, &blocks);
        let mut child = crate::solve(&n_s, pdk, &Macros::default(), &cc, bias.restrict(&devs, &nets), &placed)?;
        let ref_cards = crate::signoff_inputs(&child, pdk).2.devices;
        let keep = child.macros.len() - usize::from(child.metadata.post_fill);
        child.macros.truncate(keep);
        let mut mac = Macro { shapes: child.geometry(), ..Default::default() };
        let placed_cells = pnr_core::place_macros(&child.macros, &child.layout);
        mac.keepouts = placed_cells.iter().flat_map(|m| m.keepouts.iter().cloned()).collect();
        let metals = pdk.routing_layers();
        for (k, &port) in n_s.ports.iter().enumerate() {
            // Widest wire on the highest routed layer the port uses, else a cell pin.
            let wires = child.routes.wires.get(port.0 as usize).map_or(&[][..], Vec::as_slice);
            let top = wires.iter().filter_map(|s| metals.iter().position(|&l| l == s.layer)).max();
            let wire = top.and_then(|t| wires.iter().filter(|s| s.layer == metals[t]).max_by_key(|s| (s.rect.w.min(s.rect.h), s.rect.w.max(s.rect.h))));
            let at = wire.map(|s| (s.rect, s.layer)).or_else(|| placed_cells.iter().flat_map(|m| &m.pins).find(|p| p.net == port).map(|p| (p.at, p.layer)));
            if let Some((at, layer)) = at {
                mac.pins.push(Pin { name: format!("p{k}"), net: port, at, layer });
            }
        }
        let (x0, y0) = mac.shapes.iter().fold((i32::MAX, i32::MAX), |a, s| (a.0.min(s.rect.x), a.1.min(s.rect.y)));
        let (x1, y1) = mac.shapes.iter().fold((i32::MIN, i32::MIN), |a, s| (a.0.max(s.rect.x + s.rect.w), a.1.max(s.rect.y + s.rect.h)));
        let shift = |r: &mut Rect| (r.x, r.y) = (r.x - x0, r.y - y0);
        mac.shapes.iter_mut().for_each(|s| shift(&mut s.rect));
        mac.pins.iter_mut().for_each(|p| shift(&mut p.at));
        mac.keepouts.iter_mut().for_each(|k| shift(&mut k.rect));
        mac.bbox = Rect { x: 0, y: 0, w: (x1 - x0).max(0), h: (y1 - y0).max(0) };
        let x = &nl.insts[i0 as usize];
        blocks.push(Block {
            subckt,
            mac,
            ref_cards,
            i0_path: x.path.clone(),
            i0_ports: x.ports.iter().map(|n| nl.nets[n.0 as usize].name.clone()).collect(),
            metadata: child.metadata,
            stats: child.stats,
        });
    }
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_OTA: &str = include_str!("../../../benchmarks/fixtures/two_ota.spice");

    #[test]
    fn eligible_is_post_order_and_skips_shorted_ports() {
        let deck = |extra: &str| {
            format!(
                ".subckt inner a b c\nM1 a b c c nfet W=1u L=1u\nM2 c b a a nfet W=1u L=1u\n.ends inner\n\
                 .subckt outer p q r\nXI p q r inner\nM3 p q r r nfet W=1u L=1u\n.ends outer\n\
                 .subckt top x y z w\nXO x y z outer\n{extra}.ends top\n"
            )
        };
        let nl = crate::parse(&deck("")).unwrap();
        assert_eq!(eligible(&nl, 2), ["inner", "outer"]);
        assert!(!eligible(&nl, 4).contains(&"inner".to_string()), "inner has 2 devices");
        let shorted = crate::parse(&deck("XS w w z inner\n")).unwrap();
        assert!(!eligible(&shorted, 2).contains(&"inner".to_string()), "XS shorts two inner ports");
    }

    #[test]
    fn rename_maps_ports_internals_and_synthetics() {
        let b = Block {
            subckt: "s".into(),
            mac: Macro::default(),
            ref_cards: Vec::new(),
            i0_path: "X1".into(),
            i0_ports: vec!["a".into(), "b".into()],
            metadata: Default::default(),
            stats: RunStats::default(),
        };
        let j = ["c".to_string(), "d".to_string()];
        assert_eq!(rename("a", &b, "X2", &j), "c");
        assert_eq!(rename("b", &b, "X2", &j), "d");
        assert_eq!(rename("X1/n", &b, "X2", &j), "X2/n");
        assert_eq!(rename("~3.0.1", &b, "X2", &j), "X2/~3.0.1");
    }

    #[test]
    fn local_keeps_parent_names_and_ports() {
        let nl = crate::parse(TWO_OTA).unwrap();
        let i = nl.insts.iter().position(|x| x.path == "X1").unwrap() as u32;
        let (n, devs, nets) = local(&nl, i);
        assert_eq!(n.devices.len(), 5);
        assert_eq!(devs.len(), 5);
        let name = |l: &Netlist, p: &NetId| l.nets[p.0 as usize].name.clone();
        let want: Vec<String> = nl.insts[i as usize].ports.iter().map(|p| name(&nl, p)).collect();
        assert_eq!(n.ports.iter().map(|p| name(&n, p)).collect::<Vec<_>>(), want);
        assert!(n.nets.iter().any(|x| x.name == "X1/vtail"));
        for (k, net) in n.nets.iter().enumerate() {
            assert_eq!(nl.nets[nets[k].0 as usize].name, net.name);
        }
        for (k, d) in n.devices.iter().enumerate() {
            assert_eq!(d.name, nl.devices[devs[k].0 as usize].name);
            assert!(d.name.starts_with("X1/"));
        }
        assert!(n.insts.is_empty() && n.device_inst.iter().all(Option::is_none));
    }
}
