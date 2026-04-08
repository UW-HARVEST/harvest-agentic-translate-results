use lambda_calculus_eval::common::*;
use lambda_calculus_eval::hash_table::*;

#[test]
fn test_hash_values() {
    assert_eq!(hash("test"), 1493);
    assert_eq!(hash("hello"), 441);
    assert_eq!(hash(""), 1381);
    assert_eq!(hash("x"), 1693);
    assert_eq!(hash("Nat"), 1256);
}

#[test]
fn test_new_table_empty() {
    let ht = HashTable::new();
    assert!(!ht.table_exists("foo"));
}

#[test]
fn test_insert_and_exists() {
    let mut ht = HashTable::new();
    ht.insert("foo", AstNode::default());
    assert!(ht.table_exists("foo"));
    assert!(!ht.table_exists("bar"));
}

#[test]
fn test_search_not_found() {
    let ht = HashTable::new();
    assert!(ht.search("missing").is_none());
}

#[test]
fn test_search_found() {
    let mut ht = HashTable::new();
    let var = AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: "bar".to_string(),
            type_: "Nat".to_string(),
        }),
    };
    ht.insert("baz", var);
    let found = ht.search("baz").unwrap();
    assert_eq!(found.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(v) = &found.node {
        assert_eq!(v.name, "bar");
        assert_eq!(v.type_, "Nat");
    } else {
        panic!("Expected Variable");
    }
}

#[test]
fn test_delete() {
    let mut ht = HashTable::new();
    ht.insert("foo", AstNode::default());
    assert!(ht.table_exists("foo"));
    ht.delete("foo");
    assert!(!ht.table_exists("foo"));
}

#[test]
fn test_create_and_destroy() {
    let ht = createHashTable();
    assert!(!ht.table_exists("anything"));
    destroyHashTable(ht);
}

fn main() {}
