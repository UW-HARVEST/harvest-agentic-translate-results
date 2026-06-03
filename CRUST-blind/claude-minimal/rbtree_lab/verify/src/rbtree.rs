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
        // y = x->left
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate: x.left must not be NIL");

        // x->left = y->right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y->right != nil: y->right->parent = x
        if let Some(yr) = &y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y->parent = x->parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        // Update x's parent's child pointer (or root) to y.
        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let is_left = match &xp.borrow().left {
                    Some(n) => Rc::ptr_eq(n, &x),
                    None => false,
                };
                if is_left {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y->right = x; x->parent = y
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x->right
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate: x.right must not be NIL");

        // x->right = y->left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y->left != nil: y->left->parent = x
        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y->parent = x->parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        // Update x's parent's child pointer (or root) to y.
        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let is_left = match &xp.borrow().left {
                    Some(n) => Rc::ptr_eq(n, &x),
                    None => false,
                };
                if is_left {
                    xp.borrow_mut().left = Some(y.clone());
                } else {
                    xp.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y->left = x; x->parent = y
        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    /// Breaks Rc cycles by clearing parent/child pointers as it walks the tree.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            let (left, right) = {
                let mut nb = n.borrow_mut();
                let l = nb.left.take();
                let r = nb.right.take();
                nb.parent = None;
                (l, r)
            };
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
            // Stop if z is root or z's parent is black.
            let parent = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }

            // If parent is red it cannot be root, so it has a parent.
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break,
            };

            let parent_is_left = match &grandparent.borrow().left {
                Some(n) => Rc::ptr_eq(n, &parent),
                None => false,
            };

            if parent_is_left {
                let uncle = grandparent.borrow().right.clone();
                let uncle_is_red = uncle
                    .as_ref()
                    .map(|u| u.borrow().color == Color::Red)
                    .unwrap_or(false);

                if uncle_is_red {
                    // CASE 1: uncle is red — recolor and move up.
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = &uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 2: z is right child — rotate left around parent.
                    let z_is_right = match &parent.borrow().right {
                        Some(n) => Rc::ptr_eq(n, &z),
                        None => false,
                    };
                    if z_is_right {
                        z = parent.clone();
                        self.left_rotate(z.clone());
                    }
                    // CASE 3: recolor and rotate right around grandparent.
                    let p = z.borrow().parent.clone().unwrap();
                    p.borrow_mut().color = Color::Black;
                    let gp = p.borrow().parent.clone().unwrap();
                    gp.borrow_mut().color = Color::Red;
                    self.right_rotate(gp);
                }
            } else {
                let uncle = grandparent.borrow().left.clone();
                let uncle_is_red = uncle
                    .as_ref()
                    .map(|u| u.borrow().color == Color::Red)
                    .unwrap_or(false);

                if uncle_is_red {
                    // CASE 4: uncle is red — recolor and move up.
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = &uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 5: z is left child — rotate right around parent.
                    let z_is_left = match &parent.borrow().left {
                        Some(n) => Rc::ptr_eq(n, &z),
                        None => false,
                    };
                    if z_is_left {
                        z = parent.clone();
                        self.right_rotate(z.clone());
                    }
                    // CASE 6: recolor and rotate left around grandparent.
                    let p = z.borrow().parent.clone().unwrap();
                    p.borrow_mut().color = Color::Black;
                    let gp = p.borrow().parent.clone().unwrap();
                    gp.borrow_mut().color = Color::Red;
                    self.left_rotate(gp);
                }
            }
        }

        // Root must be black.
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

        // Standard BST insert: find parent y of new node.
        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();

        while let Some(n) = x {
            y = Some(n.clone());
            let next = if key < n.borrow().key {
                n.borrow().left.clone()
            } else {
                n.borrow().right.clone()
            };
            x = next;
        }

        z.borrow_mut().parent = y.clone();

        match &y {
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

        self.rbtree_insert_fixup(z.clone());

        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(n) = current {
            let nk = n.borrow().key;
            if nk == key {
                return Some(n);
            }
            current = if nk < key {
                n.borrow().right.clone()
            } else {
                n.borrow().left.clone()
            };
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let left = curr.borrow().left.clone();
            match left {
                Some(l) => curr = l,
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
                Some(r) => curr = r,
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
                let is_left = match &up.borrow().left {
                    Some(n) => Rc::ptr_eq(n, &u),
                    None => false,
                };
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

    /// Internal delete-fixup that also tracks `x`'s parent and which side
    /// `x` lives on. We need this because in Rust we use `None` to represent
    /// the NIL sentinel and cannot store a "parent" on a None value the way
    /// the C code uses `t->nil->parent`.
    fn delete_fixup_helper(
        &mut self,
        mut x: Option<NodeRef>,
        mut x_parent: Option<NodeRef>,
        mut x_is_left: bool,
    ) {
        loop {
            // Loop while x is not root and x is black (NIL counts as black).
            if x_parent.is_none() {
                break;
            }
            let x_color = x
                .as_ref()
                .map(|n| n.borrow().color.clone())
                .unwrap_or(Color::Black);
            if x_color != Color::Black {
                break;
            }

            let parent = x_parent.clone().unwrap();

            if x_is_left {
                // Sibling is parent.right
                let mut w = parent
                    .borrow()
                    .right
                    .clone()
                    .expect("delete_fixup: sibling cannot be NIL (left case)");

                // CASE 1: sibling is red.
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .right
                        .clone()
                        .expect("delete_fixup: sibling after rotate must exist");
                }

                let wl_color = w
                    .borrow()
                    .left
                    .as_ref()
                    .map(|n| n.borrow().color.clone())
                    .unwrap_or(Color::Black);
                let wr_color = w
                    .borrow()
                    .right
                    .as_ref()
                    .map(|n| n.borrow().color.clone())
                    .unwrap_or(Color::Black);

                if wl_color == Color::Black && wr_color == Color::Black {
                    // CASE 2: both of sibling's children are black.
                    w.borrow_mut().color = Color::Red;
                    let new_x = parent.clone();
                    let new_x_parent = parent.borrow().parent.clone();
                    let new_x_is_left = match &new_x_parent {
                        Some(p) => match &p.borrow().left {
                            Some(l) => Rc::ptr_eq(l, &new_x),
                            None => false,
                        },
                        None => false,
                    };
                    x = Some(new_x);
                    x_parent = new_x_parent;
                    x_is_left = new_x_is_left;
                } else {
                    // CASE 3: sibling's right child is black.
                    if wr_color == Color::Black {
                        if let Some(wl) = w.borrow().left.clone() {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent.borrow().right.clone().unwrap();
                    }
                    // CASE 4: sibling's right child is red.
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                    x_is_left = false;
                }
            } else {
                // Sibling is parent.left
                let mut w = parent
                    .borrow()
                    .left
                    .clone()
                    .expect("delete_fixup: sibling cannot be NIL (right case)");

                // CASE 5 (mirror of 1): sibling is red.
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .left
                        .clone()
                        .expect("delete_fixup: sibling after rotate must exist");
                }

                let wl_color = w
                    .borrow()
                    .left
                    .as_ref()
                    .map(|n| n.borrow().color.clone())
                    .unwrap_or(Color::Black);
                let wr_color = w
                    .borrow()
                    .right
                    .as_ref()
                    .map(|n| n.borrow().color.clone())
                    .unwrap_or(Color::Black);

                if wl_color == Color::Black && wr_color == Color::Black {
                    // CASE 6 (mirror of 2): both children black.
                    w.borrow_mut().color = Color::Red;
                    let new_x = parent.clone();
                    let new_x_parent = parent.borrow().parent.clone();
                    let new_x_is_left = match &new_x_parent {
                        Some(p) => match &p.borrow().left {
                            Some(l) => Rc::ptr_eq(l, &new_x),
                            None => false,
                        },
                        None => false,
                    };
                    x = Some(new_x);
                    x_parent = new_x_parent;
                    x_is_left = new_x_is_left;
                } else {
                    // CASE 7 (mirror of 3): sibling's left child is black.
                    if wl_color == Color::Black {
                        if let Some(wr) = w.borrow().right.clone() {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent.borrow().left.clone().unwrap();
                    }
                    // CASE 8 (mirror of 4): sibling's left child is red.
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                    x_is_left = false;
                }
            }
        }

        if let Some(n) = &x {
            n.borrow_mut().color = Color::Black;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        // We can derive parent and side from `x` only when `x` itself is non-NIL.
        // When `x` is NIL we have no way to recover its parent here, so this
        // public entrypoint is a no-op in that case. `erase` calls the
        // internal helper directly with the parent it tracked.
        if let Some(n) = x.as_ref() {
            let parent = n.borrow().parent.clone();
            let x_is_left = match &parent {
                Some(p) => match &p.borrow().left {
                    Some(l) => Rc::ptr_eq(l, n),
                    None => false,
                },
                None => false,
            };
            self.delete_fixup_helper(x.clone(), parent, x_is_left);
        }
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();
        let p_parent = p.borrow().parent.clone();
        let p_color = p.borrow().color.clone();

        // Was p the left child of its parent?
        let p_is_left = match &p_parent {
            Some(pp) => match &pp.borrow().left {
                Some(l) => Rc::ptr_eq(l, &p),
                None => false,
            },
            None => false,
        };

        let mut y_original_color = p_color.clone();
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;
        let x_is_left: bool;

        if p_left.is_none() {
            x = p_right.clone();
            x_parent = p_parent.clone();
            x_is_left = p_is_left;
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            x = p_left.clone();
            x_parent = p_parent.clone();
            x_is_left = p_is_left;
            self.transplant(p.clone(), p_left);
        } else {
            // Find the minimum of the right subtree.
            let mut y = p_right.clone().unwrap();
            loop {
                let yl = y.borrow().left.clone();
                match yl {
                    Some(l) => y = l,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            let y_right = y.borrow().right.clone();
            x = y_right.clone();
            let y_parent = y.borrow().parent.clone();

            let y_parent_is_p = match &y_parent {
                Some(yp) => Rc::ptr_eq(yp, &p),
                None => false,
            };

            if y_parent_is_p {
                // y is the direct right child of p; x lives at y.right.
                x_parent = Some(y.clone());
                x_is_left = false;
            } else {
                // y is somewhere deeper in the right subtree (always a left
                // child of its parent because we walked left from p.right).
                x_parent = y_parent.clone();
                x_is_left = true;
                self.transplant(y.clone(), y_right);
                y.borrow_mut().right = p_right.clone();
                if let Some(pr) = &p_right {
                    pr.borrow_mut().parent = Some(y.clone());
                }
            }

            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p_left.clone();
            if let Some(pl) = &p_left {
                pl.borrow_mut().parent = Some(y.clone());
            }
            y.borrow_mut().color = p_color;
        }

        if y_original_color == Color::Black {
            self.delete_fixup_helper(x, x_parent, x_is_left);
        }

        // Detach p so it does not retain references to other tree nodes.
        {
            let mut pb = p.borrow_mut();
            pb.parent = None;
            pb.left = None;
            pb.right = None;
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
            let right = c.borrow().right.clone();
            let key = c.borrow().key;

            self.subtree_to_array(left, arr, n, count);
            if *count < n {
                arr.push(key);
                *count += 1;
            } else {
                return;
            }
            self.subtree_to_array(right, arr, n, count);
        }
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr = Vec::with_capacity(n);
        if self.root.is_some() {
            let mut count = 0usize;
            self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        }
        arr
    }
}
