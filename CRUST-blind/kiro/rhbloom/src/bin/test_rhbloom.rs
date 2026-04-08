use rhbloom::rhbloom::RHBloom;

// Murmurhash2 matching the C test harness, used to generate deterministic keys
fn murmurhash2(key: &[u8], seed: u32) -> u32 {
    let m: u32 = 0x5bd1e995;
    let r = 24;
    let mut h: u32 = seed ^ (key.len() as u32);
    let mut data = key;
    while data.len() >= 4 {
        let mut k = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        k = k.wrapping_mul(m);
        k ^= k >> r;
        k = k.wrapping_mul(m);
        h = h.wrapping_mul(m);
        h ^= k;
        data = &data[4..];
    }
    if data.len() >= 3 { h ^= (data[2] as u32) << 16; }
    if data.len() >= 2 { h ^= (data[1] as u32) << 8; }
    if data.len() >= 1 { h ^= data[0] as u32; h = h.wrapping_mul(m); }
    h ^= h >> 13;
    h = h.wrapping_mul(m);
    h ^= h >> 15;
    h
}

fn hash(x: i32) -> u64 {
    murmurhash2(&x.to_le_bytes(), 0) as u64
}

// --- new ---

#[test]
fn test_new_default_state() {
    let b = RHBloom::new(100, 0.01);
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), std::mem::size_of::<RHBloom>());
}

#[test]
fn test_new_small_n_clamped_to_16() {
    let b = RHBloom::new(0, 0.01);
    assert_eq!(b.memsize(), std::mem::size_of::<RHBloom>());
    assert!(!b.upgraded());
}

#[test]
fn test_new_n_equals_1() {
    let b = RHBloom::new(1, 0.01);
    assert!(!b.upgraded());
}

// --- test on empty filter ---

#[test]
fn test_empty_filter_returns_false() {
    let b = RHBloom::new(100, 0.01);
    assert!(!b.test(hash(0)));
    assert!(!b.test(hash(999)));
    assert!(!b.test(0));
    assert!(!b.test(u64::MAX));
}

// --- add / test in robinhood phase ---

#[test]
fn test_add_and_test_single_key() {
    let mut b = RHBloom::new(1000, 0.01);
    assert!(b.add(hash(0)));
    assert!(b.test(hash(0)));
    assert!(!b.test(hash(1)));
    assert!(!b.upgraded());
}

#[test]
fn test_add_multiple_keys_robinhood() {
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..10 {
        b.add(hash(i));
    }
    assert!(!b.upgraded());
    for i in 0..10 {
        assert!(b.test(hash(i)));
    }
    assert!(!b.test(hash(999)));
}

#[test]
fn test_add_returns_true() {
    let mut b = RHBloom::new(100, 0.01);
    assert!(b.add(hash(42)));
}

#[test]
fn test_duplicate_add() {
    let mut b = RHBloom::new(100, 0.01);
    assert!(b.add(hash(42)));
    assert!(b.add(hash(42)));
    assert!(b.test(hash(42)));
}

// --- memsize in robinhood phase ---

#[test]
fn test_memsize_initial() {
    let b = RHBloom::new(16, 0.01);
    // No buckets allocated yet, so memsize = base only
    assert_eq!(b.memsize(), 80);
}

#[test]
fn test_memsize_after_adds_robinhood() {
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..10 {
        b.add(hash(i));
    }
    // First grow creates 16 buckets, second grow doubles to 32
    // 32 * 8 = 256, plus base 80 = 336
    assert_eq!(b.memsize(), 336);
}

// --- upgrade to bloom ---

#[test]
fn test_upgrade_to_bloom() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    // After upgrade: memsize = base + m>>3 = 80 + 128 = 208
    assert_eq!(b.memsize(), 208);
}

#[test]
fn test_all_keys_found_after_upgrade() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    for i in 0..=100 {
        assert!(b.test(hash(i)), "key {} not found after upgrade", i);
    }
}

#[test]
fn test_upgrade_small_p() {
    let mut b = RHBloom::new(16, 0.5);
    for i in 0..=16 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    // m>>3 = 4, base 80 + 4 = 84
    assert_eq!(b.memsize(), 84);
}

// --- upgraded ---

#[test]
fn test_upgraded_false_initially() {
    let b = RHBloom::new(100, 0.01);
    assert!(!b.upgraded());
}

#[test]
fn test_upgraded_true_after_bloom() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
}

// --- clear ---

#[test]
fn test_clear_on_empty_filter() {
    let mut b = RHBloom::new(100, 0.01);
    b.clear(); // should not panic
    assert!(!b.upgraded());
}

#[test]
fn test_clear_robinhood_phase() {
    let mut b = RHBloom::new(1000, 0.01);
    for i in 0..10 {
        b.add(hash(i));
    }
    assert!(!b.upgraded());
    b.clear();
    // After clear in robinhood phase, keys should not be found
    for i in 0..10 {
        assert!(!b.test(hash(i)));
    }
}

#[test]
fn test_clear_preserves_upgraded_state() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    assert!(b.upgraded());
    let ms = b.memsize();
    b.clear();
    assert!(b.upgraded());
    assert_eq!(b.memsize(), ms);
}

#[test]
fn test_clear_zeros_bloom_bits() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    b.clear();
    for i in 0..=100 {
        assert!(!b.test(hash(i)));
    }
}

#[test]
fn test_readd_after_clear() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    b.clear();
    for i in 0..=100 {
        b.add(hash(i));
    }
    for i in 0..=100 {
        assert!(b.test(hash(i)), "key {} not found after re-add", i);
    }
}

// --- test_step equivalent (matches C test logic) ---

#[test]
fn test_step_small() {
    // Mirrors C test_step(rhbloom, 0, 0.01)
    let mut b = RHBloom::new(0, 0.01);
    let n = 0;
    let nn = n + 1;
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
    let hits: i32 = (0..nn).filter(|&i| b.test(hash(i))).count() as i32;
    assert_eq!(hits, nn);
}

#[test]
fn test_step_medium() {
    // Mirrors C test_step(rhbloom, 1000, 0.01)
    let mut b = RHBloom::new(1000, 0.01);
    let n = 1000;
    let p = 0.01;
    let nn = n + 1;
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
    let hits: i32 = (0..nn).filter(|&i| b.test(hash(i))).count() as i32;
    assert_eq!(hits, nn);

    // Check false positive rate
    let fp: i32 = (nn..nn * 2).filter(|&i| b.test(hash(i))).count() as i32;
    let fp_rate = fp as f64 / n as f64;
    assert!(fp_rate - p < 0.1, "false positive rate too high: {}", fp_rate);
}

#[test]
fn test_step_then_clear_then_redo() {
    // Mirrors the C test pattern: test_step, clear, test_step again
    for &(n, p) in &[(0, 0.01), (1000, 0.01), (1000, 0.5)] {
        let mut b = RHBloom::new(n, p);
        let nn = n as i32 + 1;
        // First pass
        for i in 0..nn {
            b.add(hash(i));
        }
        assert!(b.upgraded());
        let hits: i32 = (0..nn).filter(|&i| b.test(hash(i))).count() as i32;
        assert_eq!(hits, nn);

        // Clear and redo
        b.clear();
        for i in 0..nn {
            b.add(hash(i));
        }
        assert!(b.upgraded());
        let hits: i32 = (0..nn).filter(|&i| b.test(hash(i))).count() as i32;
        assert_eq!(hits, nn);
    }
}

// --- boundary keys ---

#[test]
fn test_key_zero() {
    let mut b = RHBloom::new(100, 0.01);
    b.add(0);
    assert!(b.test(0));
}

#[test]
fn test_key_max() {
    let mut b = RHBloom::new(100, 0.01);
    b.add(u64::MAX);
    assert!(b.test(u64::MAX));
}

#[test]
fn test_key_one() {
    let mut b = RHBloom::new(100, 0.01);
    b.add(1);
    assert!(b.test(1));
    assert!(!b.test(0));
}

// --- various p values ---

#[test]
fn test_various_probabilities() {
    let mut p = 0.01;
    while p < 0.70 {
        let mut b = RHBloom::new(1000, p);
        let nn = 1001;
        for i in 0..nn {
            b.add(hash(i));
        }
        assert!(b.upgraded());
        let hits: i32 = (0..nn).filter(|&i| b.test(hash(i))).count() as i32;
        assert_eq!(hits, nn, "failed for p={}", p);
        p += 0.05;
    }
}

// --- free ---

#[test]
fn test_free() {
    let mut b = RHBloom::new(100, 0.01);
    for i in 0..=100 {
        b.add(hash(i));
    }
    b.free();
    assert!(!b.upgraded());
    assert_eq!(b.memsize(), std::mem::size_of::<RHBloom>());
}

fn main() {}
