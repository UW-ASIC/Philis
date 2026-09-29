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
