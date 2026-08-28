//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH the C and
//! the Rust `.so` through their exports, and asserts the same
//! rejection/sentinel comes back (not merely "both failed").

mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};

/// `hmdel_key` unconditionally writes `stbds_temp(raw_a) = 0` before it even
/// looks the key up, so "nothing changed" comparisons must ignore `temp`.
fn without_temp(mut s: FullSnap) -> FullSnap {
    s.hdr.temp = 0;
    s
}

// ===========================================================================
// stbds_arrgrowf
// ===========================================================================

/// E1 — `min_cap <= arrcap(a)` -> unchanged pointer, untouched header.
#[test]
fn err_e1_arrgrowf_noop() {
    let p = libs();
    // a == NULL, addlen == 0, min_cap == 0  ->  returns NULL
    for elemsize in [0usize, 1, 4, 8, 20, 1024] {
        let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        assert!(c.is_null(), "C: arrgrowf(NULL,{elemsize},0,0) must return NULL");
        assert!(r.is_null(), "RUST: arrgrowf(NULL,{elemsize},0,0) must return NULL");
    }
    // live array, request no more than the current capacity
    for elemsize in [1usize, 4, 8] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        ca.grow(0, 10);
        ra.grow(0, 10);
        let (cp, rp) = (ca.a, ra.a);
        let (cs, rs) = (ca.snap(), ra.snap());
        for min_cap in [0usize, 1, 9, 10] {
            for addlen in [0usize, 1, 5, 10] {
                if addlen > 10 {
                    continue;
                }
                ca.grow(addlen, min_cap);
                ra.grow(addlen, min_cap);
                assert_eq!(ca.a, cp, "C must not realloc (e={elemsize})");
                assert_eq!(ra.a, rp, "RUST must not realloc (e={elemsize})");
                diff_eq!(ca.snap(), ra.snap(), "e={elemsize} noop({addlen},{min_cap})");
                assert_eq!(ca.snap(), cs);
                assert_eq!(ra.snap(), rs);
            }
        }
        ca.free();
        ra.free();
    }
}

/// E2 — fresh allocation initialises length/hash_table/temp and clamps cap to 4.
#[test]
fn err_e2_arrgrowf_fresh() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        for (addlen, min_cap, want_cap) in [
            (0usize, 1usize, 4usize),
            (0, 2, 4),
            (0, 3, 4),
            (0, 4, 4),
            (0, 5, 5),
            (1, 0, 4),
            (3, 0, 4),
            (4, 0, 4),
            (5, 0, 5),
            (7, 3, 7),
            (2, 9, 9),
        ] {
            let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
            let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
            let cs = unsafe { snap_hdr(c) };
            let rs = unsafe { snap_hdr(r) };
            diff_eq!(cs.clone(), rs, "arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
            assert_eq!(cs.length, 0);
            assert_eq!(cs.temp, 0);
            assert!(!cs.has_table);
            assert_eq!(cs.capacity, want_cap, "cap for ({addlen},{min_cap})");
            unsafe {
                (p.c.arrfreef)(c);
                (p.r.arrfreef)(r);
            }
        }
    }
}

/// E3 — elemsize == 0 still allocates and sets capacity.
#[test]
fn err_e3_arrgrowf_elemsize0() {
    let p = libs();
    for min_cap in [1usize, 4, 5, 100, 1_000_000] {
        let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap) };
        let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap) };
        let cs = unsafe { snap_hdr(c) };
        let rs = unsafe { snap_hdr(r) };
        diff_eq!(cs.clone(), rs, "arrgrowf(NULL,0,0,{min_cap})");
        assert_eq!(cs.capacity, min_cap.max(4));
        // and a second grow on the zero-elemsize array
        let c2 = unsafe { (p.c.arrgrowf)(c, 0, 7, 0) };
        let r2 = unsafe { (p.r.arrgrowf)(r, 0, 7, 0) };
        diff_eq!(
            unsafe { snap_hdr(c2) },
            unsafe { snap_hdr(r2) },
            "second grow, e=0, min_cap={min_cap}"
        );
        unsafe {
            (p.c.arrfreef)(c2);
            (p.r.arrfreef)(r2);
        }
    }
}

/// E4 — `size_t` wrap-around in `min_len` and in `elemsize*min_cap + 32`.
#[test]
fn err_e4_arrgrowf_wrap() {
    let p = libs();

    // (a) min_len = arrlen + addlen wraps to 0 -> the `min_cap <= arrcap` early
    //     return fires and nothing happens.
    for elemsize in [1usize, 4, 8] {
        let mut ca = Arr::new(&p.c, elemsize);
        let mut ra = Arr::new(&p.r, elemsize);
        ca.setlen(5);
        ra.setlen(5);
        // `arrsetlen` exposes uninitialised bytes in BOTH libraries; write them
        // so the payload comparison below is meaningful.
        for i in 0..5 {
            let v: Vec<u8> = (0..elemsize).map(|k| (i * 7 + k) as u8).collect();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    (ca.a as *mut u8).add(i * elemsize),
                    elemsize,
                );
                core::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    (ra.a as *mut u8).add(i * elemsize),
                    elemsize,
                );
            }
        }
        let (cp, rp) = (ca.a, ra.a);
        let addlen = usize::MAX - 5 + 1; // arrlen(5) + addlen == 0
        ca.grow(addlen, 0);
        ra.grow(addlen, 0);
        assert_eq!(ca.a, cp, "C: wrapped min_len must early-return");
        assert_eq!(ra.a, rp, "RUST: wrapped min_len must early-return");
        diff_eq!(ca.snap(), ra.snap(), "e={elemsize} wrapped min_len");
        ca.free();
        ra.free();
    }

    // (b) `elemsize * min_cap + sizeof(header)` wraps to a small, allocatable
    //     size.  16 * (2^60 + 30) == 2^64 + 480  ->  480; + 32  ->  512 bytes.
    let min_cap = (1usize << 60) + 30;
    let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 16, 0, min_cap) };
    let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 16, 0, min_cap) };
    let cs = unsafe { snap_hdr(c) };
    let rs = unsafe { snap_hdr(r) };
    diff_eq!(cs.clone(), rs, "wrapped byte size");
    assert_eq!(cs.capacity, min_cap, "capacity is the wrapped-in min_cap");
    assert_eq!(cs.length, 0);
    unsafe {
        (p.c.arrfreef)(c);
        (p.r.arrfreef)(r);
    }

    // (c) addlen == SIZE_MAX from NULL: 1 * SIZE_MAX + 32 wraps to 31.
    let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0) };
    let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 1, usize::MAX, 0) };
    diff_eq!(
        unsafe { snap_hdr(c) },
        unsafe { snap_hdr(r) },
        "addlen == SIZE_MAX"
    );
    assert_eq!(unsafe { snap_hdr(c) }.capacity, usize::MAX);
    unsafe {
        (p.c.arrfreef)(c);
        (p.r.arrfreef)(r);
    }
}

// ===========================================================================
// stbds_hmfree_func
// ===========================================================================

/// E5 — `a == NULL` is a no-op.
#[test]
fn err_e5_hmfree_null() {
    let p = libs();
    for elemsize in [0usize, 1, 4, 8, 20, usize::MAX] {
        unsafe {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

/// E6 — non-NULL array whose `hash_table` is NULL.
#[test]
fn err_e6_hmfree_no_table() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        // array built by a *get* on a NULL map -> length 1, hash_table == NULL
        let key = vec![0u8; 8];
        let mut ct: isize = 0;
        let mut rt: isize = 0;
        let ch = unsafe {
            (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                4,
                &mut ct,
                STBDS_HM_BINARY,
            )
        };
        let rh = unsafe {
            (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                4,
                &mut rt,
                STBDS_HM_BINARY,
            )
        };
        diff_eq!(ct, rt, "e={elemsize} bootstrap temp");
        let craw = (ch as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let rraw = (rh as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        diff_eq!(
            unsafe { snap_hdr(craw) },
            unsafe { snap_hdr(rraw) },
            "e={elemsize} bootstrapped header"
        );
        assert!(!unsafe { snap_hdr(craw) }.has_table);
        unsafe {
            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
}

// ===========================================================================
// stbds_hmget_key_ts / stbds_hmget_key
// ===========================================================================

/// E7 — `a == NULL`: bootstraps the default slot and reports `-1`.
#[test]
fn err_e7_hmget_ts_null() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 20] {
        for mode in [-2147483648i32, -1, 0, 1, 2, 3, 255, 256, 999, 2147483647] {
            // Never dereferences the key on this path, so any mode is safe.
            let key = vec![0x7Fu8; 32];
            let mut ct: isize = 0x1234;
            let mut rt: isize = 0x1234;
            let ch = unsafe {
                (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    8,
                    &mut ct,
                    mode as c_int,
                )
            };
            let rh = unsafe {
                (p.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_ptr() as *mut c_void,
                    8,
                    &mut rt,
                    mode as c_int,
                )
            };
            diff_eq!(ct, rt, "e={elemsize} mode={mode} bootstrap *temp");
            assert_eq!(ct, STBDS_INDEX_EMPTY, "must report STBDS_INDEX_EMPTY");
            assert!(!ch.is_null() && !rh.is_null());
            let craw = (ch as *mut u8).wrapping_sub(elemsize) as *mut c_void;
            let rraw = (rh as *mut u8).wrapping_sub(elemsize) as *mut c_void;
            let cs = unsafe { snap_hdr(craw) };
            diff_eq!(cs.clone(), unsafe { snap_hdr(rraw) }, "e={elemsize} mode={mode} hdr");
            assert_eq!(cs.length, 1);
            assert_eq!(cs.capacity, 4);
            assert!(!cs.has_table);
            // the bootstrapped element is memset to 0
            let cb = unsafe { core::slice::from_raw_parts(ch as *const u8, 0) };
            let _ = cb;
            let c0 = unsafe { core::slice::from_raw_parts(craw as *const u8, elemsize).to_vec() };
            let r0 = unsafe { core::slice::from_raw_parts(rraw as *const u8, elemsize).to_vec() };
            diff_eq!(c0.clone(), r0, "e={elemsize} default slot zeroed");
            assert!(c0.iter().all(|&b| b == 0));
            unsafe {
                (p.c.hmfree_func)(craw, elemsize);
                (p.r.hmfree_func)(rraw, elemsize);
            }
        }
    }
}

/// E8 — `hash_table == 0`: `*temp = -1`, header `temp` untouched.
#[test]
fn err_e8_hmget_ts_no_table() {
    let p = libs();
    let elemsize = 8usize;
    let key = vec![9u8; 8];
    let mut ct: isize = 0;
    let mut rt: isize = 0;
    let ch = unsafe {
        (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_ptr() as *mut c_void,
            4,
            &mut ct,
            STBDS_HM_BINARY,
        )
    };
    let rh = unsafe {
        (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_ptr() as *mut c_void,
            4,
            &mut rt,
            STBDS_HM_BINARY,
        )
    };
    let craw = (ch as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let rraw = (rh as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    // poison header->temp so we can prove _ts leaves it alone
    unsafe {
        (*hdr_of(craw)).temp = 0x7EE7;
        (*hdr_of(rraw)).temp = 0x7EE7;
    }
    for mode in [-1i32, 0, 1, 2, 999] {
        let mut ct2: isize = 0xABC;
        let mut rt2: isize = 0xABC;
        let ch2 = unsafe {
            (p.c.hmget_key_ts)(ch, elemsize, key.as_ptr() as *mut c_void, 4, &mut ct2, mode)
        };
        let rh2 = unsafe {
            (p.r.hmget_key_ts)(rh, elemsize, key.as_ptr() as *mut c_void, 4, &mut rt2, mode)
        };
        diff_eq!(ct2, rt2, "mode={mode} table-less *temp");
        assert_eq!(ct2, -1);
        assert_eq!(ch2, ch, "must return `a` unchanged");
        assert_eq!(rh2, rh, "must return `a` unchanged");
        diff_eq!(
            unsafe { snap_hdr(craw) },
            unsafe { snap_hdr(rraw) },
            "mode={mode} table-less hdr"
        );
        assert_eq!(unsafe { (*hdr_of(craw)).temp }, 0x7EE7, "temp untouched");
        assert_eq!(unsafe { (*hdr_of(rraw)).temp }, 0x7EE7, "temp untouched");
    }
    unsafe {
        (p.c.hmfree_func)(craw, elemsize);
        (p.r.hmfree_func)(rraw, elemsize);
    }
}

/// E9 / E10 / E11 — key-not-found returns the `-1` sentinel from both
/// half-scans of the probe loop.
#[test]
fn err_e9_hmget_ts_missing() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    let mut rng = Rng::new(9);
    for load in [1usize, 3, 5, 6, 11, 12, 23, 24, 47, 100] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..load as i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        for _ in 0..500 {
            let k = (rng.next_u32() | 0x8000_0000) as i32; // guaranteed absent
            let a = cm.hmgeti_ts(k.to_ne_bytes().as_ptr() as *mut c_void);
            let b = rm.hmgeti_ts(k.to_ne_bytes().as_ptr() as *mut c_void);
            diff_eq!(a, b, "load={load} miss *temp");
            assert_eq!(a, STBDS_INDEX_EMPTY, "miss must be -1");
            let a2 = cm.hmgeti(&k.to_ne_bytes());
            let b2 = rm.hmgeti(&k.to_ne_bytes());
            diff_eq!(a2, b2, "load={load} miss header temp");
            assert_eq!(a2, STBDS_INDEX_EMPTY);
        }
        diff_eq!(cm.snap(), rm.snap(), "load={load} after misses");
        cm.hmfree();
        rm.hmfree();
    }
}

/// E10 / E11 — probes that must traverse the wrap-around half-scan.  Tables are
/// deliberately loaded to just under `used_count_threshold` so most buckets are
/// dense and the probe passes the bucket boundary.
#[test]
fn err_e10_find_slot_wrap() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    let mut rng = Rng::new(10);
    // slot_count 8 with 6 used, 16 with 12 used, 32 with 24 used, ...
    for load in [6usize, 12, 24, 48, 96] {
        for seed in [DEFAULT_SEED, 0usize, 1, usize::MAX] {
            reset_seed(&p, seed);
            let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
            let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
            for i in 0..load as i32 {
                cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
                rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            }
            diff_eq!(cm.snap(), rm.snap(), "load={load} seed={seed:#x} filled");
            for _ in 0..800 {
                let k = (rng.next_u32() as i32).wrapping_add(1_000_000);
                let a = cm.hmgeti(&k.to_ne_bytes());
                let b = rm.hmgeti(&k.to_ne_bytes());
                diff_eq!(a, b, "load={load} seed={seed:#x} wrap miss");
                assert_eq!(a, -1);
            }
            // deletes of absent keys go through the same probe path
            for _ in 0..200 {
                let k = (rng.next_u32() as i32).wrapping_add(2_000_000);
                let a = cm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
                let b = rm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
                diff_eq!(a, b, "load={load} wrap del miss");
                assert_eq!(a, 0);
            }
            diff_eq!(cm.snap(), rm.snap(), "load={load} seed={seed:#x} final");
            cm.hmfree();
            rm.hmfree();
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E12 — `hmget_key` additionally writes `header->temp`.
#[test]
fn err_e12_hmget_key_missing() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    // fresh from NULL: hmget_key must write temp = -1 into the new header
    let key = 42i32.to_ne_bytes();
    let a = cm.hmgeti(&key);
    let b = rm.hmgeti(&key);
    diff_eq!(a, b, "hmget_key on NULL");
    assert_eq!(a, -1);
    diff_eq!(cm.snap(), rm.snap(), "hmget_key on NULL state");
    for i in 0..30i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
    }
    for i in [-1i32, 30, 31, 1000, i32::MIN, i32::MAX] {
        let a = cm.hmgeti(&i.to_ne_bytes());
        let b = rm.hmgeti(&i.to_ne_bytes());
        diff_eq!(a, b, "hmget_key miss {i}");
        assert_eq!(a, -1);
        assert_eq!(cm.temp(), -1, "header->temp must be -1");
        assert_eq!(rm.temp(), -1, "header->temp must be -1");
        diff_eq!(cm.snap(), rm.snap(), "hmget_key miss {i} state");
    }
    cm.hmfree();
    rm.hmfree();
}

// ===========================================================================
// stbds_hmput_default
// ===========================================================================

/// E13 — `a == NULL`.
#[test]
fn err_e13_hmput_default_null() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 20, 64] {
        let c = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let r = unsafe { (p.r.hmput_default)(std::ptr::null_mut(), elemsize) };
        assert!(!c.is_null() && !r.is_null());
        let craw = (c as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let rraw = (r as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let cs = unsafe { snap_hdr(craw) };
        diff_eq!(cs.clone(), unsafe { snap_hdr(rraw) }, "hmput_default(NULL,{elemsize})");
        assert_eq!(cs.length, 1);
        assert_eq!(cs.capacity, 4);
        assert!(!cs.has_table);
        let c0 = unsafe { core::slice::from_raw_parts(craw as *const u8, elemsize).to_vec() };
        let r0 = unsafe { core::slice::from_raw_parts(rraw as *const u8, elemsize).to_vec() };
        diff_eq!(c0.clone(), r0, "default slot zeroed e={elemsize}");
        assert!(c0.iter().all(|&b| b == 0));
        unsafe {
            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
}

/// E14 — `a != NULL` but `length == 0`.
#[test]
fn err_e14_hmput_default_len0() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 20] {
        // build a raw array (length 0) and hand its "hash pointer" to hmput_default
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let ra = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        assert_eq!(unsafe { snap_hdr(ca) }.length, 0);
        let ch = (ca as *mut u8).wrapping_add(elemsize) as *mut c_void;
        let rh = (ra as *mut u8).wrapping_add(elemsize) as *mut c_void;
        let c = unsafe { (p.c.hmput_default)(ch, elemsize) };
        let r = unsafe { (p.r.hmput_default)(rh, elemsize) };
        let craw = (c as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let rraw = (r as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let cs = unsafe { snap_hdr(craw) };
        diff_eq!(cs.clone(), unsafe { snap_hdr(rraw) }, "len0 e={elemsize}");
        assert_eq!(cs.length, 1, "length must be bumped to 1");
        unsafe {
            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
}

/// E15 — `a != NULL` and `length != 0`: pure no-op.
#[test]
fn err_e15_hmput_default_noop() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    for prefill in [0usize, 1, 7, 30] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        cm.hmdefault(&0i32.to_ne_bytes());
        rm.hmdefault(&0i32.to_ne_bytes());
        for i in 0..prefill as i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        let (cbefore, rbefore) = (cm.t, rm.t);
        let cs = cm.snap();
        let rs = rm.snap();
        for _ in 0..5 {
            let c = unsafe { (p.c.hmput_default)(cm.t, spec.elemsize) };
            let r = unsafe { (p.r.hmput_default)(rm.t, spec.elemsize) };
            assert_eq!(c, cbefore, "C: hmput_default must be a no-op");
            assert_eq!(r, rbefore, "RUST: hmput_default must be a no-op");
        }
        diff_eq!(cm.snap(), rm.snap(), "prefill={prefill} noop");
        assert_eq!(cm.snap(), cs);
        assert_eq!(rm.snap(), rs);
        cm.hmfree();
        rm.hmfree();
    }
}

// ===========================================================================
// stbds_hmput_key
// ===========================================================================

/// E16 / E17 — `a == NULL` bootstrap and the first-insert table creation
/// (including `string.mode` selection for every mode value).
#[test]
fn err_e16_hmput_key_null() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 20] {
        let keysize = elemsize.min(8);
        reset_seed(&p, DEFAULT_SEED);
        let key = vec![0x33u8; keysize];
        let c = unsafe {
            (p.c.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                keysize,
                STBDS_HM_BINARY,
            )
        };
        let r = unsafe {
            (p.r.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_ptr() as *mut c_void,
                keysize,
                STBDS_HM_BINARY,
            )
        };
        let craw = (c as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let rraw = (r as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        // `hmput_key` only memcpy's `keysize` key bytes; the rest of the element
        // is uninitialised realloc memory in BOTH libraries until the calling
        // macro assigns `.value`.  Do that now so the payload is comparable.
        if elemsize > keysize {
            let v: Vec<u8> = (0..elemsize - keysize).map(|k| (k as u8) ^ 0xA5).collect();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    (c as *mut u8).add(keysize),
                    v.len(),
                );
                core::ptr::copy_nonoverlapping(
                    v.as_ptr(),
                    (r as *mut u8).add(keysize),
                    v.len(),
                );
            }
        }
        let cs = unsafe { snap_full(craw, Spec::bytes(elemsize, keysize)) };
        diff_eq!(
            cs.clone(),
            unsafe { snap_full(rraw, Spec::bytes(elemsize, keysize)) },
            "hmput_key(NULL,{elemsize})"
        );
        assert_eq!(cs.hdr.length, 2, "default slot + 1 element");
        assert!(cs.hdr.has_table);
        assert_eq!(cs.idx.slot_count, 8);
        assert_eq!(cs.idx.used_count, 1);
        assert_eq!(cs.idx.arena.mode, 0, "BINARY -> string.mode 0");
        unsafe {
            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
}

#[test]
fn err_e17_hmput_key_first() {
    let p = libs();
    let elemsize = 16usize;
    // string.mode is `mode >= STBDS_HM_STRING ? SH_DEFAULT : 0`
    for (mode, want_string_mode) in [
        (i32::MIN, 0u8),
        (-1000, 0),
        (-1, 0),
        (0, 0),
        (1, 1),
        (2, 1),
        (3, 1),
        (255, 1),
        (256, 1),
        (999, 1),
        (i32::MAX, 1),
    ] {
        reset_seed(&p, DEFAULT_SEED);
        let mut keys = Keys::new();
        let k = keys.add(b"a-first-key-for-mode-selection");
        let (c, r) = if mode >= STBDS_HM_STRING {
            unsafe {
                (
                    (p.c.hmput_key)(std::ptr::null_mut(), elemsize, k as *mut c_void, 8, mode),
                    (p.r.hmput_key)(std::ptr::null_mut(), elemsize, k as *mut c_void, 8, mode),
                )
            }
        } else {
            let kb = vec![0x5Cu8; 8];
            unsafe {
                (
                    (p.c.hmput_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        kb.as_ptr() as *mut c_void,
                        8,
                        mode,
                    ),
                    (p.r.hmput_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        kb.as_ptr() as *mut c_void,
                        8,
                        mode,
                    ),
                )
            }
        };
        let craw = (c as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        let rraw = (r as *mut u8).wrapping_sub(elemsize) as *mut c_void;
        // initialise the value half (uninitialised in both libs until the macro
        // assigns `.value`) so the element bytes are comparable
        let v: Vec<u8> = (0..elemsize - 8).map(|k| (k as u8) ^ 0x3C).collect();
        unsafe {
            core::ptr::copy_nonoverlapping(v.as_ptr(), (c as *mut u8).add(8), v.len());
            core::ptr::copy_nonoverlapping(v.as_ptr(), (r as *mut u8).add(8), v.len());
        }
        let spec = Spec::bytes(elemsize, 8);
        let cs = unsafe { snap_full(craw, spec) };
        diff_eq!(cs.clone(), unsafe { snap_full(rraw, spec) }, "mode={mode} first put");
        assert_eq!(
            cs.idx.arena.mode, want_string_mode,
            "mode={mode} -> string.mode"
        );
        unsafe {
            (p.c.hmfree_func)(craw, elemsize);
            (p.r.hmfree_func)(rraw, elemsize);
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E18 — load-factor growth doubles `slot_count` and carries the seed/arena over.
#[test]
fn err_e18_hmput_key_grow() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    let mut seen_counts = Vec::new();
    let mut first_seed = None;
    for i in 0..200i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        let cs = cm.snap();
        diff_eq!(cs.clone(), rm.snap(), "grow put {i}");
        if first_seed.is_none() {
            first_seed = Some(cs.idx.seed);
        }
        assert_eq!(cs.idx.seed, first_seed.unwrap(), "seed must survive rehashing");
        if seen_counts.last() != Some(&cs.idx.slot_count) {
            seen_counts.push(cs.idx.slot_count);
        }
        // threshold invariants
        assert_eq!(
            cs.idx.used_count_threshold,
            cs.idx.slot_count - (cs.idx.slot_count >> 2)
        );
        assert!(cs.idx.used_count < cs.idx.used_count_threshold + 1);
    }
    // 200 inserts: growth fires when used_count reaches slot_count - slot_count/4,
    // i.e. at 6, 12, 24, 48, 96 and 192 -> the last table is 512 slots.
    assert_eq!(seen_counts, vec![8usize, 16, 32, 64, 128, 256, 512]);
    cm.hmfree();
    rm.hmfree();
}

/// E19 / E20 — duplicate-key hits in each half-scan.
#[test]
fn err_e19_hmput_dup() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    for load in [1usize, 6, 12, 24, 100] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..load as i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        let before_len = cm.snap().hdr.length;
        for round in 0..3 {
            for i in 0..load as i32 {
                let a = cm.hmput(&i.to_ne_bytes(), &(i * 100 + round).to_ne_bytes());
                let b = rm.hmput(&i.to_ne_bytes(), &(i * 100 + round).to_ne_bytes());
                diff_eq!(a, b, "load={load} dup put {i}");
                assert_eq!(a, i as isize, "dup must reuse the slot");
                diff_eq!(cm.snap(), rm.snap(), "load={load} dup state {i}");
            }
        }
        assert_eq!(cm.snap().hdr.length, before_len, "length must not grow");
        cm.hmfree();
        rm.hmfree();
    }
}

#[test]
fn err_e20_hmput_dup_wrap() {
    let p = libs();
    // 500 string keys with SH_DEFAULT so `temp_key` is written (or, in the
    // wrap-around half-scan, deliberately NOT written -- the E20 quirk).
    let spec = Spec::bytes(16, 8);
    let mut rng = Rng::new(20);
    let mut keys = Keys::new();
    let mut pool = Vec::new();
    for _ in 0..500 {
        let n = rng.range(1, 24);
        let b = rng.cstr_body(n, false);
        pool.push(keys.add(&b));
    }
    for seed in [DEFAULT_SEED, 0usize, usize::MAX] {
        reset_seed(&p, seed);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_STRING);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_STRING);
        for (i, &k) in pool.iter().enumerate() {
            cm.shput(k, &(i as u64).to_ne_bytes());
            rm.shput(k, &(i as u64).to_ne_bytes());
        }
        for round in 0..3u64 {
            for (i, &k) in pool.iter().enumerate() {
                let a = cm.shput(k, &(round * 1000 + i as u64).to_ne_bytes());
                let b = rm.shput(k, &(round * 1000 + i as u64).to_ne_bytes());
                diff_eq!(a, b, "seed={seed:#x} dup shput {i}");
                diff_eq!(cm.snap(), rm.snap(), "seed={seed:#x} dup shput state {i}");
                // temp_key must agree exactly (including when C leaves it stale)
                let ct = unsafe { raw_temp_key(cm.raw()) };
                let rt = unsafe { raw_temp_key(rm.raw()) };
                assert_eq!(ct, rt, "seed={seed:#x} temp_key pointer must match at {i}");
            }
        }
        cm.hmfree();
        rm.hmfree();
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E21 — a tombstone seen before the empty slot is reused (`tombstone_count--`).
#[test]
fn err_e21_hmput_reuse_tombstone() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    let mut rng = Rng::new(21);
    let mut reuse_seen = 0usize;
    for seed in [DEFAULT_SEED, 0usize, 1, 7, usize::MAX] {
        reset_seed(&p, seed);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..90i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        for step in 0..1500i32 {
            let k = (rng.next_u32() % 400) as i32;
            let before = cm.snap().idx.tombstone_count;
            if rng.below(2) == 0 {
                let a = cm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
                let b = rm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
                diff_eq!(a, b, "seed={seed:#x} del {step}");
            } else {
                let a = cm.hmput(&k.to_ne_bytes(), &step.to_ne_bytes());
                let b = rm.hmput(&k.to_ne_bytes(), &step.to_ne_bytes());
                diff_eq!(a, b, "seed={seed:#x} put {step}");
                let after = cm.snap().idx.tombstone_count;
                if after + 1 == before {
                    reuse_seen += 1;
                }
            }
            diff_eq!(cm.snap(), rm.snap(), "seed={seed:#x} step {step}");
        }
        cm.hmfree();
        rm.hmfree();
    }
    assert!(
        reuse_seen > 0,
        "the tombstone-reuse branch was never exercised"
    );
    reset_seed(&p, DEFAULT_SEED);
}

/// E22 / E23 / E39 — out-of-range `mode` values crossing the FFI boundary.
#[test]
fn err_e22_mode_out_of_range() {
    let p = libs();

    // (a) mode >= 1 (2, 3, 255, 256, 999, INT_MAX) behaves exactly like STRING.
    for mode in [1i32, 2, 3, 4, 255, 256, 999, 65_537, i32::MAX] {
        reset_seed(&p, DEFAULT_SEED);
        let spec = Spec::bytes(16, 8);
        let mut rng = Rng::new(22 + mode as u64);
        let mut keys = Keys::new();
        let mut pool = Vec::new();
        for _ in 0..60 {
            let n = rng.range(4, 24);
            pool.push(keys.add(&rng.cstr_body(n, false)));
        }
        let mut cm = Map::new(&p.c, spec, mode);
        let mut rm = Map::new(&p.r, spec, mode);
        for (i, &k) in pool.iter().enumerate() {
            let a = cm.shput(k, &(i as u64).to_ne_bytes());
            let b = rm.shput(k, &(i as u64).to_ne_bytes());
            diff_eq!(a, b, "mode={mode} put {i}");
            diff_eq!(cm.snap(), rm.snap(), "mode={mode} put state {i}");
        }
        assert_eq!(cm.snap().idx.arena.mode, 1, "mode={mode} -> SH_DEFAULT");
        for (i, &k) in pool.iter().enumerate() {
            let a = cm.shgeti(k);
            let b = rm.shgeti(k);
            diff_eq!(a, b, "mode={mode} get {i}");
            assert_eq!(a, i as isize);
        }
        // Deleting in reverse insertion order keeps `old_index == final_index`,
        // which skips the memmove + re-find (see ERRORS.md E34 for why the
        // re-find is UB in the C for mode != 1).
        for (i, &k) in pool.iter().enumerate().rev() {
            let a = cm.hmdel(k as *mut c_void, 0);
            let b = rm.hmdel(k as *mut c_void, 0);
            diff_eq!(a, b, "mode={mode} del {i}");
            assert_eq!(a, 1);
            diff_eq!(cm.snap(), rm.snap(), "mode={mode} del state {i}");
        }
        cm.hmfree();
        rm.hmfree();
    }

    // (b) mode < 1 (0, -1, -1000, INT_MIN) behaves exactly like BINARY.
    for mode in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        reset_seed(&p, DEFAULT_SEED);
        let spec = Spec::bytes(8, 4);
        let mut cm = Map::new(&p.c, spec, mode);
        let mut rm = Map::new(&p.r, spec, mode);
        for i in 0..60i32 {
            let a = cm.hmput(&i.to_ne_bytes(), &(i * 3).to_ne_bytes());
            let b = rm.hmput(&i.to_ne_bytes(), &(i * 3).to_ne_bytes());
            diff_eq!(a, b, "mode={mode} put {i}");
            diff_eq!(cm.snap(), rm.snap(), "mode={mode} put state {i}");
        }
        assert_eq!(cm.snap().idx.arena.mode, 0, "mode={mode} -> string.mode 0");
        for i in 0..70i32 {
            let a = cm.hmgeti(&i.to_ne_bytes());
            let b = rm.hmgeti(&i.to_ne_bytes());
            diff_eq!(a, b, "mode={mode} get {i}");
        }
        for i in 0..60i32 {
            let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
            let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
            diff_eq!(a, b, "mode={mode} del {i}");
            diff_eq!(cm.snap(), rm.snap(), "mode={mode} del state {i}");
        }
        cm.hmfree();
        rm.hmfree();
    }

    // (c) out-of-range modes on paths that never hash/deref the key at all.
    for mode in [i32::MIN, -1, 0, 1, 2, 999, i32::MAX] {
        let e = 8usize;
        let key = vec![0u8; 8];
        // hmdel on NULL
        let c = unsafe {
            (p.c.hmdel_key)(
                std::ptr::null_mut(),
                e,
                key.as_ptr() as *mut c_void,
                4,
                0,
                mode,
            )
        };
        let r = unsafe {
            (p.r.hmdel_key)(
                std::ptr::null_mut(),
                e,
                key.as_ptr() as *mut c_void,
                4,
                0,
                mode,
            )
        };
        assert!(c.is_null() && r.is_null(), "mode={mode} hmdel(NULL) -> NULL");
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E24 / E29 / E30 / E31 / E32 / E38 — none of the live `assert()`s in the C
/// build may fire under heavy legitimate load (the C `.so` imports
/// `__assert_fail`, so a violation would abort this process).
#[test]
fn err_e24_no_abort_under_load() {
    let p = libs();
    let mut rng = Rng::new(24);
    for (elemsize, keysize) in [(8usize, 4usize), (16, 8), (20, 8), (4, 4), (1, 1)] {
        let spec = Spec::bytes(elemsize, keysize);
        for seed in [DEFAULT_SEED, 0usize, usize::MAX] {
            reset_seed(&p, seed);
            let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
            let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
            for step in 0..4000usize {
                let k = rng.bytes(keysize);
                let v = rng.bytes(elemsize - keysize);
                match rng.below(10) {
                    0..=5 => {
                        let a = cm.hmput(&k, &v);
                        let b = rm.hmput(&k, &v);
                        diff_eq!(a, b, "e={elemsize} put {step}");
                    }
                    6..=8 => {
                        let a = cm.hmdel(k.as_ptr() as *mut c_void, 0);
                        let b = rm.hmdel(k.as_ptr() as *mut c_void, 0);
                        diff_eq!(a, b, "e={elemsize} del {step}");
                    }
                    _ => {
                        let a = cm.hmgeti(&k);
                        let b = rm.hmgeti(&k);
                        diff_eq!(a, b, "e={elemsize} get {step}");
                    }
                }
                if step % 97 == 0 {
                    diff_eq!(cm.snap(), rm.snap(), "e={elemsize} load {step}");
                    let s = cm.snap();
                    // the make_hash_index assert (E38); only meaningful once a
                    // table exists (a get/del-first map has hash_table == NULL)
                    if s.idx.present {
                        assert!(
                            s.idx.used_count_threshold + s.idx.tombstone_count_threshold
                                < s.idx.slot_count,
                            "E38 invariant broken: {:?}",
                            s.idx
                        );
                    }
                }
            }
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} seed={seed:#x} final");
            cm.hmfree();
            rm.hmfree();
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ===========================================================================
// stbds_hmdel_key
// ===========================================================================

/// E25 — `a == NULL` returns `NULL` (the `hmdel` macro then yields 0).
#[test]
fn err_e25_hmdel_null() {
    let p = libs();
    for elemsize in [0usize, 1, 4, 8, 20] {
        for keysize in [0usize, 1, 4, 8] {
            for keyoffset in [0usize, 4, 8, 1000] {
                for mode in [i32::MIN, -1, 0, 1, 2, 999, i32::MAX] {
                    let key = vec![0u8; 16];
                    let c = unsafe {
                        (p.c.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        )
                    };
                    let r = unsafe {
                        (p.r.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        )
                    };
                    assert!(c.is_null(), "C hmdel_key(NULL,...) must be NULL");
                    assert!(r.is_null(), "RUST hmdel_key(NULL,...) must be NULL");
                }
            }
        }
    }
}

/// E26 — `hash_table == 0`: `temp = 0`, `a` returned unchanged.
#[test]
fn err_e26_hmdel_no_table() {
    let p = libs();
    let elemsize = 8usize;
    let key = vec![7u8; 8];
    let mut ct = 0isize;
    let mut rt = 0isize;
    let ch = unsafe {
        (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_ptr() as *mut c_void,
            4,
            &mut ct,
            STBDS_HM_BINARY,
        )
    };
    let rh = unsafe {
        (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            elemsize,
            key.as_ptr() as *mut c_void,
            4,
            &mut rt,
            STBDS_HM_BINARY,
        )
    };
    let craw = (ch as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    let rraw = (rh as *mut u8).wrapping_sub(elemsize) as *mut c_void;
    unsafe {
        (*hdr_of(craw)).temp = 0x1234;
        (*hdr_of(rraw)).temp = 0x1234;
    }
    for mode in [i32::MIN, -1, 0, 1, 2, 999, i32::MAX] {
        let c = unsafe { (p.c.hmdel_key)(ch, elemsize, key.as_ptr() as *mut c_void, 4, 0, mode) };
        let r = unsafe { (p.r.hmdel_key)(rh, elemsize, key.as_ptr() as *mut c_void, 4, 0, mode) };
        assert_eq!(c, ch, "must return `a` unchanged");
        assert_eq!(r, rh, "must return `a` unchanged");
        let cs = unsafe { snap_hdr(craw) };
        diff_eq!(cs.clone(), unsafe { snap_hdr(rraw) }, "mode={mode} no-table del");
        assert_eq!(cs.temp, 0, "temp must be reset to 0");
        assert_eq!(cs.length, 1, "nothing removed");
    }
    unsafe {
        (p.c.hmfree_func)(craw, elemsize);
        (p.r.hmfree_func)(rraw, elemsize);
    }
}

/// E27 — key absent: `temp = 0`, nothing changes.
#[test]
fn err_e27_hmdel_missing() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    for load in [1usize, 6, 12, 40, 200] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..load as i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        let cs = cm.snap();
        let rs = rm.snap();
        for k in [-1i32, -100, load as i32, load as i32 + 1, 999_999, i32::MIN, i32::MAX] {
            let a = cm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
            let b = rm.hmdel(k.to_ne_bytes().as_ptr() as *mut c_void, 0);
            diff_eq!(a, b, "load={load} del missing {k}");
            assert_eq!(a, 0, "missing delete must report 0");
            diff_eq!(cm.snap(), rm.snap(), "load={load} del missing {k} state");
            // `hmdel_key` always writes temp = 0 first, so compare without it
            assert_eq!(without_temp(cm.snap()), without_temp(cs.clone()), "untouched");
            assert_eq!(without_temp(rm.snap()), without_temp(rs.clone()), "untouched");
            assert_eq!(cm.snap().hdr.temp, 0, "temp must be 0 after a missed delete");
            assert_eq!(rm.snap().hdr.temp, 0, "temp must be 0 after a missed delete");
        }
        cm.hmfree();
        rm.hmfree();
    }
}

/// E28 — a hit sets `temp = 1`, tombstones the slot and shrinks `length`.
#[test]
fn err_e28_hmdel_hit() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    for i in 0..10i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
    }
    let before = cm.snap();
    let a = cm.hmdel(3i32.to_ne_bytes().as_ptr() as *mut c_void, 0);
    let b = rm.hmdel(3i32.to_ne_bytes().as_ptr() as *mut c_void, 0);
    diff_eq!(a, b, "hit del result");
    assert_eq!(a, 1, "hit must report 1");
    let after = cm.snap();
    diff_eq!(after.clone(), rm.snap(), "hit del state");
    assert_eq!(after.hdr.length, before.hdr.length - 1);
    assert_eq!(after.idx.used_count, before.idx.used_count - 1);
    assert_eq!(after.idx.tombstone_count, before.idx.tombstone_count + 1);
    let tomb: Vec<(usize, isize)> = after
        .idx
        .slots
        .iter()
        .copied()
        .filter(|&(h, i)| h == STBDS_HASH_DELETED && i == STBDS_INDEX_DELETED)
        .collect();
    assert_eq!(
        tomb.len(),
        1,
        "exactly one (HASH_DELETED, INDEX_DELETED) slot: {:?}",
        after.idx.slots
    );
    cm.hmfree();
    rm.hmfree();
}

/// E33 — deleting the physically-last element skips the memmove + re-find.
#[test]
fn err_e33_hmdel_last() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    for load in [1usize, 2, 7, 13, 50] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..load as i32 {
            cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
            rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        }
        // reverse insertion order -> old_index == final_index every time
        for i in (0..load as i32).rev() {
            let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
            let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
            diff_eq!(a, b, "load={load} last-del {i}");
            assert_eq!(a, 1);
            diff_eq!(cm.snap(), rm.snap(), "load={load} last-del {i} state");
        }
        assert_eq!(cm.hmlen(), 0);
        cm.hmfree();
        rm.hmfree();
    }
}

/// E34 — `mode == 1` + `SH_STRDUP` frees the duplicated key; `mode == 2` does
/// not (and would take the memcmp re-find path, so only last-element deletes
/// are exercised).
#[test]
fn err_e34_hmdel_mode2_quirk() {
    let p = libs();
    let spec = Spec::ptr(16);
    let mut rng = Rng::new(34);
    let mut keys = Keys::new();
    let pool: Vec<*mut c_char> = (0..40)
        .map(|_| {
            let n = rng.range(4, 20);
            keys.add(&rng.cstr_body(n, false))
        })
        .collect();

    // mode == 1: the strdup'd key IS freed (arbitrary delete order is fine)
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, STBDS_SH_STRDUP);
    let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, STBDS_SH_STRDUP);
    for (i, &k) in pool.iter().enumerate() {
        cm.shput(k, &(i as u64).to_ne_bytes());
        rm.shput(k, &(i as u64).to_ne_bytes());
    }
    for (i, &k) in pool.iter().enumerate() {
        let a = cm.hmdel(k as *mut c_void, 0);
        let b = rm.hmdel(k as *mut c_void, 0);
        diff_eq!(a, b, "mode=1 strdup del {i}");
        assert_eq!(a, 1);
        diff_eq!(cm.snap(), rm.snap(), "mode=1 strdup del {i} state");
    }
    cm.hmfree();
    rm.hmfree();

    // mode == 2: `mode == STBDS_HM_STRING` is false, so the key is NOT freed and
    // the (UB) memcmp re-find is avoided by deleting in reverse order.
    for mode in [2i32, 3, 999, i32::MAX] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new_shmode(&p.c, spec, mode, STBDS_SH_STRDUP);
        let mut rm = Map::new_shmode(&p.r, spec, mode, STBDS_SH_STRDUP);
        for (i, &k) in pool.iter().enumerate() {
            let a = cm.shput(k, &(i as u64).to_ne_bytes());
            let b = rm.shput(k, &(i as u64).to_ne_bytes());
            diff_eq!(a, b, "mode={mode} strdup put {i}");
        }
        diff_eq!(cm.snap(), rm.snap(), "mode={mode} strdup filled");
        for (i, &k) in pool.iter().enumerate().rev() {
            let a = cm.hmdel(k as *mut c_void, 0);
            let b = rm.hmdel(k as *mut c_void, 0);
            diff_eq!(a, b, "mode={mode} strdup rev-del {i}");
            assert_eq!(a, 1);
            diff_eq!(cm.snap(), rm.snap(), "mode={mode} strdup rev-del {i} state");
        }
        cm.hmfree();
        rm.hmfree();
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E35 — shrink when `used_count < used_count_shrink_threshold`.
#[test]
fn err_e35_hmdel_shrink() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    for i in 0..200i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
    }
    assert_eq!(cm.snap().idx.slot_count, 512, "200 inserts land in a 512-slot table");
    let mut shrinks = Vec::new();
    let mut prev = cm.snap().idx.slot_count;
    for i in 0..200i32 {
        let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        diff_eq!(a, b, "shrink del {i}");
        diff_eq!(cm.snap(), rm.snap(), "shrink del {i} state");
        let now = cm.snap().idx.slot_count;
        if now != prev {
            shrinks.push((prev, now));
            prev = now;
        }
    }
    assert_eq!(
        shrinks,
        vec![(512usize, 256usize), (256, 128), (128, 64), (64, 32), (32, 16), (16, 8)],
        "expected the full shrink chain"
    );
    cm.hmfree();
    rm.hmfree();
}

/// E36 — tombstone rebuild at the *same* `slot_count`.
#[test]
fn err_e36_hmdel_tombstone_rebuild() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    // 16-slot table: used_count_threshold 12, shrink_threshold 4,
    // tombstone_count_threshold 3.  Fill to 12, then delete 4:
    // used_count 8 (>= 4, so no shrink) but tombstone_count 4 (> 3) -> rebuild.
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    for i in 0..12i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
    }
    let s = cm.snap();
    assert_eq!(s.idx.slot_count, 16);
    assert_eq!(s.idx.used_count, 12);
    assert_eq!(s.idx.tombstone_count_threshold, 3);
    assert_eq!(s.idx.used_count_shrink_threshold, 4);
    let mut tombs = Vec::new();
    for i in 0..4i32 {
        let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        diff_eq!(a, b, "tombstone del {i}");
        diff_eq!(cm.snap(), rm.snap(), "tombstone del {i} state");
        let s = cm.snap();
        tombs.push((s.idx.slot_count, s.idx.tombstone_count, s.idx.used_count));
    }
    assert_eq!(
        tombs,
        vec![(16usize, 1usize, 11usize), (16, 2, 10), (16, 3, 9), (16, 0, 8)],
        "the 4th delete must rebuild at the same slot_count and clear tombstones"
    );
    cm.hmfree();
    rm.hmfree();
}

/// E37 — an 8-slot table never shrinks (`used_count_shrink_threshold == 0`).
#[test]
fn err_e37_no_shrink_at_8() {
    let p = libs();
    let spec = Spec::bytes(8, 4);
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    for i in 0..6i32 {
        cm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
        rm.hmput(&i.to_ne_bytes(), &i.to_ne_bytes());
    }
    let s = cm.snap();
    assert_eq!(s.idx.slot_count, 8);
    assert_eq!(s.idx.used_count_shrink_threshold, 0);
    for i in 0..6i32 {
        let a = cm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        let b = rm.hmdel(i.to_ne_bytes().as_ptr() as *mut c_void, 0);
        diff_eq!(a, b, "8-slot del {i}");
        diff_eq!(cm.snap(), rm.snap(), "8-slot del {i} state");
        assert_eq!(cm.snap().idx.slot_count, 8, "must never shrink below 8");
    }
    cm.hmfree();
    rm.hmfree();
}

/// E40 — `keysize == 0` in BINARY mode: `memcmp(...,0) == 0`, so every key
/// compares equal and only the first insert ever lands.
#[test]
fn err_e40_keysize_zero() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16] {
        reset_seed(&p, DEFAULT_SEED);
        let spec = Spec::bytes(elemsize, 0);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..20u8 {
            let v = vec![i; elemsize];
            let a = cm.hmput(&[], &v);
            let b = rm.hmput(&[], &v);
            diff_eq!(a, b, "e={elemsize} keysize0 put {i}");
            assert_eq!(a, 0, "every key must collapse onto index 0");
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} keysize0 state {i}");
        }
        assert_eq!(cm.hmlen(), 1, "only one element can ever exist");
        let a = cm.hmgeti(&[]);
        let b = rm.hmgeti(&[]);
        diff_eq!(a, b, "e={elemsize} keysize0 get");
        assert_eq!(a, 0);
        let a = cm.hmdel(std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void, 0);
        let b = rm.hmdel(std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void, 0);
        diff_eq!(a, b, "e={elemsize} keysize0 del");
        assert_eq!(a, 1);
        diff_eq!(cm.snap(), rm.snap(), "e={elemsize} keysize0 after del");
        cm.hmfree();
        rm.hmfree();
    }
}

// ===========================================================================
// Hash functions
// ===========================================================================

/// E41 — `stbds_hash_bytes(NULL, 0, seed)` never dereferences.
#[test]
fn err_e41_hash_bytes_len0() {
    let p = libs();
    let mut rng = Rng::new(41);
    for i in 0..200 {
        let seed = if i == 0 {
            0
        } else if i == 1 {
            usize::MAX
        } else {
            rng.next_u64() as usize
        };
        let c = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        let r = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed) };
        diff_eq!(c, r, "hash_bytes(NULL,0,{seed:#x})");
    }
}

/// E43 — `stbds_hash_string("")`.
#[test]
fn err_e43_hash_string_empty() {
    let p = libs();
    let empty = [0u8];
    let mut rng = Rng::new(43);
    for i in 0..200 {
        let seed = if i == 0 {
            0
        } else if i == 1 {
            usize::MAX
        } else {
            rng.next_u64() as usize
        };
        let c = unsafe { (p.c.hash_string)(empty.as_ptr() as *mut c_char, seed) };
        let r = unsafe { (p.r.hash_string)(empty.as_ptr() as *mut c_char, seed) };
        diff_eq!(c, r, "hash_string(\"\",{seed:#x})");
    }
}

/// E45 — `hash < 2` is bumped to `hash + 2` in both `hmput_key` and
/// `hm_find_slot`.
#[test]
fn err_e45_hash_lt_2() {
    let p = libs();
    let empty = [0u8];
    let k0 = unsafe { (p.c.hash_string)(empty.as_ptr() as *mut c_char, 0) };
    let k0r = unsafe { (p.r.hash_string)(empty.as_ptr() as *mut c_char, 0) };
    diff_eq!(k0, k0r, "hash_string(\"\",0)");
    for target in [0usize, 1] {
        let seed = target.wrapping_sub(k0);
        assert_eq!(
            unsafe { (p.c.hash_string)(empty.as_ptr() as *mut c_char, seed) },
            target
        );
        assert_eq!(
            unsafe { (p.r.hash_string)(empty.as_ptr() as *mut c_char, seed) },
            target
        );
        reset_seed(&p, seed);
        let spec = Spec::bytes(16, 8);
        let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, STBDS_SH_DEFAULT);
        let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, STBDS_SH_DEFAULT);
        let mut keys = Keys::new();
        let ek = keys.add(b"");
        let other = keys.add(b"not-empty");
        let a = cm.shput(ek, &[1u8; 8]);
        let b = rm.shput(ek, &[1u8; 8]);
        diff_eq!(a, b, "target={target} put");
        let a = cm.shput(other, &[2u8; 8]);
        let b = rm.shput(other, &[2u8; 8]);
        diff_eq!(a, b, "target={target} put other");
        diff_eq!(cm.snap(), rm.snap(), "target={target} state");
        let s = cm.snap();
        assert!(
            s.idx.slots.iter().any(|&(h, _)| h == target + 2),
            "stored hash must be {}: {:?}",
            target + 2,
            s.idx.slots
        );
        // find_slot must also apply the bump
        let a = cm.shgeti(ek);
        let b = rm.shgeti(ek);
        diff_eq!(a, b, "target={target} get");
        assert_eq!(a, 0);
        let a = cm.hmdel(ek as *mut c_void, 0);
        let b = rm.hmdel(ek as *mut c_void, 0);
        diff_eq!(a, b, "target={target} del");
        assert_eq!(a, 1);
        diff_eq!(cm.snap(), rm.snap(), "target={target} after del");
        cm.hmfree();
        rm.hmfree();
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ===========================================================================
// stbds_stralloc / stbds_strreset
// ===========================================================================

fn arena_step(lib: &Lib, a: *mut CStringArena, body: &[u8]) -> (ArenaSnap, Vec<u8>, bool) {
    let mut v = body.to_vec();
    v.push(0);
    unsafe {
        let ptr = (lib.stralloc)(a, v.as_ptr() as *mut c_char);
        let snap = snap_arena(a);
        let in_block = {
            let st = (*a).storage;
            if st.is_null() {
                false
            } else {
                let base = (&raw const (*st).storage) as *const u8;
                ptr as *const u8 == base.add((*a).remaining)
            }
        };
        (snap, read_cstr(ptr), in_block)
    }
}

/// E46 / E50 — `len > a->remaining` allocates a block and bumps `block`.
#[test]
fn err_e46_stralloc_newblock() {
    let p = libs();
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    // first call: remaining == 0 < len, so a block is always allocated
    let c = arena_step(&p.c, &mut ca, b"first");
    let r = arena_step(&p.r, &mut ra, b"first");
    diff_eq!(c.clone(), r, "first alloc");
    assert_eq!(c.0.block, 1, "block bumped 0 -> 1");
    assert_eq!(c.0.remaining, 512 - 6);
    assert!(c.2, "must sit in the current block");
    // keep filling until a new block is needed
    let mut blocks = 1usize;
    for i in 0..200 {
        let c = arena_step(&p.c, &mut ca, &vec![b'x'; 20]);
        let r = arena_step(&p.r, &mut ra, &vec![b'x'; 20]);
        diff_eq!(c.clone(), r, "fill {i}");
        if c.0.chain_len != blocks {
            blocks = c.0.chain_len;
        }
    }
    assert!(blocks > 1, "should have allocated more blocks");
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "reset");
}

/// E47 — `len > blocksize`: dedicated block spliced *after* `storage`,
/// `remaining` untouched.
#[test]
fn err_e47_stralloc_oversized() {
    let p = libs();
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    let c = arena_step(&p.c, &mut ca, b"seed");
    let r = arena_step(&p.r, &mut ra, b"seed");
    diff_eq!(c.clone(), r, "seed");
    let rem_before = c.0.remaining;
    let c = arena_step(&p.c, &mut ca, &vec![b'B'; 5000]);
    let r = arena_step(&p.r, &mut ra, &vec![b'B'; 5000]);
    diff_eq!(c.clone(), r, "oversized");
    assert_eq!(c.0.remaining, rem_before, "remaining must NOT be reduced");
    assert_eq!(c.0.chain_len, 2, "spliced after storage");
    assert!(!c.2, "returned pointer is in the dedicated block");
    assert_eq!(c.1.len(), 5000);
    // the arena keeps working afterwards
    for i in 0..30 {
        let c = arena_step(&p.c, &mut ca, &vec![b'z'; 10]);
        let r = arena_step(&p.r, &mut ra, &vec![b'z'; 10]);
        diff_eq!(c.clone(), r, "post-oversized {i}");
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "reset");
}

/// E48 — oversized while `a->storage == NULL`.
#[test]
fn err_e48_stralloc_oversized_empty() {
    let p = libs();
    for n in [512usize, 513, 1024, 200_000] {
        let mut ca = CStringArena::zeroed();
        let mut ra = CStringArena::zeroed();
        let c = arena_step(&p.c, &mut ca, &vec![b'Q'; n]);
        let r = arena_step(&p.r, &mut ra, &vec![b'Q'; n]);
        diff_eq!(c.clone(), r, "oversized-first n={n}");
        assert_eq!(c.0.remaining, 0, "remaining forced to 0");
        assert_eq!(c.0.chain_len, 1);
        assert_eq!(c.0.block, 1, "block was still bumped");
        assert_eq!(c.1.len(), n);
        // the returned pointer IS storage->storage here, but remaining == 0
        assert!(c.2, "p == storage->storage + 0");
        let c2 = arena_step(&p.c, &mut ca, b"next");
        let r2 = arena_step(&p.r, &mut ra, b"next");
        diff_eq!(c2.clone(), r2, "after oversized-first n={n}");
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "reset n={n}");
    }
}

/// E49 — `a->block` saturates at 22 once `blocksize == 1 << 20`.
#[test]
fn err_e49_stralloc_block_saturate() {
    let p = libs();
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    let mut want = 300usize;
    let mut max_block = 0u8;
    for i in 0..40 {
        let c = arena_step(&p.c, &mut ca, &vec![b'S'; want]);
        let r = arena_step(&p.r, &mut ra, &vec![b'S'; want]);
        diff_eq!(c.clone(), r, "saturate {i} (len {want})");
        max_block = max_block.max(c.0.block);
        want = want * 3 / 2 + 1;
        if want > 2_500_000 {
            want = 2_500_000;
        }
    }
    assert_eq!(max_block, 22, "block must saturate at exactly 22");
    for i in 0..10 {
        let c = arena_step(&p.c, &mut ca, &vec![b'T'; 1_500_000]);
        let r = arena_step(&p.r, &mut ra, &vec![b'T'; 1_500_000]);
        diff_eq!(c.clone(), r, "post-saturate {i}");
        assert_eq!(c.0.block, 22);
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "reset");
}

/// E51 — the empty string consumes exactly one byte.
#[test]
fn err_e51_stralloc_empty_str() {
    let p = libs();
    let mut ca = CStringArena::zeroed();
    let mut ra = CStringArena::zeroed();
    let c = arena_step(&p.c, &mut ca, b"");
    let r = arena_step(&p.r, &mut ra, b"");
    diff_eq!(c.clone(), r, "empty string first");
    assert_eq!(c.0.remaining, 511, "512 - 1");
    assert_eq!(c.1, Vec::<u8>::new());
    for i in 0..511 {
        let c = arena_step(&p.c, &mut ca, b"");
        let r = arena_step(&p.r, &mut ra, b"");
        diff_eq!(c.clone(), r, "empty string {i}");
        assert_eq!(c.0.remaining, 510 - i);
    }
    // remaining is now 0 -> next call must allocate
    let c = arena_step(&p.c, &mut ca, b"");
    let r = arena_step(&p.r, &mut ra, b"");
    diff_eq!(c.clone(), r, "empty string after exhaustion");
    assert_eq!(c.0.chain_len, 2);
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    diff_eq!(unsafe { snap_arena(&ca) }, unsafe { snap_arena(&ra) }, "reset");
}

/// E52 — `strreset` on an arena with `storage == NULL`.
#[test]
fn err_e52_strreset_empty() {
    let p = libs();
    for (storage_null, remaining, block, mode) in [
        (true, 0usize, 0u8, 0u8),
        (true, 1234, 7, 3),
        (true, usize::MAX, 255, 255),
    ] {
        assert!(storage_null);
        let mut ca = CStringArena::zeroed();
        let mut ra = CStringArena::zeroed();
        ca.remaining = remaining;
        ca.block = block;
        ca.mode = mode;
        ra.remaining = remaining;
        ra.block = block;
        ra.mode = mode;
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        let cs = unsafe { snap_arena(&ca) };
        diff_eq!(cs.clone(), unsafe { snap_arena(&ra) }, "strreset empty");
        assert_eq!(
            cs,
            ArenaSnap {
                has_storage: false,
                remaining: 0,
                block: 0,
                mode: 0,
                chain_len: 0
            }
        );
        // idempotent
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(
            unsafe { snap_arena(&ca) },
            unsafe { snap_arena(&ra) },
            "strreset twice"
        );
    }
}

// ===========================================================================
// stbds_shmode_func
// ===========================================================================

/// E53 — out-of-range `mode` is truncated by `(unsigned char) mode`.
#[test]
fn err_e53_shmode_out_of_range() {
    let p = libs();
    let elemsize = 16usize;
    for (mode, want) in [
        (0i32, 0u8),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 5),
        (255, 255),
        (256, 0),
        (257, 1),
        (511, 255),
        (999, 231),
        (-1, 255),
        (-2, 254),
        (-256, 0),
        (i32::MIN, 0),
        (i32::MAX, 255),
    ] {
        reset_seed(&p, DEFAULT_SEED);
        let spec = Spec::bytes(elemsize, 8);
        let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_BINARY, mode);
        let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_BINARY, mode);
        let cs = cm.snap();
        diff_eq!(cs.clone(), rm.snap(), "shmode_func(_,{mode}) pristine");
        assert_eq!(cs.idx.arena.mode, want, "(unsigned char){mode}");
        // Only 1/2/3 have a `case` in hmput_key's switch; everything else takes
        // `default:` -> memcpy.  Drive BINARY puts, which is safe for all modes
        // except SH_DEFAULT/STRDUP/ARENA (which store pointers).
        if !(1..=3).contains(&(want as i32)) {
            let mut rng = Rng::new(53 + mode as i64 as u64);
            for i in 0..40 {
                let k = rng.bytes(8);
                let v = rng.bytes(elemsize - 8);
                let a = cm.hmput(&k, &v);
                let b = rm.hmput(&k, &v);
                diff_eq!(a, b, "shmode={mode} put {i}");
                diff_eq!(cm.snap(), rm.snap(), "shmode={mode} put state {i}");
            }
            // hmfree_func must NOT try to free keys unless string.mode == SH_STRDUP
            diff_eq!(cm.snap(), rm.snap(), "shmode={mode} final");
        }
        cm.hmfree();
        rm.hmfree();
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E54 — `elemsize == 0`.
#[test]
fn err_e54_shmode_elemsize0() {
    let p = libs();
    for mode in [0i32, 1, 2, 3, 255, 256, -1, i32::MIN, i32::MAX] {
        reset_seed(&p, DEFAULT_SEED);
        let c = unsafe { (p.c.shmode_func)(0, mode) };
        let r = unsafe { (p.r.shmode_func)(0, mode) };
        // elemsize 0 -> STBDS_ARR_TO_HASH is the identity
        let cs = unsafe { snap_full(c, Spec::bytes(0, 0)) };
        diff_eq!(cs.clone(), unsafe { snap_full(r, Spec::bytes(0, 0)) }, "shmode(0,{mode})");
        assert_eq!(cs.hdr.length, 1);
        assert_eq!(cs.hdr.capacity, 4);
        assert_eq!(cs.idx.slot_count, 8);
        assert_eq!(cs.idx.arena.mode, (mode as u32 & 0xFF) as u8);
        unsafe {
            (p.c.hmfree_func)(c, 0);
            (p.r.hmfree_func)(r, 0);
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ===========================================================================
// arr_push / strkey
// ===========================================================================

/// E55 / E56 — `arr_push(num <= 0)` does nothing and its assert holds.
#[test]
fn err_e55_arr_push_nonpositive() {
    let p = libs();
    for num in [0i32, -1, -2, -49, -50, -51, -1000, i32::MIN, i32::MIN + 1] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
}

/// E57 — `0 < num <= 50`: one outer iteration with an empty inner loop.
#[test]
fn err_e57_arr_push_small() {
    let p = libs();
    for num in [1i32, 2, 3, 25, 49, 50] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
    // and the first size that actually pushes
    for num in [51i32, 52, 100, 101] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
}

/// E58 — `strkey` on the extremes of `int`.
#[test]
fn err_e58_strkey_extremes() {
    let p = libs();
    for n in [
        0i32,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        -2147483647,
    ] {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            let cb = core::slice::from_raw_parts(cp as *const u8, 256).to_vec();
            let rb = core::slice::from_raw_parts(rp as *const u8, 256).to_vec();
            diff_eq!(cb, rb, "strkey({n}) buffer");
            assert_eq!(read_cstr(cp), format!("test_{n}").as_bytes());
        }
    }
}

/// E59 — the static buffer is shared between calls.
#[test]
fn err_e59_strkey_aliases() {
    let p = libs();
    unsafe {
        let a = (p.c.strkey)(-2147483648);
        let b = (p.c.strkey)(0);
        assert_eq!(a, b, "C: one static buffer");
        let x = (p.r.strkey)(-2147483648);
        let y = (p.r.strkey)(0);
        assert_eq!(x, y, "RUST: one static buffer");
        let cb = core::slice::from_raw_parts(b as *const u8, 256).to_vec();
        let rb = core::slice::from_raw_parts(y as *const u8, 256).to_vec();
        diff_eq!(cb, rb, "leftover tail after the long render");
        assert_eq!(read_cstr(b), b"test_0");
    }
}

// ===========================================================================
// Generic FFI-boundary rows
// ===========================================================================

/// E61 — `stbds_rand_seed` extremes and the resulting LCG chain.
#[test]
fn err_e61_seed_extremes() {
    let p = libs();
    for seed in [
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        1 << 63,
        (1 << 63) | 1,
        0x3141_5926,
    ] {
        let mut cs = Vec::new();
        let mut rs = Vec::new();
        unsafe {
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
            let mut cts = Vec::new();
            let mut rts = Vec::new();
            for _ in 0..8 {
                let ct = (p.c.shmode_func)(8, STBDS_SH_NONE);
                let rt = (p.r.shmode_func)(8, STBDS_SH_NONE);
                let craw = (ct as *mut u8).sub(8) as *mut c_void;
                let rraw = (rt as *mut u8).sub(8) as *mut c_void;
                cs.push(snap_idx(craw).seed);
                rs.push(snap_idx(rraw).seed);
                cts.push(craw);
                rts.push(rraw);
            }
            for (c, r) in cts.into_iter().zip(rts) {
                (p.c.hmfree_func)(c, 8);
                (p.r.hmfree_func)(r, 8);
            }
        }
        diff_eq!(cs.clone(), rs, "seed chain from {seed:#x}");
        assert_eq!(cs[0], seed, "first table takes the seed verbatim");
        // verify the documented LCG
        for w in cs.windows(2) {
            assert_eq!(
                w[1],
                w[0]
                    .wrapping_mul(0x27bb_2ee6_87b0_b0fd)
                    .wrapping_add(0xb504_f32d),
                "LCG step from {:#x}",
                w[0]
            );
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

/// E62 — `keyoffset != 0` in `stbds_hmdel_key`, which the `hm*` macros never do
/// for `hmput`/`hmget`.
#[test]
fn err_e62_hmdel_keyoffset() {
    let p = libs();
    let elemsize = 16usize;
    let keysize = 8usize;
    let spec = Spec::bytes(elemsize, keysize);

    // (a) the stored key does NOT appear at keyoffset -> lookup fails -> temp 0
    for keyoffset in [1usize, 4, 8] {
        reset_seed(&p, DEFAULT_SEED);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for i in 0..20u64 {
            // value bytes deliberately different from the key bytes
            let k = i.to_ne_bytes();
            let v = (i ^ 0xFFFF_FFFF_FFFF_FFFF).to_ne_bytes();
            cm.hmput(&k, &v);
            rm.hmput(&k, &v);
        }
        let cs = cm.snap();
        let rs = rm.snap();
        for i in 0..20u64 {
            let k = i.to_ne_bytes();
            let a = cm.hmdel(k.as_ptr() as *mut c_void, keyoffset);
            let b = rm.hmdel(k.as_ptr() as *mut c_void, keyoffset);
            diff_eq!(a, b, "keyoffset={keyoffset} del {i}");
            assert_eq!(a, 0, "keyoffset={keyoffset}: key isn't there, must report 0");
            diff_eq!(cm.snap(), rm.snap(), "keyoffset={keyoffset} del {i} state");
        }
        assert_eq!(without_temp(cm.snap()), without_temp(cs));
        assert_eq!(without_temp(rm.snap()), without_temp(rs));
        cm.hmfree();
        rm.hmfree();
    }

    // (b) the key IS duplicated at keyoffset == 8, and only the physically-last
    //     element is deleted (so no memmove + re-find happens).
    reset_seed(&p, DEFAULT_SEED);
    let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
    let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
    for i in 0..20u64 {
        let k = i.to_ne_bytes();
        cm.hmput(&k, &k); // key duplicated into the value slot
        rm.hmput(&k, &k);
    }
    diff_eq!(cm.snap(), rm.snap(), "keyoffset dup filled");
    for i in (0..20u64).rev() {
        let k = i.to_ne_bytes();
        let a = cm.hmdel(k.as_ptr() as *mut c_void, 8);
        let b = rm.hmdel(k.as_ptr() as *mut c_void, 8);
        diff_eq!(a, b, "keyoffset=8 dup del {i}");
        assert_eq!(a, 1, "the duplicated key at offset 8 must be found");
        diff_eq!(cm.snap(), rm.snap(), "keyoffset=8 dup del {i} state");
    }
    assert_eq!(cm.hmlen(), 0);
    cm.hmfree();
    rm.hmfree();
}

/// E63 — `keysize` larger than the nominal key field (still inside the element,
/// so the C's `memcmp`/`memcpy` over-read stays in bounds).
#[test]
fn err_e63_keysize_oversized() {
    let p = libs();
    // elemsize 32, nominal key 8 bytes, but keysize 24 -> memcmp spans key+value
    for (elemsize, keysize) in [(32usize, 24usize), (32, 32), (16, 12), (8, 8), (8, 5)] {
        reset_seed(&p, DEFAULT_SEED);
        let spec = Spec::bytes(elemsize, keysize);
        let mut rng = Rng::new(63 + elemsize as u64 * 100 + keysize as u64);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        let mut ks = Vec::new();
        for i in 0..80 {
            let k = rng.bytes(keysize);
            let v = rng.bytes(elemsize - keysize);
            let a = cm.hmput(&k, &v);
            let b = rm.hmput(&k, &v);
            diff_eq!(a, b, "e={elemsize} k={keysize} put {i}");
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} k={keysize} state {i}");
            ks.push(k);
        }
        for (i, k) in ks.iter().enumerate() {
            let a = cm.hmgeti(k);
            let b = rm.hmgeti(k);
            diff_eq!(a, b, "e={elemsize} k={keysize} get {i}");
            assert_eq!(a, i as isize);
        }
        for (i, k) in ks.iter().enumerate().rev() {
            let a = cm.hmdel(k.as_ptr() as *mut c_void, 0);
            let b = rm.hmdel(k.as_ptr() as *mut c_void, 0);
            diff_eq!(a, b, "e={elemsize} k={keysize} del {i}");
            assert_eq!(a, 1);
            diff_eq!(cm.snap(), rm.snap(), "e={elemsize} k={keysize} del {i} state");
        }
        cm.hmfree();
        rm.hmfree();
    }
}

/// E60 is documented-only (`stbds_arrfreef(NULL)` is `free((char*)NULL - 32)`,
/// undefined in the C itself).  This test only records that both libraries would
/// execute the identical arithmetic, without performing the invalid free.
#[test]
fn err_e60_arrfreef_null_documented() {
    // stbds_arrfreef(a) == STBDS_FREE(NULL, stbds_header(a)) == free(a - 32).
    // For a == NULL that is free((void*)-32), which glibc aborts on.  Both the C
    // and the Rust translation compute `(a as *mut u8) - 32` with wrapping
    // semantics, so they agree bit-for-bit; the call itself is deliberately not
    // made.  `arrfree()` (lib.c:121) null-checks before calling, so no
    // legitimate use reaches it.
    let a: *mut c_void = std::ptr::null_mut();
    let header = (a as *mut u8).wrapping_sub(32) as usize;
    assert_eq!(header, usize::MAX - 31, "the pointer both libs would free");
}
