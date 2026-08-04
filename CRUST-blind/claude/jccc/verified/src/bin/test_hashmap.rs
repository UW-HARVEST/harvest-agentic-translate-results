use jccc::hashmap::{
    create_hashmap, destroy_hashmap, double_cap, equal_key, fnva1, hm_get, hm_set,
    test_hash_init, test_hash_init_and_store, test_hash_set_and_double_get, test_hash_set_and_get,
};

#[test]
fn test_fnva1_known_values() {
    // These match the C-reference values produced by running fnva1 with the
    // C implementation.
    assert_eq!(fnva1(""), 16777619);
    assert_eq!(fnva1("test"), 1515068489);
    assert_eq!(fnva1("hello"), 3614833289);
    assert_eq!(fnva1("a"), 4004178234);
    assert_eq!(fnva1("abc"), 3498811271);
}

#[test]
fn test_equal_key_basic() {
    assert!(equal_key("test", "test"));
    assert!(equal_key("", ""));
    assert!(!equal_key("test", "tes"));
    assert!(!equal_key("a", "b"));
}

#[test]
fn test_create_hashmap_init() {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    assert_eq!(h.buckets.len(), 100);
    // All buckets should be None.
    for b in h.buckets.iter() {
        assert!(b.is_none());
    }
}

#[test]
fn test_hm_set_and_get() {
    let mut h = create_hashmap(100);
    let r = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_eq!(r, 0);
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);

    let got = hm_get(&h, "test");
    assert!(got.is_some());
    let node = got.unwrap();
    assert_eq!(node.key, "test");
    let v = node.value.downcast_ref::<String>().unwrap();
    assert_eq!(v, "jake");
}

#[test]
fn test_hm_set_collision_returns_minus_one() {
    // Same key, second set should return -1 (linear probing not implemented).
    let mut h = create_hashmap(100);
    let r1 = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_eq!(r1, 0);
    let r2 = hm_set(&mut h, "test", Box::new(String::from("bob")));
    assert_eq!(r2, -1);
    // The original value should still be there.
    let got = hm_get(&h, "test").unwrap();
    let v = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(v, "jake");
}

#[test]
fn test_hm_get_missing_key() {
    let h = create_hashmap(100);
    let got = hm_get(&h, "missing");
    assert!(got.is_none());
}

#[test]
fn test_hm_get_wrong_key_collision() {
    // Two different keys that hash to the same bucket cause hm_get to return
    // None for the second when the bucket is occupied by a different key.
    let mut h = create_hashmap(2);
    // Put one entry.
    let r1 = hm_set(&mut h, "test", Box::new(123_i32));
    assert_eq!(r1, 0);
    // Asking for a different key that may or may not collide -- check absent.
    let got = hm_get(&h, "definitely_not_there");
    assert!(got.is_none());
}

#[test]
fn test_double_cap() {
    let mut h = create_hashmap(100);
    let r = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_eq!(r, 0);
    double_cap(&mut h);
    assert_eq!(h.cap, 200);
    // After double_cap, hm_get should still work because the bucket index
    // (using new cap) gives the same position when previous index was < new cap.
    let got = hm_get(&h, "test").unwrap();
    let v = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(v, "jake");
}

#[test]
fn test_destroy_hashmap_clears_buckets() {
    let mut h = create_hashmap(50);
    hm_set(&mut h, "k", Box::new(1_i32));
    destroy_hashmap(&mut h);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 0);
    assert_eq!(h.buckets.len(), 0);
}

#[test]
fn test_internal_test_helpers() {
    assert_eq!(test_hash_init(), 0);
    assert_eq!(test_hash_init_and_store(), 0);
    assert_eq!(test_hash_set_and_get(), 0);
    assert_eq!(test_hash_set_and_double_get(), 0);
}

fn main() {}
