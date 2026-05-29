use blt::cbt::Cbt;

#[test]
fn test_cbt_new_empty_state() {
    let cbt = Cbt::cbt_new();
    assert_eq!(cbt.cbt_size(), 0);
    assert_eq!(cbt.cbt_overhead(), 72);
    assert!(!cbt.cbt_has("x"));
    assert!(cbt.cbt_at("x").is_none());
    assert!(cbt.cbt_get_at("x").is_none());
    assert!(cbt.cbt_first().is_none());
    assert!(cbt.cbt_last().is_none());
}

#[test]
fn test_cbt_put_at_and_get_at() {
    let mut cbt = Cbt::cbt_new();
    let leaf = cbt.cbt_put_at(Box::new(1i32), "hello");
    assert_eq!(leaf.key, "hello");
    cbt.cbt_put_at(Box::new(2i32), "world");
    cbt.cbt_put_at(Box::new(3i32), "foo");
    cbt.cbt_put_at(Box::new(4i32), "bar");

    assert_eq!(cbt.cbt_size(), 4);
    // 4 leaves + 3 internal nodes -> 72 + 4*40 + 3*24 = 72 + 160 + 72 = 304
    assert_eq!(cbt.cbt_overhead(), 304);

    let v = cbt
        .cbt_get_at("hello")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(1));
    let v = cbt
        .cbt_get_at("foo")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(3));
    assert!(cbt.cbt_get_at("missing").is_none());

    assert!(cbt.cbt_has("foo"));
    assert!(!cbt.cbt_has("missing"));
}

#[test]
fn test_cbt_first_last_iteration_and_key() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "hello");
    cbt.cbt_put_at(Box::new(2i32), "world");
    cbt.cbt_put_at(Box::new(3i32), "foo");
    cbt.cbt_put_at(Box::new(4i32), "bar");

    let first = cbt.cbt_first().unwrap();
    let last = cbt.cbt_last().unwrap();
    assert_eq!(cbt.cbt_key(&first), "bar");
    assert_eq!(cbt.cbt_key(&last), "world");
    let val = cbt
        .cbt_get(&first)
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(val, Some(4));
}

#[test]
fn test_cbt_forall_visits_in_sorted_order() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "hello");
    cbt.cbt_put_at(Box::new(2i32), "world");
    cbt.cbt_put_at(Box::new(3i32), "foo");
    cbt.cbt_put_at(Box::new(4i32), "bar");

    let mut keys: Vec<String> = Vec::new();
    cbt.cbt_forall(|leaf| keys.push(leaf.key.clone()));
    assert_eq!(keys, vec!["bar", "foo", "hello", "world"]);
}

#[test]
fn test_cbt_forall_at_visits_in_sorted_order() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(10i32), "alpha");
    cbt.cbt_put_at(Box::new(20i32), "beta");
    cbt.cbt_put_at(Box::new(30i32), "gamma");

    let mut entries: Vec<(String, i32)> = Vec::new();
    cbt.cbt_forall_at(|data, key| {
        let v = *data.downcast::<i32>().unwrap();
        entries.push((key.to_string(), v));
    });
    assert_eq!(
        entries,
        vec![
            ("alpha".to_string(), 10),
            ("beta".to_string(), 20),
            ("gamma".to_string(), 30),
        ]
    );
}

#[test]
fn test_cbt_remove_returns_data_and_decreases_size() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "hello");
    cbt.cbt_put_at(Box::new(2i32), "world");
    cbt.cbt_put_at(Box::new(3i32), "foo");
    cbt.cbt_put_at(Box::new(4i32), "bar");

    let removed = cbt
        .cbt_remove("foo")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(removed, Some(3));
    assert_eq!(cbt.cbt_size(), 3);
    assert!(!cbt.cbt_has("foo"));
}

#[test]
fn test_cbt_remove_all_clears() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "a");
    cbt.cbt_put_at(Box::new(2i32), "b");
    cbt.cbt_put_at(Box::new(3i32), "c");
    cbt.cbt_remove_all();
    assert_eq!(cbt.cbt_size(), 0);
    assert!(cbt.cbt_first().is_none());
    assert_eq!(cbt.cbt_overhead(), 72);
    // Reusable
    cbt.cbt_put_at(Box::new(42i32), "x");
    cbt.cbt_put_at(Box::new(43i32), "y");
    assert_eq!(cbt.cbt_size(), 2);
}

#[test]
fn test_cbt_remove_all_with_callback() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "a");
    cbt.cbt_put_at(Box::new(2i32), "b");
    cbt.cbt_put_at(Box::new(3i32), "c");

    let mut called: Vec<(String, i32)> = Vec::new();
    cbt.cbt_remove_all_with(|data, key| {
        let v = *data.downcast::<i32>().unwrap();
        called.push((key.to_string(), v));
    });
    called.sort();
    assert_eq!(
        called,
        vec![
            ("a".to_string(), 1),
            ("b".to_string(), 2),
            ("c".to_string(), 3),
        ]
    );
    assert_eq!(cbt.cbt_size(), 0);
}

#[test]
fn test_cbt_put_with_creates_or_updates() {
    let mut cbt = Cbt::cbt_new();
    let leaf = cbt.cbt_put_with(|_| Box::new(7i32), "key");
    assert_eq!(leaf.key, "key");
    let v = cbt
        .cbt_get_at("key")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(7));

    // Update existing.
    cbt.cbt_put_with(|_| Box::new(99i32), "key");
    let v = cbt
        .cbt_get_at("key")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(99));
    assert_eq!(cbt.cbt_size(), 1);
}

#[test]
fn test_cbt_insert_returns_is_new_flag() {
    let mut cbt = Cbt::cbt_new();
    let (is_new1, leaf1) = cbt.cbt_insert("hello");
    assert!(is_new1);
    assert_eq!(leaf1.key, "hello");
    let (is_new2, leaf2) = cbt.cbt_insert("hello");
    assert!(!is_new2);
    assert_eq!(leaf2.key, "hello");
    assert_eq!(cbt.cbt_size(), 1);
}

#[test]
fn test_cbt_put_updates_data_at_leaf() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "k");
    let mut leaf = cbt.cbt_first().unwrap();
    cbt.cbt_put(&mut leaf, Box::new(99i32));
    let v = cbt
        .cbt_get_at("k")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(99));
}

#[test]
fn test_cbt_at_returns_some_or_none() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "present");
    let leaf = cbt.cbt_at("present").unwrap();
    assert_eq!(leaf.key, "present");
    assert!(cbt.cbt_at("absent").is_none());
}

#[test]
fn test_cbt_new_u_fixed_length() {
    let mut cbt = Cbt::cbt_new_u(4);
    // Note: Rust public API uses &str. Ensure we still get the BTreeMap
    // semantics expected by the tests when fixed-length keys are 4 chars.
    cbt.cbt_put_at(Box::new(100i32), "abcd");
    cbt.cbt_put_at(Box::new(200i32), "abce");
    cbt.cbt_put_at(Box::new(300i32), "0000");
    assert_eq!(cbt.cbt_size(), 3);
    let v = cbt
        .cbt_get_at("abcd")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(100));
    let v = cbt
        .cbt_get_at("abce")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(200));
    let v = cbt
        .cbt_get_at("0000")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(300));
}

#[test]
fn test_cbt_new_enc_basic() {
    let mut cbt = Cbt::cbt_new_enc();
    // The Rust translation goes through the &str API; ensure normal usage
    // still works for storage and lookup.
    cbt.cbt_put_at(Box::new(1000i32), "abc");
    cbt.cbt_put_at(Box::new(2000i32), "abd");
    cbt.cbt_put_at(Box::new(3000i32), "abcd");
    assert_eq!(cbt.cbt_size(), 3);
    let v = cbt
        .cbt_get_at("abc")
        .and_then(|b| b.downcast::<i32>().ok())
        .map(|b| *b);
    assert_eq!(v, Some(1000));
}

#[test]
fn test_cbt_size_count_consistency() {
    let mut cbt = Cbt::cbt_new();
    let n = 10;
    for i in 0..n {
        let key = format!("key-{:02}", i);
        cbt.cbt_put_at(Box::new(i as i32), &key);
    }
    assert_eq!(cbt.cbt_size(), n as i32);
    // Re-put existing key shouldn't change size.
    cbt.cbt_put_at(Box::new(0i32), "key-00");
    assert_eq!(cbt.cbt_size(), n as i32);
}

#[test]
fn test_cbt_delete_consumes() {
    let mut cbt = Cbt::cbt_new();
    cbt.cbt_put_at(Box::new(1i32), "hello");
    cbt.cbt_put_at(Box::new(2i32), "world");
    // cbt_delete frees underlying storage; just ensure no panic.
    cbt.cbt_delete();
}

#[test]
fn test_cbt_overhead_empty_and_after_inserts() {
    let mut cbt = Cbt::cbt_new();
    assert_eq!(cbt.cbt_overhead(), 72);
    cbt.cbt_put_at(Box::new(0i32), "k1");
    // 1 leaf + 0 internal: 72 + 40 + 0 = 112
    assert_eq!(cbt.cbt_overhead(), 112);
    cbt.cbt_put_at(Box::new(0i32), "k2");
    // 2 leaves + 1 internal: 72 + 80 + 24 = 176
    assert_eq!(cbt.cbt_overhead(), 176);
}

fn main() {}
