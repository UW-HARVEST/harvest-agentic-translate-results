use std::any::Any;
use std::fmt;

#[derive(Debug)]
pub struct BucketNode {
    pub key: String,
    pub value: Box<dyn Any>,
    pub next: Option<Box<BucketNode>>,
}

#[derive(Debug)]
pub struct Hashmap {
    pub buckets: Vec<Option<Box<BucketNode>>>,
    pub size: i32,
    pub cap: i32,
    pub hash: fn(&str) -> u32,
    pub equals: fn(&str, &str) -> bool,
}

pub fn fnva1(value: &str) -> u32 {
    let mut h: u32 = 16777619;
    let prime: u32 = 2166136261;
    for b in value.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(prime);
    }
    h
}

pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}

pub fn create_hashmap(cap: i32) -> Hashmap {
    let mut buckets = Vec::with_capacity(cap as usize);
    for _ in 0..cap {
        buckets.push(None);
    }
    Hashmap {
        buckets,
        size: 0,
        cap,
        hash: fnva1,
        equals: equal_key,
    }
}

pub fn destroy_hashmap(h: &mut Hashmap) {
    h.buckets.clear();
    h.size = 0;
    h.cap = 0;
}

pub fn hm_get<'a>(h: &'a Hashmap, key: &'a str) -> Option<&'a BucketNode> {
    let a = (h.hash)(key) % (h.cap as u32);
    let bucket = &h.buckets[a as usize];
    match bucket {
        None => None,
        Some(b) => {
            if (h.equals)(&b.key, key) {
                Some(b)
            } else {
                None
            }
        }
    }
}

pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    let a = (h.hash)(key) % (h.cap as u32);
    let idx = a as usize;
    if h.buckets[idx].is_none() {
        if h.size == h.cap {
            double_cap(h);
        }
        h.size += 1;
        h.buckets[idx] = Some(Box::new(BucketNode {
            key: key.to_string(),
            value,
            next: None,
        }));
        0
    } else {
        -1
    }
}

pub fn double_cap(h: &mut Hashmap) {
    let new_cap = (h.cap * 2) as usize;
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        new_buckets.push(None);
    }
    for bucket in h.buckets.drain(..) {
        if let Some(b) = bucket {
            let a = (h.hash)(&b.key) % (h.cap as u32);
            new_buckets[a as usize] = Some(b);
        }
    }
    h.buckets = new_buckets;
    h.cap = h.cap * 2;
}

pub fn test_hash_init() -> i32 {
    let h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    0
}

pub fn test_hash_init_and_store() -> i32 {
    let mut h = create_hashmap(100);
    assert_eq!(h.size, 0);
    assert_eq!(h.cap, 100);
    let key = "test";
    let ret = hm_set(&mut h, key, Box::new("jake".to_string()));
    assert!(ret != -1);
    let ind = (h.hash)(key) % (h.cap as u32);
    let b = h.buckets[ind as usize].as_ref().unwrap();
    assert_eq!(b.key, key);
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);
    0
}

pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert!(ret != -1);
    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
    0
}

pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    assert!(ret != -1);
    double_cap(&mut h);
    let got = hm_get(&h, "test").unwrap();
    let val = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(val, "jake");
    0
}
