# M2+ cells, segment 10 (M6): GAP-19

Base: `m2-cells` after merging `m2` at `1df326d`. One item.

## GAP-19 Minimize drain or source by node impedance — class: do

Spec: `98-gap-critic.md` §GAP-19 (H12-29, Hastings §12.2.8). DAG: cells, M6, hard dep EXT-18 (merged).

### Current code facts (plan corrections)

- Plan cites `kernel/cells/src/mosfet.rs:324-328` for the source-outer start: **stale**. The choice is
  `mosfet.rs:463-465`: `let s0 = if self.mirror_pins { true } else { legal_row(sequence, true) };`.
  For one device `legal_row` has no boundary to check and returns true, so region 0 is always S. With even
  `nf` both end regions are S: S gets `nf/2 + 1` regions, D `nf/2` (drain minimized). Odd `nf` already
  puts S at one end and D at the other, so it has nothing to choose.
- `Mosfet` (`mosfet.rs:44-56`) has no outer-electrode field. Struct literals: `mosfet.rs:96`, `:165` and the
  tests `:1406, 1412, 1509, 1526` (8 in total, all in this file).
- `Mosfet::enumerate` (`mosfet.rs:77-169`): series stacks return early (`:89-98`). A single device reaches
  `styles = [(Single,false,false)]`. `Cc1d` needs ≥2 devices, and split gates need `n_dev > 1`. So a single
  device gets `rows ∈ {1, 2 (nf even, nf ≥ 4)}` × `double_gate ∈ {false, true if wide}`.
- The dummy near terminal (`mosfet.rs:662`, `ends = [(.., term(0,false)), (.., term(n-1,true))]`), the pins
  (`:629-632`) and `Figures.sd` (`:592-633`) all derive from `is_s`/`term`. They follow `s0` with no
  further edit, and so do the LVS dummy cards (`cellgen.rs:1076`, `d.edge`) and the AS/AD values.
- EXT-18 is in: `NetFacts.z_ohm: Option<f32>` (`kernel/analog/src/intent.rs:237`), filled by
  `backend/annotator/src/classify.rs:275-312`. In the flow it is **always None** (PERF-09 has no gds,
  `classify.rs:582`). `NetClass::Bias` exists (`kernel/analog/src/metadata.rs:26-27`).
- cellgen: `enumerate_folded(netlist, macros, constraints, pdk, merge_distinct_gates, fold, net_classes)`
  (`frontend/library/src/cellgen.rs:63-71`) sees the classes but not `Intent.nets`. Its only flow caller,
  `frontend/library/src/lib.rs:2140`, has `problem.intent: analog::intent::Intent`
  (`backend/annotator/src/lib.rs:73`). `enumerate` (`cellgen.rs:55`) passes `&[]`. Single devices are drawn in
  phase 2 (`cellgen.rs:186-199`). The CELL-18 precedent is `plate_rank`/`bottom_on_p` (`cellgen.rs:223-244`).
- FinFET (`cells::finfet`, `cellgen.rs:862`) is out of scope: it is a separate generator and sky130 does not use it.

### Rule (pins down "lower z_ohm, or Supply/Ground/Bias")

A net is **low-Z by class** when its class is Supply, Ground or Bias. For a single MOS with source net `s`
and drain net `d`, draw the **drain outer** (minimize the source) iff `s` is not low-Z by class and either:
1. `d` is low-Z by class (follower, or drain on a rail: the drain's junction C sits on an AC ground), or
2. `z_ohm(d)` and `z_ohm(s)` are both `Some`, and `z_ohm(d) < z_ohm(s)`.

Every other case keeps today's source-outer convention, including the case where `z_ohm` is missing on either
net. cellgen always keeps **one** convention per device, so a design the rule does not touch gets the
same variant lists and indices as today, and its bench lex keys do not change.

### Edits

1. `kernel/cells/src/mosfet.rs`
   - `Mosfet` gains a field, documented as:
     `/// Even-finger single device: D at both ends, S minimized (GAP-19; Hastings §12.2.8). cellgen keeps this or the default by node impedance.`
     `pub drain_outer: bool,`
   - `enumerate`: add `drain_outer: false` to both literals. Then, at the end of the non-series path, for
     `n_dev == 1`, append a `drain_outer: true` copy of each variant whose per-row finger count is even:
     `v.nf / v.rows % 2 == 0 && !v.mirror_pins`. Appended after the existing variants, so their indices stay stable:
     ```rust
     let mut out: Vec<Self> = /* existing chain */ .collect();
     if n_dev == 1 {
         let flipped: Vec<Self> = out.iter().filter(|v| v.nf / v.rows.max(1) % 2 == 0).map(|v| Mosfet { drain_outer: true, ..v.clone() }).collect();
         out.extend(flipped);
     }
     out
     ```
   - `draw_row:463`:
     `let s0 = if self.mirror_pins { true } else if self.drain_outer { false } else { legal_row(sequence, true) };`
     The `debug_assert` at `:466` still holds, because `legal_row` is trivially true for one device.
   - Test literals `:1406, 1412, 1509, 1526`: add `drain_outer: false`.
   - New `pub fn outer_terminal(m: &Macro) -> Option<&'static str>` beside `taps_in_reach`, documented as:
     `/// "S" or "D": the terminal of the leftmost S/D pin of member 0 (region 0), None for a macro without one.`
     Body: `m.pins.iter().filter(|p| p.name == "d0:S" || p.name == "d0:D").min_by_key(|p| p.at.x).map(|p| if p.name.ends_with('S') { "S" } else { "D" })`.
     cellgen needs it because a `Macro` does not carry its `Mosfet` parameters.
2. `frontend/library/src/cellgen.rs`
   - `enumerate_folded` gains a last parameter `nets: &[analog::intent::NetFacts]` (indexed by `NetId`;
     empty = no impedance facts). Extend the doc comment: "and a single even-finger MOS puts its
     lower-impedance S/D outer (GAP-19, [`drain_outer`])". `enumerate` passes `&[]`, and
     `lib.rs:2140` passes `&problem.intent.nets`.
   - New helper next to `bottom_on_p`:
     ```rust
     /// Whether single MOS `d` draws its drain outer (GAP-19, H12-29): its source is not a rail or bias
     /// net, and its drain is one, or both carry `z_ohm` (EXT-18) and the drain's is lower.
     fn drain_outer(net_classes: &[NetClassification], nets: &[NetFacts], d: &Device) -> bool
     ```
     It uses `low = |n| net_classes.iter().any(|c| c.net == n && matches!(c.class, Supply | Ground | Bias))` and
     `z = |n| nets.get(n.0 as usize).and_then(|f| f.z_ohm)`. With `(Some(s), Some(dn)) = (terminal(d,"S"), terminal(d,"D"))`:
     `!low(s) && (low(dn) || matches!((z(dn), z(s)), (Some(a), Some(b)) if a < b))`. Any other shape returns false.
   - Phase 2, single-device branch (`:186-199`): for `DeviceKind::Nmos | Pmos`, after `draw_variants`, run
     `let want = if drain_outer(net_classes, nets, d) { "D" } else { "S" };`
     `alternatives.retain(|m| cells::mosfet::outer_terminal(m).is_none_or(|t| t == want));`
     If that would leave the list empty, keep the drawn list as it was. Odd `nf` has no D-outer form, so the
     retain only filters even-`nf` lists, and their S-outer forms always exist.
     The empty-placeholder `Macro::default()` has no pins, and `is_none_or` keeps it.
   - Merged groups (phase 1) are untouched: `drain_outer` is only enumerated for `n_dev == 1`.

### Tests

- `kernel/cells/src/mosfet.rs` tests, `fn drain_outer_swaps_the_minimized_electrode`. It uses
  `testkit::pdk()`/`testkit::group_of(DeviceKind::Nmos, 1, 4, 1680, 150)` as the neighbours do.
  - `enumerate`: `vs.iter().filter(|v| v.drain_outer).count() == vs.iter().filter(|v| !v.drain_outer && v.nf / v.rows % 2 == 0).count()`,
    and that count is ≥ 1. Each `drain_outer` variant also appears in the list as its `!drain_outer` twin.
  - Draw `rows: 1` both ways. Default: `outer_terminal == Some("S")`, and in `figures.sd` owner 0 `S` area > `D` area.
    `drain_outer`: `Some("D")`, and `D` area > `S` area. The swapped areas are equal (`S_default == D_flipped`,
    `D_default == S_flipped`), which shows it is the same geometry relabelled.
  - Dummies: drawn with `dummies_per_edge: 1`, every `m.dummies` near edge is `"D"`.
  - `group_of(.., 1, 3, ..)` (odd) and a 2-device group: no variant has `drain_outer`.
- `frontend/library/src/cellgen.rs` tests, `fn a_follower_minimizes_its_source`. It builds a netlist with nets
  `vdd, vss, g, out, mid` and a single NMOS (`nf = 2`). It calls `enumerate_folded(&nl, &Macros::default(),
  &Constraints::default(), &pdk(), true, &folds(&nl, &pdk, &[], &[]), &classes, &nets)` with
  classes vdd Supply and vss Ground, and checks the outer terminal of every alternative of cell 0:
  - follower `D=vdd G=g S=out B=vss`, no `z_ohm`: all `Some("D")`. In `figures.sd`, owner 0 `S` area < `D` area (the acceptance check).
  - common source `D=out S=vss`: all `Some("S")` (source on a rail).
  - `D=mid S=out`, no classes on those two, with `z_ohm(out)=1e5, z_ohm(mid)=1e3`: all `"D"`. Swapped values: all `"S"`.
    With `z_ohm(mid)` set to `None`: all `"S"` (today's convention).
  - The variant count of the follower cell equals that of the common-source cell (one convention kept).
  `NetFacts` literal: `{ evidence: EvidenceLevel::Default, port: false, dc_mv: None, rc: RcClass::Unknown, z_ohm, shield_ref: None }`.
- Commands (`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`):
  `cargo test -p cells drain_outer`, `cargo test -p library a_follower_minimizes_its_source`, then
  `cargo test -p cells` and `cargo test -p library` in full. Struct literals and the enumerate order changed,
  and the existing variant-index tests must stay green unedited.

### Acceptance and notes

- Acceptance (`Figures.sd` shows the smaller S/D area on the higher-impedance terminal) is checked by the follower case above.
- In the flow, `z_ohm` is None until PERF provides gds. Only the class rule (rule 1) can fire on the benches. Any bench
  device with a rail or bias drain, a non-rail source and even `nf` changes its variant. If a bench lex key moves,
  report it as an expected consequence; do not loosen the key to absorb it.
- Out of scope: FinFET generator and merged/matched groups (their conventions are fixed by `legal_row`).
