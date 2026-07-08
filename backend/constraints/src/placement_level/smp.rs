use crate::types::{DeviceId, GroupConstraint, HsmpgNodeType, SmpEdgeType};

/// Edge in the SMP (Symmetry-Matching-Proximity) multigraph.
#[derive(Debug, Clone)]
pub struct SmpEdge {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    pub edge_type: SmpEdgeType,
}

/// Node in the hierarchical SMP graph (HSMPG) tree. Arena-indexed.
#[derive(Debug, Clone)]
pub struct HsmpgNode {
    pub node_id: String,
    pub node_type: HsmpgNodeType,
    /// Children indices into the arena `Vec<HsmpgNode>`.
    pub children: Vec<usize>,
    pub devices: Vec<DeviceId>,
}

impl GroupConstraint for HsmpgNode {
    fn devices(&self) -> &[DeviceId] { &self.devices }
}
