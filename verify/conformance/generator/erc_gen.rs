use crate::helper::*;

/// ERC engine not implemented yet — test cells and manifest entries are wired
/// so the conformance runner acknowledges them as pending. When the ERC engine
/// lands, flip these from SKIP to real checks.
pub fn generate(s: &mut Suite) {
    floating_gate(s);
    floating_gate_pass(s);
    floating_well(s);
    floating_well_pass(s);
    missing_tie(s);
    supply_short(s);
    supply_short_pass(s);
    unconnected_pin(s);
    soft_connection(s);
    multiple_drivers(s);
    multiple_drivers_pass(s);
    tie_high_low(s);
    tie_high_low_pass(s);
    antenna_electrical(s);
    esd_check(s);
    hv_domain(s);
    em_check(s);
    p2p_resistance(s);
}

/// Gate poly not connected to any metal/li → floating gate.
fn floating_gate(s: &mut Suite) {
    s.gds.begin_cell("ERC_FG");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    // source/drain li contacts present, but NO li on gate poly
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();
    s.add_erc("ERC_FLOAT_GATE", "ERC_FG", "floating_gate", 1);
}

/// Clean gate: poly connected to li → not floating.
fn floating_gate_pass(s: &mut Suite) {
    s.gds.begin_cell("ERC_FG_OK");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    // li contact ON the gate poly → gate driven
    s.gds.rect(LI, 200, -50, 50, 50);
    s.gds.end_cell();
    s.add_erc("ERC_FLOAT_GATE_PASS", "ERC_FG_OK", "floating_gate", 0);
}

/// nwell with PMOS inside but no well tap contact.
fn floating_well(s: &mut Suite) {
    s.gds.begin_cell("ERC_FW");
    s.gds.rect(NWELL, 0, 0, 600, 400);
    s.gds.rect(DIFF, 50, 100, 500, 200);
    s.gds.rect(PSDM, 0, 50, 600, 300);
    s.gds.rect(POLY, 250, 50, 50, 300);
    s.gds.rect(LI, 100, 150, 100, 100);
    s.gds.rect(LI, 400, 150, 100, 100);
    // no tap to tie well to supply
    s.gds.end_cell();
    s.add_erc("ERC_FLOAT_WELL", "ERC_FW", "floating_well", 1);
}

/// Clean well: nwell with n+ tap (nsdm + diff inside) → not floating.
fn floating_well_pass(s: &mut Suite) {
    s.gds.begin_cell("ERC_FW_OK");
    s.gds.rect(NWELL, 0, 0, 600, 400);
    s.gds.rect(DIFF, 50, 100, 500, 200);
    s.gds.rect(PSDM, 0, 50, 600, 300);
    s.gds.rect(POLY, 250, 50, 50, 300);
    s.gds.rect(LI, 100, 150, 100, 100);
    s.gds.rect(LI, 400, 150, 100, 100);
    // n+ tap: diff + nsdm inside the nwell
    s.gds.rect(DIFF, 50, 310, 100, 80);
    s.gds.rect(NSDM, 0, 300, 200, 100);
    s.gds.end_cell();
    s.add_erc("ERC_FLOAT_WELL_PASS", "ERC_FW_OK", "floating_well", 0);
}

/// Active diffusion far from any substrate/well tap.
fn missing_tie(s: &mut Suite) {
    s.gds.begin_cell("ERC_MT");
    // large diff region, tap contacts only at far corner
    s.gds.rect(DIFF, 0, 0, 5000, 5000);
    s.gds.rect(NSDM, -50, -50, 5100, 5100);
    s.gds.rect(LI, 4800, 4800, 100, 100); // tap only at far corner
    s.gds.end_cell();
    s.add_erc("ERC_MISSING_TIE", "ERC_MT", "missing_tie", 1);
}

/// Inverter with NMOS source and PMOS source shorted → VDD/VSS short.
fn supply_short(s: &mut Suite) {
    s.gds.begin_cell("ERC_SHORT");
    // NMOS
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    // PMOS
    s.gds.rect(DIFF, 0, 500, 500, 200);
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 800);
    // source contacts SHORTED by one tall li bar
    s.gds.rect(LI, 50, 50, 100, 600); // connects both sources
    s.gds.rect(LI, 350, 50, 100, 600); // output
    s.gds.end_cell();
    s.add_erc("ERC_VDD_VSS_SHORT", "ERC_SHORT", "supply_short", 1);
}

/// Clean inverter: sources on separate nets → no supply short.
fn supply_short_pass(s: &mut Suite) {
    // Reuse LVS_INV cell (clean inverter with separate source contacts)
    s.add_erc("ERC_VDD_VSS_PASS", "LVS_INV", "supply_short", 0);
}

/// Metal stub not connected to any active device terminal.
fn unconnected_pin(s: &mut Suite) {
    s.gds.begin_cell("ERC_UP");
    // isolated met1 rectangle — dangling pin
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_erc("ERC_UNCONNECTED", "ERC_UP", "unconnected_pin", 1);
}

/// Multiple drivers: two NMOS devices with different gates driving the same drain net.
fn multiple_drivers(s: &mut Suite) {
    s.gds.begin_cell("ERC_MD");
    // Two parallel NMOS transistors with DIFFERENT gates, drains shorted via li
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 400, 500, 200);
    s.gds.rect(NSDM, -50, 350, 600, 300);
    // separate gates
    s.gds.rect(POLY, 200, -50, 50, 300);   // gate A
    s.gds.rect(POLY, 200, 350, 50, 300);   // gate B (different net)
    // separate source contacts
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 50, 450, 100, 100);
    // SHARED drain: one tall li bar connecting both drains
    s.gds.rect(LI, 350, 50, 100, 500);
    s.gds.end_cell();
    s.add_erc("ERC_MULTI_DRIVER", "ERC_MD", "multiple_drivers", 1);
}

/// Clean: single driver on output net (standard inverter).
fn multiple_drivers_pass(s: &mut Suite) {
    // LVS_INV is a clean inverter — N and P share gate, so drain net has one driving signal
    s.add_erc("ERC_MULTI_DRIVER_PASS", "LVS_INV", "multiple_drivers", 0);
}

/// Tie-high/low: gate shorted to a supply rail (S/D-only net, no output drives it).
/// Build a CMOS pair where gate li accidentally bridges to the PMOS source li,
/// merging the gate net with the VDD rail.
fn tie_high_low(s: &mut Suite) {
    s.gds.begin_cell("ERC_THL");
    // Standard CMOS: NMOS at bottom, PMOS at top
    s.gds.rect(DIFF, 0, 0, 500, 200);       // NMOS diff
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(DIFF, 0, 500, 500, 200);      // PMOS diff
    s.gds.rect(PSDM, -50, 450, 600, 300);
    s.gds.rect(NWELL, -50, 450, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 800);     // shared gate
    // Source contacts
    s.gds.rect(LI, 50, 50, 100, 100);        // NMOS source (VSS)
    s.gds.rect(LI, 50, 550, 100, 100);       // PMOS source (VDD)
    // Li bridge from gate poly to PMOS source: spans both x regions
    // Gate poly: x=[200,250]. PMOS source segment: x=[0,200], y=[500,700].
    // Bridge li overlaps both.
    s.gds.rect(LI, 150, 500, 90, 100);       // x=[150,240] overlaps source x=[0,200] AND poly x=[200,250], NOT drain x=[250,500]
    // Drain contacts
    s.gds.rect(LI, 350, 50, 100, 600);       // output Y
    s.gds.end_cell();
    s.add_erc("ERC_TIE_HL", "ERC_THL", "tie_high_low", 1);
}

/// Clean: gate properly driven (not shorted to supply).
fn tie_high_low_pass(s: &mut Suite) {
    s.add_erc("ERC_TIE_HL_PASS", "LVS_INV", "tie_high_low", 0);
}

/// Cumulative antenna: metal area across ALL layers / gate area.
/// Hardcoded ratio limit = 200. Gate area = poly × diff overlap.
fn antenna_electrical(s: &mut Suite) {
    // PASS: small metal on both met1 and met2, total ratio < 200
    // Gate: 50×200 = 10000. Met1: 200×200 = 40000. Met2: 200×200 = 40000.
    // Total metal area = 80000. Ratio = 8.0 < 200.
    s.gds.begin_cell("ERC_AE_PASS");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 210, 210, 30, 30);
    s.gds.rect(MCON, 215, 215, 20, 20);
    s.gds.rect(MET1, 200, 200, 200, 200);
    s.gds.rect(VIA1, 300, 300, 50, 50);
    s.gds.rect(MET2, 200, 200, 200, 200);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();
    s.add_erc("ERC_ANTENNA_ELEC_PASS", "ERC_AE_PASS", "antenna_electrical", 0);

    // FAIL: huge metal on met1 + met2, cumulative ratio > 200
    // Gate area = 10000. Met1: 5000×5000=25M. Met2: 5000×5000=25M.
    // Total = 50M. Ratio = 5000 > 200.
    s.gds.begin_cell("ERC_AE_FAIL");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 210, 210, 30, 30);
    s.gds.rect(MCON, 215, 215, 20, 20);
    s.gds.rect(MET1, 200, 200, 5000, 5000);
    s.gds.rect(VIA1, 300, 300, 50, 50);
    s.gds.rect(MET2, 200, 200, 5000, 5000);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 350, 50, 100, 100);
    s.gds.end_cell();
    s.add_erc("ERC_ANTENNA_ELEC_FAIL", "ERC_AE_FAIL", "antenna_electrical", 1);
}

/// Net connected only through high-resistance well path.
fn soft_connection(s: &mut Suite) {
    s.gds.begin_cell("ERC_SOFT");
    // two li pads connected only through nwell (high-R path)
    s.gds.rect(NWELL, 0, 0, 1000, 200);
    s.gds.rect(LI, 50, 50, 100, 100);
    s.gds.rect(LI, 850, 50, 100, 100);
    // no metal strap between them — only well conduction
    s.gds.end_cell();
    s.add_erc("ERC_SOFT_CONN", "ERC_SOFT", "soft_connection", 1);
}

/// ESD missing: conductor touching cell boundary with no device on that net.
fn esd_check(s: &mut Suite) {
    // FAIL: met1 rect touching origin edge, no device
    s.gds.begin_cell("ERC_ESD");
    s.gds.rect(MET1, 0, 0, 200, 200);
    s.gds.end_cell();
    s.add_erc("ERC_ESD_MISSING", "ERC_ESD", "esd_missing", 1);

    // PASS: clean inverter — has devices on all nets
    s.add_erc("ERC_ESD_PASS", "LVS_INV", "esd_missing", 0);
}

/// HV domain crossing: conductor spans both inside and outside nwell, not on a device net.
fn hv_domain(s: &mut Suite) {
    // FAIL: met1 conductor extends past nwell boundary (no device on this net)
    s.gds.begin_cell("ERC_HV");
    s.gds.rect(NWELL, 0, 0, 500, 500);
    s.gds.rect(MET1, 200, 200, 600, 100); // extends past nwell at x=500
    s.gds.end_cell();
    s.add_erc("ERC_HV_CROSS", "ERC_HV", "hv_domain_crossing", 1);

    // PASS: clean inverter — conductors stay in their domains
    s.add_erc("ERC_HV_PASS", "LVS_INV", "hv_domain_crossing", 0);
}

/// EM current density: narrow met1 wire on a device S/D net.
fn em_check(s: &mut Suite) {
    // FAIL: NMOS with narrow 80nm met1 output wire on drain net
    s.gds.begin_cell("ERC_EM");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 50, 50, 100, 100);     // source contact
    s.gds.rect(LI, 350, 50, 100, 100);    // drain contact
    s.gds.rect(MCON, 360, 60, 80, 80);
    s.gds.rect(MET1, 350, 50, 80, 500);   // narrow 80nm met1 on drain net
    s.gds.end_cell();
    s.add_erc("ERC_EM_FAIL", "ERC_EM", "em_current_density", 1);

    // PASS: clean inverter — no met1 on device nets (li-only wiring)
    s.add_erc("ERC_EM_PASS", "LVS_INV", "em_current_density", 0);
}

/// Point-to-point resistance: very long narrow met1 wire on a device net (high R > 10 ohm).
fn p2p_resistance(s: &mut Suite) {
    // FAIL: NMOS with 20000nm long, 100nm wide met1 = 200 squares = 20 ohm > 10
    s.gds.begin_cell("ERC_P2P");
    s.gds.rect(DIFF, 0, 0, 500, 200);
    s.gds.rect(NSDM, -50, -50, 600, 300);
    s.gds.rect(POLY, 200, -50, 50, 300);
    s.gds.rect(LI, 50, 50, 100, 100);     // source contact
    s.gds.rect(LI, 350, 50, 100, 100);    // drain contact
    s.gds.rect(MCON, 360, 60, 80, 80);
    s.gds.rect(MET1, 350, 50, 20000, 100); // 200 squares = 20 ohm > 10
    s.gds.end_cell();
    s.add_erc("ERC_P2P_FAIL", "ERC_P2P", "p2p_resistance", 1);

    // PASS: clean inverter — no met1, no R
    s.add_erc("ERC_P2P_PASS", "LVS_INV", "p2p_resistance", 0);
}
