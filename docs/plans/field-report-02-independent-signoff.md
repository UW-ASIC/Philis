# Field report 02 — independent signoff disagrees with Philis (2026-09-29)

From the TinyTapeout_Flows session. GDS: `/home/omare/Documents/Projects/Trial/ResearchBoutros/analog/strongarm/output/pnr/v3_seed5/strongarm.gds`.
Checks: `ResearchBoutros/analog/common/layout` (verify route, `make gen-drc` logic).

| Check | Result | Philis's own signoff |
|---|---|---|
| klayout DRC, `$PDK_ROOT/sky130A/libs.tech/klayout/drc/sky130A_mr.drc` | **264**: li.5 48, via2.1a_a 72, via.5a 40, m2.5 39, via2.5 34, li.6 16, li.3 8, m5.4 7 | "DRC 2 blocking" |
| magic DRC (sky130A tech) | **648** | — |
| netgen LVS (magic extract vs source) | **fails**, 12 nets vs 10: both n-wells float (no taps drawn), the GDS carries no pin labels | MATCH |

For contrast, a substrate2 generator for the same netlist is 0/0 on klayout and magic, and netgen matches.
Reporter's conclusion: Philis stays in their flow for quick placement feedback, but its signoff numbers are no longer trusted.

Open question: which of these is the cause? (a) GPurify deck missing rules (via/via2 enclosure, li.5/li.6, m2.5, m5.4);
(b) the exported GDS differs from the shapes signoff checked; (c) GPurify engine bugs; (d) LVS not treating well
connectivity as a net (floating wells pass) and no labels in the GDS.

## Root cause (investigation 2026-10-01; scripts and outputs in the session scratchpad `signoff-investigation/`)

**Main cause: the reporter's build is old.** EDA-Packaged `philis-0.1.0` (`/nix/store/pk69pf01…`) = Philis e8bc59e
(2026-09-22, 60 commits behind main 1095153), linking GPurify e3c8eb2 (2026-07-25) with the old JSON sky130 rule table.
The investigation reproduced the reporter's numbers exactly: klayout 264, magic 648.
On the same GDS, current GPurify 4ef439d reports every klayout rule family. Current Philis main on strongarm
(8 iterations, seed 5) produces **0 klayout and 0 magic violations**, with n-wells tied to vdd.

| Rule | klayout, reporter GDS | klayout, main GDS | GPurify 4ef439d, reporter GDS | Old deck | Current deck | Cause |
|---|---|---|---|---|---|---|
| li.3 | 8 edge pairs (2 sites) | 0 | 4 (same 2 sites) | present | l.350 | already reported by Philis |
| li.5 / li.6 | 48 / 16 | 0 | 48 / 16 | missing | l.352–353 | old deck |
| via.5a | 40 | 0 | 40 | coded as a different rule | l.389 | old deck |
| m2.5 | 39 | 0 | 39 | missing | l.391 | old deck |
| via2.1a_a | 72 (36 vias, 170 nm) | 0 | 36 | 170 nm, should be 200 nm | l.392 | old deck |
| via2.5 | 34 | 0 | 34 | missing | l.395 | old deck |
| m5.4 | 7 | 0 | 7 | 1.6 µm², should be 4 µm² | l.382 | old deck |
| m1.1 (ota, pwm, wta, ptat) | >0 | 0 | ota: 0 | present | l.361 | **GPurify engine: misses a diagonal neck at overlapping rectangles' inner corner** (two-shape repro: klayout 2, GPurify 0) |

The old generator also drew body ties on diff with both implants and never on tap (65/44), so its n-wells float.
Current GPurify ERC `nwell.4` catches that, and current main draws real taps.

**Still open on main (ranked fixes):**
1. Repackage EDA-Packaged philis from main and put the commit hash in `--version`. Main's CLI currently lacks
   `run`, `-o`, `--seed`, `--max-iters` and `--interface`, so this depends on fix 3.
2. GDS export (`frontend/library/src/gds.rs`):
   - Name the top cell after the subckt, not `TOP`.
   - Write a text label for each signoff pin on the metal's label layer (`<metal>/5`, or `/16` with a pin shape).
     Old builds put labels on 236/0, which sky130 tools ignore.
3. CLI (`frontend/cli/src/main.rs`): restore `run … -o <dir>`, writing the GDS, `signoff.txt`/`.json`, and the LVS
   reference SPICE including dummies, as `examples/xlvs_dump.rs` already does.
4. **netgen LVS on main fails 14 devices against 9.** Philis adds dummy transistors to its own reference but not to the
   schematic, e.g. `drn_n vss vss`. Main's own signoff on that run reported 4 hard `lvs.parameter_mismatch`.
5. GPurify engine: make the width check Euclidean at the inner corners of merged shapes (the m1.1 miss).
6. GPurify LVS:
   - Make the n-well a net and give MOS devices a bulk terminal. Today it reports Match with floating wells;
     deck l.667–672.
   - Accept `X` cards with `W=` and unitless µm values in reference netlists.
7. GPurify deck: scope `capm.4` to `via3.interacting(capm)` and `cap2m.4` to `via4.interacting(cap2m)`. Today they
   flag every via3/via4 with no MIM capacitor.
8. CI cross-check: run klayout `sky130A_mr.drc` plus magic and netgen on a few benchmark GDS files. Tools are in the
   nix store: klayout 0.30.4, magic 8.3.573, netgen 1.5.292.
