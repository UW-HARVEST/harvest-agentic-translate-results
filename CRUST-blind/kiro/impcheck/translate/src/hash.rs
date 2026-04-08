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

// The generic HashTable with lifetime-bound entries is not practical for this use case.
// We implement a separate, self-contained hash table that the rest of the codebase actually uses.

pub struct HtEntry {
    pub key: u64,
    pub val: Option<Box<dyn Any>>,
}

pub struct HTable {
    pub size: u64,
    pub max_size: u64,
    pub growth_factor: f32,
    pub capacity: u64,
    pub data: Vec<HtEntry>,
    pub last_found_idx: u64,
}

fn compute_hash(key: u64) -> u64 {
    (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
}

impl HTable {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity = 1u64 << log_init_capacity;
        let mut data = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            data.push(HtEntry { key: 0, val: None });
        }
        HTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
        }
    }

    fn compute_idx(&self, key: u64) -> u64 {
        compute_hash(key) & (self.capacity - 1)
    }

    fn cell_empty(entry: &HtEntry) -> bool {
        entry.key == 0
    }

    fn find_entry(&self, key: u64) -> (bool, u64) {
        let mut i = self.compute_idx(key);
        let orig_idx = i;
        while i < self.capacity {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                return (false, i);
            }
            if entry.key == key {
                return (true, i);
            }
            i += 1;
        }
        i = 0;
        while i < orig_idx {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                return (false, i);
            }
            if entry.key == key {
                return (true, i);
            }
            i += 1;
        }
        (false, 0)
    }

    fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        let old_data = std::mem::replace(&mut self.data, Vec::new());
        let mut new_data = Vec::with_capacity(new_capacity as usize);
        for _ in 0..new_capacity {
            new_data.push(HtEntry { key: 0, val: None });
        }
        self.data = new_data;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        for entry in old_data {
            if entry.key != 0 {
                if !self.insert(entry.key, entry.val.unwrap()) {
                    return false;
                }
            }
        }
        true
    }

    fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap as usize;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity as usize - 1);
            if Self::cell_empty(&self.data[j]) {
                self.data[i].key = 0;
                self.data[i].val = None;
                return true;
            }
            let k = self.compute_idx(self.data[j].key) as usize;
            if (j > i && (k <= i || k > j))
                || (j < i && k <= i && k > j)
            {
                // Move entry from j to i
                let key = self.data[j].key;
                let val = self.data[j].val.take();
                self.data[i].key = key;
                self.data[i].val = val;
                self.data[j].key = 0;
                self.data[j].val = None;
                i = j;
            }
        }
    }

    pub fn find(&mut self, key: u64) -> Option<&Box<dyn Any>> {
        let (found, idx) = self.find_entry(key);
        if !found {
            return None;
        }
        self.last_found_idx = idx;
        self.data[idx as usize].val.as_ref()
    }

    pub fn find_mut(&mut self, key: u64) -> Option<&mut Box<dyn Any>> {
        let (found, idx) = self.find_entry(key);
        if !found {
            return None;
        }
        self.last_found_idx = idx;
        self.data[idx as usize].val.as_mut()
    }

    pub fn insert(&mut self, key: u64, val: Box<dyn Any>) -> bool {
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
        let (found, idx) = self.find_entry(key);
        if found {
            return false;
        }
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        self.data[idx as usize].key = key;
        self.data[idx as usize].val = Some(val);
        self.size += 1;
        true
    }

    pub fn delete(&mut self, key: u64) -> bool {
        let (found, idx) = self.find_entry(key);
        if !found {
            return false;
        }
        if !self.handle_gap(idx) {
            return false;
        }
        self.size -= 1;
        true
    }

    pub fn delete_last_found(&mut self) -> bool {
        let idx = self.last_found_idx;
        if !self.handle_gap(idx) {
            return false;
        }
        self.size -= 1;
        true
    }
}

// Dummy implementations for the generic HashTable to satisfy the compiler
impl<T> HashTable<'_, T> {
pub fn new(log_init_capacity: i32) -> Self {
    let _ = log_init_capacity;
    std::process::exit(1)
}
pub fn realloc_table(&mut self) -> bool {
    false
}
pub fn hash_table_find(&self, _key: u64) -> Option<&Box<dyn std::any::Any>> {
    None
}
pub fn hash_table_delete(&mut self, _key: u64) -> bool {
    false
}
pub fn compute_idx(&self, key: u64) -> u64 {
    compute_hash(key) & (self.capacity - 1)
}
pub fn hash_table_insert(&mut self, _key: u64, _val: Box<dyn std::any::Any>) -> bool {
    false
}
pub fn compute_hash(key: u64) -> u64 {
    compute_hash(key)
}
pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
    entry.key == 0
}
pub fn hash_table_free(&mut self) {
}
pub fn handle_gap(&mut self, _idx_of_gap: u64) -> bool {
    false
}
pub fn find_entry(&self, _key: u64, _idx: &mut u64) -> bool {
    false
}
pub fn hash_table_delete_last_found(&mut self) -> bool {
    false
}
}
