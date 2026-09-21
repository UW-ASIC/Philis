//! # `macroMaster` — substrate2-style, PDK-agnostic macro generators
//!
//! Two authoring tiers, split by which builder you are handed (see
//! `docs/adr/0004` and `CONTEXT.md`):
//!
//! - **[`DeviceGen`]** — a leaf that draws fundamental geometry through a
//!   [`DeviceBuilder`] (the only surface with [`DeviceBuilder::draw`]). A Device
//!   is the only thing validated by the gpurify engine, against the
//!   [`GenericPdk`] — a synthetic canonical rule set that forces PDK-agnostic
//!   authoring. Shared diffusion (multi-finger MOS) lives *inside* one Device.
//! - **[`Composition`]** — assembles Devices through a [`CompBuilder`]
//!   (instantiate + [`CompBuilder::place`] + [`CompBuilder::connect`], **no**
//!   raw geometry). It guarantees *placement* legality by construction: on-grid,
//!   and **no device-on-device overlap** — [`CompBuilder::place`] rejects an
//!   overlap at placement time with [`GenError::Overlap`]. Matched structures
//!   (diff pairs, mirrors) are Devices wired by nets, never overlapped.
//!
//! What is **not** guaranteed here is the routed result: nets declared with
//! [`CompBuilder::connect`] are routed downstream (`backend/gr`+`dr`) and the
//! real DRC/LVS/ERC/PEX is gpurify **signoff** against a foundry PDK. In-crate
//! LVS is *structural* — the declared connect-graph vs [`Schematic::schematic`],
//! needing no geometry.
//!
//! | substrate2 | here |
//! |---|---|
//! | `Block { type Io; fn io() }` | [`Block`] |
//! | `Input/Output/InOut<Signal>` | [`Input`]/[`Output`]/[`InOut`]/[`Signal`] |
//! | `impl Layout` (draws) | [`DeviceGen`] + [`DeviceBuilder`] |
//! | `cell.generate` + `align_mut` | [`Composition`] + [`CompBuilder::place`] |
//! | `impl Schematic` | [`Schematic`] |
//! | `Sky130::layer(...)` | [`Process`] / [`GenericPdk`] |

#![allow(dead_code)]

mod adapter;
mod check;
#[cfg(all(test, feature = "gpurify"))]
mod variant_signoff;
mod generic_deck;

pub use check::{check, check_device, check_lvs};
/// The synthetic generic deck, exported so cross-PDK tests can elaborate the
/// same generator against it *and* a foundry deck.
pub use generic_deck::GENERIC_DECK_JSON;
pub use pnr_core::Process;

use pnr_core::{Device, DeviceKind, Dir, LayerId, Macro, Net, NetId, Netlist, Orient, Pin, Rect};

// ===========================================================================
//  Typed IO — direction lives in the type (substrate2 `Input`/`Output`/`InOut`)
// ===========================================================================

/// A single-bit signal — the leaf of an IO bundle (substrate2 `Signal`).
#[derive(Clone, Copy, Default, Debug)]
pub struct Signal;

/// An **input** port (substrate2 `Input<T>`).
#[derive(Clone, Copy, Default, Debug)]
pub struct Input<T>(pub T);

/// An **output** port (substrate2 `Output<T>`).
#[derive(Clone, Copy, Default, Debug)]
pub struct Output<T>(pub T);

/// A bidirectional port — power, body, analog nets (substrate2 `InOut<T>`).
#[derive(Clone, Copy, Default, Debug)]
pub struct InOut<T>(pub T);

/// A named, directioned terminal a block exposes — the flattened form of a
/// direction-wrapped [`Signal`], recorded for ERC/LVS. The direction is fixed by
/// the wrapper it came from, so it can never disagree with the type.
#[derive(Clone, Debug)]
pub struct PortInfo {
    /// Terminal name, unique within the bundle.
    pub name: String,
    /// Direction, fixed by the wrapper type it came from.
    pub dir: Dir,
}

impl Input<Signal> {
    /// Name this input, yielding the flat [`PortInfo`] `ports()` returns.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::In }
    }
}
impl Output<Signal> {
    /// Name this output.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::Out }
    }
}
impl InOut<Signal> {
    /// Name this bidirectional terminal.
    #[must_use]
    pub fn port(&self, name: &str) -> PortInfo {
        PortInfo { name: name.into(), dir: Dir::InOut }
    }
}

/// A typed port bundle (substrate2 `Io`). Because each implementor is a distinct
/// Rust type, connecting the wrong bundle is a `rustc` error — the compile-time
/// guarantee. `Default` lets [`Block::io`] and the checks build it argument-free.
pub trait Io: Default {
    /// The block's terminals, in a stable order, flattened to name + direction.
    fn ports(&self) -> Vec<PortInfo>;
}

// ===========================================================================
//  Block / DeviceGen / Composition / Schematic (the generator traits)
// ===========================================================================

/// A generator's parameter struct (substrate2 `Block`). Implementors add
/// [`DeviceGen`] *or* [`Composition`], and optionally [`Schematic`].
pub trait Block {
    /// This block's typed IO — checked by `rustc`.
    type Io: Io;

    /// Descriptive cell name, used in reports/netlists.
    fn name(&self) -> String;

    /// Construct the runtime IO bundle. Usually `Self::Io::default()`.
    fn io(&self) -> Self::Io {
        Self::Io::default()
    }
}

/// A PDK-agnostic **device** generator: the only tier that draws raw geometry.
///
/// The body resolves every layer/rule from `cell.process()` and draws through
/// [`DeviceBuilder::draw`]. It is validated by [`check_device`] against the
/// [`GenericPdk`]; passing that (with by-role layers and by-rule dimensions) is
/// the evidence the Device is portable to any real PDK. Shared diffusion
/// (multi-finger MOS) is handled *inside* the Device — never by overlapping two.
pub trait DeviceGen: Block {
    /// Draw the device. Only [`DeviceBuilder::draw`]/[`DeviceBuilder::connect`]
    /// are available — no sub-instantiation, no placement.
    fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError>;

    /// The schematic devices this generator draws — what makes a composition
    /// *readable as a circuit* rather than as geometry.
    ///
    /// `None` (the default) means **opaque**: the geometry is drawn but its
    /// device structure is not declared. One opaque instance makes the whole
    /// [`BuiltComp::netlist`] `None`, and a composition with no netlist gets no
    /// analog routing rules and no geometric LVS — it is routed and
    /// DRC-checked, nothing more. Declare this on anything you want the
    /// constraint engine to see.
    ///
    /// A multi-device generator (e.g. [`variants::MatchedPair`]) returns one
    /// entry per schematic device.
    fn devices(&self) -> Option<Vec<GenDevice>> {
        None
    }
}

/// One schematic device a [`DeviceGen`] draws, named in the generator's own
/// terms. The composition resolves it to real [`NetId`]s at build.
#[derive(Clone, Debug)]
pub struct GenDevice {
    /// Electrical kind — what the recogniser matches on.
    pub kind: DeviceKind,
    /// `(schematic terminal, this generator's port name)` — `("D", "d")` for a
    /// single-device generator, `("D", "d1")` for the first leg of a pair. The
    /// terminal keys are the ones the annotator's catalog and
    /// `library`'s LVS reference speak: `D`/`G`/`S`/`B`, `P`/`N`, `C`/`B`/`E`.
    pub terminals: Vec<(String, String)>,
    /// Device parameters in **nm**, keyed as the netlist parser keys them
    /// (`w`, `l`, `nf`, `m`) — the annotator sizes matched groups from these.
    pub params: Vec<(String, i64)>,
}

/// A PDK-agnostic **composition**: assembles [`DeviceGen`]s. No raw geometry.
///
/// Instantiate Devices with [`CompBuilder::instantiate`], position them with
/// [`CompBuilder::place`] (which rejects overlap), and wire nets with
/// [`CompBuilder::connect`]. Placement legality is by construction; routing and
/// final DRC are downstream signoff.
pub trait Composition: Block {
    /// Build the macro from Devices. There is deliberately no `draw` on
    /// [`CompBuilder`] — a Composition cannot emit stray geometry.
    fn build<P: Process>(&self, cell: &mut CompBuilder<P>) -> Result<(), GenError>;
}

/// An optional **schematic** generator (substrate2 `Schematic`). Implement it on
/// a [`Composition`] to opt into the *structural* LVS in [`check_lvs`] (declared
/// connect-graph vs this netlist).
pub trait Schematic: Block {
    /// The intended device-level netlist for this block.
    fn schematic(&self) -> Netlist;
}

// ===========================================================================
//  Relative placement (substrate2 AlignMode / align_bbox)
// ===========================================================================

/// How to align one instance's bbox against another's (substrate2 `AlignMode`).
///
/// `ToTheRight`/`ToTheLeft`/`Above`/`Beneath` *abut* (edge-to-edge, plus offset);
/// the flush/centre modes overlap-align a pair of edges or centres. You say
/// "pmos above nmos by 600nm", never "pmos at y=1200".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlignMode {
    /// Left edges flush.
    Left,
    /// Right edges flush.
    Right,
    /// Bottom edges flush.
    Bottom,
    /// Top edges flush.
    Top,
    /// Centres flush along x.
    CenterHorizontal,
    /// Centres flush along y.
    CenterVertical,
    /// Abut: self's left placed at other's right (+offset).
    ToTheRight,
    /// Abut: self's right placed at other's left (−offset).
    ToTheLeft,
    /// Abut: self's bottom placed at other's top (+offset).
    Above,
    /// Abut: self's top placed at other's bottom (−offset).
    Beneath,
}

/// A placed (or to-be-placed) device instance (substrate2 `Instance`). Carries
/// the Device's geometry and its current bbox. Positioning happens through
/// [`CompBuilder::place`]/[`CompBuilder::place_by`], which own the overlap check;
/// an [`Instance`] handle returned by them is the reference for the *next* place.
pub struct Instance {
    name: String,
    device: String,
    mac: Macro,
    bbox: Rect,
    /// How this instance has been turned from its drawn orientation — GDS
    /// SREF-compatible D4 member, recorded for emit/metadata. The shapes are
    /// kept flat (already transformed), so this is bookkeeping, not a pending
    /// transform.
    orient: Orient,
    /// A sub-*composition*'s internal connect-graph, imported into the parent
    /// at commit (prefixed by this instance's name) so the child's internal
    /// nets stay whole for top-level routing. Empty for Devices.
    edges: Vec<(String, String)>,
    /// The schematic devices behind this instance, `(name, device)` with both
    /// the name and each terminal's port name **relative to this instance** —
    /// `("", Mos)` for a plain Device, `("m1", Mos)` for one drawn inside a
    /// sub-composition. `build_with` prefixes them with the instance name, which
    /// is exactly how `commit` prefixes pins and edges, so the port names come
    /// out as binding keys. `None` = opaque geometry (see [`DeviceGen::devices`]).
    devices: Option<Vec<(String, GenDevice)>>,
}

impl Clone for Instance {
    fn clone(&self) -> Self {
        Instance {
            name: self.name.clone(),
            device: self.device.clone(),
            mac: Macro {
                shapes: self.mac.shapes.clone(),
                pins: self.mac.pins.clone(),
                bbox: self.mac.bbox,
            },
            bbox: self.bbox,
            orient: self.orient,
            edges: self.edges.clone(),
            devices: self.devices.clone(),
        }
    }
}

impl Instance {
    /// This instance's current bounding box.
    #[must_use]
    pub fn bbox(&self) -> Rect {
        self.bbox
    }

    /// The instance name — the `m1` in the `m1.d` terminals `connect` takes.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// A qualified terminal reference on this instance (`inst.term("d")` ⇒
    /// `"m1.d"`), for [`CompBuilder::connect`].
    #[must_use]
    pub fn term(&self, port: &str) -> String {
        format!("{}.{}", self.name, port)
    }

    /// The Device's name (for reports).
    #[must_use]
    pub fn device(&self) -> &str {
        &self.device
    }

    /// How this instance has been turned (identity unless placed by a
    /// mirroring/rotating op).
    #[must_use]
    pub fn orient(&self) -> Orient {
        self.orient
    }

    /// Align this (not-yet-committed) instance against a placed reference —
    /// substrate2's `align_mut`, exposed so a caller can compose two aligns
    /// (e.g. `Bottom` then `ToTheRight`) before one `place`. `place_by` is the
    /// one-align shorthand.
    pub fn align(&mut self, mode: AlignMode, reference: &Instance, offset: i32) {
        let (dx, dy) = align_delta(self.bbox, reference.bbox, mode, offset);
        self.translate(dx, dy);
    }

    /// Mirror all geometry about the vertical line `x = axis` (`x ↦ 2·axis −
    /// x`). Grid-preserving when `axis` is on-grid. Pin *names* stay; only
    /// their positions flip — a mirrored `g` is still `g`.
    fn mirror_x_about(&mut self, axis: i32) {
        let flip = |r: Rect| Rect { x: 2 * axis - (r.x + r.w), y: r.y, w: r.w, h: r.h };
        self.bbox = flip(self.bbox);
        self.mac.bbox = flip(self.mac.bbox);
        for s in &mut self.mac.shapes {
            s.rect = flip(s.rect);
        }
        for p in &mut self.mac.pins {
            p.at = flip(p.at);
        }
        // Composing a y-axis mirror (Mx180) onto the current turn.
        self.orient = match self.orient {
            Orient::R0 => Orient::Mx180,
            Orient::Mx180 => Orient::R0,
            Orient::R90 => Orient::Mx270,
            Orient::Mx270 => Orient::R90,
            Orient::R180 => Orient::Mx,
            Orient::Mx => Orient::R180,
            Orient::R270 => Orient::Mx90,
            Orient::Mx90 => Orient::R270,
        };
    }

    fn translate(&mut self, dx: i32, dy: i32) {
        self.bbox.x += dx;
        self.bbox.y += dy;
        // Keep the macro's cached bbox coherent with its shapes.
        self.mac.bbox.x += dx;
        self.mac.bbox.y += dy;
        for s in &mut self.mac.shapes {
            s.rect.x += dx;
            s.rect.y += dy;
        }
        for p in &mut self.mac.pins {
            p.at.x += dx;
            p.at.y += dy;
        }
    }
}

/// Translation to align box `a` onto box `b` per `mode` + signed `offset`
/// (substrate2 `align_mut`). Abutment modes fold the offset into the primary
/// axis; flush/centre modes take it as a nudge along the free axis.
fn align_delta(a: Rect, b: Rect, mode: AlignMode, offset: i32) -> (i32, i32) {
    let (dx, dy) = match mode {
        AlignMode::Left => (b.x - a.x, 0),
        AlignMode::Right => ((b.x + b.w) - (a.x + a.w), 0),
        AlignMode::Bottom => (0, b.y - a.y),
        AlignMode::Top => (0, (b.y + b.h) - (a.y + a.h)),
        AlignMode::CenterHorizontal => (0, (b.y + b.h / 2) - (a.y + a.h / 2)),
        AlignMode::CenterVertical => ((b.x + b.w / 2) - (a.x + a.w / 2), 0),
        AlignMode::ToTheRight => ((b.x + b.w) - a.x + offset, 0),
        AlignMode::ToTheLeft => (b.x - (a.x + a.w) - offset, 0),
        AlignMode::Above => (0, (b.y + b.h) - a.y + offset),
        AlignMode::Beneath => (0, b.y - (a.y + a.h) - offset),
    };
    match mode {
        AlignMode::Left | AlignMode::Right => (dx, offset),
        AlignMode::Bottom | AlignMode::Top => (offset, dy),
        AlignMode::CenterHorizontal => (offset, dy),
        AlignMode::CenterVertical => (dx, offset),
        _ => (dx, dy),
    }
}

/// Strict interior overlap of two bboxes. Edge-flush abutment (a shared boundary,
/// the substrate2 idiom) is **not** overlap; only positive-area intersection is.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Resolve a declared connect-graph into nets: union-find over edge endpoints.
/// A class containing an io-port name takes that name (first in `ports` order
/// wins); purely internal classes synthesize `net{k}`. Returns the net names
/// and the terminal → net-index binding — the map that later assigns every
/// retained pin its [`NetId`].
pub(crate) fn resolve_nets(
    edges: &[(String, String)],
    ports: &[String],
) -> (Vec<String>, std::collections::HashMap<String, usize>) {
    use std::collections::HashMap;
    // Intern terminals.
    let mut idx: HashMap<&str, usize> = HashMap::new();
    for (a, b) in edges {
        for t in [a.as_str(), b.as_str()] {
            let n = idx.len();
            idx.entry(t).or_insert(n);
        }
    }
    // ponytail: path-halving DSU over a Vec — terminal counts are tiny.
    let mut parent: Vec<usize> = (0..idx.len()).collect();
    fn find(parent: &mut Vec<usize>, mut i: usize) -> usize {
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
    // Name classes: port name if a member is a declared port, else synthesized.
    let mut class_of_root: HashMap<usize, usize> = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    let mut binding: HashMap<String, usize> = HashMap::new();
    let terms: Vec<&str> = {
        let mut v: Vec<(&str, usize)> = idx.iter().map(|(t, i)| (*t, *i)).collect();
        v.sort_by_key(|&(_, i)| i);
        v.into_iter().map(|(t, _)| t).collect()
    };
    for t in &terms {
        let root = find(&mut parent, idx[t]);
        let class = *class_of_root.entry(root).or_insert_with(|| {
            names.push(String::new()); // named below
            names.len() - 1
        });
        binding.insert((*t).to_string(), class);
    }
    for (i, name) in names.iter_mut().enumerate() {
        let port = ports.iter().find(|p| binding.get(*p) == Some(&i));
        *name = match port {
            Some(p) => p.clone(),
            None => format!("net{i}"),
        };
    }
    (names, binding)
}

// ===========================================================================
//  The two builders — the capability split
// ===========================================================================

/// A recorded net: two named terminals the generator joined. Structural LVS and
/// the floating-port check read this.
#[derive(Clone)]
struct NetEdge {
    a: String,
    b: String,
}

/// The **device**-authoring surface (substrate2 `CellBuilder`, draw side). The
/// only builder with [`DeviceBuilder::draw`]; it has no `instantiate`/`place`, so
/// a Device is pure geometry.
pub struct DeviceBuilder<'a, P: Process> {
    builder: &'a mut cells::Builder,
    process: &'a P,
    nets: Vec<NetEdge>,
}

impl<'a, P: Process> DeviceBuilder<'a, P> {
    fn new(builder: &'a mut cells::Builder, process: &'a P) -> Self {
        Self { builder, process, nets: Vec::new() }
    }

    /// The bound [`Process`] — the only source of a [`LayerId`] or a rule value,
    /// so a Device can carry no hardcoded PDK constant.
    #[must_use]
    pub fn process(&self) -> &P {
        self.process
    }

    /// Draw a rectangle on a resolved layer (substrate2 `cell.draw`). Grid-checks
    /// against the bound process — the one DRC needing no rule table. Off-grid ⇒
    /// [`GenError::OffGrid`].
    pub fn draw(&mut self, layer: LayerId, r: Rect) -> Result<(), GenError> {
        if !on_grid(r, self.process.grid()) {
            return Err(GenError::OffGrid);
        }
        self.builder.rect(layer, r);
        Ok(())
    }

    /// Register a net attach point on a resolved layer — the geometry a router
    /// (or a parent Composition) targets. `name` is the Device-local port name
    /// (`g`/`d`/`s`/`b`); Compositions qualify it with the instance name at
    /// commit. The `NetId` is a placeholder until composition-level net
    /// resolution binds it.
    pub fn pin(&mut self, name: &str, layer: LayerId, at: Rect) -> Result<(), GenError> {
        if !on_grid(at, self.process.grid()) {
            return Err(GenError::OffGrid);
        }
        self.builder.pin(Pin { name: name.into(), net: NetId(0), at, layer });
        Ok(())
    }

    /// Join two named terminals into a net (tracked for the floating-port check).
    pub fn connect(&mut self, a: &str, b: &str) {
        self.nets.push(NetEdge { a: a.into(), b: b.into() });
    }

    pub(crate) fn nets_edges(&self) -> Vec<(String, String)> {
        self.nets.iter().map(|e| (e.a.clone(), e.b.clone())).collect()
    }
}

/// The **composition** surface (substrate2 `CellBuilder`, compose side). Has
/// `instantiate`/`place`/`connect` but **no** `draw` — a Composition cannot emit
/// raw geometry.
pub struct CompBuilder<'a, P: Process> {
    builder: &'a mut cells::Builder,
    process: &'a P,
    placed: Vec<Instance>,
    nets: Vec<NetEdge>,
    /// Declared mirror symmetries: `(left instance, right instance, axis x)`.
    sym_pairs: Vec<(String, String, i32)>,
}

impl<'a, P: Process> CompBuilder<'a, P> {
    fn new(builder: &'a mut cells::Builder, process: &'a P) -> Self {
        Self { builder, process, placed: Vec::new(), nets: Vec::new(), sym_pairs: Vec::new() }
    }

    /// The bound [`Process`].
    #[must_use]
    pub fn process(&self) -> &P {
        self.process
    }

    /// Instantiate a [`DeviceGen`] (substrate2 `cell.generate`): draw it into a
    /// fresh builder on the same grid and return an [`Instance`] at the origin,
    /// ready to [`CompBuilder::place`]. `name` is the instance name terminals
    /// are qualified by (`connect(&inst.term("d"), "vout")`).
    pub fn instantiate<D: DeviceGen>(&mut self, name: &str, dev: &D) -> Result<Instance, GenError> {
        let mut sub = cells::Builder::new(self.process.grid());
        {
            let mut db = DeviceBuilder::new(&mut sub, self.process);
            dev.layout(&mut db)?;
        }
        let mac = sub.finish();
        let bbox = mac.bbox;
        // A single-device generator's device carries the instance's own name
        // (`m1`); a multi-device one suffixes the ordinal (`m1.0`, `m1.1`). Port
        // names are already Device-local, so they need no relative prefix.
        let devices = dev.devices().map(|ds| {
            let multi = ds.len() > 1;
            ds.into_iter()
                .enumerate()
                .map(|(i, d)| (if multi { i.to_string() } else { String::new() }, d))
                .collect()
        });
        Ok(Instance {
            name: name.into(),
            device: dev.name(),
            mac,
            bbox,
            orient: Orient::R0,
            edges: Vec::new(),
            devices,
        })
    }

    /// Instantiate a **sub-composition** — hierarchy. The child is built
    /// against the same process; its port-net pins surface under the *port*
    /// name (so the parent connects `x1.vout`), its internal pins stay
    /// qualified (`x1.m1.d`), and its internal connect-graph is imported at
    /// commit so internal nets stay whole for top-level routing.
    pub fn instantiate_comp<C: Composition>(
        &mut self,
        name: &str,
        comp: &C,
    ) -> Result<Instance, GenError> {
        let built = build_composition(comp, self.process)?;
        let mut mac = built.flat;
        // Port-net pins take the port's name; internal pins keep their
        // qualified name. Both get this instance's prefix at commit.
        for p in &mut mac.pins {
            let net = &built.nets[p.net.0 as usize];
            if built.ports.contains(net) {
                p.name.clone_from(net);
            }
        }
        let bbox = mac.bbox;
        Ok(Instance {
            name: name.into(),
            device: comp.name(),
            mac,
            bbox,
            orient: Orient::R0,
            edges: built.edges,
            // Already relative to the child's root, which is this instance —
            // `build_with` prefixes them exactly as `commit` prefixes the child's
            // pins and edges.
            devices: built.devices,
        })
    }

    /// Place `inst` as `reference`'s **mirror partner**: geometry flipped about
    /// a shared vertical axis sitting `gap/2` past `reference`'s right edge
    /// (axis snapped up to the grid), so the pair is exactly symmetric about
    /// it — the matched-pair idiom. Overlap is rejected like any placement.
    /// Records the `(reference, inst, axis)` symmetry for emit/metadata.
    pub fn place_mirrored(
        &mut self,
        mut inst: Instance,
        reference: &Instance,
        gap: i32,
    ) -> Result<Instance, GenError> {
        let g = self.process.grid().max(1);
        let axis = {
            let a = reference.bbox.x + reference.bbox.w + gap / 2;
            (a + g - 1) / g * g // snap up: never tighter than asked
        };
        // Align vertically with the reference, then flip about the axis. The
        // flip maps the instance's left edge to the right of the axis by
        // however far left of the axis it sat — placing it there directly.
        let dy = reference.bbox.y - inst.bbox.y;
        let dx = reference.bbox.x - inst.bbox.x;
        inst.translate(dx, dy);
        inst.mirror_x_about(axis);
        let placed = self.commit(inst)?;
        self.sym_pairs.push((reference.name.clone(), placed.name.clone(), axis));
        Ok(placed)
    }

    /// Commit an [`Instance`] at its current position. Rejects overlap with any
    /// already-placed device ([`GenError::Overlap`]) — the by-construction
    /// placement guarantee. Returns the placed handle (for use as a `place_by`
    /// reference).
    pub fn place(&mut self, inst: Instance) -> Result<Instance, GenError> {
        self.commit(inst)
    }

    /// Align `inst` relative to a placed `reference` per `mode`+`offset`, then
    /// commit it. Overlap (against *any* placed device, not just `reference`) is
    /// rejected at this call ([`GenError::Overlap`]).
    pub fn place_by(
        &mut self,
        mut inst: Instance,
        mode: AlignMode,
        reference: &Instance,
        offset: i32,
    ) -> Result<Instance, GenError> {
        let (dx, dy) = align_delta(inst.bbox, reference.bbox, mode, offset);
        inst.translate(dx, dy);
        self.commit(inst)
    }

    fn commit(&mut self, inst: Instance) -> Result<Instance, GenError> {
        if self.placed.iter().any(|p| overlaps(p.bbox, inst.bbox)) {
            return Err(GenError::Overlap);
        }
        for s in &inst.mac.shapes {
            self.builder.rect(s.layer, s.rect);
        }
        // Retain the Device's pins, qualified by instance name — this is what
        // keeps the composed Macro routable (and net-resolvable) downstream.
        for p in &inst.mac.pins {
            let mut q = p.clone();
            q.name = format!("{}.{}", inst.name, p.name);
            self.builder.pin(q);
        }
        // A sub-composition's internal connect-graph joins the parent's,
        // prefixed — its internal nets must resolve here or top-level routing
        // sees them shattered into singletons.
        for (a, b) in &inst.edges {
            self.nets.push(NetEdge {
                a: format!("{}.{a}", inst.name),
                b: format!("{}.{b}", inst.name),
            });
        }
        let handle = inst.clone();
        self.placed.push(inst);
        Ok(handle)
    }

    /// Join two named terminals into a net (substrate2 `cell.connect`), tracked
    /// for the floating-port check and structural LVS.
    pub fn connect(&mut self, a: &str, b: &str) {
        self.nets.push(NetEdge { a: a.into(), b: b.into() });
    }

    pub(crate) fn nets_edges(&self) -> Vec<(String, String)> {
        self.nets.iter().map(|e| (e.a.clone(), e.b.clone())).collect()
    }
}

/// A [`Composition`] built against a live process — everything downstream
/// (routing, signoff, GDS) needs: per-instance placed macros with net-bound
/// qualified pins, the flat composed macro, and the resolved nets.
pub struct BuiltComp {
    /// `(instance name, placed macro)` in placement order. Pins are qualified
    /// (`m1.g`) and net-bound; shapes/pins are at the *placed* position.
    pub instances: Vec<(String, Macro)>,
    /// All shapes + qualified, net-bound pins in one flat macro.
    pub flat: Macro,
    /// Net names, indexed by [`NetId`].
    pub nets: Vec<String>,
    /// The declared connect edges (qualified terminals / io ports).
    pub edges: Vec<(String, String)>,
    /// The io port names, in declaration order.
    pub ports: Vec<String>,
    /// Declared mirror symmetries: `(left instance, right instance, axis x)`.
    pub sym_pairs: Vec<(String, String, i32)>,
    /// The composition **as a circuit**, on this build's own [`NetId`]
    /// numbering — `netlist.nets[i].name == nets[i]`, so a rule the annotator
    /// extracts from it indexes [`pnr_core::Routes`] correctly with no remap.
    ///
    /// `None` when any instance is opaque (see [`DeviceGen::devices`]): a
    /// partial netlist is worse than none, because the annotator would
    /// recognise the wrong structure and LVS would report a device-count
    /// mismatch that is an artefact of the gap rather than of the layout.
    pub netlist: Option<Netlist>,
    /// The schematic devices, names and port names relative to this build's
    /// root — kept so `instantiate_comp` can re-nest this composition inside a
    /// parent, where the numbering above no longer applies.
    pub(crate) devices: Option<Vec<(String, GenDevice)>>,
}

/// Build a [`Composition`] against `process` and resolve its connectivity —
/// the substrate3 elaboration entry. Placement is whatever the generator's
/// `place`/`place_by` calls chose (recomputed against *this* process's rules
/// and cell extents — that is the PDK-agnostic contract); routing is the
/// caller's next step (`library::elaborate` feeds this to `gr`+`dr`).
pub fn build_composition<C: Composition, P: Process>(
    comp: &C,
    process: &P,
) -> Result<BuiltComp, GenError> {
    let ports: Vec<String> = comp.io().ports().into_iter().map(|p| p.name).collect();
    build_with(process, ports, |c| comp.build(c))
}

/// [`build_composition`] for a build script that is *data*, not a type — an
/// interpreted IR (`library::emit`) has dynamic ports and a dynamic body, which
/// the associated-type [`Io`] cannot express. Same output, same guarantees.
pub fn build_with<P: Process>(
    process: &P,
    ports: Vec<String>,
    f: impl FnOnce(&mut CompBuilder<P>) -> Result<(), GenError>,
) -> Result<BuiltComp, GenError> {
    let mut builder = cells::Builder::new(process.grid());
    let (edges, mut instances, sym_pairs, devices) = {
        let mut c = CompBuilder::new(&mut builder, process);
        f(&mut c)?;
        let placed: Vec<(String, Macro)> =
            c.placed.iter().map(|i| (i.name.clone(), i.mac.clone())).collect();
        // Qualify every instance's devices by its name — `m1` + `""` ⇒ `m1`,
        // `x1` + `m1` ⇒ `x1.m1` — and the port names with it, so they land on
        // the same keys `commit` gave the pins. One opaque instance ⇒ no netlist.
        let mut devices = Some(Vec::new());
        for i in &c.placed {
            let (Some(out), Some(ds)) = (devices.as_mut(), i.devices.as_ref()) else {
                devices = None;
                break;
            };
            for (sub, d) in ds {
                let name =
                    if sub.is_empty() { i.name.clone() } else { format!("{}.{sub}", i.name) };
                out.push((
                    name,
                    GenDevice {
                        kind: d.kind,
                        terminals: d
                            .terminals
                            .iter()
                            .map(|(t, port)| (t.clone(), format!("{}.{port}", i.name)))
                            .collect(),
                        params: d.params.clone(),
                    },
                ));
            }
        }
        (c.nets_edges(), placed, c.sym_pairs.clone(), devices)
    };
    let mut flat = builder.finish();
    // One binding for both views: connect-resolved classes first, then a
    // singleton net per unwired *qualified name* — keyed by name, so the flat
    // macro and the per-instance macros assign identical NetIds.
    let (mut nets, mut binding) = resolve_nets(&edges, &ports);
    let net_of = |name: &str, nets: &mut Vec<String>,
                      binding: &mut std::collections::HashMap<String, usize>| {
        let id = *binding.entry(name.to_string()).or_insert_with(|| {
            nets.push(format!("net{}", nets.len()));
            nets.len() - 1
        });
        NetId(u16::try_from(id).expect("net count fits u16"))
    };
    for pin in &mut flat.pins {
        let name = pin.name.clone();
        pin.net = net_of(&name, &mut nets, &mut binding);
    }
    for (name, mac) in &mut instances {
        for pin in &mut mac.pins {
            pin.name = format!("{name}.{}", pin.name);
            let qualified = pin.name.clone();
            pin.net = net_of(&qualified, &mut nets, &mut binding);
        }
    }
    // Resolve the declared devices onto the *same* binding the pins just used,
    // so `netlist.nets[i].name == nets[i]` and a `NetId` means one thing across
    // the macro, the routes and the rules. A terminal whose port was never
    // drawn or connected mints a singleton here exactly as an unwired pin does.
    let netlist = devices.as_ref().map(|ds| {
        let devices = ds
            .iter()
            .map(|(name, d)| Device {
                name: name.clone(),
                kind: d.kind,
                terminals: d
                    .terminals
                    .iter()
                    .map(|(t, port)| (t.clone(), net_of(port, &mut nets, &mut binding)))
                    .collect(),
                params: d.params.clone(),
            })
            .collect();
        // Built last: the loops above append singleton nets as they go.
        Netlist { nets: nets.iter().map(|n| Net { name: n.clone() }).collect(), devices }
    });

    Ok(BuiltComp { instances, flat, nets, edges, ports, sym_pairs, netlist, devices })
}

/// Bind a composed [`Macro`]'s qualified pins to resolved [`NetId`]s: run
/// [`resolve_nets`] over the declared edges, rewrite each pin's `net` by its
/// qualified name, and give every *unwired* pin a fresh singleton net (so a
/// router sees it as its own — unconnected — net rather than as net 0).
/// Returns the net names, indexed by `NetId`.
pub fn bind_pins(mac: &mut Macro, edges: &[(String, String)], ports: &[String]) -> Vec<String> {
    let (mut names, binding) = resolve_nets(edges, ports);
    for pin in &mut mac.pins {
        let id = match binding.get(&pin.name) {
            Some(&i) => i,
            None => {
                names.push(format!("net{}", names.len()));
                names.len() - 1
            }
        };
        pin.net = NetId(u16::try_from(id).expect("net count fits u16"));
    }
    names
}

const ZERO: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

/// True iff every corner of `r` sits on the `grid` lattice. `grid <= 0` disables
/// the check (a PDK with no grid).
fn on_grid(r: Rect, grid: i32) -> bool {
    if grid <= 0 {
        return true;
    }
    [r.x, r.y, r.x + r.w, r.y + r.h].iter().all(|v| v % grid == 0)
}

// ===========================================================================
//  Generic PDK — the synthetic yardstick a Device is validated against
// ===========================================================================

/// The synthetic yardstick a Device is validated against — arbitrary layer roles
/// and lenient rule values on a single grid. Not a foundry PDK; it exists only to
/// run the gpurify Device check and to *force* PDK-agnostic authoring.
///
/// Two forms, chosen by the `gpurify` feature. **Off** (default): a hand-coded
/// [`Process`] with the same roles/grid as the deck, so Devices draw identically
/// but no engine runs (the Device check is structural only). **On**: it wraps a
/// real [`verify::Pdk`] parsed from the hardcoded [`generic_deck`] deck, and
/// [`check_device`] runs real DRC against it. Construct with `GenericPdk::default()`.
#[cfg(not(feature = "gpurify"))]
pub struct GenericPdk(generic_deck::DeckView);

#[cfg(not(feature = "gpurify"))]
impl Default for GenericPdk {
    fn default() -> Self {
        Self(generic_deck::DeckView::parse(generic_deck::GENERIC_DECK_JSON))
    }
}

/// Both forms answer from the **same deck**. This one reads it directly rather
/// than through `verify`, which is what the `gpurify` feature gates.
///
/// This used to be a hand-written `match` listing roles and rule values inline.
/// That made the crate silently PDK-*specific*: it was a second copy of the deck
/// that drifted from it, and a role the deck defined but the `match` had not been
/// updated for resolved to `None` — which every generator treated as "this
/// process has no such layer" and skipped drawing.
#[cfg(not(feature = "gpurify"))]
impl Process for GenericPdk {
    fn layer(&self, role: &str) -> Option<LayerId> {
        self.0.layer(role)
    }
    fn rule(&self, name: &str, default: i32) -> i32 {
        self.0.rule(name, default)
    }
    fn grid(&self) -> i32 {
        self.0.grid()
    }
}

#[cfg(feature = "gpurify")]
pub struct GenericPdk(verify::Pdk);

#[cfg(feature = "gpurify")]
impl Default for GenericPdk {
    fn default() -> Self {
        Self(
            verify::Pdk::from_json(generic_deck::GENERIC_DECK_JSON)
                .expect("hardcoded generic deck is valid JSON"),
        )
    }
}

#[cfg(feature = "gpurify")]
impl Process for GenericPdk {
    fn layer(&self, role: &str) -> Option<LayerId> {
        self.0.layer(role)
    }
    fn rule(&self, name: &str, default: i32) -> i32 {
        self.0.rule(name, default)
    }
    fn grid(&self) -> i32 {
        self.0.grid()
    }
}

#[cfg(feature = "gpurify")]
impl GenericPdk {
    /// The wrapped [`verify::Pdk`] — the deck [`check_device`] runs DRC against.
    pub(crate) fn pdk(&self) -> &verify::Pdk {
        &self.0
    }
}

// ===========================================================================
//  Variant library — the shipped Devices (substrate2 sky130 tiles)
// ===========================================================================

/// The built-in [`DeviceGen`] library. Meant to be "enough to use", so that
/// authoring a new Device (raw geometry) is the rare, opt-in path.
pub mod variants {
    use super::{
        Block, DeviceBuilder, DeviceGen, GenDevice, GenError, InOut, Input, Io, PortInfo,
        Process, Signal,
    };
    use pnr_core::DeviceKind;

    /// A transistor's terminals: gate in, drain/source/body bidirectional.
    #[derive(Default)]
    pub struct MosIo {
        pub g: Input<Signal>,
        pub d: InOut<Signal>,
        pub s: InOut<Signal>,
        pub b: InOut<Signal>,
    }
    impl Io for MosIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.g.port("g"), self.d.port("d"), self.s.port("s"), self.b.port("b")]
        }
    }

    /// A **multi-finger MOSFET** — `nf` poly gates over one continuous diffusion,
    /// with shared source/drain between adjacent fingers. Shared diffusion is
    /// entirely internal to this one Device (substrate2 `MosTile`); a matched
    /// pair is two of these wired by nets — or one [`MatchedPair`] when they
    /// must interdigitate.
    ///
    /// Geometry is **sourced from `cells::mosfet`** (the real, signoff-grade
    /// generator) via [`crate::adapter`] — macroMaster only exposes it.
    pub struct Mos {
        pub kind: DeviceKind,
        /// Finger width, `nm`.
        pub w: i32,
        /// Gate length, `nm`.
        pub l: i32,
        /// Finger count.
        pub nf: u16,
        /// Requested finger pattern; `None` = smallest legal variant.
        pub pattern: Option<cells::Pattern>,
        /// Requested dummies per edge; `None` = generator's choice.
        pub dummies_per_edge: Option<u8>,
    }

    impl Mos {
        /// The common case: no pattern/dummy preference.
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
            let (pat, dum, nf) = (self.pattern, self.dummies_per_edge, self.nf.max(1));
            let mac = crate::adapter::draw_cell_where::<cells::mosfet::Mosfet>(
                self.kind,
                self.w,
                self.l,
                nf,
                cell.process(),
                // `nf` is part of the filter, not just the sizing seed.
                // `cells::mosfet` treats the requested count as a *seed* and
                // enumerates every legal refold of the same total width; the
                // adapter then takes the smallest-area variant, which is always
                // the 1-finger fold. So without this the parameter was silently
                // discarded — `Mos::new(.., nf)` drew one finger for every `nf`,
                // and `devices()` below would report a finger count the layout
                // does not have, which is an LVS mismatch per device.
                move |m| {
                    m.nf == nf
                        && pat.is_none_or(|p| m.style == p)
                        && dum.is_none_or(|d| m.dummies_per_edge == d)
                },
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            // Forward the generator's pins under the Device-local port names
            // (`d0:G` → `g`) so a Composition can qualify and route them.
            for p in &mac.pins {
                let term = p.name.rsplit(':').next().unwrap_or(&p.name).to_ascii_lowercase();
                cell.pin(&term, p.layer, p.at)?;
            }
            cell.connect("g", "g");
            cell.connect("d", "d");
            cell.connect("s", "s");
            cell.connect("b", "b");
            Ok(())
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            Some(vec![mos_device(self.kind, self.w, self.l, self.nf, "")])
        }
    }

    /// One MOS [`GenDevice`], ports suffixed by `leg` (`""` ⇒ `g`/`d`/`s`,
    /// `"1"` ⇒ `g1`/`d1`/`s1`). Body is always the unsuffixed `b`: both
    /// [`Mos`] and [`MatchedPair`] declare a single shared bulk port.
    fn mos_device(kind: DeviceKind, w: i32, l: i32, nf: u16, leg: &str) -> GenDevice {
        GenDevice {
            kind,
            terminals: vec![
                ("D".into(), format!("d{leg}")),
                ("G".into(), format!("g{leg}")),
                ("S".into(), format!("s{leg}")),
                ("B".into(), "b".into()),
            ],
            params: vec![
                ("w".into(), i64::from(w)),
                ("l".into(), i64::from(l)),
                ("nf".into(), i64::from(nf.max(1))),
            ],
        }
    }

    /// A matched pair's terminals: per-leg gate/drain/source, shared body.
    #[derive(Default)]
    pub struct MatchedPairIo {
        pub g1: Input<Signal>,
        pub d1: InOut<Signal>,
        pub s1: InOut<Signal>,
        pub g2: Input<Signal>,
        pub d2: InOut<Signal>,
        pub s2: InOut<Signal>,
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

    /// Two matched MOSFETs drawn as **one interdigitated macro** (ABBA /
    /// checkerboard / ABAB per `pattern`) — the group-collapse geometry a
    /// diff pair or current mirror wants, exposed at the Device tier with
    /// per-leg pins (`d0:G` → `g1`, `d1:G` → `g2`). This is what makes merged
    /// matched groups *expressible* in generated code (emit v2).
    pub struct MatchedPair {
        pub kind: DeviceKind,
        /// Unit finger width, `nm`.
        pub w: i32,
        /// Gate length, `nm`.
        pub l: i32,
        /// Fingers per leg (both legs equal — the matched case).
        pub nf_each: u16,
        /// Interleaving pattern; `None` = smallest legal (usually Cc1d/ABBA).
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
            let mac = crate::adapter::draw_group_where::<cells::mosfet::Mosfet>(
                self.kind,
                self.w,
                self.l,
                &[nf, nf],
                cell.process(),
                // Per-leg finger count pinned, same reason as `Mos::layout`.
                move |m| m.nf == nf && pat.is_none_or(|p| m.style == p),
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            // `d{i}:{T}` → `{t}{i+1}`: member ordinal becomes the leg suffix.
            // Except `B`: [`MatchedPairIo`] declares *one* shared body, and the
            // two legs sit on one merged tap rail, so both bulk pins surface as
            // `b`. Suffixing them gave `b1`/`b2`, which no Io port and no
            // `connect` below ever named -- so both floated, `bind_pins` handed
            // each a singleton net, and the router drilled a via + met1 pad onto
            // two adjacent tap cuts one guard pitch apart. That is a met1
            // spacing violation per matched pair, on geometry nothing asked for.
            for p in &mac.pins {
                let term = match p.name.split_once(':') {
                    Some((_, "B")) => "b".to_string(),
                    Some((d, t)) => {
                        let leg = d.trim_start_matches('d').parse::<usize>().unwrap_or(0) + 1;
                        format!("{}{leg}", t.to_ascii_lowercase())
                    }
                    None => p.name.to_ascii_lowercase(),
                };
                cell.pin(&term, p.layer, p.at)?;
            }
            for t in ["g1", "d1", "s1", "g2", "d2", "s2", "b"] {
                cell.connect(t, t);
            }
            Ok(())
        }

        /// Two devices, one per leg — the whole point of the variant is that it
        /// is *two* transistors in one drawn macro, and the recogniser has to
        /// see both to match the pair.
        fn devices(&self) -> Option<Vec<GenDevice>> {
            let nf = self.nf_each.max(1);
            Some(vec![
                mos_device(self.kind, self.w, self.l, nf, "1"),
                mos_device(self.kind, self.w, self.l, nf, "2"),
            ])
        }
    }

    /// A two-terminal resistor's terminals (both bidirectional).
    #[derive(Default)]
    pub struct ResIo {
        pub a: InOut<Signal>,
        pub b: InOut<Signal>,
    }
    impl Io for ResIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.a.port("a"), self.b.port("b")]
        }
    }

    /// A resistor — geometry **sourced from `cells::resistor`** via
    /// [`crate::adapter`] (was a single poly bar). `w`/`len` are the body unit
    /// geometry the `cells` generator decomposes.
    pub struct Res {
        /// Body width, `nm`.
        pub w: i32,
        /// Body length, `nm`.
        pub len: i32,
    }
    impl Block for Res {
        type Io = ResIo;
        fn name(&self) -> String {
            format!("res_w{}_l{}", self.w, self.len)
        }
    }
    /// A capacitor's plates (both bidirectional).
    #[derive(Default)]
    pub struct CapIo {
        pub top: InOut<Signal>,
        pub bot: InOut<Signal>,
    }
    impl Io for CapIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.top.port("top"), self.bot.port("bot")]
        }
    }

    /// A unit-plate capacitor array — geometry from `cells::capacitor` (the
    /// metal-stack construction is the generator's variant choice).
    pub struct Cap {
        /// Unit plate side, `nm`.
        pub unit: i32,
        /// Total unit plates.
        pub units: u16,
    }
    impl Block for Cap {
        type Io = CapIo;
        fn name(&self) -> String {
            format!("cap_u{}_n{}", self.unit, self.units)
        }
    }
    impl DeviceGen for Cap {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let mac = crate::adapter::draw_cell::<cells::capacitor::Capacitor>(
                DeviceKind::Capacitor,
                self.unit,
                self.unit,
                self.units,
                cell.process(),
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            for p in &mac.pins {
                let term = p.name.rsplit(':').next().unwrap_or(&p.name).to_ascii_lowercase();
                cell.pin(&term, p.layer, p.at)?; // TOP/BOT → top/bot
            }
            cell.connect("top", "top");
            cell.connect("bot", "bot");
            Ok(())
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            Some(vec![two_terminal(
                DeviceKind::Capacitor,
                "top",
                "bot",
                vec![
                    ("w".into(), i64::from(self.unit)),
                    ("l".into(), i64::from(self.unit)),
                    ("m".into(), i64::from(self.units.max(1))),
                ],
            )])
        }
    }

    /// A two-terminal [`GenDevice`] on the schematic's `P`/`N` keys.
    fn two_terminal(
        kind: DeviceKind,
        p: &str,
        n: &str,
        params: Vec<(String, i64)>,
    ) -> GenDevice {
        GenDevice {
            kind,
            terminals: vec![("P".into(), p.into()), ("N".into(), n.into())],
            params,
        }
    }

    /// A BJT's terminals.
    #[derive(Default)]
    pub struct BjtIo {
        pub c: InOut<Signal>,
        pub b: InOut<Signal>,
        pub e: InOut<Signal>,
    }
    impl Io for BjtIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.c.port("c"), self.b.port("b"), self.e.port("e")]
        }
    }

    /// A unit-device BJT array — area ratios come from `units`, never emitter
    /// scaling (the 1:8 bandgap is eight of these), which is why there is no
    /// emitter-size parameter here at all.
    pub struct Bjt {
        pub kind: DeviceKind,
        /// Unit-device count.
        pub units: u16,
    }
    impl Block for Bjt {
        type Io = BjtIo;
        fn name(&self) -> String {
            format!("bjt_{:?}_n{}", self.kind, self.units)
        }
    }
    impl DeviceGen for Bjt {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let mac = crate::adapter::draw_cell::<cells::bjt::Bjt>(
                self.kind,
                0, // unit geometry comes from the deck's bjt_* construction dims
                0,
                self.units,
                cell.process(),
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            for p in &mac.pins {
                let term = p.name.rsplit(':').next().unwrap_or(&p.name).to_ascii_lowercase();
                cell.pin(&term, p.layer, p.at)?; // C/B/E → c/b/e
            }
            cell.connect("c", "c");
            cell.connect("b", "b");
            cell.connect("e", "e");
            Ok(())
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            Some(vec![GenDevice {
                kind: self.kind,
                terminals: vec![
                    ("C".into(), "c".into()),
                    ("B".into(), "b".into()),
                    ("E".into(), "e".into()),
                ],
                // Area ratio is unit count, never emitter scaling — so `m` is
                // the only size parameter a BJT has here.
                params: vec![("m".into(), i64::from(self.units.max(1)))],
            }])
        }
    }

    /// A diode's terminals: anode / cathode.
    #[derive(Default)]
    pub struct DiodeIo {
        pub a: InOut<Signal>,
        pub k: InOut<Signal>,
    }
    impl Io for DiodeIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.a.port("a"), self.k.port("k")]
        }
    }

    /// A junction diode — geometry from `cells::diode` (`diode_w`/`diode_l`
    /// construction dims size the unit; `fingers` parallels them).
    pub struct Diode {
        pub fingers: u16,
    }
    impl Block for Diode {
        type Io = DiodeIo;
        fn name(&self) -> String {
            format!("diode_n{}", self.fingers)
        }
    }
    impl DeviceGen for Diode {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let mac = crate::adapter::draw_cell::<cells::diode::Diode>(
                DeviceKind::Diode,
                0, // unit geometry from the deck's diode_w/diode_l dims
                0,
                self.fingers,
                cell.process(),
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            for p in &mac.pins {
                let term = p.name.rsplit(':').next().unwrap_or(&p.name).to_ascii_lowercase();
                cell.pin(&term, p.layer, p.at)?; // A/K → a/k
            }
            cell.connect("a", "a");
            cell.connect("k", "k");
            Ok(())
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            // Anode is the schematic's `P`, cathode its `N`.
            Some(vec![two_terminal(
                DeviceKind::Diode,
                "a",
                "k",
                vec![("nf".into(), i64::from(self.fingers.max(1)))],
            )])
        }
    }

    impl DeviceGen for Res {
        fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
            let mac = crate::adapter::draw_cell::<cells::resistor::Resistor>(
                DeviceKind::Resistor,
                self.w,
                self.len,
                1,
                cell.process(),
            );
            for s in &mac.shapes {
                cell.draw(s.layer, s.rect)?;
            }
            // `cells::resistor` names its terminals `d0:P`/`d0:N`; the Io here
            // calls them `a`/`b`. Anything else forwards lowercased.
            for p in &mac.pins {
                let raw = p.name.rsplit(':').next().unwrap_or(&p.name);
                let term = match raw {
                    "P" => "a".to_string(),
                    "N" => "b".to_string(),
                    t => t.to_ascii_lowercase(),
                };
                cell.pin(&term, p.layer, p.at)?;
            }
            cell.connect("a", "b"); // one body, both terminals on it
            Ok(())
        }

        fn devices(&self) -> Option<Vec<GenDevice>> {
            Some(vec![two_terminal(
                DeviceKind::Resistor,
                "a",
                "b",
                vec![("w".into(), i64::from(self.w)), ("l".into(), i64::from(self.len))],
            )])
        }
    }
}

// ===========================================================================
//  Macros registry + GenError
// ===========================================================================

/// User-registered macros, keyed by netlist instance/subckt name. Orchestration
/// consults this **before** `cells`: a matched name uses the injected macro;
/// everything else is auto-drawn. The "manual block as a pass-in" path.
#[derive(Default)]
pub struct Macros {
    entries: Vec<(String, Macro)>,
}

impl Macros {
    /// Register a realised macro for `name`. Later registration shadows earlier.
    pub fn register(&mut self, name: &str, m: Macro) {
        if let Some(slot) = self.entries.iter_mut().find(|(n, _)| n == name) {
            slot.1 = m;
        } else {
            self.entries.push((name.to_string(), m));
        }
    }

    /// Look up an injected macro; `None` ⇒ fall through to `cells` auto-draw.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Macro> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, m)| m)
    }

    /// Open a viewer window on a registered macro and block until closed.
    /// No-op (with a warning) if `name` isn't registered.
    #[cfg(feature = "display")]
    pub fn show(&self, name: &str) {
        match self.get(name) {
            Some(m) => display(m, name),
            None => eprintln!("macro_master::Macros::show: no macro named {name:?}"),
        }
    }
}

/// Display a macro's drawn geometry in the GPU viewer, blocking until the window
/// is closed. The macroMaster → visualizer bridge (kernel-internal; `library`
/// re-exports the same viewer for the flow side).
#[cfg(feature = "display")]
pub fn display(mac: &Macro, title: &str) {
    visualizer::show_macro(mac, title);
}

/// Failure while a generator draws or places — the errors detectable **without**
/// a PDK rule table. Everything spacing-related is a downstream
/// [`pnr_core::Violation`], not a `GenError`.
#[derive(Debug, PartialEq, Eq)]
pub enum GenError {
    /// A coordinate did not land on the fabrication grid.
    OffGrid,
    /// A layer role the bound process does not define.
    IllegalLayer,
    /// A declared terminal was left floating (no `connect` reaches it).
    Disconnected,
    /// A placed device's bbox overlaps an already-placed device — the
    /// by-construction placement invariant Composition enforces.
    Overlap,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};

    // A tiny composition of two Mos placed side-by-side (no overlap), wired to
    // the four external nets — the S1/S3/S4 fixture.
    #[derive(Default)]
    struct PairIo {
        a: InOut<Signal>,
        b: InOut<Signal>,
        tail: InOut<Signal>,
        body: InOut<Signal>,
    }
    impl Io for PairIo {
        fn ports(&self) -> Vec<PortInfo> {
            vec![self.a.port("a"), self.b.port("b"), self.tail.port("tail"), self.body.port("body")]
        }
    }
    struct GoodPair;
    impl Block for GoodPair {
        type Io = PairIo;
        fn name(&self) -> String {
            "goodpair".into()
        }
    }
    impl Composition for GoodPair {
        fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
            let mos = || variants::Mos::new(DeviceKind::Nmos, 200, 60, 2);
            let li = c.instantiate("m1", &mos())?;
            let left = c.place(li)?;
            let ri = c.instantiate("m2", &mos())?;
            let right = c.place_by(ri, AlignMode::ToTheRight, &left, 80)?;
            c.connect(&left.term("g"), "a");
            c.connect(&right.term("g"), "b");
            c.connect(&left.term("s"), "tail");
            c.connect(&right.term("s"), "tail");
            c.connect(&left.term("b"), "body");
            c.connect(&right.term("b"), "body");
            Ok(())
        }
    }
    impl Schematic for GoodPair {
        fn schematic(&self) -> Netlist {
            let nets = ["a", "b", "tail", "body"]
                .iter()
                .map(|n| Net { name: (*n).into() })
                .collect();
            let mos = |name: &str| Device {
                name: name.into(),
                kind: DeviceKind::Nmos,
                terminals: vec![("G".into(), NetId(0)), ("S".into(), NetId(2))],
                params: vec![("W".into(), 200)],
            };
            Netlist { nets, devices: vec![mos("MA"), mos("MB")] }
        }
    }

    // S1 (structural, default build): the toy variant is on-grid + fully wired,
    // so the structural Device check is clean.
    #[cfg(not(feature = "gpurify"))]
    #[test]
    fn variant_mos_structurally_clean_on_generic() {
        let r = check_device(&variants::Mos::new(DeviceKind::Nmos, 200, 60, 3), &GenericPdk::default());
        assert!(r.hard_violations.is_empty(), "multi-finger MOS is on-grid + wired");
    }

    // S1 (gpurify): a MOS is now drawn by the real `cells::mosfet` generator via
    // the adapter — but at `l=60`, under the deck's 150nm poly min-width, so it
    // still fails. This is the forcing function working on *real* geometry: a
    // sub-minimum gate is caught by the stiffened deck regardless of who drew it.
    // (A properly-sized MOS passing is the goal of the `cells` DRC hardening —
    // macroMaster TODO #3/#4.)
    #[cfg(feature = "gpurify")]
    #[test]
    fn toy_variant_mos_fails_stiffened_generic() {
        let r = check_device(&variants::Mos::new(DeviceKind::Nmos, 200, 60, 3), &GenericPdk::default());
        assert!(
            r.hard_violations.iter().any(|v| v.rule.starts_with("drc/")),
            "under-sized toy gate must be caught by the stiffened deck, got: {:?}",
            r.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>()
        );
    }

    // S2: overlapping two devices is rejected at placement time.
    #[test]
    fn placing_overlapping_devices_is_rejected() {
        struct Overlap;
        impl Block for Overlap {
            type Io = PairIo;
            fn name(&self) -> String {
                "overlap".into()
            }
        }
        impl Composition for Overlap {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 200, 60, 2))?;
                let b = c.instantiate("m2", &variants::Mos::new(DeviceKind::Nmos, 200, 60, 2))?;
                c.place(a)?;
                c.place(b)?; // second at the origin ⇒ overlaps the first
                Ok(())
            }
        }
        let r = check(&Overlap, &GenericPdk::default());
        assert!(
            r.hard_violations.iter().any(|v| v.rule.contains("overlap")),
            "second device at the origin must be rejected as an overlap"
        );
    }

    // S2 (positive): a legal side-by-side abut places clean.
    #[test]
    fn abutting_devices_places_clean() {
        let r = check(&GoodPair, &GenericPdk::default());
        assert!(r.hard_violations.is_empty(), "flush-abutted, wired pair is clean");
    }

    // S3: structural LVS passes when connect-graph matches the schematic...
    #[test]
    fn structural_lvs_matches() {
        let r = check_lvs(&GoodPair, &GenericPdk::default());
        assert!(r.hard_violations.is_empty(), "2 devices, nets {{a,b,tail,body}} match schematic");
    }

    // ...and fails when the schematic disagrees (wrong device count).
    #[test]
    fn structural_lvs_catches_device_count_mismatch() {
        struct WrongCount;
        impl Block for WrongCount {
            type Io = PairIo;
            fn name(&self) -> String {
                "wrongcount".into()
            }
        }
        impl Composition for WrongCount {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                GoodPair.build(c) // instantiates 2 devices, wires 4 nets
            }
        }
        impl Schematic for WrongCount {
            fn schematic(&self) -> Netlist {
                // Claims a single device — must not match the two that were built.
                let nets = ["a", "b", "tail", "body"]
                    .iter()
                    .map(|n| Net { name: (*n).into() })
                    .collect();
                Netlist {
                    nets,
                    devices: vec![Device {
                        name: "MA".into(),
                        kind: DeviceKind::Nmos,
                        terminals: vec![],
                        params: vec![],
                    }],
                }
            }
        }
        let r = check_lvs(&WrongCount, &GenericPdk::default());
        assert!(
            r.hard_violations.iter().any(|v| v.rule.contains("lvs")),
            "1 schematic device vs 2 built ⇒ structural LVS violation"
        );
    }

    // S4: a declared port no `connect` reaches is caught as floating.
    #[test]
    fn floating_declared_port_is_caught() {
        struct Floaty;
        impl Block for Floaty {
            type Io = PairIo; // declares a,b,tail,body
            fn name(&self) -> String {
                "floaty".into()
            }
        }
        impl Composition for Floaty {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                let a = c.instantiate("m1", &variants::Mos::new(DeviceKind::Nmos, 200, 60, 1))?;
                let m1 = c.place(a)?;
                c.connect(&m1.term("g"), "a"); // b, tail, body left floating
                Ok(())
            }
        }
        let r = check(&Floaty, &GenericPdk::default());
        assert!(
            r.hard_violations.iter().any(|v| v.rule.contains("floating") || v.rule.contains("disconnected")),
            "unconnected declared ports must be flagged"
        );
    }

    // Proof the gpurify seam genuinely runs the engine (not a vacuous empty
    // store): a poly that is on-grid — so `draw` accepts it — but far below the
    // deck's poly min-width must come back as a real DRC violation.
    #[cfg(feature = "gpurify")]
    #[test]
    fn gpurify_device_check_catches_sub_min_width() {
        use pnr_core::Rect;
        #[derive(Default)]
        struct OneIo(InOut<Signal>);
        impl Io for OneIo {
            fn ports(&self) -> Vec<PortInfo> {
                vec![self.0.port("a")]
            }
        }
        struct Skinny;
        impl Block for Skinny {
            type Io = OneIo;
            fn name(&self) -> String {
                "skinny".into()
            }
        }
        impl DeviceGen for Skinny {
            fn layout<P: Process>(&self, cell: &mut DeviceBuilder<P>) -> Result<(), GenError> {
                let poly = cell.process().layer("poly").ok_or(GenError::IllegalLayer)?;
                // 5nm wide: on-grid (draw passes) but way under poly min-width (50).
                cell.draw(poly, Rect { x: 0, y: 0, w: 5, h: 500 })?;
                cell.connect("a", "a");
                Ok(())
            }
        }
        let r = check_device(&Skinny, &GenericPdk::default());
        assert!(
            r.hard_violations.iter().any(|v| v.rule.starts_with("drc/")),
            "real gpurify DRC must flag the 5nm poly, got: {:?}",
            r.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>()
        );
    }

    // M1: the composed Macro keeps every Device pin, qualified by instance name
    // — the property that makes a Composition routable downstream.
    #[test]
    fn composition_macro_retains_qualified_pins() {
        let pdk = GenericPdk::default();
        let mut b = cells::Builder::new(pdk.grid());
        {
            let mut c = CompBuilder::new(&mut b, &pdk);
            GoodPair.build(&mut c).expect("GoodPair builds clean");
        }
        let mac = b.finish();
        // No `b` here: `cells::mosfet` emits a body pin only when it draws a
        // tap, which this minimal variant does not — a `cells` gap, not ours.
        for want in ["m1.g", "m1.d", "m1.s", "m2.g", "m2.d", "m2.s"] {
            assert!(
                mac.pins.iter().any(|p| p.name == want),
                "composed macro must retain pin {want}, has {:?}",
                mac.pins.iter().map(|p| &p.name).collect::<Vec<_>>()
            );
        }
    }

    // M1: connect edges resolve into port-named net classes via union-find.
    #[test]
    fn nets_resolve_to_port_named_classes() {
        let edges = vec![
            ("m1.s".to_string(), "tail".to_string()),
            ("m2.s".to_string(), "m1.s".to_string()), // transitively `tail`
            ("m1.d".to_string(), "m2.g".to_string()), // internal net, no port
        ];
        let ports = vec!["tail".to_string()];
        let (names, binding) = resolve_nets(&edges, &ports);
        assert_eq!(names.len(), 2, "two classes: tail + one internal");
        assert_eq!(names[binding["m2.s"]], "tail", "transitive union reaches the port name");
        assert!(names[binding["m1.d"]].starts_with("net"), "internal class gets synthesized name");
    }

    // M1: bound pins carry the resolved NetId; unwired pins get singleton nets.
    #[test]
    fn bind_pins_assigns_resolved_net_ids() {
        let pdk = GenericPdk::default();
        let mut b = cells::Builder::new(pdk.grid());
        let edges = {
            let mut c = CompBuilder::new(&mut b, &pdk);
            GoodPair.build(&mut c).expect("GoodPair builds clean");
            c.nets_edges()
        };
        let mut mac = b.finish();
        let ports: Vec<String> =
            GoodPair.io().ports().into_iter().map(|p| p.name).collect();
        let names = bind_pins(&mut mac, &edges, &ports);
        let net_of = |pin: &str| {
            let p = mac.pins.iter().find(|p| p.name == pin).expect(pin);
            names[p.net.0 as usize].clone()
        };
        assert_eq!(net_of("m1.g"), "a");
        assert_eq!(net_of("m2.g"), "b");
        assert_eq!(net_of("m1.s"), "tail");
        assert_eq!(net_of("m2.s"), "tail");
        assert!(net_of("m1.d").starts_with("net"), "unwired drain gets a singleton net");
        assert_ne!(net_of("m1.d"), net_of("m2.d"), "unwired pins are separate nets");
    }

    // M2 seam: build_composition's flat and per-instance views agree on NetIds,
    // and instance macros sit at their placed positions.
    #[test]
    fn build_composition_views_agree() {
        let built = build_composition(&GoodPair, &GenericPdk::default()).expect("builds");
        assert_eq!(built.instances.len(), 2);
        let flat_net = |pin: &str| built.flat.pins.iter().find(|p| p.name == pin).expect(pin).net;
        for (_, mac) in &built.instances {
            for p in &mac.pins {
                assert_eq!(p.net, flat_net(&p.name), "flat and instance NetId agree for {}", p.name);
            }
        }
        assert_eq!(built.nets[flat_net("m1.s").0 as usize], "tail");
        // m2 was placed to the right of m1 — its macro is translated, not at origin.
        let (n1, m1) = &built.instances[0];
        let (n2, m2) = &built.instances[1];
        assert_eq!((n1.as_str(), n2.as_str()), ("m1", "m2"));
        assert!(m2.bbox.x > m1.bbox.x, "m2 sits to the right of m1");
    }

    // M3: a mirrored partner is exactly symmetric about the recorded axis —
    // every pin position reflects, names and nets survive.
    #[test]
    fn place_mirrored_is_exactly_symmetric() {
        struct MirrorPair;
        impl Block for MirrorPair {
            type Io = PairIo;
            fn name(&self) -> String {
                "mirrorpair".into()
            }
        }
        impl Composition for MirrorPair {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                let mos = || variants::Mos::new(DeviceKind::Nmos, 200, 60, 2);
                let li = c.instantiate("m1", &mos())?;
                let left = c.place(li)?;
                let ri = c.instantiate("m2", &mos())?;
                let right = c.place_mirrored(ri, &left, 100)?;
                assert_eq!(right.orient(), pnr_core::Orient::Mx180);
                c.connect(&left.term("g"), "a");
                c.connect(&right.term("g"), "b");
                c.connect(&left.term("s"), "tail");
                c.connect(&right.term("s"), "tail");
                c.connect(&left.term("b"), "body");
                c.connect(&right.term("b"), "body");
                Ok(())
            }
        }
        let built = build_composition(&MirrorPair, &GenericPdk::default()).expect("builds");
        assert_eq!(built.sym_pairs.len(), 1);
        let (l, r, axis) = built.sym_pairs[0].clone();
        assert_eq!((l.as_str(), r.as_str()), ("m1", "m2"));
        let (_, m1) = &built.instances[0];
        let (_, m2) = &built.instances[1];
        // Bboxes reflect about the axis...
        assert_eq!(m2.bbox.x, 2 * axis - (m1.bbox.x + m1.bbox.w));
        assert_eq!((m2.bbox.w, m2.bbox.h), (m1.bbox.w, m1.bbox.h));
        // ...and so does every pin, name-for-name.
        for p1 in &m1.pins {
            let local = p1.name.split('.').nth(1).unwrap();
            let p2 = m2
                .pins
                .iter()
                .find(|p| p.name == format!("m2.{local}") && p.at.y == p1.at.y
                    && p.at.x == 2 * axis - (p1.at.x + p1.at.w))
                .unwrap_or_else(|| panic!("no mirrored partner for {}", p1.name));
            assert_eq!((p2.at.w, p2.at.h), (p1.at.w, p1.at.h));
        }
    }

    // M4: a Composition instantiating a Composition — child ports surface as
    // `x1.<port>` pins, child internal nets stay whole in the parent's graph.
    #[test]
    fn hierarchical_composition_keeps_child_nets_whole() {
        struct TwoPairs;
        impl Block for TwoPairs {
            type Io = PairIo;
            fn name(&self) -> String {
                "twopairs".into()
            }
        }
        impl Composition for TwoPairs {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                let x1 = c.instantiate_comp("x1", &GoodPair)?;
                let x1 = c.place(x1)?;
                let x2 = c.instantiate_comp("x2", &GoodPair)?;
                let x2 = c.place_by(x2, AlignMode::Above, &x1, 200)?;
                c.connect(&x1.term("a"), "a");
                c.connect(&x1.term("b"), "b");
                c.connect(&x1.term("tail"), &x2.term("tail"));
                c.connect(&x1.term("tail"), "tail");
                c.connect(&x1.term("body"), &x2.term("body"));
                c.connect(&x1.term("body"), "body");
                c.connect(&x2.term("a"), "a");
                c.connect(&x2.term("b"), "b");
                Ok(())
            }
        }
        let built = build_composition(&TwoPairs, &GenericPdk::default()).expect("builds");
        assert_eq!(built.instances.len(), 2);
        let net_of = |pin: &str| {
            let p = built.flat.pins.iter().find(|p| p.name == pin).unwrap_or_else(|| {
                panic!("pin {pin} missing, have {:?}",
                    built.flat.pins.iter().map(|p| &p.name).collect::<Vec<_>>())
            });
            built.nets[p.net.0 as usize].clone()
        };
        // Child port pins surface with the port name and land on the parent net
        // (the gate pin was on child net `a`, a port, so it *is* `x1.a` now).
        assert_eq!(net_of("x1.tail"), "tail");
        assert_eq!(net_of("x2.tail"), "tail", "cross-instance tie via parent edge");
        assert_eq!(net_of("x1.a"), "a");
        assert_eq!(net_of("x2.b"), "b");
        // Child *internal* pins keep their qualified names and stay private:
        // the two children's unwired drains are distinct singleton nets.
        assert_ne!(net_of("x1.m1.d"), net_of("x2.m1.d"), "internals do not merge across instances");
    }

    #[test]
    fn overlaps_treats_flush_abut_as_legal() {
        let a = Rect { x: 0, y: 0, w: 100, h: 50 };
        let flush = Rect { x: 100, y: 0, w: 100, h: 50 }; // shares the x=100 edge
        let over = Rect { x: 90, y: 0, w: 100, h: 50 };
        assert!(!overlaps(a, flush), "shared boundary is not overlap");
        assert!(overlaps(a, over), "10nm interior intrusion is overlap");
    }

    // M5: the widened library is structurally clean on the generic deck.
    #[cfg(not(feature = "gpurify"))]
    #[test]
    fn widened_library_structurally_clean() {
        let g = GenericPdk::default();
        let r = check_device(&variants::Cap { unit: 2000, units: 4 }, &g);
        assert!(r.hard_violations.is_empty(), "cap: {:?}", r.hard_violations.len());
        let r = check_device(&variants::Bjt { kind: DeviceKind::Npn, units: 2 }, &g);
        assert!(r.hard_violations.is_empty(), "bjt: {:?}", r.hard_violations.len());
        let r = check_device(&variants::Diode { fingers: 2 }, &g);
        assert!(r.hard_violations.is_empty(), "diode: {:?}", r.hard_violations.len());
    }

    // M5: a MatchedPair draws ONE macro with per-leg pins — the merged
    // matched-group geometry, expressible at the Device tier.
    #[test]
    fn matched_pair_exposes_per_leg_pins() {
        let g = GenericPdk::default();
        let pair = variants::MatchedPair {
            kind: DeviceKind::Nmos,
            w: 200,
            l: 60,
            nf_each: 2,
            pattern: None,
        };
        let mut b = cells::Builder::new(g.grid());
        {
            let mut db = DeviceBuilder::new(&mut b, &g);
            pair.layout(&mut db).expect("pair draws");
        }
        let mac = b.finish();
        for leg in ["g1", "d1", "s1", "g2", "d2", "s2"] {
            assert!(
                mac.pins.iter().any(|p| p.name == leg),
                "missing per-leg pin {leg}, have {:?}",
                mac.pins.iter().map(|p| &p.name).collect::<Vec<_>>()
            );
        }
    }

    // M5: the topology parameters select the drawn alternative. A lone device
    // only enumerates `Single` (interleaving needs a partner — the filter
    // falls back, by design), so the discriminating knob here is the dummy
    // count; pattern discrimination is covered by `MatchedPair`.
    #[test]
    fn mos_dummies_parameter_selects_variant() {
        let g = GenericPdk::default();
        let draw = |dummies| {
            let mut m = variants::Mos::new(DeviceKind::Nmos, 200, 60, 4);
            m.dummies_per_edge = dummies;
            let mut b = cells::Builder::new(g.grid());
            {
                let mut db = DeviceBuilder::new(&mut b, &g);
                m.layout(&mut db).expect("draws");
            }
            b.finish()
        };
        let one = draw(Some(1));
        let two = draw(Some(2));
        assert!(
            one.shapes != two.shapes || one.bbox != two.bbox,
            "dummy parameter had no effect on the drawn geometry"
        );
    }

    // M4: the structural LVS counts **transistors**, not placed instances — a
    // hierarchy is one instance per child and many devices. Regression for
    // `check_lvs` reporting "built 2, schematic 4" on a correct composition.
    #[test]
    fn hierarchical_structural_lvs_counts_devices_not_instances() {
        struct TwoPairs;
        impl Block for TwoPairs {
            type Io = PairIo;
            fn name(&self) -> String {
                "twopairs".into()
            }
        }
        impl Composition for TwoPairs {
            fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
                let x1 = c.instantiate_comp("x1", &GoodPair)?;
                let x1 = c.place(x1)?;
                let x2 = c.instantiate_comp("x2", &GoodPair)?;
                let x2 = c.place_by(x2, AlignMode::Above, &x1, 200)?;
                c.connect(&x1.term("a"), "a");
                c.connect(&x1.term("b"), "b");
                c.connect(&x1.term("tail"), &x2.term("tail"));
                c.connect(&x1.term("tail"), "tail");
                c.connect(&x1.term("body"), &x2.term("body"));
                c.connect(&x1.term("body"), "body");
                c.connect(&x2.term("a"), "a");
                c.connect(&x2.term("b"), "b");
                Ok(())
            }
        }
        impl Schematic for TwoPairs {
            fn schematic(&self) -> Netlist {
                let nets = ["a", "b", "tail", "body"]
                    .iter()
                    .map(|n| Net { name: (*n).into() })
                    .collect();
                let mos = |name: &str| Device {
                    name: name.into(),
                    kind: DeviceKind::Nmos,
                    terminals: vec![("G".into(), NetId(0)), ("S".into(), NetId(2))],
                    params: vec![("W".into(), 200)],
                };
                Netlist { nets, devices: vec![mos("M1"), mos("M2"), mos("M3"), mos("M4")] }
            }
        }
        // ...and the build really does hold four, named through the hierarchy.
        let built = build_composition(&TwoPairs, &GenericPdk::default()).expect("builds");
        let names: Vec<&str> = built
            .netlist
            .as_ref()
            .expect("all-declared hierarchy has a netlist")
            .devices
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, ["x1.m1", "x1.m2", "x2.m1", "x2.m2"]);

        let r = check_lvs(&TwoPairs, &GenericPdk::default());
        assert!(
            r.hard_violations.is_empty(),
            "4 transistors in 2 sub-instances must match a 4-device schematic, got {:?}",
            r.hard_violations.iter().map(|v| v.rule.clone()).collect::<Vec<_>>()
        );
    }
}
