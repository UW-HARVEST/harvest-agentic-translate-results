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

// ---------- Local helpers ----------

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn left_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().left_child_node.clone()
}

fn right_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().right_child_node.clone()
}

fn left_depth_of(node: &Rc<RefCell<BOSNode>>) -> u32 {
    match node.borrow().left_child_node.as_ref() {
        Some(n) => n.borrow().depth + 1,
        None => 0,
    }
}

fn right_depth_of(node: &Rc<RefCell<BOSNode>>) -> u32 {
    match node.borrow().right_child_node.as_ref() {
        Some(n) => n.borrow().depth + 1,
        None => 0,
    }
}

fn balance_of(node: &Rc<RefCell<BOSNode>>) -> i32 {
    right_depth_of(node) as i32 - left_depth_of(node) as i32
}

fn is_left_child_of(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    match parent.borrow().left_child_node.as_ref() {
        Some(c) => Rc::ptr_eq(c, child),
        None => false,
    }
}

fn _is_right_child_of(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    match parent.borrow().right_child_node.as_ref() {
        Some(c) => Rc::ptr_eq(c, child),
        None => false,
    }
}

fn rotate_right(
    root: &mut Option<Rc<RefCell<BOSNode>>>,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    // Rotate right:
    //
    //      P                     L
    //  L        R     -->    c1      P
    //c1 c2                        c2     R
    //
    let l = p.borrow().left_child_node.clone().expect("left child must exist");
    let p_parent = parent_of(p);

    // Update P's parent (or root) to point to L
    if let Some(ref pp) = p_parent {
        if is_left_child_of(pp, p) {
            pp.borrow_mut().left_child_node = Some(l.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        *root = Some(l.clone());
    }

    // L.parent = P.parent
    l.borrow_mut().parent_node = p_parent.as_ref().map(|x| Rc::downgrade(x));

    // P.left = L.right; P.left_count = L.right_count
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;

    // If P.left now exists, set its parent to P
    if let Some(ref pl) = l_right {
        pl.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // P.depth = max(left,right)
    let pld = left_depth_of(p);
    let prd = right_depth_of(p);
    p.borrow_mut().depth = pld.max(prd);

    // L.right = P; P.parent = L
    l.borrow_mut().right_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L.right_count = P.left_count + P.right_count + 1
    let plcc = p.borrow().left_child_count;
    let prcc = p.borrow().right_child_count;
    l.borrow_mut().right_child_count = plcc + prcc + 1;

    // L.depth = max(left, right)
    let lld = left_depth_of(&l);
    let lrd = right_depth_of(&l);
    l.borrow_mut().depth = lld.max(lrd);

    l
}

fn rotate_left(
    root: &mut Option<Rc<RefCell<BOSNode>>>,
    p: &Rc<RefCell<BOSNode>>,
) -> Rc<RefCell<BOSNode>> {
    // Rotate left:
    //
    //      P                     R
    //  L        R     -->    P      c2
    //         c1 c2        L  c1
    //
    let r = p.borrow().right_child_node.clone().expect("right child must exist");
    let p_parent = parent_of(p);

    if let Some(ref pp) = p_parent {
        if is_left_child_of(pp, p) {
            pp.borrow_mut().left_child_node = Some(r.clone());
        } else {
            pp.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        *root = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(|x| Rc::downgrade(x));

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;

    if let Some(ref pr) = r_left {
        pr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let pld = left_depth_of(p);
    let prd = right_depth_of(p);
    p.borrow_mut().depth = pld.max(prd);

    r.borrow_mut().left_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    let plcc = p.borrow().left_child_count;
    let prcc = p.borrow().right_child_count;
    r.borrow_mut().left_child_count = plcc + prcc + 1;

    let rld = left_depth_of(&r);
    let rrd = right_depth_of(&r);
    r.borrow_mut().depth = rld.max(rrd);

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
        match self.root_node.as_ref() {
            Some(r) => {
                let r = r.borrow();
                r.left_child_count + r.right_child_count + 1
            }
            None => 0,
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Walk down to find insert position; increment counts on the way.
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut current = self.root_node.clone();
        let mut go_left = false;

        while let Some(n) = current {
            let cmp = (self.cmp_function)(&key, &n.borrow().key);
            if cmp < 0 {
                n.borrow_mut().left_child_count += 1;
                let next = n.borrow().left_child_node.clone();
                parent_node = Some(n);
                go_left = true;
                current = next;
            } else {
                n.borrow_mut().right_child_count += 1;
                let next = n.borrow().right_child_node.clone();
                parent_node = Some(n);
                go_left = false;
                current = next;
            }
        }

        // Create the new node.
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

        let parent_unwrapped = match parent_node {
            None => {
                // Empty tree case
                self.root_node = Some(new_node.clone());
                return new_node;
            }
            Some(p) => {
                if go_left {
                    p.borrow_mut().left_child_node = Some(new_node.clone());
                } else {
                    p.borrow_mut().right_child_node = Some(new_node.clone());
                }
                p
            }
        };

        // Check if the depth changed: only if this is the parent's first child.
        let parent_has_left = parent_unwrapped.borrow().left_child_node.is_some();
        let parent_has_right = parent_unwrapped.borrow().right_child_node.is_some();
        if parent_has_left ^ parent_has_right {
            parent_unwrapped.borrow_mut().depth += 1;

            let mut walker = parent_unwrapped;
            loop {
                let walker_parent = parent_of(&walker);
                match walker_parent {
                    None => break,
                    Some(p) => {
                        walker = p;

                        let new_left_depth = left_depth_of(&walker);
                        let new_right_depth = right_depth_of(&walker);
                        let max_depth = new_left_depth.max(new_right_depth);

                        let cur_depth = walker.borrow().depth;
                        if cur_depth != max_depth {
                            walker.borrow_mut().depth = max_depth;
                        } else {
                            break;
                        }

                        // Check AVL property
                        if new_left_depth == new_right_depth + 2 {
                            // Left-right case
                            let walker_left = walker
                                .borrow()
                                .left_child_node
                                .clone()
                                .expect("left child exists");
                            if balance_of(&walker_left) > 0 {
                                rotate_left(&mut self.root_node, &walker_left);
                            }
                            walker = rotate_right(&mut self.root_node, &walker);
                        } else if new_right_depth == new_left_depth + 2 {
                            // Right-left case
                            let walker_right = walker
                                .borrow()
                                .right_child_node
                                .clone()
                                .expect("right child exists");
                            if balance_of(&walker_right) < 0 {
                                rotate_right(&mut self.root_node, &walker_right);
                            }
                            walker = rotate_left(&mut self.root_node, &walker);
                        }
                    }
                }
            }
        }

        new_node
    }
    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>>;

        let node_left = left_of(node);
        let node_right = right_of(node);

        if node_left.is_some() && node_right.is_some() {
            // Two-children case
            let nleft = node_left.unwrap();
            let nright = node_right.unwrap();
            let left_depth = nleft.borrow().depth;
            let right_depth = nright.borrow().depth;

            let candidate: Rc<RefCell<BOSNode>>;
            let lost_child: Option<Rc<RefCell<BOSNode>>>;

            if left_depth >= right_depth {
                // Pick predecessor (largest in left subtree)
                node.borrow_mut().left_child_count -= 1;
                let mut cand = nleft.clone();
                loop {
                    let next = right_of(&cand);
                    match next {
                        None => break,
                        Some(n) => {
                            cand.borrow_mut().right_child_count -= 1;
                            cand = n;
                        }
                    }
                }
                lost_child = left_of(&cand);
                candidate = cand;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut cand = nright.clone();
                loop {
                    let next = left_of(&cand);
                    match next {
                        None => break,
                        Some(n) => {
                            cand.borrow_mut().left_child_count -= 1;
                            cand = n;
                        }
                    }
                }
                lost_child = right_of(&cand);
                candidate = cand;
            }

            let bubble_start = parent_of(&candidate).expect("candidate has a parent");

            // Detach candidate from its old position; attach lost_child there.
            if is_left_child_of(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was.
            let node_parent = parent_of(node);
            if let Some(ref np) = node_parent {
                if is_left_child_of(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node =
                node_parent.as_ref().map(|x| Rc::downgrade(x));

            // Copy node's children/counts into candidate.
            let n_lc = node.borrow().left_child_node.clone();
            let n_lcc = node.borrow().left_child_count;
            let n_rc = node.borrow().right_child_node.clone();
            let n_rcc = node.borrow().right_child_count;
            candidate.borrow_mut().left_child_node = n_lc.clone();
            candidate.borrow_mut().left_child_count = n_lcc;
            candidate.borrow_mut().right_child_node = n_rc.clone();
            candidate.borrow_mut().right_child_count = n_rcc;

            if let Some(ref lc) = candidate.borrow().left_child_node.clone() {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref rc) = candidate.borrow().right_child_node.clone() {
                rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate (if bubble_start != node).
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start.clone();
                loop {
                    if Rc::ptr_eq(&bs, &candidate) {
                        break;
                    }
                    let new_depth = left_depth_of(&bs).max(right_depth_of(&bs));
                    bs.borrow_mut().depth = new_depth;
                    let bal = balance_of(&bs);
                    let bs_after: Rc<RefCell<BOSNode>>;
                    if bal > 1 {
                        let r_child = right_of(&bs).expect("right child exists");
                        if balance_of(&r_child) < 0 {
                            rotate_right(&mut self.root_node, &r_child);
                        }
                        bs_after = rotate_left(&mut self.root_node, &bs);
                    } else if bal < -1 {
                        let l_child = left_of(&bs).expect("left child exists");
                        if balance_of(&l_child) > 0 {
                            rotate_left(&mut self.root_node, &l_child);
                        }
                        bs_after = rotate_right(&mut self.root_node, &bs);
                    } else {
                        bs_after = bs.clone();
                    }
                    let next = parent_of(&bs_after);
                    match next {
                        None => break,
                        Some(p) => bs = p,
                    }
                }
            }

            // Fix candidate's depth.
            let cd = left_depth_of(&candidate).max(right_depth_of(&candidate));
            candidate.borrow_mut().depth = cd;

            bubble_up = parent_of(&candidate);

            if let Some(ref bu) = bubble_up {
                if is_left_child_of(bu, &candidate) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Zero or one child case.
            let node_parent = parent_of(node);
            if node_parent.is_none() {
                // Was the root
                if let Some(l) = node_left.clone() {
                    self.root_node = Some(l.clone());
                    l.borrow_mut().parent_node = None;
                } else if let Some(r) = node_right.clone() {
                    self.root_node = Some(r.clone());
                    r.borrow_mut().parent_node = None;
                } else {
                    self.root_node = None;
                }
                bubble_up = None;
            } else {
                let np = node_parent.unwrap();
                let candidate;
                let candidate_count;
                if let Some(r) = node_right.clone() {
                    candidate = Some(r);
                    candidate_count = node.borrow().right_child_count;
                } else if let Some(l) = node_left.clone() {
                    candidate = Some(l);
                    candidate_count = node.borrow().left_child_count;
                } else {
                    candidate = None;
                    candidate_count = 0;
                }

                if is_left_child_of(&np, node) {
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

        // Bubble up to the root, updating depths and rebalancing.
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up {
            let mut current = bu;
            if !bubbling_finished {
                let ld = left_depth_of(&current);
                let rd = right_depth_of(&current);
                let new_depth = ld.max(rd);
                let depth_changed = new_depth != current.borrow().depth;
                current.borrow_mut().depth = new_depth;

                let bal = balance_of(&current);
                if bal < -1 {
                    let l_child = left_of(&current).expect("left child exists");
                    if balance_of(&l_child) > 0 {
                        rotate_left(&mut self.root_node, &l_child);
                    }
                    current = rotate_right(&mut self.root_node, &current);
                } else if bal > 1 {
                    let r_child = right_of(&current).expect("right child exists");
                    if balance_of(&r_child) < 0 {
                        rotate_right(&mut self.root_node, &r_child);
                    }
                    current = rotate_left(&mut self.root_node, &current);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Decrement parent's child count for the appropriate side.
            let parent_opt = parent_of(&current);
            if let Some(ref p) = parent_opt {
                if is_left_child_of(p, &current) {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up = parent_opt;
        }

        // Mark node invalid and unref it.
        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach the node from any tree references it might still hold.
        node.borrow_mut().parent_node = None;
        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
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
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
            (n.weak_ref_count, n.weak_ref_node_valid)
        };
        if count == 0 {
            if let Some(f) = self.free_function {
                f(node);
            }
            return None;
        } else if valid != 0 {
            return Some(node.clone());
        }
        None
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
        if self.root_node.is_none() {
            return;
        }
        println!("digraph {{\n  ordering = out;");
        if let Some(ref r) = self.root_node {
            print_helper(r);
        }
        println!("}}");
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
    if let Some(ref lc) = left {
        println!("  {} -> {}", key, lc.borrow().key);
        print_helper(lc);
    }
    if let Some(ref rc) = right {
        println!("  {} -> {}", key, rc.borrow().key);
        print_helper(rc);
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
            let next = current.borrow().left_child_node.clone();
            match next {
                None => return Some(current),
                Some(n) => current = n,
            }
        }
    }

    let mut current = node.clone();
    loop {
        let parent_opt = parent_of(&current);
        match parent_opt {
            None => return None,
            Some(p) => {
                let is_right = match p.borrow().right_child_node.as_ref() {
                    Some(c) => Rc::ptr_eq(c, &current),
                    None => false,
                };
                if is_right {
                    current = p;
                } else {
                    return Some(p);
                }
            }
        }
    }
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(left) = node.borrow().left_child_node.clone() {
        let mut current = left;
        loop {
            let next = current.borrow().right_child_node.clone();
            match next {
                None => return Some(current),
                Some(n) => current = n,
            }
        }
    }

    let mut current = node.clone();
    loop {
        let parent_opt = parent_of(&current);
        match parent_opt {
            None => return None,
            Some(p) => {
                let is_left = match p.borrow().left_child_node.as_ref() {
                    Some(c) => Rc::ptr_eq(c, &current),
                    None => false,
                };
                if is_left {
                    current = p;
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
    let mut current = node.clone();
    loop {
        let parent_opt = parent_of(&current);
        match parent_opt {
            None => break,
            Some(p) => {
                let is_right = match p.borrow().right_child_node.as_ref() {
                    Some(c) => Rc::ptr_eq(c, &current),
                    None => false,
                };
                if is_right {
                    counter += 1 + p.borrow().left_child_count;
                }
                current = p;
            }
        }
    }
    counter
}
