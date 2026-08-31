//! Phase B/C differential tests for the lexer helper predicates and the
//! low-level string/index helpers:
//! `jsY_iswhite`, `jsY_isnewline`, `jsY_ishex`, `jsY_tohex`, `jsY_tokenstring`,
//! `jsY_findword`, `js_isarrayindex`, `js_utflen`, `js_utfptrtoidx`, `js_runeat`,
//! `js_intern`.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// jsY_* character predicates -- exhaustive over the whole int range of interest
// ---------------------------------------------------------------------------

fn char_probe_ints() -> Vec<c_int> {
    let mut v: Vec<c_int> = Vec::new();
    // Exhaustive over all of Unicode plus a margin on both sides. These take an
    // `int c` so negatives and out-of-range values are real inputs.
    for c in -300i64..=0x110100 {
        v.push(c as c_int);
    }
    v.extend([
        c_int::MIN,
        c_int::MIN + 1,
        -0x7FFFFFFF,
        -65536,
        -256,
        -1,
        0x7FFFFFFE,
        c_int::MAX,
    ]);
    let mut rng = Rng::new(0x1EAF_0001);
    for _ in 0..30_000 {
        v.push(rng.next_u32() as c_int);
    }
    v
}

#[test]
fn jsy_char_predicates_match() {
    // `jsY_ishex` calls the libc `isdigit`, which is UB outside
    // [-1, UCHAR_MAX]; the well-defined domain is what the lexer actually feeds
    // it. We still sweep the full int range because the C accepts it, and both
    // impls must agree byte for byte.
    let probes = char_probe_ints();
    for name in ["jsY_iswhite", "jsY_isnewline", "jsY_ishex", "jsY_tohex"] {
        let (fc, fr) = pair::<FnCharPred>(name);
        let mut b = Batch::new();
        for &c in &probes {
            b.check(&format!("{name}({c})"), unsafe { fc(c) }, unsafe { fr(c) });
        }
        b.finish(name);
    }
}

#[test]
fn jsy_tokenstring_matches_over_full_int_range() {
    // ERRORS row: out-of-range token numbers must return the literal
    // "<unknown>" (jslex.c: jsY_tokenstring bounds check). Token values are a C
    // enum, so any int is a legal FFI input.
    let (fc, fr) = pair::<FnTokenString>("jsY_tokenstring");
    let mut b = Batch::new();
    let mut cases: Vec<c_int> = (-2000i32..=2000).collect();
    cases.extend([c_int::MIN, c_int::MIN + 1, -1, 0, c_int::MAX - 1, c_int::MAX]);
    let mut rng = Rng::new(0x1EAF_0002);
    for _ in 0..20_000 {
        cases.push(rng.next_u32() as c_int);
    }
    for c in cases {
        let a = unsafe { read_cstr(fc(c)) }.map(|x| show(&x));
        let bb = unsafe { read_cstr(fr(c)) }.map(|x| show(&x));
        b.check(&format!("jsY_tokenstring({c})"), a, bb);
    }
    b.finish("jsY_tokenstring");
}

#[test]
fn jsy_findword_matches() {
    // `jsY_findword` is a binary search over a caller-supplied sorted list.
    // Axes: list length 0 / 1 / many; needle present / absent / before / after;
    // `num` larger or smaller than the real list length.
    let (fc, fr) = pair::<FnFindword>("jsY_findword");
    let mut b = Batch::new();

    let words: Vec<&str> = vec![
        "break", "case", "catch", "continue", "debugger", "default", "delete", "do", "else",
        "finally", "for", "function", "if", "in", "instanceof", "new", "return", "switch", "this",
        "throw", "try", "typeof", "var", "void", "while", "with",
    ];
    let cwords: Vec<std::ffi::CString> = words.iter().map(|w| cstr(w)).collect();
    let ptrs: Vec<*const c_char> = cwords.iter().map(|c| c.as_ptr()).collect();

    let mut needles: Vec<String> =
        words.iter().map(|s| s.to_string()).collect();
    needles.extend(
        [
            "", "a", "zzz", "brea", "breaks", "AAA", "if", "iff", "i", "wit", "withx", "\u{7f}",
            "0", "~",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    // random needles
    let mut rng = Rng::new(0x1EAF_0003);
    for _ in 0..5000 {
        let n = rng.below(8) as usize;
        let s: String = (0..n).map(|_| (b'a' + rng.below(26) as u8) as char).collect();
        needles.push(s);
    }

    // num sweeps 0..=len, plus one past (a caller bug the C tolerates as long as
    // the list is long enough -- we cap at len so we never read out of bounds).
    for num in 0..=ptrs.len() {
        for needle in &needles {
            let n = cstr(needle);
            let a = unsafe { fc(n.as_ptr(), ptrs.as_ptr(), num as c_int) };
            let bb = unsafe { fr(n.as_ptr(), ptrs.as_ptr(), num as c_int) };
            b.check(&format!("jsY_findword({needle:?}, num={num})"), a, bb);
        }
    }
    // ERRORS row: num <= 0 -> r = num-1 < l = 0 -> immediate -1, no list access.
    // (num == INT_MIN is excluded: `num - 1` is signed-overflow UB in the C and
    // wraps to INT_MAX, which then dereferences the list out of bounds. That is
    // a C-side UB, not a comparable behaviour -- see ERRORS.md.)
    for num in [0 as c_int, -1, -100, -1000] {
        let n = cstr("break");
        let a = unsafe { fc(n.as_ptr(), ptrs.as_ptr(), num) };
        let bb = unsafe { fr(n.as_ptr(), ptrs.as_ptr(), num) };
        b.check(&format!("jsY_findword(num={num})"), a, bb);
    }
    b.finish("jsY_findword");
}

// ---------------------------------------------------------------------------
// js_isarrayindex
// ---------------------------------------------------------------------------

fn index_string_corpus() -> Vec<Vec<u8>> {
    let fixed: &[&str] = &[
        "",
        "0",
        "00",
        "01",
        "007",
        "0x1",
        "1",
        "9",
        "10",
        "99",
        "100",
        "1000",
        "123456789",
        "1234567890",
        "2147483647",
        "2147483646",
        "2147483648",
        "4294967295",
        "4294967296",
        "9999999999",
        "99999999999999999999",
        "214748364",
        "214748365",
        "2147483640",
        "-1",
        "-0",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "1e2",
        "a",
        "1a",
        "a1",
        "length",
        "NaN",
        "Infinity",
        "1_0",
        "\u{ff10}",
        "０",
    ];
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.as_bytes().to_vec()).collect();
    let mut rng = Rng::new(0x1EAF_0004);
    // random digit strings incl. lengths that straddle INT_MAX/10
    for _ in 0..30_000 {
        let n = rng.below(14) as usize;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(b"0123456789")).collect();
        v.push(s);
    }
    // random mixed strings
    for _ in 0..10_000 {
        let n = rng.below(8) as usize;
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(b"0123456789-+. eExXaz")).collect();
        v.push(s);
    }
    v
}

#[test]
fn isarrayindex_matches() {
    let (c, r) = Impl::both();
    let jc = c.newstate(0);
    let jr = r.newstate(0);
    let fc = c.f::<FnIsArrayIndex>("js_isarrayindex");
    let fr = r.f::<FnIsArrayIndex>("js_isarrayindex");
    let mut b = Batch::new();
    for s in index_string_corpus() {
        let buf = cbytes(&s);
        // Poison idx so "not written" is distinguishable.
        let mut ic: c_int = -777;
        let mut ir: c_int = -777;
        let a = unsafe { fc(jc, buf.as_ptr() as *const c_char, &mut ic) };
        let bb = unsafe { fr(jr, buf.as_ptr() as *const c_char, &mut ir) };
        b.check(&format!("js_isarrayindex({:?})", show(&s)), (a, ic), (bb, ir));
    }
    b.finish("js_isarrayindex");
    c.freestate(jc);
    r.freestate(jr);
}

// ---------------------------------------------------------------------------
// js_utflen / js_utfptrtoidx / js_runeat
// ---------------------------------------------------------------------------

fn utf_string_corpus() -> Vec<Vec<u8>> {
    let fixed: Vec<&str> = vec![
        "",
        "a",
        "ab",
        "abc",
        "hello world",
        "\u{e9}",
        "caf\u{e9}",
        "\u{4f60}\u{597d}",
        "\u{1f600}",
        "\u{1f600}\u{1f601}\u{1f602}",
        "a\u{e9}b\u{4f60}c\u{1f600}d",
        "\u{10ffff}",
        "\u{ffff}",
        "\u{10000}",
        "\u{d7ff}",
        "\u{e000}",
    ];
    let mut v: Vec<Vec<u8>> = fixed.iter().map(|s| s.as_bytes().to_vec()).collect();
    // Deliberately malformed byte sequences (chartorune's Runeerror paths).
    let bad: &[&[u8]] = &[
        b"\x80",
        b"\xbf",
        b"\xc0",
        b"\xc0\x80",
        b"\xc1\x80",
        b"\xc2",
        b"\xe0\x80\x80",
        b"\xed\xa0\x80",
        b"\xf0\x80\x80\x80",
        b"\xf4\x90\x80\x80",
        b"\xf8\x80\x80\x80",
        b"\xff\xff\xff\xff",
        b"a\x80b",
        b"a\xc0\x80b",
        b"\xf0\x9f\x98\x80\x80",
    ];
    v.extend(bad.iter().map(|b| b.to_vec()));
    let mut rng = Rng::new(0x1EAF_0005);
    for _ in 0..20_000 {
        let n = rng.below(20) as usize;
        let s: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        v.push(s);
    }
    for _ in 0..8000 {
        // random well-formed UTF-8
        let n = rng.below(8) as usize;
        let mut s = String::new();
        for _ in 0..n {
            let cp = match rng.below(4) {
                0 => rng.below(0x80),
                1 => 0x80 + rng.below(0x780),
                2 => 0x800 + rng.below(0xF800),
                _ => 0x10000 + rng.below(0x100000),
            };
            if let Some(ch) = char::from_u32(cp) {
                s.push(ch);
            }
        }
        v.push(s.into_bytes());
    }
    v
}

#[test]
fn utflen_matches() {
    let (fc, fr) = pair::<FnUtflen>("js_utflen");
    let mut b = Batch::new();
    for s in utf_string_corpus() {
        // pad so chartorune's unconditional lookahead stays in bounds
        let mut buf = cbytes(&s);
        buf.extend_from_slice(&[0u8; 8]);
        let a = unsafe { fc(buf.as_ptr() as *const c_char) };
        let bb = unsafe { fr(buf.as_ptr() as *const c_char) };
        b.check(&format!("js_utflen({:02x?})", &s), a, bb);
    }
    b.finish("js_utflen");
}

#[test]
fn utfptrtoidx_matches() {
    // Axis: `p` at every byte offset from 0 to len (including offsets that land
    // in the middle of a multi-byte sequence, and p == s meaning 0).
    let (fc, fr) = pair::<FnUtfptrtoidx>("js_utfptrtoidx");
    let mut b = Batch::new();
    for s in utf_string_corpus().into_iter().take(4000) {
        let mut buf = cbytes(&s);
        buf.extend_from_slice(&[0u8; 8]);
        let base = buf.as_ptr() as *const c_char;
        for off in 0..=s.len() {
            let p = unsafe { base.add(off) };
            let a = unsafe { fc(base, p) };
            let bb = unsafe { fr(base, p) };
            b.check(&format!("js_utfptrtoidx({:02x?}, +{off})", &s), a, bb);
        }
        // p < s -> loop body never runs -> 0
        let a = unsafe { fc(base, base) };
        let bb = unsafe { fr(base, base) };
        b.check("js_utfptrtoidx(p == s)", a, bb);
    }
    b.finish("js_utfptrtoidx");
}

#[test]
fn runeat_matches() {
    // Axes: index before/at/after the end, negative index, surrogate-pair
    // splitting for astral code points (i -= 2), and malformed input.
    let (c, r) = Impl::both();
    let jc = c.newstate(0);
    let jr = r.newstate(0);
    let fc = c.f::<FnRuneat>("js_runeat");
    let fr = r.f::<FnRuneat>("js_runeat");
    let mut b = Batch::new();
    for s in utf_string_corpus().into_iter().take(3000) {
        let mut buf = cbytes(&s);
        buf.extend_from_slice(&[0u8; 8]);
        let p = buf.as_ptr() as *const c_char;
        let hi = (s.len() as c_int) + 4;
        for i in -3..=hi {
            let a = unsafe { fc(jc, p, i) };
            let bb = unsafe { fr(jr, p, i) };
            b.check(&format!("js_runeat({:02x?}, {i})", &s), a, bb);
        }
    }
    b.finish("js_runeat");
    c.freestate(jc);
    r.freestate(jr);
}

// ---------------------------------------------------------------------------
// js_intern -- the string interning table (jsintern.c)
// ---------------------------------------------------------------------------

#[test]
fn intern_returns_stable_and_equal_strings() {
    // CONFIGS rows: interning the same string twice must return the SAME pointer
    // (within one impl), and the resulting bytes must match across impls.
    // Shapes: empty, short, long, repeated, many distinct (tree rebalancing),
    // multi-byte UTF-8.
    let (c, r) = Impl::both();
    let jc = c.newstate(0);
    let jr = r.newstate(0);
    let fc = c.f::<FnIntern>("js_intern");
    let fr = r.f::<FnIntern>("js_intern");
    let mut b = Batch::new();

    let mut inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        b"length".to_vec(),
        b"prototype".to_vec(),
        "caf\u{e9}".as_bytes().to_vec(),
        "\u{4f60}\u{597d}".as_bytes().to_vec(),
        vec![b'x'; 1000],
        vec![b'y'; 4096],
    ];
    // Many distinct keys to exercise the interning tree's rebalancing.
    let mut rng = Rng::new(0x1EAF_0006);
    for _ in 0..3000 {
        let n = 1 + rng.below(12) as usize;
        inputs.push((0..n).map(|_| *rng.pick(b"abcdefghijklmnopqrstuvwxyz0123456789_")).collect());
    }

    let mut first_c: std::collections::HashMap<Vec<u8>, usize> = Default::default();
    let mut first_r: std::collections::HashMap<Vec<u8>, usize> = Default::default();
    for s in &inputs {
        let buf = cbytes(s);
        let pc = unsafe { fc(jc, buf.as_ptr() as *const c_char) };
        let pr = unsafe { fr(jr, buf.as_ptr() as *const c_char) };
        let sc = unsafe { read_cstr(pc) };
        let sr = unsafe { read_cstr(pr) };
        b.check(&format!("js_intern({:?}) bytes", show(s)), &sc, &sr);
        // stability: same key -> same pointer within each impl
        let key = sc.clone().unwrap_or_default();
        let stable_c = *first_c.entry(key.clone()).or_insert(pc as usize) == pc as usize;
        let stable_r = *first_r.entry(key).or_insert(pr as usize) == pr as usize;
        b.check(&format!("js_intern({:?}) pointer stability", show(s)), stable_c, stable_r);
    }
    // Re-intern everything: still identical and still stable.
    for s in &inputs {
        let buf = cbytes(s);
        let pc = unsafe { fc(jc, buf.as_ptr() as *const c_char) };
        let pr = unsafe { fr(jr, buf.as_ptr() as *const c_char) };
        let sc = unsafe { read_cstr(pc) };
        let sr = unsafe { read_cstr(pr) };
        b.check(&format!("js_intern re-run({:?})", show(s)), &sc, &sr);
        let key = sc.clone().unwrap_or_default();
        b.check(
            &format!("js_intern re-run({:?}) stability", show(s)),
            first_c.get(&key) == Some(&(pc as usize)),
            first_r.get(&key) == Some(&(pr as usize)),
        );
    }
    b.finish("js_intern");
    c.freestate(jc);
    r.freestate(jr);
}
