use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash;
const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

const POWER: u32 = 131;
const REHASHER: u32 = 718841;

fn compute_hash_str(s: &str) -> u32 {
    let mut res: u32 = 0;
    for c in s.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(c as u32);
    }
    res
}

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
        assert!(initial_size != 0);
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
        let mut h = compute_hash_str(key);
        loop {
            let idx = (h % (self.max_size as u32)) as usize;
            match self.arr[idx].key {
                None => return idx,
                Some(k) if k == key => return idx,
                _ => {
                    h = h.wrapping_add(REHASHER);
                }
            }
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_arr = std::mem::take(&mut self.arr);
            self.max_size *= 2;
            self.arr = (0..self.max_size)
                .map(|_| OpenEntry { key: None, data: None })
                .collect();
            for entry in old_arr.into_iter() {
                if let Some(k) = entry.key {
                    let idx = self.query(k);
                    self.arr[idx] = entry;
                }
            }
        }

        let idx = self.query(key);
        self.arr[idx] = OpenEntry { key: Some(key), data: Some(data) };
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
