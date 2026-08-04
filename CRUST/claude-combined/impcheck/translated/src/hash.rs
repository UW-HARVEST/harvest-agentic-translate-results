use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

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

struct HtState {
    map: HashMap<u64, *mut Box<dyn Any>>,
    last_found_idx: u64,
}

thread_local! {
    static STORAGE: RefCell<HashMap<usize, HtState>>
        = RefCell::new(HashMap::new());
}

fn empty_slice<'a, T>() -> &'a mut [T] {
    unsafe {
        std::slice::from_raw_parts_mut(std::ptr::NonNull::<T>::dangling().as_ptr(), 0)
    }
}

impl<T: 'static> HashTable<'_, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity = 1u64 << log_init_capacity;
        let mut entries: Vec<HashTableEntry<'static, T>> = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            entries.push(HashTableEntry {
                key: 0,
                val: empty_slice::<'static, T>(),
            });
        }
        let leaked: &'static mut [HashTableEntry<'static, T>] =
            Box::leak(entries.into_boxed_slice());
        let id = leaked.as_ptr() as usize;
        STORAGE.with(|s| {
            s.borrow_mut().insert(
                id,
                HtState {
                    map: HashMap::new(),
                    last_found_idx: 0,
                },
            );
        });
        let data_anon: &mut [HashTableEntry<T>] = unsafe { std::mem::transmute(leaked) };
        Self {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data: data_anon,
            last_found_idx: 0,
        }
    }

    fn id(&self) -> usize {
        self.data.as_ptr() as usize
    }

    pub fn realloc_table(&mut self) -> bool {
        let old_id = self.id();
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        let old_capacity = self.capacity;

        let mut old_keys: Vec<u64> = Vec::with_capacity(self.size as usize);
        for i in 0..old_capacity as usize {
            if self.data[i].key != 0 {
                old_keys.push(self.data[i].key);
            }
        }

        let old_data_ptr: *mut [HashTableEntry<T>] = self.data as *mut [HashTableEntry<T>];

        let old_state: HtState = STORAGE.with(|s| {
            s.borrow_mut().remove(&old_id).unwrap_or(HtState {
                map: HashMap::new(),
                last_found_idx: 0,
            })
        });

        let mut entries: Vec<HashTableEntry<'static, T>> = Vec::with_capacity(new_capacity as usize);
        for _ in 0..new_capacity {
            entries.push(HashTableEntry {
                key: 0,
                val: empty_slice::<'static, T>(),
            });
        }
        let leaked: &'static mut [HashTableEntry<'static, T>] =
            Box::leak(entries.into_boxed_slice());
        let new_id = leaked.as_ptr() as usize;

        unsafe {
            let _ = Box::from_raw(old_data_ptr);
        }

        let data_anon: &mut [HashTableEntry<T>] = unsafe { std::mem::transmute(leaked) };
        self.data = data_anon;

        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;

        STORAGE.with(|s| {
            s.borrow_mut().insert(
                new_id,
                HtState {
                    map: HashMap::new(),
                    last_found_idx: 0,
                },
            );
        });

        for key in old_keys {
            if let Some(&val_ptr) = old_state.map.get(&key) {
                let mut idx = 0u64;
                if self.find_entry(key, &mut idx) {
                    return false;
                }
                self.data[idx as usize].key = key;
                STORAGE.with(|s| {
                    s.borrow_mut()
                        .get_mut(&new_id)
                        .unwrap()
                        .map
                        .insert(key, val_ptr);
                });
                self.size += 1;
            }
        }

        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn std::any::Any>> {
        let mut idx = 0u64;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        let id = self.id();
        let raw_ptr: *mut Box<dyn std::any::Any> = STORAGE.with(|s| {
            let mut sr = s.borrow_mut();
            if let Some(state) = sr.get_mut(&id) {
                state.last_found_idx = idx;
                state.map.get(&key).copied().unwrap_or(std::ptr::null_mut())
            } else {
                std::ptr::null_mut()
            }
        });
        if raw_ptr.is_null() {
            None
        } else {
            Some(unsafe { &*raw_ptr })
        }
    }

    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        let mut idx = 0u64;
        if !self.find_entry(key, &mut idx) {
            return false;
        }
        if !self.handle_gap(idx) {
            return false;
        }
        let id = self.id();
        STORAGE.with(|s| {
            if let Some(state) = s.borrow_mut().get_mut(&id) {
                if let Some(ptr) = state.map.remove(&key) {
                    unsafe {
                        let _ = Box::from_raw(ptr);
                    }
                }
            }
        });
        self.size -= 1;
        true
    }

    pub fn compute_idx(&self, key: u64) -> u64 {
        Self::compute_hash(key) & (self.capacity - 1)
    }

    pub fn hash_table_insert(&mut self, key: u64, val: Box<dyn std::any::Any>) -> bool {
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
        let mut idx = 0u64;
        if self.find_entry(key, &mut idx) {
            return false;
        }
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        self.data[idx as usize].key = key;
        let boxed = Box::new(val);
        let raw_ptr: *mut Box<dyn std::any::Any> = Box::into_raw(boxed);
        let id = self.id();
        STORAGE.with(|s| {
            s.borrow_mut()
                .get_mut(&id)
                .unwrap()
                .map
                .insert(key, raw_ptr);
        });
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
        let id = self.id();
        STORAGE.with(|s| {
            if let Some(state) = s.borrow_mut().remove(&id) {
                for (_, ptr) in state.map {
                    unsafe {
                        let _ = Box::from_raw(ptr);
                    }
                }
            }
        });
        if !self.data.is_empty() {
            unsafe {
                let _ = Box::from_raw(self.data as *mut [HashTableEntry<T>]);
            }
        }
        let empty: &'static mut [HashTableEntry<'static, T>] = unsafe {
            std::slice::from_raw_parts_mut(
                std::ptr::NonNull::<HashTableEntry<'static, T>>::dangling().as_ptr(),
                0,
            )
        };
        self.data = unsafe { std::mem::transmute(empty) };
        self.size = 0;
        self.capacity = 0;
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        let mask = self.capacity - 1;
        loop {
            j = (j + 1) & mask;
            if Self::cell_empty(&self.data[j as usize]) {
                let entry = &mut self.data[i as usize];
                entry.key = 0;
                entry.val = empty_slice();
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            let movable = (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j);
            if movable {
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
        let id = self.id();
        let idx = STORAGE.with(|s| {
            s.borrow().get(&id).map(|st| st.last_found_idx).unwrap_or(0)
        });
        let key = self.data[idx as usize].key;
        if !self.handle_gap(idx) {
            return false;
        }
        STORAGE.with(|s| {
            if let Some(state) = s.borrow_mut().get_mut(&id) {
                if let Some(ptr) = state.map.remove(&key) {
                    unsafe {
                        let _ = Box::from_raw(ptr);
                    }
                }
            }
        });
        self.size -= 1;
        true
    }
}
