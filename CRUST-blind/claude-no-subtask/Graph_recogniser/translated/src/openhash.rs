use std::sync::{Arc, RwLock};
use crate::hash::{hash_str, rehash_internal, POWER, REHASHER};

const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

pub struct OpenEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
}

impl OpenEntry {
    fn empty() -> Self {
        OpenEntry {
            key: None,
            data: None,
        }
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
        debug_assert!(initial_size != POWER as usize && initial_size != REHASHER as usize);

        let mut arr: Vec<OpenEntry> = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            arr.push(OpenEntry::empty());
        }

        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }

    fn query(&self, key: &str) -> usize {
        debug_assert!(!key.is_empty() || true); // empty key allowed only as marker; user keys non-None
        let mut h = hash_str(key);
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
            h = rehash_internal(h);
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
            let new_max = self.max_size * 2;
            let mut new_arr: Vec<OpenEntry> = Vec::with_capacity(new_max);
            for _ in 0..new_max {
                new_arr.push(OpenEntry::empty());
            }

            // Move old entries to new array
            let old_arr = std::mem::replace(&mut self.arr, new_arr);
            let old_size = self.max_size;
            self.max_size = new_max;

            for entry in old_arr.into_iter().take(old_size) {
                if let (Some(k), Some(d)) = (entry.key, entry.data) {
                    let idx = self.query(k);
                    self.arr[idx] = OpenEntry {
                        key: Some(k),
                        data: Some(d),
                    };
                }
            }
        }

        let idx = self.query(key);
        debug_assert!(self.arr[idx].key.is_none());
        self.arr[idx] = OpenEntry {
            key: Some(key),
            data: Some(data),
        };
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        match self.arr[idx].key {
            Some(_) => self.arr[idx].data,
            None => None,
        }
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
