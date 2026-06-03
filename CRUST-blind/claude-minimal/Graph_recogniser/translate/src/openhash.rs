use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::{hash, rehash, POWER, REHASHER};
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
impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        crate::check!(initial_size != 0);
        crate::check!(initial_size != POWER as usize && initial_size != REHASHER as usize);
        let mut arr: Vec<OpenEntry> = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            arr.push(OpenEntry { key: None, data: None });
        }
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }
    fn query(&self, key: &str) -> usize {
        crate::check!(!key.is_empty() || key == "");
        let mut h = hash(key_to_static(key));
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
            crate::check!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);
        }
        self.cur_size += 1;
        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let new_max = self.max_size * 2;
            let old_max = self.max_size;
            let mut new_arr: Vec<OpenEntry> = Vec::with_capacity(new_max);
            for _ in 0..new_max {
                new_arr.push(OpenEntry { key: None, data: None });
            }
            // Take old array
            let old_arr = std::mem::replace(&mut self.arr, new_arr);
            self.max_size = new_max;
            for i in 0..old_max {
                if let Some(cur_key) = old_arr[i].key {
                    let idx = self.query(cur_key);
                    self.arr[idx].key = old_arr[i].key;
                    self.arr[idx].data = old_arr[i].data;
                }
            }
        }
        let idx = self.query(key);
        crate::check!(self.arr[idx].key.is_none());
        self.arr[idx].key = Some(key);
        self.arr[idx].data = Some(data);
    }
    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        crate::check!(self.arr[idx].key.is_some());
        self.arr[idx].data
    }
    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}

// Helper to coerce a &str to a 'static str when needed for hash function.
// Since hash only reads the bytes, we can safely use unsafe to extend lifetime
// for hashing purposes only.
fn key_to_static(key: &str) -> &'static str {
    unsafe { std::mem::transmute::<&str, &'static str>(key) }
}
