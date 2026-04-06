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
        pub fn quadtree_bounds_extend(&mut self, x: f64, y: f64) {
            let nw = self.nw.as_mut().unwrap();
            nw.x = nw.x.min(x);
            nw.y = nw.y.max(y);
            let se = self.se.as_mut().unwrap();
            se.x = se.x.max(x);
            se.y = se.y.min(y);
            self.width = (self.nw.as_ref().unwrap().x - self.se.as_ref().unwrap().x).abs();
            self.height = (self.nw.as_ref().unwrap().y - self.se.as_ref().unwrap().y).abs();
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
        fn node_contains(node: &QuadtreeNode<T>, point: &QuadtreePoint) -> bool {
            if let Some(ref bounds) = node.bounds {
                let nw = bounds.nw.as_ref().unwrap();
                let se = bounds.se.as_ref().unwrap();
                nw.x <= point.x && nw.y >= point.y && se.x >= point.x && se.y <= point.y
            } else {
                false
            }
        }

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
            self.point.is_none()
        }

        pub fn quadtree_node_isempty(&self) -> bool {
            self.nw.is_none() && self.ne.is_none() && self.sw.is_none() && self.se.is_none() && self.point.is_none()
        }

        pub fn quadtree_node_isleaf(&self) -> bool {
            !(self.nw.is_some() && self.ne.is_some() && self.sw.is_some() && self.se.is_some())
        }

        pub fn quadtree_node_reset(&mut self, _value_free: Option<fn(Option<T>)>) {
            self.point = None;
            self.key = None;
        }

        pub fn quadtree_node_with_bounds(minx: f64, miny: f64, maxx: f64, maxy: f64) -> QuadtreeNode<T> {
            let mut bounds = QuadtreeBounds::quadtree_bounds_new();
            bounds.quadtree_bounds_extend(maxx, maxy);
            bounds.quadtree_bounds_extend(minx, miny);
            QuadtreeNode {
                ne: None, nw: None, se: None, sw: None,
                bounds: Some(Box::new(bounds)),
                point: None, key: None,
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
        pub fn split_node_(&mut self, _node: Option<Box<QuadtreeNode<T>>>) {}
        pub fn insert_(&mut self, _tree: Option<Box<QuadtreeNode<T>>>, _point: Option<Box<QuadtreePoint>>, _key: Option<T>) {}

        fn get_quadrant_mut(root: &mut QuadtreeNode<T>, px: f64, py: f64) -> Option<&mut QuadtreeNode<T>> {
            let test = QuadtreePoint { x: px, y: py };
            let which = {
                let mut r = 0u8;
                if let Some(ref nw) = root.nw {
                    if QuadtreeNode::<T>::node_contains(nw, &test) { r = 1; }
                }
                if r == 0 { if let Some(ref ne) = root.ne {
                    if QuadtreeNode::<T>::node_contains(ne, &test) { r = 2; }
                }}
                if r == 0 { if let Some(ref sw) = root.sw {
                    if QuadtreeNode::<T>::node_contains(sw, &test) { r = 3; }
                }}
                if r == 0 { if let Some(ref se) = root.se {
                    if QuadtreeNode::<T>::node_contains(se, &test) { r = 4; }
                }}
                r
            };
            match which {
                1 => root.nw.as_mut().map(|b| b.as_mut()),
                2 => root.ne.as_mut().map(|b| b.as_mut()),
                3 => root.sw.as_mut().map(|b| b.as_mut()),
                4 => root.se.as_mut().map(|b| b.as_mut()),
                _ => None,
            }
        }

        fn split_node_impl(node: &mut QuadtreeNode<T>, key_free: Option<fn(Option<T>)>) -> bool {
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
            Self::insert_impl(node, old_point, old_key, key_free) != 0
        }

        // Returns: 0 = fail, 1 = new insert, 2 = replace
        fn insert_impl(root: &mut QuadtreeNode<T>, point: Box<QuadtreePoint>, key: Option<T>, key_free: Option<fn(Option<T>)>) -> i32 {
            if root.quadtree_node_isempty() {
                root.point = Some(point);
                root.key = key;
                1
            } else if root.quadtree_node_isleaf() {
                if root.point.as_ref().unwrap().x == point.x && root.point.as_ref().unwrap().y == point.y {
                    root.point = None;
                    root.key = None;
                    root.point = Some(point);
                    root.key = key;
                    2
                } else {
                    if !Self::split_node_impl(root, key_free) {
                        return 0;
                    }
                    Self::insert_impl(root, point, key, key_free)
                }
            } else if root.quadtree_node_ispointer() {
                let (px, py) = (point.x, point.y);
                match Self::get_quadrant_mut(root, px, py) {
                    Some(q) => Self::insert_impl(q, point, key, key_free),
                    None => 0,
                }
            } else {
                0
            }
        }

        fn find_mut(node: &mut QuadtreeNode<T>, x: f64, y: f64) -> *mut Option<Box<QuadtreePoint>> {
            if node.quadtree_node_isleaf() {
                if node.point.as_ref().unwrap().x == x && node.point.as_ref().unwrap().y == y {
                    return &mut node.point;
                }
            } else if node.quadtree_node_ispointer() {
                let (px, py) = (x, y);
                let test = QuadtreePoint { x, y };
                let which = {
                    let mut r = 0u8;
                    if let Some(ref nw) = node.nw {
                        if QuadtreeNode::<T>::node_contains(nw, &test) { r = 1; }
                    }
                    if r == 0 { if let Some(ref ne) = node.ne {
                        if QuadtreeNode::<T>::node_contains(ne, &test) { r = 2; }
                    }}
                    if r == 0 { if let Some(ref sw) = node.sw {
                        if QuadtreeNode::<T>::node_contains(sw, &test) { r = 3; }
                    }}
                    if r == 0 { if let Some(ref se) = node.se {
                        if QuadtreeNode::<T>::node_contains(se, &test) { r = 4; }
                    }}
                    r
                };
                match which {
                    1 => return Self::find_mut(node.nw.as_mut().unwrap(), px, py),
                    2 => return Self::find_mut(node.ne.as_mut().unwrap(), px, py),
                    3 => return Self::find_mut(node.sw.as_mut().unwrap(), px, py),
                    4 => return Self::find_mut(node.se.as_mut().unwrap(), px, py),
                    _ => {}
                }
            }
            std::ptr::null_mut()
        }

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

        pub fn quadtree_search(&mut self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            let root = self.root.as_mut().unwrap();
            let ptr = Self::find_mut(root, x, y);
            if ptr.is_null() {
                static mut NONE_VAL: Option<Box<QuadtreePoint>> = None;
                unsafe { &mut *std::ptr::addr_of_mut!(NONE_VAL) }
            } else {
                unsafe { &mut *ptr }
            }
        }

        pub fn quadtree_insert(&mut self, x: f64, y: f64, key: Option<T>) -> bool {
            let point = Box::new(QuadtreePoint::quadtree_point_new(x, y));
            if !QuadtreeNode::<T>::node_contains(self.root.as_ref().unwrap(), &point) {
                return false;
            }
            let key_free = self.key_free;
            let root = self.root.as_mut().unwrap();
            let status = Self::insert_impl(root, point, key, key_free);
            match status {
                1 => { self.length += 1; true }
                2 => true,
                _ => false,
            }
        }

        pub fn quadtree_walk(&mut self, descent: fn(&mut Option<Box<QuadtreeNode<T>>>), ascent: fn(&mut Option<Box<QuadtreeNode<T>>>)) {
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
            walk_impl(&mut self.root, descent, ascent);
        }
    }
}
pub fn elision_<T>(_key: Option<Box<T>>) {}
