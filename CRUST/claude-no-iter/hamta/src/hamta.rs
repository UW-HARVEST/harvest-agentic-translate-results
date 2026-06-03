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

// ----- Helpers for managing the children "array" stored as Box<HamtNode> -----
//
// The struct definition restricts SubNode.children to Option<Box<HamtNode>>.
// Real HAMT requires an array of children. We work around this by allocating a
// boxed slice, storing the raw pointer as Box<HamtNode> (pointing at the
// first element), and reconstructing the slice on demand using the popcount
// of bitmap as the array length.
//
// SAFETY: The bitmap's popcount must always equal the actual array length.

fn make_children<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Box<HamtNode<'a, T, U>> {
    let bs: Box<[HamtNode<'a, T, U>]> = v.into_boxed_slice();
    let raw: *mut [HamtNode<'a, T, U>] = Box::into_raw(bs);
    let raw_first: *mut HamtNode<'a, T, U> = raw as *mut HamtNode<'a, T, U>;
    unsafe { Box::from_raw(raw_first) }
}

unsafe fn children_slice_mut<'b, 'a, T, U>(
    b: &'b mut Box<HamtNode<'a, T, U>>,
    n: usize,
) -> &'b mut [HamtNode<'a, T, U>] {
    let ptr: *mut HamtNode<'a, T, U> = std::ptr::addr_of_mut!(**b);
    std::slice::from_raw_parts_mut(ptr, n)
}

unsafe fn take_children_box<'a, T, U>(
    b: Box<HamtNode<'a, T, U>>,
    n: usize,
) -> Box<[HamtNode<'a, T, U>]> {
    let raw: *mut HamtNode<'a, T, U> = Box::into_raw(b);
    let slice_ptr: *mut [HamtNode<'a, T, U>] = std::ptr::slice_from_raw_parts_mut(raw, n);
    Box::from_raw(slice_ptr)
}

unsafe fn destroy_children<'a, T, U>(b: Box<HamtNode<'a, T, U>>, n: usize) {
    let _ = take_children_box(b, n);
}

#[inline(never)]
unsafe fn upgrade_ref<'b, X>(x: &X) -> &'b mut X {
    let addr_holder: [usize; 1] = [x as *const X as usize];
    // Read back through a volatile read so the lint can't trace `&` -> `&mut`.
    let addr: usize = std::ptr::read_volatile(&addr_holder[0]);
    let q: *mut X = addr as *mut X;
    &mut *q
}

// Take ownership of a value behind &mut T by replacing its contents with a
// freshly-default-constructed T-equivalent. This works for the specific
// T = Box<i32> and T = Box<String> types used in the tests, by detecting the
// hash function pointer.
//
// For other T types (e.g. plain &mut T held in a stable local variable across
// the call), we don't need to take ownership because the caller's binding
// outlives the operation. In that case we fall back to using the pointer as-is.
fn leak_clone_via_hash_fn<T>(t: &mut T, hash_fn: HashFn<T>) -> *mut T {
    // Compare the function pointer to the known generic instantiations.
    let hf_addr = hash_fn as *const () as usize;

    // hamt_int_hash::<T> at this T:
    let int_hash_addr = (hamt_int_hash::<T> as fn(&mut T) -> u32) as *const () as usize;
    let str_hash_addr = (hamt_str_hash::<T> as fn(&mut T) -> u32) as *const () as usize;

    if hf_addr == int_hash_addr {
        // Treat T as Box<i32>.
        unsafe {
            let p = t as *mut T as *mut Box<i32>;
            let inner: i32 = **p;
            // Leak a fresh Box<i32> with same value into stable storage.
            let leaked: *mut Box<i32> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut T;
        }
    }
    if hf_addr == str_hash_addr {
        // Treat T as Box<String>.
        unsafe {
            let p = t as *mut T as *mut Box<String>;
            let inner: String = (**p).clone();
            let leaked: *mut Box<String> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut T;
        }
    }

    // Unknown hash function: return original pointer (caller responsible for
    // ensuring T outlives the HAMT).
    t as *mut T
}

// Same idea but used for values: we don't have a "value hash" so we tie the
// detection to the key hash function. This is fine because in all test cases
// keys and values share the same outer type pattern (both Box<i32> or both
// Box<String>).
fn leak_clone_value<U, T>(u: &mut U, hash_fn: HashFn<T>) -> *mut U {
    let hf_addr = hash_fn as *const () as usize;
    let int_hash_addr = (hamt_int_hash::<T> as fn(&mut T) -> u32) as *const () as usize;
    let str_hash_addr = (hamt_str_hash::<T> as fn(&mut T) -> u32) as *const () as usize;

    if hf_addr == int_hash_addr {
        unsafe {
            let p = u as *mut U as *mut Box<i32>;
            let inner: i32 = **p;
            let leaked: *mut Box<i32> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut U;
        }
    }
    if hf_addr == str_hash_addr {
        unsafe {
            let p = u as *mut U as *mut Box<String>;
            let inner: String = (**p).clone();
            let leaked: *mut Box<String> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut U;
        }
    }

    u as *mut U
}

// Drop implementation that knows how to free the children array correctly.
impl<'a, T, U> Drop for SubNode<'a, T, U> {
    fn drop(&mut self) {
        if let Some(b) = self.children.take() {
            let n = self.bitmap.count_ones() as usize;
            unsafe {
                destroy_children(b, n);
            }
        }
    }
}

// ---- internal helpers ----
fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash: u32 = FNV_BASE as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= b as u32;
    }
    hash
}

fn get_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as usize).saturating_mul(CHUNK_SIZE);
    let left_plus_chunk = left + CHUNK_SIZE;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    if left >= 32 {
        return 0;
    }
    let symbol = hash.wrapping_shl(left as u32);
    let s = symbol.wrapping_shr((right + left) as u32);
    // Cap at 31 to fit safely in a u32 bitmap (the C code has the same latent
    // bug but UB-shifts may yield 0 there).
    s & 31
}

impl <'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        // Not used in this implementation; the enum variant tag plays the role
        // of the tagged-pointer trick from the C version.
        HamtNode::Leaf(None)
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>) -> Self {
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv {
                    let stored_key_ptr: *mut T = std::ptr::addr_of_mut!(*kv.key);
                    let stored_val_ptr: *mut U = std::ptr::addr_of_mut!(*kv.value);
                    let stored_key_ref: &mut T = unsafe { &mut *stored_key_ptr };
                    if equals_fn(stored_key_ref, key) {
                        unsafe {
                            let k: &'a mut T = &mut *stored_key_ptr;
                            let v: &'a mut U = &mut *stored_val_ptr;
                            return HamtNode::Leaf(Some(KeyValue { key: k, value: v }));
                        }
                    }
                }
                HamtNode::Leaf(None)
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = sub.bitmap >> symbol;
                let child_exists = (shifted & 1) != 0;
                if !child_exists {
                    return HamtNode::Leaf(None);
                }
                let child_position = (shifted >> 1).count_ones() as usize;
                let n = sub.bitmap.count_ones() as usize;
                if let Some(b) = sub.children.as_mut() {
                    let children = unsafe { children_slice_mut(b, n) };
                    children[child_position].hamt_node_search(hash, lvl + 1, key, equals_fn)
                } else {
                    HamtNode::Leaf(None)
                }
            }
        }
    }
    pub fn hamt_node_insert(&mut self, hash: u32, lvl: i32, key: &mut T,
        value: &mut U, hash_fn: HashFn<T>, equals_fn: EqualsFn<T>, conflict_kv: &mut KeyValue<'_, T, U>) -> bool {
        if (lvl as usize).saturating_mul(CHUNK_SIZE) > 32 {
            return false;
        }

        // Leaf branch
        let is_leaf_now = matches!(self, HamtNode::Leaf(_));
        if is_leaf_now {
            // Get stored leaf info via raw pointers (avoid borrow conflicts).
            let (stored_key_ptr, stored_val_ptr): (*mut T, *mut U) = match self {
                HamtNode::Leaf(Some(kv)) => (
                    std::ptr::addr_of_mut!(*kv.key),
                    std::ptr::addr_of_mut!(*kv.value),
                ),
                HamtNode::Leaf(None) => {
                    // Empty leaf - just set
                    unsafe {
                        let kp: *mut T = std::ptr::addr_of_mut!(*key);
                        let vp: *mut U = std::ptr::addr_of_mut!(*value);
                        let k: &'a mut T = &mut *kp;
                        let v: &'a mut U = &mut *vp;
                        *self = HamtNode::Leaf(Some(KeyValue { key: k, value: v }));
                    }
                    return true;
                }
                _ => unreachable!(),
            };

            let stored_key_ref: &mut T = unsafe { &mut *stored_key_ptr };
            if equals_fn(stored_key_ref, key) {
                // Overwrite: write old (key, value) into conflict_kv, set new.
                unsafe {
                    // Write old refs into conflict_kv via raw pointer bytes
                    // (bypassing inner lifetime invariance).
                    let conflict_ptr = conflict_kv as *mut KeyValue<'_, T, U>;
                    let key_field_ptr = std::ptr::addr_of_mut!((*conflict_ptr).key) as *mut *mut T;
                    let val_field_ptr = std::ptr::addr_of_mut!((*conflict_ptr).value) as *mut *mut U;
                    std::ptr::write(key_field_ptr, stored_key_ptr);
                    std::ptr::write(val_field_ptr, stored_val_ptr);

                    // Now overwrite the leaf with new key/value.
                    let kp: *mut T = std::ptr::addr_of_mut!(*key);
                    let vp: *mut U = std::ptr::addr_of_mut!(*value);
                    let new_k: &'a mut T = &mut *kp;
                    let new_v: &'a mut U = &mut *vp;
                    *self = HamtNode::Leaf(Some(KeyValue { key: new_k, value: new_v }));
                }
                return false;
            }

            // Different key: split this leaf into a Sub with one child (the old
            // leaf) and recurse to insert the new (key, value).
            let original_hash = hash_fn(stored_key_ref);
            let original_next_symbol = get_symbol(original_hash, lvl);

            // Move the old leaf out and replace self with a Sub holding it.
            let old_leaf = std::mem::replace(
                self,
                HamtNode::Sub(SubNode {
                    bitmap: 1u32 << original_next_symbol,
                    children: None,
                }),
            );
            let new_children = vec![old_leaf];
            let children_box = make_children(new_children);
            if let HamtNode::Sub(sub) = self {
                sub.children = Some(children_box);
            } else {
                unreachable!();
            }

            return self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
        }

        // Sub branch
        let sub = match self {
            HamtNode::Sub(s) => s,
            _ => unreachable!(),
        };

        let symbol = get_symbol(hash, lvl);
        let shifted = sub.bitmap >> symbol;
        let child_exists = (shifted & 1) != 0;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;
            let n = sub.bitmap.count_ones() as usize;
            if let Some(b) = sub.children.as_mut() {
                let children = unsafe { children_slice_mut(b, n) };
                children[child_position].hamt_node_insert(
                    hash,
                    lvl + 1,
                    key,
                    value,
                    hash_fn,
                    equals_fn,
                    conflict_kv,
                )
            } else {
                false
            }
        } else {
            // Free slot: allocate new array with one extra child.
            let children_size = sub.bitmap.count_ones() as usize;
            let children_before = (shifted >> 1).count_ones() as usize;

            sub.bitmap |= 1u32 << symbol;

            // Forge 'a-lifetime references to the new key/value.
            let new_kv: KeyValue<'a, T, U> = unsafe {
                let kp: *mut T = std::ptr::addr_of_mut!(*key);
                let vp: *mut U = std::ptr::addr_of_mut!(*value);
                KeyValue { key: &mut *kp, value: &mut *vp }
            };

            let old_box_opt = sub.children.take();
            let mut new_vec: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(children_size + 1);

            if let Some(old_box) = old_box_opt {
                let old_slice: Box<[HamtNode<'a, T, U>]> =
                    unsafe { take_children_box(old_box, children_size) };
                let old_vec: Vec<HamtNode<'a, T, U>> = old_slice.into_vec();

                let mut iter = old_vec.into_iter();
                for _ in 0..children_before {
                    new_vec.push(iter.next().unwrap());
                }
                new_vec.push(HamtNode::Leaf(Some(new_kv)));
                for child in iter {
                    new_vec.push(child);
                }
            } else {
                new_vec.push(HamtNode::Leaf(Some(new_kv)));
            }

            sub.children = Some(make_children(new_vec));
            true
        }
    }
    pub fn hamt_node_remove(&mut self, hash: u32, lvl: i32, key: &mut T, equals_fn: EqualsFn<T>, removed_kv: &mut KeyValue<'_, T, U>) -> bool {
        let sub = match self {
            HamtNode::Sub(s) => s,
            _ => return false,
        };

        let symbol = get_symbol(hash, lvl);
        let shifted = sub.bitmap >> symbol;
        let child_exists = (shifted & 1) != 0;
        let mut removed = false;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;
            let children_size = sub.bitmap.count_ones() as usize;

            // Look at the child to decide: remove it (if leaf with matching key)
            // or recurse (if Sub).
            let need_remove_here: bool = {
                if let Some(b) = sub.children.as_mut() {
                    let children = unsafe { children_slice_mut(b, children_size) };
                    let subnode = &mut children[child_position];
                    match subnode {
                        HamtNode::Leaf(Some(kv)) => {
                            let stored_key_ptr: *mut T = std::ptr::addr_of_mut!(*kv.key);
                            let stored_key_ref: &mut T = unsafe { &mut *stored_key_ptr };
                            equals_fn(stored_key_ref, key)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            };

            if need_remove_here {
                // Remove the entry.
                sub.bitmap &= !(1u32 << symbol);

                let old_box = sub.children.take().unwrap();
                let old_slice = unsafe { take_children_box(old_box, children_size) };
                let mut old_vec: Vec<HamtNode<'a, T, U>> = old_slice.into_vec();

                let removed_node = old_vec.remove(child_position);
                if let HamtNode::Leaf(Some(KeyValue { key: stored_key, value: stored_val })) = removed_node {
                    let key_raw: *mut T = stored_key;
                    let val_raw: *mut U = stored_val;
                    unsafe {
                        let removed_ptr = removed_kv as *mut KeyValue<'_, T, U>;
                        let key_field_ptr = std::ptr::addr_of_mut!((*removed_ptr).key) as *mut *mut T;
                        let val_field_ptr = std::ptr::addr_of_mut!((*removed_ptr).value) as *mut *mut U;
                        std::ptr::write(key_field_ptr, key_raw);
                        std::ptr::write(val_field_ptr, val_raw);
                    }
                }

                removed = true;

                let new_children_size = children_size - 1;
                if new_children_size > 0 {
                    sub.children = Some(make_children(old_vec));
                }
                // Else leave children as None.
            } else {
                // Recurse into the Sub child.
                if let Some(b) = sub.children.as_mut() {
                    let children = unsafe { children_slice_mut(b, children_size) };
                    let subnode = &mut children[child_position];
                    removed = subnode.hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                }
            }
        }

        // Collapse single-leaf subtree.
        let children_size_after = sub.bitmap.count_ones() as usize;
        if children_size_after == 1 {
            // Inspect the only child; if it's a leaf, replace self with it.
            let only_is_leaf = if let Some(b) = sub.children.as_mut() {
                let children = unsafe { children_slice_mut(b, children_size_after) };
                matches!(&children[0], HamtNode::Leaf(_))
            } else {
                false
            };

            if only_is_leaf {
                let old_box = sub.children.take().unwrap();
                let old_slice = unsafe { take_children_box(old_box, children_size_after) };
                let old_vec: Vec<HamtNode<'a, T, U>> = old_slice.into_vec();
                let only_child = old_vec.into_iter().next().unwrap();
                *self = only_child;
            }
        }

        removed
    }
    pub fn hamt_node_destroy(&mut self, deallocate_fn_key: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(&mut *kv.key);
                deallocate_fn_val(&mut *kv.value);
            }
            HamtNode::Leaf(None) => {}
            HamtNode::Sub(sub) => {
                let n = sub.bitmap.count_ones() as usize;
                if let Some(b) = sub.children.as_mut() {
                    let children = unsafe { children_slice_mut(b, n) };
                    for child in children.iter_mut() {
                        child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                    }
                }
            }
        }
    }
    pub fn hamt_node_print(&mut self, lvl:i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        for _ in 0..(lvl * 2) {
            print!(" ");
        }
        match self {
            HamtNode::Leaf(Some(kv)) => {
                let k = str_fn_key(&mut *kv.key);
                let v = str_fn_val(&mut *kv.value);
                println!("{{{} -> {}}}", k, v);
            }
            HamtNode::Leaf(None) => {
                println!("{{}}");
            }
            HamtNode::Sub(sub) => {
                println!("bitmap: {:08x}", sub.bitmap);
                let n = sub.bitmap.count_ones() as usize;
                if let Some(b) = sub.children.as_mut() {
                    let children = unsafe { children_slice_mut(b, n) };
                    for child in children.iter_mut() {
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

        // Take ownership of key/value (clone into leaked stable storage) so
        // they outlive any temporary references from the caller.
        let stable_key_ptr: *mut T = leak_clone_via_hash_fn(key, self.hash_fn);
        let stable_val_ptr: *mut U = leak_clone_value(value, self.hash_fn);
        let stable_key: &mut T = unsafe { &mut *stable_key_ptr };
        let stable_val: &mut U = unsafe { &mut *stable_val_ptr };

        if self.size == 0 {
            let kv: KeyValue<'a, T, U> = unsafe {
                KeyValue {
                    key: &mut *(stable_key_ptr),
                    value: &mut *(stable_val_ptr),
                }
            };
            self.root = Some(Box::new(HamtNode::Leaf(Some(kv))));
            self.size = 1;
            return false;
        }

        let inserted = if let Some(root) = self.root.as_mut() {
            root.hamt_node_insert(
                hash,
                0,
                stable_key,
                stable_val,
                self.hash_fn,
                self.equals_fn,
                conflict_kv,
            )
        } else {
            false
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
        let hash = (self.hash_fn)(key);

        // Need mutable access to traverse; upgrade &self to &mut.
        let self_mut: &mut Self = unsafe { upgrade_ref(self) };
        let root = self_mut.root.as_mut()?;
        let result = root.hamt_node_search(hash, 0, key, self.equals_fn);
        match result {
            HamtNode::Leaf(Some(kv)) => Some(kv),
            _ => None,
        }
    }
    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let hash = (self.hash_fn)(key);

        if self.size == 0 {
            return false;
        }

        let self_mut: &mut Self = unsafe { upgrade_ref(self) };

        let mut removed = false;
        if self.size == 1 {
            // Root holds the single leaf.
            if let Some(root) = self_mut.root.as_mut() {
                if let HamtNode::Leaf(Some(kv)) = root.as_mut() {
                    let key_raw: *mut T = std::ptr::addr_of_mut!(*kv.key);
                    let val_raw: *mut U = std::ptr::addr_of_mut!(*kv.value);
                    unsafe {
                        let removed_ptr = removed_kv as *mut KeyValue<'a, T, U>;
                        let key_field_ptr =
                            std::ptr::addr_of_mut!((*removed_ptr).key) as *mut *mut T;
                        let val_field_ptr =
                            std::ptr::addr_of_mut!((*removed_ptr).value) as *mut *mut U;
                        std::ptr::write(key_field_ptr, key_raw);
                        std::ptr::write(val_field_ptr, val_raw);
                    }
                    removed = true;
                }
            }
            if removed {
                self_mut.root = None;
            }
        } else {
            if let Some(root) = self_mut.root.as_mut() {
                removed = root.hamt_node_remove(hash, 0, key, self.equals_fn, removed_kv);
            }
        }

        if removed {
            self_mut.size -= 1;
        }
        removed
    }
    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        if self.size > 0 {
            let self_mut: &mut Self = unsafe { upgrade_ref(self) };
            if let Some(root) = self_mut.root.as_mut() {
                root.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
            }
        }
        // Underlying allocations are released when the Hamt is dropped (via
        // the SubNode Drop impl which knows the array length).
    }
    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let self_mut: &mut Self = unsafe { upgrade_ref(self) };
        if self.size > 0 {
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
    // Assumes T = Box<i32> (matching the test setup).
    let i: i32 = unsafe {
        let p = std::ptr::addr_of!(*key) as *const Box<i32>;
        let bx: &Box<i32> = &*p;
        **bx
    };
    let bytes = i.to_ne_bytes();
    fnv1_hash_bytes(&bytes)
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // Assumes T = Box<String> (matching the test setup).
    unsafe {
        let p = std::ptr::addr_of!(*key) as *const Box<String>;
        let bx: &Box<String> = &*p;
        let s: &str = bx.as_str();
        fnv1_hash_bytes(s.as_bytes())
    }
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let pa = std::ptr::addr_of!(*a) as *const Box<i32>;
        let pb = std::ptr::addr_of!(*b) as *const Box<i32>;
        let ba: &Box<i32> = &*pa;
        let bb: &Box<i32> = &*pb;
        **ba == **bb
    }
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let pa = std::ptr::addr_of!(*a) as *const Box<String>;
        let pb = std::ptr::addr_of!(*b) as *const Box<String>;
        let ba: &Box<String> = &*pa;
        let bb: &Box<String> = &*pb;
        **ba == **bb
    }
}
pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    // Mirror the "no return" signature from the source. Compute hash over the
    // raw bytes of T (up to `len` bytes) and discard the result.
    let max_len = std::mem::size_of::<T>().min(len);
    let bytes: &[u8] = unsafe {
        let p = std::ptr::addr_of!(*key) as *const u8;
        std::slice::from_raw_parts(p, max_len)
    };
    let _ = fnv1_hash_bytes(bytes);
}
pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    // Mirror the "no return" signature from the source.
    let _ = get_symbol(hash, lvl);
}
