//! # `Process` — the PDK-agnostic seam
//!
//! A generator never names a GDS layer number or a design-rule value inline. It
//! asks the bound **process** for a layer by *role* (`"poly"`, `"metal1"`) and a
//! rule by *name* (`"min_width"`, `"contact"`). Swapping the PDK swaps the
//! numbers behind these keys, not the generator.
//!
//! The vocabulary is **string keys**, not enums: the `cells` builder already
//! looks layers/rules up by role/name strings, and a real PDK carries dozens of
//! them — enumerating every one as a Rust variant buys nothing. Asking for a key
//! the bound process omits is a run-time `None`/`default` the generator handles
//! (`if let Some(li) = p.layer("li")` / `p.rule("contact", 170)`), exactly as
//! substrate2 handles optional implant/marker layers.
//!
//! The one concrete implementation lives in `verify::Pdk` (it wraps a gdsverify
//! deck); `cells` and `macroMaster` are written against this trait alone, so they
//! stay PDK-agnostic and depend only on `pnr_core`.

use crate::LayerId;

/// The PDK-agnostic process contract every generator is written against.
///
/// Mirrors substrate2's `Pdk`/layer-set role: the generator holds a
/// `&dyn Process` and never sees a raw layer number.
pub trait Process {
    /// Resolve a layer **role** (e.g. `"poly"`, `"metal1"`) to the concrete
    /// [`LayerId`] the drawn geometry carries. `None` ⇒ this process omits that
    /// (optional) layer; the generator draws it with `if let Some(..)`.
    fn layer(&self, role: &str) -> Option<LayerId>;

    /// Resolve a design **rule** by `name` to its value in `nm`, or `default`
    /// when the process does not specify it.
    fn rule(&self, name: &str, default: i32) -> i32;

    /// Fabrication grid in `nm`. All coordinates a generator emits snap to this;
    /// off-grid is the one DRC a generator can enforce with no rule table.
    fn grid(&self) -> i32;
}
