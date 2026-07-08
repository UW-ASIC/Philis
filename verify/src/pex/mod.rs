//! PEX — analytical (pattern-based, 2.5D) parasitic extraction. No field solver.
//!
//! Models (per the research):
//!   * Resistance:   R = Rs * L / W     (sheet resistance times number of squares)
//!   * Area cap:     C_area = Ca * A     (parallel-plate to substrate, A in um^2)
//!   * Fringe cap:   C_fringe = Cf * P   (edge/fringe to ground, P = perimeter in um)
//!   * Coupling cap: C_c = Ck * Lp * (Sref / S)   (lateral, parallel-run-length model,
//!                   scaled inversely with spacing relative to the reference spacing)
//!
//! Each analytical formula maps to a geometric relationship:
//!   - resistance      -> a single wire polygon's L/W
//!   - area+fringe     -> a plate's area & perimeter over the ground plane
//!   - coupling        -> two same-layer polygons facing each other (parallel run length)

use crate::geometry::*;
use crate::params::{Deck, LayerTable, PexLayerParams};
use crate::traits::{Backend, VerifyCheck};

#[derive(Debug, Clone)]
pub enum Parasitic {
    Resistance { layer: String, ohm: f64, length_nm: i32, width_nm: i32 },
    ViaResistance { layer: String, ohm: f64 },
    AreaCap { layer: String, af: f64, area_um2: f64, perimeter_um: f64 },
    CouplingCap { layer: String, af: f64, spacing_nm: i32, run_length_um: f64 },
    InterlayerCap { layer_a: String, layer_b: String, af: f64, overlap_area_um2: f64 },
}

pub struct PexReport {
    pub parasitics: Vec<Parasitic>,
}

impl PexReport {
    pub fn total_resistance(&self, layer: &str) -> f64 {
        self.parasitics.iter().filter_map(|p| match p {
            Parasitic::Resistance { layer: l, ohm, .. }
            | Parasitic::ViaResistance { layer: l, ohm, .. } if l == layer => Some(*ohm),
            _ => None,
        }).sum()
    }
    pub fn total_cap(&self) -> f64 {
        self.parasitics.iter().map(|p| match p {
            Parasitic::AreaCap { af, .. }
            | Parasitic::CouplingCap { af, .. }
            | Parasitic::InterlayerCap { af, .. } => *af,
            Parasitic::Resistance { .. } | Parasitic::ViaResistance { .. } => 0.0,
        }).sum()
    }
    pub fn resistances(&self) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::Resistance { .. })).collect()
    }
    pub fn area_caps(&self) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::AreaCap { .. })).collect()
    }
    pub fn coupling_caps(&self) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::CouplingCap { .. })).collect()
    }
    pub fn via_resistances(&self) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::ViaResistance { .. })).collect()
    }
    pub fn interlayer_caps(&self) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::InterlayerCap { .. })).collect()
    }

    /// Fringe-to-total-plate-cap ratio for characterization. Returns 0.0 if no plate caps.
    pub fn fringe_ratio(&self) -> f64 {
        let mut area_total = 0.0_f64;
        let mut fringe_total = 0.0_f64;
        for p in &self.parasitics {
            if let Parasitic::AreaCap { area_um2, perimeter_um, .. } = p {
                // We don't store the per-component breakdown, but we can re-derive:
                // the AreaCap af already includes both. We need the original params to split.
                // Instead, use the geometry: area_cap ~ area, fringe_cap ~ perimeter.
                // But we don't have params here. Use area_um2 as proxy for area component,
                // perimeter_um as proxy for fringe component (they're proportional).
                area_total += *area_um2;
                fringe_total += *perimeter_um;
            }
        }
        let denom = area_total + fringe_total;
        if denom == 0.0 { 0.0 } else { fringe_total / denom }
    }

    /// Count the number of coupling cap pairs (for bus/interdigitated analysis).
    pub fn bus_coupling_pairs(&self) -> usize {
        self.parasitics.iter().filter(|p| matches!(p, Parasitic::CouplingCap { .. })).count()
    }

    /// Sum coupling-cap contributions from floating (unconnected) polygons.
    /// Since `PexReport` does not carry polygon attribution, this delegates to
    /// `run_pex_by_net` results: pass the by-net map and it returns the cap_af
    /// accrued to the sentinel net `u32::MAX` (polygons not on any device net).
    pub fn floating_metal_cap(&self, by_net: &std::collections::HashMap<u32, NetParasitics>) -> f64 {
        by_net.get(&u32::MAX).map_or(0.0, |np| np.cap_af)
    }

    /// Filter for resistance parasitics where length/width > threshold (high aspect ratio).
    pub fn high_ar_polygons(&self, threshold: f64) -> Vec<&Parasitic> {
        self.parasitics.iter().filter(|p| {
            if let Parasitic::Resistance { length_nm, width_nm, .. } = p {
                *width_nm > 0 && (*length_nm as f64 / *width_nm as f64) > threshold
            } else {
                false
            }
        }).collect()
    }
}

// --- Extractor structs implementing VerifyCheck ----------------------------

pub struct ResistanceExtractor { pub layer: LayerId, pub params: PexLayerParams }
pub struct ViaResistanceExtractor { pub layer: LayerId, pub params: PexLayerParams }
pub struct AreaFringeCapExtractor { pub layer: LayerId, pub params: PexLayerParams }
pub struct CouplingCapExtractor { pub layer: LayerId, pub params: PexLayerParams }

impl VerifyCheck for ResistanceExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str { "resistance" }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_resistance(store, &deck.layers, self.layer, &self.params, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for ViaResistanceExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str { "via_resistance" }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_via_resistance(store, &deck.layers, self.layer, &self.params, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for AreaFringeCapExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str { "area_fringe_cap" }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_area_fringe_cap(store, &deck.layers, self.layer, &self.params, deck, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for CouplingCapExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str { "coupling_cap" }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_coupling_cap(store, &deck.layers, self.layer, &self.params, backend, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

/// Build PEX extractors from the deck's per-layer params.
pub fn pex_rules_from_deck(deck: &Deck) -> Vec<Box<dyn VerifyCheck<Output = Vec<Parasitic>>>> {
    let mut rules: Vec<Box<dyn VerifyCheck<Output = Vec<Parasitic>>>> = Vec::new();
    for (&lid, params) in &deck.pex {
        rules.push(Box::new(ResistanceExtractor { layer: lid, params: params.clone() }));
        rules.push(Box::new(ViaResistanceExtractor { layer: lid, params: params.clone() }));
        rules.push(Box::new(AreaFringeCapExtractor { layer: lid, params: params.clone() }));
        rules.push(Box::new(CouplingCapExtractor { layer: lid, params: params.clone() }));
    }
    rules
}

const NM_PER_UM: f64 = 1000.0;

/// A parasitic together with the polygon(s) it came from: `[poly, u32::MAX]` for the
/// single-polygon extractors (R, area/fringe C), `[poly_a, poly_b]` for coupling C.
type Attributed = (Parasitic, [u32; 2]);

fn extract_all(store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Attributed> {
    let mut out = Vec::new();
    for (&lid, params) in &deck.pex {
        extract_resistance(store, &deck.layers, lid, params, &mut out);
        extract_via_resistance(store, &deck.layers, lid, params, &mut out);
        extract_area_fringe_cap(store, &deck.layers, lid, params, deck, &mut out);
        extract_coupling_cap(store, &deck.layers, lid, params, backend, &mut out);
    }
    extract_interlayer_cap(store, deck, &mut out);
    out
}

pub fn run_pex(store: &GeometryStore, deck: &Deck) -> PexReport {
    let parasitics = extract_all(store, deck, Backend::Cpu).into_iter().map(|(p, _)| p).collect();
    PexReport { parasitics }
}

/// Aggregate parasitics of one extracted net (see [`run_pex_by_net`]).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NetParasitics {
    pub r_ohm: f64,
    pub cap_af: f64,
}

/// Per-net PEX: same extractors as [`run_pex`], but each parasitic is attributed to the
/// extracted net(s) of its source polygon(s) via `net_of_poly` (as produced by
/// `ExtractedNetlist::net_of_poly`; entries of `u32::MAX` are skipped).
///
/// * Resistance and area/fringe cap accrue to the polygon's net.
/// * Coupling cap accrues in FULL to BOTH nets (conservative for budget checks); pairs on
///   the same net are skipped — same-net coupling is not a parasitic to budget.
pub fn run_pex_by_net(
    store: &GeometryStore, deck: &Deck, net_of_poly: &[u32],
) -> std::collections::HashMap<u32, NetParasitics> {
    let net = |p: u32| -> u32 { net_of_poly.get(p as usize).copied().unwrap_or(u32::MAX) };
    let mut out: std::collections::HashMap<u32, NetParasitics> = std::collections::HashMap::new();
    for (par, polys) in extract_all(store, deck, Backend::Cpu) {
        match par {
            Parasitic::Resistance { ohm, .. } | Parasitic::ViaResistance { ohm, .. } => {
                let n = net(polys[0]);
                if n != u32::MAX { out.entry(n).or_default().r_ohm += ohm; }
            }
            Parasitic::AreaCap { af, .. } => {
                let n = net(polys[0]);
                if n != u32::MAX { out.entry(n).or_default().cap_af += af; }
            }
            Parasitic::CouplingCap { af, .. } | Parasitic::InterlayerCap { af, .. } => {
                let (na, nb) = (net(polys[0]), net(polys[1]));
                if na == nb { continue; } // same-net coupling: not a parasitic to budget
                if na != u32::MAX { out.entry(na).or_default().cap_af += af; }
                if nb != u32::MAX { out.entry(nb).or_default().cap_af += af; }
            }
        }
    }
    out
}

/// Resistance of each wire polygon: squares = L/W (long dimension over short), R = Rs*squares.
fn extract_resistance(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, p: &PexLayerParams,
    out: &mut Vec<Attributed>,
) {
    for poly in store.polys_on_layer(layer) {
        let bb = store.poly_bbox[poly.0 as usize];
        let w = bb.width();
        let h = bb.height();
        if w == 0 || h == 0 { continue; }
        let (length, width) = if w >= h { (w, h) } else { (h, w) };
        // Only treat clearly wire-like shapes (aspect ratio >= 2) as resistors; skip square
        // plates (those are extracted as capacitors, not resistors, in this simple model).
        if (length as f64) < 2.0 * (width as f64) { continue; }
        let squares = length as f64 / width as f64;
        let ohm = p.sheet_res_ohm_sq * squares;
        out.push((Parasitic::Resistance {
            layer: lt.name(layer).into(), ohm, length_nm: length, width_nm: width,
        }, [poly.0, u32::MAX]));
    }
}

/// Fixed per-via resistance for each polygon on a via/contact layer.
fn extract_via_resistance(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, p: &PexLayerParams,
    out: &mut Vec<Attributed>,
) {
    if p.via_res_ohm == 0.0 { return; }
    for poly in store.polys_on_layer(layer) {
        out.push((Parasitic::ViaResistance {
            layer: lt.name(layer).into(), ohm: p.via_res_ohm,
        }, [poly.0, u32::MAX]));
    }
}

/// Area + fringe capacitance to substrate for each plate-like polygon.
///
/// Feature 4 (Dummy-Fill Impact): polygons with area < 0.01 um² (10000 nm²) and aspect
/// ratio near 1 are treated as fill squares — area_cap contribution is halved (0.5x).
///
/// Feature 6 (Ground-Plane Shielding): if a large polygon (> 1 um²) on a lower metal
/// layer overlaps this polygon in x-y, area_cap is reduced by 0.3x (shielding).
/// Simplified: met1 is below met2 — only met2 wires with a met1 ground plane are shielded.
fn extract_area_fringe_cap(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, p: &PexLayerParams,
    _deck: &Deck, out: &mut Vec<Attributed>,
) {
    let layer_name = lt.name(layer);

    // Feature 6: precompute lower-layer large polygons for shielding check.
    // Convention: "met2" is shielded by "met1" (met1 is below met2).
    let shield_layer = if layer_name == "met2" { lt.id("met1") } else { None };
    let shield_polys: Vec<Bbox> = shield_layer
        .map(|sl| {
            store.polys_on_layer(sl).iter()
                .map(|q| store.poly_bbox[q.0 as usize])
                .filter(|bb| {
                    let area_um2 = (bb.width() as f64 / NM_PER_UM) * (bb.height() as f64 / NM_PER_UM);
                    area_um2 > 1.0
                })
                .collect()
        })
        .unwrap_or_default();

    for poly in store.polys_on_layer(layer) {
        let bb = store.poly_bbox[poly.0 as usize];
        let w = bb.width();
        let h = bb.height();
        if w == 0 || h == 0 { continue; }
        // plate-like: aspect ratio < 2 (square-ish). Wires are handled as R; here we want the
        // conformance PEX_PLATE_C (a square) to get the area+fringe model.
        let (long, short) = if w >= h { (w, h) } else { (h, w) };
        if (long as f64) >= 2.0 * (short as f64) { continue; }
        let area_um2 = (w as f64 / NM_PER_UM) * (h as f64 / NM_PER_UM);
        let perim_um = 2.0 * (w as f64 + h as f64) / NM_PER_UM;

        // Feature 4: dummy-fill detection — small area + near-square aspect ratio.
        let area_nm2 = (w as i64) * (h as i64);
        let ar = long as f64 / short as f64;
        let fill_factor = if area_nm2 < 10_000 && ar < 1.5 { 0.5 } else { 1.0 };

        // Feature 6: ground-plane shielding — large lower-layer polygon overlaps this one.
        let shielded = shield_polys.iter().any(|sp| bb.overlaps(sp));
        let shield_factor = if shielded { 0.3 } else { 1.0 };

        let area_cap = p.area_cap_af_um2 * area_um2 * fill_factor * shield_factor;
        let fringe_cap = p.fringe_cap_af_um * perim_um;
        let af = area_cap + fringe_cap;
        out.push((Parasitic::AreaCap {
            layer: layer_name.into(), af, area_um2, perimeter_um: perim_um,
        }, [poly.0, u32::MAX]));
    }
}

/// Inter-layer coupling capacitance between wires on DIFFERENT metal layers that cross
/// over/under each other. C = interlayer_cap_af_um2 * overlap_area_um2 for each polygon
/// pair that overlaps in the x-y plane.
fn extract_interlayer_cap(
    store: &GeometryStore, deck: &Deck, out: &mut Vec<Attributed>,
) {
    let pex_layers: Vec<(LayerId, &PexLayerParams)> = deck.pex.iter()
        .map(|(&lid, p)| (lid, p))
        .collect();
    let n = pex_layers.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let (lid_a, params_a) = pex_layers[i];
            let (lid_b, params_b) = pex_layers[j];
            // Use the average of both layers' interlayer_cap coefficients (or whichever is nonzero).
            let coeff = if params_a.interlayer_cap_af_um2 != 0.0 && params_b.interlayer_cap_af_um2 != 0.0 {
                (params_a.interlayer_cap_af_um2 + params_b.interlayer_cap_af_um2) / 2.0
            } else {
                params_a.interlayer_cap_af_um2 + params_b.interlayer_cap_af_um2  // one is 0.0
            };
            if coeff == 0.0 { continue; }
            let polys_a = store.polys_on_layer(lid_a);
            let polys_b = store.polys_on_layer(lid_b);
            for &pa in &polys_a {
                let ba = store.poly_bbox[pa.0 as usize];
                for &pb in &polys_b {
                    let bb = store.poly_bbox[pb.0 as usize];
                    // Compute x-y overlap area (bbox intersection).
                    let ox = (ba.xmax.min(bb.xmax) - ba.xmin.max(bb.xmin)).max(0);
                    let oy = (ba.ymax.min(bb.ymax) - ba.ymin.max(bb.ymin)).max(0);
                    if ox <= 0 || oy <= 0 { continue; }
                    let overlap_area_um2 = (ox as f64 / NM_PER_UM) * (oy as f64 / NM_PER_UM);
                    let af = coeff * overlap_area_um2;
                    out.push((Parasitic::InterlayerCap {
                        layer_a: deck.layers.name(lid_a).into(),
                        layer_b: deck.layers.name(lid_b).into(),
                        af,
                        overlap_area_um2,
                    }, [pa.0, pb.0]));
                }
            }
        }
    }
}

/// Lateral coupling capacitance between parallel same-layer wires that face each other.
fn extract_coupling_cap(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, p: &PexLayerParams,
    backend: Backend, out: &mut Vec<Attributed>,
) {
    let polys = store.polys_on_layer(layer);
    let n = polys.len();
    let n_pairs = n * n.saturating_sub(1) / 2;

    // GPU path: compute run length + gap for all pairs in parallel
    if backend == Backend::Gpu && n_pairs >= (1 << 18) {
        let xmins: Vec<f32> = polys.iter().map(|q| store.poly_bbox[q.0 as usize].xmin as f32).collect();
        let ymins: Vec<f32> = polys.iter().map(|q| store.poly_bbox[q.0 as usize].ymin as f32).collect();
        let xmaxs: Vec<f32> = polys.iter().map(|q| store.poly_bbox[q.0 as usize].xmax as f32).collect();
        let ymaxs: Vec<f32> = polys.iter().map(|q| store.poly_bbox[q.0 as usize].ymax as f32).collect();
        let mut pa = Vec::with_capacity(n_pairs);
        let mut pb = Vec::with_capacity(n_pairs);
        for i in 0..n { for j in (i + 1)..n { pa.push(i as u32); pb.push(j as u32); } }
        if let Some((runs, gaps)) = crate::traits::coupling_scan_gpu(&xmins, &ymins, &xmaxs, &ymaxs, &pa, &pb) {
            let mut idx = 0;
            for i in 0..n {
                for j in (i + 1)..n {
                    let run_nm = runs[idx];
                    let gap_nm = gaps[idx];
                    idx += 1;
                    if run_nm > 0.0 && gap_nm > 0.0 {
                        let run_um = run_nm as f64 / NM_PER_UM;
                        let spacing = gap_nm as i32;
                        let scale = p.coupling_ref_spacing_nm / spacing as f64;
                        let af = p.coupling_cap_af_um * run_um * scale;
                        out.push((Parasitic::CouplingCap {
                            layer: lt.name(layer).into(), af, spacing_nm: spacing, run_length_um: run_um,
                        }, [polys[i].0, polys[j].0]));
                    }
                }
            }
            return;
        }
    }

    // CPU fallback
    for i in 0..n {
        for j in (i + 1)..n {
            let a = store.poly_bbox[polys[i].0 as usize];
            let b = store.poly_bbox[polys[j].0 as usize];
            let x_overlap = a.xmax.min(b.xmax) - a.xmin.max(b.xmin);
            let y_gap = if a.ymax <= b.ymin { b.ymin - a.ymax }
                        else if b.ymax <= a.ymin { a.ymin - b.ymax }
                        else { -1 };
            if x_overlap > 0 && y_gap > 0 {
                let run_um = x_overlap as f64 / NM_PER_UM;
                let spacing = y_gap;
                let scale = p.coupling_ref_spacing_nm / spacing as f64;
                let af = p.coupling_cap_af_um * run_um * scale;
                out.push((Parasitic::CouplingCap {
                    layer: lt.name(layer).into(), af, spacing_nm: spacing, run_length_um: run_um,
                }, [polys[i].0, polys[j].0]));
                continue;
            }
            let y_overlap = a.ymax.min(b.ymax) - a.ymin.max(b.ymin);
            let x_gap = if a.xmax <= b.xmin { b.xmin - a.xmax }
                        else if b.xmax <= a.xmin { a.xmin - b.xmax }
                        else { -1 };
            if y_overlap > 0 && x_gap > 0 {
                let run_um = y_overlap as f64 / NM_PER_UM;
                let spacing = x_gap;
                let scale = p.coupling_ref_spacing_nm / spacing as f64;
                let af = p.coupling_cap_af_um * run_um * scale;
                out.push((Parasitic::CouplingCap {
                    layer: lt.name(layer).into(), af, spacing_nm: spacing, run_length_um: run_um,
                }, [polys[i].0, polys[j].0]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_deck() -> Deck {
        Deck::from_json(r#"{
            "layers": { "met1": { "layer": 68, "datatype": 20 } },
            "drc": {},
            "pex": { "met1": {
                "sheet_res_ohm_sq": 0.1,
                "area_cap_af_um2": 25.0,
                "fringe_cap_af_um": 40.0,
                "coupling_cap_af_um": 100.0,
                "coupling_ref_spacing_nm": 200
            } }
        }"#).unwrap()
    }

    #[test]
    fn per_net_attribution() {
        let deck = test_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let mut st = GeometryStore::new();
        st.add_rect(met1, 0, 0, 2000, 200);   // wire A: 10 squares -> 1.0 ohm
        st.add_rect(met1, 0, 400, 2000, 200); // wire B: parallel, 200nm gap

        // distinct nets: R per net, full coupling to both
        let by_net = run_pex_by_net(&st, &deck, &[0, 1]);
        let a = by_net[&0];
        let b = by_net[&1];
        assert!((a.r_ohm - 1.0).abs() < 1e-9);
        assert!((b.r_ohm - 1.0).abs() < 1e-9);
        // coupling: 100 aF/um * 2.0 um * (200/200) = 200 aF, added to BOTH nets
        assert!((a.cap_af - 200.0).abs() < 1e-9);
        assert!((b.cap_af - 200.0).abs() < 1e-9);

        // same net: coupling skipped, resistances accumulate
        let by_net = run_pex_by_net(&st, &deck, &[0, 0]);
        let a = by_net[&0];
        assert!((a.r_ohm - 2.0).abs() < 1e-9);
        assert!(a.cap_af.abs() < 1e-9);

        // totals through run_pex are unchanged by the attribution refactor
        let report = run_pex(&st, &deck);
        assert!((report.total_resistance("met1") - 2.0).abs() < 1e-9);
    }
}
