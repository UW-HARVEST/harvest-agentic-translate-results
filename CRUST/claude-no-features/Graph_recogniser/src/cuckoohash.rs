use std::sync::{Arc, RwLock};
use crate::hash::{hash, alternative_hash, POWER, ALTERNATIVE_POWER};

const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;
#[cfg(not(debug_assertions))]
const EPS: f32 = 0.0;

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

/// Compute first-array hash for arbitrary &str (mirrors hash() but without &'static).
fn compute_first_hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for byte in key.bytes() {
        h = h.wrapping_mul(POWER).wrapping_add(byte as u32);
    }
    h
}

/// Compute second-array hash for arbitrary &str (mirrors alternative_hash()).
fn compute_second_hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for byte in key.bytes() {
        h = h.wrapping_mul(ALTERNATIVE_POWER).wrapping_add(byte as u32);
    }
    h
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        assert!(initial_size as u32 != POWER && initial_size as u32 != ALTERNATIVE_POWER);
        let half_size = (initial_size / 2) as u32;
        let mut first_arr = Vec::with_capacity(half_size as usize);
        let mut second_arr = Vec::with_capacity(half_size as usize);
        for _ in 0..half_size {
            first_arr.push(CuckooEntry::empty());
            second_arr.push(CuckooEntry::empty());
        }
        Arc::new(RwLock::new(CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half_size,
            first_arr,
            second_arr,
        }))
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;
        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut k = key;
        let mut d = data;

        // Try first slot
        {
            let idx = (compute_first_hash(k) as usize) % (self.max_size as usize);
            if self.first_arr[idx].key.is_none() {
                self.first_arr[idx].key = Some(k);
                self.first_arr[idx].data = Some(d);
                return;
            }
        }

        // Try second slot
        {
            let idx = (compute_second_hash(k) as usize) % (self.max_size as usize);
            if self.second_arr[idx].key.is_none() {
                self.second_arr[idx].key = Some(k);
                self.second_arr[idx].data = Some(d);
                return;
            }
        }

        // Cuckoo eviction loop
        loop {
            // Swap with first_arr entry
            {
                let idx = (compute_first_hash(k) as usize) % (self.max_size as usize);
                let entry = &mut self.first_arr[idx];
                let swap_key = entry.key.unwrap();
                let swap_data = entry.data.unwrap();
                entry.key = Some(k);
                entry.data = Some(d);
                entry.marker = self.cur_marker;
                k = swap_key;
                d = swap_data;
            }

            // Try to place in second
            {
                let idx = (compute_second_hash(k) as usize) % (self.max_size as usize);
                if self.second_arr[idx].marker == self.cur_marker {
                    // Cycle detected
                    self.refill();
                    self.insert(k, d);
                    return;
                }
                if self.second_arr[idx].key.is_none() {
                    self.second_arr[idx].key = Some(k);
                    self.second_arr[idx].data = Some(d);
                    return;
                }
            }

            // Swap with second_arr entry
            {
                let idx = (compute_second_hash(k) as usize) % (self.max_size as usize);
                let entry = &mut self.second_arr[idx];
                let swap_key = entry.key.unwrap();
                let swap_data = entry.data.unwrap();
                entry.key = Some(k);
                entry.data = Some(d);
                entry.marker = self.cur_marker;
                k = swap_key;
                d = swap_data;
            }

            // Try to place in first
            {
                let idx = (compute_first_hash(k) as usize) % (self.max_size as usize);
                if self.first_arr[idx].marker == self.cur_marker {
                    // Cycle detected
                    self.refill();
                    self.insert(k, d);
                    return;
                }
                if self.first_arr[idx].key.is_none() {
                    self.first_arr[idx].key = Some(k);
                    self.first_arr[idx].data = Some(d);
                    return;
                }
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx1 = (compute_first_hash(key) as usize) % (self.max_size as usize);
        let entry = &self.first_arr[idx1];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }
        let idx2 = (compute_second_hash(key) as usize) % (self.max_size as usize);
        let entry = &self.second_arr[idx2];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
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
        assert!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);

        let old_first = std::mem::take(&mut self.first_arr);
        let old_second = std::mem::take(&mut self.second_arr);
        let old_size = self.max_size;

        // Re-initialize
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;
        self.first_arr = Vec::with_capacity(new_size as usize);
        self.second_arr = Vec::with_capacity(new_size as usize);
        for _ in 0..new_size {
            self.first_arr.push(CuckooEntry::empty());
            self.second_arr.push(CuckooEntry::empty());
        }
        // Suppress old_size unused warning
        let _ = old_size;

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
        let idx = (compute_first_hash(key) as usize) % (self.max_size as usize);
        // SAFETY: caller must ensure exclusive access; needed to satisfy required signature.
        let arr_ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *arr_ptr.add(idx) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = (compute_second_hash(key) as usize) % (self.max_size as usize);
        // SAFETY: caller must ensure exclusive access; needed to satisfy required signature.
        let arr_ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *arr_ptr.add(idx) }
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
        // Drop is sufficient
        drop(self);
    }
}

// Reference unused functions to silence dead-code warnings while preserving required signatures.
#[allow(dead_code)]
fn _silence_unused() {
    let _ = hash;
    let _ = alternative_hash;
}
