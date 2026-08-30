//! Level 5 — the top-level `driver` program (`mdmain.c` vs `src/main.rs`).
//!
//! This is the only place the two implementations are compared as processes,
//! because `main` combines `atoi` parsing, the helpers' stdout and the final
//! `op=`/`summary=` lines. stdout, stderr and the exit status must all match.

mod common;

use common::{c_bin_path, driver_args, rust_bin_path, show};
use std::process::{Command, Output};

fn run(bin: &std::path::Path, args: &[&std::ffi::OsStr]) -> Output {
    assert!(bin.exists(), "missing {}", bin.display());
    Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()))
}

#[test]
fn driver_output_matches_for_all_inputs() {
    let (cb, rb) = (c_bin_path(), rust_bin_path());
    for args in driver_args() {
        let co = run(&cb, &args);
        let ro = run(&rb, &args);
        assert_eq!(
            co.stdout,
            ro.stdout,
            "stdout for {args:?}:\nC   = {}\nRust= {}",
            show(&co.stdout),
            show(&ro.stdout)
        );
        assert_eq!(co.stderr, ro.stderr, "stderr for {args:?}");
        assert_eq!(
            co.status.code(),
            ro.status.code(),
            "exit status for {args:?}"
        );
    }
}

/// `argc < 3` takes the usage path: `usage: %s A B` on stderr, exit code 2.
/// `argv[0]` differs between the two builds, so it is normalised away.
#[test]
fn driver_usage_path_matches() {
    let (cb, rb) = (c_bin_path(), rust_bin_path());
    for args in [vec![], vec!["only-one"]] {
        let a: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        let co = run(&cb, &a);
        let ro = run(&rb, &a);
        let cn = String::from_utf8_lossy(&co.stderr).replace(&*cb.to_string_lossy(), "PROG");
        let rn = String::from_utf8_lossy(&ro.stderr).replace(&*rb.to_string_lossy(), "PROG");
        assert_eq!(cn, rn, "usage stderr for {args:?}");
        assert_eq!(co.stdout, ro.stdout, "usage stdout for {args:?}");
        assert_eq!(co.status.code(), ro.status.code(), "usage exit for {args:?}");
        assert_eq!(co.status.code(), Some(2), "usage exit should be 2");
    }
}

/// A numeric sweep through `main`, which also pins down `atoi` behaviour.
#[test]
fn driver_output_matches_over_numeric_sweep() {
    let (cb, rb) = (c_bin_path(), rust_bin_path());
    let values = [
        "0",
        "1",
        "-1",
        "2",
        "-3",
        "7",
        "-7",
        "13",
        "-99",
        "32768",
        "65536",
        "2147483647",
        "-2147483648",
        "2147483648",
        "-2147483649",
        "9223372036854775807",
        "-9223372036854775808",
        "9223372036854775808",
        "99999999999999999999",
        "-99999999999999999999",
    ];
    for a in values {
        for b in values {
            let args: Vec<&std::ffi::OsStr> =
                vec![std::ffi::OsStr::new(a), std::ffi::OsStr::new(b)];
            let co = run(&cb, &args);
            let ro = run(&rb, &args);
            assert_eq!(
                co.stdout,
                ro.stdout,
                "stdout for [{a}, {b}]:\nC   = {}\nRust= {}",
                show(&co.stdout),
                show(&ro.stdout)
            );
            assert_eq!(co.stderr, ro.stderr, "stderr for [{a}, {b}]");
            assert_eq!(co.status.code(), ro.status.code(), "exit for [{a}, {b}]");
        }
    }
}

/// `atoi` details: every `isspace` character is skipped, at most one sign is
/// consumed, digits stop at the first non-digit and the rest is ignored.
#[test]
fn driver_matches_for_atoi_edge_forms() {
    let (cb, rb) = (c_bin_path(), rust_bin_path());
    let forms = [
        " 5", "\t5", "\n5", "\x0b5", "\x0c5", "\r5", " \t\n\x0b\x0c\r 5", "+5", "-5", "++5",
        "--5", "+-5", "- 5", "+ 5", "5 6", "5abc", "abc5", "", " ", "+", "-", ".", "0", "-0",
        "+0", "00000000000000000000007", "0009", "1e3", "0x1f", "010", "٣", "５",
        "  -0000000000000000000012xyz", "2147483647abc", "-2147483648abc",
        "18446744073709551616", "-18446744073709551616",
    ];
    for f in forms {
        for other in ["1", "-3"] {
            let args: Vec<&std::ffi::OsStr> =
                vec![std::ffi::OsStr::new(f), std::ffi::OsStr::new(other)];
            let co = run(&cb, &args);
            let ro = run(&rb, &args);
            assert_eq!(
                co.stdout,
                ro.stdout,
                "stdout for [{f:?}, {other}]:\nC   = {}\nRust= {}",
                show(&co.stdout),
                show(&ro.stdout)
            );
            assert_eq!(co.stderr, ro.stderr, "stderr for [{f:?}, {other}]");
            assert_eq!(
                co.status.code(),
                ro.status.code(),
                "exit for [{f:?}, {other}]"
            );
            // and with the operands swapped
            let args: Vec<&std::ffi::OsStr> =
                vec![std::ffi::OsStr::new(other), std::ffi::OsStr::new(f)];
            let co = run(&cb, &args);
            let ro = run(&rb, &args);
            assert_eq!(co.stdout, ro.stdout, "stdout for [{other}, {f:?}]");
            assert_eq!(co.status.code(), ro.status.code(), "exit for [{other}, {f:?}]");
        }
    }
}

/// `main` receives raw bytes, so arguments that are not valid UTF-8 must be
/// parsed (and echoed on the usage path) exactly as C sees them.
#[test]
fn driver_matches_for_non_utf8_arguments() {
    use std::os::unix::ffi::OsStrExt;
    let (cb, rb) = (c_bin_path(), rust_bin_path());
    let raw: [&[u8]; 5] = [
        b"\xff\xfe",
        b"12\xff34",
        b"\xff12",
        b" \x80-7",
        b"-9\xc3",
    ];
    for bytes in raw {
        let a = std::ffi::OsStr::from_bytes(bytes);
        let args = vec![a, std::ffi::OsStr::new("4")];
        let co = run(&cb, &args);
        let ro = run(&rb, &args);
        assert_eq!(
            co.stdout,
            ro.stdout,
            "stdout for {bytes:?}:\nC   = {}\nRust= {}",
            show(&co.stdout),
            show(&ro.stdout)
        );
        assert_eq!(co.stderr, ro.stderr, "stderr for {bytes:?}");
        assert_eq!(co.status.code(), ro.status.code(), "exit for {bytes:?}");
    }
}
