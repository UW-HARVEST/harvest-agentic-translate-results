use std::any::Any;

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
impl<T> HashTable<'_, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let max_size = capacity >> 1;
        // Use an empty mutable slice as the initial data store.
        let data: &mut [HashTableEntry<T>] = &mut [];
        HashTable {
            size: 0,
            max_size,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
        }
    }
    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        self.capacity = new_capacity;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        true
    }
    pub fn hash_table_find(&self, _key: u64) -> Option<&Box<dyn Any>> {
        None
    }
    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return false;
        }
        if !self.handle_gap(idx) {
            return false;
        }
        if self.size > 0 {
            self.size -= 1;
        }
        true
    }
    pub fn compute_idx(&self, key: u64) -> u64 {
        if self.capacity == 0 {
            return 0;
        }
        Self::compute_hash(key) & (self.capacity - 1)
    }
    pub fn hash_table_insert(&mut self, key: u64, _val: Box<dyn Any>) -> bool {
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
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }
    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        entry.key == 0
    }
    pub fn hash_table_free(&mut self) {
        self.size = 0;
        self.capacity = 0;
        self.max_size = 0;
    }
    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        if self.capacity == 0 || self.data.is_empty() {
            return false;
        }
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if (j as usize) >= self.data.len() {
                return false;
            }
            if Self::cell_empty(&self.data[j as usize]) {
                if (i as usize) < self.data.len() {
                    self.data[i as usize].key = 0;
                }
                return true;
            }
            let key_j = self.data[j as usize].key;
            let k = self.compute_idx(key_j);
            let cond = (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j);
            if cond {
                if (i as usize) < self.data.len() && (j as usize) < self.data.len() {
                    self.data[i as usize].key = self.data[j as usize].key;
                    self.data[j as usize].key = 0;
                }
                i = j;
            }
        }
    }
    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        if self.capacity == 0 || self.data.is_empty() {
            *idx = 0;
            return false;
        }
        let orig_idx = self.compute_idx(key);
        let mut i = orig_idx;
        while i < self.capacity {
            if (i as usize) >= self.data.len() {
                break;
            }
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
        let mut i = 0u64;
        while i < orig_idx {
            if (i as usize) >= self.data.len() {
                break;
            }
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
        if !self.handle_gap(self.last_found_idx) {
            return false;
        }
        if self.size > 0 {
            self.size -= 1;
        }
        true
    }
}
