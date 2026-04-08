use lambda_calculus_eval::{common, hash_table};

#[test]
fn test_hash_known_values() {
    assert_eq!(hash_table::hash("test"), 1493);
    assert_eq!(hash_table::hash(""), 1381);
    assert_eq!(hash_table::hash("a"), 1670);
    assert_eq!(hash_table::hash("hello"), 441);
}

#[test]
fn test_hash_table_insert_and_exists() {
    let mut ht = hash_table::HashTable::new();
    assert!(!ht.table_exists("key1"));
    ht.insert("key1", common::AstNode::default());
    assert!(ht.table_exists("key1"));
}

#[test]
fn test_hash_table_search() {
    let mut ht = hash_table::HashTable::new();
    ht.insert("key2", common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "x".to_string(),
            type_: "Nat".to_string(),
        }),
    });
    let found = ht.search("key2");
    assert!(found.is_some());
    if let common::AstNodeUnion::Variable(ref v) = found.unwrap().node {
        assert_eq!(v.name, "x");
    }
}

#[test]
fn test_hash_table_search_missing() {
    let ht = hash_table::HashTable::new();
    assert!(ht.search("nonexistent").is_none());
}

#[test]
fn test_hash_table_delete() {
    let mut ht = hash_table::HashTable::new();
    ht.insert("key1", common::AstNode::default());
    assert!(ht.table_exists("key1"));
    ht.delete("key1");
    assert!(!ht.table_exists("key1"));
}

#[test]
fn test_create_hash_table() {
    let ht = hash_table::createHashTable();
    assert!(!ht.table_exists("anything"));
}

#[test]
fn test_hash_table_overwrite() {
    let mut ht = hash_table::HashTable::new();
    ht.insert("key", common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "first".to_string(),
            type_: String::new(),
        }),
    });
    ht.insert("key", common::AstNode {
        type_: common::AstNodeType::VAR,
        node: common::AstNodeUnion::Variable(common::Variable {
            name: "second".to_string(),
            type_: String::new(),
        }),
    });
    // HashMap overwrites, so search should return the latest
    let found = ht.search("key").unwrap();
    if let common::AstNodeUnion::Variable(ref v) = found.node {
        assert_eq!(v.name, "second");
    }
}

fn main() {}
