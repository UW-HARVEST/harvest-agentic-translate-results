use std::cell::RefCell;
use std::rc::{Rc, Weak};

type NodeRef = Rc<RefCell<BOSNode>>;

#[derive(Debug)]
pub struct BOSTree {
    /// The root node of the tree.
    pub root_node: Option<Rc<RefCell<BOSNode>>>,
    /// The key comparison function.
    pub cmp_function: BOSTreeCmpFunction,
    /// The optional free function for node data.
    pub free_function: Option<BOSTreeFreeFunction>,
}

/// The node structure.
#[derive(Debug)]
pub struct BOSNode {
    /// Number of nodes in the left subtree.
    pub left_child_count: u32,
    /// Number of nodes in the right subtree.
    pub right_child_count: u32,
    /// Cached depth of the node.
    pub depth: u32,
    /// Left child node.
    pub left_child_node: Option<Rc<RefCell<BOSNode>>>,
    /// Right child node.
    pub right_child_node: Option<Rc<RefCell<BOSNode>>>,
    /// Parent node (using a weak reference to avoid cycles).
    pub parent_node: Option<Weak<RefCell<BOSNode>>>,
    /// The key for this node.
    pub key: String,
    /// The associated data.
    pub data: Option<String>,
    /// Internal weak reference counter.
    pub weak_ref_count: u8,
    /// Validity flag for the weak reference.
    pub weak_ref_node_valid: u8,
}

/// Type alias for a key comparison function.
/// Should return a positive value if `b` is larger than `a`,
/// a negative value if `a` is larger, and zero if equal.
pub type BOSTreeCmpFunction = fn(&str, &str) -> i32;

/// Type alias for a free function which will be called on nodes
/// that are removed.
pub type BOSTreeFreeFunction = fn(&Rc<RefCell<BOSNode>>);

fn parent_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent_node.as_ref().and_then(Weak::upgrade)
}

fn left_depth(node: &NodeRef) -> u32 {
    node.borrow()
        .left_child_node
        .as_ref()
        .map_or(0, |child| child.borrow().depth + 1)
}

fn right_depth(node: &NodeRef) -> u32 {
    node.borrow()
        .right_child_node
        .as_ref()
        .map_or(0, |child| child.borrow().depth + 1)
}

fn balance(node: &NodeRef) -> i32 {
    right_depth(node) as i32 - left_depth(node) as i32
}

fn subtree_size(node: &Option<NodeRef>) -> u32 {
    node.as_ref().map_or(0, |child| {
        let child = child.borrow();
        child.left_child_count + child.right_child_count + 1
    })
}

fn refresh(node: &NodeRef) {
    let (left_child, right_child) = {
        let borrowed = node.borrow();
        (
            borrowed.left_child_node.clone(),
            borrowed.right_child_node.clone(),
        )
    };

    let left_count = subtree_size(&left_child);
    let right_count = subtree_size(&right_child);
    let depth = left_child
        .as_ref()
        .map_or(0, |child| child.borrow().depth + 1)
        .max(right_child.as_ref().map_or(0, |child| child.borrow().depth + 1));

    let mut borrowed = node.borrow_mut();
    borrowed.left_child_count = left_count;
    borrowed.right_child_count = right_count;
    borrowed.depth = depth;
}

fn set_parent(child: &Option<NodeRef>, parent: Option<&NodeRef>) {
    if let Some(child) = child {
        child.borrow_mut().parent_node = parent.map(Rc::downgrade);
    }
}

fn is_left_child(parent: &NodeRef, child: &NodeRef) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .is_some_and(|left| Rc::ptr_eq(left, child))
}

fn replace_in_parent(tree: &mut BOSTree, node: &NodeRef, replacement: Option<NodeRef>) {
    if let Some(parent) = parent_of(node) {
        if is_left_child(&parent, node) {
            parent.borrow_mut().left_child_node = replacement.clone();
        } else {
            parent.borrow_mut().right_child_node = replacement.clone();
        }
        set_parent(&replacement, Some(&parent));
    } else {
        tree.root_node = replacement.clone();
        set_parent(&replacement, None);
    }
}

fn rotate_right(tree: &mut BOSTree, p: NodeRef) -> NodeRef {
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires a left child");
    let parent = parent_of(&p);

    if let Some(parent) = parent {
        if is_left_child(&parent, &p) {
            parent.borrow_mut().left_child_node = Some(l.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(l.clone());
        }
        l.borrow_mut().parent_node = Some(Rc::downgrade(&parent));
    } else {
        tree.root_node = Some(l.clone());
        l.borrow_mut().parent_node = None;
    }

    let l_right = l.borrow().right_child_node.clone();
    {
        let mut p_mut = p.borrow_mut();
        p_mut.left_child_node = l_right.clone();
    }
    set_parent(&l_right, Some(&p));

    {
        let mut l_mut = l.borrow_mut();
        l_mut.right_child_node = Some(p.clone());
    }
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    refresh(&p);
    refresh(&l);
    l
}

fn rotate_left(tree: &mut BOSTree, p: NodeRef) -> NodeRef {
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires a right child");
    let parent = parent_of(&p);

    if let Some(parent) = parent {
        if is_left_child(&parent, &p) {
            parent.borrow_mut().left_child_node = Some(r.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(r.clone());
        }
        r.borrow_mut().parent_node = Some(Rc::downgrade(&parent));
    } else {
        tree.root_node = Some(r.clone());
        r.borrow_mut().parent_node = None;
    }

    let r_left = r.borrow().left_child_node.clone();
    {
        let mut p_mut = p.borrow_mut();
        p_mut.right_child_node = r_left.clone();
    }
    set_parent(&r_left, Some(&p));

    {
        let mut r_mut = r.borrow_mut();
        r_mut.left_child_node = Some(p.clone());
    }
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    refresh(&p);
    refresh(&r);
    r
}

fn rebalance_node(tree: &mut BOSTree, node: NodeRef) -> NodeRef {
    refresh(&node);
    let current_balance = balance(&node);

    if current_balance < -1 {
        let left = node.borrow().left_child_node.clone();
        if let Some(left) = left {
            if balance(&left) > 0 {
                rotate_left(tree, left);
            }
        }
        rotate_right(tree, node)
    } else if current_balance > 1 {
        let right = node.borrow().right_child_node.clone();
        if let Some(right) = right {
            if balance(&right) < 0 {
                rotate_right(tree, right);
            }
        }
        rotate_left(tree, node)
    } else {
        node
    }
}

fn rebalance_upwards(tree: &mut BOSTree, start: Option<NodeRef>) {
    let mut current = start;
    while let Some(node) = current {
        let new_root = rebalance_node(tree, node);
        refresh(&new_root);
        current = parent_of(&new_root);
    }
}

fn leftmost(mut node: NodeRef) -> NodeRef {
    loop {
        let next = node.borrow().left_child_node.clone();
        match next {
            Some(next) => node = next,
            None => return node,
        }
    }
}

fn rightmost(mut node: NodeRef) -> NodeRef {
    loop {
        let next = node.borrow().right_child_node.clone();
        match next {
            Some(next) => node = next,
            None => return node,
        }
    }
}

impl BOSTree {
    /// Create a new tree with a mandatory comparison function and an optional free function.
    pub fn bostree_new(
        cmp_function: BOSTreeCmpFunction,
        free_function: Option<BOSTreeFreeFunction>,
    ) -> Self {
        Self {
            root_node: None,
            cmp_function,
            free_function,
        }
    }

    /// Return the number of nodes in the tree.
    pub fn bostree_node_count(&self) -> u32 {
        self.root_node
            .as_ref()
            .map_or(0, |root| root.borrow().left_child_count + root.borrow().right_child_count + 1)
    }

    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: None,
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        let Some(mut current) = self.root_node.clone() else {
            self.root_node = Some(new_node.clone());
            return new_node;
        };

        loop {
            let cmp = {
                let new_key = new_node.borrow();
                let current_key = current.borrow();
                (self.cmp_function)(&new_key.key, &current_key.key)
            };

            if cmp < 0 {
                let next = current.borrow().left_child_node.clone();
                if let Some(next) = next {
                    current = next;
                } else {
                    current.borrow_mut().left_child_node = Some(new_node.clone());
                    new_node.borrow_mut().parent_node = Some(Rc::downgrade(&current));
                    rebalance_upwards(self, Some(current));
                    return new_node;
                }
            } else {
                let next = current.borrow().right_child_node.clone();
                if let Some(next) = next {
                    current = next;
                } else {
                    current.borrow_mut().right_child_node = Some(new_node.clone());
                    new_node.borrow_mut().parent_node = Some(Rc::downgrade(&current));
                    rebalance_upwards(self, Some(current));
                    return new_node;
                }
            }
        }
    }

    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let left = node.borrow().left_child_node.clone();
        let right = node.borrow().right_child_node.clone();

        let rebalance_start = match (left, right) {
            (None, None) => {
                let start = parent_of(node);
                replace_in_parent(self, node, None);
                start
            }
            (Some(child), None) | (None, Some(child)) => {
                let start = parent_of(node);
                replace_in_parent(self, node, Some(child));
                start
            }
            (Some(left_child), Some(right_child)) => {
                let use_predecessor = left_child.borrow().depth >= right_child.borrow().depth;
                let candidate = if use_predecessor {
                    rightmost(left_child.clone())
                } else {
                    leftmost(right_child.clone())
                };

                let candidate_parent = parent_of(&candidate);
                let detached_child = if use_predecessor {
                    candidate.borrow().left_child_node.clone()
                } else {
                    candidate.borrow().right_child_node.clone()
                };

                if let Some(candidate_parent) = candidate_parent.clone() {
                    if !Rc::ptr_eq(&candidate_parent, node) {
                        replace_in_parent(self, &candidate, detached_child);
                    }
                }

                let original_left = node.borrow().left_child_node.clone();
                let original_right = node.borrow().right_child_node.clone();
                replace_in_parent(self, node, Some(candidate.clone()));

                if use_predecessor {
                    if candidate_parent
                        .as_ref()
                        .is_some_and(|parent| !Rc::ptr_eq(parent, node))
                    {
                        candidate.borrow_mut().left_child_node = original_left.clone();
                        set_parent(&original_left, Some(&candidate));
                    }
                    candidate.borrow_mut().right_child_node = original_right.clone();
                    set_parent(&original_right, Some(&candidate));
                } else {
                    candidate.borrow_mut().left_child_node = original_left.clone();
                    set_parent(&original_left, Some(&candidate));
                    if candidate_parent
                        .as_ref()
                        .is_some_and(|parent| !Rc::ptr_eq(parent, node))
                    {
                        candidate.borrow_mut().right_child_node = original_right.clone();
                        set_parent(&original_right, Some(&candidate));
                    }
                }

                refresh(&candidate);

                if let Some(candidate_parent) = candidate_parent {
                    if !Rc::ptr_eq(&candidate_parent, node) {
                        rebalance_upwards(self, Some(candidate_parent));
                    }
                }

                Some(candidate)
            }
        };

        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
        node.borrow_mut().parent_node = None;
        node.borrow_mut().left_child_count = 0;
        node.borrow_mut().right_child_count = 0;
        node.borrow_mut().depth = 0;
        node.borrow_mut().weak_ref_node_valid = 0;

        rebalance_upwards(self, rebalance_start);
        self.bostree_node_weak_unref(node);
    }

    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let should_free = {
            let mut borrowed = node.borrow_mut();
            if borrowed.weak_ref_count == 0 {
                return None;
            }
            borrowed.weak_ref_count -= 1;
            borrowed.weak_ref_count == 0
        };

        if should_free {
            if let Some(free_function) = self.free_function {
                free_function(node);
            }
            None
        } else if node.borrow().weak_ref_node_valid != 0 {
            Some(node.clone())
        } else {
            None
        }
    }

    /// Lookup a node in the tree by its key.
    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        while let Some(node) = current {
            let cmp = (self.cmp_function)(key, &node.borrow().key);
            if cmp == 0 {
                return Some(node);
            } else if cmp < 0 {
                current = node.borrow().left_child_node.clone();
            } else {
                current = node.borrow().right_child_node.clone();
            }
        }
        None
    }

    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        let mut remaining = index;

        while let Some(node) = current {
            let left_count = node.borrow().left_child_count;
            if left_count <= remaining {
                remaining -= left_count;
                if remaining == 0 {
                    return Some(node);
                }
                remaining -= 1;
                current = node.borrow().right_child_node.clone();
            } else {
                current = node.borrow().left_child_node.clone();
            }
        }

        None
    }

    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        fn print_helper(node: &NodeRef) {
            let borrowed = node.borrow();
            println!(
                "  {} [label=\"{} ({},{},{})\"];",
                borrowed.key,
                borrowed.key,
                borrowed.left_child_count,
                borrowed.right_child_count,
                borrowed.depth
            );

            if let Some(parent) = borrowed.parent_node.as_ref().and_then(Weak::upgrade) {
                println!("  {} -> {} [color=green];", borrowed.key, parent.borrow().key);
            }

            let left = borrowed.left_child_node.clone();
            let right = borrowed.right_child_node.clone();
            drop(borrowed);

            if let Some(left) = left {
                println!("  {} -> {}", node.borrow().key, left.borrow().key);
                print_helper(&left);
            }
            if let Some(right) = right {
                println!("  {} -> {}", node.borrow().key, right.borrow().key);
                print_helper(&right);
            }
        }

        if let Some(root) = self.root_node.clone() {
            println!("digraph {{");
            println!("  ordering = out;");
            print_helper(&root);
            println!("}}");
        }
    }
}

/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let mut borrowed = node.borrow_mut();
    if borrowed.weak_ref_count > 0 && borrowed.weak_ref_count < 127 {
        borrowed.weak_ref_count += 1;
    }
    drop(borrowed);
    node.clone()
}

/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(right) = node.borrow().right_child_node.clone() {
        return Some(leftmost(right));
    }

    let mut current = node.clone();
    while let Some(parent) = parent_of(&current) {
        if parent
            .borrow()
            .right_child_node
            .as_ref()
            .is_some_and(|right| Rc::ptr_eq(right, &current))
        {
            current = parent;
        } else {
            return Some(parent);
        }
    }
    None
}

/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(left) = node.borrow().left_child_node.clone() {
        return Some(rightmost(left));
    }

    let mut current = node.clone();
    while let Some(parent) = parent_of(&current) {
        if parent
            .borrow()
            .left_child_node
            .as_ref()
            .is_some_and(|left| Rc::ptr_eq(left, &current))
        {
            current = parent;
        } else {
            return Some(parent);
        }
    }
    None
}

/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut current = node.clone();

    while let Some(parent) = parent_of(&current) {
        if parent
            .borrow()
            .right_child_node
            .as_ref()
            .is_some_and(|right| Rc::ptr_eq(right, &current))
        {
            counter += 1 + parent.borrow().left_child_count;
        }
        current = parent;
    }

    counter
}
