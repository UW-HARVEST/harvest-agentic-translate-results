use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REGISTRY: RefCell<HashMap<usize, HashStorage>> = RefCell::new(HashMap::new());
}

struct HashStorage {
    keys: Vec<u64>,
    values: Vec<Option<&'static Box<dyn Any>>>,
    last_found_idx: u64,
}

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

impl<T: 'static> HashTable<'_, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let token = Box::leak(vec![HashTableEntry {
            key: 0,
            val: empty_slice(),
        }]
        .into_boxed_slice());
        let capacity = 1u64 << log_init_capacity;
        let id = token.as_ptr() as usize;
        REGISTRY.with(|registry| {
            registry.borrow_mut().insert(
                id,
                HashStorage {
                    keys: vec![0; capacity as usize],
                    values: vec![None; capacity as usize],
                    last_found_idx: 0,
                },
            );
        });
        Self {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data: token,
            last_found_idx: 0,
        }
    }

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        let old_entries = REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let storage = registry.get_mut(&self.id()).unwrap();
            let mut entries = Vec::new();
            for idx in 0..self.capacity as usize {
                if storage.keys[idx] != 0 {
                    entries.push((storage.keys[idx], storage.values[idx]));
                }
            }
            storage.keys = vec![0; new_capacity as usize];
            storage.values = vec![None; new_capacity as usize];
            storage.last_found_idx = 0;
            entries
        });

        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        for (key, value) in old_entries {
            if let Some(value) = value {
                if !self.insert_existing(key, value) {
                    return false;
                }
            }
        }
        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn Any>> {
        let mut idx = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let storage = registry.get_mut(&self.id()).unwrap();
            storage.last_found_idx = idx;
            storage.values[idx as usize]
        })
    }

    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        let mut idx = 0;
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

    pub fn hash_table_insert(&mut self, key: u64, val: Box<dyn Any>) -> bool {
        if key == 0 {
            return false;
        }
        if self.size == self.max_size {
            if !self.realloc_table() || self.size >= self.max_size {
                return false;
            }
        }

        let leaked: &'static Box<dyn Any> = Box::leak(Box::new(val));
        self.insert_existing(key, leaked)
    }

    pub fn compute_hash(key: u64) -> u64 {
        (0xcbf29ce484222325_u64 ^ key).wrapping_mul(0x0000_0100_0000_01B3_u64)
    }

    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        entry.key == 0
    }

    pub fn hash_table_free(&mut self) {
        REGISTRY.with(|registry| {
            registry.borrow_mut().remove(&self.id());
        });
        self.size = 0;
        self.max_size = 0;
        self.capacity = 0;
        self.last_found_idx = 0;
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let capacity = self.capacity;
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let storage = registry.get_mut(&self.id()).unwrap();
            let mut i = idx_of_gap;
            let mut j = i;

            loop {
                j = (j + 1) & (capacity - 1);
                if storage.keys[j as usize] == 0 {
                    storage.keys[i as usize] = 0;
                    storage.values[i as usize] = None;
                    return true;
                }

                let k = Self::compute_hash(storage.keys[j as usize]) & (capacity - 1);
                if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
                    storage.keys[i as usize] = storage.keys[j as usize];
                    storage.values[i as usize] = storage.values[j as usize];
                    storage.keys[j as usize] = 0;
                    storage.values[j as usize] = None;
                    i = j;
                }
            }
        })
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        let mut i = self.compute_idx(key);
        let orig_idx = i;
        REGISTRY.with(|registry| {
            let registry = registry.borrow();
            let storage = registry.get(&self.id()).unwrap();
            while i < self.capacity {
                let entry_key = storage.keys[i as usize];
                if entry_key == 0 {
                    *idx = i;
                    return false;
                }
                if entry_key == key {
                    *idx = i;
                    return true;
                }
                i += 1;
            }

            i = 0;
            while i < orig_idx {
                let entry_key = storage.keys[i as usize];
                if entry_key == 0 {
                    *idx = i;
                    return false;
                }
                if entry_key == key {
                    *idx = i;
                    return true;
                }
                i += 1;
            }
            false
        })
    }

    pub fn hash_table_delete_last_found(&mut self) -> bool {
        let idx = REGISTRY.with(|registry| {
            registry
                .borrow()
                .get(&self.id())
                .map(|storage| storage.last_found_idx)
                .unwrap_or(0)
        });
        if !self.handle_gap(idx) {
            return false;
        }
        self.size -= 1;
        true
    }

    fn insert_existing(&mut self, key: u64, value: &'static Box<dyn Any>) -> bool {
        let mut idx = 0;
        if self.find_entry(key, &mut idx) {
            return false;
        }
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            let storage = registry.get_mut(&self.id()).unwrap();
            if storage.keys[idx as usize] != 0 {
                return false;
            }
            storage.keys[idx as usize] = key;
            storage.values[idx as usize] = Some(value);
            true
        })
        .then(|| {
            self.size += 1;
        })
        .is_some()
    }

    fn id(&self) -> usize {
        self.data.as_ptr() as usize
    }
}

fn empty_slice<T>() -> &'static mut [T] {
    Box::leak(Vec::<T>::new().into_boxed_slice())
}
