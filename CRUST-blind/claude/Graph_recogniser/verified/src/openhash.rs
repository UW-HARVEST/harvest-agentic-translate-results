use std::sync::{Arc, RwLock};
use crate::hash::{POWER, REHASHER};
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

fn compute_hash(key: &str) -> u32 {
    let mut res: u32 = 0;
    for b in key.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(b as u32);
    }
    res
}

fn rehash_value(h: u32) -> u32 {
    h.wrapping_add(REHASHER)
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        debug_assert!(initial_size != 0);
        debug_assert!(initial_size as u32 != POWER && initial_size as u32 != REHASHER);
        let arr = Self::allocate_arr(initial_size);
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }
    fn allocate_arr(size: usize) -> Vec<OpenEntry> {
        let mut v = Vec::with_capacity(size);
        for _ in 0..size {
            v.push(OpenEntry { key: None, data: None });
        }
        v
    }
    fn query(&self, key: &str) -> usize {
        let mut h = compute_hash(key);
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
            h = rehash_value(h);
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        #[cfg(debug_assertions)]
        debug_assert!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_max = self.max_size;
            self.max_size *= 2;
            let new_arr = Self::allocate_arr(self.max_size);
            let old_arr = std::mem::replace(&mut self.arr, new_arr);

            for h in 0..old_max {
                if let Some(cur_key) = old_arr[h].key {
                    let cur_data = old_arr[h].data;
                    let idx = self.query(cur_key);
                    self.arr[idx].key = Some(cur_key);
                    self.arr[idx].data = cur_data;
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
        debug_assert!(self.arr[idx].key.is_some());
        self.arr[idx].data
    }
    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
