// hashmap.rs - faithful translation of hashmap.c

use std::ffi::c_void;
use std::ptr;

pub type TreeId = u64;

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

#[derive(Clone, Copy)]
pub struct HashmapEntry {
    pub key: TreeId,
    pub value: *mut c_void,
    pub occupied: i32,
    pub deleted: i32,
}

impl Default for HashmapEntry {
    fn default() -> Self {
        HashmapEntry {
            key: 0,
            value: ptr::null_mut(),
            occupied: 0,
            deleted: 0,
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
    let mut hash: u64 = 14695981039346656037u64;
    let bytes = key.to_le_bytes();
    for i in 0..8 {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

impl Hashmap {
    pub fn new() -> Self {
        Hashmap {
            entries: vec![HashmapEntry::default(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let old_entries = std::mem::take(&mut self.entries);

        self.capacity = old_capacity * 2;
        self.entries = vec![HashmapEntry::default(); self.capacity];
        self.size = 0;
        self.deleted_count = 0;

        for i in 0..old_capacity {
            if old_entries[i].occupied != 0 && old_entries[i].deleted == 0 {
                self.put(old_entries[i].key, old_entries[i].value);
            }
        }

        0
    }

    pub fn put(&mut self, key: TreeId, value: *mut c_void) -> i32 {
        if self.should_resize() {
            if self.resize() != 0 {
                return -1;
            }
        }

        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if self.entries[current].occupied == 0 {
                // Empty slot
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].occupied = 1;
                self.entries[current].deleted = 0;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted != 0 {
                // Reuse deleted slot
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].deleted = 0;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                // Update existing
                self.entries[current].value = value;
                return 0;
            }

            probe += 1;
        }

        -1
    }

    pub fn get(&self, key: TreeId) -> *mut c_void {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if self.entries[current].occupied == 0 {
                return ptr::null_mut();
            }

            if self.entries[current].deleted == 0 && self.entries[current].key == key {
                return self.entries[current].value;
            }

            probe += 1;
        }

        ptr::null_mut()
    }

    pub fn remove(&mut self, key: TreeId) -> *mut c_void {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if self.entries[current].occupied == 0 {
                return ptr::null_mut();
            }

            if self.entries[current].deleted == 0 && self.entries[current].key == key {
                let value = self.entries[current].value;
                self.entries[current].deleted = 1;
                self.size -= 1;
                self.deleted_count += 1;
                return value;
            }

            probe += 1;
        }

        ptr::null_mut()
    }

    pub fn contains(&self, key: TreeId) -> bool {
        !self.get(key).is_null()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for i in 0..self.capacity {
            self.entries[i].occupied = 0;
            self.entries[i].deleted = 0;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
}
