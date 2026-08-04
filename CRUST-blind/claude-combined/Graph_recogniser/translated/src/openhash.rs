use std::sync::{Arc, RwLock};
use crate::check;
use crate::hash::{hash_any, rehash, POWER, REHASHER};

const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

#[derive(Clone)]
pub struct OpenEntry {
    pub key: Option<&'static str>,
    pub data: Option<&'static str>,
}

pub struct OpenHashTable {
    pub cur_size: usize,
    pub max_size: usize,
    pub arr: Vec<OpenEntry>,
}

impl OpenHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        check::check(initial_size != 0);
        check::check(initial_size as u32 != POWER && initial_size as u32 != REHASHER);
        let arr = (0..initial_size)
            .map(|_| OpenEntry { key: None, data: None })
            .collect();
        Arc::new(RwLock::new(OpenHashTable {
            cur_size: 0,
            max_size: initial_size,
            arr,
        }))
    }

    /// Find the index in `arr` where `key` is stored or where it should be
    /// inserted (the first empty slot or matching cell along the probe chain).
    fn query(&self, key: &str) -> usize {
        check::check(!key.is_empty() || key.is_empty());
        let mut h: u32 = hash_any(key);
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
        // CHECK that we are below load_factor + eps
        #[cfg(debug_assertions)]
        check::check((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let new_max = self.max_size * 2;
            let old_arr = std::mem::replace(
                &mut self.arr,
                (0..new_max)
                    .map(|_| OpenEntry { key: None, data: None })
                    .collect(),
            );
            self.max_size = new_max;

            for entry in old_arr.iter() {
                if let Some(cur_key) = entry.key {
                    let idx = self.query(cur_key);
                    self.arr[idx] = entry.clone();
                }
            }
        }

        let idx = self.query(key);
        check::check(self.arr[idx].key.is_none());
        self.arr[idx].key = Some(key);
        self.arr[idx].data = Some(data);
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        let cell = &self.arr[idx];
        check::check(cell.key.is_some());
        cell.data
    }

    pub fn free_open_hash_table(&mut self) {
        self.arr.clear();
        self.cur_size = 0;
        self.max_size = 0;
    }
}
