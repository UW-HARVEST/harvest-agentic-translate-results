//! Faithful translation of `c_src/src/hashmap.c` / `c_src/include/hashmap.h`.
//!
//! The C structures are part of the public header, so consumers observe both
//! their layout and the exact `void *` values stored in them.  The translation
//! therefore keeps the original `#[repr(C)]` layout, the original
//! `malloc`/`calloc`/`free` allocation behaviour and the original pointer
//! semantics -- including the places where the C code leaves memory
//! uninitialised.

#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};
use core::mem::size_of;
use core::ptr;

/// `#define HASHMAP_INITIAL_CAPACITY 16`
pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
/// `#define HASHMAP_LOAD_FACTOR 0.75`
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

/// `typedef uint64_t tree_id_t;`
pub type tree_id_t = u64;

/// ```c
/// typedef struct hashmap_entry {
///     tree_id_t key;
///     void *value;
///     int occupied;
///     int deleted;
/// } hashmap_entry_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

/// ```c
/// typedef struct {
///     hashmap_entry_t *entries;
///     size_t capacity;
///     size_t size;
///     size_t deleted_count;
/// } hashmap_t;
/// ```
#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

// Layout parity with the reference platform (x86-64 LP64), verified against
// `sizeof`/`offsetof` of the C compiler.
const _: () = assert!(size_of::<hashmap_entry_t>() == 24);
const _: () = assert!(size_of::<hashmap_t>() == 32);

/// ```c
/// static uint64_t hash_function(tree_id_t key) {
///     // FNV-1a hash
///     uint64_t hash = 14695981039346656037ULL;
///     uint8_t *bytes = (uint8_t *)&key;
///     for (size_t i = 0; i < sizeof(tree_id_t); i++) {
///         hash ^= bytes[i];
///         hash *= 1099511628211ULL;
///     }
///     return hash;
/// }
/// ```
///
/// The bytes are read out of the key's storage in memory order, exactly as the
/// `uint8_t *` alias does.
fn hash_function(key: tree_id_t) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    let bytes = &key as *const tree_id_t as *const u8;

    for i in 0..size_of::<tree_id_t>() {
        hash ^= unsafe { *bytes.add(i) } as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    hash
}

/// ```c
/// static int should_resize(hashmap_t *map) {
///     double load = (double)(map->size + map->deleted_count) / map->capacity;
///     return load > HASHMAP_LOAD_FACTOR;
/// }
/// ```
unsafe fn should_resize(map: *mut hashmap_t) -> c_int {
    let load = (*map).size.wrapping_add((*map).deleted_count) as f64 / (*map).capacity as f64;
    (load > HASHMAP_LOAD_FACTOR) as c_int
}

/// `static int hashmap_resize(hashmap_t *map)`
unsafe fn hashmap_resize(map: *mut hashmap_t) -> c_int {
    let old_capacity = (*map).capacity;
    let old_entries = (*map).entries;

    // Double capacity
    (*map).capacity = old_capacity.wrapping_mul(2);
    (*map).entries =
        libc::calloc((*map).capacity, size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;
    if (*map).entries.is_null() {
        (*map).entries = old_entries;
        (*map).capacity = old_capacity;
        return -1;
    }

    (*map).size = 0;
    (*map).deleted_count = 0;

    // Rehash all entries
    let mut i: usize = 0;
    while i < old_capacity {
        let e = old_entries.add(i);
        if (*e).occupied != 0 && (*e).deleted == 0 {
            hashmap_put(map, (*e).key, (*e).value);
        }
        i += 1;
    }

    libc::free(old_entries as *mut c_void);
    0
}

/// `hashmap_t* hashmap_create(void)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_create() -> *mut hashmap_t {
    let map = libc::malloc(size_of::<hashmap_t>()) as *mut hashmap_t;
    if map.is_null() {
        return ptr::null_mut();
    }

    (*map).capacity = HASHMAP_INITIAL_CAPACITY;
    (*map).size = 0;
    (*map).deleted_count = 0;
    (*map).entries =
        libc::calloc((*map).capacity, size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;

    if (*map).entries.is_null() {
        libc::free(map as *mut c_void);
        return ptr::null_mut();
    }

    map
}

/// `void hashmap_destroy(hashmap_t *map)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(map: *mut hashmap_t) {
    if !map.is_null() {
        libc::free((*map).entries as *mut c_void);
        libc::free(map as *mut c_void);
    }
}

/// `int hashmap_put(hashmap_t *map, tree_id_t key, void *value)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_put(
    map: *mut hashmap_t,
    key: tree_id_t,
    value: *mut c_void,
) -> c_int {
    if map.is_null() {
        return -1;
    }

    if should_resize(map) != 0 && hashmap_resize(map) != 0 {
        return -1;
    }

    let hash = hash_function(key);
    let index = (hash % (*map).capacity as u64) as usize;
    let mut probe: usize = 0;

    // Linear probing
    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let e = (*map).entries.add(current);

        if (*e).occupied == 0 {
            // Empty slot
            (*e).key = key;
            (*e).value = value;
            (*e).occupied = 1;
            (*e).deleted = 0;
            (*map).size = (*map).size.wrapping_add(1);
            return 0;
        } else if (*e).deleted != 0 {
            // Reuse deleted slot
            (*e).key = key;
            (*e).value = value;
            (*e).deleted = 0;
            (*map).size = (*map).size.wrapping_add(1);
            // C wraps a `size_t` here; never underflows through the public API,
            // but a hand-built `hashmap_t` can make it happen.
            (*map).deleted_count = (*map).deleted_count.wrapping_sub(1);
            return 0;
        } else if (*e).key == key {
            // Update existing
            (*e).value = value;
            return 0;
        }

        probe += 1;
    }

    -1 // Map is full (shouldn't happen with resizing)
}

/// `void* hashmap_get(hashmap_t *map, tree_id_t key)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_get(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let hash = hash_function(key);
    let index = (hash % (*map).capacity as u64) as usize;
    let mut probe: usize = 0;

    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let e = (*map).entries.add(current);

        if (*e).occupied == 0 {
            return ptr::null_mut();
        }

        if (*e).deleted == 0 && (*e).key == key {
            return (*e).value;
        }

        probe += 1;
    }

    ptr::null_mut()
}

/// `void* hashmap_remove(hashmap_t *map, tree_id_t key)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let hash = hash_function(key);
    let index = (hash % (*map).capacity as u64) as usize;
    let mut probe: usize = 0;

    while probe < (*map).capacity {
        let current = (index + probe) % (*map).capacity;
        let e = (*map).entries.add(current);

        if (*e).occupied == 0 {
            return ptr::null_mut();
        }

        if (*e).deleted == 0 && (*e).key == key {
            let value = (*e).value;
            (*e).deleted = 1;
            (*map).size = (*map).size.wrapping_sub(1);
            (*map).deleted_count = (*map).deleted_count.wrapping_add(1);
            return value;
        }

        probe += 1;
    }

    ptr::null_mut()
}

/// `int hashmap_contains(hashmap_t *map, tree_id_t key)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(map: *mut hashmap_t, key: tree_id_t) -> c_int {
    (!hashmap_get(map, key).is_null()) as c_int
}

/// `size_t hashmap_size(hashmap_t *map)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_size(map: *mut hashmap_t) -> usize {
    if !map.is_null() {
        (*map).size
    } else {
        0
    }
}

/// `void hashmap_clear(hashmap_t *map)`
#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(map: *mut hashmap_t) {
    if map.is_null() {
        return;
    }

    let mut i: usize = 0;
    while i < (*map).capacity {
        let e = (*map).entries.add(i);
        (*e).occupied = 0;
        (*e).deleted = 0;
        i += 1;
    }

    (*map).size = 0;
    (*map).deleted_count = 0;
}
