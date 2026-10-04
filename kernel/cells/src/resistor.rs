//! Resistor generator: each device a series chain of `segments` sky130
//! precision poly resistors — a continuous poly strip whose body is marked by
//! `rpoly`, with long contacted heads, all under rpm + npc + psdm.

use crate::builder::dim;
use analog::cell::SeriesParallel;
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, Macro, Node, Process, Rect};

use crate::builder::{
    cut_lattice, greedy_centroid, pin, req, sizing, snap_cut, unitization, Builder, Sizing,
};
use crate::{Cell, Pattern};

/// A recipe's per-device resistance, integers from the sidecar:
/// R = sheet·(L + dl)/weff + head/(weff + head_dw),  weff = W + dw − narrow·max(knee − W, 0),
/// W and L in µm (Hastings eq 6.22: a body's value plus its two heads, so
/// splitting a body into n devices adds n − 1 head terms).
#[derive(Clone, Copy, Debug)]
pub struct ResModel {
    pub sheet_mohm: i64,
    pub head_mohm_um: i64,
    pub dl_nm: i32,
    pub dw_nm: i32,
    pub head_dw_nm: i32,
    pub narrow_permille: i32,
    pub knee_nm: i32,
}

impl ResModel {
    /// `None` when the recipe states no `res_sheet_mohm`.
    #[must_use]
    pub fn of(p: &dyn Process) -> Option<Self> {
        let sheet = p.rule("res_sheet_mohm", 0);
        if sheet <= 0 {
            return None;
        }
        let r = |name: &str| p.rule(name, 0);
        Some(Self {
            sheet_mohm: i64::from(sheet),
            head_mohm_um: i64::from(r("res_head_mohm_um")),
            dl_nm: r("res_dl_nm"),
            dw_nm: r("res_dw_nm"),
            head_dw_nm: r("res_head_dw_nm"),
            narrow_permille: r("res_narrow_permille"),
            knee_nm: r("res_knee_nm"),
        })
    }

    /// Effective width, µm.
    fn weff(&self, w_nm: i32) -> f64 {
        let w = f64::from(w_nm) / 1e3;
        w + f64::from(self.dw_nm) / 1e3
            - f64::from(self.narrow_permille) / 1e3 * (f64::from(self.knee_nm) / 1e3 - w).max(0.0)
    }

    /// Both heads of one device, Ω.
    fn head_ohm(&self, weff: f64) -> f64 {
        self.head_mohm_um as f64 / 1e3 / (weff + f64::from(self.head_dw_nm) / 1e3)
    }

    /// One device of drawn W×L, Ω.
    #[must_use]
    pub fn ohm(&self, w_nm: i32, l_nm: i32) -> f64 {
        let weff = self.weff(w_nm);
        self.sheet_mohm as f64 / 1e3 * (f64::from(l_nm) + f64::from(self.dl_nm)) / 1e3 / weff
            + self.head_ohm(weff)
    }

    /// Body length so `n` series devices of width `w` total `target`:
    /// L = (target/n − head/(weff+head_dw))·weff/sheet − dl, snapped to `lat`;
    /// `None` below `min_nm` or when the snapped residual exceeds `tol_ppm`.
    #[must_use]
    pub fn seg_len(
        &self,
        w_nm: i32,
        target_ohm: f64,
        n: u32,
        min_nm: i32,
        lat: i32,
        tol_ppm: i32,
    ) -> Option<i32> {
        let weff = self.weff(w_nm);
        let l_um = (target_ohm / f64::from(n.max(1)) - self.head_ohm(weff)) * weff
            / (self.sheet_mohm as f64 / 1e3)
            - f64::from(self.dl_nm) / 1e3;
        let lat = lat.max(1);
        let l = ((l_um * 1e3 / f64::from(lat)).round() as i32).saturating_mul(lat);
        let err = (f64::from(n) * self.ohm(w_nm, l) - target_ohm).abs() / target_ohm * 1e6;
        (l >= min_nm && err <= f64::from(tol_ppm)).then_some(l)
    }
}

/// How far a segmented resistor may drift from its schematic value, ppm
/// (`res_value_tol_ppm`): lengths snap to the cut lattice.
#[must_use]
pub fn value_tol_ppm(p: &dyn Process) -> i32 {
    p.rule("res_value_tol_ppm", 0)
}

/// Parallel strings per member: `dev_nf[d]` when the unitization composes
/// units in parallel (SPICE `m`), else one.
fn strings(group: &DeviceGroup, c: &Constraints, s: &Sizing) -> Vec<usize> {
    let parallel =
        unitization(group, c).is_some_and(|u| u.series_parallel == SeriesParallel::Parallel);
    s.dev_nf
        .iter()
        .map(|&nf| if parallel { usize::from(nf.max(1)) } else { 1 })
        .collect()
}

/// One resistor variant: each string split into `segments` series segments
/// whose lengths keep the string's model value; member d draws `dev_nf[d]`
/// parallel strings when its unitization is `Parallel`. Strings laid out
/// `Single` (each string's segments adjacent) or `Interdig`.
#[derive(Clone)]
pub struct Resistor {
    pub segments: u16,
    pub pattern: Pattern,
}

impl Cell for Resistor {
    fn enumerate(
        group: &DeviceGroup,
        constraints: &Constraints,
        process: &dyn Process,
    ) -> Vec<Self> {
        // No resistor marker, no poly resistor on this process.
        if group.devices.is_empty() || process.layer("rpoly").is_none() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let patterns = if group.devices.len() > 1 {
            vec![Pattern::Single, Pattern::Interdig]
        } else {
            vec![Pattern::Single]
        };
        // Interleaved multi-segment devices jump between their columns on
        // met1, one track per device inside the heads: as many devices as
        // tracks fit.
        let tracks = jumper_tracks(process).len();
        let n_strings: usize = strings(group, constraints, &s).iter().sum();
        feasible_segments(&s, process)
            .into_iter()
            .flat_map(|segments| {
                patterns
                    .iter()
                    .map(move |&pattern| Resistor { segments, pattern })
            })
            .filter(|r| {
                !(matches!(r.pattern, Pattern::Interdig) && r.segments > 1 && n_strings > tracks)
            })
            .collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let mut b = Builder::new(process.grid());
        let s = group_sizing(group, constraints, process);
        let n_dev = group.devices.len();
        let r = |name: &str, default: i32| process.rule(name, default);

        let poly = req(process, "poly");
        let rpoly = req(process, "rpoly");
        let li = req(process, "li");
        let licon = req(process, "licon");
        let (mcon, met1) = (req(process, "mcon"), req(process, "met1"));
        let (m_ct, m_enc) = (dim(process, "mcon_size"), dim(process, "m1_enc"));
        let tracks = jumper_tracks(process);
        let lat = cut_lattice(process);

        let ct = dim(process, "contact");
        // Head contact: a square cut, or the deck's precision-resistor slot
        // where it states one (sky130 licon.1b: 190 x 2000 inside rpm).
        let (cw, ch) = slot(process);
        let cut_space = process.space("licon").unwrap_or(ct);
        // Poly past a cut, and li past a cut, on every side.
        let border = r("poly_encloses_licon_one_side", 0)
            .max(r("li_encloses_licon", 0))
            .max(process.enclosure("poly", "licon").unwrap_or(0))
            .max(process.endcap("poly", "licon").unwrap_or(0))
            .max(process.endcap("li", "licon").unwrap_or(0));
        let head = head_len(process);
        let lap = LAP;
        // A cut's inner side keeps the marker's spacing too, where the deck
        // states one.
        let inner = border
            .max(process.space_between("rpoly", "licon").unwrap_or(0))
            .max(process.space_between("rpoly_b", "licon").unwrap_or(0))
            .max(process.space_between("res_block", "licon").unwrap_or(0));
        let seg_gap = seg_gap(process);
        let n_segments = i32::from(self.segments.max(1));
        let body_w = s.unit_w.max(r("res_min_width", 0));
        // Each segment's length keeps the string at the schematic device's
        // model value (heads included); without a model only n = 1 is
        // offered, which draws the written L.
        let seg_l = ResModel::of(process)
            .and_then(|m| {
                m.seg_len(
                    body_w,
                    m.ohm(s.unit_w, s.unit_l),
                    n_segments as u32,
                    0,
                    lat,
                    i32::MAX,
                )
            })
            .unwrap_or(s.unit_l / n_segments)
            .max(process.width("rpoly").unwrap_or(0));
        let seg_pitch = body_w + seg_gap;
        let total_h = head + seg_l + head;
        let head_li_h = head - lap;

        // Contact column of a head, outer end first (the top head mirrors it).
        let cut_x = snap_cut(body_w / 2 - cw / 2, lat);
        let cut_ys: Vec<i32> = (0..)
            .map(|k| border + k * (ch + cut_space))
            .take_while(|&y| y + ch + inner <= head_li_h)
            .collect();
        // The outermost cut of a head: where the pins land.
        let end_cut = |sx: i32, top: bool| Rect {
            x: sx + cut_x,
            y: if top { total_h - border - ch } else { border },
            w: cw,
            h: ch,
        };

        // String g is string j of member di: one pseudo-device for sequencing.
        let per = strings(group, constraints, &s);
        let owner: Vec<(usize, usize)> = per
            .iter()
            .enumerate()
            .flat_map(|(d, &k)| (0..k).map(move |j| (d, j)))
            .collect();
        let sequence = res_segment_sequence(owner.len(), self.pattern, n_segments);
        let mut seg_of = vec![0i32; owner.len()];
        // Per string, the column x of its previous segment.
        let mut prev: Vec<Option<i32>> = vec![None; owner.len()];
        for (slot, &g) in sequence.iter().enumerate() {
            let (di, j) = owner[g];
            let seg = seg_of[g];
            seg_of[g] += 1;
            let sx = slot as i32 * seg_pitch;
            // Even segments run bottom -> top, odd ones top -> bottom.
            let enters_top = seg % 2 == 1;

            b.rect(
                poly,
                Rect {
                    x: sx,
                    y: 0,
                    w: body_w,
                    h: total_h,
                },
            );
            // The body: its marker(s) and salicide block, as the recipe says.
            let body = Rect {
                x: sx,
                y: head - lap,
                w: body_w,
                h: seg_l + 2 * lap,
            };
            b.rect(rpoly, body);
            if let Some(l) = process.layer("rpoly_b") {
                b.rect(l, body);
            }
            b.keepout(
                Rect {
                    x: sx,
                    y: head,
                    w: body_w,
                    h: seg_l,
                },
                pnr_core::KeepWhy::ResistorBody { owner: di as u8 },
            );
            // The body: current runs up an even segment, down an odd one. With
            // m parallel strings a segment carries 1/m² of the member's
            // value (∂R/∂R_i, Hastings eqs 8.24-8.25).
            b.unit(pnr_core::Unit {
                owner: di as u8,
                x: sx + body_w / 2,
                y: total_h / 2,
                weight: i64::from(body_w) * i64::from(seg_l) / (per[di] * per[di]) as i64,
                phi: (0, if enters_top { -1 } else { 1 }),
                sa: 0,
                sb: 0,
            });
            for top in [false, true] {
                let y0 = if top { total_h - head_li_h } else { 0 };
                b.rect(
                    li,
                    Rect {
                        x: sx,
                        y: y0,
                        w: body_w,
                        h: head_li_h,
                    },
                );
                for &cy in &cut_ys {
                    let y = if top { total_h - cy - ch } else { cy };
                    b.rect(
                        licon,
                        Rect {
                            x: sx + cut_x,
                            y,
                            w: cw,
                            h: ch,
                        },
                    );
                }
            }
            match prev[g] {
                // One pin pair per string; the router joins repeated names.
                None => b.pin(pin(di, "P", end_cut(sx, enters_top), li)),
                // Jumper from the previous segment's exit head, which is at
                // this segment's entry end: li to an adjacent column, else
                // met1 on the string's own track (over others' heads).
                Some(px) if sx - px == seg_pitch => {
                    let h = border + ch + border;
                    let y = if enters_top { total_h - h } else { 0 };
                    b.rect(
                        li,
                        Rect {
                            x: px,
                            y,
                            w: sx + body_w - px,
                            h,
                        },
                    );
                }
                Some(px) => {
                    let t = tracks[g % tracks.len().max(1)];
                    let y = if enters_top { total_h - t - m_ct } else { t };
                    for cx in [px, sx] {
                        b.rect(
                            mcon,
                            Rect {
                                x: cx + cut_x,
                                y,
                                w: m_ct,
                                h: m_ct,
                            },
                        );
                    }
                    b.rect(
                        met1,
                        Rect {
                            x: px + cut_x - m_enc,
                            y: y - m_enc,
                            w: sx - px + m_ct + 2 * m_enc,
                            h: m_ct + 2 * m_enc,
                        },
                    );
                }
            }
            // Each segment extracts as its own resistor: string j runs
            // P -> Internal(j·n + 1) -> ... -> Internal(j·n + n-1) -> N.
            let node = |k: i32| match k {
                0 => Node::Pin("P"),
                k if k == n_segments => Node::Pin("N"),
                k => Node::Internal((j as i32 * n_segments + k) as u16),
            };
            b.drawn(Drawn {
                owner: di as u8,
                device: None,
                kind: DrawnKind::Resistor,
                nodes: [node(seg), node(seg + 1), Node::Unused],
                w: body_w,
                l: seg_l,
            });
            prev[g] = Some(sx);
            if seg == n_segments - 1 {
                b.pin(pin(di, "N", end_cut(sx, !enters_top), li));
            }
        }

        // End dummies on a matched array: a poly strip one pitch past each end,
        // contacted like a segment but unmarked (no rpoly: interconnect, not a
        // resistor), tied to ground through a `GND` pin, so the end segments
        // see the same etch neighbours as the inner ones (Hastings §8.3 r7).
        let n_cols = sequence.len() as i32;
        let dummies = n_dev > 1
            || crate::builder::unitization(group, constraints).is_some_and(|u| u.dummy_required);
        // Dummies are plain poly: they keep the deck's resistor-body-to-poly
        // spacing (sky130 poly.9), never less than the segment gap.
        let d_pitch = body_w + seg_gap.max(process.space_between("rpoly", "poly").unwrap_or(0));
        // Plain poly takes plain (square) contacts: the precision slot is
        // the resistor body's.
        let sq_x = snap_cut(body_w / 2 - ct / 2, lat);
        let sq_ys: Vec<i32> = (0..)
            .map(|k| border + k * (ct + cut_space))
            .take_while(|&y| y + ct + inner <= head_li_h)
            .collect();
        let (lo, hi) = if dummies {
            for sx in [-d_pitch, (n_cols - 1) * seg_pitch + d_pitch] {
                b.rect(
                    poly,
                    Rect {
                        x: sx,
                        y: 0,
                        w: body_w,
                        h: total_h,
                    },
                );
                for top in [false, true] {
                    let y0 = if top { total_h - head_li_h } else { 0 };
                    b.rect(
                        li,
                        Rect {
                            x: sx,
                            y: y0,
                            w: body_w,
                            h: head_li_h,
                        },
                    );
                    for &cy in &sq_ys {
                        let y = if top { total_h - cy - ct } else { cy };
                        b.rect(
                            licon,
                            Rect {
                                x: sx + sq_x,
                                y,
                                w: ct,
                                h: ct,
                            },
                        );
                    }
                }
                let at = Rect {
                    x: sx + sq_x,
                    y: border,
                    w: ct,
                    h: ct,
                };
                b.pin(pnr_core::Pin {
                    name: "GND".into(),
                    net: pnr_core::NetId(u16::MAX),
                    at,
                    layer: li,
                });
            }
            (-d_pitch, (n_cols - 1) * seg_pitch + d_pitch + body_w)
        } else {
            (0, n_cols * seg_pitch - seg_gap)
        };

        // A precision recipe's region over the array: rpm with npc on it
        // (sky130 xhrpoly), and the recipe's implant reaching `res_keepout`
        // past the poly so a neighbour keeps its distance from the body.
        let array_w = hi - lo;
        let rpm_enc = r("rpm_encloses_poly", 0).max(process.enclosure("rpm", "poly").unwrap_or(0));
        let rpm_w = (array_w + 2 * rpm_enc).max(process.width("rpm").unwrap_or(0));
        let rpm_rect = Rect {
            x: lo + array_w / 2 - rpm_w / 2,
            y: -rpm_enc,
            w: rpm_w,
            h: total_h + 2 * rpm_enc,
        };
        if let Some(rpm) = process.layer("rpm") {
            b.rect(rpm, rpm_rect);
            if let Some(npc) = process.layer("npc") {
                b.rect(npc, rpm_rect);
            }
        }
        if let Some(imp) = process.layer("res_implant") {
            // The implant past the resistor's poly, the deck's.
            let keep = r("res_keepout", 0)
                .max(process.enclosure("res_implant", "rpoly").unwrap_or(0))
                .max(process.enclosure("res_implant", "poly").unwrap_or(0));
            // ...and past the rpm it dopes (sky130 rpm.4).
            let ri = process.enclosure("res_implant", "rpm").unwrap_or(0);
            let (x0, x1) = (
                (rpm_rect.x - ri).min(lo - keep),
                (rpm_rect.x + rpm_w + ri).max(hi + keep),
            );
            let (y0, y1) = (
                (rpm_rect.y - ri).min(-keep),
                (rpm_rect.y + rpm_rect.h + ri).max(total_h + keep),
            );
            b.rect(
                imp,
                Rect {
                    x: x0,
                    y: y0,
                    w: x1 - x0,
                    h: y1 - y0,
                },
            );
        }
        // One salicide block over every body (dummies too: a blocked
        // neighbour is the resistor's own environment), past the poly by the
        // deck's extension, widened to its min area.
        if let Some(blk) = process.layer("res_block") {
            let ext = process.extension("res_block", "poly").unwrap_or(0);
            let (y0, h) = (head - lap, seg_l + 2 * lap);
            let mut x0 = lo - ext;
            let mut w = hi - lo + 2 * ext;
            let need =
                i32::try_from(process.area("res_block").unwrap_or(0) / i64::from(h.max(1)) + 1)
                    .unwrap_or(0);
            if w < need {
                x0 -= (need - w) / 2;
                w = need;
            }
            b.rect(blk, Rect { x: x0, y: y0, w, h });
        }
        // Head contacts on poly outside a precision region still sit in npc.
        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// Heights (from a head's outer end) of the met1 jumper tracks: clear of the
/// pin's cut row by a met1 pad (the router lands one on the pin) and met1
/// spacing, a met1 pitch apart, inside the head.
fn jumper_tracks(process: &dyn Process) -> Vec<i32> {
    let r = |name: &str, default: i32| process.rule(name, default);
    let lat = cut_lattice(process);
    let (ct, m_ct, m_enc) = (
        dim(process, "contact"),
        dim(process, "mcon_size"),
        dim(process, "m1_enc"),
    );
    let border = r("poly_encloses_licon_one_side", 0)
        .max(r("li_encloses_licon", 0))
        .max(process.enclosure("poly", "licon").unwrap_or(0))
        .max(process.endcap("poly", "licon").unwrap_or(0))
        .max(process.endcap("li", "licon").unwrap_or(0));
    let head_li_h = head_len(process) - LAP;
    let pitch = m_ct + 2 * m_enc + dim(process, "met1_space");
    let first = snap_cut(
        border + ct + (m_ct + 2 * m_enc) + dim(process, "met1_space") + lat - 1,
        lat,
    );
    (0..)
        .map(|k| first + k * snap_cut(pitch + lat - 1, lat))
        .take_while(|&y| y + m_ct + m_enc + border <= head_li_h)
        .collect()
}

/// Column gap: the deck's segment gap, and never under the li spacing its
/// full-width heads need (wide-metal spacing included).
fn seg_gap(process: &dyn Process) -> i32 {
    let lat = cut_lattice(process);
    snap_cut(
        process
            .rule("res_seg_gap", 0)
            .max(process.space("li").unwrap_or(0))
            + lat
            - 1,
        lat,
    )
}

/// Head contact `(w, h)`: the sidecar's precision-resistor slot
/// (`res_contact_w`/`res_contact_h`), else a square `contact`.
fn slot(process: &dyn Process) -> (i32, i32) {
    let ct = dim(process, "contact");
    match (
        process.rule("res_contact_w", 0),
        process.rule("res_contact_h", 0),
    ) {
        (w, h) if w > 0 && h > 0 => (w, h),
        _ => (ct, ct),
    }
}

/// How far the resistor marker reaches into each head: none, so the body
/// proper is exactly the segment length; the heads' cuts keep the deck's
/// marker spacing on their own (`inner`).
const LAP: i32 = 0;

/// Contacted head length: the deck's `res_head` (sky130's `xpc` must extend
/// the body by >= 2.16 um), and never less than one bordered cut past
/// [`LAP`], or the head carries no contact and the resistor no terminal.
fn head_len(process: &dyn Process) -> i32 {
    let r = |name: &str, default: i32| process.rule(name, default);
    let border = r("poly_encloses_licon_one_side", 0)
        .max(r("li_encloses_licon", 0))
        .max(process.enclosure("poly", "licon").unwrap_or(0))
        .max(process.endcap("poly", "licon").unwrap_or(0))
        .max(process.endcap("li", "licon").unwrap_or(0));
    let inner = border
        .max(process.space_between("rpoly", "licon").unwrap_or(0))
        .max(process.space_between("rpoly_b", "licon").unwrap_or(0))
        .max(process.space_between("res_block", "licon").unwrap_or(0));
    let lat = cut_lattice(process);
    snap_cut(
        r("res_head", 0).max(LAP + border + slot(process).1 + inner) + lat - 1,
        lat,
    )
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default body: min segment width and a nominal 10µm body (`res_min_segment`).
    let def_w = process.rule("res_min_width", 0);
    let def_l = process.rule("res_min_segment", 0);
    sizing(group, c, def_w, def_l)
}

/// String index per segment column: `Interdig` interleaves strings (greedy
/// centroid), anything else keeps each string's segments adjacent.
fn res_segment_sequence(n_dev: usize, pattern: Pattern, n_segments: i32) -> Vec<usize> {
    let per_dev = n_segments as usize;
    if pattern == Pattern::Interdig && n_dev >= 2 {
        return greedy_centroid(&vec![per_dev; n_dev]);
    }
    (0..n_dev)
        .flat_map(|di| std::iter::repeat_n(di, per_dev))
        .collect()
}

/// Segment counts (1 or even) whose value-preserving length
/// ([`ResModel::seg_len`]) is at least `res_min_segment` and within
/// `res_value_tol_ppm` once snapped, the 8 squarest. Without a model only 1:
/// splitting adds head terms the generator cannot account for.
fn feasible_segments(s: &Sizing, process: &dyn Process) -> Vec<u16> {
    let tol = value_tol_ppm(process);
    let min_seg = process.rule("res_min_segment", 0).max(1);
    let Some(model) = ResModel::of(process) else {
        return vec![1];
    };
    let lat = cut_lattice(process);
    let target = model.ohm(s.unit_w, s.unit_l);
    let body_w = s.unit_w.max(process.rule("res_min_width", 0));
    let seg_gap = seg_gap(process);
    let head = head_len(process);
    // n = 1 never changes the value: kept even below the minimum segment.
    let mut opts: Vec<(i32, i32)> = (1..=64u32)
        .filter(|&n| n == 1 || n % 2 == 0)
        .filter_map(|n| {
            Some((
                n as i32,
                model
                    .seg_len(body_w, target, n, min_seg, lat, tol)
                    .or((n == 1).then_some(s.unit_l))?,
            ))
        })
        .collect();
    opts.sort_by(|&(a, la), &(b, lb)| {
        let quality = |n: i32, l: i32| {
            (f64::from(n * (body_w + seg_gap)) / f64::from(2 * head + l))
                .ln()
                .abs()
        };
        quality(a, la).total_cmp(&quality(b, lb))
    });
    opts.truncate(8);
    let mut out: Vec<u16> = opts.into_iter().map(|(n, _)| n as u16).collect();
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, drawn alone, is DRC- and ERC-clean.
    #[test]
    fn every_variant_is_drc_and_erc_clean() {
        use crate::testkit;
        use pnr_core::DeviceKind;
        let Some(pdk) = testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let mut dirty = Vec::new();
        for n in [1, 2] {
            dirty.extend(testkit::dirty::<Resistor>(
                DeviceKind::Resistor,
                n,
                1,
                500,
                10_000,
                &pdk,
            ));
            // Long enough for several segments: interleaved met1 jumpers.
            dirty.extend(testkit::dirty::<Resistor>(
                DeviceKind::Resistor,
                n,
                1,
                500,
                40_000,
                &pdk,
            ));
        }
        dirty.extend(testkit::dirty::<Resistor>(
            DeviceKind::Resistor,
            3,
            1,
            500,
            40_000,
            &pdk,
        ));
        // Through the high_po recipe (a model: segmented variants), one and
        // two members, one and two parallel strings each.
        let op = high_po(&pdk);
        for (n, nf) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
            for (i, m) in variants(n, nf, &op) {
                let rules = testkit::findings(&m.shapes, &testkit::ports_with(&m, &[]), &pdk);
                if !rules.is_empty() {
                    dirty.push(format!("high_po n={n} nf={nf} #{i}: {rules:?}"));
                }
            }
        }
        assert!(
            dirty.is_empty(),
            "DRC/ERC-dirty variants:\n{}",
            dirty.join("\n")
        );
    }

    /// sky130 `res_high_po` W = 0.69, L = 40 µm, from the model card (derived).
    const R_690_40: f64 = 18_949.2;

    fn high_po(pdk: &verify::Pdk) -> verify::pdk::Overlay<'_> {
        verify::pdk::Overlay {
            pdk,
            recipe: pdk
                .recipe("resistor", "res_high_po")
                .expect("high_po recipe"),
        }
    }

    /// Every variant of `n` members of W = 690, L = 40 µm at `nf` parallel
    /// strings each, drawn.
    fn variants(n: usize, nf: u16, p: &dyn Process) -> Vec<(usize, Macro)> {
        let (group, c) =
            crate::testkit::group_of(pnr_core::DeviceKind::Resistor, n, nf, 690, 40_000);
        let vs = Resistor::enumerate(&group, &c, p);
        assert!(!vs.is_empty(), "no variants");
        vs.iter()
            .map(|v| (usize::from(v.segments), v.draw(&group, &c, p)))
            .collect()
    }

    /// Model value per string (owner, string), Σ over its segments: string
    /// j's internal nodes are j·n + 1 ..= j·n + n − 1; with n = 1 every card
    /// is its own string.
    fn string_ohms(
        m: &Macro,
        n: usize,
        model: &ResModel,
    ) -> std::collections::BTreeMap<(u8, usize), f64> {
        let mut out = std::collections::BTreeMap::new();
        for (i, d) in m.drawn.iter().enumerate() {
            let j = d.nodes.iter().find_map(|x| match x {
                Node::Internal(k) => Some(usize::from(*k) / n),
                _ => None,
            });
            *out.entry((
                d.owner,
                if n == 1 {
                    i
                } else {
                    j.expect("a segmented string's card has an internal node")
                },
            ))
            .or_insert(0.0) += model.ohm(d.w, d.l);
        }
        out
    }

    fn within(r: f64) -> bool {
        (r - R_690_40).abs() / R_690_40 <= 0.005
    }

    #[test]
    fn segments_preserve_the_model_value() {
        let Some(pdk) = crate::testkit::pdk() else {
            return;
        };
        let op = high_po(&pdk);
        let model = ResModel::of(&op).expect("high_po states a model");
        assert!(
            (model.ohm(690, 40_000) - R_690_40).abs() < 0.5,
            "{}",
            model.ohm(690, 40_000)
        );
        let vs = variants(1, 1, &op);
        assert!(
            vs.iter().any(|(n, _)| *n == 2),
            "a 2-segment variant at L = 40 um"
        );
        for (n, m) in vs {
            let ohms = string_ohms(&m, n, &model);
            assert_eq!(ohms.len(), 1, "n={n}");
            assert!(ohms.values().all(|&r| within(r)), "n={n}: {ohms:?}");
        }
    }

    #[test]
    fn short_segments_are_not_offered() {
        let Some(pdk) = crate::testkit::pdk() else {
            return;
        };
        // n = 4 needs 9.148 um segments, under res_min_segment 10 um.
        assert!(variants(1, 1, &high_po(&pdk)).iter().all(|(n, _)| *n != 4));
    }

    #[test]
    fn a_multiplied_resistor_draws_parallel_strings() {
        let Some(pdk) = crate::testkit::pdk() else {
            return;
        };
        let op = high_po(&pdk);
        let model = ResModel::of(&op).expect("model");
        for (n, m) in variants(1, 2, &op) {
            assert_eq!(m.drawn.len(), 2 * n, "n={n}");
            for t in ["d0:P", "d0:N"] {
                assert_eq!(
                    m.pins.iter().filter(|p| p.name == t).count(),
                    2,
                    "n={n} {t}"
                );
            }
            let ohms = string_ohms(&m, n, &model);
            assert_eq!(ohms.len(), 2, "n={n}: {ohms:?}");
            assert!(ohms.values().all(|&r| within(r)), "n={n}: {ohms:?}");
        }
    }

    #[test]
    fn every_variant_extracts_one_resistor_per_segment() {
        let Some(pdk) = crate::testkit::pdk() else {
            return;
        };
        let op = high_po(&pdk);
        for nf in [1, 2] {
            for (n, m) in variants(1, nf, &op) {
                let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic)
                    .expect("extracts");
                let rs = spice.lines().filter(|l| l.starts_with('R')).count();
                assert_eq!(rs, m.drawn.len(), "nf={nf} n={n}:\n{spice}");
            }
        }
    }
}
