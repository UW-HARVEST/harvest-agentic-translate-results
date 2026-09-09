//! CONFIGS.md rows 1-13 — leaf functions: UTF-8 codec, unicode tables,
//! number formatting/parsing and numeric coercions.
//!
//! Every call goes through `libloading` into either the C `.so` or the Rust `.so`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

/* ------------------------------------------------------------------ */
/* row 1 — chartorune / runetochar / runelen round-trip, all runes      */
/* ------------------------------------------------------------------ */
#[test]
fn row01_rune_codec_full_range() {
    let p = both();
    let (cc, rc) = (p.c.jsU_runetochar, p.r.jsU_runetochar);
    let (cl, rl) = (p.c.jsU_runelen, p.r.jsU_runelen);
    let (cd, rd) = (p.c.jsU_chartorune, p.r.jsU_chartorune);

    let mut runes: Vec<i32> = Vec::new();
    // exhaustive over the interesting low range + all boundaries, sampled above
    runes.extend(0..0x1_2000);
    for b in [
        0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000, 0xFFFD, 0xFFFE,
        0xFFFF, 0x1_0000, 0x1F_FFFF, 0x10_FFFF, 0x11_0000, 0x1F_FFFF, 0x7FFF_FFFF, -1, -0x80,
        i32::MIN, i32::MIN + 1,
    ] {
        runes.push(b);
    }
    let mut rng = Rng::new(0xC0DE_0001);
    for _ in 0..40_000 {
        runes.push(rng.i32() % 0x20_0000);
    }
    for _ in 0..5_000 {
        runes.push(rng.i32());
    }

    for r in runes {
        // runelen
        let (a, b) = unsafe { (cl(r), rl(r)) };
        assert_eq!(a, b, "jsU_runelen({r:#x})");

        // runetochar: encode into a padded buffer, compare bytes AND return
        let mut bc = [0x5Au8; 16];
        let mut br = [0x5Au8; 16];
        let rr: i32 = r;
        let (na, nb) = unsafe {
            (
                cc(bc.as_mut_ptr() as *mut c_char, &rr),
                rc(br.as_mut_ptr() as *mut c_char, &rr),
            )
        };
        assert_eq!(na, nb, "jsU_runetochar({r:#x}) return");
        assert_eq!(bc, br, "jsU_runetochar({r:#x}) bytes");
        assert_eq!(na, a, "runetochar/runelen disagree for {r:#x} (C side)");

        // chartorune on the encoding we just produced
        let mut z = bc[..na as usize].to_vec();
        z.push(0);
        let mut ra: i32 = -12345;
        let mut rb: i32 = -12345;
        let (ka, kb) = unsafe {
            (
                cd(&mut ra, z.as_ptr() as *const c_char),
                cd(&mut rb, z.as_ptr() as *const c_char),
            )
        };
        assert_eq!((ka, ra), (kb, rb), "jsU_chartorune(encode({r:#x}))");
    }
}

/* ------------------------------------------------------------------ */
/* row 2 — chartorune on randomized/malformed byte sequences           */
/* ------------------------------------------------------------------ */
#[test]
fn row02_chartorune_malformed() {
    let p = both();
    let (cd, rd) = (p.c.jsU_chartorune, p.r.jsU_chartorune);
    let mut rng = Rng::new(0xC0DE_0002);

    // every 1-, 2- and 3-byte prefix exhaustively (NUL-terminated after)
    let mut cases: Vec<Vec<u8>> = Vec::new();
    cases.push(vec![]); // ""
    for b0 in 0u16..256 {
        cases.push(vec![b0 as u8]);
    }
    for b0 in 0xC0u16..0x100 {
        for b1 in 0u16..256 {
            cases.push(vec![b0 as u8, b1 as u8]);
        }
    }
    for b0 in 0xE0u16..0x100 {
        for b1 in 0u16..256 {
            for b2 in [0x00u8, 0x41, 0x7F, 0x80, 0xBF, 0xC0, 0xFF] {
                cases.push(vec![b0 as u8, b1 as u8, b2]);
            }
        }
    }
    for _ in 0..60_000 {
        let n = rng.below(6) as usize;
        cases.push((0..n).map(|_| (rng.next_u32() & 0xFF) as u8).collect());
    }

    for c in cases {
        let mut z = c.clone();
        z.push(0);
        let mut ra: i32 = -1;
        let mut rb: i32 = -1;
        let (ka, kb) = unsafe {
            (
                cd(&mut ra, z.as_ptr() as *const c_char),
                cd(&mut rb, z.as_ptr() as *const c_char),
            )
        };
        assert_eq!((ka, ra), (kb, rb), "jsU_chartorune({c:02x?})");
    }
}

/* ------------------------------------------------------------------ */
/* row 3 — unicode predicate / simple case tables, every rune          */
/* ------------------------------------------------------------------ */
#[test]
fn row03_unicode_tables() {
    let p = both();
    for r in -0x1000i32..0x11_2000 {
        unsafe {
            assert_eq!(
                (p.c.jsU_isalpharune)(r),
                (p.r.jsU_isalpharune)(r),
                "isalpharune({r:#x})"
            );
            assert_eq!(
                (p.c.jsU_islowerrune)(r),
                (p.r.jsU_islowerrune)(r),
                "islowerrune({r:#x})"
            );
            assert_eq!(
                (p.c.jsU_isupperrune)(r),
                (p.r.jsU_isupperrune)(r),
                "isupperrune({r:#x})"
            );
            assert_eq!(
                (p.c.jsU_tolowerrune)(r),
                (p.r.jsU_tolowerrune)(r),
                "tolowerrune({r:#x})"
            );
            assert_eq!(
                (p.c.jsU_toupperrune)(r),
                (p.r.jsU_toupperrune)(r),
                "toupperrune({r:#x})"
            );
        }
    }
    let mut rng = Rng::new(0xC0DE_0003);
    for _ in 0..20_000 {
        let r = rng.i32();
        unsafe {
            assert_eq!((p.c.jsU_isalpharune)(r), (p.r.jsU_isalpharune)(r), "isalpharune({r})");
            assert_eq!((p.c.jsU_tolowerrune)(r), (p.r.jsU_tolowerrune)(r), "tolowerrune({r})");
            assert_eq!((p.c.jsU_toupperrune)(r), (p.r.jsU_toupperrune)(r), "toupperrune({r})");
        }
    }
}

/* ------------------------------------------------------------------ */
/* row 4 — full (multi-rune) case folding                              */
/* ------------------------------------------------------------------ */
unsafe fn rune_slice(p: *const i32) -> Option<Vec<i32>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let r = unsafe { *p.offset(i) };
        if r == 0 {
            break;
        }
        v.push(r);
        i += 1;
        assert!(i < 64, "unterminated full-case sequence");
    }
    Some(v)
}

#[test]
fn row04_full_case_folding() {
    let p = both();
    for r in -0x100i32..0x3_0000 {
        unsafe {
            let a = rune_slice((p.c.jsU_tolowerrune_full)(r));
            let b = rune_slice((p.r.jsU_tolowerrune_full)(r));
            assert_eq!(a, b, "tolowerrune_full({r:#x})");
            let a = rune_slice((p.c.jsU_toupperrune_full)(r));
            let b = rune_slice((p.r.jsU_toupperrune_full)(r));
            assert_eq!(a, b, "toupperrune_full({r:#x})");
        }
    }
}

/* ------------------------------------------------------------------ */
/* row 5 — js_utflen / js_utfptrtoidx / js_runeat                      */
/* ------------------------------------------------------------------ */
#[test]
fn row05_utf_string_helpers() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0005);
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };
    assert!(!jc.is_null() && !jr.is_null());

    let mut samples: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        "\u{00e9}".as_bytes().to_vec(),
        "\u{20ac}".as_bytes().to_vec(),
        "\u{1f600}".as_bytes().to_vec(),
        "h\u{e9}ll\u{f6} w\u{f8}rld \u{1f600}\u{1f601}".as_bytes().to_vec(),
        vec![0xC3],
        vec![0xE2, 0x82],
        vec![0xFF, 0xFE, 0x41],
        vec![0x80, 0x80, 0x80],
    ];
    for _ in 0..4000 {
        samples.push(rng.cstr_bytes(24));
    }

    for sm in &samples {
        let mut z = sm.clone();
        z.push(0);
        let sp = z.as_ptr() as *const c_char;
        unsafe {
            assert_eq!((p.c.js_utflen)(sp), (p.r.js_utflen)(sp), "js_utflen({sm:02x?})");
            for off in 0..=sm.len() {
                let q = sp.add(off);
                assert_eq!(
                    (p.c.js_utfptrtoidx)(sp, q),
                    (p.r.js_utfptrtoidx)(sp, q),
                    "js_utfptrtoidx({sm:02x?}, +{off})"
                );
            }
            for i in -2..(sm.len() as i32 + 3) {
                assert_eq!(
                    (p.c.js_runeat)(jc, sp, i),
                    (p.r.js_runeat)(jr, sp, i),
                    "js_runeat({sm:02x?}, {i})"
                );
            }
        }
    }
    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}

/* ------------------------------------------------------------------ */
/* row 6 — js_itoa                                                     */
/* ------------------------------------------------------------------ */
#[test]
fn row06_itoa() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0006);
    let mut vals: Vec<i32> = vec![0, 1, -1, 9, 10, -10, i32::MIN, i32::MAX, i32::MIN + 1];
    for i in -3000..3000 {
        vals.push(i);
    }
    for _ in 0..200_000 {
        vals.push(rng.i32());
    }
    for v in vals {
        let mut bc = [0x5Au8; 64];
        let mut br = [0x5Au8; 64];
        unsafe {
            let ra = (p.c.js_itoa)(bc.as_mut_ptr().add(1) as *mut c_char, v);
            let rb = (p.r.js_itoa)(br.as_mut_ptr().add(1) as *mut c_char, v);
            assert_eq!(s(ra), s(rb), "js_itoa({v}) result");
            // returned pointer offset into the buffer must also match
            let oa = ra as usize - bc.as_ptr() as usize;
            let ob = rb as usize - br.as_ptr() as usize;
            assert_eq!(oa, ob, "js_itoa({v}) returned pointer offset");
            assert_eq!(bc, br, "js_itoa({v}) buffer bytes");
        }
    }
}

/* ------------------------------------------------------------------ */
/* rows 7-8 — grisu2 / fmtexp / numbertostring                         */
/* ------------------------------------------------------------------ */
#[test]
fn row07_08_number_to_string() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0007);
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };

    let mut vals: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.1, 0.5, 1.5, 100.0, 1e-7, 1e-6, 9.999999e-7, 1e20, 1e21, 1e22,
        -1e21, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, f64::MAX, f64::MIN, f64::MIN_POSITIVE,
        f64::EPSILON, 5e-324, 4294967295.0, 4294967296.0, 2147483647.0, 2147483648.0,
        -2147483648.0, 9007199254740991.0, 9007199254740992.0, 1.7976931348623157e308,
        2.2250738585072014e-308, 123456789012345678901234567890.0, 0.3, 1.0 / 3.0,
    ];
    for i in -2000..2000 {
        vals.push(i as f64);
        vals.push(i as f64 / 7.0);
    }
    for _ in 0..150_000 {
        vals.push(rng.f64());
    }

    for v in vals {
        // jsV_numbertostring (buf[32])
        let mut bc = [0x5Au8; 64];
        let mut br = [0x5Au8; 64];
        unsafe {
            let a = (p.c.jsV_numbertostring)(jc, bc.as_mut_ptr() as *mut c_char, v);
            let b = (p.r.jsV_numbertostring)(jr, br.as_mut_ptr() as *mut c_char, v);
            assert_eq!(s(a), s(b), "jsV_numbertostring({v:?} bits={:#x})", v.to_bits());
        }
        // js_grisu2 (only defined for finite non-zero values; guard like jsdtoa.c callers)
        if v.is_finite() && v != 0.0 {
            let av = v.abs();
            let mut bc = [0x5Au8; 64];
            let mut br = [0x5Au8; 64];
            let mut kc: c_int = 0;
            let mut kr: c_int = 0;
            unsafe {
                let na = (p.c.js_grisu2)(av, bc.as_mut_ptr() as *mut c_char, &mut kc);
                let nb = (p.r.js_grisu2)(av, br.as_mut_ptr() as *mut c_char, &mut kr);
                assert_eq!((na, kc), (nb, kr), "js_grisu2({av:?}) n/K");
                assert_eq!(
                    &bc[..na as usize],
                    &br[..nb as usize],
                    "js_grisu2({av:?}) digits"
                );
            }
        }
    }

    // js_fmtexp over the whole exponent range
    for e in -400i32..400 {
        let mut bc = [0x5Au8; 32];
        let mut br = [0x5Au8; 32];
        unsafe {
            (p.c.js_fmtexp)(bc.as_mut_ptr() as *mut c_char, e);
            (p.r.js_fmtexp)(br.as_mut_ptr() as *mut c_char, e);
        }
        assert_eq!(bc, br, "js_fmtexp({e})");
    }

    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}

/* ------------------------------------------------------------------ */
/* rows 9-11 — strtod / stringtofloat / strtol / stringtonumber        */
/* ------------------------------------------------------------------ */
fn numeric_text_corpus(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"\t\n\r\x0b\x0c ".to_vec(),
        b"0".to_vec(),
        b"-0".to_vec(),
        b"+0".to_vec(),
        b"1".to_vec(),
        b"-1".to_vec(),
        b".5".to_vec(),
        b"5.".to_vec(),
        b".".to_vec(),
        b"-.".to_vec(),
        b"1e".to_vec(),
        b"1e+".to_vec(),
        b"1e-".to_vec(),
        b"1e10".to_vec(),
        b"1e400".to_vec(),
        b"1e-400".to_vec(),
        b"-1e400".to_vec(),
        b"1e308".to_vec(),
        b"1e309".to_vec(),
        b"0x10".to_vec(),
        b"0X1f".to_vec(),
        b"0x".to_vec(),
        b"0b101".to_vec(),
        b"0o17".to_vec(),
        b"017".to_vec(),
        b"Infinity".to_vec(),
        b"-Infinity".to_vec(),
        b"+Infinity".to_vec(),
        b"infinity".to_vec(),
        b"Inf".to_vec(),
        b"NaN".to_vec(),
        b"nan".to_vec(),
        b"abc".to_vec(),
        b"  12  ".to_vec(),
        b"12abc".to_vec(),
        b"1.2.3".to_vec(),
        b"1__2".to_vec(),
        b"00000000000000000000001".to_vec(),
        b"0.000000000000000000001".to_vec(),
        b"1234567890123456789012345678901234567890".to_vec(),
        b"2.2250738585072011e-308".to_vec(),
        b"4.9406564584124654e-324".to_vec(),
        b"1.7976931348623158e308".to_vec(),
        b"0.5e1".to_vec(),
        b"1e2147483647".to_vec(),
        b"1e-2147483648".to_vec(),
        b"1e99999999999999999999".to_vec(),
        b"-".to_vec(),
        b"+".to_vec(),
        b"ff".to_vec(),
        b"zz".to_vec(),
        b"7fffffffffffffff".to_vec(),
        b"-8000000000000000".to_vec(),
        b"99999999999999999999999999".to_vec(),
    ];
    let alpha: &[u8] = b"0123456789.eE+- \tabcdefxXoOnfiIty_";
    while v.len() < n {
        let len = rng.below(20) as usize;
        v.push((0..len).map(|_| *rng.pick(alpha)).collect());
    }
    v
}

#[test]
fn row09_10_11_string_to_number() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0009);
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };

    let corpus = numeric_text_corpus(&mut rng, 60_000);
    for t in &corpus {
        let mut z = t.clone();
        z.push(0);
        let sp = z.as_ptr() as *const c_char;

        // js_strtod
        unsafe {
            let mut ea: *mut c_char = std::ptr::null_mut();
            let mut eb: *mut c_char = std::ptr::null_mut();
            let a = (p.c.js_strtod)(sp, &mut ea);
            let b = (p.r.js_strtod)(sp, &mut eb);
            assert!(
                same_f64(a, b),
                "js_strtod({:?}) -> {a:?} vs {b:?}",
                String::from_utf8_lossy(t)
            );
            let oa = if ea.is_null() { usize::MAX } else { ea as usize - sp as usize };
            let ob = if eb.is_null() { usize::MAX } else { eb as usize - sp as usize };
            assert_eq!(oa, ob, "js_strtod({:?}) end offset", String::from_utf8_lossy(t));
        }
        // js_stringtofloat
        unsafe {
            let mut ea: *mut c_char = std::ptr::null_mut();
            let mut eb: *mut c_char = std::ptr::null_mut();
            let a = (p.c.js_stringtofloat)(sp, &mut ea);
            let b = (p.r.js_stringtofloat)(sp, &mut eb);
            assert!(
                same_f64(a, b),
                "js_stringtofloat({:?}) -> {a:?} vs {b:?}",
                String::from_utf8_lossy(t)
            );
            let oa = if ea.is_null() { usize::MAX } else { ea as usize - sp as usize };
            let ob = if eb.is_null() { usize::MAX } else { eb as usize - sp as usize };
            assert_eq!(oa, ob, "js_stringtofloat({:?}) end", String::from_utf8_lossy(t));
        }
        // js_strtol over several radixes
        for radix in [0, 2, 8, 10, 16, 36, 1, 37, -1] {
            unsafe {
                let mut ea: *mut c_char = std::ptr::null_mut();
                let mut eb: *mut c_char = std::ptr::null_mut();
                let a = (p.c.js_strtol)(sp, &mut ea, radix);
                let b = (p.r.js_strtol)(sp, &mut eb, radix);
                assert!(
                    same_f64(a, b),
                    "js_strtol({:?},{radix}) -> {a:?} vs {b:?}",
                    String::from_utf8_lossy(t)
                );
                let oa = if ea.is_null() { usize::MAX } else { ea as usize - sp as usize };
                let ob = if eb.is_null() { usize::MAX } else { eb as usize - sp as usize };
                assert_eq!(
                    oa, ob,
                    "js_strtol({:?},{radix}) end",
                    String::from_utf8_lossy(t)
                );
            }
        }
        // jsV_stringtonumber
        unsafe {
            let a = (p.c.jsV_stringtonumber)(jc, sp);
            let b = (p.r.jsV_stringtonumber)(jr, sp);
            assert!(
                same_f64(a, b),
                "jsV_stringtonumber({:?}) -> {a:?} vs {b:?}",
                String::from_utf8_lossy(t)
            );
        }
    }
    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}

/* ------------------------------------------------------------------ */
/* row 12 — numeric coercions                                          */
/* ------------------------------------------------------------------ */
#[test]
fn row12_numeric_coercions() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0012);
    let mut vals: Vec<f64> = vec![
        0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5, -1.5, 2.5, -2.5, 0.5, -0.5,
        2147483647.0, 2147483648.0, -2147483648.0, -2147483649.0, 4294967295.0, 4294967296.0,
        4294967297.0, 65535.0, 65536.0, 32767.0, 32768.0, -32768.0, -32769.0, 1e100, -1e100,
        9007199254740993.0, f64::MAX, f64::MIN,
    ];
    for i in -70000..70000i32 {
        vals.push(i as f64);
    }
    for _ in 0..200_000 {
        vals.push(rng.f64());
    }
    for v in vals {
        unsafe {
            assert_eq!(
                (p.c.jsV_numbertointeger)(v),
                (p.r.jsV_numbertointeger)(v),
                "jsV_numbertointeger({v:?} bits={:#x})",
                v.to_bits()
            );
            assert_eq!(
                (p.c.jsV_numbertoint32)(v),
                (p.r.jsV_numbertoint32)(v),
                "jsV_numbertoint32({v:?})"
            );
            assert_eq!(
                (p.c.jsV_numbertouint32)(v),
                (p.r.jsV_numbertouint32)(v),
                "jsV_numbertouint32({v:?})"
            );
            assert_eq!(
                (p.c.jsV_numbertoint16)(v),
                (p.r.jsV_numbertoint16)(v),
                "jsV_numbertoint16({v:?})"
            );
            assert_eq!(
                (p.c.jsV_numbertouint16)(v),
                (p.r.jsV_numbertouint16)(v),
                "jsV_numbertouint16({v:?})"
            );
        }
    }
}

/* ------------------------------------------------------------------ */
/* row 13 — js_isarrayindex                                            */
/* ------------------------------------------------------------------ */
#[test]
fn row13_isarrayindex() {
    let p = both();
    let mut rng = Rng::new(0xC0DE_0013);
    let jc = unsafe { (p.c.js_newstate)(None, std::ptr::null_mut(), 0) };
    let jr = unsafe { (p.r.js_newstate)(None, std::ptr::null_mut(), 0) };

    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"0".to_vec(),
        b"00".to_vec(),
        b"01".to_vec(),
        b"1".to_vec(),
        b"-1".to_vec(),
        b"+1".to_vec(),
        b"1.0".to_vec(),
        b" 1".to_vec(),
        b"1 ".to_vec(),
        b"a".to_vec(),
        b"1a".to_vec(),
        b"4294967294".to_vec(),
        b"4294967295".to_vec(),
        b"4294967296".to_vec(),
        b"2147483647".to_vec(),
        b"2147483648".to_vec(),
        b"9999999999999999999999".to_vec(),
        b"length".to_vec(),
        b"0x10".to_vec(),
    ];
    for i in 0..5000u64 {
        cases.push(format!("{i}").into_bytes());
    }
    for _ in 0..30_000 {
        let n = rng.below(14) as usize;
        cases.push((0..n).map(|_| *rng.pick(b"0123456789abc+-. ")).collect());
    }
    for _ in 0..5_000 {
        cases.push(format!("{}", rng.next_u64()).into_bytes());
    }

    for t in &cases {
        let mut z = t.clone();
        z.push(0);
        let sp = z.as_ptr() as *const c_char;
        let mut ia: c_int = -777;
        let mut ib: c_int = -777;
        unsafe {
            let a = (p.c.js_isarrayindex)(jc, sp, &mut ia);
            let b = (p.r.js_isarrayindex)(jr, sp, &mut ib);
            assert_eq!(
                (a, ia),
                (b, ib),
                "js_isarrayindex({:?})",
                String::from_utf8_lossy(t)
            );
        }
    }
    unsafe {
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}
