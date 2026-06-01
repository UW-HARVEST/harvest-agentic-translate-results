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

thread_local! {
    static TABLE_REGISTRY: RefCell<HashMap<usize, u64>> = RefCell::new(HashMap::new());
    static NEXT_TABLE_ID: RefCell<u64> = RefCell::new(1);
    // For each table id we map key -> &'static Box<dyn Any> (leaked)
    static STORAGE: RefCell<HashMap<u64, HashMap<u64, &'static Box<dyn Any>>>> = RefCell::new(HashMap::new());
    static LAST_FOUND: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
}

fn get_or_create_id<T>(ht: &HashTable<T>) -> u64 {
    let addr = ht.data.as_ptr() as usize;
    TABLE_REGISTRY.with(|reg| {
        let mut g = reg.borrow_mut();
        if let Some(&id) = g.get(&addr) {
            return id;
        }
        let id = NEXT_TABLE_ID.with(|n| {
            let mut nb = n.borrow_mut();
            let v = *nb;
            *nb += 1;
            v
        });
        g.insert(addr, id);
        STORAGE.with(|s| {
            s.borrow_mut().entry(id).or_insert_with(HashMap::new);
        });
        id
    })
}

fn register_addr_for_id(addr: usize, id: u64) {
    TABLE_REGISTRY.with(|reg| {
        reg.borrow_mut().insert(addr, id);
    });
}

impl<T: 'static> HashTable<'_, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let cap = capacity as usize;

        let mut entries: Vec<HashTableEntry<'static, T>> = Vec::with_capacity(cap);
        for _ in 0..cap {
            let empty: &'static mut [T] = Box::leak(Vec::<T>::new().into_boxed_slice());
            entries.push(HashTableEntry {
                key: 0,
                val: empty,
            });
        }
        let boxed = entries.into_boxed_slice();
        let leaked: &'static mut [HashTableEntry<'static, T>] = Box::leak(boxed);
        let data: &'_ mut [HashTableEntry<'_, T>] = unsafe { std::mem::transmute(leaked) };

        let ht = HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data,
            last_found_idx: 0,
        };
        get_or_create_id(&ht);
        ht
    }

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * (self.capacity as f32)) as u64;
        let new_cap = new_capacity as usize;
        let old_cap = self.capacity as usize;

        let mut old_keys: Vec<u64> = Vec::with_capacity(old_cap);
        for i in 0..old_cap {
            old_keys.push(self.data[i].key);
        }

        let mut entries: Vec<HashTableEntry<'static, T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            let empty: &'static mut [T] = Box::leak(Vec::<T>::new().into_boxed_slice());
            entries.push(HashTableEntry {
                key: 0,
                val: empty,
            });
        }
        let boxed = entries.into_boxed_slice();
        let leaked: &'static mut [HashTableEntry<'static, T>] = Box::leak(boxed);
        let new_data: &'_ mut [HashTableEntry<'_, T>] = unsafe { std::mem::transmute(leaked) };

        let old_id = get_or_create_id(self);

        self.data = new_data;
        self.size = 0;
        self.max_size = (self.growth_factor * (self.max_size as f32)) as u64;
        self.capacity = new_capacity;

        register_addr_for_id(self.data.as_ptr() as usize, old_id);

        for k in old_keys.into_iter() {
            if k != 0 {
                if !self.reinsert_key(k) {
                    return false;
                }
            }
        }
        true
    }

    fn reinsert_key(&mut self, key: u64) -> bool {
        let mut idx: u64 = 0;
        if Self::find_entry(self, key, &mut idx) {
            return false;
        }
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        self.data[idx as usize].key = key;
        self.size += 1;
        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn std::any::Any>> {
        let mut idx: u64 = 0;
        if !Self::find_entry(self, key, &mut idx) {
            return None;
        }
        let id = get_or_create_id(self);
        LAST_FOUND.with(|lf| {
            lf.borrow_mut().insert(id, idx);
        });
        STORAGE.with(|s| {
            let g = s.borrow();
            let table = g.get(&id)?;
            // The stored value is &'static Box<dyn Any>; safe to copy
            let r: &'static Box<dyn Any> = *table.get(&key)?;
            Some(r)
        })
    }

    pub fn hash_table_delete(&mut self, key: u64) -> bool {
        let mut idx: u64 = 0;
        if !Self::find_entry(self, key, &mut idx) {
            return false;
        }
        if !Self::handle_gap(self, idx) {
            return false;
        }
        let id = get_or_create_id(self);
        STORAGE.with(|s| {
            if let Some(m) = s.borrow_mut().get_mut(&id) {
                m.remove(&key);
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
            if !Self::realloc_table(self) {
                return false;
            }
            if self.size >= self.max_size {
                return false;
            }
        }
        let mut idx: u64 = 0;
        if Self::find_entry(self, key, &mut idx) {
            return false;
        }
        if !Self::cell_empty(&self.data[idx as usize]) {
            return false;
        }
        self.data[idx as usize].key = key;
        self.size += 1;

        let id = get_or_create_id(self);
        // Leak the box and store a static reference to it
        let leaked_box: &'static Box<dyn Any> = Box::leak(Box::new(val));
        STORAGE.with(|s| {
            s.borrow_mut()
                .entry(id)
                .or_insert_with(HashMap::new)
                .insert(key, leaked_box);
        });
        true
    }

    pub fn compute_hash(key: u64) -> u64 {
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }

    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        entry.key == 0
    }

    pub fn hash_table_free(&mut self) {
        let id = get_or_create_id(self);
        STORAGE.with(|s| {
            s.borrow_mut().remove(&id);
        });
        self.size = 0;
        self.capacity = 0;
        self.data = &mut [];
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        let mask = self.capacity - 1;
        loop {
            j = (j + 1) & mask;
            if Self::cell_empty(&self.data[j as usize]) {
                self.data[i as usize].key = 0;
                return true;
            }
            let k = Self::compute_hash(self.data[j as usize].key) & mask;
            let move_it = (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j);
            if move_it {
                let key = self.data[j as usize].key;
                self.data[i as usize].key = key;
                self.data[j as usize].key = 0;
                i = j;
            }
        }
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        let mask = self.capacity - 1;
        let orig = Self::compute_hash(key) & mask;
        let mut i = orig;
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
        while i < orig {
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
        let id = get_or_create_id(self);
        let idx = LAST_FOUND.with(|lf| {
            *lf.borrow().get(&id).unwrap_or(&self.last_found_idx)
        });
        let key = self.data[idx as usize].key;
        if !Self::handle_gap(self, idx) {
            return false;
        }
        if key != 0 {
            STORAGE.with(|s| {
                if let Some(m) = s.borrow_mut().get_mut(&id) {
                    m.remove(&key);
                }
            });
        }
        self.size -= 1;
        true
    }
}
