//! Phase C — one differential test per row of ERRORS.md (E1–E17), plus the
//! generic FFI boundaries: unreadable/closed file descriptors, zero-length and
//! oversized input, values one step past every documented range, and
//! out-of-range arguments pushed across the ABI.

mod common;

use common::*;

/// Every input in `inputs` must make both implementations behave identically,
/// and (because `scanf`'s result is never checked) must still produce exactly
/// one 9-byte record and exit status 0.
#[track_caller]
fn expect_same_and_zero_status(inputs: &[&[u8]], row: &str) {
    for input in inputs {
        let run = diff_main_input(input);
        assert_eq!(
            run.status,
            Status::Exited(0),
            "{row}: exit status must be 0 for {}",
            preview(input)
        );
        assert_eq!(
            run.out.len(),
            9,
            "{row}: expected exactly one 9-byte record for {}, got {}",
            preview(input),
            preview(&run.out)
        );
    }
}

/// Assert both implementations produce this exact output for this input.
#[track_caller]
fn expect_output(input: &[u8], expect: &str, row: &str) {
    let run = diff_main_input(input);
    assert_eq!(
        as_text(&run.out),
        expect,
        "{row}: unexpected output for {}",
        preview(input)
    );
    assert_eq!(run.status, Status::Exited(0), "{row}: exit status");
}

// ---------------------------------------------------------------------------
// E1 / E2 / E3 — `scanf` input failures
// ---------------------------------------------------------------------------

/// E1 — empty stdin: immediate EOF, `scanf` returns EOF, `x` keeps its `0`.
#[test]
fn e1_empty_stdin_input_failure() {
    expect_output(b"", "00000000\n", "E1");
    expect_same_and_zero_status(&[b""], "E1");
    // Also through a pipe that is closed without any data.
    diff_main(Stdin::Pipe(b""), Stdout::File, "E1 empty pipe");
}

/// E2 — whitespace only, every character class, single and mixed, short and
/// longer than both stdio buffers.
#[test]
fn e2_whitespace_only_input_failure() {
    let singles: [&[u8]; 6] = [b" ", b"\t", b"\n", b"\x0b", b"\x0c", b"\r"];
    for s in singles {
        expect_output(s, "00000000\n", "E2");
    }
    let mixes: [&[u8]; 6] = [
        b"   ",
        b"\n\n\n",
        b" \t\n\x0b\x0c\r",
        b"\r\n",
        b"\t \t \t ",
        b"\x0c\x0b",
    ];
    expect_same_and_zero_status(&mixes, "E2");
    for n in [4_096usize, 8_192, 40_000] {
        let long = vec![b' '; n];
        expect_output(&long, "00000000\n", "E2 long");
        let long_nl = vec![b'\n'; n];
        expect_output(&long_nl, "00000000\n", "E2 long newlines");
    }
}

/// E3 — stdin cannot be read at all: fd 0 closed (`EBADF`) and fd 0 on a
/// directory (`EISDIR`).  Both are `scanf` input failures.
#[test]
fn e3_unreadable_stdin_input_failure() {
    let run = diff_main(Stdin::Closed, Stdout::File, "E3 fd 0 closed");
    assert_eq!(as_text(&run.out), "00000000\n");
    assert_eq!(run.status, Status::Exited(0));

    let run = diff_main(Stdin::Directory, Stdout::File, "E3 fd 0 is a directory");
    assert_eq!(as_text(&run.out), "00000000\n");
    assert_eq!(run.status, Status::Exited(0));
}

// ---------------------------------------------------------------------------
// E4 / E5 / E6 — `scanf` matching failures
// ---------------------------------------------------------------------------

/// E4 — the first non-whitespace byte cannot start a `%d` conversion.
#[test]
fn e4_leading_non_numeric_matching_failure() {
    let cases: [&[u8]; 16] = [
        b"a", b"z", b"A", b".", b",", b"/", b":", b"\\", b"x", b"#", b"\0", b"\x80", b"\xff",
        b"\x08", b"\x0e", b"[",
    ];
    for c in cases {
        expect_output(c, "00000000\n", "E4");
    }
    // Whitespace first, then the offending byte.
    for c in cases {
        let mut input = b"  \t\n ".to_vec();
        input.extend_from_slice(c);
        input.extend_from_slice(b"123");
        expect_output(&input, "00000000\n", "E4 after whitespace");
    }
}

/// E4 (exhaustive classifier check) — every single byte value as the whole
/// input, and as a prefix before `12`.  This pins C's `isspace`/digit
/// classification for all 256 byte values, including the ones just outside the
/// `0x09..=0x0d` whitespace range.
#[test]
fn e4b_all_byte_values_as_prefix() {
    for b in 0u8..=255 {
        diff_main_input(&[b]);
        diff_main_input(&[b, b'1', b'2']);
        diff_main_input(&[b'-', b, b'1']);
        diff_main_input(&[b'+', b, b'1']);
        diff_main_input(&[b'1', b, b'2']);
    }
}

/// E5 — a sign followed by EOF.
#[test]
fn e5_sign_then_eof_matching_failure() {
    let cases: [&[u8]; 6] = [b"-", b"+", b"   -", b"\n+", b"\t\t-", b"\x0b\x0c+"];
    for c in cases {
        expect_output(c, "00000000\n", "E5");
    }
}

/// E6 — a sign followed by a non-digit.
#[test]
fn e6_sign_then_non_digit_matching_failure() {
    let cases: [&[u8]; 12] = [
        b"-a", b"+ 5", b"- 5", b"--5", b"+-5", b"-+5", b"-.", b"-\n5", b"+\t7", b"-\0", b"+\xff",
        b"-x1",
    ];
    for c in cases {
        expect_output(c, "00000000\n", "E6");
    }
}

// ---------------------------------------------------------------------------
// E7 / E8 / E9 — out-of-range conversions
// ---------------------------------------------------------------------------

/// E7 — magnitude above `LONG_MAX`: glibc saturates at `LONG_MAX` and stores
/// `(int)LONG_MAX == -1`.
#[test]
fn e7_positive_overflow_saturates_to_long_max() {
    let cases: [&[u8]; 8] = [
        b"9223372036854775808",
        b"9223372036854775809",
        b"18446744073709551615",
        b"18446744073709551616",
        b"99999999999999999999",
        b"10000000000000000000000000000000000000000",
        b"+9223372036854775808",
        b"340282366920938463463374607431768211456",
    ];
    for c in cases {
        expect_output(c, "ffffffff\n", "E7");
    }
    // A 5000-digit number, and one with leading zeros in front of the overflow.
    let long = format!("{}\n", "9".repeat(5_000));
    expect_output(long.as_bytes(), "ffffffff\n", "E7 5000 nines");
    let padded = format!("{}9223372036854775808\n", "0".repeat(100));
    expect_output(padded.as_bytes(), "ffffffff\n", "E7 padded");
}

/// E8 — magnitude above `LONG_MAX` with a minus sign: saturates at `LONG_MIN`
/// and stores `(int)LONG_MIN == 0`.
#[test]
fn e8_negative_overflow_saturates_to_long_min() {
    let cases: [&[u8]; 6] = [
        b"-9223372036854775809",
        b"-9223372036854775810",
        b"-18446744073709551616",
        b"-99999999999999999999",
        b"-10000000000000000000000000000000000000000",
        b"-340282366920938463463374607431768211456",
    ];
    for c in cases {
        expect_output(c, "00000000\n", "E8");
    }
    let long = format!("-{}\n", "9".repeat(5_000));
    expect_output(long.as_bytes(), "00000000\n", "E8 5000 nines");
}

/// E9 — one step past the `int` range but still inside `long`: no error, the
/// value is silently narrowed to its low 32 bits.
#[test]
fn e9_int_range_overflow_is_silently_narrowed() {
    let cases: [(&[u8], &str); 12] = [
        (b"2147483647", "ffffff7f"),
        (b"2147483648", "00000080"),
        (b"2147483649", "01000080"),
        (b"-2147483648", "00000080"),
        (b"-2147483649", "ffffff7f"),
        (b"-2147483650", "feffff7f"),
        (b"4294967295", "ffffffff"),
        (b"4294967296", "00000000"),
        (b"4294967297", "01000000"),
        (b"9223372036854775807", "ffffffff"),
        (b"-9223372036854775808", "00000000"),
        // 10^12 = 0xE8_D4A5_1000; the low 32 bits are 0xD4A51000.
        (b"1000000000000", "0010a5d4"),
    ];
    for (input, expect) in cases {
        expect_output(input, &format!("{expect}\n"), "E9");
    }
}

// ---------------------------------------------------------------------------
// E10 / E17 — the ignored `scanf` result
// ---------------------------------------------------------------------------

/// E10 — `scanf`'s return value is never inspected: every input, valid or not,
/// yields exactly one record.
#[test]
fn e10_scanf_result_never_checked() {
    let inputs: [&[u8]; 14] = [
        b"",
        b" ",
        b"\n\n",
        b"a",
        b"-",
        b"+",
        b".",
        b"\0",
        b"\xff\xff",
        b"0",
        b"42",
        b"-42",
        b"9223372036854775808",
        b"abc123",
    ];
    expect_same_and_zero_status(&inputs, "E10");
}

/// E17 — the exit status is unconditionally 0, even for the failure rows.
#[test]
fn e17_exit_status_always_zero() {
    let inputs: [&[u8]; 8] = [
        b"",
        b"   ",
        b"junk",
        b"-",
        b"+x",
        b"99999999999999999999999999",
        b"-99999999999999999999999999",
        b"12",
    ];
    for input in inputs {
        let run = diff_main_input(input);
        assert_eq!(run.status, Status::Exited(0), "E17 for {}", preview(input));
    }
    // Same at the process level (the C `main` returns 0, so the shell sees 0).
    for input in inputs {
        let c = run_exe_with_file_stdin(c_exe_path(), input);
        let r = run_exe_with_file_stdin(rust_exe_path(), input);
        assert_eq!((c.status, c.signal), (Some(0), None), "E17 C exe");
        assert_eq!((r.status, r.signal), (Some(0), None), "E17 Rust exe");
    }
}

// ---------------------------------------------------------------------------
// E11 / E12 / E13 — ignored stdout write errors
// ---------------------------------------------------------------------------

/// E11 — fd 1 closed: `printf`'s error is ignored by the C code, so nothing is
/// reported and the process still exits 0.  Checked for both entry points.
#[test]
fn e11_stdout_closed_is_ignored() {
    let p = pair();
    let c = run_child(Stdin::File(b"42\n"), Stdout::Closed, || unsafe {
        (p.c.main)()
    });
    let r = run_child(Stdin::File(b"42\n"), Stdout::Closed, || unsafe {
        (p.rs.main)()
    });
    assert_eq!(c.status, r.status, "E11 main with fd 1 closed");
    assert_eq!(c.status, Status::Exited(0), "E11 main must still exit 0");

    let c = run_child(Stdin::Inherit, Stdout::Closed, || unsafe {
        (p.c.driver)(0x11223344);
        0
    });
    let r = run_child(Stdin::Inherit, Stdout::Closed, || unsafe {
        (p.rs.driver)(0x11223344);
        0
    });
    assert_eq!(c.status, r.status, "E11 driver with fd 1 closed");
    assert_eq!(c.status, Status::Exited(0), "E11 driver must still return");
}

/// E12 — fd 1 is `/dev/full`: the write fails with `ENOSPC` at flush time and
/// the error is ignored.
#[test]
fn e12_stdout_enospc_is_ignored() {
    let p = pair();
    let c = run_child(Stdin::File(b"-7\n"), Stdout::DevFull, || unsafe {
        (p.c.main)()
    });
    let r = run_child(Stdin::File(b"-7\n"), Stdout::DevFull, || unsafe {
        (p.rs.main)()
    });
    assert_eq!(c.status, r.status, "E12 main on /dev/full");
    assert_eq!(c.status, Status::Exited(0), "E12 main must still exit 0");

    let c = run_child(Stdin::Inherit, Stdout::DevFull, || unsafe {
        (p.c.driver)(-1);
        0
    });
    let r = run_child(Stdin::Inherit, Stdout::DevFull, || unsafe {
        (p.rs.driver)(-1);
        0
    });
    assert_eq!(c.status, r.status, "E12 driver on /dev/full");
    assert_eq!(c.status, Status::Exited(0), "E12 driver must still return");
}

/// E13 — fd 1 is a pipe with no reader and `SIGPIPE` has its default (C)
/// disposition: the process must be killed by signal 13, not exit normally.
#[test]
fn e13_stdout_epipe_raises_sigpipe() {
    let p = pair();
    let c = run_child(Stdin::File(b"42\n"), Stdout::ClosedPipe, || unsafe {
        (p.c.main)()
    });
    let r = run_child(Stdin::File(b"42\n"), Stdout::ClosedPipe, || unsafe {
        (p.rs.main)()
    });
    assert_eq!(c.status, r.status, "E13 main on a reader-less pipe");
    assert_eq!(c.status, Status::Signaled(13), "E13 expected SIGPIPE");

    let c = run_child(Stdin::Inherit, Stdout::ClosedPipe, || unsafe {
        (p.c.driver)(7);
        0
    });
    let r = run_child(Stdin::Inherit, Stdout::ClosedPipe, || unsafe {
        (p.rs.driver)(7);
        0
    });
    assert_eq!(c.status, r.status, "E13 driver on a reader-less pipe");
    assert_eq!(c.status, Status::Signaled(13), "E13 expected SIGPIPE");
}

/// E13 (process level) — the same for the real programs: Rust's runtime sets
/// `SIGPIPE` to `SIG_IGN` by default, which would make the translation exit 0
/// where the C program dies.
#[test]
fn e13b_executables_die_from_sigpipe_too() {
    for input in [&b"42\n"[..], b"", b"abc"] {
        let c = run_exe_with_stdout_to_reader_less_pipe(c_exe_path(), input);
        let r = run_exe_with_stdout_to_reader_less_pipe(rust_exe_path(), input);
        assert_eq!(
            c, r,
            "E13b executables diverged on a reader-less stdout pipe for {}",
            preview(input)
        );
        assert_eq!(c, Status::Signaled(13), "E13b expected SIGPIPE");
    }
}

// ---------------------------------------------------------------------------
// E14 / E15 — `print_hex`'s missing validation
// ---------------------------------------------------------------------------

/// E14 + E15 — `print_hex` performs no length or null validation, but it is
/// `static`: it is absent from `nm -D` of the C `.so`, so no FFI caller can
/// reach those paths.  `driver` always passes `&x` and `sizeof(int)`, which the
/// `driver` rows cover.  This test pins the reason the two rows are
/// unreachable, so a future change that exports it fails here.
#[test]
fn e14_e15_print_hex_is_not_reachable_through_ffi() {
    let p = pair();
    for (name, lib) in [("C", c_so_path()), ("Rust", rust_so_path())] {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(lib)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.contains("print_hex"),
            "{name} .so unexpectedly exports print_hex; ERRORS.md rows E14/E15 \
             would become reachable and need real differential tests"
        );
    }
    // The only reachable length is 4, i.e. exactly one 9-byte record per call.
    let c = capture_child(|| unsafe { (p.c.driver)(0) });
    let r = capture_child(|| unsafe { (p.rs.driver)(0) });
    assert_eq!(c.out.len(), 9);
    assert_eq!(r.out.len(), 9);
}

// ---------------------------------------------------------------------------
// E16 — out-of-range arguments across the FFI boundary
// ---------------------------------------------------------------------------

/// E16 — `driver` accepts every `int` bit pattern, so the only invalid input is
/// a *wider* one.  The symbol is called through an `extern "C" fn(i64)`
/// signature with garbage in the upper half (this is the equivalent of passing a
/// C enum an int with no valid variant: the callee must behave exactly like the
/// C callee does).  Also covers the generic "null pointer" boundary: neither
/// exported function takes a pointer, so there is no pointer to nullify — the
/// nearest equivalent is an argument register whose upper bits are junk.
#[test]
fn e16_null_and_wide_arguments() {
    let mut values: Vec<i64> = vec![
        0,
        -1,
        1,
        i32::MAX as i64,
        i32::MAX as i64 + 1,
        i32::MIN as i64,
        i32::MIN as i64 - 1,
        u32::MAX as i64,
        u32::MAX as i64 + 1,
        i64::MAX,
        i64::MIN,
        0x0000_0001_0000_0000,
        0x7fff_ffff_8000_0000u64 as i64,
        -0x1_0000_0000,
    ];
    let mut rng = Rng::new(0xE016);
    for _ in 0..500 {
        values.push(rng.next_u64() as i64);
    }
    diff_driver_batch_wide(&values, "E16 wide arguments");
}

/// E16 (b) — the exported `main` is declared `int main(void)` in C, but a
/// foreign caller can pass arguments anyway (that is what the real C start-up
/// code does for `int main(int, char**)`).  Both implementations must ignore
/// them identically.
#[test]
fn e16b_main_called_with_extra_arguments() {
    use std::os::raw::c_int;
    type MainArgv = unsafe extern "C" fn(c_int, *const *const u8) -> c_int;

    let p = pair();
    // Same symbol, re-declared with the `(argc, argv)` signature.
    let c_main: MainArgv = unsafe { std::mem::transmute(p.c.main) };
    let rs_main: MainArgv = unsafe { std::mem::transmute(p.rs.main) };

    let arg0 = b"driver\0".as_ptr();
    let argv: [*const u8; 2] = [arg0, std::ptr::null()];

    for input in [&b"1234\n"[..], b"", b"junk"] {
        let c = run_child(Stdin::File(input), Stdout::File, || unsafe {
            c_main(1, argv.as_ptr())
        });
        let r = run_child(Stdin::File(input), Stdout::File, || unsafe {
            rs_main(1, argv.as_ptr())
        });
        assert_eq!(
            (as_text(&c.out), c.status),
            (as_text(&r.out), r.status),
            "E16b main(argc, argv) for {}",
            preview(input)
        );
        assert_eq!(c.out.len(), 9);
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries: zero-length and oversized input
// ---------------------------------------------------------------------------

/// Zero-length and very large stdin (1 MiB) — the generic length boundaries.
#[test]
fn generic_zero_length_and_oversized_input() {
    diff_main_input(b"");
    // 1 MiB of digits: far past every buffer size involved.
    let huge = format!("{}\n", "1234567890".repeat(100_000));
    assert!(huge.len() > 1_000_000);
    diff_main_input(huge.as_bytes());
    // 1 MiB of whitespace followed by a number.
    let mut ws = vec![b' '; 1_000_000];
    ws.extend_from_slice(b"-2147483648\n");
    diff_main_input(&ws);
    // 1 MiB of junk.
    let junk = vec![b'q'; 1_000_000];
    diff_main_input(&junk);
}

// ---------------------------------------------------------------------------
// helper used by E13b
// ---------------------------------------------------------------------------

fn run_exe_with_stdout_to_reader_less_pipe(exe: &std::path::Path, input: &[u8]) -> Status {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    // A pipe whose read end is closed before the child writes.
    let (rd, wr) = make_pipe();
    let write_end = unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(wr) };
    close_fd(rd);

    let in_path = scratch_file("epipe-stdin");
    std::fs::write(&in_path, input).unwrap();
    let stdin = std::fs::File::open(&in_path).unwrap();

    let status = Command::new(exe)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(write_end))
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let _ = std::fs::remove_file(&in_path);

    match status.signal() {
        Some(s) => Status::Signaled(s),
        None => Status::Exited(status.code().unwrap_or(-1)),
    }
}

/// E13 (c) — the `SIGPIPE` **disposition is inherited**, it is not the program's
/// choice: a C program started from a parent that ignores `SIGPIPE` gets `EPIPE`
/// and exits 0, while one started normally is killed by signal 13.  Rust's
/// runtime overwrites the disposition with `SIG_IGN` before `main`, so the
/// translation has to restore whatever it inherited (see `src/main.rs`).
#[test]
fn e13c_sigpipe_disposition_is_inherited() {
    use std::io::Seek;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::{Command, Stdio};

    const SIGPIPE: c_int_alias = 13;
    const SIG_DFL: usize = 0;
    const SIG_IGN: usize = 1;
    use std::os::raw::c_int as c_int_alias;
    extern "C" {
        fn signal(signum: c_int_alias, handler: usize) -> usize;
    }

    for inherited in [SIG_DFL, SIG_IGN] {
        for input in [&b"  1234xyz rest\n"[..], b"42\n", b"", b"junk"] {
            let mut outs = Vec::new();
            for exe in [c_exe_path(), rust_exe_path()] {
                // stdout: a pipe whose read end is already closed.
                let (rd, wr) = make_pipe();
                close_fd(rd);
                let write_end =
                    unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(wr) };

                let path = scratch_file("sigpipe-stdin");
                std::fs::write(&path, input).unwrap();
                let mut stdin = std::fs::File::open(&path).unwrap();
                let child_stdin = stdin.try_clone().unwrap();
                let _ = stdin.as_raw_fd();

                let mut cmd = Command::new(exe);
                cmd.stdin(Stdio::from(child_stdin))
                    .stdout(Stdio::from(write_end))
                    .stderr(Stdio::null());
                unsafe {
                    cmd.pre_exec(move || {
                        // Hand the child the disposition under test; `SIG_IGN`
                        // survives `exec`, just like a real parent's would.
                        signal(SIGPIPE, inherited);
                        Ok(())
                    });
                }
                let status = cmd.status().unwrap();
                let offset = stdin.stream_position().unwrap();
                let _ = std::fs::remove_file(&path);
                outs.push((status.code(), status.signal(), offset));
            }
            assert_eq!(
                outs[0],
                outs[1],
                "inherited SIGPIPE = {} diverged for {} (code, signal, fd0 offset)",
                if inherited == SIG_DFL { "SIG_DFL" } else { "SIG_IGN" },
                preview(input)
            );
            // And the C behaviour is the documented one.
            if inherited == SIG_DFL {
                assert_eq!(outs[0].1, Some(13), "SIG_DFL: must die from SIGPIPE");
            } else {
                assert_eq!(outs[0].0, Some(0), "SIG_IGN: must exit 0 instead");
            }
        }
    }
}
