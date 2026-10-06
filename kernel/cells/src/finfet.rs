//! FinFET MOS generator: the planar generator's job on a fin process.
//! Horizontal fins at the deck's fin pitch (channel width is a fin count),
//! vertical gates at its gate pitch (length is the deck's one gate width).
//! Each S/D gap carries a trench and a local-interconnect strip with a V0 up
//! to an M1 pin; each gate a local-interconnect pad below the active, its V0
//! on one M1 strap per device. Select implants and the well at the deck's
//! enclosures; a finned tap strip above carries the bulk. Every number is
//! the deck's.

use crate::builder::dim;
use analog::matching::pattern::{self, Outer};
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, Process, Rect};

use crate::builder::{pin, req, sizing, unitization, Builder, Sizing};
use crate::Cell;

/// One FinFET variant: each member on its own active, side by side
/// (variant 0), or every member interleaved on one shared common-centroid
/// active. Every active ends in one uncontacted edge gate per side.
#[derive(Clone)]
pub struct FinFet {
    /// Members interleave on one active ([`pattern::diffusion_cc_row`]).
    pub shared: bool,
}

/// The deck's front-end numbers, nm (areas nm²); a rule the deck omits reads
/// 0. Roles map onto the planar names: fin layer `fin`, gate `poly`, active
/// `diff`, V0 `licon`, M1 `li`, V1 `mcon`, M2 `met1`.
struct Rules {
    /// Fin width.
    fin_w: i32,
    /// Fin pitch: width plus fin spacing.
    fin_p: i32,
    /// Gate length: the poly width, never under `min_gate_l`.
    gate_w: i32,
    /// Gate pitch: gate length plus poly spacing.
    gate_p: i32,
    /// Active extension past the outer fins' ends.
    act_past_fin: i32,
    /// Active extension past the outer (edge) gates.
    act_past_gate: i32,
    /// Gate extension past the active (end cap).
    gate_past_act: i32,
    /// S/D trench width.
    sdt_w: i32,
    /// S/D trench to gate spacing.
    sdt_gate: i32,
    /// S/D local-interconnect width.
    lisd_w: i32,
    /// V0 cut size.
    v0: i32,
    /// LISD enclosure of V0 (max of end cap and enclosure).
    v0_in_lisd: i32,
    /// M1 enclosure of V0 (max of end cap and enclosure).
    v0_in_m1: i32,
    /// M1 minimum width.
    m1_w: i32,
    /// M1 minimum spacing.
    m1_s: i32,
    /// M1 minimum area.
    m1_area: i64,
    /// Gate local-interconnect width.
    lig_w: i32,
    /// Gate local-interconnect minimum area.
    lig_area: i64,
    /// LIG clearance to the channel (to poly or to active, the larger).
    lig_channel: i32,
    /// Active to active spacing.
    act_s: i32,
    /// Select implant enclosure of active.
    sel_enc: i32,
    /// Select implant extension past the gates.
    sel_past_gate: i32,
    /// Select implant minimum width.
    sel_w: i32,
    /// Nwell enclosure of active.
    well_enc: i32,
    /// Field gate end to (tap) active spacing.
    gate_field: i32,
}

impl Rules {
    /// Reads every rule from `p`.
    fn of(p: &dyn Process) -> Self {
        let w = |r: &str| p.width(r).unwrap_or(0);
        let s = |r: &str| p.space(r).unwrap_or(0);
        let ext = |a: &str, b: &str| p.extension(a, b).unwrap_or(0);
        let cap = |a: &str, b: &str| p.endcap(a, b).unwrap_or(0).max(p.enclosure(a, b).unwrap_or(0));
        let (fin_w, gate_w) = (w("fin"), w("poly").max(dim(p, "min_gate_l")));
        Rules {
            fin_w,
            fin_p: fin_w + s("fin"),
            gate_w,
            gate_p: gate_w + s("poly"),
            act_past_fin: ext("diff", "fin"),
            act_past_gate: ext("diff", "poly"),
            gate_past_act: ext("poly", "diff"),
            sdt_w: w("sdt"),
            sdt_gate: p.space_between("sdt", "poly").unwrap_or(0),
            lisd_w: w("lisd"),
            v0: w("licon"),
            v0_in_lisd: cap("lisd", "licon"),
            v0_in_m1: cap("li", "licon"),
            m1_w: w("li"),
            m1_s: s("li"),
            m1_area: p.area("li").unwrap_or(0),
            lig_w: w("lig"),
            lig_area: p.area("lig").unwrap_or(0),
            lig_channel: p.space_between("lig", "poly").unwrap_or(0).max(p.space_between("lig", "diff").unwrap_or(0)),
            act_s: s("diff"),
            sel_enc: p.enclosure("nsdm", "diff").unwrap_or(0).max(p.enclosure("psdm", "diff").unwrap_or(0)),
            sel_past_gate: ext("nsdm", "poly").max(ext("psdm", "poly")),
            sel_w: w("nsdm").max(w("psdm")),
            well_enc: p.enclosure("nwell", "diff").unwrap_or(0),
            gate_field: p.space_between("diff", "poly").unwrap_or(0),
        }
    }
}

impl Cell for FinFet {
    /// Separate actives always; the shared common-centroid row too when the
    /// group has two or more members and a diffusion-legal centroid order
    /// exists. Nothing for an empty group or a deck without a `fin` layer.
    fn enumerate(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() || process.layer("fin").is_none() {
            return vec![];
        }
        let s = group_sizing(group, c, process);
        let mut v = vec![FinFet { shared: false }];
        if s.dev_nf.len() > 1 && pattern::diffusion_cc_row(&s.dev_nf, Outer::Drain).is_some() {
            v.push(FinFet { shared: true });
        }
        v
    }

    /// Draws this variant.
    ///
    /// # Panics
    /// If `group` is empty, if the deck lacks a mandatory role ([`req`]), or
    /// for `shared` when no centroid row exists (never enumerated).
    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let r = Rules::of(process);
        let s = group_sizing(group, constraints, process);
        let pmos = unitization(group, constraints).is_some_and(|u| u.device_type == DeviceKind::Pmos);
        let mut b = Builder::new(process.grid());
        let (fin, gate, act) = (req(process, "fin"), req(process, "poly"), req(process, "diff"));
        let (sdt, lisd, lig) = (req(process, "sdt"), req(process, "lisd"), req(process, "lig"));
        let (v0, m1) = (req(process, "licon"), req(process, "li"));
        let (v1, m2) = (req(process, "mcon"), req(process, "met1"));
        // Drain strap: a V1 per drain strip on an M1 landing pad, one M2 strap.
        let v1_w = process.width("mcon").unwrap_or(r.v0);
        let v1_in_m1 = process.endcap("li", "mcon").unwrap_or(0).max(process.enclosure("li", "mcon").unwrap_or(0));
        let v1_in_m2 = process.endcap("met1", "mcon").unwrap_or(0).max(process.enclosure("met1", "mcon").unwrap_or(0));
        let m2_w = process.width("met1").unwrap_or(r.m1_w).max(v1_w + 2 * v1_in_m2.min(v1_in_m1));
        let v1_pad = v1_w + 2 * v1_in_m1;
        let (own_sel, tap_sel) = if pmos { (req(process, "psdm"), req(process, "nsdm")) } else { (req(process, "nsdm"), req(process, "psdm")) };

        // Channel width → the nearest fin count, at least one; the drawn
        // `nfin·fin_p` is the height `Unit.weight` carries.
        let nfin = ((s.unit_w + r.fin_p / 2) / r.fin_p.max(1)).max(1);
        let h = nfin * r.fin_p;
        // S/D gap between gates; `end` sizes the tap strip's first contact.
        let gap = r.gate_p - r.gate_w;
        let end = gap.max(r.act_past_gate).max(r.sdt_w + 2 * r.sdt_gate);
        let n_dev = group.devices.len();
        // Rows: (owner of each gate, S regions odd). Separate: one active per
        // member, region 0 a source. Shared: one CC row, its ends drains, every
        // member boundary on a source; member `k` takes drain and gate track `k`.
        let rows: Vec<(Vec<usize>, bool)> = if self.shared {
            vec![(pattern::diffusion_cc_row(&s.dev_nf, Outer::Drain).expect("enumerate offers shared only for a legal row"), true)]
        } else {
            (0..n_dev).map(|di| (vec![di; usize::from(s.dev_nf.get(di).copied().unwrap_or(1).max(1))], false)).collect()
        };
        let track = |k: usize| if self.shared { k as i32 } else { 0 };
        let n_tracks = if self.shared { n_dev as i32 } else { 1 };
        // Rows: M1 S/D pins at the active's centre; gate pads below it, far
        // enough that the gate strap clears the S/D M1 by its spacing.
        let v0_y = (h - r.v0) / 2;
        let m1_h = (r.v0 + 2 * r.v0_in_m1).max(i32::try_from(r.m1_area / i64::from(r.m1_w.max(1)) + 1).unwrap_or(0));
        let sd_m1_y = v0_y + r.v0 / 2 - m1_h / 2;
        // Drain track `t`: its V1 pads and M2 strap, one M2 pitch per track up
        // (end-of-line spacing: a one-drain member's strap is all line ends).
        let d_pitch = m2_w + process.space("met1").unwrap_or(0).max(process.eol_space("met1").unwrap_or(0));
        let y_d = |t: i32| v0_y + r.v0 / 2 + t * d_pitch;
        // Source strap above every drain track's landing pads.
        let d_top = (sd_m1_y + m1_h).max(y_d(n_tracks - 1) + v1_pad / 2);
        let s_strap_y = d_top + r.m1_s.max(process.eol_space("li").unwrap_or(0));
        let strap_top = sd_m1_y.min(-r.lig_channel) - r.m1_s;
        let gv0_y = strap_top - r.m1_w + (r.m1_w - r.v0) / 2;
        let lig_h = (r.v0 + 2).max(i32::try_from(r.lig_area / i64::from(r.lig_w.max(r.gate_w).max(1)) + 1).unwrap_or(0));
        let lig_y = (gv0_y + r.v0 / 2 - lig_h / 2).min(-r.lig_channel - lig_h);
        // Gate track `t` sits `t·gpitch` below track 0: its M1 strap and its
        // LIG pads each clear the next track's by their spacing.
        let eol = |l: &str| process.eol_space(l).unwrap_or(0);
        let gpitch = (r.m1_w + r.m1_s.max(eol("li"))).max(lig_h + process.space("lig").unwrap_or(0).max(eol("lig")));
        let gate_lo_min = lig_y - (n_tracks - 1) * gpitch;
        let gate_hi = h + r.gate_past_act;
        let lw = r.lisd_w.max(r.v0 + 2 * r.v0_in_lisd);

        let mut x = 0;
        let mut acts: Vec<Rect> = Vec::new();
        for (owners, s_odd) in &rows {
            let n = owners.len() as i32;
            let mut members: Vec<usize> = Vec::new();
            for &k in owners {
                if !members.contains(&k) {
                    members.push(k);
                }
            }
            // One uncontacted edge gate per diffusion end (gates -1 and n,
            // diffusion-break style). Not a `pnr_core::Dummy`: that means a
            // bulk-tied, extracted transistor with an LVS card, and this deck
            // recognises no devices. A deck that does needs a tie and a record.
            let a = Rect { x, y: 0, w: 2 * r.act_past_gate + (n + 2) * r.gate_w + (n + 1) * gap, h };
            b.rect(act, a);
            acts.push(a);
            for k in 0..nfin {
                let y = k * r.fin_p + (r.fin_p - r.fin_w) / 2;
                b.rect(fin, Rect { x: a.x + r.act_past_fin, y, w: a.w - 2 * r.act_past_fin, h: r.fin_w });
            }
            let gx = |i: i32| a.x + r.act_past_gate + (i + 1) * r.gate_p;
            // S/D region j between gates j-1 and j, every one `gap` wide.
            let (mut s_x, mut d_x): (Vec<i32>, Vec<(usize, i32)>) = (Vec::new(), Vec::new());
            for j in 0..=n {
                let cx = (gx(j - 1) + r.gate_w + gx(j)) / 2;
                b.rect(sdt, Rect { x: cx - r.sdt_w / 2, y: 0, w: r.sdt_w, h });
                b.rect(lisd, Rect { x: cx - lw / 2, y: 0, w: lw, h });
                b.rect(v0, Rect { x: cx - r.v0 / 2, y: v0_y, w: r.v0, h: r.v0 });
                if (j % 2 == 1) == *s_odd {
                    // A source strip runs up to the source strap.
                    b.rect(m1, Rect { x: cx - r.m1_w / 2, y: sd_m1_y, w: r.m1_w, h: s_strap_y + r.m1_w - sd_m1_y });
                    s_x.push(cx);
                } else {
                    // A drain is its right gate's (the row's last: its left).
                    let k = owners[(j as usize).min(owners.len() - 1)];
                    let yd = y_d(track(k));
                    b.rect(m1, Rect { x: cx - r.m1_w / 2, y: sd_m1_y, w: r.m1_w, h: m1_h.max(yd - sd_m1_y) });
                    b.rect(m1, Rect { x: cx - v1_pad / 2, y: yd - v1_pad / 2, w: v1_pad, h: v1_pad });
                    b.rect(v1, Rect { x: cx - v1_w / 2, y: yd - v1_w / 2, w: v1_w, h: v1_w });
                    d_x.push((k, cx));
                }
            }
            let (s0, s1) = (s_x[0], *s_x.last().unwrap());
            b.rect(m1, Rect { x: s0 - r.m1_w / 2, y: s_strap_y, w: s1 - s0 + r.m1_w, h: r.m1_w });
            for &k in &members {
                b.pin(pin(k, "S", Rect { x: s0 - r.m1_w / 2, y: s_strap_y, w: r.m1_w, h: r.m1_w }, m1));
                let xs: Vec<i32> = d_x.iter().filter(|d| d.0 == k).map(|d| d.1).collect();
                let (d0, d1, yd) = (xs[0], *xs.last().unwrap(), y_d(track(k)));
                b.rect(m2, Rect { x: d0 - v1_w / 2 - v1_in_m2, y: yd - m2_w / 2, w: d1 - d0 + v1_w + 2 * v1_in_m2, h: m2_w });
                b.pin(pin(k, "D", Rect { x: d0 - v1_w / 2, y: yd - v1_w / 2, w: v1_w, h: v1_w }, m2));
            }
            for i in -1..=n {
                let g = gx(i);
                if i < 0 || i == n {
                    b.rect(gate, Rect { x: g, y: gate_lo_min, w: r.gate_w, h: gate_hi - gate_lo_min });
                    continue;
                }
                let k = owners[i as usize];
                let off = track(k) * gpitch;
                b.rect(gate, Rect { x: g, y: lig_y - off, w: r.gate_w, h: gate_hi - lig_y + off });
                let lgw = r.lig_w.max(r.gate_w);
                b.rect(lig, Rect { x: g + r.gate_w / 2 - lgw / 2, y: lig_y - off, w: lgw, h: lig_h });
                b.rect(v0, Rect { x: g + r.gate_w / 2 - r.v0 / 2, y: gv0_y - off, w: r.v0, h: r.v0 });
                let c = g + r.gate_w / 2;
                let phi = if (i % 2 == 0) != *s_odd { 1 } else { -1 };
                b.unit(pnr_core::Unit { owner: k as u8, x: c, y: h / 2, weight: i64::from(r.gate_w) * i64::from(h), phi: (phi, 0), sa: c - a.x, sb: a.x + a.w - c });
            }
            // One M1 strap per member over its own gate contacts.
            for &k in &members {
                let gs: Vec<i32> = (0..n).filter(|&i| owners[i as usize] == k).map(gx).collect();
                let off = track(k) * gpitch;
                let (g0, g1) = (gs[0] + r.gate_w / 2 - r.v0 / 2, *gs.last().unwrap() + r.gate_w / 2 + r.v0 / 2);
                b.rect(m1, Rect { x: g0 - r.v0_in_m1, y: strap_top - off - r.m1_w, w: g1 - g0 + 2 * r.v0_in_m1, h: r.m1_w });
                b.pin(pin(k, "G", Rect { x: g0, y: gv0_y - off, w: r.v0, h: r.v0 }, m1));
            }
            x = a.x + a.w + r.act_s.max(2 * r.sel_enc);
        }
        let (x0, x1) = (acts[0].x, x - r.act_s.max(2 * r.sel_enc));
        // The device select over the actives and gates.
        let sel = Rect { x: x0 - r.sel_enc, y: (-r.sel_enc).min(gate_lo_min - r.sel_past_gate), w: x1 - x0 + 2 * r.sel_enc, h: 0 };
        let sel_top = (h + r.sel_enc).max(gate_hi + r.sel_past_gate);
        let sel = Rect { h: (sel_top - sel.y).max(r.sel_w), ..sel };
        b.rect(own_sel, sel);

        // Tap strip above: its own active and fins under the other select,
        // clear of the device select and of the gates' field ends.
        let tap_h = r.fin_p.max(r.lisd_w);
        // Its rail (at the tap's centre) a spacing above the source strap.
        let tap_y = (sel.y + sel.h + r.sel_enc)
            .max(gate_hi + r.gate_field)
            .max(h + r.act_s)
            .max(s_strap_y + r.m1_w + r.m1_s - (tap_h - r.m1_w) / 2);
        let tap = Rect { x: x0, y: tap_y, w: x1 - x0, h: tap_h };
        b.rect(act, tap);
        b.rect(fin, Rect { x: tap.x + r.act_past_fin, y: tap_y + (tap_h - r.fin_w) / 2, w: tap.w - 2 * r.act_past_fin, h: r.fin_w });
        let tsel = Rect { x: tap.x - r.sel_enc, y: tap_y - r.sel_enc, w: tap.w + 2 * r.sel_enc, h: (tap_h + 2 * r.sel_enc).max(r.sel_w) };
        b.rect(tap_sel, tsel);
        let tv0_y = tap_y + (tap_h - r.v0) / 2;
        let mut cx = tap.x + end / 2;
        let mut first = None;
        while cx + lw / 2 <= tap.x + tap.w - r.sdt_gate {
            b.rect(sdt, Rect { x: cx - r.sdt_w / 2, y: tap_y, w: r.sdt_w, h: tap_h });
            b.rect(lisd, Rect { x: cx - lw / 2, y: tap_y, w: lw, h: tap_h });
            b.rect(v0, Rect { x: cx - r.v0 / 2, y: tv0_y, w: r.v0, h: r.v0 });
            first.get_or_insert(cx);
            // A deck without a gate pitch gets one contact, not an endless walk.
            if r.gate_p <= 0 {
                break;
            }
            cx += r.gate_p;
        }
        let rail = Rect { x: tap.x, y: tv0_y + r.v0 / 2 - r.m1_w / 2, w: tap.w, h: r.m1_w };
        b.rect(m1, rail);
        let at = Rect { x: first.unwrap_or(tap.x) - r.v0 / 2, y: tv0_y, w: r.v0, h: r.v0 };
        for di in 0..n_dev {
            b.pin(pin(di, "B", at, m1));
        }
        if pmos {
            if let Some(well) = process.layer("nwell") {
                let top = tsel.y + tsel.h;
                b.rect(well, Rect { x: x0 - r.well_enc, y: sel.y.min(-r.well_enc), w: x1 - x0 + 2 * r.well_enc, h: top.max(tap_y + tap_h + r.well_enc) - sel.y.min(-r.well_enc) });
            }
        }
        b.finish()
    }
}

/// [`sizing`] with a one-fin default width and the deck's minimum gate length.
fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let fin_p = process.width("fin").unwrap_or(0) + process.space("fin").unwrap_or(0);
    sizing(group, c, fin_p.max(1), dim(process, "min_gate_l"))
}

#[cfg(test)]
mod tests {
    use super::FinFet;
    use crate::testkit::group_of;
    use crate::Cell;
    use pnr_core::{DeviceKind, Macro, Process, Rect};

    fn deck() -> verify::Pdk {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let json = std::fs::read_to_string(root.join("pdks/generic_finfet.json")).expect("pdks/generic_finfet.json");
        verify::Pdk::from_json(&json).expect("generic_finfet loads")
    }

    /// `kind` group with per-member finger counts `nf` (ratio = counts).
    fn group(kind: DeviceKind, nf: &[u16], w: i32) -> (pnr_core::DeviceGroup, analog::Constraints) {
        let (g, mut c) = group_of(kind, nf.len(), 1, w, 20);
        c.unitization[0].dev_nf = nf.to_vec();
        c.unitization[0].target_ratio = nf.to_vec();
        (g, c)
    }

    fn on(m: &Macro, pdk: &verify::Pdk, role: &str) -> Vec<Rect> {
        let l = pdk.layer(role).expect("role");
        m.shapes.iter().filter(|s| s.layer == l).map(|s| s.rect).collect()
    }

    #[test]
    fn nfin_rounds_to_the_nearest_fin() {
        let pdk = deck();
        for (w, fins) in [(10, 1i64), (40, 1), (41, 2), (1680, 62)] {
            let (g, c) = group(DeviceKind::Nmos, &[1], w);
            let m = FinFet { shared: false }.draw(&g, &c, &pdk);
            assert_eq!(m.units[0].weight, 20 * 27 * fins, "w={w}");
        }
    }

    #[test]
    fn every_active_has_one_dummy_gate_per_end() {
        let pdk = deck();
        let (g, c) = group(DeviceKind::Nmos, &[2, 2], 1680);
        let vs = FinFet::enumerate(&g, &c, &pdk);
        assert_eq!(vs.iter().map(|v| v.shared).collect::<Vec<_>>(), [false, true]);
        for v in vs {
            let m = v.draw(&g, &c, &pdk);
            let actives = on(&m, &pdk, "diff").len() - 1; // minus the tap's
            assert_eq!(actives, if v.shared { 1 } else { 2 });
            assert_eq!(on(&m, &pdk, "poly").len(), m.units.len() + 2 * actives, "shared={}", v.shared);
        }
    }

    #[test]
    fn sa_and_sb_reach_the_active_ends() {
        let pdk = deck();
        for nf in [&[2u16, 2][..], &[3]] {
            let (g, c) = group(DeviceKind::Nmos, nf, 1680);
            for v in FinFet::enumerate(&g, &c, &pdk) {
                let m = v.draw(&g, &c, &pdk);
                let acts = on(&m, &pdk, "diff");
                for u in &m.units {
                    assert!(u.sa > 0 && u.sb > 0);
                    let a = acts.iter().find(|a| a.x <= u.x && u.x <= a.x + a.w && a.y <= u.y && u.y <= a.y + a.h).expect("unit on an active");
                    assert_eq!(u.sa + u.sb, a.w, "nf={nf:?} shared={}", v.shared);
                }
                for k in 0..nf.len() {
                    let mine = m.units.iter().filter(|u| usize::from(u.owner) == k);
                    let (sa, sb) = mine.fold((0, 0), |(a, b), u| (a + u.sa, b + u.sb));
                    assert_eq!(sa, sb, "nf={nf:?} shared={} member {k}", v.shared);
                }
            }
        }
    }

    /// T6: the shared row's members share one centroid and cancel their
    /// S/D orientation.
    #[test]
    fn the_shared_row_is_common_centroid() {
        let pdk = deck();
        for nf in [[2u16, 2], [2, 4], [4, 4]] {
            let (g, c) = group(DeviceKind::Nmos, &nf, 1680);
            let vs = FinFet::enumerate(&g, &c, &pdk);
            let v = vs.iter().find(|v| v.shared).unwrap_or_else(|| panic!("{nf:?}: no shared variant"));
            let m = v.draw(&g, &c, &pdk);
            let n_all = m.units.len() as i64;
            let x_all: i64 = m.units.iter().map(|u| i64::from(u.x)).sum();
            for (k, &want) in nf.iter().enumerate() {
                let mine: Vec<_> = m.units.iter().filter(|u| usize::from(u.owner) == k).collect();
                assert_eq!(mine.len(), usize::from(want), "{nf:?} member {k}");
                let x_k: i64 = mine.iter().map(|u| i64::from(u.x)).sum();
                assert_eq!(x_k * n_all, x_all * mine.len() as i64, "{nf:?} member {k} centroid");
                assert_eq!(mine.iter().map(|u| i32::from(u.phi.0)).sum::<i32>(), 0, "{nf:?} member {k} phi");
            }
        }
        let (g, c) = group(DeviceKind::Nmos, &[1, 1], 1680);
        assert!(FinFet::enumerate(&g, &c, &pdk).iter().all(|v| !v.shared));
    }

    /// T1: separate and shared rows, Nmos and Pmos, DRC and ERC clean, with
    /// drains and gates private per member (a cross-member short is an ERC
    /// finding). Release only: generic_finfet's ERC trips an engine debug
    /// assertion (`tests/cell_selfcheck.rs`, `the_mosfet_is_clean_on_every_deck`).
    #[cfg(not(debug_assertions))]
    #[test]
    fn every_variant_is_drc_and_erc_clean() {
        let pdk = deck();
        let mut dirty = Vec::new();
        for kind in [DeviceKind::Nmos, DeviceKind::Pmos] {
            for nf in [&[1u16][..], &[2, 2], &[2, 4], &[4, 4]] {
                let (g, c) = group(kind, nf, 1680);
                dirty.extend(crate::testkit::dirty_group_with::<FinFet>(&g, &c, &pdk, &["S", "B"]).into_iter().map(|d| format!("{kind:?} {nf:?} {d}")));
            }
        }
        assert!(dirty.is_empty(), "{}", dirty.join("\n"));
    }
}

/// Corner cases for the FinFET generator (cleanup step 2). Oracles: the doc
/// comments and structural invariants of a drawn macro.
#[cfg(test)]
mod cleanup_tests {
    use super::FinFet;
    use crate::builder::{contains, fake::Deck};
    use crate::testkit::group_of;
    use crate::Cell;
    use pnr_core::{DeviceGroup, DeviceKind};

    fn finfet_deck() -> verify::Pdk {
        verify::Pdk::builtin("generic_finfet").expect("generic_finfet builtin")
    }

    #[test]
    fn enumerate_offers_nothing_for_an_empty_group_or_a_planar_deck() {
        let (_, c) = group_of(DeviceKind::Nmos, 1, 1, 1680, 20);
        assert!(FinFet::enumerate(&DeviceGroup { devices: vec![] }, &c, &finfet_deck()).is_empty());
        let (g, c) = group_of(DeviceKind::Nmos, 2, 2, 1680, 150);
        assert!(FinFet::enumerate(&g, &c, &verify::Pdk::builtin("sky130").unwrap()).is_empty(), "no fin layer");
    }

    #[test]
    fn a_lone_device_is_never_shared() {
        let (g, c) = group_of(DeviceKind::Nmos, 1, 4, 1680, 20);
        let vs = FinFet::enumerate(&g, &c, &finfet_deck());
        assert_eq!(vs.iter().map(|v| v.shared).collect::<Vec<_>>(), [false]);
    }

    /// Metamorphic: drawing is pure and stays inside its bbox, with one
    /// S/D/G/B pin per member.
    #[test]
    fn every_variant_draws_deterministically_inside_its_bbox() {
        let pdk = finfet_deck();
        for (kind, n, nf) in [(DeviceKind::Nmos, 1, 1), (DeviceKind::Pmos, 2, 2), (DeviceKind::Nmos, 2, 4)] {
            let (g, c) = group_of(kind, n, nf, 1680, 20);
            for v in FinFet::enumerate(&g, &c, &pdk) {
                let (a, b) = (v.draw(&g, &c, &pdk), v.draw(&g, &c, &pdk));
                assert_eq!(a.shapes, b.shapes, "{kind:?} shared={}", v.shared);
                assert_eq!(a.pins, b.pins);
                assert!(a.shapes.iter().all(|s| contains(&a.bbox, &s.rect)));
                assert!(a.pins.iter().all(|p| contains(&a.bbox, &p.at)));
                for di in 0..n {
                    for t in ["S", "D", "G", "B"] {
                        let k = a.pins.iter().filter(|p| p.name == format!("d{di}:{t}")).count();
                        assert_eq!(k, 1, "{kind:?} shared={} d{di}:{t}", v.shared);
                    }
                }
            }
        }
    }

    /// A deck with a fin layer but no gate pitch must still draw (the tap
    /// strip's contact walk advances by the gate pitch).
    #[test]
    fn a_zero_gate_pitch_terminates() {
        let roles = ["fin", "poly", "diff", "sdt", "lisd", "lig", "licon", "li", "mcon", "met1", "nsdm", "psdm"];
        let p = Deck::new(5, &roles).with("w:fin", 10).with("s:fin", 10).with("w:licon", 20).with("w:li", 20).with("ext:diff:poly", 100);
        let (g, c) = group_of(DeviceKind::Nmos, 1, 1, 40, 20);
        let vs = FinFet::enumerate(&g, &c, &p);
        assert_eq!(vs.len(), 1);
        let m = vs[0].draw(&g, &c, &p);
        assert!(!m.shapes.is_empty());
    }
}
