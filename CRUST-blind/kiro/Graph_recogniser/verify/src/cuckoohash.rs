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

fn allocate_arr(size: u32) -> Vec<CuckooEntry> {
    (0..size).map(|_| CuckooEntry { key: None, data: None, marker: 0 }).collect()
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        check!(initial_size >= 2);
        let half = (initial_size / 2) as u32;
        Arc::new(RwLock::new(Self {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: allocate_arr(half),
            second_arr: allocate_arr(half),
        }))
    }

    fn first_index(&self, key: &str) -> usize {
        (crate::hash::hash_str(key) % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        (crate::hash::alternative_hash_str(key) % self.max_size) as usize
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }
        self.cur_size += 1;

        let fi = self.first_index(key);
        if self.first_arr[fi].key.is_none() {
            self.first_arr[fi].key = Some(key);
            self.first_arr[fi].data = Some(data);
            return;
        }
        check!(self.first_arr[fi].key.unwrap() != key);

        let si = self.second_index(key);
        if self.second_arr[si].key.is_none() {
            self.second_arr[si].key = Some(key);
            self.second_arr[si].data = Some(data);
            return;
        }
        check!(self.second_arr[si].key.unwrap() != key);

        let mut key = key;
        let mut data = data;

        loop {
            // swap with first entry
            let fi = self.first_index(key);
            {
                let e = &mut self.first_arr[fi];
                std::mem::swap(&mut key, e.key.as_mut().unwrap());
                std::mem::swap(&mut data, e.data.as_mut().unwrap());
                e.marker = self.cur_marker;
            }

            let si = self.second_index(key);
            if self.second_arr[si].marker == self.cur_marker {
                self.cur_size -= 1;
                self.refill();
                self.insert(key, data);
                return;
            }
            if self.second_arr[si].key.is_none() {
                self.second_arr[si].key = Some(key);
                self.second_arr[si].data = Some(data);
                return;
            }
            check!(self.second_arr[si].key.unwrap() != key);

            // swap with second entry
            {
                let e = &mut self.second_arr[si];
                std::mem::swap(&mut key, e.key.as_mut().unwrap());
                std::mem::swap(&mut data, e.data.as_mut().unwrap());
                e.marker = self.cur_marker;
            }

            let fi = self.first_index(key);
            if self.first_arr[fi].marker == self.cur_marker {
                self.cur_size -= 1;
                self.refill();
                self.insert(key, data);
                return;
            }
            if self.first_arr[fi].key.is_none() {
                self.first_arr[fi].key = Some(key);
                self.first_arr[fi].data = Some(data);
                return;
            }
            check!(self.first_arr[fi].key.unwrap() != key);
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let fi = self.first_index(key);
        if let Some(k) = self.first_arr[fi].key {
            if k == key {
                return self.first_arr[fi].data;
            }
        }
        let si = self.second_index(key);
        check!(self.second_arr[si].key.is_some() && self.second_arr[si].key.unwrap() == key);
        self.second_arr[si].data
    }

    fn resize(&mut self) {
        self.recreate(self.max_size * 2);
    }

    fn refill(&mut self) {
        self.recreate(self.max_size + 1);
    }

    fn recreate(&mut self, new_size: u32) {
        let old_first = std::mem::replace(&mut self.first_arr, allocate_arr(new_size));
        let old_second = std::mem::replace(&mut self.second_arr, allocate_arr(new_size));
        self.max_size = new_size;
        self.cur_size = 0;
        self.cur_marker = 0;

        for entry in old_first.into_iter() {
            if let Some(k) = entry.key {
                self.insert(k, entry.data.unwrap());
            }
        }
        for entry in old_second.into_iter() {
            if let Some(k) = entry.key {
                self.insert(k, entry.data.unwrap());
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.first_index(key);
        let ptr = self.first_arr.as_ptr().wrapping_add(idx) as *mut CuckooEntry;
        unsafe { &mut *ptr }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        let ptr = self.second_arr.as_ptr().wrapping_add(idx) as *mut CuckooEntry;
        unsafe { &mut *ptr }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            check!(entry.key.unwrap() != key);
            false
        }
    }

    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        std::mem::swap(key, entry.key.as_mut().unwrap());
        std::mem::swap(data, entry.data.as_mut().unwrap());
    }

    fn free_cukoo_hash_table(self) {
        drop(self);
    }
}
