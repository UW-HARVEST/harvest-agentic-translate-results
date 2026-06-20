use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash;
const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

fn cuckoo_hash_value(key: &str) -> u32 {
    key.bytes().fold(0, |acc, byte| {
        acc.wrapping_mul(crate::hash::POWER)
            .wrapping_add(byte as u32)
    })
}

fn cuckoo_index(max_size: u32, key: &str) -> usize {
    (cuckoo_hash_value(key) % max_size) as usize
}

fn empty_cuckoo_entry() -> CuckooEntry {
    CuckooEntry {
        key: None,
        data: None,
        marker: 0,
    }
}

fn swap_key_data_entry_impl(key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
    let swap_key = entry.key.replace(*key).expect("occupied entry must contain a key");
    let swap_data = entry.data.replace(*data).expect("occupied entry must contain data");
    *key = swap_key;
    *data = swap_data;
}

fn try_to_store_impl(key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
    if entry.key.is_none() {
        entry.key = Some(key);
        entry.data = Some(data);
        true
    } else {
        #[cfg(debug_assertions)]
        debug_assert_ne!(entry.key, Some(key));
        false
    }
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
        debug_assert!(initial_size >= 2);
        debug_assert!(
            initial_size as u32 != crate::hash::POWER
                && initial_size as u32 != crate::hash::ALTERNATIVE_POWER
        );

        let size = (initial_size / 2) as u32;
        Arc::new(RwLock::new(Self {
            cur_size: 0,
            cur_marker: 0,
            max_size: size,
            first_arr: std::iter::repeat_with(empty_cuckoo_entry)
                .take(size as usize)
                .collect(),
            second_arr: std::iter::repeat_with(empty_cuckoo_entry)
                .take(size as usize)
                .collect(),
        }))
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);

        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut first_idx = cuckoo_index(self.max_size, key);
        if try_to_store_impl(key, data, &mut self.first_arr[first_idx]) {
            return;
        }

        let mut second_idx = cuckoo_index(self.max_size, key);
        if try_to_store_impl(key, data, &mut self.second_arr[second_idx]) {
            return;
        }

        let mut cur_key = key;
        let mut cur_data = data;

        loop {
            swap_key_data_entry_impl(&mut cur_key, &mut cur_data, &mut self.first_arr[first_idx]);
            self.first_arr[first_idx].marker = self.cur_marker;

            second_idx = cuckoo_index(self.max_size, cur_key);
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.insert(cur_key, cur_data);
                break;
            }

            if try_to_store_impl(cur_key, cur_data, &mut self.second_arr[second_idx]) {
                break;
            }

            swap_key_data_entry_impl(&mut cur_key, &mut cur_data, &mut self.second_arr[second_idx]);
            self.second_arr[second_idx].marker = self.cur_marker;

            first_idx = cuckoo_index(self.max_size, cur_key);
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.insert(cur_key, cur_data);
                break;
            }

            if try_to_store_impl(cur_key, cur_data, &mut self.first_arr[first_idx]) {
                break;
            }
        }
    }
    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_idx = cuckoo_index(self.max_size, key);
        let first_entry = &self.first_arr[first_idx];
        if first_entry.key == Some(key) {
            return first_entry.data;
        }

        let second_idx = cuckoo_index(self.max_size, key);
        let second_entry = &self.second_arr[second_idx];
        #[cfg(debug_assertions)]
        debug_assert!(second_entry.key == Some(key));

        second_entry.data
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
        debug_assert!(new_size as f32 * LOAD_FACTOR + EPS > self.cur_size as f32);

        let old_first = std::mem::replace(
            &mut self.first_arr,
            std::iter::repeat_with(empty_cuckoo_entry)
                .take(new_size as usize)
                .collect(),
        );
        let old_second = std::mem::replace(
            &mut self.second_arr,
            std::iter::repeat_with(empty_cuckoo_entry)
                .take(new_size as usize)
                .collect(),
        );

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        for entry in old_first {
            if let (Some(key), Some(data)) = (entry.key, entry.data) {
                self.insert(key, data);
            }
        }

        for entry in old_second {
            if let (Some(key), Some(data)) = (entry.key, entry.data) {
                self.insert(key, data);
            }
        }
    }
    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = cuckoo_index(self.max_size, key);
        let ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(idx) }
    }
    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = cuckoo_index(self.max_size, key);
        let ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(idx) }
    }
    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        let _ = self;
        try_to_store_impl(key, data, entry)
    }
    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        let _ = self;
        swap_key_data_entry_impl(key, data, entry)
    }
    fn free_cukoo_hash_table(self) {
        let _ = self;
    }
}
