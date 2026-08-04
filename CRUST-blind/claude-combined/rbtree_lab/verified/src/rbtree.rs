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

// Helpers ----------------------------------------------------------

fn new_nil() -> NodeRef {
    Rc::new(RefCell::new(Node {
        key: 0,
        color: Color::Black,
        left: None,
        right: None,
        parent: None,
    }))
}

fn ptr_eq_opt_ref(a: &Option<NodeRef>, b: &NodeRef) -> bool {
    a.as_ref().map_or(false, |x| Rc::ptr_eq(x, b))
}

fn is_black_or_nil(n: &Option<NodeRef>) -> bool {
    n.as_ref().map_or(true, |x| x.borrow().color == Color::Black)
}

impl Default for RBTree {
    fn default() -> Self {
        Self::new()
    }
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate requires non-nil left child");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y.right != nil: y.right.parent = x
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match x_parent {
            None => self.root = Some(y.clone()),
            Some(xp) => {
                let is_left = ptr_eq_opt_ref(&xp.borrow().left, &x);
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
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate requires non-nil right child");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match x_parent {
            None => self.root = Some(y.clone()),
            Some(xp) => {
                let is_left = ptr_eq_opt_ref(&xp.borrow().left, &x);
                if is_left {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            // Break parent cycle and recurse into children
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
            let parent = match z.borrow().parent.clone() {
                Some(p) if p.borrow().color == Color::Red => p,
                _ => break,
            };
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break, // parent is root, won't be red after final fixup
            };
            let parent_is_left = ptr_eq_opt_ref(&grandparent.borrow().left, &parent);
            if parent_is_left {
                let uncle = grandparent.borrow().right.clone();
                let uncle_red = uncle
                    .as_ref()
                    .map_or(false, |u| u.borrow().color == Color::Red);
                if uncle_red {
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    let z_is_right = ptr_eq_opt_ref(&parent.borrow().right, &z);
                    let new_z = if z_is_right {
                        let new_z = parent.clone();
                        self.left_rotate(new_z.clone());
                        new_z
                    } else {
                        z
                    };
                    let p2 = new_z.borrow().parent.clone().expect("parent");
                    p2.borrow_mut().color = Color::Black;
                    let gp2 = p2.borrow().parent.clone().expect("grandparent");
                    gp2.borrow_mut().color = Color::Red;
                    self.right_rotate(gp2);
                    z = new_z;
                }
            } else {
                let uncle = grandparent.borrow().left.clone();
                let uncle_red = uncle
                    .as_ref()
                    .map_or(false, |u| u.borrow().color == Color::Red);
                if uncle_red {
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    let z_is_left = ptr_eq_opt_ref(&parent.borrow().left, &z);
                    let new_z = if z_is_left {
                        let new_z = parent.clone();
                        self.right_rotate(new_z.clone());
                        new_z
                    } else {
                        z
                    };
                    let p2 = new_z.borrow().parent.clone().expect("parent");
                    p2.borrow_mut().color = Color::Black;
                    let gp2 = p2.borrow().parent.clone().expect("grandparent");
                    gp2.borrow_mut().color = Color::Red;
                    self.left_rotate(gp2);
                    z = new_z;
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

        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();
        while let Some(x_node) = x {
            y = Some(x_node.clone());
            let next = if key < x_node.borrow().key {
                x_node.borrow().left.clone()
            } else {
                x_node.borrow().right.clone()
            };
            x = next;
        }

        z.borrow_mut().parent = y.clone();

        match &y {
            None => self.root = Some(z.clone()),
            Some(y_node) => {
                let y_key = y_node.borrow().key;
                if key < y_key {
                    y_node.borrow_mut().left = Some(z.clone());
                } else {
                    y_node.borrow_mut().right = Some(z.clone());
                }
            }
        }

        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(c) = current {
            let c_key = c.borrow().key;
            if c_key == key {
                return Some(c);
            }
            let next = if c_key < key {
                c.borrow().right.clone()
            } else {
                c.borrow().left.clone()
            };
            current = next;
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut current = self.root.clone()?;
        loop {
            let next = current.borrow().left.clone();
            match next {
                Some(n) => current = n,
                None => return Some(current),
            }
        }
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut current = self.root.clone()?;
        loop {
            let next = current.borrow().right.clone();
            match next {
                Some(n) => current = n,
                None => return Some(current),
            }
        }
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let u_parent = u.borrow().parent.clone();
        match &u_parent {
            None => self.root = v.clone(),
            Some(up) => {
                let is_left = ptr_eq_opt_ref(&up.borrow().left, &u);
                if is_left {
                    up.borrow_mut().left = v.clone();
                } else {
                    up.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(v_node) = &v {
            v_node.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    /// If x is None, this is a no-op (there's no parent context to work with).
    /// Internal callers should pass a real node (using a temporary NIL placeholder when needed).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let x = match x {
            Some(n) => n,
            None => return,
        };
        self.delete_fixup_inner(x);
    }

    fn delete_fixup_inner(&mut self, x: NodeRef) {
        let mut x = x;
        loop {
            let is_root = self
                .root
                .as_ref()
                .map_or(false, |r| Rc::ptr_eq(r, &x));
            if is_root || x.borrow().color == Color::Red {
                break;
            }
            let parent = x.borrow().parent.clone().expect("non-root has parent");
            let x_is_left = ptr_eq_opt_ref(&parent.borrow().left, &x);
            if x_is_left {
                let mut w = parent
                    .borrow()
                    .right
                    .clone()
                    .expect("sibling must exist (rb invariant)");
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .right
                        .clone()
                        .expect("sibling must exist after rotate");
                }
                let w_left_black = is_black_or_nil(&w.borrow().left);
                let w_right_black = is_black_or_nil(&w.borrow().right);
                if w_left_black && w_right_black {
                    w.borrow_mut().color = Color::Red;
                    x = parent;
                } else {
                    if w_right_black {
                        if let Some(wl) = w.borrow().left.clone() {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent
                            .borrow()
                            .right
                            .clone()
                            .expect("sibling after rotate");
                    }
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone().expect("root exists");
                }
            } else {
                let mut w = parent
                    .borrow()
                    .left
                    .clone()
                    .expect("sibling must exist (rb invariant)");
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .left
                        .clone()
                        .expect("sibling after rotate");
                }
                let w_left_black = is_black_or_nil(&w.borrow().left);
                let w_right_black = is_black_or_nil(&w.borrow().right);
                if w_right_black && w_left_black {
                    w.borrow_mut().color = Color::Red;
                    x = parent;
                } else {
                    if w_left_black {
                        if let Some(wr) = w.borrow().right.clone() {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent
                            .borrow()
                            .left
                            .clone()
                            .expect("sibling after rotate");
                    }
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
                    x = self.root.clone().expect("root exists");
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

        // Track temp NIL: (parent, was_left_child) so we can detach after fixup.
        // Special case: parent == None means temp nil became the root.
        let mut temp_nil: Option<(Option<NodeRef>, bool)> = None;
        let x_for_fixup: Option<NodeRef>;

        if p_left.is_none() {
            // x = p.right (might be None)
            let x = p_right.clone();
            // Capture p's parent and side before transplant
            let p_parent = p.borrow().parent.clone();
            let p_was_left = match &p_parent {
                Some(pp) => ptr_eq_opt_ref(&pp.borrow().left, &p),
                None => false,
            };
            self.transplant(p.clone(), p_right.clone());

            x_for_fixup = match x {
                Some(n) => Some(n),
                None => {
                    // Need temp nil at p's old position
                    let nil = new_nil();
                    nil.borrow_mut().parent = p_parent.clone();
                    match &p_parent {
                        None => {
                            // p was root; tree is now empty; place nil as root
                            self.root = Some(nil.clone());
                        }
                        Some(pp) => {
                            if p_was_left {
                                pp.borrow_mut().left = Some(nil.clone());
                            } else {
                                pp.borrow_mut().right = Some(nil.clone());
                            }
                        }
                    }
                    temp_nil = Some((p_parent, p_was_left));
                    Some(nil)
                }
            };
        } else if p_right.is_none() {
            let x = p_left.clone();
            let p_parent = p.borrow().parent.clone();
            let p_was_left = match &p_parent {
                Some(pp) => ptr_eq_opt_ref(&pp.borrow().left, &p),
                None => false,
            };
            self.transplant(p.clone(), p_left.clone());

            x_for_fixup = match x {
                Some(n) => Some(n),
                None => {
                    let nil = new_nil();
                    nil.borrow_mut().parent = p_parent.clone();
                    match &p_parent {
                        None => {
                            self.root = Some(nil.clone());
                        }
                        Some(pp) => {
                            if p_was_left {
                                pp.borrow_mut().left = Some(nil.clone());
                            } else {
                                pp.borrow_mut().right = Some(nil.clone());
                            }
                        }
                    }
                    temp_nil = Some((p_parent, p_was_left));
                    Some(nil)
                }
            };
        } else {
            // y = min of p's right subtree
            y = p_right.clone().unwrap();
            loop {
                let yl = y.borrow().left.clone();
                match yl {
                    Some(n) => y = n,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            let x = y.borrow().right.clone();

            let y_parent = y.borrow().parent.clone().expect("y has parent");
            if Rc::ptr_eq(&y_parent, &p) {
                // x.parent = y (already true if x is Some, not relevant if None except for fixup)
                if let Some(x_node) = &x {
                    x_node.borrow_mut().parent = Some(y.clone());
                }

                // For fixup, x is at y.right
                let x_for_fixup_local: Option<NodeRef> = match &x {
                    Some(n) => Some(n.clone()),
                    None => {
                        let nil = new_nil();
                        nil.borrow_mut().parent = Some(y.clone());
                        y.borrow_mut().right = Some(nil.clone());
                        temp_nil = Some((Some(y.clone()), false));
                        Some(nil)
                    }
                };

                // transplant(p, y)
                self.transplant(p.clone(), Some(y.clone()));
                y.borrow_mut().left = p_left.clone();
                if let Some(pl) = &p_left {
                    pl.borrow_mut().parent = Some(y.clone());
                }
                y.borrow_mut().color = p.borrow().color.clone();

                x_for_fixup = x_for_fixup_local;
            } else {
                // y is a left child of y_parent (since we descended at least one left step)
                self.transplant(y.clone(), y.borrow().right.clone());
                let p_right_clone = p_right.clone().unwrap();
                y.borrow_mut().right = Some(p_right_clone.clone());
                p_right_clone.borrow_mut().parent = Some(y.clone());

                // x's "position" is y_parent.left (since y was a left child)
                let x_for_fixup_local: Option<NodeRef> = match &x {
                    Some(n) => Some(n.clone()),
                    None => {
                        let nil = new_nil();
                        nil.borrow_mut().parent = Some(y_parent.clone());
                        y_parent.borrow_mut().left = Some(nil.clone());
                        temp_nil = Some((Some(y_parent.clone()), true));
                        Some(nil)
                    }
                };

                self.transplant(p.clone(), Some(y.clone()));
                y.borrow_mut().left = p_left.clone();
                if let Some(pl) = &p_left {
                    pl.borrow_mut().parent = Some(y.clone());
                }
                y.borrow_mut().color = p.borrow().color.clone();

                x_for_fixup = x_for_fixup_local;
            }
        }

        if y_original_color == Color::Black {
            if let Some(x_real) = x_for_fixup {
                self.delete_fixup_inner(x_real);
            }
        }

        // Detach temp NIL if used
        if let Some((parent_opt, was_left)) = temp_nil {
            match parent_opt {
                None => {
                    // Tree was just p; nil was placed as root
                    self.root = None;
                }
                Some(parent) => {
                    if was_left {
                        parent.borrow_mut().left = None;
                    } else {
                        parent.borrow_mut().right = None;
                    }
                }
            }
        }

        // Detach p from any lingering links to break potential cycles
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;
        p.borrow_mut().parent = None;
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
        let mut count = 0usize;
        let root = self.root.clone();
        self.subtree_to_array(root, &mut arr, n, &mut count);
        arr
    }
}
