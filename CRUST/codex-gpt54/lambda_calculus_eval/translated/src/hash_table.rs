use crate::common;
use std::collections::HashMap;

pub const HASH_TABLE_SIZE: usize = 2000;

pub struct HashTable {
    table: HashMap<String, common::AstNode>,
}

impl HashTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &str, value: common::AstNode) {
        self.table.insert(key.to_string(), value);
    }

    pub(crate) fn insert_placeholder(&mut self, key: &str) {
        self.table.insert(key.to_string(), common::AstNode::default());
    }

    pub fn table_exists(&self, key: &str) -> bool {
        self.table.contains_key(key)
    }

    pub fn search(&self, key: &str) -> Option<&common::AstNode> {
        self.table.get(key).filter(|node| !common::is_placeholder_ast(node))
    }

    pub fn delete(&mut self, key: &str) {
        self.table.remove(key);
    }
}

pub fn hash(key: &str) -> u32 {
    let mut hash: u64 = 5381;
    for byte in key.bytes() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(u64::from(byte));
    }
    (hash % HASH_TABLE_SIZE as u64) as u32
}

pub fn createHashTable() -> HashTable {
    HashTable::new()
}

pub fn destroyHashTable(_hashTable: HashTable) {}
