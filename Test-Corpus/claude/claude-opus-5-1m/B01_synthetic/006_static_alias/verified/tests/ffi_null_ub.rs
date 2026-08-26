// Phase C — the NULL-pointer boundary (ERRORS.md rows 17-20).
//
// The C code dereferences its pointer arguments unconditionally, so passing NULL
// is undefined behaviour: the process faults. The Rust translation must fault the
// same way instead of, say, returning an error code or panicking with a different
// exit status.
//
// Each case runs in a *separate process* (this test binary re-executes itself
// with DIFFTEST_NULL_CASE set) so a fatal signal can be observed safely, without
// forking a multi-threaded test process.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const ENV: &str = "DIFFTEST_NULL_CASE";

fn run_case(case: &str) {
    let pair = load_pair("nullub");
    let imp = if case.starts_with("c_") { &pair.c } else { &pair.rust };
    // Pre-warm the image so that lazy initialisation happens before the faulting
    // call (keeps the observed signal about the NULL dereference only).
    let mut warm = Argv::new(&[b"driver".as_slice()]);
    let _ = call_main(imp, 1, &mut warm);

    match case.trim_start_matches("c_").trim_start_matches("rust_") {
        "static_alias_null" => {
            let r = unsafe { (imp.static_alias)(std::ptr::null_mut()) };
            println!("no fault: {r:?}");
        }
        "main_null_argv" => {
            let r = unsafe { (imp.main)(3, std::ptr::null_mut()) };
            println!("no fault: {r}");
        }
        "main_null_arg1" => {
            let mut argv: Vec<*mut std::ffi::c_char> = vec![
                b"driver\0".as_ptr() as *mut std::ffi::c_char,
                std::ptr::null_mut(),
                b"3\0".as_ptr() as *mut std::ffi::c_char,
                std::ptr::null_mut(),
            ];
            let r = unsafe { (imp.main)(3, argv.as_mut_ptr()) };
            println!("no fault: {r}");
        }
        "main_null_arg2" => {
            let mut argv: Vec<*mut std::ffi::c_char> = vec![
                b"driver\0".as_ptr() as *mut std::ffi::c_char,
                b"7\0".as_ptr() as *mut std::ffi::c_char,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ];
            let r = unsafe { (imp.main)(3, argv.as_mut_ptr()) };
            println!("no fault: {r}");
        }
        other => panic!("unknown case {other}"),
    }
}

/// (exit code, terminating signal) of a child running `case`.
fn outcome(case: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "null_pointer_boundary_parity", "--nocapture"])
        .env(ENV, case)
        .output()
        .expect("spawn self");
    (out.status.code(), out.status.signal())
}

#[test]
fn null_pointer_boundary_parity() {
    if let Ok(case) = std::env::var(ENV) {
        run_case(&case);
        return;
    }

    for case in [
        "static_alias_null",
        "main_null_argv",
        "main_null_arg1",
        "main_null_arg2",
    ] {
        let c = outcome(&format!("c_{case}"));
        let r = outcome(&format!("rust_{case}"));
        assert_eq!(
            c.1,
            Some(11),
            "{case}: expected the C implementation to die from SIGSEGV, got {c:?}"
        );
        if cfg!(debug_assertions) {
            // The dev profile enables rustc's UB checks, which turn the NULL
            // dereference into a panic (aborting an `extern "C"` frame, SIGABRT)
            // instead of letting the faulting load happen. NULL is undefined
            // behaviour in the C code, so there is no defined result to match;
            // what must hold is that the Rust side also dies abnormally instead
            // of returning a value. Exact signal parity is asserted by the same
            // test in the release profile (see the assert below).
            assert!(
                r.0.is_none() && r.1.is_some(),
                "{case}: Rust did not terminate abnormally: {r:?}"
            );
            eprintln!("{case}: C {c:?}, Rust {r:?} (dev profile: rustc UB check aborts)");
        } else {
            assert_eq!(c, r, "{case}: C (code, signal) = {c:?} but Rust = {r:?}");
            eprintln!("{case}: both faulted with (code, signal) = {c:?}");
        }
    }
}
