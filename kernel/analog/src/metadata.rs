//! Net classification tags the annotator turns into routing rules.

use pnr_core::geom::LayerId;
use pnr_core::ids::NetId;

/// A net's functional class and the routing budgets it implies.
/// Units: capacitance aF, resistance mΩ; `None` = unbudgeted.
#[derive(Clone, Debug)]
pub struct NetClassification {
    pub net: NetId,
    pub class: NetClass,
    pub voltage_domain: Option<VoltDomain>,
    pub shielding_required: bool,
    pub c_budget_af: Option<i64>,
    pub r_budget_mohm: Option<i64>,
    /// Max total coupling onto this net.
    pub max_coupling_af: Option<i64>,
    pub preferred_layers: Vec<LayerId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetClass {
    Signal,
    Clock,
    Supply,
    Ground,
    /// Sensitive reference (bias, bandgap, ADC reference).
    Sensitive,
    Substrate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoltDomain {
    Core,
    Io,
    Analog,
}
