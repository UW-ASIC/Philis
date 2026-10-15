//! plan-09 P2: devices on the typed PDKs.
//!
//! As in Substrate, a device is a schematic primitive ([`Mos`]) and a layout
//! tile ([`MosTile`]); a composing block draws the joins. [`Device`] is the
//! smallest such block, for a tile whose every terminal is one region.

use macro_master::block::Block;
use macro_master::context::Context;
use macro_master::error::Result;
use macro_master::layout::{CellBundle, Layout};
use macro_master::pdk::{asap7, gf180mcu, ihp_sg13g2, DrawnPdk, Mos, MosModel, MosTile, PhilisPdk};
use macro_master::schematic::{CellBuilder, Schematic};
use macro_master::signoff;
use macro_master::types::codegen::{PortGeometryBundle, View};
use macro_master::types::schematic::IoNodeBundle;
use macro_master::types::MosIo;

/// One transistor, schematic and layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Device<M>(Mos<M>);

impl<M: MosModel> std::hash::Hash for Device<M> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.0.hash(h);
    }
}

impl<M: MosModel> Block for Device<M> {
    type Io = MosIo;
    fn name(&self) -> macro_master::arcstr::ArcStr {
        self.0.name()
    }
    fn io(&self) -> MosIo {
        MosIo::default()
    }
}

impl<M: MosModel> Schematic for Device<M> {
    type Schema = M::Pdk;
    type NestedData = ();
    fn schematic(&self, io: &IoNodeBundle<Self>, cell: &mut CellBuilder<M::Pdk>) -> Result<()> {
        let m = cell.instantiate(self.0);
        cell.connect(io.d, m.io().d);
        cell.connect(io.g, m.io().g);
        cell.connect(io.s, m.io().s);
        cell.connect(io.b, m.io().b);
        Ok(())
    }
}

impl<M: MosModel> Layout for Device<M>
where
    M::Pdk: DrawnPdk,
{
    type Schema = M::Pdk;
    type Bundle = View<MosIo, PortGeometryBundle<M::Pdk>>;
    type Data = ();
    fn layout(&self, cell: &mut macro_master::layout::CellBuilder<M::Pdk>) -> Result<(Self::Bundle, ())> {
        let tile = cell.generate(MosTile(self.0));
        let io = tile.io();
        for (t, n) in [("d", io.d.num_elems()), ("g", io.g.num_elems()), ("s", io.s.num_elems()), ("b", io.b.num_elems())] {
            assert_eq!(n, 1, "{}: `{t}` is {n} regions; a composing block must join them", self.0.name());
        }
        let bundle = CellBundle::<Self> { d: io.d[0].clone(), g: io.g[0].clone(), s: io.s[0].clone(), b: io.b[0].clone() };
        cell.draw(tile)?;
        Ok((bundle, ()))
    }
}

/// `model` at the deck's minimum length with `nf` 1680 nm fingers (the
/// width the generators' own clean tests draw).
fn mos<M: MosModel>(model: M, nf: i64) -> Mos<M> {
    let (l, _) = M::Pdk::deck().min_channel(model.is_pmos(), model.name());
    Mos::new(model, 1680 * nf, i64::from(l), nf)
}

/// One-finger: the device signs off clean against its own schematic.
fn signs_off<M: MosModel>(model: M)
where
    M::Pdk: DrawnPdk,
{
    let r = signoff::run(&Context::builder().build(), Device(mos(model, 1))).expect("signoff runs");
    assert!(r.is_clean(), "{} {model:?}: {:#?}\n{}", <M::Pdk as PhilisPdk>::DECK, r.failures(), r.coverage);
}

/// Any finger count: the tile alone is DRC/ERC clean.
fn tile_clean<M: MosModel>(model: M, nf: i64)
where
    M::Pdk: DrawnPdk,
{
    let r = signoff::run_layout(&Context::builder().build(), MosTile(mos(model, nf))).expect("signoff runs");
    assert!(r.layout_clean(), "{} {model:?} nf={nf}: {:#?}", <M::Pdk as PhilisPdk>::DECK, r.failures());
}

#[test]
fn gf180mcu() {
    for m in [gf180mcu::Model::Nfet03v3, gf180mcu::Model::Pfet03v3] {
        signs_off(m);
        tile_clean(m, 2);
    }
}

#[test]
fn ihp_sg13g2() {
    for m in [ihp_sg13g2::Model::Sg13LvNmos, ihp_sg13g2::Model::Sg13LvPmos] {
        signs_off(m);
        tile_clean(m, 2);
    }
}

#[test]
fn asap7() {
    for m in [asap7::Model::NmosRvt, asap7::Model::PmosRvt] {
        signs_off(m);
        tile_clean(m, 2);
    }
}

/// A planar two-finger device leaves its two sources for the composing block
/// to join; the FinFET generator straps them itself.
#[test]
fn tiles_expose_unjoined_regions() {
    let planar = MosTile(mos(gf180mcu::Model::Nfet03v3, 2)).io();
    assert_eq!((planar.d.len(), planar.s.len()), (1, 2), "gf180 nf=2: one drain, two sources");
    let fin = MosTile(mos(asap7::Model::NmosRvt, 2)).io();
    assert_eq!(fin.s.len(), 1, "asap7 nf=2: one strapped source");
}

/// A flavoured model draws its recogniser's markers, so it extracts as
/// itself.
///
/// ponytail: gf180mcu's 6 V models are not drawable clean yet: `min_channel`
/// gives them the 3.3 V length (PL.2_5V_n) and the `dualgate` overlay misses
/// DV.6/7/8, DF.2a. A Philis `cells` fix, then add them here.
#[test]
fn flavoured_models_extract_as_themselves() {
    signs_off(asap7::Model::NmosLvt);
    signs_off(asap7::Model::PmosSlvt);
}

/// ...and that check can fail: an RVT drawing against an LVT schematic does
/// not match.
#[test]
fn a_model_mismatch_fails_lvs() {
    let ctx = Context::builder().build();
    let layout = ctx.export_layir(Device(mos(asap7::Model::NmosRvt, 1))).expect("layout");
    let netlist = <asap7::Asap7 as PhilisPdk>::export_spice(&ctx, Device(mos(asap7::Model::NmosLvt, 1))).expect("schematic");
    let r: signoff::Report = signoff::check(&layout.layir, asap7::Layer::gds, &netlist, <asap7::Asap7 as PhilisPdk>::deck()).expect("runs").into();
    assert!(!r.lvs_matches(), "an RVT layout matched an LVT schematic");
}
