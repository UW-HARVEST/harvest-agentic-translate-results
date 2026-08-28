//! Level 3: the hash map proper - `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func` - driven
//! through the same macro expansions the C header defines.

mod common;

use common::*;
use std::ffi::c_void;

/// Reset both libraries' global hash seed so the two tables draw the same
/// per-table seed.
unsafe fn reseed(seed: usize) {
    let libs = libs();
    libs.c.rand_seed(seed);
    libs.rs.rand_seed(seed);
}

#[test]
fn hmget_on_null_map() {
    let _g = guard();
    let libs = libs();
    unsafe {
        reseed(0x3141_5926);
        let fmt = Fmt::BinaryKV;
        let (ct, ci) = hmgeti_i32(&libs.c, std::ptr::null_mut(), 7);
        let (rt, ri) = hmgeti_i32(&libs.rs, std::ptr::null_mut(), 7);
        assert_eq!(ci, ri, "index from empty map");
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));
        libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
        libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
    }
}

#[test]
fn hmget_key_ts_on_null_map() {
    let _g = guard();
    let libs = libs();
    unsafe {
        reseed(1);
        let mut key = 42i32;
        let mut ctemp = 12345isize;
        let mut rtemp = 12345isize;
        let ct = libs.c.hmget_key_ts(
            std::ptr::null_mut(),
            8,
            &mut key as *mut i32 as *mut c_void,
            4,
            &mut ctemp,
            HM_BINARY,
        );
        let rt = libs.rs.hmget_key_ts(
            std::ptr::null_mut(),
            8,
            &mut key as *mut i32 as *mut c_void,
            4,
            &mut rtemp,
            HM_BINARY,
        );
        assert_eq!(ctemp, rtemp, "temp out-param");
        assert_eq!(snap_hm(ct, Fmt::BinaryKV), snap_hm(rt, Fmt::BinaryKV));
        libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
        libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
    }
}

#[test]
fn hmdel_on_null_map() {
    let _g = guard();
    let libs = libs();
    unsafe {
        let (ct, cr) = hmdel_i32(&libs.c, std::ptr::null_mut(), 3);
        let (rt, rr) = hmdel_i32(&libs.rs, std::ptr::null_mut(), 3);
        assert!(ct.is_null() && rt.is_null(), "hmdel on NULL must stay NULL");
        assert_eq!(cr, rr);
    }
}

#[test]
fn hmfree_on_null_is_noop() {
    let _g = guard();
    let libs = libs();
    unsafe {
        libs.c.hmfree_func(std::ptr::null_mut(), 8);
        libs.rs.hmfree_func(std::ptr::null_mut(), 8);
    }
}

#[test]
fn binary_map_sequential_inserts() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;
    for &seed in &[0x3141_5926usize, 0, 1, usize::MAX] {
        unsafe {
            reseed(seed);
            let mut ct: *mut u8 = std::ptr::null_mut();
            let mut rt: *mut u8 = std::ptr::null_mut();
            // 1000 inserts walk the table through slot counts 8 -> 2048,
            // i.e. many `stbds_make_hash_index` rehash passes.
            for k in 0..1000i32 {
                ct = hmput_i32(&libs.c, ct, k, k * 3 + 1);
                rt = hmput_i32(&libs.rs, rt, k, k * 3 + 1);
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "insert k={k} seed={seed:#x}"
                );
            }
            // Every key must still be findable, with the same index.
            for k in 0..1000i32 {
                let (c2, ci) = hmgeti_i32(&libs.c, ct, k);
                let (r2, ri) = hmgeti_i32(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "lookup k={k}");
                assert!(ci >= 0, "key {k} lost");
            }
            // Absent keys.
            for k in 1000..1100i32 {
                let (c2, ci) = hmgeti_i32(&libs.c, ct, k);
                let (r2, ri) = hmgeti_i32(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "absent lookup k={k}");
                assert_eq!(ci, -1);
            }
            assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));
            libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
            libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
        }
    }
}

#[test]
fn binary_map_overwrite_existing() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;
    unsafe {
        reseed(0xBEEF);
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for k in 0..64i32 {
            ct = hmput_i32(&libs.c, ct, k, k);
            rt = hmput_i32(&libs.rs, rt, k, k);
        }
        // Re-put the same keys: must hit the "key already present" branch and
        // report the existing index rather than appending.
        for k in 0..64i32 {
            ct = hmput_i32(&libs.c, ct, k, k * 100);
            rt = hmput_i32(&libs.rs, rt, k, k * 100);
            assert_eq!(
                temp_of(ct, 8),
                temp_of(rt, 8),
                "overwrite index for k={k}"
            );
            assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "overwrite k={k}");
        }
        libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
        libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
    }
}

#[test]
fn binary_map_delete_patterns() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;

    // Deleting in different orders exercises the tail-swap, the tombstone
    // rebuild threshold and the shrink threshold in different sequences.
    let orders: [&str; 4] = ["forward", "backward", "even_then_odd", "strided"];
    for order in orders {
        unsafe {
            reseed(0x1234_5678);
            let mut ct: *mut u8 = std::ptr::null_mut();
            let mut rt: *mut u8 = std::ptr::null_mut();
            let n = 300i32;
            for k in 0..n {
                ct = hmput_i32(&libs.c, ct, k, k ^ 0x55);
                rt = hmput_i32(&libs.rs, rt, k, k ^ 0x55);
            }
            assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "{order}: after fill");

            let keys: Vec<i32> = match order {
                "forward" => (0..n).collect(),
                "backward" => (0..n).rev().collect(),
                "even_then_odd" => (0..n)
                    .filter(|k| k % 2 == 0)
                    .chain((0..n).filter(|k| k % 2 == 1))
                    .collect(),
                _ => {
                    let mut v = Vec::new();
                    for s in 0..7 {
                        for k in (s..n).step_by(7) {
                            v.push(k);
                        }
                    }
                    v
                }
            };

            for (step, k) in keys.iter().enumerate() {
                let (c2, cr) = hmdel_i32(&libs.c, ct, *k);
                let (r2, rr) = hmdel_i32(&libs.rs, rt, *k);
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "{order}: hmdel result step {step} key {k}");
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "{order}: state after deleting {k} (step {step})"
                );
            }
            // Deleting an already-absent key.
            for k in [0i32, n - 1, n + 5, -1] {
                let (c2, cr) = hmdel_i32(&libs.c, ct, k);
                let (r2, rr) = hmdel_i32(&libs.rs, rt, k);
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "{order}: absent hmdel {k}");
                assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));
            }
            libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
            libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
        }
    }
}

#[test]
fn binary_map_random_operation_mix() {
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;
    for trial in 0..6u64 {
        unsafe {
            reseed(0x9E37_79B9 ^ trial as usize);
            let mut ct: *mut u8 = std::ptr::null_mut();
            let mut rt: *mut u8 = std::ptr::null_mut();
            let mut rng = Rng::new(0xC0FFEE + trial * 7919);
            for step in 0..3000 {
                // Small key space so insert/delete/refill collide heavily and
                // tombstones accumulate.
                let k = (rng.below(120) as i32) - 20;
                match rng.below(10) {
                    0..=4 => {
                        let v = rng.next_i32();
                        ct = hmput_i32(&libs.c, ct, k, v);
                        rt = hmput_i32(&libs.rs, rt, k, v);
                    }
                    5..=7 => {
                        let (c2, ci) = hmgeti_i32(&libs.c, ct, k);
                        let (r2, ri) = hmgeti_i32(&libs.rs, rt, k);
                        ct = c2;
                        rt = r2;
                        assert_eq!(ci, ri, "trial {trial} step {step}: get {k}");
                    }
                    _ => {
                        let (c2, cr) = hmdel_i32(&libs.c, ct, k);
                        let (r2, rr) = hmdel_i32(&libs.rs, rt, k);
                        ct = c2;
                        rt = r2;
                        assert_eq!(cr, rr, "trial {trial} step {step}: del {k}");
                    }
                }
                assert_eq!(
                    snap_hm(ct, fmt),
                    snap_hm(rt, fmt),
                    "trial {trial} step {step}"
                );
            }
            libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
            libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
        }
    }
}

#[test]
fn binary_map_wide_keys() {
    // `struct { int key[2]; int b, c, d; }`: 8-byte key, 20-byte element, so
    // both the memcmp key compare and the element stride differ from the
    // simple case.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::Binary2KV;
    let es = fmt.elemsize();
    let ks = fmt.keysize();

    unsafe fn put(lib: &Lib, t: *mut u8, key: [i32; 2], b: i32) -> *mut u8 {
        let mut k = key;
        let t = lib.hmput_key(
            t as *mut c_void,
            20,
            k.as_mut_ptr() as *mut c_void,
            8,
            HM_BINARY,
        );
        let temp = temp_of(t, 20);
        let slot = t.offset(temp * 20);
        (slot as *mut i32).write_unaligned(key[0]);
        (slot.add(4) as *mut i32).write_unaligned(key[1]);
        (slot.add(8) as *mut i32).write_unaligned(b);
        (slot.add(12) as *mut i32).write_unaligned(b + 1);
        (slot.add(16) as *mut i32).write_unaligned(b + 2);
        t
    }

    unsafe {
        reseed(0x0BAD_F00D);
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for k in 0..400i32 {
            let key = [k, k * 7 + 1];
            ct = put(&libs.c, ct, key, k);
            rt = put(&libs.rs, rt, key, k);
            assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "wide insert k={k}");
        }
        for k in 0..400i32 {
            let mut key = [k, k * 7 + 1];
            let c2 = libs.c.hmget_key(
                ct as *mut c_void,
                es,
                key.as_mut_ptr() as *mut c_void,
                ks,
                HM_BINARY,
            );
            let r2 = libs.rs.hmget_key(
                rt as *mut c_void,
                es,
                key.as_mut_ptr() as *mut c_void,
                ks,
                HM_BINARY,
            );
            ct = c2;
            rt = r2;
            assert_eq!(temp_of(ct, es), temp_of(rt, es), "wide get k={k}");
        }
        for k in (0..400i32).step_by(3) {
            let mut key = [k, k * 7 + 1];
            ct = libs.c.hmdel_key(
                ct as *mut c_void,
                es,
                key.as_mut_ptr() as *mut c_void,
                ks,
                0,
                HM_BINARY,
            );
            rt = libs.rs.hmdel_key(
                rt as *mut c_void,
                es,
                key.as_mut_ptr() as *mut c_void,
                ks,
                0,
                HM_BINARY,
            );
            assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "wide del k={k}");
        }
        libs.c.hmfree_func(ct.sub(es) as *mut c_void, es);
        libs.rs.hmfree_func(rt.sub(es) as *mut c_void, es);
    }
}

#[test]
fn hmget_key_ts_matches_on_populated_map() {
    let _g = guard();
    let libs = libs();
    unsafe {
        reseed(77);
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for k in 0..200i32 {
            ct = hmput_i32(&libs.c, ct, k * 2, k);
            rt = hmput_i32(&libs.rs, rt, k * 2, k);
        }
        for k in -5..410i32 {
            let mut key = k;
            let mut ctemp = -999isize;
            let mut rtemp = -999isize;
            ct = libs.c.hmget_key_ts(
                ct as *mut c_void,
                8,
                &mut key as *mut i32 as *mut c_void,
                4,
                &mut ctemp,
                HM_BINARY,
            );
            rt = libs.rs.hmget_key_ts(
                rt as *mut c_void,
                8,
                &mut key as *mut i32 as *mut c_void,
                4,
                &mut rtemp,
                HM_BINARY,
            );
            assert_eq!(ctemp, rtemp, "hmget_key_ts temp for k={k}");
        }
        assert_eq!(snap_hm(ct, Fmt::BinaryKV), snap_hm(rt, Fmt::BinaryKV));
        libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
        libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
    }
}

#[test]
fn map_without_table_after_hmput_default() {
    // `stbds_hmput_default` creates the array but no hash table; a subsequent
    // get must take the `table == 0` branch.
    let _g = guard();
    let libs = libs();
    unsafe {
        let ct = libs.c.hmput_default(std::ptr::null_mut(), 8);
        let rt = libs.rs.hmput_default(std::ptr::null_mut(), 8);
        let (ct, ci) = hmgeti_i32(&libs.c, ct, 5);
        let (rt, ri) = hmgeti_i32(&libs.rs, rt, 5);
        assert_eq!(ci, ri);
        assert_eq!(snap_hm(ct, Fmt::BinaryKV), snap_hm(rt, Fmt::BinaryKV));

        let (ct, cr) = hmdel_i32(&libs.c, ct, 5);
        let (rt, rr) = hmdel_i32(&libs.rs, rt, 5);
        assert_eq!(cr, rr);
        assert_eq!(snap_hm(ct, Fmt::BinaryKV), snap_hm(rt, Fmt::BinaryKV));

        libs.c.hmfree_func(ct.sub(8) as *mut c_void, 8);
        libs.rs.hmfree_func(rt.sub(8) as *mut c_void, 8);
    }
}
