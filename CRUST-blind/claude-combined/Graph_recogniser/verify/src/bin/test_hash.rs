use Graph_recogniser::hash;

#[test]
fn test_hash_known_values() {
    assert_eq!(hash::hash("stefan"), 4266133291u32);
    assert_eq!(hash::hash("manov"), 2255687513u32);
    assert_eq!(hash::hash("hristo"), 218760243u32);
    assert_eq!(hash::hash("tenchev"), 3026463807u32);
    assert_eq!(hash::hash("dimitar"), 1183767080u32);
    assert_eq!(hash::hash("kajabachev"), 4005941662u32);
    assert_eq!(hash::hash("georgi"), 773914255u32);
    assert_eq!(hash::hash("popov"), 3170694872u32);
    assert_eq!(hash::hash("nikola"), 1416358648u32);
    assert_eq!(hash::hash("yolov"), 1526158221u32);
    assert_eq!(hash::hash("a"), 97u32);
    assert_eq!(hash::hash("b"), 98u32);
    assert_eq!(hash::hash("ab"), 12805u32);
    assert_eq!(hash::hash("ba"), 12935u32);
    assert_eq!(hash::hash("abc"), 1677554u32);
    assert_eq!(hash::hash(""), 0u32);
    assert_eq!(hash::hash("x"), 120u32);
}

#[test]
fn test_alternative_hash_known_values() {
    assert_eq!(hash::alternative_hash("stefan"), 395527203u32);
    assert_eq!(hash::alternative_hash("manov"), 3492875689u32);
    assert_eq!(hash::alternative_hash("hristo"), 995706907u32);
    assert_eq!(hash::alternative_hash("nikola"), 2993889568u32);
    assert_eq!(hash::alternative_hash("a"), 97u32);
    assert_eq!(hash::alternative_hash("b"), 98u32);
    assert_eq!(hash::alternative_hash("ab"), 16685u32);
    assert_eq!(hash::alternative_hash("ba"), 16855u32);
    assert_eq!(hash::alternative_hash("abc"), 2853234u32);
    assert_eq!(hash::alternative_hash(""), 0u32);
}

#[test]
fn test_rehash_known_values() {
    assert_eq!(hash::rehash(hash::hash("stefan")), 4266852132u32);
    assert_eq!(hash::rehash(hash::hash("a")), 718938u32);
    assert_eq!(hash::rehash(hash::hash("b")), 718939u32);
    assert_eq!(hash::rehash(hash::hash("")), 718841u32);
    assert_eq!(hash::rehash(hash::hash("x")), 718961u32);
}

#[test]
fn test_hash_by_power_custom() {
    assert_eq!(hash::hash_by_power("hello", 31), 99162322u32);
    assert_eq!(hash::hash_by_power("hello", 131), 792145550u32);
    assert_eq!(hash::hash_by_power("", 31), 0u32);
}

#[test]
fn test_compare_keys() {
    use std::cmp::Ordering;
    assert_eq!(hash::compare_keys("abc", "abc"), Ordering::Equal);
    assert_eq!(hash::compare_keys("abc", "abd"), Ordering::Less);
    assert_eq!(hash::compare_keys("abd", "abc"), Ordering::Greater);
    assert_eq!(hash::compare_keys("abc", "abcd"), Ordering::Less);
    assert_eq!(hash::compare_keys("abcd", "abc"), Ordering::Greater);
}

#[test]
fn test_constants() {
    assert_eq!(hash::POWER, 131);
    assert_eq!(hash::ALTERNATIVE_POWER, 171);
    assert_eq!(hash::REHASHER, 718841);
    assert_eq!(hash::EMPTY_KEY, None);
    assert_eq!(hash::EMPTY_DATA, None);
}

fn main() {}
