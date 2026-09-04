//! Phase B — compile + match differential tests across the option axes.
mod common;

use common::diff::*;
use common::*;

const SEED: u64 = 0xBEEF_0102_0304;

/// A broad corpus of patterns covering the constructs the C compiler branches on.
pub const PATTERNS: &[&str] = &[
    // literals & simple
    "", "a", "abc", "a|b", "a|b|c|d|e", "^abc$", ".", ".*", ".+", ".?",
    // quantifiers
    "a*", "a+", "a?", "a{3}", "a{2,5}", "a{0,}", "a{1,}", "a{0,1}", "a{255}",
    "a*?", "a+?", "a??", "a{2,5}?", "a*+", "a++", "a?+", "a{2,5}+",
    // groups
    "(a)", "(a)(b)", "(a(b(c)))", "(?:a)", "(?:a|b)", "(?>a|ab)", "(?|(a)|(b))",
    "((((((((((a))))))))))",
    // named groups
    "(?<n>a)", "(?<a>x)(?<b>y)", "(?'n'a)", "(?P<n>a)", "(?<n>a)\\k<n>",
    "(?<n>a)(?<m>b)\\k<n>\\k<m>",
    // backreferences
    "(a)\\1", "(a)(b)\\2\\1", "(a)\\g{1}", "(a)\\g1", "(a)\\g{-1}",
    // classes
    "[abc]", "[^abc]", "[a-z]", "[a-z0-9]", "[^a-z]", "[]]", "[^]]", "[-a]",
    "[a-]", "[\\d]", "[\\D]", "[\\s\\S]", "[\\w]", "[[:alpha:]]", "[[:^digit:]]",
    "[[:alpha:][:digit:]]", "[\\x00-\\xff]", "[a-c-e]", "[\\Q-]\\E]",
    // unicode properties
    "\\p{L}", "\\p{Lu}", "\\P{L}", "\\p{Greek}", "\\p{Han}", "\\p{Any}",
    "\\p{Xan}", "\\p{Xps}", "\\p{Xsp}", "\\p{Xwd}", "\\p{Xuc}",
    "[\\p{L}\\p{N}]", "[^\\p{L}]", "\\p{Cc}", "\\p{Zs}",
    // escapes
    "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\h", "\\H", "\\v", "\\V",
    "\\R", "\\X", "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G", "\\K", "\\N",
    "\\Qa+b\\E", "\\x41", "\\x{1F600}", "\\101", "\\o{101}", "\\cA", "\\e",
    "\\t\\n\\r\\f\\a",
    // anchors & multiline
    "^a", "a$", "^a$", "(?m)^a$", "(?s).", "(?i)ABC", "(?x) a b c",
    "(?xx) a b c", "(?U)a*", "(?J)(?<n>a)(?<n>b)",
    // lookaround
    "(?=a)", "(?!a)", "(?<=a)b", "(?<!a)b", "a(?=b)c*", "(?<=ab|cd)e",
    "(?<=a{2,4})b", "(?*a)", "(?<*a)",
    // conditionals
    "(a)?(?(1)b|c)", "(?(?=a)b|c)", "(?<n>a)?(?(<n>)b|c)", "(?(DEFINE)(?<x>a))(?&x)",
    // recursion / subroutines
    "(a)(?1)", "(?R)?", "a(?R)?b", "(?<n>a)(?&n)", "(?1)(a)", "(a(?2)?)(b)",
    // atomic / possessive
    "(?>a*)b", "a*+b", "[a-z]*+", "(?>(a|ab))c",
    // verbs
    "(*FAIL)", "(*ACCEPT)", "a(*SKIP)b", "a(*PRUNE)b", "a(*THEN)b",
    "(*MARK:m)a", "(*:m)a", "(*COMMIT)a", "a(*SKIP:x)b|c",
    // options in pattern
    "(*UTF)a", "(*UCP)a", "(*CR)a$", "(*LF)a$", "(*CRLF)a$", "(*ANY)a$",
    "(*ANYCRLF)a$", "(*NUL)a$", "(*BSR_ANYCRLF)\\R", "(*BSR_UNICODE)\\R",
    "(*LIMIT_MATCH=100)a", "(*LIMIT_DEPTH=100)a", "(*LIMIT_HEAP=100)a",
    "(*NO_AUTO_POSSESS)a*", "(*NO_START_OPT)a", "(*NOTEMPTY)a*",
    "(*NOTEMPTY_ATSTART)a*", "(*NO_DOTSTAR_ANCHOR).*a", "(*NO_JIT)a",
    // callouts
    "(?C)a", "(?C1)a", "(?C255)a", "(?C`txt`)a", "(?C{txt})a",
    // wide / utf patterns
    "\u{00e9}", "\u{20ac}", "\u{1F600}", "[\u{00e9}\u{20ac}]", "\u{00e9}+",
    "[^\u{00e9}]", "\\x{e9}", "[\\x{100}-\\x{200}]",
    // longer realistic patterns
    "^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+)@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z]{2,}$",
    "^(\\d{1,3})\\.(\\d{1,3})\\.(\\d{1,3})\\.(\\d{1,3})$",
    "(\\w+)\\s+(\\w+)\\s+(\\w+)",
    "<(\\w+)[^>]*>(.*?)</\\1>",
    "(?i)\\b(\\w+)\\b\\s+\\1\\b",
    "a(?:b|c|d|e|f|g|h|i|j|k){2,4}z",
    "((a+)*)+b",
    "(?:(?:(?:(?:(?:a))))){1,2}",
];

/// Subjects to try against every pattern.
pub const SUBJECTS: &[&str] = &[
    "", "a", "b", "abc", "aaa", "aaaa", "abcabc", "xxabcyy", "A", "ABC",
    "a\nb", "a\r\nb", "a\rb", "a\0b", " a b c ", "\t\n", "aB", "0123456789",
    "the quick brown fox", "hello hello world", "<b>bold</b>",
    "1.2.3.4", "255.255.255.255", "user@example.com",
    "\u{00e9}", "\u{00e9}\u{00e9}", "\u{20ac}5", "\u{1F600}!",
    "caf\u{00e9} au lait", "\u{03B1}\u{03B2}\u{03B3}", "\u{4E00}\u{4E8C}",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
    "abababababababab",
];

fn subj_bytes() -> Vec<&'static [u8]> {
    SUBJECTS.iter().map(|s| s.as_bytes()).collect()
}

// =========================================================== CONFIGS rows 1..
/// Row: default options, every pattern, both engines, all subjects.
#[test]
fn compile_match_default_options() {
    let subjects = subj_bytes();
    unsafe {
        for pat in PATTERNS {
            diff_compile_and_match(
                pat.as_bytes(),
                &CompileCfg::new(0),
                &subjects,
                &MatchCfg::new(0),
                &[Engine::Interpreter, Engine::Dfa],
                &format!("default {:?}", pat),
            );
        }
    }
}

/// Row: each single compile option in isolation, over the whole corpus.
#[test]
fn compile_match_each_single_compile_option() {
    let subjects = subj_bytes();
    let opts: [(&str, u32); 26] = [
        ("ANCHORED", PCRE2_ANCHORED),
        ("ENDANCHORED", PCRE2_ENDANCHORED),
        ("ALLOW_EMPTY_CLASS", PCRE2_ALLOW_EMPTY_CLASS),
        ("ALT_BSUX", PCRE2_ALT_BSUX),
        ("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT),
        ("CASELESS", PCRE2_CASELESS),
        ("DOLLAR_ENDONLY", PCRE2_DOLLAR_ENDONLY),
        ("DOTALL", PCRE2_DOTALL),
        ("DUPNAMES", PCRE2_DUPNAMES),
        ("EXTENDED", PCRE2_EXTENDED),
        ("EXTENDED_MORE", PCRE2_EXTENDED_MORE),
        ("FIRSTLINE", PCRE2_FIRSTLINE),
        ("MATCH_UNSET_BACKREF", PCRE2_MATCH_UNSET_BACKREF),
        ("MULTILINE", PCRE2_MULTILINE),
        ("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE),
        ("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS),
        ("NO_DOTSTAR_ANCHOR", PCRE2_NO_DOTSTAR_ANCHOR),
        ("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE),
        ("UCP", PCRE2_UCP),
        ("UNGREEDY", PCRE2_UNGREEDY),
        ("UTF", PCRE2_UTF),
        ("ALT_CIRCUMFLEX", PCRE2_ALT_CIRCUMFLEX),
        ("ALT_VERBNAMES", PCRE2_ALT_VERBNAMES),
        ("USE_OFFSET_LIMIT", PCRE2_USE_OFFSET_LIMIT),
        ("LITERAL", PCRE2_LITERAL),
        ("ALT_EXTENDED_CLASS", PCRE2_ALT_EXTENDED_CLASS),
    ];
    unsafe {
        for (name, opt) in opts {
            for pat in PATTERNS {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(opt),
                    &subjects,
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("{} {:?}", name, pat),
                );
            }
        }
    }
}

/// Row: UTF and UTF|UCP — the unicode code paths.
#[test]
fn compile_match_utf_and_ucp() {
    let subjects = subj_bytes();
    unsafe {
        for (name, opt) in [
            ("UTF", PCRE2_UTF),
            ("UTF|UCP", PCRE2_UTF | PCRE2_UCP),
            ("UTF|CASELESS", PCRE2_UTF | PCRE2_CASELESS),
            ("UTF|UCP|CASELESS", PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS),
            ("UTF|MATCH_INVALID_UTF", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF),
        ] {
            for pat in PATTERNS {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(opt),
                    &subjects,
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("{} {:?}", name, pat),
                );
            }
        }
    }
}

/// Row: every newline convention x every bsr convention.
#[test]
fn compile_match_newline_x_bsr() {
    let nl_subjects: Vec<&[u8]> = vec![
        b"a\nb", b"a\rb", b"a\r\nb", b"a\0b", b"a\x0bb", b"a\x0cb",
        b"a\xc2\x85b", b"a\xe2\x80\xa8b", b"\n", b"\r", b"\r\n", b"",
        b"line1\nline2\nline3",
    ];
    let pats: [&str; 12] = [
        "a$", "^a", "(?m)^a$", "(?m)a$", "\\R", "\\R+", ".", ".*",
        "\\N", "a.b", "^", "$",
    ];
    unsafe {
        for nl in ALL_NEWLINES {
            for bsr in [BSR_UNICODE, BSR_ANYCRLF] {
                for pat in pats {
                    let cfg = CompileCfg::new(0).newline(nl).bsr(bsr);
                    diff_compile_and_match(
                        pat.as_bytes(),
                        &cfg,
                        &nl_subjects,
                        &MatchCfg::new(0),
                        &[Engine::Interpreter, Engine::Dfa],
                        &format!("nl={} bsr={} {:?}", nl, bsr, pat),
                    );
                    // and with MULTILINE, which changes the newline handling
                    let cfg = CompileCfg::new(PCRE2_MULTILINE).newline(nl).bsr(bsr);
                    diff_compile_and_match(
                        pat.as_bytes(),
                        &cfg,
                        &nl_subjects,
                        &MatchCfg::new(0),
                        &[Engine::Interpreter, Engine::Dfa],
                        &format!("ML nl={} bsr={} {:?}", nl, bsr, pat),
                    );
                }
            }
        }
    }
}

/// Row: every `PCRE2_EXTRA_*` flag.
#[test]
fn compile_match_each_extra_option() {
    let subjects = subj_bytes();
    let extras: [(&str, u32); 17] = [
        ("ALLOW_SURROGATE_ESCAPES", PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
        ("BAD_ESCAPE_IS_LITERAL", PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
        ("MATCH_WORD", PCRE2_EXTRA_MATCH_WORD),
        ("MATCH_LINE", PCRE2_EXTRA_MATCH_LINE),
        ("ESCAPED_CR_IS_LF", PCRE2_EXTRA_ESCAPED_CR_IS_LF),
        ("ALT_BSUX", PCRE2_EXTRA_ALT_BSUX),
        ("ALLOW_LOOKAROUND_BSK", PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ("CASELESS_RESTRICT", PCRE2_EXTRA_CASELESS_RESTRICT),
        ("ASCII_BSD", PCRE2_EXTRA_ASCII_BSD),
        ("ASCII_BSS", PCRE2_EXTRA_ASCII_BSS),
        ("ASCII_BSW", PCRE2_EXTRA_ASCII_BSW),
        ("ASCII_POSIX", PCRE2_EXTRA_ASCII_POSIX),
        ("ASCII_DIGIT", PCRE2_EXTRA_ASCII_DIGIT),
        ("PYTHON_OCTAL", PCRE2_EXTRA_PYTHON_OCTAL),
        ("NO_BS0", PCRE2_EXTRA_NO_BS0),
        ("NEVER_CALLOUT", PCRE2_EXTRA_NEVER_CALLOUT),
        ("TURKISH_CASING", PCRE2_EXTRA_TURKISH_CASING),
    ];
    unsafe {
        for (name, extra) in extras {
            for pat in PATTERNS {
                // TURKISH_CASING requires UTF+UCP; test it that way
                let base = if extra == PCRE2_EXTRA_TURKISH_CASING {
                    PCRE2_UTF | PCRE2_UCP
                } else {
                    0
                };
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(base).extra(extra),
                    &subjects,
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("EXTRA_{} {:?}", name, pat),
                );
            }
        }
    }
}

/// Row: every match-time option.
#[test]
fn compile_match_each_match_option() {
    let subjects = subj_bytes();
    let mopts: [(&str, u32); 9] = [
        ("NOTBOL", PCRE2_NOTBOL),
        ("NOTEOL", PCRE2_NOTEOL),
        ("NOTEMPTY", PCRE2_NOTEMPTY),
        ("NOTEMPTY_ATSTART", PCRE2_NOTEMPTY_ATSTART),
        ("PARTIAL_SOFT", PCRE2_PARTIAL_SOFT),
        ("PARTIAL_HARD", PCRE2_PARTIAL_HARD),
        ("ANCHORED", PCRE2_ANCHORED),
        ("ENDANCHORED", PCRE2_ENDANCHORED),
        ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
    ];
    unsafe {
        for (name, mo) in mopts {
            for pat in PATTERNS {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0),
                    &subjects,
                    &MatchCfg::new(mo),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("match_{} {:?}", name, pat),
                );
            }
        }
    }
}

/// Row: DFA-specific options.
///
/// NOTE: `PCRE2_DFA_RESTART` is deliberately NOT used with a fresh workspace
/// here. PCRE2 documents that it may only be passed with a workspace left
/// behind by a previous PARTIAL match, and its sanity check in
/// `pcre2_dfa_match.c:3453` is not sufficient to catch a zeroed workspace —
/// the C library itself segfaults on such a call. Verified: the C `.so` and the
/// Rust `.so` crash identically, so it is invalid usage rather than a
/// divergence. `dfa_restart_after_partial` below exercises it correctly.
#[test]
fn dfa_shortest_and_partial() {
    let subjects = subj_bytes();
    unsafe {
        for (name, mo) in [
            ("DFA_SHORTEST", PCRE2_DFA_SHORTEST),
            ("DFA_SHORTEST|PARTIAL_SOFT", PCRE2_DFA_SHORTEST | PCRE2_PARTIAL_SOFT),
            ("DFA_SHORTEST|PARTIAL_HARD", PCRE2_DFA_SHORTEST | PCRE2_PARTIAL_HARD),
        ] {
            for pat in PATTERNS {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0),
                    &subjects,
                    &MatchCfg::new(mo),
                    &[Engine::Dfa],
                    &format!("{} {:?}", name, pat),
                );
            }
        }
    }
}

/// Row: `PCRE2_DFA_RESTART` used the documented way — a PARTIAL match first,
/// then a restart on the remainder with the SAME workspace. The workspace
/// contents left behind by the first call are themselves compared, since a
/// divergence there changes the restarted match.
#[test]
fn dfa_restart_after_partial() {
    let pats: [&str; 10] = [
        "abcd", "ab+cd", "\\d{4}", "^abcdef$", "(ab)(cd)", "a.c.e",
        "[a-z]{5}", "ab|abcd", "(?:ab){3}", "\\w+@\\w+",
    ];
    // split each subject so the first half is a partial match
    let subjects: [&[u8]; 8] = [
        b"abcd", b"abbbcd", b"1234", b"abcdef", b"abcd", b"abcde",
        b"abcde", b"ab@cd",
    ];
    unsafe {
        let (c, r) = both();
        for pat in pats {
            let cc = compile_in(c, pat.as_bytes(), pat.len(), &CompileCfg::new(0));
            let rr = compile_in(r, pat.as_bytes(), pat.len(), &CompileCfg::new(0));
            if cc.code.is_null() {
                continue;
            }
            for subj in subjects {
                for split in 1..subj.len() {
                    let mut cws = [0i32; 1000];
                    let mut rws = [0i32; 1000];
                    let cmd = (c.match_data_create)(10, std::ptr::null_mut());
                    let rmd = (r.match_data_create)(10, std::ptr::null_mut());

                    // pass 1: PARTIAL_HARD over subject[..split]
                    let crc1 = (c.dfa_match)(
                        cc.code, subj.as_ptr(), split, 0, PCRE2_PARTIAL_HARD,
                        cmd, std::ptr::null_mut(), cws.as_mut_ptr(), cws.len(),
                    );
                    let rrc1 = (r.dfa_match)(
                        rr.code, subj.as_ptr(), split, 0, PCRE2_PARTIAL_HARD,
                        rmd, std::ptr::null_mut(), rws.as_mut_ptr(), rws.len(),
                    );
                    assert_eq!(
                        crc1, rrc1,
                        "{:?}: pass-1 rc split={} subj={:?}",
                        pat, split, String::from_utf8_lossy(subj)
                    );

                    if crc1 == ERR_PARTIAL {
                        // the workspace state itself must agree
                        assert_eq!(
                            cws[0], rws[0],
                            "{:?}: workspace[0] differs split={}", pat, split
                        );
                        assert_eq!(
                            cws[1], rws[1],
                            "{:?}: workspace[1] differs split={}", pat, split
                        );
                        let n = 2 + (cws[1].max(0) as usize) * 3;
                        let n = n.min(cws.len());
                        assert_eq!(
                            &cws[..n], &rws[..n],
                            "{:?}: live workspace differs split={}", pat, split
                        );

                        // pass 2: restart on the remainder
                        let crc2 = (c.dfa_match)(
                            cc.code, subj.as_ptr(), subj.len(), split,
                            PCRE2_DFA_RESTART, cmd, std::ptr::null_mut(),
                            cws.as_mut_ptr(), cws.len(),
                        );
                        let rrc2 = (r.dfa_match)(
                            rr.code, subj.as_ptr(), subj.len(), split,
                            PCRE2_DFA_RESTART, rmd, std::ptr::null_mut(),
                            rws.as_mut_ptr(), rws.len(),
                        );
                        assert_eq!(
                            crc2, rrc2,
                            "{:?}: restart rc split={} subj={:?}",
                            pat, split, String::from_utf8_lossy(subj)
                        );
                        if crc2 > 0 {
                            let cov = (c.get_ovector_pointer)(cmd);
                            let rov = (r.get_ovector_pointer)(rmd);
                            let n = (crc2 as usize) * 2;
                            assert_eq!(
                                std::slice::from_raw_parts(cov, n),
                                std::slice::from_raw_parts(rov, n),
                                "{:?}: restart ovector split={}", pat, split
                            );
                        }
                    }
                    (c.match_data_free)(cmd);
                    (r.match_data_free)(rmd);
                }
            }
        }
    }
}

/// Row: DFA workspace sizes, including ones too small (PCRE2_ERROR_DFA_WSSIZE)
/// and the minimum legal size.
#[test]
fn dfa_workspace_sizes() {
    let pats: [&str; 8] = [
        "a", "(a)(b)(c)", "a|b|c|d|e|f|g|h", "(?:a|b){1,20}",
        "[a-z]+[0-9]+", "((((a))))", "a{1,50}", "(a|bb|ccc|dddd)+",
    ];
    let subjects: [&[u8]; 5] = [b"", b"a", b"abc", b"aaaaaaaaaa", b"abc123"];
    unsafe {
        let (c, r) = both();
        for pat in pats {
            let cc = compile_in(c, pat.as_bytes(), pat.len(), &CompileCfg::new(0));
            let rr = compile_in(r, pat.as_bytes(), pat.len(), &CompileCfg::new(0));
            if cc.code.is_null() {
                continue;
            }
            for subj in subjects {
                for wsz in [0usize, 1, 2, 3, 4, 5, 10, 20, 21, 22, 100, 1000] {
                    let mut cws = vec![0i32; wsz.max(1)];
                    let mut rws = vec![0i32; wsz.max(1)];
                    let cmd = (c.match_data_create)(10, std::ptr::null_mut());
                    let rmd = (r.match_data_create)(10, std::ptr::null_mut());
                    let crc = (c.dfa_match)(
                        cc.code, subj.as_ptr(), subj.len(), 0, 0, cmd,
                        std::ptr::null_mut(), cws.as_mut_ptr(), wsz,
                    );
                    let rrc = (r.dfa_match)(
                        rr.code, subj.as_ptr(), subj.len(), 0, 0, rmd,
                        std::ptr::null_mut(), rws.as_mut_ptr(), wsz,
                    );
                    assert_eq!(
                        crc, rrc,
                        "{:?}: dfa wscount={} subj={:?} rc differs",
                        pat, wsz, String::from_utf8_lossy(subj)
                    );
                    if crc > 0 {
                        let cov = (c.get_ovector_pointer)(cmd);
                        let rov = (r.get_ovector_pointer)(rmd);
                        let n = (crc as usize) * 2;
                        assert_eq!(
                            std::slice::from_raw_parts(cov, n),
                            std::slice::from_raw_parts(rov, n),
                            "{:?}: dfa wscount={} ovector", pat, wsz
                        );
                    }
                    (c.match_data_free)(cmd);
                    (r.match_data_free)(rmd);
                }
            }
        }
    }
}

/// Row: ovector sizes 0,1,2,...,and oversized — the "unused entries set to
/// UNSET" and "not enough room" paths.
#[test]
fn ovector_sizes() {
    let pats: [&str; 8] = [
        "(a)(b)(c)", "a", "(a)", "(a)(b)(c)(d)(e)(f)", "(?<n>a)(?<m>b)",
        "(a)|(b)", "((a)(b))", "(a)(?:x)?(b)",
    ];
    let subjects: Vec<&[u8]> = vec![b"abc", b"abcdef", b"a", b"", b"xabcx"];
    unsafe {
        for pat in pats {
            for ovec in [0u32, 1, 2, 3, 4, 8, 100] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0),
                    &subjects,
                    &MatchCfg::new(0).ovec(ovec),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("ovec={} {:?}", ovec, pat),
                );
            }
        }
    }
}

/// Row: subject length variants — explicit length vs PCRE2_ZERO_TERMINATED,
/// and every start offset in the subject.
#[test]
fn subject_lengths_and_all_start_offsets() {
    let pats: [&str; 10] = [
        "a", "a+", "^a", "a$", "(a)(b)", "\\ba\\b", "(?<=x)a", "a(?=b)",
        ".*", "\\G a",
    ];
    let subjects: [&[u8]; 6] = [b"", b"a", b"ab", b"aab", b"xaybz", b"aaaa"];
    unsafe {
        for pat in pats {
            let (cc, rr) =
                compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in subjects {
                // every legal start offset
                for start in 0..=subj.len() {
                    for engine in [Engine::Interpreter, Engine::Dfa] {
                        assert_match_eq(
                            &cc, &rr, subj, subj.len(), start,
                            &MatchCfg::new(0), engine,
                            &format!("{:?} start={}", pat, start),
                        );
                    }
                }
                // PCRE2_ZERO_TERMINATED (needs a NUL-terminated buffer)
                let mut z = subj.to_vec();
                z.push(0);
                for engine in [Engine::Interpreter, Engine::Dfa] {
                    assert_match_eq(
                        &cc, &rr, &z, PCRE2_ZERO_TERMINATED, 0,
                        &MatchCfg::new(0), engine,
                        &format!("{:?} zero-terminated", pat),
                    );
                }
            }
        }
    }
}

/// Row: patterns compiled with the library's own `pcre2_maketables()` output
/// rather than the built-in default tables.
#[test]
fn own_character_tables() {
    let subjects = subj_bytes();
    unsafe {
        for pat in PATTERNS {
            diff_compile_and_match(
                pat.as_bytes(),
                &CompileCfg::new(0).own_tables(),
                &subjects,
                &MatchCfg::new(0),
                &[Engine::Interpreter, Engine::Dfa],
                &format!("own_tables {:?}", pat),
            );
        }
    }
}

/// Row: match/depth/heap limits at values that force the limit errors.
#[test]
fn match_depth_heap_limits() {
    // catastrophic-backtracking patterns so the limits actually bite
    let pats: [&str; 6] = [
        "(a+)+b", "(a|aa)+c", "((a)*)*b", "a{1,1000}b", "(?:a?){1,100}b",
        "(\\w+\\s?)*$",
    ];
    let subjects: Vec<&[u8]> = vec![
        b"aaaaaaaaaaaaaaaaaaaaaaaa",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!",
        b"aaaa",
        b"the quick brown fox jumps over",
    ];
    unsafe {
        for pat in pats {
            for lim in [1u32, 2, 10, 100, 1000, 10000] {
                for which in 0..3 {
                    let mcfg = match which {
                        0 => MatchCfg::new(0).match_limit(lim),
                        1 => MatchCfg::new(0).depth_limit(lim),
                        _ => MatchCfg::new(0).heap_limit(lim),
                    };
                    diff_compile_and_match(
                        pat.as_bytes(),
                        &CompileCfg::new(0),
                        &subjects,
                        &mcfg,
                        &[Engine::Interpreter, Engine::Dfa],
                        &format!("limit{}={} {:?}", which, lim, pat),
                    );
                }
            }
        }
    }
}

/// Row: USE_OFFSET_LIMIT + `pcre2_set_offset_limit`.
#[test]
fn offset_limit() {
    let pats: [&str; 4] = ["a", "b+", "(?<=x)y", "\\d+"];
    let subjects: Vec<&[u8]> = vec![b"xxaxxaxx", b"bbbb", b"xyxy", b"12ab34"];
    unsafe {
        for pat in pats {
            for lim in [0usize, 1, 2, 4, 8, 100, PCRE2_UNSET] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(PCRE2_USE_OFFSET_LIMIT),
                    &subjects,
                    &MatchCfg::new(0).offset_limit(lim),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("offset_limit={} {:?}", lim, pat),
                );
                // without USE_OFFSET_LIMIT the same call must be rejected
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0),
                    &subjects,
                    &MatchCfg::new(0).offset_limit(lim),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("no-USE_OFFSET_LIMIT offset_limit={} {:?}", lim, pat),
                );
            }
        }
    }
}

/// Row: max_varlookbehind and parens_nest_limit at their boundaries.
#[test]
fn varlookbehind_and_parens_nest_limits() {
    unsafe {
        let lb_pats: [&str; 6] = [
            "(?<=a)b", "(?<=ab)c", "(?<=a{2,4})b", "(?<=a|bb|ccc)d",
            "(?<=(?:ab){1,10})c", "(?<!a{1,20})b",
        ];
        for pat in lb_pats {
            for n in [0u32, 1, 2, 3, 4, 10, 20, 255, 65535] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0).varlookbehind(n),
                    &[b"ab", b"aab", b"abc"],
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("varlookbehind={} {:?}", n, pat),
                );
            }
        }
        // nested parentheses at increasing depth against increasing limits
        for depth in [1usize, 2, 5, 10, 20, 50] {
            let pat = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
            for limit in [0u32, 1, 2, 5, 10, 20, 50, 250] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0).parens_nest(limit),
                    &[b"a"],
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("parens_nest={} depth={}", limit, depth),
                );
            }
        }
    }
}

/// Row: max_pattern_length / max_pattern_compiled_length boundaries.
#[test]
fn pattern_length_limits() {
    unsafe {
        for pat in ["a", "abcdefghij", "(a)(b)(c)(d)(e)", "a{1,100}b*c+"] {
            for n in [0usize, 1, 2, 5, 10, 11, 100, usize::MAX] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0).max_len(n),
                    &[b"abc"],
                    &MatchCfg::new(0),
                    &[Engine::Interpreter],
                    &format!("max_pattern_length={} {:?}", n, pat),
                );
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0).max_compiled(n),
                    &[b"abc"],
                    &MatchCfg::new(0),
                    &[Engine::Interpreter],
                    &format!("max_compiled={} {:?}", n, pat),
                );
            }
        }
    }
}

/// Row: `pcre2_set_optimize` flag values (including out-of-range ones).
#[test]
fn optimize_flags() {
    let subjects = subj_bytes();
    unsafe {
        // PCRE2_OPTIMIZATION_NONE=0, FULL=1, AUTO_POSSESS=2/3, START_OPTIMIZE=4/5
        for n in [0u32, 1, 2, 3, 4, 5, 6, 7, 0xFFFF_FFFF] {
            for pat in ["a*b", "(a|b)*c", "^abc", ".*x", "[a-z]+9"] {
                diff_compile_and_match(
                    pat.as_bytes(),
                    &CompileCfg::new(0).optimize(n),
                    &subjects,
                    &MatchCfg::new(0),
                    &[Engine::Interpreter, Engine::Dfa],
                    &format!("optimize={} {:?}", n, pat),
                );
            }
        }
    }
}

/// Row: randomized patterns — property-style, fixed seed. Catches
/// value-dependent codegen divergence the hand-written corpus misses.
#[test]
fn randomized_patterns() {
    let mut g = Rng::new(SEED);
    // pattern fragments that combine into (mostly) valid regexes
    let frags: [&str; 46] = [
        "a", "b", "c", "z", "0", "9", ".", "\\d", "\\w", "\\s", "\\S", "\\D",
        "[a-z]", "[^a-z]", "[abc]", "[[:digit:]]", "\\p{L}", "\\p{Nd}",
        "(a)", "(?:b)", "(?<n>c)", "(?>d)", "a|b", "(a|b)",
        "*", "+", "?", "{2}", "{1,3}", "*?", "+?", "??", "*+", "++",
        "^", "$", "\\b", "\\B", "\\A", "\\z", "\\R", "\\X", "\\K",
        "(?=a)", "(?!a)", "(?<=a)",
    ];
    let opt_pool: [u32; 10] = [
        0,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_EXTENDED,
        PCRE2_UNGREEDY,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_NO_AUTO_POSSESS,
    ];
    let subjects: Vec<&[u8]> = vec![
        b"", b"a", b"abc", b"aaa", b"abcabc", b"a\nb", b"0a9z",
        b"\xc3\xa9x", b"the fox", b"aaaaaaaaab",
    ];
    unsafe {
        for i in 0..4000 {
            let n = g.range(1, 7);
            let mut pat = String::new();
            for _ in 0..n {
                pat.push_str(g.pick(&frags));
            }
            let opts = *g.pick(&opt_pool);
            diff_compile_and_match(
                pat.as_bytes(),
                &CompileCfg::new(opts),
                &subjects,
                &MatchCfg::new(0),
                &[Engine::Interpreter, Engine::Dfa],
                &format!("rand#{} opts={:#x} {:?}", i, opts, pat),
            );
        }
    }
}

/// Row: randomized RAW BYTE patterns — pure fuzzing of the compiler, including
/// invalid syntax (compile must fail identically) and invalid UTF-8.
#[test]
fn randomized_raw_byte_patterns() {
    let mut g = Rng::new(SEED ^ 0xABCD);
    // bias towards metacharacters so the parser is stressed
    let alpha: &[u8] = b"ab019()[]{}|*+?.^$\\-,:!<>=&#'\"`/pPQEdswWSDbBAzZGKXRNhHvVcxou \t\n\r\x00\xc3\xa9\xff\x80";
    unsafe {
        for i in 0..15000 {
            let n = g.range(0, 20) as usize;
            let pat = g.bytes_from(n, alpha);
            for opts in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_EXTENDED] {
                // compile-only comparison (patterns are mostly invalid)
                let (cc, rr) = compile_both(
                    &pat,
                    pat.len(),
                    &CompileCfg::new(opts),
                    &format!("raw#{} opts={:#x} {:02x?}", i, opts, pat),
                );
                if cc.code.is_null() {
                    continue;
                }
                for subj in [&b""[..], b"a", b"abc", b"a\nb", b"\xc3\xa9"] {
                    for engine in [Engine::Interpreter, Engine::Dfa] {
                        assert_match_eq(
                            &cc, &rr, subj, subj.len(), 0,
                            &MatchCfg::new(0), engine,
                            &format!("raw#{} {:02x?}", i, pat),
                        );
                    }
                }
            }
        }
    }
}

/// Row: randomized SUBJECTS against a fixed set of interesting patterns —
/// the mirror of the above, stressing the matcher rather than the compiler.
#[test]
fn randomized_subjects() {
    let mut g = Rng::new(SEED ^ 0x1234);
    let pats: [&str; 22] = [
        "a*b", "(a|b)+c", "^(\\w+)\\s(\\w+)$", "(?i)[a-z]+", "\\d{2,4}",
        "(a)(b)?(c)?", "(?<x>\\w)\\k<x>", "a(?=b)|c(?!d)", "(?<=ab)c",
        "[^\\n]*\\n", "\\b\\w+\\b", "(?:ab)+", "a{2,}", ".*?x", "(a+)+$",
        "\\R+", "\\X+", "(?s).*", "(?m)^\\w+$", "[[:alpha:]]+[[:digit:]]+",
        "\\p{L}+", "(*MARK:m)a|(*MARK:n)b",
    ];
    unsafe {
        for pat in pats {
            for opts in [0u32, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL] {
                let (cc, rr) = compile_both(
                    pat.as_bytes(), pat.len(), &CompileCfg::new(opts), pat,
                );
                if cc.code.is_null() {
                    continue;
                }
                for _ in 0..150 {
                    let n = g.below(24) as usize;
                    let subj = g.bytes_from(n, b"ab c9\n\r\t0xyzXYZ\xc3\xa9\x00");
                    for start in [0usize, subj.len() / 2, subj.len()] {
                        for engine in [Engine::Interpreter, Engine::Dfa] {
                            for mo in [
                                0,
                                PCRE2_NOTBOL,
                                PCRE2_NOTEOL,
                                PCRE2_NOTEMPTY,
                                PCRE2_PARTIAL_SOFT,
                            ] {
                                assert_match_eq(
                                    &cc, &rr, &subj, subj.len(), start,
                                    &MatchCfg::new(mo), engine,
                                    &format!("randsubj {:?} {:02x?}", pat, subj),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Row: randomized UTF-8 subjects with UTF/UCP compiled patterns, including
/// start offsets that land mid-character (which C must reject identically).
#[test]
fn randomized_utf_subjects() {
    let mut g = Rng::new(SEED ^ 0x5678);
    let pats: [&str; 12] = [
        "\\p{L}+", "\\p{Greek}+", ".", ".*", "\\X", "\\X+", "[\\x{80}-\\x{fff}]",
        "\\w+", "(?i)\\p{Lu}", "\u{00e9}+", "[^\u{20ac}]", "\\p{Han}",
    ];
    unsafe {
        for pat in pats {
            for opts in [
                PCRE2_UTF,
                PCRE2_UTF | PCRE2_UCP,
                PCRE2_UTF | PCRE2_CASELESS,
                PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
            ] {
                let (cc, rr) = compile_both(
                    pat.as_bytes(), pat.len(), &CompileCfg::new(opts), pat,
                );
                if cc.code.is_null() {
                    continue;
                }
                for _ in 0..200 {
                    // build a valid UTF-8 subject
                    let mut subj = Vec::new();
                    for _ in 0..g.below(8) {
                        let cp = match g.below(4) {
                            0 => g.below(0x80),
                            1 => g.range(0x80, 0x800),
                            2 => g.range(0x800, 0xD800),
                            _ => g.range(0x10000, 0x110000),
                        };
                        let ch = char::from_u32(cp).unwrap_or('a');
                        let mut b = [0u8; 4];
                        subj.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                    }
                    // every byte offset, including mid-character ones
                    for start in 0..=subj.len() {
                        for engine in [Engine::Interpreter, Engine::Dfa] {
                            assert_match_eq(
                                &cc, &rr, &subj, subj.len(), start,
                                &MatchCfg::new(0), engine,
                                &format!("utf {:?} {:02x?} start={}", pat, subj, start),
                            );
                            // and with NO_UTF_CHECK, which skips validation
                            assert_match_eq(
                                &cc, &rr, &subj, subj.len(), start,
                                &MatchCfg::new(PCRE2_NO_UTF_CHECK), engine,
                                &format!("utf nocheck {:?} start={}", pat, start),
                            );
                        }
                    }
                }
                // deliberately INVALID UTF-8 subjects
                for _ in 0..300 {
                    let n = g.below(10) as usize;
                    let subj = g.raw_bytes(n);
                    for engine in [Engine::Interpreter, Engine::Dfa] {
                        assert_match_eq(
                            &cc, &rr, &subj, subj.len(), 0,
                            &MatchCfg::new(0), engine,
                            &format!("badutf {:?} {:02x?}", pat, subj),
                        );
                    }
                }
            }
        }
    }
}

/// Row: `pcre2_code_copy` / `pcre2_code_copy_with_tables` produce codes that
/// behave identically to the original in both libraries.
#[test]
fn code_copy_variants() {
    let subjects = subj_bytes();
    unsafe {
        let (c, r) = both();
        for pat in PATTERNS {
            for &with_tables in &[false, true] {
                let cfg = CompileCfg::new(0);
                let cc = compile_in(c, pat.as_bytes(), pat.len(), &cfg);
                let rr = compile_in(r, pat.as_bytes(), pat.len(), &cfg);
                if cc.code.is_null() {
                    continue;
                }
                let ccopy = if with_tables {
                    (c.code_copy_with_tables)(cc.code)
                } else {
                    (c.code_copy)(cc.code)
                };
                let rcopy = if with_tables {
                    (r.code_copy_with_tables)(rr.code)
                } else {
                    (r.code_copy)(rr.code)
                };
                assert_eq!(
                    ccopy.is_null(),
                    rcopy.is_null(),
                    "code_copy nullness {:?}",
                    pat
                );
                if ccopy.is_null() {
                    continue;
                }
                let label = format!("copy(tables={}) {:?}", with_tables, pat);
                assert_pattern_info_eq(ccopy, rcopy, &label);
                let cb = serialized_bytes(c, ccopy);
                let rb = serialized_bytes(r, rcopy);
                assert_eq!(cb, rb, "{}: serialized copy differs", label);
                for subj in &subjects {
                    for engine in [Engine::Interpreter, Engine::Dfa] {
                        let co = run_match(
                            c, ccopy, subj, subj.len(), 0, &MatchCfg::new(0), engine,
                        );
                        let ro = run_match(
                            r, rcopy, subj, subj.len(), 0, &MatchCfg::new(0), engine,
                        );
                        assert_eq!(co, ro, "{}: match on copy differs", label);
                    }
                }
                (c.code_free)(ccopy);
                (r.code_free)(rcopy);
            }
        }
    }
}
