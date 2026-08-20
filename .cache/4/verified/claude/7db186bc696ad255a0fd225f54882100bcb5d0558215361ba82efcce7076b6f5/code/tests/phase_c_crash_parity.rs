//! Phase C — crash-parity tests (ERRORS.md rows 38-40).
//!
//! Three inputs make the C library dereference a NULL pointer. That is a real
//! input an external caller can supply, so the Rust must fail *the same way*.
//! Each call is made in a forked child and the two implementations are compared
//! by how the child terminated (same signal, or both exiting normally).
//!
//! Kept in its own test binary and forced single-threaded, because `fork()` from
//! a multi-threaded harness must do as little as possible in the child.

mod common;
use common::*;
use std::os::raw::c_char;

fn same_death(what: &str, c: Death, r: Death) {
    assert_eq!(
        c, r,
        "{what}: C and Rust terminated differently (C={c:?}, Rust={r:?})"
    );
}

/// Row 38: `parse_uname_string(NULL, &osd)` — `strstr(NULL, " [Ver: ")`.
#[test]
fn row38_null_uname_with_valid_osd() {
    let p = pair();
    let c = run_in_child(|| unsafe {
        let mut osd = OsData::poisoned(0);
        (p.c.parse_uname_string)(std::ptr::null_mut(), &mut osd);
    });
    let r = run_in_child(|| unsafe {
        let mut osd = OsData::poisoned(0);
        (p.rs.parse_uname_string)(std::ptr::null_mut(), &mut osd);
    });
    same_death("parse_uname_string(NULL, &osd)", c, r);
    assert!(
        matches!(c, Death::Signalled(_)),
        "expected the C to die on a NULL uname, got {c:?}"
    );
}

/// Row 39: `get_os_arch(NULL)` — `strstr(NULL, "x86_64")`.
#[test]
fn row39_null_os_header() {
    let p = pair();
    let c = run_in_child(|| unsafe {
        (p.c.get_os_arch)(std::ptr::null_mut());
    });
    let r = run_in_child(|| unsafe {
        (p.rs.get_os_arch)(std::ptr::null_mut());
    });
    same_death("get_os_arch(NULL)", c, r);
    assert!(
        matches!(c, Death::Signalled(_)),
        "expected the C to die on a NULL header, got {c:?}"
    );
}

/// Row 40: `w_regexec` with a compilable pattern, `nmatch > 0` and
/// `pmatch == NULL` — glibc's `regexec` writes through the NULL pointer. Both
/// sides must reach `regexec` and die identically. (With `nmatch == 0`, or with
/// a pattern that fails to compile, neither side crashes — also asserted.)
#[test]
fn row40_null_pmatch_with_nonzero_nmatch() {
    let p = pair();
    // (pattern, subject, expected-to-crash?) — `regexec` only writes through
    // `pmatch` when the match *succeeds*, so a non-matching pattern is a
    // control that must return cleanly on both sides.
    let cases: &[(&[u8], &[u8], bool)] = &[
        (b"^([0-9]+)\\.*\0", b"10.0.1\0", true),
        (b"abc\0", b"xxabcxx\0", true),
        (b"\0", b"anything\0", true),
        (b"^[0-9]+\\.([0-9]+)\\.*\0", b"10.0.19041\0", true),
        (b"abc\0", b"10.0.1\0", false),  // REG_NOMATCH -> no write
        (b"^zzz$\0", b"10.0.1\0", false),
    ];
    for (pat, subj, should_crash) in cases {
        for nm in [1usize, 2, 8] {
            let c = run_in_child(|| unsafe {
                (p.c.w_regexec)(
                    pat.as_ptr() as *const c_char,
                    subj.as_ptr() as *const c_char,
                    nm,
                    std::ptr::null_mut(),
                );
            });
            let r = run_in_child(|| unsafe {
                (p.rs.w_regexec)(
                    pat.as_ptr() as *const c_char,
                    subj.as_ptr() as *const c_char,
                    nm,
                    std::ptr::null_mut(),
                );
            });
            same_death(
                &format!("w_regexec({pat:?}, {subj:?}, nmatch={nm}, NULL pmatch)"),
                c,
                r,
            );
            if *should_crash {
                assert!(
                    matches!(c, Death::Signalled(_)),
                    "expected the C to die for pattern {pat:?} subject {subj:?} nmatch={nm}, got {c:?}"
                );
            } else {
                assert_eq!(
                    c,
                    Death::Exited(0),
                    "pattern {pat:?} does not match {subj:?}, so nothing is written"
                );
            }
        }
    }
}

/// The negative control for row 40: `nmatch == 0` must *not* crash, and an
/// uncompilable pattern must *not* crash even with a NULL `pmatch`, on either
/// side. This proves the fork harness distinguishes crash from clean return.
#[test]
fn row40_negative_controls() {
    let p = pair();
    let cases: &[(&[u8], usize)] = &[
        (b"^([0-9]+)\\.*\0", 0), // nmatch == 0 -> regexec never touches pmatch
        (b"(\0", 0),             // regcomp fails first
        (b"(\0", 4),             // regcomp fails before regexec
        (b"[a-\0", 8),
    ];
    for (pat, nm) in cases {
        let c = run_in_child(|| unsafe {
            (p.c.w_regexec)(
                pat.as_ptr() as *const c_char,
                b"10.0.1\0".as_ptr() as *const c_char,
                *nm,
                std::ptr::null_mut(),
            );
        });
        let r = run_in_child(|| unsafe {
            (p.rs.w_regexec)(
                pat.as_ptr() as *const c_char,
                b"10.0.1\0".as_ptr() as *const c_char,
                *nm,
                std::ptr::null_mut(),
            );
        });
        same_death(&format!("control {pat:?} nmatch={nm}"), c, r);
        assert_eq!(
            c,
            Death::Exited(0),
            "control {pat:?} nmatch={nm} should not crash"
        );
    }

    // NULL pattern / NULL string with a NULL pmatch: short-circuited, no crash.
    for (pat_null, str_null) in [(true, false), (false, true), (true, true)] {
        let c = run_in_child(|| unsafe {
            let pp = if pat_null {
                std::ptr::null()
            } else {
                b"a\0".as_ptr() as *const c_char
            };
            let sp = if str_null {
                std::ptr::null()
            } else {
                b"a\0".as_ptr() as *const c_char
            };
            (p.c.w_regexec)(pp, sp, 8, std::ptr::null_mut());
        });
        let r = run_in_child(|| unsafe {
            let pp = if pat_null {
                std::ptr::null()
            } else {
                b"a\0".as_ptr() as *const c_char
            };
            let sp = if str_null {
                std::ptr::null()
            } else {
                b"a\0".as_ptr() as *const c_char
            };
            (p.rs.w_regexec)(pp, sp, 8, std::ptr::null_mut());
        });
        same_death("control null args", c, r);
        assert_eq!(c, Death::Exited(0));
    }

    // parse_uname_string(NULL, NULL) must not crash on either side (row 21).
    let c = run_in_child(|| unsafe {
        (p.c.parse_uname_string)(std::ptr::null_mut(), std::ptr::null_mut());
    });
    let r = run_in_child(|| unsafe {
        (p.rs.parse_uname_string)(std::ptr::null_mut(), std::ptr::null_mut());
    });
    same_death("parse_uname_string(NULL, NULL)", c, r);
    assert_eq!(c, Death::Exited(0));
}
