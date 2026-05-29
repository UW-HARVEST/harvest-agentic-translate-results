pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // The C code does `h ^= buf[i]` where buf is `char *`. On most platforms
        // `char` is signed, and when XORing with a uint64_t the byte gets sign-extended.
        // We replicate that behavior here for compatibility with C-produced hashes.
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}
pub fn getDigest(h: u64) -> String {
    // The C code interprets the hash as a union with hexrepr[8] little-endian
    // and prints from byte 7 down to byte 0, which yields the big-endian hex
    // representation of the 64-bit integer.
    let bytes = h.to_le_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0]
    )
}
