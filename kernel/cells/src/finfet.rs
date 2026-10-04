//! FinFET MOS generator: the planar generator's job on a fin process.
//! Horizontal fins at the deck's fin pitch (channel width is a fin count),
//! vertical gates at its gate pitch (length is the deck's one gate width).
//! Each S/D gap carries a trench and a local-interconnect strip with a V0 up
//! to an M1 pin; each gate a local-interconnect pad below the active, its V0
//! on one M1 strap per device. Select implants and the well at the deck's
//! enclosures; a finned tap strip above carries the bulk. Every number is
//! the deck's.

use crate::builder::dim;
use analog::Constraints;
use pnr_core::{DeviceGroup, DeviceKind, Macro, Process, Rect};

use crate::builder::{pin, req, sizing, unitization, Builder, Sizing};
use crate::Cell;

/// One FinFET variant: every member's fingers in one row, members side by
/// side on their own active.
#[derive(Clone)]
pub struct FinFet;

/// The deck's front-end numbers, nm.
struct Rules {
    fin_w: i32,
    fin_p: i32,
    gate_w: i32,
    gate_p: i32,
    act_past_fin: i32,
    act_past_gate: i32,
    gate_past_act: i32,
    sdt_w: i32,
    sdt_gate: i32,
    lisd_w: i32,
    v0: i32,
    v0_in_lisd: i32,
    v0_in_m1: i32,
    m1_w: i32,
    m1_s: i32,
    m1_area: i64,
    lig_w: i32,
    lig_area: i64,
    lig_channel: i32,
    act_s: i32,
    sel_enc: i32,
    sel_past_gate: i32,
    sel_w: i32,
    well_enc: i32,
    gate_field: i32,
}

impl Rules {
    fn of(p: &dyn Process) -> Self {
        let w = |r: &str| p.width(r).unwrap_or(0);
        let s = |r: &str| p.space(r).unwrap_or(0);
        let ext = |a: &str, b: &str| p.extension(a, b).unwrap_or(0);
        let cap = |a: &str, b: &str| {
            p.endcap(a, b)
                .unwrap_or(0)
                .max(p.enclosure(a, b).unwrap_or(0))
        };
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
            lig_channel: p
                .space_between("lig", "poly")
                .unwrap_or(0)
                .max(p.space_between("lig", "diff").unwrap_or(0)),
            act_s: s("diff"),
            sel_enc: p
                .enclosure("nsdm", "diff")
                .unwrap_or(0)
                .max(p.enclosure("psdm", "diff").unwrap_or(0)),
            sel_past_gate: ext("nsdm", "poly").max(ext("psdm", "poly")),
            sel_w: w("nsdm").max(w("psdm")),
            well_enc: p.enclosure("nwell", "diff").unwrap_or(0),
            gate_field: p.space_between("diff", "poly").unwrap_or(0),
        }
    }
}

impl Cell for FinFet {
    fn enumerate(group: &DeviceGroup, _c: &Constraints, process: &dyn Process) -> Vec<Self> {
        if group.devices.is_empty() || process.layer("fin").is_none() {
            return vec![];
        }
        vec![FinFet]
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        let r = Rules::of(process);
        let s = group_sizing(group, constraints, process);
        let pmos =
            unitization(group, constraints).is_some_and(|u| u.device_type == DeviceKind::Pmos);
        let mut b = Builder::new(process.grid());
        let (fin, gate, act) = (
            req(process, "fin"),
            req(process, "poly"),
            req(process, "diff"),
        );
        let (sdt, lisd, lig) = (
            req(process, "sdt"),
            req(process, "lisd"),
            req(process, "lig"),
        );
        let (v0, m1) = (req(process, "licon"), req(process, "li"));
        let (v1, m2) = (req(process, "mcon"), req(process, "met1"));
        // Drain strap: a V1 per drain strip on an M1 landing pad, one M2 strap.
        let v1_w = process.width("mcon").unwrap_or(r.v0);
        let v1_in_m1 = process
            .endcap("li", "mcon")
            .unwrap_or(0)
            .max(process.enclosure("li", "mcon").unwrap_or(0));
        let v1_in_m2 = process
            .endcap("met1", "mcon")
            .unwrap_or(0)
            .max(process.enclosure("met1", "mcon").unwrap_or(0));
        let m2_w = process
            .width("met1")
            .unwrap_or(r.m1_w)
            .max(v1_w + 2 * v1_in_m2.min(v1_in_m1));
        let v1_pad = v1_w + 2 * v1_in_m1;
        let (own_sel, tap_sel) = if pmos {
            (req(process, "psdm"), req(process, "nsdm"))
        } else {
            (req(process, "nsdm"), req(process, "psdm"))
        };

        // Channel width → fins; the active holds them at the fin pitch.
        let nfin = (s.unit_w / r.fin_p.max(1)).max(1);
        let h = nfin * r.fin_p;
        // S/D gap between gates, and the end regions (at least the active's
        // extension past the outer gate, and room for the trench).
        let gap = r.gate_p - r.gate_w;
        let end = gap.max(r.act_past_gate).max(r.sdt_w + 2 * r.sdt_gate);
        // Rows: M1 S/D pins at the active's centre; gate pads below it, far
        // enough that the gate strap clears the S/D M1 by its spacing.
        let v0_y = (h - r.v0) / 2;
        let m1_h = (r.v0 + 2 * r.v0_in_m1)
            .max(i32::try_from(r.m1_area / i64::from(r.m1_w.max(1)) + 1).unwrap_or(0));
        let sd_m1_y = v0_y + r.v0 / 2 - m1_h / 2;
        // Source strap above the drain strips' landing pads.
        let d_top = (sd_m1_y + m1_h).max(v0_y + r.v0 / 2 + v1_pad / 2);
        let s_strap_y = d_top + r.m1_s.max(process.eol_space("li").unwrap_or(0));
        let strap_top = sd_m1_y.min(-r.lig_channel) - r.m1_s;
        let gv0_y = strap_top - r.m1_w + (r.m1_w - r.v0) / 2;
        let lig_h = (r.v0 + 2).max(
            i32::try_from(r.lig_area / i64::from(r.lig_w.max(r.gate_w).max(1)) + 1).unwrap_or(0),
        );
        let lig_y = (gv0_y + r.v0 / 2 - lig_h / 2).min(-r.lig_channel - lig_h);
        let gate_lo = lig_y;
        let gate_hi = h + r.gate_past_act;

        let n_dev = group.devices.len();
        let (mut s_x, mut d_x): (Vec<i32>, Vec<i32>) = (Vec::new(), Vec::new());
        let mut x = 0;
        let mut acts: Vec<Rect> = Vec::new();
        for di in 0..n_dev {
            let nf = i32::from(s.dev_nf.get(di).copied().unwrap_or(1).max(1));
            let a = Rect {
                x,
                y: 0,
                w: 2 * end + nf * r.gate_w + (nf - 1) * gap,
                h,
            };
            b.rect(act, a);
            acts.push(a);
            for k in 0..nfin {
                let y = k * r.fin_p + (r.fin_p - r.fin_w) / 2;
                b.rect(
                    fin,
                    Rect {
                        x: a.x + r.act_past_fin,
                        y,
                        w: a.w - 2 * r.act_past_fin,
                        h: r.fin_w,
                    },
                );
            }
            let gx = |i: i32| a.x + end + i * r.gate_p;
            // S/D regions: 0 left of gate 0, i right of gate i-1.
            for j in 0..=nf {
                let (lo, hi) = if j == 0 {
                    (a.x, gx(0))
                } else if j == nf {
                    (gx(nf - 1) + r.gate_w, a.x + a.w)
                } else {
                    (gx(j - 1) + r.gate_w, gx(j))
                };
                let cx = (lo + hi) / 2;
                b.rect(
                    sdt,
                    Rect {
                        x: cx - r.sdt_w / 2,
                        y: 0,
                        w: r.sdt_w,
                        h,
                    },
                );
                let lw = r.lisd_w.max(r.v0 + 2 * r.v0_in_lisd);
                b.rect(
                    lisd,
                    Rect {
                        x: cx - lw / 2,
                        y: 0,
                        w: lw,
                        h,
                    },
                );
                let via = Rect {
                    x: cx - r.v0 / 2,
                    y: v0_y,
                    w: r.v0,
                    h: r.v0,
                };
                b.rect(v0, via);
                if j % 2 == 0 {
                    // A source strip runs up to the source strap.
                    b.rect(
                        m1,
                        Rect {
                            x: cx - r.m1_w / 2,
                            y: sd_m1_y,
                            w: r.m1_w,
                            h: s_strap_y + r.m1_w - sd_m1_y,
                        },
                    );
                    s_x.push(cx);
                } else {
                    b.rect(
                        m1,
                        Rect {
                            x: cx - r.m1_w / 2,
                            y: sd_m1_y,
                            w: r.m1_w,
                            h: m1_h,
                        },
                    );
                    let c = (cx, v0_y + r.v0 / 2);
                    b.rect(
                        m1,
                        Rect {
                            x: c.0 - v1_pad / 2,
                            y: c.1 - v1_pad / 2,
                            w: v1_pad,
                            h: v1_pad,
                        },
                    );
                    b.rect(
                        v1,
                        Rect {
                            x: c.0 - v1_w / 2,
                            y: c.1 - v1_w / 2,
                            w: v1_w,
                            h: v1_w,
                        },
                    );
                    d_x.push(cx);
                }
            }
            let (s0, s1) = (s_x[0], *s_x.last().unwrap());
            let strap = Rect {
                x: s0 - r.m1_w / 2,
                y: s_strap_y,
                w: s1 - s0 + r.m1_w,
                h: r.m1_w,
            };
            b.rect(m1, strap);
            b.pin(pin(
                di,
                "S",
                Rect {
                    x: s0 - r.m1_w / 2,
                    y: s_strap_y,
                    w: r.m1_w,
                    h: r.m1_w,
                },
                m1,
            ));
            let (d0, d1) = (d_x[0], *d_x.last().unwrap());
            let dy = v0_y + r.v0 / 2 - m2_w / 2;
            b.rect(
                m2,
                Rect {
                    x: d0 - v1_w / 2 - v1_in_m2,
                    y: dy,
                    w: d1 - d0 + v1_w + 2 * v1_in_m2,
                    h: m2_w,
                },
            );
            b.pin(pin(
                di,
                "D",
                Rect {
                    x: d0 - v1_w / 2,
                    y: v0_y + r.v0 / 2 - v1_w / 2,
                    w: v1_w,
                    h: v1_w,
                },
                m2,
            ));
            s_x.clear();
            d_x.clear();
            for i in 0..nf {
                let g = gx(i);
                b.rect(
                    gate,
                    Rect {
                        x: g,
                        y: gate_lo,
                        w: r.gate_w,
                        h: gate_hi - gate_lo,
                    },
                );
                let lw = r.lig_w.max(r.gate_w);
                b.rect(
                    lig,
                    Rect {
                        x: g + r.gate_w / 2 - lw / 2,
                        y: lig_y,
                        w: lw,
                        h: lig_h,
                    },
                );
                b.rect(
                    v0,
                    Rect {
                        x: g + r.gate_w / 2 - r.v0 / 2,
                        y: gv0_y,
                        w: r.v0,
                        h: r.v0,
                    },
                );
                b.unit(pnr_core::Unit {
                    owner: di as u8,
                    x: g + r.gate_w / 2,
                    y: h / 2,
                    weight: i64::from(r.gate_w) * i64::from(h),
                    phi: (if i % 2 == 0 { 1 } else { -1 }, 0),
                    sa: 0,
                    sb: 0,
                });
            }
            // One M1 strap over the device's gate contacts.
            let (g0, g1) = (
                gx(0) + r.gate_w / 2 - r.v0 / 2,
                gx(nf - 1) + r.gate_w / 2 + r.v0 / 2,
            );
            let strap = Rect {
                x: g0 - r.v0_in_m1,
                y: strap_top - r.m1_w,
                w: g1 - g0 + 2 * r.v0_in_m1,
                h: r.m1_w,
            };
            b.rect(m1, strap);
            b.pin(pin(
                di,
                "G",
                Rect {
                    x: g0,
                    y: gv0_y,
                    w: r.v0,
                    h: r.v0,
                },
                m1,
            ));
            x = a.x + a.w + r.act_s.max(2 * r.sel_enc - 0);
        }
        let (x0, x1) = (acts[0].x, x - r.act_s.max(2 * r.sel_enc));
        // The device select over the actives and gates.
        let sel = Rect {
            x: x0 - r.sel_enc,
            y: (-r.sel_enc).min(gate_lo - r.sel_past_gate),
            w: x1 - x0 + 2 * r.sel_enc,
            h: 0,
        };
        let sel_top = (h + r.sel_enc).max(gate_hi + r.sel_past_gate);
        let sel = Rect {
            h: (sel_top - sel.y).max(r.sel_w),
            ..sel
        };
        b.rect(own_sel, sel);

        // Tap strip above: its own active and fins under the other select,
        // clear of the device select and of the gates' field ends.
        let tap_h = r.fin_p.max(r.lisd_w);
        // Its rail (at the tap's centre) a spacing above the source strap.
        let tap_y = (sel.y + sel.h + r.sel_enc)
            .max(gate_hi + r.gate_field)
            .max(h + r.act_s)
            .max(s_strap_y + r.m1_w + r.m1_s - (tap_h - r.m1_w) / 2);
        let tap = Rect {
            x: x0,
            y: tap_y,
            w: x1 - x0,
            h: tap_h,
        };
        b.rect(act, tap);
        b.rect(
            fin,
            Rect {
                x: tap.x + r.act_past_fin,
                y: tap_y + (tap_h - r.fin_w) / 2,
                w: tap.w - 2 * r.act_past_fin,
                h: r.fin_w,
            },
        );
        let tsel = Rect {
            x: tap.x - r.sel_enc,
            y: tap_y - r.sel_enc,
            w: tap.w + 2 * r.sel_enc,
            h: (tap_h + 2 * r.sel_enc).max(r.sel_w),
        };
        b.rect(tap_sel, tsel);
        let tv0_y = tap_y + (tap_h - r.v0) / 2;
        let lw = r.lisd_w.max(r.v0 + 2 * r.v0_in_lisd);
        let mut cx = tap.x + end / 2;
        let mut first = None;
        while cx + lw / 2 <= tap.x + tap.w - r.sdt_gate {
            b.rect(
                sdt,
                Rect {
                    x: cx - r.sdt_w / 2,
                    y: tap_y,
                    w: r.sdt_w,
                    h: tap_h,
                },
            );
            b.rect(
                lisd,
                Rect {
                    x: cx - lw / 2,
                    y: tap_y,
                    w: lw,
                    h: tap_h,
                },
            );
            b.rect(
                v0,
                Rect {
                    x: cx - r.v0 / 2,
                    y: tv0_y,
                    w: r.v0,
                    h: r.v0,
                },
            );
            first.get_or_insert(cx);
            cx += r.gate_p;
        }
        let rail = Rect {
            x: tap.x,
            y: tv0_y + r.v0 / 2 - r.m1_w / 2,
            w: tap.w,
            h: r.m1_w,
        };
        b.rect(m1, rail);
        let at = Rect {
            x: first.unwrap_or(tap.x) - r.v0 / 2,
            y: tv0_y,
            w: r.v0,
            h: r.v0,
        };
        for di in 0..n_dev {
            b.pin(pin(di, "B", at, m1));
        }
        if pmos {
            if let Some(well) = process.layer("nwell") {
                let top = tsel.y + tsel.h;
                b.rect(
                    well,
                    Rect {
                        x: x0 - r.well_enc,
                        y: sel.y.min(-r.well_enc),
                        w: x1 - x0 + 2 * r.well_enc,
                        h: top.max(tap_y + tap_h + r.well_enc) - sel.y.min(-r.well_enc),
                    },
                );
            }
        }
        b.finish()
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let fin_p = process.width("fin").unwrap_or(0) + process.space("fin").unwrap_or(0);
    sizing(group, c, fin_p.max(1), dim(process, "min_gate_l"))
}
