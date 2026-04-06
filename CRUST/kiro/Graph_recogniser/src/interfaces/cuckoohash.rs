use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash;
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

fn alloc_arr(size: u32) -> Vec<CuckooEntry> {
    (0..size).map(|_| CuckooEntry { key: None, data: None, marker: 0 }).collect()
}

fn hash_idx(key: &str, max_size: u32) -> usize {
    (crate::hash::hash_str(key) % max_size) as usize
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        let half = (initial_size / 2) as u32;
        Arc::new(RwLock::new(Self {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: alloc_arr(half),
            second_arr: alloc_arr(half),
        }))
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }
        self.cur_size += 1;

        let idx = hash_idx(key, self.max_size);
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
        loop {
            // swap with first_arr entry
            let idx = hash_idx(key, self.max_size);
            let e = &mut self.first_arr[idx];
            std::mem::swap(&mut key, e.key.as_mut().unwrap());
            std::mem::swap(&mut data, e.data.as_mut().unwrap());
            e.marker = self.cur_marker;

            // try second
            let idx2 = hash_idx(key, self.max_size);
            if self.second_arr[idx2].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1;
                self.insert(key, data);
                return;
            }
            if self.second_arr[idx2].key.is_none() {
                self.second_arr[idx2].key = Some(key);
                self.second_arr[idx2].data = Some(data);
                return;
            }

            // swap with second_arr entry
            let e2 = &mut self.second_arr[idx2];
            std::mem::swap(&mut key, e2.key.as_mut().unwrap());
            std::mem::swap(&mut data, e2.data.as_mut().unwrap());
            e2.marker = self.cur_marker;

            // try first
            let idx3 = hash_idx(key, self.max_size);
            if self.first_arr[idx3].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1;
                self.insert(key, data);
                return;
            }
            if self.first_arr[idx3].key.is_none() {
                self.first_arr[idx3].key = Some(key);
                self.first_arr[idx3].data = Some(data);
                return;
            }
        }
    }
    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = hash_idx(key, self.max_size);
        if let Some(k) = self.first_arr[idx].key {
            if k == key {
                return self.first_arr[idx].data;
            }
        }
        if let Some(k) = self.second_arr[idx].key {
            if k == key {
                return self.second_arr[idx].data;
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
        let old_first = std::mem::replace(&mut self.first_arr, alloc_arr(new_size));
        let old_second = std::mem::replace(&mut self.second_arr, alloc_arr(new_size));
        let old_cur_size = self.cur_size;
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        for e in old_first.into_iter().chain(old_second.into_iter()) {
            if let Some(k) = e.key {
                self.insert(k, e.data.unwrap());
            }
        }
    }
    #[allow(invalid_reference_casting)]
    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = hash_idx(key, self.max_size);
        unsafe { &mut *(std::ptr::from_ref(&self.first_arr[idx]).cast_mut()) }
    }
    #[allow(invalid_reference_casting)]
    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = hash_idx(key, self.max_size);
        unsafe { &mut *(std::ptr::from_ref(&self.second_arr[idx]).cast_mut()) }
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
        std::mem::swap(key, entry.key.as_mut().unwrap());
        std::mem::swap(data, entry.data.as_mut().unwrap());
    }
    fn free_cukoo_hash_table(self) {
        // Drop happens automatically
    }
}
