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

fn get_parent(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    node.borrow().parent_node.as_ref().and_then(|w| w.upgrade())
}

fn is_left_child(parent: &Rc<RefCell<BOSNode>>, child: &Rc<RefCell<BOSNode>>) -> bool {
    parent.borrow().left_child_node.as_ref().map_or(false, |l| Rc::ptr_eq(l, child))
}

fn balance(node: &Rc<RefCell<BOSNode>>) -> i32 {
    let n = node.borrow();
    let ld = n.left_child_node.as_ref().map_or(0i32, |l| l.borrow().depth as i32 + 1);
    let rd = n.right_child_node.as_ref().map_or(0i32, |r| r.borrow().depth as i32 + 1);
    rd - ld
}

fn update_depth(node: &Rc<RefCell<BOSNode>>) {
    let n = node.borrow();
    let ld = n.left_child_node.as_ref().map_or(0, |l| l.borrow().depth + 1);
    let rd = n.right_child_node.as_ref().map_or(0, |r| r.borrow().depth + 1);
    let d = ld.max(rd);
    drop(n);
    node.borrow_mut().depth = d;
}

fn rotate_right(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let l = p.borrow().left_child_node.as_ref().unwrap().clone();
    let parent = get_parent(p);
    if let Some(ref par) = parent {
        if is_left_child(par, p) {
            par.borrow_mut().left_child_node = Some(l.clone());
        } else {
            par.borrow_mut().right_child_node = Some(l.clone());
        }
    } else {
        tree.root_node = Some(l.clone());
    }
    l.borrow_mut().parent_node = parent.as_ref().map(|p| Rc::downgrade(p));

    let l_right = l.borrow().right_child_node.clone();
    let l_right_count = l.borrow().right_child_count;
    p.borrow_mut().left_child_node = l_right.clone();
    p.borrow_mut().left_child_count = l_right_count;
    if let Some(ref lr) = l_right {
        lr.borrow_mut().parent_node = Some(Rc::downgrade(p));
    }
    update_depth(p);
    p.borrow_mut().parent_node = Some(Rc::downgrade(&l));

    l.borrow_mut().right_child_node = Some(p.clone());
    l.borrow_mut().right_child_count = p.borrow().left_child_count + p.borrow().right_child_count + 1;
    update_depth(&l);
    l
}

fn rotate_left(tree: &mut BOSTree, p: &Rc<RefCell<BOSNode>>) -> Rc<RefCell<BOSNode>> {
    let r = p.borrow().right_child_node.as_ref().unwrap().clone();
    let parent = get_parent(p);
    if let Some(ref par) = parent {
        if is_left_child(par, p) {
            par.borrow_mut().left_child_node = Some(r.clone());
        } else {
            par.borrow_mut().right_child_node = Some(r.clone());
        }
    } else {
        tree.root_node = Some(r.clone());
    }
    r.borrow_mut().parent_node = parent.as_ref().map(|p| Rc::downgrade(p));

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
    r.borrow_mut().left_child_count = p.borrow().left_child_count + p.borrow().right_child_count + 1;
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

        while let Some(cur) = current {
            parent = Some(cur.clone());
            let cmp = (self.cmp_function)(&key, &cur.borrow().key);
            if cmp < 0 {
                cur.borrow_mut().left_child_count += 1;
                let next = cur.borrow().left_child_node.clone();
                current = next;
                go_left = true;
            } else {
                cur.borrow_mut().right_child_count += 1;
                let next = cur.borrow().right_child_node.clone();
                current = next;
                go_left = false;
            }
        }

        let new_node = Rc::new(RefCell::new(BOSNode {
            left_child_count: 0, right_child_count: 0, depth: 0,
            left_child_node: None, right_child_node: None,
            parent_node: parent.as_ref().map(|p| Rc::downgrade(p)),
            key, data,
            weak_ref_count: 1, weak_ref_node_valid: 1,
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
                let new_left_depth = parent_node.borrow().left_child_node.as_ref().map_or(0u32, |l| l.borrow().depth + 1);
                let new_right_depth = parent_node.borrow().right_child_node.as_ref().map_or(0u32, |r| r.borrow().depth + 1);
                let max_depth = new_left_depth.max(new_right_depth);

                if parent_node.borrow().depth != max_depth {
                    parent_node.borrow_mut().depth = max_depth;
                } else {
                    break;
                }

                if new_left_depth >= 2 && new_left_depth - 2 == new_right_depth {
                    let left_child = parent_node.borrow().left_child_node.as_ref().unwrap().clone();
                    if balance(&left_child) > 0 {
                        rotate_left(self, &left_child);
                    }
                    let rotated = rotate_right(self, &parent_node);
                    cur = get_parent(&rotated);
                } else if new_right_depth >= 2 && new_left_depth + 2 == new_right_depth {
                    let right_child = parent_node.borrow().right_child_node.as_ref().unwrap().clone();
                    if balance(&right_child) < 0 {
                        rotate_right(self, &right_child);
                    }
                    let rotated = rotate_left(self, &parent_node);
                    cur = get_parent(&rotated);
                } else {
                    cur = get_parent(&parent_node);
                }
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

            let candidate;
            let lost_child;
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
            if is_left_child(&bubble_start, &candidate) {
                bubble_start.borrow_mut().left_child_node = lost_child.clone();
            } else {
                bubble_start.borrow_mut().right_child_node = lost_child.clone();
            }
            if let Some(ref lc) = lost_child {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&bubble_start));
            }

            let node_parent = get_parent(node);
            if let Some(ref np) = node_parent {
                if is_left_child(np, node) {
                    np.borrow_mut().left_child_node = Some(candidate.clone());
                } else {
                    np.borrow_mut().right_child_node = Some(candidate.clone());
                }
            } else {
                self.root_node = Some(candidate.clone());
            }
            candidate.borrow_mut().parent_node = node_parent.as_ref().map(|p| Rc::downgrade(p));

            {
                let nb = node.borrow();
                candidate.borrow_mut().left_child_node = nb.left_child_node.clone();
                candidate.borrow_mut().left_child_count = nb.left_child_count;
                candidate.borrow_mut().right_child_node = nb.right_child_node.clone();
                candidate.borrow_mut().right_child_count = nb.right_child_count;
            }

            if let Some(ref lc) = candidate.borrow().left_child_node.clone() {
                lc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }
            if let Some(ref rc) = candidate.borrow().right_child_node.clone() {
                rc.borrow_mut().parent_node = Some(Rc::downgrade(&candidate));
            }

            // Rebalance from bubble_start up to candidate
            if !Rc::ptr_eq(&bubble_start, node) {
                let mut bs = bubble_start;
                while !Rc::ptr_eq(&bs, &candidate) {
                    update_depth(&bs);
                    let bal = balance(&bs);
                    if bal > 1 {
                        let right = bs.borrow().right_child_node.as_ref().unwrap().clone();
                        if balance(&right) < 0 {
                            rotate_right(self, &right);
                        }
                        let rotated = rotate_left(self, &bs);
                        bs = get_parent(&rotated).unwrap_or(rotated);
                    } else if bal < -1 {
                        let left = bs.borrow().left_child_node.as_ref().unwrap().clone();
                        if balance(&left) > 0 {
                            rotate_left(self, &left);
                        }
                        let rotated = rotate_right(self, &bs);
                        bs = get_parent(&rotated).unwrap_or(rotated);
                    } else {
                        bs = get_parent(&bs).unwrap();
                    }
                }
            }

            update_depth(&candidate);

            bubble_up = get_parent(&candidate);
            if let Some(ref bu) = bubble_up {
                if is_left_child(bu, &candidate) {
                    bu.borrow_mut().left_child_count -= 1;
                } else {
                    bu.borrow_mut().right_child_count -= 1;
                }
            }
        } else {
            let node_parent = get_parent(node);
            if node_parent.is_none() {
                if has_left {
                    self.root_node = node.borrow().left_child_node.clone();
                } else {
                    self.root_node = node.borrow().right_child_node.clone();
                }
                if let Some(ref r) = self.root_node {
                    r.borrow_mut().parent_node = None;
                }
                bubble_up = None;
            } else {
                let np = node_parent.unwrap();
                let (cand, cand_count) = if has_right {
                    (node.borrow().right_child_node.clone(), node.borrow().right_child_count)
                } else {
                    (node.borrow().left_child_node.clone(), node.borrow().left_child_count)
                };

                if is_left_child(&np, node) {
                    np.borrow_mut().left_child_node = cand.clone();
                    np.borrow_mut().left_child_count = cand_count;
                } else {
                    np.borrow_mut().right_child_node = cand.clone();
                    np.borrow_mut().right_child_count = cand_count;
                }

                if let Some(ref c) = cand {
                    c.borrow_mut().parent_node = Some(Rc::downgrade(&np));
                }

                bubble_up = Some(np);
            }
        }

        // Bubble up: fix depths, rebalance, decrement child counts
        let mut bubbling_finished = false;
        let mut bu = bubble_up;
        while let Some(current) = bu {
            if !bubbling_finished {
                let left_depth = current.borrow().left_child_node.as_ref().map_or(0u32, |l| l.borrow().depth + 1);
                let right_depth = current.borrow().right_child_node.as_ref().map_or(0u32, |r| r.borrow().depth + 1);
                let new_depth = left_depth.max(right_depth);
                let depth_changed = new_depth != current.borrow().depth;
                current.borrow_mut().depth = new_depth;

                let bal = balance(&current);
                if bal < -1 {
                    let left = current.borrow().left_child_node.as_ref().unwrap().clone();
                    if balance(&left) > 0 {
                        rotate_left(self, &left);
                    }
                    let rotated = rotate_right(self, &current);
                    let parent = get_parent(&rotated);
                    if let Some(ref p) = parent {
                        if is_left_child(p, &rotated) {
                            p.borrow_mut().left_child_count -= 1;
                        } else {
                            p.borrow_mut().right_child_count -= 1;
                        }
                    }
                    bu = parent;
                    continue;
                } else if bal > 1 {
                    let right = current.borrow().right_child_node.as_ref().unwrap().clone();
                    if balance(&right) < 0 {
                        rotate_right(self, &right);
                    }
                    let rotated = rotate_left(self, &current);
                    let parent = get_parent(&rotated);
                    if let Some(ref p) = parent {
                        if is_left_child(p, &rotated) {
                            p.borrow_mut().left_child_count -= 1;
                        } else {
                            p.borrow_mut().right_child_count -= 1;
                        }
                    }
                    bu = parent;
                    continue;
                } else if !depth_changed {
                    bubbling_finished = true;
                }
            }

            let parent = get_parent(&current);
            if let Some(ref p) = parent {
                if is_left_child(p, &current) {
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

    pub fn bostree_node_weak_unref(
        &mut self,
        node: &Rc<RefCell<BOSNode>>,
    ) -> Option<Rc<RefCell<BOSNode>>> {
        let count = {
            let mut n = node.borrow_mut();
            n.weak_ref_count -= 1;
            n.weak_ref_count
        };
        if count == 0 {
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
                let next = node.borrow().left_child_node.clone();
                current = next;
            } else {
                let next = node.borrow().right_child_node.clone();
                current = next;
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
                let next = node.borrow().right_child_node.clone();
                current = next;
            } else {
                let next = node.borrow().left_child_node.clone();
                current = next;
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
    let n = node.borrow();
    println!("  {} [label=\"\\N ({},{},{})\"];", n.key, n.left_child_count, n.right_child_count, n.depth);
    if let Some(ref pw) = n.parent_node {
        if let Some(p) = pw.upgrade() {
            println!("  {} -> {} [color=green];", n.key, p.borrow().key);
        }
    }
    let left = n.left_child_node.clone();
    let right = n.right_child_node.clone();
    if let Some(ref l) = left {
        println!("  {} -> {}", n.key, l.borrow().key);
    }
    if let Some(ref r) = right {
        println!("  {} -> {}", n.key, r.borrow().key);
    }
    drop(n);
    if let Some(ref l) = left {
        print_helper(l);
    }
    if let Some(ref r) = right {
        print_helper(r);
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
        let mut cur = node.borrow().right_child_node.as_ref().unwrap().clone();
        while cur.borrow().left_child_node.is_some() {
            let next = cur.borrow().left_child_node.as_ref().unwrap().clone();
            cur = next;
        }
        return Some(cur);
    }
    if node.borrow().parent_node.as_ref().and_then(|w| w.upgrade()).is_some() {
        let mut cur = node.clone();
        loop {
            let parent = get_parent(&cur);
            match parent {
                Some(ref p) if !is_left_child(p, &cur) => cur = p.clone(),
                _ => break,
            }
        }
        return get_parent(&cur);
    }
    None
}

pub fn bostree_previous_node(node: &Rc<RefCell<BOSNode>>) -> Option<Rc<RefCell<BOSNode>>> {
    if node.borrow().left_child_node.is_some() {
        let mut cur = node.borrow().left_child_node.as_ref().unwrap().clone();
        while cur.borrow().right_child_node.is_some() {
            let next = cur.borrow().right_child_node.as_ref().unwrap().clone();
            cur = next;
        }
        return Some(cur);
    }
    if node.borrow().parent_node.as_ref().and_then(|w| w.upgrade()).is_some() {
        let mut cur = node.clone();
        loop {
            let parent = get_parent(&cur);
            match parent {
                Some(ref p) if is_left_child(p, &cur) => cur = p.clone(),
                _ => break,
            }
        }
        return get_parent(&cur);
    }
    None
}

pub fn bostree_rank(node: &Rc<RefCell<BOSNode>>) -> u32 {
    let mut counter = node.borrow().left_child_count;
    let mut cur = Some(node.clone());
    while let Some(n) = cur {
        if let Some(parent) = get_parent(&n) {
            if !is_left_child(&parent, &n) {
                counter += 1 + parent.borrow().left_child_count;
            }
        }
        cur = get_parent(&n);
    }
    counter
}
