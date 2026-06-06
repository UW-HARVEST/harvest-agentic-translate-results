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

// ===== Helpers =====

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn is_left_child(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |c| Rc::ptr_eq(c, child))
}

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

fn compute_depth(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let n = node.borrow();
    let left_depth = n
        .left_child_node
        .as_ref()
        .map_or(0u32, |c| c.borrow().depth + 1);
    let right_depth = n
        .right_child_node
        .as_ref()
        .map_or(0u32, |c| c.borrow().depth + 1);
    left_depth.max(right_depth)
}

fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let new_depth = compute_depth(node);
    node.borrow_mut().depth = new_depth;
}

/// Rotate right around `p`. Returns the new subtree root (`L`, the old left child of `p`).
fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires left child");

    // Re-parent: P's parent now points to L instead of P.
    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());

    if let Some(pp) = &p_parent {
        if is_left_child(pp, p) {
            pp.borrow_mut().left_child_node = Some(l.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    l.borrow_mut().parent_node = p_parent_weak;

    // Move L's right child to be P's left child.
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;

    {
        let mut p_mut = p.borrow_mut();
        p_mut.left_child_node = l_right.clone();
        p_mut.left_child_count = l_right_count;
    }

    if let Some(lr) = &l_right {
        lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // Update P's depth, then make P the right child of L.
    update_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    l.borrow_mut().right_child_node = Some(p.clone());

    let new_l_right_count = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    l.borrow_mut().right_child_count = new_l_right_count;

    update_depth(&l);

    l
}

/// Rotate left around `p`. Returns the new subtree root (`R`, the old right child of `p`).
fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires right child");

    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());

    if let Some(pp) = &p_parent {
        if is_left_child(pp, p) {
            pp.borrow_mut().left_child_node = Some(r.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent_weak;

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;

    {
        let mut p_mut = p.borrow_mut();
        p_mut.right_child_node = r_left.clone();
        p_mut.right_child_count = r_left_count;
    }

    if let Some(rl) = &r_left {
        rl.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    update_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    r.borrow_mut().left_child_node = Some(p.clone());

    let new_r_left_count = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    r.borrow_mut().left_child_count = new_r_left_count;

    update_depth(&r);

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
            None => 0,
            Some(root) => {
                let r = root.borrow();
                r.left_child_count + r.right_child_count + 1
            }
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
            key: key.clone(),
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        if self.root_node.is_none() {
            self.root_node = Some(new_node.clone());
            return new_node;
        }

        // Walk down to find the correct position. Update child counts on the way.
        let mut current = self.root_node.clone().unwrap();
        let parent_node;
        let was_left_child;
        loop {
            let cmp = (self.cmp_function)(&key, &current.borrow().key);
            if cmp < 0 {
                current.borrow_mut().left_child_count += 1;
                let next = current.borrow().left_child_node.clone();
                match next {
                    Some(n) => current = n,
                    None => {
                        parent_node = current;
                        was_left_child = true;
                        break;
                    }
                }
            } else {
                current.borrow_mut().right_child_count += 1;
                let next = current.borrow().right_child_node.clone();
                match next {
                    Some(n) => current = n,
                    None => {
                        parent_node = current;
                        was_left_child = false;
                        break;
                    }
                }
            }
        }

        // Attach new node.
        new_node.borrow_mut().parent_node = Some(Rc::downgrade(&parent_node));
        if was_left_child {
            parent_node.borrow_mut().left_child_node = Some(new_node.clone());
        } else {
            parent_node.borrow_mut().right_child_node = Some(new_node.clone());
        }

        // Determine if the parent's depth changed (only changes if the new node
        // is the parent's first child).
        let parent_first_child = {
            let p = parent_node.borrow();
            p.left_child_node.is_some() ^ p.right_child_node.is_some()
        };

        if parent_first_child {
            parent_node.borrow_mut().depth = 1;

            // Walk up the tree, updating depths and rebalancing.
            let mut current_parent_opt = parent_of(&parent_node);
            while let Some(current_parent) = current_parent_opt {
                let new_left_depth = current_parent
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |c| c.borrow().depth + 1);
                let new_right_depth = current_parent
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |c| c.borrow().depth + 1);
                let max_depth = new_left_depth.max(new_right_depth);

                let prev_depth = current_parent.borrow().depth;
                if prev_depth != max_depth {
                    current_parent.borrow_mut().depth = max_depth;
                } else {
                    // Depth didn't change here; won't change further up.
                    break;
                }

                let mut adjusted_node = current_parent.clone();
                let l = new_left_depth as i32;
                let r = new_right_depth as i32;
                if l - 2 == r {
                    // Left way deeper: handle left-right case first.
                    let left = current_parent.borrow().left_child_node.clone().unwrap();
                    if balance(&left) > 0 {
                        rotate_left(self, &left);
                    }
                    adjusted_node = rotate_right(self, &current_parent);
                } else if l + 2 == r {
                    // Right way deeper.
                    let right = current_parent.borrow().right_child_node.clone().unwrap();
                    if balance(&right) < 0 {
                        rotate_right(self, &right);
                    }
                    adjusted_node = rotate_left(self, &current_parent);
                }

                current_parent_opt = parent_of(&adjusted_node);
            }
        }

        new_node
    }

    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let bubble_up: Option<Rc<RefCell<BOSNode>>>;

        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        if has_left && has_right {
            // Both children present: find a replacement candidate (in-order
            // predecessor or successor based on which subtree is deeper) and
            // splice it in place of the node being removed.
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
                // Use predecessor (rightmost of left subtree).
                node.borrow_mut().left_child_count -= 1;
                let mut c = node.borrow().left_child_node.clone().unwrap();
                loop {
                    let next = c.borrow().right_child_node.clone();
                    match next {
                        Some(n) => {
                            c.borrow_mut().right_child_count -= 1;
                            c = n;
                        }
                        None => break,
                    }
                }
                lost_child = c.borrow().left_child_node.clone();
                candidate = c;
            } else {
                // Use successor (leftmost of right subtree).
                node.borrow_mut().right_child_count -= 1;
                let mut c = node.borrow().right_child_node.clone().unwrap();
                loop {
                    let next = c.borrow().left_child_node.clone();
                    match next {
                        Some(n) => {
                            c.borrow_mut().left_child_count -= 1;
                            c = n;
                        }
                        None => break,
                    }
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            let bubble_start = parent_of(&candidate).expect("candidate must have a parent");

            // Detach the candidate from bubble_start; lost_child takes its place.
            if is_left_child(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(lc) = &lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node used to be.
            let node_parent_weak = node.borrow().parent_node.clone();
            let node_parent = node_parent_weak.as_ref().and_then(|w| w.upgrade());
            if let Some(np) = &node_parent {
                if is_left_child(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent_weak;

            // Inherit node's children.
            let node_left = node.borrow().left_child_node.clone();
            let node_left_count = node.borrow().left_child_count;
            let node_right = node.borrow().right_child_node.clone();
            let node_right_count = node.borrow().right_child_count;

            {
                let mut c = candidate.borrow_mut();
                c.left_child_node = node_left.clone();
                c.left_child_count = node_left_count;
                c.right_child_node = node_right.clone();
                c.right_child_count = node_right_count;
            }

            if let Some(lc) = &node_left {
                if !Rc::ptr_eq(lc, &candidate) {
                    lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }
            if let Some(rc) = &node_right {
                if !Rc::ptr_eq(rc, &candidate) {
                    rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }

            // Rebalance from bubble_start up to (but not including) candidate.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start.clone();
                while !Rc::ptr_eq(&bs, &candidate) {
                    update_depth(&bs);
                    let bal = balance(&bs);
                    if bal > 1 {
                        let right = bs.borrow().right_child_node.clone().unwrap();
                        if balance(&right) < 0 {
                            rotate_right(self, &right);
                        }
                        bs = rotate_left(self, &bs);
                    } else if bal < -1 {
                        let left = bs.borrow().left_child_node.clone().unwrap();
                        if balance(&left) > 0 {
                            rotate_left(self, &left);
                        }
                        bs = rotate_right(self, &bs);
                    }
                    let parent = parent_of(&bs);
                    match parent {
                        Some(p) => bs = p,
                        None => break,
                    }
                }
            }

            update_depth(&candidate);

            bubble_up = parent_of(&candidate);

            if let Some(bu) = &bubble_up {
                if is_left_child(bu, &candidate) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // 0 or 1 children: simpler case.
            let parent = parent_of(node);
            if parent.is_none() {
                // Node was the root.
                let left = node.borrow().left_child_node.clone();
                let right = node.borrow().right_child_node.clone();
                if let Some(l) = left {
                    self.root_node = Some(l.clone());
                    l.borrow_mut().parent_node = None;
                } else if let Some(r) = right {
                    self.root_node = Some(r.clone());
                    r.borrow_mut().parent_node = None;
                } else {
                    self.root_node = None;
                }
                bubble_up = None;
            } else {
                let parent = parent.unwrap();
                let candidate;
                let candidate_count;
                let right = node.borrow().right_child_node.clone();
                if right.is_some() {
                    candidate = right;
                    candidate_count = node.borrow().right_child_count;
                } else {
                    candidate = node.borrow().left_child_node.clone();
                    candidate_count = node.borrow().left_child_count;
                }

                if is_left_child(&parent, node) {
                    let mut p = parent.borrow_mut();
                    p.left_child_node = candidate.clone();
                    p.left_child_count = candidate_count;
                } else {
                    let mut p = parent.borrow_mut();
                    p.right_child_node = candidate.clone();
                    p.right_child_count = candidate_count;
                }

                if let Some(c) = &candidate {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&parent));
                }

                bubble_up = Some(parent);
            }
        }

        // Bubble up: rebalance and decrement child counts up to the root.
        let mut bubbling_finished = false;
        let mut current_bubble = bubble_up;
        while let Some(bu) = current_bubble {
            let new_bu = if !bubbling_finished {
                let new_depth = compute_depth(&bu);
                let depth_changed = new_depth != bu.borrow().depth;
                bu.borrow_mut().depth = new_depth;

                let bal = balance(&bu);
                let result;
                if bal < -1 {
                    let left = bu.borrow().left_child_node.clone().unwrap();
                    if balance(&left) > 0 {
                        rotate_left(self, &left);
                    }
                    result = rotate_right(self, &bu);
                } else if bal > 1 {
                    let right = bu.borrow().right_child_node.clone().unwrap();
                    if balance(&right) < 0 {
                        rotate_right(self, &right);
                    }
                    result = rotate_left(self, &bu);
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    result = bu;
                }
                result
            } else {
                bu
            };

            // Update parent's child count.
            let parent = parent_of(&new_bu);
            if let Some(p) = &parent {
                if is_left_child(p, &new_bu) {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            current_bubble = parent;
        }

        // Mark node as removed (no longer in tree) and decrement its weak ref.
        node.borrow_mut().weak_ref_node_valid = 0;
        // Also detach from tree-internal pointers (cosmetic).
        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
        node.borrow_mut().parent_node = None;
        let _ = self.bostree_node_weak_unref(node);
    }

    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let count;
        {
            let mut n = node.borrow_mut();
            if n.weak_ref_count > 0 {
                n.weak_ref_count -= 1;
            }
            count = n.weak_ref_count;
        }
        if count == 0 {
            if let Some(free_fn) = self.free_function {
                free_fn(node);
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
        let mut node_opt = self.root_node.clone();
        while let Some(n) = node_opt {
            let cmp = (self.cmp_function)(key, &n.borrow().key);
            if cmp == 0 {
                return Some(n);
            }
            let next = if cmp < 0 {
                n.borrow().left_child_node.clone()
            } else {
                n.borrow().right_child_node.clone()
            };
            node_opt = next;
        }
        None
    }

    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut idx = index;
        let mut node_opt = self.root_node.clone();
        while let Some(n) = node_opt {
            let lcc = n.borrow().left_child_count;
            if lcc <= idx {
                idx -= lcc;
                if idx == 0 {
                    return Some(n);
                }
                idx -= 1;
                node_opt = n.borrow().right_child_node.clone();
            } else {
                node_opt = n.borrow().left_child_node.clone();
            }
        }
        None
    }

    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        if let Some(root) = &self.root_node {
            println!("digraph {{\n  ordering = out;");
            print_helper(root);
            println!("}}");
        }
    }
}

#[cfg(debug_assertions)]
fn print_helper(node: &Rc<RefCell<BOSNode>>) {
    let (key, lcc, rcc, depth, left, right, parent) = {
        let n = node.borrow();
        (
            n.key.clone(),
            n.left_child_count,
            n.right_child_count,
            n.depth,
            n.left_child_node.clone(),
            n.right_child_node.clone(),
            n.parent_node.as_ref().and_then(|w| w.upgrade()),
        )
    };
    println!("  {} [label=\"\\N ({},{},{})\"];", key, lcc, rcc, depth);
    if let Some(p) = parent {
        println!("  {} -> {} [color=green];", key, p.borrow().key);
    }
    if let Some(l) = left {
        println!("  {} -> {}", key, l.borrow().key);
        print_helper(&l);
    }
    if let Some(r) = right {
        println!("  {} -> {}", key, r.borrow().key);
        print_helper(&r);
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
    let right = node.borrow().right_child_node.clone();
    if let Some(r) = right {
        let mut n = r;
        loop {
            let next = n.borrow().left_child_node.clone();
            match next {
                Some(l) => n = l,
                None => return Some(n),
            }
        }
    }
    // No right subtree: walk up while we are a right child.
    let mut current = node.clone();
    loop {
        let parent = parent_of(&current);
        match parent {
            None => return None,
            Some(p) => {
                let was_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if !was_right {
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
    if let Some(l) = left {
        let mut n = l;
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
                let was_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if !was_left {
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
    let mut current_opt = Some(node.clone());
    while let Some(n) = current_opt {
        let parent = parent_of(&n);
        if let Some(p) = &parent {
            let was_right = p
                .borrow()
                .right_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, &n));
            if was_right {
                counter += 1 + p.borrow().left_child_count;
            }
        }
        current_opt = parent;
    }
    counter
}
