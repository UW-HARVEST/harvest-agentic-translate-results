//! Phase B/C — differential tests for the leaf-level pure functions:
//! `utf.c`, the `jslex.c` character helpers, `jsdtoa.c`, and the `jsvalue.c`
//! numeric coercions. Every call goes through the `.so` exports.
//!
//! CONFIGS.md rows 1-17, 62. ERRORS.md sections 2-3.

mod common;

use common::*;
use std::os::raw::{c_char, c_double, c_int, c_short, c_uint, c_ushort};

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// utf.c
// ===========================================================================

type FnChartorune = extern "C" fn(*mut Rune, *const c_char) -> c_int;
type FnRunetochar = extern "C" fn(*mut c_char, *const Rune) -> c_int;
type FnRunelen = extern "C" fn(c_int) -> c_int;
type FnRunePred = extern "C" fn(Rune) -> c_int;
type FnRuneMap = extern "C" fn(Rune) -> Rune;
type FnRuneFull = extern "C" fn(Rune) -> *const Rune;

fn decode(f: FnChartorune, bytes: &[u8]) -> (c_int, Rune) {
    // Caller must supply a NUL-terminated buffer.
    let mut r: Rune = -12345;
    let n = f(&mut r as *mut Rune, bytes.as_ptr() as *const c_char);
    (n, r)
}

/// CONFIGS row 2 + ERRORS section 2: all 256 lead bytes.
#[test]
fn utf_chartorune_all_lead_bytes() {
    let p: Pair<FnChartorune> = both_fn("jsU_chartorune");
    for b in 0u16..256 {
        // lone lead byte, then lead byte followed by every possible second byte
        let buf = [b as u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        assert_eq!(
            decode(p.c, &buf),
            decode(p.rust, &buf),
            "lone lead byte {b:#04x}"
        );
        for b2 in [0x00u8, 0x41, 0x7F, 0x80, 0xBF, 0xC0, 0xFF] {
            for b3 in [0x00u8, 0x80, 0xBF, 0xFF] {
                for b4 in [0x00u8, 0x80, 0xBF, 0xFF] {
                    let buf = [b as u8, b2, b3, b4, 0u8, 0u8];
                    assert_eq!(
                        decode(p.c, &buf),
                        decode(p.rust, &buf),
                        "sequence {:02x} {:02x} {:02x} {:02x}",
                        b,
                        b2,
                        b3,
                        b4
                    );
                }
            }
        }
    }
}

/// CONFIGS row 1: encode/decode round trip over every rune.
#[test]
fn utf_runetochar_runelen_every_rune() {
    let enc: Pair<FnRunetochar> = both_fn("jsU_runetochar");
    let len: Pair<FnRunelen> = both_fn("jsU_runelen");
    let dec: Pair<FnChartorune> = both_fn("jsU_chartorune");

    let check = |r: Rune| {
        assert_eq!((len.c)(r), (len.rust)(r), "jsU_runelen({r:#x})");

        let mut cb = [0u8; 8];
        let mut rb = [0u8; 8];
        let nc = (enc.c)(cb.as_mut_ptr() as *mut c_char, &r as *const Rune);
        let nr = (enc.rust)(rb.as_mut_ptr() as *mut c_char, &r as *const Rune);
        assert_eq!(nc, nr, "jsU_runetochar len for {r:#x}");
        assert_eq!(
            &cb[..nc.max(0) as usize],
            &rb[..nr.max(0) as usize],
            "jsU_runetochar bytes for {r:#x}"
        );

        // round trip through chartorune
        assert_eq!(decode(dec.c, &cb), decode(dec.rust, &rb), "round trip {r:#x}");
    };

    // Exhaustive over the whole Unicode range plus out-of-range values.
    for r in 0..=0x10FFFF {
        check(r);
    }
    for r in [
        -1,
        -2,
        -0x7FFF_FFFF,
        i32::MIN,
        0x110000,
        0x1FFFFF,
        0x7FFFFFFF,
        0xFFFD,
        0xD800,
        0xDFFF,
    ] {
        check(r);
    }
}

/// CONFIGS rows 4-6: case predicates and mappings for every rune.
#[test]
fn utf_case_tables_every_rune() {
    let isal: Pair<FnRunePred> = both_fn("jsU_isalpharune");
    let islo: Pair<FnRunePred> = both_fn("jsU_islowerrune");
    let isup: Pair<FnRunePred> = both_fn("jsU_isupperrune");
    let tolo: Pair<FnRuneMap> = both_fn("jsU_tolowerrune");
    let toup: Pair<FnRuneMap> = both_fn("jsU_toupperrune");
    let tolof: Pair<FnRuneFull> = both_fn("jsU_tolowerrune_full");
    let toupf: Pair<FnRuneFull> = both_fn("jsU_toupperrune_full");

    unsafe fn read_full(p: *const Rune) -> Option<Vec<Rune>> {
        if p.is_null() {
            return None;
        }
        let mut v = Vec::new();
        let mut i = 0isize;
        loop {
            let x = unsafe { *p.offset(i) };
            if x == 0 {
                break;
            }
            v.push(x);
            i += 1;
            if i > 8 {
                break;
            }
        }
        Some(v)
    }

    let check = |r: Rune| {
        assert_eq!((isal.c)(r), (isal.rust)(r), "isalpharune({r:#x})");
        assert_eq!((islo.c)(r), (islo.rust)(r), "islowerrune({r:#x})");
        assert_eq!((isup.c)(r), (isup.rust)(r), "isupperrune({r:#x})");
        assert_eq!((tolo.c)(r), (tolo.rust)(r), "tolowerrune({r:#x})");
        assert_eq!((toup.c)(r), (toup.rust)(r), "toupperrune({r:#x})");
        unsafe {
            assert_eq!(
                read_full((tolof.c)(r)),
                read_full((tolof.rust)(r)),
                "tolowerrune_full({r:#x})"
            );
            assert_eq!(
                read_full((toupf.c)(r)),
                read_full((toupf.rust)(r)),
                "toupperrune_full({r:#x})"
            );
        }
    };

    for r in 0..=0x10FFFF {
        check(r);
    }
    for r in [-1, -100, i32::MIN, 0x110000, 0x7FFFFFFF] {
        check(r);
    }
}

/// CONFIGS row 7 + ERRORS: `js_utflen` / `js_utfptrtoidx`.
#[test]
fn utf_len_and_ptrtoidx() {
    let utflen: Pair<extern "C" fn(*const c_char) -> c_int> = both_fn("js_utflen");
    let p2i: Pair<extern "C" fn(*const c_char, *const c_char) -> c_int> =
        both_fn("js_utfptrtoidx");

    let mut rng = Rng::new(SEED);
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello".to_vec(),
        "\u{00e9}".as_bytes().to_vec(),
        "\u{4e2d}\u{6587}".as_bytes().to_vec(),
        "\u{1F600}".as_bytes().to_vec(),
        vec![0x80],
        vec![0xC3],
        vec![0xE2, 0x82],
        vec![0xFF, 0xFE, 0xFD],
        "a\u{e9}b\u{4e2d}c\u{1F600}d".as_bytes().to_vec(),
    ];
    for _ in 0..500 {
        let n = rng.below(24) as usize;
        cases.push(rng.bytes_nonul(n));
    }

    for case in &cases {
        let z = cstr_bytes(case);
        let s = z.as_ptr() as *const c_char;
        assert_eq!(
            (utflen.c)(s),
            (utflen.rust)(s),
            "js_utflen({:?})",
            String::from_utf8_lossy(case)
        );
        // every pointer inside the string, plus the NUL, plus past-the-end
        for off in 0..=z.len() {
            let q = unsafe { s.add(off) };
            assert_eq!(
                (p2i.c)(s, q),
                (p2i.rust)(s, q),
                "js_utfptrtoidx({:?}, +{off})",
                String::from_utf8_lossy(case)
            );
        }
        // ERRORS section 2: `p` before `s`
        let base = unsafe { s.add(z.len()) };
        assert_eq!(
            (p2i.c)(base, s),
            (p2i.rust)(base, s),
            "js_utfptrtoidx with p before s"
        );
    }
}

// ===========================================================================
// jslex.c character helpers  (CONFIGS rows 8-10)
// ===========================================================================

#[test]
fn lex_char_class_helpers() {
    let iswhite: Pair<extern "C" fn(c_int) -> c_int> = both_fn("jsY_iswhite");
    let isnl: Pair<extern "C" fn(c_int) -> c_int> = both_fn("jsY_isnewline");
    let ishex: Pair<extern "C" fn(c_int) -> c_int> = both_fn("jsY_ishex");
    let tohex: Pair<extern "C" fn(c_int) -> c_int> = both_fn("jsY_tohex");

    for c in -300..0x3000 {
        assert_eq!((iswhite.c)(c), (iswhite.rust)(c), "jsY_iswhite({c})");
        assert_eq!((isnl.c)(c), (isnl.rust)(c), "jsY_isnewline({c})");
        assert_eq!((ishex.c)(c), (ishex.rust)(c), "jsY_ishex({c})");
        assert_eq!((tohex.c)(c), (tohex.rust)(c), "jsY_tohex({c})");
    }
    for c in [
        -1,
        i32::MIN,
        i32::MAX,
        0xFEFF,
        0x2028,
        0x2029,
        0x00A0,
        0x1680,
        0x2000,
        0x3000,
        0x10FFFF,
    ] {
        assert_eq!((iswhite.c)(c), (iswhite.rust)(c), "jsY_iswhite({c})");
        assert_eq!((isnl.c)(c), (isnl.rust)(c), "jsY_isnewline({c})");
        assert_eq!((ishex.c)(c), (ishex.rust)(c), "jsY_ishex({c})");
        assert_eq!((tohex.c)(c), (tohex.rust)(c), "jsY_tohex({c})");
    }
}

/// CONFIGS row 9 + ERRORS section 2: out-of-range token ids.
#[test]
fn lex_tokenstring_all_ids() {
    let f: Pair<extern "C" fn(c_int) -> *const c_char> = both_fn("jsY_tokenstring");
    for t in -5..400 {
        let a = unsafe { read_cstr((f.c)(t)) };
        let b = unsafe { read_cstr((f.rust)(t)) };
        assert_eq!(a, b, "jsY_tokenstring({t}) C={:?} RUST={:?}", show(&a), show(&b));
    }
}

/// CONFIGS row 10 + ERRORS section 2: `jsY_findword` hit/miss, empty list.
#[test]
fn lex_findword() {
    let f: Pair<extern "C" fn(*const c_char, *const *const c_char, c_int) -> c_int> =
        both_fn("jsY_findword");

    // Words must be sorted for the binary search, as the C callers guarantee.
    let words = [
        "break", "case", "catch", "continue", "default", "delete", "do", "else",
        "finally", "for", "function", "if", "in", "instanceof", "new", "return",
        "switch", "this", "throw", "try", "typeof", "var", "void", "while", "with",
    ];
    let owned: Vec<Vec<u8>> = words.iter().map(|w| cstr(w)).collect();
    let list: Vec<*const c_char> = owned.iter().map(|w| w.as_ptr() as *const c_char).collect();

    let mut needles: Vec<String> = words.iter().map(|s| s.to_string()).collect();
    needles.extend(
        [
            "", "a", "zzz", "brea", "breakk", "BREAK", "with ", "in", "ins", "class",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    let mut rng = Rng::new(SEED ^ 0xF00D);
    for _ in 0..300 {
        let n = rng.below(8) as usize;
        needles.push(
            String::from_utf8_lossy(&(0..n).map(|_| rng.range_i32(97, 123) as u8).collect::<Vec<u8>>())
                .into_owned(),
        );
    }

    // list sizes 0, 1, 2, ..., full  (row 10 covers empty / one / many)
    for num in 0..=list.len() {
        for needle in &needles {
            let z = cstr(needle);
            let np = z.as_ptr() as *const c_char;
            let a = (f.c)(np, list.as_ptr(), num as c_int);
            let b = (f.rust)(np, list.as_ptr(), num as c_int);
            assert_eq!(a, b, "jsY_findword({needle:?}, num={num})");
        }
    }
}

// ===========================================================================
// jsdtoa.c  (CONFIGS rows 11-16)
// ===========================================================================

/// CONFIGS row 11: `js_itoa` incl. `INT_MIN`.
#[test]
fn dtoa_itoa() {
    let f: Pair<extern "C" fn(*mut c_char, c_int) -> *const c_char> = both_fn("js_itoa");
    let mut rng = Rng::new(SEED ^ 0x1111);
    let mut vals: Vec<i32> = vec![0, 1, -1, 9, 10, -10, 99, 100, i32::MAX, i32::MIN, i32::MIN + 1];
    for _ in 0..5000 {
        vals.push(rng.next_u32() as i32);
    }
    for v in vals {
        let mut cb = [0u8; 64];
        let mut rb = [0u8; 64];
        let ca = (f.c)(cb.as_mut_ptr() as *mut c_char, v);
        let ra = (f.rust)(rb.as_mut_ptr() as *mut c_char, v);
        let a = unsafe { read_cstr(ca) };
        let b = unsafe { read_cstr(ra) };
        assert_eq!(a, b, "js_itoa({v}) C={} RUST={}", show(&a), show(&b));
        // the returned pointer must sit at the same offset inside the buffer
        let off_c = ca as usize - cb.as_ptr() as usize;
        let off_r = ra as usize - rb.as_ptr() as usize;
        assert_eq!(off_c, off_r, "js_itoa({v}) return offset");
    }
}

/// CONFIGS row 12: `js_grisu2` over randomized positive doubles + binades.
#[test]
fn dtoa_grisu2() {
    let f: Pair<extern "C" fn(c_double, *mut c_char, *mut c_int) -> c_int> =
        both_fn("js_grisu2");
    let mut rng = Rng::new(SEED ^ 0x2222);

    let mut vals: Vec<f64> = vec![
        1.0,
        0.5,
        0.1,
        1e-300,
        1e300,
        f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
        9007199254740991.0,
        1.7976931348623157e308,
        123456789.0,
        1e21,
        1e-7,
    ];
    for e in -1074i32..=1023 {
        // exact powers of two, including subnormals (powi would underflow to 0)
        let bits: u64 = if e >= -1022 {
            ((e + 1023) as u64) << 52
        } else {
            1u64 << (e + 1074)
        };
        vals.push(f64::from_bits(bits));
    }
    for _ in 0..20000 {
        vals.push(rng.positive_double());
    }
    // `jsV_numbertostring` only ever reaches `js_grisu2` with a non-zero finite
    // value (zero / NaN / Inf return early at jsvalue.c:275-277), and the C
    // assert-enabled build aborts on 0.0. Stay inside that domain.
    vals.retain(|v| v.is_finite() && *v != 0.0);
    // `js_grisu2` is also reached with negative values (jsvalue.c handles the
    // sign after the call), so cover both signs.
    let negs: Vec<f64> = vals.iter().map(|v| -v).collect();
    vals.extend(negs);

    for v in vals {
        if std::env::var_os("GRISU_TRACE").is_some() {
            use std::io::Write;
            let mut e = std::io::stderr();
            let _ = writeln!(e, "TRY {:#018x} {v:e}", v.to_bits());
            let _ = e.flush();
        }
        let mut cb = [0u8; 64];
        let mut rb = [0u8; 64];
        let mut ck: c_int = -999;
        let mut rk: c_int = -999;
        let cn = (f.c)(v, cb.as_mut_ptr() as *mut c_char, &mut ck);
        let rn = (f.rust)(v, rb.as_mut_ptr() as *mut c_char, &mut rk);
        assert_eq!(cn, rn, "js_grisu2({v:e}) length");
        assert_eq!(ck, rk, "js_grisu2({v:e}) K");
        assert_eq!(
            &cb[..cn.max(0) as usize],
            &rb[..rn.max(0) as usize],
            "js_grisu2({v:e}) digits"
        );
    }
}

/// CONFIGS row 13: `js_fmtexp`.
#[test]
fn dtoa_fmtexp() {
    let f: Pair<extern "C" fn(*mut c_char, c_int)> = both_fn("js_fmtexp");
    for e in -500..=500 {
        let mut cb = [0u8; 64];
        let mut rb = [0u8; 64];
        (f.c)(cb.as_mut_ptr() as *mut c_char, e);
        (f.rust)(rb.as_mut_ptr() as *mut c_char, e);
        let a = unsafe { read_cstr(cb.as_ptr() as *const c_char) };
        let b = unsafe { read_cstr(rb.as_ptr() as *const c_char) };
        assert_eq!(a, b, "js_fmtexp({e}) C={} RUST={}", show(&a), show(&b));
    }
}

fn numeric_string_corpus(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = [
        "", " ", "0", "-0", "+0", "1", "-1", "+1", "  12  ", "\t\n\r 42",
        "0x", "0X", "0x0", "0xff", "0xFF", "0XABCDEF", "0x10000000000000000",
        "1e", "1e+", "1e-", "1e5", "1E5", "1e+5", "1e-5", "1e999", "1e-999",
        ".", ".5", "5.", "5.5", "-.5", "+.5", ".e5", "0.0", "00", "007",
        "Infinity", "-Infinity", "+Infinity", "inf", "INF", "nan", "NaN",
        "abc", "-abc", "1abc", "1.2.3", "1_000", "1,000", "--1", "++1",
        "0b101", "0o17", "018", "08", "09", "1e0000000005",
        "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "4.9406564584124654e-324", "2.2250738585072014e-308",
        "9007199254740993", "18446744073709551616",
        "-0.0", " -0 ", "1e308", "1e309", "1e-323", "1e-324",
        "12345678901234567890123456789012345678901234567890",
        "0.1000000000000000055511151231257827021181583404541015625",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();

    for _ in 0..2000 {
        let n = rng.below(20) as usize;
        let mut s = Vec::with_capacity(n);
        for _ in 0..n {
            let alphabet = b"0123456789+-.eExXaAbBcCdDfF _\t";
            s.push(alphabet[rng.below(alphabet.len() as u32) as usize]);
        }
        v.push(s);
    }
    v
}

/// CONFIGS row 14 + ERRORS section 3: `js_strtod`.
#[test]
fn dtoa_strtod() {
    let f: Pair<extern "C" fn(*const c_char, *mut *mut c_char) -> c_double> =
        both_fn("js_strtod");
    let mut rng = Rng::new(SEED ^ 0x3333);
    for case in numeric_string_corpus(&mut rng) {
        let z = cstr_bytes(&case);
        let s = z.as_ptr() as *const c_char;
        let mut ce: *mut c_char = std::ptr::null_mut();
        let mut re: *mut c_char = std::ptr::null_mut();
        let a = (f.c)(s, &mut ce);
        let b = (f.rust)(s, &mut re);
        let label = String::from_utf8_lossy(&case).into_owned();
        assert!(
            same_double(a, b),
            "js_strtod({label:?}) C={a:?} ({:#x}) RUST={b:?} ({:#x})",
            a.to_bits(),
            b.to_bits()
        );
        let coff = ce as usize - s as usize;
        let roff = re as usize - s as usize;
        assert_eq!(coff, roff, "js_strtod({label:?}) endptr offset");

        // ERRORS section 3: endptr may be NULL
        let a2 = (f.c)(s, std::ptr::null_mut());
        let b2 = (f.rust)(s, std::ptr::null_mut());
        assert!(
            same_double(a2, b2),
            "js_strtod({label:?}) with NULL endptr C={a2:?} RUST={b2:?}"
        );
    }
}

/// CONFIGS row 15 + ERRORS section 3: `js_strtol` over every radix.
#[test]
fn dtoa_strtol() {
    let f: Pair<extern "C" fn(*const c_char, *mut *mut c_char, c_int) -> c_double> =
        both_fn("js_strtol");
    let mut rng = Rng::new(SEED ^ 0x4444);
    let corpus = numeric_string_corpus(&mut rng);
    // `js_strtol` (jsvalue.c:7) accepts a digit while `table[c] < base`, and
    // `table[c]` is 80 for every non-digit *including the NUL terminator*. So
    // any `base > 80` walks off the end of the buffer — the C build faults.
    // Everything up to and including 80 is in-domain; sweep all of it plus the
    // negative / zero / one-past-36 boundaries.
    let radices: Vec<c_int> = (-2..=80).collect();
    for radix in radices {
        for case in &corpus {
            let z = cstr_bytes(case);
            let s = z.as_ptr() as *const c_char;
            let mut ce: *mut c_char = std::ptr::null_mut();
            let mut re: *mut c_char = std::ptr::null_mut();
            let a = (f.c)(s, &mut ce, radix);
            let b = (f.rust)(s, &mut re, radix);
            let label = String::from_utf8_lossy(case).into_owned();
            assert!(
                same_double(a, b),
                "js_strtol({label:?}, radix={radix}) C={a:?} RUST={b:?}"
            );
            let coff = ce as usize - s as usize;
            let roff = re as usize - s as usize;
            assert_eq!(coff, roff, "js_strtol({label:?}, radix={radix}) endptr");
        }
    }
}

/// CONFIGS row 16: `js_stringtofloat`.
#[test]
fn dtoa_stringtofloat() {
    let f: Pair<extern "C" fn(*const c_char, *mut *mut c_char) -> c_double> =
        both_fn("js_stringtofloat");
    let mut rng = Rng::new(SEED ^ 0x5555);
    for case in numeric_string_corpus(&mut rng) {
        let z = cstr_bytes(&case);
        let s = z.as_ptr() as *const c_char;
        let mut ce: *mut c_char = std::ptr::null_mut();
        let mut re: *mut c_char = std::ptr::null_mut();
        let a = (f.c)(s, &mut ce);
        let b = (f.rust)(s, &mut re);
        let label = String::from_utf8_lossy(&case).into_owned();
        assert!(
            same_double(a, b),
            "js_stringtofloat({label:?}) C={a:?} RUST={b:?}"
        );
        assert_eq!(
            ce as usize - s as usize,
            re as usize - s as usize,
            "js_stringtofloat({label:?}) endptr"
        );
    }
}

// ===========================================================================
// jsvalue.c numeric coercions  (CONFIGS rows 17, 62)
// ===========================================================================

#[test]
fn value_number_coercions() {
    let toint: Pair<extern "C" fn(c_double) -> c_int> = both_fn("jsV_numbertointeger");
    let toi32: Pair<extern "C" fn(c_double) -> c_int> = both_fn("jsV_numbertoint32");
    let tou32: Pair<extern "C" fn(c_double) -> c_uint> = both_fn("jsV_numbertouint32");
    let toi16: Pair<extern "C" fn(c_double) -> c_short> = both_fn("jsV_numbertoint16");
    let tou16: Pair<extern "C" fn(c_double) -> c_ushort> = both_fn("jsV_numbertouint16");

    let mut rng = Rng::new(SEED ^ 0x6666);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.9,
        -0.9,
        1.5,
        -1.5,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967295.0,
        4294967296.0,
        4294967297.0,
        65535.0,
        65536.0,
        32767.0,
        32768.0,
        -32768.0,
        -32769.0,
        1e300,
        -1e300,
        5e-324,
        9007199254740992.0,
        -9007199254740992.0,
        1e21,
        f64::MAX,
        f64::MIN,
    ];
    for _ in 0..30000 {
        vals.push(rng.double());
    }
    // dense sweep around the 32-bit and 16-bit boundaries
    for k in [
        -2147483650i64,
        -65538,
        -32770,
        -2,
        32766,
        65534,
        2147483645,
        4294967293,
    ] {
        for d in 0..6 {
            vals.push((k + d) as f64);
            vals.push((k + d) as f64 + 0.5);
            vals.push((k + d) as f64 - 0.5);
        }
    }

    for v in vals {
        assert_eq!(
            (toint.c)(v),
            (toint.rust)(v),
            "jsV_numbertointeger({v:?} {:#x})",
            v.to_bits()
        );
        assert_eq!((toi32.c)(v), (toi32.rust)(v), "jsV_numbertoint32({v:?})");
        assert_eq!((tou32.c)(v), (tou32.rust)(v), "jsV_numbertouint32({v:?})");
        assert_eq!((toi16.c)(v), (toi16.rust)(v), "jsV_numbertoint16({v:?})");
        assert_eq!((tou16.c)(v), (tou16.rust)(v), "jsV_numbertouint16({v:?})");
    }
}

/// CONFIGS row 62: `jsV_numbertostring` needs a `js_State` for the error path
/// only, but the formatting is pure. Drive it through both `.so`s.
#[test]
fn value_numbertostring() {
    let f: Pair<extern "C" fn(JsState, *mut c_char, c_double) -> *const c_char> =
        both_fn("jsV_numbertostring");
    let (capi, rapi) = both_apis();
    let jc = (capi.js_newstate)(None, std::ptr::null_mut(), 0);
    let jr = (rapi.js_newstate)(None, std::ptr::null_mut(), 0);
    assert!(!jc.is_null() && !jr.is_null());

    let mut rng = Rng::new(SEED ^ 0x7777);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
        -1.0,
        0.1,
        1e20,
        1e21,
        1e-6,
        1e-7,
        123456789012345678901.0,
        5e-324,
        f64::MAX,
        f64::MIN_POSITIVE,
        1.0 / 3.0,
        100.0,
        1e100,
        -1e-100,
    ];
    for _ in 0..30000 {
        vals.push(rng.double());
    }

    for v in vals {
        let mut cb = [0u8; 64];
        let mut rb = [0u8; 64];
        let ca = (f.c)(jc, cb.as_mut_ptr() as *mut c_char, v);
        let ra = (f.rust)(jr, rb.as_mut_ptr() as *mut c_char, v);
        let a = unsafe { read_cstr(ca) };
        let b = unsafe { read_cstr(ra) };
        assert_eq!(
            a,
            b,
            "jsV_numbertostring({v:?} bits={:#x}) C={} RUST={}",
            v.to_bits(),
            show(&a),
            show(&b)
        );
    }

    (capi.js_freestate)(jc);
    (rapi.js_freestate)(jr);
}

/// ERRORS section 2: `js_isarrayindex` / `js_runeat`.
#[test]
fn value_isarrayindex_and_runeat() {
    let iai: Pair<extern "C" fn(JsState, *const c_char, *mut c_int) -> c_int> =
        both_fn("js_isarrayindex");
    let rat: Pair<extern "C" fn(JsState, *const c_char, c_int) -> c_int> = both_fn("js_runeat");
    let (capi, rapi) = both_apis();
    let jc = (capi.js_newstate)(None, std::ptr::null_mut(), 0);
    let jr = (rapi.js_newstate)(None, std::ptr::null_mut(), 0);

    let mut rng = Rng::new(SEED ^ 0x8888);
    let mut cases: Vec<Vec<u8>> = [
        "", "0", "1", "00", "01", "-1", "+1", "4294967295", "4294967296",
        "4294967294", "2147483647", "2147483648", "99999999999999999999",
        "1x", "x1", " 1", "1 ", "0.0", "1e2", "length", "a", "\u{4e2d}",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    for _ in 0..2000 {
        let n = rng.below(14) as usize;
        let alphabet = b"0123456789-+ .exabc";
        cases.push(
            (0..n)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
                .collect(),
        );
    }

    for case in &cases {
        let z = cstr_bytes(case);
        let s = z.as_ptr() as *const c_char;
        let label = String::from_utf8_lossy(case).into_owned();
        let mut ci: c_int = -777;
        let mut ri: c_int = -777;
        let a = (iai.c)(jc, s, &mut ci);
        let b = (iai.rust)(jr, s, &mut ri);
        assert_eq!(a, b, "js_isarrayindex({label:?}) return");
        if a != 0 {
            assert_eq!(ci, ri, "js_isarrayindex({label:?}) *idx");
        }
    }

    // js_runeat over UTF-8 strings, including out-of-range indices.
    let strs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"abc".to_vec(),
        "a\u{e9}b\u{4e2d}c\u{1F600}".as_bytes().to_vec(),
        vec![0xFF, 0x41, 0x80],
    ];
    for s in &strs {
        let z = cstr_bytes(s);
        let p = z.as_ptr() as *const c_char;
        for i in -3..20 {
            assert_eq!(
                (rat.c)(jc, p, i),
                (rat.rust)(jr, p, i),
                "js_runeat({:?}, {i})",
                String::from_utf8_lossy(s)
            );
        }
    }

    (capi.js_freestate)(jc);
    (rapi.js_freestate)(jr);
}
