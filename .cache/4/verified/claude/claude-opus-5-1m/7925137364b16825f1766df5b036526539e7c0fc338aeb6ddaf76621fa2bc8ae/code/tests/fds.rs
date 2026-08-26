// Phase B (continued) — CONFIGS.md rows C18–C23: the descriptor-kind axes.
//
// glibc chooses line- vs full-buffering from the *kind* of the stdout
// descriptor, and refills stdin in BUFSIZ chunks; Rust uses a LineWriter and an
// 8 KiB BufReader.  These rows prove the final byte stream is identical anyway.

mod common;

use common::*;
use std::path::Path;

const CASES: &[&str] = &[
    "1 2 3",
    "0 2 3",
    "1 5 3",
    "1 2 9",
    "1 2",
    "1",
    "",
    "   \n\t ",
    "abc",
    "4294967297 2 3",
    "1\n2\n3\n",
];

/// C18 — stdin is a regular (seekable) file vs a pipe.
#[test]
fn c18_stdin_file_vs_pipe() {
    for s in CASES {
        assert_same_cfg(&In::File(s.as_bytes()), Out::Pipe, &[], "C18 stdin file");
        // ...and the same program must produce the same bytes either way.
        let from_file = run_cfg(&c_bin(), &In::File(s.as_bytes()), Out::Pipe, &[]);
        let from_pipe = run_cfg(&c_bin(), &In::Pipe(s.as_bytes()), Out::Pipe, &[]);
        assert_eq!(from_file, from_pipe, "C18 C: file vs pipe stdin for {s:?}");
        let rf = run_cfg(Path::new(RUST_BIN), &In::File(s.as_bytes()), Out::Pipe, &[]);
        let rp = run_cfg(Path::new(RUST_BIN), &In::Pipe(s.as_bytes()), Out::Pipe, &[]);
        assert_eq!(rf, rp, "C18 Rust: file vs pipe stdin for {s:?}");
    }
    let mut rng = Rng::new(0x18);
    for _ in 0..100 {
        let s = format!("{} {} {}", rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same_cfg(&In::File(s.as_bytes()), Out::Pipe, &[], "C18 random via file");
    }
}

/// C19 — stdin is /dev/null (immediate EOF on a character device).
#[test]
fn c19_stdin_dev_null() {
    assert_same_cfg(&In::DevNull, Out::Pipe, &[], "C19 /dev/null stdin");
    assert_same_cfg(&In::Path("/dev/null".into()), Out::Pipe, &[], "C19 /dev/null by path");
}

/// C20 — stdin larger than glibc's BUFSIZ and Rust's 8 KiB StdinLock buffer, so
/// the scan spans several refills.
#[test]
fn c20_large_stdin() {
    for pad in [4096usize, 8191, 8192, 8193, 65536, 200_000] {
        let s = format!("{}1 2 3", " ".repeat(pad));
        assert_same_str(&s, "C20 leading whitespace bigger than a buffer");
        let s = format!("{}{}1{}2{}3", "\n".repeat(pad), " ", "\n".repeat(pad), "\t".repeat(pad));
        assert_same_str(&s, "C20 whitespace runs across refills");
        // A single token whose digits straddle the refill boundary.
        let digits = "0".repeat(pad) + "1";
        assert_same_str(&format!("{digits} 2 3"), "C20 token across refills");
    }
    let mut rng = Rng::new(0x20);
    for _ in 0..40 {
        let pad = 8000 + rng.below(500) as usize;
        let s = format!("{}{} {} {}", " ".repeat(pad), rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same_str(&s, "C20 random near the buffer boundary");
    }
}

/// C21 — stdin delivered one byte at a time (short reads).
#[test]
fn c21_byte_at_a_time_stdin() {
    for s in CASES {
        assert_same_cfg(
            &In::PipeByteAtATime(s.as_bytes()),
            Out::Pipe,
            &[],
            "C21 byte-at-a-time stdin",
        );
    }
    let mut rng = Rng::new(0x21);
    for _ in 0..40 {
        let s = format!("{} {} {}", rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same_cfg(&In::PipeByteAtATime(s.as_bytes()), Out::Pipe, &[], "C21 random slow");
    }
}

/// C22 — stdout is a regular file (fully buffered) vs a pipe vs /dev/null.
#[test]
fn c22_stdout_kinds() {
    for s in CASES {
        assert_same_cfg(&In::Pipe(s.as_bytes()), Out::File, &[], "C22 stdout to file");
        assert_same_cfg(&In::Pipe(s.as_bytes()), Out::DevNull, &[], "C22 stdout to /dev/null");

        // Byte stream must not depend on the descriptor kind, for either program.
        let cf = run_cfg(&c_bin(), &In::Pipe(s.as_bytes()), Out::File, &[]);
        let cp = run_cfg(&c_bin(), &In::Pipe(s.as_bytes()), Out::Pipe, &[]);
        assert_eq!(cf.stdout, cp.stdout, "C22 C: file vs pipe stdout for {s:?}");
        let rf = run_cfg(Path::new(RUST_BIN), &In::Pipe(s.as_bytes()), Out::File, &[]);
        let rp = run_cfg(Path::new(RUST_BIN), &In::Pipe(s.as_bytes()), Out::Pipe, &[]);
        assert_eq!(rf.stdout, rp.stdout, "C22 Rust: file vs pipe stdout for {s:?}");
        assert_eq!(cf.stdout, rf.stdout, "C22 C vs Rust via file for {s:?}");
    }
}

/// C23 — extra argv values, combined with several stdin shapes.
#[test]
fn c23_argv_variants() {
    let argvs: [&[&str]; 5] = [
        &[],
        &["a", "b", "c"],
        &["1", "2", "3"],
        &[""],
        &["--flag=value", "-x"],
    ];
    for args in argvs {
        for s in CASES {
            assert_same_cfg(&In::Pipe(s.as_bytes()), Out::Pipe, args, "C23 argv");
        }
    }
}
