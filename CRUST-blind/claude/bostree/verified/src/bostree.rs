use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[inline]
fn max(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

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

// --- Internal helpers ---

fn get_parent(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
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

fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let new_depth = {
        let n = node.borrow();
        let left = n
            .left_child_node
            .as_ref()
            .map_or(0u32, |c| c.borrow().depth + 1);
        let right = n
            .right_child_node
            .as_ref()
            .map_or(0u32, |c| c.borrow().depth + 1);
        max(left, right)
    };
    node.borrow_mut().depth = new_depth;
}

/// Rotate right around P. Returns the new top of the rotated subtree (formerly L).
fn rotate_right(
    root: &mut Option<Rc<RefCell<BOSNode>>>,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires P to have a left child");
    let p_parent = get_parent(p);

    // Replace P with L in P's parent (or in root if P was root).
    match &p_parent {
        Some(pp) => {
            let p_is_left = pp
                .borrow()
                .left_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, p));
            if p_is_left {
                pp.borrow_mut().left_child_node = Some(l.clone());
            } else {
                pp.borrow_mut().right_child_node = Some(l.clone());
            }
        }
        None => {
            *root = Some(l.clone());
        }
    }

    // L's parent <- P's old parent
    l.borrow_mut().parent_node = p_parent.as_ref().map(|pp| Rc::downgrade(pp));

    // P->left = L->right (with count)
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;

    if let Some(ref lr) = l_right {
        lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    update_depth(p);

    // L->right = P; P->parent = L
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));
    l.borrow_mut().right_child_node = Some(p.clone());

    let pl = p.borrow().left_child_count;
    let pr = p.borrow().right_child_count;
    l.borrow_mut().right_child_count = pl + pr + 1;

    update_depth(&l);

    l
}

/// Rotate left around P. Returns the new top of the rotated subtree (formerly R).
fn rotate_left(
    root: &mut Option<Rc<RefCell<BOSNode>>>,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires P to have a right child");
    let p_parent = get_parent(p);

    match &p_parent {
        Some(pp) => {
            let p_is_left = pp
                .borrow()
                .left_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, p));
            if p_is_left {
                pp.borrow_mut().left_child_node = Some(r.clone());
            } else {
                pp.borrow_mut().right_child_node = Some(r.clone());
            }
        }
        None => {
            *root = Some(r.clone());
        }
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(|pp| Rc::downgrade(pp));

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;

    if let Some(ref rl) = r_left {
        rl.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    update_depth(p);

    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));
    r.borrow_mut().left_child_node = Some(p.clone());

    let pl = p.borrow().left_child_count;
    let pr = p.borrow().right_child_count;
    r.borrow_mut().left_child_count = pl + pr + 1;

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
            Some(n) => {
                let nb = n.borrow();
                nb.left_child_count + nb.right_child_count + 1
            }
            None => 0,
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find tree position to insert new node.
        let mut current = self.root_node.clone();
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;

        while let Some(n) = current {
            parent_node = Some(n.clone());
            let cmp = (self.cmp_function)(&key, &n.borrow().key);
            if cmp < 0 {
                n.borrow_mut().left_child_count += 1;
                current = n.borrow().left_child_node.clone();
                go_left = true;
            } else {
                n.borrow_mut().right_child_count += 1;
                current = n.borrow().right_child_node.clone();
                go_left = false;
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: parent_node.as_ref().map(|p| Rc::downgrade(p)),
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        // No parent means this is the first node.
        let parent = match parent_node {
            None => {
                self.root_node = Some(new_node.clone());
                return new_node;
            }
            Some(p) => p,
        };

        if go_left {
            parent.borrow_mut().left_child_node = Some(new_node.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(new_node.clone());
        }

        // Check if the depth changed: only if the parent had no other child before.
        let parent_has_one_child = {
            let p = parent.borrow();
            p.left_child_node.is_some() ^ p.right_child_node.is_some()
        };

        if parent_has_one_child {
            parent.borrow_mut().depth += 1;
            let mut p_node = parent;
            while let Some(p_parent) = get_parent(&p_node) {
                p_node = p_parent;

                let new_left_depth = p_node
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let new_right_depth = p_node
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let max_depth = max(new_left_depth, new_right_depth);

                let cur_depth = p_node.borrow().depth;
                if cur_depth != max_depth {
                    p_node.borrow_mut().depth = max_depth;
                } else {
                    // Depth unchanged here, propagation can stop.
                    break;
                }

                let nld = new_left_depth as i64;
                let nrd = new_right_depth as i64;
                if nld - 2 == nrd {
                    // Left-right case
                    let left_child = p_node.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        rotate_left(&mut self.root_node, &left_child);
                    }
                    p_node = rotate_right(&mut self.root_node, &p_node);
                } else if nld + 2 == nrd {
                    // Right-left case
                    let right_child = p_node.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        rotate_right(&mut self.root_node, &right_child);
                    }
                    p_node = rotate_left(&mut self.root_node, &p_node);
                }
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
            // Two-children case.
            let (candidate, lost_child) = {
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
                let go_left = left_depth >= right_depth;

                if go_left {
                    node.borrow_mut().left_child_count -= 1;
                    let mut cand = node.borrow().left_child_node.clone().unwrap();
                    while cand.borrow().right_child_node.is_some() {
                        cand.borrow_mut().right_child_count -= 1;
                        let next = cand.borrow().right_child_node.clone().unwrap();
                        cand = next;
                    }
                    let lc = cand.borrow().left_child_node.clone();
                    (cand, lc)
                } else {
                    node.borrow_mut().right_child_count -= 1;
                    let mut cand = node.borrow().right_child_node.clone().unwrap();
                    while cand.borrow().left_child_node.is_some() {
                        cand.borrow_mut().left_child_count -= 1;
                        let next = cand.borrow().left_child_node.clone().unwrap();
                        cand = next;
                    }
                    let lc = cand.borrow().right_child_node.clone();
                    (cand, lc)
                }
            };

            let bubble_start = get_parent(&candidate).expect("candidate must have a parent");

            // Splice out candidate, replacing it with lost_child in bubble_start.
            let candidate_is_left = bubble_start
                .borrow()
                .left_child_node
                .as_ref()
                .map_or(false, |c| Rc::ptr_eq(c, &candidate));
            if candidate_is_left {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was.
            let node_parent = get_parent(node);
            match &node_parent {
                Some(np) => {
                    let node_is_left = np
                        .borrow()
                        .left_child_node
                        .as_ref()
                        .map_or(false, |c| Rc::ptr_eq(c, node));
                    if node_is_left {
                        np.borrow_mut().left_child_node = Some(candidate.clone());
                    } else {
                        np.borrow_mut().right_child_node = Some(candidate.clone());
                    }
                }
                None => {
                    self.root_node = Some(candidate.clone());
                }
            }
            candidate.borrow_mut().parent_node =
                node_parent.as_ref().map(|p| Rc::downgrade(p));

            // Move node's children to candidate. Note: read AFTER splice so that
            // if candidate was directly under node, the changed node->[lr]_child_node
            // is observed.
            let node_left = node.borrow().left_child_node.clone();
            let node_right = node.borrow().right_child_node.clone();
            let node_left_count = node.borrow().left_child_count;
            let node_right_count = node.borrow().right_child_count;

            candidate.borrow_mut().left_child_node = node_left.clone();
            candidate.borrow_mut().left_child_count = node_left_count;
            candidate.borrow_mut().right_child_node = node_right.clone();
            candidate.borrow_mut().right_child_count = node_right_count;

            if let Some(ref lc) = node_left {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref rc) = node_right {
                rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance up to candidate.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start.clone();
                while !Rc::ptr_eq(&bs, &candidate) {
                    update_depth(&bs);
                    let bal = balance(&bs);
                    if bal > 1 {
                        let right_child = bs.borrow().right_child_node.clone().unwrap();
                        if balance(&right_child) < 0 {
                            rotate_right(&mut self.root_node, &right_child);
                        }
                        bs = rotate_left(&mut self.root_node, &bs);
                    } else if bal < -1 {
                        let left_child = bs.borrow().left_child_node.clone().unwrap();
                        if balance(&left_child) > 0 {
                            rotate_left(&mut self.root_node, &left_child);
                        }
                        bs = rotate_right(&mut self.root_node, &bs);
                    }
                    let parent = get_parent(&bs).expect("walk should reach candidate");
                    bs = parent;
                }
            }

            update_depth(&candidate);

            bubble_up = get_parent(&candidate);

            if let Some(ref bu) = bubble_up {
                let candidate_is_left_of_bu = bu
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &candidate));
                if candidate_is_left_of_bu {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Zero or one child case.
            let node_parent = get_parent(node);
            match &node_parent {
                None => {
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
                }
                Some(np) => {
                    let (candidate, candidate_count) =
                        if let Some(rc) = node.borrow().right_child_node.clone() {
                            let cc = node.borrow().right_child_count;
                            (Some(rc), cc)
                        } else {
                            let cc = node.borrow().left_child_count;
                            (node.borrow().left_child_node.clone(), cc)
                        };

                    let node_is_left = np
                        .borrow()
                        .left_child_node
                        .as_ref()
                        .map_or(false, |c| Rc::ptr_eq(c, node));
                    if node_is_left {
                        np.borrow_mut().left_child_node = candidate.clone();
                        np.borrow_mut().left_child_count = candidate_count;
                    } else {
                        np.borrow_mut().right_child_node = candidate.clone();
                        np.borrow_mut().right_child_count = candidate_count;
                    }

                    if let Some(ref c) = candidate {
                        c.borrow_mut().parent_node = Some(Rc::downgrade(np));
                    }

                    bubble_up = Some(np.clone());
                }
            }
        }

        // Bubble up rebalancing.
        let mut bubbling_finished = false;
        let mut bu = bubble_up;
        while let Some(node_bu) = bu {
            let post_node = if !bubbling_finished {
                let left_depth = node_bu
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let right_depth = node_bu
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |n| n.borrow().depth + 1);
                let new_depth = max(left_depth, right_depth);
                let cur_depth = node_bu.borrow().depth;
                let depth_changed = new_depth != cur_depth;
                node_bu.borrow_mut().depth = new_depth;

                let bal = balance(&node_bu);
                if bal < -1 {
                    let left_child = node_bu.borrow().left_child_node.clone().unwrap();
                    if balance(&left_child) > 0 {
                        rotate_left(&mut self.root_node, &left_child);
                    }
                    rotate_right(&mut self.root_node, &node_bu)
                } else if bal > 1 {
                    let right_child = node_bu.borrow().right_child_node.clone().unwrap();
                    if balance(&right_child) < 0 {
                        rotate_right(&mut self.root_node, &right_child);
                    }
                    rotate_left(&mut self.root_node, &node_bu)
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    node_bu.clone()
                }
            } else {
                node_bu.clone()
            };

            // Decrement parent's child count for the side that contains post_node.
            let parent = get_parent(&post_node);
            if let Some(ref p) = parent {
                let post_is_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &post_node));
                if post_is_left {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bu = parent;
        }

        node.borrow_mut().weak_ref_node_valid = 0;
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
    pub fn bostree_select(&self, mut index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        while let Some(n) = current {
            let lcc = n.borrow().left_child_count;
            if lcc <= index {
                index -= lcc;
                if index == 0 {
                    return Some(n);
                }
                index -= 1;
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
        if let Some(root) = &self.root_node {
            println!("digraph {{\n  ordering = out;");
            print_helper(root);
            println!("}}");
        }
    }
}

#[cfg(debug_assertions)]
fn print_helper(node: &Rc<RefCell<BOSNode>>) {
    let (key_clone, lcc, rcc, dep, parent_key, lc, rc) = {
        let nb = node.borrow();
        (
            nb.key.clone(),
            nb.left_child_count,
            nb.right_child_count,
            nb.depth,
            nb.parent_node
                .as_ref()
                .and_then(|w| w.upgrade())
                .map(|p| p.borrow().key.clone()),
            nb.left_child_node.clone(),
            nb.right_child_node.clone(),
        )
    };
    println!(
        "  {} [label=\"\\N ({},{},{})\"];",
        key_clone, lcc, rcc, dep
    );
    if let Some(pk) = parent_key {
        println!("  {} -> {} [color=green];", key_clone, pk);
    }
    if let Some(l) = lc {
        println!("  {} -> {}", key_clone, l.borrow().key);
        print_helper(&l);
    }
    if let Some(r) = rc {
        println!("  {} -> {}", key_clone, r.borrow().key);
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
        let mut cur = r;
        loop {
            let next = cur.borrow().left_child_node.clone();
            match next {
                Some(n) => cur = n,
                None => return Some(cur),
            }
        }
    }
    let mut cur = node.clone();
    loop {
        let parent = cur
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent {
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |r| Rc::ptr_eq(r, &cur));
                if is_right {
                    cur = p;
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
    let left = node.borrow().left_child_node.clone();
    if let Some(l) = left {
        let mut cur = l;
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
        let parent = cur
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent {
            Some(p) => {
                let is_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |l| Rc::ptr_eq(l, &cur));
                if is_left {
                    cur = p;
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
    let mut cur = node.clone();
    loop {
        let parent = cur
            .borrow()
            .parent_node
            .as_ref()
            .and_then(|w| w.upgrade());
        match parent {
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |r| Rc::ptr_eq(r, &cur));
                if is_right {
                    counter += 1 + p.borrow().left_child_count;
                }
                cur = p;
            }
            None => break,
        }
    }
    counter
}
