# LVS engine

## Reference side

[`lvs/netlist.rs`](../../../verify/src/lvs/netlist.rs) implements a strict SPICE/CDL
foundation with source spans, engineering numbers, parameter expressions, models,
subcircuits and caller-resolved includes. [`lvs/binding.rs`](../../../verify/src/lvs/binding.rs)
adds deterministic parameter evaluation, configured model binding, black-box/equated
cells and bound hierarchy records. Unknown/unsupported constructs fail parsing/binding.

## Layout side

Legacy extraction in [`lvs/extract.rs`](../../../verify/src/lvs/extract.rs) uses exact
supported rectilinear overlap/contact after bbox pruning, declared via relations and
configured device recognition. [`lvs/detailed_extract.rs`](../../../verify/src/lvs/detailed_extract.rs)
produces production identities, body/well terminals, typed properties, soft/open
evidence and hierarchy paths.

[`lvs/gds_adapter.rs`](../../../verify/src/lvs/gds_adapter.rs) provides the accepted
checked adapter for the declared strict orthogonal GDS subset. It preserves explicit
text/property and hierarchy provenance, validates child/opaque-boundary port maps,
and correlates nonopaque hierarchy against strict physical flattening. Unsupported,
ambiguous or incomplete evidence errors; a synthetic root or invented port is invalid.

## Comparison and hierarchy

[`lvs/production.rs`](../../../verify/src/lvs/production.rs) defines detailed layout/
reference records, named net/port identities, legal terminal states, typed tolerances,
deterministic mappings and actionable topology/property witnesses. Legacy comparison
remains for compatibility. [`lvs/hier_production.rs`](../../../verify/src/lvs/hier_production.rs)
provides abstract hierarchy, transforms/arrays, policies and cache foundations.

## Production gap

Complete the selected SPICE/CDL/Spectre dialect; extend checked GDS semantics beyond
the declared subset;
implement exact all-angle/derived connectivity and soft-connect; extract foundry MOS,
R, C, diode, BJT and selected custom devices/properties; finish deterministic bounded
matching, named seeds, legal swaps and symmetric reductions; implement true child-port,
black-box/equated/selective-flatten semantics; prove flat/hier equivalence and capacity.

Every mismatch must reference both schematic and layout objects plus a reproducible
witness. See [Wave 4](../work-packages/wave-4.md).
