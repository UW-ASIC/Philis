//! In-process signoff of a Substrate block through GPurify (`verify`).
//!
//! [`check`] takes the block's exported LayIR library (in its PDK's layer
//! type), the PDK's layer → GDS map, its SCIR netlist converted to the
//! [`Spice`] schema, and a `verify` [`Pdk`]; it runs DRC/ERC/LVS/PEX in one
//! pass and returns `verify`'s [`Signoff`].
//!
//! - **Layout** is flattened, and each shape goes to the deck layer whose GDS
//!   pair it carries. A shape on a pair the deck does not draw is not checked:
//!   it is a [`Coverage::skipped_rules`](verify::Coverage) row
//!   `layout.unmapped_layer`, never dropped silently.
//! - **Pins**: one label per top-cell port, at the centre of the port's first
//!   shape, on that shape's layer.
//! - **Reference**: the SCIR hierarchy is flattened to cards. A device's kind
//!   comes from the deck's recogniser for its model ([`Pdk::model_kind`]); a
//!   MOS goes in as `nf·m` cards of per-finger `w/nf`, as Philis's own
//!   reference does.

use std::collections::{HashMap, HashSet};

use gdsconv::GdsLayer;
use layir::Element;
use pnr_core::{LayerId, Rect, Shape};
use scir::{ChildId, ParamValue, SignalId};
use spice::{Primitive, Spice};
use substrate::geometry::bbox::Bbox;
use substrate::geometry::transform::{TransformRef, Transformation};
use verify::{Intent, LabeledPin, Pdk, RefDeviceIn, RefInput, RefKind, Signoff};

/// Why a block could not be handed to the checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The library has no cell, or several, that no other cell instantiates.
    Top(usize),
    /// A non-rectangular shape, on this GDS layer (the checker takes rects).
    Polygon(GdsLayer),
    /// A device model the deck has no recogniser for.
    UnknownModel(String),
    /// A SCIR primitive this adapter does not translate.
    Primitive(String),
    /// A port or instance connection the netlist leaves unresolved.
    Netlist(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Top(n) => write!(f, "expected one top cell, found {n}"),
            Self::Polygon(l) => write!(f, "non-rectangular shape on GDS layer {l}"),
            Self::UnknownModel(m) => write!(f, "no deck recogniser for model `{m}`"),
            Self::Primitive(p) => write!(f, "unsupported primitive: {p}"),
            Self::Netlist(s) => write!(f, "netlist: {s}"),
        }
    }
}

impl std::error::Error for Error {}

/// Full signoff of `layout` against `schematic` on `pdk`. `gds` maps a layout
/// layer to its GDS drawing pair (sky130: `Sky130Layer::gds_layer`).
///
/// # Errors
/// See [`Error`]: anything that would make the check incomplete fails here
/// rather than reading as clean.
pub fn check<L: Clone>(
    layout: &layir::Library<L>,
    gds: impl Fn(&L) -> GdsLayer,
    schematic: &scir::Library<Spice>,
    pdk: &Pdk,
) -> Result<Signoff, Error> {
    let (shapes, pins, unmapped) = geometry(layout, &gds, pdk)?;
    let mut reference = reference(schematic, pdk)?;
    // The external ports are the schematic's; every label must name a net.
    reference.external_ports = Some(reference.ports.clone());
    reference.ports = pins.iter().map(|p| p.name.clone()).collect();
    let mut s = verify::signoff_checked(&shapes, &pins, &reference, &Intent::default(), pdk);
    for (l, n) in unmapped {
        s.coverage.skipped_rules.push(("layout.unmapped_layer".into(), format!("GDS {l}: {n} shapes not checked")));
    }
    Ok(s)
}

/// Unmapped GDS pairs and how many shapes each had.
type Unmapped = Vec<(GdsLayer, usize)>;

/// Flat deck-layer rects, port labels, and what no deck layer drew.
fn geometry<L: Clone>(
    lib: &layir::Library<L>,
    gds: &impl Fn(&L) -> GdsLayer,
    pdk: &Pdk,
) -> Result<(Vec<Shape>, Vec<LabeledPin>, Unmapped), Error> {
    let children: HashSet<_> = lib.cells().flat_map(|(_, c)| c.instances().map(|(_, i)| i.child())).collect();
    let tops: Vec<_> = lib.cells().filter(|(id, _)| !children.contains(id)).map(|(id, _)| id).collect();
    let [top] = tops[..] else { return Err(Error::Top(tops.len())) };

    let table = pdk.layer_gds();
    let deck_layer = |g: GdsLayer| {
        table.iter().position(|&pair| pair == (g.0, g.1) && pair != (0, 0)).map(|i| LayerId(i as u16))
    };

    let mut flat = Vec::new();
    flatten(lib, top, Transformation::identity(), &mut flat);
    let mut shapes = Vec::with_capacity(flat.len());
    let mut unmapped: HashMap<GdsLayer, usize> = HashMap::new();
    for s in &flat {
        let g = gds(s.layer());
        let Some(layer) = deck_layer(g) else {
            *unmapped.entry(g).or_default() += 1;
            continue;
        };
        let r = s.shape().rect().ok_or(Error::Polygon(g))?;
        shapes.push(Shape { layer, rect: rect(r) });
    }

    let mut pins = Vec::new();
    for (name, port) in lib.cell(top).ports() {
        let Some(s) = port.elements().find_map(|e| match e {
            Element::Shape(s) => Some(s),
            Element::Text(_) => None,
        }) else {
            continue;
        };
        let g = gds(s.layer());
        let Some(layer) = deck_layer(g) else {
            return Err(Error::Netlist(format!("port `{name}` is on GDS {g}, which no deck layer draws")));
        };
        let c = s.shape().bbox_rect().center();
        pins.push(LabeledPin { name: name.to_string(), layer: layer.0, x: c.x as i32, y: c.y as i32 });
    }

    let mut unmapped: Unmapped = unmapped.into_iter().collect();
    unmapped.sort_by_key(|(g, _)| (g.0, g.1));
    Ok((shapes, pins, unmapped))
}

fn flatten<L: Clone>(lib: &layir::Library<L>, id: layir::CellId, t: Transformation, out: &mut Vec<layir::Shape<L>>) {
    let cell = lib.cell(id);
    for e in cell.elements() {
        if let Element::Shape(s) = e {
            out.push(s.transform_ref(t));
        }
    }
    for (_, inst) in cell.instances() {
        flatten(lib, inst.child(), Transformation::cascade(t, inst.transformation()), out);
    }
}

fn rect(r: substrate::geometry::rect::Rect) -> Rect {
    Rect { x: r.left() as i32, y: r.bot() as i32, w: (r.right() - r.left()) as i32, h: (r.top() - r.bot()) as i32 }
}

/// One bit of a SCIR signal: `None` for a scalar.
type Bit = (SignalId, Option<usize>);

/// The flat LVS reference of `lib`'s top cell, with its port bits as `ports`.
fn reference(lib: &scir::Library<Spice>, pdk: &Pdk) -> Result<RefInput, Error> {
    let top = lib.top_cell().ok_or(Error::Top(0))?;
    let cell = lib.cell(top);
    let mut ports = Vec::new();
    for p in cell.ports() {
        let info = cell.signal(p.signal());
        ports.extend(bits(info).map(|b| bit_name("", info, b.1)));
    }
    let mut devices = Vec::new();
    walk(lib, top, "", &HashMap::new(), pdk, &mut devices)?;
    Ok(RefInput { devices, ports, external_ports: None })
}

fn bits(info: &scir::SignalInfo) -> impl Iterator<Item = Bit> + '_ {
    let idx: Vec<Option<usize>> = match info.width {
        Some(w) => (0..w).map(Some).collect(),
        None => vec![None],
    };
    idx.into_iter().map(move |i| (info.id, i))
}

fn bit_name(prefix: &str, info: &scir::SignalInfo, i: Option<usize>) -> String {
    match i {
        Some(i) => format!("{prefix}{}[{i}]", info.name),
        None => format!("{prefix}{}", info.name),
    }
}

/// Expand `cell`'s instances into `out`. `bound` names the bits the parent
/// connected; every other bit is a local net under `prefix`.
fn walk(
    lib: &scir::Library<Spice>,
    id: scir::CellId,
    prefix: &str,
    bound: &HashMap<Bit, String>,
    pdk: &Pdk,
    out: &mut Vec<RefDeviceIn>,
) -> Result<(), Error> {
    let cell = lib.cell(id);
    let net = |b: Bit| bound.get(&b).cloned().unwrap_or_else(|| bit_name(prefix, cell.signal(b.0), b.1));
    // The parent-side nets of one instance connection, bit by bit.
    let conn = |inst: &scir::Instance, port: &str| -> Result<Vec<String>, Error> {
        let c = inst.connections().get(port).ok_or_else(|| Error::Netlist(format!("{prefix}{}: port `{port}` unconnected", inst.name())))?;
        Ok(c.parts()
            .flat_map(|s| match s.range() {
                Some(r) => r.indices().map(|i| (s.signal(), Some(i))).collect::<Vec<_>>(),
                None => vec![(s.signal(), None)],
            })
            .map(net)
            .collect())
    };
    for (_, inst) in cell.instances() {
        match inst.child() {
            ChildId::Cell(child_id) => {
                let child = lib.cell(child_id);
                let mut child_bound = HashMap::new();
                for p in child.ports() {
                    let info = child.signal(p.signal());
                    let parent = conn(inst, &info.name)?;
                    let own: Vec<Bit> = bits(info).collect();
                    if own.len() != parent.len() {
                        return Err(Error::Netlist(format!("{prefix}{}.{}: width {} connected to {}", inst.name(), info.name, own.len(), parent.len())));
                    }
                    child_bound.extend(own.into_iter().zip(parent));
                }
                walk(lib, child_id, &format!("{prefix}{}.", inst.name()), &child_bound, pdk, out)?;
            }
            ChildId::Primitive(pid) => {
                let one = |port: &str| -> Result<String, Error> {
                    let mut v = conn(inst, port)?;
                    if v.len() != 1 {
                        return Err(Error::Netlist(format!("{prefix}{}.{port}: {} bits on a primitive terminal", inst.name(), v.len())));
                    }
                    Ok(v.remove(0))
                };
                let (model, ports, params): (Option<&str>, Vec<&str>, Option<&HashMap<_, ParamValue>>) = match lib.primitive(pid) {
                    Primitive::RawInstance { ports, cell, params } | Primitive::RawInstanceWithCell { ports, cell, params, .. } => {
                        (Some(cell.as_str()), ports.iter().map(|p| p.as_str()).collect(), Some(params))
                    }
                    Primitive::Mos { model, params } => (Some(model.as_str()), vec!["D", "G", "S", "B"], Some(params)),
                    Primitive::Diode2 { model, params } => (Some(model.as_str()), vec!["1", "2"], Some(params)),
                    Primitive::Bjt { model, params, .. } => (Some(model.as_str()), vec!["NC", "NB", "NE"], Some(params)),
                    Primitive::Res2 { params, .. } => (None, vec!["1", "2"], Some(params)),
                    Primitive::Cap2 { .. } => (None, vec!["1", "2"], None),
                    other => return Err(Error::Primitive(format!("{prefix}{}: {other:?}", inst.name()))),
                };
                let kind = match (model, lib.primitive(pid)) {
                    (_, Primitive::Res2 { .. }) => RefKind::Resistor,
                    (_, Primitive::Cap2 { .. }) => RefKind::Capacitor,
                    (Some(m), _) => pdk.model_kind(m).ok_or_else(|| Error::UnknownModel(m.to_string()))?,
                    (None, _) => unreachable!("every modelless primitive is matched above"),
                };
                let terminals = ports.iter().map(|p| one(p)).collect::<Result<Vec<_>, _>>()?;
                let num = |k: &str| {
                    params.and_then(|ps| ps.iter().find(|(n, _)| n.as_ref().eq_ignore_ascii_case(k))).and_then(|(_, v)| match v {
                        ParamValue::Numeric(d) => d.to_string().parse::<f64>().ok(),
                        ParamValue::String(s) => s.parse::<f64>().ok(),
                    })
                };
                let m = num("m").map_or(1, |m| m.round().max(1.0) as usize);
                let (cards, params) = if matches!(kind, RefKind::Nmos | RefKind::Pmos) {
                    match (num("w"), num("l")) {
                        (Some(w), Some(l)) => {
                            let nf = num("nf").map_or(1, |n| n.round().max(1.0) as usize);
                            (nf * m, vec![("w".to_string(), si_length(w) / nf as f64), ("l".to_string(), si_length(l))])
                        }
                        // No size: one card without params, so LVS reports the device.
                        _ => (m, Vec::new()),
                    }
                } else {
                    (m, Vec::new())
                };
                for _ in 0..cards {
                    out.push(RefDeviceIn { kind, model: model.map(str::to_string), terminals: terminals.clone(), params: params.clone() });
                }
            }
        }
    }
    Ok(())
}

/// A SPICE length in metres: `>= 0.01` is bare micrometres (`w=1.2`),
/// smaller is already metres (`w=1.2u`), as Philis's parser reads them.
fn si_length(v: f64) -> f64 {
    if v.abs() >= 0.01 { v * 1e-6 } else { v }
}
