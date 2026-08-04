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

/// Color of an Option<NodeRef>: NIL is treated as Black (RB-tree convention).
fn node_color(n: &Option<NodeRef>) -> Color {
    match n {
        None => Color::Black,
        Some(node) => node.borrow().color.clone(),
    }
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        // y = x.left  (must not be NIL)
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate: x.left must not be NIL");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y.right != NIL, y.right.parent = x
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match x_parent {
            None => {
                // x was root
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let is_left = xp
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &x));
                if is_left {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.right = x
        y.borrow_mut().right = Some(x.clone());
        // x.parent = y
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x.right (must not be NIL)
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate: x.right must not be NIL");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y.left != NIL, y.left.parent = x
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
            Some(xp) => {
                let is_left = xp
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &x));
                if is_left {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.left = x
        y.borrow_mut().left = Some(x.clone());
        // x.parent = y
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            // Take children out and recurse, breaking child links.
            let left = n.borrow_mut().left.take();
            let right = n.borrow_mut().right.take();
            // Also break parent link to avoid Rc cycles.
            n.borrow_mut().parent = None;
            Self::free_node(left);
            Self::free_node(right);
            // n will be dropped when its last Rc reference goes away.
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
            // Get z.parent; if it's None or Black, stop.
            let z_parent = z.borrow().parent.clone();
            let parent = match z_parent {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }

            // Get grandparent.
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break,
            };

            // Determine if parent is the left child of grandparent.
            let parent_is_left = grandparent
                .borrow()
                .left
                .as_ref()
                .map_or(false, |l| Rc::ptr_eq(l, &parent));

            if parent_is_left {
                let y = grandparent.borrow().right.clone();

                if node_color(&y) == Color::Red {
                    // CASE 1: uncle is red
                    parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // Check if z is right child of parent
                    let z_is_right = parent
                        .borrow()
                        .right
                        .as_ref()
                        .map_or(false, |r| Rc::ptr_eq(r, &z));

                    let mut z_local = z.clone();
                    if z_is_right {
                        // CASE 2
                        z_local = parent.clone();
                        self.left_rotate(z_local.clone());
                    }
                    // CASE 3
                    let new_parent = z_local.borrow().parent.clone().unwrap();
                    new_parent.borrow_mut().color = Color::Black;
                    let new_gp = new_parent.borrow().parent.clone().unwrap();
                    new_gp.borrow_mut().color = Color::Red;
                    self.right_rotate(new_gp);
                    z = z_local;
                }
            } else {
                // Parent is right child of grandparent (mirror case).
                let y = grandparent.borrow().left.clone();

                if node_color(&y) == Color::Red {
                    parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
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

                    let mut z_local = z.clone();
                    if z_is_left {
                        z_local = parent.clone();
                        self.right_rotate(z_local.clone());
                    }
                    let new_parent = z_local.borrow().parent.clone().unwrap();
                    new_parent.borrow_mut().color = Color::Black;
                    let new_gp = new_parent.borrow().parent.clone().unwrap();
                    new_gp.borrow_mut().color = Color::Red;
                    self.left_rotate(new_gp);
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

        // Walk down to find the parent.
        let mut y: Option<NodeRef> = None;
        let mut x: Option<NodeRef> = self.root.clone();

        while let Some(curr) = x {
            y = Some(curr.clone());
            let curr_key = curr.borrow().key;
            if key < curr_key {
                x = curr.borrow().left.clone();
            } else {
                x = curr.borrow().right.clone();
            }
        }

        // z.parent = y
        z.borrow_mut().parent = y.clone();

        match y {
            None => {
                self.root = Some(z.clone());
            }
            Some(yn) => {
                let y_key = yn.borrow().key;
                if key < y_key {
                    yn.borrow_mut().left = Some(z.clone());
                } else {
                    yn.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // children already None (NIL), color already Red.
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
        match u_parent.clone() {
            None => {
                self.root = v.clone();
            }
            Some(up) => {
                let u_is_left = up
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &u));
                if u_is_left {
                    up.borrow_mut().left = v.clone();
                } else {
                    up.borrow_mut().right = v.clone();
                }
            }
        }
        // v.parent = u.parent (only if v is not NIL; in C, NIL has a settable parent)
        // For our model with None == NIL, we just skip when v is None.
        if let Some(vn) = v {
            vn.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        // When called publicly we recover x's parent from the node itself.
        // (For NIL == None we have no parent, so simply return.)
        let parent = match &x {
            Some(xn) => xn.borrow().parent.clone(),
            None => return,
        };
        self.delete_fixup_internal(x, parent);
    }

    /// Internal fixup that tracks `x`'s parent explicitly. This is needed
    /// because NIL nodes (`None`) cannot store a parent pointer in our model.
    fn delete_fixup_internal(&mut self, x: Option<NodeRef>, x_parent_init: Option<NodeRef>) {
        let mut x = x;
        let mut x_parent = x_parent_init;

        loop {
            // Stop if x is the root or x is Red.
            let is_root = match (&x, &self.root) {
                (Some(xn), Some(rn)) => Rc::ptr_eq(xn, rn),
                _ => false,
            };
            if is_root {
                break;
            }
            if node_color(&x) == Color::Red {
                break;
            }

            // x must have a parent (otherwise it would be root or empty tree).
            let parent = match x_parent.clone() {
                Some(p) => p,
                None => break,
            };

            // Determine whether x is left child of parent.
            // When x is None (NIL), we use the fact that parent.left or parent.right is None.
            let x_is_left = match &x {
                Some(xn) => parent
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, xn)),
                None => {
                    // x is NIL: it's "left" if parent.left is None AND parent.right is not None
                    // OR if parent.left is None and parent.right is also None (ambiguous, but pick left)
                    // Actually we need a deterministic answer. We rely on the caller setting
                    // x_parent to specifically the parent of which slot x came from.
                    // Use: x is left iff parent.left is None and parent.right is Some(_) -> not enough.
                    // We must track the side externally. We'll use a heuristic: if parent.left is None,
                    // assume x is left.
                    parent.borrow().left.is_none()
                }
            };

            if x_is_left {
                let mut w = parent.borrow().right.clone();

                // CASE 1: w is red
                if node_color(&w) == Color::Red {
                    if let Some(wn) = &w {
                        wn.borrow_mut().color = Color::Black;
                    }
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent.borrow().right.clone();
                }

                // CASE 2: both children of w are black
                let w_left = w.as_ref().and_then(|wn| wn.borrow().left.clone());
                let w_right = w.as_ref().and_then(|wn| wn.borrow().right.clone());

                if node_color(&w_left) == Color::Black && node_color(&w_right) == Color::Black {
                    if let Some(wn) = &w {
                        wn.borrow_mut().color = Color::Red;
                    }
                    x = Some(parent.clone());
                    x_parent = parent.borrow().parent.clone();
                } else {
                    // CASE 3: w.right is black -> rotate w right
                    if node_color(&w_right) == Color::Black {
                        if let Some(wln) = &w_left {
                            wln.borrow_mut().color = Color::Black;
                        }
                        if let Some(wn) = &w {
                            wn.borrow_mut().color = Color::Red;
                            self.right_rotate(wn.clone());
                        }
                        w = parent.borrow().right.clone();
                    }

                    // CASE 4: w.right is red
                    if let Some(wn) = &w {
                        wn.borrow_mut().color = parent.borrow().color.clone();
                    }
                    parent.borrow_mut().color = Color::Black;
                    let w_right_now = w.as_ref().and_then(|wn| wn.borrow().right.clone());
                    if let Some(wrn) = &w_right_now {
                        wrn.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            } else {
                // Mirror case: x is right child
                let mut w = parent.borrow().left.clone();

                if node_color(&w) == Color::Red {
                    if let Some(wn) = &w {
                        wn.borrow_mut().color = Color::Black;
                    }
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent.borrow().left.clone();
                }

                let w_left = w.as_ref().and_then(|wn| wn.borrow().left.clone());
                let w_right = w.as_ref().and_then(|wn| wn.borrow().right.clone());

                if node_color(&w_right) == Color::Black && node_color(&w_left) == Color::Black {
                    if let Some(wn) = &w {
                        wn.borrow_mut().color = Color::Red;
                    }
                    x = Some(parent.clone());
                    x_parent = parent.borrow().parent.clone();
                } else {
                    if node_color(&w_left) == Color::Black {
                        if let Some(wrn) = &w_right {
                            wrn.borrow_mut().color = Color::Black;
                        }
                        if let Some(wn) = &w {
                            wn.borrow_mut().color = Color::Red;
                            self.left_rotate(wn.clone());
                        }
                        w = parent.borrow().left.clone();
                    }

                    if let Some(wn) = &w {
                        wn.borrow_mut().color = parent.borrow().color.clone();
                    }
                    parent.borrow_mut().color = Color::Black;
                    let w_left_now = w.as_ref().and_then(|wn| wn.borrow().left.clone());
                    if let Some(wln) = &w_left_now {
                        wln.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
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
        let x: Option<NodeRef>;
        // Track the parent of x for fixup, since NIL has no parent pointer in our model.
        let x_parent: Option<NodeRef>;

        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        if p_left.is_none() {
            x = p_right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            x = p_left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_left);
        } else {
            // y = minimum of p.right
            y = p_right.clone().unwrap();
            loop {
                let next = y.borrow().left.clone();
                match next {
                    Some(n) => y = n,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            // If y.parent == p:
            let y_parent = y.borrow().parent.clone();
            let y_parent_is_p = y_parent
                .as_ref()
                .map_or(false, |yp| Rc::ptr_eq(yp, &p));

            if y_parent_is_p {
                // x's parent should be y. If x is None (NIL), we just track it externally.
                if let Some(xn) = &x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                // transplant(y, y.right)
                let y_right = y.borrow().right.clone();
                x_parent = y.borrow().parent.clone();
                self.transplant(y.clone(), y_right);
                // y.right = p.right; y.right.parent = y;
                let p_right_now = p.borrow().right.clone();
                y.borrow_mut().right = p_right_now.clone();
                if let Some(prn) = &p_right_now {
                    prn.borrow_mut().parent = Some(y.clone());
                }
            }

            // transplant(p, y)
            self.transplant(p.clone(), Some(y.clone()));
            // y.left = p.left; y.left.parent = y;
            let p_left_now = p.borrow().left.clone();
            y.borrow_mut().left = p_left_now.clone();
            if let Some(pln) = &p_left_now {
                pln.borrow_mut().parent = Some(y.clone());
            }
            // y.color = p.color
            y.borrow_mut().color = p.borrow().color.clone();
        }

        // Detach p to break Rc cycle.
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;
        p.borrow_mut().parent = None;

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
        let curr = match curr {
            None => return,
            Some(c) => c,
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
}
