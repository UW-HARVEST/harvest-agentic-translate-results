// Hash table mapping u64 keys to values.
// Note: The signatures in this file are inconsistent (the struct stores
// `&'a mut [T]` values, but `hash_table_insert` accepts `Box<dyn Any>`).
// We implement the underlying open-addressing logic as faithfully as we can.

use std::any::Any;

pub struct HashTableEntry<'a, T> {
    pub key: u64,
    pub val: &'a mut [T],
}

pub struct HashTable<'a, T> {
    pub size: u64,
    pub max_size: u64,
    pub growth_factor: f32,
    pub capacity: u64,
    pub data: &'a mut [HashTableEntry<'a, T>],
    pub last_found_idx: u64,
}

fn make_empty_entry<'a, T>() -> HashTableEntry<'a, T> {
    let v: Vec<T> = Vec::new();
    let leaked: *mut [T] = Box::into_raw(v.into_boxed_slice());
    // SAFETY: length is 0, so no aliasing concerns. The boxed slice has
    // 'static-lifetime backing.
    let val: &'a mut [T] = unsafe { &mut *(leaked as *mut [T]) };
    HashTableEntry { key: 0, val }
}

fn make_data_slice<'a, T>(capacity: u64) -> &'a mut [HashTableEntry<'a, T>] {
    let mut entries: Vec<HashTableEntry<'a, T>> = Vec::with_capacity(capacity as usize);
    for _ in 0..capacity {
        entries.push(make_empty_entry::<'a, T>());
    }
    let boxed = entries.into_boxed_slice();
    let raw: *mut [HashTableEntry<'a, T>] = Box::into_raw(boxed);
    // SAFETY: We hand out a unique mutable reference to the leaked
    // allocation; we will reclaim it via Box::from_raw later.
    unsafe { &mut *raw }
}

unsafe fn drop_data_slice<T>(data: &mut [HashTableEntry<'_, T>]) {
    if data.is_empty() {
        return;
    }
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len));
}

impl<'a, T> HashTable<'a, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let data = make_data_slice::<'a, T>(capacity);
        HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
        }
    }

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        if new_capacity <= self.capacity {
            return false;
        }
        let new_data = make_data_slice::<'a, T>(new_capacity);
        let old_capacity = self.capacity;
        let old_data = std::mem::replace(&mut self.data, new_data);

        self.capacity = new_capacity;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;

        // Re-insert keys from old data
        let old_ptr = old_data.as_ptr();
        for i in 0..(old_capacity as usize) {
            // SAFETY: we have exclusive access to the old slice via old_data
            let key = unsafe { (*old_ptr.add(i)).key };
            if key != 0 {
                let mut idx: u64 = 0;
                if !self.find_entry(key, &mut idx) {
                    self.data[idx as usize].key = key;
                    self.size += 1;
                }
            }
        }

        // Reclaim old slice
        unsafe {
            drop_data_slice::<T>(old_data);
        }
        true
    }

    pub fn hash_table_find(&self, _key: u64) -> Option<&Box<dyn std::any::Any>> {
        // Signature mismatch: we cannot return references to data that was
        // never typed as Box<dyn Any>. Always return None.
        None
    }

    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return false;
        }
        if !self.handle_gap(idx) {
            return false;
        }
        self.size -= 1;
        true
    }

    pub fn compute_idx(&self, key: u64) -> u64 {
        Self::compute_hash(key) & (self.capacity - 1)
    }

    pub fn hash_table_insert(&mut self, key: u64, _val: Box<dyn Any>) -> bool {
        if key == 0 {
            return false;
        }
        if self.size == self.max_size {
            if !self.realloc_table() {
                return false;
            }
            if self.size >= self.max_size {
                return false;
            }
        }
        let mut idx: u64 = 0;
        if self.find_entry(key, &mut idx) {
            return false;
        }
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        self.data[idx as usize].key = key;
        self.size += 1;
        true
    }

    pub fn compute_hash(key: u64) -> u64 {
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }

    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        entry.key == 0
    }

    pub fn hash_table_free(&mut self) {
        if !self.data.is_empty() {
            // Take ownership of the slice with mem::replace using a
            // temporarily empty (but properly typed) slice.
            let empty = make_data_slice::<'a, T>(0);
            let old = std::mem::replace(&mut self.data, empty);
            unsafe {
                drop_data_slice::<T>(old);
            }
        }
        self.size = 0;
        self.capacity = 0;
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if Self::cell_empty(&self.data[j as usize]) {
                self.data[i as usize].key = 0;
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
                let key_j = self.data[j as usize].key;
                self.data[i as usize].key = key_j;
                self.data[j as usize].key = 0;
                i = j;
            }
        }
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        let mut i = self.compute_idx(key);
        let orig_idx = i;
        while i < self.capacity {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                *idx = i;
                return false;
            }
            if entry.key == key {
                *idx = i;
                return true;
            }
            i += 1;
        }
        i = 0;
        while i < orig_idx {
            let entry = &self.data[i as usize];
            if Self::cell_empty(entry) {
                *idx = i;
                return false;
            }
            if entry.key == key {
                *idx = i;
                return true;
            }
            i += 1;
        }
        false
    }

    pub fn hash_table_delete_last_found(&mut self) -> bool {
        if !self.handle_gap(self.last_found_idx) {
            return false;
        }
        self.size -= 1;
        true
    }
}
