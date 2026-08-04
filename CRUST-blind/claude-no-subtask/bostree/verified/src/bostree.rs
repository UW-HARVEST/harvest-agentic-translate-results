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

// ---------- Internal helpers ----------

fn imax_u32(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow()
        .parent_node
        .as_ref()
        .and_then(|p| p.upgrade())
}

fn is_left_child(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |c| Rc::ptr_eq(c, child))
}

fn left_depth_plus_one(n: &BOSNode) -> u32 {
    n.left_child_node
        .as_ref()
        .map_or(0, |c| c.borrow().depth + 1)
}

fn right_depth_plus_one(n: &BOSNode) -> u32 {
    n.right_child_node
        .as_ref()
        .map_or(0, |c| c.borrow().depth + 1)
}

fn compute_depth(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let n = node.borrow();
    imax_u32(left_depth_plus_one(&n), right_depth_plus_one(&n))
}

fn balance_factor(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    right_depth_plus_one(&n) as i32 - left_depth_plus_one(&n) as i32
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    // Rotate right:
    //
    //      P                     L
    //   L     R    -->        c1   P
    // c1 c2                       c2  R
    //
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires left child");

    let p_parent = parent_of(p);

    // Update P's parent's child pointer (or root)
    if let Some(ref pp) = p_parent {
        let pp_left = is_left_child(pp, p);
        if pp_left {
            pp.borrow_mut().left_child_node = Some(l.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    // L's parent = P's parent
    l.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P.left = L.right (c2), copy count
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    {
        let mut pb = p.borrow_mut();
        pb.left_child_node = l_right.clone();
        pb.left_child_count = l_right_count;
    }
    if let Some(ref c2) = l_right {
        c2.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // P's depth
    let pd = compute_depth(p);
    p.borrow_mut().depth = pd;

    // L.right = P, P.parent = L
    l.borrow_mut().right_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L.right_child_count = P total
    let p_total = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    l.borrow_mut().right_child_count = p_total;

    // L's depth
    let ld = compute_depth(&l);
    l.borrow_mut().depth = ld;

    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    // Rotate left:
    //
    //      P                     R
    //   L     R    -->        P    c2
    //       c1 c2            L  c1
    //
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires right child");

    let p_parent = parent_of(p);

    if let Some(ref pp) = p_parent {
        let pp_left = is_left_child(pp, p);
        if pp_left {
            pp.borrow_mut().left_child_node = Some(r.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P.right = R.left, copy count
    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    {
        let mut pb = p.borrow_mut();
        pb.right_child_node = r_left.clone();
        pb.right_child_count = r_left_count;
    }
    if let Some(ref c1) = r_left {
        c1.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let pd = compute_depth(p);
    p.borrow_mut().depth = pd;

    r.borrow_mut().left_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    let p_total = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    r.borrow_mut().left_child_count = p_total;

    let rd = compute_depth(&r);
    r.borrow_mut().depth = rd;

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
        match self.root_node {
            Some(ref r) => {
                let n = r.borrow();
                n.left_child_count + n.right_child_count + 1
            }
            None => 0,
        }
    }

    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find tree position to insert new node
        let mut parent_opt: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;
        let mut current = self.root_node.clone();

        while let Some(n) = current {
            let cmp = (self.cmp_function)(&key, &n.borrow().key);
            if cmp < 0 {
                n.borrow_mut().left_child_count += 1;
                let next = n.borrow().left_child_node.clone();
                parent_opt = Some(n.clone());
                go_left = true;
                current = next;
            } else {
                n.borrow_mut().right_child_count += 1;
                let next = n.borrow().right_child_node.clone();
                parent_opt = Some(n.clone());
                go_left = false;
                current = next;
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: parent_opt.as_ref().map(Rc::downgrade),
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        let mut parent_node = match parent_opt {
            Some(p) => {
                if go_left {
                    p.borrow_mut().left_child_node = Some(new_node.clone());
                } else {
                    p.borrow_mut().right_child_node = Some(new_node.clone());
                }
                p
            }
            None => {
                // First node
                self.root_node = Some(new_node.clone());
                return new_node;
            }
        };

        // Check if depth changed: only if this is parent's first child
        let parent_has_one_child = {
            let pb = parent_node.borrow();
            pb.left_child_node.is_some() ^ pb.right_child_node.is_some()
        };

        if parent_has_one_child {
            parent_node.borrow_mut().depth += 1;

            // Bubble up
            loop {
                let next_parent = parent_of(&parent_node);
                let np = match next_parent {
                    Some(p) => p,
                    None => break,
                };
                parent_node = np;

                let new_left_depth: u32 = parent_node
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0, |c| c.borrow().depth + 1);
                let new_right_depth: u32 = parent_node
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0, |c| c.borrow().depth + 1);
                let max_depth = imax_u32(new_left_depth, new_right_depth);

                let cur_depth = parent_node.borrow().depth;
                if cur_depth != max_depth {
                    parent_node.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                // Check AVL property using signed difference
                let nld = new_left_depth as i32;
                let nrd = new_right_depth as i32;
                if nld - 2 == nrd {
                    // Left-right case
                    let left_child = parent_node.borrow().left_child_node.clone().unwrap();
                    if balance_factor(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    parent_node = rotate_right(self, &parent_node);
                } else if nld + 2 == nrd {
                    // Right-left case
                    let right_child = parent_node.borrow().right_child_node.clone().unwrap();
                    if balance_factor(&right_child) < 0 {
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
        let bubble_up: Option<Rc<RefCell<BOSNode>>>;

        let has_both = {
            let n = node.borrow();
            n.left_child_node.is_some() && n.right_child_node.is_some()
        };

        if has_both {
            // Determine which side to take the candidate from
            let (lc, rc) = {
                let n = node.borrow();
                (
                    n.left_child_node.clone().unwrap(),
                    n.right_child_node.clone().unwrap(),
                )
            };
            let left_depth = lc.borrow().depth;
            let right_depth = rc.borrow().depth;
            drop(lc);
            drop(rc);

            let candidate: Rc<RefCell<BOSNode>>;
            let lost_child: Option<Rc<RefCell<BOSNode>>>;

            if left_depth >= right_depth {
                // Pick rightmost in left subtree
                node.borrow_mut().left_child_count -= 1;
                let mut current = node.borrow().left_child_node.clone().unwrap();
                loop {
                    let r = current.borrow().right_child_node.clone();
                    match r {
                        Some(rc) => {
                            current.borrow_mut().right_child_count -= 1;
                            current = rc;
                        }
                        None => break,
                    }
                }
                lost_child = current.borrow().left_child_node.clone();
                candidate = current;
            } else {
                // Pick leftmost in right subtree
                node.borrow_mut().right_child_count -= 1;
                let mut current = node.borrow().right_child_node.clone().unwrap();
                loop {
                    let l = current.borrow().left_child_node.clone();
                    match l {
                        Some(lc) => {
                            current.borrow_mut().left_child_count -= 1;
                            current = lc;
                        }
                        None => break,
                    }
                }
                lost_child = current.borrow().right_child_node.clone();
                candidate = current;
            }

            let bubble_start = parent_of(&candidate)
                .expect("candidate must have a parent (node is its ancestor)");

            // Replace candidate at its old position with lost_child
            let bs_left_is_candidate = is_left_child(&bubble_start, &candidate);
            if bs_left_is_candidate {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was
            let node_parent = parent_of(node);
            if let Some(ref np) = node_parent {
                if is_left_child(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            // Copy node's children into candidate
            let (n_left, n_lcount, n_right, n_rcount) = {
                let n = node.borrow();
                (
                    n.left_child_node.clone(),
                    n.left_child_count,
                    n.right_child_node.clone(),
                    n.right_child_count,
                )
            };
            {
                let mut c = candidate.borrow_mut();
                c.left_child_node = n_left.clone();
                c.left_child_count = n_lcount;
                c.right_child_node = n_right.clone();
                c.right_child_count = n_rcount;
            }
            if let Some(ref l) = n_left {
                l.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref r) = n_right {
                r.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate (if bubble_start != node)
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut current = bubble_start;
                while !Rc::ptr_eq(&current, &candidate) {
                    let cd = compute_depth(&current);
                    current.borrow_mut().depth = cd;
                    let bal = balance_factor(&current);
                    if bal > 1 {
                        let r = current.borrow().right_child_node.clone().unwrap();
                        if balance_factor(&r) < 0 {
                            rotate_right(self, &r);
                        }
                        current = rotate_left(self, &current);
                    } else if bal < -1 {
                        let l = current.borrow().left_child_node.clone().unwrap();
                        if balance_factor(&l) > 0 {
                            rotate_left(self, &l);
                        }
                        current = rotate_right(self, &current);
                    }
                    let next = parent_of(&current);
                    match next {
                        Some(p) => current = p,
                        None => break,
                    }
                }
            }

            // Fixup candidate's depth
            let cd = compute_depth(&candidate);
            candidate.borrow_mut().depth = cd;

            bubble_up = parent_of(&candidate);

            // Decrement immediate parent's child count for candidate
            if let Some(ref bp) = bubble_up {
                if is_left_child(bp, &candidate) {
                    bp.borrow_mut().left_child_count -= 1;
                } else {
                    bp.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Node has at most one child
            let node_parent = parent_of(node);
            if node_parent.is_none() {
                // Node was root
                let (l_opt, r_opt) = {
                    let n = node.borrow();
                    (n.left_child_node.clone(), n.right_child_node.clone())
                };
                if let Some(ref l) = l_opt {
                    self.root_node = Some(l.clone());
                    l.borrow_mut().parent_node = None;
                } else if let Some(ref r) = r_opt {
                    self.root_node = Some(r.clone());
                    r.borrow_mut().parent_node = None;
                } else {
                    self.root_node = None;
                }
                bubble_up = None;
            } else {
                let np = node_parent.unwrap();
                let (candidate, candidate_count) = {
                    let n = node.borrow();
                    if let Some(ref r) = n.right_child_node {
                        (Some(r.clone()), n.right_child_count)
                    } else if let Some(ref l) = n.left_child_node {
                        (Some(l.clone()), n.left_child_count)
                    } else {
                        (None, 0)
                    }
                };
                if is_left_child(&np, node) {
                    let mut npm = np.borrow_mut();
                    npm.left_child_node = candidate.clone();
                    npm.left_child_count = candidate_count;
                } else {
                    let mut npm = np.borrow_mut();
                    npm.right_child_node = candidate.clone();
                    npm.right_child_count = candidate_count;
                }
                if let Some(ref c) = candidate {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&np));
                }
                bubble_up = Some(np);
            }
        }

        // Bubble up: rebalance and fix child counts
        let mut bubbling_finished = false;
        let mut bu_opt = bubble_up;
        while let Some(bn) = bu_opt {
            let mut current = bn;
            if !bubbling_finished {
                let new_left_depth: u32 = current
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0, |c| c.borrow().depth + 1);
                let new_right_depth: u32 = current
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0, |c| c.borrow().depth + 1);
                let new_depth = imax_u32(new_left_depth, new_right_depth);
                let depth_changed = current.borrow().depth != new_depth;
                current.borrow_mut().depth = new_depth;

                let bal = balance_factor(&current);
                if bal < -1 {
                    let l = current.borrow().left_child_node.clone().unwrap();
                    if balance_factor(&l) > 0 {
                        rotate_left(self, &l);
                    }
                    current = rotate_right(self, &current);
                } else if bal > 1 {
                    let r = current.borrow().right_child_node.clone().unwrap();
                    if balance_factor(&r) < 0 {
                        rotate_right(self, &r);
                    }
                    current = rotate_left(self, &current);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Decrement parent's child count for current
            let p = parent_of(&current);
            if let Some(ref pp) = p {
                if is_left_child(pp, &current) {
                    pp.borrow_mut().left_child_count -= 1;
                } else {
                    pp.borrow_mut().right_child_count -= 1;
                }
            }
            bu_opt = p;
        }

        node.borrow_mut().weak_ref_node_valid = 0;
        let _ = self.bostree_node_weak_unref(node);
    }

    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let (count, valid) = {
            let mut n = node.borrow_mut();
            if n.weak_ref_count > 0 {
                n.weak_ref_count -= 1;
            }
            (n.weak_ref_count, n.weak_ref_node_valid)
        };
        if count == 0 {
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
            }
            let next = if cmp < 0 {
                n.borrow().left_child_node.clone()
            } else {
                n.borrow().right_child_node.clone()
            };
            current = next;
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
        if let Some(ref root) = self.root_node {
            println!("digraph {{");
            println!("  ordering = out;");
            Self::bostree_print_helper(root);
            println!("}}");
        }
    }

    #[cfg(debug_assertions)]
    fn bostree_print_helper(node: &Rc<RefCell<BOSNode>>) {
        let key;
        let lcc;
        let rcc;
        let depth;
        let parent_key;
        let left;
        let right;
        {
            let n = node.borrow();
            key = n.key.clone();
            lcc = n.left_child_count;
            rcc = n.right_child_count;
            depth = n.depth;
            parent_key = n
                .parent_node
                .as_ref()
                .and_then(|p| p.upgrade())
                .map(|p| p.borrow().key.clone());
            left = n.left_child_node.clone();
            right = n.right_child_node.clone();
        }
        println!(
            "  {} [label=\"\\N ({},{},{})\"];",
            key, lcc, rcc, depth
        );
        if let Some(pk) = parent_key {
            println!("  {} -> {} [color=green];", key, pk);
        }
        if let Some(ref l) = left {
            let lk = l.borrow().key.clone();
            println!("  {} -> {}", key, lk);
            Self::bostree_print_helper(l);
        }
        if let Some(ref r) = right {
            let rk = r.borrow().key.clone();
            println!("  {} -> {}", key, rk);
            Self::bostree_print_helper(r);
        }
    }
}
/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    {
        let mut n = node.borrow_mut();
        if n.weak_ref_count < 127 {
            n.weak_ref_count += 1;
        }
    }
    node.clone()
}
/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    // If there is a right child, descend to its leftmost node
    let right = node.borrow().right_child_node.clone();
    if let Some(r) = right {
        let mut current = r;
        loop {
            let l = current.borrow().left_child_node.clone();
            match l {
                Some(lc) => current = lc,
                None => return Some(current),
            }
        }
    }

    // Otherwise walk up while we are the right child
    let mut current = node.clone();
    loop {
        let p_opt = parent_of(&current);
        match p_opt {
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
    // If there is a left child, descend to its rightmost node
    let left = node.borrow().left_child_node.clone();
    if let Some(l) = left {
        let mut current = l;
        loop {
            let r = current.borrow().right_child_node.clone();
            match r {
                Some(rc) => current = rc,
                None => return Some(current),
            }
        }
    }

    // Otherwise walk up while we are the left child
    let mut current = node.clone();
    loop {
        let p_opt = parent_of(&current);
        match p_opt {
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
    let mut current = node.clone();
    loop {
        let p_opt = parent_of(&current);
        match p_opt {
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |c| Rc::ptr_eq(c, &current));
                if is_right {
                    counter += 1 + p.borrow().left_child_count;
                }
                current = p;
            }
            None => return counter,
        }
    }
}
