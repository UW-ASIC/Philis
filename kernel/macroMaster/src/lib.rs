//! # `macroMaster` — hand-authored, PDK-agnostic macros
//!
//! Two authoring tiers:
//!
//! - [`DeviceGen`] draws raw geometry through a [`DeviceBuilder`]. The shipped
//!   ones in [`variants`] take their geometry from the `cells` generators.
//! - [`Composition`] instantiates Devices (or sub-compositions) through a
//!   [`CompBuilder`], places them relative to each other ([`CompBuilder::place`]
//!   rejects overlap) and declares nets with [`CompBuilder::connect`].
//!
//! [`build_composition`] / [`build_with`] turn a composition into a
//! [`BuiltComp`]: placed macros with net-bound pins plus, when every instance
//! declares its devices, the schematic [`Netlist`]. Routing and signoff are the
//! caller's (`library::elaborate`).

mod adapter;

pub use pnr_core::Process;

use std::collections::{HashMap, HashSet};

use pnr_core::{Device, DeviceKind, Dir, LayerId, Macro, Net, NetId, Netlist, Orient, Pin, Rect};

// ===========================================================================
//  Typed IO
// ===========================================================================

/// A single-bit signal.
#[derive(Clone, Copy, Default, Debug)]
pub struct Signal;

/// An input port.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input<T>(pub T);

/// An output port.
#[derive(Clone, Copy, Default, Debug)]
pub struct Output<T>(pub T);

/// A bidirectional port (power, body, analog nets).
#[derive(Clone, Copy, Default, Debug)]
pub struct InOut<T>(pub T);

/// A named, directioned terminal of a block.
#[derive(Clone, Debug)]
pub struct PortInfo {
    /// Port name; also the net name a connect class containing it takes.
    pub name: String,
    /// Signal direction, as reported to the netlist.
    pub dir: Dir,
}

impl Input<Signal> {
    /// This port as an input named `name`.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::In }
    }
}
impl Output<Signal> {
    /// This port as an output named `name`.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::Out }
    }
}
impl InOut<Signal> {
    /// This port as a bidirectional terminal named `name`.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::InOut }
    }
}

/// A typed port bundle.
pub trait Io: Default {
    /// The terminals, in a stable order.
    fn ports(&self) -> Vec<PortInfo>;
}

// ===========================================================================
//  Generator traits
// ===========================================================================

/// A generator's parameter struct; implement [`DeviceGen`] or [`Composition`].
pub trait Block {
    /// The block's port bundle; its [`Io::ports`] order is the net-naming
    /// priority in [`build_composition`].
    type Io: Io;

    /// Cell name, used in reports.
    fn name(&self) -> String;

    /// The port bundle; the default is `Self::Io::default()`.
    fn io(&self) -> Self::Io {
        Self::Io::default()
    }
}

/// A leaf that draws raw geometry.
pub trait DeviceGen: Block {
    /// Draws the device at the origin through `cell`.
    ///
    /// # Errors
    /// [`GenError::OffGrid`] when a shape or pin is off the process grid.
    fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError>;

    /// The schematic devices this generator draws. `None` (opaque) makes the
    /// whole [`BuiltComp::netlist`] `None`: a partial netlist would make LVS
    /// report the gap rather than the layout.
    fn devices(&self) -> Option<Vec<GenDevice>> {
        None
    }
}

/// One schematic device a [`DeviceGen`] draws.
#[derive(Clone, Debug)]
pub struct GenDevice {
    /// Schematic device class.
    pub kind: DeviceKind,
    /// `(schematic terminal, generator port)`, e.g. `("D", "d1")`.
    pub terminals: Vec<(String, String)>,
    /// Parameters in nm / counts, keyed as the netlist parser keys them.
    pub params: Vec<(String, i64)>,
}

/// Assembles Devices; cannot draw raw geometry.
pub trait Composition: Block {
    /// Instantiates, places and connects the children through `cell`.
    ///
    /// # Errors
    /// Whatever a [`CompBuilder`] call returns; the build stops at the first.
    fn build<P: Process>(&self, cell: &mut CompBuilder<P>) -> Result<(), GenError>;
}

/// Failure while drawing or placing.
#[derive(Debug, PartialEq, Eq)]
pub enum GenError {
    /// A coordinate is off the fabrication grid.
    OffGrid,
    /// A placed instance's bbox overlaps an already-placed one.
    Overlap,
    /// [`CompBuilder::place_mirrored`] would reverse the named instance's net
    /// current direction (an odd-finger device): use
    /// [`CompBuilder::place_copy`].
    Orientation(String),
    /// Two placed instances share this name.
    DuplicateName(String),
    /// A connect endpoint that is neither an io port nor a pin or device
    /// terminal of a placed instance.
    UnknownTerminal(String),
}

// ===========================================================================
//  Relative placement
// ===========================================================================

/// How to align an instance's bbox against a reference's. The abut modes
/// (`ToTheRight`/`ToTheLeft`/`Above`/`Beneath`) add `offset` along the primary
/// axis; the flush/centre modes nudge by `offset` along the free axis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlignMode {
    /// Left edges flush; `offset` moves up.
    Left,
    /// Right edges flush; `offset` moves up.
    Right,
    /// Bottom edges flush; `offset` moves right.
    Bottom,
    /// Top edges flush; `offset` moves right.
    Top,
    /// Vertical centres equal (rounded toward −∞ of each half height);
    /// `offset` moves right.
    CenterHorizontal,
    /// Horizontal centres equal; `offset` moves up.
    CenterVertical,
    /// Left edge on the reference's right edge plus `offset`; y unchanged.
    ToTheRight,
    /// Right edge on the reference's left edge minus `offset`; y unchanged.
    ToTheLeft,
    /// Bottom edge on the reference's top edge plus `offset`; x unchanged.
    Above,
    /// Top edge on the reference's bottom edge minus `offset`; x unchanged.
    Beneath,
}

/// An instantiated Device or sub-composition, positioned but not yet (or
/// already) placed.
#[derive(Clone)]
pub struct Instance {
    name: String,
    mac: Macro,
    /// A sub-composition's connect-graph, imported (prefixed) at commit.
    edges: Vec<(String, String)>,
    /// `(name, device)` relative to this instance; `None` = opaque.
    devices: Option<Vec<(String, GenDevice)>>,
}

impl Instance {
    /// The instance's bounding box in the composition frame, nm.
    #[must_use]
    pub fn bbox(&self) -> Rect {
        self.mac.bbox
    }

    /// The instance name: the prefix its pins and devices get.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `inst.term("d")` ⇒ `"m1.d"`, for [`CompBuilder::connect`].
    #[must_use]
    pub fn term(&self, port: &str) -> String {
        format!("{}.{}", self.name, port)
    }

    /// Moves this instance relative to `reference` by [`AlignMode`]; shapes,
    /// pins and unit centres move with the bbox.
    pub fn align(&mut self, mode: AlignMode, reference: &Instance, offset: i32) {
        let (dx, dy) = align_delta(self.bbox(), reference.bbox(), mode, offset);
        self.transform(|r| Rect { x: r.x + dx, y: r.y + dy, ..r });
    }

    /// Applies `o` about the bbox origin: the bbox keeps its lower-left
    /// corner, and each unit's centre and current direction turn with the
    /// geometry.
    pub fn orient(&mut self, o: Orient) {
        let b = self.bbox();
        let nb = o.apply_rect(b);
        let (dx, dy) = (b.x - nb.x, b.y - nb.y);
        self.transform(|r| {
            let r = o.apply_rect(r);
            Rect { x: r.x + dx, y: r.y + dy, ..r }
        });
        for u in &mut self.mac.units {
            let (px, py) = o.apply(i32::from(u.phi.0), i32::from(u.phi.1));
            u.phi = (px as i8, py as i8);
        }
    }

    /// Map bbox, shapes, pins and unit centres (as zero-size rects) by `f`.
    /// Unit directions and dummies are left as they are.
    fn transform(&mut self, f: impl Fn(Rect) -> Rect) {
        self.mac.bbox = f(self.mac.bbox);
        for s in &mut self.mac.shapes {
            s.rect = f(s.rect);
        }
        for p in &mut self.mac.pins {
            p.at = f(p.at);
        }
        for u in &mut self.mac.units {
            let r = f(Rect { x: u.x, y: u.y, w: 0, h: 0 });
            (u.x, u.y) = (r.x, r.y);
        }
        // Keepouts are geometry too: they follow the shapes they protect.
        for k in &mut self.mac.keepouts {
            k.rect = f(k.rect);
        }
    }

    /// Σ of the units' current directions: what a mirror must preserve.
    fn phi_sum(&self) -> (i32, i32) {
        self.mac.units.iter().fold((0, 0), |(x, y), u| (x + i32::from(u.phi.0), y + i32::from(u.phi.1)))
    }
}

/// The `(dx, dy)` that moves `a` into `mode` alignment with `b` (see
/// [`AlignMode`] for what `offset` does per mode).
fn align_delta(a: Rect, b: Rect, mode: AlignMode, offset: i32) -> (i32, i32) {
    match mode {
        AlignMode::Left => (b.x - a.x, offset),
        AlignMode::Right => ((b.x + b.w) - (a.x + a.w), offset),
        AlignMode::Bottom => (offset, b.y - a.y),
        AlignMode::Top => (offset, (b.y + b.h) - (a.y + a.h)),
        AlignMode::CenterHorizontal => (offset, (b.y + b.h / 2) - (a.y + a.h / 2)),
        AlignMode::CenterVertical => ((b.x + b.w / 2) - (a.x + a.w / 2), offset),
        AlignMode::ToTheRight => ((b.x + b.w) - a.x + offset, 0),
        AlignMode::ToTheLeft => (b.x - (a.x + a.w) - offset, 0),
        AlignMode::Above => (0, (b.y + b.h) - a.y + offset),
        AlignMode::Beneath => (0, b.y - (a.y + a.h) - offset),
    }
}

/// Positive-area intersection; a shared edge is not overlap.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Union-find over the connect-graph. A class containing an io port takes
/// that port's name (first in `ports` order), others become `net{k}`. Returns
/// the net names and the terminal → net-index binding.
fn resolve_nets(edges: &[(String, String)], ports: &[String]) -> (Vec<String>, HashMap<String, usize>) {
    let mut idx: HashMap<&str, usize> = HashMap::new();
    let mut terms: Vec<&str> = Vec::new();
    for (a, b) in edges {
        for t in [a.as_str(), b.as_str()] {
            idx.entry(t).or_insert_with(|| {
                terms.push(t);
                terms.len() - 1
            });
        }
    }
    let mut parent: Vec<usize> = (0..terms.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for (a, b) in edges {
        let (ra, rb) = (find(&mut parent, idx[a.as_str()]), find(&mut parent, idx[b.as_str()]));
        parent[ra] = rb;
    }
    let mut class_of_root: HashMap<usize, usize> = HashMap::new();
    let mut n_classes = 0;
    let mut binding: HashMap<String, usize> = HashMap::new();
    for (i, t) in terms.iter().enumerate() {
        let root = find(&mut parent, i);
        let class = *class_of_root.entry(root).or_insert_with(|| {
            n_classes += 1;
            n_classes - 1
        });
        binding.insert((*t).to_string(), class);
    }
    let names = (0..n_classes)
        .map(|i| {
            ports.iter().find(|p| binding.get(*p) == Some(&i)).cloned().unwrap_or_else(|| format!("net{i}"))
        })
        .collect();
    (names, binding)
}

// ===========================================================================
//  Builders
// ===========================================================================

/// The Device drawing surface.
pub struct DeviceBuilder<'a, P: Process> {
    builder: &'a mut cells::Builder,
    process: &'a P,
}

impl<'a, P: Process> DeviceBuilder<'a, P> {
    /// The process the device is drawn against.
    #[must_use]
    pub fn process(&self) -> &P {
        self.process
    }

    /// Draws `r` on `layer`, nm.
    ///
    /// # Errors
    /// [`GenError::OffGrid`] when a corner is off the process grid.
    pub fn draw(&mut self, layer: LayerId, r: Rect) -> Result<(), GenError> {
        check_grid(r, self.process.grid())?;
        self.builder.rect(layer, r);
        Ok(())
    }

    /// Registers a Device-local port pin (`g`, `d`, …); its net is bound when
    /// the composition is built.
    ///
    /// # Errors
    /// [`GenError::OffGrid`] when a corner of `at` is off the process grid.
    pub fn pin(&mut self, name: &str, layer: LayerId, at: Rect) -> Result<(), GenError> {
        check_grid(at, self.process.grid())?;
        self.builder.pin(Pin { name: name.into(), net: NetId(0), at, layer });
        Ok(())
    }
}

/// `Ok` when every corner of `r` is a multiple of `grid`; `grid <= 0`
/// disables the check.
fn check_grid(r: Rect, grid: i32) -> Result<(), GenError> {
    let on = grid <= 0 || [r.x, r.y, r.x + r.w, r.y + r.h].iter().all(|v| v % grid == 0);
    if on { Ok(()) } else { Err(GenError::OffGrid) }
}

/// The Composition surface: instantiate, place, connect. No raw geometry.
pub struct CompBuilder<'a, P: Process> {
    process: &'a P,
    placed: Vec<Instance>,
    edges: Vec<(String, String)>,
}

impl<P: Process> CompBuilder<'_, P> {
    /// The process the composition is built against.
    #[must_use]
    pub fn process(&self) -> &P {
        self.process
    }

    /// Draws a [`DeviceGen`] into an [`Instance`] at the origin, ready to
    /// place. A multi-device generator's devices are named `{name}.{ordinal}`.
    ///
    /// # Errors
    /// Whatever [`DeviceGen::layout`] returns.
    pub fn instantiate<D: DeviceGen>(&mut self, name: &str, dev: &D) -> Result<Instance, GenError> {
        let mut b = cells::Builder::new(self.process.grid());
        dev.layout(&mut DeviceBuilder { builder: &mut b, process: self.process })?;
        // A multi-device generator suffixes each device with its ordinal.
        let devices = dev.devices().map(|ds| {
            let multi = ds.len() > 1;
            ds.into_iter()
                .enumerate()
                .map(|(i, d)| (if multi { i.to_string() } else { String::new() }, d))
                .collect()
        });
        Ok(Instance { name: name.into(), mac: b.finish(), edges: Vec::new(), devices })
    }

    /// Instantiates a sub-composition. Its port-net pins surface under the
    /// port name (`x1.vout`); internal pins stay qualified (`x1.m1.d`), and
    /// its connect-graph is imported at commit.
    ///
    /// # Errors
    /// Whatever building `comp` returns (see [`build_with`]).
    pub fn instantiate_comp<C: Composition>(&mut self, name: &str, comp: &C) -> Result<Instance, GenError> {
        let built = build_composition(comp, self.process)?;
        let mut mac = built.flat;
        for p in &mut mac.pins {
            let net = &built.nets[p.net.0 as usize];
            if built.ports.contains(net) {
                p.name.clone_from(net);
            }
        }
        Ok(Instance { name: name.into(), mac, edges: built.edges, devices: built.devices })
    }

    /// Places `inst` as `reference`'s mirror partner about a vertical axis
    /// `gap / 2` right of `reference`'s right edge, snapped up (toward +∞) to
    /// the grid, bottoms aligned.
    ///
    /// # Errors
    /// [`GenError::Orientation`] when both carry units and the mirror changes
    /// Σ current direction (an odd-finger MOS): the pair would not match.
    /// Otherwise whatever [`CompBuilder::place`] returns.
    pub fn place_mirrored(&mut self, mut inst: Instance, reference: &Instance, gap: i32) -> Result<Instance, GenError> {
        let g = self.process.grid().max(1);
        let (a, b) = (reference.bbox(), inst.bbox());
        // Ceiling onto the grid (toward +∞, also for negative coordinates).
        let axis = -(-(a.x + a.w + gap / 2)).div_euclid(g) * g;
        let (dx, dy) = (a.x - b.x, a.y - b.y);
        inst.transform(|r| Rect { x: 2 * axis - (r.x + dx + r.w), y: r.y + dy, ..r });
        for u in &mut inst.mac.units {
            u.phi.0 = -u.phi.0;
        }
        if !inst.mac.units.is_empty() && !reference.mac.units.is_empty() && inst.phi_sum() != reference.phi_sum() {
            return Err(GenError::Orientation(inst.name));
        }
        self.place(inst)
    }

    /// Places `inst` as a translated copy right of `reference` (bottoms
    /// aligned, `gap` nm between): the matching partner for a device a mirror
    /// would reverse.
    ///
    /// # Errors
    /// Whatever [`CompBuilder::place`] returns.
    pub fn place_copy(&mut self, mut inst: Instance, reference: &Instance, gap: i32) -> Result<Instance, GenError> {
        inst.align(AlignMode::Bottom, reference, 0);
        self.place_by(inst, AlignMode::ToTheRight, reference, gap)
    }

    /// Commits `inst` where it stands and returns the placed handle. A shared
    /// edge with a placed instance is legal.
    ///
    /// # Errors
    /// [`GenError::DuplicateName`] for a name already placed,
    /// [`GenError::OffGrid`] for a bbox off the grid, [`GenError::Overlap`]
    /// for positive-area overlap with any placed instance. Nothing is
    /// committed on error.
    pub fn place(&mut self, inst: Instance) -> Result<Instance, GenError> {
        if self.placed.iter().any(|p| p.name == inst.name) {
            return Err(GenError::DuplicateName(inst.name));
        }
        check_grid(inst.bbox(), self.process.grid())?;
        if self.placed.iter().any(|p| overlaps(p.bbox(), inst.bbox())) {
            return Err(GenError::Overlap);
        }
        for (a, b) in &inst.edges {
            self.edges.push((format!("{}.{a}", inst.name), format!("{}.{b}", inst.name)));
        }
        self.placed.push(inst.clone());
        Ok(inst)
    }

    /// [`Instance::align`] then [`CompBuilder::place`].
    ///
    /// # Errors
    /// Whatever [`CompBuilder::place`] returns.
    pub fn place_by(&mut self, mut inst: Instance, mode: AlignMode, reference: &Instance, offset: i32) -> Result<Instance, GenError> {
        inst.align(mode, reference, offset);
        self.place(inst)
    }

    /// Joins two terminals (`m1.d`, or an io port name) into one net. Names
    /// are checked when the build finishes ([`GenError::UnknownTerminal`]).
    pub fn connect(&mut self, a: &str, b: &str) {
        self.edges.push((a.into(), b.into()));
    }
}

/// A built [`Composition`].
pub struct BuiltComp {
    /// `(instance name, placed macro)` in placement order; pins qualified
    /// (`m1.g`) and net-bound.
    pub instances: Vec<(String, Macro)>,
    /// All shapes and qualified, net-bound pins in one macro.
    pub flat: Macro,
    /// Net names, indexed by [`NetId`].
    pub nets: Vec<String>,
    /// The declared connect edges.
    pub edges: Vec<(String, String)>,
    /// The io port names, in declaration order.
    pub ports: Vec<String>,
    /// The composition as a circuit on this build's [`NetId`]s
    /// (`netlist.nets[i].name == nets[i]`); `None` if any instance is opaque.
    pub netlist: Option<Netlist>,
    /// Devices relative to this build's root, for re-nesting.
    devices: Option<Vec<(String, GenDevice)>>,
}

/// Builds a [`Composition`] against `process` and resolves its nets.
///
/// # Errors
/// See [`build_with`].
pub fn build_composition<C: Composition, P: Process>(comp: &C, process: &P) -> Result<BuiltComp, GenError> {
    let ports: Vec<String> = comp.io().ports().into_iter().map(|p| p.name).collect();
    build_with(process, ports, |c| comp.build(c))
}

/// [`build_composition`] for a build script that is data (dynamic ports and
/// body), not a type.
///
/// Every terminal in a connect class takes one net, named after the first
/// io port (in `ports` order) in the class, else `net{k}`; each unwired pin or
/// device port gets a singleton net.
///
/// # Errors
/// Whatever `f` returns, then [`GenError::UnknownTerminal`] for a connect
/// endpoint that is neither an io port nor a pin, device terminal or imported
/// edge endpoint of a placed instance.
///
/// # Panics
/// When the build needs more than `u16::MAX + 1` nets.
pub fn build_with<P: Process>(
    process: &P,
    ports: Vec<String>,
    f: impl FnOnce(&mut CompBuilder<P>) -> Result<(), GenError>,
) -> Result<BuiltComp, GenError> {
    let mut c = CompBuilder { process, placed: Vec::new(), edges: Vec::new() };
    f(&mut c)?;
    let CompBuilder { placed, edges, .. } = c;
    // Every endpoint names an io port, or a pin, device terminal or imported
    // edge endpoint of a placed instance.
    let mut known: HashSet<String> = ports.iter().cloned().collect();
    for i in &placed {
        let q = |t: &str| format!("{}.{t}", i.name);
        known.extend(i.mac.pins.iter().map(|p| q(&p.name)));
        known.extend(i.devices.iter().flatten().flat_map(|(_, d)| d.terminals.iter().map(|(_, port)| q(port))));
        known.extend(i.edges.iter().flat_map(|(a, b)| [q(a), q(b)]));
    }
    if let Some(t) = edges.iter().flat_map(|(a, b)| [a, b]).find(|t| !known.contains(*t)) {
        return Err(GenError::UnknownTerminal(t.clone()));
    }

    // Qualify each instance's devices and ports by its name (`x1` + `m1` ⇒
    // `x1.m1`), the same keys the pins get. One opaque instance ⇒ no netlist.
    let devices: Option<Vec<(String, GenDevice)>> = placed
        .iter()
        .map(|i| i.devices.as_ref().map(|ds| (i, ds)))
        .collect::<Option<Vec<_>>>()
        .map(|all| {
            all.into_iter()
                .flat_map(|(i, ds)| {
                    ds.iter().map(move |(sub, d)| {
                        let name = if sub.is_empty() { i.name.clone() } else { format!("{}.{sub}", i.name) };
                        let terminals =
                            d.terminals.iter().map(|(t, port)| (t.clone(), format!("{}.{port}", i.name))).collect();
                        (name, GenDevice { kind: d.kind, terminals, params: d.params.clone() })
                    })
                })
                .collect()
        });

    // One binding for every view: connect classes first, then a singleton net
    // per unwired qualified name.
    let (mut nets, mut binding) = resolve_nets(&edges, &ports);
    let mut net_of = |name: &str| {
        // Look up before inserting: most names are already bound, and an
        // `entry` would allocate the key every call.
        let id = match binding.get(name) {
            Some(&id) => id,
            None => {
                nets.push(format!("net{}", nets.len()));
                binding.insert(name.to_string(), nets.len() - 1);
                nets.len() - 1
            }
        };
        NetId(u16::try_from(id).expect("net count fits u16"))
    };
    let mut flat = cells::Builder::new(process.grid());
    let mut instances = Vec::with_capacity(placed.len());
    for inst in placed {
        let mut mac = inst.mac;
        for s in &mac.shapes {
            flat.rect(s.layer, s.rect);
        }
        for p in &mut mac.pins {
            p.name = format!("{}.{}", inst.name, p.name);
            p.net = net_of(&p.name);
            flat.pin(p.clone());
        }
        instances.push((inst.name, mac));
    }
    // A device port nothing drew or connected mints a singleton, like a pin.
    let net_devices: Option<Vec<Device>> = devices.as_ref().map(|ds| {
        ds.iter()
            .map(|(name, d)| Device {
                name: name.clone(),
                kind: d.kind, model: String::new(),
                terminals: d.terminals.iter().map(|(t, port)| (t.clone(), net_of(port))).collect(),
                params: d.params.clone(),
            })
            .collect()
    });
    let netlist = net_devices
        .map(|devices| Netlist { nets: nets.iter().map(|n| Net { name: n.clone() }).collect(), devices, ..Default::default() });
    Ok(BuiltComp { instances, flat: flat.finish(), nets, edges, ports, netlist, devices })
}

// ===========================================================================
//  Shipped Devices
// ===========================================================================

/// The built-in [`DeviceGen`]s, drawn by the `cells` generators.
pub mod variants {
    use super::{Block, DeviceBuilder, DeviceGen, GenDevice, GenError, InOut, Input, Io, PortInfo, Process, Signal};
    use pnr_core::{DeviceKind, Macro};

    /// Replay a `cells` macro into `cell`, renaming each `d{i}:{T}` pin.
    fn replay<P: Process>(
        cell: &mut DeviceBuilder<P>,
        mac: &Macro,
        rename: impl Fn(usize, &str) -> String,
    ) -> Result<(), GenError> {
        for s in &mac.shapes {
            cell.draw(s.layer, s.rect)?;
        }
        for p in &mac.pins {
            let (d, t) = p.name.split_once(':').unwrap_or(("d0", &p.name));
            let di = d.trim_start_matches('d').parse().unwrap_or(0);
            cell.pin(&rename(di, t), p.layer, p.at)?;
        }
        for &u in &mac.units {
            cell.builder.unit(u);
        }
        for k in &mac.keepouts {
            cell.builder.keepout(k.rect, k.why);
        }
        Ok(())
    }

    /// MOS terminals: gate in, drain/source/body bidirectional.
    #[derive(Default)]
    pub struct MosIo {
        /// Gate.
        pub g: Input<Signal>,
        /// Drain.
        pub d: InOut<Signal>,
        /// Source.
        pub s: InOut<Signal>,
        /// Body (bulk).
        pub b: InOut<Signal>,
    }
    impl Io for MosIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.g.port("g"), self.d.port("d"), self.s.port("s"), self.b.port("b")]
        }
    }

    /// A multi-finger MOSFET (`cells::mosfet`).
    pub struct Mos {
        /// `Nmos` or `Pmos`.
        pub kind: DeviceKind,
        /// Finger width, nm.
        pub w: i32,
        /// Gate length, nm.
        pub l: i32,
        /// Finger count; `0` draws one.
        pub nf: u16,
        /// Finger pattern; `None` = smallest.
        pub pattern: Option<cells::Pattern>,
        /// Dummies per edge. They sit on the diffusion, so each is a real
        /// (off) transistor in the schematic too; opt-in: `Some(n > 0)` draws
        /// the generator's one per end, `None`/`Some(0)` none.
        pub dummies_per_edge: Option<u8>,
    }

    impl Mos {
        /// The smallest pattern, no dummies.
        #[must_use]
        pub fn new(kind: DeviceKind, w: i32, l: i32, nf: u16) -> Self {
            Self { kind, w, l, nf, pattern: None, dummies_per_edge: None }
        }
    }

    impl Block for Mos {
        type Io = MosIo;
        fn name(&self) -> String {
            format!("mos_{:?}_w{}_l{}_nf{}", self.kind, self.w, self.l, self.nf)
        }
    }
    impl DeviceGen for Mos {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let (pat, nf) = (self.pattern, self.nf.max(1));
            let mac = crate::adapter::draw::<cells::mosfet::Mosfet>(self.kind, self.w, self.l, &[nf], self.dummies(), cell.process(), |m| {
                m.nf == nf && pat.is_none_or(|p| m.style == p)
            });
            replay(cell, &mac, |_, t| t.to_ascii_lowercase())
        }

        /// The device, then each end dummy (an off transistor extraction sees):
        /// drain on the end S/D region (source at the left end; the right end is
        /// source for even `nf`, drain for odd), gate/source/body on `b`.
        fn devices(&self) -> Option<Vec<GenDevice>> {
            let mut out = vec![mos_device(self.kind, self.w, self.l, self.nf, "")];
            if self.dummies() {
                let right = if self.nf.max(1) % 2 == 0 { "s" } else { "d" };
                for edge in ["s", right] {
                    out.push(GenDevice {
                        kind: self.kind,
                        terminals: ["D", "G", "S", "B"].iter().zip([edge, "b", "b", "b"]).map(|(t, n)| ((*t).into(), n.into())).collect(),
                        params: vec![("w".into(), i64::from(self.w)), ("l".into(), i64::from(self.l)), ("nf".into(), 1)],
                    });
                }
            }
            Some(out)
        }
    }

    impl Mos {
        /// Whether end dummies are drawn (`dummies_per_edge` is `Some(n > 0)`).
        fn dummies(&self) -> bool {
            self.dummies_per_edge.is_some_and(|n| n > 0)
        }
    }

    /// One MOS device, ports suffixed by `leg`; the body is always the shared `b`.
    fn mos_device(kind: DeviceKind, w: i32, l: i32, nf: u16, leg: &str) -> GenDevice {
        GenDevice {
            kind,
            terminals: vec![
                ("D".into(), format!("d{leg}")),
                ("G".into(), format!("g{leg}")),
                ("S".into(), format!("s{leg}")),
                ("B".into(), "b".into()),
            ],
            // `w` is the SPICE total over `nf` fingers (`pnr_core::MosSize`).
            params: vec![("w".into(), i64::from(w) * i64::from(nf.max(1))), ("l".into(), i64::from(l)), ("nf".into(), i64::from(nf.max(1)))],
        }
    }

    /// Matched pair terminals: per-leg gate/drain/source, shared body.
    #[derive(Default)]
    pub struct MatchedPairIo {
        /// Leg 1 gate.
        pub g1: Input<Signal>,
        /// Leg 1 drain.
        pub d1: InOut<Signal>,
        /// Leg 1 source.
        pub s1: InOut<Signal>,
        /// Leg 2 gate.
        pub g2: Input<Signal>,
        /// Leg 2 drain.
        pub d2: InOut<Signal>,
        /// Leg 2 source.
        pub s2: InOut<Signal>,
        /// Body (bulk).
        pub b: InOut<Signal>,
    }
    impl Io for MatchedPairIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![
                self.g1.port("g1"),
                self.d1.port("d1"),
                self.s1.port("s1"),
                self.g2.port("g2"),
                self.d2.port("d2"),
                self.s2.port("s2"),
                self.b.port("b"),
            ]
        }
    }

    /// Two matched MOSFETs drawn as one interleaved macro (`d0:G` → `g1`,
    /// `d1:G` → `g2`; both bulk pins → `b`).
    pub struct MatchedPair {
        /// `Nmos` or `Pmos`.
        pub kind: DeviceKind,
        /// Unit finger width, nm.
        pub w: i32,
        /// Gate length, nm.
        pub l: i32,
        /// Fingers per leg; `0` draws one.
        pub nf_each: u16,
        /// Interleaving; `None` = smallest.
        pub pattern: Option<cells::Pattern>,
    }

    impl Block for MatchedPair {
        type Io = MatchedPairIo;
        fn name(&self) -> String {
            format!("pair_{:?}_w{}_l{}_nf{}", self.kind, self.w, self.l, self.nf_each)
        }
    }
    impl DeviceGen for MatchedPair {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let (pat, nf) = (self.pattern, self.nf_each.max(1));
            let mac = crate::adapter::draw::<cells::mosfet::Mosfet>(self.kind, self.w, self.l, &[nf, nf], false, cell.process(), |m| {
                m.nf == nf && pat.is_none_or(|p| m.style == p)
            });
            replay(cell, &mac, |di, t| match t {
                "B" => "b".into(),
                t => format!("{}{}", t.to_ascii_lowercase(), di + 1),
            })
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            let nf = self.nf_each.max(1);
            Some(vec![
                mos_device(self.kind, self.w, self.l, nf, "1"),
                mos_device(self.kind, self.w, self.l, nf, "2"),
            ])
        }
    }

    /// Resistor terminals.
    #[derive(Default)]
    pub struct ResIo {
        /// Resistor end `P`.
        pub a: InOut<Signal>,
        /// Resistor end `N`.
        pub b: InOut<Signal>,
    }
    impl Io for ResIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.a.port("a"), self.b.port("b")]
        }
    }

    /// A poly resistor (`cells::resistor`); `a` = `P`, `b` = `N`.
    pub struct Res {
        /// Body width, nm.
        pub w: i32,
        /// Body length, nm.
        pub len: i32,
    }
    impl Block for Res {
        type Io = ResIo;
        fn name(&self) -> String {
            format!("res_w{}_l{}", self.w, self.len)
        }
    }
    impl DeviceGen for Res {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let mac = crate::adapter::draw::<cells::resistor::Resistor>(
                DeviceKind::Resistor,
                self.w,
                self.len,
                &[1],
                false,
                cell.process(),
                |_| true,
            );
            replay(cell, &mac, |_, t| if t == "P" { "a".into() } else { "b".into() })
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            Some(vec![GenDevice {
                kind: DeviceKind::Resistor,
                terminals: vec![("P".into(), "a".into()), ("N".into(), "b".into())],
                params: vec![("w".into(), i64::from(self.w)), ("l".into(), i64::from(self.len))],
            }])
        }
    }
}

// ===========================================================================
//  Macros registry
// ===========================================================================

/// User-registered macros keyed by netlist instance name; a registered name
/// replaces `cells` auto-draw for that device.
#[derive(Default)]
pub struct Macros {
    entries: Vec<(String, Macro)>,
}

impl Macros {
    /// Registers `m` for `name`, replacing an earlier registration.
    pub fn register(&mut self, name: &str, m: Macro) {
        match self.entries.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = m,
            None => self.entries.push((name.to_string(), m)),
        }
    }

    /// The macro registered for `name`, if any. O(registrations).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Macro> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, m)| m)
    }
}

#[cfg(test)]
mod tests;
