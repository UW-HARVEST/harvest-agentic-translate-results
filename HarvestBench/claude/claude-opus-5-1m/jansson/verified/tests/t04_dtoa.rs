//! Differential tests for `dtoa.c` (`dtoa_r`, `dtoa`, `freedtoa`, `gethex`,
//! `strtod__unused`, `dtoa_divmax`) and `strconv.c` (`jsonp_dtostr`,
//! `jsonp_strtod`).
//!
//! Covers CONFIGS.md rows 23-35 and ERRORS.md rows 235-243.
//!
//! Everything is driven through `dlsym`'d function pointers on the two shared
//! objects, so the Rust side is exercised exactly like an external consumer.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Serialisation
//
// `dtoa.c` is compiled WITHOUT `MULTIPLE_THREADS`, so `dtoa_result`, the
// `Balloc`/`Bfree` free list and `p5s` are plain mutable statics. Cargo runs
// `#[test]` functions on several threads at once, which would race on them, so
// every test in this file takes one process-wide lock.
// ---------------------------------------------------------------------------

fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sentinel written into `*decpt` / `*sign` before every call so that "the
/// library never touched it" is distinguishable from any real value.
const SENT: c_int = -424242;

fn fdesc(v: f64) -> String {
    format!("{:?}/{:#018x}", v, v.to_bits())
}

// ---------------------------------------------------------------------------
// dtoa_r / dtoa snapshots
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct DR {
    null: bool,
    digits: Vec<u8>,
    decpt: c_int,
    sign: c_int,
    /// `None` => `rve` argument was NULL.
    /// `Some(i64::MIN)` => `rve` was passed but never written.
    /// otherwise `*rve - ret` as a byte offset.
    rve: Option<i64>,
    /// The whole caller buffer (0xAA-prefilled) plus a 16-byte guard region.
    buf: Vec<u8>,
}

impl std::fmt::Debug for DR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DR{{ ret:{}, digits:{:?}, decpt:{}, sign:{}, rve:{:?}, buf:{:02x?} }}",
            if self.null { "NULL" } else { "buf" },
            String::from_utf8_lossy(&self.digits),
            self.decpt,
            self.sign,
            self.rve,
            self.buf,
        )
    }
}

const GUARD: usize = 16;

/// `dtoa_r` with a caller-supplied buffer of `blen` bytes (plus a guard).
unsafe fn dr_buf(l: &Lib, v: f64, mode: c_int, ndigits: c_int, blen: usize, want_rve: bool) -> DR {
    let mut b: Vec<u8> = vec![0xAA; blen + GUARD];
    let mut decpt: c_int = SENT;
    let mut sign: c_int = SENT;
    let mut rvep: *mut c_char = std::ptr::null_mut();
    let rp: *mut *mut c_char = if want_rve {
        &mut rvep
    } else {
        std::ptr::null_mut()
    };
    let ret = (l.dtoa_r)(
        v,
        mode,
        ndigits,
        &mut decpt,
        &mut sign,
        rp,
        b.as_mut_ptr() as *mut c_char,
        blen,
    );
    DR {
        null: ret.is_null(),
        digits: if ret.is_null() {
            Vec::new()
        } else {
            cstr_bytes(ret)
        },
        decpt,
        sign,
        rve: if !want_rve {
            None
        } else if rvep.is_null() {
            Some(i64::MIN)
        } else {
            Some(rvep as isize as i64 - ret as isize as i64)
        },
        buf: b,
    }
}

/// `dtoa_r(..., buf = NULL, blen = 0)` — heap-allocated result, released with
/// the OWNING library's `freedtoa`.
unsafe fn dr_heap(l: &Lib, v: f64, mode: c_int, ndigits: c_int, want_rve: bool) -> DR {
    let mut decpt: c_int = SENT;
    let mut sign: c_int = SENT;
    let mut rvep: *mut c_char = std::ptr::null_mut();
    let rp: *mut *mut c_char = if want_rve {
        &mut rvep
    } else {
        std::ptr::null_mut()
    };
    let ret = (l.dtoa_r)(
        v,
        mode,
        ndigits,
        &mut decpt,
        &mut sign,
        rp,
        std::ptr::null_mut(),
        0,
    );
    let snap = DR {
        null: ret.is_null(),
        digits: if ret.is_null() {
            Vec::new()
        } else {
            cstr_bytes(ret)
        },
        decpt,
        sign,
        rve: if !want_rve {
            None
        } else if rvep.is_null() {
            Some(i64::MIN)
        } else {
            Some(rvep as isize as i64 - ret as isize as i64)
        },
        buf: Vec::new(),
    };
    if !ret.is_null() {
        (l.freedtoa)(ret);
    }
    snap
}

/// `dtoa` — the per-library static result is NOT freed by us on purpose: the
/// next `dtoa` call must free it itself.
unsafe fn dtoa_call(l: &Lib, v: f64, mode: c_int, ndigits: c_int) -> DR {
    let mut decpt: c_int = SENT;
    let mut sign: c_int = SENT;
    let mut rvep: *mut c_char = std::ptr::null_mut();
    let ret = (l.dtoa)(v, mode, ndigits, &mut decpt, &mut sign, &mut rvep);
    DR {
        null: ret.is_null(),
        digits: if ret.is_null() {
            Vec::new()
        } else {
            cstr_bytes(ret)
        },
        decpt,
        sign,
        rve: Some(if rvep.is_null() {
            i64::MIN
        } else {
            rvep as isize as i64 - ret as isize as i64
        }),
        buf: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// jsonp_dtostr snapshot
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct DS {
    ret: c_int,
    buf: Vec<u8>,
}

impl std::fmt::Debug for DS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DS{{ ret:{}, buf:{:02x?} ({:?}) }}",
            self.ret,
            self.buf,
            String::from_utf8_lossy(&self.buf),
        )
    }
}

unsafe fn dtostr(l: &Lib, v: f64, size: usize, precision: c_int) -> DS {
    let mut b: Vec<u8> = vec![0xAA; size + GUARD];
    let ret = (l.jsonp_dtostr)(b.as_mut_ptr() as *mut c_char, size, v, precision);
    DS { ret, buf: b }
}

// ---------------------------------------------------------------------------
// values
// ---------------------------------------------------------------------------

fn specials() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1e-323,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        1e16,
        1e17,
        1e-4,
        1e-5,
        123456789012345678.0,
    ]
}

fn nan_and_inf() -> Vec<f64> {
    vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // quiet NaN, payload 1
        f64::from_bits(0xfff8_dead_beef_cafe), // negative quiet NaN, payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7ff4_0000_0000_0000),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]
}

// ===========================================================================
// 1. CONFIGS 23 — the exact call jsonp_dtostr makes
// ===========================================================================

#[test]
fn dtoa_r_jsonp_dtostr_path() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        let mut rng = Rng::new(0x11_2233_4455_6677);
        for _ in 0..20000 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        for v in vals {
            let cv = dr_buf(&d.c, v, 0, 0, 25, true);
            let rv = dr_buf(&d.rs, v, 0, 0, 25, true);
            eq(
                &format!("dtoa_r({}, mode=0, ndigits=0, blen=25)", fdesc(v)),
                cv,
                rv,
            );
        }
    }
}

// ===========================================================================
// 2. CONFIGS 24 / ERRORS 238, 241 — every mode x ndigits, big and tiny blen
// ===========================================================================

#[test]
fn dtoa_r_all_modes_and_ndigits() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        let mut rng = Rng::new(0x2222_0000_1111_3333);
        for _ in 0..1200 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        let ndigits_set = [-5, -1, 0, 1, 2, 5, 15, 17, 30];
        for mode in 0..=9 {
            for &nd in ndigits_set.iter() {
                for (i, &v) in vals.iter().enumerate() {
                    // roomy buffer
                    let cv = dr_buf(&d.c, v, mode, nd, 64, true);
                    let rv = dr_buf(&d.rs, v, mode, nd, 64, true);
                    eq(
                        &format!("dtoa_r({}, mode={}, ndigits={}, blen=64)", fdesc(v), mode, nd),
                        cv,
                        rv,
                    );
                    // deliberately too-short buffer => NULL + *rve length hint
                    let cv = dr_buf(&d.c, v, mode, nd, 3, true);
                    let rv = dr_buf(&d.rs, v, mode, nd, 3, true);
                    eq(
                        &format!("dtoa_r({}, mode={}, ndigits={}, blen=3)", fdesc(v), mode, nd),
                        cv,
                        rv,
                    );
                    // rotate through the whole "just too short / just long
                    // enough" boundary region without a full cross product
                    let blen = 1 + (i % 21);
                    let cv = dr_buf(&d.c, v, mode, nd, blen, true);
                    let rv = dr_buf(&d.rs, v, mode, nd, blen, true);
                    eq(
                        &format!(
                            "dtoa_r({}, mode={}, ndigits={}, blen={})",
                            fdesc(v),
                            mode,
                            nd,
                            blen
                        ),
                        cv,
                        rv,
                    );
                }
                // the pathological small lengths, on the special values only
                for &blen in [0usize, 1, 2, 4, 8, 9, 18, 19].iter() {
                    for &v in specials().iter() {
                        let cv = dr_buf(&d.c, v, mode, nd, blen, true);
                        let rv = dr_buf(&d.rs, v, mode, nd, blen, true);
                        eq(
                            &format!(
                                "dtoa_r({}, mode={}, ndigits={}, blen={})",
                                fdesc(v),
                                mode,
                                nd,
                                blen
                            ),
                            cv,
                            rv,
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 3. CONFIGS 25 / ERRORS 240 — mode outside 0..9 is treated as mode 0
// ===========================================================================

#[test]
fn dtoa_r_mode_out_of_range() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        vals.extend(nan_and_inf());
        let mut rng = Rng::new(0x3333_4444_5555_6666);
        for _ in 0..250 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        for &mode in [-1, -100, 10, 11, 1000, i32::MIN, i32::MAX].iter() {
            for &nd in [-1, 0, 1, 17].iter() {
                for &v in vals.iter() {
                    let cv = dr_buf(&d.c, v, mode, nd, 64, true);
                    let rv = dr_buf(&d.rs, v, mode, nd, 64, true);
                    eq(
                        &format!("dtoa_r({}, mode={}, ndigits={}, blen=64)", fdesc(v), mode, nd),
                        cv,
                        rv,
                    );
                    // must be identical to mode 0 with the same value
                    let c0 = dr_buf(&d.c, v, 0, nd, 64, true);
                    assert_eq!(
                        c0,
                        dr_buf(&d.c, v, mode, nd, 64, true),
                        "C: mode {} is not equivalent to mode 0 for {}",
                        mode,
                        fdesc(v)
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 4. CONFIGS 27 / ERRORS 239 — NaN / +-Inf
// ===========================================================================

#[test]
fn dtoa_r_specials() {
    let _g = lock();
    let d = duo();
    unsafe {
        for v in nan_and_inf() {
            for &mode in [0, 1, 2, 3, 4, 5, 6, 9, -1, 100].iter() {
                for &nd in [-1, 0, 1, 17, 30].iter() {
                    for &blen in [0usize, 1, 2, 3, 4, 8, 9, 25, 64].iter() {
                        let cv = dr_buf(&d.c, v, mode, nd, blen, true);
                        let rv = dr_buf(&d.rs, v, mode, nd, blen, true);
                        let what = format!(
                            "dtoa_r({}, mode={}, ndigits={}, blen={})",
                            fdesc(v),
                            mode,
                            nd,
                            blen
                        );
                        eq(&what, cv, rv);
                        // ERRORS 239: *decpt == 9999 for NaN/Inf
                        let cv = dr_buf(&d.c, v, mode, nd, blen, true);
                        assert_eq!(cv.decpt, 9999, "C: *decpt for {} in {}", fdesc(v), what);
                        let rv = dr_buf(&d.rs, v, mode, nd, blen, true);
                        assert_eq!(rv.decpt, 9999, "RUST: *decpt for {} in {}", fdesc(v), what);
                        assert_eq!(
                            cv.sign,
                            if v.to_bits() >> 63 == 1 { 1 } else { 0 },
                            "C: *sign for {}",
                            fdesc(v)
                        );
                    }
                    // and the heap path
                    let cv = dr_heap(&d.c, v, mode, nd, true);
                    let rv = dr_heap(&d.rs, v, mode, nd, true);
                    eq(
                        &format!(
                            "dtoa_r({}, mode={}, ndigits={}, buf=NULL)",
                            fdesc(v),
                            mode,
                            nd
                        ),
                        cv,
                        rv,
                    );
                }
            }
        }
    }
}

// ===========================================================================
// 5. CONFIGS 26, 29 — rve == NULL, and buf == NULL (heap) + freedtoa
// ===========================================================================

#[test]
fn dtoa_r_rve_null_and_heap_alloc() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        vals.extend(nan_and_inf());
        let mut rng = Rng::new(0x5555_6666_7777_8888);
        for _ in 0..250 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        for &v in vals.iter() {
            for &(mode, nd) in [
                (0, 0),
                (0, 17),
                (1, 0),
                (2, 1),
                (2, 17),
                (3, 5),
                (4, 6),
                (5, 3),
                (6, 4),
                (9, 2),
            ]
            .iter()
            {
                // rve == NULL, caller buffer
                let cv = dr_buf(&d.c, v, mode, nd, 64, false);
                let rv = dr_buf(&d.rs, v, mode, nd, 64, false);
                eq(
                    &format!(
                        "dtoa_r({}, mode={}, ndigits={}, blen=64, rve=NULL)",
                        fdesc(v),
                        mode,
                        nd
                    ),
                    cv,
                    rv,
                );
                // rve == NULL, too-short caller buffer (the `return buf` with
                // `rve == NULL` branch)
                let cv = dr_buf(&d.c, v, mode, nd, 2, false);
                let rv = dr_buf(&d.rs, v, mode, nd, 2, false);
                eq(
                    &format!(
                        "dtoa_r({}, mode={}, ndigits={}, blen=2, rve=NULL)",
                        fdesc(v),
                        mode,
                        nd
                    ),
                    cv,
                    rv,
                );
                // buf == NULL, blen == 0 => heap-allocated, freed by owner
                let cv = dr_heap(&d.c, v, mode, nd, true);
                let rv = dr_heap(&d.rs, v, mode, nd, true);
                eq(
                    &format!(
                        "dtoa_r({}, mode={}, ndigits={}, buf=NULL, rve!=NULL)",
                        fdesc(v),
                        mode,
                        nd
                    ),
                    cv,
                    rv,
                );
                let cv = dr_heap(&d.c, v, mode, nd, false);
                let rv = dr_heap(&d.rs, v, mode, nd, false);
                eq(
                    &format!(
                        "dtoa_r({}, mode={}, ndigits={}, buf=NULL, rve=NULL)",
                        fdesc(v),
                        mode,
                        nd
                    ),
                    cv,
                    rv,
                );
            }
        }
    }
}

// ===========================================================================
// 6. CONFIGS 28 — dtoa()'s static result reuse (implicit freedtoa)
// ===========================================================================

#[test]
fn dtoa_static_result_reuse() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        vals.extend(nan_and_inf());
        let mut rng = Rng::new(0x6666_7777_8888_9999);
        for _ in 0..500 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        let modes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -3, 42];
        let nds = [-2, 0, 1, 3, 17];
        let mut n = 0usize;
        for (i, &v) in vals.iter().enumerate() {
            let mode = modes[i % modes.len()];
            let nd = nds[i % nds.len()];
            // NOTE: deliberately never freed here — the next `dtoa` call must
            // release the previous static result itself.
            let cv = dtoa_call(&d.c, v, mode, nd);
            let rv = dtoa_call(&d.rs, v, mode, nd);
            eq(
                &format!("dtoa({}, mode={}, ndigits={})", fdesc(v), mode, nd),
                cv,
                rv,
            );
            n += 1;
        }
        assert!(n >= 300, "expected at least 300 dtoa calls, made {}", n);
    }
}

// ===========================================================================
// 7. CONFIGS 30 / ERRORS 242 — gethex
// ===========================================================================

/// The input plus 16 trailing NULs: `gethex` unconditionally reads `*sp + 2`
/// (it expects to skip a `0x` prefix), so short inputs must be padded to keep
/// the read in bounds. Both libraries see byte-identical memory.
fn padded(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

unsafe fn gethex_one(
    l: &Lib,
    base: *const c_char,
    start: usize,
    rounding: c_int,
    sign: c_int,
) -> (i64, u64) {
    let mut sp: *const c_char = base.add(start);
    let mut u = U {
        d: f64::from_bits(0xdead_beef_cafe_babe),
    };
    (l.gethex)(&mut sp, &mut u, rounding, sign);
    ((sp as isize - base as isize) as i64, u.bits())
}

#[test]
fn gethex_shapes() {
    let _g = lock();
    let d = duo();
    let inputs: &[&str] = &[
        "0x1p0",
        "0x1.8p3",
        "0xAp-4",
        "0x0p0",
        "0x1.fffffffffffffp+1023",
        "0x1p-1074",
        "0x1p-1075",
        "0x10000000000000000p0",
        "xyz",
        "",
        "0x",
        "0x.8p0",
        "p0",
        // a few extra shapes that hit the remaining gethex branches
        "0x0.0p0",
        "0x1P+4",
        "0x1p",
        "0x1p+",
        "0x1p-",
        "0x0000000000001p0",
        "0x1.0000000000001p0",
        "0x1p99999999999999",
        "0x1p-99999999999999",
        "0xfffffffffffff800p-64",
        "0x1.8p0",
        "0x2.8p0",
    ];
    unsafe {
        for s in inputs {
            let buf = padded(s.as_bytes());
            let base = buf.as_ptr() as *const c_char;
            for &start in [0usize, 2].iter() {
                if start > s.len() {
                    continue;
                }
                for rounding in 0..=3 {
                    for sign in 0..=1 {
                        let cv = gethex_one(&d.c, base, start, rounding, sign);
                        let rv = gethex_one(&d.rs, base, start, rounding, sign);
                        eq(
                            &format!(
                                "gethex({:?}, start={}, rounding={}, sign={}) -> (sp_off, rvp_bits)",
                                s, start, rounding, sign
                            ),
                            (cv.0, format!("{:#018x}", cv.1)),
                            (rv.0, format!("{:#018x}", rv.1)),
                        );
                    }
                }
            }
        }

        // randomized hex-float shapes
        let mut rng = Rng::new(0x7777_8888_9999_aaaa);
        let hexd = b"0123456789abcdefABCDEF";
        for _ in 0..4000 {
            let mut s: Vec<u8> = b"0x".to_vec();
            let n = 1 + rng.below(18);
            for _ in 0..n {
                s.push(hexd[rng.below(hexd.len())]);
            }
            if rng.bool() {
                s.push(b'.');
                let m = rng.below(18);
                for _ in 0..m {
                    s.push(hexd[rng.below(hexd.len())]);
                }
            }
            if rng.bool() {
                s.push(if rng.bool() { b'p' } else { b'P' });
                match rng.below(3) {
                    0 => s.push(b'+'),
                    1 => s.push(b'-'),
                    _ => {}
                }
                let m = 1 + rng.below(4);
                for _ in 0..m {
                    s.push(b'0' + rng.below(10) as u8);
                }
            }
            let buf = padded(&s);
            let base = buf.as_ptr() as *const c_char;
            let rounding = rng.below(4) as c_int;
            let sign = rng.below(2) as c_int;
            let cv = gethex_one(&d.c, base, 0, rounding, sign);
            let rv = gethex_one(&d.rs, base, 0, rounding, sign);
            eq(
                &format!(
                    "gethex({:?}, start=0, rounding={}, sign={}) -> (sp_off, rvp_bits)",
                    String::from_utf8_lossy(&s),
                    rounding,
                    sign
                ),
                (cv.0, format!("{:#018x}", cv.1)),
                (rv.0, format!("{:#018x}", rv.1)),
            );
        }
    }
}

// ===========================================================================
// 8. CONFIGS 31 / ERRORS 243 — strtod__unused
// ===========================================================================

unsafe fn strtod_one(l: &Lib, p: *const c_char, want_se: bool) -> (u64, Option<i64>) {
    let mut se: *mut c_char = std::ptr::null_mut();
    let sep: *mut *mut c_char = if want_se {
        &mut se
    } else {
        std::ptr::null_mut()
    };
    let v = (l.strtod__unused)(p, sep);
    (
        v.to_bits(),
        if !want_se {
            None
        } else if se.is_null() {
            Some(i64::MIN)
        } else {
            Some(se as isize as i64 - p as isize as i64)
        },
    )
}

#[test]
fn strtod__unused_shapes() {
    let _g = lock();
    let d = duo();
    let mut fixed: Vec<String> = [
        "0", "1", "-1", "1.5", "1e10", "1E-10", "1e999", "-1e999", "1e-999", "0x1p3", "  12", "+3",
        ".5", "5.", "abc", "", "1e", "1e+", "inf", "nan", "infinity", "-", "+", ".", "e5", "0",
        "00000", "0.0", "-0", "-0.0", "1e308", "1e309", "1e-308", "1e-323", "1e-324",
        "2.2250738585072011e-308", "2.2250738585072012e-308", "9007199254740993",
        "1.7976931348623158e308", "1.7976931348623159e308", "0x0p0", "0X1P+1", "0x", "0xg",
        "  \t\n\r\x0b\x0c -7.5e2", "1.5e+0002", "123456789012345678901234567890",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    // 300-digit numbers
    fixed.push("9".repeat(300));
    fixed.push(format!("1{}", "0".repeat(299)));
    fixed.push(format!("0.{}1", "0".repeat(299)));
    fixed.push(format!("1.{}e-10", "5".repeat(299)));
    fixed.push(format!("-{}.{}", "3".repeat(150), "7".repeat(150)));

    // 2000 randomized numeric strings
    let mut rng = Rng::new(0x8888_9999_aaaa_bbbb);
    for _ in 0..2000 {
        let mut s = String::new();
        match rng.below(6) {
            0 => s.push(' '),
            1 => s.push_str("\t "),
            2 => s.push('-'),
            3 => s.push('+'),
            _ => {}
        }
        let nint = rng.below(22);
        for _ in 0..nint {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        if rng.bool() {
            s.push('.');
            let nfrac = rng.below(22);
            for _ in 0..nfrac {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        match rng.below(4) {
            0 => {}
            1 => {
                s.push('e');
                if rng.bool() {
                    s.push(if rng.bool() { '+' } else { '-' });
                }
                let n = rng.below(5);
                for _ in 0..n {
                    s.push((b'0' + rng.below(10) as u8) as char);
                }
            }
            2 => {
                s.push('E');
                s.push('-');
                let n = 1 + rng.below(4);
                for _ in 0..n {
                    s.push((b'0' + rng.below(10) as u8) as char);
                }
            }
            _ => {
                // trailing garbage: strtod must stop at it
                s.push_str(["xyz", "..", "e", "e+", " 1", "p3"][rng.below(6)]);
            }
        }
        if s.is_empty() {
            s.push('0');
        }
        fixed.push(s);
    }

    unsafe {
        for s in fixed.iter() {
            let buf = cbuf(s.as_bytes());
            let p = buf.as_ptr() as *const c_char;
            for &want_se in [true, false].iter() {
                let cv = strtod_one(&d.c, p, want_se);
                let rv = strtod_one(&d.rs, p, want_se);
                eq(
                    &format!("strtod__unused({:?}, se={}) -> (bits, se_off)", s, want_se),
                    (format!("{:#018x}", cv.0), cv.1),
                    (format!("{:#018x}", rv.0), rv.1),
                );
            }
        }
    }
}

// ===========================================================================
// 9. CONFIGS 32 — the exported dtoa_divmax data symbol
// ===========================================================================

#[test]
fn dtoa_divmax_symbol() {
    let _g = lock();
    let d = duo();
    unsafe {
        let cv: i32 = d.c.data::<i32>("dtoa_divmax");
        let rv: i32 = d.rs.data::<i32>("dtoa_divmax");
        eq("dtoa_divmax", cv, rv);
        eq("dtoa_divmax initial value", cv, 2);
    }
}

// ===========================================================================
// 10. CONFIGS 33 / ERRORS 236 — jsonp_dtostr, precision 0..=31, size 25
// ===========================================================================

#[test]
fn jsonp_dtostr_precisions() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        vals.extend(nan_and_inf());
        vals.extend([
            1.5,
            -1.5,
            3.141592653589793,
            1e300,
            -1e300,
            1e-300,
            12345.6789,
            9007199254740993.0,
            f64::EPSILON,
            2.2250738585072011e-308,
            1e-1,
            1e15,
            1e-16,
            1e-17,
        ]);
        let mut rng = Rng::new(0x9999_aaaa_bbbb_cccc);
        for _ in 0..3000 {
            vals.push(rng.finite_f64());
            vals.push(rng.tame_f64());
        }
        for precision in 0..=31 {
            for &v in vals.iter() {
                let cv = dtostr(&d.c, v, 25, precision);
                let rv = dtostr(&d.rs, v, 25, precision);
                let what = format!("jsonp_dtostr({}, size=25, precision={})", fdesc(v), precision);
                eq(&what, cv, rv);
                // Precisions whose dtoa_r request cannot fit in `char digits[25]`
                // must be rejected. ±0.0, NaN and ±Inf are exempt: they leave
                // `dtoa_r` through `nrv_alloc`, before the `blen <= i` check.
                if precision >= 25 && v.is_finite() && v != 0.0 {
                    let cv = dtostr(&d.c, v, 25, precision);
                    assert_eq!(
                        cv.ret, -1,
                        "C: expected -1 (digits[25] too short) for {}",
                        what
                    );
                    let rv = dtostr(&d.rs, v, 25, precision);
                    assert_eq!(
                        rv.ret, -1,
                        "RUST: expected -1 (digits[25] too short) for {}",
                        what
                    );
                }
            }
        }
        // a few out-of-range precisions on the special values
        for &precision in [-1, -17, 32, 100, i32::MAX, i32::MIN].iter() {
            for &v in specials().iter() {
                let cv = dtostr(&d.c, v, 25, precision);
                let rv = dtostr(&d.rs, v, 25, precision);
                eq(
                    &format!("jsonp_dtostr({}, size=25, precision={})", fdesc(v), precision),
                    cv,
                    rv,
                );
            }
        }
    }
}

// ===========================================================================
// 11. CONFIGS 34 / ERRORS 236, 237 — jsonp_dtostr, size 0..=30
// ===========================================================================

#[test]
fn jsonp_dtostr_buffer_sizes() {
    let _g = lock();
    let d = duo();
    unsafe {
        let mut vals = specials();
        vals.extend(nan_and_inf());
        vals.extend([
            1.5,
            -1.5,
            3.141592653589793,
            1e300,
            -1e300,
            1e-300,
            12345.6789,
            -0.0001,
            1e21,
            -1e-21,
        ]);
        for &precision in [0, 1, 2, 3, 6, 15, 17, 24].iter() {
            for &v in vals.iter() {
                for size in 0..=30usize {
                    let cv = dtostr(&d.c, v, size, precision);
                    let rv = dtostr(&d.rs, v, size, precision);
                    eq(
                        &format!(
                            "jsonp_dtostr({}, size={}, precision={})",
                            fdesc(v),
                            size,
                            precision
                        ),
                        cv,
                        rv,
                    );
                    // ERRORS 237: size == 0 can never succeed
                    if size == 0 {
                        let cv = dtostr(&d.c, v, 0, precision);
                        assert_eq!(cv.ret, -1, "C: jsonp_dtostr with size 0 must return -1");
                        let rv = dtostr(&d.rs, v, 0, precision);
                        assert_eq!(rv.ret, -1, "RUST: jsonp_dtostr with size 0 must return -1");
                    }
                }
            }
        }
    }
}

// ===========================================================================
// 12. CONFIGS 35 / ERRORS 235 — jsonp_strtod
// ===========================================================================

#[derive(PartialEq, Eq, Debug)]
struct ST {
    ret: c_int,
    out_bits: String,
}

/// Builds the `strbuffer_t` with the OWNING library and closes it there too.
unsafe fn strtod_sb(l: &Lib, s: &[u8]) -> ST {
    let mut sb = strbuffer_t::zeroed();
    assert_eq!((l.strbuffer_init)(&mut sb), 0, "{}: strbuffer_init", l.which);
    if !s.is_empty() {
        assert_eq!(
            (l.strbuffer_append_bytes)(&mut sb, s.as_ptr() as *const c_char, s.len()),
            0,
            "{}: strbuffer_append_bytes",
            l.which
        );
    }
    let mut out: f64 = f64::from_bits(0xdead_beef_dead_beef);
    let ret = (l.jsonp_strtod)(&mut sb, &mut out);
    (l.strbuffer_close)(&mut sb);
    ST {
        ret,
        out_bits: format!("{:#018x}", out.to_bits()),
    }
}

#[test]
fn jsonp_strtod_values() {
    let _g = lock();
    let d = duo();
    let mut inputs: Vec<String> = [
        "0",
        "-0",
        "1",
        "1.5",
        "1e300",
        "1e308",
        "1e309",
        "1e999",
        "-1e999",
        "1e-999",
        "0.0000000000000000001",
        "-1",
        "-1.5",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "-1.7976931348623159e308",
        "2.2250738585072014e-308",
        "5e-324",
        "1e-324",
        "0.1",
        "3.141592653589793",
        "9007199254740993",
        "1e-5",
        "1e16",
        "1e17",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    // a 400-digit integer (overflows to +-HUGE_VAL => ERRORS 235)
    inputs.push("1".repeat(400));
    inputs.push(format!("-{}", "9".repeat(400)));
    inputs.push(format!("0.{}", "1".repeat(400)));

    // 2000 randomized *fully consumable* numeric strings. `jsonp_strtod` has
    // `assert(end == value + length)`, so the whole buffer must be a number.
    let mut rng = Rng::new(0xaaaa_bbbb_cccc_dddd);
    for _ in 0..2000 {
        let mut s = String::new();
        if rng.bool() {
            s.push('-');
        }
        let nint = 1 + rng.below(20);
        for _ in 0..nint {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        if rng.bool() {
            s.push('.');
            let nfrac = 1 + rng.below(20);
            for _ in 0..nfrac {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        if rng.bool() {
            s.push(if rng.bool() { 'e' } else { 'E' });
            if rng.bool() {
                s.push(if rng.bool() { '+' } else { '-' });
            }
            let n = 1 + rng.below(4);
            for _ in 0..n {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        inputs.push(s);
    }

    unsafe {
        for s in inputs.iter() {
            let cv = strtod_sb(&d.c, s.as_bytes());
            let rv = strtod_sb(&d.rs, s.as_bytes());
            eq(&format!("jsonp_strtod({:?})", s), cv, rv);
        }
        // ERRORS 235: the documented overflow rejections
        for s in ["1e999", "-1e999"].iter() {
            let cv = strtod_sb(&d.c, s.as_bytes());
            assert_eq!(cv.ret, -1, "C: jsonp_strtod({:?}) must return -1", s);
            let rv = strtod_sb(&d.rs, s.as_bytes());
            assert_eq!(rv.ret, -1, "RUST: jsonp_strtod({:?}) must return -1", s);
        }
        // and the non-overflow underflow case still succeeds
        for s in ["1e-999"].iter() {
            let cv = strtod_sb(&d.c, s.as_bytes());
            assert_eq!(cv.ret, 0, "C: jsonp_strtod({:?}) must return 0", s);
        }
    }
}
