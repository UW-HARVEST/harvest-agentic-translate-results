pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // C XORs with signed char promoted to int, then assigns to uint64.
        // For values 0..=255, the XOR result with the lower bits is the same
        // as XORing with the signed-extended value's low bits in modular arithmetic.
        // The C code does: h ^= buf[i]; where buf[i] is a char (which may be signed).
        // If signed and negative, the int promotion sign-extends, then the XOR
        // with uint64_t affects all upper bits as well.
        // To match C behavior exactly, we replicate the sign extension:
        let sign_extended = (b as i8) as i64 as u64;
        h ^= sign_extended;
    }
    h
}

#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    // C does: sprintf(str, "%.2x%.2x%.2x%.2x%.2x%.2x%.2x%.2x",
    //   hexrepr[7], hexrepr[6], ..., hexrepr[0]);
    // Where hexrepr is the 8 bytes of h in little-endian (union with uint64_t).
    // Printing bytes [7] down to [0] gives the big-endian representation.
    // So this is just hex of h in big-endian (most-significant byte first).
    format!("{:016x}", h)
}
