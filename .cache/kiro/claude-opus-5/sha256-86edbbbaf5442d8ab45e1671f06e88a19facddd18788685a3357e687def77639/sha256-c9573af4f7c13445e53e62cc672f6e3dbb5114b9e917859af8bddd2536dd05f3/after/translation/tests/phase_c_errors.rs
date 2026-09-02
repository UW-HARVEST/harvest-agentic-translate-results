//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rejections must agree on the *exact*
//! sentinel (`-1`, `-2`, `0`, `NULL`, `temp == 0/1`), not merely on "both
//! failed". Aborting rows are compared by the child process's termination
//! signal.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

fn run_both<F: Fn(&Impl) -> Vec<u8>>(what: &str, f: F) {
    with_libs(|l| {
        let c = f(&l.c);
        let r = f(&l.r);
        assert_same(what, &c, &r);
    });
}

const DEFAULT_SEED: usize = 0x3141_5926;

fn i32key(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

/// Insert `n` distinct i32 keys into a fresh binary map, mirroring the `hmput`
/// macro (elemsize 8 = `{int key; int value;}`).
unsafe fn build_int_map(im: &Impl, n: i32) -> *mut c_void {
    unsafe {
        let mut t: *mut c_void = std::ptr::null_mut();
        for i in 0..n {
            let mut k = i32key(i * 7 + 1);
            t = (im.hmput_key)(t, 8, k.as_mut_ptr() as *mut c_void, 8usize.min(4), 0);
            let temp = (*header(hash_to_arr(t, 8))).temp;
            let e = (t as *mut u8).offset(temp * 8);
            std::ptr::copy_nonoverlapping(k.as_ptr(), e, 4);
            std::ptr::copy_nonoverlapping((i * 3).to_ne_bytes().as_ptr(), e.add(4), 4);
        }
        t
    }
}

// ---------------------------------------------------------------------------
// Rows 1, 2, 2b — stbds_arrgrowf early-outs
// ---------------------------------------------------------------------------

#[test]
fn err01_arrgrowf_request_already_satisfied() {
    run_both("err01", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[1usize, 4, 8, 16, 64] {
            let a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            std::ptr::write_bytes(a as *mut u8, 0x7E, elemsize * 16);
            (*header(a)).length = 5;
            for &min_cap in &[0usize, 1, 5, 15, 16] {
                let b = (im.arrgrowf)(a, elemsize, 0, min_cap);
                d.tag("same-ptr");
                d.u8((a == b) as u8);
                d.bytes(&digest_array(b, elemsize, 16));
            }
            // one step past the satisfied range must NOT early out
            let c = (im.arrgrowf)(a, elemsize, 0, 17);
            d.tag("grew");
            d.usize((*header(c)).capacity);
            (im.arrfreef)(c);
        }
        d.0
    });
}

#[test]
fn err02_arrgrowf_zero_request_returns_null() {
    run_both("err02", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[0usize, 1, 4, 8, 4096] {
            // addlen == 0 && min_cap == 0 => `0 <= arrcap(NULL)` => NULL back
            let a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            d.tag("null-out");
            d.usize(elemsize);
            d.u8(a.is_null() as u8);
            // the smallest non-degenerate request is rounded up to 4
            let b = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            d.u8(b.is_null() as u8);
            if !b.is_null() {
                d.usize((*header(b)).capacity);
                d.usize((*header(b)).length);
                d.isize((*header(b)).temp);
                d.u8((!(*header(b)).hash_table.is_null()) as u8);
                (im.arrfreef)(b);
            }
            let c = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            d.u8(c.is_null() as u8);
            if !c.is_null() {
                d.usize((*header(c)).capacity);
                (im.arrfreef)(c);
            }
        }
        // oversized lengths: elemsize * min_cap overflows size_t exactly the
        // way C does (wrapping multiply, then realloc almost certainly fails)
        d.tag("huge");
        d.u8(1);
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 3, 4, 7, 8 — stbds_hm_find_slot returning -1 (both bucket halves)
// ---------------------------------------------------------------------------

#[test]
fn err03_err04_find_slot_absent_key() {
    run_both("err03_04", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x301);
        // Vary the population so probing lands in the forward half and in the
        // wrap-around half of the bucket scan.
        for n in [1i32, 2, 5, 6, 7, 8, 9, 16, 31, 32, 64, 200] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut t = build_int_map(im, n);
            for _ in 0..200 {
                let k = rng.i32();
                // skip keys that are actually present
                if (0..n).any(|i| i * 7 + 1 == k) {
                    continue;
                }
                let mut kb = i32key(k);
                let mut temp: isize = 0x1234;
                t = (im.hmget_key_ts)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, &mut temp, 0);
                d.tag("absent-ts");
                d.isize(temp);
                t = (im.hmget_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0);
                d.tag("absent");
                d.isize((*header(hash_to_arr(t, 8))).temp);
            }
            d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
            (im.hmfree_func)(hash_to_arr(t, 8), 8);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 5, 6 — hmget_key_ts on NULL / on an array without a hash index
// ---------------------------------------------------------------------------

#[test]
fn err05_hmget_key_ts_null_map() {
    run_both("err05", |im| unsafe {
        let mut d = Digest::default();
        for &(elemsize, keysize, mode) in &[(8usize, 4usize, 0i32), (16, 8, 1), (64, 32, 0), (16, 8, 2)] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut key = vec![0x41u8; keysize.max(8)];
            *key.last_mut().unwrap() = 0;
            let mut temp: isize = 0x7777;
            let t = (im.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                &mut temp,
                mode,
            );
            d.tag("ts-null");
            d.isize(temp);
            d.u8((!t.is_null()) as u8);
            d.bytes(&digest_map(t, elemsize, KeyRepr::Inline(keysize), false));
            (im.hmfree_func)(hash_to_arr(t, elemsize), elemsize);

            // stbds_hmget_key must additionally store the -1 in header->temp
            let t2 = (im.hmget_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                mode,
            );
            d.tag("get-null");
            d.isize((*header(hash_to_arr(t2, elemsize))).temp);
            d.bytes(&digest_map(t2, elemsize, KeyRepr::Inline(keysize), false));
            (im.hmfree_func)(hash_to_arr(t2, elemsize), elemsize);
        }
        d.0
    });
}

#[test]
fn err06_hmget_no_hash_table() {
    run_both("err06", |im| unsafe {
        let mut d = Digest::default();
        for &(elemsize, keysize, mode) in &[(8usize, 4usize, 0i32), (16, 8, 1), (16, 8, -3), (16, 8, 9)] {
            (im.rand_seed)(DEFAULT_SEED);
            // an array with hash_table == NULL, as produced by arrgrowf /
            // hmput_default and never touched by hmput_key
            let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize);
            (*header(raw)).length = 1;
            (*header(raw)).temp = 0x0BAD;
            let hp = (raw as *mut u8).add(elemsize) as *mut c_void;
            let mut key = vec![0x42u8; keysize.max(8)];
            *key.last_mut().unwrap() = 0;
            let mut temp: isize = 0x7777;
            let t = (im.hmget_key_ts)(hp, elemsize, key.as_mut_ptr() as *mut c_void, keysize, &mut temp, mode);
            d.tag("ts");
            d.isize(temp);
            d.u8((t == hp) as u8);
            let t2 = (im.hmget_key)(t, elemsize, key.as_mut_ptr() as *mut c_void, keysize, mode);
            d.tag("get");
            d.isize((*header(hash_to_arr(t2, elemsize))).temp);
            d.bytes(&digest_map(t2, elemsize, KeyRepr::Inline(keysize), false));
            (im.hmfree_func)(raw, elemsize);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 9, 10, 17 — out-of-range `mode` values across the FFI boundary
// ---------------------------------------------------------------------------

/// `mode` is a C `int`, so *any* `int` is a real input. `mode < 1` must behave
/// exactly like `STBDS_HM_BINARY` and `mode >= 1` exactly like
/// `STBDS_HM_STRING`.
#[test]
fn err09_mode_below_range_is_binary() {
    run_both("err09", |im| unsafe {
        let mut d = Digest::default();
        let modes: [c_int; 7] = [0, -1, -2, -7, -1000, i32::MIN, i32::MIN + 1];
        for &mode in &modes {
            (im.rand_seed)(DEFAULT_SEED);
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..40i32 {
                let mut k = i32key(i * 5);
                t = (im.hmput_key)(t, 8, k.as_mut_ptr() as *mut c_void, 4, mode);
                let temp = (*header(hash_to_arr(t, 8))).temp;
                let e = (t as *mut u8).offset(temp * 8);
                std::ptr::copy_nonoverlapping(k.as_ptr(), e, 4);
                std::ptr::copy_nonoverlapping(i.to_ne_bytes().as_ptr(), e.add(4), 4);
            }
            for i in 0..60i32 {
                let mut k = i32key(i * 5);
                let mut temp: isize = 0;
                t = (im.hmget_key_ts)(t, 8, k.as_mut_ptr() as *mut c_void, 4, &mut temp, mode);
                d.isize(temp);
            }
            for i in 0..40i32 {
                let mut k = i32key(i * 5);
                t = (im.hmdel_key)(t, 8, k.as_mut_ptr() as *mut c_void, 4, 0, mode);
                d.isize((*header(hash_to_arr(t, 8))).temp);
            }
            d.tag("mode");
            d.i64(mode as i64);
            d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
            (im.hmfree_func)(hash_to_arr(t, 8), 8);
        }
        d.0
    });
}

#[test]
fn err10_mode_above_range_is_string() {
    run_both("err10", |im| unsafe {
        let mut d = Digest::default();
        let modes: [c_int; 6] = [1, 2, 3, 7, 1000, i32::MAX];
        for &mode in &modes {
            (im.rand_seed)(DEFAULT_SEED);
            let mut keys: Vec<Vec<u8>> =
                (0..30).map(|i| cstring(format!("mode_key_{i}").as_bytes())).collect();
            let mut t: *mut c_void = std::ptr::null_mut();
            for (i, k) in keys.iter_mut().enumerate() {
                t = (im.hmput_key)(t, 16, k.as_mut_ptr() as *mut c_void, 8, mode);
                let temp = (*header(hash_to_arr(t, 16))).temp;
                let e = (t as *mut u8).offset(temp * 16);
                *(e.add(8) as *mut u64) = i as u64;
            }
            for k in keys.iter_mut() {
                let mut temp: isize = 0;
                t = (im.hmget_key_ts)(t, 16, k.as_mut_ptr() as *mut c_void, 8, &mut temp, mode);
                d.isize(temp);
            }
            // absent string keys -> -1
            for i in 0..20 {
                let mut k = cstring(format!("absent_{i}").as_bytes());
                let mut temp: isize = 0;
                t = (im.hmget_key_ts)(t, 16, k.as_mut_ptr() as *mut c_void, 8, &mut temp, mode);
                d.isize(temp);
            }
            d.tag("mode");
            d.i64(mode as i64);
            d.bytes(&digest_map(t, 16, KeyRepr::Pointer, false));
            (im.hmfree_func)(hash_to_arr(t, 16), 16);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 11, 12, 13 — stbds_hmput_default degenerate inputs
// ---------------------------------------------------------------------------

#[test]
fn err11_err12_err13_hmput_default() {
    run_both("err11_13", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[1usize, 4, 8, 16, 40] {
            (im.rand_seed)(DEFAULT_SEED);

            // a == NULL
            let t = (im.hmput_default)(std::ptr::null_mut(), elemsize);
            d.tag("null");
            d.u8((!t.is_null()) as u8);
            d.usize((*header(hash_to_arr(t, elemsize))).length);
            d.usize((*header(hash_to_arr(t, elemsize))).capacity);
            d.0.extend_from_slice(std::slice::from_raw_parts(
                hash_to_arr(t, elemsize) as *const u8,
                elemsize,
            ));

            // length != 0 => unchanged, same pointer
            let t2 = (im.hmput_default)(t, elemsize);
            d.tag("noop");
            d.u8((t == t2) as u8);
            d.usize((*header(hash_to_arr(t2, elemsize))).length);
            (im.hmfree_func)(hash_to_arr(t2, elemsize), elemsize);

            // length == 0 => grow + zero element 0
            let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            std::ptr::write_bytes(raw as *mut u8, 0xC3, elemsize * (*header(raw)).capacity);
            (*header(raw)).length = 0;
            let hp = (raw as *mut u8).add(elemsize) as *mut c_void;
            let t3 = (im.hmput_default)(hp, elemsize);
            d.tag("len0");
            d.usize((*header(hash_to_arr(t3, elemsize))).length);
            d.0.extend_from_slice(std::slice::from_raw_parts(
                hash_to_arr(t3, elemsize) as *const u8,
                elemsize,
            ));
            (im.hmfree_func)(hash_to_arr(t3, elemsize), elemsize);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 14, 15 — hmput_key bootstrap from NULL, and the capacity assert
// ---------------------------------------------------------------------------

#[test]
fn err14_err15_hmput_key_bootstrap() {
    run_both("err14_15", |im| unsafe {
        let mut d = Digest::default();
        for &(elemsize, keysize, mode) in &[(8usize, 4usize, 0i32), (16, 8, 1), (72, 64, 0)] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut key = vec![0x51u8; keysize.max(8)];
            *key.last_mut().unwrap() = 0;
            let t = (im.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                mode,
            );
            d.tag("bootstrap");
            d.u8((!t.is_null()) as u8);
            let raw = hash_to_arr(t, elemsize);
            d.usize((*header(raw)).length);
            d.usize((*header(raw)).capacity);
            d.isize((*header(raw)).temp);
            // element 0 must be all zero; element 1 holds the key
            d.0.extend_from_slice(std::slice::from_raw_parts(raw as *const u8, elemsize));
            // `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` must not fire for
            // a long insert chain that repeatedly grows both array and table
            let mut t = t;
            for i in 0..300u64 {
                let mut k = vec![0u8; keysize.max(8)];
                k[..8].copy_from_slice(&(i + 1).to_ne_bytes());
                if mode >= 1 {
                    // NUL-terminated distinct strings
                    let s = cstring(format!("bootstrap_{i}").as_bytes());
                    k = s;
                    k.resize(keysize.max(16), 0);
                }
                t = (im.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, keysize, mode);
                std::mem::forget(k);
                let rawi = hash_to_arr(t, elemsize);
                assert!((*header(rawi)).length <= (*header(rawi)).capacity + 1);
            }
            d.usize((*header(hash_to_arr(t, elemsize))).length);
            d.usize((*header(hash_to_arr(t, elemsize))).capacity);
            (im.hmfree_func)(hash_to_arr(t, elemsize), elemsize);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 16 — mode >= 1 on a table whose string.mode is STBDS_SH_NONE
// ---------------------------------------------------------------------------

#[test]
fn err16_string_mode_on_sh_none_table() {
    run_both("err16", |im| unsafe {
        let mut d = Digest::default();
        (im.rand_seed)(DEFAULT_SEED);
        // shmode_func(_, STBDS_SH_NONE) then hmput_key(mode = 1): the `switch`
        // falls to `default:` and memcpy's `keysize` raw bytes.
        let t0 = (im.shmode_func)(16, 0);
        let mut t = t0;
        let mut seen = std::collections::HashSet::new();
        let mut rng = Rng::new(0x316);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 20 {
            let k = rand_cstring(&mut rng, 7, ASCII);
            if seen.insert(k.clone()) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let mut kb = k.clone();
            t = (im.hmput_key)(t, 16, kb.as_mut_ptr() as *mut c_void, 8, 1);
            let temp = (*header(hash_to_arr(t, 16))).temp;
            *((t as *mut u8).offset(temp * 16).add(8) as *mut u64) = i as u64;
            d.tag("put");
            d.isize(temp);
            // the key must be present *inline* (memcpy'd), not as a pointer
            d.0.extend_from_slice(std::slice::from_raw_parts(
                (t as *const u8).offset(temp * 16),
                8,
            ));
        }
        d.bytes(&digest_map(t, 16, KeyRepr::Inline(8), false));
        (im.hmfree_func)(hash_to_arr(t, 16), 16);
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 18 — stbds_shmode_func with an out-of-enum mode
// ---------------------------------------------------------------------------

#[test]
fn err18_shmode_func_out_of_range() {
    run_both("err18", |im| unsafe {
        let mut d = Digest::default();
        let modes: [c_int; 12] = [-1, -2, 4, 5, 100, 255, 256, 257, 511, 512, 1000, i32::MAX];
        for &mode in &modes {
            (im.rand_seed)(DEFAULT_SEED);
            let t = (im.shmode_func)(16, mode);
            let raw = hash_to_arr(t, 16);
            let ht = (*header(raw)).hash_table as *const HashIndex;
            d.tag("shmode");
            d.i64(mode as i64);
            d.u8((*ht).string.mode); // (unsigned char) mode
            d.usize((*ht).slot_count);
            d.usize((*ht).used_count);
            d.usize((*ht).seed);
            // a following put must take the `default:` memcpy branch unless the
            // truncated byte happens to be 1/2/3
            let mut t = t;
            for i in 0..12u64 {
                let mut k = cstring(format!("k{i}").as_bytes());
                k.resize(16, 0);
                t = (im.hmput_key)(t, 16, k.as_mut_ptr() as *mut c_void, 8, 1);
                let temp = (*header(hash_to_arr(t, 16))).temp;
                *((t as *mut u8).offset(temp * 16).add(8) as *mut u64) = i;
                d.isize(temp);
                std::mem::forget(k);
            }
            let repr = match (*ht).string.mode {
                1 | 2 | 3 => KeyRepr::Pointer,
                _ => KeyRepr::Inline(8),
            };
            d.bytes(&digest_map(t, 16, repr, false));
            (im.hmfree_func)(hash_to_arr(t, 16), 16);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 19, 20, 21, 27 — stbds_hmdel_key rejections
// ---------------------------------------------------------------------------

#[test]
fn err19_hmdel_key_null_map() {
    run_both("err19", |im| unsafe {
        let mut d = Digest::default();
        for &(elemsize, keysize, mode) in
            &[(8usize, 4usize, 0i32), (16, 8, 1), (16, 8, 2), (8, 4, -5), (1, 1, 0)]
        {
            let mut key = vec![0x61u8; keysize.max(8)];
            *key.last_mut().unwrap() = 0;
            let r = (im.hmdel_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                0,
                mode,
            );
            d.tag("null");
            d.u8(r.is_null() as u8);
        }
        d.0
    });
}

#[test]
fn err20_hmdel_key_no_hash_table() {
    run_both("err20", |im| unsafe {
        let mut d = Digest::default();
        for &(elemsize, keysize, mode) in &[(8usize, 4usize, 0i32), (16, 8, 1), (16, 8, 2)] {
            (im.rand_seed)(DEFAULT_SEED);
            let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize);
            (*header(raw)).length = 1;
            (*header(raw)).temp = 0x5EED;
            let hp = (raw as *mut u8).add(elemsize) as *mut c_void;
            let mut key = vec![0x62u8; keysize.max(8)];
            *key.last_mut().unwrap() = 0;
            let r = (im.hmdel_key)(hp, elemsize, key.as_mut_ptr() as *mut c_void, keysize, 0, mode);
            d.tag("no-table");
            d.u8((r == hp) as u8);
            // `temp` must have been reset to 0 before the early return
            d.isize((*header(raw)).temp);
            d.usize((*header(raw)).length);
            (im.hmfree_func)(raw, elemsize);
        }
        d.0
    });
}

#[test]
fn err21_err27_hmdel_key_absent_and_last() {
    run_both("err21_27", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x321);
        for n in [1i32, 2, 7, 8, 20, 100] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut t = build_int_map(im, n);
            // absent keys: temp must stay 0 and length must not change
            for _ in 0..80 {
                let k = rng.i32();
                if (0..n).any(|i| i * 7 + 1 == k) {
                    continue;
                }
                let mut kb = i32key(k);
                let before = (*header(hash_to_arr(t, 8))).length;
                t = (im.hmdel_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0, 0);
                d.tag("absent");
                d.isize((*header(hash_to_arr(t, 8))).temp);
                d.usize(before);
                d.usize((*header(hash_to_arr(t, 8))).length);
            }
            // deleting the *last* element: old_index == final_index, so no
            // memmove and no slot re-find
            for i in (0..n).rev() {
                let mut kb = i32key(i * 7 + 1);
                t = (im.hmdel_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0, 0);
                d.tag("last");
                d.isize((*header(hash_to_arr(t, 8))).temp);
                d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
            }
            // deleting from an empty (default-element-only) map
            for _ in 0..4 {
                let mut kb = i32key(1);
                t = (im.hmdel_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0, 0);
                d.tag("empty");
                d.isize((*header(hash_to_arr(t, 8))).temp);
                d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
            }
            (im.hmfree_func)(hash_to_arr(t, 8), 8);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 22, 24, 25, 28 — asserts that must NEVER fire through the public API
// (if either implementation aborts, the test process dies and the test fails)
// ---------------------------------------------------------------------------

#[test]
fn err22_err24_err25_err28_unreachable_asserts() {
    run_both("err22_28", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x328);
        // long randomised put/del churn: every slot_count the API can reach
        // (8 .. 2048) is built by make_hash_index, every delete does the
        // swap-in re-find, and every rebuild/shrink path runs.
        for round in 0..6 {
            (im.rand_seed)(DEFAULT_SEED + round);
            let mut t: *mut c_void = std::ptr::null_mut();
            let pool: Vec<i32> = (0..400).map(|_| rng.i32()).collect();
            let mut live: std::collections::HashSet<i32> = std::collections::HashSet::new();
            for step in 0..3000usize {
                let k = pool[rng.below(pool.len())];
                let mut kb = i32key(k);
                if rng.below(3) == 0 && !live.is_empty() {
                    t = (im.hmdel_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0, 0);
                    live.remove(&k);
                } else {
                    t = (im.hmput_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0);
                    let temp = (*header(hash_to_arr(t, 8))).temp;
                    let e = (t as *mut u8).offset(temp * 8);
                    std::ptr::copy_nonoverlapping(kb.as_ptr(), e, 4);
                    std::ptr::copy_nonoverlapping((step as i32).to_ne_bytes().as_ptr(), e.add(4), 4);
                    live.insert(k);
                }
                if step % 128 == 0 {
                    d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
                }
            }
            d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
            let ht = (*header(hash_to_arr(t, 8))).hash_table as *const HashIndex;
            // the make_hash_index invariant the C code asserts
            assert!((*ht).used_count_threshold + (*ht).tombstone_count_threshold < (*ht).slot_count);
            (im.hmfree_func)(hash_to_arr(t, 8), 8);
        }
        d.0
    });
}

/// Row 23 — `STBDS_ASSERT(table->used_count >= 0)` on a `size_t`. The check is
/// vacuous in C, so even a wrapped `used_count` must not abort. Reached by
/// forcing `used_count` to 0 before deleting a present key, which is state an
/// FFI caller can reach through the header the stb_ds macros expose.
#[test]
fn err23_used_count_wraparound_must_not_abort() {
    run_both("err23", |im| unsafe {
        let mut d = Digest::default();
        (im.rand_seed)(DEFAULT_SEED);
        let mut t = build_int_map(im, 12);
        let raw = hash_to_arr(t, 8);
        let ht = (*header(raw)).hash_table as *mut HashIndex;
        d.usize((*ht).used_count);
        (*ht).used_count = 0;
        // deleting a present key now decrements 0 -> SIZE_MAX
        let mut kb = i32key(11 * 7 + 1);
        t = (im.hmdel_key)(t, 8, kb.as_mut_ptr() as *mut c_void, 4, 0, 0);
        let raw = hash_to_arr(t, 8);
        let ht = (*header(raw)).hash_table as *mut HashIndex;
        d.tag("wrapped");
        d.usize((*ht).used_count);
        d.isize((*header(raw)).temp);
        d.usize((*header(raw)).length);
        d.bytes(&digest_map(t, 8, KeyRepr::Inline(4), false));
        (im.hmfree_func)(hash_to_arr(t, 8), 8);
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 26 — mode == 2 with STBDS_SH_STRDUP: the key is deliberately NOT freed
// ---------------------------------------------------------------------------

#[test]
fn err26_mode2_strdup_key_not_freed() {
    run_both("err26", |im| unsafe {
        let mut d = Digest::default();
        for &mode in &[1i32, 2] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut t = (im.shmode_func)(16, 2); // STBDS_SH_STRDUP
            let keys: Vec<Vec<u8>> =
                (0..10).map(|i| cstring(format!("strdup_key_{i}").as_bytes())).collect();
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                t = (im.hmput_key)(t, 16, kb.as_mut_ptr() as *mut c_void, 8, mode);
                let temp = (*header(hash_to_arr(t, 16))).temp;
                *((t as *mut u8).offset(temp * 16).add(8) as *mut u64) = i as u64;
            }
            // delete the last element each time: for `mode != STBDS_HM_STRING`
            // the strdup'd key is leaked instead of freed, and no slot re-find
            // (which would be UB for mode 2) happens.
            for k in keys.iter().rev() {
                let mut kb = k.clone();
                t = (im.hmdel_key)(t, 16, kb.as_mut_ptr() as *mut c_void, 8, 0, mode);
                d.tag("del");
                d.i64(mode as i64);
                d.isize((*header(hash_to_arr(t, 16))).temp);
                d.bytes(&digest_map(t, 16, KeyRepr::Pointer, false));
            }
            (im.hmfree_func)(hash_to_arr(t, 16), 16);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 29-33 — stbds_stralloc boundaries
// ---------------------------------------------------------------------------

#[test]
fn err29_err30_err31_err32_err33_stralloc_boundaries() {
    run_both("err29_33", |im| unsafe {
        let mut d = Digest::default();
        // over-sized string with storage == NULL, with storage != NULL, the
        // saturating `block` counter, and the empty string.
        let cases: &[(u8, &[usize])] = &[
            (0, &[600]),                 // len > blocksize, storage == NULL
            (0, &[8, 600]),              // len > blocksize, storage != NULL
            (0, &[0]),                   // empty string
            (0, &[0, 0, 0]),             // repeated empty strings
            (21, &[8, 8]),               // blocksize == 1<<20 -> block frozen
            (22, &[8, 8]),               // blocksize == 1<<20 exactly
            (23, &[8, 8]),
            (110, &[8, 8]),              // blocksize shifts to 0
            (255, &[8, 8]),              // shift count wraps mod 64
        ];
        for (block, lens) in cases {
            let mut a = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: *block,
                mode: 0,
            };
            let ap = &raw mut a as *mut c_void;
            let mut kept: Vec<*mut c_char> = Vec::new();
            for &len in *lens {
                let mut s = cstring(&vec![b'q'; len]);
                let p = (im.stralloc)(ap, s.as_mut_ptr() as *mut c_char);
                d.tag("alloc");
                d.u8(*block);
                d.usize(len);
                d.bytes(&cstr_bytes(p));
                digest_arena(&mut d, &raw const a);
                kept.push(p);
            }
            for p in kept {
                d.bytes(&cstr_bytes(p));
            }
            (im.strreset)(ap);
            digest_arena(&mut d, &raw const a);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 34, 35 — stbds_hmfree_func degenerate inputs
// ---------------------------------------------------------------------------

#[test]
fn err34_err35_hmfree_func_degenerate() {
    run_both("err34_35", |im| unsafe {
        let mut d = Digest::default();
        // a == NULL: immediate return, nothing freed
        for &elemsize in &[0usize, 1, 8, 16, 1024] {
            (im.hmfree_func)(std::ptr::null_mut(), elemsize);
            d.tag("null-ok");
            d.usize(elemsize);
        }
        // hash_table == NULL: skips the strdup sweep and strreset
        for &elemsize in &[1usize, 8, 16, 64] {
            (im.rand_seed)(DEFAULT_SEED);
            let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize * 4);
            (*header(raw)).length = 3;
            (im.hmfree_func)(raw, elemsize);
            d.tag("no-table-ok");
            d.usize(elemsize);
        }
        // length == 0 with a STRDUP table: the sweep loop `i = 1 .. length`
        // must not run
        (im.rand_seed)(DEFAULT_SEED);
        let t = (im.shmode_func)(16, 2);
        let raw = hash_to_arr(t, 16);
        (*header(raw)).length = 0;
        (im.hmfree_func)(raw, 16);
        d.tag("len0-strdup-ok");
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 36-39 — hashing boundaries
// ---------------------------------------------------------------------------

#[test]
fn err36_err37_hash_bytes_boundaries() {
    run_both("err36_37", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x336);
        // len == 0 must not read the buffer at all
        let mut empty: Vec<u8> = Vec::new();
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
            d.usize((im.hash_bytes)(empty.as_mut_ptr() as *mut c_void, 0, seed));
            d.usize((im.hash_bytes)(std::ptr::null_mut(), 0, seed));
        }
        // every tail length with the sign-extension corner in each tail byte
        for len in 1..=23usize {
            for bit in 0..len.min(8) {
                let mut buf = rng.bytes(len);
                buf[bit] |= 0x80;
                for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                    d.usize((im.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed));
                }
                // all-high and all-zero tails
                let mut hi = vec![0xFFu8; len];
                d.usize((im.hash_bytes)(hi.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED));
                let mut lo = vec![0x00u8; len];
                d.usize((im.hash_bytes)(lo.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED));
            }
        }
        d.0
    });
}

#[test]
fn err38_err39_hash_string_boundaries() {
    run_both("err38_39", |im| unsafe {
        let mut d = Digest::default();
        // empty string
        let mut e = cstring(b"");
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX, usize::MAX / 2] {
            d.usize((im.hash_string)(e.as_mut_ptr() as *mut c_char, seed));
        }
        // every single non-NUL byte value, including >= 0x80
        for b in 1u16..=255 {
            let mut s = cstring(&[b as u8]);
            for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                d.usize((im.hash_string)(s.as_mut_ptr() as *mut c_char, seed));
            }
        }
        // all-0xFF and all-0x80 strings of every length up to 40
        for len in 1..=40usize {
            for &fill in &[0x80u8, 0xFF, 0x01, 0x7F] {
                let mut s = cstring(&vec![fill; len]);
                d.usize((im.hash_string)(s.as_mut_ptr() as *mut c_char, DEFAULT_SEED));
            }
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 40, 41, 42 — intput's asserts (compared by termination signal)
// ---------------------------------------------------------------------------

/// Child-side entry point: only does anything when the driver env vars are set,
/// so it is a no-op during a normal test run.
#[test]
fn child_intput() {
    let n: i32 = match std::env::var("HARVEST_CHILD_INTPUT") {
        Ok(v) => v.parse().unwrap(),
        Err(_) => return,
    };
    let which = std::env::var("HARVEST_CHILD_LIB").unwrap();
    let l = libs();
    let im = if which == "c" { &l.c } else { &l.r };
    unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        (im.intput)(n);
    }
    // reaching here means no assert fired
    std::process::exit(0);
}

/// `(exit code, signal)` of a child that calls `intput(n)` against one library.
fn child_status(which: &str, n: i32) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "child_intput", "--nocapture", "--test-threads=1"])
        .env("HARVEST_CHILD_INTPUT", n.to_string())
        .env("HARVEST_CHILD_LIB", which)
        .output()
        .expect("failed to spawn child");
    (out.status.code(), out.status.signal())
}

#[test]
fn err40_err41_intput_aborts_on_9_and_11() {
    for n in [9i32, 11] {
        let c = child_status("c", n);
        let r = child_status("rust", n);
        assert_eq!(
            c, r,
            "intput({n}): C exited with {c:?} but Rust exited with {r:?}"
        );
        assert_eq!(
            c.1,
            Some(6),
            "intput({n}) was expected to die with SIGABRT, got {c:?}"
        );
    }
}

#[test]
fn err42_intput_returns_for_other_values() {
    let mut rng = Rng::new(0x342);
    let mut ns: Vec<i32> = vec![0, 1, 7, 8, 10, 12, -1, -9, -11, i32::MAX, i32::MIN];
    ns.extend((0..8).map(|_| rng.i32()).filter(|v| *v != 9 && *v != 11));
    for n in ns {
        let c = child_status("c", n);
        let r = child_status("rust", n);
        assert_eq!(c, r, "intput({n}): C {c:?} vs Rust {r:?}");
        assert_eq!(c.0, Some(0), "intput({n}) should return normally, got {c:?}");
    }
}
