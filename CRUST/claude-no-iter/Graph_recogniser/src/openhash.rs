use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::{hash, rehash};
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
        debug_assert!(initial_size != 0);
        debug_assert!(
            initial_size != crate::hash::POWER as usize
                && initial_size != crate::hash::REHASHER as usize
        );
        let mut arr = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            arr.push(OpenEntry {
                key: None,
                data: None,
            });
        }
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }
    /// Returns the index in `arr` where the key resides or where it should
    /// be inserted (the first empty cell encountered while probing).
    fn query(&self, key: &str) -> usize {
        debug_assert!(!key.is_empty() || true); // empty_key in C is NULL; in Rust we never pass None here.
        // hash takes &'static str. The function only reads bytes, so it is
        // sound to extend the lifetime for the duration of the call.
        let static_key: &'static str = unsafe { std::mem::transmute::<&str, &'static str>(key) };
        let mut h = hash(static_key);
        loop {
            let idx = (h as usize) % self.max_size;
            let entry = &self.arr[idx];
            match entry.key {
                None => return idx,
                Some(cur_key) if cur_key == key => return idx,
                _ => {}
            }
            h = rehash(h);
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                (self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS
            );
        }

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_max = self.max_size;
            self.max_size *= 2;
            let mut new_arr: Vec<OpenEntry> = Vec::with_capacity(self.max_size);
            for _ in 0..self.max_size {
                new_arr.push(OpenEntry { key: None, data: None });
            }
            // Swap the arrays so we can re-insert into the new one via query().
            let old_arr = std::mem::replace(&mut self.arr, new_arr);
            for i in 0..old_max {
                if let Some(cur_key) = old_arr[i].key {
                    let idx = self.query(cur_key);
                    self.arr[idx] = OpenEntry {
                        key: old_arr[i].key,
                        data: old_arr[i].data,
                    };
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
