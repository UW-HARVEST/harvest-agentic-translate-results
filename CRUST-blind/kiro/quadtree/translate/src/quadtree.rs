#[allow(invalid_reference_casting, static_mut_refs)]
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
            let s = unsafe { &mut *(self as *const Self as *mut Self) };
            if let Some(ref mut nw) = s.nw {
                nw.x = nw.x.min(x);
                nw.y = nw.y.max(y);
            }
            if let Some(ref mut se) = s.se {
                se.x = se.x.max(x);
                se.y = se.y.min(y);
            }
            if let (Some(ref nw), Some(ref se)) = (&s.nw, &s.se) {
                s.width = (nw.x - se.x).abs();
                s.height = (nw.y - se.y).abs();
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
            let s = unsafe { &mut *(self as *const Self as *mut Self) };
            let pt = s.point.take();
            let k = s.key.take();
            drop(pt);
            if let Some(f) = value_free {
                f(k);
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

    fn node_contains_check<T>(outer: &QuadtreeNode<T>, it: &QuadtreePoint) -> bool {
        if let Some(ref b) = outer.bounds {
            if let (Some(ref nw), Some(ref se)) = (&b.nw, &b.se) {
                return nw.x <= it.x && nw.y >= it.y && se.x >= it.x && se.y <= it.y;
            }
        }
        false
    }

    fn get_quadrant_mut<'a, T>(root: &'a mut QuadtreeNode<T>, point: &QuadtreePoint) -> Option<&'a mut QuadtreeNode<T>> {
        let nw_ok = root.nw.as_ref().map_or(false, |n| node_contains_check(n, point));
        let ne_ok = root.ne.as_ref().map_or(false, |n| node_contains_check(n, point));
        let sw_ok = root.sw.as_ref().map_or(false, |n| node_contains_check(n, point));
        let se_ok = root.se.as_ref().map_or(false, |n| node_contains_check(n, point));
        if nw_ok { root.nw.as_deref_mut() }
        else if ne_ok { root.ne.as_deref_mut() }
        else if sw_ok { root.sw.as_deref_mut() }
        else if se_ok { root.se.as_deref_mut() }
        else { None }
    }

    fn insert_impl<T>(root: &mut QuadtreeNode<T>, point: Box<QuadtreePoint>, key: Option<T>, key_free: &Option<fn(Option<T>)>) -> i32 {
        if root.quadtree_node_isempty() {
            root.point = Some(point);
            root.key = key;
            1
        } else if root.quadtree_node_isleaf() {
            let same = {
                let rp = root.point.as_ref().unwrap();
                rp.x == point.x && rp.y == point.y
            };
            if same {
                let old_pt = root.point.take();
                let old_key = root.key.take();
                drop(old_pt);
                if let Some(f) = key_free { f(old_key); }
                root.point = Some(point);
                root.key = key;
                2
            } else {
                if !split_node_impl(root, key_free) { return 0; }
                insert_impl(root, point, key, key_free)
            }
        } else if root.quadtree_node_ispointer() {
            match get_quadrant_mut(root, &point) {
                Some(q) => insert_impl(q, point, key, key_free),
                None => 0,
            }
        } else {
            0
        }
    }

    fn split_node_impl<T>(node: &mut QuadtreeNode<T>, key_free: &Option<fn(Option<T>)>) -> bool {
        let (x, y, hw, hh) = {
            let b = node.bounds.as_ref().unwrap();
            let nw = b.nw.as_ref().unwrap();
            (nw.x, nw.y, b.width / 2.0, b.height / 2.0)
        };
        node.nw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x, y - hh, x + hw, y)));
        node.ne = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x + hw, y - hh, x + hw * 2.0, y)));
        node.sw = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x, y - hh * 2.0, x + hw, y - hh)));
        node.se = Some(Box::new(QuadtreeNode::quadtree_node_with_bounds(x + hw, y - hh * 2.0, x + hw * 2.0, y - hh)));
        let old_point = node.point.take().unwrap();
        let old_key = node.key.take();
        insert_impl(node, old_point, old_key, key_free) != 0
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
            self.root = None;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            let s = unsafe { &mut *(self as *const Self as *mut Self) };
            fn search_mut<T>(node: &mut QuadtreeNode<T>, x: f64, y: f64) -> *mut Option<Box<QuadtreePoint>> {
                if node.quadtree_node_isleaf() {
                    if let Some(ref pt) = node.point {
                        if pt.x == x && pt.y == y {
                            return &mut node.point as *mut _;
                        }
                    }
                } else if node.quadtree_node_ispointer() {
                    let test = QuadtreePoint { x, y };
                    let nw_ok = node.nw.as_ref().map_or(false, |n| node_contains_check(n, &test));
                    let ne_ok = node.ne.as_ref().map_or(false, |n| node_contains_check(n, &test));
                    let sw_ok = node.sw.as_ref().map_or(false, |n| node_contains_check(n, &test));
                    let se_ok = node.se.as_ref().map_or(false, |n| node_contains_check(n, &test));
                    if nw_ok { if let Some(ref mut c) = node.nw { return search_mut(c, x, y); } }
                    else if ne_ok { if let Some(ref mut c) = node.ne { return search_mut(c, x, y); } }
                    else if sw_ok { if let Some(ref mut c) = node.sw { return search_mut(c, x, y); } }
                    else if se_ok { if let Some(ref mut c) = node.se { return search_mut(c, x, y); } }
                }
                std::ptr::null_mut()
            }
            static mut NONE_POINT: Option<Box<QuadtreePoint>> = None;
            if let Some(ref mut root) = s.root {
                let ptr = search_mut(root, x, y);
                if !ptr.is_null() {
                    return unsafe { &mut *ptr };
                }
            }
            unsafe { &mut NONE_POINT }
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            let s = unsafe { &mut *(self as *const Self as *mut Self) };
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            if let Some(ref root) = s.root {
                if !node_contains_check(root, &point) { return false; }
            } else {
                return false;
            }
            let root = s.root.as_deref_mut().unwrap();
            let status = insert_impl(root, point, key, &s.key_free);
            if status == 0 { return false; }
            if status == 1 { s.length += 1; }
            true
        }
        pub fn quadtree_walk(&self, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
            let s = unsafe { &mut *(self as *const Self as *mut Self) };
            fn walk_impl<T>(node: &mut Option<Box<QuadtreeNode<T>>>, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
                descent(node);
                if let Some(ref mut n) = node {
                    if n.nw.is_some() { walk_impl(&mut n.nw, descent, ascent); }
                    if n.ne.is_some() { walk_impl(&mut n.ne, descent, ascent); }
                    if n.sw.is_some() { walk_impl(&mut n.sw, descent, ascent); }
                    if n.se.is_some() { walk_impl(&mut n.se, descent, ascent); }
                }
                ascent(node);
            }
            walk_impl(&mut s.root, descent, ascent);
        }
    }
}
pub fn elision_<T>(key: Option<Box<T>>) { drop(key); }
