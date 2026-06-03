pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // C: h ^= buf[i] where buf[i] is signed char.
        // When the signed char is negative, integer promotion sign-extends to int (32-bit),
        // and then the `^=` against uint64_t implicitly converts that int to uint64_t,
        // which sign-extends further to 64 bits. Reproduce that behavior here.
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}
#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    let bytes = h.to_le_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0]
    )
}
