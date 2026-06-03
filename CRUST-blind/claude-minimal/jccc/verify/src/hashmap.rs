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
pub fn create_hashmap(cap: i32) -> Hashmap {
    let mut buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(cap as usize);
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
/// Gets a value from the Hashmap.
pub fn hm_get<'a>(h: &'a Hashmap, key: &'a str) -> Option<&'a BucketNode> {
    if h.cap == 0 {
        return None;
    }
    let idx = ((h.hash)(key) % h.cap as u32) as usize;
    let bucket = h.buckets.get(idx)?;
    let node = bucket.as_ref()?;
    if (h.equals)(&node.key, key) {
        Some(node.as_ref())
    } else {
        None
    }
}
/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap == 0 {
        return -1;
    }
    let idx = ((h.hash)(key) % h.cap as u32) as usize;
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
        // C version returns -1 in the linear-probing branch.
        -1
    }
}
/// Tests setting and getting a value in the Hashmap.
pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return -1;
    }
    let got = hm_get(&h, "test");
    if got.is_none() {
        return -1;
    }
    let got = got.unwrap();
    if let Some(s) = got.value.downcast_ref::<String>() {
        if s != "jake" {
            return -1;
        }
    } else {
        return -1;
    }
    0
}
/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    let new_cap = h.cap * 2;
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(new_cap as usize);
    for _ in 0..new_cap {
        new_buckets.push(None);
    }
    // Move existing buckets into new_buckets, mirroring the C version
    // (which uses old cap modulus when computing the new index).
    let old_cap = h.cap as u32;
    let old_buckets = std::mem::replace(&mut h.buckets, Vec::new());
    for bucket in old_buckets.into_iter() {
        if let Some(node) = bucket {
            let a = ((h.hash)(&node.key) % old_cap) as usize;
            new_buckets[a] = Some(node);
        }
    }
    h.buckets = new_buckets;
    h.cap = new_cap;
}
/// A sample FNV-1 hash function.
pub fn fnva1(value: &str) -> u32 {
    let mut h: u64 = 16777619;
    let prime: u64 = 2166136261;
    for byte in value.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(prime);
    }
    h as u32
}
/// Tests initializing the Hashmap.
pub fn test_hash_init() -> i32 {
    let h = create_hashmap(100);
    if h.size != 0 || h.cap != 100 {
        return -1;
    }
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
    if h.size != 0 || h.cap != 100 {
        return -1;
    }
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return -1;
    }
    let idx = ((h.hash)("test") % h.cap as u32) as usize;
    let bucket = match h.buckets.get(idx) {
        Some(b) => b,
        None => return -1,
    };
    let node = match bucket {
        Some(n) => n,
        None => return -1,
    };
    if node.key != "test" {
        return -1;
    }
    if h.size != 1 || h.cap != 100 {
        return -1;
    }
    0
}
/// Tests setting a key-value pair, then doubling capacity, then getting the value.
pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return -1;
    }
    double_cap(&mut h);
    let got = hm_get(&h, "test");
    if got.is_none() {
        return -1;
    }
    let got = got.unwrap();
    if let Some(s) = got.value.downcast_ref::<String>() {
        if s != "jake" {
            return -1;
        }
    } else {
        return -1;
    }
    0
}
/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
