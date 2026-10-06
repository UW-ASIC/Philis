//! Per-pair cell spacing from edge profiles (PLC-07): each cell face records how
//! far in each layer role sits from the bbox, and a role × role table from the
//! deck says how far two facing roles must be apart. Two cells then owe each
//! other `max(rule − inset_a − inset_b)`, not one worst-case scalar.

use pnr_core::{LayerId, Macro, MatchClass, NetId, Orient, Process};

/// Placement spacing roles. `*_in` / `*_out`: inside / outside the cell's own n-well.
/// `other`: a drawn layer no listed role maps to (always spaced at `fallback`).
pub const ROLES: [&str; 28] = [
    "nwell", "dnwell", "diff_in", "diff_out", "tap_in", "tap_out", "poly", "nsdm", "psdm", "li", "licon", "mcon",
    "met1", "npc", "rpm", "res_implant", "diode_mk", "rpoly", "diom", "via1", "met2", "via2", "met3", "via3", "met4",
    "pnp", "npn", "other",
];
/// The `Process` role each placement role reads; `""` = none. `Pdk::layer`
/// falls back to a deck layer of the same name, so `dnwell`, `npc`, `rpm`
/// resolve on sky130; `rpoly`/`diom` are sky130 `cell.layers` roles the
/// generators draw; `via1`..`met4` resolve by stack position (capacitor
/// plates and straps), `pnp`/`npn` are sky130's BJT id layers. Whether every drawn
/// layer is covered is checked by the library test
/// `shipped_cells_draw_no_unmapped_layer`, not by this list.
pub const DECK_ROLE: [&str; 28] = [
    "nwell", "dnwell", "diff", "diff", "tap", "tap", "poly", "nsdm", "psdm", "li", "licon", "mcon", "met1", "npc",
    "rpm", "res_implant", "diode_mk", "rpoly", "diom", "via1", "met2", "via2", "met3", "via3", "met4", "pnp", "npn", "",
];
/// Number of placement roles: the width of every per-role array here.
pub const N: usize = ROLES.len();
const _: () = assert!(N <= 32, "Edge::present is a u32");
// Indices into `ROLES`, by name (checked against the table below).
const NWELL: usize = 0;
const DIFF_IN: usize = 2;
const DIFF_OUT: usize = 3;
const TAP_IN: usize = 4;
const TAP_OUT: usize = 5;
const POLY: usize = 6;
const NSDM: usize = 7;
const PSDM: usize = 8;
const NPC: usize = 13;
const RPM: usize = 14;
const DIODE_MK: usize = 16;
const DIOM: usize = 18;
const PNP: usize = 25;
const NPN: usize = 26;
const OTHER: usize = N - 1;
const _: () = {
    let named: [(usize, &str); 15] = [
        (NWELL, "nwell"), (DIFF_IN, "diff_in"), (DIFF_OUT, "diff_out"), (TAP_IN, "tap_in"), (TAP_OUT, "tap_out"),
        (POLY, "poly"), (NSDM, "nsdm"), (PSDM, "psdm"), (NPC, "npc"), (RPM, "rpm"), (DIODE_MK, "diode_mk"),
        (DIOM, "diom"), (PNP, "pnp"), (NPN, "npn"), (OTHER, "other"),
    ];
    let mut k = 0;
    while k < named.len() {
        let (a, b) = (ROLES[named[k].0].as_bytes(), named[k].1.as_bytes());
        assert!(a.len() == b.len(), "role index constant out of sync with ROLES");
        let mut c = 0;
        while c < a.len() {
            assert!(a[c] == b[c], "role index constant out of sync with ROLES");
            c += 1;
        }
        k += 1;
    }
};
/// Roles whose facing shapes may touch and merge into one figure: implant and
/// mask layers (no net), and the n-well when both wells carry the same bulk net.
const MERGEABLE: [usize; 5] = [NWELL, NSDM, PSDM, NPC, RPM];
/// Marker (id) layers: no drawn material, so a same-role pair with no deck
/// value is `NoRule` (0), not `fallback`.
const MARKER: [usize; 4] = [DIODE_MK, DIOM, PNP, NPN];

/// Foreign poly keep-out from a matched device's diffusion, nm, by
/// [`MatchClass`]: Hastings rule 23, lower ends (H13-55, L42648–42654). Policy,
/// the same on every deck.
pub const FOREIGN_POLY_NM: [i32; 3] = [0, 3000, 5000];

/// The eight orients in `Orient as usize` order: `ORIENTS[o as usize] == o`.
pub const ORIENTS: [Orient; 8] =
    [Orient::R0, Orient::R90, Orient::R180, Orient::R270, Orient::Mx, Orient::Mx90, Orient::Mx180, Orient::Mx270];

/// A bbox face, by outward normal; the discriminant indexes [`Profile::edge`]
/// and the `[L, B, R, T]` arrays of [`oriented_faces`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    /// Left, normal `−x`.
    L = 0,
    /// Bottom, normal `−y`.
    B = 1,
    /// Right, normal `+x`.
    R = 2,
    /// Top, normal `+y`.
    T = 3,
}

impl Face {
    /// Every face in discriminant order.
    const ALL: [Face; 4] = [Face::L, Face::B, Face::R, Face::T];
    /// The facing face across a gap: `L`↔`R`, `B`↔`T`.
    #[must_use]
    pub fn opposite(self) -> Face {
        Face::ALL[(self as usize + 2) % 4]
    }
    /// Outward unit normal `(dx, dy)`.
    fn normal(self) -> (i32, i32) {
        [(-1, 0), (0, -1), (1, 0), (0, 1)][self as usize]
    }
}

/// Distance from the bbox face to the nearest shape of each role, nm (`N` ≤ 32);
/// `i32::MAX` = role absent. Bit `r` of `present` is set iff `inset[r] < i32::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Per role, nm from the face to its nearest shape; ≥ 0 once [`Edge::put`].
    pub inset: [i32; N],
    /// Bit `r` set iff role `r` has a shape on this face.
    pub present: u32,
}

impl Default for Edge {
    fn default() -> Self {
        Edge { inset: [i32::MAX; N], present: 0 }
    }
}

impl Edge {
    /// Record a shape of role `r` at `inset` (clamped at 0); keeps the
    /// smaller of the old and new inset.
    ///
    /// # Panics
    /// When `r >= N`.
    pub fn put(&mut self, r: usize, inset: i32) {
        self.inset[r] = self.inset[r].min(inset.max(0));
        self.present |= 1 << r;
    }
}

/// One drawn cell's four faces, in its R0 frame unless [`oriented`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    /// Per [`Face`] (indexed by its discriminant), the roles on that face.
    pub edge: [Edge; 4],
    /// The cell's bulk net: two n-wells merge only on the same one.
    pub well_net: Option<NetId>,
    /// The cell's matching class, `None` when unmatched (PLC-13 keep-outs).
    pub matched: Option<MatchClass>,
    /// The cell's orient set (`dp::locks::Locks::orient_of`): partners in one
    /// set are exempt from each other's keep-outs.
    pub set: Option<u16>,
}

/// Where a [`SpacingTable::rule`] entry came from (diagnostics and tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Src {
    /// The process deck's `space` / `space_between`.
    Deck,
    /// A `placement_space` sidecar row larger than the deck's.
    Sidecar,
    /// The scalar `fallback`: same non-marker role or `other` with no value.
    Fallback,
    /// No rule: the pair owes nothing (value 0).
    NoRule,
}

/// Legal gaps between two facing cells: exactly `0` when `abut`, else any
/// `g >= min` (nm, lattice-rounded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    /// Touching (gap 0) is legal: no non-merging rule binds the pair.
    pub abut: bool,
    /// Smallest legal non-touching gap, nm, ≥ 0 and a multiple of the lattice.
    pub min: i32,
}

/// Role × role spacing, nm, symmetric.
#[derive(Clone, Debug)]
pub struct SpacingTable {
    /// `rule[i][j]`: minimum distance between a role-`i` and a role-`j`
    /// shape on facing cells, nm; `≤ 0` = none. Symmetric.
    pub rule: [[i32; N]; N],
    /// Provenance of each `rule` entry.
    pub src: [[Src; N]; N],
    /// Placement lattice, nm: every [`Gap::min`] rounds up to it (`≤ 0` reads as 1).
    pub lattice: i32,
    /// Today's scalar: same-role pairs without a deck value and every pair
    /// with `other`; also the gap when a cell has no profile.
    pub fallback: i32,
    /// Foreign n-well keep-out from a matched cell's `diff_out`, by
    /// [`MatchClass`] (the deck's `wpe_clearance_nm` tier). Zero = none.
    pub wpe: [i32; 3],
    /// Foreign poly keep-out from a matched cell's diffusion, by
    /// [`MatchClass`] ([`FOREIGN_POLY_NM`]). Zero = none.
    pub foreign_poly: [i32; 3],
}

/// Placement roles a sidecar role names: `diff`/`tap` cover both sides of
/// the well; empty for an unknown name.
fn expand(role: &str) -> Vec<usize> {
    match role {
        "diff" => vec![DIFF_IN, DIFF_OUT],
        "tap" => vec![TAP_IN, TAP_OUT],
        _ => ROLES.iter().position(|&r| r == role).into_iter().collect(),
    }
}

impl SpacingTable {
    /// Deck first (`space` for one deck role, else `space_between` both
    /// orders), then each sidecar `(role_a, role_b, nm)` where larger, then
    /// marker same-role 0, other same-role `fallback`, cross-role 0 (the deck
    /// states no spacing, so DRC cannot flag one); `other` pairs `fallback`.
    ///
    /// # Panics
    /// On a sidecar role not in [`ROLES`] (nor `diff`/`tap`): a typo must not
    /// silently drop a rule.
    #[must_use]
    pub fn new(p: &dyn Process, sidecar: &[(String, String, i32)], fallback: i32, lattice: i32) -> Self {
        let mut rule: [[Option<(i32, Src)>; N]; N] = [[None; N]; N];
        for i in 0..OTHER {
            for j in 0..OTHER {
                let (di, dj) = (DECK_ROLE[i], DECK_ROLE[j]);
                let v = if di == dj { p.space(di) } else { p.space_between(di, dj).max(p.space_between(dj, di)) };
                rule[i][j] = v.map(|v| (v, Src::Deck));
            }
        }
        for (a, b, nm) in sidecar {
            let (ra, rb) = (expand(a), expand(b));
            assert!(!ra.is_empty() && !rb.is_empty(), "placement_space: unknown role in `{a},{b}`");
            for &i in &ra {
                for &j in &rb {
                    if rule[i][j].is_none_or(|(v, _)| *nm > v) {
                        rule[i][j] = Some((*nm, Src::Sidecar));
                        rule[j][i] = rule[i][j];
                    }
                }
            }
        }
        let mut t = SpacingTable { rule: [[0; N]; N], src: [[Src::NoRule; N]; N], lattice, fallback, wpe: [0; 3], foreign_poly: [0; 3] };
        for i in 0..N {
            for j in 0..N {
                let (v, s) = match rule[i][j] {
                    _ if i == OTHER || j == OTHER => (fallback, Src::Fallback),
                    Some(r) => r,
                    None if DECK_ROLE[i] == DECK_ROLE[j] && !MARKER.contains(&i) => (fallback, Src::Fallback),
                    None => (0, Src::NoRule),
                };
                t.rule[i][j] = v;
                t.src[i][j] = s;
            }
        }
        t
    }

    /// Every pair at `fallback` (all [`Src::Fallback`], no keep-outs): today's
    /// scalar clearance, for callers without profiles.
    #[must_use]
    pub fn uniform(fallback: i32, lattice: i32) -> Self {
        SpacingTable { rule: [[fallback; N]; N], src: [[Src::Fallback; N]; N], lattice, fallback, wpe: [0; 3], foreign_poly: [0; 3] }
    }

    /// The largest gap any pair can owe (insets are ≥ 0), keep-outs included, lattice-rounded.
    #[must_use]
    pub fn max_gap(&self) -> i32 {
        let keep = self.wpe.iter().chain(&self.foreign_poly).copied().max().unwrap_or(0);
        round_up(self.rule.iter().flatten().copied().max().unwrap_or(0).max(self.fallback).max(keep), self.lattice)
    }

    /// Keep-outs matched `m` (face `em`) owes `o` (face `eo`) unless they share
    /// an orient set: `o`'s n-well ≥ `wpe` from `m`'s `diff_out`, `o`'s poly ≥
    /// `foreign_poly` from `m`'s diffusion. Never mergeable.
    fn keep(&self, m: &Profile, em: &Edge, o: &Profile, eo: &Edge) -> i32 {
        let Some(c) = m.matched.filter(|_| m.set.is_none() || m.set != o.set) else { return 0 };
        let need = |d: usize, r: usize, k: i32| {
            let on = em.present & (1 << d) != 0 && eo.present & (1 << r) != 0 && k > 0;
            if on { k - em.inset[d] - eo.inset[r] } else { 0 }
        };
        let (w, p) = (self.wpe[c as usize], self.foreign_poly[c as usize]);
        need(DIFF_OUT, NWELL, w).max(need(DIFF_IN, POLY, p)).max(need(DIFF_OUT, POLY, p))
    }

    /// Gap `a`'s face `fa` owes `b`'s opposite face, per facing role pair
    /// `rule − inset_a − inset_b`, maxed, floored at 0, rounded up to the
    /// lattice. Exact for any insets (no overflow, even near `i32::MAX`).
    /// Symmetric: `gap(a, f, b) == gap(b, f.opposite(), a)`. A facing pair that merges
    /// at contact (same `MERGEABLE` role at inset 0 on both, n-well only on one
    /// bulk net) still sets `min` but not `abut`: any gap in `(0, min)` would
    /// leave a notch the bridges do not fill. Matched cells' keep-outs
    /// ([`Self::keep`], PLC-13) apply both ways and are hard.
    #[must_use]
    pub fn gap(&self, a: &Profile, fa: Face, b: &Profile) -> Gap {
        let (ea, eb) = (&a.edge[fa as usize], &b.edge[fa.opposite() as usize]);
        let (mut g_all, mut g_hard) = (0, 0);
        let mut ia = ea.present;
        while ia != 0 {
            let i = ia.trailing_zeros() as usize;
            ia &= ia - 1;
            let mut jb = eb.present;
            while jb != 0 {
                let j = jb.trailing_zeros() as usize;
                jb &= jb - 1;
                let r = self.rule[i][j];
                if r <= 0 {
                    continue;
                }
                let need = r - ea.inset[i] - eb.inset[j];
                g_all = g_all.max(need);
                let merge = i == j
                    && MERGEABLE.contains(&i)
                    && ea.inset[i] == 0
                    && eb.inset[j] == 0
                    && (i != NWELL || (a.well_net.is_some() && a.well_net == b.well_net));
                if !merge {
                    g_hard = g_hard.max(need);
                }
            }
        }
        let k = self.keep(a, ea, b, eb).max(self.keep(b, eb, a, ea));
        let (g_all, g_hard) = (g_all.max(k), g_hard.max(k));
        Gap { abut: g_hard <= 0, min: round_up(g_all, self.lattice) }
    }

    /// Room a cell can owe beyond its bbox: over faces and present roles `i`,
    /// `(max_j rule[i][j] − inset_i)⁺`, `j` over every role but `other` (shipped
    /// cells draw none; region sizing only, never legality), raised by a
    /// matched cell's keep-outs on its diffusion.
    #[must_use]
    pub fn halo(&self, p: &Profile) -> i32 {
        let (w, fp) = p.matched.map_or((0, 0), |c| (self.wpe[c as usize], self.foreign_poly[c as usize]));
        let mut h = 0;
        for e in &p.edge {
            for i in (0..N).filter(|&i| e.present & (1 << i) != 0) {
                let keep = match i {
                    DIFF_IN => fp,
                    DIFF_OUT => w.max(fp),
                    _ => 0,
                };
                let reach = if i == OTHER { self.fallback } else { self.rule[i][..OTHER].iter().copied().max().unwrap_or(0).max(keep) };
                h = h.max(reach - e.inset[i]);
            }
        }
        h
    }
}

/// Smallest multiple of `lattice` (`≤ 0` reads as 1) that is ≥ `v`,
/// saturating below `i32::MAX`.
fn round_up(v: i32, lattice: i32) -> i32 {
    let l = lattice.max(1);
    (v + l - 1).div_euclid(l) * l
}

/// The role a shape on `layer` plays, by first match over [`DECK_ROLE`];
/// [`OTHER`] when none. `layers[r]` = `p.layer(DECK_ROLE[r])`.
fn role_of(layers: &[Option<LayerId>; N], layer: LayerId) -> usize {
    (0..OTHER).find(|&r| layers[r] == Some(layer)).unwrap_or(OTHER)
}

/// `p.layer` of every role's deck role, once per profiled macro.
#[must_use]
pub fn role_layers(p: &dyn Process) -> [Option<LayerId>; N] {
    std::array::from_fn(|r| if r == OTHER { None } else { p.layer(DECK_ROLE[r]) })
}

/// `m`'s profile in its R0 frame, `well_net = bulk`, unmatched, no set.
/// diff/tap count as `_in` only when contained in one of `m`'s own n-well
/// rects (a shape straddling two is read `_out`: the stricter rows). Inset
/// per face from `m.bbox`, 0 when touching or poking out. O(shapes · wells).
#[must_use]
pub fn profile(m: &Macro, p: &dyn Process, bulk: Option<NetId>) -> Profile {
    let layers = role_layers(p);
    let wells: Vec<_> = m.shapes.iter().filter(|s| Some(s.layer) == layers[NWELL]).map(|s| s.rect).collect();
    let b = m.bbox;
    let mut out = Profile { well_net: bulk, ..Profile::default() };
    for s in &m.shapes {
        let mut r = role_of(&layers, s.layer);
        if r == DIFF_IN || r == TAP_IN {
            let s = s.rect;
            let inside = wells.iter().any(|w| w.x <= s.x && w.y <= s.y && s.x + s.w <= w.x + w.w && s.y + s.h <= w.y + w.h);
            r += usize::from(!inside);
        }
        let s = s.rect;
        out.edge[Face::L as usize].put(r, s.x - b.x);
        out.edge[Face::B as usize].put(r, s.y - b.y);
        out.edge[Face::R as usize].put(r, b.x + b.w - (s.x + s.w));
        out.edge[Face::T as usize].put(r, b.y + b.h - (s.y + s.h));
    }
    out
}

/// `p` as drawn under `o`: each face moves to where `o.apply` turns its outward
/// normal (the same map `place_macro` stamps with).
#[must_use]
pub fn oriented(p: &Profile, o: Orient) -> Profile {
    let mut out = Profile { edge: [Edge::default(); 4], ..*p };
    for f in Face::ALL {
        out.edge[face_to(f, o)] = p.edge[f as usize];
    }
    out
}

/// Where R0 face `f` lands under `o`, as a [`Face`] index.
fn face_to(f: Face, o: Orient) -> usize {
    let n = o.apply(f.normal().0, f.normal().1);
    Face::ALL.iter().position(|g| g.normal() == n).expect("D4 maps axis normals to axis normals")
}

/// Per-face values `h` (L, B, R, T, R0 frame) as placed under `o`: the map
/// [`oriented`] applies to a profile.
#[must_use]
pub fn oriented_faces(h: [i32; 4], o: Orient) -> [i32; 4] {
    let mut out = [0; 4];
    for f in Face::ALL {
        out[face_to(f, o)] = h[f as usize];
    }
    out
}

/// Every drawn cell's profile under every orient, precomputed once.
#[derive(Clone, Debug, Default)]
pub struct Profiles {
    /// `of[cell][variant][orient as usize]`; a missing cell or variant has no
    /// profile (spaced at `fallback`).
    pub of: Vec<Vec<[Profile; 8]>>,
}

impl Profiles {
    /// All 8 orients of one R0 profile, indexed by `Orient as usize`.
    #[must_use]
    pub fn orients(p: &Profile) -> [Profile; 8] {
        ORIENTS.map(|o| oriented(p, o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rules: &[(usize, usize, i32)]) -> SpacingTable {
        let mut t = SpacingTable { rule: [[0; N]; N], src: [[Src::NoRule; N]; N], lattice: 5, fallback: 1270, wpe: [0; 3], foreign_poly: [0; 3] };
        for &(i, j, v) in rules {
            t.rule[i][j] = v;
            t.rule[j][i] = v;
            t.src[i][j] = Src::Deck;
            t.src[j][i] = Src::Deck;
        }
        for k in 0..N {
            t.rule[k][OTHER] = 1270;
            t.rule[OTHER][k] = 1270;
        }
        t
    }

    /// One face `f` carrying `(role, inset)`s.
    fn face(f: Face, roles: &[(usize, i32)], net: Option<u32>) -> Profile {
        let mut p = Profile { well_net: net.map(|n| NetId(n as _)), ..Profile::default() };
        for &(r, i) in roles {
            p.edge[f as usize].put(r, i);
        }
        p
    }

    #[test]
    fn ndiff_facing_foreign_nwell_needs_rule_minus_insets() {
        let t = table(&[(DIFF_OUT, NWELL, 340)]);
        let a = face(Face::R, &[(DIFF_OUT, 200)], Some(1));
        let b = face(Face::L, &[(NWELL, 0)], Some(2));
        assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: false, min: 140 });
    }

    #[test]
    fn same_bulk_pmos_may_abut() {
        let t = table(&[(NWELL, NWELL, 1270), (DIFF_IN, DIFF_IN, 270)]);
        let a = face(Face::R, &[(NWELL, 0), (DIFF_IN, 180)], Some(1));
        let b = face(Face::L, &[(NWELL, 0), (DIFF_IN, 180)], Some(1));
        assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: true, min: 1270 });
    }

    #[test]
    fn foreign_wells_never_abut() {
        let t = table(&[(NWELL, NWELL, 1270), (DIFF_IN, DIFF_IN, 270)]);
        let a = face(Face::R, &[(NWELL, 0), (DIFF_IN, 180)], Some(1));
        let b = face(Face::L, &[(NWELL, 0), (DIFF_IN, 180)], Some(2));
        assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: false, min: 1270 });
    }

    #[test]
    fn nmos_pair_with_edge_implants() {
        let t = table(&[(NSDM, NSDM, 380), (DIFF_OUT, DIFF_OUT, 270)]);
        let a = face(Face::T, &[(NSDM, 0), (DIFF_OUT, 125)], None);
        let b = face(Face::B, &[(NSDM, 0), (DIFF_OUT, 125)], None);
        assert_eq!(t.gap(&a, Face::T, &b), Gap { abut: false, min: 380 });
    }

    #[test]
    fn rotated_profile_permutes_faces() {
        let mut p = Profile::default();
        for (k, f) in Face::ALL.iter().enumerate() {
            p.edge[*f as usize].put(k, 10 * k as i32);
        }
        let at = |q: &Profile, f: Face| q.edge[f as usize];
        let r90 = oriented(&p, Orient::R90);
        for (from, to) in [(Face::L, Face::B), (Face::B, Face::R), (Face::R, Face::T), (Face::T, Face::L)] {
            assert_eq!(at(&r90, to), at(&p, from), "R90 {from:?} -> {to:?}");
        }
        let mx = oriented(&p, Orient::Mx);
        for (from, to) in [(Face::B, Face::T), (Face::T, Face::B), (Face::L, Face::L), (Face::R, Face::R)] {
            assert_eq!(at(&mx, to), at(&p, from), "Mx {from:?} -> {to:?}");
        }
        // Halos (PLC-15) follow the same map.
        assert_eq!(oriented_faces([1, 2, 3, 4], Orient::R90), [4, 1, 2, 3]);
        assert_eq!(oriented_faces([1, 2, 3, 4], Orient::Mx), [1, 4, 3, 2]);
    }

    #[test]
    fn unmapped_layer_uses_the_fallback() {
        let t = table(&[]);
        let a = face(Face::R, &[(OTHER, 0)], None);
        let b = face(Face::L, &[(OTHER, 0)], None);
        assert_eq!(t.gap(&a, Face::R, &b).min, 1270);
    }

    #[test]
    fn diode_markers_do_not_force_the_fallback() {
        let t = table(&[]);
        let a = face(Face::R, &[(DIOM, 0)], None);
        let b = face(Face::L, &[(DIOM, 0)], None);
        assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: true, min: 0 });
    }

    /// The deck table plus sky130's `wpe_clearance_nm` tiers and the policy poly keep-out.
    fn keep_table() -> SpacingTable {
        SpacingTable { wpe: [2000, 3000, 5000], foreign_poly: FOREIGN_POLY_NM, ..table(&[(DIFF_OUT, NWELL, 340)]) }
    }

    /// `a`: `diff_out` at 200 on R in orient set `set`, class `matched`; `b`: n-well at 0 on L.
    fn wpe_pair(matched: Option<MatchClass>, set: Option<u16>) -> (Profile, Profile) {
        let a = Profile { matched, set, ..face(Face::R, &[(DIFF_OUT, 200)], Some(1)) };
        let b = Profile { set, ..face(Face::L, &[(NWELL, 0)], Some(2)) };
        (a, b)
    }

    #[test]
    fn matched_nmos_keeps_wpe_distance_from_foreign_well() {
        let t = keep_table();
        let (a, b) = wpe_pair(Some(MatchClass::Moderate), None);
        assert_eq!(t.gap(&a, Face::R, &b), Gap { abut: false, min: 2800 });
        assert_eq!(t.gap(&b, Face::L, &a), Gap { abut: false, min: 2800 });
        assert!(t.max_gap() >= 5000 && t.halo(&a) >= 3000 - 200);
    }

    #[test]
    fn unmatched_nmos_uses_the_deck_rule() {
        let (a, b) = wpe_pair(None, None);
        assert_eq!(keep_table().gap(&a, Face::R, &b).min, 140);
    }

    #[test]
    fn same_set_partners_are_exempt() {
        let (a, b) = wpe_pair(Some(MatchClass::Moderate), Some(0));
        assert_eq!(keep_table().gap(&a, Face::R, &b).min, 140);
    }

    #[test]
    fn matched_diff_keeps_foreign_poly_away() {
        let a = Profile { matched: Some(MatchClass::Exceptional), ..face(Face::R, &[(DIFF_IN, 100)], Some(1)) };
        let b = face(Face::L, &[(POLY, 0)], None);
        assert_eq!(keep_table().gap(&a, Face::R, &b), Gap { abut: false, min: 4900 });
    }

    #[test]
    fn round_up_is_a_ceiling_on_the_lattice() {
        assert_eq!(round_up(0, 5), 0);
        assert_eq!(round_up(1, 5), 5);
        assert_eq!(round_up(5, 5), 5);
        assert_eq!(round_up(-3, 5), 0);
        assert_eq!(round_up(-5, 5), -5);
        assert_eq!(round_up(-6, 5), -5);
        assert_eq!(round_up(7, 0), 7, "lattice 0 reads 1");
        assert_eq!(round_up(7, -4), 7, "negative lattice reads 1");
        assert_eq!(round_up(i32::MAX, 10), 2_147_483_640, "saturates below i32::MAX");
        assert_eq!(round_up(i32::MAX, 1), i32::MAX);
        assert_eq!(round_up(i32::MIN, 10), -2_147_483_640);
    }

    #[test]
    fn expand_names() {
        assert_eq!(expand("diff"), vec![DIFF_IN, DIFF_OUT]);
        assert_eq!(expand("tap"), vec![TAP_IN, TAP_OUT]);
        assert_eq!(expand("poly"), vec![POLY]);
        assert_eq!(expand("other"), vec![OTHER]);
        assert!(expand("diff_inn").is_empty());
        assert!(expand("").is_empty());
    }

    #[test]
    fn role_of_takes_the_first_matching_role() {
        let mut layers = [None; N];
        layers[DIFF_IN] = Some(LayerId(2));
        layers[DIFF_OUT] = Some(LayerId(2));
        assert_eq!(role_of(&layers, LayerId(2)), DIFF_IN);
        assert_eq!(role_of(&layers, LayerId(3)), OTHER);
        layers[OTHER] = Some(LayerId(3));
        assert_eq!(role_of(&layers, LayerId(3)), OTHER, "`other` is never matched by layer");
    }
}
