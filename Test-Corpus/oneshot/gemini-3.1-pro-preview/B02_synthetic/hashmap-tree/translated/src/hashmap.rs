use std::collections::HashMap as StdHashMap;

pub type TreeId = u64;

pub struct HashMap<V> {
    map: StdHashMap<TreeId, V>,
}

impl<V> HashMap<V> {
    pub fn new() -> Self {
        Self {
            map: StdHashMap::new(),
        }
    }

    pub fn put(&mut self, key: TreeId, value: V) -> Result<(), ()> {
        self.map.insert(key, value);
        Ok(())
    }

    pub fn get(&self, key: TreeId) -> Option<&V> {
        self.map.get(&key)
    }

    pub fn get_mut(&mut self, key: TreeId) -> Option<&mut V> {
        self.map.get_mut(&key)
    }

    pub fn remove(&mut self, key: TreeId) -> Option<V> {
        self.map.remove(&key)
    }

    pub fn contains(&self, key: TreeId) -> bool {
        self.map.contains_key(&key)
    }

    pub fn size(&self) -> usize {
        self.map.len()
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }
}
