use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub enum Color {
    Red,
    Black,
}

pub type Key = i32;
pub type NodeRef = Rc<RefCell<Node>>;

#[derive(Debug, Clone)]
pub struct Node {
    pub key: Key,
    pub color: Color,
    pub left: Option<NodeRef>,   // None == NIL
    pub right: Option<NodeRef>,  // None == NIL
    pub parent: Option<NodeRef>, // None == NIL / root parent
}

#[derive(Debug, Clone)]
pub struct RBTree {
    /// Root of the tree. `None` means the tree is empty (NIL).
    pub root: Option<NodeRef>,
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        // y = x.left; x.left must be non-NIL for a valid right rotation
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate requires a non-NIL left child");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match &x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_is_left_x = xp
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &x));
                if xp_is_left_x {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.right = x; x.parent = y
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y.clone());
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x.right; must be non-NIL
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate requires a non-NIL right child");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();
        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match &x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_is_left_x = xp
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &x));
                if xp_is_left_x {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.left = x; x.parent = y
        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y.clone());
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            // Break Rc cycles by removing parent/child references
            let left = n.borrow_mut().left.take();
            let right = n.borrow_mut().right.take();
            n.borrow_mut().parent = None;
            Self::free_node(left);
            Self::free_node(right);
        }
    }

    /// Deletes the Red-Black Tree safely.
    pub fn delete_rbtree(self) {
        Self::free_node(self.root);
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;
        loop {
            // Loop condition: z.parent.color == RED
            let parent_opt = z.borrow().parent.clone();
            let parent = match parent_opt {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }

            // Grandparent must exist because parent is RED so it can't be the root
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break,
            };

            // Determine if parent is the left child of grandparent
            let parent_is_left = grandparent
                .borrow()
                .left
                .as_ref()
                .map_or(false, |l| Rc::ptr_eq(l, &parent));

            if parent_is_left {
                let y = grandparent.borrow().right.clone();
                let y_is_red = y
                    .as_ref()
                    .map_or(false, |n| n.borrow().color == Color::Red);

                if y_is_red {
                    // Case 1: uncle is RED
                    parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // Cases 2 & 3: uncle is BLACK
                    let z_is_right = parent
                        .borrow()
                        .right
                        .as_ref()
                        .map_or(false, |r| Rc::ptr_eq(r, &z));

                    let mut z_curr = z.clone();
                    if z_is_right {
                        // Case 2 -> Case 3
                        z_curr = parent.clone();
                        self.left_rotate(z_curr.clone());
                    }
                    // Case 3
                    let p_now = z_curr.borrow().parent.clone().unwrap();
                    let gp_now = p_now.borrow().parent.clone().unwrap();
                    p_now.borrow_mut().color = Color::Black;
                    gp_now.borrow_mut().color = Color::Red;
                    self.right_rotate(gp_now);
                    z = z_curr;
                }
            } else {
                let y = grandparent.borrow().left.clone();
                let y_is_red = y
                    .as_ref()
                    .map_or(false, |n| n.borrow().color == Color::Red);

                if y_is_red {
                    // Mirror case 1
                    parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    let z_is_left = parent
                        .borrow()
                        .left
                        .as_ref()
                        .map_or(false, |l| Rc::ptr_eq(l, &z));

                    let mut z_curr = z.clone();
                    if z_is_left {
                        z_curr = parent.clone();
                        self.right_rotate(z_curr.clone());
                    }
                    let p_now = z_curr.borrow().parent.clone().unwrap();
                    let gp_now = p_now.borrow().parent.clone().unwrap();
                    p_now.borrow_mut().color = Color::Black;
                    gp_now.borrow_mut().color = Color::Red;
                    self.left_rotate(gp_now);
                    z = z_curr;
                }
            }
        }

        if let Some(root) = self.root.clone() {
            root.borrow_mut().color = Color::Black;
        }
    }

    /// Inserts a new key and returns the inserted node.
    pub fn rbtree_insert(&mut self, key: Key) -> Option<NodeRef> {
        let z = Rc::new(RefCell::new(Node {
            key,
            color: Color::Red,
            left: None,
            right: None,
            parent: None,
        }));

        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();
        while let Some(curr) = x {
            y = Some(curr.clone());
            if key < curr.borrow().key {
                x = curr.borrow().left.clone();
            } else {
                x = curr.borrow().right.clone();
            }
        }

        z.borrow_mut().parent = y.clone();

        match y {
            None => {
                self.root = Some(z.clone());
            }
            Some(yp) => {
                if key < yp.borrow().key {
                    yp.borrow_mut().left = Some(z.clone());
                } else {
                    yp.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // z is initialized RED with NIL children already.
        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(curr) = current {
            let curr_key = curr.borrow().key;
            if curr_key == key {
                return Some(curr);
            }
            if curr_key < key {
                current = curr.borrow().right.clone();
            } else {
                current = curr.borrow().left.clone();
            }
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let left = curr.borrow().left.clone();
            match left {
                Some(n) => curr = n,
                None => return Some(curr),
            }
        }
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let right = curr.borrow().right.clone();
            match right {
                Some(n) => curr = n,
                None => return Some(curr),
            }
        }
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let up = u.borrow().parent.clone();
        match &up {
            None => {
                self.root = v.clone();
            }
            Some(p) => {
                let u_is_left = p
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &u));
                if u_is_left {
                    p.borrow_mut().left = v.clone();
                } else {
                    p.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(v_node) = v {
            v_node.borrow_mut().parent = up;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    /// Uses an internal sentinel-based representation to handle the NIL parent
    /// invariant required by the classic CLRS deletion-fixup algorithm.
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let nil = Self::make_nil();
        self.to_sentinel(&nil);
        let x_node = match x {
            Some(n) => n,
            None => nil.clone(),
        };
        self.delete_fixup_s(x_node, &nil);
        self.from_sentinel(&nil);
        nil.borrow_mut().parent = None;
        nil.borrow_mut().left = None;
        nil.borrow_mut().right = None;
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let nil = Self::make_nil();
        self.to_sentinel(&nil);
        self.erase_with_sentinel(p, &nil);
        self.from_sentinel(&nil);
        // Drop nil cleanly to avoid Rc cycles
        nil.borrow_mut().parent = None;
        nil.borrow_mut().left = None;
        nil.borrow_mut().right = None;
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(
        &self,
        curr: Option<NodeRef>,
        arr: &mut Vec<Key>,
        n: usize,
        count: &mut usize,
    ) {
        let curr = match curr {
            Some(c) => c,
            None => return,
        };

        let left = curr.borrow().left.clone();
        self.subtree_to_array(left, arr, n, count);

        if *count < n {
            arr.push(curr.borrow().key);
            *count += 1;
        } else {
            return;
        }

        let right = curr.borrow().right.clone();
        self.subtree_to_array(right, arr, n, count);
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr: Vec<Key> = Vec::with_capacity(n);
        if self.root.is_none() {
            return arr;
        }
        let mut count: usize = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }

    // ------------------------------------------------------------------
    // Internal helpers using a sentinel NIL node.
    // ------------------------------------------------------------------

    fn make_nil() -> NodeRef {
        Rc::new(RefCell::new(Node {
            key: 0,
            color: Color::Black,
            left: None,
            right: None,
            parent: None,
        }))
    }

    /// Convert all `None` slots in the tree to `Some(nil.clone())`.
    fn to_sentinel(&mut self, nil: &NodeRef) {
        match self.root.clone() {
            None => {
                self.root = Some(nil.clone());
            }
            Some(root) => {
                if root.borrow().parent.is_none() {
                    root.borrow_mut().parent = Some(nil.clone());
                }
                Self::to_sentinel_rec(&root, nil);
            }
        }
    }

    fn to_sentinel_rec(node: &NodeRef, nil: &NodeRef) {
        if Rc::ptr_eq(node, nil) {
            return;
        }
        // Replace None children with nil
        {
            let mut n = node.borrow_mut();
            if n.left.is_none() {
                n.left = Some(nil.clone());
            }
            if n.right.is_none() {
                n.right = Some(nil.clone());
            }
        }
        let l = node.borrow().left.clone().unwrap();
        let r = node.borrow().right.clone().unwrap();
        if !Rc::ptr_eq(&l, nil) {
            Self::to_sentinel_rec(&l, nil);
        }
        if !Rc::ptr_eq(&r, nil) {
            Self::to_sentinel_rec(&r, nil);
        }
    }

    /// Convert all `Some(nil)` references back to `None`.
    fn from_sentinel(&mut self, nil: &NodeRef) {
        if let Some(root) = self.root.clone() {
            if Rc::ptr_eq(&root, nil) {
                self.root = None;
                return;
            }
            // Root's parent should be cleared if it points to nil
            if let Some(p) = root.borrow().parent.clone() {
                if Rc::ptr_eq(&p, nil) {
                    root.borrow_mut().parent = None;
                }
            }
            Self::from_sentinel_rec(&root, nil);
        }
    }

    fn from_sentinel_rec(node: &NodeRef, nil: &NodeRef) {
        // Clear nil children to None.
        let (l_opt, r_opt) = {
            let mut n = node.borrow_mut();
            if let Some(l) = n.left.clone() {
                if Rc::ptr_eq(&l, nil) {
                    n.left = None;
                }
            }
            if let Some(r) = n.right.clone() {
                if Rc::ptr_eq(&r, nil) {
                    n.right = None;
                }
            }
            (n.left.clone(), n.right.clone())
        };
        if let Some(l) = l_opt {
            Self::from_sentinel_rec(&l, nil);
        }
        if let Some(r) = r_opt {
            Self::from_sentinel_rec(&r, nil);
        }
    }

    /// Sentinel-aware left rotation.
    fn left_rotate_s(&mut self, x: &NodeRef, nil: &NodeRef) {
        let y = x.borrow().right.clone().unwrap();
        // x.right = y.left
        let yl = y.borrow().left.clone().unwrap();
        x.borrow_mut().right = Some(yl.clone());
        if !Rc::ptr_eq(&yl, nil) {
            yl.borrow_mut().parent = Some(x.clone());
        }
        // y.parent = x.parent
        let xp = x.borrow().parent.clone().unwrap();
        y.borrow_mut().parent = Some(xp.clone());
        if Rc::ptr_eq(&xp, nil) {
            self.root = Some(y.clone());
        } else {
            let xpl = xp.borrow().left.clone().unwrap();
            if Rc::ptr_eq(&xpl, x) {
                xp.borrow_mut().left = Some(y.clone());
            } else {
                xp.borrow_mut().right = Some(y.clone());
            }
        }
        // y.left = x; x.parent = y
        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y.clone());
    }

    /// Sentinel-aware right rotation.
    fn right_rotate_s(&mut self, x: &NodeRef, nil: &NodeRef) {
        let y = x.borrow().left.clone().unwrap();
        let yr = y.borrow().right.clone().unwrap();
        x.borrow_mut().left = Some(yr.clone());
        if !Rc::ptr_eq(&yr, nil) {
            yr.borrow_mut().parent = Some(x.clone());
        }
        let xp = x.borrow().parent.clone().unwrap();
        y.borrow_mut().parent = Some(xp.clone());
        if Rc::ptr_eq(&xp, nil) {
            self.root = Some(y.clone());
        } else {
            let xpl = xp.borrow().left.clone().unwrap();
            if Rc::ptr_eq(&xpl, x) {
                xp.borrow_mut().left = Some(y.clone());
            } else {
                xp.borrow_mut().right = Some(y.clone());
            }
        }
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y.clone());
    }

    /// Sentinel-aware transplant.
    fn transplant_s(&mut self, u: &NodeRef, v: &NodeRef, nil: &NodeRef) {
        let up = u.borrow().parent.clone().unwrap();
        if Rc::ptr_eq(&up, nil) {
            self.root = Some(v.clone());
        } else {
            let upl = up.borrow().left.clone().unwrap();
            if Rc::ptr_eq(&upl, u) {
                up.borrow_mut().left = Some(v.clone());
            } else {
                up.borrow_mut().right = Some(v.clone());
            }
        }
        v.borrow_mut().parent = Some(up.clone());
    }

    fn is_root_s(&self, x: &NodeRef) -> bool {
        self.root.as_ref().map_or(false, |r| Rc::ptr_eq(r, x))
    }

    /// Sentinel-aware delete fixup.
    fn delete_fixup_s(&mut self, x: NodeRef, nil: &NodeRef) {
        let mut x = x;
        loop {
            if self.is_root_s(&x) {
                break;
            }
            if x.borrow().color != Color::Black {
                break;
            }

            let xp = x.borrow().parent.clone().unwrap();
            let xpl = xp.borrow().left.clone().unwrap();
            let x_is_left = Rc::ptr_eq(&xpl, &x);

            if x_is_left {
                // Left case
                let mut w = xp.borrow().right.clone().unwrap();
                // Case 1
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    xp.borrow_mut().color = Color::Red;
                    self.left_rotate_s(&xp, nil);
                    let xp2 = x.borrow().parent.clone().unwrap();
                    w = xp2.borrow().right.clone().unwrap();
                }
                let wl_color = {
                    let wl = w.borrow().left.clone().unwrap();
                    let c = wl.borrow().color.clone();
                    c
                };
                let wr_color = {
                    let wr = w.borrow().right.clone().unwrap();
                    let c = wr.borrow().color.clone();
                    c
                };
                if wl_color == Color::Black && wr_color == Color::Black {
                    // Case 2
                    w.borrow_mut().color = Color::Red;
                    let new_x = x.borrow().parent.clone().unwrap();
                    x = new_x;
                } else {
                    // Case 3 / 4
                    let wr_color_now = {
                        let wr = w.borrow().right.clone().unwrap();
                        let c = wr.borrow().color.clone();
                        c
                    };
                    if wr_color_now == Color::Black {
                        let wl = w.borrow().left.clone().unwrap();
                        wl.borrow_mut().color = Color::Black;
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate_s(&w, nil);
                        let xp2 = x.borrow().parent.clone().unwrap();
                        w = xp2.borrow().right.clone().unwrap();
                    }
                    // Case 4
                    let xp_now = x.borrow().parent.clone().unwrap();
                    let xp_color = xp_now.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    xp_now.borrow_mut().color = Color::Black;
                    let wr = w.borrow().right.clone().unwrap();
                    wr.borrow_mut().color = Color::Black;
                    self.left_rotate_s(&xp_now, nil);
                    x = self.root.clone().unwrap();
                }
            } else {
                // Right case (mirror)
                let mut w = xp.borrow().left.clone().unwrap();
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    xp.borrow_mut().color = Color::Red;
                    self.right_rotate_s(&xp, nil);
                    let xp2 = x.borrow().parent.clone().unwrap();
                    w = xp2.borrow().left.clone().unwrap();
                }
                let wl_color = {
                    let wl = w.borrow().left.clone().unwrap();
                    let c = wl.borrow().color.clone();
                    c
                };
                let wr_color = {
                    let wr = w.borrow().right.clone().unwrap();
                    let c = wr.borrow().color.clone();
                    c
                };
                if wr_color == Color::Black && wl_color == Color::Black {
                    w.borrow_mut().color = Color::Red;
                    let new_x = x.borrow().parent.clone().unwrap();
                    x = new_x;
                } else {
                    let wl_color_now = {
                        let wl = w.borrow().left.clone().unwrap();
                        let c = wl.borrow().color.clone();
                        c
                    };
                    if wl_color_now == Color::Black {
                        let wr = w.borrow().right.clone().unwrap();
                        wr.borrow_mut().color = Color::Black;
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate_s(&w, nil);
                        let xp2 = x.borrow().parent.clone().unwrap();
                        w = xp2.borrow().left.clone().unwrap();
                    }
                    let xp_now = x.borrow().parent.clone().unwrap();
                    let xp_color = xp_now.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    xp_now.borrow_mut().color = Color::Black;
                    let wl = w.borrow().left.clone().unwrap();
                    wl.borrow_mut().color = Color::Black;
                    self.right_rotate_s(&xp_now, nil);
                    x = self.root.clone().unwrap();
                }
            }
        }

        x.borrow_mut().color = Color::Black;
    }

    /// Sentinel-aware erase implementation.
    fn erase_with_sentinel(&mut self, p: NodeRef, nil: &NodeRef) {
        let mut y = p.clone();
        let mut y_orig_color = y.borrow().color.clone();

        let p_left = p.borrow().left.clone().unwrap();
        let p_right = p.borrow().right.clone().unwrap();

        let x: NodeRef;

        if Rc::ptr_eq(&p_left, nil) {
            x = p_right.clone();
            self.transplant_s(&p, &p_right, nil);
        } else if Rc::ptr_eq(&p_right, nil) {
            x = p_left.clone();
            self.transplant_s(&p, &p_left, nil);
        } else {
            // Find min of right subtree
            y = p_right.clone();
            loop {
                let yl = y.borrow().left.clone().unwrap();
                if Rc::ptr_eq(&yl, nil) {
                    break;
                }
                y = yl;
            }
            y_orig_color = y.borrow().color.clone();
            x = y.borrow().right.clone().unwrap();

            let yp = y.borrow().parent.clone().unwrap();
            if Rc::ptr_eq(&yp, &p) {
                x.borrow_mut().parent = Some(y.clone());
            } else {
                let yr = y.borrow().right.clone().unwrap();
                self.transplant_s(&y, &yr, nil);
                let pr = p.borrow().right.clone().unwrap();
                y.borrow_mut().right = Some(pr.clone());
                pr.borrow_mut().parent = Some(y.clone());
            }

            self.transplant_s(&p, &y, nil);
            let pl = p.borrow().left.clone().unwrap();
            y.borrow_mut().left = Some(pl.clone());
            pl.borrow_mut().parent = Some(y.clone());
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        if y_orig_color == Color::Black {
            self.delete_fixup_s(x, nil);
        }

        // Detach removed node
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;
        p.borrow_mut().parent = None;
    }
}
