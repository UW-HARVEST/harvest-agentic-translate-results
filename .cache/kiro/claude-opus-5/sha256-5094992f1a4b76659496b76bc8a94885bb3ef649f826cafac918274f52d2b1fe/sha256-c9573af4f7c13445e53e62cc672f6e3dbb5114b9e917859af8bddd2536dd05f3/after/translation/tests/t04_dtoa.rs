//! Phase B/C — dtoa.c and strconv.c (the float formatting/parsing core).
//! CONFIGS rows 17-21 · ERRORS rows 147-150.
mod common;
use common::*;
use std::ffi::{CStr, c_char, c_int, c_void};

/// Values that hit every interesting branch of dtoa / jsonp_dtostr.
fn interesting_doubles() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.1,
        0.2,
        0.3,
        1.0 / 3.0,
        2.0 / 3.0,
        1e-1,
        1e-2,
        1e-3,
        1e-4,   // decpt == -3
        1e-5,   // decpt == -4  => use_exp
        1e-6,
        1e-300,
        1e-320, // subnormal
        5e-324, // smallest subnormal
        f64::MIN_POSITIVE,
        1e15,
        1e16,   // decpt == 17 => use_exp
        1e17,
        1e100,
        1e308,
        f64::MAX,
        -f64::MAX,
        1.7976931348623157e308,
        2.2250738585072014e-308,
        123456789.0,
        1234567890123456.0,
        12345678901234567.0,
        9007199254740992.0,  // 2^53
        9007199254740993.0,
        4503599627370496.0,  // 2^52
        1e22,
        1e23,
        3.14159265358979,
        2.718281828459045,
        -273.15,
        6.02214076e23,
        1.602176634e-19,
        0.0625,
        1024.0,
        65536.0,
        1e-7,
        1e7,
        1e-15,
        1e-16,
        1e-17,
        99999999999999999999.0,
        1.5,
        2.5,
        3.5,
        0.049999999999999996,
        1.0000000000000002,
        0.9999999999999999,
    ];
    // 2^k and 1/2^k
    for k in -60i32..=60 {
        v.push(2f64.powi(k));
        v.push(-(2f64.powi(k)));
    }
    // 10^k
    for k in -30i32..=30 {
        v.push(10f64.powi(k));
    }
    v
}

/* ---------- CONFIGS 17 / ERRORS 150 (== 132): jsonp_dtostr ---------- */

#[test]
fn jsonp_dtostr_all_precisions() {
    let _g = dtoa_guard();
    unsafe {
        let mut vals = interesting_doubles();
        let mut rng = Rng::new(0xD70A_0001);
        for _ in 0..20000 {
            vals.push(rng.f64_finite());
            vals.push(rng.f64_smallish());
        }
        for &v in &vals {
            for prec in 0..=31i32 {
                let mut cb = [0i8; 64];
                let mut rb = [0i8; 64];
                // Real callers use size == MAX_REAL_STR_LENGTH == 25.
                let cn = (c().jsonp_dtostr)(cb.as_mut_ptr(), 25, v, prec);
                let rn = (r().jsonp_dtostr)(rb.as_mut_ptr(), 25, v, prec);
                assert_eq!(
                    cn, rn,
                    "jsonp_dtostr({v:?} bits={:#x}, size=25, prec={prec}) return",
                    v.to_bits()
                );
                if cn >= 0 {
                    let cs_ = CStr::from_ptr(cb.as_ptr()).to_bytes().to_vec();
                    let rs_ = CStr::from_ptr(rb.as_ptr()).to_bytes().to_vec();
                    assert_eq!(
                        String::from_utf8_lossy(&cs_),
                        String::from_utf8_lossy(&rs_),
                        "jsonp_dtostr({v:?} bits={:#x}, prec={prec}) text",
                        v.to_bits()
                    );
                    assert_eq!(cs_.len() as c_int, cn, "returned length == strlen");
                }
            }
        }
    }
}

#[test]
fn jsonp_dtostr_buffer_too_short() {
    let _g = dtoa_guard();
    unsafe {
        // ERRORS 150: the size guard must trip identically for every size.
        for &v in &[0.0f64, 1.0, -1.0, 1e-5, 1e300, 1.0 / 3.0, -1.0 / 7.0, 1e16] {
            for prec in [0i32, 1, 5, 17] {
                for size in 0usize..=30 {
                    let mut cb = [0i8; 64];
                    let mut rb = [0i8; 64];
                    let cn = (c().jsonp_dtostr)(cb.as_mut_ptr(), size, v, prec);
                    let rn = (r().jsonp_dtostr)(rb.as_mut_ptr(), size, v, prec);
                    assert_eq!(
                        cn, rn,
                        "jsonp_dtostr({v:?}, size={size}, prec={prec}) return"
                    );
                    if cn >= 0 {
                        assert_eq!(
                            CStr::from_ptr(cb.as_ptr()).to_bytes(),
                            CStr::from_ptr(rb.as_ptr()).to_bytes(),
                            "jsonp_dtostr({v:?}, size={size}, prec={prec}) text"
                        );
                    }
                }
            }
        }
    }
}

/* ---------- CONFIGS 18 / ERRORS 147: jsonp_strtod ---------- */

#[test]
fn jsonp_strtod_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut lits: Vec<String> = vec![
            "0".into(),
            "-0".into(),
            "0.0".into(),
            "1".into(),
            "-1".into(),
            "1.5".into(),
            "1e5".into(),
            "1E5".into(),
            "1e+5".into(),
            "1e-5".into(),
            "1.5e300".into(),
            "1.5e-300".into(),
            "123456789012345678901234567890".into(),
            "0.000000000000000000001".into(),
            "1e308".into(),
            "1e309".into(),   // ERRORS 147: overflow => -1
            "-1e309".into(),  // ERRORS 147: overflow => -1
            "1e999".into(),
            "-1e999".into(),
            "1e-999".into(),  // underflow: NOT an error in this C
            "1e-400".into(),
            "4.9406564584124654e-324".into(),
            "2.2250738585072011e-308".into(),
            "1.7976931348623157e308".into(),
            "1.7976931348623159e308".into(),
        ];
        let mut rng = Rng::new(0xD70A_0002);
        for _ in 0..6000 {
            let sign = if rng.bool() { "-" } else { "" };
            let ip = rng.range_i64(0, 1_000_000_000);
            let fp = rng.range_i64(0, 1_000_000_000);
            let ex = rng.range_i64(-330, 330);
            lits.push(match rng.below(4) {
                0 => format!("{sign}{ip}"),
                1 => format!("{sign}{ip}.{fp}"),
                2 => format!("{sign}{ip}e{ex}"),
                _ => format!("{sign}{ip}.{fp}e{ex}"),
            });
        }
        for lit in &lits {
            let mut csb = StrBuffer::default();
            let mut rsb = StrBuffer::default();
            assert_eq!((c().strbuffer_init)(&mut csb), 0);
            assert_eq!((r().strbuffer_init)(&mut rsb), 0);
            let b = lit.as_bytes();
            (c().strbuffer_append_bytes)(&mut csb, b.as_ptr() as *const c_char, b.len());
            (r().strbuffer_append_bytes)(&mut rsb, b.as_ptr() as *const c_char, b.len());
            let mut cv = f64::NAN;
            let mut rv = f64::NAN;
            let cr = (c().jsonp_strtod)(&mut csb, &mut cv);
            let rr = (r().jsonp_strtod)(&mut rsb, &mut rv);
            assert_eq!(cr, rr, "jsonp_strtod({lit:?}) return");
            if cr == 0 {
                assert_eq!(
                    cv.to_bits(),
                    rv.to_bits(),
                    "jsonp_strtod({lit:?}) value C={cv:?} R={rv:?}"
                );
            }
            (c().strbuffer_close)(&mut csb);
            (r().strbuffer_close)(&mut rsb);
        }
    }
}

/* ---------- CONFIGS 19 / ERRORS 149: dtoa_r ---------- */

#[test]
fn dtoa_r_modes_and_ndigits() {
    let _g = dtoa_guard();
    unsafe {
        let mut vals = interesting_doubles();
        let mut rng = Rng::new(0xD70A_0003);
        for _ in 0..4000 {
            vals.push(rng.f64_finite());
            vals.push(rng.f64_smallish());
        }
        vals.push(f64::NAN);
        vals.push(f64::INFINITY);
        vals.push(f64::NEG_INFINITY);

        for &v in &vals {
            // jsonp_dtostr only ever uses mode 0 and 2, but dtoa_r is exported,
            // so exercise the full documented mode range.
            for mode in 0..=5i32 {
                for nd in [0i32, 1, 2, 5, 15, 16, 17, 18, 25] {
                    let mut cbuf = [0i8; 40];
                    let mut rbuf = [0i8; 40];
                    let mut cdec: c_int = -12345;
                    let mut rdec: c_int = -12345;
                    let mut csgn: c_int = -12345;
                    let mut rsgn: c_int = -12345;
                    let mut crve: *mut c_char = std::ptr::null_mut();
                    let mut rrve: *mut c_char = std::ptr::null_mut();
                    let cp = (c().dtoa_r)(
                        v, mode, nd, &mut cdec, &mut csgn, &mut crve,
                        cbuf.as_mut_ptr(), 40,
                    );
                    let rp = (r().dtoa_r)(
                        v, mode, nd, &mut rdec, &mut rsgn, &mut rrve,
                        rbuf.as_mut_ptr(), 40,
                    );
                    let tag = format!(
                        "dtoa_r(bits={:#018x}, mode={mode}, nd={nd})",
                        v.to_bits()
                    );
                    assert_eq!(cp.is_null(), rp.is_null(), "{tag} null-ness");
                    if cp.is_null() {
                        continue;
                    }
                    let cstr = CStr::from_ptr(cp).to_bytes().to_vec();
                    let rstr = CStr::from_ptr(rp).to_bytes().to_vec();
                    assert_eq!(
                        String::from_utf8_lossy(&cstr),
                        String::from_utf8_lossy(&rstr),
                        "{tag} digits"
                    );
                    assert_eq!(cdec, rdec, "{tag} decpt");
                    assert_eq!(csgn, rsgn, "{tag} sign");
                    // rve points just past the last digit
                    let coff = crve as usize - cp as usize;
                    let roff = rrve as usize - rp as usize;
                    assert_eq!(coff, roff, "{tag} rve offset");
                    assert_eq!(coff, cstr.len(), "{tag} rve == end of digits");
                    // buf was supplied, so no freedtoa needed
                    assert_eq!(cp, cbuf.as_mut_ptr(), "{tag} used caller buffer");
                    assert_eq!(rp, rbuf.as_mut_ptr(), "{tag} used caller buffer");
                }
            }
        }
    }
}

#[test]
fn dtoa_r_short_buffer_falls_back_to_malloc() {
    let _g = dtoa_guard();
    unsafe {
        // ERRORS 149: with a too-short buf, dtoa_r allocates instead of failing;
        // with blen == 0 / buf == NULL it also allocates. Either way both
        // libraries must agree, and the result must be freed with freedtoa.
        let vals = [1.0f64 / 3.0, 1e300, 1.2345678901234567e-5, 0.0, -12345.6789];
        for &v in &vals {
            for &blen in &[0usize, 1, 2, 3, 5, 10] {
                for mode in [0i32, 2, 3] {
                    let mut cbuf = [0i8; 40];
                    let mut rbuf = [0i8; 40];
                    let mut cdec = 0;
                    let mut rdec = 0;
                    let mut csgn = 0;
                    let mut rsgn = 0;
                    let mut crve = std::ptr::null_mut();
                    let mut rrve = std::ptr::null_mut();
                    let cbp = if blen == 0 {
                        std::ptr::null_mut()
                    } else {
                        cbuf.as_mut_ptr()
                    };
                    let rbp = if blen == 0 {
                        std::ptr::null_mut()
                    } else {
                        rbuf.as_mut_ptr()
                    };
                    let cp = (c().dtoa_r)(v, mode, 17, &mut cdec, &mut csgn, &mut crve, cbp, blen);
                    let rp = (r().dtoa_r)(v, mode, 17, &mut rdec, &mut rsgn, &mut rrve, rbp, blen);
                    let tag = format!("dtoa_r short buf v={v:?} blen={blen} mode={mode}");
                    assert_eq!(cp.is_null(), rp.is_null(), "{tag} null-ness");
                    if cp.is_null() {
                        continue;
                    }
                    assert_eq!(
                        CStr::from_ptr(cp).to_bytes(),
                        CStr::from_ptr(rp).to_bytes(),
                        "{tag} digits"
                    );
                    assert_eq!((cdec, csgn), (rdec, rsgn), "{tag} decpt/sign");
                    if cp != cbuf.as_mut_ptr() {
                        (c().freedtoa)(cp);
                    }
                    if rp != rbuf.as_mut_ptr() {
                        (r().freedtoa)(rp);
                    }
                }
            }
        }
    }
}

/* ---------- CONFIGS 20: dtoa / freedtoa ---------- */

#[test]
fn dtoa_and_freedtoa() {
    let _g = dtoa_guard();
    unsafe {
        let mut vals = interesting_doubles();
        let mut rng = Rng::new(0xD70A_0004);
        for _ in 0..2000 {
            vals.push(rng.f64_finite());
        }
        for &v in &vals {
            for mode in 0..=3i32 {
                for nd in [0i32, 1, 6, 17] {
                    let mut cdec = 0;
                    let mut rdec = 0;
                    let mut csgn = 0;
                    let mut rsgn = 0;
                    let mut crve = std::ptr::null_mut();
                    let mut rrve = std::ptr::null_mut();
                    let cp = (c().dtoa)(v, mode, nd, &mut cdec, &mut csgn, &mut crve);
                    let rp = (r().dtoa)(v, mode, nd, &mut rdec, &mut rsgn, &mut rrve);
                    let tag =
                        format!("dtoa(bits={:#018x}, mode={mode}, nd={nd})", v.to_bits());
                    assert_eq!(cp.is_null(), rp.is_null(), "{tag} null-ness");
                    if cp.is_null() {
                        continue;
                    }
                    assert_eq!(
                        CStr::from_ptr(cp).to_bytes(),
                        CStr::from_ptr(rp).to_bytes(),
                        "{tag} digits"
                    );
                    assert_eq!((cdec, csgn), (rdec, rsgn), "{tag} decpt/sign");
                    let coff = crve as usize - cp as usize;
                    let roff = rrve as usize - rp as usize;
                    assert_eq!(coff, roff, "{tag} rve offset");
                    // dtoa() caches its result in dtoa_result; freeing it
                    // explicitly must work in both.
                    (c().freedtoa)(cp);
                    (r().freedtoa)(rp);
                }
            }
        }
    }
}

/* ---------- CONFIGS 21: gethex ---------- */

#[test]
fn gethex_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut lits: Vec<String> = vec![
            "0x1p0".into(),
            "0x1p+0".into(),
            "0x1p-0".into(),
            "0x1.8p3".into(),
            "0x1.8p+3".into(),
            "0x0p0".into(),
            "0x0".into(),
            "0xfp0".into(),
            "0xFFFFFFFFFFFFFFFFp0".into(),
            "0x1p1023".into(),
            "0x1p1024".into(),
            "0x1p-1022".into(),
            "0x1p-1074".into(),
            "0x1p-1075".into(),
            "0x1.fffffffffffffp1023".into(),
            "0x.8p1".into(),
            "0x8.p1".into(),
            "0x1.0000000000000000001p0".into(),
            "0xzz".into(),
            "0x".into(),
            "0x1P4".into(),
        ];
        let mut rng = Rng::new(0xD70A_0005);
        for _ in 0..3000 {
            let mant: u64 = rng.next_u64();
            let e = rng.range_i64(-1100, 1100);
            lits.push(format!("0x{mant:x}p{e}"));
            lits.push(format!("0x{:x}.{:x}p{}", rng.next_u32(), rng.next_u32(), e));
        }
        for lit in &lits {
            // gethex expects `*sp` to point just past the "0x" prefix.
            for rounding in 0..=3i32 {
                for sign in [0i32, 1] {
                    let bytes = cbytes(lit.as_bytes());
                    let start = if lit.starts_with("0x") { 2 } else { 0 };
                    let mut cp: *const c_char = bytes.as_ptr().add(start) as *const c_char;
                    let mut rp: *const c_char = bytes.as_ptr().add(start) as *const c_char;
                    let mut cu = U { d: f64::NAN };
                    let mut ru = U { d: f64::NAN };
                    (c().gethex)(&mut cp, &mut cu, rounding, sign);
                    (r().gethex)(&mut rp, &mut ru, rounding, sign);
                    let tag = format!("gethex({lit:?}, rounding={rounding}, sign={sign})");
                    assert_eq!(
                        cu.d.to_bits(),
                        ru.d.to_bits(),
                        "{tag} value C={:?} R={:?}",
                        cu.d,
                        ru.d
                    );
                    assert_eq!(
                        cp as usize - bytes.as_ptr() as usize,
                        rp as usize - bytes.as_ptr() as usize,
                        "{tag} advanced pointer"
                    );
                }
            }
        }
    }
}

/* ---------- the exported strtod__unused ---------- */

#[test]
fn strtod_unused_differential() {
    let _g = dtoa_guard();
    unsafe {
        let mut lits: Vec<String> = vec![
            "0".into(),
            "1".into(),
            "-1".into(),
            "  12.5  ".into(),
            "1e10".into(),
            "1e-10".into(),
            "1e400".into(),
            "-1e400".into(),
            "1e-400".into(),
            "inf".into(),
            "Infinity".into(),
            "nan".into(),
            "NaN(123)".into(),
            "0x1p4".into(),
            "0X1P4".into(),
            "abc".into(),
            "".into(),
            "+.5".into(),
            ".5".into(),
            "5.".into(),
            "1.7976931348623157e308".into(),
            "4.9406564584124654e-324".into(),
            "2.2250738585072011e-308".into(),
            "123456789012345678901234567890.12345".into(),
            "1e308garbage".into(),
        ];
        let mut rng = Rng::new(0xD70A_0006);
        for _ in 0..5000 {
            let sign = if rng.bool() { "-" } else { "" };
            let ip = rng.next_u64() % 1_000_000_000_000;
            let fp = rng.next_u64() % 1_000_000_000_000;
            let ex = rng.range_i64(-340, 340);
            lits.push(format!("{sign}{ip}.{fp}e{ex}"));
        }
        for lit in &lits {
            let bytes = cbytes(lit.as_bytes());
            let mut cend: *mut c_char = std::ptr::null_mut();
            let mut rend: *mut c_char = std::ptr::null_mut();
            let cv = (c().strtod__unused)(bytes.as_ptr() as *const c_char, &mut cend);
            let rv = (r().strtod__unused)(bytes.as_ptr() as *const c_char, &mut rend);
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "strtod__unused({lit:?}) value C={cv:?} R={rv:?}"
            );
            assert_eq!(
                cend as usize - bytes.as_ptr() as usize,
                rend as usize - bytes.as_ptr() as usize,
                "strtod__unused({lit:?}) end pointer"
            );
            // NULL `se` must be accepted too
            let cv2 = (c().strtod__unused)(bytes.as_ptr() as *const c_char, std::ptr::null_mut());
            let rv2 = (r().strtod__unused)(bytes.as_ptr() as *const c_char, std::ptr::null_mut());
            assert_eq!(cv2.to_bits(), rv2.to_bits(), "strtod__unused({lit:?}) NULL se");
        }
    }
}

/* ---------- the real dumper path: JSON_REAL_PRECISION (CONFIGS 63) ---------- */

#[test]
fn dumps_real_precision_matrix() {
    let _g = dtoa_guard();
    unsafe {
        let mut vals = interesting_doubles();
        let mut rng = Rng::new(0xD70A_0007);
        for _ in 0..4000 {
            vals.push(rng.f64_finite());
            vals.push(rng.f64_smallish());
        }
        for &v in &vals {
            if !v.is_finite() {
                continue;
            }
            let cj = (c().json_real)(v);
            let rj = (r().json_real)(v);
            assert_eq!(cj.is_null(), rj.is_null());
            if cj.is_null() {
                continue;
            }
            for prec in 0..=31usize {
                let flags = JSON_ENCODE_ANY | json_real_precision(prec);
                let cd = dumps(c(), cj, flags);
                let rd = dumps(r(), rj, flags);
                assert_bytes_eq(
                    &format!("json_dumps(real bits={:#018x}, prec={prec})", v.to_bits()),
                    &cd,
                    &rd,
                );
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* Make sure the harness's unused import warning does not hide anything. */
#[test]
fn _void_unused() {
    let _ = std::mem::size_of::<*mut c_void>();
}
