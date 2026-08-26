// hashmap.rs - Rust translation of hashmap.c/.h
use std::ffi::c_void;
use std::ptr;

pub type TreeId = u64;

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

#[derive(Clone, Copy)]
pub struct HashmapEntry {
    pub key: TreeId,
    pub value: *mut c_void,
    pub occupied: bool,
    pub deleted: bool,
}

impl Default for HashmapEntry {
    fn default() -> Self {
        HashmapEntry {
            key: 0,
            value: ptr::null_mut(),
            occupied: false,
            deleted: false,
        }
    }
}

pub struct Hashmap {
    pub entries: Vec<HashmapEntry>,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 14695981039346656037;
    let bytes = key.to_ne_bytes();
    for b in bytes.iter() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

pub fn hashmap_create() -> Box<Hashmap> {
    let cap = HASHMAP_INITIAL_CAPACITY;
    Box::new(Hashmap {
        entries: vec![HashmapEntry::default(); cap],
        capacity: cap,
        size: 0,
        deleted_count: 0,
    })
}

pub fn hashmap_destroy(map: Box<Hashmap>) {
    drop(map);
}

fn should_resize(map: &Hashmap) -> bool {
    let load = (map.size + map.deleted_count) as f64 / map.capacity as f64;
    load > HASHMAP_LOAD_FACTOR
}

fn hashmap_resize(map: &mut Hashmap) -> i32 {
    let old_capacity = map.capacity;
    let old_entries = std::mem::take(&mut map.entries);

    map.capacity = old_capacity * 2;
    map.entries = vec![HashmapEntry::default(); map.capacity];
    map.size = 0;
    map.deleted_count = 0;

    for i in 0..old_capacity {
        if old_entries[i].occupied && !old_entries[i].deleted {
            hashmap_put(map, old_entries[i].key, old_entries[i].value);
        }
    }
    0
}

pub fn hashmap_put(map: &mut Hashmap, key: TreeId, value: *mut c_void) -> i32 {
    if should_resize(map) {
        if hashmap_resize(map) != 0 {
            return -1;
        }
    }

    let hash = hash_function(key);
    let index = (hash as usize) % map.capacity;

    for probe in 0..map.capacity {
        let current = (index + probe) % map.capacity;

        if !map.entries[current].occupied {
            map.entries[current].key = key;
            map.entries[current].value = value;
            map.entries[current].occupied = true;
            map.entries[current].deleted = false;
            map.size += 1;
            return 0;
        } else if map.entries[current].deleted {
            map.entries[current].key = key;
            map.entries[current].value = value;
            map.entries[current].deleted = false;
            map.size += 1;
            map.deleted_count -= 1;
            return 0;
        } else if map.entries[current].key == key {
            map.entries[current].value = value;
            return 0;
        }
    }
    -1
}

pub fn hashmap_get(map: &Hashmap, key: TreeId) -> *mut c_void {
    let hash = hash_function(key);
    let index = (hash as usize) % map.capacity;

    for probe in 0..map.capacity {
        let current = (index + probe) % map.capacity;
        if !map.entries[current].occupied {
            return ptr::null_mut();
        }
        if !map.entries[current].deleted && map.entries[current].key == key {
            return map.entries[current].value;
        }
    }
    ptr::null_mut()
}

pub fn hashmap_remove(map: &mut Hashmap, key: TreeId) -> *mut c_void {
    let hash = hash_function(key);
    let index = (hash as usize) % map.capacity;

    for probe in 0..map.capacity {
        let current = (index + probe) % map.capacity;
        if !map.entries[current].occupied {
            return ptr::null_mut();
        }
        if !map.entries[current].deleted && map.entries[current].key == key {
            let value = map.entries[current].value;
            map.entries[current].deleted = true;
            map.size -= 1;
            map.deleted_count += 1;
            return value;
        }
    }
    ptr::null_mut()
}

pub fn hashmap_contains(map: &Hashmap, key: TreeId) -> bool {
    !hashmap_get(map, key).is_null()
}

pub fn hashmap_size(map: &Hashmap) -> usize {
    map.size
}

pub fn hashmap_clear(map: &mut Hashmap) {
    for entry in map.entries.iter_mut() {
        entry.occupied = false;
        entry.deleted = false;
    }
    map.size = 0;
    map.deleted_count = 0;
}
