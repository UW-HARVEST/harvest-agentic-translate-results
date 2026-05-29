// In non-test compilation, all `#[test]` functions are stripped and `main()`
// is empty, so the imports and helpers below appear unused. Silence those
// warnings here.
#![allow(unused_imports, dead_code)]

use clhash::clhash::{
    clhash, get_random_key_for_clhash, ClHasher, RANDOM_64BITWORDS_NEEDED_FOR_CLHASH,
    RANDOM_BYTES_NEEDED_FOR_CLHASH,
};

// Helper: read a u64 little-endian word at index `i` (in 64-bit words) from `key`.
fn key_word(key: &[u8], i: usize) -> u64 {
    let off = i * 8;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&key[off..off + 8]);
    u64::from_le_bytes(buf)
}

// =========== constants ===========

#[test]
fn test_random_constants() {
    assert_eq!(RANDOM_64BITWORDS_NEEDED_FOR_CLHASH, 133);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 133 * 8);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 1064);
}

// =========== get_random_key_for_clhash ===========

#[test]
fn test_get_random_key_size() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(key.len(), RANDOM_BYTES_NEEDED_FOR_CLHASH);
    assert_eq!(key.len(), 1064);
}

#[test]
fn test_get_random_key_seeds_1() {
    // Verified via C harness: get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530)
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(key_word(&key, 0), 0xA2C0401D027633A7);
    assert_eq!(key_word(&key, 1), 0xDE0F4D3CF8213AA5);
    assert_eq!(key_word(&key, 127), 0xD4E7317C0520C7D0);
    assert_eq!(key_word(&key, 128), 0xE78A3B6E6A02A590);
    assert_eq!(key_word(&key, 129), 0x4E0D3FDFCCBBBEBE);
    assert_eq!(key_word(&key, 130), 0xAD8ECEDB9CF10BA9);
    assert_eq!(key_word(&key, 131), 0x5240D4A6AB8C2E64);
    assert_eq!(key_word(&key, 132), 0x0ECEA9811C29EFBD);
}

#[test]
fn test_get_random_key_seeds_2() {
    // Verified via C harness: get_random_key_for_clhash(1, 2)
    let key = get_random_key_for_clhash(1, 2);
    assert_eq!(key_word(&key, 0), 0x0000000000800025);
    assert_eq!(key_word(&key, 1), 0x0000000002040083);
    assert_eq!(key_word(&key, 2), 0x00004000020C2460);
    assert_eq!(key_word(&key, 3), 0x0000C00002108D21);
    assert_eq!(key_word(&key, 128), 0x93DE1C3F083C00D3);
    assert_eq!(key_word(&key, 129), 0x0921F56AEECA6854);
    assert_eq!(key_word(&key, 132), 0xACDBFEBA0D919FFC);
}

#[test]
fn test_get_random_key_determinism() {
    // Same seeds should yield identical keys.
    let k1 = get_random_key_for_clhash(42, 43);
    let k2 = get_random_key_for_clhash(42, 43);
    assert_eq!(k1, k2);
}

#[test]
fn test_get_random_key_different_seeds() {
    // Different seeds should yield different keys.
    let k1 = get_random_key_for_clhash(42, 43);
    let k2 = get_random_key_for_clhash(43, 42);
    assert_ne!(k1, k2);
}

// =========== clhash: short string path ===========
// All expected values were obtained by running c_src/harness.c (compiled from
// the original C implementation) and reading its output.

#[test]
fn test_clhash_empty_string() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b""), 0x0000000000000000);
}

#[test]
fn test_clhash_one_byte() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"a"), 0x4EA7E19B3349B1B4);
}

#[test]
fn test_clhash_two_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"ab"), 0x7986C3E43CB8ED61);
}

#[test]
fn test_clhash_my_dog() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"my dog"), 0x0B394C2019976F03);
}

#[test]
fn test_clhash_my_cat() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"my cat"), 0x1CFBE7A3B913D46F);
}

#[test]
fn test_clhash_my_dog_repeats() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h1 = clhash(&key, b"my dog");
    let h2 = clhash(&key, b"my dog");
    assert_eq!(h1, h2);
    assert_eq!(h1, 0x0B394C2019976F03);
}

#[test]
fn test_clhash_my_dog_vs_my_cat() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h1 = clhash(&key, b"my dog");
    let h2 = clhash(&key, b"my cat");
    assert_ne!(h1, h2);
}

#[test]
fn test_clhash_8_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"abcdefgh"), 0xD6D5AE7C87073F93);
}

#[test]
fn test_clhash_9_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"abcdefghi"), 0xBAA843538A6F4557);
}

#[test]
fn test_clhash_16_zeros() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let zeros = [0u8; 16];
    assert_eq!(clhash(&key, &zeros), 0x30B6759619B66148);
}

#[test]
fn test_clhash_seeds_1_2_hello() {
    let key = get_random_key_for_clhash(1, 2);
    assert_eq!(clhash(&key, b"hello"), 0xC0D0582EF7E5E96D);
}

#[test]
fn test_clhash_seeds_1_2_hello_world() {
    let key = get_random_key_for_clhash(1, 2);
    assert_eq!(clhash(&key, b"hello, world!"), 0xBD1B6D44DE5FF1E9);
}

// =========== clhash: deterministic key (rs[k] = (char)(1-k)) ===========

fn build_deterministic_key() -> Vec<u8> {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        // C: rs[k] = (char)(1 - k)
        // The cast to (char) and then to unsigned for our byte slice -> just
        // take the value modulo 256 in two's complement:
        let v = (1i32 - k as i32) as i8 as u8;
        rs[k] = v;
    }
    rs
}

#[test]
fn test_deterministic_key_first_bytes() {
    // Verified via C harness: KEY3 raw[0..16]
    let rs = build_deterministic_key();
    let expected: [u8; 16] = [
        0x01, 0x00, 0xff, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf7, 0xf6, 0xf5, 0xf4, 0xf3,
        0xf2,
    ];
    assert_eq!(&rs[..16], &expected);
}

#[test]
fn test_deterministic_clhash_empty() {
    let rs = build_deterministic_key();
    assert_eq!(clhash(&rs, b""), 0x0000000000000000);
}

#[test]
fn test_deterministic_clhash_x() {
    let rs = build_deterministic_key();
    assert_eq!(clhash(&rs, b"x"), 0x6AF7AC0B8A5EF2FC);
}

#[test]
fn test_deterministic_clhash_yz() {
    let rs = build_deterministic_key();
    assert_eq!(clhash(&rs, b"yz"), 0x33CAE5CE31527BF4);
}

#[test]
fn test_deterministic_clhash_12_bytes() {
    let rs = build_deterministic_key();
    assert_eq!(clhash(&rs, b"ABCDEFGHIJKL"), 0x283A8D8D7A098D36);
}

#[test]
fn test_deterministic_clhash_8_bytes() {
    let rs = build_deterministic_key();
    assert_eq!(clhash(&rs, b"ABCDEFGH"), 0xC49E131890DF623C);
}

#[test]
fn test_deterministic_clhash_zeros_7() {
    let rs = build_deterministic_key();
    let z = [0u8; 7];
    assert_eq!(clhash(&rs, &z), 0xEE0C2DF4F75FDA3A);
}

#[test]
fn test_deterministic_clhash_zeros_8() {
    let rs = build_deterministic_key();
    let z = [0u8; 8];
    assert_eq!(clhash(&rs, &z), 0x3CD1DD0B19BF7FF9);
}

#[test]
fn test_deterministic_clhash_zeros_9() {
    let rs = build_deterministic_key();
    let z = [0u8; 9];
    assert_eq!(clhash(&rs, &z), 0xE60A01D6C7609F18);
}

#[test]
fn test_deterministic_clhash_zeros_15() {
    let rs = build_deterministic_key();
    let z = [0u8; 15];
    assert_eq!(clhash(&rs, &z), 0x38D2CB1A01A0DD68);
}

#[test]
fn test_deterministic_clhash_zeros_16() {
    let rs = build_deterministic_key();
    let z = [0u8; 16];
    assert_eq!(clhash(&rs, &z), 0x47B2F63802BE7614);
}

#[test]
fn test_deterministic_clhash_zeros_17() {
    let rs = build_deterministic_key();
    let z = [0u8; 17];
    assert_eq!(clhash(&rs, &z), 0x82C6E720C186D0FE);
}

// =========== clhash: long string path ===========

#[test]
fn test_clhash_1024_byte_pattern() {
    // i & 0xff
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1024];
    for i in 0..1024 {
        buf[i] = (i & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0xA49698CF93FBF312);
}

#[test]
fn test_clhash_2048_byte_pattern() {
    // (i*7+3) & 0xff
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 2048];
    for i in 0..2048 {
        buf[i] = ((i * 7 + 3) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0x719661B2728F54F9);
}

#[test]
fn test_clhash_1027_byte_pattern() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1027];
    for i in 0..1027 {
        buf[i] = ((i * 11 + 5) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0x8D712E61EEDACB32);
}

#[test]
fn test_clhash_1500_byte_pattern() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1500];
    for i in 0..1500 {
        buf[i] = ((i * 13 + 1) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0x82F73DDA8E6FCAF6);
}

#[test]
fn test_clhash_1032_byte_pattern() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1032];
    for i in 0..1032 {
        buf[i] = ((i * 5 + 9) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0x72C8B32F3563D236);
}

#[test]
fn test_clhash_1025_byte_pattern() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1025];
    for i in 0..1025 {
        buf[i] = ((i * 17 + 21) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0xEA5BB6424B4EC125);
}

// =========== clhash: short/long boundary cases ===========

#[test]
fn test_clhash_seed_42_43_1024_bytes() {
    let key = get_random_key_for_clhash(42, 43);
    let mut buf = vec![0u8; 1024];
    for i in 0..1024 {
        buf[i] = ((i + 99) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf), 0x7F94E95C5BAE2220);
}

#[test]
fn test_clhash_seed_42_43_1023_bytes() {
    let key = get_random_key_for_clhash(42, 43);
    // Same source buffer; only first 1023 bytes used.
    let mut buf = vec![0u8; 1024];
    for i in 0..1024 {
        buf[i] = ((i + 99) & 0xff) as u8;
    }
    assert_eq!(clhash(&key, &buf[..1023]), 0xEB61FB6405F39068);
}

#[test]
fn test_clhash_seed_42_43_1025_bytes() {
    let key = get_random_key_for_clhash(42, 43);
    // C harness uses the same 1024-byte pattern but length 1025: byte at index 1024
    // is read from one-past-end of the buf. The C code allocated buf[1024] and only
    // initialized 1024 bytes, then passed length 1025 -> reads one byte beyond end.
    // To reproduce the harness exactly, allocate 1025 bytes; compare what bytes it
    // saw. In our harness, `buf[1024]` was uninitialized but stack-allocated; the
    // expected value 0xF0E86EFEBA9A3A12 came from that exact run, which is not
    // deterministic. Skip exact value -- instead test the property: hashing 1025
    // bytes from the same buffer produces a value (no panic), and is independent
    // of seeds being applied correctly.
    let mut buf = vec![0u8; 1025];
    for i in 0..1024 {
        buf[i] = ((i + 99) & 0xff) as u8;
    }
    buf[1024] = 0; // explicit zero
    let h = clhash(&key, &buf);
    // Sanity: should differ from the 1024-byte hash and the 1023-byte hash.
    assert_ne!(h, 0x7F94E95C5BAE2220);
    assert_ne!(h, 0xEB61FB6405F39068);
}

// =========== Property-based verification ===========

#[test]
fn test_flip_bit_changes_hash_short() {
    // Mirrors clhashtest from c_src/tests/unit.c: flipping any bit in a small
    // input must produce a different hash.
    let rs = build_deterministic_key();
    for bit in 0..64u32 {
        let min_len = ((bit as usize) + 8) / 8;
        for length in min_len..=8 {
            let mut x: u64 = 0;
            let bytes_orig = x.to_le_bytes();
            let orig = clhash(&rs, &bytes_orig[..length]);
            x ^= 1u64 << bit;
            let bytes_flip = x.to_le_bytes();
            let flip = clhash(&rs, &bytes_flip[..length]);
            assert_ne!(flip, orig, "bit={} length={}", bit, length);
            x ^= 1u64 << bit;
            let bytes_back = x.to_le_bytes();
            let back = clhash(&rs, &bytes_back[..length]);
            assert_eq!(back, orig);
        }
    }
}

#[test]
fn test_avalanche_xor_invariant_short() {
    // For inputs whose length is at most sizeof(uint64_t) = 8 bytes, the
    // clhash construction is linear in the input bits. So if we have two
    // inputs `a` and `b` of the same length (<= 8), and we flip the same
    // bit in each, the XOR of the original and the flipped hash must be
    // the same for both `a` and `b`. This is the avalanche-test invariant
    // from clhashavalanchetest.
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        // C: rs[k] = k+1-k*k (with int truncation to char)
        let v = (k as i32 + 1 - (k as i32) * (k as i32)) as i8 as u8;
        rs[k] = v;
    }

    for bytelength in 1..=8usize {
        for whichcase in 0..32u32 {
            // small subset for speed
            let mut a = vec![0u8; bytelength];
            let mut b = vec![0u8; bytelength];
            for k in 0..bytelength {
                a[k] = whichcase as u8;
                b[k] = whichcase.wrapping_add(35) as u8;
            }
            let orig_a = clhash(&rs, &a);
            let orig_b = clhash(&rs, &b);
            for z in 0..(8 * bytelength) {
                let byte = z >> 3;
                let bit = z & 0x7;
                a[byte] ^= 1u8 << bit;
                let new_a = clhash(&rs, &a);
                a[byte] ^= 1u8 << bit;

                b[byte] ^= 1u8 << bit;
                let new_b = clhash(&rs, &b);
                b[byte] ^= 1u8 << bit;

                assert_ne!(orig_a, new_a);
                assert_ne!(orig_b, new_b);

                assert_eq!(orig_a ^ new_a, orig_b ^ new_b);
            }
        }
    }
}

#[test]
fn test_collision_test_eik_list() {
    // Mirrors clhashcollisiontest from unit.c: changing the very last byte
    // of a long message must change the hash.
    const NUM_TRIALS: usize = 4;
    const CLNH_NUM_BYTES_PER_BLOCK: usize = 1024;
    const KEY_OFFSET: u8 = 0x63;

    let mut k = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for j2 in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        k[j2] = ((j2 as u32 + KEY_OFFSET as u32) & 0xff) as u8;
    }

    for i in 1..NUM_TRIALS {
        for j in 1..=8usize {
            let mlen = i * CLNH_NUM_BYTES_PER_BLOCK + j;
            let mut m = vec![0u8; mlen];
            for j2 in 0..mlen {
                m[j2] = (j2 & 0xff) as u8;
            }
            let actual1 = clhash(&k, &m);
            m[mlen - 1] = m[mlen - 1].wrapping_add(1);
            let actual2 = clhash(&k, &m);
            assert_ne!(actual1, actual2, "i={}, j={}, mlen={}", i, j, mlen);
        }
    }
}

// =========== ClHasher wrapper ===========

#[test]
fn test_clhasher_matches_clhash() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let hasher = ClHasher::new(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(hasher.hash(b"my dog"), clhash(&key, b"my dog"));
    assert_eq!(hasher.hash(b"my dog"), 0x0B394C2019976F03);
    assert_eq!(hasher.hash(b"my cat"), 0x1CFBE7A3B913D46F);
    assert_eq!(hasher.hash(b""), 0x0000000000000000);
    assert_eq!(hasher.hash(b"abcdefgh"), 0xD6D5AE7C87073F93);
}

#[test]
fn test_clhasher_long_data() {
    let hasher = ClHasher::new(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let mut buf = vec![0u8; 1024];
    for i in 0..1024 {
        buf[i] = (i & 0xff) as u8;
    }
    assert_eq!(hasher.hash(&buf), 0xA49698CF93FBF312);
}

#[test]
fn test_clhasher_drop() {
    // Ensure ClHasher's Drop is well-behaved by constructing and dropping it.
    let h = ClHasher::new(1, 2);
    assert_eq!(h.hash(b"hello"), 0xC0D0582EF7E5E96D);
    drop(h);
}

fn main() {}
