//! Phase C — error-path differential tests.
//! One test (or one clearly-labelled block) per row of `ERRORS.md`, rows 1-37.
//! Rows 38-39 (the two crashing paths) live in `phase_c_crash_parity.rs`;
//! the stderr text of rows 4-8 is compared in `phase_c_stderr.rs`.

mod common;
use common::*;
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// Rows 1-3: NULL pattern / string
// ---------------------------------------------------------------------------

#[test]
fn row01_null_pattern() {
    for nm in [0usize, 1, 2, 3, 8] {
        diff_w_regexec("row1", None, Some(b"10.0.19041"), nm, Some(8));
        diff_w_regexec("row1 empty-subject", None, Some(b""), nm, Some(8));
        // ...and with a NULL pmatch on top
        diff_w_regexec("row1 nullpmatch", None, Some(b"1.2"), nm, None);
    }
}

#[test]
fn row02_null_string() {
    for nm in [0usize, 1, 2, 3, 8] {
        diff_w_regexec("row2", Some(br"^([0-9]+)\.*"), None, nm, Some(8));
        diff_w_regexec("row2 bad-pattern", Some(b"("), None, nm, Some(8));
        diff_w_regexec("row2 nullpmatch", Some(b"a"), None, nm, None);
    }
}

#[test]
fn row03_both_null() {
    for nm in [0usize, 1, 2, 3, 8, 64] {
        diff_w_regexec("row3", None, None, nm, Some(8));
        diff_w_regexec("row3 nullpmatch", None, None, nm, None);
    }
}

// ---------------------------------------------------------------------------
// Rows 4-8: `regcomp` failures — every distinct class of malformed ERE
// ---------------------------------------------------------------------------

/// Patterns that glibc's `regcomp(REG_EXTENDED)` rejects. Verified against a
/// standalone `regcomp` probe on this platform — see
/// `rows04to08_are_really_compile_failures`.
const BAD_PATTERNS: &[&[u8]] = &[
    // row 4: unmatched parenthesis (REG_EPAREN)
    b"(",
    b"a(b",
    b"((a)",
    b"a)b(",
    // row 5: unmatched bracket (REG_EBRACK / REG_ECTYPE)
    b"[a-",
    b"[",
    b"[^",
    b"[[:alpha:",
    // row 6: trailing backslash (REG_EESCAPE)
    b"a\\",
    b"\\",
    // row 7: bad repetition (REG_BADRPT / REG_BADBR / REG_EBRACE / REG_ESIZE)
    b"*",
    b"a{2,1}",
    b"{1}",
    b"a{",
    b"a{1",
    b"+",
    b"?",
    b"**",
    b"a{100000000000}",
    // row 8: invalid character class / collating element / range (REG_ECTYPE,
    // REG_ECOLLATE, REG_ERANGE)
    b"[[:bogus:]]",
    b"[[.nosuch.]]",
    b"[[=x",
    b"[a-\\",
    b"[z-a]",
];

/// Odd-but-*accepted* patterns: glibc ERE compiles these, so they exercise the
/// match path rather than the `regcomp` branch. Kept here so the distinction is
/// asserted rather than assumed.
const VALID_ODD_PATTERNS: &[&[u8]] = &[b")", b"a{,}", b"", b"^$", b"$^", b"a**b"];

#[test]
fn rows04to08_regcomp_failures() {
    for pat in BAD_PATTERNS {
        for nm in [0usize, 1, 2, 3, 8] {
            for subj in [b"".as_slice(), b"a", b"10.0.1", b"zzzz"] {
                diff_w_regexec("rows4-8", Some(pat), Some(subj), nm, Some(8));
            }
        }
        // NULL pmatch (only legal together with nmatch == 0, otherwise glibc
        // dereferences it — that is covered by the crash-parity tests).
        diff_w_regexec("rows4-8 nullpmatch", Some(pat), Some(b"a"), 0, None);
        // NULL string: must short-circuit *before* regcomp (no diagnostic).
        diff_w_regexec("rows4-8 nullstring", Some(pat), None, 2, Some(8));
    }
}

#[test]
fn valid_odd_patterns_take_the_match_path() {
    let p = pair();
    for pat in VALID_ODD_PATTERNS {
        for subj in [b"".as_slice(), b"a", b")", b"aab", b"10.0.1"] {
            for nm in [0usize, 1, 2, 8] {
                diff_w_regexec("odd", Some(pat), Some(subj), nm, Some(8));
            }
        }
        let mut pb = Buf::new(pat);
        let mut sb = Buf::new(b")a");
        let mut m = vec![RegMatch::sentinel(); 4];
        unsafe {
            let c = (p.c.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr());
            let r = (p.rs.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr());
            assert_eq!(c, r, "pattern {pat:?}");
        }
    }
}

/// Every one of the malformed patterns above must actually be rejected (i.e.
/// the row really does exercise the `regcomp` branch, not the match branch).
#[test]
fn rows04to08_are_really_compile_failures() {
    let p = pair();
    for pat in BAD_PATTERNS {
        let mut pb = Buf::new(pat);
        let mut sb = Buf::new(b"anything");
        let mut m = vec![RegMatch::sentinel(); 4];
        let (c, r) = unsafe {
            (
                (p.c.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr()),
                (p.rs.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr()),
            )
        };
        assert_eq!(c, 0, "pattern {pat:?} unexpectedly compiled in C");
        assert_eq!(r, 0, "pattern {pat:?} unexpectedly compiled in Rust");
        assert_eq!(m, vec![RegMatch::sentinel(); 4], "pmatch must be untouched");
    }
}

// ---------------------------------------------------------------------------
// Row 9: valid pattern, REG_NOMATCH
// ---------------------------------------------------------------------------

#[test]
fn row09_no_match() {
    let cases: &[(&[u8], &[u8])] = &[
        (br"^([0-9]+)\.*", b"abc"),
        (br"^([0-9]+)\.*", b""),
        (br"^[0-9]+\.([0-9]+)\.*", b"10"),
        (br"^[0-9]+\.([0-9]+)\.*", b"10."),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"10.0"),
        (br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", b"10.0."),
        (br"^$", b"x"),
        (br"zzz", b"aaa"),
    ];
    for (pat, subj) in cases {
        for nm in [0usize, 1, 2, 3, 8, 64] {
            diff_w_regexec("row9", Some(pat), Some(subj), nm, Some(64));
        }
    }
    // assert it really is the no-match branch
    let p = pair();
    let mut pb = Buf::new(br"^([0-9]+)\.*");
    let mut sb = Buf::new(b"abc");
    let mut m = vec![RegMatch::sentinel(); 4];
    unsafe {
        assert_eq!((p.c.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr()), 0);
        assert_eq!((p.rs.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr()), 0);
    }
}

// ---------------------------------------------------------------------------
// Rows 10-13: nmatch / pmatch degenerate values
// ---------------------------------------------------------------------------

#[test]
fn row10_nmatch_zero_null_pmatch() {
    for pat in [br"^([0-9]+)\.*".as_slice(), b"abc", b"("] {
        for subj in [b"10.0".as_slice(), b"abc", b""] {
            diff_w_regexec("row10", Some(pat), Some(subj), 0, None);
        }
    }
}

#[test]
fn row11_nmatch_zero_with_buffer() {
    let p = pair();
    for pat in [br"^([0-9]+)\.*".as_slice(), b"abc"] {
        for subj in [b"10.0".as_slice(), b"abc", b""] {
            diff_w_regexec("row11", Some(pat), Some(subj), 0, Some(8));
            // and the buffer must be *bit-identical to the sentinel*
            let mut pb = Buf::new(pat);
            let mut sb = Buf::new(subj);
            let mut cm = vec![RegMatch::sentinel(); 8];
            let mut rm = vec![RegMatch::sentinel(); 8];
            unsafe {
                (p.c.w_regexec)(pb.ptr(), sb.ptr(), 0, cm.as_mut_ptr());
                (p.rs.w_regexec)(pb.ptr(), sb.ptr(), 0, rm.as_mut_ptr());
            }
            assert_eq!(cm, vec![RegMatch::sentinel(); 8], "C wrote with nmatch=0");
            assert_eq!(rm, vec![RegMatch::sentinel(); 8], "Rust wrote with nmatch=0");
        }
    }
}

#[test]
fn row12_nmatch_smaller_than_group_count() {
    let pats: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        br"(a)(b)(c)(d)(e)(f)(g)",
    ];
    for pat in pats {
        for subj in [b"10.0.19041.1".as_slice(), b"abcdefg", b"1"] {
            for nm in [1usize, 2, 3] {
                diff_w_regexec("row12", Some(pat), Some(subj), nm, Some(16));
            }
        }
    }
}

#[test]
fn row13_nmatch_larger_than_group_count() {
    let pats: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.([0-9]+)\.*",
        br"nogroupsatall",
        br"^$",
    ];
    for pat in pats {
        for subj in [b"10.0.19041".as_slice(), b"nogroupsatall", b""] {
            for nm in [3usize, 8, 16, 64] {
                diff_w_regexec("row13", Some(pat), Some(subj), nm, Some(64));
            }
        }
    }
}

/// The "out-of-range enum" analogue for this API: `nmatch` is the only integral
/// parameter reachable from a caller, so sweep it densely (0..=40) plus the
/// pathological ones. `cflags`/`eflags` are hard-coded inside the C function.
#[test]
fn row13b_nmatch_dense_sweep() {
    let pats: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        br"(a)(b)(c)",
        br"noparen",
        br"(",
    ];
    for pat in pats {
        for nm in 0..=40usize {
            diff_w_regexec("row13b", Some(pat), Some(b"10.0.1.2"), nm, Some(48));
            diff_w_regexec("row13b abc", Some(pat), Some(b"abc"), nm, Some(48));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 14-16: participation / empties
// ---------------------------------------------------------------------------

#[test]
fn row14_non_participating_group() {
    let p = pair();
    let mut pb = Buf::new(br"^(a)?b");
    let mut sb = Buf::new(b"b");
    let mut cm = vec![RegMatch::sentinel(); 4];
    let mut rm = vec![RegMatch::sentinel(); 4];
    unsafe {
        assert_eq!((p.c.w_regexec)(pb.ptr(), sb.ptr(), 2, cm.as_mut_ptr()), 1);
        assert_eq!((p.rs.w_regexec)(pb.ptr(), sb.ptr(), 2, rm.as_mut_ptr()), 1);
    }
    assert_eq!(cm[1], RegMatch { rm_so: -1, rm_eo: -1 });
    assert_eq!(cm, rm);

    for nm in [1usize, 2, 3, 8] {
        diff_w_regexec("row14", Some(br"^(a)?b"), Some(b"b"), nm, Some(8));
        diff_w_regexec("row14", Some(br"^(a)?(c)?b"), Some(b"b"), nm, Some(8));
        diff_w_regexec("row14", Some(br"^(a)?(c)?b"), Some(b"ab"), nm, Some(8));
        diff_w_regexec("row14", Some(br"^(a)?(c)?b"), Some(b"cb"), nm, Some(8));
    }
}

#[test]
fn row15_empty_pattern() {
    for subj in [b"".as_slice(), b"anything", b"10.0"] {
        for nm in [0usize, 1, 2, 8] {
            diff_w_regexec("row15", Some(b""), Some(subj), nm, Some(8));
        }
    }
    let p = pair();
    let mut pb = Buf::new(b"");
    let mut sb = Buf::new(b"anything");
    let mut m = vec![RegMatch::sentinel(); 2];
    unsafe {
        assert_eq!((p.c.w_regexec)(pb.ptr(), sb.ptr(), 1, m.as_mut_ptr()), 1);
    }
    assert_eq!(m[0], RegMatch { rm_so: 0, rm_eo: 0 });
}

#[test]
fn row16_empty_subject() {
    let pats: &[&[u8]] = &[b"a", br"^([0-9]+)\.*", b"^$", b"", br"[[:digit:]]"];
    for pat in pats {
        for nm in [0usize, 1, 2, 8] {
            diff_w_regexec("row16", Some(pat), Some(b""), nm, Some(8));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17-19: get_os_arch rejections
// ---------------------------------------------------------------------------

#[test]
fn row17_no_arch_found() {
    let cases: &[&[u8]] = &[
        b"Linux host 5.15.0 GNU/Linux",
        b"no architecture here",
        b"powerpc64le",
        b"riscv64",
        b"mips",
        b"s390x",
        b"armv8",
        b"i486",
        b"i586",
    ];
    for c in cases {
        diff_get_os_arch("row17", c);
        // and confirm it really is the NULL branch
        let p = pair();
        let mut b = Buf::new(c);
        unsafe {
            assert!(
                (p.c.get_os_arch)(b.ptr()).is_null(),
                "C found an arch in {c:?}"
            );
            assert!(
                (p.rs.get_os_arch)(b.ptr()).is_null(),
                "Rust found an arch in {c:?}"
            );
        }
    }
}

#[test]
fn row18_empty_header() {
    diff_get_os_arch("row18", b"");
    let p = pair();
    let mut b = Buf::new(b"");
    unsafe {
        assert!((p.c.get_os_arch)(b.ptr()).is_null());
        assert!((p.rs.get_os_arch)(b.ptr()).is_null());
    }
}

#[test]
fn row19_case_and_near_miss() {
    let cases: &[&[u8]] = &[
        b"X86_64", b"X86_64 ", b"aix", b"Aix", b"aIx", b"ARM64", b"AARCH64", b"SPARC", b"AMD64",
        b"IA64", b"x86-64", b"x86 64", b"x8664", b"armv", b"armv5", b"armv8", b"i38", b"i68",
        b"i86p", b"86pc", b"aarch", b"arch64", b"rm64", b"AI", b"IX", b"sparC", b"Sparc",
    ];
    for c in cases {
        diff_get_os_arch("row19", c);
    }
}

// ---------------------------------------------------------------------------
// Rows 20-21: NULL os_data
// ---------------------------------------------------------------------------

#[test]
fn row20_null_osd() {
    let inputs: &[&[u8]] = &[
        b"Microsoft Windows 10 [Ver: 10.0.19041.1237]",
        b"Linux x86_64 [Ubuntu|ubuntu: 22.04 (Jammy)]",
        b"host [OS]",
        b"amd64",
        b"",
        b" [Ver: ",
        b" [",
    ];
    for i in inputs {
        diff_parse_uname_null_osd("row20", Some(i));
    }
}

#[test]
fn row21_null_osd_and_null_uname() {
    // Must not dereference `uname`: the `!osd` check comes first.
    diff_parse_uname_null_osd("row21", None);
}

// ---------------------------------------------------------------------------
// Rows 22-31: parse_uname_string sub-parses that do not fire
// ---------------------------------------------------------------------------

/// Assert that the named `os_data` members are left at the poison value by
/// *both* implementations (i.e. the C really does take the rejection branch).
fn assert_untouched(uname: &[u8], expect_untouched: &[&str]) {
    let p = pair();
    let poison = 0xAAu8;
    let mut cb = Buf::new(uname);
    let mut rb = Buf::new(uname);
    let mut c_osd = OsData::poisoned(poison);
    let mut r_osd = OsData::poisoned(poison);
    unsafe {
        (p.c.parse_uname_string)(cb.ptr(), &mut c_osd);
        (p.rs.parse_uname_string)(rb.ptr(), &mut r_osd);
    }
    for name in expect_untouched {
        let idx = OS_DATA_FIELD_NAMES.iter().position(|n| n == name).unwrap();
        assert_eq!(
            c_osd.fields[idx],
            OsData::poison_ptr(poison),
            "C set {name} for input {uname:?} but the row expects it untouched"
        );
        assert_eq!(
            r_osd.fields[idx],
            OsData::poison_ptr(poison),
            "Rust set {name} for input {uname:?} but the row expects it untouched"
        );
    }
    unsafe {
        for i in 0..9 {
            free_if_owned(c_osd.fields[i], poison);
            free_if_owned(r_osd.fields[i], poison);
        }
    }
    // and the full differential comparison on top
    diff_parse_uname("assert_untouched", uname, 0x00);
    diff_parse_uname("assert_untouched", uname, 0xAA);
}

#[test]
fn row22_neither_marker() {
    let all_but_arch = [
        "os_name",
        "os_version",
        "os_major",
        "os_minor",
        "os_codename",
        "os_platform",
        "os_build",
        "os_uname",
    ];
    for u in [
        b"".as_slice(),
        b"Linux",
        b"x86_64",
        b"Linux[Ubuntu: 22.04]",
        b"[Ver: 10.0]",
        b"Ver: 10.0",
        b"a:b(c)|d]",
    ] {
        assert_untouched(u, &all_but_arch);
    }
}

#[test]
fn row23_bracket_without_colon_space() {
    for u in [
        b"host [OS]".as_slice(),
        b"host [OS",
        b"host [OS|plat]",
        b"host [OS:notspace]",
        b"host [OS :x]",
        b"host [",
        b" [",
        b" []",
    ] {
        assert_untouched(u, &["os_version", "os_major", "os_minor", "os_codename"]);
    }
}

#[test]
fn row24_no_codename_marker() {
    for u in [
        b"host [OS: 1.2]".as_slice(),
        b"host [OS: 1.2(x)]",
        b"host [OS: rolling]",
        b"host [OS: 1.2 x]",
    ] {
        assert_untouched(u, &["os_codename"]);
    }
}

#[test]
fn row25_non_numeric_version_unix() {
    for u in [
        b"host [OS: rolling]".as_slice(),
        b"host [OS: unstable]",
        b"host [OS: v1.2]",
        b"host [OS: .1.2]",
        b"host [OS: -1]",
        b"host [OS:  1.2]",
        b"host [OS: x (code)]",
    ] {
        assert_untouched(u, &["os_major", "os_minor"]);
    }
}

#[test]
fn row26_major_without_minor_unix() {
    for u in [
        b"host [OS: 9]".as_slice(),
        b"host [OS: 11]",
        b"host [OS: 2]",
        b"host [OS: 7 (core)]",
        b"host [OS: 9.]",
        b"host [OS: 9.x]",
    ] {
        assert_untouched(u, &["os_minor"]);
    }
}

#[test]
fn row27_no_pipe_in_os_name() {
    for u in [
        b"host [OS: 1.2]".as_slice(),
        b"host [OS]",
        b"host [Ubuntu: 22.04 (Jammy)]",
    ] {
        assert_untouched(u, &["os_platform"]);
    }
}

#[test]
fn row28_windows_non_numeric_version() {
    for u in [
        b"Win [Ver: rolling]".as_slice(),
        b"Win [Ver: x10.0]",
        b"Win [Ver: .10]",
        b"Win [Ver: ]",
        b"Win [Ver: -1]",
        b"Win [Ver:  10]",
    ] {
        assert_untouched(u, &["os_major", "os_minor", "os_build"]);
    }
}

#[test]
fn row29_windows_major_only() {
    for u in [
        b"Win [Ver: 10]".as_slice(),
        b"Win [Ver: 6]",
        b"Win [Ver: 10.]",
        b"Win [Ver: 10.x]",
    ] {
        assert_untouched(u, &["os_minor", "os_build"]);
    }
}

#[test]
fn row30_windows_major_minor_only() {
    for u in [
        b"Win [Ver: 6.1]".as_slice(),
        b"Win [Ver: 10.0]",
        b"Win [Ver: 10.0.]",
        b"Win [Ver: 10.0.x]",
    ] {
        assert_untouched(u, &["os_build"]);
    }
}

#[test]
fn row31_windows_never_sets_arch() {
    for arch in ARCHS.iter() {
        let mut u = arch.as_bytes().to_vec();
        u.extend_from_slice(b" Windows [Ver: 10.0.19041]");
        assert_untouched(&u, &["os_arch"]);

        let mut v = b"Windows [Ver: 10.0 ".to_vec();
        v.extend_from_slice(arch.as_bytes());
        v.push(b']');
        assert_untouched(&v, &["os_arch"]);
    }
}

// ---------------------------------------------------------------------------
// Rows 32-36: the out-of-bounds / empty-string boundary writes
// ---------------------------------------------------------------------------

#[test]
fn row32_ver_marker_at_end() {
    // `str_tmp` lands on "", so `*(str_tmp + strlen - 1)` writes one byte
    // *before* it — inside the guard-padded buffer, so it is compared.
    for u in [
        b" [Ver: ".as_slice(),
        b"Win [Ver: ",
        b"a [Ver: ",
        b"x86_64 [Ver: ",
    ] {
        diff_parse_uname("row32", u, 0x00);
        diff_parse_uname("row32", u, 0xAA);
    }
}

#[test]
fn row33_bracket_marker_at_end() {
    // `os_name = strdup("")`, then `*(os_name + strlen - 1) = 0` writes one
    // byte before the heap block.
    for u in [b" [".as_slice(), b"host [", b"x86_64 [", b"a ["] {
        diff_parse_uname("row33", u, 0x00);
        diff_parse_uname("row33", u, 0xAA);
    }
}

#[test]
fn row34_colon_space_at_end() {
    // `os_version = strdup("")`, then the one-byte-before write.
    for u in [
        b"host [OS: ".as_slice(),
        b" [: ",
        b"host [OS|plat: ",
        b"a [b: ",
    ] {
        diff_parse_uname("row34", u, 0x00);
        diff_parse_uname("row34", u, 0xAA);
    }
}

#[test]
fn row35_paren_marker_at_end() {
    // `os_codename = strdup("")`, then the one-byte-before write.
    for u in [
        b"host [OS: 1.2 (".as_slice(),
        b"host [OS: (",
        b" [: (",
        b"host [OS|p: 1.2 (",
    ] {
        diff_parse_uname("row35", u, 0x00);
        diff_parse_uname("row35", u, 0xAA);
    }
}

#[test]
fn row36_pipe_at_end_of_os_name() {
    // `os_platform = strdup("")` — empty string, not NULL.
    let p = pair();
    for u in [
        b"host [OS|]".as_slice(),
        b"host [OS|: 1.2]",
        b"host [|: 1.2]",
        b"host [|]",
    ] {
        diff_parse_uname("row36", u, 0x00);
        diff_parse_uname("row36", u, 0xAA);
    }
    // `host [OS|]` -> os_name "OS", os_platform "" (the ']' was already eaten)
    let mut cb = Buf::new(b"host [OS|]");
    let mut osd = OsData::poisoned(0xAA);
    unsafe {
        (p.c.parse_uname_string)(cb.ptr(), &mut osd);
        let plat = field_value(osd.fields[5], 0xAA);
        assert_eq!(
            plat,
            Some(Vec::new()),
            "C should set os_platform to the empty string"
        );
        for i in 0..9 {
            free_if_owned(osd.fields[i], 0xAA);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 37: empty uname
// ---------------------------------------------------------------------------

#[test]
fn row37_empty_uname() {
    diff_parse_uname("row37", b"", 0x00);
    diff_parse_uname("row37", b"", 0xAA);
    assert_untouched(
        b"",
        &[
            "os_name",
            "os_version",
            "os_major",
            "os_minor",
            "os_codename",
            "os_platform",
            "os_build",
            "os_uname",
            "os_arch",
        ],
    );
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary sanity: os_uname is never written by either side.
// ---------------------------------------------------------------------------

#[test]
fn os_uname_member_is_never_written() {
    let mut rng = Rng::new(0xC0FFEE);
    const ALPHA: &[u8] = b" [](:|)Ver.019x86_64AIX";
    for _ in 0..5000 {
        let len = rng.range(0, 32);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHA)).collect();
        assert_untouched_quick(&s, 7);
    }
}

fn assert_untouched_quick(uname: &[u8], idx: usize) {
    let p = pair();
    let poison = 0x5Au8;
    let mut cb = Buf::new(uname);
    let mut rb = Buf::new(uname);
    let mut c_osd = OsData::poisoned(poison);
    let mut r_osd = OsData::poisoned(poison);
    unsafe {
        (p.c.parse_uname_string)(cb.ptr(), &mut c_osd);
        (p.rs.parse_uname_string)(rb.ptr(), &mut r_osd);
        assert_eq!(c_osd.fields[idx], OsData::poison_ptr(poison));
        assert_eq!(r_osd.fields[idx], OsData::poison_ptr(poison));
        for i in 0..9 {
            free_if_owned(c_osd.fields[i], poison);
            free_if_owned(r_osd.fields[i], poison);
        }
    }
}

// ---------------------------------------------------------------------------
// Extra: passing a mis-sized / mis-aligned pmatch is UB, but passing a
// *correctly typed* pmatch that is not writable for `nmatch` entries is a real
// caller mistake; verify both implementations write exactly the same number of
// entries by using an exactly-sized buffer plus guard words.
// ---------------------------------------------------------------------------

#[test]
fn pmatch_write_extent_is_identical() {
    let p = pair();
    let pats: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        br"(a)(b)(c)",
        br"noparen",
    ];
    for pat in pats {
        for nm in 0..=6usize {
            let mut pb = Buf::new(pat);
            let mut sb = Buf::new(b"10.0.19041.1");
            // 6 usable + 6 guard entries
            let mut cm = vec![RegMatch::sentinel(); 12];
            let mut rm = vec![RegMatch::sentinel(); 12];
            unsafe {
                (p.c.w_regexec)(
                    pb.ptr() as *const c_char,
                    sb.ptr() as *const c_char,
                    nm,
                    cm.as_mut_ptr(),
                );
                (p.rs.w_regexec)(
                    pb.ptr() as *const c_char,
                    sb.ptr() as *const c_char,
                    nm,
                    rm.as_mut_ptr(),
                );
            }
            assert_eq!(cm, rm, "pmatch extent differs for {pat:?} nmatch={nm}");
            for (i, e) in cm.iter().enumerate().skip(nm) {
                assert_eq!(
                    *e,
                    RegMatch::sentinel(),
                    "C wrote past nmatch={nm} at slot {i} for {pat:?}"
                );
            }
        }
    }
}
