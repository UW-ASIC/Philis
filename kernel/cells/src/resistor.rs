//! Resistor generator: each device a series chain of `segments` sky130
//! precision poly resistors — a continuous poly strip whose body is marked by
//! `rpoly`, with long contacted heads, all under rpm + npc + psdm.

use crate::builder::dim;
use analog::cell::SeriesParallel;
use analog::matching::{class::resistor_env, pattern::segment_row};
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, Macro, MatchClass, Node, Process, Rect};

use crate::builder::{cut_lattice, pin, req, sizing, snap_cut, unitization, Builder, Sizing};
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
        w + f64::from(self.dw_nm) / 1e3 - f64::from(self.narrow_permille) / 1e3 * (f64::from(self.knee_nm) / 1e3 - w).max(0.0)
    }

    /// Both heads of one device, Ω.
    fn head_ohm(&self, weff: f64) -> f64 {
        self.head_mohm_um as f64 / 1e3 / (weff + f64::from(self.head_dw_nm) / 1e3)
    }

    /// One device of drawn W×L, Ω.
    #[must_use]
    pub fn ohm(&self, w_nm: i32, l_nm: i32) -> f64 {
        let weff = self.weff(w_nm);
        self.sheet_mohm as f64 / 1e3 * (f64::from(l_nm) + f64::from(self.dl_nm)) / 1e3 / weff + self.head_ohm(weff)
    }

    /// Body length so `n` series devices of width `w` total `target`:
    /// L = (target/n − head/(weff+head_dw))·weff/sheet − dl, snapped to `lat`;
    /// `None` below `min_nm` or when the snapped residual exceeds `tol_ppm`.
    #[must_use]
    pub fn seg_len(&self, w_nm: i32, target_ohm: f64, n: u32, min_nm: i32, lat: i32, tol_ppm: i32) -> Option<i32> {
        let weff = self.weff(w_nm);
        let l_um = (target_ohm / f64::from(n.max(1)) - self.head_ohm(weff)) * weff / (self.sheet_mohm as f64 / 1e3) - f64::from(self.dl_nm) / 1e3;
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

/// κ_ox, W/(m·K) (Hastings's 0.011 W/cm/°C).
const K_OX: f64 = 1.1;

/// Hastings eq. 5.8 (L12044–12073): W_min = I·√(t_ox·R_s/(κ_ox·ΔT)), nm, rounded up; 0 when any input ≤ 0.
/// `t_ox_nm` = the oxide under the body (sidecar `res_tox_nm`, else the body layer's pex `height_nm`);
/// `dt_k` = the allowed rise (`res_self_heat_dt_k`, 5 K). CELL-23 applies it.
#[must_use]
pub fn self_heating_min_width_nm(i_ua: f32, sheet_ohm: f32, t_ox_nm: f32, dt_k: f32) -> i32 {
    if i_ua <= 0.0 || sheet_ohm <= 0.0 || t_ox_nm <= 0.0 || dt_k <= 0.0 {
        return 0;
    }
    let w = f64::from(i_ua) * 1e-6 * (f64::from(t_ox_nm) * 1e-9 * f64::from(sheet_ohm) / (K_OX * f64::from(dt_k))).sqrt();
    (w * 1e9).ceil() as i32
}

/// Eq. 5.8 inverted, the reported figure: ΔT = t_ox·R_s·(I/W)²/κ_ox, K; 0 when any input ≤ 0.
#[must_use]
pub fn self_heating_rise_k(i_ua: f32, sheet_ohm: f32, t_ox_nm: f32, w_nm: i32) -> f32 {
    if i_ua <= 0.0 || sheet_ohm <= 0.0 || t_ox_nm <= 0.0 || w_nm <= 0 {
        return 0.0;
    }
    let j = f64::from(i_ua) * 1e-6 / (f64::from(w_nm) * 1e-9);
    (f64::from(t_ox_nm) * 1e-9 * f64::from(sheet_ohm) * j * j / K_OX) as f32
}

/// Parallel strings per member: `dev_nf[d]` when the unitization composes
/// units in parallel (SPICE `m`), else one.
fn strings(group: &DeviceGroup, c: &Constraints, s: &Sizing) -> Vec<usize> {
    let parallel = unitization(group, c).is_some_and(|u| u.series_parallel == SeriesParallel::Parallel);
    s.dev_nf.iter().map(|&nf| if parallel { usize::from(nf.max(1)) } else { 1 }).collect()
}

/// Per member of `group`, its series unit count (`series_d = L_d / L_u`, EXT-15) when the covering unitization
/// composes `Series` units with a non-empty `series` over ≥ 2 members; `None` for an equal-length group.
fn series_counts(group: &DeviceGroup, c: &Constraints) -> Option<Vec<u16>> {
    let u = unitization(group, c).filter(|u| u.series_parallel == SeriesParallel::Series && !u.series.is_empty())?;
    (group.devices.len() >= 2).then(|| {
        group
            .devices
            .iter()
            .map(|d| u.devices.iter().position(|x| x == d).and_then(|i| u.series.get(i)).copied().unwrap_or(1).max(1))
            .collect()
    })
}

/// The unit length L_u′ that makes the smallest member exact, `seg_len(body_w, ohm(unit_w, ser_min·unit_l)/ser_min, 1,
/// 0, lat, i32::MAX)`; `Some` only when every member passes |ser_d·ohm(body_w, L_u′) − ohm(unit_w, ser_d·unit_l)| ≤
/// res_value_tol_ppm·ohm(…)/1e6 (plan-03 value rule: heads and `dl` make a series chain of units differ from one
/// long body). `None` without a `ResModel`.
fn unit_len(s: &Sizing, series: &[u16], process: &dyn Process) -> Option<i32> {
    let m = ResModel::of(process)?;
    let body_w = s.unit_w.max(process.rule("res_min_width", 0));
    let k_min = f64::from(*series.iter().min()?);
    let target = m.ohm(s.unit_w, (k_min as i32).saturating_mul(s.unit_l)) / k_min;
    let lu = m.seg_len(body_w, target, 1, 0, cut_lattice(process), i32::MAX)?;
    let tol = f64::from(value_tol_ppm(process));
    series
        .iter()
        .all(|&k| {
            let want = m.ohm(s.unit_w, i32::from(k).saturating_mul(s.unit_l));
            (f64::from(k) * m.ohm(body_w, lu) - want).abs() <= tol * want / 1e6
        })
        .then_some(lu)
}

/// Whether some string's consecutive segments sit more than one column apart (a met1 jumper).
fn far_join(seq: &[usize]) -> bool {
    seq.iter().enumerate().any(|(i, &g)| seq[i + 1..].iter().position(|&h| h == g).is_some_and(|k| k > 0))
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
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        // No resistor marker, no poly resistor on this process.
        if group.devices.is_empty() || process.layer("rpoly").is_none() {
            return vec![];
        }
        let s = group_sizing(group, constraints, process);
        let tracks = jumper_tracks(process, head_len(process)).len();
        // Unequal lengths (EXT-15): exact per-member blocks always; a common-centroid array of series units, one met1
        // track per member (heads grown to fit, [`head_for`]), where one unit length keeps every member's value.
        if let Some(ser) = series_counts(group, constraints) {
            let mut out = vec![Resistor { segments: 1, pattern: Pattern::Single }];
            if unit_len(&s, &ser, process).is_some() {
                out.push(Resistor { segments: 1, pattern: Pattern::Interdig });
            }
            return out;
        }
        let patterns = if group.devices.len() > 1 {
            vec![Pattern::Single, Pattern::Interdig]
        } else {
            vec![Pattern::Single]
        };
        // A sequence with a non-adjacent join puts every join on met1, one
        // track per string inside the heads: as many strings as tracks fit.
        let n_strings: usize = strings(group, constraints, &s).iter().sum();
        feasible_segments(s.unit_w, s.unit_l, process)
            .into_iter()
            .flat_map(|segments| {
                patterns.iter().map(move |&pattern| Resistor { segments, pattern })
            })
            .filter(|r| !(n_strings > tracks && far_join(&res_segment_sequence(&vec![r.segments; n_strings], r.pattern))))
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
        // Per member, (segments, segment length). Each segment's length keeps
        // the string at the schematic device's model value (heads included);
        // without a model only n = 1 is offered, which draws the written L.
        let model = ResModel::of(process);
        let fit = |n: i32, l_nm: i32| {
            model
                .and_then(|m| m.seg_len(body_w, m.ohm(s.unit_w, l_nm), n as u32, 0, lat, i32::MAX))
                .unwrap_or(l_nm / n)
                .max(process.width("rpoly").unwrap_or(0))
        };
        let cols: Vec<(i32, i32)> = match series_counts(group, constraints) {
            None => vec![(n_segments, fit(n_segments, s.unit_l)); n_dev],
            // Series units: member d is ser_d columns of the common unit.
            Some(ser) if self.pattern == Pattern::Interdig => {
                let lu = unit_len(&s, &ser, process).expect("Interdig is offered only with an exact unit").max(process.width("rpoly").unwrap_or(0));
                ser.iter().map(|&k| (i32::from(k), lu)).collect()
            }
            // Blocks: member d at the most segments (≤ ser_d) that keep its value.
            Some(ser) => ser
                .iter()
                .map(|&k| {
                    let l = i32::from(k).saturating_mul(s.unit_l);
                    let n = i32::from(feasible_segments(s.unit_w, l, process).into_iter().filter(|&n| n <= k).max().unwrap_or(1));
                    (n, fit(n, l))
                })
                .collect(),
        };
        // String g is string j of member di: one pseudo-device for sequencing.
        let per = strings(group, constraints, &s);
        let owner: Vec<(usize, usize)> = per.iter().enumerate().flat_map(|(d, &k)| (0..k).map(move |j| (d, j))).collect();
        let counts: Vec<u16> = owner.iter().map(|&(d, _)| cols[d].0 as u16).collect();
        let sequence = res_segment_sequence(&counts, self.pattern);
        // One non-adjacent join puts every join on met1 + 2 mcon, so every
        // string sees the same contact resistance per join.
        let all_met1 = far_join(&sequence);
        // Heads long enough for one met1 track per string where any join is.
        let head = if all_met1 { head_for(process, owner.len()) } else { head_len(process) };
        let tracks = jumper_tracks(process, head);
        let seg_pitch = body_w + seg_gap;
        let col_h = |di: usize| head + cols[di].1 + head;
        let max_h = (0..n_dev).map(col_h).max().unwrap_or(2 * head);
        let head_li_h = head - lap;

        // Contact column of a head, outer end first (the top head mirrors it).
        let cut_x = snap_cut(body_w / 2 - cw / 2, lat);
        let cut_ys: Vec<i32> = (0..)
            .map(|k| border + k * (ch + cut_space))
            .take_while(|&y| y + ch + inner <= head_li_h)
            .collect();
        // The outermost cut of a head: where the pins land.
        let end_cut = |sx: i32, top: bool, total_h: i32| Rect {
            x: sx + cut_x,
            y: if top { total_h - border - ch } else { border },
            w: cw,
            h: ch,
        };

        let mut seg_of = vec![0i32; owner.len()];
        // Per string, the column x of its previous segment.
        let mut prev: Vec<Option<i32>> = vec![None; owner.len()];
        for (slot, &g) in sequence.iter().enumerate() {
            let (di, j) = owner[g];
            let (n_d, seg_l) = cols[di];
            let total_h = col_h(di);
            let seg = seg_of[g];
            seg_of[g] += 1;
            let sx = slot as i32 * seg_pitch;
            // Even segments run bottom -> top, odd ones top -> bottom.
            let enters_top = seg % 2 == 1;

            b.rect(poly, Rect { x: sx, y: 0, w: body_w, h: total_h });
            // The body: its marker(s) and salicide block, as the recipe says.
            let body = Rect { x: sx, y: head - lap, w: body_w, h: seg_l + 2 * lap };
            b.rect(rpoly, body);
            if let Some(l) = process.layer("rpoly_b") {
                b.rect(l, body);
            }
            b.keepout(Rect { x: sx, y: head, w: body_w, h: seg_l }, pnr_core::KeepWhy::ResistorBody { owner: di as u8 });
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
                b.rect(li, Rect { x: sx, y: y0, w: body_w, h: head_li_h });
                for &cy in &cut_ys {
                    let y = if top { total_h - cy - ch } else { cy };
                    b.rect(licon, Rect { x: sx + cut_x, y, w: cw, h: ch });
                }
            }
            match prev[g] {
                // One pin pair per string; the router joins repeated names.
                None => b.pin(pin(di, "P", end_cut(sx, enters_top, total_h), li)),
                // Jumper from the previous segment's exit head, which is at
                // this segment's entry end: li to an adjacent column, else
                // met1 on the string's own track (over others' heads).
                Some(px) if !all_met1 && sx - px == seg_pitch => {
                    let h = border + ch + border;
                    let y = if enters_top { total_h - h } else { 0 };
                    b.rect(li, Rect { x: px, y, w: sx + body_w - px, h });
                }
                Some(px) => {
                    let t = tracks[g % tracks.len().max(1)];
                    let y = if enters_top { total_h - t - m_ct } else { t };
                    for cx in [px, sx] {
                        b.rect(mcon, Rect { x: cx + cut_x, y, w: m_ct, h: m_ct });
                    }
                    b.rect(met1, Rect { x: px + cut_x - m_enc, y: y - m_enc, w: sx - px + m_ct + 2 * m_enc, h: m_ct + 2 * m_enc });
                }
            }
            // Each segment extracts as its own resistor: string j runs
            // P -> Internal(j·n + 1) -> ... -> Internal(j·n + n-1) -> N.
            let node = |k: i32| match k {
                0 => Node::Pin("P"),
                k if k == n_d => Node::Pin("N"),
                k => Node::Internal((j as i32 * n_d + k) as u16),
            };
            b.drawn(Drawn { owner: di as u8, device: None, kind: DrawnKind::Resistor, nodes: [node(seg), node(seg + 1), Node::Unused], w: body_w, l: seg_l });
            prev[g] = Some(sx);
            if seg == n_d - 1 {
                b.pin(pin(di, "N", end_cut(sx, !enters_top, total_h), li));
            }
        }

        // End dummies on a matched array: poly strips at the segment pitch
        // past each end, contacted like a segment but unmarked (no rpoly:
        // interconnect, not a resistor), tied to ground through a `GND` pin,
        // so the end segments see the same etch neighbours as the inner ones
        // (Hastings §8.3 r7). Per end, the class's count (H08-13): one, and
        // on Exceptional enough to span `dummy_span_nm`.
        let n_cols = sequence.len() as i32;
        let u = crate::builder::unitization(group, constraints);
        let dummies = n_dev > 1 || u.is_some_and(|u| u.dummy_required);
        let class = u.and_then(|u| u.class).unwrap_or(MatchClass::Moderate);
        let env = resistor_env(class, process);
        let k_dum = i32::from(env.min_dummies).max((env.dummy_span_nm + seg_pitch - 1) / seg_pitch);
        // Minimal takes a minimum-width dummy; its inner edge stays a gap from
        // the end body.
        let dw = if class == MatchClass::Minimal { r("res_min_width", 0).max(process.width("poly").unwrap_or(0)) } else { body_w };
        // Plain poly takes plain (square) contacts: the precision slot is
        // the resistor body's.
        let sq_x = snap_cut(dw / 2 - ct / 2, lat);
        let sq_ys: Vec<i32> = (0..).map(|k| border + k * (ct + cut_space)).take_while(|&y| y + ct + inner <= head_li_h).collect();
        // Every poly column left to right as (x, w, seg_l), dummies included.
        let end_l = |slot: usize| sequence.get(slot).map_or(0, |&g| cols[owner[g].0].1);
        let mut columns: Vec<(i32, i32, i32)> = (0..n_cols).map(|i| (i * seg_pitch, body_w, end_l(i as usize))).collect();
        if dummies {
            let (l_lo, l_hi) = (end_l(0), end_l(sequence.len().saturating_sub(1)));
            for i in 1..=k_dum {
                columns.insert(0, (-(i - 1) * seg_pitch - seg_gap - dw, dw, l_lo));
                columns.push(((n_cols - 1 + i) * seg_pitch, dw, l_hi));
            }
            for &(sx, _, l) in columns.iter().filter(|c| c.0 < 0 || c.0 >= n_cols * seg_pitch) {
                let total_h = head + l + head;
                b.rect(poly, Rect { x: sx, y: 0, w: dw, h: total_h });
                for top in [false, true] {
                    let y0 = if top { total_h - head_li_h } else { 0 };
                    b.rect(li, Rect { x: sx, y: y0, w: dw, h: head_li_h });
                    for &cy in &sq_ys {
                        let y = if top { total_h - cy - ct } else { cy };
                        b.rect(licon, Rect { x: sx + sq_x, y, w: ct, h: ct });
                    }
                }
                let at = Rect { x: sx + sq_x, y: border, w: ct, h: ct };
                b.pin(pnr_core::Pin { name: "GND".into(), net: pnr_core::NetId(u16::MAX), at, layer: li });
            }
        }
        let (lo, hi) = (columns[0].0, columns.last().map_or(0, |c| c.0 + c.1));
        let total_h = max_h;

        // A precision recipe's region over the array: rpm with npc on it
        // (sky130 xhrpoly), and the recipe's implant reaching `res_keepout`
        // past the poly so a neighbour keeps its distance from the body.
        let array_w = hi - lo;
        let rpm_enc = r("rpm_encloses_poly", 0).max(process.enclosure("rpm", "poly").unwrap_or(0));
        let rpm_w = (array_w + 2 * rpm_enc).max(process.width("rpm").unwrap_or(0));
        let rpm_rect = Rect { x: lo + array_w / 2 - rpm_w / 2, y: -rpm_enc, w: rpm_w, h: total_h + 2 * rpm_enc };
        if let Some(rpm) = process.layer("rpm") {
            b.rect(rpm, rpm_rect);
            if let Some(npc) = process.layer("npc") {
                b.rect(npc, rpm_rect);
            }
        }
        if let Some(imp) = process.layer("res_implant") {
            // The implant past the resistor's poly, the deck's.
            let keep = r("res_keepout", 0).max(process.enclosure("res_implant", "rpoly").unwrap_or(0)).max(process.enclosure("res_implant", "poly").unwrap_or(0));
            // ...and past the rpm it dopes (sky130 rpm.4).
            let ri = process.enclosure("res_implant", "rpm").unwrap_or(0);
            let (x0, x1) = ((rpm_rect.x - ri).min(lo - keep), (rpm_rect.x + rpm_w + ri).max(hi + keep));
            let (y0, y1) = ((rpm_rect.y - ri).min(-keep), (rpm_rect.y + rpm_rect.h + ri).max(total_h + keep));
            b.rect(imp, Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 });
        }
        // A salicide block over every body (dummies too: a blocked neighbour
        // is the resistor's own environment), past the poly by the deck's
        // extension: one rect per run of equal body length, meeting its
        // neighbour mid-gap (one stepped union); one run is widened to its
        // min area.
        if let Some(blk) = process.layer("res_block") {
            let ext = process.extension("res_block", "poly").unwrap_or(0);
            let mid = |i: usize| crate::builder::snap_to_grid((columns[i].0 + columns[i].1 + columns[i + 1].0) / 2, process.grid());
            let mut runs: Vec<(i32, i32, i32)> = Vec::new();
            for (i, &(_, _, l)) in columns.iter().enumerate() {
                let x1 = if i + 1 < columns.len() { mid(i) } else { hi + ext };
                match runs.last_mut() {
                    Some(run) if run.2 == l => run.1 = x1,
                    _ => runs.push((runs.last().map_or(lo - ext, |r| r.1), x1, l)),
                }
            }
            if let [(x0, x1, l)] = runs[..] {
                let h = l + 2 * lap;
                let need = i32::try_from(process.area("res_block").unwrap_or(0) / i64::from(h.max(1)) + 1).unwrap_or(0);
                let w = (x1 - x0).max(need);
                b.rect(blk, Rect { x: x0 - (w - (x1 - x0)) / 2, y: head - lap, w, h });
            } else {
                for (x0, x1, l) in runs {
                    b.rect(blk, Rect { x: x0, y: head - lap, w: x1 - x0, h: l + 2 * lap });
                }
            }
        }
        // Head contacts on poly outside a precision region still sit in npc.
        b.cover_poly_cuts(process);
        b.finish()
    }
}

/// Heights (from a head's outer end) of the met1 jumper tracks: clear of the
/// pin's cut row by a met1 pad (the router lands one on the pin) and met1
/// spacing, a met1 pitch apart, inside a head of length `head`.
fn jumper_tracks(process: &dyn Process, head: i32) -> Vec<i32> {
    let r = |name: &str, default: i32| process.rule(name, default);
    let lat = cut_lattice(process);
    let (ct, m_ct, m_enc) = (dim(process, "contact"), dim(process, "mcon_size"), dim(process, "m1_enc"));
    let border = r("poly_encloses_licon_one_side", 0)
            .max(r("li_encloses_licon", 0))
            .max(process.enclosure("poly", "licon").unwrap_or(0))
            .max(process.endcap("poly", "licon").unwrap_or(0))
            .max(process.endcap("li", "licon").unwrap_or(0));
    let head_li_h = head - LAP;
    let pitch = m_ct + 2 * m_enc + dim(process, "met1_space");
    let first = snap_cut(border + ct + (m_ct + 2 * m_enc) + dim(process, "met1_space") + lat - 1, lat);
    (0..).map(|k| first + k * snap_cut(pitch + lat - 1, lat)).take_while(|&y| y + m_ct + m_enc + border <= head_li_h).collect()
}

/// [`head_len`], grown a cut lattice step at a time until `n` jumper tracks fit.
// ponytail: linear search, a few hundred steps at most; closed form if it ever shows up in a profile.
fn head_for(process: &dyn Process, n: usize) -> i32 {
    let mut head = head_len(process);
    while jumper_tracks(process, head).len() < n {
        head += cut_lattice(process).max(1);
    }
    head
}

/// Column gap: the deck's segment gap, never under the li spacing its
/// full-width heads need (wide-metal spacing included), nor the body-to-poly
/// spacing a plain-poly dummy needs (sky130 poly.9), so dummies sit at the
/// array pitch.
fn seg_gap(process: &dyn Process) -> i32 {
    let lat = cut_lattice(process);
    let gap = process.rule("res_seg_gap", 0).max(process.space("li").unwrap_or(0)).max(process.space_between("rpoly", "poly").unwrap_or(0));
    snap_cut(gap + lat - 1, lat)
}

/// Head contact `(w, h)`: the sidecar's precision-resistor slot
/// (`res_contact_w`/`res_contact_h`), else a square `contact`.
fn slot(process: &dyn Process) -> (i32, i32) {
    let ct = dim(process, "contact");
    match (process.rule("res_contact_w", 0), process.rule("res_contact_h", 0)) {
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
    snap_cut(r("res_head", 0).max(LAP + border + slot(process).1 + inner) + lat - 1, lat)
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    // Default body: min segment width and a nominal 10µm body (`res_min_segment`).
    let def_w = process.rule("res_min_width", 0);
    let def_l = process.rule("res_min_segment", 0);
    sizing(group, c, def_w, def_l)
}

/// String index per segment column from per-string segment counts:
/// `Interdig` is the common-centroid row ([`segment_row`]), anything else
/// keeps each string's segments adjacent.
fn res_segment_sequence(counts: &[u16], pattern: Pattern) -> Vec<usize> {
    if pattern == Pattern::Interdig && counts.len() >= 2 {
        return segment_row(counts).0;
    }
    counts.iter().enumerate().flat_map(|(g, &n)| std::iter::repeat_n(g, usize::from(n))).collect()
}

/// Segment counts (1 or even) whose value-preserving length
/// ([`ResModel::seg_len`]) is at least `res_min_segment` and within
/// `res_value_tol_ppm` once snapped, the 8 squarest. Without a model only 1:
/// splitting adds head terms the generator cannot account for.
fn feasible_segments(w_nm: i32, l_nm: i32, process: &dyn Process) -> Vec<u16> {
    let tol = value_tol_ppm(process);
    let min_seg = process.rule("res_min_segment", 0).max(1);
    let Some(model) = ResModel::of(process) else { return vec![1] };
    let lat = cut_lattice(process);
    let target = model.ohm(w_nm, l_nm);
    let body_w = w_nm.max(process.rule("res_min_width", 0));
    let seg_gap = seg_gap(process);
    let head = head_len(process);
    // n = 1 never changes the value: kept even below the minimum segment.
    let mut opts: Vec<(i32, i32)> = (1..=64u32)
        .filter(|&n| n == 1 || n % 2 == 0)
        .filter_map(|n| Some((n as i32, model.seg_len(body_w, target, n, min_seg, lat, tol).or((n == 1).then_some(l_nm))?)))
        .collect();
    opts.sort_by(|&(a, la), &(b, lb)| {
        let quality = |n: i32, l: i32| (f64::from(n * (body_w + seg_gap)) / f64::from(2 * head + l)).ln().abs();
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

    /// 2 kΩ/□ poly over 326.2 nm of oxide (sky130 `rbody_po`), 5 K.
    #[test]
    fn hastings_eq_5_8() {
        assert!((self_heating_min_width_nm(100.0, 2000.0, 326.2, 5.0) - 1089).abs() <= 5);
        assert!((self_heating_min_width_nm(1000.0, 2000.0, 326.2, 5.0) - 10_891).abs() <= 5);
        for (i, r, t, dt) in [(0.0, 2000.0, 326.2, 5.0), (100.0, -1.0, 326.2, 5.0), (100.0, 2000.0, 0.0, 5.0), (100.0, 2000.0, 326.2, 0.0)] {
            assert_eq!(self_heating_min_width_nm(i, r, t, dt), 0);
        }
        assert_eq!(self_heating_rise_k(100.0, 2000.0, 326.2, 0), 0.0);
        assert_eq!(self_heating_rise_k(-1.0, 2000.0, 326.2, 1089), 0.0);
        let dt = self_heating_rise_k(100.0, 2000.0, 326.2, 1089);
        assert!((dt - 5.0).abs() < 0.02, "{dt}");
    }

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
            dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, n, 1, 500, 10_000, &pdk));
            // Long enough for several segments: interleaved met1 jumpers.
            dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, n, 1, 500, 40_000, &pdk));
        }
        dirty.extend(testkit::dirty::<Resistor>(DeviceKind::Resistor, 3, 1, 500, 40_000, &pdk));
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
        // Unequal members (EXT-15): series-unit arrays and exact blocks.
        let gp = generic_po(&pdk);
        for (name, p) in [("generic_po", &gp as &dyn Process), ("high_po", &op)] {
            for ser in [[1, 4], [2, 3]] {
                for class in [MatchClass::Moderate, MatchClass::Exceptional] {
                    let (g, c) = series_group(&ser, 690, 10_000, class);
                    for (i, v) in Resistor::enumerate(&g, &c, p).iter().enumerate() {
                        let m = v.draw(&g, &c, p);
                        let rules = testkit::findings(&m.shapes, &testkit::ports_with(&m, &[]), &pdk);
                        if !rules.is_empty() {
                            dirty.push(format!("{name} {ser:?} {class:?} #{i}: {rules:?}"));
                        }
                    }
                }
            }
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    fn generic_po(pdk: &verify::Pdk) -> verify::pdk::Overlay<'_> {
        verify::pdk::Overlay { pdk, recipe: pdk.recipe("resistor", "res_generic_po").expect("generic_po recipe") }
    }

    /// Two or more members of one `Series` unitization, member d `ser[d]` units of `w`×`l` (EXT-15), dummies on.
    fn series_group(ser: &[u16], w: i32, l: i32, class: MatchClass) -> (DeviceGroup, Constraints) {
        let (group, mut c) = crate::testkit::group_of(pnr_core::DeviceKind::Resistor, ser.len(), 1, w, l);
        let u = &mut c.unitization[0];
        u.series_parallel = SeriesParallel::Series;
        u.series = ser.to_vec();
        u.dummy_required = true;
        u.class = Some(class);
        (group, c)
    }

    fn interdig(ser: &[u16], p: &dyn Process, class: MatchClass) -> Macro {
        let (g, c) = series_group(ser, 690, 10_000, class);
        let v = Resistor::enumerate(&g, &c, p).into_iter().find(|v| v.pattern == Pattern::Interdig);
        v.unwrap_or_else(|| panic!("{ser:?}: no Interdig variant")).draw(&g, &c, p)
    }

    #[test]
    fn ratio_arrays_are_common_centroid() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let gp = generic_po(&pdk);
        for ser in [[1, 2], [2, 3], [1, 4], [3, 2], [4, 5], [9, 4]] {
            let m = interdig(&ser, &gp, MatchClass::Moderate);
            let sum = |o: u8| m.units.iter().filter(|u| u.owner == o).map(|u| i64::from(u.x)).sum::<i64>();
            assert_eq!(i64::from(ser[1]) * sum(0), i64::from(ser[0]) * sum(1), "{ser:?}");
            for o in 0..2u8 {
                assert_eq!(m.units.iter().filter(|u| u.owner == o).count(), usize::from(ser[usize::from(o)]), "{ser:?}");
            }
        }
    }

    #[test]
    fn high_po_ratios_fall_back_to_exact_blocks() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let op = high_po(&pdk);
        let model = ResModel::of(&op).expect("model");
        let (g, c) = series_group(&[1, 4], 690, 10_000, MatchClass::Moderate);
        let vs = Resistor::enumerate(&g, &c, &op);
        assert!(!vs.is_empty() && vs.iter().all(|v| v.pattern == Pattern::Single));
        for v in vs {
            let m = v.draw(&g, &c, &op);
            for (o, want) in [(0u8, 5_129.7), (1, R_690_40)] {
                let r: f64 = m.drawn.iter().filter(|d| d.owner == o).map(|d| model.ohm(d.w, d.l)).sum();
                assert!((r - want).abs() / want <= 0.005, "owner {o}: {r} vs {want}");
            }
        }
    }

    #[test]
    fn dummies_sit_at_the_segment_pitch() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let gp = generic_po(&pdk);
        let poly = req(&gp, "poly");
        let pitch = 690 + seg_gap(&gp);
        for class in [MatchClass::Moderate, MatchClass::Exceptional] {
            let (g, c) = series_group(&[1, 4], 690, 10_000, class);
            for v in Resistor::enumerate(&g, &c, &gp) {
                let m = v.draw(&g, &c, &gp);
                let mut xs: Vec<i32> = m.shapes.iter().filter(|s| s.layer == poly).map(|s| s.rect.x).collect();
                xs.sort_unstable();
                xs.dedup();
                let mut steps: Vec<i32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
                steps.dedup();
                assert_eq!(steps, [pitch], "{class:?} {:?}", v.pattern);
                if class == MatchClass::Exceptional {
                    assert_eq!(xs.iter().filter(|&&x| x < 0).count() as i32, (10_000 + pitch - 1) / pitch);
                }
            }
        }
    }

    #[test]
    fn every_join_has_the_same_via_count() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let (gp, op) = (generic_po(&pdk), high_po(&pdk));
        let check = |m: &Macro, p: &dyn Process, joins: usize| {
            let (mcon, li) = (req(p, "mcon"), req(p, "li"));
            assert_eq!(m.shapes.iter().filter(|s| s.layer == mcon).count(), 2 * joins);
            assert!(m.shapes.iter().filter(|s| s.layer == li).all(|s| s.rect.w <= 690));
        };
        check(&interdig(&[1, 4], &gp, MatchClass::Moderate), &gp, 3);
        let (g, c) = crate::testkit::group_of(pnr_core::DeviceKind::Resistor, 2, 1, 690, 40_000);
        let v = Resistor::enumerate(&g, &c, &op).into_iter().find(|v| v.segments == 2 && v.pattern == Pattern::Interdig).expect("2-segment Interdig");
        check(&v.draw(&g, &c, &op), &op, 2);
    }

    #[test]
    fn each_member_s_current_directions_cancel() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let m = interdig(&[2, 4], &generic_po(&pdk), MatchClass::Moderate);
        for o in 0..2u8 {
            assert_eq!(m.units.iter().filter(|u| u.owner == o).map(|u| i32::from(u.phi.1)).sum::<i32>(), 0, "owner {o}");
        }
    }

    /// sky130 `res_high_po` W = 0.69, L = 40 µm, from the model card (derived).
    const R_690_40: f64 = 18_949.2;

    fn high_po(pdk: &verify::Pdk) -> verify::pdk::Overlay<'_> {
        verify::pdk::Overlay { pdk, recipe: pdk.recipe("resistor", "res_high_po").expect("high_po recipe") }
    }

    /// Every variant of `n` members of W = 690, L = 40 µm at `nf` parallel
    /// strings each, drawn.
    fn variants(n: usize, nf: u16, p: &dyn Process) -> Vec<(usize, Macro)> {
        let (group, c) = crate::testkit::group_of(pnr_core::DeviceKind::Resistor, n, nf, 690, 40_000);
        let vs = Resistor::enumerate(&group, &c, p);
        assert!(!vs.is_empty(), "no variants");
        vs.iter().map(|v| (usize::from(v.segments), v.draw(&group, &c, p))).collect()
    }

    /// Model value per string (owner, string), Σ over its segments: string
    /// j's internal nodes are j·n + 1 ..= j·n + n − 1; with n = 1 every card
    /// is its own string.
    fn string_ohms(m: &Macro, n: usize, model: &ResModel) -> std::collections::BTreeMap<(u8, usize), f64> {
        let mut out = std::collections::BTreeMap::new();
        for (i, d) in m.drawn.iter().enumerate() {
            let j = d.nodes.iter().find_map(|x| match x {
                Node::Internal(k) => Some(usize::from(*k) / n),
                _ => None,
            });
            *out.entry((d.owner, if n == 1 { i } else { j.expect("a segmented string's card has an internal node") })).or_insert(0.0) += model.ohm(d.w, d.l);
        }
        out
    }

    fn within(r: f64) -> bool {
        (r - R_690_40).abs() / R_690_40 <= 0.005
    }

    #[test]
    fn segments_preserve_the_model_value() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let op = high_po(&pdk);
        let model = ResModel::of(&op).expect("high_po states a model");
        assert!((model.ohm(690, 40_000) - R_690_40).abs() < 0.5, "{}", model.ohm(690, 40_000));
        let vs = variants(1, 1, &op);
        assert!(vs.iter().any(|(n, _)| *n == 2), "a 2-segment variant at L = 40 um");
        for (n, m) in vs {
            let ohms = string_ohms(&m, n, &model);
            assert_eq!(ohms.len(), 1, "n={n}");
            assert!(ohms.values().all(|&r| within(r)), "n={n}: {ohms:?}");
        }
    }

    #[test]
    fn short_segments_are_not_offered() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        // n = 4 needs 9.148 um segments, under res_min_segment 10 um.
        assert!(variants(1, 1, &high_po(&pdk)).iter().all(|(n, _)| *n != 4));
    }

    #[test]
    fn a_multiplied_resistor_draws_parallel_strings() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let op = high_po(&pdk);
        let model = ResModel::of(&op).expect("model");
        for (n, m) in variants(1, 2, &op) {
            assert_eq!(m.drawn.len(), 2 * n, "n={n}");
            for t in ["d0:P", "d0:N"] {
                assert_eq!(m.pins.iter().filter(|p| p.name == t).count(), 2, "n={n} {t}");
            }
            let ohms = string_ohms(&m, n, &model);
            assert_eq!(ohms.len(), 2, "n={n}: {ohms:?}");
            assert!(ohms.values().all(|&r| within(r)), "n={n}: {ohms:?}");
        }
    }

    #[test]
    fn every_variant_extracts_one_resistor_per_segment() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let op = high_po(&pdk);
        for nf in [1, 2] {
            for (n, m) in variants(1, nf, &op) {
                let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).expect("extracts");
                let rs = spice.lines().filter(|l| l.starts_with('R')).count();
                assert_eq!(rs, m.drawn.len(), "nf={nf} n={n}:\n{spice}");
            }
        }
    }
}
