//! Phase B: compile + match differential tests.
//! CONFIGS.md rows 1-58, 76, 110-113.
mod harness;
use harness::*;

fn cross(cfg_options: &[(&str, u32, u32)], dfa: bool) {
    let pats = curated_patterns();
    let subs = curated_subjects();
    for (name, options, xoptions) in cfg_options {
        for p in &pats {
            for s in &subs {
                let cfg = Cfg {
                    options: *options,
                    extra_options: *xoptions,
                    ..Default::default()
                };
                let pv = cs(p);
                let sv = cs(s);
                let co = run_full(c(), &cfg, &pv[..pv.len() - 1], &sv[..sv.len() - 1], dfa);
                let ro = run_full(r(), &cfg, &pv[..pv.len() - 1], &sv[..sv.len() - 1], dfa);
                if co != ro {
                    panic!(
                        "DIVERGENCE [{name}] dfa={dfa}\n pattern = {p:?}\n subject = {s:?}\n   {}",
                        explain(&co, &ro)
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------- rows 1-36, 44
#[test]
fn compile_option_matrix_interpreter() {
    cross(
        &[
            ("default", 0, 0),
            ("CASELESS", PCRE2_CASELESS, 0),
            ("MULTILINE", PCRE2_MULTILINE, 0),
            ("DOTALL", PCRE2_DOTALL, 0),
            ("EXTENDED", PCRE2_EXTENDED, 0),
            ("EXTENDED_MORE", PCRE2_EXTENDED_MORE, 0),
            ("UNGREEDY", PCRE2_UNGREEDY, 0),
            ("ANCHORED", PCRE2_ANCHORED, 0),
            ("ENDANCHORED", PCRE2_ENDANCHORED, 0),
            ("DOLLAR_ENDONLY", PCRE2_DOLLAR_ENDONLY, 0),
            ("FIRSTLINE", PCRE2_FIRSTLINE, 0),
            ("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE, 0),
            ("DUPNAMES", PCRE2_DUPNAMES, 0),
            ("MATCH_UNSET_BACKREF", PCRE2_MATCH_UNSET_BACKREF, 0),
            ("ALLOW_EMPTY_CLASS", PCRE2_ALLOW_EMPTY_CLASS, 0),
            ("ALT_BSUX", PCRE2_ALT_BSUX, 0),
            ("EXTRA_ALT_BSUX", 0, PCRE2_EXTRA_ALT_BSUX),
            ("ALT_CIRCUMFLEX|ML", PCRE2_ALT_CIRCUMFLEX | PCRE2_MULTILINE, 0),
            ("ALT_VERBNAMES", PCRE2_ALT_VERBNAMES, 0),
            ("ALT_EXTENDED_CLASS", PCRE2_ALT_EXTENDED_CLASS, 0),
            ("LITERAL", PCRE2_LITERAL, 0),
            ("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS, 0),
            ("NO_DOTSTAR_ANCHOR", PCRE2_NO_DOTSTAR_ANCHOR, 0),
            ("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE, 0),
        ],
        false,
    );
}

#[test]
fn compile_option_matrix_unicode() {
    cross(
        &[
            ("UTF", PCRE2_UTF, 0),
            ("UTF|UCP", PCRE2_UTF | PCRE2_UCP, 0),
            ("UTF|CASELESS", PCRE2_UTF | PCRE2_CASELESS, 0),
            ("UTF|UCP|CASELESS", PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS, 0),
            ("UCP", PCRE2_UCP, 0),
            ("UCP|CASELESS", PCRE2_UCP | PCRE2_CASELESS, 0),
            (
                "UTF|MATCH_INVALID_UTF",
                PCRE2_UTF | PCRE2_MATCH_INVALID_UTF,
                0,
            ),
            (
                "CASELESS_RESTRICT",
                PCRE2_CASELESS | PCRE2_UTF | PCRE2_UCP,
                PCRE2_EXTRA_CASELESS_RESTRICT,
            ),
            (
                "TURKISH_CASING",
                PCRE2_CASELESS | PCRE2_UTF,
                PCRE2_EXTRA_TURKISH_CASING,
            ),
            ("ASCII_BSD", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSD),
            ("ASCII_BSS", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSS),
            ("ASCII_BSW", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSW),
            ("ASCII_POSIX", PCRE2_UCP, PCRE2_EXTRA_ASCII_POSIX),
            ("ASCII_DIGIT", PCRE2_UCP, PCRE2_EXTRA_ASCII_DIGIT),
            (
                "ASCII_ALL",
                PCRE2_UCP,
                PCRE2_EXTRA_ASCII_BSD
                    | PCRE2_EXTRA_ASCII_BSS
                    | PCRE2_EXTRA_ASCII_BSW
                    | PCRE2_EXTRA_ASCII_POSIX
                    | PCRE2_EXTRA_ASCII_DIGIT,
            ),
            ("MATCH_WORD", 0, PCRE2_EXTRA_MATCH_WORD),
            ("MATCH_LINE", 0, PCRE2_EXTRA_MATCH_LINE),
            (
                "SURROGATE_ESCAPES",
                0,
                PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
            ),
            ("BAD_ESCAPE_IS_LITERAL", 0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
            ("ESCAPED_CR_IS_LF", 0, PCRE2_EXTRA_ESCAPED_CR_IS_LF),
            ("ALLOW_LOOKAROUND_BSK", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
            ("PYTHON_OCTAL", 0, PCRE2_EXTRA_PYTHON_OCTAL),
            ("NO_BS0", 0, PCRE2_EXTRA_NO_BS0),
        ],
        false,
    );
}

// ------------------------------------------------------------------ rows 59-63
#[test]
fn dfa_option_matrix() {
    cross(
        &[
            ("default", 0, 0),
            ("CASELESS", PCRE2_CASELESS, 0),
            ("MULTILINE", PCRE2_MULTILINE, 0),
            ("DOTALL", PCRE2_DOTALL, 0),
            ("UTF", PCRE2_UTF, 0),
            ("UTF|UCP", PCRE2_UTF | PCRE2_UCP, 0),
            ("ANCHORED", PCRE2_ANCHORED, 0),
            ("ENDANCHORED", PCRE2_ENDANCHORED, 0),
            ("EXTENDED", PCRE2_EXTENDED, 0),
        ],
        true,
    );
}

// -------------------------------------------------------------------- row 40
#[test]
fn newline_conventions() {
    let nl_subjects = [
        "a", "a\n", "a\r", "a\r\n", "\na", "\ra", "\r\na", "a\nb\rc\r\nd", "a\0b", "\0",
        "a\u{85}b", "a\u{2028}b", "a\u{2029}b", "\u{b}", "\u{c}", "a\n\n\nb",
    ];
    let nl_patterns = [
        "a$", "^a", "(?m)^a", "(?m)a$", ".", "(?s).", "\\R", "\\R+", "\\N", "^.*$", "(?m)^.*$",
        "a.b", "(*ANY)\\R", "\\r\\n", "$",
    ];
    for nl in [
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ] {
        for bsr in [PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
            for opts in [0, PCRE2_MULTILINE, PCRE2_UTF, PCRE2_MULTILINE | PCRE2_UTF] {
                for p in nl_patterns {
                    for s in nl_subjects {
                        let cfg = Cfg {
                            options: opts,
                            newline: Some(nl),
                            bsr: Some(bsr),
                            ..Default::default()
                        };
                        differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                        differential(&cfg, p.as_bytes(), s.as_bytes(), true);
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------- rows 43, 49
#[test]
fn optimize_and_tables() {
    let dirs: Vec<Vec<u32>> = vec![
        vec![],
        vec![PCRE2_OPTIMIZATION_NONE],
        vec![PCRE2_OPTIMIZATION_FULL],
        vec![PCRE2_AUTO_POSSESS_OFF],
        vec![PCRE2_AUTO_POSSESS],
        vec![PCRE2_DOTSTAR_ANCHOR_OFF],
        vec![PCRE2_DOTSTAR_ANCHOR],
        vec![PCRE2_START_OPTIMIZE_OFF],
        vec![PCRE2_START_OPTIMIZE],
        vec![
            PCRE2_OPTIMIZATION_NONE,
            PCRE2_AUTO_POSSESS,
            PCRE2_START_OPTIMIZE,
        ],
        vec![PCRE2_OPTIMIZATION_FULL, PCRE2_START_OPTIMIZE_OFF],
    ];
    let pats = [
        "a*b", ".*abc", "^abc", "abc", "(a+)+b", "\\d+x", "[a-z]+@", "a{2,}b", ".*",
        "(?:ab)*c", "\\w+\\s+\\w+", "(a|ab)c",
    ];
    let subs = curated_subjects();
    for d in &dirs {
        for use_tables in [false, true] {
            for p in pats {
                for s in &subs {
                    let cfg = Cfg {
                        optimize: d.clone(),
                        use_maketables: use_tables,
                        ..Default::default()
                    };
                    differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                }
            }
        }
    }
}

// --------------------------------------------------------- rows 45-48, 58, 113
#[test]
fn limits_and_shapes() {
    let deep = "((((((((((((((((((((a))))))))))))))))))))";
    let vlb = "(?<=a{1,10})b";
    let a100 = "a".repeat(100);
    let cases: Vec<(&str, &str)> = vec![
        (deep, "a"),
        (vlb, "aaab"),
        ("(?<=ab|cdef)x", "cdefx"),
        ("(a+)+$", "aaaaaaaaaaaaaaaaaaaaaaaaab"),
        ("(?R)?a", "aaa"),
        ("a{100}", a100.as_str()),
    ];
    for (p, s) in cases {
        for vlbl in [None, Some(0u32), Some(1), Some(3), Some(255)] {
            for pnl in [None, Some(0u32), Some(1), Some(5), Some(250)] {
                let cfg = Cfg {
                    max_varlookbehind: vlbl,
                    parens_nest_limit: pnl,
                    ..Default::default()
                };
                differential(&cfg, p.as_bytes(), s.as_bytes(), false);
            }
        }
        for mpl in [None, Some(0usize), Some(1), Some(p.len() - 1), Some(p.len()), Some(usize::MAX)]
        {
            for mpcl in [None, Some(0usize), Some(1), Some(64), Some(usize::MAX)] {
                let cfg = Cfg {
                    max_pattern_length: mpl,
                    max_pattern_compiled_length: mpcl,
                    ..Default::default()
                };
                differential(&cfg, p.as_bytes(), s.as_bytes(), false);
            }
        }
        for ml in [None, Some(0u32), Some(1), Some(10), Some(1000), Some(u32::MAX)] {
            for dl in [None, Some(0u32), Some(1), Some(10), Some(u32::MAX)] {
                for hl in [None, Some(0u32), Some(1), Some(u32::MAX)] {
                    let cfg = Cfg {
                        match_limit: ml,
                        depth_limit: dl,
                        heap_limit: hl,
                        ..Default::default()
                    };
                    differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                    differential(&cfg, p.as_bytes(), s.as_bytes(), true);
                }
            }
        }
    }
}

// ------------------------------------------------------------- rows 50, 54, 56
#[test]
fn match_options_offsets_ovecsizes() {
    let pats = [
        "a*", "^a", "a$", "(?m)^a", "abc", "(a)(b)?(c)?", "\\b\\w+\\b", "", "(?=a)", "x?",
    ];
    let subs = ["", "a", "abc", "aaa", "xabcx", "a\nb", "\n"];
    for p in pats {
        for s in subs {
            for mo in [
                0,
                PCRE2_NOTBOL,
                PCRE2_NOTEOL,
                PCRE2_NOTEMPTY,
                PCRE2_NOTEMPTY_ATSTART,
                PCRE2_NOTBOL | PCRE2_NOTEOL,
                PCRE2_NOTEMPTY | PCRE2_NOTBOL,
                PCRE2_ANCHORED,
                PCRE2_ENDANCHORED,
                PCRE2_ANCHORED | PCRE2_ENDANCHORED,
                PCRE2_NO_UTF_CHECK,
                PCRE2_COPY_MATCHED_SUBJECT,
                PCRE2_DISABLE_RECURSELOOP_CHECK,
                PCRE2_NO_JIT,
            ] {
                for so in 0..=s.len() {
                    for ovec in [None, Some(0u32), Some(1), Some(2), Some(3), Some(16)] {
                        let cfg = Cfg {
                            match_options: mo,
                            start_offset: so,
                            ovecsize: ovec,
                            ..Default::default()
                        };
                        differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                    }
                }
            }
        }
    }
}

#[test]
fn dfa_match_options() {
    let pats = ["a*", "^a", "abc", "(?m)^a", "a|ab", "x?"];
    let subs = ["", "a", "abc", "aaa", "a\nb"];
    for p in pats {
        for s in subs {
            for mo in [
                0,
                PCRE2_NOTBOL,
                PCRE2_NOTEOL,
                PCRE2_NOTEMPTY,
                PCRE2_NOTEMPTY_ATSTART,
                PCRE2_ANCHORED,
                PCRE2_ENDANCHORED,
                PCRE2_DFA_SHORTEST,
                PCRE2_DFA_SHORTEST | PCRE2_ANCHORED,
                PCRE2_NO_UTF_CHECK,
                PCRE2_COPY_MATCHED_SUBJECT,
            ] {
                for so in 0..=s.len() {
                    for ovec in [None, Some(1u32), Some(2), Some(16)] {
                        let cfg = Cfg {
                            match_options: mo,
                            start_offset: so,
                            ovecsize: ovec,
                            ..Default::default()
                        };
                        differential(&cfg, p.as_bytes(), s.as_bytes(), true);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------- row 51
#[test]
fn partial_matching() {
    let pats = ["abcd", "\\d{4}", "a(?:bc)?d", "^abc$", "(a)(b)(c)", "ab+c", "\\bfoo\\b"];
    let subs = ["", "a", "ab", "abc", "abcd", "abcde", "12", "1234", "foo"];
    for p in pats {
        for s in subs {
            for mo in [
                PCRE2_PARTIAL_SOFT,
                PCRE2_PARTIAL_HARD,
                PCRE2_PARTIAL_SOFT | PCRE2_NOTBOL,
                PCRE2_PARTIAL_HARD | PCRE2_NOTEOL,
                PCRE2_PARTIAL_SOFT | PCRE2_ANCHORED,
            ] {
                let cfg = Cfg { match_options: mo, ..Default::default() };
                differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                differential(&cfg, p.as_bytes(), s.as_bytes(), true);
            }
        }
    }
}

// ------------------------------------------------------------------- row 55
#[test]
fn offset_limit() {
    let pats = ["abc", "a", "\\d", "x*"];
    let subs = ["abcabcabc", "aaaa", "0123456789", "xxxx"];
    for p in pats {
        for s in subs {
            for lim in [None, Some(0usize), Some(1), Some(3), Some(s.len()), Some(PCRE2_UNSET)] {
                for use_flag in [false, true] {
                    let cfg = Cfg {
                        options: if use_flag { PCRE2_USE_OFFSET_LIMIT } else { 0 },
                        offset_limit: lim,
                        ..Default::default()
                    };
                    differential(&cfg, p.as_bytes(), s.as_bytes(), false);
                    differential(&cfg, p.as_bytes(), s.as_bytes(), true);
                }
            }
        }
    }
}

// -------------------------------------------------------------- rows 54, 113
#[test]
fn zero_terminated_and_byte_shapes() {
    let mut subjects: Vec<Vec<u8>> = vec![
        vec![],
        vec![b'a'],
        vec![0],
        b"abc\0def".to_vec(),
        (0u16..256).map(|b| b as u8).collect(),
        vec![b'a'; 4096],
    ];
    subjects.push((0..256).flat_map(|b| [b as u8, b'a']).collect());
    let pats: Vec<&str> = vec!["a", ".", "\\x00", "[\\x00-\\xff]+", "a+", "\\w+", "^", "$"];
    for p in &pats {
        for s in &subjects {
            for (ztp, zts) in [(false, false), (true, false), (false, true), (true, true)] {
                let cfg = Cfg {
                    zero_terminated_pattern: ztp,
                    zero_terminated_subject: zts,
                    ..Default::default()
                };
                let pv = cs(p);
                let sv = cb(s);
                differential(
                    &cfg,
                    if ztp { &pv } else { &pv[..pv.len() - 1] },
                    if zts { &sv } else { &sv[..sv.len() - 1] },
                    false,
                );
            }
        }
    }
}

// -------------------------------------- rows 1-58 randomized (property style)
#[test]
fn randomized_compile_match() {
    let option_pool: &[u32] = &[
        0,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_DOTALL,
        PCRE2_EXTENDED,
        PCRE2_UNGREEDY,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UCP,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_DUPNAMES,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_FIRSTLINE,
        PCRE2_DOLLAR_ENDONLY,
        PCRE2_MATCH_UNSET_BACKREF,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_ALLOW_EMPTY_CLASS,
        PCRE2_NO_START_OPTIMIZE,
        PCRE2_NO_AUTO_POSSESS,
        PCRE2_NO_DOTSTAR_ANCHOR,
        PCRE2_ALT_BSUX,
        PCRE2_LITERAL,
        PCRE2_MULTILINE | PCRE2_DOTALL | PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_CASELESS | PCRE2_MULTILINE,
    ];
    let xopt_pool: &[u32] = &[
        0,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSW,
        PCRE2_EXTRA_MATCH_WORD,
        PCRE2_EXTRA_MATCH_LINE,
        PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
        PCRE2_EXTRA_ALT_BSUX,
        PCRE2_EXTRA_PYTHON_OCTAL,
        PCRE2_EXTRA_ESCAPED_CR_IS_LF,
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        PCRE2_EXTRA_TURKISH_CASING,
        PCRE2_EXTRA_NO_BS0,
    ];
    let mopt_pool: &[u32] = &[
        0,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_ANCHORED,
        PCRE2_NO_UTF_CHECK,
        PCRE2_COPY_MATCHED_SUBJECT,
        PCRE2_DISABLE_RECURSELOOP_CHECK,
    ];
    let mut rng = Rng::new(0x5EED_0001);
    for i in 0..20000u32 {
        let options = *rng.pick(option_pool);
        let utf = options & PCRE2_UTF != 0;
        let pat = if rng.below(3) == 0 {
            (*rng.pick(&curated_patterns())).to_string()
        } else {
            let d = rng.range(1, 3) as u32;
            random_pattern(&mut rng, d)
        };
        let subject = if rng.below(3) == 0 {
            (*rng.pick(&curated_subjects())).as_bytes().to_vec()
        } else {
            random_subject(&mut rng, utf)
        };
        let dfa = i % 4 == 3;
        let mo = *rng.pick(mopt_pool);
        // PARTIAL_* with ENDANCHORED is rejected; DFA rejects some flags. Both
        // implementations must reject identically, so no filtering is applied.
        let cfg = Cfg {
            options,
            extra_options: *rng.pick(xopt_pool),
            newline: if rng.bool() { Some(rng.range(1, 6) as u32) } else { None },
            bsr: if rng.bool() { Some(rng.range(1, 2) as u32) } else { None },
            optimize: if rng.below(4) == 0 {
                vec![*rng.pick(&[
                    PCRE2_OPTIMIZATION_NONE,
                    PCRE2_OPTIMIZATION_FULL,
                    PCRE2_AUTO_POSSESS_OFF,
                    PCRE2_DOTSTAR_ANCHOR_OFF,
                    PCRE2_START_OPTIMIZE_OFF,
                ])]
            } else {
                vec![]
            },
            use_maketables: rng.below(8) == 0,
            match_options: mo,
            start_offset: if subject.is_empty() { 0 } else { rng.below(subject.len() + 1) },
            ovecsize: if rng.bool() { Some(rng.below(6) as u32) } else { None },
            match_limit: if rng.below(16) == 0 { Some(rng.below(200) as u32) } else { None },
            depth_limit: if rng.below(16) == 0 { Some(rng.below(200) as u32) } else { None },
            ..Default::default()
        };
        let co = run_full(c(), &cfg, pat.as_bytes(), &subject, dfa);
        let ro = run_full(r(), &cfg, pat.as_bytes(), &subject, dfa);
        if co != ro {
            panic!(
                "DIVERGENCE iter={i} dfa={dfa}\n cfg     = {cfg:?}\n pattern = {pat:?}\n subject = {:?}\n   {}",
                String::from_utf8_lossy(&subject),
                explain(&co, &ro)
            );
        }
    }
}
