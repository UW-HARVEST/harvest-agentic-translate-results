use std::sync::{Arc, RwLock};
use crate::check;
use crate::hash::{hash_any, alternative_hash, POWER, ALTERNATIVE_POWER};

const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

#[derive(Clone)]
pub struct CuckooEntry {
    pub key: Option<&'static str>,
    pub data: Option<&'static str>,
    pub marker: u32,
}

pub struct CuckooHashTable {
    pub cur_size: u32,
    pub cur_marker: u32,
    pub max_size: u32,
    pub first_arr: Vec<CuckooEntry>,
    pub second_arr: Vec<CuckooEntry>,
}

fn alt_hash_any(key: &str) -> u32 {
    crate::hash::hash_by_power_any(key, ALTERNATIVE_POWER)
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        check::check(initial_size >= 2);
        let size = (initial_size / 2) as u32;
        check::check(initial_size as u32 != POWER && initial_size as u32 != ALTERNATIVE_POWER);
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: size,
            first_arr: (0..size)
                .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
                .collect(),
            second_arr: (0..size)
                .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
                .collect(),
        };
        Arc::new(RwLock::new(table))
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);
        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.recreate(self.max_size * 2);
        }
        self.cur_size += 1;

        let mut k: &'static str = key;
        let mut d: &'static str = data;

        // First try to insert in first array
        let first_idx = (hash_any(k) % self.max_size) as usize;
        if self.first_arr[first_idx].key.is_none() {
            self.first_arr[first_idx].key = Some(k);
            self.first_arr[first_idx].data = Some(d);
            return;
        }

        // Then try second array
        let second_idx = (alt_hash_any(k) % self.max_size) as usize;
        if self.second_arr[second_idx].key.is_none() {
            self.second_arr[second_idx].key = Some(k);
            self.second_arr[second_idx].data = Some(d);
            return;
        }

        // Loop with cuckoo eviction
        let mut current_first_idx = first_idx;
        loop {
            // Swap with first_arr[current_first_idx]
            let entry = &mut self.first_arr[current_first_idx];
            let swap_key = entry.key.unwrap();
            let swap_data = entry.data.unwrap();
            entry.key = Some(k);
            entry.data = Some(d);
            entry.marker = self.cur_marker;
            k = swap_key;
            d = swap_data;

            // Check second_arr for displaced
            let second_idx = (alt_hash_any(k) % self.max_size) as usize;
            if self.second_arr[second_idx].marker == self.cur_marker {
                // Cycle detected - refill and retry
                self.recreate(self.max_size + 1);
                self.insert(k, d);
                return;
            }
            if self.second_arr[second_idx].key.is_none() {
                self.second_arr[second_idx].key = Some(k);
                self.second_arr[second_idx].data = Some(d);
                return;
            }

            // Swap with second_arr
            let entry = &mut self.second_arr[second_idx];
            let swap_key = entry.key.unwrap();
            let swap_data = entry.data.unwrap();
            entry.key = Some(k);
            entry.data = Some(d);
            entry.marker = self.cur_marker;
            k = swap_key;
            d = swap_data;

            // Check first_arr for displaced
            let new_first_idx = (hash_any(k) % self.max_size) as usize;
            if self.first_arr[new_first_idx].marker == self.cur_marker {
                self.recreate(self.max_size + 1);
                self.insert(k, d);
                return;
            }
            if self.first_arr[new_first_idx].key.is_none() {
                self.first_arr[new_first_idx].key = Some(k);
                self.first_arr[new_first_idx].data = Some(d);
                return;
            }
            current_first_idx = new_first_idx;
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_idx = (hash_any(key) % self.max_size) as usize;
        let entry = &self.first_arr[first_idx];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }
        let second_idx = (alt_hash_any(key) % self.max_size) as usize;
        let entry = &self.second_arr[second_idx];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }
        None
    }

    fn resize(&mut self) {
        self.recreate(self.max_size * 2);
    }

    fn refill(&mut self) {
        self.recreate(self.max_size + 1);
    }

    fn recreate(&mut self, new_size: u32) {
        check::check(new_size > 0);
        let old_first = std::mem::replace(
            &mut self.first_arr,
            (0..new_size)
                .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
                .collect(),
        );
        let old_second = std::mem::replace(
            &mut self.second_arr,
            (0..new_size)
                .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
                .collect(),
        );
        self.max_size = new_size;
        self.cur_size = 0;
        self.cur_marker = 0;

        for entry in old_first.iter() {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
        for entry in old_second.iter() {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        // Compute the slot index in the first array.
        let idx = (hash_any(key) % self.max_size) as usize;
        // Safety: We need a `&mut` from a `&self` to satisfy the original C
        // signature, which mirrored a raw pointer. This helper is not used
        // by the public API; `insert`/`find` use safe index-based access.
        unsafe {
            let arr_ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
            &mut *arr_ptr.add(idx)
        }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = (alt_hash_any(key) % self.max_size) as usize;
        unsafe {
            let arr_ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
            &mut *arr_ptr.add(idx)
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
        // Drop is automatic; Vec memory is freed when the table goes out of scope.
        drop(self.first_arr);
        drop(self.second_arr);
    }
}
