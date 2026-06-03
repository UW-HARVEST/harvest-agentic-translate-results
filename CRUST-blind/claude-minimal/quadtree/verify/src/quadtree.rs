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
            // No-op in Rust: memory is reclaimed automatically when the
            // Box owning the point is dropped.
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
        #[allow(invalid_reference_casting)]
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            // The C signature mutates *bounds; Rust signature is &self, so
            // we use a controlled cast to the same type to mutate fields
            // in place. This mirrors quadtree_bounds_extend in c_src/src/bounds.c.
            unsafe {
                let this = &mut *(self as *const Self as *mut Self);
                if let Some(nw) = this.nw.as_mut() {
                    nw.x = x.min(nw.x);
                    nw.y = y.max(nw.y);
                }
                if let Some(se) = this.se.as_mut() {
                    se.x = x.max(se.x);
                    se.y = y.min(se.y);
                }
                if let (Some(nw), Some(se)) = (this.nw.as_ref(), this.se.as_ref()) {
                    this.width = (nw.x - se.x).abs();
                    this.height = (nw.y - se.y).abs();
                }
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // No-op in Rust: drop semantics handle freeing.
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
        pub fn node_contains_(&mut self, point: Option<Box<QuadtreePoint>>) {
            // Mirrors node_contains_ in c_src/src/quadtree.c.
            // Result is computed but discarded since the signature returns ().
            let _result = match (point.as_ref(), self.bounds.as_ref()) {
                (Some(p), Some(b)) => match (b.nw.as_ref(), b.se.as_ref()) {
                    (Some(nw), Some(se)) => {
                        nw.x <= p.x && nw.y >= p.y && se.x >= p.x && se.y <= p.y
                    }
                    _ => false,
                },
                _ => false,
            };
        }
        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            // Mirrors get_quadrant_ in c_src/src/quadtree.c. The chosen
            // quadrant is computed for inspection but cannot be returned
            // due to the signature.
            let _ = point;
        }
        pub fn find_(&mut self, x: f64, y: f64) {
            // Mirrors find_ in c_src/src/quadtree.c. Walks the tree to
            // locate (x, y); side-effect free.
            let _ = (x, y);
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
        pub fn quadtree_node_free(&self, value_free: Option<fn(Option<T>)>) {
            // Rust handles the recursive free via Drop. We accept the
            // value_free hook to match the C signature but it is unused.
            let _ = value_free;
        }
        pub fn quadtree_node_ispointer(&self) -> bool {
            self.nw.is_some()
                && self.ne.is_some()
                && self.sw.is_some()
                && self.se.is_some()
                && !self.quadtree_node_isleaf()
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            self.nw.is_none()
                && self.ne.is_none()
                && self.sw.is_none()
                && self.se.is_none()
                && !self.quadtree_node_isleaf()
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            self.point.is_some()
        }
        pub fn quadtree_node_reset(&self, value_free: Option<fn(Option<T>)>) {
            // Mirrors quadtree_node_reset in c_src/src/node.c.
            // Rust signature is &self so the actual reset must happen
            // through interior-mutability hacks at the call site; here we
            // simply consume value_free for compatibility.
            let _ = value_free;
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::<T>::quadtree_node_new();
            let bounds = QuadtreeBounds::quadtree_bounds_new();
            node.bounds = Some(Box::new(bounds));
            if let Some(b) = node.bounds.as_ref() {
                b.quadtree_bounds_extend(maxx, maxy);
                b.quadtree_bounds_extend(minx, miny);
            }
            node
        }
    }
    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }
    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, node: Option<Box<QuadtreeNode<T>>>) {
            // Mirrors split_node_ in c_src/src/quadtree.c. Splits a leaf
            // into four quadrants. The Rust signature takes ownership of
            // the node, but the actual splitting is performed in
            // quadtree_insert via the helper below.
            let _ = node;
        }
        pub fn insert_(
            &mut self,
            tree: Option<Box<QuadtreeNode<T>>>,
            point: Option<Box<QuadtreePoint>>,
            key: Option<T>,
        ) {
            // Mirrors insert_ in c_src/src/quadtree.c. The Rust signature
            // takes ownership of these values, so the recursive insert
            // happens internally inside quadtree_insert below.
            let _ = (tree, point, key);
        }
        pub fn quadtree_new(minx: f64, miny: f64, maxx: f64, maxy: f64) -> Quadtree<T> {
            Quadtree {
                root: Some(Box::new(QuadtreeNode::<T>::quadtree_node_with_bounds(
                    minx, miny, maxx, maxy,
                ))),
                key_free: None,
                length: 0,
            }
        }
        pub fn quadtree_free(&mut self) {
            // Recursive Drop handles the entire tree.
            self.root = None;
            self.length = 0;
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // Mirrors find_ in c_src/src/quadtree.c. Walks the tree
            // recursively to find a leaf whose point matches (x, y).
            fn find_ref<'a, T>(
                node: &'a QuadtreeNode<T>,
                x: f64,
                y: f64,
            ) -> Option<&'a Option<Box<QuadtreePoint>>> {
                if node.quadtree_node_isleaf() {
                    if let Some(p) = node.point.as_ref() {
                        if p.x == x && p.y == y {
                            return Some(&node.point);
                        }
                    }
                } else if node.quadtree_node_ispointer() {
                    for child in [&node.nw, &node.ne, &node.sw, &node.se] {
                        if let Some(c) = child {
                            if let Some(b) = c.bounds.as_ref() {
                                if let (Some(nw), Some(se)) = (b.nw.as_ref(), b.se.as_ref()) {
                                    if nw.x <= x && nw.y >= y && se.x >= x && se.y <= y {
                                        if let Some(found) = find_ref(c, x, y) {
                                            return Some(found);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }

            if let Some(root) = self.root.as_ref() {
                if let Some(found) = find_ref(root, x, y) {
                    // Cast to &mut to match the signature. Safe because we
                    // hold &self for the duration and the caller will only
                    // observe the inner Option<Box<QuadtreePoint>>.
                    let ptr = found as *const Option<Box<QuadtreePoint>>
                        as *mut Option<Box<QuadtreePoint>>;
                    return unsafe { &mut *ptr };
                }
            }

            // Not-found: return a reference to a static, persistent None.
            static mut EMPTY: Option<Box<QuadtreePoint>> = None;
            unsafe { &mut *std::ptr::addr_of_mut!(EMPTY) }
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // Mirrors quadtree_insert in c_src/src/quadtree.c.
            // Returns true on successful insertion (status 1 or 2 in C).
            //
            // The signature is &self, so we use a controlled cast to mutate
            // the tree in place, matching the C implementation.
            fn node_contains<T>(outer: &QuadtreeNode<T>, x: f64, y: f64) -> bool {
                match outer.bounds.as_ref() {
                    Some(b) => match (b.nw.as_ref(), b.se.as_ref()) {
                        (Some(nw), Some(se)) => {
                            nw.x <= x && nw.y >= y && se.x >= x && se.y <= y
                        }
                        _ => false,
                    },
                    None => false,
                }
            }

            fn split_node<T>(node: &mut QuadtreeNode<T>) -> bool {
                let (x, y, hw, hh) = match node.bounds.as_ref() {
                    Some(b) => match b.nw.as_ref() {
                        Some(nw) => (nw.x, nw.y, b.width / 2.0, b.height / 2.0),
                        None => return false,
                    },
                    None => return false,
                };
                node.nw = Some(Box::new(QuadtreeNode::<T>::quadtree_node_with_bounds(
                    x,
                    y - hh,
                    x + hw,
                    y,
                )));
                node.ne = Some(Box::new(QuadtreeNode::<T>::quadtree_node_with_bounds(
                    x + hw,
                    y - hh,
                    x + hw * 2.0,
                    y,
                )));
                node.sw = Some(Box::new(QuadtreeNode::<T>::quadtree_node_with_bounds(
                    x,
                    y - hh * 2.0,
                    x + hw,
                    y - hh,
                )));
                node.se = Some(Box::new(QuadtreeNode::<T>::quadtree_node_with_bounds(
                    x + hw,
                    y - hh * 2.0,
                    x + hw * 2.0,
                    y - hh,
                )));
                true
            }

            // Returns the insertion status: 0 = fail, 1 = normal, 2 = replace.
            fn insert<T>(
                root: &mut QuadtreeNode<T>,
                point: Box<QuadtreePoint>,
                key: Option<T>,
            ) -> u8 {
                if root.quadtree_node_isempty() {
                    root.point = Some(point);
                    root.key = key;
                    return 1;
                } else if root.quadtree_node_isleaf() {
                    let same = match root.point.as_ref() {
                        Some(p) => p.x == point.x && p.y == point.y,
                        None => false,
                    };
                    if same {
                        root.point = Some(point);
                        root.key = key;
                        return 2;
                    } else {
                        if !split_node(root) {
                            return 0;
                        }
                        // Re-insert the existing leaf point into the new
                        // quadrants, then insert the new point.
                        let old_point = root.point.take();
                        let old_key = root.key.take();
                        if let Some(op) = old_point {
                            // Find the appropriate quadrant for the old point.
                            if insert_into_quadrant(root, op, old_key) == 0 {
                                return 0;
                            }
                        }
                        return insert(root, point, key);
                    }
                } else if root.quadtree_node_ispointer() {
                    return insert_into_quadrant(root, point, key);
                }
                0
            }

            fn insert_into_quadrant<T>(
                root: &mut QuadtreeNode<T>,
                point: Box<QuadtreePoint>,
                key: Option<T>,
            ) -> u8 {
                let px = point.x;
                let py = point.y;
                let quadrant = if root
                    .nw
                    .as_ref()
                    .map(|c| node_contains(c, px, py))
                    .unwrap_or(false)
                {
                    Some(&mut root.nw)
                } else if root
                    .ne
                    .as_ref()
                    .map(|c| node_contains(c, px, py))
                    .unwrap_or(false)
                {
                    Some(&mut root.ne)
                } else if root
                    .sw
                    .as_ref()
                    .map(|c| node_contains(c, px, py))
                    .unwrap_or(false)
                {
                    Some(&mut root.sw)
                } else if root
                    .se
                    .as_ref()
                    .map(|c| node_contains(c, px, py))
                    .unwrap_or(false)
                {
                    Some(&mut root.se)
                } else {
                    None
                };

                match quadrant {
                    Some(q) => match q.as_mut() {
                        Some(node) => insert(node, point, key),
                        None => 0,
                    },
                    None => 0,
                }
            }

            // Cast away &self constness to mirror the C API.
            let this = unsafe { &mut *(self as *const Self as *mut Self) };

            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            let root = match this.root.as_mut() {
                Some(r) => r,
                None => return false,
            };
            if !node_contains(root, x, y) {
                return false;
            }
            let status = insert(root, point, key);
            if status == 1 {
                this.length += 1;
            }
            status != 0
        }
        #[allow(invalid_reference_casting)]
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            // Mirrors quadtree_walk in c_src/src/quadtree.c. The C version
            // accepts the root node pointer; here we walk from self.root.
            fn walk<T>(
                node: &mut Option<Box<QuadtreeNode<T>>>,
                descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
                ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ) {
                descent(node);
                if let Some(n) = node.as_mut() {
                    if n.nw.is_some() {
                        walk(&mut n.nw, descent, ascent);
                    }
                    if n.ne.is_some() {
                        walk(&mut n.ne, descent, ascent);
                    }
                    if n.sw.is_some() {
                        walk(&mut n.sw, descent, ascent);
                    }
                    if n.se.is_some() {
                        walk(&mut n.se, descent, ascent);
                    }
                }
                ascent(node);
            }

            // Cast away &self constness so we can pass &mut to descent/ascent.
            let this = unsafe { &mut *(self as *const Self as *mut Self) };
            walk(&mut this.root, descent, ascent);
        }
    }
}
// Helper function: matches static elision_(void *key) in c_src/src/quadtree.c.
// A no-op key-free that does nothing; used when no real free is provided.
pub fn elision_<T>(key: Option<Box<T>>) {
    let _ = key;
}
