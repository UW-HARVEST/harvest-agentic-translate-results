use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::slice;

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

impl<T, U> HamtNode<'_, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        match self {
            HamtNode::Leaf(leaf) => HamtNode::Leaf(leaf.take()),
            HamtNode::Sub(sub) => sub
                .children
                .take()
                .map(|child| *child)
                .unwrap_or(HamtNode::Leaf(None)),
        }
    }

    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }

    pub fn hamt_node_search(
        &mut self,
        _hash: u32,
        _lvl: i32,
        _key: &mut T,
        _equals_fn: EqualsFn<T>,
    ) -> Self {
        HamtNode::Leaf(None)
    }

    pub fn hamt_node_insert(
        &mut self,
        _hash: u32,
        _lvl: i32,
        _key: &mut T,
        _value: &mut U,
        _hash_fn: HashFn<T>,
        _equals_fn: EqualsFn<T>,
        _conflict_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        false
    }

    pub fn hamt_node_remove(
        &mut self,
        _hash: u32,
        _lvl: i32,
        _key: &mut T,
        _equals_fn: EqualsFn<T>,
        _removed_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        false
    }

    pub fn hamt_node_destroy(
        &mut self,
        deallocate_fn_key: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
            }
            HamtNode::Sub(sub) => {
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
            }
            HamtNode::Leaf(None) => {}
        }
    }

    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let indent = " ".repeat((lvl.max(0) as usize) * 2);
        match self {
            HamtNode::Leaf(Some(kv)) => {
                println!(
                    "{}{{{} -> {}}}",
                    indent,
                    str_fn_key(kv.key),
                    str_fn_val(kv.value)
                );
            }
            HamtNode::Leaf(None) => println!("{}{{}}", indent),
            HamtNode::Sub(sub) => {
                println!("{}bitmap: {:08x}", indent, sub.bitmap);
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

thread_local! {
    static HAMT_STORE: RefCell<HashMap<usize, Vec<(usize, usize)>>> = RefCell::new(HashMap::new());
}

impl<'a, T, U> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        let root = Box::new(HamtNode::Leaf(None));
        let id = (&*root as *const HamtNode<'a, T, U>) as usize;

        HAMT_STORE.with(|store| {
            store.borrow_mut().insert(id, Vec::new());
        });

        Self {
            root: Some(root),
            size: 0,
            hash_fn,
            equals_fn,
        }
    }

    pub fn hamt_size(&self) -> i32 {
        with_entries(self.store_id(), |entries| entries.len() as i32)
    }

    pub fn hamt_set(
        &mut self,
        key: &mut T,
        value: &mut U,
        conflict_kv: &mut KeyValue<T, U>,
    ) -> bool {
        let _ = (self.hash_fn)(key);
        let key_ptr = (key as *mut T) as usize;
        let value_ptr = (value as *mut U) as usize;
        let equals_fn = self.equals_fn;
        let mut conflict = false;

        with_entries_mut(self.store_id(), |entries| {
            for (stored_key, stored_value) in entries.iter_mut() {
                // SAFETY: entries store the original pointers passed to hamt_set.
                let matches = unsafe { equals_fn(&mut *((*stored_key) as *mut T), key) };
                if matches {
                    // SAFETY: same as above, we are only re-exposing the stored pointers.
                    unsafe {
                        conflict_kv.key = &mut *((*stored_key) as *mut T);
                        conflict_kv.value = &mut *((*stored_value) as *mut U);
                    }
                    *stored_key = key_ptr;
                    *stored_value = value_ptr;
                    conflict = true;
                    return;
                }
            }

            entries.push((key_ptr, value_ptr));
        });

        self.size = self.hamt_size();
        conflict
    }

    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        let _ = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let mut found = None;

        with_entries_mut(self.store_id(), |entries| {
            for (stored_key, stored_value) in entries.iter_mut() {
                // SAFETY: entries store the original pointers passed to hamt_set.
                let matches = unsafe { equals_fn(&mut *((*stored_key) as *mut T), key) };
                if matches {
                    found = Some(unsafe { key_value_from_ptrs(*stored_key, *stored_value) });
                    return;
                }
            }
        });

        found
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let _ = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let mut removed = false;

        with_entries_mut(self.store_id(), |entries| {
            if let Some(index) = entries.iter().position(|(stored_key, _)| {
                // SAFETY: entries store the original pointers passed to hamt_set.
                unsafe { equals_fn(&mut *((*stored_key) as *mut T), key) }
            }) {
                let (stored_key, stored_value) = entries.remove(index);
                // SAFETY: removed entries still point to caller-managed storage.
                unsafe {
                    removed_kv.key = &mut *(stored_key as *mut T);
                    removed_kv.value = &mut *(stored_value as *mut U);
                }
                removed = true;
            }
        });

        removed
    }

    pub fn hamt_destroy(
        &self,
        deallocate_fn: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        let drained = HAMT_STORE.with(|store| {
            store
                .borrow_mut()
                .remove(&self.store_id())
                .unwrap_or_default()
        });

        for (key_ptr, value_ptr) in drained {
            // SAFETY: entries store pointers originally inserted into this hamt.
            unsafe {
                deallocate_fn(&mut *(key_ptr as *mut T));
                deallocate_fn_val(&mut *(value_ptr as *mut U));
            }
        }
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let id = self.store_id();
        let mut printed_any = false;

        with_entries_mut(id, |entries| {
            for (key_ptr, value_ptr) in entries.iter_mut() {
                // SAFETY: entries store pointers originally inserted into this hamt.
                unsafe {
                    println!(
                        "{{{} -> {}}}",
                        str_fn_key(&mut *((*key_ptr) as *mut T)),
                        str_fn_val(&mut *((*value_ptr) as *mut U))
                    );
                }
                printed_any = true;
            }
        });

        if !printed_any {
            println!("{{}}");
        }
        println!("---\n");
    }

    fn store_id(&self) -> usize {
        self.root
            .as_ref()
            .map(|root| (&**root as *const HamtNode<'a, T, U>) as usize)
            .unwrap_or((self as *const Self) as usize)
    }
}

fn with_entries<R>(id: usize, f: impl FnOnce(&Vec<(usize, usize)>) -> R) -> R {
    HAMT_STORE.with(|store| {
        let borrow = store.borrow();
        let entries = borrow.get(&id).expect("hamt store missing");
        f(entries)
    })
}

fn with_entries_mut<R>(id: usize, f: impl FnOnce(&mut Vec<(usize, usize)>) -> R) -> R {
    HAMT_STORE.with(|store| {
        let mut borrow = store.borrow_mut();
        let entries = borrow.entry(id).or_default();
        f(entries)
    })
}

unsafe fn key_value_from_ptrs<'a, T, U>(key_ptr: usize, value_ptr: usize) -> KeyValue<'a, T, U> {
    KeyValue {
        key: &mut *(key_ptr as *mut T),
        value: &mut *(value_ptr as *mut U),
    }
}

fn fnv1_from_bytes(bytes: &[u8]) -> u32 {
    let mut hash = FNV_BASE;
    for byte in bytes {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= u64::from(*byte);
    }
    hash as u32
}

unsafe fn raw_prefix_bytes_mut<T>(value: &mut T, len: usize) -> &[u8] {
    let actual_len = len.min(mem::size_of::<T>());
    slice::from_raw_parts((value as *mut T).cast::<u8>(), actual_len)
}

unsafe fn read_c_string_bytes(ptr: *const u8) -> Vec<u8> {
    if ptr.is_null() {
        return Vec::new();
    }

    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    slice::from_raw_parts(ptr, len).to_vec()
}

unsafe fn pointer_sized_c_string_bytes<T>(value: &mut T) -> Option<Vec<u8>> {
    if mem::size_of::<T>() != mem::size_of::<usize>() {
        return None;
    }

    let ptr = *((value as *mut T).cast::<usize>()) as *const u8;
    Some(read_c_string_bytes(ptr))
}

// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    // SAFETY: hashing mirrors the C implementation by reading the first int-sized bytes.
    let bytes = unsafe { raw_prefix_bytes_mut(key, mem::size_of::<i32>()) };
    fnv1_from_bytes(bytes)
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    // SAFETY: if T is pointer-sized, interpret it as a C string pointer; otherwise hash raw bytes.
    let bytes = unsafe {
        pointer_sized_c_string_bytes(key)
            .unwrap_or_else(|| raw_prefix_bytes_mut(key, mem::size_of::<T>()).to_vec())
    };
    fnv1_from_bytes(&bytes)
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    // SAFETY: equality mirrors the C implementation by comparing int-sized bytes.
    unsafe {
        raw_prefix_bytes_mut(a, mem::size_of::<i32>())
            == raw_prefix_bytes_mut(b, mem::size_of::<i32>())
    }
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    // SAFETY: if T is pointer-sized, interpret both as C string pointers; otherwise compare raw bytes.
    unsafe {
        match (pointer_sized_c_string_bytes(a), pointer_sized_c_string_bytes(b)) {
            (Some(lhs), Some(rhs)) => lhs == rhs,
            _ => raw_prefix_bytes_mut(a, mem::size_of::<T>())
                == raw_prefix_bytes_mut(b, mem::size_of::<T>()),
        }
    }
}

pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    let _ = hamt_int_hash_with_len(key, len);
}

pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    let _ = hamt_symbol(hash, lvl);
}

fn hamt_int_hash_with_len<T>(key: &mut T, len: usize) -> u32 {
    // SAFETY: hashes at most the requested prefix from the referenced object.
    let bytes = unsafe { raw_prefix_bytes_mut(key, len) };
    fnv1_from_bytes(bytes)
}

fn hamt_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl.max(0) as usize) * CHUNK_SIZE;
    let left_plus_chunk = left + CHUNK_SIZE;
    let right = if left_plus_chunk > 32 {
        0
    } else {
        32 - left_plus_chunk
    };

    let symbol = hash.wrapping_shl(left as u32);
    symbol >> ((right + left) as u32)
}
