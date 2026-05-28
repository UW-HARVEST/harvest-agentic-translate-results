// Translated from c_src/src/hashmap.c
// Open-addressing hash map with linear probing and tombstone deletion.

const HASHMAP_INITIAL_CAPACITY: usize = 16;
const HASHMAP_LOAD_FACTOR: f64 = 0.75;

pub type TreeId = u64;

pub struct Entry<V> {
    pub key: TreeId,
    pub value: Option<V>,
    pub occupied: bool,
    pub deleted: bool,
}

impl<V> Default for Entry<V> {
    fn default() -> Self {
        Entry {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

pub struct Hashmap<V> {
    entries: Vec<Entry<V>>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

fn hash_function(key: TreeId) -> u64 {
    // FNV-1a hash, byte-by-byte over the key bytes (native byte order, like C's
    // pointer-cast access).
    let mut hash: u64 = 14695981039346656037u64;
    let bytes = key.to_ne_bytes();
    for b in bytes.iter() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

impl<V> Hashmap<V> {
    pub fn new() -> Self {
        let capacity = HASHMAP_INITIAL_CAPACITY;
        let mut entries = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            entries.push(Entry::default());
        }
        Hashmap {
            entries,
            capacity,
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > HASHMAP_LOAD_FACTOR
    }

    fn resize(&mut self) {
        let new_capacity = self.capacity * 2;
        let mut new_entries: Vec<Entry<V>> = Vec::with_capacity(new_capacity);
        for _ in 0..new_capacity {
            new_entries.push(Entry::default());
        }
        let old_entries = std::mem::replace(&mut self.entries, new_entries);
        self.capacity = new_capacity;
        self.size = 0;
        self.deleted_count = 0;

        for entry in old_entries.into_iter() {
            if entry.occupied && !entry.deleted {
                if let Some(v) = entry.value {
                    self.put(entry.key, v);
                }
            }
        }
    }

    pub fn put(&mut self, key: TreeId, value: V) -> i32 {
        if self.should_resize() {
            self.resize();
        }

        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;

        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &mut self.entries[current];

            if !entry.occupied {
                entry.key = key;
                entry.value = Some(value);
                entry.occupied = true;
                entry.deleted = false;
                self.size += 1;
                return 0;
            } else if entry.deleted {
                entry.key = key;
                entry.value = Some(value);
                entry.deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if entry.key == key {
                entry.value = Some(value);
                return 0;
            }
        }

        -1
    }

    pub fn get(&self, key: TreeId) -> Option<&V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &self.entries[current];
            if !entry.occupied {
                return None;
            }
            if !entry.deleted && entry.key == key {
                return entry.value.as_ref();
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: TreeId) -> Option<&mut V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        let mut found_idx: Option<usize> = None;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &self.entries[current];
            if !entry.occupied {
                return None;
            }
            if !entry.deleted && entry.key == key {
                found_idx = Some(current);
                break;
            }
        }
        match found_idx {
            Some(i) => self.entries[i].value.as_mut(),
            None => None,
        }
    }

    pub fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = hash_function(key);
        let index = (hash as usize) % self.capacity;
        for probe in 0..self.capacity {
            let current = (index + probe) % self.capacity;
            let entry = &mut self.entries[current];
            if !entry.occupied {
                return None;
            }
            if !entry.deleted && entry.key == key {
                let value = entry.value.take();
                entry.deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return value;
            }
        }
        None
    }

    pub fn contains(&self, key: TreeId) -> bool {
        self.get(key).is_some()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.occupied = false;
            entry.deleted = false;
            entry.value = None;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
}
