//! Phase C — differential ERROR-PATH tests for ERRORS.md rows 500..593
//! (regexp.c, jsregexp.c, utf.c, jsdtoa.c).
//!
//! `phase_b_lowlevel.rs` fuzzes regcomp/regexec/chartorune/strtod on *valid*
//! inputs.  This file drives the explicit REJECTION paths: every distinct
//! `die()` message in regexp.c, every compile/exec limit, every throw in
//! jsregexp.c, every `goto bad` in `chartorune`, and the six `js_strtod`
//! guards.  Everything goes through the two `.so` exports only.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ utils */

fn short(s: &str) -> String {
    let n = s.chars().count();
    if n <= 40 {
        s.to_string()
    } else {
        format!("{}..<{} chars>", s.chars().take(40).collect::<String>(), n)
    }
}

/// Encode `s` as a JS double-quoted string literal. Non-ASCII is emitted
/// verbatim (mujs reads UTF-8 source), so a pattern that *contains* the two
/// characters `\` `u` keeps them.
fn jsq(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '\\' => o.push_str("\\\\"),
            '"' => o.push_str("\\\""),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `js_regcomp` in both libraries: rendered NULL-ness + error string.
fn regcomp_pair(pat: &str, cflags: c_int) -> (String, String) {
    let p = libs();
    unsafe {
        let cp = cs(pat);
        let mut ec: *const c_char = std::ptr::null();
        let mut er: *const c_char = std::ptr::null();
        let pc = (p.c.js_regcomp)(cp.as_ptr(), cflags, &mut ec);
        let pr = (p.r.js_regcomp)(cp.as_ptr(), cflags, &mut er);
        let cd = format!("null={} err={:?}", pc.is_null(), rs(ec));
        let rd = format!("null={} err={:?}", pr.is_null(), rs(er));
        if !pc.is_null() {
            (p.c.js_regfree)(pc);
        }
        if !pr.is_null() {
            (p.r.js_regfree)(pr);
        }
        (cd, rd)
    }
}

/// Both libraries must return NULL with exactly `msg`.
#[track_caller]
fn expect_regcomp_die(pat: &str, cflags: c_int, msg: &str) {
    let (cd, rd) = regcomp_pair(pat, cflags);
    same(
        &format!("js_regcomp {:?} flags={}", short(pat), cflags),
        &cd,
        &rd,
    );
    assert_eq!(
        cd,
        format!("null=true err={:?}", msg),
        "C js_regcomp({:?}) did not reject with {:?}",
        short(pat),
        msg
    );
}

/// The same rejection through `new RegExp(...)`: identical SyntaxError text.
#[track_caller]
fn expect_js_regexp_die(pat: &str, msg: &str) {
    let src = format!(
        "try {{ new RegExp({}); 'NO-THROW' }} catch (e) {{ e.name + ': ' + e.message }}",
        jsq(pat)
    );
    let p = libs();
    let c = p.c.eval(&src, 0);
    let r = p.r.eval(&src, 0);
    same(&format!("new RegExp {:?}", short(pat)), &c, &r);
    let want = format!("SyntaxError: regular expression: {}", msg);
    assert!(
        c.contains(&want),
        "C new RegExp({:?}) -> {:?}, expected to contain {:?}",
        short(pat),
        c,
        want
    );
}

/// Drive one pattern through BOTH entry points (low level + JS level).
#[track_caller]
fn expect_die_both(pat: &str, msg: &str) {
    for f in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
        expect_regcomp_die(pat, f, msg);
    }
    expect_js_regexp_die(pat, msg);
}

/// Run `f` on a thread with a large stack: the REG_MAXREC(4096) recursion in
/// `match()` needs ~1.3 MB of C stack, which is uncomfortably close to the
/// default test-thread stack size.
fn with_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    let h = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .expect("spawn big-stack thread");
    if let Err(e) = h.join() {
        std::panic::resume_unwind(e);
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

/// Compile in both libraries (must succeed identically), exec, compare rc and
/// captures, and assert the C return code is `want_rc`.
#[track_caller]
fn expect_regexec_rc(pat: &str, cflags: c_int, subj: &str, eflags: c_int, want_rc: c_int) {
    let p = libs();
    unsafe {
        let cp = cs(pat);
        let mut ec: *const c_char = std::ptr::null();
        let mut er: *const c_char = std::ptr::null();
        let pc = (p.c.js_regcomp)(cp.as_ptr(), cflags, &mut ec);
        let pr = (p.r.js_regcomp)(cp.as_ptr(), cflags, &mut er);
        same(
            &format!("regcomp-ok {:?} flags={}", short(pat), cflags),
            &format!("null={} err={:?}", pc.is_null(), rs(ec)),
            &format!("null={} err={:?}", pr.is_null(), rs(er)),
        );
        assert!(
            !pc.is_null(),
            "pattern {:?} unexpectedly failed to compile: {:?}",
            short(pat),
            rs(ec)
        );
        let sj = cs(subj);
        let mut mc = Resub::default();
        let mut mr = Resub::default();
        let rc = (p.c.js_regexec)(pc, sj.as_ptr(), &mut mc, eflags);
        let rr = (p.r.js_regexec)(pr, sj.as_ptr(), &mut mr, eflags);
        let label = format!(
            "js_regexec {:?} cflags={} subj={:?} eflags={}",
            short(pat),
            cflags,
            short(subj),
            eflags
        );
        same(
            &label,
            &format!("{} {}", rc, fmt_sub(&mc, sj.as_ptr(), rc)),
            &format!("{} {}", rr, fmt_sub(&mr, sj.as_ptr(), rr)),
        );
        /* the sub == NULL path uses the internal scratch Resub */
        let rc0 = (p.c.js_regexec)(pc, sj.as_ptr(), std::ptr::null_mut(), eflags);
        let rr0 = (p.r.js_regexec)(pr, sj.as_ptr(), std::ptr::null_mut(), eflags);
        same(&format!("{} nosub", label), &format!("{}", rc0), &format!("{}", rr0));
        (p.c.js_regfree)(pc);
        (p.r.js_regfree)(pr);
        assert_eq!(rc, want_rc, "C js_regexec rc for {}", label);
    }
}

/* ============================================================ regexp.c die */

/// Row 500 — `hex()` (regexp.c:101): non-hex digit in a `\x` / `\u` escape.
#[test]
fn regcomp_die_invalid_escape_sequence() {
    for pat in [
        r"a\xZZ", r"a\x0Z", r"a\xg1", r"a\u12G4", r"a\uZZZZ", r"[\x1Z]", r"[\u000Z]",
    ] {
        expect_die_both(pat, "invalid escape sequence");
    }
}

/// Rows 501 + 521 — `dec()` (regexp.c:108) and `parserep` (regexp.c:598).
#[test]
fn regcomp_die_invalid_quantifier() {
    for pat in ["a{b}", "a{,2}", "a{1,x}", "a{-1}", "a{1 }", "a{}"] {
        expect_die_both(pat, "invalid quantifier");
    }
    /* row 521: {M,N} with N < M */
    for pat in ["a{2,1}", "a{10,2}", "(?:ab){9,3}"] {
        expect_die_both(pat, "invalid quantifier");
    }
}

/// Rows 502..505 — `nextrune` (regexp.c:128/138/143/153).
#[test]
fn regcomp_die_unterminated_escape_sequence() {
    let cases = [
        r"a\",    /* 502: lone trailing backslash */
        r"\",     /* 502 */
        r"[a\",   /* 502 inside a class */
        r"a\c",   /* 503: \c with nothing after */
        r"\c",    /* 503 */
        r"a\x",   /* 504: no hex digits */
        r"a\x4",  /* 504: only one hex digit */
        r"a\u",   /* 505 */
        r"a\u1",  /* 505 */
        r"a\u12", /* 505 */
        r"a\u123",/* 505 */
    ];
    for pat in cases {
        expect_die_both(pat, "unterminated escape sequence");
    }
}

/// Row 506 — `nextrune` (regexp.c:170): identity escape of a letter / `_`.
#[test]
fn regcomp_die_invalid_escape_character() {
    for pat in [r"a\y", r"a\_", r"a\q", r"a\A", r"a\Z", r"[\y]", "a\\\u{e9}"] {
        expect_die_both(pat, "invalid escape character");
    }
}

/// Rows 507 + 508 — `lexcount` (regexp.c:186/200): repeat count reaches REPINF.
#[test]
fn regcomp_die_numeric_overflow() {
    /* row 507: min count */
    for pat in [
        "a{255}",
        "a{256}",
        "a{999}",
        "a{2147483647}",
        "x{2147483647}",
        "a{99999999999}",
        "a{300,1}",
    ] {
        expect_die_both(pat, "numeric overflow");
    }
    /* row 508: max count */
    for pat in ["a{1,255}", "a{1,256}", "a{0,4000}", "a{2,2147483647}"] {
        expect_die_both(pat, "numeric overflow");
    }
    /* the boundary: 254 is still accepted by both */
    let (c, r) = regcomp_pair("a{254}", 0);
    same("js_regcomp a{254}", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"");
    let (c, r) = regcomp_pair("a{1,254}", 0);
    same("js_regcomp a{1,254}", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"");
}

/// Row 509 — `newcclass` (regexp.c:213): more than REG_MAXCLASS(128) classes.
#[test]
fn regcomp_die_too_many_character_classes() {
    /* 128 classes is fine, 129 dies */
    let ok = "[a]".repeat(128);
    let (c, r) = regcomp_pair(&ok, 0);
    same("js_regcomp 128 classes", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"", "128 classes should compile");
    for n in [129usize, 130, 200] {
        expect_die_both(&"[a]".repeat(n), "too many character classes");
    }
    /* negated classes count too */
    expect_die_both(&"[^a]".repeat(129), "too many character classes");
}

/// Row 510 — `addrange` (regexp.c:224): reversed class range.
#[test]
fn regcomp_die_invalid_character_class_range() {
    for pat in ["[z-a]", "[9-0]", r"[\uFFFF-\u0001]", "[b-a]", "[\u{4e2d}-a]", r"[\u0101-\u0001]"] {
        expect_die_both(pat, "invalid character class range");
    }
    /* `lexclass` tests escaped class members with strchr("DSWdsw", g->yychar),
     * which TRUNCATES the rune to a char: U+2000 (low byte 0x00) matches the
     * terminating NUL and U+2044 (low byte 'D') matches 'D'. Those members are
     * therefore swallowed / expanded instead of becoming a plain range, so a
     * reversed range built from them does NOT die. Both libraries must make the
     * same (surprising) choice. */
    for pat in [
        r"[\u2000-\u0001]",
        r"[\u2000]",
        r"[\u2044]",
        r"[\u2064]",
        r"[\u2053]",
        r"[\u2057]",
        r"[\u2073]",
        r"[\u2077]",
        r"[\u2000-\u2100]",
        r"[\u2044-\u2001]",
    ] {
        for f in [0, REG_ICASE] {
            let (c, r) = regcomp_pair(pat, f);
            same(&format!("js_regcomp strchr-trunc {:?} flags={}", pat, f), &c, &r);
        }
        diff_eval(
            "strchr-trunc class",
            &format!(
                "try {{ String(new RegExp({}).test('a')) }} catch (e) {{ e.name + ': ' + e.message }}",
                jsq(pat)
            ),
            0,
        );
    }
}

/// Row 511 — `addrange` (regexp.c:253): more than 31 non-mergeable spans.
#[test]
fn regcomp_die_too_many_character_class_ranges() {
    /* singletons spaced 4 apart never merge (merging needs b >= p[0]-1) */
    fn spans(n: usize) -> String {
        let mut s = String::from("[");
        for i in 0..n {
            s.push_str(&format!("\\u{:04X}", 0x2001 + i * 4));
        }
        s.push(']');
        s
    }
    /* 31 spans still fit (end+2 >= spans+64 dies on the 32nd) */
    let (c, r) = regcomp_pair(&spans(31), 0);
    same("js_regcomp 31 spans", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"", "31 spans should compile");
    for n in [32usize, 40, 64] {
        expect_die_both(&spans(n), "too many character class ranges");
    }
}

/// Row 512 — `lexclass` (regexp.c:322): unterminated `[`.
#[test]
fn regcomp_die_unterminated_character_class() {
    for pat in ["[a", "[a-", "[", "[^", "[^a", "[a-z", r"[\d", "x[abc"] {
        expect_die_both(pat, "unterminated character class");
    }
}

/// Row 513 — `newrep` (regexp.c:493): unbounded repeat of an empty-matching atom.
#[test]
fn regcomp_die_infinite_loop_empty_string() {
    for pat in [
        "()*",
        "()+",
        "(?:){2,}",
        "(?:a*)*",
        "(a*)*",
        "(a?)+",
        "(|a)*",
        "((?:))+",
        "(?:a|)*",
        "(?:){0,}",
    ] {
        expect_die_both(pat, "infinite loop matching the empty string");
    }
}

/// Row 514 — `parseatom` (regexp.c:541): invalid back-reference.
#[test]
fn regcomp_die_invalid_back_reference() {
    for pat in [r"\1", r"(a)\2", r"\9", r"(a)(b)\3", r"(\1)", r"\10", r"\99"] {
        expect_die_both(pat, "invalid back-reference");
    }
    /* `\0` is NOT a back-reference: lex() turns it into a literal NUL (L_CHAR) */
    let (c, r) = regcomp_pair(r"\0", 0);
    same(r"js_regcomp \0", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"", r"\0 must compile as a NUL char");
    /* forward / self reference to a group that is not closed yet */
    expect_die_both(r"(a\1)", "invalid back-reference");
}

/// Row 515 — `parseatom` (regexp.c:552): `nsub == REG_MAXSUB(16)`.
#[test]
fn regcomp_die_too_many_captures() {
    /* nsub starts at 1, so 15 groups is the maximum */
    let ok = "()".repeat(15);
    let (c, r) = regcomp_pair(&ok, 0);
    same("js_regcomp 15 captures", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"", "15 captures should compile");
    for n in [16usize, 17, 20, 40] {
        expect_die_both(&"()".repeat(n), "too many captures");
    }
    expect_die_both(&"(a)".repeat(17), "too many captures");
    /* nested groups hit the same limit */
    let nested = format!("{}{}", "(".repeat(17), ")".repeat(17));
    expect_die_both(&nested, "too many captures");
}

/// Rows 516..519 — `parseatom` (regexp.c:557/563/570/577): unmatched '('.
#[test]
fn regcomp_die_unmatched_open_paren() {
    let cases = [
        "(a",    /* 516 capturing */
        "(",     /* 516 */
        "(?:a",  /* 517 non-capturing */
        "(?:",   /* 517 */
        "(?=a",  /* 518 positive lookahead */
        "(?=",   /* 518 */
        "(?!a",  /* 519 negative lookahead */
        "(?!",   /* 519 */
        "((a)",  /* nested */
        "(?:(a)",
    ] ;
    for pat in cases {
        expect_die_both(pat, "unmatched '('");
    }
}

/// Row 520 — `parseatom` (regexp.c:580): token that is not a valid atom.
#[test]
fn regcomp_die_syntax_error() {
    for pat in ["*a", "+", "?", "{2}", "*", "a|*", "(*)", "{1,2}", "(?:*)", "a|{3}"] {
        expect_die_both(pat, "syntax error");
    }
}

/// Row 527 — `regcompx` (regexp.c:940): leftover ')'.
#[test]
fn regcomp_die_unmatched_close_paren() {
    for pat in ["a)", "(a))", ")", ")a", "(?:a))", "a|)"] {
        expect_die_both(pat, "unmatched ')'");
    }
}

/// Row 522 — `count` (regexp.c:661): parse-tree recursion > REG_MAXREC(4096).
#[test]
fn regcomp_die_stack_overflow() {
    /* a right-leaning P_CAT chain: one node per character */
    let ok = "a".repeat(4000);
    let (c, r) = regcomp_pair(&ok, 0);
    same("js_regcomp 4000 a", &c, &r);
    assert_eq!(c, "null=false err=\"<null>\"", "4000 atoms should compile");
    for n in [4100usize, 5000, 8000] {
        expect_die_both(&"a".repeat(n), "stack overflow");
    }
    /* alternation nests just as deep */
    expect_die_both(&"a|".repeat(4100).trim_end_matches('|').to_string(), "stack overflow");
}

/// Rows 523 + 525 + 529 — the three "program too large" sites.
#[test]
fn regcomp_die_program_too_large() {
    /* row 523: per-P_REP instruction count overflows REG_MAXPROG */
    for pat in ["(?:a{254}){254}", "(?:a{200}){200}", "(?:(?:a{100}){100}){100}"] {
        expect_die_both(pat, "program too large");
    }
    /* row 525: strlen(pattern)*2 > REG_MAXPROG, i.e. > 16384 bytes */
    let ok = "a".repeat(16384);
    let (c, r) = regcomp_pair(&ok, 0);
    same("js_regcomp 16384 a", &c, &r);
    /* 16384*2 == REG_MAXPROG is not > REG_MAXPROG, but count() then trips
     * REG_MAXREC first; either way both libraries must agree. */
    let (c2, r2) = regcomp_pair(&"a".repeat(16385), 0);
    same("js_regcomp 16385 a", &c2, &r2);
    assert_eq!(
        c2, "null=true err=\"program too large\"",
        "16385-byte pattern must be rejected by the strlen*2 guard"
    );
    for n in [16385usize, 20000, 40000] {
        expect_regcomp_die(&"a".repeat(n), 0, "program too large");
    }
    expect_js_regexp_die(&"a".repeat(16385), "program too large");

    /* row 529: total 6+count() > REG_MAXPROG without any single node overflowing */
    expect_die_both(&"a{99}".repeat(400), "program too large");
    expect_die_both(&"a{50}".repeat(700), "program too large");
}

/// Rows 524, 526, 530, 531 — the four `alloc(...) == NULL` sites of
/// `regcompx`, driven through `js_regcompx` with a counting allocator that
/// fails exactly on the Nth allocation.
#[test]
fn regcompx_allocation_failure_paths() {
    #[repr(C)]
    struct Ctx {
        n: c_int,
        fail_at: c_int,
    }
    unsafe extern "C" fn alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
        extern "C" {
            fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
            fn free(p: *mut c_void);
        }
        if n == 0 {
            free(p);
            return std::ptr::null_mut();
        }
        let cx = &mut *(ctx as *mut Ctx);
        cx.n += 1;
        if cx.n == cx.fail_at {
            return std::ptr::null_mut();
        }
        realloc(p, n as usize)
    }

    let p = libs();
    /* allocation order in regcompx:
     *   1 Reprog                      -> "cannot allocate regular expression"
     *   2 Renode parse list           -> "... parse list"
     *   3 Reinst instruction list     -> "... instruction list"
     *   4 Reclass list (if a class)   -> "... character class list"        */
    let expect: [(c_int, &str); 4] = [
        (1, "cannot allocate regular expression"),
        (2, "cannot allocate regular expression parse list"),
        (3, "cannot allocate regular expression instruction list"),
        (4, "cannot allocate regular expression character class list"),
    ];
    unsafe {
        for pat in ["[a]", "[a-z]+(b)", "x[0-9]{2,3}"] {
            let cp = cs(pat);
            for (fail_at, msg) in expect {
                for flags in [0, REG_ICASE] {
                    let mut cctx = Ctx { n: 0, fail_at };
                    let mut rctx = Ctx { n: 0, fail_at };
                    let mut ec: *const c_char = std::ptr::null();
                    let mut er: *const c_char = std::ptr::null();
                    let pc = (p.c.js_regcompx)(
                        Some(alloc),
                        &mut cctx as *mut Ctx as *mut c_void,
                        cp.as_ptr(),
                        flags,
                        &mut ec,
                    );
                    let pr = (p.r.js_regcompx)(
                        Some(alloc),
                        &mut rctx as *mut Ctx as *mut c_void,
                        cp.as_ptr(),
                        flags,
                        &mut er,
                    );
                    let cd = format!("null={} err={:?} nalloc={}", pc.is_null(), rs(ec), cctx.n);
                    let rd = format!("null={} err={:?} nalloc={}", pr.is_null(), rs(er), rctx.n);
                    if !pc.is_null() {
                        (p.c.js_regfreex)(Some(alloc), &mut cctx as *mut Ctx as *mut c_void, pc);
                    }
                    if !pr.is_null() {
                        (p.r.js_regfreex)(Some(alloc), &mut rctx as *mut Ctx as *mut c_void, pr);
                    }
                    same(
                        &format!("js_regcompx {:?} flags={} fail_at={}", pat, flags, fail_at),
                        &cd,
                        &rd,
                    );
                    assert!(
                        cd.starts_with(&format!("null=true err={:?}", msg)),
                        "C js_regcompx({:?}, fail_at={}) -> {}, expected {:?}",
                        pat,
                        fail_at,
                        cd,
                        msg
                    );
                }
            }
            /* a pattern with no class never performs the 4th allocation */
            let np = cs("a+b");
            let mut cctx = Ctx { n: 0, fail_at: 4 };
            let mut rctx = Ctx { n: 0, fail_at: 4 };
            let mut ec: *const c_char = std::ptr::null();
            let mut er: *const c_char = std::ptr::null();
            let pc = (p.c.js_regcompx)(
                Some(alloc),
                &mut cctx as *mut Ctx as *mut c_void,
                np.as_ptr(),
                0,
                &mut ec,
            );
            let pr = (p.r.js_regcompx)(
                Some(alloc),
                &mut rctx as *mut Ctx as *mut c_void,
                np.as_ptr(),
                0,
                &mut er,
            );
            let cd = format!("null={} err={:?} nalloc={}", pc.is_null(), rs(ec), cctx.n);
            let rd = format!("null={} err={:?} nalloc={}", pr.is_null(), rs(er), rctx.n);
            if !pc.is_null() {
                (p.c.js_regfreex)(Some(alloc), &mut cctx as *mut Ctx as *mut c_void, pc);
            }
            if !pr.is_null() {
                (p.r.js_regfreex)(Some(alloc), &mut rctx as *mut Ctx as *mut c_void, pr);
            }
            same("js_regcompx no-class fail_at=4", &cd, &rd);
            assert_eq!(cd, "null=false err=\"<null>\" nalloc=3");
        }
    }
}

/// Row 528 — `regcompx` (regexp.c:942): `g.lookahead != EOF` after `parsealt`.
/// Documented as unreachable (parsecat/parsealt only stop on EOF, `|` or `)`);
/// this sweeps every token that could plausibly be left over and asserts the
/// two libraries agree (and that the reachable sibling, row 527, wins).
#[test]
fn regcomp_defensive_syntax_error_after_parsealt() {
    let pats = [
        "a)", "(a))", ")))", "a|b)", "(?:a)b)", "[a])", "a{1,2})", r"\b)", "()|)", "a)|b",
    ];
    for pat in pats {
        for f in [0, REG_ICASE, REG_NEWLINE] {
            let (c, r) = regcomp_pair(pat, f);
            same(&format!("js_regcomp leftover {:?} flags={}", pat, f), &c, &r);
        }
        /* every reachable leftover token is a ')' -> row 527, never row 528 */
        let (c, _) = regcomp_pair(pat, 0);
        assert_eq!(
            c, "null=true err=\"unmatched ')'\"",
            "unexpected leftover-token error for {:?}",
            pat
        );
    }
}

/* ================================================== regexp.c match() paths */

/// Row 532 — `match` (regexp.c:1075): backtracking recursion > REG_MAXREC.
#[test]
fn regexec_recursion_limit_returns_minus_one() {
    with_big_stack(|| {
        /* `a*` recurses once per repetition */
        expect_regexec_rc("a*", 0, &"a".repeat(4999), 0, -1);
        expect_regexec_rc("a*", 0, &"a".repeat(6000), 0, -1);
        expect_regexec_rc("(a)*", 0, &"a".repeat(4999), 0, -1);
        expect_regexec_rc("a+", 0, &"a".repeat(5000), 0, -1);
        expect_regexec_rc(".*", 0, &"x".repeat(5000), 0, -1);
        /* the same subject just under the limit still matches in both */
        expect_regexec_rc("a*", 0, &"a".repeat(1000), 0, 0);
        expect_regexec_rc("a*", 0, &"a".repeat(4000), 0, 0);
        /* catastrophic backtracking: (a|aa)+$ against a long run of 'a'.
         * The greedy 1-char-per-iteration path reaches REG_MAXREC after
         * ~2048 repetitions, so -1 propagates out before the exponential
         * search can start. Shorter subjects would simply never terminate. */
        for n in [2500usize, 4000, 6000] {
            let p = libs();
            unsafe {
                let cp = cs("(a|aa)+$");
                let mut ec: *const c_char = std::ptr::null();
                let mut er: *const c_char = std::ptr::null();
                let pc = (p.c.js_regcomp)(cp.as_ptr(), 0, &mut ec);
                let pr = (p.r.js_regcomp)(cp.as_ptr(), 0, &mut er);
                assert!(!pc.is_null() && !pr.is_null());
                let subj = cs(&format!("{}b", "a".repeat(n)));
                let mut mc = Resub::default();
                let mut mr = Resub::default();
                let rc = (p.c.js_regexec)(pc, subj.as_ptr(), &mut mc, 0);
                let rr = (p.r.js_regexec)(pr, subj.as_ptr(), &mut mr, 0);
                same(
                    &format!("js_regexec (a|aa)+$ over {} a's + b", n),
                    &format!("{} {}", rc, fmt_sub(&mc, subj.as_ptr(), rc)),
                    &format!("{} {}", rr, fmt_sub(&mr, subj.as_ptr(), rr)),
                );
                (p.c.js_regfree)(pc);
                (p.r.js_regfree)(pr);
                assert_eq!(rc, -1, "(a|aa)+$ over {} a's", n);
            }
        }
    });
}

/// Rows 533..549 — every no-match (`return 1`) path of `match()`.
#[test]
fn regexec_no_match_paths() {
    /* (row, pattern, cflags, subject, eflags) */
    let cases: [(u32, &str, c_int, &str, c_int); 17] = [
        (533, "b", 0, "aaa", 0),                    /* I_ANYNL: scan off the end */
        (534, "a.", 0, "a", 0),                     /* I_ANY at end of subject */
        (535, "a.b", 0, "a\nb", 0),                 /* I_ANY on a newline rune */
        (536, "ab", 0, "a", 0),                     /* I_CHAR at end of subject */
        (537, "ab", 0, "ax", 0),                    /* I_CHAR mismatch */
        (538, "a[b]", 0, "a", 0),                   /* I_CCLASS at end */
        (539, "a[b]", REG_ICASE, "aZ", 0),          /* I_CCLASS incclasscanon */
        (540, "a[b]", 0, "az", 0),                  /* I_CCLASS incclass */
        (541, "a[^b]", 0, "a", 0),                  /* I_NCCLASS at end */
        (542, "a[^b]", REG_ICASE, "aB", 0),         /* I_NCCLASS canon hit */
        (543, "a[^b]", 0, "ab", 0),                 /* I_NCCLASS hit */
        (544, r"(a)\1", REG_ICASE, "ab", 0),        /* I_REF strncmpcanon */
        (545, r"(a)\1", 0, "ab", 0),                /* I_REF strncmp */
        (546, "^a", 0, "a", REG_NOTBOL),            /* I_BOL with REG_NOTBOL */
        (547, "a$", 0, "ab", 0),                    /* I_EOL */
        (548, r"a\b", 0, "ab", 0),                  /* I_WORD */
        (549, r"a\B", 0, "a ", 0),                  /* I_NWORD */
    ];
    for (row, pat, cflags, subj, eflags) in cases {
        assert!(row >= 533 && row <= 549);
        expect_regexec_rc(pat, cflags, subj, eflags, 1);
    }
    /* a few extra newline runes for the I_ANY guard (0xA 0xD U+2028 U+2029) */
    for nl in ["\n", "\r", "\u{2028}", "\u{2029}"] {
        expect_regexec_rc("a.b", 0, &format!("a{}b", nl), 0, 1);
    }
    /* REG_NEWLINE variants of I_BOL / I_EOL that still fail */
    expect_regexec_rc("^b", REG_NEWLINE, "a\nc", 0, 1);
    expect_regexec_rc("b$", REG_NEWLINE, "a\nc", 0, 1);
    expect_regexec_rc("^a", 0, "ba", 0, 1);
    /* the same paths at the JS level */
    for src in [
        "String(/b/.exec('aaa'))",
        "String(/a./.exec('a'))",
        "String(/a.b/.exec('a\\nb'))",
        "/ab/.test('ax')",
        "/a[b]/i.test('aZ')",
        "/a[^b]/i.test('aB')",
        "String(/(a)\\1/.exec('ab'))",
        "/a$/.test('ab')",
        "/a\\b/.test('ab')",
        "/a\\B/.test('a ')",
        "'aaa'.match(/b/)",
        "'aaa'.search(/b/)",
        "'aaa'.replace(/b/,'x')",
        "'aaa'.split(/b/).length",
    ] {
        diff_eval("no-match js", src, 0);
    }
}

/// Rows 551 + 552 — `strncmpcanon` (regexp.c:1056/1057): the two early-exit
/// guards of the REG_ICASE back-reference comparison.
#[test]
fn regexec_strncmpcanon_short_operands() {
    /* row 551: the subject runs out before `n` runes are consumed */
    expect_regexec_rc(r"(é)\1", REG_ICASE, "\u{e9}\u{e9}", 0, 1);
    expect_regexec_rc(r"(ab)\1", REG_ICASE, "aba", 0, 1);
    /* row 552: the captured text runs out first. `n` is a BYTE count but the
     * loop consumes RUNES, so a multi-byte capture that ends at the end of the
     * subject terminates early while the compared text still has bytes left. */
    expect_regexec_rc(
        r"(?=.*(éé)$)\1",
        REG_ICASE,
        "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}",
        0,
        1,
    );
    expect_regexec_rc(
        r"(?=.*(中中)$)\1",
        REG_ICASE,
        "\u{4e2d}\u{4e2d}\u{4e2d}\u{4e2d}",
        0,
        1,
    );
    /* case-sensitive counterpart (row 545): `strncmp` compares BYTES, so it has
     * no early-exit quirk — "éé" DOES match /(é)\1/ without REG_ICASE. */
    expect_regexec_rc(r"(é)\1", 0, "\u{e9}\u{e9}", 0, 0);
    expect_regexec_rc(r"(é)\1", 0, "\u{e9}a", 0, 1);
    expect_regexec_rc(r"(中)\1", 0, "\u{4e2d}b", 0, 1);
    /* multi-byte captures compared under REG_ICASE against a shorter tail */
    expect_regexec_rc(r"(éé)\1", REG_ICASE, "\u{e9}\u{e9}\u{e9}", 0, 1);
    expect_regexec_rc(r"(ééé)\1", REG_ICASE, "\u{e9}\u{e9}\u{e9}\u{e9}", 0, 1);
}

/// Row 550 — `match` default case (regexp.c:1222): an opcode that is not any
/// I_*. Unreachable through `regcompx` (a corrupted/foreign Reprog would be
/// needed), so this instead sweeps patterns that emit every opcode and asserts
/// both libraries take the same branch everywhere.
#[test]
fn regexec_every_opcode_agrees() {
    let pats = [
        "a", ".", "[a-z]", "[^a-z]", "^a", "a$", r"\ba", r"\Ba", "(a)", "(?:a)", "(?=a)", "(?!a)",
        r"(a)\1", "a|b", "a*", "a+", "a?", "a{2,3}", "(a)(b)(c)", ".*",
    ];
    let subjects = ["", "a", "b", "ab", "abc", "aa", "A", "a\nb", " a ", "\u{e9}"];
    let p = libs();
    unsafe {
        for pat in pats {
            for cflags in [0, REG_ICASE, REG_NEWLINE, REG_ICASE | REG_NEWLINE] {
                let cp = cs(pat);
                let mut ec: *const c_char = std::ptr::null();
                let mut er: *const c_char = std::ptr::null();
                let pc = (p.c.js_regcomp)(cp.as_ptr(), cflags, &mut ec);
                let pr = (p.r.js_regcomp)(cp.as_ptr(), cflags, &mut er);
                same(
                    &format!("opcode sweep comp {:?} {}", pat, cflags),
                    &format!("{} {:?}", pc.is_null(), rs(ec)),
                    &format!("{} {:?}", pr.is_null(), rs(er)),
                );
                if pc.is_null() {
                    continue;
                }
                for subj in subjects {
                    for eflags in [0, REG_NOTBOL] {
                        let sj = cs(subj);
                        let mut mc = Resub::default();
                        let mut mr = Resub::default();
                        let rc = (p.c.js_regexec)(pc, sj.as_ptr(), &mut mc, eflags);
                        let rr = (p.r.js_regexec)(pr, sj.as_ptr(), &mut mr, eflags);
                        same(
                            &format!("opcode sweep {:?} {} {:?} {}", pat, cflags, subj, eflags),
                            &format!("{} {}", rc, fmt_sub(&mc, sj.as_ptr(), rc)),
                            &format!("{} {}", rr, fmt_sub(&mr, sj.as_ptr(), rr)),
                        );
                        assert!(rc == 0 || rc == 1, "unexpected rc {} for {:?}", rc, pat);
                    }
                }
                (p.c.js_regfree)(pc);
                (p.r.js_regfree)(pr);
            }
        }
    }
}

/* ========================================================== jsregexp.c */

/// Row 553 — `js_newregexpx` (jsregexp.c:37-38): every regcomp failure becomes
/// SyntaxError "regular expression: %s", driven through the C API entry point
/// `js_newregexp` (inside js_pcall, so the throw is caught).
#[test]
fn newregexp_bad_pattern_throws_syntaxerror() {
    let pats = [
        "a{2,1}", "[a", "(", ")", r"\1", "()*", "[z-a]", r"a\y", r"a\xZZ", "a{255}", "*a",
    ];
    fn act(a: &Api, J: JS) {
        unsafe {
            let pat = ps(0);
            (a.js_newregexp)(J, pat.as_ptr(), pic(0));
            emit("NO-THROW");
            emit(&format!("re={}", repr_at(a, J, -1)));
        }
    }
    for pat in pats {
        for flags in [0i64, JS_REGEXP_G as i64, JS_REGEXP_I as i64, JS_REGEXP_M as i64] {
            set_ps(0, pat);
            set_pi(0, flags);
            diff_native(&format!("js_newregexp bad {:?} flags={}", pat, flags), act, 0);
        }
        /* the same rejection through the JS constructor */
        diff_eval(
            "new RegExp bad",
            &format!(
                "try {{ new RegExp({}) }} catch (e) {{ e.name + ': ' + e.message }}",
                jsq(pat)
            ),
            0,
        );
        diff_eval(
            "RegExp() bad",
            &format!(
                "try {{ RegExp({}) }} catch (e) {{ e.name + ': ' + e.message }}",
                jsq(pat)
            ),
            0,
        );
    }
}

/// Out-of-range / negative flag ints handed to `js_newregexp` across the FFI
/// boundary (only JS_REGEXP_G/I/M are meaningful; everything else is ignored,
/// but the stored `flags` word is observable through `toString`).
#[test]
fn newregexp_out_of_range_flag_ints() {
    fn act(a: &Api, J: JS) {
        unsafe {
            let pat = ps(0);
            (a.js_newregexp)(J, pat.as_ptr(), pic(0));
            emit(&format!(
                "isregexp={} typeof={}",
                (a.js_isregexp)(J, -1),
                rs((a.js_typeof)(J, -1))
            ));
            emit(&format!("toregexp_nonnull={}", !(a.js_toregexp)(J, -1).is_null()));
            for k in ["source", "global", "ignoreCase", "multiline", "lastIndex"] {
                (a.js_getproperty)(J, -1, cs(k).as_ptr());
                emit(&format!("{}={}", k, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
            emit(&format!("str={:?}", str_at(a, J, -1)));
            emit(&format!("repr={:?}", repr_at(a, J, -1)));
            /* exercise exec/test twice to see whether the /g bit was taken */
            (a.js_copy)(J, -1);
            (a.js_setglobal)(J, cs("RE").as_ptr());
            let src = cs("var s='abab'; [String(RE.exec(s)),RE.lastIndex,RE.test(s),RE.lastIndex,String(s.match(RE)),s.replace(RE,'#')].join('/')");
            let nm = cs("f.js");
            if (a.js_ploadstring)(J, nm.as_ptr(), src.as_ptr()) == 0 {
                (a.js_pushundefined)(J);
                let rc = (a.js_pcall)(J, 0);
                emit(&format!("drive={} {}", rc, repr_at(a, J, -1)));
                (a.js_pop)(J, 1);
            }
        }
    }
    for pat in ["a(b)", "^a$", "[a-z]+"] {
        for flags in [
            0i64,
            1,
            2,
            4,
            7,
            8,
            9,
            255,
            -1,
            -2,
            0x7fff_ffff,
            i32::MIN as i64,
        ] {
            set_ps(0, pat);
            set_pi(0, flags);
            diff_native(&format!("js_newregexp {:?} flags={}", pat, flags), act, 0);
        }
    }
}

/// Rows 554 + 555 + 565 — the three `js_malloc` failure sites of jsregexp.c
/// (`escaperegexp`, the `js_strdup` clone path, `Rp_toString`), forced through
/// `js_setlimit`'s memory limit. The limit is restored after the (caught) throw.
#[test]
fn regexp_out_of_memory_paths() {
    /* row 554: escaperegexp on a '/'-heavy source string */
    fn act_escape(a: &Api, J: JS) {
        unsafe {
            (a.js_getglobal)(J, cs("RegExp").as_ptr());
            let pat = ps(0);
            (a.js_pushstring)(J, pat.as_ptr());
            (a.js_setlimit)(J, 0, pic(0));
            let rc = (a.js_pconstruct)(J, 1);
            (a.js_setlimit)(J, 0, 0);
            emit(&format!("rc={} v={:?}", rc, str_at(a, J, -1)));
            (a.js_pop)(J, 1);
        }
    }
    /* row 555: js_strdup on the clone path (new RegExp(re)) */
    fn act_clone(a: &Api, J: JS) {
        unsafe {
            let pat = ps(0);
            (a.js_newregexp)(J, pat.as_ptr(), 0);
            (a.js_getglobal)(J, cs("RegExp").as_ptr());
            (a.js_copy)(J, -2);
            (a.js_setlimit)(J, 0, pic(0));
            let rc = (a.js_pconstruct)(J, 1);
            (a.js_setlimit)(J, 0, 0);
            emit(&format!("rc={} v={:?}", rc, str_at(a, J, -1)));
            (a.js_pop)(J, 2);
        }
    }
    /* row 565: Rp_toString's js_malloc(strlen(source)+6) */
    fn act_tostring(a: &Api, J: JS) {
        unsafe {
            let pat = ps(0);
            (a.js_newregexp)(J, pat.as_ptr(), JS_REGEXP_G | JS_REGEXP_I | JS_REGEXP_M);
            (a.js_getproperty)(J, -1, cs("toString").as_ptr());
            (a.js_copy)(J, -2);
            (a.js_setlimit)(J, 0, pic(0));
            let rc = (a.js_pcall)(J, 0);
            (a.js_setlimit)(J, 0, 0);
            emit(&format!("rc={} v={:?}", rc, str_at(a, J, -1)));
            (a.js_pop)(J, 2);
        }
    }
    let pat = "a/b".repeat(300); /* 900 bytes, 300 '/' -> escaped copy is 1201 */
    for lim in [4i64, 64, 512, 1200, 1500, 100000] {
        set_ps(0, &pat);
        set_pi(0, lim);
        diff_native(&format!("escaperegexp oom lim={}", lim), act_escape, 0);
        set_ps(0, &pat);
        set_pi(0, lim);
        diff_native(&format!("regexp clone oom lim={}", lim), act_clone, 0);
        set_ps(0, &pat);
        set_pi(0, lim);
        diff_native(&format!("Rp_toString oom lim={}", lim), act_tostring, 0);
    }
}

/// Rows 556 + 559 — `lastIndex` beyond the subject length on a /g regexp
/// (`js_RegExp_prototype_exec` and `Rp_test` both reset `last` and bail out).
#[test]
fn regexp_lastindex_beyond_subject() {
    let srcs = [
        "var r=/a/g; r.lastIndex=99; [String(r.exec('a')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=99; [r.test('a'), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=2; [String(r.exec('a')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=1; [String(r.exec('a')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=1; [r.test('a'), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=NaN; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=1.7; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=65535; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex='2'; [String(r.exec('aaa')), r.lastIndex].join(',')",
        /* /g without the multiline bit sets REG_NOTBOL once last > 0 (row 546) */
        "var r=/^a/g; r.lastIndex=0; var o=[]; for(var i=0;i<3;i++){o.push(String(r.exec('aaa')));o.push(r.lastIndex)} o.join(',')",
        "var r=/^a/gm; var o=[]; for(var i=0;i<4;i++){o.push(String(r.exec('a\\na')));o.push(r.lastIndex)} o.join(',')",
        /* non-global regexps ignore lastIndex entirely */
        "var r=/a/; r.lastIndex=99; [String(r.exec('a')), r.lastIndex].join(',')",
        "var r=/a/; r.lastIndex=99; [r.test('a'), r.lastIndex].join(',')",
    ];
    for s in srcs {
        diff_eval("lastIndex", s, 0);
        diff_eval("lastIndex", s, JS_STRICT);
    }
}

/// Rows 556 + 559 with an out-of-`unsigned short`-range `lastIndex`.
///
/// Was a GENUINE DIVERGENCE, now fixed in translation/src/jsrun.rs: `re->last`
/// is `unsigned short` and the C assigns the double straight to it, so the value
/// wraps modulo 65536 (gcc: cvttsd2si then truncate), whereas a Rust
/// `as c_ushort` cast SATURATES. The Rust now truncates through i64.
#[test]
fn regexp_lastindex_out_of_ushort_range() {
    let srcs = [
        "var r=/a/g; r.lastIndex=-1; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=65536; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=65536; r.lastIndex",
        "var r=/a/g; r.lastIndex=65537; r.lastIndex",
        "var r=/a/g; r.lastIndex=131072; r.lastIndex",
        "var r=/a/g; r.lastIndex=-1; [r.test('aaa'), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=-1; r.lastIndex",
        "var r=/a/g; r.lastIndex=-100; r.lastIndex",
        "var r=/a/g; r.lastIndex=1e10; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=1e10; [r.test('aaa'), r.lastIndex].join(',')",
        "var r=/a/g; r.lastIndex=1e10; r.lastIndex",
        "var r=/a/g; r.lastIndex=-1e10; r.lastIndex",
        "var r=/a/g; r.lastIndex=Infinity; r.lastIndex",
        "var r=/a/g; r.lastIndex=-Infinity; r.lastIndex",
    ];
    let p = libs();
    let mut bad = Vec::new();
    for s in srcs {
        let c = p.c.eval(s, 0);
        let r = p.r.eval(s, 0);
        if c != r {
            bad.push(format!("  src={:?}\n    C   : {:?}\n    RUST: {:?}", s, c, r));
        }
    }
    if !bad.is_empty() {
        panic!(
            "{} of {} lastIndex cases diverge:\n{}",
            bad.len(),
            srcs.len(),
            bad.join("\n")
        );
    }
}

/// Rows 557 + 560 — `js_regexec` returning < 0 becomes Error "regexec failed"
/// in both `js_RegExp_prototype_exec` and `Rp_test`.
#[test]
fn regexp_regexec_failed_error() {
    with_big_stack(regexp_regexec_failed_error_body);
}

fn regexp_regexec_failed_error_body() {
    let srcs = [
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { /a*/.exec(s) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { /a*/.test(s) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<6000;i++) s+='a'; try { /(a)*/.exec(s) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { s.match(/a*/) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { s.match(/a*/g) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { s.replace(/a*/,'x') } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { s.search(/a*/) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4999;i++) s+='a'; try { s.split(/a*/).length } catch(e) { e.name+': '+e.message }",
        /* catastrophic backtracking through the JS level: >= ~2100 'a' so the
         * REG_MAXREC bail-out happens before the exponential search */
        "var s=''; for(var i=0;i<2500;i++) s+='a'; s+='b'; try { String(/(a|aa)+$/.test(s)) } catch(e) { e.name+': '+e.message }",
        "var s=''; for(var i=0;i<4000;i++) s+='a'; s+='b'; try { String(/(a|aa)+$/.exec(s)) } catch(e) { e.name+': '+e.message }",
    ];
    for s in srcs {
        diff_eval("regexec failed", s, 0);
    }
    /* pin the C behaviour for the canonical case of rows 557/560 */
    let p = libs();
    let c = p.c.eval(srcs[0], 0);
    assert!(
        c.contains("Error: regexec failed"),
        "C /a*/.exec(4999 a's) -> {:?}",
        c
    );
}

/// Rows 558 + 561 — `js_regexec` returning 1 (no match): `exec` -> null,
/// `test` -> false, and `last` reset when /g.
#[test]
fn regexp_exec_test_no_match_resets_last() {
    let srcs = [
        "String(/b/.exec('aaa'))",
        "/b/.test('aaa')",
        "var r=/b/g; r.lastIndex=1; [String(r.exec('aaa')), r.lastIndex].join(',')",
        "var r=/b/g; r.lastIndex=1; [r.test('aaa'), r.lastIndex].join(',')",
        "var r=/a/g; var o=[]; for(var i=0;i<4;i++){o.push(String(r.exec('aa')));o.push(r.lastIndex)} o.join(',')",
        "var r=/a/g; var o=[]; for(var i=0;i<4;i++){o.push(r.test('aa'));o.push(r.lastIndex)} o.join(',')",
        "var r=/(x)?a/; var m=r.exec('a'); [m.length, String(m[0]), String(m[1]), m.index, m.input].join(',')",
        "String('aaa'.match(/b/))",
        "'aaa'.search(/b/)",
    ];
    for s in srcs {
        diff_eval("exec/test no match", s, 0);
    }
}

/// Rows 562 + 563 + 564 — `js_toregexp` on a non-regexp `this`: TypeError
/// "not a regexp", from `Rp_test`, `Rp_toString` and `Rp_exec`.
#[test]
fn toregexp_not_a_regexp() {
    let receivers = [
        "{}",
        "1",
        "'x'",
        "null",
        "undefined",
        "[]",
        "true",
        "new String('a')",
        "new Number(1)",
        "function(){}",
        "new Date(0)",
        "Math",
    ];
    for recv in receivers {
        for m in ["test", "exec", "toString"] {
            let src = format!(
                "try {{ String(RegExp.prototype.{}.call({}, 'a')) }} catch (e) {{ e.name + ': ' + e.message }}",
                m, recv
            );
            diff_eval("not a regexp", &src, 0);
        }
    }
    /* pin the C message */
    let p = libs();
    let c = p.c.eval(
        "try { RegExp.prototype.test.call({}, 'a') } catch (e) { e.name + ': ' + e.message }",
        0,
    );
    assert!(c.contains("TypeError: not a regexp"), "C -> {:?}", c);

    /* the same through the raw C API: js_toregexp on every non-regexp value */
    fn act(a: &Api, J: JS) {
        unsafe {
            match pi(0) {
                0 => (a.js_pushnumber)(J, 1.0),
                1 => (a.js_pushstring)(J, cs("x").as_ptr()),
                2 => (a.js_pushnull)(J),
                3 => (a.js_pushundefined)(J),
                4 => (a.js_newobject)(J),
                5 => (a.js_newarray)(J),
                6 => (a.js_pushboolean)(J, 1),
                7 => (a.js_newstring)(J, cs("s").as_ptr()),
                8 => (a.js_pushglobal)(J),
                _ => (a.js_newnumber)(J, 2.0),
            }
            emit(&format!("isregexp={}", (a.js_isregexp)(J, -1)));
            let r = (a.js_toregexp)(J, -1);
            emit(&format!("NO-THROW nonnull={}", !r.is_null()));
        }
    }
    for k in 0..10i64 {
        set_pi(0, k);
        diff_native(&format!("js_toregexp non-regexp {}", k), act, 0);
    }
}

/// Row 566 — `jsB_new_RegExp` (jsregexp.c:148-149): flags supplied while
/// cloning a RegExp.
#[test]
fn new_regexp_clone_with_flags_typeerror() {
    let srcs = [
        "try { new RegExp(/a/, 'g') } catch (e) { e.name + ': ' + e.message }",
        "try { new RegExp(/a/, '') } catch (e) { e.name + ': ' + e.message }",
        "try { new RegExp(/a/g, 'i') } catch (e) { e.name + ': ' + e.message }",
        "try { new RegExp(/a/, undefined) } catch (e) { e.name + ': ' + e.message }",
        "try { new RegExp(/a/, null) } catch (e) { e.name + ': ' + e.message }",
        "try { RegExp(/a/, 'g') } catch (e) { e.name + ': ' + e.message }",
        /* the pure clone path must still work */
        "String(new RegExp(/a\\/b/gim))",
        "var r = new RegExp(/a/g); [r.source, r.global, r.ignoreCase, r.multiline].join(',')",
    ];
    for s in srcs {
        diff_eval("clone flags", s, 0);
    }
    let p = libs();
    let c = p.c.eval(srcs[0], 0);
    assert!(
        c.contains("TypeError: cannot supply flags when creating one RegExp from another"),
        "C -> {:?}",
        c
    );
}

/// Rows 567..570 — `jsB_new_RegExp` (jsregexp.c:172/175/176/177): invalid and
/// duplicated flag letters.
#[test]
fn new_regexp_invalid_and_duplicate_flags() {
    /* row 567: an unknown letter reports the first offending character */
    for f in [
        "x", "G", "I", "M", "s", "y", "u", "gx", "xg", "g i", "1", "-", "\u{e9}", "gim ",
    ] {
        let src = format!(
            "try {{ new RegExp('a', {}) }} catch (e) {{ e.name + ': ' + e.message }}",
            jsq(f)
        );
        diff_eval("invalid flag", &src, 0);
    }
    /* rows 568/569/570: duplicated g / i / m */
    for f in ["gg", "ii", "mm", "ggg", "gig", "igi", "mgm", "gimg", "gimi", "gimm"] {
        let src = format!(
            "try {{ new RegExp('a', {}) }} catch (e) {{ e.name + ': ' + e.message }}",
            jsq(f)
        );
        diff_eval("duplicate flag", &src, 0);
    }
    /* every valid permutation still works */
    for f in ["", "g", "i", "m", "gi", "gm", "im", "gim", "mig", "mgi"] {
        let src = format!(
            "var r = new RegExp('a', {}); [String(r), r.global, r.ignoreCase, r.multiline].join(',')",
            jsq(f)
        );
        diff_eval("valid flags", &src, 0);
    }
    /* flags come from ToString, so objects and numbers are converted first */
    for f in ["1", "{}", "[]", "['g']", "{toString:function(){return 'gi'}}", "true"] {
        let src = format!(
            "try {{ String(new RegExp('a', {})) }} catch (e) {{ e.name + ': ' + e.message }}",
            f
        );
        diff_eval("flags tostring", &src, 0);
    }
    let p = libs();
    for (f, want) in [
        ("x", "invalid regular expression flag: 'x'"),
        ("gg", "invalid regular expression flag: 'g'"),
        ("ii", "invalid regular expression flag: 'i'"),
        ("mm", "invalid regular expression flag: 'm'"),
    ] {
        let src = format!(
            "try {{ new RegExp('a', '{}') }} catch (e) {{ e.name + ': ' + e.message }}",
            f
        );
        let c = p.c.eval(&src, 0);
        assert!(
            c.contains(&format!("SyntaxError: {}", want)),
            "C flags {:?} -> {:?}",
            f,
            c
        );
    }
}

/// Row 553 (JS level) — every distinct regexp.c `die()` string reachable
/// through `new RegExp` / a regexp literal / `String.prototype.*`.
#[test]
fn all_die_messages_through_js_level() {
    let table: [(&str, &str); 17] = [
        (r"a\xZZ", "invalid escape sequence"),
        ("a{b}", "invalid quantifier"),
        (r"a\", "unterminated escape sequence"),
        (r"a\y", "invalid escape character"),
        ("a{255}", "numeric overflow"),
        ("[z-a]", "invalid character class range"),
        ("[a", "unterminated character class"),
        ("()*", "infinite loop matching the empty string"),
        (r"\1", "invalid back-reference"),
        ("*a", "syntax error"),
        ("a{2,1}", "invalid quantifier"),
        ("(a", "unmatched '('"),
        ("(?:a", "unmatched '('"),
        ("(?=a", "unmatched '('"),
        ("(?!a", "unmatched '('"),
        ("a)", "unmatched ')'"),
        ("(?:a{254}){254}", "program too large"),
    ];
    for (pat, msg) in table {
        expect_js_regexp_die(pat, msg);
    }
    /* the generated limits */
    expect_js_regexp_die(&"[a]".repeat(129), "too many character classes");
    expect_js_regexp_die(&"()".repeat(16), "too many captures");
    expect_js_regexp_die(&"a".repeat(4100), "stack overflow");
    expect_js_regexp_die(&"a{99}".repeat(400), "program too large");
    {
        let mut s = String::from("[");
        for i in 0..40 {
            s.push_str(&format!("\\u{:04X}", 0x2001 + i * 4));
        }
        s.push(']');
        expect_js_regexp_die(&s, "too many character class ranges");
    }
    /* the same failures reached through String.prototype.* (which build a
     * RegExp from a string argument) */
    for m in [
        "'a'.match('a{2,1}')",
        "'a'.replace('[a','x')",
        "'a'.search('(')",
        "'a'.split('a{255}')",
    ] {
        diff_eval(
            "string method bad regexp",
            &format!("try {{ String({}) }} catch (e) {{ e.name + ': ' + e.message }}", m),
            0,
        );
    }
    /* a regexp *literal* with a bad body is a lexer/parser error, not a
     * SyntaxError from regcomp: check the two libraries still agree */
    for src in [
        "try { eval('/a{2,1}/') } catch (e) { e.name + ': ' + e.message }",
        "try { eval('/[a/') } catch (e) { e.name + ': ' + e.message }",
        "try { eval('/(/') } catch (e) { e.name + ': ' + e.message }",
        "try { eval('/*/') } catch (e) { e.name + ': ' + e.message }",
    ] {
        diff_eval("regexp literal", src, 0);
    }
}

/* ================================================================= utf.c */

fn buf_of(bytes: &[u8]) -> [c_char; 12] {
    let mut b = [0 as c_char; 12];
    for (i, x) in bytes.iter().enumerate().take(11) {
        b[i] = *x as i8 as c_char;
    }
    b
}

fn chartorune_pair(bytes: &[u8]) -> (String, String) {
    let p = libs();
    let b = buf_of(bytes);
    unsafe {
        let mut rc: c_int = -1;
        let mut rr: c_int = -1;
        let nc = (p.c.jsU_chartorune)(&mut rc, b.as_ptr());
        let nr = (p.r.jsU_chartorune)(&mut rr, b.as_ptr());
        (
            format!("len={} rune={:#x}", nc, rc),
            format!("len={} rune={:#x}", nr, rr),
        )
    }
}

/// Rows 571..580 — every `goto bad` path of `chartorune` plus the `C0 80`
/// special case. Both the decoded rune and the returned length are compared.
#[test]
fn utf_chartorune_rejection_paths() {
    /* (row, bytes, expected "len=N rune=0xR") */
    let cases: [(u32, &[u8], &str); 22] = [
        /* 571: second byte is not a continuation */
        (571, &[0xC2, 0x20], "len=1 rune=0xfffd"),
        (571, &[0xC2, 0x41], "len=1 rune=0xfffd"),
        (571, &[0xC2, 0xC2], "len=1 rune=0xfffd"),
        (571, &[0xC2], "len=1 rune=0xfffd"),
        /* 572: stray continuation byte as lead */
        (572, &[0x80], "len=1 rune=0xfffd"),
        (572, &[0xBF], "len=1 rune=0xfffd"),
        (572, &[0xA0, 0x80], "len=1 rune=0xfffd"),
        /* 573: overlong 2-byte form */
        (573, &[0xC1, 0x81], "len=1 rune=0xfffd"),
        (573, &[0xC0, 0xBF], "len=1 rune=0xfffd"),
        /* 574: third byte is not a continuation */
        (574, &[0xE0, 0xA0, 0x20], "len=1 rune=0xfffd"),
        (574, &[0xE0, 0xA0], "len=1 rune=0xfffd"),
        (574, &[0xE0], "len=1 rune=0xfffd"),
        /* 575: overlong 3-byte form */
        (575, &[0xE0, 0x80, 0x80], "len=1 rune=0xfffd"),
        (575, &[0xE0, 0x9F, 0xBF], "len=1 rune=0xfffd"),
        /* 576: fourth byte is not a continuation */
        (576, &[0xF0, 0x90, 0x80, 0x20], "len=1 rune=0xfffd"),
        (576, &[0xF0, 0x90, 0x80], "len=1 rune=0xfffd"),
        (576, &[0xF0, 0x90], "len=1 rune=0xfffd"),
        (576, &[0xF0], "len=1 rune=0xfffd"),
        /* 577: overlong 4-byte form */
        (577, &[0xF0, 0x80, 0x80, 0x80], "len=1 rune=0xfffd"),
        /* 578: above Runemax */
        (578, &[0xF4, 0x90, 0x80, 0x80], "len=1 rune=0xfffd"),
        /* 579: lead byte >= T5 */
        (579, &[0xF8, 0x88, 0x80, 0x80, 0x80], "len=1 rune=0xfffd"),
        /* 580: overlong NUL accepted */
        (580, &[0xC0, 0x80], "len=2 rune=0x0"),
    ];
    for (row, bytes, want) in cases {
        assert!(row >= 571 && row <= 580);
        let (c, r) = chartorune_pair(bytes);
        same(&format!("chartorune {:02X?}", bytes), &c, &r);
        assert_eq!(c, want, "C chartorune({:02X?}) row {}", bytes, row);
    }
    /* row 579 continued: every lead byte >= 0xF8 */
    for lead in 0xF8u8..=0xFF {
        let (c, r) = chartorune_pair(&[lead, 0x88, 0x80, 0x80, 0x80]);
        same(&format!("chartorune lead {:#02x}", lead), &c, &r);
        assert_eq!(c, "len=1 rune=0xfffd", "lead {:#02x}", lead);
    }
    /* row 572 continued: every stray continuation byte */
    for lead in 0x80u8..=0xBF {
        let (c, r) = chartorune_pair(&[lead, 0x80, 0x80]);
        same(&format!("chartorune stray {:#02x}", lead), &c, &r);
        assert_eq!(c, "len=1 rune=0xfffd", "stray {:#02x}", lead);
    }
    /* the empty string and an embedded NUL */
    let (c, r) = chartorune_pair(&[]);
    same("chartorune empty", &c, &r);
    assert_eq!(c, "len=1 rune=0x0");
}

/// Rows 571..580, randomised: a few thousand random byte strings with a fixed
/// seed, decoded byte-by-byte the way every caller does.
#[test]
fn utf_chartorune_random_byte_strings() {
    let p = libs();
    let mut rng = Rng::new(0xC0DE_5EED_0000_0501);
    unsafe {
        for iter in 0..5000 {
            let n = 1 + rng.below(10) as usize;
            let mut bytes = Vec::with_capacity(n);
            for _ in 0..n {
                bytes.push(match rng.below(6) {
                    0 => (0x80 + rng.below(0x40)) as u8, /* continuation */
                    1 => (0xC0 + rng.below(0x20)) as u8, /* 2-byte lead */
                    2 => (0xE0 + rng.below(0x10)) as u8, /* 3-byte lead */
                    3 => (0xF0 + rng.below(0x10)) as u8, /* 4-byte / invalid lead */
                    4 => rng.below(0x80) as u8,
                    _ => rng.below(0x100) as u8,
                });
            }
            /* zero bytes terminate: keep them, they exercise the truncated paths */
            let b = buf_of(&bytes);
            /* walk the whole buffer the way jsstring.c does */
            let mut ac = String::new();
            let mut ar = String::new();
            let mut off = 0usize;
            while off < 11 && b[off] != 0 {
                let mut xc: c_int = -1;
                let mut xr: c_int = -1;
                let nc = (p.c.jsU_chartorune)(&mut xc, b.as_ptr().add(off));
                let nr = (p.r.jsU_chartorune)(&mut xr, b.as_ptr().add(off));
                ac.push_str(&format!("{}:{:#x} ", nc, xc));
                ar.push_str(&format!("{}:{:#x} ", nr, xr));
                if nc <= 0 || nr <= 0 {
                    break;
                }
                off += nc.max(nr) as usize;
            }
            same(
                &format!("chartorune walk #{} {:02X?}", iter, bytes),
                &ac,
                &ar,
            );
        }
    }
}

/// Rows 581 + 582 — `runetochar` above Runemax and for rune 0, plus negative
/// runes handed across the FFI boundary. `runelen` must agree too.
#[test]
fn utf_runetochar_out_of_range_and_zero() {
    let p = libs();
    /* (row, rune, expected "n=<len> bytes=[..]") */
    let cases: [(u32, c_int, &str, c_int); 8] = [
        /* 582: rune 0 -> overlong C0 80 */
        (582, 0, "c0 80", 2),
        /* 581: above Runemax -> Runeerror EF BF BD */
        (581, 0x110000, "ef bf bd", 3),
        (581, 0x1FFFFF, "ef bf bd", 3),
        (581, 0x7FFFFFFF, "ef bf bd", 3),
        /* negative runes take the 1-byte branch (c <= Rune1) */
        (581, -1, "ff", 1),
        (581, -2, "fe", 1),
        (581, i32::MIN, "00", 1),
        (581, 0x10FFFF, "f4 8f bf bf", 4),
    ];
    unsafe {
        for (row, c, want_bytes, want_n) in cases {
            assert!(row == 581 || row == 582);
            let mut bc = [0x5Au8 as c_char; 16];
            let mut br = [0x5Au8 as c_char; 16];
            let nc = (p.c.jsU_runetochar)(bc.as_mut_ptr(), &c);
            let nr = (p.r.jsU_runetochar)(br.as_mut_ptr(), &c);
            let hc = bc[..(nc.max(0) as usize).min(16)]
                .iter()
                .map(|b| format!("{:02x}", *b as u8))
                .collect::<Vec<_>>()
                .join(" ");
            same(
                &format!("runetochar {:#x}", c),
                &format!("{} {:?} {:?}", nc, hc, bc),
                &format!(
                    "{} {:?} {:?}",
                    nr,
                    br[..(nr.max(0) as usize).min(16)]
                        .iter()
                        .map(|b| format!("{:02x}", *b as u8))
                        .collect::<Vec<_>>()
                        .join(" "),
                    br
                ),
            );
            assert_eq!((nc, hc.as_str()), (want_n, want_bytes), "C runetochar({:#x})", c);
            /* runelen must agree with runetochar's length except where the C
             * deliberately differs (rune 0): compare both libraries only. */
            same(
                &format!("runelen {:#x}", c),
                &format!("{}", (p.c.jsU_runelen)(c)),
                &format!("{}", (p.r.jsU_runelen)(c)),
            );
        }
        /* row 581: runelen(0x110000) == 3 (Runeerror substitution) */
        assert_eq!((p.c.jsU_runelen)(0x110000), 3);
        same(
            "runelen 0x110000",
            &format!("{}", (p.c.jsU_runelen)(0x110000)),
            &format!("{}", (p.r.jsU_runelen)(0x110000)),
        );
        /* round-trip every out-of-range rune through runetochar+chartorune */
        for c in [0x110000, 0x200000, 0x7FFFFFFF, -1, -100000, 0] {
            let mut bc = [0 as c_char; 16];
            let n = (p.c.jsU_runetochar)(bc.as_mut_ptr(), &c);
            bc[(n.max(0) as usize).min(15)] = 0;
            let bytes: Vec<u8> = bc[..(n.max(0) as usize).min(15)]
                .iter()
                .map(|b| *b as u8)
                .collect();
            let (cc, rr) = chartorune_pair(&bytes);
            same(&format!("roundtrip {:#x}", c), &cc, &rr);
        }
    }
}

/// Row 583 — `ucd_bsearch` misses: codepoints below the first table entry or
/// not covered by any entry.
#[test]
fn utf_ucd_bsearch_misses() {
    let p = libs();
    let mut cases: Vec<c_int> = vec![
        '$' as c_int,
        '#' as c_int,
        ' ' as c_int,
        '0' as c_int,
        0,
        1,
        7,
        0x2FFFF,
        0x30000,
        0xE0000,
        0x10FFFF,
        0x110000,
        0x7FFFFFFF,
        -1,
        -2,
        i32::MIN,
        0xD800,
        0xDFFF,
        0xFFFE,
        0xFFFF,
    ];
    /* a systematic sweep of unassigned planes (guaranteed table misses) */
    let mut rng = Rng::new(0xC0DE_5EED_0000_0583);
    for _ in 0..3000 {
        cases.push((0x30000 + rng.below(0xD0000)) as c_int);
    }
    for _ in 0..2000 {
        cases.push(rng.u32() as c_int);
    }
    unsafe {
        for c in cases {
            let cd = format!(
                "alpha={} lower={} upper={} tolower={:#x} toupper={:#x}",
                (p.c.jsU_isalpharune)(c),
                (p.c.jsU_islowerrune)(c),
                (p.c.jsU_isupperrune)(c),
                (p.c.jsU_tolowerrune)(c),
                (p.c.jsU_toupperrune)(c)
            );
            let rd = format!(
                "alpha={} lower={} upper={} tolower={:#x} toupper={:#x}",
                (p.r.jsU_isalpharune)(c),
                (p.r.jsU_islowerrune)(c),
                (p.r.jsU_isupperrune)(c),
                (p.r.jsU_tolowerrune)(c),
                (p.r.jsU_toupperrune)(c)
            );
            same(&format!("ucd miss {:#x}", c), &cd, &rd);
        }
        /* the documented misses return the codepoint unchanged / 0 */
        assert_eq!((p.c.jsU_isalpharune)('$' as c_int), 0);
        assert_eq!((p.c.jsU_toupperrune)(0x2FFFF), 0x2FFFF);
        assert_eq!((p.c.jsU_tolowerrune)(0x2FFFF), 0x2FFFF);
    }
}

/// Rows 584 + 585 — `tolowerrune_full` / `toupperrune_full` returning NULL.
#[test]
fn utf_full_case_mapping_null() {
    let p = libs();
    unsafe {
        /* documented NULL cases */
        assert!(
            (p.c.jsU_tolowerrune_full)('a' as c_int).is_null(),
            "C tolowerrune_full('a') should be NULL"
        );
        assert!(
            (p.c.jsU_toupperrune_full)('A' as c_int).is_null(),
            "C toupperrune_full('A') should be NULL"
        );
        /* a full sweep of the BMP + a slice above it, plus negatives */
        let mut cases: Vec<c_int> = (-4..0x11000).collect();
        cases.extend([
            0x1E900, 0x1E921, 0x2FFFF, 0x110000, 0x7FFFFFFF, i32::MIN, -0x110000,
        ]);
        let mut nlower_null = 0usize;
        let mut nlower_hit = 0usize;
        let mut nupper_null = 0usize;
        let mut nupper_hit = 0usize;
        for c in cases {
            let lc = (p.c.jsU_tolowerrune_full)(c);
            let lr = (p.r.jsU_tolowerrune_full)(c);
            let cd = fmt_full(lc, 3);
            let rd = fmt_full(lr, 3);
            same(&format!("tolowerrune_full {:#x}", c), &cd, &rd);
            if lc.is_null() {
                nlower_null += 1;
            } else {
                nlower_hit += 1;
            }
            let uc = (p.c.jsU_toupperrune_full)(c);
            let ur = (p.r.jsU_toupperrune_full)(c);
            same(
                &format!("toupperrune_full {:#x}", c),
                &fmt_full(uc, 4),
                &fmt_full(ur, 4),
            );
            if uc.is_null() {
                nupper_null += 1;
            } else {
                nupper_hit += 1;
            }
        }
        assert!(nlower_null > 0 && nlower_hit > 0, "lower: {} {}", nlower_null, nlower_hit);
        assert!(nupper_null > 0 && nupper_hit > 0, "upper: {} {}", nupper_null, nupper_hit);
    }
    /* the JS-visible fallback (jsstring.c uses the NULL return to fall back to
     * the simple mapping) */
    for src in [
        "'a'.toLowerCase()",
        "'A'.toLowerCase()",
        "'\\u0130'.toLowerCase().length",
        "'\\u1F88'.toLowerCase()",
        "'\\u00DF'.toUpperCase()",
        "'\\uFB00'.toUpperCase()",
        "'$'.toUpperCase()",
        "'\\uD83D\\uDE00'.toUpperCase().length",
    ] {
        diff_eval("full case", src, 0);
    }
}

unsafe fn fmt_full(p: *const c_int, max: usize) -> String {
    if p.is_null() {
        return "NULL".into();
    }
    let mut s = String::from("[");
    for i in 0..max {
        let v = *p.add(i);
        if v == 0 {
            break;
        }
        s.push_str(&format!("{:#x} ", v));
    }
    s.push(']');
    s
}

/* ============================================================== jsdtoa.c */

fn bits(x: f64) -> u64 {
    x.to_bits()
}

unsafe fn off(base: *const c_char, end: *mut c_char) -> isize {
    if end.is_null() {
        -1
    } else {
        end as isize - base as isize
    }
}

/// `js_strtod` in both libraries: the returned double BITS and the end-pointer
/// offset.
#[track_caller]
fn expect_strtod(s: &str, want_bits: Option<u64>, want_off: Option<isize>) {
    let p = libs();
    unsafe {
        let cstr = cs(s);
        let mut ec: *mut c_char = std::ptr::null_mut();
        let mut er: *mut c_char = std::ptr::null_mut();
        let vc = (p.c.js_strtod)(cstr.as_ptr(), &mut ec);
        let vr = (p.r.js_strtod)(cstr.as_ptr(), &mut er);
        let cd = format!("{:#018x} off={}", bits(vc), off(cstr.as_ptr(), ec));
        let rd = format!("{:#018x} off={}", bits(vr), off(cstr.as_ptr(), er));
        same(&format!("js_strtod {:?}", s), &cd, &rd);
        if let Some(b) = want_bits {
            assert_eq!(bits(vc), b, "C js_strtod({:?}) value {:?}", s, vc);
        }
        if let Some(o) = want_off {
            assert_eq!(off(cstr.as_ptr(), ec), o, "C js_strtod({:?}) endptr", s);
        }
        /* js_strtod is also reachable with a NULL endPtr */
        let vc2 = (p.c.js_strtod)(cstr.as_ptr(), std::ptr::null_mut());
        let vr2 = (p.r.js_strtod)(cstr.as_ptr(), std::ptr::null_mut());
        same(
            &format!("js_strtod nullend {:?}", s),
            &format!("{:#018x}", bits(vc2)),
            &format!("{:#018x}", bits(vr2)),
        );
        /* and through js_stringtofloat / jsV_stringtonumber */
        let vc3 = (p.c.js_stringtofloat)(cstr.as_ptr(), &mut ec);
        let vr3 = (p.r.js_stringtofloat)(cstr.as_ptr(), &mut er);
        same(
            &format!("js_stringtofloat {:?}", s),
            &format!("{:#018x} off={}", bits(vc3), off(cstr.as_ptr(), ec)),
            &format!("{:#018x} off={}", bits(vr3), off(cstr.as_ptr(), er)),
        );
    }
}

/// Rows 588..591 — the four `js_strtod` mantissa/exponent guards.
#[test]
fn strtod_mantissa_and_exponent_guards() {
    /* row 588: a second '.' (or any non-digit) terminates the mantissa scan */
    expect_strtod("1.2.3", Some(bits(1.2)), Some(3));
    expect_strtod("0..1", Some(bits(0.0)), Some(2));
    expect_strtod(".1.2", Some(bits(0.1)), Some(2));
    expect_strtod("1.2x3", Some(bits(1.2)), Some(3));
    expect_strtod("1..", Some(bits(1.0)), Some(2));
    expect_strtod("1.2e3.4", Some(bits(1200.0)), Some(5));
    expect_strtod("..", Some(bits(0.0)), Some(0));

    /* row 589: mantissa longer than 18 significant digits */
    expect_strtod(
        "12345678901234567890123.5",
        Some(bits(1.2345678901234568e22)),
        Some(25),
    );
    expect_strtod("1234567890123456789", None, Some(19));
    expect_strtod("123456789012345678901234567890", None, Some(30));
    expect_strtod("0.00000000000000000000123456789012345678901234", None, None);
    expect_strtod(&format!("{}.5", "9".repeat(40)), None, None);
    expect_strtod(&"1".repeat(60), None, Some(60));

    /* row 590: no mantissa digits at all */
    expect_strtod("abc", Some(bits(0.0)), Some(0));
    expect_strtod("", Some(bits(0.0)), Some(0));
    expect_strtod(".", Some(bits(0.0)), Some(0));
    expect_strtod("-", Some(bits(-0.0)), Some(0));
    expect_strtod("-.", Some(bits(-0.0)), Some(0));
    expect_strtod("+", Some(bits(0.0)), Some(0));
    expect_strtod("-abc", Some(bits(-0.0)), Some(0));
    expect_strtod("+.e5", Some(bits(0.0)), Some(0));
    expect_strtod("e5", Some(bits(0.0)), Some(0));
    expect_strtod("   ", Some(bits(0.0)), Some(0));
    expect_strtod("Infinity", Some(bits(0.0)), Some(0));
    expect_strtod("NaN", Some(bits(0.0)), Some(0));
    expect_strtod("0x10", None, None);

    /* row 591: exponent digits accumulate past INT_MAX/100 and are dropped */
    expect_strtod("1e99999999999999999999", Some(bits(f64::INFINITY)), None);
    expect_strtod("1e-99999999999999999999", Some(bits(0.0)), None);
    expect_strtod("1e2147483647", Some(bits(f64::INFINITY)), None);
    expect_strtod("1e-2147483648", Some(bits(0.0)), None);
    expect_strtod(&format!("1e{}", "9".repeat(100)), Some(bits(f64::INFINITY)), None);
    expect_strtod(&format!("1e-{}", "9".repeat(100)), Some(bits(0.0)), None);
    /* fraction 0 multiplied by the clamped inf: whatever it is, both agree */
    expect_strtod("0e99999999999999999999", None, None);
    expect_strtod("0e-99999999999999999999", None, None);

    /* the same strings through the JS number parser */
    for s in [
        "1.2.3", "abc", ".", "1e999", "1e-999", "1e99999999999999999999",
        "12345678901234567890123.5",
    ] {
        diff_eval(
            "Number()",
            &format!("[Number({0}), parseFloat({0}), +{0}].join(',')", jsq(s)),
            0,
        );
    }
}

/// Rows 592 + 593 — the `errno = ERANGE` clamping of the exponent.
#[test]
fn strtod_erange_clamping() {
    /* row 593: exp > maxExponent(511) -> inf */
    expect_strtod("1e999", Some(bits(f64::INFINITY)), Some(5));
    expect_strtod("-1e999", Some(bits(f64::NEG_INFINITY)), Some(6));
    expect_strtod("1e512", Some(bits(f64::INFINITY)), Some(5));
    expect_strtod("1e511", None, Some(5));
    expect_strtod("1e309", Some(bits(f64::INFINITY)), Some(5));
    expect_strtod("1e308", None, Some(5));
    /* row 592: exp < -maxExponent(511) -> 0 */
    expect_strtod("1e-999", Some(bits(0.0)), Some(6));
    expect_strtod("-1e-999", Some(bits(-0.0)), Some(7));
    expect_strtod("1e-512", Some(bits(0.0)), Some(6));
    expect_strtod("1e-511", None, Some(6));
    expect_strtod("1e-324", None, Some(6));
    expect_strtod("1e-400", Some(bits(0.0)), Some(6));
    /* the boundary sweep: every decade around the clamp */
    for e in [-600i32, -520, -513, -512, -511, -510, 509, 510, 511, 512, 513, 520, 600] {
        expect_strtod(&format!("1e{}", e), None, None);
        expect_strtod(&format!("-1e{}", e), None, None);
        expect_strtod(&format!("9.9e{}", e), None, None);
        expect_strtod(&format!("0e{}", e), None, None);
    }
    /* and through the JS level (where ERANGE is invisible but the value is not) */
    for s in ["1e999", "-1e999", "1e-999", "-1e-999", "1e512", "1e-512"] {
        diff_eval(
            "erange",
            &format!("[Number({0}), String(Number({0}))].join('|')", jsq(s)),
            0,
        );
    }
}

/// Rows 586 + 587 — the two `minus()` asserts inside `js_grisu2`
/// (`x.e == y.e`, `x.f >= y.f`). They are invariants, not reachable errors:
/// the C library is built WITHOUT NDEBUG, so a violation would `abort()`.
/// This sweeps the degenerate/denormal inputs the rows name and asserts both
/// libraries produce identical digits (hence neither trips the assert).
#[test]
fn grisu2_minus_invariants_hold() {
    let p = libs();
    let mut ds: Vec<f64> = Vec::new();
    /* every denormal built from the smallest possible mantissas */
    for i in 1..=4000u64 {
        ds.push(f64::from_bits(i));
    }
    /* the largest denormal, the smallest normal and its neighbours */
    for i in 0..64u64 {
        ds.push(f64::from_bits((1u64 << 52) - 1 - i));
        ds.push(f64::from_bits((1u64 << 52) + i));
    }
    /* exact powers of two (mantissa == 0: the "boundary" case in grisu2) */
    for e in 1..2047u64 {
        ds.push(f64::from_bits(e << 52));
    }
    /* largest finite and its neighbours */
    for i in 0..64u64 {
        ds.push(f64::from_bits(0x7FEF_FFFF_FFFF_FFFF - i));
    }
    let mut rng = Rng::new(0xC0DE_5EED_0000_0586);
    for _ in 0..20000 {
        let v = f64::from_bits(rng.next_u64() & 0x7FEF_FFFF_FFFF_FFFF);
        if v.is_finite() && v != 0.0 {
            ds.push(v);
        }
    }
    unsafe {
        for v in ds {
            let mut bc = [0x7Fu8 as c_char; 64];
            let mut br = [0x7Fu8 as c_char; 64];
            let mut kc: c_int = -999;
            let mut kr: c_int = -999;
            let nc = (p.c.js_grisu2)(v, bc.as_mut_ptr(), &mut kc);
            let nr = (p.r.js_grisu2)(v, br.as_mut_ptr(), &mut kr);
            same(
                &format!("js_grisu2 {:#018x}", bits(v)),
                &format!("{} {} {:?}", nc, kc, bc),
                &format!("{} {} {:?}", nr, kr, br),
            );
            /* the same values through the public number formatter */
            let mut sc = [0u8 as c_char; 64];
            let mut sr = [0u8 as c_char; 64];
            let cc = rs((p.c.jsV_numbertostring)(std::ptr::null_mut(), sc.as_mut_ptr(), v));
            let rr = rs((p.r.jsV_numbertostring)(std::ptr::null_mut(), sr.as_mut_ptr(), v));
            same(&format!("numbertostring {:#018x}", bits(v)), &cc, &rr);
        }
    }
}
