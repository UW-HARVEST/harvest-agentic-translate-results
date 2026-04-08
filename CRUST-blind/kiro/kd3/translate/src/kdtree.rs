use std::cell::RefCell;
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

const DIM_X: usize = 0;
const DIM_Y: usize = 1;
const DIM_Z: usize = 2;
const NDIMS: usize = 3;
const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;

fn clone_space(s: &space) -> space {
    space {
        dim: [
            Boundaries { min: s.dim[0].min, max: s.dim[0].max },
            Boundaries { min: s.dim[1].min, max: s.dim[1].max },
            Boundaries { min: s.dim[2].min, max: s.dim[2].max },
        ]
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

        if self.count != count || self.count == 0 {
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = Vec::with_capacity(self.max_nodes);
        }

        self.next_node = 0;
        self.points.clear();
        self.node_data.clear();

        for i in 0..count {
            self.points.push(DataPoint { x: x[i], y: y[i], z: z[i], idx: i });
        }

        for _ in 0..self.max_nodes {
            self.node_data.push(Rc::new(RefCell::new(TreeNode {
                left: None, right: None, split: 0.0, idx: 0,
            })));
        }

        let root = self.build_kdtree_range(0, count - 1, 0);
        self.root = Some(root);
    }
    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        self.search_space_with_iter(iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem);
    }
    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // No-op: the provided signature has no iterator parameter.
        // Actual search with results uses search_space_with_iter via search().
    }
    pub fn delete(&mut self) {
        self.root = None;
        self.points.clear();
        self.node_data.clear();
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
    }
    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        if self.next_node < self.max_nodes {
            let node = self.node_data[self.next_node].clone();
            self.next_node += 1;
            Some(node)
        } else {
            None
        }
    }
    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        {
            let mut n = node.borrow_mut();
            n.split = split;
            n.left = None;
            n.right = None;
        }
        Some(node)
    }
    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() { 1 } else { 0 }
    }
    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        if point.x <= search_space.dim[DIM_X].max &&
           point.x >= search_space.dim[DIM_X].min &&
           point.y <= search_space.dim[DIM_Y].max &&
           point.y >= search_space.dim[DIM_Y].min &&
           point.z <= search_space.dim[DIM_Z].max &&
           point.z >= search_space.dim[DIM_Z].min { 1 } else { 0 }
    }
    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        if domain.dim[DIM_X].min <= search_space.dim[DIM_X].max &&
           domain.dim[DIM_X].min >= search_space.dim[DIM_X].min &&
           domain.dim[DIM_X].max <= search_space.dim[DIM_X].max &&
           domain.dim[DIM_X].max >= search_space.dim[DIM_X].min &&
           domain.dim[DIM_Y].min <= search_space.dim[DIM_Y].max &&
           domain.dim[DIM_Y].min >= search_space.dim[DIM_Y].min &&
           domain.dim[DIM_Y].max <= search_space.dim[DIM_Y].max &&
           domain.dim[DIM_Y].max >= search_space.dim[DIM_Y].min &&
           domain.dim[DIM_Z].min <= search_space.dim[DIM_Z].max &&
           domain.dim[DIM_Z].min >= search_space.dim[DIM_Z].min &&
           domain.dim[DIM_Z].max <= search_space.dim[DIM_Z].max &&
           domain.dim[DIM_Z].max >= search_space.dim[DIM_Z].min { 1 } else { 0 }
    }
    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        if search_space.dim[DIM_X].min > domain.dim[DIM_X].max ||
           search_space.dim[DIM_X].max < domain.dim[DIM_X].min ||
           search_space.dim[DIM_Y].min > domain.dim[DIM_Y].max ||
           search_space.dim[DIM_Y].max < domain.dim[DIM_Y].min ||
           search_space.dim[DIM_Z].min > domain.dim[DIM_Z].max ||
           search_space.dim[DIM_Z].max < domain.dim[DIM_Z].min { 0 } else { 1 }
    }
    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator) {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() {
            iter.push_interior(self.points[n.idx].idx);
        } else {
            if let Some(ref left) = n.left {
                self.report_all_leaves(left, iter);
            }
            if let Some(ref right) = n.right {
                self.report_all_leaves(right, iter);
            }
        }
    }
    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let n = node.borrow();
            if self.point_in_search_space(&self.points[n.idx], search_space) == 1 {
                iter.push_interior(self.points[n.idx].idx);
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
        let axis = depth % NDIMS;
        let root_borrowed = root.borrow();
        let split = root_borrowed.split;

        // Left branch
        let mut new_domain = clone_space(domain);
        new_domain.dim[axis].max = split;
        if let Some(ref left) = root_borrowed.left {
            self.explore_branch(left, depth, search_space, &new_domain, iter);
        }

        // Right branch
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(ref right) = root_borrowed.right {
            self.explore_branch(right, depth, search_space, &new_domain, iter);
        }
    }
    fn build_kdtree(&mut self, _points: &mut [DataPoint], _depth: usize) {
        // Unused — actual build uses build_kdtree_range
    }

    fn build_kdtree_range(&mut self, idx_from: usize, idx_to: usize, depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + ((idx_to - idx_from) / 2);
        let axis = depth % NDIMS;

        if count == 1 {
            let node = self.next_node().unwrap();
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = idx_from;
            }
            return node;
        }

        let slice = &mut self.points[idx_from..=idx_to];
        match axis {
            0 => slice.sort_by(compare_x),
            1 => slice.sort_by(compare_y),
            _ => slice.sort_by(compare_z),
        }

        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        let node = self.get_branch_node(split).unwrap();
        let left = self.build_kdtree_range(idx_from, mid, depth + 1);
        let right = self.build_kdtree_range(mid + 1, idx_to, depth + 1);

        {
            let mut n = node.borrow_mut();
            n.left = Some(left);
            n.right = Some(right);
        }

        node
    }

    fn search_space_with_iter(&self, iter: &mut Option<KDTreeIterator>,
                               x_min: f64, x_max: f64,
                               y_min: f64, y_max: f64,
                               z_min: f64, z_max: f64) {
        assert!(self.root.is_some());

        match iter {
            Some(it) => it.reset(),
            None => *iter = Some(KDTreeIterator::new()),
        }

        let search_space = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ]
        };

        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ]
        };

        let root = self.root.as_ref().unwrap();
        let it = iter.as_ref().unwrap();
        self.search_kd(root, 0, &search_space, &domain, it);

        // Sync the RefCell-based results back to the iterator's public fields
        let it_mut = iter.as_mut().unwrap();
        it_mut.sync_from_interior();
    }
}
pub struct KDTreeIterator{
    pub data: Vec<usize>,
    pub capacity: usize,
    pub size: usize,
    pub current: usize,
}

/// Interior-mutability buffer used by search methods that receive `&KDTreeIterator`
thread_local! {
    static ITER_BUFFER: RefCell<Vec<usize>> = RefCell::new(Vec::new());
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
        ITER_BUFFER.with(|buf| buf.borrow_mut().clear());
    }
    pub fn push(&mut self, value: usize) {
        if self.size == self.data.len() {
            self.data.push(value);
        } else {
            self.data[self.size] = value;
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
    pub fn rewind(&mut self) {
        self.current = 0;
    }
    pub fn delete(&mut self) {
        self.data.clear();
        self.size = 0;
        self.current = 0;
        self.capacity = 0;
    }
    pub fn sort(&mut self) {
        self.data[..self.size].sort();
    }

    /// Push via thread-local buffer (called from &self context)
    fn push_interior(&self, value: usize) {
        ITER_BUFFER.with(|buf| buf.borrow_mut().push(value));
    }

    /// Sync thread-local buffer contents into the iterator's public fields
    fn sync_from_interior(&mut self) {
        ITER_BUFFER.with(|buf| {
            let b = buf.borrow();
            for &v in b.iter() {
                self.push(v);
            }
        });
        ITER_BUFFER.with(|buf| buf.borrow_mut().clear());
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
