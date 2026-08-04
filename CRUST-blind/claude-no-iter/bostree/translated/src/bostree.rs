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

// ---------- helper functions ----------

type NodeRef = Rc<RefCell<BOSNode>>;

fn parent_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn left_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().left_child_node.clone()
}

fn right_of(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().right_child_node.clone()
}

fn depth_of_child(child: &Option<NodeRef>) -> u32 {
    match child {
        Some(c) => c.borrow().depth + 1,
        None => 0,
    }
}

fn balance_of(node: &NodeRef) -> i32 {
    let n = node.borrow();
    let left_depth = match &n.left_child_node {
        Some(c) => c.borrow().depth as i32 + 1,
        None => 0,
    };
    let right_depth = match &n.right_child_node {
        Some(c) => c.borrow().depth as i32 + 1,
        None => 0,
    };
    right_depth - left_depth
}

fn recompute_depth(node: &NodeRef) {
    let n = node.borrow();
    let left_depth = match &n.left_child_node {
        Some(c) => c.borrow().depth + 1,
        None => 0,
    };
    let right_depth = match &n.right_child_node {
        Some(c) => c.borrow().depth + 1,
        None => 0,
    };
    drop(n);
    node.borrow_mut().depth = if left_depth > right_depth {
        left_depth
    } else {
        right_depth
    };
}

fn parent_left_eq(parent: &NodeRef, candidate: &NodeRef) -> bool {
    parent
        .borrow()
        .left_child_node
        .as_ref()
        .map_or(false, |l| Rc::ptr_eq(l, candidate))
}

fn rotate_right(tree: &mut BOSTree, p: &NodeRef) -> NodeRef {
    // P                     L
    //   L        R     -->    c1      P
    // c1 c2                        c2     R
    let l = left_of(p).expect("rotate_right: L must exist");
    let p_parent = parent_of(p);

    if let Some(parent) = &p_parent {
        if parent_left_eq(parent, p) {
            parent.borrow_mut().left_child_node = Some(l.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }

    l.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P->left_child_node = L->right_child_node
    let l_right = right_of(&l);
    let l_right_count = l.borrow().right_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.left_child_node = l_right.clone();
        p_mut.left_child_count = l_right_count;
    }
    if let Some(c) = &l_right {
        c.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }
    recompute_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    // L->right_child_node = P
    let p_left_count = p.borrow().left_child_count;
    let p_right_count = p.borrow().right_child_count;
    {
        let mut l_mut = l.borrow_mut();
        l_mut.right_child_node = Some(p.clone());
        l_mut.right_child_count = p_left_count + p_right_count + 1;
    }
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));
    recompute_depth(&l);

    l
}

fn rotate_left(tree: &mut BOSTree, p: &NodeRef) -> NodeRef {
    // P                     R
    //   L        R     -->    P      c2
    //         c1 c2        L  c1
    let r = right_of(p).expect("rotate_left: R must exist");
    let p_parent = parent_of(p);

    if let Some(parent) = &p_parent {
        if parent_left_eq(parent, p) {
            parent.borrow_mut().left_child_node = Some(r.clone());
        } else {
            parent.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }

    r.borrow_mut().parent_node = p_parent.as_ref().map(Rc::downgrade);

    // P->right_child_node = R->left_child_node
    let r_left = left_of(&r);
    let r_left_count = r.borrow().left_child_count;
    {
        let mut p_mut = p.borrow_mut();
        p_mut.right_child_node = r_left.clone();
        p_mut.right_child_count = r_left_count;
    }
    if let Some(c) = &r_left {
        c.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }
    recompute_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));

    // R->left_child_node = P
    let p_left_count = p.borrow().left_child_count;
    let p_right_count = p.borrow().right_child_count;
    {
        let mut r_mut = r.borrow_mut();
        r_mut.left_child_node = Some(p.clone());
        r_mut.left_child_count = p_left_count + p_right_count + 1;
    }
    p.borrow_mut().parent_node = Some(Rc::downgrade(&r));
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
            Some(r) => {
                let n = r.borrow();
                n.left_child_count + n.right_child_count + 1
            }
            None => 0,
        }
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find tree position to insert new node
        let mut current = self.root_node.clone();
        let mut parent_node: Option<NodeRef> = None;
        let mut went_left = false;

        while let Some(n) = current {
            parent_node = Some(n.clone());
            let cmp = (self.cmp_function)(&key, &n.borrow().key);
            if cmp < 0 {
                n.borrow_mut().left_child_count += 1;
                went_left = true;
                current = left_of(&n);
            } else {
                n.borrow_mut().right_child_count += 1;
                went_left = false;
                current = right_of(&n);
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

        match &parent_node {
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
            }
        }

        // Check if the depth changed with the new node:
        // It does only change if this is the first child of the parent
        let mut parent = parent_node.unwrap();
        let has_left = parent.borrow().left_child_node.is_some();
        let has_right = parent.borrow().right_child_node.is_some();
        if has_left != has_right {
            // Bubble information up
            parent.borrow_mut().depth += 1;
            loop {
                let next_parent = parent_of(&parent);
                let p = match next_parent {
                    Some(p) => p,
                    None => break,
                };
                parent = p;

                let new_left_depth = depth_of_child(&parent.borrow().left_child_node);
                let new_right_depth = depth_of_child(&parent.borrow().right_child_node);
                let max_depth = if new_left_depth > new_right_depth {
                    new_left_depth
                } else {
                    new_right_depth
                };

                let cur_depth = parent.borrow().depth;
                if cur_depth != max_depth {
                    parent.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                // AVL violation check
                if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                    // Left-right case
                    let left_child = left_of(&parent).unwrap();
                    if balance_of(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    parent = rotate_right(self, &parent);
                } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                    // Right-left case
                    let right_child = right_of(&parent).unwrap();
                    if balance_of(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    parent = rotate_left(self, &parent);
                }
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
            // Two children case
            let left_depth = node.borrow().left_child_node.as_ref().unwrap().borrow().depth;
            let right_depth = node
                .borrow()
                .right_child_node
                .as_ref()
                .unwrap()
                .borrow()
                .depth;

            let candidate: NodeRef;
            let lost_child: Option<NodeRef>;

            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut c = left_of(node).unwrap();
                while let Some(r) = right_of(&c) {
                    c.borrow_mut().right_child_count -= 1;
                    c = r;
                }
                lost_child = left_of(&c);
                candidate = c;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut c = right_of(node).unwrap();
                while let Some(l) = left_of(&c) {
                    c.borrow_mut().left_child_count -= 1;
                    c = l;
                }
                lost_child = right_of(&c);
                candidate = c;
            }

            let bubble_start = parent_of(&candidate).expect("candidate must have a parent");

            // Detach candidate from its current spot, replace with lost_child
            if parent_left_eq(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(lc) = &lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            // Anchor candidate where node was
            let node_parent = parent_of(node);
            if let Some(np) = &node_parent {
                if parent_left_eq(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(Rc::downgrade);

            // Copy node's child structure into candidate
            let n_left = left_of(node);
            let n_right = right_of(node);
            let n_lcc = node.borrow().left_child_count;
            let n_rcc = node.borrow().right_child_count;
            {
                let mut c_mut = candidate.borrow_mut();
                c_mut.left_child_node = n_left.clone();
                c_mut.left_child_count = n_lcc;
                c_mut.right_child_node = n_right.clone();
                c_mut.right_child_count = n_rcc;
            }
            if let Some(lc) = &n_left {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(rc) = &n_right {
                rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Now node is out. Rebalance from bubble_start up to candidate.
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start.clone();
                while !Rc::ptr_eq(&bs, &candidate) {
                    recompute_depth(&bs);
                    let balance = balance_of(&bs);
                    if balance > 1 {
                        let right_child = right_of(&bs).unwrap();
                        if balance_of(&right_child) < 0 {
                            rotate_right(self, &right_child);
                        }
                        bs = rotate_left(self, &bs);
                    } else if balance < -1 {
                        let left_child = left_of(&bs).unwrap();
                        if balance_of(&left_child) > 0 {
                            rotate_left(self, &left_child);
                        }
                        bs = rotate_right(self, &bs);
                    }
                    let next = parent_of(&bs);
                    bs = match next {
                        Some(n) => n,
                        None => break,
                    };
                }
            }

            // Fix up candidate's depth
            recompute_depth(&candidate);

            // We'll have to fix up child counts up to root
            bubble_up = parent_of(&candidate);

            if let Some(bu) = &bubble_up {
                if parent_left_eq(bu, &candidate) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            // Zero or one child
            let node_parent = parent_of(node);
            if node_parent.is_none() {
                // node was root
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

                if parent_left_eq(&np, node) {
                    np.borrow_mut().left_child_node = candidate.clone();
                    np.borrow_mut().left_child_count = candidate_count;
                } else {
                    np.borrow_mut().right_child_node = candidate.clone();
                    np.borrow_mut().right_child_count = candidate_count;
                }

                if let Some(c) = &candidate {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&np));
                }

                bubble_up = Some(np);
            }
        }

        // Bubble up: rebalance up to root.
        let mut bubbling_finished = false;
        while let Some(bu) = bubble_up.clone() {
            if !bubbling_finished {
                let left_depth = depth_of_child(&bu.borrow().left_child_node);
                let right_depth = depth_of_child(&bu.borrow().right_child_node);
                let new_depth = if left_depth > right_depth {
                    left_depth
                } else {
                    right_depth
                };
                let depth_changed = new_depth != bu.borrow().depth;
                bu.borrow_mut().depth = new_depth;

                let balance = balance_of(&bu);
                let mut new_bu = bu.clone();
                if balance < -1 {
                    let lc = left_of(&bu).unwrap();
                    if balance_of(&lc) > 0 {
                        rotate_left(self, &lc);
                    }
                    new_bu = rotate_right(self, &bu);
                } else if balance > 1 {
                    let rc = right_of(&bu).unwrap();
                    if balance_of(&rc) < 0 {
                        rotate_right(self, &rc);
                    }
                    new_bu = rotate_left(self, &bu);
                } else if !depth_changed {
                    bubbling_finished = true;
                }

                // Decrement child count on parent of new_bu
                if let Some(parent) = parent_of(&new_bu) {
                    if parent_left_eq(&parent, &new_bu) {
                        parent.borrow_mut().left_child_count -= 1;
                    } else {
                        parent.borrow_mut().right_child_count -= 1;
                    }
                }
                bubble_up = parent_of(&new_bu);
            } else {
                if let Some(parent) = parent_of(&bu) {
                    if parent_left_eq(&parent, &bu) {
                        parent.borrow_mut().left_child_count -= 1;
                    } else {
                        parent.borrow_mut().right_child_count -= 1;
                    }
                }
                bubble_up = parent_of(&bu);
            }
        }

        // Detach the removed node from its parent/child links to avoid retaining references
        node.borrow_mut().parent_node = None;
        node.borrow_mut().left_child_node = None;
        node.borrow_mut().right_child_node = None;
        node.borrow_mut().weak_ref_node_valid = 0;
        self.bostree_node_weak_unref(node);
    }
    /// Decrease the weak reference count for a node; if that was the last reference,
    /// free the node and return None; otherwise return the node.
    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let cnt = {
            let mut n = node.borrow_mut();
            n.weak_ref_count = n.weak_ref_count.saturating_sub(1);
            n.weak_ref_count
        };
        if cnt == 0 {
            if let Some(ff) = self.free_function {
                ff(node);
            }
            return None;
        } else if node.borrow().weak_ref_node_valid != 0 {
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
                current = left_of(&n);
            } else {
                current = right_of(&n);
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut idx = index;
        let mut current = self.root_node.clone();
        while let Some(n) = current {
            let lcc = n.borrow().left_child_count;
            if lcc <= idx {
                idx -= lcc;
                if idx == 0 {
                    return Some(n);
                }
                idx -= 1;
                current = right_of(&n);
            } else {
                current = left_of(&n);
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
    let n = node.borrow();
    println!(
        "  {} [label=\"\\N ({},{},{})\"];",
        n.key, n.left_child_count, n.right_child_count, n.depth
    );
    if let Some(parent) = n.parent_node.as_ref().and_then(|w| w.upgrade()) {
        println!("  {} -> {} [color=green];", n.key, parent.borrow().key);
    }
    if let Some(left) = &n.left_child_node {
        println!("  {} -> {}", n.key, left.borrow().key);
        let l = left.clone();
        drop(n);
        print_helper(&l);
        return;
    }
    if let Some(right) = &n.right_child_node {
        println!("  {} -> {}", n.key, right.borrow().key);
        let r = right.clone();
        drop(n);
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
    if let Some(right) = right_of(node) {
        let mut cur = right;
        while let Some(l) = left_of(&cur) {
            cur = l;
        }
        return Some(cur);
    }

    let mut cur = node.clone();
    loop {
        let parent = parent_of(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let is_right_child = p
                    .borrow()
                    .right_child_node
                    .as_ref()
                    .map_or(false, |rc| Rc::ptr_eq(rc, &cur));
                if is_right_child {
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
    if let Some(left) = left_of(node) {
        let mut cur = left;
        while let Some(r) = right_of(&cur) {
            cur = r;
        }
        return Some(cur);
    }

    let mut cur = node.clone();
    loop {
        let parent = parent_of(&cur);
        match parent {
            None => return None,
            Some(p) => {
                let is_left_child = p
                    .borrow()
                    .left_child_node
                    .as_ref()
                    .map_or(false, |lc| Rc::ptr_eq(lc, &cur));
                if is_left_child {
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
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        if let Some(p) = parent_of(&n) {
            let is_right_child = p
                .borrow()
                .right_child_node
                .as_ref()
                .map_or(false, |rc| Rc::ptr_eq(rc, &n));
            if is_right_child {
                counter += 1 + p.borrow().left_child_count;
            }
            cur = Some(p);
        } else {
            cur = None;
        }
    }
    counter
}
