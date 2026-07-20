//! The only public entry point for physical design.
//!
//! The backend is a **Facade** around a fixed **Template Method**:
//!
//! `constraints -> cells -> placement -> routing -> signoff`
//!
//! Callers provide data through [`FlowInput`] and invoke [`Backend::run`].
//! They cannot reorder stages or call flow internals. Algorithm extensions use
//! the traits in [`strategy`], so new implementations keep the same engine
//! architecture instead of inventing a second orchestration model.
//!
//! Backend crates form this directed acyclic dependency graph (an arrow means
//! "depends on"):
//!
//! ```text
//! cells -----------\
//!                   +--> engine --> placement --> routing --> pnr-backend
//! constraints -----/          \---------------------------> pnr-backend
//! substrate3 (frontend) --> cells
//! ```
//!
//! `pnr-backend` is the composition root. Lower crates must never depend on it.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

mod flow;
pub mod gds;
pub mod interface;
pub mod pdk;

use pnr_cells::netlist::BipartiteHypergraph;

pub use flow::{FlowConfig, FlowResult, SignoffReport};
pub use interface::{BoundaryPin, DieSpec, InterfaceSpec, Side};
pub use pdk::{load_pdk, CellBase, DeviceDef, FullPdk, Pdk};
pub use pnr_constraints::{ConstraintContract, ConstraintRecord, ConstraintStatus, NetClass};

/// The sole supported backend facade.
///
/// Its fields are private so callers cannot replace the process or flow policy
/// halfway through a run. Create one after parsing the PDK, then submit designs
/// with [`Self::run`].
pub struct Backend<'a> {
    process: &'a FullPdk,
    config: &'a FlowConfig,
}

impl<'a> Backend<'a> {
    /// Bind one immutable process definition and one flow policy.
    #[must_use]
    pub const fn new(process: &'a FullPdk, config: &'a FlowConfig) -> Self {
        Self { process, config }
    }

    /// Execute the canonical backend template method.
    pub fn run(
        &self,
        input: FlowInput,
        constraints: &ConstraintRecord,
    ) -> Result<FlowResult, String> {
        flow::run(input, self.process, constraints, self.config)
    }
}

/// Validated, allocation-tight input boundary between frontend and backend.
///
/// Net classifications are stored as a boxed slice: their length is immutable,
/// capacity equals length, and routing reads them linearly beside the graph's
/// dense net arrays.
pub struct FlowInput {
    pub(crate) graph: BipartiteHypergraph,
    pub(crate) net_classes: Box<[NetClass]>,
}

impl FlowInput {
    /// Validate and compact frontend output before it crosses the boundary.
    pub fn new(graph: BipartiteHypergraph, net_classes: Vec<NetClass>) -> Result<Self, String> {
        if graph.cells.is_empty() {
            return Err("no PDK devices resolved from netlist".into());
        }
        if net_classes.len() != graph.nets.len() {
            return Err(format!(
                "frontend classified {} nets, but the graph contains {}",
                net_classes.len(),
                graph.nets.len(),
            ));
        }
        Ok(Self {
            graph,
            net_classes: net_classes.into_boxed_slice(),
        })
    }
}

/// Approved algorithm extension points.
///
/// The facade fixes stage order; these traits restrict customization to the
/// data-oriented engine slots used inside placement and routing.
pub mod strategy {
    pub use pnr_engine::{
        Accept, Core, CostFn, Density, Domain, Ledger, Legality, Schedule, Stop, Weights,
    };
}

#[cfg(test)]
mod architecture_tests {
    const CRATES: [(&str, usize, &str); 6] = [
        ("pnr-cells", 0, include_str!("../cells/Cargo.toml")),
        (
            "pnr-constraints",
            0,
            include_str!("../constraints/Cargo.toml"),
        ),
        ("pnr-engine", 1, include_str!("../engine/Cargo.toml")),
        ("pnr-placement", 2, include_str!("../placement/Cargo.toml")),
        ("pnr-routing", 3, include_str!("../routing/Cargo.toml")),
        ("pnr-backend", 4, include_str!("../Cargo.toml")),
    ];

    #[test]
    fn backend_dependencies_only_point_down_the_dag() {
        for &(dependent, dependent_layer, manifest) in &CRATES {
            for &(dependency, dependency_layer, _) in &CRATES {
                let declaration = format!("{dependency} =");
                if manifest
                    .lines()
                    .any(|line| line.trim_start().starts_with(&declaration))
                {
                    assert!(
                        dependency_layer < dependent_layer,
                        "forbidden backend edge: {dependent} -> {dependency}",
                    );
                }
            }
        }
    }

    #[test]
    fn substrate3_consumes_cells_and_backend_never_consumes_substrate3() {
        let substrate = include_str!("../../frontend/substrate3/Cargo.toml");
        assert!(substrate
            .lines()
            .any(|line| { line.trim_start().starts_with("pnr-cells =") }));
        for &(name, _, manifest) in &CRATES {
            assert!(
                !manifest.contains("substrate3") && !manifest.contains("frontend/"),
                "forbidden frontend dependency in {name}",
            );
        }
    }
}
