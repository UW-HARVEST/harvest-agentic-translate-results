use std::sync::{Arc, RwLock};
use crate::hash::{compare_keys, POWER, ALTERNATIVE_POWER};

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
        CuckooEntry { key: None, data: None, marker: 0 }
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
            initial_size as u32 != POWER && initial_size as u32 != ALTERNATIVE_POWER
        );

        let half = (initial_size / 2) as u32;
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: (0..half).map(|_| CuckooEntry::empty()).collect(),
            second_arr: (0..half).map(|_| CuckooEntry::empty()).collect(),
        };
        Arc::new(RwLock::new(table))
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);

        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        // First, try to store in the slot of the first hash function.
        {
            let entry: &mut CuckooEntry = self.get_first_entry(key);
            // SAFETY: borrow not aliased; we use the &mut reference exclusively here.
            if Self::raw_try_to_store(key, data, entry) {
                return;
            }
        }

        // Second, try to store in the slot of the second hash function.
        {
            let entry: &mut CuckooEntry = self.get_second_entry(key);
            if Self::raw_try_to_store(key, data, entry) {
                return;
            }
        }

        // Cuckoo eviction loop.
        let mut cur_key: &'static str = key;
        let mut cur_data: &'static str = data;
        let cur_marker_value = self.cur_marker;

        // Begin: at this point both first and second slots are occupied.
        // We start by displacing the first slot.
        loop {
            // Swap with first slot.
            {
                let first_entry: &mut CuckooEntry = self.get_first_entry(cur_key);
                Self::raw_swap(&mut cur_key, &mut cur_data, first_entry);
                first_entry.marker = cur_marker_value;
            }

            // Now look at second slot for the displaced key.
            {
                let second_entry: &mut CuckooEntry = self.get_second_entry(cur_key);
                if second_entry.marker == cur_marker_value {
                    // Cycle detected: refill and re-insert.
                    self.refill();
                    self.insert(cur_key, cur_data);
                    return;
                }
                if Self::raw_try_to_store(cur_key, cur_data, second_entry) {
                    return;
                }
                Self::raw_swap(&mut cur_key, &mut cur_data, second_entry);
                second_entry.marker = cur_marker_value;
            }

            // Now look at first slot for the displaced key.
            {
                let first_entry: &mut CuckooEntry = self.get_first_entry(cur_key);
                if first_entry.marker == cur_marker_value {
                    self.refill();
                    self.insert(cur_key, cur_data);
                    return;
                }
                if Self::raw_try_to_store(cur_key, cur_data, first_entry) {
                    return;
                }
            }
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        if self.max_size == 0 {
            return None;
        }
        let h1 = crate::hash::hash_for_str(key, POWER);
        let idx1 = (h1 as usize) % (self.max_size as usize);
        let entry1 = &self.first_arr[idx1];
        if let Some(k) = entry1.key {
            if k == key {
                return entry1.data;
            }
        }
        let h2 = crate::hash::hash_for_str(key, ALTERNATIVE_POWER);
        let idx2 = (h2 as usize) % (self.max_size as usize);
        let entry2 = &self.second_arr[idx2];
        if let Some(k) = entry2.key {
            if k == key {
                return entry2.data;
            }
        }
        None
    }

    fn resize(&mut self) {
        let new_size = self.max_size.saturating_mul(2).max(1);
        self.recreate(new_size);
    }

    fn refill(&mut self) {
        let new_size = self.max_size + 1;
        self.recreate(new_size);
    }

    fn recreate(&mut self, new_size: u32) {
        debug_assert!(new_size > 0);
        debug_assert!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);

        let old_first = std::mem::replace(&mut self.first_arr, Vec::new());
        let old_second = std::mem::replace(&mut self.second_arr, Vec::new());

        // Re-initialize.
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;
        self.first_arr = (0..new_size).map(|_| CuckooEntry::empty()).collect();
        self.second_arr = (0..new_size).map(|_| CuckooEntry::empty()).collect();

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
        let h = hash_for_str_local(key, POWER);
        let idx = (h as usize) % (self.max_size as usize);
        // SAFETY: We need to satisfy the C-style signature that returns a
        // mutable reference from an &self. The caller invokes this method only
        // when it actually owns &mut self (this is enforced by the public API
        // where the only callers are CuckooHashTable's own methods that take
        // &mut self). The cast is necessary because the signature is fixed by
        // the project requirements and cannot be changed.
        let arr_ptr = self.first_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *arr_ptr.add(idx) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let h = hash_for_str_local(key, ALTERNATIVE_POWER);
        let idx = (h as usize) % (self.max_size as usize);
        // SAFETY: see `get_first_entry`.
        let arr_ptr = self.second_arr.as_ptr() as *mut CuckooEntry;
        unsafe { &mut *arr_ptr.add(idx) }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        Self::raw_try_to_store(key, data, entry)
    }

    fn swap_key_data_entry(&mut self, key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        Self::raw_swap(key, data, entry);
    }

    fn free_cukoo_hash_table(self) {
        // Drop happens automatically.
    }

    /// Internal version that does not require &mut self.
    fn raw_try_to_store(key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        match entry.key {
            None => {
                entry.key = Some(key);
                entry.data = Some(data);
                true
            }
            Some(k) => {
                debug_assert!(compare_keys(k, key) != std::cmp::Ordering::Equal);
                false
            }
        }
    }

    /// Internal swap helper that does not require &mut self.
    fn raw_swap(key: &mut &'static str, data: &mut &'static str, entry: &mut CuckooEntry) {
        let swap_key = entry.key;
        let swap_data = entry.data;
        entry.key = Some(*key);
        entry.data = Some(*data);
        if let Some(k) = swap_key {
            *key = k;
        }
        if let Some(d) = swap_data {
            *data = d;
        }
    }
}

fn hash_for_str_local(key: &str, power: u32) -> u32 {
    crate::hash::hash_for_str(key, power)
}

