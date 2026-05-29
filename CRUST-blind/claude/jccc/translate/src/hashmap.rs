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
pub struct Hashmap {
    pub buckets: Vec<Option<Box<BucketNode>>>,
    pub size: i32,
    pub cap: i32,
    pub hash: fn(&str) -> u32,
    /// An equality function taking two &str and returning a bool (true if equal).
    pub equals: fn(&str, &str) -> bool,
}

impl fmt::Debug for Hashmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hashmap")
            .field("buckets", &self.buckets)
            .field("size", &self.size)
            .field("cap", &self.cap)
            .finish()
    }
}

pub fn create_hashmap(cap: i32) -> Hashmap {
    let mut buckets = Vec::with_capacity(cap.max(0) as usize);
    for _ in 0..cap.max(0) {
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
    if h.cap <= 0 {
        return None;
    }
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    match &h.buckets[a] {
        Some(b) => {
            if (h.equals)(b.key.as_str(), key) {
                Some(b.as_ref())
            } else {
                None
            }
        }
        None => None,
    }
}

/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap <= 0 {
        return -1;
    }
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    if h.buckets[a].is_none() {
        if h.size == h.cap {
            double_cap(h);
        }
        h.size += 1;
        let node = BucketNode {
            key: key.to_string(),
            value,
            next: None,
        };
        // Recompute index after potential double_cap
        let a = ((h.hash)(key) as usize) % (h.cap as usize);
        h.buckets[a] = Some(Box::new(node));
        0
    } else {
        // Handle linear probing -- not yet implemented in C either
        -1
    }
}

/// Tests setting and getting a value in the Hashmap.
pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return 1;
    }
    match hm_get(&h, "test") {
        Some(node) => {
            if let Some(s) = node.value.downcast_ref::<String>() {
                if s != "jake" {
                    return 1;
                }
            } else {
                return 1;
            }
        }
        None => return 1,
    }
    0
}

/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    let new_cap = h.cap * 2;
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(new_cap.max(0) as usize);
    for _ in 0..new_cap.max(0) {
        new_buckets.push(None);
    }
    let old_buckets = std::mem::take(&mut h.buckets);
    let old_cap = h.cap;
    for bucket in old_buckets.into_iter().flatten() {
        let a = if old_cap > 0 {
            ((h.hash)(bucket.key.as_str()) as usize) % (old_cap as usize)
        } else {
            0
        };
        if a < new_buckets.len() {
            new_buckets[a] = Some(bucket);
        }
    }
    h.buckets = new_buckets;
    h.cap = new_cap;
}

/// A sample FNV-1 hash function.
pub fn fnva1(value: &str) -> u32 {
    let mut h: u64 = 16777619;
    let prime: i64 = 2166136261;
    for byte in value.as_bytes() {
        h ^= *byte as u64;
        h = h.wrapping_mul(prime as u64);
    }
    h as u32
}

/// Tests initializing the Hashmap.
pub fn test_hash_init() -> i32 {
    let h = create_hashmap(100);
    if h.size != 0 {
        return 1;
    }
    if h.cap != 100 {
        return 1;
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
        return 1;
    }
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return 1;
    }
    let ind = ((h.hash)("test") as usize) % (h.cap as usize);
    match &h.buckets[ind] {
        Some(b) => {
            if b.key != "test" {
                return 1;
            }
        }
        None => return 1,
    }
    if h.size != 1 || h.cap != 100 {
        return 1;
    }
    0
}

/// Tests setting a key-value pair, then doubling capacity, then getting the value.
pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return 1;
    }
    double_cap(&mut h);
    match hm_get(&h, "test") {
        Some(node) => {
            if let Some(s) = node.value.downcast_ref::<String>() {
                if s != "jake" {
                    return 1;
                }
            } else {
                return 1;
            }
        }
        None => return 1,
    }
    0
}

/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
