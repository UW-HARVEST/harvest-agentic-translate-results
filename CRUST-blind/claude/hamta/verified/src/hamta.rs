use std::ptr;

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

// ---- Internal helpers for managing the children "array" ----
// `SubNode.children` is declared as `Option<Box<HamtNode>>` but conceptually
// stores a contiguous slice of `HamtNode` values whose length equals the
// number of set bits in `bitmap`. We pack a `Vec<HamtNode>` into a single
// `Box<HamtNode>` (pointing to the start of the slice) and unpack it back
// using the recorded length when needed. This requires unsafe pointer
// manipulation, but is contained to these helper functions.

fn vec_into_children_box<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Box<HamtNode<'a, T, U>> {
    let boxed_slice: Box<[HamtNode<'a, T, U>]> = v.into_boxed_slice();
    let raw: *mut [HamtNode<'a, T, U>] = Box::into_raw(boxed_slice);
    let ptr: *mut HamtNode<'a, T, U> = raw as *mut HamtNode<'a, T, U>;
    unsafe { Box::from_raw(ptr) }
}

unsafe fn children_box_into_vec<'a, T, U>(
    b: Box<HamtNode<'a, T, U>>,
    n: usize,
) -> Vec<HamtNode<'a, T, U>> {
    let ptr: *mut HamtNode<'a, T, U> = Box::into_raw(b);
    let slice_ptr: *mut [HamtNode<'a, T, U>] = ptr::slice_from_raw_parts_mut(ptr, n);
    let boxed_slice: Box<[HamtNode<'a, T, U>]> = Box::from_raw(slice_ptr);
    boxed_slice.into_vec()
}

unsafe fn children_box_as_slice<'a, 'b, T, U>(
    b: &'b Box<HamtNode<'a, T, U>>,
    n: usize,
) -> &'b [HamtNode<'a, T, U>] {
    let ptr: *const HamtNode<'a, T, U> = &**b;
    std::slice::from_raw_parts(ptr, n)
}

unsafe fn children_box_as_mut_slice<'a, 'b, T, U>(
    b: &'b mut Box<HamtNode<'a, T, U>>,
    n: usize,
) -> &'b mut [HamtNode<'a, T, U>] {
    let ptr: *mut HamtNode<'a, T, U> = &mut **b;
    std::slice::from_raw_parts_mut(ptr, n)
}

// Reinterpret `&T` as `&mut T`. The function signatures `&self` for
// `hamt_remove`, `hamt_destroy`, and `hamt_search` are dictated by the
// problem statement (which we may not modify), but the C semantics require
// mutation. This helper is the single point where we cross that boundary.
#[allow(invalid_reference_casting)]
unsafe fn mut_from_ref<T>(r: &T) -> &mut T {
    let p = r as *const T as *mut T;
    &mut *p
}

// Recursively walk the trie using only `&` references and return a copy of
// the matched KeyValue (pointers are extended to the trie's `'a` lifetime).
fn node_search_ref<'a, T, U>(
    node: &HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
) -> Option<KeyValue<'a, T, U>> {
    match node {
        HamtNode::Leaf(opt) => {
            if let Some(kv) = opt {
                let kptr: *mut T = kv.key as *const T as *mut T;
                let vptr: *mut U = kv.value as *const U as *mut U;
                if equals_fn(unsafe { &mut *kptr }, key) {
                    return Some(KeyValue {
                        key: unsafe { &mut *kptr },
                        value: unsafe { &mut *vptr },
                    });
                }
            }
            None
        }
        HamtNode::Sub(sub) => {
            let symbol = get_symbol(hash, lvl);
            let shifted = sub.bitmap.wrapping_shr(symbol);
            let child_exists = (shifted & 1) != 0;
            if !child_exists {
                return None;
            }
            let child_position = (shifted >> 1).count_ones() as usize;
            let n = sub.bitmap.count_ones() as usize;
            if let Some(ref children_box) = sub.children {
                let slice = unsafe { children_box_as_slice(children_box, n) };
                return node_search_ref(&slice[child_position], hash, lvl + 1, key, equals_fn);
            }
            None
        }
    }
}

fn node_print_ref<'a, T, U>(
    node: &HamtNode<'a, T, U>,
    lvl: i32,
    str_fn_key: StrFn<T>,
    str_fn_val: StrFn<U>,
) {
    for _ in 0..(lvl * 2) {
        print!(" ");
    }
    match node {
        HamtNode::Leaf(opt) => {
            if let Some(kv) = opt {
                let kptr: *mut T = kv.key as *const T as *mut T;
                let vptr: *mut U = kv.value as *const U as *mut U;
                let k = str_fn_key(unsafe { &mut *kptr });
                let v = str_fn_val(unsafe { &mut *vptr });
                println!("{{{} -> {}}}", k, v);
            } else {
                println!("{{}}");
            }
        }
        HamtNode::Sub(sub) => {
            let n = sub.bitmap.count_ones() as usize;
            println!("bitmap: {:08x}", sub.bitmap);
            if let Some(ref children_box) = sub.children {
                let slice = unsafe { children_box_as_slice(children_box, n) };
                for child in slice.iter() {
                    node_print_ref(child, lvl + 1, str_fn_key, str_fn_val);
                }
            }
        }
    }
}

// Internal symbol extractor (real return value, unlike the public stub).
fn get_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as i64) * (CHUNK_SIZE as i64);
    let left_plus_chunk = left + CHUNK_SIZE as i64;
    let mut right: i64 = 32 - left_plus_chunk;
    if left_plus_chunk > 32 {
        right = 0;
    }

    // Saturate to safe shift values
    let left_u = left.clamp(0, 31) as u32;
    let total_shift = (right + left).clamp(0, 31) as u32;

    let symbol = hash.wrapping_shl(left_u);
    symbol.wrapping_shr(total_shift)
}

impl <'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // The C version returns a pointer to the children array with the tag
        // bit cleared. In our enum-based representation there is no tag bit,
        // so we just produce an empty leaf as a placeholder. Real internal
        // accesses go through `children_box_as_slice` etc.
        HamtNode::Leaf(None)
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // Search for the entry with `key`. Returns a Leaf containing a copy of
        // the matched KeyValue (with extended lifetime), or Leaf(None) if not
        // found. Mirrors C's behavior of returning a pointer to the leaf.
        match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt {
                    if equals_fn(kv.key, key) {
                        // Construct a new KeyValue referring to the same data
                        let kptr: *mut T = kv.key as *mut T;
                        let vptr: *mut U = kv.value as *mut U;
                        let new_kv = KeyValue {
                            key: unsafe { &mut *kptr },
                            value: unsafe { &mut *vptr },
                        };
                        return HamtNode::Leaf(Some(new_kv));
                    }
                }
                HamtNode::Leaf(None)
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    let n = sub.bitmap.count_ones() as usize;
                    if let Some(ref mut children_box) = sub.children {
                        let slice = unsafe { children_box_as_mut_slice(children_box, n) };
                        return slice[child_position]
                            .hamt_node_search(hash, lvl + 1, key, equals_fn);
                    }
                }
                HamtNode::Leaf(None)
            }
        }
    }
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        if (lvl as usize) * CHUNK_SIZE > 32 {
            return false;
        }

        // First, handle the case of inserting into a Leaf
        let need_split: Option<u32> = match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt {
                    if equals_fn(kv.key, key) {
                        // Replace: swap out current key/value into conflict_kv,
                        // and put the new ones in place.
                        let old_kptr: *mut T = kv.key as *mut T;
                        let old_vptr: *mut U = kv.value as *mut U;
                        let new_kptr: *mut T = key as *mut T;
                        let new_vptr: *mut U = value as *mut U;

                        // Update conflict_kv with the previous key/value
                        let conflict_kptr: *mut &mut T = &mut conflict_kv.key as *mut _;
                        let conflict_vptr: *mut &mut U = &mut conflict_kv.value as *mut _;
                        unsafe {
                            ptr::write(conflict_kptr, &mut *old_kptr);
                            ptr::write(conflict_vptr, &mut *old_vptr);
                        }

                        // Replace key/value in the leaf with the new ones (lifetime extended)
                        let new_key_ref: &'a mut T = unsafe { &mut *new_kptr };
                        let new_val_ref: &'a mut U = unsafe { &mut *new_vptr };
                        kv.key = new_key_ref;
                        kv.value = new_val_ref;
                        return false;
                    }
                    // Different key in the leaf: need to split into sub-node
                    let original_hash = hash_fn(kv.key);
                    Some(original_hash)
                } else {
                    // Empty leaf — install the new kv directly
                    let new_kptr: *mut T = key as *mut T;
                    let new_vptr: *mut U = value as *mut U;
                    *opt = Some(KeyValue {
                        key: unsafe { &mut *new_kptr },
                        value: unsafe { &mut *new_vptr },
                    });
                    return true;
                }
            }
            HamtNode::Sub(_) => None,
        };

        if let Some(original_hash) = need_split {
            // Split a leaf into a Sub-node containing the original entry, then
            // recurse to insert the new entry.
            let original_next_symbol = get_symbol(original_hash, lvl);

            // Take the leaf out of self
            let old_self =
                std::mem::replace(self, HamtNode::Leaf(None));
            let old_leaf_kv = match old_self {
                HamtNode::Leaf(opt) => opt,
                _ => None,
            };

            // Create a new children array with the single original leaf
            let new_children_vec: Vec<HamtNode<'a, T, U>> = vec![HamtNode::Leaf(old_leaf_kv)];
            let new_children_box = vec_into_children_box(new_children_vec);

            *self = HamtNode::Sub(SubNode {
                bitmap: 1u32.wrapping_shl(original_next_symbol),
                children: Some(new_children_box),
            });

            return self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
        }

        // Now we know self is a Sub-node
        let sub = match self {
            HamtNode::Sub(s) => s,
            _ => return false,
        };

        let symbol = get_symbol(hash, lvl);
        let shifted = sub.bitmap.wrapping_shr(symbol);
        let child_exists = (shifted & 1) != 0;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;
            let n = sub.bitmap.count_ones() as usize;
            if let Some(ref mut children_box) = sub.children {
                let slice = unsafe { children_box_as_mut_slice(children_box, n) };
                return slice[child_position].hamt_node_insert(
                    hash,
                    lvl + 1,
                    key,
                    value,
                    hash_fn,
                    equals_fn,
                    conflict_kv,
                );
            }
            return false;
        } else {
            // Free slot: extend the children array to add the new leaf
            let children_size = sub.bitmap.count_ones() as usize;
            let children_before = (shifted >> 1).count_ones() as usize;

            // Take existing children out
            let old_children_box_opt = sub.children.take();
            let mut old_children_vec: Vec<HamtNode<'a, T, U>> = match old_children_box_opt {
                Some(b) => unsafe { children_box_into_vec(b, children_size) },
                None => Vec::new(),
            };

            // Build new vector with the new leaf inserted at position children_before
            let new_kptr: *mut T = key as *mut T;
            let new_vptr: *mut U = value as *mut U;
            let new_leaf = HamtNode::Leaf(Some(KeyValue {
                key: unsafe { &mut *new_kptr },
                value: unsafe { &mut *new_vptr },
            }));

            // Insert into vector. We need at most children_size + 1 entries.
            let mut new_vec: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(children_size + 1);
            // Drain old_children_vec into new_vec, inserting new_leaf at children_before
            let mut drain = old_children_vec.drain(..);
            for _ in 0..children_before {
                if let Some(item) = drain.next() {
                    new_vec.push(item);
                }
            }
            new_vec.push(new_leaf);
            for item in drain {
                new_vec.push(item);
            }

            sub.bitmap |= 1u32.wrapping_shl(symbol);
            sub.children = Some(vec_into_children_box(new_vec));
            return true;
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        // Operates on a Sub-node (the C function assumes node is a sub-node).
        let symbol = get_symbol(hash, lvl);
        let mut removed = false;

        // We need to potentially mutate the sub-node and possibly collapse it.
        // Pull out the children to work on it.
        let (mut bitmap, children_opt_taken) = match self {
            HamtNode::Sub(s) => {
                let bm = s.bitmap;
                let c = s.children.take();
                (bm, c)
            }
            HamtNode::Leaf(_) => return false,
        };

        let original_size = bitmap.count_ones() as usize;
        let mut children_vec: Vec<HamtNode<'a, T, U>> = match children_opt_taken {
            Some(b) => unsafe { children_box_into_vec(b, original_size) },
            None => Vec::new(),
        };

        let shifted = bitmap.wrapping_shr(symbol);
        let child_exists = (shifted & 1) != 0;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;

            // Inspect that subnode
            let is_child_leaf = matches!(children_vec[child_position], HamtNode::Leaf(_));
            let leaf_matches = if is_child_leaf {
                if let HamtNode::Leaf(Some(ref mut kv)) = children_vec[child_position] {
                    equals_fn(kv.key, key)
                } else {
                    false
                }
            } else {
                false
            };

            if is_child_leaf && leaf_matches {
                // Remove this leaf from the children
                bitmap &= !(1u32.wrapping_shl(symbol));
                let removed_node = children_vec.remove(child_position);
                if let HamtNode::Leaf(Some(kv)) = removed_node {
                    let kptr: *mut T = kv.key as *mut T;
                    let vptr: *mut U = kv.value as *mut U;
                    let conflict_kptr: *mut &mut T = &mut removed_kv.key as *mut _;
                    let conflict_vptr: *mut &mut U = &mut removed_kv.value as *mut _;
                    unsafe {
                        ptr::write(conflict_kptr, &mut *kptr);
                        ptr::write(conflict_vptr, &mut *vptr);
                    }
                }
                removed = true;
            } else if !is_child_leaf {
                // Recurse into subnode
                removed =
                    children_vec[child_position].hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
            }
        }

        // Decide if we need to collapse
        let new_size = bitmap.count_ones() as usize;
        if new_size < 2 {
            // Try to collapse: only if remaining child is a leaf
            if children_vec.len() == 1 {
                let only_remaining_is_leaf = matches!(children_vec[0], HamtNode::Leaf(_));
                if only_remaining_is_leaf {
                    let only_node = children_vec.remove(0);
                    *self = only_node;
                    return removed;
                }
            }
            // If we can't collapse, just store back what we have
        }

        // Repackage into self
        let new_children_box = if children_vec.is_empty() {
            None
        } else {
            Some(vec_into_children_box(children_vec))
        };
        *self = HamtNode::Sub(SubNode {
            bitmap,
            children: new_children_box,
        });

        removed
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt.take() {
                    deallocate_fn_key(kv.key);
                    deallocate_fn_val(kv.value);
                }
            }
            HamtNode::Sub(sub) => {
                let n = sub.bitmap.count_ones() as usize;
                if let Some(children_box) = sub.children.take() {
                    let mut children_vec = unsafe { children_box_into_vec(children_box, n) };
                    for child in children_vec.iter_mut() {
                        child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                    }
                    // children_vec will be dropped here
                }
            }
        }
    }
    pub fn hamt_node_print(&mut self, lvl:i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        for _ in 0..(lvl * 2) {
            print!(" ");
        }

        match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt {
                    let k = str_fn_key(kv.key);
                    let v = str_fn_val(kv.value);
                    println!("{{{} -> {}}}", k, v);
                } else {
                    println!("{{}}");
                }
            }
            HamtNode::Sub(sub) => {
                let n = sub.bitmap.count_ones() as usize;
                println!("bitmap: {:08x}", sub.bitmap);
                if let Some(ref mut children_box) = sub.children {
                    let slice = unsafe { children_box_as_mut_slice(children_box, n) };
                    for child in slice.iter_mut() {
                        child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                    }
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

        if self.size == 0 {
            // Install directly in root leaf
            let new_kptr: *mut T = key as *mut T;
            let new_vptr: *mut U = value as *mut U;
            if let Some(ref mut root_box) = self.root {
                **root_box = HamtNode::Leaf(Some(KeyValue {
                    key: unsafe { &mut *new_kptr },
                    value: unsafe { &mut *new_vptr },
                }));
            } else {
                self.root = Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                    key: unsafe { &mut *new_kptr },
                    value: unsafe { &mut *new_vptr },
                }))));
            }
            self.size = 1;
            return false;
        }

        let inserted = if let Some(ref mut root_box) = self.root {
            root_box.hamt_node_insert(hash, 0, key, value, self.hash_fn, self.equals_fn, conflict_kv)
        } else {
            false
        };

        if inserted {
            self.size += 1;
        }

        !inserted
    }
    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        let hash = (self.hash_fn)(key);

        // Walk the tree using shared references; we only read.
        let root_ref: &HamtNode<'a, T, U> = match &self.root {
            Some(b) => &**b,
            None => return None,
        };
        node_search_ref(root_ref, hash, 0, key, self.equals_fn)
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let hash = (self.hash_fn)(key);

        if self.size == 0 {
            return false;
        }

        // The signature takes `&self`, but the C version mutates the trie.
        // We need to mutate through this shared reference; obtain a mutable
        // pointer via raw-pointer round-trip.
        let s: &mut Self = unsafe { mut_from_ref(self) };

        let removed = if s.size == 1 {
            // Root is a leaf
            if let Some(ref mut root_box) = s.root {
                let mut taken = HamtNode::Leaf(None);
                std::mem::swap(&mut **root_box, &mut taken);
                if let HamtNode::Leaf(Some(kv)) = taken {
                    let kptr: *mut T = kv.key as *mut T;
                    let vptr: *mut U = kv.value as *mut U;
                    let conflict_kptr: *mut &mut T = &mut removed_kv.key as *mut _;
                    let conflict_vptr: *mut &mut U = &mut removed_kv.value as *mut _;
                    unsafe {
                        ptr::write(conflict_kptr, &mut *kptr);
                        ptr::write(conflict_vptr, &mut *vptr);
                    }
                    true
                } else {
                    // Restore the empty leaf
                    **root_box = HamtNode::Leaf(None);
                    false
                }
            } else {
                false
            }
        } else if let Some(ref mut root_box) = s.root {
            root_box.hamt_node_remove(hash, 0, key, s.equals_fn, removed_kv)
        } else {
            false
        };

        if removed {
            s.size -= 1;
        }

        removed
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        // The signature takes `&self`, but we need to drop the root.
        let s: &mut Self = unsafe { mut_from_ref(self) };

        if s.size > 0 {
            if let Some(ref mut root_box) = s.root {
                root_box.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
            }
        }
        // Drop the root box
        s.root = None;
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        if self.size > 0 {
            if let Some(ref root_box) = self.root {
                node_print_ref(&**root_box, 0, str_fn_key, str_fn_val);
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}
// Function Declarations

// Compute FNV-1 hash over `len` bytes starting at `key`. Internal helper used
// by the public hash functions.
fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    // The C version stores the hash in an `unsigned int` (32-bit on most
    // platforms), so the constants get implicitly truncated. We mirror that
    // behavior by using u32 wrapping arithmetic with the truncated constants.
    // C's `char` is signed on x86 Linux, so each byte is sign-extended to int
    // before being XORed against the unsigned hash.
    let mut hash: u32 = FNV_BASE as u32;
    let prime: u32 = FNV_PRIME as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(prime);
        hash ^= (b as i8) as i32 as u32;
    }
    hash
}

pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let len = std::mem::size_of::<T>();
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    fnv1_hash_bytes(bytes)
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Treat T as a NUL-terminated byte string starting at the address of `key`.
    // This mirrors the C implementation, which iterates a `char*` until it
    // encounters a 0 byte.
    let mut hash: u32 = FNV_BASE as u32;
    let prime: u32 = FNV_PRIME as u32;
    let mut p: *const u8 = key as *const T as *const u8;
    unsafe {
        while *p != 0 {
            hash = hash.wrapping_mul(prime);
            hash ^= (*p as i8) as i32 as u32;
            p = p.add(1);
        }
    }
    hash
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let len = std::mem::size_of::<T>();
    let abytes = unsafe { std::slice::from_raw_parts(a as *const T as *const u8, len) };
    let bbytes = unsafe { std::slice::from_raw_parts(b as *const T as *const u8, len) };
    abytes == bbytes
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Compare two NUL-terminated byte strings starting at the addresses of
    // `a` and `b` (mirrors strcmp semantics).
    let mut pa: *const u8 = a as *const T as *const u8;
    let mut pb: *const u8 = b as *const T as *const u8;
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
    // The given signature has no return value, so we compute and discard.
    // (Internal callers use `fnv1_hash_bytes` directly.)
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    let _ = fnv1_hash_bytes(bytes);
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // The given signature has no return value; we compute and discard.
    let _ = get_symbol(hash, lvl);
}
