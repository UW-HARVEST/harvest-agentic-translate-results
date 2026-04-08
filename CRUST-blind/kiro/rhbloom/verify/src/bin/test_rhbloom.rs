use rhbloom::rhbloom::RHBloom;

// Murmurhash2 matching the C test's hash function
fn murmurhash2(key: &[u8], seed: u32) -> u32 {
    let m: u32 = 0x5bd1e995;
    let r = 24;
    let len = key.len();
    let mut h: u32 = seed ^ (len as u32);
    let mut i = 0;
    while i + 4 <= len {
        let mut k = u32::from_le_bytes([key[i], key[i+1], key[i+2], key[i+3]]);
        k = k.wrapping_mul(m);
        k ^= k >> r;
        k = k.wrapping_mul(m);
        h = h.wrapping_mul(m);
        h ^= k;
        i += 4;
    }
    let remaining = len - i;
    if remaining >= 3 { h ^= (key[i+2] as u32) << 16; }
    if remaining >= 2 { h ^= (key[i+1] as u32) << 8; }
    if remaining >= 1 { h ^= key[i] as u32; h = h.wrapping_mul(m); }
    h ^= h >> 13;
    h = h.wrapping_mul(m);
    h ^= h >> 15;
    h
}

fn hash(x: i32) -> u64 {
    murmurhash2(&x.to_le_bytes(), 0) as u64
}

// Rust struct base size (80) vs C struct base size (64) = +16 offset
const MEMSIZE_OFFSET: usize = std::mem::size_of::<RHBloom>() - 64;

#[test]
fn test_mix() {
    assert_eq!(RHBloom::mix(0), 0);
    assert_eq!(RHBloom::mix(1), 6238072747940578789);
    assert_eq!(RHBloom::mix(2), 15839785061582574730);
    assert_eq!(RHBloom::mix(42), 12058926934050108962);
    assert_eq!(RHBloom::mix(100), 2824278126137619252);
    assert_eq!(RHBloom::mix(u64::MAX), 13029008266876403067);
    assert_eq!(RHBloom::mix(0xDEADBEEF), 5622224078331092714);
}

#[test]
fn test_empty_filter() {
    let b = RHBloom::new(100, 0.01);
    assert!(!b.test(hash(0)));
    assert!(!b.test(12345));
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), 64 + MEMSIZE_OFFSET); // C: 64
}

#[test]
fn test_new_clamps_n_to_16() {
    let b = RHBloom::new(0, 0.01);
    assert_eq!(b.memsize(), 64 + MEMSIZE_OFFSET);
    assert!(!b.upgraded());

    let b2 = RHBloom::new(5, 0.01);
    assert_eq!(b2.memsize(), 64 + MEMSIZE_OFFSET);
}

#[test]
fn test_add_and_test_robinhood_phase() {
    let mut b = RHBloom::new(1000, 0.01);
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), 64 + MEMSIZE_OFFSET);

    for i in 0..5 {
        b.add(hash(i));
    }
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), 192 + MEMSIZE_OFFSET); // C: 192

    // All added keys should be found
    for i in 0..5 {
        assert!(b.test(hash(i)), "hash({}) should be found", i);
    }
    // Non-existent keys should not be found
    for i in 100..105 {
        assert!(!b.test(hash(i)), "hash({}) should not be found", i);
    }
}

#[test]
fn test_robinhood_memsize_growth() {
    let mut b = RHBloom::new(1000, 0.01);
    assert_eq!(b.memsize(), 64 + MEMSIZE_OFFSET);

    b.add(hash(0));
    assert_eq!(b.memsize(), 192 + MEMSIZE_OFFSET); // 16 buckets * 8 = 128 + 64

    for i in 1..8 { b.add(hash(i)); }
    assert_eq!(b.memsize(), 192 + MEMSIZE_OFFSET);

    for i in 8..16 { b.add(hash(i)); }
    assert_eq!(b.memsize(), 320 + MEMSIZE_OFFSET); // 32 buckets * 8 = 256 + 64

    for i in 16..32 { b.add(hash(i)); }
    assert_eq!(b.memsize(), 576 + MEMSIZE_OFFSET);

    for i in 32..64 { b.add(hash(i)); }
    assert_eq!(b.memsize(), 1088 + MEMSIZE_OFFSET);
    assert!(!b.upgraded());
}

#[test]
fn test_upgrade_n16() {
    // n=16, p=0.01: upgrades on first add
    let mut b = RHBloom::new(16, 0.01);
    assert!(!b.upgraded());
    b.add(hash(0));
    assert!(b.upgraded());
    assert_eq!(b.memsize(), 96 + MEMSIZE_OFFSET);
}

#[test]
fn test_upgrade_n1000() {
    // n=1000, p=0.01: upgrades at add index 64
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..64 {
        b.add(hash(i));
        assert!(!b.upgraded(), "should not upgrade at add {}", i);
    }
    b.add(hash(64));
    assert!(b.upgraded());
    assert_eq!(b.memsize(), 2112 + MEMSIZE_OFFSET);
}

#[test]
fn test_small_filter_full() {
    // n=16, p=0.01: add 17 keys, all should be found
    let mut b = RHBloom::new(16, 0.01);
    for i in 0..=16 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    assert_eq!(b.memsize(), 96 + MEMSIZE_OFFSET);

    let hits: i32 = (0..=16).filter(|&i| b.test(hash(i))).count() as i32;
    assert_eq!(hits, 17);

    let fp: i32 = (17..=33).filter(|&i| b.test(hash(i))).count() as i32;
    assert_eq!(fp, 0);
}

#[test]
fn test_full_n1000() {
    let mut b = RHBloom::new(1000, 0.01);
    let nn = 1001;
    for i in 0..nn {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    assert_eq!(b.memsize(), 2112 + MEMSIZE_OFFSET);

    let hits = (0..nn).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, 1001);

    let fp = (nn..nn*2).filter(|&i| b.test(hash(i))).count();
    assert_eq!(fp, 0);
}

#[test]
fn test_false_positives_n100_p01() {
    let mut b = RHBloom::new(100, 0.1);
    let nn = 101;
    for i in 0..nn { b.add(hash(i)); }
    assert!(b.upgraded());
    assert_eq!(b.memsize(), 128 + MEMSIZE_OFFSET);

    let hits = (0..nn).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, 101);

    let fp = (nn..nn*2).filter(|&i| b.test(hash(i))).count();
    assert_eq!(fp, 9);
}

#[test]
fn test_clear_robinhood() {
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..5 { b.add(hash(i)); }
    assert!(b.test(hash(0)));
    b.clear();
    assert!(!b.test(hash(0)));
    // Re-add works
    b.add(hash(0));
    assert!(b.test(hash(0)));
}

#[test]
fn test_clear_bloom() {
    let mut b = RHBloom::new(16, 0.01);
    for i in 0..=20 { b.add(hash(i)); }
    assert!(b.upgraded());
    assert!(b.test(hash(0)));

    b.clear();
    assert!(b.upgraded()); // stays upgraded
    assert!(!b.test(hash(0))); // but data cleared

    // Re-add after clear
    for i in 0..=20 { b.add(hash(i)); }
    let hits = (0..=20).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, 21);
}

#[test]
fn test_raw_keys() {
    let mut b = RHBloom::new(100, 0.01);
    b.add(12345);
    b.add(67890);
    assert!(b.test(12345));
    assert!(b.test(67890));
    assert!(!b.test(99999));
    assert!(b.upgraded()); // n=100 upgrades on first add (like n=16)
}

#[test]
fn test_n0_edge_case() {
    let mut b = RHBloom::new(0, 0.01);
    assert_eq!(b.memsize(), 64 + MEMSIZE_OFFSET);
    assert!(!b.upgraded());
    b.add(hash(0));
    assert!(b.test(hash(0)));
}

#[test]
fn test_add_returns_true() {
    let mut b = RHBloom::new(1000, 0.01);
    assert!(b.add(hash(0)));
    // Adding same key again should also return true
    assert!(b.add(hash(0)));
}

#[test]
fn test_free() {
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..10 { b.add(hash(i)); }
    b.free();
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), std::mem::size_of::<RHBloom>());
}

#[test]
fn test_step_like_c() {
    // Replicate the C test_step for n=0, p=0.01
    let mut b = RHBloom::new(0, 0.01);
    let nn = 1;
    for i in 0..nn {
        if !b.upgraded() {
            assert!(!b.test(hash(i)));
        }
        b.add(hash(i));
        if !b.upgraded() {
            assert!(b.test(hash(i)));
        }
    }
    assert!(b.upgraded());
    let hits = (0..nn).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, nn as usize);
}

#[test]
fn test_step_n1000() {
    // Replicate C test_step for n=1000, p=0.01
    let mut b = RHBloom::new(1000, 0.01);
    let nn = 1001i32;
    for i in 0..nn {
        if !b.upgraded() {
            assert!(!b.test(hash(i)), "pre-test failed at {}", i);
        }
        b.add(hash(i));
        if !b.upgraded() {
            assert!(b.test(hash(i)), "post-test failed at {}", i);
        }
    }
    assert!(b.upgraded());
    let hits = (0..nn).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, nn as usize);
}

#[test]
fn test_step_then_clear_n1000() {
    let mut b = RHBloom::new(1000, 0.01);
    let nn = 1001i32;
    // First pass
    for i in 0..nn { b.add(hash(i)); }
    assert!(b.upgraded());
    // Clear and redo
    b.clear();
    for i in 0..nn {
        if !b.upgraded() {
            assert!(!b.test(hash(i)));
        }
        b.add(hash(i));
        if !b.upgraded() {
            assert!(b.test(hash(i)));
        }
    }
    let hits = (0..nn).filter(|&i| b.test(hash(i))).count();
    assert_eq!(hits, nn as usize);
}

fn main() {}
