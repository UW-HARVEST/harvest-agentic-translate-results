#![allow(invalid_reference_casting)]

pub mod quadtree {
    use std::mem;

    #[derive(Default)]
    pub struct QuadtreePoint {
        pub x: f64,
        pub y: f64,
    }

    impl QuadtreePoint {
        pub fn quadtree_point_new(x: f64, y: f64) -> QuadtreePoint {
            QuadtreePoint { x, y }
        }

        pub fn quadtree_point_free(&self) {}
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
            let this = self as *const Self as *mut Self;
            // The generated signature exposes only `&self`, so the mutation must happen internally.
            unsafe {
                extend_bounds_impl(&mut *this, x, y);
            }
        }

        pub fn quadtree_bounds_free(&self) {
            let this = self as *const Self as *mut Self;
            unsafe {
                (*this).nw = None;
                (*this).se = None;
                (*this).width = 0.0;
                (*this).height = 0.0;
            }
        }
    }

    pub struct QuadtreeNode<T> {
        pub ne: Option<Box<QuadtreeNode<T>>>,
        pub nw: Option<Box<QuadtreeNode<T>>>,
        pub se: Option<Box<QuadtreeNode<T>>>,
        pub sw: Option<Box<QuadtreeNode<T>>>,
        pub bounds: Option<Box<QuadtreeBounds>>,
        pub point: Option<Box<QuadtreePoint>>,
        pub key: Option<T>,
    }

    impl<T> Default for QuadtreeNode<T> {
        fn default() -> Self {
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
    }

    impl<T> QuadtreeNode<T> {
        pub fn node_contains_(&mut self, point: Option<Box<QuadtreePoint>>) {
            if let Some(point) = point {
                let _ = node_contains_impl(self, &point);
            }
        }

        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            if let Some(point) = point {
                let _ = get_quadrant_impl_mut(self, &point);
            }
        }

        pub fn find_(&mut self, x: f64, y: f64) {
            let _ = find_point_slot_mut(self, x, y);
        }

        pub fn quadtree_node_new() -> QuadtreeNode<T> {
            QuadtreeNode::default()
        }

        pub fn quadtree_node_free(&self, value_free: Option<fn(Option<T>)>) {
            let this = self as *const Self as *mut Self;
            unsafe {
                clear_node_in_place(&mut *this, value_free);
            }
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
            let this = self as *const Self as *mut Self;
            unsafe {
                reset_node_impl(&mut *this, value_free);
            }
        }

        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::quadtree_node_new();
            let mut bounds = QuadtreeBounds::quadtree_bounds_new();
            extend_bounds_impl(&mut bounds, maxx, maxy);
            extend_bounds_impl(&mut bounds, minx, miny);
            node.bounds = Some(Box::new(bounds));
            node
        }
    }

    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }

    impl<T> Default for Quadtree<T> {
        fn default() -> Self {
            Quadtree {
                root: None,
                key_free: None,
                length: 0,
            }
        }
    }

    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, node: Option<Box<QuadtreeNode<T>>>) {
            if let Some(mut node) = node {
                let _ = split_node_impl(&mut node, self.key_free);
            }
        }

        pub fn insert_(
            &mut self,
            tree: Option<Box<QuadtreeNode<T>>>,
            point: Option<Box<QuadtreePoint>>,
            key: Option<T>,
        ) {
            match (tree, point) {
                (Some(mut root), Some(point)) => {
                    let _ = insert_impl(&mut root, point, key, self.key_free);
                }
                (_, _) => {
                    elide_key(key);
                }
            }
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
            free_node_impl(&mut self.root, self.key_free);
            self.length = 0;
        }

        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            let this = self as *const Self as *mut Self;
            unsafe {
                if let Some(root) = (*this).root.as_deref_mut() {
                    if let Some(slot) = find_point_slot_mut(root, x, y) {
                        return slot;
                    }
                }
            }
            Box::leak(Box::new(None))
        }

        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            let this = self as *const Self as *mut Self;
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            unsafe {
                let Some(root) = (*this).root.as_deref_mut() else {
                    elide_key(key);
                    return false;
                };
                if !node_contains_impl(root, &point) {
                    elide_key(key);
                    return false;
                }
                let status = insert_impl(root, point, key, (*this).key_free);
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
            let this = self as *const Self as *mut Self;
            unsafe {
                walk_impl(&mut (*this).root, descent, ascent);
            }
        }
    }

    fn extend_bounds_impl(bounds: &mut QuadtreeBounds, x: f64, y: f64) {
        if bounds.nw.is_none() {
            bounds.nw = Some(Box::new(QuadtreePoint::quadtree_point_new(
                f64::INFINITY,
                f64::NEG_INFINITY,
            )));
        }
        if bounds.se.is_none() {
            bounds.se = Some(Box::new(QuadtreePoint::quadtree_point_new(
                f64::NEG_INFINITY,
                f64::INFINITY,
            )));
        }

        let nw = bounds.nw.as_deref_mut().expect("nw initialized");
        let se = bounds.se.as_deref_mut().expect("se initialized");

        nw.x = x.min(nw.x);
        nw.y = y.max(nw.y);
        se.x = x.max(se.x);
        se.y = y.min(se.y);
        bounds.width = (nw.x - se.x).abs();
        bounds.height = (nw.y - se.y).abs();
    }

    fn node_contains_impl<T>(outer: &QuadtreeNode<T>, it: &QuadtreePoint) -> bool {
        let Some(bounds) = outer.bounds.as_deref() else {
            return false;
        };
        let (Some(nw), Some(se)) = (bounds.nw.as_deref(), bounds.se.as_deref()) else {
            return false;
        };
        nw.x <= it.x && nw.y >= it.y && se.x >= it.x && se.y <= it.y
    }

    fn get_quadrant_impl_mut<'a, T>(
        root: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut QuadtreeNode<T>> {
        if root
            .nw
            .as_deref()
            .is_some_and(|node| node_contains_impl(node, point))
        {
            return root.nw.as_deref_mut();
        }
        if root
            .ne
            .as_deref()
            .is_some_and(|node| node_contains_impl(node, point))
        {
            return root.ne.as_deref_mut();
        }
        if root
            .sw
            .as_deref()
            .is_some_and(|node| node_contains_impl(node, point))
        {
            return root.sw.as_deref_mut();
        }
        if root
            .se
            .as_deref()
            .is_some_and(|node| node_contains_impl(node, point))
        {
            return root.se.as_deref_mut();
        }
        None
    }

    fn reset_node_impl<T>(node: &mut QuadtreeNode<T>, value_free: Option<fn(Option<T>)>) {
        node.point = None;
        call_value_free(value_free, node.key.take());
    }

    fn clear_node_in_place<T>(node: &mut QuadtreeNode<T>, value_free: Option<fn(Option<T>)>) {
        free_node_impl(&mut node.nw, value_free);
        free_node_impl(&mut node.ne, value_free);
        free_node_impl(&mut node.sw, value_free);
        free_node_impl(&mut node.se, value_free);
        node.bounds = None;
        reset_node_impl(node, value_free);
    }

    fn split_node_impl<T>(
        node: &mut QuadtreeNode<T>,
        value_free: Option<fn(Option<T>)>,
    ) -> bool {
        let Some(bounds) = node.bounds.as_deref() else {
            return false;
        };
        let (Some(nw), Some(_se)) = (bounds.nw.as_deref(), bounds.se.as_deref()) else {
            return false;
        };

        let x = nw.x;
        let y = nw.y;
        let hw = bounds.width / 2.0;
        let hh = bounds.height / 2.0;

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
            Some(point) => insert_impl(node, point, old_key, value_free) != 0,
            None => true,
        }
    }

    fn insert_impl<T>(
        root: &mut QuadtreeNode<T>,
        point: Box<QuadtreePoint>,
        key: Option<T>,
        value_free: Option<fn(Option<T>)>,
    ) -> u32 {
        if root.quadtree_node_isempty() {
            root.point = Some(point);
            root.key = key;
            1
        } else if root.quadtree_node_isleaf() {
            let is_same_point = root
                .point
                .as_deref()
                .is_some_and(|current| current.x == point.x && current.y == point.y);
            if is_same_point {
                reset_node_impl(root, value_free);
                root.point = Some(point);
                root.key = key;
                2
            } else if split_node_impl(root, value_free) {
                insert_impl(root, point, key, value_free)
            } else {
                elide_key(key);
                0
            }
        } else if root.quadtree_node_ispointer() {
            match get_quadrant_impl_mut(root, &point) {
                Some(quadrant) => insert_impl(quadrant, point, key, value_free),
                None => {
                    elide_key(key);
                    0
                }
            }
        } else {
            elide_key(key);
            0
        }
    }

    fn find_point_slot_mut<'a, T>(
        node: &'a mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> Option<&'a mut Option<Box<QuadtreePoint>>> {
        if node.quadtree_node_isleaf() {
            let found = node
                .point
                .as_deref()
                .is_some_and(|point| point.x == x && point.y == y);
            if found {
                return Some(&mut node.point);
            }
            return None;
        }

        if node.quadtree_node_ispointer() {
            let point = QuadtreePoint::quadtree_point_new(x, y);
            return get_quadrant_impl_mut(node, &point)
                .and_then(|quadrant| find_point_slot_mut(quadrant, x, y));
        }

        None
    }

    fn walk_impl<T>(
        root: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        if root.is_none() {
            return;
        }

        descent(root);

        if let Some(node) = root.as_deref_mut() {
            walk_impl(&mut node.nw, descent, ascent);
            walk_impl(&mut node.ne, descent, ascent);
            walk_impl(&mut node.sw, descent, ascent);
            walk_impl(&mut node.se, descent, ascent);
        }

        ascent(root);
    }

    fn free_node_impl<T>(
        node: &mut Option<Box<QuadtreeNode<T>>>,
        value_free: Option<fn(Option<T>)>,
    ) {
        if let Some(mut node_box) = node.take() {
            free_node_impl(&mut node_box.nw, value_free);
            free_node_impl(&mut node_box.ne, value_free);
            free_node_impl(&mut node_box.sw, value_free);
            free_node_impl(&mut node_box.se, value_free);
            node_box.bounds = None;
            reset_node_impl(&mut node_box, value_free);
        }
    }

    fn call_value_free<T>(value_free: Option<fn(Option<T>)>, key: Option<T>) {
        if let Some(value_free) = value_free {
            value_free(key);
        } else {
            elide_key(key);
        }
    }

    fn elide_key<T>(key: Option<T>) {
        if let Some(key) = key {
            mem::forget(key);
        }
    }
}

pub fn elision_<T>(key: Option<Box<T>>) {
    if let Some(key) = key {
        std::mem::forget(key);
    }
}
