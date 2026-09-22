//! Thin frontend adapter: text input in, typed backend request out.

use std::collections::HashMap;
use std::sync::Arc;

use pnr_backend::{ConstraintRecord, FullPdk, NetClass};
use pnr_cells::CellOutput;

pub use pnr_backend::{FlowConfig, FlowResult, SignoffReport};

/// Parse frontend input and delegate all physical-design work to
/// [`pnr_backend::Backend`]. Flat run: the whole design is one placement.
#[allow(clippy::missing_errors_doc)]
pub fn run_flow(
    spice: &str,
    deck_json: &str,
    constraints: &ConstraintRecord,
    config: &FlowConfig,
) -> Result<FlowResult, String> {
    let process = pnr_backend::load_pdk(deck_json)?;
    run_on_process(spice, &process, constraints, config, HashMap::new())
}

/// Run one (sub)design against an already-loaded process, resolving any
/// instances whose master names a key in `macros` as pre-placed sub-blocks
/// (kept whole by the parser, stamped as fixed cells by the backend). The
/// hierarchical driver calls this per subckt; a flat run passes an empty map.
#[allow(clippy::missing_errors_doc)]
pub fn run_on_process(
    spice: &str,
    process: &FullPdk,
    constraints: &ConstraintRecord,
    config: &FlowConfig,
    macros: HashMap<String, Arc<CellOutput>>,
) -> Result<FlowResult, String> {
    // Keep macro subckts whole (do not flatten to leaves) so their instances
    // stay as single nodes the backend resolves from the registry.
    let macro_names = macros.keys().cloned().collect();
    let graph = crate::netlist::parse_spice(spice, &process.netlist, &macro_names)
        .map_err(|error| error.to_string())?;

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

    let input = pnr_backend::FlowInput::new(graph, net_classes)?.with_macros(macros);
    pnr_backend::Backend::new(process, config).run(input, constraints)
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
