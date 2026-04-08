use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash_str;
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

fn make_arr(size: u32) -> Vec<CuckooEntry> {
    (0..size).map(|_| CuckooEntry { key: None, data: None, marker: 0 }).collect()
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        let half = (initial_size / 2) as u32;
        Arc::new(RwLock::new(Self {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: make_arr(half),
            second_arr: make_arr(half),
        }))
    }

    fn hash_index(&self, key: &str) -> usize {
        (hash_str(key) % self.max_size) as usize
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }
        self.cur_size += 1;

        let idx = self.hash_index(key);
        if self.first_arr[idx].key.is_none() {
            self.first_arr[idx].key = Some(key);
            self.first_arr[idx].data = Some(data);
            return;
        }
        if self.second_arr[idx].key.is_none() {
            self.second_arr[idx].key = Some(key);
            self.second_arr[idx].data = Some(data);
            return;
        }

        let mut key = key;
        let mut data = data;
        let marker = self.cur_marker;

        loop {
            // swap with first_arr entry
            let fi = self.hash_index(key);
            let old_key = self.first_arr[fi].key.take().unwrap();
            let old_data = self.first_arr[fi].data.take().unwrap();
            self.first_arr[fi].key = Some(key);
            self.first_arr[fi].data = Some(data);
            self.first_arr[fi].marker = marker;
            key = old_key;
            data = old_data;

            let si = self.hash_index(key);
            if self.second_arr[si].marker == marker {
                self.refill();
                self.insert(key, data);
                return;
            }
            if self.second_arr[si].key.is_none() {
                self.second_arr[si].key = Some(key);
                self.second_arr[si].data = Some(data);
                return;
            }

            // swap with second_arr entry
            let old_key = self.second_arr[si].key.take().unwrap();
            let old_data = self.second_arr[si].data.take().unwrap();
            self.second_arr[si].key = Some(key);
            self.second_arr[si].data = Some(data);
            self.second_arr[si].marker = marker;
            key = old_key;
            data = old_data;

            let fi = self.hash_index(key);
            if self.first_arr[fi].marker == marker {
                self.refill();
                self.insert(key, data);
                return;
            }
            if self.first_arr[fi].key.is_none() {
                self.first_arr[fi].key = Some(key);
                self.first_arr[fi].data = Some(data);
                return;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.hash_index(key);
        if let Some(k) = self.first_arr[idx].key {
            if k == key {
                return self.first_arr[idx].data;
            }
        }
        self.second_arr[idx].data
    }

    fn resize(&mut self) {
        self.recreate(self.max_size * 2);
    }

    fn refill(&mut self) {
        self.recreate(self.max_size + 1);
    }

    fn recreate(&mut self, new_size: u32) {
        let old_first = std::mem::replace(&mut self.first_arr, Vec::new());
        let old_second = std::mem::replace(&mut self.second_arr, Vec::new());

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;
        self.first_arr = make_arr(new_size);
        self.second_arr = make_arr(new_size);

        for entry in old_first {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
        for entry in old_second {
            if let (Some(k), Some(d)) = (entry.key, entry.data) {
                self.insert(k, d);
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.hash_index(key);
        let ptr = self.first_arr.as_ptr().wrapping_add(idx) as *mut CuckooEntry;
        unsafe { &mut *ptr }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.hash_index(key);
        let ptr = self.second_arr.as_ptr().wrapping_add(idx) as *mut CuckooEntry;
        unsafe { &mut *ptr }
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
        let swap_key = entry.key.take().unwrap();
        let swap_data = entry.data.take().unwrap();
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Rust drops automatically
    }
}
