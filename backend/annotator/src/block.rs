//! Recognised blocks — the annotator's output vocabulary.

use pnr_core::ids::DeviceId;

use crate::pattern::PatternMatch;

/// One recognised structure (or the glue remainder).
pub struct Block {
    /// What the structure implies for placement; `Group` for a composite
    /// whose constraints live in `sub_blocks`.
    pub kind: BlockKind,
    /// Matched pattern's name (`"glue"` for the remainder); a child carries its parent's.
    pub template: &'static str,
    /// Members, in the matched pattern's slot order (slot 0 of a mirror is its
    /// diode-connected reference).
    pub devices: Vec<DeviceId>,
    /// Set by the orchestrator: every device is covered by a user-injected macro,
    /// so the block is placed as a fixed opaque macro.
    pub injected: bool,
    /// A composite's declared couples ([`crate::catalog::roles_of`]): one 2-device
    /// child per `pairs` entry (its kind), then per `prox` entry (`Stack`).
    /// Constraints emit from these leaves; they are not in the top-level group table.
    pub sub_blocks: Vec<Block>,
    /// Declared on-axis members (tail, shared bias device), slot order.
    pub selfs: Vec<DeviceId>,
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
    /// Symmetric pair of cascodes: matched for symmetry, not a gate reference.
    CascodePair,
    /// Series stack (cascode, follower on its current source): adjacent, not matched.
    Stack,
    /// Recognised but implies no pairwise constraint (composites, logic, bias).
    Group,
    /// Unrecognised remainder.
    Glue,
}

impl BlockKind {
    /// Map a catalog pattern name to its kind. Only 2-device primitives get a
    /// constraint-bearing kind; a composite's roles live in its children
    /// ([`crate::catalog::roles_of`], whose 2-slot fallback this is).
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

    /// Devices whose gate nets are Sensitive and which are isolation victims; Stack and
    /// CascodePair are adjacent/symmetric, not a gate reference.
    #[must_use]
    pub fn is_sensitive(self) -> bool {
        matches!(self, BlockKind::DiffPair | BlockKind::CurrentMirror | BlockKind::Load)
    }
}

impl Block {
    /// A 2-device match is its own leaf, its kind from its roles (the declared
    /// pair's kind, else `Stack` for a declared prox, else `Group`); a larger one is
    /// a `Group` whose children and selfs are its pattern's declared roles, devices
    /// in slot order.
    ///
    /// Device ids are narrowed to `u16`: `annotate` refuses netlists past the
    /// `u16` id space before matching, so the cast never truncates.
    pub(crate) fn from_match(m: &PatternMatch) -> Self {
        let dev = |s: u8| DeviceId(m.instances[s as usize] as u16);
        let n = m.instances.len();
        let roles = crate::catalog::roles_of(m.pattern);
        let leaf = |kind, a, b| Block {
            kind,
            template: m.template,
            devices: vec![dev(a), dev(b)],
            injected: false,
            sub_blocks: Vec::new(),
            selfs: Vec::new(),
        };
        let sub_blocks = if n == 2 {
            Vec::new()
        } else {
            let pairs = roles.pairs.iter().map(|&(a, b, k)| leaf(k, a, b));
            pairs.chain(roles.prox.iter().map(|&(a, b)| leaf(BlockKind::Stack, a, b))).collect()
        };
        let kind = match (n, roles.pairs.first(), roles.prox.is_empty()) {
            (2, Some(&(_, _, k)), _) => k,
            (2, None, false) => BlockKind::Stack,
            _ => BlockKind::Group,
        };
        Block {
            kind,
            template: m.template,
            devices: m.instances.iter().map(|&d| DeviceId(d as u16)).collect(),
            injected: false,
            sub_blocks,
            selfs: roles.selfs.iter().map(|&s| dev(s)).collect(),
        }
    }
}

/// The hierarchy's leaves: blocks without children, glue excluded, in
/// depth-first order (a composite's children where the composite stood).
/// Borrows `blocks`; allocates one `Vec` of references.
#[must_use]
pub fn leaves(blocks: &[Block]) -> Vec<&Block> {
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
