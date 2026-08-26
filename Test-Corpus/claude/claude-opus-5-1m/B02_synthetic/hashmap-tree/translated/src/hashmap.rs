//! Faithful translation of `c_src/src/hashmap.c` / `c_src/include/hashmap.h`.
//!
//! The C map stores `void *` values.  Here the map is generic over a `Copy`
//! value type; `None` plays the role of the C `NULL` value (which is what both
//! "slot never written" and "lookup miss" produce in the original code).

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

/// `typedef uint64_t tree_id_t;`
pub type TreeId = u64;

#[derive(Clone, Copy)]
pub struct HashmapEntry<V: Copy> {
    pub key: TreeId,
    pub value: Option<V>,
    pub occupied: i32,
    pub deleted: i32,
}

impl<V: Copy> HashmapEntry<V> {
    /// Equivalent of a `calloc`'d entry: all bytes zero.
    fn zeroed() -> Self {
        HashmapEntry {
            key: 0,
            value: None,
            occupied: 0,
            deleted: 0,
        }
    }
}

pub struct Hashmap<V: Copy> {
    pub entries: Vec<HashmapEntry<V>>,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

/// `static uint64_t hash_function(tree_id_t key)` -- FNV-1a over the raw bytes
/// of the key.  The original reads the bytes of the `uint64_t` in memory order,
/// i.e. little-endian on the reference platform.
fn hash_function(key: TreeId) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    let bytes = key.to_le_bytes();

    for i in 0..core::mem::size_of::<TreeId>() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    hash
}

impl<V: Copy> Hashmap<V> {
    /// `hashmap_t* hashmap_create(void)`
    pub fn create() -> Hashmap<V> {
        let capacity = HASHMAP_INITIAL_CAPACITY;
        Hashmap {
            entries: vec![HashmapEntry::zeroed(); capacity],
            capacity,
            size: 0,
            deleted_count: 0,
        }
    }

    /// `static int should_resize(hashmap_t *map)`
    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    /// `static int hashmap_resize(hashmap_t *map)`
    fn resize(&mut self) -> i32 {
        let old_capacity = self.capacity;
        let old_entries = core::mem::take(&mut self.entries);

        // Double capacity
        self.capacity *= 2;
        self.entries = vec![HashmapEntry::zeroed(); self.capacity];

        self.size = 0;
        self.deleted_count = 0;

        // Rehash all entries
        for i in 0..old_capacity {
            if old_entries[i].occupied != 0 && old_entries[i].deleted == 0 {
                let key = old_entries[i].key;
                let value = old_entries[i].value;
                match value {
                    Some(v) => {
                        self.put(key, v);
                    }
                    None => {
                        self.put_raw(key, None);
                    }
                }
            }
        }

        0
    }

    /// `int hashmap_put(hashmap_t *map, tree_id_t key, void *value)`
    pub fn put(&mut self, key: TreeId, value: V) -> i32 {
        self.put_raw(key, Some(value))
    }

    fn put_raw(&mut self, key: TreeId, value: Option<V>) -> i32 {
        if self.should_resize() && self.resize() != 0 {
            return -1;
        }

        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        // Linear probing
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

        -1 // Map is full (shouldn't happen with resizing)
    }

    /// `void* hashmap_get(hashmap_t *map, tree_id_t key)`
    pub fn get(&self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if self.entries[current].occupied == 0 {
                return None;
            }

            if self.entries[current].deleted == 0 && self.entries[current].key == key {
                return self.entries[current].value;
            }

            probe += 1;
        }

        None
    }

    /// `void* hashmap_remove(hashmap_t *map, tree_id_t key)`
    pub fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe: usize = 0;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if self.entries[current].occupied == 0 {
                return None;
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

        None
    }

    /// `int hashmap_contains(hashmap_t *map, tree_id_t key)`
    pub fn contains(&self, key: TreeId) -> i32 {
        if self.get(key).is_some() {
            1
        } else {
            0
        }
    }

    /// `size_t hashmap_size(hashmap_t *map)`
    pub fn size(&self) -> usize {
        self.size
    }

    /// `void hashmap_clear(hashmap_t *map)`
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
