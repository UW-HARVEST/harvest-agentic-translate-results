//! Phase B/C — differential tests for the standalone regexp engine
//! (`regexp.c` / `regexp.h`): `js_regcomp`, `js_regcompx`, `js_regexec`,
//! `js_regfree`, `js_regfreex`.
//!
//! CONFIGS.md rows 18-25. ERRORS.md section 1 (rows 1-36).

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

const SEED: u64 = 0x2EE1_9876_5432_0001;

const REG_ICASE: c_int = 1;
const REG_NEWLINE: c_int = 2;
const REG_NOTBOL: c_int = 4;

type FnRegcomp =
    extern "C" fn(*const c_char, c_int, *mut *const c_char) -> Reprog;
type FnRegcompx = extern "C" fn(
    Option<JsAlloc>,
    *mut c_void,
    *const c_char,
    c_int,
    *mut *const c_char,
) -> Reprog;
type FnRegexec = extern "C" fn(Reprog, *const c_char, *mut Resub, c_int) -> c_int;
type FnRegfree = extern "C" fn(Reprog);
type FnRegfreex = extern "C" fn(Option<JsAlloc>, *mut c_void, Reprog);

struct Engine {
    comp: Pair<FnRegcomp>,
    compx: Pair<FnRegcompx>,
    exec: Pair<FnRegexec>,
    free: Pair<FnRegfree>,
    freex: Pair<FnRegfreex>,
}

fn engine() -> Engine {
    Engine {
        comp: both_fn("js_regcomp"),
        compx: both_fn("js_regcompx"),
        exec: both_fn("js_regexec"),
        free: both_fn("js_regfree"),
        freex: both_fn("js_regfreex"),
    }
}

/// Outcome of one compile+exec differential probe, in a form that can be
/// compared without touching raw addresses.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    compiled: bool,
    error: Option<Vec<u8>>,
    /// exec return code + normalized capture offsets, per (subject, eflags)
    runs: Vec<(c_int, Vec<Option<(isize, isize)>>)>,
}

fn probe(
    comp: FnRegcomp,
    exec: FnRegexec,
    free: FnRegfree,
    pattern: &[u8],
    cflags: c_int,
    subjects: &[Vec<u8>],
    eflags_set: &[c_int],
) -> Outcome {
    let pz = cstr_bytes(pattern);
    let mut err: *const c_char = std::ptr::null();
    let prog = comp(pz.as_ptr() as *const c_char, cflags, &mut err);
    if prog.is_null() {
        return Outcome {
            compiled: false,
            error: unsafe { read_cstr(err) },
            runs: Vec::new(),
        };
    }
    let mut runs = Vec::new();
    for s in subjects {
        let sz = cstr_bytes(s);
        let base = sz.as_ptr() as *const c_char;
        for &ef in eflags_set {
            let mut sub = Resub::default();
            let rc = exec(prog, base, &mut sub as *mut Resub, ef);
            let caps = if rc == 0 { sub.offsets(base) } else { Vec::new() };
            runs.push((rc, caps));
        }
        // ERRORS section 1: `sub == NULL`
        let mut sub_null_rcs = Vec::new();
        for &ef in eflags_set {
            sub_null_rcs.push(exec(prog, base, std::ptr::null_mut(), ef));
        }
        runs.push((
            sub_null_rcs.iter().sum(),
            vec![None; sub_null_rcs.len()],
        ));
    }
    free(prog);
    Outcome {
        compiled: true,
        error: None,
        runs,
    }
}

fn assert_same(
    e: &Engine,
    pattern: &[u8],
    cflags: c_int,
    subjects: &[Vec<u8>],
    eflags_set: &[c_int],
) {
    let a = probe(e.comp.c, e.exec.c, e.free.c, pattern, cflags, subjects, eflags_set);
    let b = probe(
        e.comp.rust,
        e.exec.rust,
        e.free.rust,
        pattern,
        cflags,
        subjects,
        eflags_set,
    );
    assert_eq!(
        a.compiled, b.compiled,
        "compile disagreement for /{}/ cflags={cflags}: C err={} RUST err={}",
        String::from_utf8_lossy(pattern),
        show(&a.error),
        show(&b.error)
    );
    assert_eq!(
        a.error, b.error,
        "error message disagreement for /{}/ cflags={cflags}: C={} RUST={}",
        String::from_utf8_lossy(pattern),
        show(&a.error),
        show(&b.error)
    );
    assert_eq!(
        a.runs,
        b.runs,
        "exec disagreement for /{}/ cflags={cflags}",
        String::from_utf8_lossy(pattern)
    );
}

fn all_cflags() -> Vec<c_int> {
    vec![0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE]
}

fn default_subjects() -> Vec<Vec<u8>> {
    [
        "",
        "a",
        "A",
        "abc",
        "ABC",
        "aaa",
        "abcabc",
        "xyz",
        "a\nb",
        "\n",
        "\r\n",
        "  a  ",
        "0123456789",
        "aA1!_",
        "The quick brown fox",
        "\u{e9}\u{4e2d}\u{1F600}",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
        "foo bar baz",
        "\u{0}x",
        "line1\nline2\nline3",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect()
}

// ===========================================================================
// ERRORS.md section 1 — every compile-time rejection
// ===========================================================================

#[test]
fn regexp_error_surface() {
    let e = engine();
    let subjects = vec![b"abc".to_vec()];
    let eflags = [0];

    // (pattern, the ERRORS.md row it covers)
    let cases: Vec<(&[u8], &str)> = vec![
        (b"\\q", "row 1 invalid escape sequence"),
        (b"\\A", "row 1 invalid escape sequence"),
        (b"a**", "row 2 invalid quantifier"),
        (b"a++", "row 2 invalid quantifier"),
        (b"a??", "row 2 invalid quantifier"),
        (b"a{1,2}{3}", "row 2 invalid quantifier"),
        (b"a\\", "row 3 unterminated escape sequence"),
        (b"\\", "row 3 unterminated escape sequence"),
        (b"\\xA", "row 4 unterminated escape sequence"),
        (b"\\x", "row 4 unterminated escape sequence"),
        (b"\\xZZ", "row 4 unterminated escape sequence"),
        (b"\\u12", "row 5 unterminated escape sequence"),
        (b"\\u", "row 5 unterminated escape sequence"),
        (b"\\uZZZZ", "row 5 unterminated escape sequence"),
        (b"\\c", "row 6 unterminated escape sequence"),
        (b"\\c1", "row 7 invalid escape character"),
        (b"\\c!", "row 7 invalid escape character"),
        (b"a{100000}", "row 8 numeric overflow"),
        (b"a{256}", "row 8 numeric overflow"),
        (b"a{1,99999}", "row 9 numeric overflow"),
        (b"a{1,256}", "row 9 numeric overflow"),
        (b"[z-a]", "row 11 invalid character class range"),
        (b"[\\u0100-\\u0000]", "row 11 invalid character class range"),
        (b"[abc", "row 13 unterminated character class"),
        (b"[", "row 13 unterminated character class"),
        (b"[^", "row 13 unterminated character class"),
        (b"[a-", "row 13 unterminated character class"),
        (b"(?:)*", "row 14 infinite loop matching the empty string"),
        (b"(a*)*", "row 14 infinite loop matching the empty string"),
        (b"()+", "row 14 infinite loop matching the empty string"),
        (b"(|a)*", "row 14 infinite loop matching the empty string"),
        (b"(a)\\2", "row 15 invalid back-reference"),
        (b"\\1", "row 15 invalid back-reference"),
        (b"\\9", "row 15 invalid back-reference"),
        (b"(", "row 17 unmatched '('"),
        (b"(a", "row 17 unmatched '('"),
        (b"(?:", "row 18 unmatched '('"),
        (b"(?:a", "row 18 unmatched '('"),
        (b"(?=", "row 19 unmatched '('"),
        (b"(?=a", "row 19 unmatched '('"),
        (b"(?!", "row 20 unmatched '('"),
        (b"(?!a", "row 20 unmatched '('"),
        (b"*", "row 22 invalid quantifier"),
        (b"+", "row 22 invalid quantifier"),
        (b"?", "row 22 invalid quantifier"),
        (b"{1}", "row 22 quantifier with no atom"),
        (b"{1,2}", "row 22 quantifier with no atom"),
        (b"a)", "row 25 unmatched ')'"),
        (b")", "row 25 unmatched ')'"),
        (b"a)b", "row 25 unmatched ')'"),
        // 17 capture groups -> "too many captures" (REG_MAXSUB is 16)
        (
            b"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)",
            "row 16 too many captures",
        ),
        // valid-but-adjacent shapes: must compile in both
        (b"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)", "15 groups ok"),
        (b"a{255}", "boundary: 255 is the max repeat"),
        (b"a{0,255}", "boundary: 255 is the max repeat"),
        (b"a{2,1}", "reversed repeat bounds"),
        (b"a{,2}", "malformed repeat, treated literally?"),
        (b"a{}", "malformed repeat"),
        (b"[a-a]", "single-char range"),
        (b"", "empty pattern"),
        (b"|", "empty alternatives"),
        (b"||", "empty alternatives"),
        (b"^$", "anchors only"),
        (b"(?=a)", "lookahead alone"),
        (b"(?!a)", "negative lookahead alone"),
        (b"\\b\\B\\d\\D\\s\\S\\w\\W", "all class escapes"),
        (b"\\0", "NUL escape"),
        (b"\\x41", "hex escape"),
        (b"\\u0041", "unicode escape"),
        (b"\\cA", "control escape"),
        (b"[]", "empty class"),
        (b"[^]", "empty negated class"),
        (b"[\\b]", "backspace in class"),
        (b"[a-]", "trailing dash in class"),
        (b"[-a]", "leading dash in class"),
        (b".", "any"),
        (b"$", "dollar"),
        (b"^", "caret"),
    ];

    for (pat, label) in cases {
        for cf in all_cflags() {
            let a = probe(e.comp.c, e.exec.c, e.free.c, pat, cf, &subjects, &eflags);
            let b = probe(
                e.comp.rust,
                e.exec.rust,
                e.free.rust,
                pat,
                cf,
                &subjects,
                &eflags,
            );
            assert_eq!(
                (a.compiled, &a.error),
                (b.compiled, &b.error),
                "{label}: /{}/ cflags={cf}\n  C: compiled={} err={}\n  R: compiled={} err={}",
                String::from_utf8_lossy(pat),
                a.compiled,
                show(&a.error),
                b.compiled,
                show(&b.error)
            );
            assert_eq!(a.runs, b.runs, "{label}: /{}/ cflags={cf} exec", String::from_utf8_lossy(pat));
        }
    }
}

/// ERRORS.md rows 10 & 12: the class / span limits.
#[test]
fn regexp_class_limits() {
    let e = engine();
    let subjects = vec![b"abc".to_vec(), b"".to_vec()];

    // REG_MAXCLASS = 128 distinct character classes
    for n in [1usize, 127, 128, 129, 200] {
        let pat: Vec<u8> = (0..n).flat_map(|_| b"[ab]".to_vec()).collect();
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
    // REG_MAXSPAN = 64 ranges inside a single class
    for n in [1usize, 30, 63, 64, 65, 100] {
        let mut pat = vec![b'['];
        for i in 0..n {
            pat.push(b'a' + (i % 26) as u8);
        }
        pat.push(b']');
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
    // Explicit ranges (each range consumes 2 spans in the C encoding)
    for n in [1usize, 20, 31, 32, 33, 64, 100] {
        let mut pat = vec![b'['];
        for i in 0..n {
            let c = b'a' + (i % 20) as u8;
            pat.push(c);
            pat.push(b'-');
            pat.push(c + 1);
        }
        pat.push(b']');
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
}

/// ERRORS.md rows 23-24: `REG_MAXREC` and `REG_MAXPROG`.
#[test]
fn regexp_program_and_recursion_limits() {
    let e = engine();
    let subjects = vec![b"aaaa".to_vec()];
    // long flat program: `aaaa...`
    for n in [1usize, 1000, 8000, 16000, 32000, 40000] {
        let pat: Vec<u8> = vec![b'a'; n];
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
    // nested repeats blow up the instruction count
    for n in [2usize, 8, 20, 40] {
        let pat: Vec<u8> = format!("(?:a{{{n}}}){{{n}}}").into_bytes();
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
    // deep nesting (stays well under the C stack limit but past small depths)
    for depth in [1usize, 10, 100, 500, 1000] {
        let mut pat = Vec::new();
        for _ in 0..depth {
            pat.extend_from_slice(b"(?:");
        }
        pat.push(b'a');
        for _ in 0..depth {
            pat.push(b')');
        }
        assert_same(&e, &pat, 0, &subjects, &[0]);
    }
}

/// ERRORS.md row 27: `errorp == NULL`.
#[test]
fn regexp_null_errorp_and_null_prog() {
    let e = engine();
    for pat in [&b"("[..], &b"a**"[..], &b"abc"[..], &b"[z-a]"[..]] {
        let pz = cstr_bytes(pat);
        let pc = (e.comp.c)(pz.as_ptr() as *const c_char, 0, std::ptr::null_mut());
        let pr = (e.comp.rust)(pz.as_ptr() as *const c_char, 0, std::ptr::null_mut());
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "NULL errorp compile of /{}/",
            String::from_utf8_lossy(pat)
        );
        if !pc.is_null() {
            (e.free.c)(pc);
        }
        if !pr.is_null() {
            (e.free.rust)(pr);
        }
    }
    // ERRORS.md row 36: `js_regfree(NULL)` is a no-op.
    (e.free.c)(std::ptr::null_mut());
    (e.free.rust)(std::ptr::null_mut());
}

// ===========================================================================
// CONFIGS.md rows 23 & 28: custom allocator (`js_regcompx` / `js_regfreex`)
// ===========================================================================

/// Counting bump allocator: `alloc(ctx, ptr, 0)` frees, otherwise realloc.
#[repr(C)]
struct AllocCtx {
    allocs: u64,
    frees: u64,
    /// when > 0, fail the Nth allocation (1-based) to exercise ERRORS rows 28-31
    fail_at: u64,
    seen: u64,
}

extern "C" fn counting_alloc(ctx: *mut c_void, ptr: *mut c_void, size: c_int) -> *mut c_void {
    unsafe {
        let c = &mut *(ctx as *mut AllocCtx);
        if size == 0 {
            if !ptr.is_null() {
                c.frees += 1;
                libc_free(ptr);
            }
            return std::ptr::null_mut();
        }
        c.seen += 1;
        if c.fail_at != 0 && c.seen == c.fail_at {
            return std::ptr::null_mut();
        }
        c.allocs += 1;
        libc_realloc(ptr, size as usize)
    }
}

// Minimal libc bindings (the test crate has no libc dependency).
unsafe extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn regexp_custom_allocator() {
    let e = engine();
    let subjects = default_subjects();
    let patterns: Vec<&[u8]> = vec![
        b"abc",
        b"a+b*c?",
        b"(a)(b)(c)",
        b"[a-z]+",
        b"^foo$",
        b"(?:ab|cd)+",
        b"\\d{2,4}",
        b"(",
        b"a**",
    ];

    for pat in &patterns {
        for cf in all_cflags() {
            let pz = cstr_bytes(pat);
            let run = |compx: FnRegcompx,
                           exec: FnRegexec,
                           freex: FnRegfreex|
             -> (bool, Option<Vec<u8>>, Vec<(c_int, Vec<Option<(isize, isize)>>)>, u64, u64) {
                let mut ctx = AllocCtx {
                    allocs: 0,
                    frees: 0,
                    fail_at: 0,
                    seen: 0,
                };
                let cp = &mut ctx as *mut AllocCtx as *mut c_void;
                let mut err: *const c_char = std::ptr::null();
                let prog = compx(
                    Some(counting_alloc),
                    cp,
                    pz.as_ptr() as *const c_char,
                    cf,
                    &mut err,
                );
                if prog.is_null() {
                    let msg = unsafe { read_cstr(err) };
                    return (false, msg, Vec::new(), ctx.allocs, ctx.frees);
                }
                let mut runs = Vec::new();
                for s in &subjects {
                    let sz = cstr_bytes(s);
                    let base = sz.as_ptr() as *const c_char;
                    for ef in [0, REG_NOTBOL] {
                        let mut sub = Resub::default();
                        let rc = exec(prog, base, &mut sub as *mut Resub, ef);
                        let caps = if rc == 0 { sub.offsets(base) } else { Vec::new() };
                        runs.push((rc, caps));
                    }
                }
                freex(Some(counting_alloc), cp, prog);
                (true, None, runs, ctx.allocs, ctx.frees)
            };

            let a = run(e.compx.c, e.exec.c, e.freex.c);
            let b = run(e.compx.rust, e.exec.rust, e.freex.rust);
            assert_eq!(
                (a.0, &a.1, &a.2),
                (b.0, &b.1, &b.2),
                "js_regcompx /{}/ cflags={cf}",
                String::from_utf8_lossy(pat)
            );
            // The allocation *count* is part of the observable contract for a
            // custom allocator (MuJS accounts memory through it).
            assert_eq!(
                (a.3, a.4),
                (b.3, b.4),
                "allocation counts for /{}/ cflags={cf}: C=(alloc {},free {}) RUST=(alloc {},free {})",
                String::from_utf8_lossy(pat),
                a.3,
                a.4,
                b.3,
                b.4
            );
        }
    }
}

/// ERRORS.md rows 28-31: each allocation-failure branch inside `js_regcompx`.
#[test]
fn regexp_allocation_failures() {
    let e = engine();
    for pat in [
        &b"abc"[..],
        &b"(a)(b)[c-z]+"[..],
        &b"^(?:ab|cd)*$"[..],
        &b"\\d{1,3}[a-f]"[..],
    ] {
        let pz = cstr_bytes(pat);
        for fail_at in 1..=6u64 {
            let probe1 = |compx: FnRegcompx, freex: FnRegfreex| {
                let mut ctx = AllocCtx {
                    allocs: 0,
                    frees: 0,
                    fail_at,
                    seen: 0,
                };
                let cp = &mut ctx as *mut AllocCtx as *mut c_void;
                let mut err: *const c_char = std::ptr::null();
                let prog = compx(
                    Some(counting_alloc),
                    cp,
                    pz.as_ptr() as *const c_char,
                    0,
                    &mut err,
                );
                let out = (prog.is_null(), unsafe { read_cstr(err) });
                if !prog.is_null() {
                    freex(Some(counting_alloc), cp, prog);
                }
                out
            };
            let a = probe1(e.compx.c, e.freex.c);
            let b = probe1(e.compx.rust, e.freex.rust);
            assert_eq!(
                a,
                b,
                "alloc failure #{fail_at} for /{}/: C=({}, {}) RUST=({}, {})",
                String::from_utf8_lossy(pat),
                a.0,
                show(&a.1),
                b.0,
                show(&b.1)
            );
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 18-22, 24-25: valid patterns across every flag combination
// ===========================================================================

#[test]
fn regexp_handwritten_corpus_all_flags() {
    let e = engine();
    let subjects = default_subjects();
    let eflags = [0, REG_NOTBOL];
    let patterns: Vec<&[u8]> = vec![
        b"a",
        b"abc",
        b"a|b",
        b"a|b|c",
        b"a*",
        b"a+",
        b"a?",
        b"a{2}",
        b"a{2,}",
        b"a{2,4}",
        b"a*?",
        b"a+?",
        b"a??",
        b"a{2,4}?",
        b".",
        b".*",
        b"^a",
        b"a$",
        b"^a$",
        b"^",
        b"$",
        b"\\ba\\b",
        b"\\Ba\\B",
        b"\\d",
        b"\\D",
        b"\\s",
        b"\\S",
        b"\\w",
        b"\\W",
        b"[abc]",
        b"[^abc]",
        b"[a-z]",
        b"[A-Z]",
        b"[0-9]",
        b"[a-zA-Z0-9_]",
        b"[^a-z]",
        b"[\\d\\s]",
        b"[\\w-]",
        b"(a)",
        b"(a)(b)",
        b"(a(b))",
        b"(?:a)",
        b"(?:a|b)+",
        b"(a)\\1",
        b"(a)(b)\\2\\1",
        b"(?=a)a",
        b"(?!a)b",
        b"a(?=b)",
        b"a(?!b)",
        b"(a|b)*c",
        b"[a-z]+@[a-z]+\\.[a-z]+",
        b"\\x41",
        b"\\u0041",
        b"\\cA",
        b"\\0",
        b"\\n",
        b"\\r",
        b"\\t",
        b"\\f",
        b"\\v",
        b"\\.",
        b"\\*",
        b"\\\\",
        b"\\/",
        b"[\\]]",
        b"[\\^]",
        b"(a)|(b)",
        b"^(a+)(b+)$",
        b"(.)(.)(.)",
        b"a.*?b",
        b"\\w+\\s+\\w+",
        b"(?:(a)|(b))+",
        b"[\\u0100-\\u0200]",
        b"\\u00e9",
        b"(?:\\d{1,3}\\.){3}\\d{1,3}",
        b"(a*)(b*)(c*)",
        b"[^\\n]",
        b"[\\s\\S]",
        b"x|",
        b"|x",
    ];

    for pat in &patterns {
        for cf in all_cflags() {
            assert_same(&e, pat, cf, &subjects, &eflags);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 25: randomized pattern + subject property test
// ===========================================================================

struct PatGen {
    rng: Rng,
    groups: u32,
}

impl PatGen {
    fn new(seed: u64) -> Self {
        PatGen {
            rng: Rng::new(seed),
            groups: 0,
        }
    }

    fn atom(&mut self, depth: u32, out: &mut Vec<u8>) {
        match self.rng.below(if depth >= 3 { 8 } else { 13 }) {
            0 => out.push(b"abcxyzAZ019 _\n\t.-"[self.rng.below(17) as usize]),
            1 => out.push(b'.'),
            2 => {
                out.extend_from_slice(
                    [
                        &b"\\d"[..],
                        &b"\\D"[..],
                        &b"\\s"[..],
                        &b"\\S"[..],
                        &b"\\w"[..],
                        &b"\\W"[..],
                        &b"\\b"[..],
                        &b"\\B"[..],
                        &b"\\n"[..],
                        &b"\\t"[..],
                        &b"\\."[..],
                        &b"\\\\"[..],
                        &b"\\x41"[..],
                        &b"\\u0062"[..],
                        &b"\\cA"[..],
                        &b"\\0"[..],
                    ][self.rng.below(16) as usize],
                );
            }
            3 => {
                // character class
                out.push(b'[');
                if self.rng.bool() {
                    out.push(b'^');
                }
                let n = 1 + self.rng.below(5);
                for _ in 0..n {
                    match self.rng.below(4) {
                        0 => out.push(b"abcxyzAZ019 _"[self.rng.below(13) as usize]),
                        1 => {
                            let lo = b'a' + self.rng.below(20) as u8;
                            let hi = lo + self.rng.below(6) as u8;
                            out.push(lo);
                            out.push(b'-');
                            out.push(hi);
                        }
                        2 => out.extend_from_slice(
                            [&b"\\d"[..], &b"\\w"[..], &b"\\s"[..], &b"\\]"[..]]
                                [self.rng.below(4) as usize],
                        ),
                        _ => out.push(b'-'),
                    }
                }
                out.push(b']');
            }
            4 => out.push(b'^'),
            5 => out.push(b'$'),
            6 if self.groups > 0 => {
                out.push(b'\\');
                out.push(b'1' + self.rng.below(self.groups.min(9)) as u8);
            }
            6 => out.push(b'z'),
            7 => out.push(b"abc"[self.rng.below(3) as usize]),
            8 => {
                // capturing group
                self.groups += 1;
                out.push(b'(');
                self.alt(depth + 1, out);
                out.push(b')');
            }
            9 => {
                out.extend_from_slice(b"(?:");
                self.alt(depth + 1, out);
                out.push(b')');
            }
            10 => {
                out.extend_from_slice(b"(?=");
                self.alt(depth + 1, out);
                out.push(b')');
            }
            11 => {
                out.extend_from_slice(b"(?!");
                self.alt(depth + 1, out);
                out.push(b')');
            }
            _ => out.push(b'q'),
        }
    }

    fn term(&mut self, depth: u32, out: &mut Vec<u8>) {
        self.atom(depth, out);
        match self.rng.below(10) {
            0 => out.push(b'*'),
            1 => out.push(b'+'),
            2 => out.push(b'?'),
            3 => {
                let n = self.rng.below(6);
                out.extend_from_slice(format!("{{{n}}}").as_bytes());
            }
            4 => {
                let n = self.rng.below(4);
                let m = n + self.rng.below(4);
                out.extend_from_slice(format!("{{{n},{m}}}").as_bytes());
            }
            5 => {
                let n = self.rng.below(4);
                out.extend_from_slice(format!("{{{n},}}").as_bytes());
            }
            6 => {
                out.push(b"*+?"[self.rng.below(3) as usize]);
                out.push(b'?');
            }
            _ => {}
        }
    }

    fn seq(&mut self, depth: u32, out: &mut Vec<u8>) {
        let n = 1 + self.rng.below(if depth >= 3 { 2 } else { 4 });
        for _ in 0..n {
            self.term(depth, out);
        }
    }

    fn alt(&mut self, depth: u32, out: &mut Vec<u8>) {
        self.seq(depth, out);
        let n = self.rng.below(3);
        for _ in 0..n {
            out.push(b'|');
            self.seq(depth, out);
        }
    }

    fn pattern(&mut self) -> Vec<u8> {
        self.groups = 0;
        let mut v = Vec::new();
        self.alt(0, &mut v);
        v
    }
}

fn random_subjects(rng: &mut Rng, n: usize, maxlen: u32) -> Vec<Vec<u8>> {
    let alphabet: &[u8] = b"abcxyzAZ019 _\n\t.-()[]{}|*+?\\";
    (0..n)
        .map(|_| {
            let len = rng.below(maxlen) as usize;
            (0..len)
                .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
                .collect()
        })
        .collect()
}

/// Randomized patterns are matched against SHORT subjects only: the C engine is
/// a backtracking matcher, so a pattern like `(a*)*b` against a 20-char subject
/// takes exponential time in *both* implementations. Short subjects keep the
/// differential comparison meaningful and the runtime bounded.
fn short_subjects(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = [
        "", "a", "b", "ab", "aab", "abc", "A", "0", " ", "\n", "a\nb", "z", "aaa",
        "a b", "_9", "abcabc",
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    v.extend(random_subjects(rng, 12, 7));
    v
}

#[test]
fn regexp_randomized_property() {
    let e = engine();
    let mut pg = PatGen::new(SEED);
    let mut srng = Rng::new(SEED ^ 0xABCD);
    let subjects = short_subjects(&mut srng);

    let mut tested = 0usize;
    for i in 0..25000 {
        let pat = pg.pattern();
        if pat.len() > 70 {
            continue;
        }
        let cf = all_cflags()[(i % 4) as usize];
        assert_same(&e, &pat, cf, &subjects, &[0, REG_NOTBOL]);
        tested += 1;
    }
    assert!(tested > 8000, "generator produced too few patterns: {tested}");
}

/// Same generator, but with the longer/structured subject corpus and patterns
/// restricted to ones without nested unbounded quantifiers, so the backtracking
/// matcher stays fast.
#[test]
fn regexp_randomized_property_long_subjects() {
    let e = engine();
    let mut pg = PatGen::new(SEED ^ 0x5A5A);
    let mut fixed = default_subjects();
    let mut srng = Rng::new(SEED ^ 0x1234);
    fixed.extend(random_subjects(&mut srng, 16, 20));

    fn risky(p: &[u8]) -> bool {
        // `)*`, `)+`, `)?`, `){` after a group that itself contains `*`/`+`
        let s = String::from_utf8_lossy(p);
        let stars = s.matches('*').count() + s.matches('+').count();
        stars > 1 || s.contains(")*") || s.contains(")+") || s.contains("){")
    }

    let mut tested = 0usize;
    for i in 0..25000 {
        let pat = pg.pattern();
        if pat.len() > 70 || risky(&pat) {
            continue;
        }
        let cf = all_cflags()[(i % 4) as usize];
        assert_same(&e, &pat, cf, &fixed, &[0, REG_NOTBOL]);
        tested += 1;
    }
    assert!(tested > 3000, "generator produced too few patterns: {tested}");
}

/// CONFIGS.md row 24: capture-count sweep from 0 to REG_MAXSUB.
#[test]
fn regexp_capture_counts() {
    let e = engine();
    let subjects: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"aaaaaaaaaaaaaaaaaaaa".to_vec(),
        b"abcdefghijklmnop".to_vec(),
    ];
    for n in 0..=17usize {
        let pat: Vec<u8> = (0..n).map(|_| "(a)".to_string()).collect::<String>().into_bytes();
        for cf in all_cflags() {
            assert_same(&e, &pat, cf, &subjects, &[0, REG_NOTBOL]);
        }
        // optional groups leave unset captures
        let pat2: Vec<u8> = (0..n)
            .map(|_| "(a)?".to_string())
            .collect::<String>()
            .into_bytes();
        for cf in all_cflags() {
            assert_same(&e, &pat2, cf, &subjects, &[0, REG_NOTBOL]);
        }
    }
}

/// ERRORS.md rows 32-35: exec-level behaviours.
#[test]
fn regexp_exec_edge_cases() {
    let e = engine();
    // no match; anchored + REG_NOTBOL; long subject; empty subject
    let long: Vec<u8> = vec![b'a'; 5000];
    let subjects = vec![
        b"".to_vec(),
        b"zzz".to_vec(),
        long.clone(),
        b"\n\nabc".to_vec(),
        b"abc\n".to_vec(),
    ];
    for pat in [
        &b"^abc"[..],
        &b"abc$"[..],
        &b"^$"[..],
        &b"^"[..],
        &b"$"[..],
        &b"a*"[..],
        &b"(a*)*b"[..],
        &b"nomatch"[..],
        &b"a{100}"[..],
        &b"(?:a|aa)+b"[..],
    ] {
        for cf in all_cflags() {
            assert_same(&e, pat, cf, &subjects, &[0, REG_NOTBOL, REG_NOTBOL | 8, 8, 16, -1]);
        }
    }
}
