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

// ===== Helper functions =====

/// Get the parent node (upgraded from Weak reference).
fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

/// Compute the AVL balance factor: right_depth - left_depth.
fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
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

/// Recompute and store the depth of `node`.
fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let new_depth = {
        let n = node.borrow();
        let l = n
            .left_child_node
            .as_ref()
            .map_or(0u32, |c| c.borrow().depth + 1);
        let r = n
            .right_child_node
            .as_ref()
            .map_or(0u32, |c| c.borrow().depth + 1);
        l.max(r)
    };
    node.borrow_mut().depth = new_depth;
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
        // Find tree position to insert new node.
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;

        if let Some(root) = self.root_node.clone() {
            let mut current = root;
            loop {
                let cmp = (self.cmp_function)(&key, &current.borrow().key);
                let next = if cmp < 0 {
                    current.borrow_mut().left_child_count += 1;
                    go_left = true;
                    current.borrow().left_child_node.clone()
                } else {
                    current.borrow_mut().right_child_count += 1;
                    go_left = false;
                    current.borrow().right_child_node.clone()
                };
                match next {
                    Some(n) => current = n,
                    None => {
                        parent_node = Some(current);
                        break;
                    }
                }
            }
        }

        // Create the new node.
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

        // Attach to parent or set as root.
        match parent_node.as_ref() {
            Some(p) => {
                if go_left {
                    p.borrow_mut().left_child_node = Some(new_node.clone());
                } else {
                    p.borrow_mut().right_child_node = Some(new_node.clone());
                }
            }
            None => {
                self.root_node = Some(new_node.clone());
                return new_node;
            }
        }

        let parent = parent_node.unwrap();

        // Check if the depth changed for the parent: only if this is the first
        // child of the parent.
        let (has_left, has_right) = {
            let p = parent.borrow();
            (p.left_child_node.is_some(), p.right_child_node.is_some())
        };

        if has_left ^ has_right {
            // Bubble depth changes up.
            parent.borrow_mut().depth += 1;
            let mut current = parent;
            while let Some(p_node) = parent_of(&current) {
                current = p_node;

                let (new_left_depth, new_right_depth) = {
                    let c = current.borrow();
                    let l = c
                        .left_child_node
                        .as_ref()
                        .map_or(0u32, |n| n.borrow().depth + 1);
                    let r = c
                        .right_child_node
                        .as_ref()
                        .map_or(0u32, |n| n.borrow().depth + 1);
                    (l, r)
                };
                let max_depth = new_left_depth.max(new_right_depth);

                let cur_depth = current.borrow().depth;
                if cur_depth != max_depth {
                    current.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                // Check AVL property violations.
                if new_left_depth == new_right_depth + 2 {
                    // Left-right case.
                    let left_child = current.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        self.rotate_left(&left_child);
                    }
                    current = self.rotate_right(&current);
                } else if new_right_depth == new_left_depth + 2 {
                    // Right-left case.
                    let right_child = current.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        self.rotate_right(&right_child);
                    }
                    current = self.rotate_left(&current);
                }
            }
        }

        new_node
    }
    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>> = None;

        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        if has_left && has_right {
            // Node has children on both sides — find a replacement candidate.
            let candidate: Rc<RefCell<BOSNode>>;
            let lost_child: Option<Rc<RefCell<BOSNode>>>;

            let left_depth = node
                .borrow()
                .left_child_node
                .as_ref()
                .map(|n| n.borrow().depth)
                .unwrap_or(0);
            let right_depth = node
                .borrow()
                .right_child_node
                .as_ref()
                .map(|n| n.borrow().depth)
                .unwrap_or(0);

            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut cand = node.borrow().left_child_node.clone().unwrap();
                while cand.borrow().right_child_node.is_some() {
                    cand.borrow_mut().right_child_count -= 1;
                    let next = cand.borrow().right_child_node.clone().unwrap();
                    cand = next;
                }
                lost_child = cand.borrow().left_child_node.clone();
                candidate = cand;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut cand = node.borrow().right_child_node.clone().unwrap();
                while cand.borrow().left_child_node.is_some() {
                    cand.borrow_mut().left_child_count -= 1;
                    let next = cand.borrow().left_child_node.clone().unwrap();
                    cand = next;
                }
                lost_child = cand.borrow().right_child_node.clone();
                candidate = cand;
            }

            // bubble_start is the parent of candidate (still in the tree at this point).
            let bubble_start = parent_of(&candidate).unwrap();

            // Remove candidate from bubble_start by replacing with lost_child.
            {
                let is_left = bubble_start
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &candidate));
                if is_left {
                    bubble_start.borrow_mut().left_child_node = lost_child.clone();
                } else {
                    bubble_start.borrow_mut().right_child_node = lost_child.clone();
                }
            }
            if let Some(lc) = lost_child.as_ref() {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was.
            let node_parent_weak = node.borrow().parent_node.clone();
            match node_parent_weak.as_ref().and_then(|w| w.upgrade()) {
                Some(np) => {
                    let is_left = np
                        .borrow()
                        .left_child_node
                        .as_ref()
                        .map_or(false, |c| Rc::ptr_eq(c, node));
                    if is_left {
                        np.borrow_mut().left_child_node = Some(candidate.clone());
                    } else {
                        np.borrow_mut().right_child_node = Some(candidate.clone());
                    }
                }
                None => {
                    self.root_node = Some(candidate.clone());
                }
            }
            candidate.borrow_mut().parent_node = node_parent_weak;

            // Take node's children & counts and assign to candidate.
            let n_left = node.borrow().left_child_node.clone();
            let n_left_count = node.borrow().left_child_count;
            let n_right = node.borrow().right_child_node.clone();
            let n_right_count = node.borrow().right_child_count;

            candidate.borrow_mut().left_child_node = n_left.clone();
            candidate.borrow_mut().left_child_count = n_left_count;
            candidate.borrow_mut().right_child_node = n_right.clone();
            candidate.borrow_mut().right_child_count = n_right_count;

            if let Some(l) = n_left.as_ref() {
                l.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(r) = n_right.as_ref() {
                r.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut current = bubble_start;
                while !Rc::ptr_eq(&current, &candidate) {
                    update_depth(&current);
                    let bal = balance(&current);
                    if bal > 1 {
                        let right_child = current.borrow().right_child_node.clone().unwrap();
                        if balance(&right_child) < 0 {
                            self.rotate_right(&right_child);
                        }
                        current = self.rotate_left(&current);
                    } else if bal < -1 {
                        let left_child = current.borrow().left_child_node.clone().unwrap();
                        if balance(&left_child) > 0 {
                            self.rotate_left(&left_child);
                        }
                        current = self.rotate_right(&current);
                    }
                    let next = parent_of(&current);
                    match next {
                        Some(n) => current = n,
                        None => break,
                    }
                }
            }

            // Fixup candidate's depth.
            update_depth(&candidate);

            bubble_up = parent_of(&candidate);
            if let Some(bu) = bubble_up.as_ref() {
                let is_left = bu
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &candidate));
                if is_left {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Node has zero or one child.
            let node_parent_weak = node.borrow().parent_node.clone();
            let parent_opt = node_parent_weak.as_ref().and_then(|w| w.upgrade());

            if parent_opt.is_none() {
                // node was the root.
                let n_left = node.borrow().left_child_node.clone();
                let n_right = node.borrow().right_child_node.clone();
                if n_left.is_some() {
                    self.root_node = n_left.clone();
                    if let Some(l) = n_left.as_ref() {
                        l.borrow_mut().parent_node = None;
                    }
                } else {
                    self.root_node = n_right.clone();
                    if let Some(r) = n_right.as_ref() {
                        r.borrow_mut().parent_node = None;
                    }
                }
                bubble_up = None;
            } else {
                let parent = parent_opt.unwrap();
                let candidate: Option<Rc<RefCell<BOSNode>>>;
                let candidate_count: u32;
                if let Some(r) = node.borrow().right_child_node.clone() {
                    candidate_count = node.borrow().right_child_count;
                    candidate = Some(r);
                } else {
                    candidate = node.borrow().left_child_node.clone();
                    candidate_count = node.borrow().left_child_count;
                }

                let is_left = parent
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, node));
                if is_left {
                    parent.borrow_mut().left_child_node = candidate.clone();
                    parent.borrow_mut().left_child_count = candidate_count;
                } else {
                    parent.borrow_mut().right_child_node = candidate.clone();
                    parent.borrow_mut().right_child_count = candidate_count;
                }

                if let Some(c) = candidate.as_ref() {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&parent));
                }

                bubble_up = Some(parent);
            }
        }

        // Bubble up rebalancing.
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up.clone() {
            if !bubbling_finished {
                let (left_depth, right_depth) = {
                    let n = bu.borrow();
                    let l = n
                        .left_child_node
                        .as_ref()
                        .map_or(0u32, |c| c.borrow().depth + 1);
                    let r = n
                        .right_child_node
                        .as_ref()
                        .map_or(0u32, |c| c.borrow().depth + 1);
                    (l, r)
                };
                let new_depth = left_depth.max(right_depth);
                let depth_changed = new_depth != bu.borrow().depth;
                bu.borrow_mut().depth = new_depth;

                let bal = balance(&bu);
                let mut current = bu.clone();
                if bal < -1 {
                    let left_child = current.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        self.rotate_left(&left_child);
                    }
                    current = self.rotate_right(&current);
                } else if bal > 1 {
                    let right_child = current.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        self.rotate_right(&right_child);
                    }
                    current = self.rotate_left(&current);
                } else if !depth_changed {
                    bubbling_finished = true;
                }

                // Now propagate up from `current` (the new node at this position).
                let parent_opt = parent_of(&current);
                if let Some(p) = parent_opt.as_ref() {
                    let is_left = p
                        .borrow()
                        .left_child_node
                        .as_ref()
                        .map_or(false, |c| Rc::ptr_eq(c, &current));
                    if is_left {
                        p.borrow_mut().left_child_count -= 1;
                    } else {
                        p.borrow_mut().right_child_count -= 1;
                    }
                }
                bubble_up = parent_opt;
            } else {
                let parent_opt = parent_of(&bu);
                if let Some(p) = parent_opt.as_ref() {
                    let is_left = p
                        .borrow()
                        .left_child_node
                        .as_ref()
                        .map_or(false, |c| Rc::ptr_eq(c, &bu));
                    if is_left {
                        p.borrow_mut().left_child_count -= 1;
                    } else {
                        p.borrow_mut().right_child_count -= 1;
                    }
                }
                bubble_up = parent_opt;
            }
        }

        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach node from tree references to break any lingering links.
        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
        node.borrow_mut().parent_node = None;
        self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let (new_count, valid) = {
            let mut n = node.borrow_mut();
            n.weak_ref_count -= 1;
            (n.weak_ref_count, n.weak_ref_node_valid)
        };
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
        let mut node = self.root_node.clone();
        while let Some(n) = node {
            let cmp = (self.cmp_function)(key, &n.borrow().key);
            if cmp == 0 {
                return Some(n);
            } else if cmp < 0 {
                let next = n.borrow().left_child_node.clone();
                node = next;
            } else {
                let next = n.borrow().right_child_node.clone();
                node = next;
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, mut index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut node = self.root_node.clone();
        while let Some(n) = node {
            let lcc = n.borrow().left_child_count;
            if lcc <= index {
                index -= lcc;
                if index == 0 {
                    return Some(n);
                }
                index -= 1;
                let next = n.borrow().right_child_node.clone();
                node = next;
            } else {
                let next = n.borrow().left_child_node.clone();
                node = next;
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

    // ===== Internal rotation helpers =====
    fn rotate_right(&mut self, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let l = p
            .borrow()
            .left_child_node
            .clone()
            .expect("rotate_right requires left child");

        let p_parent_weak = p.borrow().parent_node.clone();

        // Update p's parent's link to l (or root).
        match p_parent_weak.as_ref().and_then(|w| w.upgrade()) {
            Some(parent) => {
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
            }
            None => {
                self.root_node = Some(l.clone());
            }
        }

        // l->parent_node = p->parent_node
        l.borrow_mut().parent_node = p_parent_weak;

        // p->left_child_node = l->right_child_node; p->left_child_count = l->right_child_count
        let l_right = l.borrow().right_child_node.clone();
        let l_right_count = l.borrow().right_child_count;
        p.borrow_mut().left_child_node = l_right.clone();
        p.borrow_mut().left_child_count = l_right_count;
        if let Some(lr) = l_right.as_ref() {
            lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
        }

        // Update p's depth.
        update_depth(p);

        // l->right_child_node = p; p->parent_node = l
        l.borrow_mut().right_child_node = Some(p.clone());
        p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

        // l->right_child_count = p->left_child_count + p->right_child_count + 1
        let p_lc = p.borrow().left_child_count;
        let p_rc = p.borrow().right_child_count;
        l.borrow_mut().right_child_count = p_lc + p_rc + 1;

        // Update l's depth.
        update_depth(&l);

        l
    }

    fn rotate_left(&mut self, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let r = p
            .borrow()
            .right_child_node
            .clone()
            .expect("rotate_left requires right child");

        let p_parent_weak = p.borrow().parent_node.clone();

        match p_parent_weak.as_ref().and_then(|w| w.upgrade()) {
            Some(parent) => {
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
            }
            None => {
                self.root_node = Some(r.clone());
            }
        }

        r.borrow_mut().parent_node = p_parent_weak;

        let r_left = r.borrow().left_child_node.clone();
        let r_left_count = r.borrow().left_child_count;
        p.borrow_mut().right_child_node = r_left.clone();
        p.borrow_mut().right_child_count = r_left_count;
        if let Some(rl) = r_left.as_ref() {
            rl.borrow_mut().parent_node = Some(Rc::downgrade(p));
        }

        update_depth(p);

        r.borrow_mut().left_child_node = Some(p.clone());
        p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

        let p_lc = p.borrow().left_child_count;
        let p_rc = p.borrow().right_child_count;
        r.borrow_mut().left_child_count = p_lc + p_rc + 1;

        update_depth(&r);

        r
    }
}

#[cfg(debug_assertions)]
fn print_helper(node: &Rc<RefCell<BOSNode>>) {
    let n = node.borrow();
    println!(
        "  {} [label=\"\\N ({},{},{})\"];",
        n.key, n.left_child_count, n.right_child_count, n.depth
    );
    if let Some(p) = n.parent_node.as_ref().and_then(|w| w.upgrade()) {
        println!("  {} -> {} [color=green];", n.key, p.borrow().key);
    }
    if let Some(l) = n.left_child_node.as_ref() {
        println!("  {} -> {}", n.key, l.borrow().key);
        print_helper(l);
    }
    if let Some(r) = n.right_child_node.as_ref() {
        println!("  {} -> {}", n.key, r.borrow().key);
        print_helper(r);
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
    if let Some(right) = node.borrow().right_child_node.clone() {
        let mut current = right;
        loop {
            let left = current.borrow().left_child_node.clone();
            match left {
                Some(l) => current = l,
                None => return Some(current),
            }
        }
    }
    let mut current = node.clone();
    loop {
        let parent = parent_of(&current);
        match parent {
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_right {
                    current = p;
                } else {
                    return Some(p);
                }
            }
            None => return None,
        }
    }
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(left) = node.borrow().left_child_node.clone() {
        let mut current = left;
        loop {
            let right = current.borrow().right_child_node.clone();
            match right {
                Some(r) => current = r,
                None => return Some(current),
            }
        }
    }
    let mut current = node.clone();
    loop {
        let parent = parent_of(&current);
        match parent {
            Some(p) => {
                let is_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_left {
                    current = p;
                } else {
                    return Some(p);
                }
            }
            None => return None,
        }
    }
}
/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut current = Some(node.clone());
    while let Some(c) = current {
        let parent = parent_of(&c);
        if let Some(p) = parent.as_ref() {
            let is_right = p
                .borrow()
                .right_child_node
                .as_ref()
                .map_or(false, |r| Rc::ptr_eq(r, &c));
            if is_right {
                counter += 1 + p.borrow().left_child_count;
            }
        }
        current = parent;
    }
    counter
}
