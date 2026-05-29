use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash;
const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

const POWER: u32 = 131;

fn compute_hash_str(s: &str) -> u32 {
    let mut res: u32 = 0;
    for c in s.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(c as u32);
    }
    res
}

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
impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        let size = (initial_size / 2) as u32;
        let mut table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: 0,
            first_arr: Vec::new(),
            second_arr: Vec::new(),
        };
        table.initialize(size);
        Arc::new(RwLock::new(table))
    }

    fn initialize(&mut self, size: u32) {
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = size;
        self.first_arr = (0..size)
            .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
            .collect();
        self.second_arr = (0..size)
            .map(|_| CuckooEntry { key: None, data: None, marker: 0 })
            .collect();
    }

    fn first_index(&self, key: &str) -> usize {
        let h = compute_hash_str(key);
        (h % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        // C uses the same hash() for both arrays
        let h = compute_hash_str(key);
        (h % self.max_size) as usize
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;

        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut cur_key: &'static str = key;
        let mut cur_data: &'static str = data;

        // Try first entry
        let mut first_idx = self.first_index(cur_key);
        if self.first_arr[first_idx].key.is_none() {
            self.first_arr[first_idx].key = Some(cur_key);
            self.first_arr[first_idx].data = Some(cur_data);
            return;
        }

        // Try second entry
        let mut second_idx = self.second_index(cur_key);
        if self.second_arr[second_idx].key.is_none() {
            self.second_arr[second_idx].key = Some(cur_key);
            self.second_arr[second_idx].data = Some(cur_data);
            return;
        }

        loop {
            // Swap with first_entry
            let swap_key = self.first_arr[first_idx].key.unwrap();
            let swap_data = self.first_arr[first_idx].data.unwrap();
            self.first_arr[first_idx].key = Some(cur_key);
            self.first_arr[first_idx].data = Some(cur_data);
            self.first_arr[first_idx].marker = self.cur_marker;
            cur_key = swap_key;
            cur_data = swap_data;

            second_idx = self.second_index(cur_key);
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.insert(cur_key, cur_data);
                return;
            }

            if self.second_arr[second_idx].key.is_none() {
                self.second_arr[second_idx].key = Some(cur_key);
                self.second_arr[second_idx].data = Some(cur_data);
                return;
            }

            // Swap with second_entry
            let swap_key2 = self.second_arr[second_idx].key.unwrap();
            let swap_data2 = self.second_arr[second_idx].data.unwrap();
            self.second_arr[second_idx].key = Some(cur_key);
            self.second_arr[second_idx].data = Some(cur_data);
            self.second_arr[second_idx].marker = self.cur_marker;
            cur_key = swap_key2;
            cur_data = swap_data2;

            first_idx = self.first_index(cur_key);
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.insert(cur_key, cur_data);
                return;
            }

            if self.first_arr[first_idx].key.is_none() {
                self.first_arr[first_idx].key = Some(cur_key);
                self.first_arr[first_idx].data = Some(cur_data);
                return;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.first_index(key);
        if let Some(k) = self.first_arr[idx].key {
            if k == key {
                return self.first_arr[idx].data;
            }
        }
        let idx = self.second_index(key);
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
        debug_assert!(new_size > 0);

        let old_first = std::mem::take(&mut self.first_arr);
        let old_second = std::mem::take(&mut self.second_arr);

        self.initialize(new_size);

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
        // The Drop impl on Vec will free memory automatically when self goes out of scope.
        drop(self);
    }
}
