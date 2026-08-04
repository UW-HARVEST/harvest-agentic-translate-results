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
    fn new_node(key: Key, color: Color) -> NodeRef {
        Rc::new(RefCell::new(Node {
            key,
            color,
            left: None,
            right: None,
            parent: None,
        }))
    }

    fn color_of(node: Option<NodeRef>) -> Color {
        node.map(|n| n.borrow().color.clone()).unwrap_or(Color::Black)
    }

    fn parent_of(node: &Option<NodeRef>) -> Option<NodeRef> {
        node.as_ref().and_then(|n| n.borrow().parent.clone())
    }

    fn left_of(node: &Option<NodeRef>) -> Option<NodeRef> {
        node.as_ref().and_then(|n| n.borrow().left.clone())
    }

    fn right_of(node: &Option<NodeRef>) -> Option<NodeRef> {
        node.as_ref().and_then(|n| n.borrow().right.clone())
    }

    fn set_color(node: Option<NodeRef>, color: Color) {
        if let Some(node) = node {
            node.borrow_mut().color = color;
        }
    }

    fn same_node(a: &Option<NodeRef>, b: &Option<NodeRef>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }

    fn is_left_child(parent: &NodeRef, child: &Option<NodeRef>) -> bool {
        match (parent.borrow().left.clone(), child) {
            (Some(left), Some(child)) => Rc::ptr_eq(&left, child),
            (None, None) => true,
            _ => false,
        }
    }

    fn minimum_from(node: NodeRef) -> NodeRef {
        let mut curr = node;
        loop {
            let left = curr.borrow().left.clone();
            if let Some(next) = left {
                curr = next;
            } else {
                return curr;
            }
        }
    }

    fn delete_fixup_with_parent(&mut self, mut x: Option<NodeRef>, mut parent: Option<NodeRef>) {
        while !Self::same_node(&x, &self.root) && Self::color_of(x.clone()) == Color::Black {
            let Some(parent_node) = parent.clone() else {
                break;
            };

            if Self::is_left_child(&parent_node, &x) {
                let mut w = parent_node.borrow().right.clone();

                if Self::color_of(w.clone()) == Color::Red {
                    Self::set_color(w.clone(), Color::Black);
                    Self::set_color(Some(parent_node.clone()), Color::Red);
                    self.left_rotate(parent_node.clone());
                    w = parent_node.borrow().right.clone();
                }

                let w_left_black = Self::color_of(Self::left_of(&w)) == Color::Black;
                let w_right_black = Self::color_of(Self::right_of(&w)) == Color::Black;

                if w_left_black && w_right_black {
                    Self::set_color(w, Color::Red);
                    x = Some(parent_node.clone());
                    parent = Self::parent_of(&x);
                } else {
                    if Self::color_of(Self::right_of(&w)) == Color::Black {
                        Self::set_color(Self::left_of(&w), Color::Black);
                        Self::set_color(w.clone(), Color::Red);
                        if let Some(w_node) = w.clone() {
                            self.right_rotate(w_node);
                        }
                        w = parent_node.borrow().right.clone();
                    }

                    Self::set_color(w.clone(), Self::color_of(Some(parent_node.clone())));
                    Self::set_color(Some(parent_node.clone()), Color::Black);
                    Self::set_color(Self::right_of(&w), Color::Black);
                    self.left_rotate(parent_node);
                    x = self.root.clone();
                    parent = None;
                }
            } else {
                let mut w = parent_node.borrow().left.clone();

                if Self::color_of(w.clone()) == Color::Red {
                    Self::set_color(w.clone(), Color::Black);
                    Self::set_color(Some(parent_node.clone()), Color::Red);
                    self.right_rotate(parent_node.clone());
                    w = parent_node.borrow().left.clone();
                }

                let w_right_black = Self::color_of(Self::right_of(&w)) == Color::Black;
                let w_left_black = Self::color_of(Self::left_of(&w)) == Color::Black;

                if w_right_black && w_left_black {
                    Self::set_color(w, Color::Red);
                    x = Some(parent_node.clone());
                    parent = Self::parent_of(&x);
                } else {
                    if Self::color_of(Self::left_of(&w)) == Color::Black {
                        Self::set_color(Self::right_of(&w), Color::Black);
                        Self::set_color(w.clone(), Color::Red);
                        if let Some(w_node) = w.clone() {
                            self.left_rotate(w_node);
                        }
                        w = parent_node.borrow().left.clone();
                    }

                    Self::set_color(w.clone(), Self::color_of(Some(parent_node.clone())));
                    Self::set_color(Some(parent_node.clone()), Color::Black);
                    Self::set_color(Self::left_of(&w), Color::Black);
                    self.right_rotate(parent_node);
                    x = self.root.clone();
                    parent = None;
                }
            }
        }

        Self::set_color(x, Color::Black);
    }

    pub fn new() -> Self {
        Self { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        let y = {
            let mut x_borrow = x.borrow_mut();
            x_borrow.left.take()
        };
        let Some(y) = y else {
            return;
        };

        let y_right = {
            let mut y_borrow = y.borrow_mut();
            y_borrow.right.take()
        };
        {
            x.borrow_mut().left = y_right.clone();
        }
        if let Some(node) = y_right {
            node.borrow_mut().parent = Some(x.clone());
        }

        let x_parent = {
            let x_borrow = x.borrow();
            x_borrow.parent.clone()
        };
        y.borrow_mut().parent = x_parent.clone();

        if let Some(parent) = x_parent {
            if Self::is_left_child(&parent, &Some(x.clone())) {
                parent.borrow_mut().left = Some(y.clone());
            } else {
                parent.borrow_mut().right = Some(y.clone());
            }
        } else {
            self.root = Some(y.clone());
        }

        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        let y = {
            let mut x_borrow = x.borrow_mut();
            x_borrow.right.take()
        };
        let Some(y) = y else {
            return;
        };

        let y_left = {
            let mut y_borrow = y.borrow_mut();
            y_borrow.left.take()
        };
        {
            x.borrow_mut().right = y_left.clone();
        }
        if let Some(node) = y_left {
            node.borrow_mut().parent = Some(x.clone());
        }

        let x_parent = {
            let x_borrow = x.borrow();
            x_borrow.parent.clone()
        };
        y.borrow_mut().parent = x_parent.clone();

        if let Some(parent) = x_parent {
            if Self::is_left_child(&parent, &Some(x.clone())) {
                parent.borrow_mut().left = Some(y.clone());
            } else {
                parent.borrow_mut().right = Some(y.clone());
            }
        } else {
            self.root = Some(y.clone());
        }

        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(node) = node {
            let (left, right) = {
                let mut node_mut = node.borrow_mut();
                let left = node_mut.left.take();
                let right = node_mut.right.take();
                node_mut.parent = None;
                (left, right)
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

        while Self::color_of(Self::parent_of(&Some(z.clone()))) == Color::Red {
            let parent = {
                let z_borrow = z.borrow();
                z_borrow.parent.clone()
            };
            let Some(parent) = parent else {
                break;
            };
            let grandparent = {
                let parent_borrow = parent.borrow();
                parent_borrow.parent.clone()
            };
            let Some(grandparent) = grandparent else {
                break;
            };

            let parent_is_left = Self::is_left_child(&grandparent, &Some(parent.clone()));

            if parent_is_left {
                let uncle = {
                    let grandparent_borrow = grandparent.borrow();
                    grandparent_borrow.right.clone()
                };
                if Self::color_of(uncle.clone()) == Color::Red {
                    Self::set_color(Some(parent.clone()), Color::Black);
                    Self::set_color(uncle, Color::Black);
                    Self::set_color(Some(grandparent.clone()), Color::Red);
                    z = grandparent;
                } else {
                    let z_is_left = Self::is_left_child(&parent, &Some(z.clone()));
                    if !z_is_left {
                        z = parent.clone();
                        self.left_rotate(z.clone());
                    }

                    let parent = {
                        let z_borrow = z.borrow();
                        z_borrow.parent.clone()
                    };
                    if let Some(parent) = parent {
                        Self::set_color(Some(parent.clone()), Color::Black);
                        let grandparent = {
                            let parent_borrow = parent.borrow();
                            parent_borrow.parent.clone()
                        };
                        if let Some(grandparent) = grandparent {
                            Self::set_color(Some(grandparent.clone()), Color::Red);
                            self.right_rotate(grandparent);
                        }
                    }
                }
            } else {
                let uncle = {
                    let grandparent_borrow = grandparent.borrow();
                    grandparent_borrow.left.clone()
                };
                if Self::color_of(uncle.clone()) == Color::Red {
                    Self::set_color(Some(parent.clone()), Color::Black);
                    Self::set_color(uncle, Color::Black);
                    Self::set_color(Some(grandparent.clone()), Color::Red);
                    z = grandparent;
                } else {
                    let z_is_left = Self::is_left_child(&parent, &Some(z.clone()));
                    if z_is_left {
                        z = parent.clone();
                        self.right_rotate(z.clone());
                    }

                    let parent = {
                        let z_borrow = z.borrow();
                        z_borrow.parent.clone()
                    };
                    if let Some(parent) = parent {
                        Self::set_color(Some(parent.clone()), Color::Black);
                        let grandparent = {
                            let parent_borrow = parent.borrow();
                            parent_borrow.parent.clone()
                        };
                        if let Some(grandparent) = grandparent {
                            Self::set_color(Some(grandparent.clone()), Color::Red);
                            self.left_rotate(grandparent);
                        }
                    }
                }
            }
        }

        Self::set_color(self.root.clone(), Color::Black);
    }

    /// Inserts a new key and returns the inserted node.
    pub fn rbtree_insert(&mut self, key: Key) -> Option<NodeRef> {
        let mut parent = None;
        let mut curr = self.root.clone();

        while let Some(node) = curr {
            parent = Some(node.clone());
            curr = if key < node.borrow().key {
                node.borrow().left.clone()
            } else {
                node.borrow().right.clone()
            };
        }

        let new_node = Self::new_node(key, Color::Red);
        new_node.borrow_mut().parent = parent.clone();

        if let Some(parent) = parent {
            if key < parent.borrow().key {
                parent.borrow_mut().left = Some(new_node.clone());
            } else {
                parent.borrow_mut().right = Some(new_node.clone());
            }
        } else {
            self.root = Some(new_node.clone());
        }

        self.rbtree_insert_fixup(new_node.clone());
        Some(new_node)
    }

    /// Finds a node by key. Returns `None` if not found.
    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut curr = self.root.clone();

        while let Some(node) = curr {
            let node_key = node.borrow().key;
            if node_key == key {
                return Some(node);
            }
            curr = if node_key < key {
                node.borrow().right.clone()
            } else {
                node.borrow().left.clone()
            };
        }

        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone();

        while let Some(node) = curr.clone() {
            if node.borrow().left.is_none() {
                return curr;
            }
            curr = node.borrow().left.clone();
        }

        None
    }

    /// Returns the maximum node, or `None` if the tree is empty.
    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut curr = self.root.clone();

        while let Some(node) = curr.clone() {
            if node.borrow().right.is_none() {
                return curr;
            }
            curr = node.borrow().right.clone();
        }

        None
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let parent = u.borrow().parent.clone();

        if let Some(parent) = parent.clone() {
            if Self::is_left_child(&parent, &Some(u.clone())) {
                parent.borrow_mut().left = v.clone();
            } else {
                parent.borrow_mut().right = v.clone();
            }
        } else {
            self.root = v.clone();
        }

        if let Some(v) = v {
            v.borrow_mut().parent = parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let parent = Self::parent_of(&x);
        self.delete_fixup_with_parent(x, parent);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let mut y = p.clone();
        let mut y_original_color = y.borrow().color.clone();
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p.borrow().left.is_none() {
            x = p.borrow().right.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), x.clone());
        } else if p.borrow().right.is_none() {
            x = p.borrow().left.clone();
            x_parent = p.borrow().parent.clone();
            self.transplant(p.clone(), x.clone());
        } else {
            let right = p.borrow().right.clone();
            if let Some(right) = right {
                y = Self::minimum_from(right);
            }
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            if Self::parent_of(&Some(y.clone()))
                .as_ref()
                .is_some_and(|parent| Rc::ptr_eq(parent, &p))
            {
                x_parent = Some(y.clone());
                if let Some(x_node) = x.clone() {
                    x_node.borrow_mut().parent = Some(y.clone());
                }
            } else {
                x_parent = y.borrow().parent.clone();
                self.transplant(y.clone(), x.clone());

                let p_right = p.borrow().right.clone();
                {
                    y.borrow_mut().right = p_right.clone();
                }
                if let Some(right) = p_right {
                    right.borrow_mut().parent = Some(y.clone());
                }
            }

            self.transplant(p.clone(), Some(y.clone()));

            let p_left = p.borrow().left.clone();
            {
                let mut y_mut = y.borrow_mut();
                y_mut.left = p_left.clone();
                y_mut.color = p.borrow().color.clone();
            }
            if let Some(left) = p_left {
                left.borrow_mut().parent = Some(y.clone());
            }
        }

        if y_original_color == Color::Black {
            self.delete_fixup_with_parent(x, x_parent);
        }

        let mut p_mut = p.borrow_mut();
        p_mut.parent = None;
        p_mut.left = None;
        p_mut.right = None;
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(&self, curr: Option<NodeRef>, arr: &mut Vec<Key>, n: usize, count: &mut usize) {
        let _ = &self.root;
        if *count >= n {
            return;
        }

        let Some(node) = curr else {
            return;
        };

        let left = node.borrow().left.clone();
        self.subtree_to_array(left, arr, n, count);

        if *count >= n {
            return;
        }

        arr.push(node.borrow().key);
        *count += 1;

        if *count >= n {
            return;
        }

        let right = node.borrow().right.clone();
        self.subtree_to_array(right, arr, n, count);
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr = Vec::with_capacity(n);
        let mut count = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}
