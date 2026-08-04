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
        // y = x.left (must be non-NIL)
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate: x.left must not be NIL");

        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();

        // if y.right != NIL: y.right.parent = x
        if let Some(yr) = y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match &x_parent {
            None => {
                // x was root; now y is the root
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_left = xp.borrow().left.clone();
                let is_left = match &xp_left {
                    Some(xpl) => Rc::ptr_eq(xpl, &x),
                    None => false,
                };
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
        // y = x.right (must be non-NIL)
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate: x.right must not be NIL");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y.left != NIL: y.left.parent = x
        if let Some(yl) = y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        match &x_parent {
            None => {
                // x was root; now y is the root
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let xp_left = xp.borrow().left.clone();
                let is_left = match &xp_left {
                    Some(xpl) => Rc::ptr_eq(xpl, &x),
                    None => false,
                };
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
            let (left, right) = {
                let mut nb = n.borrow_mut();
                nb.parent = None;
                (nb.left.take(), nb.right.take())
            };
            Self::free_node(left);
            Self::free_node(right);
            // `n` (and its inner RefCell<Node>) is dropped when its Rc count goes to 0.
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
            // parent must exist and be RED for the loop to continue
            let parent = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }

            // Since parent is RED, parent is not root, so grandparent must exist.
            let grandparent = match parent.borrow().parent.clone() {
                Some(g) => g,
                None => break,
            };

            let parent_is_left = match grandparent.borrow().left.clone() {
                Some(gl) => Rc::ptr_eq(&gl, &parent),
                None => false,
            };

            if parent_is_left {
                let uncle = grandparent.borrow().right.clone();
                let uncle_is_red = match &uncle {
                    Some(u) => u.borrow().color == Color::Red,
                    None => false,
                };

                if uncle_is_red {
                    // CASE 1: uncle is RED
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 2: z is right child -> rotate to fall through to CASE 3
                    let z_is_right = match parent.borrow().right.clone() {
                        Some(pr) => Rc::ptr_eq(&pr, &z),
                        None => false,
                    };
                    if z_is_right {
                        z = parent.clone();
                        self.left_rotate(z.clone());
                    }
                    // CASE 3: z is left child
                    let p = z
                        .borrow()
                        .parent
                        .clone()
                        .expect("post-rotation parent must exist");
                    p.borrow_mut().color = Color::Black;
                    let g = p
                        .borrow()
                        .parent
                        .clone()
                        .expect("post-rotation grandparent must exist");
                    g.borrow_mut().color = Color::Red;
                    self.right_rotate(g);
                }
            } else {
                // mirror image: parent is right child of grandparent
                let uncle = grandparent.borrow().left.clone();
                let uncle_is_red = match &uncle {
                    Some(u) => u.borrow().color == Color::Red,
                    None => false,
                };

                if uncle_is_red {
                    // CASE 4
                    parent.borrow_mut().color = Color::Black;
                    if let Some(u) = uncle {
                        u.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 5
                    let z_is_left = match parent.borrow().left.clone() {
                        Some(pl) => Rc::ptr_eq(&pl, &z),
                        None => false,
                    };
                    if z_is_left {
                        z = parent.clone();
                        self.right_rotate(z.clone());
                    }
                    // CASE 6
                    let p = z
                        .borrow()
                        .parent
                        .clone()
                        .expect("post-rotation parent must exist");
                    p.borrow_mut().color = Color::Black;
                    let g = p
                        .borrow()
                        .parent
                        .clone()
                        .expect("post-rotation grandparent must exist");
                    g.borrow_mut().color = Color::Red;
                    self.left_rotate(g);
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

        // Walk down the tree to find insertion point
        let mut y: Option<NodeRef> = None;
        let mut x_opt: Option<NodeRef> = self.root.clone();

        while let Some(x) = x_opt {
            y = Some(x.clone());
            let x_key = x.borrow().key;
            if key < x_key {
                x_opt = x.borrow().left.clone();
            } else {
                x_opt = x.borrow().right.clone();
            }
        }

        z.borrow_mut().parent = y.clone();

        match y {
            None => {
                self.root = Some(z.clone());
            }
            Some(yy) => {
                let y_key = yy.borrow().key;
                if key < y_key {
                    yy.borrow_mut().left = Some(z.clone());
                } else {
                    yy.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // z's left/right already None (NIL); color already Red.
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
            if c_key < key {
                current = c.borrow().right.clone();
            } else {
                current = c.borrow().left.clone();
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
        match &u_parent {
            None => {
                self.root = v.clone();
            }
            Some(up) => {
                let up_left = up.borrow().left.clone();
                let is_left = match &up_left {
                    Some(upl) => Rc::ptr_eq(upl, &u),
                    None => false,
                };
                if is_left {
                    up.borrow_mut().left = v.clone();
                } else {
                    up.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(vv) = v {
            vv.borrow_mut().parent = u_parent;
        }
        // Note: when v is None (NIL), there's no place to record its parent.
        // Callers (e.g., `erase`) must track the logical parent themselves.
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    /// When `x` is `Some`, parent is derived from `x.parent`. When `x` is `None`,
    /// no fixup can be performed (caller should use `erase` which preserves
    /// parent context internally).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let parent = x.as_ref().and_then(|xx| xx.borrow().parent.clone());
        self.delete_fixup_inner(x, parent);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let p_color = p.borrow().color.clone();
        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        let mut y_original_color = p_color.clone();
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p_left.is_none() {
            // x = p.right; transplant(p, p.right)
            x = p_right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            // x = p.left; transplant(p, p.left)
            x = p_left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p_left);
        } else {
            // Find y = minimum of p's right subtree
            let mut y = p_right.clone().unwrap();
            loop {
                let yl = y.borrow().left.clone();
                match yl {
                    Some(node) => y = node,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            let y_parent = y.borrow().parent.clone();
            let y_parent_is_p = match &y_parent {
                Some(yp) => Rc::ptr_eq(yp, &p),
                None => false,
            };

            if y_parent_is_p {
                // x's logical parent is y
                if let Some(xx) = &x {
                    xx.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                // x's logical parent is y's old parent (before any transplants)
                let y_old_parent = y.borrow().parent.clone();
                let y_right = y.borrow().right.clone();
                self.transplant(y.clone(), y_right);
                // y.right = p.right
                y.borrow_mut().right = p_right.clone();
                if let Some(pr) = p_right {
                    pr.borrow_mut().parent = Some(y.clone());
                }
                x_parent = y_old_parent;
            }

            // transplant(p, y); y.left = p.left; y.left.parent = y; y.color = p.color
            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p_left.clone();
            if let Some(pl) = p_left {
                pl.borrow_mut().parent = Some(y.clone());
            }
            y.borrow_mut().color = p_color;
        }

        // Detach p from the tree (best-effort cleanup)
        {
            let mut pb = p.borrow_mut();
            pb.parent = None;
            pb.left = None;
            pb.right = None;
        }

        if y_original_color == Color::Black {
            self.delete_fixup_inner(x, x_parent);
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
}

impl RBTree {
    /// Internal delete-fixup that takes an explicit logical parent for `x`,
    /// since `None` (NIL) cannot store a parent pointer.
    fn delete_fixup_inner(&mut self, mut x: Option<NodeRef>, mut x_parent: Option<NodeRef>) {
        loop {
            // Stop if x is the root
            let x_is_root = match (&x, &self.root) {
                (None, None) => true,
                (Some(xx), Some(root)) => Rc::ptr_eq(xx, root),
                _ => false,
            };
            if x_is_root {
                break;
            }
            // Stop if x is RED
            let x_color = match &x {
                Some(xx) => xx.borrow().color.clone(),
                None => Color::Black,
            };
            if x_color != Color::Black {
                break;
            }

            let parent = match x_parent.clone() {
                Some(p) => p,
                None => break, // x has no parent and isn't root: nothing to fix.
            };

            // Determine if x is left child of parent.
            let x_is_left = match &x {
                Some(xx) => match parent.borrow().left.clone() {
                    Some(pl) => Rc::ptr_eq(&pl, xx),
                    None => false,
                },
                None => parent.borrow().left.is_none(),
            };

            if x_is_left {
                let mut w = parent
                    .borrow()
                    .right
                    .clone()
                    .expect("delete_fixup: sibling must exist");

                // CASE 1: w is RED
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .right
                        .clone()
                        .expect("delete_fixup: new sibling must exist");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let w_left_black = match &w_left {
                    Some(n) => n.borrow().color == Color::Black,
                    None => true,
                };
                let w_right_black = match &w_right {
                    Some(n) => n.borrow().color == Color::Black,
                    None => true,
                };

                if w_left_black && w_right_black {
                    // CASE 2: both of w's children are BLACK
                    w.borrow_mut().color = Color::Red;
                    let new_parent = parent.borrow().parent.clone();
                    x = Some(parent.clone());
                    x_parent = new_parent;
                } else {
                    if w_right_black {
                        // CASE 3
                        if let Some(wl) = w_left {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent
                            .borrow()
                            .right
                            .clone()
                            .expect("delete_fixup: rotated sibling must exist");
                    }
                    // CASE 4
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                    break;
                }
            } else {
                // mirror image
                let mut w = parent
                    .borrow()
                    .left
                    .clone()
                    .expect("delete_fixup: sibling must exist");

                // CASE 5: w is RED
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .left
                        .clone()
                        .expect("delete_fixup: new sibling must exist");
                }

                let w_left = w.borrow().left.clone();
                let w_right = w.borrow().right.clone();
                let w_left_black = match &w_left {
                    Some(n) => n.borrow().color == Color::Black,
                    None => true,
                };
                let w_right_black = match &w_right {
                    Some(n) => n.borrow().color == Color::Black,
                    None => true,
                };

                if w_right_black && w_left_black {
                    // CASE 6
                    w.borrow_mut().color = Color::Red;
                    let new_parent = parent.borrow().parent.clone();
                    x = Some(parent.clone());
                    x_parent = new_parent;
                } else {
                    if w_left_black {
                        // CASE 7
                        if let Some(wr) = w_right {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent
                            .borrow()
                            .left
                            .clone()
                            .expect("delete_fixup: rotated sibling must exist");
                    }
                    // CASE 8
                    let parent_color = parent.borrow().color.clone();
                    w.borrow_mut().color = parent_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
                    x = self.root.clone();
                    break;
                }
            }
        }
        if let Some(xx) = x {
            xx.borrow_mut().color = Color::Black;
        }
    }
}
