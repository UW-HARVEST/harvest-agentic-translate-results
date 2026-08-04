pub const MURMURHASH_VERSION: &str = "0.2.0";
pub const MURMURHASH_HAS_HTOLE32: usize = 1;

pub fn htole32(v: u32) -> u32 {
    #[cfg(target_endian = "big")]
    {
        ((v & 0xFF00_0000) >> 24)
            | ((v & 0x00FF_0000) >> 8)
            | ((v & 0x0000_FF00) << 8)
            | ((v & 0x0000_00FF) << 24)
    }
    #[cfg(target_endian = "little")]
    {
        v
    }
}

pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    let c1: u32 = 0xcc9e_2d51;
    let c2: u32 = 0x1b87_3593;
    let r1: u32 = 15;
    let r2: u32 = 13;
    let m: u32 = 5;
    let n: u32 = 0xe654_6b64;
    let len: u32 = key.len() as u32;
    let mut h: u32 = seed;
    let mut k: u32;

    let l: usize = (len / 4) as usize; // chunk length in 4-byte units
    let body_end = l * 4;
    let tail = &key[body_end..];

    // body: process each 4-byte chunk
    for i in 0..l {
        let offset = i * 4;
        let chunk = u32::from_ne_bytes([
            key[offset],
            key[offset + 1],
            key[offset + 2],
            key[offset + 3],
        ]);
        k = htole32(chunk);

        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);

        h ^= k;
        h = (h << r2) | (h >> (32 - r2));
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    k = 0;

    // remainder (tail) — fall-through behavior matches the C switch
    let remainder = (len & 3) as usize;
    if remainder == 3 {
        k ^= (tail[2] as u32) << 16;
    }
    if remainder >= 2 {
        k ^= (tail[1] as u32) << 8;
    }
    if remainder >= 1 {
        k ^= tail[0] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    }

    h ^= len;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;

    h
}
