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

fn make_space_clone(s: &space) -> space {
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
        // sanity check (mirrors C assert)
        assert!(count > 1);

        // Reallocate object if count does not match the previous build.
        if self.count != count {
            self.delete();
            self.count = count;
            self.max_nodes = (count - 1) * 2 + 1;
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
        }

        // reset control values
        self.next_node = 0;

        // Cache coordinates of each point and map to their original index
        self.points.clear();
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // Detach any leftover left/right links from a previous build to avoid
        // stale references confusing iteration.
        for n in &self.node_data {
            let mut nn = n.borrow_mut();
            nn.left = None;
            nn.right = None;
            nn.split = 0.0;
            nn.idx = 0;
        }

        // build tree and store ptr to root node
        let root = self.build_recursive(0, count - 1, 0);
        self.root = Some(root);
    }

    fn build_recursive(&mut self, idx_from: usize, idx_to: usize, depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + (idx_to - idx_from) / 2;
        let axis = depth % 3;

        // if there is only one point, return a leaf node
        if count == 1 {
            let node = self.next_node().expect("ran out of nodes");
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = idx_from;
            }
            return node;
        }

        // sort the points within this group to determine the median point
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
        let node = self.get_branch_node(split).expect("ran out of nodes");
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
        self.search_box(
            iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem,
        );
    }

    fn search_box(
        &self,
        iter: &mut Option<KDTreeIterator>,
        x_min: f64, x_max: f64,
        y_min: f64, y_max: f64,
        z_min: f64, z_max: f64,
    ) {
        // Either create a new iterator or reset an existing one
        if iter.is_some() {
            iter.as_mut().unwrap().reset();
        } else {
            *iter = Some(KDTreeIterator::new());
        }

        // define the search space
        let search_space = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ],
        };

        // initial domain is infinite space
        let domain = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ],
        };

        if let Some(root) = self.root.clone() {
            let iter_mut = iter.as_mut().unwrap();
            search_kd_impl(self, &root, 0, &search_space, &domain, iter_mut);
        }
    }

    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // The supplied signature has no iterator output; this is a no-op
        // placeholder. Use `search` for actual queries.
    }

    pub fn delete(&mut self) {
        // Detach internal Rc links so dropping does not have to recurse deeply.
        for n in &self.node_data {
            let mut nn = n.borrow_mut();
            nn.left = None;
            nn.right = None;
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
        let node = self.node_data[self.next_node].clone();
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
        let inside = point.x <= search_space.dim[0].max
            && point.x >= search_space.dim[0].min
            && point.y <= search_space.dim[1].max
            && point.y >= search_space.dim[1].min
            && point.z <= search_space.dim[2].max
            && point.z >= search_space.dim[2].min;
        if inside { 1 } else { 0 }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        let mut enclosed = true;
        for i in 0..3 {
            if !(domain.dim[i].min <= search_space.dim[i].max
                && domain.dim[i].min >= search_space.dim[i].min
                && domain.dim[i].max <= search_space.dim[i].max
                && domain.dim[i].max >= search_space.dim[i].min)
            {
                enclosed = false;
                break;
            }
        }
        if enclosed { 1 } else { 0 }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        let mut separate = false;
        for i in 0..3 {
            if search_space.dim[i].min > domain.dim[i].max
                || search_space.dim[i].max < domain.dim[i].min
            {
                separate = true;
                break;
            }
        }
        if !separate { 1 } else { 0 }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, _iter: &KDTreeIterator) {
        // The supplied signature gives only an immutable borrow of the
        // iterator, so we cannot push to it here. We still walk the tree to
        // mirror the recursion structure of the C implementation. Actual
        // mutating work is done by the `report_all_leaves_impl` free fn.
        let (left, right, _idx) = {
            let n = node.borrow();
            (n.left.clone(), n.right.clone(), n.idx)
        };
        if let Some(l) = &left {
            self.report_all_leaves(l, _iter);
        }
        if let Some(r) = &right {
            self.report_all_leaves(r, _iter);
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
        // As with `report_all_leaves`, the signature precludes mutating the
        // iterator. We replicate the traversal decisions of the C code so the
        // method remains a faithful read-only sketch of the algorithm.
        if self.is_leaf(node) == 1 {
            let _ = self.point_in_search_space(&self.points[node.borrow().idx], search_space);
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
            let n = root.borrow();
            (n.split, n.left.clone(), n.right.clone())
        };

        let mut new_domain = make_space_clone(domain);

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
        // The actual recursive build is implemented in `build_recursive`
        // which can use `&mut self` to access the contiguous node storage.
        // The supplied signature here only conveys a slice with no return
        // value and so cannot place nodes into the tree directly. We
        // implement it as a pre-sort helper consistent with the algorithm's
        // first step at every depth.
        let axis = depth % 3;
        match axis {
            0 => points.sort_by(compare_x),
            1 => points.sort_by(compare_y),
            _ => points.sort_by(compare_z),
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
    let (split, left, right) = {
        let n = root.borrow();
        (n.split, n.left.clone(), n.right.clone())
    };

    let mut new_domain = make_space_clone(domain);

    // explore left branch
    new_domain.dim[axis].max = split;
    if let Some(l) = &left {
        explore_branch_impl(tree, l, depth, search_space, &new_domain, iter);
    }

    // explore right branch
    new_domain.dim[axis].max = domain.dim[axis].max;
    new_domain.dim[axis].min = split;
    if let Some(r) = &right {
        explore_branch_impl(tree, r, depth, search_space, &new_domain, iter);
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
    if tree.is_leaf(node) == 1 {
        let idx = node.borrow().idx;
        if tree.point_in_search_space(&tree.points[idx], search_space) == 1 {
            iter.push(tree.points[idx].idx);
        }
    } else if tree.search_area_intersects(search_space, domain) == 1 {
        if tree.completely_enclosed(search_space, domain) == 1 {
            report_all_leaves_impl(tree, node, iter);
        } else {
            search_kd_impl(tree, node, depth + 1, search_space, domain, iter);
        }
    }
}

fn report_all_leaves_impl(
    tree: &KDTree,
    node: &Rc<RefCell<TreeNode>>,
    iter: &mut KDTreeIterator,
) {
    let (left, right, idx) = {
        let n = node.borrow();
        (n.left.clone(), n.right.clone(), n.idx)
    };
    if left.is_none() && right.is_none() {
        iter.push(tree.points[idx].idx);
    } else {
        if let Some(l) = &left {
            report_all_leaves_impl(tree, l, iter);
        }
        if let Some(r) = &right {
            report_all_leaves_impl(tree, r, iter);
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
        // Mirrors the C version: keep allocated buffer, just reset counters.
        self.data.clear();
        self.size = 0;
        self.current = 0;
    }

    pub fn push(&mut self, value: usize) {
        if self.size == self.capacity {
            // grow capacity using configured ratio
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
        }
        self.data.push(value);
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
        self.data.sort_by(compare_size_t);
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
