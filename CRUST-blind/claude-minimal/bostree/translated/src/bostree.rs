use std::cell::RefCell;
use std::cmp::max;
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

/// Compute the AVL balance factor for a node: right_depth - left_depth.
fn balance_factor(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let left_depth = n
        .left_child_node
        .as_ref()
        .map_or(0i32, |c| c.borrow().depth as i32 + 1);
    let right_depth = n
        .right_child_node
        .as_ref()
        .map_or(0i32, |c| c.borrow().depth as i32 + 1);
    right_depth - left_depth
}

/// Recompute the depth of a node based on its children's depths.
fn recompute_depth(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let n = node.borrow();
    let l_d = n
        .left_child_node
        .as_ref()
        .map_or(0u32, |c| c.borrow().depth + 1);
    let r_d = n
        .right_child_node
        .as_ref()
        .map_or(0u32, |c| c.borrow().depth + 1);
    max(l_d, r_d)
}

/// Rotate the subtree rooted at P to the right.
///
/// ```text
///       P                     L
///   L        R     -->    c1      P
/// c1 c2                        c2     R
/// ```
fn rotate_right(
    tree: &mut BOSTree,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires a left child");

    // Get parent of P (a Weak reference) and its upgraded form.
    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());

    // Update the parent's child pointer to point to L.
    if let Some(parent) = &p_parent {
        let is_left = parent
            .borrow()
            .left_child_node
            .as_ref()
            .map_or(false, |c| Rc::ptr_eq(c, p));
        if is_left {
            parent.borrow_mut().left_child_node = Some(l.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    // L's parent becomes P's old parent.
    l.borrow_mut().parent_node = p_parent_weak;

    // P's left child becomes L's right child.
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;
    if let Some(ref new_left) = l_right {
        new_left.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // Recompute P's depth, then make L its parent.
    let p_new_depth = recompute_depth(p);
    p.borrow_mut().depth = p_new_depth;
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L's right child becomes P.
    l.borrow_mut().right_child_node = Some(p.clone());
    let p_lc = p.borrow().left_child_count;
    let p_rc = p.borrow().right_child_count;
    l.borrow_mut().right_child_count = p_lc + p_rc + 1;

    let l_new_depth = recompute_depth(&l);
    l.borrow_mut().depth = l_new_depth;

    l
}

/// Rotate the subtree rooted at P to the left.
///
/// ```text
///       P                     R
///   L        R     -->    P      c2
///         c1 c2        L  c1
/// ```
fn rotate_left(
    tree: &mut BOSTree,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires a right child");

    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());

    if let Some(parent) = &p_parent {
        let is_left = parent
            .borrow()
            .left_child_node
            .as_ref()
            .map_or(false, |c| Rc::ptr_eq(c, p));
        if is_left {
            parent.borrow_mut().left_child_node = Some(r.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent_weak;

    // P's right child becomes R's left child.
    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;
    if let Some(ref new_right) = r_left {
        new_right.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let p_new_depth = recompute_depth(p);
    p.borrow_mut().depth = p_new_depth;
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    r.borrow_mut().left_child_node = Some(p.clone());
    let p_lc = p.borrow().left_child_count;
    let p_rc = p.borrow().right_child_count;
    r.borrow_mut().left_child_count = p_lc + p_rc + 1;

    let r_new_depth = recompute_depth(&r);
    r.borrow_mut().depth = r_new_depth;

    r
}

impl BOSTree {
    /// Create a new tree with a mandatory comparison function and an optional free function.
    pub fn bostree_new(
        cmp_function: BOSTreeCmpFunction,
        free_function: Option<BOSTreeFreeFunction>,
    ) -> Self {
        BOSTree {
            root_node: None,
            cmp_function,
            free_function,
        }
    }
    /// Return the number of nodes in the tree.
    pub fn bostree_node_count(&self) -> u32 {
        match &self.root_node {
            Some(root) => {
                let r = root.borrow();
                r.left_child_count + r.right_child_count + 1
            }
            None => 0,
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        let cmp_fn = self.cmp_function;

        // Walk down the tree, finding the insertion position and updating
        // child counts along the way.
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;
        let mut current_opt = self.root_node.clone();
        while let Some(current) = current_opt {
            let cmp = cmp_fn(&key, &current.borrow().key);
            let next = if cmp < 0 {
                current.borrow_mut().left_child_count += 1;
                go_left = true;
                current.borrow().left_child_node.clone()
            } else {
                current.borrow_mut().right_child_count += 1;
                go_left = false;
                current.borrow().right_child_node.clone()
            };
            parent_node = Some(current);
            current_opt = next;
        }

        // Construct the new node.
        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: parent_node.as_ref().map(Rc::downgrade),
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        // If there is no parent, the new node is the root and we are done.
        let parent = match parent_node {
            Some(p) => {
                if go_left {
                    p.borrow_mut().left_child_node = Some(new_node.clone());
                } else {
                    p.borrow_mut().right_child_node = Some(new_node.clone());
                }
                p
            }
            None => {
                self.root_node = Some(new_node.clone());
                return new_node;
            }
        };

        // The depth of the parent only changes if it had no other child before
        // (i.e. exactly one of its children is now present).
        let parent_only_child = {
            let p_b = parent.borrow();
            p_b.left_child_node.is_some() ^ p_b.right_child_node.is_some()
        };

        if parent_only_child {
            parent.borrow_mut().depth += 1;

            // Walk up from the parent, updating depths and rebalancing.
            let mut current = parent;
            loop {
                let next_parent = current
                    .borrow()
                    .parent_node
                    .as_ref()
                    .and_then(|w| w.upgrade());
                let p = match next_parent {
                    Some(p) => p,
                    None => break,
                };

                let new_left_depth = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let new_right_depth = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let max_depth = max(new_left_depth, new_right_depth);

                if p.borrow().depth != max_depth {
                    p.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                // Check AVL property and rotate if necessary.
                let mut p_after = p;
                if new_left_depth >= new_right_depth + 2 {
                    // Left is two levels deeper than right: rotate right.
                    let left_child =
                        p_after.borrow().left_child_node.clone().unwrap();
                    if balance_factor(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    p_after = rotate_right(self, &p_after);
                } else if new_right_depth >= new_left_depth + 2 {
                    // Right is two levels deeper than left: rotate left.
                    let right_child =
                        p_after.borrow().right_child_node.clone().unwrap();
                    if balance_factor(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    p_after = rotate_left(self, &p_after);
                }

                current = p_after;
            }
        }

        new_node
    }
    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>>;

        let has_both_children = {
            let n = node.borrow();
            n.left_child_node.is_some() && n.right_child_node.is_some()
        };

        if has_both_children {
            // Pick a candidate to replace `node`. Prefer the deeper subtree
            // to keep the tree balanced.
            let go_left = {
                let n = node.borrow();
                let left_depth = n.left_child_node.as_ref().unwrap().borrow().depth;
                let right_depth = n.right_child_node.as_ref().unwrap().borrow().depth;
                left_depth >= right_depth
            };

            let (candidate, lost_child) = if go_left {
                node.borrow_mut().left_child_count -= 1;
                let mut cand = node.borrow().left_child_node.clone().unwrap();
                loop {
                    let next = cand.borrow().right_child_node.clone();
                    match next {
                        Some(c) => {
                            cand.borrow_mut().right_child_count -= 1;
                            cand = c;
                        }
                        None => break,
                    }
                }
                let lc = cand.borrow().left_child_node.clone();
                (cand, lc)
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut cand = node.borrow().right_child_node.clone().unwrap();
                loop {
                    let next = cand.borrow().left_child_node.clone();
                    match next {
                        Some(c) => {
                            cand.borrow_mut().left_child_count -= 1;
                            cand = c;
                        }
                        None => break,
                    }
                }
                let rc = cand.borrow().right_child_node.clone();
                (cand, rc)
            };

            let bubble_start = candidate
                .borrow()
                .parent_node
                .as_ref()
                .and_then(|w| w.upgrade())
                .expect("candidate must have a parent");

            // Splice candidate out of its current position. The lost_child
            // (which is candidate's only child, on the opposite side of the
            // direction we descended) takes its place.
            let bs_left_is_cand = bubble_start
                .borrow()
                .left_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, &candidate));
            if bs_left_is_cand {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Move candidate to the place where `node` used to be.
            let node_parent_weak = node.borrow().parent_node.clone();
            let node_parent_upgraded =
                node_parent_weak.as_ref().and_then(|w| w.upgrade());
            if let Some(ref np) = node_parent_upgraded {
                let np_left_is_node = np
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, node));
                if np_left_is_node {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent_weak;

            // Copy node's children/counts to candidate. Note that node's
            // children may have been modified above when bubble_start == node,
            // so we read them after the splice.
            let (n_left, n_right, n_lc, n_rc) = {
                let n = node.borrow();
                (
                    n.left_child_node.clone(),
                    n.right_child_node.clone(),
                    n.left_child_count,
                    n.right_child_count,
                )
            };
            candidate.borrow_mut().left_child_node = n_left.clone();
            candidate.borrow_mut().left_child_count = n_lc;
            candidate.borrow_mut().right_child_node = n_right.clone();
            candidate.borrow_mut().right_child_count = n_rc;

            if let Some(ref l) = n_left {
                l.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref r) = n_right {
                r.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to (but not including) candidate.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start;
                while !Rc::ptr_eq(&bs, &candidate) {
                    let new_depth = recompute_depth(&bs);
                    bs.borrow_mut().depth = new_depth;
                    let bal = balance_factor(&bs);
                    let bs_after = if bal > 1 {
                        let r_child =
                            bs.borrow().right_child_node.clone().unwrap();
                        if balance_factor(&r_child) < 0 {
                            rotate_right(self, &r_child);
                        }
                        rotate_left(self, &bs)
                    } else if bal < -1 {
                        let l_child =
                            bs.borrow().left_child_node.clone().unwrap();
                        if balance_factor(&l_child) > 0 {
                            rotate_left(self, &l_child);
                        }
                        rotate_right(self, &bs)
                    } else {
                        bs.clone()
                    };
                    let parent_opt = bs_after
                        .borrow()
                        .parent_node
                        .as_ref()
                        .and_then(|w| w.upgrade());
                    match parent_opt {
                        Some(p) => bs = p,
                        None => break,
                    }
                }
            }

            // Recompute candidate's own depth.
            let cand_new_depth = recompute_depth(&candidate);
            candidate.borrow_mut().depth = cand_new_depth;

            bubble_up = candidate
                .borrow()
                .parent_node
                .as_ref()
                .and_then(|w| w.upgrade());

            // The candidate's parent has lost one descendant.
            if let Some(ref bu) = bubble_up {
                let bu_left_is_cand = bu
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &candidate));
                if bu_left_is_cand {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // The node has at most one child; removal is much simpler.
            let node_parent_upgraded = node
                .borrow()
                .parent_node
                .as_ref()
                .and_then(|w| w.upgrade());

            if node_parent_upgraded.is_none() {
                // Node was the root.
                let (left, right) = {
                    let n = node.borrow();
                    (n.left_child_node.clone(), n.right_child_node.clone())
                };
                if let Some(ref l) = left {
                    self.root_node = Some(l.clone());
                    l.borrow_mut().parent_node = None;
                } else {
                    self.root_node = right.clone();
                    if let Some(ref r) = right {
                        r.borrow_mut().parent_node = None;
                    }
                }
                bubble_up = None;
            } else {
                let np = node_parent_upgraded.unwrap();
                let (candidate, candidate_count) = {
                    let n = node.borrow();
                    if let Some(ref r) = n.right_child_node {
                        (Some(r.clone()), n.right_child_count)
                    } else {
                        (n.left_child_node.clone(), n.left_child_count)
                    }
                };

                let np_left_is_node = np
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, node));
                if np_left_is_node {
                    np.borrow_mut().left_child_node = candidate.clone();
                    np.borrow_mut().left_child_count = candidate_count;
                } else {
                    np.borrow_mut().right_child_node = candidate.clone();
                    np.borrow_mut().right_child_count = candidate_count;
                }
                if let Some(ref c) = candidate {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&np));
                }
                bubble_up = Some(np);
            }
        }

        // Walk up from bubble_up, recomputing depths/balances and updating
        // child counts up to the root.
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up {
            let new_bu;
            if !bubbling_finished {
                let new_depth = recompute_depth(&bu);
                let depth_changed = bu.borrow().depth != new_depth;
                bu.borrow_mut().depth = new_depth;

                let bal = balance_factor(&bu);
                if bal < -1 {
                    let l_child = bu.borrow().left_child_node.clone().unwrap();
                    if balance_factor(&l_child) > 0 {
                        rotate_left(self, &l_child);
                    }
                    new_bu = rotate_right(self, &bu);
                } else if bal > 1 {
                    let r_child = bu.borrow().right_child_node.clone().unwrap();
                    if balance_factor(&r_child) < 0 {
                        rotate_right(self, &r_child);
                    }
                    new_bu = rotate_left(self, &bu);
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    new_bu = bu;
                }
            } else {
                new_bu = bu;
            }

            // Update parent's child count to reflect the removed node.
            let parent_opt = new_bu
                .borrow()
                .parent_node
                .as_ref()
                .and_then(|w| w.upgrade());
            if let Some(ref p) = parent_opt {
                let p_left_is_bu = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &new_bu));
                if p_left_is_bu {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up = parent_opt;
        }

        // Mark the node invalid and decrement its weak reference count.
        node.borrow_mut().weak_ref_node_valid = 0;
        self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        node.borrow_mut().weak_ref_count -= 1;
        let weak_ref_count = node.borrow().weak_ref_count;
        let valid = node.borrow().weak_ref_node_valid;
        if weak_ref_count == 0 {
            if let Some(free_fn) = self.free_function {
                free_fn(node);
            }
            None
        } else if valid != 0 {
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
    pub fn bostree_select(&self, mut index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        while let Some(node) = current {
            let lcc = node.borrow().left_child_count;
            if lcc <= index {
                index -= lcc;
                if index == 0 {
                    return Some(node);
                }
                index -= 1;
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
        if let Some(ref root) = self.root_node {
            println!("digraph {{\n  ordering = out;");
            print_helper(root);
            println!("}}");
        }
    }
}

#[cfg(debug_assertions)]
fn print_helper(node: &Rc<RefCell<BOSNode>>) {
    let n = node.borrow();
    println!(
        "  {} [label=\"\\N ({},{},{})\"];",
        n.key, n.left_child_count, n.right_child_count, n.depth
    );
    if let Some(ref parent_weak) = n.parent_node {
        if let Some(parent) = parent_weak.upgrade() {
            println!("  {} -> {} [color=green];", n.key, parent.borrow().key);
        }
    }
    if let Some(ref left) = n.left_child_node {
        println!("  {} -> {}", n.key, left.borrow().key);
        print_helper(left);
    }
    if let Some(ref right) = n.right_child_node {
        println!("  {} -> {}", n.key, right.borrow().key);
        print_helper(right);
    }
}

/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    {
        let n = node.borrow();
        assert!(n.weak_ref_count < 127);
        assert!(n.weak_ref_count > 0);
    }
    node.borrow_mut().weak_ref_count += 1;
    node.clone()
}
/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    // If there is a right subtree, the successor is the leftmost node in it.
    let right = node.borrow().right_child_node.clone();
    if let Some(r) = right {
        let mut current = r;
        loop {
            let next = current.borrow().left_child_node.clone();
            match next {
                Some(c) => current = c,
                None => break,
            }
        }
        return Some(current);
    }

    // Otherwise, walk up while we are the right child of our parent.
    let mut current = node.clone();
    loop {
        let parent_opt = current
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent_opt {
            None => return None,
            Some(parent) => {
                let is_right = parent
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_right {
                    current = parent;
                } else {
                    return Some(parent);
                }
            }
        }
    }
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    // If there is a left subtree, the predecessor is the rightmost node in it.
    let left = node.borrow().left_child_node.clone();
    if let Some(l) = left {
        let mut current = l;
        loop {
            let next = current.borrow().right_child_node.clone();
            match next {
                Some(c) => current = c,
                None => break,
            }
        }
        return Some(current);
    }

    // Otherwise, walk up while we are the left child of our parent.
    let mut current = node.clone();
    loop {
        let parent_opt = current
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent_opt {
            None => return None,
            Some(parent) => {
                let is_left = parent
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_left {
                    current = parent;
                } else {
                    return Some(parent);
                }
            }
        }
    }
}
/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut current = node.clone();
    loop {
        let parent_opt = current
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent_opt {
            None => break,
            Some(parent) => {
                let is_right = parent
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_right {
                    counter += 1 + parent.borrow().left_child_count;
                }
                current = parent;
            }
        }
    }
    counter
}
