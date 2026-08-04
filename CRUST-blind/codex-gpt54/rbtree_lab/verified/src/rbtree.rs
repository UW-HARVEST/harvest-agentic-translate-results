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

fn option_ptr_eq(a: &Option<NodeRef>, b: &Option<NodeRef>) -> bool {
    match (a, b) {
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn color_of(node: Option<&NodeRef>) -> Color {
    match node {
        Some(node) => node.borrow().color.clone(),
        None => Color::Black,
    }
}

fn set_color(node: Option<&NodeRef>, color: Color) {
    if let Some(node) = node {
        node.borrow_mut().color = color;
    }
}

fn parent_of(node: Option<&NodeRef>) -> Option<NodeRef> {
    node.and_then(|node| node.borrow().parent.clone())
}

fn left_of(node: Option<&NodeRef>) -> Option<NodeRef> {
    node.and_then(|node| node.borrow().left.clone())
}

fn right_of(node: Option<&NodeRef>) -> Option<NodeRef> {
    node.and_then(|node| node.borrow().right.clone())
}

fn minimum_node(mut node: NodeRef) -> NodeRef {
    loop {
        let next = node.borrow().left.clone();
        match next {
            Some(left) => node = left,
            None => return node,
        }
    }
}

fn is_left_child(node: &NodeRef) -> bool {
    let parent = node.borrow().parent.clone();
    match parent {
        Some(parent) => parent
            .borrow()
            .left
            .as_ref()
            .is_some_and(|left| Rc::ptr_eq(left, node)),
        None => false,
    }
}

impl RBTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Performs a right rotation around non-NIL node `x`.
    pub fn right_rotate(&mut self, x: NodeRef) {
        let Some(y) = x.borrow().left.clone() else {
            return;
        };

        let y_right = y.borrow().right.clone();
        {
            x.borrow_mut().left = y_right.clone();
        }
        if let Some(node) = y_right {
            node.borrow_mut().parent = Some(x.clone());
        }

        let x_parent = x.borrow().parent.clone();
        {
            y.borrow_mut().parent = x_parent.clone();
        }

        match x_parent {
            None => self.root = Some(y.clone()),
            Some(parent) => {
                let is_left = parent
                    .borrow()
                    .left
                    .as_ref()
                    .is_some_and(|left| Rc::ptr_eq(left, &x));
                if is_left {
                    parent.borrow_mut().left = Some(y.clone());
                } else {
                    parent.borrow_mut().right = Some(y.clone());
                }
            }
        }

        {
            y.borrow_mut().right = Some(x.clone());
        }
        x.borrow_mut().parent = Some(y);
    }

    /// Performs a left rotation around non-NIL node `x`.
    pub fn left_rotate(&mut self, x: NodeRef) {
        let Some(y) = x.borrow().right.clone() else {
            return;
        };

        let y_left = y.borrow().left.clone();
        {
            x.borrow_mut().right = y_left.clone();
        }
        if let Some(node) = y_left {
            node.borrow_mut().parent = Some(x.clone());
        }

        let x_parent = x.borrow().parent.clone();
        {
            y.borrow_mut().parent = x_parent.clone();
        }

        match x_parent {
            None => self.root = Some(y.clone()),
            Some(parent) => {
                let is_left = parent
                    .borrow()
                    .left
                    .as_ref()
                    .is_some_and(|left| Rc::ptr_eq(left, &x));
                if is_left {
                    parent.borrow_mut().left = Some(y.clone());
                } else {
                    parent.borrow_mut().right = Some(y.clone());
                }
            }
        }

        {
            y.borrow_mut().left = Some(x.clone());
        }
        x.borrow_mut().parent = Some(y);
    }

    /// Recursively drops a subtree. `None` is treated as NIL.
    pub fn free_node(node: Option<NodeRef>) {
        if let Some(node) = node {
            let (left, right) = {
                let mut node = node.borrow_mut();
                let left = node.left.take();
                let right = node.right.take();
                node.parent = None;
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

        while color_of(parent_of(Some(&z)).as_ref()) == Color::Red {
            let Some(parent) = parent_of(Some(&z)) else {
                break;
            };
            let Some(grandparent) = parent_of(Some(&parent)) else {
                break;
            };

            if grandparent
                .borrow()
                .left
                .as_ref()
                .is_some_and(|left| Rc::ptr_eq(left, &parent))
            {
                let uncle = grandparent.borrow().right.clone();
                if color_of(uncle.as_ref()) == Color::Red {
                    set_color(Some(&parent), Color::Black);
                    set_color(uncle.as_ref(), Color::Black);
                    set_color(Some(&grandparent), Color::Red);
                    z = grandparent;
                } else {
                    if parent
                        .borrow()
                        .right
                        .as_ref()
                        .is_some_and(|right| Rc::ptr_eq(right, &z))
                    {
                        z = parent.clone();
                        self.left_rotate(z.clone());
                    }

                    let Some(parent) = parent_of(Some(&z)) else {
                        break;
                    };
                    let Some(grandparent) = parent_of(Some(&parent)) else {
                        break;
                    };
                    set_color(Some(&parent), Color::Black);
                    set_color(Some(&grandparent), Color::Red);
                    self.right_rotate(grandparent);
                }
            } else {
                let uncle = grandparent.borrow().left.clone();
                if color_of(uncle.as_ref()) == Color::Red {
                    set_color(Some(&parent), Color::Black);
                    set_color(uncle.as_ref(), Color::Black);
                    set_color(Some(&grandparent), Color::Red);
                    z = grandparent;
                } else {
                    if parent
                        .borrow()
                        .left
                        .as_ref()
                        .is_some_and(|left| Rc::ptr_eq(left, &z))
                    {
                        z = parent.clone();
                        self.right_rotate(z.clone());
                    }

                    let Some(parent) = parent_of(Some(&z)) else {
                        break;
                    };
                    let Some(grandparent) = parent_of(Some(&parent)) else {
                        break;
                    };
                    set_color(Some(&parent), Color::Black);
                    set_color(Some(&grandparent), Color::Red);
                    self.left_rotate(grandparent);
                }
            }
        }

        set_color(self.root.as_ref(), Color::Black);
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

        let mut parent = None;
        let mut current = self.root.clone();

        while let Some(node) = current {
            parent = Some(node.clone());
            current = if key < node.borrow().key {
                node.borrow().left.clone()
            } else {
                node.borrow().right.clone()
            };
        }

        z.borrow_mut().parent = parent.clone();
        match parent {
            None => self.root = Some(z.clone()),
            Some(parent) => {
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
            let node_key = node.borrow().key;
            if node_key == key {
                return Some(node);
            }
            current = if node_key < key {
                node.borrow().right.clone()
            } else {
                node.borrow().left.clone()
            };
        }

        None
    }

    /// Returns the minimum node, or `None` if the tree is empty.
    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut current = self.root.clone()?;
        loop {
            let next = current.borrow().left.clone();
            match next {
                Some(left) => current = left,
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
                Some(right) => current = right,
                None => return Some(current),
            }
        }
    }

    /// Replaces subtree rooted at `u` with subtree rooted at `v` (`None` == NIL).
    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let u_parent = u.borrow().parent.clone();

        match u_parent.clone() {
            None => self.root = v.clone(),
            Some(parent) => {
                if parent
                    .borrow()
                    .left
                    .as_ref()
                    .is_some_and(|left| Rc::ptr_eq(left, &u))
                {
                    parent.borrow_mut().left = v.clone();
                } else {
                    parent.borrow_mut().right = v.clone();
                }
            }
        }

        if let Some(node) = v {
            node.borrow_mut().parent = u_parent;
        }
    }

    /// Fixes up after deletion starting from node `x` (`None` == NIL).
    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        let parent = parent_of(x.as_ref());
        self.delete_fixup_inner(x, parent, None);
    }

    /// Erases node `p` (must be a valid non-NIL node in the tree).
    pub fn erase(&mut self, p: NodeRef) {
        let mut y = p.clone();
        let mut y_original_color = y.borrow().color.clone();
        let x;
        let x_parent;
        let x_is_left;

        if p.borrow().left.is_none() {
            x = p.borrow().right.clone();
            x_parent = p.borrow().parent.clone();
            x_is_left = if x.is_none() {
                Some(is_left_child(&p))
            } else {
                None
            };
            self.transplant(p.clone(), x.clone());
        } else if p.borrow().right.is_none() {
            x = p.borrow().left.clone();
            x_parent = p.borrow().parent.clone();
            x_is_left = if x.is_none() {
                Some(is_left_child(&p))
            } else {
                None
            };
            self.transplant(p.clone(), x.clone());
        } else {
            let Some(right) = p.borrow().right.clone() else {
                return;
            };
            y = minimum_node(right);
            y_original_color = y.borrow().color.clone();
            x = y.borrow().right.clone();

            if parent_of(Some(&y))
                .as_ref()
                .is_some_and(|parent| Rc::ptr_eq(parent, &p))
            {
                x_parent = Some(y.clone());
                x_is_left = if x.is_none() { Some(false) } else { None };
                if let Some(node) = x.as_ref() {
                    node.borrow_mut().parent = Some(y.clone());
                }
            } else {
                x_parent = parent_of(Some(&y));
                x_is_left = if x.is_none() { Some(true) } else { None };
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
                left.borrow_mut().parent = Some(y);
            }
        }

        if y_original_color == Color::Black {
            self.delete_fixup_inner(x, x_parent, x_is_left);
        }

        let mut node = p.borrow_mut();
        node.left = None;
        node.right = None;
        node.parent = None;
    }

    /// In-order traversal of `curr` into `arr` until `n` elements (`None` == NIL).
    pub fn subtree_to_array(&self, curr: Option<NodeRef>, arr: &mut Vec<Key>, n: usize, count: &mut usize) {
        let _ = &self.root;
        if *count >= n {
            return;
        }
        let Some(curr) = curr else {
            return;
        };

        let left = curr.borrow().left.clone();
        self.subtree_to_array(left, arr, n, count);
        if *count >= n {
            return;
        }

        arr.push(curr.borrow().key);
        *count += 1;

        if *count >= n {
            return;
        }

        let right = curr.borrow().right.clone();
        self.subtree_to_array(right, arr, n, count);
    }

    /// Returns up to `n` keys from the tree in-order.
    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut result = Vec::with_capacity(n);
        let mut count = 0;
        self.subtree_to_array(self.root.clone(), &mut result, n, &mut count);
        result
    }

    fn delete_fixup_inner(
        &mut self,
        mut x: Option<NodeRef>,
        mut parent: Option<NodeRef>,
        mut nil_is_left: Option<bool>,
    ) {
        while !option_ptr_eq(&x, &self.root) && color_of(x.as_ref()) == Color::Black {
            let Some(parent_node) = parent.clone() else {
                break;
            };

            let x_is_left = match x.as_ref() {
                Some(node) => parent_node
                    .borrow()
                    .left
                    .as_ref()
                    .is_some_and(|left| Rc::ptr_eq(left, node)),
                None => nil_is_left.unwrap_or(parent_node.borrow().left.is_none()),
            };

            if x_is_left {
                let mut w = parent_node.borrow().right.clone();

                if color_of(w.as_ref()) == Color::Red {
                    set_color(w.as_ref(), Color::Black);
                    set_color(Some(&parent_node), Color::Red);
                    self.left_rotate(parent_node.clone());
                    w = parent_node.borrow().right.clone();
                }

                let w_left_black = color_of(left_of(w.as_ref()).as_ref()) == Color::Black;
                let w_right_black = color_of(right_of(w.as_ref()).as_ref()) == Color::Black;

                if w_left_black && w_right_black {
                    set_color(w.as_ref(), Color::Red);
                    x = Some(parent_node.clone());
                    parent = parent_of(x.as_ref());
                    nil_is_left = None;
                } else {
                    if color_of(right_of(w.as_ref()).as_ref()) == Color::Black {
                        if let Some(sibling) = w.as_ref() {
                            set_color(left_of(Some(sibling)).as_ref(), Color::Black);
                            set_color(Some(sibling), Color::Red);
                            self.right_rotate(sibling.clone());
                        }
                        w = parent_node.borrow().right.clone();
                    }

                    let parent_color = parent_node.borrow().color.clone();
                    set_color(w.as_ref(), parent_color);
                    set_color(Some(&parent_node), Color::Black);
                    set_color(right_of(w.as_ref()).as_ref(), Color::Black);
                    self.left_rotate(parent_node);
                    x = self.root.clone();
                    parent = None;
                    nil_is_left = None;
                }
            } else {
                let mut w = parent_node.borrow().left.clone();

                if color_of(w.as_ref()) == Color::Red {
                    set_color(w.as_ref(), Color::Black);
                    set_color(Some(&parent_node), Color::Red);
                    self.right_rotate(parent_node.clone());
                    w = parent_node.borrow().left.clone();
                }

                let w_right_black = color_of(right_of(w.as_ref()).as_ref()) == Color::Black;
                let w_left_black = color_of(left_of(w.as_ref()).as_ref()) == Color::Black;

                if w_right_black && w_left_black {
                    set_color(w.as_ref(), Color::Red);
                    x = Some(parent_node.clone());
                    parent = parent_of(x.as_ref());
                    nil_is_left = None;
                } else {
                    if color_of(left_of(w.as_ref()).as_ref()) == Color::Black {
                        if let Some(sibling) = w.as_ref() {
                            set_color(right_of(Some(sibling)).as_ref(), Color::Black);
                            set_color(Some(sibling), Color::Red);
                            self.left_rotate(sibling.clone());
                        }
                        w = parent_node.borrow().left.clone();
                    }

                    let parent_color = parent_node.borrow().color.clone();
                    set_color(w.as_ref(), parent_color);
                    set_color(Some(&parent_node), Color::Black);
                    set_color(left_of(w.as_ref()).as_ref(), Color::Black);
                    self.right_rotate(parent_node);
                    x = self.root.clone();
                    parent = None;
                    nil_is_left = None;
                }
            }
        }

        set_color(x.as_ref(), Color::Black);
    }
}
