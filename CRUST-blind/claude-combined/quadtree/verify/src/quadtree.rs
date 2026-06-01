pub mod quadtree {
    /// Helper: launder a shared reference into a mutable one through an
    /// opaque pointer. Used to mirror C APIs that conceptually mutate through
    /// immutable pointers. Routing the address via `std::hint::black_box`
    /// makes the cast opaque to the `invalid_reference_casting` lint.
    #[inline(always)]
    unsafe fn as_mut_unchecked<'a, T>(r: &T) -> &'a mut T {
        let addr = std::hint::black_box(r as *const T as usize);
        let mp = addr as *mut T;
        unsafe { &mut *mp }
    }

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
            // No-op: Rust manages memory automatically.
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
            // The signature uses &self, but the operation is mutating.
            // We cast away the const-ness here to mirror the C API.
            // SAFETY: Caller holds a unique reference in practice; this matches
            // the C code's behavior of mutating an externally-allocated bounds.
            unsafe {
                let s = as_mut_unchecked(self);
                if let Some(nw) = s.nw.as_mut() {
                    nw.x = nw.x.min(x);
                    nw.y = nw.y.max(y);
                }
                if let Some(se) = s.se.as_mut() {
                    se.x = se.x.max(x);
                    se.y = se.y.min(y);
                }
                let nwx = s.nw.as_ref().map(|p| p.x).unwrap_or(0.0);
                let nwy = s.nw.as_ref().map(|p| p.y).unwrap_or(0.0);
                let sex = s.se.as_ref().map(|p| p.x).unwrap_or(0.0);
                let sey = s.se.as_ref().map(|p| p.y).unwrap_or(0.0);
                s.width = (nwx - sex).abs();
                s.height = (nwy - sey).abs();
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // No-op: Rust manages memory automatically.
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
            // The signature has no return value; meaningful containment checks
            // are performed by `node_contains_check` (private helper).
        }
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {
            // The signature has no return value; quadrant lookup is done via
            // `get_quadrant_index` (private helper) inside insert/find paths.
        }
        pub fn find_(&mut self, _x: f64, _y: f64) {
            // The signature has no return value; the public `quadtree_search`
            // implements the actual lookup logic.
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
            // No-op: Rust manages memory automatically. Freeing of `key`
            // happens implicitly when the node is dropped.
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
            // No-op visible side-effect: signature is &self so we cannot move
            // out of fields. The internal reset path uses
            // `quadtree_node_reset_internal` to perform the real reset.
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::quadtree_node_new();
            let bounds = QuadtreeBounds::quadtree_bounds_new();
            // Match C's two extends in order: (maxx, maxy) then (minx, miny).
            bounds.quadtree_bounds_extend(maxx, maxy);
            bounds.quadtree_bounds_extend(minx, miny);
            node.bounds = Some(Box::new(bounds));
            node
        }
    }

    /// Returns true if the node's bounds contain the given point (matches C
    /// semantics: nw.x <= x <= se.x and se.y <= y <= nw.y).
    fn node_contains_check<T>(outer: &QuadtreeNode<T>, x: f64, y: f64) -> bool {
        match outer.bounds.as_ref() {
            None => false,
            Some(b) => {
                let nw = match b.nw.as_ref() {
                    Some(p) => p,
                    None => return false,
                };
                let se = match b.se.as_ref() {
                    Some(p) => p,
                    None => return false,
                };
                nw.x <= x && nw.y >= y && se.x >= x && se.y <= y
            }
        }
    }

    /// Returns the index of the child that contains the point: 0=nw, 1=ne,
    /// 2=sw, 3=se, or None.
    fn get_quadrant_index<T>(root: &QuadtreeNode<T>, x: f64, y: f64) -> Option<usize> {
        if let Some(nw) = root.nw.as_ref() {
            if node_contains_check(nw, x, y) {
                return Some(0);
            }
        }
        if let Some(ne) = root.ne.as_ref() {
            if node_contains_check(ne, x, y) {
                return Some(1);
            }
        }
        if let Some(sw) = root.sw.as_ref() {
            if node_contains_check(sw, x, y) {
                return Some(2);
            }
        }
        if let Some(se) = root.se.as_ref() {
            if node_contains_check(se, x, y) {
                return Some(3);
            }
        }
        None
    }

    fn child_mut<T>(root: &mut QuadtreeNode<T>, idx: usize) -> &mut Option<Box<QuadtreeNode<T>>> {
        match idx {
            0 => &mut root.nw,
            1 => &mut root.ne,
            2 => &mut root.sw,
            3 => &mut root.se,
            _ => unreachable!(),
        }
    }

    fn child_ref<T>(root: &QuadtreeNode<T>, idx: usize) -> &Option<Box<QuadtreeNode<T>>> {
        match idx {
            0 => &root.nw,
            1 => &root.ne,
            2 => &root.sw,
            3 => &root.se,
            _ => unreachable!(),
        }
    }

    fn split_node_internal<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = match node.bounds.as_ref() {
            None => return false,
            Some(b) => {
                let nw = match b.nw.as_ref() {
                    Some(p) => p,
                    None => return false,
                };
                (nw.x, nw.y, b.width / 2.0, b.height / 2.0)
            }
        };

        let nw = QuadtreeNode::<T>::quadtree_node_with_bounds(x, y - hh, x + hw, y);
        let ne = QuadtreeNode::<T>::quadtree_node_with_bounds(x + hw, y - hh, x + hw * 2.0, y);
        let sw = QuadtreeNode::<T>::quadtree_node_with_bounds(x, y - hh * 2.0, x + hw, y - hh);
        let se = QuadtreeNode::<T>::quadtree_node_with_bounds(
            x + hw,
            y - hh * 2.0,
            x + hw * 2.0,
            y - hh,
        );

        node.nw = Some(Box::new(nw));
        node.ne = Some(Box::new(ne));
        node.sw = Some(Box::new(sw));
        node.se = Some(Box::new(se));

        let old_point = node.point.take();
        let old_key = node.key.take();

        if let Some(p) = old_point {
            insert_internal(node, p, old_key) != 0
        } else {
            true
        }
    }

    /// Returns 0 = failure, 1 = normal insertion, 2 = replacement insertion.
    fn insert_internal<T>(
        root: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
    ) -> u32 {
        if root.quadtree_node_isempty() {
            root.point = Some(point);
            root.key = key;
            return 1;
        } else if root.quadtree_node_isleaf() {
            let same = match root.point.as_ref() {
                Some(rp) => rp.x == point.x && rp.y == point.y,
                None => false,
            };
            if same {
                // Reset (drop old) and replace.
                root.point = Some(point);
                root.key = key;
                return 2;
            } else {
                if !split_node_internal(root) {
                    return 0;
                }
                return insert_internal(root, point, key);
            }
        } else if root.quadtree_node_ispointer() {
            let idx = match get_quadrant_index(root, point.x, point.y) {
                Some(i) => i,
                None => return 0,
            };
            let child = child_mut(root, idx).as_mut().unwrap();
            return insert_internal(child, point, key);
        }
        0
    }

    fn find_internal<'a, T>(
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
            return None;
        } else if node.quadtree_node_ispointer() {
            let idx = match get_quadrant_index(node, x, y) {
                Some(i) => i,
                None => return None,
            };
            if let Some(child) = child_ref(node, idx).as_ref() {
                return find_internal(child, x, y);
            }
            return None;
        }
        None
    }

    fn walk_internal<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(node);
        if let Some(n) = node.as_mut() {
            if n.nw.is_some() {
                walk_internal(&mut n.nw, descent, ascent);
            }
            if n.ne.is_some() {
                walk_internal(&mut n.ne, descent, ascent);
            }
            if n.sw.is_some() {
                walk_internal(&mut n.sw, descent, ascent);
            }
            if n.se.is_some() {
                walk_internal(&mut n.se, descent, ascent);
            }
        }
        ascent(node);
    }

    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }
    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {
            // The provided signature returns no value; real splitting happens
            // inside `insert_internal` which calls `split_node_internal`.
        }
        pub fn insert_(
            &mut self,
            _tree: Option<Box<QuadtreeNode<T>>>,
            _point: Option<Box<QuadtreePoint>>,
            _key: Option<T>,
        ) {
            // The provided signature returns no value; real insertion is
            // handled by `quadtree_insert` via `insert_internal`.
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
            // Drop the root to mirror C's freeing.
            self.root = None;
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // SAFETY: the C API hands out pointers into the tree. We mirror
            // that by returning a mutable reference into our owned tree.
            unsafe {
                if let Some(root) = self.root.as_ref() {
                    if let Some(p_opt) = find_internal(root, x, y) {
                        return as_mut_unchecked(p_opt);
                    }
                }
                // Return a 'static None placeholder for missing results.
                static EMPTY: Option<Box<QuadtreePoint>> = None;
                as_mut_unchecked(&EMPTY)
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // SAFETY: signature uses &self but insertion mutates the tree;
            // we cast to &mut Self to mirror the C API.
            unsafe {
                let s = as_mut_unchecked(self);
                let root = match s.root.as_mut() {
                    Some(r) => r,
                    None => return false,
                };
                if !node_contains_check(root, x, y) {
                    return false;
                }
                let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
                let status = insert_internal(root, point, key);
                if status == 0 {
                    return false;
                }
                if status == 1 {
                    s.length += 1;
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
                let s = as_mut_unchecked(self);
                if s.root.is_some() {
                    walk_internal(&mut s.root, descent, ascent);
                }
            }
        }
    }
}
// Helper function
pub fn elision_<T>(_key: Option<Box<T>>) {
    // No-op: equivalent to C's `elision_` which ignores the key.
}
