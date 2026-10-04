//! Routing-tier rules, scored against [`pnr_core::Routes`] (gr, dr).

pub mod antenna;
pub mod common_node;
pub mod coupling;
pub mod crosstalk;
pub mod current;
pub mod differential;
pub mod em;
pub mod ir;
pub mod metal_over_gate;
pub mod parasitic;
pub mod performance;
pub mod plate_ratio;
pub mod shield;
pub mod stack;

pub use antenna::Antenna;
pub use common_node::{CommonNode, CommonNodes};
pub use coupling::CouplingBudget;
pub use crosstalk::CrosstalkExclusion;
pub use differential::Differential;
pub use em::Electromigration;
pub use ir::IrDrop;
pub use metal_over_gate::MetalOverGate;
pub use parasitic::ParasiticBudget;
pub use performance::PerformanceBudget;
pub use plate_ratio::{PlateRatio, PlateRatios, PlateSet};
pub use shield::Shield;
pub use stack::{DiodeCredit, Stack};
