use rubiksolver::hash::Hash;

#[test]
fn test_hash_empty_does_not_contain() {
    let hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        false
    );
}

#[test]
fn test_hash_insert_then_exists() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert_eq!(hash.insert("Hello".to_string(), |a, b| a == b), true);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        true
    );
}

#[test]
fn test_hash_insert_multiple_then_exists() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    hash.insert("Hi".to_string(), |a, b| a == b);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        true
    );
    assert_eq!(
        hash.element_exists(&"Hi".to_string(), |a, b| a == b),
        true
    );
}

#[test]
fn test_hash_delete() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    hash.insert("Hi".to_string(), |a, b| a == b);
    assert_eq!(hash.delete(&"Hello".to_string(), |a, b| a == b), true);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        false
    );
    // "Hi" is still present
    assert_eq!(
        hash.element_exists(&"Hi".to_string(), |a, b| a == b),
        true
    );
}

#[test]
fn test_hash_delete_nonexistent_returns_false() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    assert_eq!(hash.delete(&"Hello".to_string(), |a, b| a == b), false);
    hash.insert("Hi".to_string(), |a, b| a == b);
    // Hash to bucket index 'H'(72) - "Hello" hashes to 72 too but doesn't exist
    assert_eq!(hash.delete(&"Hello".to_string(), |a, b| a == b), false);
}

#[test]
fn test_hash_collision_chain() {
    // Both "Hello" and "Hi" hash to the same bucket (index 72)
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    hash.insert("Hi".to_string(), |a, b| a == b);
    hash.insert("Howdy".to_string(), |a, b| a == b);
    // Delete the middle one
    assert_eq!(hash.delete(&"Hi".to_string(), |a, b| a == b), true);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        true
    );
    assert_eq!(
        hash.element_exists(&"Hi".to_string(), |a, b| a == b),
        false
    );
    assert_eq!(
        hash.element_exists(&"Howdy".to_string(), |a, b| a == b),
        true
    );
}

#[test]
fn test_hash_delete_then_reinsert() {
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("Hello".to_string(), |a, b| a == b);
    hash.delete(&"Hello".to_string(), |a, b| a == b);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        false
    );
    hash.insert("Hello".to_string(), |a, b| a == b);
    assert_eq!(
        hash.element_exists(&"Hello".to_string(), |a, b| a == b),
        true
    );
}

#[test]
fn test_hash_different_buckets() {
    // Test elements that hash to different buckets
    let mut hash: Hash<String> = Hash::new(255, |s: &String| s.as_bytes()[0] as u32);
    hash.insert("apple".to_string(), |a, b| a == b);
    hash.insert("banana".to_string(), |a, b| a == b);
    hash.insert("cherry".to_string(), |a, b| a == b);
    assert_eq!(
        hash.element_exists(&"apple".to_string(), |a, b| a == b),
        true
    );
    assert_eq!(
        hash.element_exists(&"banana".to_string(), |a, b| a == b),
        true
    );
    assert_eq!(
        hash.element_exists(&"cherry".to_string(), |a, b| a == b),
        true
    );
    assert_eq!(
        hash.element_exists(&"durian".to_string(), |a, b| a == b),
        false
    );
}

fn main() {}
