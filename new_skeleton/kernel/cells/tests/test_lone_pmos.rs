use cells::{Cell, mosfet::Mosfet};
use pnr_core::{DeviceGroup, DeviceId, DeviceKind};
use analog::cell::Unitization;

fn pdk() -> Option<verify::Pdk> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let json = std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?;
    verify::Pdk::from_json(&json).ok()
}

fn draw_one_pmos(pdk: &verify::Pdk) -> pnr_core::Macro {
    let group = DeviceGroup { devices: vec![DeviceId(0)] };
    
    // Create an explicit PMOS unitization
    let unitization = Unitization {
        devices: vec![DeviceId(0)],
        device_type: DeviceKind::Pmos,
        dev_nf: vec![1],
        target_ratio: vec![1],
        unit_w: 420,
        unit_l: 150,
        series_parallel: analog::cell::SeriesParallel::Parallel,
        same_variant_required: false,
        dummy_required: false,
        route_matching_required: false,
    };
    
    let mut constraints = analog::Constraints::default();
    constraints.unitization.push(unitization);
    
    let variants = Mosfet::enumerate(&group, &constraints, pdk);
    let v = variants.first().expect("generator must offer at least one variant");
    v.draw(&group, &constraints, pdk)
}

fn on_layer<'a>(m: &'a pnr_core::Macro, pdk: &verify::Pdk, name: &str) -> Vec<&'a pnr_core::Shape> {
    let Some(target) = pdk.gv_layer_by_name(name) else { return Vec::new() };
    m.shapes.iter().filter(|s| pdk.gv_layer(s.layer) == target).collect()
}

#[test]
fn a_lone_pmos_is_drc_clean() {
    let Some(pdk) = pdk() else {
        eprintln!("sky130 PDK unavailable — skipping");
        return;
    };
    let m = draw_one_pmos(&pdk);
    let findings = verify::drc(&m.shapes, &[], &pdk);
    let mut by_rule: std::collections::BTreeMap<String, usize> = Default::default();
    for f in &findings {
        *by_rule.entry(format!("{}:{}", f.rule, f.layer)).or_default() += 1;
    }

    println!("PMOS alone DRC violations: {}", findings.len());
    if !by_rule.is_empty() {
        println!("By rule: {:?}", by_rule);
    }
    
    let nwells = on_layer(&m, &pdk, "nwell");
    println!("NWELL shapes: {}", nwells.len());
    for nw in &nwells {
        println!("  w={} h={}", nw.rect.w, nw.rect.h);
    }
    
    assert!(
        findings.is_empty(),
        "a single PMOS must be DRC-clean; got {} violations: {:?}",
        findings.len(),
        by_rule
    );
}
