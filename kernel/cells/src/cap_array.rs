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
use pnr_core::{DeviceGroup, Drawn, DrawnKind, KeepWhy, Macro, MatchClass, Node, Process, Rect, Unit};

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
    /// Split DAC (DACP §IV-C, CELL-21; offered by [`CapArray::split`] only):
    /// members ordered LSB bank `[1, 1, 2, …, 2^(lsb-1)]` (termination first),
    /// then MSB `[1, 2, …]`, then the bridge C_A; owner ids as in
    /// [`analog::matching::dac::split_dac_assign`]; MSB bits = `members − lsb − 2`.
    /// The bridge draws two square half plates. Each bank has its own top:
    /// `bridge_top_lsb` puts the bridge's `P` on the LSB one.
    Split { lsb: u8, bridge_top_lsb: bool },
}

/// One array variant.
///
/// Pins: `d{i}:P` (all on the TOP join, one net; a [`Pattern::Split`] has one
/// TOP lead per bank), `d{i}:N` on bus `i`.
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
    /// Worst slot's second-moment residue ‖M_slot − M_array‖_F per unit, µm²
    /// ([`analog::matching::dac::second_um2`]): unlike `⟨r²⟩` it sees an
    /// anisotropic (xx vs yy, xy) imbalance.
    pub second_um2: f64,
    /// Moment orders every slot with ≥ 2 units cancels
    /// ([`analog::matching::moments::cancelled_order`], nmax 4, tol 1e-3).
    pub order: u8,
    /// Worst `|INL|` / `|DNL|`, LSB, under the t0/t gradient `g`
    /// ([`analog::matching::dac::inl_dnl`]), worst over θ in `π/(4·max(rows, cols))` steps.
    pub inl_lsb: f64,
    pub dnl_lsb: f64,
    /// Systematic per-unit ratio mismatch of each bit to C0 ([`analog::matching::dac::msys`]).
    pub msys: f64,
    /// Bottom-plate route length per unit, `max/min − 1` over C1..=CN: 0 when
    /// every bit carries the same wire per unit of capacitance.
    pub route_spread: f64,
    /// `via1` cuts on each slot's bottom-plate route.
    pub vias: Vec<u32>,
    pub area_um2: f64,
}

/// t0/t gradient at which an Exceptional bank ranks its variants, 1/µm: DACP's γ = 100 ppm read per µm (GAP-18).
pub const RANK_G_PER_UM: f64 = 1e-4;

/// Largest bank: 256 units.
const MAX_BITS: u8 = 8;

impl Cell for CapArray {
    /// An Exceptional binary bank lists its variants by (M_sys, INL, area) at `RANK_G_PER_UM` (GAP-18, CC-24).
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
        let mut out: Vec<Self> = out.into_iter().map(|(v, _)| v).collect();
        if crate::builder::unitization(group, constraints).and_then(|u| u.class) == Some(MatchClass::Exceptional) {
            let mut keyed: Vec<([f64; 3], Self)> = out
                .into_iter()
                .map(|v| {
                    let m = v.metrics(group, constraints, process, RANK_G_PER_UM);
                    ([m.msys, m.inl_lsb, m.area_um2], v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.iter().zip(&b.0).map(|(x, y)| x.total_cmp(y)).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal));
            out = keyed.into_iter().map(|(_, v)| v).collect();
        }
        out
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

/// Side, nm, of the square half of a split DAC's bridge C_A =
/// `C_T^LSB/C_T^MSB·C_u` ([`analog::matching::dac::attenuation_cap`]), by
/// [`c_u_af`]'s model: `C = c_area·u² + 4·c_perim·u`, `u = t + dw`, snapped up
/// to the cut lattice. `None` without `c_area_af_um2`.
fn ca_half_side(p: &dyn Process, unit_w: i32, unit_l: i32, lsb: u8, msb: u8) -> Option<i32> {
    // ponytail: square half; an off-grid C_A (keeps k_u) is the upgrade.
    let ca = analog::matching::dac::attenuation_cap(1 << lsb, (1 << msb) - 1);
    let target = ca * c_u_af(p, unit_w, unit_l)? / 2.0;
    let (a, cp) = (f64::from(p.rule("c_area_af_um2", 0)), f64::from(p.rule("c_perim_af_um", 0)));
    let u = (-4.0 * cp + (16.0 * cp * cp + 4.0 * a * target).sqrt()) / (2.0 * a);
    let t_nm = (u - f64::from(p.rule("c_dw_nm", 0)) * 1e-3) * 1e3;
    let lat = cut_lattice(p);
    Some((t_nm / f64::from(lat)).ceil() as i32 * lat)
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
            Pattern::Split { .. } => unreachable!("a split DAC is assigned by `split_dac_assign`"),
            Pattern::BlockChessboard { k, bs } => {
                g.chessboard(centred(dims(k, self.tall)), k);
                for i in (k + 1..n).step_by(2) {
                    g.corridor(centred(dims(i + 1, self.tall)), centred(dims(i - 1, self.tall)), i, usize::from(bs));
                }
            }
        }
        g.slot.into_iter().map(|s| s.expect("every cell assigned")).collect()
    }

    /// The [`Pattern::Split`] variant of a split DAC whose members are
    /// ordered as that pattern says: `None` unless the stack is MIM, the dummy
    /// ring is drawn, the LSB bank is a binary bank of `lsb` bits, the MSB
    /// counts are `[1, 2, …, 2^(M-1)]` (M ≥ 1), and the bridge's square half
    /// fits between the plate's min width and the unit plate.
    /// [`Cell::enumerate`] never offers it: it cannot see which bank is which.
    #[must_use]
    pub fn split(group: &DeviceGroup, c: &Constraints, process: &dyn Process, lsb: u8, bridge_top_lsb: bool) -> Option<Self> {
        plate_stack(process)?.plate?;
        crate::builder::unitization(group, c)?.dummy_required.then_some(())?;
        let s = group_sizing(group, c, process);
        let l = usize::from(lsb);
        (s.dev_nf.len() >= l + 3 && bits(&s.dev_nf[..=l]) == Some(lsb)).then_some(())?;
        let msb = &s.dev_nf[l + 1..s.dev_nf.len() - 1];
        msb.iter().enumerate().all(|(i, &u)| u32::from(u) == 1 << i).then_some(())?;
        let half = ca_half_side(process, s.unit_w, s.unit_l, lsb, msb.len() as u8)?;
        let lat = cut_lattice(process);
        let unit = |v: i32| ((v + lat - 1) / lat * lat).max(process.width("plate").unwrap_or(0));
        (half >= process.width("plate").unwrap_or(0) && half <= unit(s.unit_w).min(unit(s.unit_l))).then_some(CapArray { pattern: Pattern::Split { lsb, bridge_top_lsb }, tall: false })
    }

    /// ARR-02/03 figures for this variant under the t0/t gradient `g` (1/µm).
    /// No deck carries it, so it is the caller's sweep point, not a constant.
    #[must_use]
    pub fn metrics(&self, group: &DeviceGroup, c: &Constraints, process: &dyn Process, g: f64) -> ArrayMetrics {
        use analog::matching::{dac, moments};
        let (m, routes) = self.build(group, c, process);
        let n = routes.len() - 1;
        let um = |v: i32| f64::from(v) * 1e-3;
        let (cx, cy) = {
            let k = m.units.len() as f64;
            (m.units.iter().map(|u| um(u.x)).sum::<f64>() / k, m.units.iter().map(|u| um(u.y)).sum::<f64>() / k)
        };
        let units: Vec<(u8, f64, f64)> = m.units.iter().map(|u| (u.owner, um(u.x) - cx, um(u.y) - cy)).collect();
        let mut out = ArrayMetrics { vias: routes.iter().map(|r| r.1).collect(), ..Default::default() };
        let mut pts: Vec<Vec<moments::Pt>> = vec![Vec::new(); n + 1];
        for &(s, x, y) in &units {
            pts[usize::from(s)].push(moments::Pt { x, y, w: 1.0, phi: (0, 0) });
        }
        for p in &pts {
            let k = p.len() as f64;
            out.lin_um = out.lin_um.max((p.iter().map(|q| q.x).sum::<f64>() / k).hypot(p.iter().map(|q| q.y).sum::<f64>() / k));
        }
        let multi: Vec<&[moments::Pt]> = pts.iter().filter(|p| p.len() >= 2).map(Vec::as_slice).collect();
        out.order = moments::cancelled_order(&multi, 4, 1e-3).0;
        out.second_um2 = dac::second_um2(&units, n + 1);
        // Rows/columns as drawn (whichever grid `build` picked): distinct unit coordinates.
        let distinct = |f: fn(&Unit) -> i32| m.units.iter().map(f).collect::<std::collections::BTreeSet<_>>().len();
        let steps = 4 * distinct(|u| u.x).max(distinct(|u| u.y));
        (out.inl_lsb, out.dnl_lsb) = dac::inl_dnl(&units, n as u8, g, steps);
        let counts: Vec<u16> = pts.iter().map(|p| p.len() as u16).collect();
        out.msys = dac::msys(&units, &counts, g, steps);
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
    /// `side`: a centred square plate of that side instead (a split DAC's
    /// bridge half); the bottom plate stays the cell. Returns the plate.
    #[allow(clippy::too_many_arguments)]
    fn mim_unit(b: &mut Builder, process: &dyn Process, st: &PlateStack, cell: Rect, encp: i32, slot: Option<u8>, ring: bool, side: Option<i32>) -> Rect {
        let plate_role = st.plate.expect("MIM");
        let lat = cut_lattice(process);
        b.rect(req(process, st.bot), cell);
        let plate = match side {
            Some(t) => Rect { x: cell.x + (cell.w - t) / 2 / lat * lat, y: cell.y + (cell.h - t) / 2 / lat * lat, w: t, h: t },
            None => Rect { x: cell.x + encp, y: cell.y + encp, w: cell.w - 2 * encp, h: cell.h - 2 * encp },
        };
        if slot.is_none() && !ring {
            return plate;
        }
        b.rect(req(process, plate_role), plate);
        let Some(owner) = slot else { return plate };
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
        plate
    }

    /// The macro, plus each slot's `(bottom-route length nm, via1 cuts)`.
    fn build(&self, group: &DeviceGroup, c: &Constraints, process: &dyn Process) -> (Macro, Vec<(i64, u32)>) {
        let s = group_sizing(group, c, process);
        // A binary bank, or a general set (see `enumerate`). A bank's dummies
        // join C0 (its electrical dummy, on a rail); a general set's get a bus
        // of their own, slot `n + 1`, pinned `GND` for the caller to tie to
        // ground: tied to a member they would load its bottom plate alone.
        let split = match self.pattern {
            Pattern::Split { lsb, bridge_top_lsb } => Some((lsb, bridge_top_lsb, (s.dev_nf.len() - 2 - usize::from(lsb)) as u8)),
            _ => None,
        };
        let general = split.is_none() && bits(&s.dev_nf).is_none();
        let (n, rows, cols, slots) = if let Some((lsb, _, msb)) = split {
            // The bridge is two halves: `[2]` in its place sizes the grid.
            let mut counts = s.dev_nf.clone();
            *counts.last_mut().expect("split has members") = 2;
            let (rows, cols) = pattern::grids(&counts, 3.0)[0];
            ((s.dev_nf.len() - 1) as u8, rows, cols, analog::matching::dac::split_dac_assign(lsb, msb, rows, cols))
        } else {
            match bits(&s.dev_nf) {
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
            let g = inset.max(m1s).max(m2s).max(capm_gap).max(space(st.bot)).max(jw);
            // Split: a bank's top line runs in each row gap clear of the
            // other bank's pads, centred (gap − jw an even lattice count).
            if split.is_some() {
                (g.max(jw + 2 * (space(st.top) - encp)) + 2 * lat - 1) / (2 * lat) * 2 * lat
            } else {
                g
            }
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

        let half = split.and_then(|(lsb, _, msb)| ca_half_side(process, s.unit_w, s.unit_l, lsb, msb));
        // Split: (interior row, plate, owner) of every member unit.
        let mut plates: Vec<(usize, Rect, u8)> = Vec::new();
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
                    let side = half.filter(|_| slot == Some(n));
                    let plate = Self::mim_unit(&mut b, process, &st, Rect { x: x0, y: y0, w: uw, h: uh }, encp, slot, o == 1 && (r == 0 || c == 0 || r == gr - 1 || c == gc - 1), side);
                    if let Some(owner) = slot {
                        plates.push((r - o, plate, owner));
                    }
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
        // Split: each unit's met4 pad over its own capm, run through the
        // cell edge to its bank's line in the row gap next to it (gap `k`, under
        // interior row `k`: LSB when `k` is even, MSB when odd); the LSB lines
        // join a spine left of the array, the MSB lines one at `x_right`. A
        // strap could not carry two nets: met4 over a plate binds it (deck capm).
        // Each entry: (via2 corner, lead to the left, the slots pinned there).
        let all: Vec<u8> = (0..=n).collect();
        let stacks: Vec<(i32, i32, bool, Vec<u8>)> = if let Some((lsb, top_lsb, _)) = split {
            let strap = req(process, st.strap);
            let in_lsb = |s: u8| s <= lsb || (s == n && top_lsb);
            let line_y = |k: usize| (o + k) as i32 * py - gap_y + (gap_y - jw) / 2;
            let cxl = -(space(st.bot) + side);
            // Pad and stub are one rect, plate-wide: the deck binds a capm's
            // top to the one met4 polygon on it (a second refuses the device).
            for &(r, plate, s) in &plates {
                let ly = line_y(r + usize::from((r % 2 == 0) != in_lsb(s)));
                let y0 = ly.min(plate.y);
                b.rect(strap, Rect { y: y0, h: (ly + jw).max(plate.y + plate.h) - y0, ..plate });
            }
            for k in 0..=rows {
                let (x0, x1) = if k % 2 == 0 { (cxl - jw / 2, (cols + o) as i32 * px) } else { (o as i32 * px, x_right) };
                b.rect(strap, Rect { x: x0, y: line_y(k), w: x1 - x0, h: jw });
            }
            let mut out = Vec::new();
            for (parity, cx, left) in [(0, cxl, true), (1, x_right - jw / 2, false)] {
                let ks: Vec<usize> = (0..=rows).filter(|k| k % 2 == parity).collect();
                let (lo, hi) = (line_y(ks[0]), line_y(*ks.last().expect("rows ≥ 1")));
                b.rect(strap, Rect { x: cx - jw / 2, y: lo, w: jw, h: hi + jw - lo });
                let cy = hi + jw / 2;
                island(&mut b, cx, cy);
                out.push((cx - vd / 2, cy - vd / 2, left, all.iter().copied().filter(|&s| in_lsb(s) == left).collect()));
            }
            out
        } else if mim {
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
            vec![(cx - vd / 2, cy - vd / 2, false, all)]
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
            vec![(vx2, vy2, false, all)]
        };
        let v2 = vd;
        for (vx2, vy2, left, slots) in stacks {
            b.rect(m2, tall(Rect { x: vx2 - e2, y: vy2 - e2, w: v2 + 2 * e2, h: v2 + 2 * e2 }));
            let (vx1, vy1) = (vx2 + floor((v2 - v1) / 2), vy2 + floor((v2 - v1) / 2));
            b.rect(v1l, Rect { x: vx1, y: vy1, w: v1, h: v1 });
            let top = if left {
                // `lead` mirrored: the met1 run goes left, clear of the array.
                let x1 = vx1 + v1 + e1o;
                b.rect(m1, Rect { x: x1 - reach - pp, y: vy1 - e1o, w: reach + pp, h: pp });
                Rect { x: x1 - reach - pp, y: vy1 - e1o, w: pp, h: pp }
            } else {
                lead(&mut b, vx1, vy1)
            };
            for s in slots {
                b.pin(pin(usize::from(s), "P", top, m1));
            }
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
                dummy_required: true,
                route_matching_required: true,
                class: None, kind: None, series: Vec::new(), style: None,
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

    /// Deck-free process: met1..3 and vias, every rule at its default.
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

    /// GAP-18: an Exceptional bank leads with its lowest-M_sys variant and lists the rest by (M_sys, INL, area);
    /// a Moderate one keeps the generator order. Over n = 4..=6, the n whose plain order is not already
    /// key-sorted (else the test is vacuous).
    #[test]
    fn exceptional_banks_try_the_lowest_msys_first() {
        let key = |v: &CapArray, g: &DeviceGroup, c: &Constraints| {
            let m = v.metrics(g, c, &Flat, RANK_G_PER_UM);
            [m.msys, m.inl_lsb, m.area_um2]
        };
        let le = |a: &[f64; 3], b: &[f64; 3]| a.iter().zip(b).map(|(x, y)| x.total_cmp(y)).find(|o| o.is_ne()).is_none_or(|o| o.is_lt());
        let sorted = |ks: &[[f64; 3]]| ks.windows(2).all(|w| le(&w[0], &w[1]));
        let mut tested = 0;
        for n in 4..=6 {
            let (g, mut c) = bank(n);
            let plain = CapArray::enumerate(&g, &c, &Flat);
            let plain_keys: Vec<[f64; 3]> = plain.iter().map(|v| key(v, &g, &c)).collect();
            if sorted(&plain_keys) {
                continue;
            }
            tested += 1;
            c.unitization[0].class = Some(MatchClass::Moderate);
            let moderate = CapArray::enumerate(&g, &c, &Flat);
            let id = |v: &CapArray| (v.pattern, v.tall);
            assert_eq!(moderate.iter().map(id).collect::<Vec<_>>(), plain.iter().map(id).collect::<Vec<_>>(), "n={n}: Moderate reordered");
            c.unitization[0].class = Some(MatchClass::Exceptional);
            let ranked = CapArray::enumerate(&g, &c, &Flat);
            assert_eq!(ranked.len(), plain.len());
            assert!(plain.iter().all(|p| ranked.iter().any(|r| id(r) == id(p))), "n={n}: a variant went missing");
            let keys: Vec<[f64; 3]> = ranked.iter().map(|v| key(v, &g, &c)).collect();
            assert!(sorted(&keys), "n={n}: ranked keys {keys:?}");
            assert!(keys.iter().all(|k| keys[0][0] <= k[0]), "n={n}: first is not the lowest M_sys");
        }
        assert!(tested > 0, "every n in 4..=6 already lists its variants key-sorted");
    }

    /// DACP's trade (Table II): the spiral carries the least wire per unit, the
    /// chessboard the least INL (the one-unit C0/C1 dominate DNL on a drawn
    /// 5-bit bank, so no DNL order is asserted).
    #[test]
    fn metrics_rank_the_families() {
        let (g, c) = bank(5);
        let m = |p| CapArray { pattern: p, tall: false }.metrics(&g, &c, &Flat, 1e-5);
        let (sp, cb) = (m(Pattern::Spiral), m(Pattern::Chessboard));
        assert!(sp.route_spread < cb.route_spread, "spiral {} vs chessboard {}", sp.route_spread, cb.route_spread);
        assert!(cb.inl_lsb < sp.inl_lsb, "chessboard INL {} vs spiral {}", cb.inl_lsb, sp.inl_lsb);
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

    /// A split DAC as [`Pattern::Split`] orders it: LSB bank of `l` bits, MSB
    /// bank of `m` bits, the bridge last, at 3 µm MIM units, ringed.
    fn split_set(l: u8, m: u8) -> (DeviceGroup, Constraints) {
        let counts: Vec<u16> = (0..=l).map(|i| if i == 0 { 1 } else { 1 << (i - 1) }).chain((0..m).map(|i| 1 << i)).chain([1]).collect();
        let (g, mut c) = mim_set(&counts, 3000);
        c.unitization[0].dummy_required = true;
        (g, c)
    }

    /// CELL-21: every member draws its count (the bridge two halves), every
    /// even-count cap but the bridge's own is point-symmetric about the
    /// units' centre (exact common centroid), the array is DRC/ERC-clean with
    /// one top net per bank, and the deck extracts one capacitor per unit on
    /// exactly two top nodes.
    #[test]
    fn a_split_bank_is_point_symmetric() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let ov = mim(&pdk);
        for (l, mb) in [(3u8, 3u8), (2, 2)] {
            let (g, c) = split_set(l, mb);
            let counts = c.unitization[0].dev_nf.clone();
            let n = counts.len() - 1;
            let m = CapArray::split(&g, &c, &ov, l, true).expect("offered").draw(&g, &c, &ov);
            let (xs, ys) = (m.units.iter().map(|u| u.x), m.units.iter().map(|u| u.y));
            let (sx, sy) = (xs.clone().min().unwrap() + xs.max().unwrap(), ys.clone().min().unwrap() + ys.max().unwrap());
            for (i, &k) in counts.iter().enumerate() {
                let units: Vec<(i32, i32)> = m.units.iter().filter(|u| usize::from(u.owner) == i).map(|u| (u.x, u.y)).collect();
                assert_eq!(units.len(), if i == n { 2 } else { usize::from(k) }, "({l}, {mb}) member {i}");
                if ![0, 1, usize::from(l) + 1].contains(&i) {
                    assert!(units.iter().all(|&(x, y)| units.contains(&(sx - x, sy - y))), "({l}, {mb}) member {i} not point-symmetric: {units:?}");
                }
            }
            let mut ports = crate::testkit::ports_with(&m, &[]);
            for p in &mut ports {
                let i: usize = p.name.strip_prefix('d').and_then(|r| r.strip_suffix("_P")).map_or(usize::MAX, |d| d.parse().unwrap());
                if i != usize::MAX {
                    p.name = if i <= usize::from(l) || i == n { "TL" } else { "TM" }.into();
                }
            }
            let f = crate::testkit::findings(&m.shapes, &ports, &pdk);
            assert!(f.is_empty(), "({l}, {mb}): {f:?}");
            let spice = verify::extract_spice(&m.shapes, &[], &pdk, verify::Detail::Schematic).expect("extracts");
            let caps: Vec<&str> = spice.lines().filter(|x| x.starts_with('C')).collect();
            assert_eq!(caps.len(), (1 << l) + (1 << mb) + 1, "({l}, {mb}):\n{spice}");
            let tops: std::collections::BTreeSet<&str> = caps.iter().map(|x| x.split_whitespace().nth(1).unwrap()).collect();
            assert_eq!(tops.len(), 2, "({l}, {mb}) top nodes {tops:?}:\n{spice}");
        }
    }

    /// CELL-21: the two square halves sum to MAT-18's C_A within one lattice
    /// step of side, and are the drawn bridge plates. Their edge ratio k stays
    /// above the unit's (equal k is impossible inside a unit cell, see the
    /// card); the Δk is printed. A C_A larger than the unit cell (a 4+1 split,
    /// C_A = 16 C_u) is not offered.
    #[test]
    fn the_bridge_is_the_attenuation_cap() {
        let Some(pdk) = crate::testkit::pdk() else { return };
        let ov = mim(&pdk);
        let lat = cut_lattice(&ov);
        let c = |w: i32| c_u_af(&ov, w, w).expect("MIM model");
        let k = |w: f64| 2.0 / w;
        for (l, mb) in [(2u8, 2u8), (3, 3)] {
            let half = ca_half_side(&ov, 3000, 3000, l, mb).expect("MIM model");
            let want = analog::matching::dac::attenuation_cap(1 << l, (1 << mb) - 1) * c(3000);
            let step = 2.0 * (c(half + lat) - c(half));
            assert!((2.0 * c(half) - want).abs() <= step, "({l}, {mb}): 2·C({half}) = {} aF vs C_A {want} aF", 2.0 * c(half));
            assert!(k(f64::from(half)) >= k(3000.0));
            eprintln!("({l}, {mb}): half {half} nm, k {:.3e} vs k_u {:.3e} /nm (Δk {:.3e})", k(f64::from(half)), k(3000.0), k(f64::from(half)) - k(3000.0));
            let (g, cs) = split_set(l, mb);
            let n = cs.unitization[0].dev_nf.len() - 1;
            let m = CapArray::split(&g, &cs, &ov, l, false).expect("offered").draw(&g, &cs, &ov);
            let bridge: Vec<(i32, i32)> = m.drawn.iter().filter(|d| usize::from(d.owner) == n).map(|d| (d.w, d.l)).collect();
            assert_eq!(bridge, vec![(half, half); 2], "({l}, {mb})");
        }
        let (g, cs) = split_set(4, 1);
        assert!(CapArray::split(&g, &cs, &ov, 4, true).is_none(), "C_A = 16 C_u cannot fit a unit cell");
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
            let mm = v.metrics(&g, &c, &pdk, 1e-5);
            eprintln!(
                "{v:?}: C1..C4 = {:.3}/{:.3}/{:.3}/{:.3} fF, area {:.0} um2, lin {:.2} um, order {}, second {:.1} um2, INL {:.2e} DNL {:.2e} LSB, msys {:.2e}, route spread {:.2}, vias {:?}",
                to_top(1), to_top(2), to_top(3), to_top(4), mm.area_um2, mm.lin_um, mm.order, mm.second_um2, mm.inl_lsb, mm.dnl_lsb, mm.msys, mm.route_spread, mm.vias
            );
            for (i, u) in per_unit.iter().enumerate() {
                assert!((u / per_unit[0] - 1.0).abs() < 0.06, "{v:?}: C{} per unit {u} vs C1 {}", i + 1, per_unit[0]);
            }
        }
    }
}
