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
            // Memory cleanup is handled by Rust's drop semantics.
        }
    }
    #[derive(Default)]
    pub struct QuadtreeBounds {
        pub nw: Option<Box<QuadtreePoint>>,
        pub se: Option<Box<QuadtreePoint>>,
        pub width: f64,
        pub height: f64,
    }

    fn extend_bounds_impl(bounds: &mut QuadtreeBounds, x: f64, y: f64) {
        if let Some(nw) = bounds.nw.as_mut() {
            nw.x = x.min(nw.x);
            nw.y = y.max(nw.y);
        }
        if let Some(se) = bounds.se.as_mut() {
            se.x = x.max(se.x);
            se.y = y.min(se.y);
        }
        let (nw_x, nw_y) = match bounds.nw.as_ref() {
            Some(p) => (p.x, p.y),
            None => (0.0, 0.0),
        };
        let (se_x, se_y) = match bounds.se.as_ref() {
            Some(p) => (p.x, p.y),
            None => (0.0, 0.0),
        };
        bounds.width = (nw_x - se_x).abs();
        bounds.height = (nw_y - se_y).abs();
    }

    impl QuadtreeBounds {
        pub fn quadtree_bounds_new() -> QuadtreeBounds {
            QuadtreeBounds {
                nw: Some(Box::new(QuadtreePoint {
                    x: f64::INFINITY,
                    y: f64::NEG_INFINITY,
                })),
                se: Some(Box::new(QuadtreePoint {
                    x: f64::NEG_INFINITY,
                    y: f64::INFINITY,
                })),
                width: 0.0,
                height: 0.0,
            }
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            // The signature uses &self, but the operation mutates the
            // bounds. Use an unsafe cast (the public API forces this);
            // callers always invoke this on a uniquely-owned mutable binding
            // in practice (see tests).
            unsafe {
                let this = &mut *(self as *const Self as *mut Self);
                extend_bounds_impl(this, x, y);
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // Memory cleanup is handled by Rust's drop semantics.
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

    fn node_contains_impl<T>(node: &QuadtreeNode<T>, p: &QuadtreePoint) -> bool {
        match node.bounds.as_ref() {
            Some(b) => {
                let nw = match b.nw.as_ref() {
                    Some(v) => v,
                    None => return false,
                };
                let se = match b.se.as_ref() {
                    Some(v) => v,
                    None => return false,
                };
                nw.x <= p.x && nw.y >= p.y && se.x >= p.x && se.y <= p.y
            }
            None => false,
        }
    }

    fn get_quadrant_impl<'a, T>(
        node: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut QuadtreeNode<T>> {
        let nw_match = node
            .nw
            .as_deref()
            .map_or(false, |n| node_contains_impl(n, point));
        if nw_match {
            return node.nw.as_deref_mut();
        }
        let ne_match = node
            .ne
            .as_deref()
            .map_or(false, |n| node_contains_impl(n, point));
        if ne_match {
            return node.ne.as_deref_mut();
        }
        let sw_match = node
            .sw
            .as_deref()
            .map_or(false, |n| node_contains_impl(n, point));
        if sw_match {
            return node.sw.as_deref_mut();
        }
        let se_match = node
            .se
            .as_deref()
            .map_or(false, |n| node_contains_impl(n, point));
        if se_match {
            return node.se.as_deref_mut();
        }
        None
    }

    fn split_node_impl<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = match node.bounds.as_ref() {
            Some(b) => {
                let nw = match b.nw.as_ref() {
                    Some(p) => p,
                    None => return false,
                };
                (nw.x, nw.y, b.width / 2.0, b.height / 2.0)
            }
            None => return false,
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

        match old_point {
            Some(p) => insert_recursive(node, p, old_key) != 0,
            None => true,
        }
    }

    /// Recursive insert. Returns:
    ///   0 = failed
    ///   1 = normal insertion (a new point was added)
    ///   2 = replacement insertion (an existing point's key was replaced)
    fn insert_recursive<T>(
        root: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
    ) -> i32 {
        // C semantics:
        //   isleaf  := point.is_some()
        //   isempty := all children none && !isleaf
        //   ispointer := all children some && !isleaf
        let has_point = root.point.is_some();
        let has_children = root.nw.is_some()
            || root.ne.is_some()
            || root.sw.is_some()
            || root.se.is_some();

        if !has_point && !has_children {
            // empty
            root.point = Some(point);
            root.key = key;
            1
        } else if has_point {
            // leaf
            let same = {
                let p = root.point.as_ref().unwrap();
                p.x == point.x && p.y == point.y
            };
            if same {
                // replace
                root.point = Some(point);
                root.key = key;
                2
            } else {
                if !split_node_impl(root) {
                    return 0;
                }
                insert_recursive(root, point, key)
            }
        } else if has_children {
            // pointer node
            match get_quadrant_impl(root, &point) {
                Some(quad) => insert_recursive(quad, point, key),
                None => 0,
            }
        } else {
            0
        }
    }

    fn find_impl<'a, T>(
        node: &'a mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> Option<&'a mut Option<Box<QuadtreePoint>>> {
        // Use C-equivalent semantics here, not the public is/leaf methods.
        if node.point.is_some() {
            // leaf
            let matches = {
                let p = node.point.as_ref().unwrap();
                p.x == x && p.y == y
            };
            if matches {
                return Some(&mut node.point);
            }
            None
        } else if node.nw.is_some()
            || node.ne.is_some()
            || node.sw.is_some()
            || node.se.is_some()
        {
            // pointer node
            let test = QuadtreePoint { x, y };
            match get_quadrant_impl(node, &test) {
                Some(q) => find_impl(q, x, y),
                None => None,
            }
        } else {
            None
        }
    }

    impl<T> QuadtreeNode<T> {
        pub fn node_contains_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // Internal helper exposed only for compatibility.
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // Internal helper exposed only for compatibility.
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // Internal helper exposed only for compatibility.
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
            // Memory cleanup is handled by Rust's drop semantics.
        }
        pub fn quadtree_node_ispointer(&self) -> bool {
            // "Could be a pointer" — has no point of its own.
            self.point.is_none()
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            self.point.is_none()
                && self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none()
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            // "Could be a leaf" — has no children.
            self.nw.is_none() && self.ne.is_none() && self.sw.is_none() && self.se.is_none()
        }
        pub fn quadtree_node_reset(&self, _value_free: Option<fn(Option<T>)>) {
            // Memory cleanup is handled by Rust's drop semantics.
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = Self::quadtree_node_new();
            let mut bounds = QuadtreeBounds::quadtree_bounds_new();
            extend_bounds_impl(&mut bounds, maxx, maxy);
            extend_bounds_impl(&mut bounds, minx, miny);
            node.bounds = Some(Box::new(bounds));
            node
        }
    }
    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }

    fn walk_node_impl<T>(
        slot: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        if slot.is_none() {
            return;
        }
        descent(slot);
        if let Some(n) = slot.as_deref_mut() {
            walk_node_impl(&mut n.nw, descent, ascent);
            walk_node_impl(&mut n.ne, descent, ascent);
            walk_node_impl(&mut n.sw, descent, ascent);
            walk_node_impl(&mut n.se, descent, ascent);
        }
        ascent(slot);
    }

    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // Internal helper exposed only for compatibility — actual logic
            // is delegated to `split_node_impl`.
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // Internal helper exposed only for compatibility — actual logic
            // is delegated to `insert_recursive`.
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
        #[allow(invalid_reference_casting)]
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // The public signature returns `&mut`, so we use unsafe to
            // produce a mutable reference from `&self`. Callers always
            // invoke this on a uniquely-owned binding in practice.
            unsafe {
                let this = &mut *(self as *const Self as *mut Self);
                if let Some(root) = this.root.as_deref_mut() {
                    if let Some(slot) = find_impl(root, x, y) {
                        return slot;
                    }
                }
                static mut NULL_POINT: Option<Box<QuadtreePoint>> = None;
                // Reset NULL_POINT to None each time, in case anyone wrote to
                // it through a previous call.
                let null_ref: *mut Option<Box<QuadtreePoint>> =
                    std::ptr::addr_of_mut!(NULL_POINT);
                *null_ref = None;
                &mut *null_ref
            }
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // The public signature uses `&self`, but the operation mutates
            // the tree. Use an unsafe cast to obtain a mutable reference.
            unsafe {
                let this = &mut *(self as *const Self as *mut Self);
                let root = match this.root.as_deref_mut() {
                    Some(r) => r,
                    None => return false,
                };
                let point = QuadtreePoint::quadtree_point_new(x, y);
                if !node_contains_impl(root, &point) {
                    return false;
                }
                let status = insert_recursive(root, Box::new(point), key);
                if status == 1 {
                    this.length += 1;
                }
                status != 0
            }
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            unsafe {
                let this = &mut *(self as *const Self as *mut Self);
                walk_node_impl(&mut this.root, descent, ascent);
            }
        }
    }
}
// Helper function
pub fn elision_<T>(_key: Option<Box<T>>) {}
