# M2 reliability, segment 1: GAP-06

Branch `m2-reliability`, base `m2` @ `c2940c6` (merge: already up to date). Spec: `98-gap-critic.md` "### GAP-06",
master plan §6 C7 and the registry table row (L757). DAG: module reliability, M2, `hard: []` (the spec's "Depends on
FLOW-04" is satisfied: `Kind::Table` and the registry exist since M0).

## GAP-06 EM derating: sidecar `em_derating` table and its reader. Class: do

### Current code (corrections to the plan)

- `EmLimit` is `backend/verify/src/pdk.rs:24-35` (fields `ua_per_um`, `ua_per_cut`, `blech`, `derating`); it is built
  only in `Pdk::em_limit`, **`pdk.rs:625-665`** (plan said 525-547). `derating` is the deck triple at **`pdk.rs:648`**
  (`reference_temperature`, `activation_energy_ev`, `current_exponent`, read through the `ratio` closure); the two
  cut paths (`:655`, `:664`) copy it with `..e`, so a cut inherits its metal rule's derating.
- No shipped deck carries those params: sky130's EM rules are `pdks/decks/sky130.deck:506-510` (plan cited
  `GP/pdks/sky130.deck:491-495`; the vendored decks live in `pdks/decks/`). So `derating` is `None` everywhere.
- Consumer: `frontend/library/src/elaborate.rs:241-255` `em_limits` derates via `analog::routing::em::derate`
  (`kernel/analog/src/routing/em.rs:19`, `F ≤ 1`, never credits cooler); `frontend/library/src/lib.rs:428` passes
  `temp_c + 273.15` (plan's `lib.rs:280` is stale).
- Registry `backend/verify/src/sidecar.rs:62-…` (`KEYS`, alphabetical). FLOW-06's `em_ref_temp_c` /
  `em_activation_ev` / `em_current_exponent` rows **do not exist** (grep: no hits in `.rs`/`.json`), so nothing to
  delete. `sidecar::validate` (`:147-189`) only checks a `Table` is an object; the reader checks the shape.
- Provenance convention is a sibling `<key>_source` text (`sidecar.rs:192`, `Pdk::provenance`), not an inner
  `"_source"` field as the spec sketches. **Decision:** use `cell.em_derating_source` (row `sourced = true`), so
  `validate` enforces the source and `Pdk::unverified` works unchanged.
- Source lines (vendored decks): gf180 "Electromigration at 85 C" is **`pdks/decks/gf180mcu.deck:486`** (plan: 478);
  ihp "11 years at 105 C" is **`pdks/decks/ihp_sg13g2.deck:601`** (plan: 599); sky130 "Iavg_max at Tj=90C" is the
  tech LEF `sky130_fd_sc_hd__nom.tlef:92,119`, already quoted in `pdks/sky130.json:108`.

### Scope decision

A reader alone is inert: every table has `ea_ev`/`n` null, so the spec's own test (`derating_assumed == false`) and
REL-05's `sky130_em_rating_is_90c_with_fallback_parameters` need REL-05 step 4 (fallback 0.9 eV / n 1.1 and
`EmLimit::derating_assumed`). Both are ~10 lines in the same function, same module, so **GAP-06 lands REL-05 steps 4–5
and that test**; the REL-05 card then marks steps 4–5 done and keeps steps 1–3, 6.

### Edits

1. `backend/verify/src/sidecar.rs` `KEYS`, alphabetical after `dummy_max_l_nm`:
   `k("em_derating", Table, false, true, "backend/verify/src/pdk.rs em_limit (when the deck rule states no Black parameters)"),`
2. `pdks/sky130.json`, `pdks/gf180mcu.json`, `pdks/ihp_sg13g2.json` `cell` (next to `em_current_density_source`):
   - sky130: `"em_derating": {"t_ref_k": 363.15, "ea_ev": null, "n": null}`, `"em_derating_source": "Iavg_max at Tj=90C, sky130A/libs.ref/sky130_fd_sc_hd/techlef/sky130_fd_sc_hd__nom.tlef:92,119 (volare fa87f8f4); Ea and n not stated by the PDK"`.
   - gf180mcu: `t_ref_k 358.15`, source `pdks/decks/gf180mcu.deck:486 "Electromigration at 85 C" [DRM 54, tables 14.3/14.4]; Ea and n not stated`.
   - ihp_sg13g2: `t_ref_k 378.15`, source `pdks/decks/ihp_sg13g2.deck:601 [PS] §2.15 "11 years at 105 C"; Ea and n not stated`.
   - generic_finfet: none (synthetic deck, no rating temperature).
3. `pdk.rs` `EmLimit`: add
   ```rust
   /// `derating`'s Ea and n are Philis's fallback (Lienig Cu, 0.9 eV, n 1.1),
   /// not the process's: only `T_ref` came from `cell.em_derating`.
   pub derating_assumed: bool,
   ```
   and extend the `derating` doc: deck rule first, else `cell.em_derating`.
4. `pdk.rs`, new private method on `Pdk` next to `em_limit`:
   ```rust
   /// `cell.em_derating` (GAP-06) as `((T_ref K, Ea eV, n), assumed)`; `None`
   /// without a `t_ref_k`. A null Ea or n takes Lienig's Cu values at the most
   /// conservative stated n (0.9 eV, 1.1; the largest Ea/n, so the strongest
   /// derating above T_ref) and is `assumed`.
   fn sidecar_derating(&self) -> Option<((f32, f32, f32), bool)> {
       let t = self.cell.get("em_derating")?;
       let f = |k: &str| t.get(k)?.as_f64().map(|v| v as f32);
       let (ea, n) = (f("ea_ev"), f("n"));
       Some(((f("t_ref_k")?, ea.unwrap_or(0.9), n.unwrap_or(1.1)), ea.is_none() || n.is_none()))
   }
   ```
   In `em_limit`'s `of` closure: `let deck = (|| Some((ratio(..)?, ..)))();` then
   `let (derating, derating_assumed) = match deck { Some(d) => (Some(d), false), None => self.sidecar_derating().map_or((None, false), |(d, a)| (Some(d), a)) };`.
   Doc of `em_limit`: add "Black's parameters from the rule, else `cell.em_derating`".
5. Shape check (so a typo is not a silent `None`): in `Pdk::load` after `bad.extend(crate::sidecar::validate(..))`
   (`pdk.rs:251`), if `cell.em_derating` is an object, push an error for any key outside `t_ref_k`/`ea_ev`/`n` or any
   value that is neither null nor a positive number.

### Tests (`backend/verify/src/pdk.rs` `mod tests`, helpers `load`, `id`)

- `sky130_em_rating_is_90c_with_fallback_parameters`: `load("sky130").em_limit(id(&sky,"met1"))` has
  `derating == Some((363.15, 0.9, 1.1))` and `derating_assumed`; the cut `mcon` the same (inherits via `..e`);
  `gf180mcu` metal1 `.derating.unwrap().0 == 358.15`, `ihp_sg13g2` metal2 `== 378.15`, both `derating_assumed`.
- `a_deck_rule_value_beats_the_sidecar`: sky130 deck text with `"max_current_per_cut: 0.36mA)"` replaced by
  `"max_current_per_cut: 0.36mA, reference_temperature: 373.15, activation_energy_ev: 0.85, current_exponent: 2)"`
  (same pattern as `em_limits_are_read_per_layer_from_the_deck_rules`, `Pdk::load(&text, &sidecar)`): met1
  `derating == Some((373.15, 0.85, 2.0))`, `!derating_assumed`; met2 (untouched rule) still the sidecar
  `(363.15, 0.9, 1.1)`. If gdsverify rejects or mistypes those params on `em_current_density` (unverified: no deck
  uses them), report it; do not weaken the assertion.
- `em_derating_shape_is_checked`: sky130 sidecar with `"em_derating": {"t_ref": 363.15}` → `Pdk::load` errs naming
  `em_derating`; with `t_ref_k: -1` → errs.
- Existing `sidecar_rejects_a_misspelt_key` and the "unsourced" check cover the source: drop `em_derating_source`
  → load error (add one assert to that test or the shape test).
- Acceptance, `frontend/library/src/elaborate.rs` `mod tests`, `sky130_met1_em_limit_derates_at_125c_not_at_27c`:
  `let p = Pdk::builtin("sky130")`, met1 id from `p.layers`; `em_limits(&p, &[m1], &[], Some(398.15))[0].1.ua_per_um`
  within `281.2 ± 1.0` (2800 × exp[(0.9/(1.1·8.617e-5))(1/398.15 − 1/363.15)] = 2800 × 0.1004, computed);
  `Some(300.15)` → exactly `2800.0`.
- Command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p verify em_ && cargo test -p verify sidecar && cargo test -p library elaborate`, then `cargo test -p verify` in full (loads every builtin PDK).

### Risks

- Behaviour change: at `temp_c > 90` sky130 EM limits now derate (strongly, ×0.10 at 125 °C), so a hot-corner flow
  can newly report EM findings. That is the intended correction (H15-38); bench runs at 27 °C are unaffected (F = 1).
- Al processes are over-derated by the Cu fallback (up to 3.7× at 125 °C, REL-05 risks; §5 Q1 open).
