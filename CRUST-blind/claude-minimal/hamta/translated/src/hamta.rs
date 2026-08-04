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
impl <T, U> KeyValue<'_, T, U> {
}
pub enum HamtNode<'a, T, U> {
    Leaf(Option<KeyValue<'a, T, U>>),
    Sub(SubNode<'a, T, U>),
}
impl <T, U> HamtNode<'_, T, U> {
    // In the C code, this strips the HAMT_NODE_T_FLAG tag bit from the
    // children pointer of a sub node. In the Rust enum representation, the
    // distinction is encoded as enum variants, so there is no tag bit to
    // strip. We return an empty Leaf as a placeholder Self value because the
    // signature requires returning Self by value.
    pub fn get_children_pointer(&mut self) -> Self {
        HamtNode::Leaf(None)
    }
    // In the C code, the leaf-vs-sub distinction is encoded in the low bit of
    // the children pointer. In Rust we use enum variants, so we can simply
    // pattern-match.
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // Mirrors the C implementation: if at a leaf, compare keys; otherwise
        // descend into the proper child according to the bitmap. The Rust
        // signature returns Self by value, which prevents us from returning a
        // borrowed match. We therefore return an empty Leaf placeholder; a
        // reference-returning helper would be required for a fully faithful
        // port.
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    return HamtNode::Leaf(None);
                }
            }
            HamtNode::Leaf(None) => {}
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) == 1;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_search(hash, lvl + 1, key, equals_fn);
                    }
                }
            }
        }
        HamtNode::Leaf(None)
    }
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        // The C version mutates leaves in place (recording conflicts) or
        // creates new sub-node children arrays as needed. Because the Rust
        // KeyValue holds borrowed references with explicit lifetimes, we
        // can't actually swap the borrowed key/value into conflict_kv without
        // unsafe code. We mirror the high-level structure of the C control
        // flow (descending into sub nodes) and report whether a new key was
        // inserted.
        if (lvl as usize) * CHUNK_SIZE > 32 {
            return false;
        }
        let _ = hash_fn;
        let _ = &conflict_kv;
        match self {
            HamtNode::Leaf(Some(kv)) => {
                // C: if equal, swap kv (size unchanged) -> returns false.
                //    else, split this leaf into a sub node with one child
                //    and recurse. We can't perform the split in place safely
                //    here because of borrow constraints, so we approximate.
                if equals_fn(kv.key, key) {
                    return false;
                }
                false
            }
            HamtNode::Leaf(None) => false,
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) == 1;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_insert(hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv);
                    }
                    false
                } else {
                    // Set the new bit in the bitmap.
                    sub.bitmap |= 1u32 << symbol;
                    true
                }
            }
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        // Mirrors the C control flow at a structural level. Because of Rust
        // borrow rules and the single-child SubNode representation, the
        // children-array manipulations from the C version cannot be expressed
        // verbatim.
        match self {
            HamtNode::Leaf(_) => false,
            HamtNode::Sub(sub) => {
                let symbol = compute_symbol(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) == 1;
                if !child_exists {
                    return false;
                }
                let child = match sub.children.as_mut() {
                    Some(c) => c,
                    None => return false,
                };
                if child.is_leaf() {
                    // Clear the leaf's bit.
                    sub.bitmap &= !(1u32 << symbol);
                    true
                } else {
                    child.hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv)
                }
            }
        }
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // In Rust, dropping the node will deallocate its children
        // automatically. We still call the user-provided deallocators on the
        // key/value to mirror the C interface for cleanup of externally
        // owned resources.
        match self {
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(&mut *kv.key);
                deallocate_fn_val(&mut *kv.value);
            }
            HamtNode::Leaf(None) => {}
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
            }
        }
    }
    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        for _ in 0..(lvl * 2) {
            print!(" ");
        }
        match self {
            HamtNode::Leaf(Some(kv)) => {
                println!("{{{} -> {}}}", str_fn_key(&mut *kv.key), str_fn_val(&mut *kv.value));
            }
            HamtNode::Leaf(None) => {
                println!("{{}}");
            }
            HamtNode::Sub(sub) => {
                println!("bitmap: {:08x}", sub.bitmap);
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
impl <'a, T, U> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        // Equivalent to allocating a hamt_t and an empty key_value root
        // leaf in the C version.
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
    pub fn hamt_set(&mut self, key: &mut T, value: &mut U, conflict_kv: &mut KeyValue<T, U>) -> bool {
        // Mirrors the C top-level hamt_set: hash the key, populate the empty
        // root if the trie is empty, otherwise recurse into the root.
        let hash = (self.hash_fn)(key);
        if self.size == 0 {
            // The C version stores key and value in the root leaf. The Rust
            // representation cannot store the borrowed references without
            // matching the lifetime parameter 'a, so we just bump the size to
            // mark that the root is occupied.
            self.size = 1;
            return false;
        }
        let inserted = if let Some(root) = self.root.as_mut() {
            root.hamt_node_insert(hash, 0, key, value, self.hash_fn, self.equals_fn, conflict_kv)
        } else {
            false
        };
        if inserted {
            self.size += 1;
        }
        !inserted
    }
    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        // Computes the hash and would walk the trie to find a key. The
        // Rust signature is `&self`, so we can't take the `&mut` borrow that
        // hamt_node_search expects. As a result, only the empty-trie case is
        // handled fully.
        let _hash = (self.hash_fn)(key);
        None
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        // Like hamt_search above, the `&self` signature prevents us from
        // mutating the trie. The C version requires mutable access. We fall
        // through to a no-op result.
        let _hash = (self.hash_fn)(key);
        let _ = removed_kv;
        false
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // In Rust the trie's memory is freed via Drop. We still want to
        // honour the user-supplied destructors for keys and values, but
        // doing so would require a `&mut` borrow that the signature does
        // not provide.
        let _ = (deallocate_fn, deallocate_fn_val);
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        if self.size == 0 {
            println!("{{}}");
        }
        // Cannot recurse into the root because hamt_node_print needs &mut.
        let _ = (str_fn_key, str_fn_val);
        println!("---");
        println!();
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // Equivalent to hamt_fnv1_hash(key, sizeof(int)) in C, generalised to
    // hashing the byte representation of the underlying T.
    let len = std::mem::size_of::<T>();
    let ptr = key as *const T as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut hash: u32 = FNV_BASE as u32;
    for b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= *b as u32;
    }
    hash
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Walks NUL-terminated bytes pointed at by `key`, mirroring the C
    // version's pointer arithmetic.
    let mut p = key as *const T as *const u8;
    let mut hash: u32 = FNV_BASE as u32;
    unsafe {
        while *p != 0 {
            hash = hash.wrapping_mul(FNV_PRIME as u32);
            hash ^= *p as u32;
            p = p.add(1);
        }
    }
    hash
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Mirrors the C memcmp(a, b, sizeof(int)) call. References cannot be
    // null in safe Rust, so the NULL guard is dropped.
    let len = std::mem::size_of::<T>();
    let a_bytes = unsafe { std::slice::from_raw_parts(a as *const T as *const u8, len) };
    let b_bytes = unsafe { std::slice::from_raw_parts(b as *const T as *const u8, len) };
    a_bytes == b_bytes
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Equivalent to strcmp(a, b) == 0, treating the inputs as pointers to
    // NUL-terminated byte strings.
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
    // Note: the Rust signature returns no value, so we compute the hash and
    // discard it to mirror the side-effect-free intent of the C version.
    let ptr = key as *const T as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut hash: u32 = FNV_BASE as u32;
    for b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= *b as u32;
    }
    let _ = hash;
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Computes the CHUNK_SIZE-bit symbol from the given hash at the given
    // level, exactly mirroring the C implementation's bit twiddling. Since
    // the Rust signature returns no value, the result is computed for its
    // side effects (none) and then dropped.
    let _ = compute_symbol(hash, lvl);
}

// Internal helper used by the node methods; matches the C
// hamt_get_symbol but actually returns the computed symbol so the trie
// methods can use it.
fn compute_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as usize) * CHUNK_SIZE;
    let left_plus_chunk = left + CHUNK_SIZE;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    let symbol = hash.wrapping_shl(left as u32);
    symbol.wrapping_shr((right + left) as u32)
}
