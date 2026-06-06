pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // C does `h ^= buf[i]` where buf[i] is `char` (signed on most platforms).
        // When XOR'd with a uint64_t, signed char is promoted to int (sign-extended)
        // then converted to uint64_t (which wraps). Replicate that behavior here.
        let signed_byte = b as i8 as i64 as u64;
        h ^= signed_byte;
    }
    h
}

#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    // C's getDigest writes the bytes of the hash (as a union) in reverse byte order:
    //   "%.2x%.2x%.2x%.2x%.2x%.2x%.2x%.2x" applied to hexrepr[7..0].
    // On little-endian platforms, hexrepr[7] is the most-significant byte, so the
    // resulting string is the standard big-endian hex representation.
    format!("{:016x}", h)
}
