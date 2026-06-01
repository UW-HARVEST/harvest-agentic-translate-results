use std::any::Any;
use std::fmt;
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
/// Gets a value from the Hashmap.
pub fn hm_get<'a>(h: &'a Hashmap, key: &'a str) -> Option<&'a BucketNode> {
    if h.cap == 0 {
        return None;
    }
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    let b = h.buckets.get(a)?.as_ref()?;
    if (h.equals)(&b.key, key) {
        Some(b.as_ref())
    } else {
        None
    }
}
/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap == 0 {
        return -1;
    }
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    if h.buckets[a].is_none() {
        if h.size == h.cap {
            double_cap(h);
        }
        h.size += 1;
        let new_a = ((h.hash)(key) as usize) % (h.cap as usize);
        h.buckets[new_a] = Some(Box::new(BucketNode {
            key: key.to_string(),
            value,
            next: None,
        }));
        0
    } else {
        // Linear probing not yet supported
        -1
    }
}
/// Tests setting and getting a value in the Hashmap.
pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    if ret == -1 {
        return -1;
    }
    let got = hm_get(&h, "test");
    if got.is_none() {
        return -1;
    }
    let val = got.unwrap().value.downcast_ref::<String>();
    if val.is_none() || val.unwrap() != "jake" {
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
    // Mirror C bug: hash mod old cap, but place into new_buckets at that index.
    let old_cap = h.cap;
    for i in 0..old_cap as usize {
        if let Some(b) = h.buckets[i].take() {
            let a = ((h.hash)(&b.key) as usize) % (old_cap as usize);
            new_buckets[a] = Some(b);
        }
    }
    h.buckets = new_buckets;
    h.cap = new_cap;
}
/// A sample FNV-1 hash function.
pub fn fnva1(value: &str) -> u32 {
    // Mirror C exactly:
    // unsigned long h = 16777619;
    // long int prime = 2166136261;
    // while (*value != '\0') {
    //     h ^= *value;
    //     h *= prime;
    //     ++value;
    // }
    // return h;  // truncated to unsigned int
    let mut h: u64 = 16777619u64;
    // The literal 2166136261 is an int literal in C; on a typical platform
    // long int is 64-bit signed, so this fits.
    let prime: i64 = 2166136261i64;
    let prime_u: u64 = prime as u64;
    for byte in value.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(prime_u);
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
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    if ret == -1 {
        return -1;
    }
    let ind = ((h.hash)("test") as usize) % (h.cap as usize);
    let b = match &h.buckets[ind] {
        Some(b) => b,
        None => return -1,
    };
    if b.key != "test" {
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
    let ret = hm_set(&mut h, "test", Box::new("jake".to_string()));
    if ret == -1 {
        return -1;
    }
    double_cap(&mut h);
    let got = hm_get(&h, "test");
    if got.is_none() {
        return -1;
    }
    let val = got.unwrap().value.downcast_ref::<String>();
    if val.is_none() || val.unwrap() != "jake" {
        return -1;
    }
    0
}
/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}

impl fmt::Display for BucketNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BucketNode({})", self.key)
    }
}
