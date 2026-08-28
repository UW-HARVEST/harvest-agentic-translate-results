//! Faithful translation of c_src/src/hashmap.c and c_src/include/hashmap.h.
//!
//! The C hashmap stores `void *` values; here the map is generic over a `Copy`
//! value type and `Option<V>` models the nullable pointer (`None` == `NULL`).

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

pub type TreeId = u64;

#[derive(Clone, Copy)]
pub struct HashmapEntry<V: Copy> {
    pub key: TreeId,
    pub value: Option<V>,
    pub occupied: bool,
    pub deleted: bool,
}

impl<V: Copy> HashmapEntry<V> {
    /// calloc() zeroes the entry: key 0, NULL value, flags cleared.
    const fn zeroed() -> Self {
        HashmapEntry {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

pub struct Hashmap<V: Copy> {
    pub entries: Vec<HashmapEntry<V>>,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    // FNV-1a hash over the raw bytes of the key, in host byte order.
    let mut hash: u64 = 14695981039346656037;
    let bytes = key.to_ne_bytes();

    for i in 0..std::mem::size_of::<TreeId>() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    hash
}

impl<V: Copy> Hashmap<V> {
    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let old_entries = std::mem::take(&mut self.entries);

        // Double capacity
        self.capacity *= 2;
        self.entries = vec![HashmapEntry::zeroed(); self.capacity];

        self.size = 0;
        self.deleted_count = 0;

        // Rehash all entries
        for i in 0..old_capacity {
            if old_entries[i].occupied && !old_entries[i].deleted {
                self.put_value(old_entries[i].key, old_entries[i].value);
            }
        }

        0
    }

    /// hashmap_create()
    pub fn create() -> Self {
        Hashmap {
            entries: vec![HashmapEntry::zeroed(); HASHMAP_INITIAL_CAPACITY],
            capacity: HASHMAP_INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }

    /// hashmap_put()
    pub fn put(&mut self, key: TreeId, value: V) -> i32 {
        self.put_value(key, Some(value))
    }

    fn put_value(&mut self, key: TreeId, value: Option<V>) -> i32 {
        if self.should_resize() {
            if self.resize() != 0 {
                return -1;
            }
        }

        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        // Linear probing
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                // Empty slot
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return 0;
            } else if self.entries[current].deleted {
                // Reuse deleted slot
                self.entries[current].key = key;
                self.entries[current].value = value;
                self.entries[current].deleted = false;
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

        -1 // Map is full (shouldn't happen with resizing)
    }

    /// hashmap_get()
    pub fn get(&self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return None;
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value;
            }

            probe += 1;
        }

        None
    }

    /// hashmap_remove()
    pub fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return None;
            }

            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value;
                self.entries[current].deleted = true;
                self.size = self.size.wrapping_sub(1);
                self.deleted_count += 1;
                return value;
            }

            probe += 1;
        }

        None
    }

    /// hashmap_contains()
    pub fn contains(&self, key: TreeId) -> i32 {
        if self.get(key).is_some() {
            1
        } else {
            0
        }
    }

    /// hashmap_size()
    pub fn size(&self) -> usize {
        self.size
    }

    /// hashmap_clear()
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for i in 0..self.capacity {
            self.entries[i].occupied = false;
            self.entries[i].deleted = false;
        }

        self.size = 0;
        self.deleted_count = 0;
    }
}
