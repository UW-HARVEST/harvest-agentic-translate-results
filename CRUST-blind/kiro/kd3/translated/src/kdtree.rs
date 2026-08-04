#![allow(dead_code)]
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
#[allow(non_camel_case_types)]
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
const NDIMS: usize = 3;
const DIM_X: usize = 0;
const DIM_Y: usize = 1;
const DIM_Z: usize = 2;

fn clone_space(s: &space) -> space {
    space {
        dim: [
            Boundaries { min: s.dim[0].min, max: s.dim[0].max },
            Boundaries { min: s.dim[1].min, max: s.dim[1].max },
            Boundaries { min: s.dim[2].min, max: s.dim[2].max },
        ]
    }
}

fn point_in_ss(point: &DataPoint, ss: &space) -> bool {
    point.x <= ss.dim[DIM_X].max && point.x >= ss.dim[DIM_X].min
        && point.y <= ss.dim[DIM_Y].max && point.y >= ss.dim[DIM_Y].min
        && point.z <= ss.dim[DIM_Z].max && point.z >= ss.dim[DIM_Z].min
}

fn fully_enclosed(ss: &space, dom: &space) -> bool {
    dom.dim[DIM_X].min <= ss.dim[DIM_X].max && dom.dim[DIM_X].min >= ss.dim[DIM_X].min
        && dom.dim[DIM_X].max <= ss.dim[DIM_X].max && dom.dim[DIM_X].max >= ss.dim[DIM_X].min
        && dom.dim[DIM_Y].min <= ss.dim[DIM_Y].max && dom.dim[DIM_Y].min >= ss.dim[DIM_Y].min
        && dom.dim[DIM_Y].max <= ss.dim[DIM_Y].max && dom.dim[DIM_Y].max >= ss.dim[DIM_Y].min
        && dom.dim[DIM_Z].min <= ss.dim[DIM_Z].max && dom.dim[DIM_Z].min >= ss.dim[DIM_Z].min
        && dom.dim[DIM_Z].max <= ss.dim[DIM_Z].max && dom.dim[DIM_Z].max >= ss.dim[DIM_Z].min
}

fn areas_intersect(ss: &space, dom: &space) -> bool {
    !(ss.dim[DIM_X].min > dom.dim[DIM_X].max || ss.dim[DIM_X].max < dom.dim[DIM_X].min
        || ss.dim[DIM_Y].min > dom.dim[DIM_Y].max || ss.dim[DIM_Y].max < dom.dim[DIM_Y].min
        || ss.dim[DIM_Z].min > dom.dim[DIM_Z].max || ss.dim[DIM_Z].max < dom.dim[DIM_Z].min)
}

fn is_leaf_node(node: &TreeNode) -> bool {
    node.left.is_none() && node.right.is_none()
}

fn collect_all_leaves(points: &[DataPoint], node: &TreeNode, results: &mut Vec<usize>) {
    if is_leaf_node(node) {
        results.push(points[node.idx].idx);
    } else {
        if let Some(ref left) = node.left {
            collect_all_leaves(points, &left.borrow(), results);
        }
        if let Some(ref right) = node.right {
            collect_all_leaves(points, &right.borrow(), results);
        }
    }
}

fn do_explore_branch(points: &[DataPoint], node: &TreeNode, depth: usize, ss: &space, dom: &space, results: &mut Vec<usize>) {
    if is_leaf_node(node) {
        if point_in_ss(&points[node.idx], ss) {
            results.push(points[node.idx].idx);
        }
    } else if areas_intersect(ss, dom) {
        if fully_enclosed(ss, dom) {
            collect_all_leaves(points, node, results);
        } else {
            do_search_kd(points, node, depth + 1, ss, dom, results);
        }
    }
}

fn do_search_kd(points: &[DataPoint], root: &TreeNode, depth: usize, ss: &space, dom: &space, results: &mut Vec<usize>) {
    let axis = depth % NDIMS;
    let split = root.split;

    let mut new_domain = clone_space(dom);

    // left branch
    new_domain.dim[axis].max = split;
    if let Some(ref left) = root.left {
        do_explore_branch(points, &left.borrow(), depth, ss, &new_domain, results);
    }

    // right branch
    new_domain.dim[axis].max = dom.dim[axis].max;
    new_domain.dim[axis].min = split;
    if let Some(ref right) = root.right {
        do_explore_branch(points, &right.borrow(), depth, ss, &new_domain, results);
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
        if self.count != count {
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
        let root = self.build_kdtree_range(0, count - 1, 0);
        self.root = Some(root);
    }
    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        self.search_space_iter(iter,
            x - apothem, x + apothem,
            y - apothem, y + apothem,
            z - apothem, z + apothem);
    }
    pub fn search_space(&self, _x_min: f64, _x_max: f64, _y_min: f64, _y_max: f64, _z_min: f64, _z_max: f64) {
        // Signature doesn't accept an iterator; actual search uses search_space_iter.
    }

    fn search_space_iter(&self, iter_ptr: &mut Option<KDTreeIterator>,
                         x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        assert!(self.root.is_some());
        match iter_ptr {
            Some(iter) => iter.reset(),
            None => *iter_ptr = Some(KDTreeIterator::new()),
        }
        let iter = iter_ptr.as_mut().unwrap();

        let ss = space {
            dim: [
                Boundaries { min: x_min, max: x_max },
                Boundaries { min: y_min, max: y_max },
                Boundaries { min: z_min, max: z_max },
            ]
        };
        let dom = space {
            dim: [
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
                Boundaries { min: f64::MIN, max: f64::MAX },
            ]
        };

        let root_rc = self.root.as_ref().unwrap();
        let root = root_rc.borrow();
        let mut results = Vec::new();
        do_search_kd(&self.points, &root, 0, &ss, &dom, &mut results);
        for idx in results {
            iter.push(idx);
        }
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
        let node = Rc::new(RefCell::new(TreeNode {
            left: None, right: None, split: 0.0, idx: 0,
        }));
        self.node_data.push(Rc::clone(&node));
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
        if point_in_ss(point, search_space) { 1 } else { 0 }
    }
    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        if fully_enclosed(search_space, domain) { 1 } else { 0 }
    }
    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        if areas_intersect(search_space, domain) { 1 } else { 0 }
    }
    #[allow(unused_variables)]
    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator){
        // Actual work done via collect_all_leaves free function
    }
    #[allow(unused_variables)]
    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        // Actual work done via do_explore_branch free function
    }
    #[allow(unused_variables)]
    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        // Actual work done via do_search_kd free function
    }
    #[allow(unused_variables)]
    fn build_kdtree(&mut self, _points: &mut [DataPoint], _depth: usize) {
        // Actual work done via build_kdtree_range
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
            self.data.reserve(self.capacity - self.data.len());
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
        self.data[..self.size].sort_by(compare_size_t);
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
