use blt::blt::Blt;

#[test]
fn test_empty_tree() {
    let mut blt = Blt::blt_new();
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
    assert!(blt.blt_get("x").is_none());
    assert!(blt.blt_first().is_none());
    assert!(blt.blt_last().is_none());
    assert_eq!(blt.blt_delete("x"), 0);
}

#[test]
fn test_single_element() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(42i64));
    assert!(!blt.blt_empty());
    assert_eq!(blt.blt_size(), 1);
    let it = blt.blt_get("hello").unwrap();
    assert_eq!(it.key, "hello");
    assert!(blt.blt_get("world").is_none());
    assert_eq!(blt.blt_first().unwrap().key, "hello");
    assert_eq!(blt.blt_last().unwrap().key, "hello");
    let first = blt.blt_first().unwrap();
    assert!(blt.blt_next(&first).is_none());
    assert!(blt.blt_prev(&first).is_none());
}

#[test]
fn test_multiple_elements_ordering() {
    let mut blt = Blt::blt_new();
    blt.blt_put("cherry", Box::new(3i64));
    blt.blt_put("apple", Box::new(1i64));
    blt.blt_put("banana", Box::new(2i64));
    assert_eq!(blt.blt_size(), 3);
    assert_eq!(blt.blt_first().unwrap().key, "apple");
    assert_eq!(blt.blt_last().unwrap().key, "cherry");

    // Forward traversal
    let mut keys = Vec::new();
    let mut it = blt.blt_first();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = blt.blt_next(leaf);
    }
    assert_eq!(keys, vec!["apple", "banana", "cherry"]);

    // Backward traversal
    let mut keys = Vec::new();
    let mut it = blt.blt_last();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = blt.blt_prev(leaf);
    }
    assert_eq!(keys, vec!["cherry", "banana", "apple"]);
}

#[test]
fn test_ceil_floor() {
    let mut blt = Blt::blt_new();
    blt.blt_put("cherry", Box::new(3i64));
    blt.blt_put("apple", Box::new(1i64));
    blt.blt_put("banana", Box::new(2i64));

    assert_eq!(blt.blt_ceil("banana").unwrap().key, "banana");
    assert_eq!(blt.blt_floor("banana").unwrap().key, "banana");
    assert_eq!(blt.blt_ceil("blueberry").unwrap().key, "cherry");
    assert_eq!(blt.blt_floor("blueberry").unwrap().key, "banana");
    assert_eq!(blt.blt_ceil("a").unwrap().key, "apple");
    assert_eq!(blt.blt_floor("d").unwrap().key, "cherry");
    assert!(blt.blt_ceil("z").is_none());
    assert!(blt.blt_floor("a").is_none());
}

#[test]
fn test_delete() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(()));
    blt.blt_put("b", Box::new(()));
    blt.blt_put("c", Box::new(()));
    assert_eq!(blt.blt_delete("b"), 1);
    assert_eq!(blt.blt_size(), 2);
    assert_eq!(blt.blt_delete("b"), 0);

    let mut keys = Vec::new();
    let mut it = blt.blt_first();
    while let Some(ref leaf) = it {
        keys.push(leaf.key.clone());
        it = blt.blt_next(leaf);
    }
    assert_eq!(keys, vec!["a", "c"]);
}

#[test]
fn test_put_if_absent() {
    let mut blt = Blt::blt_new();
    assert_eq!(blt.blt_put_if_absent("x", Box::new(1i64)), 0);
    assert_eq!(blt.blt_put_if_absent("x", Box::new(2i64)), 1);
}

#[test]
fn test_setp() {
    let mut blt = Blt::blt_new();
    let (_, is_new) = blt.blt_setp("foo");
    assert!(is_new);
    let (_, is_new) = blt.blt_setp("foo");
    assert!(!is_new);
}

#[test]
fn test_allprefixed() {
    let mut blt = Blt::blt_new();
    for k in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        blt.blt_put(k, Box::new(()));
    }

    let mut keys = Vec::new();
    blt.blt_allprefixed("bl", |it| { keys.push(it.key.clone()); 1 });
    assert_eq!(keys, vec!["blink", "bliss", "blt", "blynn"]);

    let mut keys = Vec::new();
    blt.blt_allprefixed("c", |it| { keys.push(it.key.clone()); 1 });
    assert!(keys.is_empty());

    let mut keys = Vec::new();
    blt.blt_allprefixed("", |it| { keys.push(it.key.clone()); 1 });
    assert_eq!(keys, vec!["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"]);

    // Test early stop
    let mut keys = Vec::new();
    let r = blt.blt_allprefixed("bl", |it| { keys.push(it.key.clone()); 0 });
    assert_eq!(keys, vec!["blink"]);
    assert_eq!(r, 0);
}

#[test]
fn test_delete_all_elements() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(()));
    blt.blt_put("b", Box::new(()));
    blt.blt_delete("a");
    blt.blt_delete("b");
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
}

#[test]
fn test_forall() {
    let mut blt = Blt::blt_new();
    blt.blt_put("cherry", Box::new(()));
    blt.blt_put("apple", Box::new(()));
    blt.blt_put("banana", Box::new(()));
    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["apple", "banana", "cherry"]);
}

#[test]
fn test_set() {
    let mut blt = Blt::blt_new();
    let it = blt.blt_set("hello");
    assert_eq!(it.key, "hello");
    assert_eq!(blt.blt_size(), 1);
    // set again should not create duplicate
    let it2 = blt.blt_set("hello");
    assert_eq!(it2.key, "hello");
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_clear() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(()));
    blt.blt_put("b", Box::new(()));
    blt.blt_clear();
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
}

fn main() {}
