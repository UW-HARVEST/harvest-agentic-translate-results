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

// Helpers for treating None as NIL with implicit Black color
fn color_of(node: &Option<NodeRef>) -> Color {
    match node {
        Some(n) => n.borrow().color.clone(),
        None => Color::Black,
    }
}

fn nodes_eq(a: &Option<NodeRef>, b: &Option<NodeRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

fn opt_ptr_eq(a: &Option<NodeRef>, b: &NodeRef) -> bool {
    match a {
        Some(x) => Rc::ptr_eq(x, b),
        None => false,
    }
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        // y = x.left
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate: x.left must be non-NIL");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y.right != NIL: y.right.parent = x
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        // Update parent's child pointer (or root)
        match &x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_left_eq_x = opt_ptr_eq(&xp.borrow().left, &x);
                if xp_left_eq_x {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.right = x; x.parent = y
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x.right
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate: x.right must be non-NIL");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y.left != NIL: y.left.parent = x
        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        // Update parent's child pointer (or root)
        match &x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_left_eq_x = opt_ptr_eq(&xp.borrow().left, &x);
                if xp_left_eq_x {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.left = x; x.parent = y
        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            let left = n.borrow_mut().left.take();
            let right = n.borrow_mut().right.take();
            // Break the strong-reference parent cycle so Rc can drop.
            n.borrow_mut().parent = None;
            Self::free_node(left);
            Self::free_node(right);
        }
    }

    /// Deletes the Red-Black Tree safely.
    pub fn delete_rbtree(mut self) {
        Self::free_node(self.root.take());
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;
        loop {
            // Get z's parent; if None or Black, stop.
            let z_parent = z.borrow().parent.clone();
            let z_parent = match z_parent {
                Some(p) => p,
                None => break,
            };
            if z_parent.borrow().color != Color::Red {
                break;
            }

            // Grandparent must exist since parent is Red (root is Black).
            let z_grand = z_parent
                .borrow()
                .parent
                .clone()
                .expect("rbtree_insert_fixup: grandparent must exist when parent is Red");

            // Determine if parent is the left child of grandparent.
            let parent_is_left = opt_ptr_eq(&z_grand.borrow().left, &z_parent);

            if parent_is_left {
                let y = z_grand.borrow().right.clone();
                if color_of(&y) == Color::Red {
                    // CASE 1
                    z_parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    z_grand.borrow_mut().color = Color::Red;
                    z = z_grand;
                } else {
                    // y is Black (or NIL)
                    let z_is_right = opt_ptr_eq(&z_parent.borrow().right, &z);
                    let mut z_local = z;
                    if z_is_right {
                        // CASE 2
                        z_local = z_parent.clone();
                        self.left_rotate(z_local.clone());
                    }
                    // CASE 3
                    let zl_parent = z_local.borrow().parent.clone();
                    if let Some(p) = &zl_parent {
                        p.borrow_mut().color = Color::Black;
                        let gp = p.borrow().parent.clone();
                        if let Some(g) = gp {
                            g.borrow_mut().color = Color::Red;
                            self.right_rotate(g);
                        }
                    }
                    z = z_local;
                }
            } else {
                // Symmetric: parent is right child of grandparent
                let y = z_grand.borrow().left.clone();
                if color_of(&y) == Color::Red {
                    // CASE 4
                    z_parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    z_grand.borrow_mut().color = Color::Red;
                    z = z_grand;
                } else {
                    let z_is_left = opt_ptr_eq(&z_parent.borrow().left, &z);
                    let mut z_local = z;
                    if z_is_left {
                        // CASE 5
                        z_local = z_parent.clone();
                        self.right_rotate(z_local.clone());
                    }
                    // CASE 6
                    let zl_parent = z_local.borrow().parent.clone();
                    if let Some(p) = &zl_parent {
                        p.borrow_mut().color = Color::Black;
                        let gp = p.borrow().parent.clone();
                        if let Some(g) = gp {
                            g.borrow_mut().color = Color::Red;
                            self.left_rotate(g);
                        }
                    }
                    z = z_local;
                }
            }
        }
        if let Some(root) = &self.root {
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

        // Find the parent y for z.
        let mut y: Option<NodeRef> = None;
        let mut x: Option<NodeRef> = self.root.clone();
        while let Some(xn) = x {
            y = Some(xn.clone());
            let xn_key = xn.borrow().key;
            if key < xn_key {
                x = xn.borrow().left.clone();
            } else {
                x = xn.borrow().right.clone();
            }
        }

        z.borrow_mut().parent = y.clone();

        match &y {
            None => {
                self.root = Some(z.clone());
            }
            Some(yn) => {
                let yn_key = yn.borrow().key;
                if key < yn_key {
                    yn.borrow_mut().left = Some(z.clone());
                } else {
                    yn.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // z's left/right are already None; color is Red.
        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(curr) = current {
            let ck = curr.borrow().key;
            if ck == key {
                return Some(curr);
            }
            if ck < key {
                let next = curr.borrow().right.clone();
                current = next;
            } else {
                let next = curr.borrow().left.clone();
                current = next;
            }
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let l = curr.borrow().left.clone();
            match l {
                Some(ln) => curr = ln,
                None => break,
            }
        }
        Some(curr)
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let r = curr.borrow().right.clone();
            match r {
                Some(rn) => curr = rn,
                None => break,
            }
        }
        Some(curr)
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let u_parent = u.borrow().parent.clone();
        match &u_parent {
            None => {
                self.root = v.clone();
            }
            Some(up) => {
                let up_left_eq_u = opt_ptr_eq(&up.borrow().left, &u);
                if up_left_eq_u {
                    up.borrow_mut().left = v.clone();
                } else {
                    up.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(vn) = &v {
            vn.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    /// Note: when `x` is `None`, this function is a no-op since it cannot
    /// access the parent of a NIL position. Callers (i.e. `erase`) should use
    /// a phantom-NIL node to allow fixup to operate on an empty position.
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let mut x = match x {
            Some(n) => n,
            None => return,
        };

        loop {
            // Stop if x is the root.
            let is_root = match &self.root {
                Some(r) => Rc::ptr_eq(r, &x),
                None => false,
            };
            if is_root {
                break;
            }
            // Stop if x is Red (we'll color it Black below).
            if x.borrow().color != Color::Black {
                break;
            }

            let x_parent = match x.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };

            let x_is_left = opt_ptr_eq(&x_parent.borrow().left, &x);

            if x_is_left {
                let mut w = match x_parent.borrow().right.clone() {
                    Some(w) => w,
                    None => break, // shouldn't happen in a valid RB tree under fixup
                };

                // CASE 1: sibling is Red
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    x_parent.borrow_mut().color = Color::Red;
                    self.left_rotate(x_parent.clone());
                    w = match x_parent.borrow().right.clone() {
                        Some(w2) => w2,
                        None => break,
                    };
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                if color_of(&w_left) == Color::Black && color_of(&w_right) == Color::Black {
                    // CASE 2
                    w.borrow_mut().color = Color::Red;
                    x = x_parent.clone();
                } else {
                    if color_of(&w_right) == Color::Black {
                        // CASE 3
                        if let Some(wl) = &w_left {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = match x_parent.borrow().right.clone() {
                            Some(w2) => w2,
                            None => break,
                        };
                    }
                    // CASE 4
                    let parent_color = x_parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    x_parent.borrow_mut().color = Color::Black;
                    let w_right2 = w.borrow().right.clone();
                    if let Some(wr) = &w_right2 {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(x_parent.clone());
                    x = match self.root.clone() {
                        Some(r) => r,
                        None => break,
                    };
                }
            } else {
                let mut w = match x_parent.borrow().left.clone() {
                    Some(w) => w,
                    None => break,
                };

                // CASE 5: sibling is Red
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    x_parent.borrow_mut().color = Color::Red;
                    self.right_rotate(x_parent.clone());
                    w = match x_parent.borrow().left.clone() {
                        Some(w2) => w2,
                        None => break,
                    };
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                if color_of(&w_right) == Color::Black && color_of(&w_left) == Color::Black {
                    // CASE 6
                    w.borrow_mut().color = Color::Red;
                    x = x_parent.clone();
                } else {
                    if color_of(&w_left) == Color::Black {
                        // CASE 7
                        if let Some(wr) = &w_right {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = match x_parent.borrow().left.clone() {
                            Some(w2) => w2,
                            None => break,
                        };
                    }
                    // CASE 8
                    let parent_color = x_parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    x_parent.borrow_mut().color = Color::Black;
                    let w_left2 = w.borrow().left.clone();
                    if let Some(wl) = &w_left2 {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(x_parent.clone());
                    x = match self.root.clone() {
                        Some(r) => r,
                        None => break,
                    };
                }
            }
        }

        x.borrow_mut().color = Color::Black;
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let mut y = p.clone();
        let mut y_original_color = y.borrow().color.clone();

        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        // x will be the node that takes y's position; x_parent its parent.
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p_left.is_none() {
            x = p_right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            x = p_left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_left);
        } else {
            // Find min of right subtree.
            let mut y_node = p_right.clone().unwrap();
            loop {
                let l = y_node.borrow().left.clone();
                match l {
                    Some(ln) => y_node = ln,
                    None => break,
                }
            }
            y = y_node.clone();
            y_original_color = y.borrow().color.clone();
            let y_right = y.borrow().right.clone();
            x = y_right.clone();

            let y_parent = y.borrow().parent.clone();
            let y_parent_eq_p = match &y_parent {
                Some(yp) => Rc::ptr_eq(yp, &p),
                None => false,
            };

            if y_parent_eq_p {
                // x's parent stays as y.
                if let Some(xn) = &x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                self.transplant(y.clone(), y_right.clone());
                // y.right = p.right; y.right.parent = y
                y.borrow_mut().right = p_right.clone();
                if let Some(pr) = &p_right {
                    pr.borrow_mut().parent = Some(y.clone());
                }
                // After transplant, x's parent is what was y's parent.
                x_parent = y_parent;
            }

            self.transplant(p.clone(), Some(y.clone()));
            // y.left = p.left; y.left.parent = y
            y.borrow_mut().left = p_left.clone();
            if let Some(pl) = &p_left {
                pl.borrow_mut().parent = Some(y.clone());
            }
            // y.color = p.color
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        if y_original_color == Color::Black {
            // If x is None, install a phantom NIL so fixup can navigate via x.parent.
            let phantom: Option<NodeRef> = if x.is_none() {
                let ph = Rc::new(RefCell::new(Node {
                    key: 0,
                    color: Color::Black,
                    left: None,
                    right: None,
                    parent: x_parent.clone(),
                }));
                // Attach ph to the empty slot under x_parent (or as root).
                match &x_parent {
                    None => {
                        self.root = Some(ph.clone());
                    }
                    Some(xp) => {
                        let left_empty = xp.borrow().left.is_none();
                        let right_empty = xp.borrow().right.is_none();
                        // Prefer the side that is currently empty.
                        if left_empty && !right_empty {
                            xp.borrow_mut().left = Some(ph.clone());
                        } else if right_empty && !left_empty {
                            xp.borrow_mut().right = Some(ph.clone());
                        } else if left_empty && right_empty {
                            // Both empty; place on left by convention.
                            xp.borrow_mut().left = Some(ph.clone());
                        } else {
                            // Neither empty; this shouldn't happen because x was None.
                            // Fall back to right.
                            xp.borrow_mut().right = Some(ph.clone());
                        }
                    }
                }
                Some(ph)
            } else {
                None
            };

            let x_for_fixup = match &phantom {
                Some(ph) => Some(ph.clone()),
                None => x.clone(),
            };

            self.delete_fixup(x_for_fixup);

            // Detach phantom from the tree.
            if let Some(ph) = phantom {
                let ph_parent = ph.borrow().parent.clone();
                match ph_parent {
                    None => {
                        // phantom is root
                        if let Some(r) = self.root.clone() {
                            if Rc::ptr_eq(&r, &ph) {
                                self.root = None;
                            }
                        }
                    }
                    Some(pp) => {
                        let pp_left_eq_ph = opt_ptr_eq(&pp.borrow().left, &ph);
                        if pp_left_eq_ph {
                            pp.borrow_mut().left = None;
                        } else {
                            let pp_right_eq_ph = opt_ptr_eq(&pp.borrow().right, &ph);
                            if pp_right_eq_ph {
                                pp.borrow_mut().right = None;
                            }
                        }
                    }
                }
                // Break parent reference of phantom.
                ph.borrow_mut().parent = None;
            }
        }

        // Detach p from the tree (break references) so it can be dropped.
        p.borrow_mut().parent = None;
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;

        // Suppress unused warnings.
        let _ = y;
        let _ = nodes_eq;
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
        let mut arr: Vec<Key> = Vec::new();
        if self.root.is_none() {
            return arr;
        }
        let mut count: usize = 0;
        let root = self.root.clone();
        self.subtree_to_array(root, &mut arr, n, &mut count);
        arr
    }
}
