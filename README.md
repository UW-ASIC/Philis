# Philis

Constraint-aware analog place-and-route in Rust: SPICE in, placed, routed and signed-off GDS out (`philis run <netlist.sp> sky130 -o out/`). Plans and audits: [docs/plans/00-MASTER-PLAN.md](docs/plans/00-MASTER-PLAN.md).

## Progress

<!-- progress:start -->
_Last snapshot: 2026-10-04 13:56, branch `m2` at `b4054b6` (`bench local`, sky130, seed 1). Updated automatically by `benchmarks/progress.py`._

### Speed

![speed](docs/progress/speed.svg)

| circuit | wall ms | iterations | ns/iter | DRC | LVS | ERC |
|---|---|---|---|---|---|---|
| bgr_core | 98,718 | 15 | 6,581,200,000 | 0 | MATCH | 0 |
| bjt_mirror | 37,464 | 5 | 7,492,800,000 | 0 | MATCH | 0 |
| chain4 | 25,477 | 5 | 5,095,400,000 | 0 | MATCH | 0 |
| dac4 | 279,049 | 20 | 13,952,450,000 | 6 | PARTIAL(16) | 1 |
| dac4_mim | 88,970 | 20 | 4,448,500,000 | 5 | MATCH | 1 |
| mirror_ratio | 13,613 | 15 | 907,533,333 | 0 | MATCH | 0 |
| ota | 182,645 | 20 | 9,132,250,000 | 0 | MATCH | 0 |
| ota_constrained | 117,728 | 20 | 5,886,400,000 | 0 | MATCH | 0 |
| pair | 4,422 | 5 | 884,400,000 | 0 | MATCH | 0 |
| quad | 11,372 | 5 | 2,274,400,000 | 0 | MATCH | 0 |
| rc_filter | 15,319 | 10 | 1,531,900,000 | 0 | MATCH | 0 |
| res_m2 | 8,319 | 20 | 415,950,000 | 0 | MATCH | 0 |
| tq_chain | 80,408 | 20 | 4,020,400,000 | 18 | MATCH | 5 |
| tt_ota | 99,148 | 20 | 4,957,400,000 | 0 | MATCH | 0 |

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
