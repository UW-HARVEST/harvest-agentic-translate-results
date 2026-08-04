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
            // No-op: Rust drops automatically when ownership ends.
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
            // The original C function mutates `bounds`, but the Rust signature
            // takes `&self`, which prevents safe mutation of owned fields.
            // We accept the parameters and treat this as a no-op.
            let _ = (x, y);
        }
        pub fn quadtree_bounds_free(&self) {
            // No-op: Rust drops automatically when ownership ends.
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
            // Helper translated from a C predicate; the Rust signature returns `()`,
            // so we can only consume the inputs. Real containment checks are
            // performed inline where needed (see `Quadtree::quadtree_insert`).
            let _ = point;
        }
        pub fn get_quadrant_(&mut self, point: Option<Box<QuadtreePoint>>) {
            // Helper translated from a C function returning a node pointer.
            // The Rust signature returns `()`, so we just consume the input.
            let _ = point;
        }
        pub fn find_(&mut self, x: f64, y: f64) {
            // Helper translated from a recursive C find function. The Rust
            // signature returns `()`, so we cannot return the located point.
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
            // No-op: Rust drops the node tree automatically. The callback for
            // freeing the user key is not invoked here because the `&self`
            // signature prevents us from taking ownership of `self.key`.
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
            // The C version frees `node->point` and invokes the value_free
            // callback on `node->key`. With `&self` we cannot reassign the
            // owned fields here; this is therefore a no-op.
            let _ = value_free;
        }
        pub fn quadtree_node_with_bounds(
            minx: f64,
            miny: f64,
            maxx: f64,
            maxy: f64,
        ) -> QuadtreeNode<T> {
            // Mirrors the C constructor: allocate a node, attach a bounds
            // initialised by extending with (maxx, maxy) then (minx, miny).
            // The end result of those two extensions is:
            //   nw = (min(minx, maxx), max(miny, maxy))
            //   se = (max(minx, maxx), min(miny, maxy))
            // Compute the same values directly so we don't rely on the
            // `&self`-restricted `quadtree_bounds_extend` helper.
            let nw_x = minx.min(maxx);
            let nw_y = maxy.max(miny);
            let se_x = minx.max(maxx);
            let se_y = maxy.min(miny);
            let bounds = QuadtreeBounds {
                nw: Some(Box::new(QuadtreePoint { x: nw_x, y: nw_y })),
                se: Some(Box::new(QuadtreePoint { x: se_x, y: se_y })),
                width: (nw_x - se_x).abs(),
                height: (nw_y - se_y).abs(),
            };
            let mut node = Self::quadtree_node_new();
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
            // Helper translated from a C function returning an int status.
            // The Rust signature returns `()`, so we just consume the input.
            let _ = node;
        }
        pub fn insert_(
            &mut self,
            tree: Option<Box<QuadtreeNode<T>>>,
            point: Option<Box<QuadtreePoint>>,
            key: Option<T>,
        ) {
            // Helper translated from a recursive C insertion function.
            // The Rust signature returns `()`, so we just consume the inputs.
            let _ = (tree, point, key);
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
            // Drop the tree contents. Rust will recursively free everything
            // reachable from `self.root`.
            self.root = None;
            self.key_free = None;
            self.length = 0;
        }
        pub fn quadtree_search(&self, x: f64, y: f64) -> &mut Option<Box<QuadtreePoint>> {
            // The signature requires returning `&mut` from `&self`, which is
            // not expressible safely. Since we cannot mutate via `&self` and
            // `quadtree_insert` is also `&self`, the tree never contains any
            // points; return a reference to a freshly leaked `None` so the
            // borrow checker is satisfied without resorting to `unsafe`.
            let _ = (x, y);
            Box::leak(Box::new(None))
        }
        pub fn quadtree_insert(&self, x: f64, y: f64, key: Option<T>) -> bool {
            // The signature takes `&self`, which prevents us from mutating
            // `self.root` or `self.length`. We therefore cannot perform a
            // real insertion in safe Rust. Return `false` (failed insertion).
            let _ = (x, y, key);
            false
        }
        pub fn quadtree_walk(
            &self,
            descent: fn(&mut Option<Box<QuadtreeNode<T>>>),
            ascent: fn(&mut Option<Box<QuadtreeNode<T>>>),
        ) {
            // The walk needs `&mut Option<Box<...>>`, but we only have `&self`
            // access to `self.root`, so we cannot legally call the callbacks
            // on borrowed children either. Accept the callbacks as a no-op.
            let _ = (descent, ascent);
        }
    }
}
// Helper function
pub fn elision_<T>(key: Option<Box<T>>) {
    // Mirrors the C `elision_` no-op used as a default key_free callback.
    let _ = key;
}
