use std::sync::{Arc, RwLock};
use crate::hash::{rehash, POWER, REHASHER};

const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;
#[cfg(not(debug_assertions))]
const EPS: f32 = 0.0;

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
        assert!(initial_size != 0);
        assert!(initial_size as u32 != POWER && initial_size as u32 != REHASHER);
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

    /// Find the index where `key` is stored, or where it should be inserted (an empty slot).
    fn query(&self, key: &str) -> usize {
        // hash() takes &'static str, but algorithm only depends on bytes; use hash_by_power directly
        let mut h: u32 = 0;
        for byte in key.bytes() {
            h = h.wrapping_mul(crate::hash::POWER).wrapping_add(byte as u32);
        }
        loop {
            let idx = (h as usize) % self.max_size;
            match self.arr[idx].key {
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
        assert!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let new_max_size = self.max_size * 2;
            let old_arr = std::mem::replace(&mut self.arr, {
                let mut new_arr = Vec::with_capacity(new_max_size);
                for _ in 0..new_max_size {
                    new_arr.push(OpenEntry::empty());
                }
                new_arr
            });
            self.max_size = new_max_size;

            for entry in old_arr.into_iter() {
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
        self.arr[idx].data
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
