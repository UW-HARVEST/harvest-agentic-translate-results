use jccc::hashmap::{
    create_hashmap, destroy_hashmap, double_cap, equal_key, fnva1, hm_get, hm_set,
    test_hash_init, test_hash_init_and_store, test_hash_set_and_double_get,
    test_hash_set_and_get,
};

#[test]
fn test_create_hashmap_initializes_size_and_cap() {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    assert_eq!(h.buckets.len(), 100);
    for b in &h.buckets {
        assert!(b.is_none());
    }
}

#[test]
fn test_create_hashmap_small() {
    let h = create_hashmap(4);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 4);
    assert_eq!(h.buckets.len(), 4);
}

#[test]
fn test_fnva1_known_values() {
    // These exact values come from running the C code.
    assert_eq!(fnva1("test"), 1515068489);
    assert_eq!(fnva1("foo"), 1497138325);
    assert_eq!(fnva1(""), 16777619);
    assert_eq!(fnva1("a"), 4004178234);
    assert_eq!(fnva1("hello world"), 147000231);
}

#[test]
fn test_equal_key_basic() {
    assert!(equal_key("test", "test"));
    assert!(!equal_key("test", "tess"));
    assert!(equal_key("", ""));
    assert!(!equal_key("a", ""));
    assert!(!equal_key("", "a"));
}

#[test]
fn test_hm_set_and_get_basic() {
    let mut h = create_hashmap(100);
    let r = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert_eq!(r, 0);
    assert_eq!(h.size, 1);
    let got = hm_get(&h, "test");
    assert!(got.is_some());
    let val = got.unwrap().value.downcast_ref::<String>();
    assert!(val.is_some());
    assert_eq!(val.unwrap(), "jake");
    assert_eq!(got.unwrap().key, "test");
}

#[test]
fn test_hm_get_missing() {
    let h = create_hashmap(100);
    let got = hm_get(&h, "missing");
    assert!(got.is_none());
}

#[test]
fn test_hm_set_index() {
    let mut h = create_hashmap(100);
    let r = hm_set(&mut h, "test", Box::new(42i32));
    assert_eq!(r, 0);
    let ind = (fnva1("test") as usize) % 100;
    let b = h.buckets[ind].as_ref();
    assert!(b.is_some());
    assert_eq!(b.unwrap().key, "test");
    let v = b.unwrap().value.downcast_ref::<i32>();
    assert_eq!(v, Some(&42));
}

#[test]
fn test_hm_set_collision_returns_negative_one() {
    let mut h = create_hashmap(2);
    // Insert one key
    let r1 = hm_set(&mut h, "test", Box::new(1i32));
    // The first set may or may not collide based on cap. Find a colliding key.
    let target_idx = (fnva1("test") as usize) % 2;
    let _ = r1;
    // Try inserting another value with the same hash bucket.
    // We'll search for keys until we find one.
    let mut tries = 0;
    let mut other: String;
    loop {
        other = format!("k{}", tries);
        if other != "test" && (fnva1(&other) as usize) % 2 == target_idx {
            break;
        }
        tries += 1;
        if tries > 1000 {
            panic!("could not find colliding key");
        }
    }
    let r2 = hm_set(&mut h, &other, Box::new(2i32));
    assert_eq!(r2, -1);
}

#[test]
fn test_double_cap_doubles_cap() {
    let mut h = create_hashmap(4);
    double_cap(&mut h);
    assert_eq!(h.cap, 8);
    assert_eq!(h.buckets.len(), 8);
}

#[test]
fn test_double_cap_preserves_value() {
    let mut h = create_hashmap(100);
    let _ = hm_set(&mut h, "test", Box::new("jake".to_string()));
    double_cap(&mut h);
    let got = hm_get(&h, "test");
    assert!(got.is_some());
    let val = got.unwrap().value.downcast_ref::<String>();
    assert_eq!(val, Some(&"jake".to_string()));
}

#[test]
fn test_destroy_hashmap_clears() {
    let mut h = create_hashmap(10);
    let _ = hm_set(&mut h, "a", Box::new(1i32));
    destroy_hashmap(&mut h);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 0);
    assert!(h.buckets.is_empty());
}

#[test]
fn test_test_hash_init_returns_zero() {
    assert_eq!(test_hash_init(), 0);
}

#[test]
fn test_test_hash_init_and_store_returns_zero() {
    assert_eq!(test_hash_init_and_store(), 0);
}

#[test]
fn test_test_hash_set_and_get_returns_zero() {
    assert_eq!(test_hash_set_and_get(), 0);
}

#[test]
fn test_test_hash_set_and_double_get_returns_zero() {
    assert_eq!(test_hash_set_and_double_get(), 0);
}

fn main() {}
