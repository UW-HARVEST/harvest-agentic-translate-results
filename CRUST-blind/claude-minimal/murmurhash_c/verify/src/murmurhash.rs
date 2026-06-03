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

    let len: u32 = key.len() as u32;
    let l: usize = (len / 4) as usize;

    let mut h: u32 = seed;
    let mut k: u32;

    let body = &key[..l * 4];
    let tail = &key[l * 4..];

    // for each 4 byte chunk of `key'
    for i in 0..l {
        let chunk_native = u32::from_ne_bytes([
            body[i * 4],
            body[i * 4 + 1],
            body[i * 4 + 2],
            body[i * 4 + 3],
        ]);
        k = htole32(chunk_native);

        // encode next 4 byte chunk of `key'
        k = k.wrapping_mul(c1);
        k = k.rotate_left(r1);
        k = k.wrapping_mul(c2);

        // append to hash
        h ^= k;
        h = h.rotate_left(r2);
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    k = 0;

    // remainder (fall-through like the C switch)
    let rem = len & 3;
    if rem == 3 {
        k ^= (tail[2] as u32) << 16;
    }
    if rem >= 2 {
        k ^= (tail[1] as u32) << 8;
    }
    if rem >= 1 {
        k ^= tail[0] as u32;
        k = k.wrapping_mul(c1);
        k = k.rotate_left(r1);
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
