use std::sync::{Arc, RwLock};

use crate::hash::{rehash, EMPTY_KEY, POWER, REHASHER};

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

fn hash_str(key: &str) -> u32 {
    let mut res = 0u32;
    for byte in key.bytes() {
        res = res.wrapping_mul(POWER).wrapping_add(byte as u32);
    }
    res
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size != 0);
        assert!(initial_size != POWER as usize && initial_size != REHASHER as usize);

        let mut arr = Vec::with_capacity(initial_size);
        for _ in 0..initial_size {
            arr.push(OpenEntry {
                key: EMPTY_KEY,
                data: None,
            });
        }

        Arc::new(RwLock::new(Self {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }

    fn query(&self, key: &str) -> usize {
        let mut h = hash_str(key);
        loop {
            let index = h as usize % self.max_size;
            let entry = &self.arr[index];
            match entry.key {
                None => return index,
                Some(cur_key) if cur_key == key => {
                    return index;
                }
                Some(_) => h = rehash(h),
            }
        }
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        #[cfg(debug_assertions)]
        assert!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            self.max_size *= 2;
            let old_arr = std::mem::take(&mut self.arr);
            self.arr = Vec::with_capacity(self.max_size);
            for _ in 0..self.max_size {
                self.arr.push(OpenEntry {
                    key: EMPTY_KEY,
                    data: None,
                });
            }

            for old_entry in old_arr {
                if let Some(cur_key) = old_entry.key {
                    let index = self.query(cur_key);
                    self.arr[index] = old_entry;
                }
            }
        }

        let index = self.query(key);
        assert!(self.arr[index].key.is_none());
        self.arr[index].key = Some(key);
        self.arr[index].data = Some(data);
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let index = self.query(key);
        self.arr[index].data
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
