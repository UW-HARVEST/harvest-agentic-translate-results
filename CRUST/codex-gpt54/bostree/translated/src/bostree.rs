use std::cell::RefCell;
use std::rc::{Rc, Weak};

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

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(Weak::upgrade)
}

fn child_depth(node: Option<&Rc<RefCell<BOSNode>>>) -> u32 {
    node.map_or(0, |node| node.borrow().depth + 1)
}

fn max_u32(a: u32, b: u32) -> u32 {
    a.max(b)
}

fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let (left, right) = {
        let borrowed = node.borrow();
        (
            child_depth(borrowed.left_child_node.as_ref()),
            child_depth(borrowed.right_child_node.as_ref()),
        )
    };
    node.borrow_mut().depth = max_u32(left, right);
}

fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let borrowed = node.borrow();
    child_depth(borrowed.right_child_node.as_ref()) as i32
        - child_depth(borrowed.left_child_node.as_ref()) as i32
}

fn is_left_child(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .is_some_and(|left| Rc::ptr_eq(left, child))
}

fn set_parent(child: &Option<Rc<RefCell<BOSNode>>>, parent: Option<&Rc<RefCell<BOSNode>>>) {
    if let Some(child) = child {
        child.borrow_mut().parent_node = parent.map(Rc::downgrade);
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
        self.root_node.as_ref().map_or(0, |root| {
            let borrowed = root.borrow();
            borrowed.left_child_count + borrowed.right_child_count + 1
        })
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        let mut current = self.root_node.clone();
        let mut parent = None;
        let mut insert_left = false;

        while let Some(node) = current {
            let cmp = (self.cmp_function)(&key, &node.borrow().key);
            parent = Some(node.clone());
            if cmp < 0 {
                node.borrow_mut().left_child_count += 1;
                insert_left = true;
                current = node.borrow().left_child_node.clone();
            } else {
                node.borrow_mut().right_child_count += 1;
                insert_left = false;
                current = node.borrow().right_child_node.clone();
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: parent.as_ref().map(Rc::downgrade),
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        if let Some(parent) = parent {
            if insert_left {
                parent.borrow_mut().left_child_node = Some(new_node.clone());
            } else {
                parent.borrow_mut().right_child_node = Some(new_node.clone());
            }

            let first_child = {
                let borrowed = parent.borrow();
                borrowed.left_child_node.is_some() ^ borrowed.right_child_node.is_some()
            };
            if first_child {
                parent.borrow_mut().depth += 1;
                let mut bubble = parent;
                while let Some(next_parent) = parent_of(&bubble) {
                    bubble = next_parent;
                    let (left_depth, right_depth, old_depth) = {
                        let borrowed = bubble.borrow();
                        (
                            child_depth(borrowed.left_child_node.as_ref()),
                            child_depth(borrowed.right_child_node.as_ref()),
                            borrowed.depth,
                        )
                    };
                    let new_depth = max_u32(left_depth, right_depth);
                    if old_depth != new_depth {
                        bubble.borrow_mut().depth = new_depth;
                    } else {
                        break;
                    }

                    if left_depth == right_depth + 2 {
                        let left_child = bubble.borrow().left_child_node.clone().unwrap();
                        if balance(&left_child) > 0 {
                            self.rotate_left(left_child);
                        }
                        bubble = self.rotate_right(bubble);
                    } else if right_depth == left_depth + 2 {
                        let right_child = bubble.borrow().right_child_node.clone().unwrap();
                        if balance(&right_child) < 0 {
                            self.rotate_right(right_child);
                        }
                        bubble = self.rotate_left(bubble);
                    }
                }
            }
        } else {
            self.root_node = Some(new_node.clone());
        }

        new_node
    }
    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>> = None;

        let has_two_children = {
            let borrowed = node.borrow();
            borrowed.left_child_node.is_some() && borrowed.right_child_node.is_some()
        };

        if has_two_children {
            let (candidate, lost_child, bubble_start) = {
                let mut node_borrow = node.borrow_mut();
                let left_depth = child_depth(node_borrow.left_child_node.as_ref());
                let right_depth = child_depth(node_borrow.right_child_node.as_ref());
                if left_depth >= right_depth {
                    node_borrow.left_child_count -= 1;
                    let mut candidate = node_borrow.left_child_node.clone().unwrap();
                    drop(node_borrow);
                    loop {
                        let next = candidate.borrow().right_child_node.clone();
                        if let Some(right) = next {
                            candidate.borrow_mut().right_child_count -= 1;
                            candidate = right;
                        } else {
                            break;
                        }
                    }
                    let lost_child = candidate.borrow().left_child_node.clone();
                    let bubble_start = parent_of(&candidate).unwrap();
                    (candidate, lost_child, bubble_start)
                } else {
                    node_borrow.right_child_count -= 1;
                    let mut candidate = node_borrow.right_child_node.clone().unwrap();
                    drop(node_borrow);
                    loop {
                        let next = candidate.borrow().left_child_node.clone();
                        if let Some(left) = next {
                            candidate.borrow_mut().left_child_count -= 1;
                            candidate = left;
                        } else {
                            break;
                        }
                    }
                    let lost_child = candidate.borrow().right_child_node.clone();
                    let bubble_start = parent_of(&candidate).unwrap();
                    (candidate, lost_child, bubble_start)
                }
            };

            if is_left_child(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            set_parent(&lost_child, Some(&bubble_start));

            let node_parent = parent_of(node);
            if let Some(parent) = node_parent.as_ref() {
                if is_left_child(parent, node) {
                    parent.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    parent.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            let (new_left, new_left_count, new_right, new_right_count) = {
                let borrowed = node.borrow();
                (
                    borrowed.left_child_node.clone(),
                    borrowed.left_child_count,
                    borrowed.right_child_node.clone(),
                    borrowed.right_child_count,
                )
            };
            {
                let mut candidate_borrow = candidate.borrow_mut();
                candidate_borrow.left_child_node = new_left.clone();
                candidate_borrow.left_child_count = new_left_count;
                candidate_borrow.right_child_node = new_right.clone();
                candidate_borrow.right_child_count = new_right_count;
            }
            set_parent(&new_left, Some(&candidate));
            set_parent(&new_right, Some(&candidate));

            if !Rc::ptr_eq(&bubble_start, node) {
                let mut current = bubble_start;
                while !Rc::ptr_eq(&current, &candidate) {
                    update_depth(&current);
                    let node_balance = balance(&current);
                    if node_balance > 1 {
                        let right_child = current.borrow().right_child_node.clone().unwrap();
                        if balance(&right_child) < 0 {
                            self.rotate_right(right_child);
                        }
                        current = self.rotate_left(current);
                    } else if node_balance < -1 {
                        let left_child = current.borrow().left_child_node.clone().unwrap();
                        if balance(&left_child) > 0 {
                            self.rotate_left(left_child);
                        }
                        current = self.rotate_right(current);
                    }
                    current = parent_of(&current).unwrap();
                }
            }

            update_depth(&candidate);
            bubble_up = parent_of(&candidate);
            if let Some(parent) = bubble_up.as_ref() {
                if is_left_child(parent, &candidate) {
                    parent.borrow_mut().left_child_count -= 1;
                } else {
                    parent.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            let node_parent = parent_of(node);
            let (candidate, candidate_count) = {
                let borrowed = node.borrow();
                if let Some(left) = borrowed.left_child_node.clone() {
                    (Some(left), borrowed.left_child_count)
                } else if let Some(right) = borrowed.right_child_node.clone() {
                    (Some(right), borrowed.right_child_count)
                } else {
                    (None, 0)
                }
            };

            if let Some(parent) = node_parent.as_ref() {
                if is_left_child(parent, node) {
                    let mut parent_borrow = parent.borrow_mut();
                    parent_borrow.left_child_node = candidate.clone();
                    parent_borrow.left_child_count = candidate_count;
                } else {
                    let mut parent_borrow = parent.borrow_mut();
                    parent_borrow.right_child_node = candidate.clone();
                    parent_borrow.right_child_count = candidate_count;
                }
                set_parent(&candidate, Some(parent));
                bubble_up = Some(parent.clone());
            } else {
                self.root_node = candidate.clone();
                set_parent(&candidate, None);
            }
        }

        let mut bubbling_finished = false;
        while let Some(current) = bubble_up.clone() {
            if !bubbling_finished {
                let (left_depth, right_depth, old_depth) = {
                    let borrowed = current.borrow();
                    (
                        child_depth(borrowed.left_child_node.as_ref()),
                        child_depth(borrowed.right_child_node.as_ref()),
                        borrowed.depth,
                    )
                };
                let new_depth = max_u32(left_depth, right_depth);
                let depth_changed = new_depth != old_depth;
                current.borrow_mut().depth = new_depth;

                let node_balance = balance(&current);
                bubble_up = if node_balance < -1 {
                    let left_child = current.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        self.rotate_left(left_child);
                    }
                    Some(self.rotate_right(current))
                } else if node_balance > 1 {
                    let right_child = current.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        self.rotate_right(right_child);
                    }
                    Some(self.rotate_left(current))
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    Some(current)
                };
            }

            let current = bubble_up.take().unwrap();
            let parent = parent_of(&current);
            if let Some(parent) = parent.as_ref() {
                if is_left_child(parent, &current) {
                    parent.borrow_mut().left_child_count -= 1;
                } else {
                    parent.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up = parent;
        }

        {
            let mut borrowed = node.borrow_mut();
            borrowed.left_child_node = None;
            borrowed.right_child_node = None;
            borrowed.parent_node = None;
            borrowed.left_child_count = 0;
            borrowed.right_child_count = 0;
            borrowed.depth = 0;
            borrowed.weak_ref_node_valid = 0;
        }
        let _ = self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let (should_free, is_valid) = {
            let mut borrowed = node.borrow_mut();
            borrowed.weak_ref_count -= 1;
            (borrowed.weak_ref_count == 0, borrowed.weak_ref_node_valid != 0)
        };

        if should_free {
            if let Some(free_function) = self.free_function {
                free_function(node);
            }
            None
        } else if is_valid {
            Some(node.clone())
        } else {
            None
        }
    }
    /// Lookup a node in the tree by its key.
    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let mut node = self.root_node.clone();
        while let Some(current) = node {
            let cmp = (self.cmp_function)(key, &current.borrow().key);
            if cmp == 0 {
                return Some(current);
            } else if cmp < 0 {
                node = current.borrow().left_child_node.clone();
            } else {
                node = current.borrow().right_child_node.clone();
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut index = index;
        let mut node = self.root_node.clone();
        while let Some(current) = node {
            let left_count = current.borrow().left_child_count;
            if left_count <= index {
                index -= left_count;
                if index == 0 {
                    return Some(current);
                }
                index -= 1;
                node = current.borrow().right_child_node.clone();
            } else {
                node = current.borrow().left_child_node.clone();
            }
        }
        None
    }
    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        fn print_helper(node: &Rc<RefCell<BOSNode>>) {
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

        if let Some(root) = self.root_node.as_ref() {
            println!("digraph {{\n  ordering = out;");
            print_helper(root);
            println!("}}");
        }
    }

    fn rotate_right(&mut self, p: Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let l = p.borrow().left_child_node.clone().unwrap();
        let parent = parent_of(&p);

        if let Some(parent) = parent.as_ref() {
            if is_left_child(parent, &p) {
                parent.borrow_mut().left_child_node = Some(l.clone());
            } else {
                parent.borrow_mut().right_child_node = Some(l.clone());
            }
        } else {
            self.root_node = Some(l.clone());
        }
        l.borrow_mut().parent_node = parent.as_ref().map(Rc::downgrade);

        let (l_right, l_right_count) = {
            let borrowed = l.borrow();
            (borrowed.right_child_node.clone(), borrowed.right_child_count)
        };
        {
            let mut p_borrow = p.borrow_mut();
            p_borrow.left_child_node = l_right.clone();
            p_borrow.left_child_count = l_right_count;
        }
        set_parent(&l_right, Some(&p));
        update_depth(&p);
        p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

        {
            let mut l_borrow = l.borrow_mut();
            l_borrow.right_child_node = Some(p.clone());
            let p_borrow = p.borrow();
            l_borrow.right_child_count = p_borrow.left_child_count + p_borrow.right_child_count + 1;
        }
        update_depth(&l);
        l
    }

    fn rotate_left(&mut self, p: Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let r = p.borrow().right_child_node.clone().unwrap();
        let parent = parent_of(&p);

        if let Some(parent) = parent.as_ref() {
            if is_left_child(parent, &p) {
                parent.borrow_mut().left_child_node = Some(r.clone());
            } else {
                parent.borrow_mut().right_child_node = Some(r.clone());
            }
        } else {
            self.root_node = Some(r.clone());
        }
        r.borrow_mut().parent_node = parent.as_ref().map(Rc::downgrade);

        let (r_left, r_left_count) = {
            let borrowed = r.borrow();
            (borrowed.left_child_node.clone(), borrowed.left_child_count)
        };
        {
            let mut p_borrow = p.borrow_mut();
            p_borrow.right_child_node = r_left.clone();
            p_borrow.right_child_count = r_left_count;
        }
        set_parent(&r_left, Some(&p));
        update_depth(&p);
        p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

        {
            let mut r_borrow = r.borrow_mut();
            r_borrow.left_child_node = Some(p.clone());
            let p_borrow = p.borrow();
            r_borrow.left_child_count = p_borrow.left_child_count + p_borrow.right_child_count + 1;
        }
        update_depth(&r);
        r
    }
}
/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    {
        let mut borrowed = node.borrow_mut();
        assert!(borrowed.weak_ref_count < 127);
        assert!(borrowed.weak_ref_count > 0);
        borrowed.weak_ref_count += 1;
    }
    node.clone()
}
/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(mut current) = node.borrow().right_child_node.clone() {
        loop {
            let next = current.borrow().left_child_node.clone();
            if let Some(left) = next {
                current = left;
            } else {
                return Some(current);
            }
        }
    }

    let mut current = node.clone();
    while let Some(parent) = parent_of(&current) {
        if !is_left_child(&parent, &current) {
            current = parent;
        } else {
            return Some(parent);
        }
    }
    None
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(mut current) = node.borrow().left_child_node.clone() {
        loop {
            let next = current.borrow().right_child_node.clone();
            if let Some(right) = next {
                current = right;
            } else {
                return Some(current);
            }
        }
    }

    let mut current = node.clone();
    while let Some(parent) = parent_of(&current) {
        if is_left_child(&parent, &current) {
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
    let mut current = Some(node.clone());
    while let Some(node) = current {
        if let Some(parent) = parent_of(&node) {
            if parent
                .borrow()
                .right_child_node
                .as_ref()
                .is_some_and(|right| Rc::ptr_eq(right, &node))
            {
                counter += 1 + parent.borrow().left_child_count;
            }
            current = Some(parent);
        } else {
            current = None;
        }
    }
    counter
}
