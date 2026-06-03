//! Rust translation of c_src/src/lib.c
//!
//! The original C code embeds the stb_ds (stb data structures) library and
//! exposes a single public function, `intput`, which exercises an integer
//! hash map. The Rust translation preserves the public API and the
//! observable behaviour of `intput` while replacing the hand-rolled C hash
//! map with the equivalents from the Rust standard library.

use std::collections::HashMap;
use std::os::raw::c_int;

/// Reproduces the behaviour of `hmput` followed by `hmget` for an `int`-keyed
/// hash map, mirroring the assertions found in the original C code.
///
/// Note: just like the C source, calling this function with `num == 9` or
/// `num == 11` will trip the internal assertions, because those values
/// collide with the literal keys used below.
#[no_mangle]
pub extern "C" fn intput(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    // hmput(intmap, num, 7);
    intmap.insert(num, 7);
    // hmput(intmap, 11, 3);
    intmap.insert(11, 3);
    // hmput(intmap,  9, num);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap, 9) == num);
    assert_eq!(*intmap.get(&9).expect("key 9 must be present"), num);
    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    assert_eq!(*intmap.get(&11).expect("key 11 must be present"), 3);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    assert_eq!(*intmap.get(&num).expect("key num must be present"), 7);
}

// ---------------------------------------------------------------------------
// Direct Rust translations of the stb_ds primitives that backed the original
// implementation. They are kept for parity with the C source and are exposed
// as safe Rust APIs (rather than C-style void* macros) so that other Rust
// callers in this crate can use them. They are not part of the public C ABI.
// ---------------------------------------------------------------------------

/// A growable, contiguous array, equivalent to stb_ds's `arrput` family.
#[derive(Debug, Default, Clone)]
pub struct StbArray<T> {
    items: Vec<T>,
}

impl<T> StbArray<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: Vec::with_capacity(cap),
        }
    }

    /// `arrput(a, v)`
    pub fn put(&mut self, value: T) {
        self.items.push(value);
    }

    /// `arrpop(a)`
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    /// `arrlen(a)`
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `arrcap(a)`
    pub fn cap(&self) -> usize {
        self.items.capacity()
    }

    /// `arrsetcap(a, n)`
    pub fn set_cap(&mut self, n: usize) {
        if n > self.items.capacity() {
            self.items.reserve(n - self.items.len());
        }
    }

    /// `arrsetlen(a, n)` — extends with `Default::default()` if needed.
    pub fn set_len(&mut self, n: usize)
    where
        T: Default,
    {
        if n < self.items.len() {
            self.items.truncate(n);
        } else {
            self.items.resize_with(n, T::default);
        }
    }

    /// `arrlast(a)`
    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }

    /// `arrins(a, i, v)`
    pub fn ins(&mut self, i: usize, value: T) {
        self.items.insert(i, value);
    }

    /// `arrdel(a, i)`
    pub fn del(&mut self, i: usize) -> T {
        self.items.remove(i)
    }

    /// `arrdelswap(a, i)`
    pub fn del_swap(&mut self, i: usize) -> T {
        self.items.swap_remove(i)
    }

    /// `arrfree(a)`
    pub fn free(&mut self) {
        self.items.clear();
        self.items.shrink_to_fit();
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items
    }
}

impl<T> std::ops::Index<usize> for StbArray<T> {
    type Output = T;
    fn index(&self, idx: usize) -> &T {
        &self.items[idx]
    }
}

impl<T> std::ops::IndexMut<usize> for StbArray<T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.items[idx]
    }
}

/// A hash map, equivalent to stb_ds's `hmput`/`hmget` family.
#[derive(Debug, Clone)]
pub struct StbHashMap<K: std::hash::Hash + Eq, V> {
    map: HashMap<K, V>,
    default_value: Option<V>,
}

impl<K: std::hash::Hash + Eq, V> Default for StbHashMap<K, V> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            default_value: None,
        }
    }
}

impl<K: std::hash::Hash + Eq, V> StbHashMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `hmput(t, k, v)`
    pub fn put(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    /// `hmget(t, k)`
    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.map.get(key).or(self.default_value.as_ref())
    }

    /// `hmgetp(t, k)` — pointer-style accessor.
    pub fn get_ptr<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.map.get(key)
    }

    /// `hmgeti(t, k)` — returns -1 if absent, otherwise a non-negative index.
    /// Since Rust's HashMap does not expose stable indices, this returns 0
    /// when present and -1 when absent, mirroring the meaningful contract of
    /// the original (presence test).
    pub fn geti(&self, key: &K) -> isize {
        if self.map.contains_key(key) {
            0
        } else {
            -1
        }
    }

    /// `hmdel(t, k)` — returns 1 if deleted, 0 otherwise.
    pub fn del(&mut self, key: &K) -> i32 {
        if self.map.remove(key).is_some() {
            1
        } else {
            0
        }
    }

    /// `hmlen(t)`
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `hmdefault(t, v)`
    pub fn set_default(&mut self, value: V) {
        self.default_value = Some(value);
    }

    /// `hmfree(t)`
    pub fn free(&mut self) {
        self.map.clear();
        self.default_value = None;
    }
}

/// A string-keyed hash map, equivalent to stb_ds's `shput`/`shget` family.
pub type StbStrHashMap<V> = StbHashMap<String, V>;

// ---------------------------------------------------------------------------
// Hashing helpers. The original C code shipped its own SipHash and a custom
// string hash. We provide thin wrappers over the standard library's hasher
// so callers can compute size_t-style hashes for parity with the C API.
// ---------------------------------------------------------------------------

use std::hash::{Hash, Hasher};

/// Equivalent of `stbds_hash_bytes`.
pub fn stbds_hash_bytes(p: &[u8], seed: usize) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    p.hash(&mut hasher);
    hasher.finish() as usize
}

/// Equivalent of `stbds_hash_string`.
pub fn stbds_hash_string(s: &str, seed: usize) -> usize {
    stbds_hash_bytes(s.as_bytes(), seed)
}

/// Equivalent of `stbds_rand_seed` — the seeding here is a no-op because the
/// Rust standard library's hasher cannot be reseeded externally.
pub fn stbds_rand_seed(_seed: usize) {
    // Intentionally left blank: DefaultHasher is keyed by the runtime.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intput_basic() {
        // Use any value other than 9 or 11 to satisfy the original asserts.
        intput(42);
    }

    #[test]
    fn array_put_get() {
        let mut a: StbArray<i32> = StbArray::new();
        a.put(1);
        a.put(2);
        a.put(3);
        assert_eq!(a.len(), 3);
        assert_eq!(a[0], 1);
        assert_eq!(a[2], 3);
        assert_eq!(a.pop(), Some(3));
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn hashmap_put_get() {
        let mut m: StbHashMap<i32, i32> = StbHashMap::new();
        m.put(1, 100);
        m.put(2, 200);
        assert_eq!(m.get(&1), Some(&100));
        assert_eq!(m.get(&2), Some(&200));
        assert_eq!(m.geti(&1), 0);
        assert_eq!(m.geti(&3), -1);
        assert_eq!(m.del(&1), 1);
        assert_eq!(m.del(&1), 0);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn hashmap_default() {
        let mut m: StbHashMap<i32, i32> = StbHashMap::new();
        m.set_default(-1);
        assert_eq!(m.get(&999), Some(&-1));
        m.put(1, 7);
        assert_eq!(m.get(&1), Some(&7));
    }

    #[test]
    fn strmap_put_get() {
        let mut m: StbStrHashMap<i32> = StbStrHashMap::new();
        m.put("hello".to_string(), 1);
        m.put("world".to_string(), 2);
        assert_eq!(m.get(&"hello".to_string()), Some(&1));
        assert_eq!(m.get(&"world".to_string()), Some(&2));
    }
}
