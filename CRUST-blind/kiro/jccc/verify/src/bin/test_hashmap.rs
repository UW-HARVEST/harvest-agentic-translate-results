use jccc::hashmap::{
    create_hashmap, destroy_hashmap, double_cap, equal_key, fnva1, hm_get, hm_set,
};

#[test]
fn test_fnva1_values() {
    // Ground truth from C: fnva1("test") = 1515068489
    assert_eq!(fnva1("test"), 1515068489);
    assert_eq!(fnva1("jake"), 507783454);
    assert_eq!(fnva1("hello"), 3614833289);
    assert_eq!(fnva1(""), 16777619);
    assert_eq!(fnva1("a"), 4004178234);
}

#[test]
fn test_equal_key() {
    assert!(equal_key("abc", "abc"));
    assert!(!equal_key("abc", "def"));
    assert!(!equal_key("", "a"));
    assert!(equal_key("", ""));
}

#[test]
fn test_hash_init() {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    assert_eq!(h.buckets.len(), 100);
}

#[test]
fn test_hash_init_and_store() {
    let mut h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert!(ret != -1);
    let ind = fnva1("test") as usize % h.cap as usize;
    assert_eq!(ind, 89); // ground truth: fnva1("test") % 100 = 89
    let b = h.buckets[ind].as_ref().unwrap();
    assert_eq!(b.key, "test");
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);
}

#[test]
fn test_hash_set_and_get() {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert!(ret != -1);
    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
    assert_eq!(got.key, "test");
}

#[test]
fn test_hash_set_and_double_get() {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert!(ret != -1);
    double_cap(&mut h);
    assert_eq!(h.cap, 200);
    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
}

#[test]
fn test_hash_set_duplicate_returns_neg1() {
    let mut h = create_hashmap(100);
    let ret1 = hm_set(&mut h, "test", Box::new(1));
    assert_eq!(ret1, 0);
    let ret2 = hm_set(&mut h, "test", Box::new(2));
    assert_eq!(ret2, -1);
}

#[test]
fn test_destroy_hashmap() {
    let mut h = create_hashmap(50);
    hm_set(&mut h, "key", Box::new(42));
    destroy_hashmap(&mut h);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 0);
    assert_eq!(h.buckets.len(), 0);
}

#[test]
fn test_hm_get_missing_key() {
    let h = create_hashmap(100);
    assert!(hm_get(&h, "nonexistent").is_none());
}

fn main() {}
