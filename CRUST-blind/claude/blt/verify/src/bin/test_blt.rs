use blt::blt::Blt;

#[test]
fn test_blt_new_empty_state() {
    let blt = Blt::blt_new();
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
    assert_eq!(blt.blt_overhead(), 24);
    // Empty tree returns None for first/last/ceil/floor/get
    assert!(blt.blt_first().is_none());
    assert!(blt.blt_last().is_none());
    assert!(blt.blt_get("anything").is_none());
    assert!(blt.blt_ceil("anything").is_none());
    assert!(blt.blt_floor("anything").is_none());
}

#[test]
fn test_blt_put_single_key() {
    let mut blt = Blt::blt_new();
    let leaf = blt.blt_put("hello", Box::new(42i32));
    assert_eq!(leaf.key, "hello");
    assert!(!blt.blt_empty());
    assert_eq!(blt.blt_size(), 1);
    assert_eq!(blt.blt_overhead(), 24);

    let got = blt.blt_get("hello");
    assert!(got.is_some());
    assert_eq!(got.unwrap().key, "hello");
    assert!(blt.blt_get("missing").is_none());
}

#[test]
fn test_blt_put_two_keys() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(0i32));
    blt.blt_put("world", Box::new(0i32));
    assert_eq!(blt.blt_size(), 2);
    // 2 leaves -> 1 internal pair -> overhead = 24 + 2*1*16 = 56
    assert_eq!(blt.blt_overhead(), 56);
}

#[test]
fn test_blt_put_duplicate_does_not_grow() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(0i32));
    blt.blt_put("hello", Box::new(1i32));
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_overhead_5_keys() {
    let mut blt = Blt::blt_new();
    for k in &["hello", "world", "foo", "bar", "baz"] {
        blt.blt_put(k, Box::new(0i32));
    }
    assert_eq!(blt.blt_size(), 5);
    // 5 leaves -> 4 internal pairs -> overhead = 24 + 2*4*16 = 152
    assert_eq!(blt.blt_overhead(), 152);
}

#[test]
fn test_blt_setp_is_new_flag() {
    let mut blt = Blt::blt_new();
    let (it, is_new) = blt.blt_setp("first");
    assert_eq!(it.key, "first");
    assert!(is_new);
    let (it2, is_new2) = blt.blt_setp("first");
    assert_eq!(it2.key, "first");
    assert!(!is_new2);
}

#[test]
fn test_blt_set_creates_or_returns() {
    let mut blt = Blt::blt_new();
    let leaf = blt.blt_set("alpha");
    assert_eq!(leaf.key, "alpha");
    assert_eq!(blt.blt_size(), 1);
    let leaf2 = blt.blt_set("alpha");
    assert_eq!(leaf2.key, "alpha");
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_put_if_absent() {
    let mut blt = Blt::blt_new();
    // First insertion -> 0
    assert_eq!(blt.blt_put_if_absent("hello", Box::new(0i32)), 0);
    // Already present -> 1
    assert_eq!(blt.blt_put_if_absent("hello", Box::new(0i32)), 1);
    // Different key -> 0
    assert_eq!(blt.blt_put_if_absent("world", Box::new(0i32)), 0);
    assert_eq!(blt.blt_size(), 2);
}

#[test]
fn test_blt_delete() {
    let mut blt = Blt::blt_new();
    // Empty delete returns 0
    assert_eq!(blt.blt_delete("anything"), 0);

    blt.blt_put("hello", Box::new(0i32));
    blt.blt_put("world", Box::new(0i32));
    blt.blt_put("foo", Box::new(0i32));
    blt.blt_put("bar", Box::new(0i32));
    blt.blt_put("baz", Box::new(0i32));

    assert_eq!(blt.blt_delete("hello"), 1);
    assert_eq!(blt.blt_delete("zzz"), 0);
    assert_eq!(blt.blt_size(), 4);
    assert!(blt.blt_get("hello").is_none());
}

#[test]
fn test_blt_delete_only_key_makes_empty() {
    let mut blt = Blt::blt_new();
    blt.blt_put("only", Box::new(0i32));
    assert!(!blt.blt_empty());
    assert_eq!(blt.blt_delete("only"), 1);
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
}

#[test]
fn test_blt_first_last() {
    let mut blt = Blt::blt_new();
    blt.blt_put("hello", Box::new(0i32));
    blt.blt_put("world", Box::new(0i32));
    blt.blt_put("foo", Box::new(0i32));
    blt.blt_put("bar", Box::new(0i32));
    blt.blt_put("baz", Box::new(0i32));

    let first = blt.blt_first().unwrap();
    let last = blt.blt_last().unwrap();
    // Sorted: bar baz foo hello world
    assert_eq!(first.key, "bar");
    assert_eq!(last.key, "world");
}

#[test]
fn test_blt_next_prev_iteration() {
    let mut blt = Blt::blt_new();
    let words = ["the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog"];
    for w in &words {
        blt.blt_put(w, Box::new(0i32));
    }

    // De-duplicated and sorted set:
    let mut sorted: Vec<&str> = words.to_vec();
    sorted.sort();
    sorted.dedup();

    // Forward via blt_first / blt_next
    let mut acc: Vec<String> = Vec::new();
    let mut cur = blt.blt_first();
    while let Some(it) = cur {
        acc.push(it.key.clone());
        cur = blt.blt_next(&it);
    }
    assert_eq!(acc, sorted);

    // Backward via blt_last / blt_prev
    let mut acc_rev: Vec<String> = Vec::new();
    let mut cur = blt.blt_last();
    while let Some(it) = cur {
        acc_rev.push(it.key.clone());
        cur = blt.blt_prev(&it);
    }
    let expected_rev: Vec<&str> = sorted.iter().rev().copied().collect();
    assert_eq!(acc_rev, expected_rev);
}

#[test]
fn test_blt_next_prev_at_boundaries() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(0i32));
    blt.blt_put("b", Box::new(0i32));

    let first = blt.blt_first().unwrap();
    let nxt = blt.blt_next(&first).unwrap();
    assert_eq!(nxt.key, "b");
    let after = blt.blt_next(&nxt);
    assert!(after.is_none());

    let last = blt.blt_last().unwrap();
    let prv = blt.blt_prev(&last).unwrap();
    assert_eq!(prv.key, "a");
    let before = blt.blt_prev(&prv);
    assert!(before.is_none());
}

#[test]
fn test_blt_ceil_floor() {
    let mut blt = Blt::blt_new();
    for k in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        blt.blt_put(k, Box::new(0i32));
    }

    // Exact match
    assert_eq!(blt.blt_ceil("blink").unwrap().key, "blink");
    assert_eq!(blt.blt_floor("blink").unwrap().key, "blink");
    // Between values
    assert_eq!(blt.blt_ceil("blink182").unwrap().key, "bliss");
    assert_eq!(blt.blt_floor("blink182").unwrap().key, "blink");
    // Past the end
    assert!(blt.blt_ceil("zzz").is_none());
    assert!(blt.blt_floor("0").is_none());
    // Floor of last key
    assert_eq!(blt.blt_floor("zzz").unwrap().key, "blynn");
    // Ceil of first key
    assert_eq!(blt.blt_ceil("0").unwrap().key, "a");
}

#[test]
fn test_blt_allprefixed() {
    let mut blt = Blt::blt_new();
    for k in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        blt.blt_put(k, Box::new(0i32));
    }
    let collect = |prefix: &str| {
        let mut v: Vec<String> = Vec::new();
        let r = blt.blt_allprefixed(prefix, |it| {
            v.push(it.key.clone());
            1
        });
        (r, v)
    };
    let (r, v) = collect("b");
    assert_eq!(r, 1);
    assert_eq!(v, vec!["b", "ben", "blink", "bliss", "blt", "blynn"]);
    let (r, v) = collect("bl");
    assert_eq!(r, 1);
    assert_eq!(v, vec!["blink", "bliss", "blt", "blynn"]);
    let (r, v) = collect("bli");
    assert_eq!(r, 1);
    assert_eq!(v, vec!["blink", "bliss"]);
    let (r, v) = collect("a");
    assert_eq!(r, 1);
    assert_eq!(v, vec!["a", "aardvark"]);
    let (r, v) = collect("aa");
    assert_eq!(r, 1);
    assert_eq!(v, vec!["aardvark"]);
    let (r, v) = collect("c");
    assert_eq!(r, 1);
    assert_eq!(v, Vec::<String>::new());
    // Empty prefix gets all
    let (_r, v) = collect("");
    assert_eq!(
        v,
        vec!["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"]
    );
}

#[test]
fn test_blt_allprefixed_short_circuit() {
    let mut blt = Blt::blt_new();
    for k in &["one", "two", "three", "four"] {
        blt.blt_put(k, Box::new(0i32));
    }
    // Returning anything other than 1 stops iteration with that value.
    let mut count = 0;
    let r = blt.blt_allprefixed("", |_| {
        count += 1;
        if count == 2 {
            42 // stop
        } else {
            1
        }
    });
    assert_eq!(r, 42);
    assert_eq!(count, 2);
}

#[test]
fn test_blt_allprefixed_empty_tree() {
    let blt = Blt::blt_new();
    let mut called = 0;
    let r = blt.blt_allprefixed("", |_| {
        called += 1;
        1
    });
    // Empty tree returns 1 without invoking the callback.
    assert_eq!(r, 1);
    assert_eq!(called, 0);
}

#[test]
fn test_blt_forall_iterates_in_order() {
    let mut blt = Blt::blt_new();
    for k in &["c", "a", "b"] {
        blt.blt_put(k, Box::new(0i32));
    }
    let mut keys: Vec<String> = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_blt_clear_resets_tree() {
    let mut blt = Blt::blt_new();
    blt.blt_put("a", Box::new(0i32));
    blt.blt_put("b", Box::new(0i32));
    assert!(!blt.blt_empty());
    blt.blt_clear();
    assert!(blt.blt_empty());
    assert_eq!(blt.blt_size(), 0);
    assert_eq!(blt.blt_overhead(), 24);
    // Can still use the tree.
    blt.blt_put("c", Box::new(0i32));
    assert_eq!(blt.blt_size(), 1);
}

#[test]
fn test_blt_traverse_with_duplicates_and_spaces() {
    // mirrors test_traverse from C using "  2 spaces   means  empty   strings   are tested"
    let mut blt = Blt::blt_new();
    let parts = vec![
        "", "", "2", "spaces", "", "", "means", "", "empty", "", "", "strings", "", "", "are",
        "tested",
    ];
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    for k in &parts {
        blt.blt_put(k, Box::new(0i32));
    }
    let mut keys: Vec<String> = Vec::new();
    blt.blt_forall(|it| keys.push(it.key.clone()));
    let expected: Vec<String> = sorted.iter().map(|s| s.to_string()).collect();
    assert_eq!(keys, expected);
}

fn main() {}
