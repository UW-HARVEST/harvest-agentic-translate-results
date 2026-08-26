//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Non-fatal rows are compared in-process (returned sentinel + full observable
//! state). Rows whose expected C behaviour is process death (`assert` =>
//! SIGABRT, NULL dereference => SIGSEGV) are compared by running the identical
//! call in a child process against each `.so` and requiring the same
//! termination signal and the same stdout.
mod common;

use common::*;
use core::ffi::{c_char, c_int, c_void};

/// `stbds_hmput_key` writes only `keysize` key bytes into a freshly appended
/// element; bytes `[keysize..elemsize)` are whatever `realloc` handed back and
/// are therefore NOT comparable across two independent heaps. The `hmput` /
/// `shput` macros always store a value there immediately; this helper does the
/// same, with identical bytes on both sides, so the snapshot is well defined.
unsafe fn init_value_region(
    a: *mut c_void,
    b: *mut c_void,
    elemsize: usize,
    keysize: usize,
    tag: u8,
) {
    let hdr = hdr_of_map(a, elemsize);
    let n = (*hdr).length;
    for i in 0..n {
        for off in keysize..elemsize {
            let idx = i as isize - 1;
            *elem(a, elemsize, idx).add(off) = tag ^ (off as u8) ^ (i as u8);
            *elem(b, elemsize, idx).add(off) = tag ^ (off as u8) ^ (i as u8);
        }
    }
}

/// The subprocess entry point (see `common::spawn_scenario`).
#[test]
fn scenario_runner() {
    let Ok(name) = std::env::var("DIFF_SCENARIO") else {
        return;
    };
    let which = std::env::var("DIFF_LIB").expect("DIFF_LIB");
    let lib = Lib::pick(&which);
    unsafe { run_scenario(&name, &lib) };
    std::process::exit(0);
}

// ===========================================================================
// stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

/// Row 1 — `a == NULL`, `addlen == 0`, `min_cap == 0` ⇒ returns NULL.
#[test]
fn err_arrgrowf_null_nogrow() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            let a = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 0);
            let b = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 0);
            assert!(a.is_null(), "C must return NULL (elemsize={elemsize})");
            assert!(b.is_null(), "Rust must return NULL (elemsize={elemsize})");
        }
    }
}

/// Row 2 — non-NULL `a`, `min_cap <= capacity` ⇒ same pointer, header untouched.
#[test]
fn err_arrgrowf_no_grow_returns_same() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            let a0 = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 8);
            let b0 = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 8);
            (*hdr_of_arr(a0)).length = 3;
            (*hdr_of_arr(b0)).length = 3;
            (*hdr_of_arr(a0)).temp = 0x1234;
            (*hdr_of_arr(b0)).temp = 0x1234;
            for min_cap in [0usize, 1, 5, 8] {
                let a = (c.arrgrowf)(a0, elemsize, 0, min_cap);
                let b = (r.arrgrowf)(b0, elemsize, 0, min_cap);
                assert_eq!(a, a0, "C pointer changed");
                assert_eq!(b, b0, "Rust pointer changed");
                assert_eq!((*hdr_of_arr(a)).length, 3);
                assert_eq!((*hdr_of_arr(b)).length, 3);
                assert_eq!((*hdr_of_arr(a)).capacity, 8);
                assert_eq!((*hdr_of_arr(b)).capacity, 8);
                assert_eq!((*hdr_of_arr(a)).temp, 0x1234);
                assert_eq!((*hdr_of_arr(b)).temp, 0x1234);
            }
            (c.arrfreef)(a0);
            (r.arrfreef)(b0);
        }
    }
}

/// Row 3 — `min_cap` 1..3 is clamped up to 4, never honoured verbatim.
#[test]
fn err_arrgrowf_min_cap_clamped_to_4() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            for min_cap in [1usize, 2, 3] {
                let a = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, min_cap);
                let b = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, min_cap);
                assert_eq!((*hdr_of_arr(a)).capacity, 4);
                assert_eq!((*hdr_of_arr(b)).capacity, 4);
                assert_eq!((*hdr_of_arr(a)).length, 0);
                assert_eq!((*hdr_of_arr(b)).length, 0);
                assert!((*hdr_of_arr(a)).hash_table.is_null());
                assert!((*hdr_of_arr(b)).hash_table.is_null());
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

/// Row 4 — `elemsize * min_cap` wraps `size_t`. Both must wrap identically
/// (`SIZE_MAX/2 * 4 + 32 == 28`), allocate 28 bytes and still record
/// `capacity == 4`.
#[test]
fn err_arrgrowf_size_overflow() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for (elemsize, min_cap) in [
            (usize::MAX / 2, 4usize),
            (usize::MAX, 4),
            (1usize << 62, 4),
            (1usize << 63, 2),
            (usize::MAX, 1),
        ] {
            let a = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, min_cap);
            let b = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, min_cap);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "NULL-ness diverged for elemsize={elemsize:#x} min_cap={min_cap}"
            );
            if !a.is_null() {
                assert_eq!(
                    (*hdr_of_arr(a)).capacity,
                    (*hdr_of_arr(b)).capacity,
                    "capacity diverged for elemsize={elemsize:#x} min_cap={min_cap}"
                );
                assert_eq!((*hdr_of_arr(a)).length, (*hdr_of_arr(b)).length);
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
        // and the same thing observed from a child process
        assert_scenario_matches("arrgrowf_size_overflow");
    }
}

/// Row 5 — `stbds_arrfreef(NULL)` == `free((void*)-32)`: fatal, identically.
#[test]
fn fatal_arrfreef_null() {
    let _g = serial();
    // `free((void*)-32)` — glibc rejects the pointer. Not a NULL dereference,
    // so Rust's debug UB checks do not intercept it: exact signal parity.
    let out = spawn_scenario("arrfreef_null", "c");
    let outr = spawn_scenario("arrfreef_null", "rust");
    assert!(
        out.signal.is_some(),
        "arrfreef(NULL) was expected to be fatal, got {}",
        out.describe()
    );
    assert_eq!(
        (out.code, out.signal, &out.stdout),
        (outr.code, outr.signal, &outr.stdout),
        "termination differs\n  C   : {}\n  Rust: {}",
        out.describe(),
        outr.describe()
    );
}

// ===========================================================================
// stbds_hash_string / stbds_hash_bytes
// ===========================================================================

/// Row 6 — `stbds_hash_string(NULL, seed)` dereferences NULL.
#[test]
fn fatal_hash_string_null() {
    let _g = serial();
    assert_fatal_scenario_matches("hash_string_null", SIGSEGV);
}

/// Row 7 — `stbds_hash_bytes(NULL, 0, seed)` is legal (no dereference) and must
/// return the identical value.
#[test]
fn err_hash_bytes_null_len0() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for seed in [0usize, 1, 0x31415926, usize::MAX, 1 << 63] {
            let a = (c.hash_bytes)(core::ptr::null_mut(), 0, seed);
            let b = (r.hash_bytes)(core::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
            // and it must equal the hash of any zero-length buffer
            let buf = [0u8; 8];
            let d = (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, d);
        }
    }
}

/// Row 8 — `stbds_hash_bytes(NULL, 1, seed)` dereferences NULL.
#[test]
fn fatal_hash_bytes_null_len1() {
    let _g = serial();
    assert_fatal_scenario_matches("hash_bytes_null_len1", SIGSEGV);
}

/// Row 9 — oversized `len` (`SIZE_MAX`) walks off the end of the buffer.
#[test]
fn fatal_hash_bytes_huge_len() {
    let _g = serial();
    assert_fatal_scenario_matches("hash_bytes_huge_len", SIGSEGV);
}

// ===========================================================================
// stbds_hmfree_func
// ===========================================================================

/// Row 10 — `stbds_hmfree_func(NULL, elemsize)` is a no-op.
#[test]
fn err_hmfree_null_is_noop() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [0usize, 1, 16, usize::MAX] {
            (c.hmfree_func)(core::ptr::null_mut(), elemsize);
            (r.hmfree_func)(core::ptr::null_mut(), elemsize);
        }
        // still alive => it really was a no-op on both
        let a = (c.arrgrowf)(core::ptr::null_mut(), 8, 0, 4);
        let b = (r.arrgrowf)(core::ptr::null_mut(), 8, 0, 4);
        assert!(!a.is_null() && !b.is_null());
        (c.arrfreef)(a);
        (r.arrfreef)(b);
    }
}

/// Row 11 — `hash_table == NULL`: the strdup/arena teardown is skipped and
/// `free(NULL)` is harmless.
#[test]
fn err_hmfree_no_table() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [8usize, 16, 64] {
            // an array with no hash table at all
            let a = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 4);
            let b = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 4);
            (*hdr_of_arr(a)).length = 1;
            (*hdr_of_arr(b)).length = 1;
            assert!((*hdr_of_arr(a)).hash_table.is_null());
            assert!((*hdr_of_arr(b)).hash_table.is_null());
            (c.hmfree_func)(a, elemsize);
            (r.hmfree_func)(b, elemsize);

            // and via stbds_hmput_default, which also leaves hash_table NULL
            let a2 = (c.hmput_default)(core::ptr::null_mut(), elemsize);
            let b2 = (r.hmput_default)(core::ptr::null_mut(), elemsize);
            assert!(map_table(a2, elemsize).is_null());
            assert!(map_table(b2, elemsize).is_null());
            (c.hmfree_func)((a2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// stbds_hmget_key / stbds_hmget_key_ts
// ===========================================================================

/// Rows 12, 13, 14, 16 — every `stbds_hmget_key*` rejection path.
#[test]
fn err_hmget_ts_null_a() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [1usize, 8, 16, 64] {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let mut key = [0xABu8; 8];
            let kp = key.as_mut_ptr() as *mut c_void;
            let mut tc: isize = 0x5555;
            let mut tr: isize = 0x5555;
            let a = (c.hmget_key_ts)(
                core::ptr::null_mut(),
                elemsize,
                kp,
                8.min(elemsize),
                &mut tc,
                HM_BINARY,
            );
            let b = (r.hmget_key_ts)(
                core::ptr::null_mut(),
                elemsize,
                kp,
                8.min(elemsize),
                &mut tr,
                HM_BINARY,
            );
            assert_eq!(tc, -1, "C must report STBDS_INDEX_EMPTY");
            assert_eq!(tr, -1, "Rust must report STBDS_INDEX_EMPTY");
            assert!(!a.is_null() && !b.is_null(), "a new map must be allocated");
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "bootstrapped map diverged\n{}", diff_snaps(&sc, &sr));
            assert_eq!(sc.length, 1);
            assert!(!sc.has_table);
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 13 — `a != NULL` but `hash_table == 0` ⇒ `*temp = -1`, `a` unchanged.
#[test]
fn err_hmget_ts_no_table() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        let elemsize = 16usize;
        let mut key = [7u8; 8];
        let kp = key.as_mut_ptr() as *mut c_void;
        let a0 = (c.hmput_default)(core::ptr::null_mut(), elemsize);
        let b0 = (r.hmput_default)(core::ptr::null_mut(), elemsize);
        for mode in [HM_BINARY, HM_STRING, -1, 9] {
            let mut tc: isize = 0x77;
            let mut tr: isize = 0x77;
            let a = (c.hmget_key_ts)(a0, elemsize, kp, 8, &mut tc, mode);
            let b = (r.hmget_key_ts)(b0, elemsize, kp, 8, &mut tr, mode);
            assert_eq!(a, a0, "a must be returned unchanged");
            assert_eq!(b, b0);
            assert_eq!(tc, -1);
            assert_eq!(tr, -1);
        }
        // stbds_hmget_key writes stbds_temp too
        for mode in [HM_BINARY, HM_STRING] {
            let a = (c.hmget_key)(a0, elemsize, kp, 8, mode);
            let b = (r.hmget_key)(b0, elemsize, kp, 8, mode);
            assert_eq!(map_temp(a, elemsize), -1);
            assert_eq!(map_temp(b, elemsize), -1);
        }
        (c.hmfree_func)((a0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((b0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Row 14 — key absent from a populated table ⇒ `stbds_hm_find_slot` returns -1.
#[test]
fn err_hmget_ts_missing_key() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x1414_1414);
    unsafe {
        for n in [1usize, 5, 6, 12, 40] {
            (c.rand_seed)(3);
            (r.rand_seed)(3);
            let mut keys = Keys::new();
            let live: Vec<_> = (0..n).map(|_| keys.raw(&rng.bytes(8))).collect();
            let absent: Vec<_> = (0..32).map(|_| keys.raw(&rng.bytes(8))).collect();
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("miss/n{n}"));
            p.set_default(1);
            for (i, &k) in live.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            for &k in &absent {
                assert_eq!(p.geti(k, HM_BINARY), -1);
                assert_eq!(p.geti_ts(k, HM_BINARY), -1);
            }
            p.free();
        }
    }
}

/// Row 15 — `temp == NULL`.
#[test]
fn fatal_hmget_ts_null_temp() {
    let _g = serial();
    assert_fatal_scenario_matches("hmget_ts_null_temp", SIGSEGV);
}

/// Row 16 — `stbds_hmget_key(NULL, ...)` writes `stbds_temp` on the NEW array.
#[test]
fn err_hmget_key_null_a() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [8usize, 16, 64] {
            for mode in [HM_BINARY, HM_STRING, -5, 77] {
                (c.rand_seed)(9);
                (r.rand_seed)(9);
                let mut key: Vec<u8> = b"whatever\0".to_vec();
                let kp = key.as_mut_ptr() as *mut c_void;
                let a = (c.hmget_key)(core::ptr::null_mut(), elemsize, kp, 8, mode);
                let b = (r.hmget_key)(core::ptr::null_mut(), elemsize, kp, 8, mode);
                assert!(!a.is_null() && !b.is_null());
                assert_eq!(map_temp(a, elemsize), -1);
                assert_eq!(map_temp(b, elemsize), -1);
                let sc = snapshot(a, elemsize, ElemCmp::Raw);
                let sr = snapshot(b, elemsize, ElemCmp::Raw);
                assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
                (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// ===========================================================================
// out-of-range `mode` (C enums accept any int across the FFI boundary)
// ===========================================================================

/// Row 17 + G4 — every `mode < STBDS_HM_STRING` takes the binary path.
#[test]
fn err_mode_out_of_range_negative() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x1717_1717);
    unsafe {
        for mode in [c_int::MIN, -1000, -2, -1, 0] {
            (c.rand_seed)(4);
            (r.rand_seed)(4);
            let mut keys = Keys::new();
            let ks: Vec<_> = (0..20).map(|_| keys.raw(&rng.bytes(8))).collect();
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("negmode{mode}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.put(k, mode, i as u64), i as isize);
            }
            for (i, &k) in ks.iter().enumerate() {
                assert_eq!(p.geti(k, mode), i as isize);
                assert_eq!(p.geti_ts(k, mode), i as isize);
            }
            for &k in &ks {
                assert_eq!(p.del(k, 0, mode), 1);
            }
            p.free();
        }
    }
}

/// Row 18 + G4 — every `mode >= STBDS_HM_STRING` takes the string path.
#[test]
fn err_mode_out_of_range_positive() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x1818_1818);
    unsafe {
        for mode in [1 as c_int, 2, 3, 4, 255, 256, 259, 1000, c_int::MAX] {
            for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                (c.rand_seed)(6);
                (r.rand_seed)(6);
                let mut keys = Keys::new();
                let ks: Vec<_> = (0..15)
                    .map(|i| {
                        let mut s = rng.nz_bytes(1 + i % 9);
                        s.push(b'a' + (i as u8));
                        keys.cstr(&s)
                    })
                    .collect();
                let cmp = if sm == SH_DEFAULT {
                    ElemCmp::Raw
                } else {
                    ElemCmp::StrKey
                };
                let mut p =
                    Pair::new(&c, &r, 16, 8, cmp, format!("posmode{mode}/sm{sm}"));
                p.shmode(sm);
                p.set_default(1);
                for (i, &k) in ks.iter().enumerate() {
                    assert_eq!(p.put(k, mode, i as u64), i as isize);
                }
                for (i, &k) in ks.iter().enumerate() {
                    assert_eq!(p.geti(k, mode), i as isize);
                    assert_eq!(p.geti_ts(k, mode), i as isize);
                }
                // delete with the canonical mode (mode != 1 delete is row 19)
                for &k in &ks {
                    assert_eq!(p.del(k, 0, HM_STRING), 1);
                }
                p.free();
            }
        }
    }
}

/// Row 19 / 35 — `stbds_hmdel_key` with `mode >= 2` on a string map: the
/// re-find after the `memmove` uses the BINARY branch and cannot find the moved
/// key, so `STBDS_ASSERT(slot >= 0)` fires.
#[test]
fn fatal_hmdel_mode2_string_map() {
    let _g = serial();
    assert_fatal_scenario_matches("hmdel_mode2_string_map", SIGABRT);
}

/// Row 19 (non-fatal corner) — deleting the TAIL entry with `mode >= 2` skips
/// the `memmove`/re-find entirely, so it is well defined; it also skips the
/// STRDUP `free` (both libraries leak identically).
#[test]
fn err_hmdel_mode2_tail_only() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for mode in [2 as c_int, 3, 255, c_int::MAX] {
            for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                (c.rand_seed)(2);
                (r.rand_seed)(2);
                let mut keys = Keys::new();
                let k = keys.cstr(b"only_one");
                let cmp = if sm == SH_DEFAULT {
                    ElemCmp::Raw
                } else {
                    ElemCmp::StrKey
                };
                let mut p =
                    Pair::new(&c, &r, 16, 8, cmp, format!("delmode{mode}/sm{sm}"));
                p.shmode(sm);
                p.set_default(1);
                assert_eq!(p.put(k, HM_STRING, 1), 0);
                assert_eq!(p.del(k, 0, mode), 1, "tail delete must report found");
                assert_eq!(p.len(), 0);
                assert_eq!(p.geti(k, HM_STRING), -1);
                p.free();
            }
        }
    }
}

/// G4 — the full `mode` matrix on every entry point that takes one, including
/// values with no valid enum variant.
#[test]
fn err_mode_enum_matrix() {
    let _g = serial();
    let (c, r) = both();
    const MODES: &[c_int] = &[
        c_int::MIN,
        -1000,
        -1,
        0,
        1,
        2,
        3,
        4,
        255,
        256,
        259,
        1000,
        c_int::MAX,
    ];
    unsafe {
        // stbds_shmode_func: mode is truncated to `unsigned char`
        for &mode in MODES {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let elemsize = 16usize;
            let a = (c.shmode_func)(elemsize, mode);
            let b = (r.shmode_func)(elemsize, mode);
            let want = (mode as u32 & 0xFF) as u8;
            assert_eq!(
                (*map_table(a, elemsize)).string.mode,
                want,
                "C string.mode for shmode_func({mode})"
            );
            assert_eq!(
                (*map_table(b, elemsize)).string.mode,
                want,
                "Rust string.mode for shmode_func({mode})"
            );
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        // stbds_hmget_key* on a table-less map accepts every mode without ever
        // touching the key
        let elemsize = 16usize;
        let a0 = (c.hmput_default)(core::ptr::null_mut(), elemsize);
        let b0 = (r.hmput_default)(core::ptr::null_mut(), elemsize);
        for &mode in MODES {
            let mut tc: isize = 5;
            let mut tr: isize = 5;
            (c.hmget_key_ts)(a0, elemsize, core::ptr::null_mut(), 8, &mut tc, mode);
            (r.hmget_key_ts)(b0, elemsize, core::ptr::null_mut(), 8, &mut tr, mode);
            assert_eq!(tc, -1);
            assert_eq!(tr, -1);
        }
        // stbds_hmdel_key on a table-less map likewise
        for &mode in MODES {
            let a = (c.hmdel_key)(a0, elemsize, core::ptr::null_mut(), 8, 0, mode);
            let b = (r.hmdel_key)(b0, elemsize, core::ptr::null_mut(), 8, 0, mode);
            assert_eq!(a, a0);
            assert_eq!(b, b0);
            assert_eq!(map_temp(a, elemsize), 0);
            assert_eq!(map_temp(b, elemsize), 0);
        }
        (c.hmfree_func)((a0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((b0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// stbds_hmput_default / stbds_hmput_key / stbds_shmode_func
// ===========================================================================

/// Row 20 — `stbds_hmput_default(NULL, elemsize)`.
#[test]
fn err_hmput_default_null_a() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [0usize, 1, 8, 16, 64] {
            let a = (c.hmput_default)(core::ptr::null_mut(), elemsize);
            let b = (r.hmput_default)(core::ptr::null_mut(), elemsize);
            assert!(!a.is_null() && !b.is_null());
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            assert_eq!(sc.length, 1);
            assert_eq!(sc.capacity, 4);
            assert!(!sc.has_table);
            // the default element must be zeroed
            assert!(sc.elems[0].iter().all(|&x| x == 0));
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 21 — `stbds_hmput_default` is a no-op when `length != 0` (it must NOT
/// re-zero the default element).
#[test]
fn err_hmput_default_idempotent() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [1usize, 8, 16, 64] {
            let mut a = (c.hmput_default)(core::ptr::null_mut(), elemsize);
            let mut b = (r.hmput_default)(core::ptr::null_mut(), elemsize);
            // write a marker into the default element
            for i in 0..elemsize {
                *elem(a, elemsize, -1).add(i) = 0xA5;
                *elem(b, elemsize, -1).add(i) = 0xA5;
            }
            for _ in 0..4 {
                let a2 = (c.hmput_default)(a, elemsize);
                let b2 = (r.hmput_default)(b, elemsize);
                assert_eq!(a2, a, "C must return the same pointer");
                assert_eq!(b2, b, "Rust must return the same pointer");
                a = a2;
                b = b2;
                let sc = snapshot(a, elemsize, ElemCmp::Raw);
                let sr = snapshot(b, elemsize, ElemCmp::Raw);
                assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
                assert!(
                    sc.elems[0].iter().all(|&x| x == 0xA5),
                    "the default element must not be re-zeroed"
                );
            }
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
        // and the `length == 0` branch: a hand-made array with length 0
        let elemsize = 16usize;
        let raw_c = (c.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 4);
        let raw_r = (r.arrgrowf)(core::ptr::null_mut(), elemsize, 0, 4);
        let a = (c.hmput_default)((raw_c as *mut u8).add(elemsize) as *mut c_void, elemsize);
        let b = (r.hmput_default)((raw_r as *mut u8).add(elemsize) as *mut c_void, elemsize);
        assert_eq!((*hdr_of_map(a, elemsize)).length, 1);
        assert_eq!((*hdr_of_map(b, elemsize)).length, 1);
        (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Row 22 — `stbds_hmput_key(NULL, ...)` bootstraps rather than failing.
#[test]
fn err_hmput_key_null_a() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [8usize, 16, 64] {
            for mode in [HM_BINARY, -3] {
                (c.rand_seed)(1);
                (r.rand_seed)(1);
                let mut key = [0x5Au8; 8];
                let kp = key.as_mut_ptr() as *mut c_void;
                let a = (c.hmput_key)(core::ptr::null_mut(), elemsize, kp, 8, mode);
                let b = (r.hmput_key)(core::ptr::null_mut(), elemsize, kp, 8, mode);
                assert!(!a.is_null() && !b.is_null());
                assert_eq!(map_temp(a, elemsize), 0);
                assert_eq!(map_temp(b, elemsize), 0);
                // stbds_hmput_key only writes `keysize` key bytes; the rest of
                // the element is uninitialised realloc memory until the caller
                // stores a value (which is what the `hmput`/`shput` macros do).
                init_value_region(a, b, elemsize, 8, 0x11);
                let sc = snapshot(a, elemsize, ElemCmp::Raw);
                let sr = snapshot(b, elemsize, ElemCmp::Raw);
                assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
                assert_eq!(sc.length, 2, "default entry + the new entry");
                assert_eq!(sc.slot_count, BUCKET_LENGTH);
                assert_eq!(sc.str_mode, 0, "binary mode => string.mode 0");
                (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
            // mode >= HM_STRING bootstraps with string.mode == SH_DEFAULT
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let mut key: Vec<u8> = b"k\0".to_vec();
            let kp = key.as_mut_ptr() as *mut c_void;
            let a = (c.hmput_key)(core::ptr::null_mut(), elemsize, kp, 8, HM_STRING);
            let b = (r.hmput_key)(core::ptr::null_mut(), elemsize, kp, 8, HM_STRING);
            assert_eq!((*map_table(a, elemsize)).string.mode, SH_DEFAULT as u8);
            assert_eq!((*map_table(b, elemsize)).string.mode, SH_DEFAULT as u8);
            init_value_region(a, b, elemsize, 8, 0x12);
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 23 — `key == NULL` with `mode >= HM_STRING`.
#[test]
fn fatal_hmput_key_null_string_key() {
    let _g = serial();
    assert_fatal_scenario_matches("hmput_key_null_string_key", SIGSEGV);
}

/// Row 24 — `key == NULL`, binary mode, `keysize == 0`: legal.
#[test]
fn err_hmput_key_null_key_keysize0() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [1usize, 8, 16] {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let mut a = (c.hmput_key)(core::ptr::null_mut(), elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            let mut b = (r.hmput_key)(core::ptr::null_mut(), elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            assert_eq!(map_temp(a, elemsize), 0);
            assert_eq!(map_temp(b, elemsize), 0);
            // a second put with the SAME (empty) key must find the same entry
            a = (c.hmput_key)(a, elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            b = (r.hmput_key)(b, elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            assert_eq!(map_temp(a, elemsize), 0);
            assert_eq!(map_temp(b, elemsize), 0);
            // keysize == 0 => nothing at all was written into the element
            init_value_region(a, b, elemsize, 0, 0x13);
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            assert_eq!(sc.length, 2);
            // lookup + delete of the empty key
            let g = (c.hmget_key)(a, elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            let h = (r.hmget_key)(b, elemsize, core::ptr::null_mut(), 0, HM_BINARY);
            assert_eq!(map_temp(g, elemsize), 0);
            assert_eq!(map_temp(h, elemsize), 0);
            let a = (c.hmdel_key)(g, elemsize, core::ptr::null_mut(), 0, 0, HM_BINARY);
            let b = (r.hmdel_key)(h, elemsize, core::ptr::null_mut(), 0, 0, HM_BINARY);
            assert_eq!(map_temp(a, elemsize), 1);
            assert_eq!(map_temp(b, elemsize), 1);
            let sc = snapshot(a, elemsize, ElemCmp::Raw);
            let sr = snapshot(b, elemsize, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            (c.hmfree_func)((a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((b as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// Row 25 — `key == NULL`, binary mode, `keysize > 0`.
#[test]
fn fatal_hmput_key_null_binary_key() {
    let _g = serial();
    assert_fatal_scenario_matches("hmput_key_null_binary_key", SIGSEGV);
}

/// Row 26 — `keysize > elemsize`: the `memcpy` runs past the element. Both
/// libraries must behave identically (compared in a child process, because the
/// overrun corrupts the heap).
#[test]
fn err_keysize_gt_elemsize() {
    let _g = serial();
    assert_scenario_matches("keysize_gt_elemsize");
}

/// Rows 27, 33, 37, 39 — the `STBDS_ASSERT`s that are unreachable through the
/// public API. A 20 000-operation randomised put/delete soak: if any of them
/// fired, the child would die on SIGABRT instead of exiting 0.
#[test]
fn err_unreachable_asserts_never_fire() {
    let _g = serial();
    for seed in [1i64, 2, 7, 12345] {
        let out = assert_scenario_matches(&format!("assert_soak:{seed}"));
        assert_eq!(
            out.code,
            Some(0),
            "an unreachable STBDS_ASSERT fired: {}",
            out.describe()
        );
        assert_eq!(out.signal, None);
        assert!(out.stdout.starts_with(b"survived length="));
    }
}

/// Row 28 — `stbds_shmode_func` truncates `mode` to `unsigned char`; values
/// that land outside `{1,2,3}` make `stbds_hmput_key`'s `switch` fall to
/// `default:` and `memcpy` raw bytes instead of storing a `char*`.
#[test]
fn err_shmode_out_of_range() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x2828_2828);
    unsafe {
        for mode in [
            -1 as c_int,
            0,
            4,
            5,
            100,
            255,
            256,
            257,
            258,
            259,
            260,
            511,
            512,
            c_int::MIN,
            c_int::MAX,
        ] {
            let truncated = (mode as u32 & 0xFF) as u8;
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let elemsize = 16usize;
            let a = (c.shmode_func)(elemsize, mode);
            let b = (r.shmode_func)(elemsize, mode);
            assert_eq!((*map_table(a, elemsize)).string.mode, truncated);
            assert_eq!((*map_table(b, elemsize)).string.mode, truncated);

            // modes that truncate to a real variant behave as that variant;
            // everything else takes the `default:` memcpy path (insert-only,
            // since a later lookup would reinterpret the copied text as char*)
            let is_variant = matches!(truncated, 1 | 2 | 3);
            let cmp = if truncated == 2 || truncated == 3 {
                ElemCmp::StrKey
            } else {
                ElemCmp::Raw
            };
            let mut p = Pair::new(&c, &r, elemsize, 8, cmp, format!("shmode{mode}"));
            p.c.t = a;
            p.r.t = b;
            p.set_default(1);
            let mut keys = Keys::new();
            for i in 0..4usize {
                let mut s = rng.nz_bytes(12);
                s[0] = b'a' + i as u8;
                let k = keys.cstr(&s);
                let idx = p.put(k, HM_STRING, i as u64);
                assert_eq!(idx, i as isize);
                if !is_variant {
                    // the element holds the key TEXT, not a pointer
                    let raw = core::slice::from_raw_parts(elem(p.c.t, elemsize, idx), 8);
                    assert_eq!(raw, &s[..8], "default: memcpy of key text");
                }
            }
            if is_variant {
                for i in 0..4usize {
                    let _ = i;
                }
            }
            p.free();
        }
    }
}

/// Row 29 — `stbds_shmode_func(0, mode)`: `elemsize == 0`.
#[test]
fn err_shmode_elemsize0() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let a = (c.shmode_func)(0, sm);
            let b = (r.shmode_func)(0, sm);
            assert!(!a.is_null() && !b.is_null());
            // elemsize 0 => the hash pointer aliases the array base
            assert_eq!((*hdr_of_map(a, 0)).length, 1);
            assert_eq!((*hdr_of_map(b, 0)).length, 1);
            assert_eq!((*hdr_of_map(a, 0)).capacity, 4);
            assert_eq!((*hdr_of_map(b, 0)).capacity, 4);
            let sc = snapshot(a, 0, ElemCmp::Raw);
            let sr = snapshot(b, 0, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            (c.hmfree_func)(a, 0);
            (r.hmfree_func)(b, 0);
        }
    }
}

// ===========================================================================
// stbds_hmdel_key
// ===========================================================================

/// Row 30 — `stbds_hmdel_key(NULL, ...)` returns NULL (the sentinel `shdel`
/// tests).
#[test]
fn err_hmdel_null_a() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for elemsize in [0usize, 1, 16, 64] {
            for mode in [HM_BINARY, HM_STRING, -1, 99] {
                for keyoffset in [0usize, 8, usize::MAX] {
                    let a = (c.hmdel_key)(
                        core::ptr::null_mut(),
                        elemsize,
                        core::ptr::null_mut(),
                        8,
                        keyoffset,
                        mode,
                    );
                    let b = (r.hmdel_key)(
                        core::ptr::null_mut(),
                        elemsize,
                        core::ptr::null_mut(),
                        8,
                        keyoffset,
                        mode,
                    );
                    assert!(a.is_null(), "C must return NULL");
                    assert!(b.is_null(), "Rust must return NULL");
                }
            }
        }
    }
}

/// Row 31 — `hash_table == 0`: sets `stbds_temp = 0` and returns `a`.
#[test]
fn err_hmdel_no_table() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        let elemsize = 16usize;
        let a0 = (c.hmput_default)(core::ptr::null_mut(), elemsize);
        let b0 = (r.hmput_default)(core::ptr::null_mut(), elemsize);
        (*hdr_of_map(a0, elemsize)).temp = 0x1234;
        (*hdr_of_map(b0, elemsize)).temp = 0x1234;
        let mut key: Vec<u8> = b"nope\0".to_vec();
        let kp = key.as_mut_ptr() as *mut c_void;
        for mode in [HM_BINARY, HM_STRING] {
            let a = (c.hmdel_key)(a0, elemsize, kp, 8, 0, mode);
            let b = (r.hmdel_key)(b0, elemsize, kp, 8, 0, mode);
            assert_eq!(a, a0);
            assert_eq!(b, b0);
            assert_eq!(map_temp(a, elemsize), 0, "temp must be reset to 0");
            assert_eq!(map_temp(b, elemsize), 0);
        }
        (c.hmfree_func)((a0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((b0 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Row 32 — key absent: `temp == 0`, nothing else changes.
#[test]
fn err_hmdel_missing_key() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3232_9999);
    unsafe {
        for n in [1usize, 5, 6, 12, 30] {
            (c.rand_seed)(8);
            (r.rand_seed)(8);
            let mut keys = Keys::new();
            let live: Vec<_> = (0..n).map(|_| keys.raw(&rng.bytes(8))).collect();
            let absent: Vec<_> = (0..16).map(|_| keys.raw(&rng.bytes(8))).collect();
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("delmiss/n{n}"));
            p.set_default(1);
            for (i, &k) in live.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            let before = p.c.snap(ElemCmp::Raw);
            for &k in &absent {
                assert_eq!(p.del(k, 0, HM_BINARY), 0, "absent delete must report 0");
            }
            let after = p.c.snap(ElemCmp::Raw);
            assert_eq!(before.length, after.length);
            assert_eq!(before.used_count, after.used_count);
            assert_eq!(before.tomb_count, after.tomb_count);
            assert_eq!(before.slot_count, after.slot_count);
            p.free();
        }
    }
}

/// Row 34 — `STBDS_ASSERT(table->used_count >= 0)` on a `size_t` is a tautology:
/// even after `--used_count` wraps past zero it cannot fire.
#[test]
fn err_hmdel_used_count_tautology() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3434_3434);
    unsafe {
        (c.rand_seed)(1);
        (r.rand_seed)(1);
        let elemsize = 16usize;
        let mut keys = Keys::new();
        let ks: Vec<_> = (0..4).map(|_| keys.raw(&rng.bytes(8))).collect();
        let mut p = Pair::new(&c, &r, elemsize, 8, ElemCmp::Raw, "tautology");
        p.set_default(1);
        for (i, &k) in ks.iter().enumerate() {
            p.put(k, HM_BINARY, i as u64);
        }
        // forge used_count = 0 so the delete's `--used_count` wraps to SIZE_MAX
        (*map_table(p.c.t, elemsize)).used_count = 0;
        (*map_table(p.r.t, elemsize)).used_count = 0;
        // delete the TAIL entry so no memmove / re-find assert is involved
        assert_eq!(p.del(ks[3], 0, HM_BINARY), 1);
        let sc = p.c.snap(ElemCmp::Raw);
        assert_eq!(sc.used_count, usize::MAX, "used_count wrapped as expected");
        assert_eq!(p.r.snap(ElemCmp::Raw).used_count, usize::MAX);
        // and the process is still alive => the assert did not fire
        p.free();
    }
}

/// Row 36 — `STBDS_ASSERT(b->index[i] == final_index)` fires when the tail
/// element's key has been made to duplicate a live key behind the library's back.
#[test]
fn fatal_hmdel_corrupted_index() {
    let _g = serial();
    assert_fatal_scenario_matches("hmdel_corrupted_index", SIGABRT);
}

/// Row 38 — at `slot_count == STBDS_BUCKET_LENGTH` the shrink threshold is
/// forced to 0, so no amount of deleting ever shrinks the table.
#[test]
fn err_no_shrink_at_min_slots() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x3838_3838);
    unsafe {
        for n in 1..=5usize {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let mut keys = Keys::new();
            let ks: Vec<_> = (0..n).map(|_| keys.raw(&rng.bytes(8))).collect();
            let mut p = Pair::new(&c, &r, 16, 8, ElemCmp::Raw, format!("noshrink/n{n}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            let s = p.c.snap(ElemCmp::Raw);
            assert_eq!(s.slot_count, BUCKET_LENGTH);
            assert_eq!(s.uc_shrink_threshold, 0);
            for &k in &ks {
                p.del(k, 0, HM_BINARY);
                let s = p.c.snap(ElemCmp::Raw);
                assert_eq!(s.slot_count, BUCKET_LENGTH, "must never shrink below 8");
            }
            p.free();
        }
    }
}

/// G5 — `keyoffset` one past the element: the key is read out of the NEXT
/// element, so nothing matches and the delete reports "not found".
#[test]
fn err_keyoffset_past_element() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x9595_9595);
    unsafe {
        for n in [1usize, 3, 6, 12] {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let elemsize = 16usize;
            let mut keys = Keys::new();
            let ks: Vec<_> = (0..n).map(|_| keys.raw(&rng.bytes(8))).collect();
            let mut p = Pair::new(&c, &r, elemsize, 8, ElemCmp::Raw, format!("koff/n{n}"));
            p.set_default(1);
            for (i, &k) in ks.iter().enumerate() {
                p.put(k, HM_BINARY, i as u64);
            }
            // zero the whole capacity slack so the read past the last element is
            // deterministic on both sides
            for m in [&p.c, &p.r] {
                let hdr = hdr_of_map(m.t, elemsize);
                let base = (m.t as *mut u8).sub(elemsize);
                for off in ((*hdr).length * elemsize)..((*hdr).capacity * elemsize) {
                    *base.add(off) = 0;
                }
            }
            for &k in &ks {
                assert_eq!(
                    p.del(k, elemsize, HM_BINARY),
                    0,
                    "keyoffset past the element must never match"
                );
            }
            assert_eq!(p.len(), n as isize, "nothing may be deleted");
            p.free();
        }
    }
}

// ===========================================================================
// stbds_stralloc / stbds_strreset
// ===========================================================================

/// Row 40 — `stbds_stralloc(NULL, str)`.
#[test]
fn fatal_stralloc_null_arena() {
    let _g = serial();
    assert_fatal_scenario_matches("stralloc_null_arena", SIGSEGV);
}

/// Row 41 — `stbds_stralloc(&a, NULL)`.
#[test]
fn fatal_stralloc_null_str() {
    let _g = serial();
    assert_fatal_scenario_matches("stralloc_null_str", SIGSEGV);
}

/// Row 42 — forged `remaining` with `storage == NULL`: the assert passes, then
/// `a->storage->storage` faults.
#[test]
fn fatal_stralloc_forged_remaining() {
    let _g = serial();
    assert_fatal_scenario_matches("stralloc_forged_remaining", SIGSEGV);
}

/// Row 43 — `a->block` large enough that `512 << (block>>1)` shifts by >= 64
/// (C shift-count UB; x86 masks the count, and so must Rust).
#[test]
fn err_stralloc_block_shift_overflow() {
    let _g = serial();
    let (c, r) = both();
    // `blocksize = (size_t)512u << (block>>1)`. In C the shift count is UB once
    // it reaches 64; x86-64 `shl` masks it to 6 bits and Rust's `wrapping_shl`
    // masks identically, so both must agree.
    //
    // Only `block` values whose resulting `blocksize` is actually allocatable
    // are exercised in-process:
    //   * block 0..=25       => shift 0..12,  blocksize 512 .. 2 MiB
    //   * block 110..=127    => shift 55..63, blocksize wraps to 0 (=> big block)
    //   * block 128..=153    => shift count >= 64 => MASKED back to 0..13
    //   * block 238..=255    => shift count >= 64 AND blocksize wraps to 0
    // The ranges in between demand petabyte blocks; that path fails to allocate
    // and then dereferences NULL, and is covered by `fatal_stralloc_huge_block`.
    let mut blocks: Vec<u8> = Vec::new();
    blocks.extend(0u8..=25);
    blocks.extend(110u8..=127);
    blocks.extend(128u8..=153);
    blocks.extend(238u8..=255);
    unsafe {
        for block in blocks {
            let shift = (block >> 1) & 63;
            let expect_bs = 512usize.wrapping_shl(shift as u32);
            for text in [
                &b"shift_overflow_probe"[..],
                &b""[..],
                &b"x"[..],
            ] {
                let mut ac = StringArena::zeroed();
                let mut ar = StringArena::zeroed();
                ac.block = block;
                ar.block = block;
                let mut s: Vec<u8> = text.to_vec();
                s.push(0);
                let len = s.len();
                let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                assert_eq!(
                    cstr_bytes(pc),
                    cstr_bytes(pr),
                    "returned string diverged for block={block}"
                );
                assert_eq!(
                    cstr_bytes(pc).as_deref(),
                    Some(text),
                    "wrong string for block={block}"
                );
                assert_eq!(
                    (ac.remaining, ac.block),
                    (ar.remaining, ar.block),
                    "arena state diverged for block={block} (shift={shift}, bs={expect_bs}): C=({}, {}) Rust=({}, {})",
                    ac.remaining,
                    ac.block,
                    ar.remaining,
                    ar.block
                );
                // cross-check against the masked-shift model
                if len > expect_bs {
                    assert_eq!(ac.remaining, 0, "big-block path for block={block}");
                } else {
                    assert_eq!(
                        ac.remaining,
                        expect_bs - len,
                        "normal-block path for block={block} (bs={expect_bs})"
                    );
                }
                (c.strreset)(&mut ac);
                (r.strreset)(&mut ar);
            }
        }
    }
}

/// Row 43b — the `block` values that demand an unallocatable block: `realloc`
/// returns NULL and `sb->next = a->storage` faults. Fatal, identically.
#[test]
fn fatal_stralloc_huge_block() {
    let _g = serial();
    // block 108/109 => shift 54 => blocksize 2^63 (8 EiB)
    for block in [108i64, 109, 236, 237] {
        assert_fatal_scenario_matches(&format!("stralloc_huge_block:{block}"), SIGSEGV);
    }
}

/// Row 44 — `++a->block` is skipped once `blocksize >= 1<<20`.
#[test]
fn err_stralloc_block_saturates() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for block in [20u8, 21, 22, 23, 24, 25] {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            ac.block = block;
            ar.block = block;
            let mut s: Vec<u8> = b"saturation_probe\0".to_vec();
            let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), cstr_bytes(pr));
            assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block));
            let bs = 512usize << (block >> 1);
            let expect = if bs < (1 << 20) { block + 1 } else { block };
            assert_eq!(ac.block, expect, "block saturation at block={block} bs={bs}");
            assert_eq!(ar.block, expect);
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

/// Rows 45, 46 — the big-block path with `storage == NULL` and with
/// `storage != NULL`.
#[test]
fn err_stralloc_bigblock_paths() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4546_4546);
    unsafe {
        // row 45: storage == NULL
        for len in [512usize, 513, 1024, 65536] {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            let mut s = rng.nz_bytes(len);
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), cstr_bytes(pr));
            assert_eq!(ac.remaining, 0, "big block leaves remaining == 0");
            assert_eq!(ar.remaining, 0);
            assert_eq!(ac.block, 1);
            assert_eq!(ar.block, 1);
            // storage == the new block, and next == NULL
            assert!(!ac.storage.is_null() && !ar.storage.is_null());
            assert!((*ac.storage).next.is_null());
            assert!((*ar.storage).next.is_null());
            assert_eq!(pc as usize - ac.storage as usize, STRING_BLOCK_HDR);
            assert_eq!(pr as usize - ar.storage as usize, STRING_BLOCK_HDR);
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
        // row 46: storage != NULL => spliced AFTER the head, remaining untouched
        for len in [600usize, 2048, 70000] {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            let mut small: Vec<u8> = b"seed\0".to_vec();
            (c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
            let head_c = ac.storage;
            let head_r = ar.storage;
            let rem = ac.remaining;
            let mut s = rng.nz_bytes(len);
            s.push(0);
            let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(pc), cstr_bytes(pr));
            assert_eq!(ac.remaining, rem, "remaining must be preserved");
            assert_eq!(ar.remaining, rem);
            assert_eq!(ac.storage, head_c, "the head must not change");
            assert_eq!(ar.storage, head_r);
            // the big block is the head's `next`
            assert_eq!(
                pc as usize - (*head_c).next as usize,
                STRING_BLOCK_HDR,
                "big block must be spliced right after the head"
            );
            assert_eq!(pr as usize - (*head_r).next as usize, STRING_BLOCK_HDR);
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

/// Rows 47, 48 — `len == remaining` (fits) and `len == remaining + 1` (new
/// block), the two sides of the `len > a->remaining` boundary.
#[test]
fn err_stralloc_fit_boundary() {
    let _g = serial();
    let (c, r) = both();
    let mut rng = Rng::new(0x4748_4748);
    unsafe {
        for first in [0usize, 1, 100, 400, 510] {
            for delta in [-1isize, 0, 1] {
                let mut ac = StringArena::zeroed();
                let mut ar = StringArena::zeroed();
                let mut f = rng.nz_bytes(first);
                f.push(0);
                (c.stralloc)(&mut ac, f.as_mut_ptr() as *mut c_char);
                (r.stralloc)(&mut ar, f.as_mut_ptr() as *mut c_char);
                let rem = ac.remaining;
                assert_eq!(rem, ar.remaining);
                let want_len = rem as isize + delta;
                if want_len <= 0 {
                    continue;
                }
                let mut s = rng.nz_bytes(want_len as usize - 1);
                s.push(0);
                let pc = (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let pr = (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                assert_eq!(cstr_bytes(pc), cstr_bytes(pr));
                assert_eq!(
                    (ac.remaining, ac.block),
                    (ar.remaining, ar.block),
                    "boundary first={first} delta={delta} rem={rem}"
                );
                if delta <= 0 {
                    assert_eq!(ac.remaining, rem - want_len as usize);
                    assert_eq!(ac.block, 1, "no new block for a fitting string");
                } else {
                    assert_eq!(ac.block, 2, "one step past the fit needs a new block");
                }
                (c.strreset)(&mut ac);
                (r.strreset)(&mut ar);
            }
        }
    }
}

/// Row 49 — `stbds_strreset(NULL)`.
#[test]
fn fatal_strreset_null_arena() {
    let _g = serial();
    assert_fatal_scenario_matches("strreset_null_arena", SIGSEGV);
}

/// Row 50 — `stbds_strreset` on an empty / already-reset arena is idempotent.
#[test]
fn err_strreset_empty_idempotent() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        // pre-set the non-pointer fields so we can see them being cleared
        ac.remaining = 999;
        ar.remaining = 999;
        ac.block = 7;
        ar.block = 7;
        ac.mode = 3;
        ar.mode = 3;
        for _ in 0..5 {
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0));
            assert_eq!((ar.remaining, ar.block, ar.mode), (0, 0, 0));
            assert!(ac.storage.is_null() && ar.storage.is_null());
        }
    }
}

// ===========================================================================
// strkey / sh_geti
// ===========================================================================

/// Row 51 — `strkey` at the `int` extremes never overflows the 256-byte buffer.
#[test]
fn err_strkey_extremes() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        for n in [c_int::MIN, c_int::MIN + 1, -1, 0, 1, c_int::MAX - 1, c_int::MAX] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let sc = cstr_bytes(pc).unwrap();
            let sr = cstr_bytes(pr).unwrap();
            assert_eq!(sc, sr, "strkey({n})");
            assert_eq!(sc, format!("test_{n}").into_bytes());
            assert!(sc.len() < 256, "must fit the static buffer");
            let bc = core::slice::from_raw_parts(pc as *const u8, 256);
            let br = core::slice::from_raw_parts(pr as *const u8, 256);
            assert_eq!(bc, br, "full static buffer for strkey({n})");
        }
    }
}

/// Rows 52, 53 — `sh_geti` with non-positive `num`: every loop is skipped and
/// the three `shgeti(strmap,"foo") == -1` asserts still hold.
#[test]
fn err_sh_geti_nonpositive() {
    let _g = serial();
    for num in [0i32, -1, -3, -1000, c_int::MIN] {
        let out = assert_scenario_matches(&format!("sh_geti:{num}"));
        assert_eq!(out.code, Some(0), "sh_geti({num}): {}", out.describe());
        assert_eq!(out.signal, None);
        assert!(out.stdout.is_empty());
    }
}

/// Row 44b — the fatal half of `SH_NONE` + `mode >= HM_STRING`: the lookup
/// dereferences the copied key text as a `char*`.
#[test]
fn fatal_sh_none_string_lookup() {
    let _g = serial();
    assert_fatal_scenario_matches("sh_none_string_lookup", SIGSEGV);
}

// ===========================================================================
// G2 — zero lengths everywhere
// ===========================================================================

/// G2 — `0` for every `size_t` length parameter on every entry point.
#[test]
fn err_zero_lengths_matrix() {
    let _g = serial();
    let (c, r) = both();
    unsafe {
        // stbds_arrgrowf: elemsize 0, addlen 0, min_cap 0 (all combinations)
        for elemsize in [0usize, 1] {
            for addlen in [0usize, 1] {
                for min_cap in [0usize, 1] {
                    let a = (c.arrgrowf)(core::ptr::null_mut(), elemsize, addlen, min_cap);
                    let b = (r.arrgrowf)(core::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_eq!(a.is_null(), b.is_null(), "es={elemsize} al={addlen} mc={min_cap}");
                    if !a.is_null() {
                        assert_eq!((*hdr_of_arr(a)).capacity, (*hdr_of_arr(b)).capacity);
                        assert_eq!((*hdr_of_arr(a)).length, (*hdr_of_arr(b)).length);
                        (c.arrfreef)(a);
                        (r.arrfreef)(b);
                    }
                }
            }
        }
        // stbds_hash_bytes with len 0 and a real pointer
        let buf = [1u8, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(
            (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, 5),
            (r.hash_bytes)(buf.as_ptr() as *mut c_void, 0, 5)
        );
        // stbds_hash_string on the empty string
        let mut empty = [0u8; 1];
        assert_eq!(
            (c.hash_string)(empty.as_mut_ptr() as *mut c_char, 5),
            (r.hash_string)(empty.as_mut_ptr() as *mut c_char, 5)
        );
        // stbds_hmput_default / stbds_shmode_func / stbds_hmput_key with elemsize 0
        for &sm in &[SH_NONE, SH_DEFAULT] {
            (c.rand_seed)(1);
            (r.rand_seed)(1);
            let a = (c.shmode_func)(0, sm);
            let b = (r.shmode_func)(0, sm);
            let sc = snapshot(a, 0, ElemCmp::Raw);
            let sr = snapshot(b, 0, ElemCmp::Raw);
            assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
            (c.hmfree_func)(a, 0);
            (r.hmfree_func)(b, 0);
        }
        // stbds_hmput_key with keysize 0 (covered in depth by row 24)
        (c.rand_seed)(1);
        (r.rand_seed)(1);
        let key = [0u8; 8];
        let a = (c.hmput_key)(core::ptr::null_mut(), 8, key.as_ptr() as *mut c_void, 0, HM_BINARY);
        let b = (r.hmput_key)(core::ptr::null_mut(), 8, key.as_ptr() as *mut c_void, 0, HM_BINARY);
        // keysize == 0 => the whole element is uninitialised realloc memory
        init_value_region(a, b, 8, 0, 0x14);
        let sc = snapshot(a, 8, ElemCmp::Raw);
        let sr = snapshot(b, 8, ElemCmp::Raw);
        assert_eq!(sc, sr, "{}", diff_snaps(&sc, &sr));
        (c.hmfree_func)((a as *mut u8).sub(8) as *mut c_void, 8);
        (r.hmfree_func)((b as *mut u8).sub(8) as *mut c_void, 8);
        // stbds_hmfree_func with elemsize 0 on a real map
        (c.rand_seed)(1);
        (r.rand_seed)(1);
        let a = (c.shmode_func)(16, SH_ARENA);
        let b = (r.shmode_func)(16, SH_ARENA);
        (c.hmfree_func)((a as *mut u8).sub(16) as *mut c_void, 0);
        (r.hmfree_func)((b as *mut u8).sub(16) as *mut c_void, 0);
        // stbds_stralloc with the empty string
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut e = [0u8; 1];
        let pc = (c.stralloc)(&mut ac, e.as_mut_ptr() as *mut c_char);
        let pr = (r.stralloc)(&mut ar, e.as_mut_ptr() as *mut c_char);
        assert_eq!(cstr_bytes(pc), Some(Vec::new()));
        assert_eq!(cstr_bytes(pr), Some(Vec::new()));
        assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block));
        assert_eq!(ac.remaining, 511);
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}
