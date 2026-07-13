//! FastSV-style connected components on the session execution model.
//!
//! Three kernels per round (all gather-only, no device sync):
//! 1. **shortcut**: `f_new[i] = f[f[i]]` (pointer jumping)
//! 2. **neighbor min**: segmented_min over CSR neighbors' current labels
//! 3. **combine**: `f_next[i] = min(f[i], shortcut[i], neighbor_min[i])`
//!
//! Rounds are enqueued blindly (ceil(log2(n)) + 2), then ONE `session.read`.
//! Host convergence check; re-upload and continue if needed.

use crate::session::{Col, Session};
use gdsverify_macros::verify_kernel;

// ---------------------------------------------------------------------------
// Kernels (single-source: CPU + GPU via #[verify_kernel])
// ---------------------------------------------------------------------------

// ponytail: map shape indexes all columns at position i, but shortcut needs
// f[f[i]] which is a gather. Use the pair shape: column = f, pairs_a = 0..n,
// pairs_b = f[0..n]. out[i] = f[pairs_b[i]] = f[f[i]].

/// Shortcut via pair-gather: out[i] = f[pairs_b[i]] = f[f[i]].
#[verify_kernel(shape = pair)]
fn sc_gather(a_val: u32, b_val: u32) -> u32 {
    let _ = a_val;
    b_val
}

/// Segmented-min kernel: identity on neighbor label (bias=0 means no-op).
#[verify_kernel(shape = segmented_min)]
fn neighbor_min(label: u32, #[uniform] _bias: u32) -> u32 {
    label + _bias
}

/// Combine: min(f[i], shortcut[i], neighbor_min[i]).
#[verify_kernel(shape = map)]
fn combine(f_i: u32, sc_i: u32, nb_i: u32) -> u32 {
    let mut r = f_i;
    if sc_i < r { r = sc_i; }
    if nb_i < r { r = nb_i; }
    r
}

// ---------------------------------------------------------------------------
// CSR builder (host-side)
// ---------------------------------------------------------------------------

/// Build symmetric CSR from edge list. Returns (neighbors, seg_start).
fn build_csr(edges: &[(u32, u32)], n: usize) -> (Vec<u32>, Vec<u32>) {
    // Count degrees
    let mut deg = vec![0u32; n];
    for &(u, v) in edges {
        if u == v { continue; }
        if (u as usize) < n { deg[u as usize] += 1; }
        if (v as usize) < n { deg[v as usize] += 1; }
    }
    // Prefix sum -> seg_start
    let mut seg_start = Vec::with_capacity(n + 1);
    seg_start.push(0u32);
    for &d in &deg {
        seg_start.push(seg_start.last().unwrap() + d);
    }
    // Fill neighbors
    let total = *seg_start.last().unwrap() as usize;
    let mut neighbors = vec![0u32; total];
    let mut cursor = seg_start[..n].to_vec();
    for &(u, v) in edges {
        if u == v { continue; }
        if (u as usize) < n && (v as usize) < n {
            neighbors[cursor[u as usize] as usize] = v;
            cursor[u as usize] += 1;
            neighbors[cursor[v as usize] as usize] = u;
            cursor[v as usize] += 1;
        }
    }
    (neighbors, seg_start)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute connected components over `n` nodes and the given edge list.
///
/// Returns a `Vec<u32>` of length `n` where each node is labeled with the
/// minimum node index in its component.
///
/// Uses the session's backend (CPU or GPU) for the iterative kernels.
pub fn connected_components(session: &Session, edges: &[(u32, u32)], n: usize) -> Vec<u32> {
    if n == 0 {
        return Vec::new();
    }
    if edges.is_empty() {
        return (0..n as u32).collect();
    }

    let (neighbors, seg_start) = build_csr(edges, n);
    let identity: Vec<u32> = (0..n as u32).collect();

    // Upload CSR structure (immutable across rounds)
    let nb_col = session.upload(&neighbors);
    let seg_col = session.upload(&seg_start);
    // For sc_gather: pairs_a is always identity (0..n)
    let id_col = session.upload(&identity);

    let max_rounds = (n.max(2) as f64).log2().ceil() as usize + 2;
    let mut f_col: Col<u32> = session.upload(&identity);

    for _ in 0..max_rounds {
        // 1. shortcut: f_new[i] = f[f[i]]
        //    pair shape: column=f, pairs_a=id(0..n), pairs_b=f
        //    out[i] = f[pairs_b[i]] = f[f[i]]
        let sc: Col<u32> = session.launch(sc_gather_kernel::bind(&f_col, &id_col, &f_col));

        // 2. neighbor min: segmented_min over CSR neighbors' current labels
        let nb: Col<u32> = session.launch(neighbor_min_kernel::bind(&f_col, &nb_col, &seg_col, 0));

        // 3. combine: f_next[i] = min(f[i], sc[i], nb[i])
        let f_next: Col<u32> = session.launch(combine_kernel::bind(&f_col, &sc, &nb));

        // Release intermediates
        session.release(sc);
        session.release(nb);
        session.release(f_col);
        f_col = f_next;
    }

    // Single sync point
    let mut f = session.read(&f_col);

    // Host convergence check: f[f[i]] == f[i] for all i, and for every edge f[u] == f[v]
    loop {
        let converged = f.iter().all(|&fi| f[fi as usize] == fi)
            && edges.iter().all(|&(u, v)| {
                let (uu, vv) = (u as usize, v as usize);
                uu >= n || vv >= n || f[uu] == f[vv]
            });
        if converged {
            break;
        }
        // Re-upload and run more rounds
        f_col = session.upload(&f);
        for _ in 0..max_rounds {
            let sc: Col<u32> = session.launch(sc_gather_kernel::bind(&f_col, &id_col, &f_col));
            let nb: Col<u32> = session.launch(neighbor_min_kernel::bind(&f_col, &nb_col, &seg_col, 0));
            let f_next: Col<u32> = session.launch(combine_kernel::bind(&f_col, &sc, &nb));
            session.release(sc);
            session.release(nb);
            session.release(f_col);
            f_col = f_next;
        }
        f = session.read(&f_col);
    }

    // Chase pointers to canonical roots on host
    for i in 0..n {
        let mut r = f[i];
        while f[r as usize] != r {
            r = f[r as usize];
        }
        f[i] = r;
    }

    f
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    /// Reference union-find for test oracles.
    fn uf_components(edges: &[(u32, u32)], n: usize) -> Vec<u32> {
        let mut parent: Vec<u32> = (0..n as u32).collect();
        fn find(p: &mut Vec<u32>, x: u32) -> u32 {
            let mut r = x;
            while p[r as usize] != r {
                p[r as usize] = p[p[r as usize] as usize];
                r = p[r as usize];
            }
            r
        }
        for &(u, v) in edges {
            if (u as usize) >= n || (v as usize) >= n { continue; }
            let (ru, rv) = (find(&mut parent, u), find(&mut parent, v));
            if ru != rv {
                let (lo, hi) = (ru.min(rv), ru.max(rv));
                parent[hi as usize] = lo;
            }
        }
        // Flatten to canonical (minimum) labels
        for i in 0..n {
            parent[i] = find(&mut parent, i as u32);
        }
        parent
    }

    /// Two partitions are equivalent iff they induce the same equivalence classes.
    fn partition_equiv(a: &[u32], b: &[u32]) -> bool {
        if a.len() != b.len() { return false; }
        let n = a.len();
        for i in 0..n {
            for j in (i + 1)..n {
                if (a[i] == a[j]) != (b[i] == b[j]) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn empty_graph() {
        let s = Session::cpu();
        assert_eq!(connected_components(&s, &[], 0), Vec::<u32>::new());
    }

    #[test]
    fn isolated_nodes() {
        let s = Session::cpu();
        let result = connected_components(&s, &[], 5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn line_graph() {
        let s = Session::cpu();
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4)];
        let result = connected_components(&s, &edges, 5);
        let oracle = uf_components(&edges, 5);
        assert!(partition_equiv(&result, &oracle), "result={result:?} oracle={oracle:?}");
        // All in one component, labeled with min=0
        assert!(result.iter().all(|&v| v == result[0]));
    }

    #[test]
    fn star_graph() {
        let s = Session::cpu();
        let edges = vec![(0, 1), (0, 2), (0, 3), (0, 4)];
        let result = connected_components(&s, &edges, 5);
        let oracle = uf_components(&edges, 5);
        assert!(partition_equiv(&result, &oracle));
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn duplicate_edges() {
        let s = Session::cpu();
        let edges = vec![(0, 1), (1, 0), (0, 1), (2, 3), (3, 2)];
        let result = connected_components(&s, &edges, 4);
        let oracle = uf_components(&edges, 4);
        assert!(partition_equiv(&result, &oracle));
    }

    #[test]
    fn self_loops() {
        let s = Session::cpu();
        let edges = vec![(0, 0), (1, 1), (2, 3)];
        let result = connected_components(&s, &edges, 4);
        let oracle = uf_components(&edges, 4);
        assert!(partition_equiv(&result, &oracle));
    }

    #[test]
    fn two_components() {
        let s = Session::cpu();
        let edges = vec![(0, 1), (1, 2), (3, 4)];
        let result = connected_components(&s, &edges, 5);
        let oracle = uf_components(&edges, 5);
        assert!(partition_equiv(&result, &oracle));
        assert_eq!(result[0], result[1]);
        assert_eq!(result[1], result[2]);
        assert_eq!(result[3], result[4]);
        assert_ne!(result[0], result[3]);
    }

    #[test]
    fn random_200_nodes() {
        let s = Session::cpu();
        // Deterministic pseudo-random edges
        let mut edges = Vec::new();
        let mut rng: u64 = 0xDEAD_BEEF;
        for _ in 0..300 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = ((rng >> 32) % 200) as u32;
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let v = ((rng >> 32) % 200) as u32;
            edges.push((u, v));
        }
        let result = connected_components(&s, &edges, 200);
        let oracle = uf_components(&edges, 200);
        assert!(partition_equiv(&result, &oracle));
    }

    #[test]
    fn session_cpu_deterministic() {
        // Run twice, same result
        let edges = vec![(0, 1), (2, 3), (1, 3), (5, 6)];
        let r1 = connected_components(&Session::cpu(), &edges, 8);
        let r2 = connected_components(&Session::cpu(), &edges, 8);
        assert_eq!(r1, r2);
        let oracle = uf_components(&edges, 8);
        assert!(partition_equiv(&r1, &oracle));
    }
}
