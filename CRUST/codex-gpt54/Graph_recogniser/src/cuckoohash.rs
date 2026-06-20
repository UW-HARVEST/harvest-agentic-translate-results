use std::sync::{Arc, RwLock};

use crate::hash::{ALTERNATIVE_POWER, EMPTY_KEY, POWER};

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

fn hash_str(key: &str) -> u32 {
    let mut res = 0u32;
    for byte in key.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(byte as u32);
    }
    res
}

fn empty_entry() -> CuckooEntry {
    CuckooEntry {
        key: EMPTY_KEY,
        data: None,
        marker: 0,
    }
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        assert!(initial_size != POWER as usize && initial_size != ALTERNATIVE_POWER as usize);

        let max_size = (initial_size / 2) as u32;
        let mut first_arr = Vec::with_capacity(max_size as usize);
        let mut second_arr = Vec::with_capacity(max_size as usize);
        for _ in 0..max_size {
            first_arr.push(empty_entry());
            second_arr.push(empty_entry());
        }

        Arc::new(RwLock::new(Self {
            cur_size: 0,
            cur_marker: 0,
            max_size,
            first_arr,
            second_arr,
        }))
    }

    pub fn insert(&mut self, mut key: &'static str, mut data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);

        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut first_index = self.first_index(key);
        if self.try_store_in_first(key, data, first_index) {
            return;
        }

        let mut second_index = self.second_index(key);
        if self.try_store_in_second(key, data, second_index) {
            return;
        }

        loop {
            self.swap_with_first(&mut key, &mut data, first_index);
            self.first_arr[first_index].marker = self.cur_marker;

            second_index = self.second_index(key);
            if self.second_arr[second_index].marker == self.cur_marker {
                self.refill();
                self.insert(key, data);
                break;
            }

            if self.try_store_in_second(key, data, second_index) {
                break;
            }

            self.swap_with_second(&mut key, &mut data, second_index);
            self.second_arr[second_index].marker = self.cur_marker;

            first_index = self.first_index(key);
            if self.first_arr[first_index].marker == self.cur_marker {
                self.refill();
                self.insert(key, data);
                break;
            }

            if self.try_store_in_first(key, data, first_index) {
                break;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_entry = &self.first_arr[self.first_index(key)];
        if let Some(entry_key) = first_entry.key && entry_key == key {
            return first_entry.data;
        }

        let second_entry = &self.second_arr[self.second_index(key)];
        if let Some(entry_key) = second_entry.key && entry_key == key {
            return second_entry.data;
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
        assert!(new_size > 0);
        #[cfg(debug_assertions)]
        assert!(new_size as f32 * LOAD_FACTOR + EPS > self.cur_size as f32);

        let old_first = std::mem::replace(&mut self.first_arr, Vec::new());
        let old_second = std::mem::replace(&mut self.second_arr, Vec::new());

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        self.first_arr = Vec::with_capacity(new_size as usize);
        self.second_arr = Vec::with_capacity(new_size as usize);
        for _ in 0..new_size {
            self.first_arr.push(empty_entry());
            self.second_arr.push(empty_entry());
        }

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
        let index = self.first_index(key);
        let ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(index) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let index = self.second_index(key);
        let ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(index) }
    }

    fn try_to_store(
        &mut self,
        key: &'static str,
        data: &'static str,
        entry: &mut CuckooEntry,
    ) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            debug_assert_ne!(entry.key, Some(key));
            false
        }
    }

    fn swap_key_data_entry(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        entry: &mut CuckooEntry,
    ) {
        let swap_key = entry.key.expect("occupied cuckoo entry");
        let swap_data = entry.data.expect("occupied cuckoo entry");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {}

    fn first_index(&self, key: &str) -> usize {
        (hash_str(key) % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        (hash_str(key) % self.max_size) as usize
    }

    fn try_store_in_first(&mut self, key: &'static str, data: &'static str, index: usize) -> bool {
        let entry = &mut self.first_arr[index];
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            debug_assert_ne!(entry.key, Some(key));
            false
        }
    }

    fn try_store_in_second(
        &mut self,
        key: &'static str,
        data: &'static str,
        index: usize,
    ) -> bool {
        let entry = &mut self.second_arr[index];
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            debug_assert_ne!(entry.key, Some(key));
            false
        }
    }

    fn swap_with_first(&mut self, key: &mut &'static str, data: &mut &'static str, index: usize) {
        let entry = &mut self.first_arr[index];
        let swap_key = entry.key.expect("occupied cuckoo entry");
        let swap_data = entry.data.expect("occupied cuckoo entry");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn swap_with_second(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        index: usize,
    ) {
        let entry = &mut self.second_arr[index];
        let swap_key = entry.key.expect("occupied cuckoo entry");
        let swap_data = entry.data.expect("occupied cuckoo entry");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }
}
