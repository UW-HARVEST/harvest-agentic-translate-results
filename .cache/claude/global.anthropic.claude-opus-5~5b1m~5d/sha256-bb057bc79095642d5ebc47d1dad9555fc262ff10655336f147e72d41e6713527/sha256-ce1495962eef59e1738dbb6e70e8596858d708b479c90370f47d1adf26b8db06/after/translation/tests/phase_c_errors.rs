//! Phase C -- one differential test per row of `ERRORS.md` (E1..E50).
//!
//! Rows where the C legitimately dies (a failing `STBDS_ASSERT` -> SIGABRT with
//! a glibc diagnostic, or a wild-pointer dereference -> SIGSEGV) are run in
//! forked children so the *signal and the stderr text* can be compared.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const E: usize = 16;
const K: usize = 8;

// ===========================================================================
// stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

/// E1 -- `min_cap <= stbds_arrcap(a)` returns `a` untouched.
#[test]
fn err_e1_arrgrowf_no_growth_needed() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let b = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let (ha, hb) = (snap_arr(a, 0), snap_arr(b, 0));
            assert_same("e1 setup", &ha, &hb);
            assert_eq!(ha.capacity, 10);
            // every request that is already satisfied must be a pure no-op
            for min_cap in 0..=10usize {
                let na = (c.arrgrowf)(a, elemsize, 0, min_cap);
                let nb = (rs.arrgrowf)(b, elemsize, 0, min_cap);
                assert_eq!(na, a, "C must return the same pointer");
                assert_eq!(nb, b, "RUST must return the same pointer");
                assert_same("e1 header untouched", &snap_arr(na, 0), &snap_arr(nb, 0));
                assert_eq!(snap_arr(na, 0).capacity, 10, "capacity must not change");
            }
            // addlen still counts towards min_len
            for addlen in 0..=10usize {
                let na = (c.arrgrowf)(a, elemsize, addlen, 0);
                let nb = (rs.arrgrowf)(b, elemsize, addlen, 0);
                assert_eq!(na, a);
                assert_eq!(nb, b);
                assert_same("e1 addlen<=cap", &snap_arr(na, 0), &snap_arr(nb, 0));
            }
            (c.arrfreef)(a);
            (rs.arrfreef)(b);
        }
    });
}

/// E2 -- zero request on a NULL array returns NULL (no allocation at all).
#[test]
fn err_e2_arrgrowf_zero_request_null() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [0usize, 1, 8, 16, 40, usize::MAX] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let b = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert_same("e2 both NULL", &a.is_null(), &b.is_null());
            assert!(a.is_null(), "C must return NULL for a zero request");
            assert!(b.is_null(), "RUST must return NULL for a zero request");
        }
    });
}

/// E3 -- `min_cap` in 1..3 is clamped up to 4.
#[test]
fn err_e3_arrgrowf_min_cap_clamped_to_4() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [0usize, 1, 3, 8, 16] {
            for min_cap in 1..=3usize {
                for addlen in 0..=3usize {
                    let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let b = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_same("e3", &snap_arr(a, 0), &snap_arr(b, 0));
                    assert_eq!(snap_arr(a, 0).capacity, 4);
                    assert_eq!(snap_arr(a, 0).length, 0);
                    (c.arrfreef)(a);
                    (rs.arrfreef)(b);
                }
            }
        }
    });
}

/// E4 -- an allocation size that `realloc` cannot satisfy: the C ignores the
/// NULL and writes the header at `NULL + sizeof(header)` => SIGSEGV.
#[test]
fn err_e4_arrgrowf_size_overflow() {
    with_libs(0x31415926, |c, rs| {
        // Note: `elemsize * min_cap + 32` is computed in wrapping `size_t`
        // arithmetic, so some huge pairs wrap round to a *small* size that
        // `realloc` happily satisfies (e.g. `usize::MAX/4 * 8 + 32 == 24`).
        // What must hold is that C and Rust do the identical thing either way.
        let mut faulted = 0usize;
        for (elemsize, min_cap) in [
            (1usize, usize::MAX / 2),
            (1, usize::MAX / 4),
            (1, usize::MAX),
            (8, usize::MAX / 16),
            (usize::MAX / 4, 8),
            (usize::MAX, 1),
            (1 << 40, 1 << 20),
            (1 << 30, 1 << 30),
        ] {
            let out = assert_same_crash(
                &format!("e4 arrgrowf(NULL,{elemsize},0,{min_cap})"),
                || unsafe {
                    (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                },
                || unsafe {
                    (rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                },
            );
            if !out.exited {
                assert!(
                    out.code == SIGSEGV || out.code == SIGBUS,
                    "unexpected termination for ({elemsize},{min_cap}): {out:?}"
                );
                faulted += 1;
            }
        }
        assert!(
            faulted >= 3,
            "expected the realloc-failure path to fault, only {faulted} did"
        );
    });
}

/// E44 -- `stbds_arrfreef(NULL)` frees `(char *) NULL - 32`.
#[test]
fn err_e44_arrfreef_null() {
    with_libs(0x31415926, |c, rs| {
        let out = assert_same_crash(
            "e44 arrfreef(NULL)",
            || unsafe { (c.arrfreef)(std::ptr::null_mut()) },
            || unsafe { (rs.arrfreef)(std::ptr::null_mut()) },
        );
        assert!(!out.exited, "expected free() of a bogus pointer to die: {out:?}");
    });
}

// ===========================================================================
// stbds_hmfree_func
// ===========================================================================

/// E5 -- `a == NULL` returns immediately.
#[test]
fn err_e5_hmfree_null() {
    with_libs(0x31415926, |c, rs| {
        for elemsize in [0usize, 1, 8, 16, 40, usize::MAX] {
            let out = assert_same_crash(
                &format!("e5 hmfree_func(NULL,{elemsize})"),
                || unsafe { (c.hmfree_func)(std::ptr::null_mut(), elemsize) },
                || unsafe { (rs.hmfree_func)(std::ptr::null_mut(), elemsize) },
            );
            assert!(out.exited && out.code == 0, "must be a clean no-op: {out:?}");
        }
        // also in-process: must not crash the test runner
        unsafe {
            (c.hmfree_func)(std::ptr::null_mut(), 16);
            (rs.hmfree_func)(std::ptr::null_mut(), 16);
        }
    });
}

/// E6 -- an array that never went through `hmput_key` has `hash_table == NULL`.
#[test]
fn err_e6_hmfree_no_hash_table() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let b = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            (*header_of(a)).length = 3;
            (*header_of(b)).length = 3;
            assert!((*header_of(a)).hash_table.is_null());
            assert!((*header_of(b)).hash_table.is_null());
            let out = assert_same_crash(
                &format!("e6 hmfree_func(no table, {elemsize})"),
                || (c.hmfree_func)(a, elemsize),
                || (rs.hmfree_func)(b, elemsize),
            );
            assert!(out.exited && out.code == 0, "{out:?}");
            (c.hmfree_func)(a, elemsize);
            (rs.hmfree_func)(b, elemsize);
        }
    });
}

// ===========================================================================
// stbds_hmget_key(_ts)  (E7..E11, E43)
// ===========================================================================

/// E8 / E43 -- `hmget_key_ts(NULL, ...)` allocates a default element and
/// reports `STBDS_INDEX_EMPTY`.
#[test]
fn err_e8_hmget_key_ts_null_map() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(8);
        for elemsize in [1usize, 8, 16, 40] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1, c_int::MAX, c_int::MIN] {
                let key = rng.cstring(6);
                let mut tc: isize = 0x1234;
                let mut tr: isize = 0x1234;
                let a = (c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    K,
                    &mut tc,
                    mode,
                );
                let b = (rs.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    K,
                    &mut tr,
                    mode,
                );
                assert_same("e8 *temp", &tc, &tr);
                assert_eq!(tc, -1, "STBDS_INDEX_EMPTY expected");
                assert_same("e8 nullness", &a.is_null(), &b.is_null());
                assert!(!a.is_null(), "a fresh default map must be allocated");
                assert_same(
                    "e8 fresh map state",
                    &snap_map(a, elemsize, KeyKind::Binary),
                    &snap_map(b, elemsize, KeyKind::Binary),
                );
                assert_eq!(snap_map(a, elemsize, KeyKind::Binary).length, 1);
                (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
                (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
            }
        }
    });
}

#[test]
fn err_e43_hmget_key_null_writes_temp() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(43);
        for elemsize in [1usize, 8, 16, 40] {
            let key = rng.cstring(5);
            let a = (c.hmget_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                K,
                HM_BINARY,
            );
            let b = (rs.hmget_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                K,
                HM_BINARY,
            );
            assert_same(
                "e43 header temp",
                &snap_map(a, elemsize, KeyKind::Binary),
                &snap_map(b, elemsize, KeyKind::Binary),
            );
            assert_eq!(map_temp(a, elemsize), -1, "header temp must be -1");
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

/// E9 -- non-NULL map whose `hash_table` is still NULL (`hmput_default` only).
#[test]
fn err_e9_hmget_key_ts_no_table() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(9);
        for elemsize in [1usize, 8, 16, 40] {
            let a = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let b = (rs.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!((*header_of(hash_to_arr(a, elemsize))).hash_table.is_null());
            for _ in 0..50 {
                let key = rng.cstring(4);
                let mut tc: isize = 77;
                let mut tr: isize = 77;
                let na = (c.hmget_key_ts)(a, elemsize, key.as_ptr() as *mut c_void, K, &mut tc, HM_BINARY);
                let nb = (rs.hmget_key_ts)(b, elemsize, key.as_ptr() as *mut c_void, K, &mut tr, HM_BINARY);
                assert_same("e9 *temp", &tc, &tr);
                assert_eq!(tc, -1);
                assert_eq!(na, a, "must return the map unchanged");
                assert_eq!(nb, b);
                assert_same(
                    "e9 state",
                    &snap_map(na, elemsize, KeyKind::Binary),
                    &snap_map(nb, elemsize, KeyKind::Binary),
                );
            }
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

/// E7 / E10 / E11 -- absent key in a populated table => `-1`.
#[test]
fn err_e10_hmget_missing_key() {
    let mut rng = Rng::new(10);
    with_libs(0x31415926, |c, rs| {
        for &mode in &[HM_BINARY, HM_STRING] {
            let arena = if mode == HM_STRING {
                Arena::Explicit(SH_STRDUP)
            } else {
                Arena::Auto
            };
            for n in [1usize, 5, 6, 7, 13, 50, 200] {
                let (present, absent) = Keys::random_disjoint(&mut rng, n, 40, 20);
                let mut m = MapPair::new(c, rs, E, K, mode, arena);
                for i in 0..n {
                    m.put(present.ptr(i), i as u64);
                }
                for i in 0..absent.len() {
                    let t = m.get_ts(absent.ptr(i));
                    assert_eq!(t, -1, "absent key must report -1 (mode={mode}, n={n})");
                    let h = m.get(absent.ptr(i));
                    assert_eq!(h, -1, "header temp must be -1 too");
                }
                m.check(&format!("e10 mode={mode} n={n}"));
                m.free();
            }
        }
    });
}

#[test]
fn err_e11_hmget_key_wrapper_missing() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(11);
        // (a) NULL map
        let key = rng.cstring(7);
        let a = (c.hmget_key)(std::ptr::null_mut(), E, key.as_ptr() as *mut c_void, K, HM_BINARY);
        let b = (rs.hmget_key)(std::ptr::null_mut(), E, key.as_ptr() as *mut c_void, K, HM_BINARY);
        assert_eq!(map_temp(a, E), -1);
        assert_eq!(map_temp(b, E), -1);
        assert_same("e11a", &snap_map(a, E, KeyKind::Binary), &snap_map(b, E, KeyKind::Binary));
        (c.hmfree_func)(hash_to_arr(a, E), E);
        (rs.hmfree_func)(hash_to_arr(b, E), E);
        // (b) no hash table
        let a = (c.hmput_default)(std::ptr::null_mut(), E);
        let b = (rs.hmput_default)(std::ptr::null_mut(), E);
        let a = (c.hmget_key)(a, E, key.as_ptr() as *mut c_void, K, HM_BINARY);
        let b = (rs.hmget_key)(b, E, key.as_ptr() as *mut c_void, K, HM_BINARY);
        assert_eq!(map_temp(a, E), -1);
        assert_eq!(map_temp(b, E), -1);
        assert_same("e11b", &snap_map(a, E, KeyKind::Binary), &snap_map(b, E, KeyKind::Binary));
        (c.hmfree_func)(hash_to_arr(a, E), E);
        (rs.hmfree_func)(hash_to_arr(b, E), E);
    });
}

// ===========================================================================
// stbds_hmdel_key  (E12..E15)
// ===========================================================================

/// E12 -- `hmdel_key(NULL, ...)` returns NULL (the only NULL-returning export).
#[test]
fn err_e12_hmdel_null_map() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(12);
        for elemsize in [0usize, 1, 8, 16, 40, usize::MAX] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1, c_int::MAX, c_int::MIN] {
                for keyoffset in [0usize, 8, usize::MAX] {
                    let key = rng.cstring(5);
                    let a = (c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_ptr() as *mut c_void,
                        K,
                        keyoffset,
                        mode,
                    );
                    let b = (rs.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_ptr() as *mut c_void,
                        K,
                        keyoffset,
                        mode,
                    );
                    assert_same("e12 nullness", &a.is_null(), &b.is_null());
                    assert!(a.is_null(), "C must return NULL");
                    assert!(b.is_null(), "RUST must return NULL");
                }
            }
        }
    });
}

/// E13 -- non-NULL map, `hash_table == NULL`: `temp = 0`, map returned as-is.
#[test]
fn err_e13_hmdel_no_table() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(13);
        for elemsize in [1usize, 8, 16, 40] {
            let a = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let b = (rs.hmput_default)(std::ptr::null_mut(), elemsize);
            // pre-set temp to something else so the write is observable
            (*header_of(hash_to_arr(a, elemsize))).temp = 999;
            (*header_of(hash_to_arr(b, elemsize))).temp = 999;
            let key = rng.cstring(5);
            let na = (c.hmdel_key)(a, elemsize, key.as_ptr() as *mut c_void, K, 0, HM_BINARY);
            let nb = (rs.hmdel_key)(b, elemsize, key.as_ptr() as *mut c_void, K, 0, HM_BINARY);
            assert_eq!(na, a);
            assert_eq!(nb, b);
            assert_eq!(map_temp(na, elemsize), 0, "temp must be reset to 0");
            assert_same(
                "e13 state",
                &snap_map(na, elemsize, KeyKind::Binary),
                &snap_map(nb, elemsize, KeyKind::Binary),
            );
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

/// E14 -- absent key: `temp = 0`, nothing else changes.
#[test]
fn err_e14_hmdel_missing_key() {
    let mut rng = Rng::new(14);
    with_libs(0x31415926, |c, rs| {
        for &mode in &[HM_BINARY, HM_STRING] {
            let arena = if mode == HM_STRING {
                Arena::Explicit(SH_ARENA)
            } else {
                Arena::Auto
            };
            for n in [1usize, 6, 7, 40] {
                let (present, absent) = Keys::random_disjoint(&mut rng, n, 30, 18);
                let mut m = MapPair::new(c, rs, E, K, mode, arena);
                for i in 0..n {
                    m.put(present.ptr(i), i as u64);
                }
                let before = m.snap_c();
                for i in 0..absent.len() {
                    let t = m.del(absent.ptr(i), 0);
                    assert_eq!(t, 0, "missing delete must leave temp == 0");
                    m.check(&format!("e14 mode={mode} n={n} #{i}"));
                }
                let after = m.snap_c();
                assert_eq!(before.length, after.length, "length must not change");
                assert_eq!(
                    before.table.as_ref().map(|t| t.used_count),
                    after.table.as_ref().map(|t| t.used_count)
                );
                assert_eq!(
                    before.table.as_ref().map(|t| t.tombstone_count),
                    after.table.as_ref().map(|t| t.tombstone_count)
                );
                m.free();
            }
        }
    });
}

/// E15 -- present key: `temp = 1`, counters move.
#[test]
fn err_e15_hmdel_present_sets_temp1() {
    let mut rng = Rng::new(15);
    with_libs(0x31415926, |c, rs| {
        for &mode in &[HM_BINARY, HM_STRING] {
            let arena = if mode == HM_STRING {
                Arena::Explicit(SH_STRDUP)
            } else {
                Arena::Auto
            };
            for n in [1usize, 2, 6, 7, 40] {
                let keys = Keys::random(&mut rng, n, 18);
                let mut m = MapPair::new(c, rs, E, K, mode, arena);
                for i in 0..n {
                    m.put(keys.ptr(i), i as u64);
                }
                for i in 0..n {
                    let before = m.snap_c();
                    let t = m.del(keys.ptr(i), 0);
                    assert_eq!(t, 1, "hit delete must set temp == 1");
                    let after = m.snap_c();
                    assert_eq!(after.length, before.length - 1);
                    assert_eq!(
                        after.table.as_ref().unwrap().used_count,
                        before.table.as_ref().unwrap().used_count - 1
                    );
                    m.check(&format!("e15 mode={mode} n={n} #{i}"));
                    // deleting again must now miss
                    assert_eq!(m.del(keys.ptr(i), 0), 0);
                }
                m.free();
            }
        }
    });
}

// ===========================================================================
// stbds_hmput_default  (E16..E18)
// ===========================================================================

#[test]
fn err_e16_hmput_default_null() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [0usize, 1, 8, 16, 40] {
            let a = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let b = (rs.hmput_default)(std::ptr::null_mut(), elemsize);
            assert_same(
                "e16",
                &snap_map(a, elemsize, KeyKind::Binary),
                &snap_map(b, elemsize, KeyKind::Binary),
            );
            let s = snap_map(a, elemsize, KeyKind::Binary);
            assert_eq!(s.length, 1);
            assert_eq!(s.capacity, 4);
            assert!(!s.has_table);
            if elemsize > 0 {
                assert!(s.elems[0].iter().all(|&x| x == 0), "elem 0 must be zeroed");
            }
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

#[test]
fn err_e17_hmput_default_zero_len() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            // an array grown but still empty, presented as a "hash pointer"
            let ra = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            let rb = (rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            // poison the element so the memset is observable
            std::ptr::write_bytes(ra as *mut u8, 0xEE, elemsize * 4);
            std::ptr::write_bytes(rb as *mut u8, 0xEE, elemsize * 4);
            assert_eq!((*header_of(ra)).length, 0);
            let a = (c.hmput_default)(arr_to_hash(ra, elemsize), elemsize);
            let b = (rs.hmput_default)(arr_to_hash(rb, elemsize), elemsize);
            assert_same(
                "e17",
                &snap_map(a, elemsize, KeyKind::Binary),
                &snap_map(b, elemsize, KeyKind::Binary),
            );
            let s = snap_map(a, elemsize, KeyKind::Binary);
            assert_eq!(s.length, 1);
            assert!(s.elems[0].iter().all(|&x| x == 0), "elem 0 must be zeroed");
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

#[test]
fn err_e18_hmput_default_noop() {
    with_libs(0x31415926, |c, rs| unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            let mut a = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut b = (rs.hmput_default)(std::ptr::null_mut(), elemsize);
            // write a marker into element 0 -- a no-op call must NOT clear it
            std::ptr::write_bytes(hash_to_arr(a, elemsize) as *mut u8, 0x5C, elemsize);
            std::ptr::write_bytes(hash_to_arr(b, elemsize) as *mut u8, 0x5C, elemsize);
            for _ in 0..8 {
                let na = (c.hmput_default)(a, elemsize);
                let nb = (rs.hmput_default)(b, elemsize);
                assert_eq!(na, a, "no-op must return the same pointer");
                assert_eq!(nb, b);
                a = na;
                b = nb;
                assert_same(
                    "e18",
                    &snap_map(a, elemsize, KeyKind::Binary),
                    &snap_map(b, elemsize, KeyKind::Binary),
                );
                assert!(
                    snap_map(a, elemsize, KeyKind::Binary).elems[0]
                        .iter()
                        .all(|&x| x == 0x5C),
                    "no-op must not re-zero element 0"
                );
            }
            (c.hmfree_func)(hash_to_arr(a, elemsize), elemsize);
            (rs.hmfree_func)(hash_to_arr(b, elemsize), elemsize);
        }
    });
}

// ===========================================================================
// hashing edge cases  (E19..E21)
// ===========================================================================

#[test]
fn err_e19_hash_string_empty() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(19);
        let empty = [0u8; 1];
        for seed in [0usize, 1, 2, 0x31415926, usize::MAX, 1 << 63] {
            let a = (c.hash_string)(empty.as_ptr() as *mut c_char, seed);
            let b = (rs.hash_string)(empty.as_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\", {seed:#x})");
        }
        for _ in 0..2000 {
            let seed = rng.next_usize();
            let a = (c.hash_string)(empty.as_ptr() as *mut c_char, seed);
            let b = (rs.hash_string)(empty.as_ptr() as *mut c_char, seed);
            assert_eq!(a, b);
        }
    });
}

#[test]
fn err_e20_hash_bytes_zero_len() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut rng = Rng::new(20);
        let buf = rng.bytes(64);
        for seed in [0usize, 1, 0x31415926, usize::MAX, 1 << 63] {
            let a = (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed);
            let b = (rs.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, b, "hash_bytes(len=0, {seed:#x})");
        }
        for _ in 0..2000 {
            let seed = rng.next_usize();
            assert_eq!(
                (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed),
                (rs.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed)
            );
        }
    });
}

/// `len == 0` never dereferences `p`, so even a NULL pointer is accepted.
#[test]
fn err_e20b_hash_bytes_null_zero_len() {
    with_libs(0x31415926, |c, rs| {
        for seed in [0usize, 1, 0x31415926, usize::MAX] {
            let out = assert_same_crash(
                &format!("e20b hash_bytes(NULL,0,{seed:#x})"),
                || unsafe {
                    (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
                },
                || unsafe {
                    (rs.hash_bytes)(std::ptr::null_mut(), 0, seed);
                },
            );
            assert!(out.exited && out.code == 0, "must not fault: {out:?}");
        }
        unsafe {
            for seed in [0usize, 0x31415926, usize::MAX] {
                assert_eq!(
                    (c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                    (rs.hash_bytes)(std::ptr::null_mut(), 0, seed)
                );
            }
        }
    });
}

/// E21 -- the `if (hash < 2) hash += 2` guard.
///
/// A 64-bit hash of 0 or 1 is not constructible (it would need ~2^63 trials),
/// so the guard cannot be *forced*.  What is verified instead is the exact
/// mapping the guard implements: for every sampled key the value the library
/// stores in the bucket equals the value `stbds_hash_bytes`/`stbds_hash_string`
/// returns, i.e. `hash >= 2 => stored == hash`.  Both libraries must agree.
#[test]
fn err_e21_hash_lt_2_bump() {
    let mut rng = Rng::new(21);
    with_libs(0x31415926, |c, rs| unsafe {
        for &mode in &[HM_BINARY, HM_STRING] {
            for _ in 0..60 {
                let keys = Keys::random(&mut rng, 6, 16);
                let arena = if mode == HM_STRING {
                    Arena::Explicit(SH_DEFAULT)
                } else {
                    Arena::Auto
                };
                let mut m = MapPair::new(c, rs, E, K, mode, arena);
                for i in 0..keys.len() {
                    m.put(keys.ptr(i), i as u64);
                }
                let t = m.table_c().unwrap();
                let seed = t.seed;
                for i in 0..keys.len() {
                    let hc = if mode >= HM_STRING {
                        (c.hash_string)(keys.cptr(i), seed)
                    } else {
                        (c.hash_bytes)(keys.ptr(i), K, seed)
                    };
                    let hr = if mode >= HM_STRING {
                        (rs.hash_string)(keys.cptr(i), seed)
                    } else {
                        (rs.hash_bytes)(keys.ptr(i), K, seed)
                    };
                    assert_eq!(hc, hr, "e21 raw hash must agree");
                    let want = if hc < 2 { hc + 2 } else { hc };
                    assert!(
                        t.buckets.iter().any(|b| b.hash.contains(&want)),
                        "e21: stored bucket hash must be the guarded hash {want:#x}"
                    );
                    assert!(want >= 2, "bucket hashes must never be 0 or 1");
                }
                m.check("e21");
                m.free();
            }
        }
    });
}

// ===========================================================================
// string arena  (E22..E26, E50)
// ===========================================================================

fn blocksize_for(block: u8) -> usize {
    512usize.wrapping_shl((block >> 1) as u32)
}

#[test]
fn err_e22_stralloc_oversized() {
    with_libs(0x31415926, |c, rs| unsafe {
        for n in [512usize, 513, 1024, 5000, 65536] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut s = vec![b'X'; n];
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (rs.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_same("e22 arena", &snap_arena(&ac), &snap_arena(&ar));
            assert_eq!(ac.remaining, 0, "oversized-into-empty leaves remaining 0");
            assert_eq!(ac.block, 1, "block still advances");
            assert_eq!(strcmp(pc, s.as_ptr() as *const c_char), 0);
            assert_eq!(strcmp(pr, s.as_ptr() as *const c_char), 0);
            assert_eq!(pc as usize, ac.storage as usize + 8);
            assert_eq!(pr as usize, ar.storage as usize + 8);
            (c.strreset)(&mut ac);
            (rs.strreset)(&mut ar);
        }
    });
}

/// E50 -- oversized block spliced *behind* the head, `remaining` untouched.
#[test]
fn err_e50_stralloc_oversized_splice() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut small = vec![b's'; 5];
        small.push(0);
        (c.stralloc)(&mut ac, small.as_ptr() as *mut c_char);
        (rs.stralloc)(&mut ar, small.as_ptr() as *mut c_char);
        let head_c = ac.storage;
        let head_r = ar.storage;
        let rem = ac.remaining;
        for n in [512usize, 2000, 30000] {
            let mut s = vec![b'B'; n];
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (rs.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_same("e50 arena", &snap_arena(&ac), &snap_arena(&ar));
            assert_eq!(ac.storage, head_c, "head must not move");
            assert_eq!(ar.storage, head_r, "head must not move");
            assert_eq!(ac.remaining, rem, "remaining must not change");
            // the new block is `head->next`
            let second_c = *(head_c as *const usize);
            let second_r = *(head_r as *const usize);
            assert_eq!(pc as usize, second_c + 8);
            assert_eq!(pr as usize, second_r + 8);
            assert_eq!(strcmp(pc, s.as_ptr() as *const c_char), 0);
            assert_eq!(strcmp(pr, s.as_ptr() as *const c_char), 0);
        }
        (c.strreset)(&mut ac);
        (rs.strreset)(&mut ar);
        assert_same("e50 reset", &snap_arena(&ac), &snap_arena(&ar));
    });
}

#[test]
fn err_e23_stralloc_block_saturates() {
    with_libs(0x31415926, |c, rs| unsafe {
        // block 21 -> blocksize 512<<10 = 512 KiB (< 1 MiB) -> block becomes 22
        // block 22 -> blocksize 512<<11 = 1 MiB   (== cap)  -> block stays 22
        for (block, expect) in [(20u8, 21u8), (21, 22), (22, 22), (23, 23)] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            ac.block = block;
            ar.block = block;
            let mut s = vec![b'y'; 40];
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (rs.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_same(
                &format!("e23 block={block}"),
                &snap_arena(&ac),
                &snap_arena(&ar),
            );
            assert_eq!(ac.block, expect, "block progression from {block}");
            assert_eq!(ar.block, expect);
            assert_eq!(strcmp(pc, s.as_ptr() as *const c_char), 0);
            assert_eq!(strcmp(pr, s.as_ptr() as *const c_char), 0);
            (c.strreset)(&mut ac);
            (rs.strreset)(&mut ar);
        }
    });
}

/// E24 -- caller-supplied `block >= 23` makes `512 << (block>>1)` shift by more
/// than 63 bits.  The C relies on x86-64 masking the shift count to 6 bits; the
/// translation must produce the identical `blocksize` (often 0, which forces the
/// oversized path).  Only `block` values whose masked blocksize is 0 or small
/// are exercised, so no multi-gigabyte allocation is attempted.
#[test]
fn err_e24_stralloc_shift_overflow() {
    with_libs(0x31415926, |c, rs| unsafe {
        let mut tested_zero = 0usize;
        let mut tested = 0usize;
        for block in 0u8..=255 {
            let bs = blocksize_for(block);
            if !(bs == 0 || bs <= (1 << 21)) {
                continue; // would try to malloc gigabytes
            }
            if bs == 0 {
                tested_zero += 1;
            }
            tested += 1;
            for n in [0usize, 10, 600] {
                let mut ac = StringArena::new();
                let mut ar = StringArena::new();
                ac.block = block;
                ar.block = block;
                let mut s = vec![b'v'; n];
                s.push(0);
                let pc = (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
                let pr = (rs.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
                assert_same(
                    &format!("e24 block={block} bs={bs} n={n}"),
                    &snap_arena(&ac),
                    &snap_arena(&ar),
                );
                assert_eq!(strcmp(pc, s.as_ptr() as *const c_char), 0);
                assert_eq!(strcmp(pr, s.as_ptr() as *const c_char), 0);
                if bs == 0 {
                    // blocksize 0 => always the oversized path
                    assert_eq!(ac.remaining, 0, "block={block}");
                    assert_eq!(ar.remaining, 0);
                    assert_eq!(pc as usize, ac.storage as usize + 8);
                    assert_eq!(pr as usize, ar.storage as usize + 8);
                }
                (c.strreset)(&mut ac);
                (rs.strreset)(&mut ar);
            }
        }
        assert!(tested_zero >= 18, "expected many wrapped-to-zero blocksizes, got {tested_zero}");
        assert!(tested >= 30, "only {tested} block values exercised");
    });
}

/// E25 -- `STBDS_ASSERT(len <= a->remaining)` is unreachable: prove it by an
/// exhaustive `block` x `len` sweep in which neither library ever aborts.
#[test]
fn err_e25_stralloc_assert_unreachable() {
    with_libs(0x31415926, |c, rs| {
        let out = assert_same_crash(
            "e25 sweep",
            || unsafe {
                for block in 0u8..=45 {
                    if blocksize_for(block) > (1 << 21) {
                        continue;
                    }
                    for n in [0usize, 1, 2, 7, 8, 511, 512, 513, 1023, 1024] {
                        let mut a = StringArena::new();
                        a.block = block;
                        let mut s = vec![b'u'; n];
                        s.push(0);
                        (c.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                        (c.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                        (c.strreset)(&mut a);
                    }
                }
            },
            || unsafe {
                for block in 0u8..=45 {
                    if blocksize_for(block) > (1 << 21) {
                        continue;
                    }
                    for n in [0usize, 1, 2, 7, 8, 511, 512, 513, 1023, 1024] {
                        let mut a = StringArena::new();
                        a.block = block;
                        let mut s = vec![b'u'; n];
                        s.push(0);
                        (rs.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                        (rs.stralloc)(&mut a, s.as_ptr() as *mut c_char);
                        (rs.strreset)(&mut a);
                    }
                }
            },
        );
        assert!(
            out.exited && out.code == 0 && out.stderr.is_empty(),
            "the `len <= a->remaining` assert must never fire: {out:?}"
        );
    });
}

#[test]
fn err_e26_strreset_empty() {
    with_libs(0x31415926, |c, rs| unsafe {
        for _ in 0..5 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            for _ in 0..4 {
                (c.strreset)(&mut ac);
                (rs.strreset)(&mut ar);
                assert_same("e26", &snap_arena(&ac), &snap_arena(&ar));
                assert_eq!(ac, StringArena::new());
                assert_eq!(ar, StringArena::new());
            }
            // non-zero block/mode but NULL storage: the loop body never runs and
            // the whole struct is memset to 0
            ac.block = 7;
            ac.mode = 3;
            ac.remaining = 1234;
            ar.block = 7;
            ar.mode = 3;
            ar.remaining = 1234;
            (c.strreset)(&mut ac);
            (rs.strreset)(&mut ar);
            assert_eq!(ac, StringArena::new(), "strreset must zero every field");
            assert_eq!(ar, StringArena::new());
        }
    });
}

// ===========================================================================
// out-of-range enum values across the FFI boundary  (E27..E29, E45)
// ===========================================================================

/// E27 -- `stbds_shmode_func` truncates `mode` with `(unsigned char)`.
#[test]
fn err_e27_shmode_out_of_range_enum() {
    with_libs(0x31415926, |c, rs| unsafe {
        for &mode in &[
            0i32,
            1,
            2,
            3,
            4,
            5,
            127,
            128,
            255,
            256,
            257,
            511,
            -1,
            -2,
            -255,
            -256,
            1000,
            c_int::MAX,
            c_int::MIN,
        ] {
            let a = (c.shmode_func)(E, mode);
            let b = (rs.shmode_func)(E, mode);
            let sa = snap_map(a, E, KeyKind::Binary);
            let sb = snap_map(b, E, KeyKind::Binary);
            assert_same(&format!("e27 shmode_func({mode})"), &sa, &sb);
            let want = (mode as u32 & 0xff) as u8;
            assert_eq!(
                sa.table.as_ref().unwrap().arena_mode,
                want,
                "(unsigned char) truncation of mode={mode}"
            );
            assert_eq!(sa.length, 1);
            assert_eq!(sa.table.as_ref().unwrap().slot_count, 8);
            (c.hmfree_func)(hash_to_arr(a, E), E);
            (rs.hmfree_func)(hash_to_arr(b, E), E);
        }
    });
}

/// E28 -- `hmput_key` tests `mode >= STBDS_HM_STRING`, never `== 1`.
#[test]
fn err_e28_hmput_out_of_range_mode() {
    let mut rng = Rng::new(28);
    with_libs(0x31415926, |c, rs| {
        // any mode >= 1 must behave exactly like HM_STRING
        for &mode in &[1i32, 2, 3, 100, c_int::MAX] {
            let keys = Keys::random(&mut rng, 30, 16);
            let mut m = MapPair::new(c, rs, E, K, mode, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                m.check(&format!("e28 mode={mode} put#{i}"));
            }
            // the lazily created table must have become SH_DEFAULT
            assert_eq!(
                m.table_c().unwrap().arena_mode,
                SH_DEFAULT as u8,
                "mode={mode} must be treated as a string mode"
            );
            for i in 0..keys.len() {
                assert!(m.get(keys.ptr(i)) >= 0, "mode={mode} key {i}");
            }
            m.free();
        }
        // any mode <= 0 must behave exactly like HM_BINARY
        for &mode in &[0i32, -1, -2, -100, c_int::MIN] {
            let keys = BinKeys::random(&mut rng, 30, K);
            let mut m = MapPair::new(c, rs, E, K, mode, Arena::Auto);
            for i in 0..keys.len() {
                m.put(keys.ptr(i), i as u64);
                m.check(&format!("e28 mode={mode} put#{i}"));
            }
            assert_eq!(
                m.table_c().unwrap().arena_mode,
                SH_NONE as u8,
                "mode={mode} must be treated as a binary mode"
            );
            for i in 0..keys.len() {
                assert!(m.get(keys.ptr(i)) >= 0, "mode={mode} key {i}");
                assert_eq!(m.del(keys.ptr(i), 0), 1);
                m.check(&format!("e28 mode={mode} del#{i}"));
            }
            m.free();
        }
    });
}

/// E45 -- `switch (table->string.mode)` `default:` branch (raw `memcpy`).
#[test]
fn err_e45_string_mode_default_branch() {
    let mut rng = Rng::new(45);
    with_libs(0x31415926, |c, rs| unsafe {
        for &sh in &[0i32, 4, 5, 127, 255, 256, -1] {
            let truncated = (sh as u32 & 0xff) as u8;
            if matches!(truncated, 1 | 2 | 3) {
                continue; // those are the non-default branches
            }
            let keys = Keys::random(&mut rng, 5, 12);
            let a = (c.shmode_func)(E, sh);
            let b = (rs.shmode_func)(E, sh);
            let mut ta = a;
            let mut tb = b;
            for i in 0..keys.len() {
                ta = (c.hmput_key)(ta, E, keys.ptr(i), K, HM_STRING);
                tb = (rs.hmput_key)(tb, E, keys.ptr(i), K, HM_STRING);
                let ia = map_temp(ta, E);
                let ib = map_temp(tb, E);
                assert_same("e45 temp", &ia, &ib);
                // the element holds the first `keysize` *bytes of the string*
                let want = &keys.bufs[i][..K];
                for (name, t) in [("C", ta), ("RUST", tb)] {
                    let elem =
                        std::slice::from_raw_parts((t as *const u8).offset(ia * E as isize), K);
                    assert_eq!(elem, want, "{name}: default: branch must memcpy the key bytes");
                }
                write_elem_tail(ta, E, ia, K, &(i as u64).to_le_bytes());
                write_elem_tail(tb, E, ib, K, &(i as u64).to_le_bytes());
                assert_same(
                    &format!("e45 sh={sh} put#{i}"),
                    &snap_map(ta, E, KeyKind::Binary),
                    &snap_map(tb, E, KeyKind::Binary),
                );
            }
            assert_eq!(
                snap_map(ta, E, KeyKind::Binary).table.unwrap().arena_mode,
                truncated
            );
            (c.hmfree_func)(hash_to_arr(ta, E), E);
            (rs.hmfree_func)(hash_to_arr(tb, E), E);
        }
    });
}
