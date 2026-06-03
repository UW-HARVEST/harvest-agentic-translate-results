use std::sync::{Arc, RwLock};
use crate::hash::{hash, POWER, ALTERNATIVE_POWER};
const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

pub struct CuckooEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
    marker: u32,
}

impl CuckooEntry {
    fn new() -> Self {
        CuckooEntry {
            key: None,
            data: None,
            marker: 0,
        }
    }
}

pub struct CuckooHashTable {
    cur_size: u32,
    cur_marker: u32,
    max_size: u32,
    first_arr: Vec<CuckooEntry>,
    second_arr: Vec<CuckooEntry>,
}

fn allocate_arr(size: u32) -> Vec<CuckooEntry> {
    let mut v = Vec::with_capacity(size as usize);
    for _ in 0..size {
        v.push(CuckooEntry::new());
    }
    v
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        assert!(
            initial_size != POWER as usize && initial_size != ALTERNATIVE_POWER as usize
        );
        let half = (initial_size / 2) as u32;
        let table = CuckooHashTable {
            cur_size: 0,
            cur_marker: 0,
            max_size: half,
            first_arr: allocate_arr(half),
            second_arr: allocate_arr(half),
        };
        Arc::new(RwLock::new(table))
    }

    fn initialize_table(&mut self, size: u32) {
        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = size;
        self.first_arr = allocate_arr(size);
        self.second_arr = allocate_arr(size);
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker = self.cur_marker.wrapping_add(1);

        if 1.0 + self.cur_size as f32 > self.max_size as f32 * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        let mut k: &'static str = key;
        let mut d: &'static str = data;

        let first_idx = self.first_index(k);
        if Self::try_to_store(&mut self.first_arr[first_idx], k, d) {
            return;
        }

        let second_idx = self.second_index(k);
        if Self::try_to_store(&mut self.second_arr[second_idx], k, d) {
            return;
        }

        let mut cur_first_idx = first_idx;
        let cur_marker = self.cur_marker;

        loop {
            // Swap with first_arr[cur_first_idx]; the new (k, d) goes in,
            // the displaced pair becomes the new (k, d) we still need to store.
            Self::swap_kd(&mut self.first_arr[cur_first_idx], &mut k, &mut d);
            self.first_arr[cur_first_idx].marker = cur_marker;

            let cur_second_idx = self.second_index(k);
            if self.second_arr[cur_second_idx].marker == cur_marker {
                self.refill();
                self.insert(k, d);
                break;
            }

            if Self::try_to_store(&mut self.second_arr[cur_second_idx], k, d) {
                break;
            }

            Self::swap_kd(&mut self.second_arr[cur_second_idx], &mut k, &mut d);
            self.second_arr[cur_second_idx].marker = cur_marker;

            let next_first_idx = self.first_index(k);
            if self.first_arr[next_first_idx].marker == cur_marker {
                self.refill();
                self.insert(k, d);
                break;
            }

            if Self::try_to_store(&mut self.first_arr[next_first_idx], k, d) {
                break;
            }

            cur_first_idx = next_first_idx;
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let first_idx = self.first_index(key);
        let entry = &self.first_arr[first_idx];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }

        let second_idx = self.second_index(key);
        let entry = &self.second_arr[second_idx];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }
        None
    }

    fn resize(&mut self) {
        let new_size = self.max_size * 2;
        self.recreate(new_size);
    }

    fn refill(&mut self) {
        let new_size = self.max_size + 1;
        self.recreate(new_size);
    }

    fn recreate(&mut self, new_size: u32) {
        assert!(new_size > 0);
        #[cfg(debug_assertions)]
        {
            assert!((new_size as f32) * LOAD_FACTOR + EPS > self.cur_size as f32);
        }

        let old_size = self.max_size;
        let old_first = std::mem::replace(&mut self.first_arr, Vec::new());
        let old_second = std::mem::replace(&mut self.second_arr, Vec::new());

        self.initialize_table(new_size);

        for i in 0..old_size as usize {
            if let (Some(k), Some(d)) = (old_first[i].key, old_first[i].data) {
                self.insert(k, d);
            }
        }
        for i in 0..old_size as usize {
            if let (Some(k), Some(d)) = (old_second[i].key, old_second[i].data) {
                self.insert(k, d);
            }
        }
    }

    fn first_index(&self, key: &str) -> usize {
        (hash(key) % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        // Mirrors the C implementation, which uses `hash` for both arrays.
        (hash(key) % self.max_size) as usize
    }

    #[allow(dead_code)]
    fn get_first_entry(&mut self, key: &str) -> &mut CuckooEntry {
        let idx = self.first_index(key);
        &mut self.first_arr[idx]
    }

    #[allow(dead_code)]
    fn get_second_entry(&mut self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        &mut self.second_arr[idx]
    }

    fn try_to_store(entry: &mut CuckooEntry, key: &'static str, data: &'static str) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            #[cfg(debug_assertions)]
            {
                if let Some(existing) = entry.key {
                    assert!(existing != key);
                }
            }
            false
        }
    }

    fn swap_kd(entry: &mut CuckooEntry, key: &mut &'static str, data: &mut &'static str) {
        let swap_key = entry.key.expect("swap_kd called on empty entry");
        let swap_data = entry.data.expect("swap_kd called on empty entry");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    #[allow(dead_code)]
    fn swap_key_data_entry(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        entry: &mut CuckooEntry,
    ) {
        Self::swap_kd(entry, key, data);
    }

    #[allow(dead_code)]
    fn free_cukoo_hash_table(self) {
        // Owned vectors are released when `self` is dropped.
    }
}
