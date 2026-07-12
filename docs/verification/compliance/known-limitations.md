# Known limitations and explicit unsupported boundaries

This page is the quick rejection guide. A contributor must preserve these boundaries
as explicit errors until the owning work package replaces them with exact semantics.

## Geometry and layout

- Exact booleans are currently rectilinear. Arbitrary-angle boolean/offset operations
  are unsupported; bboxes are candidate filters only.
- Checked GDS verification does not accept every legal GDS construct. Unsupported
  cases include NODE for geometry, absolute STRANS semantics, non-orthogonal or
  non-integral transforms, and round/diagonal/odd/negative-width PATH semantics.
- OASIS support declares a narrow subset. PATH, TEXT, repetitions/arrays, properties,
  modal/reference tables, relative coordinates, placement transforms, CBLOCK and
  compressed blocks require Wave 2 work unless the current capability declaration
  explicitly says otherwise.
- A lossless record round trip is not automatically a valid verification adapter.
  Verification requires units, geometry validation, supported transforms, visible
  unmapped/unsupported records, and preserved evidence identity.

## DRC

- General conditional/table-driven rule execution is incomplete. Simplified PRL,
  EOL, wide-spacing, enclosure, cut, density and antenna variants are not a foundry
  rule language.
- Legacy overlap, hole/slot interpretation, and general-polygon heuristics are known
  false-clean risks until the reviewed Wave 3 fixes land.
- The Wave 3 derived-layer/coloring/result work is integration pending; seven defect
  classes listed in [status](../status.md) block acceptance.
- Foundry-calibrated fill/CMP, voltage/region/cell/hierarchy contexts, advanced-node
  mask/device checks, and deterministic distributed execution remain incomplete.

## LVS

- The checked GDS hierarchy/text/property to production `HierLayout` adapter now
  supports the declared strict orthogonal subset with explicit evidence and physical
  flatten correlation. Unsupported transforms/records and ambiguous or incomplete
  bindings remain typed errors; local correlation is not independent golden evidence.
- Spectre and broader dialect constructs, `.lib` conditions/functions, full model
  aliasing, and production include policies remain incomplete.
- Foundry-complete MOS/R/C/diode/BJT/custom recognition and properties are absent;
  actual diffusion AD/AS/PD/PS, broad legal pin swaps, and cross-boundary reductions
  need explicit implementation/correlation.
- Black-box/equated-cell and selective-flatten foundations are not yet qualified for
  arbitrary mixed hierarchies or million-instance designs.

## PEX

- Results are analytical scalar/per-net estimates, not a terminal-aware distributed
  conductor/via graph. Branch topology and terminal-to-terminal resistance can be wrong.
- There is no qualified dielectric/process-stack corner engine, field solver, calibrated
  shield/fill behavior, general device/substrate network, inductance, frequency/skin/
  proximity effects, or electrothermal extraction.
- Current SPICE export is not validated DSPF/SPEF signoff output. Multi-corner,
  selected-net, and topology-preserving reduction remain Wave 5.

## Reliability signoff

- Wave 0 analyzers consume caller-supplied typed evidence. They do not yet derive
  fabrication antenna networks, power currents/thermal maps, voltage histories,
  ESD paths/sizing, or substrate/well/guard-ring evidence from design inputs.
- Analyzer cleanliness is not device-level HBM/CDM qualification, foundry model
  acceptance, or silicon reliability certification.

## Qualification and capacity

- The repository has no licensed vendor adapters, selected foundry deck/model set,
  independent field-solver/foundry golden corpus, approved error bounds, representative
  full-chip capacity envelope, organizational signatures, or foundry/tapeout approval.
- The neutral correlation harness validates exchange structure and deterministic
  comparison. It does not establish correctness of the producing tool or parity.
