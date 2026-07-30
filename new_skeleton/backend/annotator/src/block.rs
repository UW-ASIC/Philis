//! Recognised circuit structure — the annotator's output vocabulary and the
//! nodes of the **block DAG**.
//!
//! `Block`/`BlockKind` live here, not in `core`: they are recognition *outputs*,
//! not foundation. A [`Block`] is one node of the parallelizable DAG the
//! annotator builds; [`Block::depends_on`] carries the edges (see [`crate::annotate`]).

use pnr_core::ids::{DeviceId, GroupId};

/// A recognised analog structure and one node of the block DAG: which devices
/// form it, what it is, and which blocks must be built before it.
///
/// The annotator fills this (mechanism); the analog theory (each `Rule::extract`)
/// decides what constraints it implies. A block whose devices were coalesced by a
/// rule's union-find becomes addressable as [`pnr_core::Target::Group`] via
/// [`Block::group`].
pub struct Block {
    pub kind: BlockKind,
    /// The catalog pattern that recognised this block (`"diff_pair"`,
    /// `"telescopic_ota_full"`, …), or `""` for glue / inferred blocks. Kept so
    /// reuse can key on "same template" (ALIGN `SameTemplate`) — two blocks with
    /// the same `template` and matching geometry are reuse candidates.
    pub template: &'static str,
    /// The devices comprising this block. For a recognised group this is the
    /// pattern's matched device set; for top-level glue it is the unclaimed
    /// remainder.
    pub devices: Vec<DeviceId>,
    /// The group id this block is placed as. Every block gets one so a placement
    /// rule can address the whole block via [`pnr_core::Target::Group`], and it
    /// indexes the [`pnr_core::Layout::groups`] table 1:1.
    pub group: GroupId,
    /// Blocks (by index into the DAG's `Vec<Block>`) that must be assembled/placed
    /// before this one. Empty for independent leaf blocks. The glue block depends
    /// on every recognised group. Acyclic by construction (leaves first, glue
    /// last), so the DAG can be walked concurrently by processing a block once all
    /// its `depends_on` are done.
    pub depends_on: Vec<usize>,
    /// `true` when this block's devices are covered by a **user-injected**
    /// `macro_master` macro — a pre-drawn part of the design. Such a node is placed
    /// as an opaque fixed macro: it skips auto-generation and recognition
    /// refinement (it is already a formed block). Set by the orchestrator, which
    /// owns the macro registry.
    pub injected: bool,
    /// Recognised **child primitives** nested inside this block (the hierarchy
    /// tree — ALIGN §2.5). A composite like a telescopic-OTA core is one placed /
    /// reusable block up top, but its internal diff pair, mirror, and cascodes live
    /// here as leaves. Constraints (symmetry/matching/CC) emit from these leaves —
    /// a composite would otherwise *swallow* the primitives that carry them. Empty
    /// for a block that is itself a primitive leaf, or whose internal structure the
    /// primitive tier didn't recognise. Children are **not** in the top-level DAG /
    /// group table (only parentless blocks are placed); they exist for constraint
    /// scoping and hierarchy queries.
    pub sub_blocks: Vec<Block>,
}

/// Known analog structures. Recognising which devices form one is the annotator's
/// job; the constraints each implies come from the analog rules' `extract`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    /// Differential pair ⇒ symmetry + matching(Cross) + thermal + common-centroid.
    DiffPair,
    /// Current mirror ⇒ matching(Mirror) + proximity to the reference.
    CurrentMirror,
    /// Cascode stack ⇒ proximity + matching.
    Cascode,
    /// Active/passive load ⇒ matching.
    Load,
    /// Bias generator ⇒ isolation + thermal reference.
    BiasGen,
    /// A coalesced device set from a rule's union-find whose specific topology was
    /// not classified by strict recognition — still a real, addressable group.
    Group,
    /// Unrecognised glue — no implied constraints. The top-level block.
    Glue,
}

impl BlockKind {
    /// Coarse map from a catalog pattern name to the constraint-bearing kind it
    /// implies. Keyed by substring so the 120-pattern catalog collapses onto the
    /// handful of kinds that drive constraint emission (see [`Block::template`] for
    /// the exact pattern). Larger composites (OTA cores, Gilbert cells) stay
    /// [`BlockKind::Group`] — a real addressable group whose members already carry
    /// their own primitive kinds where those primitives matched separately.
    #[must_use]
    pub fn from_template(template: &str) -> Self {
        let t = template;
        if t.contains("diff_pair") || t.contains("diff_switch") {
            BlockKind::DiffPair
        } else if t.contains("mirror") {
            BlockKind::CurrentMirror
        } else if t.contains("cascode") {
            BlockKind::Cascode
        } else if t.contains("load") {
            BlockKind::Load
        } else if t.contains("bias") || t.contains("reference") || t.contains("startup") {
            BlockKind::BiasGen
        } else {
            BlockKind::Group
        }
    }
}
