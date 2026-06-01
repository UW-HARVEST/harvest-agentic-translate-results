use std::any::Any;

/// The BLT tree structure.
#[derive(Debug)]
pub struct Blt {
    /// The root node.
    pub root: Box<BltNode>,
    /// Indicates whether the tree is empty.
    pub empty: i32,
}
/// A node in the BLT tree.
#[derive(Debug)]
pub enum BltNode {
    /// An internal node.
    Internal(InternalNode),
    /// A leaf node (external node).
    Leaf(BltIt),
}
/// Represents an internal node in the BLT tree.
#[derive(Debug)]
pub struct InternalNode {
    /// Byte number of difference (32 bits).
    pub byte: u32,
    /// Mask byte (8 bits).
    pub mask: u8,
    /// Padding (23 bits stored in a u32).
    pub padding: u32,
    /// The child node.
    pub kid: Box<BltNode>,
}
/// Represents a leaf node in the BLT tree.
#[derive(Debug)]
pub struct BltIt {
    /// The key associated with the leaf.
    pub key: String,
    /// Associated data.
    pub data: Option<Box<dyn Any>>,
}

// -----------------------------------------------------------------------------
// Internal binary tree representation, stored as a `Box<dyn Any>` inside the
// root leaf's `data` field.  The public BltNode/InternalNode types only allow
// a single `kid` per internal node, so we cannot directly represent a binary
// critbit tree with them; instead we side-channel a real tree here.
// -----------------------------------------------------------------------------

enum NodeI {
    Internal {
        byte: usize,
        mask: u8,
        left: Box<NodeI>,
        right: Box<NodeI>,
    },
    Leaf {
        key: Vec<u8>,
        data: Option<Box<dyn Any>>,
    },
}

struct BltState {
    root: Option<NodeI>,
}

impl BltState {
    fn new() -> Self {
        Self { root: None }
    }
}

fn key_byte(key: &[u8], i: usize) -> u8 {
    if i < key.len() {
        key[i]
    } else {
        0
    }
}

fn to_mask(mut x: u8) -> u8 {
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x & !(x >> 1)
}

fn follow<'a>(byte: usize, mask: u8, key: &[u8], left: &'a Box<NodeI>, right: &'a Box<NodeI>) -> &'a NodeI {
    if key_byte(key, byte) & mask != 0 {
        right.as_ref()
    } else {
        left.as_ref()
    }
}

// Walk down the tree as if the key is there. Returns the leaf reached.
fn confident_get<'a>(root: &'a NodeI, key: &[u8]) -> &'a NodeI {
    let mut p = root;
    let keylen = key.len();
    loop {
        match p {
            NodeI::Internal {
                byte, mask, left, right,
            } => {
                if *byte < keylen && (key[*byte] & *mask) != 0 {
                    p = right;
                } else {
                    p = left;
                }
            }
            NodeI::Leaf { .. } => return p,
        }
    }
}

// Returns the smallest (dir=0) or largest (dir=1) leaf reachable from `node`.
fn firstlast(node: &NodeI, dir: usize) -> &NodeI {
    let mut p = node;
    loop {
        match p {
            NodeI::Internal { left, right, .. } => {
                p = if dir == 0 { left } else { right };
            }
            NodeI::Leaf { .. } => return p,
        }
    }
}

fn leaf_to_bltit_no_data(node: &NodeI) -> BltIt {
    if let NodeI::Leaf { key, .. } = node {
        BltIt {
            key: String::from_utf8_lossy(key).into_owned(),
            data: None,
        }
    } else {
        unreachable!()
    }
}

impl Blt {
    fn state(&self) -> &BltState {
        match self.root.as_ref() {
            BltNode::Leaf(it) => it
                .data
                .as_ref()
                .and_then(|d| d.downcast_ref::<BltState>())
                .expect("internal state missing"),
            _ => panic!("root must be marker leaf"),
        }
    }

    fn state_mut(&mut self) -> &mut BltState {
        match self.root.as_mut() {
            BltNode::Leaf(it) => it
                .data
                .as_mut()
                .and_then(|d| d.downcast_mut::<BltState>())
                .expect("internal state missing"),
            _ => panic!("root must be marker leaf"),
        }
    }

    /// Creates a new BLT tree.
    pub fn blt_new() -> Self {
        Self {
            root: Box::new(BltNode::Leaf(BltIt {
                key: String::new(),
                data: Some(Box::new(BltState::new()) as Box<dyn Any>),
            })),
            empty: 1,
        }
    }

    /// Clears (destroys) the tree.
    pub fn blt_clear(&mut self) {
        self.state_mut().root = None;
        self.empty = 1;
    }

    /// Retrieves the leaf node at the given key.
    pub fn blt_get(&self, key: &str) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        let kbytes = key.as_bytes();
        let keylen = kbytes.len();

        let mut p = root;
        loop {
            match p {
                NodeI::Internal {
                    byte, mask, left, right,
                } => {
                    if *byte > keylen {
                        return None;
                    }
                    p = follow(*byte, *mask, kbytes, left, right);
                }
                NodeI::Leaf { key: lk, .. } => {
                    if lk == kbytes {
                        return Some(leaf_to_bltit_no_data(p));
                    } else {
                        return None;
                    }
                }
            }
        }
    }

    /// Creates or retrieves the leaf node at the given key.
    pub fn blt_set(&mut self, key: &str) -> BltIt {
        let (it, _) = self.blt_setp(key);
        it
    }

    /// Creates or retrieves the leaf node at the given key and returns a tuple (leaf, is_new).
    pub fn blt_setp(&mut self, key: &str) -> (BltIt, bool) {
        let kbytes = key.as_bytes().to_vec();

        // Empty tree case.
        if self.state().root.is_none() {
            self.state_mut().root = Some(NodeI::Leaf {
                key: kbytes.clone(),
                data: None,
            });
            self.empty = 0;
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                true,
            );
        }

        // Find a candidate leaf (confident_get).
        let (cand_key, found_match);
        {
            let root = self.state().root.as_ref().unwrap();
            let leaf = confident_get(root, &kbytes);
            if let NodeI::Leaf { key: lk, .. } = leaf {
                cand_key = lk.clone();
                found_match = lk == &kbytes;
            } else {
                unreachable!()
            }
        }
        if found_match {
            return (
                BltIt {
                    key: key.to_string(),
                    data: None,
                },
                false,
            );
        }

        // Compare keys to find the differing byte.
        let mut i: usize = 0;
        let (byte_idx, x_mask, dir_bit);
        loop {
            let a = key_byte(&kbytes, i);
            let b = key_byte(&cand_key, i);
            let xor = a ^ b;
            if xor != 0 {
                let m = to_mask(xor);
                byte_idx = i;
                x_mask = m;
                dir_bit = (a & m) != 0;
                break;
            }
            // The keys are equal at index i.  If we reached the end, they
            // are identical, but that case is handled above via `found_match`.
            if a == 0 && b == 0 {
                unreachable!();
            }
            i += 1;
        }

        // Walk down to the insertion point and rewrite.
        Self::insert_at(
            self.state_mut().root.as_mut().unwrap(),
            byte_idx,
            x_mask,
            dir_bit,
            kbytes.clone(),
        );

        (
            BltIt {
                key: key.to_string(),
                data: None,
            },
            true,
        )
    }

    fn insert_at(
        root: &mut NodeI,
        byte_idx: usize,
        x_mask: u8,
        dir_bit: bool,
        new_key: Vec<u8>,
    ) {
        // We descend while node's critbit is "above" ours (closer to root):
        //   node->byte < byte_idx, OR (same byte AND node->mask >= x_mask).
        // We break/split when node's critbit is at or below ours:
        //   node->byte > byte_idx, OR (same byte AND node->mask < x_mask).

        fn need_split(node: &NodeI, byte_idx: usize, x_mask: u8) -> bool {
            match node {
                NodeI::Internal { byte, mask, .. } => {
                    *byte > byte_idx || (*byte == byte_idx && *mask < x_mask)
                }
                NodeI::Leaf { .. } => true,
            }
        }

        if need_split(root, byte_idx, x_mask) {
            let leaf = NodeI::Leaf {
                key: new_key,
                data: None,
            };
            let old = std::mem::replace(
                root,
                NodeI::Leaf {
                    key: Vec::new(),
                    data: None,
                },
            );
            let (left, right) = if dir_bit {
                (Box::new(old), Box::new(leaf))
            } else {
                (Box::new(leaf), Box::new(old))
            };
            *root = NodeI::Internal {
                byte: byte_idx,
                mask: x_mask,
                left,
                right,
            };
            return;
        }

        // Descend.
        match root {
            NodeI::Internal {
                byte, mask, left, right,
            } => {
                let go_right = (key_byte(&new_key, *byte) & *mask) != 0;
                let _ = byte;
                let _ = mask;
                let next = if go_right { right.as_mut() } else { left.as_mut() };
                Self::insert_at(next, byte_idx, x_mask, dir_bit, new_key);
            }
            NodeI::Leaf { .. } => unreachable!(),
        }
    }

    /// Inserts the given key/data pair and returns the corresponding leaf.
    pub fn blt_put(&mut self, key: &str, data: Box<dyn Any>) -> BltIt {
        // Ensure node exists.
        self.blt_setp(key);
        // Now find it and set data.
        Self::set_data(self.state_mut().root.as_mut().unwrap(), key.as_bytes(), Some(data));
        BltIt {
            key: key.to_string(),
            data: None,
        }
    }

    fn set_data(node: &mut NodeI, key: &[u8], data: Option<Box<dyn Any>>) {
        let mut p = node;
        loop {
            match p {
                NodeI::Internal {
                    byte, mask, left, right,
                } => {
                    let go_right = (key_byte(key, *byte) & *mask) != 0;
                    p = if go_right { right.as_mut() } else { left.as_mut() };
                }
                NodeI::Leaf { data: d, .. } => {
                    *d = data;
                    return;
                }
            }
        }
    }

    /// Inserts the key/data pair only if the key is absent.
    /// Returns 0 on success or 1 if the key is already present.
    pub fn blt_put_if_absent(&mut self, key: &str, data: Box<dyn Any>) -> i32 {
        let (_it, is_new) = self.blt_setp(key);
        if is_new {
            Self::set_data(self.state_mut().root.as_mut().unwrap(), key.as_bytes(), Some(data));
            0
        } else {
            1
        }
    }

    /// Deletes the given key from the tree.
    /// Returns 1 if a key was deleted, 0 otherwise.
    pub fn blt_delete(&mut self, key: &str) -> i32 {
        if self.state().root.is_none() {
            return 0;
        }
        let kbytes = key.as_bytes().to_vec();
        let keylen = kbytes.len();
        let st = self.state_mut();

        // Special case: root is a leaf.
        let root_is_leaf_match = match st.root.as_ref().unwrap() {
            NodeI::Leaf { key: lk, .. } => lk == &kbytes,
            _ => false,
        };
        if root_is_leaf_match {
            st.root = None;
            self.empty = 1;
            return 1;
        }
        if let NodeI::Leaf { .. } = st.root.as_ref().unwrap() {
            return 0;
        }

        // Recursively descend; when we find the leaf, replace its parent with
        // the sibling.
        fn rec(node: &mut NodeI, key: &[u8], keylen: usize) -> i32 {
            let (byte, mask, go_right);
            match node {
                NodeI::Internal {
                    byte: b, mask: m, ..
                } => {
                    if *b > keylen {
                        return 0;
                    }
                    byte = *b;
                    mask = *m;
                    go_right = (key_byte(key, byte) & mask) != 0;
                }
                _ => return 0,
            }

            // Check whether the chosen child is a Leaf — if so, decide.
            let child_is_target = {
                let next: &NodeI = match node {
                    NodeI::Internal { left, right, .. } => {
                        if go_right { right } else { left }
                    }
                    _ => unreachable!(),
                };
                match next {
                    NodeI::Leaf { key: lk, .. } => Some(lk == key),
                    NodeI::Internal { .. } => None,
                }
            };

            match child_is_target {
                Some(true) => {
                    // Replace `node` with the sibling.
                    let owned = std::mem::replace(
                        node,
                        NodeI::Leaf {
                            key: Vec::new(),
                            data: None,
                        },
                    );
                    if let NodeI::Internal { left, right, .. } = owned {
                        let sibling = if go_right { *left } else { *right };
                        *node = sibling;
                    } else {
                        unreachable!();
                    }
                    let _ = byte;
                    let _ = mask;
                    1
                }
                Some(false) => 0,
                None => {
                    // Descend.
                    let next: &mut NodeI = match node {
                        NodeI::Internal { left, right, .. } => {
                            if go_right { right.as_mut() } else { left.as_mut() }
                        }
                        _ => unreachable!(),
                    };
                    rec(next, key, keylen)
                }
            }
        }

        let r = rec(st.root.as_mut().unwrap(), &kbytes, keylen);
        if st.root.is_none() {
            self.empty = 1;
        }
        r
    }

    /// Iterates over all leaves with keys having the given prefix.
    /// The closure should return an i32; if it returns 0, iteration stops.
    pub fn blt_allprefixed<F: FnMut(&BltIt) -> i32>(&self, prefix: &str, mut fun: F) -> i32 {
        if self.state().root.is_none() {
            return 1;
        }
        let pbytes = prefix.as_bytes();
        let plen = pbytes.len();
        let root = self.state().root.as_ref().unwrap();

        // Walk down the tree: for nodes whose byte index is < plen, follow the
        // appropriate child according to the prefix bytes; for byte indices >=
        // plen, go left and remember the highest such ancestor as `top`.
        let mut p = root;
        let mut top = root;
        loop {
            match p {
                NodeI::Internal {
                    byte, mask, left, right,
                } => {
                    if *byte >= plen {
                        p = left;
                    } else {
                        p = if (pbytes[*byte] & *mask) != 0 { right } else { left };
                        top = p;
                    }
                }
                NodeI::Leaf { .. } => break,
            }
        }

        // Verify the leaf reached actually has the prefix.
        if let NodeI::Leaf { key, .. } = p {
            if key.len() < plen || &key[..plen] != pbytes {
                return 1;
            }
        }

        fn traverse<F: FnMut(&BltIt) -> i32>(node: &NodeI, fun: &mut F) -> i32 {
            match node {
                NodeI::Internal { left, right, .. } => {
                    let s = traverse(left, fun);
                    if s != 1 {
                        return s;
                    }
                    let s = traverse(right, fun);
                    if s != 1 {
                        return s;
                    }
                    1
                }
                NodeI::Leaf { .. } => {
                    let it = leaf_to_bltit_no_data(node);
                    fun(&it)
                }
            }
        }

        traverse(top, &mut fun)
    }

    /// Iterates through all leaves in order and calls the provided closure.
    pub fn blt_forall<F: FnMut(&BltIt)>(&self, mut fun: F) {
        let _ = self.blt_allprefixed("", |it| {
            fun(it);
            1 // always continue iteration
        });
    }

    /// Returns the leaf with the smallest key.
    pub fn blt_first(&self) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        Some(leaf_to_bltit_no_data(firstlast(root, 0)))
    }

    /// Returns the leaf with the largest key.
    pub fn blt_last(&self) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        Some(leaf_to_bltit_no_data(firstlast(root, 1)))
    }

    /// Returns the next leaf (in order) after the given one.
    pub fn blt_next(&self, it: &BltIt) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        let kbytes = it.key.as_bytes();
        let mut p = root;
        let mut other: Option<&NodeI> = None;
        loop {
            match p {
                NodeI::Internal {
                    byte, mask, left, right,
                } => {
                    if (key_byte(kbytes, *byte) & *mask) == 0 {
                        other = Some(right);
                        p = left;
                    } else {
                        p = right;
                    }
                }
                NodeI::Leaf { .. } => break,
            }
        }
        other.map(|n| leaf_to_bltit_no_data(firstlast(n, 0)))
    }

    /// Returns the previous leaf (in order) before the given one.
    pub fn blt_prev(&self, it: &BltIt) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        let kbytes = it.key.as_bytes();
        let mut p = root;
        let mut other: Option<&NodeI> = None;
        loop {
            match p {
                NodeI::Internal {
                    byte, mask, left, right,
                } => {
                    if (key_byte(kbytes, *byte) & *mask) != 0 {
                        other = Some(left);
                        p = right;
                    } else {
                        p = left;
                    }
                }
                NodeI::Leaf { .. } => break,
            }
        }
        other.map(|n| leaf_to_bltit_no_data(firstlast(n, 1)))
    }

    fn ceilfloor(&self, key: &str, way: usize) -> Option<BltIt> {
        let st = self.state();
        let root = st.root.as_ref()?;
        let kbytes = key.as_bytes();

        let leaf = confident_get(root, kbytes);
        let lk = if let NodeI::Leaf { key: k, .. } = leaf {
            k.clone()
        } else {
            unreachable!()
        };

        // Compare keys.
        let max_len = kbytes.len().max(lk.len());
        for i in 0..=max_len {
            let a = key_byte(kbytes, i);
            let b = key_byte(&lk, i);
            let x = a ^ b;
            if x != 0 {
                let xm = to_mask(x);
                let byte = i;
                // Walk down the tree.
                let mut p = root;
                let mut other: Option<&NodeI> = None;
                loop {
                    match p {
                        NodeI::Internal {
                            byte: b2, mask: m2, left, right,
                        } => {
                            // C: if ((byte << 8) + p->mask < (p->byte << 8) + x) break;
                            // i.e. if our (byte, mask) is "above" current node, stop.
                            let our_above = byte < *b2 || (byte == *b2 && xm > *m2);
                            if our_above {
                                break;
                            }
                            let dir = if (*m2 & key_byte(kbytes, *b2)) != 0 { 1 } else { 0 };
                            if dir == way {
                                // The "other" child is the one we don't take, but only
                                // when our descent direction equals `way`.
                                // C: other = q + 1 - way, where way is 0 (ceil) or 1 (floor).
                                other = Some(if way == 0 { right.as_ref() } else { left.as_ref() });
                            }
                            p = if dir == 1 { right } else { left };
                        }
                        NodeI::Leaf { .. } => break,
                    }
                }
                let ndir = if (xm & key_byte(kbytes, byte)) != 0 { 1 } else { 0 };
                if ndir == way {
                    other = Some(p);
                }
                return other.map(|n| leaf_to_bltit_no_data(firstlast(n, way)));
            }
            if a == 0 {
                // Found exact match.
                return Some(BltIt {
                    key: String::from_utf8_lossy(&lk).into_owned(),
                    data: None,
                });
            }
        }
        // Shouldn't reach here.
        None
    }

    /// Returns the leaf with the smallest key ≥ the given key.
    pub fn blt_ceil(&self, key: &str) -> Option<BltIt> {
        self.ceilfloor(key, 0)
    }

    /// Returns the leaf with the largest key ≤ the given key.
    pub fn blt_floor(&self, key: &str) -> Option<BltIt> {
        self.ceilfloor(key, 1)
    }

    /// Returns the number of bytes used by the tree (excluding key storage).
    pub fn blt_overhead(&self) -> usize {
        // Match the C calculation: sizeof(BLT)=24, plus 2*sizeof(blt_node_s)=32
        // per pair of internal/leaf nodes allocated in `blt_setp`.
        const SIZE_OF_BLT: usize = 24;
        const SIZE_OF_NODE: usize = 16;
        let mut n = SIZE_OF_BLT;
        let st = self.state();
        if let Some(root) = st.root.as_ref() {
            fn count(node: &NodeI, n: &mut usize) {
                if let NodeI::Internal { left, right, .. } = node {
                    *n += 2 * SIZE_OF_NODE;
                    count(left, n);
                    count(right, n);
                }
            }
            count(root, &mut n);
        }
        n
    }

    /// Returns true if the tree is empty.
    pub fn blt_empty(&self) -> bool {
        self.empty != 0
    }

    /// Returns the number of keys in the tree.
    pub fn blt_size(&self) -> i32 {
        let mut r = 0;
        self.blt_forall(|_| r += 1);
        r
    }
}
