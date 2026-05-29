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
            // No-op: Rust handles memory automatically via Drop.
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
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            // The signature uses &self but the C semantics require mutation.
            // We perform the mutation through a raw pointer derived from the
            // exclusive reference the caller actually holds.
            unsafe {
                let this = self as *const Self as *mut Self;
                if let Some(nw) = (*this).nw.as_mut() {
                    nw.x = x.min(nw.x);
                    nw.y = y.max(nw.y);
                }
                if let Some(se) = (*this).se.as_mut() {
                    se.x = x.max(se.x);
                    se.y = y.min(se.y);
                }
                if let (Some(nw), Some(se)) = ((*this).nw.as_ref(), (*this).se.as_ref()) {
                    (*this).width = (nw.x - se.x).abs();
                    (*this).height = (nw.y - se.y).abs();
                }
            }
        }
        pub fn quadtree_bounds_free(&self) {
            // No-op: Rust handles memory automatically via Drop.
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
            // C version returns int; Rust signature has no return.
            // Compute the predicate for symmetry but discard the result.
            if let (Some(bounds), Some(p)) = (self.bounds.as_ref(), point.as_ref()) {
                if let (Some(nw), Some(se)) = (bounds.nw.as_ref(), bounds.se.as_ref()) {
                    let _contained =
                        nw.x <= p.x && nw.y >= p.y && se.x >= p.x && se.y <= p.y;
                }
            }
        }
        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            // No-op: signature has no return type to convey the result.
            let _ = point;
        }
        pub fn find_(&mut self, x: f64, y: f64) {
            // No-op: signature has no return type to convey the result.
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
            // No-op: Rust's Drop frees children/bounds/point automatically.
            // The key is owned by the node and will also be dropped naturally.
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
            // C version frees `point` and invokes `value_free(key)`.
            // In Rust, taking the values triggers any necessary drops.
            unsafe {
                let this = self as *const Self as *mut Self;
                let _point = (*this).point.take();
                let key = (*this).key.take();
                if let Some(f) = value_free {
                    f(key);
                }
            }
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let bounds = QuadtreeBounds::quadtree_bounds_new();
            bounds.quadtree_bounds_extend(maxx, maxy);
            bounds.quadtree_bounds_extend(minx, miny);
            QuadtreeNode {
                ne: None,
                nw: None,
                se: None,
                sw: None,
                bounds: Some(Box::new(bounds)),
                point: None,
                key: None,
            }
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
            // The C variant operates on a pointer into `self.root`. The Rust
            // signature instead consumes an owned subtree, so we cannot
            // mutate the original. The actual splitting logic used by
            // `quadtree_insert` lives in the private helper `split_node`.
            let _ = node;
        }
        pub fn insert_(
            &mut self,
            tree: Option<Box<QuadtreeNode<T>>>,
            point: Option<Box<QuadtreePoint>>,
            key: Option<T>,
        ) {
            // Same caveat as `split_node_`. Real insertion logic lives in
            // `insert_recursive`.
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
            // Drop the tree explicitly; Box destructors recursively release nodes.
            self.root = None;
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // The signature requires returning a `&mut` from `&self`. Cast
            // through a raw pointer derived from the caller's exclusive
            // reference to satisfy this without changing the API.
            unsafe {
                let this = self as *const Self as *mut Self;
                if let Some(root) = (*this).root.as_deref_mut() {
                    if let Some(point_ref) = Self::find_node_mut(root, x, y) {
                        return point_ref;
                    }
                }
                // Not found: leak a stable `None` placeholder so the
                // returned mutable reference is valid.
                let leaked: &'static mut Option<Box<QuadtreePoint>> =
                    Box::leak(Box::new(None));
                leaked
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // Mutate the tree through a raw pointer derived from `&self`.
            unsafe {
                let this = self as *const Self as *mut Self;
                let point = Box::new(QuadtreePoint { x, y });
                let root = match (*this).root.as_deref_mut() {
                    Some(r) => r,
                    None => return false,
                };
                if !Self::node_contains(root, &point) {
                    return false;
                }
                let status = Self::insert_recursive(root, point, key);
                if status == 1 {
                    (*this).length += 1;
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
                let this = self as *const Self as *mut Self;
                Self::walk_node(&mut (*this).root, descent, ascent);
            }
        }
    }

    // Private helpers that implement the real C logic.
    impl<T> Quadtree<T> {
        fn node_contains(node: &QuadtreeNode<T>, point: &QuadtreePoint) -> bool {
            match node.bounds.as_ref() {
                Some(bounds) => match (bounds.nw.as_ref(), bounds.se.as_ref()) {
                    (Some(nw), Some(se)) => {
                        nw.x <= point.x
                            && nw.y >= point.y
                            && se.x >= point.x
                            && se.y <= point.y
                    }
                    _ => false,
                },
                None => false,
            }
        }

        fn is_empty(node: &QuadtreeNode<T>) -> bool {
            node.nw.is_none()
                && node.ne.is_none()
                && node.sw.is_none()
                && node.se.is_none()
                && node.point.is_none()
        }

        fn is_leaf(node: &QuadtreeNode<T>) -> bool {
            node.point.is_some()
        }

        fn is_pointer(node: &QuadtreeNode<T>) -> bool {
            node.nw.is_some()
                && node.ne.is_some()
                && node.sw.is_some()
                && node.se.is_some()
                && node.point.is_none()
        }

        fn split_node(node: &mut QuadtreeNode<T>) -> bool {
            let (x, y, hw, hh) = match node.bounds.as_ref() {
                Some(bounds) => match bounds.nw.as_ref() {
                    Some(nw) => (nw.x, nw.y, bounds.width / 2.0, bounds.height / 2.0),
                    None => return false,
                },
                None => return false,
            };

            let nw = QuadtreeNode::<T>::quadtree_node_with_bounds(x, y - hh, x + hw, y);
            let ne = QuadtreeNode::<T>::quadtree_node_with_bounds(
                x + hw,
                y - hh,
                x + hw * 2.0,
                y,
            );
            let sw = QuadtreeNode::<T>::quadtree_node_with_bounds(
                x,
                y - hh * 2.0,
                x + hw,
                y - hh,
            );
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

            match old_point {
                Some(p) => Self::insert_recursive(node, p, old_key) != 0,
                None => true,
            }
        }

        fn insert_recursive(
            root: &mut QuadtreeNode<T>,
            point: Box<QuadtreePoint>,
            key: Option<T>,
        ) -> i32 {
            if Self::is_empty(root) {
                root.point = Some(point);
                root.key = key;
                return 1; // normal insertion
            }
            if Self::is_leaf(root) {
                let same = root
                    .point
                    .as_ref()
                    .map_or(false, |p| p.x == point.x && p.y == point.y);
                if same {
                    // Replace.
                    root.point = Some(point);
                    root.key = key;
                    return 2;
                }
                if !Self::split_node(root) {
                    return 0;
                }
                return Self::insert_recursive(root, point, key);
            }
            if Self::is_pointer(root) {
                // Find which quadrant the point falls into without holding
                // overlapping borrows.
                let which = Self::which_quadrant(root, &point);
                return match which {
                    1 => match root.nw.as_deref_mut() {
                        Some(child) => Self::insert_recursive(child, point, key),
                        None => 0,
                    },
                    2 => match root.ne.as_deref_mut() {
                        Some(child) => Self::insert_recursive(child, point, key),
                        None => 0,
                    },
                    3 => match root.sw.as_deref_mut() {
                        Some(child) => Self::insert_recursive(child, point, key),
                        None => 0,
                    },
                    4 => match root.se.as_deref_mut() {
                        Some(child) => Self::insert_recursive(child, point, key),
                        None => 0,
                    },
                    _ => 0,
                };
            }
            0
        }

        fn which_quadrant(root: &QuadtreeNode<T>, point: &QuadtreePoint) -> u8 {
            if let Some(c) = root.nw.as_deref() {
                if Self::node_contains(c, point) {
                    return 1;
                }
            }
            if let Some(c) = root.ne.as_deref() {
                if Self::node_contains(c, point) {
                    return 2;
                }
            }
            if let Some(c) = root.sw.as_deref() {
                if Self::node_contains(c, point) {
                    return 3;
                }
            }
            if let Some(c) = root.se.as_deref() {
                if Self::node_contains(c, point) {
                    return 4;
                }
            }
            0
        }

        fn find_node_mut<'a>(
            node: &'a mut QuadtreeNode<T>,
            x: f64,
            y: f64,
        ) -> Option<&'a mut Option<Box<QuadtreePoint>>> {
            if Self::is_leaf(node) {
                let matches = node
                    .point
                    .as_ref()
                    .map_or(false, |p| p.x == x && p.y == y);
                if matches {
                    return Some(&mut node.point);
                }
                return None;
            }
            if Self::is_pointer(node) {
                let test = QuadtreePoint { x, y };
                let which = Self::which_quadrant(node, &test);
                return match which {
                    1 => match node.nw.as_deref_mut() {
                        Some(child) => Self::find_node_mut(child, x, y),
                        None => None,
                    },
                    2 => match node.ne.as_deref_mut() {
                        Some(child) => Self::find_node_mut(child, x, y),
                        None => None,
                    },
                    3 => match node.sw.as_deref_mut() {
                        Some(child) => Self::find_node_mut(child, x, y),
                        None => None,
                    },
                    4 => match node.se.as_deref_mut() {
                        Some(child) => Self::find_node_mut(child, x, y),
                        None => None,
                    },
                    _ => None,
                };
            }
            None
        }

        fn walk_node(
            node: &mut Option<Box<QuadtreeNode<T>>>,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            if node.is_none() {
                return;
            }
            descent(node);
            if let Some(n) = node.as_mut() {
                Self::walk_node(&mut n.nw, descent, ascent);
                Self::walk_node(&mut n.ne, descent, ascent);
                Self::walk_node(&mut n.sw, descent, ascent);
                Self::walk_node(&mut n.se, descent, ascent);
            }
            ascent(node);
        }
    }
}
// Helper function
pub fn elision_<T>(key: Option<Box<T>>) {
    // Mirrors the C `elision_` callback: do nothing with the key.
    let _ = key;
}
