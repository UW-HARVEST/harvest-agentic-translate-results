use std::any::Any;

/// Represents a node in the bucket chain.
#[derive(Debug)]
pub struct BucketNode {
pub key: String,
pub value: Box<dyn Any>,
pub next: Option<Box<BucketNode>>,
}
/// Represents a simple Hashmap structure.
#[derive(Debug)]
pub struct Hashmap {
pub buckets: Vec<Option<Box<BucketNode>>>,
pub size: i32,
pub cap: i32,
pub hash: fn(&str) -> u32,
/// An equality function taking two &str and returning a bool (true if equal).
pub equals: fn(&str, &str) -> bool,
}

fn create_bucket(key: &str, value: Box<dyn Any>) -> Box<BucketNode> {
    Box::new(BucketNode {
        key: key.to_string(),
        value,
        next: None,
    })
}

pub fn create_hashmap(cap: i32) -> Hashmap {
    let mut buckets = Vec::new();
    buckets.resize_with(cap.max(0) as usize, || None);
    Hashmap {
        buckets,
        size: 0,
        cap,
        hash: fnva1,
        equals: equal_key,
    }
}
/// Gets a value from the Hashmap.
pub fn hm_get<'a>(h: &'a Hashmap, key: &'a str) -> Option<&'a BucketNode> {
    if h.cap <= 0 {
        return None;
    }
    let a = ((h.hash)(key) % (h.cap as u32)) as usize;
    let bucket = h.buckets.get(a)?.as_deref()?;
    if (h.equals)(&bucket.key, key) {
        Some(bucket)
    } else {
        None
    }
}
/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap <= 0 {
        return -1;
    }

    let a = ((h.hash)(key) % (h.cap as u32)) as usize;
    if h.buckets[a].is_none() {
        if h.size == h.cap {
            double_cap(h);
        }

        h.size += 1;
        h.buckets[a] = Some(create_bucket(key, value));
        0
    } else {
        -1
    }
}
/// Tests setting and getting a value in the Hashmap.
pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_ne!(ret, -1);

    let got = hm_get(&h, "test").expect("value should be present");
    assert_eq!(
        got.value.downcast_ref::<String>().map(String::as_str),
        Some("jake")
    );
    0
}
/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    if h.cap <= 0 {
        return;
    }

    let old_cap = h.cap;
    let mut new_buckets = Vec::new();
    new_buckets.resize_with((old_cap * 2) as usize, || None);

    for bucket in &mut h.buckets {
        if let Some(node) = bucket.take() {
            let a = ((h.hash)(&node.key) % (old_cap as u32)) as usize;
            new_buckets[a] = Some(node);
        }
    }

    h.buckets = new_buckets;
    h.cap *= 2;
}
/// A sample FNV-1 hash function.
pub fn fnva1(value: &str) -> u32 {
    let mut h: u64 = 16_777_619;
    let prime: u64 = 2_166_136_261;

    for byte in value.bytes() {
        h ^= u64::from(byte);
        h = h.wrapping_mul(prime);
    }

    h as u32
}
/// Tests initializing the Hashmap.
pub fn test_hash_init() -> i32 {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    0
}
/// Destroys the Hashmap and frees resources.
pub fn destroy_hashmap(h: &mut Hashmap) {
    h.buckets.clear();
    h.size = 0;
    h.cap = 0;
}
/// Tests initializing the Hashmap and storing a value.
pub fn test_hash_init_and_store() -> i32 {
    let mut h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);

    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_ne!(ret, -1);

    let ind = ((h.hash)("test") % (h.cap as u32)) as usize;
    let bucket = h.buckets[ind].as_ref().expect("bucket should exist");
    assert_eq!(bucket.key, "test");
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);
    0
}
/// Tests setting a key-value pair, then doubling capacity, then getting the value.
pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    assert_ne!(ret, -1);

    double_cap(&mut h);

    let got = hm_get(&h, "test").expect("value should survive resize");
    assert_eq!(
        got.value.downcast_ref::<String>().map(String::as_str),
        Some("jake")
    );
    0
}
/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
