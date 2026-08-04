use std::sync::{Arc, RwLock};
use crate::hash::rehash;
const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;
pub struct OpenEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
}
pub struct OpenHashTable {
    cur_size: usize,
    max_size: usize,
    arr: Vec<OpenEntry>,
}

fn allocate_arr(size: usize) -> Vec<OpenEntry> {
    let mut v = Vec::with_capacity(size);
    for _ in 0..size {
        v.push(OpenEntry { key: None, data: None });
    }
    v
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size != 0);
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr: allocate_arr(initial_size),
        }))
    }
    fn query(&self, key: &str) -> usize {
        // Use the same hash function as C; we can compute hash on a &str by reusing logic
        let mut h: u32 = 0;
        for b in key.bytes() {
            h = h.wrapping_mul(crate::hash::POWER).wrapping_add(b as u32);
        }
        loop {
            let idx = (h as usize) % self.max_size;
            let entry = &self.arr[idx];
            match entry.key {
                None => return idx,
                Some(k) => {
                    if k == key {
                        return idx;
                    }
                }
            }
            h = rehash(h);
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_size += 1;
        if (self.cur_size as f32) / (self.max_size as f32) > LOAD_FACTOR {
            let new_max = self.max_size * 2;
            let old_arr = std::mem::replace(&mut self.arr, allocate_arr(new_max));
            self.max_size = new_max;
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
