pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

pub fn fnv1(buf: &[u8]) -> u64 {
    let mut h: u64 = FNV_OFFSET_BASIS;
    for &b in buf {
        h = h.wrapping_mul(FNV_PRIME);
        // C reads buf as char (signed on most platforms), then XORs with the
        // (sign-extended via integer promotion) value. Match that here.
        let signed_b = b as i8 as i64 as u64;
        h ^= signed_b;
    }
    h
}

#[allow(non_snake_case)]
pub fn getDigest(h: u64) -> String {
    format!("{:016x}", h)
}
