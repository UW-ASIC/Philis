//! plan-09 P0: a Substrate2 block signs off through `macro_master::signoff`.
//!
//! The inverter is Substrate2's `examples/latest/sky130_inverter` (schematic
//! and layout), unchanged but for imports.
//! Copyright (c) 2023, Substrate Labs. BSD 3-Clause License.

use macro_master::block::Block;
use macro_master::context::Context;
use macro_master::error::Result;
use macro_master::geometry::align::{AlignMode, AlignRectMut};
use macro_master::geometry::bbox::Bbox;
use macro_master::geometry::prelude::Transformation;
use macro_master::geometry::rect::Rect;
use macro_master::geometry::span::Span;
use macro_master::geometry::transform::TransformMut;
use macro_master::geometry::union::BoundingUnion;
use macro_master::layir::Shape;
use macro_master::layout::{CellBundle, Layout};
use macro_master::schematic::{CellBuilder, ConvertSchema, Schematic};
use macro_master::spice::Spice;
use macro_master::types::codegen::{PortGeometryBundle, View};
use macro_master::types::layout::PortGeometry;
use macro_master::types::schematic::IoNodeBundle;
use macro_master::types::{InOut, Input, Io, Output, Signal};
use sky130::layers::Sky130Layer;
use sky130::layout::{NtapTile, PtapTile};
use sky130::mos::{GateDir, MosLength, Nfet01v8, NmosTile, Pfet01v8, PmosTile};
use sky130::{Sky130, Sky130OpenSchema};

#[derive(Io, Clone, Default, Debug)]
pub struct InverterIo {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub din: Input<Signal>,
    pub dout: Output<Signal>,
}

#[derive(Block, Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[substrate(io = "InverterIo")]
pub struct Inverter {
    /// NMOS width.
    pub nw: i64,
    /// PMOS width.
    pub pw: i64,
}

impl Schematic for Inverter {
    type Schema = Sky130;
    type NestedData = ();
    fn schematic(&self, io: &IoNodeBundle<Self>, cell: &mut CellBuilder<<Self as Schematic>::Schema>) -> Result<Self::NestedData> {
        let nmos = cell.instantiate(Nfet01v8::new((self.nw, 150)));
        cell.connect(io.dout, nmos.io().d);
        cell.connect(io.din, nmos.io().g);
        cell.connect(io.vss, nmos.io().s);
        cell.connect(io.vss, nmos.io().b);

        let pmos = cell.instantiate(Pfet01v8::new((self.pw, 150)));
        cell.connect(io.dout, pmos.io().d);
        cell.connect(io.din, pmos.io().g);
        cell.connect(io.vdd, pmos.io().s);
        cell.connect(io.vdd, pmos.io().b);

        Ok(())
    }
}

impl Layout for Inverter {
    type Schema = Sky130;
    type Bundle = View<InverterIo, PortGeometryBundle<Sky130>>;
    type Data = ();
    fn layout(&self, cell: &mut macro_master::layout::CellBuilder<Self::Schema>) -> Result<(Self::Bundle, Self::Data)> {
        let mut nmos = cell.generate(NmosTile::new(self.nw, MosLength::L150, 1).with_gate_dir(GateDir::Left));
        let mut pmos = cell.generate(PmosTile::new(self.pw, MosLength::L150, 1).with_gate_dir(GateDir::Left));

        nmos.transform_mut(Transformation::reflect_vert());
        pmos.align_mut(AlignMode::Above, pmos.bbox_rect(), nmos.bbox_rect(), 600);

        let dout = Shape::new(Sky130Layer::Li1, nmos.io().sd[1].primary.bounding_union(&pmos.io().sd[1].primary));
        cell.draw(dout.clone())?;

        let din = Shape::new(Sky130Layer::Li1, nmos.io().g[0].primary.bounding_union(&pmos.io().g[0].primary));
        cell.draw(din.clone())?;

        let mut ntap = cell.generate(NtapTile::new(2, 2));
        ntap.align_mut(AlignMode::Above, ntap.bbox_rect(), pmos.bbox_rect(), 0);
        ntap.align_mut(AlignMode::CenterHorizontal, ntap.bbox_rect(), pmos.bbox_rect(), 0);

        let mut ptap = cell.generate(PtapTile::new(2, 2));
        ptap.align_mut(AlignMode::Beneath, ptap.bbox_rect(), nmos.bbox_rect(), -20);
        ptap.align_mut(AlignMode::CenterHorizontal, ptap.bbox_rect(), nmos.bbox_rect(), 0);

        let vdd = ntap.io().vpb.primary.clone();
        let vss = ptap.io().vnb.primary.clone();

        let nmos_s = nmos.io().sd[0].bbox_rect();
        let vss_conn = Rect::from_spans(nmos_s.hspan(), Span::new(vss.bbox_rect().bot(), nmos_s.top()));
        cell.draw(Shape::new(Sky130Layer::Li1, vss_conn))?;

        let pmos_s = pmos.io().sd[0].bbox_rect();
        let vdd_conn = Rect::from_spans(pmos_s.hspan(), Span::new(pmos_s.bot(), vdd.bbox_rect().top()));
        cell.draw(Shape::new(Sky130Layer::Li1, vdd_conn))?;

        cell.draw(ntap)?;
        cell.draw(ptap)?;
        cell.draw(nmos)?;
        cell.draw(pmos)?;

        Ok((
            CellBundle::<Inverter> {
                vdd: PortGeometry::new(vdd),
                vss: PortGeometry::new(vss),
                din: PortGeometry::new(din),
                dout: PortGeometry::new(dout),
            },
            (),
        ))
    }
}

#[test]
fn inverter_signs_off_clean_on_sky130() {
    // Export only: nothing reads the PDK root.
    let ctx = Context::builder().install(Sky130::open("/nonexistent")).build();
    let dut = Inverter { nw: 1_200, pw: 2_400 };

    let r = macro_master::signoff::run(&ctx, dut).expect("signoff runs");
    println!("{}", r.coverage);
    assert!(r.is_clean(), "not clean: {:#?}\n{}", r.failures(), r.coverage);
}

/// The clean pass above is not vacuous: a schematic NMOS narrower than the
/// drawn one is an LVS parameter mismatch.
#[test]
fn resized_schematic_fails_lvs() {
    let ctx = Context::builder().install(Sky130::open("/nonexistent")).build();
    let layout = ctx.export_layir(Inverter { nw: 1_200, pw: 2_400 }).expect("layout");
    let schematic = ctx
        .export_scir(ConvertSchema::<_, Spice>::new(ConvertSchema::<_, Sky130OpenSchema>::new(Inverter { nw: 1_000, pw: 2_400 })))
        .expect("schematic");
    let pdk = verify::Pdk::builtin("sky130").expect("sky130 deck");

    let s = macro_master::signoff::check(&layout.layir, Sky130Layer::gds_layer, &schematic.scir, &pdk).expect("signoff runs");
    let hard: Vec<_> = s.report.hard_violations.iter().map(|v| v.rule.clone()).collect();
    assert!(hard.iter().any(|r| r.starts_with("lvs")), "LVS missed a 1.2 µm vs 1.0 µm NMOS: {hard:?}");
}
