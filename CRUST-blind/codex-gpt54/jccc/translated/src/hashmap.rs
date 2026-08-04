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
    let capacity = cap.max(0) as usize;
    Hashmap {
        buckets: std::iter::repeat_with(|| None).take(capacity).collect(),
        size: 0,
        cap: cap.max(0),
        hash: fnva1,
        equals: equal_key,
    }
}
/// Gets a value from the Hashmap.
pub fn hm_get<'a>(h: &'a Hashmap, key: &'a str) -> Option<&'a BucketNode> {
    if h.cap <= 0 {
        return None;
    }

    let bucket_index = ((h.hash)(key) % (h.cap as u32)) as usize;
    let bucket = h.buckets.get(bucket_index)?.as_ref()?;

    if (h.equals)(&bucket.key, key) {
        Some(bucket.as_ref())
    } else {
        None
    }
}
/// Sets a key-value pair in the Hashmap.
pub fn hm_set(h: &mut Hashmap, key: &str, value: Box<dyn Any>) -> i32 {
    if h.cap <= 0 {
        return -1;
    }

    let bucket_index = ((h.hash)(key) % (h.cap as u32)) as usize;
    let bucket = match h.buckets.get(bucket_index) {
        Some(bucket) => bucket,
        None => return -1,
    };

    if bucket.is_none() {
        if h.size == h.cap {
            double_cap(h);
        }

        h.size += 1;
        if let Some(slot) = h.buckets.get_mut(bucket_index) {
            *slot = Some(Box::new(BucketNode {
                key: key.to_string(),
                value,
                next: None,
            }));
            0
        } else {
            -1
        }
    } else {
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

    match hm_get(&h, "test")
        .and_then(|node| node.value.as_ref().downcast_ref::<String>())
        .map(|s| s == "jake")
    {
        Some(true) => 0,
        _ => -1,
    }
}
/// Doubles the capacity of the Hashmap.
pub fn double_cap(h: &mut Hashmap) {
    if h.cap <= 0 {
        return;
    }

    let old_cap = h.cap;
    let mut new_buckets: Vec<Option<Box<BucketNode>>> = std::iter::repeat_with(|| None)
        .take((old_cap * 2) as usize)
        .collect();

    for slot in h.buckets.iter_mut() {
        if let Some(bucket) = slot.take() {
            let bucket_index = ((h.hash)(&bucket.key) % (old_cap as u32)) as usize;
            new_buckets[bucket_index] = Some(bucket);
        }
    }

    h.buckets = new_buckets;
    h.cap = old_cap * 2;
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
    if h.size == 0 && h.cap == 100 { 0 } else { -1 }
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

    let index = ((h.hash)("test") % (h.cap as u32)) as usize;
    let bucket = match h.buckets.get(index).and_then(|slot| slot.as_ref()) {
        Some(bucket) => bucket,
        None => return -1,
    };

    if bucket.key == "test" && h.size == 1 && h.cap == 100 {
        0
    } else {
        -1
    }
}
/// Tests setting a key-value pair, then doubling capacity, then getting the value.
pub fn test_hash_set_and_double_get() -> i32 {
    let mut h = create_hashmap(100);
    let ret = hm_set(&mut h, "test", Box::new(String::from("jake")));
    if ret == -1 {
        return -1;
    }

    double_cap(&mut h);

    match hm_get(&h, "test")
        .and_then(|node| node.value.as_ref().downcast_ref::<String>())
        .map(|s| s == "jake")
    {
        Some(true) => 0,
        _ => -1,
    }
}
/// Compares two string keys for equality.
pub fn equal_key(a: &str, b: &str) -> bool {
    a == b
}
