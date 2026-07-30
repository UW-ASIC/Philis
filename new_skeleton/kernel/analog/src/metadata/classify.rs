//! Net classification (metadata tier).
//!
//! Ported from `backend/constraints/src/metadata/classify.rs`.

use pnr_core::ids::NetId;

/// **Net classification — the classifier that drives the rest.** Assigns each net
/// a functional class and the routing budgets that class implies. It is the *front*
/// of the constraint pipeline: a `Sensitive`/`Clock` net *produces* shielding and
/// tight coupling budgets; a high-current `Supply` net *produces* wide-metal/via
/// rules. Tagging the net's *function* here decouples what the circuit needs from
/// the layout *mechanics* the routing tier later derives.
///
/// - **Role:** classification metadata — run early by the `annotator`; consumed by
///   the routing tier to decide which rules apply. Not scored.
/// - **Books:** none in repo (engineering classifier).
#[derive(Clone, Debug)]
pub struct NetClassification {
    pub net: NetId, // was: net_name: String
    pub class: NetClass,
    pub voltage_domain: Option<VoltDomain>,
    pub shielding_required: bool,
    /// Parasitic C budget, atto-farad (`None` = unbudgeted). was: ff: f64
    pub c_budget_af: Option<i64>,
    /// Parasitic R budget, milli-ohm (`None` = unbudgeted). was: ohm: f64
    pub r_budget_mohm: Option<i64>,
    /// Max coupling to a neighbour, atto-farad (`None` = unbudgeted). was: ff: f64
    pub max_coupling_af: Option<i64>,
    pub preferred_layers: Vec<pnr_core::geom::LayerId>, // was: Vec<String>
}

/// Functional role of a net — the axis every routing rule keys off.
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

/// Voltage domain a net lives in — gates spacing/oxide and cross-domain rules.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoltDomain {
    Core,
    Io,
    Analog,
}
