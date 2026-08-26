//! Negative controls for the differential harness itself.
//!
//! A differential test suite is worthless if it compares empty buffers or if it
//! accidentally loads the same shared object twice. These tests prove that the
//! harness observes real output and does detect differences.

mod common;

use std::process::Command;

use common::*;

/// The exact 14 lines the C program prints for 'A' (captured from the C
/// implementation and checked in by hand).
const REFERENCE_A: &[u8] = b"alphanumeric: 8\n\
alphabetic: 1024\n\
lowercase: 0\n\
uppercase: 256\n\
digit: 0\n\
hexadecimal: 4096\n\
control: 0\n\
graphical: 32768\n\
space: 0\n\
blank: 0\n\
printing: 16384\n\
punctuation: 0\n\
to lower: a\n\
to upper: A\n";

#[test]
fn selfcheck_both_libraries_produce_the_reference_output() {
    let l = libs();
    for imp in [&l.c, &l.rust] {
        let run = run_ops(imp, &[Op::Driver(b'A' as i8)], &Cfg::default());
        assert_eq!(
            run.stdout,
            REFERENCE_A,
            "{} produced {}",
            imp.name,
            escape(&run.stdout)
        );
        assert_eq!(run.stdout.len(), 189);
    }
}

#[test]
fn selfcheck_main_reference_output_through_stdin() {
    let l = libs();
    for imp in [&l.c, &l.rust] {
        let run = run_ops(imp, &[Op::Main], &Cfg::stdin_file(b"A"));
        assert_eq!(run.stdout, REFERENCE_A, "{}", imp.name);
        assert_eq!(run.rets, vec![0]);
    }
}

#[test]
fn selfcheck_harness_detects_differences() {
    // Different inputs must produce different captured output -- otherwise the
    // comparison in every other test would be vacuous.
    let l = libs();
    let a = run_ops(&l.c, &[Op::Driver(b'A' as i8)], &Cfg::default());
    let b = run_ops(&l.rust, &[Op::Driver(b'B' as i8)], &Cfg::default());
    assert_ne!(a.stdout, b.stdout, "the harness cannot see the output");

    // ... and the equality used by assert_same is byte-exact.
    let mut tweaked = a.clone();
    tweaked.stdout[0] ^= 1;
    assert_ne!(a, tweaked);
}

#[test]
fn selfcheck_harness_captures_pipe_and_file_identically() {
    let l = libs();
    for imp in [&l.c, &l.rust] {
        let f = run_ops(imp, &[Op::Driver(b'q' as i8)], &Cfg::default());
        let p = run_ops(
            imp,
            &[Op::Driver(b'q' as i8)],
            &Cfg::default().with_stdout(StdoutSpec::Pipe),
        );
        assert_eq!(f.stdout, p.stdout, "{} file vs pipe capture", imp.name);
        assert!(!f.stdout.is_empty());
    }
}

#[test]
fn selfcheck_two_distinct_shared_objects_are_loaded() {
    // The C object imports the glibc ctype tables; the Rust object must not
    // (it carries its own transcribed tables). This proves the two handles are
    // different implementations, not the same file loaded twice.
    let undef = |p: &std::path::Path| -> String {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only"])
            .arg(p)
            .output()
            .expect("nm");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let c = undef(&c_so_path());
    let r = undef(&rust_so_path());
    assert!(c.contains("__ctype_b_loc"), "C .so should import glibc ctype");
    assert!(
        !r.contains("__ctype_b_loc"),
        "Rust .so must not depend on glibc's ctype tables"
    );
    assert_ne!(c_so_path(), rust_so_path());
}

#[test]
fn selfcheck_exit_status_and_signals_are_observed() {
    let l = libs();
    // Normal completion.
    let ok = run_ops(&l.rust, &[Op::Driver(0)], &Cfg::default());
    assert_eq!((ok.exit_code, ok.signal), (Some(0), None));
    // Death by SIGPIPE is visible to the harness.
    for imp in [&l.c, &l.rust] {
        let dead = run_ops(
            imp,
            &[Op::Driver(b'x' as i8)],
            &Cfg::default()
                .with_stdout(StdoutSpec::BrokenPipe)
                .with_default_sigpipe(),
        );
        assert_eq!(dead.signal, Some(13), "{} {dead:?}", imp.name);
    }
}
