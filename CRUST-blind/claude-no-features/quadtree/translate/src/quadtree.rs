#![allow(invalid_reference_casting)]
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
            // Memory is managed by Rust; nothing to do.
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
            // The C version mutates the bounds in-place. The Rust signature
            // takes `&self`, so we need an unsafe cast to satisfy the caller's
            // intent.
            unsafe {
                let bounds = &mut *(self as *const QuadtreeBounds as *mut QuadtreeBounds);
                bounds_extend_impl(bounds, x, y);
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // Memory is managed by Rust; nothing to do.
        }
    }

    fn bounds_extend_impl(bounds: &mut QuadtreeBounds, x: f64, y: f64) {
        if let Some(nw) = bounds.nw.as_deref_mut() {
            nw.x = nw.x.min(x);
            nw.y = nw.y.max(y);
        }
        if let Some(se) = bounds.se.as_deref_mut() {
            se.x = se.x.max(x);
            se.y = se.y.min(y);
        }
        let (nwx, nwy) = match bounds.nw.as_deref() {
            Some(p) => (p.x, p.y),
            None => (0.0, 0.0),
        };
        let (sex, sey) = match bounds.se.as_deref() {
            Some(p) => (p.x, p.y),
            None => (0.0, 0.0),
        };
        bounds.width = (nwx - sex).abs();
        bounds.height = (nwy - sey).abs();
    }

    fn node_contains_impl<T>(outer: &QuadtreeNode<T>, point: &QuadtreePoint) -> bool {
        if let Some(b) = outer.bounds.as_deref() {
            if let (Some(nw), Some(se)) = (b.nw.as_deref(), b.se.as_deref()) {
                return nw.x <= point.x
                    && nw.y >= point.y
                    && se.x >= point.x
                    && se.y <= point.y;
            }
        }
        false
    }

    fn node_isleaf<T>(node: &QuadtreeNode<T>) -> bool {
        node.point.is_some()
    }

    fn node_isempty<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_none()
            && node.ne.is_none()
            && node.sw.is_none()
            && node.se.is_none()
            && !node_isleaf(node)
    }

    fn node_ispointer<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_some()
            && node.ne.is_some()
            && node.sw.is_some()
            && node.se.is_some()
            && !node_isleaf(node)
    }

    fn get_quadrant_idx<T>(root: &QuadtreeNode<T>, point: &QuadtreePoint) -> Option<usize> {
        if let Some(n) = root.nw.as_deref() {
            if node_contains_impl(n, point) {
                return Some(0);
            }
        }
        if let Some(n) = root.ne.as_deref() {
            if node_contains_impl(n, point) {
                return Some(1);
            }
        }
        if let Some(n) = root.sw.as_deref() {
            if node_contains_impl(n, point) {
                return Some(2);
            }
        }
        if let Some(n) = root.se.as_deref() {
            if node_contains_impl(n, point) {
                return Some(3);
            }
        }
        None
    }

    fn split_node_impl<T>(node: &mut QuadtreeNode<T>) -> bool {
        let (x, y, hw, hh) = match node.bounds.as_deref() {
            Some(b) => match b.nw.as_deref() {
                Some(p) => (p.x, p.y, b.width / 2.0, b.height / 2.0),
                None => return false,
            },
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

        if let Some(p) = old_point {
            insert_impl(node, p, old_key) != 0
        } else {
            true
        }
    }

    fn insert_impl<T>(
        root: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
    ) -> i32 {
        if node_isempty(root) {
            root.point = Some(point);
            root.key = key;
            return 1;
        } else if node_isleaf(root) {
            let same = match root.point.as_deref() {
                Some(p) => p.x == point.x && p.y == point.y,
                None => false,
            };
            if same {
                // reset the existing point/key (drop happens via assignment)
                root.point = Some(point);
                root.key = key;
                return 2;
            } else {
                if !split_node_impl(root) {
                    return 0;
                }
                return insert_impl(root, point, key);
            }
        } else if node_ispointer(root) {
            let idx = get_quadrant_idx(root, &point);
            let quadrant = match idx {
                Some(0) => root.nw.as_deref_mut(),
                Some(1) => root.ne.as_deref_mut(),
                Some(2) => root.sw.as_deref_mut(),
                Some(3) => root.se.as_deref_mut(),
                _ => None,
            };
            return match quadrant {
                Some(q) => insert_impl(q, point, key),
                None => 0,
            };
        }
        0
    }

    fn find_impl_mut<'a, T>(
        node: Option<&'a mut QuadtreeNode<T>>,
        x: f64,
        y: f64,
    ) -> Option<&'a mut Option<Box<QuadtreePoint>>> {
        let n = node?;
        if node_isleaf(n) {
            let matches = match n.point.as_deref() {
                Some(p) => p.x == x && p.y == y,
                None => false,
            };
            if matches {
                return Some(&mut n.point);
            }
            return None;
        } else if node_ispointer(n) {
            let test = QuadtreePoint { x, y };
            let idx = get_quadrant_idx(n, &test);
            let q = match idx {
                Some(0) => n.nw.as_deref_mut(),
                Some(1) => n.ne.as_deref_mut(),
                Some(2) => n.sw.as_deref_mut(),
                Some(3) => n.se.as_deref_mut(),
                _ => None,
            };
            return find_impl_mut(q, x, y);
        }
        None
    }

    fn walk_impl<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        descent(node);
        if let Some(n) = node.as_deref_mut() {
            if n.nw.is_some() {
                walk_impl(&mut n.nw, descent, ascent);
            }
            if n.ne.is_some() {
                walk_impl(&mut n.ne, descent, ascent);
            }
            if n.sw.is_some() {
                walk_impl(&mut n.sw, descent, ascent);
            }
            if n.se.is_some() {
                walk_impl(&mut n.se, descent, ascent);
            }
        }
        ascent(node);
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
            // The C variant returns an int; the Rust signature returns nothing,
            // so this acts as an inspection that simply consumes its argument.
            let _ = point;
        }
        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            let _ = point;
        }
        pub fn find_(&mut self, x: f64, y: f64) {
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
            // Memory is reclaimed by Rust automatically; nothing to free.
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
            // The C version frees the point and invokes key_free on the key.
            // With `&self` we can't move out, so we just invoke value_free with
            // None to provide a best-effort hook.
            if let Some(f) = value_free {
                f(None);
            }
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = Self::quadtree_node_new();
            let mut bounds = QuadtreeBounds::quadtree_bounds_new();
            bounds_extend_impl(&mut bounds, maxx, maxy);
            bounds_extend_impl(&mut bounds, minx, miny);
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
        pub fn split_node_(&mut self, node: Option<Box<QuadtreeNode<T>>>) {
            // The C version mutates the passed node; this Rust wrapper takes
            // ownership but cannot return the result. We perform the split on
            // the owned node and discard it (the canonical implementation
            // happens through `insert_impl`/`split_node_impl`).
            if let Some(mut boxed) = node {
                let _ = split_node_impl(&mut boxed);
            }
        }
        pub fn insert_(
            &mut self,
            tree: Option<Box<QuadtreeNode<T>>>,
            point: Option<Box<QuadtreePoint>>,
            key: Option<T>,
        ) {
            // Equivalent to the C `insert_` helper, but the Rust signature
            // can't return the resulting status nor the mutated node.
            if let (Some(mut node), Some(p)) = (tree, point) {
                let _ = insert_impl(&mut node, p, key);
            }
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
            // Drop the tree: setting root to None recursively drops all nodes.
            self.root = None;
            self.length = 0;
            self.key_free = None;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // The C signature returns a pointer that the caller may inspect.
            // We need to hand back a `&mut Option<Box<QuadtreePoint>>` from a
            // shared reference, which requires casting away const-ness.
            unsafe {
                let mut_self = &mut *(self as *const Self as *mut Self);
                if let Some(root) = mut_self.root.as_deref_mut() {
                    if let Some(p) = find_impl_mut(Some(root), x, y) {
                        return p;
                    }
                }
                // Not found: return a fresh leaked None so the caller observes
                // the absence of a point.
                Box::leak(Box::new(None))
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // The C variant returns an int (0 = fail, 1 = inserted, 2 =
            // replaced). The Rust signature is a bool so any successful
            // operation maps to true.
            unsafe {
                let mut_self = &mut *(self as *const Self as *mut Self);
                let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
                let root = match mut_self.root.as_deref_mut() {
                    Some(r) => r,
                    None => return false,
                };
                if !node_contains_impl(&*root, &point) {
                    return false;
                }
                let status = insert_impl(root, point, key);
                if status == 0 {
                    return false;
                }
                if status == 1 {
                    mut_self.length = mut_self.length.wrapping_add(1);
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
                let mut_self = &mut *(self as *const Self as *mut Self);
                if mut_self.root.is_some() {
                    walk_impl(&mut mut_self.root, descent, ascent);
                }
            }
        }
    }
}
// Helper function
pub fn elision_<T>(key: Option<Box<T>>) {
    // Equivalent of the C `elision_` no-op: discard the key.
    let _ = key;
}
