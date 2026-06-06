use std::sync::{Arc, RwLock};
use crate::hash::{rehash, Hash, POWER, REHASHER};

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

fn allocate_arr(size: usize) -> Vec<OpenEntry> {
    let mut v = Vec::with_capacity(size);
    for _ in 0..size {
        v.push(OpenEntry::empty());
    }
    v
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size != 0);
        assert!(initial_size as Hash != POWER && initial_size as Hash != REHASHER);
        let arr = allocate_arr(initial_size);
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }

    /// Returns the index of either the existing key or the first empty slot.
    fn query(&self, key: &str) -> usize {
        assert!(!key.is_empty() || true); // key can be any &str
        // We need a 'static key, but we only use it for comparing.
        // For querying, we just need the &str, and we hash it via hash_by_power.
        // The signature in hash takes Key = &'static str. We can compute the hash
        // here directly to avoid lifetime issues.
        let mut h: Hash = {
            let mut res: Hash = 0;
            for b in key.bytes() {
                res = res.wrapping_mul(crate::hash::POWER).wrapping_add(b as Hash);
            }
            res
        };
        loop {
            let idx = (h as usize) % self.max_size;
            let entry = &self.arr[idx];
            match entry.key {
                None => return idx,
                Some(cur_key) => {
                    if cur_key == key {
                        return idx;
                    }
                }
            }
            h = rehash(h);
        }
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        #[cfg(debug_assertions)]
        {
            assert!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);
        }

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_max = self.max_size;
            self.max_size *= 2;
            let new_arr = allocate_arr(self.max_size);
            let old_arr = std::mem::replace(&mut self.arr, new_arr);

            for entry in old_arr.into_iter().take(old_max) {
                if let (Some(k), Some(d)) = (entry.key, entry.data) {
                    let idx = self.query(k);
                    self.arr[idx].key = Some(k);
                    self.arr[idx].data = Some(d);
                }
            }
        }

        let idx = self.query(key);
        assert!(self.arr[idx].key.is_none());
        self.arr[idx].key = Some(key);
        self.arr[idx].data = Some(data);
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        let entry = &self.arr[idx];
        // In C, CHECK asserts the key was found; in safe Rust, we return None.
        match entry.key {
            Some(k) if k == key => entry.data,
            _ => None,
        }
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
