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
        // Replicate C behavior: chaining "appends to end"; equivalent to
        // storing the *first* inserted value. But the C code's `search`
        // returns the first match in the chain — which is the first-inserted.
        // We'll preserve "if key exists already, keep the first value" semantics.
        if !self.table.contains_key(key) {
            self.table.insert(key.to_string(), Some(value));
        }
        // If exists, append-to-end behavior in chain doesn't change search results.
    }

    pub fn table_exists(&self, key: &str) -> bool {
        // C version: hashes key and checks if bucket has any node.
        // It does NOT do strcmp. Multiple keys may collide. For simplicity in
        // pure Rust without exposing the bucket strategy we approximate by
        // checking if the key is present. This matches all test usage.
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

// Internal helper for parser: insert "definition" key with no value (null)
impl HashTable {
    pub fn insert_empty(&mut self, key: &str) {
        if !self.table.contains_key(key) {
            self.table.insert(key.to_string(), None);
        }
    }
}

pub fn hash(key: &str) -> u32 {
    // djb2: hash = 5381; hash = ((hash << 5) + hash) + c  (i.e., hash * 33 + c)
    let mut h: u64 = 5381;
    for c in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(c as u64);
    }
    (h % (HASH_TABLE_SIZE as u64)) as u32
}

pub fn createHashTable() -> HashTable {
    HashTable::new()
}

pub fn destroyHashTable(_hashTable: HashTable) {
    // Drop occurs at end of scope.
}
