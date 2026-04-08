use Graph_recogniser::hash::{
    hash, alternative_hash, hash_by_power, compare_keys, rehash, hash_str,
    POWER, ALTERNATIVE_POWER, REHASHER,
};
use std::cmp::Ordering;

#[test]
fn test_constants() {
    assert_eq!(POWER, 131);
    assert_eq!(ALTERNATIVE_POWER, 171);
    assert_eq!(REHASHER, 718841);
}

#[test]
fn test_hash_values() {
    assert_eq!(hash("hello"), 792145550);
    assert_eq!(hash("world"), 937260934);
    assert_eq!(hash("a"), 97);
    assert_eq!(hash("ab"), 12805);
    assert_eq!(hash("abc"), 1677554);
    assert_eq!(hash(""), 0);
    assert_eq!(hash("test"), 262526998);
    assert_eq!(hash("stefan"), 4266133291);
    assert_eq!(hash("nikola"), 1416358648);
}

#[test]
fn test_alternative_hash_values() {
    assert_eq!(alternative_hash("hello"), 3532604422);
    assert_eq!(alternative_hash("world"), 3523421294);
    assert_eq!(alternative_hash("a"), 97);
    assert_eq!(alternative_hash("test"), 582997598);
}

#[test]
fn test_hash_by_power() {
    assert_eq!(hash_by_power("hello", 131), 792145550);
    assert_eq!(hash_by_power("hello", 171), 3532604422);
    assert_eq!(hash_by_power("hello", 1), 532);
    assert_eq!(hash_by_power("hello", 0), 111);
    assert_eq!(hash_by_power("", 131), 0);
    assert_eq!(hash_by_power("a", 0), 97);
}

#[test]
fn test_hash_str() {
    assert_eq!(hash_str("hello"), 792145550);
    assert_eq!(hash_str(""), 0);
    assert_eq!(hash_str("test"), 262526998);
}

#[test]
fn test_rehash() {
    assert_eq!(rehash(0), 718841);
    assert_eq!(rehash(1), 718842);
    assert_eq!(rehash(718841), 1437682);
    assert_eq!(rehash(4294967295), 718840); // wrapping add
}

#[test]
fn test_compare_keys() {
    assert_eq!(compare_keys("a", "b"), Ordering::Less);
    assert_eq!(compare_keys("b", "a"), Ordering::Greater);
    assert_eq!(compare_keys("a", "a"), Ordering::Equal);
    assert_eq!(compare_keys("abc", "abd"), Ordering::Less);
    assert_eq!(compare_keys("", ""), Ordering::Equal);
    assert_eq!(compare_keys("", "a"), Ordering::Less);
}

fn main() {}
