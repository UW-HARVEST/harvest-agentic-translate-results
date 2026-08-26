//! Phase B/C: differential tests for jsdtoa.c and the jsvalue.c number
//! conversions. CONFIGS.md rows 308-355; the jsdtoa.c ERRORS.md rows 461-476 and the
//! jsV_numberto* rows.
mod common;
use common::*;
use std::ffi::{c_char, c_int, CStr};

fn interesting_doubles() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.1,
        0.2,
        0.3,
        1.0 / 3.0,
        2.0 / 3.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,          // smallest subnormal
        1e-323,
        2.2250738585072011e-308, // subnormal boundary
        2.2250738585072014e-308,
        1e-7,
        1e-6,
        1e-5,
        9.999999e-7,
        1.0000001e-6,
        1e20,
        1e21,
        1e22,
        9.999999999999999e20,
        1.0000000000000001e21,
        1e100,
        1e-100,
        1e308,
        1e-308,
        i32::MIN as f64,
        i32::MAX as f64,
        u32::MAX as f64,
        (i32::MIN as f64) - 1.0,
        (i32::MAX as f64) + 1.0,
        (u32::MAX as f64) + 1.0,
        2f64.powi(31),
        2f64.powi(32),
        2f64.powi(52),
        2f64.powi(53),
        2f64.powi(53) + 1.0,
        2f64.powi(53) - 1.0,
        -(2f64.powi(53)),
        i64::MIN as f64,
        i64::MAX as f64,
        1.7976931348623157e308,
        4.9406564584124654e-324,
        0.000001,
        123456789.0,
        1234567890123456789.0,
        1e16,
        1e17,
        9007199254740992.0,
        1.1125369292536007e-308,
        3.141592653589793,
        2.718281828459045,
        1.7976931348623155e308,
        // values needing 17 significant digits
        0.1 + 0.2,
        9.109383701528e-31,
        1.2345678901234567e-5,
        5.0e-324,
        1.0000000000000002,
        0.30000000000000004,
    ];
    for e in -330i32..=310 {
        v.push(10f64.powi(e));
        v.push(-(10f64.powi(e)));
        v.push(1.5 * 10f64.powi(e));
        v.push(9.999999999999999 * 10f64.powi(e));
    }
    for e in -1080i32..=1024 {
        v.push(2f64.powi(e.max(-1074).min(1023)));
    }
    for i in 0..1000 {
        v.push(i as f64);
        v.push(-(i as f64));
        v.push(i as f64 + 0.5);
        v.push(1.0 / (i as f64 + 1.0));
    }
    v
}

fn random_doubles(n: usize, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.f64_sane()).collect()
}

fn random_bits(n: usize, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| rng.f64_any()).collect()
}

#[test]
fn t_grisu2() {
    let p = libs();
    unsafe {
        let mut all = interesting_doubles();
        all.extend(random_doubles(20000, 0xabc));
        all.extend(random_bits(20000, 0xdef));
        for x in all {
            // js_grisu2 asserts on 0 and on non-finite input, so skip those
            // (they are covered by ERRORS rows 203/204 in a separate test)
            if x == 0.0 || !x.is_finite() {
                continue;
            }
            let v = x.abs(); // js_grisu2 is called on the magnitude
            let mut ba = [0i8; 64];
            let mut bb = [0i8; 64];
            let mut ka: c_int = -777;
            let mut kb: c_int = -777;
            let na = p.c.js_grisu2(v, ba.as_mut_ptr(), &mut ka);
            let nb = p.rs.js_grisu2(v, bb.as_mut_ptr(), &mut kb);
            assert_eq!(
                (na, ka, &ba[..]),
                (nb, kb, &bb[..]),
                "js_grisu2({v:e} bits={:#x})",
                v.to_bits()
            );
        }
    }
}

/// NOTE: `js_fmtexp` (jsdtoa.c:24) formats the decimal digits of `e` into
/// `char se[9]`. A 10-digit exponent (|e| >= 1e9, and therefore `INT_MAX` /
/// `INT_MIN`) writes `se[9]`, one byte past the array — undefined behaviour in
/// the C, which crashes the process. Those inputs are therefore not
/// differentially testable; every well-defined exponent is covered below.
/// (In practice `js_fmtexp` is only ever called from `jsV_numbertostring` with
/// |e| <= ~330.)
#[test]
fn t_fmtexp() {
    let p = libs();
    unsafe {
        let mut es: Vec<c_int> = vec![
            0, 1, -1, 9, -9, 10, -10, 99, -99, 100, -100, 307, -307, 308, -308, 323, -323, 324,
            -324, 999, -999, 1000, -1000, 12345, -12345, 999_999_999, -999_999_999,
        ];
        for e in -400..=400 {
            es.push(e);
        }
        let mut rng = Rng::new(0x3e3e);
        for _ in 0..5000 {
            es.push((rng.next_u32() % 1_000_000_000) as c_int);
            es.push(-((rng.next_u32() % 1_000_000_000) as c_int));
        }
        for e in es {
            assert!(e.unsigned_abs() < 1_000_000_000);
            let mut ba = [0i8; 64];
            let mut bb = [0i8; 64];
            p.c.js_fmtexp(ba.as_mut_ptr(), e);
            p.rs.js_fmtexp(bb.as_mut_ptr(), e);
            assert_eq!(ba, bb, "js_fmtexp({e})");
        }
    }
}

#[test]
fn t_itoa() {
    let p = libs();
    unsafe {
        let mut vals: Vec<c_int> = vec![
            0,
            1,
            -1,
            9,
            10,
            -10,
            99,
            100,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
            i32::MAX - 1,
            1000000000,
            -1000000000,
        ];
        let mut rng = Rng::new(0x1a2b);
        for _ in 0..20000 {
            vals.push(rng.next_u32() as c_int);
        }
        for v in vals {
            let mut ba = [0i8; 64];
            let mut bb = [0i8; 64];
            let ra = p.c.js_itoa(ba.as_mut_ptr(), v);
            let rb = p.rs.js_itoa(bb.as_mut_ptr(), v);
            // returned pointer is an offset into the caller's buffer
            let oa = ra.offset_from(ba.as_ptr());
            let ob = rb.offset_from(bb.as_ptr());
            assert_eq!(oa, ob, "js_itoa({v}) returned offset");
            assert_eq!(
                CStr::from_ptr(ra).to_bytes(),
                CStr::from_ptr(rb).to_bytes(),
                "js_itoa({v}) text"
            );
            assert_eq!(ba, bb, "js_itoa({v}) buffer");
        }
    }
}

fn number_strings() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        " ".into(),
        "  \t\n\r ".into(),
        "0".into(),
        "-0".into(),
        "+0".into(),
        "1".into(),
        "-1".into(),
        "+1".into(),
        "1.".into(),
        ".1".into(),
        ".".into(),
        "-.".into(),
        "1e".into(),
        "1e+".into(),
        "1e-".into(),
        "1e1".into(),
        "1e+1".into(),
        "1e-1".into(),
        "1E5".into(),
        "0x10".into(),
        "0X10".into(),
        "0x".into(),
        "0xg".into(),
        "0b101".into(),
        "0o17".into(),
        "017".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "+Infinity".into(),
        "infinity".into(),
        "Inf".into(),
        "NaN".into(),
        "nan".into(),
        "-NaN".into(),
        "  12  ".into(),
        "12abc".into(),
        "abc".into(),
        "1_000".into(),
        "1e999".into(),
        "1e-999".into(),
        "1e308".into(),
        "1e309".into(),
        "1e-323".into(),
        "1e-324".into(),
        "1e-400".into(),
        "0.0000000000000000000001".into(),
        "123456789012345678901234567890".into(),
        "1.7976931348623157e308".into(),
        "1.7976931348623159e308".into(),
        "4.9406564584124654e-324".into(),
        "2.2250738585072011e-308".into(),
        "9007199254740993".into(),
        "0.1".into(),
        "0.30000000000000004".into(),
        "1e511".into(),
        "1e512".into(),
        "1e-511".into(),
        "1e-512".into(),
        "1".repeat(30),
        format!("1.{}e10", "9".repeat(40)),
        format!("0.{}", "0".repeat(40)),
        format!("{}e-500", "9".repeat(25)),
        "\u{a0}12".into(),
        "\u{feff}12".into(),
        "\u{2028}12".into(),
        "\u{2029}12".into(),
        "\u{000b}12".into(),
        "\u{000c}12".into(),
        "12\u{a0}".into(),
    ];
    let mut rng = Rng::new(0x9e3779b9);
    for _ in 0..3000 {
        // structured numeric strings
        let sign = ["", "-", "+"][rng.below(3) as usize];
        let int: String = (0..rng.below(20))
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        let frac = if rng.below(2) == 0 {
            let d: String = (0..rng.below(20))
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            format!(".{d}")
        } else {
            String::new()
        };
        let exp = if rng.below(2) == 0 {
            format!(
                "{}{}{}",
                ["e", "E"][rng.below(2) as usize],
                ["", "-", "+"][rng.below(3) as usize],
                rng.below(1000)
            )
        } else {
            String::new()
        };
        v.push(format!("{sign}{int}{frac}{exp}"));
    }
    for _ in 0..2000 {
        v.push(rng.ascii_string(14));
    }
    v
}

#[test]
fn t_strtod() {
    let p = libs();
    unsafe {
        for s in number_strings() {
            let cs = cstr(&s);
            let mut ea: *mut c_char = std::ptr::null_mut();
            let mut eb: *mut c_char = std::ptr::null_mut();
            let a = p.c.js_strtod(cs.as_ptr(), &mut ea);
            let b = p.rs.js_strtod(cs.as_ptr(), &mut eb);
            assert_eq!(a.to_bits(), b.to_bits(), "js_strtod({s:?}) value");
            let oa = ea.offset_from(cs.as_ptr() as *mut c_char);
            let ob = eb.offset_from(cs.as_ptr() as *mut c_char);
            assert_eq!(oa, ob, "js_strtod({s:?}) endptr");
            // endptr == NULL variant (ERRORS row 218)
            let a2 = p.c.js_strtod(cs.as_ptr(), std::ptr::null_mut());
            let b2 = p.rs.js_strtod(cs.as_ptr(), std::ptr::null_mut());
            assert_eq!(a2.to_bits(), b2.to_bits(), "js_strtod({s:?}) NULL endptr");
        }
    }
}

#[test]
fn t_stringtofloat() {
    let p = libs();
    unsafe {
        for s in number_strings() {
            let cs = cstr(&s);
            let mut ea: *mut c_char = std::ptr::null_mut();
            let mut eb: *mut c_char = std::ptr::null_mut();
            let a = p.c.js_stringtofloat(cs.as_ptr(), &mut ea);
            let b = p.rs.js_stringtofloat(cs.as_ptr(), &mut eb);
            assert_eq!(a.to_bits(), b.to_bits(), "js_stringtofloat({s:?}) value");
            let oa = ea.offset_from(cs.as_ptr() as *mut c_char);
            let ob = eb.offset_from(cs.as_ptr() as *mut c_char);
            assert_eq!(oa, ob, "js_stringtofloat({s:?}) endptr");
        }
    }
}

#[test]
fn t_strtol() {
    let p = libs();
    unsafe {
        let mut inputs = number_strings();
        inputs.extend([
            "zz".into(),
            "ZZ".into(),
            "0z".into(),
            "-ff".into(),
            "  -0x1f".into(),
            "7fffffff".into(),
            "ffffffffffffffff".into(),
            "1".repeat(70),
            "z".repeat(40),
            "1010101010101010101010101010101010101010".into(),
        ]);
        for s in inputs {
            let cs = cstr(&s);
            // radix 0 and 1 are the degenerate cases (CONFIGS row 127).
            // NOTE: `js_strtol` (jsvalue.c:7) loops while `table[c] < base`, and
            // the table maps every non-alphanumeric byte - INCLUDING the NUL
            // terminator - to 80. So any `base > 80` makes the C run off the end
            // of the buffer forever (SIGSEGV): undefined behaviour, not
            // differentially testable. base <= 80 always stops at the NUL.
            // The only public caller, jsB_parseInt (jsbuiltin.c:52), rejects
            // radix < 2 || radix > 36 before calling, so this is unreachable
            // from JS; it is reachable across the raw FFI boundary.
            for radix in [0i32, 1, 2, 3, 8, 10, 16, 20, 35, 36, 37, 79, 80, -1, -100, i32::MIN] {
                let mut ea: *mut c_char = std::ptr::null_mut();
                let mut eb: *mut c_char = std::ptr::null_mut();
                let a = p.c.js_strtol(cs.as_ptr(), &mut ea, radix);
                let b = p.rs.js_strtol(cs.as_ptr(), &mut eb, radix);
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "js_strtol({s:?}, radix={radix}) value"
                );
                let oa = ea.offset_from(cs.as_ptr() as *mut c_char);
                let ob = eb.offset_from(cs.as_ptr() as *mut c_char);
                assert_eq!(oa, ob, "js_strtol({s:?}, radix={radix}) endptr");
            }
        }
    }
}

#[test]
fn t_numberto_int() {
    let p = libs();
    unsafe {
        let mut all = interesting_doubles();
        all.extend(random_doubles(30000, 0x515));
        all.extend(random_bits(30000, 0x616));
        for x in all {
            assert_eq!(
                p.c.jsV_numbertointeger(x),
                p.rs.jsV_numbertointeger(x),
                "jsV_numbertointeger({x:e} {:#x})",
                x.to_bits()
            );
            assert_eq!(
                p.c.jsV_numbertoint32(x),
                p.rs.jsV_numbertoint32(x),
                "jsV_numbertoint32({x:e} {:#x})",
                x.to_bits()
            );
            assert_eq!(
                p.c.jsV_numbertouint32(x),
                p.rs.jsV_numbertouint32(x),
                "jsV_numbertouint32({x:e} {:#x})",
                x.to_bits()
            );
            assert_eq!(
                p.c.jsV_numbertoint16(x),
                p.rs.jsV_numbertoint16(x),
                "jsV_numbertoint16({x:e} {:#x})",
                x.to_bits()
            );
            assert_eq!(
                p.c.jsV_numbertouint16(x),
                p.rs.jsV_numbertouint16(x),
                "jsV_numbertouint16({x:e} {:#x})",
                x.to_bits()
            );
        }
    }
}

#[test]
fn t_numbertostring() {
    let p = libs();
    unsafe {
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        let mut all = interesting_doubles();
        all.extend(random_doubles(20000, 0x717));
        all.extend(random_bits(20000, 0x818));
        for x in all {
            let mut ba = [0i8; 32];
            let mut bb = [0i8; 32];
            set_cur(&p.c);
            let ra = p.c.jsV_numbertostring(jc, ba.as_mut_ptr(), x);
            set_cur(&p.rs);
            let rb = p.rs.jsV_numbertostring(jr, bb.as_mut_ptr(), x);
            let sa = from_c(ra);
            let sb = from_c(rb);
            assert_eq!(
                sa, sb,
                "jsV_numbertostring({x:e} bits={:#x})",
                x.to_bits()
            );
            // the buffer itself must match too when the result points into it
            let in_a = ra >= ba.as_ptr() && ra < ba.as_ptr().add(32);
            let in_b = rb >= bb.as_ptr() && rb < bb.as_ptr().add(32);
            assert_eq!(in_a, in_b, "jsV_numbertostring({x:e}) result locality");
            if in_a {
                assert_eq!(ba, bb, "jsV_numbertostring({x:e}) buffer");
            }
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
}

#[test]
fn t_stringtonumber() {
    let p = libs();
    unsafe {
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        for s in number_strings() {
            let cs = cstr(&s);
            set_cur(&p.c);
            let a = p.c.jsV_stringtonumber(jc, cs.as_ptr());
            set_cur(&p.rs);
            let b = p.rs.jsV_stringtonumber(jr, cs.as_ptr());
            assert_eq!(a.to_bits(), b.to_bits(), "jsV_stringtonumber({s:?})");
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
}

/// Round-trip through the JS layer: number -> string -> number, exercising
/// jsV_numbertostring and jsV_stringtonumber inside the interpreter.
#[test]
fn t_number_roundtrip_via_js() {
    let mut rng = Rng::new(0x2024);
    for _ in 0..300 {
        let x = rng.f64_sane();
        if !x.is_finite() {
            continue;
        }
        let src = format!("var x = {:e}; print(x, String(x), +String(x), x.toString());", x);
        diff_dostring(0, &src);
    }
}
