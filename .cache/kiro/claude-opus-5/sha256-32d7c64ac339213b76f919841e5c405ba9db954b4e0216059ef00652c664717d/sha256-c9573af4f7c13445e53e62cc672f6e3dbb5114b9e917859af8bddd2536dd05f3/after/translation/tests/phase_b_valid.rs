//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH shared objects through their exported symbols and
//! compares results byte-for-byte. All randomness is seeded per row.

mod common;

use common::*;

// ===========================================================================
// get_os_arch — the lowest-level entry point
// ===========================================================================

/// C1 — exactly one ARCHS literal at a random position, all 12 covered.
#[test]
fn c1_arch_single() {
    let mut rng = Rng::new(0xC001);
    for (i, arch) in ARCHS.iter().enumerate() {
        for _ in 0..400 {
            let pre = plain_token(&mut rng, 0, 23);
            let post = plain_token(&mut rng, 0, 23);
            let s = format!("{}{}{}", pre, arch, post);
            diff_arch(s.as_bytes(), &format!("C1 arch#{} {}", i, arch));
        }
    }
}

/// C2 — several literals present: the ARCHS-array order must win, not the
/// left-most position in the string.
#[test]
fn c2_arch_precedence() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..4000 {
        let n = rng.range(2, 5);
        let mut chosen: Vec<&str> = Vec::new();
        for _ in 0..n {
            chosen.push(rng.pick(&ARCHS));
        }
        let sep = [" ", "-", "", "_", " ("];
        let mut s = String::new();
        for (k, a) in chosen.iter().enumerate() {
            if k > 0 {
                s.push_str(rng.pick(&sep));
            }
            s.push_str(a);
        }
        diff_arch(s.as_bytes(), "C2 precedence");
    }
}

/// C3 — literal embedded inside a longer token (plain substring search).
#[test]
fn c3_arch_embedded() {
    let cases: &[&str] = &[
        "xx86_64yy",
        "aarch64le",
        "arm64e",
        "zi386z",
        "ii686",
        "i86pcx",
        "myAIXbox",
        "armv6lz",
        "armv7l",
        "ia64x",
        "amd64v3",
        "sparcv9",
        "linux-x86_64-gnu",
        "aarch64_be",
    ];
    for c in cases {
        diff_arch(c.as_bytes(), &format!("C3 {}", c));
    }
    let mut rng = Rng::new(0xC003);
    for _ in 0..3000 {
        let a = rng.pick(&ARCHS);
        let s = format!(
            "{}{}{}",
            plain_token(&mut rng, 0, 3),
            a,
            plain_token(&mut rng, 0, 3)
        );
        diff_arch(s.as_bytes(), "C3 random embed");
    }
}

/// C4 — noise with no literal, plus near-misses.
#[test]
fn c4_arch_noise() {
    let near: &[&str] = &[
        "x86-64", "x8664", "i38", "386", "ARM64", "aix", "Aix", "sparC", "amd_64", "i86",
        "aarch_64", "armv", "ia_64", "arm 64", "x86", "64", "",
    ];
    for c in near {
        diff_arch(c.as_bytes(), &format!("C4 near {:?}", c));
    }
    let mut rng = Rng::new(0xC004);
    for _ in 0..4000 {
        let n = rng.below(48);
        let mut v: Vec<u8> = Vec::with_capacity(n);
        for _ in 0..n {
            let b = (rng.below(255) + 1) as u8; // never NUL
            v.push(b);
        }
        diff_arch(&v, "C4 byte soup");
    }
}

/// C5 — boundary lengths and positions.
#[test]
fn c5_arch_lengths() {
    diff_arch(b"", "C5 empty");
    for b in [b'x', b'0', b'A', 0xFF] {
        diff_arch(&[b], "C5 one byte");
    }
    let mut rng = Rng::new(0xC005);
    for arch in ARCHS.iter() {
        // at offset 0
        diff_arch(arch.as_bytes(), &format!("C5 exact {}", arch));
        // at the very end of a 4 KiB haystack
        let mut s = plain_token_n(&mut rng, 4096 - arch.len());
        s.push_str(arch);
        diff_arch(s.as_bytes(), &format!("C5 tail4k {}", arch));
        // at the very start of a 4 KiB haystack
        let mut s = String::from(*arch);
        s.push_str(&plain_token_n(&mut rng, 4096 - arch.len()));
        diff_arch(s.as_bytes(), &format!("C5 head4k {}", arch));
    }
    // long haystack with nothing in it
    diff_arch(plain_token_n(&mut rng, 4096).as_bytes(), "C5 4k none");
}

// ===========================================================================
// w_regexec — the other low-level entry point
// ===========================================================================

/// C6 — the three patterns the library itself uses, against version-like
/// subjects, comparing return value and both regmatch_t slots.
#[test]
fn c6_regexec_library_patterns() {
    let mut rng = Rng::new(0xC006);
    for pat in LIB_PATTERNS.iter() {
        for _ in 0..2000 {
            let subject = match rng.below(8) {
                0 => version_n(&mut rng, 1),
                1 => version_n(&mut rng, 2),
                2 => version_n(&mut rng, 3),
                3 => version(&mut rng, 4, 6),
                4 => format!("{}.", version(&mut rng, 1, 3)),
                5 => format!("{}{}", version_n(&mut rng, 2), plain_token_n(&mut rng, 4)),
                6 => plain_token(&mut rng, 0, 11),
                _ => marker_token(&mut rng, 0, 15),
            };
            diff_regexec(
                Some(pat.as_bytes()),
                Some(subject.as_bytes()),
                2,
                2,
                false,
                "C6",
            );
        }
    }
}

/// C7 — nmatch matrix crossed with match / no-match, poisoned pmatch buffer.
#[test]
fn c7_regexec_nmatch_matrix() {
    let pats: &[&str] = &[
        r"^([0-9]+)\.*",
        r"^([0-9]+)\.([0-9]+)\.([0-9]+)$",
        r"(a)(b)(c)(d)(e)(f)(g)",
        r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        r"x",
    ];
    let subs: &[&str] = &["1.2.3", "abcdefg", "10.0.19041.1", "x", "nope", "", "0"];
    for p in pats {
        for s in subs {
            for nmatch in [0usize, 1, 2, 3, 8] {
                let pmatch_null = nmatch == 0;
                diff_regexec(
                    Some(p.as_bytes()),
                    Some(s.as_bytes()),
                    nmatch,
                    8,
                    pmatch_null,
                    &format!("C7 nmatch={}", nmatch),
                );
                // and the same nmatch with a real buffer even when nmatch==0
                diff_regexec(
                    Some(p.as_bytes()),
                    Some(s.as_bytes()),
                    nmatch,
                    8,
                    false,
                    &format!("C7 nmatch={} buf", nmatch),
                );
            }
        }
    }
}

/// C8 — ERE feature matrix against random subjects.
#[test]
fn c8_regexec_ere_features() {
    let pats: &[&str] = &[
        r"a|b",
        r"(a|b)+",
        r"a*",
        r"a?b",
        r"a{2,4}",
        r"[a-z]+",
        r"[^a-z]+",
        r"[[:digit:]]+",
        r"[[:alpha:]][[:alnum:]]*",
        r"^$",
        r"^a.*z$",
        r"((a)(b))",
        r"(a)(b)(c)",
        r"()",
        r"(|a)",
        r"\.",
        r"\(",
        r"[.]",
        r".",
        r"^",
        r"$",
        r"(x)|(y)",
        r"[0-9]+\.[0-9]+",
        r"^([0-9]+)$",
    ];
    let mut rng = Rng::new(0xC008);
    for p in pats {
        for _ in 0..400 {
            let s = match rng.below(4) {
                0 => plain_token(&mut rng, 0, 9),
                1 => marker_token(&mut rng, 0, 13),
                2 => version(&mut rng, 1, 4),
                _ => String::from(*rng.pick(&["", "a", "ab", "abc", "xyz", "0", "1.2", "aab"])),
            };
            diff_regexec(
                Some(p.as_bytes()),
                Some(s.as_bytes()),
                3,
                3,
                false,
                &format!("C8 {}", p),
            );
        }
    }
}

/// C9 — pattern / subject length boundaries.
#[test]
fn c9_regexec_lengths() {
    let mut rng = Rng::new(0xC009);
    // empty and one-char subjects
    for p in [r"^([0-9]+)\.*", r"", r"a", r".", r"^$"] {
        for s in ["", "a", "0", "\u{7f}"] {
            diff_regexec(Some(p.as_bytes()), Some(s.as_bytes()), 2, 2, false, "C9 short");
        }
    }
    // 4 KiB subject
    let long = plain_token_n(&mut rng, 4096);
    for p in [r"^([0-9a-zA-Z]+)", r"([0-9]+)", r"z", r".*"] {
        diff_regexec(Some(p.as_bytes()), Some(long.as_bytes()), 2, 2, false, "C9 4k subj");
    }
    // long pattern (a big alternation) — valid ERE
    let big_pat = (0..256)
        .map(|i| format!("t{}", i))
        .collect::<Vec<_>>()
        .join("|");
    diff_regexec(
        Some(big_pat.as_bytes()),
        Some(b"t200"),
        2,
        2,
        false,
        "C9 long pat",
    );
    diff_regexec(
        Some(big_pat.as_bytes()),
        Some(b"nope"),
        2,
        2,
        false,
        "C9 long pat nomatch",
    );
}

// ===========================================================================
// parse_uname_string — Windows branch (" [Ver: ")
// ===========================================================================

fn win(name: &str, ver: &str, tail: &str) -> String {
    format!("{} [Ver: {}{}", name, ver, tail)
}

/// C10 — Windows branch, major only.
#[test]
fn c10_win_major_only() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..3000 {
        let name = plain_token(&mut rng, 1, 20);
        let v = digits(&mut rng, 1, 6);
        let s = win(&name, &v, "]");
        diff_parse(s.as_bytes(), false, "C10");
        diff_parse(s.as_bytes(), true, "C10 poisoned");
    }
}

/// C11 — Windows branch, major.minor.
#[test]
fn c11_win_major_minor() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..3000 {
        let name = plain_token(&mut rng, 1, 20);
        let s = win(&name, &version_n(&mut rng, 2), "]");
        diff_parse(s.as_bytes(), false, "C11");
        diff_parse(s.as_bytes(), true, "C11 poisoned");
    }
}

/// C12 — Windows branch, major.minor.build.
#[test]
fn c12_win_major_minor_build() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..3000 {
        let name = plain_token(&mut rng, 1, 24);
        let s = win(&name, &version_n(&mut rng, 3), "]");
        diff_parse(s.as_bytes(), false, "C12");
        diff_parse(s.as_bytes(), true, "C12 poisoned");
    }
}

/// C13 — Windows branch, multi-dot build tail.
#[test]
fn c13_win_multidot_build() {
    let mut rng = Rng::new(0xC013);
    for _ in 0..3000 {
        let name = plain_token(&mut rng, 1, 16);
        let parts = rng.range(4, 7);
        let s = win(&name, &version_n(&mut rng, parts), "]");
        diff_parse(s.as_bytes(), false, "C13");
        diff_parse(s.as_bytes(), true, "C13 poisoned");
    }
    for v in [
        "10.0.19041.1", "6.3.9600.0.1", "1.2.3.4.5.6.7", "1.2.3.", "1.2.3..4",
    ] {
        diff_parse(win("Win", v, "]").as_bytes(), false, "C13 fixed");
        diff_parse(win("Win", v, "]").as_bytes(), true, "C13 fixed poisoned");
    }
}

/// C14 — leading zeros, huge digit runs, trailing dots.
#[test]
fn c14_win_odd_numbers() {
    let mut rng = Rng::new(0xC014);
    let fixed: &[&str] = &[
        "0",
        "00",
        "0000006.0000001",
        "0006.0001.",
        "000.000.000",
        "99999999999999999999.1.2",
        "1.99999999999999999999",
        "1.2.99999999999999999999.3",
        "1.",
        "1..",
        "1...2",
        ".1.2",
        "1.2.",
        "4294967296.4294967297.4294967298",
    ];
    for v in fixed {
        diff_parse(win("N", v, "]").as_bytes(), false, "C14 fixed");
        diff_parse(win("N", v, "]").as_bytes(), true, "C14 fixed poisoned");
    }
    for _ in 0..2000 {
        let mut v = String::new();
        let parts = rng.range(1, 5);
        for k in 0..parts {
            if k > 0 {
                for _ in 0..rng.range(1, 3) {
                    v.push('.');
                }
            }
            for _ in 0..rng.range(1, 12) {
                v.push((b'0' + rng.below(10) as u8) as char);
            }
        }
        for _ in 0..rng.below(3) {
            v.push('.');
        }
        diff_parse(win("N", &v, "]").as_bytes(), false, "C14 rnd");
        diff_parse(win("N", &v, "]").as_bytes(), true, "C14 rnd poisoned");
    }
}

/// C15 — Windows branch, version text starting with non-digits.
#[test]
fn c15_win_leading_text() {
    let mut rng = Rng::new(0xC015);
    for v in [
        "abc", "abc 1.2", " 1.2", "-1.2", "v6.1", "x86_64", "?", ".1", "a", "Z9",
    ] {
        diff_parse(win("N", v, "]").as_bytes(), false, "C15 fixed");
        diff_parse(win("N", v, "]").as_bytes(), true, "C15 fixed poisoned");
    }
    for _ in 0..2000 {
        let v = format!(
            "{}{}",
            plain_token(&mut rng, 1, 4)
                .replace(|c: char| c.is_ascii_digit(), "q"),
            version(&mut rng, 1, 3)
        );
        diff_parse(win("N", &v, "]").as_bytes(), false, "C15 rnd");
        diff_parse(win("N", &v, "]").as_bytes(), true, "C15 rnd poisoned");
    }
}

/// C16 — the name part of the Windows form: empty, random, and containing the
/// markers the *other* branch cares about.
#[test]
fn c16_win_name_shapes() {
    let mut rng = Rng::new(0xC016);
    let names: &[&str] = &[
        "",
        " ",
        "|",
        "a|b",
        "a: b",
        "a (b)",
        "Microsoft Windows",
        "x86_64",
        "|windows",
        "a|b|c",
        "]",
        "[",
    ];
    for n in names {
        for v in ["6.1.7601", "10", "abc"] {
            let s = win(n, v, "]");
            diff_parse(s.as_bytes(), false, "C16 fixed");
            diff_parse(s.as_bytes(), true, "C16 fixed poisoned");
        }
    }
    for _ in 0..3000 {
        let n = marker_token(&mut rng, 0, 23);
        let s = win(&n, &version(&mut rng, 1, 4), "]");
        diff_parse(s.as_bytes(), false, "C16 rnd");
        diff_parse(s.as_bytes(), true, "C16 rnd poisoned");
    }
}

/// C17 — several `" [Ver: "` markers; the first must win.
#[test]
fn c17_win_multiple_markers() {
    let mut rng = Rng::new(0xC017);
    for _ in 0..2000 {
        let a = version(&mut rng, 1, 4);
        let b = version(&mut rng, 1, 4);
        let s = format!("N [Ver: {} [Ver: {}]", a, b);
        diff_parse(s.as_bytes(), false, "C17 two");
        diff_parse(s.as_bytes(), true, "C17 two poisoned");
        let s3 = format!("N [Ver: {} [Ver: {} [Ver: 9.9]", a, b);
        diff_parse(s3.as_bytes(), false, "C17 three");
        diff_parse(s3.as_bytes(), true, "C17 three poisoned");
    }
}

/// C18 — `" [Ver: "` shadows both `" ["` and the architecture probe.
#[test]
fn c18_win_shadows_unix() {
    let mut rng = Rng::new(0xC018);
    for arch in ARCHS.iter() {
        for _ in 0..200 {
            let v = version(&mut rng, 1, 4);
            // arch before the marker
            let s = format!("{} host [Ver: {}]", arch, v);
            diff_parse(s.as_bytes(), false, "C18 arch-first");
            diff_parse(s.as_bytes(), true, "C18 arch-first poisoned");
            // a " [" earlier than " [Ver: " -> the Ver test still runs first
            let s = format!("{} [dist: 1.0] [Ver: {}]", arch, v);
            diff_parse(s.as_bytes(), false, "C18 unix-first");
            diff_parse(s.as_bytes(), true, "C18 unix-first poisoned");
            // arch after the marker
            let s = format!("host [Ver: {}] {}", v, arch);
            diff_parse(s.as_bytes(), false, "C18 arch-last");
            diff_parse(s.as_bytes(), true, "C18 arch-last poisoned");
        }
    }
}

/// C19 — version remainder of length 1 and 0 (the strip-one-char boundary).
#[test]
fn c19_win_short_version() {
    let cases: &[&str] = &["N [Ver: ", "N [Ver: ]", "N [Ver: 1", " [Ver: ", "[Ver: ", " [Ver: 9]"];
    for s in cases {
        diff_parse(s.as_bytes(), false, &format!("C19 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C19 poisoned {:?}", s));
    }
    let mut rng = Rng::new(0xC019);
    for _ in 0..2000 {
        let n = marker_token(&mut rng, 0, 7);
        for tail in ["", "]", "x", "1"] {
            let s = format!("{} [Ver: {}", n, tail);
            diff_parse(s.as_bytes(), false, "C19 rnd");
            diff_parse(s.as_bytes(), true, "C19 rnd poisoned");
        }
    }
}

// ===========================================================================
// parse_uname_string — Unix branch (" [")
// ===========================================================================

fn unix_full(prefix: &str, dist: &str, ver: &str, codename: Option<&str>) -> String {
    match codename {
        Some(c) => format!("{} [{}: {} ({})]", prefix, dist, ver, c),
        None => format!("{} [{}: {}]", prefix, dist, ver),
    }
}

/// C20 — the canonical full Unix shape.
#[test]
fn c20_unix_full() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..4000 {
        let prefix = plain_token(&mut rng, 1, 24);
        let dist = plain_token(&mut rng, 1, 16);
        let ver = version(&mut rng, 1, 3);
        let code = plain_token(&mut rng, 1, 12);
        let s = unix_full(&prefix, &dist, &ver, Some(&code));
        diff_parse(s.as_bytes(), false, "C20");
        diff_parse(s.as_bytes(), true, "C20 poisoned");
    }
}

/// C21 — `": "` present, no `" ("`.
#[test]
fn c21_unix_no_codename() {
    let mut rng = Rng::new(0xC021);
    for _ in 0..4000 {
        let prefix = plain_token(&mut rng, 1, 20);
        let dist = plain_token(&mut rng, 1, 14);
        let ver = version(&mut rng, 1, 4);
        let s = unix_full(&prefix, &dist, &ver, None);
        diff_parse(s.as_bytes(), false, "C21");
        diff_parse(s.as_bytes(), true, "C21 poisoned");
    }
}

/// C22 — no `": "` inside the bracketed part.
#[test]
fn c22_unix_no_colon() {
    let mut rng = Rng::new(0xC022);
    for _ in 0..4000 {
        let prefix = plain_token(&mut rng, 1, 20);
        let inner = plain_token(&mut rng, 0, 20);
        let s = format!("{} [{}]", prefix, inner);
        diff_parse(s.as_bytes(), false, "C22");
        diff_parse(s.as_bytes(), true, "C22 poisoned");
    }
    for s in ["N [", "N []", "N [x]", "N [:]", "N [: ]", "N [a:b]", " [", " []"] {
        diff_parse(s.as_bytes(), false, &format!("C22 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C22 poisoned {:?}", s));
    }
}

/// C23 — `"|"` before `": "` sets `os_platform`.
#[test]
fn c23_unix_pipe_platform() {
    let mut rng = Rng::new(0xC023);
    for _ in 0..4000 {
        let prefix = plain_token(&mut rng, 1, 16);
        let name = plain_token(&mut rng, 0, 12);
        let plat = plain_token(&mut rng, 0, 12);
        let ver = version(&mut rng, 1, 3);
        let code = if rng.boolean() {
            Some(plain_token(&mut rng, 1, 8))
        } else {
            None
        };
        let dist = format!("{}|{}", name, plat);
        let s = unix_full(&prefix, &dist, &ver, code.as_deref());
        diff_parse(s.as_bytes(), false, "C23");
        diff_parse(s.as_bytes(), true, "C23 poisoned");
    }
}

/// C24 — `"|"` present and no `": "`.
#[test]
fn c24_unix_pipe_no_colon() {
    let mut rng = Rng::new(0xC024);
    for _ in 0..3000 {
        let prefix = plain_token(&mut rng, 1, 16);
        let a = plain_token(&mut rng, 0, 12);
        let b = plain_token(&mut rng, 0, 12);
        let s = format!("{} [{}|{}]", prefix, a, b);
        diff_parse(s.as_bytes(), false, "C24");
        diff_parse(s.as_bytes(), true, "C24 poisoned");
    }
    for s in ["N [|]", "N [|", "N [a|", "N [|b]", "N [a|b", "N [||]"] {
        diff_parse(s.as_bytes(), false, &format!("C24 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C24 poisoned {:?}", s));
    }
}

/// C25 — multiple pipes, pipe at position 0.
#[test]
fn c25_unix_pipe_shapes() {
    let mut rng = Rng::new(0xC025);
    let fixed: &[&str] = &[
        "N [|a: 1.0]",
        "N [a|b|c: 1.0]",
        "N [|: 1.0]",
        "N [||: 1.0]",
        "N [a||b: 1.0]",
        "N [|a|: 1.0 (x)]",
        "N [a: 1.0|b]",
        "N [a: 1.0 (x|y)]",
    ];
    for s in fixed {
        diff_parse(s.as_bytes(), false, &format!("C25 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C25 poisoned {:?}", s));
    }
    for _ in 0..3000 {
        let n = rng.range(1, 4);
        let parts: Vec<String> = (0..=n)
            .map(|_| plain_token(&mut rng, 0, 8))
            .collect();
        let dist = parts.join("|");
        let s = unix_full("H", &dist, &version(&mut rng, 1, 3), None);
        diff_parse(s.as_bytes(), false, "C25 rnd");
        diff_parse(s.as_bytes(), true, "C25 rnd poisoned");
    }
}

/// C26 — Unix version shapes (no build regex in this branch).
#[test]
fn c26_unix_version_shapes() {
    let mut rng = Rng::new(0xC026);
    let fixed: &[&str] = &[
        "20", "20.04", "20.04.1", "8", "8.5.2111", "0", "0.0", "1.", "1..2", ".5", "abc",
        "20.04 LTS", "rolling", "", "9-stable", "12.4-RELEASE", "000020.000004",
        "99999999999999999999.1",
    ];
    for v in fixed {
        for code in [None, Some("focal")] {
            let s = unix_full("H", "Ubuntu", v, code);
            diff_parse(s.as_bytes(), false, "C26 fixed");
            diff_parse(s.as_bytes(), true, "C26 fixed poisoned");
        }
    }
    for _ in 0..3000 {
        let v = match rng.below(6) {
            0 => version_n(&mut rng, 1),
            1 => version_n(&mut rng, 2),
            2 => version_n(&mut rng, 3),
            3 => version(&mut rng, 4, 6),
            4 => plain_token(&mut rng, 0, 9),
            _ => marker_token(&mut rng, 0, 11),
        };
        let s = unix_full("H", "D", &v, None);
        diff_parse(s.as_bytes(), false, "C26 rnd");
        diff_parse(s.as_bytes(), true, "C26 rnd poisoned");
    }
}

/// C27 — codename shapes: repeated `" ("`, markers inside the codename.
#[test]
fn c27_unix_codename_shapes() {
    let mut rng = Rng::new(0xC027);
    let fixed: &[&str] = &[
        "H [D: 1.0 (a)]",
        "H [D: 1.0 (a) (b)]",
        "H [D: 1.0 ()]",
        "H [D: 1.0 (]",
        "H [D: 1.0 (",
        "H [D: 1.0 (a: b)]",
        "H [D: 1.0 (a|b)]",
        "H [D: 1.0 ((a))]",
        "H [D: 1.0 (a (b))]",
        "H [D:  (a)]",
        "H [D: (a)]",
        "H [D: 1.0 (a",
        "H [D: 1.0 (x86_64)]",
    ];
    for s in fixed {
        diff_parse(s.as_bytes(), false, &format!("C27 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C27 poisoned {:?}", s));
    }
    for _ in 0..3000 {
        let n = rng.range(1, 3);
        let mut v = version(&mut rng, 1, 3);
        for _ in 0..n {
            v.push_str(&format!(" ({})", marker_token(&mut rng, 0, 9)));
        }
        let s = format!("H [D: {}]", v);
        diff_parse(s.as_bytes(), false, "C27 rnd");
        diff_parse(s.as_bytes(), true, "C27 rnd poisoned");
    }
}

/// C28 — several `": "` separators; the first must win.
#[test]
fn c28_unix_multiple_colons() {
    let mut rng = Rng::new(0xC028);
    let fixed: &[&str] = &[
        "H [A: B: 1.0]",
        "H [A: B: C: 2.1 (x)]",
        "H [: 1.0]",
        "H [A: : 1.0]",
        "H [A:: 1.0]",
        "H [A: 1.0: 2.0]",
    ];
    for s in fixed {
        diff_parse(s.as_bytes(), false, &format!("C28 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C28 poisoned {:?}", s));
    }
    for _ in 0..2500 {
        let k = rng.range(2, 4);
        let parts: Vec<String> = (0..k)
            .map(|_| plain_token(&mut rng, 0, 8))
            .collect();
        let s = format!(
            "H [{}: {}]",
            parts.join(": "),
            version(&mut rng, 1, 3)
        );
        diff_parse(s.as_bytes(), false, "C28 rnd");
        diff_parse(s.as_bytes(), true, "C28 rnd poisoned");
    }
}

/// C29 — several `" ["` markers; the first must win.
#[test]
fn c29_unix_multiple_brackets() {
    let mut rng = Rng::new(0xC029);
    let fixed: &[&str] = &[
        "H [A: 1.0] [B: 2.0]",
        "H [ [A: 1.0]",
        "H [A [B: 1.0]",
        "[H [A: 1.0]",
        "H [[A: 1.0]]",
    ];
    for s in fixed {
        diff_parse(s.as_bytes(), false, &format!("C29 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C29 poisoned {:?}", s));
    }
    for _ in 0..2500 {
        let a = unix_full("H", &plain_token_n(&mut rng, 4), &version_n(&mut rng, 2), None);
        let b = unix_full("", &plain_token_n(&mut rng, 4), &version_n(&mut rng, 2), None);
        let s = format!("{}{}", a, b);
        diff_parse(s.as_bytes(), false, "C29 rnd");
        diff_parse(s.as_bytes(), true, "C29 rnd poisoned");
    }
}

/// C30 — each ARCHS literal placed in the prefix, so `get_os_arch` sees it.
#[test]
fn c30_unix_arch_each() {
    let mut rng = Rng::new(0xC030);
    for arch in ARCHS.iter() {
        for _ in 0..250 {
            let prefix = format!(
                "{}{}{}",
                plain_token(&mut rng, 0, 7),
                arch,
                plain_token(&mut rng, 0, 7)
            );
            let s = unix_full(
                &prefix,
                &plain_token(&mut rng, 1, 8),
                &version(&mut rng, 1, 3),
                if rng.boolean() {
                    Some("code")
                } else {
                    None
                },
            );
            diff_parse(s.as_bytes(), false, "C30");
            diff_parse(s.as_bytes(), true, "C30 poisoned");
        }
    }
}

/// C31 — several archs in the prefix: ARCHS-array precedence through the
/// composed pipeline.
#[test]
fn c31_unix_arch_precedence() {
    let mut rng = Rng::new(0xC031);
    for _ in 0..4000 {
        let n = rng.range(2, 4);
        let mut prefix = String::new();
        for k in 0..n {
            if k > 0 {
                prefix.push_str(rng.pick(&["-", "_", "", "."]));
            }
            prefix.push_str(rng.pick(&ARCHS));
        }
        let s = unix_full(&prefix, "D", &version_n(&mut rng, 2), None);
        diff_parse(s.as_bytes(), false, "C31");
        diff_parse(s.as_bytes(), true, "C31 poisoned");
    }
}

/// C32 — arch literal straddling the `" ["` truncation point.
#[test]
fn c32_unix_arch_straddle() {
    let mut rng = Rng::new(0xC032);
    for arch in ARCHS.iter() {
        for cut in 0..arch.len() {
            let (head, tail) = arch.split_at(cut);
            let s = format!("H{} [{}: 1.0]", head, tail);
            diff_parse(s.as_bytes(), false, "C32 split");
            diff_parse(s.as_bytes(), true, "C32 split poisoned");
        }
        // entirely inside the bracket -> invisible to get_os_arch
        let s = format!("H [{}: 1.0]", arch);
        diff_parse(s.as_bytes(), false, "C32 inside");
        diff_parse(s.as_bytes(), true, "C32 inside poisoned");
    }
    for _ in 0..2000 {
        let a = rng.pick(&ARCHS);
        let cut = rng.below(a.len() + 1);
        let (h, t) = a.split_at(cut);
        let s = format!("{} [{}{}: {}]", h, t, plain_token_n(&mut rng, 3), version_n(&mut rng, 2));
        diff_parse(s.as_bytes(), false, "C32 rnd");
        diff_parse(s.as_bytes(), true, "C32 rnd poisoned");
    }
}

// ===========================================================================
// parse_uname_string — arch-only branch (no " [" at all)
// ===========================================================================

/// C33 — no bracket marker: only `os_arch` may be written.
#[test]
fn c33_archonly_each() {
    let mut rng = Rng::new(0xC033);
    for arch in ARCHS.iter() {
        for _ in 0..250 {
            let s = format!(
                "{}{}{}",
                plain_token(&mut rng, 0, 15),
                arch,
                plain_token(&mut rng, 0, 15)
            );
            diff_parse(s.as_bytes(), false, "C33");
            diff_parse(s.as_bytes(), true, "C33 poisoned");
        }
    }
    // classic uname -a style strings with no bracket
    let real: &[&str] = &[
        "Linux host 5.15.0-88-generic #98-Ubuntu SMP x86_64 GNU/Linux",
        "Darwin mac 22.6.0 Darwin Kernel Version 22.6.0 arm64",
        "SunOS sol 5.11 11.4 i86pc i386",
        "AIX box 3 7 00F8C1234C00",
        "FreeBSD bsd 13.2-RELEASE amd64",
        "Linux pi 6.1.0 armv7l GNU/Linux",
    ];
    for s in real {
        diff_parse(s.as_bytes(), false, "C33 real");
        diff_parse(s.as_bytes(), true, "C33 real poisoned");
    }
}

/// C34 — near-misses for the two bracket markers.
#[test]
fn c34_archonly_near_misses() {
    let cases: &[&str] = &[
        "",
        "[",
        "[x]",
        "x[y]",
        "x[y: 1.0]",
        "[Ver: 1.0]",
        " [Ver:1.0]",
        " [Ver:",
        " [Ver ",
        "N [ver: 1.0]",
        "N [VER: 1.0]",
        "N  [Ver: 1.0]",
        "N\t[Ver: 1.0]",
        "N [Ver:  1.0]",
        " ",
        "  ",
        "x ",
        " x",
        "x86_64",
        "x86_64[",
        "]",
        "()",
        ": ",
        "|",
    ];
    for s in cases {
        diff_parse(s.as_bytes(), false, &format!("C34 {:?}", s));
        diff_parse(s.as_bytes(), true, &format!("C34 poisoned {:?}", s));
    }
}

// ===========================================================================
// Cross-cutting rows
// ===========================================================================

/// C35 — every branch run against a pre-poisoned struct so the exact set of
/// untouched fields is compared, not just the NULL/non-NULL pattern.
#[test]
fn c35_poisoned_struct() {
    let reps: &[&str] = &[
        "Microsoft Windows [Ver: 10.0.19041.1]",
        "Microsoft Windows [Ver: 10]",
        "Microsoft Windows [Ver: abc]",
        "Linux host [Ubuntu: 20.04 (focal)]",
        "Linux host [Ubuntu: 20.04]",
        "Linux host [Ubuntu]",
        "Linux|linux host [Ubuntu: 20.04 (focal)]",
        "Linux x86_64 host [Ubuntu|debian: 20.04 (focal)]",
        "Linux x86_64 host",
        "plain string",
        "",
        "N [",
        "N [Ver: ",
    ];
    for s in reps {
        diff_parse(s.as_bytes(), true, &format!("C35 {:?}", s));
        diff_parse(s.as_bytes(), false, &format!("C35 zeroed {:?}", s));
    }
}

/// C36 — 20 000 seeded mixed-alphabet cases, any branch.
#[test]
fn c36_fuzz_mixed() {
    let mut rng = Rng::new(0xC036);
    let frags: &[&str] = &[
        " [", " [Ver: ", ": ", " (", ")", "]", "|", ".", " ", "0", "1", "9", "10", "04",
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
        "aarch64", "arm64", "Linux", "Windows", "Ubuntu", "focal", "-", "_", "[", "(", ":",
    ];
    for i in 0..20_000 {
        let n = rng.range(0, 12);
        let mut s = String::new();
        for _ in 0..n {
            if rng.below(4) == 0 {
                s.push_str(&plain_token(&mut rng, 1, 5));
            } else {
                s.push_str(rng.pick(frags));
            }
        }
        let poison = i % 2 == 0;
        diff_parse(s.as_bytes(), poison, "C36 fuzz");
    }
}

/// C37 — real-world uname corpus with randomized numbers.
#[test]
fn c37_realworld_corpus() {
    let mut rng = Rng::new(0xC037);
    for _ in 0..4000 {
        let maj = digits(&mut rng, 1, 3);
        let min = digits(&mut rng, 1, 3);
        let bld = digits(&mut rng, 1, 6);
        let rev = digits(&mut rng, 1, 4);
        let templates: Vec<String> = vec![
            format!("Microsoft Windows [Ver: {}.{}.{}]", maj, min, bld),
            format!("Microsoft Windows [Ver: {}.{}.{}.{}]", maj, min, bld, rev),
            format!("Microsoft Windows Server 2019 [Ver: {}.{}.{}]", maj, min, bld),
            format!(
                "Linux |ubuntu [Ubuntu: {}.{} (focal fossa)]",
                maj, min
            ),
            format!("Linux x86_64 [Ubuntu: {}.{} (jammy)]", maj, min),
            format!("Linux i686 [CentOS Linux: {} (Core)]", maj),
            format!("Linux aarch64 [Debian GNU/Linux: {} (bookworm)]", maj),
            format!("Darwin arm64 [Mac OS X: {}.{}.{} (Ventura)]", maj, min, bld),
            format!("SunOS i86pc [SunOS: {}.{}]", maj, min),
            format!("AIX [AIX: {}.{}]", maj, min),
            format!("HP-UX ia64 [HP-UX: B.{}.{}]", maj, min),
            format!("Linux armv7l [Raspbian GNU/Linux: {}]", maj),
            format!("Linux sparc [Solaris]"),
            format!("FreeBSD amd64 [FreeBSD: {}.{}-RELEASE]", maj, min),
            format!("Linux x86_64 [Amazon Linux|amzn: {}]", maj),
        ];
        let s = rng.pick(&templates).clone();
        diff_parse(s.as_bytes(), false, "C37");
        diff_parse(s.as_bytes(), true, "C37 poisoned");
    }
}

/// C38 — pmatch reuse across the composed pipeline: inputs where an early
/// regex matches with a long capture and a later one fails, leaving stale
/// offsets in the shared `regmatch_t match[2]`.
#[test]
fn c38_pmatch_reuse_pipeline() {
    let mut rng = Rng::new(0xC038);
    // Windows branch: 3 regexes share match[]; craft versions where 1, 2 or 3
    // of them succeed, with widely differing capture lengths.
    let vers: &[&str] = &[
        "123456789",
        "123456789.1",
        "1.123456789",
        "123456789.1.2",
        "1.2.123456789",
        "1.2.3.123456789",
        "123456789.123456789.123456789.123456789",
        "9.9",
        "9",
        "9.9.9",
        "1.2.x",
        "1.x",
        "x",
    ];
    for v in vers {
        diff_parse(win("N", v, "]").as_bytes(), false, "C38 win");
        diff_parse(win("N", v, "]").as_bytes(), true, "C38 win poisoned");
        // Unix branch: 2 regexes share match[]
        let s = unix_full("H", "D", v, None);
        diff_parse(s.as_bytes(), false, "C38 unix");
        diff_parse(s.as_bytes(), true, "C38 unix poisoned");
        let s = unix_full("H", "D", v, Some("cn"));
        diff_parse(s.as_bytes(), false, "C38 unix cn");
        diff_parse(s.as_bytes(), true, "C38 unix cn poisoned");
    }
    for _ in 0..3000 {
        // decreasing-length captures maximise the chance of observing staleness
        let a = digits(&mut rng, 1, 12);
        let b = digits(&mut rng, 1, 12);
        let c = digits(&mut rng, 1, 12);
        let v = match rng.below(4) {
            0 => a.clone(),
            1 => format!("{}.{}", a, b),
            2 => format!("{}.{}.{}", a, b, c),
            _ => format!("{}.{}.{}.{}", a, b, c, a),
        };
        diff_parse(win("N", &v, "]").as_bytes(), true, "C38 rnd win");
        diff_parse(unix_full("H", "D", &v, None).as_bytes(), true, "C38 rnd unix");
    }
}

/// C39 — long inputs (4 KiB and 64 KiB) in each branch.
#[test]
fn c39_long_inputs() {
    let mut rng = Rng::new(0xC039);
    for len in [4096usize, 65536] {
        let pad = plain_token_n(&mut rng, len);
        // Windows: long name, long version tail
        diff_parse(win(&pad, "10.0.19041.1", "]").as_bytes(), true, "C39 win name");
        let long_digits: String = (0..len).map(|_| '7').collect();
        diff_parse(
            win("N", &format!("1.2.{}", long_digits), "]").as_bytes(),
            true,
            "C39 win build",
        );
        diff_parse(win("N", &long_digits, "]").as_bytes(), true, "C39 win major");
        // Unix: long prefix / dist / version / codename
        diff_parse(
            unix_full(&pad, "D", "20.04", Some("focal")).as_bytes(),
            true,
            "C39 unix prefix",
        );
        diff_parse(
            unix_full("H", &pad, "20.04", Some("focal")).as_bytes(),
            true,
            "C39 unix dist",
        );
        diff_parse(
            unix_full("H", "D", &long_digits, Some("focal")).as_bytes(),
            true,
            "C39 unix ver",
        );
        diff_parse(
            unix_full("H", "D", "20.04", Some(&pad)).as_bytes(),
            true,
            "C39 unix code",
        );
        // arch-only, arch at the far end
        diff_parse(format!("{}x86_64", pad).as_bytes(), true, "C39 archonly");
        diff_parse(pad.as_bytes(), true, "C39 archonly none");
    }
}

/// C40 — high-bit / non-UTF-8 bytes everywhere.
#[test]
fn c40_non_utf8_bytes() {
    let mut rng = Rng::new(0xC040);
    for _ in 0..4000 {
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&hi_bytes(&mut rng, 1, 8));
        match rng.below(3) {
            0 => {
                v.extend_from_slice(b" [Ver: ");
                v.extend_from_slice(version(&mut rng, 1, 4).as_bytes());
                v.extend_from_slice(&hi_bytes(&mut rng, 0, 3));
                v.push(b']');
            }
            1 => {
                v.extend_from_slice(b" [");
                v.extend_from_slice(&hi_bytes(&mut rng, 1, 6));
                if rng.boolean() {
                    v.push(b'|');
                    v.extend_from_slice(&hi_bytes(&mut rng, 1, 6));
                }
                v.extend_from_slice(b": ");
                v.extend_from_slice(version(&mut rng, 1, 3).as_bytes());
                if rng.boolean() {
                    v.extend_from_slice(b" (");
                    v.extend_from_slice(&hi_bytes(&mut rng, 1, 6));
                    v.push(b')');
                }
                v.push(b']');
            }
            _ => {
                v.extend_from_slice(ARCHS[rng.below(12)].as_bytes());
                v.extend_from_slice(&hi_bytes(&mut rng, 0, 7));
            }
        }
        diff_parse(&v, rng.boolean(), "C40");
    }
    // also feed high bytes to the low-level entry points
    for _ in 0..2000 {
        let h = hi_bytes(&mut rng, 1, 24);
        diff_arch(&h, "C40 arch");
        diff_regexec(Some(br"^([0-9]+)\.*"), Some(&h), 2, 2, false, "C40 regexec");
        diff_regexec(Some(br"(.+)"), Some(&h), 2, 2, false, "C40 regexec dot");
    }
}
