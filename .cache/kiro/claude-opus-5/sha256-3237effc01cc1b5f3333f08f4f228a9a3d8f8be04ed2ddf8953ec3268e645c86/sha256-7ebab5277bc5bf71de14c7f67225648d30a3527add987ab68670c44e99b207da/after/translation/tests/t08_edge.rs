//! Level 7: paths the higher-level tests may not reach --
//!   * `stbds_hmget_key_ts` called with a NULL handle (it allocates)
//!   * `stbds_hmput_default` on a non-NULL, zero-length array
//!   * `stbds_stralloc`'s "oversized string, block list already exists" branch
//!   * finding an existing key in the *wrap-around* half of `hmput_key`'s probe
//!     (the only place where the C code skips the `temp_key` write)
//!   * string-mode inserts into a table whose `string.mode` is `STBDS_SH_NONE`
//!     (the `memcpy` fall-through, which copies the pointer bytes)

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[test]
fn hmget_key_ts_allocates_on_null() {
    let p = load_pair();
    for &es in &[8usize, 16, 24] {
        p.reset_seed(DEFAULT_SEED);
        let mut key = 42i32;
        let mut ctemp: isize = 1234;
        let mut rtemp: isize = 1234;
        let ct = unsafe {
            (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                &mut key as *mut i32 as *mut c_void,
                4,
                &mut ctemp,
                STBDS_HM_BINARY,
            )
        };
        let rt = unsafe {
            (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                &mut key as *mut i32 as *mut c_void,
                4,
                &mut rtemp,
                STBDS_HM_BINARY,
            )
        };
        assert_eq!(ctemp, rtemp, "hmget_key_ts(NULL, es={es}) temp");
        assert_eq!(ctemp, -1);
        assert_bytes_eq(
            &format!("hmget_key_ts(NULL, es={es})"),
            &unsafe { snapshot_map(ct, es, true) },
            &unsafe { snapshot_map(rt, es, true) },
        );
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (p.r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn hmput_default_on_zero_length_array() {
    let p = load_pair();
    for &es in &[8usize, 16] {
        p.reset_seed(DEFAULT_SEED);
        // arrgrowf gives capacity without length; hand the hash-form pointer to
        // hmput_default so that it takes the `length == 0` branch.
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 1) };
        let ra = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, 1) };
        let ch = unsafe { (ca as *mut u8).add(es) as *mut c_void };
        let rh = unsafe { (ra as *mut u8).add(es) as *mut c_void };
        let ct = unsafe { (p.c.hmput_default)(ch, es) };
        let rt = unsafe { (p.r.hmput_default)(rh, es) };
        assert_bytes_eq(
            &format!("hmput_default(len=0, es={es})"),
            &unsafe { snapshot_map(ct, es, true) },
            &unsafe { snapshot_map(rt, es, true) },
        );
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(es) as *mut c_void, es);
            (p.r.hmfree_func)((rt as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn stralloc_oversized_with_existing_block() {
    let p = load_pair();
    let mut ca = StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut ra = ca;

    let snap = |a: &StringArena, ret: *mut c_char| -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&a.remaining.to_le_bytes());
        out.push(a.block);
        out.push(a.storage.is_null() as u8);
        if ret.is_null() {
            out.push(0);
        } else {
            out.push(1);
            out.extend_from_slice(unsafe { CStr::from_ptr(ret) }.to_bytes_with_nul());
        }
        out
    };

    // 1. a small string -> creates the first 512-byte block
    let mut small = b"hello\0".to_vec();
    let cp = unsafe { (p.c.stralloc)(&mut ca, small.as_mut_ptr() as *mut c_char) };
    let rp = unsafe { (p.r.stralloc)(&mut ra, small.as_mut_ptr() as *mut c_char) };
    assert_bytes_eq("stralloc small (fresh)", &snap(&ca, cp), &snap(&ra, rp));

    // 2. a string larger than the next block size -> "own block", spliced in
    //    behind the current head, `remaining` deliberately left untouched
    for &len in &[5000usize, 40_000, 300_000] {
        let mut big = vec![b'B'; len];
        big.push(0);
        let cp = unsafe { (p.c.stralloc)(&mut ca, big.as_mut_ptr() as *mut c_char) };
        let rp = unsafe { (p.r.stralloc)(&mut ra, big.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc oversized len={len} with existing block"),
            &snap(&ca, cp),
            &snap(&ra, rp),
        );
        // 3. small allocations must still come out of the original head block
        let cp = unsafe { (p.c.stralloc)(&mut ca, small.as_mut_ptr() as *mut c_char) };
        let rp = unsafe { (p.r.stralloc)(&mut ra, small.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc small after oversized len={len}"),
            &snap(&ca, cp),
            &snap(&ra, rp),
        );
    }

    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    assert_bytes_eq(
        "strreset after mixed blocks",
        &snap(&ca, std::ptr::null_mut()),
        &snap(&ra, std::ptr::null_mut()),
    );
}

/// Hammer the "overwrite an existing key" path across many different seeds so
/// that the wrap-around half of `hmput_key`'s bucket scan is exercised (that
/// branch is the only one that omits the `temp_key` write).
#[test]
fn overwrite_across_many_seeds() {
    let p = load_pair();
    const ES: usize = 8;
    for seed in 0..40usize {
        p.reset_seed(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for pass in 0..4 {
            for k in 0..90i32 {
                let mut key = k;
                let kp = &mut key as *mut i32 as *mut c_void;
                ct = unsafe { (p.c.hmput_key)(ct, ES, kp, 4, 0) };
                rt = unsafe { (p.r.hmput_key)(rt, ES, kp, 4, 0) };
                // write key+value exactly as the stbds_hmput macro does
                for (t, im) in [(ct, &p.c), (rt, &p.r)] {
                    let _ = im;
                    unsafe {
                        let temp = header((t as *mut u8).sub(ES) as *mut c_void).temp;
                        let e = (t as *mut u8).add(ES * temp as usize) as *mut c_int;
                        *e = k;
                        *e.add(1) = k * 10 + pass;
                    }
                }
                assert_bytes_eq(
                    &format!("seed#{seed} pass{pass} put({k})"),
                    &unsafe { snapshot_map(ct, ES, true) },
                    &unsafe { snapshot_map(rt, ES, true) },
                );
            }
        }
        unsafe {
            (p.c.hmfree_func)((ct as *mut u8).sub(ES) as *mut c_void, ES);
            (p.r.hmfree_func)((rt as *mut u8).sub(ES) as *mut c_void, ES);
        }
    }
}

/// A `STBDS_SH_NONE` table used with `STBDS_HM_STRING`: `hmput_key` falls
/// through to `memcpy`, which copies `keysize` bytes *of the string itself*
/// into the element's key field (not the pointer). The bytes are compared raw;
/// the field must not be read back as a `char *`, and string lookups on such a
/// table would make `stbds_is_key_equal` `strcmp` a garbage pointer, so only
/// the well-defined insert path is compared here.
#[test]
fn string_mode_none_table() {
    let p = load_pair();
    p.reset_seed(DEFAULT_SEED);
    const ES: usize = 16;
    const KS: usize = 8;
    let keys: Vec<CString> = (0..120)
        .map(|i| CString::new(format!("shnone{i:04}_padding")).unwrap())
        .collect();

    let mut ct = unsafe { (p.c.shmode_func)(ES, STBDS_SH_NONE) };
    let mut rt = unsafe { (p.r.shmode_func)(ES, STBDS_SH_NONE) };

    let snap = |t: *mut c_void| -> Vec<u8> {
        unsafe {
            let mut out = Vec::new();
            let raw = (t as *mut u8).sub(ES) as *mut c_void;
            snapshot_header(&mut out, raw);
            snapshot_hash_index(&mut out, raw);
            let len = header(raw).length;
            for i in 0..len {
                // raw key bytes (NOT a pointer in this mode) + the value
                out.extend_from_slice(std::slice::from_raw_parts(
                    (raw as *const u8).add(ES * i),
                    KS,
                ));
                out.extend_from_slice(
                    &(*((raw as *mut u8).add(ES * i).add(8) as *mut c_int)).to_le_bytes(),
                );
            }
            out
        }
    };

    for (i, k) in keys.iter().enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        ct = unsafe { (p.c.hmput_key)(ct, ES, kp, KS, STBDS_HM_STRING) };
        rt = unsafe { (p.r.hmput_key)(rt, ES, kp, KS, STBDS_HM_STRING) };
        for t in [ct, rt] {
            unsafe {
                let temp = header((t as *mut u8).sub(ES) as *mut c_void).temp;
                *((t as *mut u8).add(ES * temp as usize).add(8) as *mut c_int) = i as c_int;
            }
        }
        assert_bytes_eq(&format!("SH_NONE put #{i}"), &snap(ct), &snap(rt));
    }
    unsafe {
        (p.c.hmfree_func)((ct as *mut u8).sub(ES) as *mut c_void, ES);
        (p.r.hmfree_func)((rt as *mut u8).sub(ES) as *mut c_void, ES);
    }
}

/// `stbds_hmdel_key(NULL, ...)` must return NULL on both sides.
#[test]
fn hmdel_key_on_null() {
    let p = load_pair();
    let mut key = 1i32;
    let kp = &mut key as *mut i32 as *mut c_void;
    let c = unsafe { (p.c.hmdel_key)(std::ptr::null_mut(), 8, kp, 4, 0, STBDS_HM_BINARY) };
    let r = unsafe { (p.r.hmdel_key)(std::ptr::null_mut(), 8, kp, 4, 0, STBDS_HM_BINARY) };
    assert!(c.is_null() && r.is_null(), "C={c:?} Rust={r:?}");
}

/// A map that has a default element but no hash table yet: `hmget_key_ts` and
/// `hmdel_key` must both take their `table == NULL` shortcuts.
#[test]
fn no_hash_table_yet() {
    let p = load_pair();
    const ES: usize = 8;
    p.reset_seed(DEFAULT_SEED);
    let mut ct = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), ES) };
    let mut rt = unsafe { (p.r.hmput_default)(std::ptr::null_mut(), ES) };
    let mut key = 5i32;
    let kp = &mut key as *mut i32 as *mut c_void;

    let mut ctemp: isize = 7;
    let mut rtemp: isize = 7;
    ct = unsafe { (p.c.hmget_key_ts)(ct, ES, kp, 4, &mut ctemp, STBDS_HM_BINARY) };
    rt = unsafe { (p.r.hmget_key_ts)(rt, ES, kp, 4, &mut rtemp, STBDS_HM_BINARY) };
    assert_eq!(ctemp, rtemp);
    assert_bytes_eq(
        "hmget_key_ts with no table",
        &unsafe { snapshot_map(ct, ES, true) },
        &unsafe { snapshot_map(rt, ES, true) },
    );

    ct = unsafe { (p.c.hmdel_key)(ct, ES, kp, 4, 0, STBDS_HM_BINARY) };
    rt = unsafe { (p.r.hmdel_key)(rt, ES, kp, 4, 0, STBDS_HM_BINARY) };
    assert_bytes_eq(
        "hmdel_key with no table",
        &unsafe { snapshot_map(ct, ES, true) },
        &unsafe { snapshot_map(rt, ES, true) },
    );

    unsafe {
        (p.c.hmfree_func)((ct as *mut u8).sub(ES) as *mut c_void, ES);
        (p.r.hmfree_func)((rt as *mut u8).sub(ES) as *mut c_void, ES);
    }
}
