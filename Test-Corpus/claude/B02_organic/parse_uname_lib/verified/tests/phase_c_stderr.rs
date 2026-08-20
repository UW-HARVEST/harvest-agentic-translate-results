//! Phase C — the *diagnostic text* of the `regcomp` failure path
//! (ERRORS.md rows 4-8) must be byte-identical, not merely "both failed".
//!
//! `fd 2` is process-global, so this lives in its own single-test binary.

mod common;
use common::*;

/// Same list as `phase_c_errors.rs`, plus the two accepted oddities so the
/// "prints nothing" case is asserted too.
const BAD_PATTERNS: &[&[u8]] = &[
    b"(",
    b"a(b",
    b"((a)",
    b"a)b(",
    b"[a-",
    b"[",
    b"[^",
    b"[[:alpha:",
    b"a\\",
    b"\\",
    b"*",
    b"a{2,1}",
    b"{1}",
    b"a{",
    b"a{1",
    b"+",
    b"?",
    b"**",
    b"a{100000000000}",
    b"[[:bogus:]]",
    b"[[.nosuch.]]",
    b"[[=x",
    b"[a-\\",
    b"[z-a]",
    // non-ASCII and embedded percent signs: the C passes the pattern straight
    // into `fprintf(..., "%s", pattern)` so a `%` in the pattern must not be
    // re-interpreted.
    b"(%s%d%n",
    b"(\xff\xfe",
    b"(%%",
];

const GOOD_PATTERNS: &[&[u8]] = &[b")", b"a{,}", b"", b"^$", br"^([0-9]+)\.*"];

#[test]
fn regcomp_diagnostic_is_byte_identical() {
    let p = pair();

    for pat in BAD_PATTERNS.iter().chain(GOOD_PATTERNS.iter()) {
        let c_out = capture_stderr("c", || {
            let mut pb = Buf::new(pat);
            let mut sb = Buf::new(b"subject");
            let mut m = vec![RegMatch::sentinel(); 4];
            unsafe {
                (p.c.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr());
            }
        });
        let r_out = capture_stderr("rs", || {
            let mut pb = Buf::new(pat);
            let mut sb = Buf::new(b"subject");
            let mut m = vec![RegMatch::sentinel(); 4];
            unsafe {
                (p.rs.w_regexec)(pb.ptr(), sb.ptr(), 2, m.as_mut_ptr());
            }
        });
        assert_eq!(
            c_out,
            r_out,
            "stderr differs for pattern {pat:?}\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );

        if BAD_PATTERNS.contains(pat) {
            let mut expect = b"Couldn't compile regular expression '".to_vec();
            expect.extend_from_slice(pat);
            expect.extend_from_slice(b"'\n");
            assert_eq!(
                c_out,
                expect,
                "unexpected C diagnostic for {pat:?}: {:?}",
                String::from_utf8_lossy(&c_out)
            );
        } else {
            assert!(
                c_out.is_empty(),
                "pattern {pat:?} should not produce a diagnostic, got {:?}",
                String::from_utf8_lossy(&c_out)
            );
        }
    }
}

/// The NULL short-circuit must print nothing on either side, even for a pattern
/// that would otherwise fail to compile.
#[test]
fn null_short_circuit_prints_nothing() {
    let p = pair();
    let c_out = capture_stderr("cnull", || unsafe {
        let mut m = vec![RegMatch::sentinel(); 4];
        let mut sb = Buf::new(b"s");
        (p.c.w_regexec)(std::ptr::null(), sb.ptr(), 2, m.as_mut_ptr());
        let mut pb = Buf::new(b"(");
        (p.c.w_regexec)(pb.ptr(), std::ptr::null(), 2, m.as_mut_ptr());
        (p.c.w_regexec)(std::ptr::null(), std::ptr::null(), 2, m.as_mut_ptr());
    });
    let r_out = capture_stderr("rsnull", || unsafe {
        let mut m = vec![RegMatch::sentinel(); 4];
        let mut sb = Buf::new(b"s");
        (p.rs.w_regexec)(std::ptr::null(), sb.ptr(), 2, m.as_mut_ptr());
        let mut pb = Buf::new(b"(");
        (p.rs.w_regexec)(pb.ptr(), std::ptr::null(), 2, m.as_mut_ptr());
        (p.rs.w_regexec)(std::ptr::null(), std::ptr::null(), 2, m.as_mut_ptr());
    });
    assert!(c_out.is_empty(), "C printed {:?}", String::from_utf8_lossy(&c_out));
    assert_eq!(c_out, r_out);
}

/// `parse_uname_string` compiles its five patterns internally and they are all
/// valid, so it must never print anything.
#[test]
fn parse_uname_string_prints_nothing() {
    let p = pair();
    let inputs: &[&[u8]] = &[
        b"Win [Ver: 10.0.19041.1]",
        b"Linux x86_64 [Ubuntu|ubuntu: 22.04 (Jammy)]",
        b"host [OS]",
        b"",
        b" [Ver: ",
        b" [",
        b"host [OS: rolling]",
    ];
    let c_out = capture_stderr("cparse", || {
        for i in inputs {
            let mut b = Buf::new(i);
            let mut osd = OsData::poisoned(0);
            unsafe {
                (p.c.parse_uname_string)(b.ptr(), &mut osd);
                for k in 0..9 {
                    free_if_owned(osd.fields[k], 0);
                }
            }
        }
    });
    let r_out = capture_stderr("rsparse", || {
        for i in inputs {
            let mut b = Buf::new(i);
            let mut osd = OsData::poisoned(0);
            unsafe {
                (p.rs.parse_uname_string)(b.ptr(), &mut osd);
                for k in 0..9 {
                    free_if_owned(osd.fields[k], 0);
                }
            }
        }
    });
    assert!(
        c_out.is_empty(),
        "C printed {:?}",
        String::from_utf8_lossy(&c_out)
    );
    assert_eq!(c_out, r_out);
}
