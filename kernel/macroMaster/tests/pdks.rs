//! plan-09 P2: an NMOS and a PMOS `Mos` per PDK, drawn by Philis's
//! generators on the typed PDK, signed off against their own schematic.
//!
//! The planar generator leaves same-terminal pins for the router to join:
//! the two outer sources of a two-finger device, sky130's two body taps.
//! Signed off alone, those split pins read as `label_conflict`, unpaired
//! nets/devices, or `soft_connection` (the taps meet through the substrate).
//! So a split device must be DRC-clean and fail on nothing but those rules;
//! an unsplit one must be clean outright. plan-09 P4 routes them.

use macro_master::context::Context;
use macro_master::pdk::{asap7, gf180mcu, ihp_sg13g2, Mos, MosModel, PhilisPdk};
use macro_master::signoff;

/// What open must-join pins look like to a lone signoff.
const MUST_JOIN: [&str; 4] = ["erc/soft_connection", "lvs/lvs.label_conflict", "lvs/lvs.unpaired_device", "lvs/lvs.unpaired_net"];

/// `model` at the deck's minimum length, 1680 nm fingers (the width the
/// generators' own clean tests draw), `nf` of them. `split`: the drawing
/// has must-join pins.
fn signs_off<M: MosModel>(ctx: &Context, model: M, nf: i64, split: bool) {
    let deck = M::Pdk::deck();
    let (l, _) = deck.min_channel(model.is_pmos(), model.name());
    let w = 1680 * nf;
    let r = signoff::run(ctx, Mos::new(model, w, i64::from(l), nf)).unwrap_or_else(|e| panic!("{}: {e}", <M::Pdk as PhilisPdk>::DECK));
    let at = format!("{} {model:?} w={w} l={l} nf={nf}", <M::Pdk as PhilisPdk>::DECK);
    if split {
        assert!(r.drc.is_empty(), "{at}: DRC {:#?}", r.drc);
        let other: Vec<_> = r.failures().into_iter().filter(|f| !MUST_JOIN.iter().any(|m| f.starts_with(m))).collect();
        assert!(other.is_empty(), "{at}: beyond the open must-join pins: {other:#?}");
    } else {
        assert!(r.is_clean(), "{at}: {:#?}\n{}", r.failures(), r.coverage);
    }
}

#[test]
fn sky130_mos() {
    let ctx = Context::builder().install(sky130::Sky130::open("/nonexistent")).build();
    for nf in [1, 2] {
        // Two body taps at every size.
        signs_off(&ctx, sky130::mos::MosKind::Nfet01v8, nf, true);
        signs_off(&ctx, sky130::mos::MosKind::Pfet01v8, nf, true);
    }
}

#[test]
fn gf180mcu_mos() {
    let ctx = Context::builder().build();
    for nf in [1, 2] {
        signs_off(&ctx, gf180mcu::Model::Nfet03v3, nf, nf > 1);
        signs_off(&ctx, gf180mcu::Model::Pfet03v3, nf, nf > 1);
    }
}

#[test]
fn ihp_sg13g2_mos() {
    let ctx = Context::builder().build();
    for nf in [1, 2] {
        signs_off(&ctx, ihp_sg13g2::Model::Sg13LvNmos, nf, nf > 1);
        signs_off(&ctx, ihp_sg13g2::Model::Sg13LvPmos, nf, nf > 1);
    }
}

#[test]
fn asap7_mos() {
    let ctx = Context::builder().build();
    for nf in [1, 2] {
        signs_off(&ctx, asap7::Model::NmosRvt, nf, false);
        signs_off(&ctx, asap7::Model::PmosRvt, nf, false);
    }
}

/// A flavoured model draws its recogniser's markers, so it extracts as
/// itself. Unsplit devices only: a split one's must-join allowance would
/// absorb a model mismatch (`unpaired_device`).
#[test]
fn flavoured_models_extract_as_themselves() {
    let ctx = Context::builder().build();
    // ponytail: gf180mcu's 6 V models are not drawable clean yet: `min_channel`
    // gives them the 3.3 V length (PL.2_5V_n) and the `dualgate` overlay
    // misses DV.6/7/8, DF.2a. A `cells` fix, then add them here.
    signs_off(&ctx, asap7::Model::NmosLvt, 1, false);
    signs_off(&ctx, asap7::Model::PmosSlvt, 1, false);
}

/// ...and the check above can fail: an RVT drawing against an LVT schematic
/// does not match.
#[test]
fn a_model_mismatch_fails_lvs() {
    let ctx = Context::builder().build();
    let (drawn, wanted) = (Mos::new(asap7::Model::NmosRvt, 1680, 20, 1), Mos::new(asap7::Model::NmosLvt, 1680, 20, 1));
    let layout = ctx.export_layir(drawn).expect("layout");
    let netlist = <asap7::Asap7 as PhilisPdk>::export_spice(&ctx, wanted).expect("schematic");
    let r: signoff::Report = signoff::check(&layout.layir, asap7::Layer::gds, &netlist, <asap7::Asap7 as PhilisPdk>::deck()).expect("runs").into();
    assert!(!r.lvs_matches(), "an RVT layout matched an LVT schematic");
}
