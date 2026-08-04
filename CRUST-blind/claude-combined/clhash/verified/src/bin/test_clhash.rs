use clhash::clhash::{
    clhash, get_random_key_for_clhash, ClHasher, RANDOM_64BITWORDS_NEEDED_FOR_CLHASH,
    RANDOM_BYTES_NEEDED_FOR_CLHASH,
};

fn det_key() -> Vec<u8> {
    // Mirrors C unit test: rs[k] = k+1-k*k (truncated to i8)
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        let v: i32 = (k as i32) + 1 - (k as i32) * (k as i32);
        rs[k] = v as u8;
    }
    rs
}

fn det_key2() -> Vec<u8> {
    // Mirrors C clhashtest: rs[k] = (char)(1-k)
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        let v: i32 = 1 - (k as i32);
        rs[k] = v as u8;
    }
    rs
}

#[test]
fn test_constants() {
    assert_eq!(RANDOM_64BITWORDS_NEEDED_FOR_CLHASH, 133);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 133 * 8);
}

#[test]
fn test_my_dog_known_value() {
    // Exact value from C: get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530),
    // then clhash(random, "my dog", 6) == 808761308841733891 (decimal).
    let random = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(random.len(), RANDOM_BYTES_NEEDED_FOR_CLHASH);
    let h = clhash(&random, b"my dog");
    assert_eq!(h, 808761308841733891);
}

#[test]
fn test_my_dog_vs_my_cat() {
    let random = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h1 = clhash(&random, b"my dog");
    let h2 = clhash(&random, b"my cat");
    let h3 = clhash(&random, b"my dog");
    assert_eq!(h1, h3);
    assert_ne!(h1, h2);
}

#[test]
fn test_empty_input() {
    // Computed via C helper: empty input → 0
    let rs = det_key();
    let h = clhash(&rs, b"");
    assert_eq!(h, 0);
}

#[test]
fn test_short_input_5_bytes() {
    // Computed via C helper: input = bytes 0x00 0x01 0x02 0x03 0x04 → 8589201130733895532
    let rs = det_key();
    let data = [0u8, 1, 2, 3, 4];
    let h = clhash(&rs, &data);
    assert_eq!(h, 8589201130733895532);
}

#[test]
fn test_short_input_1_byte() {
    // Computed via C helper: input = single byte 0x00 → 16179631926982168776
    let rs = det_key();
    let data = [0u8];
    let h = clhash(&rs, &data);
    assert_eq!(h, 16179631926982168776);
}

#[test]
fn test_seeded_key_first_word() {
    // The xorshift128+ key generator starts with state1=0x23a23cf5033c3c81,
    // state2=0xb3816f6a2c68e530. The first generated word is computed from
    // the C helper. We verify through behavior (clhash output) rather than
    // checking individual words; that is already done in test_my_dog_known_value.
    // Here also assert determinism: same seeds → same key bytes.
    let r1 = get_random_key_for_clhash(1, 2);
    let r2 = get_random_key_for_clhash(1, 2);
    assert_eq!(r1, r2);
    let r3 = get_random_key_for_clhash(2, 1);
    assert_ne!(r1, r3);
}

#[test]
fn test_clhasher_struct() {
    let h = ClHasher::new(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h_dog = h.hash("my dog");
    let h_cat = h.hash("my cat");
    let h_dog2 = h.hash(b"my dog");
    assert_eq!(h_dog, 808761308841733891);
    assert_eq!(h_dog, h_dog2);
    assert_ne!(h_dog, h_cat);
}

#[test]
fn test_clhasher_drop_runs() {
    // Just ensure constructing and dropping a ClHasher does not panic.
    {
        let _h = ClHasher::new(7, 13);
    }
}

#[test]
fn test_avalanche_short_strings() {
    // Mirrors part of C's clhashavalanchetest. For inputs <= sizeof(uint64_t),
    // (orighash ^ newhash) == (orighash1 ^ newhash1) when the same bit is flipped.
    let rs = det_key();
    let mut a = vec![0u8; 8];
    let mut b = vec![0u8; 8];
    for whichcase in 0..256u32 {
        for k in 0..8 {
            a[k] = whichcase as u8;
            b[k] = (whichcase + 35) as u8;
        }
        let orig = clhash(&rs, &a);
        let orig1 = clhash(&rs, &b);
        for z in 0..(8 * 8) {
            let byte = z >> 3;
            let bit = z & 0x7;
            a[byte] ^= 1 << bit;
            let new_h = clhash(&rs, &a);
            a[byte] ^= 1 << bit;
            assert_ne!(orig, new_h);

            b[byte] ^= 1 << bit;
            let new_h1 = clhash(&rs, &b);
            b[byte] ^= 1 << bit;
            assert_ne!(orig1, new_h1);

            // length is 8, which is <= sizeof(uint64_t), so the linear property holds.
            assert_eq!(orig ^ new_h, orig1 ^ new_h1);
        }
    }
}

#[test]
fn test_collision_test_with_eik_list() {
    // Mirrors clhashcollisiontest from unit.c: for messages of length
    // i * 1024 + j (1 <= j <= 8), changing the last byte should change the hash.
    let mut k = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    let key_offset: u8 = 0x63;
    for j2 in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        k[j2] = ((j2 as u32 + key_offset as u32) & 0xFF) as u8;
    }
    for i in 1..10usize {
        for j in 1..=8usize {
            let mlen = i * 1024 + j;
            let mut m = vec![0u8; mlen];
            for j2 in 0..mlen {
                m[j2] = (j2 & 0xFF) as u8;
            }
            let h1 = clhash(&k, &m);
            m[mlen - 1] = m[mlen - 1].wrapping_add(1);
            let h2 = clhash(&k, &m);
            assert_ne!(h1, h2);
        }
    }
}

#[test]
fn test_bit_flip_64_bit_inputs() {
    // Mirrors clhashtest: flipping a bit changes the hash; flipping it back
    // restores the original.
    let rs = det_key2();
    for bit in 0..64usize {
        for length in ((bit + 8) / 8)..=8 {
            let mut x: u64 = 0;
            let bytes = x.to_le_bytes();
            let orig = clhash(&rs, &bytes[..length]);
            x ^= 1u64 << bit;
            let bytes2 = x.to_le_bytes();
            let flip = clhash(&rs, &bytes2[..length]);
            assert_ne!(flip, orig);
            x ^= 1u64 << bit;
            let bytes3 = x.to_le_bytes();
            let back = clhash(&rs, &bytes3[..length]);
            assert_eq!(back, orig);
        }
    }
}

#[test]
fn test_long_input_lengths() {
    // Test long strings (> 128 * 8 = 1024 bytes) which exercise the long-string
    // code path. We just verify that two identical inputs hash the same and
    // different lengths produce different hashes (with high probability).
    let rs = det_key();
    let big = vec![0xABu8; 4096];
    let h1 = clhash(&rs, &big);
    let h2 = clhash(&rs, &big);
    assert_eq!(h1, h2);
    let h3 = clhash(&rs, &big[..4095]);
    assert_ne!(h1, h3);
}

#[test]
fn test_boundary_lengths() {
    // Lengths exactly at boundaries: 1024 (= 128 words), 1024+1, 1023, 8, 7, 9.
    let rs = det_key();
    for len in [0usize, 1, 7, 8, 9, 15, 16, 17, 1023, 1024, 1025, 2048, 2049] {
        let mut data = vec![0u8; len];
        for i in 0..len {
            data[i] = (i & 0xFF) as u8;
        }
        let a = clhash(&rs, &data);
        let b = clhash(&rs, &data);
        assert_eq!(a, b, "deterministic for len {}", len);
    }
}

fn main() {}
