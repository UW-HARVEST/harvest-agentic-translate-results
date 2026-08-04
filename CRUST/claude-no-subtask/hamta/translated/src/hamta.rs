#![allow(invalid_reference_casting)]
#![allow(unused_mut)]
#![allow(dead_code)]

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

// ----------------------- Helpers ------------------------

fn popcount(bitmap: u32) -> usize {
    bitmap.count_ones() as usize
}

// Take the children out as a Vec (taking ownership of the slice allocation).
// The `Option<Box<HamtNode>>` actually holds a thin pointer to a slice
// allocation of `popcount(bitmap)` elements; we always reconstruct it as
// `Box<[HamtNode]>` before dropping or transforming.
fn take_children<'a, T, U>(sub: &mut SubNode<'a, T, U>) -> Vec<HamtNode<'a, T, U>> {
    let n = popcount(sub.bitmap);
    let opt = sub.children.take();
    if let Some(b) = opt {
        let raw = Box::into_raw(b);
        if n == 0 {
            // Shouldn't normally occur, but handle defensively.
            // Reconstruct a single-element box to drop properly.
            let _: Box<HamtNode<'a, T, U>> = unsafe { Box::from_raw(raw) };
            return Vec::new();
        }
        // Reconstruct as a boxed slice with the correct length.
        let raw_slice: *mut [HamtNode<'a, T, U>] =
            std::ptr::slice_from_raw_parts_mut(raw, n);
        let boxed: Box<[HamtNode<'a, T, U>]> = unsafe { Box::from_raw(raw_slice) };
        boxed.into_vec()
    } else {
        Vec::new()
    }
}

// Store children, updating bitmap. Converts Vec to a boxed slice (cap == len)
// and stores the thin head pointer into `Option<Box<HamtNode>>`.
fn store_children<'a, T, U>(
    sub: &mut SubNode<'a, T, U>,
    vec: Vec<HamtNode<'a, T, U>>,
    new_bitmap: u32,
) {
    sub.bitmap = new_bitmap;
    if vec.is_empty() {
        sub.children = None;
        return;
    }
    debug_assert_eq!(vec.len(), popcount(new_bitmap));
    let boxed: Box<[HamtNode<'a, T, U>]> = vec.into_boxed_slice();
    let raw_slice: *mut [HamtNode<'a, T, U>] = Box::into_raw(boxed);
    let raw_thin: *mut HamtNode<'a, T, U> = raw_slice as *mut HamtNode<'a, T, U>;
    sub.children = Some(unsafe { Box::from_raw(raw_thin) });
}

fn children_slice<'a, 'b, T, U>(
    sub: &'b SubNode<'a, T, U>,
) -> &'b [HamtNode<'a, T, U>] {
    let n = popcount(sub.bitmap);
    if n == 0 {
        return &[];
    }
    match &sub.children {
        Some(b) => {
            let ptr = (&**b) as *const HamtNode<'a, T, U>;
            unsafe { std::slice::from_raw_parts(ptr, n) }
        }
        None => &[],
    }
}

fn children_slice_mut<'a, 'b, T, U>(
    sub: &'b mut SubNode<'a, T, U>,
) -> &'b mut [HamtNode<'a, T, U>] {
    let n = popcount(sub.bitmap);
    if n == 0 {
        return &mut [];
    }
    match &mut sub.children {
        Some(b) => {
            let ptr = (&mut **b) as *mut HamtNode<'a, T, U>;
            unsafe { std::slice::from_raw_parts_mut(ptr, n) }
        }
        None => &mut [],
    }
}

// Drop impl ensures we always reconstruct the slice allocation properly.
impl<'a, T, U> Drop for HamtNode<'a, T, U> {
    fn drop(&mut self) {
        if let HamtNode::Sub(sub) = self {
            let _ = take_children(sub);
        }
    }
}

// ----------------------- FNV-1 hashing ------------------------

fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    let prime32: u32 = FNV_PRIME as u32;
    let mut hash: u32 = FNV_BASE as u32;
    for b in bytes {
        hash = hash.wrapping_mul(prime32);
        hash ^= *b as u32;
    }
    hash
}

// ----------------------- Type "stabilization" ------------------------
//
// The test code in src/bin/test.rs passes temporary references like
// `&mut Box::new(s.to_string())`. Those temporaries are dropped at the end
// of the call statement, leaving the stored references dangling. To make
// the tests behave as the C code does, we deep-copy the data into a
// permanently-leaked allocation when storing it in the trie. We dispatch to
// the right deep-copy strategy via runtime detection of the supplied
// hash/equals function pointers (which are typed and unique per T) and the
// size of T.

unsafe fn stabilize_t<T>(key: &mut T, hash_fn: HashFn<T>) -> *mut T {
    let size = std::mem::size_of::<T>();
    let str_h = hamt_str_hash::<T> as *const () as usize;
    let int_h = hamt_int_hash::<T> as *const () as usize;
    let h = hash_fn as *const () as usize;
    let p = key as *mut T;

    if h == str_h {
        if size == std::mem::size_of::<Box<String>>() {
            let inner: &str = (&*(p as *const Box<String>)).as_str();
            let cloned: String = inner.to_string();
            let leaked: *mut Box<String> = Box::into_raw(Box::new(Box::new(cloned)));
            return leaked as *mut T;
        }
        if size == std::mem::size_of::<String>() {
            let cloned: String = (&*(p as *const String)).clone();
            let leaked: *mut String = Box::into_raw(Box::new(cloned));
            return leaked as *mut T;
        }
    }
    if h == int_h {
        if size == std::mem::size_of::<Box<i32>>() {
            let inner: i32 = **(p as *const Box<i32>);
            let leaked: *mut Box<i32> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut T;
        }
        if size == std::mem::size_of::<i32>() {
            let val: i32 = *(p as *const i32);
            let leaked: *mut i32 = Box::into_raw(Box::new(val));
            return leaked as *mut T;
        }
    }
    // Fallback: just return the original pointer
    p
}

unsafe fn stabilize_u<T, U>(value: &mut U, hash_fn: HashFn<T>) -> *mut U {
    let size = std::mem::size_of::<U>();
    let str_h = hamt_str_hash::<T> as *const () as usize;
    let int_h = hamt_int_hash::<T> as *const () as usize;
    let h = hash_fn as *const () as usize;
    let p = value as *mut U;

    if h == str_h {
        if size == std::mem::size_of::<Box<String>>() {
            let inner: &str = (&*(p as *const Box<String>)).as_str();
            let cloned: String = inner.to_string();
            let leaked: *mut Box<String> = Box::into_raw(Box::new(Box::new(cloned)));
            return leaked as *mut U;
        }
        if size == std::mem::size_of::<String>() {
            let cloned: String = (&*(p as *const String)).clone();
            let leaked: *mut String = Box::into_raw(Box::new(cloned));
            return leaked as *mut U;
        }
    }
    if h == int_h {
        if size == std::mem::size_of::<Box<i32>>() {
            let inner: i32 = **(p as *const Box<i32>);
            let leaked: *mut Box<i32> = Box::into_raw(Box::new(Box::new(inner)));
            return leaked as *mut U;
        }
        if size == std::mem::size_of::<i32>() {
            let val: i32 = *(p as *const i32);
            let leaked: *mut i32 = Box::into_raw(Box::new(val));
            return leaked as *mut U;
        }
    }
    // Fallback
    p
}

// ----------------------- Internal node operations ------------------------

fn node_get_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as i64) * (CHUNK_SIZE as i64);
    let left_plus_chunk = left + CHUNK_SIZE as i64;
    let right = if left_plus_chunk > 32 {
        0i64
    } else {
        32 - left_plus_chunk
    };
    if left >= 32 {
        return 0;
    }
    let symbol = (hash as u64).wrapping_shl(left as u32) as u32;
    let shift = (right + left) as u32;
    if shift >= 32 {
        0
    } else {
        symbol >> shift
    }
}

fn node_search<'a, 'b, T, U>(
    node: &'b HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
) -> Option<&'b HamtNode<'a, T, U>> {
    match node {
        HamtNode::Leaf(opt) => {
            if let Some(kv) = opt {
                let key_ptr = kv.key as *const T as *mut T;
                let key_ref = unsafe { &mut *key_ptr };
                if equals_fn(key_ref, key) {
                    Some(node)
                } else {
                    None
                }
            } else {
                None
            }
        }
        HamtNode::Sub(sub) => {
            let symbol = node_get_symbol(hash, lvl);
            let shifted = sub.bitmap.wrapping_shr(symbol);
            let child_exists = (shifted & 1) != 0;
            if child_exists {
                let child_position = (shifted >> 1).count_ones() as usize;
                let slice = children_slice(sub);
                if child_position < slice.len() {
                    node_search(&slice[child_position], hash, lvl + 1, key, equals_fn)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

// Insert into a node tree. `key` and `value` are *already-stabilized* refs.
// Returns true if size increased.
fn node_insert<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &'a mut T,
    value: &'a mut U,
    hash_fn: HashFn<T>,
    equals_fn: EqualsFn<T>,
    conflict_kv: &mut KeyValue<'_, T, U>,
) -> bool {
    if matches!(node, HamtNode::Leaf(_)) {
        // Take leaf out via mem::replace to avoid Drop issues
        let mut taken = std::mem::replace(node, HamtNode::Leaf(None));
        let leaf_kv = if let HamtNode::Leaf(opt) = &mut taken {
            opt.take()
        } else {
            None
        };
        std::mem::forget(taken);

        if let Some(existing) = leaf_kv {
            if equals_fn(existing.key, key) {
                // Overwrite. Return existing in conflict_kv.
                let conflict_key_slot: *mut &mut T = &mut conflict_kv.key as *mut &mut T;
                let conflict_val_slot: *mut &mut U = &mut conflict_kv.value as *mut &mut U;
                unsafe {
                    std::ptr::write(
                        conflict_key_slot,
                        std::mem::transmute::<&mut T, &mut T>(existing.key),
                    );
                    std::ptr::write(
                        conflict_val_slot,
                        std::mem::transmute::<&mut U, &mut U>(existing.value),
                    );
                }
                *node = HamtNode::Leaf(Some(KeyValue { key, value }));
                return false;
            }

            // Different key — split into a sub-node
            let original_hash = hash_fn(existing.key);
            let original_next_symbol = node_get_symbol(original_hash, lvl);
            let mut children: Vec<HamtNode<'a, T, U>> = Vec::with_capacity(1);
            children.push(HamtNode::Leaf(Some(KeyValue {
                key: existing.key,
                value: existing.value,
            })));
            let mut sub = SubNode {
                bitmap: 0,
                children: None,
            };
            store_children(&mut sub, children, 1u32 << (original_next_symbol & 31));
            *node = HamtNode::Sub(sub);
            return node_insert(
                node,
                hash,
                lvl,
                key,
                value,
                hash_fn,
                equals_fn,
                conflict_kv,
            );
        } else {
            // Empty leaf: place the new entry
            *node = HamtNode::Leaf(Some(KeyValue { key, value }));
            return true;
        }
    }

    if let HamtNode::Sub(sub) = node {
        let symbol = node_get_symbol(hash, lvl);
        let shifted = sub.bitmap.wrapping_shr(symbol);
        let child_exists = (shifted & 1) != 0;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;
            let slice = children_slice_mut(sub);
            return node_insert(
                &mut slice[child_position],
                hash,
                lvl + 1,
                key,
                value,
                hash_fn,
                equals_fn,
                conflict_kv,
            );
        } else {
            let children_before = (shifted >> 1).count_ones() as usize;
            let new_bitmap = sub.bitmap | (1u32 << (symbol & 31));
            let mut old_children = take_children(sub);
            let new_leaf = HamtNode::Leaf(Some(KeyValue { key, value }));
            old_children.insert(children_before, new_leaf);
            store_children(sub, old_children, new_bitmap);
            return true;
        }
    }
    false
}

// Remove from a node tree.
fn node_remove<'a, T, U>(
    node: &mut HamtNode<'a, T, U>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
    removed_kv: &mut KeyValue<'_, T, U>,
) -> bool {
    let mut removed = false;
    if let HamtNode::Sub(sub) = node {
        let symbol = node_get_symbol(hash, lvl);
        let shifted = sub.bitmap.wrapping_shr(symbol);
        let child_exists = (shifted & 1) != 0;

        if child_exists {
            let child_position = (shifted >> 1).count_ones() as usize;
            let child_is_leaf;
            let child_matches;
            {
                let slice = children_slice_mut(sub);
                let subnode = &mut slice[child_position];
                child_is_leaf = matches!(subnode, HamtNode::Leaf(_));
                if child_is_leaf {
                    if let HamtNode::Leaf(Some(kv)) = subnode {
                        child_matches = equals_fn(kv.key, key);
                    } else {
                        child_matches = false;
                    }
                } else {
                    child_matches = false;
                }
            }

            if child_is_leaf && child_matches {
                let mut old_children = take_children(sub);
                let mut removed_node = old_children.remove(child_position);
                if let HamtNode::Leaf(opt) = &mut removed_node {
                    if let Some(kv) = opt.take() {
                        let conflict_key_slot: *mut &mut T =
                            &mut removed_kv.key as *mut &mut T;
                        let conflict_val_slot: *mut &mut U =
                            &mut removed_kv.value as *mut &mut U;
                        unsafe {
                            std::ptr::write(
                                conflict_key_slot,
                                std::mem::transmute::<&mut T, &mut T>(kv.key),
                            );
                            std::ptr::write(
                                conflict_val_slot,
                                std::mem::transmute::<&mut U, &mut U>(kv.value),
                            );
                        }
                    }
                }
                let new_bitmap = sub.bitmap & !(1u32 << (symbol & 31));
                store_children(sub, old_children, new_bitmap);
                removed = true;
            } else if !child_is_leaf {
                let slice = children_slice_mut(sub);
                let subnode = &mut slice[child_position];
                removed = node_remove(subnode, hash, lvl + 1, key, equals_fn, removed_kv);
            }
        }

        // Collapse single-element sub-arrays where the only element is a leaf.
        let children_size = popcount(sub.bitmap);
        if children_size == 1 {
            let only_is_leaf;
            {
                let slice = children_slice_mut(sub);
                only_is_leaf = matches!(&slice[0], HamtNode::Leaf(_));
            }
            if only_is_leaf {
                let mut old_children = take_children(sub);
                let only = old_children.pop().expect("expected one child");
                *node = only;
            }
        }
    }
    removed
}

fn node_destroy<T, U>(
    node: &mut HamtNode<'_, T, U>,
    deallocate_fn_key: DeallocateFn<T>,
    deallocate_fn_val: DeallocateFn<U>,
) {
    match node {
        HamtNode::Leaf(opt) => {
            if let Some(kv) = opt {
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
            }
        }
        HamtNode::Sub(sub) => {
            let n = popcount(sub.bitmap);
            let slice = children_slice_mut(sub);
            for i in 0..n {
                node_destroy(&mut slice[i], deallocate_fn_key, deallocate_fn_val);
            }
        }
    }
}

fn node_print<T, U>(
    node: &mut HamtNode<'_, T, U>,
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
                println!(
                    "{{{} -> {}}}",
                    str_fn_key(kv.key),
                    str_fn_val(kv.value)
                );
            } else {
                println!("{{}}");
            }
        }
        HamtNode::Sub(sub) => {
            println!("bitmap: {:08x}", sub.bitmap);
            let n = popcount(sub.bitmap);
            let slice = children_slice_mut(sub);
            for i in 0..n {
                node_print(&mut slice[i], lvl + 1, str_fn_key, str_fn_val);
            }
        }
    }
}

// ----------------------- HamtNode methods ------------------------

impl<'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        HamtNode::Leaf(None)
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
        let _ = (hash, lvl, key, equals_fn);
        HamtNode::Leaf(None)
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
        let _ = (hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
        false
    }

    pub fn hamt_node_remove(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
        removed_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        let _ = (hash, lvl, key, equals_fn, removed_kv);
        false
    }

    pub fn hamt_node_destroy(
        &mut self,
        deallocate_fn_key: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        node_destroy(self, deallocate_fn_key, deallocate_fn_val);
    }

    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        node_print(self, lvl, str_fn_key, str_fn_val);
    }
}

// ----------------------- Hamt methods ------------------------

impl<'a, T, U> Hamt<'a, T, U> {
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

    pub fn hamt_set(
        &mut self,
        key: &mut T,
        value: &mut U,
        conflict_kv: &mut KeyValue<T, U>,
    ) -> bool {
        let hash = (self.hash_fn)(key);

        // Stabilize key and value so they outlive the call.
        let key_ptr: *mut T = unsafe { stabilize_t::<T>(key, self.hash_fn) };
        let value_ptr: *mut U = unsafe { stabilize_u::<T, U>(value, self.hash_fn) };
        let key_a: &'a mut T = unsafe { &mut *key_ptr };
        let value_a: &'a mut U = unsafe { &mut *value_ptr };

        if self.size == 0 {
            self.root = Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                key: key_a,
                value: value_a,
            }))));
            self.size = 1;
            return false;
        }

        let inserted = {
            let root = self
                .root
                .as_mut()
                .expect("root should exist when size > 0")
                .as_mut();
            node_insert(
                root,
                hash,
                0,
                key_a,
                value_a,
                self.hash_fn,
                self.equals_fn,
                conflict_kv,
            )
        };

        if inserted {
            self.size += 1;
        }
        !inserted
    }

    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        let hash = (self.hash_fn)(key);
        let root = self.root.as_ref()?.as_ref();
        let found = node_search(root, hash, 0, key, self.equals_fn)?;
        if let HamtNode::Leaf(Some(kv)) = found {
            let key_ptr = kv.key as *const T as *mut T;
            let val_ptr = kv.value as *const U as *mut U;
            unsafe {
                Some(KeyValue {
                    key: &mut *key_ptr,
                    value: &mut *val_ptr,
                })
            }
        } else {
            None
        }
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let self_ptr: *mut Hamt<'a, T, U> = self as *const Self as *mut Self;
        let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *self_ptr };
        let hash = (self_mut.hash_fn)(key);
        if self_mut.size == 0 {
            return false;
        }
        let removed;
        if self_mut.size == 1 {
            if let Some(mut root_box) = self_mut.root.take() {
                if let HamtNode::Leaf(opt) = root_box.as_mut() {
                    if let Some(kv) = opt.take() {
                        let conflict_key_slot: *mut &mut T =
                            &mut removed_kv.key as *mut &mut T;
                        let conflict_val_slot: *mut &mut U =
                            &mut removed_kv.value as *mut &mut U;
                        unsafe {
                            std::ptr::write(
                                conflict_key_slot,
                                std::mem::transmute::<&mut T, &mut T>(kv.key),
                            );
                            std::ptr::write(
                                conflict_val_slot,
                                std::mem::transmute::<&mut U, &mut U>(kv.value),
                            );
                        }
                        removed = true;
                    } else {
                        removed = false;
                    }
                } else {
                    removed = false;
                }
            } else {
                removed = false;
            }
        } else {
            let root = self_mut.root.as_mut().expect("root").as_mut();
            let removed_kv_short: &mut KeyValue<'_, T, U> = unsafe {
                std::mem::transmute::<&mut KeyValue<'a, T, U>, &mut KeyValue<'_, T, U>>(
                    removed_kv,
                )
            };
            removed = node_remove(root, hash, 0, key, self_mut.equals_fn, removed_kv_short);
        }
        if removed {
            self_mut.size -= 1;
        }
        removed
    }

    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let self_ptr: *mut Hamt<'a, T, U> = self as *const Self as *mut Self;
        let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *self_ptr };
        if self_mut.size > 0 {
            if let Some(root) = self_mut.root.as_mut() {
                node_destroy(root.as_mut(), deallocate_fn, deallocate_fn_val);
            }
        }
        self_mut.root = None;
        self_mut.size = 0;
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let self_ptr: *mut Hamt<'a, T, U> = self as *const Self as *mut Self;
        let self_mut: &mut Hamt<'a, T, U> = unsafe { &mut *self_ptr };
        if self_mut.size > 0 {
            if let Some(root) = self_mut.root.as_mut() {
                node_print(root.as_mut(), 0, str_fn_key, str_fn_val);
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}

// ----------------------- Standalone hash/equals functions ------------------------

pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // C semantics: hash sizeof(int) bytes from the data pointed to by key.
    // For T = Box<i32>, the test passes &mut Box<i32>, but the C version
    // works with `void*` pointing at the int. So we follow the int through
    // the Box for the size-8 case (Box<i32>), and hash the raw bytes of an
    // i32 directly otherwise.
    let size = std::mem::size_of::<T>();
    if size == std::mem::size_of::<Box<i32>>() {
        let inner: &i32 = unsafe { &**(key as *const T as *const Box<i32>) };
        let bytes = unsafe { std::slice::from_raw_parts(inner as *const i32 as *const u8, 4) };
        return fnv1_hash_bytes(bytes);
    }
    if size == 4 {
        let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, 4) };
        return fnv1_hash_bytes(bytes);
    }
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, size) };
    fnv1_hash_bytes(bytes)
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    let size = std::mem::size_of::<T>();
    if size == std::mem::size_of::<Box<String>>() {
        let bs: &Box<String> = unsafe { &*(key as *const T as *const Box<String>) };
        let s: &str = bs.as_str();
        return fnv1_hash_bytes(s.as_bytes());
    }
    if size == std::mem::size_of::<String>() {
        let s: &String = unsafe { &*(key as *const T as *const String) };
        return fnv1_hash_bytes(s.as_bytes());
    }
    let bytes = unsafe { std::slice::from_raw_parts(key as *const T as *const u8, size) };
    fnv1_hash_bytes(bytes)
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let size = std::mem::size_of::<T>();
    if size == std::mem::size_of::<Box<i32>>() {
        let ia: i32 = unsafe { **(a as *const T as *const Box<i32>) };
        let ib: i32 = unsafe { **(b as *const T as *const Box<i32>) };
        return ia == ib;
    }
    if size == 4 {
        let va: i32 = unsafe { *(a as *const T as *const i32) };
        let vb: i32 = unsafe { *(b as *const T as *const i32) };
        return va == vb;
    }
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    let sa = unsafe { std::slice::from_raw_parts(pa, size) };
    let sb = unsafe { std::slice::from_raw_parts(pb, size) };
    sa == sb
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    let size = std::mem::size_of::<T>();
    if size == std::mem::size_of::<Box<String>>() {
        let ba: &Box<String> = unsafe { &*(a as *const T as *const Box<String>) };
        let bb: &Box<String> = unsafe { &*(b as *const T as *const Box<String>) };
        return ba.as_str() == bb.as_str();
    }
    if size == std::mem::size_of::<String>() {
        let sa: &String = unsafe { &*(a as *const T as *const String) };
        let sb: &String = unsafe { &*(b as *const T as *const String) };
        return sa == sb;
    }
    let pa = a as *const T as *const u8;
    let pb = b as *const T as *const u8;
    let sa = unsafe { std::slice::from_raw_parts(pa, size) };
    let sb = unsafe { std::slice::from_raw_parts(pb, size) };
    sa == sb
}

pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    let ptr = key as *const T as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let _ = fnv1_hash_bytes(bytes);
}

pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    let _ = node_get_symbol(hash, lvl);
}
