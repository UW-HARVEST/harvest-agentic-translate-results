//! Phase C — one differential test per row of ERRORS.md.
//!
//! Each test constructs the exact invalid input/condition, drives BOTH shared
//! libraries through it (`dlopen` + `dlsym` in a fresh process) and asserts
//! that the rejection is identical: same bytes on stdout **and** the same exit
//! status, not merely "both failed somehow".
//!
//! Rows 14, 15 and 17 need in-process capture of fd 1 and therefore live in
//! `tests/inprocess.rs` (see the row comments there).

mod common;

use common::*;

fn both_main(stdin: &StdinSpec, stdout: StdoutSpec, ctx: &str) -> Outcome {
    let a = artifacts();
    let c = run_symbol(&a.c_so, "main", None, stdin, stdout);
    let r = run_symbol(&a.rust_so, "main", None, stdin, stdout);
    assert_same(ctx, &c, &r);
    // Also make sure the *executables* agree on the same condition.
    let ce = run_exe(&a.c_exe, stdin, stdout);
    let re = run_exe(&a.rust_exe, stdin, stdout);
    assert_same(&format!("{ctx} (executables)"), &ce, &re);
    assert_same(&format!("{ctx} (so vs exe, C side)"), &c, &ce);
    c
}

// ---------------------------------------------------------------------------
// Row 1 — empty stdin: fscanf returns EOF, `data` keeps its ' ' initialiser
// ---------------------------------------------------------------------------

#[test]
fn err01_stdin_empty_eof() {
    let out = both_main(
        &StdinSpec::File(Vec::new()),
        StdoutSpec::File,
        "err01 empty stdin",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "21\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 2 — stdin = /dev/null
// ---------------------------------------------------------------------------

#[test]
fn err02_stdin_devnull() {
    let out = both_main(&StdinSpec::DevNull, StdoutSpec::File, "err02 /dev/null");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "21\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 3 — fd 0 closed: read(2) fails EBADF, fscanf returns EOF
// ---------------------------------------------------------------------------

#[test]
fn err03_stdin_closed_fd() {
    let out = both_main(&StdinSpec::Closed, StdoutSpec::File, "err03 closed stdin");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "21\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 4 — fd 0 is a directory: read(2) fails EISDIR
// ---------------------------------------------------------------------------

#[test]
fn err04_stdin_is_directory() {
    let out = both_main(
        &StdinSpec::Directory,
        StdoutSpec::File,
        "err04 directory stdin",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "21\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 5 — fd 0 is the write end of a pipe: read(2) fails EBADF
// ---------------------------------------------------------------------------

#[test]
fn err05_stdin_not_readable() {
    let out = both_main(
        &StdinSpec::WriteOnlyPipe,
        StdoutSpec::File,
        "err05 write-only stdin",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "21\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 6 — fd 1 closed: printf fails, return value ignored, exit status still 0
// ---------------------------------------------------------------------------

#[test]
fn err06_stdout_closed_fd() {
    let a = artifacts();
    for stdin in [
        StdinSpec::File(Vec::new()),
        StdinSpec::File(vec![b'A']),
        StdinSpec::File(vec![0x7f]),
        StdinSpec::File(vec![0xff]),
    ] {
        let c = run_symbol(&a.c_so, "main", None, &stdin, StdoutSpec::Closed);
        let r = run_symbol(&a.rust_so, "main", None, &stdin, StdoutSpec::Closed);
        assert_same("err06 closed stdout", &c, &r);
        assert!(c.stdout.is_empty(), "nothing can reach a closed fd 1");
        assert_eq!(c.status, Some(0), "C must still exit 0");

        let ce = run_exe(&a.c_exe, &stdin, StdoutSpec::Closed);
        let re = run_exe(&a.rust_exe, &stdin, StdoutSpec::Closed);
        assert_same("err06 closed stdout (executables)", &ce, &re);
        assert_eq!(ce.status, Some(0));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — stdout = /dev/full: writes fail with ENOSPC, still ignored
// ---------------------------------------------------------------------------

#[test]
fn err07_stdout_dev_full() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("skipping err07: /dev/full is not available");
        return;
    }
    let a = artifacts();
    for stdin in [StdinSpec::File(vec![b'A']), StdinSpec::File(Vec::new())] {
        let c = run_symbol(&a.c_so, "main", None, &stdin, StdoutSpec::DevFull);
        let r = run_symbol(&a.rust_so, "main", None, &stdin, StdoutSpec::DevFull);
        assert_same("err07 /dev/full", &c, &r);
        assert_eq!(c.status, Some(0), "C must still exit 0 on ENOSPC");

        let ce = run_exe(&a.c_exe, &stdin, StdoutSpec::DevFull);
        let re = run_exe(&a.rust_exe, &stdin, StdoutSpec::DevFull);
        assert_same("err07 /dev/full (executables)", &ce, &re);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — CHAR_MAX input: data + 1 overflows to CHAR_MIN (gcc's wrap)
// ---------------------------------------------------------------------------

#[test]
fn err08_char_max_overflow_boundary() {
    let out = both_main(
        &StdinSpec::File(vec![0x7f]),
        StdoutSpec::File,
        "err08 CHAR_MAX",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ffffff80\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 9 — 0xff (== -1) + 1 == 0
// ---------------------------------------------------------------------------

#[test]
fn err09_minus_one_wraps_to_zero() {
    let out = both_main(&StdinSpec::File(vec![0xff]), StdoutSpec::File, "err09 0xff");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "00\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 10 — 0x80 (== CHAR_MIN)
// ---------------------------------------------------------------------------

#[test]
fn err10_char_min_input() {
    let out = both_main(
        &StdinSpec::File(vec![0x80]),
        StdoutSpec::File,
        "err10 CHAR_MIN",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ffffff81\n");
    assert_eq!(out.status, Some(0));
}

// ---------------------------------------------------------------------------
// Row 11 — embedded NUL byte
// ---------------------------------------------------------------------------

#[test]
fn err11_nul_byte_input() {
    let out = both_main(
        &StdinSpec::File(vec![0x00]),
        StdoutSpec::File,
        "err11 NUL byte",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "01\n");
    assert_eq!(out.status, Some(0));

    // NUL in the middle of the payload must not truncate anything either.
    let out = both_main(
        &StdinSpec::File(vec![0x41, 0x00, 0x42]),
        StdoutSpec::File,
        "err11 NUL inside",
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
}

// ---------------------------------------------------------------------------
// Row 12 — "%c" does not skip leading whitespace
// ---------------------------------------------------------------------------

#[test]
fn err12_whitespace_not_skipped() {
    for (bytes, want) in [
        (vec![b'\n'], "0b\n"),
        (vec![b' '], "21\n"),
        (vec![b'\t'], "0a\n"),
        (vec![b'\r'], "0e\n"),
        (vec![0x0b], "0c\n"),
        (vec![0x0c], "0d\n"),
        (vec![b'\n', b'A'], "0b\n"),
        (vec![b' ', b' ', b'A'], "21\n"),
    ] {
        let out = both_main(
            &StdinSpec::File(bytes.clone()),
            StdoutSpec::File,
            &format!("err12 stdin={}", hex(&bytes)),
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            want,
            "err12: unexpected C output for {}",
            hex(&bytes)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13 — oversized input: only the first byte is converted
// ---------------------------------------------------------------------------

#[test]
fn err13_oversized_input() {
    let mut rng = Rng::new(0xC0FF_EE00_C0FF_EE00);
    for len in [8 * 1024usize, 64 * 1024, 512 * 1024, 1024 * 1024] {
        let mut bytes = rng.bytes(len);
        bytes[0] = 0x7f; // exercise the overflow boundary at the same time
        let out = both_main(
            &StdinSpec::File(bytes),
            StdoutSpec::File,
            &format!("err13 len={len}"),
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout), "ffffff80\n");
        assert_eq!(out.status, Some(0));
    }
}

// ---------------------------------------------------------------------------
// Row 16 — printHexCharLine with fd 1 closed
// ---------------------------------------------------------------------------

#[test]
fn err16_print_with_closed_stdout() {
    let a = artifacts();
    for v in [0i32, 1, 0x7f, -128, -1, 0x1ff, i32::MIN] {
        let c = run_symbol(
            &a.c_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::Closed,
        );
        let r = run_symbol(
            &a.rust_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::Closed,
        );
        assert_same(&format!("err16 printHexCharLine({v})"), &c, &r);
        assert!(c.stdout.is_empty());
        assert_eq!(c.status, Some(0), "the function must return normally");
    }
}

// ---------------------------------------------------------------------------
// Row 18 — the exit status is unconditionally 0
// ---------------------------------------------------------------------------

#[test]
fn err18_exit_status_always_zero() {
    let a = artifacts();
    let conditions: Vec<(&str, StdinSpec, StdoutSpec)> = vec![
        ("empty", StdinSpec::File(Vec::new()), StdoutSpec::File),
        ("closed stdin", StdinSpec::Closed, StdoutSpec::File),
        ("directory stdin", StdinSpec::Directory, StdoutSpec::File),
        ("write-only stdin", StdinSpec::WriteOnlyPipe, StdoutSpec::File),
        ("closed stdout", StdinSpec::File(vec![b'A']), StdoutSpec::Closed),
        ("devnull stdout", StdinSpec::File(vec![b'A']), StdoutSpec::DevNull),
        ("pipe stdout", StdinSpec::File(vec![b'A']), StdoutSpec::Pipe),
    ];
    for (name, stdin, stdout) in conditions {
        let c = run_symbol(&a.c_so, "main", None, &stdin, stdout);
        let r = run_symbol(&a.rust_so, "main", None, &stdin, stdout);
        assert_eq!(c.status, Some(0), "C exit status for {name}");
        assert_eq!(r.status, Some(0), "Rust exit status for {name}");
        assert_same(&format!("err18 {name}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries every C API has
// ---------------------------------------------------------------------------

/// `printHexCharLine` accepts *any* `int` bit pattern (a C enum/char parameter
/// accepts any integer across the FFI boundary): all of them must behave the
/// same, and none of them may abort/panic on the Rust side.
#[test]
fn generic_wide_and_extreme_arguments_never_diverge() {
    let a = artifacts();
    let mut rng = Rng::new(0x1357_9BDF_1357_9BDF);
    let mut values: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        127,
        128,
        129,
        255,
        256,
        257,
        i32::MAX - 1,
        i32::MAX,
    ];
    values.extend((0..24).map(|_| rng.next_i32()));
    for v in values {
        let c = run_symbol(
            &a.c_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        let r = run_symbol(
            &a.rust_so,
            "printHexCharLine",
            Some(v),
            &StdinSpec::DevNull,
            StdoutSpec::File,
        );
        assert_same(&format!("generic printHexCharLine({v:#010x})"), &c, &r);
        assert_eq!(r.status, Some(0), "Rust must not abort for {v:#010x}");
        assert!(
            r.stderr.is_empty(),
            "Rust must stay silent on stderr, got {:?}",
            String::from_utf8_lossy(&r.stderr)
        );
    }
}

/// `main` takes no arguments in the C source (`int main()`), so extra argv
/// entries must be ignored identically by both executables.
#[test]
fn generic_extra_argv_is_ignored() {
    let a = artifacts();
    for extra in [vec!["x"], vec!["--help"], vec!["a", "b", "c"]] {
        let stdin = StdinSpec::File(vec![b'Q']);
        let c = run_exe_with_args(&a.c_exe, &extra, &stdin, StdoutSpec::File);
        let r = run_exe_with_args(&a.rust_exe, &extra, &stdin, StdoutSpec::File);
        assert_same(&format!("generic argv {extra:?}"), &c, &r);
        assert_eq!(String::from_utf8_lossy(&c.stdout), "52\n");
    }
}

/// Zero-length and one-byte-past-the-end style stdin lengths.
#[test]
fn generic_stdin_length_boundaries() {
    for len in [0usize, 1, 2, 3, 4095, 4096, 4097, 8191, 8192, 8193] {
        let mut rng = Rng::new(0x2468_ACE0_2468_ACE0 ^ len as u64);
        let mut bytes = rng.bytes(len);
        if len > 0 {
            bytes[0] = 0x80; // negative char, exercises the sign extension too
        }
        let out = both_main(
            &StdinSpec::File(bytes),
            StdoutSpec::File,
            &format!("generic stdin len={len}"),
        );
        let want = if len == 0 { "21\n" } else { "ffffff81\n" };
        assert_eq!(String::from_utf8_lossy(&out.stdout), want);
    }
}
