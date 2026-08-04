use lambda_calculus_eval::common::{AstNode, AstNodeType, AstNodeUnion, Variable};
use lambda_calculus_eval::hash_table::{
    createHashTable, destroyHashTable, hash, HashTable, HASH_TABLE_SIZE,
};

fn make_var(name: &str, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

#[test]
fn test_constant_size() {
    assert_eq!(HASH_TABLE_SIZE, 2000);
}

#[test]
fn test_hash_value_djb2() {
    // Computed from running the C implementation:
    //   hash('test') = 1493
    //   hash('other') = 359
    //   hash('Nat') = 1256
    //   hash('Bool') = 1393
    assert_eq!(hash("test"), 1493);
    assert_eq!(hash("other"), 359);
    assert_eq!(hash("Nat"), 1256);
    assert_eq!(hash("Bool"), 1393);
}

#[test]
fn test_hash_different_inputs() {
    let h1 = hash("test");
    let h2 = hash("other");
    // Should likely differ (test in 0..2000 of a different value)
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_empty() {
    // hash of empty string = 5381 % 2000 = 1381
    let h = hash("");
    assert_eq!(h, 5381 % 2000);
}

#[test]
fn test_hash_single_char_a() {
    // hash of "a"  -> hash = 5381*33 + 97 = 177573 + 97 = 177670; 177670 % 2000 = 1670
    let h = hash("a");
    let expected = (5381u64 * 33 + 97) % 2000;
    assert_eq!(h as u64, expected);
}

#[test]
fn test_create_hash_table() {
    let _ht = createHashTable();
    // Should be empty initially
}

#[test]
fn test_destroy_hash_table() {
    let ht = createHashTable();
    destroyHashTable(ht);
}

#[test]
fn test_insert_and_search() {
    let mut ht = HashTable::new();
    let v = make_var("hello", "Nat");
    ht.insert("key1", v);
    let result = ht.search("key1");
    assert!(result.is_some());
    let n = result.unwrap();
    assert_eq!(n.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref var) = n.node {
        assert_eq!(var.name, "hello");
        assert_eq!(var.type_, "Nat");
    } else {
        panic!("Should be variable");
    }
}

#[test]
fn test_search_missing_returns_none() {
    let ht = HashTable::new();
    let r = ht.search("missing");
    assert!(r.is_none());
}

#[test]
fn test_table_exists_after_insert() {
    let mut ht = HashTable::new();
    let v = make_var("a", "Nat");
    ht.insert("foo", v);
    assert_eq!(ht.table_exists("foo"), true);
}

#[test]
fn test_table_exists_returns_false_for_missing() {
    let ht = HashTable::new();
    assert_eq!(ht.table_exists("missing"), false);
}

#[test]
fn test_delete() {
    let mut ht = HashTable::new();
    let v = make_var("a", "Nat");
    ht.insert("k", v);
    assert_eq!(ht.table_exists("k"), true);
    ht.delete("k");
    assert_eq!(ht.table_exists("k"), false);
}

#[test]
fn test_insert_empty() {
    let mut ht = HashTable::new();
    ht.insert_empty("Nat");
    // table_exists should return true (it's a "type" definition with no AstNode)
    assert_eq!(ht.table_exists("Nat"), true);
    // But search returns None because the value was None
    assert!(ht.search("Nat").is_none());
}

fn main() {}
