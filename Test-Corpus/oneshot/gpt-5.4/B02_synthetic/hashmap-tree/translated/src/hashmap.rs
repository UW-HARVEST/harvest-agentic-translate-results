use std::collections::HashMap;
use std::ffi::c_void;

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

pub type TreeId = u64;
pub type tree_id_t = TreeId;

#[derive(Clone, Copy)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: *mut c_void,
    pub occupied: i32,
    pub deleted: i32,
}

pub struct hashmap_t {
    map: HashMap<tree_id_t, *mut c_void>,
}

pub fn hashmap_create() -> Box<hashmap_t> {
    Box::new(hashmap_t {
        map: HashMap::with_capacity(HASHMAP_INITIAL_CAPACITY),
    })
}

pub fn hashmap_destroy(_map: Box<hashmap_t>) {}

pub fn hashmap_put(map: &mut hashmap_t, key: tree_id_t, value: *mut c_void) -> i32 {
    map.map.insert(key, value);
    0
}

pub fn hashmap_get(map: &hashmap_t, key: tree_id_t) -> *mut c_void {
    map.map.get(&key).copied().unwrap_or(std::ptr::null_mut())
}

pub fn hashmap_remove(map: &mut hashmap_t, key: tree_id_t) -> *mut c_void {
    map.map.remove(&key).unwrap_or(std::ptr::null_mut())
}

pub fn hashmap_contains(map: &hashmap_t, key: tree_id_t) -> i32 {
    if map.map.contains_key(&key) { 1 } else { 0 }
}

pub fn hashmap_size(map: &hashmap_t) -> usize {
    map.map.len()
}

pub fn hashmap_clear(map: &mut hashmap_t) {
    map.map.clear();
}
