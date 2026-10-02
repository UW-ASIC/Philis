# Field report 01 — TinyTapeout_Flows / ResearchBoutros (2026-09-29)

Reported by the TinyTapeout_Flows session after running Philis (EDA-Packaged build,
`philis run <deck> share/philis/pdks/sky130.json --interface ... --seed N`) on ~10 sky130
analog blocks. Repro decks and outputs: `/home/omare/Documents/Projects/Trial/ResearchBoutros/analog/<block>/`
(`netlist/<block>.spice`, `layout/interface.json`, `output/pnr/*`). Ranked by the reporter's impact.
Mapped at the M0 close-out: 00-MASTER-PLAN.md Status, field-report table (each item re-run on m0; the
reporter's build was Philis `e8bc59e` with GPurify `e3c8eb2`). Each owning item carries a `Field report: FR-n` line.

1. **Hang with several MIM caps.** On a deck with several `cap_mim_m3_1`, the first "extracting
   feedback" never finishes (>8 h, 1 thread, ~10 MB RSS). async_ctrl tq_chain: 0–1 cap finishes in
   ~5 min; 8 caps hang. The 1-cap run also reported LVS "device class 22 has 1 in layout vs 0 in
   reference". Repro: `analog/async_ctrl/output/pnr/tq_chain*.spice`.
2. **Runtime.** ~45–60 min per iteration, almost all in "extracting feedback", for 30–90 FETs
   (wta, pwm_driver, async_ctrl). A 100-iteration run never stops early even with DRC 0 and LVS
   match throughout (wta: 12.7 h in feedback). pwm_driver 36220 s / 34576 s per seed. strongarm
   (9 FETs) ~23 min per 100 iterations. Workaround: `--max-iters 4–8`.
3. **False LVS mismatch.** ptat_bias: "topology mismatch: device class 109 has 1 in layout vs 0 in
   reference" (other seed: "class 126 has 0 in layout vs 1"), while `extracted_pex.spice` matches the
   source device-for-device and net-for-net. pwm_driver seed 1: same with "device class 24".
4. **Generator DRC.** `poly min_extension` (−250 < 130 nm) on long-L PFETs (rescale: four
   L = 9.6 µm pfets) and on W = 0.42 µm devices (ptat_bias). Residual `LI.3` li spacing
   (153 < 170 nm) on most runs.
5. **Long-L cell footprint.** A 0.42/64.8 µm PFET becomes a ~196 µm cell ("interface die
   80000x60000 too small: cell `Xmpu` footprint exceeds the die"); width looks like ~3·L.
6. **Short.** A 7-device PTAT core shorted vb_tail to nl: "device count mismatch (ext 2N/2P vs ref 3N/3P)".
7. **`extracted_pex.spice` is not simulatable** (reporter wrote `analog/common/pex.py` to rebuild it):
   no `.subckt`; generic `nmos`/`pmos` model names; R/C/diodes omitted; parasitics only as `* net`
   comments, not R/C elements; **NMOS bulks reported on `vdd`**; drain/source freely swapped; parallel
   FETs merged into one summed-W device (undocumented); without `--interface`, ports come out as `n<id>`.
8. **Packaging.** Only sky130.json and generic_finfet.json ship; gf180/ihp exist only in
   new_skeleton with a different schema.

Works well: `--interface` naming, clean LVS on most blocks, advanced 6/6 on strongarm/ota.
