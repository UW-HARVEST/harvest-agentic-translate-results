use std::any::Any;

/// The BLT tree structure.
#[derive(Debug)]
pub struct Blt {
    pub root: Box<BltNode>,
    pub empty: i32,
}

/// A node in the BLT tree.
#[derive(Debug)]
pub enum BltNode {
    Internal(InternalNode),
    Leaf(BltIt),
}

/// Represents an internal node in the BLT tree.
#[derive(Debug)]
pub struct InternalNode {
    pub byte: u32,
    pub mask: u8,
    pub padding: u32,
    pub kid: Box<BltNode>,
}

/// Represents a leaf node in the BLT tree.
#[derive(Debug)]
pub struct BltIt {
    pub key: String,
    pub data: Option<Box<dyn Any>>,
}

// Internal tree representation with proper two children per internal node.
enum INode {
    Internal { byte: u32, mask: u8, left: Box<INode>, right: Box<INode> },
    Leaf { key: String, data: Option<Box<dyn Any>> },
}

impl std::fmt::Debug for INode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            INode::Internal { byte, mask, .. } => write!(f, "Internal(byte={}, mask={})", byte, mask),
            INode::Leaf { key, .. } => write!(f, "Leaf({})", key),
        }
    }
}

fn to_mask(mut x: u8) -> u8 {
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x & !(x >> 1)
}

fn clone_data(data: &Option<Box<dyn Any>>) -> Option<Box<dyn Any>> {
    data.as_ref().map(|d| {
        if let Some(v) = d.downcast_ref::<i32>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<i64>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<usize>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<String>() { return Box::new(v.clone()) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<()>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<bool>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<u64>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<u32>() { return Box::new(*v) as Box<dyn Any>; }
        if let Some(v) = d.downcast_ref::<f64>() { return Box::new(*v) as Box<dyn Any>; }
        Box::new(0i32) as Box<dyn Any>
    })
}

fn inode_to_bltit(node: &INode) -> BltIt {
    match node {
        INode::Leaf { key, data } => BltIt { key: key.clone(), data: clone_data(data) },
        _ => unreachable!(),
    }
}

fn follow(byte: u32, mask: u8, key: &[u8]) -> bool {
    let b = if (byte as usize) < key.len() { key[byte as usize] } else { 0 };
    b & mask != 0
}

fn confident_get<'a>(node: &'a INode, key: &[u8], keylen: usize) -> &'a INode {
    let mut p = node;
    loop {
        match p {
            INode::Leaf { .. } => return p,
            INode::Internal { byte, mask, left, right } => {
                if (*byte as usize) < keylen && follow(*byte, *mask, key) {
                    p = right;
                } else {
                    p = left;
                }
            }
        }
    }
}

fn firstlast<'a>(node: &'a INode, go_right: bool) -> &'a INode {
    let mut p = node;
    loop {
        match p {
            INode::Leaf { .. } => return p,
            INode::Internal { left, right, .. } => {
                p = if go_right { right } else { left };
            }
        }
    }
}

impl Blt {
    fn get_tree(&self) -> Option<&INode> {
        if self.empty != 0 { return None; }
        match &*self.root {
            BltNode::Leaf(it) => {
                it.data.as_ref().and_then(|d| d.downcast_ref::<Box<INode>>()).map(|b| &**b)
            }
            _ => None,
        }
    }

    fn get_tree_mut(&mut self) -> Option<&mut INode> {
        if self.empty != 0 { return None; }
        match &mut *self.root {
            BltNode::Leaf(it) => {
                it.data.as_mut().and_then(|d| d.downcast_mut::<Box<INode>>()).map(|b| &mut **b)
            }
            _ => None,
        }
    }

    fn set_tree(&mut self, tree: INode) {
        self.empty = 0;
        self.root = Box::new(BltNode::Leaf(BltIt {
            key: String::new(),
            data: Some(Box::new(Box::new(tree))),
        }));
    }

    fn take_tree(&mut self) -> Option<Box<INode>> {
        if self.empty != 0 { return None; }
        match &mut *self.root {
            BltNode::Leaf(it) => {
                it.data.take().and_then(|d| d.downcast::<Box<INode>>().ok()).map(|b| *b)
            }
            _ => None,
        }
    }

    pub fn blt_new() -> Self {
        Blt {
            root: Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None })),
            empty: 1,
        }
    }

    pub fn blt_clear(&mut self) {
        self.empty = 1;
        self.root = Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None }));
    }

    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        let tree = self.get_tree()?;
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        let mut p = tree;
        loop {
            match p {
                INode::Leaf { key: k, data } => {
                    return if k == key { Some(BltIt { key: k.clone(), data: clone_data(data) }) } else { None };
                }
                INode::Internal { byte, mask, left, right } => {
                    if *byte as usize > keylen { return None; }
                    p = if follow(*byte, *mask, key_bytes) { right } else { left };
                }
            }
        }
    }

    pub fn blt_set(&mut self, key: &str) -> BltIt {
        self.blt_setp(key).0
    }

    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let key_bytes = key.as_bytes();

        if self.empty != 0 {
            let leaf = INode::Leaf { key: key.to_string(), data: None };
            self.set_tree(leaf);
            return (BltIt { key: key.to_string(), data: None }, true);
        }

        // Take the tree out to modify it
        let mut tree = self.take_tree().unwrap();
        let result = blt_setp_impl(&mut tree, key, key_bytes);
        self.set_tree(*tree);
        result
    }

    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        let (mut it, _) = self.blt_setp(key);
        // Now update the data in the tree
        if let Some(tree) = self.get_tree_mut() {
            set_data_at(tree, key, Some(data));
        }
        // Re-fetch to get updated data
        if let Some(got) = self.blt_get(key) {
            got
        } else {
            it.data = None;
            it
        }
    }

    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_, is_new) = self.blt_setp(key);
        if is_new {
            if let Some(tree) = self.get_tree_mut() {
                set_data_at(tree, key, Some(data));
            }
            0
        } else {
            1
        }
    }

    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.empty != 0 { return 0; }
        let mut tree = self.take_tree().unwrap();
        let result = blt_delete_impl(&mut tree, key);
        if result == -1 {
            // Tree is now empty
            self.empty = 1;
            self.root = Box::new(BltNode::Leaf(BltIt { key: String::new(), data: None }));
            return 1;
        }
        self.set_tree(*tree);
        result
    }

    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        let tree = match self.get_tree() {
            Some(t) => t,
            None => return 1,
        };
        let key_bytes = prefix.as_bytes();
        let keylen = key_bytes.len();

        let mut p = tree;
        let mut top = p;
        loop {
            match p {
                INode::Leaf { .. } => break,
                INode::Internal { byte, mask, left, right } => {
                    if (*byte as usize) >= keylen {
                        p = left;
                    } else {
                        p = if follow(*byte, *mask, key_bytes) { right } else { left };
                        top = p;
                    }
                }
            }
        }

        // Check prefix matches
        if let INode::Leaf { key, .. } = p {
            if !key.starts_with(prefix) {
                return 1;
            }
        }

        traverse_allprefixed(top, &mut fun)
    }

    pub fn blt_forall<F: FnMut(&BltIt)>(&self, mut fun: F) {
        let _ = self.blt_allprefixed("", |it| {
            fun(it);
            1
        });
    }

    pub fn blt_first(&self) -> Option<BltIt> {
        let tree = self.get_tree()?;
        Some(inode_to_bltit(firstlast(tree, false)))
    }

    pub fn blt_last(&self) -> Option<BltIt> {
        let tree = self.get_tree()?;
        Some(inode_to_bltit(firstlast(tree, true)))
    }

    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let tree = self.get_tree()?;
        let key_bytes = it.key.as_bytes();
        let mut p = tree;
        let mut other: Option<&INode> = None;
        loop {
            match p {
                INode::Leaf { .. } => break,
                INode::Internal { byte, mask, left, right } => {
                    if !follow(*byte, *mask, key_bytes) {
                        other = Some(right);
                        p = left;
                    } else {
                        p = right;
                    }
                }
            }
        }
        other.map(|o| inode_to_bltit(firstlast(o, false)))
    }

    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let tree = self.get_tree()?;
        let key_bytes = it.key.as_bytes();
        let mut p = tree;
        let mut other: Option<&INode> = None;
        loop {
            match p {
                INode::Leaf { .. } => break,
                INode::Internal { byte, mask, left, right } => {
                    if follow(*byte, *mask, key_bytes) {
                        other = Some(left);
                        p = right;
                    } else {
                        p = left;
                    }
                }
            }
        }
        other.map(|o| inode_to_bltit(firstlast(o, true)))
    }

    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.blt_ceilfloor(key, false)
    }

    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.blt_ceilfloor(key, true)
    }

    fn blt_ceilfloor(&self, key: &str, way: bool) -> Option<BltIt> {
        let tree = self.get_tree()?;
        let key_bytes = key.as_bytes();
        let keylen = key_bytes.len();
        let p_leaf = confident_get(tree, key_bytes, keylen);
        let p_key = match p_leaf {
            INode::Leaf { key: k, .. } => k,
            _ => unreachable!(),
        };

        let pk_bytes = p_key.as_bytes();
        // Compare keys byte by byte
        let mut i = 0;
        loop {
            let c = if i < key_bytes.len() { key_bytes[i] } else { 0u8 };
            let pc = if i < pk_bytes.len() { pk_bytes[i] } else { 0u8 };
            let x = c ^ pc;
            if x != 0 {
                let byte_pos = i as u32;
                let mask_val = to_mask(x);
                // Walk down tree
                let mut p = tree;
                let mut other: Option<&INode> = None;
                loop {
                    match p {
                        INode::Leaf { .. } => break,
                        INode::Internal { byte, mask, left, right } => {
                            if ((byte_pos << 8) + *mask as u32) < ((*byte << 8) + mask_val as u32) {
                                // Our crit bit is higher, stop
                                break;
                            }
                            let dir = follow(*byte, *mask, key_bytes);
                            if dir == way {
                                other = Some(if way { left } else { right });
                            }
                            p = if dir { right } else { left };
                        }
                    }
                }
                let ndir = key_bytes.get(i).map_or(false, |&b| b & mask_val != 0);
                if ndir == way {
                    other = Some(p);
                }
                return other.map(|o| inode_to_bltit(firstlast(o, way)));
            }
            if c == 0 {
                return Some(inode_to_bltit(p_leaf));
            }
            i += 1;
        }
    }

    pub fn blt_overhead(&self) -> usize {
        let base = std::mem::size_of::<Blt>();
        if self.empty != 0 { return base; }
        match self.get_tree() {
            Some(tree) => base + inode_overhead(tree),
            None => base,
        }
    }

    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }

    pub fn blt_size(&self) -> i32 {
        let mut r = 0i32;
        self.blt_forall(|_| r += 1);
        r
    }
}

fn inode_overhead(node: &INode) -> usize {
    match node {
        INode::Internal { left, right, .. } => {
            2 * std::mem::size_of::<InternalNode>() + inode_overhead(left) + inode_overhead(right)
        }
        INode::Leaf { .. } => 0,
    }
}

fn traverse_allprefixed<F: FnMut(&BltIt) -> i32>(node: &INode, fun: &mut F) -> i32 {
    match node {
        INode::Internal { left, right, .. } => {
            let status = traverse_allprefixed(left, fun);
            if status != 1 { return status; }
            traverse_allprefixed(right, fun)
        }
        INode::Leaf { key, data } => {
            let it = BltIt { key: key.clone(), data: clone_data(data) };
            fun(&it)
        }
    }
}

fn set_data_at(node: &mut INode, key: &str, data: Option<Box<dyn Any>>) {
    match node {
        INode::Leaf { key: k, data: d } if k == key => {
            *d = data;
        }
        INode::Internal { byte, mask, left, right } => {
            let key_bytes = key.as_bytes();
            if follow(*byte, *mask, key_bytes) {
                set_data_at(right, key, data);
            } else {
                set_data_at(left, key, data);
            }
        }
        _ => {}
    }
}

fn blt_setp_impl(tree: &mut Box<INode>, key: &str, key_bytes: &[u8]) -> (BltIt, bool) {
    let keylen = key_bytes.len();

    // First, walk to a leaf to find the comparison target
    let p_key = {
        let p = confident_get(tree, key_bytes, keylen);
        match p {
            INode::Leaf { key: k, .. } => k.clone(),
            _ => unreachable!(),
        }
    };

    let pk_bytes = p_key.as_bytes();
    // Compare keys byte by byte
    let mut i = 0;
    loop {
        let c = if i < key_bytes.len() { key_bytes[i] } else { 0u8 };
        let pc = if i < pk_bytes.len() { pk_bytes[i] } else { 0u8 };
        let x = c ^ pc;
        if x != 0 {
            let byte_pos = i as u32;
            let mask_val = to_mask(x);
            let goes_right = c & mask_val != 0;

            // Insert new node
            insert_into_tree(tree, key, byte_pos, mask_val, goes_right);
            return (BltIt { key: key.to_string(), data: None }, true);
        }
        if c == 0 {
            // Key already exists
            let data = get_data_at(tree, key);
            return (BltIt { key: key.to_string(), data: clone_data(&data) }, false);
        }
        i += 1;
    }
}

fn get_data_at(node: &INode, key: &str) -> Option<Box<dyn Any>> {
    match node {
        INode::Leaf { key: k, data } if k == key => clone_data(data),
        INode::Internal { byte, mask, left, right } => {
            let key_bytes = key.as_bytes();
            if follow(*byte, *mask, key_bytes) {
                get_data_at(right, key)
            } else {
                get_data_at(left, key)
            }
        }
        _ => None,
    }
}

fn insert_into_tree(tree: &mut Box<INode>, key: &str, byte_pos: u32, mask_val: u8, goes_right: bool) {
    let key_bytes = key.as_bytes();
    // Find the insertion point: walk down until we find a node whose crit bit is higher than ours
    let node = &mut **tree;
    match node {
        INode::Internal { byte, mask, left, right } => {
            if (byte_pos << 8) + *mask as u32 >= (*byte << 8) + mask_val as u32 {
                // Continue walking
                if follow(*byte, *mask, key_bytes) {
                    insert_into_tree(right, key, byte_pos, mask_val, goes_right);
                } else {
                    insert_into_tree(left, key, byte_pos, mask_val, goes_right);
                }
                return;
            }
        }
        _ => {}
    }

    // Insert here: replace current node with new internal node
    let old_node = std::mem::replace(&mut **tree, INode::Leaf { key: String::new(), data: None });
    let new_leaf = INode::Leaf { key: key.to_string(), data: None };
    if goes_right {
        **tree = INode::Internal {
            byte: byte_pos,
            mask: mask_val,
            left: Box::new(old_node),
            right: Box::new(new_leaf),
        };
    } else {
        **tree = INode::Internal {
            byte: byte_pos,
            mask: mask_val,
            left: Box::new(new_leaf),
            right: Box::new(old_node),
        };
    }
}

fn blt_delete_impl(tree: &mut Box<INode>, key: &str) -> i32 {
    let key_bytes = key.as_bytes();
    let keylen = key_bytes.len();

    match &**tree {
        INode::Leaf { key: k, .. } => {
            if k == key { return -1; } // signal: tree is now empty
            return 0;
        }
        _ => {}
    }

    delete_recursive(tree, key, key_bytes, keylen)
}

fn delete_recursive(node: &mut Box<INode>, key: &str, key_bytes: &[u8], keylen: usize) -> i32 {
    match &mut **node {
        INode::Internal { byte, mask, left, right } => {
            if (*byte as usize) > keylen { return 0; }
            let go_right = follow(*byte, *mask, key_bytes);
            let child = if go_right { &mut *right } else { &mut *left };

            match &**child {
                INode::Leaf { key: k, .. } => {
                    if k != key { return 0; }
                    // Remove this leaf: replace parent with sibling
                    let sibling = if go_right {
                        std::mem::replace(&mut **left, INode::Leaf { key: String::new(), data: None })
                    } else {
                        std::mem::replace(&mut **right, INode::Leaf { key: String::new(), data: None })
                    };
                    **node = sibling;
                    return 1;
                }
                INode::Internal { .. } => {
                    let child_box = if go_right { right } else { left };
                    delete_recursive(child_box, key, key_bytes, keylen)
                }
            }
        }
        INode::Leaf { .. } => 0,
    }
}
