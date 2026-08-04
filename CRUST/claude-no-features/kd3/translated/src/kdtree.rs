use std::cell::Cell;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
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
pub struct Boundaries {
    pub min: f64,
    pub max: f64,
}
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
}

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

// Thread-local pointer to the most recently used iterator option for the
// `search_space` workaround (since the Rust signature does not take iter).
thread_local! {
    static LAST_ITER: Cell<*mut Option<KDTreeIterator>> = Cell::new(std::ptr::null_mut());
    // Raw pointer to the currently active iterator during a search.  The
    // helper functions take an immutable reference to the iterator (which
    // we cannot change without modifying signatures), so they instead
    // mutate the iterator through this raw pointer.
    static ACTIVE_ITER: Cell<*mut KDTreeIterator> = Cell::new(std::ptr::null_mut());
}

fn push_active(value: usize) {
    ACTIVE_ITER.with(|cell| {
        let p = cell.get();
        debug_assert!(!p.is_null());
        // Safety: `ACTIVE_ITER` is set to point at an iterator owned by
        // the caller of `do_search_space` for the duration of the search,
        // and there are no other live references to that iterator while
        // the search runs.
        unsafe { (*p).push(value) };
    });
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
        }
    }

    pub fn build(&mut self, x: &mut [f64], y: &mut [f64], z: &mut [f64], count: usize) {
        assert!(count > 1);

        if self.count != count {
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = Vec::with_capacity(self.max_nodes);
        } else {
            self.points.clear();
            self.node_data.clear();
        }

        self.next_node = 0;
        self.root = None;

        // Cache coordinates of each point and map to the idx of the point
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // Build a working copy that will be sorted during the build.  Leaf
        // nodes will store the *original* idx of the point they represent.
        // Because `self.points` remains in its original order (i.e.
        // `self.points[k].idx == k`), the search code can look up the point
        // for a leaf node directly via `self.points[leaf.idx]`.
        let mut working: Vec<DataPoint> = self
            .points
            .iter()
            .map(|p| DataPoint {
                x: p.x,
                y: p.y,
                z: p.z,
                idx: p.idx,
            })
            .collect();

        self.build_kdtree(&mut working, 0);

        // The root is the first node added during build.
        self.root = self.node_data.first().cloned();
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

        // Record the iter location for `search_space` (which does not
        // accept an iter parameter in this Rust port).
        LAST_ITER.with(|cell| cell.set(iter as *mut Option<KDTreeIterator>));

        let x_min = x - apothem;
        let x_max = x + apothem;
        let y_min = y - apothem;
        let y_max = y + apothem;
        let z_min = z - apothem;
        let z_max = z + apothem;

        self.do_search_space(iter, x_min, x_max, y_min, y_max, z_min, z_max);
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
        // Recover the most recent iter pointer recorded by `search`.
        let iter_ptr = LAST_ITER.with(|cell| cell.get());
        if iter_ptr.is_null() {
            return;
        }
        // Safety: the test owns `iter` for the duration of the test, and
        // the pointer was recorded while `iter` is still alive.  No other
        // references to `iter` exist concurrently.
        let iter: &mut Option<KDTreeIterator> = unsafe { &mut *iter_ptr };
        self.do_search_space(iter, x_min, x_max, y_min, y_max, z_min, z_max);
    }

    fn do_search_space(
        &self,
        iter: &mut Option<KDTreeIterator>,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        // The tree should have at least one point (and not be a single leaf)
        assert!(self.root.is_some());

        // Either create a new iterator or reset the existing one
        if let Some(it) = iter.as_mut() {
            it.reset();
        } else {
            *iter = Some(KDTreeIterator::new());
        }

        let search_space = space {
            dim: [
                Boundaries {
                    min: x_min,
                    max: x_max,
                },
                Boundaries {
                    min: y_min,
                    max: y_max,
                },
                Boundaries {
                    min: z_min,
                    max: z_max,
                },
            ],
        };

        let domain = space {
            dim: [
                Boundaries {
                    min: f64::MIN,
                    max: f64::MAX,
                },
                Boundaries {
                    min: f64::MIN,
                    max: f64::MAX,
                },
                Boundaries {
                    min: f64::MIN,
                    max: f64::MAX,
                },
            ],
        };

        // Set the active iterator pointer so helper functions can push
        // results without needing a mutable reference.
        let it_ptr: *mut KDTreeIterator = iter.as_mut().unwrap() as *mut KDTreeIterator;
        ACTIVE_ITER.with(|cell| cell.set(it_ptr));

        let it_ref: &KDTreeIterator = iter.as_ref().unwrap();
        if let Some(root) = self.root.clone() {
            self.search_kd(&root, 0, &search_space, &domain, it_ref);
        }

        // Clear the active iterator pointer.
        ACTIVE_ITER.with(|cell| cell.set(std::ptr::null_mut()));
    }

    pub fn delete(&mut self) {
        self.root = None;
        self.node_data.clear();
        self.points.clear();
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
        if n.left.is_none() && n.right.is_none() {
            1
        } else {
            0
        }
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
        if (search_space.dim[0].min > domain.dim[0].max)
            || (search_space.dim[0].max < domain.dim[0].min)
            || (search_space.dim[1].min > domain.dim[1].max)
            || (search_space.dim[1].max < domain.dim[1].min)
            || (search_space.dim[2].min > domain.dim[2].max)
            || (search_space.dim[2].max < domain.dim[2].min)
        {
            0
        } else {
            1
        }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator) {
        let (is_leaf, point_idx, left, right) = {
            let n = node.borrow();
            (
                n.left.is_none() && n.right.is_none(),
                n.idx,
                n.left.clone(),
                n.right.clone(),
            )
        };
        if is_leaf {
            push_active(self.points[point_idx].idx);
        } else {
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
        iter: &KDTreeIterator,
    ) {
        if self.is_leaf(node) == 1 {
            let point_idx = node.borrow().idx;
            if self.point_in_search_space(&self.points[point_idx], search_space) == 1 {
                push_active(self.points[point_idx].idx);
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
        iter: &KDTreeIterator,
    ) {
        let axis = depth % 3;
        let (split, left, right) = {
            let r = root.borrow();
            (r.split, r.left.clone(), r.right.clone())
        };

        let mut new_domain = space {
            dim: [
                Boundaries {
                    min: domain.dim[0].min,
                    max: domain.dim[0].max,
                },
                Boundaries {
                    min: domain.dim[1].min,
                    max: domain.dim[1].max,
                },
                Boundaries {
                    min: domain.dim[2].min,
                    max: domain.dim[2].max,
                },
            ],
        };

        // explore left branch
        new_domain.dim[axis].max = split;
        if let Some(l) = &left {
            self.explore_branch(l, depth, search_space, &new_domain, iter);
        }

        // explore right branch
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(r) = &right {
            self.explore_branch(r, depth, search_space, &new_domain, iter);
        }
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        let count = points.len();

        // If there is only one point, return a leaf node.
        if count == 1 {
            let leaf = self.next_node().unwrap();
            // Store the original idx of the point this leaf represents.
            leaf.borrow_mut().idx = points[0].idx;
            return;
        }

        let axis = depth % 3;
        // Sort the points within this group to determine the median point.
        match axis {
            0 => points.sort_by(compare_x),
            1 => points.sort_by(compare_y),
            _ => points.sort_by(compare_z),
        }

        let mid = (count - 1) / 2;
        let split = match axis {
            0 => points[mid].x,
            1 => points[mid].y,
            _ => points[mid].z,
        };

        // Create the branch node BEFORE recursing so that the children
        // recorded at `node_data[left_idx]` and `node_data[right_idx]`
        // correspond to the roots of the left/right subtrees.
        let branch = self.get_branch_node(split).unwrap();

        let left_idx = self.node_data.len();
        let (left_slice, right_slice) = points.split_at_mut(mid + 1);
        self.build_kdtree(left_slice, depth + 1);

        let right_idx = self.node_data.len();
        self.build_kdtree(right_slice, depth + 1);

        let left_node = self.node_data[left_idx].clone();
        let right_node = self.node_data[right_idx].clone();
        {
            let mut b = branch.borrow_mut();
            b.left = Some(left_node);
            b.right = Some(right_node);
        }
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
            data: vec![0; KDTREE_ITERATOR_INITIAL_SIZE],
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
            // grow capacity
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
            self.data.resize(self.capacity, 0);
        }
        self.data[self.size] = value;
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
        self.capacity = 0;
        self.size = 0;
        self.current = 0;
    }

    fn sort(&mut self) {
        let s = self.size;
        self.data[..s].sort_by(compare_size_t);
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
