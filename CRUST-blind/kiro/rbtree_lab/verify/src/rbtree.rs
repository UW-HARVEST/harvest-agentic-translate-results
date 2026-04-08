use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub enum Color {
    Red,
    Black,
}

pub type Key = i32;
pub type NodeRef = Rc<RefCell<Node>>;

#[derive(Debug, Clone)]
pub struct Node {
    pub key: Key,
    pub color: Color,
    pub left: Option<NodeRef>,   // None == NIL
    pub right: Option<NodeRef>,  // None == NIL
    pub parent: Option<NodeRef>, // None == NIL / root parent
}

#[derive(Debug, Clone)]
pub struct RBTree {
    /// Root of the tree. `None` means the tree is empty (NIL).
    pub root: Option<NodeRef>,
}

fn new_node(key: Key) -> NodeRef {
    Rc::new(RefCell::new(Node {
        key,
        color: Color::Red,
        left: None,
        right: None,
        parent: None,
    }))
}

fn ptr_eq(a: &Option<NodeRef>, b: &NodeRef) -> bool {
    a.as_ref().is_some_and(|n| Rc::ptr_eq(n, b))
}

fn color_of(n: &Option<NodeRef>) -> Color {
    n.as_ref().map_or(Color::Black, |node| node.borrow().color.clone())
}

fn set_color(n: &Option<NodeRef>, c: Color) {
    if let Some(node) = n.as_ref() {
        node.borrow_mut().color = c;
    }
}

fn parent(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().parent.clone()
}

fn left(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().left.clone()
}

fn right(n: &NodeRef) -> Option<NodeRef> {
    n.borrow().right.clone()
}

impl RBTree {
    pub fn new() -> Self {
        RBTree { root: None }
    }

    pub fn right_rotate(&mut self, x: NodeRef) {
        let y = left(&x).unwrap();
        let yr = right(&y);
        x.borrow_mut().left = yr.clone();
        if let Some(yr_n) = yr.as_ref() {
            yr_n.borrow_mut().parent = Some(x.clone());
        }
        let xp = parent(&x);
        y.borrow_mut().parent = xp.clone();
        match xp {
            None => self.root = Some(y.clone()),
            Some(p) => {
                if ptr_eq(&p.borrow().left, &x) {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }
        y.borrow_mut().right = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    pub fn left_rotate(&mut self, x: NodeRef) {
        let y = right(&x).unwrap();
        let yl = left(&y);
        x.borrow_mut().right = yl.clone();
        if let Some(yl_n) = yl.as_ref() {
            yl_n.borrow_mut().parent = Some(x.clone());
        }
        let xp = parent(&x);
        y.borrow_mut().parent = xp.clone();
        match xp {
            None => self.root = Some(y.clone()),
            Some(p) => {
                if ptr_eq(&p.borrow().left, &x) {
                    p.borrow_mut().left = Some(y.clone());
                } else {
                    p.borrow_mut().right = Some(y.clone());
                }
            }
        }
        y.borrow_mut().left = Some(x.clone());
        x.borrow_mut().parent = Some(y);
    }

    pub fn free_node(node: Option<NodeRef>) {
        if let Some(n) = node {
            let l = n.borrow_mut().left.take();
            let r = n.borrow_mut().right.take();
            Self::free_node(l);
            Self::free_node(r);
            n.borrow_mut().parent.take();
        }
    }

    pub fn delete_rbtree(mut self) {
        Self::free_node(self.root.take());
    }

    pub fn rbtree_insert_fixup(&mut self, z: NodeRef) {
        let mut z = z;
        while color_of(&parent(&z).map(|p| p)) == Color::Red {
            let zp = parent(&z).unwrap();
            let zpp = parent(&zp).unwrap();
            if ptr_eq(&left(&zpp), &zp) {
                let uncle = right(&zpp);
                if color_of(&uncle) == Color::Red {
                    set_color(&Some(zp), Color::Black);
                    set_color(&uncle, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    if ptr_eq(&right(&zp), &z) {
                        z = zp;
                        self.left_rotate(z.clone());
                    }
                    let zp = parent(&z).unwrap();
                    let zpp = parent(&zp).unwrap();
                    set_color(&Some(zp), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.right_rotate(zpp);
                }
            } else {
                let uncle = left(&zpp);
                if color_of(&uncle) == Color::Red {
                    set_color(&Some(zp), Color::Black);
                    set_color(&uncle, Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    z = zpp;
                } else {
                    if ptr_eq(&left(&zp), &z) {
                        z = zp;
                        self.right_rotate(z.clone());
                    }
                    let zp = parent(&z).unwrap();
                    let zpp = parent(&zp).unwrap();
                    set_color(&Some(zp), Color::Black);
                    set_color(&Some(zpp.clone()), Color::Red);
                    self.left_rotate(zpp);
                }
            }
        }
        if let Some(root) = self.root.as_ref() {
            root.borrow_mut().color = Color::Black;
        }
    }

    pub fn rbtree_insert(&mut self, key: Key) -> Option<NodeRef> {
        let z = new_node(key);
        let mut par: Option<NodeRef> = None;
        let mut cur = self.root.clone();
        while let Some(c) = cur {
            par = Some(c.clone());
            cur = if key < c.borrow().key { left(&c) } else { right(&c) };
        }
        z.borrow_mut().parent = par.clone();
        match par {
            None => self.root = Some(z.clone()),
            Some(p) => {
                if key < p.borrow().key {
                    p.borrow_mut().left = Some(z.clone());
                } else {
                    p.borrow_mut().right = Some(z.clone());
                }
            }
        }
        z.borrow_mut().color = Color::Red;
        self.rbtree_insert_fixup(z.clone());
        Some(z)
    }

    pub fn rbtree_find(&self, key: Key) -> Option<NodeRef> {
        let mut cur = self.root.clone();
        while let Some(c) = cur {
            let k = c.borrow().key;
            if k == key {
                return Some(c);
            }
            cur = if k < key { right(&c) } else { left(&c) };
        }
        None
    }

    pub fn rbtree_min(&self) -> Option<NodeRef> {
        let mut c = self.root.clone()?;
        while let Some(l) = left(&c) {
            c = l;
        }
        Some(c)
    }

    pub fn rbtree_max(&self) -> Option<NodeRef> {
        let mut c = self.root.clone()?;
        while let Some(r) = right(&c) {
            c = r;
        }
        Some(c)
    }

    pub fn transplant(&mut self, u: NodeRef, v: Option<NodeRef>) {
        let up = parent(&u);
        match up.clone() {
            None => self.root = v.clone(),
            Some(p) => {
                if ptr_eq(&p.borrow().left, &u) {
                    p.borrow_mut().left = v.clone();
                } else {
                    p.borrow_mut().right = v.clone();
                }
            }
        }
        if let Some(vn) = v.as_ref() {
            vn.borrow_mut().parent = up;
        }
    }

    pub fn delete_fixup(&mut self, x: Option<NodeRef>) {
        self.do_delete_fixup(x, None);
    }

    fn do_delete_fixup(&mut self, x: Option<NodeRef>, x_parent_hint: Option<NodeRef>) {
        let mut x = x;
        let mut par_hint = x_parent_hint;

        loop {
            if is_root(&self.root, &x) || color_of(&x) == Color::Red {
                break;
            }
            let p = x.as_ref().and_then(|n| parent(n)).or(par_hint.clone());
            let p = match p {
                Some(p) => p,
                None => break,
            };

            let x_is_left = match x.as_ref() {
                Some(n) => ptr_eq(&p.borrow().left, n),
                None => p.borrow().left.is_none(),
            };

            if x_is_left {
                let mut w = right(&p);
                if color_of(&w) == Color::Red {
                    set_color(&w, Color::Black);
                    set_color(&Some(p.clone()), Color::Red);
                    self.left_rotate(p.clone());
                    w = right(&p);
                }
                let wn = match w {
                    Some(n) => n,
                    None => break,
                };
                if color_of(&left(&wn)) == Color::Black && color_of(&right(&wn)) == Color::Black {
                    set_color(&Some(wn), Color::Red);
                    x = Some(p.clone());
                    par_hint = parent(&p);
                } else {
                    if color_of(&right(&wn)) == Color::Black {
                        set_color(&left(&wn), Color::Black);
                        set_color(&Some(wn.clone()), Color::Red);
                        self.right_rotate(wn);
                        w = right(&p);
                    } else {
                        w = Some(wn);
                    }
                    let wn = w.unwrap();
                    set_color(&Some(wn.clone()), p.borrow().color.clone());
                    set_color(&Some(p.clone()), Color::Black);
                    set_color(&right(&wn), Color::Black);
                    self.left_rotate(p);
                    x = self.root.clone();
                    break;
                }
            } else {
                let mut w = left(&p);
                if color_of(&w) == Color::Red {
                    set_color(&w, Color::Black);
                    set_color(&Some(p.clone()), Color::Red);
                    self.right_rotate(p.clone());
                    w = left(&p);
                }
                let wn = match w {
                    Some(n) => n,
                    None => break,
                };
                if color_of(&right(&wn)) == Color::Black && color_of(&left(&wn)) == Color::Black {
                    set_color(&Some(wn), Color::Red);
                    x = Some(p.clone());
                    par_hint = parent(&p);
                } else {
                    if color_of(&left(&wn)) == Color::Black {
                        set_color(&right(&wn), Color::Black);
                        set_color(&Some(wn.clone()), Color::Red);
                        self.left_rotate(wn);
                        w = left(&p);
                    } else {
                        w = Some(wn);
                    }
                    let wn = w.unwrap();
                    set_color(&Some(wn.clone()), p.borrow().color.clone());
                    set_color(&Some(p.clone()), Color::Black);
                    set_color(&left(&wn), Color::Black);
                    self.right_rotate(p);
                    x = self.root.clone();
                    break;
                }
            }
        }
        set_color(&x, Color::Black);
    }

    pub fn erase(&mut self, p: NodeRef) {
        let y_orig_color;
        let x: Option<NodeRef>;
        let x_parent: Option<NodeRef>;

        if p.borrow().left.is_none() {
            y_orig_color = p.borrow().color.clone();
            x = right(&p);
            x_parent = parent(&p);
            self.transplant(p.clone(), right(&p));
        } else if p.borrow().right.is_none() {
            y_orig_color = p.borrow().color.clone();
            x = left(&p);
            x_parent = parent(&p);
            self.transplant(p.clone(), left(&p));
        } else {
            let mut y = right(&p).unwrap();
            while let Some(l) = left(&y) {
                y = l;
            }
            y_orig_color = y.borrow().color.clone();
            x = right(&y);

            if Rc::ptr_eq(&parent(&y).unwrap(), &p) {
                if let Some(xn) = x.as_ref() {
                    xn.borrow_mut().parent = Some(y.clone());
                }
                x_parent = Some(y.clone());
            } else {
                x_parent = parent(&y);
                self.transplant(y.clone(), right(&y));
                y.borrow_mut().right = right(&p);
                if let Some(yr) = y.borrow().right.as_ref() {
                    yr.borrow_mut().parent = Some(y.clone());
                }
            }
            self.transplant(p.clone(), Some(y.clone()));
            y.borrow_mut().left = left(&p);
            if let Some(yl) = y.borrow().left.as_ref() {
                yl.borrow_mut().parent = Some(y.clone());
            }
            y.borrow_mut().color = p.borrow().color.clone();
        }

        if y_orig_color == Color::Black {
            self.do_delete_fixup(x, x_parent);
        }
    }

    pub fn subtree_to_array(&self, curr: Option<NodeRef>, arr: &mut Vec<Key>, n: usize, count: &mut usize) {
        if let Some(node) = curr {
            self.subtree_to_array(left(&node), arr, n, count);
            if *count < n {
                arr.push(node.borrow().key);
                *count += 1;
            } else {
                return;
            }
            self.subtree_to_array(right(&node), arr, n, count);
        }
    }

    pub fn to_array(&self, n: usize) -> Vec<Key> {
        let mut arr = Vec::new();
        let mut count = 0;
        self.subtree_to_array(self.root.clone(), &mut arr, n, &mut count);
        arr
    }
}

fn is_root(root: &Option<NodeRef>, x: &Option<NodeRef>) -> bool {
    match (root.as_ref(), x.as_ref()) {
        (Some(r), Some(n)) => Rc::ptr_eq(r, n),
        _ => false,
    }
}
