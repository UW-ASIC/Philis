# M2+ cells, segment 6 (M6): CELL-25

Worktree `philis-m2/cells`, branch `m2-cells`, fast-forwarded to `m2` at `705ef6e` (no conflicts).

| Item | Class | Why |
|---|---|---|
| CELL-25 | do | DAG: no hard deps (CELL-10, MAT-03 done in M1). Uses no PLC-06. Two plan points cannot be met as written on generic_finfet (no device extraction, see below); they are replaced by structural checks, not dropped silently. |

Commands (`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first):
`cargo test -p cells finfet` (debug: geometry tests), `cargo test -p cells --release finfet` (adds the DRC/ERC test),
`cargo test -p cells --release --test cell_selfcheck`, `cargo test -p cells --test deck_keys`,
`cargo test -p library --release ota5t_clean_on_generic_finfet`.

---

## CELL-25 FinFET generator parity. Class: do

### Code facts (plan corrections)
- `kernel/cells/src/finfet.rs` (245 lines). Plan line refs hold: `enumerate` 89-94 returns `vec![FinFet]` (unit struct,
  :20); one active per member (139-194, `x` advances by `act_s.max(2·sel_enc - 0)` :193); `Unit { sa: 0, sb: 0 }` :186;
  `nfin = (s.unit_w / fin_p).max(1)` :114 (floor). `phi = +1` for even fingers: region `j` is S iff `j` even (:158).
  Per member: one M1 source strap at `s_strap_y` (:170-173), one M2 drain strap at the V0 centre (:174-177), one M1 gate
  strap at `strap_top` (:189-192). `end` (:119) is used by the device ends and by the tap strip (`cx = tap.x + end/2`, :217).
- Stale: cellgen dispatch is `frontend/library/src/cellgen.rs:799` (plan 795); the debug-assertion note is
  `kernel/cells/tests/cell_selfcheck.rs:330-334` (plan 278-282). "No tests" is half true: finfet.rs has none, but
  `the_mosfet_is_clean_on_every_deck` (cell_selfcheck.rs:335, release) already DRC/ERCs every `FinFet` variant on
  generic_finfet for `(n, nf) ∈ {(1,1),(2,2),(2,2),(1,2)}` plus series cases, and `deck_keys.rs:131` draws it.
- The deck connects M1 and up only and recognises no devices (`pdks/generic_finfet.json` `finfet_note`;
  `pdks/decks/generic_finfet.deck:313-329`). Consequences:
  - "extraction count" is not testable on this deck: replaced by a drawn count (units per member, gates per active).
  - "the LVS card carries the drawn width": there is no MOS LVS on this deck, and no `DrawnKind` for MOS
    (`kernel/core/src/macro.rs:66-72`). `Unit.weight` already uses the drawn height `gate_w · nfin·fin_p` (:186). No
    card change; the rounding only changes `nfin`.
  - GATE has no connectivity, so an uncontacted dummy gate is not an ERC finding (no `floating_gate` on this deck).
- `pattern::diffusion_cc_row(counts, Outer::Drain)` (`kernel/analog/src/matching/pattern.rs:411`): mirror-symmetric
  row, even counts only; region `i` is S iff `i` odd, every member boundary on a source. A shared source between
  members is legal only on one source net: cellgen already enforces that for any merged MOS
  (`cellgen.rs` "different source nets must not share diffusion" test :1789), and `gate_straps_stay_private`
  (cellgen.rs:350) only walks poly, so FinFET gates of different members must never share an M1 strap.
- Deck numbers (`generic_finfet.deck`): fin pitch 27 (FIN.W.1 7 + FIN.S.1 20), gate 20 at pitch 54 (GATE.S.2 34),
  ACTIVE past GATE 25 (GATE.ACTIVE.EX.2), LIG.S.1 18 / eol 25, M1 is `li`, M2 is `met1` in the sidecar map.

### Decisions
- Dummy: one uncontacted gate per diffusion end (ASAP7 diffusion-break style), always (not gated on
  `dummy_required`: on a fin deck the edge gate is part of every active). Not recorded as `pnr_core::Dummy`: that type
  means a bulk-tied, extracted transistor (`macro.rs:106-109`) and `dummy_cards` (cellgen.rs:1004) would add an
  LVS reference card for it. Doc comment says so; if a fin deck ever recognises devices, it needs a tie and a record.
- Sharing: a second variant, never a replacement, so variant 0 stays today's geometry (search indices stable).
- LOD moat (Mosfet's `lod_moat_ext_nm`, mosfet.rs:425) is not ported: out of scope; SA/SB are now reported, so the
  `UnitLib` LOD term sees the mismatch.

### Edits (`kernel/cells/src/finfet.rs` only)
1. Type: `#[derive(Clone)] pub struct FinFet { /// Members interleave on one active ([`pattern::diffusion_cc_row`]).
   pub shared: bool }`. Doc on the struct: separate actives (variant 0) or one shared CC row.
2. `enumerate`: same early return; then
   `let s = group_sizing(group, c, process); let mut v = vec![FinFet { shared: false }];`
   `if s.dev_nf.len() > 1 && pattern::diffusion_cc_row(&s.dev_nf, Outer::Drain).is_some() { v.push(FinFet { shared: true }) }`.
   Imports: `analog::matching::pattern::{self, Outer}` (same path as mosfet.rs).
3. `nfin` (:114): `let nfin = ((s.unit_w + r.fin_p / 2) / r.fin_p.max(1)).max(1);` comment: nearest fin count, at
   least one; the drawn `nfin·fin_p` is what `Unit.weight` carries.
4. Rows: replace the per-member loop (:136-194) by a loop over rows `(owners: Vec<usize>, track: fn owner -> i32)`:
   - separate: one row per member `vec![di; nf]`, track 0 for all; region parity `s_odd = false` (today's).
   - shared: one row `diffusion_cc_row(&s.dev_nf, Outer::Drain).unwrap()`, track of member `k` = `k`; `s_odd = true`.
   Per row with `n = owners.len()`:
   - Active: `w = 2·act_past_gate + (n+2)·gate_w + (n+1)·gap`; gate `i ∈ -1..=n` at
     `gx(i) = a.x + act_past_gate + (i+1)·gate_p`; `-1` and `n` are the dummies (gate rect only, `gate_lo_min..gate_hi`,
     no LIG/V0). S/D region `j ∈ 0..=n` spans `gx(j-1)+gate_w .. gx(j)`: every region is `gap` wide, contacted as today.
   - Region `j` is S iff `(j % 2 == 1) == s_odd`. S: M1 strip up to the one source strap of the row; push to `s_x`.
     D: owner = `owners[j.min(n-1)]` if `j == n` else `owners[j]` (equal to `owners[j-1]` by legality); M1 strip from
     `sd_m1_y` to its track pad top; V1 + pad at `y_d(t) = v0c + t·(m2_w + space("met1"))`; push `(owner, cx)`.
   - Source strap: one M1 strap over the row's `s_x`; pin `d{k}:S` for every owner in the row at the first source strip
     (same `at` for all: `shared_pads_carry_one_net` then requires one net, which the shared diffusion already does).
   - Drain: per owner one M2 strap at `y_d(track)` over its drain xs, pin `d{k}:D` at its first V1.
   - Gates: per owner `k`, gate track `t`: `strap_top_t = strap_top - t·gpitch` with
     `gpitch = (m1_w + m1_s.max(eol_space("li"))).max(lig_h + space("lig").max(eol_space("lig")))`; its gates' LIG/V0 at
     that track, poly down to its `lig_y_t`; one M1 strap per owner over its own gate V0s; pin `d{k}:G`.
     `gate_lo_min` = the lowest track's `lig_y` (select bottom, dummy gate bottom).
   - `d_top` (:126) becomes the max over tracks of `y_d(t) + v1_pad/2` (and the M1 strip tops), so `s_strap_y` clears
     every drain pad.
   - Units: per real gate `i`, `owner = owners[i]`, `phi = (if (i % 2 == 0) != s_odd { 1 } else { -1 }, 0)` (left region
     S ⇒ +1), `sa = gx(i) + gate_w/2 - a.x`, `sb = a.x + a.w - (gx(i) + gate_w/2)`.
   - `x = a.x + a.w + r.act_s.max(2 * r.sel_enc)` (drops the `- 0`; CELL-29 lists it, harmless to do here).
5. Selects, tap strip, well (:195-237): unchanged except `gate_lo` → `gate_lo_min`.

### Tests (`kernel/cells/src/finfet.rs`, new `#[cfg(test)] mod tests`, deck `verify::Pdk::builtin("generic_finfet")`, groups from `crate::testkit::group_of`)
- `nfin_rounds_to_the_nearest_fin`: Nmos n=1 nf=1, `w ∈ {10, 40, 41, 1680}` → units[0].weight
  `== 20·27·{1, 1, 2, 62}` (40/27 = 1.48 → 1; 41/27 = 1.52 → 2; 10 → at least 1).
- `every_active_has_one_dummy_gate_per_end`: n=2 nf=2 Nmos, both variants: GATE shape count `== units.len() + 2·actives`
  (actives = ACTIVE shapes minus the tap's one): separate 4+4, shared 4+2.
- `sa_and_sb_reach_the_active_ends`: every variant of n=2 nf=2 and n=1 nf=3: every unit `sa > 0 && sb > 0`, and
  `sa + sb` equals the width of the ACTIVE rect containing `(u.x, u.y)`; per member `Σ sa == Σ sb` (mirror-symmetric rows).
- `the_shared_row_is_common_centroid` (T6): counts `[2,2]`, `[2,4]`, `[4,4]` via `group_of` + edited `dev_nf`
  (`target_ratio` = `dev_nf`): `enumerate` offers `shared: true`; for its units, per member
  `Σx_k · N == Σx_all · n_k` (integer), and `Σ phi_k == 0` per member; units per member `== dev_nf[k]`.
  Counts `[1,1]` offer only `shared: false`.
- `every_variant_is_drc_and_erc_clean` (`#[cfg(not(debug_assertions))]`, cell_selfcheck.rs:330's engine note): Nmos and
  Pmos, `testkit::dirty_group_with::<FinFet>(&g, &c, &pdk, &["S", "B"])` over `[1]`, `[2,2]`, `[2,4]`, `[4,4]` at
  w=1680, l=20 → `assert!(dirty.is_empty(), ..)`. Private drains and gates make a cross-member short an ERC finding.
- Unchanged and must stay green: `the_mosfet_is_clean_on_every_deck` (now also sweeps the shared variant at (2,2)),
  `deck_keys`, `ota5t_clean_on_generic_finfet` (library, release).

### Acceptance
T1 on generic_finfet for pairs: the DRC/ERC test above plus the existing sweep, 0 findings. T6 for pairs: the centroid
test. If the track stack trips a LIG or V0 rule the sweep reports it with its rule name; fix `gpitch`, never the test.

### Risks
- V0.S.1 (27 nm across unaligned tracks, deck :212) between adjacent gates on different tracks: the DRC test decides.
- `testkit::pdk()` is sky130 (lib.rs:63); `dirty_group_with` takes the deck as a parameter, so pass the builtin generic_finfet.
