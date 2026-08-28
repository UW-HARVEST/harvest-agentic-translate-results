//! Level 2: `w_regexec` — the regex wrapper used by `parse_uname_string`.

mod common;

use common::*;

/// (pattern, subject) pairs. Includes the three patterns the parser uses,
/// plus cases that distinguish POSIX extended from basic regex syntax.
fn regex_cases() -> Vec<(&'static [u8], &'static [u8])> {
    vec![
        // The exact patterns used by parse_uname_string.
        (b"^([0-9]+)\\.*", b"10.0.19045.3803"),
        (b"^[0-9]+\\.([0-9]+)\\.*", b"10.0.19045.3803"),
        (b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", b"10.0.19045.3803"),
        (b"^([0-9]+)\\.*", b"6.1.7601"),
        (b"^[0-9]+\\.([0-9]+)\\.*", b"6.1.7601"),
        (b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", b"6.1.7601"),
        (b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", b"1.2.3.4.5.6"),
        (b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", b"1.2.3."),
        // No match.
        (b"^([0-9]+)\\.*", b"abc"),
        (b"^([0-9]+)\\.*", b""),
        (b"^[0-9]+\\.([0-9]+)\\.*", b"10"),
        (b"^[0-9]+\\.([0-9]+)\\.*", b"10."),
        (b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", b"10.0"),
        // Multi-digit and leading zeros.
        (b"^([0-9]+)\\.*", b"0000123.456"),
        (b"^([0-9]+)\\.*", b"99999999999999999999.1"),
        // Extended-vs-basic discriminators: these only behave this way with
        // REG_EXTENDED, so they pin down the cflags value.
        (b"a+", b"aaa"),
        (b"a+", b"a+"),
        (b"(ab)+", b"abab"),
        (b"a|b", b"b"),
        (b"a{2,3}", b"aaa"),
        (b"x?y", b"y"),
        (b"^(foo|bar)baz$", b"barbaz"),
        (b"()", b"anything"),
        // Anchors, classes, backrefs-free constructs.
        (b"[[:digit:]]+", b"abc123"),
        (b"[[:alpha:]]+", b"123abc"),
        (b"^$", b""),
        (b".*", b"whatever"),
        (b"(.)(.)", b"xy"),
        // Subject with newline and high bytes.
        (b"^([0-9]+)\\.*", b"12\n34"),
        (b".", b"\xff"),
    ]
}

fn invalid_patterns() -> Vec<&'static [u8]> {
    vec![
        b"(",
        b")",
        b"[",
        b"[z-a]",
        b"*",
        b"a{2,1}",
        b"\\",
        b"(|",
        b"a**+",
    ]
}

#[test]
fn w_regexec_matches_c() {
    let (c, rust) = load_both();

    for (pat, subj) in regex_cases() {
        for nmatch in [0usize, 1, 2, 3] {
            let slots = 4; // always give the callee room; compare all of it
            let (rc, mc) = c.w_regexec(Some(pat), Some(subj), nmatch, slots);
            let (rr, mr) = rust.w_regexec(Some(pat), Some(subj), nmatch, slots);
            assert_eq!(
                rc,
                rr,
                "w_regexec return: pattern={:?} subject={:?} nmatch={nmatch}",
                String::from_utf8_lossy(pat),
                String::from_utf8_lossy(subj),
            );
            assert_eq!(
                mc,
                mr,
                "w_regexec pmatch: pattern={:?} subject={:?} nmatch={nmatch}\nC    = {mc:?}\nRust = {mr:?}",
                String::from_utf8_lossy(pat),
                String::from_utf8_lossy(subj),
            );
        }
    }
}

#[test]
fn w_regexec_null_arguments_match_c() {
    let (c, rust) = load_both();

    let cases: Vec<(Option<&[u8]>, Option<&[u8]>)> = vec![
        (None, None),
        (None, Some(b"subject")),
        (Some(b"^([0-9]+)"), None),
    ];

    for (pat, subj) in cases {
        let (rc, mc) = c.w_regexec(pat, subj, 2, 4);
        let (rr, mr) = rust.w_regexec(pat, subj, 2, 4);
        assert_eq!(rc, rr, "w_regexec return with NULL args: {pat:?} {subj:?}");
        // Neither implementation may touch pmatch on the early-out path.
        assert_eq!(mc, mr, "w_regexec pmatch with NULL args");
        assert!(
            mc.iter().all(|m| m.rm_so == -7 && m.rm_eo == -7),
            "pmatch was written on the NULL early-out path: {mc:?}"
        );
    }
}

#[test]
fn w_regexec_invalid_pattern_matches_c() {
    let (c, rust) = load_both();

    for pat in invalid_patterns() {
        let (rc, mc) = c.w_regexec(Some(pat), Some(b"subject"), 2, 4);
        let (rr, mr) = rust.w_regexec(Some(pat), Some(b"subject"), 2, 4);
        assert_eq!(
            rc,
            rr,
            "w_regexec return for invalid pattern {:?}",
            String::from_utf8_lossy(pat)
        );
        assert_eq!(
            mc,
            mr,
            "w_regexec pmatch for invalid pattern {:?}",
            String::from_utf8_lossy(pat)
        );
    }
}

#[test]
fn w_regexec_is_reentrant() {
    // The C compiles and frees a regex per call; repeated calls must be stable
    // (this also catches any leaked/corrupted regex_t storage in the Rust).
    let (c, rust) = load_both();

    for _ in 0..200 {
        let (rc, mc) = c.w_regexec(Some(b"^[0-9]+\\.([0-9]+)\\.*"), Some(b"10.0.19045"), 2, 2);
        let (rr, mr) = rust.w_regexec(Some(b"^[0-9]+\\.([0-9]+)\\.*"), Some(b"10.0.19045"), 2, 2);
        assert_eq!(rc, rr);
        assert_eq!(mc, mr);
        assert_eq!(rc, 1);
    }
}

