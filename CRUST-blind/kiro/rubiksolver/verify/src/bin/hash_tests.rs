use rubiksolver::hash::Hash;

fn str_hash(s: &&str) -> u32 {
    s.bytes().next().unwrap() as u32
}

fn str_eq(a: &&str, b: &&str) -> bool {
    *a == *b
}

#[test]
fn test_element_not_found_in_empty_hash() {
    let hash: Hash<&str> = Hash::new(255, str_hash);
    assert_eq!(hash.element_exists(&"Hello", str_eq), false);
}

#[test]
fn test_insert_and_exists() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    assert_eq!(hash.insert("Hello", str_eq), true);
    assert_eq!(hash.element_exists(&"Hello", str_eq), true);
}

#[test]
fn test_insert_two_same_bucket_and_exists() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    assert_eq!(hash.insert("Hello", str_eq), true);
    assert_eq!(hash.insert("Hi", str_eq), true);
    assert_eq!(hash.element_exists(&"Hello", str_eq), true);
    assert_eq!(hash.element_exists(&"Hi", str_eq), true);
}

#[test]
fn test_delete_first_of_two_in_bucket() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    hash.insert("Hello", str_eq);
    hash.insert("Hi", str_eq);
    assert_eq!(hash.delete(&"Hello", str_eq), true);
    assert_eq!(hash.element_exists(&"Hello", str_eq), false);
    assert_eq!(hash.element_exists(&"Hi", str_eq), true);
}

#[test]
fn test_delete_nonexistent() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    assert_eq!(hash.delete(&"Nothing", str_eq), false);
}

#[test]
fn test_delete_second_in_bucket() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    hash.insert("Hello", str_eq);
    hash.insert("Hi", str_eq);
    assert_eq!(hash.delete(&"Hi", str_eq), true);
    assert_eq!(hash.element_exists(&"Hi", str_eq), false);
    assert_eq!(hash.element_exists(&"Hello", str_eq), true);
}

#[test]
fn test_insert_duplicate_rejected() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    assert_eq!(hash.insert("Hello", str_eq), true);
    assert_eq!(hash.insert("Hello", str_eq), false);
    assert_eq!(hash.element_exists(&"Hello", str_eq), true);
}

#[test]
fn test_different_buckets() {
    let mut hash: Hash<&str> = Hash::new(255, str_hash);
    hash.insert("Alpha", str_eq);
    hash.insert("Bravo", str_eq);
    assert_eq!(hash.element_exists(&"Alpha", str_eq), true);
    assert_eq!(hash.element_exists(&"Bravo", str_eq), true);
    assert_eq!(hash.element_exists(&"Charlie", str_eq), false);
}

fn main() {}
