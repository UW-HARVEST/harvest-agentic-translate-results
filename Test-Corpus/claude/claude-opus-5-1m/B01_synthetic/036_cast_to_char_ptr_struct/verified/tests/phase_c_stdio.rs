//! Phase C — fidelity of the libc stdio *stream state*.
//!
//! Rows E20–E23 of ERRORS.md. The C code's I/O goes through libc's `stdin` and
//! `stdout` `FILE` objects, which carry observable state beyond the bytes of a
//! single conversion:
//!
//! * `stdin` is shared with anything else in the process that uses C stdio,
//!   including a host that `dlopen`s the library and calls the exported `main`;
//! * glibc's `exit` seeks the descriptor back to the stream's logical position,
//!   so the *next reader* of that descriptor sees the unconsumed remainder;
//! * `stdout` is fully buffered when redirected, so its bytes are ordered
//!   against a host's own `printf`s and are lost if the process `_exit`s.
//!
//! These are exactly the behaviours a `std::io`-based reimplementation cannot
//! reproduce, so each one gets a differential test.

mod common;

use common::*;

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Makes every temporary stdin file unique, so tests running in parallel in
/// this binary cannot share one path.
fn unique(tag: &str) -> std::path::PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "harvest-{tag}-{}-{}.in",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ))
}

/// Runs `exe` with a **file** as stdin and returns (stdout, the descriptor's
/// offset after the process exits, the bytes still unread on it).
///
/// The child's stdin is a dup of this handle, so both share one file offset:
/// whatever the program leaves the descriptor at is what the parent sees.
fn run_with_file_stdin(exe: &Path, content: &[u8]) -> (Vec<u8>, u64, Vec<u8>) {
    let path = unique("offset");
    std::fs::write(&path, content).expect("write stdin file");
    let mut file = std::fs::File::open(&path).expect("open stdin file");
    let child_stdin = file.try_clone().expect("clone stdin fd");

    let out = Command::new(exe)
        .stdin(Stdio::from(child_stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run with a file as stdin");

    let offset = file
        .seek(SeekFrom::Current(0))
        .expect("read the shared file offset");
    let mut leftover = Vec::new();
    file.read_to_end(&mut leftover).expect("read the remainder");
    drop(file);
    let _ = std::fs::remove_file(&path);
    (out.stdout, offset, leftover)
}

/// Same, but the exported `main` of a shared library, invoked by the helper.
fn run_so_with_file_stdin(so: &Path, content: &[u8], mode: &str) -> (Vec<u8>, Vec<u8>, u64, Vec<u8>) {
    let path = unique("offset-so");
    std::fs::write(&path, content).expect("write stdin file");
    let mut file = std::fs::File::open(&path).expect("open stdin file");
    let child_stdin = file.try_clone().expect("clone stdin fd");

    let out = Command::new(helper())
        .arg(so)
        .arg(mode)
        .stdin(Stdio::from(child_stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run the helper with a file as stdin");

    let offset = file
        .seek(SeekFrom::Current(0))
        .expect("read the shared file offset");
    let mut leftover = Vec::new();
    file.read_to_end(&mut leftover).expect("read the remainder");
    drop(file);
    let _ = std::fs::remove_file(&path);
    (out.stdout, out.stderr, offset, leftover)
}

/// E20 — on exit, glibc seeks stdin back to the stream's logical position, so
/// the remainder of the input is still there for the next reader. This is what
/// `{ ./driver; cat; } < file` relies on.
#[test]
fn e20_stdin_offset_restored_at_exit() {
    // (input, the offset the C program leaves the descriptor at)
    let cases: [(&[u8], u64); 11] = [
        (b"42 hello world\n", 2),
        (b"42x rest", 2),
        (b"   42 rest", 5),
        (b"42\nrest", 2),
        (b"-42, rest", 3),
        // The mismatching character is pushed back, so it is *not* consumed.
        (b"- 42 rest", 1),
        (b"abc rest", 0),
        (b"0x10 rest", 1),
        (b"\n\n\n5 rest", 4),
        (b"999999999999999999999999 rest", 24),
        (b"42", 2),
    ];

    for (input, expected_offset) in cases {
        let (c_out, c_off, c_left) = run_with_file_stdin(c_exe(), input);
        let (r_out, r_off, r_left) = run_with_file_stdin(rust_exe(), input);

        assert_eq!(
            c_off,
            expected_offset,
            "E20: the C program left stdin at {c_off}, not the measured {expected_offset}, for {:?}",
            String::from_utf8_lossy(input)
        );
        assert_eq!(
            (&c_out, c_off, &c_left),
            (&r_out, r_off, &r_left),
            "E20: stdin position diverged for {:?}\n  C   : offset={c_off} leftover={:?}\n  \
             Rust: offset={r_off} leftover={:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&c_left),
            String::from_utf8_lossy(&r_left)
        );
    }

    // The same through the exported `main` of each shared library.
    for input in [
        &b"42 hello world\n"[..],
        b"7 8 9",
        b"abc rest",
        b"- 5 rest",
        b"",
    ] {
        let (c_out, _, c_off, c_left) = run_so_with_file_stdin(c_lib(), input, "main");
        let (r_out, _, r_off, r_left) = run_so_with_file_stdin(rust_lib(), input, "main");
        assert_eq!(
            (&c_out, c_off, &c_left),
            (&r_out, r_off, &r_left),
            "E20: the exported main left stdin at a different position for {:?}\n  \
             C   : offset={c_off} leftover={:?}\n  Rust: offset={r_off} leftover={:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&c_left),
            String::from_utf8_lossy(&r_left)
        );
    }
}

/// E21 — the library's `main` reads through the *host's* libc `stdin`, so a host
/// that consumes a number first leaves the rest for the library (and vice
/// versa). A private buffered reader would see an already-drained descriptor.
#[test]
fn e21_stdin_shared_with_the_host() {
    for input in [
        &b"5 7"[..],
        b"5 7 9",
        b"1\n2\n",
        b"12x34",
        b"-5 -7",
        b"5",
        b"",
        b"abc 5",
        b"99999999999999999999 7",
    ] {
        for mode in ["host_scanf_then_main", "main_then_host_scanf"] {
            let c = run_main_via_so_mode(c_lib(), input, mode);
            let r = run_main_via_so_mode(rust_lib(), input, mode);
            assert_eq!(
                c,
                r,
                "E21 [{mode}]: diverged for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    // Spot-check the C's own values so the row cannot pass vacuously: with
    // "5 7", the host takes 5 and the library takes 7.
    let c = run_main_via_so_mode(c_lib(), b"5 7", "host_scanf_then_main");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "07000000030000000000000000000040\n",
        "E21: the C library did not continue where the host's scanf stopped"
    );
    assert_eq!(String::from_utf8_lossy(&c.stderr), "host scanf=1 v=5");

    let c = run_main_via_so_mode(c_lib(), b"5 7", "main_then_host_scanf");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        "05000000030000000000000000000040\n",
        "E21: the C library did not read the first number"
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        "host scanf=1 v=7",
        "E21: the host did not get the number the library left behind"
    );
}

/// E22 — `stdout` is the host's stream too: the bytes are ordered against the
/// host's own `printf`s, and they are dropped if the process leaves through
/// `_exit` without stdio cleanup.
#[test]
fn e22_stdout_shared_with_the_host() {
    for input in [&b"7"[..], b"-1", b"abc", b""] {
        // Ordering: HOST-BEFORE | <library line> | HOST-AFTER
        let c = run_main_via_so_mode(c_lib(), input, "host_printf_around_main");
        let r = run_main_via_so_mode(rust_lib(), input, "host_printf_around_main");
        assert_eq!(
            c,
            r,
            "E22 ordering diverged for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            String::from_utf8_lossy(input)
        );
        assert!(
            c.stdout.starts_with(b"HOST-BEFORE|") && c.stdout.ends_with(b"HOST-AFTER|"),
            "E22: unexpected C ordering: {:?}",
            String::from_utf8_lossy(&c.stdout)
        );

        // Durability: `_exit` discards the buffered line.
        let c = run_main_via_so_mode(c_lib(), input, "main_then_raw_exit");
        let r = run_main_via_so_mode(rust_lib(), input, "main_then_raw_exit");
        assert_eq!(
            c,
            r,
            "E22 _exit durability diverged for stdin {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            String::from_utf8_lossy(input)
        );
        assert!(
            c.stdout.is_empty(),
            "E22: the C library's output survived _exit: {:?}",
            String::from_utf8_lossy(&c.stdout)
        );
    }
}

/// E23 — structural check that the translation really goes through libc stdio
/// rather than a private stream. This is what makes E20–E22 hold, and it is
/// also what makes the *granularity* of the writes match (one `printf` per
/// byte, so two threads calling `driver` interleave the same way they do in C —
/// an interleaving that is racy and therefore asserted structurally rather than
/// byte-wise).
#[test]
fn e23_translation_uses_libc_stdio() {
    let imports = |so: &Path| -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", so.to_str().unwrap()])
            .output()
            .expect("nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| {
                // Drop the "@GLIBC_x.y" version suffix.
                s.split('@').next().unwrap_or(s).to_string()
            }))
            .collect()
    };

    let c = imports(c_lib());
    let r = imports(rust_lib());

    // The C library reads with a scanf-family function and writes with printf
    // (gcc may fold `printf("\n")` into `putchar`).
    assert!(
        c.iter().any(|s| s.contains("scanf")),
        "the C .so does not import a scanf-family symbol: {c:?}"
    );
    assert!(
        c.iter().any(|s| s == "printf" || s == "putchar"),
        "the C .so does not import printf/putchar: {c:?}"
    );

    // The Rust library must use the same libc entry points.
    assert!(
        r.iter().any(|s| s.contains("scanf")),
        "the Rust .so does not read through libc's scanf, so it cannot share \
         the host's stdin state (see ERRORS.md rows E20/E21): {r:?}"
    );
    assert!(
        r.iter().any(|s| s == "printf" || s == "putchar"),
        "the Rust .so does not write through libc's printf, so it cannot share \
         the host's stdout buffer (see ERRORS.md row E22): {r:?}"
    );
}

/// Extra: a partially consumed stdin handed to a *second* program, which is the
/// user-visible form of E20 (`{ driver; cat; } < file`).
#[test]
fn e24_next_reader_sees_the_remainder() {
    for (input, remainder) in [
        (&b"42 hello world\n"[..], &b" hello world\n"[..]),
        (b"7 rest of it", b" rest of it"),
        (b"abc def", b"abc def"),
        (b"5", b""),
    ] {
        let (_, _, c_left) = run_with_file_stdin(c_exe(), input);
        let (_, _, r_left) = run_with_file_stdin(rust_exe(), input);
        assert_eq!(
            c_left,
            remainder,
            "E24: the C program left {:?}, not the expected {:?}",
            String::from_utf8_lossy(&c_left),
            String::from_utf8_lossy(remainder)
        );
        assert_eq!(
            c_left,
            r_left,
            "E24: the next reader would see different bytes\n  after C   : {:?}\n  \
             after Rust: {:?}",
            String::from_utf8_lossy(&c_left),
            String::from_utf8_lossy(&r_left)
        );
    }
}

/// Silences the unused-import warning for `Write`, which the helpers above do
/// not need but `common` does.
#[allow(dead_code)]
fn _unused(mut w: impl Write) {
    let _ = w.write_all(b"");
}
