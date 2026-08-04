use std::sync::{Arc, RwLock};
use crate::hash::{hash, alternative_hash};
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

fn allocate_arr(size: usize) -> Vec<CuckooEntry> {
    let mut v = Vec::with_capacity(size);
    for _ in 0..size {
        v.push(CuckooEntry { key: None, data: None, marker: 0 });
    }
    v
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        let half = (initial_size / 2) as u32;
        Arc::new(RwLock::new(CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: allocate_arr(half as usize),
            second_arr: allocate_arr(half as usize),
        }))
    }

    fn first_idx(&self, key: &str) -> usize {
        // Hash by power using POWER over bytes of key (works for any &str).
        let mut h: u32 = 0;
        for b in key.bytes() {
            h = h.wrapping_mul(crate::hash::POWER).wrapping_add(b as u32);
        }
        (h as usize) % (self.max_size as usize)
    }

    fn second_idx(&self, key: &str) -> usize {
        let mut h: u32 = 0;
        for b in key.bytes() {
            h = h.wrapping_mul(crate::hash::ALTERNATIVE_POWER).wrapping_add(b as u32);
        }
        (h as usize) % (self.max_size as usize)
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);

        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;
        self.insert_inner(key, data);
    }

    fn insert_inner(&mut self, mut key: &'static str, mut data: &'static str) {
        // Try first table.
        let first_i = self.first_idx(key);
        if self.first_arr[first_i].key.is_none() {
            self.first_arr[first_i].key = Some(key);
            self.first_arr[first_i].data = Some(data);
            return;
        }
        // Try second table.
        let second_i = self.second_idx(key);
        if self.second_arr[second_i].key.is_none() {
            self.second_arr[second_i].key = Some(key);
            self.second_arr[second_i].data = Some(data);
            return;
        }

        // Eviction loop.
        let mut first_i = self.first_idx(key);
        loop {
            // Swap with first entry.
            let swap_key = self.first_arr[first_i].key.unwrap();
            let swap_data = self.first_arr[first_i].data.unwrap();
            self.first_arr[first_i].key = Some(key);
            self.first_arr[first_i].data = Some(data);
            self.first_arr[first_i].marker = self.cur_marker;
            key = swap_key;
            data = swap_data;

            // Lookup second entry for the displaced key.
            let second_i = self.second_idx(key);
            if self.second_arr[second_i].marker == self.cur_marker {
                // Cycle detected.
                self.refill();
                self.insert_inner(key, data);
                return;
            }
            if self.second_arr[second_i].key.is_none() {
                self.second_arr[second_i].key = Some(key);
                self.second_arr[second_i].data = Some(data);
                return;
            }

            // Swap with second entry.
            let swap_key = self.second_arr[second_i].key.unwrap();
            let swap_data = self.second_arr[second_i].data.unwrap();
            self.second_arr[second_i].key = Some(key);
            self.second_arr[second_i].data = Some(data);
            self.second_arr[second_i].marker = self.cur_marker;
            key = swap_key;
            data = swap_data;

            first_i = self.first_idx(key);
            if self.first_arr[first_i].marker == self.cur_marker {
                // Cycle detected.
                self.refill();
                self.insert_inner(key, data);
                return;
            }
            if self.first_arr[first_i].key.is_none() {
                self.first_arr[first_i].key = Some(key);
                self.first_arr[first_i].data = Some(data);
                return;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_i = self.first_idx(key);
        let entry = &self.first_arr[first_i];
        if let Some(k) = entry.key {
            if k == key {
                return entry.data;
            }
        }
        let second_i = self.second_idx(key);
        let entry = &self.second_arr[second_i];
        if let Some(k) = entry.key {
            if k == key {
                return entry.data;
            }
        }
        None
    }

    fn resize(&mut self) {
        let new_size = self.max_size * 2;
        self.recreate(new_size);
    }

    fn refill(&mut self) {
        let new_size = self.max_size + 1;
        self.recreate(new_size);
    }

    fn recreate(&mut self, new_size: u32) {
        assert!(new_size > 0);
        let old_first = std::mem::replace(&mut self.first_arr, allocate_arr(new_size as usize));
        let old_second = std::mem::replace(&mut self.second_arr, allocate_arr(new_size as usize));

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        for entry in old_first.into_iter() {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
        for entry in old_second.into_iter() {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.first_idx(key);
        // SAFETY: This helper is provided to mirror the C API. It uses an
        // unsafe cast to satisfy the required signature; the actual algorithm
        // uses safe index-based access in `insert_inner`/`find`.
        unsafe {
            let ptr = self.first_arr.as_ptr().add(idx) as *mut CuckooEntry;
            &mut *ptr
        }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_idx(key);
        unsafe {
            let ptr = self.second_arr.as_ptr().add(idx) as *mut CuckooEntry;
            &mut *ptr
        }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            false
        }
    }

    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        let swap_key = entry.key.unwrap_or("");
        let swap_data = entry.data.unwrap_or("");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Drop will handle deallocation automatically.
    }
}
