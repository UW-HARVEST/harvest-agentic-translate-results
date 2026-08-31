//! `strtod__unused()` — David M. Gay's `strtod` from dtoa.c, exported under a
//! renamed symbol so that it does not collide with libc's `strtod`.
//!
//! Nothing inside jansson calls it, but the C shared object exports it, so the
//! Rust shared object must export it with matching behaviour.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int};

fn probe_doubles() -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 1.0 / 3.0, 1e-10, 1e10, 1e100, 1e-100,
        1e308, 1e-308, 5e-324, f64::MAX, f64::MIN, f64::EPSILON,
        3.141592653589793, 2.718281828459045, 1e15, 1e16, 1e17, 1e21, 1e22,
        9007199254740992.0, 9007199254740993.0, 2.2250738585072014e-308,
        2.2250738585072011e-308, 1.0000000000000002, 0.9999999999999999,
    ];
    let mut s: u64 = 0x243f_6a88_85a3_08d3;
    for _ in 0..1500 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let f = f64::from_bits(s);
        if f.is_finite() {
            v.push(f);
        }
    }
    v
}

/// The input corpus: valid and malformed decimal literals, hex floats,
/// inf/nan spellings, digit-count and exponent boundaries, exact halfway cases
/// and subnormals.
fn build_texts() -> Vec<Vec<u8>> {
    let mut texts: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"0".to_vec(),
        b"-0".to_vec(),
        b"+0".to_vec(),
        b"1".to_vec(),
        b"1.5".to_vec(),
        b"-1.5e3".to_vec(),
        b"1e".to_vec(),
        b"1e+".to_vec(),
        b"1e-".to_vec(),
        b".5".to_vec(),
        b"5.".to_vec(),
        b".".to_vec(),
        b"abc".to_vec(),
        b"  12.5xyz".to_vec(),
        b"inf".to_vec(),
        b"Infinity".to_vec(),
        b"-inf".to_vec(),
        b"nan".to_vec(),
        b"NaN".to_vec(),
        b"nan(1234)".to_vec(),
        b"0x1p3".to_vec(),
        b"0X1.8p+1".to_vec(),
        b"0x".to_vec(),
        b"0x.8p0".to_vec(),
        b"0xabcdefp-4".to_vec(),
        b"1e309".to_vec(),
        b"1e-400".to_vec(),
        b"1.7976931348623157e308".to_vec(),
        b"9007199254740993".to_vec(),
        b"2.2250738585072014e-308".to_vec(),
        b"5e-324".to_vec(),
        b"0.000000000000000000000000001".to_vec(),
        format!("0.{}", "1234567890".repeat(30)).into_bytes(),
        format!("{}", "9".repeat(400)).into_bytes(),
        format!("{}e-100", "9".repeat(60)).into_bytes(),
        format!("0.{}1", "0".repeat(320)).into_bytes(),
        format!("1e{}", "9".repeat(12)).into_bytes(),
    ];
    // hex floats (gethex path)
    for h in [
        "0x0", "0x1", "0x1p0", "0x1p1", "0x1p-1", "0x1.8p1", "0x.8p0", "0x8.",
        "0x.", "0x", "0X1P10", "0x1p1024", "0x1p-1075", "0x1.fffffffffffffp1023",
        "0x1.0000000000001p0", "0x1.00000000000008p0", "0x1.00000000000018p0",
        "0x10000000000000000", "0xffffffffffffffffffffffff", "0xdeadbeefp-8",
        "0x1abcp", "0x1abcp+", "0x1abcp-", "0x1abcpz", "0xzz", "0x1.2.3",
        "-0x1p3", "+0x1p3", "  0x1p3", "0x1p99999999999999999999",
        "0x1p-99999999999999999999", "0x0.0000000000001p-1022", "0x3p-1075",
    ] {
        texts.push(h.as_bytes().to_vec());
    }
    // inf / nan spellings (INFNAN_CHECK is enabled for IEEE_8087)
    for s in [
        "inf", "INF", "Inf", "iNf", "infi", "infin", "infini", "infinit",
        "infinity", "INFINITY", "InFiNiTy", "infinityx", "-inf", "+inf",
        "  inf", "in", "i", "nan", "NAN", "NaN", "nAn", "na", "n", "nanx",
        "nan()", "nan(0)", "nan(1)", "nan(123)", "nan(0x1)", "nan(abc)",
        "nan(0xfffffffffffff)", "nan(", "nan(1", "-nan", "+nan(3)",
        "nan(1)garbage",
    ] {
        texts.push(s.as_bytes().to_vec());
    }
    // whitespace and sign handling
    for s in [
        " 1", "\t1", "\n1", "\x0b1", "\x0c1", "\r1", "   \t\n 1", "- 1", "+ 1",
        "-", "+", "--1", "++1", "-+1", " ", "\t", "", "1 ", "1\t",
    ] {
        texts.push(s.as_bytes().to_vec());
    }
    // digit-count boundaries around DBL_DIG (15), 19 and Ten_pmax (22)
    for nd in [1usize, 8, 9, 10, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 40, 41, 50] {
        let d: String = (0..nd).map(|i| char::from(b'1' + (i % 9) as u8)).collect();
        for e in [
            -400i32, -330, -324, -323, -310, -308, -100, -30, -23, -22, -1, 0, 1,
            22, 23, 30, 100, 300, 308, 309, 400,
        ] {
            texts.push(format!("{d}e{e}").into_bytes());
            texts.push(format!("-{d}e{e}").into_bytes());
            texts.push(format!("0.{d}e{e}").into_bytes());
        }
        texts.push(d.clone().into_bytes());
        texts.push(format!("{}", "9".repeat(nd)).into_bytes());
        texts.push(format!("0.{}", "9".repeat(nd)).into_bytes());
        texts.push(format!("{}5", "0".repeat(nd)).into_bytes());
    }
    // exact halfway cases (these drive the bigcomp / rounding logic)
    for k in 0..80u32 {
        let bits = (1u64 << 52) | ((k as u64) << 40) | 1;
        let v = f64::from_bits(bits);
        texts.push(format!("{v:.30e}").into_bytes());
        let half = f64::from_bits(bits) + f64::from_bits(bits + 1);
        texts.push(format!("{:.40e}", half / 2.0).into_bytes());
    }
    // subnormal boundary
    for k in 0..60u64 {
        let v = f64::from_bits(k);
        texts.push(format!("{v:.40e}").into_bytes());
        texts.push(format!("{v:.60e}").into_bytes());
    }
    // exponent field edge cases
    for s in [
        "1e19999", "1e20000", "1e-19999", "1e-20000", "1e999999999999999999999",
        "1e-999999999999999999999", "1e00000000000000000005", "1e+0", "1e-0",
        "1e0000", "1E5", "1e", "1e+", "1e-", "1ee", ".e5", ".5e", "0e999999",
        "0.0e999999", "0e-999999",
    ] {
        texts.push(s.as_bytes().to_vec());
    }
    let mut s: u64 = 0xabcd_1234;
    for _ in 0..4000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mant = (s >> 12) % 10_000_000_000_000_000;
        let e = ((s >> 3) % 780) as i64 - 390;
        let neg = if s & 1 == 0 { "-" } else { "" };
        texts.push(format!("{neg}{mant}.{mant}e{e}").into_bytes());
        texts.push(format!("{neg}{mant}e{e}").into_bytes());
        texts.push(format!("{neg}0.{mant:016}e{e}").into_bytes());
        texts.push(format!("{neg}{:x}", s).into_bytes());
        texts.push(format!("{neg}0x{:x}p{}", s >> 4, e).into_bytes());
    }
    for v in probe_doubles() {
        if v.is_finite() {
            texts.push(format!("{v:e}").into_bytes());
            texts.push(format!("{v:?}").into_bytes());
            texts.push(format!("{v:.17e}").into_bytes());
            texts.push(format!("{v:.25e}").into_bytes());
        }
    }

    texts
}

/// Exact-midpoint literals: these have far more than `STRTOD_DIGLIM == 40`
/// significant digits and are exactly halfway between two doubles, which is the
/// only way to reach dtoa.c's `bigcomp()`.
fn bigcomp_texts() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut seeds: Vec<f64> = vec![
        1.0, 2.0, 0.5, 0.1, 1.0 / 3.0, 3.141592653589793, 1e-5, 1e5, 1e100,
        1e-100, 1e300, 1e-300, f64::MAX, f64::MIN_POSITIVE, 5e-324, 1e-323,
        2.2250738585072014e-308, 2.2250738585072011e-308, 9007199254740992.0,
        1e22, 1e23, 1e16, 1e17, 123456789.0, f64::EPSILON,
    ];
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    for _ in 0..400 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let f = f64::from_bits(s);
        if f.is_finite() && f != 0.0 {
            seeds.push(f);
        }
    }
    // subnormals and the normal/subnormal boundary
    for k in 1..40u64 {
        seeds.push(f64::from_bits(k));
        seeds.push(f64::from_bits((1u64 << 52) - k));
        seeds.push(f64::from_bits((1u64 << 52) + k));
    }
    for d in seeds {
        out.extend(midpoint_literals(d));
    }
    out
}

/// Exact decimal expansion of `m * 2^e` (m > 0), as (digits, exponent) where the
/// value is `0.d0 d1 d2 ... * 10^exponent`.
///
/// Used to build the *exact* midpoint between two adjacent doubles. Those
/// midpoints have far more than `STRTOD_DIGLIM == 40` significant digits and are
/// maximally ambiguous, which is exactly what drives dtoa.c's `bigcomp()` path.
fn exact_decimal(m: u64, e: i32) -> (Vec<u8>, i32) {
    // digits, little-endian base 1e9 limbs
    let mut limbs: Vec<u32> = Vec::new();
    let mut v = m;
    while v > 0 {
        limbs.push((v % 1_000_000_000) as u32);
        v /= 1_000_000_000;
    }
    if limbs.is_empty() {
        limbs.push(0);
    }
    let mul_small = |limbs: &mut Vec<u32>, k: u32| {
        let mut carry: u64 = 0;
        for l in limbs.iter_mut() {
            let t = *l as u64 * k as u64 + carry;
            *l = (t % 1_000_000_000) as u32;
            carry = t / 1_000_000_000;
        }
        while carry > 0 {
            limbs.push((carry % 1_000_000_000) as u32);
            carry /= 1_000_000_000;
        }
    };

    let mut point = 0i32; // number of digits after the decimal point
    if e >= 0 {
        for _ in 0..e {
            mul_small(&mut limbs, 2);
        }
    } else {
        // m * 2^-|e| = m * 5^|e| / 10^|e|
        let n = (-e) as u32;
        let mut left = n;
        while left >= 12 {
            mul_small(&mut limbs, 244_140_625); // 5^12
            left -= 12;
        }
        for _ in 0..left {
            mul_small(&mut limbs, 5);
        }
        point = n as i32;
    }

    // render limbs to a big-endian digit string
    let mut s = String::new();
    for (i, l) in limbs.iter().enumerate().rev() {
        if i == limbs.len() - 1 {
            s.push_str(&l.to_string());
        } else {
            s.push_str(&format!("{:09}", l));
        }
    }
    let digits: Vec<u8> = s.into_bytes();
    // value == digits (as integer) * 10^-point == 0.digits * 10^(len - point)
    let exp10 = digits.len() as i32 - point;
    // strip leading zeros (there are none, the top limb was printed bare)
    (digits, exp10)
}

/// The exact midpoint between `d` and the next double away from zero, rendered
/// as a decimal literal, together with variants just above and just below it.
fn midpoint_literals(d: f64) -> Vec<Vec<u8>> {
    let bits = d.to_bits();
    if !d.is_finite() || d == 0.0 {
        return Vec::new();
    }
    let raw_exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    // significand and exponent of d as m * 2^e
    let (m, e) = if raw_exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), raw_exp - 1075)
    };
    if raw_exp >= 0x7fe {
        return Vec::new();
    }
    // midpoint between m*2^e and (m+1)*2^e is (2m+1)*2^(e-1)
    let (digits, exp10) = exact_decimal(2 * m + 1, e - 1);

    let mut out = Vec::new();
    let mk = |ds: &[u8], exp10: i32| -> Vec<u8> {
        let mut s = Vec::new();
        s.push(b'0');
        s.push(b'.');
        s.extend_from_slice(ds);
        s.extend_from_slice(format!("e{exp10}").as_bytes());
        s
    };
    // exact tie
    out.push(mk(&digits, exp10));
    // tie with a trailing 1 appended (just above)
    let mut up = digits.clone();
    up.push(b'1');
    out.push(mk(&up, exp10));
    // tie with lots of trailing zeros then a 1 (still just above, but the
    // deciding digit is far beyond STRTOD_DIGLIM)
    let mut up2 = digits.clone();
    up2.extend(std::iter::repeat(b'0').take(60));
    up2.push(b'1');
    out.push(mk(&up2, exp10));
    // tie with the last digit decremented (just below)
    let mut down = digits.clone();
    let last = down.len() - 1;
    if down[last] > b'0' {
        down[last] -= 1;
        down.push(b'9');
        down.push(b'9');
        out.push(mk(&down, exp10));
    }
    // tie truncated to exactly 40 significant digits, then zero padded, then 1
    if digits.len() > 40 {
        let mut t: Vec<u8> = digits[..40].to_vec();
        t.extend(std::iter::repeat(b'0').take(digits.len() - 40));
        out.push(mk(&t, exp10));
        let mut t2: Vec<u8> = digits[..40].to_vec();
        t2.extend(std::iter::repeat(b'0').take(digits.len() - 41));
        t2.push(b'1');
        out.push(mk(&t2, exp10));
    }
    // negated
    let n = out.len();
    for i in 0..n {
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&out[i]);
        out.push(neg);
    }
    out
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}

#[test]
fn strtod__unused_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnStrtod> = c.sym("strtod__unused");
    let fr: Symbol<FnStrtod> = r.sym("strtod__unused");
    let mut texts = build_texts();
    let bc = bigcomp_texts();
    eprintln!(
        "strtod__unused: {} general inputs + {} exact-midpoint inputs",
        texts.len(),
        bc.len()
    );
    assert!(bc.len() > 2000, "bigcomp corpus too small: {}", bc.len());
    texts.extend(bc);

    for t in &texts {
        let s = match std::ffi::CString::new(t.clone()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        unsafe {
            let mut ec: *mut c_char = std::ptr::null_mut();
            let mut er: *mut c_char = std::ptr::null_mut();
            *__errno_location() = 0;
            let vc = fc(s.as_ptr(), &mut ec);
            let errc = *__errno_location();
            *__errno_location() = 0;
            let vr = fr(s.as_ptr(), &mut er);
            let errr = *__errno_location();
            let oc = if ec.is_null() {
                -1
            } else {
                ec.offset_from(s.as_ptr()) as i64
            };
            let or = if er.is_null() {
                -1
            } else {
                er.offset_from(s.as_ptr()) as i64
            };
            assert_eq!(
                (vc.to_bits(), oc, errc),
                (vr.to_bits(), or, errr),
                "strtod__unused({:?})\n  C    {:e} [{:#018x}] end +{} errno {}\n  Rust {:e} [{:#018x}] end +{} errno {}",
                String::from_utf8_lossy(t),
                vc,
                vc.to_bits(),
                oc,
                errc,
                vr,
                vr.to_bits(),
                or,
                errr
            );
            // NULL `se`
            *__errno_location() = 0;
            let vc2 = fc(s.as_ptr(), std::ptr::null_mut());
            *__errno_location() = 0;
            let vr2 = fr(s.as_ptr(), std::ptr::null_mut());
            assert_eq!(
                vc2.to_bits(),
                vr2.to_bits(),
                "strtod__unused({:?}, NULL)",
                String::from_utf8_lossy(t)
            );
        }
    }
}

#[test]
fn corpus_is_large() {
    // Guard against the corpus silently becoming empty.
    let n = build_texts().len();
    println!("strtod__unused corpus: {n} inputs");
    assert!(n > 20_000, "corpus unexpectedly small: {n}");
}
