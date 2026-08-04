use rhbloom::rhbloom::RHBloom;

#[test]
fn test_mix_zero() {
    // Empirically verified with C: my_mix(0) = 0
    assert_eq!(RHBloom::mix(0), 0);
}

#[test]
fn test_mix_one() {
    // C output: my_mix(1) = 6238072747940578789
    assert_eq!(RHBloom::mix(1), 6238072747940578789u64);
}

#[test]
fn test_mix_100() {
    // C output: my_mix(100) = 2824278126137619252
    assert_eq!(RHBloom::mix(100), 2824278126137619252u64);
}

#[test]
fn test_mix_1000() {
    // C output: my_mix(1000) = 13948604908503886551
    assert_eq!(RHBloom::mix(1000), 13948604908503886551u64);
}

#[test]
fn test_mix_max_u64() {
    // C output: my_mix(UINT64_MAX) = 13029008266876403067
    assert_eq!(RHBloom::mix(u64::MAX), 13029008266876403067u64);
}

#[test]
fn test_mix_extra() {
    // C output: my_mix(42) = 12058926934050108962
    assert_eq!(RHBloom::mix(42u64), 12058926934050108962u64);
    // C output: my_mix(UINT64_MAX-1) = 15719503542151743746
    assert_eq!(RHBloom::mix(u64::MAX - 1), 15719503542151743746u64);
    // C output: my_mix(2^48 - 1) = 9305294878043952554
    assert_eq!(RHBloom::mix(281474976710655u64), 9305294878043952554u64);
    // C output: my_mix(2^48) = 3730009134440247217
    assert_eq!(RHBloom::mix(281474976710656u64), 3730009134440247217u64);
    // C output: my_mix(2^40) = 48217637115032568
    assert_eq!(RHBloom::mix(1099511627776u64), 48217637115032568u64);
}

#[test]
fn test_new_n_less_than_16_treated_as_16() {
    // The C code clamps n < 16 to 16. Verified that the C library returns
    // identical memsize for n=0 and n=16 (both 64). Internally, m and k
    // should be identical for n=0 and n=16.
    let r0 = RHBloom::new(0, 0.01);
    let r16 = RHBloom::new(16, 0.01);
    // Same m and k means same memsize for fresh filters.
    assert_eq!(r0.memsize(), r16.memsize());
    // Both should report not upgraded.
    assert!(!r0.upgraded());
    assert!(!r16.upgraded());
    // Both should test false for any key.
    assert!(!r0.test(5));
    assert!(!r16.test(5));
}

#[test]
fn test_new_initial_state() {
    // Fresh filter with n=16 p=0.01.
    // C library returns memsize=64 (struct only) and upgraded=false.
    // Rust struct size differs but no buckets and no bits should be allocated.
    let r = RHBloom::new(16, 0.01);
    assert!(!r.upgraded());
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>());
    // Test on empty filter returns false.
    assert!(!r.test(0));
    assert!(!r.test(5));
    assert!(!r.test(u64::MAX));
}

#[test]
fn test_new_initial_state_n100() {
    // Fresh filter with n=100 p=0.01.
    let r = RHBloom::new(100, 0.01);
    assert!(!r.upgraded());
    // No buckets/bits allocated yet.
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>());
    assert!(!r.test(0));
    assert!(!r.test(42));
    assert!(!r.test(u64::MAX));
}

#[test]
fn test_new_initial_state_n1000() {
    // Fresh filter with n=1000 p=0.01.
    let r = RHBloom::new(1000, 0.01);
    assert!(!r.upgraded());
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>());
    assert!(!r.test(0));
    assert!(!r.test(5));
    assert!(!r.test(u64::MAX));
}

#[test]
fn test_add_then_test_basic() {
    let mut r = RHBloom::new(16, 0.01);
    // Adding a key. C confirms test_i=1 after add.
    assert!(r.add(0));
    assert!(r.test(0));
}

#[test]
fn test_add_robinhood_few_keys_n10000() {
    // n=10000, p=0.01, count=5. C: upgraded=0, all test=1, memsize=192.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..5u64 {
        assert!(r.add(i));
    }
    assert!(!r.upgraded());
    for i in 0..5u64 {
        assert!(r.test(i), "key {} should be present", i);
    }
    // First grow allocates 16 buckets => memsize = struct_size + 16*8.
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 16 * 8);
}

#[test]
fn test_add_robinhood_100_keys() {
    // n=10000 p=0.01 count=100. C reports: upgraded=0, all hits, memsize=2112
    // 2112 - 64 (C struct) = 2048 bytes. So nbuckets = 2048/8 = 256.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..100u64 {
        assert!(r.add(i));
    }
    assert!(!r.upgraded());
    for i in 0..100u64 {
        assert!(r.test(i));
    }
    // 256 buckets => memsize = struct + 256*8.
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 256 * 8);
}

#[test]
fn test_upgrade_to_bloom_n16() {
    // n=16 p=0.01, upgrade_at=1. So a single add should trigger upgrade.
    let mut r = RHBloom::new(16, 0.01);
    assert!(!r.upgraded());
    r.add(0);
    assert!(r.upgraded());
    // After upgrade, all keys we add should be testable as present.
    for i in 1..5u64 {
        r.add(i);
    }
    for i in 0..5u64 {
        assert!(r.test(i));
    }
}

#[test]
fn test_upgrade_to_bloom_n100() {
    // n=100 p=0.01, upgrade_at=1. Same as n=16.
    let mut r = RHBloom::new(100, 0.01);
    assert!(!r.upgraded());
    r.add(42);
    assert!(r.upgraded());
}

#[test]
fn test_upgrade_to_bloom_n1000() {
    // n=1000 p=0.01, upgrade_at=65 (per C helper).
    let mut r = RHBloom::new(1000, 0.01);
    // After 64 adds, should still NOT be upgraded.
    for i in 0..64u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    // After the 65th add, should be upgraded.
    r.add(64);
    assert!(r.upgraded());
}

#[test]
fn test_upgrade_to_bloom_n10000() {
    // n=10000 p=0.01, upgrade_at=513.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..512u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    r.add(512);
    assert!(r.upgraded());
}

#[test]
fn test_no_false_negatives_after_upgrade_n1000() {
    // n=1000 p=0.01 count=1000. C reports added_hits=1000 (no false negatives),
    // fp_hits=2 (small false positive count), upgraded=1.
    let mut r = RHBloom::new(1000, 0.01);
    for i in 0..1000u64 {
        assert!(r.add(i));
    }
    assert!(r.upgraded());
    // All added keys must test true (no false negatives).
    let mut added_hits = 0;
    for i in 0..1000u64 {
        if r.test(i) {
            added_hits += 1;
        }
    }
    assert_eq!(added_hits, 1000);
}

#[test]
fn test_clear_resets_robinhood() {
    // n=10000 p=0.01 count=5: in robinhood mode. After clear, the filter
    // still has the buckets allocated but they're zeroed. C reports:
    // - Before/after clear: upgraded=0, memsize=192 (so nbuckets unchanged)
    // - After clear, test_0=1 unexpectedly because the test for an empty
    //   bucket array uses dib=0 short-circuit. Actually C output shows
    //   test_0=1, test_1..test_4=0. This is because key 0 hashes to a
    //   bucket where dib==0, so the loop matches RHBLOOM_KEY(0)==key, yes=true.
    // We replicate that exact behavior.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..5u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    let memsize_before = r.memsize();
    r.clear();
    assert!(!r.upgraded());
    assert_eq!(r.memsize(), memsize_before);
    // After clear, behavior matches the C library exactly.
    // For key 0 (mix(0)=0), it lands at bucket 0 where the cleared entry has
    // key=0 and dib=0. yes=true, no=true => returns yes=true.
    assert!(r.test(0));
    // The other keys land somewhere where key won't match the zero entry.
    assert!(!r.test(1));
    assert!(!r.test(2));
    assert!(!r.test(3));
    assert!(!r.test(4));
}

#[test]
fn test_clear_resets_bloom() {
    // n=10000 p=0.01 count=1000 -> upgraded. After clear:
    // - upgraded=1 still (m,k preserved), memsize unchanged.
    // - all test values (0..1000) should be 0.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..1000u64 {
        r.add(i);
    }
    assert!(r.upgraded());
    let memsize_before = r.memsize();
    r.clear();
    assert!(r.upgraded());
    assert_eq!(r.memsize(), memsize_before);
    // After clear, no key should be present.
    for i in 0..1000u64 {
        assert!(!r.test(i), "after clear, key {} should not be present", i);
    }
}

#[test]
fn test_clear_on_fresh_filter() {
    // Clearing a freshly created filter (no buckets, no bits) should be a no-op.
    let mut r = RHBloom::new(16, 0.01);
    let memsize = r.memsize();
    r.clear();
    assert!(!r.upgraded());
    assert_eq!(r.memsize(), memsize);
    assert!(!r.test(5));
}

#[test]
fn test_add_returns_true() {
    // C's rhbloom_add returns true on success (only fails on OOM, which we
    // don't simulate). Confirm Rust returns true.
    let mut r = RHBloom::new(100, 0.01);
    assert!(r.add(1));
    assert!(r.add(2));
    assert!(r.add(3));
}

#[test]
fn test_test_on_unknown_keys_in_robinhood_no_false_positives_from_seed() {
    // In robinhood mode, test() should never return true for keys that
    // weren't added (deterministically), barring intentional collisions.
    // Verified empirically with the C helper for these inputs.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..5u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    // None of these large keys were added, none should hit.
    assert!(!r.test(1000));
    assert!(!r.test(2000));
    assert!(!r.test(50000));
    assert!(!r.test(u64::MAX));
}

#[test]
fn test_testadd_false_on_empty_bloom_returns_false_for_fresh() {
    // testadd is a public method. After upgrade the bits are allocated.
    // Force an upgrade then call testadd directly with add=false on a key
    // that wasn't inserted.
    let mut r = RHBloom::new(100, 0.01);
    r.add(1); // triggers upgrade for n=100
    assert!(r.upgraded());
    // Insert key=2 via testadd directly with add=true.
    let mixed = RHBloom::mix(2);
    r.testadd(mixed, true);
    // Now testadd with add=false should return true for the inserted key.
    assert!(r.testadd(mixed, false));
    // For the key we added via add(), should still return true.
    let mixed1 = RHBloom::mix(1);
    assert!(r.testadd(mixed1, false));
}

#[test]
fn test_addkey_directly_in_robinhood() {
    // addkey is a public method that operates on the internal robinhood
    // table directly. It is used in robinhood mode. Replicate the wrapping
    // that add() does (mix the key first, then call addkey).
    let mut r = RHBloom::new(10000, 0.01);
    // First call add() once to make grow() allocate buckets.
    r.add(123);
    assert!(!r.upgraded());
    // Now call addkey directly with a mixed key.
    let mixed = RHBloom::mix(456);
    assert!(r.addkey(mixed));
    // Calling addkey again with the same key should still return true (no-op).
    assert!(r.addkey(mixed));
    // The key should be testable.
    assert!(r.test(456));
}

#[test]
fn test_grow_initial_allocation() {
    // grow() on a fresh filter should allocate 16 buckets.
    let mut r = RHBloom::new(10000, 0.01);
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>());
    assert!(r.grow());
    assert!(!r.upgraded());
    // 16 buckets allocated.
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 16 * 8);
}

#[test]
fn test_free_resets_state() {
    // free() resets internal state: bits empty, buckets empty, count=0.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..5u64 {
        r.add(i);
    }
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 16 * 8);
    r.free();
    // After free, memsize should be just the struct.
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>());
    assert!(!r.upgraded());
    // No keys should be present.
    assert!(!r.test(0));
    assert!(!r.test(1));
    assert!(!r.test(2));
}

#[test]
fn test_upgraded_returns_correct_state() {
    // upgraded() returns false for fresh and robinhood-mode filters,
    // and true after upgrade.
    let mut r = RHBloom::new(1000, 0.01);
    assert!(!r.upgraded());
    // Add fewer than 65 keys: stays in robinhood.
    for i in 0..32u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    // Add until upgrade.
    for i in 32..65u64 {
        r.add(i);
    }
    assert!(r.upgraded());
}

#[test]
fn test_memsize_after_upgrade_n100() {
    // n=100 p=0.01 count=100. C reports memsize=192.
    // 192 - 64 (C struct) = 128 bytes for bits. So m>>3 = 128, m=1024.
    let mut r = RHBloom::new(100, 0.01);
    for i in 0..100u64 {
        r.add(i);
    }
    assert!(r.upgraded());
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 128);
}

#[test]
fn test_memsize_after_upgrade_n1000() {
    // n=1000 p=0.01 count=1000. C reports memsize=2112.
    // 2112 - 64 = 2048 bytes for bits.
    let mut r = RHBloom::new(1000, 0.01);
    for i in 0..1000u64 {
        r.add(i);
    }
    assert!(r.upgraded());
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 2048);
}

#[test]
fn test_memsize_after_upgrade_n10000() {
    // n=10000 p=0.01 count=10000. C reports memsize=16448.
    // 16448 - 64 = 16384 bytes for bits.
    let mut r = RHBloom::new(10000, 0.01);
    for i in 0..10000u64 {
        r.add(i);
    }
    assert!(r.upgraded());
    assert_eq!(r.memsize(), std::mem::size_of::<RHBloom>() + 16384);
}

#[test]
fn test_no_false_negatives_after_upgrade_large() {
    // Replicates a scaled-down version of the C test suite. Add 5000 keys
    // and ensure ALL of them test true (no false negatives).
    let mut r = RHBloom::new(5000, 0.05);
    for i in 0..5000u64 {
        r.add(i);
    }
    assert!(r.upgraded());
    let mut hits = 0;
    for i in 0..5000u64 {
        if r.test(i) {
            hits += 1;
        }
    }
    assert_eq!(hits, 5000);
}

#[test]
fn test_no_false_negatives_robinhood_intermediate() {
    // n=1000 p=0.01: stays in robinhood until 65 adds. Add 60 keys.
    let mut r = RHBloom::new(1000, 0.01);
    for i in 0..60u64 {
        r.add(i);
    }
    assert!(!r.upgraded());
    for i in 0..60u64 {
        assert!(r.test(i), "key {} should be present in robinhood", i);
    }
}

#[test]
fn test_test_returns_false_on_filter_with_buckets_but_no_match() {
    // After grow allocates buckets, test on a non-existent key should be false.
    let mut r = RHBloom::new(10000, 0.01);
    r.add(1);
    r.add(2);
    r.add(3);
    assert!(!r.upgraded());
    assert!(!r.test(999999u64));
    assert!(!r.test(424242u64));
}

fn main() {}
