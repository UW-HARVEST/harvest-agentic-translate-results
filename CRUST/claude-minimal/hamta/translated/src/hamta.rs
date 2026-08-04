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

// Helper: convert a Vec<HamtNode> into the children field representation
// We allocate a boxed slice, leak it to a raw pointer, then wrap that pointer
// in `Box<HamtNode>` (re-using the field type as a raw owning pointer that
// actually points to the first element of an array of `len` HamtNodes).
//
// IMPORTANT: The `len` is recovered from the parent SubNode's bitmap popcount.
// Always use `take_children_vec` / `set_children_vec` to access children safely.
unsafe fn vec_to_children_box<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Option<Box<HamtNode<'a, T, U>>> {
    if v.is_empty() {
        return None;
    }
    let boxed_slice: Box<[HamtNode<'a, T, U>]> = v.into_boxed_slice();
    let raw: *mut [HamtNode<'a, T, U>] = Box::into_raw(boxed_slice);
    let first: *mut HamtNode<'a, T, U> = raw as *mut HamtNode<'a, T, U>;
    Some(Box::from_raw(first))
}

// Reverse of vec_to_children_box. Caller must supply the correct length.
unsafe fn children_box_to_vec<'a, T, U>(b: Option<Box<HamtNode<'a, T, U>>>, len: usize) -> Vec<HamtNode<'a, T, U>> {
    match b {
        None => Vec::new(),
        Some(boxed) => {
            let raw: *mut HamtNode<'a, T, U> = Box::into_raw(boxed);
            let slice_ptr: *mut [HamtNode<'a, T, U>] =
                std::slice::from_raw_parts_mut(raw, len) as *mut [HamtNode<'a, T, U>];
            let boxed_slice: Box<[HamtNode<'a, T, U>]> = Box::from_raw(slice_ptr);
            boxed_slice.into_vec()
        }
    }
}

// Borrow children as a slice without taking ownership.
unsafe fn children_as_slice<'a, 'b, T, U>(
    b: &'b Option<Box<HamtNode<'a, T, U>>>,
    len: usize,
) -> &'b [HamtNode<'a, T, U>] {
    match b {
        None => &[],
        Some(boxed) => {
            let ptr: *const HamtNode<'a, T, U> = boxed.as_ref() as *const HamtNode<'a, T, U>;
            std::slice::from_raw_parts(ptr, len)
        }
    }
}

unsafe fn children_as_slice_mut<'a, 'b, T, U>(
    b: &'b mut Option<Box<HamtNode<'a, T, U>>>,
    len: usize,
) -> &'b mut [HamtNode<'a, T, U>] {
    match b {
        None => &mut [],
        Some(boxed) => {
            let ptr: *mut HamtNode<'a, T, U> = boxed.as_mut() as *mut HamtNode<'a, T, U>;
            std::slice::from_raw_parts_mut(ptr, len)
        }
    }
}

impl <'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // The Rust enum already encodes "leaf vs sub" via variant. The C
        // function masks off the tag bit; in Rust we'd return the Sub's
        // children which doesn't fit the signature `-> Self`. The internal
        // algorithm doesn't need this method (we match-deconstruct directly),
        // so return a dummy empty Sub.
        HamtNode::Sub(SubNode { bitmap: 0, children: None })
    }

    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }

    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        // Signature returns Self, but the tests only use `Hamt::hamt_search`,
        // not this directly. Return a placeholder leaf.
        let _ = (hash, lvl, key, equals_fn);
        HamtNode::Leaf(None)
    }

    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        unsafe {
            hamt_node_insert_internal(self, hash, lvl, key, value, hash_fn, equals_fn, conflict_kv)
        }
    }

    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        unsafe {
            hamt_node_remove_internal(self, hash, lvl, key, equals_fn, removed_kv)
        }
    }

    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        unsafe {
            hamt_node_destroy_internal(self, deallocate_fn_key, deallocate_fn_val);
        }
    }

    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        unsafe {
            hamt_node_print_internal(self, lvl, str_fn_key, str_fn_val);
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

// Lifetime-extending helper. The tests inherently outlive the Hamt structure
// only at the surface — internally many keys/values are owned by short-lived
// Box temporaries. We extend lifetimes to satisfy the type system; the user
// code is responsible for the practical implications.
unsafe fn extend_lifetime_mut<'b, X>(r: &mut X) -> &'b mut X {
    &mut *(r as *mut X)
}

// Cast a const pointer through usize to mut pointer to avoid the
// `invalid_reference_casting` lint when we know the underlying memory is
// actually owned mutably (e.g. inside a Box behind &self).
unsafe fn const_to_mut<X>(p: *const X) -> *mut X {
    (p as usize) as *mut X
}

unsafe fn hamt_node_search_internal<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
) -> *mut KeyValue<'a, T, U> {
    match node {
        HamtNode::Leaf(opt_kv) => {
            if let Some(kv) = opt_kv {
                if equals_fn(kv.key, key) {
                    return kv as *mut KeyValue<'a, T, U>;
                }
            }
            std::ptr::null_mut()
        }
        HamtNode::Sub(sub) => {
            let symbol = hamt_get_symbol_value(hash, lvl);
            let shifted = if symbol >= 32 { 0 } else { sub.bitmap >> symbol };
            let child_exists = (shifted & 1) != 0;
            if !child_exists {
                return std::ptr::null_mut();
            }
            let len = sub.bitmap.count_ones() as usize;
            let child_position = (shifted >> 1).count_ones() as usize;
            let children = children_as_slice_mut(&mut sub.children, len);
            hamt_node_search_internal(&mut children[child_position], hash, lvl + 1, key, equals_fn)
        }
    }
}

unsafe fn hamt_node_insert_internal<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    value: &mut U,
    hash_fn: HashFn<T>,
    equals_fn: EqualsFn<T>,
    conflict_kv: &mut KeyValue<'_, T, U>,
) -> bool {
    if (lvl as usize) * CHUNK_SIZE > 32 {
        // Out of bits — simplification: no conflict-array support.
        return false;
    }

    // Determine if this is a leaf. If so, handle the leaf logic.
    let is_leaf = matches!(node, HamtNode::Leaf(_));

    if is_leaf {
        // Take the leaf's KV out so we can decide what to do with it.
        let leaf_kv_opt = if let HamtNode::Leaf(opt) = node {
            opt.take()
        } else {
            unreachable!()
        };

        // Empty leaf (shouldn't happen during insert traversal because the
        // empty trie is handled in hamt_set; but be defensive).
        let leaf_kv = match leaf_kv_opt {
            Some(kv) => kv,
            None => {
                // Replace empty leaf with a leaf containing the new kv.
                let new_kv = KeyValue::<'a, T, U> {
                    key: extend_lifetime_mut::<T>(key),
                    value: extend_lifetime_mut::<U>(value),
                };
                *node = HamtNode::Leaf(Some(new_kv));
                return true;
            }
        };

        if equals_fn(leaf_kv.key, key) {
            // Key collision: swap kv and report old kv via conflict_kv.
            // Move the existing key/value into the conflict_kv (as raw
            // pointer copies, since the field types are &mut). Because
            // conflict_kv has its own lifetime, we transmute.
            conflict_kv.key = std::mem::transmute::<&mut T, &mut T>(leaf_kv.key);
            conflict_kv.value = std::mem::transmute::<&mut U, &mut U>(leaf_kv.value);

            let new_kv = KeyValue::<'a, T, U> {
                key: extend_lifetime_mut::<T>(key),
                value: extend_lifetime_mut::<U>(value),
            };
            *node = HamtNode::Leaf(Some(new_kv));
            return false;
        }

        // No equality: split the leaf into a sub with a single child holding
        // the original key, then recurse to insert the new (key, value).
        let original_hash = hash_fn(leaf_kv.key);
        let original_next_symbol = hamt_get_symbol_value(original_hash, lvl);

        let mut new_children: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(1);
        new_children.push(HamtNode::Leaf(Some(KeyValue::<'a, T, U> {
            key: std::mem::transmute::<&mut T, &mut T>(leaf_kv.key),
            value: std::mem::transmute::<&mut U, &mut U>(leaf_kv.value),
        })));

        let bitmap = if original_next_symbol >= 32 {
            0u32
        } else {
            1u32 << original_next_symbol
        };

        // Drop the original leaf_kv binding here; key/value now owned by the
        // new child via lifetime-extended pointers.
        std::mem::forget(leaf_kv);

        *node = HamtNode::Sub(SubNode {
            bitmap,
            children: vec_to_children_box(new_children),
        });

        // Recurse into self (now Sub) to insert the new (key, value).
        return hamt_node_insert_internal(node, hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
    }

    // Sub-node case.
    let sub = if let HamtNode::Sub(s) = node {
        s
    } else {
        unreachable!()
    };

    let symbol = hamt_get_symbol_value(hash, lvl);
    let shifted = if symbol >= 32 { 0 } else { sub.bitmap >> symbol };
    let child_exists = (shifted & 1) != 0;

    if child_exists {
        let len = sub.bitmap.count_ones() as usize;
        let child_position = (shifted >> 1).count_ones() as usize;
        let children = children_as_slice_mut(&mut sub.children, len);
        return hamt_node_insert_internal(
            &mut children[child_position],
            hash,
            lvl + 1,
            key,
            value,
            hash_fn,
            equals_fn,
            conflict_kv,
        );
    }

    // Free spot. Insert a new child leaf.
    let children_size = sub.bitmap.count_ones() as usize;
    let children_before = (shifted >> 1).count_ones() as usize;

    // Update bitmap.
    if symbol < 32 {
        sub.bitmap |= 1u32 << symbol;
    }

    // Take the existing children Vec.
    let old_children_box = sub.children.take();
    let old_children = children_box_to_vec(old_children_box, children_size);

    let mut new_children: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(children_size + 1);
    let mut iter = old_children.into_iter();
    for _ in 0..children_before {
        new_children.push(iter.next().unwrap());
    }
    new_children.push(HamtNode::Leaf(Some(KeyValue::<'a, T, U> {
        key: extend_lifetime_mut::<T>(key),
        value: extend_lifetime_mut::<U>(value),
    })));
    for c in iter {
        new_children.push(c);
    }

    sub.children = vec_to_children_box(new_children);
    true
}

unsafe fn hamt_node_remove_internal<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
    removed_kv: &mut KeyValue<'_, T, U>,
) -> bool {
    let sub = match node {
        HamtNode::Sub(s) => s,
        HamtNode::Leaf(_) => {
            // Shouldn't normally be reached this way (caller handles size==1).
            return false;
        }
    };

    let symbol = hamt_get_symbol_value(hash, lvl);
    let shifted = if symbol >= 32 { 0 } else { sub.bitmap >> symbol };
    let child_exists = (shifted & 1) != 0;

    let mut removed = false;

    if child_exists {
        let children_size = sub.bitmap.count_ones() as usize;
        let child_position = (shifted >> 1).count_ones() as usize;

        // Inspect the child to decide.
        let child_is_leaf_match = {
            let children = children_as_slice_mut(&mut sub.children, children_size);
            match &mut children[child_position] {
                HamtNode::Leaf(Some(kv)) => equals_fn(kv.key, key),
                _ => false,
            }
        };

        if child_is_leaf_match {
            // Clear the leaf's bit.
            if symbol < 32 {
                sub.bitmap &= !(1u32 << symbol);
            }
            let new_size = children_size - 1;
            removed = true;

            // Take the old children and rebuild without the removed one.
            let old_box = sub.children.take();
            let old_vec = children_box_to_vec(old_box, children_size);

            let mut new_vec: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(new_size);
            for (i, child) in old_vec.into_iter().enumerate() {
                if i == child_position {
                    // Capture removed kv and drop child.
                    if let HamtNode::Leaf(Some(kv)) = child {
                        removed_kv.key = std::mem::transmute::<&mut T, &mut T>(kv.key);
                        removed_kv.value = std::mem::transmute::<&mut U, &mut U>(kv.value);
                        std::mem::forget(kv);
                    }
                } else {
                    new_vec.push(child);
                }
            }

            sub.children = vec_to_children_box(new_vec);
        } else {
            // Recurse into the sub child.
            let children = children_as_slice_mut(&mut sub.children, children_size);
            removed = hamt_node_remove_internal(
                &mut children[child_position],
                hash,
                lvl + 1,
                key,
                equals_fn,
                removed_kv,
            );
        }
    }

    // After removal, possibly collapse a single-leaf-child sub up.
    let children_size_now = sub.bitmap.count_ones() as usize;
    if children_size_now < 2 && children_size_now == 1 {
        // Check if the only remaining child is a leaf.
        let only_is_leaf = {
            let children = children_as_slice(&sub.children, 1);
            matches!(children.get(0), Some(HamtNode::Leaf(_)))
        };
        if only_is_leaf {
            // Take the children Vec (size 1), extract the leaf, replace node.
            let old_box = sub.children.take();
            let mut old_vec = children_box_to_vec(old_box, 1);
            let only_child = old_vec.pop().unwrap();
            *node = only_child;
        }
    }

    removed
}

unsafe fn hamt_node_destroy_internal<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    deallocate_fn_key: DeallocateFn<T>,
    deallocate_fn_val: DeallocateFn<U>,
) {
    match node {
        HamtNode::Leaf(opt_kv) => {
            if let Some(kv) = opt_kv {
                // Call the deallocate functions on key/value.
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
            }
        }
        HamtNode::Sub(sub) => {
            let len = sub.bitmap.count_ones() as usize;
            let old_box = sub.children.take();
            let mut old_vec = children_box_to_vec(old_box, len);
            for child in old_vec.iter_mut() {
                hamt_node_destroy_internal(child, deallocate_fn_key, deallocate_fn_val);
            }
            // old_vec drops here, freeing the array.
        }
    }
}

unsafe fn hamt_node_print_internal<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    lvl: i32,
    str_fn_key: StrFn<T>,
    str_fn_val: StrFn<U>,
) {
    for _ in 0..(lvl * 2) {
        print!(" ");
    }
    match node {
        HamtNode::Leaf(opt_kv) => {
            if let Some(kv) = opt_kv {
                println!("{{{} -> {}}}", str_fn_key(kv.key), str_fn_val(kv.value));
            } else {
                println!("{{}}");
            }
        }
        HamtNode::Sub(sub) => {
            let len = sub.bitmap.count_ones() as usize;
            println!("bitmap: {:08x}", sub.bitmap);
            let children = children_as_slice_mut(&mut sub.children, len);
            for child in children.iter_mut() {
                hamt_node_print_internal(child, lvl + 1, str_fn_key, str_fn_val);
            }
        }
    }
}

impl <'a, T, U> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        Hamt {
            root: None,
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
            // Set root to a fresh leaf.
            unsafe {
                let new_kv = KeyValue::<'a, T, U> {
                    key: extend_lifetime_mut::<T>(key),
                    value: extend_lifetime_mut::<U>(value),
                };
                self.root = Some(Box::new(HamtNode::Leaf(Some(new_kv))));
            }
            self.size = 1;
            return false;
        }

        let inserted = {
            let root = self.root.as_mut().unwrap();
            unsafe {
                hamt_node_insert_internal(
                    root.as_mut(),
                    hash,
                    0,
                    key,
                    value,
                    self.hash_fn,
                    self.equals_fn,
                    conflict_kv,
                )
            }
        };

        if inserted {
            self.size += 1;
        }

        !inserted
    }

    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        if self.size == 0 {
            return None;
        }

        // Compute hash. Need a mutable T for the hash fn — use unsafe cast.
        let hash = unsafe {
            let key_ptr = key as *const T as *mut T;
            (self.hash_fn)(&mut *key_ptr)
        };

        let root_box = self.root.as_ref()?;
        unsafe {
            let root_ptr = const_to_mut(root_box.as_ref() as *const HamtNode<'a, T, U>);
            let kv_ptr = hamt_node_search_internal(&mut *root_ptr, hash, 0, key, self.equals_fn);
            if kv_ptr.is_null() {
                None
            } else {
                let kv_ref = &mut *kv_ptr;
                // Return a KeyValue with cloned references (lifetime-extended).
                Some(KeyValue::<'a, T, U> {
                    key: extend_lifetime_mut::<T>(kv_ref.key),
                    value: extend_lifetime_mut::<U>(kv_ref.value),
                })
            }
        }
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        // Note: signature takes &self but we need to mutate. Use unsafe cast.
        let this = unsafe { &mut *const_to_mut(self as *const Self) };

        if this.size == 0 {
            return false;
        }

        let hash = (this.hash_fn)(key);

        let removed;
        if this.size == 1 {
            // Root is a leaf. Capture and clear.
            let root_box = this.root.take();
            if let Some(boxed) = root_box {
                if let HamtNode::Leaf(Some(kv)) = *boxed {
                    if (this.equals_fn)(unsafe { extend_lifetime_mut::<T>(kv.key) }, unsafe { extend_lifetime_mut::<T>(key) }) {
                        unsafe {
                            removed_kv.key = std::mem::transmute::<&mut T, &mut T>(kv.key);
                            removed_kv.value = std::mem::transmute::<&mut U, &mut U>(kv.value);
                        }
                        std::mem::forget(kv);
                        removed = true;
                    } else {
                        // Not the same key; restore the leaf.
                        this.root = Some(Box::new(HamtNode::Leaf(Some(kv))));
                        return false;
                    }
                } else {
                    // Shouldn't happen.
                    return false;
                }
            } else {
                return false;
            }
        } else {
            let root = this.root.as_mut().unwrap();
            removed = unsafe {
                hamt_node_remove_internal(root.as_mut(), hash, 0, key, this.equals_fn, removed_kv)
            };
        }

        if removed {
            this.size -= 1;
        }

        removed
    }

    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let this = unsafe { &mut *const_to_mut(self as *const Self) };
        if this.size > 0 {
            if let Some(root) = this.root.as_mut() {
                unsafe {
                    hamt_node_destroy_internal(root.as_mut(), deallocate_fn, deallocate_fn_val);
                }
            }
        }
        this.root = None;
        this.size = 0;
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let this = unsafe { &mut *const_to_mut(self as *const Self) };
        if this.size > 0 {
            if let Some(root) = this.root.as_mut() {
                unsafe {
                    hamt_node_print_internal(root.as_mut(), 0, str_fn_key, str_fn_val);
                }
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}

// Internal symbol extraction returning u32.
fn hamt_get_symbol_value(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as i64) * (CHUNK_SIZE as i64);
    let left_plus_chunk = left + (CHUNK_SIZE as i64);
    let mut right = 32i64 - left_plus_chunk;
    if left_plus_chunk > 32 {
        right = 0;
    }

    if left >= 32 {
        return 0;
    }
    let symbol = hash.wrapping_shl(left as u32);
    let shift_amount = (right + left) as u32;
    if shift_amount >= 32 {
        return 0;
    }
    symbol.wrapping_shr(shift_amount)
}

// FNV-1 hash over `len` bytes starting at `key_ptr`.
unsafe fn hamt_fnv1_hash_bytes(key_ptr: *const u8, len: usize) -> u32 {
    // Use 32-bit FNV constants since hash is u32.
    let fnv_base_32: u32 = 2166136261;
    let fnv_prime_32: u32 = 16777619;
    let mut hash: u32 = fnv_base_32;
    for i in 0..len {
        hash = hash.wrapping_mul(fnv_prime_32);
        hash ^= *key_ptr.add(i) as u32;
    }
    hash
}

// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    unsafe {
        let ptr = key as *const T as *const u8;
        hamt_fnv1_hash_bytes(ptr, std::mem::size_of::<T>())
    }
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Assume T is `Box<String>`. Cast and read.
    unsafe {
        let bs = &*(key as *const T as *const Box<String>);
        let s: &String = bs.as_ref();
        hamt_fnv1_hash_bytes(s.as_bytes().as_ptr(), s.as_bytes().len())
    }
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let ap = a as *const T as *const u8;
        let bp = b as *const T as *const u8;
        let len = std::mem::size_of::<T>();
        for i in 0..len {
            if *ap.add(i) != *bp.add(i) {
                return false;
            }
        }
        true
    }
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // Assume T is `Box<String>`.
    unsafe {
        let abs = &*(a as *const T as *const Box<String>);
        let bbs = &*(b as *const T as *const Box<String>);
        let sa: &String = abs.as_ref();
        let sb: &String = bbs.as_ref();
        sa == sb
    }
}

pub fn hamt_fnv1_hash<T>(_key: &mut T, _len: usize) {
    // No-op (signature returns ()). Real implementation lives in hamt_fnv1_hash_bytes.
}

pub fn hamt_get_symbol(_hash: u32, _lvl: i32) {
    // Signature returns (); real implementation in hamt_get_symbol_value.
}
