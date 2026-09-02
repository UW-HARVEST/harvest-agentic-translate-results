//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH shared
//! objects through their exported symbols, and asserts the SAME error code or
//! sentinel (not merely "both failed").

mod common;

use common::*;

// ===========================================================================
// w_regexec rejections
// ===========================================================================

/// E1 — `pattern == NULL` → return 0, `pmatch` untouched.
#[test]
fn e1_pattern_null() {
    for subj in [
        &b""[..],
        &b"1.2.3"[..],
        &b"x"[..],
        &[0xFFu8, 0xFE][..],
        &b"a very long subject string used to make staleness observable"[..],
    ] {
        for nmatch in [0usize, 1, 2, 4] {
            diff_regexec(None, Some(subj), nmatch, 4, false, "E1");
        }
    }
    // and via the sequence driver so an earlier successful match leaves state
    diff_regexec_seq(
        &[
            RegCall {
                pattern: Some(br"^([0-9]+)\.*"),
                subject: Some(b"123456"),
                nmatch: 2,
            },
            RegCall {
                pattern: None,
                subject: Some(b"123456"),
                nmatch: 2,
            },
        ],
        2,
        "E1 seq",
    );
}

/// E2 — `string == NULL` → return 0, `pmatch` untouched.
#[test]
fn e2_string_null() {
    for pat in [
        &br""[..],
        &br"^([0-9]+)\.*"[..],
        &br"["[..], // invalid AND string NULL: the NULL check must win first,
        &br"("[..], // so no diagnostic may be printed
    ] {
        for nmatch in [0usize, 1, 2, 4] {
            diff_regexec(Some(pat), None, nmatch, 4, false, "E2");
        }
    }
    // the null check happens before regcomp, so stderr must stay empty
    let (r, err) = diff_regexec_with_stderr(Some(br"["), None, 2, 2, "E2 no diagnostic");
    assert_eq!(r, 0, "E2 must return 0");
    assert!(
        err.is_empty(),
        "the NULL-string check precedes regcomp, so nothing may be printed; got {:?}",
        String::from_utf8_lossy(&err)
    );
}

/// E3 — both pointers NULL → return 0.
#[test]
fn e3_both_null() {
    for nmatch in [0usize, 1, 2, 8] {
        diff_regexec(None, None, nmatch, 8, false, "E3");
    }
    diff_regexec(None, None, 0, 1, true, "E3 pmatch null too");
    let (r, err) = diff_regexec_with_stderr(None, None, 2, 2, "E3 no diagnostic");
    assert_eq!(r, 0);
    assert!(err.is_empty());
}

/// E4 — `regcomp` failure: same return value AND the same `stderr` diagnostic.
#[test]
fn e4_regcomp_failure() {
    let bad: &[&[u8]] = &[
        br"[",
        br"[a",
        br"[z-a]",
        br"(",
        br")",
        br"(()",
        br"a{2,1}",
        br"a{",
        br"*",
        br"+",
        br"?",
        br"\\",
        br"[[:bogus:]]",
        br"[[.x.]]",
        br"a**+",
        br"{1}",
        br"(|",
        br"[]",
        br"[^]",
        br"a\",
    ];
    let mut failures = 0usize;
    for pat in bad {
        let (r, err) = diff_regexec_with_stderr(
            Some(pat),
            Some(b"1.2.3"),
            2,
            2,
            &format!("E4 {:?}", String::from_utf8_lossy(pat)),
        );
        if r == 0 && !err.is_empty() {
            failures += 1;
            // the diagnostic must be exactly the C format string
            let expect = format!(
                "Couldn't compile regular expression '{}'\n",
                String::from_utf8_lossy(pat)
            );
            assert_eq!(
                String::from_utf8_lossy(&err),
                expect,
                "unexpected diagnostic text for pattern {:?}",
                String::from_utf8_lossy(pat)
            );
        }
    }
    assert!(
        failures >= 8,
        "expected the corpus to actually trigger regcomp failures, got {}",
        failures
    );
}

/// E5 — valid pattern that does not match → return 0.
#[test]
fn e5_no_match() {
    let cases: &[(&[u8], &[u8])] = &[
        (br"^([0-9]+)\.*", b"abc"),
        (br"^([0-9]+)\.*", b""),
        (br"^[0-9]+\.([0-9]+)\.*", b"7"),
        (br"^[0-9]+\.([0-9]+)\.*", b"7.x"),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"7.8"),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"7.8.x"),
        (br"^$", b"a"),
        (br"zzz", b"aaa"),
    ];
    for (p, s) in cases {
        diff_regexec(Some(p), Some(s), 2, 2, false, "E5");
    }
    // randomized non-matching subjects for the library's own patterns
    let mut rng = Rng::new(0xE005);
    for _ in 0..4000 {
        let s = plain_token(&mut rng, 0, 12).replace(|c: char| c.is_ascii_digit(), "q");
        for p in LIB_PATTERNS.iter() {
            diff_regexec(Some(p.as_bytes()), Some(s.as_bytes()), 2, 2, false, "E5 rnd");
        }
    }
}

/// E6 — `nmatch == 0` with `pmatch == NULL`.
#[test]
fn e6_nmatch_zero_null_pmatch() {
    let cases: &[(&[u8], &[u8])] = &[
        (br"^([0-9]+)\.*", b"10.0"),
        (br"^([0-9]+)\.*", b"abc"),
        (br"", b""),
        (br"x", b"x"),
        (br"x", b"y"),
    ];
    for (p, s) in cases {
        diff_regexec(Some(p), Some(s), 0, 1, true, "E6");
    }
}

/// E7 — `nmatch` larger than `1 + re_nsub`: surplus slots become `{-1,-1}`.
#[test]
fn e7_oversized_nmatch() {
    // 1 group -> slots 2..8 exercise the surplus
    for nmatch in [2usize, 3, 4, 8, 16] {
        diff_regexec(
            Some(br"^([0-9]+)\.*"),
            Some(b"1234.5"),
            nmatch,
            16,
            false,
            &format!("E7 nmatch={}", nmatch),
        );
        diff_regexec(
            Some(br"^([0-9]+)\.*"),
            Some(b"abc"),
            nmatch,
            16,
            false,
            &format!("E7 nomatch nmatch={}", nmatch),
        );
    }
    // no groups at all
    for nmatch in [1usize, 2, 5, 16] {
        diff_regexec(Some(br"[0-9]+"), Some(b"77"), nmatch, 16, false, "E7 nogroup");
    }
}

/// E8 — `nmatch` smaller than the group count: `pmatch[1]` must stay stale.
#[test]
fn e8_undersized_nmatch() {
    for nmatch in [1usize, 2, 3] {
        diff_regexec(
            Some(br"^([0-9]+)\.([0-9]+)\.([0-9]+)$"),
            Some(b"1.22.333"),
            nmatch,
            8,
            false,
            &format!("E8 nmatch={}", nmatch),
        );
    }
    // sequence: a wide match first, then an undersized nmatch, so the leftover
    // slots hold the *previous* offsets in both libraries
    diff_regexec_seq(
        &[
            RegCall {
                pattern: Some(br"^([0-9]+)\.([0-9]+)\.([0-9]+)$"),
                subject: Some(b"111.222.333"),
                nmatch: 4,
            },
            RegCall {
                pattern: Some(br"^([0-9]+)"),
                subject: Some(b"9"),
                nmatch: 1,
            },
        ],
        4,
        "E8 seq",
    );
}

/// E9 — a capture group that does not participate reports `{-1,-1}`.
#[test]
fn e9_nonparticipating_group() {
    let cases: &[(&[u8], &[u8])] = &[
        (br"^(a)|(b)$", b"b"),
        (br"^(a)|(b)$", b"a"),
        (br"(x)?y", b"y"),
        (br"(x)?y", b"xy"),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"1.2.3"),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"1.2.3.4"),
        (br"(a)|(b)|(c)", b"c"),
    ];
    for (p, s) in cases {
        for nmatch in [2usize, 3, 4] {
            diff_regexec(Some(p), Some(s), nmatch, 4, false, "E9");
        }
    }
    // through the real pipeline: the build regex's inner group is exactly this
    // shape, and parse_uname_string reads match[1] which IS group 1
    for v in ["1.2.3", "1.2.3.4", "1.2.3.4.5"] {
        diff_parse(format!("N [Ver: {}]", v).as_bytes(), true, "E9 pipeline");
    }
}

/// E30 — empty subject with a pattern that cannot match it.
#[test]
fn e30_empty_subject() {
    for p in [
        &br"^([0-9]+)\.*"[..],
        &br"."[..],
        &br"a"[..],
        &br"[[:alpha:]]"[..],
        &br"^a*$"[..],
        &br"^$"[..],
    ] {
        diff_regexec(Some(p), Some(b""), 2, 2, false, "E30");
    }
}

/// E31 — empty pattern: glibc accepts it and it matches at offset 0.
#[test]
fn e31_empty_pattern() {
    for s in [&b""[..], &b"a"[..], &b"1.2.3"[..], &[0x80u8][..]] {
        let (r, err) = diff_regexec_with_stderr(Some(b""), Some(s), 2, 2, "E31");
        assert!(
            err.is_empty(),
            "an empty ERE compiles fine, so nothing may be printed"
        );
        assert_eq!(r, 1, "empty pattern matches everywhere");
    }
}

/// E32 — stale `pmatch` reuse across calls (the shape `parse_uname_string`
/// creates with its shared `regmatch_t match[2]`).
#[test]
fn e32_stale_pmatch_reuse() {
    // long match, then a failing match, then a short match
    diff_regexec_seq(
        &[
            RegCall {
                pattern: Some(br"^([0-9]+)\.*"),
                subject: Some(b"123456789012345"),
                nmatch: 2,
            },
            RegCall {
                pattern: Some(br"^[0-9]+\.([0-9]+)\.*"),
                subject: Some(b"123456789012345"),
                nmatch: 2,
            },
            RegCall {
                pattern: Some(br"^([0-9]+)\.*"),
                subject: Some(b"7"),
                nmatch: 2,
            },
            RegCall {
                pattern: None,
                subject: Some(b"7"),
                nmatch: 2,
            },
            RegCall {
                pattern: Some(br"["),
                subject: Some(b"7"),
                nmatch: 2,
            },
        ],
        2,
        "E32 seq",
    );
    // and the same interleaving driven through parse_uname_string
    let mut rng = Rng::new(0xE032);
    for _ in 0..3000 {
        let a = digits(&mut rng, 1, 14);
        let b = digits(&mut rng, 1, 14);
        let v = match rng.below(3) {
            0 => a.clone(),
            1 => format!("{}.{}", a, b),
            _ => format!("{}.{}.{}", a, b, a),
        };
        diff_parse(format!("N [Ver: {}]", v).as_bytes(), true, "E32 pipeline");
        diff_parse(format!("H [D: {}]", v).as_bytes(), true, "E32 pipeline unix");
    }
}

// ===========================================================================
// get_os_arch rejections
// ===========================================================================

/// E10 — no architecture literal present → NULL.
#[test]
fn e10_arch_not_found() {
    let cases: &[&str] = &[
        "Linux host 5.15.0-88-generic",
        "no arch here",
        "x86-64",
        "X86_64",
        "I386",
        "ARMV7",
        "Aarch64",
        "aix",
        "sparc64" /* contains "sparc" -> actually found; kept to prove the
                  * differential test is meaningful either way */,
        "amd 64",
        "ia 64",
        "arm 64",
    ];
    for c in cases {
        diff_arch(c.as_bytes(), &format!("E10 {:?}", c));
    }
    // exhaustive single-byte and two-byte haystacks: none can contain a literal
    for a in 1u8..=255 {
        diff_arch(&[a], "E10 1byte");
    }
    let mut rng = Rng::new(0xE010);
    for _ in 0..3000 {
        let n = rng.range(2, 3);
        let v: Vec<u8> = (0..n).map(|_| (rng.below(255) + 1) as u8).collect();
        diff_arch(&v, "E10 short");
    }
}

/// E11 — empty haystack → NULL.
#[test]
fn e11_arch_empty_string() {
    diff_arch(b"", "E11");
    // also through the pipeline: an empty uname reaches get_os_arch("")
    diff_parse(b"", true, "E11 pipeline");
    diff_parse(b"", false, "E11 pipeline zeroed");
}

// ===========================================================================
// parse_uname_string rejections
// ===========================================================================

/// E12 — `osd == NULL`: return before touching `uname`.
#[test]
fn e12_osd_null() {
    let cases: &[&str] = &[
        "",
        "Microsoft Windows [Ver: 10.0.19041.1]",
        "Linux x86_64 [Ubuntu: 20.04 (focal)]",
        "Linux|linux [D: 1.0]",
        "plain",
        "N [",
        "N [Ver: ",
        "x86_64",
    ];
    for c in cases {
        diff_parse_null_osd(c.as_bytes(), &format!("E12 {:?}", c));
    }
    let mut rng = Rng::new(0xE012);
    for _ in 0..3000 {
        let s = marker_token(&mut rng, 0, 32);
        diff_parse_null_osd(s.as_bytes(), "E12 rnd");
    }
}

/// E13 — neither `" [Ver: "` nor `" ["` present: only `os_arch` may be set.
#[test]
fn e13_no_bracket_marker() {
    let cases: &[&str] = &[
        "Linux host x86_64",
        "SunOS i86pc",
        "AIX 7 2",
        "no markers at all",
        "[Ver: 1.0]",
        "[x: 1]",
        "a: b (c)|d",
        "x86_64: 1.0 (y)|z",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E13 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E13 zeroed {:?}", c));
    }
    // randomized: build strings from an alphabet that can never form " ["
    let mut rng = Rng::new(0xE013);
    for _ in 0..4000 {
        let mut s = plain_token(&mut rng, 0, 24);
        if rng.boolean() {
            s.push_str(rng.pick(&ARCHS));
        }
        s.push_str(&plain_token(&mut rng, 0, 8));
        assert!(!s.contains(" ["));
        diff_parse(s.as_bytes(), true, "E13 rnd");
    }
}

/// E14 — no `" ["` and no architecture: nothing at all is written.
#[test]
fn e14_nothing_written() {
    let cases: &[&str] = &[
        "",
        "plain",
        "Linux host",
        "1.2.3",
        "a: b",
        "a (b)",
        "a|b",
        "]",
        "[",
        "no-arch-here",
    ];
    for c in cases {
        assert!(!c.contains(" ["), "case must not take the Unix branch");
        for a in ARCHS.iter() {
            assert!(!c.contains(a), "case must not contain an arch literal");
        }
        diff_parse(c.as_bytes(), true, &format!("E14 {:?}", c));
    }
}

/// E15 — Windows branch, non-numeric version: all three regexes fail.
#[test]
fn e15_windows_non_numeric() {
    let vers: &[&str] = &["abc", "v1.2", " 1.2", "-1", ".1.2", "x86_64", "?", "a1.2.3"];
    for v in vers {
        let s = format!("N [Ver: {}]", v);
        diff_parse(s.as_bytes(), true, &format!("E15 {:?}", v));
        diff_parse(s.as_bytes(), false, &format!("E15 zeroed {:?}", v));
    }
    let mut rng = Rng::new(0xE015);
    for _ in 0..3000 {
        let mut v = String::from(*rng.pick(&["a", "-", ".", " ", "_", "+", "/"]));
        v.push_str(&version(&mut rng, 1, 3));
        let s = format!("N [Ver: {}]", v);
        diff_parse(s.as_bytes(), true, "E15 rnd");
    }
}

/// E16 — Windows branch, major only: the minor and build regexes fail.
#[test]
fn e16_windows_major_only() {
    let mut rng = Rng::new(0xE016);
    for v in ["10", "0", "6", "000", "99999999999999999999"] {
        let s = format!("N [Ver: {}]", v);
        diff_parse(s.as_bytes(), true, &format!("E16 {:?}", v));
    }
    for _ in 0..3000 {
        let v = digits(&mut rng, 1, 8);
        // a trailing run of dots still leaves minor/build unmatched
        let dots: String = std::iter::repeat('.').take(rng.below(4)).collect();
        let s = format!("N [Ver: {}{}]", v, dots);
        diff_parse(s.as_bytes(), true, "E16 rnd");
        diff_parse(s.as_bytes(), false, "E16 rnd zeroed");
    }
}

/// E17 — Windows branch, `major.minor` only: the build regex fails.
#[test]
fn e17_windows_no_build() {
    let mut rng = Rng::new(0xE017);
    for v in ["6.1", "0.0", "10.0", "6.1.", "6.1..", "6.1.x"] {
        let s = format!("N [Ver: {}]", v);
        diff_parse(s.as_bytes(), true, &format!("E17 {:?}", v));
    }
    for _ in 0..3000 {
        let s = format!("N [Ver: {}]", version_n(&mut rng, 2));
        diff_parse(s.as_bytes(), true, "E17 rnd");
    }
}

/// E18 — Unix branch, non-numeric version: major/minor regexes fail.
#[test]
fn e18_unix_non_numeric() {
    let vers: &[&str] = &["rolling", "unstable", "", "-1", ".5", "v20.04", "x", "a1"];
    for v in vers {
        for code in [None, Some("cn")] {
            let s = match code {
                Some(c) => format!("H [D: {} ({})]", v, c),
                None => format!("H [D: {}]", v),
            };
            diff_parse(s.as_bytes(), true, &format!("E18 {:?}", v));
            diff_parse(s.as_bytes(), false, &format!("E18 zeroed {:?}", v));
        }
    }
}

/// E19 — Unix branch, no `": "`: the last byte of `os_name` is dropped and
/// nothing else is written.
#[test]
fn e19_unix_no_colon() {
    let cases: &[&str] = &[
        "H [Ubuntu]",
        "H [Ubuntu",
        "H [a:b]",
        "H [a :b]",
        "H [:]",
        "H [ ]",
        "H []",
        "H [",
        "H [x",
        "H [20.04]",
        "H [a|b]",
        "H [a (b)]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E19 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E19 zeroed {:?}", c));
    }
    let mut rng = Rng::new(0xE019);
    for _ in 0..4000 {
        let inner = plain_token(&mut rng, 0, 20);
        assert!(!inner.contains(": "));
        let s = format!("{} [{}]", plain_token(&mut rng, 0, 8), inner);
        diff_parse(s.as_bytes(), true, "E19 rnd");
    }
}

/// E20 — Unix branch with `": "` but no `" ("`: no codename.
#[test]
fn e20_unix_no_codename() {
    let cases: &[&str] = &[
        "H [D: 20.04]",
        "H [D: 20.04(focal)]",
        "H [D: 20.04 focal]",
        "H [D: (focal)]",
        "H [D: ]",
        "H [D: ",
        "H [D:  ]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E20 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E20 zeroed {:?}", c));
    }
    let mut rng = Rng::new(0xE020);
    for _ in 0..3000 {
        let v = version(&mut rng, 1, 3);
        let s = format!("H [D: {}]", v);
        assert!(!s.contains(" ("));
        diff_parse(s.as_bytes(), true, "E20 rnd");
    }
}

/// E21 — Unix branch with no `"|"`: `os_platform` stays untouched (unlike the
/// Windows branch, which always sets it to `"windows"`).
#[test]
fn e21_unix_no_platform() {
    let cases: &[&str] = &[
        "H [Ubuntu: 20.04 (focal)]",
        "H [Ubuntu: 20.04]",
        "H [Ubuntu]",
        "H [",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E21 {:?}", c));
    }
    let mut rng = Rng::new(0xE021);
    for _ in 0..3000 {
        let d = plain_token(&mut rng, 0, 12);
        assert!(!d.contains('|'));
        let s = format!("H [{}: {}]", d, version(&mut rng, 1, 3));
        diff_parse(s.as_bytes(), true, "E21 rnd");
    }
    // contrast: the Windows branch always sets os_platform
    diff_parse(b"H [Ver: 1.2.3]", true, "E21 windows contrast");
}

/// E22 — `"|"` present but after `": "`, so the truncation removes it.
#[test]
fn e22_pipe_after_colon() {
    let cases: &[&str] = &[
        "H [D: 20.04|x]",
        "H [D: 20.04 (focal|x)]",
        "H [D: |]",
        "H [D: 1.0|2.0|3.0]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E22 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E22 zeroed {:?}", c));
    }
    let mut rng = Rng::new(0xE022);
    for _ in 0..3000 {
        let d = plain_token(&mut rng, 1, 8);
        let tail = plain_token(&mut rng, 0, 8);
        let s = format!("H [{}: {}|{}]", d, version(&mut rng, 1, 3), tail);
        diff_parse(s.as_bytes(), true, "E22 rnd");
    }
}

/// E23 — architecture literal after `" ["`, cut off by the truncation.
#[test]
fn e23_arch_after_bracket() {
    for arch in ARCHS.iter() {
        for s in [
            format!("H [{}: 1.0]", arch),
            format!("H [D: 1.0 ({})]", arch),
            format!("H [D|{}: 1.0]", arch),
            format!("H [{}]", arch),
        ] {
            diff_parse(s.as_bytes(), true, &format!("E23 {}", arch));
            diff_parse(s.as_bytes(), false, &format!("E23 zeroed {}", arch));
        }
    }
}

/// E24 — the Windows branch never calls `get_os_arch`, so `os_arch` stays
/// untouched even when the architecture is right there in the string.
#[test]
fn e24_windows_never_sets_arch() {
    for arch in ARCHS.iter() {
        for s in [
            format!("{} [Ver: 10.0.1]", arch),
            format!("Windows {} [Ver: 6.1.7601]", arch),
            format!("Windows [Ver: 6.1.7601 {}]", arch),
            format!("{} [Ver: ]", arch),
        ] {
            // poisoned: proves the field is not merely NULL but truly untouched
            diff_parse(s.as_bytes(), true, &format!("E24 {}", arch));
            diff_parse(s.as_bytes(), false, &format!("E24 zeroed {}", arch));
        }
    }
}

/// E25 — Windows branch, zero-length version remainder: the strip writes one
/// byte before `str_tmp`, inside the caller's buffer.
#[test]
fn e25_windows_empty_version() {
    let cases: &[&str] = &[
        "N [Ver: ",
        " [Ver: ",
        "[Ver: ",
        "Microsoft Windows [Ver: ",
        "x86_64 [Ver: ",
        "a|b [Ver: ",
        "a: b [Ver: ",
    ];
    for c in cases {
        // the compared output includes the whole buffer + slack, so the
        // out-of-bounds byte is part of the assertion
        diff_parse(c.as_bytes(), true, &format!("E25 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E25 zeroed {:?}", c));
    }
    let mut rng = Rng::new(0xE025);
    for _ in 0..3000 {
        let s = format!("{} [Ver: ", marker_token(&mut rng, 0, 16));
        diff_parse(s.as_bytes(), true, "E25 rnd");
    }
}

/// E26 — Unix branch, zero-length version after the strip.
#[test]
fn e26_unix_empty_version() {
    let cases: &[&str] = &[
        "H [D: ]",
        "H [D: ",
        "H [D:  ",
        "H [D: x",
        "H [D: 1",
        "H [|D: ]",
        "H [D|p: ]",
        "H [D: ()]",
        "H [D:  (a)]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E26 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E26 zeroed {:?}", c));
    }
}

/// E27 — Unix branch, zero-length `os_name`.
#[test]
fn e27_unix_empty_name() {
    let cases: &[&str] = &["H [", " [", "[", "x86_64 [", "a|b [", "H [x", "H []"];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E27 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E27 zeroed {:?}", c));
    }
    let mut rng = Rng::new(0xE027);
    for _ in 0..3000 {
        let s = format!("{} [", plain_token(&mut rng, 0, 16));
        diff_parse(s.as_bytes(), true, "E27 rnd");
    }
}

/// E28 — Unix branch, zero-length codename.
#[test]
fn e28_unix_empty_codename() {
    let cases: &[&str] = &[
        "H [D: 1.0 (]",
        "H [D: 1.0 (",
        "H [D: 1.0 ()]",
        "H [D: 1.0 ()",
        "H [D: 1.0 ( )]",
        "H [D: 1.0 (x",
        "H [D: 1.0 (()]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), true, &format!("E28 {:?}", c));
        diff_parse(c.as_bytes(), false, &format!("E28 zeroed {:?}", c));
    }
}

/// E29 — empty `uname`.
#[test]
fn e29_uname_empty() {
    diff_parse(b"", true, "E29 poisoned");
    diff_parse(b"", false, "E29 zeroed");
    diff_parse_null_osd(b"", "E29 null osd");
}

// ===========================================================================
// Generic FFI boundary sweeps required by the phase, beyond the table
// ===========================================================================

/// Every single-byte and (exhaustively) every two-byte input to
/// `parse_uname_string` and `get_os_arch`. Two bytes is enough to form `" ["`
/// (the shortest marker) and `"["`, so this covers the shortest reachable
/// paths, including the empty-string strip.
#[test]
fn boundary_exhaustive_short_inputs() {
    for a in 1u8..=255 {
        diff_arch(&[a], "1byte arch");
        diff_parse(&[a], true, "1byte parse");
        diff_parse_null_osd(&[a], "1byte null osd");
    }
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            diff_parse(&[a, b], true, "2byte parse");
        }
    }
    // three bytes, restricted to the marker alphabet, so " [V" etc. are covered
    let alpha: &[u8] = b" [](:|)Ver0.x";
    for &a in alpha {
        for &b in alpha {
            for &c in alpha {
                diff_parse(&[a, b, c], true, "3byte parse");
            }
        }
    }
}

/// `nmatch` sweep at and past every interesting boundary, for a pattern with a
/// known group count. `size_t` has no invalid value the library rejects, so the
/// contract is "identical behaviour for every value the caller's buffer allows".
#[test]
fn boundary_nmatch_sweep() {
    let slots = 12usize;
    for nmatch in 0..=slots {
        for (p, s) in [
            (&br"^([0-9]+)\.([0-9]+)\.([0-9]+)$"[..], &b"1.2.3"[..]),
            (&br"^([0-9]+)\.([0-9]+)\.([0-9]+)$"[..], &b"nope"[..]),
            (&br"[0-9]+"[..], &b"42"[..]),
            (&br"^([0-9]+)\.*"[..], &b"7."[..]),
        ] {
            diff_regexec(
                Some(p),
                Some(s),
                nmatch,
                slots,
                false,
                &format!("nmatch sweep {}", nmatch),
            );
        }
    }
    // nmatch == 0 is the only value that legally pairs with a NULL pmatch
    diff_regexec(Some(br"[0-9]+"), Some(b"42"), 0, 1, true, "nmatch 0 null");
}

/// The three exported symbols invoked with every combination of NULL / non-NULL
/// pointer arguments that the C actually guards against.
#[test]
fn boundary_null_matrix() {
    // w_regexec: 2x2 pointer matrix
    for p in [None, Some(&br"^([0-9]+)"[..])] {
        for s in [None, Some(&b"12"[..])] {
            diff_regexec(p, s, 2, 2, false, "null matrix");
            if p.is_none() || s.is_none() {
                // rejected before regcomp, so no diagnostic
                let (_r, err) = diff_regexec_with_stderr(p, s, 2, 2, "null matrix stderr");
                assert!(err.is_empty());
            }
        }
    }
    // parse_uname_string: osd == NULL for every branch shape
    for u in [
        "",
        "N [Ver: 1.2.3]",
        "H [D: 1.0 (c)]",
        "x86_64",
        "H [",
        "N [Ver: ",
    ] {
        diff_parse_null_osd(u.as_bytes(), "null matrix osd");
    }
}

/// Values one step past every documented range: the 12 architecture literals
/// perturbed by ±1 in each byte position (so the near-miss neighbourhood of
/// each accepted token is covered), and version numbers straddling the
/// `int` / `regoff_t` boundaries.
#[test]
fn boundary_one_past_range() {
    for arch in ARCHS.iter() {
        let bytes = arch.as_bytes();
        for i in 0..bytes.len() {
            for delta in [-1i16, 1] {
                let mut v = bytes.to_vec();
                v[i] = (v[i] as i16 + delta) as u8;
                diff_arch(&v, &format!("one-past {} @{}", arch, i));
                let s = [&v[..], b" [D: 1.0]"].concat();
                diff_parse(&s, true, "one-past parse");
            }
        }
        // truncated by one byte at each end
        diff_arch(&bytes[1..], "one-short head");
        diff_arch(&bytes[..bytes.len() - 1], "one-short tail");
    }
    // numeric edges: values around 2^31 and 2^32 in every version position
    let edges: &[&str] = &[
        "2147483647", "2147483648", "4294967295", "4294967296", "9223372036854775807",
        "18446744073709551616",
    ];
    for e in edges {
        for v in [
            format!("{}", e),
            format!("{}.{}", e, e),
            format!("{}.{}.{}", e, e, e),
            format!("1.2.{}", e),
        ] {
            diff_parse(format!("N [Ver: {}]", v).as_bytes(), true, "one-past num win");
            diff_parse(format!("H [D: {}]", v).as_bytes(), true, "one-past num unix");
        }
    }
}
