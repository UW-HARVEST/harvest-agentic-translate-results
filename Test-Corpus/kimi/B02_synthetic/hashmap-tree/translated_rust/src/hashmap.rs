pub type TreeId = u64;

const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f64 = 0.75;

#[derive(Clone)]
struct Entry<V> {
    key: TreeId,
    value: Option<V>,
    occupied: bool,
    deleted: bool,
}

impl<V> Entry<V> {
    fn new() -> Self {
        Self {
            key: 0,
            value: None,
            occupied: false,
            deleted: false,
        }
    }
}

pub struct HashMap<V> {
    entries: Vec<Entry<V>>,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

impl<V> HashMap<V> {
    pub fn new() -> Self {
        let mut entries = Vec::with_capacity(INITIAL_CAPACITY);
        for _ in 0..INITIAL_CAPACITY {
            entries.push(Entry::new());
        }
        
        Self {
            entries,
            capacity: INITIAL_CAPACITY,
            size: 0,
            deleted_count: 0,
        }
    }
    
    fn hash(key: TreeId) -> u64 {
        let mut hash: u64 = 14695981039346656037;
        let bytes = key.to_le_bytes();
        
        for byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        
        hash
    }
    
    fn should_resize(&self) -> bool {
        let load = (self.size + self.deleted_count) as f64 / self.capacity as f64;
        load > LOAD_FACTOR
    }
    
    fn resize(&mut self) {
        let old_entries = std::mem::replace(&mut self.entries, Vec::new());
        let old_capacity = self.capacity;
        
        self.capacity *= 2;
        self.entries = Vec::with_capacity(self.capacity);
        for _ in 0..self.capacity {
            self.entries.push(Entry::new());
        }
        
        self.size = 0;
        self.deleted_count = 0;
        
        for entry in old_entries {
            if entry.occupied && !entry.deleted {
                if let Some(value) = entry.value {
                    self.put(entry.key, value);
                }
            }
        }
    }
    
    pub fn put(&mut self, key: TreeId, value: V) {
        if self.should_resize() {
            self.resize();
        }
        
        let hash = Self::hash(key);
        let mut index = (hash as usize) % self.capacity;
        let mut probe = 0;
        
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            
            if !self.entries[current].occupied {
                self.entries[current].key = key;
                self.entries[current].value = Some(value);
                self.entries[current].occupied = true;
                self.entries[current].deleted = false;
                self.size += 1;
                return;
            } else if self.entries[current].deleted {
                self.entries[current].key = key;
                self.entries[current].value = Some(value);
                self.entries[current].deleted = false;
                self.size += 1;
                self.deleted_count -= 1;
                return;
            } else if self.entries[current].key == key {
                self.entries[current].value = Some(value);
                return;
            }
            
            probe += 1;
        }
    }
    
    pub fn get(&self, key: TreeId) -> Option<&V> {
        let hash = Self::hash(key);
        let mut index = (hash as usize) % self.capacity;
        let mut probe = 0;
        
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            
            if !self.entries[current].occupied {
                return None;
            }
            
            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value.as_ref();
            }
            
            probe += 1;
        }
        
        None
    }
    
    pub fn get_mut(&mut self, key: TreeId) -> Option<&mut V> {
        let hash = Self::hash(key);
        let mut index = (hash as usize) % self.capacity;
        let mut probe = 0;
        
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            
            if !self.entries[current].occupied {
                return None;
            }
            
            if !self.entries[current].deleted && self.entries[current].key == key {
                return self.entries[current].value.as_mut();
            }
            
            probe += 1;
        }
        
        None
    }
    
    pub fn remove(&mut self, key: TreeId) -> Option<V> {
        let hash = Self::hash(key);
        let mut index = (hash as usize) % self.capacity;
        let mut probe = 0;
        
        while probe < self.capacity {
            let current = (index + probe) % self.capacity;
            
            if !self.entries[current].occupied {
                return None;
            }
            
            if !self.entries[current].deleted && self.entries[current].key == key {
                let value = self.entries[current].value.take();
                self.entries[current].deleted = true;
                self.size -= 1;
                self.deleted_count += 1;
                return value;
            }
            
            probe += 1;
        }
        
        None
    }
    
    pub fn contains(&self, key: TreeId) -> bool {
        self.get(key).is_some()
    }
    
    pub fn size(&self) -> usize {
        self.size
    }
    
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.occupied = false;
            entry.deleted = false;
            entry.value = None;
        }
        self.size = 0;
        self.deleted_count = 0;
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (TreeId, &V)> {
        self.entries.iter()
            .filter(|e| e.occupied && !e.deleted)
            .map(|e| (e.key, e.value.as_ref().unwrap()))
    }
}

impl<V> Default for HashMap<V> {
    fn default() -> Self {
        Self::new()
    }
}
