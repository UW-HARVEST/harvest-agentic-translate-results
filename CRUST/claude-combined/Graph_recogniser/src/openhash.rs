use std::sync::{Arc, RwLock};
use crate::hash::{rehash, POWER, REHASHER};

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
    (0..size).map(|_| OpenEntry::empty()).collect()
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size != 0);
        assert!(initial_size as u32 != POWER && initial_size as u32 != REHASHER);

        let table = OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr: allocate_arr(initial_size),
        };
        Arc::new(RwLock::new(table))
    }

    /// Returns the index of an entry that either matches `key` or is empty.
    fn query(&self, key: &str) -> usize {
        // hash() takes a &'static str only because of typedef, but the algorithm only needs bytes.
        let mut h: u32 = 0;
        for b in key.bytes() {
            h = h.wrapping_mul(crate::hash::POWER).wrapping_add(b as u32);
        }
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
            let new_max = self.max_size * 2;
            let old_arr = std::mem::replace(&mut self.arr, allocate_arr(new_max));
            let old_size = self.max_size;
            self.max_size = new_max;

            for i in 0..old_size {
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
