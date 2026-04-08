use clhash::clhash::{
    clhash, get_random_key_for_clhash, ClHasher,
    RANDOM_64BITWORDS_NEEDED_FOR_CLHASH, RANDOM_BYTES_NEEDED_FOR_CLHASH,
};

fn main() {}

// --- Constants ---

#[test]
fn test_constants() {
    assert_eq!(RANDOM_64BITWORDS_NEEDED_FOR_CLHASH, 133);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 133 * 8);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 1064);
}

// --- get_random_key_for_clhash ---

#[test]
fn test_get_random_key_length() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(key.len(), RANDOM_BYTES_NEEDED_FOR_CLHASH);
}

#[test]
fn test_get_random_key_deterministic() {
    let key1 = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let key2 = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(key1, key2);
}

#[test]
fn test_get_random_key_values_seed1() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let r = |i: usize| u64::from_le_bytes(key[i * 8..i * 8 + 8].try_into().unwrap());
    assert_eq!(r(0), 11727443923012301735);
    assert_eq!(r(1), 16001092925326965413);
    assert_eq!(r(2), 5675522312328131209);
    assert_eq!(r(132), 1066976533721771965);
}

#[test]
fn test_get_random_key_values_seed2() {
    let key = get_random_key_for_clhash(137, 777);
    let r = |i: usize| u64::from_le_bytes(key[i * 8..i * 8 + 8].try_into().unwrap());
    assert_eq!(r(0), 1149244865);
    assert_eq!(r(1), 8701379260);
    assert_eq!(r(132), 11205380670366400436);
}

#[test]
fn test_get_random_key_different_seeds() {
    let key1 = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let key2 = get_random_key_for_clhash(137, 777);
    assert_ne!(key1, key2);
}

// --- clhash: demo values (seeds 0x23a23cf5033c3c81, 0xb3816f6a2c68e530) ---

#[test]
fn test_clhash_demo_my_dog() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"my dog"), 808761308841733891);
}

#[test]
fn test_clhash_demo_my_cat() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"my cat"), 2088517542587126895);
}

#[test]
fn test_clhash_deterministic() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h1 = clhash(&key, b"my dog");
    let h3 = clhash(&key, b"my dog");
    assert_eq!(h1, h3);
    assert_eq!(h1, 808761308841733891);
}

#[test]
fn test_clhash_empty_string() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b""), 0);
}

#[test]
fn test_clhash_single_byte() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"a"), 5667746712765706676);
}

#[test]
fn test_clhash_5_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"hello"), 8531588392195409363);
}

#[test]
fn test_clhash_8_bytes_aligned() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"12345678"), 210716313166875572);
}

#[test]
fn test_clhash_16_bytes_aligned() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"1234567890123456"), 3390917496109661694);
}

#[test]
fn test_clhash_7_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"abcdefg"), 13954811839410827229);
}

#[test]
fn test_clhash_9_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"abcdefghi"), 13450074313225880919);
}

#[test]
fn test_clhash_15_bytes() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"abcdefghijklmno"), 3059589021337807290);
}

// --- clhash: different seeds (137, 777) ---

#[test]
fn test_clhash_seed2_test() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b"test"), 1106899457831998698);
}

#[test]
fn test_clhash_seed2_empty() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b""), 0);
}

#[test]
fn test_clhash_seed2_long_string() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(
        clhash(&key, b"the quick brown fox jumps over the lazy dog"),
        4509208149723572213
    );
}

// --- clhash: manual random source (clhashtest-style) ---

#[test]
fn test_clhash_manual_rs() {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        rs[k] = (1i32 - k as i32) as u8;
    }
    let x: u64 = 0;
    assert_eq!(clhash(&rs, &x.to_le_bytes()), 4382526952154562553);
    let x: u64 = 1;
    assert_eq!(clhash(&rs, &x.to_le_bytes()), 14853480695934256896);
}

// --- clhash: long strings (> 1024 bytes, exercises multi-block path) ---

#[test]
fn test_clhash_long_1024() {
    let key = get_random_key_for_clhash(42, 99);
    let data: Vec<u8> = (0..1024).map(|i| (i & 0xFF) as u8).collect();
    assert_eq!(clhash(&key, &data), 10565863334500808456);
}

#[test]
fn test_clhash_long_1025() {
    let key = get_random_key_for_clhash(42, 99);
    let data: Vec<u8> = (0..1025).map(|i| (i & 0xFF) as u8).collect();
    assert_eq!(clhash(&key, &data), 16873154198523271533);
}

#[test]
fn test_clhash_long_2048() {
    let key = get_random_key_for_clhash(42, 99);
    let data: Vec<u8> = (0..2048).map(|i| (i & 0xFF) as u8).collect();
    assert_eq!(clhash(&key, &data), 4221986466180303932);
}

// --- clhash: bit-flip test (from C clhashtest) ---

#[test]
fn test_clhash_bitflip() {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        rs[k] = (1i32 - k as i32) as u8;
    }
    for bit in 0..64u32 {
        let min_len = ((bit + 8) / 8) as usize;
        for length in min_len..=8 {
            let x: u64 = 0;
            let orig = clhash(&rs, &x.to_le_bytes()[..length]);
            let x_flipped: u64 = 1u64 << bit;
            let flip = clhash(&rs, &x_flipped.to_le_bytes()[..length]);
            assert_ne!(flip, orig, "bit={} length={}", bit, length);
            let back = clhash(&rs, &x.to_le_bytes()[..length]);
            assert_eq!(back, orig, "bit={} length={}", bit, length);
        }
    }
}

// --- clhash: collision test (Eik List) ---

#[test]
fn test_clhash_collision_eik_list() {
    let key_offset: u8 = 0x63;
    let mut k = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for j in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        k[j] = ((j as u64 + key_offset as u64) & 0xFF) as u8;
    }
    for i in 1..10usize {
        for j in 1..=8usize {
            let mlen = i * 1024 + j;
            let mut m: Vec<u8> = (0..mlen).map(|x| (x & 0xFF) as u8).collect();
            let h1 = clhash(&k, &m);
            m[mlen - 1] = (m[mlen - 1].wrapping_add(1)) & 0xFF;
            let h2 = clhash(&k, &m);
            assert_ne!(h1, h2, "collision at i={} j={} mlen={}", i, j, mlen);
        }
    }
}

// --- clhash: specific collision test value ---

#[test]
fn test_clhash_collision_specific_values() {
    let key_offset: u8 = 0x63;
    let mut k = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for j in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        k[j] = ((j as u64 + key_offset as u64) & 0xFF) as u8;
    }
    let m: Vec<u8> = (0..1025usize).map(|x| (x & 0xFF) as u8).collect();
    assert_eq!(clhash(&k, &m), 4383937999666532308);
    let mut m2 = m.clone();
    m2[1024] = (m2[1024].wrapping_add(1)) & 0xFF;
    assert_eq!(clhash(&k, &m2), 17183773019360414639);
}

// --- clhash: avalanche test (from C clhashavalanchetest) ---

#[test]
fn test_clhash_avalanche() {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        rs[k] = ((k as i32 + 1 - (k as i32) * (k as i32)) & 0xFF) as u8;
    }
    for bytelength in 1..16usize {
        for whichcase in 0..256u16 {
            let val = whichcase as u8;
            let array: Vec<u8> = vec![val; bytelength];
            let array1: Vec<u8> = vec![val.wrapping_add(35); bytelength];
            let orighash = clhash(&rs, &array);
            let orighash1 = clhash(&rs, &array1);
            for z in 0..8 * bytelength {
                let byte_idx = z / 8;
                let bit_idx = z % 8;
                let mut flipped = array.clone();
                flipped[byte_idx] ^= 1 << bit_idx;
                let newhash = clhash(&rs, &flipped);
                assert_ne!(orighash, newhash);

                let mut flipped1 = array1.clone();
                flipped1[byte_idx] ^= 1 << bit_idx;
                let newhash1 = clhash(&rs, &flipped1);
                assert_ne!(orighash1, newhash1);

                if bytelength <= 8 {
                    assert_eq!(
                        orighash ^ newhash,
                        orighash1 ^ newhash1,
                        "avalanche failed at bytelength={} whichcase={} z={}",
                        bytelength, whichcase, z
                    );
                }
            }
        }
    }
}

// --- ClHasher struct ---

#[test]
fn test_clhasher_new_and_hash() {
    let hasher = ClHasher::new(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(hasher.hash(b"my dog"), 808761308841733891);
    assert_eq!(hasher.hash(b"my cat"), 2088517542587126895);
    assert_eq!(hasher.hash(b""), 0);
}

#[test]
fn test_clhasher_deterministic() {
    let hasher = ClHasher::new(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    let h1 = hasher.hash(b"hello");
    let h2 = hasher.hash(b"hello");
    assert_eq!(h1, h2);
    assert_eq!(h1, 8531588392195409363);
}

#[test]
fn test_clhasher_different_seeds() {
    let hasher = ClHasher::new(137, 777);
    assert_eq!(hasher.hash(b"test"), 1106899457831998698);
}
