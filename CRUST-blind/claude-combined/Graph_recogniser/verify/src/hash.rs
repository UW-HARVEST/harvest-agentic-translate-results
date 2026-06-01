pub type Hash = u32;
pub type Key = &'static str;
pub const EMPTY_KEY: Option<Key> = None;
pub type Data = &'static str;
pub const EMPTY_DATA: Option<Data> = None;
pub const POWER: Hash = 131;
pub const ALTERNATIVE_POWER: Hash = 171;
pub const REHASHER: Hash = 718841;
/// Computes a hash using the given power value.
pub fn hash_by_power(key: Key, power: Hash) -> Hash {
    hash_by_power_any(key, power)
}
/// Computes the default hash using `POWER`.
pub fn hash(key: Key) -> Hash {
    hash_by_power_any(key, POWER)
}
/// Computes an alternative hash using `ALTERNATIVE_POWER`.
pub fn alternative_hash(key: Key) -> Hash {
    hash_by_power_any(key, ALTERNATIVE_POWER)
}
/// Compares two keys lexicographically.
pub fn compare_keys(k1: Key, k2: Key) -> std::cmp::Ordering {
    k1.cmp(k2)
}
/// Rehashes an existing hash value.
pub fn rehash(h: Hash) -> Hash {
    h.wrapping_add(REHASHER)
}

/// Internal helper that accepts any `&str` lifetime. Mirrors the C implementation
/// where each byte is multiplied into the running hash with wrapping arithmetic.
pub(crate) fn hash_by_power_any(key: &str, power: Hash) -> Hash {
    let mut res: Hash = 0;
    for &b in key.as_bytes() {
        if b == 0 {
            break;
        }
        res = res.wrapping_mul(power).wrapping_add(b as Hash);
    }
    res
}

/// Internal helper to hash any `&str` using the default power.
pub(crate) fn hash_any(key: &str) -> Hash {
    hash_by_power_any(key, POWER)
}
