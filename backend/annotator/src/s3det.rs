//! S3DET — spectral graph-similarity inference (Liu et al., ASP-DAC 2020).
//!
//! The **inference tier** (Q7a): the deterministic recogniser (`pattern`) names
//! *known* topologies; S3DET finds *untemplated* repetition — "detect that two
//! subcircuits are the same when no template names them" — over the devices the
//! recogniser left as glue. This is the cross-circuit reuse detector for
//! structures the catalog has no entry for (a custom filter section, a bespoke
//! bias network replicated per channel).
//!
//! ## The kernel (why spectral, not isomorphism)
//!
//! Exact subgraph isomorphism over neighbouring circuits is "rare … graph
//! similarity provides numeric values for comparison" (S3DET slide 14). So for two
//! candidate substructures we:
//!
//! 1. extract each one's **neighbor-context subgraph** (its devices + one hop of
//!    device neighbours through signal nets — rails are dropped so the whole design
//!    doesn't collapse into one component);
//! 2. build the graph **Laplacian** `L = D − A` (degree − adjacency);
//! 3. take its **eigenvalue distribution** (a symmetric Jacobi eigensolver — no
//!    external dep needed at these sizes); and
//! 4. compare two distributions with the **Kolmogorov–Smirnov** statistic
//!    `Dₙ = supₓ |F₁(x) − F₂(x)|`. Small `Dₙ` ⇒ similar.
//!
//! Eigenvalues are permutation-invariant, so this matches isomorphic subgraphs
//! without ever solving isomorphism. Comparing *neighbor-context* (not the bare
//! substructure) is S3DET's ambiguity fix: A's surroundings resemble B's more than
//! C's, so identical-but-differently-placed cells (A,B) match without forcing the
//! infeasible (A,C) — the "≤1 symmetry per device" feasibility rule falls out for
//! free here because the glue components are a *partition* (each device in one).

use crate::block::{Block, BlockKind};
use crate::inference::BlockInference;
use crate::netrole::{classify_nets, is_rail, AnnotationConfig, NetRole};
use pnr_core::ids::{DeviceId, GroupId};
use pnr_core::{BipartiteHypergraph, Netlist};

/// Spectral-similarity inference. `threshold` is the max K-S distance at which two
/// neighbor-context subgraphs count as the same structure (0 = identical spectra).
pub struct S3Det {
    pub threshold: f64,
}

impl Default for S3Det {
    /// A modest default: spectra within a K-S distance of 0.25 are "the same".
    /// Identical structures score ~0; genuinely different topologies clear it (a
    /// 3-node path vs triangle already scores 0.33).
    fn default() -> Self {
        Self { threshold: 0.25 }
    }
}

impl BlockInference for S3Det {
    fn infer(&self, netlist: &Netlist, claimed: &[Block]) -> Vec<Block> {
        // Candidate pool = the glue remainder (what the recogniser didn't claim).
        let glue: Vec<u32> = claimed
            .iter()
            .find(|b| matches!(b.kind, BlockKind::Glue))
            .map(|b| b.devices.iter().map(|d| u32::from(d.0)).collect())
            .unwrap_or_default();
        if glue.len() < 2 {
            return Vec::new();
        }

        let hg = BipartiteHypergraph::from_netlist(netlist);
        let roles = classify_nets(&hg, &AnnotationConfig::default());

        // Partition the glue into connected substructures (via signal nets).
        let comps = components(&hg, &roles, &glue);
        if comps.len() < 2 {
            return Vec::new();
        }

        // Precompute each component's neighbor-context spectrum.
        let spectra: Vec<Vec<f64>> = comps
            .iter()
            .map(|c| laplacian_eigenvalues(context_adjacency(&hg, &roles, c)))
            .collect();

        // Group similar components into equivalence classes. Prefilter by device
        // count (a cheap necessary condition), then K-S over the spectra.
        let mut uf = Uf::new(comps.len());
        for i in 0..comps.len() {
            for j in (i + 1)..comps.len() {
                if comps[i].len() != comps[j].len() {
                    continue;
                }
                if ks_distance(&spectra[i], &spectra[j]) <= self.threshold {
                    uf.union(i, j);
                }
            }
        }

        // Emit a block per component that belongs to a matched class (size ≥ 2).
        let mut class_size = vec![0usize; comps.len()];
        for i in 0..comps.len() {
            class_size[uf.find(i)] += 1;
        }
        let mut out = Vec::new();
        for (i, c) in comps.iter().enumerate() {
            if class_size[uf.find(i)] < 2 {
                continue; // unmatched — stays glue
            }
            out.push(Block {
                kind: BlockKind::Group,
                template: "s3det",
                devices: c.iter().map(|&d| DeviceId(d as u16)).collect(),
                group: GroupId(0), // annotate reassigns
                depends_on: Vec::new(),
                injected: false,
                sub_blocks: Vec::new(),
            });
        }
        out
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Graph extraction
// ═══════════════════════════════════════════════════════════════════════

/// Do devices `a` and `b` share a **signal** net (rails excluded)?
fn share_signal_net(hg: &BipartiteHypergraph, roles: &[NetRole], a: u32, b: u32) -> bool {
    let na = &hg.device_nets[a as usize];
    let nb = &hg.device_nets[b as usize];
    na.iter().any(|n| !is_rail(roles[n.0 as usize]) && nb.contains(n))
}

/// Connected components of the glue devices, edges = shared signal net.
fn components(hg: &BipartiteHypergraph, roles: &[NetRole], glue: &[u32]) -> Vec<Vec<u32>> {
    let mut uf = Uf::new(glue.len());
    for i in 0..glue.len() {
        for j in (i + 1)..glue.len() {
            if share_signal_net(hg, roles, glue[i], glue[j]) {
                uf.union(i, j);
            }
        }
    }
    let mut by_root: std::collections::HashMap<usize, Vec<u32>> = std::collections::HashMap::new();
    for i in 0..glue.len() {
        by_root.entry(uf.find(i)).or_default().push(glue[i]);
    }
    let mut comps: Vec<Vec<u32>> = by_root
        .into_values()
        .map(|mut c| {
            c.sort_unstable();
            c
        })
        .collect();
    comps.sort_unstable_by_key(|c| c[0]);
    comps
}

/// Neighbor-context adjacency for a component: its devices plus one hop of device
/// neighbours (reached through signal nets), as a symmetric 0/1 device-device
/// adjacency matrix. Context — not the bare component — is what makes A match B
/// over C.
fn context_adjacency(hg: &BipartiteHypergraph, roles: &[NetRole], comp: &[u32]) -> Vec<Vec<f64>> {
    // Node set: component ∪ one-hop device neighbours.
    let mut nodes: Vec<u32> = comp.to_vec();
    for &d in comp {
        for (other, _) in hg.device_nets.iter().enumerate() {
            let o = other as u32;
            if !nodes.contains(&o) && share_signal_net(hg, roles, d, o) {
                nodes.push(o);
            }
        }
    }
    nodes.sort_unstable();
    let n = nodes.len();
    let mut adj = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if share_signal_net(hg, roles, nodes[i], nodes[j]) {
                adj[i][j] = 1.0;
                adj[j][i] = 1.0;
            }
        }
    }
    adj
}

// ═══════════════════════════════════════════════════════════════════════
//  Spectral kernel — Laplacian eigenvalues + K-S distance (no external dep)
// ═══════════════════════════════════════════════════════════════════════

/// Eigenvalues of the graph Laplacian `L = D − A` of adjacency `adj`, ascending.
#[must_use]
pub fn laplacian_eigenvalues(adj: Vec<Vec<f64>>) -> Vec<f64> {
    let n = adj.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        let deg: f64 = adj[i].iter().sum();
        for j in 0..n {
            l[i][j] = if i == j { deg - adj[i][j] } else { -adj[i][j] };
        }
    }
    jacobi_eigenvalues(l)
}

/// Eigenvalues of a symmetric matrix by the cyclic Jacobi method, ascending.
#[must_use]
pub fn jacobi_eigenvalues(mut a: Vec<Vec<f64>>) -> Vec<f64> {
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    for _sweep in 0..100 {
        // Off-diagonal Frobenius norm — stop when negligible.
        let mut off = 0.0;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[p][q] * a[p][q];
            }
        }
        if off < 1e-14 {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p][q];
                if apq.abs() < 1e-18 {
                    continue;
                }
                // Rotation that zeroes a[p][q] (Golub & Van Loan §8.4).
                let tau = (a[q][q] - a[p][p]) / (2.0 * apq);
                let t = if tau >= 0.0 {
                    1.0 / (tau + (1.0 + tau * tau).sqrt())
                } else {
                    -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                // J^T A J, applied to columns then rows p,q.
                for k in 0..n {
                    let (g, h) = (a[k][p], a[k][q]);
                    a[k][p] = c * g - s * h;
                    a[k][q] = s * g + c * h;
                }
                for k in 0..n {
                    let (g, h) = (a[p][k], a[q][k]);
                    a[p][k] = c * g - s * h;
                    a[q][k] = s * g + c * h;
                }
            }
        }
    }
    let mut ev: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    ev.sort_by(|x, y| x.partial_cmp(y).unwrap());
    ev
}

/// Kolmogorov–Smirnov statistic `supₓ |F₁(x) − F₂(x)|` between two samples.
#[must_use]
pub fn ks_distance(x: &[f64], y: &[f64]) -> f64 {
    match (x.is_empty(), y.is_empty()) {
        (true, true) => return 0.0,
        (true, _) | (_, true) => return 1.0,
        _ => {}
    }
    let cdf = |s: &[f64], v: f64| s.iter().filter(|&&e| e <= v + 1e-9).count() as f64 / s.len() as f64;
    let mut pts: Vec<f64> = x.iter().chain(y.iter()).copied().collect();
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    pts.iter().map(|&v| (cdf(x, v) - cdf(y, v)).abs()).fold(0.0, f64::max)
}

// ═══════════════════════════════════════════════════════════════════════
//  Tiny union-find (local — the crate's pnr_core one is device-scoped)
// ═══════════════════════════════════════════════════════════════════════

struct Uf {
    parent: Vec<usize>,
}

impl Uf {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect() }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        // path-halving
        let mut c = x;
        while self.parent[c] != r {
            let next = self.parent[c];
            self.parent[c] = r;
            c = next;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

#[cfg(test)]
mod kernel_tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }
    // Path P3 (0-1-2) and triangle K3 adjacencies.
    fn p3() -> Vec<Vec<f64>> {
        vec![vec![0., 1., 0.], vec![1., 0., 1.], vec![0., 1., 0.]]
    }
    fn k3() -> Vec<Vec<f64>> {
        vec![vec![0., 1., 1.], vec![1., 0., 1.], vec![1., 1., 0.]]
    }

    #[test]
    fn jacobi_recovers_diagonal_and_offdiag() {
        assert!(jacobi_eigenvalues(vec![vec![2., 0.], vec![0., 3.]]).iter().zip([2., 3.]).all(|(a, b)| close(*a, b)));
        // [[0,1],[1,0]] has eigenvalues ±1.
        let e = jacobi_eigenvalues(vec![vec![0., 1.], vec![1., 0.]]);
        assert!(close(e[0], -1.0) && close(e[1], 1.0), "{e:?}");
    }

    #[test]
    fn laplacian_spectra_are_known() {
        // L(P3) eigenvalues = {0, 1, 3}; L(K3) = {0, 3, 3}.
        let ep = laplacian_eigenvalues(p3());
        assert!(close(ep[0], 0.) && close(ep[1], 1.) && close(ep[2], 3.), "{ep:?}");
        let ek = laplacian_eigenvalues(k3());
        assert!(close(ek[0], 0.) && close(ek[1], 3.) && close(ek[2], 3.), "{ek:?}");
    }

    #[test]
    fn ks_zero_for_identical_positive_for_different() {
        let ep = laplacian_eigenvalues(p3());
        assert!(close(ks_distance(&ep, &ep), 0.0));
        let ek = laplacian_eigenvalues(k3());
        // P3 vs K3 must clear the default 0.25 threshold (they are different).
        assert!(ks_distance(&ep, &ek) > 0.25, "P3 vs K3 = {}", ks_distance(&ep, &ek));
    }

    #[test]
    fn spectrum_is_permutation_invariant() {
        // Same path graph, relabeled 0-2-1: identical spectrum ⇒ K-S distance 0.
        let relabeled = vec![vec![0., 0., 1.], vec![0., 0., 1.], vec![1., 1., 0.]];
        let d = ks_distance(&laplacian_eigenvalues(p3()), &laplacian_eigenvalues(relabeled));
        assert!(close(d, 0.0), "isomorphic graphs must match, got {d}");
    }

    #[test]
    fn context_distinguishes_different_structures() {
        // A triangle (K3, L-spectrum {0,3,3}) vs a 4-node star (S4, {0,1,1,4}).
        // Different connectivity ⇒ K-S distance clears the default threshold with
        // margin — the separation S3DET relies on to not merge unlike cells.
        let star = vec![
            vec![0., 1., 1., 1.],
            vec![1., 0., 0., 0.],
            vec![1., 0., 0., 0.],
            vec![1., 0., 0., 0.],
        ];
        let d = ks_distance(&laplacian_eigenvalues(k3()), &laplacian_eigenvalues(star));
        assert!(d > 0.25, "different structures should separate, got {d}");
    }
}
