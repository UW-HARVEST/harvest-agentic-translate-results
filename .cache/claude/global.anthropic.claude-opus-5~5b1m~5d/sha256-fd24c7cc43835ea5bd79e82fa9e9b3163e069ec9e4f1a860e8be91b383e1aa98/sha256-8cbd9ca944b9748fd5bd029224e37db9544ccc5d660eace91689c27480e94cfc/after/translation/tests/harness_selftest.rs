//! Negative controls for the differential harness.
//!
//! A comparison harness that silently compares nothing would make every other
//! test in this crate vacuous. These tests prove that the harness (a) really
//! captures the bytes the loaded `.so` writes to stdout, and (b) really fails
//! when the two sides disagree.

mod common;

use common::*;
use std::ffi::CString;

#[test]
fn s01_capture_sees_the_library_output() {
    let (r, out) = capture(|| complexmode(Impl::C, 1, 20, 22, 0));
    assert_eq!(r, 42);
    assert_eq!(
        out,
        b"Mode 1: Addition\nResult: 42\nOperation performed: addition\n",
        "capture() returned {:?}",
        show(&out)
    );
    // ...and the same for the Rust .so, loaded via dlopen.
    let (r, out) = capture(|| complexmode(Impl::Rust, 1, 20, 22, 0));
    assert_eq!(r, 42);
    assert_eq!(
        out,
        b"Mode 1: Addition\nResult: 42\nOperation performed: addition\n",
        "capture() returned {:?}",
        show(&out)
    );
}

#[test]
fn s02_capture_is_non_empty_for_every_mode() {
    for mode in [1i32, 2, 3, 4, 0, 7] {
        for imp in BOTH {
            let (_, out) = capture(|| complexmode(imp, mode, 3, 4, 5));
            assert!(
                !out.is_empty(),
                "{} complexmode({mode}) produced no captured stdout — the \
                 capture mechanism is broken",
                imp.name()
            );
        }
    }
}

#[test]
fn s03_diff_detects_a_return_value_divergence() {
    // Deliberately feed the two implementations different arguments; the
    // harness must notice.
    let caught = std::panic::catch_unwind(|| {
        diff("negative control (values)", |i| match i {
            Impl::C => check_permissions(Impl::C, 0o644, 0o600),
            Impl::Rust => check_permissions(Impl::Rust, 0o444, 0o600),
        });
    });
    assert!(
        caught.is_err(),
        "diff() accepted a return-value divergence — the harness is broken"
    );
}

#[test]
fn s04_diff_detects_a_stdout_divergence() {
    // mode 1 and mode 4 return the same value for (a,b,c) = (1,2,0) — 3 — but
    // print different text, so only a stdout comparison can tell them apart.
    assert_eq!(
        capture(|| complexmode(Impl::C, 1, 1, 2, 0)).0,
        capture(|| complexmode(Impl::C, 4, 1, 2, 0)).0,
        "precondition: both modes must return the same value here"
    );
    let caught = std::panic::catch_unwind(|| {
        diff("negative control (stdout)", |i| match i {
            Impl::C => complexmode(Impl::C, 1, 1, 2, 0),
            Impl::Rust => complexmode(Impl::Rust, 4, 1, 2, 0),
        });
    });
    assert!(
        caught.is_err(),
        "diff() accepted a stdout-only divergence — the harness is broken"
    );
}

#[test]
fn s05_diff_detects_a_string_out_param_divergence() {
    let a = CString::new("multiply").unwrap();
    let b = CString::new("multiplz").unwrap();
    let caught = std::panic::catch_unwind(|| {
        diff("negative control (out-param)", |i| match i {
            Impl::C => create_result_string(Impl::C, &a, 1),
            Impl::Rust => create_result_string(Impl::Rust, &b, 1),
        });
    });
    assert!(
        caught.is_err(),
        "diff() accepted a returned-string divergence — the harness is broken"
    );
}

#[test]
fn s06_diff_oom_detects_a_divergence() {
    let caught = std::panic::catch_unwind(|| {
        diff_oom("negative control (oom)", |i| match i {
            Impl::C => 1,
            Impl::Rust => 2,
        });
    });
    assert!(
        caught.is_err(),
        "diff_oom() accepted a divergence — the harness is broken"
    );
}

#[test]
fn s07_the_two_libraries_are_distinct_objects() {
    // If both paths resolved to the same file, every differential test would be
    // comparing the C library against itself.
    let cp = std::fs::canonicalize(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src")
            .join("build"),
    )
    .unwrap();
    let rp = std::fs::canonicalize(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target"),
    )
    .unwrap();
    assert!(!rp.starts_with(&cp) && !cp.starts_with(&rp));
    // And the symbol addresses must differ.
    for name in [
        "complexmode",
        "check_permissions",
        "safe_add",
        "create_result_string",
        "multiply_with_log",
        "copy_and_sum",
        "compare_operations",
    ] {
        let mut key = name.as_bytes().to_vec();
        key.push(0);
        let ca = unsafe { libs().c.get::<unsafe extern "C" fn()>(&key) }.unwrap();
        let ra = unsafe { libs().rs.get::<unsafe extern "C" fn()>(&key) }.unwrap();
        assert_ne!(
            *ca as usize, *ra as usize,
            "{name} resolves to the same address in both .so files"
        );
    }
}
