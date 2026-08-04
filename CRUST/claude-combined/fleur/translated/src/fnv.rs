pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        h ^= b as u64;
    }
    h
}
#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    // C code prints bytes from index 7 down to 0 from a union
    // (uint64_t h | unsigned char hexrepr[8]) on little-endian systems.
    // Index 7 is the most significant byte; we print as big-endian hex.
    format!("{:016x}", h)
}
