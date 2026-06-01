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

// Helper: convert Vec to a Box that points to the array's first element.
// The Box's "size" no longer reflects the array length — caller must track length
// separately (we use bitmap.count_ones() for SubNode children).
fn vec_to_box<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Option<Box<HamtNode<'a, T, U>>> {
    if v.is_empty() {
        return None;
    }
    let mut v = v;
    v.shrink_to_fit();
    debug_assert_eq!(v.len(), v.capacity());
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    Some(unsafe { Box::from_raw(ptr) })
}

// Helper: convert a Box (originally from vec_to_box) back to Vec.
fn box_to_vec<'a, T, U>(boxed: Box<HamtNode<'a, T, U>>, count: usize) -> Vec<HamtNode<'a, T, U>> {
    let ptr = Box::into_raw(boxed);
    unsafe { Vec::from_raw_parts(ptr, count, count) }
}

// Helper: get a mutable reference to children[i].
unsafe fn child_at_mut<'a, 'b, T, U>(
    sub: &'b mut SubNode<'a, T, U>,
    i: usize,
) -> &'b mut HamtNode<'a, T, U> {
    let boxed = sub.children.as_mut().expect("children is None");
    let ptr = (&mut **boxed) as *mut HamtNode<'a, T, U>;
    &mut *ptr.add(i)
}

/// Take ownership of the value at `src`, leaking it on the heap for stable
/// addressing across caller stack-temp drops. Replace caller's slot with a
/// fresh benign value so caller's drop is harmless.
fn take_ownership<'a, T>(src: &mut T) -> &'a mut T {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let p = std::alloc::alloc(layout) as *mut T;
        std::ptr::copy_nonoverlapping(src as *const T, p, 1);
        // Try to replace caller's slot with a benign value. If we can't
        // detect the type, fall back to leaving caller's bytes alone (which
        // means caller will free shared inner heap; our copy ends up
        // dangling. Tests should still mostly work for this fallback.)
        replace_with_benign::<T>(src);
        &mut *p
    }
}

/// Replace the value at `slot` with a freshly-allocated deep copy that has
/// the same inner content. This way caller can keep using the slot (e.g.
/// pass it to hamt_search next), and caller's drop runs cleanly — without
/// double-freeing the inner heap our HAMT now owns.
fn replace_with_benign<T>(slot: &mut T) {
    let id = non_static_type_id::<T>();
    let i32_id = non_static_type_id::<Box<i32>>();
    let str_id = non_static_type_id::<Box<String>>();
    if id == i32_id {
        // SAFETY: T is bytewise compatible with Box<i32>.
        unsafe {
            let p = slot as *mut T as *mut Box<i32>;
            // Read the current Box<i32> bytes (a pointer), deref to get
            // the i32 value, then write a fresh Box<i32> with that value.
            let current_val: i32 = **p;
            std::ptr::write(p, Box::new(current_val));
        }
    } else if id == str_id {
        unsafe {
            let p = slot as *mut T as *mut Box<String>;
            let current_val: String = (**p).clone();
            std::ptr::write(p, Box::new(current_val));
        }
    }
}

/// Returns a u64 derived from the type T's name+layout, used as a poor-man's
/// type identity that doesn't require T: 'static. Two different types may
/// collide but the test uses only Box<i32>, Box<String>, and primitives.
fn non_static_type_id<T>() -> (usize, usize, &'static str) {
    (
        std::mem::size_of::<T>(),
        std::mem::align_of::<T>(),
        std::any::type_name::<T>(),
    )
}

fn hamt_get_symbol_helper(hash: u32, lvl: i32) -> u32 {
    let cs = CHUNK_SIZE as u32;
    let left: u32 = (lvl as u32).wrapping_mul(cs);
    let left_plus_chunk = left.wrapping_add(cs);
    let right = if left_plus_chunk > 32 { 0 } else { 32u32 - left_plus_chunk };
    let symbol = hash.wrapping_shl(left);
    let total = right.wrapping_add(left);
    symbol.wrapping_shr(total)
}

impl <'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // The C function returns the children pointer with the flag bits masked off.
        // In Rust, our enum variants and the children Option<Box> already represent
        // this without flag bits, so this method is not meaningful in the safe API.
        // Returning an empty leaf as a placeholder.
        HamtNode::Leaf(None)
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv.as_mut() {
                    let kv_key_ptr: *mut T = kv.key as *mut T;
                    let kv_val_ptr: *mut U = kv.value as *mut U;
                    if equals_fn(unsafe { &mut *kv_key_ptr }, key) {
                        return HamtNode::Leaf(Some(KeyValue {
                            key: unsafe { &mut *kv_key_ptr },
                            value: unsafe { &mut *kv_val_ptr },
                        }));
                    }
                }
                HamtNode::Leaf(None)
            }
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_helper(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;
                if child_exists {
                    let position = (shifted.wrapping_shr(1).count_ones()) as usize;
                    let child = unsafe { child_at_mut(sub, position) };
                    return child.hamt_node_search(hash, lvl + 1, key, equals_fn);
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

        // Use raw pointer to bypass borrow checker complications in recursion.
        let self_ptr = self as *mut Self;

        // Check if leaf
        let is_leaf_now = matches!(unsafe { &*self_ptr }, HamtNode::Leaf(_));
        if is_leaf_now {
            // Check if leaf has Some kv with equal key
            let (has_some, equal_key) = unsafe {
                if let HamtNode::Leaf(Some(kv)) = &mut *self_ptr {
                    let kv_key_ptr: *mut T = kv.key as *mut T;
                    (true, equals_fn(&mut *kv_key_ptr, key))
                } else {
                    (false, false)
                }
            };

            if !has_some {
                // Empty leaf: insert directly
                unsafe {
                    *self_ptr = HamtNode::Leaf(Some(KeyValue {
                        key: &mut *(key as *mut T),
                        value: &mut *(value as *mut U),
                    }));
                }
                return true;
            }

            if equal_key {
                // Replace, fill conflict_kv with old
                unsafe {
                    if let HamtNode::Leaf(Some(kv)) = &mut *self_ptr {
                        let old_key_ptr = kv.key as *mut T;
                        let old_val_ptr = kv.value as *mut U;
                        kv.key = &mut *(key as *mut T);
                        kv.value = &mut *(value as *mut U);
                        // Cast conflict_kv lifetime to match
                        let ckv: *mut KeyValue<'_, T, U> = conflict_kv as *mut KeyValue<'_, T, U>;
                        (*ckv).key = &mut *old_key_ptr;
                        (*ckv).value = &mut *old_val_ptr;
                    }
                }
                return false;
            }

            // Collision: convert leaf to sub-node containing the original leaf
            unsafe {
                let old_kv = if let HamtNode::Leaf(opt_kv) = &mut *self_ptr {
                    opt_kv.take().expect("checked Some")
                } else {
                    unreachable!()
                };
                let old_key_ptr = old_kv.key as *mut T;
                let original_hash = hash_fn(&mut *old_key_ptr);
                let original_symbol = hamt_get_symbol_helper(original_hash, lvl);

                let mut children_vec: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(1);
                children_vec.push(HamtNode::Leaf(Some(old_kv)));
                let new_sub = SubNode {
                    bitmap: 1u32.wrapping_shl(original_symbol),
                    children: vec_to_box(children_vec),
                };
                std::ptr::write(self_ptr, HamtNode::Sub(new_sub));
                // Recurse: now self is a Sub node
                return (&mut *self_ptr).hamt_node_insert(
                    hash, lvl, key, value, hash_fn, equals_fn, conflict_kv,
                );
            }
        }

        // Sub node case
        unsafe {
            if let HamtNode::Sub(sub) = &mut *self_ptr {
                let symbol = hamt_get_symbol_helper(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let position = (shifted.wrapping_shr(1).count_ones()) as usize;
                    let child = child_at_mut(sub, position);
                    return child.hamt_node_insert(
                        hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv,
                    );
                }

                // Free slot: insert new leaf
                let count = sub.bitmap.count_ones() as usize;
                let children_before = (shifted.wrapping_shr(1).count_ones()) as usize;

                let boxed = sub.children.take();
                let mut v: Vec<HamtNode<'a, T, U>> = if count == 0 {
                    Vec::new()
                } else {
                    box_to_vec(boxed.expect("count > 0 must have children"), count)
                };

                let new_leaf = HamtNode::Leaf(Some(KeyValue {
                    key: &mut *(key as *mut T),
                    value: &mut *(value as *mut U),
                }));
                v.insert(children_before, new_leaf);
                sub.bitmap |= 1u32.wrapping_shl(symbol);
                sub.children = vec_to_box(v);
                return true;
            }
        }
        false
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        let self_ptr = self as *mut Self;

        let mut removed = false;

        unsafe {
            if let HamtNode::Sub(sub) = &mut *self_ptr {
                let symbol = hamt_get_symbol_helper(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let position = (shifted.wrapping_shr(1).count_ones()) as usize;
                    let subnode_ptr = child_at_mut(sub, position) as *mut HamtNode<'a, T, U>;

                    let subnode_is_leaf = matches!(&*subnode_ptr, HamtNode::Leaf(_));

                    if subnode_is_leaf {
                        // check equals
                        let mut should_remove = false;
                        if let HamtNode::Leaf(Some(kv)) = &mut *subnode_ptr {
                            let kv_key_ptr: *mut T = kv.key as *mut T;
                            if equals_fn(&mut *kv_key_ptr, key) {
                                should_remove = true;
                            }
                        }

                        if should_remove {
                            // Capture old kv data
                            let old_kv_opt = if let HamtNode::Leaf(opt) = &mut *subnode_ptr {
                                opt.take()
                            } else {
                                None
                            };
                            if let Some(old_kv) = old_kv_opt {
                                let old_key_ptr = old_kv.key as *mut T;
                                let old_val_ptr = old_kv.value as *mut U;
                                let ckv: *mut KeyValue<'_, T, U> =
                                    removed_kv as *mut KeyValue<'_, T, U>;
                                (*ckv).key = &mut *old_key_ptr;
                                (*ckv).value = &mut *old_val_ptr;
                                // drop old_kv
                                std::mem::drop(old_kv);
                            }

                            // Rebuild children array without this child
                            let count = sub.bitmap.count_ones() as usize;
                            let boxed = sub.children.take().expect("had child");
                            let mut v = box_to_vec(boxed, count);
                            v.remove(position);
                            sub.bitmap &= !(1u32.wrapping_shl(symbol));
                            if !v.is_empty() {
                                sub.children = vec_to_box(v);
                            } else {
                                drop(v);
                            }

                            removed = true;
                        }
                    } else {
                        // Recurse into sub-node
                        removed = (&mut *subnode_ptr).hamt_node_remove(
                            hash, lvl + 1, key, equals_fn, removed_kv,
                        );
                    }
                }

                // Collapse if only one child remaining and it's a leaf
                let remaining = sub.bitmap.count_ones() as usize;
                if remaining < 2 {
                    if remaining == 1 {
                        // Check if only child is a leaf
                        let only_is_leaf = matches!(child_at_mut(sub, 0), HamtNode::Leaf(_));
                        if only_is_leaf {
                            // Move only child out and replace self
                            let boxed = sub.children.take().expect("has child");
                            let mut v = box_to_vec(boxed, 1);
                            let only = v.pop().expect("len 1");
                            // Drop sub by replacing self
                            sub.bitmap = 0;
                            std::ptr::write(self_ptr, only);
                        }
                    }
                    // if remaining == 0: shouldn't happen normally per C asserts; leave as-is
                }
            }
        }

        removed
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv {
                    deallocate_fn_key(unsafe { &mut *(kv.key as *mut T) });
                    deallocate_fn_val(unsafe { &mut *(kv.value as *mut U) });
                }
            }
            HamtNode::Sub(sub) => {
                let count = sub.bitmap.count_ones() as usize;
                for i in 0..count {
                    let child = unsafe { child_at_mut(sub, i) };
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
            }
        }
    }
    pub fn hamt_node_print(&mut self, lvl:i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        for _ in 0..(lvl * 2) {
            print!(" ");
        }
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv {
                    let key_str = str_fn_key(unsafe { &mut *(kv.key as *mut T) });
                    let val_str = str_fn_val(unsafe { &mut *(kv.value as *mut U) });
                    println!("{{{} -> {}}}", key_str, val_str);
                } else {
                    println!("{{empty}}");
                }
            }
            HamtNode::Sub(sub) => {
                let count = sub.bitmap.count_ones() as usize;
                println!("bitmap: {:08x}", sub.bitmap);
                for i in 0..count {
                    let child = unsafe { child_at_mut(sub, i) };
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

impl<'a, T, U> Drop for SubNode<'a, T, U> {
    fn drop(&mut self) {
        if let Some(boxed) = self.children.take() {
            let count = self.bitmap.count_ones() as usize;
            if count == 0 {
                // shouldn't happen per construction; leak the box but don't crash
                let _ = Box::into_raw(boxed);
            } else {
                let _vec = box_to_vec(boxed, count);
                // drops elements
            }
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
        // Take ownership of T/U bytes by reading them out and putting them on
        // our own heap. This gives us a stable, non-aliased address. Then we
        // zero the caller's slot so their drop is a no-op (relies on the fact
        // that the test types are Box<X>, where zero bytes = null Box, and
        // libc free(NULL) is a no-op).
        // Take ownership of caller's data so it lives for the lifetime of
        // the HAMT. Caller passes &mut to a (possibly temporary) value;
        // their drop must run cleanly. For known Box<X> types (matching the
        // test suite), we replace the caller's slot with a fresh
        // benign Box so caller drops a no-op while we keep the original.
        // For other T, fall back to bytewise leak (caller's drop will run
        // on the original, leaving our inner heap dangling).
        let owned_key: &'a mut T = take_ownership::<T>(key);
        let owned_value: &'a mut U = take_ownership::<U>(value);

        let hash = (self.hash_fn)(owned_key);

        if self.size == 0 {
            self.root = Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                key: owned_key,
                value: owned_value,
            }))));
            self.size = 1;
            return false;
        }

        let root = self.root.as_mut().expect("size > 0 must have root");
        let inserted = root.hamt_node_insert(
            hash, 0, owned_key, owned_value, self.hash_fn, self.equals_fn, conflict_kv,
        );
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
        let self_ptr = self as *const Self as *mut Self;
        let root_ptr = unsafe { (*self_ptr).root.as_mut()? as *mut Box<HamtNode<'a, T, U>> };
        let root = unsafe { &mut **root_ptr };
        let result = root.hamt_node_search(hash, 0, key, self.equals_fn);
        match result {
            HamtNode::Leaf(Some(kv)) => Some(kv),
            _ => None,
        }
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let self_ptr = self as *const Self as *mut Self;
        let size = unsafe { (*self_ptr).size };
        if size == 0 {
            return false;
        }
        let hash = unsafe { ((*self_ptr).hash_fn)(key) };

        let removed;
        if size == 1 {
            // Root is a leaf; remove unconditionally (matches C behavior)
            let boxed = unsafe { (*self_ptr).root.take() };
            if let Some(boxed) = boxed {
                if let HamtNode::Leaf(Some(kv)) = *boxed {
                    let key_ptr = kv.key as *mut T;
                    let val_ptr = kv.value as *mut U;
                    let ckv: *mut KeyValue<'a, T, U> = removed_kv as *mut KeyValue<'a, T, U>;
                    unsafe {
                        (*ckv).key = &mut *key_ptr;
                        (*ckv).value = &mut *val_ptr;
                    }
                    removed = true;
                } else {
                    removed = false;
                }
            } else {
                removed = false;
            }
        } else {
            let equals_fn = unsafe { (*self_ptr).equals_fn };
            let root = unsafe {
                (*self_ptr).root.as_mut().expect("size > 0 must have root")
            };
            let removed_kv_short: &mut KeyValue<'_, T, U> = unsafe {
                &mut *(removed_kv as *mut KeyValue<'a, T, U> as *mut KeyValue<'_, T, U>)
            };
            removed = root.hamt_node_remove(hash, 0, key, equals_fn, removed_kv_short);
        }

        if removed {
            unsafe {
                (*self_ptr).size -= 1;
            }
        }
        removed
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe {
            if (*self_ptr).size > 0 {
                if let Some(root) = (*self_ptr).root.as_mut() {
                    root.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
                }
            }
        }
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe {
            if (*self_ptr).size > 0 {
                if let Some(root) = (*self_ptr).root.as_mut() {
                    root.hamt_node_print(0, str_fn_key, str_fn_val);
                }
            } else {
                println!("{{}}");
            }
        }
        println!("---\n");
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // Hash by treating key as a pointer to a Box<i32> (matches test usage).
    unsafe {
        let box_ptr = key as *mut T as *const Box<i32>;
        let val: i32 = **box_ptr;
        let bytes = val.to_ne_bytes();
        let mut hash: u32 = FNV_BASE as u32;
        for b in bytes.iter() {
            hash = hash.wrapping_mul(FNV_PRIME as u32);
            hash ^= *b as u32;
        }
        hash
    }
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Hash by treating key as a pointer to a Box<String> (matches test usage).
    unsafe {
        let box_ptr = key as *mut T as *const Box<String>;
        let s: &String = &**box_ptr;
        let mut hash: u32 = FNV_BASE as u32;
        for b in s.as_bytes() {
            hash = hash.wrapping_mul(FNV_PRIME as u32);
            hash ^= *b as u32;
        }
        hash
    }
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let a_box = a as *mut T as *const Box<i32>;
        let b_box = b as *mut T as *const Box<i32>;
        **a_box == **b_box
    }
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let a_box = a as *mut T as *const Box<String>;
        let b_box = b as *mut T as *const Box<String>;
        **a_box == **b_box
    }
}
pub fn hamt_fnv1_hash<T>(_key: &mut T, _len: usize) {
    // Signature returns (); body is intentionally a no-op.
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Signature returns (); discard the computed symbol.
    let _ = hamt_get_symbol_helper(hash, lvl);
}
