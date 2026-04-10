pub mod quadtree {
    use std::ptr;

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
                nw: Some(Box::new(QuadtreePoint { x: f64::INFINITY, y: f64::NEG_INFINITY })),
                se: Some(Box::new(QuadtreePoint { x: f64::NEG_INFINITY, y: f64::INFINITY })),
                width: 0.0,
                height: 0.0,
            }
        }
        pub fn quadtree_bounds_extend(&self, x: f64, y: f64) {
            let p = ptr::from_ref(self) as *mut Self;
            unsafe {
                let nw = (*p).nw.as_mut().unwrap();
                nw.x = nw.x.min(x);
                nw.y = nw.y.max(y);
                let se = (*p).se.as_mut().unwrap();
                se.x = se.x.max(x);
                se.y = se.y.min(y);
                (*p).width = ((*p).nw.as_ref().unwrap().x - (*p).se.as_ref().unwrap().x).abs();
                (*p).height = ((*p).nw.as_ref().unwrap().y - (*p).se.as_ref().unwrap().y).abs();
            }
        }
        pub fn quadtree_bounds_free(&self) {}
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
        pub fn node_contains_(&mut self, _point: Option<Box<QuadtreePoint>>) {}
        pub fn get_quadrant_(&mut self, _point: Option<Box<QuadtreePoint>>) {}
        pub fn find_(&mut self, _x: f64, _y: f64) {}
        pub fn quadtree_node_new() -> QuadtreeNode<T> {
            QuadtreeNode {
                ne: None, nw: None, se: None, sw: None,
                bounds: None, point: None, key: None,
            }
        }
        pub fn quadtree_node_free(&self, _value_free: Option<fn(Option<T>)>) {}
        pub fn quadtree_node_ispointer(&self) -> bool {
            self.nw.is_some() && self.ne.is_some() && self.sw.is_some() && self.se.is_some()
                && !self.quadtree_node_isleaf()
        }
        pub fn quadtree_node_isempty(&self) -> bool {
            self.nw.is_none() && self.ne.is_none() && self.sw.is_none() && self.se.is_none()
                && !self.quadtree_node_isleaf()
        }
        pub fn quadtree_node_isleaf(&self) -> bool {
            self.point.is_some()
        }
        pub fn quadtree_node_reset(&self, value_free: Option<fn(Option<T>)>) {
            let p = ptr::from_ref(self) as *mut Self;
            unsafe {
                let pt = (*p).point.take();
                let k = (*p).key.take();
                drop(pt);
                if let Some(f) = value_free { f(k); }
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

    fn node_contains_check<T>(outer: &QuadtreeNode<T>, pt: &QuadtreePoint) -> bool {
        if let Some(ref b) = outer.bounds {
            let nw = b.nw.as_ref().unwrap();
            let se = b.se.as_ref().unwrap();
            nw.x <= pt.x && nw.y >= pt.y && se.x >= pt.x && se.y <= pt.y
        } else {
            false
        }
    }

    fn get_quadrant_ref<'a, T>(root: &'a mut QuadtreeNode<T>, point: &QuadtreePoint) -> Option<&'a mut QuadtreeNode<T>> {
        if root.nw.as_ref().map_or(false, |n| node_contains_check(n, point)) {
            return root.nw.as_deref_mut();
        }
        if root.ne.as_ref().map_or(false, |n| node_contains_check(n, point)) {
            return root.ne.as_deref_mut();
        }
        if root.sw.as_ref().map_or(false, |n| node_contains_check(n, point)) {
            return root.sw.as_deref_mut();
        }
        if root.se.as_ref().map_or(false, |n| node_contains_check(n, point)) {
            return root.se.as_deref_mut();
        }
        None
    }

    fn find_impl<T>(node: &QuadtreeNode<T>, x: f64, y: f64) -> bool {
        if node.quadtree_node_isleaf() {
            let pt = node.point.as_ref().unwrap();
            pt.x == x && pt.y == y
        } else if node.quadtree_node_ispointer() {
            let test = QuadtreePoint { x, y };
            for child in [&node.nw, &node.ne, &node.sw, &node.se] {
                if let Some(ref c) = child {
                    if node_contains_check(c, &test) {
                        return find_impl(c, x, y);
                    }
                }
            }
            false
        } else {
            false
        }
    }

    fn find_node_mut<'a, T>(node: &'a mut QuadtreeNode<T>, x: f64, y: f64) -> Option<&'a mut QuadtreeNode<T>> {
        if node.quadtree_node_isleaf() {
            let pt = node.point.as_ref().unwrap();
            if pt.x == x && pt.y == y {
                return Some(node);
            }
        } else if node.quadtree_node_ispointer() {
            let test = QuadtreePoint { x, y };
            let contains_nw = node.nw.as_ref().map_or(false, |n| node_contains_check(n, &test));
            let contains_ne = node.ne.as_ref().map_or(false, |n| node_contains_check(n, &test));
            let contains_sw = node.sw.as_ref().map_or(false, |n| node_contains_check(n, &test));
            let contains_se = node.se.as_ref().map_or(false, |n| node_contains_check(n, &test));
            if contains_nw { return find_node_mut(node.nw.as_deref_mut().unwrap(), x, y); }
            if contains_ne { return find_node_mut(node.ne.as_deref_mut().unwrap(), x, y); }
            if contains_sw { return find_node_mut(node.sw.as_deref_mut().unwrap(), x, y); }
            if contains_se { return find_node_mut(node.se.as_deref_mut().unwrap(), x, y); }
        }
        None
    }

    fn split_node_mut<T>(node: &mut QuadtreeNode<T>, key_free: Option<fn(Option<T>)>) -> bool {
        let b = node.bounds.as_ref().unwrap();
        let x = b.nw.as_ref().unwrap().x;
        let y = b.nw.as_ref().unwrap().y;
        let hw = b.width / 2.0;
        let hh = b.height / 2.0;

        node.nw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x, y - hh, x + hw, y)));
        node.ne = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x + hw, y - hh, x + hw * 2.0, y)));
        node.sw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x, y - hh * 2.0, x + hw, y - hh)));
        node.se = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x + hw, y - hh * 2.0, x + hw * 2.0, y - hh)));

        let old_point = node.point.take().unwrap();
        let old_key = node.key.take();

        insert_impl(node, old_point, old_key, key_free) != 0
    }

    fn insert_impl<T>(root: &mut QuadtreeNode<T>, point: Box<QuadtreePoint>, key: Option<T>, key_free: Option<fn(Option<T>)>) -> i32 {
        if root.quadtree_node_isempty() {
            root.point = Some(point);
            root.key = key;
            1
        } else if root.quadtree_node_isleaf() {
            let rp = root.point.as_ref().unwrap();
            if rp.x == point.x && rp.y == point.y {
                let old_pt = root.point.take();
                let old_key = root.key.take();
                drop(old_pt);
                if let Some(f) = key_free { f(old_key); }
                root.point = Some(point);
                root.key = key;
                2
            } else {
                if !split_node_mut(root, key_free) {
                    return 0;
                }
                insert_impl(root, point, key, key_free)
            }
        } else if root.quadtree_node_ispointer() {
            let test_pt = QuadtreePoint { x: point.x, y: point.y };
            match get_quadrant_ref(root, &test_pt) {
                Some(q) => insert_impl(q, point, key, key_free),
                None => 0,
            }
        } else {
            0
        }
    }

    fn walk_impl<T>(node_opt: &mut Option<Box<QuadtreeNode<T>>>, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
        if node_opt.is_none() { return; }
        descent(node_opt);
        let node = node_opt.as_deref_mut().unwrap();
        if node.nw.is_some() { walk_impl(&mut node.nw, descent, ascent); }
        if node.ne.is_some() { walk_impl(&mut node.ne, descent, ascent); }
        if node.sw.is_some() { walk_impl(&mut node.sw, descent, ascent); }
        if node.se.is_some() { walk_impl(&mut node.se, descent, ascent); }
        ascent(node_opt);
    }

    #[derive(Default)]
    pub struct Quadtree<T> {
        pub root: Option<Box<QuadtreeNode<T>>>,
        pub key_free: Option<fn(Option<T>)>,
        pub length: u32,
    }
    impl<T> Quadtree<T> {
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {}
        pub fn insert_(&mut self, _tree: Option<Box<QuadtreeNode<T>>>, _point: Option<Box<QuadtreePoint>>, _key: Option<T>) {}
        pub fn quadtree_new(minx: f64, miny: f64, maxx: f64, maxy: f64) -> Quadtree<T> {
            Quadtree {
                root: Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(minx, miny, maxx, maxy))),
                key_free: None,
                length: 0,
            }
        }
        pub fn quadtree_free(&mut self) {
            self.root.take();
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            let p = ptr::from_ref(self) as *mut Self;
            unsafe {
                let root = (*p).root.as_deref_mut().unwrap();
                if find_impl(root, x, y) {
                    let node = find_node_mut((*p).root.as_deref_mut().unwrap(), x, y).unwrap();
                    &mut node.point
                } else {
                    static mut NONE_POINT: Option<Box<QuadtreePoint>> = None;
                    NONE_POINT = None;
                    &mut *(&raw mut NONE_POINT as *mut Option<Box<QuadtreePoint>>)
                }
            }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            let p = ptr::from_ref(self) as *mut Self;
            unsafe {
                let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
                let root = (*p).root.as_ref().unwrap();
                if !node_contains_check(root, &point) {
                    return false;
                }
                let root_mut = (*p).root.as_deref_mut().unwrap();
                let status = insert_impl(root_mut, point, key, (*p).key_free);
                if status == 1 { (*p).length += 1; }
                status != 0
            }
        }
        pub fn quadtree_walk(&self, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
            let p = ptr::from_ref(self) as *mut Self;
            unsafe { walk_impl(&mut (*p).root, descent, ascent); }
        }
    }
}

pub fn elision_<T>(key: Option<Box<T>>) {
    drop(key);
}
