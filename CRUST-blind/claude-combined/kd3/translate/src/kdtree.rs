use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::Ordering;

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;
const NDIMS: usize = 3;

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
        // sanity check
        assert!(count > 1);

        // Reallocate if count differs
        if self.count != count {
            self.delete();
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            for _ in 0..count {
                self.points.push(DataPoint { x: 0.0, y: 0.0, z: 0.0, idx: 0 });
            }
            self.node_data = Vec::with_capacity(self.max_nodes);
        }

        // reset control values
        self.next_node = 0;
        self.node_data.clear();

        // populate points
        for i in 0..count {
            self.points[i].idx = i;
            self.points[i].x = x[i];
            self.points[i].y = y[i];
            self.points[i].z = z[i];
        }

        // build tree
        self.root = self.build_kdtree_recursive(0, count - 1, 0);
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        self.do_search_space(
            iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem,
        );
    }

    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // The provided signature doesn't include an iterator parameter, so this
        // function cannot meaningfully report results. The actual search-space
        // implementation is exposed via `search` (which calls into the
        // private `do_search_space`).
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
        if (point.x <= search_space.dim[0].max) &&
           (point.x >= search_space.dim[0].min) &&
           (point.y <= search_space.dim[1].max) &&
           (point.y >= search_space.dim[1].min) &&
           (point.z <= search_space.dim[2].max) &&
           (point.z >= search_space.dim[2].min) {
            1
        } else {
            0
        }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        if (domain.dim[0].min <= search_space.dim[0].max) &&
           (domain.dim[0].min >= search_space.dim[0].min) &&
           (domain.dim[0].max <= search_space.dim[0].max) &&
           (domain.dim[0].max >= search_space.dim[0].min) &&
           (domain.dim[1].min <= search_space.dim[1].max) &&
           (domain.dim[1].min >= search_space.dim[1].min) &&
           (domain.dim[1].max <= search_space.dim[1].max) &&
           (domain.dim[1].max >= search_space.dim[1].min) &&
           (domain.dim[2].min <= search_space.dim[2].max) &&
           (domain.dim[2].min >= search_space.dim[2].min) &&
           (domain.dim[2].max <= search_space.dim[2].max) &&
           (domain.dim[2].max >= search_space.dim[2].min) {
            1
        } else {
            0
        }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        let separated = (search_space.dim[0].min > domain.dim[0].max) ||
                        (search_space.dim[0].max < domain.dim[0].min) ||
                        (search_space.dim[1].min > domain.dim[1].max) ||
                        (search_space.dim[1].max < domain.dim[1].min) ||
                        (search_space.dim[2].min > domain.dim[2].max) ||
                        (search_space.dim[2].max < domain.dim[2].min);
        if separated { 0 } else { 1 }
    }

    fn report_all_leaves(&self, _node: &Rc<RefCell<TreeNode>>, _iter: &KDTreeIterator) {
        // The signature requires an immutable iterator reference, which
        // cannot be used to write results. The actual implementation is
        // provided by `report_all_leaves_mut` below and used by the search
        // routines.
    }

    fn explore_branch(&self, _node: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // See note on `report_all_leaves`. Real logic lives in
        // `explore_branch_mut`.
    }

    fn search_kd(&self, _root: &Rc<RefCell<TreeNode>>, _depth: usize, _search_space: &space, _domain: &space, _iter: &KDTreeIterator) {
        // See note on `report_all_leaves`. Real logic lives in
        // `search_kd_mut`.
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        // The provided signature operates on a slice that cannot represent a
        // sub-range of `self.points` while also borrowing `self` mutably to
        // populate `node_data`. The actual recursive build is performed in
        // `build_kdtree_recursive` and is invoked from `build`.
        let _ = (points, depth);
    }
}

impl KDTree {
    fn build_kdtree_recursive(
        &mut self,
        idx_from: usize,
        idx_to: usize,
        depth: usize,
    ) -> Option<Rc<RefCell<TreeNode>>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + (idx_to - idx_from) / 2;
        let axis = depth % NDIMS;

        if count == 1 {
            let node = self.next_node()?;
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = idx_from;
            }
            return Some(node);
        }

        let cmp: fn(&DataPoint, &DataPoint) -> Ordering = match axis {
            0 => compare_x,
            1 => compare_y,
            _ => compare_z,
        };
        self.points[idx_from..=idx_to].sort_by(cmp);

        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        let node = self.get_branch_node(split)?;
        let left = self.build_kdtree_recursive(idx_from, mid, depth + 1);
        let right = self.build_kdtree_recursive(mid + 1, idx_to, depth + 1);
        {
            let mut n = node.borrow_mut();
            n.left = left;
            n.right = right;
        }
        Some(node)
    }

    fn do_search_space(
        &self,
        iter: &mut Option<KDTreeIterator>,
        x_min: f64, x_max: f64,
        y_min: f64, y_max: f64,
        z_min: f64, z_max: f64,
    ) {
        // sanity checks
        let root = match &self.root {
            Some(r) => r,
            None => panic!("kdtree must contain at least one point"),
        };
        assert!(self.is_leaf(root) == 0, "tree root must not be a leaf");

        // Either create a new iterator or reset existing one
        match iter.as_mut() {
            Some(it) => it.reset(),
            None => {
                *iter = Some(KDTreeIterator::new());
            }
        }
        let it = iter.as_mut().expect("iterator just initialized");

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

        self.search_kd_mut(root, 0, &search_space, &domain, it);
    }

    fn report_all_leaves_mut(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        let n = node.borrow();
        if n.left.is_none() && n.right.is_none() {
            iter.push(self.points[n.idx].idx);
        } else {
            if let Some(l) = &n.left {
                self.report_all_leaves_mut(l, iter);
            }
            if let Some(r) = &n.right {
                self.report_all_leaves_mut(r, iter);
            }
        }
    }

    fn explore_branch_mut(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let is_leaf = {
            let n = node.borrow();
            n.left.is_none() && n.right.is_none()
        };
        if is_leaf {
            let idx_in_points = node.borrow().idx;
            let p = &self.points[idx_in_points];
            if self.point_in_search_space(p, search_space) == 1 {
                iter.push(p.idx);
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves_mut(node, iter);
            } else {
                self.search_kd_mut(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn search_kd_mut(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % NDIMS;
        let split = root.borrow().split;
        let domain_min = domain.dim[axis].min;
        let domain_max = domain.dim[axis].max;

        let mut new_domain = space {
            dim: [
                Boundaries { min: domain.dim[0].min, max: domain.dim[0].max },
                Boundaries { min: domain.dim[1].min, max: domain.dim[1].max },
                Boundaries { min: domain.dim[2].min, max: domain.dim[2].max },
            ],
        };

        // explore left branch: domain becomes (..split]
        new_domain.dim[axis].max = split;
        new_domain.dim[axis].min = domain_min;
        let left = root.borrow().left.clone();
        if let Some(l) = left {
            self.explore_branch_mut(&l, depth, search_space, &new_domain, iter);
        }

        // explore right branch: domain becomes [split..)
        new_domain.dim[axis].max = domain_max;
        new_domain.dim[axis].min = split;
        let right = root.borrow().right.clone();
        if let Some(r) = right {
            self.explore_branch_mut(&r, depth, search_space, &new_domain, iter);
        }
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
            assert!(KDTREE_ITERATOR_GROWTH_RATIO > 1);
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

// Public free-function wrappers for the iterator methods that the C API
// exposes as standalone functions. These are intentionally not public to
// avoid changing the module's exports beyond what the impl block defines.
impl KDTreeIterator {
    pub fn rewind_pub(&mut self) {
        self.rewind();
    }
    pub fn delete_pub(&mut self) {
        self.delete();
    }
    pub fn sort_pub(&mut self) {
        self.sort();
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
