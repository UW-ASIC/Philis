# PEX engine

## Current analytical model

[`pex/mod.rs`](../../../verify/src/pex/mod.rs) extracts supported rectilinear conductors
using configured per-layer scalars:

| Mechanism | Current formula |
|---|---|
| conductor resistance | `Rs * Leq / Weq`, with Manhattan area/perimeter equivalent geometry |
| ground capacitance | `Ca * area + Cf * perimeter` |
| lateral coupling | `Ck * parallel_run * reference_spacing / spacing` |
| interlayer coupling | configured coefficient times bbox overlap area |
| via resistance | configured fixed resistance per cut polygon |

Unsupported R/ground-C geometry emits an extraction diagnostic;
`run_pex_by_net_checked` makes it blocking. Fill/shield effects are not guessed from
shape size or layer names. Port-connected lumped export avoids dangling parasitic nodes.

The 386 aF regression is physically decomposed and must remain so: **10 aF area +
176 aF fringe + 200 aF mutual coupling = 386 aF**. A total-only expected value is
not sufficient evidence.

## What the result means

Current resistance is a scalar per-net/polygon estimate, not a conductor/via node
network solve. Interlayer/lateral capacitance is an analytical geometric approximation,
not a calibrated field solution. Rudimentary SPICE output is not validated DSPF/SPEF.

## Production gap

Build a terminal-mapped conductor/via graph; add versioned process stack, dielectric,
corner, width/thickness/temperature/size and via-array models; calibrate ground/lateral/
vertical/diagonal coupling with same-net/shield/fill context; add required device,
substrate, RLC/frequency and electrothermal options; implement protected-terminal
reduction and validated DSPF/SPEF, selected-net, multi-corner, restart/distributed output.

Element topology and physical component values must correlate to field-solver structures
and the target golden extractor. See [Wave 5](../work-packages/wave-5.md).
