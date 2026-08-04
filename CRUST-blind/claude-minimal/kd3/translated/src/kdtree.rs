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

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

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

        // Reallocate object if count does not match (or this is the first build)
        if self.count != count {
            // delete previous storage
            self.delete();
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = Vec::with_capacity(self.max_nodes);
        } else {
            // reuse capacity, just clear contents
            self.points.clear();
            self.node_data.clear();
            self.root = None;
        }

        // reset control values
        self.next_node = 0;

        // cache coordinates of each point and map to the idx of the point
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // build tree and store ptr to root node
        self.root = self.build_kdtree(0, count - 1, 0);
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);

        // sanity checks: tree must have at least one point and root must not be a leaf
        let root_clone = self.root.as_ref().expect("tree is empty").clone();
        assert!(self.is_leaf(&root_clone) == 0);

        // Either create a new iterator or reset an existing one
        if iter.is_some() {
            iter.as_mut().unwrap().reset();
        } else {
            *iter = Some(KDTreeIterator::new());
        }
        let it = iter.as_mut().unwrap();

        // define the search space (cube around (x,y,z) with given apothem)
        let search_space = space {
            dim: [
                Boundaries { min: x - apothem, max: x + apothem },
                Boundaries { min: y - apothem, max: y + apothem },
                Boundaries { min: z - apothem, max: z + apothem },
            ],
        };

        // set initial domain to (effectively) infinite space
        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ],
        };

        // search tree
        self.search_kd(&root_clone, 0, &search_space, &domain, it);
    }

    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // The provided signature has no iterator parameter, so there is nowhere to
        // record the results. The full search-space implementation lives inside
        // `search` (which mirrors the C kdtree_search_space behaviour).
    }

    pub fn delete(&mut self) {
        // drop root first so the tree's internal Rc graph is dismantled
        self.root = None;
        self.node_data.clear();
        self.points.clear();
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
    }

    /// Allocate the next available tree node from the contiguous pool.
    fn next_node(&mut self) -> Option<Rc<RefCell<TreeNode>>> {
        assert!(self.next_node < self.max_nodes);
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

    /// Return a branch node with the supplied split value.
    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        node.borrow_mut().split = split;
        Some(node)
    }

    /// Return a leaf node holding the index of the actual data point.
    fn get_leaf_node(&mut self, offset: usize) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        {
            let mut n = node.borrow_mut();
            n.left = None;
            n.right = None;
            n.idx = offset;
        }
        Some(node)
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
        if domain.dim[0].min <= search_space.dim[0].max
            && domain.dim[0].min >= search_space.dim[0].min
            && domain.dim[0].max <= search_space.dim[0].max
            && domain.dim[0].max >= search_space.dim[0].min
            && domain.dim[1].min <= search_space.dim[1].max
            && domain.dim[1].min >= search_space.dim[1].min
            && domain.dim[1].max <= search_space.dim[1].max
            && domain.dim[1].max >= search_space.dim[1].min
            && domain.dim[2].min <= search_space.dim[2].max
            && domain.dim[2].min >= search_space.dim[2].min
            && domain.dim[2].max <= search_space.dim[2].max
            && domain.dim[2].max >= search_space.dim[2].min
        {
            1
        } else {
            0
        }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        // It's easier to determine whether the cubes are completely separate;
        // negate that result.
        let separate = search_space.dim[0].min > domain.dim[0].max
            || search_space.dim[0].max < domain.dim[0].min
            || search_space.dim[1].min > domain.dim[1].max
            || search_space.dim[1].max < domain.dim[1].min
            || search_space.dim[2].min > domain.dim[2].max
            || search_space.dim[2].max < domain.dim[2].min;
        if separate { 0 } else { 1 }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let idx = node.borrow().idx;
            iter.push(self.points[idx].idx);
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

    fn explore_branch(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        if self.is_leaf(node) == 1 {
            let idx = node.borrow().idx;
            if self.point_in_search_space(&self.points[idx], search_space) == 1 {
                iter.push(self.points[idx].idx);
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves(node, iter);
            } else {
                self.search_kd(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn search_kd(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % 3;
        let split = root.borrow().split;
        let orig_max = domain.dim[axis].max;

        // make a working copy of the current domain (memcpy in the C version)
        let mut new_domain = space {
            dim: [
                Boundaries { min: domain.dim[0].min, max: domain.dim[0].max },
                Boundaries { min: domain.dim[1].min, max: domain.dim[1].max },
                Boundaries { min: domain.dim[2].min, max: domain.dim[2].max },
            ],
        };

        // explore left branch
        new_domain.dim[axis].max = split;
        let left = root.borrow().left.clone();
        if let Some(l) = left {
            self.explore_branch(&l, depth, search_space, &new_domain, iter);
        }

        // explore right branch
        new_domain.dim[axis].max = orig_max;
        new_domain.dim[axis].min = split;
        let right = root.borrow().right.clone();
        if let Some(r) = right {
            self.explore_branch(&r, depth, search_space, &new_domain, iter);
        }
    }

    /// Recursive routine to build a k-d tree over the points in
    /// self.points[idx_from..=idx_to]. Mirrors `_build_kdtree` in the C source.
    fn build_kdtree(
        &mut self,
        idx_from: usize,
        idx_to: usize,
        depth: usize,
    ) -> Option<Rc<RefCell<TreeNode>>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + ((idx_to - idx_from) / 2);
        let axis = depth % 3;

        // single point -> leaf node
        if count == 1 {
            return self.get_leaf_node(idx_from);
        }

        // sort the points within this group along the current axis to find the median
        {
            let slice = &mut self.points[idx_from..=idx_to];
            match axis {
                0 => slice.sort_by(compare_x),
                1 => slice.sort_by(compare_y),
                _ => slice.sort_by(compare_z),
            }
        }

        // determine point where axis will be split
        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        // recursively build a tree for the left and right planes
        let node = self.get_branch_node(split)?;
        let left = self.build_kdtree(idx_from, mid, depth + 1);
        let right = self.build_kdtree(mid + 1, idx_to, depth + 1);
        {
            let mut n = node.borrow_mut();
            n.left = left;
            n.right = right;
        }
        Some(node)
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

    /// Reset the iterator so its memory can be reused (matches `_iterator_reset`).
    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
        self.data.clear();
    }

    /// Add a new value into the iterator. Resize if full (matches `_iterator_push`).
    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            assert!(KDTREE_ITERATOR_GROWTH_RATIO > 1);
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
        }
        self.data.push(value);
        self.size += 1;
    }

    /// Returns the next entry in the iteration, or `None` at the end
    /// (matches `kdtree_iterator_get_next` returning KDTREE_END).
    pub fn get_next(&mut self) -> Option<usize> {
        if self.current == self.size {
            return None;
        }
        let v = self.data[self.current];
        self.current += 1;
        Some(v)
    }

    /// Rewind the iterator (matches `kdtree_iterator_rewind`).
    fn rewind(&mut self) {
        self.current = 0;
    }

    /// Free memory associated with the iterator (matches `kdtree_iterator_delete`).
    fn delete(&mut self) {
        self.data.clear();
        self.data.shrink_to_fit();
        self.size = 0;
        self.current = 0;
        self.capacity = 0;
    }

    /// Sort entries within the iterator (matches `kdtree_iterator_sort`).
    fn sort(&mut self) {
        let size = self.size;
        self.data[..size].sort_by(compare_size_t);
    }
}

fn compare_x(a: &DataPoint, b: &DataPoint) -> Ordering {
    if a.x > b.x {
        Ordering::Greater
    } else if a.x < b.x {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_y(a: &DataPoint, b: &DataPoint) -> Ordering {
    if a.y > b.y {
        Ordering::Greater
    } else if a.y < b.y {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_z(a: &DataPoint, b: &DataPoint) -> Ordering {
    if a.z > b.z {
        Ordering::Greater
    } else if a.z < b.z {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn compare_size_t(a: &usize, b: &usize) -> Ordering {
    if a > b {
        Ordering::Greater
    } else if a < b {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}
