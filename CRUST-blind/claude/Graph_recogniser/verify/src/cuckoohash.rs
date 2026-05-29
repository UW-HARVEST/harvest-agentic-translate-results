use std::sync::{Arc, RwLock};
use crate::hash::{POWER, ALTERNATIVE_POWER};
const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;
pub struct CuckooEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
    marker: u32,
}
pub struct CuckooHashTable {
    cur_size: u32,
    cur_marker: u32,
    max_size: u32,
    first_arr: Vec<CuckooEntry>,
    second_arr: Vec<CuckooEntry>,
}

fn compute_hash(key: &str) -> u32 {
    let mut res: u32 = 0;
    for b in key.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(b as u32);
    }
    res
}

fn compute_alt_hash(key: &str) -> u32 {
    let mut res: u32 = 0;
    for b in key.bytes() {
        res = res.wrapping_mul(ALTERNATIVE_POWER).wrapping_add(b as u32);
    }
    res
}

fn allocate_arr(size: u32) -> Vec<CuckooEntry> {
    let mut v = Vec::with_capacity(size as usize);
    for _ in 0..size {
        v.push(CuckooEntry { key: None, data: None, marker: 0 });
    }
    v
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        debug_assert!(initial_size >= 2);
        debug_assert!(initial_size as u32 != POWER && initial_size as u32 != ALTERNATIVE_POWER);
        let half = (initial_size / 2) as u32;
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: allocate_arr(half),
            second_arr: allocate_arr(half),
        };
        Arc::new(RwLock::new(table))
    }

    fn first_index(&self, key: &str) -> usize {
        (compute_hash(key) % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        (compute_alt_hash(key) % self.max_size) as usize
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;
        self.do_insert(key, data);
    }

    fn do_insert(&mut self, mut key: &'static str, mut data: &'static str) {
        // Try first array
        let first_idx = self.first_index(key);
        if self.first_arr[first_idx].key.is_none() {
            self.first_arr[first_idx].key = Some(key);
            self.first_arr[first_idx].data = Some(data);
            return;
        } else {
            debug_assert!(self.first_arr[first_idx].key != Some(key));
        }

        // Try second array
        let second_idx = self.second_index(key);
        if self.second_arr[second_idx].key.is_none() {
            self.second_arr[second_idx].key = Some(key);
            self.second_arr[second_idx].data = Some(data);
            return;
        } else {
            debug_assert!(self.second_arr[second_idx].key != Some(key));
        }

        let mut first_idx = first_idx;
        loop {
            // Swap with first_entry
            let swap_key = self.first_arr[first_idx].key.unwrap();
            let swap_data = self.first_arr[first_idx].data.unwrap();
            self.first_arr[first_idx].key = Some(key);
            self.first_arr[first_idx].data = Some(data);
            self.first_arr[first_idx].marker = self.cur_marker;
            key = swap_key;
            data = swap_data;

            let second_idx = self.second_index(key);
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.do_insert(key, data);
                return;
            }
            if self.second_arr[second_idx].key.is_none() {
                self.second_arr[second_idx].key = Some(key);
                self.second_arr[second_idx].data = Some(data);
                return;
            } else {
                debug_assert!(self.second_arr[second_idx].key != Some(key));
            }

            // Swap with second_entry
            let swap_key = self.second_arr[second_idx].key.unwrap();
            let swap_data = self.second_arr[second_idx].data.unwrap();
            self.second_arr[second_idx].key = Some(key);
            self.second_arr[second_idx].data = Some(data);
            self.second_arr[second_idx].marker = self.cur_marker;
            key = swap_key;
            data = swap_data;

            first_idx = self.first_index(key);
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.do_insert(key, data);
                return;
            }
            if self.first_arr[first_idx].key.is_none() {
                self.first_arr[first_idx].key = Some(key);
                self.first_arr[first_idx].data = Some(data);
                return;
            } else {
                debug_assert!(self.first_arr[first_idx].key != Some(key));
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_idx = self.first_index(key);
        let entry = &self.first_arr[first_idx];
        if let Some(k) = entry.key {
            if k == key {
                return entry.data;
            }
        }
        let second_idx = self.second_index(key);
        let entry = &self.second_arr[second_idx];
        debug_assert!(entry.key.is_some() && entry.key.unwrap() == key);
        entry.data
    }

    fn resize(&mut self) {
        self.recreate(self.max_size * 2);
    }

    fn refill(&mut self) {
        self.recreate(self.max_size + 1);
    }

    fn recreate(&mut self, new_size: u32) {
        debug_assert!(new_size > 0);
        #[cfg(debug_assertions)]
        debug_assert!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);

        let old_first = std::mem::replace(&mut self.first_arr, allocate_arr(new_size));
        let old_second = std::mem::replace(&mut self.second_arr, allocate_arr(new_size));
        let old_size = self.max_size;

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        for i in 0..old_size as usize {
            if let Some(k) = old_first[i].key {
                let d = old_first[i].data.unwrap();
                self.insert(k, d);
            }
        }
        for i in 0..old_size as usize {
            if let Some(k) = old_second[i].key {
                let d = old_second[i].data.unwrap();
                self.insert(k, d);
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        // The signature requires returning a mutable reference from &self,
        // mirroring the C function which returns a non-const pointer from a non-const table.
        // We obtain a raw mutable pointer through the Vec's internal buffer.
        let idx = self.first_index(key);
        let ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(idx) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        let ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(idx) }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            debug_assert!(entry.key != Some(key));
            false
        }
    }

    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        let swap_key = entry.key.unwrap();
        let swap_data = entry.data.unwrap();
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Dropping self will free everything.
        drop(self);
    }
}

