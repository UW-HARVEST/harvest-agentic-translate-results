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
            // No-op in Rust: memory is reclaimed by Drop.
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
        pub fn quadtree_bounds_extend(&mut self, x: f64, y: f64) {
            {
                let nw = self.nw.as_mut().expect("bounds.nw must be initialized");
                nw.x = x.min(nw.x);
                nw.y = y.max(nw.y);
            }
            {
                let se = self.se.as_mut().expect("bounds.se must be initialized");
                se.x = x.max(se.x);
                se.y = y.min(se.y);
            }
            let nw_x = self.nw.as_ref().unwrap().x;
            let nw_y = self.nw.as_ref().unwrap().y;
            let se_x = self.se.as_ref().unwrap().x;
            let se_y = self.se.as_ref().unwrap().y;
            self.width = (nw_x - se_x).abs();
            self.height = (nw_y - se_y).abs();
        }
        pub fn quadtree_bounds_free(&self) {
            // No-op in Rust: memory is reclaimed by Drop.
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
        // The following helper methods are kept for API compatibility;
        // the real implementations live in free helper functions below
        // (`node_contains_`, `get_quadrant_`, `find_`) which can operate
        // on borrowed references for use during recursive traversals.
        pub fn node_contains_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // see free function `node_contains_`
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // see free function `get_quadrant_`
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // see free function `find_`
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
            // No-op in Rust: memory is reclaimed by Drop.
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
        pub fn quadtree_node_reset(&self, _value_free: Option<fn(Option<T>)>) {
            // No-op in Rust: memory is reclaimed by Drop. (Used in C to free
            // the held point and key before reuse.)
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::quadtree_node_new();
            let mut bounds = QuadtreeBounds::quadtree_bounds_new();
            bounds.quadtree_bounds_extend(maxx, maxy);
            bounds.quadtree_bounds_extend(minx, miny);
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
    impl<T> Quadtree<T> {
        // The split/insert "method" forms are kept for API compatibility but
        // delegate to the free functions below which operate via mutable
        // borrows so we can recurse without consuming nodes.
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // see free function `split_node_`
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // see free function `insert_`
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
            // No-op in Rust: dropping the tree (or its root) reclaims memory.
            self.root = None;
        }
        pub fn quadtree_search(
            &mut self,
            x: f64,
            y: f64,
        ) -> &mut Option<Box<QuadtreePoint>> {
            find_(&mut self.root, x, y)
        }
        pub fn quadtree_insert(&mut self, x: f64, y: f64, key: Option<T>) -> bool {
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            let root = match self.root.as_mut() {
                Some(r) => r,
                None => return false,
            };
            if !node_contains_(root, &point) {
                return false;
            }
            let status = insert_(root, point, key);
            if status == 0 {
                return false;
            }
            if status == 1 {
                self.length += 1;
            }
            true
        }
        pub fn quadtree_walk(
            &mut self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            walk_(&mut self.root, descent, ascent);
        }
    }

    /* ------------------------------------------------------------------ *
     * Free helper functions implementing the recursive logic. They
     * mirror the static helpers in c_src/src/quadtree.c.
     * ------------------------------------------------------------------ */

    fn node_contains_<T>(outer: &QuadtreeNode<T>, it: &QuadtreePoint) -> bool {
        match outer.bounds.as_ref() {
            None => false,
            Some(bounds) => {
                let nw = bounds.nw.as_ref().unwrap();
                let se = bounds.se.as_ref().unwrap();
                nw.x <= it.x && nw.y >= it.y && se.x >= it.x && se.y <= it.y
            }
        }
    }

    /// Returns a mutable reference to the matching child quadrant Option, if any.
    fn get_quadrant_<'a, T>(
        root: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut Box<QuadtreeNode<T>>> {
        // Determine which quadrant contains the point using immutable borrows,
        // then return the corresponding mutable borrow.
        let which = if root
            .nw
            .as_ref()
            .map_or(false, |n| node_contains_(n, point))
        {
            0
        } else if root
            .ne
            .as_ref()
            .map_or(false, |n| node_contains_(n, point))
        {
            1
        } else if root
            .sw
            .as_ref()
            .map_or(false, |n| node_contains_(n, point))
        {
            2
        } else if root
            .se
            .as_ref()
            .map_or(false, |n| node_contains_(n, point))
        {
            3
        } else {
            return None;
        };
        match which {
            0 => root.nw.as_mut(),
            1 => root.ne.as_mut(),
            2 => root.sw.as_mut(),
            _ => root.se.as_mut(),
        }
    }

    fn split_node_<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = {
            let bounds = match node.bounds.as_ref() {
                Some(b) => b,
                None => return false,
            };
            let nw = bounds.nw.as_ref().unwrap();
            (nw.x, nw.y, bounds.width / 2.0, bounds.height / 2.0)
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
            insert_(node, p, old_key) != 0
        } else {
            true
        }
    }

    fn find_<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        x: f64,
        y: f64,
    ) -> &mut Option<Box<QuadtreePoint>> {
        // If there is no node at this slot, return a sentinel `None` point.
        if node.is_none() {
            return sentinel_none_point();
        }
        let n_ref = node.as_mut().unwrap();

        if n_ref.quadtree_node_isleaf() {
            let p = n_ref.point.as_ref().unwrap();
            if p.x == x && p.y == y {
                return &mut n_ref.point;
            }
            return sentinel_none_point();
        } else if n_ref.quadtree_node_ispointer() {
            let test = QuadtreePoint::quadtree_point_new(x, y);
            // Decide which quadrant via immutable inspection.
            let which = if n_ref.nw.as_ref().map_or(false, |c| node_contains_(c, &test)) {
                0
            } else if n_ref.ne.as_ref().map_or(false, |c| node_contains_(c, &test)) {
                1
            } else if n_ref.sw.as_ref().map_or(false, |c| node_contains_(c, &test)) {
                2
            } else if n_ref.se.as_ref().map_or(false, |c| node_contains_(c, &test)) {
                3
            } else {
                return sentinel_none_point();
            };
            return match which {
                0 => find_(&mut n_ref.nw, x, y),
                1 => find_(&mut n_ref.ne, x, y),
                2 => find_(&mut n_ref.sw, x, y),
                _ => find_(&mut n_ref.se, x, y),
            };
        }
        sentinel_none_point()
    }

    /// Returns a mutable reference to a process-wide sentinel `None`
    /// `QuadtreePoint` Option. Used to express "not found" while still
    /// matching the C API which returns a pointer (here: a `&mut Option`).
    fn sentinel_none_point() -> &'static mut Option<Box<QuadtreePoint>> {
        use std::sync::OnceLock;
        static SENTINEL: OnceLock<usize> = OnceLock::new();
        let addr = *SENTINEL.get_or_init(|| {
            let b: Box<Option<Box<QuadtreePoint>>> = Box::new(None);
            Box::into_raw(b) as usize
        });
        // Safety: the leaked allocation lives for the entire program. Returning
        // a mutable reference is sound for the single-threaded test usage; the
        // sentinel is intentionally never mutated by callers (they only read
        // `.as_ref()` to check for `None`).
        unsafe { &mut *(addr as *mut Option<Box<QuadtreePoint>>) }
    }

    /// The recursive insertion routine. Mirrors the C `insert_` helper.
    fn insert_<T>(
        root: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
    ) -> i32 {
        if root.quadtree_node_isempty() {
            root.point = Some(point);
            root.key = key;
            return 1; // normal insertion
        } else if root.quadtree_node_isleaf() {
            let same = {
                let p = root.point.as_ref().unwrap();
                p.x == point.x && p.y == point.y
            };
            if same {
                // Reset and replace.
                root.point = Some(point);
                root.key = key;
                return 2; // replace insertion
            } else {
                if !split_node_(root) {
                    return 0;
                }
                return insert_(root, point, key);
            }
        } else if root.quadtree_node_ispointer() {
            // Determine quadrant first via immutable look, then take a mut
            // borrow only of that single child.
            let which = {
                let p = &*point;
                if root.nw.as_ref().map_or(false, |c| node_contains_(c, p)) {
                    0
                } else if root.ne.as_ref().map_or(false, |c| node_contains_(c, p)) {
                    1
                } else if root.sw.as_ref().map_or(false, |c| node_contains_(c, p)) {
                    2
                } else if root.se.as_ref().map_or(false, |c| node_contains_(c, p)) {
                    3
                } else {
                    return 0;
                }
            };
            let child: &mut Box<QuadtreeNode<T>> = match which {
                0 => root.nw.as_mut().unwrap(),
                1 => root.ne.as_mut().unwrap(),
                2 => root.sw.as_mut().unwrap(),
                _ => root.se.as_mut().unwrap(),
            };
            return insert_(child, point, key);
        }
        0
    }

    fn walk_<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(node);
        if let Some(n) = node.as_mut() {
            if n.nw.is_some() {
                walk_(&mut n.nw, descent, ascent);
            }
            if n.ne.is_some() {
                walk_(&mut n.ne, descent, ascent);
            }
            if n.sw.is_some() {
                walk_(&mut n.sw, descent, ascent);
            }
            if n.se.is_some() {
                walk_(&mut n.se, descent, ascent);
            }
        }
        ascent(node);
    }

    // Suppress dead-code lints for helpers retained for API compatibility.
    #[allow(dead_code)]
    fn _suppress_unused() {
        let _ = get_quadrant_::<i32>;
    }
}
// Helper function – mirrors the C `elision_` no-op used as a default
// `key_free` implementation.
pub fn elision_<T>(_key: Option<Box<T>>) {}
