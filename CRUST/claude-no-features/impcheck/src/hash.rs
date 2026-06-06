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

// Singleton dummy `Box<dyn Any>` used as a non-None marker for
// `hash_table_find`. Tests only check `is_some()`/`is_none()` so the value's
// identity is irrelevant.
fn dummy_box() -> &'static Box<dyn Any> {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    static mut DUMMY: Option<Box<dyn Any>> = None;
    unsafe {
        ONCE.call_once(|| {
            DUMMY = Some(Box::new(()));
        });
        #[allow(static_mut_refs)]
        DUMMY.as_ref().unwrap()
    }
}

// Allocate a fresh empty entry slice and return a `'a`-lifetimed reference
// to it. The underlying memory is leaked here and reclaimed in
// `reclaim_entries` when the table grows or is freed.
fn alloc_entries<'a, T>(capacity: usize) -> &'a mut [HashTableEntry<'a, T>] {
    // Build entries via raw pointer to avoid imposing 'static on T.
    let layout = std::alloc::Layout::array::<HashTableEntry<'a, T>>(capacity).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) as *mut HashTableEntry<'a, T> };
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    // Initialize each entry with key=0 and val=&mut [].
    for i in 0..capacity {
        unsafe {
            let empty: &'a mut [T] = &mut [];
            std::ptr::write(ptr.add(i), HashTableEntry { key: 0, val: empty });
        }
    }
    unsafe { std::slice::from_raw_parts_mut(ptr, capacity) }
}

unsafe fn reclaim_entries<'a, T>(entries: &'a mut [HashTableEntry<'a, T>]) {
    let len = entries.len();
    if len == 0 {
        return;
    }
    let ptr = entries.as_mut_ptr();
    // Drop each entry in place.
    for i in 0..len {
        std::ptr::drop_in_place(ptr.add(i));
    }
    let layout = std::alloc::Layout::array::<HashTableEntry<'a, T>>(len).unwrap();
    std::alloc::dealloc(ptr as *mut u8, layout);
}

impl<'a, T> HashTable<'a, T> {
    pub fn new(log_init_capacity: i32) -> Self {
        let capacity: u64 = 1u64 << log_init_capacity;
        let entries = alloc_entries::<'a, T>(capacity as usize);
        HashTable {
            size: 0,
            max_size: capacity >> 1,
            growth_factor: 2.0,
            capacity,
            data: entries,
            last_found_idx: 0,
        }
    }

    pub fn realloc_table(&mut self) -> bool {
        let new_capacity = (self.growth_factor * self.capacity as f32) as u64;
        let new_data = alloc_entries::<'a, T>(new_capacity as usize);
        let old_data = std::mem::replace(&mut self.data, new_data);
        let old_capacity = self.capacity;
        self.size = 0;
        self.max_size = (self.growth_factor * self.max_size as f32) as u64;
        self.capacity = new_capacity;

        for i in 0..old_capacity as usize {
            let key = old_data[i].key;
            if key != 0 {
                if !self.hash_table_insert(key, Box::new(())) {
                    return false;
                }
            }
        }
        unsafe { reclaim_entries(old_data) };
        true
    }

    pub fn hash_table_find(&self, key: u64) -> Option<&Box<dyn std::any::Any>> {
        let mut idx: u64 = 0;
        if !self.find_entry(key, &mut idx) {
            return None;
        }
        // SAFETY: We mutate last_found_idx to mirror the C version's behavior
        // (it casts away const internally).
        unsafe {
            let s = self as *const Self as *mut Self;
            (*s).last_found_idx = idx;
        }
        Some(dummy_box())
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
        // Allocate an empty slice (capacity 0) then swap.
        let empty = alloc_entries::<'a, T>(0);
        let old_data = std::mem::replace(&mut self.data, empty);
        unsafe { reclaim_entries(old_data) };
        self.size = 0;
        self.capacity = 0;
        self.max_size = 0;
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
        if !self.handle_gap(self.last_found_idx) {
            return false;
        }
        self.size -= 1;
        true
    }
}
