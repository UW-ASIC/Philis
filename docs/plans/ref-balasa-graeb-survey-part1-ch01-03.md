# Analog Layout Synthesis survey (Graeb ed., 2011), Part I: topological placement, chapters 1–3

Source: H. E. Graeb (ed.), *Analog Layout Synthesis: A Survey of Topological Approaches*, Springer 2011. The range covers the front matter, ch. 1 (F. Balasa, "Device-Level Topological Placement with Symmetry Constraints"), ch. 2 (M. P.-H. Lin and Y.-W. Chang, "Hierarchical Placement with Layout Constraints") and ch. 3 (M. Strasser, M. Eick, H. Graeb and U. Schlichtmann, "Deterministic Analog Placement by Enhanced Shape Functions").
Reftext: `scratchpad/reftext/balasa_graeb_survey.txt`, lines 1–8167, read in full. PDF page = book page + 14 (book p. 3 = PDF p. 17). Every PDF page cited below comes from form-feed counting.
PDF pages opened to fix garbled equations: p. 50 (Lemma 1), p. 98 (eq. 2.5), pp. 130–131 (eqs. 3.26–3.28, VCG inequalities) and pp. 136–137 (eqs. 3.37–3.43).

Codebase evidence comes from a quick grep of the working tree as of 2026-09-28. It is not an audit.

---

## 1. Coverage

### 1.1 Read chunks (Read tool, offset..end)
1. 1..1000
2. 1001..1900
3. 1901..2800
4. 2801..3700
5. 3701..4600
6. 4601..5500
7. 5501..6400
8. 6401..7300
9. 7301..8167

No lines were skipped. Lines 8165–8167 are the "Part III / Chapter 4" heading, where the next assignment begins. Lines 4523–4524 ("Part II Routing") are an extraction artifact placed before chapter 3; chapter 3 belongs to Part I (see the Contents, L433–445).

### 1.2 Every heading in the range (reftext line)
- Title/copyright L1–43; Preface L44–169; The Authors L170–427; Contents L428–468; Part I Placement L469–470
- **Ch. 1 Device-Level Topological Placement with Symmetry Constraints** L471; Abstract L480
  - 1.1 Introduction L501; 1.1.1 CAD for Analog Layout L503; 1.1.2 The Device-Level Analog Placement Problem L574; 1.1.3 Overview of Analog Placement Methods L594; 1.1.4 Placement for Layout Symmetry L647; 1.1.5 The Absolute Representation of the Layout L726; 1.1.6 Topological Representations of the Layout L772; 1.1.7 Selecting a Topological Representation for Analog Placement L834
  - 1.2 Data Structures for Rectilinear Border Contours L895; 1.2.1 Segment Trees L907; 1.2.2 Red–Black Interval Trees L1174; 1.2.3 Deterministic Skip Lists L1427; 1.2.3.1 Insertion and Deletion of a Key in a 1-3 DSL L1507; 1.2.3.2 Implementation Aspects of 1-3 DSL's L1573; 1.2.3.3 Algorithm Computing the Border Contour of the Layout L1633; 1.2.4 Johnson's Priority Queue L1770
  - 1.3 Symmetric-Feasible Sequence-Pairs L1811; 1.3.1 Evaluation of Symmetric-Feasible Sequence-Pairs L1947; 1.3.1.1 The Computation of the Device Ordinates L1993; 1.3.1.2 The Computation of the Device Abscissae L2102; 1.3.2 Handling Multiple Symmetry Groups L2327; 1.3.3 The Design of the Move Set L2375
  - 1.4 Topological Placement with Symmetry Constraints Using Other Layout Representations L2409; 1.4.1 A Comparative Overlook on Transitive Closure Graphs L2412; 1.4.2 A Comparative Overlook on Tree Representations of the Layout L2456; 1.4.2.1 Symmetric-Feasible Binary Trees L2550; 1.4.2.2 The Design of the Move Set L2600
  - 1.5 Experimental Results L2731; 1.6 Conclusions L2821; References L2833–2974
- **Ch. 2 Hierarchical Placement with Layout Constraints** L2975; Abstract L2983
  - 2.1 Introduction L2994
  - 2.2 Preliminaries L3088; 2.2.1 Symmetry Constraints L3090; 2.2.2 Symmetry Island L3156; 2.2.3 Review of B*-Trees L3218
  - 2.3 Placement of a Symmetry Group L3254; 2.3.1 Automatically Symmetric-Feasible B*-tree L3256; 2.3.2 ASF-B*-Tree Packing L3454
  - 2.4 The Hierarchical Framework L3507; 2.4.1 Hierarchical HB*-Tree L3509; 2.4.2 HB*-Tree with Rectilinear Symmetry Islands L3522; 2.4.3 HB*-Tree Packing L3628
  - 2.5 The Algorithm L3738; 2.5.1 HB*-Tree Perturbation L3751; 2.5.2 ASF-B*-Tree Perturbation L3767; 2.5.2.1 Module Rotation L3788; 2.5.2.2 Node Movement L3801; 2.5.2.3 Node Swapping L3814; 2.5.2.4 Representative Change L3831; 2.5.2.5 Symmetry-Type Conversion L3850; 2.5.3 Contour Node Related Updates L3920
  - 2.6 Comparisons with Other Approaches L3968; 2.6.1 Comparisons of Time Complexities L3979; 2.6.2 Comparisons of Experimental Results L4046
  - 2.7 Advanced Symmetry Constraints L4156; 2.7.1 Multiple Symmetry-Group Alignment L4164; 2.7.2 Consideration of NonSymmetry-Island Placements L4234
  - 2.8 Hierarchical Constraints L4359; 2.8.1 Hierarchical Symmetry L4361; 2.8.2 Hierarchical Clustering/Proximity L4376
  - 2.9 Conclusion L4422; References L4434–4522
- **Ch. 3 Deterministic Analog Placement by Enhanced Shape Functions** L4525; Abstract L4534
  - 3.1 Introduction L4549; 3.1.1 Definitions L4573; 3.1.2 Analog Circuit Placement Requirements L4668; 3.1.3 Context of This Work L4874; 3.1.4 Contributions L4943
  - 3.2 Placement Constraint Generation L5013; 3.2.1 Placement Requirements L5027; 3.2.1.1 Types of Placement Requirements L5029; 3.2.1.2 Importance Order L5078; 3.2.2 SMP Graph and Its Generation L5141; 3.2.2.1 Building Block Recognition L5208; 3.2.2.2 Symmetry Analysis L5230; 3.2.3 HSMPG Tree and Its Generation L5355; 3.2.3.1 Generation Algorithm L5369; 3.2.3.2 Example L5409; 3.2.3.3 Discussion L5426; 3.2.4 Constraint Generation L5613
  - 3.3 B*-Tree Placement Considering Linear and Piecewise-Linear Constraints L5655; 3.3.1 Linear Constraint Handling L5741; 3.3.2 Piecewise-Linear Constraint Handling L5940
  - 3.4 Enhanced Shape Functions L6034; 3.4.1 Review of Shape Functions L6043; 3.4.2 Definition of Enhanced Shape Functions L6104; 3.4.3 Combination of Enhanced Shape Functions L6129 (Horizontal Addition L6176; Vertical Addition L6212)
  - 3.5 Hierarchically Guided Enumeration L6338
  - 3.6 Experimental Results L6586; 3.6.1 Discussion of the Presented Approach L6602 (Example 3 L6676; Example 4 L6786; Example 5 L7027); 3.6.2 Comparison with Other Approaches L7417; 3.6.3 Experiment with Linear Minimum Distance Constraints L7733; 3.6.4 Experiment with PWL Minimum Distance Constraints L7995
  - 3.7 Conclusion L8009; References L8038–8164

---

## 2. Section-by-section digest

### Front matter
- **Preface** (L44–169, PDF pp. 4–6). The preface opens with an EDA Weekly (2005) statistic: analog appears on 75% of chips, takes 40% of design effort and causes 50% of first-silicon errors (L49–51). Industrial analog layout synthesis is "still in its infancy" (L56–61). Ch. 1 restricts search to the symmetry-feasible fraction of topological codes (L87–98). Ch. 2 builds the HB*-tree on hierarchical clustering by model, function or signal flow (L99–108). Ch. 3 derives the hierarchy and the symmetry, proximity and matching constraints from the netlist, then enumerates deterministically with enhanced shape functions and has no tuning parameter (L109–121).
- **The Authors / Contents** (L170–468). These give affiliations and page numbers only. Ch. 1 is on book p. 3, ch. 2 on p. 61, ch. 3 on p. 95 (L435–445).

### Chapter 1 (Balasa)
- **Abstract** (L480–498, PDF p. 17)
  - The traditional method is a stochastic search over flat (absolute) coordinates. This chapter instead uses non-slicing topological codes, restricted to "symmetric-feasible" sequence pairs or trees.
  - The topological approaches run much faster at similar quality.
- **1.1.1 CAD for Analog Layout** (L503–570, PDF pp. 17–19)
  - The chapter separates block-level layout synthesis (sized schematic to mask) from system-level assembly and addresses block-level placement (L548–556).
  - Optimization-based P&R is compared with procedural module generators (precoded layouts) and template-driven methods (L557–570). Optimization is general and flexible, but its quality depends on the cost function and on a *complete* constraint set (L566–570).
- **1.1.2 The Device-Level Analog Placement Problem** (L574–590, PDF p. 19)
  - Deciding whether fixed-orientation rectangles pack into a W×H chip is NP-complete, and minimum-area packing is NP-hard (L576–579).
  - An analog placer additionally needs (1) symmetry and matching constraints, (2) device merging (geometry sharing) to cut area and parasitics, and (3) a library of module generators whose reshaping the placer can use (L583–590).
- **1.1.3 Overview of Analog Placement Methods** (L594–643, PDF pp. 19–20)
  - Constructive placement (expert base, schematic-driven) is fast but depends on the order in which devices are selected (L596–606).
  - Constrained combinatorial optimization first used nonquantitative hard/soft constraints and later became performance-driven with estimation models (L612–617).
  - SA and GA are the usual engines (ILAC, KOAN/ANAGRAM II, PUPPY-A, LAYLA). A GA+SA hybrid exists, as does LP on sequence-pair constraint graphs inside SA [21] (L618–637).
- **1.1.4 Placement for Layout Symmetry** (L647–722, PDF pp. 20–22)
  - Symmetry matches layout-induced parasitics between halves. Mismatch in those parasitics raises offset and degrades PSRR (L649–660).
  - For thermal symmetry, place thermally sensitive pairs symmetrically about radiating devices so both see the same temperature (L661–670).
  - Three symmetry types (L676–712):
    - Mirror: mirrored orientation, which permits mirror-symmetric routing.
    - Perfect: identical orientation, for anisotropic disturbances such as oblique implant. It needs parasitic-matched routing that is not geometrically symmetric.
    - Self-symmetric: a device on the axis, e.g. a bias device connected to both halves, or a thermal centre.
  - A symmetry group shares one axis. Pair equations use left-bottom corners: (x_i + w_i) + x_j = 2·x_axis and y_i = y_j. A self-symmetric device satisfies x_i + w_i/2 = x_axis. Axes are vertical by convention (L714–722).
- **1.1.5 The Absolute Representation** (L726–768, PDF pp. 22–23)
  - Moves are translation, rotation and mirror. Illegal overlap is allowed and penalized, typically quadratically, driven to zero (L739–744).
  - Advantages: symmetry and matching are easy to maintain, and beneficial overlaps (merging) can be explored (L744–749; footnote 2, L783–785).
  - Drawbacks: slow convergence over a large space, and nonzero overlap at the end forces a post-processing step. The overlap weight is hard to tune: too large blocks the search, too small lets cells collapse (L750–762).
- **1.1.6 Topological Representations** (L772–831, PDF pp. 23–24)
  - Slicing (Polish expressions) cannot reach many topologies and packs badly when cell sizes differ, which is typical in analog. It also needs virtual symmetry axes in the cost (L777–803).
  - Sequence pair (SP, Murata): O(n²) evaluation via constraint graphs; LCS evaluation O(n log log n) (Tang/Wong) (L804–814).
  - BSG evaluates in O(n²) (L814–817). O-tree reduces redundancy; B*-tree and Balasa's binary tree are equivalent to it (L818–823). CBL encodes mosaic floorplans with zero dead space (L824–825). TCG uses two transitive-closure graphs (L826–831).
- **1.1.7 Selecting a Topological Representation** (L834–891, PDF pp. 25–26)
  - The usual criteria are low code redundancy and fast (ideally linear) evaluation (L840–845).
  - In analog, most codes are symmetry-infeasible. Discarding infeasible codes during SA was "extremely ineffective" (L854–862).
  - A representation therefore needs a property that characterizes feasible codes without building the layout, plus moves that stay inside that feasible subspace (L863–877).
  - The O-tree symmetry evaluation in [44] is quadratic, against linear evaluation without symmetry (L883–888).
- **1.2 Data Structures for Rectilinear Border Contours** (L895–903, PDF p. 26)
  - Given y-coordinates and a topological sort of the horizontal constraint graph (HCG), these structures compute x-coordinates independently of the representation.
- **1.2.1 Segment Trees** (L907–1171, PDF pp. 26–32)
  - A segment tree stores intervals over normalized integer endpoints; its depth is ⌈log2(r−l)⌉ (L909–946).
  - The algorithm sorts and unifies S = ∪{y_i, y_i+h_i} into ranks, then processes cells in HCG topological order. UpdateSegmentTree sets x_i = max v.x over the standard nodes covering [a_i, b_i]; UpdateRightContour then sets them to x_i + w_i. Width W = max v.x. Complexity is O(n log n) (L994–1069).
  - Worked example: nine blocks, S of size 7, depth drops from 7 to 3 after normalization, and the result is x = [0 0 0 50 70 90 90 110 110] with W = 150 (L1070–1101).
  - Building the tree once over [0, n] and only re-initializing it per SA iteration avoids a 15–20% runtime penalty (L1108–1171).
- **1.2.2 Red–Black Interval Trees** (L1174–1423, PDF pp. 32–39)
  - Disjoint intervals partition [0, H] and are kept in a red–black BST.
  - UpdateRedBlackTree handles the interval trichotomy cases (a), (b), (c1)–(c4) and merges adjacent intervals with equal abscissae (L1248–1300).
  - Amortized cost is O(log n) per update (a node is deleted at most once per creation), O(n log n) overall, with O(n) space and ≤2n−1 nodes (L1318–1345).
  - Ten-block example: W = 15 (L1347–1411). RB trees need ≤2 rotations per insert and ≤3 per delete, against Θ(log n) for AVL (L1414–1423).
- **1.2.3 Deterministic Skip Lists** (L1427–1504, PDF pp. 39–40)
  - The probabilistic skip list averages O(log n) but has Θ(n log n) space and Θ(n) time in the worst case, and depends on the RNG (L1429–1485).
  - A DSL with gap sizes ⌈m/2⌉−1 … m−1 maps one-to-one onto B-trees of order m. The 1-3 DSL is the simplest (L1486–1504).
- **1.2.3.1 1-3 DSL insert/delete** (L1507–1570, PDF pp. 40–42)
  - Insertion is top-down: split any size-3 gap by raising its middle element.
  - Deletion is top-down: merge or borrow on size-1 gaps. The scheme generalizes to k−(2k+1) DSLs.
- **1.2.3.2 Implementation aspects** (L1573–1631, PDF pp. 42–43)
  - The linked-list implementation (down/right links, sentinels) avoids an O(log² n) promotion cost.
  - Footnote 8: "Analog circuits seldom contain more than 100 cells per hierarchical level" (L1604–1605).
- **1.2.3.3 Border contour with DSL** (L1633–1767, PDF pp. 43–47)
  - Keys are the y-breakpoints of the right contour. UpdateDSL takes x_i = max x over keys in [q.key, b), removes keys inside (a, b), inserts a and b, and merges collinear segments.
  - Complexity is O(n log n) amortized with ≤2n+7 nodes. Example (L1707–1744).
- **1.2.4 Johnson's Priority Queue** (L1770–1807, PDF pp. 47–48)
  - Buckets hang on the leaves of a host binary tree of height ⌈log2(N+1)⌉. Insert and delete cost O(log log N), so contour evaluation is O(n log log n).
- **1.3 Symmetric-Feasible Sequence-Pairs** (L1811–1944, PDF pp. 48–50)
  - SP semantics: A precedes B in both α and β means A is left of B. A before B in α but after B in β means A is above B (L1826–1838).
  - Condition (1.1) and its sufficiency: the condition is sufficient but not necessary, and counterexamples are rare and depend on cell dimensions (L1856–1908).
  - Lemma 1 upper bound. Example: 35,280 of 25,401,600 codes remain, a 99.86% reduction (L1911–1944).
- **1.3.1 Evaluation of S-F SPs** (L1947–1990, PDF pp. 51–52)
  - Maximal common subsequences of (α, β) are HCG paths; the width-weighted LCS is the placement width. Common subsequences of (α^R, β) give the VCG.
  - Buckets hold (index in β, LCS length). Any contour structure from §1.2 can replace the queue.
- **1.3.1.1 Ordinates, Step 1y** (L1993–2099, PDF pp. 52–54)
  - Step 1y runs LCS over reversed α and enforces equal y for symmetric partners, restarting when a raised y breaks a vertical constraint. Example 1 (L2021–2028).
  - (1.1) guarantees termination. Height is minimal because y is raised bottom-up by the minimum amount.
  - Worst case is Θ(p) repeats, O(p·n log log n); in practice at most 3 passes (L2080–2099).
- **1.3.1.2 Abscissae, Steps 1x–4x** (L2102–2323, PDF pp. 54–60)
  - Step 1x: plain LCS abscissae (L2104–2119).
  - Step 2x: build the embedding DAG of pairs from the right contour and place the axis at x_sym = max over leaf pairs (B_j, B_k) of (x_j + x_k + w_k)/2. Shift leaves by d, and give inner nodes d = min over children (L2129–2209). Cost is O((p+s) log log H), or O(H + n log log n) with radix-sort normalization (L2210–2250).
  - Step 3x: sweep right and push the right member of each pair by d = 2x_sym − x_j − (x_k + w_k) (L2251–2256).
  - Step 4x: sweep left from the sentinel (n+1, W) and pull left members (L2257–2277).
  - All steps are O(n log log n). The symmetry group's x-span is minimal relative to the topological constraints (L2278–2323).
- **1.3.2 Multiple Symmetry Groups** (L2327–2371, PDF pp. 60–61)
  - Groups must not block each other (the interleaving pattern at L2331–2335). The simplest guard is to keep groups from intermingling in α and β.
  - Embedded groups are allowed when the outer group's cells surround the inner group's cells in both sequences (example with 3 groups, L2342–2358).
  - Groups are processed one at a time and then frozen, inner before outer (ordered by an embedding DAG); cost O(G·n log log n). A non-S-F code can loop forever (L2359–2371).
- **1.3.3 Move Set** (L2375–2405, PDF pp. 61–62)
  - Start from an S-F code and use only moves that preserve (1.1): paired interchange in β, paired relocation, and paired rotation/mirror. Asymmetric cells move freely.
  - Whole-group moves run with low, temperature-decreasing probability.
- **1.4.1 TCG** (L2412–2452, PDF pp. 62–63)
  - TCG and SP are equivalent. β is the topological sort of C_h ∪ C_v; α is the same with C_v reversed.
  - TCG→SP costs O(n log n) and SP→TCG costs Θ(n²). Both are P-admissible with (n!)² codes. TCG-S evaluation is quadratic, so S-F SP is faster.
- **1.4.2 Tree representations** (L2456–2547, PDF pp. 63–65)
  - O-tree code (T, π): T is a 2n-bit string (L2459–2465). The O-tree↔binary-tree bijection is at L2466–2492.
  - Code count is b_n·n! with b_n the Catalan number. Lemma 2: trees < SPs for n ≥ 3 (L2493–2523).
  - When symmetry groups dominate, S-F SPs are fewer than trees (L2526–2533).
  - The O-tree approach in [44] detects infeasibility with graph cycle tests, which is quadratic (L2534–2539).
- **1.4.2.1 S-F Binary Trees** (L2550–2597, PDF pp. 65–66)
  - Positioning rules for binary trees (L2552–2559). The (inorder, preorder) pair determines the tree (L2560–2575).
  - Condition (1.2), plus the inter-group non-blocking exclusion (L2576–2597).
- **1.4.2.2 Move Set** (L2600–2727, PDF pp. 66–69)
  - Two complete move sets: Stasheff rotation ((S1S2)S3) → (S1(S2S3)) plus label swaps, and the E-N swap on Dyck paths plus label swaps (L2613–2683).
  - Symmetry-preserving variants (L2686–2727):
    - Interchange paired with swapping the partners' symmetric counterparts; cross-group swaps only between parent and child nodes.
    - Paired E-N swaps, O(n).
    - Whole-group subtree rebuild, O(n²), with low probability.
    - Orientation changes that respect mirror, perfect or self symmetry.
- **1.5 Experimental Results** (L2731–2817, PDF pp. 69–71)
  - The C++ SA tool shares one cost and schedule across representations. It handles systematic mismatch, alignment and soft-cell (capacitor) aspect ranges (L2733–2746).
  - Table 1.1 covers 17–116 cells in 0.2–21 min (L2763–2777).
  - The priority-queue evaluation is fastest and segment trees are slowest; RB trees and DSL are nearly as fast (L2786–2791).
  - S-F binary trees are faster but give poorer quality than S-F SPs (L2794–2802).
  - Topological S-F search beats absolute representation in CPU and sometimes quality, and tuning is easier (L2812–2817).
- **1.6 Conclusions** (L2821–2829, PDF p. 71). The chapter's conclusion: take symmetry into account directly during exploration.
- **References** (L2833–2974). Key ones: [21] Kouda SP+LP (L2878); [29] SP (L2898); [31] FAST-SP (L2904); [35] B*-tree (L2913); [43] Balasa–Lampaert S-F SP (L2928); [45] S-F trees (L2933).

### Chapter 2 (Lin, Chang)
- **Abstract / 2.1 Introduction** (L2983–3084, PDF pp. 74–77)
  - Basic constraints (L2996–3009):
    - Common centroid, for current mirrors and diff pairs against process mismatch.
    - Symmetry, for differential subcircuits.
    - Proximity, for a common model or function, so the subcircuit can share a substrate/well region or one guard ring. A proximity outline may be rectilinear.
  - Hierarchical symmetry and hierarchical proximity come from the design hierarchy, which is either exact (the circuit hierarchy) or virtual (clusters by model, function or constraint) (L3036–3050).
  - Simultaneous hierarchical optimization is preferred to bottom-up integration, because a subcircuit's local optimum may not be globally optimal (L3069–3083).
- **2.2.1 Symmetry Constraints** (L3090–3152, PDF pp. 77–79)
  - A symmetry group holds p pairs and q self-symmetric modules. Its axis is vertical or horizontal (two symmetry types) (L3092–3133).
  - Centre-coordinate equations (2.1) (vertical) and (2.2) (horizontal) (L3143–3152).
- **2.2.2 Symmetry Island** (L3156–3215, PDF pp. 79–80)
  - Pelgrom (2.3): σ²(ΔP) = A_P²/(WL) + S_P²·D_x². Farther pairs match worse, so symmetric devices should be adjacent (L3158–3183).
  - Definition 2.1: every module abuts at least one other module of the group, and the group is connected (L3187–3193).
- **2.2.3 Review of B*-Trees** (L3218–3250, PDF pp. 80–81)
  - Node semantics: the left child is the lowest adjacent module to the right; the right child is the first module above at the same x.
  - Preorder packing with a doubly linked contour runs in linear time.
- **2.3.1 ASF-B*-tree** (L3256–3451, PDF pp. 81–85)
  - Representatives (Def. 2.2/2.3) and the representative B*-tree (Def. 2.4). Mirrored placement (Def. 2.5) and symmetric feasibility (Def. 2.6).
  - Lemma 2.1: a self-symmetric representative abuts the axis. Lemma 2.2: pair representatives are always feasible.
  - Property 2.1: self-symmetric representatives lie on the rightmost branch (vertical axis) or the leftmost branch (horizontal axis). Def. 2.7 defines the ASF-B*-tree.
  - Theorems: 2.1 (feasible), 2.2 (packing gives an island), 2.3 (unique correspondence).
- **2.3.2 ASF-B*-Tree Packing** (L3454–3503, PDF pp. 85–86)
  - Preorder packing keeps both horizontal and vertical contours, then mirrors the representatives.
  - The island's bottom contour comes from the convex points of the dual vertical contours (Fig. 2.11).
- **2.4.1 HB*-Tree** (L3509–3519, PDF p. 86)
  - A hierarchy node holds one ASF-B*-tree per symmetry group. Islands and non-symmetric modules are optimized simultaneously.
- **2.4.2 Rectilinear islands** (L3522–3625, PDF pp. 86–89)
  - Slicing rectilinear modules into sub-blocks [33] is hard to maintain. Contour nodes instead represent the island's top contour segments.
  - Property 2.2 gives six structural rules, with proofs.
- **2.4.3 HB*-Tree Packing** (L3628–3734, PDF pp. 89–91)
  - Pack the ASF-B*-tree, fit the island's bottom contour against the current contour (minimizing dead space), then pack the contour nodes.
  - Reserve well/guard-ring white space when adjacent device types differ (L3685–3691).
  - Theorem 2.4: packing runs in O(n) (L3694–3734).
- **2.5 Algorithm** (L3738–3748, PDF p. 91). SA with cost (2.4), Φ = α·A_P + β·HPWL.
- **2.5.1 HB*-Tree Perturbation** (L3751–3764, PDF p. 91)
  - Op1 rotate, Op2 move, Op3 swap.
  - Hierarchy nodes are chosen less often, to avoid big jumps.
  - Two placements are forbidden: a node as the right child of a hierarchy node, or as the left child of a contour node. Contour nodes move only with their hierarchy node.
- **2.5.2 ASF-B*-Tree Perturbation** (L3767–3774). Op4 changes a representative; Op5 converts the symmetry type. Property 2.1 must hold after every move.
- **2.5.2.1 Rotation** (L3788–3798, PDF p. 92). Rotating a pair rotates both members. Rotating a self-symmetric module reshapes its representative.
- **2.5.2.2 Node Movement** (L3801–3811). A pair representative can move anywhere. A self-symmetric representative moves only along the rightmost (leftmost) branch.
- **2.5.2.3 Node Swapping** (L3814–3828, PDF pp. 92–93). Two pair representatives can swap freely. If a self-symmetric representative is involved, the other node must be on the same branch.
- **2.5.2.4 Representative Change** (L3831–3847, PDF p. 93)
  - Flip which pair member is the representative, or flip the self-symmetric representative.
  - This keeps area and changes only wirelength, in O(1).
- **2.5.2.5 Symmetry-Type Conversion** (L3850–3916, PDF pp. 93–94)
  - Rotate every module and swap left/right children.
  - It is rarely used, because the axis direction is usually predefined by power/ground lines or signal flow (L3915–3916).
- **2.5.3 Contour Node Related Updates** (L3920–3964, PDF pp. 94–96)
  - When an island loses top segments, dangling nodes attach to the nearest contour node: as its right child if that slot is free, else as the left child of the leftmost-skewed descendant.
  - Amortized O(1). Example in Fig. 2.20.
- **2.6.1 Complexity comparison** (L3979–4042, PDF pp. 96–97)
  - Table 2.2 gives perturbation and packing times for 12 approaches. ASF-B*+HB* has perturbation O(lg n) and packing O(n), the fastest. SP+LP and B*+ESF+LP have Ω(n²) packing.
- **2.6.2 Experimental results** (L4046–4152, PDF pp. 97–100)
  - Benchmarks: MCNC (apte, hp, ami33, ami49) plus two industrial circuits (Tables 2.3/2.4).
  - Initial temperature T0 = −Δ_avg/ln P, cooling 0.9, 20,000 iterations per temperature, left-skewed initial tree (L4074–4081).
  - Tables 2.5/2.6 (area and time) are quoted in BAL1-37.
- **2.7.1 Multi-group alignment** (L4164–4230, PDF pp. 100–102)
  - Insert a zero-width (zero-height) dummy block beside each island and make it the island's parent node. Adjusting the dummy's size aligns the axes.
- **2.7.2 Non-island placements** (L4234–4249, PDF p. 102)
  - Pack non-symmetric modules into a cluster, then treat the cluster as a self-symmetric member or pair two clusters as a symmetry pair (Fig. 2.24).
- **2.8.1 Hierarchical Symmetry** (L4361–4372, PDF p. 105)
  - A group may contain self-symmetric sub-groups and group pairs. Mixed ASF-B*/HB*-trees yield nested islands.
- **2.8.2 Hierarchical Clustering/Proximity** (L4376–4418, PDF pp. 105–106)
  - Clusters and super-clusters must each be placed connected.
  - Each cluster has its own HB*-tree; a move first selects a tree.
  - The framework can host other placers (CBL, grid-based common centroid [24], signal-flow [17]).
- **2.9 Conclusion** (L4422–4430). Linear-time packing, with pruning of non-island placements. **References** L4434–4522.

### Chapter 3 (Strasser, Eick, Graeb, Schlichtmann)
- **Abstract / 3.1 Introduction** (L4534–4561, PDF pp. 109–110)
  - Automatic constraint generation (building blocks + symmetry, with prioritized rules) feeds Plantage.
  - Plantage enumerates guided by the hierarchy and returns a Pareto front of aspect ratios.
- **3.1.1 Definitions** (L4573–4664, PDF pp. 111–112)
  - Device vs module (a device = 1..k modules); COG (3.1)–(3.3); distance d(m,n) = min(d_hor, d_vert) of COG gaps (3.4)–(3.6).
  - Groups (3.7)–(3.11), with each module in exactly one group. Area-weighted group COG (3.12).
  - Module variants (Def. 3.7), e.g. finger counts.
- **3.1.2 Placement Requirements** (L4668–4870, PDF pp. 112–116)
  - Variant constraints, e.g. equal finger counts in a diff pair.
  - Device proximity.
  - Symmetry, centre form: (3.13) and (3.14).
  - Common centroid (3.15), with the 16-transistor example.
  - Linear minimum distance for wells and guard rings: (3.16) and (3.17).
  - Piecewise-linear minimum distance for DTI (3.18).
  - Symmetric routing requires symmetric placement (L4864–4867).
- **3.1.3 Context** (L4874–4930, PDF pp. 116–118)
  - Constraint extraction in prior work used sensitivity analysis [5–7], net susceptibility classes [8], and structural symmetry via graph labeling or recursive pair detection [9–12]. The sizing-rules library [13] also exists.
  - Absolute placers search R^{2N}. The topological list covers O-tree, B*, H/ASF-B*, SP, BSG, CBL and TCG-S.
  - B*-tree convention used in ch. 3: the left child is placed above, the right child to the right, and overlapping y-projections order left-to-right by preorder (L4908–4918).
- **3.1.4 Contributions** (L4943–5009, PDF pp. 118–119)
  - The hierarchy tree has devices as leaves. A complete B*-tree enumeration is infeasible, so enumeration is bounded by hierarchy.
  - Enhanced-shape sum; Pareto set of aspect ratios; deterministic and parallelizable; variant selection built into the enumeration.
- **3.2 / 3.2.1.1 Requirement types** (L5013–5075, PDF pp. 119–120)
  - Flow (Fig. 3.6): netlist → symmetry analysis and building-block recognition → SMP graph → HSMPG tree → constraints.
  - T = {M_B, M_S, S, P_B, P_N} (Table 3.1).
- **3.2.1.2 Importance Order** (L5078–5137, PDF pp. 120–121)
  - M_S ≻ M_B ≻ P_B ≻ S ≻ P_N (3.19).
  - Rationale: breaking a symmetric pair's matching degrades offset (critical), while breaking in-block matching equally on both halves degrades gain (less critical). Block proximity never harms symmetry, because symmetry affects whole blocks.
- **3.2.2 SMP Graph** (L5141–5157, PDF p. 122)
  - The SMP graph is a multigraph over devices, with typed edges.
  - It is initialized with a clique per net (P_N).
- **3.2.2.1 Building Block Recognition** (L5208–5227, PDF p. 123)
  - Subgraph isomorphism against the library of [13].
  - dp, level shifter, simple CM: match T1–T2. Cascode, four-transistor and wide-swing CMs: match T1–T2 and T3–T4, plus proximity T1–T3 and T2–T4 (Fig. 3.11).
- **3.2.2.2 Symmetry Analysis** (L5230–5333, PDF pp. 123–126)
  - A symmetry compound is the set of pairs sharing an axis. Each pair gets an M_S edge; the compound gets an S clique, formed by eliminating the axis coordinate.
  - Example: 4 pairs, 1 compound.
- **3.2.3 HSMPG Tree** (L5355–5365, PDF p. 126). Group types: PG, MG, SG.
- **3.2.3.1 Generation Algorithm** (L5369–5406, PDF pp. 126–127)
  - Algorithm 3.1: for each type τ in importance order, filter the edges, find connected components (|N| > 1), create a group, and contract it to a super node.
  - Group type: MG for M_B/M_S, SG for S, PG otherwise.
- **3.2.3.2 Example** (L5409–5423)
  - Produces MG_S,1–4, then MG_B,1–2, then SG1, then PG_N,1, which covers the whole circuit.
- **3.2.3.3 Discussion** (L5426–5610, PDF pp. 127–129)
  - The static order was correct in all experiments.
  - A dynamic priority Φ: E_SMP → ℕ from simulated violation impact is possible.
- **3.2.4 Constraint Generation** (L5613–5652, PDF pp. 129–130)
  - MG → same variant + alignment, or common centroid when devices are split into sub-devices.
  - SG → "symmetry (pair)" y-equalities (3.23) and "symmetry (groups)" x-centre alignment (3.24)–(3.25).
  - PG → no explicit constraint, because bottom-up construction implies proximity.
- **3.3 B*-tree → placement** (L5655–5737, PDF pp. 130–131)
  - HCG and VCG as difference relations. Algorithm 3.2 builds the VCG from the B*-tree (y_i + h_i ≤ y_j per edge).
  - The LP/MIP pipeline is shown in Fig. 3.17.
- **3.3.1 Linear constraints** (L5741–5936, PDF pp. 131–135)
  - LP (3.26)–(3.28): minimize y_e subject to M_v·y ≥ d_v and C_v·y = k_v.
  - The shadow-based HCG construction (core shadow 3.30; partial shadow 3.31–3.32; Algorithm 3.3; example Fig. 3.21) adds only the edges needed for minimum distances.
  - LP (3.33)–(3.35) on x is solved by simplex.
- **3.3.2 PWL constraints** (L5940–6030, PDF pp. 135–137)
  - The DTI distance set is non-convex, so a MIP is required. A binary r_e per edge selects the range, with big-M β. Equations (3.36)–(3.43).
  - β > Σ module widths + worst-case min distances (×(#symmetry constraints + 1)). λ < w_min/(N·d_DTI).
- **3.4.1 Shape functions** (L6043–6100, PDF pp. 137–138)
  - Horizontal addition gives (w1+w2, max h); vertical addition gives (max w, h1+h2). Dominated shapes are pruned. The result is a Pareto front, and a slicing structure.
- **3.4.2 Enhanced shape functions** (L6104–6125, PDF pp. 138–139)
  - An enhanced shape is (w, h, B*-tree).
  - It is dominated if its h is higher at ≤ w, or its netlength is higher at equal (w, h).
- **3.4.3 ESF combination** (L6129–6335, PDF pp. 139–142)
  - Horizontal addition attaches β's root to α's lowest-rightmost node. Vertical addition splits β into baseline segments and attaches each to the α node that shadows it (contour-based), left to right.
  - Both preserve in/preorder relations, hence constraint feasibility (3.44)–(3.49).
- **3.5 Hierarchically Guided Enumeration** (L6338–6583, PDF pp. 143–146)
  - B*-tree counts: 336 for 4 modules, 57,657,600 for 8.
  - Basic enumeration: all B*-trees × all allowed variant combinations per basic group, parallelizable.
  - Parents combine child ESFs in every order; ESF addition is not commutative (footnote 1, L6504–6505). Algorithm 3.4.
- **3.6.1 Results** (L6602–7163, PDF pp. 147–152)
  - Table 3.2: 5 circuits, 9–42 devices, 2–7 variants per module, groups of 2–14 (average 2.2–2.9).
  - Best area usage 110–129%; Plantage runtime 1–691 s; constraint generation 0.3–1.2 s.
  - The n/p well distance dominates the area overhead. Details for examples 3–5.
- **3.6.2 Comparison** (L7417–7729, PDF pp. 153–155). Table 3.4, with area usage in % of module area.
- **3.6.3 Linear min-distance experiment** (L7733–7748, PDF p. 155)
  - Each symmetry group sits in its own well: d_well to the outside, 2·d_well between wells.
  - CPU rises 76% (593 s) and 72% (664 s). Area usage 107.74% and 109.24% (Fig. 3.42/3.43 captions, L7863, L7960).
- **3.6.4 PWL experiment** (L7995–8005, PDF p. 157). 30 DTI modules, about 15 min (MIP much slower than simplex), area usage 110%.
- **3.7 Conclusion** (L8009–8034, PDF pp. 157–158). The first placer to handle proximity, symmetry, CC, minimum distance and variant constraints deterministically. **References** L8038–8164.

---

## 3. Actionable extraction

### Current Philis baseline (for context)
- gp is momentum gradient descent on HPWL plus density plus analog cost (`backend/gp/src/lib.rs:1-3`).
- dp is Metropolis SA over **absolute coordinates** with displacement, swap, rotate, reshape and DTI-branch moves. Hard equalities are repaired by projection (`backend/dp/src/lib.rs:1-8`, `:303-340`, `:371`). A terminal O(n²) push-apart legalizer follows (`backend/dp/src/legalize.rs:1-16`).
- Symmetry is vertical-axis only, centre form (`kernel/analog/src/placement/symmetry.rs:10-16`), with one axis per top-level annotator block ("stage") (`backend/annotator/src/emit.rs:3-16`).
- Spacing is one global `clearance`: the max min-spacing over the nwell/diff/tap/poly/implant/li layers (`frontend/library/src/lib.rs:529-537`).
- There is no sequence pair, B*-tree, contour, shape function or LP anywhere in `kernel/`, `backend/` or `frontend/` (grep: no hits for `sequence.pair|B\*|contour|shape.function|pareto|simplex`).

### BAL1-01 Symmetry equations: corner form, centre form, vertical and horizontal axis
- Kind: formula
- Statement:
  - Corner form (ch. 1): for pair (B_i, B_j) in group k, (x_i + w_i) + x_j = 2·x_symAxis,k and y_i = y_j. For a self-symmetric B_i, x_i + w_i/2 = x_symAxis,k.
  - Centre form (ch. 2), vertical axis (2.1): x_j + x_j' = 2·x̂_i and y_j = y_j' for every pair, and x_k^s = x̂_i for every self-symmetric module. Horizontal axis (2.2): x_j = x_j', y_j + y_j' = 2·ŷ_i, y_k^s = ŷ_i.
  - Ch. 3 corner-to-centre form (3.13) and (3.14): ½[(x_i + w_i/2) + (x_i' + w_i'/2)] = x_sym and y_i + h_i/2 = y_i' + h_i'/2.
  - Pair members have equal dimensions and orientation (L3130–3131).
- Source: §1.1.4, L714–722 (PDF p. 22); §2.2.1 eqs. (2.1)/(2.2), L3134–3152 (PDF pp. 78–79); §3.1.2 eqs. (3.13)/(3.14), L4695–4707 (PDF p. 113).
- Philis stage: annotator, dp, verify
- Automation recipe:
  - Keep `Symmetry{a,b,axis}` in centre form.
  - Add an axis orientation field `dir: V|H` to `AxisId` metadata. Pick H when the stage's supply rails run vertically or its signal flow is horizontal (ch. 2 says the direction is normally set by power/ground lines or signal flow, L3915–3916).
  - `error()` becomes (y_a + y_b − 2·axis, x_a − x_b) for H.
- Beats hand layout because: every pair is checked to exact integer equality, on both axes, every epoch.
- Philis status: **partial**. Vertical only (`kernel/analog/src/placement/symmetry.rs:10-16`, `:51-58`, `:135-151`); no horizontal symmetry type.

### BAL1-02 Three symmetry types: mirror, perfect, self
- Kind: rule / data-model
- Statement:
  - Mirror: identical geometry, mirrored orientation. This is the standard form and allows mirror-symmetric routing.
  - Perfect: identical orientation. Needed against anisotropic disturbances such as oblique implant. Routing must then be parasitic-matched but not geometrically mirrored.
  - Self-symmetric: a geometrically symmetric device on the axis, e.g. a bias device tied to both halves or a thermal centre.
- Source: §1.1.4, L676–712 (PDF pp. 20–22); footnote 1, L734–736.
- Philis stage: annotator, cells, dp, dr
- Automation recipe:
  - Add `kind: Mirror|Perfect` to `Symmetry`.
  - Default to Perfect when the deck says the implant tilt is nonzero, or when the pair is a diff input pair with a tight offset budget. Otherwise use Mirror, which lets `dr` copy routes by reflection.
  - Enforce the chosen kind as a hard equality on `orient`: equal for Perfect, reflected about the axis for Mirror.
  - Self-symmetric already exists as a == b.
- Beats hand layout because: the choice is made per pair from the offset budget and the deck's implant tilt.
- Philis status: **partial**. dp never introduces a mirror and matched devices keep their seeded orientation, so the placer produces perfect symmetry only (`backend/dp/src/lib.rs:555-560`). Self-symmetric tails exist (`backend/annotator/src/emit.rs:199-208`). There is no mirror-orientation option.

### BAL1-03 Thermal symmetry about radiating devices
- Kind: heuristic
- Statement: thermally sensitive pairs (bipolars in particular) go symmetrically about the heat-radiating devices, so both members are equidistant and see the same temperature. Otherwise temperature mismatch results, and unbalanced thermal coupling can even cause oscillation.
- Source: §1.1.4, L661–670 (PDF pp. 20–21).
- Philis stage: annotator, dp
- Automation recipe: for each matched pair, find the top-power device in `Layout::power_uw` and add a soft equality |d(a, hot)| = |d(b, hot)|. Better, place the hot device self-symmetric on the pair's axis when both share a stage.
- Beats hand layout because: it is computed from operating-point power for every pair, not only the obvious output stage.
- Philis status: **partial**. `ThermalGradient` budgets the ΔT seen by a pair from the temperature field (`kernel/analog/src/placement/thermal.rs:13-27`). It does not make the heat source self-symmetric.

### BAL1-04 Choose a representation whose codes are symmetric-feasible by construction
- Kind: heuristic (design principle)
- Statement:
  - Rejecting infeasible codes during SA was "extremely ineffective", because infeasible codes overwhelm feasible ones.
  - A representation is useful for analog only if (a) feasibility can be recognized from the code and (b) moves stay inside the feasible subspace. Linear evaluation matters less than this.
  - Slicing is a poor choice: it cannot reach many topologies and packs badly with disparate sizes.
- Source: §1.1.6–1.1.7, L796–803, L854–877 (PDF pp. 24–25).
- Philis stage: dp, flow
- Automation recipe: see BAL1-29..34 (ASF-B*/HB*-tree) and BAL1-06..13 (S-F SP). Symmetry and non-overlap become properties of the decoder instead of projected repairs.
- Beats hand layout because: the search visits only legal symmetric layouts, so all of its time goes to optimizing wire and parasitics.
- Philis status: **missing**. dp is absolute-coordinate SA with projection (`backend/dp/src/lib.rs:1-8`).

### BAL1-05 Absolute-representation failure modes
- Kind: check / heuristic
- Statement:
  - With flat coordinates and an overlap penalty (typically quadratic): convergence is slow; the final overlap is nonzero and needs post-processing that adds runtime and degrades optimality; the overlap weight is fragile (too large impedes the search, too small lets cells collapse). TimberWolf needed negative-feedback weight control.
  - Its one advantage is easy modelling of matching and symmetry, and of beneficial overlap (merging).
- Source: §1.1.5, L739–768, footnote 2, L783–785 (PDF pp. 22–23). Ch. 2 restates it at L3986–3993 (PDF p. 96).
- Philis stage: dp
- Automation recipe:
  - Keep dp's absolute moves only as a final low-temperature refinement after a topological packing.
  - Record in the dp report how much encroachment the terminal legalizer removed, as a metric of how far the SA was from legal.
- Beats hand layout because: not applicable. This entry diagnoses Philis's current dp design.
- Philis status: **present, and it is the criticized pattern**. Clearance encroachment in the gate plus a terminal O(n²) legalizer (`backend/dp/src/lib.rs:124-176`, `backend/dp/src/legalize.rs:15-16`).

### BAL1-06 Symmetric-feasible sequence pair (condition 1.1)
- Kind: check / data-model
- Statement:
  - SP semantics: α_A⁻¹ < α_B⁻¹ ∧ β_A⁻¹ < β_B⁻¹ ⇒ A is left of B. α_A⁻¹ < α_B⁻¹ ∧ β_B⁻¹ < β_A⁻¹ ⇒ A is above B.
  - (α, β) is S-F if, for all distinct x, y in any symmetry group: α_x⁻¹ < α_y⁻¹ ⇔ β_sym(y)⁻¹ < β_sym(x)⁻¹.
  - Consequences: a pair keeps the same order in α and β, so the members sit side by side. Two cells from different pairs appear in reversed relative order to their partners. The condition works for self-symmetric cells (x = sym(x)).
  - It is **sufficient, not necessary**. A symmetric placement whose unique SP violates (1.1) exists [21]; such cases are rare and depend on dimensions.
- Source: §1.3, eq. (1.1), L1813–1908 (PDF pp. 48–50).
- Philis stage: dp
- Automation recipe: `struct SeqPair { alpha: Vec<u16>, beta: Vec<u16> }` plus `fn sf_ok(&self, groups) -> bool`, a debug assert run after every move. Pairs come from `RuleBatch::mirror_pairs` (`kernel/analog/src/rule.rs:197`).
- Beats hand layout because: symmetry holds by construction for every candidate, instead of being repaired at the end.
- Philis status: **missing**.

### BAL1-07 Search-space size of S-F sequence pairs (Lemma 1)
- Kind: formula / metric
- Statement:
  - For n cells and G groups with p_k pairs and s_k self-symmetric cells: #S-F SPs ≤ (n!)² / ∏_k (2p_k + s_k)!.
  - Example: n = 7, p = s = 2 gives (7!)²/6! = 35,280 of 25,401,600 codes, a 99.86% reduction.
  - If the whole circuit is one group (n = 2p): ≤ (2p)!, at least half the tree-code count b_2p·(2p)!.
- Source: Lemma 1, L1911–1944 (PDF p. 50, verified against the PDF); L2526–2533 (PDF p. 64).
- Philis stage: dp, flow
- Automation recipe: log log10 of the search-space size per stage at dp start. Use it to size `MOVES_PER_CELL` (`backend/dp/src/lib.rs:23`) by the feasible space instead of n.
- Beats hand layout because: effort is allocated from the size of the space.
- Philis status: **missing**.

### BAL1-08 S-F SP evaluation: ordinates (Step 1y)
- Kind: algorithm
- Statement:
  - Sweep α in reverse (i = n..1). For each cell B_j = α_i at β-index l: find the predecessor bucket (largest index < l) and set y_j = max(y_j, its length). If the partner B_k was already visited with y_k < y_j, set y_k = y_j and restart Step 1y.
  - Insert (l, y_j + h_j) and drop dominated buckets (index > l, length ≤ y_j + h_j).
  - (1.1) guarantees termination. Height is minimal because the lowest member rises by the minimum.
  - Cost: O(n log log n) per pass with Johnson's queue. Worst case ⌈p/2⌉+1 passes, O(p·n log log n); in practice at most 3 passes.
- Source: §1.3.1.1, L1993–2099, Fig. 1.18 (PDF pp. 52–54).
- Philis stage: dp
- Automation recipe: n ≤ ~100 per level (footnote, L1605), so use a plain O(n²) longest path over the SP order instead of Johnson's queue. The pass-restart logic is the part that matters. Output `Layout::y` = y + h/2 (Philis uses centres).
- Beats hand layout because: every candidate is packed to minimum height with exact y-equality.
- Philis status: **missing**.

### BAL1-09 S-F SP evaluation: abscissae initialization (Step 1x)
- Kind: algorithm
- Statement: sweep α forward. x_j = length of the predecessor bucket; insert (l, x_j + w_j); prune dominated buckets. O(n log log n). Without symmetry, Step 1y plus Step 1x is the whole FAST-SP evaluator.
- Source: §1.3.1.2, L2104–2119, L2279–2280 (PDF pp. 54, 58).
- Philis stage: dp
- Automation recipe: same longest-path routine with widths plus the pairwise d_min (BAL1-45) as edge weights.
- Beats hand layout because: every candidate is compacted exactly.
- Philis status: **missing**.

### BAL1-10 Axis selection by embedding DAG (Step 2x)
- Kind: algorithm
- Statement:
  - Build a DAG of pairs and self-symmetric cells: an arc outer → inner whenever a new cell's y-span covers a right-contour segment of a group cell (insert y-span endpoints in α order into a keyed contour).
  - x_symAxis = max over sink nodes (B_j, B_k) of (x_j + x_k + w_k)/2.
  - Each sink shifts right by d = x_symAxis − (x_j + x_k + w_k)/2 ≥ 0. Inner-to-outer, each node takes d = min over its children.
  - Result: inner pairs hug the axis and the group's x-span is minimal relative to the topological constraints.
  - Cost O((p+s) log log H), or O(H + n log log n) with radix-sort rank normalization.
- Source: §1.3.1.2, L2129–2250, Figs. 1.19–1.20 (PDF pp. 55–58).
- Philis stage: dp
- Automation recipe: this replaces `SymmetryGroup::project`'s "mean of midpoints" axis (`kernel/analog/src/placement/symmetry.rs:133-151`), which minimizes displacement but not group span.
- Beats hand layout because: the axis position that minimizes group width is computed exactly.
- Philis status: **missing** (the axis is the mean of the midpoints).

### BAL1-11 Sweep-to-right and sweep-to-left symmetry fixing (Steps 3x/4x)
- Kind: algorithm
- Statement:
  - Step 3x repeats the forward LCS. Before inserting, if (B_k, B_j) is a pair and d = 2·x_sym − x_j − (x_k + w_k) > 0, it sets x_j += d.
  - Step 4x sweeps α backward from the sentinel (n+1, W): x_j = length(succ) − w_j; if d = x_j + (x_k + w_k) − 2·x_sym > 0, it sets x_j −= d; it prunes buckets with index < l and length ≥ x_j.
  - Both are O(n log log n). (1.1) prevents intertwined pairs, so the algorithm ends after Step 4x.
- Source: §1.3.1.2, L2251–2323 (PDF p. 58–60).
- Philis stage: dp
- Automation recipe: this gives exact symmetry **without an LP solver** (Philis has none), and is the lightest path to exact symmetric compaction.
- Beats hand layout because: exact symmetry and packing, deterministic.
- Philis status: **missing**.

### BAL1-12 Multiple and embedded symmetry groups in one SP
- Kind: rule / algorithm
- Statement:
  - Two groups deadlock if α⁻¹(x) < α⁻¹(y) < α⁻¹(sym y) < α⁻¹(sym x) and β⁻¹(y) < β⁻¹(x) < β⁻¹(sym x) < β⁻¹(sym y). The easiest prevention is to forbid intermingling of groups in α and β.
  - Embedded groups (when the schematic calls for them) are allowed if the outer group's cells surround the inner group's in both sequences.
  - Process groups one by one and freeze each; inner groups go first (order from a group-embedding DAG). Cost O(G·n log log n).
  - A code that is not S-F for every group may loop forever.
- Source: §1.3.2, L2327–2371, Fig. 1.22 (PDF pp. 60–61).
- Philis stage: annotator, dp
- Automation recipe: the annotator emits group nesting (stage inside stage) from BAL1-52's HSMPG tree. The SP move set enforces "no intermingling" by default and "surround" for nested groups. Add an iteration cap to the fixing loop as a guard.
- Beats hand layout because: nested symmetric structures (e.g. a CMFB amplifier inside a fully differential stage) stay exact.
- Philis status: **missing**. One axis per top-level block; no nesting (`backend/annotator/src/emit.rs:3-16`).

### BAL1-13 S-F SP initial code and move set
- Kind: algorithm
- Statement:
  - Initial code: α = a1…ap c1…cs bp…b1 and β = a1…ap cs…c1 bp…b1, where (a_i, b_i) are pairs and c_j are self-symmetric. Pairs nest like brackets around a vertical stack of self-symmetric cells. More generally, pick any α order for the group and put the partners in reverse order in β.
  - Moves:
    - Interchanging two cells of different pairs in α forces interchanging their partners in β.
    - Moving a cell in α forces its partner to move in β, within a range that depends on the first move.
    - Rotation and mirroring act on both members.
    - Asymmetric cells move freely.
    - Whole-group moves run with low, temperature-decreasing probability.
- Source: §1.3.3, L2375–2405 (PDF pp. 61–62).
- Philis stage: dp
- Automation recipe: add SP moves beside the existing dp moves in `place()` (`backend/dp/src/lib.rs:303-340`). Keep the Metropolis gate and the PEX tier unchanged. The hard-violation count for symmetry and overlap is then 0 by construction.
- Beats hand layout because: enables exhaustive stochastic exploration inside the legal symmetric subspace.
- Philis status: **missing**. Compound symmetric moves exist in absolute form: group shift, pair expand, pair swap (`backend/dp/src/lib.rs:309-326`, `:473-535`).

### BAL1-14 Contour by segment tree over normalized y
- Kind: algorithm
- Statement:
  - Rank-normalize S = ∪{y_i, y_i + h_i}. For cells in HCG topological order: x_i = max v.x over the standard-interval nodes of [a_i, b_i], then set those nodes to x_i + w_i. W = max v.x.
  - O(n log n). Build once over [0, n] and re-initialize per SA iteration; rebuilding each time costs 15–20% runtime.
- Source: §1.2.1, L994–1171 (PDF pp. 27–32).
- Philis stage: dp
- Automation recipe: not needed at Philis sizes (≤100 per level, L1605). An O(n) skyline `Vec<(y0, y1, x)>` scan per insert (O(n²) total) is simpler.
- Beats hand layout because: n/a (speed only).
- Philis status: **missing**. **Not recommended**; YAGNI at the current n.

### BAL1-15 Contour by red–black interval tree
- Kind: algorithm
- Statement: disjoint intervals cover [0, H], with the trichotomy update rules. Collinear neighbours merge. Amortized O(log n) per update, O(n log n) total, ≤2n−1 nodes. RB needs ≤2 rotations per insert and ≤3 per delete, against Θ(log n) for AVL.
- Source: §1.2.2, L1174–1423 (PDF pp. 32–39).
- Philis stage: dp
- Automation recipe: only if a flat block exceeds a few hundred cells. `BTreeMap<y, x>` from std gives the same bound without custom RB code.
- Beats hand layout because: n/a.
- Philis status: **missing**; not needed.

### BAL1-16 Contour by 1-3 deterministic skip list
- Kind: algorithm
- Statement: keys are the y-breakpoints of the right contour, with value x of the segment above each key. UpdateDSL: find the largest key ≤ a; x_i = max over keys in [q.key, b); insert a and b; remove keys in (a, b); merge equal-x neighbours. O(n log n) amortized, ≤2n+7 nodes. Top-down insert splits gaps of size 3, top-down delete merges or borrows.
- Source: §1.2.3–1.2.3.3, L1427–1767 (PDF pp. 39–47).
- Philis stage: dp
- Automation recipe: same as BAL1-15. `BTreeMap` covers it.
- Beats hand layout because: n/a.
- Philis status: **missing**; not needed.

### BAL1-17 Johnson priority queue for LCS evaluation
- Kind: algorithm
- Statement: bucket keys 1..N live on the leaves of a host tree of size 2^h + N with h = ⌈log2(N+1)⌉. Insert and delete cost O(log log N), giving O(n log log n) SP evaluation. Buckets carry (index in β, LCS length).
- Source: §1.2.4, L1770–1807; §1.3.1, L1987–1990 (PDF pp. 47–52).
- Philis stage: dp
- Automation recipe: skip it. Use O(n²) or `BTreeMap` O(n log n) LCS.
- Beats hand layout because: n/a.
- Philis status: **missing**; not needed.

### BAL1-18 Rank-normalize coordinates before contour work
- Kind: algorithm
- Statement: replace each y by its rank in the sorted unique endpoint set. This shrinks the structures: in the example, segment-tree depth drops from ⌈log2 120⌉ = 7 to 3. With radix sort, normalization is O(n + H).
- Source: L994–1002, L1081–1084, L2223–2250 (PDF pp. 27–29, 57–58).
- Philis stage: dp
- Automation recipe: only if a contour structure is added. A sort plus dedup of `Vec<i32>` suffices.
- Beats hand layout because: n/a.
- Philis status: **missing**; minor.

### BAL1-19 TCG is equivalent to SP; do not adopt TCG
- Kind: rule
- Statement: β is the unique topological sort of C_h ∪ C_v, and α that of C_h ∪ reversed C_v. TCG→SP costs O(n log n); SP→TCG costs Θ(n²) because the graphs have Θ(n²) arcs. Both have (n!)² codes and are P-admissible. TCG-S evaluation is quadratic, so S-F SP is faster at equal quality.
- Source: §1.4.1, L2412–2452 (PDF pp. 62–63).
- Philis stage: dp
- Automation recipe: none. This is a design decision to record in docs/API-WISH.md.
- Beats hand layout because: n/a.
- Philis status: n/a.

### BAL1-20 Tree codes: O-tree / binary tree count and Lemma 2
- Kind: formula / data-model
- Statement:
  - O-tree code (T, π): T is a 2n-bit DFS string (0 = descend, 1 = ascend) and π a label permutation. O-trees with n+1 nodes biject onto binary trees with n nodes.
  - The number of labeled binary trees is b_n·n!, with Catalan b_n = C(2n, n)/(n+1).
  - Lemma 2: b_n·n! < (n!)² for n ≥ 3; they are equal for n = 1, 2.
  - When symmetry dominates, S-F SPs are fewer than trees; when the asymmetric part dominates, trees are fewer.
  - B*-tree counts are 336 for 4 modules and 57,657,600 for 8 (= n!·b_n).
- Source: §1.4.2, L2459–2533 (PDF pp. 63–64); §3.5, L6342–6344 (PDF p. 143).
- Philis stage: dp, flow
- Automation recipe: choose the representation per stage. A stage with symmetric-module share ≥ ~50% uses S-F SP or ASF-B*; otherwise use B*/HB*. Exhaustive enumeration is feasible only up to about 6–7 modules (by the n!·b_n growth), which bounds a basic group in BAL1-60.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-21 Symmetric-feasible binary tree (condition 1.2)
- Kind: check
- Statement:
  - Binary-tree placement rules (Balasa [36]): a left-subtree cell is above its parent; when two cells' y-projections overlap, the one first in preorder is on the left.
  - S-F condition: for distinct A, B in a group, A precedes B in inorder ⇔ sym(B) precedes sym(A) in preorder.
  - Cross-group exclusion: forbid the pattern A <in B <in sym(B) <in sym(A) together with B <pre A <pre sym(A) <pre sym(B), which would stop horizontal alignment.
- Source: §1.4.2.1, L2550–2597, footnote 16, L2628–2630 (PDF pp. 65–66).
- Philis stage: dp
- Automation recipe: use as a debug assertion if a B*-tree variant is chosen. ASF-B*-tree (BAL1-29) makes it unnecessary.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-22 Complete (reachability-proven) move sets for binary trees
- Kind: algorithm
- Statement:
  - (a) Rotation ((S1S2)S3) ↔ (S1(S2S3)), with NULL children as dummy leaves; Stasheff-polytope connectivity makes every shape reachable. (b) The E-N swap on the Dyck path of n E's and n N's (no prefix has more N than E). Label interchange completes either set.
  - "Detach subtree and reattach" is avoided because it cannot easily preserve (1.2).
  - Balasa's tool uses (b). An E-N swap costs O(n).
- Source: §1.4.2.2(a), L2600–2683, Figs. 1.24 (PDF pp. 66–68).
- Philis stage: dp
- Automation recipe: if a B*-tree is implemented, include at least one provably complete move pair (rotation + swap). This guarantees the anneal can reach the optimum topology.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-23 Symmetry-preserving tree moves
- Kind: algorithm
- Statement:
  - Interchange within a group, other than a pair with itself, also swaps the partners.
  - Cross-group or mixed interchange is allowed only between parent and child nodes; this is O(1) and complete for labels.
  - Moving a cell in a group performs two E-N swaps, one for the cell and one for its partner, each O(n).
  - Group rebuild: extract a group's nodes, build a fresh S-F subtree and reattach it. This is O(n²), with low probability decreasing with T.
  - Orientation changes respect mirror, perfect or self symmetry.
- Source: §1.4.2.2(b), L2686–2727 (PDF pp. 68–69).
- Philis stage: dp
- Automation recipe: the move table for a tree-based dp. The group-rebuild move doubles as the escape from local minima that the current dp lacks.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-24 Empirical: S-F topological search beats absolute; SP quality ≥ tree
- Kind: metric
- Statement:
  - With identical cost and schedule, every S-F topological technique beats the absolute representation in CPU and sometimes in quality, and SA tuning is easier.
  - S-F binary trees run faster than S-F SPs, but their quality "seems to be poorer".
  - Evaluation speed: priority queue ≈ RB tree ≈ DSL; segment tree is slowest.
  - Table 1.1 (C++, SUN Blade 100), cells / area (µm × µm; µ lost in extraction) / min:
    - gain-boost amp: 17 / 71.2×68.0 / 0.2
    - telescopic opamp with gain-boost: 36 / 527.2×96.0 / 1.6
    - programmable capacitor block 1: 28 / 175.2×115.2 / 0.5
    - programmable capacitor block 2: 34 / 220.8×186 / 1.1
    - 15 MHz buffer: 64 / 189.5×250.5 / 3.2
    - selectable-gain amplifier: 79 / 191.5×251.0 / 4.3
    - bias generator: 85 / 237×186 / 5.0
    - charge pump: 98 / 220.5×333.0 / 12.6
    - limiter: 111 / 177.5×375.0 / 16.9
    - frequency divider (5 symmetry groups): 116 / 350×147 / 21.0
- Source: §1.5, L2731–2817, Table 1.1 (PDF pp. 69–71).
- Philis stage: flow, benchmarks
- Automation recipe: benchmark any new representation against the current dp at equal wall time. Report area, HPWL, Θ and the legalizer residual.
- Beats hand layout because: n/a (evidence for the representation choice).
- Philis status: n/a.

### BAL1-25 Constraint taxonomy: CC, symmetry, proximity, and their hierarchical forms
- Kind: data-model
- Statement:
  - Common centroid: current mirrors and diff pairs, against process mismatch.
  - Symmetry: the whole differential subcircuit, against parasitic mismatch.
  - Proximity: a common device model or function, so the subcircuit shares one substrate/well region or one guard ring. This reduces area, wirelength and substrate coupling, and the outline may be rectilinear.
  - Hierarchical symmetry and hierarchical proximity nest these constraints over an exact (circuit) or virtual (clustered by model, function or constraint) hierarchy.
- Source: §2.1, L2996–3050, Figs. 2.1–2.3 (PDF pp. 75–77).
- Philis stage: annotator
- Automation recipe: extend `Block` (`backend/annotator/src/block.rs:7-19`) with `constraint: Sym|CC|Prox` per hierarchy node. Allow the tree to be deeper than stage → primitive, e.g. by using BAL1-52's HSMPG tree.
- Beats hand layout because: every nesting level is recorded and enforced, not only the top level.
- Philis status: **partial**. Two-level blocks: a stage and its ≤2-device leaves (`backend/annotator/src/block.rs:76-90`). Proximity is only a pairwise edge gap (`kernel/analog/src/placement/proximity.rs:7-16`).

### BAL1-26 Pelgrom distance term motivates adjacency of symmetric devices
- Kind: formula
- Statement: σ²(ΔP) = A_P²/(W·L) + S_P²·D_x², where A_P is the area proportionality constant, W·L the device area, S_P the distance variation and D_x the spacing. Larger pair spacing gives larger mismatch, so the symmetric modules of a group should be adjacent.
- Source: §2.2.2, eq. (2.3), L3158–3183 (PDF p. 79).
- Philis stage: annotator, dp
- Automation recipe: already used for budgets. Also use D_x to rank the symmetry-island preference (BAL1-27).
- Beats hand layout because: spacing is budgeted per pair from the deck's A_VT/S_VT.
- Philis status: **implemented** (`kernel/analog/src/placement/matching_pair.rs:28-39`; placement owns the distance term per `backend/annotator/src/emit.rs:19-27`).

### BAL1-27 Symmetry island (check)
- Kind: check / metric
- Statement: a symmetry group forms a symmetry island if every module abuts at least one other module of the same group and the group's placement is connected (Def. 2.1).
- Source: §2.2.2, Def. 2.1, L3184–3193, Fig. 2.6 (PDF pp. 79–80).
- Philis stage: verify, dp
- Automation recipe:
  - Graph over group members, with an edge when the edge-to-edge gap ≤ the pair's d_min (BAL1-45) and the facing edges overlap in projection. Connected components via union-find.
  - Report `islands` (1 is ideal) per stage in `Report`.
  - As a Θ budget, add (components − 1) × a penalty, or make it a hard rule once the topological placer guarantees it.
- Beats hand layout because: every stage is checked for island connectivity, where a human judges it by eye.
- Philis status: **missing**. There is no connectivity or island check; `Proximity` is pairwise only.

### BAL1-28 B*-tree definition and linear-time packing
- Kind: data-model / algorithm
- Statement:
  - Ch. 2 convention: the root is the bottom-left module. For node n (module b), the left child is the lowest adjacent module to the right of b (x_left = x_b + w_b); the right child is the first module above b at the same x (x_right = x_b).
  - y comes from a contour (doubly linked list). Preorder traversal packs in linear time.
  - Ch. 3 uses the transposed convention: left child above, right child to the right, preorder order on y-overlap (L4908–4918).
- Source: §2.2.3, L3218–3250, Fig. 2.7 (PDF pp. 80–81); §3.1.3, L4908–4918 (PDF p. 117).
- Philis stage: dp
- Automation recipe:
  - `struct BTree { root: u16, l: Vec<u16>, r: Vec<u16> }` using `u16::MAX` as null.
  - The skyline is a `Vec<(x0, x1, ytop)>` scanned linearly, O(n) per insert, fine at n ≤ 100.
  - Convert to `Layout` centres (x + w/2, y + h/2), with the per-pair d_min from BAL1-45 added as halo.
- Beats hand layout because: every candidate is compacted and overlap-free by construction.
- Philis status: **missing**.

### BAL1-29 ASF-B*-tree: symmetry by construction, with a symmetry island guaranteed
- Kind: data-model / rule
- Statement:
  - Keep one representative per pair (b_j', Def. 2.2) and, for each self-symmetric module, its right half (vertical axis) or top half (horizontal axis) (Def. 2.3).
  - Pack only the representatives and mirror the rest (Def. 2.5).
  - Lemma 2.1: a self-symmetric representative must abut the axis. Lemma 2.2: a pair representative off the axis is always feasible, since |x − x̂| ≥ w/2.
  - Property 2.1: self-symmetric representatives lie on the rightmost branch (vertical axis) or the leftmost branch (horizontal axis). An ASF-B*-tree is a representative B*-tree satisfying 2.1 (Def. 2.7).
  - Theorems: 2.1, symmetric-feasible for either axis; 2.2, packing gives a symmetry island; 2.3, one-to-one with compacted symmetric placements, so there is no redundancy.
- Source: §2.3.1, L3256–3451, Figs. 2.8–2.9 (PDF pp. 81–85).
- Philis stage: dp
- Automation recipe:
  - Per stage `SymmetryGroup`, build the ASF-B*-tree from `mirror_pairs` (a == b means self-symmetric; see `backend/annotator/src/emit.rs:206`). Pack the representatives with the axis at x = 0 (the island's left boundary), then mirror.
  - Self-symmetric width = w/2 on the half-plane.
  - This retires `SymmetryGroup::project` for groups placed this way.
- Beats hand layout because: every candidate is exactly symmetric and island-connected, with no redundant codes. An SA explores thousands of compact island shapes per second.
- Philis status: **missing**.

### BAL1-30 ASF-B*-tree packing and island bottom contour
- Kind: algorithm
- Statement:
  - Pack in preorder, keeping both horizontal and vertical contours. Compute mirrored coordinates from (2.1)/(2.2).
  - The island's bottom contour is found by traversing both vertical contours bottom-up, keeping the convex points and joining them horizontally.
  - O(n(S_i)).
- Source: §2.3.2, L3454–3503, Figs. 2.10–2.11 (PDF pp. 85–86); Theorem 2.4 proof, L3707–3715.
- Philis stage: dp
- Automation recipe: return `Island { members, top: Vec<Seg>, bottom: Vec<Seg>, w, h }` for use by the HB*-tree packer (BAL1-32).
- Beats hand layout because: n/a (mechanism).
- Philis status: **missing**.

### BAL1-31 HB*-tree with hierarchy nodes and contour nodes (Property 2.2)
- Kind: data-model
- Statement:
  - A hierarchy node represents an island (holding its ASF-B*-tree). Contour nodes represent the island's top contour segments from left to right. Six rules:
    1. The left child of a hierarchy node is a non-contour node.
    2. The right child of a hierarchy node is the leftmost top-contour node.
    3. The left child of a contour node is the next contour node to the right.
    4. The children of regular nodes are non-contour nodes.
    5. The right child of a contour node is a non-contour node.
    6. The parent of a contour node is a contour node or the hierarchy node.
  - Rectilinear islands are handled without slicing them into sub-blocks.
- Source: §2.4.1–2.4.2, L3509–3625, Figs. 2.12–2.13 (PDF pp. 86–89).
- Philis stage: dp
- Automation recipe: the node enum is `Module(u16) | Hier(group) | Contour(group, seg)`. The move generator filters targets using rules 1–6.
- Beats hand layout because: other devices can nest into the notches of a rectilinear island, where a human usually leaves the island's bounding box empty.
- Philis status: **missing**.

### BAL1-32 HB*-tree packing, including well/guard-ring white space
- Kind: algorithm
- Statement:
  - Pack in preorder. At a hierarchy node, pack its ASF-B*-tree, then find the island's best y by matching its bottom contour against the current contour, which minimizes dead space. Then pack the left subtree, then the contour nodes, which replace the hierarchy node in the contour.
  - When a module's device type (NMOS/PMOS) differs from the adjacent packed modules in the contour, snap it to reserve well or guard-ring space.
  - Theorem 2.4: O(m + n) = O(n).
- Source: §2.4.3, L3628–3734, Fig. 2.14 (PDF pp. 89–91).
- Philis stage: dp
- Automation recipe: this is the full packer. Store a device class (well id: nwell net, pwell/substrate, DTI tub) on each skyline segment. When packing module m next to segment class c ≠ class(m), add d_min(class(m), c) from BAL1-45.
- Beats hand layout because: well spacing is applied exactly and only where the classes differ, and every well boundary is checked.
- Philis status: **missing**. One global clearance everywhere (`frontend/library/src/lib.rs:529-537`).

### BAL1-33 SA cost and schedule for HB*-tree
- Kind: heuristic
- Statement: Φ(P) = α·A_P + β·W_P, with A_P the bounding-rectangle area and W_P the HPWL (2.4). T0 = −Δ_avg/ln P, where Δ_avg is the average uphill cost and P the initial uphill-acceptance probability (2.5; sign verified in the PDF). Cooling 0.9 per pass, 20,000 iterations per temperature, left-skewed initial HB*-tree.
- Source: §2.5, L3740–3748; §2.6.2, L4074–4081 (PDF pp. 91, 98).
- Philis stage: dp
- Automation recipe: keep Philis's PEX tier as W_P and add bounding area explicitly (today the `Utilization` floor handles it). Philis measured the Lampaert T0 = −ΔC⁺/ln 0.6 schedule and rejected it (`backend/dp/src/lib.rs:296-300`). Re-measure only once the representation changes.
- Beats hand layout because: n/a.
- Philis status: **partial**. SA exists with α = 0.93 and T0 = 0.02·mean|ΔPEX| (`backend/dp/src/lib.rs:24`, `:276-289`).

### BAL1-34 HB*/ASF-B* perturbations Op1–Op5
- Kind: algorithm
- Statement:
  - Op1 rotate, Op2 move, Op3 swap. Non-hierarchy nodes get higher selection probability, because hierarchy moves are big jumps. Contour nodes move only with their hierarchy node.
  - Inside an ASF-B*-tree:
    - Rotating a pair rotates both members. Rotating a self-symmetric module reshapes its representative.
    - A pair representative can move anywhere. A self-symmetric representative moves only along the rightmost (leftmost) branch.
    - Swapping two pair representatives is free. If a self-symmetric representative is involved, the other node must be on the same branch.
    - Op4, representative change: flip which member of a pair is the representative, or flip a self-symmetric module. It keeps area and changes wirelength, in O(1).
    - Op5, symmetry-type conversion: rotate every module and swap the left/right children of every node. It is rarely used.
- Source: §2.5.1–2.5.2.5, L3751–3916, Figs. 2.15–2.19 (PDF pp. 91–94).
- Philis stage: dp
- Automation recipe: this is the move table.
  - The existing reshape move (variants) and DTI branch flip remain as extra ops. Reshape must reshape both members of a pair (BAL1-43).
  - Op4 is the cheapest HPWL-only move and should carry a high share in low-T epochs.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-35 Dangling-node repair after island contour change
- Kind: algorithm
- Statement:
  - If a perturbed island loses top segments, each dangling node attaches to the nearest contour node: as its right child if that slot is empty, else as the left child of the leftmost-skewed descendant of that right child.
  - This keeps the relative topology, in amortized O(1).
- Source: §2.5.3, L3920–3964, Fig. 2.20 (PDF pp. 94–96).
- Philis stage: dp
- Automation recipe: required for correctness of the HB*-tree implementation. Unit-test it on the Fig. 2.20 case.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-36 Complexity table of symmetric placement approaches
- Kind: data-model (reference numbers)
- Statement: Table 2.2 (n = modules, m = symmetry pairs/groups), perturbation / packing:

  | Approach | Perturbation | Packing |
  |---|---|---|
  | B*-tree [1] | O(lg n) | O(n²) |
  | B*-tree + segment tree | O(lg n) | O(n lg n) |
  | B*-tree + RB tree | O(lg n) | O(n lg n) |
  | B*-tree + skip list | O(lg n) | O(n lg n) |
  | SP | O(1) | O(n²) |
  | SP + LP | O(1) | Ω(n²) |
  | SP with dummy nodes | O(1) | O(n²) |
  | SP with priority queue | O(1) | O(m·n lg lg n) |
  | TCG-S | O(n²) | O(n²) |
  | TCG | O(n) | O(n²) |
  | B*-tree with ESF + LP | N/A | Ω(n²) |
  | ASF-B*-tree + HB*-tree | O(lg n) | O(n) |
- Source: §2.6.1, L3979–4042, Table 2.2 (PDF pp. 96–97).
- Philis stage: dp
- Automation recipe: use it to justify ASF-B*+HB* as the dp representation (see §4).
- Beats hand layout because: n/a.
- Philis status: n/a.

### BAL1-37 Benchmark results: HB*-tree against SP, segment tree, TCG-S, SP+LP, SP-dummy
- Kind: metric
- Statement:
  - MCNC (Table 2.5), area mm² / time s:
    - apte: SP 48.12/25; seg 47.52/11; TCG-S 47.52/3; SP-dummy 46.92/13; HB* 46.92/2; HB* with area + WL 47.90, 10.20 mm, 3 s
    - hp: 9.84/138; 9.71/62; 9.71/50; 9.43/13; 9.35/2; 10.10, 30.74 mm, 16 s
    - ami33: 1.24/684; 1.23/307; 1.21/423; 1.24/23; 1.23/12; 1.29, 47.23 mm, 39 s
    - ami49: 37.82/2,038; 37.31/983; 37.04/1,247; 38.32/29; 36.85/20; 41.32, 769.99 mm, 96 s
    - Normalized areas: 1.03 / 1.02 / 1.01 / 1.02 / 1.00. HB* is 4.09× faster than SP-dummy on the same machine.
  - Industrial (Table 2.6), area 10³ µm² / time s:
    - biasynth_2p4g (65 modules; symmetry modules 8+12+5; module area 4.70): SP 5.40/780; seg 5.40/246; SP+LP 4.96/206; SP-dummy 5.57/134; HB* without rotation 5.15; HB* 4.92, 22 s
    - lnamixbias_2p4g (110 modules; 16+6+6+12+4; module area 46.00): 50.80/2,824; 50.30/726; 50.15/3,027; 52.21/227; 50.28; 48.63, 43 s
  - Reductions: 7.1%, 6.6%, 1.6% and 10.3% area; 39.88× and 5.68× speedups over SP+LP and SP-dummy. Without rotation the overhead is 2.4% against SP+LP and 4% against HB* itself.
- Source: §2.6.2, L4046–4152, Tables 2.3–2.6 (PDF pp. 97–100).
- Philis stage: benchmarks
- Automation recipe: add a module-only (no-netlist) fixture mode to `benchmarks/src/fixtures.rs` so the dp packer can be scored on area usage against these numbers. The module sizes are in [22]/[14], not in this book ("not given" here).
- Beats hand layout because: n/a (quantitative calibration).
- Philis status: **missing**.

### BAL1-38 Aligning several symmetry groups on one axis
- Kind: algorithm
- Statement: to align several islands to one vertical (horizontal) axis, insert a zero-width (zero-height) dummy block at the left of (below) each island and make it the island's parent. The hierarchy node is its left (right) child. Adjusting each dummy's width (height) aligns the axes. The technique extends Wu and Chang's alignment method (ref. [32], L4230).
- Source: §2.7.1, L4164–4230 (PDF pp. 100–102).
- Philis stage: annotator, dp
- Automation recipe: when BAL1-51's symmetry compound spans several recognized blocks (e.g. input pair stage and folded-cascode load stage), the annotator emits them with **one** `AxisId`, and dp aligns the islands via dummy blocks.
- Beats hand layout because: the axis is shared across stages, so symmetric routes can be long straight mirrors.
- Philis status: **missing**. One axis per top-level block (`backend/annotator/src/emit.rs:3-5`).

### BAL1-39 Folding asymmetric modules into a symmetry group as clusters
- Kind: algorithm
- Statement: non-symmetric modules can join a symmetry group as a self-symmetric cluster (pack them with a B*-tree; the cluster's bounding box becomes a self-symmetric representative), or as a symmetry pair of two clusters (C2, C2'), whose representative takes the larger dimensions. This yields non-island placements when needed.
- Source: §2.7.2, L4234–4249, Fig. 2.24 (PDF p. 102).
- Philis stage: dp
- Automation recipe: use it for the glue block's bias devices that connect to both halves. They sit on the axis as a self-symmetric cluster, as ch. 1 §1.1.4 L706–711 recommends.
- Beats hand layout because: n/a.
- Philis status: **partial**. Self-symmetric tails exist (`backend/annotator/src/emit.rs:203-208`); there are no clusters.

### BAL1-40 Hierarchical symmetry (group of groups)
- Kind: data-model / algorithm
- Statement: a symmetry group S_i may contain self-symmetric sub-groups S_j^s and symmetry-group pairs (S_k, S_k'). The top group contains everything. A mixed ASF-B*/HB* tree packs the islands hierarchically.
- Source: §2.8.1, L4361–4372, Fig. 2.3 (PDF pp. 76–77, 105).
- Philis stage: annotator, dp
- Automation recipe: in fully differential circuits, e.g. two identical halves each containing a mirror, emit group pairs as an `(AxisId, [GroupId; 2])` relation. The packer mirrors the whole sub-island.
- Beats hand layout because: nested symmetry is kept exact at every level simultaneously.
- Philis status: **missing**.

### BAL1-41 Hierarchical clustering (proximity) as connected placement
- Kind: rule / algorithm
- Statement: a cluster contains at least two modules, or one module and a sub-cluster, or two sub-clusters. Every super-cluster's modules and sub-clusters must be placed connected. There is one HB*-tree per cluster (#trees = #subcircuits + 1). A move first selects a tree, then applies a B*-tree operation. Packing is preorder, descending into a hierarchy node's tree before continuing. The framework can host a grid-based common-centroid placer [24] or a signal-flow placer [17] inside a node.
- Source: §2.8.2, L4376–4418, Fig. 2.25 (PDF pp. 105–106).
- Philis stage: annotator, dp, verify
- Automation recipe:
  - Hierarchy nodes come from annotator blocks (stages). The CC unit arrays that `cells` already generates as one macro remain leaves.
  - Check: each block's members form one connected component. Reuse BAL1-27's union-find.
- Beats hand layout because: every block's connectivity is checked on every candidate.
- Philis status: **missing**. Blocks only emit rules (docs/plans/audit-07-flow-frontend-docs.md:675 says the same).

### BAL1-42 Module, COG and distance definitions
- Kind: formula
- Statement:
  - x_COG(m) = x_m + w_m/2 and y_COG(m) = y_m + h_m/2.
  - d(m,n) = min(d_hor, d_vert), with d_hor = max(|x_COG(m) − x_COG(n)| − (w_m + w_n)/2, 0) and d_vert analogous.
  - Group COG is area-weighted: x_COG(G) = Σ w_m h_m x_COG(m) / Σ w_m h_m.
  - Groups are disjoint. A device consists of 1..k modules (e.g. parallel sub-transistors).
- Source: §3.1.1, eqs. (3.1)–(3.12), L4573–4664 (PDF pp. 111–112).
- Philis stage: dp, verify
- Automation recipe:
  - Philis's `Proximity::gap` is Euclidean, hypot(g_x, g_y) (`kernel/analog/src/placement/proximity.rs:19-25`). Keep it for proximity.
  - Use the source's min(d_hor, d_vert) for minimum-distance constraints, because spacing rules are Manhattan per axis.
- Beats hand layout because: n/a.
- Philis status: **partial**.

### BAL1-43 Variant constraints (matched devices use the same variant)
- Kind: rule
- Statement: modules may have several variants (e.g. finger counts, capacitor aspect ratios). A variant constraint restricts combinations for matching, e.g. both diff-pair transistors with the same number of gate fingers. Matching groups get same-variant constraints "including same orientation" (L6630).
- Source: §3.1.2, L4677–4683; §3.2.4, L5620–5621; §3.6.1, L6629–6630 (PDF pp. 113, 129, 147).
- Philis stage: cells, dp
- Automation recipe: in `try_reshape` (`backend/dp/src/lib.rs:584-620`), when c is in a matched pair or `Unitization::same_variant_required` group, reshape every member to the same variant index in one trial. Reject a reshape that leaves a pair with unequal variants.
- Beats hand layout because: variant equality is checked for every pair on every reshape, so no mismatch-by-shape slips through.
- Philis status: **partial**. `same_variant_required` exists at the cell tier (`kernel/analog/src/cell.rs:53`, set in `backend/annotator/src/constraints.rs:60`). dp's reshape changes one cell at a time (`backend/dp/src/lib.rs:584-620`), and no pair coupling was found.

### BAL1-44 Common centroid as an exact linear equality
- Kind: formula / rule
- Statement: for groups A and B, x_COG(A) = x_COG(B) (3.15) (and y). Example: a 16-transistor differential pair with a1–a8 and b1–b8 in parallel shares one COG (Fig. 3.1b). Matching groups use common centroid instead of alignment when devices are split into sub-devices. CC equalities enter the LP as C·x = k (3.28/3.35).
- Source: §3.1.2, L4709–4739; §3.2.4, L5622–5625 (PDF pp. 113–114, 129).
- Philis stage: cells, dp
- Automation recipe: keep CC inside `cells` unit arrays, where it is exact by pattern. When units are separate placement modules, add the CC equality rows to the compaction LP (BAL1-55).
- Beats hand layout because: the equality is exact to the grid.
- Philis status: **partial**. `CentroidGroup` is a budget with coincidence tolerance, and its doc says "the exact equality belongs in the pattern representation" (`kernel/analog/src/placement/cc.rs:14-28`).

### BAL1-45 Pairwise linear minimum distance (well, guard ring)
- Kind: rule / deck-requirement
- Statement:
  - ∀ m ∈ A, n ∉ A: d(m,n) ≥ d_min(m,n) ≥ 0 (3.16), and d_max(m) ≥ d_min(m,n) for all n (3.17).
  - Transistors in the same well may abut. Modules outside the well keep d_g (the guard-ring width) or the well spacing.
  - Plantage applied an n/p minimum distance in every example. It dominated the area overhead: example 3 is always above 121% because of it (L6780–6783).
  - With one well per symmetry group, a module keeps d_well from modules outside its well and 2·d_well from modules in other wells (L7739–7741).
- Source: §3.1.2, L4742–4771, Fig. 3.2 (PDF pp. 114–115); §3.6, L6660–6663, L7733–7748 (PDF pp. 148, 155).
- Philis stage: deck, dp, gp
- Automation recipe:
  - Classify every cell by body/well net and type: nwell-net id, p-substrate, pwell id, DTI tub.
  - d_min(m,n) = 0 (or the abutment rule) when the class and well net match, and the deck's diff/tap/poly spacing otherwise.
  - Use deck `nwell` spacing / enclosure + diff-to-nwell spacing when the classes differ, plus the guard-ring width when either cell is ringed.
  - Store a small `k×k` class matrix and replace the scalar `Rules::clearance` (`backend/gp/src/lib.rs:156-162`) with `clearance(class_a, class_b)`.
- Beats hand layout because: every pair gets its exact spacing, so no nwell-sized gap is left between two NMOS.
- Philis status: **partial, wasteful**. `place_rules` takes the **max** min-spacing over nwell/diff/tap/poly/nsdm/psdm/li and applies it to every pair (`frontend/library/src/lib.rs:529-537`).

### BAL1-46 Piecewise-linear DTI distance as a MIP
- Kind: formula / algorithm
- Statement:
  - Allowed gap: d ≤ s_max (shared, stretched trench) or d ≥ d_DTI (separate trenches); the band between is forbidden (3.18). Per module, trench stretch satisfies s_l,m + s_r,m ≤ s_max,m (3.40).
  - Per edge e with binary r_e (3.36):
    - (3.37) e − r_e·β ≤ s_max,e − s_r,m
    - (3.38) e + (1 − r_e)·β ≥ d_DTI
    - (3.39) e ≥ s_r,n + s_l,m
    - (3.41) e − s_l,m − s_r,n − r_e·β ≤ 0
  - Objective x_e + λ·Σ(s_l + s_r) (3.43).
  - β > Σ module widths + worst-case minimum distances × (#symmetry constraints + 1). λ < w_min/(N·d_DTI). Symbols verified against PDF pp. 136–137.
  - A 30-module DTI example took ~15 min at 110% area usage, with the MIP much slower than simplex.
- Source: §3.1.2, L4800–4863, Figs. 3.3–3.4; §3.3.2, L5940–6030; §3.6.4, L7995–8005 (PDF pp. 115–116, 135–137, 157).
- Philis stage: dp
- Automation recipe: Philis already models the disjunction as an SA branch variable `Layout::branch` flipped by dp. That is the SA equivalent of r_e, and a MIP is not needed. What is missing is the per-module stretch budget (3.40): a module stretched on both sides may use at most s_max,m in total. Add it as a check in `DtiBand`: each module's summed shared-gap stretch ≤ s_max,m.
- Beats hand layout because: every DTI pair's side is optimized jointly with the rest of the layout.
- Philis status: **partial** (`kernel/analog/src/placement/dti.rs:7-30`, `backend/dp/src/lib.rs:537`). The per-module stretch budget is missing.

### BAL1-47 Constraint-extraction method families
- Kind: heuristic (landscape)
- Statement: prior extraction used:
  - sensitivity analysis for parasitic, matching and symmetry constraints [5–7];
  - classifying nets by susceptibility to find building blocks and their matching [8];
  - structural symmetry detection via subgraph isomorphism, solved by graph labeling [9, 12] or recursive symmetric-pair detection [10, 11];
  - the sizing-rules library of building blocks [13].
- Source: §3.1.3, L4874–4889 (PDF p. 116).
- Philis stage: annotator
- Automation recipe: Philis has library matching (catalog) and net classes. Add recursive symmetric-pair detection (BAL1-51) to cover symmetric structures that no catalog entry matches.
- Beats hand layout because: symmetric pairs are found across the whole netlist, including the unrecognized glue.
- Philis status: **partial**. Pattern catalog with priorities (`backend/annotator/src/catalog.rs:13-24`); net classes (`backend/annotator/src/classify.rs`).

### BAL1-48 Requirement types and importance order M_S ≻ M_B ≻ P_B ≻ S ≻ P_N
- Kind: rule
- Statement:
  - Types:
    - M_B: matching inside a building block.
    - M_S: matching between the members of a symmetric pair.
    - P_B: proximity inside a building block.
    - S: symmetry.
    - P_N: netlist proximity.
  - Order (3.19): M_S ≻ M_B ≻ P_B ≻ S ≻ P_N.
  - Rationale: violating M_S (e.g. N1/N2 across halves) degrades offset, which is critical. Violating M_B (N1/N3 inside a mirror) equally on both halves shifts both operating points alike and degrades gain, which is less critical. P_B only acts inside blocks and so cannot harm symmetry, which acts on whole blocks. P_N is least important.
- Source: §3.2.1.1–3.2.1.2, L5029–5137, Table 3.1, Fig. 3.7 (PDF pp. 119–121).
- Philis stage: annotator, dp
- Automation recipe: tag each emitted batch with its τ. When two batches conflict (e.g. a pair's `MatchingPair` against a mirror's `Proximity`), use the order in `Prices` to set initial λ ratios, or as the tie-break inside the Θ tier. It also sets the order of HSMPG grouping (BAL1-52).
- Beats hand layout because: conflicts are resolved by a declared, consistent priority instead of case-by-case judgement.
- Philis status: **missing**. There is no requirement-type priority across batches; `Prices` weights by criticality.

### BAL1-49 SMP graph (typed multigraph of requirements)
- Kind: data-model
- Statement: nodes are devices. There is an edge per shared requirement, typed by τ ∈ T, and multi-edges are allowed. It is initialized with a clique per net (P_N: every device pair on net n), then filled with M_B/P_B from building blocks and M_S/S from symmetry analysis.
- Source: §3.2.2, Def. 3.10, L5141–5157, Figs. 3.8–3.15 (PDF pp. 122–126).
- Philis stage: annotator
- Automation recipe:
  - `Vec<(DeviceId, DeviceId, ReqType)>` built after `pattern::recognize`.
  - The source puts a clique on every net and does not say whether supply nets are excluded. Philis should exclude the Supply/Ground net classes, otherwise one component swallows the circuit. This exclusion is a Philis recommendation, not from the source.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-50 Building-block library to requirement mapping (Fig. 3.11)
- Kind: rule
- Statement:
  - Differential pair, level shifter and simple current mirror: match T1–T2.
  - Cascode, 4-transistor and wide-swing current mirrors: match T1–T2 (lower) and T3–T4 (upper), with building-block proximity T1–T3 and T2–T4.
  - For the example amplifier, the recognizer finds 5 simple CMs and 1 DP (L5224–5227).
- Source: §3.2.2.1, L5208–5227, Fig. 3.11 (PDF p. 123).
- Philis stage: annotator
- Automation recipe: check that composite mirrors emit P_B(T1, T3) and P_B(T2, T4) (a `Stack` Proximity) as well as M_B on both levels. The catalog already contains `cascode_mirror`, `wide_swing_cascode_mirror`, `current_mirror_4` and `level_shifter` (`backend/annotator/src/catalog.rs:732`, `:756`, `:1043`, `:1002`).
- Beats hand layout because: the cascode alignment is applied to every composite mirror automatically.
- Philis status: **implemented / partial**. Composites become `Group` with ≤2-device children; `Stack` emits `Proximity` (`backend/annotator/src/block.rs:37-56`, `backend/annotator/src/emit.rs:3-11`).

### BAL1-51 Symmetry analysis: compounds that span building blocks
- Kind: algorithm
- Statement: find symmetric device pairs (algorithm similar to Arsintescu [10]). All pairs sharing one axis form a symmetry compound C. Emit M_S per pair and an S clique over the compound (the axis coordinate eliminated). Example: p1 = (P1, P2), p2 = (N1, N2), p3 = (N3, N4) and p4 = (P3, P4) form one compound C1.
- Source: §3.2.2.2, L5230–5333, Figs. 3.13–3.15 (PDF pp. 123–126).
- Philis stage: annotator
- Automation recipe:
  - Recursive pair detection: seed with recognized diff pairs. Propagate along nets: if (a, b) is symmetric, the devices on net(a.D) and net(b.D) with identical (type, W, L, pin roles) pair up. Iterate to a fixpoint.
  - The union of pairs reached from one seed is one compound, with one AxisId.
  - This unifies the stage axes that today are per block.
- Beats hand layout because: the symmetry axis is propagated through the whole signal path, so no symmetric pair is missed.
- Philis status: **missing**. Symmetry comes only from 2-device leaves within one top-level block (`backend/annotator/src/emit.rs:3-16`).

### BAL1-52 HSMPG tree generation (Algorithm 3.1)
- Kind: algorithm
- Statement:
  - For τ from highest to lowest in T_I:
    1. G_τ = filter(G_SMP, τ).
    2. Take the connected components with more than one node.
    3. Create a group per component: MG if τ ∈ {M_B, M_S}, SG if τ = S, else PG.
    4. Contract each group to a super node, redirecting crossing edges.
  - The method is agglomerative clustering with SMP similarity.
  - Example: MG_S,1–4, then MG_B,1–2, then SG1, then PG_N,1 (root).
  - A static order was correct in all experiments. A dynamic priority Φ: E → ℕ from simulated violation impact is possible.
- Source: §3.2.3–3.2.3.3, L5355–5610, Algorithm 3.1, Fig. 3.16 (PDF pp. 126–129).
- Philis stage: annotator
- Automation recipe:
  - Implement it on BAL1-49's edge list with a union-find per τ pass; it is about 60 lines.
  - Output `Vec<Group { kind, children }>` as the placement hierarchy for dp (BAL1-31/41) and as the axis source (SG → AxisId).
  - Table 3.2 shows groups of size 2–14 (average 2.2–2.9), so enumeration per group is cheap.
- Beats hand layout because: the hierarchy is derived deterministically from the netlist and the constraint priorities.
- Philis status: **missing**. Blocks come from catalog matches, not from requirement clustering (`backend/annotator/src/lib.rs:1-7`).

### BAL1-53 Constraints from the HSMPG tree
- Kind: rule
- Statement:
  - Matching group: same-variant + alignment constraints. Replace alignment with common centroid when the devices consist of sub-devices.
  - Symmetry group: "symmetry (pair)" y_a = y_b for every M_S pair (3.23), plus "symmetry (groups)" x_MG,i = ½(x_m1 + x_m2) all equal across the SG (3.24)–(3.25).
  - Proximity groups: no explicit constraint, because bottom-up construction keeps the members together.
  - Table 3.2 counts constraints per circuit (alignment 1–10, device proximity 3–14, symmetry pairs 2–10, CC 0–2, variant 3–16, hierarchical proximity 2–9, minimum distance 1–2).
- Source: §3.2.4, L5613–5652; Table 3.2, L6633–6657 (PDF pp. 129–130, 148).
- Philis stage: annotator
- Automation recipe: emit an `Alignment` rule (the centre y of a matched pair are equal) for MG. `routing/align.rs` was deleted in the working tree (git status) and has no placement counterpart.
- Beats hand layout because: alignment is enforced as an equality on every matched pair.
- Philis status: **partial**. Symmetry pair y-equality exists (`kernel/analog/src/placement/symmetry.rs:10-16`). There is no alignment rule for non-symmetric matched groups (grep `align`: no hits in placement or annotator).

### BAL1-54 B*-tree to VCG (Algorithm 3.2) and LP height minimization
- Kind: algorithm
- Statement:
  - VCG: a left child (above, in the ch. 3 convention) is a direct successor. A right child shares its parent's predecessor. The start node precedes the root; a node without a left child connects to the end node. Each edge (n_i, n_j) gives y_i + h_i ≤ y_j.
  - LP: y_opt = argmin y_e s.t. M_v·y ≥ d_v (minimum distances) and C_v·y = k_v (symmetry, CC) (3.26)–(3.28). Minimizing y_e minimizes height.
- Source: §3.3–3.3.1, L5655–5765, Algorithm 3.2, Figs. 3.17–3.18 (PDF pp. 130–132; verified against the PDF).
- Philis stage: dp
- Automation recipe: this is the compaction core replacing `legalize::separate_overlaps`. Without symmetry or CC equalities, longest path on the VCG is exact and needs no LP. With them, either run Balasa's sweeps (BAL1-10/11) or add a small pure-Rust LP (e.g. `minilp` via `good_lp`) behind a feature. The legalizer's own ponytail note already names "constraint graph + LP compaction" as the upgrade (`backend/dp/src/legalize.rs:15-16`).
- Beats hand layout because: every candidate gets the minimum-area compaction that satisfies all equalities exactly.
- Philis status: **missing**.

### BAL1-55 Shadow-based HCG with minimum distances (Algorithm 3.3) and LP for x
- Kind: algorithm
- Statement:
  - Core shadow C_S,m = [y_m; y_m + h_m] (3.30). Partial shadow P_S,m = [y_m − d_max(m); y_m[ ∪ [y_m + h_m; y_m + h_m + d_max(m)] (3.31)–(3.32).
  - Sort and unify all y-endpoints into y-regions, stored in a tree. Initialize every region with the start node.
  - For modules in preorder, for each region in the core shadow: for each registered n with d_vert(m,n) < d_min(m,n), add edge n → m. Clear the region and register m. Also register m in its partial-shadow regions.
  - Remove multi-edges and connect the remaining region entries to the end node.
  - Only the edges needed for minimum distances are created: y-overlap pairs, plus diagonal pairs that are too close vertically.
  - Then x_opt = argmin x_e s.t. M_h·x ≥ d_x and C_h·x = k_h (3.33)–(3.35), by simplex.
- Source: §3.3.1, L5793–5936, Algorithm 3.3, Figs. 3.19–3.21 (PDF pp. 132–135).
- Philis stage: dp
- Automation recipe: build the HCG with pairwise d_min from BAL1-45. This also fixes the diagonal-proximity gap that pure y-overlap HCGs miss.
- Beats hand layout because: diagonal spacing violations are caught by construction, which a human checks only by DRC afterwards.
- Philis status: **missing**.

### BAL1-56 Shape functions and Pareto pruning
- Kind: algorithm
- Statement: a shape function is an ordered set of (w, h) shapes. Horizontal addition gives (w1 + w2, max(h1, h2)); vertical addition gives (max(w1, w2), h1 + h2). Combine all pairs, then drop any shape with larger h at ≤ w; the remainder is the Pareto front. Plain shape functions produce slicing structures only.
- Source: §3.4.1, L6043–6100, Figs. 3.23–3.24 (PDF pp. 137–138).
- Philis stage: dp, flow
- Automation recipe: see BAL1-57/60.
- Beats hand layout because: every aspect ratio's best area is available at once, so the floorplanner can choose.
- Philis status: **missing**.

### BAL1-57 Enhanced shape (w, h, B*-tree) and its dominance rule
- Kind: data-model
- Statement: an enhanced shape stores its B*-tree as well as (w, h). It is suboptimal if another has lower h at the same or lower w, or lower netlength at equal (w, h).
- Source: §3.4.2, L6104–6125 (PDF pp. 138–139).
- Philis stage: dp
- Automation recipe: `struct Shape { w: i32, h: i32, hpwl: f64, tree: BTree }`. Prune with a sort by w followed by a running-min-h scan. Extend the dominance key with Θ (the analog residual) so a smaller but more mismatched shape does not dominate. That extension is a Philis addition consistent with the lexicographic key (`frontend/library/src/lib.rs:336-339`).
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-58 Horizontal enhanced-shape addition
- Kind: algorithm
- Statement: attach β's root to α's **lowest-rightmost** node: the node with no right child whose ancestors are all right children or the root. In- and preorder relations within α and β are unchanged, so every constraint satisfied by α and β stays satisfied, provided no new constraint spans both. The result is at most (w_i + w_j, max(h_i, h_j)) and often smaller (w_sum < w1 + w2, Fig. 3.25).
- Source: §3.4.3, L6129–6209, eqs. (3.44)–(3.45) (PDF pp. 139–140).
- Philis stage: dp
- Automation recipe: one pointer assignment on cloned trees.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-59 Vertical enhanced-shape addition
- Kind: algorithm
- Statement: split β into segments rooted at its baseline modules (those reachable from the root through right edges only). Assign segments left to right. Each segment root becomes the left child of the α node that limits its downward shift, found from the x-projection shadow on α's contour. Relations (3.46)–(3.49) keep in/preorder relations, hence feasibility.
- Source: §3.4.3, L6212–6335, Figs. 3.27–3.29 (PDF pp. 140–142).
- Philis stage: dp
- Automation recipe: implement it on the same skyline as BAL1-28. Unit-test it on Fig. 3.29.
- Beats hand layout because: n/a.
- Philis status: **missing**.

### BAL1-60 Hierarchically bounded enumeration (Plantage, Algorithm 3.4)
- Kind: algorithm
- Statement:
  - Basic groups (leaf-sibling sets of the HSMPG tree): enumerate all B*-trees × all allowed variant combinations. Place each via LP/MIP (BAL1-54/55) and keep the Pareto enhanced shapes. The basic enumerations are independent and parallelizable.
  - Parents: combine the children's ESFs in **every sequence**, since addition is not commutative, using both horizontal and vertical addition for every shape pair. Place, then prune.
  - The final ESF is a Pareto front of complete placements with different aspect ratios.
  - Hierarchy order implies proximity.
  - Deterministic, with no tuning parameters. The root proximity group of example 5 was too large and was split by hand (L6624–6626).
- Source: §3.5, L6338–6583, Figs. 3.30–3.33 (PDF pp. 143–146).
- Philis stage: dp, flow
- Automation recipe:
  - Use it as a deterministic seed generator for dp: gp → Plantage ESF → pick the shape closest to the die aspect or the `Utilization` floor → dp refinement.
  - Cap basic-group size at ~6 modules (8 modules = 57.7 M trees) and fall back to SA for larger groups.
  - Parallelize the basic groups with `std::thread::scope`.
- Beats hand layout because: small groups are enumerated exhaustively, so the optimum is found there; a human tries a handful of arrangements.
- Philis status: **missing**.

### BAL1-61 Plantage results and runtime data
- Kind: metric
- Statement:
  - Table 3.2 (examples 1–5):

    | | 1 Miller | 2 Comparator | 3 Folded cascode | 4 Fully diff. | 5 Buffer |
    |---|---|---|---|---|---|
    | Devices | 9 | 10 | 22 | 30 | 42 |
    | Modules | 13 | 10 | 22 | 32 | 46 |
    | Variants per module | 3–6 | 2 | 2–4 | 2–3 | 2–7 |
    | Groups | 5 | 8 | 17 | 19 | 21 |
    | Pareto placements | 35 | 4 | 12 | 12 | 114 |
    | Best area usage | 115% | 110% | 121% | 129% | 111% |
    | Constraint generation | 0.3 s | 0.3 s | 0.5 s | 0.9 s | 1.2 s |
    | Plantage | 14 s | 1 s | 44 s | 691 s | 134 s |

  - Table 3.4 (area usage % / s):
    - biasynth_2p4g: SP 114.89/780; segment tree 114.89/246; SP+LP 106.38/403; SP-dummy 118.51/134; symmetry islands 104.68/22; SP with Johnson N/A; Plantage 104.96/337
    - lnamixbias_2p4g: 110.43/2,824; 109.35/726; 108.59/3,252; 113.50/227; 105.72/43; 109/480; 107.68/387
  - With wells: 107.74% (593 s, +76% CPU) and 109.24% (664 s, +72%).
- Source: §3.6, L6602–6673, L7417–7748, Tables 3.2–3.4 (PDF pp. 147–155).
- Philis stage: benchmarks
- Automation recipe: record area usage (BAL1-62) per fixture in `benchmarks/src/bench.rs` so it is comparable with these numbers.
- Beats hand layout because: n/a (calibration).
- Philis status: **missing** as a metric.

### BAL1-62 Area-usage metric
- Kind: metric
- Statement: area usage = bounding-rectangle area / area used by modules and wells. The ideal is 100%, which constraints and device shapes prevent in general. It is used as the quality metric when total module area is constant.
- Source: §3.6.1, L6666–6668; §3.6.2, L7429–7430 (PDF pp. 148, 153).
- Philis stage: verify, benchmarks
- Automation recipe: `Report.area_usage = footprint / Σ cell area`, and include well area once cells report well polygons. Print it in the bench table next to the utilization floor.
- Beats hand layout because: n/a.
- Philis status: **partial**. `Utilization` computes footprint · u_min / Σ cell area as a budget (`kernel/analog/src/placement/utilization.rs:6-26`), but no area-usage number is reported against a reference.

### BAL1-63 Pareto front of aspect ratios as the flow output
- Kind: data-model / flow
- Statement: Plantage returns a set of area-optimal placements with different aspect ratios instead of a single layout, so the designer chooses. Example 5 has 114 placements; P39 appears in three different variants across the shown placements.
- Source: §3.1.4, L4968–4986; §3.5, L6582–6583; §3.6.1, L7159–7163 (PDF pp. 118, 146, 152).
- Philis stage: flow
- Automation recipe: have `library::run` keep the k best epochs that are non-dominated in (footprint w, footprint h, Θ, PEX), not only the lex-best, and expose them in `RunStats`. This fits the multi-start loop (`frontend/library/src/lib.rs:183-189`, `:336-372`).
- Beats hand layout because: a manual layout offers one shape; the tool offers the whole trade-off curve.
- Philis status: **missing** (a single lexicographic winner).

### BAL1-64 Device merging and generator reshaping inside placement
- Kind: rule
- Statement: an analog placer must exploit device merging (geometry sharing: shared diffusion, which reduces area and parasitics) and the reshaping ability of module generators during placement. The absolute representation's advantage is that beneficial overlaps can be explored.
- Source: §1.1.2, L583–590; footnote 2, L783–785 (PDF pp. 19, 23).
- Philis stage: cells, dp
- Automation recipe: keep the abutment table as permission. In a tree representation, merged devices become one module, or a pair with d_min = −(shared diffusion width).
- Beats hand layout because: every legal merge is tried during search, where a human merges only the obvious ones.
- Philis status: **partial**. dp reads `groups` as abutment permission from the diffusion-sharing table (`frontend/library/src/lib.rs:577-579`), and variants are reshaped (`backend/dp/src/lib.rs:584`).

### BAL1-65 Segment-tree reuse across SA iterations
- Kind: heuristic
- Statement: allocate the contour structure once, sized for the full n, and only reset it per SA iteration. Rebuilding it costs 15–20% of evaluation time.
- Source: §1.2.1, L1108–1171 (PDF pp. 30–32).
- Philis stage: dp
- Automation recipe: in any new packer, keep the skyline `Vec` in the `Sa` struct and `clear()` it instead of reallocating. dp's `Snap` already follows this pattern (`backend/dp/src/lib.rs:34-77`).
- Beats hand layout because: n/a.
- Philis status: n/a (the pattern is already followed).

---

## 4. Top-15 priorities for Philis

1. **Pairwise, well-class minimum distance instead of one global clearance** (BAL1-45, BAL1-32). Philis applies the max layer spacing between every pair (`frontend/library/src/lib.rs:529-537`). The source shows the n/p distance dominates area overhead. It is the cheapest large area win.
2. **ASF-B*-tree per symmetry group inside an HB*-tree as dp's representation** (BAL1-28–BAL1-35, BAL1-04). Symmetry, non-overlap and symmetry islands hold by construction, with O(n) packing (Table 2.2). This retires projection repair and the terminal O(n²) legalizer.
3. **Constraint-graph compaction (VCG/HCG with shadows) with exact symmetry via Balasa sweeps, LP only if CC equalities are needed** (BAL1-54, BAL1-55, BAL1-10, BAL1-11). This gives exact, minimum-area legalization and removes the `legalize.rs` ponytail ceiling.
4. **HSMPG tree from the SMP graph with importance order M_S ≻ M_B ≻ P_B ≻ S ≻ P_N** (BAL1-48, BAL1-49, BAL1-52, BAL1-53). This gives the placement hierarchy and priorities that the annotator's two-level blocks lack.
5. **Symmetry compounds across blocks (propagated pair detection) sharing one axis** (BAL1-51, BAL1-38). Today each block gets its own axis, so a fully differential path is split into several axes.
6. **Symmetry-island and cluster-connectivity check in the report and Θ** (BAL1-27, BAL1-41). A cheap union-find check that also measures item 2's benefit.
7. **Pair-coupled reshape (same-variant constraint in dp)** (BAL1-43). A single-cell reshape can give matched devices different variants.
8. **Plantage-style bounded enumeration with enhanced shape functions as a deterministic seed** (BAL1-56–BAL1-60). Exhaustive for groups of ≤6 modules; parallel; replaces gp's random start for small blocks.
9. **Pareto set of aspect ratios from the epoch loop** (BAL1-63, BAL1-57). The designer or top-level floorplanner chooses a shape instead of accepting one lex-winner.
10. **Horizontal-axis symmetry type, with the axis direction from rails and signal flow** (BAL1-01, BAL1-34 Op5).
11. **Hierarchical symmetry: group pairs and nested groups** (BAL1-40, BAL1-12). Needed for fully differential circuits with CMFB and replicated halves.
12. **Mirror vs perfect symmetry choice per pair** (BAL1-02). Mirror enables reflected routing in dr; perfect handles tilted implants. Today only perfect is produced.
13. **Area-usage metric plus biasynth/lnamixbias-style module-only fixtures** (BAL1-62, BAL1-61, BAL1-37). A quantitative calibration against published placers.
14. **DTI per-module stretch budget (3.40)** (BAL1-46). This completes the existing `DtiBand` disjunction.
15. **Common-centroid equality for split sub-devices placed as separate modules, in the compaction LP** (BAL1-44, BAL1-53). This complements the pattern-exact CC inside `cells`.

Not recommended (YAGNI at ≤100 cells per hierarchical level, L1605): segment tree, RB interval tree, deterministic skip list and Johnson queue (BAL1-14–17), and TCG (BAL1-19). An O(n²) skyline or `BTreeMap` covers the same need.
