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

// Helpers (private)
fn color_of(n: &Option<NodeRef>) -> Color {
    match n {
        None => Color::Black,
        Some(node) => node.borrow().color.clone(),
    }
}

fn left_of(n: &Option<NodeRef>) -> Option<NodeRef> {
    n.as_ref().and_then(|x| x.borrow().left.clone())
}

fn right_of(n: &Option<NodeRef>) -> Option<NodeRef> {
    n.as_ref().and_then(|x| x.borrow().right.clone())
}

fn opt_eq(a: &Option<NodeRef>, b: &Option<NodeRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => Rc::ptr_eq(x, y),
        _ => false,
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
            .expect("right_rotate: x.left must be non-NIL");
        // x.left = y.right
        let y_right = y.borrow().right.clone();
        x.borrow_mut().left = y_right.clone();
        if let Some(yr) = y_right {
            yr.borrow_mut().parent = Some(x.clone());
        }
        // y.parent = x.parent
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();
        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let is_left = match xp.borrow().left.clone() {
                    Some(xpl) => Rc::ptr_eq(&xpl, &x),
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
            .expect("left_rotate: x.right must be non-NIL");
        let y_left = y.borrow().left.clone();
        x.borrow_mut().right = y_left.clone();
        if let Some(yl) = y_left {
            yl.borrow_mut().parent = Some(x.clone());
        }
        let x_parent = x.borrow().parent.clone();
        y.borrow_mut().parent = x_parent.clone();
        match x_parent {
            None => {
                self.root = Some(y.clone());
            }
            Some(xp) => {
                let is_left = match xp.borrow().left.clone() {
                    Some(xpl) => Rc::ptr_eq(&xpl, &x),
                    None => false,
                };
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
            // Take children out, breaking parent <-> child cycles.
            let l = n.borrow_mut().left.take();
            let r = n.borrow_mut().right.take();
            n.borrow_mut().parent = None;
            Self::free_node(l);
            Self::free_node(r);
        }
    }

    /// Deletes the Red-Black Tree safely.
    pub fn delete_rbtree(self) {
        Self::free_node(self.root);
    }

    /// Fixes the Red-Black Tree after insertion (z must be non-NIL).
    pub fn rbtree_insert_fixup(&mut self, mut z: NodeRef) {
        loop {
            // while z.parent.color == RED
            let z_parent = match z.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            if z_parent.borrow().color != Color::Red {
                break;
            }
            // z's grandparent
            let zpp = match z_parent.borrow().parent.clone() {
                Some(p) => p,
                None => break,
            };
            // Is z's parent the left child of zpp?
            let parent_is_left = match zpp.borrow().left.clone() {
                Some(zpl) => Rc::ptr_eq(&zpl, &z_parent),
                None => false,
            };
            if parent_is_left {
                let y = zpp.borrow().right.clone(); // uncle
                if color_of(&y) == Color::Red {
                    // CASE 1
                    z_parent.borrow_mut().color = Color::Black;
                    if let Some(ref yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp.clone();
                } else {
                    // CASE 2: z is right child -> left rotate parent
                    let z_is_right = match z_parent.borrow().right.clone() {
                        Some(ref zr) => Rc::ptr_eq(zr, &z),
                        None => false,
                    };
                    if z_is_right {
                        z = z_parent.clone();
                        self.left_rotate(z.clone());
                    }
                    // CASE 3
                    let zp = z.borrow().parent.clone().unwrap();
                    zp.borrow_mut().color = Color::Black;
                    let zpp2 = zp.borrow().parent.clone().unwrap();
                    zpp2.borrow_mut().color = Color::Red;
                    self.right_rotate(zpp2);
                }
            } else {
                let y = zpp.borrow().left.clone();
                if color_of(&y) == Color::Red {
                    // CASE 4
                    z_parent.borrow_mut().color = Color::Black;
                    if let Some(ref yn) = y {
                        yn.borrow_mut().color = Color::Black;
                    }
                    zpp.borrow_mut().color = Color::Red;
                    z = zpp.clone();
                } else {
                    // CASE 5
                    let z_is_left = match z_parent.borrow().left.clone() {
                        Some(ref zl) => Rc::ptr_eq(zl, &z),
                        None => false,
                    };
                    if z_is_left {
                        z = z_parent.clone();
                        self.right_rotate(z.clone());
                    }
                    // CASE 6
                    let zp = z.borrow().parent.clone().unwrap();
                    zp.borrow_mut().color = Color::Black;
                    let zpp2 = zp.borrow().parent.clone().unwrap();
                    zpp2.borrow_mut().color = Color::Red;
                    self.left_rotate(zpp2);
                }
            }
        }
        if let Some(ref root) = self.root {
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
        while let Some(xn) = x.clone() {
            y = Some(xn.clone());
            if z.borrow().key < xn.borrow().key {
                x = xn.borrow().left.clone();
            } else {
                x = xn.borrow().right.clone();
            }
        }

        z.borrow_mut().parent = y.clone();
        match y {
            None => {
                self.root = Some(z.clone());
            }
            Some(yn) => {
                if z.borrow().key < yn.borrow().key {
                    yn.borrow_mut().left = Some(z.clone());
                } else {
                    yn.borrow_mut().right = Some(z.clone());
                }
            }
        }

        // z.left/right/color already initialized (None/None/Red)
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
                None => break,
            }
        }
        Some(curr)
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        loop {
            let next = curr.borrow().right.clone();
            match next {
                Some(n) => curr = n,
                None => break,
            }
        }
        Some(curr)
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let u_parent = u.borrow().parent.clone();
        match u_parent.clone() {
            None => {
                self.root = v.clone();
            }
            Some(up) => {
                let is_left = match up.borrow().left.clone() {
                    Some(upl) => Rc::ptr_eq(&upl, &u),
                    None => false,
                };
                if is_left {
                    up.borrow_mut().left = v.clone();
                } else {
                    up.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(ref vn) = v {
            vn.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        // Public wrapper: if x is Some we can derive its parent. If x is None,
        // there is nothing to fix without external context, so just ensure root
        // is black.
        let x_parent = x.as_ref().and_then(|n| n.borrow().parent.clone());
        self.delete_fixup_inner(x, x_parent);
    }

    fn delete_fixup_inner(
        &mut self,
        mut x: Option<NodeRef>,
        mut x_parent: Option<NodeRef>,
    ) {
        loop {
            // while x != root && x.color == BLACK
            if opt_eq(&x, &self.root) {
                break;
            }
            if color_of(&x) == Color::Red {
                break;
            }
            let xp = match x_parent.clone() {
                Some(p) => p,
                None => break,
            };
            // Determine if x is xp.left
            let xp_left = xp.borrow().left.clone();
            let x_is_left = opt_eq(&xp_left, &x);
            if x_is_left {
                let mut w = xp.borrow().right.clone();
                // CASE 1: sibling w is RED
                if color_of(&w) == Color::Red {
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = Color::Black;
                    }
                    xp.borrow_mut().color = Color::Red;
                    self.left_rotate(xp.clone());
                    w = xp.borrow().right.clone();
                }
                // CASE 2: w is BLACK and both children are BLACK
                let wl = left_of(&w);
                let wr = right_of(&w);
                if color_of(&wl) == Color::Black && color_of(&wr) == Color::Black {
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = Color::Red;
                    }
                    x = Some(xp.clone());
                    x_parent = xp.borrow().parent.clone();
                } else {
                    // CASE 3: w.right is BLACK
                    if color_of(&wr) == Color::Black {
                        if let Some(ref wln) = wl {
                            wln.borrow_mut().color = Color::Black;
                        }
                        if let Some(ref wn) = w {
                            wn.borrow_mut().color = Color::Red;
                            self.right_rotate(wn.clone());
                        }
                        w = xp.borrow().right.clone();
                    }
                    // CASE 4
                    let xp_color = xp.borrow().color.clone();
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = xp_color;
                    }
                    xp.borrow_mut().color = Color::Black;
                    let wr2 = right_of(&w);
                    if let Some(wrn) = wr2 {
                        wrn.borrow_mut().color = Color::Black;
                    }
                    self.left_rotate(xp.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            } else {
                let mut w = xp.borrow().left.clone();
                // CASE 5: sibling w is RED
                if color_of(&w) == Color::Red {
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = Color::Black;
                    }
                    xp.borrow_mut().color = Color::Red;
                    self.right_rotate(xp.clone());
                    w = xp.borrow().left.clone();
                }
                let wl = left_of(&w);
                let wr = right_of(&w);
                // CASE 6
                if color_of(&wr) == Color::Black && color_of(&wl) == Color::Black {
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = Color::Red;
                    }
                    x = Some(xp.clone());
                    x_parent = xp.borrow().parent.clone();
                } else {
                    // CASE 7
                    if color_of(&wl) == Color::Black {
                        if let Some(ref wrn) = wr {
                            wrn.borrow_mut().color = Color::Black;
                        }
                        if let Some(ref wn) = w {
                            wn.borrow_mut().color = Color::Red;
                            self.left_rotate(wn.clone());
                        }
                        w = xp.borrow().left.clone();
                    }
                    // CASE 8
                    let xp_color = xp.borrow().color.clone();
                    if let Some(ref wn) = w {
                        wn.borrow_mut().color = xp_color;
                    }
                    xp.borrow_mut().color = Color::Black;
                    let wl2 = left_of(&w);
                    if let Some(wln) = wl2 {
                        wln.borrow_mut().color = Color::Black;
                    }
                    self.right_rotate(xp.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            }
        }
        if let Some(ref xn) = x {
            xn.borrow_mut().color = Color::Black;
        }
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
            // Successor: leftmost in right subtree
            y = p_right.clone().unwrap();
            loop {
                let yl = y.borrow().left.clone();
                match yl {
                    Some(yln) => y = yln,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            let y_parent_is_p = match y.borrow().parent.clone() {
                Some(yp) => Rc::ptr_eq(&yp, &p),
                None => false,
            };
            if y_parent_is_p {
                if let Some(ref xn) = x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                // Capture y's old parent BEFORE transplant changes anything
                // structurally for x's logical parent.
                x_parent = y.borrow().parent.clone();
                let y_right = y.borrow().right.clone();
                self.transplant(y.clone(), y_right);
                let p_right_node = p.borrow().right.clone().unwrap();
                y.borrow_mut().right = Some(p_right_node.clone());
                p_right_node.borrow_mut().parent = Some(y.clone());
            }
            // Replace p with y
            self.transplant(p.clone(), Some(y.clone()));
            let p_left_node = p.borrow().left.clone().unwrap();
            y.borrow_mut().left = Some(p_left_node.clone());
            p_left_node.borrow_mut().parent = Some(y.clone());
            let p_color = p.borrow().color.clone();
            y.borrow_mut().color = p_color;
        }

        // Detach p from the tree (clean up references).
        p.borrow_mut().parent = None;
        p.borrow_mut().left = None;
        p.borrow_mut().right = None;

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
        let mut count: usize = 0;
        if let Some(ref root) = self.root {
            self.subtree_to_array(Some(root.clone()), &mut arr, n, &mut count);
        }
        arr
    }
}
