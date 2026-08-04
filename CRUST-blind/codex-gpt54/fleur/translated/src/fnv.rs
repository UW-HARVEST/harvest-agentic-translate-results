pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    for &byte in buf {
        h = h.wrapping_mul(FNV_PRIME);
        h ^= u64::from(byte);
    }
    h
}
pub fn getDigest(h: u64) -> String {
    format!("{h:016x}")
}
