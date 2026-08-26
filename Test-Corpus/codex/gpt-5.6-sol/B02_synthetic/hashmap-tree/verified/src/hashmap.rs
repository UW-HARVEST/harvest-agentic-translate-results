const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f64 = 0.75;

enum Slot<V> {
    Empty,
    Occupied { key: u64, value: V },
    Deleted,
}

pub struct HashMap<V> {
    entries: Vec<Slot<V>>,
    size: usize,
    deleted_count: usize,
}

impl<V> HashMap<V> {
    pub fn new() -> Self {
        Self {
            entries: empty_entries(INITIAL_CAPACITY),
            size: 0,
            deleted_count: 0,
        }
    }

    fn should_resize(&self) -> bool {
        (self.size + self.deleted_count) as f64 / self.entries.len() as f64 > LOAD_FACTOR
    }

    fn resize(&mut self) {
        let new_capacity = self.entries.len() * 2;
        let old_entries =
            std::mem::replace(&mut self.entries, empty_entries(new_capacity));
        self.size = 0;
        self.deleted_count = 0;

        for entry in old_entries {
            if let Slot::Occupied { key, value } = entry {
                self.put(key, value);
            }
        }
    }

    pub fn put(&mut self, key: u64, value: V) {
        if self.should_resize() {
            self.resize();
        }

        let start = hash_function(key) as usize % self.entries.len();
        for probe in 0..self.entries.len() {
            let current = (start + probe) % self.entries.len();
            match &mut self.entries[current] {
                slot @ Slot::Empty => {
                    *slot = Slot::Occupied { key, value };
                    self.size += 1;
                    return;
                }
                slot @ Slot::Deleted => {
                    *slot = Slot::Occupied { key, value };
                    self.size += 1;
                    self.deleted_count -= 1;
                    return;
                }
                Slot::Occupied {
                    key: existing_key,
                    value: existing_value,
                } if *existing_key == key => {
                    *existing_value = value;
                    return;
                }
                Slot::Occupied { .. } => {}
            }
        }
    }

    pub fn get(&self, key: u64) -> Option<&V> {
        let index = self.find_index(key)?;
        match &self.entries[index] {
            Slot::Occupied { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
        let index = self.find_index(key)?;
        match &mut self.entries[index] {
            Slot::Occupied { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn remove(&mut self, key: u64) -> Option<V> {
        let index = self.find_index(key)?;
        let old_entry = std::mem::replace(&mut self.entries[index], Slot::Deleted);
        if let Slot::Occupied { value, .. } = old_entry {
            self.size -= 1;
            self.deleted_count += 1;
            Some(value)
        } else {
            None
        }
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
            *entry = Slot::Empty;
        }
        self.size = 0;
        self.deleted_count = 0;
    }

    fn find_index(&self, key: u64) -> Option<usize> {
        let start = hash_function(key) as usize % self.entries.len();
        for probe in 0..self.entries.len() {
            let current = (start + probe) % self.entries.len();
            match &self.entries[current] {
                Slot::Empty => return None,
                Slot::Occupied {
                    key: existing_key,
                    ..
                } if *existing_key == key => return Some(current),
                Slot::Occupied { .. } | Slot::Deleted => {}
            }
        }
        None
    }
}

fn empty_entries<V>(capacity: usize) -> Vec<Slot<V>> {
    std::iter::repeat_with(|| Slot::Empty)
        .take(capacity)
        .collect()
}

fn hash_function(key: u64) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in key.to_ne_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}
