//! One SREF/AREF step in a root-to-shape hierarchy path.

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstancePathEntry {
    pub parent_structure: String,
    pub element_index: u32,
    pub referenced_structure: String,
    pub column: u16,
    pub row: u16,
}
