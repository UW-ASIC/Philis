//! The read-only geometry view a placement rule scores against.

use crate::geom::Orient;
use crate::ids::{AxisId, DeviceId, Target};

/// Where every device sits and how big it is — the state a placement algorithm is
/// optimising.
///
/// A placement rule reads a `Layout` and never mutates it: scoring is **pure**,
/// so the same `(rule, layout)` always yields the same score. That purity is what
/// lets the SA engine evaluate millions of trial moves reproducibly for a fixed
/// seed.
///
/// SoA tables indexed by [`DeviceId`] hold centre and half-extents; `axis` holds
/// symmetry-axis positions by [`AxisId`]; `groups` maps a [`crate::GroupId`] to
/// its member devices, so [`Layout::centre`]/[`Layout::extent`] resolve a
/// [`Target::Group`] to the members' bounding box — a rule scores a group exactly
/// like a device.
pub struct Layout {
    /// Device centre x, `nm`.
    pub x: Vec<i32>,
    /// Device centre y, `nm`.
    pub y: Vec<i32>,
    /// Device half-width (half the drawn x-extent), `nm`.
    pub hw: Vec<i32>,
    /// Device half-height (half the drawn y-extent), `nm`.
    pub hh: Vec<i32>,
    /// How each device's macro is turned before it is stamped ([`Orient`]).
    ///
    /// `hw`/`hh` are the extents **after** this transform, so every existing
    /// bbox/overlap read stays correct without knowing about orientation — the
    /// invariant is that whoever writes `orient` swaps `hw`/`hh` in the same
    /// breath when [`Orient::swaps_axes`]. The transform is applied to the drawn
    /// geometry at the single stamping site (`frontend/library::geometry`).
    pub orient: Vec<Orient>,
    /// **Which drawn variant each cell is currently using** — an index into that
    /// cell's alternatives (`gp::VariantSpace`).
    ///
    /// This is what makes variant choice a *placement search variable* rather than
    /// a decision frozen before placement (PLAN §2). A variant change relocates
    /// pins, so routability and extracted parasitics are **discontinuous** in this
    /// index — there is no gradient across it and no valid continuous relaxation of
    /// it. It therefore lives here, in the search state, and is changed by discrete
    /// moves the same way `x`/`y`/`orient` are.
    ///
    /// Invariant, identical in spirit to the `orient`/`hw`/`hh` one above: whoever
    /// writes `variant[i]` must write `hw[i]`/`hh[i]` from the new variant's
    /// footprint in the same breath, or every bbox and overlap read silently scores
    /// the old geometry.
    ///
    /// `0` means "the first alternative", which is what a cell with no enumerated
    /// alternatives also reads as — so an all-zero table is the correct
    /// no-variant-search state and needs no special case.
    pub variant: Vec<u16>,
    /// Symmetry-axis x-position by [`AxisId`], `nm`.
    pub axis: Vec<i32>,
    /// **Disjunctive branch commitments** by [`crate::ids::BranchId`] — which component
    /// of a union-shaped feasible set each such constraint is currently committed to.
    ///
    /// Search state, exactly like `x`/`y`/`variant`, and flipped by a discrete move. It
    /// exists because a penalty cannot choose between disconnected components: for deep
    /// trench isolation the feasible set is `{d ≤ s_max} ∪ {d ≥ d_dti}`, and a penalty on
    /// how far into the illegal interval a pair sits *peaks in the middle of that
    /// interval* and descends away in both directions. The optimiser is told to leave
    /// the gap but never which side to leave by, so the component a pair ends up in is
    /// decided by wherever it happened to start (PLAN §4b).
    ///
    /// Once the commitment is explicit, each branch is an ordinary convex interval that
    /// the gradient handles cleanly, and choosing between them is a move that can be
    /// priced and reverted.
    ///
    /// `false` / `true` mean whatever the owning rule documents — for
    /// `analog::placement::DtiBand`, `false` = share a trench and `true` = fully
    /// isolate. An all-`false` table is a valid starting commitment.
    pub branch: Vec<bool>,
    /// Member devices of each group, by [`crate::GroupId`].
    pub groups: Vec<Vec<DeviceId>>,
    /// Steady-state dissipation per device, µW — the thermal solver's *input*.
    /// `0` for a device whose operating point is unknown, which makes it a pure
    /// heat sensor rather than a source.
    pub power_uw: Vec<i32>,
    /// Temperature rise above ambient per device, milli-°C — the thermal
    /// solver's *output*, and derived state: a cache of
    /// [`crate::thermal::rises_mc`] at the current positions.
    ///
    /// Stale until [`Layout::refresh_temps`] is called. It is refreshed at
    /// epoch/iteration boundaries rather than per move, because temperature is a
    /// global property of the whole power distribution (see [`crate::thermal`]).
    pub temp_mc: Vec<i32>,
}

impl Layout {
    /// Bounding box of a [`Target`] as `(cx, cy, hw, hh)` in `nm`. A device is its
    /// own box; a group is the box enclosing all its members.
    ///
    /// # Panics
    /// If the target is out of range — a caller bug, not a runtime condition.
    #[inline]
    #[must_use]
    pub fn bbox(&self, t: Target) -> (i32, i32, i32, i32) {
        match t {
            Target::Device(d) => {
                let i = d.0 as usize;
                (self.x[i], self.y[i], self.hw[i], self.hh[i])
            }
            Target::Group(g) => {
                let members = &self.groups[g.0 as usize];
                let mut it = members.iter();
                let first = it.next().expect("group has no members");
                let fi = first.0 as usize;
                let (mut lo_x, mut lo_y) = (self.x[fi] - self.hw[fi], self.y[fi] - self.hh[fi]);
                let (mut hi_x, mut hi_y) = (self.x[fi] + self.hw[fi], self.y[fi] + self.hh[fi]);
                for d in it {
                    let i = d.0 as usize;
                    lo_x = lo_x.min(self.x[i] - self.hw[i]);
                    lo_y = lo_y.min(self.y[i] - self.hh[i]);
                    hi_x = hi_x.max(self.x[i] + self.hw[i]);
                    hi_y = hi_y.max(self.y[i] + self.hh[i]);
                }
                ((lo_x + hi_x) / 2, (lo_y + hi_y) / 2, (hi_x - lo_x) / 2, (hi_y - lo_y) / 2)
            }
        }
    }

    /// Centre of a [`Target`] in `nm` (device centre, or group bbox centre).
    #[inline]
    #[must_use]
    pub fn centre(&self, t: Target) -> (i32, i32) {
        let (cx, cy, _, _) = self.bbox(t);
        (cx, cy)
    }

    /// Half-extents `(hw, hh)` of a [`Target`] in `nm`.
    #[inline]
    #[must_use]
    pub fn extent(&self, t: Target) -> (i32, i32) {
        let (_, _, hw, hh) = self.bbox(t);
        (hw, hh)
    }

    /// x-position of symmetry axis `a`, `nm`.
    ///
    /// Falls back to the die-centre estimate for an unknown axis rather than
    /// panicking: axis ids are handed out per *block*, and a circuit can carry
    /// more blocks than devices (every device its own block, plus the glue
    /// node), so the table is not guaranteed to be device-length.
    #[inline]
    #[must_use]
    pub fn axis_x(&self, a: AxisId) -> i32 {
        match self.axis.get(a.0 as usize) {
            Some(&v) => v,
            None => self.centre_x_estimate(),
        }
    }

    /// Mean device centre on x, `nm` — the neutral default for an axis nobody
    /// has placed yet.
    #[inline]
    #[must_use]
    pub fn centre_x_estimate(&self) -> i32 {
        if self.x.is_empty() {
            return 0;
        }
        (self.x.iter().map(|&v| i64::from(v)).sum::<i64>() / self.x.len() as i64) as i32
    }

    /// Stage-boundary structural check: every SoA table is device-length, extents
    /// are non-negative, and every group member is a real device.
    ///
    /// Compiled out without `debug_assertions`, so it costs nothing in the
    /// benchmark build and still fires in `cargo test`/dev. `ctx` names the stage
    /// that produced the layout, because "lengths disagree" is useless without
    /// knowing who wrote it. Turn it on in a release run with
    /// `RUSTFLAGS="-C debug-assertions=yes"`.
    #[inline]
    pub fn debug_check(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        let n = self.x.len();
        for (name, len) in [
            ("y", self.y.len()),
            ("hw", self.hw.len()),
            ("hh", self.hh.len()),
            ("orient", self.orient.len()),
            // `variant` is length-checked like every other SoA column because the
            // reshape move indexes it positionally: a short table makes
            // `variant[i]` either panic or address the wrong cell's choice, and the
            // `hw`/`hh` invariant then documents geometry nobody wrote.
            ("variant", self.variant.len()),
            ("power_uw", self.power_uw.len()),
            ("temp_mc", self.temp_mc.len()),
        ] {
            assert_eq!(len, n, "{ctx}: Layout.{name} has {len} entries, x has {n}");
        }
        for i in 0..n {
            assert!(self.hw[i] >= 0 && self.hh[i] >= 0, "{ctx}: device {i} has negative extent");
        }
        for (gi, members) in self.groups.iter().enumerate() {
            for d in members {
                assert!(
                    (d.0 as usize) < n,
                    "{ctx}: group {gi} names device {} of {n}",
                    d.0
                );
            }
        }
    }

    /// Post-placement legality check: **no two device footprints overlap**.
    ///
    /// Not "mostly" — at all. Each device is drawn as one self-contained macro
    /// with its own diffusion, tap, implants and well, so two footprints on top
    /// of each other means two devices' geometry is drawn on top of each other.
    /// That is what LVS later reports as merged nets, doubled `W`, or a channel
    /// that "ambiguously matches MOS rules [nmos, pmos]" — five stages away from
    /// the placer that caused it.
    ///
    /// ponytail: this asserts the *drawn-macro* invariant, which is stricter than
    /// abutment. When `cellgen` can merge a matched group into one macro, the
    /// merged devices become one entry here and this stays exactly as it is.
    #[inline]
    pub fn debug_check_placed(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        self.debug_check(ctx);
        for a in 0..self.x.len() {
            for b in (a + 1)..self.x.len() {
                let ox = (self.hw[a] + self.hw[b]) - (self.x[a] - self.x[b]).abs();
                let oy = (self.hh[a] + self.hh[b]) - (self.y[a] - self.y[b]).abs();
                assert!(
                    ox <= 0 || oy <= 0,
                    "{ctx}: devices {a} and {b} overlap {ox}x{oy} nm — their drawn \
                     geometry is stacked, and every downstream DRC/LVS number is \
                     measured on it"
                );
            }
        }
    }

    /// Euclidean **edge-to-edge** gap between two targets' bounding boxes, `nm` —
    /// `0.0` when they overlap or touch. `|Δcentre| − (extent_a + extent_b)` per
    /// axis, clamped at 0, combined. Works for device↔device, group↔group, or a
    /// mix.
    #[inline]
    #[must_use]
    pub fn edge_gap(&self, a: Target, b: Target) -> f32 {
        let (ax, ay, ahw, ahh) = self.bbox(a);
        let (bx, by, bhw, bhh) = self.bbox(b);
        let gx = ((ax - bx).abs() - (ahw + bhw)).max(0);
        let gy = ((ay - by).abs() - (ahh + bhh)).max(0);
        (((gx as i64) * (gx as i64) + (gy as i64) * (gy as i64)) as f32).sqrt()
    }
}
