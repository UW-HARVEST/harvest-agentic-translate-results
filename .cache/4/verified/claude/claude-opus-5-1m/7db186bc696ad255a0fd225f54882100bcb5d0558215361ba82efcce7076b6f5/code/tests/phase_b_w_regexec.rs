//! Phase B — valid-path differential tests for the low-level `w_regexec`.
//! Covers CONFIGS.md rows 7-12.

mod common;
use common::*;

/// Build a random version-ish string: digits, dots, letters, parentheses.
fn random_version(rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::new();
    let comps = rng.range(0, 5);
    for i in 0..comps {
        if i > 0 {
            v.push(b'.');
        }
        match rng.below(6) {
            0 => v.extend_from_slice(b"0"),
            1 => v.extend_from_slice(b"00"),
            2 => {
                // long numeric component
                let n = rng.range(1, 12);
                for _ in 0..n {
                    v.push(b'0' + rng.below(10) as u8);
                }
            }
            3 => v.extend_from_slice(b"rolling"),
            4 => {
                let n = rng.range(1, 3);
                for _ in 0..n {
                    v.push(b'0' + rng.below(10) as u8);
                }
            }
            _ => {
                let alt = *rng.pick(&[b"x".as_slice(), b"9", b"04", b"11", b""]);
                v.extend_from_slice(alt);
            }
        }
    }
    if rng.chance(4) {
        v.extend_from_slice(b" LTS");
    }
    if rng.chance(5) {
        v.extend_from_slice(b" (codename)");
    }
    if rng.chance(6) {
        v.push(b']');
    }
    v
}

/// Row 7: the five patterns `parse_uname_string` actually compiles, over
/// randomized version strings, `nmatch = 2` (exactly what the C uses).
#[test]
fn row07_production_patterns_over_random_versions() {
    let mut rng = Rng::new(0x2007);
    for (pi, pat) in PATTERNS.iter().enumerate() {
        for iter in 0..3000 {
            let subj = random_version(&mut rng);
            diff_w_regexec(
                &format!("row7 pat#{pi} iter={iter}"),
                Some(pat.as_bytes()),
                Some(&subj),
                2,
                Some(4),
            );
        }
    }
}

/// Row 8: `nmatch` swept over {0,1,2,3,8,64} against a 64-slot buffer, so the
/// exact number of slots glibc writes is compared.
#[test]
fn row08_nmatch_sweep() {
    let mut rng = Rng::new(0x2008);
    let nmatches = [0usize, 1, 2, 3, 8, 64];
    let pats: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.([0-9]+)\.*",
        br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        br"(a)(b)(c)(d)(e)",
        br"((((x))))",
        br"nogroups",
    ];
    for pat in pats {
        for &nm in &nmatches {
            for iter in 0..120 {
                let subj = if iter % 3 == 0 {
                    random_version(&mut rng)
                } else if iter % 3 == 1 {
                    b"abcde".to_vec()
                } else {
                    b"xxnogroupsxx".to_vec()
                };
                diff_w_regexec(
                    &format!("row8 nmatch={nm}"),
                    Some(pat),
                    Some(&subj),
                    nm,
                    Some(64),
                );
            }
        }
    }
}

/// Row 9: 0 / 1 / 2 / nested group patterns, matching and non-matching.
#[test]
fn row09_group_arities() {
    let cases: &[(&[u8], &[&[u8]])] = &[
        (br"abc", &[b"abc", b"xxabcxx", b"ab", b"", b"ABC"]),
        (br"^(a)$", &[b"a", b"aa", b"", b"b"]),
        (br"^(a)(b)$", &[b"ab", b"a", b"b", b"abc"]),
        (br"^((a)(b))+$", &[b"ab", b"abab", b"aba", b""]),
        (br"^([0-9]+(\.[0-9]+)*)$", &[b"1", b"1.2", b"1.2.3.4.5", b"1.", b".1"]),
        (br"(a|b)(c|d)", &[b"ac", b"bd", b"ad", b"bc", b"aa", b""]),
        (br"[[:digit:]]+", &[b"abc123", b"abc", b"9"]),
        (br"^$", &[b"", b"a"]),
        (br"x*", &[b"", b"x", b"yyy", b"xxxy"]),
    ];
    for (pat, subjects) in cases {
        for subj in subjects.iter() {
            for nm in [0usize, 1, 2, 3, 6] {
                diff_w_regexec("row9", Some(pat), Some(subj), nm, Some(8));
            }
        }
    }
}

/// Row 10: unanchored patterns matching inside a long (4 KiB) subject.
#[test]
fn row10_long_subjects() {
    let mut rng = Rng::new(0x2010);
    for _ in 0..200 {
        let len = rng.range(2000, 4096);
        let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
        // plant a digit run at a random offset
        let at = rng.below(len - 10);
        let run = rng.range(1, 9);
        for k in 0..run {
            s[at + k] = b'0' + rng.below(10) as u8;
        }
        diff_w_regexec("row10 digits", Some(br"([0-9]+)"), Some(&s), 3, Some(8));
        diff_w_regexec("row10 anchored", Some(br"^([0-9]+)"), Some(&s), 3, Some(8));
        diff_w_regexec(
            "row10 dotted",
            Some(br"([0-9]+)\.([0-9]+)"),
            Some(&s),
            4,
            Some(8),
        );
    }
    // very long pattern too
    let long_pat: Vec<u8> = std::iter::repeat(b'a').take(3000).collect();
    let long_sub: Vec<u8> = std::iter::repeat(b'a').take(4000).collect();
    diff_w_regexec("row10 longpat", Some(&long_pat), Some(&long_sub), 2, Some(4));
}

/// Row 11: a group that does not participate in the match — glibc reports
/// `{-1,-1}` and the C code would compute `rm_eo - rm_so == 0`.
#[test]
fn row11_non_participating_groups() {
    let cases: &[(&[u8], &[&[u8]])] = &[
        (br"^(a)?b", &[b"b", b"ab", b"xb"]),
        (br"^(a)?(b)?c", &[b"c", b"ac", b"bc", b"abc"]),
        (br"^(x)*y", &[b"y", b"xy", b"xxy"]),
        (br"^([0-9]+)?\.*", &[b".", b"1.", b"", b"..."]),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", &[b"1.2.3", b"1.2.3.4"]),
    ];
    for (pat, subjects) in cases {
        for subj in subjects.iter() {
            for nm in [1usize, 2, 3, 4] {
                diff_w_regexec("row11", Some(pat), Some(subj), nm, Some(8));
            }
        }
    }
}

/// Row 12: alternation where *which* group participates is value dependent.
#[test]
fn row12_value_dependent_group_choice() {
    let mut rng = Rng::new(0x2012);
    let pats: &[&[u8]] = &[
        br"^(a+)|(b+)$",
        br"^([0-9]+)|([a-z]+)$",
        br"^([0-9]*)([a-z]*)$",
        br"^([0-9]+)\.|^([a-z]+)-",
    ];
    for pat in pats {
        for _ in 0..1500 {
            let len = rng.range(0, 8);
            let s: Vec<u8> = (0..len)
                .map(|_| *rng.pick(b"ab019z.-".as_slice()))
                .collect();
            for nm in [1usize, 2, 3, 4] {
                diff_w_regexec("row12", Some(pat), Some(&s), nm, Some(8));
            }
        }
    }
}

/// Row 12b: byte-level fuzz of *both* pattern and subject. Patterns that fail
/// to compile are exercised here too (both sides must agree on the failure),
/// which overlaps ERRORS.md rows 4-8.
#[test]
fn row12b_pattern_and_subject_fuzz() {
    const PAT_ALPHA: &[u8] = b"ab019.*+?[]()^$|\\-{},:";
    const SUB_ALPHA: &[u8] = b"ab019. ()[]|:";
    let mut rng = Rng::new(0x2012b);
    for _ in 0..20000 {
        let pl = rng.range(0, 10);
        let pat: Vec<u8> = (0..pl).map(|_| *rng.pick(PAT_ALPHA)).collect();
        let sl = rng.range(0, 12);
        let sub: Vec<u8> = (0..sl).map(|_| *rng.pick(SUB_ALPHA)).collect();
        let nm = *rng.pick(&[0usize, 1, 2, 3, 5]);
        diff_w_regexec("row12b fuzz", Some(&pat), Some(&sub), nm, Some(8));
    }
}
