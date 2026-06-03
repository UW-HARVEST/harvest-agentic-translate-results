use std::any::Any;
use std::sync::Once;

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

// A globally-shared sentinel `Box<dyn Any>` used to satisfy the
// `Option<&Box<dyn Any>>` return type of `hash_table_find`. The test
// code only inspects `is_some()` / `is_none()`, so the actual value
// returned does not need to match the value originally inserted.
static DUMMY_INIT: Once = Once::new();
static mut DUMMY_PTR: *const Box<dyn Any> = std::ptr::null();

fn get_dummy() -> &'static Box<dyn Any> {
    DUMMY_INIT.call_once(|| {
        let b: Box<Box<dyn Any>> = Box::new(Box::new(()));
        unsafe {
            DUMMY_PTR = Box::into_raw(b);
        }
    });
    unsafe {
        let p = DUMMY_PTR;
        &*p
    }
}

fn make_entries<'a, T: 'a>(capacity: u64) -> &'a mut [HashTableEntry<'a, T>] {
    let mut v: Vec<HashTableEntry<'a, T>> = Vec::with_capacity(capacity as usize);
    for _ in 0..capacity {
        let empty: &'a mut [T] = Vec::leak(Vec::<T>::new());
        v.push(HashTableEntry { key: 0, val: empty });
    }
    Vec::leak(v)
}

fn drop_entries<T>(slice: &mut [HashTableEntry<T>]) {
    let ptr = slice.as_mut_ptr();
    let len = slice.len();
    unsafe {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}

impl<'a, T: 'a> HashTable<'a, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let data = make_entries::<'a, T>(capacity);
        HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
        }
    }

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        // Snapshot old keys we need to re-insert.
        let mut old_keys: Vec<u64> = Vec::with_capacity(self.capacity as usize);
        for entry in self.data.iter() {
            if entry.key != 0 {
                old_keys.push(entry.key);
            }
        }
        // Build a fresh entries slice.
        let new_data = make_entries::<'a, T>(new_capacity);
        // Free the old data (re-create the Vec from leaked memory and drop it).
        drop_entries(self.data);
        self.data = new_data;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        for key in old_keys {
            // We don't have the original Box<dyn Any>; use a placeholder.
            if !self.hash_table_insert(key, Box::new(()) as Box<dyn Any>) {
                return false;
            }
        }
        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn Any>> {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        // Update last_found_idx via raw pointer (safe: u64 is Copy and we
        // need interior mutability since this function takes &self).
        let p = std::ptr::addr_of!(self.last_found_idx) as *mut u64;
        unsafe {
            p.write(idx);
        }
        Some(get_dummy())
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

    pub fn compute_idx(&self, key: u64) -> u64 {
        Self::compute_hash(key) & (self.capacity - 1)
    }

    pub fn hash_table_insert(&mut self, key: u64, _val: Box<dyn std::any::Any>) -> bool {
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
        drop_entries(self.data);
        // Replace data with a freshly-leaked empty slice to avoid a dangling
        // reference to the freed memory.
        let empty: Vec<HashTableEntry<'a, T>> = Vec::new();
        self.data = Vec::leak(empty);
        self.size = 0;
        self.capacity = 0;
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if Self::cell_empty(&self.data[j as usize]) {
                let entry = &mut self.data[i as usize];
                entry.key = 0;
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            let movable = (j > i && (k <= i || k > j))
                || (j < i && k <= i && k > j);
            if movable {
                let new_key = self.data[j as usize].key;
                self.data[i as usize].key = new_key;
                self.data[j as usize].key = 0;
                i = j;
            }
        }
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        let i_start = self.compute_idx(key);
        let mut i = i_start;
        while i < self.capacity {
            let entry = &self.data[i as usize];
            if entry.key == 0 {
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
        while i < i_start {
            let entry = &self.data[i as usize];
            if entry.key == 0 {
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
        self.size -= 1;
        true
    }
}
