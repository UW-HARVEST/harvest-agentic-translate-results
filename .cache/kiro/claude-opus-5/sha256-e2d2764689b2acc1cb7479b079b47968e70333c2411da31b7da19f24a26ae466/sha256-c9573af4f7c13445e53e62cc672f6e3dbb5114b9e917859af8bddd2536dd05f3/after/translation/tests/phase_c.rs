//! Phase C — error-path differential tests, one test per `ERRORS.md` row,
//! plus the generic FFI boundaries (NULL, zero/oversized lengths, out-of-range
//! enum values crossing the FFI boundary).
//!
//! Rows that abort (`assert` failure) are run in a re-executed child process so
//! that the C `.so` and the Rust `.so` can be compared by exit signal.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;

const E_INT_INT: usize = 8;
const E_STR: usize = 16;

// ---------------------------------------------------------------------------
// Child-process driver for the abort scenarios
// ---------------------------------------------------------------------------

fn selected_lib() -> &'static Lib {
    let p = libs();
    match std::env::var("DIFF_LIB").as_deref() {
        Ok("c") => &p.c,
        Ok("rust") => &p.r,
        other => panic!("DIFF_LIB must be c|rust, got {other:?}"),
    }
}

/// (exit code, terminating signal) of `<self> --ignored --exact <scenario>`
fn run_scenario(scenario: &str, which: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "--test-threads=1", scenario])
        .env("DIFF_LIB", which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

const SIGABRT: i32 = 6;

fn assert_same_abort(scenario: &str) {
    let c = run_scenario(scenario, "c");
    let r = run_scenario(scenario, "rust");
    assert_eq!(c, r, "{scenario}: C {c:?} vs Rust {r:?}");
    assert_eq!(c.1, Some(SIGABRT), "{scenario}: expected SIGABRT, got {c:?}");
}

// ===========================================================================
// ERRORS row 1 -- stbds_make_hash_index assertion
//   used_count_threshold + tombstone_count_threshold >= slot_count
// Constructed by forcing a grow from a hand-crafted slot_count == 1 table, so
// stbds_make_hash_index(2, table) is reached: 2-0 + 0 >= 2 -> assert fails.
// ===========================================================================

#[test]
#[ignore = "abort scenario; driven by err01"]
fn scen_make_hash_index_assert() {
    let lib = selected_lib();
    unsafe {
        let t = (lib.shmode_func)(E_INT_INT, 0) as *mut u8;
        let tbl = hm_table(t, E_INT_INT);
        (*tbl).slot_count = 1;
        (*tbl).used_count = 0;
        (*tbl).used_count_threshold = 0; // used_count >= threshold -> grow
        let mut key = [1u8, 2, 3, 4];
        let _ = (lib.hmput_key)(
            t as *mut c_void,
            E_INT_INT,
            key.as_mut_ptr() as *mut c_void,
            4,
            0,
        );
        // must not get here
        eprintln!("NO ABORT");
        std::process::exit(70);
    }
}

#[test]
fn err01_make_hash_index_assert() {
    assert_same_abort("scen_make_hash_index_assert");
}

// ===========================================================================
// ERRORS row 2 -- hmput_key `(size_t)i+1 <= stbds_arrcap(a)`
// Only reachable on allocation failure; stbds_arrgrowf always sets
// capacity = min_cap >= min_len otherwise. Positive test: a table whose
// capacity is already enormous skips the grow entirely and must NOT abort.
// ===========================================================================

#[test]
fn err02_hmput_key_capacity_assert_not_reachable() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(2);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        for _ in 0..200 {
            let k = rng.bytes(4);
            let v = rng.bytes(4);
            assert_eq!(
                c_m.hmput(&k, &v, STBDS_HM_BINARY),
                r_m.hmput(&k, &v, STBDS_HM_BINARY),
                "err02"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err02");
            // the invariant the assert guards must hold on both
            let hc = *header(c_m.t.sub(E_INT_INT));
            let hr = *header(r_m.t.sub(E_INT_INT));
            assert!(hc.length <= hc.capacity && hr.length <= hr.capacity);
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS row 3 -- hmdel_key `slot < table->slot_count`
// stbds_hm_find_slot masks `pos` with slot_count-1, so the returned slot is
// always in range. Verified as an invariant over a long delete stream.
// ===========================================================================

#[test]
fn err03_hmdel_slot_in_range_invariant() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(3);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        let mut live: Vec<Vec<u8>> = Vec::new();
        for _ in 0..1500 {
            if rng.below(3) != 0 || live.is_empty() {
                let mut k = rng.bytes(4);
                k[3] = 0;
                let v = rng.bytes(4);
                c_m.hmput(&k, &v, STBDS_HM_BINARY);
                r_m.hmput(&k, &v, STBDS_HM_BINARY);
                if !live.contains(&k) {
                    live.push(k);
                }
            } else {
                let k = live.remove(rng.below(live.len()));
                assert_eq!(
                    c_m.hmdel(&k, STBDS_HM_BINARY, 0),
                    r_m.hmdel(&k, STBDS_HM_BINARY, 0),
                    "err03"
                );
            }
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err03");
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS row 4 -- `STBDS_ASSERT(table->used_count >= 0)` on a `size_t`
// is always true, so an underflow must NOT abort; it must wrap identically.
// ===========================================================================

#[test]
fn err04_used_count_underflow_does_not_abort() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(4);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..5u32 {
            let k = i.to_le_bytes().to_vec();
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
            keys.push(k);
        }
        // force used_count to 0 while slots are still in use
        (*hm_table(c_m.t, E_INT_INT)).used_count = 0;
        (*hm_table(r_m.t, E_INT_INT)).used_count = 0;
        // deleting now decrements 0 -> SIZE_MAX on both
        let last = keys.last().unwrap().clone();
        assert_eq!(
            c_m.hmdel(&last, STBDS_HM_BINARY, 0),
            r_m.hmdel(&last, STBDS_HM_BINARY, 0),
            "err04"
        );
        assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err04");
        assert_eq!(
            (*hm_table(c_m.t, E_INT_INT)).used_count,
            usize::MAX,
            "err04: size_t underflow expected"
        );
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS rows 5 & 6 -- hmdel_key `slot >= 0` / `b->index[i] == final_index`
// after the hole-filling move.  Reachable with mode >= 2: is_key_equal takes
// the string path but `mode == STBDS_HM_STRING` is false, so the re-lookup key
// is the RAW ELEMENT BYTES handed to stbds_hash_string -> not found -> abort.
// ===========================================================================

#[test]
#[ignore = "abort scenario; driven by err05"]
fn scen_hmdel_relookup_assert() {
    let lib = selected_lib();
    unsafe {
        let mut m = Map::new(lib, E_STR, 8, 8);
        // three distinct, long-lived keys
        let mut k: Vec<Vec<u8>> = (0..3)
            .map(|i| {
                let mut s = format!("relookup_key_{:04}_{}", i, "y".repeat(20)).into_bytes();
                s.push(0);
                s
            })
            .collect();
        for kk in k.iter_mut() {
            let v = [0u8; 8];
            m.shput(kk.as_mut_ptr() as *mut c_char, &v, 2);
        }
        // delete the FIRST element -> old_index != final_index -> re-lookup
        m.shdel(k[0].as_mut_ptr() as *mut c_char, 2, 0);
        eprintln!("NO ABORT");
        std::process::exit(70);
    }
}

#[test]
fn err05_hmdel_relookup_assert() {
    assert_same_abort("scen_hmdel_relookup_assert");
}

#[test]
fn err06_hmdel_index_assert_shares_row05_path() {
    // `STBDS_ASSERT(b->index[i] == final_index)` sits immediately after the
    // `slot >= 0` assert on the same crafted path; err05 already proves both
    // libraries abort there identically.  Nothing further is constructible
    // without corrupting the bucket array itself.
    assert_same_abort("scen_hmdel_relookup_assert");
}

// ===========================================================================
// ERRORS row 7 -- stralloc `len <= a->remaining`
// Unreachable: the `else` branch always sets remaining = blocksize >= len, and
// the `len > blocksize` branch returns early.  Boundary probe: block values
// whose blocksize wraps to 0 must take the oversized path, not abort.
// ===========================================================================

#[test]
fn err07_stralloc_assert_unreachable_blocksize_zero() {
    let p = libs();
    #[repr(C)]
    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
    struct Arena {
        storage: usize,
        remaining: usize,
        block: u8,
        mode: u8,
        _pad: [u8; 6],
    }
    // block>>1 in 55..=63 -> 512 << k overflows to 0
    for block in [110u8, 111, 120, 126, 127, 254, 255] {
        let k = ((block >> 1) as u32) & 63;
        let blocksize = 512usize.wrapping_shl(k);
        if blocksize != 0 {
            continue;
        }
        for len in [1usize, 5, 100, 700] {
            let mut ac = Arena { block, ..Default::default() };
            let mut ar = Arena { block, ..Default::default() };
            let mut s = vec![b'z'; len];
            s.push(0);
            let mut s2 = s.clone();
            unsafe {
                let rc = (p.c.stralloc)(
                    &mut ac as *mut Arena as *mut c_void,
                    s.as_mut_ptr() as *mut c_char,
                );
                let rr = (p.r.stralloc)(
                    &mut ar as *mut Arena as *mut c_void,
                    s2.as_mut_ptr() as *mut c_char,
                );
                assert_eq!(cstr(rc), cstr(rr), "err07: block={block} len={len}");
                assert_eq!(
                    (ac.storage == 0, ac.remaining, ac.block, ac.mode),
                    (ar.storage == 0, ar.remaining, ar.block, ar.mode),
                    "err07: block={block} len={len}"
                );
                (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
                (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
            }
        }
    }
}

// ===========================================================================
// ERRORS row 8 -- arr_push `arrlen(arr) == 0` never fires
// ===========================================================================

#[test]
fn err08_arr_push_assert_never_fires() {
    let p = libs();
    for num in [i32::MIN, -1000, -1, 0, 1, 50, 51, 200] {
        unsafe {
            (p.c.arr_push)(num as c_int);
            (p.r.arr_push)(num as c_int);
        }
    }
}

// ===========================================================================
// ERRORS row 9 -- arrgrowf early return (`min_cap <= arrcap`)
// ===========================================================================

#[test]
fn err09_arrgrowf_early_return() {
    let p = libs();
    unsafe {
        // a == NULL, nothing requested -> NULL, no allocation
        for elemsize in [0usize, 1, 4, 8, 4096] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(a.is_null(), "err09: C returned {a:?} for elemsize={elemsize}");
            assert!(b.is_null(), "err09: Rust returned {b:?} for elemsize={elemsize}");
        }
        // a != NULL and min_cap <= cap -> identical pointer, header untouched
        for elemsize in [1usize, 4, 16] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7) as *mut u8;
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7) as *mut u8;
            let sa = snap_arr(a, elemsize);
            let sb = snap_arr(b, elemsize);
            assert_eq!(sa, sb, "err09: fresh");
            for mc in [0usize, 1, 6, 7] {
                let a2 = (p.c.arrgrowf)(a as *mut c_void, elemsize, 0, mc) as *mut u8;
                let b2 = (p.r.arrgrowf)(b as *mut c_void, elemsize, 0, mc) as *mut u8;
                assert_eq!(a2, a, "err09: C must return the same pointer");
                assert_eq!(b2, b, "err09: Rust must return the same pointer");
                assert_eq!(snap_arr(a2, elemsize), snap_arr(b2, elemsize), "err09");
                assert_eq!(snap_arr(a2, elemsize), sa, "err09: header untouched");
            }
            (p.c.arrfreef)(a as *mut c_void);
            (p.r.arrfreef)(b as *mut c_void);
        }
    }
}

// ===========================================================================
// ERRORS row 10 & 44 -- NULL handling
// ===========================================================================

#[test]
fn err10_null_pointer_inputs() {
    let p = libs();
    unsafe {
        // hmfree_func(NULL, *) is an explicit no-op
        for elemsize in [1usize, 8, 16, 1024] {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
        // hmput_default(NULL, es)
        for elemsize in [1usize, 4, 8, 16, 64] {
            let a = (p.c.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            let b = (p.r.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            assert!(!a.is_null() && !b.is_null());
            assert_eq!(
                snap_map(a, elemsize, KeyRepr::Raw),
                snap_map(b, elemsize, KeyRepr::Raw),
                "err10: hmput_default(NULL)"
            );
            (p.c.hmfree_func)(a.sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)(b.sub(elemsize) as *mut c_void, elemsize);
        }
        // hmdel_key(NULL, ...) -> NULL, key never dereferenced
        for mode in [0i32, 1, 2, -1, i32::MAX] {
            let a = (p.c.hmdel_key)(
                std::ptr::null_mut(),
                E_INT_INT,
                std::ptr::null_mut(),
                4,
                0,
                mode as c_int,
            );
            let b = (p.r.hmdel_key)(
                std::ptr::null_mut(),
                E_INT_INT,
                std::ptr::null_mut(),
                4,
                0,
                mode as c_int,
            );
            assert!(a.is_null() && b.is_null(), "err10: hmdel_key(NULL) mode={mode}");
        }
        // hash_bytes(NULL, 0, seed)
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            assert_eq!(
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed),
                "err10: hash_bytes(NULL,0)"
            );
        }
    }
}

// ===========================================================================
// ERRORS rows 11, 12, 15, 16, 17 -- lookup misses return the -1 sentinel
// ===========================================================================

#[test]
fn err11_lookup_miss_returns_minus_one() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(11);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        // fill enough that probing wraps inside buckets (both scan loops)
        for i in 0..100u32 {
            let k = (i * 7 + 1).to_le_bytes().to_vec();
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
        }
        let mut misses = 0;
        for _ in 0..3000 {
            let k = rng.bytes(4);
            let a = c_m.hmgeti(&k, STBDS_HM_BINARY);
            let b = r_m.hmgeti(&k, STBDS_HM_BINARY);
            assert_eq!(a, b, "err11: key={k:02x?}");
            if a == -1 {
                misses += 1;
            }
            // the pointer must be returned unchanged by a lookup (row 16)
            let pc = c_m.t;
            let pr = r_m.t;
            assert_eq!(c_m.hmgeti(&k, STBDS_HM_BINARY), a);
            assert_eq!(r_m.hmgeti(&k, STBDS_HM_BINARY), b);
            assert_eq!(c_m.t, pc, "err11: hmget_key must not move the table (C)");
            assert_eq!(r_m.t, pr, "err11: hmget_key must not move the table (Rust)");
        }
        assert!(misses > 2000, "err11: expected mostly misses, got {misses}");
        c_m.free();
        r_m.free();
    }
}

#[test]
fn err12_probe_wraparound_miss() {
    let p = libs();
    // Densely fill a small table so that misses have to traverse the
    // wrap-around (`i = 0 .. limit`) scan before hitting an empty slot.
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(12);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        for i in 0..6u32 {
            let k = i.to_le_bytes().to_vec();
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
        }
        for _ in 0..5000 {
            let k = rng.bytes(4);
            assert_eq!(
                c_m.hmgeti(&k, STBDS_HM_BINARY),
                r_m.hmgeti(&k, STBDS_HM_BINARY),
                "err12"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err12");
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS row 13 -- hmget_key_ts(NULL, ...) -> *temp = -1, non-NULL result
// ===========================================================================

#[test]
fn err13_hmget_key_ts_null_table() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 24] {
        for mode in [0i32, 1, 2, -1] {
            reset_seeds(p, 0x3141_5926);
            unsafe {
                let mut key = [0xABu8; 8];
                let mut tc: isize = 0x7777;
                let mut tr: isize = 0x7777;
                let a = (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    4,
                    &mut tc,
                    mode as c_int,
                ) as *mut u8;
                let b = (p.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    4,
                    &mut tr,
                    mode as c_int,
                ) as *mut u8;
                assert_eq!(tc, -1, "err13: C *temp");
                assert_eq!(tr, -1, "err13: Rust *temp");
                assert!(!a.is_null() && !b.is_null());
                assert_eq!(
                    snap_map(a, elemsize, KeyRepr::Raw),
                    snap_map(b, elemsize, KeyRepr::Raw),
                    "err13: elemsize={elemsize} mode={mode}"
                );
                (p.c.hmfree_func)(a.sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)(b.sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 14 & 19 -- hash_table == NULL: `key` is never dereferenced,
// so a NULL key must be accepted by both hmget_key_ts and hmdel_key.
// ===========================================================================

#[test]
fn err14_no_table_accepts_null_key() {
    let p = libs();
    for elemsize in [4usize, 8, 16] {
        for mode in [0i32, 1, 2, -5, i32::MAX] {
            unsafe {
                let a = (p.c.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
                let b = (p.r.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
                assert!((*header(a.sub(elemsize))).hash_table.is_null());
                assert!((*header(b.sub(elemsize))).hash_table.is_null());

                // row 14: hmget_key_ts with a NULL key
                let mut tc: isize = 0x1234;
                let mut tr: isize = 0x1234;
                let a2 = (p.c.hmget_key_ts)(
                    a as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    &mut tc,
                    mode as c_int,
                ) as *mut u8;
                let b2 = (p.r.hmget_key_ts)(
                    b as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    &mut tr,
                    mode as c_int,
                ) as *mut u8;
                assert_eq!((tc, tr), (-1, -1), "err14: *temp must be -1");
                assert_eq!(a2, a, "err14: pointer returned unchanged (C)");
                assert_eq!(b2, b, "err14: pointer returned unchanged (Rust)");

                // row 14 via hmget_key: the header temp field gets the sentinel
                let a3 = (p.c.hmget_key)(
                    a as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    mode as c_int,
                ) as *mut u8;
                let b3 = (p.r.hmget_key)(
                    b as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    mode as c_int,
                ) as *mut u8;
                assert_eq!(hm_temp(a3, elemsize), -1);
                assert_eq!(hm_temp(b3, elemsize), -1);

                // row 19: hmdel_key with a NULL key, no table -> temp = 0
                let a4 = (p.c.hmdel_key)(
                    a as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    0,
                    mode as c_int,
                ) as *mut u8;
                let b4 = (p.r.hmdel_key)(
                    b as *mut c_void,
                    elemsize,
                    std::ptr::null_mut(),
                    4,
                    0,
                    mode as c_int,
                ) as *mut u8;
                assert_eq!(a4, a, "err19: returns a (C)");
                assert_eq!(b4, b, "err19: returns a (Rust)");
                assert_eq!(hm_temp(a4, elemsize), 0, "err19: temp = 0 (C)");
                assert_eq!(hm_temp(b4, elemsize), 0, "err19: temp = 0 (Rust)");
                assert_eq!(
                    snap_map(a4, elemsize, KeyRepr::Raw),
                    snap_map(b4, elemsize, KeyRepr::Raw),
                    "err14/19: elemsize={elemsize} mode={mode}"
                );
                (p.c.hmfree_func)(a.sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)(b.sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// ERRORS row 17 -- hmget_key mirrors the ts out-param into the header
// ===========================================================================

#[test]
fn err17_hmget_key_stores_sentinel_in_header() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(17);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        for i in 0..30u32 {
            let k = i.to_le_bytes().to_vec();
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
        }
        for i in 30..200u32 {
            let k = i.to_le_bytes().to_vec();
            let hc = c_m.hmgeti(&k, STBDS_HM_BINARY);
            let hr = r_m.hmgeti(&k, STBDS_HM_BINARY);
            let tc = c_m.hmgeti_ts(&k, STBDS_HM_BINARY);
            let tr = r_m.hmgeti_ts(&k, STBDS_HM_BINARY);
            assert_eq!((hc, tc), (hr, tr), "err17: i={i}");
            assert_eq!(hc, tc, "err17: header temp == ts out-param (C)");
            assert_eq!(hr, tr, "err17: header temp == ts out-param (Rust)");
            assert_eq!(hc, -1, "err17: absent key must be -1");
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS row 18 -- hmdel_key(NULL, ...) -> NULL (covered in err10, asserted
// here across every elemsize/keysize/keyoffset combination)
// ===========================================================================

#[test]
fn err18_hmdel_key_null_returns_null() {
    let p = libs();
    let mut key = [7u8; 16];
    for elemsize in [1usize, 4, 8, 16] {
        for keysize in [0usize, 1, 4, 8, 16] {
            for keyoffset in [0usize, 4, 1024] {
                for mode in [0i32, 1, 2, -1] {
                    unsafe {
                        let a = (p.c.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode as c_int,
                        );
                        let b = (p.r.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode as c_int,
                        );
                        assert!(a.is_null() && b.is_null(), "err18: {a:?} {b:?}");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 20, 21, 22 -- delete of an absent vs present key
// ===========================================================================

#[test]
fn err20_21_22_delete_sentinels() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(2021);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        let present: Vec<Vec<u8>> = (0..30u32).map(|i| i.to_le_bytes().to_vec()).collect();
        for k in &present {
            let v = rng.bytes(4);
            c_m.hmput(k, &v, STBDS_HM_BINARY);
            r_m.hmput(k, &v, STBDS_HM_BINARY);
        }
        // row 20: absent key -> temp == 0 and length unchanged
        for i in 1000..1200u32 {
            let k = i.to_le_bytes().to_vec();
            let lc = hm_raw_len(c_m.t, E_INT_INT);
            let dc = c_m.hmdel(&k, STBDS_HM_BINARY, 0);
            let dr = r_m.hmdel(&k, STBDS_HM_BINARY, 0);
            assert_eq!((dc, dr), (0, 0), "err20: absent key must yield 0");
            assert_eq!(hm_raw_len(c_m.t, E_INT_INT), lc, "err20: length unchanged");
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err20");
        }
        // row 21: present key -> temp == 1 and length -= 1
        for k in &present {
            let lc = hm_raw_len(c_m.t, E_INT_INT);
            let dc = c_m.hmdel(k, STBDS_HM_BINARY, 0);
            let dr = r_m.hmdel(k, STBDS_HM_BINARY, 0);
            assert_eq!((dc, dr), (1, 1), "err21: present key must yield 1");
            assert_eq!(hm_raw_len(c_m.t, E_INT_INT), lc - 1, "err21: length -= 1");
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err21");
            // deleting twice: second time reports 0
            assert_eq!(
                c_m.hmdel(k, STBDS_HM_BINARY, 0),
                r_m.hmdel(k, STBDS_HM_BINARY, 0),
                "err21: double delete"
            );
        }
        // row 22: a fresh table has no tombstones, so tombstone_count stays 0
        let mut c2 = Map::new(&p.c, E_INT_INT, 4, 4);
        let mut r2 = Map::new(&p.r, E_INT_INT, 4, 4);
        for i in 0..6u32 {
            let k = i.to_le_bytes().to_vec();
            let v = rng.bytes(4);
            c2.hmput(&k, &v, STBDS_HM_BINARY);
            r2.hmput(&k, &v, STBDS_HM_BINARY);
            assert_eq!(c2.snap(KeyRepr::Raw), r2.snap(KeyRepr::Raw), "err22");
            assert_eq!((*hm_table(c2.t, E_INT_INT)).tombstone_count, 0, "err22");
            assert_eq!((*hm_table(r2.t, E_INT_INT)).tombstone_count, 0, "err22");
        }
        c2.free();
        r2.free();
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS rows 23, 24, 25 -- arrgrowf min_cap clamping
// ===========================================================================

#[test]
fn err23_24_25_arrgrowf_clamping() {
    let p = libs();
    unsafe {
        // fresh array: min_cap < 4 must be clamped to 4
        for (addlen, min_cap) in [
            (0usize, 1usize),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (5, 0),
            (3, 2),
            (2, 3),
            (9, 4),
            (4, 9),
        ] {
            for elemsize in [1usize, 4, 8, 16] {
                let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) as *mut u8;
                let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) as *mut u8;
                let sa = snap_arr(a, elemsize);
                let sb = snap_arr(b, elemsize);
                assert_eq!(sa, sb, "err23: addlen={addlen} min_cap={min_cap}");
                if !sa.null {
                    let want = std::cmp::max(std::cmp::max(addlen, min_cap), 4);
                    assert_eq!(sa.capacity, want, "err25: clamp to >=4");
                }
                if !a.is_null() {
                    (p.c.arrfreef)(a as *mut c_void);
                    (p.r.arrfreef)(b as *mut c_void);
                }
            }
        }
        // doubling: min_cap < 2*cap -> 2*cap
        for elemsize in [4usize, 16] {
            let mut a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) as *mut u8;
            let mut b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8) as *mut u8;
            for want in [9usize, 17, 33, 65, 129] {
                a = (p.c.arrgrowf)(a as *mut c_void, elemsize, 0, want) as *mut u8;
                b = (p.r.arrgrowf)(b as *mut c_void, elemsize, 0, want) as *mut u8;
                assert_eq!(snap_arr(a, elemsize), snap_arr(b, elemsize), "err24: want={want}");
            }
            (p.c.arrfreef)(a as *mut c_void);
            (p.r.arrfreef)(b as *mut c_void);
        }
    }
}

// ===========================================================================
// ERRORS rows 26, 30, 31 -- threshold initialisation & implicit string.mode
// ===========================================================================

#[test]
fn err26_30_31_threshold_and_string_mode() {
    let p = libs();
    unsafe {
        // row 26: slot_count <= 8 -> used_count_shrink_threshold == 0
        for m in [0i32, 1, 2, 3] {
            reset_seeds(p, 0x3141_5926);
            let a = (p.c.shmode_func)(E_STR, m as c_int) as *mut u8;
            let b = (p.r.shmode_func)(E_STR, m as c_int) as *mut u8;
            assert_eq!(
                snap_map(a, E_STR, KeyRepr::Raw),
                snap_map(b, E_STR, KeyRepr::Raw),
                "err26: mode={m}"
            );
            let ta = hm_table(a, E_STR);
            assert_eq!((*ta).slot_count, 8);
            assert_eq!((*ta).used_count_shrink_threshold, 0, "err26");
            assert_eq!((*ta).used_count_threshold, 6);
            assert_eq!((*ta).tombstone_count_threshold, 1);
            (p.c.hmfree_func)(a.sub(E_STR) as *mut c_void, E_STR);
            (p.r.hmfree_func)(b.sub(E_STR) as *mut c_void, E_STR);
        }
        // rows 30 & 31: string.mode chosen by the first hmput_key's mode
        for (mode, want) in [
            (0i32, 0u8),
            (-1, 0),
            (i32::MIN, 0),
            (1, 1),
            (2, 1),
            (7, 1),
            (i32::MAX, 1),
        ] {
            reset_seeds(p, 0x3141_5926);
            let mut key = *b"abcdefghij\0\0\0\0\0\0";
            let a = (p.c.hmput_key)(
                std::ptr::null_mut(),
                E_STR,
                key.as_mut_ptr() as *mut c_void,
                8,
                mode as c_int,
            ) as *mut u8;
            let b2 = (p.r.hmput_key)(
                std::ptr::null_mut(),
                E_STR,
                key.as_mut_ptr() as *mut c_void,
                8,
                mode as c_int,
            ) as *mut u8;
            // complete the element like a struct assignment would: the value
            // half is indeterminate straight out of `hmput_key`.
            let ic = hm_temp(a, E_STR);
            let ir = hm_temp(b2, E_STR);
            assert_eq!(ic, ir, "err30/31: index mode={mode}");
            let v = [0x5Au8; 8];
            std::ptr::copy_nonoverlapping(v.as_ptr(), a.offset(ic * E_STR as isize).add(8), 8);
            std::ptr::copy_nonoverlapping(v.as_ptr(), b2.offset(ir * E_STR as isize).add(8), 8);
            assert_eq!((*hm_table(a, E_STR)).string.mode, want, "err30/31: mode={mode}");
            assert_eq!((*hm_table(b2, E_STR)).string.mode, want, "err30/31: mode={mode}");
            assert_eq!(
                snap_map(a, E_STR, KeyRepr::Raw),
                snap_map(b2, E_STR, KeyRepr::Raw),
                "err30/31: mode={mode}"
            );
            (p.c.hmfree_func)(a.sub(E_STR) as *mut c_void, E_STR);
            (p.r.hmfree_func)(b2.sub(E_STR) as *mut c_void, E_STR);
        }
    }
}

// ===========================================================================
// ERRORS row 27 -- `if (hash < 2) hash += 2`
// A hash of exactly 0 or 1 cannot be produced on demand (2^-63), but the
// consequence is observable: no occupied bucket may ever carry hash 0 or 1,
// and both libraries must agree on every bucket word.
// ===========================================================================

#[test]
fn err27_hash_never_collides_with_sentinels() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(27);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        for _ in 0..500 {
            let k = rng.bytes(4);
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
        }
        let sc = c_m.snap(KeyRepr::Raw);
        assert_eq!(sc, r_m.snap(KeyRepr::Raw), "err27");
        let t = sc.table.unwrap();
        for b in &t.buckets {
            for (j, h) in b.hash.iter().enumerate() {
                if b.index[j] >= 0 {
                    assert!(*h >= 2, "err27: occupied slot has sentinel hash {h}");
                }
            }
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS rows 28, 29, 47 -- `mode` is an int, not an enum: out-of-range values
// crossing the FFI boundary must behave identically.
// ===========================================================================

#[test]
fn err28_29_47_out_of_range_mode_binary_side() {
    let p = libs();
    // mode < 1 -> binary path for every entry point
    for mode in [0i32, -1, -2, -128, -32768, i32::MIN, i32::MIN + 1] {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(28);
        let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
        let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
        unsafe {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..40 {
                let k = rng.bytes(4);
                let v = rng.bytes(4);
                assert_eq!(
                    c_m.hmput(&k, &v, mode as c_int),
                    r_m.hmput(&k, &v, mode as c_int),
                    "err28: put mode={mode}"
                );
                assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err28: mode={mode}");
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            for k in &keys {
                assert_eq!(
                    c_m.hmgeti(k, mode as c_int),
                    r_m.hmgeti(k, mode as c_int),
                    "err28: get mode={mode}"
                );
                assert_eq!(
                    c_m.hmgeti_ts(k, mode as c_int),
                    r_m.hmgeti_ts(k, mode as c_int),
                    "err28: get_ts mode={mode}"
                );
            }
            for _ in 0..40 {
                let k = rng.bytes(4);
                assert_eq!(
                    c_m.hmgeti(&k, mode as c_int),
                    r_m.hmgeti(&k, mode as c_int),
                    "err28: miss mode={mode}"
                );
            }
            while let Some(k) = keys.pop() {
                assert_eq!(
                    c_m.hmdel(&k, mode as c_int, 0),
                    r_m.hmdel(&k, mode as c_int, 0),
                    "err28: del mode={mode}"
                );
                assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err28: mode={mode}");
            }
            c_m.free();
            r_m.free();
        }
    }
}

// ===========================================================================
// ERRORS row 33 -- `switch (table->string.mode)` default arm
// An out-of-range string.mode (4..255, set through stbds_shmode_func) must fall
// into the `memcpy(elem, key, keysize)` arm on both sides.
// ===========================================================================

#[test]
fn err33_string_mode_default_arm() {
    let p = libs();
    for shmode in [0i32, 4, 5, 17, 100, 254, 255] {
        reset_seeds(p, 0x3141_5926);
        unsafe {
            let mut c_t = (p.c.shmode_func)(E_STR, shmode as c_int) as *mut u8;
            let mut r_t = (p.r.shmode_func)(E_STR, shmode as c_int) as *mut u8;
            // Insert with STBDS_HM_STRING: the hash comes from stbds_hash_string
            // but the key is stored with the default arm's memcpy.  Only inserts
            // and structural comparison -- a subsequent LOOKUP would reinterpret
            // those raw bytes as a char* (a genuine C bug we must not trigger).
            let keys: Vec<Vec<u8>> = (0..5)
                .map(|i| {
                    let mut s = format!("defarm_{:03}_{}", i, "w".repeat(24)).into_bytes();
                    s.push(0);
                    s
                })
                .collect();
            for k in &keys {
                let kp = k.as_ptr() as *mut c_char;
                c_t = (p.c.hmput_key)(c_t as *mut c_void, E_STR, kp as *mut c_void, 8, 1)
                    as *mut u8;
                r_t = (p.r.hmput_key)(r_t as *mut c_void, E_STR, kp as *mut c_void, 8, 1)
                    as *mut u8;
                let ic = hm_temp(c_t, E_STR);
                let ir = hm_temp(r_t, E_STR);
                assert_eq!(ic, ir, "err33: shmode={shmode}");
                // finish the element like a struct assignment would
                let v = [0xEEu8; 8];
                std::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    c_t.offset(ic * E_STR as isize).add(8),
                    8,
                );
                std::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    r_t.offset(ir * E_STR as isize).add(8),
                    8,
                );
                assert_eq!(
                    snap_map(c_t, E_STR, KeyRepr::Raw),
                    snap_map(r_t, E_STR, KeyRepr::Raw),
                    "err33: shmode={shmode}"
                );
                // the default arm copies the first 8 bytes of the key string
                let stored = std::slice::from_raw_parts(c_t.offset(ic * E_STR as isize), 8);
                assert_eq!(stored, &k[..8], "err33: memcpy arm, shmode={shmode}");
            }
            // free the array only: hmfree_func would walk the bogus key pointers
            (p.c.arrfreef)(c_t.sub(E_STR) as *mut c_void);
            (p.r.arrfreef)(r_t.sub(E_STR) as *mut c_void);
        }
    }
}

// ===========================================================================
// ERRORS rows 34 & 35 -- `mode == STBDS_HM_STRING` exactly
// mode 1 frees the strdup'd key and re-looks-up through char**; mode 2 does
// neither.  Compared on the delete-last path, which does not abort.
// ===========================================================================

#[test]
fn err34_35_mode_exact_equality_on_delete() {
    let p = libs();
    for mode in [1i32, 2, 3, 99] {
        for shmode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            reset_seeds(p, 0x3141_5926);
            let kr = KeyRepr::PtrString { off: 0 };
            let owned: Vec<Vec<u8>> = (0..6)
                .map(|i| {
                    let mut s = format!("exact_{:03}_{}", i, "v".repeat(20)).into_bytes();
                    s.push(0);
                    s
                })
                .collect();
            let mut c_m = Map::new(&p.c, E_STR, 8, 8);
            let mut r_m = Map::new(&p.r, E_STR, 8, 8);
            unsafe {
                c_m.sh_new(shmode);
                r_m.sh_new(shmode);
                for k in &owned {
                    let kp = k.as_ptr() as *mut c_char;
                    let v = [0x11u8; 8];
                    assert_eq!(
                        c_m.shput(kp, &v, mode as c_int),
                        r_m.shput(kp, &v, mode as c_int),
                        "err34: put mode={mode} shmode={shmode}"
                    );
                }
                assert_eq!(c_m.snap(kr), r_m.snap(kr), "err34: filled");
                // delete from the BACK: old_index == final_index every time, so
                // the divergent re-lookup path is never entered.
                for k in owned.iter().rev() {
                    let kp = k.as_ptr() as *mut c_char;
                    assert_eq!(
                        c_m.shdel(kp, mode as c_int, 0),
                        r_m.shdel(kp, mode as c_int, 0),
                        "err34: del mode={mode} shmode={shmode}"
                    );
                    assert_eq!(
                        c_m.snap(kr),
                        r_m.snap(kr),
                        "err34/35: mode={mode} shmode={shmode}"
                    );
                }
                c_m.free();
                r_m.free();
            }
        }
    }
}

// ===========================================================================
// ERRORS rows 36 & 37 -- shrink (÷2) and tombstone rebuild (same size)
// ===========================================================================

#[test]
fn err36_37_shrink_and_rebuild() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(3637);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        let keys: Vec<Vec<u8>> = (0..300u32).map(|i| i.to_le_bytes().to_vec()).collect();
        for k in &keys {
            let v = rng.bytes(4);
            c_m.hmput(k, &v, STBDS_HM_BINARY);
            r_m.hmput(k, &v, STBDS_HM_BINARY);
        }
        let grown = (*hm_table(c_m.t, E_INT_INT)).slot_count;
        assert!(grown >= 512, "err36: expected growth, slot_count={grown}");
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        let mut prev = grown;
        let mut prev_tomb = (*hm_table(c_m.t, E_INT_INT)).tombstone_count;
        for k in &keys {
            assert_eq!(
                c_m.hmdel(k, STBDS_HM_BINARY, 0),
                r_m.hmdel(k, STBDS_HM_BINARY, 0),
                "err36: del"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err36/37");
            let t = hm_table(c_m.t, E_INT_INT);
            if (*t).slot_count < prev {
                saw_shrink = true;
            } else if (*t).tombstone_count < prev_tomb {
                saw_rebuild = true;
            }
            prev = (*t).slot_count;
            prev_tomb = (*t).tombstone_count;
        }
        assert!(saw_shrink, "err36: shrink path never taken");
        assert!(saw_rebuild, "err37: tombstone rebuild path never taken");
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// ERRORS rows 38, 39, 40, 41, 50 -- arena block sizing & reset
// (the exhaustive sweep lives in phase_b row50/row51; here the individual
// branch decisions are pinned down)
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Arena {
    storage: usize,
    remaining: usize,
    block: u8,
    mode: u8,
    _pad: [u8; 6],
}

impl Arena {
    fn norm(&self) -> (bool, usize, u8, u8) {
        (self.storage == 0, self.remaining, self.block, self.mode)
    }
}

#[test]
fn err38_39_40_41_arena_branches() {
    let p = libs();
    unsafe {
        // row 38/39: first allocation -> blocksize 512, block becomes 1
        let mut ac = Arena::default();
        let mut ar = Arena::default();
        let mut s = b"hi\0".to_vec();
        let mut s2 = s.clone();
        let rc = (p.c.stralloc)(&mut ac as *mut Arena as *mut c_void, s.as_mut_ptr() as *mut c_char);
        let rr = (p.r.stralloc)(&mut ar as *mut Arena as *mut c_void, s2.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr(rc), cstr(rr));
        assert_eq!(ac.norm(), ar.norm(), "err38");
        assert_eq!(ac.block, 1, "err39: ++a->block");
        assert_eq!(ac.remaining, 512 - 3, "err38: blocksize 512 minus len");

        // row 40: oversized string with an existing block -> spliced behind the
        // head; `remaining` must NOT change
        let before = ac.remaining;
        let mut big = vec![b'B'; 5000];
        big.push(0);
        let mut big2 = big.clone();
        let rc = (p.c.stralloc)(&mut ac as *mut Arena as *mut c_void, big.as_mut_ptr() as *mut c_char);
        let rr = (p.r.stralloc)(&mut ar as *mut Arena as *mut c_void, big2.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr(rc).len(), 5000);
        assert_eq!(cstr(rc), cstr(rr));
        assert_eq!(ac.norm(), ar.norm(), "err40");
        assert_eq!(ac.remaining, before, "err40: remaining untouched");

        // row 41: strreset frees the whole chain and zeroes the arena
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);
        assert_eq!(ac, Arena::default(), "err41");
        // and on an already-empty arena
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);

        // row 40 variant: oversized as the FIRST allocation -> becomes head and
        // sets remaining = 0
        let mut ac = Arena::default();
        let mut ar = Arena::default();
        let mut big = vec![b'C'; 900];
        big.push(0);
        let mut big2 = big.clone();
        let rc = (p.c.stralloc)(&mut ac as *mut Arena as *mut c_void, big.as_mut_ptr() as *mut c_char);
        let rr = (p.r.stralloc)(&mut ar as *mut Arena as *mut c_void, big2.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr(rc), cstr(rr));
        assert_eq!(ac.norm(), ar.norm(), "err40b");
        assert_eq!(ac.remaining, 0, "err40b: remaining = 0");
        assert_eq!(ac.block, 1);
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);

        // row 39 boundary: the ++block stops once blocksize reaches 1<<20
        let mut ac = Arena { block: 22, ..Default::default() };
        let mut ar = Arena { block: 22, ..Default::default() };
        let mut s = vec![b'D'; 10];
        s.push(0);
        let mut s2 = s.clone();
        let rc = (p.c.stralloc)(&mut ac as *mut Arena as *mut c_void, s.as_mut_ptr() as *mut c_char);
        let rr = (p.r.stralloc)(&mut ar as *mut Arena as *mut c_void, s2.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr(rc), cstr(rr));
        assert_eq!(ac.norm(), ar.norm(), "err39: 512<<11 == 1<<20");
        assert_eq!(ac.block, 22, "err39: block must NOT advance at the cap");
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);
    }
}

// ===========================================================================
// ERRORS rows 42 & 43 -- arr_push loop bounds
// ===========================================================================

#[test]
fn err42_arr_push_non_positive() {
    let p = libs();
    for num in [0i32, -1, -50, -51, -1000, i32::MIN] {
        unsafe {
            (p.c.arr_push)(num as c_int);
            (p.r.arr_push)(num as c_int);
        }
    }
    // row 43: `i += 50` on a signed int would overflow for num == INT_MAX
    // (undefined behaviour in C); deliberately NOT exercised. The largest
    // value that terminates in bounded time is checked instead.
    unsafe {
        (p.c.arr_push)(20_000);
        (p.r.arr_push)(20_000);
    }
}

// ===========================================================================
// ERRORS row 45 -- zero lengths
// ===========================================================================

#[test]
fn err45_zero_lengths() {
    let p = libs();
    unsafe {
        // hash_bytes with len 0 over a non-NULL pointer
        let mut buf = [0u8; 8];
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            assert_eq!(
                (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed),
                (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed),
                "err45: hash_bytes len 0"
            );
        }
        // hash_string on the empty string
        let mut empty = [0i8; 1];
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            assert_eq!(
                (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed),
                "err45: hash_string empty"
            );
        }
        // keysize == 0: memcmp of 0 bytes always matches, so the first key in
        // the probe chain is returned for every lookup.
        reset_seeds(p, 0x3141_5926);
        let mut c_m = Map::new(&p.c, E_INT_INT, 0, 4);
        let mut r_m = Map::new(&p.r, E_INT_INT, 0, 4);
        for i in 0..10u32 {
            let v = i.to_le_bytes().to_vec();
            assert_eq!(
                c_m.hmput(&[], &v, STBDS_HM_BINARY),
                r_m.hmput(&[], &v, STBDS_HM_BINARY),
                "err45: keysize 0 put"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err45: keysize 0");
        }
        assert_eq!(
            c_m.hmgeti(&[], STBDS_HM_BINARY),
            r_m.hmgeti(&[], STBDS_HM_BINARY),
            "err45: keysize 0 get"
        );
        assert_eq!(
            c_m.hmdel(&[], STBDS_HM_BINARY, 0),
            r_m.hmdel(&[], STBDS_HM_BINARY, 0),
            "err45: keysize 0 del"
        );
        assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err45: keysize 0 del");
        c_m.free();
        r_m.free();

        // elemsize == 0 through arrgrowf
        let a = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 1, 0) as *mut u8;
        let b = (p.r.arrgrowf)(std::ptr::null_mut(), 0, 1, 0) as *mut u8;
        assert_eq!(snap_arr(a, 0), snap_arr(b, 0), "err45: elemsize 0");
        (p.c.arrfreef)(a as *mut c_void);
        (p.r.arrfreef)(b as *mut c_void);
    }
}

// ===========================================================================
// ERRORS row 46 -- oversized lengths: elemsize*min_cap wraps `size_t`
// ===========================================================================

#[test]
fn err46_oversized_elemsize_overflow() {
    let p = libs();
    unsafe {
        // elemsize * min_cap wraps to a small number; both must compute the
        // same (wrapped) realloc size and the same resulting header.
        for (elemsize, addlen, min_cap) in [
            (1usize << 62, 0usize, 4usize),  // 2^62*4 == 0 (mod 2^64)
            (1usize << 63, 0, 4),            // 2^63*4 == 0
            (usize::MAX, 0, 4),              // MAX*4 == -4
            (1usize << 61, 0, 8),            // 2^61*8 == 0
        ] {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) as *mut u8;
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) as *mut u8;
            assert_eq!(a.is_null(), b.is_null(), "err46: elemsize={elemsize:#x}");
            if !a.is_null() {
                let ha = *header(a);
                let hb = *header(b);
                assert_eq!(
                    (ha.length, ha.capacity, ha.temp, ha.hash_table.is_null()),
                    (hb.length, hb.capacity, hb.temp, hb.hash_table.is_null()),
                    "err46: elemsize={elemsize:#x} min_cap={min_cap}"
                );
                (p.c.arrfreef)(a as *mut c_void);
                (p.r.arrfreef)(b as *mut c_void);
            }
        }
        // huge but non-wrapping request: realloc fails -> both dereference
        // (header write) the same way. Not exercised: it is a guaranteed crash
        // in the C original, identical in both.
    }
}

// ===========================================================================
// ERRORS rows 48 -- out-of-range values for the stbds_shmode_func enum
// ===========================================================================

#[test]
fn err48_shmode_out_of_range_enum() {
    let p = libs();
    for m in [
        -1i32,
        -2,
        -255,
        -256,
        -257,
        4,
        5,
        100,
        255,
        256,
        257,
        512,
        65535,
        65536,
        i32::MAX,
        i32::MIN,
    ] {
        for elemsize in [8usize, 16, 32] {
            reset_seeds(p, 0x3141_5926);
            unsafe {
                let a = (p.c.shmode_func)(elemsize, m as c_int) as *mut u8;
                let b = (p.r.shmode_func)(elemsize, m as c_int) as *mut u8;
                assert_eq!(
                    snap_map(a, elemsize, KeyRepr::Raw),
                    snap_map(b, elemsize, KeyRepr::Raw),
                    "err48: mode={m} elemsize={elemsize}"
                );
                assert_eq!(
                    (*hm_table(a, elemsize)).string.mode,
                    (m as u32 & 0xff) as u8,
                    "err48: (unsigned char) truncation for {m}"
                );
                // free the table + array directly: for STRDUP-truncated modes
                // hmfree_func would walk key pointers that were never stored.
                (p.c.hmfree_func)(a.sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)(b.sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// ERRORS row 49 -- keyoffset one step past the element
// ===========================================================================

#[test]
fn err49_keyoffset_past_element() {
    let p = libs();
    const ES: usize = 16;
    for keyoffset in [ES, ES + 4, 2 * ES] {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(49);
        let mut c_m = Map::new(&p.c, ES, 4, 8);
        let mut r_m = Map::new(&p.r, ES, 4, 8);
        unsafe {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..20u32 {
                let k = (i * 3 + 1).to_le_bytes().to_vec();
                let v = rng.bytes(8);
                c_m.hmput(&k, &v, STBDS_HM_BINARY);
                r_m.hmput(&k, &v, STBDS_HM_BINARY);
                keys.push(k);
            }
            // grow the capacity so reading at elem+keyoffset stays inside the
            // allocation on both sides (the C code does not bounds-check it)
            for i in 20..40u32 {
                let k = (i * 3 + 1).to_le_bytes().to_vec();
                let v = rng.bytes(8);
                c_m.hmput(&k, &v, STBDS_HM_BINARY);
                r_m.hmput(&k, &v, STBDS_HM_BINARY);
                keys.push(k);
            }
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "err49: filled");
            // the shifted comparison never matches, so every delete misses
            for k in &keys {
                let dc = c_m.hmdel(k, STBDS_HM_BINARY, keyoffset);
                let dr = r_m.hmdel(k, STBDS_HM_BINARY, keyoffset);
                assert_eq!(dc, dr, "err49: keyoffset={keyoffset}");
                assert_eq!(
                    c_m.snap(KeyRepr::Raw),
                    r_m.snap(KeyRepr::Raw),
                    "err49: keyoffset={keyoffset}"
                );
            }
            c_m.free();
            r_m.free();
        }
    }
}

// ===========================================================================
// ERRORS row 50 -- arena block at the extremes (255 etc.)
// ===========================================================================

#[test]
fn err50_arena_block_extremes() {
    let p = libs();
    for block in [0u8, 1, 2, 21, 22, 23, 30, 31, 110, 111, 128, 129, 254, 255] {
        let k = ((block >> 1) as u32) & 63;
        let blocksize = 512usize.wrapping_shl(k);
        if blocksize > (1 << 24) {
            continue;
        }
        for len in [0usize, 1, 511, 512, 513] {
            let mut ac = Arena { block, ..Default::default() };
            let mut ar = Arena { block, ..Default::default() };
            let mut s = vec![b'e'; len];
            s.push(0);
            let mut s2 = s.clone();
            unsafe {
                let rc = (p.c.stralloc)(
                    &mut ac as *mut Arena as *mut c_void,
                    s.as_mut_ptr() as *mut c_char,
                );
                let rr = (p.r.stralloc)(
                    &mut ar as *mut Arena as *mut c_void,
                    s2.as_mut_ptr() as *mut c_char,
                );
                assert_eq!(cstr(rc), cstr(rr), "err50: block={block} len={len}");
                assert_eq!(ac.norm(), ar.norm(), "err50: block={block} len={len}");
                (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
                (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
                assert_eq!(ac, ar);
            }
        }
    }
}
