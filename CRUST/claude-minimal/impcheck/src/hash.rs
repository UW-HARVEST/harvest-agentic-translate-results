use std::any::Any;
use std::marker::PhantomData;

pub struct HashTableEntry {
    pub key: u64,
    pub val: Option<Box<dyn Any>>,
}

impl HashTableEntry {
    fn empty() -> Self {
        HashTableEntry { key: 0, val: None }
    }
}

pub struct HashTable<T> {
    pub size: u64,
    pub max_size: u64,
    pub growth_factor: f32,
    pub capacity: u64,
    pub data: Vec<HashTableEntry>,
    pub last_found_idx: u64,
    pub _phantom: PhantomData<T>,
}

impl<T> HashTable<T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let mut data: Vec<HashTableEntry> = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            data.push(HashTableEntry::empty());
        }
        HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
            _phantom: PhantomData,
        }
    }

    pub fn compute_hash(key: u64) -> u64 {
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }

    pub fn compute_idx(&self, key: u64) -> u64 {
        Self::compute_hash(key) & (self.capacity - 1)
    }

    pub fn cell_empty(entry: &HashTableEntry) -> bool {
        entry.key == 0
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
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

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity: u64 = (self.growth_factor * self.capacity as f32) as u64;
        let mut old_data: Vec<HashTableEntry> = std::mem::take(&mut self.data);
        let old_capacity = self.capacity;
        let mut new_data: Vec<HashTableEntry> = Vec::with_capacity(new_capacity as usize);
        for _ in 0..new_capacity {
            new_data.push(HashTableEntry::empty());
        }
        self.data = new_data;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        for i in 0..(old_capacity as usize) {
            let cell = std::mem::replace(&mut old_data[i], HashTableEntry::empty());
            if cell.key != 0 {
                if !self.hash_table_insert(cell.key, cell.val.unwrap()) {
                    return false;
                }
            }
        }
        true
    }

    pub fn hash_table_find(&mut self, key: u64) -> Option<&Box<dyn Any>> {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        self.last_found_idx = idx;
        self.data[idx as usize].val.as_ref()
    }

    pub fn hash_table_insert(&mut self, key: u64, val: Box<dyn Any>) -> bool {
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
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        let entry = &mut self.data[idx as usize];
        entry.key = key;
        entry.val = Some(val);
        self.size += 1;
        true
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if Self::cell_empty(&self.data[j as usize]) {
                let entry = &mut self.data[i as usize];
                entry.key = 0;
                entry.val = None;
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
                // Move entry j -> i
                let moved = std::mem::replace(&mut self.data[j as usize], HashTableEntry::empty());
                self.data[i as usize] = moved;
                i = j;
            }
        }
    }

    pub fn hash_table_delete(&mut self, key: u64) -> bool {
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

    pub fn hash_table_delete_last_found(&mut self) -> bool {
        if !self.handle_gap(self.last_found_idx) {
            return false;
        }
        self.size -= 1;
        true
    }

    pub fn hash_table_free(&mut self) {
        self.data.clear();
        self.size = 0;
        self.capacity = 0;
    }
}
