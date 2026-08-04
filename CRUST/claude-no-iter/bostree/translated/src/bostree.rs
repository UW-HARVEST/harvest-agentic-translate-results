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

// ---------- Helper functions ----------

fn imax(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

fn opt_depth(opt: &Option<Rc<RefCell<BOSNode>>>) -> u32 {
    opt.as_ref().map_or(0, |n| n.borrow().depth + 1)
}

fn node_balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let left_depth = n
        .left_child_node
        .as_ref()
        .map_or(0, |l| l.borrow().depth as i32 + 1);
    let right_depth = n
        .right_child_node
        .as_ref()
        .map_or(0, |r| r.borrow().depth as i32 + 1);
    right_depth - left_depth
}

/// Determine whether `child` is the left child of `parent`.
/// Returns true for left, false for right.
fn parent_side_is_left(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |c| Rc::ptr_eq(c, child))
}

fn parent_of(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let l = p
        .borrow()
        .left_child_node
        .clone()
        .expect("rotate_right requires left child");

    // Re-link parent of P to point to L instead of P.
    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());
    if let Some(ref parent) = p_parent {
        if parent_side_is_left(parent, p) {
            parent.borrow_mut().left_child_node = Some(l.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    // L now takes P's parent.
    l.borrow_mut().parent_node = p_parent_weak;

    // P's new left child = L's old right child.
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;
    if let Some(ref l_right_node) = l_right {
        l_right_node.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    // Recompute P's depth.
    let pl = opt_depth(&p.borrow().left_child_node);
    let pr = opt_depth(&p.borrow().right_child_node);
    p.borrow_mut().depth = imax(pl, pr);

    // L's right child = P; P's parent = L.
    l.borrow_mut().right_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L's right_child_count = P's total + 1.
    let p_total = p.borrow().left_child_count + p.borrow().right_child_count + 1;
    l.borrow_mut().right_child_count = p_total;

    // Recompute L's depth.
    let ll = opt_depth(&l.borrow().left_child_node);
    let lr = opt_depth(&l.borrow().right_child_node);
    l.borrow_mut().depth = imax(ll, lr);

    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let r = p
        .borrow()
        .right_child_node
        .clone()
        .expect("rotate_left requires right child");

    let p_parent_weak = p.borrow().parent_node.clone();
    let p_parent = p_parent_weak.as_ref().and_then(|w| w.upgrade());
    if let Some(ref parent) = p_parent {
        if parent_side_is_left(parent, p) {
            parent.borrow_mut().left_child_node = Some(r.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent_weak;

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;
    if let Some(ref r_left_node) = r_left {
        r_left_node.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }

    let pl = opt_depth(&p.borrow().left_child_node);
    let pr = opt_depth(&p.borrow().right_child_node);
    p.borrow_mut().depth = imax(pl, pr);

    r.borrow_mut().left_child_node = Some(p.clone());
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    let p_total = p.borrow().left_child_count + p.borrow().right_child_count + 1;
    r.borrow_mut().left_child_count = p_total;

    let rl = opt_depth(&r.borrow().left_child_node);
    let rr = opt_depth(&r.borrow().right_child_node);
    r.borrow_mut().depth = imax(rl, rr);

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
            None => 0,
            Some(ref root) => {
                let r = root.borrow();
                r.left_child_count + r.right_child_count + 1
            }
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        let cmp = self.cmp_function;

        // Descend to find the insertion point, updating child counts on the way.
        let mut current_opt = self.root_node.clone();
        let mut parent_node: Option<Rc<RefCell<BOSNode>>> = None;
        let mut went_left = false;
        while let Some(current) = current_opt {
            parent_node = Some(current.clone());
            let c = cmp(&key, &current.borrow().key);
            if c < 0 {
                current.borrow_mut().left_child_count += 1;
                went_left = true;
                current_opt = current.borrow().left_child_node.clone();
            } else {
                current.borrow_mut().right_child_count += 1;
                went_left = false;
                current_opt = current.borrow().right_child_node.clone();
            }
        }

        // Create the new node with weak ref count 1, valid flag set.
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

        // Attach to parent (or set as root).
        let parent = match parent_node {
            None => {
                self.root_node = Some(new_node.clone());
                return new_node;
            }
            Some(p) => {
                if went_left {
                    p.borrow_mut().left_child_node = Some(new_node.clone());
                } else {
                    p.borrow_mut().right_child_node = Some(new_node.clone());
                }
                p
            }
        };

        // Determine if depth changed for the parent: only if this is the first
        // child of the parent (XOR of left and right children).
        let has_left = parent.borrow().left_child_node.is_some();
        let has_right = parent.borrow().right_child_node.is_some();
        if has_left ^ has_right {
            // Parent had no children before; now it has one. Depth bubbles up.
            parent.borrow_mut().depth += 1;

            let mut current = parent;
            loop {
                let next_parent = parent_of(&current);
                match next_parent {
                    None => break,
                    Some(p) => current = p,
                }

                let new_left = opt_depth(&current.borrow().left_child_node);
                let new_right = opt_depth(&current.borrow().right_child_node);
                let max_depth = imax(new_left, new_right);

                if current.borrow().depth != max_depth {
                    current.borrow_mut().depth = max_depth;
                } else {
                    // No depth change here means none above either.
                    break;
                }

                // AVL violation checks. Use signed math to avoid u32 underflow.
                let nl = new_left as i32;
                let nr = new_right as i32;
                if nl - 2 == nr {
                    // Left is two levels deeper than right; rotate right.
                    let left_child = current.borrow().left_child_node.clone().unwrap();
                    if node_balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    current = rotate_right(self, &current);
                } else if nl + 2 == nr {
                    // Right is two levels deeper than left; rotate left.
                    let right_child = current.borrow().right_child_node.clone().unwrap();
                    if node_balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    current = rotate_left(self, &current);
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
            // Find a candidate to replace `node` from the deeper subtree.
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
                    let r = c.borrow().right_child_node.clone();
                    match r {
                        Some(next) => {
                            c.borrow_mut().right_child_count -= 1;
                            c = next;
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
                    let l = c.borrow().left_child_node.clone();
                    match l {
                        Some(next) => {
                            c.borrow_mut().left_child_count -= 1;
                            c = next;
                        }
                        None => break,
                    }
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            // bubble_start = candidate's parent.
            let bubble_start = parent_of(&candidate).expect("candidate must have a parent");

            // Replace candidate with lost_child in bubble_start.
            if parent_side_is_left(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was.
            let node_parent_weak = node.borrow().parent_node.clone();
            let node_parent = node_parent_weak.as_ref().and_then(|w| w.upgrade());
            if let Some(ref np) = node_parent {
                if parent_side_is_left(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent_weak;

            // Move node's children/counts to candidate.
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
            if let Some(ref l) = node_left {
                if !Rc::ptr_eq(l, &candidate) {
                    l.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }
            if let Some(ref r) = node_right {
                if !Rc::ptr_eq(r, &candidate) {
                    r.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
                }
            }

            // Rebalance from bubble_start up to candidate (exclusive).
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start;
                while !Rc::ptr_eq(&bs, &candidate) {
                    let new_left = opt_depth(&bs.borrow().left_child_node);
                    let new_right = opt_depth(&bs.borrow().right_child_node);
                    bs.borrow_mut().depth = imax(new_left, new_right);
                    let balance = node_balance(&bs);
                    if balance > 1 {
                        let right_child = bs.borrow().right_child_node.clone().unwrap();
                        if node_balance(&right_child) < 0 {
                            rotate_right(self, &right_child);
                        }
                        bs = rotate_left(self, &bs);
                    } else if balance < -1 {
                        let left_child = bs.borrow().left_child_node.clone().unwrap();
                        if node_balance(&left_child) > 0 {
                            rotate_left(self, &left_child);
                        }
                        bs = rotate_right(self, &bs);
                    }
                    let next = parent_of(&bs).expect("inner rebalance must reach candidate");
                    bs = next;
                }
            }

            // Fix candidate's depth.
            let cl = opt_depth(&candidate.borrow().left_child_node);
            let cr = opt_depth(&candidate.borrow().right_child_node);
            candidate.borrow_mut().depth = imax(cl, cr);

            bubble_up = parent_of(&candidate);

            // Decrement bubble_up's child count (for the side candidate is on).
            if let Some(ref bu) = bubble_up {
                if parent_side_is_left(bu, &candidate) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Node has at most one child.
            let np = parent_of(node);
            match np {
                None => {
                    // Node was the root.
                    let left = node.borrow().left_child_node.clone();
                    let right = node.borrow().right_child_node.clone();
                    if let Some(l) = left {
                        l.borrow_mut().parent_node = None;
                        self.root_node = Some(l);
                    } else if let Some(r) = right {
                        r.borrow_mut().parent_node = None;
                        self.root_node = Some(r);
                    } else {
                        self.root_node = None;
                    }
                    bubble_up = None;
                }
                Some(np_rc) => {
                    let candidate;
                    let candidate_count;
                    if has_right {
                        candidate = node.borrow().right_child_node.clone();
                        candidate_count = node.borrow().right_child_count;
                    } else {
                        candidate = node.borrow().left_child_node.clone();
                        candidate_count = node.borrow().left_child_count;
                    }

                    if parent_side_is_left(&np_rc, node) {
                        np_rc.borrow_mut().left_child_node = candidate.clone();
                        np_rc.borrow_mut().left_child_count = candidate_count;
                    } else {
                        np_rc.borrow_mut().right_child_node = candidate.clone();
                        np_rc.borrow_mut().right_child_count = candidate_count;
                    }
                    if let Some(ref c) = candidate {
                        c.borrow_mut().parent_node = Some(Rc::downgrade(&np_rc));
                    }
                    bubble_up = Some(np_rc);
                }
            }
        }

        // Bubble depth/balance changes up the tree, decrementing child counts.
        let mut bubble_up_opt = bubble_up;
        let mut bubbling_finished = false;
        while let Some(mut bu) = bubble_up_opt {
            if !bubbling_finished {
                let left_depth = opt_depth(&bu.borrow().left_child_node);
                let right_depth = opt_depth(&bu.borrow().right_child_node);
                let new_depth = imax(left_depth, right_depth);
                let depth_changed = bu.borrow().depth != new_depth;
                bu.borrow_mut().depth = new_depth;

                let balance = node_balance(&bu);
                if balance < -1 {
                    let left_child = bu.borrow().left_child_node.clone().unwrap();
                    if node_balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    bu = rotate_right(self, &bu);
                } else if balance > 1 {
                    let right_child = bu.borrow().right_child_node.clone().unwrap();
                    if node_balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    bu = rotate_left(self, &bu);
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            // Decrement parent's child count for the side bu sits on.
            let p = parent_of(&bu);
            if let Some(ref pp) = p {
                if parent_side_is_left(pp, &bu) {
                    pp.borrow_mut().left_child_count -= 1;
                } else {
                    pp.borrow_mut().right_child_count -= 1;
                }
            }
            bubble_up_opt = p;
        }

        // Mark the removed node invalid and unref.
        node.borrow_mut().weak_ref_node_valid = 0;
        self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let mut should_free = false;
        let mut still_valid = false;
        {
            let mut n = node.borrow_mut();
            if n.weak_ref_count > 0 {
                n.weak_ref_count -= 1;
            }
            if n.weak_ref_count == 0 {
                should_free = true;
            } else if n.weak_ref_node_valid != 0 {
                still_valid = true;
            }
        }
        if should_free {
            if let Some(free_fn) = self.free_function {
                free_fn(node);
            }
            return None;
        }
        if still_valid {
            return Some(node.clone());
        }
        None
    }
    /// Lookup a node in the tree by its key.
    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let cmp = self.cmp_function;
        let mut node_opt = self.root_node.clone();
        while let Some(node) = node_opt {
            let c = cmp(key, &node.borrow().key);
            if c == 0 {
                return Some(node);
            } else if c < 0 {
                node_opt = node.borrow().left_child_node.clone();
            } else {
                node_opt = node.borrow().right_child_node.clone();
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut idx = index;
        let mut node_opt = self.root_node.clone();
        while let Some(node) = node_opt {
            let lcc = node.borrow().left_child_count;
            if lcc <= idx {
                idx -= lcc;
                if idx == 0 {
                    return Some(node);
                }
                idx -= 1;
                node_opt = node.borrow().right_child_node.clone();
            } else {
                node_opt = node.borrow().left_child_node.clone();
            }
        }
        None
    }
    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        fn helper(node: &Rc<RefCell<BOSNode>>) {
            {
                let n = node.borrow();
                println!(
                    "  {} [label=\"\\N ({},{},{})\"];",
                    n.key, n.left_child_count, n.right_child_count, n.depth
                );
                if let Some(ref pw) = n.parent_node {
                    if let Some(parent) = pw.upgrade() {
                        println!("  {} -> {} [color=green];", n.key, parent.borrow().key);
                    }
                }
            }
            let left = node.borrow().left_child_node.clone();
            if let Some(l) = left {
                println!("  {} -> {}", node.borrow().key, l.borrow().key);
                helper(&l);
            }
            let right = node.borrow().right_child_node.clone();
            if let Some(r) = right {
                println!("  {} -> {}", node.borrow().key, r.borrow().key);
                helper(&r);
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
    // If there's a right subtree, find its leftmost node.
    let right = node.borrow().right_child_node.clone();
    if let Some(r) = right {
        let mut current = r;
        loop {
            let left = current.borrow().left_child_node.clone();
            match left {
                Some(l) => current = l,
                None => return Some(current),
            }
        }
    }
    // Otherwise, walk up while the node is a right child.
    let mut current = node.clone();
    loop {
        let parent_opt = parent_of(&current);
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
    // If there's a left subtree, find its rightmost node.
    let left = node.borrow().left_child_node.clone();
    if let Some(l) = left {
        let mut current = l;
        loop {
            let right = current.borrow().right_child_node.clone();
            match right {
                Some(r) => current = r,
                None => return Some(current),
            }
        }
    }
    // Otherwise, walk up while the node is a left child.
    let mut current = node.clone();
    loop {
        let parent_opt = parent_of(&current);
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
        let parent_opt = parent_of(&current);
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
