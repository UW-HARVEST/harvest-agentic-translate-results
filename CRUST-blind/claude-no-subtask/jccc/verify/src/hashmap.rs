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

pub fn create_hashmap(cap: i32) -> Hashmap {
    let cap_usize = if cap < 0 { 0 } else { cap as usize };
    let mut buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(cap_usize);
    for _ in 0..cap_usize {
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
    let bucket = h.buckets.get(a)?;
    let node = bucket.as_ref()?;
    if (h.equals)(&node.key, key) {
        return Some(node.as_ref());
    }
    // Walk chain (chain handling for completeness)
    let mut cur = node.next.as_ref();
    while let Some(n) = cur {
        if (h.equals)(&n.key, key) {
            return Some(n.as_ref());
        }
        cur = n.next.as_ref();
    }
    None
}

/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap <= 0 {
        return -1;
    }
    let a = ((h.hash)(key) as usize) % (h.cap as usize);
    let existing = h.buckets[a].is_some();

    if !existing {
        if h.size == h.cap {
            double_cap(h);
        }
        h.size += 1;
        // Re-compute index after potential double_cap.
        let a2 = ((h.hash)(key) as usize) % (h.cap as usize);
        h.buckets[a2] = Some(Box::new(BucketNode {
            key: key.to_string(),
            value,
            next: None,
        }));
        0
    } else {
        // C version returns -1 here (linear probing not implemented)
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
    let got = hm_get(&h, "test").unwrap();
    let v = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(v, "jake");
    0
}

/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    let new_cap = h.cap * 2;
    let new_cap_usize = if new_cap < 0 { 0 } else { new_cap as usize };
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = Vec::with_capacity(new_cap_usize);
    for _ in 0..new_cap_usize {
        new_buckets.push(None);
    }

    let old_cap = h.cap;
    let old_cap_usize = if old_cap < 0 { 0 } else { old_cap as usize };
    // Take old buckets out so we can move nodes.
    let old_buckets = std::mem::replace(&mut h.buckets, new_buckets);

    for (i, b) in old_buckets.into_iter().enumerate() {
        if i >= old_cap_usize {
            break;
        }
        if let Some(node) = b {
            // C bug: C uses h->cap (old cap) here instead of new cap, so to
            // mimic, use old cap.
            let a = ((h.hash)(&node.key) as usize) % (old_cap_usize.max(1));
            if a < new_cap_usize {
                h.buckets[a] = Some(node);
            }
        }
    }
    h.cap = new_cap;
}

/// A sample FNV-1 hash function.
pub fn fnva1(value: &str) -> u32 {
    // C uses unsigned long for h (init 16777619) and long int prime (2166136261).
    // We mimic with u64 and wrap.
    let mut h: u64 = 16777619;
    let prime: u64 = 2166136261;
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

    let key = "test";
    let ret = hm_set(&mut h, key, Box::new("jake".to_string()));
    if ret == -1 {
        return -1;
    }
    let ind = ((h.hash)(key) as usize) % (h.cap as usize);
    let b = h.buckets[ind].as_ref().unwrap();
    assert_eq!(b.key, key);
    assert_eq!(h.size, 1);
    assert_eq!(h.cap, 100);
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
    let got = hm_get(&h, "test").unwrap();
    let v = got.value.downcast_ref::<String>().unwrap();
    assert_eq!(v, "jake");
    0
}

/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
