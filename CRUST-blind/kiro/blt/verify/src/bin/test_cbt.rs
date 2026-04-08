use blt::cbt::Cbt;

#[test]
fn test_empty_tree() {
    let cbt = Cbt::cbt_new();
    assert_eq!(cbt.cbt_size(), 0);
    assert!(!cbt.cbt_has("x"));
    assert!(cbt.cbt_first().is_none());
    assert!(cbt.cbt_last().is_none());
}

#[test]
fn test_insert_and_retrieve() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(42i64), "hello");
    cbt.cbt_put_at(Box::new(99i64), "world");
    assert_eq!(cbt.cbt_size(), 2);
    assert!(cbt.cbt_has("hello"));
    assert!(!cbt.cbt_has("missing"));

    // Check iteration order (sorted)
    let mut keys = Vec::new();
    let mut it = cbt.cbt_first();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = Cbt::cbt_next(leaf);
    }
    assert_eq!(keys, vec!["hello", "world"]);

    // Check last
    assert_eq!(cbt.cbt_last().unwrap().key, "world");
}

#[test]
fn test_multiple_elements_ordering() {
    let mut cbt = Cbt::cbt_new();
    let keys_input = ["cherry", "apple", "banana", "date", "elderberry"];
    for (i, k) in keys_input.iter().enumerate() {
        cbt.cbt_put_at(Box::new((i + 1) as i64), k);
    }
    assert_eq!(cbt.cbt_size(), 5);

    let mut keys = Vec::new();
    let mut it = cbt.cbt_first();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = Cbt::cbt_next(leaf);
    }
    assert_eq!(keys, vec!["apple", "banana", "cherry", "date", "elderberry"]);
}

#[test]
fn test_cbt_at() {
    let mut cbt = Cbt::cbt_new();
    let keys_input = ["cherry", "apple", "banana", "date", "elderberry"];
    for (i, k) in keys_input.iter().enumerate() {
        cbt.cbt_put_at(Box::new((i + 1) as i64), k);
    }
    let at = cbt.cbt_at("banana");
    assert!(at.is_some());
    assert_eq!(at.unwrap().key, "banana");
    assert!(cbt.cbt_at("fig").is_none());
}

#[test]
fn test_remove() {
    let mut cbt = Cbt::cbt_new();
    let keys_input = ["cherry", "apple", "banana", "date", "elderberry"];
    for (i, k) in keys_input.iter().enumerate() {
        cbt.cbt_put_at(Box::new((i + 1) as i64), k);
    }
    let removed = cbt.cbt_remove("cherry");
    assert!(removed.is_some());
    assert_eq!(cbt.cbt_size(), 4);
    assert!(!cbt.cbt_has("cherry"));

    let mut keys = Vec::new();
    let mut it = cbt.cbt_first();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = Cbt::cbt_next(leaf);
    }
    assert_eq!(keys, vec!["apple", "banana", "date", "elderberry"]);
}

#[test]
fn test_put_with() {
    let mut cbt = Cbt::cbt_new();
    let leaf1 = cbt.cbt_put_with(|old| {
        let v = old.downcast_ref::<i64>().copied().unwrap_or(0);
        Box::new(v + 1)
    }, "foo");
    assert_eq!(leaf1.key, "foo");

    let leaf2 = cbt.cbt_put_with(|old| {
        let v = old.downcast_ref::<i64>().copied().unwrap_or(0);
        Box::new(v + 1)
    }, "foo");
    assert_eq!(leaf2.key, "foo");

    let leaf3 = cbt.cbt_put_with(|old| {
        let v = old.downcast_ref::<i64>().copied().unwrap_or(0);
        Box::new(v + 1)
    }, "bar");
    assert_eq!(leaf3.key, "bar");
    assert_eq!(cbt.cbt_size(), 2);
}

#[test]
fn test_remove_all() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i64), "a");
    cbt.cbt_put_at(Box::new(2i64), "b");
    cbt.cbt_put_at(Box::new(3i64), "c");
    cbt.cbt_remove_all();
    assert_eq!(cbt.cbt_size(), 0);
    assert!(cbt.cbt_first().is_none());
}

#[test]
fn test_forall() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i64), "x");
    cbt.cbt_put_at(Box::new(2i64), "y");
    cbt.cbt_put_at(Box::new(3i64), "z");
    let mut keys = Vec::new();
    cbt.cbt_forall(|leaf| keys.push(leaf.key.clone()));
    assert_eq!(keys, vec!["x", "y", "z"]);
}

#[test]
fn test_forall_at() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i64), "x");
    cbt.cbt_put_at(Box::new(2i64), "y");
    cbt.cbt_put_at(Box::new(3i64), "z");
    let mut keys = Vec::new();
    cbt.cbt_forall_at(|_data, key| keys.push(key.to_string()));
    assert_eq!(keys, vec!["x", "y", "z"]);
}

#[test]
fn test_cbt_insert() {
    let mut cbt = Cbt::cbt_new();
    let (is_new, leaf) = cbt.cbt_insert("foo");
    assert!(is_new);
    assert_eq!(leaf.key, "foo");
    let (is_new2, leaf2) = cbt.cbt_insert("foo");
    assert!(!is_new2);
    assert_eq!(leaf2.key, "foo");
    let (is_new3, leaf3) = cbt.cbt_insert("bar");
    assert!(is_new3);
    assert_eq!(leaf3.key, "bar");
    assert_eq!(cbt.cbt_size(), 2);
}

#[test]
fn test_cbt_key() {
    let mut cbt = Cbt::cbt_new();
    let (_, leaf) = cbt.cbt_insert("mykey");
    assert_eq!(cbt.cbt_key(&leaf), "mykey");
}

#[test]
fn test_remove_all_with() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i64), "a");
    cbt.cbt_put_at(Box::new(2i64), "b");
    let mut removed_keys = Vec::new();
    cbt.cbt_remove_all_with(|_data, key| removed_keys.push(key.to_string()));
    assert_eq!(cbt.cbt_size(), 0);
    // Keys should be in sorted order (linked list order)
    assert_eq!(removed_keys, vec!["a", "b"]);
}

fn main() {}
