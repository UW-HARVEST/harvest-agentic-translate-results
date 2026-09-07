//! Phase B — valid-path differential tests, one test function per
//! `CONFIGS.md` row group. Every case is executed against both `.so` exports
//! and every observable byte is compared.

mod common;
use common::*;

const N: usize = 200; // randomized inputs per row

// ===========================================================================
// get_os_arch — rows 1..8
// ===========================================================================

/// CONFIGS row 1 — exactly one arch token, randomized text and position.
#[test]
fn cfg01_arch_single_token_each_arch() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 1);
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        for _ in 0..N {
            let pre = rng.word_between(0, 12);
            let post = rng.word_between(0, 12);
            let mut s = Vec::new();
            s.extend_from_slice(&pre);
            if rng.bool() {
                s.push(b' ');
            }
            s.extend_from_slice(arch);
            if rng.bool() {
                s.push(b' ');
            }
            s.extend_from_slice(&post);
            cases.push(Case::arch(&s));
        }
    }
    assert_same(&c, &r, &cases, "cfg01 get_os_arch single token");
}

/// CONFIGS row 2 — several arch tokens: ARCHS order wins over string order.
#[test]
fn cfg02_arch_multiple_tokens_priority_order() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases = Vec::new();
    for _ in 0..(N * 4) {
        let k = rng.range(2, 5);
        let mut s = Vec::new();
        for i in 0..k {
            if i > 0 {
                s.push(b' ');
            }
            s.extend_from_slice(rng.pick(&ARCHS));
        }
        cases.push(Case::arch(&s));
    }
    // exhaustive ordered pairs too
    for a in ARCHS.iter() {
        for b in ARCHS.iter() {
            let mut s = a.to_vec();
            s.push(b'-');
            s.extend_from_slice(b);
            cases.push(Case::arch(&s));
        }
    }
    assert_same(&c, &r, &cases, "cfg02 get_os_arch priority order");
}

/// CONFIGS row 3 — overlapping / confusable tokens.
#[test]
fn cfg03_arch_overlapping_tokens() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"i386", b"i686", b"i3866", b"ii386", b"xi686x", b"amd64", b"ia64", b"amd64ia64",
        b"ia64amd64", b"x86_64", b"i86pc", b"x86_64 i86pc", b"i86pc x86_64", b"86_64",
        b"aarch64", b"arm64", b"aarch64 arm64", b"arm64 aarch64", b"armv6", b"armv7",
        b"armv67", b"armv76", b"AIX", b"aix", b"AIXX", b"xAIX", b"sparc", b"sparcv9",
        b"SPARC", b"i86pcx86_64", b"aarch64arm64", b"x86_64aarch64",
    ];
    let cases: Vec<Case> = fixed.iter().map(|s| Case::arch(s)).collect();
    assert_same(&c, &r, &cases, "cfg03 get_os_arch overlapping tokens");
}

/// CONFIGS row 4 — token embedded in a longer word (strstr is not anchored).
#[test]
fn cfg04_arch_token_inside_word() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        for _ in 0..(N / 4) {
            let mut s = rng.word_between(1, 6);
            s.extend_from_slice(arch);
            s.extend_from_slice(&rng.word_between(1, 6));
            cases.push(Case::arch(&s));
        }
    }
    assert_same(&c, &r, &cases, "cfg04 get_os_arch token inside word");
}

/// CONFIGS row 5 — near-miss spellings that must NOT match.
#[test]
fn cfg05_arch_near_misses() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"x86-64", b"x86 64", b"X86_64", b"i 386", b"i-686", b"arm_64", b"arm 64",
        b"aarch-64", b"AARCH64", b"Arm64", b"i86-pc", b"ia-64", b"amd-64", b"armv 6",
        b"spar c", b"Sparc", b"ai x", b"x8664", b"i68", b"i38", b"arm6", b"aarch6",
    ];
    let cases: Vec<Case> = fixed.iter().map(|s| Case::arch(s)).collect();
    assert_same(&c, &r, &cases, "cfg05 get_os_arch near misses");
}

/// CONFIGS row 6 — random noise, length 0..64.
#[test]
fn cfg06_arch_random_noise() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 6);
    let mut cases = Vec::new();
    for _ in 0..(N * 10) {
        let len = rng.range(0, 64);
        cases.push(Case::arch(&rng.bytes(len)));
    }
    assert_same(&c, &r, &cases, "cfg06 get_os_arch random noise");
}

/// CONFIGS row 7 — realistic uname lines.
#[test]
fn cfg07_arch_realistic_unames() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = real_unames().iter().map(|s| Case::arch(s)).collect();
    assert_same(&c, &r, &cases, "cfg07 get_os_arch realistic unames");
}

/// CONFIGS row 8 — 64 KiB input with the token at the very end.
#[test]
fn cfg08_arch_huge_input() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 8);
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        let mut s = rng.word(64 * 1024);
        s.extend_from_slice(arch);
        cases.push(Case::arch(&s));
    }
    // and a 64 KiB input with no token at all
    cases.push(Case::arch(&rng.word(64 * 1024)));
    assert_same(&c, &r, &cases, "cfg08 get_os_arch 64KiB input");
}

// ===========================================================================
// w_regexec — rows 9..21
// ===========================================================================

const PAT_MAJOR: &[u8] = b"^([0-9]+)\\.*";
const PAT_MINOR: &[u8] = b"^[0-9]+\\.([0-9]+)\\.*";
const PAT_BUILD: &[u8] = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*";

/// CONFIGS row 9 — the three real patterns against randomized version strings.
#[test]
fn cfg09_regexec_real_patterns() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 9);
    let mut cases = Vec::new();
    for _ in 0..(N * 4) {
        let comps = rng.range(1, 6);
        let mut v = rng.version(comps);
        match rng.below(4) {
            0 => {}
            1 => v.extend_from_slice(&rng.word_between(1, 5)),
            2 => {
                v.push(b'.');
            }
            _ => {
                let mut w = rng.word_between(1, 3);
                w.extend_from_slice(&v);
                v = w;
            }
        }
        for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD] {
            cases.push(Case::regex(pat, &v, 2, 2));
        }
    }
    assert_same(&c, &r, &cases, "cfg09 w_regexec real patterns");
}

/// CONFIGS row 10 — zero-group pattern with nmatch=2.
#[test]
fn cfg10_regexec_zero_group_pattern() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 10);
    let mut cases = Vec::new();
    for _ in 0..N {
        let s = if rng.bool() {
            rng.version_between(1, 4)
        } else {
            rng.bytes_between(0, 20)
        };
        for pat in [&b"^[0-9]+"[..], &b"[a-z]+"[..], &b"^$"[..], &b"."[..]] {
            cases.push(Case::regex(pat, &s, 2, 2));
        }
    }
    assert_same(&c, &r, &cases, "cfg10 w_regexec zero-group pattern");
}

/// CONFIGS row 11 — nmatch smaller than the group count.
#[test]
fn cfg11_regexec_nmatch_one() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 11);
    let mut cases = Vec::new();
    for _ in 0..N {
        let s = rng.version_between(1, 4);
        cases.push(Case::regex(PAT_MAJOR, &s, 1, 2));
        cases.push(Case::regex(PAT_MINOR, &s, 1, 2));
        cases.push(Case::regex(PAT_BUILD, &s, 1, 4));
    }
    assert_same(&c, &r, &cases, "cfg11 w_regexec nmatch=1");
}

/// CONFIGS row 12 — nested groups with nmatch=3.
#[test]
fn cfg12_regexec_nested_groups() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 12);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let s = rng.version_between(1, 7);
        cases.push(Case::regex(PAT_BUILD, &s, 3, 3));
        cases.push(Case::regex(b"(([0-9]+)\\.([0-9]+))", &s, 4, 4));
        cases.push(Case::regex(b"^([0-9]+(\\.[0-9]+)*)$", &s, 3, 3));
    }
    assert_same(&c, &r, &cases, "cfg12 w_regexec nested groups");
}

/// CONFIGS row 13 — nmatch=0 with a real array: array must stay untouched.
#[test]
fn cfg13_regexec_nmatch_zero() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 13);
    let mut cases = Vec::new();
    for _ in 0..N {
        let s = rng.version_between(1, 4);
        for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD, b"zzz"] {
            cases.push(Case::regex(pat, &s, 0, 2));
        }
    }
    assert_same(&c, &r, &cases, "cfg13 w_regexec nmatch=0");
}

/// CONFIGS row 14 — nmatch larger than re_nsub+1: surplus entries are -1/-1.
#[test]
fn cfg14_regexec_nmatch_surplus() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 14);
    let mut cases = Vec::new();
    for _ in 0..N {
        let s = rng.version_between(1, 4);
        for nm in [3usize, 4, 8, 16] {
            cases.push(Case::regex(PAT_MAJOR, &s, nm, nm));
            cases.push(Case::regex(PAT_BUILD, &s, nm, nm));
        }
    }
    assert_same(&c, &r, &cases, "cfg14 w_regexec surplus nmatch");
}

/// CONFIGS row 15 — very large nmatch with a matching array.
#[test]
fn cfg15_regexec_nmatch_4096() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 15);
    let mut cases = Vec::new();
    for _ in 0..8 {
        let s = rng.version_between(1, 5);
        cases.push(Case::regex(PAT_MAJOR, &s, 4096, 4096));
        cases.push(Case::regex(PAT_BUILD, &s, 4096, 4096));
        cases.push(Case::regex(b"nomatch", &s, 4096, 4096));
    }
    assert_same(&c, &r, &cases, "cfg15 w_regexec nmatch=4096");
}

/// CONFIGS row 16 — anchored vs unanchored, match at offset 0 vs deeper.
#[test]
fn cfg16_regexec_anchoring() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 16);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let pre = rng.word_between(0, 8);
        let v = rng.version_between(1, 4);
        let mut s = pre.clone();
        s.extend_from_slice(&v);
        for pat in [
            &b"^([0-9]+)"[..],
            &b"([0-9]+)"[..],
            &b"([0-9]+)$"[..],
            &b"^(.*)$"[..],
            &b"([a-z]*)([0-9]*)"[..],
        ] {
            cases.push(Case::regex(pat, &s, 3, 3));
        }
    }
    assert_same(&c, &r, &cases, "cfg16 w_regexec anchoring");
}

/// CONFIGS row 17 — assorted REG_EXTENDED constructs.
#[test]
fn cfg17_regexec_extended_constructs() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 17);
    let pats: &[&[u8]] = &[
        b"(a|b)+",
        b"[[:digit:]]{2,4}",
        b"([0-9]{1,3})\\.([0-9]{1,3})",
        b"^[^0-9]*([0-9]+)",
        b"(x?y?z?)",
        b"a{0,}",
        b"[-.]+",
        b"([A-Z]+)|([a-z]+)",
        b"(\\.)*",
        b"^$",
        b"()",
        b"(|a)",
    ];
    let mut cases = Vec::new();
    for _ in 0..N {
        let s = if rng.bool() {
            rng.bytes_between(0, 24)
        } else {
            rng.version_between(1, 4)
        };
        for pat in pats {
            cases.push(Case::regex(pat, &s, 3, 3));
        }
    }
    assert_same(&c, &r, &cases, "cfg17 w_regexec extended constructs");
}

/// CONFIGS row 18 — empty pattern.
#[test]
fn cfg18_regexec_empty_pattern() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 18);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let s = rng.bytes_between(0, 24);
        cases.push(Case::regex(b"", &s, 2, 2));
        cases.push(Case::regex(b"", &s, 0, 2));
    }
    assert_same(&c, &r, &cases, "cfg18 w_regexec empty pattern");
}

/// CONFIGS row 19 — empty subject.
#[test]
fn cfg19_regexec_empty_subject() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 19);
    let mut cases = Vec::new();
    for pat in [PAT_MAJOR, PAT_MINOR, PAT_BUILD, b"", b"^$", b".", b"x*"] {
        cases.push(Case::regex(pat, b"", 2, 2));
    }
    for _ in 0..N {
        let pat = rng.version_between(1, 3);
        cases.push(Case::regex(&pat, b"", 2, 2));
    }
    assert_same(&c, &r, &cases, "cfg19 w_regexec empty subject");
}

/// CONFIGS row 20 — high bytes / newlines in the subject.
#[test]
fn cfg20_regexec_high_bytes() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 20);
    let mut cases = Vec::new();
    for _ in 0..(N * 4) {
        let mut s = Vec::new();
        for _ in 0..rng.range(0, 24) {
            s.push(match rng.below(3) {
                0 => 0x80 + rng.below(0x80) as u8,
                1 => b'\n',
                _ => b'0' + rng.below(10) as u8,
            });
        }
        for pat in [PAT_MAJOR, PAT_BUILD, &b"^(.+)$"[..], &b"([0-9]+)"[..]] {
            cases.push(Case::regex(pat, &s, 3, 3));
        }
    }
    assert_same(&c, &r, &cases, "cfg20 w_regexec high bytes");
}

/// CONFIGS row 21 — 64 KiB subject, unanchored match near the end.
#[test]
fn cfg21_regexec_huge_subject() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 21);
    let mut cases = Vec::new();
    for _ in 0..4 {
        let mut s = rng.word(64 * 1024);
        s.extend_from_slice(b"12.34.56");
        cases.push(Case::regex(b"([0-9]+)\\.([0-9]+)", &s, 3, 3));
        cases.push(Case::regex(PAT_MAJOR, &s, 2, 2));
        let mut t = b"12.34.56.78.90".to_vec();
        t.extend_from_slice(&rng.word(64 * 1024));
        cases.push(Case::regex(PAT_BUILD, &t, 2, 2));
    }
    assert_same(&c, &r, &cases, "cfg21 w_regexec 64KiB subject");
}

// ===========================================================================
// parse_uname_string — rows 22..56
// ===========================================================================

fn ver_input(rng: &mut Rng, comps: usize) -> Vec<u8> {
    let mut s = rng.word_between(0, 14);
    s.extend_from_slice(b" [Ver: ");
    s.extend_from_slice(&rng.version(comps));
    s.push(b']');
    s
}

/// CONFIGS row 22 — Ver branch, 3 components.
#[test]
fn cfg22_parse_ver_three_components() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 22);
    let cases: Vec<Case> = (0..N * 3).map(|_| Case::parse(&ver_input(&mut rng, 3))).collect();
    assert_same(&c, &r, &cases, "cfg22 parse Ver 3 components");
}

/// CONFIGS row 23 — Ver branch, 4+ dotted components.
#[test]
fn cfg23_parse_ver_many_components() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 23);
    let mut cases = Vec::new();
    for comps in 4..=9 {
        for _ in 0..N {
            cases.push(Case::parse(&ver_input(&mut rng, comps)));
        }
    }
    assert_same(&c, &r, &cases, "cfg23 parse Ver many components");
}

/// CONFIGS row 24 — Ver branch, major.minor only.
#[test]
fn cfg24_parse_ver_two_components() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 24);
    let cases: Vec<Case> = (0..N * 2).map(|_| Case::parse(&ver_input(&mut rng, 2))).collect();
    assert_same(&c, &r, &cases, "cfg24 parse Ver 2 components");
}

/// CONFIGS row 25 — Ver branch, major only.
#[test]
fn cfg25_parse_ver_one_component() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 25);
    let cases: Vec<Case> = (0..N * 2).map(|_| Case::parse(&ver_input(&mut rng, 1))).collect();
    assert_same(&c, &r, &cases, "cfg25 parse Ver 1 component");
}

/// CONFIGS row 26 — Ver branch, non-numeric version.
#[test]
fn cfg26_parse_ver_non_numeric() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 26);
    let mut cases = Vec::new();
    for _ in 0..(N * 3) {
        let mut s = rng.word_between(0, 10);
        s.extend_from_slice(b" [Ver: ");
        s.extend_from_slice(&rng.bytes_between(0, 20));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    for lit in [
        &b"W [Ver: abc]"[..],
        &b"W [Ver: .1.2]"[..],
        &b"W [Ver: -1.2.3]"[..],
        &b"W [Ver: +1.2.3]"[..],
        &b"W [Ver:  1.2.3]"[..],
        &b"W [Ver: v1.2.3]"[..],
        &b"W [Ver: 1..2]"[..],
        &b"W [Ver: 1.2.]"[..],
        &b"W [Ver: .]"[..],
        &b"W [Ver: ]"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    assert_same(&c, &r, &cases, "cfg26 parse Ver non-numeric");
}

/// CONFIGS row 27 — leading zeros and very long digit runs.
#[test]
fn cfg27_parse_ver_long_digit_runs() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for n in [1usize, 2, 8, 20, 40, 100, 400] {
        let d: Vec<u8> = std::iter::repeat(b'9').take(n).collect();
        let z: Vec<u8> = std::iter::repeat(b'0').take(n).collect();
        for digits in [&d, &z] {
            let mut s = b"W [Ver: ".to_vec();
            s.extend_from_slice(digits);
            s.push(b'.');
            s.extend_from_slice(digits);
            s.push(b'.');
            s.extend_from_slice(digits);
            s.push(b']');
            cases.push(Case::parse(&s));
        }
    }
    cases.push(Case::parse(b"W [Ver: 000.000.000]"));
    cases.push(Case::parse(b"W [Ver: 0.0.0]"));
    cases.push(Case::parse(b"W [Ver: 99999999999999999999.1.2]"));
    cases.push(Case::parse(b"W [Ver: 04.05.06]"));
    assert_same(&c, &r, &cases, "cfg27 parse Ver long digit runs");
}

/// CONFIGS row 28 — trailing text after the version.
#[test]
fn cfg28_parse_ver_trailing_text() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 28);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let mut s = rng.word_between(1, 10);
        s.extend_from_slice(b" [Ver: ");
        s.extend_from_slice(&rng.version_between(1, 4));
        s.push(b' ');
        s.extend_from_slice(&rng.word_between(1, 8));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    for lit in [
        &b"Microsoft Windows 10 [Ver: 10.0.19041 SP1]"[..],
        &b"W [Ver: 6.1.7601.65536 Service Pack 1]"[..],
        &b"W [Ver: 10.0.19041]extra"[..],
        &b"W [Ver: 10.0.19041"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    assert_same(&c, &r, &cases, "cfg28 parse Ver trailing text");
}

/// CONFIGS row 29 — Ver branch never sets os_arch.
#[test]
fn cfg29_parse_ver_never_sets_arch() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        let mut s = b"Windows ".to_vec();
        s.extend_from_slice(arch);
        s.extend_from_slice(b" [Ver: 10.0.19041]");
        cases.push(Case::parse(&s));

        let mut t = b"Windows [Ver: 10.0.19041 ".to_vec();
        t.extend_from_slice(arch);
        t.push(b']');
        cases.push(Case::parse(&t));
    }
    assert_same(&c, &r, &cases, "cfg29 parse Ver never sets arch");
}

/// CONFIGS row 30/31 — marker precedence and duplicate markers.
#[test]
fn cfg30_parse_marker_precedence() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"a [b] c [Ver: 1.2.3]",
        b"a [b: 1.2 (x)] c [Ver: 1.2.3]",
        b"a [Ver: 1.2.3] b [c: 4.5]",
        b"a [Ver: 1.2.3] b [Ver: 9.9.9]",
        b"a [Ver: ] b [Ver: 9.9.9]",
        b"[Ver: 1.2.3]",
        b" [Ver: 1.2.3]",
        b"  [Ver: 1.2.3]",
        b"x [Ver: 1.2.3] [Ver: 4.5.6]",
        b"x [Ver:1.2.3]",
        b"x [Ver 1.2.3]",
        b"x [ver: 1.2.3]",
        b"x [VER: 1.2.3]",
        b"x  [Ver: 1.2.3]",
    ];
    let cases: Vec<Case> = fixed.iter().map(|s| Case::parse(s)).collect();
    assert_same(&c, &r, &cases, "cfg30 parse marker precedence");
}

/// CONFIGS row 32 — empty prefix before " [Ver: ".
#[test]
fn cfg32_parse_ver_empty_prefix() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = [
        &b" [Ver: 6.1.7601]"[..],
        &b" [Ver: 1]"[..],
        &b" [Ver: 1.2]"[..],
        &b" [Ver: ]"[..],
    ]
    .iter()
    .map(|s| Case::parse(s))
    .collect();
    assert_same(&c, &r, &cases, "cfg32 parse Ver empty prefix");
}

fn unix_input(rng: &mut Rng, codename: bool, pipe: bool, comps: usize) -> Vec<u8> {
    let mut s = rng.word_between(0, 10);
    s.extend_from_slice(b" [");
    s.extend_from_slice(&rng.word_between(1, 8));
    if pipe {
        s.push(b'|');
        s.extend_from_slice(&rng.word_between(1, 6));
    }
    s.extend_from_slice(b": ");
    s.extend_from_slice(&rng.version(comps));
    if codename {
        s.extend_from_slice(b" (");
        s.extend_from_slice(&rng.word_between(1, 8));
        s.push(b')');
    }
    s.push(b']');
    s
}

/// CONFIGS row 33 — Unix branch, full shape.
#[test]
fn cfg33_parse_unix_full() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 33);
    let cases: Vec<Case> = (0..N * 4)
        .map(|_| {
            let comps = rng.range(1, 4);
            Case::parse(&unix_input(&mut rng, true, false, comps))
        })
        .collect();
    assert_same(&c, &r, &cases, "cfg33 parse Unix full");
}

/// CONFIGS row 34 — Unix branch, no codename.
#[test]
fn cfg34_parse_unix_no_codename() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 34);
    let cases: Vec<Case> = (0..N * 4)
        .map(|_| {
            let comps = rng.range(1, 4);
            Case::parse(&unix_input(&mut rng, false, false, comps))
        })
        .collect();
    assert_same(&c, &r, &cases, "cfg34 parse Unix no codename");
}

/// CONFIGS row 35 — Unix branch, no ": " (else arm strips the last char).
#[test]
fn cfg35_parse_unix_no_colon_space() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 35);
    let mut cases = Vec::new();
    for _ in 0..(N * 3) {
        let mut s = rng.word_between(0, 10);
        s.extend_from_slice(b" [");
        s.extend_from_slice(&rng.word_between(0, 12));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    for lit in [
        &b"host [ubuntu]"[..],
        &b"host [u]"[..],
        &b"host []"[..],
        &b"host ["[..],
        &b"host [a:b]"[..],
        &b"host [a :b]"[..],
        &b"host [: ]"[..],
        &b"host [:]"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    assert_same(&c, &r, &cases, "cfg35 parse Unix no colon-space");
}

/// CONFIGS row 36/37 — pipe in os_name, with and without ": " / codename.
#[test]
fn cfg36_parse_unix_pipe_platform() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 36);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let comps = rng.range(1, 4);
        cases.push(Case::parse(&unix_input(&mut rng, true, true, comps)));
        cases.push(Case::parse(&unix_input(&mut rng, false, true, comps)));
        // pipe but no ": "
        let mut s = rng.word_between(0, 8);
        s.extend_from_slice(b" [");
        s.extend_from_slice(&rng.word_between(1, 6));
        s.push(b'|');
        s.extend_from_slice(&rng.word_between(0, 6));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    for lit in [
        &b"h [ubuntu|debian: 22.04 (jammy)]"[..],
        &b"h [ubuntu|debian: 22.04]"[..],
        &b"h [ubuntu|debian]"[..],
        &b"h [|debian: 1.2]"[..],
        &b"h [ubuntu|: 1.2]"[..],
        &b"h [ubuntu|]"[..],
        &b"h [|]"[..],
        &b"h [a|b|c: 1.2]"[..],
        // pipe AFTER the ": " split point is invisible (os_name truncated)
        &b"h [ubuntu: 22.04|x]"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    assert_same(&c, &r, &cases, "cfg36 parse Unix pipe platform");
}

/// CONFIGS row 38/39 — version shapes in the Unix branch.
#[test]
fn cfg38_parse_unix_version_shapes() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 38);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let mut s = rng.word_between(0, 8);
        s.extend_from_slice(b" [");
        s.extend_from_slice(&rng.word_between(1, 6));
        s.extend_from_slice(b": ");
        s.extend_from_slice(&rng.bytes_between(0, 24));
        s.push(b']');
        cases.push(Case::parse(&s));
    }
    for lit in [
        &b"h [os: 22]"[..],
        &b"h [os: 22.]"[..],
        &b"h [os: .22]"[..],
        &b"h [os: 22.04.1]"[..],
        &b"h [os: jammy]"[..],
        &b"h [os: 22.04 LTS (jammy)]"[..],
        &b"h [os: ]"[..],
        &b"h [os: (x)]"[..],
        &b"h [os:  (x)]"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    assert_same(&c, &r, &cases, "cfg38 parse Unix version shapes");
}

/// CONFIGS rows 40/41/42 — arch position relative to " [".
#[test]
fn cfg40_parse_unix_arch_position() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for arch in ARCHS.iter() {
        // before " ["
        let mut a = b"host ".to_vec();
        a.extend_from_slice(arch);
        a.extend_from_slice(b" [ubuntu: 22.04 (jammy)]");
        cases.push(Case::parse(&a));
        // after " [" only  -> invisible
        let mut b = b"host [ubuntu: 22.04 ".to_vec();
        b.extend_from_slice(arch);
        b.push(b']');
        cases.push(Case::parse(&b));
        // both sides -> the pre-" [" one is the only visible text
        for other in ARCHS.iter() {
            let mut d = b"h ".to_vec();
            d.extend_from_slice(arch);
            d.extend_from_slice(b" [o: 1.2 ");
            d.extend_from_slice(other);
            d.push(b']');
            cases.push(Case::parse(&d));
        }
    }
    assert_same(&c, &r, &cases, "cfg40 parse Unix arch position");
}

/// CONFIGS rows 43/44/45/46 — leftmost-occurrence semantics of every marker.
#[test]
fn cfg43_parse_unix_duplicate_markers() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"h [a: 1.2: 3.4]",
        b"h [a: b: 1.2]",
        b"h [a: 1.2 (x) (y)]",
        b"h [a: 1.2 (x (y))]",
        b"h [a|b|c: 1.2]",
        b"h [a: 1.2] [b: 3.4]",
        b"h [a [b: 1.2]",
        b"h [ [a: 1.2]",
        b"h [a: 1.2 [b: 3.4]]",
        b"h [a: (x) 1.2]",
        b"h [: : ]",
        b"h [ (: 1.2]",
        b"h [a: 1.2 (]",
        b"h [a: 1.2 )]",
        b"h [a: |1.2]",
    ];
    let cases: Vec<Case> = fixed.iter().map(|s| Case::parse(s)).collect();
    assert_same(&c, &r, &cases, "cfg43 parse duplicate markers");
}

/// CONFIGS rows 47/48/49/50 — neither branch, and near-miss markers.
#[test]
fn cfg47_parse_neither_branch() {
    let (c, r) = open_pair();
    let mut cases = Vec::new();
    for lit in [
        &b""[..],
        &b"plain text"[..],
        &b"a[b: 1]"[..],
        &b"a[Ver: 1.2.3]"[..],
        &b"[Ver: 1.2.3]"[..],
        &b"[a: 1]"[..],
        &b"a ["[..],
        &b"["[..],
        &b" "[..],
        &b"Linux host 5.15.0-generic x86_64"[..],
        &b"Darwin mac 21.6.0 arm64"[..],
        &b"AIX p7 1 7"[..],
        &b"SunOS sun 5.11 i86pc"[..],
    ] {
        cases.push(Case::parse(lit));
    }
    for arch in ARCHS.iter() {
        let mut s = b"no markers here ".to_vec();
        s.extend_from_slice(arch);
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "cfg47 parse neither branch");
}

/// CONFIGS row 51 — realistic Wazuh-style agent uname corpus.
#[test]
fn cfg51_parse_realistic_corpus() {
    let (c, r) = open_pair();
    let cases: Vec<Case> = real_unames().iter().map(|s| Case::parse(s)).collect();
    assert_same(&c, &r, &cases, "cfg51 parse realistic corpus");
}

/// CONFIGS row 52 — 64 KiB uname with markers at varying positions.
#[test]
fn cfg52_parse_huge_input() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 52);
    let mut cases = Vec::new();
    for _ in 0..6 {
        let filler = rng.word(64 * 1024);
        let mut a = filler.clone();
        a.extend_from_slice(b" [Ver: 10.0.19041.1234]");
        cases.push(Case::parse(&a));

        let mut b = b"host [".to_vec();
        b.extend_from_slice(&filler);
        b.extend_from_slice(b": 22.04 (jammy)]");
        cases.push(Case::parse(&b));

        let mut d = b"host [os: 22.04 (".to_vec();
        d.extend_from_slice(&filler);
        d.extend_from_slice(b")]");
        cases.push(Case::parse(&d));

        let mut e = b"h x86_64 [os: ".to_vec();
        e.extend_from_slice(&rng.version(3));
        e.push(b' ');
        e.extend_from_slice(&filler);
        e.push(b']');
        cases.push(Case::parse(&e));
    }
    assert_same(&c, &r, &cases, "cfg52 parse 64KiB input");
}

/// CONFIGS row 53 — high bytes / tabs / newlines inside every field.
#[test]
fn cfg53_parse_high_bytes() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 53);
    let mut cases = Vec::new();
    for _ in 0..(N * 2) {
        let mut s = rng.noise_between(0, 8);
        s.extend_from_slice(b" [");
        s.extend_from_slice(&rng.noise_between(0, 8));
        s.extend_from_slice(b": ");
        s.extend_from_slice(&rng.noise_between(0, 8));
        s.extend_from_slice(b" (");
        s.extend_from_slice(&rng.noise_between(0, 8));
        s.extend_from_slice(b")]");
        cases.push(Case::parse(&s));

        let mut t = rng.noise_between(0, 8);
        t.extend_from_slice(b" [Ver: ");
        t.extend_from_slice(&rng.noise_between(0, 8));
        t.push(b']');
        cases.push(Case::parse(&t));
    }
    assert_same(&c, &r, &cases, "cfg53 parse high bytes");
}

/// CONFIGS row 54 — prepopulated os_data: which fields get overwritten.
#[test]
fn cfg54_parse_prepopulated_osdata() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"W [Ver: 10.0.19041]",
        b"h [ubuntu|debian: 22.04 (jammy)] x86_64",
        b"h [ubuntu]",
        b"no markers x86_64",
        b"no markers at all",
        b"",
    ];
    let cases: Vec<Case> = fixed.iter().map(|s| Case::parse_prepop(s)).collect();
    assert_same(&c, &r, &cases, "cfg54 parse prepopulated os_data");
}

/// CONFIGS row 55 — repeated calls on the same (already mutated) buffer.
#[test]
fn cfg55_parse_repeated_calls() {
    let (c, r) = open_pair();
    let fixed: &[&[u8]] = &[
        b"W [Ver: 10.0.19041]",
        b"h [ubuntu|debian: 22.04 (jammy)] x86_64",
        b"h x86_64 [ubuntu: 22.04]",
        b"h [ubuntu]",
        b"a [b] c [Ver: 1.2.3]",
        b"plain x86_64",
    ];
    let mut cases = Vec::new();
    for s in fixed {
        for times in [2usize, 3, 4] {
            cases.push(Case::parse_n(s, times));
        }
    }
    assert_same(&c, &r, &cases, "cfg55 parse repeated calls");
}

/// CONFIGS row 56 — random byte soup with markers injected at random offsets.
#[test]
fn cfg56_parse_random_soup() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 56);
    let markers: &[&[u8]] = &[b" [Ver: ", b" [", b": ", b" (", b"|", b"]", b")"];
    let mut cases = Vec::new();
    for _ in 0..4000 {
        let mut s = rng.bytes_between(0, 96);
        let inject = rng.range(0, 5);
        for _ in 0..inject {
            let m: &[u8] = rng.pick(markers);
            let at = rng.below(s.len() + 1);
            let tail = s.split_off(at);
            s.extend_from_slice(m);
            s.extend_from_slice(&tail);
        }
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "cfg56 parse random soup");
}

/// CONFIGS row 56 (continued) — the same soup, but built only from the
/// characters the parser is sensitive to, so markers arise naturally.
#[test]
fn cfg56b_parse_marker_alphabet_soup() {
    let (c, r) = open_pair();
    let mut rng = Rng::new(SEED ^ 0x56b);
    const A: &[u8] = b" [](:|)Ver0123456789.";
    let mut cases = Vec::new();
    for _ in 0..4000 {
        let len = rng.range(0, 40);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(A)).collect();
        cases.push(Case::parse(&s));
    }
    assert_same(&c, &r, &cases, "cfg56b parse marker-alphabet soup");
}

// ===========================================================================
// Shared corpus
// ===========================================================================

fn real_unames() -> Vec<Vec<u8>> {
    let lits: &[&[u8]] = &[
        b"Linux |ubuntu-22 |5.15.0-76-generic |#83-Ubuntu SMP Thu Jun 15 19:16:32 UTC 2023 |x86_64 [Ubuntu|ubuntu: 22.04.2 LTS (Jammy Jellyfish)]",
        b"Linux |centos7 |3.10.0-1160.el7.x86_64 |#1 SMP Mon Oct 19 16:18:59 UTC 2020 |x86_64 [CentOS Linux|centos: 7.9.2009 (Core)]",
        b"Linux |debian11 |5.10.0-23-amd64 |#1 SMP Debian 5.10.179-1 |x86_64 [Debian GNU/Linux|debian: 11 (bullseye)]",
        b"Linux |amzn2 |5.10.184-175.731.amzn2.x86_64 |#1 SMP |x86_64 [Amazon Linux|amzn: 2]",
        b"Linux |alpine |5.15.0 |#1 SMP |x86_64 [Alpine Linux|alpine: 3.18.2]",
        b"Linux |rhel9 |5.14.0-284.el9.aarch64 |#1 SMP |aarch64 [Red Hat Enterprise Linux|rhel: 9.2 (Plow)]",
        b"Linux |sles |5.14.21-150400 |#1 SMP |x86_64 [SLES|sles: 15-SP4]",
        b"Linux |arch |6.4.1-arch1-1 |#1 SMP PREEMPT_DYNAMIC |x86_64 [Arch Linux|arch: rolling]",
        b"Darwin |mac.local |21.6.0 |Darwin Kernel Version 21.6.0 |arm64 [Mac OS X|darwin: 12.5 (Monterey)]",
        b"Darwin |mac.local |19.6.0 |Darwin Kernel Version 19.6.0 |x86_64 [Mac OS X|darwin: 10.15.7]",
        b"AIX |aixhost |1 |7 |00F84C0C4C00 [AIX|aix: 7.2]",
        b"SunOS |solaris |5.11 |11.4 |i86pc [SunOS|sunos: 11.4]",
        b"HP-UX |hpux |B.11.31 |U |ia64 [HP-UX|hpux: 11.31]",
        b"FreeBSD |bsd |13.2-RELEASE |FreeBSD 13.2 |amd64 [FreeBSD|freebsd: 13.2]",
        b"Microsoft Windows 10 Pro [Ver: 10.0.19045.3086]",
        b"Microsoft Windows Server 2019 Datacenter [Ver: 10.0.17763.4377]",
        b"Microsoft Windows 7 Professional [Ver: 6.1.7601]",
        b"Microsoft Windows 11 Enterprise [Ver: 10.0.22621.1848]",
        b"Microsoft Windows XP [Ver: 5.1.2600]",
        b"Linux |raspi |6.1.21-v8+ |#1 SMP PREEMPT |armv7l [Raspbian GNU/Linux|raspbian: 11 (bullseye)]",
        b"Linux |armhost |4.9.0 |#1 |armv6l [Debian|debian: 9 (stretch)]",
        b"Linux |sparcbox |5.10.0 |#1 |sparc64 [Debian|debian: 11 (bullseye)]",
        b"Linux |i386box |4.9.0-13-686 |#1 SMP Debian |i686 [Debian GNU/Linux|debian: 9 (stretch)]",
        b"Linux |host |5.15.0 |#1 |x86_64 [Ubuntu: 22.04 (jammy)]",
        b"Linux |host |5.15.0 |#1 |x86_64 [Ubuntu]",
        b"Linux host 5.15.0-76-generic #83-Ubuntu SMP x86_64 GNU/Linux",
    ];
    lits.iter().map(|s| s.to_vec()).collect()
}
