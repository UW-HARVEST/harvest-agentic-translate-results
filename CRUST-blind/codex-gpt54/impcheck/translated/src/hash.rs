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

struct HashState {
    size: u64,
    max_size: u64,
    growth_factor: f32,
    capacity: u64,
    keys: Vec<u64>,
    vals: Vec<Option<&'static Box<dyn Any>>>,
    last_found_idx: u64,
}

thread_local! {
    static HASH_TABLES: RefCell<HashMap<usize, HashState>> = RefCell::new(HashMap::new());
}

impl<T: 'static> HashTable<'_, T> {
fn id(&self) -> usize {
    self.data.as_ptr() as usize
}

fn compute_idx_for_capacity(capacity: u64, key: u64) -> u64 {
    Self::compute_hash(key) & (capacity - 1)
}

fn sync_from_state(&mut self, state: &HashState) {
    self.size = state.size;
    self.max_size = state.max_size;
    self.growth_factor = state.growth_factor;
    self.capacity = state.capacity;
    self.last_found_idx = state.last_found_idx;
}

fn find_entry_in_state(state: &HashState, key: u64, idx: &mut u64) -> bool {
    let mut i = Self::compute_idx_for_capacity(state.capacity, key);
    let orig_idx = i;
    while i < state.capacity {
        let pos = i as usize;
        if state.keys[pos] == 0 {
            *idx = i;
            return false;
        }
        if state.keys[pos] == key {
            *idx = i;
            return true;
        }
        i += 1;
    }
    i = 0;
    while i < orig_idx {
        let pos = i as usize;
        if state.keys[pos] == 0 {
            *idx = i;
            return false;
        }
        if state.keys[pos] == key {
            *idx = i;
            return true;
        }
        i += 1;
    }
    false
}

fn handle_gap_in_state(state: &mut HashState, idx_of_gap: u64) -> bool {
    let mut i = idx_of_gap;
    let mut j = i;
    loop {
        j = (j + 1) & (state.capacity - 1);
        let j_pos = j as usize;
        if state.keys[j_pos] == 0 {
            let i_pos = i as usize;
            state.keys[i_pos] = 0;
            state.vals[i_pos] = None;
            return true;
        }
        let k = Self::compute_idx_for_capacity(state.capacity, state.keys[j_pos]);
        if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
            let i_pos = i as usize;
            state.keys[i_pos] = state.keys[j_pos];
            state.vals[i_pos] = state.vals[j_pos].take();
            state.keys[j_pos] = 0;
            state.vals[j_pos] = None;
            i = j;
        }
    }
}

pub fn new(log_init_capacity: i32) -> Self {
    let capacity = if log_init_capacity <= 0 {
        1
    } else {
        1_u64 << log_init_capacity
    };
    let empty_vals: &'static mut [T] = Box::leak(Vec::<T>::new().into_boxed_slice());
    let entry = HashTableEntry { key: 0, val: empty_vals };
    let data = Box::leak(vec![entry].into_boxed_slice());
    let id = data.as_ptr() as usize;
    HASH_TABLES.with(|tables| {
        tables.borrow_mut().insert(
            id,
            HashState {
                size: 0,
                max_size: capacity >> 1,
                growth_factor: 2.0,
                capacity,
                keys: vec![0; capacity as usize],
                vals: vec![None; capacity as usize],
                last_found_idx: 0,
            },
        );
    });
    Self {
        size: 0,
        max_size: capacity >> 1,
        growth_factor: 2.0,
        capacity,
        data,
        last_found_idx: 0,
    }
}
pub fn realloc_table(&mut self) -> bool {
    let id = self.id();
    let mut ok = true;
    HASH_TABLES.with(|tables| {
        if let Some(state) = tables.borrow_mut().get_mut(&id) {
            let new_capacity = (state.growth_factor * state.capacity as f32) as u64;
            let old_keys = state.keys.clone();
            let old_vals = state.vals.clone();
            state.capacity = new_capacity;
            state.max_size = (state.growth_factor * state.max_size as f32) as u64;
            state.keys = vec![0; new_capacity as usize];
            state.vals = vec![None; new_capacity as usize];
            state.size = 0;
            for (key, val) in old_keys.into_iter().zip(old_vals.into_iter()) {
                if key == 0 {
                    continue;
                }
                let mut idx = 0;
                if Self::find_entry_in_state(state, key, &mut idx) {
                    ok = false;
                    return;
                }
                state.keys[idx as usize] = key;
                state.vals[idx as usize] = val;
                state.size += 1;
            }
            self.sync_from_state(state);
        }
    });
    ok
}
pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn Any>> {
    let id = self.id();
    HASH_TABLES.with(|tables| {
        let mut tables = tables.borrow_mut();
        let state = tables.get_mut(&id)?;
        let mut idx = 0;
        if !Self::find_entry_in_state(state, key, &mut idx) {
            return None;
        }
        state.last_found_idx = idx;
        state.vals[idx as usize]
    })
}
pub fn hash_table_delete(&mut self, key: u64) -> bool {
    let id = self.id();
    let mut result = false;
    HASH_TABLES.with(|tables| {
        if let Some(state) = tables.borrow_mut().get_mut(&id) {
            let mut idx = 0;
            if !Self::find_entry_in_state(state, key, &mut idx) {
                result = false;
                return;
            }
            result = Self::handle_gap_in_state(state, idx);
            if result {
                state.size -= 1;
                self.sync_from_state(state);
            }
        }
    });
    result
}
pub fn compute_idx(&self, key: u64) -> u64 {
    Self::compute_idx_for_capacity(self.capacity, key)
}
pub fn hash_table_insert(&mut self, key: u64, val: Box<dyn Any>) -> bool {
    if key == 0 {
        return false;
    }
    if self.size == self.max_size && !self.realloc_table() {
        return false;
    }
    let id = self.id();
    let leaked: &'static Box<dyn Any> = Box::leak(Box::new(val));
    let mut result = false;
    HASH_TABLES.with(|tables| {
        if let Some(state) = tables.borrow_mut().get_mut(&id) {
            if state.size >= state.max_size {
                result = false;
                return;
            }
            let mut idx = 0;
            if Self::find_entry_in_state(state, key, &mut idx) {
                result = false;
                return;
            }
            if state.keys[idx as usize] != 0 {
                result = false;
                return;
            }
            state.keys[idx as usize] = key;
            state.vals[idx as usize] = Some(leaked);
            state.size += 1;
            self.sync_from_state(state);
            result = true;
        }
    });
    result
}
pub fn compute_hash(key: u64) -> u64 {
    (0xcbf29ce484222325_u64 ^ key).wrapping_mul(0x0000_0100_0000_01B3_u64)
}
pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
    entry.key == 0
}
pub fn hash_table_free(&mut self) {
    HASH_TABLES.with(|tables| {
        tables.borrow_mut().remove(&self.id());
    });
    self.size = 0;
    self.max_size = 0;
    self.capacity = 0;
    self.last_found_idx = 0;
}
pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
    let id = self.id();
    let mut result = false;
    HASH_TABLES.with(|tables| {
        if let Some(state) = tables.borrow_mut().get_mut(&id) {
            result = Self::handle_gap_in_state(state, idx_of_gap);
            if result {
                self.sync_from_state(state);
            }
        }
    });
    result
}
pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
    HASH_TABLES.with(|tables| {
        tables
            .borrow()
            .get(&self.id())
            .map(|state| Self::find_entry_in_state(state, key, idx))
            .unwrap_or(false)
    })
}
pub fn hash_table_delete_last_found(&mut self) -> bool {
    let id = self.id();
    let mut result = false;
    HASH_TABLES.with(|tables| {
        if let Some(state) = tables.borrow_mut().get_mut(&id) {
            let idx = state.last_found_idx;
            result = Self::handle_gap_in_state(state, idx);
            if result {
                state.size -= 1;
                self.sync_from_state(state);
            }
        }
    });
    result
}
}
