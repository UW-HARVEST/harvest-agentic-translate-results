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
    let len = key.len() as u32;
    let mut h: u32 = seed;
    let l = (len / 4) as usize;

    // body: process 4-byte chunks
    for i in 0..l {
        let off = i * 4;
        let chunk = u32::from_le_bytes([
            key[off],
            key[off + 1],
            key[off + 2],
            key[off + 3],
        ]);
        let mut k = htole32(chunk);
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);

        h ^= k;
        h = (h << r2) | (h >> (32 - r2));
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    let tail_off = l * 4;
    let mut k: u32 = 0;

    // remainder (note the C switch falls through)
    let rem = (len & 3) as usize;
    if rem == 3 {
        k ^= (key[tail_off + 2] as u32) << 16;
    }
    if rem >= 2 {
        k ^= (key[tail_off + 1] as u32) << 8;
    }
    if rem >= 1 {
        k ^= key[tail_off] as u32;
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
