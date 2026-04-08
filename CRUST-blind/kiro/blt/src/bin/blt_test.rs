use blt::blt::*;

#[test]
fn test_blt_new_empty() {
    let blt = Blt::blt_new();
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
    assert!(blt.blt_first().is_none());
    assert!(blt.blt_last().is_none());
    assert!(blt.blt_get("x").is_none());
}

#[test]
fn test_blt_put_get() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(42i32));
    assert!(!blt.blt_empty());
    assert_eq!(blt.blt_size(), 1);
    assert!(blt.blt_get("hello").is_some());
    assert_eq!(blt.blt_get("hello").unwrap().key, "hello");
    assert!(blt.blt_get("world").is_none());
}

#[test]
fn test_blt_sorted_order() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    blt.blt_put("world", Box::new(()));
    blt.blt_put("abc", Box::new(()));
    blt.blt_put("xyz", Box::new(()));
    assert_eq!(blt.blt_size(), 4);

    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["abc", "hello", "world", "xyz"]);
}

#[test]
fn test_blt_first_last() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    blt.blt_put("world", Box::new(()));
    blt.blt_put("abc", Box::new(()));
    blt.blt_put("xyz", Box::new(()));
    assert_eq!(blt.blt_first().unwrap().key, "abc");
    assert_eq!(blt.blt_last().unwrap().key, "xyz");
}

#[test]
fn test_blt_next_prev() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    blt.blt_put("world", Box::new(()));
    blt.blt_put("abc", Box::new(()));
    blt.blt_put("xyz", Box::new(()));

    // Forward traversal
    let mut keys = Vec::new();
    let mut it = blt.blt_first();
    while let Some(ref cur) = it {
        keys.push(cur.key.clone());
        it = blt.blt_next(cur);
    }
    assert_eq!(keys, vec!["abc", "hello", "world", "xyz"]);

    // Backward traversal
    let mut keys = Vec::new();
    let mut it = blt.blt_last();
    while let Some(ref cur) = it {
        keys.push(cur.key.clone());
        it = blt.blt_prev(cur);
    }
    assert_eq!(keys, vec!["xyz", "world", "hello", "abc"]);
}

#[test]
fn test_blt_delete() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    blt.blt_put("world", Box::new(()));
    blt.blt_put("abc", Box::new(()));
    blt.blt_put("xyz", Box::new(()));

    assert_eq!(blt.blt_delete("abc"), 1);
    assert_eq!(blt.blt_delete("abc"), 0);
    assert_eq!(blt.blt_size(), 3);
    assert!(blt.blt_get("abc").is_none());
}

#[test]
fn test_blt_delete_empty() {
    let mut blt = Blt::blt_new();
    assert_eq!(blt.blt_delete("x"), 0);
}

#[test]
fn test_blt_delete_single() {
    let mut blt = Blt::blt_new();
    blt.blt_put("only", Box::new(()));
    assert_eq!(blt.blt_delete("only"), 1);
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
}

#[test]
fn test_blt_put_if_absent() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    // Already present → returns 1
    assert_eq!(blt.blt_put_if_absent("hello", Box::new(())), 1);
    // New key → returns 0
    assert_eq!(blt.blt_put_if_absent("new", Box::new(())), 0);
    assert_eq!(blt.blt_size(), 2);
}

#[test]
fn test_blt_setp() {
    let mut blt = Blt::blt_new();
    let (it, is_new) = blt.blt_setp("hello");
    assert!(is_new);
    assert_eq!(it.key, "hello");

    let (it2, is_new2) = blt.blt_setp("hello");
    assert!(!is_new2);
    assert_eq!(it2.key, "hello");
}

#[test]
fn test_blt_set() {
    let mut blt = Blt::blt_new();
    let it = blt.blt_set("hello");
    assert_eq!(it.key, "hello");
    assert_eq!(blt.blt_size(), 1);
    // set again should not duplicate
    blt.blt_set("hello");
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_allprefixed() {
    let mut blt = Blt::blt_new();
    for k in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        blt.blt_put(k, Box::new(()));
    }

    let mut collect = |prefix: &str| -> Vec<String> {
        let mut v = Vec::new();
        blt.blt_allprefixed(prefix, |it| { v.push(it.key.clone()); 1 });
        v
    };

    assert_eq!(collect("b"), vec!["b", "ben", "blink", "bliss", "blt", "blynn"]);
    assert_eq!(collect("bl"), vec!["blink", "bliss", "blt", "blynn"]);
    assert_eq!(collect("bli"), vec!["blink", "bliss"]);
    assert_eq!(collect("a"), vec!["a", "aardvark"]);
    assert_eq!(collect("aa"), vec!["aardvark"]);
    assert_eq!(collect("c"), Vec::<String>::new());
}

#[test]
fn test_blt_allprefixed_empty() {
    let blt = Blt::blt_new();
    let mut v = Vec::new();
    let r = blt.blt_allprefixed("x", |it| { v.push(it.key.clone()); 1 });
    assert_eq!(r, 1);
    assert!(v.is_empty());
}

#[test]
fn test_blt_allprefixed_early_stop() {
    let mut blt = Blt::blt_new();
    for k in &["a", "b", "c", "d"] {
        blt.blt_put(k, Box::new(()));
    }
    let mut v = Vec::new();
    let r = blt.blt_allprefixed("", |it| {
        v.push(it.key.clone());
        if it.key == "b" { 0 } else { 1 }
    });
    assert_eq!(r, 0);
    assert_eq!(v, vec!["a", "b"]);
}

#[test]
fn test_blt_ceil_floor() {
    let mut blt = Blt::blt_new();
    for k in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        blt.blt_put(k, Box::new(()));
    }

    assert_eq!(blt.blt_ceil("blink").unwrap().key, "blink");
    assert_eq!(blt.blt_ceil("blink182").unwrap().key, "bliss");
    assert_eq!(blt.blt_floor("blink").unwrap().key, "blink");
    assert_eq!(blt.blt_floor("blink182").unwrap().key, "blink");
    assert!(blt.blt_ceil("z").is_none());
    assert!(blt.blt_floor("").is_none());
    assert_eq!(blt.blt_ceil("").unwrap().key, "a");
    assert_eq!(blt.blt_floor("z").unwrap().key, "blynn");
}

#[test]
fn test_blt_ceil_floor_empty() {
    let blt = Blt::blt_new();
    assert!(blt.blt_ceil("x").is_none());
    assert!(blt.blt_floor("x").is_none());
}

#[test]
fn test_blt_overhead() {
    let blt = Blt::blt_new();
    assert_eq!(blt.blt_overhead(), std::mem::size_of::<Blt>());

    let mut blt2 = Blt::blt_new();
    blt2.blt_put("hello", Box::new(()));
    // Single leaf, no internal nodes
    assert_eq!(blt2.blt_overhead(), std::mem::size_of::<Blt>());
}

#[test]
fn test_blt_overhead_multiple() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(()));
    blt.blt_put("world", Box::new(()));
    blt.blt_put("abc", Box::new(()));
    blt.blt_put("xyz", Box::new(()));
    // 4 items = 3 internal nodes, each adds 2 * sizeof(BltNode)
    let expected = std::mem::size_of::<Blt>() + 3 * 2 * std::mem::size_of::<BltNode>();
    assert_eq!(blt.blt_overhead(), expected);
}

#[test]
fn test_blt_traverse_abcd() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(()));
    blt.blt_put("c", Box::new(()));
    blt.blt_put("b", Box::new(()));
    blt.blt_put("d", Box::new(()));
    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_blt_traverse_single() {
    let mut blt = Blt::blt_new();
    blt.blt_put("only", Box::new(()));
    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["only"]);
    assert_eq!(blt.blt_first().unwrap().key, "only");
    assert_eq!(blt.blt_last().unwrap().key, "only");
    assert!(blt.blt_next(&blt.blt_first().unwrap()).is_none());
    assert!(blt.blt_prev(&blt.blt_last().unwrap()).is_none());
}

#[test]
fn test_blt_traverse_two_strings() {
    let mut blt = Blt::blt_new();
    blt.blt_put("two", Box::new(()));
    blt.blt_put("strings", Box::new(()));
    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["strings", "two"]);
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

#[test]
fn test_blt_duplicate_put() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(1i32));
    blt.blt_put("hello", Box::new(2i32));
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_many_keys_sorted() {
    let mut blt = Blt::blt_new();
    let words = vec![
        "the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog",
    ];
    for w in &words {
        blt.blt_put(w, Box::new(()));
    }
    let mut keys = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    let mut expected: Vec<String> = words.iter().map(|s| s.to_string()).collect();
    expected.sort();
    expected.dedup();
    assert_eq!(keys, expected);
}

fn main() {}
