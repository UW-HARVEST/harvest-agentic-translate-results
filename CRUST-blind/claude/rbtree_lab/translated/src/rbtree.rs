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
        // y = x.left  (must be non-NIL)
        let y = x.borrow().left.clone().expect("right_rotate: x.left must be non-NIL");

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

        // Update x's old parent's pointer (or set root)
        match &x_parent {
            None => self.root = Some(y.clone()),
            Some(xp) => {
                let xp_left = xp.borrow().left.clone();
                let x_is_left = match xp_left {
                    Some(ref ln) => Rc::ptr_eq(ln, &x),
                    None => false,
                };
                if x_is_left {
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
        // y = x.right  (must be non-NIL)
        let y = x.borrow().right.clone().expect("left_rotate: x.right must be non-NIL");

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

        // Update x's old parent's pointer (or set root)
        match &x_parent {
            None => self.root = Some(y.clone()),
            Some(xp) => {
                let xp_left = xp.borrow().left.clone();
                let x_is_left = match xp_left {
                    Some(ref ln) => Rc::ptr_eq(ln, &x),
                    None => false,
                };
                if x_is_left {
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
            Self::free_node(left);
            Self::free_node(right);
            // Break parent cycle
            n.borrow_mut().parent = None;
        }
    }

    /// Deletes the Red-Black Tree safely.
    pub fn delete_rbtree(mut self) {
        Self::free_node(self.root.take());
        // self drops here
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;
        loop {
            // Read z's parent
            let zp = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            // Check parent's color
            if zp.borrow().color != Color::Red {
                break;
            }
            // Get grandparent (z's parent is red, so it's not the root, so grandparent exists)
            let zpp = match zp.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };

            // Determine if zp is the left child of zpp
            let zpp_left = zpp.borrow().left.clone();
            let zp_is_left = match zpp_left {
                Some(ref ln) => Rc::ptr_eq(ln, &zp),
                None => false,
            };

            if zp_is_left {
                let y = zpp.borrow().right.clone();
                let y_color = match &y {
                    Some(yn) => yn.borrow().color.clone(),
                    None => Color::Black,
                };

                if y_color == Color::Red {
                    // Case 1: uncle is red
                    zp.borrow_mut().color = Color::Black;
                    if let Some(yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp;
                } else {
                    // Determine if z is right child of zp
                    let zp_right = zp.borrow().right.clone();
                    let z_is_right = match zp_right {
                        Some(ref rn) => Rc::ptr_eq(rn, &z),
                        None => false,
                    };

                    if z_is_right {
                        // Case 2: z is a right child -> rotate left around parent
                        z = zp.clone();
                        self.left_rotate(z.clone());
                    }

                    // Case 3
                    let zp_now = z.borrow().parent.clone().expect("z must have parent in case 3");
                    zp_now.borrow_mut().color = Color::Black;
                    let zpp_now = zp_now.borrow().parent.clone().expect("z must have grandparent in case 3");
                    zpp_now.borrow_mut().color = Color::Red;
                    self.right_rotate(zpp_now);
                }
            } else {
                // Symmetric: zp is the right child of zpp
                let y = zpp.borrow().left.clone();
                let y_color = match &y {
                    Some(yn) => yn.borrow().color.clone(),
                    None => Color::Black,
                };

                if y_color == Color::Red {
                    zp.borrow_mut().color = Color::Black;
                    if let Some(yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp;
                } else {
                    let zp_left = zp.borrow().left.clone();
                    let z_is_left = match zp_left {
                        Some(ref ln) => Rc::ptr_eq(ln, &z),
                        None => false,
                    };

                    if z_is_left {
                        z = zp.clone();
                        self.right_rotate(z.clone());
                    }

                    let zp_now = z.borrow().parent.clone().expect("z must have parent in case 3");
                    zp_now.borrow_mut().color = Color::Black;
                    let zpp_now = zp_now.borrow().parent.clone().expect("z must have grandparent in case 3");
                    zpp_now.borrow_mut().color = Color::Red;
                    self.left_rotate(zpp_now);
                }
            }
        }

        // Root must be black
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

        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();

        while let Some(xn) = x {
            y = Some(xn.clone());
            let xkey = xn.borrow().key;
            x = if key < xkey {
                xn.borrow().left.clone()
            } else {
                xn.borrow().right.clone()
            };
        }

        z.borrow_mut().parent = y.clone();

        match y {
            None => self.root = Some(z.clone()),
            Some(yn) => {
                let ykey = yn.borrow().key;
                if key < ykey {
                    yn.borrow_mut().left = Some(z.clone());
                } else {
                    yn.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // z.left = NIL, z.right = NIL, z.color = RED already set above
        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(c) = current {
            let ckey = c.borrow().key;
            if ckey == key {
                return Some(c);
            }
            current = if ckey < key {
                c.borrow().right.clone()
            } else {
                c.borrow().left.clone()
            };
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let next = curr.borrow().left.clone();
            match next {
                Some(n) => curr = n,
                None => return Some(curr),
            }
        }
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let next = curr.borrow().right.clone();
            match next {
                Some(n) => curr = n,
                None => return Some(curr),
            }
        }
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let u_parent = u.borrow().parent.clone();
        match &u_parent {
            None => self.root = v.clone(),
            Some(up) => {
                let up_left = up.borrow().left.clone();
                let u_is_left = match up_left {
                    Some(ref ln) => Rc::ptr_eq(ln, &u),
                    None => false,
                };
                if u_is_left {
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
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        // Read x's parent for the initial call. If x is None we have no way to
        // know its parent through this public API, so default to None.
        let x_parent = match &x {
            Some(n) => n.borrow().parent.clone(),
            None => None,
        };
        self.delete_fixup_internal(x, x_parent);
    }

    /// Internal delete-fixup that tracks parent explicitly so it works correctly
    /// when `x` is NIL (`None`).
    fn delete_fixup_internal(&mut self, mut x: Option<NodeRef>, mut x_parent: Option<NodeRef>) {
        loop {
            // x != root?
            let is_root = match (&x, &self.root) {
                (Some(xn), Some(rn)) => Rc::ptr_eq(xn, rn),
                (None, None) => true,
                _ => false,
            };
            if is_root {
                break;
            }

            // x.color != BLACK?
            let x_color = match &x {
                Some(xn) => xn.borrow().color.clone(),
                None => Color::Black,
            };
            if x_color != Color::Black {
                break;
            }

            let xp = match &x_parent {
                Some(p) => p.clone(),
                None => break, // shouldn't happen if invariants hold
            };

            // Determine if x is the left child of xp
            let xp_left = xp.borrow().left.clone();
            let is_left = match (&x, &xp_left) {
                (Some(xn), Some(ln)) => Rc::ptr_eq(xn, ln),
                (None, None) => true,
                _ => false,
            };

            if is_left {
                // Sibling w must exist (otherwise black-height invariant violated)
                let mut w = xp.borrow().right.clone().expect("sibling must exist");

                // CASE 1: w is red
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    xp.borrow_mut().color = Color::Red;
                    self.left_rotate(xp.clone());
                    w = xp.borrow().right.clone().expect("sibling must exist after rotation");
                }

                // Children colors of w
                let w_left_color = match w.borrow().left.clone() {
                    Some(n) => n.borrow().color.clone(),
                    None => Color::Black,
                };
                let w_right_color = match w.borrow().right.clone() {
                    Some(n) => n.borrow().color.clone(),
                    None => Color::Black,
                };

                if w_left_color == Color::Black && w_right_color == Color::Black {
                    // CASE 2: both w's children are black
                    w.borrow_mut().color = Color::Red;
                    let new_x_parent = xp.borrow().parent.clone();
                    x = Some(xp.clone());
                    x_parent = new_x_parent;
                } else {
                    // CASE 3: w right child is black (so w left child is red)
                    if w_right_color == Color::Black {
                        if let Some(wl) = w.borrow().left.clone() {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = xp.borrow().right.clone().expect("sibling must exist after rotation");
                    }

                    // CASE 4: w right child is red
                    let xp_color = xp.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    xp.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(xp.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            } else {
                // Symmetric case: x is right child
                let mut w = xp.borrow().left.clone().expect("sibling must exist");

                // CASE 5: w is red
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    xp.borrow_mut().color = Color::Red;
                    self.right_rotate(xp.clone());
                    w = xp.borrow().left.clone().expect("sibling must exist after rotation");
                }

                let w_left_color = match w.borrow().left.clone() {
                    Some(n) => n.borrow().color.clone(),
                    None => Color::Black,
                };
                let w_right_color = match w.borrow().right.clone() {
                    Some(n) => n.borrow().color.clone(),
                    None => Color::Black,
                };

                if w_left_color == Color::Black && w_right_color == Color::Black {
                    // CASE 6
                    w.borrow_mut().color = Color::Red;
                    let new_x_parent = xp.borrow().parent.clone();
                    x = Some(xp.clone());
                    x_parent = new_x_parent;
                } else {
                    // CASE 7: w left child is black (so w right child is red)
                    if w_left_color == Color::Black {
                        if let Some(wr) = w.borrow().right.clone() {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = xp.borrow().left.clone().expect("sibling must exist after rotation");
                    }

                    // CASE 8: w left child is red
                    let xp_color = xp.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    xp.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(xp.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            }
        }

        if let Some(xn) = &x {
            xn.borrow_mut().color = Color::Black;
        }
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let mut y = p.clone();
        let mut y_original_color = y.borrow().color.clone();

        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p_left.is_none() {
            // p has no left child
            x = p_right.clone();
            let pp = p.borrow().parent.clone();
            self.transplant(p.clone(), p_right);
            x_parent = pp;
        } else if p_right.is_none() {
            // p has no right child
            x = p_left.clone();
            let pp = p.borrow().parent.clone();
            self.transplant(p.clone(), p_left);
            x_parent = pp;
        } else {
            // p has both children: find successor
            let mut yt = p.borrow().right.clone().expect("right child exists");
            loop {
                let next = yt.borrow().left.clone();
                match next {
                    Some(n) => yt = n,
                    None => break,
                }
            }
            y = yt;
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            let y_parent = y.borrow().parent.clone();
            let yp_eq_p = match &y_parent {
                Some(yp) => Rc::ptr_eq(yp, &p),
                None => false,
            };

            if yp_eq_p {
                // y is direct child of p
                if let Some(ref xn) = x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                // y is deeper; transplant y with y.right (== x)
                let y_right = y.borrow().right.clone();
                self.transplant(y.clone(), y_right);
                // After transplant: x's parent (if Some) is now y_parent.
                // We track x_parent as y_parent for the case x is None too.
                x_parent = y_parent;

                // y.right = p.right; y.right.parent = y
                let pr = p.borrow().right.clone();
                y.borrow_mut().right = pr.clone();
                if let Some(pr) = pr {
                    pr.borrow_mut().parent = Some(y.clone());
                }
            }

            // transplant(p, y)
            self.transplant(p.clone(), Some(y.clone()));
            // y.left = p.left; p.left.parent = y
            let pl = p.borrow().left.clone();
            y.borrow_mut().left = pl.clone();
            if let Some(pl) = pl {
                pl.borrow_mut().parent = Some(y.clone());
            }
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        // Detach p from the tree to break cycles and free its memory
        {
            let mut pb = p.borrow_mut();
            pb.parent = None;
            pb.left = None;
            pb.right = None;
        }

        if y_original_color == Color::Black {
            self.delete_fixup_internal(x, x_parent);
        }
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(
        &self,
        curr: Option<NodeRef>,
        arr: &mut Vec<Key>,
        n: usize,
        count: &mut usize,
    ) {
        if let Some(c) = curr {
            let left = c.borrow().left.clone();
            self.subtree_to_array(left, arr, n, count);
            if *count < n {
                arr.push(c.borrow().key);
                *count += 1;
            } else {
                return;
            }
            let right = c.borrow().right.clone();
            self.subtree_to_array(right, arr, n, count);
        }
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
}

impl Default for RBTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RBTree {
    fn drop(&mut self) {
        // Break parent cycles so nodes can be freed
        Self::free_node(self.root.take());
    }
}
