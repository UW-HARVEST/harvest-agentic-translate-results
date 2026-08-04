use std::sync::{Arc, RwLock};
use crate::hash::{hash, alternative_hash, Hash, POWER, ALTERNATIVE_POWER};

const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

pub struct CuckooEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
    marker: u32,
}

impl CuckooEntry {
    fn empty() -> Self {
        CuckooEntry {
            key: None,
            data: None,
            marker: 0,
        }
    }
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
        v.push(CuckooEntry::empty());
    }
    v
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        assert!(initial_size as Hash != POWER && initial_size as Hash != ALTERNATIVE_POWER);
        let half = (initial_size / 2) as u32;
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: allocate_arr(half as usize),
            second_arr: allocate_arr(half as usize),
        };
        Arc::new(RwLock::new(table))
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        // Try first array.
        let first_idx = (hash(key) % self.max_size) as usize;
        if self.try_to_store_idx(key, data, true, first_idx) {
            return;
        }

        // Try second array.
        let second_idx = (alternative_hash(key) % self.max_size) as usize;
        if self.try_to_store_idx(key, data, false, second_idx) {
            return;
        }

        // Cuckoo eviction loop.
        let mut cur_key: &'static str = key;
        let mut cur_data: &'static str = data;
        let mut first_idx = first_idx;
        let mut second_idx;

        loop {
            // Swap with first_arr[first_idx].
            self.swap_idx(&mut cur_key, &mut cur_data, true, first_idx);
            self.first_arr[first_idx].marker = self.cur_marker;

            second_idx = (alternative_hash(cur_key) % self.max_size) as usize;
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1; // refill re-inserts; the upcoming insert will increment again
                self.insert(cur_key, cur_data);
                break;
            }

            if self.try_to_store_idx(cur_key, cur_data, false, second_idx) {
                break;
            }

            // Swap with second_arr[second_idx].
            self.swap_idx(&mut cur_key, &mut cur_data, false, second_idx);
            self.second_arr[second_idx].marker = self.cur_marker;

            first_idx = (hash(cur_key) % self.max_size) as usize;
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.cur_size -= 1;
                self.insert(cur_key, cur_data);
                break;
            }

            if self.try_to_store_idx(cur_key, cur_data, true, first_idx) {
                break;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        if self.max_size == 0 {
            return None;
        }
        // First array check.
        let h1 = {
            let mut res: Hash = 0;
            for b in key.bytes() {
                res = res.wrapping_mul(POWER).wrapping_add(b as Hash);
            }
            res
        };
        let idx1 = (h1 % self.max_size) as usize;
        let e1 = &self.first_arr[idx1];
        if let Some(k) = e1.key {
            if k == key {
                return e1.data;
            }
        }
        // Second array check.
        let h2 = {
            let mut res: Hash = 0;
            for b in key.bytes() {
                res = res.wrapping_mul(ALTERNATIVE_POWER).wrapping_add(b as Hash);
            }
            res
        };
        let idx2 = (h2 % self.max_size) as usize;
        let e2 = &self.second_arr[idx2];
        if let Some(k) = e2.key {
            if k == key {
                return e2.data;
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
        assert!(new_size > 0);
        let old_first = std::mem::replace(&mut self.first_arr, allocate_arr(new_size as usize));
        let old_second = std::mem::replace(&mut self.second_arr, allocate_arr(new_size as usize));
        let _old_size = self.max_size;

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

    // The following helper signatures return &mut from &self, which is impossible
    // in fully safe Rust. We provide minimal-unsafe implementations that satisfy
    // the prescribed signatures. Internal logic uses index-based safe paths above.

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let h = {
            let mut res: Hash = 0;
            for b in key.bytes() {
                res = res.wrapping_mul(POWER).wrapping_add(b as Hash);
            }
            res
        };
        let idx = (h % self.max_size) as usize;
        let ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *ptr.add(idx) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let h = {
            let mut res: Hash = 0;
            for b in key.bytes() {
                res = res.wrapping_mul(ALTERNATIVE_POWER).wrapping_add(b as Hash);
            }
            res
        };
        let idx = (h % self.max_size) as usize;
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
        let swap_key = entry.key.unwrap_or("");
        let swap_data = entry.data.unwrap_or("");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Drop self; vectors will free.
        drop(self);
    }

    // ---- Internal index-based safe helpers ----

    fn try_to_store_idx(&mut self, key: &'static str, data: &'static str, first: bool, idx: usize) -> bool {
        let entry = if first {
            &mut self.first_arr[idx]
        } else {
            &mut self.second_arr[idx]
        };
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            false
        }
    }

    fn swap_idx(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        first: bool,
        idx: usize,
    ) {
        let entry = if first {
            &mut self.first_arr[idx]
        } else {
            &mut self.second_arr[idx]
        };
        let swap_key = entry.key.unwrap_or("");
        let swap_data = entry.data.unwrap_or("");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }
}
