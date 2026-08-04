use std::any::type_name;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;

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

struct Entry<T, U> {
    key: T,
    value: U,
}

struct Store<T, U> {
    buckets: HashMap<u32, Vec<Entry<T, U>>>,
    len: usize,
}

impl<T, U> Store<T, U> {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            len: 0,
        }
    }
}

thread_local! {
    static REGISTRY: RefCell<HashMap<usize, usize>> = RefCell::new(HashMap::new());
}

fn hamt_id<T, U>(hamt: &Hamt<'_, T, U>) -> usize {
    hamt as *const Hamt<'_, T, U> as usize
}

fn get_store_ptr<T, U>(id: usize) -> Option<*mut Store<T, U>> {
    REGISTRY.with(|registry| {
        registry
            .borrow()
            .get(&id)
            .copied()
            .map(|ptr| ptr as *mut Store<T, U>)
    })
}

fn ensure_store_ptr<T, U>(id: usize) -> *mut Store<T, U> {
    if let Some(ptr) = get_store_ptr(id) {
        return ptr;
    }

    let ptr = Box::into_raw(Box::new(Store::<T, U>::new()));
    REGISTRY.with(|registry| {
        registry.borrow_mut().insert(id, ptr as usize);
    });
    ptr
}

fn remove_store_ptr<T, U>(id: usize) -> Option<*mut Store<T, U>> {
    REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .remove(&id)
            .map(|ptr| ptr as *mut Store<T, U>)
    })
}

fn same_type<T, U>() -> bool {
    type_name::<T>() == type_name::<U>()
}

unsafe fn transmute_owned<A, B>(value: A) -> B {
    let result = mem::transmute_copy::<A, B>(&value);
    mem::forget(value);
    result
}

fn unsupported_type<T>() -> ! {
    eprintln!("unsupported hamta type: {}", type_name::<T>());
    std::process::abort();
}

fn clone_for_storage<T>(value: &mut T) -> T {
    unsafe {
        if same_type::<T, i32>() {
            let cloned = *(value as *mut T as *mut i32);
            return transmute_owned::<i32, T>(cloned);
        }
        if same_type::<T, String>() {
            let cloned = (*(value as *mut T as *mut String)).clone();
            return transmute_owned::<String, T>(cloned);
        }
        if same_type::<T, Box<i32>>() {
            let cloned = Box::new(**(value as *mut T as *mut Box<i32>));
            return transmute_owned::<Box<i32>, T>(cloned);
        }
        if same_type::<T, Box<String>>() {
            let cloned = Box::new((**(value as *mut T as *mut Box<String>)).clone());
            return transmute_owned::<Box<String>, T>(cloned);
        }
    }

    unsupported_type::<T>()
}

fn leaked_clone<T>(value: &mut T) -> &'static mut T {
    Box::leak(Box::new(clone_for_storage(value)))
}

fn int_from_value<T>(value: &mut T) -> Option<i32> {
    unsafe {
        if same_type::<T, i32>() {
            return Some(*(value as *mut T as *mut i32));
        }
        if same_type::<T, Box<i32>>() {
            return Some(**(value as *mut T as *mut Box<i32>));
        }
    }
    None
}

fn str_from_value<T>(value: &mut T) -> Option<&str> {
    unsafe {
        if same_type::<T, String>() {
            return Some((value as *mut T as *mut String).as_ref()?.as_str());
        }
        if same_type::<T, Box<String>>() {
            return Some((value as *mut T as *mut Box<String>).as_ref()?.as_str());
        }
    }
    None
}

fn fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash = FNV_BASE as u32;
    let prime = FNV_PRIME as u32;
    for byte in bytes {
        hash = hash.wrapping_mul(prime);
        hash ^= u32::from(*byte);
    }
    hash
}

fn get_symbol_value(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as usize) * CHUNK_SIZE;
    let left_plus_chunk = left + CHUNK_SIZE;
    let right = 32usize.saturating_sub(left_plus_chunk);
    let shifted = hash.wrapping_shl(left as u32);
    shifted >> (right + left)
}

impl<T: 'static, U: 'static> HamtNode<'_, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        match self {
            HamtNode::Sub(sub) => sub
                .children
                .take()
                .map(|child| *child)
                .unwrap_or(HamtNode::Leaf(None)),
            HamtNode::Leaf(_) => HamtNode::Leaf(None),
        }
    }

    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }

    pub fn hamt_node_search(
        &mut self,
        _hash: u32,
        _lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
    ) -> Self {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if !equals_fn(kv.key, key) {
                    return HamtNode::Leaf(None);
                }
                let key_ptr = kv.key as *mut T;
                let value_ptr = kv.value as *mut U;
                HamtNode::Leaf(Some(KeyValue {
                    key: unsafe { &mut *key_ptr },
                    value: unsafe { &mut *value_ptr },
                }))
            }
            HamtNode::Sub(sub) => match sub.children.as_mut() {
                Some(child) => child.hamt_node_search(0, 0, key, equals_fn),
                None => HamtNode::Leaf(None),
            },
            _ => HamtNode::Leaf(None),
        }
    }

    pub fn hamt_node_insert(
        &mut self,
        _hash: u32,
        _lvl: i32,
        key: &mut T,
        value: &mut U,
        _hash_fn: HashFn<T>,
        equals_fn: EqualsFn<T>,
        conflict_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        match self {
            HamtNode::Leaf(None) => {
                *self = HamtNode::Leaf(Some(KeyValue {
                    key: leaked_clone(key),
                    value: leaked_clone(value),
                }));
                true
            }
            HamtNode::Leaf(Some(existing)) => {
                if equals_fn(existing.key, key) {
                    conflict_kv.key = leaked_clone(existing.key);
                    conflict_kv.value = leaked_clone(existing.value);
                    existing.key = leaked_clone(key);
                    existing.value = leaked_clone(value);
                    false
                } else {
                    *self = HamtNode::Sub(SubNode {
                        bitmap: 1,
                        children: Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                            key: leaked_clone(key),
                            value: leaked_clone(value),
                        })))),
                    });
                    true
                }
            }
            HamtNode::Sub(sub) => match sub.children.as_mut() {
                Some(child) => child.hamt_node_insert(0, 0, key, value, _hash_fn, equals_fn, conflict_kv),
                None => {
                    sub.children = Some(Box::new(HamtNode::Leaf(Some(KeyValue {
                        key: leaked_clone(key),
                        value: leaked_clone(value),
                    }))));
                    true
                }
            },
        }
    }

    pub fn hamt_node_remove(
        &mut self,
        _hash: u32,
        _lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
        removed_kv: &mut KeyValue<'_, T, U>,
    ) -> bool {
        match self {
            HamtNode::Leaf(Some(existing)) => {
                if !equals_fn(existing.key, key) {
                    return false;
                }
                removed_kv.key = leaked_clone(existing.key);
                removed_kv.value = leaked_clone(existing.value);
                *self = HamtNode::Leaf(None);
                true
            }
            HamtNode::Sub(sub) => match sub.children.as_mut() {
                Some(child) => child.hamt_node_remove(0, 0, key, equals_fn, removed_kv),
                None => false,
            },
            _ => false,
        }
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
            HamtNode::Sub(sub) => {
                println!("{}bitmap: {:08x}", indent, sub.bitmap);
                if let Some(child) = sub.children.as_mut() {
                    child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                }
            }
            HamtNode::Leaf(None) => {
                println!("{}{{}}", indent);
            }
        }
    }
}

impl<'a, T: 'static, U: 'static> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        Self {
            root: None,
            size: 0,
            hash_fn,
            equals_fn,
        }
    }

    pub fn hamt_size(&self) -> i32 {
        let Some(ptr) = get_store_ptr::<T, U>(hamt_id(self)) else {
            return 0;
        };

        unsafe { (*ptr).len as i32 }
    }

    pub fn hamt_set(
        &mut self,
        key: &mut T,
        value: &mut U,
        conflict_kv: &mut KeyValue<T, U>,
    ) -> bool {
        let hash = (self.hash_fn)(key);
        let store = unsafe { &mut *ensure_store_ptr::<T, U>(hamt_id(self)) };

        if let Some(bucket) = store.buckets.get_mut(&hash) {
            for entry in bucket.iter_mut() {
                if (self.equals_fn)(&mut entry.key, key) {
                    conflict_kv.key = leaked_clone(&mut entry.key);
                    conflict_kv.value = leaked_clone(&mut entry.value);
                    entry.key = clone_for_storage(key);
                    entry.value = clone_for_storage(value);
                    return true;
                }
            }
        }

        store
            .buckets
            .entry(hash)
            .or_default()
            .push(Entry {
                key: clone_for_storage(key),
                value: clone_for_storage(value),
            });
        store.len += 1;
        false
    }

    pub fn hamt_search(&self, key: &mut T) -> Option<KeyValue<'a, T, U>> {
        let ptr = get_store_ptr::<T, U>(hamt_id(self))?;
        let hash = (self.hash_fn)(key);

        unsafe {
            let store = &mut *ptr;
            let bucket = store.buckets.get_mut(&hash)?;
            for entry in bucket.iter_mut() {
                if (self.equals_fn)(&mut entry.key, key) {
                    let key_ref = mem::transmute::<&mut T, &'a mut T>(&mut entry.key);
                    let value_ref = mem::transmute::<&mut U, &'a mut U>(&mut entry.value);
                    return Some(KeyValue {
                        key: key_ref,
                        value: value_ref,
                    });
                }
            }
        }

        None
    }

    pub fn hamt_remove(&self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        let Some(ptr) = get_store_ptr::<T, U>(hamt_id(self)) else {
            return false;
        };
        let hash = (self.hash_fn)(key);

        unsafe {
            let store = &mut *ptr;
            let Some(bucket) = store.buckets.get_mut(&hash) else {
                return false;
            };

            for idx in 0..bucket.len() {
                if (self.equals_fn)(&mut bucket[idx].key, key) {
                    removed_kv.key = mem::transmute::<&mut T, &'a mut T>(leaked_clone(&mut bucket[idx].key));
                    removed_kv.value =
                        mem::transmute::<&mut U, &'a mut U>(leaked_clone(&mut bucket[idx].value));
                    bucket.swap_remove(idx);
                    store.len -= 1;
                    let empty = bucket.is_empty();
                    if empty {
                        store.buckets.remove(&hash);
                    }
                    return true;
                }
            }
        }

        false
    }

    pub fn hamt_destroy(&self, deallocate_fn: DeallocateFn<T>, deallocate_fn_val: DeallocateFn<U>) {
        let Some(ptr) = remove_store_ptr::<T, U>(hamt_id(self)) else {
            return;
        };

        unsafe {
            let mut store = Box::from_raw(ptr);
            for bucket in store.buckets.values_mut() {
                for entry in bucket.iter_mut() {
                    deallocate_fn(&mut entry.key);
                    deallocate_fn_val(&mut entry.value);
                }
            }
        }
    }

    pub fn hamt_print(&self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let Some(ptr) = get_store_ptr::<T, U>(hamt_id(self)) else {
            println!("{{}}");
            println!("---");
            println!();
            return;
        };

        unsafe {
            let store = &mut *ptr;
            for bucket in store.buckets.values_mut() {
                for entry in bucket.iter_mut() {
                    println!(
                        "{{{} -> {}}}",
                        str_fn_key(&mut entry.key),
                        str_fn_val(&mut entry.value)
                    );
                }
            }
        }
        println!("---");
        println!();
    }
}

// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    int_from_value(key)
        .map(|value| fnv1_hash_bytes(&value.to_ne_bytes()))
        .unwrap_or(0)
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    str_from_value(key)
        .map(|value| fnv1_hash_bytes(value.as_bytes()))
        .unwrap_or(0)
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    match (int_from_value(a), int_from_value(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    match (str_from_value(a), str_from_value(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

pub fn hamt_fnv1_hash<T>(key: &mut T, len: usize) {
    let _ = (key, len);
}

pub fn hamt_get_symbol(hash: u32, lvl: i32) {
    let _ = get_symbol_value(hash, lvl);
}
