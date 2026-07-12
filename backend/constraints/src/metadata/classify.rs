use crate::types::{NetClass, NetConstraint, VoltDomain};

/// Classification of a net for routing constraint derivation.
#[derive(Debug, Clone)]
pub struct NetClassification {
    pub net_name: String,
    pub net_class: NetClass,
    pub voltage_domain: Option<VoltDomain>,
    pub shielding_required: bool,
    pub parasitic_c_budget_ff: Option<f64>,
    pub parasitic_r_budget_ohm: Option<f64>,
    pub preferred_layers: Vec<String>,
    pub max_coupling_ff: Option<f64>,
}

impl NetConstraint for NetClassification {
    fn net_name(&self) -> &str {
        &self.net_name
    }
}
