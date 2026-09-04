//! Phase B — differential tests for the LOWEST-level exported entry points
//! (pure functions: utf.c, jsdtoa.c, jsvalue.c number conversions, jslex.c
//! helpers, regexp.c). Everything is called through the two `.so` exports.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

/* ------------------------------------------------------------------ utf.c */

/// CONFIGS rows: jsU_chartorune on 1/2/3/4-byte sequences, truncated
/// sequences, overlong forms, C0 80, >= T5 lead bytes, invalid continuations.
#[test]
fn utf_chartorune_random_bytes() {
    let p = libs();
    let mut rng = Rng::new(SEED);
    // exhaustive single bytes plus random multi-byte windows
    for b in 0u16..=255 {
        let buf = [b as c_char, 0, 0, 0, 0, 0, 0, 0];
        cmp_chartorune(&p.c, &p.r, &buf, &format!("single {:#02x}", b));
    }
    for _ in 0..40000 {
        let mut buf = [0 as c_char; 8];
        let n = 1 + rng.below(6) as usize;
        for i in 0..n {
            // bias towards lead/continuation bytes
            buf[i] = match rng.below(4) {
                0 => (0x80 + rng.below(0x40)) as i8 as c_char,
                1 => (0xC0 + rng.below(0x40)) as i8 as c_char,
                2 => rng.below(0x80) as i8 as c_char,
                _ => rng.below(0x100) as i8 as c_char,
            };
        }
        buf[7] = 0;
        cmp_chartorune(&p.c, &p.r, &buf, "random");
    }
    // known special cases
    for s in [
        "\u{0}", "\u{7f}", "\u{80}", "\u{7ff}", "\u{800}", "\u{ffff}", "\u{10000}", "\u{10ffff}",
        "\u{fffd}",
    ] {
        let mut buf = [0 as c_char; 8];
        for (i, b) in s.as_bytes().iter().enumerate().take(7) {
            buf[i] = *b as i8 as c_char;
        }
        cmp_chartorune(&p.c, &p.r, &buf, s);
    }
    // overlong NUL (C0 80) and truncated 4-byte
    for raw in [
        [0xC0u8, 0x80, 0, 0],
        [0xE0, 0x80, 0x80, 0],
        [0xF0, 0x90, 0x80, 0],
        [0xF4, 0x8F, 0xBF, 0xBF],
        [0xF8, 0x88, 0x80, 0x80],
        [0xFF, 0xFF, 0xFF, 0xFF],
    ] {
        let mut buf = [0 as c_char; 8];
        for i in 0..4 {
            buf[i] = raw[i] as i8 as c_char;
        }
        cmp_chartorune(&p.c, &p.r, &buf, "raw");
    }
}

fn cmp_chartorune(c: &Api, r: &Api, buf: &[c_char; 8], label: &str) {
    unsafe {
        let mut rc: c_int = -12345;
        let mut rr: c_int = -12345;
        let nc = (c.jsU_chartorune)(&mut rc, buf.as_ptr());
        let nr = (r.jsU_chartorune)(&mut rr, buf.as_ptr());
        same(
            &format!("chartorune {} {:?}", label, buf),
            &format!("{} {}", nc, rc),
            &format!("{} {}", nr, rr),
        );
    }
}

#[test]
fn utf_runetochar_and_runelen() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases: Vec<c_int> = vec![
        0, 1, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xfffd, 0xffff, 0x10000,
        0x10ffff, 0x110000, 0x7fffffff, -1, -2, i32::MIN,
    ];
    for _ in 0..20000 {
        cases.push(match rng.below(4) {
            0 => rng.below(0x80) as c_int,
            1 => rng.below(0x10000) as c_int,
            2 => rng.below(0x200000) as c_int,
            _ => rng.u32() as c_int,
        });
    }
    unsafe {
        for c in cases {
            let mut bc = [0x7Fu8 as c_char; 16];
            let mut br = [0x7Fu8 as c_char; 16];
            let nc = (p.c.jsU_runetochar)(bc.as_mut_ptr(), &c);
            let nr = (p.r.jsU_runetochar)(br.as_mut_ptr(), &c);
            same(
                &format!("runetochar {:#x}", c),
                &format!("{} {:?}", nc, bc),
                &format!("{} {:?}", nr, br),
            );
            same(
                &format!("runelen {:#x}", c),
                &format!("{}", (p.c.jsU_runelen)(c)),
                &format!("{}", (p.r.jsU_runelen)(c)),
            );
        }
    }
}

#[test]
fn utf_case_and_class_tables_exhaustive() {
    let p = libs();
    unsafe {
        // full BMP + a sample above, plus negatives / out of range
        let mut cases: Vec<c_int> = (-8..0x11002).collect();
        cases.extend([0x7fffffff, i32::MIN, -0x110000]);
        for c in cases {
            let cc = format!(
                "{} {} {} {} {}",
                (p.c.jsU_isalpharune)(c),
                (p.c.jsU_islowerrune)(c),
                (p.c.jsU_isupperrune)(c),
                (p.c.jsU_tolowerrune)(c),
                (p.c.jsU_toupperrune)(c)
            );
            let rr = format!(
                "{} {} {} {} {}",
                (p.r.jsU_isalpharune)(c),
                (p.r.jsU_islowerrune)(c),
                (p.r.jsU_isupperrune)(c),
                (p.r.jsU_tolowerrune)(c),
                (p.r.jsU_toupperrune)(c)
            );
            same(&format!("runeclass {:#x}", c), &cc, &rr);

            let fc = fullcase((p.c.jsU_tolowerrune_full)(c));
            let fr = fullcase((p.r.jsU_tolowerrune_full)(c));
            same(&format!("tolowerrune_full {:#x}", c), &fc, &fr);
            let fc = fullcase((p.c.jsU_toupperrune_full)(c));
            let fr = fullcase((p.r.jsU_toupperrune_full)(c));
            same(&format!("toupperrune_full {:#x}", c), &fc, &fr);
        }
    }
}

unsafe fn fullcase(p: *const c_int) -> String {
    if p.is_null() {
        return "<null>".into();
    }
    let mut s = String::new();
    let mut i = 0;
    // the tables are NUL terminated sequences of at most 3 runes
    while i < 8 {
        let v = *p.add(i);
        s.push_str(&format!("{:#x},", v));
        if v == 0 {
            break;
        }
        i += 1;
    }
    s
}

#[test]
fn utf_len_and_ptrtoidx() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 3);
    unsafe {
        for _ in 0..4000 {
            let s = rng.string(24);
            let cstr = cs(&s);
            let n = cstr.as_bytes().len();
            same(
                &format!("js_utflen {:?}", s),
                &format!("{}", (p.c.js_utflen)(cstr.as_ptr())),
                &format!("{}", (p.r.js_utflen)(cstr.as_ptr())),
            );
            for k in 0..=n {
                let q = cstr.as_ptr().add(k);
                same(
                    &format!("js_utfptrtoidx {:?} +{}", s, k),
                    &format!("{}", (p.c.js_utfptrtoidx)(cstr.as_ptr(), q)),
                    &format!("{}", (p.r.js_utfptrtoidx)(cstr.as_ptr(), q)),
                );
            }
        }
    }
}

/* ------------------------------------------------------------- jsdtoa.c */

#[test]
fn dtoa_strtod_random_strings() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases: Vec<String> = vec![
        "".into(),
        " ".into(),
        "0".into(),
        "-0".into(),
        "+0".into(),
        ".".into(),
        "-.".into(),
        "1.".into(),
        ".5".into(),
        "1e".into(),
        "1e+".into(),
        "1e-".into(),
        "1e999".into(),
        "1e-999".into(),
        "1e308".into(),
        "1e309".into(),
        "1e-308".into(),
        "1e-324".into(),
        "1e-400".into(),
        "0x10".into(),
        "0X10".into(),
        "Infinity".into(),
        "-Infinity".into(),
        "inf".into(),
        "nan".into(),
        "NaN".into(),
        "1.2.3".into(),
        "12345678901234567890123456789".into(),
        "0.000000000000000000001".into(),
        "1e2147483647".into(),
        "1e-2147483648".into(),
        "\t\n 42".into(),
        "4 2".into(),
        "--1".into(),
        "1_000".into(),
    ];
    for _ in 0..20000 {
        let mut s = String::new();
        if rng.below(3) == 0 {
            s.push(if rng.below(2) == 0 { '-' } else { '+' });
        }
        let digits = rng.below(25) as usize;
        for _ in 0..digits {
            s.push((b'0' + rng.below(10) as u8) as char);
        }
        if rng.below(2) == 0 {
            s.push('.');
            for _ in 0..rng.below(22) {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        if rng.below(2) == 0 {
            s.push(if rng.below(2) == 0 { 'e' } else { 'E' });
            if rng.below(2) == 0 {
                s.push(if rng.below(2) == 0 { '-' } else { '+' });
            }
            for _ in 0..1 + rng.below(4) {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        if rng.below(8) == 0 {
            s.push_str("xyz");
        }
        cases.push(s);
    }
    unsafe {
        for s in cases {
            let cstr = cs(&s);
            for radix in [0, 2, 8, 10, 16, 36] {
                let mut ec: *mut c_char = std::ptr::null_mut();
                let mut er: *mut c_char = std::ptr::null_mut();
                if radix == 0 {
                    let vc = (p.c.js_strtod)(cstr.as_ptr(), &mut ec);
                    let vr = (p.r.js_strtod)(cstr.as_ptr(), &mut er);
                    same(
                        &format!("js_strtod {:?}", s),
                        &format!("{:#x} {}", bits(vc), off(cstr.as_ptr(), ec)),
                        &format!("{:#x} {}", bits(vr), off(cstr.as_ptr(), er)),
                    );
                    let vc = (p.c.js_stringtofloat)(cstr.as_ptr(), &mut ec);
                    let vr = (p.r.js_stringtofloat)(cstr.as_ptr(), &mut er);
                    same(
                        &format!("js_stringtofloat {:?}", s),
                        &format!("{:#x} {}", bits(vc), off(cstr.as_ptr(), ec)),
                        &format!("{:#x} {}", bits(vr), off(cstr.as_ptr(), er)),
                    );
                } else {
                    let vc = (p.c.js_strtol)(cstr.as_ptr(), &mut ec, radix);
                    let vr = (p.r.js_strtol)(cstr.as_ptr(), &mut er, radix);
                    same(
                        &format!("js_strtol {:?} radix {}", s, radix),
                        &format!("{:#x} {}", bits(vc), off(cstr.as_ptr(), ec)),
                        &format!("{:#x} {}", bits(vr), off(cstr.as_ptr(), er)),
                    );
                }
            }
        }
    }
}

unsafe fn off(base: *const c_char, end: *mut c_char) -> isize {
    if end.is_null() {
        -1
    } else {
        end as isize - base as isize
    }
}

#[test]
fn dtoa_grisu2_itoa_fmtexp() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 5);
    unsafe {
        /* js_itoa: full int range sample incl. INT_MIN */
        let mut ints: Vec<c_int> = vec![0, 1, -1, 9, 10, -10, i32::MAX, i32::MIN, i32::MIN + 1];
        for _ in 0..20000 {
            ints.push(rng.u32() as c_int);
        }
        for v in ints {
            let mut bc = [0x7Fu8 as c_char; 48];
            let mut br = [0x7Fu8 as c_char; 48];
            let rc = (p.c.js_itoa)(bc.as_mut_ptr().add(24), v);
            let rr = (p.r.js_itoa)(br.as_mut_ptr().add(24), v);
            same(
                &format!("js_itoa {}", v),
                &format!("{:?} {:?}", rs(rc), bc),
                &format!("{:?} {:?}", rs(rr), br),
            );
        }
        /* js_fmtexp */
        let mut exps: Vec<c_int> = vec![0, 1, -1, 9, 10, 99, 100, 308, -308, 324, -324, 1000, -1000];
        for _ in 0..5000 {
            exps.push(rng.range_i64(-2000, 2000) as c_int);
        }
        for e in exps {
            let mut bc = [0x7Fu8 as c_char; 32];
            let mut br = [0x7Fu8 as c_char; 32];
            (p.c.js_fmtexp)(bc.as_mut_ptr(), e);
            (p.r.js_fmtexp)(br.as_mut_ptr(), e);
            same(&format!("js_fmtexp {}", e), &format!("{:?}", bc), &format!("{:?}", br));
        }
        /* js_grisu2 */
        let mut ds: Vec<f64> = vec![
            1.0,
            0.5,
            0.1,
            1e-300,
            1e300,
            f64::MIN_POSITIVE,
            5e-324,
            1.7976931348623157e308,
            123456789.0,
            1e21,
            1e-7,
        ];
        for _ in 0..20000 {
            let v = rng.f64();
            if v.is_finite() && v != 0.0 {
                ds.push(v.abs());
            }
        }
        for v in ds {
            let mut bc = [0x7Fu8 as c_char; 64];
            let mut br = [0x7Fu8 as c_char; 64];
            let mut kc: c_int = -999;
            let mut kr: c_int = -999;
            let nc = (p.c.js_grisu2)(v, bc.as_mut_ptr(), &mut kc);
            let nr = (p.r.js_grisu2)(v, br.as_mut_ptr(), &mut kr);
            same(
                &format!("js_grisu2 {:e} ({:#x})", v, bits(v)),
                &format!("{} {} {:?}", nc, kc, bc),
                &format!("{} {} {:?}", nr, kr, br),
            );
        }
    }
}

/* ------------------------------- jsvalue.c number/string conversions */

#[test]
fn number_conversions_pure() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 6);
    let mut ds: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967295.0,
        4294967296.0,
        65535.0,
        65536.0,
        32767.0,
        32768.0,
        -32769.0,
        1e21,
        1e-7,
        9007199254740993.0,
        1e308,
        -1e308,
        f64::MIN_POSITIVE,
        5e-324,
    ];
    for _ in 0..30000 {
        ds.push(rng.f64());
    }
    unsafe {
        for v in &ds {
            let v = *v;
            let cc = format!(
                "{} {} {} {} {}",
                (p.c.jsV_numbertointeger)(v),
                (p.c.jsV_numbertoint32)(v),
                (p.c.jsV_numbertouint32)(v),
                (p.c.jsV_numbertoint16)(v),
                (p.c.jsV_numbertouint16)(v)
            );
            let rr = format!(
                "{} {} {} {} {}",
                (p.r.jsV_numbertointeger)(v),
                (p.r.jsV_numbertoint32)(v),
                (p.r.jsV_numbertouint32)(v),
                (p.r.jsV_numbertoint16)(v),
                (p.r.jsV_numbertouint16)(v)
            );
            same(&format!("numberto* {:e} ({:#x})", v, bits(v)), &cc, &rr);
        }
    }
    /* jsV_numbertostring / jsV_stringtonumber need a js_State */
    unsafe {
        let jc = p.c.newstate(0);
        let jr = p.r.newstate(0);
        for v in &ds {
            let v = *v;
            let mut bc = [0x7Fu8 as c_char; 32];
            let mut br = [0x7Fu8 as c_char; 32];
            let sc = (p.c.jsV_numbertostring)(jc, bc.as_mut_ptr(), v);
            let sr = (p.r.jsV_numbertostring)(jr, br.as_mut_ptr(), v);
            same(
                &format!("jsV_numbertostring {:e} ({:#x})", v, bits(v)),
                &format!("{:?}", rs(sc)),
                &format!("{:?}", rs(sr)),
            );
        }
        let mut strs: Vec<String> = vec![
            "".into(),
            " ".into(),
            "\t\n\r\u{b}\u{c}\u{a0}".into(),
            "0".into(),
            "-0".into(),
            "0x1f".into(),
            "0X1F".into(),
            "0b101".into(),
            "0o17".into(),
            "017".into(),
            "Infinity".into(),
            "-Infinity".into(),
            "+Infinity".into(),
            "InfinityX".into(),
            "1e3".into(),
            "  12  ".into(),
            "12a".into(),
            "1.5e-3".into(),
            ".5".into(),
            "5.".into(),
            "0x".into(),
            "0xg".into(),
            "-0x10".into(),
        ];
        for _ in 0..8000 {
            strs.push(rng.string(12));
        }
        for _ in 0..8000 {
            // numeric-ish strings
            let mut s = String::new();
            for _ in 0..rng.below(8) {
                s.push([' ', '0', '1', '9', '.', 'e', 'x', '-', '+', 'a'][rng.below(10) as usize]);
            }
            strs.push(s);
        }
        for s in &strs {
            let cstr = cs(s);
            let vc = (p.c.jsV_stringtonumber)(jc, cstr.as_ptr());
            let vr = (p.r.jsV_stringtonumber)(jr, cstr.as_ptr());
            same(
                &format!("jsV_stringtonumber {:?}", s),
                &format!("{:#x}", bits(vc)),
                &format!("{:#x}", bits(vr)),
            );
        }
        (p.c.js_freestate)(jc);
        (p.r.js_freestate)(jr);
    }
}

/* -------------------------------------------------------------- jslex.c */

#[test]
fn lex_char_class_helpers() {
    let p = libs();
    unsafe {
        let mut cases: Vec<c_int> = (-300..0x3100).collect();
        cases.extend([0xFEFF, 0x1FFFE, 0x7fffffff, i32::MIN]);
        for c in cases {
            let cc = format!(
                "{} {} {} {}",
                (p.c.jsY_iswhite)(c),
                (p.c.jsY_isnewline)(c),
                (p.c.jsY_ishex)(c),
                (p.c.jsY_tohex)(c)
            );
            let rr = format!(
                "{} {} {} {}",
                (p.r.jsY_iswhite)(c),
                (p.r.jsY_isnewline)(c),
                (p.r.jsY_ishex)(c),
                (p.r.jsY_tohex)(c)
            );
            same(&format!("lexclass {:#x}", c), &cc, &rr);
        }
        /* jsY_tokenstring over the whole token space plus out-of-range */
        for t in -16..400 {
            same(
                &format!("jsY_tokenstring {}", t),
                &format!("{:?}", rs((p.c.jsY_tokenstring)(t))),
                &format!("{:?}", rs((p.r.jsY_tokenstring)(t))),
            );
        }
    }
}

#[test]
fn lex_findword() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 7);
    /* a sorted word list, as jsY_findword requires */
    let words = [
        cs("alpha"),
        cs("beta"),
        cs("delta"),
        cs("gamma"),
        cs("omega"),
        cs("zeta"),
    ];
    let ptrs: Vec<*const c_char> = words.iter().map(|w| w.as_ptr()).collect();
    unsafe {
        let mut probes: Vec<String> = words
            .iter()
            .map(|w| w.to_str().unwrap().to_string())
            .collect();
        probes.extend(["".into(), "a".into(), "zz".into(), "gammaa".into(), "Gamma".into()]);
        for _ in 0..3000 {
            probes.push(rng.string(6));
        }
        for s in probes {
            let q = cs(&s);
            for n in 0..=ptrs.len() as c_int {
                same(
                    &format!("jsY_findword {:?} n={}", s, n),
                    &format!("{}", (p.c.jsY_findword)(q.as_ptr(), ptrs.as_ptr(), n)),
                    &format!("{}", (p.r.jsY_findword)(q.as_ptr(), ptrs.as_ptr(), n)),
                );
            }
        }
    }
}

/* ------------------------------------------------------------- regexp.c */

fn random_pattern(rng: &mut Rng) -> String {
    let atoms = [
        "a", "b", ".", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S", "[a-z]", "[^a-z]", "[abc]",
        "[]", "[^]", "(a)", "(?:a)", "(?=a)", "(?!a)", "\\1", "\\b", "\\B", "^", "$", "\\n",
        "\\x41", "\\u0041", "\\cA", "\\0", "\\/", "[\\b]", "[a-", "z]", "(", ")", "|", "*", "+",
        "?", "{2}", "{2,}", "{2,4}", "{4,2}", "{,2}", "\\", "(a|b)+", "(a)(b)(c)", "\\10",
    ];
    let n = 1 + rng.below(6) as usize;
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(atoms[rng.below(atoms.len() as u64) as usize]);
    }
    s
}

#[test]
fn regexp_compile_and_exec() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 8);
    let mut pats: Vec<String> = vec![
        "a".into(),
        "".into(),
        "a*".into(),
        "a+b".into(),
        "(a)(b)".into(),
        "^a$".into(),
        "[a-z]+".into(),
        "[^a-z]+".into(),
        "a{2,3}".into(),
        "a{3,2}".into(),
        "a**".into(),
        "(".into(),
        ")".into(),
        "[a".into(),
        "a{1".into(),
        "\\".into(),
        "\\x".into(),
        "\\u12".into(),
        "(?=a)".into(),
        "(?!a)".into(),
        "(a)\\1".into(),
        "\\2".into(),
        "(){0,}".into(),
        "(a*)*".into(),
        "a|b|c".into(),
        "|".into(),
        "(|)".into(),
        "[z-a]".into(),
        "[\\d-z]".into(),
        "\\B\\b".into(),
        "x{2147483648}".into(),
        "a{99999999999}".into(),
    ];
    for _ in 0..6000 {
        pats.push(random_pattern(&mut rng));
    }
    let subjects = [
        "",
        "a",
        "b",
        "ab",
        "aab",
        "abc",
        "AAA",
        "aaaaaaaaaa",
        "hello world",
        "line1\nline2",
        "\n",
        "xyz\u{e9}\u{4e2d}",
        "0123456789",
        " a ",
    ];
    unsafe {
        for pat in &pats {
            let cp = cs(pat);
            for flags in 0..4 {
                let mut ec: *const c_char = std::ptr::null();
                let mut er: *const c_char = std::ptr::null();
                let pc = (p.c.js_regcomp)(cp.as_ptr(), flags, &mut ec);
                let pr = (p.r.js_regcomp)(cp.as_ptr(), flags, &mut er);
                same(
                    &format!("js_regcomp {:?} flags={}", pat, flags),
                    &format!("null={} err={:?}", pc.is_null(), rs(ec)),
                    &format!("null={} err={:?}", pr.is_null(), rs(er)),
                );
                if pc.is_null() || pr.is_null() {
                    if !pc.is_null() {
                        (p.c.js_regfree)(pc);
                    }
                    if !pr.is_null() {
                        (p.r.js_regfree)(pr);
                    }
                    continue;
                }
                for subj in subjects {
                    let sj = cs(subj);
                    for eflags in [0, REG_NOTBOL] {
                        let mut mc = Resub::default();
                        let mut mr = Resub::default();
                        let rc = (p.c.js_regexec)(pc, sj.as_ptr(), &mut mc, eflags);
                        let rr = (p.r.js_regexec)(pr, sj.as_ptr(), &mut mr, eflags);
                        same(
                            &format!(
                                "js_regexec {:?} flags={} subj={:?} ef={}",
                                pat, flags, subj, eflags
                            ),
                            &format!("{} {}", rc, fmt_sub(&mc, sj.as_ptr(), rc)),
                            &format!("{} {}", rr, fmt_sub(&mr, sj.as_ptr(), rr)),
                        );
                        /* also exercise the sub==NULL path */
                        let rc = (p.c.js_regexec)(pc, sj.as_ptr(), std::ptr::null_mut(), eflags);
                        let rr = (p.r.js_regexec)(pr, sj.as_ptr(), std::ptr::null_mut(), eflags);
                        same(
                            &format!("js_regexec nosub {:?} subj={:?}", pat, subj),
                            &format!("{}", rc),
                            &format!("{}", rr),
                        );
                    }
                }
                (p.c.js_regfree)(pc);
                (p.r.js_regfree)(pr);
            }
        }
    }
}

fn fmt_sub(m: &Resub, base: *const c_char, rc: c_int) -> String {
    if rc != 0 {
        return "-".into();
    }
    let mut s = format!("n={}", m.nsub);
    for i in 0..(m.nsub.max(0) as usize).min(REG_MAXSUB) {
        let sp = m.sub[i].sp;
        let ep = m.sub[i].ep;
        if sp.is_null() || ep.is_null() {
            s.push_str(" -");
        } else {
            s.push_str(&format!(
                " {}..{}",
                sp as isize - base as isize,
                ep as isize - base as isize
            ));
        }
    }
    s
}

#[test]
fn regexp_regcompx_custom_allocator() {
    /* js_regcompx/js_regfreex with an explicit allocator (the C default path). */
    unsafe extern "C" fn alloc(_ctx: *mut std::ffi::c_void, p: *mut std::ffi::c_void, n: c_int) -> *mut std::ffi::c_void {
        extern "C" {
            fn realloc(p: *mut std::ffi::c_void, n: usize) -> *mut std::ffi::c_void;
            fn free(p: *mut std::ffi::c_void);
        }
        if n == 0 {
            free(p);
            return std::ptr::null_mut();
        }
        realloc(p, n as usize)
    }
    let p = libs();
    let pats = ["a+b", "(", "[a-z]{2,3}", "", "(?:x)|y"];
    unsafe {
        for pat in pats {
            let cp = cs(pat);
            for flags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                let mut ec: *const c_char = std::ptr::null();
                let mut er: *const c_char = std::ptr::null();
                let pc = (p.c.js_regcompx)(Some(alloc), std::ptr::null_mut(), cp.as_ptr(), flags, &mut ec);
                let pr = (p.r.js_regcompx)(Some(alloc), std::ptr::null_mut(), cp.as_ptr(), flags, &mut er);
                same(
                    &format!("js_regcompx {:?} flags={}", pat, flags),
                    &format!("null={} err={:?}", pc.is_null(), rs(ec)),
                    &format!("null={} err={:?}", pr.is_null(), rs(er)),
                );
                if !pc.is_null() {
                    let sj = cs("aab");
                    let mut mc = Resub::default();
                    let mut mr = Resub::default();
                    let rc = (p.c.js_regexec)(pc, sj.as_ptr(), &mut mc, 0);
                    let rr = (p.r.js_regexec)(pr, sj.as_ptr(), &mut mr, 0);
                    same(
                        &format!("js_regcompx exec {:?}", pat),
                        &format!("{} {}", rc, fmt_sub(&mc, sj.as_ptr(), rc)),
                        &format!("{} {}", rr, fmt_sub(&mr, sj.as_ptr(), rr)),
                    );
                    (p.c.js_regfreex)(Some(alloc), std::ptr::null_mut(), pc);
                    (p.r.js_regfreex)(Some(alloc), std::ptr::null_mut(), pr);
                }
            }
        }
    }
}
