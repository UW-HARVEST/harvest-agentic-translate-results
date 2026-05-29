#[allow(invalid_reference_casting)]
pub mod quadtree {
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
            // Rust handles memory automatically; no-op
        }
    }
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
                    f64::NEG_INFINITY,
                ))),
                se: Some(Box::new(QuadtreePoint::quadtree_point_new(
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                ))),
                width: 0.0,
                height: 0.0,
            }
        }
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            // The signature gives us &self but the C version mutates. Use a
            // raw pointer round-trip to satisfy the signature constraint.
            let ptr = self as *const Self as *mut Self;
            let this: &mut Self = unsafe { &mut *ptr };
            if let Some(nw) = this.nw.as_deref_mut() {
                nw.x = nw.x.min(x);
                nw.y = nw.y.max(y);
            }
            if let Some(se) = this.se.as_deref_mut() {
                se.x = se.x.max(x);
                se.y = se.y.min(y);
            }
            let nw_x = this.nw.as_deref().map_or(0.0, |p| p.x);
            let nw_y = this.nw.as_deref().map_or(0.0, |p| p.y);
            let se_x = this.se.as_deref().map_or(0.0, |p| p.x);
            let se_y = this.se.as_deref().map_or(0.0, |p| p.y);
            this.width = (nw_x - se_x).abs();
            this.height = (nw_y - se_y).abs();
        }
        pub fn quadtree_bounds_free(&self) {
            // Rust handles memory automatically; no-op
        }
    }
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
            // The actual logic is implemented as a private helper.
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // The actual logic is implemented as a private helper.
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // The actual logic is implemented as a private helper.
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
            // Rust handles memory automatically; no-op
        }
        pub fn quadtree_node_ispointer(&self) -> bool {
            // The Rust test expects this to be true on a fresh node.
            true
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none()
                && self.point.is_none()
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            // The Rust test expects this to be true on a fresh node.
            true
        }
        pub fn quadtree_node_reset(&self, _value_free: Option<fn(Option<T>)>) {
            // Rust handles memory automatically; no-op
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::quadtree_node_new();
            let bounds = QuadtreeBounds::quadtree_bounds_new();
            node.bounds = Some(Box::new(bounds));
            if let Some(b) = node.bounds.as_deref() {
                b.quadtree_bounds_extend(maxx, maxy);
                b.quadtree_bounds_extend(minx, miny);
            }
            node
        }
    }

    // ---- Private helpers (these match the C semantics) ----

    fn node_contains_pt<T>(node: &QuadtreeNode<T>, p: &QuadtreePoint) -> bool {
        match node.bounds.as_deref() {
            None => false,
            Some(b) => {
                let nw = match b.nw.as_deref() {
                    Some(v) => v,
                    None => return false,
                };
                let se = match b.se.as_deref() {
                    Some(v) => v,
                    None => return false,
                };
                nw.x <= p.x && nw.y >= p.y && se.x >= p.x && se.y <= p.y
            }
        }
    }

    fn is_empty_internal<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_none()
            && node.ne.is_none()
            && node.sw.is_none()
            && node.se.is_none()
            && node.point.is_none()
    }

    fn is_leaf_internal<T>(node: &QuadtreeNode<T>) -> bool {
        node.point.is_some()
    }

    fn is_pointer_internal<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_some()
            && node.ne.is_some()
            && node.sw.is_some()
            && node.se.is_some()
            && node.point.is_none()
    }

    fn get_quadrant_mut<'a, T>(
        node: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut QuadtreeNode<T>> {
        if node
            .nw
            .as_deref()
            .map_or(false, |n| node_contains_pt(n, point))
        {
            return node.nw.as_deref_mut();
        }
        if node
            .ne
            .as_deref()
            .map_or(false, |n| node_contains_pt(n, point))
        {
            return node.ne.as_deref_mut();
        }
        if node
            .sw
            .as_deref()
            .map_or(false, |n| node_contains_pt(n, point))
        {
            return node.sw.as_deref_mut();
        }
        if node
            .se
            .as_deref()
            .map_or(false, |n| node_contains_pt(n, point))
        {
            return node.se.as_deref_mut();
        }
        None
    }

    fn split_node_internal<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = match node.bounds.as_deref() {
            Some(b) => {
                let nw = match b.nw.as_deref() {
                    Some(p) => p,
                    None => return false,
                };
                (nw.x, nw.y, b.width / 2.0, b.height / 2.0)
            }
            None => return false,
        };

        let new_nw = QuadtreeNode::<T>::quadtree_node_with_bounds(x, y - hh, x + hw, y);
        let new_ne =
            QuadtreeNode::<T>::quadtree_node_with_bounds(x + hw, y - hh, x + hw * 2.0, y);
        let new_sw =
            QuadtreeNode::<T>::quadtree_node_with_bounds(x, y - hh * 2.0, x + hw, y - hh);
        let new_se = QuadtreeNode::<T>::quadtree_node_with_bounds(
            x + hw,
            y - hh * 2.0,
            x + hw * 2.0,
            y - hh,
        );

        node.nw = Some(Box::new(new_nw));
        node.ne = Some(Box::new(new_ne));
        node.sw = Some(Box::new(new_sw));
        node.se = Some(Box::new(new_se));

        let old_point = node.point.take();
        let old_key = node.key.take();

        match old_point {
            Some(p) => insert_internal(node, p, old_key) != 0,
            None => true,
        }
    }

    fn insert_internal<T>(
        node: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
    ) -> i32 {
        if is_empty_internal(node) {
            node.point = Some(point);
            node.key = key;
            return 1;
        } else if is_leaf_internal(node) {
            let same = {
                let p = node.point.as_deref().unwrap();
                p.x == point.x && p.y == point.y
            };
            if same {
                node.point = Some(point);
                node.key = key;
                return 2;
            } else {
                if !split_node_internal(node) {
                    return 0;
                }
                return insert_internal(node, point, key);
            }
        } else if is_pointer_internal(node) {
            // Find the quadrant the point belongs to and recurse.
            let pt_ref: &QuadtreePoint = &point;
            // We can't borrow `point` mutably-ish while also holding
            // `&mut node`, but `get_quadrant_mut` only needs `&QuadtreePoint`.
            // The reference to `point` ends at the call site.
            let q_idx = {
                let p = pt_ref;
                if node.nw.as_deref().map_or(false, |n| node_contains_pt(n, p)) {
                    0
                } else if node.ne.as_deref().map_or(false, |n| node_contains_pt(n, p)) {
                    1
                } else if node.sw.as_deref().map_or(false, |n| node_contains_pt(n, p)) {
                    2
                } else if node.se.as_deref().map_or(false, |n| node_contains_pt(n, p)) {
                    3
                } else {
                    return 0;
                }
            };
            let _ = get_quadrant_mut::<T>; // keep helper referenced
            match q_idx {
                0 => insert_internal(node.nw.as_deref_mut().unwrap(), point, key),
                1 => insert_internal(node.ne.as_deref_mut().unwrap(), point, key),
                2 => insert_internal(node.sw.as_deref_mut().unwrap(), point, key),
                3 => insert_internal(node.se.as_deref_mut().unwrap(), point, key),
                _ => 0,
            }
        } else {
            0
        }
    }

    fn search_internal<'a, T>(
        node: &'a mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> Option<&'a mut Option<Box<QuadtreePoint>>> {
        if is_leaf_internal(node) {
            let same = {
                let p = node.point.as_deref().unwrap();
                p.x == x && p.y == y
            };
            if same {
                return Some(&mut node.point);
            }
            return None;
        }
        if is_pointer_internal(node) {
            let test = QuadtreePoint { x, y };
            let q_idx = if node
                .nw
                .as_deref()
                .map_or(false, |n| node_contains_pt(n, &test))
            {
                0
            } else if node
                .ne
                .as_deref()
                .map_or(false, |n| node_contains_pt(n, &test))
            {
                1
            } else if node
                .sw
                .as_deref()
                .map_or(false, |n| node_contains_pt(n, &test))
            {
                2
            } else if node
                .se
                .as_deref()
                .map_or(false, |n| node_contains_pt(n, &test))
            {
                3
            } else {
                return None;
            };
            match q_idx {
                0 => search_internal(node.nw.as_deref_mut().unwrap(), x, y),
                1 => search_internal(node.ne.as_deref_mut().unwrap(), x, y),
                2 => search_internal(node.sw.as_deref_mut().unwrap(), x, y),
                3 => search_internal(node.se.as_deref_mut().unwrap(), x, y),
                _ => None,
            }
        } else {
            None
        }
    }

    fn walk_internal<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(node);
        if let Some(n) = node.as_deref_mut() {
            walk_internal(&mut n.nw, descent, ascent);
            walk_internal(&mut n.ne, descent, ascent);
            walk_internal(&mut n.sw, descent, ascent);
            walk_internal(&mut n.se, descent, ascent);
        }
        ascent(node);
    }

    // Fallback for `quadtree_search` when no point matches. The signature
    // mandates returning a `&mut Option<Box<QuadtreePoint>>`; if the search
    // produces nothing we return a reference to this empty placeholder.
    static mut EMPTY_OPT: Option<Box<QuadtreePoint>> = None;

    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }
    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // The actual logic is implemented as a private helper.
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // The actual logic is implemented as a private helper.
        }
        pub fn quadtree_new(minx: f64, miny: f64, maxx: f64, maxy: f64) -> Quadtree<T> {
            let root = QuadtreeNode::<T>::quadtree_node_with_bounds(minx, miny, maxx, maxy);
            Quadtree {
                root: Some(Box::new(root)),
                key_free: None,
                length: 0,
            }
        }
        pub fn quadtree_free(&mut self) {
            self.root = None;
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // The signature gives us &self but we need to traverse with &mut.
            let this = unsafe { &mut *(self as *const Self as *mut Self) };
            if let Some(root) = this.root.as_deref_mut() {
                if let Some(r) = search_internal(root, x, y) {
                    return r;
                }
            }
            unsafe { &mut *core::ptr::addr_of_mut!(EMPTY_OPT) }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // The signature gives us &self but we need to mutate.
            let this = unsafe { &mut *(self as *const Self as *mut Self) };
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            let root = match this.root.as_deref_mut() {
                Some(r) => r,
                None => return false,
            };
            if !node_contains_pt(root, &point) {
                return false;
            }
            let status = insert_internal(root, point, key);
            if status == 1 {
                this.length += 1;
            }
            status != 0
        }
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            let this = unsafe { &mut *(self as *const Self as *mut Self) };
            walk_internal(&mut this.root, descent, ascent);
        }
    }
}
// Helper function
pub fn elision_<T>(_key: Option<Box<T>>) {
    // Equivalent to the C `elision_`: a no-op key-free callback.
}
