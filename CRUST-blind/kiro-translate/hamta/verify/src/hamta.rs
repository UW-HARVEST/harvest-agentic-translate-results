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
        // In the Rust version, this is not needed as we use enum variants directly.
        // Return a placeholder — actual child access is done via pattern matching.
        match self {
            HamtNode::Sub(sub) => {
                // Return the node itself conceptually; in practice we access children directly
                HamtNode::Sub(SubNode {
                    bitmap: sub.bitmap,
                    children: Vec::new(),
                })
            }
            HamtNode::Leaf(opt) => HamtNode::Leaf(opt.take()),
        }
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // This signature returns Self but conceptually should return Option.
        // We return Leaf(None) for "not found" and Leaf(Some(...)) for found.
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    // Can't move out, so we return a Leaf(None) as a signal of "found"
                    // The caller should use hamt_search which handles this differently.
                    // For interface compliance, return Leaf(None) — actual search is in Hamt::hamt_search
                    HamtNode::Leaf(None)
                } else {
                    HamtNode::Leaf(None)
                }
            }
            HamtNode::Leaf(None) => HamtNode::Leaf(None),
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    sub.children[child_position].hamt_node_search(hash, lvl + 1, key, equals_fn)
                } else {
                    HamtNode::Leaf(None)
                }
            }
        }
    }
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T, 
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        if (lvl as usize) * CHUNK_SIZE > 32 {
            return false;
        }

        match self {
            HamtNode::Leaf(leaf_opt) => {
                if let Some(kv) = leaf_opt {
                    if equals_fn(kv.key, key) {
                        // Conflict: swap keys and values
                        std::mem::swap(&mut *conflict_kv.key, &mut *kv.key);
                        std::mem::swap(&mut *conflict_kv.value, &mut *kv.value);
                        std::mem::swap(kv.key, key);
                        std::mem::swap(kv.value, value);
                        // Now conflict_kv has old, node has new
                        return false;
                    }
                    // Convert leaf to sub-node, then re-insert
                    let original_hash = hash_fn(kv.key);
                    let original_symbol = hamt_get_symbol_impl(original_hash, lvl);

                    let leaf_kv = leaf_opt.take().unwrap();
                    let new_child = HamtNode::Leaf(Some(leaf_kv));

                    let bitmap = 1u32 << original_symbol;
                    *self = HamtNode::Sub(SubNode {
                        bitmap,
                        children: vec![new_child],
                    });
                    return self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
                } else {
                    // Empty leaf — shouldn't happen in normal flow
                    return false;
                }
            }
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    sub.children[child_position].hamt_node_insert(hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv)
                } else {
                    let children_before = (shifted >> 1).count_ones() as usize;
                    sub.bitmap |= 1 << symbol;

                    // Create new leaf for the inserted key/value
                    // We need to create a leaf but we only have &mut references...
                    // Use unsafe to extend lifetime — this mirrors the C code's raw pointer semantics
                    let new_leaf = HamtNode::Leaf(Some(KeyValue {
                        key: unsafe { &mut *(key as *mut T) },
                        value: unsafe { &mut *(value as *mut U) },
                    }));
                    sub.children.insert(children_before, new_leaf);
                    true
                }
            }
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        match self {
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) != 0;

                let mut removed = false;
                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;

                    if sub.children[child_position].is_leaf() {
                        // Check if this leaf matches
                        if let HamtNode::Leaf(Some(kv)) = &mut sub.children[child_position] {
                            if equals_fn(kv.key, key) {
                                std::mem::swap(removed_kv.key, kv.key);
                                std::mem::swap(removed_kv.value, kv.value);
                                sub.bitmap &= !(1 << symbol);
                                sub.children.remove(child_position);
                                removed = true;
                            }
                        }
                    } else {
                        removed = sub.children[child_position].hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                    }
                }

                // Collapse if only one child remains and it's a leaf
                let children_size = sub.bitmap.count_ones() as usize;
                if children_size < 2 && children_size == 1 {
                    if sub.children[0].is_leaf() {
                        let child = sub.children.remove(0);
                        *self = child;
                    }
                }

                removed
            }
            _ => false,
        }
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Sub(sub) => {
                for child in sub.children.iter_mut() {
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
            }
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
            }
            HamtNode::Leaf(None) => {}
        }
    }
    pub fn hamt_node_print(&mut self, lvl:i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let indent = "  ".repeat(lvl as usize);
        match self {
            HamtNode::Leaf(Some(kv)) => {
                println!("{}{{{} -> {}}}", indent, str_fn_key(kv.key), str_fn_val(kv.value));
            }
            HamtNode::Leaf(None) => {}
            HamtNode::Sub(sub) => {
                println!("{}bitmap: {:08x}", indent, sub.bitmap);
                for child in sub.children.iter_mut() {
                    child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                }
            }
        }
    }
}
pub struct SubNode<'a, T, U> {
    pub bitmap: u32,
    pub children: Vec<HamtNode<'a, T, U>>,
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
        if self.size == 0 {
            let root = self.root.as_mut().unwrap();
            *root.as_mut() = HamtNode::Leaf(Some(KeyValue {
                key: unsafe { &mut *(key as *mut T) },
                value: unsafe { &mut *(value as *mut U) },
            }));
            self.size = 1;
            return false;
        }

        let hash = (self.hash_fn)(key);
        let hash_fn = self.hash_fn;
        let equals_fn = self.equals_fn;
        let root = self.root.as_mut().unwrap();
        let inserted = root.hamt_node_insert(hash, 0, key, value, hash_fn, equals_fn, conflict_kv);

        if inserted {
            self.size += 1;
        }
        !inserted
    }
    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        if self.size == 0 {
            return None;
        }
        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let root_ptr = &**self.root.as_ref().unwrap() as *const HamtNode<'a, T, U>;
        hamt_node_search_impl(root_ptr.cast_mut(), hash, 0, key, equals_fn)
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        if self.size == 0 {
            return false;
        }

        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;

        let self_ptr = (self as *const Hamt<'a, T, U>).cast_mut();
        unsafe {
            let mut removed = false;

            if (*self_ptr).size == 1 {
                if let HamtNode::Leaf(Some(kv)) = (*self_ptr).root.as_mut().unwrap().as_mut() {
                    if equals_fn(kv.key, key) {
                        std::mem::swap(removed_kv.key, kv.key);
                        std::mem::swap(removed_kv.value, kv.value);
                        removed = true;
                    }
                }
            } else {
                removed = (*self_ptr).root.as_mut().unwrap().hamt_node_remove(hash, 0, key, equals_fn, removed_kv);
            }

            if removed {
                (*self_ptr).size -= 1;
            }
            removed
        }
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let self_ptr = (self as *const Hamt<'a, T, U>).cast_mut();
        unsafe {
            if (*self_ptr).size > 0 {
                (*self_ptr).root.as_mut().unwrap().hamt_node_destroy(deallocate_fn, deallocate_fn_val);
            }
        }
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        if self.size > 0 {
            let root_ptr = &**self.root.as_ref().unwrap() as *const HamtNode<'a, T, U>;
            unsafe {
                (*(root_ptr.cast_mut())).hamt_node_print(0, str_fn_key, str_fn_val);
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}

fn hamt_node_search_impl<'a, T, U>(node: *mut HamtNode<'a, T, U>, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Option<KeyValue<'a, T, U>> {
    unsafe {
        match &mut *node {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    Some(KeyValue {
                        key: &mut *(kv.key as *mut T),
                        value: &mut *(kv.value as *mut U),
                    })
                } else {
                    None
                }
            }
            HamtNode::Leaf(None) => None,
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                if (shifted & 1) != 0 {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    hamt_node_search_impl(&mut sub.children[child_position] as *mut _, hash, lvl + 1, key, equals_fn)
                } else {
                    None
                }
            }
        }
    }
}

fn hamt_get_symbol_impl(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as u32) * (CHUNK_SIZE as u32);
    let left_plus_chunk = left + CHUNK_SIZE as u32;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    let symbol = hash << left;
    symbol >> (right + left)
}

// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let bytes = unsafe {
        std::slice::from_raw_parts(key as *const T as *const u8, std::mem::size_of::<T>())
    };
    hamt_fnv1_hash_impl(bytes)
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Treat T as a String or &str — get bytes
    // In practice, T would be String
    let s = unsafe { &*(key as *const T as *const String) };
    let mut hash = FNV_BASE;
    for &b in s.as_bytes() {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= b as u64;
    }
    hash as u32
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let a_bytes = unsafe {
        std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
    };
    let b_bytes = unsafe {
        std::slice::from_raw_parts(b as *const T as *const u8, std::mem::size_of::<T>())
    };
    a_bytes == b_bytes
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    let a_str = unsafe { &*(a as *const T as *const String) };
    let b_str = unsafe { &*(b as *const T as *const String) };
    a_str == b_str
}
pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    let bytes = unsafe {
        std::slice::from_raw_parts(key as *const T as *const u8, len)
    };
    hamt_fnv1_hash_impl(bytes);
}

fn hamt_fnv1_hash_impl(bytes: &[u8]) -> u32 {
    let mut hash: u64 = FNV_BASE;
    for &b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= b as u64;
    }
    hash as u32
}

pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    hamt_get_symbol_impl(hash, lvl);
}
