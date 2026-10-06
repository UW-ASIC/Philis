//! Indices into the SoA tables. Meaningless without the table they index.

/// Index into [`crate::Netlist::devices`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DeviceId(pub u16);

/// Index into [`crate::Netlist::nets`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NetId(pub u16);

/// Index into [`crate::Layout::axis`]. Shared by mirror partners.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AxisId(pub u16);

/// Index into [`crate::Layout::branch`]: one disjunctive commitment (e.g. DTI
/// share-vs-isolate) the search can flip. Assigned by the annotator.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BranchId(pub u16);

/// Index into [`crate::Layout::groups`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GroupId(pub u16);

/// What a placement constraint relates: one device or a whole group.
/// [`crate::Layout::bbox`] resolves a group to its members' bounding box, so a
/// rule scores either the same way.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Target {
    Device(DeviceId),
    Group(GroupId),
}

impl Target {
    /// Map a device target through `cell_of[device] = cell` (group collapse).
    /// Groups, and device ids past the end of `cell_of`, pass through.
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
