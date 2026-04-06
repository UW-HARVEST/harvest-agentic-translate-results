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

fn new_node(key: Key) -> NodeRef {
    Rc::new(RefCell::new(Node {
        key,
        color: Color::Red,
        left: None,
        right: None,
        parent: None,
    }))
}

fn color_of(n: &Option<NodeRef>) -> Color {
    match n {
        Some(r) => r.borrow().color.clone(),
        None => Color::Black,
    }
}

fn is_same(a: &Option<NodeRef>, b: &Option<NodeRef>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => Rc::ptr_eq(x, y),
        (None, None) => true,
        _ => false,
    }
}

fn parent_of(n: &Option<NodeRef>) -> Option<NodeRef> {
    n.as_ref().and_then(|r| r.borrow().parent.clone())
}

fn left_of(n: &Option<NodeRef>) -> Option<NodeRef> {
    n.as_ref().and_then(|r| r.borrow().left.clone())
}

fn right_of(n: &Option<NodeRef>) -> Option<NodeRef> {
    n.as_ref().and_then(|r| r.borrow().right.clone())
}

fn set_color(n: &Option<NodeRef>, c: Color) {
    if let Some(r) = n {
        r.borrow_mut().color = c;
    }
}

fn set_parent(n: &Option<NodeRef>, p: &Option<NodeRef>) {
    if let Some(r) = n {
        r.borrow_mut().parent = p.clone();
    }
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        let y = x.borrow().left.clone().unwrap();
        // x.left = y.right
        let yr = y.borrow().right.clone();
        x.borrow_mut().left = yr.clone();
        set_parent(&yr, &Some(x.clone()));
        // y.parent = x.parent
        let xp = x.borrow().parent.clone();
        y.borrow_mut().parent = xp.clone();
        match xp {
            None => self.root = Some(y.clone()),
            Some(ref p) => {
                if is_same(&p.borrow().left, &Some(x.clone())) {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        let y = x.borrow().right.clone().unwrap();
        let yl = y.borrow().left.clone();
        x.borrow_mut().right = yl.clone();
        set_parent(&yl, &Some(x.clone()));
        let xp = x.borrow().parent.clone();
        y.borrow_mut().parent = xp.clone();
        match xp {
            None => self.root = Some(y.clone()),
            Some(ref p) => {
                if is_same(&p.borrow().left, &Some(x.clone())) {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }
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
            n.borrow_mut().parent = None;
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
        while color_of(&parent_of(&Some(z.clone()))) == Color::Red {
            let zp = z.borrow().parent.clone().unwrap();
            let zpp = zp.borrow().parent.clone().unwrap();
            if is_same(&zpp.borrow().left, &Some(zp.clone())) {
                // z's parent is left child of grandparent
                let y = zpp.borrow().right.clone(); // uncle
                if color_of(&y) == Color::Red {
                    // Case 1
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&y, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    if is_same(&zp.borrow().right, &Some(z.clone())) {
                        // Case 2
                        z = zp.clone();
                        self.left_rotate(z.clone());
                    }
                    // Case 3
                    let zp = z.borrow().parent.clone().unwrap();
                    let zpp = zp.borrow().parent.clone().unwrap();
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.right_rotate(zpp);
                }
            } else {
                // z's parent is right child of grandparent
                let y = zpp.borrow().left.clone(); // uncle
                if color_of(&y) == Color::Red {
                    // Case 4
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&y, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    if is_same(&zp.borrow().left, &Some(z.clone())) {
                        // Case 5
                        z = zp.clone();
                        self.right_rotate(z.clone());
                    }
                    // Case 6
                    let zp = z.borrow().parent.clone().unwrap();
                    let zpp = zp.borrow().parent.clone().unwrap();
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.left_rotate(zpp);
                }
            }
        }
        if let Some(ref r) = self.root {
            r.borrow_mut().color = Color::Black;
        }
    }

    /// Inserts a new key and returns the inserted node.
    pub fn rbtree_insert(&mut self, key: Key) -> Option<NodeRef> {
        let z = new_node(key);
        let mut y: Option<NodeRef> = None;
        let mut x = self.root.clone();
        while let Some(curr) = x {
            y = Some(curr.clone());
            if key < curr.borrow().key {
                x = curr.borrow().left.clone();
            } else {
                x = curr.borrow().right.clone();
            }
        }
        z.borrow_mut().parent = y.clone();
        match y {
            None => self.root = Some(z.clone()),
            Some(ref ynode) => {
                if key < ynode.borrow().key {
                    ynode.borrow_mut().left = Some(z.clone());
                } else {
                    ynode.borrow_mut().right = Some(z.clone());
                }
            }
        }
        z.borrow_mut().color = Color::Red;
        self.rbtree_insert_fixup(z.clone());
        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut curr = self.root.clone();
        while let Some(node) = curr {
            let k = node.borrow().key;
            if k == key {
                return Some(node);
            } else if k < key {
                curr = node.borrow().right.clone();
            } else {
                curr = node.borrow().left.clone();
            }
        }
        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        while curr.borrow().left.is_some() {
            let next = curr.borrow().left.clone().unwrap();
            curr = next;
        }
        Some(curr)
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone()?;
        while curr.borrow().right.is_some() {
            let next = curr.borrow().right.clone().unwrap();
            curr = next;
        }
        Some(curr)
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let up = u.borrow().parent.clone();
        match up {
            None => self.root = v.clone(),
            Some(ref p) => {
                if is_same(&p.borrow().left, &Some(u.clone())) {
                    p.borrow_mut().left = v.clone();
                } else {
                    p.borrow_mut().right = v.clone();
                }
            }
        }
        set_parent(&v, &up);
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let mut x = x;
        while !is_same(&x, &self.root) && color_of(&x) == Color::Black {
            let xp = parent_of(&x).unwrap();
            if is_same(&x, &left_of(&Some(xp.clone()))) {
                // LEFT CASES
                let mut w = right_of(&Some(xp.clone()));
                // Case 1
                if color_of(&w) == Color::Red {
                    set_color(&w, Color::Black);
                    set_color(&Some(xp.clone()), Color::Red);
                    self.left_rotate(xp.clone());
                    w = right_of(&parent_of(&x));
                }
                // Case 2
                if color_of(&left_of(&w)) == Color::Black && color_of(&right_of(&w)) == Color::Black {
                    set_color(&w, Color::Red);
                    x = Some(xp);
                } else {
                    // Case 3
                    if color_of(&right_of(&w)) == Color::Black {
                        set_color(&left_of(&w), Color::Black);
                        set_color(&w, Color::Red);
                        self.right_rotate(w.clone().unwrap());
                        w = right_of(&parent_of(&x));
                    }
                    // Case 4
                    let xp = parent_of(&x).unwrap();
                    set_color(&w, xp.borrow().color.clone());
                    set_color(&Some(xp.clone()), Color::Black);
                    set_color(&right_of(&w), Color::Black);
                    self.left_rotate(xp);
                    x = self.root.clone();
                }
            } else {
                // RIGHT CASES
                let mut w = left_of(&Some(xp.clone()));
                // Case 5
                if color_of(&w) == Color::Red {
                    set_color(&w, Color::Black);
                    set_color(&Some(xp.clone()), Color::Red);
                    self.right_rotate(xp.clone());
                    w = left_of(&parent_of(&x));
                }
                // Case 6
                if color_of(&right_of(&w)) == Color::Black && color_of(&left_of(&w)) == Color::Black {
                    set_color(&w, Color::Red);
                    x = Some(xp);
                } else {
                    // Case 7
                    if color_of(&left_of(&w)) == Color::Black {
                        set_color(&right_of(&w), Color::Black);
                        set_color(&w, Color::Red);
                        self.left_rotate(w.clone().unwrap());
                        w = left_of(&parent_of(&x));
                    }
                    // Case 8
                    let xp = parent_of(&x).unwrap();
                    set_color(&w, xp.borrow().color.clone());
                    set_color(&Some(xp.clone()), Color::Black);
                    set_color(&left_of(&w), Color::Black);
                    self.right_rotate(xp);
                    x = self.root.clone();
                }
            }
        }
        set_color(&x, Color::Black);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let y_orig_color;
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p.borrow().left.is_none() {
            y_orig_color = p.borrow().color.clone();
            x = p.borrow().right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p.borrow().right.clone());
        } else if p.borrow().right.is_none() {
            y_orig_color = p.borrow().color.clone();
            x = p.borrow().left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), p.borrow().left.clone());
        } else {
            // Find successor (min of right subtree)
            let mut y = p.borrow().right.clone().unwrap();
            while y.borrow().left.is_some() {
                let next = y.borrow().left.clone().unwrap();
                y = next;
            }
            y_orig_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            if Rc::ptr_eq(&y.borrow().parent.clone().unwrap(), &p) {
                // x's parent should be y (even if x is None)
                x_parent = Some(y.clone());
                // If x is Some, set its parent
                set_parent(&x, &Some(y.clone()));
            } else {
                x_parent = y.borrow().parent.clone();
                self.transplant(y.clone(), y.borrow().right.clone());
                y.borrow_mut().right = p.borrow().right.clone();
                set_parent(&y.borrow().right, &Some(y.clone()));
            }
            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p.borrow().left.clone();
            set_parent(&y.borrow().left, &Some(y.clone()));
            y.borrow_mut().color = p.borrow().color.clone();
        }

        if y_orig_color == Color::Black {
            // For delete_fixup, if x is None we need its parent accessible.
            // We use a sentinel node temporarily.
            if x.is_none() {
                let sentinel = Rc::new(RefCell::new(Node {
                    key: 0,
                    color: Color::Black,
                    left: None,
                    right: None,
                    parent: x_parent.clone(),
                }));
                // Place sentinel in the tree where x (None) would be
                if let Some(ref par) = x_parent {
                    if is_same(&par.borrow().left, &None) {
                        // Check: was x supposed to be left or right?
                        // If right is also None, we need to figure out which side.
                        // In the C code, x was placed by transplant, so check which child is nil.
                        // We need to determine the correct side. After transplant, x replaced
                        // the node that was moved. Let's check both sides.
                        // Actually, after transplant x is already linked. But x is None.
                        // We need to figure out which child slot x occupies.
                        // The simplest: check if left is None (x could be left).
                        // But both could be None. Let's just put it on left if left is None,
                        // otherwise right.
                        par.borrow_mut().left = Some(sentinel.clone());
                    } else if is_same(&par.borrow().right, &None) {
                        par.borrow_mut().right = Some(sentinel.clone());
                    } else {
                        // Both children are non-None; shouldn't happen for x's parent
                        par.borrow_mut().left = Some(sentinel.clone());
                    }
                } else {
                    self.root = Some(sentinel.clone());
                }
                self.delete_fixup(Some(sentinel.clone()));
                // Remove sentinel from tree
                let sp = sentinel.borrow().parent.clone();
                if let Some(ref par) = sp {
                    if is_same(&par.borrow().left, &Some(sentinel.clone())) {
                        par.borrow_mut().left = None;
                    } else if is_same(&par.borrow().right, &Some(sentinel.clone())) {
                        par.borrow_mut().right = None;
                    }
                }
                if is_same(&self.root, &Some(sentinel.clone())) {
                    self.root = None;
                }
            } else {
                self.delete_fixup(x);
            }
        }
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(&self, curr: Option<NodeRef>, arr: &mut Vec<Key>, n: usize, count: &mut usize) {
        if let Some(node) = curr {
            self.subtree_to_array(node.borrow().left.clone(), arr, n, count);
            if *count < n {
                arr.push(node.borrow().key);
                *count += 1;
            } else {
                return;
            }
            self.subtree_to_array(node.borrow().right.clone(), arr, n, count);
        }
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr = Vec::new();
        let mut count = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}
