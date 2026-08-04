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

/// Returns true if the optional node has the given color (None counts as Black).
fn is_black(n: &Option<NodeRef>) -> bool {
    match n {
        None => true,
        Some(node) => node.borrow().color == Color::Black,
    }
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        // y = x.left (must be non-nil)
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate requires x.left to be non-nil");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y.right is non-nil, set its parent to x
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(p) => {
                let p_left = p.borrow().left.clone();
                let is_left = p_left
                    .as_ref()
                    .map_or(false, |pl| Rc::ptr_eq(pl, &x));
                if is_left {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.right = x; x.parent = y
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x.right (must be non-nil)
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate requires x.right to be non-nil");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y.left is non-nil, set its parent to x
        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(p) => {
                let p_left = p.borrow().left.clone();
                let is_left = p_left
                    .as_ref()
                    .map_or(false, |pl| Rc::ptr_eq(pl, &x));
                if is_left {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
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
            n.borrow_mut().parent = None;
            Self::free_node(left);
            Self::free_node(right);
        }
    }

    /// Deletes the Red-Black Tree safely.
    pub fn delete_rbtree(mut self) {
        let root = self.root.take();
        Self::free_node(root);
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;

        loop {
            // Get z's parent
            let parent = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }
            // Grandparent must exist because parent is red and root is black.
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break,
            };

            let gp_left = grandparent.borrow().left.clone();
            let parent_is_left = gp_left
                .as_ref()
                .map_or(false, |gl| Rc::ptr_eq(gl, &parent));

            if parent_is_left {
                // Uncle is grandparent.right
                let uncle = grandparent.borrow().right.clone();
                let uncle_red = uncle
                    .as_ref()
                    .map_or(false, |u| u.borrow().color == Color::Red);

                if uncle_red {
                    // CASE 1
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = &uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 2: z is right child -> left rotate around parent
                    let parent_right = parent.borrow().right.clone();
                    let z_is_right = parent_right
                        .as_ref()
                        .map_or(false, |pr| Rc::ptr_eq(pr, &z));

                    let mut z2 = z.clone();
                    if z_is_right {
                        z2 = parent.clone();
                        self.left_rotate(z2.clone());
                    }
                    // CASE 3
                    let p2 = z2.borrow().parent.clone().expect("parent must exist");
                    p2.borrow_mut().color = Color::Black;
                    let gp2 = p2.borrow().parent.clone().expect("grandparent must exist");
                    gp2.borrow_mut().color = Color::Red;
                    self.right_rotate(gp2);
                    z = z2;
                }
            } else {
                // Mirror: parent is right child of grandparent
                let uncle = grandparent.borrow().left.clone();
                let uncle_red = uncle
                    .as_ref()
                    .map_or(false, |u| u.borrow().color == Color::Red);

                if uncle_red {
                    // CASE 4
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = &uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 5: z is left child -> right rotate around parent
                    let parent_left = parent.borrow().left.clone();
                    let z_is_left = parent_left
                        .as_ref()
                        .map_or(false, |pl| Rc::ptr_eq(pl, &z));

                    let mut z2 = z.clone();
                    if z_is_left {
                        z2 = parent.clone();
                        self.right_rotate(z2.clone());
                    }
                    // CASE 6
                    let p2 = z2.borrow().parent.clone().expect("parent must exist");
                    p2.borrow_mut().color = Color::Black;
                    let gp2 = p2.borrow().parent.clone().expect("grandparent must exist");
                    gp2.borrow_mut().color = Color::Red;
                    self.left_rotate(gp2);
                    z = z2;
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

        // BST insert: walk down the tree
        let mut y: Option<NodeRef> = None;
        let mut x: Option<NodeRef> = self.root.clone();

        while let Some(curr) = x {
            y = Some(curr.clone());
            let go_left = key < curr.borrow().key;
            x = if go_left {
                curr.borrow().left.clone()
            } else {
                curr.borrow().right.clone()
            };
        }

        z.borrow_mut().parent = y.clone();

        match &y {
            None => {
                self.root = Some(z.clone());
            }
            Some(p) => {
                if key < p.borrow().key {
                    p.borrow_mut().left = Some(z.clone());
                } else {
                    p.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // Already initialized: left/right = None, color = Red

        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(node) = current {
            let nk = node.borrow().key;
            if nk == key {
                return Some(node);
            }
            current = if nk < key {
                node.borrow().right.clone()
            } else {
                node.borrow().left.clone()
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
            None => {
                self.root = v.clone();
            }
            Some(up) => {
                let up_left = up.borrow().left.clone();
                let is_left = up_left
                    .as_ref()
                    .map_or(false, |upl| Rc::ptr_eq(upl, &u));
                if is_left {
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

    /// Internal fixup that takes explicit parent because `x` may be NIL (None).
    fn delete_fixup_helper(
        &mut self,
        mut x: Option<NodeRef>,
        mut x_parent: Option<NodeRef>,
    ) {
        loop {
            // Stop if x is the root.
            let is_root = match (&x, &self.root) {
                (Some(xn), Some(rt)) => Rc::ptr_eq(xn, rt),
                _ => false,
            };
            if is_root {
                break;
            }
            // Stop if x is red (None counts as black).
            let x_is_black = is_black(&x);
            if !x_is_black {
                break;
            }
            // We need a parent to do anything.
            let parent = match &x_parent {
                Some(p) => p.clone(),
                None => break,
            };

            let p_left = parent.borrow().left.clone();
            let x_is_left = if let Some(xn) = &x {
                p_left.as_ref().map_or(false, |pl| Rc::ptr_eq(pl, xn))
            } else {
                // x is NIL; x is on the side where parent's child pointer is None.
                p_left.is_none()
            };

            if x_is_left {
                // Sibling is parent.right
                let mut w = parent
                    .borrow()
                    .right
                    .clone()
                    .expect("sibling must exist for delete fixup");

                // CASE 1: sibling is red
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .right
                        .clone()
                        .expect("sibling must still exist");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let wl_black = is_black(&w_left);
                let wr_black = is_black(&w_right);

                if wl_black && wr_black {
                    // CASE 2: both nephews black
                    w.borrow_mut().color = Color::Red;
                    let new_parent = parent.borrow().parent.clone();
                    x = Some(parent.clone());
                    x_parent = new_parent;
                } else {
                    let mut w = w;
                    let w_right_now = w.borrow().right.clone();
                    if is_black(&w_right_now) {
                        // CASE 3: right nephew black, left red -> rotate to CASE 4
                        if let Some(wl) = w.borrow().left.clone() {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent
                            .borrow()
                            .right
                            .clone()
                            .expect("sibling must still exist after rotation");
                    }
                    // CASE 4
                    let pcolor = parent.borrow().color.clone();
                    w.borrow_mut().color = pcolor;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            } else {
                // Mirror: sibling is parent.left
                let mut w = parent
                    .borrow()
                    .left
                    .clone()
                    .expect("sibling must exist for delete fixup");

                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .left
                        .clone()
                        .expect("sibling must still exist");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let wl_black = is_black(&w_left);
                let wr_black = is_black(&w_right);

                if wl_black && wr_black {
                    w.borrow_mut().color = Color::Red;
                    let new_parent = parent.borrow().parent.clone();
                    x = Some(parent.clone());
                    x_parent = new_parent;
                } else {
                    let mut w = w;
                    let w_left_now = w.borrow().left.clone();
                    if is_black(&w_left_now) {
                        if let Some(wr) = w.borrow().right.clone() {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent
                            .borrow()
                            .left
                            .clone()
                            .expect("sibling must still exist after rotation");
                    }
                    let pcolor = parent.borrow().color.clone();
                    w.borrow_mut().color = pcolor;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            }

            // Suppress unused-variable warning if needed
            let _ = &x_parent;
        }

        // x.color = BLACK (no-op for None)
        if let Some(xn) = &x {
            xn.borrow_mut().color = Color::Black;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let x_parent = x.as_ref().and_then(|n| n.borrow().parent.clone());
        self.delete_fixup_helper(x, x_parent);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        let mut y_original_color = p.borrow().color.clone();
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p_left.is_none() {
            // No left child: replace p with its right.
            x = p_right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            // No right child: replace p with its left.
            x = p_left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_left);
        } else {
            // Both children exist: find in-order successor y in p's right subtree.
            let mut cur = p_right.clone().expect("right child exists");
            loop {
                let l = cur.borrow().left.clone();
                match l {
                    Some(n) => cur = n,
                    None => break,
                }
            }
            let y = cur;
            y_original_color = y.borrow().color.clone();
            let y_right = y.borrow().right.clone();
            x = y_right.clone();

            let y_parent_is_p = y
                .borrow()
                .parent
                .as_ref()
                .map_or(false, |yp| Rc::ptr_eq(yp, &p));

            if y_parent_is_p {
                // x's effective parent (in tree, after the rest of the operations) is y.
                x_parent = Some(y.clone());
                // x.parent is already y by construction; nothing to update for non-nil x.
            } else {
                let y_old_parent = y.borrow().parent.clone();
                self.transplant(y.clone(), y_right.clone());
                // Now repurpose y to take p's place: y.right = p.right
                y.borrow_mut().right = p_right.clone();
                if let Some(pr) = &p_right {
                    pr.borrow_mut().parent = Some(y.clone());
                }
                x_parent = y_old_parent;
            }

            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p_left.clone();
            if let Some(pl) = &p_left {
                pl.borrow_mut().parent = Some(y.clone());
            }
            let pcolor = p.borrow().color.clone();
            y.borrow_mut().color = pcolor;
        }

        if y_original_color == Color::Black {
            self.delete_fixup_helper(x, x_parent);
        }

        // Detach p from any tree references.
        p.borrow_mut().parent = None;
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(
        &self,
        curr: Option<NodeRef>,
        arr: &mut Vec<Key>,
        n: usize,
        count: &mut usize,
    ) {
        let node = match curr {
            Some(c) => c,
            None => return,
        };
        let left = node.borrow().left.clone();
        self.subtree_to_array(left, arr, n, count);
        if *count < n {
            let key = node.borrow().key;
            arr.push(key);
            *count += 1;
        } else {
            return;
        }
        let right = node.borrow().right.clone();
        self.subtree_to_array(right, arr, n, count);
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr: Vec<Key> = Vec::new();
        if self.root.is_none() {
            return arr;
        }
        let mut count: usize = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}

