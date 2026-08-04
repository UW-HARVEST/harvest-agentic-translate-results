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
pub fn getDigest(h: u64) -> String {
    // The C code prints hexrepr[7]..hexrepr[0], which on little-endian
    // is the most-significant byte first; this is equivalent to printing
    // the u64 in big-endian hex.
    format!("{:016x}", h)
}
