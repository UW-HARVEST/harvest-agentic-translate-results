use blt::blt::Blt;

#[test]
fn test_empty_tree() {
    let t = Blt::blt_new();
    assert!(t.blt_empty());
    assert_eq!(t.blt_size(), 0);
    assert_eq!(t.blt_overhead(), 24);
    assert!(t.blt_get("hello").is_none());
    assert!(t.blt_first().is_none());
    assert!(t.blt_last().is_none());
}

#[test]
fn test_clear_makes_empty() {
    let mut t = Blt::blt_new();
    t.blt_put("a", Box::new(1u64));
    assert!(!t.blt_empty());
    t.blt_clear();
    assert!(t.blt_empty());
    assert_eq!(t.blt_size(), 0);
    assert!(t.blt_get("a").is_none());
}

#[test]
fn test_put_get() {
    let mut t = Blt::blt_new();
    t.blt_put("hello", Box::new(1u64));
    t.blt_put("world", Box::new(2u64));
    t.blt_put("foo", Box::new(3u64));
    t.blt_put("bar", Box::new(4u64));
    t.blt_put("baz", Box::new(5u64));

    assert_eq!(t.blt_size(), 5);
    assert_eq!(t.blt_overhead(), 152);
    assert!(!t.blt_empty());

    assert!(t.blt_get("hello").is_some());
    assert_eq!(t.blt_get("hello").unwrap().key, "hello");
    assert!(t.blt_get("missing").is_none());

    assert_eq!(t.blt_first().unwrap().key, "bar");
    assert_eq!(t.blt_last().unwrap().key, "world");
}

#[test]
fn test_iteration() {
    let mut t = Blt::blt_new();
    for s in &["hello", "world", "foo", "bar", "baz"] {
        t.blt_put(s, Box::new(0u8));
    }

    let mut keys: Vec<String> = Vec::new();
    let mut it = t.blt_first();
    while let Some(node) = it {
        keys.push(node.key.clone());
        it = t.blt_next(&node);
    }
    assert_eq!(keys, vec!["bar", "baz", "foo", "hello", "world"]);

    // backward
    let mut keys: Vec<String> = Vec::new();
    let mut it = t.blt_last();
    while let Some(node) = it {
        keys.push(node.key.clone());
        it = t.blt_prev(&node);
    }
    assert_eq!(keys, vec!["world", "hello", "foo", "baz", "bar"]);
}

#[test]
fn test_blt_forall_in_order() {
    let mut t = Blt::blt_new();
    for s in &["red", "string", "blue", "string"] {
        t.blt_put(s, Box::new(0u8));
    }
    let mut keys: Vec<String> = Vec::new();
    t.blt_forall(|it| keys.push(it.key.clone()));
    assert_eq!(keys, vec!["blue", "red", "string"]);
}

#[test]
fn test_ceil_and_floor() {
    let mut t = Blt::blt_new();
    for s in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        t.blt_put(s, Box::new(0u8));
    }

    assert_eq!(t.blt_ceil("blink").unwrap().key, "blink");
    assert_eq!(t.blt_ceil("blink182").unwrap().key, "bliss");
    assert_eq!(t.blt_floor("blink").unwrap().key, "blink");
    assert_eq!(t.blt_floor("blink182").unwrap().key, "blink");

    // Above largest -> ceil returns None.
    assert!(t.blt_ceil("z").is_none());
    // Below smallest -> floor returns None.
    assert!(t.blt_floor("0").is_none());
}

#[test]
fn test_allprefixed() {
    let mut t = Blt::blt_new();
    for s in &["a", "aardvark", "b", "ben", "blink", "bliss", "blt", "blynn"] {
        t.blt_put(s, Box::new(0u8));
    }

    fn collect(t: &Blt, prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        t.blt_allprefixed(prefix, |it| {
            out.push(it.key.clone());
            1
        });
        out
    }

    assert_eq!(collect(&t, "b"), vec!["b", "ben", "blink", "bliss", "blt", "blynn"]);
    assert_eq!(collect(&t, "bl"), vec!["blink", "bliss", "blt", "blynn"]);
    assert_eq!(collect(&t, "bli"), vec!["blink", "bliss"]);
    assert_eq!(collect(&t, "a"), vec!["a", "aardvark"]);
    assert_eq!(collect(&t, "aa"), vec!["aardvark"]);
    let empty: Vec<String> = Vec::new();
    assert_eq!(collect(&t, "c"), empty);
}

#[test]
fn test_delete() {
    let mut t = Blt::blt_new();
    for s in &["hello", "world", "foo", "bar", "baz"] {
        t.blt_put(s, Box::new(0u8));
    }

    assert_eq!(t.blt_delete("hello"), 1);
    assert_eq!(t.blt_delete("missing"), 0);
    assert_eq!(t.blt_size(), 4);
    assert!(t.blt_get("hello").is_none());
}

#[test]
fn test_put_if_absent() {
    let mut t = Blt::blt_new();
    t.blt_put("world", Box::new(2u64));

    assert_eq!(t.blt_put_if_absent("world", Box::new(99u64)), 1);
    assert_eq!(t.blt_put_if_absent("newkey", Box::new(100u64)), 0);
    assert_eq!(t.blt_size(), 2);
    assert!(t.blt_get("world").is_some());
    assert!(t.blt_get("newkey").is_some());
}

#[test]
fn test_delete_to_empty() {
    let mut t = Blt::blt_new();
    t.blt_put("only", Box::new(0u8));
    assert!(!t.blt_empty());
    assert_eq!(t.blt_delete("only"), 1);
    assert!(t.blt_empty());
    assert_eq!(t.blt_size(), 0);
    assert_eq!(t.blt_overhead(), 24);
}

#[test]
fn test_overhead_growth() {
    let mut t = Blt::blt_new();
    assert_eq!(t.blt_overhead(), 24);
    t.blt_put("x", Box::new(0u8));
    assert_eq!(t.blt_overhead(), 24);
    t.blt_put("y", Box::new(0u8));
    assert_eq!(t.blt_overhead(), 56);
    t.blt_put("z", Box::new(0u8));
    assert_eq!(t.blt_overhead(), 88);
}

#[test]
fn test_first_last_single() {
    let mut t = Blt::blt_new();
    t.blt_put("only", Box::new(0u8));
    assert_eq!(t.blt_first().unwrap().key, "only");
    assert_eq!(t.blt_last().unwrap().key, "only");
    let it = t.blt_first().unwrap();
    assert!(t.blt_next(&it).is_none());
    assert!(t.blt_prev(&it).is_none());
}

#[test]
fn test_set_p_returns_new_flag() {
    let mut t = Blt::blt_new();
    let (it, new) = t.blt_setp("alpha");
    assert_eq!(it.key, "alpha");
    assert!(new);
    let (it2, new2) = t.blt_setp("alpha");
    assert_eq!(it2.key, "alpha");
    assert!(!new2);
}

fn main() {}
