//! Phase B/C: differential tests for regexp.c through js_regcomp / js_regcompx /
//! js_regexec / js_regfree / js_regfreex.
//! CONFIGS.md rows 198-284; the regexp.c ERRORS.md rows 731-793.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/// A custom allocator for js_regcompx / js_regfreex (CONFIGS row 2).
static mut ALLOC_CALLS: i64 = 0;

unsafe extern "C" fn my_alloc(ctx: *mut c_void, ptr: *mut c_void, n: c_int) -> *mut c_void {
    ALLOC_CALLS += 1;
    // ctx is a magic cookie we assert on
    assert_eq!(ctx as usize, 0xC0FFEE);
    if n == 0 {
        libc_free(ptr);
        return std::ptr::null_mut();
    }
    libc_realloc(ptr, n as usize)
}

extern "C" {
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

/// Compile + exec on one library, returning a fully-owned description of the
/// result so C and Rust can be compared structurally.
#[derive(Debug, PartialEq, Eq, Clone)]
struct ExecOut {
    /// None => regcomp failed; Some(msg) is the error string
    comp_err: Option<String>,
    /// per-(text, eflags) results: (regexec return, captures as byte offsets)
    runs: Vec<(c_int, Vec<Option<(isize, isize)>>, c_int)>,
}

unsafe fn compile_and_run(
    l: &Lib,
    pattern: &str,
    cflags: c_int,
    texts: &[(&str, c_int)],
    use_x: bool,
    sub_null: bool,
) -> ExecOut {
    let cp = cstr(pattern);
    let mut errp: *const c_char = std::ptr::null();
    let prog = if use_x {
        l.js_regcompx(
            Some(my_alloc),
            0xC0FFEE as *mut c_void,
            cp.as_ptr(),
            cflags,
            &mut errp,
        )
    } else {
        l.js_regcomp(cp.as_ptr(), cflags, &mut errp)
    };
    if prog.is_null() {
        return ExecOut {
            comp_err: Some(from_c(errp)),
            runs: vec![],
        };
    }
    // On success the C code leaves *errorp untouched; don't compare it.
    let mut runs = vec![];
    for (text, eflags) in texts {
        let ct = cstr(text);
        let base = ct.as_ptr();
        if sub_null {
            let rc = l.js_regexec(prog, base, std::ptr::null_mut(), *eflags);
            runs.push((rc, vec![], -1));
        } else {
            let mut sub = Resub::default();
            // poison so we can see exactly which fields the engine writes
            sub.nsub = 12345;
            for i in 0..REG_MAXSUB {
                sub.sub[i].sp = 1 as *const c_char;
                sub.sub[i].ep = 2 as *const c_char;
            }
            let rc = l.js_regexec(prog, base, &mut sub, *eflags);
            let mut caps = vec![];
            if rc == 0 {
                let n = sub.nsub.clamp(0, REG_MAXSUB as c_int);
                for i in 0..n as usize {
                    let sp = sub.sub[i].sp;
                    let ep = sub.sub[i].ep;
                    if sp.is_null() || ep.is_null() {
                        caps.push(None);
                    } else {
                        caps.push(Some((sp.offset_from(base), ep.offset_from(base))));
                    }
                }
            }
            runs.push((rc, caps, sub.nsub));
        }
    }
    if use_x {
        l.js_regfreex(Some(my_alloc), 0xC0FFEE as *mut c_void, prog);
    } else {
        l.js_regfree(prog);
    }
    ExecOut {
        comp_err: None,
        runs,
    }
}

fn default_texts() -> Vec<(&'static str, c_int)> {
    let mut v = vec![];
    for t in [
        "",
        "a",
        "b",
        "ab",
        "abc",
        "aab",
        "aaa",
        "xxabcyy",
        "ABC",
        "AbC",
        "\n",
        "a\nb",
        "a\nb\nc",
        "hello world",
        "The quick brown fox",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
        "123-456-7890",
        "  leading",
        "trailing  ",
        "a.b.c",
        "[]{}()",
        "\u{e9}\u{e8}\u{e0}",
        "\u{4e2d}\u{6587}",
        "\u{1f600}",
        "tab\there",
        "0123456789",
        "_under_score_",
        "MiXeDcAsE",
        "aA",
        "z",
        "$^*+?",
        "\\",
        "()",
        "aaab",
        "abab",
        "abcabc",
        "foo bar foo",
        "\u{131}\u{130}",
        "\u{df}",
        "\u{3c3}\u{3c2}\u{3a3}",
    ] {
        v.push((t, 0));
        v.push((t, REG_NOTBOL));
    }
    v
}

fn patterns() -> Vec<&'static str> {
    vec![
        // literals
        "a", "abc", "", "z", "\u{e9}", "\u{4e2d}\u{6587}", "\u{1f600}",
        // dot / anchors / boundaries
        ".", "..", "^a", "a$", "^$", "^abc$", "\\ba\\b", "\\Bb\\B", "\\b", "\\B",
        // char classes (predefined)
        "\\d", "\\D", "\\s", "\\S", "\\w", "\\W",
        "\\d+", "\\D+", "\\s+", "\\S+", "\\w+", "\\W+",
        // classes in a set
        "[\\d]", "[\\D]", "[\\s]", "[\\S]", "[\\w]", "[\\W]",
        "[^\\d]", "[^\\D]", "[^\\w]",
        // explicit classes
        "[abc]", "[^abc]", "[a-z]", "[^a-z]", "[a-zA-Z0-9_]", "[]a]", "[a-]", "[-a]",
        "[\\]]", "[\\-]", "[\\n]", "[\\t]", "[\\x41]", "[\\u0041]", "[\\cA]",
        "[a-cx-z]", "[[]", "[.]", "[$]", "[^]a]", "[\\b]",
        // quantifiers
        "a*", "a+", "a?", "a{2}", "a{2,}", "a{2,4}", "a{0}", "a{0,0}", "a{1,1}",
        "a*?", "a+?", "a??", "a{2,4}?", "a{2,}?",
        ".*", ".+", ".*?", "^.*$",
        "(a)*", "(a)+", "(ab)*", "(a|b)*",
        // alternation
        "a|b", "a|b|c", "|a", "a|", "|", "abc|abd", "(a|)", "(|a)",
        // groups
        "(a)", "(a)(b)", "((a))", "(?:a)", "(?:a|b)", "(a)(b)(c)(d)(e)",
        "(a(b(c(d))))", "(?:(a)(?:(b)))",
        // lookahead
        "a(?=b)", "a(?!b)", "(?=a)", "(?!a)", "^(?=.*a)(?=.*b)", "(?=(a))",
        // back-references
        "(a)\\1", "(a)(b)\\2\\1", "(a*)\\1", "(?:(a)|b)\\1",
        // escapes
        "\\.", "\\*", "\\+", "\\?", "\\(", "\\)", "\\[", "\\]", "\\{", "\\}",
        "\\|", "\\^", "\\$", "\\\\", "\\/", "\\n", "\\r", "\\t", "\\f", "\\v",
        "\\0", "\\x41", "\\x4a", "\\u0041", "\\u4e2d", "\\cA", "\\cz",
        // combinations
        "^(\\w+)@(\\w+)\\.(\\w+)$", "([0-9]{3})-([0-9]{3})-([0-9]{4})",
        "(foo|bar)+", "a.*b.*c", "[^ ]+ [^ ]+", "(\\s*)(\\S+)(\\s*)",
        "(a+)+b", "(a|a)*b", "\\w+\\s\\w+",
        // 16 captures exactly (REG_MAXSUB) -- 15 groups + whole match
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)",
        // empty-loop cases
        "()*", "(?:)*", "(a?)*",
    ]
}

#[test]
fn t_regexp_pattern_matrix() {
    with_big_stack(body_t_regexp_pattern_matrix);
}

fn body_t_regexp_pattern_matrix() {
    let p = libs();
    let texts = default_texts();
    unsafe {
        for pat in patterns() {
            for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                for sub_null in [false, true] {
                    let a = compile_and_run(&p.c, pat, cflags, &texts, false, sub_null);
                    let b = compile_and_run(&p.rs, pat, cflags, &texts, false, sub_null);
                    assert_eq!(
                        a, b,
                        "regexp divergence pattern={pat:?} cflags={cflags} sub_null={sub_null}"
                    );
                }
            }
        }
    }
}

#[test]
fn t_regexp_custom_allocator() {
    with_big_stack(body_t_regexp_custom_allocator);
}

fn body_t_regexp_custom_allocator() {
    let p = libs();
    let texts = default_texts();
    unsafe {
        for pat in patterns() {
            for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                let a = compile_and_run(&p.c, pat, cflags, &texts, true, false);
                let b = compile_and_run(&p.rs, pat, cflags, &texts, true, false);
                assert_eq!(
                    a, b,
                    "regcompx divergence pattern={pat:?} cflags={cflags}"
                );
            }
        }
        assert!(ALLOC_CALLS > 0, "custom allocator was never called");
    }
}

/// ERRORS: every distinct `die()` message from regexp.c.
#[test]
fn t_regexp_compile_errors() {
    with_big_stack(body_t_regexp_compile_errors);
}

fn body_t_regexp_compile_errors() {
    let p = libs();
    let bad = [
        // unmatched brackets / parens
        "(", ")", "(a", "a)", "((a)", "(a))", "[", "[a", "[^", "[a-", "[]",
        // invalid quantifiers
        "*", "+", "?", "{2}", "*a", "+a", "?a", "a**", "a++", "a?+", "a{2}{3}",
        "a{", "a{}", "a{,}", "a{,2}", "a{2,1}", "a{1,2,3}",
        // numeric overflow in quantifiers (REPINF = 255)
        "a{256}", "a{255}", "a{1,256}", "a{99999}", "a{4294967296}",
        "a{2147483648}", "a{0,99999999999}",
        // invalid escapes / unterminated escapes
        "\\", "a\\", "\\x", "\\x4", "\\xg", "\\u", "\\u00", "\\u004", "\\uzzzz",
        "\\c", "a\\c",
        // invalid back-references
        "\\1", "\\2", "(a)\\2", "\\9", "(a)\\1\\2",
        // too many captures (REG_MAXSUB = 16 -> 15 groups max)
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)",
        "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)(n)(o)(p)(q)",
        // char-class range problems
        "[z-a]", "[b-a]", "[\\d-x]", "[a-\\d]",
        // empty-string infinite loop
        "(?:)*", "()+", "(|)*", "(a*)*", "(|a)*", "()*",
        // program too large (REG_MAXPROG = 32768)
        &"a".repeat(40000),
        &"(a)".repeat(20000),
        &"a{255}".repeat(300),
        // too many character classes (REG_MAXCLASS = 128)
        &"[a]".repeat(200),
        // too many ranges in one class (REG_MAXSPAN = 64)
        &format!("[{}]", "a-z".repeat(100)),
        // misc syntax
        "(?", "(?a)", "(?<a)", "(?P<n>a)", "a|*", "|*", "[a-b-c]",
    ];
    unsafe {
        for pat in bad {
            for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                let cp = cstr(pat);
                // with an errorp
                let mut ea: *const c_char = std::ptr::null();
                let mut eb: *const c_char = std::ptr::null();
                let pa = p.c.js_regcomp(cp.as_ptr(), cflags, &mut ea);
                let pb = p.rs.js_regcomp(cp.as_ptr(), cflags, &mut eb);
                assert_eq!(
                    pa.is_null(),
                    pb.is_null(),
                    "regcomp({pat:?}, {cflags}) success mismatch: \
                     C err={:?} RUST err={:?}",
                    if pa.is_null() { from_c(ea) } else { "ok".into() },
                    if pb.is_null() { from_c(eb) } else { "ok".into() }
                );
                if pa.is_null() {
                    assert_eq!(
                        from_c(ea),
                        from_c(eb),
                        "regcomp({pat:?}, {cflags}) error message"
                    );
                } else {
                    p.c.js_regfree(pa);
                    p.rs.js_regfree(pb);
                }
                // ERRORS: errorp == NULL must not crash and must return the
                // same success/failure
                let pa = p.c.js_regcomp(cp.as_ptr(), cflags, std::ptr::null_mut());
                let pb = p.rs.js_regcomp(cp.as_ptr(), cflags, std::ptr::null_mut());
                assert_eq!(
                    pa.is_null(),
                    pb.is_null(),
                    "regcomp({pat:?}, {cflags}) NULL errorp"
                );
                if !pa.is_null() {
                    p.c.js_regfree(pa);
                    p.rs.js_regfree(pb);
                }
            }
        }
    }
}

/// ERRORS: js_regfree(NULL) / js_regfreex(.., NULL) must be a no-op.
#[test]
fn t_regfree_null() {
    let p = libs();
    unsafe {
        p.c.js_regfree(std::ptr::null_mut());
        p.rs.js_regfree(std::ptr::null_mut());
        p.c.js_regfreex(
            Some(my_alloc),
            0xC0FFEE as *mut c_void,
            std::ptr::null_mut(),
        );
        p.rs.js_regfreex(
            Some(my_alloc),
            0xC0FFEE as *mut c_void,
            std::ptr::null_mut(),
        );
    }
}

/// Randomised pattern fuzzing: build patterns from a grammar of regexp
/// fragments and compare compile result + all match results.
#[test]
fn t_regexp_fuzz() {
    with_big_stack(body_t_regexp_fuzz);
}

fn body_t_regexp_fuzz() {
    let p = libs();
    let frags = [
        "a", "b", "c", ".", "\\d", "\\w", "\\s", "\\D", "\\W", "\\S", "[abc]", "[^abc]",
        "[a-z]", "[0-9]", "x", "^", "$", "\\b", "\\B", "(a)", "(?:b)", "(a|b)", "(?=a)",
        "(?!a)", "\\.", "\\\\", "\\n", "\\x41", "\\u0062", "[\\w-]", "[^\\s]",
    ];
    let quants = ["", "*", "+", "?", "{2}", "{1,3}", "{0,2}", "*?", "+?", "??", "{2,}"];
    let texts: Vec<(&str, c_int)> = default_texts();
    let mut rng = Rng::new(0x8E6E_3C71_0000_0001);
    unsafe {
        for _ in 0..4000 {
            let n = 1 + rng.below(5) as usize;
            let mut pat = String::new();
            for i in 0..n {
                if i > 0 && rng.below(5) == 0 {
                    pat.push('|');
                }
                pat.push_str(frags[rng.below(frags.len() as u32) as usize]);
                pat.push_str(quants[rng.below(quants.len() as u32) as usize]);
            }
            for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                let a = compile_and_run(&p.c, &pat, cflags, &texts, false, false);
                let b = compile_and_run(&p.rs, &pat, cflags, &texts, false, false);
                assert_eq!(a, b, "regexp fuzz divergence pattern={pat:?} cflags={cflags}");
            }
        }
    }
}

/// Randomised *text* fuzzing against a fixed set of patterns, including
/// non-UTF8 bytes and long inputs.
#[test]
fn t_regexp_text_fuzz() {
    with_big_stack(body_t_regexp_text_fuzz);
}

fn body_t_regexp_text_fuzz() {
    let p = libs();
    let pats = [
        ".*", "(a+)(b+)", "^(\\w+)\\s+(\\w+)$", "[^x]*x", "(a|b)+c", "\\b\\w+\\b",
        "(.)(.)(.)", "\\d{2,4}", "(?:ab)+", "x?y?z?", "(\\s*)(\\S+)", "a(?=b)b",
    ];
    let mut rng = Rng::new(0xFEED_BEEF);
    unsafe {
        for pat in pats {
            let mut texts: Vec<String> = vec![];
            for _ in 0..300 {
                texts.push(rng.ascii_string(24));
            }
            for _ in 0..300 {
                texts.push(rng.unicode_string(12));
            }
            for _ in 0..200 {
                texts.push(String::from_utf8_lossy(&rng.raw_bytes(16)).into_owned());
            }
            texts.push("a".repeat(500));
            texts.push("ab".repeat(300));
            texts.push(String::new());
            let tv: Vec<(&str, c_int)> = texts
                .iter()
                .flat_map(|t| [(t.as_str(), 0), (t.as_str(), REG_NOTBOL)])
                .collect();
            for cflags in [0, REG_ICASE, REG_NEWLINE] {
                let a = compile_and_run(&p.c, pat, cflags, &tv, false, false);
                let b = compile_and_run(&p.rs, pat, cflags, &tv, false, false);
                assert_eq!(a, b, "regexp text fuzz pattern={pat:?} cflags={cflags}");
            }
        }
    }
}

/// ERRORS: REG_MAXREC (4096) recursion limit -> regexec returns -1.
#[test]
fn t_regexp_recursion_limit() {
    with_big_stack(body_t_regexp_recursion_limit);
}

fn body_t_regexp_recursion_limit() {
    let p = libs();
    unsafe {
        // `match()` in regexp.c recurses once per repetition, so a LINEAR
        // pattern over a long subject walks the depth counter straight to
        // REG_MAXREC (4096) and returns -1. We deliberately avoid catastrophic
        // backtracking patterns like `(a*)*b`: those also hit the limit but
        // explore exponentially many paths first and take hours.
        let mut cases: Vec<(String, String)> = vec![];
        for n in [
            1, 2, 100, 1000, 4000, 4090, 4094, 4095, 4096, 4097, 4098, 4100, 5000, 9000,
        ] {
            let a = "a".repeat(n);
            cases.push(("a*b".into(), a.clone()));
            cases.push(("a+b".into(), a.clone()));
            cases.push(("(a)*b".into(), a.clone()));
            cases.push((".*x".into(), a.clone()));
            cases.push(("a*".into(), a.clone()));
            cases.push(("[a]*b".into(), a.clone()));
            cases.push(("(?:a)*b".into(), a.clone()));
            cases.push(("a*?b".into(), a.clone()));
        }
        // a modest exponential case, small enough to finish quickly
        cases.push(("(a*)*b".into(), "a".repeat(16)));
        cases.push(("(a|a)*b".into(), "a".repeat(15)));
        for (pat, text) in cases {
            let tv = vec![(text.as_str(), 0), (text.as_str(), REG_NOTBOL)];
            for cflags in [0, REG_ICASE] {
                let a = compile_and_run(&p.c, &pat, cflags, &tv, false, false);
                let b = compile_and_run(&p.rs, &pat, cflags, &tv, false, false);
                assert_eq!(a, b, "recursion limit pattern={pat:?} cflags={cflags}");
            }
        }
    }
}

/// Out-of-range eflags / cflags across the FFI boundary (C enums accept any int).
#[test]
fn t_regexp_out_of_range_flags() {
    with_big_stack(body_t_regexp_out_of_range_flags);
}

fn body_t_regexp_out_of_range_flags() {
    let p = libs();
    let texts: Vec<(&str, c_int)> = vec![("abc", 0), ("ABC", 0), ("a\nb", 0)];
    unsafe {
        for pat in ["a", "^a", "A", "(a)(b)", ".", "[a-z]+"] {
            for cflags in [
                -1, 0, 3, 4, 5, 7, 8, 16, 64, 255, 1024, i32::MAX, i32::MIN, 0x7fff_fffe,
            ] {
                let a = compile_and_run(&p.c, pat, cflags, &texts, false, false);
                let b = compile_and_run(&p.rs, pat, cflags, &texts, false, false);
                assert_eq!(a, b, "oor cflags={cflags} pattern={pat:?}");
            }
            for eflags in [-1, 0, 1, 2, 3, 5, 6, 7, 8, 255, i32::MAX, i32::MIN] {
                let tv: Vec<(&str, c_int)> =
                    vec![("abc", eflags), ("ABC", eflags), ("a\nb", eflags)];
                let a = compile_and_run(&p.c, pat, 0, &tv, false, false);
                let b = compile_and_run(&p.rs, pat, 0, &tv, false, false);
                assert_eq!(a, b, "oor eflags={eflags} pattern={pat:?}");
            }
        }
    }
}

/// ERRORS/CONFIGS: the exact `REG_MAXPROG` (32768) boundary. Patterns are
/// compiled at increasing sizes so the precise instruction count at which
/// `regcomp` starts returning `"program too large"` is compared, not merely
/// "both eventually fail".
#[test]
fn t_regexp_maxprog_boundary() {
    with_big_stack(body_t_regexp_maxprog_boundary);
}

fn body_t_regexp_maxprog_boundary() {
    let p = libs();
    unsafe {
        let compile = |l: &Lib, pat: &str| -> Option<String> {
            let cp = cstr(pat);
            let mut e: *const c_char = std::ptr::null();
            let prog = l.js_regcomp(cp.as_ptr(), 0, &mut e);
            if prog.is_null() {
                Some(from_c(e))
            } else {
                l.js_regfree(prog);
                None
            }
        };
        // shapes with different per-element instruction costs
        let shapes: [(&str, usize, usize, usize); 6] = [
            ("a", 1, 40000, 137),        // one literal rune per repeat
            ("(?:a)", 1, 40000, 149),    // same, wrapped
            ("[a]", 1, 20000, 71),       // a character class per repeat
            ("a?", 1, 20000, 79),        // a split per repeat
            ("(a)", 1, 12000, 53),       // capture pairs
            ("a|", 1, 12000, 61),        // alternation
        ];
        for (unit, lo, hi, step) in shapes {
            let mut n = lo;
            let mut boundary_c: Option<usize> = None;
            let mut boundary_r: Option<usize> = None;
            while n <= hi {
                let pat = unit.repeat(n);
                let a = compile(&p.c, &pat);
                let b = compile(&p.rs, &pat);
                assert_eq!(
                    a, b,
                    "REG_MAXPROG boundary: unit={unit:?} n={n} (pattern len {})",
                    pat.len()
                );
                if a.is_some() && boundary_c.is_none() {
                    boundary_c = Some(n);
                }
                if b.is_some() && boundary_r.is_none() {
                    boundary_r = Some(n);
                }
                n += step;
            }
            assert_eq!(
                boundary_c, boundary_r,
                "REG_MAXPROG first-failing size differs for unit={unit:?}"
            );
            assert!(
                boundary_c.is_some(),
                "unit={unit:?} never reached REG_MAXPROG up to n={hi}; \
                 the boundary is not being exercised"
            );
            // walk the exact boundary one repeat at a time
            let b0 = boundary_c.unwrap();
            for n in b0.saturating_sub(step + 2)..=(b0 + 2) {
                let pat = unit.repeat(n);
                assert_eq!(
                    compile(&p.c, &pat),
                    compile(&p.rs, &pat),
                    "REG_MAXPROG exact boundary: unit={unit:?} n={n}"
                );
            }
        }
    }
}
