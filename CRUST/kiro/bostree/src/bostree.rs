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
pub type BOSTreeCmpFunction = fn(&str, &str) -> i32;
/// Type alias for a free function which will be called on nodes that are removed.
pub type BOSTreeFreeFunction = fn(&Rc<RefCell<BOSNode>>);

fn imax(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

fn get_parent(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn set_parent(node: &Rc<RefCell<BOSNode>>, parent: Option<&Rc<RefCell<BOSNode>>>) {
    node.borrow_mut().parent_node = parent.map(|p| Rc::downgrade(p));
}

fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let ld = n.left_child_node.as_ref().map_or(0, |l| l.borrow().depth as i32 + 1);
    let rd = n.right_child_node.as_ref().map_or(0, |r| r.borrow().depth as i32 + 1);
    rd - ld
}

fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let ld = node.borrow().left_child_node.as_ref().map_or(0, |l| l.borrow().depth + 1);
    let rd = node.borrow().right_child_node.as_ref().map_or(0, |r| r.borrow().depth + 1);
    node.borrow_mut().depth = imax(ld, rd);
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let l = p.borrow().left_child_node.clone().unwrap();
    let parent = get_parent(p);

    if let Some(ref par) = parent {
        let is_left = par.borrow().left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, p));
        if is_left {
            par.borrow_mut().left_child_node = Some(l.clone());
        } else {
            par.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }
    set_parent(&l, parent.as_ref());

    let lr = l.borrow().right_child_node.clone();
    p.borrow_mut().left_child_node = lr.clone();
    p.borrow_mut().left_child_count = l.borrow().right_child_count;
    if let Some(ref lr_node) = lr {
        set_parent(lr_node, Some(p));
    }
    update_depth(p);
    set_parent(p, Some(&l));

    l.borrow_mut().right_child_node = Some(p.clone());
    {
        let pb = p.borrow();
        l.borrow_mut().right_child_count = pb.left_child_count + pb.right_child_count + 1;
    }
    update_depth(&l);
    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let r = p.borrow().right_child_node.clone().unwrap();
    let parent = get_parent(p);

    if let Some(ref par) = parent {
        let is_left = par.borrow().left_child_node.as_ref().map_or(false, |n| Rc::ptr_eq(n, p));
        if is_left {
            par.borrow_mut().left_child_node = Some(r.clone());
        } else {
            par.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }
    set_parent(&r, parent.as_ref());

    let rl = r.borrow().left_child_node.clone();
    p.borrow_mut().right_child_node = rl.clone();
    p.borrow_mut().right_child_count = r.borrow().left_child_count;
    if let Some(ref rl_node) = rl {
        set_parent(rl_node, Some(p));
    }
    update_depth(p);
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
    pub fn bostree_new(
        cmp_function: BOSTreeCmpFunction,
        free_function: Option<BOSTreeFreeFunction>,
    ) -> Self {
        BOSTree { root_node: None, cmp_function, free_function }
    }

    pub fn bostree_node_count(&self) -> u32 {
        self.root_node.as_ref().map_or(0, |r| {
            let n = r.borrow();
            n.left_child_count + n.right_child_count + 1
        })
    }

    pub fn bostree_insert(&mut self, key: String, data: Option<String>) -> Rc<RefCell<BOSNode>> {
        let mut current = self.root_node.clone();
        let mut parent: Option<Rc<RefCell<BOSNode>>> = None;
        let mut go_left = false;

        while let Some(node) = current {
            parent = Some(node.clone());
            let cmp = (self.cmp_function)(&key, &node.borrow().key);
            if cmp < 0 {
                node.borrow_mut().left_child_count += 1;
                go_left = true;
                current = node.borrow().left_child_node.clone();
            } else {
                node.borrow_mut().right_child_count += 1;
                go_left = false;
                current = node.borrow().right_child_node.clone();
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0, right_child_count: 0, depth: 0,
            left_child_node: None, right_child_node: None,
            parent_node: parent.as_ref().map(|p| Rc::downgrade(p)),
            key, data, weak_ref_count: 1, weak_ref_node_valid: 1,
        }));

        if let Some(ref par) = parent {
            if go_left {
                par.borrow_mut().left_child_node = Some(new_node.clone());
            } else {
                par.borrow_mut().right_child_node = Some(new_node.clone());
            }
        } else {
            self.root_node = Some(new_node.clone());
            return new_node;
        }

        let par = parent.unwrap();
        let has_left = par.borrow().left_child_node.is_some();
        let has_right = par.borrow().right_child_node.is_some();
        if has_left ^ has_right {
            par.borrow_mut().depth += 1;
            let mut cur = get_parent(&par);
            while let Some(parent_node) = cur {
                let new_left_depth = parent_node.borrow().left_child_node.as_ref()
                    .map_or(0, |l| l.borrow().depth + 1);
                let new_right_depth = parent_node.borrow().right_child_node.as_ref()
                    .map_or(0, |r| r.borrow().depth + 1);
                let max_depth = imax(new_left_depth, new_right_depth);

                if parent_node.borrow().depth != max_depth {
                    parent_node.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                let result;
                if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                    let left = parent_node.borrow().left_child_node.clone().unwrap();
                    if balance(&left) > 0 {
                        rotate_left(self, &left);
                    }
                    result = rotate_right(self, &parent_node);
                } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                    let right = parent_node.borrow().right_child_node.clone().unwrap();
                    if balance(&right) < 0 {
                        rotate_right(self, &right);
                    }
                    result = rotate_left(self, &parent_node);
                } else {
                    result = parent_node;
                }
                cur = get_parent(&result);
            }
        }
        new_node
    }

    pub fn bostree_remove(&mut self, node: &Rc<RefCell<BOSNode>>) {
        let mut bubble_up: Option<Rc<RefCell<BOSNode>>> = None;
        let has_left = node.borrow().left_child_node.is_some();
        let has_right = node.borrow().right_child_node.is_some();

        if has_left && has_right {
            let left_depth = node.borrow().left_child_node.as_ref().unwrap().borrow().depth;
            let right_depth = node.borrow().right_child_node.as_ref().unwrap().borrow().depth;

            let (candidate, lost_child);
            if left_depth >= right_depth {
                node.borrow_mut().left_child_count -= 1;
                let mut c = node.borrow().left_child_node.clone().unwrap();
                while c.borrow().right_child_node.is_some() {
                    c.borrow_mut().right_child_count -= 1;
                    let next = c.borrow().right_child_node.clone().unwrap();
                    c = next;
                }
                lost_child = c.borrow().left_child_node.clone();
                candidate = c;
            } else {
                node.borrow_mut().right_child_count -= 1;
                let mut c = node.borrow().right_child_node.clone().unwrap();
                while c.borrow().left_child_node.is_some() {
                    c.borrow_mut().left_child_count -= 1;
                    let next = c.borrow().left_child_node.clone().unwrap();
                    c = next;
                }
                lost_child = c.borrow().right_child_node.clone();
                candidate = c;
            }

            let bubble_start = get_parent(&candidate).unwrap();

            // Detach candidate from its parent
            {
                let is_left = bubble_start.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &candidate));
                if is_left {
                    bubble_start.borrow_mut().left_child_node = lost_child.clone();
                } else {
                    bubble_start.borrow_mut().right_child_node = lost_child.clone();
                }
            }
            if let Some(ref lc) = lost_child {
                set_parent(lc, Some(&bubble_start));
            }

            // Place candidate where node was
            let node_parent = get_parent(node);
            if let Some(ref par) = node_parent {
                let is_left = par.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, node));
                if is_left {
                    par.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    par.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            set_parent(&candidate, node_parent.as_ref());

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
                let mut bs = bubble_start;
                while !Rc::ptr_eq(&bs, &candidate) {
                    update_depth(&bs);
                    let bal = balance(&bs);
                    let result;
                    if bal > 1 {
                        let right = bs.borrow().right_child_node.clone().unwrap();
                        if balance(&right) < 0 {
                            rotate_right(self, &right);
                        }
                        result = rotate_left(self, &bs);
                    } else if bal < -1 {
                        let left = bs.borrow().left_child_node.clone().unwrap();
                        if balance(&left) > 0 {
                            rotate_left(self, &left);
                        }
                        result = rotate_right(self, &bs);
                    } else {
                        result = bs;
                    }
                    bs = get_parent(&result).unwrap();
                }
            }

            update_depth(&candidate);
            bubble_up = get_parent(&candidate);

            if let Some(ref bu) = bubble_up {
                let is_left = bu.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &candidate));
                if is_left {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            let node_parent = get_parent(node);
            if node_parent.is_none() {
                if has_left {
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
                let par = node_parent.unwrap();
                let (cand, cand_count) = if node.borrow().right_child_node.is_some() {
                    (node.borrow().right_child_node.clone(), node.borrow().right_child_count)
                } else {
                    (node.borrow().left_child_node.clone(), node.borrow().left_child_count)
                };

                let is_left = par.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, node));
                if is_left {
                    par.borrow_mut().left_child_node = cand.clone();
                    par.borrow_mut().left_child_count = cand_count;
                } else {
                    par.borrow_mut().right_child_node = cand.clone();
                    par.borrow_mut().right_child_count = cand_count;
                }
                if let Some(ref c) = cand {
                    set_parent(c, Some(&par));
                }
                bubble_up = Some(par);
            }
        }

        // Bubble up: fix depths, rebalance, decrement child counts
        let mut bubbling_finished = false;
        let mut bubble = bubble_up;
        while let Some(bu) = bubble {
            let after;
            if !bubbling_finished {
                let left_depth = bu.borrow().left_child_node.as_ref()
                    .map_or(0, |l| l.borrow().depth + 1);
                let right_depth = bu.borrow().right_child_node.as_ref()
                    .map_or(0, |r| r.borrow().depth + 1);
                let new_depth = imax(left_depth, right_depth);
                let depth_changed = new_depth != bu.borrow().depth;
                bu.borrow_mut().depth = new_depth;

                let bal = balance(&bu);
                if bal < -1 {
                    let left = bu.borrow().left_child_node.clone().unwrap();
                    if balance(&left) > 0 {
                        rotate_left(self, &left);
                    }
                    after = rotate_right(self, &bu);
                } else if bal > 1 {
                    let right = bu.borrow().right_child_node.clone().unwrap();
                    if balance(&right) < 0 {
                        rotate_right(self, &right);
                    }
                    after = rotate_left(self, &bu);
                } else {
                    if !depth_changed {
                        bubbling_finished = true;
                    }
                    after = bu;
                }
            } else {
                after = bu;
            }

            let parent = get_parent(&after);
            if let Some(ref p) = parent {
                let is_left = p.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &after));
                if is_left {
                    p.borrow_mut().left_child_count -= 1;
                } else {
                    p.borrow_mut().right_child_count -= 1;
                }
            }
            bubble = parent;
        }

        node.borrow_mut().weak_ref_node_valid = 0;
        self.bostree_node_weak_unref(node);
    }

    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        node.borrow_mut().weak_ref_count -= 1;
        if node.borrow().weak_ref_count == 0 {
            if let Some(ref free_fn) = self.free_function {
                free_fn(node);
            }
            None
        } else if node.borrow().weak_ref_node_valid != 0 {
            Some(node.clone())
        } else {
            None
        }
    }

    pub fn bostree_lookup(&self, key: &str) -> Option<Rc<RefCell<BOSNode>>> {
        let mut current = self.root_node.clone();
        while let Some(node) = current {
            let cmp = (self.cmp_function)(key, &node.borrow().key);
            if cmp == 0 {
                return Some(node);
            } else if cmp < 0 {
                current = node.borrow().left_child_node.clone();
            } else {
                current = node.borrow().right_child_node.clone();
            }
        }
        None
    }

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
                current = node.borrow().right_child_node.clone();
            } else {
                current = node.borrow().left_child_node.clone();
            }
        }
        None
    }

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
        let pk = n.parent_node.as_ref().and_then(|pw| pw.upgrade()).map(|p| p.borrow().key.clone());
        (n.key.clone(), n.left_child_count, n.right_child_count, n.depth,
         pk, n.left_child_node.clone(), n.right_child_node.clone())
    };
    println!("  {} [label=\"\\N ({},{},{})\"];", key, lcc, rcc, depth);
    if let Some(pk) = parent_key {
        println!("  {} -> {} [color=green];", key, pk);
    }
    if let Some(ref left) = left {
        println!("  {} -> {}", key, left.borrow().key);
        print_helper(left);
    }
    if let Some(ref right) = right {
        println!("  {} -> {}", key, right.borrow().key);
        print_helper(right);
    }
}

pub fn bostree_node_weak_ref(node: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    assert!(node.borrow().weak_ref_count < 127);
    assert!(node.borrow().weak_ref_count > 0);
    node.borrow_mut().weak_ref_count += 1;
    node.clone()
}

pub fn bostree_next_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if node.borrow().right_child_node.is_some() {
        let mut cur = node.borrow().right_child_node.clone().unwrap();
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
        let parent = get_parent(&cur);
        match parent {
            Some(p) => {
                let is_right = p.borrow().right_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
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

pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if node.borrow().left_child_node.is_some() {
        let mut cur = node.borrow().left_child_node.clone().unwrap();
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
            Some(p) => {
                let is_left = p.borrow().left_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
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

pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut cur = node.clone();
    loop {
        let parent = get_parent(&cur);
        match parent {
            Some(p) => {
                let is_right = p.borrow().right_child_node.as_ref()
                    .map_or(false, |n| Rc::ptr_eq(n, &cur));
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
