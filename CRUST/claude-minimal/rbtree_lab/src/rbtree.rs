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
        // y = x.left  (must exist; rotation requires non-nil pivot child)
        let y = x
            .borrow()
            .left
            .clone()
            .expect("right_rotate: x.left must be non-nil");

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

        // Hook y up to x's old parent slot.
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

        // y.right = x
        y.borrow_mut().right = Some(x.clone());
        // x.parent = y
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        // y = x.right (must exist)
        let y = x
            .borrow()
            .right
            .clone()
            .expect("left_rotate: x.right must be non-nil");

        // x.right = y.left
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();

        // if y.left != nil: y.left.parent = x
        if let Some(yl) = &y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();

        // Hook y up to x's old parent slot.
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
        // Postorder traversal, breaking parent cycles so the Rc counts drop
        // to zero and memory is reclaimed.
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
    pub fn delete_rbtree(self) {
        Self::free_node(self.root);
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;
        loop {
            // If z has no parent (z is root) or parent is black, stop.
            let parent = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            if parent.borrow().color != Color::Red {
                break;
            }

            // A red parent cannot be the root, so a grandparent must exist.
            let grandparent = parent
                .borrow()
                .parent
                .clone()
                .expect("red parent must have a grandparent");

            let parent_is_left = grandparent
                .borrow()
                .left
                .as_ref()
                .map_or(false, |l| Rc::ptr_eq(l, &parent));

            if parent_is_left {
                // Uncle y = grandparent.right
                let y = grandparent.borrow().right.clone();
                let y_is_red = y
                    .as_ref()
                    .map_or(false, |yn| yn.borrow().color == Color::Red);

                if y_is_red {
                    // CASE 1: recolor and move z up two levels.
                    parent.borrow_mut().color = Color::Black;
                    if let Some(yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    grandparent.borrow_mut().color = Color::Red;
                    z = grandparent;
                } else {
                    // CASE 2: z is a right child -> rotate around parent.
                    let z_is_right = parent
                        .borrow()
                        .right
                        .as_ref()
                        .map_or(false, |r| Rc::ptr_eq(r, &z));
                    if z_is_right {
                        z = parent.clone();
                        self.left_rotate(z.clone());
                    }
                    // CASE 3.
                    let zp = z.borrow().parent.clone().unwrap();
                    zp.borrow_mut().color = Color::Black;
                    let zpp = zp.borrow().parent.clone().unwrap();
                    zpp.borrow_mut().color = Color::Red;
                    self.right_rotate(zpp);
                }
            } else {
                // Mirror: parent is grandparent's right child.
                let y = grandparent.borrow().left.clone();
                let y_is_red = y
                    .as_ref()
                    .map_or(false, |yn| yn.borrow().color == Color::Red);

                if y_is_red {
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
                    if z_is_left {
                        z = parent.clone();
                        self.right_rotate(z.clone());
                    }
                    let zp = z.borrow().parent.clone().unwrap();
                    zp.borrow_mut().color = Color::Black;
                    let zpp = zp.borrow().parent.clone().unwrap();
                    zpp.borrow_mut().color = Color::Red;
                    self.left_rotate(zpp);
                }
            }
        }

        // Root is always black.
        if let Some(r) = &self.root {
            r.borrow_mut().color = Color::Black;
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
            let next = if key < curr.borrow().key {
                curr.borrow().left.clone()
            } else {
                curr.borrow().right.clone()
            };
            x = next;
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

        // z.left, z.right are already None (== nil), color is already Red.
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
        // The C version writes nil->parent unconditionally; with Option-based
        // nil that's a no-op, so we only update v's parent when v exists.
        if let Some(vn) = &v {
            vn.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let mut x = x;
        loop {
            // while (x != root && x->color == BLACK)
            let is_root = match (&x, &self.root) {
                (None, None) => true,
                (Some(xn), Some(rn)) => Rc::ptr_eq(xn, rn),
                _ => false,
            };
            if is_root {
                break;
            }
            // We need x to navigate via x.parent. With the Option-as-nil model
            // we cannot recover the parent of a None x, so bail out.
            let xn = match &x {
                Some(n) => n.clone(),
                None => break,
            };
            if xn.borrow().color != Color::Black {
                break;
            }

            let parent = xn
                .borrow()
                .parent
                .clone()
                .expect("non-root node must have a parent");
            let xn_is_left = parent
                .borrow()
                .left
                .as_ref()
                .map_or(false, |l| Rc::ptr_eq(l, &xn));

            if xn_is_left {
                let mut w = parent
                    .borrow()
                    .right
                    .clone()
                    .expect("sibling must exist for fixup");

                // CASE 1: red sibling.
                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .right
                        .clone()
                        .expect("sibling must exist after rotation");
                }

                let w_left_black = w
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(true, |l| l.borrow().color == Color::Black);
                let w_right_black = w
                    .borrow()
                    .right
                    .as_ref()
                    .map_or(true, |r| r.borrow().color == Color::Black);

                // CASE 2: both nephews black.
                if w_left_black && w_right_black {
                    w.borrow_mut().color = Color::Red;
                    x = Some(parent.clone());
                } else {
                    // CASE 3: right nephew black, left nephew red.
                    if w_right_black {
                        if let Some(wl) = w.borrow().left.clone() {
                            wl.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent.borrow().right.clone().unwrap();
                    }
                    // CASE 4: right nephew red.
                    let p_color = parent.borrow().color.clone();
                    w.borrow_mut().color = p_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wr) = w.borrow().right.clone() {
                        wr.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                }
            } else {
                // Mirror cases 5-8.
                let mut w = parent
                    .borrow()
                    .left
                    .clone()
                    .expect("sibling must exist for fixup");

                if w.borrow().color == Color::Red {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent
                        .borrow()
                        .left
                        .clone()
                        .expect("sibling must exist after rotation");
                }

                let w_left_black = w
                    .borrow()
                    .left
                    .as_ref()
                    .map_or(true, |l| l.borrow().color == Color::Black);
                let w_right_black = w
                    .borrow()
                    .right
                    .as_ref()
                    .map_or(true, |r| r.borrow().color == Color::Black);

                if w_right_black && w_left_black {
                    w.borrow_mut().color = Color::Red;
                    x = Some(parent.clone());
                } else {
                    if w_left_black {
                        if let Some(wr) = w.borrow().right.clone() {
                            wr.borrow_mut().color = Color::Black;
                        }
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent.borrow().left.clone().unwrap();
                    }
                    let p_color = parent.borrow().color.clone();
                    w.borrow_mut().color = p_color;
                    parent.borrow_mut().color = Color::Black;
                    if let Some(wl) = w.borrow().left.clone() {
                        wl.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(parent.clone());
                    x = self.root.clone();
                }
            }
        }

        if let Some(xn) = x {
            xn.borrow_mut().color = Color::Black;
        }
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let y_original_color: Color;
        let x: Option<NodeRef>;

        let p_left = p.borrow().left.clone();
        let p_right = p.borrow().right.clone();

        if p_left.is_none() {
            x = p_right.clone();
            y_original_color = p.borrow().color.clone();
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            x = p_left.clone();
            y_original_color = p.borrow().color.clone();
            self.transplant(p.clone(), p_left);
        } else {
            // Find successor: minimum of p.right.
            let mut y_node = p.borrow().right.clone().unwrap();
            loop {
                let next = y_node.borrow().left.clone();
                match next {
                    Some(n) => y_node = n,
                    None => break,
                }
            }
            let y = y_node;
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            let y_parent = y.borrow().parent.clone();
            let y_parent_is_p = y_parent
                .as_ref()
                .map_or(false, |yp| Rc::ptr_eq(yp, &p));

            if y_parent_is_p {
                // The C code sets x->parent = y here even when x is nil; with
                // Option-based nil we just update real children.
                if let Some(xn) = &x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
            } else {
                let y_right = y.borrow().right.clone();
                self.transplant(y.clone(), y_right);
                let p_right_again = p.borrow().right.clone();
                y.borrow_mut().right = p_right_again.clone();
                if let Some(yr) = p_right_again {
                    yr.borrow_mut().parent = Some(y.clone());
                }
            }

            self.transplant(p.clone(), Some(y.clone()));
            let p_left_again = p.borrow().left.clone();
            y.borrow_mut().left = p_left_again.clone();
            if let Some(yl) = p_left_again {
                yl.borrow_mut().parent = Some(y.clone());
            }
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        if y_original_color == Color::Black {
            self.delete_fixup(x);
        }

        // Drop p's links so the Rc holding it (the local parameter) can fully
        // release once it goes out of scope.
        {
            let mut pb = p.borrow_mut();
            pb.left = None;
            pb.right = None;
            pb.parent = None;
        }
        drop(p);
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
        let mut arr = Vec::with_capacity(n);
        if self.root.is_none() {
            return arr;
        }
        let mut count: usize = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}
