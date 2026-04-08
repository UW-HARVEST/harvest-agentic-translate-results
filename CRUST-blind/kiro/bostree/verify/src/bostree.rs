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

fn imax(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

fn bostree_balance(node: &RefCell<BOSNode>) -> i32 {
    let n = node.borrow();
    let left_depth = n.left_child_node.as_ref().map_or(0i32, |l| l.borrow().depth as i32 + 1);
    let right_depth = n.right_child_node.as_ref().map_or(0i32, |r| r.borrow().depth as i32 + 1);
    right_depth - left_depth
}

fn update_depth(node: &RefCell<BOSNode>) {
    let mut n = node.borrow_mut();
    let ld = n.left_child_node.as_ref().map_or(0, |l| l.borrow().depth + 1);
    let rd = n.right_child_node.as_ref().map_or(0, |r| r.borrow().depth + 1);
    n.depth = imax(ld as i32, rd as i32) as u32;
}

fn get_parent(node: &RefCell<BOSNode>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn set_parent(node: &Rc<RefCell<BOSNode>>, parent: Option<&Rc<RefCell<BOSNode>>>) {
    node.borrow_mut().parent_node = parent.map(|p| Rc::downgrade(p));
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let l = p.borrow().left_child_node.as_ref().unwrap().clone();

    // Fix parent linkage
    let parent = get_parent(&p);
    if let Some(ref par) = parent {
        let mut pb = par.borrow_mut();
        if pb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, p)) {
            pb.left_child_node = Some(l.clone());
        } else {
            pb.right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }
    l.borrow_mut().parent_node = parent.as_ref().map(|par| Rc::downgrade(par));

    // Move L's right child to P's left
    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;
    if let Some(ref lr) = l_right {
        set_parent(lr, Some(p));
    }
    update_depth(&p);
    set_parent(p, Some(&l));

    // Set P as L's right child
    l.borrow_mut().right_child_node = Some(p.clone());
    {
        let pb = p.borrow();
        l.borrow_mut().right_child_count = pb.left_child_count + pb.right_child_count + 1;
    }
    update_depth(&l);

    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let r = p.borrow().right_child_node.as_ref().unwrap().clone();

    let parent = get_parent(&p);
    if let Some(ref par) = parent {
        let mut pb = par.borrow_mut();
        if pb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, p)) {
            pb.left_child_node = Some(r.clone());
        } else {
            pb.right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }
    r.borrow_mut().parent_node = parent.as_ref().map(|par| Rc::downgrade(par));

    let r_left = r.borrow().left_child_node.clone();
    let r_left_count = r.borrow().left_child_count;
    p.borrow_mut().right_child_node = r_left.clone();
    p.borrow_mut().right_child_count = r_left_count;
    if let Some(ref rl) = r_left {
        set_parent(rl, Some(p));
    }
    update_depth(&p);
    set_parent(p, Some(&r));

    r.borrow_mut().left_child_node = Some(p.clone());
    {
        let pb = p.borrow();
        r.borrow_mut().left_child_count = pb.left_child_count + pb.right_child_count + 1;
    }
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
        self.root_node.as_ref().map_or(0, |r| {
            let b = r.borrow();
            b.left_child_count + b.right_child_count + 1
        })
    }
    /// Insert a new member into the tree and return the associated node.
    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        // Find insertion point
        let mut current = self.root_node.clone();
        let mut parent: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left_path: Vec<(Rc<RefCell<BOSNode>>, bool)> = Vec::new();

        while let Some(ref node) = current {
            let cmp = (self.cmp_function)(&key, &node.borrow().key);
            parent = Some(node.clone());
            if cmp < 0 {
                go_left_path.push((node.clone(), true));
                let next = node.borrow().left_child_node.clone();
                current = next;
            } else {
                go_left_path.push((node.clone(), false));
                let next = node.borrow().right_child_node.clone();
                current = next;
            }
        }

        // Update child counts along the path
        for (n, is_left) in &go_left_path {
            if *is_left {
                n.borrow_mut().left_child_count += 1;
            } else {
                n.borrow_mut().right_child_count += 1;
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0,
            right_child_count: 0,
            depth: 0,
            left_child_node: None,
            right_child_node: None,
            parent_node: parent.as_ref().map(|p| Rc::downgrade(p)),
            key,
            data,
            weak_ref_count: 1,
            weak_ref_node_valid: 1,
        }));

        // Attach to parent
        if let Some(ref par) = parent {
            let last = go_left_path.last().unwrap();
            if last.1 {
                par.borrow_mut().left_child_node = Some(new_node.clone());
            } else {
                par.borrow_mut().right_child_node = Some(new_node.clone());
            }
        } else {
            self.root_node = Some(new_node.clone());
            return new_node;
        }

        // Check if depth changed: only if parent now has exactly one child
        let par = parent.unwrap();
        let has_left = par.borrow().left_child_node.is_some();
        let has_right = par.borrow().right_child_node.is_some();
        if has_left ^ has_right {
            // Only one child means depth changed
            par.borrow_mut().depth += 1;
            let mut current_parent = get_parent(&par);
            while let Some(pp) = current_parent {
                let new_left_depth = pp.borrow().left_child_node.as_ref().map_or(0u32, |l| l.borrow().depth + 1);
                let new_right_depth = pp.borrow().right_child_node.as_ref().map_or(0u32, |r| r.borrow().depth + 1);
                let max_depth = imax(new_left_depth as i32, new_right_depth as i32) as u32;

                if pp.borrow().depth != max_depth {
                    pp.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                    // Left-right case
                    {
                        let left = pp.borrow().left_child_node.as_ref().unwrap().clone();
                        if bostree_balance(&left) > 0 {
                            rotate_left(self, &left);
                        }
                    }
                    let rotated = rotate_right(self, &pp);
                    current_parent = get_parent(&rotated);
                } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                    // Right-left case
                    {
                        let right = pp.borrow().right_child_node.as_ref().unwrap().clone();
                        if bostree_balance(&right) < 0 {
                            rotate_right(self, &right);
                        }
                    }
                    let rotated = rotate_left(self, &pp);
                    current_parent = get_parent(&rotated);
                } else {
                    current_parent = get_parent(&pp);
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
            // Both children exist
            let left_depth = node.borrow().left_child_node.as_ref().unwrap().borrow().depth;
            let right_depth = node.borrow().right_child_node.as_ref().unwrap().borrow().depth;

            let (candidate, lost_child);
            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut c = node.borrow().left_child_node.as_ref().unwrap().clone();
                while c.borrow().right_child_node.is_some() {
                    c.borrow_mut().right_child_count -= 1;
                    let next = c.borrow().right_child_node.as_ref().unwrap().clone();
                    c = next;
                }
                lost_child = c.borrow().left_child_node.clone();
                candidate = c;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut c = node.borrow().right_child_node.as_ref().unwrap().clone();
                while c.borrow().left_child_node.is_some() {
                    c.borrow_mut().left_child_count -= 1;
                    let next = c.borrow().left_child_node.as_ref().unwrap().clone();
                    c = next;
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            let bubble_start = get_parent(&candidate).unwrap();
            {
                let mut bsb = bubble_start.borrow_mut();
                if bsb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, &candidate)) {
                    bsb.left_child_node = lost_child.clone();
                } else {
                    bsb.right_child_node = lost_child.clone();
                }
            }
            if let Some(ref lc) = lost_child {
                set_parent(lc, Some(&bubble_start));
            }

            // Anchor candidate in node's place
            let node_parent = get_parent(node);
            if let Some(ref np) = node_parent {
                let mut npb = np.borrow_mut();
                if npb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, node)) {
                    npb.left_child_node = Some(candidate.clone());
                } else {
                    npb.right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(|p| Rc::downgrade(p));

            {
                let nb = node.borrow();
                let mut cb = candidate.borrow_mut();
                cb.left_child_node = nb.left_child_node.clone();
                cb.left_child_count = nb.left_child_count;
                cb.right_child_node = nb.right_child_node.clone();
                cb.right_child_count = nb.right_child_count;
            }

            if let Some(ref left) = candidate.borrow().left_child_node.clone() {
                set_parent(left, Some(&candidate));
            }
            if let Some(ref right) = candidate.borrow().right_child_node.clone() {
                set_parent(right, Some(&candidate));
            }

            // Rebalance from bubble_start up to candidate
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start.clone();
                while !Rc::ptr_eq(&bs, &candidate) {
                    update_depth(&bs);
                    let balance = bostree_balance(&bs);
                    if balance > 1 {
                        {
                            let right = bs.borrow().right_child_node.as_ref().unwrap().clone();
                            if bostree_balance(&right) < 0 {
                                rotate_right(self, &right);
                            }
                        }
                        let rotated = rotate_left(self, &bs);
                        bs = get_parent(&rotated).unwrap_or(rotated);
                        continue;
                    } else if balance < -1 {
                        {
                            let left = bs.borrow().left_child_node.as_ref().unwrap().clone();
                            if bostree_balance(&left) > 0 {
                                rotate_left(self, &left);
                            }
                        }
                        let rotated = rotate_right(self, &bs);
                        bs = get_parent(&rotated).unwrap_or(rotated);
                        continue;
                    }
                    bs = get_parent(&bs).unwrap_or(bs.clone());
                    // Safety: if we reach candidate, the while condition will stop
                    if Rc::ptr_eq(&bs, &candidate) || get_parent(&bs).is_none() {
                        break;
                    }
                }
            }

            // Fix candidate depth
            update_depth(&candidate);

            bubble_up = get_parent(&candidate);

            // Fix immediate parent child count
            if let Some(ref bu) = bubble_up {
                let mut bub = bu.borrow_mut();
                if bub.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, &candidate)) {
                    bub.left_child_count -= 1;
                } else {
                    bub.right_child_count -= 1;
                }
            }
        } else {
            // One or zero children
            let node_parent = get_parent(node);
            if node_parent.is_none() {
                // Node was root
                if node.borrow().left_child_node.is_some() {
                    let left = node.borrow().left_child_node.clone();
                    self.root_node = left.clone();
                    if let Some(ref l) = left {
                        l.borrow_mut().parent_node = None;
                    }
                } else {
                    let right = node.borrow().right_child_node.clone();
                    self.root_node = right.clone();
                    if let Some(ref r) = right {
                        r.borrow_mut().parent_node = None;
                    }
                }
                bubble_up = None;
            } else {
                let cand = if node.borrow().right_child_node.is_some() {
                    (node.borrow().right_child_node.clone(), node.borrow().right_child_count)
                } else {
                    (node.borrow().left_child_node.clone(), node.borrow().left_child_count)
                };

                let np = node_parent.unwrap();
                {
                    let mut npb = np.borrow_mut();
                    if npb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, node)) {
                        npb.left_child_node = cand.0.clone();
                        npb.left_child_count = cand.1;
                    } else {
                        npb.right_child_node = cand.0.clone();
                        npb.right_child_count = cand.1;
                    }
                }
                if let Some(ref c) = cand.0 {
                    set_parent(c, Some(&np));
                }
                bubble_up = Some(np);
            }
        }

        // Bubble up: fix depths, rebalance, and decrement child counts
        let mut bubbling_finished = false;
        let mut bu = bubble_up;
        while let Some(ref current) = bu {
            if !bubbling_finished {
                let left_depth = current.borrow().left_child_node.as_ref().map_or(0u32, |l| l.borrow().depth + 1);
                let right_depth = current.borrow().right_child_node.as_ref().map_or(0u32, |r| r.borrow().depth + 1);
                let new_depth = imax(left_depth as i32, right_depth as i32) as u32;
                let depth_changed = new_depth != current.borrow().depth;
                current.borrow_mut().depth = new_depth;

                let balance = bostree_balance(current);
                let next;
                if balance < -1 {
                    {
                        let left = current.borrow().left_child_node.as_ref().unwrap().clone();
                        if bostree_balance(&left) > 0 {
                            rotate_left(self, &left);
                        }
                    }
                    let rotated = rotate_right(self, current);
                    next = rotated;
                } else if balance > 1 {
                    {
                        let right = current.borrow().right_child_node.as_ref().unwrap().clone();
                        if bostree_balance(&right) < 0 {
                            rotate_right(self, &right);
                        }
                    }
                    let rotated = rotate_left(self, current);
                    next = rotated;
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    next = current.clone();
                }

                let parent = get_parent(&next);
                if let Some(ref p) = parent {
                    let mut pb = p.borrow_mut();
                    if pb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, &next)) {
                        pb.left_child_count -= 1;
                    } else {
                        pb.right_child_count -= 1;
                    }
                }
                bu = parent;
            } else {
                let parent = get_parent(current);
                if let Some(ref p) = parent {
                    let mut pb = p.borrow_mut();
                    if pb.left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, current)) {
                        pb.left_child_count -= 1;
                    } else {
                        pb.right_child_count -= 1;
                    }
                }
                bu = parent;
            }
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
        let count = {
            let mut nb = node.borrow_mut();
            nb.weak_ref_count -= 1;
            nb.weak_ref_count
        };
        if count == 0 {
            if let Some(ref free_fn) = self.free_function {
                free_fn(node);
            }
            // Node will be dropped when all Rc references go away
            None
        } else if node.borrow().weak_ref_node_valid != 0 {
            Some(node.clone())
        } else {
            None
        }
    }
    /// Lookup a node in the tree by its key.
    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        while let Some(node) = current {
            let cmp = (self.cmp_function)(key, &node.borrow().key);
            if cmp == 0 {
                return Some(node);
            } else if cmp < 0 {
                let next = node.borrow().left_child_node.clone();
                current = next;
            } else {
                let next = node.borrow().right_child_node.clone();
                current = next;
            }
        }
        None
    }
    /// Return the node at the given in-order index (starting at 0).
    pub fn bostree_select(&self, index: u32) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        let mut idx = index;
        while let Some(node) = current {
            let lcc = node.borrow().left_child_count;
            if lcc <= idx {
                idx -= lcc;
                if idx == 0 {
                    return Some(node);
                }
                idx -= 1;
                let next = node.borrow().right_child_node.clone();
                current = next;
            } else {
                let next = node.borrow().left_child_node.clone();
                current = next;
            }
        }
        None
    }
    /// Print the tree (only available in debug builds).
    #[cfg(debug_assertions)]
    pub fn bostree_print(&self) {
        if let Some(ref root) = self.root_node {
            println!("digraph {{\n  ordering = out;");
            bostree_print_helper(root);
            println!("}}");
        }
    }
}

#[cfg(debug_assertions)]
fn bostree_print_helper(node: &Rc<RefCell<BOSNode>>) {
    let (key, lcc, rcc, depth, parent_key, left, right) = {
        let n = node.borrow();
        let pk = n.parent_node.as_ref().and_then(|w| w.upgrade()).map(|p| p.borrow().key.clone());
        (
            n.key.clone(),
            n.left_child_count,
            n.right_child_count,
            n.depth,
            pk,
            n.left_child_node.clone(),
            n.right_child_node.clone(),
        )
    };
    println!("  {} [label=\"\\N ({},{},{})\"];", key, lcc, rcc, depth);
    if let Some(pk) = parent_key {
        println!("  {} -> {} [color=green];", key, pk);
    }
    if let Some(ref left) = left {
        println!("  {} -> {}", key, left.borrow().key);
        bostree_print_helper(left);
    }
    if let Some(ref right) = right {
        println!("  {} -> {}", key, right.borrow().key);
        bostree_print_helper(right);
    }
}

/// Increase the weak reference count for a node and return the node.
pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    {
        let mut nb = node.borrow_mut();
        assert!(nb.weak_ref_count < 127);
        assert!(nb.weak_ref_count > 0);
        nb.weak_ref_count += 1;
    }
    node.clone()
}
/// Return the next node in an in-order traversal.
pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if node.borrow().right_child_node.is_some() {
        let mut current = node.borrow().right_child_node.clone().unwrap();
        loop {
            let next = current.borrow().left_child_node.clone();
            match next {
                Some(n) => current = n,
                None => return Some(current),
            }
        }
    } else {
        let mut current = node.clone();
        loop {
            let parent = get_parent(&current);
            match parent {
                Some(p) => {
                    let is_right = p.borrow().right_child_node.as_ref().map_or(false, |r| Rc::ptr_eq(r, &current));
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
}
/// Return the previous node in an in-order traversal.
pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if node.borrow().left_child_node.is_some() {
        let mut current = node.borrow().left_child_node.clone().unwrap();
        loop {
            let next = current.borrow().right_child_node.clone();
            match next {
                Some(n) => current = n,
                None => return Some(current),
            }
        }
    } else {
        let mut current = node.clone();
        loop {
            let parent = get_parent(&current);
            match parent {
                Some(p) => {
                    let is_left = p.borrow().left_child_node.as_ref().map_or(false, |l| Rc::ptr_eq(l, &current));
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
}
/// Return the rank (in-order index) of the given node.
pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut current = Some(node.clone());
    while let Some(ref n) = current {
        let parent = get_parent(n);
        if let Some(ref p) = parent {
            let is_right = p.borrow().right_child_node.as_ref().map_or(false, |r| Rc::ptr_eq(r, n));
            if is_right {
                counter += 1 + p.borrow().left_child_count;
            }
        }
        current = parent;
    }
    counter
}
