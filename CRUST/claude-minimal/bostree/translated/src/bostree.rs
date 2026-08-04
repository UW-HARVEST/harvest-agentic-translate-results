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

// ---- Internal helpers ----

fn child_depth(child: &Option<Rc<RefCell<BOSNode>>>) -> u32 {
    child.as_ref().map_or(0, |n| n.borrow().depth + 1)
}

fn node_balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let left = child_depth(&n.left_child_node) as i32;
    let right = child_depth(&n.right_child_node) as i32;
    right - left
}

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn is_left_child_of(node: &Rc<RefCell<BOSNode>>, parent: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |c| Rc::ptr_eq(c, node))
}

fn rotate_right(
    tree: &mut BOSTree,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    // Rotate right:
    //
    //      P                     L
    //  L        R     -->    c1      P
    //c1 c2                        c2     R
    //
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires a left child");

    let p_parent = parent_of(p);

    // Update parent's pointer to point at L instead of P (or update root).
    if let Some(ref parent) = p_parent {
        if is_left_child_of(p, parent) {
            parent.borrow_mut().left_child_node = Some(l.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    // L's parent now is P's old parent.
    l.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P's left child becomes L's previous right child.
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.left_child_node = l_right.clone();
        p_mut.left_child_count = l_right_count;
    }
    if let Some(ref c) = l_right {
        c.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // Recompute P's depth.
    let p_new_depth = {
        let p_b = p.borrow();
        max(child_depth(&p_b.left_child_node), child_depth(&p_b.right_child_node))
    };
    p.borrow_mut().depth = p_new_depth;
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L's right child becomes P.
    l.borrow_mut().right_child_node = Some(p.clone());
    let p_lcc = p.borrow().left_child_count;
    let p_rcc = p.borrow().right_child_count;
    l.borrow_mut().right_child_count = p_lcc + p_rcc + 1;

    let l_new_depth = {
        let l_b = l.borrow();
        max(child_depth(&l_b.left_child_node), child_depth(&l_b.right_child_node))
    };
    l.borrow_mut().depth = l_new_depth;

    l
}

fn rotate_left(
    tree: &mut BOSTree,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    // Rotate left:
    //
    //      P                     R
    //  L        R     -->    P      c2
    //         c1 c2        L  c1
    //
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires a right child");

    let p_parent = parent_of(p);

    if let Some(ref parent) = p_parent {
        if is_left_child_of(p, parent) {
            parent.borrow_mut().left_child_node = Some(r.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.right_child_node = r_left.clone();
        p_mut.right_child_count = r_left_count;
    }
    if let Some(ref c) = r_left {
        c.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let p_new_depth = {
        let p_b = p.borrow();
        max(child_depth(&p_b.left_child_node), child_depth(&p_b.right_child_node))
    };
    p.borrow_mut().depth = p_new_depth;
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    r.borrow_mut().left_child_node = Some(p.clone());
    let p_lcc = p.borrow().left_child_count;
    let p_rcc = p.borrow().right_child_count;
    r.borrow_mut().left_child_count = p_lcc + p_rcc + 1;

    let r_new_depth = {
        let r_b = r.borrow();
        max(child_depth(&r_b.left_child_node), child_depth(&r_b.right_child_node))
    };
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

        // Empty tree: become the root.
        if self.root_node.is_none() {
            self.root_node = Some(new_node.clone());
            return new_node;
        }

        // Walk down to find the parent. Increment child counts along the way.
        let mut current = self.root_node.clone().unwrap();
        let went_left;
        loop {
            let cmp = (self.cmp_function)(&new_node.borrow().key, &current.borrow().key);
            if cmp < 0 {
                current.borrow_mut().left_child_count += 1;
                let next = current.borrow().left_child_node.clone();
                match next {
                    Some(n) => current = n,
                    None => {
                        went_left = true;
                        break;
                    }
                }
            } else {
                current.borrow_mut().right_child_count += 1;
                let next = current.borrow().right_child_node.clone();
                match next {
                    Some(n) => current = n,
                    None => {
                        went_left = false;
                        break;
                    }
                }
            }
        }

        let mut parent_node = current;
        new_node.borrow_mut().parent_node = Some(Rc::downgrade(&parent_node));

        // Track whether parent had any other child before this insert so we know
        // whether the parent's depth grew.
        let parent_had_other_child = if went_left {
            parent_node.borrow().right_child_node.is_some()
        } else {
            parent_node.borrow().left_child_node.is_some()
        };

        if went_left {
            parent_node.borrow_mut().left_child_node = Some(new_node.clone());
        } else {
            parent_node.borrow_mut().right_child_node = Some(new_node.clone());
        }

        // The depth of the parent only changes if this is the parent's first child.
        if !parent_had_other_child {
            parent_node.borrow_mut().depth += 1;

            while let Some(grand) = parent_of(&parent_node) {
                parent_node = grand;

                let new_left_depth = child_depth(&parent_node.borrow().left_child_node);
                let new_right_depth = child_depth(&parent_node.borrow().right_child_node);
                let max_depth = max(new_left_depth, new_right_depth);

                let prev_depth = parent_node.borrow().depth;
                if prev_depth != max_depth {
                    parent_node.borrow_mut().depth = max_depth;
                } else {
                    // Depth did not change; balance also did not change at higher levels.
                    break;
                }

                // Check the AVL property using signed comparison to avoid wrap-around.
                let lhs = new_left_depth as i32;
                let rhs = new_right_depth as i32;
                if lhs - 2 == rhs {
                    // Left-heavy. Possibly a left-right case.
                    let left_child = parent_node.borrow().left_child_node.clone().unwrap();
                    if node_balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    parent_node = rotate_right(self, &parent_node);
                } else if lhs + 2 == rhs {
                    // Right-heavy. Possibly a right-left case.
                    let right_child = parent_node.borrow().right_child_node.clone().unwrap();
                    if node_balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    parent_node = rotate_left(self, &parent_node);
                }
            }
        }

        new_node
    }
    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        let bubble_up: Option<Rc<RefCell<BOSNode>>>;

        if has_left && has_right {
            // Both children exist; find an in-order neighbor to replace this node.
            let left_depth = node.borrow().left_child_node.as_ref().unwrap().borrow().depth;
            let right_depth = node
                .borrow()
                .right_child_node
                .as_ref()
                .unwrap()
                .borrow()
                .depth;

            let candidate;
            let lost_child;

            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut c = node.borrow().left_child_node.clone().unwrap();
                loop {
                    let next_right = c.borrow().right_child_node.clone();
                    match next_right {
                        Some(r) => {
                            c.borrow_mut().right_child_count -= 1;
                            c = r;
                        }
                        None => break,
                    }
                }
                lost_child = c.borrow().left_child_node.clone();
                candidate = c;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut c = node.borrow().right_child_node.clone().unwrap();
                loop {
                    let next_left = c.borrow().left_child_node.clone();
                    match next_left {
                        Some(l) => {
                            c.borrow_mut().left_child_count -= 1;
                            c = l;
                        }
                        None => break,
                    }
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            // bubble_start is candidate's old parent.
            let bubble_start = parent_of(&candidate).expect("candidate must have parent");

            // Detach candidate from bubble_start, replacing with lost_child.
            if is_left_child_of(&candidate, &bubble_start) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate into node's position.
            let node_parent = parent_of(node);
            if let Some(ref np) = node_parent {
                if is_left_child_of(node, np) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            // Take over node's children/counts.
            let node_left_child = node.borrow().left_child_node.clone();
            let node_left_count = node.borrow().left_child_count;
            let node_right_child = node.borrow().right_child_node.clone();
            let node_right_count = node.borrow().right_child_count;
            {
                let mut cm = candidate.borrow_mut();
                cm.left_child_node = node_left_child.clone();
                cm.left_child_count = node_left_count;
                cm.right_child_node = node_right_child.clone();
                cm.right_child_count = node_right_count;
            }
            if let Some(ref l) = node_left_child {
                l.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref r) = node_right_child {
                r.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate (exclusive),
            // unless bubble_start is the original node (which is being removed).
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = Some(bubble_start);
                while let Some(cur) = bs {
                    if Rc::ptr_eq(&cur, &candidate) {
                        break;
                    }

                    let new_depth = {
                        let b = cur.borrow();
                        max(child_depth(&b.left_child_node), child_depth(&b.right_child_node))
                    };
                    cur.borrow_mut().depth = new_depth;

                    let balance = node_balance(&cur);
                    let next: Rc<RefCell<BOSNode>>;
                    if balance > 1 {
                        let rc = cur.borrow().right_child_node.clone().unwrap();
                        if node_balance(&rc) < 0 {
                            rotate_right(self, &rc);
                        }
                        next = rotate_left(self, &cur);
                    } else if balance < -1 {
                        let lc = cur.borrow().left_child_node.clone().unwrap();
                        if node_balance(&lc) > 0 {
                            rotate_left(self, &lc);
                        }
                        next = rotate_right(self, &cur);
                    } else {
                        next = cur.clone();
                    }
                    bs = parent_of(&next);
                }
            }

            // Recompute candidate's depth.
            let cand_new_depth = {
                let b = candidate.borrow();
                max(child_depth(&b.left_child_node), child_depth(&b.right_child_node))
            };
            candidate.borrow_mut().depth = cand_new_depth;

            bubble_up = parent_of(&candidate);
            if let Some(ref bp) = bubble_up {
                if is_left_child_of(&candidate, bp) {
                    bp.borrow_mut().left_child_count -= 1;
                } else {
                    bp.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // 0 or 1 child
            let node_parent = parent_of(node);
            if node_parent.is_none() {
                // Removing the root.
                let new_root = if has_left {
                    node.borrow().left_child_node.clone()
                } else {
                    node.borrow().right_child_node.clone()
                };
                if let Some(ref nr) = new_root {
                    nr.borrow_mut().parent_node = None;
                }
                self.root_node = new_root;
                bubble_up = None;
            } else {
                let np = node_parent.unwrap();
                let candidate;
                let candidate_count;
                if has_right {
                    candidate = node.borrow().right_child_node.clone();
                    candidate_count = node.borrow().right_child_count;
                } else if has_left {
                    candidate = node.borrow().left_child_node.clone();
                    candidate_count = node.borrow().left_child_count;
                } else {
                    candidate = None;
                    candidate_count = 0;
                }

                if is_left_child_of(node, &np) {
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

        // Walk up from bubble_up, rebalancing and adjusting child counts.
        let mut bubbling_finished = false;
        let mut bu = bubble_up;
        while let Some(cur) = bu {
            let mut active = cur.clone();
            if !bubbling_finished {
                let left_depth = child_depth(&active.borrow().left_child_node);
                let right_depth = child_depth(&active.borrow().right_child_node);
                let new_depth = max(left_depth, right_depth);
                let depth_changed = new_depth != active.borrow().depth;
                active.borrow_mut().depth = new_depth;

                let balance = node_balance(&active);
                if balance < -1 {
                    let lc = active.borrow().left_child_node.clone().unwrap();
                    if node_balance(&lc) > 0 {
                        rotate_left(self, &lc);
                    }
                    active = rotate_right(self, &active);
                } else if balance > 1 {
                    let rc = active.borrow().right_child_node.clone().unwrap();
                    if node_balance(&rc) < 0 {
                        rotate_right(self, &rc);
                    }
                    active = rotate_left(self, &active);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Adjust the parent's child count for the side where `active` lives.
            let parent = parent_of(&active);
            if let Some(ref p) = parent {
                if is_left_child_of(&active, p) {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bu = parent;
        }

        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach the removed node from its old structure to avoid stale pointers
        // and keep the API tidy.
        {
            let mut n = node.borrow_mut();
            n.parent_node = None;
            n.left_child_node = None;
            n.right_child_node = None;
            n.left_child_count = 0;
            n.right_child_count = 0;
            n.depth = 0;
        }
        let _ = self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let new_count;
        let valid;
        {
            let mut n = node.borrow_mut();
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
            new_count = n.weak_ref_count;
            valid = n.weak_ref_node_valid;
        }
        if new_count == 0 {
            if let Some(ff) = self.free_function {
                ff(node);
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
        while let Some(n) = current {
            let cmp = (self.cmp_function)(key, &n.borrow().key);
            if cmp == 0 {
                return Some(n);
            } else if cmp < 0 {
                let next = n.borrow().left_child_node.clone();
                current = next;
            } else {
                let next = n.borrow().right_child_node.clone();
                current = next;
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        let mut idx = index;
        while let Some(n) = current {
            let lcc = n.borrow().left_child_count;
            if lcc <= idx {
                idx -= lcc;
                if idx == 0 {
                    return Some(n);
                }
                idx -= 1;
                let next = n.borrow().right_child_node.clone();
                current = next;
            } else {
                let next = n.borrow().left_child_node.clone();
                current = next;
            }
        }
        None
    }
    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        fn helper(node: &Rc<RefCell<BOSNode>>) {
            let n = node.borrow();
            println!(
                "  {} [label=\"\\N ({},{},{})\"];",
                n.key, n.left_child_count, n.right_child_count, n.depth
            );
            if let Some(ref pw) = n.parent_node {
                if let Some(p) = pw.upgrade() {
                    println!("  {} -> {} [color=green];", n.key, p.borrow().key);
                }
            }
            if let Some(ref l) = n.left_child_node {
                println!("  {} -> {}", n.key, l.borrow().key);
                helper(l);
            }
            if let Some(ref r) = n.right_child_node {
                println!("  {} -> {}", n.key, r.borrow().key);
                helper(r);
            }
        }

        if let Some(ref root) = self.root_node {
            println!("digraph {{\n  ordering = out;");
            helper(root);
            println!("}}");
        }
    }
}
/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    {
        let mut n = node.borrow_mut();
        assert!(n.weak_ref_count < 127);
        assert!(n.weak_ref_count > 0);
        n.weak_ref_count += 1;
    }
    node.clone()
}
/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    // If there is a right subtree, return its leftmost node.
    let right = node.borrow().right_child_node.clone();
    if let Some(mut n) = right {
        loop {
            let next = n.borrow().left_child_node.clone();
            match next {
                Some(l) => n = l,
                None => return Some(n),
            }
        }
    }
    // Otherwise walk up until we are the left child of some ancestor.
    let mut current = node.clone();
    loop {
        let parent = parent_of(&current);
        match parent {
            None => return None,
            Some(p) => {
                let we_are_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if !we_are_right {
                    return Some(p);
                }
                current = p;
            }
        }
    }
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    let left = node.borrow().left_child_node.clone();
    if let Some(mut n) = left {
        loop {
            let next = n.borrow().right_child_node.clone();
            match next {
                Some(r) => n = r,
                None => return Some(n),
            }
        }
    }
    let mut current = node.clone();
    loop {
        let parent = parent_of(&current);
        match parent {
            None => return None,
            Some(p) => {
                let we_are_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if !we_are_left {
                    return Some(p);
                }
                current = p;
            }
        }
    }
}
/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut current = Some(node.clone());
    while let Some(n) = current {
        let parent = parent_of(&n);
        if let Some(ref p) = parent {
            let is_right = p
                .borrow()
                .right_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, &n));
            if is_right {
                counter += 1 + p.borrow().left_child_count;
            }
        }
        current = parent;
    }
    counter
}
