//! Harness sanity checks: both `.so`s load, every symbol resolves, and the
//! mirrored C layouts in `tests/common/mod.rs` really match what the C library
//! writes into memory.

mod common;

use common::*;
use std::ffi::c_void;

#[test]
fn t00_both_libraries_load_and_expose_all_16_symbols() {
    let p = pair();
    // Simply reaching here means every `sym!()` lookup succeeded in both.
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}

#[test]
fn t01_layout_sanity_array_header() {
    assert_eq!(std::mem::size_of::<ArrayHeader>(), 32);
    assert_eq!(std::mem::size_of::<HashBucket>(), 128);
    assert_eq!(std::mem::size_of::<StringArena>(), 24);
    // temp_key .. slot_count_log2 = 9 * 8, + arena 24, + storage 8
    assert_eq!(std::mem::size_of::<HashIndex>(), 9 * 8 + 24 + 8);
}

/// The C library must actually populate the fields where our mirror says they
/// are. `stbds_shmode_func` sets `slot_count = 8`, `seed = <the global seed>`
/// and `string.mode = mode`, so a known reseed pins all three down.
#[test]
fn t02_layout_matches_real_c_memory() {
    let p = pair();
    let _g = lock();
    for lib in [&p.c, &p.rs] {
        unsafe {
            (lib.rand_seed)(0xDEAD_BEEF_1234_5678);
            let elemsize = 16usize;
            let t = (lib.shmode_func)(elemsize, SH_ARENA);
            let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
            let h = header(raw);
            assert_eq!((*h).length, 1, "{}: length", lib.name);
            assert_eq!((*h).capacity, 4, "{}: capacity", lib.name);
            let table = (*h).hash_table as *mut HashIndex;
            assert!(!table.is_null(), "{}: table", lib.name);
            assert_eq!((*table).slot_count, 8, "{}: slot_count", lib.name);
            assert_eq!((*table).slot_count_log2, 3, "{}: log2", lib.name);
            assert_eq!((*table).used_count, 0, "{}: used_count", lib.name);
            assert_eq!((*table).used_count_threshold, 6, "{}: used_thr", lib.name);
            assert_eq!((*table).tombstone_count_threshold, 1, "{}: tomb_thr", lib.name);
            assert_eq!((*table).used_count_shrink_threshold, 0, "{}: shrink", lib.name);
            assert_eq!(
                (*table).seed, 0xDEAD_BEEF_1234_5678,
                "{}: seed",
                lib.name
            );
            assert_eq!((*table).string.mode, SH_ARENA as u8, "{}: mode", lib.name);
            assert!((*table).string.storage.is_null(), "{}: arena", lib.name);
            // all buckets empty
            let b = (*table).storage;
            for j in 0..BUCKET_LENGTH {
                assert_eq!((*b).hash[j], 0, "{}: bh{}", lib.name, j);
                assert_eq!((*b).index[j], -1, "{}: bi{}", lib.name, j);
            }
            hmfree(lib, t, elemsize);
        }
    }
}

/// The seed self-advance `seed = seed*a + b` from `stbds_load_32_or_64` must be
/// identical on both sides.
#[test]
fn t03_seed_advance_identical() {
    diff("seed advance", |lib, log| unsafe {
        for s in [0usize, 1, 0x31415926, usize::MAX, 0xA5A5_5A5A_A5A5_5A5A] {
            (lib.rand_seed)(s);
            for _ in 0..8 {
                let t = (lib.shmode_func)(16, SH_NONE);
                let raw = (t as *mut u8).sub(16) as *mut c_void;
                let table = (*header(raw)).hash_table as *mut HashIndex;
                log.usz("table_seed", (*table).seed);
                hmfree(lib, t, 16);
            }
        }
    });
}
