const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f64 = 0.75;

struct Entry<V> {
    key: u64,
    value: Option<V>,
    occupied: bool,
    deleted: bool,
}

impl<V> Entry<V> {
    fn empty() -> Self {
        Self {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

pub struct HashMap<V> {
    entries: Vec<Entry<V>>,
    size: usize,
    deleted_count: usize,
}

impl<V> HashMap<V> {
    pub fn new() -> Self {
        Self {
            entries: Self::empty_entries(INITIAL_CAPACITY),
            size: 0,
            deleted_count: 0,
        }
    }

    fn empty_entries(capacity: usize) -> Vec<Entry<V>> {
        std::iter::repeat_with(Entry::empty)
            .take(capacity)
            .collect()
    }

    fn hash(key: u64) -> u64 {
        let mut hash = 14_695_981_039_346_656_037_u64;
        for byte in key.to_ne_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        hash
    }

    fn should_resize(&self) -> bool {
        (self.size + self.deleted_count) as f64 / self.entries.len() as f64 > LOAD_FACTOR
    }

    fn resize(&mut self) {
        let new_capacity = self.entries.len() * 2;
        let old_entries = std::mem::replace(&mut self.entries, Self::empty_entries(new_capacity));
        self.size = 0;
        self.deleted_count = 0;

        for mut entry in old_entries {
            if entry.occupied && !entry.deleted {
                self.put(entry.key, entry.value.take().unwrap());
            }
        }
    }

    pub fn put(&mut self, key: u64, value: V) -> i32 {
        if self.should_resize() {
            self.resize();
        }

        let capacity = self.entries.len();
        let index = Self::hash(key) as usize % capacity;
        let mut value = Some(value);

        for probe in 0..capacity {
            let current = (index + probe) % capacity;
            let entry = &mut self.entries[current];

            if !entry.occupied {
                entry.key = key;
                entry.value = value.take();
                entry.occupied = true;
                entry.deleted = false;
                self.size += 1;
                return 0;
            } else if entry.deleted {
                entry.key = key;
                entry.value = value.take();
                entry.deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return 0;
            } else if entry.key == key {
                entry.value = value.take();
                return 0;
            }
        }

        -1
    }

    pub fn get(&self, key: u64) -> Option<&V> {
        let capacity = self.entries.len();
        let index = Self::hash(key) as usize % capacity;

        for probe in 0..capacity {
            let entry = &self.entries[(index + probe) % capacity];
            if !entry.occupied {
                return None;
            }
            if !entry.deleted && entry.key == key {
                return entry.value.as_ref();
            }
        }

        None
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
        let capacity = self.entries.len();
        let index = Self::hash(key) as usize % capacity;
        let mut found = None;

        for probe in 0..capacity {
            let current = (index + probe) % capacity;
            let entry = &self.entries[current];
            if !entry.occupied {
                break;
            }
            if !entry.deleted && entry.key == key {
                found = Some(current);
                break;
            }
        }

        found.and_then(|current| self.entries[current].value.as_mut())
    }

    pub fn remove(&mut self, key: u64) -> Option<V> {
        let capacity = self.entries.len();
        let index = Self::hash(key) as usize % capacity;

        for probe in 0..capacity {
            let entry = &mut self.entries[(index + probe) % capacity];
            if !entry.occupied {
                return None;
            }
            if !entry.deleted && entry.key == key {
                entry.deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return entry.value.take();
            }
        }

        None
    }

    pub fn contains(&self, key: u64) -> bool {
        self.get(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.occupied = false;
            entry.deleted = false;
            entry.value = None;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
}
