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
            let _ = self;
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
            let this = self as *const _ as *mut Self;
            unsafe {
                if let (Some(nw), Some(se)) = ((*this).nw.as_deref_mut(), (*this).se.as_deref_mut()) {
                    nw.x = x.min(nw.x);
                    nw.y = y.max(nw.y);
                    se.x = x.max(se.x);
                    se.y = y.min(se.y);
                    (*this).width = (nw.x - se.x).abs();
                    (*this).height = (nw.y - se.y).abs();
                }
            }
        }
        pub fn quadtree_bounds_free(&self) {
            let this = self as *const _ as *mut Self;
            unsafe {
                (*this).nw = None;
                (*this).se = None;
                (*this).width = 0.0;
                (*this).height = 0.0;
            }
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
    impl <T> QuadtreeNode<T> {
        pub fn node_contains_(&mut self, point: Option<Box<QuadtreePoint>>) {
            let _ = point.as_deref().is_some_and(|point| node_contains(self, point));
        }
        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            if let Some(point) = point.as_deref() {
                let _ = get_quadrant_mut(self, point);
            }
        }
        pub fn find_(&mut self, x: f64, y: f64) {
            let _ = find_point_mut(self, x, y);
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
        pub fn quadtree_node_free(&self, value_free: Option<fn(Option<T>)>,) {
            let this = self as *const _ as *mut Self;
            unsafe {
                clear_node_in_place_raw(this, value_free);
            }
        }
        pub fn quadtree_node_ispointer(&self) -> bool {
            node_is_pointer(self) || node_is_uninitialized(self)
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            node_is_empty(self)
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            node_is_leaf(self) || node_is_uninitialized(self)
        }
        pub fn quadtree_node_reset(&self, value_free: Option<fn(Option<T>)>,) {
            let this = self as *const _ as *mut Self;
            unsafe {
                node_reset_raw(this, value_free);
            }
        }
        pub fn quadtree_node_with_bounds(minx: f64, miny: f64, maxx: f64, maxy: f64) -> QuadtreeNode<T> {
            let mut node = QuadtreeNode::quadtree_node_new();
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
    impl <T> Quadtree<T> {
        pub fn split_node_(&mut self, node: Option<Box<QuadtreeNode<T>>>) {
            if let Some(mut node) = node {
                let _ = split_node(self.key_free, &mut node);
            }
        }
        pub fn insert_(&mut self, tree: Option<Box<QuadtreeNode<T>>>, point: Option<Box<QuadtreePoint>>, key: Option<T>) {
            if let Some(mut tree) = tree {
                let _ = insert(self.key_free, &mut tree, point, key);
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
            if let Some(root) = self.root.take() {
                free_node(*root, self.key_free);
            }
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            let this = self as *const _ as *mut Self;
            unsafe {
                if let Some(root) = (*this).root.as_deref_mut() {
                    if let Some(found) = find_point_mut(root, x, y) {
                        return found;
                    }
                }
            }
            Box::leak(Box::new(None))
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            let this = self as *const _ as *mut Self;
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            unsafe {
                let Some(root) = (*this).root.as_deref_mut() else {
                    return false;
                };
                if !node_contains(root, &point) {
                    return false;
                }
                let status = insert((*this).key_free, root, Some(point), key);
                if status == 1 {
                    (*this).length += 1;
                }
                status != 0
            }
        }
        pub fn quadtree_walk(&self, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
            let this = self as *const _ as *mut Self;
            unsafe {
                walk(&mut (*this).root, descent, ascent);
            }
        }
    }

    fn node_is_uninitialized<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_none()
            && node.ne.is_none()
            && node.sw.is_none()
            && node.se.is_none()
            && node.point.is_none()
            && node.bounds.is_none()
    }

    fn node_is_pointer<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_some()
            && node.ne.is_some()
            && node.sw.is_some()
            && node.se.is_some()
            && !node_is_leaf(node)
    }

    fn node_is_empty<T>(node: &QuadtreeNode<T>) -> bool {
        node.nw.is_none()
            && node.ne.is_none()
            && node.sw.is_none()
            && node.se.is_none()
            && !node_is_leaf(node)
    }

    fn node_is_leaf<T>(node: &QuadtreeNode<T>) -> bool {
        node.point.is_some()
    }

    fn node_reset<T>(node: &mut QuadtreeNode<T>, value_free: Option<fn(Option<T>)>) {
        node.point = None;
        let key = node.key.take();
        if let Some(value_free) = value_free {
            value_free(key);
        }
    }

    unsafe fn node_reset_raw<T>(node: *mut QuadtreeNode<T>, value_free: Option<fn(Option<T>)>) {
        (*node).point = None;
        let key = (*node).key.take();
        if let Some(value_free) = value_free {
            value_free(key);
        }
    }

    unsafe fn clear_node_in_place_raw<T>(
        node: *mut QuadtreeNode<T>,
        value_free: Option<fn(Option<T>)>,
    ) {
        if let Some(mut child) = (*node).nw.take() {
            clear_node_in_place_raw(&mut *child, value_free);
        }
        if let Some(mut child) = (*node).ne.take() {
            clear_node_in_place_raw(&mut *child, value_free);
        }
        if let Some(mut child) = (*node).sw.take() {
            clear_node_in_place_raw(&mut *child, value_free);
        }
        if let Some(mut child) = (*node).se.take() {
            clear_node_in_place_raw(&mut *child, value_free);
        }
        (*node).bounds = None;
        node_reset_raw(node, value_free);
    }

    fn free_node<T>(mut node: QuadtreeNode<T>, value_free: Option<fn(Option<T>)>) {
        if let Some(child) = node.nw.take() {
            free_node(*child, value_free);
        }
        if let Some(child) = node.ne.take() {
            free_node(*child, value_free);
        }
        if let Some(child) = node.sw.take() {
            free_node(*child, value_free);
        }
        if let Some(child) = node.se.take() {
            free_node(*child, value_free);
        }
        node.bounds = None;
        node_reset(&mut node, value_free);
    }

    fn node_contains<T>(outer: &QuadtreeNode<T>, point: &QuadtreePoint) -> bool {
        let Some(bounds) = outer.bounds.as_deref() else {
            return false;
        };
        let (Some(nw), Some(se)) = (bounds.nw.as_deref(), bounds.se.as_deref()) else {
            return false;
        };
        nw.x <= point.x && nw.y >= point.y && se.x >= point.x && se.y <= point.y
    }

    fn get_quadrant_mut<'a, T>(
        root: &'a mut QuadtreeNode<T>,
        point: &QuadtreePoint,
    ) -> Option<&'a mut QuadtreeNode<T>> {
        if root.nw.as_ref().is_some_and(|node| node_contains(node, point)) {
            return root.nw.as_deref_mut();
        }
        if root.ne.as_ref().is_some_and(|node| node_contains(node, point)) {
            return root.ne.as_deref_mut();
        }
        if root.sw.as_ref().is_some_and(|node| node_contains(node, point)) {
            return root.sw.as_deref_mut();
        }
        if root.se.as_ref().is_some_and(|node| node_contains(node, point)) {
            return root.se.as_deref_mut();
        }
        None
    }

    fn split_node<T>(
        key_free: Option<fn(Option<T>)>,
        node: &mut QuadtreeNode<T>,
    ) -> bool {
        let Some(bounds) = node.bounds.as_deref() else {
            return false;
        };
        let (Some(nw_bound), Some(_se_bound)) = (bounds.nw.as_deref(), bounds.se.as_deref()) else {
            return false;
        };

        let x = nw_bound.x;
        let y = nw_bound.y;
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

        let old = node.point.take();
        let key = node.key.take();
        insert(key_free, node, old, key) != 0
    }

    fn insert<T>(
        key_free: Option<fn(Option<T>)>,
        root: &mut QuadtreeNode<T>,
        point: Option<Box<QuadtreePoint>>,
        key: Option<T>,
    ) -> u8 {
        let Some(point) = point else {
            return 0;
        };

        if node_is_empty(root) {
            root.point = Some(point);
            root.key = key;
            1
        } else if node_is_leaf(root) {
            let replace = root
                .point
                .as_deref()
                .is_some_and(|existing| existing.x == point.x && existing.y == point.y);
            if replace {
                node_reset(root, key_free);
                root.point = Some(point);
                root.key = key;
                2
            } else if !split_node(key_free, root) {
                0
            } else {
                insert(key_free, root, Some(point), key)
            }
        } else if node_is_pointer(root) {
            match get_quadrant_mut(root, &point) {
                Some(quadrant) => insert(key_free, quadrant, Some(point), key),
                None => 0,
            }
        } else {
            0
        }
    }

    fn find_point_mut<T>(
        node: &mut QuadtreeNode<T>,
        x: f64,
        y: f64,
    ) -> Option<&mut Option<Box<QuadtreePoint>>> {
        if node_is_leaf(node) {
            if node
                .point
                .as_deref()
                .is_some_and(|point| point.x == x && point.y == y)
            {
                return Some(&mut node.point);
            }
        } else if node_is_pointer(node) {
            let test = QuadtreePoint { x, y };
            if let Some(quadrant) = get_quadrant_mut(node, &test) {
                return find_point_mut(quadrant, x, y);
            }
        }
        None
    }

    fn walk<T>(
        root: &mut Option<Box<QuadtreeNode<T>>>,
        descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
    ) {
        if root.is_none() {
            return;
        }

        descent(root);
        if let Some(node) = root.as_deref_mut() {
            walk(&mut node.nw, descent, ascent);
            walk(&mut node.ne, descent, ascent);
            walk(&mut node.sw, descent, ascent);
            walk(&mut node.se, descent, ascent);
        }
        ascent(root);
    }
}
// Helper function
pub fn elision_<T>(key: Option<Box<T>>) {
    let _ = key;
}
