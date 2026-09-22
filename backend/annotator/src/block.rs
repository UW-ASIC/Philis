//! Recognised blocks — the annotator's output vocabulary.

use pnr_core::ids::DeviceId;

use crate::pattern::PatternMatch;

/// One recognised structure (or the glue remainder).
pub struct Block {
    pub kind: BlockKind,
    /// Members, in the matched pattern's slot order (slot 0 of a mirror is its
    /// diode-connected reference).
    pub devices: Vec<DeviceId>,
    /// Set by the orchestrator: every device is covered by a user-injected macro,
    /// so the block is placed as a fixed opaque macro.
    pub injected: bool,
    /// Primitive (≤2-device) children of a composite. Constraints emit from these
    /// leaves; they are not in the top-level group table.
    pub sub_blocks: Vec<Block>,
}

/// What a recognised structure implies for placement (see [`crate::emit`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    /// Symmetric matched pair: diff pair, diff switch, cross-coupled pair.
    DiffPair,
    /// Diode reference (slot 0) + output legs.
    CurrentMirror,
    /// Matched load pair (active / diode / triode load).
    Load,
    /// Series stack (cascode, follower on its current source): adjacent, not matched.
    Stack,
    /// Recognised but implies no pairwise constraint (composites, logic, bias).
    Group,
    /// Unrecognised remainder.
    Glue,
}

impl BlockKind {
    /// Map a catalog pattern name to its kind. Only 2-device primitives get a
    /// constraint-bearing kind; a composite's roles live in its children.
    #[must_use]
    pub fn from_template(t: &str, n_devices: usize) -> Self {
        if n_devices != 2 {
            BlockKind::Group
        } else if t.contains("diff_pair") || t.contains("diff_switch") || t.starts_with("cross_coupled") {
            BlockKind::DiffPair
        } else if t.contains("mirror") {
            BlockKind::CurrentMirror
        } else if t.contains("load") {
            BlockKind::Load
        } else if t.contains("cascode") || t == "source_follower" || t == "series_stack" {
            BlockKind::Stack
        } else {
            BlockKind::Group
        }
    }

    /// Devices whose gate nets are classified Sensitive.
    #[must_use]
    pub fn is_sensitive(self) -> bool {
        matches!(self, BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load | BlockKind::Stack)
    }
}

impl Block {
    pub(crate) fn from_match(m: &PatternMatch) -> Self {
        Block {
            kind: BlockKind::from_template(m.template, m.instances.len()),
            devices: m.instances.iter().map(|&d| DeviceId(d as u16)).collect(),
            injected: false,
            sub_blocks: Vec::new(),
        }
    }
}

/// The hierarchy's leaves: blocks without children, glue excluded.
pub(crate) fn leaves(blocks: &[Block]) -> Vec<&Block> {
    fn walk<'a>(bs: &'a [Block], out: &mut Vec<&'a Block>) {
        for b in bs {
            if !b.sub_blocks.is_empty() {
                walk(&b.sub_blocks, out);
            } else if b.kind != BlockKind::Glue {
                out.push(b);
            }
        }
    }
    let mut out = Vec::new();
    walk(blocks, &mut out);
    out
}
