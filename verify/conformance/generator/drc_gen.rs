use crate::helper::*;

pub fn generate(s: &mut Suite) {
    min_width(s);
    min_width_lshape(s);
    min_spacing(s);
    min_spacing_diff(s);
    min_enclosure(s);
    min_enclosure_unhosted(s);
    min_enclosure_concave(s);
    min_extension(s);
    min_extension_horiz(s);
    min_area(s);
    min_area_boundary(s);
    max_width(s);
    notch(s);
    notch_vs_spacing(s);
    min_edge_length(s);
    off_grid(s);
    angle(s);
    min_density(s);
    min_density_multiwindow(s);
    max_density(s);
    density_gradient(s);
    overlap(s);
    corner_to_corner(s);
    fuzz_degenerate(s);
    fuzz_cross_cutting(s);
    strict_mode(s);
    antenna(s);
    eol_spacing(s);
    well_enclosure(s);
    wide_dependent_spacing(s);
    prl_spacing(s);
    asymmetric_enclosure(s);
    min_enclosed_area(s);
    cheesing(s);
    redundant_via(s);
    via_array_spacing(s);
    max_distance_to_tap(s);
    multi_patterning(s);
}

fn min_width(s: &mut Suite) {
    s.gds.begin_cell("DRC_MW_PASS");
    s.gds.rect(MET1, 0, 0, 100, 1000);
    s.gds.end_cell();
    s.add_drc("DRC_MW_PASS", "DRC_MW_PASS", "min_width", 0);

    s.gds.begin_cell("DRC_MW_FAIL");
    s.gds.rect(MET1, 0, 0, 80, 1000);
    s.gds.end_cell();
    s.add_drc_measured("DRC_MW_FAIL", "DRC_MW_FAIL", "min_width", 1, 80);
}

/// L-shape with 80nm narrow arm → non-rectangular width violation
fn min_width_lshape(s: &mut Suite) {
    // L: bottom bar 200×80 (narrow), left arm 100×300
    s.gds.begin_cell("DRC_MW_L");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (200, 0), (200, 80), (100, 80), (100, 300), (0, 300),
    ]);
    s.gds.end_cell();
    s.add_drc_measured("DRC_MW_LSHAPE", "DRC_MW_L", "min_width", 1, 80);
}

fn min_spacing(s: &mut Suite) {
    s.gds.begin_cell("DRC_MS_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET1, 300, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MS_PASS", "DRC_MS_PASS", "min_spacing", 0);

    s.gds.begin_cell("DRC_MS_FAIL");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET1, 280, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_MS_FAIL", "DRC_MS_FAIL", "min_spacing", 1, 80);
}

fn min_spacing_diff(s: &mut Suite) {
    s.gds.begin_cell("DRC_MSD_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET2, 0, 300, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MSD_PASS", "DRC_MSD_PASS", "min_spacing_diff", 0);

    s.gds.begin_cell("DRC_MSD_FAIL");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET2, 0, 280, 200, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_MSD_FAIL", "DRC_MSD_FAIL", "min_spacing_diff", 1, 80);
}

fn min_enclosure(s: &mut Suite) {
    s.gds.begin_cell("DRC_ENC_PASS");
    s.gds.rect(MET1, 0, 0, 300, 300);
    s.gds.rect(MET2, 50, 50, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_ENC_PASS", "DRC_ENC_PASS", "min_enclosure", 0);

    s.gds.begin_cell("DRC_ENC_FAIL");
    s.gds.rect(MET1, 0, 0, 300, 300);
    s.gds.rect(MET2, 30, 30, 240, 240);
    s.gds.end_cell();
    s.add_drc_measured("DRC_ENC_FAIL", "DRC_ENC_FAIL", "min_enclosure", 1, 30);
}

/// met2 rect with NO met1 at all → enclosure measured=0
fn min_enclosure_unhosted(s: &mut Suite) {
    s.gds.begin_cell("DRC_ENC_NONE");
    s.gds.rect(MET2, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_ENC_UNHOSTED", "DRC_ENC_NONE", "min_enclosure", 1, 0);
}

/// Enclosure across concave corner: L-shaped outer polygon.
/// Inner rect near the concave corner tests that enclosure isn't fooled by bbox.
fn min_enclosure_concave(s: &mut Suite) {
    // L-shape outer: bottom 500×200, left arm 200×500
    // Inner rect placed at (60, 60) 80×80 — well inside the bottom portion → PASS
    s.gds.begin_cell("DRC_ENC_CONC_P");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (500, 0), (500, 200), (200, 200), (200, 500), (0, 500),
    ]);
    s.gds.rect(MET2, 60, 60, 80, 80);
    s.gds.end_cell();
    s.add_drc("DRC_ENC_CONC_PASS", "DRC_ENC_CONC_P", "min_enclosure", 0);

    // Inner rect at (250, 250) — inside L's bbox but OUTSIDE the actual polygon
    // → unhosted (measured=0)
    s.gds.begin_cell("DRC_ENC_CONC_F");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (500, 0), (500, 200), (200, 200), (200, 500), (0, 500),
    ]);
    s.gds.rect(MET2, 250, 250, 80, 80);
    s.gds.end_cell();
    s.add_drc_measured("DRC_ENC_CONC_FAIL", "DRC_ENC_CONC_F", "min_enclosure", 1, 0);
}

fn min_extension(s: &mut Suite) {
    s.gds.begin_cell("DRC_EXT_PASS");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.end_cell();
    s.add_drc("DRC_EXT_PASS", "DRC_EXT_PASS", "min_extension", 0);

    s.gds.begin_cell("DRC_EXT_FAIL");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(POLY, 200, -20, 50, 240);
    s.gds.end_cell();
    s.add_drc_measured("DRC_EXT_FAIL", "DRC_EXT_FAIL", "min_extension", 2, 20);
}

/// Horizontal poly wider than diff → extension measured along x
fn min_extension_horiz(s: &mut Suite) {
    // wide poly bar, narrow diff strip crossing it vertically
    // poly bbox wider than tall → horizontal orientation
    s.gds.begin_cell("DRC_EXT_H_PASS");
    s.gds.rect(DIFF, 200, 0, 100, 500);
    s.gds.rect(POLY, -50, 200, 600, 50); // extends 50nm past diff on both sides in x
    s.gds.end_cell();
    s.add_drc("DRC_EXT_H_PASS", "DRC_EXT_H_PASS", "min_extension", 0);

    s.gds.begin_cell("DRC_EXT_H_FAIL");
    s.gds.rect(DIFF, 200, 0, 100, 500);
    s.gds.rect(POLY, 180, 200, 140, 50); // 20nm extension on each side
    s.gds.end_cell();
    s.add_drc_measured("DRC_EXT_H_FAIL", "DRC_EXT_H_FAIL", "min_extension", 2, 20);
}

fn min_area(s: &mut Suite) {
    s.gds.begin_cell("DRC_MA_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MA_PASS", "DRC_MA_PASS", "min_area", 0);

    s.gds.begin_cell("DRC_MA_FAIL");
    s.gds.rect(MET1, 0, 0, 100, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_MA_FAIL", "DRC_MA_FAIL", "min_area", 1, 20000);
}

/// Area exactly at limit (40000) → boundary, no violation
fn min_area_boundary(s: &mut Suite) {
    s.gds.begin_cell("DRC_MA_EXACT");
    s.gds.rect(MET1, 0, 0, 400, 100); // 400×100 = 40000 exactly
    s.gds.end_cell();
    s.add_drc("DRC_MA_EXACT", "DRC_MA_EXACT", "min_area", 0);
}

fn max_width(s: &mut Suite) {
    s.gds.begin_cell("DRC_XW_PASS");
    s.gds.rect(MET1, 0, 0, 2000, 2000);
    s.gds.end_cell();
    s.add_drc("DRC_XW_PASS", "DRC_XW_PASS", "max_width", 0);

    s.gds.begin_cell("DRC_XW_FAIL");
    s.gds.rect(MET1, 0, 0, 6000, 6000);
    s.gds.end_cell();
    s.add_drc_measured("DRC_XW_FAIL", "DRC_XW_FAIL", "max_width", 1, 6000);
}

fn notch(s: &mut Suite) {
    // U-shape with 100nm notch (arms 100nm wide each, notch gap = 100)
    s.gds.begin_cell("DRC_N_PASS");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (300, 0), (300, 300), (200, 300),
        (200, 100), (100, 100), (100, 300), (0, 300),
    ]);
    s.gds.end_cell();
    s.add_drc("DRC_N_PASS", "DRC_N_PASS", "notch", 0);

    // U-shape with 80nm notch
    s.gds.begin_cell("DRC_N_FAIL");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (300, 0), (300, 300), (190, 300),
        (190, 100), (110, 100), (110, 300), (0, 300),
    ]);
    s.gds.end_cell();
    s.add_drc_measured("DRC_N_FAIL", "DRC_N_FAIL", "notch", 1, 80);
}

/// Notch vs spacing disambiguation on merged shapes.
/// Two bars with sub-min gap connected by bridge → one merged shape → "notch".
/// Same bars without bridge → separate shapes → "min_spacing".
fn notch_vs_spacing(s: &mut Suite) {
    // Merged: left + right bars connected by bottom bridge
    s.gds.begin_cell("DRC_NVS_MERGE");
    s.gds.rect(MET1, 0, 0, 200, 500);
    s.gds.rect(MET1, 280, 0, 200, 500);
    s.gds.rect(MET1, 0, 0, 480, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_NVS_MERGE_NOTCH", "DRC_NVS_MERGE", "notch", 1, 80);
    s.add_drc("DRC_NVS_MERGE_SPACE", "DRC_NVS_MERGE", "min_spacing", 0);

    // Separate: same bars, no bridge
    s.gds.begin_cell("DRC_NVS_SEP");
    s.gds.rect(MET1, 0, 0, 200, 500);
    s.gds.rect(MET1, 280, 0, 200, 500);
    s.gds.end_cell();
    s.add_drc_measured("DRC_NVS_SEP_SPACE", "DRC_NVS_SEP", "min_spacing", 1, 80);
    s.add_drc("DRC_NVS_SEP_NOTCH", "DRC_NVS_SEP", "notch", 0);
}

fn min_edge_length(s: &mut Suite) {
    s.gds.begin_cell("DRC_EL_PASS");
    s.gds.rect(MET1, 0, 0, 100, 500);
    s.gds.end_cell();
    s.add_drc("DRC_EL_PASS", "DRC_EL_PASS", "min_edge_length", 0);

    // 80nm short edges → 2 violations
    s.gds.begin_cell("DRC_EL_FAIL");
    s.gds.rect(MET1, 0, 0, 80, 500);
    s.gds.end_cell();
    s.add_drc_measured("DRC_EL_FAIL", "DRC_EL_FAIL", "min_edge_length", 2, 80);
}

fn off_grid(s: &mut Suite) {
    s.gds.begin_cell("DRC_OG_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_OG_PASS", "DRC_OG_PASS", "off_grid", 0);

    // x=103 off 5nm grid → 2 vertices off-grid
    s.gds.begin_cell("DRC_OG_FAIL");
    s.gds.rect(MET1, 0, 0, 103, 200);
    s.gds.end_cell();
    s.add_drc("DRC_OG_FAIL", "DRC_OG_FAIL", "off_grid", 2);
}

fn angle(s: &mut Suite) {
    s.gds.begin_cell("DRC_ANG_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_ANG_PASS", "DRC_ANG_PASS", "angle", 0);

    // right triangle: hypotenuse at 135° → 1 violation
    s.gds.begin_cell("DRC_ANG_FAIL");
    s.gds.boundary(MET1.0, MET1.1, &[(0, 0), (200, 0), (0, 200)]);
    s.gds.end_cell();
    s.add_drc("DRC_ANG_FAIL", "DRC_ANG_FAIL", "angle", 1);
}

fn min_density(s: &mut Suite) {
    // frac = 2400000/4000000 = 0.6 ≥ 0.3
    s.gds.begin_cell("DRC_MD_PASS");
    s.gds.rect(MET1, 0, 0, 1200, 2000);
    s.gds.end_cell();
    s.add_drc("DRC_MD_PASS", "DRC_MD_PASS", "min_density", 0);

    // frac = 250000/4000000 = 0.0625 < 0.3
    s.gds.begin_cell("DRC_MD_FAIL");
    s.gds.rect(MET1, 0, 0, 500, 500);
    s.gds.end_cell();
    s.add_drc_frac("DRC_MD_FAIL", "DRC_MD_FAIL", "min_density", 1, 0.0625, 0.3);
}

/// Rect spanning two density windows: one passes, one fails
fn min_density_multiwindow(s: &mut Suite) {
    // 3000×1000 rect → 2 windows of 2000×2000 each
    // window 0: frac = 2000*1000/4000000 = 0.5 ≥ 0.3 → OK
    // window 1: frac = 1000*1000/4000000 = 0.25 < 0.3 → FAIL
    s.gds.begin_cell("DRC_MD_MULTI");
    s.gds.rect(MET1, 0, 0, 3000, 1000);
    s.gds.end_cell();
    s.add_drc_frac("DRC_MD_MULTI", "DRC_MD_MULTI", "min_density", 1, 0.25, 0.3);
}

fn max_density(s: &mut Suite) {
    // frac = 1000000/4000000 = 0.25 < 0.8
    s.gds.begin_cell("DRC_XD_PASS");
    s.gds.rect(MET1, 0, 0, 1000, 1000);
    s.gds.end_cell();
    s.add_drc("DRC_XD_PASS", "DRC_XD_PASS", "max_density", 0);

    // frac = 3240000/4000000 = 0.81 > 0.8
    s.gds.begin_cell("DRC_XD_FAIL");
    s.gds.rect(MET1, 0, 0, 1800, 1800);
    s.gds.end_cell();
    s.add_drc_frac("DRC_XD_FAIL", "DRC_XD_FAIL", "max_density", 1, 0.81, 0.8);
}

/// Density gradient: dense window 0 passes both rules, sparse window 1 fails min_density.
/// Tests that windowed coverage handles sharp density boundaries.
fn density_gradient(s: &mut Suite) {
    // Window=2000. Layout spans 2 windows in x.
    // Dense plate in window 0, tiny marker in window 1 to force the bbox out.
    // gb = [200,200,2600,1800], nx=2, ny=1
    // W0 [200,200,2200,2200]: plate 1600×1600 → 2560000/4000000 = 0.64 → passes both
    // W1 [2200,200,4200,2200]: tiny 100×100 → 10000/4000000 = 0.0025 < 0.3 → FAIL min
    s.gds.begin_cell("DRC_DGRAD");
    s.gds.rect(MET1, 200, 200, 1600, 1600);
    s.gds.rect(MET1, 2500, 500, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_DGRAD_MAX", "DRC_DGRAD", "max_density", 0);
    s.add_drc_frac("DRC_DGRAD_MIN", "DRC_DGRAD", "min_density", 1, 0.0025, 0.3);
}

fn overlap(s: &mut Suite) {
    // met1 and met2 overlap by 50nm
    s.gds.begin_cell("DRC_OV_PASS");
    s.gds.rect(MET1, 0, 0, 500, 500);
    s.gds.rect(MET2, 450, 0, 500, 500);
    s.gds.end_cell();
    s.add_drc("DRC_OV_PASS", "DRC_OV_PASS", "overlap", 0);

    // overlap only 30nm
    s.gds.begin_cell("DRC_OV_FAIL");
    s.gds.rect(MET1, 0, 0, 500, 500);
    s.gds.rect(MET2, 470, 0, 500, 500);
    s.gds.end_cell();
    s.add_drc_measured("DRC_OV_FAIL", "DRC_OV_FAIL", "overlap", 1, 30);
}

fn corner_to_corner(s: &mut Suite) {
    // diagonal dist = sqrt(200²+200²) = 282 ≥ 150
    s.gds.begin_cell("DRC_C2C_PASS");
    s.gds.rect(MET1, 0, 0, 100, 100);
    s.gds.rect(MET1, 300, 300, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_C2C_PASS", "DRC_C2C_PASS", "corner_to_corner", 0);

    // diagonal dist = sqrt(100²+100²) = 141 < 150
    s.gds.begin_cell("DRC_C2C_FAIL");
    s.gds.rect(MET1, 0, 0, 100, 100);
    s.gds.rect(MET1, 200, 200, 100, 100);
    s.gds.end_cell();
    s.add_drc_measured("DRC_C2C_FAIL", "DRC_C2C_FAIL", "corner_to_corner", 1, 141);
}

/// Fuzzing: degenerate geometry the engine must handle without crash or silent heal.
fn fuzz_degenerate(s: &mut Suite) {
    // Zero-width sliver (w=0 rect): engine detects sub-min width, doesn't crash
    s.gds.begin_cell("DRC_FUZZ_ZERO");
    s.gds.rect(MET1, 0, 0, 0, 500);
    s.gds.end_cell();
    s.add_drc_measured("DRC_FUZZ_ZERO_W", "DRC_FUZZ_ZERO", "min_width", 1, 0);

    // Coincident edges: two rects sharing an entire edge → merged, no false violations
    s.gds.begin_cell("DRC_FUZZ_COIN");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET1, 200, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_FUZZ_COIN_SP", "DRC_FUZZ_COIN", "min_spacing", 0);
    s.add_drc("DRC_FUZZ_COIN_N", "DRC_FUZZ_COIN", "notch", 0);

    // Off-grid vertices (boolean-op artifact): 3 of 4 vertices off 5nm grid
    s.gds.begin_cell("DRC_FUZZ_OFFG");
    s.gds.boundary(MET1.0, MET1.1, &[(0, 0), (103, 0), (103, 207), (0, 207)]);
    s.gds.end_cell();
    s.add_drc("DRC_FUZZ_OFFG", "DRC_FUZZ_OFFG", "off_grid", 3);
}

/// Cross-cutting fuzzing: self-intersecting polygon and acute sliver.
/// Engine must not crash; lock observed behavior as golden.
fn fuzz_cross_cutting(s: &mut Suite) {
    // Self-intersecting (butterfly/bowtie): edges cross → degenerate
    // Vertices form a figure-8: (0,0)→(200,200)→(200,0)→(0,200)
    // The angle check fires on the diagonal edges (not 0° or 90°).
    s.gds.begin_cell("DRC_FUZZ_SELF");
    s.gds.boundary(MET1.0, MET1.1, &[
        (0, 0), (200, 200), (200, 0), (0, 200),
    ]);
    s.gds.end_cell();
    // 2 diagonal edges (45°/135°) violate; 2 vertical edges (90°) are allowed
    s.add_drc("DRC_FUZZ_SELF_ANG", "DRC_FUZZ_SELF", "angle", 2);

    // Acute sliver: very narrow triangle (1nm base × 500nm height)
    // Tests that the engine handles acute angles and near-zero area
    s.gds.begin_cell("DRC_FUZZ_ACUTE");
    s.gds.boundary(MET1.0, MET1.1, &[(0, 0), (5, 0), (0, 500)]);
    s.gds.end_cell();
    // The hypotenuse is not 0° or 90° → 1 angle violation
    s.add_drc("DRC_FUZZ_ACUTE_ANG", "DRC_FUZZ_ACUTE", "angle", 1);
}

/// STRICT mode tests: same geometry, different expected results.
/// In nominal: merged-shape gap → "notch". In STRICT: → "min_spacing".
fn strict_mode(s: &mut Suite) {
    // Reuse DRC_NVS_MERGE cell (merged bars with 80nm notch).
    // Nominal: notch=1, min_spacing=0 (already tested above).
    // STRICT: notch=0 (reclassified), min_spacing=1 (same-net enforced).
    s.add_drc_strict_measured(
        "DRC_STRICT_MERGE_SP", "DRC_NVS_MERGE", "min_spacing", 1, 80);
    s.add_drc_strict(
        "DRC_STRICT_MERGE_N", "DRC_NVS_MERGE", "notch", 0);

    // Separate bars: same result in both modes (min_spacing=1).
    s.add_drc_strict_measured(
        "DRC_STRICT_SEP_SP", "DRC_NVS_SEP", "min_spacing", 1, 80);
}

/// Antenna: metal area / gate area ratio check.
/// ratio limit = 100. Gate area = poly × diff overlap.
fn antenna(s: &mut Suite) {
    // PASS: small metal on gate net, ratio < 100
    // Gate: 50×200 poly over 500×200 diff → gate area = 50×200 = 10000
    // Gate contact: li overlapping poly top, mcon, small met1
    // Metal area = 200×200 = 40000, ratio = 4.0 < 100
    s.gds.begin_cell("DRC_ANT_PASS");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 210, 210, 30, 30);     // gate contact (overlaps poly)
    s.gds.rect(MCON, 215, 215, 20, 20);
    s.gds.rect(MET1, 200, 200, 200, 200);  // small antenna metal
    s.gds.rect(LI, 50, 50, 100, 100);      // S/D contact (separate net)
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_ANT_PASS", "DRC_ANT_PASS", "antenna", 0);

    // FAIL: huge metal antenna, ratio > 100
    // Same gate area = 10000. Metal = 5000×5000 = 25M. ratio = 2500 > 100
    s.gds.begin_cell("DRC_ANT_FAIL");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 210, 210, 30, 30);
    s.gds.rect(MCON, 215, 215, 20, 20);
    s.gds.rect(MET1, 200, 200, 5000, 5000); // huge antenna
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_ANT_FAIL", "DRC_ANT_FAIL", "antenna", 1);
}

/// EOL spacing: short edge (< eol_width=200) facing nearby polygon within eol_spacing=150.
fn eol_spacing(s: &mut Suite) {
    // PASS: two wires with short edges but far enough apart (200nm gap > 150nm limit)
    s.gds.begin_cell("DRC_EOL_PASS");
    s.gds.rect(MET1, 0, 0, 100, 500);   // 100nm short edge at top and bottom
    s.gds.rect(MET1, 0, 700, 100, 500);  // 200nm gap
    s.gds.end_cell();
    s.add_drc("DRC_EOL_PASS", "DRC_EOL_PASS", "eol_spacing", 0);

    // FAIL: two wires with short edges (100nm < 200nm eol_width), gap = 100nm < 150nm
    s.gds.begin_cell("DRC_EOL_FAIL");
    s.gds.rect(MET1, 0, 0, 100, 500);
    s.gds.rect(MET1, 0, 600, 100, 500);  // 100nm gap
    s.gds.end_cell();
    s.add_drc_measured("DRC_EOL_FAIL", "DRC_EOL_FAIL", "eol_spacing", 1, 100);

    // PASS: wide wire (500nm edge) — not an EOL situation even if close
    s.gds.begin_cell("DRC_EOL_WIDE");
    s.gds.rect(MET1, 0, 0, 500, 500);
    s.gds.rect(MET1, 0, 600, 500, 500);  // 100nm gap but 500nm edge > 200nm eol_width
    s.gds.end_cell();
    s.add_drc("DRC_EOL_WIDE", "DRC_EOL_WIDE", "eol_spacing", 0);
}

/// Well/implant enclosure: nwell must enclose diff by >= 50nm.
fn well_enclosure(s: &mut Suite) {
    // PASS: diff well inside nwell with 60nm margin on all sides
    s.gds.begin_cell("DRC_WE_PASS");
    s.gds.rect(NWELL, 0, 0, 320, 320);
    s.gds.rect(DIFF, 60, 60, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_WE_PASS", "DRC_WE_PASS", "min_enclosure", 0);

    // FAIL: diff only 30nm from nwell edge
    s.gds.begin_cell("DRC_WE_FAIL");
    s.gds.rect(NWELL, 0, 0, 260, 260);
    s.gds.rect(DIFF, 30, 30, 200, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_WE_FAIL", "DRC_WE_FAIL", "min_enclosure", 1, 30);

    // FAIL: diff completely outside nwell → unhosted, measured=0
    s.gds.begin_cell("DRC_WE_NONE");
    s.gds.rect(NWELL, 0, 0, 200, 200);
    s.gds.rect(DIFF, 300, 300, 200, 200);
    s.gds.end_cell();
    s.add_drc_measured("DRC_WE_NONE", "DRC_WE_NONE", "min_enclosure", 1, 0);
}

/// Wide-dependent spacing: wide metal (width >= 500nm) needs spacing >= 200nm.
fn wide_dependent_spacing(s: &mut Suite) {
    // PASS: wide wire (600nm) with 200nm gap to narrow wire
    s.gds.begin_cell("DRC_WDS_PASS");
    s.gds.rect(MET1, 0, 0, 200, 600);   // wide: min(200,600) = 200 < 500 → not wide
    s.gds.rect(MET1, 400, 0, 200, 200);  // 200nm gap
    s.gds.end_cell();
    s.add_drc("DRC_WDS_PASS", "DRC_WDS_PASS", "wide_dependent_spacing", 0);

    // FAIL: truly wide wire (min dim 500nm) with only 150nm gap
    s.gds.begin_cell("DRC_WDS_FAIL");
    s.gds.rect(MET1, 0, 0, 500, 1000);  // wide: min(500,1000) = 500 >= 500
    s.gds.rect(MET1, 650, 0, 200, 200); // 150nm gap
    s.gds.end_cell();
    s.add_drc_measured("DRC_WDS_FAIL", "DRC_WDS_FAIL", "wide_dependent_spacing", 1, 150);

    // PASS: wide wire with 200nm gap (at limit)
    s.gds.begin_cell("DRC_WDS_WIDE_OK");
    s.gds.rect(MET1, 0, 0, 500, 1000);
    s.gds.rect(MET1, 700, 0, 200, 200); // 200nm gap
    s.gds.end_cell();
    s.add_drc("DRC_WDS_WIDE_OK", "DRC_WDS_WIDE_OK", "wide_dependent_spacing", 0);
}

/// PRL (parallel run length) spacing: when two wires run parallel for >= prl_threshold,
/// spacing must be >= prl_spacing.
fn prl_spacing(s: &mut Suite) {
    // PASS: PRL=700 >= 500 threshold, gap=200 >= 200 required
    s.gds.begin_cell("DRC_PRL_PASS");
    s.gds.rect(MET1, 0, 0, 200, 800);
    s.gds.rect(MET1, 400, 100, 200, 800);
    s.gds.end_cell();
    s.add_drc("DRC_PRL_PASS", "DRC_PRL_PASS", "prl_spacing", 0);

    // FAIL: PRL=700 >= 500, gap=150 < 200
    s.gds.begin_cell("DRC_PRL_FAIL");
    s.gds.rect(MET1, 0, 0, 200, 800);
    s.gds.rect(MET1, 350, 100, 200, 800);
    s.gds.end_cell();
    s.add_drc_measured("DRC_PRL_FAIL", "DRC_PRL_FAIL", "prl_spacing", 1, 150);
}

/// Asymmetric enclosure: via1 inside met1 must have at least `min` enclosure
/// on the larger side of each axis.
fn asymmetric_enclosure(s: &mut Suite) {
    // PASS: via1 well inside met1, max(left,right)=180>=30, max(top,bottom)=180>=30
    s.gds.begin_cell("DRC_AENC_PASS");
    s.gds.rect(MET1, 0, 0, 300, 300);
    s.gds.rect(VIA1, 20, 20, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_AENC_PASS", "DRC_AENC_PASS", "asymmetric_enclosure", 0);

    // FAIL: via1 with only 10nm on both sides of x-axis, max(left,right)=10<30
    s.gds.begin_cell("DRC_AENC_FAIL");
    s.gds.rect(MET1, 0, 0, 120, 300);
    s.gds.rect(VIA1, 10, 50, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_AENC_FAIL", "DRC_AENC_FAIL", "asymmetric_enclosure", 1);
}

/// Minimum enclosed area: the area enclosed by a polygon ring must be >= min.
fn min_enclosed_area(s: &mut Suite) {
    // PASS: outer 500x500 with inner 100x100 cutout, enclosed=240000 >= 50000
    s.gds.begin_cell("DRC_MEA_PASS");
    s.gds.rect(MET1, 0, 0, 500, 500);
    s.gds.rect(MET1, 100, 100, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_MEA_PASS", "DRC_MEA_PASS", "min_enclosed_area", 0);

    // FAIL: outer 300x300 with inner 280x280, enclosed=11600 < 50000
    s.gds.begin_cell("DRC_MEA_FAIL");
    s.gds.rect(MET1, 0, 0, 300, 300);
    s.gds.rect(MET1, 10, 10, 280, 280);
    s.gds.end_cell();
    s.add_drc("DRC_MEA_FAIL", "DRC_MEA_FAIL", "min_enclosed_area", 1);
}

/// Cheesing: large metal plates (area > max) must have slots (inner cutouts).
fn cheesing(s: &mut Suite) {
    // PASS: large plate with slot
    s.gds.begin_cell("DRC_CH_PASS");
    s.gds.rect(MET1, 0, 0, 2000, 2000);
    s.gds.rect(MET1, 500, 500, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_CH_PASS", "DRC_CH_PASS", "cheesing", 0);

    // FAIL: large plate without slot, area=4M > 2M
    s.gds.begin_cell("DRC_CH_FAIL");
    s.gds.rect(MET1, 0, 0, 2000, 2000);
    s.gds.end_cell();
    s.add_drc("DRC_CH_FAIL", "DRC_CH_FAIL", "cheesing", 1);

    // PASS: small plate, area=1M < 2M
    s.gds.begin_cell("DRC_CH_SMALL");
    s.gds.rect(MET1, 0, 0, 1000, 1000);
    s.gds.end_cell();
    s.add_drc("DRC_CH_SMALL", "DRC_CH_SMALL", "cheesing", 0);
}

/// Redundant via: each via must have at least min_count vias within `within` distance.
fn redundant_via(s: &mut Suite) {
    // PASS: two via1 within 500nm, count=2 >= 2
    s.gds.begin_cell("DRC_RV_PASS");
    s.gds.rect(VIA1, 0, 0, 100, 100);
    s.gds.rect(VIA1, 200, 0, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_RV_PASS", "DRC_RV_PASS", "redundant_via", 0);

    // FAIL: single isolated via1, count=1 < 2
    s.gds.begin_cell("DRC_RV_FAIL");
    s.gds.rect(VIA1, 0, 0, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_RV_FAIL", "DRC_RV_FAIL", "redundant_via", 1);
}

/// Via array spacing: when vias form an array (group > array_threshold),
/// spacing between adjacent vias must be >= array_spacing.
fn via_array_spacing(s: &mut Suite) {
    // PASS: 2 vias (group size 2 <= threshold 3), no array check triggered
    s.gds.begin_cell("DRC_VAS_PASS");
    s.gds.rect(VIA1, 0, 0, 100, 100);
    s.gds.rect(VIA1, 150, 0, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_VAS_PASS", "DRC_VAS_PASS", "via_array_spacing", 0);

    // FAIL: 4 vias close together (group > 3), spacing 50nm < 200nm
    s.gds.begin_cell("DRC_VAS_FAIL");
    s.gds.rect(VIA1, 0, 0, 100, 100);
    s.gds.rect(VIA1, 150, 0, 100, 100);
    s.gds.rect(VIA1, 300, 0, 100, 100);
    s.gds.rect(VIA1, 450, 0, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_VAS_FAIL", "DRC_VAS_FAIL", "via_array_spacing", 1);
}

/// Max distance to tap: every point in diff must be within max_dist of a tap (li).
fn max_distance_to_tap(s: &mut Suite) {
    // PASS: diff with li tap nearby, all corners within 3000nm
    s.gds.begin_cell("DRC_MDT_PASS");
    s.gds.rect(DIFF, 0, 0, 1000, 1000);
    s.gds.rect(NSDM, -50, -50, 1100, 1100);
    s.gds.rect(LI, 400, 400, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MDT_PASS", "DRC_MDT_PASS", "max_distance_to_tap", 0);

    // FAIL: large diff with tap only at one corner, opposite corner > 3000nm away
    s.gds.begin_cell("DRC_MDT_FAIL");
    s.gds.rect(DIFF, 0, 0, 5000, 5000);
    s.gds.rect(NSDM, -50, -50, 5100, 5100);
    s.gds.rect(LI, 0, 0, 100, 100);
    s.gds.end_cell();
    s.add_drc("DRC_MDT_FAIL", "DRC_MDT_FAIL", "max_distance_to_tap", 1);
}

/// Multi-patterning: shapes within min spacing must be colorable with num_colors colors.
fn multi_patterning(s: &mut Suite) {
    // PASS: 2 shapes within 100nm — always 2-colorable
    s.gds.begin_cell("DRC_MP_PASS");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET1, 280, 0, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MP_PASS", "DRC_MP_PASS", "multi_patterning", 0);

    // FAIL: 3 shapes all within 100nm of each other (triangle), needs 3 colors but only 2
    s.gds.begin_cell("DRC_MP_FAIL");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.rect(MET1, 280, 0, 200, 200);
    s.gds.rect(MET1, 140, 250, 200, 200);
    s.gds.end_cell();
    s.add_drc("DRC_MP_FAIL", "DRC_MP_FAIL", "multi_patterning", 1);
}
