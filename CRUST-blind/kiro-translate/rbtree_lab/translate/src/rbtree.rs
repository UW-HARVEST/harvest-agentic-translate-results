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

// Helper to check if two NodeRefs point to the same node
fn same(a: &NodeRef, b: &NodeRef) -> bool {
    Rc::ptr_eq(a, b)
}

fn is_red(node: &Option<NodeRef>) -> bool {
    match node {
        Some(n) => n.borrow().color == Color::Red,
        None => false,
    }
}

fn is_black(node: &Option<NodeRef>) -> bool {
    !is_red(node)
}

fn set_color(node: &Option<NodeRef>, color: Color) {
    if let Some(n) = node {
        n.borrow_mut().color = color;
    }
}

fn parent_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent.clone()
}

fn left_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().left.clone()
}

fn right_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().right.clone()
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        let y = left_of(&x).expect("right_rotate: x.left must exist");

        // x.left = y.right
        let yr = right_of(&y);
        x.borrow_mut().left = yr.clone();
        if let Some(ref yr_node) = yr {
            yr_node.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let xp = parent_of(&x);
        y.borrow_mut().parent = xp.clone();

        match xp {
            None => self.root = Some(y.clone()),
            Some(ref p) => {
                let is_left = p.borrow().left.as_ref().map_or(false, |l| same(l, &x));
                if is_left {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.right = x, x.parent = y
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        let y = right_of(&x).expect("left_rotate: x.right must exist");

        // x.right = y.left
        let yl = left_of(&y);
        x.borrow_mut().right = yl.clone();
        if let Some(ref yl_node) = yl {
            yl_node.borrow_mut().parent = Some(x.clone());
        }

        // y.parent = x.parent
        let xp = parent_of(&x);
        y.borrow_mut().parent = xp.clone();

        match xp {
            None => self.root = Some(y.clone()),
            Some(ref p) => {
                let is_left = p.borrow().left.as_ref().map_or(false, |l| same(l, &x));
                if is_left {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }

        // y.left = x, x.parent = y
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
        while is_red(&parent_of(&z)) {
            let zp = parent_of(&z).unwrap();
            let zpp = parent_of(&zp).unwrap();

            if zp.borrow().parent.as_ref().map_or(false, |pp| {
                pp.borrow().left.as_ref().map_or(false, |l| same(l, &zp))
            }) {
                // z's parent is left child of grandparent
                let y = right_of(&zpp); // uncle

                if is_red(&y) {
                    // Case 1
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&y, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    // Case 2
                    if zp.borrow().right.as_ref().map_or(false, |r| same(r, &z)) {
                        z = zp.clone();
                        self.left_rotate(z.clone());
                    }
                    // Case 3
                    let zp = parent_of(&z).unwrap();
                    let zpp = parent_of(&zp).unwrap();
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.right_rotate(zpp);
                }
            } else {
                // z's parent is right child of grandparent
                let y = left_of(&zpp); // uncle

                if is_red(&y) {
                    // Case 4
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&y, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    // Case 5
                    if zp.borrow().left.as_ref().map_or(false, |l| same(l, &z)) {
                        z = zp.clone();
                        self.right_rotate(z.clone());
                    }
                    // Case 6
                    let zp = parent_of(&z).unwrap();
                    let zpp = parent_of(&zp).unwrap();
                    set_color(&Some(zp.clone()), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.left_rotate(zpp);
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
            Some(ref parent) => {
                if key < parent.borrow().key {
                    parent.borrow_mut().left = Some(z.clone());
                } else {
                    parent.borrow_mut().right = Some(z.clone());
                }
            }
        }

        self.rbtree_insert_fixup(z.clone());
        Some(z)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut current = self.root.clone();
        while let Some(node) = current {
            let k = node.borrow().key;
            if k == key {
                return Some(node);
            } else if k < key {
                current = node.borrow().right.clone();
            } else {
                current = node.borrow().left.clone();
            }
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
        let up = parent_of(&u);
        match up {
            None => self.root = v.clone(),
            Some(ref p) => {
                let is_left = p.borrow().left.as_ref().map_or(false, |l| same(l, &u));
                if is_left {
                    p.borrow_mut().left = v.clone();
                } else {
                    p.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(ref v_node) = v {
            v_node.borrow_mut().parent = up;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    /// `x_parent` is needed because x might be None (NIL) and we need its parent.
    fn delete_fixup_inner(&mut self, mut x: Option<NodeRef>, mut x_parent: Option<NodeRef>) {
        while !self.root.as_ref().map_or(true, |r| x.as_ref().map_or(false, |xn| same(xn, r)))
            && is_black(&x)
        {
            // x is not root and x is black
            let parent = x_parent.clone().unwrap();

            let x_is_left = parent.borrow().left.as_ref().map_or(true, |l| {
                x.as_ref().map_or(true, |xn| same(l, xn))
            }) && !(parent.borrow().left.is_none() && x.is_some());

            // Determine if x is left child
            let is_left = if x.is_some() {
                parent.borrow().left.as_ref().map_or(false, |l| same(l, x.as_ref().unwrap()))
            } else {
                // x is None (NIL). Check which child of parent is None.
                // If left is None and right is not, x is left child.
                // If both are None, we need context — but in RB delete fixup,
                // we track this from the caller.
                x_is_left
            };

            if is_left {
                let mut w = parent.borrow().right.clone().unwrap();

                // Case 1: w is red
                if is_red(&Some(w.clone())) {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.left_rotate(parent.clone());
                    w = parent.borrow().right.clone().unwrap();
                }

                // Case 2: both children of w are black
                if is_black(&left_of(&w)) && is_black(&right_of(&w)) {
                    w.borrow_mut().color = Color::Red;
                    x = Some(parent.clone());
                    x_parent = parent_of(&parent);
                } else {
                    // Case 3: w.right is black
                    if is_black(&right_of(&w)) {
                        set_color(&left_of(&w), Color::Black);
                        w.borrow_mut().color = Color::Red;
                        self.right_rotate(w.clone());
                        w = parent.borrow().right.clone().unwrap();
                    }
                    // Case 4
                    w.borrow_mut().color = parent.borrow().color.clone();
                    parent.borrow_mut().color = Color::Black;
                    set_color(&right_of(&w), Color::Black);
                    self.left_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            } else {
                let mut w = parent.borrow().left.clone().unwrap();

                // Case 5: w is red
                if is_red(&Some(w.clone())) {
                    w.borrow_mut().color = Color::Black;
                    parent.borrow_mut().color = Color::Red;
                    self.right_rotate(parent.clone());
                    w = parent.borrow().left.clone().unwrap();
                }

                // Case 6: both children of w are black
                if is_black(&right_of(&w)) && is_black(&left_of(&w)) {
                    w.borrow_mut().color = Color::Red;
                    x = Some(parent.clone());
                    x_parent = parent_of(&parent);
                } else {
                    // Case 7: w.left is black
                    if is_black(&left_of(&w)) {
                        set_color(&right_of(&w), Color::Black);
                        w.borrow_mut().color = Color::Red;
                        self.left_rotate(w.clone());
                        w = parent.borrow().left.clone().unwrap();
                    }
                    // Case 8
                    w.borrow_mut().color = parent.borrow().color.clone();
                    parent.borrow_mut().color = Color::Black;
                    set_color(&left_of(&w), Color::Black);
                    self.right_rotate(parent.clone());
                    x = self.root.clone();
                    x_parent = None;
                }
            }
        }

        set_color(&x, Color::Black);
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let x_parent = x.as_ref().and_then(|n| parent_of(n));
        self.delete_fixup_inner(x, x_parent);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let y_original_color;
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        let p_left = left_of(&p);
        let p_right = right_of(&p);

        if p_left.is_none() {
            y_original_color = p.borrow().color.clone();
            x = p_right.clone();
            x_parent = parent_of(&p);
            self.transplant(p.clone(), p_right);
        } else if p_right.is_none() {
            y_original_color = p.borrow().color.clone();
            x = p_left.clone();
            x_parent = parent_of(&p);
            self.transplant(p.clone(), p_left);
        } else {
            // Find successor (min of right subtree)
            let mut y = p_right.clone().unwrap();
            loop {
                let yl = y.borrow().left.clone();
                match yl {
                    Some(l) => y = l,
                    None => break,
                }
            }
            y_original_color = y.borrow().color.clone();
            x = right_of(&y);

            if parent_of(&y).as_ref().map_or(false, |yp| same(yp, &p)) {
                // y is direct child of p
                x_parent = Some(y.clone());
                // x.parent = y is already set (or x is None)
                if let Some(ref xn) = x {
                    xn.borrow_mut().parent = Some(y.clone());
                }
            } else {
                x_parent = parent_of(&y);
                self.transplant(y.clone(), x.clone());
                y.borrow_mut().right = p.borrow().right.clone();
                if let Some(ref yr) = y.borrow().right {
                    yr.borrow_mut().parent = Some(y.clone());
                }
            }

            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = p.borrow().left.clone();
            if let Some(ref yl) = y.borrow().left {
                yl.borrow_mut().parent = Some(y.clone());
            }
            y.borrow_mut().color = p.borrow().color.clone();
        }

        if y_original_color == Color::Black {
            self.delete_fixup_inner(x, x_parent);
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
        if self.root.is_none() {
            return arr;
        }
        let mut count = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}
