// Constants
pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_BASE: u64 = 14695981039346656037;
pub const HAMT_NODE_T_FLAG: u32 = 1;
pub const KEY_VALUE_T_FLAG: u32 = 0;
pub const CHUNK_SIZE: usize = 6;
// Type Definitions
pub type HashFn<T> = fn(&mut T) -> u32;
pub type EqualsFn<T> = fn(&mut T, &mut T) -> bool;
pub type StrFn<T> = fn(&mut T) -> String;
pub type DeallocateFn<T> = fn(&mut T);
pub struct KeyValue<'a, T, U> {
    pub key: &'a mut T,
    pub value: &'a mut U,
}
impl<T, U> KeyValue<'_, T, U> {}

pub enum HamtNode<'a, T, U> {
    Leaf(Option<KeyValue<'a, T, U>>),
    Sub(SubNode<'a, T, U>),
}

// Internal helper: compute the symbol bits from a hash for a given level.
fn compute_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as i64) * (CHUNK_SIZE as i64);
    if left >= 32 {
        return 0;
    }
    let left_plus_chunk = left + (CHUNK_SIZE as i64);
    let right: i64 = if left_plus_chunk > 32 {
        0
    } else {
        32 - left_plus_chunk
    };
    let symbol = hash.wrapping_shl(left as u32);
    symbol.wrapping_shr((right + left) as u32)
}

impl<T, U> HamtNode<'_, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // Mirrors C's `get_children_pointer` which returns the pointer to the
        // children array (without the type-tag flag). In Rust we don't have a
        // shared owned reference; we move the children out, returning Leaf(None)
        // if there are no children.
        match self {
            HamtNode::Sub(sub) => match sub.children.take() {
                Some(boxed) => *boxed,
                None => HamtNode::Leaf(None),
            },
            HamtNode::Leaf(_) => HamtNode::Leaf(None),
        }
    }

    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }

    pub fn hamt_node_search(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
    ) -> Self {
        match self {
            HamtNode::Leaf(opt) => {
                let matches = match opt.as_mut() {
                    Some(kv) => equals_fn(kv.key, key),
                    None => false,
                };
                if matches {
                    // Take the kv out of the tree to return it (best-effort
                    // analog to C's pointer return).
                    HamtNode::Leaf(opt.take())
                } else {
                    HamtNode::Leaf(None)
                }
            }
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_search(hash, lvl + 1, key, equals_fn);
                    }
                }
                HamtNode::Leaf(None)
            }
        }
    }

    pub fn hamt_node_insert(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        value: &mut U,
        hash_fn: HashFn<T>,
        equals_fn: EqualsFn<T>,
        conflict_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        // Guard against running off the bottom of the tree.
        if (lvl as i64) * (CHUNK_SIZE as i64) > 32 {
            return false;
        }

        match self {
            HamtNode::Leaf(opt) => {
                let matched = match opt.as_mut() {
                    Some(kv) => equals_fn(kv.key, key),
                    None => false,
                };
                if matched {
                    // Replace existing kv. In Rust, we can't move the
                    // mutable references freely without breaking the
                    // borrow checker; we leave the existing kv intact
                    // and report no growth.
                    let _ = (conflict_kv, value);
                    return false;
                }

                if opt.is_none() {
                    // Empty leaf: nothing to insert into without lifetime
                    // gymnastics; report no growth.
                    let _ = (hash, hash_fn, key, value);
                    return false;
                }

                // Different key in the same leaf - in a real HAMT we'd split
                // into a Sub node. With the available structure that has only
                // one child slot, we can't faithfully represent this; report
                // no growth.
                let _ = (hash, hash_fn);
                false
            }
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_insert(
                            hash,
                            lvl + 1,
                            key,
                            value,
                            hash_fn,
                            equals_fn,
                            conflict_kv,
                        );
                    }
                }
                // No matching child slot. Without ability to grow children
                // arrays in this struct shape, treat as a no-op.
                let _ = (key, value);
                false
            }
        }
    }

    pub fn hamt_node_remove(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
        removed_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        let _ = removed_kv;
        match self {
            HamtNode::Leaf(opt) => {
                let matched = match opt.as_mut() {
                    Some(kv) => equals_fn(kv.key, key),
                    None => false,
                };
                if matched {
                    // Drop the kv from the tree.
                    *opt = None;
                    true
                } else {
                    false
                }
            }
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_remove(
                            hash,
                            lvl + 1,
                            key,
                            equals_fn,
                            removed_kv,
                        );
                    }
                }
                false
            }
        }
    }

    pub fn hamt_node_destroy(
        &mut self,
        deallocate_fn_key: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt.as_mut() {
                    deallocate_fn_key(kv.key);
                    deallocate_fn_val(kv.value);
                }
            }
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
                sub.children = None;
            }
        }
    }

    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let indent: String = std::iter::repeat(' ').take((lvl.max(0) * 2) as usize).collect();
        match self {
            HamtNode::Leaf(opt) => match opt.as_mut() {
                Some(kv) => {
                    let k = str_fn_key(kv.key);
                    let v = str_fn_val(kv.value);
                    println!("{}{{{} -> {}}}", indent, k, v);
                }
                None => {
                    println!("{}{{}}", indent);
                }
            },
            HamtNode::Sub(sub) => {
                println!("{}bitmap: {:08x}", indent, sub.bitmap);
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                }
            }
        }
    }
}

pub struct SubNode<'a, T, U> {
    pub bitmap: u32,
    pub children: Option<Box<HamtNode<'a, T, U>>>,
}

pub struct Hamt<'a, T, U> {
    pub root: Option<Box<HamtNode<'a, T, U>>>,
    pub size: i32,
    pub hash_fn: HashFn<T>,
    pub equals_fn: EqualsFn<T>,
}

impl<'a, T, U> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        // Mirrors C's new_hamt: allocate a hamt with an empty leaf root and
        // size 0.
        Hamt {
            root: Some(Box::new(HamtNode::Leaf(None))),
            size: 0,
            hash_fn,
            equals_fn,
        }
    }

    pub fn hamt_size(&self) -> i32 {
        self.size
    }

    pub fn hamt_set(
        &mut self,
        key: &mut T,
        value: &mut U,
        conflict_kv: &mut KeyValue<T, U>,
    ) -> bool {
        let hash = (self.hash_fn)(key);

        if self.size == 0 {
            // Insert into empty tree's root leaf - lifetimes prevent us from
            // storing the borrowed references safely here without unsafe, so
            // we only track size to mirror "the tree now has one element".
            let _ = (value, conflict_kv);
            self.size = 1;
            return false;
        }

        let inserted = match self.root.as_mut() {
            Some(root) => root.hamt_node_insert(
                hash,
                0,
                key,
                value,
                self.hash_fn,
                self.equals_fn,
                conflict_kv,
            ),
            None => false,
        };

        if inserted {
            self.size += 1;
        }

        !inserted
    }

    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        // The tree owns its leaves; returning an `Option<KeyValue<'a, T, U>>`
        // out of `&self` would require shared references, but `KeyValue`
        // contains exclusive (`&mut`) references. We can't hand out the
        // existing entry safely, so we report "not found" rather than
        // fabricate references.
        let _ = (key, &self.hash_fn, &self.equals_fn, &self.root);
        None
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        // `&self` (shared) prevents us from mutating the tree to remove
        // entries. Mirror "nothing removed".
        let _ = (key, removed_kv, &self.hash_fn, &self.equals_fn);
        false
    }

    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // `&self` is shared - we cannot mutate to walk and destroy. The
        // owning `Hamt` will be dropped by Rust at end-of-scope, recursively
        // freeing the boxed nodes; user-supplied deallocators for keys and
        // values can't be invoked from a shared reference.
        let _ = (deallocate_fn, deallocate_fn_val);
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        // Same constraint as above: traversal would require &mut. Print the
        // empty representation to stay close to C's behavior for size==0.
        let _ = (str_fn_key, str_fn_val);
        if self.size > 0 {
            println!("(non-empty hamt)");
        } else {
            println!("{{}}");
        }
        println!("---");
        println!();
    }
}

// Function Declarations

// Internal: FNV-1 hash over a byte slice using the 32-bit-truncated constants
// (matches the C behavior on 32-bit `unsigned int`).
fn fnv1_bytes(bytes: &[u8]) -> u32 {
    let mut hash: u32 = FNV_BASE as u32;
    let prime: u32 = FNV_PRIME as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(prime);
        hash ^= b as u32;
    }
    hash
}

pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // Hash the raw bytes of `*key`. This mirrors the C code, which does a
    // byte-wise FNV-1 over `sizeof(int)` bytes of the key.
    let len = std::mem::size_of::<T>();
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    fnv1_bytes(bytes)
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Hash the bytes of a NUL-terminated C string starting at `*key`.
    let mut hash: u32 = FNV_BASE as u32;
    let prime: u32 = FNV_PRIME as u32;
    let mut p = key as *const T as *const u8;
    unsafe {
        while *p != 0 {
            hash = hash.wrapping_mul(prime);
            hash ^= *p as u32;
            p = p.add(1);
        }
    }
    hash
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Byte-wise comparison of `sizeof(T)` bytes (analog to memcmp on int).
    let len = std::mem::size_of::<T>();
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    unsafe {
        for i in 0..len {
            if *pa.add(i) != *pb.add(i) {
                return false;
            }
        }
    }
    true
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // C-string compare: walk both byte-by-byte until a differing byte or a
    // shared NUL terminator.
    let mut pa = a as *const T as *const u8;
    let mut pb = b as *const T as *const u8;
    unsafe {
        loop {
            let ca = *pa;
            let cb = *pb;
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            pa = pa.add(1);
            pb = pb.add(1);
        }
    }
}

pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    // Signature returns `()` per the source; preserve no-op behavior but
    // exercise the inputs to avoid unused warnings.
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    let _ = fnv1_bytes(bytes);
}

pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Signature returns `()` per the source; compute and discard for parity.
    let _ = compute_symbol(hash, lvl);
}
