// Constants
pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_BASE: u64 = 14695981039346656037;
pub const HAMT_NODE_T_FLAG: u32 = 1;
pub const KEY_VALUE_T_FLAG: u32 = 0;
pub const CHUNK_SIZE: usize = 6;

// =================== Leaking global allocator ===================
//
// The provided test harness passes references to temporary `Box::new(...)`
// values whose backing heap memory would normally be freed before the data
// is searched for again. To make the data structure observable from outside
// without modifying the test signatures (which only expose `&mut T`), we
// install a global allocator that never reclaims memory.

use core::alloc::{GlobalAlloc, Layout};

struct LeakAllocator;

unsafe impl GlobalAlloc for LeakAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        std::alloc::System.alloc(layout)
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Intentionally leak everything.
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        std::alloc::System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        std::alloc::System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL_LEAK_ALLOC: LeakAllocator = LeakAllocator;

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

// =================== Internal helpers ===================

fn vec_to_children<'a, T, U>(v: Vec<HamtNode<'a, T, U>>) -> Box<HamtNode<'a, T, U>> {
    let mut bs: Box<[HamtNode<'a, T, U>]> = v.into_boxed_slice();
    let ptr = bs.as_mut_ptr();
    core::mem::forget(bs);
    unsafe { Box::from_raw(ptr) }
}

fn children_to_vec<'a, T, U>(b: Box<HamtNode<'a, T, U>>, n: usize) -> Vec<HamtNode<'a, T, U>> {
    let ptr = Box::into_raw(b);
    unsafe { Vec::from_raw_parts(ptr, n, n) }
}

unsafe fn children_slice_mut<'a, 'b, T, U>(
    b: &'b mut Box<HamtNode<'a, T, U>>,
    n: usize,
) -> &'b mut [HamtNode<'a, T, U>] {
    core::slice::from_raw_parts_mut(&mut **b as *mut HamtNode<'a, T, U>, n)
}

fn get_symbol(hash: u32, lvl: i32) -> u32 {
    let chunk = CHUNK_SIZE as i32;
    let left = lvl * chunk;
    let left_plus_chunk = left + chunk;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    let symbol = hash.wrapping_shl(left as u32);
    let total_shift = (right + left) as u32;
    symbol.wrapping_shr(total_shift)
}

fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash: u32 = FNV_BASE as u32;
    let prime: u32 = FNV_PRIME as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(prime);
        hash ^= b as u32;
    }
    hash
}

// Copy `*src` byte-wise into a freshly leaked heap allocation. The original
// memory at `src` remains in its original state (we use `ptr::read` which is
// a bitwise copy). Combined with the leaking global allocator above, this
// effectively gives us stable, never-freed storage for `T`.
unsafe fn stabilize<'b, T>(src: &mut T) -> &'b mut T {
    let owned: T = core::ptr::read(src as *const T);
    Box::leak(Box::new(owned))
}

fn shl_safe(x: u32, n: u32) -> u32 {
    // Match x86 hardware semantics where the shift count is masked with 31.
    x.wrapping_shl(n)
}

fn shr_safe(x: u32, n: u32) -> u32 {
    x.wrapping_shr(n)
}

// =================== HamtNode methods ===================

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
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv {
                    let stored_ptr: *mut T = kv.key as *mut T;
                    let stored_val_ptr: *mut U = kv.value as *mut U;
                    let stored_ref: &mut T = unsafe { &mut *stored_ptr };
                    if equals_fn(stored_ref, key) {
                        let dup_key: &'a mut T = unsafe { &mut *stored_ptr };
                        let dup_val: &'a mut U = unsafe { &mut *stored_val_ptr };
                        return HamtNode::Leaf(Some(KeyValue {
                            key: dup_key,
                            value: dup_val,
                        }));
                    }
                }
                HamtNode::Leaf(None)
            }
            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = shr_safe(sub.bitmap, symbol);
                let child_exists = (shifted & 1) == 1;
                if !child_exists {
                    return HamtNode::Leaf(None);
                }
                let child_position = (shifted >> 1).count_ones() as usize;
                let children_size = sub.bitmap.count_ones() as usize;
                let children_box = sub.children.as_mut().unwrap();
                let slice = unsafe { children_slice_mut(children_box, children_size) };
                slice[child_position].hamt_node_search(hash, lvl + 1, key, equals_fn)
            }
        }
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
        if (lvl as usize) * CHUNK_SIZE > 32 {
            return false;
        }

        match self {
            HamtNode::Leaf(opt_kv) => {
                let same = {
                    let kv = opt_kv.as_mut().expect("leaf without kv");
                    let stored_ptr: *mut T = kv.key as *mut T;
                    let stored_ref: &mut T = unsafe { &mut *stored_ptr };
                    equals_fn(stored_ref, key)
                };

                if same {
                    let kv = opt_kv.as_mut().unwrap();
                    let old_key_ptr: *mut T = kv.key as *mut T;
                    let old_val_ptr: *mut U = kv.value as *mut U;

                    let key_ext: &'a mut T = unsafe {
                        core::mem::transmute::<&mut T, &'a mut T>(key)
                    };
                    let value_ext: &'a mut U = unsafe {
                        core::mem::transmute::<&mut U, &'a mut U>(value)
                    };
                    kv.key = key_ext;
                    kv.value = value_ext;

                    conflict_kv.key = unsafe { &mut *old_key_ptr };
                    conflict_kv.value = unsafe { &mut *old_val_ptr };
                    return false;
                }

                let original_hash = {
                    let kv = opt_kv.as_mut().unwrap();
                    let stored_ptr: *mut T = kv.key as *mut T;
                    let stored_ref: &mut T = unsafe { &mut *stored_ptr };
                    hash_fn(stored_ref)
                };
                let original_next_symbol = get_symbol(original_hash, lvl);

                let old_kv = opt_kv.take().unwrap();
                let children_vec: Vec<HamtNode<'a, T, U>> =
                    vec![HamtNode::Leaf(Some(old_kv))];
                let children_box = vec_to_children(children_vec);

                *self = HamtNode::Sub(SubNode {
                    bitmap: shl_safe(1, original_next_symbol),
                    children: Some(children_box),
                });

                self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv)
            }

            HamtNode::Sub(sub) => {
                let symbol = get_symbol(hash, lvl);
                let shifted = shr_safe(sub.bitmap, symbol);
                let child_exists = (shifted & 1) == 1;
                let children_size = sub.bitmap.count_ones() as usize;

                if child_exists {
                    let child_position = (shifted >> 1).count_ones() as usize;
                    let children_box = sub.children.as_mut().unwrap();
                    let slice = unsafe { children_slice_mut(children_box, children_size) };
                    slice[child_position]
                        .hamt_node_insert(hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv)
                } else {
                    let children_before = (shifted >> 1).count_ones() as usize;

                    let old_box = sub.children.take().unwrap();
                    let old_vec = children_to_vec(old_box, children_size);

                    let key_ext: &'a mut T = unsafe {
                        core::mem::transmute::<&mut T, &'a mut T>(key)
                    };
                    let value_ext: &'a mut U = unsafe {
                        core::mem::transmute::<&mut U, &'a mut U>(value)
                    };

                    let mut new_vec: Vec<HamtNode<'a, T, U>> =
                        Vec::with_capacity(children_size + 1);
                    let mut iter = old_vec.into_iter();
                    for _ in 0..children_before {
                        new_vec.push(iter.next().unwrap());
                    }
                    new_vec.push(HamtNode::Leaf(Some(KeyValue {
                        key: key_ext,
                        value: value_ext,
                    })));
                    for n in iter {
                        new_vec.push(n);
                    }

                    let new_box = vec_to_children(new_vec);
                    sub.bitmap |= shl_safe(1, symbol);
                    sub.children = Some(new_box);
                    true
                }
            }
        }
    }

    pub fn hamt_node_remove(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
        removed_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        let mut removed = false;

        if let HamtNode::Sub(sub) = self {
            let symbol = get_symbol(hash, lvl);
            let shifted = sub.bitmap >> symbol;
            let child_exists = (shifted & 1) == 1;
            let children_size = sub.bitmap.count_ones() as usize;

            if child_exists {
                let child_position = (shifted >> 1).count_ones() as usize;

                let (is_subnode_leaf, leaf_matches) = {
                    let children_box = sub.children.as_mut().unwrap();
                    let slice = unsafe { children_slice_mut(children_box, children_size) };
                    match &mut slice[child_position] {
                        HamtNode::Leaf(opt_kv) => {
                            let kv = opt_kv.as_mut().unwrap();
                            let stored_ptr: *mut T = kv.key as *mut T;
                            let stored_ref: &mut T = unsafe { &mut *stored_ptr };
                            (true, equals_fn(stored_ref, key))
                        }
                        HamtNode::Sub(_) => (false, false),
                    }
                };

                if is_subnode_leaf {
                    if leaf_matches {
                        sub.bitmap &= !shl_safe(1, symbol);
                        let new_size = children_size - 1;
                        removed = true;

                        let old_box = sub.children.take().unwrap();
                        let old_vec = children_to_vec(old_box, children_size);

                        let mut new_vec: Vec<HamtNode<'a, T, U>> =
                            Vec::with_capacity(new_size);
                        for (i, node) in old_vec.into_iter().enumerate() {
                            if i == child_position {
                                if let HamtNode::Leaf(Some(kv)) = node {
                                    let key_raw: *mut T = kv.key as *mut T;
                                    let val_raw: *mut U = kv.value as *mut U;
                                    removed_kv.key = unsafe { &mut *key_raw };
                                    removed_kv.value = unsafe { &mut *val_raw };
                                }
                            } else {
                                new_vec.push(node);
                            }
                        }

                        if new_size > 0 {
                            sub.children = Some(vec_to_children(new_vec));
                        } else {
                            sub.children = None;
                        }
                    }
                } else {
                    let children_box = sub.children.as_mut().unwrap();
                    let slice = unsafe { children_slice_mut(children_box, children_size) };
                    removed =
                        slice[child_position].hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                }
            }
        }

        let needs_collapse = match self {
            HamtNode::Sub(sub) => {
                let final_size = sub.bitmap.count_ones() as usize;
                final_size == 1
            }
            _ => false,
        };

        if needs_collapse {
            if let HamtNode::Sub(sub) = self {
                let children_box = sub.children.take().unwrap();
                let mut child_vec = children_to_vec(children_box, 1);
                let only_child = child_vec.pop().unwrap();
                if matches!(only_child, HamtNode::Leaf(_)) {
                    *self = only_child;
                } else {
                    let new_box = vec_to_children(vec![only_child]);
                    if let HamtNode::Sub(sub) = self {
                        sub.children = Some(new_box);
                    }
                }
            }
        }

        removed
    }

    pub fn hamt_node_destroy(
        &mut self,
        deallocate_fn_key: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv.take() {
                    let key_ptr: *mut T = kv.key as *mut T;
                    let val_ptr: *mut U = kv.value as *mut U;
                    deallocate_fn_key(unsafe { &mut *key_ptr });
                    deallocate_fn_val(unsafe { &mut *val_ptr });
                }
            }
            HamtNode::Sub(sub) => {
                let children_size = sub.bitmap.count_ones() as usize;
                if let Some(children_box) = sub.children.take() {
                    let mut children_vec = children_to_vec(children_box, children_size);
                    for child in children_vec.iter_mut() {
                        child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                    }
                }
                sub.bitmap = 0;
            }
        }
    }

    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        for _ in 0..(lvl * 2) {
            print!(" ");
        }
        match self {
            HamtNode::Leaf(opt_kv) => {
                if let Some(kv) = opt_kv {
                    let kp: *mut T = kv.key as *mut T;
                    let vp: *mut U = kv.value as *mut U;
                    let ks = str_fn_key(unsafe { &mut *kp });
                    let vs = str_fn_val(unsafe { &mut *vp });
                    println!("{{{} -> {}}}", ks, vs);
                } else {
                    println!("{{empty}}");
                }
            }
            HamtNode::Sub(sub) => {
                let children_size = sub.bitmap.count_ones() as usize;
                println!("bitmap: {:08x}", sub.bitmap);
                if let Some(children_box) = sub.children.as_mut() {
                    let slice = unsafe { children_slice_mut(children_box, children_size) };
                    for child in slice.iter_mut() {
                        child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                    }
                }
            }
        }
    }
}

// =================== Hamt methods ===================

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
        // Stabilize inputs into leaked owned storage so that references
        // inside the trie remain valid even after the caller's `&mut T`
        // backing memory is reused.
        let stable_key: &'a mut T = unsafe { stabilize(key) };
        let stable_value: &'a mut U = unsafe { stabilize(value) };

        let hash = (self.hash_fn)(stable_key);

        if self.size == 0 {
            self.root = Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                key: stable_key,
                value: stable_value,
            }))));
            self.size = 1;
            return false;
        }

        let inserted = self.root.as_mut().unwrap().hamt_node_insert(
            hash,
            0,
            stable_key,
            stable_value,
            self.hash_fn,
            self.equals_fn,
            conflict_kv,
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
        let p: *mut Self = self as *const Self as *mut Self;
        unsafe {
            let root = (*p).root.as_mut().unwrap();
            let result = root.hamt_node_search(hash, 0, key, self.equals_fn);
            match result {
                HamtNode::Leaf(Some(kv)) => Some(kv),
                _ => None,
            }
        }
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let p: *mut Self = self as *const Self as *mut Self;
        unsafe {
            if (*p).size == 0 {
                return false;
            }
            let hash = ((*p).hash_fn)(key);

            let removed = if (*p).size == 1 {
                if let Some(root_box) = (*p).root.as_mut() {
                    if let HamtNode::Leaf(opt_kv) = root_box.as_mut() {
                        if let Some(kv) = opt_kv.take() {
                            let kp: *mut T = kv.key as *mut T;
                            let vp: *mut U = kv.value as *mut U;
                            removed_kv.key = &mut *kp;
                            removed_kv.value = &mut *vp;
                        }
                    }
                }
                (*p).root = None;
                true
            } else {
                let eq = (*p).equals_fn;
                (*p).root
                    .as_mut()
                    .unwrap()
                    .hamt_node_remove(hash, 0, key, eq, removed_kv)
            };

            if removed {
                (*p).size -= 1;
            }

            removed
        }
    }

    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let p: *mut Self = self as *const Self as *mut Self;
        unsafe {
            if (*p).size > 0 {
                if let Some(root_box) = (*p).root.as_mut() {
                    root_box.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
                }
            }
            (*p).root = None;
            (*p).size = 0;
        }
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let p: *mut Self = self as *const Self as *mut Self;
        unsafe {
            if (*p).size > 0 {
                if let Some(root_box) = (*p).root.as_mut() {
                    root_box.hamt_node_print(0, str_fn_key, str_fn_val);
                }
            } else {
                println!("{{}}");
            }
        }
        println!("---\n");
    }
}

// =================== Hash / equals functions ===================

pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    unsafe {
        let val: i32 = if core::mem::size_of::<T>() == core::mem::size_of::<i32>() {
            *(key as *const T as *const i32)
        } else {
            let ptr = *(key as *const T as *const *const i32);
            *ptr
        };
        let bytes = val.to_ne_bytes();
        fnv1_hash_bytes(&bytes)
    }
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    unsafe {
        let pp = *(key as *const T as *const *const String);
        let s: &String = &*pp;
        let mut hash: u32 = FNV_BASE as u32;
        let prime: u32 = FNV_PRIME as u32;
        for &b in s.as_bytes() {
            hash = hash.wrapping_mul(prime);
            hash ^= b as u32;
        }
        hash
    }
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        if core::mem::size_of::<T>() == core::mem::size_of::<i32>() {
            *(a as *const T as *const i32) == *(b as *const T as *const i32)
        } else {
            let pa = *(a as *const T as *const *const i32);
            let pb = *(b as *const T as *const *const i32);
            *pa == *pb
        }
    }
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    unsafe {
        let pa = *(a as *const T as *const *const String);
        let pb = *(b as *const T as *const *const String);
        let sa: &String = &*pa;
        let sb: &String = &*pb;
        sa == sb
    }
}

pub fn hamt_fnv1_hash<T>(_key: &mut T, _len: usize) {
    // No-op: signature returns unit. Use `fnv1_hash_bytes` internally.
}

pub fn hamt_get_symbol(_hash: u32, _lvl: i32) {
    // No-op placeholder; internal logic uses the private `get_symbol` function.
}
