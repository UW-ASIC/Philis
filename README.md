# Philis

Constraint-aware analog place-and-route: SPICE in, placed, routed and signed-off GDS out.

## Progress

<!-- progress:start -->
_Last snapshot: 2026-10-04 14:28, branch `m2 + progress-charts` at `3032483` (`bench local`, sky130, seed 1). Updated automatically by `benchmarks/progress.py`._

### Feedback loop

![feedback](docs/progress/feedback.svg)

### Speed

![speed](docs/progress/speed.svg)

| circuit | wall ms | iterations | ns/iter | DRC | LVS | ERC |
|---|---|---|---|---|---|---|
| bgr_core | 56,132 | 15 | 3,742,133,333 | 0 | MATCH | 0 |
| dac4 | 113,905 | 20 | 5,695,250,000 | 7 | PARTIAL(16) | 1 |
| ota | 344,260 | 20 | 17,213,000,000 | 0 | MATCH | 0 |
| pair | 14,169 | 5 | 2,833,800,000 | 0 | MATCH | 0 |
| rc_filter | 14,081 | 10 | 1,408,100,000 | 0 | MATCH | 0 |
| tq_chain | 155,605 | 20 | 7,780,250,000 | 18 | MATCH | 5 |

### Sample layouts

**ota**

![ota](docs/progress/layouts/ota.svg)

**dac4**

![dac4](docs/progress/layouts/dac4.svg)

**bgr_core**

![bgr_core](docs/progress/layouts/bgr_core.svg)

**rc_filter**

![rc_filter](docs/progress/layouts/rc_filter.svg)

**pair**

![pair](docs/progress/layouts/pair.svg)

**tq_chain**

![tq_chain](docs/progress/layouts/tq_chain.svg)
<!-- progress:end -->
