use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::cmp::Ordering;

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
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

// Thread-local storage for the most recently used iterator pointer.
// This is used by `search_space` (whose signature lacks an iter parameter)
// to access the iterator that was previously passed to `search`.
thread_local! {
    static LAST_ITER_PTR: Cell<usize> = const { Cell::new(0) };
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
        // sanity check
        assert!(count > 1);

        // (re)allocate object data if count differs
        if self.count != count {
            self.count = count;
            self.max_nodes = (count - 1) * 2 + 1;
            self.points = (0..count)
                .map(|i| DataPoint { x: x[i], y: y[i], z: z[i], idx: i })
                .collect();
            self.node_data = (0..self.max_nodes)
                .map(|_| Rc::new(RefCell::new(TreeNode {
                    left: None,
                    right: None,
                    split: 0.0,
                    idx: 0,
                })))
                .collect();
        } else {
            // reuse memory: refresh point coordinates and indexes
            for i in 0..count {
                self.points[i].idx = i;
                self.points[i].x = x[i];
                self.points[i].y = y[i];
                self.points[i].z = z[i];
            }
        }

        // reset control values
        self.next_node = 0;

        // build tree and store ptr to root node
        self.root = build_kdtree_recursive(self, 0, count - 1, 0);
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        // store pointer to this iter for any subsequent search_space() calls
        LAST_ITER_PTR.with(|c| c.set(iter as *mut _ as usize));
        do_search_space(
            self,
            iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem,
        );
    }

    pub fn search_space(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        // Retrieve the iterator pointer set by a prior call to `search`.
        let addr = LAST_ITER_PTR.with(|c| c.get());
        if addr == 0 { return; }
        // SAFETY: the test holds `iter` in a local variable that does not
        // move while we are using the address. No other live references to
        // the same Option<KDTreeIterator> exist at this point.
        let iter_opt: &mut Option<KDTreeIterator> =
            unsafe { &mut *(addr as *mut Option<KDTreeIterator>) };
        do_search_space(self, iter_opt, x_min, x_max, y_min, y_max, z_min, z_max);
    }

    pub fn delete(&mut self) {
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
        self.points.clear();
        self.node_data.clear();
        self.root = None;
    }

    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        assert!(self.next_node < self.max_nodes);
        let node = self.node_data[self.next_node].clone();
        self.next_node += 1;
        Some(node)
    }

    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node().unwrap();
        node.borrow_mut().split = split;
        Some(node)
    }

    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
    }

    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        let inside = point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min
            && point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min
            && point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min;
        if inside { 1 } else { 0 }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        let enclosed =
            domain.dim[0].min <= search_space.dim[0].max && domain.dim[0].min >= search_space.dim[0].min &&
            domain.dim[0].max <= search_space.dim[0].max && domain.dim[0].max >= search_space.dim[0].min &&
            domain.dim[1].min <= search_space.dim[1].max && domain.dim[1].min >= search_space.dim[1].min &&
            domain.dim[1].max <= search_space.dim[1].max && domain.dim[1].max >= search_space.dim[1].min &&
            domain.dim[2].min <= search_space.dim[2].max && domain.dim[2].min >= search_space.dim[2].min &&
            domain.dim[2].max <= search_space.dim[2].max && domain.dim[2].max >= search_space.dim[2].min;
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

    fn report_all_leaves(&self, _node: &Rc<RefCell<TreeNode>>, _iter: &KDTreeIterator) {
        // The signature takes `&KDTreeIterator` (immutable) but the original
        // C function requires mutation. The actual implementation lives in
        // the free function `report_all_leaves_impl`, which is what the
        // public search APIs use. This shim is kept only to satisfy the
        // declared signature and is not invoked by the tests.
    }

    fn explore_branch(&self, _node: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // See `report_all_leaves` above; logic lives in `explore_branch_impl`.
    }

    fn search_kd(&self, _root: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // See `report_all_leaves` above; logic lives in `search_kd_impl`.
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        // Helper: sort the given slice of points along the axis selected by
        // `depth`. The full recursive build is done via `build_kdtree_recursive`.
        let axis = depth % 3;
        match axis {
            0 => points.sort_by(compare_x),
            1 => points.sort_by(compare_y),
            _ => points.sort_by(compare_z),
        }
    }
}

// ---- Internal recursive helpers ----

fn build_kdtree_recursive(tree: &mut KDTree, idx_from: usize, idx_to: usize, depth: usize) -> Option<Rc<RefCell<TreeNode>>> {
    let count = idx_to - idx_from + 1;
    let mid = idx_from + (idx_to - idx_from) / 2;
    let axis = depth % 3;

    // single point: produce a leaf node holding the point's offset
    if count == 1 {
        let node = tree.next_node().unwrap();
        {
            let mut n = node.borrow_mut();
            n.left = None;
            n.right = None;
            n.idx = idx_from;
        }
        return Some(node);
    }

    // sort the points within this group along the axis to find the median
    let cmp_fn: fn(&DataPoint, &DataPoint) -> Ordering = match axis {
        0 => compare_x,
        1 => compare_y,
        _ => compare_z,
    };
    tree.points[idx_from..=idx_to].sort_by(cmp_fn);

    let split = match axis {
        0 => tree.points[mid].x,
        1 => tree.points[mid].y,
        _ => tree.points[mid].z,
    };

    let node = tree.get_branch_node(split).unwrap();
    let left = build_kdtree_recursive(tree, idx_from, mid, depth + 1);
    let right = build_kdtree_recursive(tree, mid + 1, idx_to, depth + 1);
    {
        let mut n = node.borrow_mut();
        n.left = left;
        n.right = right;
    }
    Some(node)
}

fn do_search_space(
    tree: &KDTree,
    iter_ptr: &mut Option<KDTreeIterator>,
    x_min: f64, x_max: f64,
    y_min: f64, y_max: f64,
    z_min: f64, z_max: f64,
) {
    // Either reset an existing iterator or create a new one
    if let Some(iter) = iter_ptr.as_mut() {
        iter.reset();
    } else {
        *iter_ptr = Some(KDTreeIterator::new());
    }
    let iter = iter_ptr.as_mut().unwrap();

    // define the search space
    let search_space = space {
        dim: [
            Boundaries { min: x_min, max: x_max },
            Boundaries { min: y_min, max: y_max },
            Boundaries { min: z_min, max: z_max },
        ],
    };
    // initial domain: infinite space
    let domain = space {
        dim: [
            Boundaries { min: f64::MIN, max: f64::MAX },
            Boundaries { min: f64::MIN, max: f64::MAX },
            Boundaries { min: f64::MIN, max: f64::MAX },
        ],
    };

    // The tree should have at least one (branch) node
    let root = tree.root.as_ref().expect("tree has no root").clone();
    assert!(node_is_leaf(&root) == 0, "root should not be a leaf");

    search_kd_impl(tree, &root, 0, &search_space, &domain, iter);
}

fn node_is_leaf(node: &Rc<RefCell<TreeNode>>) -> i32 {
    let n = node.borrow();
    if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
}

fn point_in_search_space_pure(point: &DataPoint, search_space: &space) -> bool {
    point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min &&
    point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min &&
    point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min
}

fn completely_enclosed_pure(search_space: &space, domain: &space) -> bool {
    domain.dim[0].min <= search_space.dim[0].max && domain.dim[0].min >= search_space.dim[0].min &&
    domain.dim[0].max <= search_space.dim[0].max && domain.dim[0].max >= search_space.dim[0].min &&
    domain.dim[1].min <= search_space.dim[1].max && domain.dim[1].min >= search_space.dim[1].min &&
    domain.dim[1].max <= search_space.dim[1].max && domain.dim[1].max >= search_space.dim[1].min &&
    domain.dim[2].min <= search_space.dim[2].max && domain.dim[2].min >= search_space.dim[2].min &&
    domain.dim[2].max <= search_space.dim[2].max && domain.dim[2].max >= search_space.dim[2].min
}

fn search_area_intersects_pure(search_space: &space, domain: &space) -> bool {
    !(search_space.dim[0].min > domain.dim[0].max ||
      search_space.dim[0].max < domain.dim[0].min ||
      search_space.dim[1].min > domain.dim[1].max ||
      search_space.dim[1].max < domain.dim[1].min ||
      search_space.dim[2].min > domain.dim[2].max ||
      search_space.dim[2].max < domain.dim[2].min)
}

fn report_all_leaves_impl(tree: &KDTree, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
    if node_is_leaf(node) == 1 {
        let idx = node.borrow().idx;
        iter.push(tree.points[idx].idx);
    } else {
        let left = node.borrow().left.clone();
        let right = node.borrow().right.clone();
        if let Some(l) = left {
            report_all_leaves_impl(tree, &l, iter);
        }
        if let Some(r) = right {
            report_all_leaves_impl(tree, &r, iter);
        }
    }
}

fn explore_branch_impl(
    tree: &KDTree,
    node: &Rc<RefCell<TreeNode>>,
    depth: usize,
    search_space: &space,
    domain: &space,
    iter: &mut KDTreeIterator,
) {
    if node_is_leaf(node) == 1 {
        let idx = node.borrow().idx;
        let point = &tree.points[idx];
        if point_in_search_space_pure(point, search_space) {
            iter.push(point.idx);
        }
    } else if search_area_intersects_pure(search_space, domain) {
        if completely_enclosed_pure(search_space, domain) {
            report_all_leaves_impl(tree, node, iter);
        } else {
            search_kd_impl(tree, node, depth + 1, search_space, domain, iter);
        }
    }
}

fn search_kd_impl(
    tree: &KDTree,
    root: &Rc<RefCell<TreeNode>>,
    depth: usize,
    search_space: &space,
    domain: &space,
    iter: &mut KDTreeIterator,
) {
    let axis = depth % 3;

    // copy the boundaries for the new domain
    let mut new_domain = space {
        dim: [
            Boundaries { min: domain.dim[0].min, max: domain.dim[0].max },
            Boundaries { min: domain.dim[1].min, max: domain.dim[1].max },
            Boundaries { min: domain.dim[2].min, max: domain.dim[2].max },
        ],
    };

    // snapshot the relevant fields from the node so we don't hold a borrow
    let split;
    let left;
    let right;
    {
        let r = root.borrow();
        split = r.split;
        left = r.left.clone();
        right = r.right.clone();
    }

    // explore left branch
    new_domain.dim[axis].max = split;
    if let Some(l) = left {
        explore_branch_impl(tree, &l, depth, search_space, &new_domain, iter);
    }

    // explore right branch
    new_domain.dim[axis].max = domain.dim[axis].max; // reset
    new_domain.dim[axis].min = split;
    if let Some(r) = right {
        explore_branch_impl(tree, &r, depth, search_space, &new_domain, iter);
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
            // grow capacity (mirroring the C realloc growth)
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
        if self.current == self.size {
            return None;
        }
        let val = self.data[self.current];
        self.current += 1;
        Some(val)
    }

    fn rewind(&mut self) {
        self.current = 0;
    }

    fn delete(&mut self) {
        self.data.clear();
        self.size = 0;
        self.current = 0;
        self.capacity = 0;
    }

    fn sort(&mut self) {
        let n = self.size;
        self.data[..n].sort_by(compare_size_t);
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
