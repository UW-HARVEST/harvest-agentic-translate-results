use Graph_recogniser::hash::{hash, alternative_hash, hash_by_power, rehash, compare_keys};
use Graph_recogniser::hash::{POWER, ALTERNATIVE_POWER, REHASHER};
use std::cmp::Ordering;

#[test]
fn test_hash_known_keys() {
    assert_eq!(hash("stefan"), 4266133291);
    assert_eq!(hash("hristo"), 218760243);
    assert_eq!(hash("dimitar"), 1183767080);
    assert_eq!(hash("georgi"), 773914255);
    assert_eq!(hash("stanislav"), 705159857);
    assert_eq!(hash("nikola"), 1416358648);
    assert_eq!(hash("andrei"), 3850980359);
    assert_eq!(hash("hello"), 792145550);
    assert_eq!(hash("world"), 937260934);
}

#[test]
fn test_hash_empty_and_short() {
    assert_eq!(hash(""), 0);
    assert_eq!(hash("a"), 97);
    assert_eq!(hash("ab"), 12805);
    assert_eq!(hash("abc"), 1677554);
}

#[test]
fn test_alternative_hash_known_keys() {
    assert_eq!(alternative_hash("stefan"), 395527203);
    assert_eq!(alternative_hash("hristo"), 995706907);
    assert_eq!(alternative_hash("dimitar"), 70102384);
    assert_eq!(alternative_hash("georgi"), 2612827751);
    assert_eq!(alternative_hash("hello"), 3532604422);
    assert_eq!(alternative_hash("world"), 3523421294);
}

#[test]
fn test_alternative_hash_empty_and_short() {
    assert_eq!(alternative_hash(""), 0);
    assert_eq!(alternative_hash("a"), 97);
    assert_eq!(alternative_hash("ab"), 16685);
    assert_eq!(alternative_hash("abc"), 2853234);
}

#[test]
fn test_hash_by_power_values() {
    assert_eq!(hash_by_power("hello", 131), 792145550);
    assert_eq!(hash_by_power("hello", 171), 3532604422);
    assert_eq!(hash_by_power("hello", 0), 111);
    assert_eq!(hash_by_power("hello", 1), 532);
}

#[test]
fn test_constants() {
    assert_eq!(POWER, 131);
    assert_eq!(ALTERNATIVE_POWER, 171);
    assert_eq!(REHASHER, 718841);
}

#[test]
fn test_rehash() {
    assert_eq!(rehash(0), 718841);
    assert_eq!(rehash(1), 718842);
    assert_eq!(rehash(100), 718941);
    // Wrapping: 4294248455 + 718841 = 4294967296 = 0 (mod 2^32)
    assert_eq!(rehash(4294248455), 0);
}

#[test]
fn test_compare_keys() {
    assert_eq!(compare_keys("abc", "abc"), Ordering::Equal);
    assert_eq!(compare_keys("abc", "abd"), Ordering::Less);
    assert_eq!(compare_keys("abd", "abc"), Ordering::Greater);
    assert_eq!(compare_keys("a", "b"), Ordering::Less);
    assert_eq!(compare_keys("b", "a"), Ordering::Greater);
}

fn main() {}
