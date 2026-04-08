use jccc::hashmap::*;

#[test]
fn test_fnva1_known_values() {
    assert_eq!(fnva1("test"), 1515068489);
    assert_eq!(fnva1(""), 16777619);
    assert_eq!(fnva1("a"), 4004178234);
    assert_eq!(fnva1("hello"), 3614833289);
}

#[test]
fn test_equal_key() {
    assert!(equal_key("abc", "abc"));
    assert!(!equal_key("abc", "def"));
    assert!(!equal_key("", "a"));
    assert!(equal_key("", ""));
}

#[test]
fn test_create_hashmap() {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    assert_eq!(h.buckets.len(), 100);
}

#[test]
fn test_destroy_hashmap() {
    let mut h = create_hashmap(50);
    destroy_hashmap(&mut h);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 0);
    assert!(h.buckets.is_empty());
}

#[test]
fn test_hm_set_and_get() {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert_eq!(ret, 0);
    assert_eq!(h.size, 1);

    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
}

#[test]
fn test_hm_get_missing() {
    let h = create_hashmap(100);
    assert!(hm_get(&h, "nonexistent").is_none());
}

#[test]
fn test_double_cap() {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert_eq!(ret, 0);

    double_cap(&mut h);
    assert_eq!(h.cap, 200);

    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
}

#[test]
fn test_hm_set_multiple() {
    let mut h = create_hashmap(100);
    assert_eq!(hm_set(&mut h, "key1", Box::new(1)), 0);
    assert_eq!(hm_set(&mut h, "key2", Box::new(2)), 0);
    assert_eq!(h.size, 2);
}

fn main() {}
