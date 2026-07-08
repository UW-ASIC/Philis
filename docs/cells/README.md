# Cell Shape Catalogs

Per-device exhaustive documentation of every layout axis (shape, orientation,
formation, pattern, dummies, guard rings, contacts) the cell generators can —
and should — enumerate. Each device has:

- **`.md`** — axis-by-axis catalog with tier tables, formulas, and an
  "implemented in `generator.rs` today?" column per axis.
- **`.html`** — self-contained GDS-style 2D views (inline SVG, shared
  palette), 3D isometric views for vertical devices, orientation charts.
- **`.excalidraw`** — simplified Excalidraw scene of key views.

All six use the same layer palette:

| Role | Fill | Opacity | Notes |
|---|---|---|---|
| diff/active | `#2E8B57` | 0.55 | |
| poly | `#C23B22` | 0.70 | |
| licon/contact | `#111111` | solid | small squares |
| li | `#4682B4` | 0.50 | |
| met1 | `#1E90FF` | 0.40 | |
| met2 | `#9370DB` | 0.40 | |
| nwell | `#F5DEB3` | 0.35 | dashed `#8B7355` outline |
| dummy | same colors | — | diagonal-hatch SVG pattern overlay |

## Device catalogs

| Device | Catalog | Key shapes | 3D view? |
|---|---|---|---|
| [MOSFET](mosfet.md) | 9 axes, 8-transform table, chi metric | single/multi-finger, ABBA, cross-quad | cross-section only (planar) |
| [Resistor](resistor.md) | 9 axes, material ranking, Kelvin | bar, serpentine, interdig array, L-pair | vertical plug resistor |
| [Capacitor](capacitor.md) | 7 axes, type ranking, array patterns | MIM unit, MOM comb, ratio array | MIM + poly-poly stacks |
| [BJT](bjt.md) | 7 axes, ratioed arrays, piezojunction | CBE rings, annular PNP, 8-around-1 | vertical NPN cutaway w/ NBL+sinker |
| [Diode](diode.md) | 7 axes, area/perimeter geometry | square, stripe, waffle, Schottky+ring | P+/N-well junction cutaway |
| [Inductor](inductor.md) | 7 axes, chirality under mirrors | square/octagonal/differential spirals | 2-layer stacked spiral |

## Framing doc

Architecture rationale (spec-enumeration pipeline, Pareto selection, why
modularity lives in the enumerator not in generator subclasses):
[`docs/frontend/GENERATOR_MODULARITY.md`](../frontend/GENERATOR_MODULARITY.md)
