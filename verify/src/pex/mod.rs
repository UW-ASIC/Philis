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
//!   - resistance      -> a single conductor polygon's perimeter/area-equivalent L/W
//!   - area+fringe     -> every conductor polygon's exact area & perimeter
//!   - coupling        -> two same-layer polygons facing each other (parallel run length)

use crate::geometry::*;
use crate::params::{Deck, LayerTable, PexLayerParams};
use crate::traits::{Backend, VerifyCheck};

#[derive(Debug, Clone)]
pub enum Parasitic {
    Resistance {
        layer: String,
        ohm: f64,
        length_nm: i32,
        width_nm: i32,
    },
    ViaResistance {
        layer: String,
        ohm: f64,
    },
    AreaCap {
        layer: String,
        /// Total ground capacitance (`area_af + fringe_af`).
        af: f64,
        /// Parallel-plate contribution, in attofarads.
        area_af: f64,
        /// Edge/fringe contribution, in attofarads.
        fringe_af: f64,
        area_um2: f64,
        perimeter_um: f64,
    },
    CouplingCap {
        layer: String,
        af: f64,
        spacing_nm: i32,
        run_length_um: f64,
    },
    InterlayerCap {
        layer_a: String,
        layer_b: String,
        af: f64,
        overlap_area_um2: f64,
    },
    /// The analytical extractor refused geometry it cannot model faithfully. Diagnostics
    /// live in the result (rather than silently producing zero R/C) so signoff callers can
    /// fail closed using [`PexReport::is_complete`].
    ExtractionDiagnostic {
        layer: String,
        polygon: u32,
        model: String,
        message: String,
    },
}

pub struct PexReport {
    pub parasitics: Vec<Parasitic>,
}

impl PexReport {
    pub fn total_resistance(&self, layer: &str) -> f64 {
        self.parasitics
            .iter()
            .filter_map(|p| match p {
                Parasitic::Resistance { layer: l, ohm, .. }
                | Parasitic::ViaResistance { layer: l, ohm, .. }
                    if l == layer =>
                {
                    Some(*ohm)
                }
                _ => None,
            })
            .sum()
    }
    pub fn total_cap(&self) -> f64 {
        self.parasitics
            .iter()
            .map(|p| match p {
                Parasitic::AreaCap { af, .. }
                | Parasitic::CouplingCap { af, .. }
                | Parasitic::InterlayerCap { af, .. } => *af,
                Parasitic::Resistance { .. }
                | Parasitic::ViaResistance { .. }
                | Parasitic::ExtractionDiagnostic { .. } => 0.0,
            })
            .sum()
    }
    pub fn resistances(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::Resistance { .. }))
            .collect()
    }
    pub fn area_caps(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::AreaCap { .. }))
            .collect()
    }
    pub fn coupling_caps(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::CouplingCap { .. }))
            .collect()
    }
    pub fn via_resistances(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::ViaResistance { .. }))
            .collect()
    }
    pub fn interlayer_caps(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::InterlayerCap { .. }))
            .collect()
    }

    pub fn diagnostics(&self) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::ExtractionDiagnostic { .. }))
            .collect()
    }

    /// True only when all encountered geometry was supported by the analytical model.
    pub fn is_complete(&self) -> bool {
        self.diagnostics().is_empty()
    }

    /// Fringe-to-total ground-cap ratio for characterization. Returns 0.0 if no ground cap.
    /// Both numerator and denominator are capacitances in attofarads.
    pub fn fringe_ratio(&self) -> f64 {
        let mut total = 0.0_f64;
        let mut fringe_total = 0.0_f64;
        for p in &self.parasitics {
            if let Parasitic::AreaCap { af, fringe_af, .. } = p {
                total += *af;
                fringe_total += *fringe_af;
            }
        }
        if total == 0.0 {
            0.0
        } else {
            fringe_total / total
        }
    }

    /// Count the number of coupling cap pairs (for bus/interdigitated analysis).
    pub fn bus_coupling_pairs(&self) -> usize {
        self.parasitics
            .iter()
            .filter(|p| matches!(p, Parasitic::CouplingCap { .. }))
            .count()
    }

    /// Sum coupling-cap contributions from floating (unconnected) polygons.
    /// Since `PexReport` does not carry polygon attribution, this delegates to
    /// `run_pex_by_net` results: pass the by-net map and it returns the cap_af
    /// accrued to the sentinel net `u32::MAX` (polygons not on any device net).
    pub fn floating_metal_cap(
        &self,
        by_net: &std::collections::HashMap<u32, NetParasitics>,
    ) -> f64 {
        by_net.get(&u32::MAX).map_or(0.0, |np| np.cap_af)
    }

    /// Filter for resistance parasitics where length/width > threshold (high aspect ratio).
    pub fn high_ar_polygons(&self, threshold: f64) -> Vec<&Parasitic> {
        self.parasitics
            .iter()
            .filter(|p| {
                if let Parasitic::Resistance {
                    length_nm,
                    width_nm,
                    ..
                } = p
                {
                    *width_nm > 0 && (*length_nm as f64 / *width_nm as f64) > threshold
                } else {
                    false
                }
            })
            .collect()
    }
}

// --- Extractor structs implementing VerifyCheck ----------------------------

pub struct ResistanceExtractor {
    pub layer: LayerId,
    pub params: PexLayerParams,
}
pub struct ViaResistanceExtractor {
    pub layer: LayerId,
    pub params: PexLayerParams,
}
pub struct AreaFringeCapExtractor {
    pub layer: LayerId,
    pub params: PexLayerParams,
}
pub struct CouplingCapExtractor {
    pub layer: LayerId,
    pub params: PexLayerParams,
}

impl VerifyCheck for ResistanceExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str {
        "resistance"
    }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_resistance(store, &deck.layers, self.layer, &self.params, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for ViaResistanceExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str {
        "via_resistance"
    }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_via_resistance(store, &deck.layers, self.layer, &self.params, &mut out);
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for AreaFringeCapExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str {
        "area_fringe_cap"
    }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_area_fringe_cap(
            store,
            &deck.layers,
            self.layer,
            &self.params,
            deck,
            &mut out,
        );
        out.into_iter().map(|(p, _)| p).collect()
    }
}

impl VerifyCheck for CouplingCapExtractor {
    type Output = Vec<Parasitic>;
    fn id(&self) -> &str {
        "coupling_cap"
    }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Parasitic> {
        let mut out = Vec::new();
        extract_coupling_cap(
            store,
            &deck.layers,
            self.layer,
            &self.params,
            backend,
            &mut out,
        );
        out.into_iter().map(|(p, _)| p).collect()
    }
}

/// Build PEX extractors from the deck's per-layer params.
pub fn pex_rules_from_deck(deck: &Deck) -> Vec<Box<dyn VerifyCheck<Output = Vec<Parasitic>>>> {
    let mut rules: Vec<Box<dyn VerifyCheck<Output = Vec<Parasitic>>>> = Vec::new();
    for (&lid, params) in &deck.pex {
        rules.push(Box::new(ResistanceExtractor {
            layer: lid,
            params: params.clone(),
        }));
        rules.push(Box::new(ViaResistanceExtractor {
            layer: lid,
            params: params.clone(),
        }));
        rules.push(Box::new(AreaFringeCapExtractor {
            layer: lid,
            params: params.clone(),
        }));
        rules.push(Box::new(CouplingCapExtractor {
            layer: lid,
            params: params.clone(),
        }));
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
    let parasitics = extract_all(store, deck, Backend::Cpu)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
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
/// `ExtractedNetlist::net_of_poly`). Unattributed/floating geometry is retained
/// under the sentinel net `u32::MAX`, so [`PexReport::floating_metal_cap`] can
/// report it rather than silently losing it.
///
/// * Resistance and area/fringe cap accrue to the polygon's net.
/// * Coupling cap accrues in FULL to BOTH nets (conservative for budget checks); pairs on
///   the same net are skipped — same-net coupling is not a parasitic to budget.
///
/// This compatibility entry point returns numeric values even when another polygon produced
/// an extraction diagnostic. Signoff callers should use [`run_pex_by_net_checked`] to fail
/// closed on unsupported geometry.
pub fn run_pex_by_net(
    store: &GeometryStore,
    deck: &Deck,
    net_of_poly: &[u32],
) -> std::collections::HashMap<u32, NetParasitics> {
    aggregate_by_net(extract_all(store, deck, Backend::Cpu), net_of_poly)
}

/// Fail-closed per-net PEX. Returns all extraction diagnostics instead of a partial numeric
/// map when any configured R/C model encounters unsupported geometry.
pub fn run_pex_by_net_checked(
    store: &GeometryStore,
    deck: &Deck,
    net_of_poly: &[u32],
) -> Result<std::collections::HashMap<u32, NetParasitics>, Vec<Parasitic>> {
    let extracted = extract_all(store, deck, Backend::Cpu);
    let diagnostics: Vec<Parasitic> = extracted
        .iter()
        .filter_map(|(parasitic, _)| {
            matches!(parasitic, Parasitic::ExtractionDiagnostic { .. }).then(|| parasitic.clone())
        })
        .collect();
    if diagnostics.is_empty() {
        Ok(aggregate_by_net(extracted, net_of_poly))
    } else {
        Err(diagnostics)
    }
}

fn aggregate_by_net(
    extracted: Vec<Attributed>,
    net_of_poly: &[u32],
) -> std::collections::HashMap<u32, NetParasitics> {
    let net = |p: u32| -> u32 { net_of_poly.get(p as usize).copied().unwrap_or(u32::MAX) };
    let mut out: std::collections::HashMap<u32, NetParasitics> = std::collections::HashMap::new();
    for (par, polys) in extracted {
        match par {
            Parasitic::Resistance { ohm, .. } | Parasitic::ViaResistance { ohm, .. } => {
                let n = net(polys[0]);
                out.entry(n).or_default().r_ohm += ohm;
            }
            Parasitic::AreaCap { af, .. } => {
                let n = net(polys[0]);
                out.entry(n).or_default().cap_af += af;
            }
            Parasitic::CouplingCap { af, .. } | Parasitic::InterlayerCap { af, .. } => {
                let (na, nb) = (net(polys[0]), net(polys[1]));
                if na == nb {
                    continue;
                } // same-net coupling: not a parasitic to budget
                out.entry(na).or_default().cap_af += af;
                out.entry(nb).or_default().cap_af += af;
            }
            Parasitic::ExtractionDiagnostic { .. } => {}
        }
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct RectilinearMetrics {
    area_nm2: f64,
    perimeter_nm: f64,
    equivalent_length_nm: f64,
    equivalent_width_nm: f64,
}

/// Measure a simple Manhattan polygon without substituting its bounding box.
///
/// The equivalent rectangle is the rectangle with the same area and perimeter as the
/// polygon. Its dimensions are the roots of `t² - (P/2)t + A = 0`. This is exact for a
/// rectangle and for ideal constant-width orthogonal traces (including bends). Indentations
/// increase perimeter and therefore conservatively increase the estimated number of squares.
fn rectilinear_metrics(store: &GeometryStore, poly: PolyId) -> Result<RectilinearMetrics, String> {
    let (start, end) = store.poly_range(poly);
    let count = end - start;
    if count < 4 {
        return Err(format!(
            "polygon has {count} vertices; at least four are required"
        ));
    }
    if crate::geometry::poly_self_intersects(store, poly) {
        return Err("self-intersecting polygon is unsupported".into());
    }

    let mut area2 = 0_i128;
    let mut perimeter_nm = 0_f64;
    for i in 0..count {
        let (x0, y0) = store.poly_vertex(start, i);
        let (x1, y1) = store.poly_vertex(start, (i + 1) % count);
        let dx = x1 as i64 - x0 as i64;
        let dy = y1 as i64 - y0 as i64;
        if dx == 0 && dy == 0 {
            return Err(format!("zero-length edge at vertex {i}"));
        }
        if dx != 0 && dy != 0 {
            return Err(format!("non-Manhattan edge at vertex {i}"));
        }
        perimeter_nm += (dx.abs() + dy.abs()) as f64;
        area2 += x0 as i128 * y1 as i128 - x1 as i128 * y0 as i128;
    }
    let area_nm2 = area2.abs() as f64 / 2.0;
    if !area_nm2.is_finite() || area_nm2 <= 0.0 || perimeter_nm <= 0.0 {
        return Err("polygon has zero or non-finite area/perimeter".into());
    }

    let semiperimeter = perimeter_nm / 2.0;
    let raw_discriminant = semiperimeter * semiperimeter - 4.0 * area_nm2;
    let tolerance = semiperimeter * semiperimeter * 1e-12;
    if raw_discriminant < -tolerance {
        return Err("area/perimeter cannot form a rectilinear equivalent rectangle".into());
    }
    let root = raw_discriminant.max(0.0).sqrt();
    let equivalent_length_nm = (semiperimeter + root) / 2.0;
    let equivalent_width_nm = (semiperimeter - root) / 2.0;
    if equivalent_width_nm <= 0.0 || !equivalent_length_nm.is_finite() {
        return Err("polygon has no positive equivalent conductor width".into());
    }
    Ok(RectilinearMetrics {
        area_nm2,
        perimeter_nm,
        equivalent_length_nm,
        equivalent_width_nm,
    })
}

fn report_dimension_nm(value: f64) -> i32 {
    value.round().clamp(1.0, i32::MAX as f64) as i32
}

fn extraction_diagnostic(
    lt: &LayerTable,
    layer: LayerId,
    poly: PolyId,
    model: &str,
    message: String,
) -> Attributed {
    (
        Parasitic::ExtractionDiagnostic {
            layer: lt.name(layer).into(),
            polygon: poly.0,
            model: model.into(),
            message,
        },
        [poly.0, u32::MAX],
    )
}

/// Sheet resistance of every valid conductor polygon.
///
/// `R = Rs * Leq/Weq`, where the equivalent dimensions come from exact polygon area and
/// perimeter. The scalar remains a conservative analytical estimate: terminal-aware current
/// flow and distributed reduction require an RC network extractor or field solver.
fn extract_resistance(
    store: &GeometryStore,
    lt: &LayerTable,
    layer: LayerId,
    p: &PexLayerParams,
    out: &mut Vec<Attributed>,
) {
    if p.sheet_res_ohm_sq == 0.0 {
        return;
    }
    for poly in store.polys_on_layer(layer) {
        let metrics = match rectilinear_metrics(store, poly) {
            Ok(metrics) => metrics,
            Err(message) => {
                out.push(extraction_diagnostic(
                    lt,
                    layer,
                    poly,
                    "sheet_resistance",
                    message,
                ));
                continue;
            }
        };
        let squares = metrics.equivalent_length_nm / metrics.equivalent_width_nm;
        let ohm = p.sheet_res_ohm_sq * squares;
        out.push((
            Parasitic::Resistance {
                layer: lt.name(layer).into(),
                ohm,
                length_nm: report_dimension_nm(metrics.equivalent_length_nm),
                width_nm: report_dimension_nm(metrics.equivalent_width_nm),
            },
            [poly.0, u32::MAX],
        ));
    }
}

/// Fixed per-via resistance for each polygon on a via/contact layer.
fn extract_via_resistance(
    store: &GeometryStore,
    lt: &LayerTable,
    layer: LayerId,
    p: &PexLayerParams,
    out: &mut Vec<Attributed>,
) {
    if p.via_res_ohm == 0.0 {
        return;
    }
    for poly in store.polys_on_layer(layer) {
        out.push((
            Parasitic::ViaResistance {
                layer: lt.name(layer).into(),
                ohm: p.via_res_ohm,
            },
            [poly.0, u32::MAX],
        ));
    }
}

/// Area + fringe capacitance to substrate for every valid conductor polygon.
///
/// Fill density and ground-plane shielding need explicit process-calibrated models. They are
/// deliberately not inferred from polygon size or hard-coded layer names here.
fn extract_area_fringe_cap(
    store: &GeometryStore,
    lt: &LayerTable,
    layer: LayerId,
    p: &PexLayerParams,
    _deck: &Deck,
    out: &mut Vec<Attributed>,
) {
    let layer_name = lt.name(layer);
    if p.area_cap_af_um2 == 0.0 && p.fringe_cap_af_um == 0.0 {
        return;
    }

    for poly in store.polys_on_layer(layer) {
        let metrics = match rectilinear_metrics(store, poly) {
            Ok(metrics) => metrics,
            Err(message) => {
                out.push(extraction_diagnostic(
                    lt,
                    layer,
                    poly,
                    "ground_capacitance",
                    message,
                ));
                continue;
            }
        };
        let area_um2 = metrics.area_nm2 / (NM_PER_UM * NM_PER_UM);
        let perim_um = metrics.perimeter_nm / NM_PER_UM;
        let area_af = p.area_cap_af_um2 * area_um2;
        let fringe_af = p.fringe_cap_af_um * perim_um;
        let af = area_af + fringe_af;
        out.push((
            Parasitic::AreaCap {
                layer: layer_name.into(),
                af,
                area_af,
                fringe_af,
                area_um2,
                perimeter_um: perim_um,
            },
            [poly.0, u32::MAX],
        ));
    }
}

/// Inter-layer coupling capacitance between wires on DIFFERENT metal layers that cross
/// over/under each other. C = interlayer_cap_af_um2 * overlap_area_um2 for each polygon
/// pair that overlaps in the x-y plane.
fn extract_interlayer_cap(store: &GeometryStore, deck: &Deck, out: &mut Vec<Attributed>) {
    let pex_layers: Vec<(LayerId, &PexLayerParams)> =
        deck.pex.iter().map(|(&lid, p)| (lid, p)).collect();
    let n = pex_layers.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let (lid_a, params_a) = pex_layers[i];
            let (lid_b, params_b) = pex_layers[j];
            // Use the average of both layers' interlayer_cap coefficients (or whichever is nonzero).
            let coeff = if params_a.interlayer_cap_af_um2 != 0.0
                && params_b.interlayer_cap_af_um2 != 0.0
            {
                (params_a.interlayer_cap_af_um2 + params_b.interlayer_cap_af_um2) / 2.0
            } else {
                params_a.interlayer_cap_af_um2 + params_b.interlayer_cap_af_um2 // one is 0.0
            };
            if coeff == 0.0 {
                continue;
            }
            let polys_a: Vec<PolyId> = store.polys_on_layer(lid_a).collect();
            let polys_b: Vec<PolyId> = store.polys_on_layer(lid_b).collect();
            // overlap requires bbox intersection: x-sweep candidates, not all pairs
            for (pa, pb) in crate::drc::candidate_pairs(store, &polys_a, Some(&polys_b), 0) {
                let ba = store.poly_bbox[pa.0 as usize];
                let bb = store.poly_bbox[pb.0 as usize];
                // Compute x-y overlap area (bbox intersection).
                let ox = (ba.xmax.min(bb.xmax) - ba.xmin.max(bb.xmin)).max(0);
                let oy = (ba.ymax.min(bb.ymax) - ba.ymin.max(bb.ymin)).max(0);
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                let overlap_area_um2 = (ox as f64 / NM_PER_UM) * (oy as f64 / NM_PER_UM);
                let af = coeff * overlap_area_um2;
                out.push((
                    Parasitic::InterlayerCap {
                        layer_a: deck.layers.name(lid_a).into(),
                        layer_b: deck.layers.name(lid_b).into(),
                        af,
                        overlap_area_um2,
                    },
                    [pa.0, pb.0],
                ));
            }
        }
    }
}

/// Lateral coupling capacitance between parallel same-layer wires that face each other.
fn extract_coupling_cap(
    store: &GeometryStore,
    lt: &LayerTable,
    layer: LayerId,
    p: &PexLayerParams,
    backend: Backend,
    out: &mut Vec<Attributed>,
) {
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
    let n = polys.len();
    let n_pairs = n * n.saturating_sub(1) / 2;

    // GPU path: compute run length + gap for all pairs in parallel
    if backend == Backend::Gpu && n_pairs >= (1 << 18) {
        let xmins: Vec<f32> = polys
            .iter()
            .map(|q| store.poly_bbox[q.0 as usize].xmin as f32)
            .collect();
        let ymins: Vec<f32> = polys
            .iter()
            .map(|q| store.poly_bbox[q.0 as usize].ymin as f32)
            .collect();
        let xmaxs: Vec<f32> = polys
            .iter()
            .map(|q| store.poly_bbox[q.0 as usize].xmax as f32)
            .collect();
        let ymaxs: Vec<f32> = polys
            .iter()
            .map(|q| store.poly_bbox[q.0 as usize].ymax as f32)
            .collect();
        let mut pa = Vec::with_capacity(n_pairs);
        let mut pb = Vec::with_capacity(n_pairs);
        for i in 0..n {
            for j in (i + 1)..n {
                pa.push(i as u32);
                pb.push(j as u32);
            }
        }
        if let Some((runs, gaps)) =
            crate::traits::coupling_scan_gpu(&xmins, &ymins, &xmaxs, &ymaxs, &pa, &pb)
        {
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
                        out.push((
                            Parasitic::CouplingCap {
                                layer: lt.name(layer).into(),
                                af,
                                spacing_nm: spacing,
                                run_length_um: run_um,
                            },
                            [polys[i].0, polys[j].0],
                        ));
                    }
                }
            }
            return;
        }
    }

    // CPU fallback
    // ponytail: all-pairs — the 1/S model has no distance cutoff, so every pair
    // couples; add a coupling_max_spacing_nm deck param before sweep-pruning this.
    for i in 0..n {
        for j in (i + 1)..n {
            let a = store.poly_bbox[polys[i].0 as usize];
            let b = store.poly_bbox[polys[j].0 as usize];
            let x_overlap = a.xmax.min(b.xmax) - a.xmin.max(b.xmin);
            let y_gap = if a.ymax <= b.ymin {
                b.ymin - a.ymax
            } else if b.ymax <= a.ymin {
                a.ymin - b.ymax
            } else {
                -1
            };
            if x_overlap > 0 && y_gap > 0 {
                let run_um = x_overlap as f64 / NM_PER_UM;
                let spacing = y_gap;
                let scale = p.coupling_ref_spacing_nm / spacing as f64;
                let af = p.coupling_cap_af_um * run_um * scale;
                out.push((
                    Parasitic::CouplingCap {
                        layer: lt.name(layer).into(),
                        af,
                        spacing_nm: spacing,
                        run_length_um: run_um,
                    },
                    [polys[i].0, polys[j].0],
                ));
                continue;
            }
            let y_overlap = a.ymax.min(b.ymax) - a.ymin.max(b.ymin);
            let x_gap = if a.xmax <= b.xmin {
                b.xmin - a.xmax
            } else if b.xmax <= a.xmin {
                a.xmin - b.xmax
            } else {
                -1
            };
            if y_overlap > 0 && x_gap > 0 {
                let run_um = y_overlap as f64 / NM_PER_UM;
                let spacing = x_gap;
                let scale = p.coupling_ref_spacing_nm / spacing as f64;
                let af = p.coupling_cap_af_um * run_um * scale;
                out.push((
                    Parasitic::CouplingCap {
                        layer: lt.name(layer).into(),
                        af,
                        spacing_nm: spacing,
                        run_length_um: run_um,
                    },
                    [polys[i].0, polys[j].0],
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_deck() -> Deck {
        Deck::from_json(
            r#"{
            "layers": { "met1": { "layer": 68, "datatype": 20 } },
            "drc": {},
            "pex": { "met1": {
                "sheet_res_ohm_sq": 0.1,
                "area_cap_af_um2": 25.0,
                "fringe_cap_af_um": 40.0,
                "coupling_cap_af_um": 100.0,
                "coupling_ref_spacing_nm": 200
            } }
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn per_net_attribution() {
        let deck = test_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let mut st = GeometryStore::new();
        st.add_rect(met1, 0, 0, 2000, 200); // wire A: 10 squares -> 1.0 ohm
        st.add_rect(met1, 0, 400, 2000, 200); // wire B: parallel, 200nm gap

        // Every conductor gets R and ground C. For either wire:
        // area C = 25 * 0.4 = 10 aF; fringe C = 40 * 4.4 = 176 aF.
        const GROUND_CAP: f64 = 186.0;

        // distinct nets: R and ground C per net, full coupling to both
        let by_net = run_pex_by_net(&st, &deck, &[0, 1]);
        let a = by_net[&0];
        let b = by_net[&1];
        assert!((a.r_ohm - 1.0).abs() < 1e-9);
        assert!((b.r_ohm - 1.0).abs() < 1e-9);
        // coupling: 100 aF/um * 2.0 um * (200/200) = 200 aF, added to BOTH nets
        assert!((a.cap_af - (GROUND_CAP + 200.0)).abs() < 1e-9);
        assert!((b.cap_af - (GROUND_CAP + 200.0)).abs() < 1e-9);

        // same net: coupling skipped; resistance and ground capacitance accumulate
        let by_net = run_pex_by_net(&st, &deck, &[0, 0]);
        let a = by_net[&0];
        assert!((a.r_ohm - 2.0).abs() < 1e-9);
        assert!((a.cap_af - 2.0 * GROUND_CAP).abs() < 1e-9);

        // Both analytical contributions exist on both conductor polygons.
        let report = run_pex(&st, &deck);
        assert!((report.total_resistance("met1") - 2.0).abs() < 1e-9);
        assert_eq!(report.resistances().len(), 2);
        assert_eq!(report.area_caps().len(), 2);
        assert!(report.is_complete());

        // Floating attribution is retained under the sentinel net.  This used
        // to be silently skipped, making floating_metal_cap always return zero.
        let floating = run_pex_by_net(&st, &deck, &[u32::MAX, 1]);
        assert!((report.floating_metal_cap(&floating) - (GROUND_CAP + 200.0)).abs() < 1e-9);
    }

    #[test]
    fn exact_concave_area_perimeter_and_equivalent_resistance() {
        let deck = test_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let mut st = GeometryStore::new();
        // Constant-width L: area 0.19 um², perimeter 4.0 um. The equivalent rectangle
        // is 1900 x 100 nm, so the estimate is exactly 19 squares.
        st.add_polygon(
            met1,
            &[
                (0, 0),
                (1000, 0),
                (1000, 100),
                (100, 100),
                (100, 1000),
                (0, 1000),
            ],
        );

        let report = run_pex(&st, &deck);
        assert!(report.is_complete());
        assert!((report.total_resistance("met1") - 1.9).abs() < 1e-9);
        let cap = report.area_caps()[0];
        match cap {
            Parasitic::AreaCap {
                af,
                area_af,
                fringe_af,
                area_um2,
                perimeter_um,
                ..
            } => {
                assert!((*area_um2 - 0.19).abs() < 1e-12);
                assert!((*perimeter_um - 4.0).abs() < 1e-12);
                assert!((*area_af - 4.75).abs() < 1e-12);
                assert!((*fringe_af - 160.0).abs() < 1e-12);
                assert!((*af - 164.75).abs() < 1e-12);
                assert!((report.fringe_ratio() - 160.0 / 164.75).abs() < 1e-12);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn square_conductor_receives_both_resistance_and_capacitance() {
        let deck = test_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let mut st = GeometryStore::new();
        st.add_rect(met1, 0, 0, 1000, 1000);

        let report = run_pex(&st, &deck);
        assert!((report.total_resistance("met1") - 0.1).abs() < 1e-12);
        assert_eq!(report.area_caps().len(), 1);
        assert!((report.total_cap() - 185.0).abs() < 1e-12);
        assert!((report.fringe_ratio() - 160.0 / 185.0).abs() < 1e-12);
    }

    #[test]
    fn unconfigured_fill_and_shielding_heuristics_do_not_change_capacitance() {
        let deck = Deck::from_json(
            r#"{
            "layers": {
                "met1": { "layer": 68, "datatype": 20 },
                "met2": { "layer": 69, "datatype": 20 }
            },
            "drc": {},
            "pex": {
                "met1": {
                    "sheet_res_ohm_sq": 0.1,
                    "area_cap_af_um2": 25.0,
                    "fringe_cap_af_um": 40.0,
                    "coupling_cap_af_um": 0.0,
                    "coupling_ref_spacing_nm": 200
                },
                "met2": {
                    "sheet_res_ohm_sq": 0.08,
                    "area_cap_af_um2": 15.0,
                    "fringe_cap_af_um": 30.0,
                    "coupling_cap_af_um": 0.0,
                    "coupling_ref_spacing_nm": 200
                }
            }
        }"#,
        )
        .unwrap();
        let met1 = deck.layers.id("met1").unwrap();
        let met2 = deck.layers.id("met2").unwrap();
        let mut st = GeometryStore::new();
        st.add_rect(met1, 0, 0, 2000, 2000); // old code treated this as a met2 shield
        st.add_rect(met2, 500, 500, 1000, 1000);
        st.add_rect(met1, 3000, 0, 90, 90); // old code inferred dummy fill

        let report = run_pex(&st, &deck);
        let mut met1_small = None;
        let mut met2_plate = None;
        for cap in report.area_caps() {
            if let Parasitic::AreaCap {
                layer,
                af,
                area_um2,
                ..
            } = cap
            {
                if layer == "met1" && (*area_um2 - 0.0081).abs() < 1e-12 {
                    met1_small = Some(*af);
                }
                if layer == "met2" {
                    met2_plate = Some(*af);
                }
            }
        }
        assert!((met1_small.unwrap() - 14.6025).abs() < 1e-12);
        assert!((met2_plate.unwrap() - 135.0).abs() < 1e-12);
    }

    #[test]
    fn unsupported_geometry_is_reported_instead_of_silently_zeroed() {
        let deck = test_deck();
        let met1 = deck.layers.id("met1").unwrap();
        let mut st = GeometryStore::new();
        st.add_polygon(met1, &[(0, 0), (1000, 0), (500, 500)]);

        let report = run_pex(&st, &deck);
        assert!(!report.is_complete());
        assert_eq!(report.diagnostics().len(), 2); // independent R and ground-C models
        assert!(report.resistances().is_empty());
        assert!(report.area_caps().is_empty());
        assert_eq!(
            run_pex_by_net_checked(&st, &deck, &[0]).unwrap_err().len(),
            2
        );
    }
}
