//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test constructs the exact rejection
//! condition the C source checks and asserts both `.so`s answer with the *same*
//! sentinel / error value (not merely "both failed").

mod common;

use common::*;

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

// ===========================================================================
// E1..E3 — w_regexec null-pointer guard (lib.c:36-38)
// ===========================================================================

/// The guard must leave `pmatch` completely untouched and return exactly 0.
fn assert_regexec_rejected_untouched(
    pattern: Option<&[u8]>,
    subject: Option<&[u8]>,
    nmatch: usize,
    ctx: &str,
) {
    let b = both();
    let slots = nmatch.max(4);
    let (vc, mc) = call_regexec(b.c.w_regexec, pattern, subject, nmatch, slots);
    let (vr, mr) = call_regexec(b.rs.w_regexec, pattern, subject, nmatch, slots);
    assert_eq!(vc, 0, "[{ctx}] C did not return the 0 sentinel");
    assert_eq!(vr, vc, "[{ctx}] Rust returned {vr}, C returned {vc}");
    assert_eq!(mc, mr, "[{ctx}] pmatch differs: C={mc:?} Rust={mr:?}");
    assert!(
        mc.iter().all(|m| *m == SENTINEL),
        "[{ctx}] C wrote into pmatch on the reject path: {mc:?}"
    );
    assert!(
        mr.iter().all(|m| *m == SENTINEL),
        "[{ctx}] Rust wrote into pmatch on the reject path: {mr:?}"
    );
}

#[test]
fn e1_pattern_null() {
    for sub in ["", "1.2.3", "abc", "\u{ff}\u{fe}"] {
        for n in [0usize, 1, 2, 3, 8] {
            assert_regexec_rejected_untouched(None, Some(sub.as_bytes()), n, "E1");
        }
    }
}

#[test]
fn e2_string_null() {
    for pat in ["", "^([0-9]+)\\.*", "abc", "((("] {
        for n in [0usize, 1, 2, 3, 8] {
            assert_regexec_rejected_untouched(Some(pat.as_bytes()), None, n, "E2");
        }
    }
    // The pattern must NOT even be compiled — an *invalid* pattern with a NULL
    // subject must still take the lib.c:36 path (return 0, nothing on stderr).
    let b = both();
    let (_, ec) = capture_stderr("e2c", || {
        call_regexec(b.c.w_regexec, Some(b"((("), None, 2, 4)
    });
    let (_, er) = capture_stderr("e2r", || {
        call_regexec(b.rs.w_regexec, Some(b"((("), None, 2, 4)
    });
    assert!(ec.is_empty(), "E2: C printed on the null-subject path: {ec:?}");
    assert_eq!(ec, er, "E2: stderr differs on the null-subject path");
}

#[test]
fn e3_both_null() {
    for n in [0usize, 1, 2, 3, 8, 64] {
        assert_regexec_rejected_untouched(None, None, n, "E3");
    }
}

// ===========================================================================
// E4 — regcomp failure (lib.c:40-43)
// ===========================================================================

#[test]
fn e4_regcomp_failure_matrix() {
    // Every construct that can make glibc's ERE compiler fail.
    let bad: [&str; 34] = [
        "(",
        ")",
        "(()",
        "())",
        "((()",
        "[",
        "[]",
        "[^",
        "[a",
        "[a-",
        "[z-a]",
        "[[:bogus:]]",
        "[[:alpha",
        "[[.x.]",
        "[[=x=]",
        "a{",
        "a{1",
        "a{1,",
        "a{2,1}",
        "a{,",
        "a{1,2",
        "*",
        "+",
        "?",
        "{1,2}",
        "\\",
        "a\\",
        "|*",
        "(*)",
        "(|*)",
        "a**{",
        "\\{",
        "[a-\\",
        "((((((((((((((((((((((((((((((((((",
    ];
    let subjects: [&str; 4] = ["", "a", "abc123", "1.2.3"];
    let b = both();
    for pat in bad {
        for sub in subjects {
            let slots = 4;
            // Return value + pmatch must match.
            let ((vc, mc), ec) = capture_stderr("e4c", || {
                call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 2, slots)
            });
            let ((vr, mr), er) = capture_stderr("e4r", || {
                call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 2, slots)
            });
            assert_eq!(
                vc, vr,
                "E4: return differs for pattern {pat:?} subject {sub:?}: C={vc} Rust={vr}"
            );
            assert_eq!(
                mc, mr,
                "E4: pmatch differs for pattern {pat:?} subject {sub:?}: C={mc:?} Rust={mr:?}"
            );
            // If the C rejected the pattern it must have said so on stderr, and
            // the Rust must produce byte-identical diagnostics.
            assert_eq!(
                ec, er,
                "E4: stderr differs for pattern {pat:?}\n  C   = {:?}\n  Rust= {:?}",
                String::from_utf8_lossy(&ec),
                String::from_utf8_lossy(&er)
            );
            if !ec.is_empty() {
                assert_eq!(vc, 0, "E4: C printed a diagnostic but returned {vc}");
                let expected =
                    format!("Couldn't compile regular expression '{pat}'\n").into_bytes();
                assert_eq!(
                    ec, expected,
                    "E4: unexpected C diagnostic text for {pat:?}: {:?}",
                    String::from_utf8_lossy(&ec)
                );
            }
        }
    }
}

// ===========================================================================
// E5 — regexec REG_NOMATCH collapsed to 0 (lib.c:45,47)
// ===========================================================================

#[test]
fn e5_nomatch() {
    let cases: [(&str, &str); 12] = [
        ("^([0-9]+)\\.*", "abc"),
        ("^([0-9]+)\\.*", ""),
        ("^[0-9]+\\.([0-9]+)\\.*", "1"),
        ("^[0-9]+\\.([0-9]+)\\.*", "1.a"),
        ("^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", "1.2"),
        ("^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", "1.2.a"),
        ("^abc$", "abcd"),
        ("^$", "x"),
        ("[0-9]", "abcdef"),
        ("zzz", "aaa"),
        ("^x", "yx"),
        ("q$", "qa"),
    ];
    let b = both();
    for (pat, sub) in cases {
        for n in [0usize, 1, 2, 4] {
            let (vc, mc) = call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 4);
            let (vr, mr) = call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 4);
            assert_eq!(vc, 0, "E5: C should report no-match for {pat:?}/{sub:?}");
            assert_eq!(vr, vc, "E5: {pat:?}/{sub:?}: C={vc} Rust={vr}");
            assert_eq!(mc, mr, "E5: pmatch differs for {pat:?}/{sub:?}");
        }
    }
}

// ===========================================================================
// E6..E8 — nmatch domain
// ===========================================================================

#[test]
fn e6_nmatch_zero() {
    let b = both();
    for (pat, sub) in [
        ("^([0-9]+)\\.*", "10.0.1"),
        ("abc", "xxabcxx"),
        ("^$", ""),
        ("nope", "yyy"),
        ("(a)(b)(c)", "abc"),
    ] {
        let (vc, mc) = call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 0, 8);
        let (vr, mr) = call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 0, 8);
        assert_eq!(vr, vc, "E6: return differs for {pat:?}/{sub:?}");
        assert_eq!(mc, mr, "E6: pmatch differs for {pat:?}/{sub:?}");
        assert!(
            mc.iter().all(|m| *m == SENTINEL),
            "E6: C wrote pmatch with nmatch==0: {mc:?}"
        );
        assert!(
            mr.iter().all(|m| *m == SENTINEL),
            "E6: Rust wrote pmatch with nmatch==0: {mr:?}"
        );
    }
}

#[test]
fn e7_nmatch_zero_pmatch_null() {
    // nmatch == 0 makes a NULL pmatch legal for regexec; both must agree.
    let b = both();
    for (pat, sub) in [
        ("^([0-9]+)\\.*", "10.0.1"),
        ("abc", "xxabcxx"),
        ("nope", "yyy"),
        ("^$", ""),
        ("(((", "x"),
    ] {
        let mut pb = Buf::new(pat.as_bytes());
        let mut sb = Buf::new(sub.as_bytes());
        let (vc, vr) = with_stderr_silenced(|| unsafe {
            (
                (b.c.w_regexec)(pb.cptr(), sb.cptr(), 0, std::ptr::null_mut()),
                (b.rs.w_regexec)(pb.cptr(), sb.cptr(), 0, std::ptr::null_mut()),
            )
        });
        let _ = (pb.ptr(), sb.ptr());
        assert_eq!(vr, vc, "E7: return differs for {pat:?}/{sub:?}");
    }
}

#[test]
fn e8_nmatch_oversized() {
    // nmatch far larger than the group count: surplus slots must be filled
    // identically (glibc sets them to {-1,-1}).
    let b = both();
    let cases: [(&str, &str); 8] = [
        ("^([0-9]+)\\.*", "10.0.1"),
        ("^([0-9]+)\\.([0-9]+)$", "1.2"),
        ("(a)", "a"),
        ("a", "a"),
        ("^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", "1.2.3.4"),
        ("^(a)(b)?(c)?$", "a"),
        ("(x)|(y)", "y"),
        ("^$", ""),
    ];
    for (pat, sub) in cases {
        for n in [1usize, 2, 3, 5, 8, 16, 64, 128] {
            let (vc, mc) =
                call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 128);
            let (vr, mr) =
                call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 128);
            assert_eq!(vr, vc, "E8: return differs for {pat:?}/{sub:?} nmatch={n}");
            assert_eq!(mc, mr, "E8: pmatch differs for {pat:?}/{sub:?} nmatch={n}");
            // slots beyond nmatch must remain the caller's sentinel
            for (i, m) in mc.iter().enumerate().skip(n) {
                assert_eq!(*m, SENTINEL, "E8: C wrote past nmatch at slot {i}");
            }
            for (i, m) in mr.iter().enumerate().skip(n) {
                assert_eq!(*m, SENTINEL, "E8: Rust wrote past nmatch at slot {i}");
            }
        }
    }
}

#[test]
fn e9_nonparticipating_group() {
    // A group that exists in the pattern but does not take part in the match
    // yields {-1,-1}; `dup_match` would then compute base + (-1) and a size of 0.
    let b = both();
    let cases: [(&str, &str); 10] = [
        ("^(a)?b", "b"),
        ("^(a)?b", "ab"),
        ("(x)|(y)", "x"),
        ("(x)|(y)", "y"),
        ("^([0-9]+)?$", ""),
        ("^(z)*a", "a"),
        ("^a(b)?(c)?$", "a"),
        ("^(q)|r", "r"),
        ("^([0-9]+)(\\.[0-9]+)*$", "5"),
        ("^([0-9]+)\\.[0-9]+(\\.[0-9]+)?$", "1.2"),
    ];
    for (pat, sub) in cases {
        for n in [1usize, 2, 3, 8] {
            let (vc, mc) =
                call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 8);
            let (vr, mr) =
                call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), n, 8);
            assert_eq!(vr, vc, "E9: return differs for {pat:?}/{sub:?} nmatch={n}");
            assert_eq!(mc, mr, "E9: pmatch differs for {pat:?}/{sub:?} nmatch={n}");
        }
    }

    // And drive the *same* condition through parse_uname_string, where the
    // non-participating group would be consumed by the malloc/snprintf pair.
    // `^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*` group 2 never participates for
    // a 3-component build, so match[1] is what matters — cover both.
    for v in ["1.2.3", "1.2.3.4", "1.2.3.4.5", "0.0.0"] {
        diff_parse(format!("w [Ver: {v}]").as_bytes(), "E9/parse");
    }
}

#[test]
fn e10_empty_pattern() {
    let b = both();
    for sub in ["", "a", "abc", "1.2.3"] {
        for n in [0usize, 1, 2, 8] {
            let (vc, mc) = call_regexec(b.c.w_regexec, Some(b""), Some(sub.as_bytes()), n, 8);
            let (vr, mr) = call_regexec(b.rs.w_regexec, Some(b""), Some(sub.as_bytes()), n, 8);
            assert_eq!(vr, vc, "E10: return differs for subject {sub:?} nmatch={n}");
            assert_eq!(mc, mr, "E10: pmatch differs for subject {sub:?} nmatch={n}");
        }
    }
}

#[test]
fn e11_empty_subject() {
    let b = both();
    let pats: [&str; 12] = [
        "", "^$", "a", "a*", "a?", "^", "$", "^.*$", ".", "[0-9]*", "()", "^([0-9]+)\\.*",
    ];
    for pat in pats {
        for n in [0usize, 1, 2, 8] {
            let (vc, mc) = call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(b""), n, 8);
            let (vr, mr) = call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(b""), n, 8);
            assert_eq!(vr, vc, "E11: return differs for pattern {pat:?} nmatch={n}");
            assert_eq!(mc, mr, "E11: pmatch differs for pattern {pat:?} nmatch={n}");
        }
    }
}

// ===========================================================================
// E12..E13 — get_os_arch NULL sentinel (lib.c:19,29)
// ===========================================================================

#[test]
fn e12_arch_not_found() {
    let b = both();
    let no_arch: [&str; 16] = [
        "Linux host 5.15.0-generic",
        "Windows",
        "x86",
        "arm",
        "aarch",
        "AMD64",
        "IA64",
        "Sparc",
        "armv",
        "i38",
        "86_64",
        "no arch at all here",
        "..........",
        "\u{ff}\u{fe}\u{80}",
        "\t\n ",
        "0123456789",
    ];
    for input in no_arch {
        let rc = call_arch(b.c.get_os_arch, input.as_bytes());
        let rr = call_arch(b.rs.get_os_arch, input.as_bytes());
        assert_eq!(rc, None, "E12: C found an arch in {input:?}: {rc:?}");
        assert_eq!(rr, rc, "E12: {input:?}: C={rc:?} Rust={rr:?}");
        diff_arch(input.as_bytes(), "E12");
    }
}

#[test]
fn e13_arch_empty() {
    let b = both();
    let rc = call_arch(b.c.get_os_arch, b"");
    let rr = call_arch(b.rs.get_os_arch, b"");
    assert_eq!(rc, None, "E13: C returned {rc:?} for the empty string");
    assert_eq!(rr, rc, "E13: C={rc:?} Rust={rr:?}");
}

// ===========================================================================
// E14..E15 — parse_uname_string guards
// ===========================================================================

#[test]
fn e14_osd_null() {
    // lib.c:64-65: silent no-op, and the caller's buffer must be untouched.
    let b = both();
    let inputs: [&str; 10] = [
        "",
        "Linux x86_64",
        "w [Ver: 10.0.1]",
        "h [Ubuntu|ubuntu: 22.04 (jammy)]",
        " [",
        " [Ver: ",
        " []",
        "aarch64",
        " [a: b]",
        "|",
    ];
    for input in inputs {
        let mut bc = Buf::new(input.as_bytes());
        let mut br = Buf::new(input.as_bytes());
        let pristine = Buf::new(input.as_bytes()).image();
        unsafe {
            (b.c.parse_uname_string)(bc.ptr(), std::ptr::null_mut());
            (b.rs.parse_uname_string)(br.ptr(), std::ptr::null_mut());
        }
        assert_eq!(
            bc.image(),
            pristine,
            "E14: C mutated the buffer despite osd==NULL for {input:?}"
        );
        assert_eq!(
            br.image(),
            bc.image(),
            "E14: Rust differs from C with osd==NULL for {input:?}"
        );
    }
}

#[test]
fn e15_no_bracket_at_all() {
    // Neither " [Ver: " nor " [" → lib.c:68 and lib.c:98 both fail; only
    // os_arch may be written. Verified against a pre-filled os_data so that
    // "untouched" is distinguishable from "set to NULL".
    let inputs: [&str; 14] = [
        "",
        "Linux",
        "Linux host 5.15.0-generic",
        "Linux host 5.15.0 x86_64",
        "SunOS s11 5.11 i86pc",
        "AIX p7 1 7",
        "Darwin mac 22.6.0 arm64",
        "[Ubuntu: 22.04]",
        "Ubuntu: 22.04",
        "no-space[bracket]",
        "trailing space ",
        "|pipe|only|",
        "a: b (c)",
        "\u{ff}\u{fe} aarch64",
    ];
    let b = both();
    for input in inputs {
        assert!(
            !input.contains(" ["),
            "E15 fixture {input:?} must not contain \" [\""
        );
        diff_parse(input.as_bytes(), "E15");
        diff_parse_prefilled(input.as_bytes(), "E15/prefilled");

        // Explicitly assert the C's own behaviour: exactly 8 untouched fields
        // unless an arch token is present.
        let sent = Sentinels::new("e15");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        for i in 0..9 {
            if FIELD_NAMES[i] == "os_arch" {
                continue;
            }
            assert!(
                out.untouched[i],
                "E15: C wrote {} for {input:?}",
                FIELD_NAMES[i]
            );
        }
    }
}

// ===========================================================================
// E16..E20 — degenerate / underflow paths
// ===========================================================================

#[test]
fn e16_bracket_without_colon() {
    let inputs: [&str; 14] = [
        "host [Ubuntu]",
        "host [Ubuntu",
        "host [U]",
        "host [a|b]",
        "host [a|b",
        "host [|]",
        "host [:]",
        "host [:x]",
        "host [x:]",
        "host [ :x]",
        "host x86_64 [Ubuntu]",
        " [Ubuntu]",
        "host [Ubuntu] extra",
        "host [Ubuntu]]]",
    ];
    let b = both();
    for input in inputs {
        assert!(
            !input.split_once(" [").unwrap().1.contains(": "),
            "E16 fixture {input:?} must not contain \": \" after \" [\""
        );
        diff_parse(input.as_bytes(), "E16");
        diff_parse_prefilled(input.as_bytes(), "E16/prefilled");
        let sent = Sentinels::new("e16");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        for f in ["os_version", "os_major", "os_minor", "os_codename", "os_uname"] {
            let i = FIELD_NAMES.iter().position(|x| *x == f).unwrap();
            assert!(out.untouched[i], "E16: C wrote {f} for {input:?}");
        }
    }
}

#[test]
fn e17_empty_os_name_underflow() {
    // " [" at the very end → strdup("") → lib.c:131 writes os_name[-1].
    for input in [" [", "host [", "host x86_64 [", "a b [", " [ [", "x [Ver [", "  ["] {
        diff_parse(input.as_bytes(), "E17");
        diff_parse_prefilled(input.as_bytes(), "E17/prefilled");
    }
    let b = both();
    let out = run_parse_zeroed(b.c.parse_uname_string, b"host [");
    assert_eq!(
        out.fields[0],
        Some(Vec::new()),
        "E17: C should leave os_name as the empty string"
    );
}

#[test]
fn e18_empty_os_version_underflow() {
    // ": " at the very end → os_version == "" → lib.c:106 writes os_version[-1].
    for input in [
        " [: ",
        "host [Ubuntu: ",
        "host [a|b: ",
        "host x86_64 [Ubuntu: ",
        " [: : ",
    ] {
        diff_parse(input.as_bytes(), "E18");
        diff_parse_prefilled(input.as_bytes(), "E18/prefilled");
    }
    let b = both();
    let out = run_parse_zeroed(b.c.parse_uname_string, b"host [Ubuntu: ");
    assert_eq!(
        out.fields[1],
        Some(Vec::new()),
        "E18: C should leave os_version as the empty string"
    );
}

#[test]
fn e19_empty_os_codename_underflow() {
    // " (" at the end of the version → os_codename == "" → lib.c:113.
    // NOTE: lib.c:106 chops the version's last byte *before* lib.c:109 looks
    // for " (", so the input must end with " ()" (not " (") for os_codename to
    // become the empty string.
    for input in [
        "host [Ubuntu: 22.04 (",
        "host [Ubuntu: 22.04 ()",
        "host [Ubuntu: 22.04 ()x",
        "host [Ubuntu: (",
        "host [Ubuntu: ()",
        "host [Ubuntu: ())",
        " [: (",
        " [: ()",
        "host [a|b: 1.2 (",
        "host [a|b: 1.2 ()",
        "host [Ubuntu: 22.04 ( ",
        "host [Ubuntu: 22.04 (  ",
    ] {
        diff_parse(input.as_bytes(), "E19");
        diff_parse_prefilled(input.as_bytes(), "E19/prefilled");
    }
    let b = both();
    let out = run_parse_zeroed(b.c.parse_uname_string, b"host [Ubuntu: 22.04 ()");
    assert_eq!(
        out.fields[4],
        Some(Vec::new()),
        "E19: C should leave os_codename as the empty string"
    );
}

#[test]
fn e20_empty_ver_underflow_caller_buffer() {
    // uname ends exactly with " [Ver: " → lib.c:72 writes *into the caller's
    // buffer*, one byte before the (empty) remainder — i.e. over the trailing
    // space of " [Ver: ". diff_parse compares the whole buffer image including
    // guard bytes, so any difference in that write is caught.
    for input in [
        " [Ver: ",
        "w [Ver: ",
        "Microsoft Windows 10 [Ver: ",
        "x86_64 [Ver: ",
        " [Ver:  ",
        "a [Ver: b [Ver: ",
    ] {
        diff_parse(input.as_bytes(), "E20");
        diff_parse_prefilled(input.as_bytes(), "E20/prefilled");
    }
    // Prove the write really happens where we think it does.
    let b = both();
    let out = run_parse_zeroed(b.c.parse_uname_string, b"w [Ver: ");
    // buffer layout: 16 guard bytes, then "w [Ver: \0", then 16 guard bytes
    assert_eq!(out.buffer[16], b'w');
    assert_eq!(out.buffer[17], 0, "*str_tmp = '\\0' at the space");
    assert_eq!(out.buffer[16 + 7], 0, "trim wrote over the trailing space");
    let outr = run_parse_zeroed(b.rs.parse_uname_string, b"w [Ver: ");
    assert_eq!(out.buffer, outr.buffer, "E20: buffer image differs");
}

// ===========================================================================
// E21..E24 — regex-miss paths inside parse_uname_string
// ===========================================================================

#[test]
fn e21_ver_nonnumeric() {
    let b = both();
    for payload in [
        "abc", "", " ", "a1", "-1", "+1", ".", "..", "x.y.z", "v10.0.1", " 10.0.1", "\u{ff}",
    ] {
        let input = format!("Win [Ver: {payload}]");
        diff_parse(input.as_bytes(), "E21");
        diff_parse_prefilled(input.as_bytes(), "E21/prefilled");
        let sent = Sentinels::new("e21");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        for f in ["os_major", "os_minor", "os_build"] {
            let i = FIELD_NAMES.iter().position(|x| *x == f).unwrap();
            assert!(
                out.untouched[i],
                "E21: C wrote {f} for non-numeric payload {payload:?}"
            );
        }
        // …but these three are always written on the Ver path
        for f in ["os_name", "os_version", "os_platform"] {
            let i = FIELD_NAMES.iter().position(|x| *x == f).unwrap();
            assert!(!out.untouched[i], "E21: C failed to write {f}");
        }
    }
}

#[test]
fn e22_ver_major_only() {
    let b = both();
    for payload in ["10", "0", "6", "007", "4294967296", "1a", "10x.2"] {
        let input = format!("Win [Ver: {payload}]");
        diff_parse(input.as_bytes(), "E22");
        let sent = Sentinels::new("e22");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        let i_minor = FIELD_NAMES.iter().position(|x| *x == "os_minor").unwrap();
        let i_build = FIELD_NAMES.iter().position(|x| *x == "os_build").unwrap();
        assert!(out.untouched[i_minor], "E22: C wrote os_minor for {payload:?}");
        assert!(out.untouched[i_build], "E22: C wrote os_build for {payload:?}");
    }
}

#[test]
fn e23_ver_major_minor_only() {
    let b = both();
    for payload in ["10.0", "1.2", "0.0", "10.0.", "10.0.a", "10.0..", "10.0x.1"] {
        let input = format!("Win [Ver: {payload}]");
        diff_parse(input.as_bytes(), "E23");
        let sent = Sentinels::new("e23");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        let i_build = FIELD_NAMES.iter().position(|x| *x == "os_build").unwrap();
        assert!(out.untouched[i_build], "E23: C wrote os_build for {payload:?}");
    }
}

#[test]
fn e24_nonver_nonnumeric() {
    let b = both();
    for version in [
        "", " ", "abc", "x1", "-1", ".", "..", "LTS", "jammy", "v22.04", " 22.04", "\u{ff}\u{fe}",
    ] {
        let input = format!("host [Ubuntu: {version}]");
        diff_parse(input.as_bytes(), "E24");
        diff_parse_prefilled(input.as_bytes(), "E24/prefilled");
        let sent = Sentinels::new("e24");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        for f in ["os_major", "os_minor"] {
            let i = FIELD_NAMES.iter().position(|x| *x == f).unwrap();
            assert!(out.untouched[i], "E24: C wrote {f} for version {version:?}");
        }
        // os_build is NEVER written on the POSIX path
        let i_build = FIELD_NAMES.iter().position(|x| *x == "os_build").unwrap();
        assert!(out.untouched[i_build], "E24: C wrote os_build on the POSIX path");
    }
}

// ===========================================================================
// E25..E27
// ===========================================================================

#[test]
fn e25_uname_empty() {
    let b = both();
    diff_parse(b"", "E25");
    diff_parse_prefilled(b"", "E25/prefilled");
    let sent = Sentinels::new("e25");
    let out = run_parse(b.c.parse_uname_string, b"", &sent.ptrs);
    assert!(
        out.untouched.iter().all(|x| *x),
        "E25: C wrote something for the empty uname: {:?}",
        out.untouched
    );
    let sentr = Sentinels::new("e25r");
    let outr = run_parse(b.rs.parse_uname_string, b"", &sentr.ptrs);
    assert_eq!(out.untouched, outr.untouched, "E25: untouched mask differs");
    assert_eq!(out.buffer, outr.buffer, "E25: buffer differs");
}

#[test]
fn e26_no_pipe() {
    let b = both();
    for input in [
        "host [Ubuntu: 22.04]",
        "host [Ubuntu]",
        "host [Ubuntu: 22.04 (jammy)]",
        "host [a: b]",
        "host [ ]",
    ] {
        assert!(!input.contains('|'));
        diff_parse(input.as_bytes(), "E26");
        diff_parse_prefilled(input.as_bytes(), "E26/prefilled");
        let sent = Sentinels::new("e26");
        let out = run_parse(b.c.parse_uname_string, input.as_bytes(), &sent.ptrs);
        let i = FIELD_NAMES.iter().position(|x| *x == "os_platform").unwrap();
        assert!(out.untouched[i], "E26: C wrote os_platform for {input:?}");
    }
}

#[test]
fn e27_trailing_pipe() {
    let b = both();
    for input in [
        "host [Ubuntu|: 22.04]",
        "host [Ubuntu|]",
        "host [|: 1]",
        "host [|]",
        "host [a|b|: 1]",
    ] {
        diff_parse(input.as_bytes(), "E27");
        diff_parse_prefilled(input.as_bytes(), "E27/prefilled");
    }
    // "Ubuntu|" then ": " -> os_name "Ubuntu|" -> platform strdup("") == ""
    let out = run_parse_zeroed(b.c.parse_uname_string, b"host [Ubuntu|: 22.04]");
    let i = FIELD_NAMES.iter().position(|x| *x == "os_platform").unwrap();
    assert_eq!(
        out.fields[i],
        Some(Vec::new()),
        "E27: expected an empty os_platform, got {:?}",
        out.fields[i]
    );
    let outr = run_parse_zeroed(b.rs.parse_uname_string, b"host [Ubuntu|: 22.04]");
    assert_eq!(out.fields, outr.fields, "E27: fields differ");
}

// ===========================================================================
// E28 — BRE syntax under REG_EXTENDED
// ===========================================================================

#[test]
fn e28_bre_vs_ere() {
    // Under REG_EXTENDED, `\(`/`\)` are literals and `\{` is a literal, so
    // patterns that are legal BRE may be legal-but-different ERE, or invalid.
    let pats: [&str; 18] = [
        r"\(a\)",
        r"\(a",
        r"a\)",
        r"\(\)",
        r"a\{2\}",
        r"a\{2,3\}",
        r"\{",
        r"\}",
        r"\<a\>",
        r"a\|b",
        r"\+",
        r"\?",
        r"\(a\)\1",
        r"(a)\1",
        r"[[:<:]]",
        r"\w",
        r"\s",
        r"\b",
    ];
    let subjects: [&str; 12] = [
        "", "a", "(a)", "aa", "aaa", "a{2}", "{", "}", "a|b", "+", "?", "ab",
    ];
    let b = both();
    for pat in pats {
        for sub in subjects {
            let ((vc, mc), ec) = capture_stderr("e28c", || {
                call_regexec(b.c.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 3, 6)
            });
            let ((vr, mr), er) = capture_stderr("e28r", || {
                call_regexec(b.rs.w_regexec, Some(pat.as_bytes()), Some(sub.as_bytes()), 3, 6)
            });
            assert_eq!(vc, vr, "E28: return differs for {pat:?}/{sub:?}");
            assert_eq!(mc, mr, "E28: pmatch differs for {pat:?}/{sub:?}");
            assert_eq!(ec, er, "E28: stderr differs for {pat:?}/{sub:?}");
        }
    }
}

// ===========================================================================
// E29 — NULL uname / os_header: the C has no guard (documented UB)
// ===========================================================================

const CHILD_ENV: &str = "DIFF_NULL_TARGET";

#[test]
fn e29_null_uname_both_fault() {
    if let Ok(target) = std::env::var(CHILD_ENV) {
        // Child mode: perform the unguarded call and, if it somehow returns,
        // exit 0.
        let b = both();
        let mut osd = OsData::zeroed();
        unsafe {
            match target.as_str() {
                "c_parse" => (b.c.parse_uname_string)(std::ptr::null_mut(), &mut osd),
                "rs_parse" => (b.rs.parse_uname_string)(std::ptr::null_mut(), &mut osd),
                "c_arch" => {
                    let p = (b.c.get_os_arch)(std::ptr::null_mut());
                    std::process::exit(if p.is_null() { 10 } else { 11 });
                }
                "rs_arch" => {
                    let p = (b.rs.get_os_arch)(std::ptr::null_mut());
                    std::process::exit(if p.is_null() { 10 } else { 11 });
                }
                other => panic!("unknown child target {other}"),
            }
        }
        std::process::exit(20);
    }

    let exe = std::env::current_exe().unwrap();
    let run = |target: &str| -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        let st = std::process::Command::new(&exe)
            .args(["--exact", "e29_null_uname_both_fault", "--nocapture"])
            .env(CHILD_ENV, target)
            .env("RUST_BACKTRACE", "0")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn child");
        (st.code(), st.signal())
    };

    let c_parse = run("c_parse");
    let rs_parse = run("rs_parse");
    assert_eq!(
        c_parse, rs_parse,
        "E29: parse_uname_string(NULL, …) outcome differs: C={c_parse:?} Rust={rs_parse:?}"
    );

    let c_arch = run("c_arch");
    let rs_arch = run("rs_arch");
    assert_eq!(
        c_arch, rs_arch,
        "E29: get_os_arch(NULL) outcome differs: C={c_arch:?} Rust={rs_arch:?}"
    );
}

// ===========================================================================
// Generic boundary sweep required by Phase C beyond the table
// ===========================================================================

#[test]
fn generic_boundaries_off_by_one_and_oversized() {
    let rng = Rng::new(SEED ^ 0xB0);
    // One step past every "documented range" the code has: the component count
    // in a Ver payload, the number of dots, and the length of the numbers.
    for n in 0..=12usize {
        let mut payload = Vec::new();
        for i in 0..n {
            if i > 0 {
                payload.push(b'.');
            }
            payload.extend_from_slice(b"1");
        }
        diff_parse(&cat(&[b"w [Ver: ", &payload, b"]"]), "GB/components");
        diff_parse(&cat(&[b"h [D: ", &payload, b"]"]), "GB/components-posix");
    }
    // Numbers one step past every integer boundary the snprintf/malloc pair
    // could plausibly care about.
    for lit in [
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
    ] {
        diff_parse(format!("w [Ver: {lit}.{lit}.{lit}.{lit}]").as_bytes(), "GB/ints");
        diff_parse(format!("h [D: {lit}.{lit}]").as_bytes(), "GB/ints-posix");
    }
    // A number long enough that match_size is large but still sane.
    let long_num = vec![b'9'; 4096];
    diff_parse(
        &cat(&[b"w [Ver: ", &long_num, b".", &long_num, b".", &long_num, b"]"]),
        "GB/long-numbers",
    );
    // Zero-length and oversized string lengths everywhere.
    for len in [0usize, 1, 2, 3, 6, 7, 8, 9, 1024, 4096] {
        let f = rng.bytes_from(SAFE_ALPHA, len);
        diff_parse(&f, "GB/len");
        diff_parse(&cat(&[&f, b" ["]), "GB/len-bracket");
        diff_parse(&cat(&[&f, b" [Ver: "]), "GB/len-ver");
        diff_parse(&cat(&[b" [", &f]), "GB/len-after-bracket");
        diff_parse(&cat(&[b" [Ver: ", &f]), "GB/len-after-ver");
        diff_arch(&f, "GB/len-arch");
    }
    // Prefixes of every separator token: one byte short and one byte long.
    for tok in [" [Ver: ", " [", ": ", " (", "|"] {
        for take in 0..=tok.len() {
            let p = &tok[..take];
            diff_parse(p.as_bytes(), "GB/sep-prefix");
            diff_parse(format!("a{p}b").as_bytes(), "GB/sep-prefix-embedded");
            diff_parse(format!("a{p}").as_bytes(), "GB/sep-prefix-suffix");
            diff_parse(format!("{p}b").as_bytes(), "GB/sep-prefix-prefix");
        }
    }
    // nmatch boundary values, including values a C caller could pass that have
    // no sensible meaning (the closest thing this API has to an out-of-range
    // enum). Slots are always >= nmatch so the sweep stays in-bounds.
    let b = both();
    for n in [0usize, 1, 2, 3, 127, 128] {
        let (vc, mc) = call_regexec(b.c.w_regexec, Some(b"^(a)(b)?$"), Some(b"a"), n, 128);
        let (vr, mr) = call_regexec(b.rs.w_regexec, Some(b"^(a)(b)?$"), Some(b"a"), n, 128);
        assert_eq!(vc, vr, "GB: w_regexec return differs at nmatch={n}");
        assert_eq!(mc, mr, "GB: w_regexec pmatch differs at nmatch={n}");
    }
    // Empty-string arguments in every position.
    diff_regexec(Some(b""), Some(b""), 0, 4, "GB/empty-empty-0");
    diff_regexec(Some(b""), Some(b""), 1, 4, "GB/empty-empty-1");
    diff_regexec(Some(b""), Some(b""), 4, 4, "GB/empty-empty-4");
    // NULL in every combination, over the full nmatch sweep.
    for n in [0usize, 1, 4, 128] {
        diff_regexec(None, Some(b"x"), n, 128, "GB/null-pat");
        diff_regexec(Some(b"x"), None, n, 128, "GB/null-sub");
        diff_regexec(None, None, n, 128, "GB/null-both");
    }
    let _ = s("");
}
