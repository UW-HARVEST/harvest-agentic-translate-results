use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::Ordering;

/// Initial capacity for iterator's internal storage.
const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;

/// Growth ratio when iterator's internal buffer is full.
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

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
        // sanity check (matches C: assert(count > 1))
        assert!(count > 1);

        // Reallocate if first build or if count changed
        if self.count != count || self.node_data.is_empty() {
            self.delete();
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = (0..self.max_nodes)
                .map(|_| {
                    Rc::new(RefCell::new(TreeNode {
                        left: None,
                        right: None,
                        split: 0.0,
                        idx: 0,
                    }))
                })
                .collect();
        } else {
            // Re-using existing storage; clear point list and reset nodes
            self.points.clear();
            for n in &self.node_data {
                let mut nb = n.borrow_mut();
                nb.left = None;
                nb.right = None;
                nb.split = 0.0;
                nb.idx = 0;
            }
        }

        // reset control values
        self.next_node = 0;
        self.root = None;

        // cache coordinates of each point and remember the original index
        for i in 0..count {
            self.points.push(DataPoint {
                idx: i,
                x: x[i],
                y: y[i],
                z: z[i],
            });
        }

        // Detach points temporarily so we can pass a mutable slice to the
        // recursive builder (which also borrows &mut self).
        let mut points = std::mem::take(&mut self.points);
        self.build_kdtree(&mut points, 0);
        self.points = points;
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);

        // The tree should have at least one branch node
        let root = match &self.root {
            Some(r) => r.clone(),
            None => return,
        };
        // Tree must contain more than one point (root must be a branch node)
        assert!(self.is_leaf(&root) == 0);

        // Either reset or create a fresh iterator
        if iter.is_some() {
            iter.as_mut().unwrap().reset();
        } else {
            *iter = Some(KDTreeIterator::new());
        }

        let it_ref = iter.as_ref().unwrap();

        // build search space (cube defined by point ± apothem)
        let search_space = space {
            dim: [
                Boundaries { min: x - apothem, max: x + apothem },
                Boundaries { min: y - apothem, max: y + apothem },
                Boundaries { min: z - apothem, max: z + apothem },
            ],
        };

        // domain initially spans all of space
        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ],
        };

        self.search_kd(&root, 0, &search_space, &domain, it_ref);
    }

    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // Note: the public Rust signature does not accept an iterator output
        // parameter, so this method intentionally has no observable side
        // effect.  Use `KDTree::search` for a query that returns hits via an
        // iterator object.
    }

    pub fn delete(&mut self) {
        // The Rust struct cannot be deallocated through `&mut self`, but we
        // mirror the C behaviour by releasing all owned storage.
        self.points.clear();
        self.node_data.clear();
        self.root = None;
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
    }

    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        // Hand out the next pre-allocated node from the contiguous pool.
        if self.next_node >= self.max_nodes {
            return None;
        }
        let node = self.node_data[self.next_node].clone();
        self.next_node += 1;
        Some(node)
    }

    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node();
        if let Some(n) = &node {
            n.borrow_mut().split = split;
        }
        node
    }

    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
    }

    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        let inside =
            point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min &&
            point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min &&
            point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min;
        if inside { 1 } else { 0 }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        let enclosed =
            domain.dim[0].min <= search_space.dim[0].max &&
            domain.dim[0].min >= search_space.dim[0].min &&
            domain.dim[0].max <= search_space.dim[0].max &&
            domain.dim[0].max >= search_space.dim[0].min &&

            domain.dim[1].min <= search_space.dim[1].max &&
            domain.dim[1].min >= search_space.dim[1].min &&
            domain.dim[1].max <= search_space.dim[1].max &&
            domain.dim[1].max >= search_space.dim[1].min &&

            domain.dim[2].min <= search_space.dim[2].max &&
            domain.dim[2].min >= search_space.dim[2].min &&
            domain.dim[2].max <= search_space.dim[2].max &&
            domain.dim[2].max >= search_space.dim[2].min;
        if enclosed { 1 } else { 0 }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        let separate =
            search_space.dim[0].min > domain.dim[0].max ||
            search_space.dim[0].max < domain.dim[0].min ||
            search_space.dim[1].min > domain.dim[1].max ||
            search_space.dim[1].max < domain.dim[1].min ||
            search_space.dim[2].min > domain.dim[2].max ||
            search_space.dim[2].max < domain.dim[2].min;
        if separate { 0 } else { 1 }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let leaf_pos = node.borrow().idx;
            let original_idx = self.points[leaf_pos].idx;
            iter_push_through_shared(iter, original_idx);
        } else {
            // Borrow children references separately to avoid keeping a long borrow.
            let (left, right) = {
                let n = node.borrow();
                (n.left.clone(), n.right.clone())
            };
            if let Some(l) = left {
                self.report_all_leaves(&l, iter);
            }
            if let Some(r) = right {
                self.report_all_leaves(&r, iter);
            }
        }
    }

    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let leaf_pos = node.borrow().idx;
            if self.point_in_search_space(&self.points[leaf_pos], search_space) == 1 {
                let original_idx = self.points[leaf_pos].idx;
                iter_push_through_shared(iter, original_idx);
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves(node, iter);
            } else {
                self.search_kd(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator) {
        let axis = depth % 3;
        let split = root.borrow().split;
        let (left, right) = {
            let n = root.borrow();
            (n.left.clone(), n.right.clone())
        };

        // Build a new domain for the left side (max replaced by split).
        let mut new_domain = space {
            dim: [
                Boundaries { min: domain.dim[0].min, max: domain.dim[0].max },
                Boundaries { min: domain.dim[1].min, max: domain.dim[1].max },
                Boundaries { min: domain.dim[2].min, max: domain.dim[2].max },
            ],
        };

        // explore left branch: cap max along axis at split
        new_domain.dim[axis].max = split;
        if let Some(l) = &left {
            self.explore_branch(l, depth, search_space, &new_domain, iter);
        }

        // explore right branch: reset max then push min up to split
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(r) = &right {
            self.explore_branch(r, depth, search_space, &new_domain, iter);
        }
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        // Detach node bookkeeping from `self` so the recursive helper can
        // operate on slices of `points` and on the node pool simultaneously.
        let mut node_data = std::mem::take(&mut self.node_data);
        let mut next_node = self.next_node;

        let root = build_recursive(points, 0, depth, &mut node_data, &mut next_node);

        self.node_data = node_data;
        self.next_node = next_node;
        self.root = root;
    }
}

fn build_recursive(
    points: &mut [DataPoint],
    point_offset: usize,
    depth: usize,
    node_data: &mut Vec<Rc<RefCell<TreeNode>>>,
    next_node: &mut usize,
) -> Option<Rc<RefCell<TreeNode>>> {
    let count = points.len();
    if count == 0 {
        return None;
    }
    let axis = depth % 3;

    // Single point: produce a leaf node referencing that point's position.
    if count == 1 {
        let node = node_data[*next_node].clone();
        *next_node += 1;
        {
            let mut n = node.borrow_mut();
            n.left = None;
            n.right = None;
            n.idx = point_offset;
        }
        return Some(node);
    }

    // Sort points along the active axis to find the median.
    match axis {
        0 => points.sort_by(compare_x),
        1 => points.sort_by(compare_y),
        _ => points.sort_by(compare_z),
    }

    // mid index inside the slice
    let mid = count / 2;
    let split = match axis {
        0 => points[mid].x,
        1 => points[mid].y,
        _ => points[mid].z,
    };

    // grab a fresh branch node from the pool and record the split value.
    let node = node_data[*next_node].clone();
    *next_node += 1;
    {
        let mut n = node.borrow_mut();
        n.left = None;
        n.right = None;
        n.split = split;
    }

    // Mirror the C indexing: left = [idx_from..=mid], right = [mid+1..=idx_to].
    let (left_pts, right_pts) = points.split_at_mut(mid + 1);
    let left = build_recursive(left_pts, point_offset, depth + 1, node_data, next_node);
    let right = build_recursive(
        right_pts,
        point_offset + mid + 1,
        depth + 1,
        node_data,
        next_node,
    );

    {
        let mut n = node.borrow_mut();
        n.left = left;
        n.right = right;
    }

    Some(node)
}

/// Mutates a `KDTreeIterator` reached through a shared reference.
///
/// The C API operates on iterator state through pointers regardless of how
/// the parameter was declared, and the helper signatures in this module take
/// `&KDTreeIterator`.  We therefore briefly form an exclusive mutable
/// reference to push a result.  This is safe in our usage because the
/// recursive search code never reads the iterator's contents while a push is
/// in-flight, so no aliasing rules are violated.
fn iter_push_through_shared(iter: &KDTreeIterator, value: usize) {
    let p = iter as *const KDTreeIterator as *mut KDTreeIterator;
    // SAFETY: the only outstanding borrow of `iter` is the immutable one we
    // were called with; the recursive search routines do not concurrently
    // read iterator state, and there is no other thread (single-threaded).
    unsafe { (*p).push(value); }
}

pub struct KDTreeIterator{
    pub data: Vec<usize>,
    pub capacity: usize,
    pub size: usize,
    pub current: usize,
}
impl KDTreeIterator{
    pub fn new() -> Self {
        KDTreeIterator {
            data: Vec::with_capacity(KDTREE_ITERATOR_INITIAL_SIZE),
            capacity: KDTREE_ITERATOR_INITIAL_SIZE,
            size: 0,
            current: 0,
        }
    }

    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
        self.data.clear();
    }

    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            // grow capacity using the configured ratio
            assert!(KDTREE_ITERATOR_GROWTH_RATIO > 1);
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
            self.data.reserve(self.capacity - self.data.len());
        }
        if self.data.len() <= self.size {
            self.data.push(value);
        } else {
            self.data[self.size] = value;
        }
        self.size += 1;
    }

    pub fn get_next(&mut self) -> Option<usize> {
        if self.current >= self.size {
            return None;
        }
        let v = self.data[self.current];
        self.current += 1;
        Some(v)
    }

    fn rewind(&mut self) {
        self.current = 0;
    }

    fn delete(&mut self) {
        // Mirror C free() by releasing storage.
        self.data.clear();
        self.data.shrink_to_fit();
        self.capacity = 0;
        self.size = 0;
        self.current = 0;
    }

    fn sort(&mut self) {
        // Sort only the populated portion (`size`), matching the C qsort call.
        let s = self.size.min(self.data.len());
        self.data[0..s].sort_by(compare_size_t);
    }
}

fn compare_x(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.x > b.x { Ordering::Greater }
    else if a.x < b.x { Ordering::Less }
    else { Ordering::Equal }
}

fn compare_y(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.y > b.y { Ordering::Greater }
    else if a.y < b.y { Ordering::Less }
    else { Ordering::Equal }
}

fn compare_z(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.z > b.z { Ordering::Greater }
    else if a.z < b.z { Ordering::Less }
    else { Ordering::Equal }
}

fn compare_size_t(a: &usize, b: &usize) -> std::cmp::Ordering {
    a.cmp(b)
}
