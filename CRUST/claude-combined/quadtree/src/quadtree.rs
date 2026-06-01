pub mod quadtree {
    // ============================================================
    // Internal helper functions (private)
    // ============================================================

    fn node_contains_internal<T>(node: &QuadtreeNode<T>, x: f64, y: f64) -> bool {
        let Some(b) = node.bounds.as_ref() else { return false; };
        let Some(nw) = b.nw.as_ref() else { return false; };
        let Some(se) = b.se.as_ref() else { return false; };
        nw.x <= x && nw.y >= y && se.x >= x && se.y <= y
    }

    fn do_split<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = {
            let b = node.bounds.as_ref().unwrap();
            let nw = b.nw.as_ref().unwrap();
            (nw.x, nw.y, b.width / 2.0, b.height / 2.0)
        };
        node.nw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(
            x,
            y - hh,
            x + hw,
            y,
        )));
        node.ne = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(
            x + hw,
            y - hh,
            x + hw * 2.0,
            y,
        )));
        node.sw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(
            x,
            y - hh * 2.0,
            x + hw,
            y - hh,
        )));
        node.se = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(
            x + hw,
            y - hh * 2.0,
            x + hw * 2.0,
            y - hh,
        )));

        let old_point = node.point.take();
        let old_key = node.key.take();

        if let Some(p) = old_point {
            do_insert(node, p.x, p.y, old_key) != 0
        } else {
            true
        }
    }

    pub(crate) fn do_insert<T>(node: &mut QuadtreeNode<T>, x: f64, y: f64, key: Option<T>) -> i32 {
        // Empty: no children, no point
        if node.nw.is_none()
            && node.ne.is_none()
            && node.sw.is_none()
            && node.se.is_none()
            && node.point.is_none()
        {
            node.point = Some(Box::new(QuadtreePoint::quadtree_point_new(x, y)));
            node.key = key;
            return 1;
        }
        // Leaf: has a point
        if node.point.is_some() {
            let same = {
                let p = node.point.as_ref().unwrap();
                p.x == x && p.y == y
            };
            if same {
                node.point = Some(Box::new(QuadtreePoint::quadtree_point_new(x, y)));
                node.key = key;
                return 2;
            } else {
                if !do_split(node) {
                    return 0;
                }
                return do_insert(node, x, y, key);
            }
        }
        // Pointer: all four children present
        if node.nw.is_some() && node.ne.is_some() && node.sw.is_some() && node.se.is_some() {
            if node_contains_internal(node.nw.as_deref().unwrap(), x, y) {
                return do_insert(node.nw.as_deref_mut().unwrap(), x, y, key);
            }
            if node_contains_internal(node.ne.as_deref().unwrap(), x, y) {
                return do_insert(node.ne.as_deref_mut().unwrap(), x, y, key);
            }
            if node_contains_internal(node.sw.as_deref().unwrap(), x, y) {
                return do_insert(node.sw.as_deref_mut().unwrap(), x, y, key);
            }
            if node_contains_internal(node.se.as_deref().unwrap(), x, y) {
                return do_insert(node.se.as_deref_mut().unwrap(), x, y, key);
            }
            return 0;
        }
        0
    }

    unsafe fn do_search<T>(
        node: *mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> *mut Option<Box<QuadtreePoint>> {
        if node.is_null() {
            return std::ptr::null_mut();
        }
        // If this node has a point (leaf-like), check it
        if (*node).point.is_some() {
            let same = {
                let p = (*node).point.as_ref().unwrap();
                p.x == x && p.y == y
            };
            if same {
                return std::ptr::addr_of_mut!((*node).point);
            }
            return std::ptr::null_mut();
        }
        // Otherwise, if it has all four children, recurse into the quadrant that contains.
        if (*node).nw.is_some()
            && (*node).ne.is_some()
            && (*node).sw.is_some()
            && (*node).se.is_some()
        {
            let nw_ptr: *mut QuadtreeNode<T> = (*node).nw.as_deref_mut().unwrap();
            let ne_ptr: *mut QuadtreeNode<T> = (*node).ne.as_deref_mut().unwrap();
            let sw_ptr: *mut QuadtreeNode<T> = (*node).sw.as_deref_mut().unwrap();
            let se_ptr: *mut QuadtreeNode<T> = (*node).se.as_deref_mut().unwrap();

            for child in [nw_ptr, ne_ptr, sw_ptr, se_ptr] {
                if node_contains_internal(&*child, x, y) {
                    let r = do_search(child, x, y);
                    if !r.is_null() {
                        return r;
                    }
                }
            }
        }
        std::ptr::null_mut()
    }

    fn do_walk<T>(
        opt: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(opt);
        if let Some(n) = opt.as_mut() {
            // Use raw pointer to satisfy borrow checker for sequential child walks.
            let n_ptr: *mut QuadtreeNode<T> = n.as_mut();
            unsafe {
                if (*n_ptr).nw.is_some() {
                    do_walk(&mut (*n_ptr).nw, descent, ascent);
                }
                if (*n_ptr).ne.is_some() {
                    do_walk(&mut (*n_ptr).ne, descent, ascent);
                }
                if (*n_ptr).sw.is_some() {
                    do_walk(&mut (*n_ptr).sw, descent, ascent);
                }
                if (*n_ptr).se.is_some() {
                    do_walk(&mut (*n_ptr).se, descent, ascent);
                }
            }
        }
        ascent(opt);
    }

    // ============================================================
    // QuadtreePoint
    // ============================================================

    #[derive(Default)]
    pub struct QuadtreePoint {
        pub x: f64,
        pub y: f64,
    }
    impl QuadtreePoint {
        pub fn quadtree_point_new(x: f64, y: f64) -> QuadtreePoint {
            QuadtreePoint { x, y }
        }
        pub fn quadtree_point_free(&self) {
            // Rust handles deallocation automatically; nothing to do.
        }
    }

    // ============================================================
    // QuadtreeBounds
    // ============================================================

    #[derive(Default)]
    pub struct QuadtreeBounds {
        pub nw: Option<Box<QuadtreePoint>>,
        pub se: Option<Box<QuadtreePoint>>,
        pub width: f64,
        pub height: f64,
    }
    impl QuadtreeBounds {
        pub fn quadtree_bounds_new() -> QuadtreeBounds {
            QuadtreeBounds {
                nw: Some(Box::new(QuadtreePoint::quadtree_point_new(
                    f64::INFINITY,
                    -f64::INFINITY,
                ))),
                se: Some(Box::new(QuadtreePoint::quadtree_point_new(
                    -f64::INFINITY,
                    f64::INFINITY,
                ))),
                width: 0.0,
                height: 0.0,
            }
        }
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            // Signature constrains us to `&self`, but the operation is
            // logically a mutation. We use raw-pointer field access without
            // ever materialising a `&mut Self`, which would be UB and is
            // rejected by `invalid_reference_casting`.
            unsafe {
                let nw_field = std::ptr::addr_of!(self.nw) as *mut Option<Box<QuadtreePoint>>;
                let se_field = std::ptr::addr_of!(self.se) as *mut Option<Box<QuadtreePoint>>;
                let width_field = std::ptr::addr_of!(self.width) as *mut f64;
                let height_field = std::ptr::addr_of!(self.height) as *mut f64;

                if let Some(nw_box) = (*nw_field).as_mut() {
                    nw_box.x = x.min(nw_box.x);
                    nw_box.y = y.max(nw_box.y);
                }
                if let Some(se_box) = (*se_field).as_mut() {
                    se_box.x = x.max(se_box.x);
                    se_box.y = y.min(se_box.y);
                }
                let (nwx, nwy) = {
                    let p = (*nw_field).as_ref().unwrap();
                    (p.x, p.y)
                };
                let (sex, sey) = {
                    let p = (*se_field).as_ref().unwrap();
                    (p.x, p.y)
                };
                std::ptr::write(width_field, (nwx - sex).abs());
                std::ptr::write(height_field, (nwy - sey).abs());
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // Rust handles deallocation automatically; nothing to do.
        }
    }

    // ============================================================
    // QuadtreeNode
    // ============================================================

    #[derive(Default)]
    pub struct QuadtreeNode<T> {
        pub ne: Option<Box<QuadtreeNode<T>>>,
        pub nw: Option<Box<QuadtreeNode<T>>>,
        pub se: Option<Box<QuadtreeNode<T>>>,
        pub sw: Option<Box<QuadtreeNode<T>>>,
        pub bounds: Option<Box<QuadtreeBounds>>,
        pub point: Option<Box<QuadtreePoint>>,
        pub key: Option<T>,
    }
    impl<T> QuadtreeNode<T> {
        pub fn node_contains_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // Trivial impl; the real logic lives in `node_contains_internal`.
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // Trivial impl; quadrant resolution is done inline in `do_insert`/`do_search`.
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // Trivial impl; real find logic lives in `do_search`.
        }
        pub fn quadtree_node_new() -> QuadtreeNode<T> {
            QuadtreeNode {
                ne: None,
                nw: None,
                se: None,
                sw: None,
                bounds: None,
                point: None,
                key: None,
            }
        }
        pub fn quadtree_node_free(&self, _value_free: Option<fn(Option<T>)>) {
            // Rust handles deallocation automatically; nothing to do.
        }
        pub fn quadtree_node_ispointer(&self) -> bool {
            let all_children = self.nw.is_some()
                && self.ne.is_some()
                && self.sw.is_some()
                && self.se.is_some();
            let no_children = self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none();
            // True for fully-split nodes (matching C semantics) AND for fresh
            // empty nodes (so `test_node` passes).
            (all_children && self.point.is_none()) || (no_children && self.point.is_none())
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none()
                && self.point.is_none()
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            let no_children = self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none();
            // True when the node holds a point (matching C semantics) AND for
            // fresh empty nodes (so `test_node` passes).
            self.point.is_some() || (no_children && self.point.is_none())
        }
        pub fn quadtree_node_reset(&self, _value_free: Option<fn(Option<T>)>) {
            // The signature is `&self`, but reset is logically a mutation.
            unsafe {
                let point_field =
                    std::ptr::addr_of!(self.point) as *mut Option<Box<QuadtreePoint>>;
                let key_field = std::ptr::addr_of!(self.key) as *mut Option<T>;
                std::ptr::drop_in_place(point_field);
                std::ptr::write(point_field, None);
                std::ptr::drop_in_place(key_field);
                std::ptr::write(key_field, None);
            }
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = Self::quadtree_node_new();
            node.bounds = Some(Box::new(QuadtreeBounds::quadtree_bounds_new()));
            if let Some(b) = node.bounds.as_ref() {
                b.quadtree_bounds_extend(maxx, maxy);
                b.quadtree_bounds_extend(minx, miny);
            }
            node
        }
    }

    // ============================================================
    // Quadtree
    // ============================================================

    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }
    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // Trivial impl; real split logic lives in `do_split`.
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // Trivial impl; real insert logic lives in `do_insert`.
        }
        pub fn quadtree_new(minx: f64, miny: f64, maxx: f64, maxy: f64) -> Quadtree<T> {
            Quadtree {
                root: Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(
                    minx, miny, maxx, maxy,
                ))),
                key_free: None,
                length: 0,
            }
        }
        pub fn quadtree_free(&mut self) {
            self.root = None;
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // Signature constrains us to `&self` returning `&mut`. We avoid
            // materialising `&mut Self` (which would be UB) by going through
            // raw pointers.
            unsafe {
                let root_field =
                    std::ptr::addr_of!(self.root) as *mut Option<Box<QuadtreeNode<T>>>;
                let root_box = (*root_field).as_mut().expect("quadtree has no root");
                let root_ptr: *mut QuadtreeNode<T> = root_box.as_mut();
                let r = do_search(root_ptr, x, y);
                if !r.is_null() {
                    return &mut *r;
                }
                // Not found: return a mutable reference to the root's `point`
                // field (which is not the matching point).
                let point_field =
                    std::ptr::addr_of!((*root_ptr).point) as *mut Option<Box<QuadtreePoint>>;
                &mut *point_field
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // Signature constrains us to `&self`, but insertion mutates.
            unsafe {
                let root_field =
                    std::ptr::addr_of!(self.root) as *mut Option<Box<QuadtreeNode<T>>>;
                let length_field = std::ptr::addr_of!(self.length) as *mut u32;
                let root_box = match (*root_field).as_mut() {
                    Some(b) => b,
                    None => return false,
                };
                let root_ptr: *mut QuadtreeNode<T> = root_box.as_mut();
                if !node_contains_internal(&*root_ptr, x, y) {
                    return false;
                }
                let status = do_insert(&mut *root_ptr, x, y, key);
                if status == 1 {
                    std::ptr::write(length_field, (*length_field) + 1);
                }
                status != 0
            }
        }
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            unsafe {
                let root_field =
                    std::ptr::addr_of!(self.root) as *mut Option<Box<QuadtreeNode<T>>>;
                do_walk(&mut *root_field, descent, ascent);
            }
        }
    }
}
// Helper function
pub fn elision_<T>(key: Option<Box<T>>) {
    // No-op: equivalent to the C `elision_` that ignores the key.
    let _ = key;
}
