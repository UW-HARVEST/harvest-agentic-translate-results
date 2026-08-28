//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row is driven with many randomized
//! inputs from a fixed-seed PRNG. Both implementations are invoked only through
//! their `.so` exports.

mod common;

use common::*;

// ===========================================================================
// Input builders (derived from the separators the C actually looks for)
// ===========================================================================

/// Filler that contains none of the parser's separator sequences.
fn filler(rng: &Rng, len: usize) -> Vec<u8> {
    rng.bytes_from(SAFE_ALPHA, len)
}

fn s(v: &str) -> Vec<u8> {
    v.as_bytes().to_vec()
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// `"<prefix> [Ver: <payload>]"` — the Windows branch (`lib.c:68`).
fn win(prefix: &[u8], payload: &[u8]) -> Vec<u8> {
    cat(&[prefix, b" [Ver: ", payload, b"]"])
}

/// `"<prefix> [<name>: <version>]"` — the POSIX branch with a `": "`.
fn posix(prefix: &[u8], name: &[u8], version: &[u8]) -> Vec<u8> {
    cat(&[prefix, b" [", name, b": ", version, b"]"])
}

/// `"<prefix> [<name>]"` — the POSIX branch without a `": "` (`lib.c:130`).
fn posix_nocolon(prefix: &[u8], name: &[u8]) -> Vec<u8> {
    cat(&[prefix, b" [", name, b"]"])
}

/// A dotted version with `n` numeric components.
fn dotted(rng: &Rng, n: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for i in 0..n {
        if i > 0 {
            out.push(b'.');
        }
        out.extend_from_slice(&rng.number());
    }
    out
}

// ===========================================================================
// C1..C5 — get_os_arch
// ===========================================================================

#[test]
fn c1_arch_each_alone() {
    for a in ARCHS {
        diff_arch(a.as_bytes(), "C1/exact");
    }
    // and every arch as the entire string with surrounding whitespace variants
    for a in ARCHS {
        for wrap in [" {}", "{} ", " {} ", "\t{}\n", "({})", "[{}]"] {
            diff_arch(wrap.replace("{}", a).as_bytes(), "C1/wrapped");
        }
    }
}

#[test]
fn c2_arch_embedded_positions() {
    let rng = Rng::new(SEED ^ 2);
    for a in ARCHS {
        for _ in 0..250 {
            let pre = filler(&rng, rng.below(24));
            let post = filler(&rng, rng.below(24));
            diff_arch(&cat(&[&pre, a.as_bytes(), &post]), "C2/embedded");
            diff_arch(&cat(&[a.as_bytes(), &post]), "C2/at-start");
            diff_arch(&cat(&[&pre, a.as_bytes()]), "C2/at-end");
        }
    }
}

#[test]
fn c3_arch_precedence() {
    let rng = Rng::new(SEED ^ 3);
    // ARCHS-array order wins, not position in the string.
    for _ in 0..3000 {
        let n = rng.range(2, 4);
        let mut parts: Vec<Vec<u8>> = Vec::new();
        for _ in 0..n {
            parts.push(s(rng.arch()));
            parts.push(filler(&rng, rng.below(6)));
        }
        let joined: Vec<u8> = parts.concat();
        diff_arch(&joined, "C3/precedence");
    }
    // Hand-picked orderings that specifically invert array order vs string order
    for (a, b) in [
        ("aarch64", "x86_64"),
        ("arm64", "i386"),
        ("armv7", "sparc"),
        ("ia64", "amd64"),
        ("AIX", "i86pc"),
        ("i686", "i386"),
        ("arm64", "aarch64"),
        ("armv7", "armv6"),
    ] {
        diff_arch(format!("{a} {b}").as_bytes(), "C3/pair-fwd");
        diff_arch(format!("{b} {a}").as_bytes(), "C3/pair-rev");
        diff_arch(format!("{a}{b}").as_bytes(), "C3/pair-glued");
    }
}

#[test]
fn c4_arch_near_misses() {
    let near = [
        "x86", "86_64", "x86-64", "X86_64", "I386", "i38", "386", "sparc64", "SPARC", "amd6",
        "amd", "arm", "armv8", "armv", "aarch6", "arch64", "arm6", "arm_64", "ia6", "ia32", "aix",
        "Aix", "i86p", "86pc", "i86", "x8664", "i-386", "amd64x", "prefixx86_64suffix",
    ];
    for n in near {
        diff_arch(n.as_bytes(), "C4/near-miss");
    }
    // sanity: some "near misses" DO contain a real token (sparc64 ⊃ sparc,
    // amd64x ⊃ amd64, prefixx86_64suffix ⊃ x86_64) — the C finds them, so must Rust.
    for n in ["sparc64", "amd64x", "prefixx86_64suffix", "xxi386xx", "aarch64le"] {
        diff_arch(n.as_bytes(), "C4/substring-hit");
    }
}

#[test]
fn c5_arch_random_fuzz() {
    let rng = Rng::new(SEED ^ 5);
    // Alphabet biased toward arch-token characters so partial tokens appear.
    let alpha: &[u8] = b"xX86_64iI3rmvspachdAIXa1720nlq -.";
    for _ in 0..3000 {
        let len = rng.below(129);
        diff_arch(&rng.bytes_from(alpha, len), "C5/fuzz-biased");
    }
    for _ in 0..3000 {
        let len = rng.below(129);
        diff_arch(&rng.bytes_from(HOSTILE_ALPHA, len), "C5/fuzz-hostile");
    }
}

// ===========================================================================
// C6..C11 — w_regexec
// ===========================================================================

#[test]
fn c6_regexec_parser_patterns() {
    let rng = Rng::new(SEED ^ 6);
    for p in PARSER_PATTERNS {
        for _ in 0..1200 {
            let subject = match rng.below(9) {
                0 => Vec::new(),
                1 => dotted(&rng, 1),
                2 => dotted(&rng, 2),
                3 => dotted(&rng, 3),
                4 => dotted(&rng, rng.range(4, 7)),
                5 => cat(&[&dotted(&rng, rng.range(1, 3)), b"."]),
                6 => cat(&[&dotted(&rng, rng.range(1, 3)), b"...."]),
                7 => cat(&[&filler(&rng, rng.range(1, 5)), &dotted(&rng, 3)]),
                _ => cat(&[&dotted(&rng, rng.range(1, 3)), &filler(&rng, rng.range(1, 6))]),
            };
            diff_regexec_n(p.as_bytes(), &subject, 2, "C6/parser-patterns");
            // also with nmatch != 2 to check slot filling
            diff_regexec(Some(p.as_bytes()), Some(&subject), 1, 4, "C6/nmatch1");
            diff_regexec(Some(p.as_bytes()), Some(&subject), 4, 4, "C6/nmatch4");
        }
    }
}

#[test]
fn c7_regexec_no_groups_nmatch_sweep() {
    let rng = Rng::new(SEED ^ 7);
    let pats: [&str; 8] = [
        "^abc", "abc", "^[0-9]+$", "[0-9]+", "^.*$", "x", "^$", "[[:alpha:]]+",
    ];
    let subjects: [&str; 10] = [
        "", "a", "abc", "xabcx", "123", "abc123", "   ", "0", "zzzzzzzz", "abc\tdef",
    ];
    for p in pats {
        for sub in subjects {
            for n in [0usize, 1, 2, 3, 8, 64] {
                diff_regexec(Some(p.as_bytes()), Some(sub.as_bytes()), n, 64, "C7/sweep");
            }
        }
    }
    for _ in 0..1500 {
        let p = *rng.pick(&pats);
        let sub = rng.bytes_from(b"abc0123 xz\t", rng.below(24));
        let n = *rng.pick(&[0usize, 1, 2, 3, 8, 64]);
        diff_regexec(Some(p.as_bytes()), Some(&sub), n, 64, "C7/random");
    }
}

#[test]
fn c8_regexec_groups_nmatch_sweep() {
    let rng = Rng::new(SEED ^ 8);
    let pats: [&str; 10] = [
        r"^([0-9]+)$",
        r"^([0-9]+)\.([0-9]+)$",
        r"^([0-9]+)\.([0-9]+)\.([0-9]+)$",
        r"^(([0-9]+)\.([0-9]+))",
        r"^((a)(b)(c))+",
        r"^(a)?b",
        r"(x)|(y)",
        r"^([a-z]*)([0-9]*)$",
        r"^(([0-9]+)(\.[0-9]+)*)",
        r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
    ];
    let subjects: [&str; 14] = [
        "", "1", "1.2", "1.2.3", "1.2.3.4.5.6", "abc", "abcabc", "b", "ab", "x", "y", "xy",
        "abc123", "0.0.0.0",
    ];
    for p in pats {
        for sub in subjects {
            for n in [1usize, 2, 3, 8, 64] {
                diff_regexec(Some(p.as_bytes()), Some(sub.as_bytes()), n, 64, "C8/sweep");
            }
        }
    }
    for _ in 0..2000 {
        let p = *rng.pick(&pats);
        let sub = if rng.bool() {
            dotted(&rng, rng.range(1, 6))
        } else {
            rng.bytes_from(b"abcxy0123.", rng.below(20))
        };
        let n = *rng.pick(&[1usize, 2, 3, 5, 8, 64]);
        diff_regexec(Some(p.as_bytes()), Some(&sub), n, 64, "C8/random");
    }
}

#[test]
fn c9_regexec_unanchored_offsets() {
    let rng = Rng::new(SEED ^ 9);
    let pats: [&str; 6] = [
        r"([0-9]+)",
        r"([0-9]+)\.([0-9]+)",
        r"([a-z]+)([0-9]+)",
        r"(Ver: )([0-9.]+)",
        r"\[([^]]*)\]",
        r"( \()([a-z ]+)(\))",
    ];
    for p in pats {
        for _ in 0..1500 {
            let pre = rng.bytes_from(b"abc XYZ[]().:|", rng.below(20));
            let mid = dotted(&rng, rng.range(1, 4));
            let post = rng.bytes_from(b"abc XYZ[]().:|", rng.below(20));
            let subject = cat(&[&pre, &mid, &post]);
            diff_regexec(Some(p.as_bytes()), Some(&subject), 4, 8, "C9/offsets");
        }
    }
}

#[test]
fn c10_regexec_ere_features() {
    let rng = Rng::new(SEED ^ 10);
    let pats: [&str; 22] = [
        r"a|b",
        r"^(a|bb|ccc)$",
        r"a{2,4}",
        r"^[0-9]{1,3}$",
        r"[[:digit:]]+",
        r"[[:space:]]",
        r"[[:upper:]][[:lower:]]*",
        r"[^0-9]+",
        r"^.$",
        r".*",
        r"x$",
        r"^x",
        r"\.",
        r"\[",
        r"\\",
        r"(a+)+b",
        r"[]a]",
        r"[a-]",
        r"()",
        r"^()$",
        r"[0-9]?[0-9]?[0-9]",
        r"^([0-9]+)(\.([0-9]+))?(\.([0-9]+))?$",
    ];
    let mut subjects: Vec<Vec<u8>> = vec![
        s(""),
        s("a"),
        s("b"),
        s("bb"),
        s("ccc"),
        s("aa"),
        s("aaaa"),
        s("aaaaaa"),
        s("aaaaab"),
        s("1"),
        s("12"),
        s("123"),
        s("1234"),
        s("Abc"),
        s(" "),
        s("."),
        s("["),
        s("\\"),
        s("]"),
        s("-"),
        s("x"),
        s("zzx"),
        s("1.2"),
        s("1.2.3"),
    ];
    // long (>= 512 B) subjects
    subjects.push(vec![b'a'; 512]);
    subjects.push(vec![b'a'; 1024]);
    subjects.push({
        let mut v = vec![b'z'; 600];
        v.extend_from_slice(b"123");
        v
    });
    subjects.push(rng.bytes_from(b"ab019 .", 700));

    for p in pats {
        for sub in &subjects {
            for n in [0usize, 1, 2, 6] {
                diff_regexec(Some(p.as_bytes()), Some(sub), n, 8, "C10/ere");
            }
        }
    }
}

#[test]
fn c11_regexec_random_pattern_fuzz() {
    let rng = Rng::new(SEED ^ 11);
    // Mixed valid/invalid patterns — regcomp failures go down the lib.c:40 path,
    // which is also an ERRORS.md row, but it must not diverge here either.
    let pat_alpha: &[u8] = b"ab019.*+?()[]{}|^$\\-:, ";
    with_stderr_silenced(|| {
        for _ in 0..4000 {
            let p = rng.bytes_from(pat_alpha, rng.below(13));
            let sub = rng.bytes_from(b"ab019. []()|", rng.below(20));
            diff_regexec(Some(&p), Some(&sub), 4, 8, "C11/pattern-fuzz");
        }
    });
}

// ===========================================================================
// C12..C19 — parse_uname_string, Windows (" [Ver: ") path
// ===========================================================================

#[test]
fn c12_ver_major_only() {
    let rng = Rng::new(SEED ^ 12);
    for _ in 0..2000 {
        let prefix = filler(&rng, rng.below(24));
        let payload = rng.number();
        diff_parse(&win(&prefix, &payload), "C12/major-only");
    }
    for lit in ["0", "1", "10", "6", "0000", "007", "4294967296", "99999999999999999999"] {
        diff_parse(&win(b"Microsoft Windows 10", lit.as_bytes()), "C12/literal");
    }
}

#[test]
fn c13_ver_major_minor() {
    let rng = Rng::new(SEED ^ 13);
    for _ in 0..2000 {
        let prefix = filler(&rng, rng.below(24));
        let payload = dotted(&rng, 2);
        diff_parse(&win(&prefix, &payload), "C13/major.minor");
    }
}

#[test]
fn c14_ver_major_minor_build() {
    let rng = Rng::new(SEED ^ 14);
    for _ in 0..2000 {
        let prefix = filler(&rng, rng.below(24));
        let payload = dotted(&rng, 3);
        diff_parse(&win(&prefix, &payload), "C14/major.minor.build");
    }
    for lit in ["10.0.19045", "6.1.7601", "0.0.0", "10.0.22621"] {
        diff_parse(
            &win(b"Microsoft Windows 11 Pro", lit.as_bytes()),
            "C14/literal",
        );
    }
}

#[test]
fn c15_ver_multidot_build() {
    let rng = Rng::new(SEED ^ 15);
    for n in 4..=8 {
        for _ in 0..600 {
            let prefix = filler(&rng, rng.below(20));
            let payload = dotted(&rng, n);
            diff_parse(&win(&prefix, &payload), "C15/multidot");
        }
    }
    for lit in [
        "10.0.19045.3803",
        "6.3.9600.20520.1",
        "1.2.3.4.5.6.7.8.9",
        "10.0.19045.",
        "10.0.19045..",
        "10.0.19045.3803.",
    ] {
        diff_parse(&win(b"Windows", lit.as_bytes()), "C15/literal");
    }
}

#[test]
fn c16_ver_mixed_junk() {
    let rng = Rng::new(SEED ^ 16);
    for _ in 0..3000 {
        let prefix = filler(&rng, rng.below(16));
        let payload = match rng.below(8) {
            0 => cat(&[&dotted(&rng, rng.range(1, 3)), &filler(&rng, rng.range(1, 6))]),
            1 => cat(&[&filler(&rng, rng.range(1, 6)), &dotted(&rng, rng.range(1, 3))]),
            2 => cat(&[&dotted(&rng, 1), b".", &filler(&rng, 2)]),
            3 => cat(&[&dotted(&rng, 2), b"...."]),
            4 => filler(&rng, rng.range(1, 12)),
            5 => Vec::new(),
            6 => cat(&[b".", &dotted(&rng, 2)]),
            _ => rng.bytes_from(b"0123456789.abcXY -", rng.below(20)),
        };
        diff_parse(&win(&prefix, &payload), "C16/junk");
    }
    for lit in [
        "", ".", "..", "abc", "1a", "a1", "1.a", "1.2a", "1.2.a", "-1", "+1", "1 2", " 1.2.3",
        "1.2.3 ", "1..2", "1.2..3", "1.2.3..4",
    ] {
        diff_parse(&win(b"Win", lit.as_bytes()), "C16/literal");
    }
}

#[test]
fn c17_ver_prefix_shapes() {
    let rng = Rng::new(SEED ^ 17);
    for _ in 0..2500 {
        let payload = dotted(&rng, rng.range(1, 4));
        let prefix: Vec<u8> = match rng.below(6) {
            0 => Vec::new(),
            1 => filler(&rng, rng.range(1, 30)),
            2 => cat(&[&filler(&rng, rng.below(8)), b"|", &filler(&rng, rng.below(8))]),
            // an arch token in the prefix must NOT produce os_arch on this path
            3 => cat(&[&filler(&rng, rng.below(8)), s(rng.arch()).as_slice(), &filler(&rng, rng.below(8))]),
            4 => cat(&[b"Microsoft Windows Server 2019 ", s(rng.arch()).as_slice()]),
            _ => rng.bytes_from(HOSTILE_ALPHA, rng.below(24)),
        };
        diff_parse(&win(&prefix, &payload), "C17/prefix");
    }
}

#[test]
fn c18_ver_repeated_marker() {
    let rng = Rng::new(SEED ^ 18);
    for _ in 0..2000 {
        let a = dotted(&rng, rng.range(1, 4));
        let b = dotted(&rng, rng.range(1, 4));
        let p1 = filler(&rng, rng.below(10));
        let p2 = filler(&rng, rng.below(10));
        // two markers: strstr finds the first
        diff_parse(
            &cat(&[&p1, b" [Ver: ", &a, b"]", &p2, b" [Ver: ", &b, b"]"]),
            "C18/two-markers",
        );
        // three
        diff_parse(
            &cat(&[&p1, b" [Ver: ", &a, b" [Ver: ", &b, b" [Ver: ", &a, b"]"]),
            "C18/three-markers",
        );
    }
}

#[test]
fn c19_ver_payload_contains_posix_separators() {
    let rng = Rng::new(SEED ^ 19);
    let seps: [&[u8]; 6] = [b" [", b": ", b" (", b"|", b"]", b" "];
    for _ in 0..3000 {
        let prefix = filler(&rng, rng.below(12));
        let mut payload = dotted(&rng, rng.range(1, 3));
        let n = rng.range(1, 3);
        for _ in 0..n {
            payload.extend_from_slice(*rng.pick(&seps));
            payload.extend_from_slice(&filler(&rng, rng.below(6)));
        }
        diff_parse(&win(&prefix, &payload), "C19/payload-seps");
    }
    // Also: " [" occurring BEFORE " [Ver: " — the Windows path must still win.
    for _ in 0..1000 {
        let payload = dotted(&rng, rng.range(1, 4));
        let name = filler(&rng, rng.range(1, 8));
        let ver = dotted(&rng, 2);
        diff_parse(
            &cat(&[b"host [", &name, b": ", &ver, b"] [Ver: ", &payload, b"]"]),
            "C19/bracket-before-ver",
        );
    }
}

// ===========================================================================
// C20..C29 — parse_uname_string, POSIX path
// ===========================================================================

#[test]
fn c20_posix_plain() {
    let rng = Rng::new(SEED ^ 20);
    for _ in 0..2500 {
        let prefix = filler(&rng, rng.below(24));
        let name = filler(&rng, rng.range(1, 16));
        let ver = dotted(&rng, rng.range(1, 4));
        diff_parse(&posix(&prefix, &name, &ver), "C20/plain");
    }
    for (n, v) in [
        ("Ubuntu", "22.04.3"),
        ("CentOS Linux", "7.9"),
        ("Debian GNU/Linux", "12"),
        ("Alpine Linux", "3.18.4"),
        ("Mac OS X", "13.5.2"),
    ] {
        diff_parse(&posix(b"host 5.15.0-generic", n.as_bytes(), v.as_bytes()), "C20/lit");
    }
}

#[test]
fn c21_posix_codename() {
    let rng = Rng::new(SEED ^ 21);
    for _ in 0..2500 {
        let prefix = filler(&rng, rng.below(20));
        let name = filler(&rng, rng.range(1, 14));
        let ver = dotted(&rng, rng.range(1, 4));
        let code = filler(&rng, rng.below(14));
        let version_field = cat(&[&ver, b" (", &code, b")"]);
        diff_parse(&posix(&prefix, &name, &version_field), "C21/codename");
    }
    for (n, v, c) in [
        ("Ubuntu", "22.04.3 LTS", "Jammy Jellyfish"),
        ("Debian GNU/Linux", "12", "bookworm"),
        ("CentOS Linux", "7.9.2009", "Core"),
    ] {
        diff_parse(
            &posix(
                b"host 5.15.0",
                n.as_bytes(),
                format!("{v} ({c})").as_bytes(),
            ),
            "C21/lit",
        );
    }
}

#[test]
fn c22_posix_pipe() {
    let rng = Rng::new(SEED ^ 22);
    for _ in 0..2500 {
        let prefix = filler(&rng, rng.below(20));
        let name = filler(&rng, rng.below(14));
        let plat = filler(&rng, rng.below(14));
        let ver = dotted(&rng, rng.range(1, 4));
        diff_parse(
            &posix(&prefix, &cat(&[&name, b"|", &plat]), &ver),
            "C22/pipe",
        );
    }
    for (n, p, v) in [
        ("Ubuntu", "ubuntu", "22.04"),
        ("CentOS Linux", "centos", "7.9"),
        ("Amazon Linux", "amzn", "2023"),
    ] {
        diff_parse(
            &posix(
                b"host 6.1.0",
                format!("{n}|{p}").as_bytes(),
                v.as_bytes(),
            ),
            "C22/lit",
        );
    }
    // multiple pipes: strstr finds the first, platform gets everything after it
    for _ in 0..800 {
        let a = filler(&rng, rng.below(6));
        let b = filler(&rng, rng.below(6));
        let c = filler(&rng, rng.below(6));
        let ver = dotted(&rng, 2);
        diff_parse(
            &posix(b"h", &cat(&[&a, b"|", &b, b"|", &c]), &ver),
            "C22/multi-pipe",
        );
    }
}

#[test]
fn c23_posix_codename_and_pipe() {
    let rng = Rng::new(SEED ^ 23);
    for _ in 0..4000 {
        let prefix = filler(&rng, rng.below(18));
        let name = filler(&rng, rng.below(10));
        let plat = filler(&rng, rng.below(10));
        let ver = dotted(&rng, rng.range(1, 4));
        let code = filler(&rng, rng.below(12));
        let have_pipe = rng.bool();
        let have_code = rng.bool();
        let name_field = if have_pipe {
            cat(&[&name, b"|", &plat])
        } else {
            name.clone()
        };
        let ver_field = if have_code {
            cat(&[&ver, b" (", &code, b")"])
        } else {
            ver.clone()
        };
        diff_parse(&posix(&prefix, &name_field, &ver_field), "C23/cross");
    }
}

#[test]
fn c24_posix_no_colon_pipe_cross() {
    let rng = Rng::new(SEED ^ 24);
    for _ in 0..3000 {
        let prefix = filler(&rng, rng.below(20));
        let name = filler(&rng, rng.below(18));
        let body = if rng.bool() {
            cat(&[&name, b"|", &filler(&rng, rng.below(12))])
        } else {
            name.clone()
        };
        diff_parse(&posix_nocolon(&prefix, &body), "C24/no-colon");
    }
    for lit in ["Ubuntu", "x", "", "a|b", "|", "a|", "|b", "Linux 5.15"] {
        diff_parse(&posix_nocolon(b"host", lit.as_bytes()), "C24/lit");
    }
}

#[test]
fn c25_posix_arch_each_and_after_bracket() {
    let rng = Rng::new(SEED ^ 25);
    for a in ARCHS {
        for _ in 0..200 {
            let pre = filler(&rng, rng.below(12));
            let post = filler(&rng, rng.below(12));
            let name = filler(&rng, rng.range(1, 10));
            let ver = dotted(&rng, rng.range(1, 4));
            // arch in the prefix -> found
            diff_parse(
                &posix(&cat(&[&pre, a.as_bytes(), &post]), &name, &ver),
                "C25/arch-in-prefix",
            );
            // arch ONLY after " [" -> prefix is truncated, so NOT found
            diff_parse(
                &posix(&pre, &cat(&[&name, a.as_bytes()]), &ver),
                "C25/arch-in-name",
            );
            diff_parse(
                &posix(&pre, &name, &cat(&[&ver, b" ", a.as_bytes()])),
                "C25/arch-in-version",
            );
        }
    }
}

#[test]
fn c26_posix_no_bracket_arch_cross() {
    let rng = Rng::new(SEED ^ 26);
    for _ in 0..3000 {
        let body = match rng.below(4) {
            0 => filler(&rng, rng.below(40)),
            1 => cat(&[
                &filler(&rng, rng.below(14)),
                s(rng.arch()).as_slice(),
                &filler(&rng, rng.below(14)),
            ]),
            2 => rng.bytes_from(b"abc[]:()| .0123", rng.below(30)),
            _ => cat(&[b"Linux host 5.15.0-generic #1 SMP ", s(rng.arch()).as_slice()]),
        };
        // ensure the " [" marker is genuinely absent for the "no" arm
        if !body.windows(2).any(|w| w == b" [") && !body.windows(7).any(|w| w == b" [Ver: ") {
            diff_parse(&body, "C26/no-bracket");
        } else {
            diff_parse(&body, "C26/no-bracket-or-bracket");
        }
    }
    for lit in [
        "Linux host 5.15.0 x86_64",
        "Linux host 5.15.0",
        "SunOS s11 5.11 i86pc",
        "AIX p7 1 7",
        "Darwin mac 22.6.0 arm64",
        "",
        "x",
    ] {
        diff_parse(lit.as_bytes(), "C26/lit");
    }
}

#[test]
fn c27_posix_pipe_after_colon() {
    let rng = Rng::new(SEED ^ 27);
    for _ in 0..2000 {
        let prefix = filler(&rng, rng.below(14));
        let name = filler(&rng, rng.range(1, 10));
        let ver = dotted(&rng, rng.range(1, 3));
        // "|" lives in the VERSION part; os_name is already truncated at ": "
        // so lib.c:135 must not find it.
        diff_parse(
            &posix(&prefix, &name, &cat(&[&ver, b"|", &filler(&rng, rng.below(8))])),
            "C27/pipe-in-version",
        );
        // "|" in both parts: only the one in os_name counts
        diff_parse(
            &posix(
                &prefix,
                &cat(&[&name, b"|", &filler(&rng, rng.below(6))]),
                &cat(&[&ver, b"|", &filler(&rng, rng.below(6))]),
            ),
            "C27/pipe-in-both",
        );
        // "|" in the prefix only (dropped entirely)
        diff_parse(
            &posix(&cat(&[&prefix, b"|", &filler(&rng, rng.below(6))]), &name, &ver),
            "C27/pipe-in-prefix",
        );
    }
}

#[test]
fn c28_posix_repeated_separators() {
    let rng = Rng::new(SEED ^ 28);
    for _ in 0..3000 {
        let f1 = filler(&rng, rng.below(8));
        let f2 = filler(&rng, rng.below(8));
        let f3 = filler(&rng, rng.below(8));
        let ver = dotted(&rng, rng.range(1, 3));
        match rng.below(5) {
            0 => diff_parse(
                &cat(&[&f1, b" [", &f2, b" [", &f3, b": ", &ver, b"]"]),
                "C28/two-brackets",
            ),
            1 => diff_parse(
                &cat(&[&f1, b" [", &f2, b": ", &f3, b": ", &ver, b"]"]),
                "C28/two-colons",
            ),
            2 => diff_parse(
                &cat(&[
                    &f1, b" [", &f2, b": ", &ver, b" (", &f3, b" (", &f1, b")]",
                ]),
                "C28/two-parens",
            ),
            3 => diff_parse(
                &cat(&[
                    &f1, b" [", &f2, b": ", &ver, b"] [", &f3, b": ", &ver, b"]",
                ]),
                "C28/two-groups",
            ),
            _ => diff_parse(
                &cat(&[
                    &f1, b" [ [ [", &f2, b": : ", &ver, b" ( (", &f3, b")]",
                ]),
                "C28/dense",
            ),
        }
    }
}

#[test]
fn c29_posix_version_shapes() {
    let rng = Rng::new(SEED ^ 29);
    for n in 1..=6 {
        for _ in 0..600 {
            let prefix = filler(&rng, rng.below(16));
            let name = filler(&rng, rng.range(1, 12));
            let ver = dotted(&rng, n);
            diff_parse(&posix(&prefix, &name, &ver), "C29/dotted-n");
        }
    }
    for lit in [
        "", "1", "1.2", "1.2.3", "1.2.3.4", "0", "0.0", "007.008", "abc", "1a.2b", "a1.b2",
        "22.04 LTS", "10.15.7", ".", "..", "1.", "1..", "1.2.", "4294967296.4294967297",
        "99999999999999999999.88888888888888888888",
    ] {
        diff_parse(&posix(b"host", b"Distro", lit.as_bytes()), "C29/lit");
    }
}

// ===========================================================================
// C30..C34 — cross-cutting shapes
// ===========================================================================

#[test]
fn c30_prefilled_osdata() {
    let rng = Rng::new(SEED ^ 30);
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..400 {
        inputs.push(win(
            &filler(&rng, rng.below(12)),
            &dotted(&rng, rng.range(1, 4)),
        ));
        let name = filler(&rng, rng.range(1, 10));
        let ver = dotted(&rng, rng.range(1, 3));
        inputs.push(posix(&filler(&rng, rng.below(12)), &name, &ver));
        inputs.push(posix(
            &cat(&[&filler(&rng, rng.below(6)), s(rng.arch()).as_slice()]),
            &cat(&[&name, b"|", &filler(&rng, rng.below(6))]),
            &cat(&[&ver, b" (", &filler(&rng, rng.below(8)), b")"]),
        ));
        inputs.push(posix_nocolon(&filler(&rng, rng.below(10)), &name));
        inputs.push(filler(&rng, rng.below(24)));
        inputs.push(Vec::new());
    }
    for i in &inputs {
        diff_parse_prefilled(i, "C30/prefilled");
    }
}

#[test]
fn c31_long_and_non_ascii() {
    let rng = Rng::new(SEED ^ 31);
    for _ in 0..600 {
        let big = |r: &Rng, n: usize| r.bytes_from(SAFE_ALPHA, n);
        let prefix = big(&rng, rng.range(256, 700));
        let name = big(&rng, rng.range(64, 300));
        let ver = dotted(&rng, rng.range(2, 5));
        let code = big(&rng, rng.range(64, 300));
        diff_parse(&win(&prefix, &ver), "C31/long-win");
        diff_parse(&posix(&prefix, &name, &ver), "C31/long-posix");
        diff_parse(
            &posix(&prefix, &cat(&[&name, b"|", &big(&rng, 200)]), &cat(&[&ver, b" (", &code, b")"])),
            "C31/long-full",
        );
    }
    let hi: &[u8] = b"\x80\x81\xa0\xc3\xa9\xe2\x82\xac\xf0\x9f\x92\xa9\xfe\xff\x7f\x01\x02";
    for _ in 0..1500 {
        let prefix = rng.bytes_from(hi, rng.below(20));
        let name = rng.bytes_from(hi, rng.below(20));
        let code = rng.bytes_from(hi, rng.below(20));
        let ver = dotted(&rng, rng.range(1, 4));
        diff_parse(&win(&prefix, &cat(&[&ver, &rng.bytes_from(hi, rng.below(6))])), "C31/hi-win");
        diff_parse(&posix(&prefix, &name, &ver), "C31/hi-posix");
        diff_parse(
            &posix(&prefix, &name, &cat(&[&ver, b" (", &code, b")"])),
            "C31/hi-codename",
        );
    }
}

#[test]
fn c32_full_random_fuzz() {
    let rng = Rng::new(SEED ^ 32);
    // Alphabet dense in every byte the parser special-cases, so all branch
    // combinations are reached by construction.
    let alpha: &[u8] = b"  [[]]:::  ((()))||...VVeerr  0123456789abz\x80\xff\t";
    for _ in 0..6000 {
        let len = rng.below(161);
        diff_parse(&rng.bytes_from(alpha, len), "C32/fuzz");
    }
    // Fragment-assembly fuzz: splice real separator tokens together.
    let frags: [&[u8]; 20] = [
        b" [Ver: ", b" [", b": ", b" (", b")", b"]", b"|", b" ", b".", b"0", b"12", b"345",
        b"abc", b"Ver", b"Ver: ", b"[", b"  ", b"..", b"x86_64", b"aarch64",
    ];
    for _ in 0..6000 {
        let n = rng.range(1, 12);
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(*rng.pick(&frags));
        }
        diff_parse(&v, "C32/fragments");
    }
}

#[test]
fn c33_realistic_corpus() {
    let corpus = [
        "Microsoft Windows 10 Pro [Ver: 10.0.19045.3803]",
        "Microsoft Windows Server 2019 Datacenter [Ver: 10.0.17763.4974]",
        "Microsoft Windows 7 Professional [Ver: 6.1.7601]",
        "Microsoft Windows 11 Home [Ver: 10.0.22621.2861]",
        "Microsoft Windows XP [Ver: 5.1.2600]",
        "Linux ubuntu 5.15.0-88-generic #98-Ubuntu SMP x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]",
        "Linux centos7 3.10.0-1160.el7.x86_64 #1 SMP x86_64 [CentOS Linux|centos: 7.9.2009 (Core)]",
        "Linux deb12 6.1.0-13-amd64 #1 SMP amd64 [Debian GNU/Linux|debian: 12 (bookworm)]",
        "Linux alpine 6.1.55-0-lts #1-Alpine SMP x86_64 [Alpine Linux|alpine: 3.18.4]",
        "Linux amzn2023 6.1.61-85.141.amzn2023.x86_64 #1 SMP x86_64 [Amazon Linux|amzn: 2023]",
        "Linux rpi 6.1.21-v7+ #1642 SMP armv7l [Raspbian GNU/Linux|raspbian: 11 (bullseye)]",
        "Linux arm 5.10.0 #1 SMP aarch64 [Ubuntu|ubuntu: 20.04.6 LTS (Focal Fossa)]",
        "Darwin mac.local 22.6.0 Darwin Kernel Version 22.6.0 arm64 [Mac OS X|darwin: 13.5.2 (Ventura)]",
        "Darwin mac.local 21.6.0 x86_64 [Mac OS X|darwin: 12.7]",
        "SunOS solaris 5.11 11.4.42.111.0 i86pc [SunOS|sunos: 11.4]",
        "SunOS sol10 5.10 Generic_147148-26 sparc [SunOS|sunos: 5.10]",
        "AIX aixhost 1 7 00F84C0C4C00 [AIX|aix: 7.2]",
        "HP-UX hpux B.11.31 U ia64 [HP-UX|hpux: 11.31]",
        "FreeBSD bsd 13.2-RELEASE amd64 [FreeBSD|freebsd: 13.2]",
        "Linux nover 5.15.0 i686 [SomeDistro]",
        "Linux nopipe 5.15.0 i386 [SomeDistro: 1.2.3]",
        "Linux noarchhere 5.15.0 [D|p: 9 (nine)]",
        "Linux x86_64",
        "Linux",
        "",
        " [Ver: ]",
        " []",
        " [: ]",
        "a [Ver: 1.2.3.4.5]",
        "x86_64 [Ver: 10.0.1]",
        "windows [Ver: 10.0.19041.1] [Ubuntu: 22.04]",
    ];
    for c in corpus {
        diff_parse(c.as_bytes(), "C33/corpus");
        diff_parse_prefilled(c.as_bytes(), "C33/corpus-prefilled");
    }
}

#[test]
fn c34_degenerate_separator_only() {
    let cases: [&str; 40] = [
        "",
        " ",
        "  ",
        "[",
        "]",
        ":",
        "(",
        ")",
        "|",
        " [",
        " []",
        " []]",
        " [Ver: ",
        " [Ver: ]",
        " [Ver:",
        " [Ver",
        "[Ver: 1]",
        " [: ",
        " [: ]",
        " [ ( ",
        " [a: ",
        " [a: ]",
        " [a: b (",
        " [a: b ()",
        " [a: b ( )",
        " [|",
        " [|]",
        " [a|",
        " [a|]",
        " [|a]",
        " [a|b",
        " [a: b|c]",
        " [ ",
        " [  ",
        "  [  ",
        " [ [ ",
        " [Ver:  ",
        " [Ver: 1",
        "  [Ver: 1]",
        " [Ver: ] [Ver: ]",
    ];
    for c in cases {
        diff_parse(c.as_bytes(), "C34/degenerate");
        diff_parse_prefilled(c.as_bytes(), "C34/degenerate-prefilled");
    }
}

// ===========================================================================
// C35 — allocation-size boundaries
// ===========================================================================

#[test]
fn c35_allocation_size_boundaries() {
    // `lib.c:77,84,91` allocate exactly `match_size + 1` bytes and `strdup`
    // allocates `strlen + 1`. glibc's usable size is a step function of the
    // request, so sweeping the match length across every 16-byte bin boundary
    // makes an off-by-one in the requested size observable through
    // `malloc_usable_size` (which `diff_parse` compares for every field).
    for n in 1..=100usize {
        let d = vec![b'9'; n];
        // os_major on the Ver path
        diff_parse(&cat(&[b"w [Ver: ", &d, b"]"]), "C35/ver-major");
        // os_minor on the Ver path
        diff_parse(&cat(&[b"w [Ver: 1.", &d, b"]"]), "C35/ver-minor");
        // os_build on the Ver path
        diff_parse(&cat(&[b"w [Ver: 1.2.", &d, b"]"]), "C35/ver-build");
        // multi-dot build: match_size spans the whole `([0-9]+(\.[0-9]+)*)` group
        diff_parse(&cat(&[b"w [Ver: 1.2.", &d, b".", &d, b"]"]), "C35/ver-build-multi");
        // os_major / os_minor on the POSIX path
        diff_parse(&cat(&[b"h [D: ", &d, b"]"]), "C35/posix-major");
        diff_parse(&cat(&[b"h [D: 1.", &d, b"]"]), "C35/posix-minor");
    }
    // strdup'd fields: os_name, os_version, os_codename, os_platform, os_arch
    let rng = Rng::new(SEED ^ 35);
    for n in 0..=100usize {
        let f = rng.bytes_from(SAFE_ALPHA, n);
        diff_parse(&cat(&[&f, b" [Ver: 1.2.3]"]), "C35/strdup-name-win");
        diff_parse(&posix(b"h", &f, b"1.2"), "C35/strdup-name");
        diff_parse(&posix(b"h", b"D", &f), "C35/strdup-version");
        diff_parse(&posix(b"h", &cat(&[b"D|", &f]), b"1.2"), "C35/strdup-platform");
        diff_parse(
            &posix(b"h", b"D", &cat(&[b"1.2 (", &f, b")"])),
            "C35/strdup-codename",
        );
        diff_arch(&cat(&[&f, b"x86_64"]), "C35/strdup-arch");
    }
}
