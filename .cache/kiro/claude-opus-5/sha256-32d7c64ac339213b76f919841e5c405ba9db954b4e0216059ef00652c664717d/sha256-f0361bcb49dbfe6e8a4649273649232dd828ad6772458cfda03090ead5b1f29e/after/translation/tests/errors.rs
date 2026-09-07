//! Phase C — error/rejection-path differential tests.
//!
//! One test per `ERRORS.md` row (or per tight group of rows). Rows marked
//! **UB** in `ERRORS.md` run one-child-per-case (`assert_same_isolated`) so a
//! fatal signal is compared rather than masking later cases.

mod common;
use common::*;

const PAT_MAJOR: &[u8] = b"^([0-9]+)\\.*";
const PAT_MINOR: &[u8] = b"^[0-9]+\\.([0-9]+)\\.*";
const PAT_BUILD: &[u8] = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*";

// ===========================================================================
// get_os_arch
// ===========================================================================

/// ERRORS row 1 + 3 — no arch token / empty string → NULL.
#[test]
fn err01_arch_returns_null_when_absent() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x01);
    let mut cases = vec![
        Case::arch(b""),
        Case::arch(b" "),
        Case::arch(b"no architecture in here"),
        Case::arch(b"x86"),
        Case::arch(b"64"),
        Case::arch(b"ARM64"),
        Case::arch(b"X86_64"),
    ];
    for _ in 0..2000 {
        // letters only, so no ARCHS entry (which all contain a digit except AIX)
        // can appear; AIX is excluded by using lowercase only.
        let n = rng.range(0, 40);
        let s: Vec<u8> = (0..n)
            .map(|_| b'a' + (rng.below(26) as u8))
            .collect();
        cases.push(Case::arch(&s));
    }
    assert_same(&c, &r, &cases, "err01 get_os_arch NULL result");
}

/// ERRORS row 2 — `os_header == NULL` dereferenced by `strstr` (**UB**).
#[test]
fn err02_arch_null_pointer_faults_identically() {
    let (c, r) = open_pair();
    let case = Case::Arch { input: None };
    // Assert the *specific* termination, not just "both agree", so a harness
    // regression (e.g. a poll timeout) cannot produce a false pass.
    let oc = probe(&c, &case);
    let or = probe(&r, &case);
    assert_eq!(
        signal_of(oc.status),
        Some(11),
        "C get_os_arch(NULL) should SIGSEGV, got {oc:?}"
    );
    assert_eq!(
        signal_of(or.status),
        Some(11),
        "Rust get_os_arch(NULL) should SIGSEGV, got {or:?}"
    );
    assert_same_isolated(&c, &r, &[case], "err02 get_os_arch(NULL)");
}

// ===========================================================================
// w_regexec — argument guards
// ===========================================================================

/// ERRORS rows 4, 5, 6 — the `!(pattern && string)` guard.
#[test]
fn err04_regexec_null_argument_guard() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for nmatch in [0usize, 1, 2, 8] {
        for arr in [None, Some(0usize), Some(2), Some(8)] {
            // pattern NULL (row 4)
            cases.push(Case::Regex {
                pat: None,
                subj: Some(b"10.0.19041".to_vec()),
                nmatch,
                arr,
            });
            // string NULL (row 5)
            cases.push(Case::Regex {
                pat: Some(PAT_MAJOR.to_vec()),
                subj: None,
                nmatch,
                arr,
            });
            // both NULL (row 6)
            cases.push(Case::Regex {
                pat: None,
                subj: None,
                nmatch,
                arr,
            });
            // pattern NULL together with an *invalid* pattern-shaped subject
            cases.push(Case::Regex {
                pat: None,
                subj: Some(b"[".to_vec()),
                nmatch,
                arr,
            });
        }
    }
    assert_same(&c, &r, &cases, "err04 w_regexec NULL guard");
}

// ===========================================================================
// w_regexec — regcomp failures (rows 7..10)
// ===========================================================================

/// ERRORS rows 7, 8, 9, 10 — every distinct `regcomp` failure class. The
/// child's `stderr` is captured and compared, so the exact
/// `Couldn't compile regular expression '%s'` message is verified too.
#[test]
fn err07_regexec_regcomp_failures() {
    let (c, r) = open_pair();
    let bad: &[&[u8]] = &[
        b"[",              // REG_EBRACK   (row 7)
        b"[a-",            //
        b"[[:foo:]]",      // REG_ECTYPE
        b"[a-\\",          //
        b"\\",             // REG_EESCAPE  (row 8)
        b"a\\",            //
        b"(",              // REG_EPAREN   (row 9)
        b"(a",             //
        b")",              // (glibc: REG_EPAREN for unmatched ')')
        b"a)",             //
        b"*",              // REG_BADRPT   (row 10)
        b"+",              //
        b"?",              //
        b"{1",             // REG_EBRACE
        b"a{",             //
        b"a{2,1}",         // REG_BADBR
        b"a{999999999999}",//
        b"a**",            //
        b"[]",             //
        b"[^]",            //
        b"\\1",            // REG_ESUBREG / backref in ERE
        b"a|*",            //
        b"|*",             //
    ];
    let mut cases = Vec::new();
    let mut isolated = Vec::new();
    for pat in bad {
        for nmatch in [0usize, 1, 2] {
            cases.push(Case::Regex {
                pat: Some(pat.to_vec()),
                subj: Some(b"10.0.19041".to_vec()),
                nmatch,
                arr: Some(2),
            });
        }
        // Invalid pattern AND NULL string: the NULL guard wins, so *no* stderr
        // message must be produced.
        cases.push(Case::Regex {
            pat: Some(pat.to_vec()),
            subj: None,
            nmatch: 2,
            arr: Some(2),
        });
        // Invalid pattern AND NULL pmatch. Some entries above (`a**`, `\1`,
        // `a|*`, …) are in fact accepted by glibc's ERE, so this reaches
        // `regexec` and faults — hence isolated execution.
        isolated.push(Case::Regex {
            pat: Some(pat.to_vec()),
            subj: Some(b"x".to_vec()),
            nmatch: 2,
            arr: None,
        });
    }
    assert_same(&c, &r, &cases, "err07 w_regexec regcomp failures");
    assert_same_isolated(&c, &r, &isolated, "err07 w_regexec bad pattern + NULL pmatch");
}

/// ERRORS row 7 — the stderr text itself, isolated per pattern so the message
/// is attributed to one input.
#[test]
fn err07b_regexec_regcomp_stderr_text() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = [&b"["[..], &b"("[..], &b"\\"[..], &b"*"[..], &b"a{2,1}"[..]]
        .iter()
        .map(|p| Case::Regex {
            pat: Some(p.to_vec()),
            subj: Some(b"1.2.3".to_vec()),
            nmatch: 2,
            arr: Some(2),
        })
        .collect();
    assert_same_isolated(&c, &r, &cases, "err07b w_regexec regcomp stderr text");
}

// ===========================================================================
// w_regexec — match failures and nmatch/pmatch boundaries (rows 11..18)
// ===========================================================================

/// ERRORS row 11 + 17 — `REG_NOMATCH` → 0, and what glibc writes to `pmatch`.
#[test]
fn err11_regexec_no_match() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x11);
    let mut cases = Vec::new();
    for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD] {
        for subj in [
            &b""[..],
            &b"abc"[..],
            &b".1.2"[..],
            &b" 1.2.3"[..],
            &b"-1"[..],
            &b"a1.2.3"[..],
        ] {
            for nmatch in [0usize, 1, 2, 3, 8] {
                cases.push(Case::Regex {
                    pat: Some(pat.to_vec()),
                    subj: Some(subj.to_vec()),
                    nmatch,
                    arr: Some(8),
                });
            }
        }
    }
    for _ in 0..500 {
        let s = rng.word_between(0, 12); // letters never match ^[0-9]
        for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD] {
            cases.push(Case::regex(pat, &s, 2, 2));
        }
    }
    assert_same(&c, &r, &cases, "err11 w_regexec REG_NOMATCH");
}

/// ERRORS rows 12, 13 — `nmatch == 0` with NULL and with non-NULL `pmatch`.
#[test]
fn err12_regexec_nmatch_zero_is_safe() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD, b"", b"nope"] {
        for subj in [&b"1.2.3"[..], &b""[..], &b"abc"[..]] {
            cases.push(Case::Regex {
                pat: Some(pat.to_vec()),
                subj: Some(subj.to_vec()),
                nmatch: 0,
                arr: None,
            });
            cases.push(Case::Regex {
                pat: Some(pat.to_vec()),
                subj: Some(subj.to_vec()),
                nmatch: 0,
                arr: Some(4),
            });
        }
    }
    assert_same(&c, &r, &cases, "err12 w_regexec nmatch=0");
}

/// ERRORS row 14 — `nmatch > 0` with `pmatch == NULL` (**UB**, faults inside
/// `regexec`).
#[test]
fn err14_regexec_null_pmatch_faults_identically() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for nmatch in [1usize, 2, 3, 8] {
        for pat in [PAT_MAJOR, PAT_BUILD, b""] {
            cases.push(Case::Regex {
                pat: Some(pat.to_vec()),
                subj: Some(b"10.0.19041".to_vec()),
                nmatch,
                arr: None,
            });
        }
    }
    assert_same_isolated(&c, &r, &cases, "err14 w_regexec NULL pmatch");

    // The termination itself must be a real fault in both, not a harness
    // timeout or a silent "returned 0".
    let probe_case = Case::Regex {
        pat: Some(PAT_MAJOR.to_vec()),
        subj: Some(b"10.0.19041".to_vec()),
        nmatch: 2,
        arr: None,
    };
    for lib in [&c, &r] {
        let o = probe(lib, &probe_case);
        assert_eq!(
            signal_of(o.status),
            Some(11),
            "{} w_regexec(nmatch=2, pmatch=NULL) should SIGSEGV, got {o:?}",
            lib.name
        );
    }
}

/// ERRORS rows 15, 16 — nmatch above / below the pattern's group count.
#[test]
fn err15_regexec_nmatch_vs_group_count() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x15);
    let pats: &[&[u8]] = &[
        b"^[0-9]+",                 // 0 groups
        PAT_MAJOR,                  // 1 group
        PAT_BUILD,                  // 2 groups
        b"(a)(b)(c)(d)(e)",         // 5 groups
    ];
    let mut cases = Vec::new();
    for _ in 0..100 {
        let v = rng.version_between(1, 5);
        for pat in pats {
            for nmatch in [0usize, 1, 2, 3, 6, 8, 16] {
                cases.push(Case::regex(pat, &v, nmatch, 16));
            }
        }
        for pat in pats {
            for nmatch in [0usize, 1, 2, 3, 6, 8, 16] {
                cases.push(Case::regex(pat, b"abcde", nmatch, 16));
            }
        }
    }
    assert_same(&c, &r, &cases, "err15 w_regexec nmatch vs group count");
}

/// ERRORS row 18 — empty pattern is *valid* in glibc ERE.
#[test]
fn err18_regexec_empty_pattern_is_valid() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for subj in [&b""[..], &b"a"[..], &b"1.2.3"[..], &b"\xff\xfe"[..]] {
        for nmatch in [0usize, 1, 2, 4] {
            cases.push(Case::Regex {
                pat: Some(Vec::new()),
                subj: Some(subj.to_vec()),
                nmatch,
                arr: Some(4),
            });
        }
    }
    assert_same(&c, &r, &cases, "err18 w_regexec empty pattern");
}

/// ERRORS rows 43, 44 — oversized `nmatch` crossing the FFI boundary.
///
/// There is no enum in this ABI (documented as row 43 in `ERRORS.md`); the
/// widest "out-of-range value" a caller can pass is `nmatch`, so every
/// interesting magnitude is exercised, including `SIZE_MAX`.
#[test]
fn err43_regexec_oversized_nmatch() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for nmatch in [
        4096usize,
        1 << 20,
        1 << 32,
        1 << 48,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        for pat in [PAT_MAJOR, PAT_BUILD, b"nomatch"] {
            cases.push(Case::Regex {
                pat: Some(pat.to_vec()),
                subj: Some(b"10.0.19041.1234".to_vec()),
                nmatch,
                arr: Some(4096),
            });
        }
    }
    assert_same_isolated(&c, &r, &cases, "err43 w_regexec oversized nmatch");
}

/// ERRORS row 45 — oversized inputs (no length limit exists in the C).
#[test]
fn err45_oversized_inputs() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x45);
    let big = rng.word(64 * 1024);
    let mut cases = vec![
        Case::arch(&big),
        Case::regex(PAT_MAJOR, &big, 2, 2),
        Case::regex(PAT_BUILD, &big, 2, 2),
        Case::parse(&big),
    ];
    let mut pat = Vec::new();
    for _ in 0..4096 {
        pat.extend_from_slice(b"[0-9]*");
    }
    cases.push(Case::regex(&pat, b"12345", 2, 2));
    let mut with_marker = big.clone();
    with_marker.extend_from_slice(b" [Ver: 10.0.19041]");
    cases.push(Case::parse(&with_marker));
    assert_same(&c, &r, &cases, "err45 oversized inputs");
}

// ===========================================================================
// parse_uname_string — argument guards (rows 19..21)
// ===========================================================================

/// ERRORS row 19 — `osd == NULL` returns before touching the buffer.
#[test]
fn err19_parse_null_osd_is_a_noop() {
    let (c, r) = open_pair();
    let inputs: &[&[u8]] = &[
        b"",
        b"W [Ver: 10.0.19041]",
        b"h [ubuntu|debian: 22.04 (jammy)] x86_64",
        b"h [ubuntu]",
        b"plain x86_64",
        b" [Ver: ",
        b" [",
    ];
    let cases: Vec<Case> = inputs
        .iter()
        .map(|s| Case::Parse {
            input: Some(s.to_vec()),
            osd_null: true,
            prepopulate: false,
            times: 1,
        })
        .collect();
    assert_same(&c, &r, &cases, "err19 parse_uname_string(osd=NULL)");
}

/// ERRORS rows 20, 21 — `uname == NULL`: faults when `osd != NULL`, returns
/// cleanly when `osd == NULL` (the `osd` check comes first).
#[test]
fn err20_parse_null_uname() {
    let (c, r) = open_pair();
    let cases = vec![
        // row 20: SIGSEGV in strstr(NULL, " [Ver: ")
        Case::Parse {
            input: None,
            osd_null: false,
            prepopulate: false,
            times: 1,
        },
        Case::Parse {
            input: None,
            osd_null: false,
            prepopulate: true,
            times: 1,
        },
        // row 21: no crash, osd checked first
        Case::Parse {
            input: None,
            osd_null: true,
            prepopulate: false,
            times: 1,
        },
        Case::Parse {
            input: None,
            osd_null: true,
            prepopulate: false,
            times: 3,
        },
    ];
    assert_same_isolated(&c, &r, &cases, "err20 parse_uname_string(uname=NULL)");

    // Row 20 must fault; row 21 must NOT (osd is checked first).
    let faulting = Case::Parse {
        input: None,
        osd_null: false,
        prepopulate: false,
        times: 1,
    };
    let safe = Case::Parse {
        input: None,
        osd_null: true,
        prepopulate: false,
        times: 1,
    };
    for lib in [&c, &r] {
        let o = probe(lib, &faulting);
        assert_eq!(
            signal_of(o.status),
            Some(11),
            "{} parse_uname_string(NULL, &osd) should SIGSEGV, got {o:?}",
            lib.name
        );
        let o = probe(lib, &safe);
        assert_eq!(
            signal_of(o.status),
            None,
            "{} parse_uname_string(NULL, NULL) should return cleanly, got {o:?}",
            lib.name
        );
    }
}

// ===========================================================================
// parse_uname_string — "no marker" rejection (rows 22, 23, 40, 49, 50)
// ===========================================================================

/// ERRORS rows 22, 23, 40 (plus CONFIGS rows 49, 50) — inputs the parser
/// rejects outright because neither marker is present.
#[test]
fn err22_parse_no_marker() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x22);
    let mut cases: Vec<Case> = [
        &b""[..],
        &b" "[..],
        &b"["[..],
        &b"]"[..],
        &b"a"[..],
        &b"plain text with no markers"[..],
        &b"a[b: 1]"[..],           // row 49: no space before '['
        &b"a[Ver: 1.2.3]"[..],     // row 49
        &b"[Ver: 1.2.3]"[..],      // row 49 (marker needs the leading space)
        &b"a [Ver:1.2.3]"[..],     // row 50: falls through to " [" branch
        &b"a [Ver 1.2.3]"[..],     // row 50
        &b"a [ver: 1.2.3]"[..],    // case sensitive
        &b"no markers x86_64"[..], // row 22: only os_arch
        &b"x86_64"[..],
        &b"AIX"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    // random letter-only strings: no marker, no arch -> every field NULL
    for _ in 0..2000 {
        let n = rng.range(0, 40);
        let s: Vec<u8> = (0..n).map(|_| b'a' + (rng.below(26) as u8)).collect();
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "err22 parse no marker");
}

// ===========================================================================
// parse_uname_string — regex-failure branches (rows 24..27, 35, 36, 41, 42)
// ===========================================================================

/// ERRORS rows 24, 25, 26 — the three Ver-branch regexes failing independently.
#[test]
fn err24_parse_ver_regex_failures() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x24);
    let mut cases: Vec<Case> = [
        &b"W [Ver: abc]"[..],       // all three fail
        &b"W [Ver: 10]"[..],        // minor + build fail
        &b"W [Ver: 10.0]"[..],      // build fails
        &b"W [Ver: 10.0.1]"[..],    // all succeed
        &b"W [Ver: 10.]"[..],       // minor fails
        &b"W [Ver: 10.0.]"[..],     // build fails
        &b"W [Ver: .10.0.1]"[..],   // major fails -> all fail
        &b"W [Ver: 10.a.1]"[..],    // minor fails, build fails
        &b"W [Ver: 10.0.a]"[..],    // build fails
        &b"W [Ver: 10.0.1a]"[..],   // build succeeds up to 'a'
        &b"W [Ver:  10.0.1]"[..],   // leading space -> major fails
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    for _ in 0..1500 {
        let mut s = b"W [Ver: ".to_vec();
        s.extend_from_slice(&rng.bytes_between(0, 18));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "err24 parse Ver regex failures");
}

/// ERRORS row 27 — the Ver branch never calls `get_os_arch`.
#[test]
fn err27_parse_ver_leaves_arch_null() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        for tmpl in [
            &b"PRE [Ver: 10.0.1]"[..],
            &b"[Ver: 10.0.1] POST"[..],
            &b"PRE [Ver: 10.0.1 POST]"[..],
        ] {
            let mut s = Vec::new();
            for w in tmpl.split(|b| *b == b'P') {
                s.extend_from_slice(w);
            }
            // deliberately splice the arch into three positions
            let mut a = arch.to_vec();
            a.extend_from_slice(b" [Ver: 10.0.19041]");
            cases.push(Case::parse(&a));
            let mut b = b"host [Ver: 10.0.19041 ".to_vec();
            b.extend_from_slice(arch);
            b.push(b']');
            cases.push(Case::parse(&b));
            let mut d = b"host ".to_vec();
            d.extend_from_slice(arch);
            d.extend_from_slice(b" x [Ver: 10.0.19041]");
            cases.push(Case::parse(&d));
        }
    }
    assert_same(&c, &r, &cases, "err27 parse Ver leaves os_arch NULL");
}

/// ERRORS rows 35, 36 — Unix-branch regexes failing.
#[test]
fn err35_parse_unix_regex_failures() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x35);
    let mut cases: Vec<Case> = [
        &b"h [os: abc]"[..],
        &b"h [os: 22]"[..],
        &b"h [os: 22.]"[..],
        &b"h [os: .22]"[..],
        &b"h [os: 22.04]"[..],
        &b"h [os:  22.04]"[..],
        &b"h [os: v22.04]"[..],
        &b"h [os: 22.04 (jammy)]"[..],
        &b"h [os: abc (jammy)]"[..],
        &b"h [os: (jammy)]"[..],
        &b"h [os: ]"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    for _ in 0..1500 {
        let mut s = b"h [os: ".to_vec();
        s.extend_from_slice(&rng.bytes_between(0, 18));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "err35 parse Unix regex failures");
}

/// ERRORS rows 41, 42 — anchored regexes vs sign-looking and huge digit runs.
#[test]
fn err41_parse_numeric_edges() {
    let (c, r) = open_pair();
    let mut cases: Vec<Case> = [
        &b"W [Ver: -1.2.3]"[..],
        &b"W [Ver: +1.2.3]"[..],
        &b"h [os: -1.2]"[..],
        &b"h [os: +1.2]"[..],
        &b"W [Ver: 99999999999999999999.1.2]"[..],
        &b"W [Ver: 2147483647.2147483648.4294967296]"[..],
        &b"h [os: 99999999999999999999.88888888888888888888]"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    for n in [1usize, 100, 1000, 5000] {
        let d: Vec<u8> = std::iter::repeat(b'7').take(n).collect();
        let mut s = b"W [Ver: ".to_vec();
        s.extend_from_slice(&d);
        s.push(b'.');
        s.extend_from_slice(&d);
        s.push(b'.');
        s.extend_from_slice(&d);
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "err41 parse numeric edges");
}

// ===========================================================================
// parse_uname_string — missing sub-markers (rows 29, 33, 34, 37)
// ===========================================================================

/// ERRORS rows 29, 33, 34 — absent `": "` / `" ("` / `"|"`.
#[test]
fn err29_parse_missing_submarkers() {
    let (c, r) = open_pair();
    let mut cases: Vec<Case> = [
        &b"h [ubuntu]"[..],              // no ": "  -> row 29
        &b"h [u]"[..],
        &b"h [a:b]"[..],                 // ':' without space
        &b"h [a :b]"[..],
        &b"h [ubuntu: 22.04]"[..],       // no " ("  -> row 33
        &b"h [ubuntu: 22.04(jammy)]"[..],// '(' without space
        &b"h [ubuntu: 22.04 jammy]"[..],
        &b"h [ubuntu: 22.04 (jammy)]"[..],
        &b"h [ubuntu: 22.04]"[..],       // no "|"   -> row 34
        &b"h [ubuntu|debian: 22.04]"[..],
        &b"h [ubuntu: 22.04|x]"[..],     // '|' after the ": " split is invisible
        &b"h [ubuntu: 22.04 (jammy)|x]"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    // and the same set with a prepopulated os_data, so "left untouched" is
    // distinguishable from "explicitly set to NULL".
    let extra: Vec<Case> = cases
        .iter()
        .filter_map(|cs| match cs {
            Case::Parse { input: Some(v), .. } => Some(Case::parse_prepop(v)),
            _ => None,
        })
        .collect();
    cases.extend(extra);
    assert_same(&c, &r, &cases, "err29 parse missing sub-markers");
}

/// ERRORS row 37 — `get_os_arch` runs on the buffer already truncated at `" ["`.
#[test]
fn err37_parse_arch_hidden_after_truncation() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        let mut a = b"host [os: 1.2 ".to_vec();
        a.extend_from_slice(arch);
        a.push(b']');
        cases.push(Case::parse(&a));
        cases.push(Case::parse_prepop(&a));

        let mut b = b"host [".to_vec();
        b.extend_from_slice(arch);
        b.extend_from_slice(b": 1.2]");
        cases.push(Case::parse(&b));

        let mut d = b" [".to_vec();
        d.extend_from_slice(arch);
        d.push(b']');
        cases.push(Case::parse(&d));
    }
    assert_same(&c, &r, &cases, "err37 parse arch hidden after truncation");
}

// ===========================================================================
// parse_uname_string — out-of-bounds strip_last_char (rows 28, 30, 31, 32)
// ===========================================================================

/// ERRORS row 28 — empty text after `" [Ver: "`: the one-byte-back write lands
/// *inside* the caller's buffer, so the mutated buffer is compared directly.
#[test]
fn err28_parse_ver_empty_tail_writes_into_buffer() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = [
        &b" [Ver: "[..],
        &b"a [Ver: "[..],
        &b"abc [Ver: "[..],
        &b"a [Ver: ]"[..],
        &b"a [Ver:  "[..],
        &b"a [Ver: \t"[..],
        &b"a [Ver: 1"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    assert_same_isolated(&c, &r, &cases, "err28 parse Ver empty tail (UB write)");
}

/// ERRORS rows 30, 31, 32 — `strip_last_char` on an empty `malloc`'d string
/// writes one byte before the allocation (**UB**, but identical on both sides).
#[test]
fn err30_parse_empty_strdup_underflow() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = [
        &b" ["[..],          // row 30: os_name == ""
        &b"a ["[..],
        &b"a [ "[..],
        &b"a [a: "[..],      // row 31: os_version == ""
        &b" [: "[..],
        &b"a [b: "[..],
        &b"a [b: 1 ("[..],   // row 32: os_codename == ""
        &b"a [b:  ("[..],
        &b"a [b: ("[..],
        &b"a [|"[..],
        &b"a [|: "[..],
        &b"a [: ("[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    assert_same_isolated(&c, &r, &cases, "err30 parse empty-strdup underflow (UB)");
}

// ===========================================================================
// parse_uname_string — remaining rows (38, 39, 46)
// ===========================================================================

/// ERRORS row 38 — `os_uname` is never assigned on any path.
#[test]
fn err38_parse_never_touches_os_uname() {
    let (c, r) = open_pair();
    let inputs: &[&[u8]] = &[
        b"",
        b"W [Ver: 10.0.19041]",
        b"W [Ver: abc]",
        b"h [ubuntu|debian: 22.04 (jammy)] x86_64",
        b"h [ubuntu]",
        b"h [ubuntu: 22.04]",
        b"plain x86_64",
        b"plain",
        b" [",
        b" [Ver: ",
    ];
    let cases: Vec<Case> = inputs.iter().map(|s| Case::parse_prepop(s)).collect();
    assert_same_isolated(&c, &r, &cases, "err38 parse leaves os_uname untouched");
}

/// ERRORS row 39 — `" [Ver: "` wins even when a plain `" ["` occurs earlier.
#[test]
fn err39_parse_ver_marker_wins() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = [
        &b"a [b] c [Ver: 1.2.3]"[..],
        &b"a [b: 1.2 (x)] c [Ver: 1.2.3]"[..],
        &b"a [ [Ver: 1.2.3]"[..],
        &b"a [Ver: 1.2.3] [b: 4.5]"[..],
        &b"a [b [Ver: 1.2.3] c]"[..],
        &b"x86_64 [b] c [Ver: 1.2.3]"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    assert_same(&c, &r, &cases, "err39 parse Ver marker precedence");
}

/// ERRORS row 46 — calling twice on an already-populated `os_data` overwrites.
#[test]
fn err46_parse_repeated_overwrites_fields() {
    let (c, r) = open_pair();
    let inputs: &[&[u8]] = &[
        b"W [Ver: 10.0.19041]",
        b"h [ubuntu|debian: 22.04 (jammy)] x86_64",
        b"h [ubuntu]",
        b"plain x86_64",
        b" [Ver: ",
        b" [",
    ];
    let mut cases = Vec::new();
    for s in inputs {
        for times in [1usize, 2, 3, 5] {
            cases.push(Case::parse_n(s, times));
        }
        cases.push(Case::Parse {
            input: Some(s.to_vec()),
            osd_null: false,
            prepopulate: true,
            times: 2,
        });
    }
    assert_same_isolated(&c, &r, &cases, "err46 parse repeated calls overwrite");
}

// ===========================================================================
// Generic FFI boundaries not tied to a single row
// ===========================================================================

/// One step past every documented boundary, in one place: 0/1 lengths, the
/// smallest and largest `nmatch` that still make sense, and every combination
/// of NULL argument for all three entry points.
#[test]
fn err_generic_boundary_matrix() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();

    // get_os_arch: length 0 and 1, and each single byte
    cases.push(Case::arch(b""));
    for b in 1u8..=127 {
        cases.push(Case::arch(&[b]));
    }
    for b in 128u8..=255 {
        cases.push(Case::arch(&[b]));
    }

    // w_regexec: every NULL combination x nmatch {0,1} x pmatch {NULL, [1]}
    for pat in [None, Some(PAT_MAJOR.to_vec())] {
        for subj in [None, Some(b"1".to_vec())] {
            for nmatch in [0usize, 1] {
                for arr in [None, Some(1usize)] {
                    if nmatch > 0 && arr.is_none() && pat.is_some() && subj.is_some() {
                        continue; // covered (and faults) in err14
                    }
                    cases.push(Case::Regex {
                        pat: pat.clone(),
                        subj: subj.clone(),
                        nmatch,
                        arr,
                    });
                }
            }
        }
    }

    // parse_uname_string: length 0 and 1, every single byte
    cases.push(Case::parse(b""));
    for b in 1u8..=255 {
        cases.push(Case::parse(&[b]));
    }
    // and every two-byte string over the marker alphabet
    const A: &[u8] = b" [](:|)V0.";
    for x in A {
        for y in A {
            cases.push(Case::parse(&[*x, *y]));
            for z in A {
                cases.push(Case::parse(&[*x, *y, *z]));
            }
        }
    }
    assert_same(&c, &r, &cases, "err generic boundary matrix");
}
