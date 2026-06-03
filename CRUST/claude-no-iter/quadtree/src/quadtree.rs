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
            // In Rust, memory is freed automatically when the value goes out of
            // scope. This function is a no-op kept for API compatibility with
            // the C implementation.
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
            // The C function modifies the bounds in place. The Rust signature
            // takes `&self`, so we use a pointer cast to mutate. This is the
            // simplest way to mirror C semantics while honoring the required
            // signature.
            unsafe {
                let s = self as *const Self as *mut Self;
                if let Some(nw) = (*s).nw.as_deref_mut() {
                    nw.x = x.min(nw.x);
                    nw.y = y.max(nw.y);
                }
                if let Some(se) = (*s).se.as_deref_mut() {
                    se.x = x.max(se.x);
                    se.y = y.min(se.y);
                }
                let nwx = (*s).nw.as_deref().map(|p| p.x).unwrap_or(0.0);
                let nwy = (*s).nw.as_deref().map(|p| p.y).unwrap_or(0.0);
                let sex = (*s).se.as_deref().map(|p| p.x).unwrap_or(0.0);
                let sey = (*s).se.as_deref().map(|p| p.y).unwrap_or(0.0);
                (*s).width = (nwx - sex).abs();
                (*s).height = (nwy - sey).abs();
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // Memory is freed automatically.
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
            // The signature does not allow returning a value, so the actual
            // containment check is implemented in `node_contains` below. The
            // passed-in point is consumed and dropped.
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // The actual quadrant lookup is done by `get_quadrant_mut`.
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // The actual find logic is implemented inside `Quadtree::quadtree_search`.
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
            // Memory is freed automatically when this node goes out of scope.
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
            // Reset the node by clearing its point and key.
            unsafe {
                let s = self as *const Self as *mut Self;
                (*s).point = None;
                (*s).key = None;
            }
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node: QuadtreeNode<T> = Self::quadtree_node_new();
            let bounds = QuadtreeBounds::quadtree_bounds_new();
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
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // The actual split logic is in the `do_split` helper. The passed-in
            // node is consumed and dropped.
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // The actual recursive insertion logic is in `do_insert`.
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
            // Drop the entire tree. Rust frees nested boxes automatically.
            self.root = None;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            unsafe {
                let s = self as *const Self as *mut Self;
                if let Some(root) = (*s).root.as_deref_mut() {
                    if let Some(found) = find_point_mut(root, x, y) {
                        return &mut *found;
                    }
                }
                empty_point_ref()
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            let point = QuadtreePoint::quadtree_point_new(x, y);
            unsafe {
                let s = self as *const Self as *mut Self;
                let root = match (*s).root.as_deref_mut() {
                    Some(r) => r,
                    None => return false,
                };
                if !node_contains(root, &point) {
                    return false;
                }
                let status = do_insert(root, point, key);
                if status == 0 {
                    return false;
                }
                if status == 1 {
                    (*s).length += 1;
                }
                true
            }
        }
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            unsafe {
                let s = self as *const Self as *mut Self;
                walk_node(&mut (*s).root, descent, ascent);
            }
        }
    }

    /// Returns a mutable reference to a process-wide `None` slot used by
    /// `quadtree_search` to satisfy its return type when no match is found.
    fn empty_point_ref() -> &'static mut Option<Box<QuadtreePoint>> {
        static mut EMPTY: Option<Box<QuadtreePoint>> = None;
        unsafe {
            // Make sure the slot is reset every time we look it up; callers
            // shouldn't rely on a stale value from a previous "not found".
            EMPTY = None;
            &mut *std::ptr::addr_of_mut!(EMPTY)
        }
    }

    /// Recursively walk the tree, invoking `descent` before recursing and
    /// `ascent` afterwards.
    fn walk_node<T>(
        node_opt: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(node_opt);
        if let Some(node) = node_opt.as_deref_mut() {
            if node.nw.is_some() {
                walk_node(&mut node.nw, descent, ascent);
            }
            if node.ne.is_some() {
                walk_node(&mut node.ne, descent, ascent);
            }
            if node.sw.is_some() {
                walk_node(&mut node.sw, descent, ascent);
            }
            if node.se.is_some() {
                walk_node(&mut node.se, descent, ascent);
            }
        }
        ascent(node_opt);
    }

    /// Returns true when the given point lies within the bounds of `outer`.
    fn node_contains<T>(outer: &QuadtreeNode<T>, point: &QuadtreePoint) -> bool {
        match outer.bounds.as_deref() {
            Some(bounds) => match (bounds.nw.as_deref(), bounds.se.as_deref()) {
                (Some(nw), Some(se)) => {
                    nw.x <= point.x && nw.y >= point.y && se.x >= point.x && se.y <= point.y
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Returns a mutable reference to the child quadrant containing `point`,
    /// or `None` when the point lies outside every quadrant.
    fn get_quadrant_mut<'a, T>(
        root: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut QuadtreeNode<T>> {
        let in_nw = root
            .nw
            .as_deref()
            .map(|n| node_contains(n, point))
            .unwrap_or(false);
        let in_ne = root
            .ne
            .as_deref()
            .map(|n| node_contains(n, point))
            .unwrap_or(false);
        let in_sw = root
            .sw
            .as_deref()
            .map(|n| node_contains(n, point))
            .unwrap_or(false);
        let in_se = root
            .se
            .as_deref()
            .map(|n| node_contains(n, point))
            .unwrap_or(false);
        if in_nw {
            return root.nw.as_deref_mut();
        }
        if in_ne {
            return root.ne.as_deref_mut();
        }
        if in_sw {
            return root.sw.as_deref_mut();
        }
        if in_se {
            return root.se.as_deref_mut();
        }
        None
    }

    /// Splits a leaf node into four quadrant children. The caller is expected
    /// to re-insert the node's existing point afterwards.
    fn do_split<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, w, h) = match node.bounds.as_deref() {
            Some(b) => match b.nw.as_deref() {
                Some(p) => (p.x, p.y, b.width, b.height),
                None => return false,
            },
            None => return false,
        };
        let hw = w / 2.0;
        let hh = h / 2.0;
        let nw = QuadtreeNode::quadtree_node_with_bounds(x, y - hh, x + hw, y);
        let ne = QuadtreeNode::quadtree_node_with_bounds(x + hw, y - hh, x + hw * 2.0, y);
        let sw = QuadtreeNode::quadtree_node_with_bounds(x, y - hh * 2.0, x + hw, y - hh);
        let se = QuadtreeNode::quadtree_node_with_bounds(
            x + hw,
            y - hh * 2.0,
            x + hw * 2.0,
            y - hh,
        );
        node.nw = Some(Box::new(nw));
        node.ne = Some(Box::new(ne));
        node.sw = Some(Box::new(sw));
        node.se = Some(Box::new(se));
        true
    }

    /// Insert into the appropriate quadrant of `node`. Returns the same status
    /// codes as `do_insert`.
    fn do_insert_into_root<T>(
        node: &mut QuadtreeNode<T>,
        point: QuadtreePoint,
        key: Option<T>,
    ) -> i32 {
        match get_quadrant_mut(node, &point) {
            None => 0,
            Some(q) => do_insert(q, point, key),
        }
    }

    /// Recursive insertion. Returns:
    ///   0 = failure
    ///   1 = normal insertion
    ///   2 = replacement of an existing point
    fn do_insert<T>(node: &mut QuadtreeNode<T>, point: QuadtreePoint, key: Option<T>) -> i32 {
        let is_leaf = node.point.is_some();
        let has_children = node.nw.is_some()
            || node.ne.is_some()
            || node.sw.is_some()
            || node.se.is_some();

        if !is_leaf && !has_children {
            // empty node: store the point here.
            node.point = Some(Box::new(point));
            node.key = key;
            return 1;
        }
        if is_leaf {
            let (px, py) = {
                let p = node.point.as_deref().unwrap();
                (p.x, p.y)
            };
            if px == point.x && py == point.y {
                node.point = Some(Box::new(point));
                node.key = key;
                return 2;
            }
            if !do_split(node) {
                return 0;
            }
            // Move the existing leaf's point/key into the appropriate child.
            let old_point = node.point.take().map(|b| *b).unwrap();
            let old_key = node.key.take();
            let s1 = do_insert_into_root(node, old_point, old_key);
            if s1 == 0 {
                return 0;
            }
            return do_insert_into_root(node, point, key);
        }
        // pointer node (has children, no leaf point)
        do_insert_into_root(node, point, key)
    }

    /// Recursively search for a leaf with the given coordinates and return a
    /// raw pointer to the slot holding its point. Using a raw pointer keeps
    /// the recursion free of borrow-checker conflicts on partial subtrees.
    fn find_point_mut<T>(
        node: &mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> Option<*mut Option<Box<QuadtreePoint>>> {
        if node.point.is_some() {
            let (px, py) = {
                let p = node.point.as_deref().unwrap();
                (p.x, p.y)
            };
            if px == x && py == y {
                return Some(&mut node.point as *mut _);
            }
            return None;
        }
        if node.nw.is_some()
            && node.ne.is_some()
            && node.sw.is_some()
            && node.se.is_some()
        {
            let test = QuadtreePoint::quadtree_point_new(x, y);
            if let Some(child) = get_quadrant_mut(node, &test) {
                return find_point_mut(child, x, y);
            }
        }
        None
    }
}
// Helper function: matches the C `elision_` no-op used as a default for key
// destructors. Consumes the boxed key and drops it.
pub fn elision_<T>(_key: Option<Box<T>>) {}
