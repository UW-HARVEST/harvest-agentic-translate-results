//! Phase B rows 12-23: dtoa / strtod / strtol / number<->string conversions.
mod common;
use common::*;
use std::os::raw::{c_char, c_int};

fn interesting_doubles() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.1,
        0.2,
        0.3,
        1.0 / 3.0,
        1e-300,
        1e300,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,
        1e21,
        1e-7,
        1e-6,
        123456789.0,
        1234567890123456789.0,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967295.0,
        4294967296.0,
        4503599627370496.0,
        9007199254740992.0,
        9007199254740993.0,
        -9007199254740992.0,
        32767.0,
        32768.0,
        -32768.0,
        -32769.0,
        65535.0,
        65536.0,
        1e17,
        1e18,
        1e19,
        1e20,
        1e22,
        0.000001,
        1234.5678,
        -1234.5678,
    ];
    for e in -320i32..=308 {
        v.push(10f64.powi(e));
        v.push(-(10f64.powi(e)));
    }
    for e in -1074i32..=1023 {
        v.push(2f64.powi(e));
    }
    let mut rng = Rng::new(0x1234_5678);
    for _ in 0..8000 {
        v.push(rng.nice_f64());
        v.push(rng.f64_bits());
    }
    v
}

#[test]
fn row12_itoa() {
    let p = pair();
    let mut cases: Vec<c_int> = vec![0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for i in -2000..2000 {
        cases.push(i);
    }
    let mut rng = Rng::new(0xabc);
    for _ in 0..20000 {
        cases.push(rng.i32());
    }
    for a in cases {
        let mut ba: [c_char; 64] = [0x55; 64];
        let mut bb: [c_char; 64] = [0x55; 64];
        let ra = unsafe { rstr((p.c.js_itoa)(ba.as_mut_ptr(), a)) };
        let rb = unsafe { rstr((p.r.js_itoa)(bb.as_mut_ptr(), a)) };
        assert_eq!(ra, rb, "js_itoa({a})");
        assert_eq!(&ba[..], &bb[..], "js_itoa buffer for {a}");
    }
}

/// `js_fmtexp` formats the exponent into a 9-byte stack buffer (`char se[9]`),
/// so exponents needing 10+ digits overflow that buffer in the C original —
/// undefined behaviour, and unreachable in practice (the only caller,
/// `jsV_numbertostring`, passes `point-1` with `|point| < 400`).  The
/// differential test therefore covers the whole representable domain
/// `|e| <= 999999999`.
#[test]
fn row13_fmtexp() {
    let p = pair();
    let mut cases: Vec<c_int> = Vec::new();
    for e in -1200..1200 {
        cases.push(e);
    }
    for e in [
        0,
        999_999_999,
        -999_999_999,
        99_999_999,
        -99_999_999,
        123_456_789,
        -123_456_789,
        9,
        10,
        99,
        100,
        -9,
        -10,
        -99,
        -100,
    ] {
        cases.push(e);
    }
    let mut rng = Rng::new(0xf00d);
    for _ in 0..20000 {
        let e = (rng.next_u32() % 1_000_000_000) as c_int;
        cases.push(e);
        cases.push(-e);
    }
    for e in cases {
        let mut ba: [c_char; 64] = [0x55; 64];
        let mut bb: [c_char; 64] = [0x55; 64];
        unsafe { (p.c.js_fmtexp)(ba.as_mut_ptr(), e) };
        unsafe { (p.r.js_fmtexp)(bb.as_mut_ptr(), e) };
        assert_eq!(&ba[..], &bb[..], "js_fmtexp(e={e})");
    }
}

/// `js_grisu2`'s contract (established by its only caller,
/// `jsV_numbertostring`) is: finite and non-zero.  `±0.0` trips
/// `assert(x.f >= y.f)` at `jsdtoa.c:387` in the C build, so it is excluded
/// here and covered as an ERRORS.md row instead.
#[test]
fn row14_grisu2() {
    let p = pair();
    for d in interesting_doubles() {
        if !d.is_finite() || d == 0.0 {
            continue;
        }
        let mut ba: [c_char; 64] = [0x55; 64];
        let mut bb: [c_char; 64] = [0x55; 64];
        let mut ka: c_int = -999;
        let mut kb: c_int = -999;
        let na = unsafe { (p.c.js_grisu2)(d, ba.as_mut_ptr(), &mut ka) };
        let nb = unsafe { (p.r.js_grisu2)(d, bb.as_mut_ptr(), &mut kb) };
        assert_eq!((na, ka), (nb, kb), "grisu2({d:?}) n/K");
        assert_eq!(
            &ba[..na.max(0) as usize],
            &bb[..nb.max(0) as usize],
            "grisu2({d:?}) digits"
        );
    }
}

fn numeric_strings() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        " ".into(),
        "\t\n\r\u{b}\u{c} ".into(),
        "0".into(),
        "-0".into(),
        "+0".into(),
        "1".into(),
        "-1".into(),
        "  42  ".into(),
        "3.14".into(),
        ".5".into(),
        "5.".into(),
        "1e10".into(),
        "1E10".into(),
        "1e+10".into(),
        "1e-10".into(),
        "1e".into(),
        "1e+".into(),
        "1e999999".into(),
        "1e-999999".into(),
        "0x10".into(),
        "0X10".into(),
        "0xZ".into(),
        "0x".into(),
        "0b101".into(),
        "0o17".into(),
        "017".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "+Infinity".into(),
        "inf".into(),
        "nan".into(),
        "NaN".into(),
        "abc".into(),
        "12abc".into(),
        "--1".into(),
        "1.2.3".into(),
        "1_000".into(),
        ".".into(),
        "-.".into(),
        "1e1e1".into(),
        "9007199254740993".into(),
        "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
        "0.00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001".into(),
        "4.9406564584124654e-324".into(),
        "2.2250738585072011e-308".into(),
        "1.7976931348623157e308".into(),
        "1.7976931348623159e308".into(),
    ];
    let mut rng = Rng::new(0xfeed);
    let alphabet: &[u8] = b"0123456789.eE+- xXaAbBfFnNiIty\t";
    for _ in 0..6000 {
        let n = rng.below(14) as usize;
        let s: String = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize] as char)
            .collect();
        v.push(s);
    }
    for _ in 0..3000 {
        let d = Rng::new(rng.next_u64()).nice_f64();
        v.push(format!("{d:?}"));
        v.push(format!("{d:e}"));
    }
    v
}

#[test]
fn row15_strtod_with_endptr() {
    let p = pair();
    for s in numeric_strings() {
        let buf = cbuf(s.as_bytes());
        let mut ea: *mut c_char = std::ptr::null_mut();
        let mut eb: *mut c_char = std::ptr::null_mut();
        let da = unsafe { (p.c.js_strtod)(buf.as_ptr(), &mut ea) };
        let db = unsafe { (p.r.js_strtod)(buf.as_ptr(), &mut eb) };
        assert!(dbl_eq(da, db), "js_strtod({s:?}): C={da:?} RUST={db:?}");
        let oa = ea as usize - buf.as_ptr() as usize;
        let ob = eb as usize - buf.as_ptr() as usize;
        assert_eq!(oa, ob, "js_strtod endptr for {s:?}");
    }
}

#[test]
fn row16_strtod_null_endptr() {
    let p = pair();
    for s in numeric_strings() {
        let buf = cbuf(s.as_bytes());
        let da = unsafe { (p.c.js_strtod)(buf.as_ptr(), std::ptr::null_mut()) };
        let db = unsafe { (p.r.js_strtod)(buf.as_ptr(), std::ptr::null_mut()) };
        assert!(dbl_eq(da, db), "js_strtod({s:?}, NULL)");
    }
}

fn strtol_check(s: &str, radix: c_int) {
    let p = pair();
    let buf = cbuf(s.as_bytes());
    let mut ea: *mut c_char = std::ptr::null_mut();
    let mut eb: *mut c_char = std::ptr::null_mut();
    let da = unsafe { (p.c.js_strtol)(buf.as_ptr(), &mut ea, radix) };
    let db = unsafe { (p.r.js_strtol)(buf.as_ptr(), &mut eb, radix) };
    assert!(
        dbl_eq(da, db),
        "js_strtol({s:?}, radix={radix}): C={da:?} RUST={db:?}"
    );
    let oa = ea as usize - buf.as_ptr() as usize;
    let ob = eb as usize - buf.as_ptr() as usize;
    assert_eq!(oa, ob, "js_strtol endptr for {s:?} radix={radix}");
}

fn strtol_strings() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "".into(),
        " ".into(),
        "0".into(),
        "-0".into(),
        "+0".into(),
        "0x".into(),
        "0x0".into(),
        "0xff".into(),
        "0XFF".into(),
        "0777".into(),
        "9".into(),
        "z".into(),
        "Z".into(),
        "zz".into(),
        "  -0x1F ".into(),
        "1111111111111111111111111111111111111111".into(),
        "-99999999999999999999999999999".into(),
        "7fffffffffffffff".into(),
        "  \t+12345".into(),
    ];
    let mut rng = Rng::new(0xba5e);
    let alphabet: &[u8] = b"0123456789abcdefghzZ+- xX\t";
    for _ in 0..4000 {
        let n = rng.below(12) as usize;
        v.push(
            (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize] as char)
                .collect(),
        );
    }
    v
}

#[test]
fn row17_strtol_radix_auto() {
    for s in strtol_strings() {
        strtol_check(&s, 0);
    }
}

#[test]
fn row18_strtol_common_radices() {
    for s in strtol_strings() {
        for r in [2, 8, 10, 16, 36, 3, 7, 11, 20, 35] {
            strtol_check(&s, r);
        }
    }
}

/// `js_strtol` classifies characters through a 256-byte table whose "not a
/// digit" marker is the value 80, and its loop condition is `table[c] < base`.
/// For `base > 80` the NUL terminator itself becomes a valid digit and the C
/// loop walks off the end of the buffer — undefined behaviour, and unreachable
/// from the library (`jsB_parseInt` rejects `radix < 2 || radix > 36`, the other
/// callers hard-code 10 and 16).  The test therefore sweeps the whole defined
/// range `base <= 80`, including 0, 1, 37..80 and negative bases.
#[test]
fn row19_strtol_out_of_range_radix() {
    for s in ["0", "1", "z", "10", "", " 7", "zzzz", "0x10", "777", "Zz9"] {
        for r in [1, 37, 38, 50, 79, 80, -1, -16, i32::MIN, i32::MIN + 1] {
            strtol_check(s, r);
        }
    }
}

#[test]
fn row20_stringtofloat() {
    let p = pair();
    for s in numeric_strings() {
        let buf = cbuf(s.as_bytes());
        let mut ea: *mut c_char = std::ptr::null_mut();
        let mut eb: *mut c_char = std::ptr::null_mut();
        let da = unsafe { (p.c.js_stringtofloat)(buf.as_ptr(), &mut ea) };
        let db = unsafe { (p.r.js_stringtofloat)(buf.as_ptr(), &mut eb) };
        assert!(dbl_eq(da, db), "js_stringtofloat({s:?})");
        let oa = ea as usize - buf.as_ptr() as usize;
        let ob = eb as usize - buf.as_ptr() as usize;
        assert_eq!(oa, ob, "js_stringtofloat endptr for {s:?}");
    }
}

#[test]
fn row21_number_to_int_conversions() {
    let p = pair();
    for d in interesting_doubles() {
        let a = unsafe {
            (
                (p.c.jsV_numbertointeger)(d),
                (p.c.jsV_numbertoint32)(d),
                (p.c.jsV_numbertouint32)(d),
                (p.c.jsV_numbertoint16)(d),
                (p.c.jsV_numbertouint16)(d),
            )
        };
        let b = unsafe {
            (
                (p.r.jsV_numbertointeger)(d),
                (p.r.jsV_numbertoint32)(d),
                (p.r.jsV_numbertouint32)(d),
                (p.r.jsV_numbertoint16)(d),
                (p.r.jsV_numbertouint16)(d),
            )
        };
        assert_eq!(a, b, "jsV_numberto*({d:?})");
    }
}

#[test]
fn row22_numbertostring() {
    let p = pair();
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    for d in interesting_doubles() {
        let mut ba: [c_char; 64] = [0x55; 64];
        let mut bb: [c_char; 64] = [0x55; 64];
        let sa = unsafe { rstr((p.c.jsV_numbertostring)(jc, ba.as_mut_ptr(), d)) };
        let sb = unsafe { rstr((p.r.jsV_numbertostring)(jr, bb.as_mut_ptr(), d)) };
        assert_eq!(sa, sb, "jsV_numbertostring({d:?})");
    }
    unsafe { (p.c.js_freestate)(jc) };
    unsafe { (p.r.js_freestate)(jr) };
}

#[test]
fn row23_stringtonumber() {
    let p = pair();
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    for s in numeric_strings() {
        let buf = cbuf(s.as_bytes());
        let da = unsafe { (p.c.jsV_stringtonumber)(jc, buf.as_ptr()) };
        let db = unsafe { (p.r.jsV_stringtonumber)(jr, buf.as_ptr()) };
        assert!(dbl_eq(da, db), "jsV_stringtonumber({s:?}) C={da:?} R={db:?}");
    }
    unsafe { (p.c.js_freestate)(jc) };
    unsafe { (p.r.js_freestate)(jr) };
}

#[test]
fn extra_isarrayindex_and_runeat() {
    let p = pair();
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    let mut names: Vec<String> = vec![
        "".into(),
        "0".into(),
        "00".into(),
        "01".into(),
        "1".into(),
        "9".into(),
        "10".into(),
        "4294967294".into(),
        "4294967295".into(),
        "4294967296".into(),
        "-1".into(),
        "1.5".into(),
        "1e3".into(),
        " 1".into(),
        "1 ".into(),
        "2147483647".into(),
        "2147483648".into(),
        "99999999999999999999".into(),
        "abc".into(),
        "0x10".into(),
    ];
    let mut rng = Rng::new(0x1a1a);
    for _ in 0..4000 {
        names.push(format!("{}", rng.next_u32()));
        let n = rng.below(8) as usize;
        names.push(
            (0..n)
                .map(|_| b"0123456789-.abc"[rng.below(15) as usize] as char)
                .collect(),
        );
    }
    for n in &names {
        let buf = cbuf(n.as_bytes());
        let mut ia: c_int = -7;
        let mut ib: c_int = -7;
        let ra = unsafe { (p.c.js_isarrayindex)(jc, buf.as_ptr(), &mut ia) };
        let rb = unsafe { (p.r.js_isarrayindex)(jr, buf.as_ptr(), &mut ib) };
        assert_eq!((ra, ia), (rb, ib), "js_isarrayindex({n:?})");
    }
    for s in ["", "a", "abc", "héllo", "\u{1f600}x", "\u{ffff}"] {
        let buf = cbuf(s.as_bytes());
        for i in -2..12 {
            let ra = unsafe { (p.c.js_runeat)(jc, buf.as_ptr(), i) };
            let rb = unsafe { (p.r.js_runeat)(jr, buf.as_ptr(), i) };
            assert_eq!(ra, rb, "js_runeat({s:?}, {i})");
        }
    }
    unsafe { (p.c.js_freestate)(jc) };
    unsafe { (p.r.js_freestate)(jr) };
}
