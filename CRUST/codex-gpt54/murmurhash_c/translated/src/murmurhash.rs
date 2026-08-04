pub const MURMURHASH_VERSION: &str = "0.2.0";
pub const MURMURHASH_HAS_HTOLE32: usize = 1;
pub fn htole32(v: u32) -> u32 {
    v.to_le()
}
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    let c1 = 0xcc9e2d51u32;
    let c2 = 0x1b873593u32;
    let r1 = 15u32;
    let r2 = 13u32;
    let m = 5u32;
    let n = 0xe6546b64u32;

    let len = key.len() as u32;
    let mut h = seed;

    let body_len = key.len() / 4;
    let tail = &key[body_len * 4..];

    for chunk in key[..body_len * 4].chunks_exact(4) {
        let mut k = htole32(u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        k = k.wrapping_mul(c1);
        k = k.rotate_left(r1);
        k = k.wrapping_mul(c2);

        h ^= k;
        h = h.rotate_left(r2);
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    let mut k = 0u32;
    match len & 3 {
        3 => {
            k ^= (tail[2] as u32) << 16;
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
            k = k.wrapping_mul(c1);
            k = k.rotate_left(r1);
            k = k.wrapping_mul(c2);
            h ^= k;
        }
        2 => {
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
            k = k.wrapping_mul(c1);
            k = k.rotate_left(r1);
            k = k.wrapping_mul(c2);
            h ^= k;
        }
        1 => {
            k ^= tail[0] as u32;
            k = k.wrapping_mul(c1);
            k = k.rotate_left(r1);
            k = k.wrapping_mul(c2);
            h ^= k;
        }
        _ => {}
    }

    h ^= len;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}
