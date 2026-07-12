//! Thin frontend adapter: text input in, typed backend request out.

use std::collections::HashSet;

use pnr_backend::{ConstraintRecord, NetClass};

pub use pnr_backend::{FlowConfig, FlowResult, SignoffReport};

/// Parse frontend input and delegate all physical-design work to
/// [`pnr_backend::Backend`].
#[allow(clippy::missing_errors_doc)]
pub fn run_flow(
    spice: &str,
    deck_json: &str,
    constraints: &ConstraintRecord,
    config: &FlowConfig,
) -> Result<FlowResult, String> {
    // Step 1: load and validate the process once, producing the three views
    // consumed by netlist parsing, cell generation, and physical verification.
    let process = pnr_backend::load_pdk(deck_json)?;

    // Step 2: parse source text into the backend-owned dense hypergraph. This
    // is the frontend's only transformation of circuit syntax.
    let graph = crate::netlist::parse_spice(spice, &process.netlist, &HashSet::new())
        .map_err(|error| error.to_string())?;

    // Step 3: classify source nets and immediately convert frontend labels to
    // the backend constraint vocabulary; no annotator type crosses the API.
    let net_classes =
        pnr_annotator::classify_nets(&graph, &pnr_annotator::AnnotationConfig::default())
            .into_iter()
            .map(|role| match role {
                pnr_annotator::NetRole::Signal => NetClass::Signal,
                pnr_annotator::NetRole::Supply => NetClass::Supply,
                pnr_annotator::NetRole::Ground => NetClass::Ground,
                pnr_annotator::NetRole::Clock => NetClass::Clock,
            })
            .collect();

    // Step 4: validate the boundary payload and hand control to the backend's
    // fixed template method. The frontend does no placement, routing, or signoff.
    let input = pnr_backend::FlowInput::new(graph, net_classes)?;
    pnr_backend::Backend::new(&process, config).run(input, constraints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_devices_cross_the_frontend_backend_boundary() {
        const NETLIST: &str = "\
.subckt minv in out mid VDD VSS
XM1 mid in VSS VSS nfet_01v8 W=0.42u L=0.15u
XM2 mid in VDD VDD pfet_01v8 W=0.42u L=0.15u
XR1 mid out res_generic_po W=0.33u L=0.1u
.ends minv
";
        let deck = std::fs::read_to_string(format!(
            "{}/../../pdks/sky130.json",
            env!("CARGO_MANIFEST_DIR"),
        ))
        .expect("read test PDK");
        let result = run_flow(
            NETLIST,
            &deck,
            &ConstraintRecord::default(),
            &FlowConfig::default(),
        )
        .expect("flow");

        assert!(result.routing.report.unrouted.is_empty());
        assert_eq!(result.routing.report.overuse, 0);
        assert_eq!(result.signoff.drc_blocking.len(), 0);
        assert!(result.signoff.lvs.matched, "{}", result.signoff.lvs.reason);
    }
}
