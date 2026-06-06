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

// ---------------- Helper Functions ----------------

fn get_parent(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn set_parent(node: &Rc<RefCell<BOSNode>>, parent: Option<&Rc<RefCell<BOSNode>>>) {
    node.borrow_mut().parent_node = parent.map(Rc::downgrade);
}

fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let left_depth = n
        .left_child_node
        .as_ref()
        .map_or(0i32, |l| l.borrow().depth as i32 + 1);
    let right_depth = n
        .right_child_node
        .as_ref()
        .map_or(0i32, |r| r.borrow().depth as i32 + 1);
    right_depth - left_depth
}

fn compute_depth(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let n = node.borrow();
    let left_depth = n
        .left_child_node
        .as_ref()
        .map_or(0u32, |l| l.borrow().depth + 1);
    let right_depth = n
        .right_child_node
        .as_ref()
        .map_or(0u32, |r| r.borrow().depth + 1);
    std::cmp::max(left_depth, right_depth)
}

fn is_left_child(node: &Rc<RefCell<BOSNode>>, parent: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |c| Rc::ptr_eq(c, node))
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
            Some(r) => {
                let n = r.borrow();
                n.left_child_count + n.right_child_count + 1
            }
            None => 0,
        }
    }

    fn rotate_right(&mut self, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let l = p.borrow().left_child_node.clone().expect("rotate_right requires a left child");
        let p_parent = get_parent(p);

        // Update parent's child pointer (or root)
        if let Some(ref pp) = p_parent {
            if is_left_child(p, pp) {
                pp.borrow_mut().left_child_node = Some(l.clone());
            } else {
                pp.borrow_mut().right_child_node = Some(l.clone());
            }
        } else {
            self.root_node = Some(l.clone());
        }

        // L's parent = P's old parent
        set_parent(&l, p_parent.as_ref());

        // P's left = L's right
        let l_right = l.borrow().right_child_node.clone();
        let l_right_count = l.borrow().right_child_count;
        p.borrow_mut().left_child_node = l_right.clone();
        p.borrow_mut().left_child_count = l_right_count;
        if let Some(ref lr) = l_right {
            set_parent(lr, Some(p));
        }

        // P's depth
        let p_depth = compute_depth(p);
        p.borrow_mut().depth = p_depth;

        // L's right = P, P's parent = L
        l.borrow_mut().right_child_node = Some(p.clone());
        set_parent(p, Some(&l));

        // L's right_child_count = P's left + P's right + 1
        let new_count = {
            let pb = p.borrow();
            pb.left_child_count + pb.right_child_count + 1
        };
        l.borrow_mut().right_child_count = new_count;

        // L's depth
        let l_depth = compute_depth(&l);
        l.borrow_mut().depth = l_depth;

        l
    }

    fn rotate_left(&mut self, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
        let r = p
            .borrow()
            .right_child_node
            .clone()
            .expect("rotate_left requires a right child");
        let p_parent = get_parent(p);

        if let Some(ref pp) = p_parent {
            if is_left_child(p, pp) {
                pp.borrow_mut().left_child_node = Some(r.clone());
            } else {
                pp.borrow_mut().right_child_node = Some(r.clone());
            }
        } else {
            self.root_node = Some(r.clone());
        }

        set_parent(&r, p_parent.as_ref());

        let r_left = r.borrow().left_child_node.clone();
        let r_left_count = r.borrow().left_child_count;
        p.borrow_mut().right_child_node = r_left.clone();
        p.borrow_mut().right_child_count = r_left_count;
        if let Some(ref rl) = r_left {
            set_parent(rl, Some(p));
        }

        let p_depth = compute_depth(p);
        p.borrow_mut().depth = p_depth;

        r.borrow_mut().left_child_node = Some(p.clone());
        set_parent(p, Some(&r));

        let new_count = {
            let pb = p.borrow();
            pb.left_child_count + pb.right_child_count + 1
        };
        r.borrow_mut().left_child_count = new_count;

        let r_depth = compute_depth(&r);
        r.borrow_mut().depth = r_depth;

        r
    }

    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find tree position to insert new node
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;
        let mut current = self.root_node.clone();

        while let Some(node) = current {
            let next = {
                let mut nb = node.borrow_mut();
                let cmp = (self.cmp_function)(&key, &nb.key);
                if cmp < 0 {
                    nb.left_child_count += 1;
                    go_left = true;
                    nb.left_child_node.clone()
                } else {
                    nb.right_child_count += 1;
                    go_left = false;
                    nb.right_child_node.clone()
                }
            };
            parent_node = Some(node);
            current = next;
        }

        // Create new node
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

        let parent = match parent_node {
            Some(p) => p,
            None => {
                self.root_node = Some(new_node.clone());
                return new_node;
            }
        };

        // Attach to parent
        if go_left {
            parent.borrow_mut().left_child_node = Some(new_node.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(new_node.clone());
        }

        // Check if depth changed - only when this was the first child of parent
        let depth_changed = {
            let p = parent.borrow();
            p.left_child_node.is_some() != p.right_child_node.is_some()
        };

        if depth_changed {
            parent.borrow_mut().depth += 1;
            let mut p_current = parent;
            loop {
                let next = get_parent(&p_current);
                let mut p = match next {
                    Some(np) => np,
                    None => break,
                };

                let new_left_depth = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |l| l.borrow().depth + 1);
                let new_right_depth = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |r| r.borrow().depth + 1);
                let max_depth = std::cmp::max(new_left_depth, new_right_depth);

                let current_depth = p.borrow().depth;
                if current_depth != max_depth {
                    p.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                // Check AVL property
                if new_left_depth as i32 - 2 == new_right_depth as i32 {
                    // Left-right case
                    let left_child = p.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        self.rotate_left(&left_child);
                    }
                    p = self.rotate_right(&p);
                } else if new_left_depth as i32 + 2 == new_right_depth as i32 {
                    // Right-left case
                    let right_child = p.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        self.rotate_right(&right_child);
                    }
                    p = self.rotate_left(&p);
                }

                p_current = p;
            }
        }

        new_node
    }

    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>>;

        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        if has_left && has_right {
            // Two-child case
            let candidate: Rc<RefCell<BOSNode>>;
            let lost_child: Option<Rc<RefCell<BOSNode>>>;
            let left_depth = node
                .borrow()
                .left_child_node
                .as_ref()
                .unwrap()
                .borrow()
                .depth;
            let right_depth = node
                .borrow()
                .right_child_node
                .as_ref()
                .unwrap()
                .borrow()
                .depth;

            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut c = node.borrow().left_child_node.clone().unwrap();
                loop {
                    let next = c.borrow().right_child_node.clone();
                    if let Some(n) = next {
                        c.borrow_mut().right_child_count -= 1;
                        c = n;
                    } else {
                        break;
                    }
                }
                lost_child = c.borrow().left_child_node.clone();
                candidate = c;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut c = node.borrow().right_child_node.clone().unwrap();
                loop {
                    let next = c.borrow().left_child_node.clone();
                    if let Some(n) = next {
                        c.borrow_mut().left_child_count -= 1;
                        c = n;
                    } else {
                        break;
                    }
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            let bubble_start = get_parent(&candidate).expect("candidate must have a parent");

            // Replace candidate with lost_child in bubble_start
            if is_left_child(&candidate, &bubble_start) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                set_parent(lc, Some(&bubble_start));
            }

            // Anchor candidate at node's old place
            let node_parent = get_parent(node);
            if let Some(ref np) = node_parent {
                if is_left_child(node, np) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            set_parent(&candidate, node_parent.as_ref());

            // Copy node's children into candidate
            let nl = node.borrow().left_child_node.clone();
            let nlc = node.borrow().left_child_count;
            let nr = node.borrow().right_child_node.clone();
            let nrc = node.borrow().right_child_count;
            candidate.borrow_mut().left_child_node = nl.clone();
            candidate.borrow_mut().left_child_count = nlc;
            candidate.borrow_mut().right_child_node = nr.clone();
            candidate.borrow_mut().right_child_count = nrc;

            if let Some(ref l) = nl {
                set_parent(l, Some(&candidate));
            }
            if let Some(ref r) = nr {
                set_parent(r, Some(&candidate));
            }

            // Rebalance from bubble_start up to candidate
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut current = bubble_start;
                while !Rc::ptr_eq(&current, &candidate) {
                    let new_depth = compute_depth(&current);
                    current.borrow_mut().depth = new_depth;

                    let bal = balance(&current);
                    if bal > 1 {
                        let right = current.borrow().right_child_node.clone().unwrap();
                        if balance(&right) < 0 {
                            self.rotate_right(&right);
                        }
                        current = self.rotate_left(&current);
                    } else if bal < -1 {
                        let left = current.borrow().left_child_node.clone().unwrap();
                        if balance(&left) > 0 {
                            self.rotate_left(&left);
                        }
                        current = self.rotate_right(&current);
                    }

                    let next = get_parent(&current);
                    match next {
                        Some(n) => current = n,
                        None => break,
                    }
                }
            }

            // Fix candidate's depth
            let cd = compute_depth(&candidate);
            candidate.borrow_mut().depth = cd;

            bubble_up = get_parent(&candidate);

            if let Some(ref bu) = bubble_up {
                if is_left_child(&candidate, bu) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // One- or zero-child case
            let node_parent = get_parent(node);
            match node_parent {
                None => {
                    // Node was root
                    let l = node.borrow().left_child_node.clone();
                    let r = node.borrow().right_child_node.clone();
                    if let Some(left) = l {
                        self.root_node = Some(left.clone());
                        set_parent(&left, None);
                    } else if let Some(right) = r {
                        self.root_node = Some(right.clone());
                        set_parent(&right, None);
                    } else {
                        self.root_node = None;
                    }
                    bubble_up = None;
                }
                Some(np) => {
                    let candidate: Option<Rc<RefCell<BOSNode>>>;
                    let candidate_count: u32;
                    if let Some(r) = node.borrow().right_child_node.clone() {
                        candidate_count = node.borrow().right_child_count;
                        candidate = Some(r);
                    } else {
                        candidate_count = node.borrow().left_child_count;
                        candidate = node.borrow().left_child_node.clone();
                    }

                    if is_left_child(node, &np) {
                        np.borrow_mut().left_child_node = candidate.clone();
                        np.borrow_mut().left_child_count = candidate_count;
                    } else {
                        np.borrow_mut().right_child_node = candidate.clone();
                        np.borrow_mut().right_child_count = candidate_count;
                    }

                    if let Some(ref c) = candidate {
                        set_parent(c, Some(&np));
                    }
                    bubble_up = Some(np);
                }
            }
        }

        // Bubble up
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up {
            let mut current_bu = bu;
            if !bubbling_finished {
                let new_depth = compute_depth(&current_bu);
                let depth_changed = new_depth != current_bu.borrow().depth;
                current_bu.borrow_mut().depth = new_depth;

                let bal = balance(&current_bu);
                if bal < -1 {
                    let left = current_bu.borrow().left_child_node.clone().unwrap();
                    if balance(&left) > 0 {
                        self.rotate_left(&left);
                    }
                    current_bu = self.rotate_right(&current_bu);
                } else if bal > 1 {
                    let right = current_bu.borrow().right_child_node.clone().unwrap();
                    if balance(&right) < 0 {
                        self.rotate_right(&right);
                    }
                    current_bu = self.rotate_left(&current_bu);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Decrement parent's child count
            let parent = get_parent(&current_bu);
            if let Some(ref p) = parent {
                if is_left_child(&current_bu, p) {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up = parent;
        }

        // Mark node invalid and unref it
        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach the node from the tree structure to avoid stale pointers
        node.borrow_mut().parent_node = None;
        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
        self.bostree_node_weak_unref(node);
    }

    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let (count, valid) = {
            let mut n = node.borrow_mut();
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
            (n.weak_ref_count, n.weak_ref_node_valid)
        };
        if count == 0 {
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
            let next = {
                let nb = node.borrow();
                let cmp = (self.cmp_function)(key, &nb.key);
                if cmp == 0 {
                    return Some(node.clone());
                } else if cmp < 0 {
                    nb.left_child_node.clone()
                } else {
                    nb.right_child_node.clone()
                }
            };
            current = next;
        }
        None
    }

    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut idx = index;
        let mut current = self.root_node.clone();
        while let Some(node) = current {
            let next = {
                let nb = node.borrow();
                let lcc = nb.left_child_count;
                if lcc <= idx {
                    idx -= lcc;
                    if idx == 0 {
                        return Some(node.clone());
                    }
                    idx -= 1;
                    nb.right_child_node.clone()
                } else {
                    nb.left_child_node.clone()
                }
            };
            current = next;
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
    let (key, lcc, rcc, depth, parent_key, left, right) = {
        let n = node.borrow();
        let parent_key = n
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|p| p.borrow().key.clone());
        (
            n.key.clone(),
            n.left_child_count,
            n.right_child_count,
            n.depth,
            parent_key,
            n.left_child_node.clone(),
            n.right_child_node.clone(),
        )
    };
    println!("  {} [label=\"\\N ({},{},{})\"];", key, lcc, rcc, depth);
    if let Some(pk) = parent_key {
        println!("  {} -> {} [color=green];", key, pk);
    }
    if let Some(ref l) = left {
        println!("  {} -> {}", key, l.borrow().key);
        print_helper(l);
    }
    if let Some(ref r) = right {
        println!("  {} -> {}", key, r.borrow().key);
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
    // If there's a right child, descend to its leftmost.
    let right = node.borrow().right_child_node.clone();
    if let Some(mut cur) = right {
        loop {
            let next = cur.borrow().left_child_node.clone();
            match next {
                Some(n) => cur = n,
                None => return Some(cur),
            }
        }
    }

    // Else walk up until we come from the left.
    let mut cur = node.clone();
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let cur_is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &cur));
                if cur_is_right {
                    cur = p;
                } else {
                    return Some(p);
                }
            }
        }
    }
}

/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    let left = node.borrow().left_child_node.clone();
    if let Some(mut cur) = left {
        loop {
            let next = cur.borrow().right_child_node.clone();
            match next {
                Some(n) => cur = n,
                None => return Some(cur),
            }
        }
    }

    let mut cur = node.clone();
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let cur_is_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &cur));
                if cur_is_left {
                    cur = p;
                } else {
                    return Some(p);
                }
            }
        }
    }
}

/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut cur = node.clone();
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => break,
            Some(p) => {
                let cur_is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &cur));
                if cur_is_right {
                    counter += 1 + p.borrow().left_child_count;
                }
                cur = p;
            }
        }
    }
    counter
}
