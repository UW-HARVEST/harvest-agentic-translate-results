use std::sync::{Arc, RwLock};
use crate::hash::{rehash, compare_keys, POWER, REHASHER};

const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

pub struct OpenEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
}

impl OpenEntry {
    fn empty() -> Self {
        OpenEntry { key: None, data: None }
    }
}

pub struct OpenHashTable {
    cur_size: usize,
    max_size: usize,
    arr: Vec<OpenEntry>,
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        debug_assert!(initial_size != 0);
        debug_assert!(initial_size as u32 != POWER && initial_size as u32 != REHASHER);

        let mut arr = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            arr.push(OpenEntry::empty());
        }

        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }

    /// Returns the index of the cell where `key` is/should be stored.
    fn query(&self, key: &str) -> usize {
        // Mirror the C `query` which probes via rehash until an empty cell or a
        // cell with the matching key is found.
        let mut h = crate::hash::hash_for_str(key, POWER);
        loop {
            let idx = (h as usize) % self.max_size;
            let entry = &self.arr[idx];
            match entry.key {
                None => return idx,
                Some(k) if k == key => return idx,
                _ => {
                    h = rehash(h);
                }
            }
        }
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        debug_assert!(
            (self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS_OR_ZERO
        );

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let new_max = self.max_size * 2;
            let mut new_arr: Vec<OpenEntry> = Vec::with_capacity(new_max);
            for _ in 0..new_max {
                new_arr.push(OpenEntry::empty());
            }
            let old_arr = std::mem::replace(&mut self.arr, new_arr);
            self.max_size = new_max;

            // Rehash all old entries.
            for entry in old_arr.into_iter() {
                if let (Some(k), Some(d)) = (entry.key, entry.data) {
                    let idx = self.query(k);
                    self.arr[idx].key = Some(k);
                    self.arr[idx].data = Some(d);
                }
            }
        }

        let idx = self.query(key);
        debug_assert!(self.arr[idx].key.is_none());
        self.arr[idx].key = Some(key);
        self.arr[idx].data = Some(data);
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        let entry = &self.arr[idx];
        match entry.key {
            Some(k) => {
                let _ = compare_keys(k, k); // preserve ordering invocation similar to C
                entry.data
            }
            None => None,
        }
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}

#[cfg(debug_assertions)]
const EPS_OR_ZERO: f32 = EPS;
#[cfg(not(debug_assertions))]
const EPS_OR_ZERO: f32 = 0.0;
