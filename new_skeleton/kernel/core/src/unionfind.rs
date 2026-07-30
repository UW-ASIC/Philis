//! Union-find over device indices — how a rule's `extract` coalesces a set of
//! devices that constrain a set of devices into one addressable group.
//!
//! When a rule relates group↔group (e.g. two multi-finger halves of a diff
//! pair), it `union`s each side's devices; the annotator later reads the
//! resulting sets as `Target::Group`s and as the [`crate::Layout`] group table.

/// Disjoint-set forest with path compression + union by rank.
pub struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
}

impl UnionFind {
    /// A forest of `n` singletons.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self { parent: (0..n as u32).collect(), rank: vec![0; n] }
    }

    /// Representative of `x`'s set (path-compressed).
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

    /// Merge the sets of `a` and `b`.
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

    /// The non-singleton sets, each as a sorted member list. Singletons (devices
    /// in no group) are omitted — they stay addressable as `Target::Device`.
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
