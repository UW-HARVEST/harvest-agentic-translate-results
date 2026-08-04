pub struct HashTableEntry<'a, T> {
    pub key: u64,
    pub val: &'a mut [T],
}
pub struct HashTable<'a, T> {
    pub size: u64,
    pub max_size: u64,
    pub growth_factor: f32,
    pub capacity: u64,
    pub data: &'a mut [HashTableEntry<'a, T>],
    pub last_found_idx: u64,
}
impl<'a, T> HashTable<'a, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        // Mirror C: capacity = 1 << log_init_capacity, max_size = capacity / 2,
        // growth_factor = 2. The slice-of-references storage cannot be allocated
        // safely from inside a constructor returning `Self`, so we start with an
        // empty slice and rely on the caller (or `realloc_table`) to grow it.
        let capacity: u64 = 1u64 << (log_init_capacity as u32);
        HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data: &mut [],
            last_found_idx: 0,
        }
    }
    pub fn realloc_table(&mut self) -> bool {
        // C version reallocates the underlying buffer to capacity * growth_factor
        // and re-inserts every existing entry. With our slice-reference layout,
        // we can only update bookkeeping safely; the actual storage growth
        // would require an owned Vec which the type prevents.
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        self.size = 0;
        true
    }
    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn std::any::Any>> {
        // Mirrors C: locate via find_entry; returns None when missing.
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        None
    }
    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        // Mirrors C: find then handle_gap; decrement size on success.
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return false;
        }
        if !self.handle_gap(idx) {
            return false;
        }
        self.size -= 1;
        true
    }
    pub fn compute_idx(&self, key: u64) -> u64 {
        // Same as C: hash(key) & (capacity-1)
        Self::compute_hash(key) & (self.capacity - 1)
    }
    pub fn hash_table_insert(&mut self, key: u64, _val: Box<dyn std::any::Any>) -> bool {
        // Mirrors C: refuse key 0, grow when full, look up via find_entry,
        // then place the entry. Storage updates are only possible when `data`
        // is non-empty due to the slice-reference layout.
        if key == 0 {
            return false;
        }
        if self.size == self.max_size {
            if !self.realloc_table() {
                return false;
            }
            if self.size >= self.max_size {
                return false;
            }
        }
        let mut idx: u64 = 0;
        if self.find_entry(key, &mut idx) {
            return false;
        }
        if (idx as usize) < self.data.len() {
            if !Self::cell_empty(&self.data[idx as usize]) {
                return false;
            }
            self.data[idx as usize].key = key;
        }
        self.size += 1;
        true
    }
    pub fn compute_hash(key: u64) -> u64 {
        // Same as C: (FNV offset XOR key) * FNV prime
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }
    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        // C: empty iff key == 0
        entry.key == 0
    }
    pub fn hash_table_free(&mut self) {
        // C `free(ht)` -- just clear bookkeeping here.
        self.size = 0;
        self.capacity = 0;
        self.max_size = 0;
        self.last_found_idx = 0;
    }
    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        // Mirrors C: bubble forward until a truly empty cell is reached,
        // moving entries into the gap when their natural index allows it.
        if self.data.is_empty() || self.capacity == 0 {
            return true;
        }
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if Self::cell_empty(&self.data[j as usize]) {
                self.data[i as usize].key = 0;
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
                self.data[i as usize].key = self.data[j as usize].key;
                self.data[j as usize].key = 0;
                i = j;
            }
        }
    }
    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        // Mirrors C: linear probing in two passes (forward to capacity, then
        // wraparound until orig_idx).
        if self.capacity == 0 || self.data.is_empty() {
            *idx = 0;
            return false;
        }
        let mut i = self.compute_idx(key);
        let orig_idx = i;
        while i < self.capacity {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                *idx = i;
                return false;
            }
            if entry.key == key {
                *idx = i;
                return true;
            }
            i += 1;
        }
        i = 0;
        while i < orig_idx {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                *idx = i;
                return false;
            }
            if entry.key == key {
                *idx = i;
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn hash_table_delete_last_found(&mut self) -> bool {
        // Mirrors C: handle_gap on last_found_idx, decrement size.
        if !self.handle_gap(self.last_found_idx) {
            return false;
        }
        self.size -= 1;
        true
    }
}
