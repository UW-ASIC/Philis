//! Union-find over device indices: rules' `extract` unions matched devices, and
//! the annotator reads the sets back as the group table.

/// Disjoint-set forest with path compression + union by rank over the
/// elements `0..n`. Near-O(1) amortised per operation.
pub struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
}

impl UnionFind {
    /// A forest of `n` singletons.
    ///
    /// # Panics
    /// In debug builds, if `n` exceeds `u32::MAX` (elements are `u32`).
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self { parent: (0..n as u32).collect(), rank: vec![0; n] }
    }

    /// Representative of `x`'s set. Compresses the path from `x`, so later
    /// finds on it are O(1); representatives change only through
    /// [`UnionFind::union`].
    ///
    /// # Panics
    /// If `x >= n`.
    pub fn find(&mut self, x: u32) -> u32 {
        let mut root = x;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        let mut cur = x;
        while self.parent[cur as usize] != root {
            let next = self.parent[cur as usize];
            self.parent[cur as usize] = root;
            cur = next;
        }
        root
    }

    /// Merges the sets of `a` and `b`; a no-op when already joined.
    ///
    /// # Panics
    /// If `a` or `b` is `>= n`.
    pub fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        let (lo, hi) = if self.rank[ra as usize] < self.rank[rb as usize] { (ra, rb) } else { (rb, ra) };
        self.parent[lo as usize] = hi;
        if self.rank[lo as usize] == self.rank[hi as usize] {
            self.rank[hi as usize] += 1;
        }
    }

    /// The non-singleton sets, each as an ascending member list, ordered by
    /// smallest member (deterministic). Singletons (devices in no group) are
    /// omitted — they stay addressable as `Target::Device`. O(n); takes
    /// `&mut self` only to compress paths.
    pub fn groups(&mut self) -> Vec<Vec<u32>> {
        let n = self.parent.len();
        let mut by_root: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for i in 0..n as u32 {
            let r = self.find(i);
            by_root.entry(r).or_default().push(i);
        }
        by_root.into_values().filter(|s| s.len() > 1).collect()
    }
}
