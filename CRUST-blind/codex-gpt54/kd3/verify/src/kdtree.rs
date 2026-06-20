use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

const DIM_X: usize = 0;
const DIM_Y: usize = 1;
const DIM_Z: usize = 2;
const NDIMS: usize = 3;
const KDTREE_ITERATOR_INITIAL_SIZE: usize = 50;
const KDTREE_ITERATOR_GROWTH_RATIO: usize = 2;

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

        if self.count != count {
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = (0..count)
                .map(|_| DataPoint {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    idx: 0,
                })
                .collect();
            self.node_data = (0..self.max_nodes)
                .map(|_| Rc::new(RefCell::new(empty_node())))
                .collect();
        }

        self.next_node = 0;

        for i in 0..count {
            self.points[i].idx = i;
            self.points[i].x = x[i];
            self.points[i].y = y[i];
            self.points[i].z = z[i];
        }

        self.root = Some(self.build_range(0, count - 1, 0));
    }

    pub fn search(&self, iter: &mut Option<KDTreeIterator>, x: f64, y: f64, z: f64, apothem: f64) {
        assert!(apothem >= 0.0);

        if let Some(existing) = iter.as_mut() {
            existing.reset();
            self.search_space_impl(
                existing,
                x - apothem,
                x + apothem,
                y - apothem,
                y + apothem,
                z - apothem,
                z + apothem,
            );
        } else {
            let mut new_iter = KDTreeIterator::new();
            self.search_space_impl(
                &mut new_iter,
                x - apothem,
                x + apothem,
                y - apothem,
                y + apothem,
                z - apothem,
                z + apothem,
            );
            *iter = Some(new_iter);
        }
    }

    pub fn search_space(&self, x_min: f64, x_max: f64, y_min: f64, y_max: f64, z_min: f64, z_max: f64) {
        let mut iter = KDTreeIterator::new();
        self.search_space_impl(&mut iter, x_min, x_max, y_min, y_max, z_min, z_max);
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

        let node = self.node_data[self.next_node].clone();
        self.next_node += 1;
        Some(node)
    }

    fn get_branch_node(&mut self, split: f64) -> Option<Rc<RefCell<TreeNode>>> {
        let node = self.next_node()?;
        {
            let mut node_ref = node.borrow_mut();
            node_ref.left = None;
            node_ref.right = None;
            node_ref.split = split;
            node_ref.idx = 0;
        }
        Some(node)
    }

    fn is_leaf(&self, node: &Rc<RefCell<TreeNode>>) -> i32 {
        let node_ref = node.borrow();
        if node_ref.left.is_none() && node_ref.right.is_none() {
            1
        } else {
            0
        }
    }

    fn point_in_search_space(&self, point: &DataPoint, search_space: &space) -> i32 {
        if point.x <= search_space.dim[DIM_X].max
            && point.x >= search_space.dim[DIM_X].min
            && point.y <= search_space.dim[DIM_Y].max
            && point.y >= search_space.dim[DIM_Y].min
            && point.z <= search_space.dim[DIM_Z].max
            && point.z >= search_space.dim[DIM_Z].min
        {
            1
        } else {
            0
        }
    }

    fn completely_enclosed(&self, search_space: &space, domain: &space) -> i32 {
        if domain.dim[DIM_X].min <= search_space.dim[DIM_X].max
            && domain.dim[DIM_X].min >= search_space.dim[DIM_X].min
            && domain.dim[DIM_X].max <= search_space.dim[DIM_X].max
            && domain.dim[DIM_X].max >= search_space.dim[DIM_X].min
            && domain.dim[DIM_Y].min <= search_space.dim[DIM_Y].max
            && domain.dim[DIM_Y].min >= search_space.dim[DIM_Y].min
            && domain.dim[DIM_Y].max <= search_space.dim[DIM_Y].max
            && domain.dim[DIM_Y].max >= search_space.dim[DIM_Y].min
            && domain.dim[DIM_Z].min <= search_space.dim[DIM_Z].max
            && domain.dim[DIM_Z].min >= search_space.dim[DIM_Z].min
            && domain.dim[DIM_Z].max <= search_space.dim[DIM_Z].max
            && domain.dim[DIM_Z].max >= search_space.dim[DIM_Z].min
        {
            1
        } else {
            0
        }
    }

    fn search_area_intersects(&self, search_space: &space, domain: &space) -> i32 {
        if (search_space.dim[DIM_X].min > domain.dim[DIM_X].max)
            || (search_space.dim[DIM_X].max < domain.dim[DIM_X].min)
            || (search_space.dim[DIM_Y].min > domain.dim[DIM_Y].max)
            || (search_space.dim[DIM_Y].max < domain.dim[DIM_Y].min)
            || (search_space.dim[DIM_Z].min > domain.dim[DIM_Z].max)
            || (search_space.dim[DIM_Z].max < domain.dim[DIM_Z].min)
        {
            0
        } else {
            1
        }
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, iter: &KDTreeIterator) {
        let mut scratch = KDTreeIterator {
            data: iter.data.clone(),
            capacity: iter.capacity,
            size: iter.size,
            current: iter.current,
        };
        self.report_all_leaves_into(node, &mut scratch);
    }

    fn explore_branch(&self, node: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator) {
        let mut scratch = KDTreeIterator {
            data: iter.data.clone(),
            capacity: iter.capacity,
            size: iter.size,
            current: iter.current,
        };
        self.explore_branch_into(node, depth, search_space, domain, &mut scratch);
    }

    fn search_kd(&self, root: &Rc<RefCell<TreeNode>>, depth: usize, search_space: &space, domain: &space, iter: &KDTreeIterator) {
        let mut scratch = KDTreeIterator {
            data: iter.data.clone(),
            capacity: iter.capacity,
            size: iter.size,
            current: iter.current,
        };
        self.search_kd_into(root, depth, search_space, domain, &mut scratch);
    }

    fn build_kdtree(&mut self, points: &mut [DataPoint], depth: usize) {
        self.count = points.len();
        self.next_node = 0;

        if self.count == 0 {
            self.max_nodes = 0;
            self.points.clear();
            self.node_data.clear();
            self.root = None;
            return;
        }

        self.max_nodes = ((self.count - 1) * 2) + 1;
        self.points = points
            .iter()
            .map(|p| DataPoint {
                x: p.x,
                y: p.y,
                z: p.z,
                idx: p.idx,
            })
            .collect();
        self.node_data = (0..self.max_nodes)
            .map(|_| Rc::new(RefCell::new(empty_node())))
            .collect();
        self.root = Some(self.build_range(0, self.count - 1, depth));
    }

    fn get_leaf_node(&mut self, offset: usize) -> Rc<RefCell<TreeNode>> {
        let node = self.next_node().expect("tree node pool exhausted");
        {
            let mut node_ref = node.borrow_mut();
            node_ref.left = None;
            node_ref.right = None;
            node_ref.split = 0.0;
            node_ref.idx = offset;
        }
        node
    }

    fn build_range(&mut self, idx_from: usize, idx_to: usize, depth: usize) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        if count == 1 {
            return self.get_leaf_node(idx_from);
        }

        let mid = idx_from + ((idx_to - idx_from) / 2);
        let axis = depth % NDIMS;
        match axis {
            DIM_X => self.points[idx_from..=idx_to].sort_by(compare_x),
            DIM_Y => self.points[idx_from..=idx_to].sort_by(compare_y),
            _ => self.points[idx_from..=idx_to].sort_by(compare_z),
        }

        let point = &self.points[mid];
        let split = if axis == DIM_X {
            point.x
        } else if axis == DIM_Y {
            point.y
        } else {
            point.z
        };

        let node = self
            .get_branch_node(split)
            .expect("tree node pool exhausted");
        let left = self.build_range(idx_from, mid, depth + 1);
        let right = self.build_range(mid + 1, idx_to, depth + 1);

        {
            let mut node_ref = node.borrow_mut();
            node_ref.left = Some(left);
            node_ref.right = Some(right);
        }

        node
    }

    fn search_space_impl(
        &self,
        iter: &mut KDTreeIterator,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        z_min: f64,
        z_max: f64,
    ) {
        let root = self.root.as_ref().expect("tree must be built before search");
        assert!(self.is_leaf(root) == 0);

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
                    min: -f64::MAX,
                    max: f64::MAX,
                },
                Boundaries {
                    min: -f64::MAX,
                    max: f64::MAX,
                },
                Boundaries {
                    min: -f64::MAX,
                    max: f64::MAX,
                },
            ],
        };

        self.search_kd_into(root, 0, &search_space, &domain, iter);
    }

    fn report_all_leaves_into(&self, node: &Rc<RefCell<TreeNode>>, iter: &mut KDTreeIterator) {
        let (is_leaf, idx, left, right) = {
            let node_ref = node.borrow();
            (
                node_ref.left.is_none() && node_ref.right.is_none(),
                node_ref.idx,
                node_ref.left.clone(),
                node_ref.right.clone(),
            )
        };

        if is_leaf {
            iter.push(self.points[idx].idx);
        } else {
            if let Some(left) = left {
                self.report_all_leaves_into(&left, iter);
            }
            if let Some(right) = right {
                self.report_all_leaves_into(&right, iter);
            }
        }
    }

    fn explore_branch_into(
        &self,
        node: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let _ = depth;
        let (is_leaf, idx) = {
            let node_ref = node.borrow();
            (
                node_ref.left.is_none() && node_ref.right.is_none(),
                node_ref.idx,
            )
        };

        if is_leaf {
            if self.point_in_search_space(&self.points[idx], search_space) != 0 {
                iter.push(self.points[idx].idx);
            }
        } else if self.search_area_intersects(search_space, domain) != 0 {
            if self.completely_enclosed(search_space, domain) != 0 {
                self.report_all_leaves_into(node, iter);
            } else {
                self.search_kd_into(node, depth + 1, search_space, domain, iter);
            }
        }
    }

    fn search_kd_into(
        &self,
        root: &Rc<RefCell<TreeNode>>,
        depth: usize,
        search_space: &space,
        domain: &space,
        iter: &mut KDTreeIterator,
    ) {
        let axis = depth % NDIMS;
        let (split, left, right) = {
            let root_ref = root.borrow();
            (
                root_ref.split,
                root_ref.left.clone().expect("branch nodes must have a left child"),
                root_ref.right.clone().expect("branch nodes must have a right child"),
            )
        };

        let mut new_domain = copy_space(domain);
        new_domain.dim[axis].max = split;
        self.explore_branch_into(&left, depth, search_space, &new_domain, iter);

        let mut new_domain = copy_space(domain);
        new_domain.dim[axis].min = split;
        self.explore_branch_into(&right, depth, search_space, &new_domain, iter);
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
            let needed = self.capacity.saturating_sub(self.data.capacity());
            if needed > 0 {
                self.data.reserve(needed);
            }
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
        self.data.shrink_to_fit();
        self.capacity = 0;
        self.size = 0;
        self.current = 0;
    }

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
    a.cmp(b)
}

fn empty_node() -> TreeNode {
    TreeNode {
        left: None,
        right: None,
        split: 0.0,
        idx: 0,
    }
}

fn copy_space(value: &space) -> space {
    space {
        dim: [
            Boundaries {
                min: value.dim[DIM_X].min,
                max: value.dim[DIM_X].max,
            },
            Boundaries {
                min: value.dim[DIM_Y].min,
                max: value.dim[DIM_Y].max,
            },
            Boundaries {
                min: value.dim[DIM_Z].min,
                max: value.dim[DIM_Z].max,
            },
        ],
    }
}
