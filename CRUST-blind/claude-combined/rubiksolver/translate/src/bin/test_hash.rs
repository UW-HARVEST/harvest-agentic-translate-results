use rubiksolver::hash::Hash;

fn hash_fn(s: &&'static str) -> u32 {
    s.as_bytes()[0] as u32
}

fn cmp_eq(a: &&'static str, b: &&'static str) -> bool {
    *a == *b
}

#[test]
fn test_hash_basic() {
    let mut h: Hash<&'static str> = Hash::new(255, hash_fn);
    assert_eq!(h.element_exists(&"Hello", cmp_eq), false);
    assert_eq!(h.insert("Hello", cmp_eq), true);
    assert_eq!(h.element_exists(&"Hello", cmp_eq), true);
    assert_eq!(h.insert("Hi", cmp_eq), true);
    assert_eq!(h.element_exists(&"Hi", cmp_eq), true);
    assert_eq!(h.delete(&"Hello", cmp_eq), true);
    assert_eq!(h.element_exists(&"Hello", cmp_eq), false);
    // Hi should still exist
    assert_eq!(h.element_exists(&"Hi", cmp_eq), true);
}

#[test]
fn test_hash_delete_nonexistent() {
    let mut h: Hash<&'static str> = Hash::new(255, hash_fn);
    // Deleting from empty hash
    assert_eq!(h.delete(&"NotThere", cmp_eq), false);

    h.insert("Apple", cmp_eq);
    // Deleting an element that doesn't exist but bucket has other elements
    // 'A' = 0x41, 'B' = 0x42 - different buckets so different test
    h.insert("Apricot", cmp_eq); // same bucket as Apple
    assert_eq!(h.delete(&"Avocado", cmp_eq), false);
    assert_eq!(h.element_exists(&"Apple", cmp_eq), true);
    assert_eq!(h.element_exists(&"Apricot", cmp_eq), true);
}

#[test]
fn test_hash_chain_delete() {
    // Test deleting from middle/end of chain
    let mut h: Hash<&'static str> = Hash::new(255, hash_fn);
    h.insert("Apple", cmp_eq);
    h.insert("Apricot", cmp_eq);
    h.insert("Avocado", cmp_eq);
    // Delete middle
    assert_eq!(h.delete(&"Apricot", cmp_eq), true);
    assert_eq!(h.element_exists(&"Apple", cmp_eq), true);
    assert_eq!(h.element_exists(&"Apricot", cmp_eq), false);
    assert_eq!(h.element_exists(&"Avocado", cmp_eq), true);
    // Delete first remaining
    assert_eq!(h.delete(&"Apple", cmp_eq), true);
    assert_eq!(h.element_exists(&"Apple", cmp_eq), false);
    assert_eq!(h.element_exists(&"Avocado", cmp_eq), true);
    // Delete last
    assert_eq!(h.delete(&"Avocado", cmp_eq), true);
    assert_eq!(h.element_exists(&"Avocado", cmp_eq), false);
}

#[test]
fn test_hash_insert_returns_true() {
    let mut h: Hash<&'static str> = Hash::new(255, hash_fn);
    assert_eq!(h.insert("a", cmp_eq), true);
    assert_eq!(h.insert("b", cmp_eq), true);
}

fn main() {}
