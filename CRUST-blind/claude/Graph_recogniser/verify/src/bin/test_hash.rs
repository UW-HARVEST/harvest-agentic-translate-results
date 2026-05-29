use Graph_recogniser::hash::{
    self, alternative_hash, compare_keys, hash_by_power, rehash, ALTERNATIVE_POWER, EMPTY_DATA,
    EMPTY_KEY, POWER, REHASHER,
};
use std::cmp::Ordering;

#[test]
fn test_constants() {
    assert_eq!(POWER, 131);
    assert_eq!(ALTERNATIVE_POWER, 171);
    assert_eq!(REHASHER, 718841);
    assert!(EMPTY_KEY.is_none());
    assert!(EMPTY_DATA.is_none());
}

#[test]
fn test_hash_empty_string() {
    // C: hash("") = 0
    assert_eq!(hash::hash(""), 0);
    assert_eq!(alternative_hash(""), 0);
}

#[test]
fn test_hash_basic_strings() {
    // Values computed by running the C implementation:
    assert_eq!(hash::hash("a"), 97);
    assert_eq!(hash::hash("ab"), 12805);
    assert_eq!(hash::hash("abc"), 1677554);
    assert_eq!(hash::hash("stefan"), 4266133291);
    assert_eq!(hash::hash("hristo"), 218760243);
    assert_eq!(hash::hash("dimitar"), 1183767080);
    assert_eq!(hash::hash("georgi"), 773914255);
    assert_eq!(hash::hash("stanislav"), 705159857);
    assert_eq!(hash::hash("nikola"), 1416358648);
    assert_eq!(hash::hash("andrei"), 3850980359);
    assert_eq!(hash::hash("iulen"), 1122614009);
    assert_eq!(hash::hash("iasen"), 1077772316);
    assert_eq!(hash::hash("samuele"), 41638808);
    assert_eq!(hash::hash("henning"), 1031699281);
    assert_eq!(hash::hash("javier"), 3680815763);
    assert_eq!(hash::hash("key1"), 242294898);
    assert_eq!(hash::hash("Hello, World!"), 3015313285);
}

#[test]
fn test_alternative_hash_values() {
    // Values from running C implementation:
    assert_eq!(alternative_hash("a"), 97);
    assert_eq!(alternative_hash("ab"), 16685);
    assert_eq!(alternative_hash("abc"), 2853234);
    assert_eq!(alternative_hash("stefan"), 395527203);
    assert_eq!(alternative_hash("hristo"), 995706907);
    assert_eq!(alternative_hash("dimitar"), 70102384);
    assert_eq!(alternative_hash("georgi"), 2612827751);
    assert_eq!(alternative_hash("stanislav"), 2605642241);
    assert_eq!(alternative_hash("nikola"), 2993889568);
    assert_eq!(alternative_hash("andrei"), 569524503);
    assert_eq!(alternative_hash("iulen"), 172675385);
    assert_eq!(alternative_hash("iasen"), 72875852);
    assert_eq!(alternative_hash("samuele"), 2367729560);
    assert_eq!(alternative_hash("henning"), 347383657);
    assert_eq!(alternative_hash("javier"), 4069234059);
    assert_eq!(alternative_hash("key1"), 537996658);
    assert_eq!(alternative_hash("Hello, World!"), 2407992709);
}

#[test]
fn test_hash_by_power_custom_power() {
    // Values from running C implementation:
    assert_eq!(hash_by_power("hello", 7), 290506);
    assert_eq!(hash_by_power("hello", 31), 99162322);
    assert_eq!(hash_by_power("", 31), 0);
}

#[test]
fn test_hash_by_power_with_default_power() {
    assert_eq!(hash_by_power("stefan", POWER), 4266133291);
    assert_eq!(hash_by_power("stefan", ALTERNATIVE_POWER), 395527203);
}

#[test]
fn test_rehash_values() {
    assert_eq!(rehash(0), 718841);
    assert_eq!(rehash(97), 718938);
    assert_eq!(rehash(12805), 731646);
    assert_eq!(rehash(1677554), 2396395);
    // Wrapping behavior: REHASHER added with u32 wrap.
    let big: u32 = u32::MAX - 100;
    assert_eq!(rehash(big), big.wrapping_add(REHASHER));
}

#[test]
fn test_rehash_of_hash() {
    assert_eq!(rehash(hash::hash("stefan")), 4266852132);
    assert_eq!(rehash(hash::hash("hristo")), 219479084);
    assert_eq!(rehash(hash::hash("")), 718841);
}

#[test]
fn test_compare_keys_equal() {
    assert_eq!(compare_keys("foo", "foo"), Ordering::Equal);
    assert_eq!(compare_keys("", ""), Ordering::Equal);
    assert_eq!(compare_keys("stefan", "stefan"), Ordering::Equal);
}

#[test]
fn test_compare_keys_less() {
    assert_eq!(compare_keys("a", "b"), Ordering::Less);
    assert_eq!(compare_keys("", "a"), Ordering::Less);
    assert_eq!(compare_keys("hristo", "stefan"), Ordering::Less);
}

#[test]
fn test_compare_keys_greater() {
    assert_eq!(compare_keys("b", "a"), Ordering::Greater);
    assert_eq!(compare_keys("stefan", ""), Ordering::Greater);
    assert_eq!(compare_keys("z", "a"), Ordering::Greater);
}

#[test]
fn test_hash_long_string() {
    // Computed via the C implementation:
    assert_eq!(hash::hash("Hello, World!"), 3015313285);
    assert_eq!(alternative_hash("Hello, World!"), 2407992709);
}

fn main() {}
