//! # Metadata tier — **classification tags that *produce* other constraints**.
//!
//! These do not score placements or routes. They tag devices/nets/pads with
//! functional context ([`classify::NetClassification`], [`bias_current::BiasCurrentTag`],
//! [`esd::EsdConstraint`]) that the `annotator` turns into the concrete
//! [`crate::Constraints`] and [`crate::Rule`]s of the other tiers. This is the
//! front of the derivation chain: metadata → constraints → rules.
//!
//! **TODO(ADR-0003):** this tier is a staging area, not permanent. Every tag here
//! ultimately expresses itself as a placement or routing constraint; once
//! [`crate::extract`] and the routing tier are fleshed out, dissolve `metadata`
//! and move each tag to the tier that consumes it. See
//! `docs/adr/0003-metadata-tier-into-placement-routing.md`.

pub mod bias_current;
pub mod classify;
pub mod esd;

pub use bias_current::BiasCurrentTag;
pub use classify::{NetClass, NetClassification, VoltDomain};
pub use esd::{EsdConstraint, EsdProtectionType};
