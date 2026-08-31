// Level 2: number <-> string primitives (jsdtoa.c, jsvalue.c leaf helpers).
mod common;

use common::both;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn double_corpus() -> Vec<f64> {
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
        1e-1,
        1e-5,
        1e-10,
        1e-20,
        1e-100,
        1e-300,
        1e-308,
        5e-324,
        2.2250738585072014e-308,
        1e1,
        1e5,
        1e10,
        1e20,
        1e21,
        1e22,
        1e23,
        1e100,
        1e300,
        1.7976931348623157e308,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        123456789.0,
        1234567890123456789.0,
        9007199254740991.0,
        9007199254740992.0,
        9007199254740993.0,
        4294967295.0,
        4294967296.0,
        2147483647.0,
        -2147483648.0,
        3.141592653589793,
        2.718281828459045,
        1.4142135623730951,
        6.02214076e23,
        1.602176634e-19,
        100.0,
        1000.0,
        0.000001,
        0.0000001,
        1e-6,
        1e-7,
        12345.6789,
        -98765.4321,
        1e-323,
        1.5e-323,
    ];
    // Deterministic pseudo-random doubles via bit patterns.
    let mut x: u64 = 0x243F_6A88_85A3_08D3;
    for _ in 0..4000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let d = f64::from_bits(x);
        if d.is_finite() {
            v.push(d);
        }
        // also a "nicer" magnitude
        let e = ((x >> 32) as u32 as f64) / ((x as u32 as f64) + 1.0);
        if e.is_finite() {
            v.push(e);
        }
    }
    // integers and simple decimals
    for i in -300..300 {
        v.push(i as f64);
        v.push(i as f64 / 7.0);
        v.push(i as f64 * 1e17);
    }
    v
}

#[test]
fn grisu2_matches() {
    let (c, r) = unsafe {
        both::<unsafe extern "C-unwind" fn(f64, *mut c_char, *mut c_int) -> c_int>("js_grisu2")
    };
    for &d in double_corpus().iter() {
        // js_grisu2 is only ever called by jsV_numbertostring for non-zero
        // finite values; the C code asserts/aborts on 0.
        if d == 0.0 || !d.is_finite() {
            continue;
        }
        let mut bc = [0u8; 64];
        let mut br = [0u8; 64];
        let mut kc: c_int = 0;
        let mut kr: c_int = 0;
        let nc = unsafe { c(d, bc.as_mut_ptr() as *mut c_char, &mut kc) };
        let nr = unsafe { r(d, br.as_mut_ptr() as *mut c_char, &mut kr) };
        assert_eq!((nc, kc), (nr, kr), "grisu2({:e}) n/K mismatch", d);
        assert_eq!(
            &bc[..nc.max(0) as usize],
            &br[..nr.max(0) as usize],
            "grisu2({:e}) digits mismatch",
            d
        );
    }
}

#[test]
fn fmtexp_matches() {
    let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(*mut c_char, c_int)>("js_fmtexp") };
    let mut cases: Vec<c_int> = (-400..400).collect();
    // NOTE: the C code writes decimal digits of |e| into a 9-byte buffer, so it
    // only supports up to 9 digits; anything larger is UB in the C original.
    cases.extend([1000, -1000, 99999, -99999, 999999999, -999999999]);
    for &e in cases.iter() {
        let mut bc = [0u8; 64];
        let mut br = [0u8; 64];
        unsafe { c(bc.as_mut_ptr() as *mut c_char, e) };
        unsafe { r(br.as_mut_ptr() as *mut c_char, e) };
        assert_eq!(bc, br, "fmtexp({})", e);
    }
}

#[test]
fn itoa_matches() {
    let (c, r) = unsafe {
        both::<unsafe extern "C-unwind" fn(*mut c_char, c_int) -> *const c_char>("js_itoa")
    };
    let mut cases: Vec<c_int> = (-2000..2000).collect();
    cases.extend([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 100000, -100000]);
    for i in 0..32 {
        cases.push(1i32 << i);
        cases.push(-(1i32 << i));
    }
    for &v in cases.iter() {
        let mut bc = [0u8; 64];
        let mut br = [0u8; 64];
        let pc = unsafe { c(bc.as_mut_ptr() as *mut c_char, v) };
        let pr = unsafe { r(br.as_mut_ptr() as *mut c_char, v) };
        let sc = unsafe { common::cstr_to_bytes(pc) };
        let sr = unsafe { common::cstr_to_bytes(pr) };
        assert_eq!(sc, sr, "itoa({})", v);
        // returned pointer must point inside the caller-supplied buffer
        let offc = pc as usize - bc.as_ptr() as usize;
        let offr = pr as usize - br.as_ptr() as usize;
        assert_eq!(offc, offr, "itoa({}) return offset", v);
        assert_eq!(bc, br, "itoa({}) buffer contents", v);
    }
}

fn numeric_string_corpus() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    let long9 = "9".repeat(400);
    let strs = [
        "", " ", "0", "-0", "+0", "1", "-1", "+1", "  12  ", "12abc", "abc", ".", "-.", ".5",
        "0.5", "5.", "1e3", "1E3", "1e+3", "1e-3", "1e", "1e+", "1e-", "1.5e10", "1.5e-10",
        "0x10", "0X10", "0xff", "0xFF", "-0x10", "0x", "0b101", "0o17", "017", "08", "09",
        "Infinity", "-Infinity", "+Infinity", "infinity", "INF", "inf", "NaN", "nan",
        "1.7976931348623157e308", "1.7976931348623159e308", "1e309", "-1e309", "1e-400",
        "5e-324", "2.5e-324", "0.000000000000000000001", "123456789012345678901234567890",
        "9007199254740993", "0.1", "0.2", "0.3", "3.141592653589793238462643383279",
        "2147483647", "2147483648", "-2147483648", "-2147483649", "4294967295", "4294967296",
        "\t\n\r\x0b\x0c 42", "42\t\n", "--1", "++1", "1_000", "1,000", ".e3", "1.2.3",
        "0.0e0", "00", "000.000", "1e0000000000000000000003", "1e-0000000000000000000003",
        "  \n  -3.5e2  ", "-", "+", "e5", "0e0", "0e-0",
        "1000000000000000000000", "1e21", "1e-7",
        long9.as_str(), "0.0000000000000000000000000000001",
    ];
    for s in strs {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        v.push(b);
    }
    // Long digit strings
    for n in [17usize, 18, 19, 20, 25, 40, 80, 200, 780, 800] {
        let mut s = String::from("1.");
        for i in 0..n {
            s.push((b'0' + ((i * 7 + 3) % 10) as u8) as char);
        }
        let mut b = s.into_bytes();
        b.push(0);
        v.push(b);
    }
    v
}

#[test]
fn strtod_matches() {
    let (c, r) =
        unsafe { both::<unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char) -> f64>("js_strtod") };
    for s in numeric_string_corpus() {
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let dc = unsafe { c(s.as_ptr() as *const c_char, &mut ec) };
        let dr = unsafe { r(s.as_ptr() as *const c_char, &mut er) };
        assert_eq!(
            dc.to_bits(),
            dr.to_bits(),
            "strtod({:?}) value: C={} Rust={}",
            String::from_utf8_lossy(&s[..s.len() - 1]),
            dc,
            dr
        );
        let offc = ec as usize - s.as_ptr() as usize;
        let offr = er as usize - s.as_ptr() as usize;
        assert_eq!(
            offc,
            offr,
            "strtod({:?}) end pointer",
            String::from_utf8_lossy(&s[..s.len() - 1])
        );
        // and with NULL end pointer
        let dc2 = unsafe { c(s.as_ptr() as *const c_char, std::ptr::null_mut()) };
        let dr2 = unsafe { r(s.as_ptr() as *const c_char, std::ptr::null_mut()) };
        assert_eq!(dc2.to_bits(), dr2.to_bits());
    }
}

#[test]
fn strtol_matches() {
    let (c, r) = unsafe {
        both::<unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char, c_int) -> f64>("js_strtol")
    };
    let mut corpus = numeric_string_corpus();
    for s in [
        "zz", "ZZ", "7f", "777", "11111111111111111111111111111111111111", "-ff", "  0x1f",
        "gg", "1z", "0777", "deadBEEF", "-", "+",
    ] {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        corpus.push(b);
    }
    for base in [0i32, 2, 3, 8, 10, 16, 36, 1, 37, -1] {
        for s in &corpus {
            let mut ec: *mut c_char = std::ptr::null_mut();
            let mut er: *mut c_char = std::ptr::null_mut();
            let dc = unsafe { c(s.as_ptr() as *const c_char, &mut ec, base) };
            let dr = unsafe { r(s.as_ptr() as *const c_char, &mut er, base) };
            assert_eq!(
                dc.to_bits(),
                dr.to_bits(),
                "strtol({:?}, base={}) value: C={} Rust={}",
                String::from_utf8_lossy(&s[..s.len() - 1]),
                base,
                dc,
                dr
            );
            let offc = ec as usize - s.as_ptr() as usize;
            let offr = er as usize - s.as_ptr() as usize;
            assert_eq!(
                offc,
                offr,
                "strtol({:?}, base={}) end pointer",
                String::from_utf8_lossy(&s[..s.len() - 1]),
                base
            );
        }
    }
}

#[test]
fn stringtofloat_matches() {
    let (c, r) = unsafe {
        both::<unsafe extern "C-unwind" fn(*const c_char, *mut *mut c_char) -> f64>("js_stringtofloat")
    };
    for s in numeric_string_corpus() {
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let dc = unsafe { c(s.as_ptr() as *const c_char, &mut ec) };
        let dr = unsafe { r(s.as_ptr() as *const c_char, &mut er) };
        assert_eq!(
            dc.to_bits(),
            dr.to_bits(),
            "stringtofloat({:?}): C={} Rust={}",
            String::from_utf8_lossy(&s[..s.len() - 1]),
            dc,
            dr
        );
        assert_eq!(
            ec as usize - s.as_ptr() as usize,
            er as usize - s.as_ptr() as usize,
            "stringtofloat({:?}) end pointer",
            String::from_utf8_lossy(&s[..s.len() - 1])
        );
    }
}

#[test]
fn number_int_conversions_match() {
    // jsV_numbertoint32 / uint32 / int16 / uint16 / integer are pure.
    let corpus = {
        let mut v = double_corpus();
        v.extend([
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -f64::NAN,
            0.9999999999,
            -0.9999999999,
            65535.5,
            65536.5,
            -65535.5,
            2147483647.5,
            -2147483648.5,
            4294967295.5,
            1e18,
            -1e18,
        ]);
        v
    };
    let (ci32, ri32) =
        unsafe { both::<unsafe extern "C-unwind" fn(f64) -> i32>("jsV_numbertoint32") };
    let (cu32, ru32) =
        unsafe { both::<unsafe extern "C-unwind" fn(f64) -> u32>("jsV_numbertouint32") };
    let (ci16, ri16) = unsafe { both::<unsafe extern "C-unwind" fn(f64) -> i16>("jsV_numbertoint16") };
    let (cu16, ru16) = unsafe { both::<unsafe extern "C-unwind" fn(f64) -> u16>("jsV_numbertouint16") };
    let (cint, rint) = unsafe { both::<unsafe extern "C-unwind" fn(f64) -> i32>("jsV_numbertointeger") };
    for &d in corpus.iter() {
        assert_eq!(unsafe { ci32(d) }, unsafe { ri32(d) }, "numbertoint32({})", d);
        assert_eq!(unsafe { cu32(d) }, unsafe { ru32(d) }, "numbertouint32({})", d);
        assert_eq!(unsafe { ci16(d) }, unsafe { ri16(d) }, "numbertoint16({})", d);
        assert_eq!(unsafe { cu16(d) }, unsafe { ru16(d) }, "numbertouint16({})", d);
        assert_eq!(
            unsafe { cint(d) },
            unsafe { rint(d) },
            "numbertointeger({})",
            d
        );
    }
}

#[test]
fn lexer_char_helpers_match() {
    for name in ["jsY_iswhite", "jsY_isnewline", "jsY_ishex", "jsY_tohex"] {
        let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(c_int) -> c_int>(name) };
        for x in -300..0x2200i32 {
            assert_eq!(unsafe { c(x) }, unsafe { r(x) }, "{}({:#x})", name, x);
        }
        for x in [0xFEFF, 0x2028, 0x2029, 0x00A0, 0x200B, 0x3000, 0xFFFF, 0x10000] {
            assert_eq!(unsafe { c(x) }, unsafe { r(x) }, "{}({:#x})", name, x);
        }
    }
}

#[test]
fn findword_and_tokenstring_match() {
    // jsY_findword(const char *s, const char **list, int num)
    let (ctok, rtok) =
        unsafe { both::<unsafe extern "C-unwind" fn(c_int) -> *const c_char>("jsY_tokenstring") };
    for t in -5..400 {
        let a = unsafe { common::cstr_to_bytes(ctok(t)) };
        let b = unsafe { common::cstr_to_bytes(rtok(t)) };
        assert_eq!(a, b, "jsY_tokenstring({})", t);
    }

    let (cfw, rfw) = unsafe {
        both::<unsafe extern "C-unwind" fn(*const c_char, *const *const c_char, c_int) -> c_int>(
            "jsY_findword",
        )
    };
    let words = ["alpha", "beta", "delta", "gamma", "omega"];
    let cwords: Vec<CString> = words.iter().map(|w| CString::new(*w).unwrap()).collect();
    let ptrs: Vec<*const c_char> = cwords.iter().map(|s| s.as_ptr()).collect();
    for probe in [
        "alpha", "beta", "delta", "gamma", "omega", "a", "z", "", "alphaa", "epsilon", "zeta",
        "Alpha", "omegb",
    ] {
        let p = CString::new(probe).unwrap();
        let a = unsafe { cfw(p.as_ptr(), ptrs.as_ptr(), ptrs.len() as c_int) };
        let b = unsafe { rfw(p.as_ptr(), ptrs.as_ptr(), ptrs.len() as c_int) };
        assert_eq!(a, b, "jsY_findword({:?})", probe);
    }
}
