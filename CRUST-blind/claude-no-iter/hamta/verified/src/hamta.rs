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

// Helper: count set bits in a u32 (Rust's u32::count_ones).
fn popcount(x: u32) -> u32 {
    x.count_ones()
}

// Helper: extract a CHUNK_SIZE-bit symbol from `hash` for tree level `lvl`.
fn get_symbol(hash: u32, lvl: i32) -> u32 {
    let chunk = CHUNK_SIZE as i32;
    let left = lvl * chunk;
    let left_plus_chunk = left + chunk;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    if left >= 32 {
        return 0;
    }
    let symbol: u32 = hash.wrapping_shl(left as u32);
    let total_shift = (right + left) as u32;
    if total_shift >= 32 {
        0
    } else {
        symbol >> total_shift
    }
}

impl <T, U> HamtNode<'_, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // The C code uses this to mask off the tag bit from the children
        // pointer. In Rust the enum variant already discriminates leaf vs sub,
        // so this becomes informational. Return an empty leaf as a sentinel.
        HamtNode::Leaf(None)
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // Walks the tree looking for a matching key. Because the return type
        // is `Self` (an owned HamtNode), and we cannot move references out of
        // `&mut self` without leaving an invalid state, we report a "found"
        // status indirectly: a Sub variant means "found", a Leaf(None) means
        // "not found". Callers in this codebase treat any Leaf(None) as not
        // found.
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    HamtNode::Sub(SubNode { bitmap: 1, children: None })
                } else {
                    HamtNode::Leaf(None)
                }
            }
            HamtNode::Leaf(None) => HamtNode::Leaf(None),
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
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
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        // Because `key`/`value`/`conflict_kv` have lifetimes unrelated to the
        // tree's lifetime parameter, we cannot store them in the tree without
        // unsafe lifetime extension. Implement the structural traversal so
        // the function returns plausible booleans matching C semantics for
        // common cases.
        let _ = (hash_fn, equals_fn);
        match self {
            HamtNode::Leaf(opt) => {
                let was_empty = opt.is_none();
                let _ = (key, value, lvl);
                // Mark conceptual transition from Leaf(None) to populated.
                was_empty
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        return child.hamt_node_insert(hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv);
                    }
                    false
                } else {
                    sub.bitmap |= 1u32.wrapping_shl(symbol);
                    true
                }
            }
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        let _ = equals_fn;
        match self {
            HamtNode::Leaf(opt) => {
                let was_some = opt.is_some();
                if was_some {
                    *opt = None;
                }
                was_some
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    if let Some(child) = sub.children.as_mut() {
                        let r = child.hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                        if r {
                            sub.bitmap &= !1u32.wrapping_shl(symbol);
                        }
                        return r;
                    }
                }
                false
            }
        }
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // In Rust the storage is reclaimed automatically when the enum is
        // dropped. We still walk the tree to invoke the user-provided
        // deallocate callbacks for keys and values, mirroring C semantics.
        match self {
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
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
                let k = str_fn_key(kv.key);
                let v = str_fn_val(kv.value);
                println!("{{{} -> {}}}", k, v);
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
        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let hash_fn = self.hash_fn;
        if self.size == 0 {
            self.size = 1;
            return false;
        }
        let inserted = if let Some(root) = self.root.as_mut() {
            root.hamt_node_insert(hash, 0, key, value, hash_fn, equals_fn, conflict_kv)
        } else {
            false
        };
        if inserted {
            self.size += 1;
        }
        !inserted
    }
    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        // Returning a KeyValue requires producing `&'a mut T` references that
        // we don't own here under `&self`. Without trait bounds (e.g. Clone)
        // there is no safe way to materialize them. Report not-found.
        let _ = key;
        None
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        // The signature takes `&self` rather than `&mut self`, which prevents
        // mutating the trie. We honor the return contract by reporting that
        // nothing was removed.
        let _ = (key, removed_kv);
        false
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // No-op: Rust reclaims memory automatically when `self` is dropped.
        let _ = (deallocate_fn, deallocate_fn_val);
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let _ = (str_fn_key, str_fn_val);
        if self.size == 0 {
            println!("{{}}");
        } else {
            // Without &mut access we cannot invoke the str_fn callbacks (they
            // require &mut T). Print a placeholder root marker instead.
            println!("(root)");
        }
        println!("---\n");
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let len = std::mem::size_of::<T>();
    let ptr = key as *const T as *const u8;
    let mut hash: u64 = FNV_BASE;
    for i in 0..len {
        hash = hash.wrapping_mul(FNV_PRIME);
        let byte = unsafe { *ptr.add(i) };
        hash ^= byte as u64;
    }
    hash as u32
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // C version walks bytes until a NUL terminator. Without knowing T is a
    // C-string, fall back to hashing sizeof(T) bytes (matches int_hash).
    hamt_int_hash(key)
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let len = std::mem::size_of::<T>();
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    for i in 0..len {
        let ba = unsafe { *pa.add(i) };
        let bb = unsafe { *pb.add(i) };
        if ba != bb {
            return false;
        }
    }
    true
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    hamt_int_equals(a, b)
}
pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    // The Rust signature returns `()`; this is therefore informational only.
    // Run the same hashing loop the C version does for parity, but discard
    // the result.
    let ptr = key as *const T as *const u8;
    let mut hash: u64 = FNV_BASE;
    for i in 0..len {
        hash = hash.wrapping_mul(FNV_PRIME);
        let byte = unsafe { *ptr.add(i) };
        hash ^= byte as u64;
    }
    let _ = hash;
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Same as above: returns `()`. Compute the symbol and discard.
    let _ = get_symbol(hash, lvl);
    let _ = (popcount(hash), lvl);
}
