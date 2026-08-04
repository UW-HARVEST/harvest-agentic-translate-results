use std::cell::{Cell, RefCell};
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

thread_local! {
    // Stores raw pointer to the current `KDTreeIterator` (as usize, 0 = none).
    // Used to bridge between `search`/`search_space` (which manage the iter)
    // and the recursive helpers whose signatures take iter as `&KDTreeIterator`.
    static CURRENT_ITER: Cell<usize> = const { Cell::new(0) };
}

fn set_current_iter(it: &mut KDTreeIterator) {
    CURRENT_ITER.with(|c| c.set(it as *mut KDTreeIterator as usize));
}

fn current_iter_mut() -> Option<&'static mut KDTreeIterator> {
    let ptr = CURRENT_ITER.with(|c| c.get());
    if ptr == 0 {
        None
    } else {
        Some(unsafe { &mut *(ptr as *mut KDTreeIterator) })
    }
}

fn make_empty_space() -> space {
    space {
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
    }
}

fn copy_space(s: &space) -> space {
    space {
        dim: [
            Boundaries {
                min: s.dim[0].min,
                max: s.dim[0].max,
            },
            Boundaries {
                min: s.dim[1].min,
                max: s.dim[1].max,
            },
            Boundaries {
                min: s.dim[2].min,
                max: s.dim[2].max,
            },
        ],
    }
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

        // Reallocate if size doesn't match.
        if self.count != count || self.node_data.is_empty() {
            self.count = count;
            self.max_nodes = ((count - 1) * 2) + 1;
            self.points = Vec::with_capacity(count);
            self.node_data = (0..self.max_nodes)
                .map(|_| {
                    Rc::new(RefCell::new(TreeNode {
                        left: None,
                        right: None,
                        split: 0.0,
                        idx: 0,
                    }))
                })
                .collect();
        } else {
            self.points.clear();
            // Reset node states for reuse.
            for n in &self.node_data {
                let mut nm = n.borrow_mut();
                nm.left = None;
                nm.right = None;
                nm.split = 0.0;
                nm.idx = 0;
            }
        }

        // Reset control values.
        self.next_node = 0;

        // Cache coordinates.
        for i in 0..count {
            self.points.push(DataPoint {
                x: x[i],
                y: y[i],
                z: z[i],
                idx: i,
            });
        }

        // Build tree.
        let root = self.build_internal(0, count - 1, 0);
        self.root = Some(root);
    }

    fn build_internal(
        &mut self,
        idx_from: usize,
        idx_to: usize,
        depth: usize,
    ) -> Rc<RefCell<TreeNode>> {
        let count = idx_to - idx_from + 1;
        let mid = idx_from + (idx_to - idx_from) / 2;
        let axis = depth % 3;

        if count == 1 {
            // Leaf node.
            let node = self.next_node().expect("ran out of pre-allocated nodes");
            {
                let mut n = node.borrow_mut();
                n.left = None;
                n.right = None;
                n.idx = idx_from;
            }
            return node;
        }

        // Sort sub-slice along the current axis.
        {
            let slice = &mut self.points[idx_from..=idx_to];
            match axis {
                0 => slice.sort_by(compare_x),
                1 => slice.sort_by(compare_y),
                _ => slice.sort_by(compare_z),
            }
        }

        // Determine split.
        let split = match axis {
            0 => self.points[mid].x,
            1 => self.points[mid].y,
            _ => self.points[mid].z,
        };

        // Allocate branch node before recursive calls (matches C order).
        let branch = self
            .get_branch_node(split)
            .expect("ran out of pre-allocated nodes");

        // Recursive build.
        let left = self.build_internal(idx_from, mid, depth + 1);
        let right = self.build_internal(mid + 1, idx_to, depth + 1);

        {
            let mut n = branch.borrow_mut();
            n.left = Some(left);
            n.right = Some(right);
        }

        branch
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
        // Either create a new iterator or reset an existing one.
        if iter.is_none() {
            *iter = Some(KDTreeIterator::new());
        } else {
            iter.as_mut().unwrap().reset();
        }
        let it = iter.as_mut().unwrap();

        // Store pointer so the recursive helpers can mutate the iterator.
        set_current_iter(it);

        let search_space_ = space {
            dim: [
                Boundaries {
                    min: x - apothem,
                    max: x + apothem,
                },
                Boundaries {
                    min: y - apothem,
                    max: y + apothem,
                },
                Boundaries {
                    min: z - apothem,
                    max: z + apothem,
                },
            ],
        };
        let domain = make_empty_space();

        if let Some(root) = &self.root {
            self.search_kd(root, 0, &search_space_, &domain, it);
        }
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
        // Recover iterator pointer set by the most recent `search` call.
        let ptr = CURRENT_ITER.with(|c| c.get());
        if ptr == 0 {
            return;
        }
        let it = unsafe { &mut *(ptr as *mut KDTreeIterator) };
        it.reset();

        let search_space_ = space {
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
        let domain = make_empty_space();

        if let Some(root) = &self.root {
            self.search_kd(root, 0, &search_space_, &domain, it);
        }
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
        for i in 0..3 {
            if search_space.dim[i].min > domain.dim[i].max
                || search_space.dim[i].max < domain.dim[i].min
            {
                return 0;
            }
        }
        1
    }

    fn report_all_leaves(&self, node: &Rc<RefCell<TreeNode>>, _iter: &KDTreeIterator) {
        if self.is_leaf(node) != 0 {
            let leaf_idx = node.borrow().idx;
            let it = current_iter_mut().expect("no current iterator");
            it.push(self.points[leaf_idx].idx);
        } else {
            let (left, right) = {
                let n = node.borrow();
                (n.left.clone(), n.right.clone())
            };
            if let Some(l) = &left {
                self.report_all_leaves(l, _iter);
            }
            if let Some(r) = &right {
                self.report_all_leaves(r, _iter);
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
        if self.is_leaf(node) != 0 {
            let leaf_idx = node.borrow().idx;
            if self.point_in_search_space(&self.points[leaf_idx], search_space) != 0 {
                let it = current_iter_mut().expect("no current iterator");
                it.push(self.points[leaf_idx].idx);
            }
        } else if self.search_area_intersects(search_space, domain) != 0 {
            if self.completely_enclosed(search_space, domain) != 0 {
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

        let mut new_domain = copy_space(domain);

        // Explore left branch.
        new_domain.dim[axis].max = split;
        if let Some(l) = &left {
            self.explore_branch(l, depth, search_space, &new_domain, iter);
        }

        // Explore right branch.
        new_domain.dim[axis].max = domain.dim[axis].max;
        new_domain.dim[axis].min = split;
        if let Some(r) = &right {
            self.explore_branch(r, depth, search_space, &new_domain, iter);
        }
    }

    fn build_kdtree(&mut self, _points: &mut [DataPoint], _depth: usize) {
        // This stub variant is unused; the real recursive build is implemented
        // in `build_internal`, which uses index ranges over `self.points` so it
        // can populate the pre-allocated `node_data` arena correctly.
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

    pub fn reset(&mut self) {
        self.size = 0;
        self.current = 0;
    }

    pub fn push(&mut self, value: usize) {
        if self.size >= self.capacity {
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
        let n = self.size;
        if n > 0 && n <= self.data.len() {
            self.data[..n].sort_by(compare_size_t);
        }
    }
}

fn compare_x(a: &DataPoint, b: &DataPoint) -> Ordering {
    a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal)
}
fn compare_y(a: &DataPoint, b: &DataPoint) -> Ordering {
    a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal)
}
fn compare_z(a: &DataPoint, b: &DataPoint) -> Ordering {
    a.z.partial_cmp(&b.z).unwrap_or(Ordering::Equal)
}
fn compare_size_t(a: &usize, b: &usize) -> Ordering {
    a.cmp(b)
}
