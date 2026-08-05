//! Phase B — lowest-level entry points. CONFIGS.md rows 26-30, 34, 36.
//! hashtable_*, strbuffer_*, utf8_*, jsonp_dtostr, jsonp_strtod, version.
mod common;
#[path = "gen.rs"]
mod gen;

use common::*;
use gen::Rng;
use std::os::raw::{c_char, c_double, c_int, c_void};

// strbuffer_t = { char*; size_t; size_t; } = 24 bytes
const STRBUFFER_SZ: usize = 24;
// hashtable_t = size_t + ptr + size_t + list(2 ptr) + ordered_list(2 ptr) = 56
const HASHTABLE_SZ: usize = 56;

type FnSbInit = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnSbClose = unsafe extern "C" fn(*mut c_void);
type FnSbAppendByte = unsafe extern "C" fn(*mut c_void, c_char) -> c_int;
type FnSbAppendBytes = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> c_int;
type FnSbPop = unsafe extern "C" fn(*mut c_void) -> c_char;
type FnSbValue = unsafe extern "C" fn(*const c_void) -> *const c_char;

#[test]
fn row_27_strbuffer() {
    let l = libs();
    for seed in 0..300u64 {
        let run = |lib: &libloading::Library| unsafe {
            let init: libloading::Symbol<FnSbInit> = sym(lib, b"strbuffer_init");
            let close: libloading::Symbol<FnSbClose> = sym(lib, b"strbuffer_close");
            let ab: libloading::Symbol<FnSbAppendByte> = sym(lib, b"strbuffer_append_byte");
            let abs: libloading::Symbol<FnSbAppendBytes> = sym(lib, b"strbuffer_append_bytes");
            let pop: libloading::Symbol<FnSbPop> = sym(lib, b"strbuffer_pop");
            let value: libloading::Symbol<FnSbValue> = sym(lib, b"strbuffer_value");

            let mut sb = vec![0u8; STRBUFFER_SZ];
            let p = sb.as_mut_ptr() as *mut c_void;
            assert_eq!(init(p), 0);
            let mut rng = Rng::new(seed);
            let mut popped: Vec<i8> = Vec::new();
            for _ in 0..rng.below(60) {
                match rng.below(3) {
                    0 => {
                        ab(p, (rng.below(94) + 33) as c_char);
                    }
                    1 => {
                        let data: Vec<u8> = (0..rng.below(8))
                            .map(|_| (rng.below(94) + 33) as u8)
                            .collect();
                        abs(p, data.as_ptr() as *const c_char, data.len());
                    }
                    _ => {
                        popped.push(pop(p) as i8);
                    }
                }
            }
            let v = value(p);
            let out = cstr_to_vec(v);
            close(p);
            (out, popped)
        };
        let c = run(&l.c);
        let r = run(&l.r);
        assert_eq!(c, r, "strbuffer mismatch seed={seed}");
    }
}

#[test]
fn row_28_utf8() {
    let l = libs();
    // utf8_encode across all codepoints, utf8_check_first over all bytes,
    // utf8_check_full / iterate over random byte sequences.
    type FnEncode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
    type FnCheckFirst = unsafe extern "C" fn(c_char) -> usize;
    type FnCheckFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
    type FnIterate = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char;
    type FnCheckString = unsafe extern "C" fn(*const c_char, usize) -> c_int;

    unsafe {
        let c_enc: libloading::Symbol<FnEncode> = sym(&l.c, b"utf8_encode");
        let r_enc: libloading::Symbol<FnEncode> = sym(&l.r, b"utf8_encode");
        // Full codepoint sweep including out-of-range.
        for cp in (-5i64..0x110020).step_by(1) {
            let cp = cp as i32;
            let mut cbuf = [0u8; 8];
            let mut rbuf = [0u8; 8];
            let mut csz = 999usize;
            let mut rsz = 999usize;
            let cr = c_enc(cp, cbuf.as_mut_ptr() as *mut c_char, &mut csz);
            let rr = r_enc(cp, rbuf.as_mut_ptr() as *mut c_char, &mut rsz);
            assert_eq!(cr, rr, "utf8_encode ret cp={cp}");
            assert_eq!(csz, rsz, "utf8_encode size cp={cp}");
            if cr == 0 {
                assert_eq!(&cbuf[..csz], &rbuf[..rsz], "utf8_encode bytes cp={cp}");
            }
        }

        let c_cf: libloading::Symbol<FnCheckFirst> = sym(&l.c, b"utf8_check_first");
        let r_cf: libloading::Symbol<FnCheckFirst> = sym(&l.r, b"utf8_check_first");
        for b in 0..=255u16 {
            assert_eq!(c_cf(b as i8 as c_char), r_cf(b as i8 as c_char), "check_first byte={b}");
        }

        let c_full: libloading::Symbol<FnCheckFull> = sym(&l.c, b"utf8_check_full");
        let r_full: libloading::Symbol<FnCheckFull> = sym(&l.r, b"utf8_check_full");
        let c_it: libloading::Symbol<FnIterate> = sym(&l.c, b"utf8_iterate");
        let r_it: libloading::Symbol<FnIterate> = sym(&l.r, b"utf8_iterate");
        let c_str: libloading::Symbol<FnCheckString> = sym(&l.c, b"utf8_check_string");
        let r_str: libloading::Symbol<FnCheckString> = sym(&l.r, b"utf8_check_string");

        for seed in 0..2000u64 {
            let mut rng = Rng::new(seed ^ 0x99);
            let n = rng.below(6) as usize + 1;
            // Bias toward valid-ish leading bytes sometimes
            let seq: Vec<u8> = (0..n)
                .map(|_| {
                    if rng.boolean() {
                        rng.below(256) as u8
                    } else {
                        // valid continuation-ish or ascii
                        (rng.below(0x80)) as u8
                    }
                })
                .collect();
            let mut ccp = 0i32;
            let mut rcp = 0i32;
            let cl = c_full(seq.as_ptr() as *const c_char, seq.len(), &mut ccp);
            let rl = r_full(seq.as_ptr() as *const c_char, seq.len(), &mut rcp);
            assert_eq!(cl, rl, "check_full len seq={seq:?}");
            if cl != 0 {
                assert_eq!(ccp, rcp, "check_full cp seq={seq:?}");
            }
            // iterate
            let mut ic = 0i32;
            let mut ir = 0i32;
            let pc = c_it(seq.as_ptr() as *const c_char, seq.len(), &mut ic);
            let pr = r_it(seq.as_ptr() as *const c_char, seq.len(), &mut ir);
            let coff = if pc.is_null() { -1i64 } else { pc as i64 - seq.as_ptr() as i64 };
            let roff = if pr.is_null() { -1i64 } else { pr as i64 - seq.as_ptr() as i64 };
            assert_eq!(coff, roff, "iterate offset seq={seq:?}");
            if !pc.is_null() {
                assert_eq!(ic, ir, "iterate cp seq={seq:?}");
            }
            // check_string
            assert_eq!(
                c_str(seq.as_ptr() as *const c_char, seq.len()),
                r_str(seq.as_ptr() as *const c_char, seq.len()),
                "check_string seq={seq:?}"
            );
        }
    }
}

#[test]
fn row_29_dtostr() {
    let l = libs();
    type FnDtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;
    unsafe {
        let c_d: libloading::Symbol<FnDtostr> = sym(&l.c, b"jsonp_dtostr");
        let r_d: libloading::Symbol<FnDtostr> = sym(&l.r, b"jsonp_dtostr");
        for seed in 0..5000u64 {
            let mut rng = Rng::new(seed ^ 0x1357);
            let bits = rng.next();
            let mut v = f64::from_bits(bits);
            if !v.is_finite() {
                v = (rng.next() % 1_000_000) as f64 / 1000.0;
            }
            for prec in 0..18i32 {
                let mut cbuf = [0u8; 64];
                let mut rbuf = [0u8; 64];
                let cr = c_d(cbuf.as_mut_ptr() as *mut c_char, cbuf.len(), v, prec);
                let rr = r_d(rbuf.as_mut_ptr() as *mut c_char, rbuf.len(), v, prec);
                assert_eq!(cr, rr, "dtostr ret v={v} prec={prec}");
                assert_eq!(&cbuf[..], &rbuf[..], "dtostr buf v={v} ({bits:#x}) prec={prec}");
            }
        }
    }
}

#[test]
fn row_30_strtod() {
    // jsonp_strtod parses from a strbuffer. Build a strbuffer in each lib,
    // fill it with a number string, call jsonp_strtod, compare double bits + ret.
    let l = libs();
    type FnStrtod = unsafe extern "C" fn(*mut c_void, *mut c_double) -> c_int;
    let numbers: Vec<String> = {
        let mut v = vec![
            "0".into(), "-0".into(), "1.5".into(), "-1.5".into(), "1e10".into(),
            "1e-10".into(), "3.141592653589793".into(), "1e308".into(), "1e-308".into(),
            "123456789.123456789".into(), "0.0001".into(), "9999999999999999".into(),
        ];
        let mut rng = Rng::new(0xDEAD);
        for _ in 0..2000 {
            let mant = rng.next() % 1_000_000_000;
            let frac = rng.next() % 1_000_000;
            let exp = (rng.below(40) as i64) - 20;
            v.push(format!("{}.{}e{}", mant, frac, exp));
        }
        v
    };
    unsafe {
        for numstr in &numbers {
            let run = |lib: &libloading::Library| -> (c_int, u64) {
                let init: libloading::Symbol<FnSbInit> = sym(lib, b"strbuffer_init");
                let close: libloading::Symbol<FnSbClose> = sym(lib, b"strbuffer_close");
                let abs: libloading::Symbol<FnSbAppendBytes> = sym(lib, b"strbuffer_append_bytes");
                let strtod: libloading::Symbol<FnStrtod> = sym(lib, b"jsonp_strtod");
                let mut sb = vec![0u8; STRBUFFER_SZ];
                let p = sb.as_mut_ptr() as *mut c_void;
                init(p);
                abs(p, numstr.as_ptr() as *const c_char, numstr.len());
                let mut out = 0f64;
                let ret = strtod(p, &mut out);
                close(p);
                (ret, out.to_bits())
            };
            let c = run(&l.c);
            let r = run(&l.r);
            assert_eq!(c, r, "jsonp_strtod mismatch for {numstr}");
        }
    }
}

#[test]
fn row_26_hashtable() {
    // Drive hashtable_* directly. Values must be json_t*; use json_null()
    // (a singleton) so we don't need to manage refcounts, and compare the
    // observable iteration order + get results.
    let l = libs();
    type FnHtInit = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FnHtClose = unsafe extern "C" fn(*mut c_void);
    type FnHtSet = unsafe extern "C" fn(*mut c_void, *const c_char, usize, *mut c_void) -> c_int;
    type FnHtGet = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> *mut c_void;
    type FnHtDel = unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> c_int;
    type FnHtClear = unsafe extern "C" fn(*mut c_void);
    type FnHtIter = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
    type FnHtIterNext = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type FnHtIterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type FnHtIterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
    type FnNull = unsafe extern "C" fn() -> *mut c_void;

    for seed in 0..300u64 {
        let run = |lib: &libloading::Library| unsafe {
            let init: libloading::Symbol<FnHtInit> = sym(lib, b"hashtable_init");
            let close: libloading::Symbol<FnHtClose> = sym(lib, b"hashtable_close");
            let set: libloading::Symbol<FnHtSet> = sym(lib, b"hashtable_set");
            let get: libloading::Symbol<FnHtGet> = sym(lib, b"hashtable_get");
            let del: libloading::Symbol<FnHtDel> = sym(lib, b"hashtable_del");
            let clear: libloading::Symbol<FnHtClear> = sym(lib, b"hashtable_clear");
            let iter: libloading::Symbol<FnHtIter> = sym(lib, b"hashtable_iter");
            let iter_next: libloading::Symbol<FnHtIterNext> = sym(lib, b"hashtable_iter_next");
            let iter_key: libloading::Symbol<FnHtIterKey> = sym(lib, b"hashtable_iter_key");
            let iter_key_len: libloading::Symbol<FnHtIterKeyLen> = sym(lib, b"hashtable_iter_key_len");
            let jnull: libloading::Symbol<FnNull> = sym(lib, b"json_null");

            // NOTE: hashtable order depends on hashtable_seed. Seed is process-
            // global and autoseeded randomly, so iteration order (which is
            // insertion order via ordered_list, NOT hash order) is what we
            // compare — that's deterministic regardless of seed.
            let mut ht = vec![0u8; HASHTABLE_SZ];
            let p = ht.as_mut_ptr() as *mut c_void;
            assert_eq!(init(p), 0);
            let mut rng = Rng::new(seed);
            let null = jnull();
            let mut present: Vec<String> = Vec::new();
            for _ in 0..rng.below(30) {
                let key = format!("k{}", rng.below(15));
                match rng.below(4) {
                    0 | 1 => {
                        set(p, key.as_ptr() as *const c_char, key.len(), null);
                        if !present.contains(&key) {
                            present.push(key);
                        }
                    }
                    2 => {
                        let _ = del(p, key.as_ptr() as *const c_char, key.len());
                        present.retain(|k| k != &key);
                    }
                    _ => {
                        // get: record hit/miss into order-independent probe
                        let _ = get(p, key.as_ptr() as *const c_char, key.len());
                    }
                }
            }
            // Collect iteration order (insertion order).
            let mut order: Vec<Vec<u8>> = Vec::new();
            let mut it = iter(p);
            while !it.is_null() {
                let k = iter_key(it);
                let kl = iter_key_len(it);
                order.push(std::slice::from_raw_parts(k as *const u8, kl).to_vec());
                it = iter_next(p, it);
            }
            // Also probe get() for every possible key.
            let mut probes: Vec<bool> = Vec::new();
            for i in 0..15 {
                let key = format!("k{}", i);
                probes.push(!get(p, key.as_ptr() as *const c_char, key.len()).is_null());
            }
            clear(p);
            let after_clear_iter = iter(p).is_null();
            close(p);
            (order, probes, after_clear_iter)
        };
        let c = run(&l.c);
        let r = run(&l.r);
        assert_eq!(c, r, "hashtable mismatch seed={seed}");
    }
}

#[test]
fn row_34_version() {
    let l = libs();
    type FnVerStr = unsafe extern "C" fn() -> *const c_char;
    type FnVerCmp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    unsafe {
        let c_s: libloading::Symbol<FnVerStr> = sym(&l.c, b"jansson_version_str");
        let r_s: libloading::Symbol<FnVerStr> = sym(&l.r, b"jansson_version_str");
        assert_eq!(cstr_to_vec(c_s()), cstr_to_vec(r_s()));

        let c_c: libloading::Symbol<FnVerCmp> = sym(&l.c, b"jansson_version_cmp");
        let r_c: libloading::Symbol<FnVerCmp> = sym(&l.r, b"jansson_version_cmp");
        for maj in 0..4 {
            for min in [0, 14, 15, 16, 100] {
                for mic in [0, 1, 5] {
                    assert_eq!(c_c(maj, min, mic), r_c(maj, min, mic), "version_cmp {maj}.{min}.{mic}");
                }
            }
        }
    }
}
