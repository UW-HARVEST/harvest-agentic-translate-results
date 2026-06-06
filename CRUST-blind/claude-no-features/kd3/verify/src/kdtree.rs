use std::cell::Cell;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

// The internal traversal helpers below match the original C signatures, which
// receive a (logically mutable) iterator by shared reference. Because we are
// not allowed to change struct or function signatures, we route mutation
// through a thread-local raw pointer. The pointer is set by `KDTree::search`
// (which holds an `&mut KDTreeIterator`), used by the recursive traversal
// helpers, and cleared on completion. This keeps the unsafe surface tiny.
thread_local! {
    static CURRENT_ITER: Cell<*mut KDTreeIterator> = const { Cell::new(std::ptr::null_mut()) };
}

struct IterGuard;

impl IterGuard {
    fn install(iter: &mut KDTreeIterator) -> Self {
        CURRENT_ITER.with(|c| c.set(iter as *mut KDTreeIterator));
        IterGuard
    }
}

impl Drop for IterGuard {
    fn drop(&mut self) {
        CURRENT_ITER.with(|c| c.set(std::ptr::null_mut()));
    }
}

fn iter_push_global(value: usize) {
    let ptr = CURRENT_ITER.with(|c| c.get());
    if ptr.is_null() {
        return;
    }
    // SAFETY: `ptr` was set from a live `&mut KDTreeIterator` and the guard
    // ensures it is cleared before the borrow ends. Traversal is single
    // threaded and the pointer is not aliased while mutation happens here.
    let iter = unsafe { &mut *ptr };
    iter.push(value);
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

fn make_infinite_domain() -> space {
    space {
        dim: [
            Boundaries { min: f64::MIN, max: f64::MAX },
            Boundaries { min: f64::MIN, max: f64::MAX },
            Boundaries { min: f64::MIN, max: f64::MAX },
        ],
    }
}

fn clone_space(s: &space) -> space {
    space {
        dim: [
            Boundaries { min: s.dim[0].min, max: s.dim[0].max },
            Boundaries { min: s.dim[1].min, max: s.dim[1].max },
            Boundaries { min: s.dim[2].min, max: s.dim[2].max },
        ],
    }
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
        assert!(count > 1, "kdtree requires at least two points");

        // Reuse object if possible. If the count differs, drop existing state.
        if self.count != count {
            self.delete();
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
        }

        // Reset control values and storage that we will repopulate.
        self.next_node = 0;
        self.node_data.clear();
        self.root = None;

        // Cache coordinates, mapping each to its original index.
        self.points.clear();
        self.points.reserve(count);
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // Take points out so we can pass &mut [DataPoint] while still using &mut self.
        let mut points = std::mem::take(&mut self.points);
        self.build_kdtree(&mut points[..], 0);
        self.points = points;
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);

        // Either reuse existing iterator or create a new one.
        if iter.is_some() {
            iter.as_mut().unwrap().reset();
        } else {
            *iter = Some(KDTreeIterator::new());
        }

        let search_space = space {
            dim: [
                Boundaries { min: x - apothem, max: x + apothem },
                Boundaries { min: y - apothem, max: y + apothem },
                Boundaries { min: z - apothem, max: z + apothem },
            ],
        };
        let domain = make_infinite_domain();

        let it_mut = iter.as_mut().unwrap();
        let _guard = IterGuard::install(it_mut);
        // Re-borrow as shared so we can match the recursion's `&KDTreeIterator` signature.
        let it: &KDTreeIterator = it_mut;

        if let Some(root) = self.root.clone() {
            // The tree must contain at least one branch node.
            assert!(self.is_leaf(&root) == 0);
            self.search_kd(&root, 0, &search_space, &domain, it);
        }
    }

    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // The Rust signature exposes no iterator/output parameter, so this
        // method has no externally observable effect. The C implementation
        // produces results into a `kdtree_iterator`; in the Rust API the
        // caller should use `search` (which forwards to a search-space style
        // traversal internally).
    }

    pub fn delete(&mut self) {
        self.points.clear();
        self.node_data.clear();
        self.root = None;
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
    }

    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        let node = Rc::new(RefCell::new(TreeNode {
            left: None,
            right: None,
            split: 0.0,
            idx: 0,
        }));
        self.node_data.push(node.clone());
        self.next_node += 1;
        Some(node)
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
        let in_x = point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min;
        let in_y = point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min;
        let in_z = point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min;
        if in_x && in_y && in_z { 1 } else { 0 }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        let x_ok = domain.dim[0].min <= search_space.dim[0].max
            && domain.dim[0].min >= search_space.dim[0].min
            && domain.dim[0].max <= search_space.dim[0].max
            && domain.dim[0].max >= search_space.dim[0].min;
        let y_ok = domain.dim[1].min <= search_space.dim[1].max
            && domain.dim[1].min >= search_space.dim[1].min
            && domain.dim[1].max <= search_space.dim[1].max
            && domain.dim[1].max >= search_space.dim[1].min;
        let z_ok = domain.dim[2].min <= search_space.dim[2].max
            && domain.dim[2].min >= search_space.dim[2].min
            && domain.dim[2].max <= search_space.dim[2].max
            && domain.dim[2].max >= search_space.dim[2].min;
        if x_ok && y_ok && z_ok { 1 } else { 0 }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        let separated = search_space.dim[0].min > domain.dim[0].max
            || search_space.dim[0].max < domain.dim[0].min
            || search_space.dim[1].min > domain.dim[1].max
            || search_space.dim[1].max < domain.dim[1].min
            || search_space.dim[2].min > domain.dim[2].max
            || search_space.dim[2].max < domain.dim[2].min;
        if separated { 0 } else { 1 }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator){
        if self.is_leaf(node) == 1 {
            let offset = node.borrow().idx;
            iter_push_global(self.points[offset].idx);
        } else {
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

    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        if self.is_leaf(node) == 1 {
            let offset = node.borrow().idx;
            let point = &self.points[offset];
            if self.point_in_search_space(point, search_space) == 1 {
                iter_push_global(point.idx);
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves(node, iter);
            } else {
                self.search_kd(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        let axis = depth % 3;
        let split = root.borrow().split;

        // Walk left branch with [domain.min, split] for this axis.
        let (left_child, right_child) = {
            let n = root.borrow();
            (n.left.clone(), n.right.clone())
        };

        let mut new_domain = clone_space(domain);
        new_domain.dim[axis].max = split;
        if let Some(left) = left_child {
            self.explore_branch(&left, depth, search_space, &new_domain, iter);
        }

        // Reset and walk right branch with [split, domain.max] for this axis.
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(right) = right_child {
            self.explore_branch(&right, depth, search_space, &new_domain, iter);
        }
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        let root = self.build_kdtree_impl(points, 0, depth);
        self.root = Some(root);
    }
}

impl KDTree {
    fn build_kdtree_impl(&mut self, points: &mut [DataPoint], base_offset: usize, depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = points.len();
        let axis = depth % 3;

        // Single point => leaf node referring to its position in self.points.
        if count == 1 {
            let node = self.next_node().expect("node allocation");
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = base_offset;
            }
            return node;
        }

        // Sort the slice along the current axis to find the median.
        match axis {
            0 => points.sort_by(compare_x),
            1 => points.sort_by(compare_y),
            _ => points.sort_by(compare_z),
        }

        // mid index relative to slice (matches C: idx_from + (idx_to - idx_from) / 2).
        let mid = (count - 1) / 2;
        let split = match axis {
            0 => points[mid].x,
            1 => points[mid].y,
            _ => points[mid].z,
        };

        let node = self.get_branch_node(split).expect("branch node allocation");

        let (left_pts, right_pts) = points.split_at_mut(mid + 1);

        let left = self.build_kdtree_impl(left_pts, base_offset, depth + 1);
        let right = self.build_kdtree_impl(right_pts, base_offset + mid + 1, depth + 1);

        {
            let mut n = node.borrow_mut();
            n.left = Some(left);
            n.right = Some(right);
        }

        node
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
    }

    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
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
        let v = self.data[self.current];
        self.current += 1;
        Some(v)
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
        let n = self.size;
        if n > 1 {
            self.data[..n].sort_by(compare_size_t);
        }
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
    if a > b { Ordering::Greater }
    else if a < b { Ordering::Less }
    else { Ordering::Equal }
}
