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

/// Returns the color of a node; `None` is treated as BLACK (sentinel NIL).
fn color_of(n: &Option<NodeRef>) -> Color {
    n.as_ref().map_or(Color::Black, |x| x.borrow().color.clone())
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
                let xp_left = xp.borrow().left.clone();
                let is_left = match &xp_left {
                    Some(l) => Rc::ptr_eq(l, &x),
                    None => false,
                };
                if is_left {
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

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match &x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_left = xp.borrow().left.clone();
                let is_left = match &xp_left {
                    Some(l) => Rc::ptr_eq(l, &x),
                    None => false,
                };
                if is_left {
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
            // Take children out so we can recurse without holding any borrow.
            let left;
            let right;
            {
                let mut nb = n.borrow_mut();
                left = nb.left.take();
                right = nb.right.take();
                // Break parent cycle so the Rc<RefCell<Node>> can be dropped.
                nb.parent = None;
            }
            Self::free_node(left);
            Self::free_node(right);
            // The Rc to `n` will drop here, freeing the node if this was the
            // last strong reference (parent links have already been cleared).
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
            // while (z.parent.color == RED)
            let zp_opt = z.borrow().parent.clone();
            let zp = match zp_opt {
                Some(p) => p,
                None => break,
            };
            if zp.borrow().color != Color::Red {
                break;
            }

            let zpp_opt = zp.borrow().parent.clone();
            let zpp = match zpp_opt {
                Some(g) => g,
                None => break,
            };

            let zpp_left = zpp.borrow().left.clone();
            let parent_is_left = match &zpp_left {
                Some(l) => Rc::ptr_eq(l, &zp),
                None => false,
            };

            if parent_is_left {
                let y = zpp.borrow().right.clone();

                if color_of(&y) == Color::Red {
                    // Case 1: uncle red
                    zp.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp;
                } else {
                    // Case 2: z is right child -> rotate to make it left child
                    let zp_right = zp.borrow().right.clone();
                    let z_is_right = match &zp_right {
                        Some(r) => Rc::ptr_eq(r, &z),
                        None => false,
                    };

                    let z_local = if z_is_right {
                        let zp_clone = zp.clone();
                        self.left_rotate(zp_clone.clone());
                        zp_clone
                    } else {
                        z.clone()
                    };

                    // Case 3
                    let z_par = z_local
                        .borrow()
                        .parent
                        .clone()
                        .expect("z_local must have a parent in case 3");
                    z_par.borrow_mut().color = Color::Black;
                    let z_gpar = z_par
                        .borrow()
                        .parent
                        .clone()
                        .expect("z_par must have a parent in case 3");
                    z_gpar.borrow_mut().color = Color::Red;
                    self.right_rotate(z_gpar);
                    z = z_local;
                }
            } else {
                // Mirror: parent is grandparent's right child
                let y = zpp.borrow().left.clone();

                if color_of(&y) == Color::Red {
                    // Case 4 (mirrored case 1)
                    zp.borrow_mut().color = Color::Black;
                    if let Some(yn) = &y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp;
                } else {
                    // Case 5 (mirrored case 2)
                    let zp_left = zp.borrow().left.clone();
                    let z_is_left = match &zp_left {
                        Some(l) => Rc::ptr_eq(l, &z),
                        None => false,
                    };

                    let z_local = if z_is_left {
                        let zp_clone = zp.clone();
                        self.right_rotate(zp_clone.clone());
                        zp_clone
                    } else {
                        z.clone()
                    };

                    // Case 6
                    let z_par = z_local
                        .borrow()
                        .parent
                        .clone()
                        .expect("z_local must have a parent in case 6");
                    z_par.borrow_mut().color = Color::Black;
                    let z_gpar = z_par
                        .borrow()
                        .parent
                        .clone()
                        .expect("z_par must have a parent in case 6");
                    z_gpar.borrow_mut().color = Color::Red;
                    self.left_rotate(z_gpar);
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

        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();

        while let Some(curr) = x {
            y = Some(curr.clone());
            let curr_key = curr.borrow().key;
            let next = if key < curr_key {
                curr.borrow().left.clone()
            } else {
                curr.borrow().right.clone()
            };
            x = next;
        }

        z.borrow_mut().parent = y.clone();

        match &y {
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

        // z.left, z.right and z.color are already set from creation
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
            current = if curr_key < key {
                curr.borrow().right.clone()
            } else {
                curr.borrow().left.clone()
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
                let u_is_left = match &up_left {
                    Some(l) => Rc::ptr_eq(l, &u),
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
        let mut x = match x {
            Some(n) => n,
            None => return,
        };

        loop {
            // while (x != root && x.color == BLACK)
            let is_root = match &self.root {
                Some(r) => Rc::ptr_eq(r, &x),
                None => false,
            };
            if is_root {
                break;
            }
            if x.borrow().color != Color::Black {
                break;
            }

            let x_parent_opt = x.borrow().parent.clone();
            let x_parent = match x_parent_opt {
                Some(p) => p,
                None => break,
            };

            let xp_left = x_parent.borrow().left.clone();
            let x_is_left = match &xp_left {
                Some(l) => Rc::ptr_eq(l, &x),
                None => false,
            };

            if x_is_left {
                // LEFT case
                let mut w = x_parent
                    .borrow()
                    .right
                    .clone()
                    .expect("sibling must exist (cannot be nil) for black non-root node");

                // Case 1: w is RED
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    x_parent.borrow_mut().color = Color::Red;
                    self.left_rotate(x_parent.clone());
                    w = x_parent
                        .borrow()
                        .right
                        .clone()
                        .expect("new sibling must exist after rotation");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let w_left_black = color_of(&w_left) == Color::Black;
                let w_right_black = color_of(&w_right) == Color::Black;

                if w_left_black && w_right_black {
                    // Case 2: both of w's children are black
                    w.borrow_mut().color = Color::Red;
                    x = x_parent;
                } else {
                    // Case 3: w.right is BLACK (and w.left is RED)
                    if color_of(&w_right) == Color::Black {
                        if let Some(wl) = &w_left {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = x_parent
                            .borrow()
                            .right
                            .clone()
                            .expect("new sibling must exist after rotation");
                    }

                    // Case 4
                    let xp_color = x_parent.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    x_parent.borrow_mut().color = Color::Black;
                    let w_right_now = w.borrow().right.clone();
                    if let Some(wr) = &w_right_now {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(x_parent.clone());
                    x = self.root.clone().expect("root must exist after rotation");
                }
            } else {
                // RIGHT case (mirror)
                let mut w = x_parent
                    .borrow()
                    .left
                    .clone()
                    .expect("sibling must exist (cannot be nil) for black non-root node");

                // Case 5 (mirrored 1)
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    x_parent.borrow_mut().color = Color::Red;
                    self.right_rotate(x_parent.clone());
                    w = x_parent
                        .borrow()
                        .left
                        .clone()
                        .expect("new sibling must exist after rotation");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let w_left_black = color_of(&w_left) == Color::Black;
                let w_right_black = color_of(&w_right) == Color::Black;

                if w_right_black && w_left_black {
                    // Case 6 (mirrored 2)
                    w.borrow_mut().color = Color::Red;
                    x = x_parent;
                } else {
                    // Case 7 (mirrored 3): w.left is BLACK (and w.right is RED)
                    if color_of(&w_left) == Color::Black {
                        if let Some(wr) = &w_right {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = x_parent
                            .borrow()
                            .left
                            .clone()
                            .expect("new sibling must exist after rotation");
                    }

                    // Case 8 (mirrored 4)
                    let xp_color = x_parent.borrow().color.clone();
                    w.borrow_mut().color = xp_color;
                    x_parent.borrow_mut().color = Color::Black;
                    let w_left_now = w.borrow().left.clone();
                    if let Some(wl) = &w_left_now {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(x_parent.clone());
                    x = self.root.clone().expect("root must exist after rotation");
                }
            }
        }

        x.borrow_mut().color = Color::Black;
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let mut y = p.clone();
        let mut y_original_color = y.borrow().color.clone();
        let x: Option<NodeRef>;
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
            // Both children exist - find successor (min of right subtree).
            let mut succ = p_right.clone().expect("right child exists");
            loop {
                let l = succ.borrow().left.clone();
                match l {
                    Some(n) => succ = n,
                    None => break,
                }
            }
            y = succ;
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            let y_parent = y.borrow().parent.clone();
            let y_parent_is_p = match &y_parent {
                Some(yp) => Rc::ptr_eq(yp, &p),
                None => false,
            };

            if y_parent_is_p {
                // x.parent = y (in C this also writes to nil's parent slot)
                if let Some(xn) = &x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                let y_right = y.borrow().right.clone();
                // x's parent ends up being y's original parent after this transplant.
                x_parent = y.borrow().parent.clone();
                self.transplant(y.clone(), y_right);
                // y.right = p.right; y.right.parent = y;
                y.borrow_mut().right = p_right.clone();
                if let Some(pr) = &p_right {
                    pr.borrow_mut().parent = Some(y.clone());
                }
            }

            // Replace p with y; copy p's left subtree under y; preserve color.
            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p_left.clone();
            if let Some(pl) = &p_left {
                pl.borrow_mut().parent = Some(y.clone());
            }
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        // Detach `p` so its memory can be released safely (avoid lingering parent links).
        {
            let mut pb = p.borrow_mut();
            pb.left = None;
            pb.right = None;
            pb.parent = None;
        }

        if y_original_color == Color::Black {
            if x.is_some() {
                self.delete_fixup(x);
            } else {
                // Use a temporary sentinel node so delete_fixup can navigate
                // via x.parent. After fixup, unlink the sentinel.
                let sentinel = Rc::new(RefCell::new(Node {
                    key: 0,
                    color: Color::Black,
                    left: None,
                    right: None,
                    parent: x_parent.clone(),
                }));

                // Place sentinel into the tree at x's would-be position.
                match &x_parent {
                    None => {
                        // x was the root (root is now None after transplants).
                        self.root = Some(sentinel.clone());
                    }
                    Some(xp) => {
                        // Exactly one child slot of x_parent should be None
                        // — that slot is where x conceptually lived.
                        let xp_left = xp.borrow().left.clone();
                        if xp_left.is_none() {
                            xp.borrow_mut().left = Some(sentinel.clone());
                        } else {
                            xp.borrow_mut().right = Some(sentinel.clone());
                        }
                    }
                }

                self.delete_fixup(Some(sentinel.clone()));

                // Remove sentinel from the tree.
                let sent_parent = sentinel.borrow().parent.clone();
                match &sent_parent {
                    None => {
                        let root_is_sentinel = match &self.root {
                            Some(r) => Rc::ptr_eq(r, &sentinel),
                            None => false,
                        };
                        if root_is_sentinel {
                            self.root = None;
                        }
                    }
                    Some(sp) => {
                        let sp_left = sp.borrow().left.clone();
                        let sentinel_is_left = match &sp_left {
                            Some(l) => Rc::ptr_eq(l, &sentinel),
                            None => false,
                        };
                        if sentinel_is_left {
                            sp.borrow_mut().left = None;
                        } else {
                            let sp_right = sp.borrow().right.clone();
                            let sentinel_is_right = match &sp_right {
                                Some(r) => Rc::ptr_eq(r, &sentinel),
                                None => false,
                            };
                            if sentinel_is_right {
                                sp.borrow_mut().right = None;
                            }
                        }
                    }
                }
                sentinel.borrow_mut().parent = None;
            }
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
        if let Some(node) = curr {
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
