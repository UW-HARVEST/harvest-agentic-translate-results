use crate::common;
use std::collections::HashMap;

pub const HASH_TABLE_SIZE: usize = 2000;

pub struct HashTable {
    table: HashMap<String, Option<common::AstNode>>,
}

impl HashTable {
    pub fn new() -> Self {
        HashTable {
            table: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: common::AstNode) {
        self.table.insert(key.to_string(), Some(value));
    }

    pub fn insert_null(&mut self, key: &str) {
        self.table.insert(key.to_string(), None);
    }

    pub fn table_exists(&self, key: &str) -> bool {
        self.table.contains_key(key)
    }

    pub fn search(&self, key: &str) -> Option<&common::AstNode> {
        match self.table.get(key) {
            Some(Some(v)) => Some(v),
            _ => None,
        }
    }

    pub fn delete(&mut self, key: &str) {
        self.table.remove(key);
    }
}

pub fn hash(key: &str) -> u32 {
    let mut hash: u64 = 5381;
    for c in key.bytes() {
        hash = hash.wrapping_shl(5).wrapping_add(hash).wrapping_add(c as u64);
    }
    (hash % HASH_TABLE_SIZE as u64) as u32
}

pub fn createHashTable() -> HashTable {
    HashTable::new()
}

pub fn destroyHashTable(_hashTable: HashTable) {
    // dropped automatically
}
