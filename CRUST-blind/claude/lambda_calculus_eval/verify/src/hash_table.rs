use crate::common;
use std::collections::HashMap;
pub const HASH_TABLE_SIZE: usize = 2000;
pub struct HashTable {
    table: HashMap<String, common::AstNode>,
}

/// Returns true if the given AstNode is a "null marker" used to represent
/// a key inserted with no associated value (the C code inserts NULL as the
/// value for lambda parameter bindings).
fn is_null_marker(node: &common::AstNode) -> bool {
    if let common::AstNodeUnion::Variable(v) = &node.node {
        // A null marker is identified by empty name and empty type and
        // VAR-type discriminant.
        if matches!(node.type_, common::AstNodeType::VAR)
            && v.name == "__NULL_MARKER__"
        {
            return true;
        }
    }
    false
}

/// Construct the canonical "null marker" AstNode used internally by the
/// hash table to represent a binding that has no associated definition.
pub(crate) fn null_marker() -> common::AstNode {
    common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "__NULL_MARKER__".to_string(),
            type_: String::new(),
        }),
    }
}

impl HashTable {
    pub fn new() -> Self {
        HashTable {
            table: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: &str, value: common::AstNode) {
        self.table.insert(key.to_string(), value);
    }
    pub fn table_exists(&self, key: &str) -> bool {
        self.table.contains_key(key)
    }
    pub fn search(&self, key: &str) -> Option<&common::AstNode> {
        match self.table.get(key) {
            Some(v) => {
                if is_null_marker(v) {
                    None
                } else {
                    Some(v)
                }
            }
            None => None,
        }
    }
    pub fn delete(&mut self, key: &str) {
        self.table.remove(key);
    }
}
pub fn hash(key: &str) -> u32 {
    // djb2 hash matching the C implementation: hash * 33 + c, mod table size
    let mut h: u64 = 5381;
    for c in key.bytes() {
        h = h.wrapping_mul(33).wrapping_add(c as u64);
    }
    (h % HASH_TABLE_SIZE as u64) as u32
}
pub fn createHashTable() -> HashTable {
    HashTable::new()
}
pub fn destroyHashTable(_hashTable: HashTable) {
    // Rust handles drop automatically; nothing to do.
}
