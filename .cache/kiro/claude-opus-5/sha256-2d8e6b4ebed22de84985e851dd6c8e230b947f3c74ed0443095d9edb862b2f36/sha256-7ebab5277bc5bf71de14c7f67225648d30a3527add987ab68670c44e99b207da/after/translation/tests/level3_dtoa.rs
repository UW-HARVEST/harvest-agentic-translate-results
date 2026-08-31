//! Level 3: dtoa.c and strconv.c
//!
//! `dtoa_r` is the workhorse behind every real number jansson prints, so it is
//! swept over a large space of (value, mode, ndigits) triples.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_double, c_int, c_void};

/// Interesting doubles: boundaries of dtoa's fast paths, powers of ten,
/// subnormals, values with short and long shortest-representations, etc.
fn probe_doubles() -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        0.5,
        0.1,
        0.2,
        0.3,
        1.0 / 3.0,
        2.0 / 3.0,
        1e-1,
        1e-2,
        1e-3,
        1e-4,
        1e-5,
        1e-10,
        1e-20,
        1e-100,
        1e-200,
        1e-300,
        1e-310,
        1e-320,
        5e-324,           // smallest subnormal
        f64::MIN_POSITIVE, // smallest normal
        1e1,
        1e2,
        1e15,
        1e16,
        1e17,
        1e20,
        1e21,
        1e22,
        1e23,
        1e100,
        1e200,
        1e300,
        1e308,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        9007199254740992.0,  // 2^53
        9007199254740993.0,  // not representable, rounds
        9007199254740994.0,
        4503599627370496.0, // 2^52
        123456789.0,
        1234567890123456.0,
        12345678901234567.0,
        123456789012345678901234567890.0,
        3.141592653589793,
        2.718281828459045,
        1.7976931348623157e308,
        2.2250738585072014e-308,
        2.2250738585072011e-308, // largest subnormal
        0.000001,
        0.0000001,
        1.5,
        2.5,
        3.5,
        -2.5,
        1e-6,
        99999999999999999999.0,
        1.0000000000000002,
        0.9999999999999999,
        1e-323,
        4e-324,
        1.1125369292536007e-308,
        // values that stress the "round to even" tie handling
        1.0 / 1024.0,
        8.98846567431158e307,
        5.562684646268003e-309,
    ];

    // powers of two across the whole exponent range
    for e in -1074i32..=1023 {
        if e % 7 == 0 {
            v.push(f64::from_bits(0).max(0.0) + libm_ldexp(1.0, e));
        }
    }
    // powers of ten
    for e in -323i32..=308 {
        v.push(parse_pow10(e));
    }
    // pseudo-random bit patterns (finite only, plus a few non-finite)
    let mut s: u64 = 0x243f_6a88_85a3_08d3;
    for _ in 0..4000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        v.push(f64::from_bits(s));
    }
    // pseudo-random "human" decimals
    let mut s: u64 = 0x1234_5678;
    for _ in 0..2000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mant = (s >> 20) % 1_000_000_000;
        let e = ((s >> 5) % 60) as i32 - 30;
        v.push(mant as f64 * parse_pow10(e));
    }
    v
}

fn libm_ldexp(x: f64, e: i32) -> f64 {
    let mut r = x;
    let mut e = e;
    while e > 0 {
        let s = e.min(1000);
        r *= f64::from_bits(0x3ff0000000000000).powi(0) * 2f64.powi(s);
        e -= s;
    }
    while e < 0 {
        let s = (-e).min(1000);
        r /= 2f64.powi(s);
        e += s;
    }
    r
}

fn parse_pow10(e: i32) -> f64 {
    format!("1e{e}").parse::<f64>().unwrap()
}

struct DtoaOut {
    ret: Option<Vec<u8>>,
    decpt: c_int,
    sign: c_int,
    /// offset of `rve` inside the returned string, or -1
    rve_off: i64,
    /// whether the returned pointer was inside the caller-supplied buffer
    in_buf: bool,
    /// the full caller buffer contents after the call
    buf: Vec<u8>,
}

impl std::fmt::Debug for DtoaOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DtoaOut")
            .field("ret", &self.ret.as_ref().map(|b| String::from_utf8_lossy(b).into_owned()))
            .field("decpt", &self.decpt)
            .field("sign", &self.sign)
            .field("rve_off", &self.rve_off)
            .field("in_buf", &self.in_buf)
            .finish()
    }
}

impl PartialEq for DtoaOut {
    fn eq(&self, o: &Self) -> bool {
        self.ret == o.ret
            && self.decpt == o.decpt
            && self.sign == o.sign
            && self.rve_off == o.rve_off
            && self.in_buf == o.in_buf
            && self.buf == o.buf
    }
}

unsafe fn call_dtoa_r(
    l: &Lib,
    val: f64,
    mode: c_int,
    ndigits: c_int,
    blen: usize,
) -> DtoaOut {
    let f: Symbol<FnDtoaR> = l.sym("dtoa_r");
    let free: Symbol<FnFreedtoa> = l.sym("freedtoa");

    let mut buf = vec![0x7fu8; blen.max(1) + 8];
    let bufp = if blen == 0 {
        std::ptr::null_mut()
    } else {
        buf.as_mut_ptr() as *mut c_char
    };
    let mut decpt: c_int = -99999;
    let mut sign: c_int = -99999;
    let mut rve: *mut c_char = std::ptr::null_mut();

    let p = f(val, mode, ndigits, &mut decpt, &mut sign, &mut rve, bufp, blen);

    let ret = if p.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
    };
    let rve_off = if p.is_null() || rve.is_null() {
        -1
    } else {
        rve.offset_from(p) as i64
    };
    let in_buf = !p.is_null() && !bufp.is_null() && p == bufp;
    let bufsnap = if blen == 0 { Vec::new() } else { buf.clone() };

    if !p.is_null() && !in_buf {
        // heap allocated by dtoa_r's own rv_alloc/nrv_alloc; only freedtoa()
        // knows how to release it (the payload is offset into the block).
        free(p);
    }

    DtoaOut {
        ret,
        decpt,
        sign,
        rve_off,
        in_buf,
        buf: bufsnap,
    }
}

#[test]
fn dtoa_r_mode0_matches() {
    let (c, r) = libs();
    for v in probe_doubles() {
        unsafe {
            let a = call_dtoa_r(c, v, 0, 0, 25);
            let b = call_dtoa_r(r, v, 0, 0, 25);
            assert_eq!(a, b, "dtoa_r({v:e} [{:#018x}], mode 0)", v.to_bits());
        }
    }
}

#[test]
fn dtoa_r_mode2_matches() {
    let (c, r) = libs();
    let vals = probe_doubles();
    for v in &vals {
        for nd in [1i32, 2, 3, 5, 6, 8, 15, 16, 17, 18, 20, 24] {
            unsafe {
                let a = call_dtoa_r(c, *v, 2, nd, 25);
                let b = call_dtoa_r(r, *v, 2, nd, 25);
                assert_eq!(
                    a, b,
                    "dtoa_r({v:e} [{:#018x}], mode 2, nd {nd})",
                    v.to_bits()
                );
            }
        }
    }
}

#[test]
fn dtoa_r_all_modes_matches() {
    let (c, r) = libs();
    // A smaller value set but every mode 0..=9 and a wide ndigits range,
    // including negative ndigits.
    let vals: Vec<f64> = probe_doubles().into_iter().step_by(11).collect();
    for v in &vals {
        for mode in 0i32..=9 {
            for nd in [-5i32, -1, 0, 1, 2, 4, 7, 15, 17, 19, 25, 40] {
                unsafe {
                    let a = call_dtoa_r(c, *v, mode, nd, 40);
                    let b = call_dtoa_r(r, *v, mode, nd, 40);
                    assert_eq!(
                        a, b,
                        "dtoa_r({v:e} [{:#018x}], mode {mode}, nd {nd})",
                        v.to_bits()
                    );
                }
            }
        }
    }
}

#[test]
fn dtoa_r_buffer_variants_match() {
    let (c, r) = libs();
    let vals: Vec<f64> = probe_doubles().into_iter().step_by(37).collect();
    for v in &vals {
        // blen == 0 => NULL buf, dtoa_r must allocate.
        // Small blen => must fall back to allocating (or fail) identically.
        for blen in [0usize, 1, 2, 3, 4, 5, 8, 12, 20, 25, 32, 64] {
            for (mode, nd) in [(0i32, 0i32), (2, 17), (2, 1), (3, 5), (5, 20)] {
                unsafe {
                    let a = call_dtoa_r(c, *v, mode, nd, blen);
                    let b = call_dtoa_r(r, *v, mode, nd, blen);
                    assert_eq!(
                        a, b,
                        "dtoa_r({v:e} [{:#018x}], mode {mode}, nd {nd}, blen {blen})",
                        v.to_bits()
                    );
                }
            }
        }
    }
}

#[test]
fn dtoa_r_ilim_zero_path_matches() {
    // Targets the `ilim == 0 && j + k >= 0` fast path, which is the one that
    // reads `pfive[k-1]` and therefore `pfive[-1]` when k == 0. Reaching it
    // needs mode 3 or 5 with ndigits == -k - 1, so sweep ndigits densely over
    // values spanning every decimal exponent k.
    let (c, r) = libs();
    let mut vals: Vec<f64> = Vec::new();
    // dense coverage of k == 0 (1.0 <= |x| < 10) plus every other magnitude
    let mut s: u64 = 0x51ed_270b_c7f2_1a3d;
    for _ in 0..400 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let frac = (s >> 11) as f64 / (1u64 << 53) as f64;
        vals.push(1.0 + 9.0 * frac);
        vals.push(-(1.0 + 9.0 * frac));
    }
    for e in -320i32..=308 {
        let base: f64 = format!("1e{e}").parse().unwrap();
        if base == 0.0 || !base.is_finite() {
            continue;
        }
        vals.push(base);
        vals.push(base * 1.5);
        vals.push(base * 9.99);
        vals.push(base * 5.0);
    }
    vals.extend(probe_doubles());

    for v in &vals {
        for mode in [3i32, 5] {
            for nd in -12i32..=4 {
                for blen in [0usize, 40] {
                    unsafe {
                        let a = call_dtoa_r(c, *v, mode, nd, blen);
                        let b = call_dtoa_r(r, *v, mode, nd, blen);
                        assert_eq!(
                            a, b,
                            "dtoa_r({v:e} [{:#018x}], mode {mode}, nd {nd}, blen {blen})",
                            v.to_bits()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dtoa_divmax_matches() {
    let (c, r) = libs();
    let sc: Symbol<*mut c_int> = c.sym("dtoa_divmax");
    let sr: Symbol<*mut c_int> = r.sym("dtoa_divmax");
    assert_eq!(unsafe { **sc }, unsafe { **sr }, "dtoa_divmax initial value");
}

// -------------------------------------------------------------- jsonp_dtostr

fn dtostr(l: &Lib, value: f64, precision: c_int, size: usize) -> (c_int, Vec<u8>) {
    let f: Symbol<FnJsonpDtostr> = l.sym("jsonp_dtostr");
    let mut buf = vec![0x5au8; size + 16];
    let rc = unsafe { f(buf.as_mut_ptr() as *mut c_char, size, value, precision) };
    buf.truncate(size + 16);
    (rc, buf)
}

#[test]
fn jsonp_dtostr_matches() {
    let (c, r) = libs();
    for v in probe_doubles() {
        if !v.is_finite() {
            continue; // jansson never calls dtostr with non-finite values
        }
        for precision in [0i32, 1, 2, 3, 6, 10, 15, 16, 17] {
            for size in [32usize, 64, 100] {
                let a = dtostr(c, v, precision, size);
                let b = dtostr(r, v, precision, size);
                assert_eq!(
                    a.0, b.0,
                    "jsonp_dtostr({v:e} [{:#018x}], p {precision}, size {size}) rc",
                    v.to_bits()
                );
                assert_eq!(
                    a.1, b.1,
                    "jsonp_dtostr({v:e} [{:#018x}], p {precision}, size {size}) buffer",
                    v.to_bits()
                );
            }
        }
    }
}

#[test]
fn jsonp_dtostr_short_buffers_match() {
    let (c, r) = libs();
    let vals: Vec<f64> = probe_doubles()
        .into_iter()
        .filter(|v| v.is_finite())
        .step_by(29)
        .collect();
    for v in &vals {
        for size in 1usize..40 {
            for precision in [0i32, 5, 17] {
                let a = dtostr(c, *v, precision, size);
                let b = dtostr(r, *v, precision, size);
                assert_eq!(
                    a, b,
                    "jsonp_dtostr({v:e} [{:#018x}], p {precision}, size {size})",
                    v.to_bits()
                );
            }
        }
    }
}

#[test]
fn jsonp_dtostr_nonfinite_matches() {
    // jansson guards against these, but the functions must still agree.
    let (c, r) = libs();
    for v in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -f64::NAN] {
        for precision in [0i32, 17] {
            let a = dtostr(c, v, precision, 64);
            let b = dtostr(r, v, precision, 64);
            assert_eq!(a, b, "jsonp_dtostr({v}, p {precision})");
        }
    }
}

// -------------------------------------------------------------- jsonp_strtod

/// `jsonp_strtod` takes a strbuffer whose contents it parses; build one on the
/// target library so the assert inside it (end == value+length) holds.
fn strtod_via_strbuffer(l: &Lib, text: &[u8]) -> (c_int, u64) {
    let init: Symbol<FnStrbufferInit> = l.sym("strbuffer_init");
    let app: Symbol<FnStrbufferAppendBytes> = l.sym("strbuffer_append_bytes");
    let close: Symbol<FnStrbufferClose> = l.sym("strbuffer_close");
    let f: Symbol<FnJsonpStrtod> = l.sym("jsonp_strtod");
    let mut b = StrbufferT::default();
    let mut out: c_double = -12345.0;
    unsafe {
        assert_eq!(init(&mut b), 0);
        assert_eq!(app(&mut b, text.as_ptr() as *const c_char, text.len()), 0);
        let rc = f(&mut b, &mut out);
        close(&mut b);
        (rc, out.to_bits())
    }
}

#[test]
fn jsonp_strtod_matches() {
    let (c, r) = libs();
    let mut texts: Vec<Vec<u8>> = vec![
        b"0".to_vec(),
        b"-0".to_vec(),
        b"0.0".to_vec(),
        b"1".to_vec(),
        b"-1".to_vec(),
        b"1.0".to_vec(),
        b"0.1".to_vec(),
        b"3.141592653589793".to_vec(),
        b"1e10".to_vec(),
        b"1e-10".to_vec(),
        b"1E+10".to_vec(),
        b"1e308".to_vec(),
        b"1e309".to_vec(),   // overflow -> ERANGE
        b"-1e309".to_vec(),  // overflow -> ERANGE
        b"1e-323".to_vec(),
        b"1e-324".to_vec(),  // underflow -> ERANGE but not HUGE_VAL
        b"1e-400".to_vec(),
        b"5e-324".to_vec(),
        b"1.7976931348623157e308".to_vec(),
        b"1.7976931348623159e308".to_vec(),
        b"2.2250738585072014e-308".to_vec(),
        b"9007199254740993".to_vec(),
        b"123456789012345678901234567890".to_vec(),
        b"0.000000000000000000000000001".to_vec(),
        b"1234567890.0987654321".to_vec(),
        b"-2.5e-5".to_vec(),
        b"1e1000".to_vec(),
        b"-1e1000".to_vec(),
        // very long digit strings (exercise dtoa/strtod bigint paths)
        format!("0.{}", "1234567890".repeat(30)).into_bytes(),
        format!("{}e-100", "9".repeat(60)).into_bytes(),
        format!("{}", "9".repeat(400)).into_bytes(),
        format!("0.{}1", "0".repeat(320)).into_bytes(),
    ];
    let mut s: u64 = 0xfeed_face;
    for _ in 0..1500 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mant = (s >> 12) % 10_000_000_000_000_000;
        let e = ((s >> 3) % 700) as i64 - 350;
        let neg = if s & 1 == 0 { "-" } else { "" };
        texts.push(format!("{neg}{mant}e{e}").into_bytes());
    }
    // round-trip every probe double through its shortest representation
    for v in probe_doubles() {
        if v.is_finite() {
            let mut buf = vec![0u8; 64];
            let (rc, b) = {
                let f: Symbol<FnJsonpDtostr> = c.sym("jsonp_dtostr");
                let rc = unsafe { f(buf.as_mut_ptr() as *mut c_char, 64, v, 17) };
                (rc, buf)
            };
            if rc > 0 {
                texts.push(b[..rc as usize].to_vec());
            }
        }
    }

    for t in &texts {
        let a = strtod_via_strbuffer(c, t);
        let b = strtod_via_strbuffer(r, t);
        assert_eq!(
            a,
            b,
            "jsonp_strtod({:?}) -> C {:?}/{:e} vs Rust {:?}/{:e}",
            String::from_utf8_lossy(t),
            a.0,
            f64::from_bits(a.1),
            b.0,
            f64::from_bits(b.1)
        );
    }
}

#[test]
fn jsonp_strtod_round_trips_through_dtostr() {
    // The pair (jsonp_dtostr, jsonp_strtod) must be a lossless round trip in
    // both libraries, and must agree with each other.
    let (c, r) = libs();
    for v in probe_doubles() {
        if !v.is_finite() {
            continue;
        }
        for l in [c, r] {
            let (rc, buf) = dtostr(l, v, 17, 64);
            assert!(rc > 0, "{}: dtostr({v:e}) rc {rc}", l.name);
            let text = &buf[..rc as usize];
            let (srrc, bits) = strtod_via_strbuffer(l, text);
            assert_eq!(srrc, 0, "{}: strtod({:?})", l.name, String::from_utf8_lossy(text));
            assert_eq!(
                bits,
                v.to_bits(),
                "{}: round trip {v:e} via {:?}",
                l.name,
                String::from_utf8_lossy(text)
            );
        }
    }
}

// ------------------------------------------------- newly required dtoa exports

#[test]
fn dtoa_and_freedtoa_match() {
    let (c, r) = libs();
    let fc: Symbol<FnDtoa> = c.sym("dtoa");
    let fr: Symbol<FnDtoa> = r.sym("dtoa");
    let gc: Symbol<FnFreedtoa> = c.sym("freedtoa");
    let gr: Symbol<FnFreedtoa> = r.sym("freedtoa");

    let vals: Vec<f64> = probe_doubles().into_iter().step_by(13).collect();
    for v in &vals {
        for mode in 0i32..=5 {
            for nd in [0i32, 1, 5, 17, 25] {
                unsafe {
                    let mut dc: c_int = -1;
                    let mut sc: c_int = -1;
                    let mut rc_: *mut c_char = std::ptr::null_mut();
                    let pc = fc(*v, mode, nd, &mut dc, &mut sc, &mut rc_);
                    let sc_str = if pc.is_null() {
                        None
                    } else {
                        Some(std::ffi::CStr::from_ptr(pc).to_bytes().to_vec())
                    };
                    let rvec = if pc.is_null() || rc_.is_null() {
                        -1
                    } else {
                        rc_.offset_from(pc) as i64
                    };

                    let mut dr: c_int = -1;
                    let mut sr: c_int = -1;
                    let mut rr_: *mut c_char = std::ptr::null_mut();
                    let pr = fr(*v, mode, nd, &mut dr, &mut sr, &mut rr_);
                    let sr_str = if pr.is_null() {
                        None
                    } else {
                        Some(std::ffi::CStr::from_ptr(pr).to_bytes().to_vec())
                    };
                    let rver = if pr.is_null() || rr_.is_null() {
                        -1
                    } else {
                        rr_.offset_from(pr) as i64
                    };

                    assert_eq!(
                        (&sc_str, dc, sc, rvec),
                        (&sr_str, dr, sr, rver),
                        "dtoa({v:e} [{:#018x}], mode {mode}, nd {nd})",
                        v.to_bits()
                    );

                    // freedtoa() is the only valid way to release a dtoa()
                    // result; neither implementation is NULL safe, so only
                    // real pointers are passed.
                    if !pc.is_null() {
                        gc(pc);
                    }
                    if !pr.is_null() {
                        gr(pr);
                    }
                }
            }
        }
    }
}
