# Audit 05 — Routing (`backend/gr`, `backend/dr`, their driver, and the routing rules that score them)

Auditor scope: global routing (`gr`), detailed routing (`dr`), how `frontend/library` configures and calls them each epoch, and how `kernel/analog/src/routing/*` scores their output. Read line by line on the current working tree (uncommitted changes included). Finding IDs: `AT-NN`. Code citations are `path:line` relative to the repo root. Reference citations give the book or paper, the section, figure or equation, and the line range in the extracted `reftext/*.txt`, plus the PDF page computed by form-feed count (`p = 1 + count of \f before the line`).

Numbers marked **derived** were computed by hand from deck rule text and the code's formulas. They were not produced by running the tool. No cargo command was run for this audit.

---

## 0. Scope

### 0.1 Files read completely

| File | Lines |
|---|---|
| backend/gr/src/lib.rs | 1393 |
| backend/gr/Cargo.toml | 8 |
| backend/dr/src/lib.rs | 2722 |
| backend/dr/Cargo.toml | 10 |
| frontend/library/src/lib.rs (the driver) | 1439 |
| frontend/library/src/elaborate.rs (router configuration from the deck: `routing_stack`, `detailed_router`, `em_limits`, `stack`, `antenna_diodes`; the router-only `route_built` loop) | 460 |
| kernel/analog/src/routing/mod.rs | 25 |
| kernel/analog/src/routing/antenna.rs | 67 |
| kernel/analog/src/routing/common_node.rs | 123 |
| kernel/analog/src/routing/coupling.rs | 192 |
| kernel/analog/src/routing/crosstalk.rs | 115 |
| kernel/analog/src/routing/differential.rs | 198 |
| kernel/analog/src/routing/em.rs | 243 |
| kernel/analog/src/routing/ir.rs | 94 |
| kernel/analog/src/routing/parasitic.rs | 126 |
| kernel/analog/src/routing/performance.rs | 95 |
| kernel/analog/src/routing/shield.rs | 134 |
| kernel/analog/src/routing/stack.rs | 532 |
| kernel/core/src/routes.rs | 83 |

### 0.2 Supporting excerpts read (not the audited subsystem)

- kernel/core/src/macro.rs:1–66 (`place_macro`)
- kernel/analog/src/rule.rs:215–320 (`RuleBatch for Vec<R>`, `kind` = `type_name`, `over`)
- backend/verify/src/pdk.rs:337–437, 439–510, 393–406, 525–547, 698–737 (routing layers, vias, pads, pitch, spacing, EM, C per µm)
- frontend/library/src/oppoint.rs:39–99 (`terminal_ua`, `net_current_ua`)
- frontend/library/src/cellgen.rs:335–375 (`price` → `gr::price_group`) and 937–966 (`bind_pins`)
- kernel/cells/src/mosfet.rs:455–482 (one S/D pin per diffusion region); kernel/cells/src/builder.rs:217–220
- backend/annotator/src/extract.rs:1–120 (which routing rules the router receives)
- benchmarks/tests/signoff_fixtures.rs:1–41 and 150–168; frontend/library/tests/ota_cross_pdk.rs:190–230
- docs/CRATES.md:45–117
- The GPurify decks at the revision `Cargo.lock` pins (8df8c09): `sky130.deck` 340–420, `gf180mcu.deck` 385–440, `ihp_sg13g2.deck` 360–440

### 0.3 Reference text read

- Graeb (ed.) *Analog Layout Synthesis*, ch. 4 "Routing Analog Circuits" (Dündar and Unutulmaz), `balasa_graeb_survey.txt` 8150–10420 (the whole chapter, pp. 162–209)
- Lampaert *et al.* ch. 5 "Routing", `lampaert.txt` 5420–5640 and 6060–6380 (pp. 131–152)
- Lienig and Thiele, `lienig_em.txt` 3450–3600, 4020–4145, 4540–4705, 6114–6290 (§3.2, §3.3, §3.5.2–3.5.3, §4.6)
- Hastings 3e, `hastings.txt` 13114–13250 (§5.1.6 antenna), 41750–41835 (metal over matched MOS), 48068–48240 (§15.4.1–15.4.2 vias and pitch), 48502–48800 (§15.4.4 special routing techniques)
- Li *et al.*, "Performance-driven Wire Sizing" (TODAES'22), `todaes22.txt` 1–60, 108–125, 180–216, 575–585, 624–645
- Performance-driven analog layout survey, `perf_driven_survey.txt` 60–175
- Karmokar *et al.* (CC DAC P&R), `cc_dac_constructive.txt` 1–30 and 770–830; CC review, `cc_review.txt` 374–380 and 436–456
- Charbon *et al.*, `charbon_substrate.txt` 2030–2135 and 2600–2640

---

## 1. Architecture and data flow

### 1.1 Where routing sits in an epoch

`Flow::epoch` (frontend/library/src/lib.rs:561–709) runs place → rings → route → signoff every epoch. There are up to `feedback_iters`=200 × `outer_iters`=4 epochs per start (lib.rs:65–78, 347–374), `starts`=3 parallel starts, and up to two cell topologies (lib.rs:174–189). The route step is:

1. `gr::GlobalRoute { net_weight, .. }.route(&layout, &macros, &rings, routing, layers, neg)` → `(Routes, Report)`. **The `Report` is discarded** (`let (global, _)`, lib.rs:617–618).
2. `dr::DetailedRoute { cfg: DetailedCfg { common: common_nodes(layout), stack, ..d_router.cfg } }.route(&global, &pins, &placed, &rings, routing, layers, cuts, neg)` (lib.rs:626–631).
3. If any `Antenna` hard rule is still violated, `elaborate::antenna_diodes` places one deck diode per net with `dr::place_near`. `dr.route` then runs **a second time from scratch** with the diodes added to `rings`, and each diode's marker shape is pushed into its net's wires (lib.rs:632–652; elaborate.rs:330–373).
4. The route report enters the epoch key as `(rv, rt)` in `lex_key` (lib.rs:941–958).

`gr::Negotiation` (the PathFinder history) is created once per `solve` (lib.rs:342) and threaded through every epoch's `gr` and `dr` calls.

The router configuration is fixed once per `solve` from the deck (lib.rs:279–318, elaborate.rs:210–240 and 383–460):

- **Stack.** `routing_stack` takes the deck's routing metals up to the first one whose `min_width` exceeds the first via's pad (elaborate.rs:211–216). It then always splits the bottom metal and its cut off as `pin_access` (elaborate.rs:219).
- **Wire width.** `wire_width = max(first-via pad, max min_width over the stack)` (elaborate.rs:398).
- **Pitch.** One pitch for all layers: `pitch = max(wire, largest via pad) + worst route_spacing` (elaborate.rs:393–399; verify/pdk.rs:430–433).
- **Fatten caps.** `fat_signal = 2·wire`, and `fat_supply` = just under the stack's smallest wide-metal threshold, unless the deck gives `route_*_width` (elaborate.rs:429–435).
- **Electrical costs.** `layer_c`, `beside_c`, `layer_r` and `via_r` are normalised to the cheapest layer (elaborate.rs:436–451).
- **EM limits.** Deck EM limits, derated to the op-point temperature (elaborate.rs:248–262).
- **Net lists and currents.** `supply_nets` come from the Supply and Ground classes. `pin_ua` comes from op-point terminal currents. `net_weight` is `gp::net_weights`, restricted to nets with a C budget or a perf sensitivity and normalised to [0,1] (lib.rs:304–316).

**Derived stacks at the pinned deck revision** (formula: verify/pdk.rs:345–366 and 393–406; elaborate.rs:210–219, 376–378, 393–399):

| Deck | First-via pad (access pad) | Metals kept by `take_while` | After the `pin_access` split: routed layers | `wire_width` vs min width of routed metals |
|---|---|---|---|---|
| sky130 | mcon 170 + 2·60 = **290** nm (met1 min area 0.083 µm² gives a 289 side, rounded up to 290) | li, met1, met2 (met3 `m3.1` = 300 > 290 stops it) | **met1, met2 only** (li is pin access) | 290 vs 140 (**2.07×**) |
| gf180mcu | via1 260 + 2·10 = 280, raised to **380** by the 0.1444 µm² min area | metal1–metal5 (metaltop 440 > 380) | **metal2–metal5** (metal1 becomes pin access) | 380 vs 280 (1.36×) |
| ihp_sg13g2 | via1 190 + 2·50 = 290; metal2 min area 0.144 µm² gives **380** | metal1–metal5 | **metal2–metal5** (metal1 becomes pin access) | 380 vs 200 (1.9×) |

For sky130 the via (met1–met2) pad is 150 + 2·85 = 320 (via.1a, via.5a, m2.5), so the **pitch is 320 + 140 = 460 nm on both layers** (derived; the deck has no EOL rule, sky130.deck:356–395). The unit tests use a "sky130-like" 430 nm pitch (dr/src/lib.rs:2142–2144), not the flow's value.

### 1.2 `gr` — what is actually implemented

- **Graph.** `GcellGrid`: a single 2-D layer of N×N gcells (default N=16) over the rectangle **from the origin (0,0)** to the far corner of every bbox and pin (`die_extent`, gr/src/lib.rs:361–368). Every gcell has capacity 6 (gr:31–35). Edges have unit cost to the 4 neighbours (gr:487–503). Congestion is per node, not per edge.
- **Nets.** `build_nets` (gr:412–427) groups pins by net id. It keeps pin centres, deduplicates them, and routes only nets with ≥2 distinct points.
- **Order.** `order_by_priority` (gr:250–271) sorts by (is a shield reference → last; tier: `Differential` < other hard < budget < free; weight descending; pin count ascending; index).
- **Search.** `route_net` (gr:882–1010) builds a Steiner-like tree. The targets are sorted by Manhattan distance from the first terminal. Each target is then reached by a **multi-source Dijkstra from every node already in the tree** (no A* predictor). Node cost (gr:936–941) is `hist + p_fac·max(0, usage_eff+1−cap) + penalty + parasitic + KEEPOUT_COST(if over a foreign matched cell)`. The electrical term (`Elec`, gr:855–872, 908–935) is `weight·layer_c[l] + beside_c[l]·Σ_{occupied adjacent tracks}(weight + their sens) + current·layer_r[l]`, plus `current·via_r` on a layer change. In `gr` itself `layer_c = [1.0]` and `beside_c` is empty (gr:190–192), so only `weight` per gcell is priced.
- **Negotiation.** `run_pathfinder` (gr:1019–1043) is a PathFinder-style negotiated rip-up and reroute. Each iteration reroutes the "dirty" nets (no tree, or a node over capacity) in order, then `hist += hist_inc·overuse` (gr:1046–1054). p_fac is **constant** (3.0 in `gr`), history is additive, and there are ≤ 40 iterations. The run stops at zero overflow, on an iteration with no reroute, or at `max_iters`. The first 8 iterations restrict each search to `corridors`; `gr` has none (gr:1012–1013, 1032).
- **Keep-apart.** One `keep_apart` post-pass (gr:307–338) reroutes each net that shares gcells with a keep-away partner. The shared gcells are priced at `p_fac`, and the result is kept only if sharing drops without new overflow.
- **Rings and history.** Ring shapes on routing layers are charged one usage unit, capped at cap−1 (gr:198–202, 348–358). History is seeded from, and folded into, `Negotiation` keyed by `(tier, layer, x/500 nm, y/500 nm)` with `max` folding (gr:37–92).
- **Output.** A degenerate centre-line `Shape` (w or h = 1) per gcell step on `layers[0]` (gr:207–219, 371–374). The `Report` holds the analog tiers evaluated on this coarse geometry plus overflow/capacity (gr:379–408), and the flow discards it.
- **`price_group`.** Separately, `price_group` (gr:108–133) routes one macro's own pin nets on a 16×16 grid over its bbox, for `cellgen::price` (cellgen.rs:360–374).

### 1.3 `dr` — what is actually implemented

`DetailedRoute::route` (dr/src/lib.rs:183–886) runs these phases.

1. **Frame and lattice.** A routing frame is shifted so that geometry reaching negative coordinates fits, with a 2-pitch halo plus a 3-pitch margin (dr:209–219, 237–247). `TrackGrid`: `n_layers = min(|layers|, |cuts|+1)` layers (dr:207). Every node has capacity 1. The pitch is uniform, raised to `max(die)/1200` on big dies (gr:527–532). **Even layers run horizontal and odd layers vertical, with no wrong-way edges** (gr:630–638). A via is a node-to-node edge with cost `VIA_COST`=4 (dr:26).
2. **Pin reservation.** Each pin reserves the layer-0 nodes within `reach` of its centre. A node in reach of two nets becomes `CONTESTED`, which no trunk may use (dr:335–358).
3. **Landing.** Pins are processed in first-seen order of `(pins ∪ placed pins ∪ ring pins)` (dr:196–204, 360–431). The candidate search walks Chebyshev rings (≤8 rings, ≤12 candidates) on layers 0–1 only, skipping stacked layer-1 nodes (gr:574–611). The first choice is a joint node + L-jog choice {full, narrow} × {flip} that clears foreign stitch zones and laid jogs. The fallbacks are clean (radius 2), then short-free (radius 3), then any own/free node (radius 4). A pin with no node is counted as `unlanded`. `claim_jog_sweep` reserves the jog's nodes (dr:1110–1149).
4. **Corridors.** The 3×3 gcells of a 16×16 grid over **dr's own frame** around every coarse point of `gr` and every pin (dr:432–450).
5. **Electrical pricing and obstacles.**
   - `plain` is set for `Differential` nets, which then route blind to `Elec` (dr:469–472).
   - `keepout` costs nodes over the bbox of a "matched" cell (units of more than one owner) `KEEPOUT_COST`=2 for nets without a pin in that cell (dr:476–498).
   - Ring bands on routing layers are charged `usage = max(1)`. So is cell metal on routing layers, grown by `pitch − wire/2` (dr:507–524).
6. **Negotiation.** `run_pathfinder` with p_fac 2, history +0.5, ≤150 iterations; the first 8 are corridor-restricted (dr:27–29, 525).
7. **IR-driven R pricing.** Nets named by a violated `IrDrop` budget get `current = carried µA / max` (dr:531–550). The reroute that uses it happens in step 8's `_` arm.
8. **Rule repair.** `repair_constraints` (dr:1410–1515) runs ≤4 rounds with p_fac doubling each round. Every violated hard batch, and every budget or extra batch with residual > 0, generates trials:
   - `Differential`: copy the partner's tree by translation or vertical mirror (`copy_tree`, dr:1643–1691), else reroute with a mirror-image guide field (`mirror_guide`, dr:1697–1718).
   - `CommonNode`: reroute in a bisector-balance field (dr:1792–1805).
   - `CrosstalkExclusion` and `CouplingBudget`: a keep-away field, `COUPLE_COST` on the 8 same-layer neighbours of the other nets' trees (dr:1808–1822).
   - `Antenna`: one trial per layer, pricing that layer (`jumper`), then a gate lift field, then a plain reroute (dr:1486–1498, 1753–1788).
   - Anything else: rip up the violating nets and reroute with no field at doubled p_fac (dr:1499–1505).
   A trial is kept iff `(hard count, Σ residual)` drops lexicographically and overuse does not rise (dr:1520–1548, 1625–1635).
9. **Shields.** For each `Shield` ask, the free parallel stretches (≥2 nodes) beside each victim run are claimed, one node per stretch becomes a reference-net terminal, and the reference net is rerouted. The result is kept all-or-nothing on overuse (dr:1556–1621).
10. **Geometry.** `extract_geometry` → per-layer runs and deduplicated vias (gr:1085–1125). `build_routes` draws wires at `wire_width`, cuts at the deck size, and pads on both sides (dr:1322–1355). `add_pin_access` draws L-jogs with cuts and pads to each pin (dr:1174–1317).
11. **Short breaking.** While any two nets' shapes touch on a shared conductor, the access shape (or, for trunk vs trunk, the later net's shape) is deleted and counted as `sacrificed` (dr:617–635, 2075–2088).
12. **Fattening.** Nets are processed supply-first. Each trunk tries widths from its cap downward in steps of `2·grid` until one clears foreign metal by the layer's **widest** wide-metal spacing (dr:637–715). The cap is: EM need for `Differential` and C-weighted signal nets; `max(fat_signal, need)` for supplies whose IR budget holds; otherwise `fat_width`. The EM need per segment is `Limit::width_nm(I_edge, Blech domain)`, where `I_edge` is the largest KCL branch current (`branch_currents`, dr:897–934) of the tree edges inside the segment.
13. **Via arrays.** A cut whose two metals' overlap fits ≥2 cuts becomes an array, using the deck spacing (array spacing past its count threshold). Fewer cuts than the EM count goes to Θ (dr:717–806).
14. **Differential trim.** The lighter net of a pair gets one same-layer stub continuing a run (`trim_pair`, dr:944–990).
15. **Fill.** Same-net sliver and notch filling against foreign metal, with pins and their leads joined temporarily (dr:819–877, 1876–2053). The frame shift is then undone.
16. **Report** (dr:1828–1860).
    - Hard: analog hard batches, `open net` (layer-blind xy connectivity), `pin access sacrificed`, `drawn short`.
    - Budget: analog budget residuals, `em underwidth`, `em cuts`, overuse/capacity.
    - Cost: overuse + the cost arm (empty in the flow, §2.4).

### 1.4 Name for what this is

A **two-stage router**. The first stage is a **coarse 2-D gcell negotiator whose output only seeds loose corridors**. The second is a **capacity-1, fixed-pitch, reserved-direction track-lattice PathFinder** with node-level electrical costs, a post-hoc rule-driven rip-up loop, and **post-route geometric editing**: fattening, via arrays, stubs and fill. Width, symmetry, shielding, EM and via redundancy are decided **after** the path search. None is a resource the search reserves.

---

## 2. Module-by-module findings

### 2.1 backend/gr/src/lib.rs (1393 lines)

**What it does.** Coarse routing, plus the shared search core `dr` imports: `RGraph`, `TrackGrid`, `RouteHot`, `RouteCtx`, `Dij`, `route_net`, `run_pathfinder`, `extract_geometry`, `to_shapes`, `order_by_priority`, `symmetric_nets` and `analog_tiers` (dr:21, 457, 470, 1331–1334, 1837).

**Correctness problems**

1. **The global frame is anchored at the origin, not at the layout bbox** (gr:185–186, 361–368).
   - `die_extent` starts at `(2,2)` and grows only by far corners, and `gcell_terms` shifts by `(0,0)`.
   - A pin at negative x or y maps through `((x/gw) as u32)`. Rust's saturating cast turns that into gcell column/row 0, so all such pins collapse onto the boundary gcells.
   - A layout sitting far from the origin spends most of the 16×16 gcells on empty area.
   - `dr` knows placed geometry goes negative and shifts for it (dr:209–219). `price_group` handles its own offset (gr:113–124). `GlobalRoute::route` does neither.
2. **Capacity 6 is a constant**, unrelated to gcell size, pitch or layer count (gr:31–35, 466–469). On a 16×16 grid over a 50 µm die a gcell is ≈3.1 µm, about 6–7 sky130 tracks per layer across 2 layers. Over a 200 µm die a gcell is 12.5 µm and holds ~27 tracks per layer. Overflow is therefore not a routability measure.
3. **The global graph is single-layer** (gr:106–108, 190–192, 504–507). There is no layer assignment, no via cost and no per-layer capacity. The output segments are all on `layers[0]`, which also makes the discarded analog scores wrong: `Differential`, `ParasiticBudget` and `Antenna` all see one layer.
4. **PathFinder p_fac is constant across iterations** (gr:1019–1043). McMurchie–Ebeling PathFinder raises the present-congestion factor each iteration to force convergence (external knowledge, not in docs/ref/). Here convergence relies on history growth alone, so the ≤40-iteration cap is the real terminator.
   - History is added (`hist + p_fac·over`), where PathFinder multiplies (`(b+h)·p`).
   - `moved` is true whenever any dirty net was rerouted, even onto an identical path (gr:1032–1035), so the "no-move" stop rarely fires.
5. **Negotiation history never decays and is keyed by absolute 500 nm buckets across placements** (gr:37–92). The comment's "anti-oscillation guarantee" holds for a fixed routing problem. Here the placement changes every epoch, so stale congestion prices from earlier placements accumulate monotonically (`max` fold, no decay). Two further effects:
   - For the Global tier, gcell centres (µm-scale) move with the die extent, so the carried history lands in different buckets each epoch and is effectively arbitrary.
   - The test `negotiation_persists_across_calls` asserts `pressure()` keeps climbing (gr:1196–1218).
   This duplicates flow audit AF-06; it is repeated here because the router owns the data structure.
6. **`GroupPrice.reachable` measures nothing.** The gcell graph has no blockages, `price_group` sets no corridor or reservation, so `route_net` always finds a path and `reachable` is always true (gr:108–133). Only `hpwl` and `overflow` (the latter meaningless, item 2) vary.
7. **`seg_shape` emits 1-nm-wide shapes, including 1×1 squares for lone terminals** (gr:214–217, 371–374). The discarded `analog_tiers` then reads these squares as "cuts" in the `Differential` signature (differential.rs:69–78).

**Placeholders / measures-nothing**

- `GlobalRoute::route`'s `Report` is computed every epoch and thrown away (lib.rs:617–618; also elaborate.rs:175–176).
- `gr` output reaches `dr` only as corridor seeds (dr:432–450) and as part of dr's frame extent (dr:216, 238–242). Since dr's corridors are 3×3 dilations on a 16×16 grid (a 3/16 band per axis per point), and apply only to the first 8 of 150 iterations, gr's influence on the final route is weak and unmeasured.
- `price_group`'s `_layers` is unused (gr:106–108).

**Hard-coded constants** (none come from the deck)

- `GlobalCfg::default` = 16 gcells, capacity 6, 40 iterations, p_fac 3, hist 0.5 (gr:31–35)
- `HIST_QUANTUM_NM = 500` (gr:55)
- `KEEPOUT_COST = 2.0` (gr:757)
- `CORRIDOR_EPOCHS = 8` (gr:1013)
- `TrackGrid` coarsening at `max(die)/1200` (gr:528)
- Landing candidates limited to layers 0–1 (gr:586)
- The stacked-via exclusion window of ±2 rows is a fixed heuristic, not a deck stacking rule (gr:595–599)

**Dead code / API.** `Tier`, `Negotiation::seed/accumulate/pressure` are used. `NONE`, `RGraph::beside` default and `RouteCtx::new` are used by dr. No dead functions found.

**Missing edge cases**

- Negative coordinates (item 1).
- A net whose pins collapse to one point is not routed (gr:410–427). That is correct for coarse routing, but a same-net pair of pins 10 nm apart in different cells is then never checked for connection at the global stage.
- Multi-pin nets use a static target order from the first terminal, not the nearest to the growing tree (gr:943–948). The Prim variant in Lampaert §5.7.4 connects the closest terminal to the growing source each step (lampaert.txt:6231–6280, pp. 149–150).

**Tests (11)**

| Test | Asserts |
|---|---|
| `two_pin_net_routes_coarsely` (gr:1184–1193) | a coarse route exists |
| `negotiation_persists_across_calls` (1196–1218) | history carries and changes the second result |
| `budget_residual_reaches_theta`, `hard_batch_margin_is_measured_not_counted`, `overflow_theta_is_a_capacity_residual_not_a_count` (1234–1264) | score arithmetic |
| `price_group_measures_the_group_not_the_die` (1266–1278) | the offset handling in `price_group` |
| `bump_history_matches_definition` (1280–1286) | |
| `a_keepaway_rule_parts_two_nets_during_search` (1291–1318) | |
| `a_sensitive_net_steers_off_a_coupled_track` (1323–1361) | `Elec` coupling on `TrackGrid` |
| `a_current_carrying_net_avoids_via_resistance` (1366–1379) | |
| `symmetric_nets_first_then_impact_shields_last` (1383–1392) | |

Not tested: negative coordinates in `GlobalRoute::route`, gcell capacity against real tracks, convergence (overflow reaching 0 on a solvable instance), and determinism across runs.

### 2.2 backend/dr/src/lib.rs (2722 lines)

**What it does.** §1.3.

**Correctness problems**

1. **Per-pin current is over-counted for multi-finger devices** (dr:227–232, 234–236).
   - `pin_ua` finds the pin by `(net, rect)` and returns the **whole terminal current** for its name (`d{k}:S`, `d{k}:D`).
   - A MOSFET cell emits one `d{k}:S`/`d{k}:D` pin per diffusion region (kernel/cells/src/mosfet.rs:458–481), and `pin_currents` stores one value per terminal name (lib.rs:1035–1055).
   - An N-finger device's S pins therefore each inject I_S, so `branch_currents` sees ≈(#S pins)·I_S. Every EM width and cut count on those nets is over-sized by that factor. Where fattening cannot reach it, the net reports false `em underwidth` / `em cuts` Θ.
   - Fix: divide the terminal current across the pins of that terminal (by finger count or pin share).
2. **Foreign metal is a soft obstacle, and shorts to it are invisible to dr.**
   - Ring bands and cell metal on routing layers are charged `usage=max(1)`, "full but crossable at a price" (dr:507–524, 1153–1162).
   - A trunk that ends up on such a node after negotiation physically shorts or spaces into foreign metal.
   - `first_short` and `cross_net_shorts` compare only `routes.wires` between nets (dr:2075–2113), and cell/ring metal is not in `routes.wires`. The case therefore surfaces only as `routing overuse` in Θ (dr:1855–1857), a budget, until signoff's LVS/DRC counts it.
   - The ring comment says a hard block "would seal the enclosure on a two-layer stack". That problem is itself caused by AT-01 (two layers on sky130).
3. **Residual overuse on a capacity-1 lattice is a physical conflict, but it is scored as Θ** (dr:1855–1857). Net-vs-net overuse becomes an `open net` through the short resolver (dr:629–633), which is hard. Net-vs-foreign-metal overuse stays Θ (item 2).
4. **`unreachable_shapes` is layer-blind** (dr:2115–2135). Two same-net shapes on different metals that overlap in xy count as connected with no cut, so an open between layers is under-reported. The comment says it matches `Routes::debug_check` (routes.rs:43–75), which is equally layer-blind. Signoff LVS is the only real check.
5. **Access geometry is excluded from every EM check.** The fatten loop stops at `pre_access` (dr:654). Access cuts carry no tree-edge current, so `edges_ua` returns 0 and `cuts(0)=1` (dr:748–749). The "narrow" jog option is `min(wire, pin.w, pin.h)` floored at `min_width` (dr:384, 1209). The access segment carries the pin's full current, the largest single-terminal current on the net. Hastings: a lead "must meet electromigration rules at every point along its length. One cannot neck down a wide lead" (§15.4.4 High-Current Signals, hastings.txt:48738–48747, p821). Lienig §3.5.3: terminal regions whose ampacity is below the wire's current "should then be eliminated as potential connecting points" (lienig_em.txt:4687–4699, p102). The pin-access cut (mcon on sky130) is always a single cut.
6. **EM shortfall on trunks is a budget (Θ), not a violation** (dr:711–713, 1852–1854, 882–884). An under-width segment is a reliability failure. The hard `Electromigration` rule passes when *any* segment is wide enough (em.rs:106–120, audit-02 AR-05), so per-segment EM never reaches V.
7. **The fattening cap for C-weighted signals and differential pairs is the EM need only** (dr:667–679). An R-sensitive net is never widened for R. See TODAES'22 in §3, and AT-17.
8. **Fattening candidates step by `2·grid`** from the cap down (dr:701–706). With sky130 grid 5 and fat_supply ≈ 2990 nm (derived: `m1.3b` threshold 3 µm − 10 nm, elaborate.rs:429–435), that is up to ~270 candidate widths per segment. Each candidate runs `foreign()` over all shapes of all other nets and all cell metal, and `clear()` repeats that for every touching same-net shape (dr:688–700). This is roughly O(segments × 270 × S × k) per dr call.
9. **`trim_pair` repairs the metric, not the circuit** (dr:936–990).
   - A dangling same-layer stub carries no current. It adds ground C and adds to "summed R".
   - `Differential` with a stack compares ground C and **summed run R** (differential.rs:61–64). A stub therefore equalises the measured R while the terminal-to-terminal R stays unequal.
   - The stub is placed after rule repair and fattening. It is not EM-checked, and it adds antenna area to a net that drives the next stage's gates.
   - The ponytail comment acknowledges "it is not on a terminal path".
10. **Differential repair can be defeated by landing and grid asymmetry.**
    - Landing is first-come in pin order (dr:196–204, 360–431). Net A's pins claim nodes before net B's, so mirror-placed pins can land on non-mirrored nodes.
    - The lattice origin is `low − halo − margin` (dr:218) and track centres sit at `origin + pitch/2 + k·pitch` (gr:641–644). Nothing puts the placement's symmetry axis (`Layout::axis`) on a track or half-track, so the mirror of a track is in general not a track.
    - `copy_tree` needs an exact bin mapping of landed terminals (dr:1646–1674). `mirror_guide` tolerates one pitch but is only a 1.0-per-node soft field that congestion can override (dr:1381, 1697–1718).
    - The mirror axis is the mean of the two nets' landed terminal x (dr:1703–1704), not `Layout::axis`.
    - Only vertical-axis mirrors and translations are tried (dr:1637–1662).
11. **`Differential` nets are exempt from all electrical pricing** (`plain`, dr:469–472; gr:738–742, 794–795). The rationale is that the cost field around a pair is asymmetric. The effect is that a sensitive differential pair pays nothing for running beside a clock or an output, and only `CrosstalkExclusion` repair (post hoc) can move it.
12. **The `_` repair arm is blind** (dr:1499–1505). For `ParasiticBudget`, `PerformanceBudget`, `Electromigration` and `Shield`, a trial rips up the net and reroutes it with the same `Elec` weights and no penalty field, only p_fac doubled.
    - With its own usage removed, the net usually re-finds the same path, so the trial is spent without effect.
    - For EM it cannot help at all, because width is decided after the search.
    - Only `IrDrop` benefits, because `cold.current` was set beforehand (dr:539–550).
13. **Repair is judged on pre-fattening, pre-access geometry** (`probe`, dr:555). The later passes change the geometry the rules score: access jogs, short breaking, fattening, via arrays, `trim_pair` and fill. `Differential` area and pad terms, coupling, shield coverage, antenna area and IR all move. The final `score` reports the result (dr:881), but nothing repairs what those passes break.
14. **Differential pair fattening can break symmetry.** Each half is fattened to its own EM need (dr:673–674). Unequal branch currents, or item 1's over-counting on unequal finger counts, give unequal widths.
15. **The short resolver turns shorts into opens by deleting shapes** (dr:617–635). It picks the first touching pair in (a, b, i, j) order, which is deterministic but arbitrary with respect to criticality. For trunk vs trunk it drops the later net's trunk shape and decrements `pre_access[b]`. That index book-keeping assumes the removed shape precedes the access shapes, which holds because `j < pre_access[b]` in that branch.
16. **`GCELLS_PER_SIDE` "must match gr so corridors line up"** (dr:30–31). The two grids are over different frames (gr: origin to far corner; dr: shifted frame with halo and margin), so they never line up. The corridor is rebuilt from coarse points (dr:433–447), so equality of the constant is irrelevant. The comment is misleading.
17. The doc comment on `widen` is duplicated on one line (dr:1028).

**Placeholders / measures-nothing**

- `cold.current` is empty unless an IR budget broke (dr:466). R is otherwise never priced.
- `em_cuts` for access cuts is always 1 (item 5).
- `place_near` scans rings up to `reach/step` with step = grid, so a 50 µm reach at a 5 nm grid is 10,000 rings × up to 8·ring positions per ring, each checked against every obstacle (dr:998–1026; called with 50,000 nm, elaborate.rs:358). The worst case is huge when the region is crowded.

**Hard-coded constants** (all tuning literals, not deck-derived)

| Constant | Value | Where |
|---|---|---|
| `VIA_COST` | 4 | dr:26 |
| `P_FAC` | 2 | dr:27 |
| `HIST_INC` | 0.5 | dr:28 |
| `MAX_ITERS` | 150 | dr:29 |
| `GCELLS_PER_SIDE` | 16 | dr:31 |
| `HARD_ROUNDS` | 4 | dr:33 |
| halo, margin | 2 and 3 pitches | dr:213 |
| landing radii | 8, 2, 3, 4; k = 12 | dr:387, 409–412 |
| `GUIDE_COST` | 1 | dr:1381 |
| `COUPLE_COST` | 2 | dr:1383 |
| `JUMP_COST` | `VIA_COST/2` | dr:1386 |
| `LIFT_COST` | 16·`VIA_COST` | dr:1390 |
| `BALANCE_COST` | 0.5 | dr:1805 |
| heal passes | 4 | dr:1881 |
| notch passes | 2 | dr:1985 |

`VIA_COST` is unrelated to the deck via R; the deck R is used only through `via_r` when current is priced.

**Dead / debt**

- `DetailedCfg::pin_access_*` are used.
- The `extra` and `CommonNodes` batches enter only `residual`, never the hard count (dr:1420–1425).
- `kind()`-string dispatch (`ends_with("Differential")`, `rsplit("::")`, `ends_with("IrDrop")`; dr:534, 811, 1444; gr:253, 277) is stringly typed. A future rule named `…Differential` or `…Antenna` would silently take a repair arm.

**Missing edge cases**

- No wrong-way routing on a layer, so a short jog on the "wrong" direction costs two vias (gr:630–638).
- No stacked-via rule from the deck; a fixed ±2-row heuristic only at landing (gr:595–599).
- No over-the-cell policy beyond matched-cell keepout. Metal can run over any non-matched cell's gates and over resistors. Hastings rule 21: "avoid routing unconnected leads over matched resistors unless they are field plated" (hastings.txt:25382–25384, p422).
- Multi-finger pins are all routed by the router (strapping), with no notion that the cell may already strap them.
- `branch_currents` cuts cycles to a DFS spanning tree (dr:895–896, 914–925). Shields and via arrays do not create cycles in the node tree, but a reference net rerouted to several shield claims can form a mesh in geometry that the node tree does not show.

**Tests (25)**

- **Realisation and landing:** `detailed_realises_a_two_pin_net`, `every_pin_is_touched_by_its_own_net`, `ring_pins_are_routed`, `inflated_bbox_does_not_block_routing`, `a_pin_on_the_upper_metal_is_stitched_to_its_node`, `a_node_between_two_nets_pins_carries_neither`, `no_two_nets_share_geometry_on_a_layer`.
- **Scoring:** `empty_coarse_gives_empty_routes`, `budget_residual_reaches_theta`.
- **Rules:** `a_skewed_common_node_is_rebalanced`, `foreign_nets_route_around_a_matched_cell`, `antenna_repair_jumps_to_a_higher_metal`, `series_r_is_priced_only_past_the_drop_budget`, `a_shield_request_draws_tied_reference_tracks_both_sides`, `trim_lengthens_the_lighter_side_of_a_pair`, `a_differential_pair_is_not_fattened`, `mirror_guide_follows_the_partner`, `copy_tree_reproduces_the_template_exactly`.
- **EM:** `segments_are_sized_by_their_branch_current`, `branch_currents_sum_the_far_side`, `fat_trunks_get_via_arrays`.
- **Geometry fixes:** `a_bridged_gap_between_same_net_trunks_is_still_filled`, `an_access_cut_beside_a_same_net_via_is_not_duplicated`.
- **Other:** `negotiation_persists_across_calls`, `place_near_finds_the_closest_free_spot`.

All use synthetic layers (`LAYERS=[0,1]`, `CUTS=[(2,100,140,140)]`) and a 430 nm pitch (dr:2141–2147). None runs the real-deck DRC on dr output. None routes a `Differential` pair end-to-end through `DetailedRoute::route` and checks mismatch = 0. None exercises `CrosstalkExclusion` or `CouplingBudget` repair in dr. None covers multi-finger current division (item 1).

End-to-end coverage comes from benchmarks/tests/signoff_fixtures.rs:15–41, 150–156: DRC 0 on 7 small sky130 fixtures, with the OTA-class fixtures behind `--ignored`. `ota_cross_pdk` excludes routing-layer DRC as known "dr debt" on other PDKs (ota_cross_pdk.rs:196–224).

### 2.3 frontend/library/src/lib.rs (1439 lines) and elaborate.rs (460 lines): how routing is driven

1. **The `gr` report is discarded** (lib.rs:617–618; elaborate.rs:175–176).
2. **dr can run twice per epoch** when diodes are inserted (lib.rs:646). Both runs accumulate history into `neg`, so the second doubles the history bump for that epoch.
3. **The stack is truncated at the first metal wider than the access pad** (elaborate.rs:211–216). The bottom metal is **always** split off as pin access (elaborate.rs:219), even on decks with no li (gf180, ihp; §1.1 table), so metal1 is never a routing layer there. The comment names li explicitly (elaborate.rs:206–209).
4. **One wire width = the first-via pad** (elaborate.rs:396–398). That is 2.07× met1 min width on sky130, 1.9× on ihp and 1.36× on gf180 (derived). Pads are square at the larger of the symmetric and asymmetric enclosure (verify/pdk.rs:354–365). Hastings' via-to-via pitch uses "the smallest overlap allowed for two opposite sides" (§15.4.2 eq. 15.19, hastings.txt:48199–48219, p813). The resulting pitch (460 nm on sky130) is wider than that pitch.
5. **`net_weight` for the router is zero on every net without a C budget or a perf sensitivity** (lib.rs:312–316). `gp::net_weights` classes alone do not steer routing.
6. **`em_rules` fills the hard EM rule with the largest single-terminal current** (lib.rs:991–1028). `pin_currents` is all-or-nothing (lib.rs:1035–1038). `oppoint::terminal_ua` returns an **empty** draw list (current 0) for every non-MOS device (oppoint.rs:45–47), so resistor, BJT and diode currents are known zeros that pass EM and IR. This ties to audit-07 AF-03.
7. **IR budgets** come from `annotator::ir::budgets` with `vdd_mv` defaulting to 1800 when there is no op config (lib.rs:283–293).
8. **`em_limits` derates to one die temperature** (elaborate.rs:242–262). There is no per-conductor self-heating (acknowledged at elaborate.rs:245–247).

### 2.4 kernel/analog/src/routing/* (scoring, as seen by the router)

Rule-level defects are audit-02's (AR-05 EM best-segment, AR-06 shield min-of-sides, AR-07 coupling counts shields and supplies, AR-12 Differential hard with tolerance). Router-relevant points:

- **mod.rs (25):** re-exports only.
- **antenna.rs (67).** Per-stage check through `Stack::antenna` with gate pins (antenna.rs:60–66). dr consumes `violating_ids` for jumper and lift trials (dr:1486–1498). The fallback without a stack sums every layer against the tightest ratio (antenna.rs:64–65). No tests in this file; `stack.rs` has 2 antenna tests (stack.rs:478–531).
  - The diode credit applies from the first stage even if the diode is reached only through a higher metal (stack.rs:286–291, acknowledged).
  - No latent antenna effect: Hastings §5.1.6 says a group of minimum-spaced geometries clears late and must be checked as one conductor (hastings.txt:13166–13172, p228).
- **common_node.rs (123).** Measures |R_a − R_b| on the port graph from the feeds (common_node.rs:40–53). dr repairs it with `balance_field`, which prices node imbalance by Manhattan pin distance rather than R, so a layer change or width is not seen (dr:1792–1802). 1 test.
- **coupling.rs (192).**
  - `EPS_H_AF = 12.0` is a PDK-specific fallback literal (coupling.rs:14–18).
  - The router's `beside_c` covers only the two immediately adjacent tracks of the same layer (gr:449–452, 648–660), while the rule sums every same-layer parallel pair at any gap (coupling.rs:49–67). The router cannot see coupling at 2+ pitches, which the rule counts.
  - Neither sees inter-layer overlap C.
  - 6 tests.
- **crosstalk.rs (115).** Euclidean clearance (crosstalk.rs:37–42). Extracted with spacing 0, then the annotator sets `route_space·multiple` (extract.rs:57–61). dr's `keep_away` enforces at most a one-track (8-neighbourhood) standoff (dr:1807–1822), so a spacing floor above `2·pitch − wire` cannot be met by repair. No tests.
- **differential.rs (198).** Per-layer geometric signature, or stack-mode max(ground C, summed run R) (differential.rs:54–90). This summed R is the metric that `trim_pair` games (§2.2 item 9). It is extracted for diff-pair **drain** nets only at 5% (differential.rs:116–133). 2 tests.
- **em.rs (243).** `Limit::width_nm` and `cuts` are correct per Lienig eqs. 3.21 and 3.25 without g(H). The inhomogeneity factor g(H) > 1 for non-uniform flow (lienig_em.txt:4637–4645, p101) is absent, and dr's via arrays assume equal sharing (dr:721–722). The rule's best-segment check is AR-05. 5 tests.
- **ir.rs (94).** Worst-path R × whole current (ir.rs:31–36). This is what triggers dr's R pricing. 1 test.
- **parasitic.rs (126).** `cost = len²·1e-6` is hand-tuned (parasitic.rs:41–49). It is **unused by the routers in the flow**, because the annotator pushes no routing cost-arm batches (extract.rs:22–106), so `analog_cost` in gr and dr `score` is 0 (gr:406, dr:1858). 1 test.
- **performance.rs (95).** A linear C budget priced as length × a single `af_per_nm` (performance.rs:31–33): no layer, width or coupling term. The router sees it only through the `_` blind reroute (§2.2 item 12) and through `net_weight` (lib.rs:294–296, 313–316). 1 test.
- **shield.rs (134).** Same-layer coverage only (shield.rs:18–19, 29–62). dr builds only same-layer side shields (dr:1556–1621). 1 test.
- **stack.rs (532).** `ground_af` sums rather than unions (stack.rs:52–64). `path_resistance_ohm`, `port_graph` and `pieces` are all O(k²) (stack.rs:94–146, 183–251, 380–402), and all of them run inside dr's repair `key` for every trial. 7 tests.

---

## 3. Gap analysis against "best constraint-aware analog P&R that beats hand layout"

Each row is what an expert or the state of the art does, then what Philis does. Items marked *external* are not in `docs/ref/` and are general knowledge about the named tools.

1. **Use every useful layer, with per-layer width and pitch.**
   - *Expert / SotA:* Lampaert §5.2 requires n-layer routing ("an analog routing algorithm has to take advantage of all available routing layers", lampaert.txt:5472–5476, p132). ROAD routes on a 3-D non-uniform grid (survey §4.5.3.2, balasa_graeb_survey.txt:9407–9431, pp. 187–188).
   - *Philis:* 2 layers on sky130; metal1 dropped on gf180/ihp; one pitch and one wire width for all layers (§1.1; AT-01, AT-02).
2. **Variable wire width inside the search.**
   - *Expert / SotA:* Lampaert rejects gridded models because "variable wire widths … can not be used since the grid-based model requires a uniform pitch" (§5.4.2, lampaert.txt:5555–5562, p134). The survey: "variable width routing and layout constraint observance are necessary" (balasa_graeb_survey.txt:8509–8512, p168). TODAES'22 does wire sizing *between* global and detailed routing (todaes22.txt:187–188, p5) and realises it as parallel routes on a gridded router (todaes22.txt:628–640, pp. 15–16). The CC-DAC router uses parallel wires and parallel vias for critical bits (cc_dac_constructive.txt:790–795, p8).
   - *Philis:* every net searches at width 1 track; widening happens after, into leftover space only, and a shortfall goes to Θ (AT-03).
3. **Current-driven topology (wire planning) before sizing.**
   - *Expert / SotA:* Lienig §3.3: the net topology must be planned with the currents, "current-intensive segments are kept as short as possible" (lienig_em.txt:4054–4141, pp. 89–91). Survey §4.7 describes current-driven Steiner construction that sums terminal currents at Steiner points (balasa_graeb_survey.txt:10336–10372, pp. 208–209).
   - *Philis:* the topology is min-cost with a static nearest-first order. Current enters only as R pricing after an IR budget breaks (AT-16).
4. **Symmetric routing that is exact by construction.**
   - *Expert / SotA:* ANAGRAM II evolves both halves at once, so a blockage on one side blocks both. ROAD mirrors obstacles and routes one side (balasa_graeb_survey.txt:8827–8905, pp. 175–177). Lampaert routes one net and legalises each expansion on both sides of the axis (lampaert.txt:6199–6225, pp. 148–149). The "connector" technique handles pairs that must cross the axis (balasa_graeb_survey.txt:8905–8911, p177). "Exact route matching" requires equal length per layer (perf_driven_survey.txt:86–89, p2).
   - *Philis:* routes one net blind to parasitics, then tries a post-hoc copy or mirror. The lattice is not axis-aligned, landing is asymmetric, there is no horizontal or point symmetry and no crossing connector. A dangling stub equalises the metric (AT-04, AT-20, AT-21, AT-38).
5. **Parasitic cost in performance units with hard bounds during search.**
   - *Expert / SotA:* Lampaert eqs. 5.9–5.11: segment cost = ΣΔP_j from sensitivities to C_i, R_i and C_ki, including overlap, lateral and fringe C between layers (lampaert.txt:6099–6175, pp. 146–148). ANAGRAM III and Lampaert's manufacturability phase prune partial paths that violate bounds (balasa_graeb_survey.txt:9470–9480, pp. 188–189; lampaert.txt:6351–6363, p152). ROAD raises the weights of violated parasitics and reroutes (balasa_graeb_survey.txt:9440–9449, p188).
   - *Philis:* a per-net scalar weight in [0,1] × relative ground C, plus one-track lateral C. No R unless IR broke. No inter-layer C. No bound pruning. Blind reroute on violation (AT-12, AT-13, AT-32).
6. **Supply and ground topology: star nodes, Kelvin connections, net splitting, supplies first.**
   - *Expert / SotA:* Hastings §15.4.4: star nodes and Kelvin force/sense (hastings.txt:48509–48560, p817). Net splitting separates high-current paths (survey §4.4.1, balasa_graeb_survey.txt:8806–8826, p175; ROAD splits nets, 9449–9451, p188). ILAC routes power nets first (balasa_graeb_survey.txt:9068–9073, p180).
   - *Philis:* no star or Kelvin topology, no net splitting, supplies routed as ordinary free nets (tier 3). Supplies widen only to `fat_signal` unless IR broke. The only balanced-branch mechanism is `CommonNode` for matched pairs' shared sources (AT-15).
7. **Shielding in 3-D.**
   - *Expert / SotA:* Hastings Fig. 15.28A puts a metal-2 plate between a noisy metal-3 and a sensitive metal-1, extended beyond the intersection; noisy signals "should not route on an adjacent layer either above or below a sensitive signal" (hastings.txt:48639–48690, pp. 819–820). Survey Fig. 4.16: same-layer, different-layer and bulk shields (balasa_graeb_survey.txt:8950–8980, p178). Matched arrays get electrostatic shields above them (hastings.txt:22671–22684).
   - *Philis:* same-layer side tracks only, added after repair, all-or-nothing, not re-verified after fattening (AT-22).
8. **Metal over matched devices.**
   - *Expert / SotA:* Hastings: "designers should not place metallization above the active gate regions of critical matched transistors … match the metal patterns surrounding the transistors"; metal coverage causes up to 20% drain-current mismatch (hastings.txt:41789–41797, p703). ROAD can forbid routing over sensitive modules (balasa_graeb_survey.txt:9408–9409, p187). The perf survey lists "forbidding routing over the active regions of transistors" (perf_driven_survey.txt:83–84, p2).
   - *Philis:* a soft cost of 2.0 per node, foreign nets only, over the whole bbox. Own nets (drain straps) run freely and asymmetrically over the pair. Nothing constrains metal-pattern symmetry (AT-14).
9. **EM-correct terminals and vias, with redundant vias.**
   - *Expert / SotA:* Lienig §3.5.3 terminal ampacity (lienig_em.txt:4687–4699, p102). §4.6.1: double vias are inserted after routing for reliability (lienig_em.txt:6116–6120, p133). §4.6.4: via geometry against the current turn (lienig_em.txt:6234–6260, pp. 136–137).
   - *Philis:* access jogs and pin cuts are never EM-checked. Min-width trunks get single cuts. Arrays assume equal sharing (AT-06, AT-18, AT-19).
10. **Global routing that serves placement and detailed routing.**
    - *Expert / SotA:* ALADIN runs a Steiner global route for every intermediate placement, with congestion-weighted channels (balasa_graeb_survey.txt:9686–9730, pp. 193–195). The KOAN/ANAGRAM II idea anneals wires with devices (9615–9630, p192). TODAES'22 uses the global route as features for wire sizing.
    - *Philis:* a 2-D, constant-capacity, origin-anchored gcell router whose score is discarded and whose route only loosens early corridors (AT-08, AT-09).
11. **Manufacturability and yield routing.**
    - *Expert / SotA:* Lampaert's manufacturability phase spends leftover performance margin on bridging-fault critical area (eq. 5.15, lampaert.txt:6351–6370, p152). Survey §4.7 (balasa_graeb_survey.txt:10301–10335, pp. 207–208).
    - *Philis:* none. Wire spreading, via doubling and critical-area cost are absent (AT-35).
12. **Learned routing guidance (MAGICAL).**
    - *Expert / SotA:* MAGICAL-lineage GeniusRoute trains a VAE on manual layouts to predict routing regions, then turns them into guidance for the detailed router (perf_driven_survey.txt:150–158, p3).
    - *Philis:* none. Corridors come only from `gr`. This is a longer-term item. The corridor mechanism (`RouteCtx::corridors`) is the natural hook.
13. **Search efficiency.**
    - *Expert / SotA:* ROAD, Lampaert and CORAL use A* with an admissible predictor (lampaert.txt:6181–6197, p148; balasa_graeb_survey.txt:9480–9500, p189).
    - *Philis:* plain multi-source Dijkstra, a full `Dij` allocation per trial, O(S²) and O(S³) geometric loops (AT-12).
14. **Template / generator routing (BAG, LAYGEN; *external* for BAG).**
    - *Expert / SotA:* the survey's §4.5.6 template approaches, LAYGEN adapting template routes (balasa_graeb_survey.txt:9800–9890, pp. 196–198). BAG draws track-based wires with explicit widths from generator code (*external*).
    - *Philis:* `macroMaster` generators have no route templates. `elaborate_ir` re-routes from scratch (audit-07).

To beat hand layout, the router must produce what an expert draws for analog-critical nets:
- mirror-exact pairs on an axis-aligned grid;
- minimum-width, low-C sensitive wires with asymmetric-enclosure vias;
- wide, short, via-arrayed current paths with EM-correct pin landings;
- star or Kelvin returns;
- 3-D shields;
- no metal over matched gates;
- every rule re-verified after the geometry passes.

Today only the parasitic-weighted search, the IR repricing, `CommonNode` balancing, antenna jumpers and lifts, same-layer shields and via arrays on fattened trunks exist. Most of them are post-hoc.

---

## 4. Ranked findings

| ID | Severity | file:line | Finding | Suggested fix direction |
|---|---|---|---|---|
| AT-01 | critical | frontend/library/src/elaborate.rs:210–219 (take_while 215, split 219) | The routing stack stops at the first metal whose min width exceeds the first via pad. On sky130 the router uses **met1+met2 only** (met3 300 nm > 290 nm pad; derived). The bottom metal is always split off as pin access, so gf180 and ihp lose metal1 for routing. There is no thick-metal supply routing and no upper-layer shields or jumpers | Per-layer width, pitch and direction tables (a layer joins the stack with its own min width). Split off pin access only when the deck marks a local-interconnect role |
| AT-02 | high | elaborate.rs:393–399; verify/pdk.rs:354–365; gr/src/lib.rs:524–532, 630–638 | One wire width (the first-via pad: 2.07× met1 min width on sky130, derived) and one pitch (460 nm on sky130, derived) for every layer. Square pads use the larger enclosure on all sides. Fixed preferred direction with no wrong-way edges. The result is more ground C per µm and more area than Hastings' via-to-via pitch (hastings.txt:48199–48219, p813) | Draw trunks at the layer min width. Use asymmetric pads oriented along the wire. Use a per-layer pitch derived from the via-to-via pitch. Allow wrong-way jogs at a cost |
| AT-03 | critical | dr/src/lib.rs:637–715, 1852–1854; gr/src/lib.rs:615–620 | Width is not a routing resource: every net searches at 1 track and EM or R width is grown afterwards into leftover space. A shortfall is Θ, not V. High-current and R-critical nets cannot be guaranteed their width | Assign widths (EM need, IR, R-sensitivity) **before** detailed routing, as TODAES'22 does (todaes22.txt:187–216). Realise width as k parallel tracks or a multi-track footprint in the lattice (reserve k adjacent nodes plus via arrays). Make per-segment EM under-width a hard V |
| AT-04 | critical | dr/src/lib.rs:196–204, 218, 1643–1718; gr/src/lib.rs:641–644 | Symmetric routing is not exact. Pins land first-come, so mirrored pins land asymmetrically. The track lattice is not aligned to `Layout::axis`. `copy_tree` needs exact bin symmetry, and the fallback `mirror_guide` is a soft 1.0 field. Pairs route blind to parasitics | Align the lattice origin so the axis lies on a track or half-track (snap `Layout::axis` in dp). Land and route symmetric pairs jointly: one search in which each expansion must be legal on both sides (Lampaert §5.7.3, lampaert.txt:6199–6225; ANAGRAM II, balasa_graeb_survey.txt:8880–8898) |
| AT-05 | high | dr/src/lib.rs:507–524, 1153–1162, 2075–2113, 1855–1857 | Ring and cell metal are soft obstacles (charged usage). A trunk left on them after negotiation is a physical short or spacing violation that dr's short checks cannot see (they compare `routes.wires` only). It is reported as Θ overuse | Make foreign cell and ring metal hard blockages, per layer, grown by spacing. Treat residual overuse as hard V. Include cell and ring metal in `cross_net_shorts` |
| AT-06 | high | dr/src/lib.rs:654, 721–722, 748–749, 384, 1207–1209 | Pin-access jogs and pin cuts, which carry the pin's full current, are never EM-sized or arrayed. The "narrow" jog may neck to `min(pin.w, pin.h)`. This contradicts Hastings (no necking, hastings.txt:48741–48747, p821) and Lienig terminal ampacity (lienig_em.txt:4687–4699, p102) | Size access jogs and cuts from the pin's current. Choose landing sides and pin regions by ampacity. Array the access cut |
| AT-07 | high | dr/src/lib.rs:227–236; frontend/library/src/lib.rs:1035–1055; kernel/cells/src/mosfet.rs:458–481 | Every pin of a multi-finger terminal is assigned the whole terminal current. Branch currents, EM widths and cut counts are therefore inflated by the pin count. This yields false `em underwidth` and `em cuts` Θ and over-fattening | Split terminal current across that terminal's pins (equal share, or by finger width). Add a test with a 4-finger device |
| AT-08 | high | gr/src/lib.rs:185–186, 361–368, 471–477 | The global grid spans origin → far corner. Pins at negative coordinates saturate into gcell 0, and an offset layout wastes most of the 16×16 gcells | Anchor the grid on the pins', bboxes' and rings' bbox, as `price_group` does (gr:113–124) |
| AT-09 | medium | gr/src/lib.rs:31–35, 106–108, 190–192, 207–223; frontend/library/src/lib.rs:617–618 | `gr` is a 2-D graph with constant capacity 6, no layers or vias, output on `layers[0]`, and an analog report discarded every epoch. Its only effect is loose corridors for 8 of 150 dr iterations | Either delete gr (area-route in dr, as Lampaert argues, lampaert.txt:5511–5540, p134), or make it 3-D with capacity from tracks per gcell per layer and use it as the placement-time routability and parasitic estimator (ALADIN, balasa_graeb_survey.txt:9686–9730) |
| AT-10 | medium | gr/src/lib.rs:37–92; frontend/library/src/lib.rs:342 | PathFinder history is carried across unrelated placements, never decays, and is `max`-folded. Global-tier buckets move with the die extent. Stale prices grow over up to 800 epochs (duplicates audit-07 AF-06) | Reset history per placement (or decay it by a factor per epoch). Key the global tier by gcell, not by the 500 nm bucket |
| AT-11 | low | gr/src/lib.rs:936–941, 1019–1043 | Present-congestion factor constant; history additive; the "no move" stop rarely fires | Grow p_fac geometrically per iteration. Use `(b + h)·p` cost. Stop on stationary overflow |
| AT-12 | medium | gr/src/lib.rs:882–1010; dr/src/lib.rs:1420–1425, 1535, 1605, 617–635, 688–706, 1792–1802 | Runtime: Dijkstra with no A* predictor; a full `Dij` allocation per trial; the repair `key` rebuilds all routes and re-scores all O(k²) rules per trial; `first_short` is O(S²) inside a delete loop (O(S³)); fattening tries ~270 widths per segment (derived) with O(S) scans; `balance_field` is O(nodes × pins). No per-stage timing exists (benchmarks/src/bench.rs:425–432 times the whole flow) | Add an A* Manhattan predictor (Lampaert §5.7.2, lampaert.txt:6181–6197). Reuse one `Dij`. Index shapes per layer (a grid bucket). Binary-search widths. Score rules incrementally per touched net. Add per-stage timers |
| AT-13 | medium | gr/src/lib.rs:449–452, 648–660, 908–927; elaborate.rs:436–444 | Search coupling covers only the two adjacent same-layer tracks. It ignores overlap and crossing C between layers and lateral C at 2+ pitches, which Hastings flags as equally harmful (hastings.txt:48683–48690, p820) and which Lampaert eq. 5.10 prices (lampaert.txt:6140–6160, p147) | Add up/down neighbour nodes to `beside` with per-layer-pair overlap C. Extend lateral C to k tracks with a 1/gap falloff |
| AT-14 | high | gr/src/lib.rs:748–757, 936–940; dr/src/lib.rs:476–498 | Metal over matched devices is a soft cost of 2.0 per node for foreign nets only, over the cell bbox (including any ring halo). Own nets run over gates freely, with no pattern symmetry. Hastings reports up to 20% mismatch from metal coverage (hastings.txt:41789–41797, p703) | Hard keep-out on the gate region (from `units`) for all nets up to a deck-declared layer. Mirror-symmetric metal over matched cells (own straps via `copy_tree`). Optional full-cover plate |
| AT-15 | high | gr/src/lib.rs:250–271; dr/src/lib.rs:667–679; frontend/library/src/lib.rs:304–309 | Supplies are ordinary free nets: no planned rails, no star or Kelvin topology, no net splitting, no priority. They widen only to `fat_signal` unless an IR budget broke | Pre-route supply trunks on upper metals (needs AT-01). Add a star/Kelvin topology rule for returns shared by matched devices (Hastings §15.4.4, hastings.txt:48509–48560). Net-split force and sense (survey §4.4.1). Route supplies first (ILAC, balasa_graeb_survey.txt:9068–9073) |
| AT-16 | medium | gr/src/lib.rs:943–1008; dr/src/lib.rs:531–550 | The net topology ignores current: a static nearest-from-root target order and min-cost joins. Current-intensive segments are not kept short (Lienig §3.3, lienig_em.txt:4106–4141, pp. 90–91) | Current-driven wire planning per net before detailed routing: a Steiner tree built by merging terminal currents (survey §4.7, balasa_graeb_survey.txt:10336–10372). Feed its segment widths to AT-03 |
| AT-17 | medium | dr/src/lib.rs:667–679 | C-weighted signal nets and differential pairs are never widened beyond EM need. R-sensitive nets (source-degeneration lines, current-mirror gates) lose gm and UGF. TODAES'22 reports UGF 688.5→972.0 MHz from 1×→3× width on two OTA wires (todaes22.txt:44–54, p2) | Separate R-sensitivity from C-sensitivity per net (the annotator provides the direction; the perf rows provide ∂/∂R). Size by the sign of the combined sensitivity. Keep both halves of a pair at one width (todaes22.txt:215–216) |
| AT-18 | medium | dr/src/lib.rs:762–780 | Min-width trunks always get one cut. There is no redundant (double) via insertion (Lienig §4.6.1, lienig_em.txt:6116–6120, p133) | After routing, double every via where space allows (a second cut along the wire). Prefer arrays on current paths |
| AT-19 | low | dr/src/lib.rs:717–806; kernel/analog/src/routing/em.rs:60–71 | Via arrays assume equal sharing. No g(H) factor (lienig_em.txt:4637–4645, p101), no turn geometry (§4.6.4, lienig_em.txt:6234–6260) | Apply a g(H) > 1 at a direction change. Orient arrays across the current flow |
| AT-20 | high | dr/src/lib.rs:936–990, 808–817; kernel/analog/src/routing/differential.rs:61–64 | `trim_pair` adds a dangling stub that equalises the rule's **summed** R and C while the terminal-path R stays unequal. It also adds antenna area and skips EM and repair re-checks | Measure pair matching with terminal-resolved R (`Stack::terminal_resistance_ohm`) and C. Replace stubs with in-path serpentine or detour on the shorter net (LEMAR-style, todaes22.txt:118–119). Otherwise leave the pair unfixed and report it |
| AT-21 | medium | dr/src/lib.rs:1637–1718 | Symmetry covers only translation and vertical-axis mirror. No horizontal axis, no point (common-centroid) symmetry, no crossing "connector" for pairs whose terminals straddle the axis (balasa_graeb_survey.txt:8905–8911, p177) | Generalise the transform set (both axes, 180° rotation). Add a connector primitive for crossed pairs |
| AT-22 | medium | dr/src/lib.rs:587–596, 1556–1621; kernel/analog/src/routing/shield.rs:18–19 | Shields are same-layer side tracks only, added after repair and never re-verified after fattening. There are no plates above or below, and no bulk shield (Hastings Fig. 15.28, hastings.txt:48639–48690; survey Fig. 4.16, balasa_graeb_survey.txt:8950–8980) | Model shields as reserved tracks in the search (the victim routes with its shields as a bundle). Add plate shields on adjacent layers at crossings with noisy nets (needs AT-01) |
| AT-23 | medium | dr/src/lib.rs:555, 582, 607–877 | Repair is judged on pre-access, pre-fatten geometry. Access, short breaking, fattening, via arrays, stubs and fill then change what rules score, with no repair after | Re-score after the geometry passes and loop (bounded) back into repair for the nets whose residuals worsened. Or move fattening and access into the search (AT-03) |
| AT-24 | medium | dr/src/lib.rs:1499–1505 | The `_` repair arm reroutes with no field at doubled p_fac. For C and perf budgets it usually reproduces the same path, and for EM it cannot help. Trials are wasted | Budget-specific fields: raise the net's `Elec` weight (ROAD-style weight escalation, balasa_graeb_survey.txt:9440–9449). Skip EM in repair |
| AT-25 | medium | dr/src/lib.rs:469–472; gr/src/lib.rs:738–742, 794–806 | `Differential` nets are exempt from all parasitic and coupling pricing (`plain`) | Symmetrise the field instead: price node n by max(cost(n), cost(mirror(n))) for both halves |
| AT-26 | medium | frontend/library/src/lib.rs:312–316; gr/src/lib.rs:247–248, 855–872 | Search pricing is a normalised per-net scalar, not ΔP from sensitivities (Lampaert eqs. 5.10–5.11, lampaert.txt:6140–6175). Nets without a C budget or perf sensitivity have weight 0. There is no R sensitivity | Carry per-net `(∂P/∂C, ∂P/∂R, ∂P/∂C_k)` from `perf::sensitivities`, extended to R and coupling, into `Elec` in ΔP units, and prune paths past a hard bound (ANAGRAM III, balasa_graeb_survey.txt:9470–9480) |
| AT-27 | medium | benchmarks/tests/signoff_fixtures.rs:15–41; frontend/library/tests/ota_cross_pdk.rs:196–224 | Routing-layer DRC is clean only on 7 small sky130 fixtures. Cross-PDK routing-layer DRC is excluded as "dr debt". There is no dr unit test against a real deck | Make routing-layer DRC 0 a gated test on gf180, ihp and generic_finfet elaborations. Add dr tests that run `verify::drc` on real decks |
| AT-28 | medium | dr/src/lib.rs:2115–2135; kernel/core/src/routes.rs:43–75 | Open detection is layer-blind: xy overlap between different metals without a cut counts as connected | Connectivity by the `joins` relation (same layer touch, or cut with an adjacent metal), as `cross_net_shorts` already models |
| AT-29 | low | dr/src/lib.rs:617–635 | The short resolver deletes the first touching shape in index order and converts the short into an open. Criticality is ignored | Pick the victim shape by net priority (the order from `order_by_priority`). Try a local reroute of the access before deleting |
| AT-30 | low | dr/src/lib.rs:30–31, 1028 | The `GCELLS_PER_SIDE` comment is false (the frames differ). The doc line on `widen` is duplicated | Fix the comments |
| AT-31 | low | gr/src/lib.rs:253, 277; dr/src/lib.rs:534, 811, 1444 | Rule dispatch by `type_name` string suffix | A `RuleBatch::repair_kind()` enum |
| AT-32 | low | gr/src/lib.rs:108–133 | `GroupPrice.reachable` is always true on an obstacle-free gcell grid, and `overflow` is at a meaningless capacity | Price reachability from real blockages (cell metal), or drop the field |
| AT-33 | medium | kernel/analog/src/routing/differential.rs:116–133; frontend/library/src/lib.rs:763–801 | Matched routing covers only diff-pair drains (`Differential`) and shared sources (`CommonNode`). Missing: mirror gate and output nets, cascode pairs, CC capacitor top and bottom plates (routing parasitic mismatch, cc_review.txt:438–453, p6; parallel wires and via counts in CC DACs, cc_dac_constructive.txt:659–666, 790–795, pp. 7–8) | Extract matched-net sets from annotator blocks (mirror, cascode, cap array) and route them through the symmetric/copy machinery (AT-04) with per-layer exact matching (perf_driven_survey.txt:86–89) |
| AT-34 | low | frontend/library/src/lib.rs:632–652 | dr runs twice when diodes are added, and history is accumulated twice per epoch | Reroute only the affected nets, or skip the first accumulate |
| AT-35 | low | — | No yield-driven phase: no wire spreading or bridging-fault critical area when margin remains (Lampaert §5.8.3 eq. 5.15, lampaert.txt:6351–6370, p152) | A post-route "spend margin" pass: spread wires and double vias with the performance bounds held |
| AT-36 | low | docs/CRATES.md:56–63, 97 | Stale doc. Open issue 2 ("Routing EM is not wired") is done: lib.rs:304–311 fills `supply_nets`, `pin_ua` and `em`. "EM width = I / 1 mA/µm" and the field `net_current_ua` no longer exist (EM width is deck-driven, dr:149–153) | Update CRATES.md |
| AT-37 | low | gr/src/lib.rs:527–532 | The lattice pitch is silently raised to `max(die)/1200` on large dies (>~550 µm on sky130, derived), losing track density | Report it. Use a hierarchical or windowed lattice instead of coarsening |
| AT-38 | medium | dr/src/lib.rs:673–674 | The two halves of a differential pair are fattened to their own EM need independently, so unequal currents (or AT-07 inflation) give unequal widths | Use one width per matched set: the max of the members' needs |

---

## 5. Questions the planners must resolve

1. **Routing model.** Keep a track lattice and extend it (per-layer pitch and direction, wrong-way edges, multi-track wires as parallel tracks, as TODAES'22 and the CC-DAC router do), or move to a gridless area router (Lampaert §5.4, ANAGRAM II)? The lattice is simpler to keep DRC-clean. Gridless is what the analog literature recommends for variable width.
2. **Keep or kill `gr`.** Delete it (area-route in dr), or rebuild it as a 3-D, track-capacity global router that also serves as the placement-loop estimator? Today it costs one route per epoch and contributes loose corridors only.
3. **Width planning.** Which stage owns wire width: the annotator (EM, IR, R-sensitivity), a new wire-planning step between gr and dr, or dr? What width grid is allowed on FinFET decks (discrete widths → parallel wires only)?
4. **Symmetry contract with placement.** Will dp snap each symmetry axis to a track or half-track of the routing lattice? Where does dr read the axis from (`Layout::axis` vs terminal centroid)? Are asymmetric pin sets (ABBA) allowed to route with detours, or must placement choose topologies whose pins mirror exactly?
5. **What is hard.** Which router outcomes become V instead of Θ: per-segment EM, via EM, residual overuse, shorts to cell metal, differential mismatch above tolerance?
6. **Over-the-device policy.** Which layers may cross matched devices, per deck (gate keep-out up to metal N, or a full-cover plate)? This needs a deck or cell key; none exists today.
7. **Supply topology inputs.** Should the annotator emit star-node and Kelvin (force/sense) requirements, and net-split intent, from recognised blocks, or should the user declare them? Which layers are reserved for supplies once AT-01 lands?
8. **Sensitivity model.** Extend `perf::sensitivities` to series R and coupling C (currently ground C only; audit-07 notes this) so the router can price ΔP (Lampaert eq. 5.10)? What is the per-epoch simulation budget?
9. **History semantics.** Should PathFinder history reset per placement, decay per epoch, or be keyed relative to cells?
10. **Runtime budget.** What wall time per dr call is acceptable at 200 epochs × 4 outers × 3 starts × 2 topologies? Should full detailed routing run only for promoted candidates, with a cheaper estimate otherwise?
11. **Matched-net extraction.** Which blocks beyond diff pairs must produce matched-route sets (mirrors, cascodes, CC capacitor arrays, CM feedback), and with what metric (per-layer exact vs terminal RC within tolerance)?
12. **Stubs.** Is a dangling stub ever acceptable for matching, or must equalisation be in-path?
