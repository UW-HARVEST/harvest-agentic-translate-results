// hashmap.rs
//
// Faithful translation of c_src/src/hashmap.c and c_src/include/hashmap.h.
//
// The C code stores `void *` values in an open-addressed table using linear
// probing with tombstones. In Rust we keep the same table layout and probing
// logic, but the payload is a generic `usize` handle (an index into an
// arena owned by the caller). `Option<usize>` stands in for the C `void *`,
// with `None` playing the role of `NULL`.

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const HASHMAP_LOAD_FACTOR: f64 = 0.75;

pub type TreeId = u64;

/// Mirrors `hashmap_entry_t`.
#[derive(Clone, Copy)]
pub struct HashmapEntry {
    pub key: TreeId,
    pub value: Option<usize>,
    pub occupied: bool,
    pub deleted: bool,
}

impl HashmapEntry {
    /// Equivalent to the zeroed entry produced by `calloc`.
    const fn zeroed() -> Self {
        HashmapEntry {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

/// Mirrors `hashmap_t`.
pub struct Hashmap {
    pub entries: Vec<HashmapEntry>,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

/// FNV-1a over the raw bytes of the key, exactly as in `hash_function`.
/// The C code casts `&key` to `uint8_t *`, so the byte order is the host's
/// (little-endian on the reference platform).
fn hash_function(key: TreeId) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    let bytes = key.to_le_bytes();

    for i in 0..core::mem::size_of::<TreeId>() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    hash
}

impl Hashmap {
    /// `hashmap_create`
    pub fn create() -> Hashmap {
        let capacity = HASHMAP_INITIAL_CAPACITY;
        Hashmap {
            entries: vec![HashmapEntry::zeroed(); capacity],
            capacity,
            size: 0,
            deleted_count: 0,
        }
    }

    /// `should_resize`
    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    /// `hashmap_resize`. Allocation cannot fail here, so this always
    /// corresponds to the success path (return 0) of the C function.
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
            if old_entries[i].occupied && !old_entries[i].deleted {
                self.put(old_entries[i].key, old_entries[i].value);
            }
        }

        0
    }

    /// `hashmap_put`
    pub fn put(&mut self, key: TreeId, value: Option<usize>) -> i32 {
        if self.should_resize() && self.resize() != 0 {
            return -1;
        }

        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

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

    /// `hashmap_get`
    pub fn get(&self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

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

    /// `hashmap_remove`
    pub fn remove(&mut self, key: TreeId) -> Option<usize> {
        let hash = hash_function(key);
        let index = (hash % self.capacity as u64) as usize;
        let mut probe = 0usize;

        while probe < self.capacity {
            let current = (index + probe) % self.capacity;

            if !self.entries[current].occupied {
                return None;
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

        None
    }

    /// `hashmap_contains`
    pub fn contains(&self, key: TreeId) -> i32 {
        i32::from(self.get(key).is_some())
    }

    /// `hashmap_size`
    pub fn size(&self) -> usize {
        self.size
    }

    /// `hashmap_clear`
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
