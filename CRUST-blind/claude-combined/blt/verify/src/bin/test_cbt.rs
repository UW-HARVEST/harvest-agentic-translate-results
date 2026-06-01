use blt::cbt::Cbt;
use std::any::Any;
use std::rc::Rc;

fn unwrap_u64(b: Box<dyn Any>) -> u64 {
    // Data is wrapped: Box<Rc<dyn Any>> where the inner Any is u64.
    let rc: Rc<dyn Any> = *b.downcast::<Rc<dyn Any>>().expect("wrapped Rc");
    *rc.downcast_ref::<u64>().expect("u64")
}

fn unwrap_u64_ref(b: &Box<dyn Any>) -> u64 {
    let rc = b.downcast_ref::<Rc<dyn Any>>().expect("wrapped Rc");
    *rc.downcast_ref::<u64>().expect("u64")
}

#[test]
fn test_empty_tree() {
    let c = Cbt::cbt_new();
    assert_eq!(c.cbt_size(), 0);
    assert_eq!(c.cbt_overhead(), 72);
    assert!(c.cbt_first().is_none());
    assert!(c.cbt_last().is_none());
    assert!(!c.cbt_has("nope"));
    assert!(c.cbt_at("nope").is_none());
    assert!(c.cbt_get_at("nope").is_none());
}

#[test]
fn test_put_and_size() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "hello");
    c.cbt_put_at(Box::new(2u64), "world");
    c.cbt_put_at(Box::new(3u64), "foo");
    c.cbt_put_at(Box::new(4u64), "bar");
    c.cbt_put_at(Box::new(5u64), "baz");

    assert_eq!(c.cbt_size(), 5);
    assert_eq!(c.cbt_overhead(), 368);

    let g = c.cbt_get_at("hello").expect("hello present");
    assert_eq!(unwrap_u64(g), 1);
    let g = c.cbt_get_at("world").expect("world present");
    assert_eq!(unwrap_u64(g), 2);
    assert!(c.cbt_get_at("missing").is_none());
}

#[test]
fn test_iteration_order() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "hello");
    c.cbt_put_at(Box::new(2u64), "world");
    c.cbt_put_at(Box::new(3u64), "foo");
    c.cbt_put_at(Box::new(4u64), "bar");
    c.cbt_put_at(Box::new(5u64), "baz");

    let mut keys = Vec::new();
    let mut data = Vec::new();
    let mut it = c.cbt_first();
    while let Some(node) = it {
        keys.push(node.key.clone());
        data.push(unwrap_u64_ref(&node.data));
        it = Cbt::cbt_next(&node);
    }
    assert_eq!(keys, vec!["bar", "baz", "foo", "hello", "world"]);
    assert_eq!(data, vec![4u64, 5u64, 3u64, 1u64, 2u64]);
}

#[test]
fn test_first_last_match() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(10u64), "mango");
    c.cbt_put_at(Box::new(20u64), "apple");
    c.cbt_put_at(Box::new(30u64), "zebra");
    let f = c.cbt_first().unwrap();
    assert_eq!(f.key, "apple");
    assert_eq!(unwrap_u64_ref(&f.data), 20);
    let l = c.cbt_last().unwrap();
    assert_eq!(l.key, "zebra");
    assert_eq!(unwrap_u64_ref(&l.data), 30);
}

#[test]
fn test_has_and_at() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "a");
    c.cbt_put_at(Box::new(2u64), "b");
    assert!(c.cbt_has("a"));
    assert!(c.cbt_has("b"));
    assert!(!c.cbt_has("c"));
    let a = c.cbt_at("a").unwrap();
    assert_eq!(a.key, "a");
    assert_eq!(unwrap_u64_ref(&a.data), 1);
    assert!(c.cbt_at("c").is_none());
}

#[test]
fn test_remove() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "hello");
    c.cbt_put_at(Box::new(2u64), "world");
    c.cbt_put_at(Box::new(3u64), "foo");

    let removed = c.cbt_remove("hello").unwrap();
    assert_eq!(unwrap_u64(removed), 1);
    assert_eq!(c.cbt_size(), 2);
    assert!(!c.cbt_has("hello"));
    assert!(c.cbt_has("world"));
    assert!(c.cbt_has("foo"));

    assert!(c.cbt_remove("nonexistent").is_none());
    assert_eq!(c.cbt_size(), 2);
}

#[test]
fn test_remove_all() {
    let mut c = Cbt::cbt_new();
    for (i, s) in ["a", "b", "c"].iter().enumerate() {
        c.cbt_put_at(Box::new(i as u64), s);
    }
    assert_eq!(c.cbt_size(), 3);
    c.cbt_remove_all();
    assert_eq!(c.cbt_size(), 0);
    assert!(!c.cbt_has("a"));
    assert!(c.cbt_first().is_none());
}

#[test]
fn test_remove_all_with_callback() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "a");
    c.cbt_put_at(Box::new(2u64), "b");
    let mut keys: Vec<String> = Vec::new();
    let mut data: Vec<u64> = Vec::new();
    c.cbt_remove_all_with(|d, k| {
        // d is wrapped Rc<dyn Any>
        let rc: Rc<dyn Any> = *d.downcast::<Rc<dyn Any>>().unwrap();
        data.push(*rc.downcast_ref::<u64>().unwrap());
        keys.push(k.to_string());
    });
    assert_eq!(c.cbt_size(), 0);
    assert_eq!(keys, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(data, vec![1u64, 2u64]);
}

#[test]
fn test_overhead_growth() {
    let mut c = Cbt::cbt_new();
    assert_eq!(c.cbt_overhead(), 72);
    c.cbt_put_at(Box::new(0u64), "hello");
    assert_eq!(c.cbt_overhead(), 112);
    c.cbt_put_at(Box::new(0u64), "world");
    assert_eq!(c.cbt_overhead(), 176);
    c.cbt_put_at(Box::new(0u64), "foo");
    assert_eq!(c.cbt_overhead(), 240);
}

#[test]
fn test_put_overwrites() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "key");
    c.cbt_put_at(Box::new(2u64), "key");
    assert_eq!(c.cbt_size(), 1);
    let g = c.cbt_get_at("key").unwrap();
    assert_eq!(unwrap_u64(g), 2);
}

#[test]
fn test_forall() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(1u64), "alpha");
    c.cbt_put_at(Box::new(2u64), "beta");
    c.cbt_put_at(Box::new(3u64), "gamma");

    let mut keys = Vec::new();
    c.cbt_forall(|leaf| keys.push(leaf.key.clone()));
    assert_eq!(keys, vec!["alpha", "beta", "gamma"]);

    let mut keys2 = Vec::new();
    c.cbt_forall_at(|_data, key| keys2.push(key.to_string()));
    assert_eq!(keys2, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn test_cbt_insert_returns_new_flag() {
    let mut c = Cbt::cbt_new();
    let (new, leaf) = c.cbt_insert("first");
    assert!(new);
    assert_eq!(leaf.key, "first");
    let (new2, leaf2) = c.cbt_insert("first");
    assert!(!new2);
    assert_eq!(leaf2.key, "first");
    assert_eq!(c.cbt_size(), 1);
}

#[test]
fn test_put_with_callback() {
    let mut c = Cbt::cbt_new();
    let leaf = c.cbt_put_with(|_| Box::new(42u64) as Box<dyn Any>, "foo");
    assert_eq!(leaf.key, "foo");
    assert_eq!(c.cbt_size(), 1);
    let g = c.cbt_get_at("foo").unwrap();
    assert_eq!(unwrap_u64(g), 42);
}

#[test]
fn test_new_u_constructs() {
    let c = Cbt::cbt_new_u(8);
    assert_eq!(c.cbt_size(), 0);
    assert_eq!(c.len, 8);
}

#[test]
fn test_new_enc_constructs() {
    let c = Cbt::cbt_new_enc();
    assert_eq!(c.cbt_size(), 0);
    assert_eq!(c.len, -1);
}

#[test]
fn test_cbt_key_returns_key() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(0u64), "thekey");
    let leaf = c.cbt_first().unwrap();
    assert_eq!(c.cbt_key(&leaf), "thekey");
}

#[test]
fn test_get_returns_data() {
    let mut c = Cbt::cbt_new();
    c.cbt_put_at(Box::new(99u64), "k");
    let leaf = c.cbt_at("k").unwrap();
    let d = c.cbt_get(&leaf).unwrap();
    assert_eq!(unwrap_u64(d), 99);
}

fn main() {}
