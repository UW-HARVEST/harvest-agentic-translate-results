use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

const EXT: i16 = -1;

#[derive(Debug)]
pub struct CbtNode { pub crit: i16, pub left: Option<Box<CbtNode>>, pub right: Option<Box<CbtNode>> }
#[derive(Debug)]
pub struct CbtLeaf {
    pub crit: i16, pub data: Box<dyn Any>, pub key: String,
    pub prev: Option<Weak<RefCell<CbtLeaf>>>, pub next: Option<Rc<RefCell<CbtLeaf>>>,
}
pub type CbtLeafPtr = Rc<RefCell<CbtLeaf>>;
pub type DupFn = dyn Fn(&Cbt, &dyn Any) -> Box<dyn Any>;
pub type GetLenFn = dyn Fn(&Cbt, &dyn Any) -> i32;
pub type CmpFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;
pub type GetCritFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;

pub struct Cbt {
    pub count: i32, pub root: Option<Box<CbtNode>>,
    pub first: Option<CbtLeafPtr>, pub last: Option<CbtLeafPtr>,
    pub dup: Option<Box<DupFn>>, pub getlen: Option<Box<GetLenFn>>,
    pub cmp: Option<Box<CmpFn>>, pub getcrit: Option<Box<GetCritFn>>,
    pub len: i32,
}

fn store_leaf(lp: CbtLeafPtr) -> Box<CbtNode> {
    let raw = Rc::into_raw(lp) as *mut CbtNode;
    Box::new(CbtNode { crit: EXT, left: Some(unsafe { Box::from_raw(raw) }), right: None })
}
fn borrow_leaf(node: &CbtNode) -> CbtLeafPtr {
    let raw = &**node.left.as_ref().unwrap() as *const CbtNode as *const RefCell<CbtLeaf>;
    unsafe { Rc::increment_strong_count(raw); Rc::from_raw(raw) }
}
fn extract_leaf(mut node: Box<CbtNode>) -> CbtLeafPtr {
    let raw = Box::into_raw(node.left.take().unwrap()) as *const RefCell<CbtLeaf>;
    std::mem::forget(node);
    unsafe { Rc::from_raw(raw) }
}
fn is_leaf(n: &CbtNode) -> bool { n.crit == EXT }
fn testbit(key: &[u8], bit: i16) -> bool {
    let bi = bit as usize;
    (1 << (7 - (bi & 7))) & key.get(bi >> 3).copied().unwrap_or(0) != 0
}
fn snap(lp: &CbtLeafPtr) -> CbtLeaf {
    let b = lp.borrow();
    CbtLeaf { crit: EXT, data: Box::new(()), key: b.key.clone(), prev: b.prev.clone(), next: b.next.clone() }
}
fn any_str(a: &dyn Any) -> &str { a.downcast_ref::<String>().unwrap() }

fn getcrit_default(_: &Cbt, k0: &dyn Any, k1: &dyn Any) -> i32 {
    let (s0, s1) = (any_str(k0).as_bytes(), any_str(k1).as_bytes());
    let mut i = 0;
    loop {
        let (c0, c1) = (s0.get(i).copied().unwrap_or(0), s1.get(i).copied().unwrap_or(0));
        if c0 == c1 { if c0 == 0 { return 0; } i += 1; continue; }
        let c = c0 ^ c1;
        let mut bit = 7i32;
        while (c >> bit) == 0 { bit -= 1; }
        let crit = ((i as i32) << 3) + 7 - bit + 1;
        if (c0 >> bit) & 1 != 0 { return crit; } else { return -crit; }
    }
}
fn cmp_default(_: &Cbt, k0: &dyn Any, k1: &dyn Any) -> i32 {
    match any_str(k0).cmp(any_str(k1)) { std::cmp::Ordering::Less => -1, std::cmp::Ordering::Equal => 0, std::cmp::Ordering::Greater => 1 }
}
fn getlen_default(_: &Cbt, k: &dyn Any) -> i32 { any_str(k).len() as i32 + 1 }
fn dup_default(_: &Cbt, k: &dyn Any) -> Box<dyn Any> { Box::new(any_str(k).to_string()) }

impl Cbt {
    fn init() -> Self {
        Cbt { count: 0, root: None, first: None, last: None, len: 0,
            dup: None, getlen: None, cmp: None, getcrit: None }
    }
    pub fn cbt_new() -> Self {
        let mut c = Self::init(); c.len = 0;
        c.dup = Some(Box::new(dup_default)); c.getlen = Some(Box::new(getlen_default));
        c.cmp = Some(Box::new(cmp_default)); c.getcrit = Some(Box::new(getcrit_default)); c
    }
    pub fn cbt_new_u(len: i32) -> Self {
        let mut c = Self::init(); c.len = len;
        c.getcrit = Some(Box::new(move |_cbt: &Cbt, k0: &dyn Any, k1: &dyn Any| {
            let (b0, b1) = (k0.downcast_ref::<Vec<u8>>().unwrap(), k1.downcast_ref::<Vec<u8>>().unwrap());
            let mut i = 0;
            loop {
                if i == len as usize { return 0; }
                if b0[i] != b1[i] { break; } i += 1;
            }
            let c = b0[i] ^ b1[i]; let mut bit = 7i32;
            while (c >> bit) == 0 { bit -= 1; }
            let crit = ((i as i32) << 3) + 7 - bit + 1;
            if (b0[i] >> bit) & 1 != 0 { crit } else { -crit }
        }));
        c.cmp = Some(Box::new(move |_: &Cbt, k0: &dyn Any, k1: &dyn Any| {
            let (b0, b1) = (k0.downcast_ref::<Vec<u8>>().unwrap(), k1.downcast_ref::<Vec<u8>>().unwrap());
            b0[..len as usize].cmp(&b1[..len as usize]) as i32
        }));
        c.getlen = Some(Box::new(move |_: &Cbt, _: &dyn Any| len));
        c.dup = Some(Box::new(move |_: &Cbt, k: &dyn Any| Box::new(k.downcast_ref::<Vec<u8>>().unwrap().clone()) as Box<dyn Any>));
        c
    }
    pub fn cbt_new_enc() -> Self {
        let mut c = Self::init();
        fn enc_len(k: &dyn Any) -> i32 { let b = k.downcast_ref::<Vec<u8>>().unwrap(); b[0] as i32 + ((b[1] as i32) << 8) }
        c.getlen = Some(Box::new(|_: &Cbt, k: &dyn Any| enc_len(k)));
        c.cmp = Some(Box::new(|_: &Cbt, k0: &dyn Any, k1: &dyn Any| {
            let (b0, b1) = (k0.downcast_ref::<Vec<u8>>().unwrap(), k1.downcast_ref::<Vec<u8>>().unwrap());
            let (l0, l1) = (enc_len(k0), enc_len(k1));
            if l0 != l1 { 1 } else { b0[..l0 as usize +2].cmp(&b1[..l0 as usize +2]) as i32 }
        }));
        c.dup = Some(Box::new(|_: &Cbt, k: &dyn Any| {
            let b = k.downcast_ref::<Vec<u8>>().unwrap();
            let l = enc_len(k) as usize + 2; Box::new(b[..l].to_vec()) as Box<dyn Any>
        }));
        c.getcrit = Some(Box::new(|_: &Cbt, k0: &dyn Any, k1: &dyn Any| {
            let (b0, b1) = (k0.downcast_ref::<Vec<u8>>().unwrap(), k1.downcast_ref::<Vec<u8>>().unwrap());
            let n = enc_len(k0).min(enc_len(k1)) as usize;
            let limit = n + 2; let mut i = 0;
            loop { if i == limit { return 0; } if b0[i] != b1[i] { break; } i += 1; }
            let c = b0[i] ^ b1[i]; let mut bit = 7i32;
            while (c >> bit) == 0 { bit -= 1; }
            let crit = ((i as i32) << 3) + 7 - bit + 1;
            if (b0[i] >> bit) & 1 != 0 { crit } else { -crit }
        }));
        c
    }
    pub fn cbt_delete(mut self) {
        self.cbt_remove_all();
    }
    pub fn cbt_size(&self) -> i32 { self.count }
    pub fn cbt_first(&self) -> Option<CbtLeaf> { self.first.as_ref().map(snap) }
    pub fn cbt_last(&self) -> Option<CbtLeaf> { self.last.as_ref().map(snap) }
    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> { _leaf.next.as_ref().map(snap) }
    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            if rc.borrow().key == _leaf.key { rc.borrow_mut().data = _data; return; }
            let n = rc.borrow().next.clone(); cur = n;
        }
    }
    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> { Some(Box::new(())) }
    pub fn cbt_key(&self, _leaf: &CbtLeaf) -> &str {
        // Safety: the returned &str borrows from _leaf which the caller ensures outlives the return
        unsafe { &*(_leaf.key.as_str() as *const str) }
    }

    fn walk_to_leaf<'a>(&self, key: &str) -> Option<CbtLeafPtr> {
        let root = self.root.as_ref()?;
        let ka: Box<dyn Any> = Box::new(key.to_string());
        let len = ((self.getlen.as_ref().unwrap())(self, &*ka) << 3) - 1;
        let mut p = root.as_ref();
        loop {
            if is_leaf(p) { break; }
            if (len as i16) < p.crit {
                loop { p = p.left.as_ref().unwrap(); if is_leaf(p) { break; } }
                break;
            }
            p = if testbit(key.as_bytes(), p.crit) { p.right.as_ref().unwrap() } else { p.left.as_ref().unwrap() };
        }
        Some(borrow_leaf(p))
    }
    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        let lp = self.walk_to_leaf(key)?;
        let ka: Box<dyn Any> = Box::new(key.to_string());
        let lk: Box<dyn Any> = Box::new(lp.borrow().key.clone());
        if (self.cmp.as_ref().unwrap())(self, &*lk, &*ka) == 0 { Some(snap(&lp)) } else { None }
    }
    pub fn cbt_has(&self, key: &str) -> bool { self.cbt_at(key).is_some() }
    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        self.cbt_at(key).map(|_| Box::new(()) as Box<dyn Any>)
    }
    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let s = snap(&rc); f(&s);
            let n = rc.borrow().next.clone(); cur = n;
        }
    }
    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let (k, n) = { let b = rc.borrow(); (b.key.clone(), b.next.clone()) };
            f(Box::new(()), &k); cur = n;
        }
    }

    fn insert_with_fn(&mut self, key: &str, mut f: Box<dyn FnMut(Box<dyn Any>) -> Box<dyn Any> + '_>) -> (bool, CbtLeafPtr) {
        let ka: Box<dyn Any> = Box::new(key.to_string());
        if self.root.is_none() {
            let data = f(Box::new(()));
            let leaf = Rc::new(RefCell::new(CbtLeaf {
                crit: EXT, data, key: key.to_string(), prev: None, next: None,
            }));
            self.root = Some(store_leaf(leaf.clone()));
            self.first = Some(leaf.clone()); self.last = Some(leaf.clone());
            self.count += 1;
            return (true, leaf);
        }
        // Walk to a leaf
        let kb = key.as_bytes();
        let keylen = ((self.getlen.as_ref().unwrap())(self, &*ka) << 3) - 1;
        let leaf_ptr = {
            let mut p = self.root.as_ref().unwrap().as_ref();
            loop {
                if is_leaf(p) { break borrow_leaf(p); }
                p = if (keylen as i16) < p.crit || testbit(kb, p.crit) {
                    p.right.as_ref().unwrap()
                } else {
                    p.left.as_ref().unwrap()
                };
            }
        };
        let leaf_key: Box<dyn Any> = Box::new(leaf_ptr.borrow().key.clone());
        let res = (self.getcrit.as_ref().unwrap())(self, &*ka, &*leaf_key);
        if res == 0 {
            let old_data = std::mem::replace(&mut leaf_ptr.borrow_mut().data, Box::new(()));
            leaf_ptr.borrow_mut().data = f(old_data);
            return (false, leaf_ptr);
        }
        self.count += 1;
        let data = f(Box::new(()));
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: EXT, data, key: key.to_string(), prev: None, next: None,
        }));
        let new_leaf_node = store_leaf(new_leaf.clone());
        let pnode_crit = (res.unsigned_abs() as i16) - 1;
        // Walk to find insertion point
        let mut path: Vec<bool> = Vec::new(); // true = went right
        {
            let mut p = self.root.as_ref().unwrap().as_ref();
            while !is_leaf(p) && pnode_crit > p.crit {
                let go_right = testbit(kb, p.crit);
                path.push(go_right);
                p = if go_right { p.right.as_ref().unwrap() } else { p.left.as_ref().unwrap() };
            }
        }
        // Navigate to insertion point and splice
        let mut p = self.root.as_mut().unwrap().as_mut();
        for &went_right in &path {
            p = if went_right { p.right.as_mut().unwrap().as_mut() } else { p.left.as_mut().unwrap().as_mut() };
        }
        // p is the subtree to split. Take it out and replace with new internal node.
        let old_subtree = std::mem::replace(p, CbtNode { crit: 0, left: None, right: None });
        if res > 0 {
            // new key is bigger, goes right
            // Find rightmost leaf of left subtree (old_subtree) for linked list
            let pred = rightmost_leaf(&old_subtree);
            { let mut nl = new_leaf.borrow_mut();
              nl.next = pred.borrow().next.clone();
              nl.prev = Some(Rc::downgrade(&pred));
            }
            if let Some(ref next) = new_leaf.borrow().next {
                next.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            } else { self.last = Some(new_leaf.clone()); }
            pred.borrow_mut().next = Some(new_leaf.clone());
            *p = CbtNode { crit: pnode_crit, left: Some(Box::new(old_subtree)), right: Some(new_leaf_node) };
        } else {
            // new key is smaller, goes left
            let succ = leftmost_leaf(&old_subtree);
            { let mut nl = new_leaf.borrow_mut();
              nl.prev = succ.borrow().prev.clone();
              nl.next = Some(succ.clone());
            }
            if let Some(ref prev_weak) = new_leaf.borrow().prev {
                if let Some(prev) = prev_weak.upgrade() { prev.borrow_mut().next = Some(new_leaf.clone()); }
            } else { self.first = Some(new_leaf.clone()); }
            succ.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            *p = CbtNode { crit: pnode_crit, left: Some(new_leaf_node), right: Some(Box::new(old_subtree)) };
        }
        (true, new_leaf)
    }

    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let d = RefCell::new(Some(data));
        let (_, lp) = self.insert_with_fn(key, Box::new(move |_| d.borrow_mut().take().unwrap_or(Box::new(()))));
        snap(&lp)
    }
    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        _f: F,
        key: &str,
    ) -> CbtLeaf {
        let f: Box<dyn FnMut(Box<dyn Any>) -> Box<dyn Any>> = Box::new(_f);
        let (_, lp) = self.insert_with_fn(key, f);
        snap(&lp)
    }
    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let (is_new, lp) = self.insert_with_fn(key, Box::new(|old| old));
        (is_new, snap(&lp))
    }

    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        if self.root.is_none() { return None; }
        let kb = key.as_bytes();
        // Walk with parent tracking
        let mut path: Vec<bool> = Vec::new();
        {
            let mut p = self.root.as_ref().unwrap().as_ref();
            while !is_leaf(p) {
                let go_right = testbit(kb, p.crit);
                path.push(go_right);
                p = if go_right { p.right.as_ref().unwrap() } else { p.left.as_ref().unwrap() };
            }
        }
        // Navigate to the leaf
        let leaf_ptr = {
            let mut p = self.root.as_ref().unwrap().as_ref();
            for &went_right in &path { p = if went_right { p.right.as_ref().unwrap() } else { p.left.as_ref().unwrap() }; }
            borrow_leaf(p)
        };
        self.count -= 1;
        // Unlink from linked list
        let (prev_weak, next_rc) = {
            let b = leaf_ptr.borrow();
            (b.prev.clone(), b.next.clone())
        };
        if let Some(ref next) = next_rc {
            next.borrow_mut().prev = prev_weak.clone();
        } else {
            self.last = prev_weak.as_ref().and_then(|w| w.upgrade());
        }
        if let Some(ref pw) = prev_weak {
            if let Some(prev) = pw.upgrade() { prev.borrow_mut().next = next_rc; }
        } else {
            self.first = next_rc;
        }
        let data = std::mem::replace(&mut leaf_ptr.borrow_mut().data, Box::new(()));
        // Remove from tree
        if path.is_empty() {
            // Root is the leaf
            let old = self.root.take().unwrap();
            let _ = extract_leaf(old);
            return Some(data);
        }
        // Navigate to parent, replace parent with sibling
        let parent_path = &path[..path.len()-1];
        let last_dir = *path.last().unwrap();
        let mut p = self.root.as_mut().unwrap().as_mut();
        for &went_right in parent_path {
            p = if went_right { p.right.as_mut().unwrap().as_mut() } else { p.left.as_mut().unwrap().as_mut() };
        }
        // p is the parent node. Replace it with the sibling.
        let sibling = if last_dir {
            let old_right = p.right.take().unwrap();
            let _ = extract_leaf(old_right);
            p.left.take().unwrap()
        } else {
            let old_left = p.left.take().unwrap();
            let _ = extract_leaf(old_left);
            p.right.take().unwrap()
        };
        *p = *sibling;
        Some(data)
    }

    pub fn cbt_remove_all(&mut self) {
        fn free_tree(node: Box<CbtNode>) {
            if is_leaf(&node) { let _ = extract_leaf(node); return; }
            if let Some(l) = node.left { free_tree(l); }
            if let Some(r) = node.right { free_tree(r); }
        }
        if let Some(root) = self.root.take() { free_tree(root); }
        self.count = 0; self.first = None; self.last = None;
    }
    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let (k, n) = { let b = rc.borrow(); (b.key.clone(), b.next.clone()) };
            let data = std::mem::replace(&mut rc.borrow_mut().data, Box::new(()));
            f(data, &k); cur = n;
        }
        self.cbt_remove_all();
    }
    pub fn cbt_overhead(&self) -> usize {
        let n = std::mem::size_of::<Cbt>();
        if self.root.is_none() { return n; }
        fn add(node: &CbtNode) -> usize {
            if is_leaf(node) { return std::mem::size_of::<CbtLeaf>(); }
            let mut s = std::mem::size_of::<CbtNode>();
            if let Some(ref l) = node.left { s += add(l); }
            if let Some(ref r) = node.right { s += add(r); }
            s
        }
        n + add(self.root.as_ref().unwrap())
    }
}

impl Drop for Cbt {
    fn drop(&mut self) {
        self.cbt_remove_all();
    }
}

fn rightmost_leaf(node: &CbtNode) -> CbtLeafPtr {
    let mut p = node;
    while !is_leaf(p) { p = p.right.as_ref().unwrap(); }
    borrow_leaf(p)
}
fn leftmost_leaf(node: &CbtNode) -> CbtLeafPtr {
    let mut p = node;
    while !is_leaf(p) { p = p.left.as_ref().unwrap(); }
    borrow_leaf(p)
}
