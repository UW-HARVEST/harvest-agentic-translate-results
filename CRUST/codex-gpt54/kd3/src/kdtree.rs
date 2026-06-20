use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;

const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

thread_local! {
    static LAST_ITERATOR: Cell<*mut KDTreeIterator> = const { Cell::new(std::ptr::null_mut()) };
}

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
}

impl KDTree {
    pub fn new() -> Self {
        Self {
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
        assert!(x.len() >= count);
        assert!(y.len() >= count);
        assert!(z.len() >= count);

        self.count = count;
        self.max_nodes = ((count - 1) * 2) + 1;
        self.next_node = 0;
        self.node_data.clear();
        self.root = None;

        self.points = (0..count)
            .map(|i| DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            })
            .collect();

        self.root = build_kdtree_slice(
            &mut self.points[..],
            0,
            0,
            &mut self.node_data,
            &mut self.next_node,
        );
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);

        if iter.is_none() {
            *iter = Some(KDTreeIterator::new());
        }

        let iter_ref = iter.as_mut().expect("iterator must exist");
        LAST_ITERATOR.with(|last| last.set(iter_ref as *mut KDTreeIterator));

        self.search_space_into(
            iter_ref,
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
        LAST_ITERATOR.with(|last| {
            let ptr = last.get();
            if ptr.is_null() {
                return;
            }

            // The public Rust stub dropped the iterator argument that exists in C.
            // Reuse the most recently supplied iterator to preserve observable behavior.
            unsafe {
                self.search_space_into(&mut *ptr, x_min, x_max, y_min, y_max, z_min, z_max);
            }
        });
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
        if self.next_node >= self.max_nodes {
            return None;
        }

        self.next_node += 1;
        let node = Rc::new(RefCell::new(TreeNode {
            left: None,
            right: None,
            split: 0.0,
            idx: 0,
        }));
        self.node_data.push(Rc::clone(&node));
        Some(node)
    }

    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        node.borrow_mut().split = split;
        Some(node)
    }

    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let node = node.borrow();
        ((node.left.is_none()) && (node.right.is_none())) as i32
    }

    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        point_in_search_space_impl(point, search_space) as i32
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        completely_enclosed_impl(search_space, domain) as i32
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        search_area_intersects_impl(search_space, domain) as i32
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator) {
        let _ = (node, iter);
    }

    fn explore_branch(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &KDTreeIterator,
    ) {
        let _ = (node, depth, search_space, domain, iter);
    }

    fn search_kd(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &KDTreeIterator,
    ) {
        let _ = (root, depth, search_space, domain, iter);
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        self.next_node = 0;
        self.node_data.clear();
        self.root =
            build_kdtree_slice(points, 0, depth, &mut self.node_data, &mut self.next_node);
    }

    fn search_space_into(
        &self,
        iter: &mut KDTreeIterator,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        let root = self.root.as_ref().expect("tree must be built before searching");
        assert!(self.is_leaf(root) == 0);

        iter.reset();

        let search_space = make_space(x_min, x_max, y_min, y_max, z_min, z_max);
        let domain = make_space(
            -f64::MAX,
            f64::MAX,
            -f64::MAX,
            f64::MAX,
            -f64::MAX,
            f64::MAX,
        );

        self.search_kd_mut(root, 0, &search_space, &domain, iter);
    }

    fn report_all_leaves_mut(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        let (left, right, idx, is_leaf) = {
            let node_ref = node.borrow();
            (
                node_ref.left.clone(),
                node_ref.right.clone(),
                node_ref.idx,
                node_ref.left.is_none() && node_ref.right.is_none(),
            )
        };

        if is_leaf {
            iter.push(self.points[idx].idx);
        } else {
            self.report_all_leaves_mut(left.as_ref().expect("branch must have left child"), iter);
            self.report_all_leaves_mut(
                right.as_ref().expect("branch must have right child"),
                iter,
            );
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
        let is_leaf = self.is_leaf(node) != 0;
        if is_leaf {
            let idx = node.borrow().idx;
            if self.point_in_search_space(&self.points[idx], search_space) != 0 {
                iter.push(self.points[idx].idx);
            }
        } else if self.search_area_intersects(search_space, domain) != 0 {
            if self.completely_enclosed(search_space, domain) != 0 {
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
        let axis = depth % 3;
        let (split, left, right) = {
            let root_ref = root.borrow();
            (
                root_ref.split,
                root_ref.left.clone().expect("branch must have left child"),
                root_ref.right.clone().expect("branch must have right child"),
            )
        };

        let mut new_domain = clone_space(domain);
        new_domain.dim[axis].max = split;
        self.explore_branch_mut(&left, depth, search_space, &new_domain, iter);

        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        self.explore_branch_mut(&right, depth, search_space, &new_domain, iter);
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
        Self {
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
        if self.size == self.capacity {
            self.capacity *= KDTREE_ITERATOR_GROWTH_RATIO;
            self.data
                .reserve(self.capacity.saturating_sub(self.data.capacity()));
        }

        self.data.push(value);
        self.size += 1;
    }

    pub fn get_next(&mut self) -> Option<usize> {
        if self.current == self.size {
            None
        } else {
            let value = self.data[self.current];
            self.current += 1;
            Some(value)
        }
    }

    fn rewind(&mut self) {
        self.current = 0;
    }

    fn delete(&mut self) {
        self.data.clear();
        self.data.shrink_to(0);
        self.capacity = 0;
        self.size = 0;
        self.current = 0;
    }

    fn sort(&mut self) {
        self.data.sort_by(compare_size_t);
        self.rewind();
    }
}

fn compare_x(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    cmp_f64(a.x, b.x)
}

fn compare_y(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    cmp_f64(a.y, b.y)
}

fn compare_z(a: &DataPoint, b: &DataPoint) -> std::cmp::Ordering {
    cmp_f64(a.z, b.z)
}

fn compare_size_t(a: &usize, b: &usize) -> std::cmp::Ordering {
    a.cmp(b)
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    if a > b {
        Ordering::Greater
    } else if a < b {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

fn clone_space(src: &space) -> space {
    make_space(
        src.dim[0].min,
        src.dim[0].max,
        src.dim[1].min,
        src.dim[1].max,
        src.dim[2].min,
        src.dim[2].max,
    )
}

fn make_space(x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) -> space {
    space {
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
    }
}

fn point_in_search_space_impl(point: &DataPoint, search_space: &space) -> bool {
    (point.x <= search_space.dim[0].max)
        && (point.x >= search_space.dim[0].min)
        && (point.y <= search_space.dim[1].max)
        && (point.y >= search_space.dim[1].min)
        && (point.z <= search_space.dim[2].max)
        && (point.z >= search_space.dim[2].min)
}

fn completely_enclosed_impl(search_space: &space, domain: &space) -> bool {
    (domain.dim[0].min <= search_space.dim[0].max)
        && (domain.dim[0].min >= search_space.dim[0].min)
        && (domain.dim[0].max <= search_space.dim[0].max)
        && (domain.dim[0].max >= search_space.dim[0].min)
        && (domain.dim[1].min <= search_space.dim[1].max)
        && (domain.dim[1].min >= search_space.dim[1].min)
        && (domain.dim[1].max <= search_space.dim[1].max)
        && (domain.dim[1].max >= search_space.dim[1].min)
        && (domain.dim[2].min <= search_space.dim[2].max)
        && (domain.dim[2].min >= search_space.dim[2].min)
        && (domain.dim[2].max <= search_space.dim[2].max)
        && (domain.dim[2].max >= search_space.dim[2].min)
}

fn search_area_intersects_impl(search_space: &space, domain: &space) -> bool {
    !((search_space.dim[0].min > domain.dim[0].max)
        || (search_space.dim[0].max < domain.dim[0].min)
        || (search_space.dim[1].min > domain.dim[1].max)
        || (search_space.dim[1].max < domain.dim[1].min)
        || (search_space.dim[2].min > domain.dim[2].max)
        || (search_space.dim[2].max < domain.dim[2].min))
}

fn build_kdtree_slice(
    points: &mut [DataPoint],
    base_idx: usize,
    depth: usize,
    node_data: &mut Vec<Rc<RefCell<TreeNode>>>,
    next_node: &mut usize,
) -> Option<Rc<RefCell<TreeNode>>> {
    if points.is_empty() {
        return None;
    }

    *next_node += 1;

    if points.len() == 1 {
        let node = Rc::new(RefCell::new(TreeNode {
            left: None,
            right: None,
            split: 0.0,
            idx: base_idx,
        }));
        node_data.push(Rc::clone(&node));
        return Some(node);
    }

    match depth % 3 {
        0 => points.sort_by(compare_x),
        1 => points.sort_by(compare_y),
        _ => points.sort_by(compare_z),
    }

    let mid = (points.len() - 1) / 2;
    let split = match depth % 3 {
        0 => points[mid].x,
        1 => points[mid].y,
        _ => points[mid].z,
    };

    let node = Rc::new(RefCell::new(TreeNode {
        left: None,
        right: None,
        split,
        idx: 0,
    }));
    node_data.push(Rc::clone(&node));

    let (left_points, right_points) = points.split_at_mut(mid + 1);
    let left = build_kdtree_slice(left_points, base_idx, depth + 1, node_data, next_node);
    let right = build_kdtree_slice(
        right_points,
        base_idx + mid + 1,
        depth + 1,
        node_data,
        next_node,
    );

    {
        let mut node_ref = node.borrow_mut();
        node_ref.left = left;
        node_ref.right = right;
    }

    Some(node)
}
