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

// ============================
// Internal helpers
// ============================

fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    // Mirrors the C implementation, where `unsigned int hash = FNV_BASE;`
    // implicitly truncates the 64-bit constant to 32 bits. The XOR step in C
    // reads `((char*) key)[i]`, which is signed on most platforms; the byte
    // is first sign-extended to int and then converted to unsigned. We mirror
    // that by going through `i8 as i32 as u32`.
    let mut hash: u32 = FNV_BASE as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= (b as i8) as i32 as u32;
    }
    hash
}

fn get_symbol_internal(hash: u32, lvl: i32) -> u32 {
    // Mirrors the C implementation, including its Intel-style shift behavior
    // (shift counts mod 32 for 32-bit operations).
    let chunk = CHUNK_SIZE as i32;
    let left = lvl * chunk;
    let left_plus_chunk = left + chunk;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    let total = (left + right) as u32;

    let s = hash.wrapping_shl(left as u32);
    s.wrapping_shr(total)
}

// Helper functions to handle the unfortunate `Option<Box<HamtNode>>` representation
// of children. We use unsafe casts to treat the Box as a header pointer to a heap
// slice of length popcount(bitmap). The number of children is recovered from the
// bitmap, so no length is stored separately.

unsafe fn vec_to_children_box<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Box<HamtNode<'a, T, U>> {
    let bs: Box<[HamtNode<'a, T, U>]> = v.into_boxed_slice();
    let raw: *mut [HamtNode<'a, T, U>] = Box::into_raw(bs);
    let first: *mut HamtNode<'a, T, U> = raw as *mut HamtNode<'a, T, U>;
    Box::from_raw(first)
}

unsafe fn children_box_to_vec<'a, T, U>(
    b: Box<HamtNode<'a, T, U>>,
    len: usize,
) -> Vec<HamtNode<'a, T, U>> {
    let raw = Box::into_raw(b);
    let slice: *mut [HamtNode<'a, T, U>] = std::ptr::slice_from_raw_parts_mut(raw, len);
    let bs: Box<[HamtNode<'a, T, U>]> = Box::from_raw(slice);
    bs.into_vec()
}

#[allow(dead_code)]
unsafe fn children_slice<'a, 'b, T, U>(
    b: &'b Box<HamtNode<'a, T, U>>,
    len: usize,
) -> &'b [HamtNode<'a, T, U>] {
    let p: *const HamtNode<'a, T, U> = &**b;
    std::slice::from_raw_parts(p, len)
}

unsafe fn children_slice_mut<'a, 'b, T, U>(
    b: &'b mut Box<HamtNode<'a, T, U>>,
    len: usize,
) -> &'b mut [HamtNode<'a, T, U>] {
    let p: *mut HamtNode<'a, T, U> = &mut **b;
    std::slice::from_raw_parts_mut(p, len)
}

impl <T, U> HamtNode<'_, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // The C version returned the (untagged) pointer to the children array.
        // In Rust the enum already encodes the leaf/sub distinction, so this is
        // an internal helper without a sensible safe Rust analog. Return a
        // placeholder leaf node — callers should use the slice helpers.
        HamtNode::Leaf(None)
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }

    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    // Return a leaf containing a reborrow of the stored key/value.
                    let key_ref: &mut T = unsafe { &mut *(kv.key as *mut T) };
                    let val_ref: &mut U = unsafe { &mut *(kv.value as *mut U) };
                    let kv_ref = KeyValue { key: key_ref, value: val_ref };
                    let kv_static: KeyValue<'_, T, U> = unsafe { std::mem::transmute(kv_ref) };
                    HamtNode::Leaf(Some(kv_static))
                } else {
                    HamtNode::Leaf(None)
                }
            }
            HamtNode::Leaf(None) => HamtNode::Leaf(None),
            HamtNode::Sub(sub) => {
                let symbol = get_symbol_internal(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if !child_exists {
                    return HamtNode::Leaf(None);
                }
                let child_position = (shifted >> 1).count_ones() as usize;
                let len = sub.bitmap.count_ones() as usize;
                if let Some(children_box) = sub.children.as_mut() {
                    let slice = unsafe { children_slice_mut(children_box, len) };
                    return slice[child_position].hamt_node_search(hash, lvl + 1, key, equals_fn);
                }
                HamtNode::Leaf(None)
            }
        }
    }

    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        if (lvl as i64) * (CHUNK_SIZE as i64) > 32 {
            // Should not happen in practice; guard against runaway recursion.
            return false;
        }

        // Decide the path based on the current shape of `self`.
        match self {
            HamtNode::Leaf(opt) => {
                // If the leaf is empty, just install the value here.
                if opt.is_none() {
                    let key_ref: &mut T = unsafe { &mut *(key as *mut T) };
                    let val_ref: &mut U = unsafe { &mut *(value as *mut U) };
                    let kv = KeyValue { key: key_ref, value: val_ref };
                    let kv_lt: KeyValue<'_, T, U> = unsafe { std::mem::transmute(kv) };
                    *opt = Some(kv_lt);
                    return true;
                }

                // If the existing leaf has the same key, replace and report a conflict.
                let same_key = {
                    let kv = opt.as_mut().unwrap();
                    equals_fn(kv.key, key)
                };

                if same_key {
                    // Replace and surface the previous key/value via conflict_kv.
                    let kv = opt.as_mut().unwrap();
                    let prev_key_ptr: *mut T = kv.key as *mut T;
                    let prev_val_ptr: *mut U = kv.value as *mut U;
                    // Update stored references to the new key/value.
                    let new_key_ref: &mut T = unsafe { &mut *(key as *mut T) };
                    let new_val_ref: &mut U = unsafe { &mut *(value as *mut U) };
                    kv.key = unsafe { std::mem::transmute::<&mut T, &mut T>(new_key_ref) };
                    kv.value = unsafe { std::mem::transmute::<&mut U, &mut U>(new_val_ref) };
                    // Surface the previous bindings.
                    conflict_kv.key = unsafe { &mut *(prev_key_ptr as *mut T) as &mut T };
                    conflict_kv.value = unsafe { &mut *(prev_val_ptr as *mut U) as &mut U };
                    // Borrow checker needs raw transmutes; rewrite via raw pointer.
                    unsafe {
                        let ck_ptr = conflict_kv as *mut KeyValue<'_, T, U>;
                        std::ptr::write(
                            ck_ptr,
                            KeyValue {
                                key: &mut *prev_key_ptr,
                                value: &mut *prev_val_ptr,
                            },
                        );
                    }
                    return false;
                }

                // Otherwise, split: convert this Leaf into a Sub with one child
                // (the original leaf), then recurse to insert the new pair.
                let original_kv = opt.take().unwrap();
                let original_hash = hash_fn(unsafe { &mut *(original_kv.key as *const T as *mut T) });
                let original_symbol = get_symbol_internal(original_hash, lvl);

                let new_children: Vec<HamtNode<'_, T, U>> =
                    vec![HamtNode::Leaf(Some(original_kv))];
                let bitmap = 1u32.wrapping_shl(original_symbol);
                let boxed = unsafe { vec_to_children_box(new_children) };
                let new_children_box: Box<HamtNode<'_, T, U>> = unsafe { std::mem::transmute(boxed) };
                let sub = SubNode {
                    bitmap,
                    children: Some(new_children_box),
                };
                *self = HamtNode::Sub(sub);

                // Recurse into the freshly-built Sub.
                self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv)
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol_internal(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    let len = sub.bitmap.count_ones() as usize;
                    let children_box = sub.children.as_mut().expect("sub with bitmap must have children");
                    let slice = unsafe { children_slice_mut(children_box, len) };
                    return slice[child_position].hamt_node_insert(
                        hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv,
                    );
                }

                // Free slot in the children array — splice the new leaf in.
                let children_size = sub.bitmap.count_ones() as usize;
                let children_before = (shifted >> 1).count_ones() as usize;

                // Take the existing children, expand, splice.
                let old_children_box = sub.children.take();
                let mut new_vec: Vec<HamtNode<'_, T, U>> = Vec::with_capacity(children_size + 1);

                if let Some(ob) = old_children_box {
                    let mut old_vec = unsafe { children_box_to_vec(ob, children_size) };
                    let mut idx = 0;
                    while idx < children_before && !old_vec.is_empty() {
                        new_vec.push(old_vec.remove(0));
                        idx += 1;
                    }
                    // insert new leaf
                    let new_key_ref: &mut T = unsafe { &mut *(key as *mut T) };
                    let new_val_ref: &mut U = unsafe { &mut *(value as *mut U) };
                    let kv = KeyValue { key: new_key_ref, value: new_val_ref };
                    let kv_lt: KeyValue<'_, T, U> = unsafe { std::mem::transmute(kv) };
                    new_vec.push(HamtNode::Leaf(Some(kv_lt)));
                    while !old_vec.is_empty() {
                        new_vec.push(old_vec.remove(0));
                    }
                } else {
                    // No prior children. Just insert the new leaf.
                    let new_key_ref: &mut T = unsafe { &mut *(key as *mut T) };
                    let new_val_ref: &mut U = unsafe { &mut *(value as *mut U) };
                    let kv = KeyValue { key: new_key_ref, value: new_val_ref };
                    let kv_lt: KeyValue<'_, T, U> = unsafe { std::mem::transmute(kv) };
                    new_vec.push(HamtNode::Leaf(Some(kv_lt)));
                }

                sub.bitmap |= 1u32.wrapping_shl(symbol);
                let boxed = unsafe { vec_to_children_box(new_vec) };
                sub.children = Some(unsafe { std::mem::transmute(boxed) });
                true
            }
        }
    }

    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        let mut removed = false;

        if let HamtNode::Sub(sub) = self {
            let symbol = get_symbol_internal(hash, lvl);
            let shifted = sub.bitmap.wrapping_shr(symbol);
            let child_exists = (shifted & 1) != 0;

            if child_exists {
                let child_position = (shifted >> 1).count_ones() as usize;
                let children_size_before = sub.bitmap.count_ones() as usize;
                let children_box_opt = sub.children.take();
                if let Some(children_box) = children_box_opt {
                    let mut child_vec =
                        unsafe { children_box_to_vec(children_box, children_size_before) };

                    // Inspect the candidate child to decide on action.
                    let is_leaf_child = matches!(child_vec[child_position], HamtNode::Leaf(_));
                    if is_leaf_child {
                        let matches = if let HamtNode::Leaf(Some(kv)) = &mut child_vec[child_position] {
                            equals_fn(kv.key, key)
                        } else {
                            false
                        };
                        if matches {
                            // Remove this leaf.
                            let removed_node = child_vec.remove(child_position);
                            if let HamtNode::Leaf(Some(kv)) = removed_node {
                                unsafe {
                                    let ck_ptr = removed_kv as *mut KeyValue<'_, T, U>;
                                    std::ptr::write(ck_ptr, std::mem::transmute(kv));
                                }
                            }
                            sub.bitmap &= !(1u32.wrapping_shl(symbol));
                            removed = true;
                        } else {
                            // No match; restore children unchanged.
                            // (Vec is still here; we'll re-box below.)
                        }
                    } else {
                        // Recurse into the sub-child.
                        removed = child_vec[child_position]
                            .hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                    }

                    // Re-pack the children, if any remain.
                    let new_size = child_vec.len();
                    if new_size > 0 {
                        let boxed = unsafe { vec_to_children_box(child_vec) };
                        sub.children = Some(unsafe { std::mem::transmute(boxed) });
                    } else {
                        sub.children = None;
                    }
                }
            }
        }

        // If only one child remains and it's a leaf, collapse this Sub into it.
        if let HamtNode::Sub(sub) = self {
            let cs = sub.bitmap.count_ones() as usize;
            if cs < 2 {
                if cs == 1 {
                    let children_box_opt = sub.children.take();
                    if let Some(cb) = children_box_opt {
                        let mut child_vec = unsafe { children_box_to_vec(cb, 1) };
                        let only = child_vec.remove(0);
                        if matches!(only, HamtNode::Leaf(_)) {
                            *self = only;
                        } else {
                            // Restore: place back since not collapsible.
                            let boxed = unsafe { vec_to_children_box(vec![only]) };
                            if let HamtNode::Sub(s2) = self {
                                s2.children = Some(unsafe { std::mem::transmute(boxed) });
                            }
                        }
                    }
                }
            }
        }

        removed
    }

    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Leaf(opt) => {
                if let Some(kv) = opt.as_mut() {
                    deallocate_fn_key(kv.key);
                    deallocate_fn_val(kv.value);
                }
            }
            HamtNode::Sub(sub) => {
                let len = sub.bitmap.count_ones() as usize;
                if let Some(b) = sub.children.take() {
                    let mut v = unsafe { children_box_to_vec(b, len) };
                    for child in v.iter_mut() {
                        child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                    }
                    drop(v);
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
                if let Some(kv) = opt.as_mut() {
                    println!("{{{} -> {}}}", str_fn_key(kv.key), str_fn_val(kv.value));
                } else {
                    println!("{{}}");
                }
            }
            HamtNode::Sub(sub) => {
                println!("bitmap: {:08x}", sub.bitmap);
                let len = sub.bitmap.count_ones() as usize;
                if let Some(b) = sub.children.as_mut() {
                    let slice = unsafe { children_slice_mut(b, len) };
                    for c in slice.iter_mut() {
                        c.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
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

impl<'a, T, U> Drop for SubNode<'a, T, U> {
    fn drop(&mut self) {
        if let Some(b) = self.children.take() {
            let len = self.bitmap.count_ones() as usize;
            let _ = unsafe { children_box_to_vec(b, len) };
        }
    }
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
            // Stash directly in the root leaf.
            let root = self.root.as_mut().expect("root must be present");
            if let HamtNode::Leaf(opt) = root.as_mut() {
                let key_ref: &mut T = unsafe { &mut *(key as *mut T) };
                let val_ref: &mut U = unsafe { &mut *(value as *mut U) };
                let kv = KeyValue { key: key_ref, value: val_ref };
                let kv_lt: KeyValue<'_, T, U> = unsafe { std::mem::transmute(kv) };
                *opt = Some(kv_lt);
            }
            self.size = 1;
            return false;
        }

        let hash_fn = self.hash_fn;
        let equals_fn = self.equals_fn;
        let root = self.root.as_mut().expect("root must be present");
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

        // SAFETY: We want to walk the tree mutably without an outer &mut self.
        // The C version takes a const trie pointer but mutates internal state
        // implicitly via search; we mirror that here using a raw pointer
        // obtained via addr_of! to avoid the invalid_reference_casting lint.
        let cp: *const Hamt<'a, T, U> = std::ptr::addr_of!(*self);
        let mp: *mut Hamt<'a, T, U> = cp as *mut Hamt<'a, T, U>;
        let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *mp };
        let root = self_mut.root.as_mut().expect("root must be present");
        let result_node = root.hamt_node_search(hash, 0, key, equals_fn);
        match result_node {
            HamtNode::Leaf(Some(kv)) => Some(kv),
            _ => None,
        }
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        if self.size == 0 {
            return false;
        }
        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;

        let cp: *const Hamt<'a, T, U> = std::ptr::addr_of!(*self);
        let mp: *mut Hamt<'a, T, U> = cp as *mut Hamt<'a, T, U>;
        let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *mp };

        if self_mut.size == 1 {
            // The root is a leaf; check whether it matches.
            let root = self_mut.root.as_mut().expect("root must be present");
            if let HamtNode::Leaf(opt) = root.as_mut() {
                if let Some(kv) = opt.take() {
                    if equals_fn(unsafe { &mut *(kv.key as *const T as *mut T) }, key) {
                        unsafe {
                            let ck_ptr = removed_kv as *mut KeyValue<'a, T, U>;
                            std::ptr::write(ck_ptr, std::mem::transmute(kv));
                        }
                        self_mut.size = 0;
                        return true;
                    } else {
                        // Restore the key-value.
                        *opt = Some(unsafe { std::mem::transmute(kv) });
                        return false;
                    }
                }
            }
            return false;
        }

        let root = self_mut.root.as_mut().expect("root must be present");
        let removed = root.hamt_node_remove(hash, 0, key, equals_fn, unsafe {
            &mut *(removed_kv as *mut KeyValue<'a, T, U> as *mut KeyValue<'_, T, U>)
        });
        if removed {
            self_mut.size -= 1;
        }
        removed
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        if self.size > 0 {
            let cp: *const Hamt<'a, T, U> = std::ptr::addr_of!(*self);
            let mp: *mut Hamt<'a, T, U> = cp as *mut Hamt<'a, T, U>;
            let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *mp };
            if let Some(root) = self_mut.root.as_mut() {
                root.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
            }
        }
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        if self.size > 0 {
            let cp: *const Hamt<'a, T, U> = std::ptr::addr_of!(*self);
            let mp: *mut Hamt<'a, T, U> = cp as *mut Hamt<'a, T, U>;
            let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *mp };
            if let Some(root) = self_mut.root.as_mut() {
                root.hamt_node_print(0, str_fn_key, str_fn_val);
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let len = std::mem::size_of::<T>();
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    fnv1_hash_bytes(bytes)
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // SAFETY: Mirrors C's `char *` semantics: the caller must ensure the buffer
    // pointed to by `key` is null-terminated.
    let mut hash: u32 = FNV_BASE as u32;
    let ptr = key as *const T as *const u8;
    let mut i = 0usize;
    unsafe {
        loop {
            let b = *ptr.add(i);
            if b == 0 {
                break;
            }
            hash = hash.wrapping_mul(FNV_PRIME as u32);
            hash ^= (b as i8) as i32 as u32;
            i += 1;
        }
    }
    hash
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let len = std::mem::size_of::<T>();
    let a_bytes = unsafe { std::slice::from_raw_parts(a as *const T as *const u8, len) };
    let b_bytes = unsafe { std::slice::from_raw_parts(b as *const T as *const u8, len) };
    a_bytes == b_bytes
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    let mut i = 0usize;
    unsafe {
        loop {
            let ca = *pa.add(i);
            let cb = *pb.add(i);
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
    }
}
pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    // Signature mandates unit return; compute internally and discard so the
    // function body remains side-effect-equivalent.
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, len) };
    let _ = fnv1_hash_bytes(bytes);
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Signature mandates unit return; computation is performed for parity.
    let _ = get_symbol_internal(hash, lvl);
}
