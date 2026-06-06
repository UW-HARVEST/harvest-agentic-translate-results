use std::sync::{Arc, RwLock};
use crate::hash::{alternative_hash, POWER, ALTERNATIVE_POWER, hash_str_by_power};

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

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        debug_assert!(initial_size >= 2);
        debug_assert!(
            initial_size != POWER as usize && initial_size != ALTERNATIVE_POWER as usize
        );

        let half = (initial_size / 2) as u32;
        let mut first_arr = Vec::with_capacity(half as usize);
        let mut second_arr = Vec::with_capacity(half as usize);
        for _ in 0..half {
            first_arr.push(CuckooEntry::empty());
            second_arr.push(CuckooEntry::empty());
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
        self.cur_marker += 1;

        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut k = key;
        let mut d = data;

        // Try first entry
        let mut first_idx = self.first_index(k);
        if self.try_store_at_first(first_idx, k, d) {
            return;
        }

        // Try second entry
        let mut second_idx = self.second_index(k);
        if self.try_store_at_second(second_idx, k, d) {
            return;
        }

        loop {
            // Swap with first_idx
            let old_first = std::mem::replace(
                &mut self.first_arr[first_idx],
                CuckooEntry {
                    key: Some(k),
                    data: Some(d),
                    marker: self.cur_marker,
                },
            );
            // Now we have to evict old_first
            if let (Some(ek), Some(ed)) = (old_first.key, old_first.data) {
                k = ek;
                d = ed;
            } else {
                // Slot was empty (shouldn't happen here, but be safe)
                return;
            }

            second_idx = self.second_index(k);
            if self.second_arr[second_idx].marker == self.cur_marker {
                self.refill();
                self.insert(k, d);
                break;
            }

            if self.try_store_at_second(second_idx, k, d) {
                break;
            }

            // Swap with second_idx
            let old_second = std::mem::replace(
                &mut self.second_arr[second_idx],
                CuckooEntry {
                    key: Some(k),
                    data: Some(d),
                    marker: self.cur_marker,
                },
            );
            if let (Some(ek), Some(ed)) = (old_second.key, old_second.data) {
                k = ek;
                d = ed;
            } else {
                return;
            }

            first_idx = self.first_index(k);
            if self.first_arr[first_idx].marker == self.cur_marker {
                self.refill();
                self.insert(k, d);
                break;
            }

            if self.try_store_at_first(first_idx, k, d) {
                break;
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_idx = self.first_index(key);
        let entry = &self.first_arr[first_idx];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }

        let second_idx = self.second_index(key);
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
        debug_assert!(new_size > 0);
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                (new_size as f32) * LOAD_FACTOR + EPS > (self.cur_size as f32)
            );
        }

        let old_first = std::mem::take(&mut self.first_arr);
        let old_second = std::mem::take(&mut self.second_arr);

        // Reinitialize
        let mut new_first = Vec::with_capacity(new_size as usize);
        let mut new_second = Vec::with_capacity(new_size as usize);
        for _ in 0..new_size {
            new_first.push(CuckooEntry::empty());
            new_second.push(CuckooEntry::empty());
        }
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;
        self.first_arr = new_first;
        self.second_arr = new_second;

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
        // Safety: we are returning a &mut to a vector element, derived from
        // an immutable reference to self. Callers must ensure exclusive access
        // for the duration of the returned reference. In this codebase the
        // raw mutation primitives are used carefully through `&mut self`
        // public methods, so this internal helper is only safely used
        // alongside disciplined access.
        unsafe {
            let ptr = self.first_arr.as_ptr().add(idx) as *mut CuckooEntry;
            &mut *ptr
        }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        // Safety: see `get_first_entry`.
        unsafe {
            let ptr = self.second_arr.as_ptr().add(idx) as *mut CuckooEntry;
            &mut *ptr
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

    fn swap_key_data_entry(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        entry: &mut CuckooEntry,
    ) {
        let swap_key = entry.key.unwrap_or("");
        let swap_data = entry.data.unwrap_or("");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Dropping consumes self, freeing the vectors.
        drop(self);
    }

    // ---- Private helpers using indices ----

    fn first_index(&self, key: &str) -> usize {
        let h = hash_str_by_power(key, POWER);
        (h as usize) % (self.max_size as usize)
    }

    fn second_index(&self, key: &str) -> usize {
        // The C code uses `hash` for both first and second; reproducing that.
        // (Despite the existence of `alternative_hash`, the actual C code in
        // get_second_entry calls `hash(key)`.)
        let h = hash_str_by_power(key, POWER);
        (h as usize) % (self.max_size as usize)
    }

    fn try_store_at_first(
        &mut self,
        idx: usize,
        key: &'static str,
        data: &'static str,
    ) -> bool {
        if self.first_arr[idx].key.is_none() {
            self.first_arr[idx].key = Some(key);
            self.first_arr[idx].data = Some(data);
            true
        } else {
            false
        }
    }

    fn try_store_at_second(
        &mut self,
        idx: usize,
        key: &'static str,
        data: &'static str,
    ) -> bool {
        if self.second_arr[idx].key.is_none() {
            self.second_arr[idx].key = Some(key);
            self.second_arr[idx].data = Some(data);
            true
        } else {
            false
        }
    }
}

// Implement Default for CuckooEntry so std::mem::take works on Vec<CuckooEntry>
impl Default for CuckooEntry {
    fn default() -> Self {
        CuckooEntry::empty()
    }
}

// Suppress unused warnings for the alternative_hash helper, which is part
// of the public API in `hash.rs`.
#[allow(dead_code)]
fn _use_alternative_hash(k: &'static str) -> u32 {
    alternative_hash(k)
}
