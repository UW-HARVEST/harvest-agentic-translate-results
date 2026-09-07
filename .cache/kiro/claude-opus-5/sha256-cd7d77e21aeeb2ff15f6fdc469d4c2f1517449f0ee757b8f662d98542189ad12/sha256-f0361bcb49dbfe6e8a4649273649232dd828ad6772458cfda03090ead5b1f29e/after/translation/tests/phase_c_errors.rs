//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Rejections that return a value or a sentinel are compared in-process.
//! Rejections that abort or fault are run in a forked child so that the exact
//! termination signal of C and Rust can be compared (not merely "both failed").

mod common;

use common::*;
use std::ffi::c_void;
use std::ptr;

fn k32(v: u32) -> [u8; 4] {
    v.to_ne_bytes()
}

unsafe fn hdr_of(a: *mut c_void) -> (usize, usize, bool, isize) {
    let h = (a as *const u8).sub(HEADER);
    (
        ptr::read_unaligned(h as *const usize),
        ptr::read_unaligned(h.add(8) as *const usize),
        !ptr::read_unaligned(h.add(16) as *const *const u8).is_null(),
        ptr::read_unaligned(h.add(24) as *const isize),
    )
}

// ===========================================================================
// rows 1-3: stbds_arrgrowf
// ===========================================================================

/// row 1: `min_cap <= arrcap(a)` — the growth request is rejected.
#[test]
fn err_01_arrgrowf_rejects_shrink() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            let ca = (c.arrgrowf)(ptr::null_mut(), elemsize, 0, 4);
            let ra = (r.arrgrowf)(ptr::null_mut(), elemsize, 0, 4);
            for min_cap in [0usize, 1, 2, 3, 4] {
                let ca2 = (c.arrgrowf)(ca, elemsize, 0, min_cap);
                let ra2 = (r.arrgrowf)(ra, elemsize, 0, min_cap);
                same("row1 identity", ca2 == ca, ra2 == ra);
                assert!(ca2 == ca, "row1: C did reallocate");
                same("row1 header", hdr_of(ca2), hdr_of(ra2));
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

/// row 2: `a == NULL, addlen == 0, min_cap == 0` → `NULL`, no allocation.
#[test]
fn err_02_arrgrowf_null_null() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 4096] {
            let ca = (c.arrgrowf)(ptr::null_mut(), elemsize, 0, 0);
            let ra = (r.arrgrowf)(ptr::null_mut(), elemsize, 0, 0);
            same("row2", ca.is_null(), ra.is_null());
            assert!(ca.is_null(), "row2: C returned non-NULL");
        }
    }
}

/// row 3: allocation failure → the C code writes through `NULL+32`.
#[test]
fn err_03_arrgrowf_alloc_failure() {
    let _g = lock();
    let (c, r) = libs();
    let huge = 1usize << 62;
    let co = in_child(|| unsafe {
        (c.arrgrowf)(ptr::null_mut(), 1, 0, huge);
    });
    let ro = in_child(|| unsafe {
        (r.arrgrowf)(ptr::null_mut(), 1, 0, huge);
    });
    same("row3 outcome", co, ro);
    assert!(is_fatal(co), "row3: C did not fault, got {co:?}");
}

/// row 4: `stbds_arrfreef(NULL)` frees a wild pointer.
#[test]
fn err_04_arrfreef_null() {
    let _g = lock();
    let (c, r) = libs();
    let co = in_child(|| unsafe { (c.arrfreef)(ptr::null_mut()) });
    let ro = in_child(|| unsafe { (r.arrfreef)(ptr::null_mut()) });
    same("row4 outcome", co, ro);
    assert!(is_fatal(co), "row4: C did not fault, got {co:?}");
}

// ===========================================================================
// rows 5-8: hashing
// ===========================================================================

/// row 5: `stbds_hash_string(NULL, seed)`.
#[test]
fn err_05_hash_string_null() {
    let _g = lock();
    let (c, r) = libs();
    for seed in [0usize, DEFAULT_SEED, usize::MAX] {
        let co = in_child(|| unsafe {
            std::hint::black_box((c.hash_string)(ptr::null_mut(), seed));
        });
        let ro = in_child(|| unsafe {
            std::hint::black_box((r.hash_string)(ptr::null_mut(), seed));
        });
        same("row5 outcome", co, ro);
        assert!(is_fatal(co), "row5: C did not fault, got {co:?}");
    }
}

/// row 6: empty string is well defined.
#[test]
fn err_06_hash_string_empty() {
    let _g = lock();
    let (c, r) = libs();
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX, usize::MAX - 1] {
        let mut e = [0u8; 1];
        unsafe {
            same(
                &format!("row6 seed={seed:#x}"),
                (c.hash_string)(e.as_mut_ptr() as *mut _, seed),
                (r.hash_string)(e.as_mut_ptr() as *mut _, seed),
            );
        }
    }
}

/// row 7: `len == 0` never reads the buffer, even a NULL one.
#[test]
fn err_07_hash_bytes_len0() {
    let _g = lock();
    let (c, r) = libs();
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
        unsafe {
            same(
                &format!("row7 NULL seed={seed:#x}"),
                (c.hash_bytes)(ptr::null_mut(), 0, seed),
                (r.hash_bytes)(ptr::null_mut(), 0, seed),
            );
            let mut b = [0xAAu8; 8];
            same(
                &format!("row7 buf seed={seed:#x}"),
                (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed),
                (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed),
            );
        }
    }
}

/// row 8: `p == NULL, len > 0`.
#[test]
fn err_08_hash_bytes_null_buffer() {
    let _g = lock();
    let (c, r) = libs();
    for len in [1usize, 4, 8, 64] {
        let co = in_child(|| unsafe {
            std::hint::black_box((c.hash_bytes)(ptr::null_mut(), len, DEFAULT_SEED));
        });
        let ro = in_child(|| unsafe {
            std::hint::black_box((r.hash_bytes)(ptr::null_mut(), len, DEFAULT_SEED));
        });
        same(&format!("row8 len={len} outcome"), co, ro);
        assert!(is_fatal(co), "row8: C did not fault, got {co:?}");
    }
}

// ===========================================================================
// row 9: the unreachable make_hash_index assert
// ===========================================================================

/// row 9: the smallest `slot_count` the public API can produce is 8, and the
/// shrink path never goes below it — so the assert is unreachable. Verified by
/// driving the map down to empty and observing `slot_count` in both libraries.
#[test]
fn err_09_min_slot_count_is_8() {
    let _g = lock();
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..400u32 {
        m.put(&format!("row9 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for i in 0..400u32 {
        m.del(&format!("row9 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        let sc = m.snap_c().table.map(|t| t.slot_count).unwrap_or(0);
        assert!(sc >= 8, "row9: slot_count dropped to {sc} in C");
    }
    let t = m.snap_c().table.unwrap();
    assert_eq!(t.slot_count, 8, "row9: final slot_count");
    m.free();

    // and creating a table always starts at 8
    for mode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::from_shmode(16, 8, mode, KeyKind::Binary);
        assert_eq!(m.snap_c().table.unwrap().slot_count, 8);
        m.free();
    }
}

/// row 10: `stbds_hmfree_func(NULL, elemsize)` returns without doing anything.
#[test]
fn err_10_hmfree_null() {
    let _g = lock();
    let (c, r) = libs();
    let co = in_child(|| unsafe {
        for e in [0usize, 1, 8, 16] {
            (c.hmfree_func)(ptr::null_mut(), e);
        }
    });
    let ro = in_child(|| unsafe {
        for e in [0usize, 1, 8, 16] {
            (r.hmfree_func)(ptr::null_mut(), e);
        }
    });
    same("row10 outcome", co, ro);
    assert_eq!(co, Outcome::Exited(0), "row10: C must return quietly");
    // also in-process, so a later divergence would be visible
    unsafe {
        (c.hmfree_func)(ptr::null_mut(), 8);
        (r.hmfree_func)(ptr::null_mut(), 8);
    }
}

// ===========================================================================
// rows 11-17: lookup rejections
// ===========================================================================

/// rows 11-12: `stbds_hm_find_slot` returns `-1` from both of its scan loops.
/// The two `return -1` sites differ only in which half of the bucket the empty
/// slot is found in, which is not directly observable; a large randomized miss
/// corpus over tables at many fill levels covers both.
#[test]
fn err_11_12_find_slot_misses() {
    let _g = lock();
    let mut rng = Rng::new(0x11);
    for &n in &[1usize, 5, 6, 7, 13, 40, 200, 700] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
        let mut present = std::collections::HashSet::new();
        for i in 0..n {
            let k = rng.next_u32();
            present.insert(k);
            m.put(&format!("rows11-12 n={n} put{i}"), &k32(k), STBDS_HM_BINARY);
        }
        let mut misses = 0;
        for i in 0..400 {
            let k = rng.next_u32();
            if present.contains(&k) {
                continue;
            }
            let t = m.get(&format!("rows11-12 n={n} miss{i}"), &k32(k), STBDS_HM_BINARY);
            assert_eq!(t, -1, "rows11-12: C reported a hit for an absent key");
            misses += 1;
        }
        assert!(misses > 300);
        m.free();
    }
}

/// row 13: `hmget_key_ts` on a NULL map allocates and reports `-1`.
#[test]
fn err_13_get_ts_on_null() {
    let _g = lock();
    for elemsize in [8usize, 16, 24] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        let t = m.get_ts("row13", &k32(1), STBDS_HM_BINARY);
        assert_eq!(t, -1, "row13: C must report -1");
        let s = m.snap_c();
        assert!(!s.is_null && s.length == 1 && !s.has_table, "row13: C state");
        assert!(s.elements[0].iter().all(|&b| b == 0), "row13: element 0 zeroed");
        m.free();
    }
}

/// row 14: handle whose `hash_table == NULL`.
#[test]
fn err_14_get_ts_no_table() {
    let _g = lock();
    for elemsize in [8usize, 16] {
        // via hmput_default
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        m.put_default("row14 default");
        m.set_default_tail(0x33);
        for k in 0..8u32 {
            let t = m.get_ts(&format!("row14 ts{k}"), &k32(k), STBDS_HM_BINARY);
            assert_eq!(t, -1, "row14: C must report -1");
            let t = m.get(&format!("row14 g{k}"), &k32(k), STBDS_HM_BINARY);
            assert_eq!(t, -1);
        }
        assert!(!m.snap_c().has_table);
        m.free();

        // via a previous get on NULL
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        m.get_ts("row14b first", &k32(0), STBDS_HM_BINARY);
        for k in 0..8u32 {
            let t = m.get_ts(&format!("row14b ts{k}"), &k32(k), STBDS_HM_BINARY);
            assert_eq!(t, -1);
        }
        m.free();
    }
}

/// row 15: populated table, key absent.
#[test]
fn err_15_get_ts_absent() {
    let _g = lock();
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..50u32 {
        m.put(&format!("row15 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    for i in 50..200u32 {
        let t = m.get_ts(&format!("row15 ts{i}"), &k32(i), STBDS_HM_BINARY);
        assert_eq!(t, -1, "row15: C must report -1 for an absent key");
    }
    m.free();
}

/// row 16: `temp == NULL`.
#[test]
fn err_16_get_ts_null_temp() {
    let _g = lock();
    let (c, r) = libs();

    // on a NULL map (the *temp write happens right after the allocation)
    let co = in_child(|| unsafe {
        let mut k = k32(1);
        (c.hmget_key_ts)(
            ptr::null_mut(),
            8,
            k.as_mut_ptr() as *mut c_void,
            4,
            ptr::null_mut(),
            STBDS_HM_BINARY,
        );
    });
    let ro = in_child(|| unsafe {
        let mut k = k32(1);
        (r.hmget_key_ts)(
            ptr::null_mut(),
            8,
            k.as_mut_ptr() as *mut c_void,
            4,
            ptr::null_mut(),
            STBDS_HM_BINARY,
        );
    });
    same("row16 outcome (NULL map)", co, ro);
    assert!(is_fatal(co), "row16: C did not fault, got {co:?}");

    // on a populated map
    let build = |l: &'static Lib| {
        move || unsafe {
            let mut h: *mut c_void = ptr::null_mut();
            for i in 0..10u32 {
                let mut k = k32(i);
                h = (l.hmput_key)(h, 8, k.as_mut_ptr() as *mut c_void, 4, STBDS_HM_BINARY);
            }
            let mut k = k32(3);
            (l.hmget_key_ts)(
                h,
                8,
                k.as_mut_ptr() as *mut c_void,
                4,
                ptr::null_mut(),
                STBDS_HM_BINARY,
            );
        }
    };
    let co = in_child(build(c));
    let ro = in_child(build(r));
    same("row16 outcome (populated)", co, ro);
    assert!(is_fatal(co), "row16: C did not fault, got {co:?}");
}

/// row 17: `hmget_key` mirrors the `temp` into the array header.
#[test]
fn err_17_get_key_writes_header_temp() {
    let _g = lock();
    // NULL map
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    assert_eq!(m.get("row17 on NULL", &k32(1), STBDS_HM_BINARY), -1);
    assert_eq!(m.snap_c().temp, -1, "row17: header temp must be -1");
    // no table
    m.put_default("row17 default");
    m.set_default_tail(9);
    assert_eq!(m.get("row17 no table", &k32(1), STBDS_HM_BINARY), -1);
    assert_eq!(m.snap_c().temp, -1);
    // populated, absent key
    for i in 0..20u32 {
        m.put(&format!("row17 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    assert_eq!(m.get("row17 absent", &k32(999), STBDS_HM_BINARY), -1);
    assert_eq!(m.snap_c().temp, -1);
    m.free();
}

// ===========================================================================
// rows 18, 28, 29, 34: the dead assertions
// ===========================================================================

/// rows 18, 28, 29, 34: assertions that the C code can never trip. A heavy
/// mixed workload is run in a child for each library; both must exit cleanly.
#[test]
fn err_18_28_29_34_dead_asserts_never_fire() {
    let _g = lock();
    let (c, r) = libs();

    let work = |l: &'static Lib| {
        move || unsafe {
            let mut rng = Rng::new(0x1234_5678);
            (l.rand_seed)(DEFAULT_SEED);
            // binary map churn (row 18, 28, 29)
            let mut h: *mut c_void = ptr::null_mut();
            let mut live: Vec<u32> = Vec::new();
            for _ in 0..20_000 {
                match rng.below(3) {
                    0 => {
                        let k = rng.next_u32() % 3000;
                        let mut kb = k32(k);
                        h = (l.hmput_key)(
                            h,
                            8,
                            kb.as_mut_ptr() as *mut c_void,
                            4,
                            STBDS_HM_BINARY,
                        );
                        live.push(k);
                    }
                    1 => {
                        let k = rng.next_u32() % 3000;
                        let mut kb = k32(k);
                        h = (l.hmget_key)(
                            h,
                            8,
                            kb.as_mut_ptr() as *mut c_void,
                            4,
                            STBDS_HM_BINARY,
                        );
                    }
                    _ => {
                        if !live.is_empty() {
                            let ix = rng.below(live.len());
                            let k = live.swap_remove(ix);
                            let mut kb = k32(k);
                            h = (l.hmdel_key)(
                                h,
                                8,
                                kb.as_mut_ptr() as *mut c_void,
                                4,
                                0,
                                STBDS_HM_BINARY,
                            );
                        }
                    }
                }
            }
            if !h.is_null() {
                (l.hmfree_func)((h as *mut u8).sub(8) as *mut c_void, 8);
            }
            // string arena churn (row 34)
            let mut arena = Arena::zeroed();
            arena.mode = 3;
            for _ in 0..4000 {
                let n = rng.range(0, 2500);
                let mut s: Vec<u8> = (0..n).map(|_| b'x').collect();
                s.push(0);
                (l.stralloc)(&mut arena, s.as_mut_ptr() as *mut _);
            }
            (l.strreset)(&mut arena);
        }
    };

    let co = in_child(work(c));
    let ro = in_child(work(r));
    same("rows18/28/29/34 outcome", co, ro);
    assert_eq!(
        co,
        Outcome::Exited(0),
        "rows18/28/29/34: C tripped an assertion"
    );
}

// ===========================================================================
// rows 19-24: out-of-range enum / mode values across the FFI boundary
// ===========================================================================

/// rows 19-20: out-of-range `mode` for `hmput_key` / `hmget_key` / `hmdel_key`.
#[test]
fn err_19_20_out_of_range_mode() {
    let _g = lock();
    let mut rng = Rng::new(0x19);

    // row 20: negative modes behave as binary
    for mode in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
        for i in 0..40u32 {
            m.put(&format!("row20 mode={mode} put{i}"), &k32(i), mode);
        }
        for i in 0..60u32 {
            m.get(&format!("row20 mode={mode} get{i}"), &k32(i), mode);
        }
        for i in 0..20u32 {
            m.del(&format!("row20 mode={mode} del{i}"), &k32(i), 0, mode);
        }
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            0,
            "row20: negative mode must leave string.mode == 0"
        );
        m.free();
    }

    // row 19: modes > 1 behave as string
    for mode in [2i32, 3, 127, 1000, i32::MAX, i32::MAX - 1] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(16, 8, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys = Vec::new();
        for i in 0..40 {
            let s = rng.ascii_cstring_range(1, 16);
            // duplicates must be excluded: a duplicate put is an *update*, so
            // the reverse-order delete below would no longer keep
            // `old_index == final_index`
            if keys.iter().any(|k: &Vec<u8>| k == &s) {
                continue;
            }
            keys.push(s.clone());
            m.put(&format!("row19 mode={mode} put{i}"), &s, mode);
        }
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            STBDS_SH_DEFAULT as u8,
            "row19: mode>1 must set string.mode = STBDS_SH_DEFAULT"
        );
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("row19 mode={mode} get{i}"), s, mode);
        }
        // deleting in reverse insertion order keeps `old_index == final_index`,
        // so the (always-failing) mode>1 re-find branch is never entered here;
        // that branch is ERRORS.md row 53, covered by err_53_del_mode_gt1.
        for (i, s) in keys.iter().enumerate().rev() {
            m.del(&format!("row19 mode={mode} del{i}"), s, 0, mode);
        }
        m.free();
    }
}

/// rows 21, 23: `key == NULL` faults in both hashing modes.
#[test]
fn err_21_23_null_key_faults() {
    let _g = lock();
    let (c, r) = libs();

    for (keysize, mode, label) in [(4usize, STBDS_HM_BINARY, "row21"), (8, STBDS_HM_STRING, "row23")]
    {
        let f = |l: &'static Lib| {
            move || unsafe {
                (l.hmput_key)(ptr::null_mut(), 16, ptr::null_mut(), keysize, mode);
            }
        };
        let co = in_child(f(c));
        let ro = in_child(f(r));
        same(&format!("{label} put outcome"), co, ro);
        assert!(is_fatal(co), "{label}: C did not fault, got {co:?}");

        // and on a populated map, where the lookup path hashes the key
        let g = |l: &'static Lib| {
            move || unsafe {
                let mut h: *mut c_void = ptr::null_mut();
                for i in 0..6u32 {
                    let mut kb = if mode == STBDS_HM_BINARY {
                        k32(i).to_vec()
                    } else {
                        let mut v = format!("k{i}").into_bytes();
                        v.push(0);
                        v
                    };
                    h = (l.hmput_key)(h, 16, kb.as_mut_ptr() as *mut c_void, keysize, mode);
                }
                (l.hmget_key)(h, 16, ptr::null_mut(), keysize, mode);
            }
        };
        let co = in_child(g(c));
        let ro = in_child(g(r));
        same(&format!("{label} get outcome"), co, ro);
        assert!(is_fatal(co), "{label}: C did not fault, got {co:?}");
    }
}

/// row 22: `key == NULL` with `keysize == 0` is well defined — no byte is read,
/// every key collides, and the map saturates at one entry.
#[test]
fn err_22_null_key_zero_keysize() {
    let _g = lock();
    let (c, r) = libs();
    reset_seeds(DEFAULT_SEED);
    unsafe {
        let elemsize = 8usize;
        let mut ch: *mut c_void = ptr::null_mut();
        let mut rh: *mut c_void = ptr::null_mut();
        for i in 0..25 {
            ch = (c.hmput_key)(ch, elemsize, ptr::null_mut(), 0, STBDS_HM_BINARY);
            rh = (r.hmput_key)(rh, elemsize, ptr::null_mut(), 0, STBDS_HM_BINARY);
            let ct = ptr::read_unaligned((ch as *const u8).sub(elemsize + 8) as *const isize);
            let rt = ptr::read_unaligned((rh as *const u8).sub(elemsize + 8) as *const isize);
            same(&format!("row22 put{i} temp"), ct, rt);
            write_tail_at(ch, elemsize, ct, 0, i as u8);
            write_tail_at(rh, elemsize, rt, 0, i as u8);
            same(
                &format!("row22 put{i} state"),
                snap_map(ch, elemsize, KeyKind::Binary, false),
                snap_map(rh, elemsize, KeyKind::Binary, false),
            );
        }
        let s = snap_map(ch, elemsize, KeyKind::Binary, false);
        assert_eq!(s.length, 2, "row22: C must saturate at a single entry");
        (c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// row 24: out-of-range `mode` for `stbds_shmode_func` truncates to
/// `mode & 0xff` and the `switch` in `hmput_key` falls through to `default:`
/// for every truncated value outside `{1,2,3}`.
#[test]
fn err_24_shmode_out_of_range() {
    let _g = lock();
    let mut rng = Rng::new(0x24);

    // (a) truncated values outside {1,2,3}: the `default:` switch arm does a
    // raw `memcpy` of `keysize` bytes, so binary keys are the right driver.
    for mode in [4i32, 5, 100, 254, 255, 256, -1, -2, i32::MIN, i32::MAX] {
        let low = (mode & 0xff) as u8;
        assert!(
            !matches!(low, 1 | 2 | 3),
            "test bug: mode {mode} truncates into the arena-mode range"
        );
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::from_shmode(16, 4, mode, KeyKind::Binary);
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            low,
            "row24: C truncation for mode {mode}"
        );
        for i in 0..40u32 {
            m.put(&format!("row24 mode={mode} put{i}"), &k32(i), STBDS_HM_BINARY);
        }
        for i in 0..60u32 {
            m.get(&format!("row24 mode={mode} get{i}"), &k32(i), STBDS_HM_BINARY);
        }
        for i in (0..40u32).rev() {
            m.del(&format!("row24 mode={mode} del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        }
        m.free();
    }

    // (b) truncated values that land *inside* {1,2,3}: the arena machinery is
    // selected, so the table must be driven with real C strings.
    for mode in [257i32, 258, 259, 513, -253, -254, -255] {
        let low = (mode & 0xff) as u8;
        assert!(
            matches!(low, 1 | 2 | 3),
            "test bug: mode {mode} does not truncate into the arena-mode range"
        );
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, mode, KeyKind::StringPtr { keyoffset: 0 });
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            low,
            "row24: C truncation for mode {mode}"
        );
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..40 {
            let s = rng.ascii_cstring_range(1, 20);
            if keys.iter().any(|k| k == &s) {
                continue;
            }
            keys.push(s.clone());
            m.put(&format!("row24 mode={mode} sput{i}"), &s, STBDS_HM_STRING);
        }
        for (i, s) in keys.iter().enumerate() {
            m.get(&format!("row24 mode={mode} sget{i}"), s, STBDS_HM_STRING);
        }
        for (i, s) in keys.iter().enumerate().rev() {
            m.del(&format!("row24 mode={mode} sdel{i}"), s, 0, STBDS_HM_STRING);
        }
        m.free();
    }
}

// ===========================================================================
// rows 25-27, 32, 33: hmdel_key rejections and rebuild boundaries
// ===========================================================================

/// row 25: `stbds_hmdel_key(NULL, ...)` returns NULL.
#[test]
fn err_25_del_on_null() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        for elemsize in [0usize, 8, 16] {
            for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, -1, 5] {
                let mut k = k32(1);
                let cp = (c.hmdel_key)(
                    ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let rp = (r.hmdel_key)(
                    ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                same("row25", cp.is_null(), rp.is_null());
                assert!(cp.is_null(), "row25: C must return NULL");
            }
        }
    }
}

/// row 26: delete against a handle with no hash table → `temp = 0`.
#[test]
fn err_26_del_no_table() {
    let _g = lock();
    for elemsize in [8usize, 16] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 4, KeyKind::Binary);
        m.put_default("row26 default");
        m.set_default_tail(0x21);
        let before = m.snap_c();
        for i in 0..8u32 {
            let t = m.del(&format!("row26 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
            assert_eq!(t, 0, "row26: C must report temp == 0");
        }
        let after = m.snap_c();
        assert_eq!(before.length, after.length, "row26: length must not change");
        assert!(!after.has_table);
        m.free();
    }
}

/// row 27: delete of an absent key → `temp = 0`, no bookkeeping change.
#[test]
fn err_27_del_absent() {
    let _g = lock();
    let mut rng = Rng::new(0x27);
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..60u32 {
        m.put(&format!("row27 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    let before = m.snap_c();
    for i in 0..300 {
        let k = 1000 + rng.next_u32() % 100_000;
        let t = m.del(&format!("row27 del{i}"), &k32(k), 0, STBDS_HM_BINARY);
        assert_eq!(t, 0, "row27: C must report temp == 0 for an absent key");
    }
    let after = m.snap_c();
    assert_eq!(before.length, after.length);
    let (bt, at) = (before.table.unwrap(), after.table.unwrap());
    assert_eq!(bt.used_count, at.used_count);
    assert_eq!(bt.tombstone_count, at.tombstone_count);
    assert_eq!(bt.slots, at.slots);
    m.free();
}

/// rows 30-31: the two `hmdel_key` re-find assertions. The last element's key
/// bytes are mutated behind the library's back (what a consumer that edits a
/// key in place does), so the re-find of the moved element either misses
/// entirely (row 30) or lands on a different entry (row 31).
#[test]
fn err_30_31_del_refind_asserts() {
    let _g = lock();
    let (c, r) = libs();

    /// `dup_of == None` → row 30 (`assert(slot >= 0)`);
    /// `dup_of == Some(k)` → row 31 (`assert(b->index[i] == final_index)`).
    fn scenario(l: &'static Lib, dup_of: Option<u32>) -> impl FnOnce() {
        move || unsafe {
            let elemsize = 8usize;
            let keysize = 4usize;
            (l.rand_seed)(DEFAULT_SEED);
            let mut h: *mut c_void = ptr::null_mut();
            for i in 0..7u32 {
                let mut kb = k32(i);
                h = (l.hmput_key)(h, elemsize, kb.as_mut_ptr() as *mut c_void, keysize, 0);
            }
            let raw = (h as *mut u8).sub(elemsize);
            let len = ptr::read_unaligned(raw.sub(HEADER) as *const usize);
            let final_index = len - 2;
            // overwrite the key bytes of the last entry
            let newkey: u32 = dup_of.unwrap_or(0xDEAD_BEEF);
            ptr::copy_nonoverlapping(
                newkey.to_ne_bytes().as_ptr(),
                (h as *mut u8).add(final_index * elemsize),
                keysize,
            );
            // delete entry 0, which is not the last -> forces the move+re-find
            let mut dk = k32(0);
            (l.hmdel_key)(
                h,
                elemsize,
                dk.as_mut_ptr() as *mut c_void,
                keysize,
                0,
                0,
            );
        }
    }

    let co = in_child(scenario(c, None));
    let ro = in_child(scenario(r, None));
    same("row30 outcome", co, ro);
    assert_eq!(
        co,
        Outcome::Signaled(SIGABRT),
        "row30: C must abort, got {co:?}"
    );

    let co = in_child(scenario(c, Some(1)));
    let ro = in_child(scenario(r, Some(1)));
    same("row31 outcome", co, ro);
    assert_eq!(
        co,
        Outcome::Signaled(SIGABRT),
        "row31: C must abort, got {co:?}"
    );
}

/// row 32: the shrink boundary, including the `slot_count == 8` no-shrink rule.
#[test]
fn err_32_shrink_boundary() {
    let _g = lock();
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    // 12 entries -> slot_count 16 (grew at used_count >= 6, then >= 12 -> 32)
    for i in 0..12u32 {
        m.put(&format!("row32 put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    let mut seen_shrink = false;
    let mut prev = m.snap_c().table.unwrap().slot_count;
    for i in 0..12u32 {
        m.del(&format!("row32 del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        let sc = m.snap_c().table.unwrap().slot_count;
        if sc < prev {
            seen_shrink = true;
        }
        assert!(sc >= 8);
        prev = sc;
    }
    assert!(seen_shrink, "row32: C never took the shrink path");
    assert_eq!(prev, 8);
    m.free();

    // slot_count == 8: used_count_shrink_threshold is forced to 0, so no shrink
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    for i in 0..5u32 {
        m.put(&format!("row32b put{i}"), &k32(i), STBDS_HM_BINARY);
    }
    assert_eq!(m.snap_c().table.unwrap().slot_count, 8);
    assert_eq!(m.snap_c().table.unwrap().used_count_shrink_threshold, 0);
    for i in 0..5u32 {
        m.del(&format!("row32b del{i}"), &k32(i), 0, STBDS_HM_BINARY);
        assert_eq!(m.snap_c().table.unwrap().slot_count, 8);
    }
    m.free();
}

/// row 33: the tombstone-rebuild boundary.
#[test]
fn err_33_tombstone_rebuild() {
    let _g = lock();
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
    // keep used_count high enough that the shrink branch is not taken, then
    // churn keys so tombstone_count crosses its threshold
    for i in 0..40u32 {
        m.put(&format!("row33 base{i}"), &k32(i), STBDS_HM_BINARY);
    }
    let mut rebuilt = false;
    for round in 0..200u32 {
        let k = 10_000 + round;
        m.put(&format!("row33 put{round}"), &k32(k), STBDS_HM_BINARY);
        let before = m.snap_c().table.unwrap().tombstone_count;
        m.del(&format!("row33 del{round}"), &k32(k), 0, STBDS_HM_BINARY);
        let after = m.snap_c().table.unwrap().tombstone_count;
        if after < before + 1 {
            rebuilt = true;
        }
    }
    assert!(rebuilt, "row33: C never took the tombstone-rebuild path");
    m.free();
}

// ===========================================================================
// rows 35-41: string arena
// ===========================================================================

/// row 35: arena with `remaining >= len` but `storage == NULL`.
#[test]
fn err_35_stralloc_null_storage_with_remaining() {
    let _g = lock();
    let (c, r) = libs();
    let f = |l: &'static Lib| {
        move || unsafe {
            let mut a = Arena::zeroed();
            a.remaining = 1000;
            let mut s = b"hello\0".to_vec();
            (l.stralloc)(&mut a, s.as_mut_ptr() as *mut _);
        }
    };
    let co = in_child(f(c));
    let ro = in_child(f(r));
    same("row35 outcome", co, ro);
    assert!(is_fatal(co), "row35: C did not fault, got {co:?}");
}

/// row 36: `stbds_stralloc(NULL, str)`.
#[test]
fn err_36_stralloc_null_arena() {
    let _g = lock();
    let (c, r) = libs();
    let f = |l: &'static Lib| {
        move || unsafe {
            let mut s = b"hello\0".to_vec();
            (l.stralloc)(ptr::null_mut(), s.as_mut_ptr() as *mut _);
        }
    };
    let co = in_child(f(c));
    let ro = in_child(f(r));
    same("row36 outcome", co, ro);
    assert!(is_fatal(co), "row36: C did not fault, got {co:?}");
}

/// row 37: `stbds_stralloc(&arena, NULL)`.
#[test]
fn err_37_stralloc_null_str() {
    let _g = lock();
    let (c, r) = libs();
    let f = |l: &'static Lib| {
        move || unsafe {
            let mut a = Arena::zeroed();
            (l.stralloc)(&mut a, ptr::null_mut());
        }
    };
    let co = in_child(f(c));
    let ro = in_child(f(r));
    same("row37 outcome", co, ro);
    assert!(is_fatal(co), "row37: C did not fault, got {co:?}");
}

/// row 38: `block` saturates at 22 and `blocksize` caps at `1 << 20`.
#[test]
fn err_38_arena_block_saturation() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for step in 0..30 {
            let blocksize = 512usize << (ca.block >> 1);
            // len == blocksize forces a fresh block every time
            let mut s: Vec<u8> = vec![b'z'; blocksize - 1];
            s.push(0);
            let mut s2 = s.clone();
            let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut _);
            let rp = (r.stralloc)(&mut ra, s2.as_mut_ptr() as *mut _);
            same(&format!("row38 step{step} block"), ca.block, ra.block);
            same(
                &format!("row38 step{step} remaining"),
                ca.remaining,
                ra.remaining,
            );
            let cs = std::slice::from_raw_parts(cp as *const u8, blocksize - 1).to_vec();
            let rs = std::slice::from_raw_parts(rp as *const u8, blocksize - 1).to_vec();
            same(&format!("row38 step{step} content"), cs, rs);
            assert!(ca.block <= 22, "row38: block exceeded 22 in C");
            assert!(blocksize <= (1 << 20), "row38: blocksize exceeded 1 MiB");
        }
        assert_eq!(ca.block, 22, "row38: C block did not saturate at 22");
        same("row38 final block", ca.block, ra.block);
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
}

/// row 39: the oversize-block branch, including as the first allocation.
#[test]
fn err_39_stralloc_oversize_block() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        for (n, preload) in [(513usize, false), (2000, false), (2000, true), (100_000, true)] {
            let mut ca = Arena::zeroed();
            let mut ra = Arena::zeroed();
            if preload {
                let mut s = b"seed\0".to_vec();
                let mut s2 = s.clone();
                (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut _);
                (r.stralloc)(&mut ra, s2.as_mut_ptr() as *mut _);
                same("row39 preload block", ca.block, ra.block);
                same("row39 preload remaining", ca.remaining, ra.remaining);
            }
            let mut s: Vec<u8> = vec![b'q'; n];
            s.push(0);
            let mut s2 = s.clone();
            let cp = (c.stralloc)(&mut ca, s.as_mut_ptr() as *mut _);
            let rp = (r.stralloc)(&mut ra, s2.as_mut_ptr() as *mut _);
            same(&format!("row39 n={n} preload={preload} block"), ca.block, ra.block);
            same(
                &format!("row39 n={n} preload={preload} remaining"),
                ca.remaining,
                ra.remaining,
            );
            let cs = std::slice::from_raw_parts(cp as *const u8, n).to_vec();
            let rs = std::slice::from_raw_parts(rp as *const u8, n).to_vec();
            same(&format!("row39 n={n} preload={preload} content"), cs, rs);
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            same("row39 after reset", ca.remaining, ra.remaining);
        }
    }
}

/// row 40: `stbds_strreset(NULL)`.
#[test]
fn err_40_strreset_null() {
    let _g = lock();
    let (c, r) = libs();
    let co = in_child(|| unsafe { (c.strreset)(ptr::null_mut()) });
    let ro = in_child(|| unsafe { (r.strreset)(ptr::null_mut()) });
    same("row40 outcome", co, ro);
    assert!(is_fatal(co), "row40: C did not fault, got {co:?}");
}

/// row 41: resetting an already-zeroed arena is a well-defined no-op.
#[test]
fn err_41_strreset_zeroed() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        let mut ca = Arena::zeroed();
        let mut ra = Arena::zeroed();
        for _ in 0..5 {
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            same(
                "row41",
                (ca.remaining, ca.block, ca.mode, ca.storage.is_null()),
                (ra.remaining, ra.block, ra.mode, ra.storage.is_null()),
            );
            assert!(ca.storage.is_null() && ca.remaining == 0 && ca.block == 0 && ca.mode == 0);
        }
        // a non-zero mode is also cleared by strreset
        ca.mode = 3;
        ra.mode = 3;
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        same("row41 mode cleared", ca.mode, ra.mode);
        assert_eq!(ca.mode, 0);
    }
}

// ===========================================================================
// rows 42-48: the hm_geti assertions
// ===========================================================================

/// rows 42-48: every `STBDS_ASSERT` inside `hm_geti` must hold in both
/// libraries. `hm_geti` is run in a forked child so the termination status can
/// be compared; a behavioural divergence shows up as `SIGABRT` on one side.
#[test]
fn err_42_48_hm_geti_assertions() {
    let _g = lock();
    let (c, r) = libs();
    let mut nums: Vec<i32> = vec![i32::MIN, -1000, -1, 0]; // row 48
    nums.extend(1..=24); // rows 42-47 at every small boundary
    nums.extend([31, 32, 33, 64, 100, 127, 128, 200, 501]);

    for num in nums {
        for seed in [DEFAULT_SEED, 0usize, usize::MAX] {
            let co = in_child(|| unsafe {
                (c.rand_seed)(seed);
                (c.hm_geti)(num);
            });
            let ro = in_child(|| unsafe {
                (r.rand_seed)(seed);
                (r.hm_geti)(num);
            });
            same(&format!("rows42-48 num={num} seed={seed:#x}"), co, ro);
            assert_eq!(
                co,
                Outcome::Exited(0),
                "rows42-48: C aborted for num={num} seed={seed:#x}"
            );
        }
    }
}

// ===========================================================================
// rows 49-52: generic FFI boundaries
// ===========================================================================

/// row 49: `elemsize == 0`.
#[test]
fn err_49_zero_elemsize() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        // arrgrowf
        for addlen in [0usize, 1, 8] {
            for min_cap in [0usize, 1, 4, 100] {
                let ca = (c.arrgrowf)(ptr::null_mut(), 0, addlen, min_cap);
                let ra = (r.arrgrowf)(ptr::null_mut(), 0, addlen, min_cap);
                same("row49 arrgrowf null-ness", ca.is_null(), ra.is_null());
                if !ca.is_null() {
                    same("row49 arrgrowf header", hdr_of(ca), hdr_of(ra));
                    (c.arrfreef)(ca);
                    (r.arrfreef)(ra);
                }
            }
        }
    }
    // hmput_key / hmget_key / hmdel_key with elemsize == 0 and keysize == 0
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(0, 0, KeyKind::Binary);
    for i in 0..20 {
        m.put(&format!("row49 put{i}"), &[], STBDS_HM_BINARY);
        m.get(&format!("row49 get{i}"), &[], STBDS_HM_BINARY);
    }
    for i in 0..4 {
        m.del(&format!("row49 del{i}"), &[], 0, STBDS_HM_BINARY);
    }
    m.free();
}

/// row 50: `keysize == 0` in binary mode — every key collides.
#[test]
fn err_50_zero_keysize() {
    let _g = lock();
    let mut rng = Rng::new(0x50);
    for elemsize in [8usize, 16] {
        reset_seeds(DEFAULT_SEED);
        let mut m = MapPair::new_null(elemsize, 0, KeyKind::Binary);
        for i in 0..30 {
            let k = rng.bytes(8);
            m.put(&format!("row50 e={elemsize} put{i}"), &k, STBDS_HM_BINARY);
            let t = m.get(&format!("row50 e={elemsize} get{i}"), &k, STBDS_HM_BINARY);
            assert_eq!(t, 0, "row50: every key must map onto entry 0 in C");
        }
        assert_eq!(m.snap_c().length, 2, "row50: only one entry may exist");
        m.del("row50 del", &rng.bytes(8), 0, STBDS_HM_BINARY);
        assert_eq!(m.snap_c().length, 1);
        m.free();
    }
}

/// row 51: `keysize` covering the whole element (larger than the logical key).
#[test]
fn err_51_keysize_equals_elemsize() {
    let _g = lock();
    let mut rng = Rng::new(0x51);
    reset_seeds(DEFAULT_SEED);
    let mut m = MapPair::new_null(16, 16, KeyKind::Binary);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..200 {
        // logical key in the first 4 bytes, "payload" in the rest — but the
        // library is told keysize == 16, so the payload is part of the key
        let mut k = rng.bytes(16);
        k[0..4].copy_from_slice(&k32((i % 50) as u32));
        keys.push(k.clone());
        m.put(&format!("row51 put{i}"), &k, STBDS_HM_BINARY);
    }
    for (i, k) in keys.iter().enumerate() {
        let t = m.get(&format!("row51 get{i}"), k, STBDS_HM_BINARY);
        assert!(t >= 0, "row51: full-element key must be found by C");
        // the same logical prefix with a different payload must miss
        let mut alt = k.clone();
        alt[15] ^= 0xFF;
        m.get(&format!("row51 alt{i}"), &alt, STBDS_HM_BINARY);
    }
    for (i, k) in keys.iter().enumerate().take(50) {
        m.del(&format!("row51 del{i}"), k, 0, STBDS_HM_BINARY);
    }
    m.free();
}

/// row 52: boundary seeds are not special-cased anywhere.
#[test]
fn err_52_boundary_seeds() {
    let _g = lock();
    let (c, r) = libs();
    let mut rng = Rng::new(0x52);
    unsafe {
        for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, DEFAULT_SEED] {
            for len in 0..24usize {
                let mut b = rng.bytes(len.max(1));
                same(
                    &format!("row52 hash_bytes len={len} seed={seed:#x}"),
                    (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed),
                    (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed),
                );
                let mut s = rng.cstring(len);
                same(
                    &format!("row52 hash_string len={len} seed={seed:#x}"),
                    (c.hash_string)(s.as_mut_ptr() as *mut _, seed),
                    (r.hash_string)(s.as_mut_ptr() as *mut _, seed),
                );
            }
            // and through a whole map lifecycle
            reset_seeds(seed);
            let mut m = MapPair::new_null(8, 4, KeyKind::Binary);
            for i in 0..60u32 {
                m.put(&format!("row52 seed={seed:#x} put{i}"), &k32(i), STBDS_HM_BINARY);
            }
            for i in 0..80u32 {
                m.get(&format!("row52 seed={seed:#x} get{i}"), &k32(i), STBDS_HM_BINARY);
            }
            for i in 0..60u32 {
                m.del(&format!("row52 seed={seed:#x} del{i}"), &k32(i), 0, STBDS_HM_BINARY);
            }
            m.free();
        }
    }
}

/// row 53: `hmdel_key` with `mode > STBDS_HM_STRING` on a string-keyed table.
/// `mode == STBDS_HM_STRING` is false, so the re-find of the moved entry passes
/// the element's *address* while `stbds_hm_find_slot` hashes it as a C string —
/// the lookup always misses and `assert(slot >= 0)` fires.
#[test]
fn err_53_del_mode_gt1_refind_aborts() {
    let _g = lock();
    let (c, r) = libs();

    fn scenario(l: &'static Lib, sh_mode: i32, mode: i32) -> impl FnOnce() {
        move || unsafe {
            let elemsize = 16usize;
            (l.rand_seed)(DEFAULT_SEED);
            let mut h = (l.shmode_func)(elemsize, sh_mode);
            // the key buffers must outlive the map: in STBDS_SH_DEFAULT mode the
            // library stores the caller's pointer, and a dangling key makes the
            // delete miss instead of reaching the re-find.
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..8u32 {
                let mut s = format!("key_{i}").into_bytes();
                s.push(0);
                keys.push(s);
            }
            for i in 0..8usize {
                let p = keys[i].as_mut_ptr() as *mut c_void;
                h = (l.hmput_key)(h, elemsize, p, 8, STBDS_HM_STRING);
            }
            // delete the FIRST entry: old_index (0) != final_index -> re-find
            let p = keys[0].as_mut_ptr() as *mut c_void;
            (l.hmdel_key)(h, elemsize, p, 8, 0, mode);
            std::hint::black_box(&keys);
        }
    }

    for sh_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for mode in [2i32, 3, 127, i32::MAX] {
            let co = in_child(scenario(c, sh_mode, mode));
            let ro = in_child(scenario(r, sh_mode, mode));
            same(&format!("row53 sh={sh_mode} mode={mode} outcome"), co, ro);
            assert_eq!(
                co,
                Outcome::Signaled(SIGABRT),
                "row53: C must abort for sh={sh_mode} mode={mode}, got {co:?}"
            );
        }
    }

    // The mirror case is well defined: deleting the LAST entry never re-finds.
    for sh_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        reset_seeds(DEFAULT_SEED);
        let mut m =
            MapPair::from_shmode(16, 8, sh_mode, KeyKind::StringPtr { keyoffset: 0 });
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..8u32 {
            let mut s = format!("key_{i}").into_bytes();
            s.push(0);
            keys.push(s.clone());
            m.put(&format!("row53 sh={sh_mode} put{i}"), &s, STBDS_HM_STRING);
        }
        for (i, s) in keys.iter().enumerate().rev() {
            let t = m.del(&format!("row53 sh={sh_mode} tail-del{i}"), s, 0, 2);
            assert_eq!(t, 1, "row53: deleting the last entry must succeed");
        }
        m.free();
    }
}
