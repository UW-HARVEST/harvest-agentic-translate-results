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
    let mut res: Hash = 0;
    for byte in key.bytes() {
        res = res.wrapping_mul(power).wrapping_add(byte as Hash);
    }
    res
}
/// Computes the default hash using `POWER`.
pub fn hash(key: Key) -> Hash {
    hash_by_power(key, POWER)
}
/// Computes an alternative hash using `ALTERNATIVE_POWER`.
pub fn alternative_hash(key: Key) -> Hash {
    hash_by_power(key, ALTERNATIVE_POWER)
}
/// Compares two keys lexicographically.
pub fn compare_keys(k1: Key, k2: Key) -> std::cmp::Ordering {
    k1.cmp(k2)
}
/// Rehashes an existing hash value.
pub fn rehash(h: Hash) -> Hash {
    h.wrapping_add(REHASHER)
}
