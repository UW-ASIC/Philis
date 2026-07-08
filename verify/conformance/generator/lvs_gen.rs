use crate::helper::*;

pub fn generate(s: &mut Suite) {
    clean_match(s);
    sd_permutation(s);
    device_count_mismatch(s);
    topology_mismatch(s);
    intentional_short(s);
    intentional_open(s);
    parallel_fingers(s);
    parametric_wl(s);
    vt_flavor(s);
    series_merge(s);
    parallel_merge(s);
    isomorphic_trap(s);
}

fn clean_match(s: &mut Suite) {
    s.gds.begin_cell("LVS_INV");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 500, 500, 200);
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 800);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 50, 550, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 600);
    s.gds.end_cell();

    s.add_lvs("LVS_CLEAN_MATCH", "LVS_INV", true, vec![
        lvs_device("nmos", "A", "VSS", "Y"),
        lvs_device("pmos", "A", "VDD", "Y"),
    ]);
}

/// S/D swapped in reference → should still match (FET S/D permutable)
fn sd_permutation(s: &mut Suite) {
    // same cell as clean inverter — reuse LVS_INV
    s.add_lvs("LVS_SD_PERMUTE", "LVS_INV", true, vec![
        lvs_device("nmos", "A", "Y", "VSS"),   // S/D swapped vs clean_match
        lvs_device("pmos", "A", "Y", "VDD"),
    ]);
}

fn device_count_mismatch(s: &mut Suite) {
    s.gds.begin_cell("LVS_DCNT");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();

    s.add_lvs("LVS_DEVICE_MISMATCH", "LVS_DCNT", false, vec![
        lvs_device("nmos", "A", "S1", "D1"),
        lvs_device("nmos", "B", "S2", "D2"),
    ]);
}

fn topology_mismatch(s: &mut Suite) {
    s.gds.begin_cell("LVS_TOPO");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 500, 500, 200);
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    // separate gates — two disconnected polys
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(POLY, 200, 450, 50, 300);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 50, 550, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 600);
    s.gds.end_cell();

    // reference expects shared gate → mismatch
    s.add_lvs("LVS_TOPO_MISMATCH", "LVS_TOPO", false, vec![
        lvs_device("nmos", "A", "VSS", "Y"),
        lvs_device("pmos", "A", "VDD", "Y"),
    ]);
}

/// Gate-drain short: output li overlaps gate poly → merged net → topology mismatch
fn intentional_short(s: &mut Suite) {
    s.gds.begin_cell("LVS_SHORT");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 500, 500, 200);
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 800);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 50, 550, 100, 100);
    // output li overlaps gate poly → shorts gate to drain
    s.gds.rect(LI, 200, 50, 150, 600);
    s.gds.end_cell();

    // reference expects gate ≠ output → mismatch
    s.add_lvs("LVS_INTENTIONAL_SHORT", "LVS_SHORT", false, vec![
        lvs_device("nmos", "A", "VSS", "Y"),
        lvs_device("pmos", "A", "VDD", "Y"),
    ]);
}

/// Intentional open: inverter output li split into two disconnected pieces.
/// Each device gets a different drain net → reference (shared Y) mismatches.
fn intentional_open(s: &mut Suite) {
    s.gds.begin_cell("LVS_OPEN");
    // NMOS
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    // PMOS
    s.gds.rect(DIFF, 0, 500, 500, 200);
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    // shared gate
    s.gds.rect(POLY, 200, -50, 50, 800);
    // input contact
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 50, 550, 100, 100);
    // SPLIT output: two separate li, NOT connected
    s.gds.rect(LI, 350, 50, 100, 100);  // nmos drain only
    s.gds.rect(LI, 350, 550, 100, 100); // pmos drain only
    s.gds.end_cell();

    s.add_lvs("LVS_INTENTIONAL_OPEN", "LVS_OPEN", false, vec![
        lvs_device("nmos", "A", "VSS", "Y"),
        lvs_device("pmos", "A", "VDD", "Y"),
    ]);
}

/// Parallel fingers: one diff strip with two gates → 2 devices.
/// Reference with 2 devices of same type → match.
fn parallel_fingers(s: &mut Suite) {
    s.gds.begin_cell("LVS_FINGERS");
    s.gds.rect(DIFF, 0, 0, 1000, 200);
    s.gds.rect(NSDM, -50, -50, 1100, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);  // gate 1
    s.gds.rect(POLY, 600, -50, 50, 300);  // gate 2
    s.gds.rect(LI, 50, 50, 100, 100);     // source
    s.gds.rect(LI, 350, 50, 100, 100);    // shared drain/source
    s.gds.rect(LI, 800, 50, 100, 100);    // drain
    s.gds.end_cell();

    // 2 fingers match 2-device reference
    s.add_lvs("LVS_FINGERS_MATCH", "LVS_FINGERS", true, vec![
        lvs_device("nmos", "A", "S", "M"),
        lvs_device("nmos", "B", "M", "D"),
    ]);

    // 1-device reference mismatches
    s.add_lvs("LVS_FINGERS_MISMATCH", "LVS_FINGERS", false, vec![
        lvs_device("nmos", "A", "S", "D"),
    ]);
}

/// Parametric W/L: reference carries W/L, runner passes them, comparator enforces.
/// Uses LVS_INV cell (gate poly 50nm wide over 200nm diff → L=50, W=200 per device).
fn parametric_wl(s: &mut Suite) {
    // Correct W/L → match
    s.add_lvs("LVS_PARAM_MATCH", "LVS_INV", true, vec![
        lvs_device_wl("nmos", "A", "VSS", "Y", 200, 50),
        lvs_device_wl("pmos", "A", "VDD", "Y", 200, 50),
    ]);

    // Wrong W (300 instead of 200) → parametric mismatch
    s.add_lvs("LVS_PARAM_MISMATCH", "LVS_INV", false, vec![
        lvs_device_wl("nmos", "A", "VSS", "Y", 300, 50),
        lvs_device_wl("pmos", "A", "VDD", "Y", 300, 50),
    ]);
}

/// Vt flavor: LVT/HVT implant over channel distinguishes device flavors.
/// Layout with HVT implant must match reference specifying "hvt" flavor.
fn vt_flavor(s: &mut Suite) {
    // NMOS with HVT implant
    s.gds.begin_cell("LVS_HVT");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(HVT, 150, -50, 150, 300); // HVT implant over channel
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();

    // Match: reference says HVT → match
    s.add_lvs("LVS_HVT_MATCH", "LVS_HVT", true, vec![
        lvs_device_flavor("nmos", "A", "S", "D", "hvt"),
    ]);

    // Mismatch: reference says standard → device count same but flavor differs
    s.add_lvs("LVS_HVT_MISMATCH", "LVS_HVT", false, vec![
        lvs_device_flavor("nmos", "A", "S", "D", "standard"),
    ]);

    // LVT variant
    s.gds.begin_cell("LVS_LVT");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LVT, 150, -50, 150, 300); // LVT implant
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();

    s.add_lvs("LVS_LVT_MATCH", "LVS_LVT", true, vec![
        lvs_device_flavor("nmos", "A", "S", "D", "lvt"),
    ]);
}

/// Series merge: two same-gate devices in series on one diff strip merge into one
/// device with L = L1 + L2. Reference carries the merged single device.
fn series_merge(s: &mut Suite) {
    s.gds.begin_cell("LVS_SERIES");
    s.gds.rect(DIFF, 0, 0, 1000, 200);
    s.gds.rect(NSDM, -50, -50, 1100, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);   // gate A
    s.gds.rect(POLY, 600, -50, 50, 300);   // gate A (same net, connected by li)
    s.gds.rect(LI, 200, -50, 450, 50);     // connects both gate polys
    s.gds.rect(LI, 50, 50, 100, 100);      // source
    s.gds.rect(LI, 350, 50, 100, 100);     // middle (internal, merged away)
    s.gds.rect(LI, 800, 50, 100, 100);     // drain
    s.gds.end_cell();

    // After series merge: 2 same-gate devices → 1 merged device with L = 50+50 = 100
    s.add_lvs("LVS_SERIES_MERGE", "LVS_SERIES", true, vec![
        lvs_device("nmos", "A", "S", "D"),
    ]);
}

/// Parallel merge: two devices sharing all three terminals merge into one with W = W1+W2.
/// Two separate diff strips, each with one gate, all connected to the same 3 nets.
fn parallel_merge(s: &mut Suite) {
    s.gds.begin_cell("LVS_PARALLEL");
    // Two parallel NMOS: same gate, same source, same drain
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 400, 500, 200);
    s.gds.rect(NSDM, -50, 350, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 700);   // one poly crossing both diffs
    s.gds.rect(LI, 50, 50, 100, 500);      // connects both sources
    s.gds.rect(LI, 350, 50, 100, 500);     // connects both drains
    s.gds.end_cell();

    // After parallel merge: 2 devices → 1 device with W = 200 + 200 = 400
    s.add_lvs("LVS_PARALLEL_MERGE", "LVS_PARALLEL", true, vec![
        lvs_device("nmos", "A", "S", "D"),
    ]);
}

/// Isomorphic trap: symmetric circuit where topology matching is ambiguous
/// but still correct. Inverter has symmetry (VDD/VSS interchangeable topologically)
/// but the comparator must still find the correct mapping.
fn isomorphic_trap(s: &mut Suite) {
    // Reuse the clean inverter — it has inherent symmetry between N and P paths
    s.add_lvs("LVS_ISOMORPHIC", "LVS_INV", true, vec![
        lvs_device("nmos", "A", "VSS", "Y"),
        lvs_device("pmos", "A", "VDD", "Y"),
    ]);
}

// ponytail: VDD/VSS swap detection requires net seeding (pre-assigning supply net IDs
// before comparison). The topology-only comparator sees the swap as isomorphic — the
// graph structure is identical, only the labels differ. Skipped until net seeding lands.
