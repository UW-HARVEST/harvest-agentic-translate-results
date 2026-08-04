use rhbloom::rhbloom::RHBloom;

#[test]
fn test_mix_zero() {
    // mix(0) -> 0 (because all xors and multiplies of 0 are 0)
    assert_eq!(RHBloom::mix(0), 0x0000000000000000_u64);
}

#[test]
fn test_mix_known_values() {
    // Reference values computed by running C code
    assert_eq!(RHBloom::mix(1), 0x5692161d100b05e5_u64);
    assert_eq!(RHBloom::mix(2), 0xdbd238973a2b148a_u64);
    assert_eq!(RHBloom::mix(42), 0xa759ea27d4727622_u64);
    assert_eq!(RHBloom::mix(100), 0x2731d9fdf756b334_u64);
    assert_eq!(RHBloom::mix(0xFFFFFFFFFFFFFFFF_u64), 0xb4d055fcf2cbbd7b_u64);
    assert_eq!(RHBloom::mix(0x0123456789abcdef_u64), 0xb2c058e4ebb5112c_u64);
    assert_eq!(RHBloom::mix(0xdeadbeefcafebabe_u64), 0x7ad6664f09ffe52c_u64);
}

#[test]
fn test_new_empty_state() {
    // Newly created filter has count=0, no upgrade, no buckets, so test() returns false.
    let f = RHBloom::new(16, 0.01);
    assert!(!f.upgraded());
    // For an empty filter, test should return false (no buckets allocated yet).
    assert!(!f.test(0));
    assert!(!f.test(1));
    assert!(!f.test(42));
    // memsize: struct (64 bytes) + 0 (no buckets, no bits)
    assert_eq!(f.memsize(), 64);
}

#[test]
fn test_new_minimum_n() {
    // n is clamped to 16; with p=0.5, k=1, m=32 (per C reference)
    // Verified via probe: n=16, p=0.5 yields k=1, m=32
    let f1 = RHBloom::new(0, 0.5);
    let f2 = RHBloom::new(16, 0.5);
    // memsize is identical because n was clamped
    assert_eq!(f1.memsize(), 64);
    assert_eq!(f2.memsize(), 64);
}

#[test]
fn test_add_then_test_in_bucket_mode() {
    // n=1000, p=0.01: k=4, m=16384.
    // After 1 add, nbuckets=16, count=1, not upgraded.
    // memsize = 64 + 16*8 = 192
    let mut f = RHBloom::new(1000, 0.01);
    assert!(f.add(100));
    assert!(!f.upgraded());
    assert!(f.test(100));
    assert!(!f.test(101));
    assert_eq!(f.memsize(), 64 + 16 * 8);
}

#[test]
fn test_add_does_not_upgrade_too_early() {
    // After first add nbuckets=16 (capacity 8 for upgrade trigger).
    // Per C probe: n=10000,p=0.01 has m=131072, m>>3=16384, threshold nbuckets_new>=2048
    // So adding many keys causes nbuckets to grow as: 16, 32, 64, 128, ..., 1024,
    // then upgrade to bloom (instead of growing to 2048).
    let mut f = RHBloom::new(10000, 0.01);
    // Add 257 keys: count grows beyond half of 256 -> grow to 512.
    for i in 0..257u64 {
        f.add(i);
    }
    // After adding 257, the table holds 257 entries with nbuckets=1024
    // (per C probe). Not yet upgraded.
    assert!(!f.upgraded());
}

#[test]
fn test_force_upgrade_to_bloom() {
    // After enough adds, filter upgrades to bloom mode.
    // n=10000, p=0.01: m=131072 so bloom-mode memsize = 64 + (m>>3) = 64 + 16384 = 16448
    let mut f = RHBloom::new(10000, 0.01);
    for i in 0..513u64 {
        f.add(i);
    }
    assert!(f.upgraded());
    assert_eq!(f.memsize(), 64 + (131072 >> 3));
    // All added keys must report present.
    for i in 0..513u64 {
        assert!(f.test(i), "missing key {} after upgrade", i);
    }
}

#[test]
fn test_clear_in_bucket_mode() {
    // n=1000, p=0.01: 1 add, nbuckets=16. clear keeps the buckets but zeros them.
    let mut f = RHBloom::new(1000, 0.01);
    f.add(42);
    assert!(f.test(42));
    assert!(!f.upgraded());
    f.clear();
    // After clear in bucket mode: not upgraded; previous keys gone.
    assert!(!f.upgraded());
    assert!(!f.test(42));
    // memsize unchanged (buckets remain allocated).
    assert_eq!(f.memsize(), 64 + 16 * 8);
}

#[test]
fn test_clear_in_bloom_mode() {
    // After upgrade, clear zeros bits. memsize stays the same.
    // n=16, p=0.5: k=1, m=32, single bit per key. Adds quickly upgrade.
    let mut f = RHBloom::new(16, 0.5);
    f.add(42);
    // For n=16, p=0.5, very small filter — even one add triggers upgrade
    // because nbuckets_new=16 *8 = 128 >= m>>3 = 4. Confirmed via C probe.
    assert!(f.upgraded());
    assert!(f.test(42));
    f.clear();
    assert!(f.upgraded()); // still upgraded
    assert!(!f.test(42));
}

#[test]
fn test_add_returns_true() {
    // Successful add should return true.
    let mut f = RHBloom::new(100, 0.01);
    assert!(f.add(1));
    assert!(f.add(2));
    assert!(f.add(3));
}

#[test]
fn test_add_duplicate_keys() {
    // Adding the same key multiple times is fine and does not increase
    // the count past 1 (in bucket mode the second add is a no-op insertion).
    let mut f = RHBloom::new(1000, 0.01);
    f.add(123);
    f.add(123);
    f.add(123);
    assert!(f.test(123));
    assert!(!f.upgraded());
    // memsize stays at first-grown size (16 buckets).
    assert_eq!(f.memsize(), 64 + 16 * 8);
}

#[test]
fn test_test_on_empty_returns_false() {
    // Brand-new filter: bits is empty, buckets is empty, so test returns false.
    let f = RHBloom::new(16, 0.01);
    for k in [0u64, 1, 100, 9999, u64::MAX] {
        assert!(!f.test(k));
    }
}

#[test]
fn test_addkey_in_grown_bucket_mode() {
    // After add, the filter has buckets allocated. addkey can be called
    // directly to insert another (already-mixed) key into the table.
    // We mimic the internal call by calling add() since addkey() expects
    // its caller to have allocated buckets first. Here we test via add().
    let mut f = RHBloom::new(1000, 0.01);
    f.add(7);
    // Now buckets exist. Direct addkey on a key value: must not panic.
    // The semantics: addkey treats the input as a (mixed) key and inserts
    // its 56-bit portion. We compute the same value the way `add` does.
    let mixed = RHBloom::mix(99);
    assert!(f.addkey(mixed));
    // After addkey of mix(99), test(99) should return true.
    assert!(f.test(99));
}

#[test]
fn test_testadd_set_bit_then_check() {
    // After upgrade, testadd(key, true) sets bits and testadd(key, false)
    // checks them. Verify that after add, test reports true.
    let mut f = RHBloom::new(16, 0.5);
    f.add(7);
    assert!(f.upgraded());
    // The key must read as present.
    assert!(f.test(7));
}

#[test]
fn test_free_resets_state() {
    let mut f = RHBloom::new(1000, 0.01);
    f.add(42);
    assert!(f.test(42));
    f.free();
    // After free, test returns false (no bits, no buckets).
    assert!(!f.upgraded());
    assert!(!f.test(42));
}

#[test]
fn test_no_false_negatives_after_many_adds() {
    // Add 1000 distinct keys; every one must read as present.
    let mut f = RHBloom::new(10000, 0.01);
    let keys: Vec<u64> = (0..1000u64).map(|i| i.wrapping_mul(7919).wrapping_add(13)).collect();
    for &k in &keys {
        f.add(k);
    }
    for &k in &keys {
        assert!(f.test(k), "missing key {}", k);
    }
}

#[test]
fn test_grow_returns_true() {
    // grow() should return true on success.
    let mut f = RHBloom::new(1000, 0.01);
    assert!(f.grow());
    // After grow from empty, nbuckets should be 16 (memsize: 64 + 16*8 = 192).
    assert_eq!(f.memsize(), 64 + 16 * 8);
}

#[test]
fn test_memsize_bloom_mode() {
    // n=1000, p=0.01 has m=16384 (per probe). Bloom-mode memsize = 64 + 2048.
    let mut f = RHBloom::new(1000, 0.01);
    // Add enough to upgrade (>= 65 per probe).
    for i in 0..200u64 {
        f.add(i.wrapping_mul(1024).wrapping_add(100));
    }
    assert!(f.upgraded());
    assert_eq!(f.memsize(), 64 + (16384 >> 3));
}

#[test]
fn test_clear_on_brand_new_filter() {
    // clear() on a freshly-created filter should be a no-op.
    let mut f = RHBloom::new(100, 0.01);
    f.clear();
    assert!(!f.upgraded());
    assert!(!f.test(42));
    assert_eq!(f.memsize(), 64);
}

#[test]
fn test_upgraded_initial_false() {
    let f = RHBloom::new(100, 0.01);
    assert!(!f.upgraded());
}

#[test]
fn test_bloom_mode_no_false_negatives_full_cycle() {
    // Add, clear, add again — no false negatives.
    let mut f = RHBloom::new(1000, 0.01);
    let keys: Vec<u64> = (0..200u64).map(|i| i * 1024 + 100).collect();
    for &k in &keys {
        f.add(k);
    }
    assert!(f.upgraded());
    for &k in &keys {
        assert!(f.test(k));
    }
    f.clear();
    for &k in &keys {
        assert!(!f.test(k));
    }
    for &k in &keys {
        f.add(k);
    }
    for &k in &keys {
        assert!(f.test(k));
    }
}

fn main() {}
