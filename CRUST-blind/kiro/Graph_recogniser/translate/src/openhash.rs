use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash_str;
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
        let arr = (0..initial_size).map(|_| OpenEntry { key: None, data: None }).collect();
        Arc::new(RwLock::new(Self {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }
    fn query(&self, key: &str) -> usize {
        let mut h = hash_str(key);
        loop {
            let idx = (h as usize) % self.max_size;
            match self.arr[idx].key {
                None => return idx,
                Some(k) if k == key => return idx,
                _ => { h = crate::hash::rehash(h); }
            }
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_max = self.max_size;
            self.max_size *= 2;
            let old_arr: Vec<OpenEntry> = std::mem::replace(
                &mut self.arr,
                (0..self.max_size).map(|_| OpenEntry { key: None, data: None }).collect(),
            );
            for entry in old_arr.into_iter().take(old_max) {
                if let Some(k) = entry.key {
                    let idx = self.query(k);
                    self.arr[idx].key = Some(k);
                    self.arr[idx].data = entry.data;
                }
            }
        }

        let idx = self.query(key);
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
