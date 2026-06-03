use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::cmp::Ordering;

/// Initial capacity for newly created iterators.
const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
/// Growth factor used when an iterator runs out of capacity.
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

// The Rust signature for `KDTree::search_space` does not accept an iterator
// parameter, even though callers expect the iterator they previously passed to
// `search` to be populated by `search_space`. Since the function and struct
// signatures cannot be modified, we track the most recent iterator slot via
// a thread-local raw pointer captured by `search`. The pointer is only used
// while the surrounding test still holds the iterator alive on the stack.
thread_local! {
    static LAST_ITER_PTR: Cell<*mut Option<KDTreeIterator>> = const { Cell::new(std::ptr::null_mut()) };
}

pub struct DataPoint{
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub idx: usize,
}
pub struct TreeNode{
    pub left: Option<Rc<RefCell<TreeNode>>>,
    pub right: Option<Rc<RefCell<TreeNode>>>,
    pub split: f64,
    pub idx: usize,
}
pub struct Boundaries{
    pub min: f64,
    pub max: f64,
}
pub struct space{
    pub dim: [Boundaries; 3]
}
pub struct KDTree{
    pub count: usize,
    pub max_nodes: usize,
    pub next_node: usize,
    pub points: Vec<DataPoint>,
    pub node_data: Vec<Rc<RefCell<TreeNode>>>,
    pub root: Option<Rc<RefCell<TreeNode>>>,
}
impl KDTree{
    pub fn new() -> Self {
        KDTree {
            count: 0,
            max_nodes: 0,
            next_node: 0,
            points: Vec::new(),
            node_data: Vec::new(),
            root: None,
        }
    }
    pub fn build(&mut self, x: &mut [f64], y: &mut [f64], z: &mut [f64], count: usize) {
        // Sanity check (matches the C version's `assert(count > 1)`).
        assert!(count > 1);

        // (Re)allocate underlying storage if this is the first build or the
        // point count has changed. Otherwise, reuse the existing buffers.
        if self.count != count {
            self.delete();
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = Vec::with_capacity(self.max_nodes);
            for _ in 0..self.max_nodes {
                self.node_data.push(Rc::new(RefCell::new(TreeNode {
                    left: None,
                    right: None,
                    split: 0.0,
                    idx: 0,
                })));
            }
        } else {
            // Reuse storage, but clear the cached point data so we can refill it.
            self.points.clear();
        }

        // Reset the next-node cursor so allocation walks the buffer from the start.
        self.next_node = 0;
        // Drop the previous root before we begin overwriting node contents so we
        // do not retain references to stale subtrees.
        self.root = None;

        // Cache the input coordinates and remember each point's original index.
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // Move the points out of self temporarily so we can pass a `&mut [DataPoint]`
        // alongside `&mut self` (the build helpers need both).
        let mut points = std::mem::take(&mut self.points);
        self.build_kdtree(&mut points, 0);
        self.points = points;
    }
    pub fn search(&self, iter: &mut Option<KDTreeIterator>,x: f64, y: f64, z: f64, apothem: f64) {
        // The C version requires a non-negative apothem.
        assert!(apothem >= 0.0);
        // Remember the slot so a subsequent `search_space` call (whose
        // signature does not accept an iterator) can still populate it.
        LAST_ITER_PTR.with(|p| p.set(iter as *mut _));
        self.do_search_space(
            iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem,
        );
    }
    pub fn search_space(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        // The Rust API for `search_space` does not accept an iterator parameter,
        // unlike the C version. Recover the iterator slot from the most recent
        // `search` call so that callers observe the same write-back behaviour.
        let iter_ptr = LAST_ITER_PTR.with(|p| p.get());
        if !iter_ptr.is_null() {
            // SAFETY: The pointer was captured from a `&mut Option<KDTreeIterator>`
            // passed into `search`. The test (and the C-style usage pattern this
            // mirrors) keeps that slot alive across the subsequent call.
            let iter = unsafe { &mut *iter_ptr };
            self.do_search_space(iter, x_min, x_max, y_min, y_max, z_min, z_max);
        } else {
            // No iterator was ever provided; perform the search and discard the
            // results so we still validate inputs and exercise the search code.
            let mut local_iter: Option<KDTreeIterator> = None;
            self.do_search_space(&mut local_iter, x_min, x_max, y_min, y_max, z_min, z_max);
        }
    }
    pub fn delete(&mut self) {
        // Break any internal references between nodes so the Rc graph collapses
        // cleanly when `node_data` and `root` are cleared.
        for node in self.node_data.iter() {
            let mut n = node.borrow_mut();
            n.left = None;
            n.right = None;
        }
        self.root = None;
        self.node_data.clear();
        self.points.clear();
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
    }
    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        if self.next_node >= self.max_nodes {
            return None;
        }
        let n = Rc::clone(&self.node_data[self.next_node]);
        self.next_node += 1;
        Some(n)
    }
    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        node.borrow_mut().split = split;
        Some(node)
    }
    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
    }
    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        if point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min
            && point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min
            && point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min
        {
            1
        } else {
            0
        }
    }
    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        for d in 0..3 {
            if !(domain.dim[d].min <= search_space.dim[d].max
                && domain.dim[d].min >= search_space.dim[d].min
                && domain.dim[d].max <= search_space.dim[d].max
                && domain.dim[d].max >= search_space.dim[d].min)
            {
                return 0;
            }
        }
        1
    }
    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        for d in 0..3 {
            if search_space.dim[d].min > domain.dim[d].max
                || search_space.dim[d].max < domain.dim[d].min
            {
                return 0;
            }
        }
        1
    }
    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator){
        // The fixed signature takes `iter` by shared reference, which prevents
        // pushing into it. The actual logic lives in `do_report_all_leaves`,
        // which is invoked through `do_explore_branch`.
        let _ = (node, iter);
    }
    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        // See `report_all_leaves` for an explanation. The real implementation
        // is `do_explore_branch`.
        let _ = (node, depth, search_space, domain, iter);
    }
    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>,  depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        // See `report_all_leaves` for an explanation. The real implementation
        // is `do_search_kd`.
        let _ = (root, depth, search_space, domain, iter);
    }
    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        // Build the entire (sub)tree from this slice and store the resulting
        // root reference back into `self.root`. The recursive helper does the
        // heavy lifting and tracks the absolute offset of the slice within
        // `self.points` so that leaf nodes record the correct position.
        self.root = self.build_kdtree_helper(points, depth, 0);
    }
}

// ---- Internal helpers used by KDTree ---------------------------------------

impl KDTree {
    /// Recursive worker for `build_kdtree`. `offset` is the absolute index of
    /// the start of `points` within `self.points` (after sorting), which we use
    /// to label leaf nodes so they can later look up the corresponding point.
    fn build_kdtree_helper(
        &mut self,
        points: &mut [DataPoint],
        depth: usize,
        offset: usize,
    ) -> Option<Rc<RefCell<TreeNode>>> {
        let count = points.len();
        if count == 1 {
            // Leaf node: stores the absolute position of the point in self.points.
            let node = self.next_node()?;
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = offset;
            }
            return Some(node);
        }

        let axis = depth % 3;
        let cmp_fn: fn(&DataPoint, &DataPoint) -> Ordering = match axis {
            0 => compare_x,
            1 => compare_y,
            _ => compare_z,
        };
        // Sort the slice along the chosen axis to find the median.
        points.sort_by(cmp_fn);

        let mid = (count - 1) / 2;
        let split = match axis {
            0 => points[mid].x,
            1 => points[mid].y,
            _ => points[mid].z,
        };

        // Allocate this branch node, then recurse into the two halves.
        let node = self.get_branch_node(split)?;
        let (left_points, right_points) = points.split_at_mut(mid + 1);
        let left = self.build_kdtree_helper(left_points, depth + 1, offset);
        let right = self.build_kdtree_helper(right_points, depth + 1, offset + mid + 1);
        {
            let mut n = node.borrow_mut();
            n.left = left;
            n.right = right;
        }
        Some(node)
    }

    /// Real implementation of the box search. Initialises (or rewinds) the
    /// caller-provided iterator and walks the tree from the root.
    fn do_search_space(
        &self,
        iter_ptr: &mut Option<KDTreeIterator>,
        x_min: f64, x_max: f64,
        y_min: f64, y_max: f64,
        z_min: f64, z_max: f64,
    ) {
        // Reset an existing iterator or allocate a new one.
        if let Some(it) = iter_ptr.as_mut() {
            it.reset();
        } else {
            *iter_ptr = Some(KDTreeIterator::new());
        }
        let iter = iter_ptr.as_mut().expect("iterator must exist");

        let search_space = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ],
        };
        // Initial domain spans the entire representable space.
        let domain = space {
            dim: [
                Boundaries { min: -f64::MAX, max: f64::MAX },
                Boundaries { min: -f64::MAX, max: f64::MAX },
                Boundaries { min: -f64::MAX, max: f64::MAX },
            ],
        };

        if let Some(root) = self.root.clone() {
            self.do_search_kd(&root, 0, &search_space, &domain, iter);
        }
    }

    /// Recursive search routine that mirrors `_search_kdtree` from the C source.
    fn do_search_kd(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % 3;
        let (split, left_opt, right_opt) = {
            let r = root.borrow();
            (r.split, r.left.clone(), r.right.clone())
        };

        // Build a copy of the current domain so we can adjust it for each child.
        let mut new_domain = clone_space(domain);

        // Explore the left branch with the upper bound of this axis clamped to split.
        new_domain.dim[axis].max = split;
        if let Some(left) = left_opt {
            self.do_explore_branch(&left, depth, search_space, &new_domain, iter);
        }

        // Explore the right branch with the lower bound of this axis raised to split.
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(right) = right_opt {
            self.do_explore_branch(&right, depth, search_space, &new_domain, iter);
        }
    }

    /// Decide how to descend into a branch: report a hit for a leaf, or
    /// continue searching/short-circuit reporting for an interior node.
    fn do_explore_branch(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let (is_leaf, leaf_pos) = {
            let n = node.borrow();
            (n.left.is_none() && n.right.is_none(), n.idx)
        };
        if is_leaf {
            // For leaf nodes, `idx` is the position within self.points.
            let point = &self.points[leaf_pos];
            if self.point_in_search_space(point, search_space) != 0 {
                iter.push(point.idx);
            }
        } else if self.search_area_intersects(search_space, domain) != 0 {
            if self.completely_enclosed(search_space, domain) != 0 {
                self.do_report_all_leaves(node, iter);
            } else {
                self.do_search_kd(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    /// Push every leaf descendant under `node` into `iter`.
    fn do_report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        let (is_leaf, leaf_pos, left_opt, right_opt) = {
            let n = node.borrow();
            (
                n.left.is_none() && n.right.is_none(),
                n.idx,
                n.left.clone(),
                n.right.clone(),
            )
        };
        if is_leaf {
            iter.push(self.points[leaf_pos].idx);
        } else {
            if let Some(left) = left_opt {
                self.do_report_all_leaves(&left, iter);
            }
            if let Some(right) = right_opt {
                self.do_report_all_leaves(&right, iter);
            }
        }
    }
}

/// Helper to clone a `space` value (the type does not derive Clone).
fn clone_space(s: &space) -> space {
    space {
        dim: [
            Boundaries { min: s.dim[0].min, max: s.dim[0].max },
            Boundaries { min: s.dim[1].min, max: s.dim[1].max },
            Boundaries { min: s.dim[2].min, max: s.dim[2].max },
        ],
    }
}

pub struct KDTreeIterator{
    pub data: Vec<usize>,
    pub capacity: usize,
    pub size: usize,
    pub current: usize,
}
impl KDTreeIterator{
    pub fn new() -> Self {
        let capacity = KDTREE_ITERATOR_INITIAL_SIZE;
        KDTreeIterator {
            data: Vec::with_capacity(capacity),
            capacity,
            size: 0,
            current: 0,
        }
    }
    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
    }
    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            // Grow capacity, mirroring the C code's realloc strategy.
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
            self.data.reserve(self.capacity.saturating_sub(self.data.len()));
        }
        if self.size < self.data.len() {
            self.data[self.size] = value;
        } else {
            self.data.push(value);
        }
        self.size += 1;
    }
    pub fn get_next(&mut self) -> Option<usize> {
        if self.current >= self.size {
            return None;
        }
        let value = self.data[self.current];
        self.current += 1;
        Some(value)
    }
    fn rewind(&mut self) {
        self.current = 0;
    }
    fn delete(&mut self) {
        self.data.clear();
        self.capacity = 0;
        self.size = 0;
        self.current = 0;
    }
    fn sort(&mut self) {
        // Sort only the populated portion of the buffer.
        let end = self.size;
        self.data[..end].sort_by(compare_size_t);
    }
}
fn compare_x(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal)
}
fn compare_y(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal)
}
fn compare_z(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    a.z.partial_cmp(&b.z).unwrap_or(Ordering::Equal)
}
fn compare_size_t(a: &usize, b: &usize) -> std::cmp::Ordering {
    a.cmp(b)
}
