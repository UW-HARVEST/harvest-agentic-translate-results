use std::collections::HashMap;
use std::sync::{Arc, RwLock};
#[allow(unused_imports)]
use crate::log::{LogType, Logger};
#[allow(unused_imports)]
use crate::check;
use crate::hash::{hash, alternative_hash, compare_keys, POWER, ALTERNATIVE_POWER};
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
impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        crate::check!(initial_size >= 2);
        crate::check!(initial_size != POWER as usize && initial_size != ALTERNATIVE_POWER as usize);
        let half = (initial_size / 2) as u32;
        let mut first_arr: Vec<CuckooEntry> = Vec::with_capacity(half as usize);
        let mut second_arr: Vec<CuckooEntry> = Vec::with_capacity(half as usize);
        for _ in 0..half {
            first_arr.push(CuckooEntry { key: None, data: None, marker: 0 });
            second_arr.push(CuckooEntry { key: None, data: None, marker: 0 });
        }
        Arc::new(RwLock::new(CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr,
            second_arr,
        }))
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);
        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }
        self.cur_size += 1;

        // Try first entry
        let h1 = hash(key) as usize % self.max_size as usize;
        if self.first_arr[h1].key.is_none() {
            self.first_arr[h1].key = Some(key);
            self.first_arr[h1].data = Some(data);
            return;
        } else {
            crate::check!(compare_keys(self.first_arr[h1].key.unwrap(), key) != std::cmp::Ordering::Equal);
        }

        // Try second entry
        let h2 = alternative_hash(key) as usize % self.max_size as usize;
        if self.second_arr[h2].key.is_none() {
            self.second_arr[h2].key = Some(key);
            self.second_arr[h2].data = Some(data);
            return;
        } else {
            crate::check!(compare_keys(self.second_arr[h2].key.unwrap(), key) != std::cmp::Ordering::Equal);
        }

        let mut cur_key = key;
        let mut cur_data = data;
        let mut first_idx = h1;

        loop {
            // swap with first entry
            let swap_key = self.first_arr[first_idx].key.unwrap();
            let swap_data = self.first_arr[first_idx].data.unwrap();
            self.first_arr[first_idx].key = Some(cur_key);
            self.first_arr[first_idx].data = Some(cur_data);
            self.first_arr[first_idx].marker = self.cur_marker;
            cur_key = swap_key;
            cur_data = swap_data;

            // get second entry for cur_key
            let second_idx = alternative_hash(cur_key) as usize % self.max_size as usize;
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1; // recursive insert will increment
                self.insert(cur_key, cur_data);
                return;
            }

            if self.second_arr[second_idx].key.is_none() {
                self.second_arr[second_idx].key = Some(cur_key);
                self.second_arr[second_idx].data = Some(cur_data);
                return;
            } else {
                crate::check!(compare_keys(self.second_arr[second_idx].key.unwrap(), cur_key) != std::cmp::Ordering::Equal);
            }

            // swap with second entry
            let swap_key = self.second_arr[second_idx].key.unwrap();
            let swap_data = self.second_arr[second_idx].data.unwrap();
            self.second_arr[second_idx].key = Some(cur_key);
            self.second_arr[second_idx].data = Some(cur_data);
            self.second_arr[second_idx].marker = self.cur_marker;
            cur_key = swap_key;
            cur_data = swap_data;

            // get first entry for cur_key
            first_idx = hash(cur_key) as usize % self.max_size as usize;
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1; // recursive insert will increment
                self.insert(cur_key, cur_data);
                return;
            }

            if self.first_arr[first_idx].key.is_none() {
                self.first_arr[first_idx].key = Some(cur_key);
                self.first_arr[first_idx].data = Some(cur_data);
                return;
            } else {
                crate::check!(compare_keys(self.first_arr[first_idx].key.unwrap(), cur_key) != std::cmp::Ordering::Equal);
            }
        }
    }
    pub fn find(&self, key: &str) -> Option<&'static str> {
        let static_key = unsafe { std::mem::transmute::<&str, &'static str>(key) };
        let h1 = hash(static_key) as usize % self.max_size as usize;
        if let Some(cur_key) = self.first_arr[h1].key {
            if cur_key == key {
                return self.first_arr[h1].data;
            }
        }
        let h2 = alternative_hash(static_key) as usize % self.max_size as usize;
        if let Some(cur_key) = self.second_arr[h2].key {
            if cur_key == key {
                return self.second_arr[h2].data;
            }
        }
        crate::check!(false);
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
        crate::check!(new_size > 0);
        #[cfg(debug_assertions)]
        {
            crate::check!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);
        }

        let old_size = self.max_size;
        let old_first = std::mem::take(&mut self.first_arr);
        let old_second = std::mem::take(&mut self.second_arr);

        let mut new_first: Vec<CuckooEntry> = Vec::with_capacity(new_size as usize);
        let mut new_second: Vec<CuckooEntry> = Vec::with_capacity(new_size as usize);
        for _ in 0..new_size {
            new_first.push(CuckooEntry { key: None, data: None, marker: 0 });
            new_second.push(CuckooEntry { key: None, data: None, marker: 0 });
        }
        self.first_arr = new_first;
        self.second_arr = new_second;
        self.max_size = new_size;
        self.cur_size = 0;
        self.cur_marker = 0;

        for i in 0..old_size as usize {
            if let (Some(k), Some(d)) = (old_first[i].key, old_first[i].data) {
                self.insert(k, d);
            }
        }
        for i in 0..old_size as usize {
            if let (Some(k), Some(d)) = (old_second[i].key, old_second[i].data) {
                self.insert(k, d);
            }
        }
    }
    fn get_first_entry(&mut self, key: &str) -> &mut CuckooEntry {
        let static_key = unsafe { std::mem::transmute::<&str, &'static str>(key) };
        let h = hash(static_key) as usize % self.max_size as usize;
        &mut self.first_arr[h]
    }
    fn get_second_entry(&mut self, key: &str) -> &mut CuckooEntry {
        let static_key = unsafe { std::mem::transmute::<&str, &'static str>(key) };
        let h = alternative_hash(static_key) as usize % self.max_size as usize;
        &mut self.second_arr[h]
    }
    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            crate::check!(compare_keys(entry.key.unwrap(), key) != std::cmp::Ordering::Equal);
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
        // Vec drops automatically
    }
}

impl Default for CuckooEntry {
    fn default() -> Self {
        CuckooEntry { key: None, data: None, marker: 0 }
    }
}
