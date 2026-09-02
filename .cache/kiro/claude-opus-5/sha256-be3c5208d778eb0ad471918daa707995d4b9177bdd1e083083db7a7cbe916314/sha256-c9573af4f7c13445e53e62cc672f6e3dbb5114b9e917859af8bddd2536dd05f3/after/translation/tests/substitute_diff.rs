//! Phase C — `pcre2_substitute` error surface, plus Phase B valid-path
//! substitution differential coverage (the two are inseparable here because the
//! same call reports both).

mod common;
use common::*;
use std::ffi::c_void;

fn compile_ok(p: &'static Pair, pat: &[u8], opts: u32) -> CodePair {
    compile_both(p, pat, pat.len(), opts, std::ptr::null_mut(), std::ptr::null_mut(), "subst-setup")
        .expect("pattern should compile")
}

/// Run `pcre2_substitute` through both libraries with identical arguments and
/// compare the return code, the output length, and the produced bytes.
#[allow(clippy::too_many_arguments)]
fn subst_cmp(
    p: &Pair,
    cp: &CodePair,
    subject: &[u8],
    slen: Sz,
    startoffset: Sz,
    options: u32,
    repl: &[u8],
    rlen: Sz,
    outcap: usize,
    label: &str,
) -> i32 {
    let mut bc = vec![0xCDu8; outcap.max(1) + 16];
    let mut br = vec![0xCDu8; outcap.max(1) + 16];
    let mut lc: Sz = outcap;
    let mut lr: Sz = outcap;
    // PCRE2_ZERO_TERMINATED makes the library run strlen() over the buffer, so a
    // real NUL terminator has to be present.
    let subj_owned;
    let sp: *const u8 = if slen == PCRE2_ZERO_TERMINATED {
        subj_owned = {
            let mut v = subject.to_vec();
            v.push(0);
            v
        };
        subj_owned.as_ptr()
    } else if subject.is_empty() {
        std::ptr::null()
    } else {
        subject.as_ptr()
    };
    let repl_owned;
    let rp: *const u8 = if rlen == PCRE2_ZERO_TERMINATED {
        repl_owned = {
            let mut v = repl.to_vec();
            v.push(0);
            v
        };
        repl_owned.as_ptr()
    } else if repl.is_empty() {
        std::ptr::null()
    } else {
        repl.as_ptr()
    };
    let a = unsafe {
        (p.c.substitute)(
            cp.c, sp, slen, startoffset, options, std::ptr::null_mut(), std::ptr::null_mut(), rp,
            rlen, bc.as_mut_ptr(), &mut lc,
        )
    };
    let b = unsafe {
        (p.r.substitute)(
            cp.r, sp, slen, startoffset, options, std::ptr::null_mut(), std::ptr::null_mut(), rp,
            rlen, br.as_mut_ptr(), &mut lr,
        )
    };
    assert_eq!(a, b, "substitute rc [{}]", label);
    assert_eq!(lc, lr, "substitute out-length [{}]", label);
    assert_eq!(bc, br, "substitute out-buffer [{}]", label);
    a
}

// ===========================================================================
// Error paths
// ===========================================================================

#[test]
fn substitute_null_and_bad_arguments() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", 0);
    let mut buf = vec![0u8; 64];
    unsafe {
        // NOTE: pcre2_substitute() reads `code->overall_options` and `*blength` in
        // its declaration block, before any validation, so a NULL `code` or a NULL
        // `blength` is undefined behaviour in the C library (verified in
        // pcre2_substitute.c) and is not part of the error surface.
        // NULL replacement with non-zero length
        let mut lc: Sz = 64;
        let mut lr: Sz = 64;
        let a = (p.c.substitute)(
            cp.c, b"aaa".as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null(), 1, buf.as_mut_ptr(), &mut lc,
        );
        let b = (p.r.substitute)(
            cp.r, b"aaa".as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null(), 1, buf.as_mut_ptr(), &mut lr,
        );
        assert_eq!((a, lc), (b, lr));
        assert_eq!(a, err::NULL);
        // NULL subject with non-zero length
        let mut lc: Sz = 64;
        let mut lr: Sz = 64;
        let a = (p.c.substitute)(
            cp.c, std::ptr::null(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
            b"x".as_ptr(), 1, buf.as_mut_ptr(), &mut lc,
        );
        let b = (p.r.substitute)(
            cp.r, std::ptr::null(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
            b"x".as_ptr(), 1, buf.as_mut_ptr(), &mut lr,
        );
        assert_eq!((a, lc), (b, lr));
        assert_eq!(a, err::NULL);
        // SUBSTITUTE_MATCHED without a match_data -> NULL
        let mut lc: Sz = 64;
        let mut lr: Sz = 64;
        let a = (p.c.substitute)(
            cp.c, b"aaa".as_ptr(), 3, 0, o::SUBSTITUTE_MATCHED, std::ptr::null_mut(),
            std::ptr::null_mut(), b"x".as_ptr(), 1, buf.as_mut_ptr(), &mut lc,
        );
        let b = (p.r.substitute)(
            cp.r, b"aaa".as_ptr(), 3, 0, o::SUBSTITUTE_MATCHED, std::ptr::null_mut(),
            std::ptr::null_mut(), b"x".as_ptr(), 1, buf.as_mut_ptr(), &mut lr,
        );
        assert_eq!((a, lc), (b, lr));
        assert_eq!(a, err::NULL);
    }
    free_code_pair(p, cp);
}

#[test]
fn substitute_bad_start_offset() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", 0);
    for off in [0usize, 3, 4, 100, usize::MAX] {
        let rc = subst_cmp(p, &cp, b"aaa", 3, off, 0, b"x", 1, 64, &format!("off{}", off));
        if off > 3 {
            assert_eq!(rc, err::BADOFFSET, "off {} should be BADOFFSET", off);
        }
    }
    free_code_pair(p, cp);
}

#[test]
fn substitute_rejects_unknown_option_bits() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", 0);
    for bit in 0..32u32 {
        let opt = 1u32 << bit;
        subst_cmp(p, &cp, b"aaa", 3, 0, opt, b"x", 1, 64, &format!("optbit{}", bit));
    }
    for opt in [u32::MAX, 0xFFF0_0000, o::PARTIAL_HARD | o::SUBSTITUTE_GLOBAL] {
        subst_cmp(p, &cp, b"aaa", 3, 0, opt, b"x", 1, 64, &format!("opts{:#x}", opt));
    }
    free_code_pair(p, cp);
}

/// Every distinct malformed replacement string the C code rejects.
static BAD_REPLACEMENTS: &[(&str, &[u8], u32)] = &[
    // simple (non-extended) mode
    ("dollar-at-end", b"$", 0),
    ("dollar-brace-unterminated", b"${1", 0),
    ("dollar-brace-empty", b"${}", 0),
    ("dollar-brace-bad", b"${1x}", 0),
    ("dollar-lt-unterminated", b"$<1", 0),
    ("dollar-bad-char", b"$%", 0),
    ("group-out-of-range", b"$9", 0),
    ("group-name-unknown", b"${nope}", 0),
    // extended mode escapes
    ("ext-backslash-at-end", b"\\", o::SUBSTITUTE_EXTENDED),
    ("ext-bad-escape", b"\\q", o::SUBSTITUTE_EXTENDED),
    ("ext-case-unterminated", b"\\U", o::SUBSTITUTE_EXTENDED),
    ("ext-cond-unterminated", b"${1:+a", o::SUBSTITUTE_EXTENDED),
    ("ext-cond-bad", b"${1:x}", o::SUBSTITUTE_EXTENDED),
    ("ext-cond-missing-brace", b"${1:+a:b", o::SUBSTITUTE_EXTENDED),
    ("ext-cond-unknown-group", b"${zz:+a:b}", o::SUBSTITUTE_EXTENDED),
    ("ext-nested-unterminated", b"${1:+${2:+x", o::SUBSTITUTE_EXTENDED),
    ("ext-lowercase-esc", b"\\l", o::SUBSTITUTE_EXTENDED),
    ("ext-uppercase-esc", b"\\u", o::SUBSTITUTE_EXTENDED),
    ("ext-end-case", b"\\E", o::SUBSTITUTE_EXTENDED),
    ("ext-dollar-quote", b"$'", o::SUBSTITUTE_EXTENDED),
    ("ext-dollar-underscore", b"$_", o::SUBSTITUTE_EXTENDED),
    ("ext-dollar-backtick", b"$`", o::SUBSTITUTE_EXTENDED),
    ("ext-octal", b"\\o{101}", o::SUBSTITUTE_EXTENDED),
    ("ext-octal-bad", b"\\o{9}", o::SUBSTITUTE_EXTENDED),
    ("ext-octal-nobrace", b"\\o1", o::SUBSTITUTE_EXTENDED),
    ("ext-hex", b"\\x{41}", o::SUBSTITUTE_EXTENDED),
    ("ext-hex-bad", b"\\x{zz}", o::SUBSTITUTE_EXTENDED),
    ("ext-hex-nobrace", b"\\x41", o::SUBSTITUTE_EXTENDED),
    ("ext-hex-toobig", b"\\x{110000}", o::SUBSTITUTE_EXTENDED),
];

#[test]
fn substitute_bad_replacement_corpus() {
    let p = libs();
    for pat in [&b"(a)"[..], &b"(?<nm>a)"[..], &b"a"[..], &b"(a)(b)?"[..]] {
        let cp = compile_ok(libs(), pat, 0);
        for (name, repl, extra) in BAD_REPLACEMENTS {
            for glob in [0u32, o::SUBSTITUTE_GLOBAL] {
                for lit in [0u32, o::SUBSTITUTE_LITERAL] {
                    let label =
                        format!("{}|{}|{:#x}", String::from_utf8_lossy(pat), name, extra | glob | lit);
                    subst_cmp(p, &cp, b"ab", 2, 0, extra | glob | lit, repl, repl.len(), 128, &label);
                    // zero-terminated form too
                    subst_cmp(
                        p,
                        &cp,
                        b"ab",
                        PCRE2_ZERO_TERMINATED,
                        0,
                        extra | glob | lit,
                        repl,
                        PCRE2_ZERO_TERMINATED,
                        128,
                        &format!("{}|zt", label),
                    );
                }
            }
        }
        free_code_pair(p, cp);
    }
}

#[test]
fn substitute_output_overflow() {
    let p = libs();
    let cp = compile_ok(libs(), b"a", 0);
    for cap in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 16, 64] {
        for opt in [
            0u32,
            o::SUBSTITUTE_GLOBAL,
            o::SUBSTITUTE_OVERFLOW_LENGTH,
            o::SUBSTITUTE_GLOBAL | o::SUBSTITUTE_OVERFLOW_LENGTH,
            o::SUBSTITUTE_REPLACEMENT_ONLY,
            o::SUBSTITUTE_REPLACEMENT_ONLY | o::SUBSTITUTE_GLOBAL,
        ] {
            subst_cmp(p, &cp, b"aXaXa", 5, 0, opt, b"ZZ", 2, cap, &format!("cap{}|{:#x}", cap, opt));
        }
    }
    free_code_pair(p, cp);
}

#[test]
fn substitute_toomanyreplace() {
    let p = libs();
    // An empty match with GLOBAL on a long subject stresses the replacement
    // counter; also exercise patterns that can match empty.
    for pat in [&b"a*"[..], &b""[..], &b"(?=a)"[..], &b"\\b"[..]] {
        if let Ok(cp) = compile_both(p, pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), "tmr")
        {
            let subj = vec![b'a'; 64];
            subst_cmp(
                p,
                &cp,
                &subj,
                subj.len(),
                0,
                o::SUBSTITUTE_GLOBAL,
                b"-",
                1,
                4096,
                &format!("tmr {:?}", String::from_utf8_lossy(pat)),
            );
            free_code_pair(p, cp);
        }
    }
}

#[test]
fn substitute_partial_and_badsubspattern() {
    let p = libs();
    // PARTIAL_* is rejected by pcre2_substitute (BADOPTION); \K in a lookaround
    // produces BADSUBSPATTERN.
    for pat in [&b"a"[..], &b"(?<=a\\Kb)"[..], &b"a\\Kb"[..]] {
        if let Ok(cp) = compile_both(p, pat, pat.len(), 0, std::ptr::null_mut(), std::ptr::null_mut(), "ps")
        {
            for opt in [
                0u32,
                o::PARTIAL_SOFT,
                o::PARTIAL_HARD,
                o::SUBSTITUTE_GLOBAL,
                o::SUBSTITUTE_GLOBAL | o::PARTIAL_HARD,
            ] {
                subst_cmp(
                    p,
                    &cp,
                    b"abab",
                    4,
                    0,
                    opt,
                    b"X",
                    1,
                    128,
                    &format!("{:?}|{:#x}", String::from_utf8_lossy(pat), opt),
                );
            }
            free_code_pair(p, cp);
        }
    }
}

#[test]
fn substitute_matched_mode_consistency_errors() {
    // PCRE2_SUBSTITUTE_MATCHED requires the match_data to come from the same
    // pattern / subject / offset / options; each mismatch has its own error code.
    let p = libs();
    let cp1 = compile_ok(libs(), b"(a)"[..].as_ref(), 0);
    let cp2 = compile_ok(libs(), b"(b)"[..].as_ref(), 0);
    unsafe {
        let mdc = (p.c.match_data_create_from_pattern)(cp1.c, std::ptr::null_mut());
        let mdr = (p.r.match_data_create_from_pattern)(cp1.r, std::ptr::null_mut());
        let subj = b"xaxa";
        let a = (p.c.pcre2_match)(cp1.c, subj.as_ptr(), 4, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp1.r, subj.as_ptr(), 4, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b);

        let run = |code_c: Code, code_r: Code, s: &[u8], slen: Sz, off: Sz, opts: u32, label: &str| {
            let mut bc = vec![0u8; 128];
            let mut br = vec![0u8; 128];
            let mut lc: Sz = 128;
            let mut lr: Sz = 128;
            let a = unsafe {
                (p.c.substitute)(
                    code_c, s.as_ptr(), slen, off, opts | o::SUBSTITUTE_MATCHED, mdc,
                    std::ptr::null_mut(), b"Z".as_ptr(), 1, bc.as_mut_ptr(), &mut lc,
                )
            };
            let b = unsafe {
                (p.r.substitute)(
                    code_r, s.as_ptr(), slen, off, opts | o::SUBSTITUTE_MATCHED, mdr,
                    std::ptr::null_mut(), b"Z".as_ptr(), 1, br.as_mut_ptr(), &mut lr,
                )
            };
            assert_eq!(a, b, "substitute-matched rc [{}]", label);
            assert_eq!(lc, lr, "substitute-matched len [{}]", label);
            assert_eq!(bc, br, "substitute-matched buf [{}]", label);
            a
        };

        assert_eq!(run(cp1.c, cp1.r, subj, 4, 0, 0, "same"), 1);
        assert_eq!(run(cp2.c, cp2.r, subj, 4, 0, 0, "diff-pattern"), err::DIFFSUBSPATTERN);
        assert_eq!(run(cp1.c, cp1.r, b"yaya", 4, 0, 0, "diff-subject"), err::DIFFSUBSSUBJECT);
        assert_eq!(run(cp1.c, cp1.r, subj, 4, 1, 0, "diff-offset"), err::DIFFSUBSOFFSET);
        assert_eq!(
            run(cp1.c, cp1.r, subj, 4, 0, o::NOTBOL, "diff-options"),
            err::DIFFSUBSOPTIONS
        );
        // Different subject length is a different subject too.
        run(cp1.c, cp1.r, subj, 3, 0, 0, "diff-subject-length");
        // A match_data produced by pcre2_dfa_match cannot be reused: DFA_UFUNC.
        let mut ws = [0i32; 256];
        let a = (p.c.dfa_match)(cp1.c, subj.as_ptr(), 4, 0, 0, mdc, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
        let b = (p.r.dfa_match)(cp1.r, subj.as_ptr(), 4, 0, 0, mdr, std::ptr::null_mut(), ws.as_mut_ptr(), 256);
        assert_eq!(a, b);
        assert_eq!(run(cp1.c, cp1.r, subj, 4, 0, 0, "dfa-matchdata"), err::DFA_UFUNC);
        // A failed match in the match_data.
        let a = (p.c.pcre2_match)(cp1.c, b"zzz".as_ptr(), 3, 0, 0, mdc, std::ptr::null_mut());
        let b = (p.r.pcre2_match)(cp1.r, b"zzz".as_ptr(), 3, 0, 0, mdr, std::ptr::null_mut());
        assert_eq!(a, b);
        run(cp1.c, cp1.r, b"zzz", 3, 0, 0, "nomatch-matchdata");
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
    }
    free_code_pair(p, cp1);
    free_code_pair(p, cp2);
}

#[test]
fn substitute_unset_group_handling() {
    let p = libs();
    let cp = compile_ok(libs(), b"(a)|(b)", 0);
    for opt in [
        0u32,
        o::SUBSTITUTE_UNSET_EMPTY,
        o::SUBSTITUTE_UNKNOWN_UNSET,
        o::SUBSTITUTE_UNSET_EMPTY | o::SUBSTITUTE_UNKNOWN_UNSET,
        o::SUBSTITUTE_EXTENDED,
        o::SUBSTITUTE_EXTENDED | o::SUBSTITUTE_UNSET_EMPTY,
        o::SUBSTITUTE_EXTENDED | o::SUBSTITUTE_UNKNOWN_UNSET,
    ] {
        for repl in [
            &b"[$1]"[..],
            &b"[$2]"[..],
            &b"[$3]"[..],
            &b"[${1}]"[..],
            &b"[${nope}]"[..],
            &b"[${1:-x}]"[..],
            &b"[${2:+y:z}]"[..],
        ] {
            subst_cmp(
                p,
                &cp,
                b"ab",
                2,
                0,
                opt,
                repl,
                repl.len(),
                128,
                &format!("{:#x}|{:?}", opt, String::from_utf8_lossy(repl)),
            );
        }
    }
    free_code_pair(p, cp);
}

#[test]
fn substitute_case_callout_errors() {
    // PCRE2_ERROR_REPLACECASE is produced when the case callout fails.
    unsafe extern "C" fn cc_fail(
        _input: *const u8,
        _ilen: Sz,
        _output: *mut u8,
        _olen: Sz,
        _ty: i32,
        _data: *mut c_void,
    ) -> Sz {
        PCRE2_UNSET
    }
    unsafe extern "C" fn cc_identity(
        input: *const u8,
        ilen: Sz,
        output: *mut u8,
        olen: Sz,
        _ty: i32,
        _data: *mut c_void,
    ) -> Sz {
        if ilen <= olen {
            unsafe { std::ptr::copy_nonoverlapping(input, output, ilen) };
        }
        ilen
    }
    unsafe extern "C" fn cc_grow(
        input: *const u8,
        ilen: Sz,
        output: *mut u8,
        olen: Sz,
        _ty: i32,
        _data: *mut c_void,
    ) -> Sz {
        // Claim a much bigger output than provided.
        if ilen * 3 <= olen {
            for i in 0..ilen * 3 {
                unsafe { *output.add(i) = *input.add(i % ilen) };
            }
        }
        ilen * 3
    }
    let p = libs();
    let cp = compile_ok(libs(), b"(abc)", 0);
    type CB = unsafe extern "C" fn(*const u8, Sz, *mut u8, Sz, i32, *mut c_void) -> Sz;
    for (name, cb) in [
        ("fail", cc_fail as CB),
        ("identity", cc_identity as CB),
        ("grow", cc_grow as CB),
    ] {
        unsafe {
            let mc = (p.c.match_context_create)(std::ptr::null_mut());
            let mr = (p.r.match_context_create)(std::ptr::null_mut());
            assert_eq!(
                (p.c.set_substitute_case_callout)(mc, Some(cb), std::ptr::null_mut()),
                (p.r.set_substitute_case_callout)(mr, Some(cb), std::ptr::null_mut())
            );
            for repl in [&b"\\U$1"[..], &b"\\L$1"[..], &b"\\u$1"[..], &b"\\l$1"[..], &b"$1"[..]] {
                for cap in [0usize, 4, 128] {
                    let mut bc = vec![0xCDu8; cap + 16];
                    let mut br = vec![0xCDu8; cap + 16];
                    let mut lc: Sz = cap;
                    let mut lr: Sz = cap;
                    let a = (p.c.substitute)(
                        cp.c, b"xabcx".as_ptr(), 5, 0, o::SUBSTITUTE_EXTENDED, std::ptr::null_mut(),
                        mc, repl.as_ptr(), repl.len(), bc.as_mut_ptr(), &mut lc,
                    );
                    let b = (p.r.substitute)(
                        cp.r, b"xabcx".as_ptr(), 5, 0, o::SUBSTITUTE_EXTENDED, std::ptr::null_mut(),
                        mr, repl.as_ptr(), repl.len(), br.as_mut_ptr(), &mut lr,
                    );
                    assert_eq!(
                        (a, lc, &bc),
                        (b, lr, &br),
                        "case callout {} repl {:?} cap {}",
                        name,
                        String::from_utf8_lossy(repl),
                        cap
                    );
                }
            }
            (p.c.match_context_free)(mc);
            (p.r.match_context_free)(mr);
        }
    }
    free_code_pair(p, cp);
}

#[test]
fn substitute_callout_return_values() {
    // A substitute callout may return 0 (accept), >0 (skip) or <0 (abort).
    unsafe extern "C" fn cb0(b: *mut SubstCalloutBlock, _d: *mut c_void) -> i32 {
        let _ = unsafe { (*b).subscount };
        0
    }
    unsafe extern "C" fn cb1(_b: *mut SubstCalloutBlock, _d: *mut c_void) -> i32 {
        1
    }
    unsafe extern "C" fn cbneg(_b: *mut SubstCalloutBlock, _d: *mut c_void) -> i32 {
        -77
    }
    unsafe extern "C" fn cb_alt(b: *mut SubstCalloutBlock, _d: *mut c_void) -> i32 {
        if unsafe { (*b).subscount } % 2 == 0 { 1 } else { 0 }
    }
    let p = libs();
    let cp = compile_ok(libs(), b"a", 0);
    type CB = unsafe extern "C" fn(*mut SubstCalloutBlock, *mut c_void) -> i32;
    for (name, cb) in [
        ("accept", cb0 as CB),
        ("skip", cb1 as CB),
        ("abort", cbneg as CB),
        ("alternate", cb_alt as CB),
    ] {
        unsafe {
            let mc = (p.c.match_context_create)(std::ptr::null_mut());
            let mr = (p.r.match_context_create)(std::ptr::null_mut());
            assert_eq!(
                (p.c.set_substitute_callout)(mc, Some(cb), std::ptr::null_mut()),
                (p.r.set_substitute_callout)(mr, Some(cb), std::ptr::null_mut())
            );
            for opt in [0u32, o::SUBSTITUTE_GLOBAL] {
                let mut bc = vec![0xCDu8; 160];
                let mut br = vec![0xCDu8; 160];
                let mut lc: Sz = 128;
                let mut lr: Sz = 128;
                let a = (p.c.substitute)(
                    cp.c, b"aXaXa".as_ptr(), 5, 0, opt, std::ptr::null_mut(), mc, b"Z".as_ptr(), 1,
                    bc.as_mut_ptr(), &mut lc,
                );
                let b = (p.r.substitute)(
                    cp.r, b"aXaXa".as_ptr(), 5, 0, opt, std::ptr::null_mut(), mr, b"Z".as_ptr(), 1,
                    br.as_mut_ptr(), &mut lr,
                );
                assert_eq!((a, lc, &bc), (b, lr, &br), "subst callout {} opt {:#x}", name, opt);
            }
            (p.c.match_context_free)(mc);
            (p.r.match_context_free)(mr);
        }
    }
    free_code_pair(p, cp);
}

// ===========================================================================
// Phase B: valid-path randomized substitution
// ===========================================================================

const SUBST_OPTS: &[u32] = &[
    0,
    o::SUBSTITUTE_GLOBAL,
    o::SUBSTITUTE_EXTENDED,
    o::SUBSTITUTE_GLOBAL | o::SUBSTITUTE_EXTENDED,
    o::SUBSTITUTE_LITERAL,
    o::SUBSTITUTE_LITERAL | o::SUBSTITUTE_GLOBAL,
    o::SUBSTITUTE_UNSET_EMPTY,
    o::SUBSTITUTE_UNKNOWN_UNSET | o::SUBSTITUTE_UNSET_EMPTY,
    o::SUBSTITUTE_OVERFLOW_LENGTH,
    o::SUBSTITUTE_REPLACEMENT_ONLY,
    o::SUBSTITUTE_REPLACEMENT_ONLY | o::SUBSTITUTE_GLOBAL,
    o::SUBSTITUTE_GLOBAL | o::SUBSTITUTE_EXTENDED | o::SUBSTITUTE_UNSET_EMPTY,
    o::NOTBOL,
    o::NOTEOL,
    o::NOTEMPTY,
    o::NOTEMPTY_ATSTART,
    o::ANCHORED,
    o::ENDANCHORED,
    o::NO_UTF_CHECK,
];

static SUBST_PATTERNS: &[(&[u8], u32)] = &[
    (b"a", 0),
    (b"a+", 0),
    (b"(a)(b)?", 0),
    (b"(?<x>a)(?<y>b)?", 0),
    (b"", 0),
    (b"\\b", 0),
    (b"^", o::MULTILINE),
    (b"$", o::MULTILINE),
    (b"[a-c]", 0),
    (b"(?i)abc", 0),
    (b".", o::DOTALL),
    (b"\\R", 0),
    (b"x*", 0),
    (b"(a|bb|ccc)", 0),
    (b"\\p{L}", o::UCP),
    (b"\\X", o::UTF),
    (b"(?:a)(?=b)", 0),
    (b"(?<=a)b", 0),
];

static REPLACEMENTS: &[&[u8]] = &[
    b"",
    b"X",
    b"$0",
    b"$1",
    b"[$1|$2]",
    b"${1}${2}",
    b"$*",
    b"${x}-${y}",
    b"\\U$1\\E$2",
    b"\\u$0",
    b"\\l$0",
    b"${1:+yes:no}",
    b"${1:-dflt}",
    b"\\x{41}\\o{102}",
    b"a$1b$2c",
    b"$$",
    b"\\$",
    b"\\\\",
];

#[test]
fn substitute_randomized_matrix() {
    let p = libs();
    let mut rng = Rng::new(0x5B57_1175_E3D0);
    let alphabet = b"abcXY\n\r\t \x00\xC3\xA9";
    for (pat, popts) in SUBST_PATTERNS {
        let cp = match compile_both(p, pat, pat.len(), *popts, std::ptr::null_mut(), std::ptr::null_mut(), "rnd")
        {
            Ok(cp) => cp,
            Err(_) => continue,
        };
        for opts in SUBST_OPTS {
            for repl in REPLACEMENTS {
                for _ in 0..8 {
                    let n = rng.below(14);
                    let subj: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
                    let cap = [0usize, 1, 4, 16, 256][rng.below(5)];
                    let slen = if rng.bool() && !subj.contains(&0) {
                        PCRE2_ZERO_TERMINATED
                    } else {
                        subj.len()
                    };
                    let rlen = if rng.bool() { PCRE2_ZERO_TERMINATED } else { repl.len() };
                    let start = if subj.is_empty() { 0 } else { rng.below(subj.len() + 1) };
                    let label = format!(
                        "pat={:?} opts={:#x} repl={:?} subj={:?} cap={} slen={} rlen={} start={}",
                        String::from_utf8_lossy(pat),
                        opts,
                        String::from_utf8_lossy(repl),
                        subj,
                        cap,
                        slen as i64,
                        rlen as i64,
                        start
                    );
                    subst_cmp(p, &cp, &subj, slen, start, *opts, repl, rlen, cap, &label);
                }
            }
        }
        free_code_pair(p, cp);
    }
}
