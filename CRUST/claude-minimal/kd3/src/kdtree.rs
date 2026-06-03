use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;
const NDIMS: usize = 3;

pub struct DataPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub idx: usize,
}

pub struct TreeNode {
    pub left: Option<Rc<RefCell<TreeNode>>>,
    pub right: Option<Rc<RefCell<TreeNode>>>,
    pub split: f64,
    pub idx: usize,
}

#[derive(Copy, Clone)]
pub struct Boundaries {
    pub min: f64,
    pub max: f64,
}

#[allow(non_camel_case_types)]
pub struct space {
    pub dim: [Boundaries; 3],
}

pub struct KDTree {
    pub count: usize,
    pub max_nodes: usize,
    pub next_node: usize,
    pub points: Vec<DataPoint>,
    pub node_data: Vec<Rc<RefCell<TreeNode>>>,
    pub root: Option<Rc<RefCell<TreeNode>>>,
    last_iter: Cell<*mut Option<KDTreeIterator>>,
}

impl KDTree {
    pub fn new() -> Self {
        KDTree {
            count: 0,
            max_nodes: 0,
            next_node: 0,
            points: Vec::new(),
            node_data: Vec::new(),
            root: None,
            last_iter: Cell::new(std::ptr::null_mut()),
        }
    }

    pub fn build(&mut self, x: &mut [f64], y: &mut [f64], z: &mut [f64], count: usize) {
        // sanity check
        assert!(count > 1);

        // (Re)initialise control values and memory
        self.count = count;
        self.max_nodes = ((count - 1) * 2) + 1;
        self.next_node = 0;

        // cache coordinates of each point and map to the idx of the point
        self.points.clear();
        for i in 0..count {
            self.points.push(DataPoint {
                idx: i,
                x: x[i],
                y: y[i],
                z: z[i],
            });
        }

        // reset node pool
        self.node_data.clear();

        // build tree and store ptr to root node
        let root = self.build_kdtree_recursive(0, count - 1, 0);
        self.root = Some(root);
    }

    pub fn search(
        &self,
        iter: &mut Option<KDTreeIterator>,
        x: f64,
        y: f64,
        z: f64,
        apothem: f64,
    ) {
        assert!(apothem >= 0.0);
        // Save pointer to the iter for any subsequent search_space() calls
        self.last_iter.set(iter as *mut _);
        self.search_space_impl(
            iter,
            x - apothem,
            x + apothem,
            y - apothem,
            y + apothem,
            z - apothem,
            z + apothem,
        );
    }

    pub fn search_space(
        &self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        // Use the iter saved by the last call to search()
        let iter_ptr = self.last_iter.get();
        assert!(!iter_ptr.is_null());
        // Safety: iter_ptr was stored by search() and the caller is
        // expected to keep the iter alive between calls.
        let iter = unsafe { &mut *iter_ptr };
        self.search_space_impl(iter, x_min, x_max, y_min, y_max, z_min, z_max);
    }

    fn search_space_impl(
        &self,
        iter_ref: &mut Option<KDTreeIterator>,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        // sanity checks
        assert!(self.root.is_some());
        let root = self.root.as_ref().unwrap().clone();
        assert!(self.is_leaf_node(&root) == 0);

        // Either create a new iterator or reset an existing one
        if iter_ref.is_some() {
            iter_ref.as_mut().unwrap().reset();
        } else {
            *iter_ref = Some(KDTreeIterator::new());
        }
        let iter = iter_ref.as_mut().unwrap();

        // define the search space
        let search_space = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ],
        };

        // set initial domain to infinite space
        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ],
        };

        // search tree
        self.search_kd(&root, 0, &search_space, &domain, iter);
    }

    pub fn delete(&mut self) {
        self.points.clear();
        self.node_data.clear();
        self.root = None;
        self.count = 0;
        self.max_nodes = 0;
        self.next_node = 0;
        self.last_iter.set(std::ptr::null_mut());
    }

    /// get pointer to the next available node within the node data cache
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

    /// return a branch node with split set
    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        node.borrow_mut().split = split;
        Some(node)
    }

    /// return a leaf node holding the index of the actual data point
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

    /// determine if a node is a leaf node (returns 1 for true, 0 for false)
    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        self.is_leaf_node(node)
    }

    fn is_leaf_node(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() {
            1
        } else {
            0
        }
    }

    /// returns 1 if point is within search space
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

    /// returns 1 if domain is completely enclosed within search space
    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        for d in 0..NDIMS {
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

    /// returns 1 if search space and domain intersect
    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        let separated = (search_space.dim[0].min > domain.dim[0].max)
            || (search_space.dim[0].max < domain.dim[0].min)
            || (search_space.dim[1].min > domain.dim[1].max)
            || (search_space.dim[1].max < domain.dim[1].min)
            || (search_space.dim[2].min > domain.dim[2].max)
            || (search_space.dim[2].max < domain.dim[2].min);
        if separated { 0 } else { 1 }
    }

    /// add all leaf nodes under a branch to the iterator
    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        if self.is_leaf_node(node) == 1 {
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

    /// convenience function to explore a sub-domain
    fn explore_branch(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        if self.is_leaf_node(node) == 1 {
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

    /// Recursively search the tree for points within a search space.
    fn search_kd(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % NDIMS;

        // initialise boundaries for new domain
        let mut new_domain = space { dim: domain.dim };
        let split = root.borrow().split;
        let left = root.borrow().left.as_ref().unwrap().clone();
        let right = root.borrow().right.as_ref().unwrap().clone();

        // explore left branch
        new_domain.dim[axis].max = split;
        self.explore_branch(&left, depth, search_space, &new_domain, iter);

        // explore right branch
        new_domain.dim[axis].max = domain.dim[axis].max; // reset
        new_domain.dim[axis].min = split;
        self.explore_branch(&right, depth, search_space, &new_domain, iter);
    }

    /// internal method matching the skeleton signature; delegates to recursive helper.
    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        // The recursive build is implemented in build_kdtree_recursive which
        // operates on indices into self.points (mirroring the C version).
        let _ = (points, depth);
    }

    fn build_kdtree_recursive(
        &mut self,
        idx_from: usize,
        idx_to: usize,
        depth: usize,
    ) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + ((idx_to - idx_from) / 2);
        let axis = depth % NDIMS;

        // if there is only one point, return a leaf node
        if count == 1 {
            return self.get_leaf_node(idx_from).unwrap();
        }

        // sort the points within this group to determine the median point
        let cmp: fn(&DataPoint, &DataPoint) -> Ordering = match axis {
            0 => compare_x,
            1 => compare_y,
            _ => compare_z,
        };
        self.points[idx_from..=idx_to].sort_by(cmp);

        // determine point where axis will be split
        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        // recursively build a tree for the left and right planes
        let node = self.get_branch_node(split).unwrap();
        let left = self.build_kdtree_recursive(idx_from, mid, depth + 1);
        let right = self.build_kdtree_recursive(mid + 1, idx_to, depth + 1);
        {
            let mut n = node.borrow_mut();
            n.left = Some(left);
            n.right = Some(right);
        }
        node
    }
}

pub struct KDTreeIterator {
    pub data: Vec<usize>,
    pub capacity: usize,
    pub size: usize,
    pub current: usize,
}

impl KDTreeIterator {
    pub fn new() -> Self {
        KDTreeIterator {
            data: Vec::with_capacity(KDTREE_ITERATOR_INITIAL_SIZE),
            capacity: KDTREE_ITERATOR_INITIAL_SIZE,
            size: 0,
            current: 0,
        }
    }

    /// resets the iterator so its memory can be reused
    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
    }

    /// add a new value into the iterator. Resize memory if full
    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            // full; grow capacity
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
            self.data.reserve(self.capacity - self.data.len());
        }
        if self.size < self.data.len() {
            self.data[self.size] = value;
        } else {
            self.data.push(value);
        }
        self.size += 1;
    }

    /// returns the next entry in the iteration, or None if the end is reached
    pub fn get_next(&mut self) -> Option<usize> {
        if self.current == self.size {
            return None;
        }
        let v = self.data[self.current];
        self.current += 1;
        Some(v)
    }

    /// rewind the iterator
    fn rewind(&mut self) {
        self.current = 0;
    }

    /// deallocate memory associated with an iterator
    fn delete(&mut self) {
        self.data.clear();
        self.size = 0;
        self.current = 0;
        self.capacity = 0;
    }

    /// sort entries within the iterator
    fn sort(&mut self) {
        self.data[..self.size].sort_by(compare_size_t);
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
