use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::cmp::Ordering;

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

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;

thread_local! {
    /// Stores a raw pointer to the most recently used Option<KDTreeIterator>
    /// from a `search` call. This is used by `search_space` to update the
    /// same iterator since its signature unfortunately does not take it.
    static LAST_ITER_PTR: Cell<usize> = Cell::new(0);
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
        assert!(count > 1);

        // (Re)allocate if size mismatched.
        if self.count != count {
            self.count = count;
            self.max_nodes = (count - 1) * 2 + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = Vec::with_capacity(self.max_nodes);
        } else {
            self.points.clear();
            self.node_data.clear();
            self.root = None;
        }

        self.next_node = 0;

        // Cache coordinates and original indices
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        let root = self.build_recursive(0, count - 1, 0);
        self.root = Some(root);
    }

    fn build_recursive(&mut self, idx_from: usize, idx_to: usize, depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + (idx_to - idx_from) / 2;
        let axis = depth % 3;

        // Leaf node
        if count == 1 {
            let node = Rc::new(RefCell::new(TreeNode {
                left: None,
                right: None,
                split: 0.0,
                idx: idx_from,
            }));
            self.node_data.push(node.clone());
            self.next_node += 1;
            return node;
        }

        // Sort the range by axis
        let cmp_fn: fn(&DataPoint, &DataPoint) -> Ordering = match axis {
            0 => compare_x,
            1 => compare_y,
            _ => compare_z,
        };
        self.points[idx_from..=idx_to].sort_by(cmp_fn);

        // Get split value at the median
        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        // Allocate branch node first
        let node = Rc::new(RefCell::new(TreeNode {
            left: None,
            right: None,
            split,
            idx: 0,
        }));
        self.node_data.push(node.clone());
        self.next_node += 1;

        // Recurse for left and right
        let left = self.build_recursive(idx_from, mid, depth + 1);
        let right = self.build_recursive(mid + 1, idx_to, depth + 1);

        {
            let mut n = node.borrow_mut();
            n.left = Some(left);
            n.right = Some(right);
        }

        node
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        // Save raw pointer for later search_space invocations
        LAST_ITER_PTR.with(|p| p.set(iter as *mut _ as usize));

        self.do_search(
            iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem,
        );
    }

    pub fn search_space(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        // Retrieve the iterator pointer set by a prior `search` call.
        // The iterator must remain valid (e.g. on the caller's stack).
        let ptr_val = LAST_ITER_PTR.with(|p| p.get());
        if ptr_val == 0 {
            return;
        }
        let ptr = ptr_val as *mut Option<KDTreeIterator>;
        // SAFETY: caller is responsible for ensuring the iterator referenced
        // by the previous `search` invocation is still valid.
        let iter_opt: &mut Option<KDTreeIterator> = unsafe { &mut *ptr };

        self.do_search(iter_opt, x_min, x_max, y_min, y_max, z_min, z_max);
    }

    fn do_search(
        &self,
        iter_opt: &mut Option<KDTreeIterator>,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        // Sanity checks
        assert!(self.root.is_some());
        let root = self.root.as_ref().unwrap().clone();
        assert!(self.is_leaf(&root) == 0);

        // Create or reset iterator
        if iter_opt.is_none() {
            *iter_opt = Some(KDTreeIterator::new());
        } else {
            iter_opt.as_mut().unwrap().reset();
        }

        let search_space = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ],
        };
        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ],
        };

        let iter = iter_opt.as_mut().unwrap();
        self.search_kd_internal(&root, 0, &search_space, &domain, iter);
    }

    fn search_kd_internal(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % 3;
        let (split, left, right) = {
            let n = root.borrow();
            (n.split, n.left.clone(), n.right.clone())
        };

        // Explore left branch
        let mut new_domain = clone_space(domain);
        new_domain.dim[axis].max = split;
        if let Some(l) = &left {
            self.explore_branch_internal(l, depth, search_space, &new_domain, iter);
        }

        // Explore right branch (reset max, set min)
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(r) = &right {
            self.explore_branch_internal(r, depth, search_space, &new_domain, iter);
        }
    }

    fn explore_branch_internal(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        if self.is_leaf(node) == 1 {
            let leaf_idx = node.borrow().idx;
            let point = &self.points[leaf_idx];
            if self.point_in_search_space(point, search_space) == 1 {
                iter.push(point.idx);
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves_internal(node, iter);
            } else {
                self.search_kd_internal(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn report_all_leaves_internal(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let leaf_idx = node.borrow().idx;
            iter.push(self.points[leaf_idx].idx);
        } else {
            let (left, right) = {
                let n = node.borrow();
                (n.left.clone(), n.right.clone())
            };
            if let Some(l) = &left {
                self.report_all_leaves_internal(l, iter);
            }
            if let Some(r) = &right {
                self.report_all_leaves_internal(r, iter);
            }
        }
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
        if self.next_node >= self.max_nodes {
            return None;
        }
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
        let n = self.next_node()?;
        n.borrow_mut().split = split;
        Some(n)
    }

    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
    }

    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        if point.x <= search_space.dim[0].max
            && point.x >= search_space.dim[0].min
            && point.y <= search_space.dim[1].max
            && point.y >= search_space.dim[1].min
            && point.z <= search_space.dim[2].max
            && point.z >= search_space.dim[2].min
        {
            1
        } else {
            0
        }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        for i in 0..3 {
            if !(domain.dim[i].min <= search_space.dim[i].max
                && domain.dim[i].min >= search_space.dim[i].min
                && domain.dim[i].max <= search_space.dim[i].max
                && domain.dim[i].max >= search_space.dim[i].min)
            {
                return 0;
            }
        }
        1
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        for i in 0..3 {
            if search_space.dim[i].min > domain.dim[i].max
                || search_space.dim[i].max < domain.dim[i].min
            {
                return 0;
            }
        }
        1
    }

    fn report_all_leaves(&self, _node: &Rc<RefCell<TreeNode>>, _iter: &KDTreeIterator) {
        // Note: signature takes &KDTreeIterator (not &mut), so this method
        // cannot append items. Use `report_all_leaves_internal` for the
        // working version. Kept as a no-op to satisfy the trait shape.
    }

    fn explore_branch(&self, _node: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // Note: signature takes &KDTreeIterator (not &mut). See report_all_leaves.
    }

    fn search_kd(&self, _root: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // Note: signature takes &KDTreeIterator (not &mut). See report_all_leaves.
    }

    fn build_kdtree(&mut self, _points: &mut [DataPoint], _depth: usize) {
        // Note: this method's signature does not return a node and does not
        // expose an offset into self.points, so it cannot be used to build
        // sub-trees in the same way the C `_build_kdtree` does. The actual
        // recursive builder lives in `build_recursive`, which is what
        // `build` invokes. Kept as a no-op.
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
        self.data.clear();
    }

    pub fn push(&mut self, value: usize) {
        self.data.push(value);
        self.size += 1;
        if self.data.capacity() > self.capacity {
            self.capacity = self.data.capacity();
        }
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
        self.data[..self.size].sort();
    }
}

fn compare_x(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.x > b.x {
        Ordering::Greater
    } else if a.x < b.x {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_y(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.y > b.y {
        Ordering::Greater
    } else if a.y < b.y {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_z(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.z > b.z {
        Ordering::Greater
    } else if a.z < b.z {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_size_t(a: &usize, b: &usize) -> std::cmp::Ordering {
    if a > b {
        Ordering::Greater
    } else if a < b {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}
