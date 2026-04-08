use clhash::clhash::{
    clhash, get_random_key_for_clhash, ClHasher, RANDOM_64BITWORDS_NEEDED_FOR_CLHASH,
    RANDOM_BYTES_NEEDED_FOR_CLHASH,
};

// ---- Constants ----

#[test]
fn test_constants() {
    assert_eq!(RANDOM_64BITWORDS_NEEDED_FOR_CLHASH, 133);
    assert_eq!(RANDOM_BYTES_NEEDED_FOR_CLHASH, 133 * 8);
}

// ---- get_random_key_for_clhash ----

#[test]
fn test_get_random_key_length() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(key.len(), RANDOM_BYTES_NEEDED_FOR_CLHASH);
}

#[test]
fn test_get_random_key_values() {
    let key = get_random_key_for_clhash(137, 777);
    let k64: Vec<u64> = key
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();

    // Verify first few values match C xorshift128plus output
    assert_eq!(k64[0], 1149244865);
    assert_eq!(k64[1], 8701379260);
    assert_eq!(k64[2], 9640526657320736);
    assert_eq!(k64[3], 64317040884696645);
    assert_eq!(k64[4], 109926439917670026);
    assert_eq!(k64[5], 118642916319092990);
    assert_eq!(k64[6], 13980211360741814764);
    assert_eq!(k64[7], 14746191864734490451);
    assert_eq!(k64[8], 10233102483202606683);
    assert_eq!(k64[9], 7835856983069276281);

    // Verify tail values
    assert_eq!(k64[128], 12491235888376946051);
    assert_eq!(k64[129], 17217646778639590103);
    assert_eq!(k64[130], 10933674056228117724);
    assert_eq!(k64[131], 7081780302273033876);
    assert_eq!(k64[132], 11205380670366400436);
}

#[test]
fn test_get_random_key_deterministic() {
    let k1 = get_random_key_for_clhash(137, 777);
    let k2 = get_random_key_for_clhash(137, 777);
    assert_eq!(k1, k2);
}

#[test]
fn test_get_random_key_different_seeds() {
    let k1 = get_random_key_for_clhash(137, 777);
    let k2 = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_ne!(k1, k2);
}

// ---- clhash: empty and boundary inputs ----

#[test]
fn test_clhash_empty_string() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b""), 0);
}

#[test]
fn test_clhash_single_byte() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b"a"), 1382967411330071092);
    assert_eq!(clhash(&key, b"b"), 1382967437022510576);
    assert_eq!(clhash(&key, b"\0"), 1382967151133016072);
}

// ---- clhash: short strings ----

#[test]
fn test_clhash_short_strings() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b"my dog"), 10235581410102137208);
    assert_eq!(clhash(&key, b"my cat"), 526488957445861319);
    assert_eq!(clhash(&key, b"hello"), 18255269798239507943);
    assert_eq!(clhash(&key, b"test"), 1106899457831998698);
}

// ---- clhash: aligned lengths (multiples of 8) ----

#[test]
fn test_clhash_aligned_lengths() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b"12345678"), 14742390747119455523);
    assert_eq!(clhash(&key, b"1234567890123456"), 2526366401639115986);
    assert_eq!(clhash(&key, b"123456789012345678901234"), 17305781157606364117);
}

// ---- clhash: unaligned length (15 bytes) ----

#[test]
fn test_clhash_unaligned_length() {
    let key = get_random_key_for_clhash(137, 777);
    assert_eq!(clhash(&key, b"123456789012345"), 14437393240895616983);
}

// ---- clhash: different seeds ----

#[test]
fn test_clhash_different_seeds() {
    let key = get_random_key_for_clhash(0x23a23cf5033c3c81, 0xb3816f6a2c68e530);
    assert_eq!(clhash(&key, b"my dog"), 808761308841733891);
    assert_eq!(clhash(&key, b"my cat"), 2088517542587126895);
}

// ---- clhash: determinism ----

#[test]
fn test_clhash_deterministic() {
    let key = get_random_key_for_clhash(137, 777);
    let h1 = clhash(&key, b"test");
    let h2 = clhash(&key, b"test");
    assert_eq!(h1, h2);
}

// ---- clhash: manual key (like C unit test) ----

fn make_manual_key() -> Vec<u8> {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    for k in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        rs[k] = (1i32 - k as i32) as u8;
    }
    rs
}

#[test]
fn test_clhash_manual_key_zeros() {
    let rs = make_manual_key();
    let zero = [0u8; 8];
    assert_eq!(clhash(&rs, &zero[..1]), 3518691437419927626);
    assert_eq!(clhash(&rs, &zero[..2]), 6897406170948942194);
    assert_eq!(clhash(&rs, &zero[..3]), 9611629948823427475);
    assert_eq!(clhash(&rs, &zero[..4]), 9322531023046572802);
    assert_eq!(clhash(&rs, &zero[..5]), 6610039946375863267);
    assert_eq!(clhash(&rs, &zero[..6]), 3807777169057266395);
    assert_eq!(clhash(&rs, &zero[..7]), 17153135610892900922);
    assert_eq!(clhash(&rs, &zero[..8]), 4382526952154562553);
}

#[test]
fn test_clhash_manual_key_strings() {
    let rs = make_manual_key();
    assert_eq!(clhash(&rs, b"hello"), 16856428472158674338);
    assert_eq!(clhash(&rs, b"world"), 10065635763051777486);
}

// ---- clhash: long strings (> 1024 bytes, triggers multi-block path) ----

fn make_long_string(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i & 0xFF) as u8).collect()
}

#[test]
fn test_clhash_long_string_generated_key() {
    let key = get_random_key_for_clhash(137, 777);
    let longstr = make_long_string(2048);
    assert_eq!(clhash(&key, &longstr[..1024]), 13878933686121200245);
    assert_eq!(clhash(&key, &longstr[..1025]), 14167079484582886430);
    assert_eq!(clhash(&key, &longstr), 16890874039237711337);
}

#[test]
fn test_clhash_long_string_manual_key() {
    let rs = make_manual_key();
    let longstr = make_long_string(2048);
    assert_eq!(clhash(&rs, &longstr[..1024]), 8030893932835744853);
    assert_eq!(clhash(&rs, &longstr[..1025]), 127030318649434636);
    assert_eq!(clhash(&rs, &longstr), 13927702028376612513);
}

// ---- clhash: bit-flip test (from C unit test) ----

#[test]
fn test_clhash_bitflip() {
    let rs = make_manual_key();
    for bit in 0..64u32 {
        let min_len = ((bit + 8) / 8) as usize;
        for length in min_len..=8 {
            let x: u64 = 0;
            let orig = clhash(&rs, &x.to_le_bytes()[..length]);
            let flipped_x = x ^ (1u64 << bit);
            let flip = clhash(&rs, &flipped_x.to_le_bytes()[..length]);
            assert_ne!(flip, orig, "bit={bit} length={length}");
            let back = clhash(&rs, &x.to_le_bytes()[..length]);
            assert_eq!(back, orig, "bit={bit} length={length}");
        }
    }
}

// ---- ClHasher struct ----

#[test]
fn test_clhasher_new_and_hash() {
    let hasher = ClHasher::new(137, 777);
    assert_eq!(hasher.hash(b"my dog"), 10235581410102137208);
    assert_eq!(hasher.hash(b"my cat"), 526488957445861319);
    assert_eq!(hasher.hash("hello"), 18255269798239507943);
}

#[test]
fn test_clhasher_empty() {
    let hasher = ClHasher::new(137, 777);
    assert_eq!(hasher.hash(b""), 0);
}

#[test]
fn test_clhasher_deterministic() {
    let hasher = ClHasher::new(137, 777);
    let h1 = hasher.hash(b"test");
    let h2 = hasher.hash(b"test");
    assert_eq!(h1, h2);
}

// ---- Collision resistance (from C unit test) ----

#[test]
fn test_clhash_collision_resistance() {
    let mut rs = vec![0u8; RANDOM_BYTES_NEEDED_FOR_CLHASH];
    let key_offset: u8 = 0x63;
    for j in 0..RANDOM_BYTES_NEEDED_FOR_CLHASH {
        rs[j] = ((j as u64 + key_offset as u64) & 0xFF) as u8;
    }
    let block_size = 1024usize;
    for i in 1..10usize {
        for j in 1..=8usize {
            let mlen = i * block_size + j;
            let mut m: Vec<u8> = (0..mlen).map(|k| (k & 0xFF) as u8).collect();
            let h1 = clhash(&rs, &m);
            m[mlen - 1] = m[mlen - 1].wrapping_add(1);
            let h2 = clhash(&rs, &m);
            assert_ne!(h1, h2, "collision at mlen={mlen}");
        }
    }
}

fn main() {}
