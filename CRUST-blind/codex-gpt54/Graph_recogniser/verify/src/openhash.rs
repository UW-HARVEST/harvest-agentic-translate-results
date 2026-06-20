use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::log::{LogType, Logger};
use crate::check;
use crate::hash::hash;
const LOAD_FACTOR: f32 = 0.6;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

fn open_hash_value(key: &str) -> u32 {
    key.bytes().fold(0, |acc, byte| {
        acc.wrapping_mul(crate::hash::POWER)
            .wrapping_add(byte as u32)
    })
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
       debug_assert!(initial_size != 0);
       debug_assert!(initial_size as u32 != crate::hash::POWER && initial_size as u32 != crate::hash::REHASHER);

       Arc::new(RwLock::new(Self {
           cur_size: 0,
           max_size: initial_size,
           arr: std::iter::repeat_with(|| OpenEntry {
               key: None,
               data: None,
           })
           .take(initial_size)
           .collect(),
       }))
    }
    fn query(&self, key: &str) -> usize {
        let mut h = open_hash_value(key);

        loop {
            let idx = (h as usize) % self.max_size;
            let entry = &self.arr[idx];

            if entry.key.is_none() || entry.key == Some(key) {
                return idx;
            }

            h = crate::hash::rehash(h);
        }
    }
    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        #[cfg(debug_assertions)]
        debug_assert!((self.cur_size as f32 / self.max_size as f32) < LOAD_FACTOR + EPS);

        self.cur_size += 1;

        if (self.cur_size as f32 / self.max_size as f32) > LOAD_FACTOR {
            let old_arr = std::mem::replace(
                &mut self.arr,
                std::iter::repeat_with(|| OpenEntry {
                    key: None,
                    data: None,
                })
                .take(self.max_size * 2)
                .collect(),
            );
            self.max_size *= 2;

            for entry in old_arr {
                if let Some(cur_key) = entry.key {
                    let idx = self.query(cur_key);
                    self.arr[idx] = OpenEntry {
                        key: Some(cur_key),
                        data: entry.data,
                    };
                }
            }
        }

        let idx = self.query(key);
        #[cfg(debug_assertions)]
        debug_assert!(self.arr[idx].key.is_none());

        self.arr[idx].key = Some(key);
        self.arr[idx].data = Some(data);
    }
    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx = self.query(key);
        #[cfg(debug_assertions)]
        debug_assert!(self.arr[idx].key.is_some());

        self.arr[idx].data
    }
    pub fn free_open_hash_table(&mut self) {
        self.cur_size = 0;
        self.max_size = 0;
        self.arr.clear();
    }
}
