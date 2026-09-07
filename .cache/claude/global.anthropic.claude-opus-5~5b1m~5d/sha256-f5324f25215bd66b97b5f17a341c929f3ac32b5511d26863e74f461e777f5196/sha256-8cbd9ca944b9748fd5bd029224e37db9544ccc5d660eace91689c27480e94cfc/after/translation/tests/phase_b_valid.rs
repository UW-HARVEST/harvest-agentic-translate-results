//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Both implementations are driven exclusively through their `.so` exports.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// get_os_arch — rows 1-7
// ===========================================================================

#[test]
fn row01_get_os_arch_each_arch_exactly() {
    for (i, a) in ARCHS.iter().enumerate() {
        assert_arch_eq(&format!("row01/arch{i}"), a);
    }
}

#[test]
fn row02_get_os_arch_arch_at_random_position() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..4000 {
        let arch = *rng.pick(&ARCHS);
        let pre = rng.safe_word(0, 24);
        let post = rng.safe_word(0, 24);
        let mut s = Vec::new();
        s.extend_from_slice(&pre);
        s.extend_from_slice(arch);
        s.extend_from_slice(&post);
        assert_arch_eq(&format!("row02/#{i}"), &s);
    }
}

#[test]
fn row03_get_os_arch_multiple_archs_priority() {
    // Exhaustive over ordered pairs, plus randomized triples.
    for (i, a) in ARCHS.iter().enumerate() {
        for (j, b) in ARCHS.iter().enumerate() {
            let mut s = Vec::new();
            s.extend_from_slice(a);
            s.push(b' ');
            s.extend_from_slice(b);
            assert_arch_eq(&format!("row03/pair{i}-{j}"), &s);
        }
    }
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..3000 {
        let n = rng.range(2, 5);
        let mut s = rng.safe_word(0, 6);
        for _ in 0..n {
            s.extend_from_slice(rng.pick(&ARCHS));
            s.extend_from_slice(&rng.safe_word(0, 6));
        }
        assert_arch_eq(&format!("row03/multi{i}"), &s);
    }
}

#[test]
fn row04_get_os_arch_no_arch_present() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..5000 {
        let s = rng.safe_word(0, 60);
        assert!(!contains_any_arch(&s));
        assert_arch_eq(&format!("row04/#{i}"), &s);
    }
}

#[test]
fn row05_get_os_arch_tiny_inputs() {
    assert_arch_eq("row05/empty", b"");
    for c in 0x20u8..0x7f {
        assert_arch_eq(&format!("row05/char{c}"), &[c]);
    }
    // Every 2-char ASCII pair (shorter than the shortest arch, "AIX" = 3).
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..2000 {
        let a = rng.range(0x20, 0x7e) as u8;
        let b = rng.range(0x20, 0x7e) as u8;
        assert_arch_eq(&format!("row05/pair{i}"), &[a, b]);
    }
}

#[test]
fn row06_get_os_arch_near_miss_tokens() {
    let near: &[&[u8]] = &[
        b"i86", b"arm", b"aarch", b"x86", b"64", b"aix", b"Aix", b"aIX", b"AIx", b"x86_6",
        b"86_64", b"i38", b"386", b"i68", b"686", b"spar", b"parc", b"amd", b"md64", b"i86p",
        b"86pc", b"ia6", b"a64", b"armv", b"rmv6", b"rmv7", b"armv5", b"armv8", b"arm6",
        b"aarch6", b"arch64", b"ARM64", b"X86_64", b"AMD64", b"SPARC",
    ];
    for (i, t) in near.iter().enumerate() {
        assert_arch_eq(&format!("row06/#{i}"), t);
    }
}

#[test]
fn row07_get_os_arch_very_long_input() {
    let mut rng = Rng::new(SEED ^ 7);
    for (i, arch) in ARCHS.iter().enumerate() {
        let mut s = vec![b'z'; 4096];
        s.extend_from_slice(arch);
        assert_arch_eq(&format!("row07/tail{i}"), &s);

        let mut s2 = Vec::new();
        s2.extend_from_slice(arch);
        s2.extend(std::iter::repeat(b'q').take(4096));
        assert_arch_eq(&format!("row07/head{i}"), &s2);
    }
    for i in 0..200 {
        let mut s = rng.safe_word(0, 8);
        while s.len() < 3000 {
            s.extend_from_slice(&rng.safe_word(1, 40));
        }
        s.extend_from_slice(rng.pick(&ARCHS));
        s.extend_from_slice(&rng.safe_word(0, 40));
        assert_arch_eq(&format!("row07/rand{i}"), &s);
    }
}

// ===========================================================================
// w_regexec — rows 8-17
// ===========================================================================

fn random_version(rng: &mut Rng) -> Vec<u8> {
    let comps = rng.range(0, 5);
    let mut v = Vec::new();
    for k in 0..comps {
        if k > 0 {
            v.push(b'.');
        }
        v.extend_from_slice(&rng.digits(1, 4));
    }
    // Sometimes append trailing junk / extra dots, matching real uname shapes.
    match rng.below(6) {
        0 => v.extend_from_slice(b"..."),
        1 => v.extend_from_slice(b"LTS"),
        2 => v.extend_from_slice(b"-rc1"),
        3 => v.push(b'.'),
        _ => {}
    }
    if rng.below(8) == 0 {
        // leading non-digit
        let mut w = rng.safe_word(1, 3);
        w.extend_from_slice(&v);
        return w;
    }
    v
}

#[test]
fn row08_w_regexec_internal_patterns_on_versions() {
    let mut rng = Rng::new(SEED ^ 8);
    let pats: &[&[u8]] = &[PAT_MAJOR, PAT_MINOR, PAT_BUILD];
    for i in 0..8000 {
        let subject = random_version(&mut rng);
        for (pi, p) in pats.iter().enumerate() {
            assert_regexec_eq(
                &format!("row08/#{i}/p{pi}"),
                Some(p),
                Some(&subject),
                2,
                &pm_init(2),
            );
        }
    }
    // Hand-picked shapes too.
    let fixed: &[&[u8]] = &[
        b"10.0.19041.1234",
        b"10.0.19041",
        b"10.0",
        b"10",
        b"0.0.0",
        b"6.1.7601.24545",
        b"0007.0000.99999999999",
        b"1.2.3.4.5.6.7.8.9",
        b"20.04",
        b"20.04LTS",
        b"",
        b".",
        b"..",
        b"1..",
        b"1.2..",
        b"1.2.3..",
        b"abc",
        b"a1.2.3",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for (si, s) in fixed.iter().enumerate() {
            assert_regexec_eq(
                &format!("row08/fixed{si}/p{pi}"),
                Some(p),
                Some(s),
                2,
                &pm_init(2),
            );
        }
    }
}

#[test]
fn row09_w_regexec_zero_groups_nmatch2() {
    let mut rng = Rng::new(SEED ^ 9);
    let pats: &[&[u8]] = &[b"^[0-9]+", b"abc", b"^.*$", b"[a-z]+", b"^$"];
    for (pi, p) in pats.iter().enumerate() {
        for i in 0..1000 {
            let s = rng.word(0, 20);
            assert_regexec_eq(
                &format!("row09/p{pi}/#{i}"),
                Some(p),
                Some(&s),
                2,
                &pm_init(2),
            );
        }
    }
}

#[test]
fn row10_w_regexec_one_group_nmatch1() {
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..2000 {
        let s = random_version(&mut rng);
        assert_regexec_eq(
            &format!("row10/#{i}"),
            Some(PAT_MAJOR),
            Some(&s),
            1,
            &pm_init(2),
        );
    }
}

#[test]
fn row11_w_regexec_nmatch_zero() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..2000 {
        let s = random_version(&mut rng);
        assert_regexec_eq(
            &format!("row11/#{i}"),
            Some(PAT_MAJOR),
            Some(&s),
            0,
            &pm_init(2),
        );
        assert_regexec_eq(
            &format!("row11/build#{i}"),
            Some(PAT_BUILD),
            Some(&s),
            0,
            &pm_init(4),
        );
    }
}

#[test]
fn row12_w_regexec_nmatch_larger_than_groups() {
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..2000 {
        let s = random_version(&mut rng);
        assert_regexec_eq(
            &format!("row12/#{i}"),
            Some(PAT_BUILD),
            Some(&s),
            8,
            &pm_init(8),
        );
        assert_regexec_eq(
            &format!("row12/major#{i}"),
            Some(PAT_MAJOR),
            Some(&s),
            8,
            &pm_init(8),
        );
        assert_regexec_eq(
            &format!("row12/32#{i}"),
            Some(PAT_BUILD),
            Some(&s),
            32,
            &pm_init(32),
        );
    }
}

#[test]
fn row13_w_regexec_non_participating_group() {
    let mut rng = Rng::new(SEED ^ 13);
    let pats: &[&[u8]] = &[
        PAT_BUILD,
        br"^(a)|(b)$",
        br"(x)?(y)",
        br"([0-9]+)(\.[0-9]+)?",
        br"^(foo)|([0-9]+)$",
    ];
    let subs: &[&[u8]] = &[
        b"1.2.3", b"1.2.3.4", b"a", b"b", b"ab", b"y", b"xy", b"5", b"5.6", b"foo", b"77",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for (si, s) in subs.iter().enumerate() {
            for &nm in &[0usize, 1, 2, 3, 4, 8] {
                assert_regexec_eq(
                    &format!("row13/p{pi}/s{si}/n{nm}"),
                    Some(p),
                    Some(s),
                    nm,
                    &pm_init(8),
                );
            }
        }
    }
    for i in 0..1500 {
        let s = random_version(&mut rng);
        assert_regexec_eq(
            &format!("row13/rand{i}"),
            Some(PAT_BUILD),
            Some(&s),
            3,
            &pm_init(4),
        );
    }
}

#[test]
fn row14_w_regexec_unanchored_nonzero_offset() {
    let mut rng = Rng::new(SEED ^ 14);
    let pats: &[&[u8]] = &[br"([0-9]+)", br"(v[0-9]+\.[0-9]+)", br"([A-Z]+)", br"(z+)"];
    for i in 0..3000 {
        let pre = rng.word(0, 30);
        let mid = rng.pick(&[
            b"v1.2".to_vec(),
            b"12345".to_vec(),
            b"ABC".to_vec(),
            b"zzz".to_vec(),
        ])
        .clone();
        let post = rng.word(0, 30);
        let mut s = pre;
        s.extend_from_slice(&mid);
        s.extend_from_slice(&post);
        for (pi, p) in pats.iter().enumerate() {
            assert_regexec_eq(
                &format!("row14/#{i}/p{pi}"),
                Some(p),
                Some(&s),
                2,
                &pm_init(2),
            );
        }
    }
}

#[test]
fn row15_w_regexec_empty_subject() {
    let pats: &[&[u8]] = &[
        br"^.*", br"^a", br"^$", br"", br"()", br"(.*)", br"^([0-9]+)\.*", PAT_MAJOR, PAT_MINOR,
        PAT_BUILD,
    ];
    for (pi, p) in pats.iter().enumerate() {
        for &nm in &[0usize, 1, 2, 4] {
            assert_regexec_eq(
                &format!("row15/p{pi}/n{nm}"),
                Some(p),
                Some(b""),
                nm,
                &pm_init(4),
            );
        }
    }
}

#[test]
fn row16_w_regexec_ere_metacharacters() {
    let pats: &[&[u8]] = &[
        br"a+", br"a*", br"a?", br"a|b", br"(ab)+", br"[abc]", br"[^abc]", br"a{2}", br"a{2,}",
        br"a{2,4}", br".", br"^a", br"a$", br"\.", br"\\", br"[[:digit:]]+", br"[[:alpha:]]+",
        br"[[:space:]]", br"()", br"(a)(b)(c)", br"((a))", br"a|", br"|a", br"[-a]", br"[a-]",
        br"[]a]", br"x{0,0}", br"(a|b)*c", br"^(([0-9]+)\.)*([0-9]+)$",
    ];
    let mut rng = Rng::new(SEED ^ 16);
    for (pi, p) in pats.iter().enumerate() {
        for i in 0..300 {
            let s = rng.word(0, 24);
            assert_regexec_eq(
                &format!("row16/p{pi}/#{i}"),
                Some(p),
                Some(&s),
                4,
                &pm_init(4),
            );
        }
        for s in [
            &b""[..],
            b"a",
            b"aa",
            b"aaaa",
            b"ab",
            b"abc",
            b"c",
            b"1.2.3",
            b"\\",
            b".",
            b" ",
        ] {
            assert_regexec_eq(
                &format!("row16/p{pi}/fixed"),
                Some(p),
                Some(s),
                4,
                &pm_init(4),
            );
        }
    }
}

#[test]
fn row17_w_regexec_long_subject() {
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..200 {
        let mut s = vec![b'a'; 4096];
        s.extend_from_slice(b"NEEDLE42");
        s.extend_from_slice(&rng.word(0, 20));
        for (pi, p) in [&br"(NEEDLE)([0-9]+)"[..], br"([0-9]+)", br"^(a+)"]
            .iter()
            .enumerate()
        {
            assert_regexec_eq(
                &format!("row17/#{i}/p{pi}"),
                Some(p),
                Some(&s),
                4,
                &pm_init(4),
            );
        }
    }
}

// ===========================================================================
// parse_uname_string — Windows branch, rows 18-27
// ===========================================================================

#[test]
fn row18_windows_full_version() {
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..6000 {
        let name = rng.safe_word(0, 20);
        let maj = rng.digits(1, 4);
        let min = rng.digits(1, 4);
        let build = rng.digits(1, 6);
        let mut u = name;
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&maj);
        u.push(b'.');
        u.extend_from_slice(&min);
        u.push(b'.');
        u.extend_from_slice(&build);
        u.push(b']');
        assert_parse_both_inits(&format!("row18/#{i}"), &u);
    }
    for u in [
        &b"Microsoft Windows 10 [Ver: 10.0.19041]"[..],
        b"Microsoft Windows Server 2019 [Ver: 10.0.17763]",
        b"Windows [Ver: 6.1.7601]",
    ] {
        assert_parse_both_inits("row18/fixed", u);
    }
}

#[test]
fn row19_windows_four_plus_components() {
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..4000 {
        let n = rng.range(4, 8);
        let mut ver = Vec::new();
        for k in 0..n {
            if k > 0 {
                ver.push(b'.');
            }
            ver.extend_from_slice(&rng.digits(1, 5));
        }
        let mut u = rng.safe_word(0, 12);
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&ver);
        u.push(b']');
        assert_parse_both_inits(&format!("row19/#{i}"), &u);
    }
    assert_parse_both_inits("row19/fixed", b"W [Ver: 10.0.19041.1234]");
    assert_parse_both_inits("row19/fixed2", b"W [Ver: 1.2.3.4.5.6.7.8.9]");
}

#[test]
fn row20_windows_two_components() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 14);
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&rng.digits(1, 5));
        u.push(b'.');
        u.extend_from_slice(&rng.digits(1, 5));
        u.push(b']');
        assert_parse_both_inits(&format!("row20/#{i}"), &u);
    }
    assert_parse_both_inits("row20/fixed", b"W [Ver: 6.1]");
}

#[test]
fn row21_windows_one_component() {
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 14);
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&rng.digits(1, 6));
        u.push(b']');
        assert_parse_both_inits(&format!("row21/#{i}"), &u);
    }
    assert_parse_both_inits("row21/fixed", b"W [Ver: 10]");
}

#[test]
fn row22_windows_branch_ignores_arch() {
    let mut rng = Rng::new(SEED ^ 22);
    for (ai, arch) in ARCHS.iter().enumerate() {
        let mut u = Vec::from(&b"Windows "[..]);
        u.extend_from_slice(arch);
        u.extend_from_slice(b" [Ver: 10.0.1234]");
        assert_parse_both_inits(&format!("row22/pre{ai}"), &u);

        // arch inside the version part as well
        let mut u2 = Vec::from(&b"Windows [Ver: 10.0."[..]);
        u2.extend_from_slice(arch);
        u2.push(b']');
        assert_parse_both_inits(&format!("row22/in{ai}"), &u2);
    }
    for i in 0..2000 {
        let mut u = rng.safe_word(0, 8);
        u.extend_from_slice(rng.pick(&ARCHS));
        u.extend_from_slice(&rng.safe_word(0, 8));
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&random_version(&mut rng));
        u.push(b']');
        assert_parse_both_inits(&format!("row22/rand{i}"), &u);
    }
}

#[test]
fn row23_windows_branch_ignores_pipe_and_paren() {
    let cases: &[&[u8]] = &[
        b"Windows|windows [Ver: 10.0.1]",
        b"Windows (Server) [Ver: 10.0.1]",
        b"Win|a|b (x) [Ver: 6.1.2]",
        b"W [Ver: 10.0.1 (build)]",
        b"W [Ver: 10.0.1|x]",
        b"|[Ver: 1.2.3]",
        b" |x [Ver: 1.2.3]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row23/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..2000 {
        let mut u = rng.safe_word(0, 8);
        if rng.bool() {
            u.push(b'|');
            u.extend_from_slice(&rng.safe_word(0, 8));
        }
        if rng.bool() {
            u.extend_from_slice(b" (");
            u.extend_from_slice(&rng.safe_word(0, 8));
            u.push(b')');
        }
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&random_version(&mut rng));
        u.push(b']');
        assert_parse_both_inits(&format!("row23/rand{i}"), &u);
    }
}

#[test]
fn row24_ver_wins_over_plain_bracket() {
    let cases: &[&[u8]] = &[
        b"N [Ver: 10.0.1] [ubuntu: 20.04 (focal)]",
        b"N [ubuntu: 20.04] [Ver: 10.0.1]",
        b"N [x] [Ver: 1.2]",
        b"N [Ver: 1.2] [x]",
        b"N [a: b (c)] more [Ver: 6.1.7601]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row24/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2000 {
        let a = rng.safe_word(0, 10);
        let b = rng.safe_word(0, 10);
        let ver = random_version(&mut rng);
        let mut u = Vec::new();
        if rng.bool() {
            u.extend_from_slice(&a);
            u.extend_from_slice(b" [Ver: ");
            u.extend_from_slice(&ver);
            u.extend_from_slice(b"] [");
            u.extend_from_slice(&b);
            u.push(b']');
        } else {
            u.extend_from_slice(&a);
            u.extend_from_slice(b" [");
            u.extend_from_slice(&b);
            u.extend_from_slice(b"] [Ver: ");
            u.extend_from_slice(&ver);
            u.push(b']');
        }
        assert_parse_both_inits(&format!("row24/rand{i}"), &u);
    }
}

#[test]
fn row25_windows_multiple_ver_markers() {
    let cases: &[&[u8]] = &[
        b"N [Ver: 1.2.3] [Ver: 4.5.6]",
        b"N [Ver: [Ver: 1.2.3]]",
        b"N [Ver:  [Ver: 9.9.9]",
        b" [Ver:  [Ver: ",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row25/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 25);
    for i in 0..1500 {
        let mut u = rng.safe_word(0, 6);
        let n = rng.range(2, 4);
        for _ in 0..n {
            u.extend_from_slice(b" [Ver: ");
            u.extend_from_slice(&random_version(&mut rng));
            u.push(b']');
        }
        assert_parse_both_inits(&format!("row25/rand{i}"), &u);
    }
}

#[test]
fn row26_windows_leading_zeros_and_huge_numbers() {
    let cases: &[&[u8]] = &[
        b"W [Ver: 0007.0000.99999999999]",
        b"W [Ver: 000.000.000]",
        b"W [Ver: 0.0.0]",
        b"W [Ver: 99999999999999999999.1.2]",
        b"W [Ver: 1.99999999999999999999.2]",
        b"W [Ver: 00000000000000000001]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row26/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..2500 {
        let mut u = Vec::from(&b"W [Ver: "[..]);
        for k in 0..rng.range(1, 4) {
            if k > 0 {
                u.push(b'.');
            }
            if rng.bool() {
                u.extend_from_slice(b"0000");
            }
            u.extend_from_slice(&rng.digits(1, 25));
        }
        u.push(b']');
        assert_parse_both_inits(&format!("row26/rand{i}"), &u);
    }
}

#[test]
fn row27_windows_trailing_char_not_bracket() {
    let cases: &[&[u8]] = &[
        b"W [Ver: 10.0.1",
        b"W [Ver: 10.0.19",
        b"W [Ver: 10.0.1)",
        b"W [Ver: 10.0.1 ",
        b"W [Ver: 10.0.1]]",
        b"W [Ver: x",
        b"W [Ver: 1",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row27/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 27);
    for i in 0..2500 {
        let mut u = rng.safe_word(0, 8);
        u.extend_from_slice(b" [Ver: ");
        u.extend_from_slice(&random_version(&mut rng));
        // Random (possibly absent) terminator that is not necessarily ']'.
        match rng.below(5) {
            0 => u.push(b']'),
            1 => u.push(b')'),
            2 => u.push(b' '),
            3 => u.extend_from_slice(&rng.digits(1, 1)),
            _ => {}
        }
        assert_parse_both_inits(&format!("row27/rand{i}"), &u);
    }
}

// ===========================================================================
// parse_uname_string — Unix branch, rows 28-39
// ===========================================================================

fn unix_uname(rng: &mut Rng, with_colon: bool, with_paren: bool, with_pipe: bool) -> Vec<u8> {
    let mut u = rng.safe_word(0, 16);
    u.extend_from_slice(b" [");
    u.extend_from_slice(&rng.safe_word(0, 12));
    if with_pipe {
        u.push(b'|');
        u.extend_from_slice(&rng.safe_word(0, 10));
    }
    if with_colon {
        u.extend_from_slice(b": ");
        let comps = rng.range(1, 3);
        for k in 0..comps {
            if k > 0 {
                u.push(b'.');
            }
            u.extend_from_slice(&rng.digits(1, 4));
        }
        if with_paren {
            u.extend_from_slice(b" (");
            u.extend_from_slice(&rng.safe_word(0, 10));
            u.push(b')');
        }
    }
    u.push(b']');
    u
}

#[test]
fn row28_unix_full_form() {
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..6000 {
        let u = unix_uname(&mut rng, true, true, false);
        assert_parse_both_inits(&format!("row28/#{i}"), &u);
    }
    for u in [
        &b"Linux host 5.4.0 x86_64 [Ubuntu: 20.04 (focal)]"[..],
        b"Linux [Debian GNU/Linux: 11.2 (bullseye)]",
        b"Linux [CentOS Linux: 7.9 (Core)]",
    ] {
        assert_parse_both_inits("row28/fixed", u);
    }
}

#[test]
fn row29_unix_no_codename() {
    let mut rng = Rng::new(SEED ^ 29);
    for i in 0..5000 {
        let u = unix_uname(&mut rng, true, false, false);
        assert_parse_both_inits(&format!("row29/#{i}"), &u);
    }
    assert_parse_both_inits("row29/fixed", b"Linux [ubuntu: 20.04]");
}

#[test]
fn row30_unix_no_colon() {
    let mut rng = Rng::new(SEED ^ 30);
    for i in 0..5000 {
        let u = unix_uname(&mut rng, false, false, false);
        assert_parse_both_inits(&format!("row30/#{i}"), &u);
    }
    for u in [
        &b"Linux [ubuntu]"[..],
        b"Linux [x]",
        b"Linux [ab]",
        b"a [b]",
        b"Linux []",
    ] {
        assert_parse_both_inits("row30/fixed", u);
    }
}

#[test]
fn row31_unix_pipe_platform_with_colon() {
    let mut rng = Rng::new(SEED ^ 31);
    for i in 0..5000 {
        let pn = rng.bool();
        let u = unix_uname(&mut rng, true, pn, true);
        assert_parse_both_inits(&format!("row31/#{i}"), &u);
    }
    for u in [
        &b"Linux [Ubuntu|ubuntu: 20.04 (focal)]"[..],
        b"Linux [Debian|debian: 11.2]",
        b"L [n|p: 1.2 (c)]",
    ] {
        assert_parse_both_inits("row31/fixed", u);
    }
}

#[test]
fn row32_unix_pipe_platform_no_colon() {
    let mut rng = Rng::new(SEED ^ 32);
    for i in 0..5000 {
        let u = unix_uname(&mut rng, false, false, true);
        assert_parse_both_inits(&format!("row32/#{i}"), &u);
    }
    for u in [
        &b"Linux [Ubuntu|ubuntu]"[..],
        b"L [n|p]",
        b"L [|p]",
        b"L [n|]",
        b"L [|]",
    ] {
        assert_parse_both_inits("row32/fixed", u);
    }
}

#[test]
fn row33_unix_pipe_only_in_version() {
    let cases: &[&[u8]] = &[
        b"Linux [ubuntu: 20.04|x]",
        b"Linux [ubuntu: 20.04 (focal|f)]",
        b"L [n: 1.2|p]",
        b"L [n: |p]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row33/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 33);
    for i in 0..2500 {
        let mut u = rng.safe_word(0, 10);
        u.extend_from_slice(b" [");
        u.extend_from_slice(&rng.safe_word(1, 10));
        u.extend_from_slice(b": ");
        u.extend_from_slice(&rng.digits(1, 3));
        u.push(b'.');
        u.extend_from_slice(&rng.digits(1, 3));
        u.push(b'|');
        u.extend_from_slice(&rng.safe_word(0, 8));
        u.push(b']');
        assert_parse_both_inits(&format!("row33/rand{i}"), &u);
    }
}

#[test]
fn row34_unix_arch_before_bracket() {
    let mut rng = Rng::new(SEED ^ 34);
    for (ai, arch) in ARCHS.iter().enumerate() {
        let mut u = Vec::from(&b"Linux host "[..]);
        u.extend_from_slice(arch);
        u.extend_from_slice(b" [ubuntu: 20.04 (focal)]");
        assert_parse_both_inits(&format!("row34/arch{ai}"), &u);
    }
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 10);
        u.extend_from_slice(rng.pick(&ARCHS));
        u.extend_from_slice(&rng.safe_word(0, 10));
        let (b1, b2, b3) = (rng.bool(), rng.bool(), rng.bool());
        let tail = unix_uname(&mut rng, b1, b2, b3);
        u.extend_from_slice(&tail);
        assert_parse_both_inits(&format!("row34/rand{i}"), &u);
    }
}

#[test]
fn row35_unix_arch_only_after_bracket() {
    let mut rng = Rng::new(SEED ^ 35);
    for (ai, arch) in ARCHS.iter().enumerate() {
        let mut u = Vec::from(&b"Linux [name-"[..]);
        u.extend_from_slice(arch);
        u.extend_from_slice(b": 1.2 (c)]");
        assert_parse_both_inits(&format!("row35/arch{ai}"), &u);
    }
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 10);
        u.extend_from_slice(b" [");
        u.extend_from_slice(&rng.safe_word(0, 6));
        u.extend_from_slice(rng.pick(&ARCHS));
        u.extend_from_slice(&rng.safe_word(0, 6));
        u.extend_from_slice(b": ");
        u.extend_from_slice(&rng.digits(1, 3));
        u.push(b']');
        assert_parse_both_inits(&format!("row35/rand{i}"), &u);
    }
}

#[test]
fn row36_unix_repeated_separators_first_wins() {
    let cases: &[&[u8]] = &[
        b"a [b] [c]",
        b"a [b: 1] [c: 2]",
        b"a [b: 1: 2]",
        b"a [b: 1 (c) (d)]",
        b"a [b|c|d: 1.2]",
        b"a [b: 1.2 (c (d))]",
        b" [ [ [",
        b"a [b: : ]",
        b"a [b:  (: ]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row36/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 36);
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 6);
        for _ in 0..rng.range(1, 3) {
            u.extend_from_slice(b" [");
            u.extend_from_slice(&rng.safe_word(0, 6));
            for _ in 0..rng.range(0, 2) {
                u.extend_from_slice(b": ");
                u.extend_from_slice(&rng.digits(1, 3));
            }
            for _ in 0..rng.range(0, 2) {
                u.extend_from_slice(b" (");
                u.extend_from_slice(&rng.safe_word(0, 5));
                u.push(b')');
            }
            u.push(b']');
        }
        assert_parse_both_inits(&format!("row36/rand{i}"), &u);
    }
}

#[test]
fn row37_unix_codename_with_parens_and_spaces() {
    let cases: &[&[u8]] = &[
        b"L [n: 1.2 (a b c)]",
        b"L [n: 1.2 ((x))]",
        b"L [n: 1.2 (x)]extra",
        b"L [n: 1.2 (]",
        b"L [n: 1.2 ()]",
        b"L [n: 1.2 (  )]",
        b"L [n: 1.2 (Core) ]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row37/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 37);
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 8);
        u.extend_from_slice(b" [");
        u.extend_from_slice(&rng.safe_word(1, 8));
        u.extend_from_slice(b": ");
        u.extend_from_slice(&rng.digits(1, 3));
        u.push(b'.');
        u.extend_from_slice(&rng.digits(1, 3));
        u.extend_from_slice(b" (");
        let cn = rng.word(0, 12); // may itself contain '(' ')' spaces
        u.extend_from_slice(&cn);
        u.push(b')');
        u.push(b']');
        assert_parse_both_inits(&format!("row37/rand{i}"), &u);
    }
}

#[test]
fn row38_unix_three_component_version() {
    let mut rng = Rng::new(SEED ^ 38);
    for i in 0..4000 {
        let mut u = rng.safe_word(0, 10);
        u.extend_from_slice(b" [");
        u.extend_from_slice(&rng.safe_word(1, 8));
        u.extend_from_slice(b": ");
        for k in 0..rng.range(3, 6) {
            if k > 0 {
                u.push(b'.');
            }
            u.extend_from_slice(&rng.digits(1, 4));
        }
        if rng.bool() {
            u.extend_from_slice(b" (");
            u.extend_from_slice(&rng.safe_word(0, 8));
            u.push(b')');
        }
        u.push(b']');
        assert_parse_both_inits(&format!("row38/#{i}"), &u);
    }
    assert_parse_both_inits("row38/fixed", b"L [n: 1.2.3]");
    assert_parse_both_inits("row38/fixed2", b"L [n: 5.14.0-70 (x)]");
}

#[test]
fn row39_unix_version_digits_then_letters() {
    let cases: &[&[u8]] = &[
        b"L [n: 20.04LTS]",
        b"L [n: 7Server]",
        b"L [n: 1.2rc3]",
        b"L [n: 1a.2b]",
        b"L [n: notaversion]",
        b"L [n: .1.2]",
        b"L [n: v1.2]",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_both_inits(&format!("row39/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 39);
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 8);
        u.extend_from_slice(b" [");
        u.extend_from_slice(&rng.safe_word(1, 8));
        u.extend_from_slice(b": ");
        u.extend_from_slice(&random_version(&mut rng));
        u.push(b']');
        assert_parse_both_inits(&format!("row39/rand{i}"), &u);
    }
}

// ===========================================================================
// parse_uname_string — no-separator, sentinels, buffers, fuzz — rows 40-48
// ===========================================================================

#[test]
fn row40_no_separator_with_arch() {
    let mut rng = Rng::new(SEED ^ 40);
    for (ai, arch) in ARCHS.iter().enumerate() {
        let mut u = Vec::from(&b"Linux host 5.4.0 "[..]);
        u.extend_from_slice(arch);
        assert_parse_both_inits(&format!("row40/arch{ai}"), &u);
    }
    for i in 0..3000 {
        let mut u = rng.safe_word(0, 20);
        u.extend_from_slice(rng.pick(&ARCHS));
        u.extend_from_slice(&rng.safe_word(0, 20));
        assert_parse_both_inits(&format!("row40/rand{i}"), &u);
    }
}

#[test]
fn row41_no_separator_no_arch() {
    let mut rng = Rng::new(SEED ^ 41);
    for i in 0..4000 {
        let u = rng.safe_word(0, 50);
        assert_parse_both_inits(&format!("row41/#{i}"), &u);
    }
    for u in [
        &b"Linux"[..],
        b"Linux ppc64le 5.4",
        b"[",
        b"[Ver: 1.2.3]",
        b"Ver: 1.2.3",
        b"(",
        b"|",
        b": ",
    ] {
        assert_parse_both_inits("row41/fixed", u);
    }
}

#[test]
fn row42_sentinel_fields_preserved() {
    // Every branch, checked with a fully pre-populated os_data.
    let cases: &[&[u8]] = &[
        b"W [Ver: 10.0.19041]",
        b"W [Ver: abc]",
        b"L [n: 1.2 (c)]",
        b"L [n: 1.2]",
        b"L [n]",
        b"L [n|p: 1.2]",
        b"L x86_64",
        b"nothing here",
        b"",
    ];
    for (i, u) in cases.iter().enumerate() {
        assert_parse_sent_eq(&format!("row42/#{i}"), u);
    }
    let mut rng = Rng::new(SEED ^ 42);
    for i in 0..4000 {
        let u = fuzz_uname(&mut rng);
        assert_parse_sent_eq(&format!("row42/rand{i}"), &u);
    }
}

#[test]
fn row43_os_uname_never_written() {
    // Proven by comparing against C with sentinels; additionally assert the
    // *C* behaviour explicitly so the invariant is documented, then require
    // Rust to agree.
    let sc = Sentinels::new();
    let sr = Sentinels::new();
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 43);
    for i in 0..3000 {
        let u = fuzz_uname(&mut rng);
        let a = c.parse_sent(&u, &sc);
        let b = r.parse_sent(&u, &sr);
        assert_eq!(
            a.snapshot.fields[7],
            Some(b"<<untouched-7>>".to_vec()),
            "row43/#{i}: C unexpectedly wrote os_uname for {:?}",
            String::from_utf8_lossy(&u)
        );
        assert_eq!(
            a, b,
            "row43/#{i}: diverged for {:?}",
            String::from_utf8_lossy(&u)
        );
    }
}

#[test]
fn row44_uname_buffer_mutation_byte_for_byte() {
    // assert_parse_eq already compares the full guarded buffer; this test
    // focuses on inputs whose in-place truncation is the interesting part and
    // asserts the guard bytes stayed intact identically in both.
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 44);
    for i in 0..6000 {
        let u = fuzz_uname(&mut rng);
        let a = c.parse(&u);
        let b = r.parse(&u);
        assert_eq!(
            a.buffer,
            b.buffer,
            "row44/#{i}: uname buffer diverged for {:?}\n C   ={:?}\n Rust={:?}",
            String::from_utf8_lossy(&u),
            a.buffer,
            b.buffer
        );
        assert_eq!(
            a.snapshot, b.snapshot,
            "row44/#{i}: fields diverged for {:?}",
            String::from_utf8_lossy(&u)
        );
    }
}

/// Random `uname` assembled from the grammar the C actually recognises.
fn fuzz_uname(rng: &mut Rng) -> Vec<u8> {
    let mut u = Vec::new();
    let parts = rng.range(0, 6);
    for _ in 0..parts {
        match rng.below(14) {
            0 => u.extend_from_slice(b" [Ver: "),
            1 => u.extend_from_slice(b" ["),
            2 => u.extend_from_slice(b": "),
            3 => u.extend_from_slice(b" ("),
            4 => u.push(b'|'),
            5 => u.push(b']'),
            6 => u.push(b')'),
            7 => u.extend_from_slice(rng.pick(&ARCHS)),
            8 => u.extend_from_slice(&rng.digits(1, 5)),
            9 => {
                let d1 = rng.digits(1, 3);
                let d2 = rng.digits(1, 3);
                u.extend_from_slice(&d1);
                u.push(b'.');
                u.extend_from_slice(&d2);
            }
            10 => {
                for k in 0..rng.range(1, 5) {
                    if k > 0 {
                        u.push(b'.');
                    }
                    u.extend_from_slice(&rng.digits(1, 3));
                }
            }
            11 => u.push(b'.'),
            12 => u.push(b' '),
            _ => u.extend_from_slice(&rng.safe_word(0, 10)),
        }
    }
    u
}

#[test]
fn row45_full_fuzz() {
    let mut rng = Rng::new(SEED ^ 45);
    for i in 0..20000 {
        let u = fuzz_uname(&mut rng);
        assert_parse_eq(&format!("row45/#{i}"), &u);
    }
}

#[test]
fn row46_empty_uname() {
    assert_parse_both_inits("row46/empty", b"");
    assert_parse_both_inits("row46/space", b" ");
    assert_parse_both_inits("row46/nul-only", b"");
}

#[test]
fn row47_long_uname() {
    let mut rng = Rng::new(SEED ^ 47);
    for i in 0..200 {
        let mut u = vec![b'z'; 4000];
        let (b1, b2, b3) = (rng.bool(), rng.bool(), rng.bool());
        u.extend_from_slice(&unix_uname(&mut rng, b1, b2, b3));
        assert_parse_both_inits(&format!("row47/unix{i}"), &u);

        let mut w = vec![b'w'; 4000];
        w.extend_from_slice(b" [Ver: ");
        w.extend_from_slice(&random_version(&mut rng));
        w.push(b']');
        assert_parse_both_inits(&format!("row47/win{i}"), &w);

        let mut a = vec![b'q'; 4000];
        a.extend_from_slice(rng.pick(&ARCHS));
        assert_parse_both_inits(&format!("row47/arch{i}"), &a);
    }
    // Very long version component (stresses malloc/snprintf sizing).
    let mut big = Vec::from(&b"W [Ver: "[..]);
    big.extend(std::iter::repeat(b'9').take(3000));
    big.push(b'.');
    big.extend(std::iter::repeat(b'8').take(3000));
    big.push(b'.');
    big.extend(std::iter::repeat(b'7').take(3000));
    big.push(b']');
    assert_parse_both_inits("row47/bigver", &big);
}

#[test]
fn row48_composed_pipeline_matches_field_extraction() {
    // Drive the LOW-LEVEL entry point (`w_regexec`) with exactly the arguments
    // the internal call sites use, reproduce the malloc+snprintf duplication in
    // the test, and check it against what `parse_uname_string` produced. This
    // catches divergence in the composed pipeline, not just per-wrapper.
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 48);

    for i in 0..4000 {
        let ver = random_version(&mut rng);

        // --- Windows branch: str_tmp is the version text ---
        let mut u = Vec::from(&b"N [Ver: "[..]);
        u.extend_from_slice(&ver);
        u.push(b']');
        let pc = c.parse(&u);
        let pr = r.parse(&u);
        assert_eq!(pc, pr, "row48/win#{i}: parse diverged");

        let expect = |pat: &[u8]| -> Option<Vec<u8>> {
            let (ret_c, pm_c) = c.regexec(Some(pat), Some(&ver), 2, &pm_init(2));
            let (ret_r, pm_r) = r.regexec(Some(pat), Some(&ver), 2, &pm_init(2));
            assert_eq!(ret_c, ret_r, "row48: w_regexec ret diverged");
            assert_eq!(pm_c, pm_r, "row48: w_regexec pmatch diverged");
            if ret_c == 0 {
                return None;
            }
            let m = pm_c[1];
            let size = (m.rm_eo - m.rm_so).max(0) as usize;
            let so = m.rm_so;
            if so < 0 {
                // Non-participating group: C mallocs 1 byte and snprintf writes
                // just the NUL -> empty string.
                return Some(Vec::new());
            }
            Some(ver[so as usize..so as usize + size].to_vec())
        };

        assert_eq!(
            pc.snapshot.fields[2],
            expect(PAT_MAJOR),
            "row48/win#{i}: os_major != low-level w_regexec extraction for {:?}",
            String::from_utf8_lossy(&ver)
        );
        assert_eq!(
            pc.snapshot.fields[3],
            expect(PAT_MINOR),
            "row48/win#{i}: os_minor mismatch for {:?}",
            String::from_utf8_lossy(&ver)
        );
        assert_eq!(
            pc.snapshot.fields[6],
            expect(PAT_BUILD),
            "row48/win#{i}: os_build mismatch for {:?}",
            String::from_utf8_lossy(&ver)
        );

        // --- Unix branch: os_version is the regex subject ---
        let mut v = Vec::from(&b"N [n: "[..]);
        v.extend_from_slice(&ver);
        v.push(b']');
        let qc = c.parse(&v);
        let qr = r.parse(&v);
        assert_eq!(qc, qr, "row48/unix#{i}: parse diverged");
        if let Some(osver) = qc.snapshot.fields[1].clone() {
            let expect2 = |pat: &[u8]| -> Option<Vec<u8>> {
                let (ret_c, pm_c) = c.regexec(Some(pat), Some(&osver), 2, &pm_init(2));
                let (ret_r, pm_r) = r.regexec(Some(pat), Some(&osver), 2, &pm_init(2));
                assert_eq!(ret_c, ret_r);
                assert_eq!(pm_c, pm_r);
                if ret_c == 0 {
                    return None;
                }
                let m = pm_c[1];
                if m.rm_so < 0 {
                    return Some(Vec::new());
                }
                let so = m.rm_so as usize;
                let size = (m.rm_eo - m.rm_so) as usize;
                Some(osver[so..so + size].to_vec())
            };
            assert_eq!(
                qc.snapshot.fields[2],
                expect2(PAT_MAJOR),
                "row48/unix#{i}: os_major mismatch for {:?}",
                String::from_utf8_lossy(&osver)
            );
            assert_eq!(
                qc.snapshot.fields[3],
                expect2(PAT_MINOR),
                "row48/unix#{i}: os_minor mismatch for {:?}",
                String::from_utf8_lossy(&osver)
            );
            // The Unix branch never runs the build regex.
            assert_eq!(qc.snapshot.fields[6], None, "row48/unix#{i}: os_build set");
        }

        // --- get_os_arch composed into the Unix branch ---
        let mut a = rng.safe_word(0, 8);
        if rng.bool() {
            a.extend_from_slice(rng.pick(&ARCHS));
        }
        let arch_prefix = a.clone();
        a.extend_from_slice(b" [n: 1.2]");
        let ac = c.parse(&a);
        let ar = r.parse(&a);
        assert_eq!(ac, ar, "row48/arch#{i}: parse diverged");
        assert_eq!(
            ac.snapshot.fields[8],
            c.get_arch(&arch_prefix),
            "row48/arch#{i}: os_arch != get_os_arch(truncated uname)"
        );
        assert_eq!(
            ac.snapshot.fields[8],
            r.get_arch(&arch_prefix),
            "row48/arch#{i}: Rust get_os_arch disagrees"
        );
    }
}
