//! Translation of c_src/src/hashmap.c
//!
//! Memory layout of `hashmap_t` and `hashmap_entry_t` matches the C structs so
//! callers (including `tree.c`-style consumers) can directly access fields like
//! `map->entries[i].occupied` if they peek into the struct.

use core::ffi::c_void;
use core::ptr;
use libc::{c_int, calloc, free, malloc, size_t};

pub type tree_id_t = u64;

pub const HASHMAP_INITIAL_CAPACITY: size_t = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

#[repr(C)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: size_t,
    pub size: size_t,
    pub deleted_count: size_t,
}

fn hash_function(key: tree_id_t) -> u64 {
    // FNV-1a hash, matching C bytewise iteration over the little-endian
    // representation of `key` (this is what reading bytes via a pointer cast
    // does on the little-endian x86_64/aarch64 targets used at Amazon).
    let mut hash: u64 = 14_695_981_039_346_656_037u64;
    let bytes = key.to_le_bytes();
    for &b in bytes.iter() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1_099_511_628_211u64);
    }
    hash
}

unsafe fn should_resize(map: *const hashmap_t) -> bool {
    let m = &*map;
    let load = (m.size + m.deleted_count) as f64 / m.capacity as f64;
    load > HASHMAP_LOAD_FACTOR
}

unsafe fn hashmap_resize_internal(map: *mut hashmap_t) -> c_int {
    let m = &mut *map;
    let old_capacity = m.capacity;
    let old_entries = m.entries;

    // Double capacity
    m.capacity *= 2;
    m.entries = calloc(m.capacity, core::mem::size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;
    if m.entries.is_null() {
        m.entries = old_entries;
        m.capacity = old_capacity;
        return -1;
    }

    m.size = 0;
    m.deleted_count = 0;

    // Rehash all entries
    for i in 0..old_capacity {
        let e = &*old_entries.add(i);
        if e.occupied != 0 && e.deleted == 0 {
            hashmap_put(map, e.key, e.value);
        }
    }

    free(old_entries as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_create() -> *mut hashmap_t {
    let map = malloc(core::mem::size_of::<hashmap_t>()) as *mut hashmap_t;
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &mut *map;
    m.capacity = HASHMAP_INITIAL_CAPACITY;
    m.size = 0;
    m.deleted_count = 0;
    m.entries =
        calloc(m.capacity, core::mem::size_of::<hashmap_entry_t>()) as *mut hashmap_entry_t;

    if m.entries.is_null() {
        free(map as *mut c_void);
        return ptr::null_mut();
    }

    map
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_destroy(map: *mut hashmap_t) {
    if !map.is_null() {
        let m = &mut *map;
        free(m.entries as *mut c_void);
        free(map as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_put(
    map: *mut hashmap_t,
    key: tree_id_t,
    value: *mut c_void,
) -> c_int {
    if map.is_null() {
        return -1;
    }

    if should_resize(map) {
        if hashmap_resize_internal(map) != 0 {
            return -1;
        }
    }

    let m = &mut *map;
    let hash = hash_function(key);
    let index = (hash as size_t) % m.capacity;
    let mut probe: size_t = 0;

    // Linear probing
    while probe < m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &mut *m.entries.add(current);

        if entry.occupied == 0 {
            // Empty slot
            entry.key = key;
            entry.value = value;
            entry.occupied = 1;
            entry.deleted = 0;
            m.size += 1;
            return 0;
        } else if entry.deleted != 0 {
            // Reuse deleted slot
            entry.key = key;
            entry.value = value;
            entry.deleted = 0;
            m.size += 1;
            m.deleted_count -= 1;
            return 0;
        } else if entry.key == key {
            // Update existing
            entry.value = value;
            return 0;
        }

        probe += 1;
    }

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_get(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &mut *map;
    let hash = hash_function(key);
    let index = (hash as size_t) % m.capacity;
    let mut probe: size_t = 0;

    while probe < m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &*m.entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }

        if entry.deleted == 0 && entry.key == key {
            return entry.value;
        }

        probe += 1;
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_remove(map: *mut hashmap_t, key: tree_id_t) -> *mut c_void {
    if map.is_null() {
        return ptr::null_mut();
    }

    let m = &mut *map;
    let hash = hash_function(key);
    let index = (hash as size_t) % m.capacity;
    let mut probe: size_t = 0;

    while probe < m.capacity {
        let current = (index + probe) % m.capacity;
        let entry = &mut *m.entries.add(current);

        if entry.occupied == 0 {
            return ptr::null_mut();
        }

        if entry.deleted == 0 && entry.key == key {
            let value = entry.value;
            entry.deleted = 1;
            m.size -= 1;
            m.deleted_count += 1;
            return value;
        }

        probe += 1;
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_contains(map: *mut hashmap_t, key: tree_id_t) -> c_int {
    if hashmap_get(map, key).is_null() {
        0
    } else {
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_size(map: *mut hashmap_t) -> size_t {
    if map.is_null() {
        0
    } else {
        (*map).size
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hashmap_clear(map: *mut hashmap_t) {
    if map.is_null() {
        return;
    }

    let m = &mut *map;
    for i in 0..m.capacity {
        let entry = &mut *m.entries.add(i);
        entry.occupied = 0;
        entry.deleted = 0;
    }

    m.size = 0;
    m.deleted_count = 0;
}
