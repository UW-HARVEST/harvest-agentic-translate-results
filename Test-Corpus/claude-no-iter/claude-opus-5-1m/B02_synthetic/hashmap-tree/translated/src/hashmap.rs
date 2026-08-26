// hashmap.rs - translation of hashmap.c
//
// Open-addressing hashmap with FNV-1a hash, linear probing, deletion tombstones,
// and 0.75 load factor doubling. Values are heap-allocated tree nodes,
// represented in safe Rust as `Box<TreeNode>` raw pointers stored as `*mut TreeNode`.

use crate::tree::TreeNode;

pub type TreeId = u64;

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

#[derive(Clone)]
pub struct HashmapEntry {
    pub key: TreeId,
    pub value: *mut TreeNode,
    pub occupied: bool,
    pub deleted: bool,
}

impl HashmapEntry {
    fn empty() -> Self {
        HashmapEntry {
            key: 0,
            value: std::ptr::null_mut(),
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
    // FNV-1a hash, byte order matches C (little-endian raw bytes of u64)
    let mut hash: u64 = 14695981039346656037u64;
    let bytes = key.to_ne_bytes();
    for i in 0..8 {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

impl Hashmap {
    pub fn create() -> Box<Hashmap> {
        Box::new(Hashmap {
            entries: vec![HashmapEntry::empty(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        })
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let old_entries = std::mem::take(&mut self.entries);

        self.capacity *= 2;
        self.entries = vec![HashmapEntry::empty(); self.capacity];
        self.size = 0;
        self.deleted_count = 0;

        for i in 0..old_capacity {
            if old_entries[i].occupied && !old_entries[i].deleted {
                self.put(old_entries[i].key, old_entries[i].value);
            }
        }
        0
    }

    pub fn put(&mut self, key: TreeId, value: *mut TreeNode) -> i32 {
        if self.should_resize() {
            if self.resize() != 0 {
                return -1;
            }
        }

        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if self.entries[current].key == key {
                self.entries[current].value = value;
                return 0;
            }

            probe += 1;
        }

        -1
    }

    pub fn get(&self, key: TreeId) -> *mut TreeNode {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return std::ptr::null_mut();
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value;
            }

            probe += 1;
        }

        std::ptr::null_mut()
    }

    pub fn remove(&mut self, key: TreeId) -> *mut TreeNode {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return std::ptr::null_mut();
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value;
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return value;
            }

            probe += 1;
        }

        std::ptr::null_mut()
    }

    pub fn contains(&self, key: TreeId) -> bool {
        !self.get(key).is_null()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.occupied = false;
            entry.deleted = false;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
}
