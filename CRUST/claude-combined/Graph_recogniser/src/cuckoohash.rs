use std::sync::{Arc, RwLock};
use crate::hash::{POWER, ALTERNATIVE_POWER};

const LOAD_FACTOR: f32 = 1.0;
#[cfg(debug_assertions)]
const EPS: f32 = 1e-3;

pub struct CuckooEntry {
    key: Option<&'static str>,
    data: Option<&'static str>,
    marker: u32,
}

impl CuckooEntry {
    fn empty() -> Self {
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
    (0..size).map(|_| CuckooEntry::empty()).collect()
}

fn hash_str(key: &str) -> u32 {
    let mut h: u32 = 0;
    for b in key.bytes() {
        h = h.wrapping_mul(POWER).wrapping_add(b as u32);
    }
    h
}

fn alt_hash_str(key: &str) -> u32 {
    let mut h: u32 = 0;
    for b in key.bytes() {
        h = h.wrapping_mul(ALTERNATIVE_POWER).wrapping_add(b as u32);
    }
    h
}

impl CuckooHashTable {
    pub fn new(initial_size: usize) -> Arc<RwLock<Self>> {
        assert!(initial_size >= 2);
        assert!(initial_size as u32 != POWER && initial_size as u32 != ALTERNATIVE_POWER);

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

    fn first_index(&self, key: &str) -> usize {
        (hash_str(key) % self.max_size) as usize
    }

    fn second_index(&self, key: &str) -> usize {
        // The C code uses hash() (the same hash) for both arrays in get_first_entry/get_second_entry.
        // Re-checking: yes, both helpers use hash(key), not alternative_hash. They only differ in
        // which array they index into.
        (hash_str(key) % self.max_size) as usize
    }

    pub fn insert(&mut self, key: &'static str, data: &'static str) {
        self.cur_marker += 1;

        if 1.0 + (self.cur_size as f32) > (self.max_size as f32) * LOAD_FACTOR {
            self.resize();
        }

        self.cur_size += 1;

        // Try to store in first array
        let idx1 = self.first_index(key);
        if self.first_arr[idx1].key.is_none() {
            self.first_arr[idx1].key = Some(key);
            self.first_arr[idx1].data = Some(data);
            return;
        }

        // Try to store in second array
        let idx2 = self.second_index(key);
        if self.second_arr[idx2].key.is_none() {
            self.second_arr[idx2].key = Some(key);
            self.second_arr[idx2].data = Some(data);
            return;
        }

        // Cuckoo eviction
        let mut cur_key = key;
        let mut cur_data = data;
        let mut first_idx = idx1;

        loop {
            // Swap with first_arr[first_idx]
            let swap_key = self.first_arr[first_idx].key;
            let swap_data = self.first_arr[first_idx].data;
            self.first_arr[first_idx].key = Some(cur_key);
            self.first_arr[first_idx].data = Some(cur_data);
            self.first_arr[first_idx].marker = self.cur_marker;
            cur_key = swap_key.expect("evicted key must be Some");
            cur_data = swap_data.expect("evicted data must be Some");

            let second_idx = self.second_index(cur_key);
            if self.second_arr[second_idx].marker == self.cur_marker {
                // cycle detected
                self.refill();
                self.insert(cur_key, cur_data);
                return;
            }

            if self.second_arr[second_idx].key.is_none() {
                self.second_arr[second_idx].key = Some(cur_key);
                self.second_arr[second_idx].data = Some(cur_data);
                return;
            }

            // Swap with second_arr[second_idx]
            let swap_key = self.second_arr[second_idx].key;
            let swap_data = self.second_arr[second_idx].data;
            self.second_arr[second_idx].key = Some(cur_key);
            self.second_arr[second_idx].data = Some(cur_data);
            self.second_arr[second_idx].marker = self.cur_marker;
            cur_key = swap_key.expect("evicted key must be Some");
            cur_data = swap_data.expect("evicted data must be Some");

            let next_first_idx = self.first_index(cur_key);
            if self.first_arr[next_first_idx].marker == self.cur_marker {
                self.refill();
                self.insert(cur_key, cur_data);
                return;
            }

            if self.first_arr[next_first_idx].key.is_none() {
                self.first_arr[next_first_idx].key = Some(cur_key);
                self.first_arr[next_first_idx].data = Some(cur_data);
                return;
            }

            first_idx = next_first_idx;
        }
    }

    pub fn find(&self, key: &str) -> Option<&'static str> {
        let idx1 = self.first_index(key);
        let entry = &self.first_arr[idx1];
        if let Some(cur_key) = entry.key {
            if cur_key == key {
                return entry.data;
            }
        }
        let idx2 = self.second_index(key);
        let entry = &self.second_arr[idx2];
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

        let old_first = std::mem::replace(&mut self.first_arr, allocate_arr(new_size));
        let old_second = std::mem::replace(&mut self.second_arr, allocate_arr(new_size));
        let old_size = self.max_size;

        self.cur_size = 0;
        self.cur_marker = 0;
        self.max_size = new_size;

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

    fn get_first_entry(&self, key: &str) -> &mut CuckooEntry {
        // The signature mirrors the C API which returned a non-const pointer from a
        // const-ish context. The public API (insert/find) does not use these helpers
        // and instead uses safe index-based access. The implementation here uses raw
        // pointer arithmetic to satisfy the signature without modifying types.
        let idx = self.first_index(key);
        let p = self.first_arr.as_ptr();
        unsafe { &mut *(p.add(idx) as *mut CuckooEntry) }
    }

    fn get_second_entry(&self, key: &str) -> &mut CuckooEntry {
        let idx = self.second_index(key);
        let p = self.second_arr.as_ptr();
        unsafe { &mut *(p.add(idx) as *mut CuckooEntry) }
    }

    fn try_to_store(&mut self, key: &'static str, data: &'static str, entry: &mut CuckooEntry) -> bool {
        if entry.key.is_none() {
            entry.key = Some(key);
            entry.data = Some(data);
            true
        } else {
            false
        }
    }

    fn swap_key_data_entry(
        &mut self,
        key: &mut &'static str,
        data: &mut &'static str,
        entry: &mut CuckooEntry,
    ) {
        let swap_key = entry.key.expect("swapped entry must be non-empty");
        let swap_data = entry.data.expect("swapped entry must be non-empty");
        entry.key = Some(*key);
        entry.data = Some(*data);
        *key = swap_key;
        *data = swap_data;
    }

    fn free_cukoo_hash_table(self) {
        // Vec drop frees memory automatically; nothing to do.
    }
}
