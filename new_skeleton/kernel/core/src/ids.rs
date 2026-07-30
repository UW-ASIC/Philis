//! Indices into the SoA tables, never pointers, plus the generalized placement
//! target. Each id is the smallest integer that plausibly fits, meaningless
//! without the table it indexes.

/// Position of a device in [`crate::Netlist::devices`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DeviceId(pub u16);

/// Position of a net in [`crate::Netlist::nets`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NetId(pub u16);

/// Position of a symmetry axis in a placement. Shared by mirror partners.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AxisId(pub u16);

/// One **disjunctive branch decision**, indexing [`crate::Layout::branch`].
///
/// Some analog constraints have a feasible set that is a *union* of disconnected
/// intervals rather than one interval: deep-trench isolation is `{d ≤ s_max} ∪
/// {d ≥ d_dti}`, injector exclusion is share-a-ring or take-a-private-one, the
/// implant-merge/LVS guard is merge-by-design or keep-the-keepout. For each of those
/// the search must *commit* to a component, and that commitment is a search variable
/// like a position — so it needs an addressable slot in the layout.
///
/// A rule carries this id rather than the bool itself because `analog::RuleBatch` is
/// type-erased: the placer cannot reach inside a batch to find a branch, but it can
/// flip `layout.branch[id]` and re-score. Assigned by the annotator at extraction.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BranchId(pub u16);

/// A recognised group of devices placed as one unit — the annotator's block made
/// addressable. Indexes [`crate::Layout`]'s group table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GroupId(pub u16);

/// What a placement constraint relates: an individual device **or** a whole
/// group. The same constraint type (matching, symmetry, thermal…) works at
/// either scale — device↔device *or* group↔group — by carrying `Target`s instead
/// of a bare [`DeviceId`]. [`crate::Layout`] resolves a `Group` to its members'
/// bounding box, so a rule's scoring code is identical either way. This is why
/// there is no separate "group constraint" type — it is the same constraint,
/// generalized.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Target {
    Device(DeviceId),
    Group(GroupId),
}

impl Target {
    /// Re-express a device target in a collapsed cell space: `cell_of[device] =
    /// cell index` (group collapse, PLAN §2). Group targets pass through — the
    /// group table is remapped by members, so its ids stay valid.
    ///
    /// A device id out of `cell_of`'s range passes through unchanged rather than
    /// panicking: the map is device-length by construction, so an out-of-range id
    /// was already dangling and the caller's range assert is the place to hear it.
    #[inline]
    #[must_use]
    pub fn retarget(self, cell_of: &[u16]) -> Self {
        match self {
            Target::Device(d) => match cell_of.get(d.0 as usize) {
                Some(&c) => Target::Device(DeviceId(c)),
                None => self,
            },
            Target::Group(_) => self,
        }
    }
}
