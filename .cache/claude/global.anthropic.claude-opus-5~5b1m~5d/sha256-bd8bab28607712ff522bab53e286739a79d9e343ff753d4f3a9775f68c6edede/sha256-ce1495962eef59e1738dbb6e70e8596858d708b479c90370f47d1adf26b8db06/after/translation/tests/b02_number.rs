//! Phase B/C differential tests for number formatting and parsing:
//! `js_grisu2`, `js_fmtexp`, `js_itoa`, `js_strtod`, `js_strtol`,
//! `js_stringtofloat`, `jsV_numbertostring`, `jsV_stringtonumber`,
//! `jsV_numbertointeger/int32/uint32/int16/uint16`.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Interesting double corpus
// ---------------------------------------------------------------------------

fn special_doubles() -> Vec<f64> {
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
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,             // smallest subnormal
        2.2250738585072011e-308, // largest subnormal-ish boundary
        1e-320,
        1e308,
        1.7976931348623157e308,
        i32::MIN as f64,
        i32::MAX as f64,
        i32::MIN as f64 - 1.0,
        i32::MAX as f64 + 1.0,
        u32::MAX as f64,
        u32::MAX as f64 + 1.0,
        i16::MIN as f64,
        i16::MAX as f64,
        u16::MAX as f64,
        u16::MAX as f64 + 1.0,
        (1u64 << 53) as f64,
        (1u64 << 53) as f64 + 2.0,
        -((1u64 << 53) as f64),
        9007199254740993.0,
        1e21,
        1e-21,
        1e22,
        1e-7,
        1e-6,
        123456789012345678901234567890.0,
        4.9406564584124654e-324,
        2.2250738585072014e-308,
        0.000001,
        0.0000001,
        1234567890123456789.0,
        -1234567890123456789.0,
        3.141592653589793,
        2.718281828459045,
    ];
    // powers of ten
    for e in -330i32..=310 {
        v.push(10f64.powi(e));
        v.push(-(10f64.powi(e)));
    }
    // powers of two
    for e in -1080i32..=1030 {
        let x = 2f64.powi(e);
        if x != 0.0 || e < -1070 {
            v.push(x);
        }
    }
    // small integers and near-integers
    for i in -600i64..=600 {
        v.push(i as f64);
        v.push(i as f64 + 0.5);
        v.push(i as f64 + 0.25);
    }
    v
}

fn random_doubles(seed: u64, n: usize) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        // Mix of raw-bit doubles (covers NaN/Inf/subnormals) and structured ones.
        v.push(if rng.bool() { rng.any_f64() } else { rng.finite_f64() });
    }
    v
}

// ---------------------------------------------------------------------------
// js_grisu2
// ---------------------------------------------------------------------------

/// `js_grisu2` is only ever called by `jsV_numbertostring` after it has ruled
/// out 0, NaN and Inf (jsvalue.c:275-285), so those are not reachable inputs;
/// they are covered through `jsV_numbertostring` below. Here we drive every
/// reachable finite non-zero double, including subnormals and the extremes.
#[test]
fn grisu2_matches_for_all_finite_nonzero() {
    let (fc, fr) = pair::<FnGrisu2>("js_grisu2");
    let mut b = Batch::new();
    let mut cases: Vec<f64> = special_doubles();
    cases.extend(random_doubles(0x6115_0002, 60_000));
    for v in cases {
        if v == 0.0 || !v.is_finite() {
            continue;
        }
        // Generous buffers; grisu2 emits at most ~18 digits.
        let mut bufc = [0u8; 256];
        let mut bufr = [0u8; 256];
        let mut kc: c_int = 0;
        let mut kr: c_int = 0;
        let nc = unsafe { fc(v, bufc.as_mut_ptr() as *mut c_char, &mut kc) };
        let nr = unsafe { fr(v, bufr.as_mut_ptr() as *mut c_char, &mut kr) };
        b.check(
            &format!("grisu2({})", fmt_f64(v)),
            (nc, kc, bufc.to_vec()),
            (nr, kr, bufr.to_vec()),
        );
    }
    b.finish("js_grisu2");
}

// ---------------------------------------------------------------------------
// js_fmtexp
// ---------------------------------------------------------------------------

#[test]
fn fmtexp_matches() {
    // `js_fmtexp` writes "e%+d" using a 9-char scratch, so |e| must fit in 9
    // digits. It is called only as `js_fmtexp(p, point - 1)` where point is a
    // decimal exponent (jsvalue.c:302), i.e. roughly -330..330; we cover far
    // beyond that plus the sign boundary.
    let (fc, fr) = pair::<FnFmtexp>("js_fmtexp");
    let mut b = Batch::new();
    let mut cases: Vec<c_int> = Vec::new();
    for e in -2000i32..=2000 {
        cases.push(e);
    }
    cases.extend([
        -999_999_999, -100_000_000, -99_999_999, -1_000_000, -100_000, -10_000, 0, 9, 10, 99, 100,
        999, 1000, 9999, 100_000, 1_000_000, 99_999_999, 100_000_000, 999_999_999,
    ]);
    let mut rng = Rng::new(0xFEED_0011);
    for _ in 0..20_000 {
        cases.push(rng.range_i64(-999_999_999, 999_999_999) as c_int);
    }
    for e in cases {
        let mut bufc = [0xAAu8; 64];
        let mut bufr = [0xAAu8; 64];
        unsafe { fc(bufc.as_mut_ptr() as *mut c_char, e) };
        unsafe { fr(bufr.as_mut_ptr() as *mut c_char, e) };
        b.check(&format!("fmtexp(e={e})"), bufc.to_vec(), bufr.to_vec());
    }
    b.finish("js_fmtexp");
}

// ---------------------------------------------------------------------------
// js_itoa
// ---------------------------------------------------------------------------

#[test]
fn itoa_matches_full_i32_boundaries_and_random() {
    let (fc, fr) = pair::<FnItoa>("js_itoa");
    let mut b = Batch::new();
    let mut cases: Vec<c_int> = Vec::new();
    for v in -5000i64..=5000 {
        cases.push(v as c_int);
    }
    // Every power-of-ten boundary and the i32 extremes (INT_MIN is the tricky
    // one: the C negates via `-(unsigned)v`).
    for p in 0..10u32 {
        let t = 10i64.pow(p);
        for d in -2i64..=2 {
            cases.push((t + d) as c_int);
            cases.push(-(t + d) as c_int);
        }
    }
    cases.extend([c_int::MIN, c_int::MIN + 1, c_int::MAX, c_int::MAX - 1, 0, -1, 1]);
    let mut rng = Rng::new(0xFEED_0012);
    for _ in 0..50_000 {
        cases.push(rng.next_u32() as c_int);
    }
    for v in cases {
        let mut bufc = [0xAAu8; 64];
        let mut bufr = [0xAAu8; 64];
        let pc = unsafe { fc(bufc.as_mut_ptr() as *mut c_char, v) };
        let pr = unsafe { fr(bufr.as_mut_ptr() as *mut c_char, v) };
        // must return the same buffer it was handed
        assert_eq!(pc as usize, bufc.as_ptr() as usize, "C js_itoa should return `out`");
        assert_eq!(pr as usize, bufr.as_ptr() as usize, "Rust js_itoa should return `out`");
        b.check(&format!("itoa({v})"), bufc.to_vec(), bufr.to_vec());
    }
    b.finish("js_itoa");
}

// ---------------------------------------------------------------------------
// js_strtod / js_stringtofloat / js_strtol -- string corpus
// ---------------------------------------------------------------------------

fn numeric_string_corpus() -> Vec<Vec<u8>> {
    let fixed: &[&str] = &[
        "",
        " ",
        "\t",
        "\n",
        "\r",
        "  \t\r\n  ",
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        "+1",
        "00",
        "007",
        "0.0",
        ".0",
        "0.",
        ".",
        "-.",
        "+.",
        "1.",
        ".1",
        "1.5",
        "-1.5",
        "1e1",
        "1E1",
        "1e+1",
        "1e-1",
        "1e",
        "1e+",
        "1e-",
        "1ee1",
        "e1",
        "E1",
        ".e1",
        "1.e1",
        ".5e1",
        "1.5e10",
        "1.5E-10",
        "0x0",
        "0X0",
        "0x",
        "0X",
        "0x1",
        "0xff",
        "0xFF",
        "0xFf",
        "0xg",
        "0x1g",
        "0x7fffffff",
        "0x80000000",
        "0xffffffff",
        "0x100000000",
        "0xffffffffffffffff",
        "0xfffffffffffffffffffffffff",
        "-0x10",
        "+0x10",
        " 0x10 ",
        "Infinity",
        "+Infinity",
        "-Infinity",
        "infinity",
        "INFINITY",
        "Infinit",
        "Infinityx",
        " Infinity ",
        "NaN",
        "nan",
        "NAN",
        "1234567890",
        "12345678901234567890",
        "123456789012345678901234567890123456789012345678901234567890",
        "1e308",
        "1e309",
        "1e-308",
        "1e-323",
        "1e-324",
        "1e-400",
        "1e400",
        "-1e400",
        "1e1000000",
        "1e-1000000",
        "1e2147483647",
        "1e-2147483648",
        "1e99999999999999999999",
        "0.0000000000000000000000001",
        "1000000000000000000000000000",
        "4.9406564584124654e-324",
        "2.2250738585072014e-308",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "9007199254740992",
        "9007199254740993",
        "0.1",
        "0.2",
        "0.3",
        "0.30000000000000004",
        "5e-324",
        "  12  ",
        "12abc",
        "abc",
        "abc12",
        "--1",
        "++1",
        "+-1",
        "1-",
        "1+",
        "1 2",
        "1,2",
        "1_000",
        "0b101",
        "0o17",
        "0e0",
        "-0e0",
        "1e0",
        "0.0e0",
        "\u{a0}1",
        "\u{feff}1",
        "\u{2028}1",
        "1\u{2029}",
        "\x0b1",
        "\x0c1",
        "1\x0b",
        // long digit strings that exercise strtod's mantSize/fracExp dropping
        "1111111111111111111111111111111111111111111111111111111111111111111111",
        "0.1111111111111111111111111111111111111111111111111111111111111111111",
        "10000000000000000000000000000000000000000000000000000000000000000000e-60",
        "0.00000000000000000000000000000000000000000000000000000000000000001e60",
    ];
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.as_bytes().to_vec()).collect();

    // Property-style random numeric-ish strings.
    let mut rng = Rng::new(0xD00D_2024);
    let alphabet: &[u8] = b"0123456789.eE+- \txXaAfFnNiIty_,";
    for _ in 0..30_000 {
        let n = rng.below(14) as usize;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        v.push(s);
    }
    // Well-formed random decimals, so we hit the precise-rounding paths.
    for _ in 0..20_000 {
        let mut s = String::new();
        if rng.bool() {
            s.push(if rng.bool() { '-' } else { '+' });
        }
        let idigits = rng.below(22);
        for _ in 0..idigits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        if rng.bool() {
            s.push('.');
            for _ in 0..rng.below(22) {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        if rng.bool() {
            s.push(if rng.bool() { 'e' } else { 'E' });
            if rng.bool() {
                s.push(if rng.bool() { '-' } else { '+' });
            }
            for _ in 0..(1 + rng.below(4)) {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        v.push(s.into_bytes());
    }
    // Random hex strings for js_strtol.
    for _ in 0..8000 {
        let mut s = String::from(if rng.bool() { "0x" } else { "0X" });
        for _ in 0..(1 + rng.below(20)) {
            s.push(*rng.pick(b"0123456789abcdefABCDEFghz") as char);
        }
        v.push(s.into_bytes());
    }
    v
}

#[test]
fn strtod_matches() {
    let (fc, fr) = pair::<FnStrtod>("js_strtod");
    let mut b = Batch::new();
    for s in numeric_string_corpus() {
        let buf = cbytes(&s);
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let vc = unsafe { fc(buf.as_ptr() as *const c_char, &mut ec) };
        let vr = unsafe { fr(buf.as_ptr() as *const c_char, &mut er) };
        let base = buf.as_ptr() as usize;
        let oc = if ec.is_null() { -1i64 } else { ec as usize as i64 - base as i64 };
        let or = if er.is_null() { -1i64 } else { er as usize as i64 - base as i64 };
        b.check(
            &format!("strtod({:?})", show(&s)),
            (vc.to_bits(), oc),
            (vr.to_bits(), or),
        );
    }
    b.finish("js_strtod");
}

#[test]
fn strtod_null_endptr_is_accepted() {
    // ERRORS row: endPtr == NULL must be tolerated (no store).
    let (fc, fr) = pair::<FnStrtod>("js_strtod");
    let mut b = Batch::new();
    for s in ["1.5", "", "abc", "1e400", "-0"] {
        let buf = cbytes(s.as_bytes());
        let vc = unsafe { fc(buf.as_ptr() as *const c_char, std::ptr::null_mut()) };
        let vr = unsafe { fr(buf.as_ptr() as *const c_char, std::ptr::null_mut()) };
        b.check(&format!("strtod({s:?}, NULL)"), vc.to_bits(), vr.to_bits());
    }
    b.finish("js_strtod NULL endptr");
}

#[test]
fn stringtofloat_matches() {
    let (fc, fr) = pair::<FnStringtofloat>("js_stringtofloat");
    let mut b = Batch::new();
    for s in numeric_string_corpus() {
        let buf = cbytes(&s);
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let vc = unsafe { fc(buf.as_ptr() as *const c_char, &mut ec) };
        let vr = unsafe { fr(buf.as_ptr() as *const c_char, &mut er) };
        let base = buf.as_ptr() as usize;
        let oc = if ec.is_null() { -1i64 } else { ec as usize as i64 - base as i64 };
        let or = if er.is_null() { -1i64 } else { er as usize as i64 - base as i64 };
        b.check(
            &format!("stringtofloat({:?})", show(&s)),
            (vc.to_bits(), oc),
            (vr.to_bits(), or),
        );
    }
    b.finish("js_stringtofloat");
}

#[test]
fn strtol_matches_across_all_bases() {
    // CONFIGS axis: `base` selects between the fast base-10 loop and the
    // generic table loop (jsvalue.c:31-36). The table stores 80 for invalid
    // digits, so `base > 80` makes every byte a digit -- an out-of-range base
    // is a real input the C accepts, so it must match.
    let (fc, fr) = pair::<FnStrtol>("js_strtol");
    let mut b = Batch::new();
    // NOTE: `base > 80` is genuinely undefined in the C: the digit table stores
    // 80 for every invalid byte -- *including the NUL terminator* -- so
    // `table[c] < base` never becomes false and the loop reads off the end of
    // the buffer (jsvalue.c:34-35). That is a C-side out-of-bounds read, not a
    // comparable behaviour, so the differential sweep stops at the largest
    // well-defined base (80). See ERRORS.md.
    let bases: Vec<c_int> = vec![
        c_int::MIN, -100, -36, -1, 0, 1, 2, 3, 7, 8, 9, 10, 11, 15, 16, 17, 26, 35, 36, 37, 40,
        62, 63, 64, 79, 80,
    ];
    let corpus = numeric_string_corpus();
    for base in bases {
        for s in corpus.iter().take(600) {
            let buf = cbytes(s);
            let mut ec: *mut c_char = std::ptr::null_mut();
            let mut er: *mut c_char = std::ptr::null_mut();
            let vc = unsafe { fc(buf.as_ptr() as *const c_char, &mut ec, base) };
            let vr = unsafe { fr(buf.as_ptr() as *const c_char, &mut er, base) };
            let base_p = buf.as_ptr() as usize;
            let oc = ec as usize as i64 - base_p as i64;
            let or = er as usize as i64 - base_p as i64;
            b.check(
                &format!("strtol({:?}, base={base})", show(s)),
                (vc.to_bits(), oc),
                (vr.to_bits(), or),
            );
        }
    }
    b.finish("js_strtol across bases");
}

#[test]
fn strtol_random_digit_strings() {
    let (fc, fr) = pair::<FnStrtol>("js_strtol");
    let mut b = Batch::new();
    let mut rng = Rng::new(0xB00B_1357);
    for _ in 0..40_000 {
        let base = rng.range_i64(2, 36) as c_int;
        let n = rng.below(25) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        let buf = cbytes(&s);
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let vc = unsafe { fc(buf.as_ptr() as *const c_char, &mut ec, base) };
        let vr = unsafe { fr(buf.as_ptr() as *const c_char, &mut er, base) };
        let base_p = buf.as_ptr() as usize;
        b.check(
            &format!("strtol({:02x?}, base={base})", &buf[..buf.len() - 1]),
            (vc.to_bits(), ec as usize as i64 - base_p as i64),
            (vr.to_bits(), er as usize as i64 - base_p as i64),
        );
    }
    b.finish("js_strtol random");
}

#[test]
fn strtol_null_endptr_is_accepted() {
    // ERRORS row: `if (p) *p = ...` -- NULL endptr must be tolerated.
    let (fc, fr) = pair::<FnStrtol>("js_strtol");
    let mut b = Batch::new();
    for s in ["123", "", "ff", "zz"] {
        for base in [10 as c_int, 16, 36] {
            let buf = cbytes(s.as_bytes());
            let vc = unsafe { fc(buf.as_ptr() as *const c_char, std::ptr::null_mut(), base) };
            let vr = unsafe { fr(buf.as_ptr() as *const c_char, std::ptr::null_mut(), base) };
            b.check(&format!("strtol({s:?}, NULL, {base})"), vc.to_bits(), vr.to_bits());
        }
    }
    b.finish("js_strtol NULL endptr");
}

// ---------------------------------------------------------------------------
// jsV_numberto* conversions (pure, no js_State needed)
// ---------------------------------------------------------------------------

fn conv_corpus() -> Vec<f64> {
    let mut v = special_doubles();
    v.extend(random_doubles(0xC0DE_0001, 60_000));
    // Dense coverage of the modular-wrap boundaries.
    for k in [
        0f64,
        1.0,
        32767.0,
        32768.0,
        65535.0,
        65536.0,
        2147483647.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        4294967297.0,
        8589934592.0,
        1e15,
    ] {
        for d in -3i32..=3 {
            v.push(k + d as f64);
            v.push(-(k + d as f64));
            v.push(k + d as f64 + 0.5);
            v.push(-(k + d as f64) - 0.5);
        }
    }
    v
}

#[test]
fn numbertointeger_matches() {
    let (fc, fr) = pair::<FnNumToInt>("jsV_numbertointeger");
    let mut b = Batch::new();
    for v in conv_corpus() {
        b.check(
            &format!("jsV_numbertointeger({})", fmt_f64(v)),
            unsafe { fc(v) },
            unsafe { fr(v) },
        );
    }
    b.finish("jsV_numbertointeger");
}

#[test]
fn numbertoint32_matches() {
    let (fc, fr) = pair::<FnNumToInt>("jsV_numbertoint32");
    let mut b = Batch::new();
    for v in conv_corpus() {
        b.check(
            &format!("jsV_numbertoint32({})", fmt_f64(v)),
            unsafe { fc(v) },
            unsafe { fr(v) },
        );
    }
    b.finish("jsV_numbertoint32");
}

#[test]
fn numbertouint32_matches() {
    let (fc, fr) = pair::<FnNumToUint>("jsV_numbertouint32");
    let mut b = Batch::new();
    for v in conv_corpus() {
        b.check(
            &format!("jsV_numbertouint32({})", fmt_f64(v)),
            unsafe { fc(v) },
            unsafe { fr(v) },
        );
    }
    b.finish("jsV_numbertouint32");
}

#[test]
fn numbertoint16_matches() {
    let (fc, fr) = pair::<FnNumToShort>("jsV_numbertoint16");
    let mut b = Batch::new();
    for v in conv_corpus() {
        b.check(
            &format!("jsV_numbertoint16({})", fmt_f64(v)),
            unsafe { fc(v) },
            unsafe { fr(v) },
        );
    }
    b.finish("jsV_numbertoint16");
}

#[test]
fn numbertouint16_matches() {
    let (fc, fr) = pair::<FnNumToUshort>("jsV_numbertouint16");
    let mut b = Batch::new();
    for v in conv_corpus() {
        b.check(
            &format!("jsV_numbertouint16({})", fmt_f64(v)),
            unsafe { fc(v) },
            unsafe { fr(v) },
        );
    }
    b.finish("jsV_numbertouint16");
}

// ---------------------------------------------------------------------------
// jsV_numbertostring / jsV_stringtonumber (need a js_State)
// ---------------------------------------------------------------------------

#[test]
fn numbertostring_matches() {
    let (c, r) = Impl::both();
    let jc = c.newstate(0);
    let jr = r.newstate(0);
    let fc = c.f::<FnNumberToString>("jsV_numbertostring");
    let fr = r.f::<FnNumberToString>("jsV_numbertostring");
    let mut b = Batch::new();
    let mut cases = special_doubles();
    cases.extend(random_doubles(0xC0DE_0002, 60_000));
    for v in cases {
        // The C signature is `char buf[32]`; give both the same 32-byte buffer
        // size but a little slack so an overrun is visible rather than fatal.
        let mut bufc = [0xAAu8; 64];
        let mut bufr = [0xAAu8; 64];
        let pc = unsafe { fc(jc, bufc.as_mut_ptr() as *mut c_char, v) };
        let pr = unsafe { fr(jr, bufr.as_mut_ptr() as *mut c_char, v) };
        let sc = unsafe { read_cstr(pc) };
        let sr = unsafe { read_cstr(pr) };
        // Also verify whether the result points at the caller buffer or at a
        // static literal ("0"/"NaN"/"Infinity"/"-Infinity") -- same in both.
        let inbuf_c = pc as usize == bufc.as_ptr() as usize;
        let inbuf_r = pr as usize == bufr.as_ptr() as usize;
        b.check(
            &format!("jsV_numbertostring({})", fmt_f64(v)),
            (sc.as_ref().map(|x| show(x)), inbuf_c),
            (sr.as_ref().map(|x| show(x)), inbuf_r),
        );
    }
    b.finish("jsV_numbertostring");
    c.freestate(jc);
    r.freestate(jr);
}

#[test]
fn stringtonumber_matches() {
    let (c, r) = Impl::both();
    let jc = c.newstate(0);
    let jr = r.newstate(0);
    let fc = c.f::<FnStringToNumber>("jsV_stringtonumber");
    let fr = r.f::<FnStringToNumber>("jsV_stringtonumber");
    let mut b = Batch::new();
    for s in numeric_string_corpus() {
        let buf = cbytes(&s);
        let vc = unsafe { fc(jc, buf.as_ptr() as *const c_char) };
        let vr = unsafe { fr(jr, buf.as_ptr() as *const c_char) };
        b.check(
            &format!("jsV_stringtonumber({:?})", show(&s)),
            vc.to_bits(),
            vr.to_bits(),
        );
    }
    b.finish("jsV_stringtonumber");
    c.freestate(jc);
    r.freestate(jr);
}

/// Number formatting round-trip through the whole engine, which composes
/// numbertostring -> grisu2 -> fmtexp on the real code path.
#[test]
fn number_tostring_via_script() {
    let mut b = Batch::new();
    let mut cases = special_doubles();
    cases.extend(random_doubles(0xC0DE_0003, 3000));
    for v in cases.iter().take(6000) {
        let lit = format!("{:e}", v);
        b.script(0, &format!("String({lit})"));
    }
    for radix in [2, 3, 8, 10, 16, 36] {
        for v in [0.0, -0.0, 1.0, -1.5, 255.0, 1e21, 1e-7, f64::NAN, f64::INFINITY, 0.1] {
            let lit = format!("{:e}", v);
            b.script(0, &format!("({lit}).toString({radix})"));
        }
    }
    b.finish("number formatting via script");
}
