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

type NodeRef = Rc<RefCell<BOSNode>>;

// --- Helper functions ---

fn get_parent(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn is_left_child_of(node: &NodeRef, parent: &NodeRef) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |n| Rc::ptr_eq(n, node))
}

fn left_depth(node: &NodeRef) -> u32 {
    node.borrow()
        .left_child_node
        .as_ref()
        .map_or(0, |n| n.borrow().depth + 1)
}

fn right_depth(node: &NodeRef) -> u32 {
    node.borrow()
        .right_child_node
        .as_ref()
        .map_or(0, |n| n.borrow().depth + 1)
}

fn balance_of(node: &NodeRef) -> i32 {
    right_depth(node) as i32 - left_depth(node) as i32
}

fn recompute_depth(node: &NodeRef) {
    let d = left_depth(node).max(right_depth(node));
    node.borrow_mut().depth = d;
}

fn rotate_right(tree: &mut BOSTree, p: &NodeRef) -> NodeRef {
    // P -> L (left child becomes new root of subtree)
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires left child");
    let p_parent = get_parent(p);

    match &p_parent {
        Some(pp) => {
            let is_left = is_left_child_of(p, pp);
            if is_left {
                pp.borrow_mut().left_child_node = Some(Rc::clone(&l));
            } else {
                pp.borrow_mut().right_child_node = Some(Rc::clone(&l));
            }
        }
        None => {
            tree.root_node = Some(Rc::clone(&l));
        }
    }

    l.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P's left becomes L's right
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;
    if let Some(ref lr) = l_right {
        lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    recompute_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L's right = P
    l.borrow_mut().right_child_node = Some(Rc::clone(p));
    let p_lc = p.borrow().left_child_count;
    let p_rc = p.borrow().right_child_count;
    l.borrow_mut().right_child_count = p_lc + p_rc + 1;

    recompute_depth(&l);

    l
}

fn rotate_left(tree: &mut BOSTree, p: &NodeRef) -> NodeRef {
    // P -> R (right child becomes new root of subtree)
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires right child");
    let p_parent = get_parent(p);

    match &p_parent {
        Some(pp) => {
            let is_left = is_left_child_of(p, pp);
            if is_left {
                pp.borrow_mut().left_child_node = Some(Rc::clone(&r));
            } else {
                pp.borrow_mut().right_child_node = Some(Rc::clone(&r));
            }
        }
        None => {
            tree.root_node = Some(Rc::clone(&r));
        }
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P's right becomes R's left
    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;
    if let Some(ref rl) = r_left {
        rl.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    recompute_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    // R's left = P
    r.borrow_mut().left_child_node = Some(Rc::clone(p));
    let p_lc = p.borrow().left_child_count;
    let p_rc = p.borrow().right_child_count;
    r.borrow_mut().left_child_count = p_lc + p_rc + 1;

    recompute_depth(&r);

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
            Some(r) => {
                let r = r.borrow();
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
            self.root_node = Some(Rc::clone(&new_node));
            return new_node;
        }

        // Walk down to find insertion point, incrementing counts along the way.
        let mut current = Rc::clone(self.root_node.as_ref().unwrap());
        let parent_node;
        let went_left;
        loop {
            let cmp = (self.cmp_function)(&key, &current.borrow().key);
            if cmp < 0 {
                current.borrow_mut().left_child_count += 1;
                let next = current.borrow().left_child_node.clone();
                match next {
                    Some(n) => current = n,
                    None => {
                        parent_node = current;
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
                        parent_node = current;
                        went_left = false;
                        break;
                    }
                }
            }
        }

        // Anchor new_node into parent_node.
        new_node.borrow_mut().parent_node = Some(Rc::downgrade(&parent_node));
        if went_left {
            parent_node.borrow_mut().left_child_node = Some(Rc::clone(&new_node));
        } else {
            parent_node.borrow_mut().right_child_node = Some(Rc::clone(&new_node));
        }

        // Check if depth changed for parent (only if it had no other children).
        let only_one_child = {
            let p = parent_node.borrow();
            p.left_child_node.is_some() ^ p.right_child_node.is_some()
        };
        if !only_one_child {
            // Parent already had a sibling child, depth unchanged at parent.
            return new_node;
        }

        parent_node.borrow_mut().depth = 1;

        let mut current_p = parent_node;

        while let Some(pp) = get_parent(&current_p) {
            current_p = pp;

            let new_left_depth = left_depth(&current_p);
            let new_right_depth = right_depth(&current_p);
            let max_depth = new_left_depth.max(new_right_depth);

            let cur_depth = current_p.borrow().depth;
            if cur_depth != max_depth {
                current_p.borrow_mut().depth = max_depth;
            } else {
                break;
            }

            // AVL rotations.
            if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                // Left subtree too deep.
                let left = current_p.borrow().left_child_node.clone();
                if let Some(left) = left {
                    if balance_of(&left) > 0 {
                        rotate_left(self, &left);
                    }
                }
                current_p = rotate_right(self, &current_p);
            } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                // Right subtree too deep.
                let right = current_p.borrow().right_child_node.clone();
                if let Some(right) = right {
                    if balance_of(&right) < 0 {
                        rotate_right(self, &right);
                    }
                }
                current_p = rotate_left(self, &current_p);
            }
        }

        new_node
    }

    /// Remove a given node from the tree.
    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<NodeRef>;

        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        if has_left && has_right {
            // Both children: find replacement candidate.
            let left_child = node.borrow().left_child_node.clone().unwrap();
            let right_child = node.borrow().right_child_node.clone().unwrap();
            let left_d = left_child.borrow().depth;
            let right_d = right_child.borrow().depth;

            let candidate;
            let lost_child;
            let from_left;
            if left_d >= right_d {
                node.borrow_mut().left_child_count -= 1;
                let mut cand = left_child;
                while cand.borrow().right_child_node.is_some() {
                    cand.borrow_mut().right_child_count -= 1;
                    let next = cand.borrow().right_child_node.clone().unwrap();
                    cand = next;
                }
                lost_child = cand.borrow().left_child_node.clone();
                candidate = cand;
                from_left = true;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut cand = right_child;
                while cand.borrow().left_child_node.is_some() {
                    cand.borrow_mut().left_child_count -= 1;
                    let next = cand.borrow().left_child_node.clone().unwrap();
                    cand = next;
                }
                lost_child = cand.borrow().right_child_node.clone();
                candidate = cand;
                from_left = false;
            }

            let bubble_start = get_parent(&candidate).expect("candidate must have parent");

            // Detach candidate from bubble_start
            if is_left_child_of(&candidate, &bubble_start) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }
            let _ = from_left; // silence unused

            // Anchor candidate into the place where node used to be.
            let node_parent = get_parent(node);
            match &node_parent {
                Some(np) => {
                    if is_left_child_of(node, np) {
                        np.borrow_mut().left_child_node = Some(Rc::clone(&candidate));
                    } else {
                        np.borrow_mut().right_child_node = Some(Rc::clone(&candidate));
                    }
                }
                None => {
                    self.root_node = Some(Rc::clone(&candidate));
                }
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            // Take node's children
            let n_left = node.borrow().left_child_node.clone();
            let n_right = node.borrow().right_child_node.clone();
            let n_lc = node.borrow().left_child_count;
            let n_rc = node.borrow().right_child_count;
            candidate.borrow_mut().left_child_node = n_left.clone();
            candidate.borrow_mut().left_child_count = n_lc;
            candidate.borrow_mut().right_child_node = n_right.clone();
            candidate.borrow_mut().right_child_count = n_rc;

            if let Some(ref c) = n_left {
                if !Rc::ptr_eq(c, &candidate) {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }
            if let Some(ref c) = n_right {
                if !Rc::ptr_eq(c, &candidate) {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }

            // Rebalance up to candidate from bubble_start.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut cur = bubble_start;
                while !Rc::ptr_eq(&cur, &candidate) {
                    recompute_depth(&cur);
                    let bal = balance_of(&cur);
                    if bal > 1 {
                        let r = cur.borrow().right_child_node.clone();
                        if let Some(rch) = r {
                            if balance_of(&rch) < 0 {
                                rotate_right(self, &rch);
                            }
                        }
                        cur = rotate_left(self, &cur);
                    } else if bal < -1 {
                        let l = cur.borrow().left_child_node.clone();
                        if let Some(lch) = l {
                            if balance_of(&lch) > 0 {
                                rotate_left(self, &lch);
                            }
                        }
                        cur = rotate_right(self, &cur);
                    }
                    let next = get_parent(&cur);
                    match next {
                        Some(n) => cur = n,
                        None => break,
                    }
                }
            }

            recompute_depth(&candidate);
            bubble_up = get_parent(&candidate);

            if let Some(ref bu) = bubble_up {
                if is_left_child_of(&candidate, bu) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Single-side or no children.
            let node_parent = get_parent(node);
            match node_parent {
                None => {
                    // Node is root.
                    let n_left = node.borrow().left_child_node.clone();
                    let n_right = node.borrow().right_child_node.clone();
                    if n_left.is_some() {
                        self.root_node = n_left.clone();
                        if let Some(ref c) = n_left {
                            c.borrow_mut().parent_node = None;
                        }
                    } else {
                        self.root_node = n_right.clone();
                        if let Some(ref c) = n_right {
                            c.borrow_mut().parent_node = None;
                        }
                    }
                    bubble_up = None;
                }
                Some(np) => {
                    let candidate;
                    let candidate_count;
                    if let Some(rc) = node.borrow().right_child_node.clone() {
                        candidate_count = node.borrow().right_child_count;
                        candidate = Some(rc);
                    } else {
                        candidate_count = node.borrow().left_child_count;
                        candidate = node.borrow().left_child_node.clone();
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
        }

        // Bubble up: rebalance and update child counts to the root.
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up.clone() {
            if !bubbling_finished {
                let l_d = left_depth(&bu);
                let r_d = right_depth(&bu);
                let new_depth = l_d.max(r_d);
                let depth_changed = new_depth != bu.borrow().depth;
                bu.borrow_mut().depth = new_depth;

                let bal = balance_of(&bu);
                if bal < -1 {
                    let l = bu.borrow().left_child_node.clone();
                    if let Some(lch) = l {
                        if balance_of(&lch) > 0 {
                            rotate_left(self, &lch);
                        }
                    }
                    let new_top = rotate_right(self, &bu);
                    bubble_up = Some(new_top);
                } else if bal > 1 {
                    let r = bu.borrow().right_child_node.clone();
                    if let Some(rch) = r {
                        if balance_of(&rch) < 0 {
                            rotate_right(self, &rch);
                        }
                    }
                    let new_top = rotate_left(self, &bu);
                    bubble_up = Some(new_top);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Re-fetch since rotation may have changed bubble_up.
            let cur = bubble_up.clone().unwrap();
            let cur_parent = get_parent(&cur);
            if let Some(ref cp) = cur_parent {
                if is_left_child_of(&cur, cp) {
                    cp.borrow_mut().left_child_count -= 1;
                } else {
                    cp.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up = cur_parent;
        }

        // Mark node invalid and weak-unref it.
        node.borrow_mut().weak_ref_node_valid = 0;
        // Detach node from tree-related references so it can be dropped.
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
        {
            let mut n = node.borrow_mut();
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
        }
        let count = node.borrow().weak_ref_count;
        if count == 0 {
            if let Some(ff) = self.free_function {
                ff(node);
            }
            return None;
        } else if node.borrow().weak_ref_node_valid != 0 {
            return Some(Rc::clone(node));
        }
        None
    }

    /// Lookup a node in the tree by its key.
    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let mut cur = self.root_node.clone();
        while let Some(n) = cur {
            let cmp = (self.cmp_function)(key, &n.borrow().key);
            if cmp == 0 {
                return Some(n);
            } else if cmp < 0 {
                let next = n.borrow().left_child_node.clone();
                cur = next;
            } else {
                let next = n.borrow().right_child_node.clone();
                cur = next;
            }
        }
        None
    }

    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut idx = index;
        let mut cur = self.root_node.clone();
        while let Some(n) = cur {
            let lc = n.borrow().left_child_count;
            if lc <= idx {
                idx -= lc;
                if idx == 0 {
                    return Some(n);
                }
                idx -= 1;
                let next = n.borrow().right_child_node.clone();
                cur = next;
            } else {
                let next = n.borrow().left_child_node.clone();
                cur = next;
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
            print_helper(root);
            println!("}}");
        }
    }
}

#[cfg(debug_assertions)]
fn print_helper(node: &NodeRef) {
    let n = node.borrow();
    println!(
        "  {} [label=\"\\N ({},{},{})\"];",
        n.key, n.left_child_count, n.right_child_count, n.depth
    );
    if let Some(ref p) = n.parent_node {
        if let Some(parent) = p.upgrade() {
            println!("  {} -> {} [color=green];", n.key, parent.borrow().key);
        }
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
    Rc::clone(node)
}

/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if let Some(r) = node.borrow().right_child_node.clone() {
        let mut cur = r;
        loop {
            let next = cur.borrow().left_child_node.clone();
            match next {
                Some(n) => cur = n,
                None => return Some(cur),
            }
        }
    }

    // Walk up while we are a right child.
    let mut cur = Rc::clone(node);
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
                if is_right {
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
    if let Some(l) = node.borrow().left_child_node.clone() {
        let mut cur = l;
        loop {
            let next = cur.borrow().right_child_node.clone();
            match next {
                Some(n) => cur = n,
                None => return Some(cur),
            }
        }
    }

    let mut cur = Rc::clone(node);
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let is_left = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
                if is_left {
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
    let mut cur = Rc::clone(node);
    loop {
        let parent = get_parent(&cur);
        match parent {
            None => return counter,
            Some(p) => {
                let is_right = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
                if is_right {
                    counter += 1 + p.borrow().left_child_count;
                }
                cur = p;
            }
        }
    }
}
