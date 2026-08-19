//! Phase B (rows C21-C23) — process-level differential tests.
//!
//! The C executable built by `c_src/CMakeLists.txt` and the Rust `[[bin]]` are
//! started with the same arguments; stdout, stderr and the exit status must be
//! byte-identical. This covers the composed pipeline (kernel `argv` -> `main`
//! -> exit status), which the in-process `.so` tests cannot reach.

mod common;

use common::*;

/// C21 — random ASCII arguments, argc 2..4.
#[test]
fn cfg_c21_cli_random_ascii() {
    let rng = Rng::new(0x21_0000_0001);
    for _ in 0..250 {
        let s = random_ascii(&rng, rng.range(0, 40) as usize);
        let a = decorate_number(&rng, rng.range(0, 45) as i64);
        let b = decorate_number(&rng, rng.range(0, 45) as i64);
        let args: Vec<Vec<u8>> = match rng.below(3) {
            0 => vec![s],
            1 => vec![s, a],
            _ => vec![s, a, b],
        };
        assert_same_cli(&args);
    }
}

/// C22 — non-UTF-8 `argv[1]` through `OsString`.
#[test]
fn cfg_c22_cli_non_utf8() {
    let rng = Rng::new(0x22_0000_0001);
    let fixed: &[&[u8]] = &[
        b"\xff",
        b"\x80\x81\x82",
        b"\xc3",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"a\xffb\xfec",
    ];
    for f in fixed {
        assert_same_cli(&[f.to_vec()]);
        for start in 0..=f.len() + 1 {
            assert_same_cli(&[f.to_vec(), format!("{start}").into_bytes()]);
            for stop in 0..=f.len() + 1 {
                assert_same_cli(&[
                    f.to_vec(),
                    format!("{start}").into_bytes(),
                    format!("{stop}").into_bytes(),
                ]);
            }
        }
    }
    for _ in 0..150 {
        // never NUL (impossible through execve) but every other byte value
        let s = random_bytes(&rng, rng.range(1, 40) as usize);
        let a = decorate_number(&rng, rng.range(0, 45) as i64);
        assert_same_cli(&[s.clone(), a.clone()]);
        assert_same_cli(&[s, a, decorate_number(&rng, rng.range(0, 45) as i64)]);
    }
}

/// C23 — full CLI fuzz: 0..6 arguments, every numeric form, every string shape,
/// valid and invalid mixed.
#[test]
fn cfg_c23_cli_fuzz() {
    let rng = Rng::new(0x23_0000_0001);
    let mut seen_status = [false; 2];
    for _ in 0..600 {
        let n = rng.below(6); // 0..5 user arguments
        let mut args: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let a = if i == 0 {
                random_string_shape(&rng)
            } else {
                match rng.below(5) {
                    0 => no_conversion_string(&rng),
                    1 => {
                        let mut v = decorate_number(&rng, rng.range(0, 40) as i64);
                        v.extend_from_slice(&random_junk(&rng));
                        v
                    }
                    2 => decorate_number(&rng, -(rng.range(0, 40) as i64)),
                    3 => {
                        let big: &[&[u8]] = &[
                            b"2147483648",
                            b"4294967296",
                            b"9223372036854775808",
                            b"-9223372036854775809",
                            b"99999999999999999999999",
                        ];
                        rng.pick(big).to_vec()
                    }
                    _ => decorate_number(&rng, rng.range(0, 40) as i64),
                }
            };
            args.push(a);
        }
        let out = assert_same_cli(&args);
        match out.code {
            Some(0) => seen_status[0] = true,
            Some(1) => seen_status[1] = true,
            other => panic!("unexpected exit status {other:?}"),
        }
        assert!(out.stderr.is_empty(), "the C program never writes to stderr");
    }
    assert!(
        seen_status[0] && seen_status[1],
        "the fuzz run must reach both success and failure"
    );
}

/// The two executables agree on the "no arguments at all" case too.
#[test]
fn cfg_c23_cli_no_arguments() {
    let out = assert_same_cli(&[]);
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stdout,
        b"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n"
    );
}

/// C27 — a single huge argument (100 000 bytes, close to the kernel's
/// MAX_ARG_STRLEN): exercises writing more than a pipe buffer of output and the
/// `size_t` -> `int` narrowing of `len` at a realistic size.
#[test]
fn cfg_c27_cli_huge_argument() {
    let rng = Rng::new(0x27_0000_0001);
    let big: Vec<u8> = (0..100_000).map(|_| rng.range(1, 255) as u8).collect();
    let out = assert_same_cli(&[big.clone()]);
    assert_eq!(out.code, Some(0));
    assert_eq!(out.stdout.len(), big.len() + 1);

    for (a, b) in [
        (0usize, 100_000usize),
        (1, 99_999),
        (50_000, 50_001),
        (99_999, 100_000),
        (0, 1),
    ] {
        let out = assert_same_cli(&[
            big.clone(),
            format!("{a}").into_bytes(),
            format!("{b}").into_bytes(),
        ]);
        assert_eq!(out.code, Some(0));
        assert_eq!(out.stdout.len(), b - a + 1);
    }
    // start == len, and one past it
    let out = assert_same_cli(&[big.clone(), b"100000".to_vec()]);
    assert_eq!((out.code, out.stdout.as_slice()), (Some(0), &b"\n"[..]));
    let out = assert_same_cli(&[big, b"100001".to_vec()]);
    assert_eq!(out.code, Some(1));
}

// ---------------------------------------------------------------------------
// Write-failure boundaries: the exit status of a CLI program depends on how it
// reacts to a broken pipe and to a closed stdout.
// ---------------------------------------------------------------------------

use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};

extern "C" {
    fn close(fd: std::ffi::c_int) -> std::ffi::c_int;
}

/// Spawn `exe`, read a few bytes of its output, then close the read end of the
/// pipe while it still has data to write. Returns `(exit code, signal)`.
fn run_with_early_closed_pipe(exe: &std::path::Path, arg: &[u8]) -> (Option<i32>, Option<i32>) {
    use std::os::unix::ffi::OsStrExt;
    let mut child = Command::new(exe)
        .arg(std::ffi::OsStr::from_bytes(arg))
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut pipe = child.stdout.take().expect("stdout");
    let mut buf = [0u8; 8];
    let _ = pipe.read_exact(&mut buf);
    drop(pipe); // close the read end: further writes get EPIPE / SIGPIPE
    let st = child.wait().expect("wait");
    (st.code(), st.signal())
}

/// C28 — a broken pipe must kill both programs the same way. The C keeps the
/// default `SIGPIPE` disposition; the Rust runtime sets `SIG_IGN`, so the
/// translation has to restore `SIG_DFL` (regression test for that).
#[test]
fn cfg_c28_cli_broken_pipe() {
    let arg = vec![b'x'; 100_000]; // more than a pipe buffer
    let c = run_with_early_closed_pipe(&c_exe(), &arg);
    let r = run_with_early_closed_pipe(&rust_exe(), &arg);
    assert_eq!(
        c,
        (None, Some(13)),
        "the C program must be killed by SIGPIPE"
    );
    assert_eq!(r, c, "Rust must be killed by SIGPIPE exactly like C");
}

/// C29 — stdout closed before `main` runs: both must still exit with the same
/// status (the output is simply lost).
#[test]
fn cfg_c29_cli_closed_stdout() {
    use std::os::unix::ffi::OsStrExt;
    let cases: &[&[&[u8]]] = &[
        &[b"abcdef"],
        &[b"abcdef", b"2"],
        &[b"abcdef", b"2", b"4"],
        &[b"abcdef", b"9"],      // error path
        &[b"abcdef", b"4", b"2"], // error path
        &[],                     // usage path
    ];
    for args in cases {
        let mut codes = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let mut cmd = Command::new(&exe);
            for a in args.iter() {
                cmd.arg(std::ffi::OsStr::from_bytes(a));
            }
            unsafe {
                cmd.pre_exec(|| {
                    close(1);
                    Ok(())
                });
            }
            let st = cmd.status().expect("status");
            codes.push((st.code(), st.signal()));
        }
        assert_eq!(
            codes[0], codes[1],
            "closed stdout, args={args:?}: C and Rust must agree"
        );
    }
}
