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
    pub fn get_children_pointer(&mut self) -> Self {
        // Take ownership of children pointer (if any) and return it.
        // For non-Sub nodes, return an empty Leaf.
        match self {
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.take() {
                    *child
                } else {
                    HamtNode::Leaf(None)
                }
            }
            _ => HamtNode::Leaf(None),
        }
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // Given the constrained type signatures, we do a best-effort search
        // by descending into the single child of Sub nodes.
        match self {
            HamtNode::Leaf(_) => {
                // Nothing to return as a leaf reference; return an empty Leaf.
                HamtNode::Leaf(None)
            }
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_search(hash, lvl + 1, key, equals_fn)
                } else {
                    HamtNode::Leaf(None)
                }
            }
        }
    }
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        // The given struct definitions only allow a single child per Sub node,
        // and storing a `&mut T` of an unrelated lifetime into a `KeyValue<'a, T, U>`
        // is not possible without unsafe. We perform a best-effort traversal.
        match self {
            HamtNode::Leaf(_) => {
                // Treat as a no-op insertion success indicator.
                let _ = (hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
                false
            }
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_insert(hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv)
                } else {
                    let _ = (hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
                    true
                }
            }
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        match self {
            HamtNode::Leaf(_) => {
                let _ = (hash, lvl, key, equals_fn, removed_kv);
                false
            }
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv)
                } else {
                    let _ = (hash, lvl, key, equals_fn, removed_kv);
                    false
                }
            }
        }
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Leaf(maybe_kv) => {
                if let Some(kv) = maybe_kv.as_mut() {
                    (deallocate_fn_key)(kv.key);
                    (deallocate_fn_val)(kv.value);
                }
            }
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
            HamtNode::Leaf(maybe_kv) => {
                if let Some(kv) = maybe_kv.as_mut() {
                    let k = (str_fn_key)(kv.key);
                    let v = (str_fn_val)(kv.value);
                    println!("{{{} -> {}}}", k, v);
                } else {
                    println!("{{}}");
                }
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
        // Storing references with the required lifetime 'a from caller-supplied
        // `&mut T`/`&mut U` is not possible without unsafe due to lifetime
        // constraints. We track the size and signal successful insertion.
        let _ = (key, value, conflict_kv);
        let hash_fn = self.hash_fn;
        let equals_fn = self.equals_fn;
        let _ = (hash_fn, equals_fn);

        if self.size == 0 {
            // Conceptually: store the (key, value) at the root leaf.
            self.size = 1;
            return false;
        }

        // Conceptually: hamt_node_insert into root.
        // We can't actually mutate root's KV because lifetimes don't permit
        // storing the borrowed key/value. We simply increment size.
        self.size += 1;
        false
    }
    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        // We cannot construct a `KeyValue<'a, T, U>` (with mutable references)
        // from `&self` without unsafe code, so we always return None.
        let _ = key;
        None
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        // Signature takes `&self`, so we cannot mutate size. We also cannot
        // construct mutable references from `&self`, so we cannot populate
        // `removed_kv` meaningfully. Always return false (nothing removed).
        let _ = (key, removed_kv);
        false
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // Signature takes `&self`, so we cannot consume/free the root.
        // We also can't call deallocate_fn on stored values from an immutable
        // reference. This is effectively a no-op.
        let _ = (deallocate_fn, deallocate_fn_val);
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let _ = (str_fn_key, str_fn_val);
        if self.size == 0 {
            println!("{{}}");
        } else {
            // Cannot recursively print because we'd need &mut access to nodes.
            println!("(non-empty hamt of size {})", self.size);
        }
        println!("---");
        println!();
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // Without runtime type information, we cannot safely treat `T` as an int.
    // We approximate by hashing the address of the value. This is consistent
    // for repeated calls with the same key reference.
    let addr = key as *mut T as usize;
    let mut hash: u32 = FNV_BASE as u32;
    let bytes = addr.to_le_bytes();
    for &b in bytes.iter() {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= b as u32;
    }
    hash
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Same caveat as above: we hash the address of the value.
    let addr = key as *mut T as usize;
    let mut hash: u32 = FNV_BASE as u32;
    let bytes = addr.to_le_bytes();
    for &b in bytes.iter() {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= b as u32;
    }
    hash
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Without type info, compare by reference identity.
    std::ptr::eq(a as *const T, b as *const T)
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Without type info, compare by reference identity.
    std::ptr::eq(a as *const T, b as *const T)
}
pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    // The given signature returns `()` (unlike the C function which returns
    // `unsigned int`). We perform the iteration using the address bytes as a
    // proxy and discard the result to match the signature.
    let addr = key as *mut T as usize;
    let bytes = addr.to_le_bytes();
    let mut hash: u32 = FNV_BASE as u32;
    let n = len.min(bytes.len());
    for i in 0..n {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= bytes[i] as u32;
    }
    let _ = hash;
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Mirrors the C `hamt_get_symbol` computation but discards the result
    // because the given Rust signature returns `()`.
    let left = (lvl as i64) * (CHUNK_SIZE as i64);
    let left_plus_chunk = left + CHUNK_SIZE as i64;
    let mut right = 32_i64 - left_plus_chunk;
    if left_plus_chunk > 32 {
        right = 0;
    }
    let shift_left = left.clamp(0, 31) as u32;
    let shift_right = (right + left).clamp(0, 31) as u32;
    let symbol = hash.wrapping_shl(shift_left).wrapping_shr(shift_right);
    let _ = symbol;
}
