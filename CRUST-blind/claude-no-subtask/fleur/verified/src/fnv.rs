pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // C does: h ^= buf[i]; where buf[i] is signed char; sign-extended to uint64_t.
        // Mimic the C behavior by sign-extending the byte through i8 -> i64 -> u64.
        let signed = b as i8 as i64 as u64;
        h ^= signed;
    }
    h
}

#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    // The C code prints bytes in reverse order: hexrepr[7] down to hexrepr[0]
    // hexrepr is a union with the u64 h, on a little-endian machine
    // hexrepr[0] is the LSB. So getDigest prints from MSB to LSB - i.e. big-endian hex.
    let bytes = h.to_le_bytes();
    let mut s = String::with_capacity(16);
    for i in (0..8).rev() {
        s.push_str(&format!("{:02x}", bytes[i]));
    }
    s
}
