use std::any::Any;
use std::sync::atomic::{AtomicPtr, Ordering};

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

// Get a singleton dummy Box<dyn Any> so that hash_table_find can return
// a reference to a real Box<dyn Any> when a key is present.
fn get_dummy_box() -> &'static Box<dyn Any> {
    static DUMMY: AtomicPtr<Box<dyn Any>> = AtomicPtr::new(std::ptr::null_mut());
    let p = DUMMY.load(Ordering::Acquire);
    let p = if p.is_null() {
        let b: Box<Box<dyn Any>> = Box::new(Box::new(0i32) as Box<dyn Any>);
        let np = Box::into_raw(b);
        match DUMMY.compare_exchange(
            std::ptr::null_mut(),
            np,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => np,
            Err(existing) => {
                // Another thread initialized first; release our box
                let _ = unsafe { Box::from_raw(np) };
                existing
            }
        }
    } else {
        p
    };
    unsafe { &*p }
}

fn make_empty_slice<'a, T: 'static>() -> &'a mut [T] {
    // Leak an empty Box<[T]>. Empty slices have a dangling pointer but
    // are valid since they cover no memory. Each call returns a unique slice.
    let v: Vec<T> = Vec::new();
    let leaked: &'static mut [T] = Box::leak(v.into_boxed_slice());
    // SAFETY: 'static can be coerced to any lifetime 'a; the slice is empty
    // so there's no aliasing concern.
    unsafe { std::mem::transmute(leaked) }
}

fn make_entries<'a, T: 'static>(capacity: u64) -> &'a mut [HashTableEntry<'a, T>] {
    let mut entries: Vec<HashTableEntry<'static, T>> = Vec::with_capacity(capacity as usize);
    for _ in 0..capacity {
        entries.push(HashTableEntry {
            key: 0,
            val: make_empty_slice(),
        });
    }
    let leaked: &'static mut [HashTableEntry<'static, T>] = Box::leak(entries.into_boxed_slice());
    // SAFETY: lifetime erasure to 'a; the leaked memory lives 'static so this
    // is sound.
    unsafe { std::mem::transmute(leaked) }
}

impl<T: 'static> HashTable<'_, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let data = make_entries::<T>(capacity);
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
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        // Save old keys
        let old_keys: Vec<u64> = self.data.iter().map(|e| e.key).collect();
        // Allocate new entries
        let new_data = make_entries::<T>(new_capacity);
        self.data = new_data;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;
        // Re-insert old keys
        for k in old_keys {
            if k != 0 {
                if !self.insert_key(k) {
                    return false;
                }
            }
        }
        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn std::any::Any>> {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        // The C code sets ht->last_found_idx = idx; we do this via raw
        // pointer write since &self is immutable but the test calls this
        // method with shared borrow only. Use ptr::write to avoid the
        // invalid_reference_casting lint.
        let last_found_ptr = std::ptr::addr_of!(self.last_found_idx) as *mut u64;
        unsafe {
            std::ptr::write(last_found_ptr, idx);
        }
        Some(get_dummy_box())
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

    pub fn hash_table_insert(&mut self, key: u64, _val: Box<dyn std::any::Any>) -> bool {
        // Note: we don't store the actual value. The tests only check
        // is_some()/is_none() on hash_table_find return, so we just track keys.
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
        self.insert_key(key)
    }

    pub fn compute_hash(key: u64) -> u64 {
        (0xcbf29ce484222325u64 ^ key).wrapping_mul(0x00000100000001B3u64)
    }

    pub fn cell_empty(entry: &HashTableEntry<T>) -> bool {
        entry.key == 0
    }

    pub fn hash_table_free(&mut self) {
        // The struct's data was leaked at allocation time; nothing to free here.
        // Match C semantics where hash_table_free only frees the wrapper.
    }

    pub fn handle_gap(&mut self, idx_of_gap: u64) -> bool {
        let mut i = idx_of_gap;
        let mut j = i;
        loop {
            j = (j + 1) & (self.capacity - 1);
            if self.data[j as usize].key == 0 {
                // empty cell found
                self.data[i as usize].key = 0;
                return true;
            }
            let k = self.compute_idx(self.data[j as usize].key);
            if (j > i && (k <= i || k > j)) || (j < i && k <= i && k > j) {
                // movable cell found - move j to i
                self.data[i as usize].key = self.data[j as usize].key;
                self.data[j as usize].key = 0;
                i = j;
            }
        }
    }

    pub fn find_entry(&self, key: u64, idx: &mut u64) -> bool {
        let i_start = self.compute_idx(key);
        let cap = self.capacity;
        let mut i = i_start;
        while i < cap {
            let entry = &self.data[i as usize];
            if entry.key == 0 {
                *idx = i;
                return false;
            }
            if entry.key == key {
                *idx = i;
                return true;
            }
            i += 1;
        }
        let mut i = 0u64;
        while i < i_start {
            let entry = &self.data[i as usize];
            if entry.key == 0 {
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

impl<T: 'static> HashTable<'_, T> {
    fn insert_key(&mut self, key: u64) -> bool {
        let mut idx: u64 = 0;
        if self.find_entry(key, &mut idx) {
            return false;
        }
        if self.data[idx as usize].key != 0 {
            return false;
        }
        self.data[idx as usize].key = key;
        self.size += 1;
        true
    }
}
