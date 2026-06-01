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

/* ------------------- Helpers -------------------- */

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn left_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().left_child_node.clone()
}

fn right_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().right_child_node.clone()
}

fn compute_depth(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let n = node.borrow();
    let l = n.left_child_node.as_ref().map_or(0, |x| x.borrow().depth + 1);
    let r = n.right_child_node.as_ref().map_or(0, |x| x.borrow().depth + 1);
    if l > r { l } else { r }
}

fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let l = n.left_child_node.as_ref().map_or(0, |x| x.borrow().depth as i32 + 1);
    let r = n.right_child_node.as_ref().map_or(0, |x| x.borrow().depth as i32 + 1);
    r - l
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    // Rotate right:
    //
    //      P                     L
    //  L        R     -->    c1      P
    //c1 c2                        c2     R
    //
    let l = left_of(p).expect("rotate_right requires a left child");
    let p_parent = parent_of(p);

    // Update P->parent's child pointer
    if let Some(ref pp) = p_parent {
        let mut pp_mut = pp.borrow_mut();
        let pp_left_is_p = pp_mut
            .left_child_node
            .as_ref()
            .map_or(false, |x| Rc::ptr_eq(x, p));
        if pp_left_is_p {
            pp_mut.left_child_node = Some(l.clone());
        } else {
            pp_mut.right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    // L's parent = P's parent
    l.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P's left = L's right
    let l_right = right_of(&l);
    let l_right_count = l.borrow().right_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.left_child_node = l_right.clone();
        p_mut.left_child_count = l_right_count;
    }
    if let Some(ref lr) = l_right {
        lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // P's depth
    let new_p_depth = compute_depth(p);
    p.borrow_mut().depth = new_p_depth;

    // L's right = P, P's parent = L
    l.borrow_mut().right_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L's right count = P's left + right + 1
    let new_l_right_count = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    l.borrow_mut().right_child_count = new_l_right_count;

    // L's depth
    let new_l_depth = compute_depth(&l);
    l.borrow_mut().depth = new_l_depth;

    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    // Rotate left:
    //
    //      P                     R
    //  L        R     -->    P      c2
    //         c1 c2        L  c1
    //
    let r = right_of(p).expect("rotate_left requires a right child");
    let p_parent = parent_of(p);

    if let Some(ref pp) = p_parent {
        let mut pp_mut = pp.borrow_mut();
        let pp_left_is_p = pp_mut
            .left_child_node
            .as_ref()
            .map_or(false, |x| Rc::ptr_eq(x, p));
        if pp_left_is_p {
            pp_mut.left_child_node = Some(r.clone());
        } else {
            pp_mut.right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    let r_left = left_of(&r);
    let r_left_count = r.borrow().left_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.right_child_node = r_left.clone();
        p_mut.right_child_count = r_left_count;
    }
    if let Some(ref rl) = r_left {
        rl.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let new_p_depth = compute_depth(p);
    p.borrow_mut().depth = new_p_depth;

    r.borrow_mut().left_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    let new_r_left_count = {
        let pb = p.borrow();
        pb.left_child_count + pb.right_child_count + 1
    };
    r.borrow_mut().left_child_count = new_r_left_count;

    let new_r_depth = compute_depth(&r);
    r.borrow_mut().depth = new_r_depth;

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
            Some(r) => {
                let rb = r.borrow();
                rb.left_child_count + rb.right_child_count + 1
            }
            None => 0,
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find tree position to insert new node
        let mut current = self.root_node.clone();
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;

        while let Some(n) = current {
            parent_node = Some(n.clone());
            let cmp = (self.cmp_function)(&key, &n.borrow().key);
            if cmp < 0 {
                n.borrow_mut().left_child_count += 1;
                go_left = true;
                current = n.borrow().left_child_node.clone();
            } else {
                n.borrow_mut().right_child_count += 1;
                go_left = false;
                current = n.borrow().right_child_node.clone();
            }
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

        // Attach to parent or root
        if let Some(ref pn) = parent_node {
            if go_left {
                pn.borrow_mut().left_child_node = Some(new_node.clone());
            } else {
                pn.borrow_mut().right_child_node = Some(new_node.clone());
            }
        } else {
            self.root_node = Some(new_node.clone());
            return new_node;
        }

        // Check if depth changed: only if this is the first child of the parent
        let parent = parent_node.as_ref().unwrap().clone();
        let needs_depth_update = {
            let pb = parent.borrow();
            let has_left = pb.left_child_node.is_some();
            let has_right = pb.right_child_node.is_some();
            has_left ^ has_right
        };

        if needs_depth_update {
            parent.borrow_mut().depth += 1;
            let mut current_parent = parent_of(&parent);
            while let Some(pnode) = current_parent {
                let new_left_depth = pnode
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |x| x.borrow().depth + 1);
                let new_right_depth = pnode
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |x| x.borrow().depth + 1);
                let max_depth = if new_left_depth > new_right_depth {
                    new_left_depth
                } else {
                    new_right_depth
                };

                let old_depth = pnode.borrow().depth;
                if old_depth != max_depth {
                    pnode.borrow_mut().depth = max_depth;
                } else {
                    // No depth change here; we can break.
                    break;
                }

                // Check AVL violation
                if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                    // Handle left-right case
                    let left_child = left_of(&pnode).unwrap();
                    if balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    let new_root = rotate_right(self, &pnode);
                    current_parent = parent_of(&new_root);
                } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                    // Handle right-left case
                    let right_child = right_of(&pnode).unwrap();
                    if balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    let new_root = rotate_left(self, &pnode);
                    current_parent = parent_of(&new_root);
                } else {
                    current_parent = parent_of(&pnode);
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
            // Both children: find replacement
            let candidate;
            let lost_child: Option<Rc<RefCell<BOSNode>>>;
            let took_from_left;
            {
                let left_depth = node.borrow().left_child_node.as_ref().unwrap().borrow().depth;
                let right_depth = node
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .depth;
                if left_depth >= right_depth {
                    took_from_left = true;
                    node.borrow_mut().left_child_count -= 1;
                    let mut cand = node.borrow().left_child_node.as_ref().unwrap().clone();
                    while let Some(rc) = right_of(&cand) {
                        cand.borrow_mut().right_child_count -= 1;
                        cand = rc;
                    }
                    lost_child = left_of(&cand);
                    candidate = cand;
                } else {
                    took_from_left = false;
                    node.borrow_mut().right_child_count -= 1;
                    let mut cand = node.borrow().right_child_node.as_ref().unwrap().clone();
                    while let Some(lc) = left_of(&cand) {
                        cand.borrow_mut().left_child_count -= 1;
                        cand = lc;
                    }
                    lost_child = right_of(&cand);
                    candidate = cand;
                }
            }
            let _ = took_from_left;

            let bubble_start = parent_of(&candidate).expect("candidate must have a parent");

            // Reparent lost_child to take candidate's place
            {
                let mut bs = bubble_start.borrow_mut();
                let bs_left_is_cand = bs
                    .left_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, &candidate));
                if bs_left_is_cand {
                    bs.left_child_node = lost_child.clone();
                } else {
                    bs.right_child_node = lost_child.clone();
                }
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node used to be.
            let node_parent = parent_of(node);
            if let Some(ref np) = node_parent {
                let mut np_mut = np.borrow_mut();
                let left_is_node = np_mut
                    .left_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, node));
                if left_is_node {
                    np_mut.left_child_node = Some(candidate.clone());
                } else {
                    np_mut.right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }

            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            // Copy node's children pointers and counts to candidate.
            let (n_left, n_lcount, n_right, n_rcount) = {
                let nb = node.borrow();
                (
                    nb.left_child_node.clone(),
                    nb.left_child_count,
                    nb.right_child_node.clone(),
                    nb.right_child_count,
                )
            };
            {
                let mut c = candidate.borrow_mut();
                c.left_child_node = n_left.clone();
                c.left_child_count = n_lcount;
                c.right_child_node = n_right.clone();
                c.right_child_count = n_rcount;
            }
            if let Some(ref lc) = candidate.borrow().left_child_node.clone() {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref rc) = candidate.borrow().right_child_node.clone() {
                rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate.
            // bubble_start may equal node (when candidate was a direct child).
            // In that case, skip this rebalancing loop.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = Some(bubble_start.clone());
                while let Some(cur) = bs {
                    if Rc::ptr_eq(&cur, &candidate) {
                        break;
                    }
                    let new_depth = compute_depth(&cur);
                    cur.borrow_mut().depth = new_depth;
                    let bal = balance(&cur);
                    let new_root = if bal > 1 {
                        let right_child = right_of(&cur).unwrap();
                        if balance(&right_child) < 0 {
                            rotate_right(self, &right_child);
                        }
                        rotate_left(self, &cur)
                    } else if bal < -1 {
                        let left_child = left_of(&cur).unwrap();
                        if balance(&left_child) > 0 {
                            rotate_left(self, &left_child);
                        }
                        rotate_right(self, &cur)
                    } else {
                        cur.clone()
                    };
                    bs = parent_of(&new_root);
                }
            }

            // Fixup candidate's depth
            let new_cand_depth = compute_depth(&candidate);
            candidate.borrow_mut().depth = new_cand_depth;

            bubble_up = parent_of(&candidate);

            // Decrement immediate parent's child count.
            if let Some(ref bu) = bubble_up {
                let mut bu_mut = bu.borrow_mut();
                let left_is_cand = bu_mut
                    .left_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, &candidate));
                if left_is_cand {
                    bu_mut.left_child_count -= 1;
                } else {
                    bu_mut.right_child_count -= 1;
                }
            }
        } else {
            // At most one child
            let node_parent = parent_of(node);
            if node_parent.is_none() {
                if has_left {
                    let lc = left_of(node).unwrap();
                    self.root_node = Some(lc.clone());
                    lc.borrow_mut().parent_node = None;
                } else if has_right {
                    let rc = right_of(node).unwrap();
                    self.root_node = Some(rc.clone());
                    rc.borrow_mut().parent_node = None;
                } else {
                    self.root_node = None;
                }
                bubble_up = None;
            } else {
                let np = node_parent.unwrap();
                let (candidate, candidate_count) = if has_right {
                    (right_of(node), node.borrow().right_child_count)
                } else if has_left {
                    (left_of(node), node.borrow().left_child_count)
                } else {
                    (None, 0)
                };

                {
                    let mut np_mut = np.borrow_mut();
                    let left_is_node = np_mut
                        .left_child_node
                        .as_ref()
                        .map_or(false, |x| Rc::ptr_eq(x, node));
                    if left_is_node {
                        np_mut.left_child_node = candidate.clone();
                        np_mut.left_child_count = candidate_count;
                    } else {
                        np_mut.right_child_node = candidate.clone();
                        np_mut.right_child_count = candidate_count;
                    }
                }

                if let Some(ref c) = candidate {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&np));
                }

                bubble_up = Some(np);
            }
        }

        // Bubble up to fix child counts and depths and rebalance
        let mut bubbling_finished = false;
        let mut bu = bubble_up;
        while let Some(cur) = bu {
            let mut next_node_for_count = cur.clone();
            if !bubbling_finished {
                let left_depth = cur
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(0u32, |x| x.borrow().depth + 1);
                let right_depth = cur
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(0u32, |x| x.borrow().depth + 1);
                let new_depth = if left_depth > right_depth {
                    left_depth
                } else {
                    right_depth
                };
                let depth_changed = new_depth != cur.borrow().depth;
                cur.borrow_mut().depth = new_depth;

                let bal = balance(&cur);
                if bal < -1 {
                    let left_child = left_of(&cur).unwrap();
                    if balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    next_node_for_count = rotate_right(self, &cur);
                } else if bal > 1 {
                    let right_child = right_of(&cur).unwrap();
                    if balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    next_node_for_count = rotate_left(self, &cur);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Update parent's child counts
            let parent_opt = parent_of(&next_node_for_count);
            if let Some(ref p) = parent_opt {
                let mut pm = p.borrow_mut();
                let left_is_cur = pm
                    .left_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, &next_node_for_count));
                if left_is_cur {
                    pm.left_child_count -= 1;
                } else {
                    pm.right_child_count -= 1;
                }
            }
            bu = parent_opt;
        }

        // Mark node invalid and unref
        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach node's child links to break Rc cycles when freed.
        {
            let mut nb = node.borrow_mut();
            nb.left_child_node = None;
            nb.right_child_node = None;
            nb.parent_node = None;
        }
        let _ = self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let (count_after, valid) = {
            let mut n = node.borrow_mut();
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
            (n.weak_ref_count, n.weak_ref_node_valid)
        };
        if count_after == 0 {
            if let Some(ff) = self.free_function {
                ff(node);
            }
            // Drop our reference (Rc handles freeing automatically)
            return None;
        }
        if valid != 0 {
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
                current = n.borrow().left_child_node.clone();
            } else {
                current = n.borrow().right_child_node.clone();
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
                current = n.borrow().right_child_node.clone();
            } else {
                current = n.borrow().left_child_node.clone();
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
    if let Some(p) = n.parent_node.as_ref().and_then(|w| w.upgrade()) {
        println!("  {} -> {} [color=green];", n.key, p.borrow().key);
    }
    if let Some(ref l) = n.left_child_node {
        println!("  {} -> {}", n.key, l.borrow().key);
        print_helper(l);
    }
    if let Some(ref r) = n.right_child_node {
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
    if let Some(rc) = right_of(node) {
        let mut cur = rc;
        while let Some(lc) = left_of(&cur) {
            cur = lc;
        }
        return Some(cur);
    }
    let mut cur = node.clone();
    loop {
        let p = parent_of(&cur);
        match p {
            None => return None,
            Some(pn) => {
                let pn_right_is_cur = pn
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, &cur));
                if pn_right_is_cur {
                    cur = pn;
                } else {
                    return Some(pn);
                }
            }
        }
    }
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(lc) = left_of(node) {
        let mut cur = lc;
        while let Some(rc) = right_of(&cur) {
            cur = rc;
        }
        return Some(cur);
    }
    let mut cur = node.clone();
    loop {
        let p = parent_of(&cur);
        match p {
            None => return None,
            Some(pn) => {
                let pn_left_is_cur = pn
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |x| Rc::ptr_eq(x, &cur));
                if pn_left_is_cur {
                    cur = pn;
                } else {
                    return Some(pn);
                }
            }
        }
    }
}
/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        let parent = parent_of(&n);
        if let Some(ref p) = parent {
            let p_right_is_n = p
                .borrow()
                .right_child_node
                .as_ref()
                .map_or(false, |x| Rc::ptr_eq(x, &n));
            if p_right_is_n {
                counter += 1 + p.borrow().left_child_count;
            }
        }
        cur = parent;
    }
    counter
}
