use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Represents an internal CBT node (non‐leaf).
#[derive(Debug)]
pub struct CbtNode {
    pub crit: i16,
    pub left: Option<Box<CbtNode>>,
    pub right: Option<Box<CbtNode>>,
}

/// Represents a leaf node in the crit‐bit tree.
#[derive(Debug)]
pub struct CbtLeaf {
    pub crit: i16,
    pub data: Box<dyn Any>,
    pub key: String,
    pub prev: Option<Weak<RefCell<CbtLeaf>>>,
    pub next: Option<Rc<RefCell<CbtLeaf>>>,
}

pub type CbtLeafPtr = Rc<RefCell<CbtLeaf>>;
pub type DupFn = dyn Fn(&Cbt, &dyn Any) -> Box<dyn Any>;
pub type GetLenFn = dyn Fn(&Cbt, &dyn Any) -> i32;
pub type CmpFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;
pub type GetCritFn = dyn Fn(&Cbt, &dyn Any, &dyn Any) -> i32;

// Internal tree enum: either an internal node or a leaf (Rc pointer).
enum Node {
    Internal { crit: i16, left: Box<Node>, right: Box<Node> },
    Leaf(CbtLeafPtr),
}

pub struct Cbt {
    pub count: i32,
    pub root: Option<Box<CbtNode>>,  // unused; we use `tree` instead
    pub first: Option<CbtLeafPtr>,
    pub last: Option<CbtLeafPtr>,
    pub dup: Option<Box<DupFn>>,
    pub getlen: Option<Box<GetLenFn>>,
    pub cmp: Option<Box<CmpFn>>,
    pub getcrit: Option<Box<GetCritFn>>,
    pub len: i32,
    // Private field for actual tree storage
    tree: Option<Box<Node>>,
}

const EXT: i16 = -1;

fn testbit(key: &[u8], bit: i16) -> bool {
    let byte_idx = (bit >> 3) as usize;
    let bit_idx = 7 - (bit & 7);
    if byte_idx < key.len() {
        (key[byte_idx] >> bit_idx) & 1 != 0
    } else {
        false
    }
}

fn clone_leaf(leaf: &CbtLeafPtr) -> CbtLeaf {
    let b = leaf.borrow();
    CbtLeaf {
        crit: b.crit,
        data: clone_any(&*b.data),
        key: b.key.clone(),
        prev: b.prev.clone(),
        next: b.next.clone(),
    }
}

fn clone_any(a: &dyn Any) -> Box<dyn Any> {
    if let Some(v) = a.downcast_ref::<i64>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<i32>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<usize>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<isize>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<String>() { return Box::new(v.clone()); }
    if let Some(v) = a.downcast_ref::<()>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<bool>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<u64>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<u32>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<f64>() { return Box::new(*v); }
    if let Some(v) = a.downcast_ref::<Vec<u8>>() { return Box::new(v.clone()); }
    Box::new(())
}

// Default ASCIIZ mode functions
fn getcrit_default(_cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<String>().unwrap();
    let k1 = key1.downcast_ref::<String>().unwrap();
    let b0 = k0.as_bytes();
    let b1 = k1.as_bytes();
    let mut i = 0;
    loop {
        let c0 = if i < b0.len() { b0[i] } else { 0 };
        let c1 = if i < b1.len() { b1[i] } else { 0 };
        if c0 == c1 {
            if c0 == 0 { return 0; }
            i += 1;
            continue;
        }
        let c = c0 ^ c1;
        let mut bit = 7;
        while (c >> bit) == 0 { bit -= 1; }
        let crit = ((i as i32) << 3) + 7 - bit as i32 + 1;
        if (c0 >> bit) & 1 != 0 { crit } else { -crit };
        return if (c0 >> bit) & 1 != 0 { crit } else { -crit };
    }
}

fn cmp_default(_cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<String>().unwrap();
    let k1 = key1.downcast_ref::<String>().unwrap();
    k0.cmp(k1) as i32
}

fn getlen_default(_cbt: &Cbt, key: &dyn Any) -> i32 {
    let k = key.downcast_ref::<String>().unwrap();
    k.len() as i32 + 1
}

fn dup_default(_cbt: &Cbt, key: &dyn Any) -> Box<dyn Any> {
    let k = key.downcast_ref::<String>().unwrap();
    Box::new(k.clone())
}

// "u" mode functions
fn getcrit_u(cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<Vec<u8>>().unwrap();
    let k1 = key1.downcast_ref::<Vec<u8>>().unwrap();
    let len = cbt.len as usize;
    for i in 0..len {
        if k0[i] != k1[i] {
            let c = k0[i] ^ k1[i];
            let mut bit: i32 = 7;
            while (c >> bit) == 0 { bit -= 1; }
            let crit = ((i as i32) << 3) + 7 - bit + 1;
            return if (k0[i] >> bit) & 1 != 0 { crit } else { -crit };
        }
    }
    0
}

fn cmp_u(cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<Vec<u8>>().unwrap();
    let k1 = key1.downcast_ref::<Vec<u8>>().unwrap();
    let len = cbt.len as usize;
    k0[..len].cmp(&k1[..len]) as i32
}

fn getlen_u(cbt: &Cbt, _key: &dyn Any) -> i32 { cbt.len }

fn dup_u(cbt: &Cbt, key: &dyn Any) -> Box<dyn Any> {
    let k = key.downcast_ref::<Vec<u8>>().unwrap();
    Box::new(k[..cbt.len as usize].to_vec())
}

// "enc" mode functions
fn getlen_enc(_cbt: &Cbt, key: &dyn Any) -> i32 {
    let k = key.downcast_ref::<Vec<u8>>().unwrap();
    k[0] as i32 + ((k[1] as i32) << 8)
}

fn cmp_enc(_cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<Vec<u8>>().unwrap();
    let k1 = key1.downcast_ref::<Vec<u8>>().unwrap();
    let len0 = k0[0] as usize + ((k0[1] as usize) << 8);
    let len1 = k1[0] as usize + ((k1[1] as usize) << 8);
    if len0 != len1 { return 1; }
    k0[..len0+2].cmp(&k1[..len0+2]) as i32
}

fn dup_enc(_cbt: &Cbt, key: &dyn Any) -> Box<dyn Any> {
    let k = key.downcast_ref::<Vec<u8>>().unwrap();
    let len = k[0] as usize + ((k[1] as usize) << 8) + 2;
    Box::new(k[..len].to_vec())
}

fn getcrit_enc(_cbt: &Cbt, key0: &dyn Any, key1: &dyn Any) -> i32 {
    let k0 = key0.downcast_ref::<Vec<u8>>().unwrap();
    let k1 = key1.downcast_ref::<Vec<u8>>().unwrap();
    let n0 = k0[0] as usize + ((k0[1] as usize) << 8);
    let n1 = k1[0] as usize + ((k1[1] as usize) << 8);
    let n = n0.min(n1) + 2;
    for i in 0..n {
        if k0[i] != k1[i] {
            let c = k0[i] ^ k1[i];
            let mut bit: i32 = 7;
            while (c >> bit) == 0 { bit -= 1; }
            let crit = ((i as i32) << 3) + 7 - bit + 1;
            return if (k0[i] >> bit) & 1 != 0 { crit } else { -crit };
        }
    }
    0
}

impl Node {
    fn find_leaf(&self) -> &CbtLeafPtr {
        match self {
            Node::Leaf(l) => l,
            Node::Internal { left, .. } => left.find_leaf(),
        }
    }
    fn find_leaf_right(&self) -> &CbtLeafPtr {
        match self {
            Node::Leaf(l) => l,
            Node::Internal { right, .. } => right.find_leaf_right(),
        }
    }
    fn find_leaf_left(&self) -> &CbtLeafPtr {
        match self {
            Node::Leaf(l) => l,
            Node::Internal { left, .. } => left.find_leaf_left(),
        }
    }
    fn overhead(&self) -> usize {
        match self {
            Node::Leaf(_) => std::mem::size_of::<CbtLeaf>(),
            Node::Internal { left, right, .. } => {
                std::mem::size_of::<CbtNode>() + left.overhead() + right.overhead()
            }
        }
    }
    fn forall_with(&self, f: &mut dyn FnMut(&dyn Any, &str)) {
        match self {
            Node::Leaf(l) => {
                let b = l.borrow();
                f(&*b.data, &b.key);
            }
            Node::Internal { left, right, .. } => {
                left.forall_with(f);
                right.forall_with(f);
            }
        }
    }
}

impl Cbt {
    fn new_empty() -> Self {
        Cbt {
            count: 0,
            root: None,
            first: None,
            last: None,
            dup: None,
            getlen: None,
            cmp: None,
            getcrit: None,
            len: 0,
            tree: None,
        }
    }

    fn key_to_any(&self, key: &str) -> Box<dyn Any> {
        Box::new(key.to_string())
    }

    fn key_bytes(&self, key: &str) -> Vec<u8> {
        let mut v: Vec<u8> = key.as_bytes().to_vec();
        v.push(0); // NUL terminator for ASCIIZ
        v
    }

    fn leaf_key_bytes(leaf: &CbtLeafPtr) -> Vec<u8> {
        let b = leaf.borrow();
        let mut v: Vec<u8> = b.key.as_bytes().to_vec();
        v.push(0);
        v
    }

    fn do_getcrit(&self, key0: &dyn Any, key1: &dyn Any) -> i32 {
        (self.getcrit.as_ref().unwrap())(self, key0, key1)
    }

    fn do_cmp(&self, key0: &dyn Any, key1: &dyn Any) -> i32 {
        (self.cmp.as_ref().unwrap())(self, key0, key1)
    }

    fn do_getlen(&self, key: &dyn Any) -> i32 {
        (self.getlen.as_ref().unwrap())(self, key)
    }

    fn walk_to_leaf<'a>(node: &'a Node, key_bytes: &[u8], keylen: i16) -> &'a CbtLeafPtr {
        match node {
            Node::Leaf(l) => l,
            Node::Internal { crit, left, right, .. } => {
                if keylen < *crit {
                    // Follow left until we hit a leaf
                    let mut n = left.as_ref();
                    loop {
                        match n {
                            Node::Leaf(l) => return l,
                            Node::Internal { left, .. } => n = left.as_ref(),
                        }
                    }
                }
                if testbit(key_bytes, *crit) {
                    Self::walk_to_leaf(right, key_bytes, keylen)
                } else {
                    Self::walk_to_leaf(left, key_bytes, keylen)
                }
            }
        }
    }

    pub fn cbt_new() -> Self {
        let mut cbt = Self::new_empty();
        cbt.cmp = Some(Box::new(cmp_default));
        cbt.dup = Some(Box::new(dup_default));
        cbt.getlen = Some(Box::new(getlen_default));
        cbt.getcrit = Some(Box::new(getcrit_default));
        cbt
    }

    pub fn cbt_new_u(len: i32) -> Self {
        let mut cbt = Self::new_empty();
        cbt.len = len;
        cbt.cmp = Some(Box::new(cmp_u));
        cbt.dup = Some(Box::new(dup_u));
        cbt.getlen = Some(Box::new(getlen_u));
        cbt.getcrit = Some(Box::new(getcrit_u));
        cbt
    }

    pub fn cbt_new_enc() -> Self {
        let mut cbt = Self::new_empty();
        cbt.cmp = Some(Box::new(cmp_enc));
        cbt.dup = Some(Box::new(dup_enc));
        cbt.getlen = Some(Box::new(getlen_enc));
        cbt.getcrit = Some(Box::new(getcrit_enc));
        cbt
    }

    pub fn cbt_delete(self) {
        // Drop
    }

    pub fn cbt_get_at(&self, key: &str) -> Option<Box<dyn Any>> {
        let leaf = self.cbt_at(key)?;
        Some(leaf.data)
    }

    pub fn cbt_put_at(&mut self, data: Box<dyn Any>, key: &str) -> CbtLeaf {
        let mut data_opt = Some(data);
        self.cbt_put_with(|_| data_opt.take().unwrap_or_else(|| Box::new(())), key)
    }

    pub fn cbt_size(&self) -> i32 {
        self.count
    }

    pub fn cbt_first(&self) -> Option<CbtLeaf> {
        self.first.as_ref().map(clone_leaf)
    }

    pub fn cbt_last(&self) -> Option<CbtLeaf> {
        self.last.as_ref().map(clone_leaf)
    }

    pub fn cbt_next(_leaf: &CbtLeaf) -> Option<CbtLeaf> {
        _leaf.next.as_ref().map(clone_leaf)
    }

    pub fn cbt_put(&mut self, _leaf: &mut CbtLeaf, _data: Box<dyn Any>) {
        // Find the actual leaf in the linked list and update its data
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            if rc.borrow().key == _leaf.key {
                rc.borrow_mut().data = _data;
                return;
            }
            let next = rc.borrow().next.clone();
            cur = next;
        }
    }

    pub fn cbt_get(&self, _leaf: &CbtLeaf) -> Option<Box<dyn Any>> {
        Some(clone_any(&*_leaf.data))
    }

    pub fn cbt_key<'a>(&self, _leaf: &'a CbtLeaf) -> &'a str {
        &_leaf.key
    }

    pub fn cbt_at(&self, key: &str) -> Option<CbtLeaf> {
        let tree = self.tree.as_ref()?;
        let key_any = self.key_to_any(key);
        let keylen = (self.do_getlen(&*key_any) << 3) - 1;
        let key_bytes = self.key_bytes(key);
        let leaf = Self::walk_to_leaf(tree, &key_bytes, keylen as i16);
        let leaf_key_any: Box<dyn Any> = Box::new(leaf.borrow().key.clone());
        if self.do_cmp(&*key_any, &*leaf_key_any) == 0 {
            Some(clone_leaf(leaf))
        } else {
            None
        }
    }

    pub fn cbt_has(&self, key: &str) -> bool {
        self.cbt_at(key).is_some()
    }

    pub fn cbt_forall<F: FnMut(&CbtLeaf)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let leaf = clone_leaf(&rc);
            _f(&leaf);
            let next = rc.borrow().next.clone();
            cur = next;
        }
    }

    pub fn cbt_forall_at<F: FnMut(Box<dyn Any>, &str)>(&self, mut _f: F) {
        let mut cur = self.first.clone();
        while let Some(rc) = cur {
            let b = rc.borrow();
            _f(clone_any(&*b.data), &b.key);
            let next = b.next.clone();
            drop(b);
            cur = next;
        }
    }

    pub fn cbt_remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        let key_any = self.key_to_any(key);
        let key_bytes = self.key_bytes(key);

        // Find and remove from tree
        let tree = self.tree.take()?;
        let (new_tree, removed_leaf) = self.remove_from_tree(tree, &key_bytes, &key_any);
        self.tree = new_tree.map(Box::new);

        let leaf_rc = removed_leaf?;
        self.count -= 1;

        // Update linked list
        let prev = leaf_rc.borrow().prev.clone();
        let next = leaf_rc.borrow().next.clone();

        if let Some(ref next_rc) = next {
            next_rc.borrow_mut().prev = prev.clone();
        } else {
            self.last = prev.as_ref().and_then(|w| w.upgrade());
        }

        if let Some(ref prev_weak) = prev {
            if let Some(prev_rc) = prev_weak.upgrade() {
                prev_rc.borrow_mut().next = next;
            }
        } else {
            self.first = next;
        }

        let data = std::mem::replace(&mut leaf_rc.borrow_mut().data, Box::new(()));
        Some(data)
    }

    fn remove_from_tree(&self, node: Box<Node>, key_bytes: &[u8], key_any: &Box<dyn Any>) -> (Option<Node>, Option<CbtLeafPtr>) {
        match *node {
            Node::Leaf(ref l) => {
                let leaf_key: Box<dyn Any> = Box::new(l.borrow().key.clone());
                if self.do_cmp(&**key_any, &*leaf_key) == 0 {
                    if let Node::Leaf(l) = *node {
                        (None, Some(l))
                    } else {
                        unreachable!()
                    }
                } else {
                    (Some(*node), None)
                }
            }
            Node::Internal { crit, left, right } => {
                let keylen = (self.do_getlen(&**key_any) << 3) - 1;
                if (keylen as i16) < crit {
                    // Key is shorter than crit, shouldn't happen if key exists
                    return (Some(Node::Internal { crit, left, right }), None);
                }
                if testbit(key_bytes, crit) {
                    let (new_right, removed) = self.remove_from_tree(right, key_bytes, key_any);
                    match new_right {
                        None => (Some(*left), removed),
                        Some(r) => (Some(Node::Internal { crit, left, right: Box::new(r) }), removed),
                    }
                } else {
                    let (new_left, removed) = self.remove_from_tree(left, key_bytes, key_any);
                    match new_left {
                        None => (Some(*right), removed),
                        Some(l) => (Some(Node::Internal { crit, left: Box::new(l), right }), removed),
                    }
                }
            }
        }
    }

    pub fn cbt_remove_all(&mut self) {
        self.tree = None;
        self.root = None;
        self.count = 0;
        self.first = None;
        self.last = None;
    }

    pub fn cbt_remove_all_with<F: FnMut(Box<dyn Any>, &str)>(&mut self, mut _f: F) {
        if let Some(tree) = self.tree.take() {
            Self::clear_recurse(*tree, &mut _f);
        }
        self.count = 0;
        self.first = None;
        self.last = None;
    }

    fn clear_recurse<F: FnMut(Box<dyn Any>, &str)>(node: Node, f: &mut F) {
        match node {
            Node::Leaf(l) => {
                let mut b = l.borrow_mut();
                let data = std::mem::replace(&mut b.data, Box::new(()));
                let key = b.key.clone();
                f(data, &key);
            }
            Node::Internal { left, right, .. } => {
                Self::clear_recurse(*left, f);
                Self::clear_recurse(*right, f);
            }
        }
    }

    pub fn cbt_put_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        mut _f: F,
        key: &str,
    ) -> CbtLeaf {
        let (_, leaf) = self.cbt_insert_with(&mut _f, key);
        leaf
    }

    fn cbt_insert_with<F: FnMut(Box<dyn Any>) -> Box<dyn Any>>(
        &mut self,
        f: &mut F,
        key: &str,
    ) -> (bool, CbtLeaf) {
        let key_any = self.key_to_any(key);
        let key_bytes = self.key_bytes(key);

        if self.tree.is_none() {
            let data = f(Box::new(()));
            let leaf = Rc::new(RefCell::new(CbtLeaf {
                crit: EXT,
                data,
                key: key.to_string(),
                prev: None,
                next: None,
            }));
            self.first = Some(leaf.clone());
            self.last = Some(leaf.clone());
            self.tree = Some(Box::new(Node::Leaf(leaf.clone())));
            self.count = 1;
            return (true, clone_leaf(&leaf));
        }

        // Walk to a leaf
        let keylen = (self.do_getlen(&*key_any) << 3) - 1;
        let tree = self.tree.as_ref().unwrap();
        let existing_leaf = Self::walk_to_leaf(tree, &key_bytes, keylen as i16).clone();

        let leaf_key_any: Box<dyn Any> = Box::new(existing_leaf.borrow().key.clone());
        let res = self.do_getcrit(&*key_any, &*leaf_key_any);

        if res == 0 {
            // Key already exists, update data
            let old_data = std::mem::replace(&mut existing_leaf.borrow_mut().data, Box::new(()));
            let new_data = f(old_data);
            existing_leaf.borrow_mut().data = new_data;
            return (false, clone_leaf(&existing_leaf));
        }

        self.count += 1;
        let new_crit = (res.abs() - 1) as i16;

        let data = f(Box::new(()));
        let new_leaf = Rc::new(RefCell::new(CbtLeaf {
            crit: EXT,
            data,
            key: key.to_string(),
            prev: None,
            next: None,
        }));

        // Insert into linked list
        if res > 0 {
            // New key is bigger, goes on right
            // Find rightmost leaf of left subtree = predecessor
            let tree = self.tree.as_ref().unwrap();
            let pred = Self::find_predecessor(tree, &key_bytes, new_crit);
            new_leaf.borrow_mut().next = pred.borrow().next.clone();
            new_leaf.borrow_mut().prev = Some(Rc::downgrade(&pred));
            if let Some(ref next) = pred.borrow().next {
                next.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
            } else {
                self.last = Some(new_leaf.clone());
            }
            pred.borrow_mut().next = Some(new_leaf.clone());
        } else {
            // New key is smaller, goes on left
            let tree = self.tree.as_ref().unwrap();
            let succ = Self::find_successor(tree, &key_bytes, new_crit);
            new_leaf.borrow_mut().prev = succ.borrow().prev.clone();
            new_leaf.borrow_mut().next = Some(succ.clone());
            if let Some(ref prev_weak) = succ.borrow().prev {
                if let Some(prev_rc) = prev_weak.upgrade() {
                    prev_rc.borrow_mut().next = Some(new_leaf.clone());
                }
            } else {
                self.first = Some(new_leaf.clone());
            }
            succ.borrow_mut().prev = Some(Rc::downgrade(&new_leaf));
        }

        // Insert into tree
        let tree = self.tree.take().unwrap();
        let new_tree = Self::insert_node(*tree, new_crit, res > 0, new_leaf.clone(), &key_bytes);
        self.tree = Some(Box::new(new_tree));

        (true, clone_leaf(&new_leaf))
    }

    fn find_predecessor(tree: &Node, key_bytes: &[u8], new_crit: i16) -> CbtLeafPtr {
        // Walk down to where the new node will be inserted, then find rightmost of left subtree
        let mut node = tree;
        // Find the subtree that will become the left child
        loop {
            match node {
                Node::Leaf(l) => return l.clone(),
                Node::Internal { crit, left, right, .. } => {
                    if new_crit <= *crit { break; }
                    if testbit(key_bytes, *crit) {
                        node = right;
                    } else {
                        node = left;
                    }
                }
            }
        }
        // node is the subtree that will become left child (since res > 0, new key goes right)
        // Find rightmost leaf of this subtree
        node.find_leaf_right().clone()
    }

    fn find_successor(tree: &Node, key_bytes: &[u8], new_crit: i16) -> CbtLeafPtr {
        let mut node = tree;
        loop {
            match node {
                Node::Leaf(l) => return l.clone(),
                Node::Internal { crit, left, right, .. } => {
                    if new_crit <= *crit { break; }
                    if testbit(key_bytes, *crit) {
                        node = right;
                    } else {
                        node = left;
                    }
                }
            }
        }
        // node is the subtree that will become right child (since res < 0, new key goes left)
        // Find leftmost leaf of this subtree
        node.find_leaf_left().clone()
    }

    fn insert_node(node: Node, new_crit: i16, goes_right: bool, new_leaf: CbtLeafPtr, key_bytes: &[u8]) -> Node {
        match node {
            Node::Internal { crit, left, right } if new_crit > crit => {
                if testbit(key_bytes, crit) {
                    Node::Internal {
                        crit,
                        left,
                        right: Box::new(Self::insert_node(*right, new_crit, goes_right, new_leaf, key_bytes)),
                    }
                } else {
                    Node::Internal {
                        crit,
                        left: Box::new(Self::insert_node(*left, new_crit, goes_right, new_leaf, key_bytes)),
                        right,
                    }
                }
            }
            other => {
                if goes_right {
                    Node::Internal {
                        crit: new_crit,
                        left: Box::new(other),
                        right: Box::new(Node::Leaf(new_leaf)),
                    }
                } else {
                    Node::Internal {
                        crit: new_crit,
                        left: Box::new(Node::Leaf(new_leaf)),
                        right: Box::new(other),
                    }
                }
            }
        }
    }

    pub fn cbt_insert(&mut self, key: &str) -> (bool, CbtLeaf) {
        let mut identity = |old: Box<dyn Any>| -> Box<dyn Any> { old };
        self.cbt_insert_with(&mut identity, key)
    }

    pub fn cbt_overhead(&self) -> usize {
        let mut n = std::mem::size_of::<Cbt>();
        if let Some(ref tree) = self.tree {
            n += tree.overhead();
        }
        n
    }
}
