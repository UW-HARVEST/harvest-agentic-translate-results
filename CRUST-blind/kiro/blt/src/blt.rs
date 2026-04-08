use std::any::Any;

#[derive(Debug)]
pub struct Blt {
    pub root: Box<BltNode>,
    pub empty: i32,
}

#[derive(Debug)]
pub enum BltNode {
    Internal(InternalNode),
    Leaf(BltIt),
}

#[derive(Debug)]
pub struct InternalNode {
    pub byte: u32,
    pub mask: u8,
    pub padding: u32,
    pub kid: Box<BltNode>,
}

#[derive(Debug)]
pub struct BltIt {
    pub key: String,
    pub data: Option<Box<dyn Any>>,
}

fn alloc_pair(left: BltNode, right: BltNode) -> Box<BltNode> {
    let v: Vec<BltNode> = vec![left, right];
    let boxed_slice = v.into_boxed_slice();
    let ptr = Box::into_raw(boxed_slice) as *mut BltNode;
    unsafe { Box::from_raw(ptr) }
}

fn get_left(kid: &BltNode) -> &BltNode { kid }
fn get_right(kid: &BltNode) -> &BltNode { unsafe { &*(kid as *const BltNode).add(1) } }
fn get_left_mut(kid: &mut BltNode) -> &mut BltNode { kid }
fn get_right_mut(kid: &mut BltNode) -> &mut BltNode { unsafe { &mut *(kid as *mut BltNode).add(1) } }

fn take_pair(kid: Box<BltNode>) -> (BltNode, BltNode) {
    let ptr = Box::into_raw(kid);
    let v = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, 2)) }.into_vec();
    let mut it = v.into_iter();
    (it.next().unwrap(), it.next().unwrap())
}

fn to_mask(mut x: u8) -> u8 {
    x |= x >> 1; x |= x >> 2; x |= x >> 4;
    x & !(x >> 1)
}

fn firstlast(node: &BltNode, dir: usize) -> Option<BltIt> {
    let mut p = node;
    loop {
        match p {
            BltNode::Internal(n) => p = if dir == 0 { get_left(&n.kid) } else { get_right(&n.kid) },
            BltNode::Leaf(l) => return Some(BltIt { key: l.key.clone(), data: None }),
        }
    }
}

fn clone_leaf(l: &BltIt) -> BltIt { BltIt { key: l.key.clone(), data: None } }

// Walk to a leaf using confident_get logic, return the key found
fn confident_get_key(root: &BltNode, key: &[u8], keylen: usize) -> Option<String> {
    let mut p = root;
    loop {
        match p {
            BltNode::Internal(n) => {
                p = if (n.byte as usize) < keylen && (key[n.byte as usize] & n.mask != 0) {
                    get_right(&n.kid)
                } else {
                    get_left(&n.kid)
                };
            }
            BltNode::Leaf(l) => return Some(l.key.clone()),
        }
    }
}

impl Blt {
    pub fn blt_new() -> Self {
        Blt { root: Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None })), empty: 1 }
    }

    pub fn blt_clear(&mut self) {
        fn free_node(node: BltNode) {
            if let BltNode::Internal(n) = node {
                let (l, r) = take_pair(n.kid); free_node(l); free_node(r);
            }
        }
        if self.empty == 0 {
            let old = std::mem::replace(&mut self.root, Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None })));
            free_node(*old);
        }
        self.empty = 1;
    }

    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let kb = key.as_bytes();
        let keylen = kb.len();
        let mut p = &*self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if n.byte as usize > keylen { return None; }
                    p = if kb.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 { get_right(&n.kid) } else { get_left(&n.kid) };
                }
                BltNode::Leaf(l) => return if l.key == key { Some(clone_leaf(l)) } else { None },
            }
        }
    }

    pub fn blt_set(&mut self, key: &str) -> BltIt { self.blt_setp(key).0 }

    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let kb = key.as_bytes();
        if self.empty != 0 {
            self.empty = 0;
            self.root = Box::new(BltNode::Leaf(BltIt { key: key.to_string(), data: None }));
            return (BltIt { key: key.to_string(), data: None }, true);
        }
        let found_key = confident_get_key(&self.root, kb, kb.len()).unwrap();
        let pc = found_key.as_bytes();
        let mut i = 0;
        loop {
            let c = kb.get(i).copied().unwrap_or(0);
            let pc_b = pc.get(i).copied().unwrap_or(0);
            let x = c ^ pc_b;
            if x != 0 {
                let mask = to_mask(x);
                let byte = i as u32;
                let goes_right = c & mask != 0;
                // Find insertion point and insert using raw pointers
                let p: *mut BltNode = &mut *self.root;
                unsafe { Self::insert_node(p, key, kb, byte, mask, goes_right); }
                return (BltIt { key: key.to_string(), data: None }, true);
            }
            if c == 0 { return (BltIt { key: key.to_string(), data: None }, false); }
            i += 1;
        }
    }

    unsafe fn insert_node(mut p: *mut BltNode, key: &str, kb: &[u8], byte: u32, mask: u8, goes_right: bool) {
        // Walk to find the insertion point
        loop {
            match &*p {
                BltNode::Internal(n) => {
                    if ((byte << 8) as u64 + n.mask as u64) < ((n.byte << 8) as u64 + mask as u64) {
                        break;
                    }
                    let n_mut = match &mut *p { BltNode::Internal(n) => n, _ => unreachable!() };
                    p = if kb.get(n_mut.byte as usize).copied().unwrap_or(0) & n_mut.mask != 0 {
                        get_right_mut(&mut n_mut.kid) as *mut BltNode
                    } else {
                        get_left_mut(&mut n_mut.kid) as *mut BltNode
                    };
                }
                BltNode::Leaf(_) => break,
            }
        }
        let new_leaf = BltNode::Leaf(BltIt { key: key.to_string(), data: None });
        let old = std::mem::replace(&mut *p, BltNode::Leaf(BltIt { key: String::new(), data: None }));
        let (left, right) = if goes_right { (old, new_leaf) } else { (new_leaf, old) };
        *p = BltNode::Internal(InternalNode { byte, mask, padding: 0, kid: alloc_pair(left, right) });
    }

    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        self.blt_set(key);
        self.set_data(key, Some(data));
        BltIt { key: key.to_string(), data: None }
    }

    fn set_data(&mut self, key: &str, data: Option<Box<dyn Any>>) {
        if self.empty != 0 { return; }
        let kb = key.as_bytes();
        let mut p = &mut *self.root;
        loop {
            match p {
                BltNode::Internal(n) => {
                    p = if kb.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 { get_right_mut(&mut n.kid) } else { get_left_mut(&mut n.kid) };
                }
                BltNode::Leaf(l) => { if l.key == key { l.data = data; } return; }
            }
        }
    }

    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_, is_new) = self.blt_setp(key);
        if is_new { self.set_data(key, Some(data)); 0 } else { 1 }
    }

    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.empty != 0 { return 0; }
        let kb = key.as_bytes();
        let keylen = kb.len();
        // Check existence first
        if self.blt_get(key).is_none() { return 0; }
        // Root is a leaf
        if let BltNode::Leaf(ref l) = *self.root {
            if l.key == key { self.empty = 1; return 1; }
            return 0;
        }
        // Use raw pointers to navigate and delete
        let p: *mut BltNode = &mut *self.root;
        unsafe {
            let mut cur = p;
            let mut parent: *mut BltNode = std::ptr::null_mut();
            loop {
                match &*cur {
                    BltNode::Internal(n) => {
                        if n.byte as usize > keylen { return 0; }
                        parent = cur;
                        let n_mut = match &mut *cur { BltNode::Internal(n) => n, _ => unreachable!() };
                        cur = if kb.get(n_mut.byte as usize).copied().unwrap_or(0) & n_mut.mask != 0 {
                            get_right_mut(&mut n_mut.kid) as *mut BltNode
                        } else {
                            get_left_mut(&mut n_mut.kid) as *mut BltNode
                        };
                    }
                    BltNode::Leaf(l) => {
                        if l.key != key { return 0; }
                        break;
                    }
                }
            }
            // parent points to the internal node whose child is the leaf to delete
            let parent_node = match &mut *parent { BltNode::Internal(n) => n, _ => unreachable!() };
            let left_ptr = get_left(&parent_node.kid) as *const BltNode;
            let is_left = std::ptr::eq(left_ptr, cur as *const BltNode);
            let (left, right) = take_pair(std::mem::replace(&mut parent_node.kid, Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None }))));
            let sibling = if is_left { right } else { left };
            *parent = sibling;
        }
        1
    }

    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        if self.empty != 0 { return 1; }
        let kb = prefix.as_bytes();
        let keylen = kb.len();
        let mut p = &*self.root;
        let mut top = p;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if n.byte as usize >= keylen {
                        p = get_left(&n.kid);
                    } else {
                        p = if kb[n.byte as usize] & n.mask != 0 { get_right(&n.kid) } else { get_left(&n.kid) };
                        top = p;
                    }
                }
                BltNode::Leaf(l) => {
                    if keylen > l.key.len() || l.key.as_bytes()[..keylen] != kb[..keylen] { return 1; }
                    break;
                }
            }
        }
        fn traverse<F: FnMut(&BltIt) -> i32>(node: &BltNode, fun: &mut F) -> i32 {
            match node {
                BltNode::Internal(n) => {
                    let s = traverse(get_left(&n.kid), fun);
                    if s != 1 { return s; }
                    traverse(get_right(&n.kid), fun)
                }
                BltNode::Leaf(l) => fun(l),
            }
        }
        traverse(top, &mut fun)
    }

    pub fn blt_forall<F: FnMut(&BltIt)>(&self, mut fun: F) {
        let _ = self.blt_allprefixed("", |it| { fun(it); 1 });
    }

    pub fn blt_first(&self) -> Option<BltIt> { if self.empty != 0 { None } else { firstlast(&self.root, 0) } }
    pub fn blt_last(&self) -> Option<BltIt> { if self.empty != 0 { None } else { firstlast(&self.root, 1) } }

    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let kb = it.key.as_bytes();
        let mut p = &*self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if kb.get(n.byte as usize).copied().unwrap_or(0) & n.mask == 0 {
                        other = Some(get_right(&n.kid)); p = get_left(&n.kid);
                    } else { p = get_right(&n.kid); }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.and_then(|o| firstlast(o, 0))
    }

    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let kb = it.key.as_bytes();
        let mut p = &*self.root;
        let mut other: Option<&BltNode> = None;
        loop {
            match p {
                BltNode::Internal(n) => {
                    if kb.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 {
                        other = Some(get_left(&n.kid)); p = get_right(&n.kid);
                    } else { p = get_left(&n.kid); }
                }
                BltNode::Leaf(_) => break,
            }
        }
        other.and_then(|o| firstlast(o, 1))
    }

    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> { self.ceilfloor(key, 0) }
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> { self.ceilfloor(key, 1) }

    fn ceilfloor(&self, key: &str, way: usize) -> Option<BltIt> {
        if self.empty != 0 { return None; }
        let kb = key.as_bytes();
        let keylen = kb.len();
        let found_key = confident_get_key(&self.root, kb, keylen)?;
        let pc = found_key.as_bytes();
        let mut i = 0;
        loop {
            let c = kb.get(i).copied().unwrap_or(0);
            let pc_b = pc.get(i).copied().unwrap_or(0);
            let x = c ^ pc_b;
            if x != 0 {
                let byte = i as u32;
                let x_mask = to_mask(x);
                let mut p = &*self.root;
                let mut other: Option<&BltNode> = None;
                loop {
                    match p {
                        BltNode::Internal(n) => {
                            if ((byte << 8) as u64 + n.mask as u64) < ((n.byte << 8) as u64 + x_mask as u64) { break; }
                            let dir = if kb.get(n.byte as usize).copied().unwrap_or(0) & n.mask != 0 { 1usize } else { 0 };
                            if dir == way { other = Some(if way == 0 { get_right(&n.kid) } else { get_left(&n.kid) }); }
                            p = if dir != 0 { get_right(&n.kid) } else { get_left(&n.kid) };
                        }
                        BltNode::Leaf(_) => break,
                    }
                }
                let ndir = if kb.get(i).copied().unwrap_or(0) & x_mask != 0 { 1usize } else { 0 };
                if ndir == way { other = Some(p); }
                return other.and_then(|o| firstlast(o, way));
            }
            if c == 0 { return Some(BltIt { key: found_key.clone(), data: None }); }
            i += 1;
        }
    }

    pub fn blt_overhead(&self) -> usize {
        let base = std::mem::size_of::<Blt>();
        if self.empty != 0 { return base; }
        fn add(node: &BltNode) -> usize {
            match node {
                BltNode::Internal(n) => 2 * std::mem::size_of::<BltNode>() + add(get_left(&n.kid)) + add(get_right(&n.kid)),
                BltNode::Leaf(_) => 0,
            }
        }
        base + add(&self.root)
    }

    pub fn blt_empty(&self) -> bool { self.empty != 0 }

    pub fn blt_size(&self) -> i32 {
        let mut r = 0i32;
        self.blt_forall(|_| r += 1);
        r
    }
}
