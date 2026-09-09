//! CONFIGS.md rows 14-21 — the low-level regexp engine (`regexp.c`),
//! driven directly through `js_regcomp`/`js_regcompx`/`js_regexec`/`js_regfree`.
//! Also covers ERRORS.md rows S1-S6 and every `die()` site in `regexp.c`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* a shared allocator for js_regcompx / js_regfreex (row 20) */
unsafe extern "C-unwind" fn xalloc(_ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        if n == 0 {
            libc_free(ptr);
            std::ptr::null_mut()
        } else {
            libc_realloc(ptr, n as usize)
        }
    }
}
unsafe fn libc_free(p: *mut c_void) {
    if !p.is_null() {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void) = std::mem::transmute(free_addr());
            f(p)
        }
    }
}
unsafe fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    unsafe {
        let f: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void =
            std::mem::transmute(realloc_addr());
        f(p, n)
    }
}
fn free_addr() -> *const c_void {
    unsafe extern "C" {
        fn free(p: *mut c_void);
    }
    free as *const c_void
}
fn realloc_addr() -> *const c_void {
    unsafe extern "C" {
        fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    }
    realloc as *const c_void
}

/* ------------------------------------------------------------------ */

/// Hand-written patterns: every `die()` message in `regexp.c` plus valid shapes.
fn pattern_corpus() -> Vec<&'static str> {
    vec![
        /* --- valid --- */
        "", "a", "abc", "a|b", "a|b|c", "a*", "a+", "a?", "a*?", "a+?", "a??", "a{2}", "a{2,}",
        "a{2,4}", "a{0}", "a{0,0}", "a{2,4}?", "(a)", "(a)(b)", "(a|b)*", "(?:a)", "(?=a)", "(?!a)",
        "[a]", "[abc]", "[a-z]", "[^a-z]", "[a-]", "[-a]", "[]]", "[]", "[^]", ".", "^a", "a$",
        "^a$", "\\b", "\\B", "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\n", "\\r", "\\t", "\\v",
        "\\f", "\\0", "\\x41", "\\u0041", "\\cA", "\\cz", "\\.", "\\\\", "\\/", "\\-",
        "(a)\\1", "(a)(b)\\2\\1", "a(?=b)c", "(?:a|b)+c", "[\\d]", "[\\w-]", "[\\b]",
        "[a-c0-9_]", "(((((((((a)))))))))", "a{1,2}{3}", "\\u{41}",
        "[\\u0041-\\u005A]", "x(?:(a)|(b))y", "(a+)+b", "^(a|ab)*$",
        "a\u{00e9}b", "\u{20ac}+", "[\u{00e0}-\u{00ff}]", "\u{1f600}",
        /* nine and ten groups, and >REG_MAXSUB captures */
        "(1)(2)(3)(4)(5)(6)(7)(8)(9)", "(1)(2)(3)(4)(5)(6)(7)(8)(9)(10)",
        "(1)(2)(3)(4)(5)(6)(7)(8)(9)(a)(b)(c)(d)(e)(f)",
        "(1)(2)(3)(4)(5)(6)(7)(8)(9)(a)(b)(c)(d)(e)(f)(g)",
        "(1)(2)(3)(4)(5)(6)(7)(8)(9)(a)(b)(c)(d)(e)(f)(g)(h)",
        /* --- each die() site --- */
        "*",             // invalid quantifier (nothing to repeat)
        "+",             // invalid quantifier
        "?",             // invalid quantifier
        "{2}",           // invalid quantifier
        "a**",           // invalid quantifier
        "a++",           // invalid quantifier
        "a{2}{3}*",      // invalid quantifier chain
        "\\",            // unterminated escape sequence
        "\\x",           // unterminated escape sequence
        "\\x4",          // unterminated escape sequence
        "\\xZZ",         // unterminated escape sequence
        "\\u",           // unterminated escape sequence
        "\\u00",         // unterminated escape sequence
        "\\u00ZZ",       // unterminated escape sequence
        "\\c",           // unterminated escape sequence
        "\\c1",          // invalid escape character
        "\\p",           // invalid escape character?
        "\\8",           // invalid back-reference / escape
        "\\9",
        "\\1",           // invalid back-reference (no group)
        "(a)\\2",        // invalid back-reference
        "a{2,1}",        // numeric order / invalid quantifier
        "a{99999999999}", // numeric overflow
        "a{1,99999999999}", // numeric overflow
        "a{,1}",
        "[",             // unterminated character class
        "[a",            // unterminated character class
        "[a-",           // unterminated character class
        "[^",            // unterminated character class
        "[z-a]",         // invalid character class range
        "[\\d-z]",       // invalid character class range
        "(",             // unmatched '('
        "(a",            // unmatched '('
        "(?:",           // unmatched '('
        "(?=",           // unmatched '('
        "(?!",           // unmatched '('
        "(?",            // syntax error
        "(?<a>x)",       // syntax error (no named groups in mujs)
        ")",             // unmatched ')'
        "a)",            // unmatched ')'
        "a|)",
        "()*",           // infinite loop matching the empty string
        "(a*)*",         // infinite loop matching the empty string
        "(|a)*",
        "(?:)+",
        "\\u{}",
    ]
}

fn subject_corpus() -> Vec<&'static str> {
    vec![
        "", "a", "A", "b", "abc", "ABC", "aaa", "aab", "xay", "\n", "a\nb", "\r\n", "ab\ncd",
        "1", "12345", "  spaces  ", "_", "-", "]", "[", "z", "aA0_", "hello world",
        "a\u{00e9}b", "\u{20ac}\u{20ac}", "\u{1f600}x", "\u{00e0}\u{00ff}",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab", "0123456789abcdefghijklmnopqrstuvwxyz",
        "The quick brown Fox\njumps over 42 lazy dogs.", "\u{7}\u{1b}\u{7f}",
    ]
}

fn random_pattern(rng: &mut Rng) -> Vec<u8> {
    const ATOMS: &[&str] = &[
        "a", "b", "c", "z", "0", "9", "_", ".", "\\d", "\\w", "\\s", "\\S", "\\D", "\\W", "\\b",
        "\\B", "^", "$", "[a-c]", "[^a-c]", "[abc]", "[]", "[", "]", "(", ")", "(?:", "(?=", "(?!",
        "|", "*", "+", "?", "{1,2}", "{2}", "{0,}", "{,2}", "{9999999999}", "*?", "+?", "??",
        "\\1", "\\2", "\\9", "\\", "\\x41", "\\xZ", "\\u0041", "\\u00", "\\cA", "\\c", "-",
        "\u{00e9}", "\u{20ac}", "\u{1f600}", "\\n", "\\0", "\\/", "\\.",
    ];
    let n = rng.below(9) as usize;
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(rng.pick(ATOMS).as_bytes());
    }
    out
}

fn random_subject(rng: &mut Rng) -> Vec<u8> {
    const CH: &[&str] = &[
        "a", "b", "c", "A", "B", "z", "0", "1", "9", "_", "-", " ", "\n", "\r", "\t", ".", "[", "]",
        "(", ")", "\u{00e9}", "\u{20ac}", "\u{1f600}", "\u{7f}",
    ];
    let n = rng.below(14) as usize;
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(rng.pick(CH).as_bytes());
    }
    out
}

/// Compile + exec on both libs, comparing compile errors, match result and
/// every capture offset. `use_x` selects `js_regcompx`/`js_regfreex`.
fn diff_regexp(pat: &[u8], subjects: &[Vec<u8>], cflags: c_int, eflags: c_int, use_x: bool) {
    let p = both();
    let mut pz = pat.to_vec();
    pz.push(0);
    let pp = pz.as_ptr() as *const c_char;

    let mut ec: *const c_char = std::ptr::null();
    let mut er: *const c_char = std::ptr::null();
    let (pc, pr) = unsafe {
        if use_x {
            (
                (p.c.js_regcompx)(xalloc, std::ptr::null_mut(), pp, cflags, &mut ec),
                (p.r.js_regcompx)(xalloc, std::ptr::null_mut(), pp, cflags, &mut er),
            )
        } else {
            (
                (p.c.js_regcomp)(pp, cflags, &mut ec),
                (p.r.js_regcomp)(pp, cflags, &mut er),
            )
        }
    };
    let ctx = || format!("pattern={:?} cflags={cflags}", String::from_utf8_lossy(pat));

    assert_eq!(
        pc.is_null(),
        pr.is_null(),
        "regcomp NULL-ness differs for {} (C err={:?}, Rust err={:?})",
        ctx(),
        unsafe { opt_str(ec) }.map(|v| String::from_utf8_lossy(&v).into_owned()),
        unsafe { opt_str(er) }.map(|v| String::from_utf8_lossy(&v).into_owned()),
    );
    if pc.is_null() {
        assert_eq!(
            unsafe { opt_str(ec) },
            unsafe { opt_str(er) },
            "regcomp error message differs for {}",
            ctx()
        );
        return;
    }

    for subj in subjects {
        let mut sz = subj.clone();
        sz.push(0);
        let sc = sz.as_ptr() as *const c_char;

        // with capture output
        let mut mc = Resub::default();
        let mut mr = Resub::default();
        let (a, b) = unsafe {
            (
                (p.c.js_regexec)(pc, sc, &mut mc, eflags),
                (p.r.js_regexec)(pr, sc, &mut mr, eflags),
            )
        };
        let sctx = || {
            format!(
                "{} eflags={eflags} subject={:?}",
                ctx(),
                String::from_utf8_lossy(subj)
            )
        };
        assert_eq!(a, b, "js_regexec return differs for {}", sctx());
        if a == 0 {
            assert_eq!(mc.nsub, mr.nsub, "nsub differs for {}", sctx());
            for i in 0..REG_MAXSUB {
                let off = |q: *const c_char| {
                    if q.is_null() {
                        -1i64
                    } else {
                        q as i64 - sc as i64
                    }
                };
                assert_eq!(
                    (off(mc.sub[i].sp), off(mc.sub[i].ep)),
                    (off(mr.sub[i].sp), off(mr.sub[i].ep)),
                    "capture[{i}] differs for {}",
                    sctx()
                );
            }
        }

        // row 19: sub = NULL
        let (a, b) = unsafe {
            (
                (p.c.js_regexec)(pc, sc, std::ptr::null_mut(), eflags),
                (p.r.js_regexec)(pr, sc, std::ptr::null_mut(), eflags),
            )
        };
        assert_eq!(a, b, "js_regexec(sub=NULL) differs for {}", sctx());
    }

    unsafe {
        if use_x {
            (p.c.js_regfreex)(xalloc, std::ptr::null_mut(), pc);
            (p.r.js_regfreex)(xalloc, std::ptr::null_mut(), pr);
        } else {
            (p.c.js_regfree)(pc);
            (p.r.js_regfree)(pr);
        }
    }
}

fn run_flag_combo(cflags: c_int, eflags: c_int, seed: u64, use_x: bool) {
    let subjects: Vec<Vec<u8>> = subject_corpus()
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    for pat in pattern_corpus() {
        diff_regexp(pat.as_bytes(), &subjects, cflags, eflags, use_x);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..6000 {
        let pat = random_pattern(&mut rng);
        let subs: Vec<Vec<u8>> = (0..4).map(|_| random_subject(&mut rng)).collect();
        diff_regexp(&pat, &subs, cflags, eflags, use_x);
    }
}

/* rows 14-18: the cflags x eflags cross-product */
#[test]
fn row14_regexp_plain() {
    run_flag_combo(0, 0, 0xC0DE_0014, false);
}
#[test]
fn row15_regexp_icase() {
    run_flag_combo(REG_ICASE, 0, 0xC0DE_0015, false);
}
#[test]
fn row16_regexp_newline() {
    run_flag_combo(REG_NEWLINE, 0, 0xC0DE_0016, false);
}
#[test]
fn row17_regexp_icase_newline() {
    run_flag_combo(REG_ICASE | REG_NEWLINE, 0, 0xC0DE_0017, false);
}
#[test]
fn row18_regexp_notbol() {
    for cf in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
        run_flag_combo(cf, REG_NOTBOL, 0xC0DE_0018 + cf as u64, false);
    }
}

/* row 20: regcompx/regfreex with a caller-supplied allocator */
#[test]
fn row20_regcompx_custom_alloc() {
    run_flag_combo(0, 0, 0xC0DE_0020, true);
    run_flag_combo(REG_ICASE | REG_NEWLINE, REG_NOTBOL, 0xC0DE_0021, true);
}

/* row 21 + ERRORS S5/S6: capture counts, and out-of-range flag ints */
#[test]
fn row21_captures_and_out_of_range_flags() {
    let subjects: Vec<Vec<u8>> = subject_corpus()
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();

    // 0..20 capture groups
    for n in 0..21usize {
        let pat: String = (0..n).map(|i| format!("({})", (b'a' + (i % 26) as u8) as char)).collect();
        let mut subj = subjects.clone();
        subj.push("abcdefghijklmnopqrstuvwxyz".as_bytes().to_vec());
        for cf in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
            diff_regexp(pat.as_bytes(), &subj, cf, 0, false);
        }
    }

    // out-of-range / garbage flag ints (C enums accept any int)
    let odd: [c_int; 12] = [
        -1,
        8,
        16,
        64,
        1024,
        0x4000_0000,
        i32::MAX,
        i32::MIN,
        REG_NOTBOL,           // compile flag position holds an exec flag
        REG_ICASE | 8,
        REG_NEWLINE | 0x100,
        7,
    ];
    let pats = ["a", "^a$", "[a-z]+", "(a)(b)", "\\bx\\b", "a|B"];
    for &cf in &odd {
        for &ef in &odd {
            for pt in pats {
                diff_regexp(pt.as_bytes(), &subjects, cf, ef, false);
            }
        }
    }
}

/* ERRORS.md S4 + regexp.c:661/672/921/951 — recursion / program-size limits.
 *
 * NOTE: subjects are deliberately SHORT for the catastrophic-backtracking
 * patterns.  `(a+)+b` against a long run of 'a' is exponential in *both*
 * libraries (that is the C behaviour, faithfully reproduced), so a long
 * subject would only make the differential test hang, not find a bug. */
#[test]
fn regexp_limit_parse_recursion() {
    let subjects = vec![b"".to_vec(), b"a".to_vec()];
    for depth in [10usize, 100, 1000, 4000, 4095, 4096, 4097, 6000, 20000] {
        let pat = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        diff_regexp(pat.as_bytes(), &subjects[..1], 0, 0, false);
        let pat = format!("{}a{}", "(?:".repeat(depth), ")".repeat(depth));
        diff_regexp(pat.as_bytes(), &subjects[..1], 0, 0, false);
    }
}

#[test]
fn regexp_limit_program_size() {
    let subjects = vec![b"".to_vec(), b"aaaaaaaa".to_vec()];
    for n in [100usize, 1000, 10000, 40000, 70000] {
        diff_regexp("a".repeat(n).as_bytes(), &subjects, 0, 0, false);
        diff_regexp(("a|".repeat(n) + "b").as_bytes(), &subjects, 0, 0, false);
        diff_regexp(format!("(a){{{}}}", n).as_bytes(), &subjects, 0, 0, false);
        diff_regexp(format!("a{{{},{}}}", n, n + 1).as_bytes(), &subjects, 0, 0, false);
    }
}

#[test]
fn regexp_limit_character_classes() {
    let subjects = vec![b"".to_vec(), b"ab".to_vec(), "\u{0200}".as_bytes().to_vec()];
    // REG_MAXCLASS = 128 distinct classes
    for n in [1usize, 127, 128, 129, 300] {
        let pat = "[a-b]".repeat(n);
        diff_regexp(pat.as_bytes(), &subjects, 0, 0, false);
        diff_regexp(pat.as_bytes(), &subjects, REG_ICASE, 0, false);
    }
    // one class with very many ranges (nelem(cc->spans) bound)
    for n in [1usize, 20, 32, 63, 64, 65, 100, 200] {
        let inner: String = (0..n)
            .map(|i| format!("\\u{:04x}-\\u{:04x}", 0x100 + i * 4, 0x102 + i * 4))
            .collect();
        let pat = format!("[{}]", inner);
        diff_regexp(pat.as_bytes(), &subjects, 0, 0, false);
        diff_regexp(pat.as_bytes(), &subjects, REG_ICASE, 0, false);
        diff_regexp(pat.as_bytes(), &subjects, REG_NEWLINE, 0, false);
    }
}

#[test]
fn regexp_limit_exec_recursion() {
    // short subjects only — see the note above
    let subjects: Vec<Vec<u8>> = (0..=14).map(|n| vec![b'a'; n]).collect();
    for pat in ["(a*)*b", "(a|a)*b", "(a+)+b", "^(a|aa)+$", "(a?)*b", "((a)*)*b"] {
        diff_regexp(pat.as_bytes(), &subjects, 0, 0, false);
    }
    // long subject with a *linear* pattern still exercises the exec loop deeply
    let long = vec![b'a'; 100_000];
    for pat in ["a*b", "^a*$", "[a]*", "(a)*"] {
        diff_regexp(pat.as_bytes(), std::slice::from_ref(&long), 0, 0, false);
    }
}

#[test]
fn regexp_limit_max_captures() {
    let subjects = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
    ];
    for n in [1usize, 9, 14, 15, 16, 17, 20, 50] {
        let pat: String = (0..n).map(|_| "(a)".to_string()).collect();
        diff_regexp(pat.as_bytes(), &subjects, 0, 0, false);
        // backreferences at and past REG_MAXSUB
        let pat = format!("{}\\{}", pat, n.min(9));
        diff_regexp(pat.as_bytes(), &subjects, 0, 0, false);
    }
}
