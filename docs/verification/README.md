# Verification engineering guide

This directory is the single source of truth for developing and qualifying the
repository's DRC, LVS, PEX, ERC, and reliability-signoff stack. It describes what
the code does today, what remains before a proprietary-grade qualification attempt,
and the evidence required to change a capability claim.

The current audited score is **24.5 / 112 capability families (22%)**: DRC
12.0/44, LVS 8.0/36, and PEX 4.5/32. That score is deliberately frozen. New code,
unit tests, and local conformance results establish a foundation; they do not count
as independent correlation or foundry qualification.

## Start here

Read these pages in order before choosing work:

1. [Current status](status.md) — settled results, accepted foundations, and
   the exact remaining roadmap.
2. [Architecture and source map](architecture.md), then
   [inputs, decks and APIs](inputs-and-api.md).
3. [Verification contracts](contracts.md) — units, identity, provenance, and the
   permanent fail-closed rules.
4. [Contributor reading order and ownership](contributing/reading-order.md), then
   [worktrees, test gates and golden policy](contributing/test-gates.md).
5. Engine boundaries: [geometry/layout](engines/geometry-layout.md),
   [DRC](engines/drc.md), [LVS](engines/lvs.md), [PEX](engines/pex.md), and
   [reliability signoff](engines/signoff.md).
6. Compliance evidence: [capability matrix](compliance/capability-matrix.md),
   [known limitations](compliance/known-limitations.md),
   [bug dispositions](compliance/bug-dispositions.md), and
   [correlation/qualification](compliance/qualification.md).
7. [Implementation roadmap](compliance/roadmap.md) — dependency order and detailed
   Wave 2–7 work packages.

The machine-readable score record is
[`compliance/capabilities.json`](compliance/capabilities.json), validated by
[`compliance/capabilities.schema.json`](compliance/capabilities.schema.json) and the
strict cross-record checker
[`scripts/validate_capabilities.py`](scripts/validate_capabilities.py).

## What these documents do not claim

“Proprietary-grade” is not a vendor-neutral feature checkbox. A usable signoff
claim is scoped to an exact engine revision, process, foundry deck, model set,
input/output subset, and qualification corpus. It requires independent golden-tool
correlation and acceptance by the foundry or tapeout owner. This repository does
not currently contain those external licenses, decks, golden results, or approvals.

Local conformance, KLayout detection agreement, and passing unit tests are useful
regression evidence. None is a substitute for marker, topology, value, capacity,
and full-chip correlation against the target qualified flow.

## Repository boundary

Verification implementation lives in [`verify/`](../../verify/). The backend is a
downstream consumer whose compile and behavior gates live in [`backend/`](../../backend/).
These pages replace only the former verification/compliance notes. Books, research,
design assets, and all other top-level documentation remain independent.
