//! Typed PDKs: a Substrate schema for every Philis deck, and the device
//! blocks Philis draws on them.
//!
//! - [`gf180mcu`], [`ihp_sg13g2`], [`asap7`] are generated from their decks
//!   by `build.rs`: a `Layer` enum (one variant per drawn GDS pair), a `Model`
//!   enum (the deck's MOS models), and the schema type.
//! - sky130 is Substrate2's own [`sky130::Sky130`], with [`PhilisPdk`]
//!   implemented on it.
//!
//! Devices follow Substrate's split (upstream sky130's `Nfet01v8` vs
//! `NmosTile`): a schematic primitive and a layout tile are separate blocks,
//! and the block that composes them draws the joins.
//!
//! - [`Mos`]: one transistor's schematic primitive, on any of the four.
//! - [`MosTile`]: its layout on a generated PDK ([`DrawnPdk`]), drawn by
//!   Philis's `cells` generator (planar, or fins on a deck with a `fin`
//!   layer, with the markers a flavoured model's recogniser needs). Each
//!   terminal is an array with one entry per region the drawing leaves
//!   unjoined (a two-finger planar device's two sources). sky130 uses
//!   upstream's own tiles.
//!
//! Units are nm. A [`Mos`]'s `w` is the total width over its `nf` fingers, as
//! in SPICE.

use std::any::Any;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, OnceLock};

use arcstr::ArcStr;
use gds::GdsUnits;
use gdsconv::GdsLayer;
use layir::{Cell, Element, Instance, LibraryBuilder, Text};
use pnr_core::{DeviceKind, LayerId, Macro, Process};
use rust_decimal::Decimal;
use spice::Spice;
use substrate::block::Block;
use substrate::context::Context;
use substrate::geometry::bbox::Bbox;
use substrate::geometry::rect::Rect;
use substrate::geometry::transform::Transformation;
use substrate::layout::{CellBundle, Layout};
use substrate::schematic::{ConvertSchema, PrimitiveBinding, Schematic};
use substrate::types::codegen::{PortGeometryBundle, View};
use substrate::types::layout::PortGeometry;
use substrate::types::schematic::IoNodeBundle;
use substrate::types::{Array, ArrayBundle, InOut, Input, Io, MosIo, Signal};
use substrate::{layout, schematic};
use unicase::UniCase;

/// A PDK Philis has a deck for.
pub trait PhilisPdk: layout::schema::Schema + schematic::schema::Schema + Sized {
    /// The name [`verify::Pdk::builtin`] loads.
    const DECK: &'static str;
    /// This PDK's MOS models.
    type Model: MosModel<Pdk = Self>;

    /// The deck (rules, recognisers, GDS map), loaded once.
    fn deck() -> &'static verify::Pdk;
    /// The GDS drawing pair of `layer`.
    fn gds(layer: &<Self as layout::schema::Schema>::Layer) -> GdsLayer;
    /// The layer drawn on GDS pair `gds`.
    fn from_gds(gds: GdsLayer) -> Option<<Self as layout::schema::Schema>::Layer>;
    /// The schematic primitive for one MOS: `w` total over `nf` fingers, nm.
    fn mos(model: Self::Model, w: i64, l: i64, nf: i64) -> <Self as scir::schema::Schema>::Primitive;
    /// `block`'s schematic as a [`Spice`] netlist (upstream sky130 goes
    /// through its open-PDK schema).
    ///
    /// # Errors
    /// Substrate failing to generate or convert the schematic.
    fn export_spice<B: Schematic<Schema = Self>>(ctx: &Context, block: B) -> substrate::error::Result<scir::Library<Spice>>;
}

/// A MOS model of one PDK.
pub trait MosModel: Copy + Debug + Eq + Send + Sync + Any {
    type Pdk: PhilisPdk<Model = Self>;
    /// The model name the deck's recogniser takes.
    fn name(self) -> &'static str;
    fn is_pmos(self) -> bool;
}

/// A generated PDK's schematic primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive<M> {
    /// A MOS with ports `D G S B`; `w` total over `nf` fingers, nm.
    Mos { model: M, w: i64, l: i64, nf: i64 },
}

impl<M: MosModel> Primitive<M> {
    /// The SPICE card: an `M` card of the deck's model, lengths in metres.
    ///
    /// ponytail: the card form each simulator's model files want (`X` subckt
    /// vs `M`, `nfin`) is still open (plan-09 Next); LVS reads this one.
    fn to_spice(&self) -> spice::Primitive {
        let Primitive::Mos { model, w, l, nf } = *self;
        let p = |k: &str, v: Decimal| (UniCase::new(ArcStr::from(k)), scir::ParamValue::Numeric(v));
        spice::Primitive::Mos {
            model: model.name().into(),
            params: HashMap::from([p("w", Decimal::new(w, 9)), p("l", Decimal::new(l, 9)), p("nf", Decimal::from(nf))]),
        }
    }
}

/// One transistor: `w` total width over `nf` fingers, `l` length, nm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mos<M> {
    pub model: M,
    pub w: i64,
    pub l: i64,
    pub nf: i64,
}

// By model name: upstream sky130's `MosKind` is not `Hash`.
impl<M: MosModel> Hash for Mos<M> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (self.model.name(), self.w, self.l, self.nf).hash(h);
    }
}

impl<M: MosModel> Mos<M> {
    #[must_use]
    pub fn new(model: M, w: i64, l: i64, nf: i64) -> Self {
        Self { model, w, l, nf }
    }
}

impl<M: MosModel> Block for Mos<M> {
    type Io = MosIo;

    fn name(&self) -> ArcStr {
        arcstr::format!("{}_w{}_l{}_nf{}", self.model.name(), self.w, self.l, self.nf)
    }

    fn io(&self) -> MosIo {
        MosIo::default()
    }
}

impl<M: MosModel> Schematic for Mos<M> {
    type Schema = M::Pdk;
    type NestedData = ();

    fn schematic(&self, io: &IoNodeBundle<Self>, cell: &mut schematic::CellBuilder<M::Pdk>) -> substrate::error::Result<()> {
        let mut prim = PrimitiveBinding::new(M::Pdk::mos(self.model, self.w, self.l, self.nf));
        prim.connect("D", io.d);
        prim.connect("G", io.g);
        prim.connect("S", io.s);
        prim.connect("B", io.b);
        cell.set_primitive(prim);
        Ok(())
    }
}

/// A PDK whose devices Philis's `cells` generators draw (the generated
/// ones; sky130 has upstream's tiles).
pub trait DrawnPdk: PhilisPdk {}

/// [`MosTile`]'s terminals: per terminal, one entry per region the drawing
/// leaves unjoined, left to right.
#[derive(Debug, Clone, Io)]
pub struct MosTileIo {
    pub d: InOut<Array<Signal>>,
    pub g: Input<Array<Signal>>,
    pub s: InOut<Array<Signal>>,
    pub b: InOut<Array<Signal>>,
}

/// One transistor's layout, drawn by `cells`: `w` total width over `nf`
/// fingers, `l` length, nm. Its schematic is [`Mos`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MosTile<M>(pub Mos<M>);

impl<M: MosModel> Hash for MosTile<M> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.hash(h);
    }
}

/// The terminals [`MosTileIo`] carries, in field order.
const TERMINALS: [&str; 4] = ["D", "G", "S", "B"];

impl<M: MosModel> MosTile<M>
where
    M::Pdk: DrawnPdk,
{
    fn draw(&self) -> Macro {
        let m = self.0;
        draw_mos(M::Pdk::deck(), m.model.name(), m.model.is_pmos(), m.w, m.l, m.nf)
    }
}

impl<M: MosModel> Block for MosTile<M>
where
    M::Pdk: DrawnPdk,
{
    type Io = MosTileIo;

    fn name(&self) -> ArcStr {
        arcstr::format!("{}_tile", self.0.name())
    }

    fn io(&self) -> MosTileIo {
        let regions = regions(&self.draw());
        let n = |t: &str| Array::new(regions.iter().filter(|(term, _)| term == t).count(), Signal::new());
        MosTileIo { d: InOut(n("D")), g: Input(n("G")), s: InOut(n("S")), b: InOut(n("B")) }
    }
}

impl<M: MosModel> Layout for MosTile<M>
where
    M::Pdk: DrawnPdk,
{
    type Schema = M::Pdk;
    type Bundle = View<MosTileIo, PortGeometryBundle<M::Pdk>>;
    type Data = ();

    fn layout(&self, cell: &mut layout::CellBuilder<M::Pdk>) -> substrate::error::Result<(Self::Bundle, ())> {
        let mac = self.draw();
        let layer = layer_map::<M::Pdk>();
        for s in &mac.shapes {
            cell.draw(layir::Shape::new(layer(s.layer)?, rect(s.rect)))?;
        }
        let mut arrays: Vec<Vec<PortGeometry<_>>> = vec![Vec::new(); TERMINALS.len()];
        for (term, pins) in regions(&mac) {
            let shapes = pins.iter().map(|p| Ok(layir::Shape::new(layer(p.layer)?, rect(p.at)))).collect::<substrate::error::Result<Vec<_>>>()?;
            let mut shapes = shapes.into_iter();
            let mut g = PortGeometry::new(shapes.next().expect("a region has a pin"));
            g.unnamed_shapes.extend(shapes);
            arrays[TERMINALS.iter().position(|t| *t == term).expect("a MOS terminal")].push(g);
        }
        let mut arrays = arrays.into_iter().map(|v| ArrayBundle::new(Signal, v));
        let mut next = || arrays.next().expect("one array per terminal");
        Ok((CellBundle::<Self> { d: next(), g: next(), s: next(), b: next() }, ()))
    }
}

/// The drawing's pins grouped into regions: same terminal, joined through
/// shapes on the pin's own layer. In `TERMINALS` order, then left to right.
fn regions(mac: &Macro) -> Vec<(String, Vec<pnr_core::Pin>)> {
    let touch = |a: pnr_core::Rect, b: pnr_core::Rect| a.x <= b.x + b.w && b.x <= a.x + a.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
    // Union-find over the shapes of each layer, so a pin's island is the
    // component of a shape it touches.
    let shapes = &mac.shapes;
    let mut parent: Vec<usize> = (0..shapes.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    for i in 0..shapes.len() {
        for j in i + 1..shapes.len() {
            if shapes[i].layer == shapes[j].layer && touch(shapes[i].rect, shapes[j].rect) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                parent[a] = b;
            }
        }
    }
    let mut out: Vec<(String, Option<usize>, Vec<pnr_core::Pin>)> = Vec::new();
    let mut pins: Vec<&pnr_core::Pin> = mac.pins.iter().collect();
    pins.sort_by_key(|p| (p.at.x, p.at.y));
    for p in pins {
        let term = p.name.rsplit(':').next().unwrap_or(&p.name).to_ascii_uppercase();
        let island = (0..shapes.len()).find(|&i| shapes[i].layer == p.layer && touch(shapes[i].rect, p.at)).map(|i| find(&mut parent, i));
        match out.iter_mut().find(|(t, isl, _)| *t == term && island.is_some() && *isl == island) {
            Some((_, _, v)) => v.push(p.clone()),
            None => out.push((term, island, vec![p.clone()])),
        }
    }
    let mut out: Vec<_> = out.into_iter().map(|(t, _, v)| (t, v)).collect();
    out.sort_by_key(|(t, _)| TERMINALS.iter().position(|x| x == t).unwrap_or(TERMINALS.len()));
    out
}

/// The `cells` drawing of one MOS: fins on a deck with a `fin` layer, else
/// planar; a flavoured model draws its recogniser's markers.
fn draw_mos(pdk: &verify::Pdk, model: &str, pmos: bool, w: i64, l: i64, nf: i64) -> Macro {
    let kind = if pmos { DeviceKind::Pmos } else { DeviceKind::Nmos };
    let nf = u16::try_from(nf.max(1)).unwrap_or(u16::MAX);
    let finger = (w / i64::from(nf)) as i32;
    let ov = pdk.mos_overlay(model);
    let process: &dyn Process = ov.as_ref().map_or(pdk as &dyn Process, |o| o);
    if process.layer("fin").is_some() {
        crate::legacy::adapter::draw::<cells::finfet::FinFet>(kind, finger, l as i32, &[nf], false, process, |_| true)
    } else {
        crate::legacy::adapter::draw::<cells::mosfet::Mosfet>(kind, finger, l as i32, &[nf], false, process, |m| m.nf == nf && m.rows == 1)
    }
}

/// Deck layer → `P`'s layout layer, through the GDS pair.
fn layer_map<P: PhilisPdk>() -> impl Fn(LayerId) -> substrate::error::Result<<P as layout::schema::Schema>::Layer> {
    let table = P::deck().layer_gds();
    move |id: LayerId| {
        let (l, d) = table.get(id.0 as usize).copied().unwrap_or((0, 0));
        P::from_gds(GdsLayer(l, d)).ok_or_else(|| error(format!("{}: deck layer {} (GDS {l}/{d}) has no layout layer", P::DECK, id.0)))
    }
}

fn rect(r: pnr_core::Rect) -> Rect {
    Rect::from_sides(i64::from(r.x), i64::from(r.y), i64::from(r.x + r.w), i64::from(r.y + r.h))
}

fn error(msg: String) -> substrate::error::Error {
    substrate::error::Error::Boxed(Arc::new(std::io::Error::other(msg)))
}

/// GDS units every Philis deck uses: 1 nm database unit.
pub const GDS_UNITS: GdsUnits = GdsUnits::new(1e-3, 1e-9);

/// Map a `P` LayIR library to GDS layers, for `Context::write_layout`. Each
/// port gets a text label on its conductor's label layer
/// ([`verify::Pdk::label_gds`]), at the centre of each of its shapes.
#[must_use]
pub fn to_gds<P: PhilisPdk>(lib: &layir::Library<<P as layout::schema::Schema>::Layer>) -> (layir::Library<GdsLayer>, GdsUnits) {
    let deck = P::deck();
    let table = deck.layer_gds();
    let label = |g: GdsLayer| {
        table.iter().position(|&pair| pair == (g.0, g.1)).and_then(|id| deck.label_gds(id as u16)).map_or(g, |(l, d)| GdsLayer(l, d))
    };
    let mut out = LibraryBuilder::<GdsLayer>::new();
    for id in lib.topological_order() {
        let cell = lib.cell(id);
        let mut ocell = Cell::new(cell.name());
        for elt in cell.elements() {
            ocell.add_element(elt.map_layer(P::gds));
        }
        for (_, inst) in cell.instances() {
            let child = out.cell_id_named(lib.cell(inst.child()).name());
            ocell.add_instance(Instance::with_transformation(child, inst.name(), inst.transformation()));
        }
        for (name, port) in cell.ports() {
            let mut oport = port.map_layer(P::gds);
            for s in port.elements().filter_map(|e| match e {
                Element::Shape(s) => Some(s),
                Element::Text(_) => None,
            }) {
                let c = s.bbox_rect().center();
                oport.add_element(Element::Text(Text::with_transformation(label(P::gds(s.layer())), name.clone(), Transformation::translate(c.x, c.y))));
            }
            ocell.add_port(name, oport);
        }
        out.add_cell(ocell);
    }
    (out.build().expect("a library built from a valid library"), GDS_UNITS)
}

/// One generated PDK module (invoked by `build.rs` output).
macro_rules! define_pdk {
    ($module:ident, $ty:ident, $deck:literal,
     [$(($layer:ident, $lname:literal, $gl:literal, $gd:literal)),* $(,)?],
     [$(($model:ident, $mname:literal, $pmos:literal)),* $(,)?]) => {
        #[doc = concat!("The `", $deck, "` deck as a Substrate PDK, generated from the deck.")]
        pub mod $module {
            use super::*;

            /// Drawn layers: one per deck layer with a GDS pair.
            #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
            pub enum Layer {
                $(#[doc = concat!("`", $lname, "`, GDS ", stringify!($gl), "/", stringify!($gd), ".")] $layer,)*
            }

            impl Layer {
                /// Every layer, in deck order.
                pub const ALL: &'static [Layer] = &[$(Layer::$layer),*];

                /// The deck's name for this layer.
                #[must_use]
                pub fn name(&self) -> &'static str {
                    match self { $(Layer::$layer => $lname,)* }
                }

                /// The GDS drawing pair.
                #[must_use]
                pub fn gds(&self) -> GdsLayer {
                    match self { $(Layer::$layer => GdsLayer($gl, $gd),)* }
                }

                /// The layer drawn on `gds`.
                #[must_use]
                pub fn from_gds(gds: GdsLayer) -> Option<Self> {
                    match (gds.0, gds.1) { $(($gl, $gd) => Some(Layer::$layer),)* _ => None }
                }
            }

            /// The deck's MOS models.
            #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
            pub enum Model {
                $(#[doc = concat!("`", $mname, "`.")] $model,)*
            }

            impl MosModel for Model {
                type Pdk = $ty;
                fn name(self) -> &'static str {
                    match self { $(Model::$model => $mname,)* }
                }
                fn is_pmos(self) -> bool {
                    match self { $(Model::$model => $pmos,)* }
                }
            }

            #[doc = concat!("The `", $deck, "` PDK.")]
            #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
            pub struct $ty;

            impl layout::schema::Schema for $ty {
                type Layer = Layer;
            }

            impl scir::schema::Schema for $ty {
                type Primitive = Primitive<Model>;
            }

            impl scir::schema::FromSchema<$ty> for Spice {
                type Error = Infallible;
                fn convert_primitive(p: Primitive<Model>) -> Result<spice::Primitive, Infallible> {
                    Ok(p.to_spice())
                }
                fn convert_instance(_: &mut scir::Instance, _: &Primitive<Model>) -> Result<(), Infallible> {
                    Ok(())
                }
            }

            impl DrawnPdk for $ty {}

            impl PhilisPdk for $ty {
                const DECK: &'static str = $deck;
                type Model = Model;

                fn deck() -> &'static verify::Pdk {
                    static DECK: OnceLock<verify::Pdk> = OnceLock::new();
                    DECK.get_or_init(|| verify::Pdk::builtin($deck).expect("a built-in deck"))
                }
                fn gds(layer: &Layer) -> GdsLayer {
                    layer.gds()
                }
                fn from_gds(gds: GdsLayer) -> Option<Layer> {
                    Layer::from_gds(gds)
                }
                fn mos(model: Model, w: i64, l: i64, nf: i64) -> Primitive<Model> {
                    Primitive::Mos { model, w, l, nf }
                }
                fn export_spice<B: Schematic<Schema = Self>>(ctx: &Context, block: B) -> substrate::error::Result<scir::Library<Spice>> {
                    Ok(ctx.export_scir(ConvertSchema::<_, Spice>::new(block))?.scir)
                }
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/pdks.rs"));

// ===========================================================================
//  sky130: Substrate2's own PDK
// ===========================================================================

impl PhilisPdk for sky130::Sky130 {
    const DECK: &'static str = "sky130";
    type Model = sky130::mos::MosKind;

    fn deck() -> &'static verify::Pdk {
        static DECK: OnceLock<verify::Pdk> = OnceLock::new();
        DECK.get_or_init(|| verify::Pdk::builtin("sky130").expect("a built-in deck"))
    }
    fn gds(layer: &sky130::layers::Sky130Layer) -> GdsLayer {
        layer.gds_layer()
    }
    fn from_gds(gds: GdsLayer) -> Option<sky130::layers::Sky130Layer> {
        <sky130::layers::Sky130Layer as gdsconv::conv::FromGds>::from_gds(gds)
    }
    fn mos(model: sky130::mos::MosKind, w: i64, l: i64, nf: i64) -> sky130::Primitive {
        sky130::Primitive::Mos { kind: model, params: sky130::mos::MosParams { w, l, nf } }
    }
    fn export_spice<B: Schematic<Schema = Self>>(ctx: &Context, block: B) -> substrate::error::Result<scir::Library<Spice>> {
        Ok(ctx.export_scir(ConvertSchema::<_, Spice>::new(ConvertSchema::<_, sky130::Sky130OpenSchema>::new(block)))?.scir)
    }
}

impl MosModel for sky130::mos::MosKind {
    type Pdk = sky130::Sky130;

    fn name(self) -> &'static str {
        use sky130::mos::MosKind as K;
        match self {
            K::Nfet01v8 => "nfet_01v8",
            K::Nfet01v8Lvt => "nfet_01v8_lvt",
            K::Nfet03v3Nvt => "nfet_03v3_nvt",
            K::Nfet05v0Nvt => "nfet_05v0_nvt",
            K::Nfet20v0 => "nfet_20v0",
            K::SpecialNfetLatch => "special_nfet_latch",
            K::SpecialNfetPass => "special_nfet_pass",
            K::SpecialPfetPass => "special_pfet_pass",
            K::Pfet01v8 => "pfet_01v8",
            K::Pfet01v8Hvt => "pfet_01v8_hvt",
            K::Pfet01v8Lvt => "pfet_01v8_lvt",
            K::Pfet20v0 => "pfet_20v0",
        }
    }

    fn is_pmos(self) -> bool {
        self.name().contains("pfet")
    }
}
