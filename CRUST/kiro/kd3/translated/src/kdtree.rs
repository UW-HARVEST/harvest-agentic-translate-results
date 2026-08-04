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

thread_local! {
    static LAST_ITER_PTR: RefCell<*mut Option<KDTreeIterator>> = RefCell::new(std::ptr::null_mut());
}

fn copy_space(s: &space) -> space {
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
        self.count = count;
        self.max_nodes = ((count - 1) * 2) + 1;
        self.next_node = 0;
        self.node_data = Vec::with_capacity(self.max_nodes);
        self.points = (0..count).map(|i| DataPoint { x: x[i], y: y[i], z: z[i], idx: i }).collect();
        let mut indices: Vec<usize> = (0..count).collect();
        self.root = Some(self.build_tree(&mut indices, 0));
    }
    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);
        LAST_ITER_PTR.with(|p| {
            *p.borrow_mut() = iter as *mut Option<KDTreeIterator>;
        });
        self.search_space_impl(iter, x - apothem, x + apothem, y - apothem, y + apothem, z - apothem, z + apothem);
    }
    pub fn search_space(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        LAST_ITER_PTR.with(|p| {
            let ptr = *p.borrow();
            assert!(!ptr.is_null());
            let iter = unsafe { &mut *ptr };
            self.search_space_impl(iter, x_min, x_max, y_min, y_max, z_min, z_max);
        });
    }

    fn search_space_impl(&self, iter: &mut Option<KDTreeIterator>, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        let root = self.root.as_ref().unwrap();
        assert!(self.is_leaf(root) == 0);

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
        // Use a raw pointer to the iterator to allow pushing from &self methods
        let iter_ptr = iter.as_mut().unwrap() as *mut KDTreeIterator;
        self.search_kd_raw(self.root.as_ref().unwrap(), 0, &search_space, &domain, iter_ptr);
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
        if point.x <= search_space.dim[0].max && point.x >= search_space.dim[0].min
            && point.y <= search_space.dim[1].max && point.y >= search_space.dim[1].min
            && point.z <= search_space.dim[2].max && point.z >= search_space.dim[2].min
        { 1 } else { 0 }
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
    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator){
        self.report_all_leaves_raw(node, iter as *const KDTreeIterator as *mut KDTreeIterator);
    }

    fn report_all_leaves_raw(&self, node: &Rc<RefCell<TreeNode>>, iter_ptr: *mut KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let idx = node.borrow().idx;
            unsafe { (*iter_ptr).push(self.points[idx].idx); }
        } else {
            let n = node.borrow();
            if let Some(ref left) = n.left {
                self.report_all_leaves_raw(left, iter_ptr);
            }
            if let Some(ref right) = n.right {
                self.report_all_leaves_raw(right, iter_ptr);
            }
        }
    }

    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        self.explore_branch_raw(node, depth, search_space, domain, iter as *const KDTreeIterator as *mut KDTreeIterator);
    }

    fn explore_branch_raw(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter_ptr: *mut KDTreeIterator) {
        if self.is_leaf(node) == 1 {
            let idx = node.borrow().idx;
            if self.point_in_search_space(&self.points[idx], search_space) == 1 {
                unsafe { (*iter_ptr).push(self.points[idx].idx); }
            }
        } else if self.search_area_intersects(search_space, domain) == 1 {
            if self.completely_enclosed(search_space, domain) == 1 {
                self.report_all_leaves_raw(node, iter_ptr);
            } else {
                self.search_kd_raw(node, depth + 1, search_space, domain, iter_ptr);
            }
        }
    }

    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator){
        self.search_kd_raw(root, depth, search_space, domain, iter as *const KDTreeIterator as *mut KDTreeIterator);
    }

    fn search_kd_raw(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter_ptr: *mut KDTreeIterator) {
        let axis = depth % 3;
        let root_borrow = root.borrow();
        let split = root_borrow.split;

        let mut left_domain = copy_space(domain);
        left_domain.dim[axis].max = split;
        if let Some(ref left) = root_borrow.left {
            self.explore_branch_raw(left, depth, search_space, &left_domain, iter_ptr);
        }

        let mut right_domain = copy_space(domain);
        right_domain.dim[axis].min = split;
        if let Some(ref right) = root_borrow.right {
            self.explore_branch_raw(right, depth, search_space, &right_domain, iter_ptr);
        }
    }

    fn build_kdtree(&mut self, _points: &mut [DataPoint], _depth: usize) {
        // Not used; build_tree is the actual recursive builder
    }

    fn build_tree(&mut self, indices: &mut [usize], depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = indices.len();
        let axis = depth % 3;

        if count == 1 {
            let node = self.next_node().unwrap();
            {
                let mut n = node.borrow_mut();
                n.idx = indices[0];
            }
            return node;
        }

        let points = &self.points;
        indices.sort_by(|&a, &b| {
            let va = match axis { 0 => points[a].x, 1 => points[a].y, _ => points[a].z };
            let vb = match axis { 0 => points[b].x, 1 => points[b].y, _ => points[b].z };
            if va > vb { Ordering::Greater } else if va < vb { Ordering::Less } else { Ordering::Equal }
        });

        let mid = (count - 1) / 2;
        let mid_idx = indices[mid];
        let split = match axis {
            0 => points[mid_idx].x,
            1 => points[mid_idx].y,
            _ => points[mid_idx].z,
        };

        let node = self.get_branch_node(split).unwrap();
        let (left_slice, right_slice) = indices.split_at_mut(mid + 1);
        let left = self.build_tree(left_slice, depth + 1);
        let right = self.build_tree(right_slice, depth + 1);
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
            data: Vec::with_capacity(50),
            capacity: 50,
            size: 0,
            current: 0,
        }
    }
    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
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
        self.data[..self.size].sort();
    }
}
fn compare_x(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.x > b.x { Ordering::Greater } else if a.x < b.x { Ordering::Less } else { Ordering::Equal }
}
fn compare_y(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.y > b.y { Ordering::Greater } else if a.y < b.y { Ordering::Less } else { Ordering::Equal }
}
fn compare_z(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    if a.z > b.z { Ordering::Greater } else if a.z < b.z { Ordering::Less } else { Ordering::Equal }
}
fn compare_size_t(a: &usize, b: &usize) -> std::cmp::Ordering {
    a.cmp(b)
}
