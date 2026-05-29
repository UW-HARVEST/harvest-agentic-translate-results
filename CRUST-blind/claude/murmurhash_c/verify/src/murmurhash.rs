pub const MURMURHASH_VERSION: &str = "0.2.0";
pub const MURMURHASH_HAS_HTOLE32: usize = 1;
pub fn htole32(v: u32) -> u32 {
    v.to_le()
}
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    let c1: u32 = 0xcc9e2d51;
    let c2: u32 = 0x1b873593;
    let r1: u32 = 15;
    let r2: u32 = 13;
    let m: u32 = 5;
    let n: u32 = 0xe6546b64;
    let mut h: u32 = seed;
    let mut k: u32;

    let len = key.len() as u32;
    let l = (len / 4) as usize; // chunk length (number of 4-byte chunks)
    let body_end = l * 4;

    // for each 4 byte chunk of key
    for i in 0..l {
        let off = i * 4;
        // little-endian read of 4 bytes
        k = (key[off] as u32)
            | ((key[off + 1] as u32) << 8)
            | ((key[off + 2] as u32) << 16)
            | ((key[off + 3] as u32) << 24);
        k = htole32(k);

        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);

        h ^= k;
        h = (h << r2) | (h >> (32 - r2));
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    k = 0;
    let tail = &key[body_end..];

    // remainder - C code uses fall-through
    let rem = (len & 3) as usize;
    if rem == 3 {
        k ^= (tail[2] as u32) << 16;
        k ^= (tail[1] as u32) << 8;
        k ^= tail[0] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    } else if rem == 2 {
        k ^= (tail[1] as u32) << 8;
        k ^= tail[0] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    } else if rem == 1 {
        k ^= tail[0] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    }

    h ^= len;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;

    h
}
