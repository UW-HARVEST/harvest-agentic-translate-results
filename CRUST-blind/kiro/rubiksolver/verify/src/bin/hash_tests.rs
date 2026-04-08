use rubiksolver::hash::Hash;

#[test]
fn test_element_not_found_initially() {
    let hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert!(!hash.element_exists(&"Hello".to_string(), |a, b| a == b));
}

#[test]
fn test_insert_and_exists() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert!(hash.insert("Hello".to_string(), |a, b| a == b));
    assert!(hash.element_exists(&"Hello".to_string(), |a, b| a == b));
}

#[test]
fn test_insert_same_bucket_and_exists() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    assert!(hash.insert("Hi".to_string(), |a, b| a == b));
    assert!(hash.element_exists(&"Hi".to_string(), |a, b| a == b));
    assert!(hash.element_exists(&"Hello".to_string(), |a, b| a == b));
}

#[test]
fn test_delete_and_not_exists() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    hash.insert("Hi".to_string(), |a, b| a == b);
    assert!(hash.delete(&"Hello".to_string(), |a, b| a == b));
    assert!(!hash.element_exists(&"Hello".to_string(), |a, b| a == b));
    // "Hi" should still exist
    assert!(hash.element_exists(&"Hi".to_string(), |a, b| a == b));
}

#[test]
fn test_delete_nonexistent_returns_false() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert!(!hash.delete(&"Nothing".to_string(), |a, b| a == b));
}

#[test]
fn test_insert_duplicate_returns_false() {
    // Rust version checks for duplicates (unlike C which always returns true)
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert!(hash.insert("Hello".to_string(), |a, b| a == b));
    assert!(!hash.insert("Hello".to_string(), |a, b| a == b));
}

fn main() {}
