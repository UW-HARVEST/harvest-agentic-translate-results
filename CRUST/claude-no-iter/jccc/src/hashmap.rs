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
            .field("size", &self.size)
            .field("cap", &self.cap)
            .finish()
    }
}

/// Creates a new hashmap with the given capacity.
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
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    let b = h.buckets.get(a)?.as_deref()?;
    if (h.equals)(&b.key, key) {
        return Some(b);
    }
    None
}

/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    if h.buckets[a].is_none() {
        if h.size == h.cap {
            double_cap(h);
        }
        h.size += 1;
        h.buckets[a] = Some(Box::new(BucketNode {
            key: key.to_string(),
            value,
            next: None,
        }));
        0
    } else {
        // Linear probing not handled.
        -1
    }
}

/// Tests setting and getting a value in the Hashmap.
pub fn test_hash_set_and_get() -> i32 {
    let mut h = create_hashmap(100);
    let name = "jake".to_string();
    let key = "test".to_string();
    let ret = hm_set(&mut h, &key, Box::new(name));
    assert_ne!(ret, -1);
    let got = hm_get(&h, &key).unwrap();
    if let Some(s) = got.value.downcast_ref::<String>() {
        assert_eq!(s, "jake");
    } else {
        panic!("expected String");
    }
    0
}

/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    let new_cap = (h.cap * 2) as usize;
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(new_cap);
    for _ in 0..new_cap {
        new_buckets.push(None);
    }
    let old_cap = h.cap as usize;
    let old_buckets = std::mem::take(&mut h.buckets);
    for slot in old_buckets.into_iter() {
        if let Some(b) = slot {
            // C bug: uses old cap modulus when re-bucketing — keep it identical.
            let a = ((h.hash)(&b.key) as usize) % old_cap;
            new_buckets[a] = Some(b);
        }
    }
    h.buckets = new_buckets;
    h.cap *= 2;
}

/// A sample FNV-1 hash function (mirrors the C `fnva1`).
pub fn fnva1(value: &str) -> u32 {
    // Mirror the C version: `unsigned long h = 16777619; long int prime = 2166136261;`
    // Truncated to u32 to match the function signature.
    let mut h: u64 = 16_777_619;
    let prime: u64 = 2_166_136_261;
    for &b in value.as_bytes() {
        h ^= b as u64;
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
    let name = "jake".to_string();
    let key = "test".to_string();
    let ret = hm_set(&mut h, &key, Box::new(name));
    assert_ne!(ret, -1);
    let ind = ((h.hash)(&key) % (h.cap as u32)) as usize;
    let b = h.buckets[ind].as_ref().unwrap();
    assert_eq!(b.key, key);
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);
    0
}

/// Tests setting a key-value pair, then doubling capacity, then getting the value.
pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let name = "jake".to_string();
    let key = "test".to_string();
    let ret = hm_set(&mut h, &key, Box::new(name));
    assert_ne!(ret, -1);
    double_cap(&mut h);
    let got = hm_get(&h, &key).unwrap();
    if let Some(s) = got.value.downcast_ref::<String>() {
        assert_eq!(s, "jake");
    } else {
        panic!("expected String");
    }
    0
}

/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
