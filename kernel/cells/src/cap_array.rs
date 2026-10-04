//! Binary-weighted capacitor-DAC array, after Karmokar et al., "Constructive
//! Placement and Routing for Common-Centroid Capacitor Arrays in Binary-Weighted
//! and Split DACs", IEEE TCAD 42(9), 2023 (DACP below).
//!
//! A group whose unit counts are `[1, 1, 2, 4, …, 2^(N-1)]` (slot 0 the
//! electrical dummy C0, slot `i` the bit capacitor Ci, DACP §II) is drawn as
//! `2^N` unit plates in a point-symmetric pattern, ringed by dummies tied to
//! C0's bottom plate (Hastings 3e §7.1, capacitor matching), with every
//! bottom plate routed inside the macro. Separate from [`crate::capacitor`],
//! which draws one merged plate per device and implements no pattern.
//!
//! Stack: BOT on `met1`, TOP on `met2` (inset), TOP strapped on `met3` per
//! column and joined *above* the array; bottom plates reach per-bit `met1`
//! tracks in the channel right of each column over `met2` branches, and the
//! tracks drop to per-bit `met2` buses *below* the array. TOP metal never
//! crosses a bottom-plate route, so the bit-route-to-top coupling DACP §IV-B
//! calls `C_TB` is structurally absent (ARR-03).
//!
//! A capacitor recipe with a `plate` role (sky130 `mim_m3_1`) draws the deck's
//! MIM instead (CELL-08): per unit a `bottom` plate (met3) enclosing the
//! `plate` (capm, the netlist's W×L) by the deck's enclosure, a `top_contact`
//! array (via3) on it, a `top` strap (met4) per column joined in the gap
//! under the top dummy row, and the bottom stub dropping through
//! `bottom_contact` (via2) onto the same met2 branch. Ring dummies carry an
//! uncontacted capm (no met4 over it, so extraction sees no device). The
//! MIM ring is drawn only when `dummy_required`: without it a lone MIM
//! (tq_chain's) is one plate, not the centre of nine.

use crate::builder::dim;

use analog::matching::pattern::{self, Fill, Grid};
use analog::Constraints;
use pnr_core::{DeviceGroup, Drawn, DrawnKind, KeepWhy, Macro, Node, Process, Rect, Unit};

use crate::builder::{cut_lattice, pin, req, sizing, Builder, Sizing};
use crate::Cell;

/// Unit placement family (DACP §IV-A, Fig. 6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// C0/C1 diagonally opposite at the centre, then each bit along a spiral
    /// outward, every unit with its point reflection. Most same-bit adjacency,
    /// least dispersion.
    Spiral,
    /// After DACP ref. [8]: the MSB on the black squares, each lower bit spread
    /// over what is left. Most dispersion, most branches.
    Chessboard,
    /// Algorithm 1: a complete chessboard for C0..=Ck, then corridors of Ci in
    /// `bs`-sided blocks with Ci+1 in the leftovers.
    BlockChessboard { k: u8, bs: u8 },
}

/// One array variant.
///
/// Pins: `d{i}:P` (all on the TOP join, one net), `d{i}:N` on bus `i`.
#[derive(Clone, Debug)]
pub struct CapArray {
    pub pattern: Pattern,
    /// Rows > columns (odd N only; even N is square).
    pub tall: bool,
}

/// Per-variant figures of merit (ARR-02/03).
#[derive(Clone, Debug, Default)]
pub struct ArrayMetrics {
    /// Worst slot's unit-centroid offset from the array centre, µm. Nonzero only
    /// through the one-unit C0/C1, which cannot be common-centroid; a linear
    /// gradient `g`/µm mis-sizes that slot by `g ×` this.
    pub lin_um: f64,
    /// Worst slot's `|⟨r²⟩ − ⟨r²⟩_array|`, µm²: its error per unit coefficient
    /// of a bowl-shaped (quadratic) gradient.
    pub quad_um2: f64,
    /// Worst `|INL|` / `|DNL|`, LSB (DACP Eqs. 17–18, ideal-referenced, all
    /// codes), in the field `ε = g·(x cosθ + y sinθ) + q·r²`, worst over θ in
    /// 45° steps.
    pub inl_lsb: f64,
    pub dnl_lsb: f64,
    /// Bottom-plate route length per unit, `max/min − 1` over C1..=CN: 0 when
    /// every bit carries the same wire per unit of capacitance.
    pub route_spread: f64,
    /// `via1` cuts on each slot's bottom-plate route.
    pub vias: Vec<u32>,
    pub area_um2: f64,
}

/// Largest bank: 256 units.
const MAX_BITS: u8 = 8;

impl Cell for CapArray {
    fn enumerate(group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Vec<Self> {
        let Some(st) = plate_stack(process) else { return vec![] };
        let dev_nf = group_sizing(group, constraints, process).dev_nf;
        let Some(n) = bits(&dev_nf) else {
            // Any other matched set (≥ 2 devices, ≤ 256 units): the same array
            // with a centrosymmetric general assignment, compact or dispersed.
            // A MIM is drawn only here, so a single one is a set of one.
            let total: u32 = dev_nf.iter().map(|&u| u32::from(u)).sum();
            if (dev_nf.len() < 2 && st.plate.is_none()) || total > 1 << MAX_BITS {
                return vec![];
            }
            return [Pattern::Spiral, Pattern::Chessboard].into_iter().map(|pattern| CapArray { pattern, tall: false }).collect();
        };
        let mut patterns = vec![Pattern::Spiral, Pattern::Chessboard];
        // Corridors come in (Ci, Ci+1) pairs and each level must nest centred,
        // so N − k is even and the core has at least 2×2 units.
        for k in (2..n).filter(|k| (n - k) % 2 == 0) {
            let (r, c) = dims(k, false);
            for bs in (1..=r.min(c) / 2).take(2) {
                patterns.push(Pattern::BlockChessboard { k, bs: bs as u8 });
            }
        }
        let talls: &[bool] = if n % 2 == 1 { &[false, true] } else { &[false] };
        let mut out: Vec<(Self, Vec<u8>)> = Vec::new();
        for &tall in talls {
            for &pattern in &patterns {
                let v = CapArray { pattern, tall };
                let a = v.assign(n);
                // Two block sizes can land on the same assignment.
                if !out.iter().any(|(o, oa)| o.tall == tall && *oa == a) {
                    out.push((v, a));
                }
            }
        }
        out.into_iter().map(|(v, _)| v).collect()
    }

    fn draw(&self, group: &DeviceGroup, constraints: &Constraints, process: &dyn Process) -> Macro {
        self.build(group, constraints, process).0
    }
}

/// `N` when `dev_nf` is exactly `[1, 1, 2, …, 2^(N-1)]`, `N ≥ 2`.
#[must_use]
pub fn bits(dev_nf: &[u16]) -> Option<u8> {
    let n = dev_nf.len().checked_sub(1)?;
    let want = |i: usize| if i == 0 { 1 } else { 1u16 << (i - 1) };
    (2..=usize::from(MAX_BITS)).contains(&n).then_some(())?;
    dev_nf.iter().enumerate().all(|(i, &u)| u == want(i)).then_some(n as u8)
}

/// The plate stack by role names `process` resolves. A capacitor recipe names
/// `bottom`, `plate` (MIM only), `top_contact`, `top`, `strap` and
/// `bottom_contact`; a role it leaves unset falls back to today's MOM layer
/// (met1 BOT, met2 TOP, via2 centre cut, met3 strap, via1 off the stub), so the
/// base deck and a MOM recipe draw the same array. `None` when a layer is missing.
struct PlateStack {
    bot: &'static str,
    plate: Option<&'static str>,
    top: &'static str,
    top_cut: &'static str,
    strap: &'static str,
    bot_cut: &'static str,
}

fn plate_stack(p: &dyn Process) -> Option<PlateStack> {
    let role = |r: &'static str, mom: &'static str| if p.layer(r).is_some() { r } else { mom };
    let s = PlateStack {
        bot: role("bottom", "met1"),
        plate: p.layer("plate").map(|_| "plate"),
        top: role("top", "met2"),
        top_cut: role("top_contact", "via2"),
        strap: role("strap", "met3"),
        bot_cut: role("bottom_contact", "via1"),
    };
    [s.bot, s.top, s.top_cut, s.strap, s.bot_cut, "met1", "met2", "met3", "via1", "via2"].iter().all(|r| p.layer(r).is_some()).then_some(s)
}

/// One `w_nm`×`l_nm` MIM unit's capacitance, aF, from the recipe's model keys:
/// `c_area_af_um2·(W+dw)(L+dw) + c_perim_af_um·2(W+L+2dw)`, `dw` = `c_dw_nm`
/// (sky130 `cap_mim_m3_1`: camimc, cpmimc, m3_dw). `None` without
/// `c_area_af_um2` (a MOM recipe: its C is unknown, AV-32).
#[must_use]
pub fn c_u_af(p: &dyn Process, w_nm: i32, l_nm: i32) -> Option<f64> {
    let area = p.rule("c_area_af_um2", 0);
    let dw = f64::from(p.rule("c_dw_nm", 0));
    let (w, l) = ((f64::from(w_nm) + dw) * 1e-3, (f64::from(l_nm) + dw) * 1e-3);
    (area > 0).then(|| f64::from(area) * w * l + f64::from(p.rule("c_perim_af_um", 0)) * 2.0 * (w + l))
}

/// Rows × columns holding `2^m` units, columns ≥ rows unless `tall`.
fn dims(m: u8, tall: bool) -> (usize, usize) {
    let (r, c) = (1usize << (m / 2), 1usize << (m - m / 2));
    if tall { (c, r) } else { (r, c) }
}

impl CapArray {
    /// Slot per unit, row-major over the `2^n` interior cells.
    #[must_use]
    pub fn assign(&self, n: u8) -> Vec<u8> {
        let (rows, cols) = dims(n, self.tall);
        let mut g = Grid { cols, slot: vec![None; rows * cols] };
        let centred = |(r, c): (usize, usize)| ((rows - r) / 2, (cols - c) / 2, r, c);
        match self.pattern {
            Pattern::Spiral => {
                // Doubled offsets from the centre; the ring is Chebyshev scaled
                // to the array's aspect, the angle orders each ring.
                let key = |i: usize| {
                    let (dr, dc) = (2 * (i / cols) as i64 - rows as i64 + 1, 2 * (i % cols) as i64 - cols as i64 + 1);
                    ((dr.abs() * cols as i64).max(dc.abs() * rows as i64), (dr as f64).atan2(dc as f64))
                };
                let mut order: Vec<usize> = (0..rows * cols).collect();
                order.sort_by(|&a, &b| {
                    let (ka, kb) = (key(a), key(b));
                    ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
                });
                g.slot[order[0]] = Some(0);
                let j = g.refl(order[0]);
                g.slot[j] = Some(1);
                for b in 2..=n {
                    let mut left = 1usize << (b - 1);
                    for &i in &order {
                        if left > 0 && g.free_pair(i) {
                            g.put_pair(i, b);
                            left -= 2;
                        }
                    }
                }
            }
            Pattern::Chessboard => g.chessboard((0, 0, rows, cols), n),
            Pattern::BlockChessboard { k, bs } => {
                g.chessboard(centred(dims(k, self.tall)), k);
                for i in (k + 1..n).step_by(2) {
                    g.corridor(centred(dims(i + 1, self.tall)), centred(dims(i - 1, self.tall)), i, usize::from(bs));
                }
            }
        }
        g.slot.into_iter().map(|s| s.expect("every cell assigned")).collect()
    }

    /// ARR-02/03 figures for this variant under gradient `g` (1/µm) and
    /// curvature `q` (1/µm²). No deck carries these, so they are the caller's
    /// sweep points, not constants.
    #[must_use]
    pub fn metrics(&self, group: &DeviceGroup, c: &Constraints, process: &dyn Process, g: f64, q: f64) -> ArrayMetrics {
        let (m, routes) = self.build(group, c, process);
        let n = routes.len() - 1;
        let um = |v: i32| f64::from(v) * 1e-3;
        let (cx, cy) = {
            let k = m.units.len() as f64;
            (m.units.iter().map(|u| um(u.x)).sum::<f64>() / k, m.units.iter().map(|u| um(u.y)).sum::<f64>() / k)
        };
        let pos: Vec<(usize, f64, f64)> = m.units.iter().map(|u| (usize::from(u.owner), um(u.x) - cx, um(u.y) - cy)).collect();
        let r2 = |x: f64, y: f64| x * x + y * y;
        let mean_r2 = pos.iter().map(|&(_, x, y)| r2(x, y)).sum::<f64>() / pos.len() as f64;
        let mut out = ArrayMetrics { vias: routes.iter().map(|r| r.1).collect(), ..Default::default() };
        for s in 0..=n {
            let mine: Vec<(f64, f64)> = pos.iter().filter(|p| p.0 == s).map(|p| (p.1, p.2)).collect();
            let k = mine.len() as f64;
            let (mx, my) = (mine.iter().map(|p| p.0).sum::<f64>() / k, mine.iter().map(|p| p.1).sum::<f64>() / k);
            out.lin_um = out.lin_um.max(r2(mx, my).sqrt());
            out.quad_um2 = out.quad_um2.max((mine.iter().map(|p| r2(p.0, p.1)).sum::<f64>() / k - mean_r2).abs());
        }
        for step in 0..4 {
            let th = f64::from(step) * std::f64::consts::FRAC_PI_4;
            let mut cap = vec![0.0; n + 1];
            for &(s, x, y) in &pos {
                cap[s] += 1.0 + g * (x * th.cos() + y * th.sin()) + q * r2(x, y);
            }
            let total: f64 = cap.iter().sum();
            let codes = 1usize << n;
            let t = |k: usize| (1..=n).filter(|b| k >> (b - 1) & 1 == 1).map(|b| cap[b]).sum::<f64>() / total * codes as f64;
            for k in 0..codes {
                out.inl_lsb = out.inl_lsb.max((t(k) - k as f64).abs());
                if k + 1 < codes {
                    out.dnl_lsb = out.dnl_lsb.max((t(k + 1) - t(k) - 1.0).abs());
                }
            }
        }
        let per_unit: Vec<f64> = (1..=n).map(|s| routes[s].0 as f64 / f64::from(1u32 << (s - 1))).collect();
        let (lo, hi) = per_unit.iter().fold((f64::MAX, 0.0f64), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        out.route_spread = hi / lo - 1.0;
        out.area_um2 = um(m.bbox.w) * um(m.bbox.h);
        out
    }

    /// One MIM unit at `cell` (its bottom plate): the plate inset by `encp`; on
    /// a member's unit a centred `top_contact` array inset by the deck's plate
    /// enclosure (capm.4) at its cut pitch, the LVS card and the plate
    /// keep-out. A ring dummy keeps its plate, uncontacted; an interior
    /// empty cell draws none, as the column strap would make it a device.
    #[allow(clippy::too_many_arguments)]
    fn mim_unit(b: &mut Builder, process: &dyn Process, st: &PlateStack, cell: Rect, encp: i32, slot: Option<u8>, ring: bool) {
        let plate_role = st.plate.expect("MIM");
        b.rect(req(process, st.bot), cell);
        let plate = Rect { x: cell.x + encp, y: cell.y + encp, w: cell.w - 2 * encp, h: cell.h - 2 * encp };
        if slot.is_none() && !ring {
            return;
        }
        b.rect(req(process, plate_role), plate);
        let Some(owner) = slot else { return };
        let lat = cut_lattice(process);
        let v = process.width(st.top_cut).unwrap_or(lat);
        let pitch = v + process.space(st.top_cut).unwrap_or(v);
        let e = process.enclosure(plate_role, st.top_cut).unwrap_or(0);
        let n = |side: i32| ((side - 2 * e + pitch - v) / pitch).max(1);
        let (nx, ny) = (n(plate.w), n(plate.h));
        let x0 = plate.x + (plate.w - nx * pitch + pitch - v) / 2 / lat * lat;
        let y0 = plate.y + (plate.h - ny * pitch + pitch - v) / 2 / lat * lat;
        let cut = req(process, st.top_cut);
        for i in 0..nx {
            for j in 0..ny {
                b.rect(cut, Rect { x: x0 + i * pitch, y: y0 + j * pitch, w: v, h: v });
            }
        }
        b.unit(Unit { owner, x: cell.x + cell.w / 2, y: cell.y + cell.h / 2, weight: i64::from(cell.w) * i64::from(cell.h), phi: (0, 0), sa: 0, sb: 0 });
        b.drawn(Drawn { owner, device: None, kind: DrawnKind::Capacitor, nodes: [Node::Pin("P"), Node::Pin("N"), Node::Unused], w: plate.w, l: plate.h });
        b.keepout(plate, KeepWhy::CapPlate { owner });
    }

    /// The macro, plus each slot's `(bottom-route length nm, via1 cuts)`.
    fn build(&self, group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> (Macro, Vec<(i64, u32)>) {
        let s = group_sizing(group, c, process);
        // A binary bank, or a general set (see `enumerate`). A bank's dummies
        // join C0 (its electrical dummy, on a rail); a general set's get a bus
        // of their own, slot `n + 1`, pinned `GND` for the caller to tie to
        // ground: tied to a member they would load its bottom plate alone.
        let general = bits(&s.dev_nf).is_none();
        let (n, rows, cols, slots) = match bits(&s.dev_nf) {
            Some(n) => {
                let (rows, cols) = dims(n, self.tall);
                (n, rows, cols, self.assign(n).into_iter().map(Some).collect::<Vec<_>>())
            }
            None => {
                let (rows, cols) = pattern::grids(&s.dev_nf, 3.0)[0];
                let fill = if self.pattern == Pattern::Chessboard { Fill::Dispersed } else { Fill::Compact };
                let slots = pattern::centro_assign(&s.dev_nf, rows, cols, fill).0;
                ((s.dev_nf.len() - 1) as u8, rows, cols, slots)
            }
        };
        let st = plate_stack(process).expect("enumerate offers variants only on a plate stack");
        let mim = st.plate.is_some();
        let [m1, m2, m3, v1l, v2l] = ["met1", "met2", "met3", "via1", "via2"].map(|l| req(process, l));
        let lat = cut_lattice(process);
        let up = |v: i32| (v + lat - 1).div_euclid(lat) * lat;
        let floor = |v: i32| v.div_euclid(lat) * lat;
        let r = |name: &str, d: i32| process.rule(name, d);
        // Every number is the deck's, by role (`Process`), so the array draws
        // on any stack: spacing includes wide-metal spacing (plates are wide)
        // and end-of-line spacing (stubs and strap ends are line ends).
        let space = |m: &str| up(process.space(m).max(process.eol_space(m)).unwrap_or(0));
        let enc = |outer: &str, inner: &str| process.enclosure(outer, inner).unwrap_or(0);
        let cap = |outer: &str, inner: &str| process.endcap(outer, inner).unwrap_or(0);
        let wmin = |m: &str| process.width(m).unwrap_or(0);

        let v1 = process.width("via1").unwrap_or(dim(process, "contact"));
        let v2 = process.width("via2").unwrap_or(v1);
        // The deck's end-cap holds on both ends of one axis, so tracks and
        // buses enclose their vias by it on every side: symmetric.
        let e1o = up(cap("met1", "via1").max(cap("met2", "via1")).max(enc("met1", "via1")).max(enc("met2", "via1")));
        let e1 = e1o;
        let e3 = up(enc("met3", "via2").max(enc("met2", "via2")));
        let (m1s, m2s) = (space("met1"), space("met2"));
        let inset = up(r("plate_spacing", 0));
        // A unit is never smaller than its inset top plate can be drawn: the
        // plate holds the centre via2 and meets met2's width and area.
        let area_side = (process.area("met2").unwrap_or(0) as f64).sqrt().ceil() as i32;
        let min_unit = up(2 * inset + (v2 + 2 * e3).max(wmin("met2")).max(area_side));
        // MIM: the plate is the netlist's W×L, the bottom plate encloses it.
        let encp = st.plate.map_or(0, |pl| up(enc(st.bot, pl)));
        let (uw, uh) = if mim {
            let side = |v: i32| up(v).max(wmin("plate")) + 2 * encp;
            (side(s.unit_w), side(s.unit_l))
        } else {
            (up(s.unit_w).max(min_unit), up(s.unit_l).max(min_unit))
        };
        // MIM top: the via3 (`top_contact`) cut, and the met4 join's width,
        // centred on its cut like the met3 one.
        let vt = process.width(st.top_cut).unwrap_or(v2);
        let e4 = up(enc(st.top, st.top_cut).max(cap(st.top, st.top_cut)));
        let jw = ((vt + 2 * e4).max(wmin(st.top)) + 2 * lat - 1) / (2 * lat) * 2 * lat;
        // Track, branch and bus width: one via1, `e1` below/left and `e1o`
        // above/right, so the along-axis long side and this one together meet
        // the asymmetric rule (best side of *each* axis); never under either
        // metal's width.
        let w1 = up((e1 + v1 + e1o).max(wmin("met1")).max(wmin("met2")));
        // Pins sit on met1, the router's landing layer (it lands a met2 pin with
        // no via up to it), at the end of a met1 lead off the via1 at `(x, y)`:
        // the router drops its own via near the pin, clear of this one.
        let pp = up((v1 + 2 * e1o).max(wmin("met1")));
        let reach = pp + space("via1") + w1 + m1s;
        let lead = |b: &mut Builder, x: i32, y: i32| {
            b.rect(m1, Rect { x: x - e1o, y: y - e1o, w: reach + pp, h: pp });
            Rect { x: x - e1o + reach, y: y - e1o, w: pp, h: pp }
        };
        // Even multiple of the lattice so the strap centres on the plate.
        let e3o = up(cap("met3", "via2")).max(e3);
        // Centred on its vias, so the end-cap holds on both sides.
        let m3w = ((v2 + 2 * e3o).max(wmin("met3")) + 2 * lat - 1) / (2 * lat) * 2 * lat;
        // An isolated met2 piece (a branch, a via pad) grows across its run,
        // about its centre, to the deck's min area.
        let m2_area = process.area("met2").unwrap_or(0);
        let tall = |r: Rect| {
            let h = up(i32::try_from((m2_area + i64::from(r.w) - 1) / i64::from(r.w.max(1))).unwrap_or(i32::MAX)).max(r.h);
            Rect { y: r.y - floor((h - r.h) / 2), h, ..r }
        };
        // MIM rows also clear capm spacing between plates (capm.2a) and hold
        // the met4 join, which then stays `encp` off the dummy capm above.
        let gap_y = if mim {
            let capm_gap = up(process.space("plate").unwrap_or(0) - 2 * encp);
            inset.max(m1s).max(m2s).max(capm_gap).max(space(st.bot)).max(jw)
        } else {
            inset.max(m1s).max(m2s)
        };

        // Channel right of each column, x from the plate's right edge: the
        // branch-start via clears TOP's met2 by `m2s`, then the tracks.
        let vx = up((m2s + e1 - inset).max(0));
        let t0 = vx + v1 + e1o + m1s;
        let tp = w1 + m1s;
        // Ring offset: 1 with the dummy ring, 0 for a MIM set without
        // `dummy_required` (a MOM bank's ring is C0's, always drawn).
        let o = usize::from(!mim || crate::builder::unitization(group, c).is_some_and(|u| u.dummy_required));
        let (gr, gc) = (rows + 2 * o, cols + 2 * o);
        let dummy_slot = if general { n + 1 } else { 0 };
        let owner = |r: usize, c: usize| -> Option<u8> {
            if r >= o && c >= o && r < rows + o && c < cols + o { slots[(r - o) * cols + c - o] } else { None }
        };
        // Per column, the slots with a unit there (dummies are slot 0), in slot order.
        let tracks: Vec<Vec<u8>> = (0..gc)
            .map(|c| {
                let mut t: Vec<u8> = (0..gr).map(|r| owner(r, c).unwrap_or(dummy_slot)).collect();
                t.sort_unstable();
                t.dedup();
                t
            })
            .collect();
        let tmax = tracks.iter().map(Vec::len).max().unwrap_or(1) as i32;
        let branch_end = t0 + (tmax - 1) * tp + e1 + v1 + e1o;
        let ch_w = up((t0 + tmax * tp).max(branch_end + m2s - inset));
        let (px, py) = (uw + ch_w, uh + gap_y);
        let track_x = |c: usize, s: u8| {
            let t = tracks[c].iter().position(|&x| x == s).expect("slot has a track") as i32;
            c as i32 * px + uw + t0 + t * tp
        };
        // Bus `s` below the array: row 0's TOP plates start at `inset`. A bus-wide
        // gap more than min spacing leaves the router room for its landing pads.
        let bus_y = |s: u8| inset - m2s - (i32::from(s) + 1) * w1 - i32::from(s) * (w1 + 2 * m2s);
        // Pins stand one track pitch clear of the last channel.
        let x_right = gc as i32 * px + tp;

        let mut b = Builder::new(process.grid());
        let mut route = vec![(0i64, 0u32); usize::from(dummy_slot.max(n)) + 1];
        // Per (column, slot): the highest via on its track.
        let mut top_of: Vec<Vec<i32>> = tracks.iter().map(|t| vec![i32::MIN; t.len()]).collect();
        for r in 0..gr {
            for c in 0..gc {
                let (x0, y0) = (c as i32 * px, r as i32 * py);
                let slot = owner(r, c);
                let s = slot.unwrap_or(dummy_slot);
                if mim {
                    Self::mim_unit(&mut b, process, &st, Rect { x: x0, y: y0, w: uw, h: uh }, encp, slot, o == 1 && (r == 0 || c == 0 || r == gr - 1 || c == gc - 1));
                    let tx = track_x(c, s);
                    let (vy, vb) = (y0 + floor((uh - v1) / 2), process.width(st.bot_cut).unwrap_or(v2));
                    let vyb = y0 + floor((uh - vb) / 2);
                    let eb3 = up(enc(st.bot, st.bot_cut).max(cap(st.bot, st.bot_cut)));
                    let eb2 = up(enc("met2", st.bot_cut).max(cap("met2", st.bot_cut)));
                    // Stub on the bottom plate's metal, its cut onto the branch.
                    let hb = up((vb + 2 * eb3).max(wmin(st.bot)));
                    b.rect(req(process, st.bot), Rect { x: x0 + uw - 2 * lat, y: vyb + vb / 2 - hb / 2, w: 2 * lat + vx + vb + eb3, h: hb });
                    b.rect(req(process, st.bot_cut), Rect { x: x0 + uw + vx, y: vyb, w: vb, h: vb });
                    let bx = x0 + uw + vx - eb2;
                    let (by0, by1) = ((vy - e1).min(vyb - eb2), (vy - e1 + w1).max(vyb + vb + eb2));
                    let bw = tx + e1 + v1 + e1o - bx;
                    b.rect(m2, tall(Rect { x: bx, y: by0, w: bw, h: by1 - by0 }));
                    b.rect(v1l, Rect { x: tx + e1, y: vy, w: v1, h: v1 });
                    let t = tracks[c].iter().position(|&x| x == s).unwrap();
                    top_of[c][t] = top_of[c][t].max(vy + v1 + e1o);
                    route[usize::from(s)].0 += i64::from(bw);
                    route[usize::from(s)].1 += 2;
                    continue;
                }
                b.rect(m1, Rect { x: x0, y: y0, w: uw, h: uh });
                b.rect(m2, Rect { x: x0 + inset, y: y0 + inset, w: uw - 2 * inset, h: uh - 2 * inset });
                let (cx, cy) = (x0 + floor((uw - v2) / 2), y0 + floor((uh - v2) / 2));
                if slot.is_some() {
                    b.rect(v2l, Rect { x: cx, y: cy, w: v2, h: v2 });
                    b.unit(Unit { owner: s, x: x0 + uw / 2, y: y0 + uh / 2, weight: i64::from(uw) * i64::from(uh), phi: (0, 0), sa: 0, sb: 0 });
                } else {
                    // A dummy's plates are shorted: it is environment, not C.
                    b.rect(v1l, Rect { x: cx, y: cy, w: v1, h: v1 });
                }
                // Branch: met1 stub off the plate, via1, met2 over the other
                // tracks, via1 onto this slot's track.
                let vy = y0 + floor((uh - v1) / 2);
                let tx = track_x(c, s);
                b.rect(m1, Rect { x: x0 + uw - 2 * lat, y: vy - e1, w: 2 * lat + vx + v1 + e1o, h: w1 });
                b.rect(v1l, Rect { x: x0 + uw + vx, y: vy, w: v1, h: v1 });
                let bx = x0 + uw + vx - e1;
                let bw = tx + e1 + v1 + e1o - bx;
                b.rect(m2, tall(Rect { x: bx, y: vy - e1, w: bw, h: w1 }));
                b.rect(v1l, Rect { x: tx + e1, y: vy, w: v1, h: v1 });
                let t = tracks[c].iter().position(|&x| x == s).unwrap();
                top_of[c][t] = top_of[c][t].max(vy + v1 + e1o);
                route[usize::from(s)].0 += i64::from(bw);
                route[usize::from(s)].1 += 2;
            }
        }
        for (c, ts) in tracks.iter().enumerate() {
            for (t, &s) in ts.iter().enumerate() {
                let (tx, by) = (track_x(c, s), bus_y(s));
                b.rect(m1, Rect { x: tx, y: by, w: w1, h: top_of[c][t] - by });
                b.rect(v1l, Rect { x: tx + e1, y: by + e1, w: v1, h: v1 });
                route[usize::from(s)].0 += i64::from(top_of[c][t] - by);
                route[usize::from(s)].1 += 1;
            }
        }
        // The via2 under the met1/via1 pin stack: TOP's (MOM: off the met3
        // strap; MIM: off the met3 island) and, MIM, the dummy tie's.
        let (dcut, vd) = if mim { (st.bot_cut, process.width(st.bot_cut).unwrap_or(v2)) } else { ("via2", v2) };
        let e2 = up(enc("met2", dcut).max(cap("met2", dcut)));
        // MIM hop between met4 and met2, centred on (cx, cy): via3, a met3
        // island grown to the deck's area (off capm, capm.11), via2 under it.
        let ei = up(enc(st.bot, st.top_cut).max(cap(st.bot, st.top_cut)));
        let eb3 = up(enc(st.bot, st.bot_cut).max(cap(st.bot, st.bot_cut)));
        let side = up((vt + 2 * ei).max(vd + 2 * eb3).max(wmin(st.bot)));
        let island = |b: &mut Builder, cx: i32, cy: i32| {
            b.rect(req(process, st.top_cut), Rect { x: cx - vt / 2, y: cy - vt / 2, w: vt, h: vt });
            let h = up(i32::try_from((process.area(st.bot).unwrap_or(0) + i64::from(side) - 1) / i64::from(side)).unwrap_or(i32::MAX)).max(side);
            b.rect(req(process, st.bot), Rect { x: cx - side / 2, y: cy - side / 2 - floor((h - side) / 2), w: side, h });
            b.rect(req(process, dcut), Rect { x: cx - vd / 2, y: cy - vd / 2, w: vd, h: vd });
        };
        for s in 0..=dummy_slot.max(n) {
            // Only a ringless set's dummy slot can be empty (no hole, no ring).
            let Some(x0) = (0..gc).filter(|&c| tracks[c].contains(&s)).map(|c| track_x(c, s)).min() else { continue };
            let bus = Rect { x: x0, y: bus_y(s), w: x_right - x0, h: w1 };
            b.rect(m2, bus);
            route[usize::from(s)].0 += i64::from(bus.w);
            let (mut vx, mut vy) = (x_right - e1o - v1, bus.y + e1);
            if mim && s == dummy_slot {
                // The dummy ring's tie hops over met4 to its pin, so at met3's
                // etch its plates are no gate's antenna (ar.met3.1 counts met3
                // alone, no diode credit): tied straight to a rail they charge
                // the rail's MOS dummy gates past the limit.
                let cy = bus.y + w1 / 2;
                let xa = x_right - e2 - vd / 2;
                let xb = xa + up(side + space(st.bot));
                let pad = |b: &mut Builder, cx: i32| b.rect(m2, tall(Rect { x: cx - vd / 2 - e2, y: cy - vd / 2 - e2, w: vd + 2 * e2, h: vd + 2 * e2 }));
                pad(&mut b, xa);
                island(&mut b, xa, cy);
                b.rect(req(process, st.top), Rect { x: xa - vt / 2 - e4, y: cy - jw / 2, w: xb - xa + vt + 2 * e4, h: jw });
                island(&mut b, xb, cy);
                pad(&mut b, xb);
                (vx, vy) = (xb - vd / 2 + floor((vd - v1) / 2), cy - vd / 2 + floor((vd - v1) / 2));
            }
            b.rect(v1l, Rect { x: vx, y: vy, w: v1, h: v1 });
            let at = lead(&mut b, vx, vy);
            if s == dummy_slot && general {
                b.pin(pnr_core::Pin { name: "GND".into(), net: pnr_core::NetId(u16::MAX), at, layer: m1 });
            } else {
                b.pin(pin(usize::from(s), "N", at, m1));
            }
        }
        // TOP: a met3 strap down each interior column, joined above the dummy
        // row and run out to a via2/via1 stack onto a met1 pin, as the buses.
        // MIM: a met4 strap over each interior column's plates, joined in the
        // gap under the top dummy row, down through via3 onto a met3 island
        // (off capm, capm.11) and its via2 to the same met2/via1 stack.
        let (vx2, vy2) = if mim {
            let join_y = (rows + o - 1) as i32 * py + uh;
            let strap = req(process, st.strap);
            for c in o..cols + o {
                let y = o as i32 * py + encp;
                b.rect(strap, Rect { x: c as i32 * px + encp, y, w: uw - 2 * encp, h: join_y + jw - y });
            }
            let x0 = o as i32 * px + encp;
            b.rect(strap, Rect { x: x0, y: join_y, w: x_right - x0, h: jw });
            let (cx, cy) = (x_right - e4 - vt + vt / 2, join_y + (jw - vt) / 2 + vt / 2);
            island(&mut b, cx, cy);
            (cx - vd / 2, cy - vd / 2)
        } else {
            let join_y = gr as i32 * py + gap_y;
            let strap_x = |c: usize| c as i32 * px + uw / 2 - m3w / 2;
            let strap_y0 = py + floor((uh - v2) / 2) - e3;
            for c in 1..=cols {
                b.rect(m3, Rect { x: strap_x(c), y: strap_y0, w: m3w, h: join_y + m3w - strap_y0 });
            }
            let join = Rect { x: strap_x(1), y: join_y, w: x_right - strap_x(1), h: m3w };
            b.rect(m3, join);
            let (vx2, vy2) = (x_right - e3 - v2, join_y + (m3w - v2) / 2);
            b.rect(v2l, Rect { x: vx2, y: vy2, w: v2, h: v2 });
            (vx2, vy2)
        };
        let v2 = vd;
        b.rect(m2, tall(Rect { x: vx2 - e2, y: vy2 - e2, w: v2 + 2 * e2, h: v2 + 2 * e2 }));
        let (vx1, vy1) = (vx2 + floor((v2 - v1) / 2), vy2 + floor((v2 - v1) / 2));
        b.rect(v1l, Rect { x: vx1, y: vy1, w: v1, h: v1 });
        let top = lead(&mut b, vx1, vy1);
        for s in 0..=usize::from(n) {
            b.pin(pin(s, "P", top, m1));
        }
        (b.finish(), route)
    }
}

fn group_sizing(group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> Sizing {
    let def = process.rule("cap_unit_side", 0);
    sizing(group, c, def, def)
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::cell::{SeriesParallel, Unitization};
    use pnr_core::{DeviceId, DeviceKind};

    fn bank(n: u8) -> (DeviceGroup, Constraints) {
        let dev_nf: Vec<u16> = (0..=n).map(|i| if i == 0 { 1 } else { 1 << (i - 1) }).collect();
        let group = DeviceGroup { devices: (0..=u16::from(n)).map(DeviceId).collect() };
        let c = Constraints {
            unitization: vec![Unitization {
                devices: group.devices.clone(),
                device_type: DeviceKind::Capacitor,
                target_ratio: dev_nf.clone(),
                dev_nf,
                unit_w: 2000,
                unit_l: 2000,
                series_parallel: SeriesParallel::Parallel,
                same_variant_required: true,
                dummy_required: true,
                route_matching_required: true,
            }],
            ..Default::default()
        };
        (group, c)
    }

    fn every_variant(n: u8) -> Vec<CapArray> {
        let mut v = Vec::new();
        for tall in [false, true] {
            for pattern in [Pattern::Spiral, Pattern::Chessboard] {
                v.push(CapArray { pattern, tall });
            }
            for k in (2..n).filter(|k| (n - k) % 2 == 0) {
                v.push(CapArray { pattern: Pattern::BlockChessboard { k, bs: 1 }, tall });
            }
        }
        v
    }

    #[test]
    fn only_binary_banks_are_recognised() {
        assert_eq!(bits(&[1, 1, 2, 4, 8]), Some(4));
        assert_eq!(bits(&[1, 1, 2]), Some(2));
        assert_eq!(bits(&[1, 2, 4, 8]), None, "no electrical dummy");
        assert_eq!(bits(&[1, 1, 2, 4, 4]), None);
    }

    /// ARR-01/02: every pattern places exactly `2^(i-1)` units of Ci, and every
    /// Ci with an even count is common-centroid (it sits in reflected pairs).
    #[test]
    fn every_pattern_is_exact_and_point_symmetric() {
        for n in 2..=7u8 {
            for v in every_variant(n) {
                let a = v.assign(n);
                for s in 0..=n {
                    let want = if s == 0 { 1 } else { 1 << (s - 1) };
                    assert_eq!(a.iter().filter(|&&x| x == s).count(), want, "{v:?} n={n} C{s}");
                }
                let len = a.len();
                for (i, &s) in a.iter().enumerate() {
                    if s >= 2 {
                        assert_eq!(a[len - 1 - i], s, "{v:?} n={n}: C{s} unit {i} has no mirror");
                    }
                }
            }
        }
    }

    /// DACP Fig. 6(a): the spiral keeps the one-unit C0/C1 on the centre four
    /// cells, and the chessboard gives the MSB every black square.
    #[test]
    fn spiral_centres_the_odd_units_and_chessboard_colours_the_msb() {
        let a = CapArray { pattern: Pattern::Spiral, tall: false }.assign(4);
        for s in [0, 1] {
            let i = a.iter().position(|&x| x == s).unwrap();
            assert!(matches!((i / 4, i % 4), (1 | 2, 1 | 2)), "C{s} at {i}");
        }
        let a = CapArray { pattern: Pattern::Chessboard, tall: false }.assign(4);
        assert!((0..16).all(|i| (a[i] == 4) == ((i / 4 + i % 4) % 2 == 0)));
    }

    /// Algorithm 1 on 4 bits: C0..C2 fill the 2×2 core, C3/C4 the corridor.
    #[test]
    fn block_chessboard_keeps_the_core() {
        let a = CapArray { pattern: Pattern::BlockChessboard { k: 2, bs: 1 }, tall: false }.assign(4);
        for i in [5, 6, 9, 10] {
            assert!(a[i] <= 2, "core cell {i} holds C{}", a[i]);
        }
    }

    /// DACP's trade (Table II): the spiral carries the least wire per unit, the
    /// chessboard's dispersion the least INL/DNL.
    #[test]
    fn metrics_rank_the_families() {
        struct Flat;
        impl Process for Flat {
            fn layer(&self, role: &str) -> Option<pnr_core::LayerId> {
                ["met1", "met2", "met3", "via1", "via2"].iter().position(|l| *l == role).map(|i| pnr_core::LayerId(i as u16 + 1))
            }
            fn rule(&self, _: &str, d: i32) -> i32 {
                d
            }
            fn grid(&self) -> i32 {
                5
            }
        }
        let (g, c) = bank(5);
        let m = |p| CapArray { pattern: p, tall: false }.metrics(&g, &c, &Flat, 1e-5, 1e-6);
        let (sp, cb) = (m(Pattern::Spiral), m(Pattern::Chessboard));
        assert!(sp.route_spread < cb.route_spread, "spiral {} vs chessboard {}", sp.route_spread, cb.route_spread);
        assert!(cb.inl_lsb < sp.inl_lsb && cb.dnl_lsb < sp.dnl_lsb, "{cb:?} vs {sp:?}");
        assert_eq!(sp.vias.len(), 6);
    }

    /// Every variant, drawn alone on sky130, is DRC- and ERC-clean.
    #[test]
    fn every_variant_is_drc_and_erc_clean() {
        let Some(pdk) = crate::testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let mut dirty = Vec::new();
        for n in [4, 5] {
            let (g, c) = bank(n);
            dirty.extend(crate::testkit::dirty_group::<CapArray>(&g, &c, &pdk));
        }
        for counts in GENERAL {
            let (g, c) = set(counts);
            dirty.extend(crate::testkit::dirty_group::<CapArray>(&g, &c, &pdk).into_iter().map(|d| format!("{counts:?} {d}")));
        }
        assert!(dirty.is_empty(), "DRC/ERC-dirty variants:\n{}", dirty.join("\n"));
    }

    /// Non-binary matched sets: Razavi Ex. 19.4's 8:1, equal pairs, odd
    /// counts, a triple.
    const GENERAL: [&[u16]; 4] = [&[8, 1], &[2, 2], &[3, 5], &[4, 4, 4]];

    fn set(counts: &[u16]) -> (DeviceGroup, Constraints) {
        let (_, mut c) = bank(2);
        let group = DeviceGroup { devices: (0..counts.len() as u16).map(DeviceId).collect() };
        let u = &mut c.unitization[0];
        u.devices = group.devices.clone();
        u.dev_nf = counts.to_vec();
        u.target_ratio = counts.to_vec();
        (group, c)
    }

    /// Each device gets exactly its units; an even count is point-symmetric,
    /// so its centroid sits on the array centre.
    #[test]
    fn a_general_set_is_exact_and_even_counts_are_centred() {
        for counts in GENERAL {
            for pattern in [Pattern::Spiral, Pattern::Chessboard] {
                let (rows, cols) = pattern::grids(counts, 3.0)[0];
                let fill = if pattern == Pattern::Chessboard { Fill::Dispersed } else { Fill::Compact };
                let slots = pattern::centro_assign(counts, rows, cols, fill).0;
                for (d, &n) in counts.iter().enumerate() {
                    let mine: Vec<usize> = (0..slots.len()).filter(|&i| slots[i] == Some(d as u8)).collect();
                    assert_eq!(mine.len(), usize::from(n), "{counts:?} {pattern:?}: device {d}");
                    if n % 2 == 0 {
                        let (sr, sc) = mine.iter().fold((0, 0), |(r, c), &i| (r + 2 * (i / cols) as i64 - rows as i64 + 1, c + 2 * (i % cols) as i64 - cols as i64 + 1));
                        assert_eq!((sr, sc), (0, 0), "{counts:?} {pattern:?}: device {d} off centre");
                    }
                }
            }
        }
    }

    /// sky130's `cap_mim_m3_1` recipe as the generator sees it.
    fn mim(pdk: &verify::Pdk) -> verify::pdk::Overlay<'_> {
        verify::pdk::Overlay { pdk, recipe: pdk.recipe("capacitor", "sky130_fd_pr__cap_mim_m3_1").expect("sky130 has the MIM recipe") }
    }

    /// `counts` (a bank when [`bits`] reads it) at `side`×`side` nm units.
    fn mim_set(counts: &[u16], side: i32) -> (DeviceGroup, Constraints) {
        let (g, mut c) = set(counts);
        c.unitization[0].unit_w = side;
        c.unitization[0].unit_l = side;
        (g, c)
    }

    /// Every variant drawn on the MIM overlay, DRC + ERC on sky130 (`P` one net).
    fn dirty_mim(g: &DeviceGroup, c: &Constraints, pdk: &verify::Pdk) -> Vec<String> {
        let ov = mim(pdk);
        let variants = CapArray::enumerate(g, c, &ov);
        assert!(!variants.is_empty(), "no MIM variants");
        variants
            .iter()
            .filter_map(|v| {
                let m = v.draw(g, c, &ov);
                let f = crate::testkit::findings(&m.shapes, &crate::testkit::ports_with(&m, &["P"]), pdk);
                (!f.is_empty()).then(|| format!("{v:?}: {f:?}"))
            })
            .collect()
    }

    /// CELL-08: a MIM bank is DRC- and ERC-clean in every variant, ringed or not.
    #[test]
    fn a_mim_bank_is_drc_and_erc_clean() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let (g, mut c) = mim_set(&[1, 1, 2, 4], 5000);
        for dummies in [true, false] {
            c.unitization[0].dummy_required = dummies;
            let dirty = dirty_mim(&g, &c, &pdk);
            assert!(dirty.is_empty(), "DRC/ERC-dirty MIM variants (dummies {dummies}):\n{}", dirty.join("\n"));
        }
    }

    /// CELL-08: the deck's capm recogniser finds one capacitor per active
    /// unit (8 for `[1, 1, 2, 4]`): no dummy plate is contacted.
    #[test]
    fn a_mim_bank_extracts_one_capacitor_per_unit() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let (g, c) = mim_set(&[1, 1, 2, 4], 5000);
        let ov = mim(&pdk);
        for v in CapArray::enumerate(&g, &c, &ov) {
            let m = v.draw(&g, &c, &ov);
            let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).expect("extracts");
            assert_eq!(spice.lines().filter(|l| l.starts_with('C')).count(), 8, "{v:?}:\n{spice}");
        }
    }

    /// CELL-08: each unit's capm is the netlist's W×L, worth the model's C_u
    /// (sky130 typical: 2.00 fF/µm², 0.19 fF/µm, m3_dw −25 nm → 53 282 aF at
    /// 5 µm), and each member draws its count of them.
    #[test]
    fn the_mim_unit_is_the_model_s() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let counts = [1u16, 1, 2, 4];
        let (g, c) = mim_set(&counts, 5000);
        let ov = mim(&pdk);
        let cu = c_u_af(&ov, 5000, 5000).expect("the MIM recipe states its model");
        assert!((cu - 53_282.0).abs() <= 10.0, "C_u {cu} aF");
        let m = CapArray::enumerate(&g, &c, &ov)[0].draw(&g, &c, &ov);
        assert!(m.drawn.iter().all(|d| d.kind == DrawnKind::Capacitor && (d.w, d.l) == (5000, 5000)), "{:?}", m.drawn);
        for (i, &n) in counts.iter().enumerate() {
            let sum: f64 = m.drawn.iter().filter(|d| usize::from(d.owner) == i).map(|d| c_u_af(&ov, d.w, d.l).unwrap()).sum();
            assert!((sum - f64::from(n) * cu).abs() < 1e-6, "member {i}: {sum} aF");
        }
    }

    /// CELL-08: a lone MIM (tq_chain's 21.87 µm, no `dummy_required`) is one
    /// drawn unit with no dummy ring (bbox under two units a side), clean.
    #[test]
    fn a_single_mim_is_one_unit() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let (g, mut c) = mim_set(&[1], 21_870);
        c.unitization[0].dummy_required = false;
        let ov = mim(&pdk);
        let variants = CapArray::enumerate(&g, &c, &ov);
        assert!(!variants.is_empty());
        for v in &variants {
            let m = v.draw(&g, &c, &ov);
            assert_eq!(m.drawn.len(), 1, "{v:?}");
            assert!(m.bbox.w < 2 * 21_870 && m.bbox.h < 2 * 21_870, "{v:?}: ringed, bbox {:?}", m.bbox);
        }
        let dirty = dirty_mim(&g, &c, &pdk);
        assert!(dirty.is_empty(), "{}", dirty.join("\n"));
    }

    /// ARR-01, measured: each variant extracted alone on sky130, the TOP-to-Ci
    /// coupling per unit within 6% of C1's. `--nocapture` prints the table the
    /// bench fixture reports.
    #[test]
    fn extracted_bit_ratios_follow_the_weights() {
        let Some(pdk) = crate::testkit::pdk() else {
            eprintln!("sky130 PDK unavailable — skipping");
            return;
        };
        let (g, c) = bank(4);
        for v in CapArray::enumerate(&g, &c, &pdk) {
            let m = v.draw(&g, &c, &pdk);
            let net = |p: &pnr_core::Pin| match p.name.split_once(':') {
                Some((_, "P")) => "top".to_string(),
                _ => format!("c{}", &p.name[1..p.name.find(':').unwrap()]),
            };
            let labels: Vec<verify::LabeledPin> = m
                .pins
                .iter()
                .map(|p| verify::LabeledPin { name: net(p), layer: p.layer.0, x: p.at.x + p.at.w / 2, y: p.at.y + p.at.h / 2 })
                .collect();
            let reference = verify::RefInput::default();
            let (_, _, caps) = verify::signoff_with_caps(&m.shapes, &labels, &reference, &pdk);
            let to_top = |i: u8| {
                let n = format!("c{i}");
                caps.iter().find(|(a, b, _)| *a == n && b.as_deref() == Some("top")).map_or(0.0, |r| r.2)
            };
            let per_unit: Vec<f64> = (1..=4).map(|i| to_top(i) / f64::from(1u32 << (i - 1))).collect();
            let mm = v.metrics(&g, &c, &pdk, 1e-5, 1e-6);
            eprintln!(
                "{v:?}: C1..C4 = {:.3}/{:.3}/{:.3}/{:.3} fF, area {:.0} um2, lin {:.2} um, quad {:.1} um2, INL {:.2e} DNL {:.2e} LSB, route spread {:.2}, vias {:?}",
                to_top(1), to_top(2), to_top(3), to_top(4), mm.area_um2, mm.lin_um, mm.quad_um2, mm.inl_lsb, mm.dnl_lsb, mm.route_spread, mm.vias
            );
            for (i, u) in per_unit.iter().enumerate() {
                assert!((u / per_unit[0] - 1.0).abs() < 0.06, "{v:?}: C{} per unit {u} vs C1 {}", i + 1, per_unit[0]);
            }
        }
    }
}
