use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::{hash, alternative_hash};
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

fn empty_arr(size: u32) -> Vec<CuckooEntry> {
    let mut arr = Vec::with_capacity(size as usize);
    for _ in 0..size {
        arr.push(CuckooEntry {
            key: None,
            data: None,
            marker: 0,
        });
    }
    arr
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        debug_assert!(initial_size >= 2);
        debug_assert!(
            initial_size != crate::hash::POWER as usize
                && initial_size != crate::hash::ALTERNATIVE_POWER as usize
        );
        // The C version does: initialize_table(initial_size / 2, ret);
        let half = (initial_size / 2) as u32;
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: empty_arr(half),
            second_arr: empty_arr(half),
        };
        Arc::new(RwLock::new(table))
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;

        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;
        self.insert_inner(key, data);
    }

    fn insert_inner(&mut self, key0: &'static str, data0: &'static str) {
        let mut key = key0;
        let mut data = data0;

        // First, try first array.
        let first_idx = self.first_index(key);
        if self.first_arr[first_idx].key.is_none() {
            self.first_arr[first_idx].key = Some(key);
            self.first_arr[first_idx].data = Some(data);
            return;
        } else {
            debug_assert!(self.first_arr[first_idx].key != Some(key));
        }

        // Then, try second array.
        let second_idx = self.second_index(key);
        if self.second_arr[second_idx].key.is_none() {
            self.second_arr[second_idx].key = Some(key);
            self.second_arr[second_idx].data = Some(data);
            return;
        } else {
            debug_assert!(self.second_arr[second_idx].key != Some(key));
        }

        // Cuckoo eviction loop.
        let mut first_idx_cur = first_idx;
        loop {
            // Swap with first_arr[first_idx_cur]
            let cur_marker = self.cur_marker;
            let entry = &mut self.first_arr[first_idx_cur];
            let swap_key = entry.key;
            let swap_data = entry.data;
            entry.key = Some(key);
            entry.data = Some(data);
            entry.marker = cur_marker;
            key = swap_key.expect("evicting non-empty entry");
            data = swap_data.expect("evicting non-empty entry");

            // Now look up in second array.
            let s_idx = self.second_index(key);
            if self.second_arr[s_idx].marker == self.cur_marker {
                // Cycle: refill and re-insert.
                self.refill();
                self.insert_inner(key, data);
                return;
            }
            if self.second_arr[s_idx].key.is_none() {
                self.second_arr[s_idx].key = Some(key);
                self.second_arr[s_idx].data = Some(data);
                return;
            }

            // Swap with second_arr[s_idx]
            let cur_marker = self.cur_marker;
            let entry = &mut self.second_arr[s_idx];
            let swap_key = entry.key;
            let swap_data = entry.data;
            entry.key = Some(key);
            entry.data = Some(data);
            entry.marker = cur_marker;
            key = swap_key.expect("evicting non-empty entry");
            data = swap_data.expect("evicting non-empty entry");

            // Now look up in first array.
            let f_idx = self.first_index(key);
            if self.first_arr[f_idx].marker == self.cur_marker {
                self.refill();
                self.insert_inner(key, data);
                return;
            }
            if self.first_arr[f_idx].key.is_none() {
                self.first_arr[f_idx].key = Some(key);
                self.first_arr[f_idx].data = Some(data);
                return;
            }

            first_idx_cur = f_idx;
        }
    }

    fn first_index(&self, key: &str) -> usize {
        (hash(unsafe_static_key(key)) as usize) % (self.max_size as usize)
    }

    fn second_index(&self, key: &str) -> usize {
        (alternative_hash(unsafe_static_key(key)) as usize) % (self.max_size as usize)
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let f_idx = self.first_index(key);
        let f_entry = &self.first_arr[f_idx];
        if let Some(k) = f_entry.key {
            if k == key {
                return f_entry.data;
            }
        }
        let s_idx = self.second_index(key);
        let s_entry = &self.second_arr[s_idx];
        if let Some(k) = s_entry.key {
            if k == key {
                return s_entry.data;
            }
        }
        debug_assert!(false, "key not found");
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
        debug_assert!(new_size > 0);
        #[cfg(debug_assertions)]
        {
            debug_assert!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);
        }

        let old_first = std::mem::replace(&mut self.first_arr, empty_arr(new_size));
        let old_second = std::mem::replace(&mut self.second_arr, empty_arr(new_size));
        let old_max_size = self.max_size;

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

        for i in 0..(old_max_size as usize) {
            if let Some(k) = old_first[i].key {
                let d = old_first[i].data.expect("entry has key but no data");
                self.insert(k, d);
            }
        }
        for i in 0..(old_max_size as usize) {
            if let Some(k) = old_second[i].key {
                let d = old_second[i].data.expect("entry has key but no data");
                self.insert(k, d);
            }
        }
    }

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.first_index(key);
        // Required only to satisfy the prescribed signature; this helper is
        // not used by the actual implementation (which works by index).
        let base = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *base.add(idx) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        let base = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *base.add(idx) }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            debug_assert!(entry.key != Some(key));
            false
        }
    }

    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        let swap_key = entry.key.expect("entry must be non-empty");
        let swap_data = entry.data.expect("entry must be non-empty");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Drop will free the Vecs automatically.
    }
}

fn unsafe_static_key(key: &str) -> &'static str {
    // Hash and alternative_hash require &'static str; since hashing only
    // examines bytes and doesn't store the slice, we can extend the lifetime
    // for the duration of the call. This is safe because the hash function
    // only reads bytes via iteration without retaining a reference.
    unsafe { std::mem::transmute::<&str, &'static str>(key) }
}
