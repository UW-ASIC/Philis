# Verification contracts

These rules are architectural invariants, not implementation preferences.

## Permanent merge policy

- A checker returns `CLEAN` only with complete valid inputs and evaluated scope.
- Every rule and model has a stable unique ID and explicit units.
- Unknown syntax, layers, devices, properties, units, or unsupported geometry are
  errors; they are never skipped checks.
- A bounding box is a conservative candidate filter only. It cannot establish
  contact, containment, width, overlap, electrical connectivity, or a clean result.
- Every bug fix adds a minimal reproducer and a negative-direction test.
- Golden deltas require an engineering disposition. Expected files are not changed
  merely to make a test green.
- Capability scores change only when a family works on general supported input and
  has independent correlation evidence.

## Data contract

Coordinates remain integer database units through parsing and geometry. A deck-aware
GDS load requires `UNITS` and rejects a mismatch with `deck.dbu_nm`; it does not
silently rescale. Physical quantities carry dimensions and units at API and artifact
boundaries. Never compare unlike quantities or infer units from a field name.

Stable identity must survive adapters: rule/model ID, source format record, GDS
layer/datatype, cell and instance hierarchy path, polygon/text/property provenance,
net/device/terminal identity, stimulus, process/corner, model revision, and limit
revision. An adapter may report ambiguity or `Unsupported`; it may not invent a port,
net label, body pin, process coefficient, or waiver.

Geometry APIs distinguish validated polygon topology from raw records. Unknown or
unrepresentable records stay visible in a lossless representation or produce a typed
error in the checked verification adapter. A private approximation must not downgrade
a typed `Unsupported` to an empty result.

## Result contract

Signoff aggregation uses four states:

| State | Meaning |
|---|---|
| `CLEAN` | complete required evidence evaluated; no violations |
| `VIOLATIONS` | complete enough to evaluate; one or more limits fail |
| `NOT_RUN` | required analysis/configuration/evidence was not supplied |
| `ERROR` | malformed, ambiguous, unsupported, singular, or incomplete evaluation |

`all_clean()` is true only when every required family is `CLEAN`. Compatibility APIs
may preserve legacy behavior if their names and docs make the policy explicit; the
signoff path always chooses checked, strict APIs.

## Determinism and provenance

Equal frozen inputs produce byte-stable ordering and artifacts. Parallelism, tiles,
hierarchy-preserving and flattened paths, restart, and incremental execution must not
alter semantic results. Waivers and invalidation keys include sufficient source and
model provenance that an edited geometry, ancestor, deck, or limit cannot inherit a
stale clean/disposition.
