/// A simple hash table using a vector of buckets.
/// The table uses a provided hash function (which returns a u32)
/// and a caller‑supplied equality function for comparing elements.
pub struct Hash<T> {
    buckets: Vec<Vec<T>>,
    hash_function: Box<dyn Fn(&T) -> u32>,
    max_key: u32,
}
impl<T> Hash<T> {
    /// Create a new hash table with `max_key` buckets.
    /// The `hash_function` is used to compute a hash for each element.
    pub fn new(max_key: u32, hash_function: impl Fn(&T) -> u32 + 'static) -> Self {
        let mut buckets = Vec::with_capacity((max_key + 1) as usize);
        for _ in 0..=max_key {
            buckets.push(Vec::new());
        }
        Hash {
            buckets,
            hash_function: Box::new(hash_function),
            max_key,
        }
    }
    /// Inserts an element if no equal element exists (using the provided compare_equal).
    /// Returns true if the element was inserted.
    pub fn insert(&mut self, element: T, compare_equal: impl Fn(&T, &T) -> bool) -> bool {
        let ix = (self.hash_function)(&element);
        assert!(ix <= self.max_key);
        let bucket = &self.buckets[ix as usize];
        for existing in bucket {
            if compare_equal(existing, &element) {
                return false;
            }
        }
        self.buckets[ix as usize].push(element);
        true
    }
    /// Returns true if an element equal (per compare_equal) to `element` exists.
    pub fn element_exists(&self, element: &T, compare_equal: impl Fn(&T, &T) -> bool) -> bool {
        let ix = (self.hash_function)(element);
        assert!(ix <= self.max_key);
        for existing in &self.buckets[ix as usize] {
            if compare_equal(existing, element) {
                return true;
            }
        }
        false
    }
    /// Deletes an element (using compare_equal) from the hash table.
    /// Returns true if the element was found and removed.
    pub fn delete(&mut self, element: &T, compare_equal: impl Fn(&T, &T) -> bool) -> bool {
        let ix = (self.hash_function)(element);
        assert!(ix <= self.max_key);
        let bucket = &self.buckets[ix as usize];
        if let Some(pos) = bucket.iter().position(|e| compare_equal(e, element)) {
            self.buckets[ix as usize].remove(pos);
            true
        } else {
            false
        }
    }
}
