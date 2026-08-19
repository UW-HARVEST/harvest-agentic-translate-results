//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! Every test builds the exact invalid input/condition, calls BOTH libraries
//! through their exported symbols (loaded with `libloading` by
//! `examples/so_runner.rs`) and asserts that they fail (or silently recover) in
//! exactly the same way: same stdout bytes, same exit code, same signal.

mod harness;
use harness::*;

/// Helper: run one `main()` case on both libraries and additionally pin down
/// the exact expected observable result of the C reference.
fn assert_main_eq(label: &str, kind: StdinKind<'_>, expect_stdout: &[u8]) {
    let c = run_main(c_so(), &format!("{label}.c"), kind);
    let r = run_main(rust_so(), &format!("{label}.rs"), kind);
    assert_eq!(
        c.observable(),
        r.observable(),
        "[{label}] divergence\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(
        c.stdout, expect_stdout,
        "[{label}] the C reference itself changed behaviour: {c:?}"
    );
    assert_eq!(c.code, Some(0), "[{label}] C exit code: {c:?}");
    assert_eq!(c.signal, None, "[{label}] C died from a signal: {c:?}");
    assert!(c.stderr.is_empty(), "[{label}] C stderr: {c:?}");
    assert!(r.stderr.is_empty(), "[{label}] Rust stderr: {r:?}");
}

fn assert_driver_eq(label: &str, s1: &[u8], s2: &[u8], expect_stdout: &[u8]) {
    let cases = vec![(s1.to_vec(), s2.to_vec())];
    let c = run_driver_batch(c_so(), &format!("{label}.c"), &cases);
    let r = run_driver_batch(rust_so(), &format!("{label}.rs"), &cases);
    assert_eq!(
        c.observable(),
        r.observable(),
        "[{label}] divergence for s1={:?} s2={:?}\n  C   : {c:?}\n  Rust: {r:?}",
        hex(s1),
        hex(s2)
    );
    assert_eq!(
        c.stdout, expect_stdout,
        "[{label}] the C reference itself changed behaviour: {c:?}"
    );
    assert!(r.stderr.is_empty(), "[{label}] Rust stderr: {r:?}");
}

// ---------------------------------------------------------------------------
// E1 — EOF before any byte for the first fgets (fgets returns NULL, ignored)
// ---------------------------------------------------------------------------
#[test]
fn err_e1_empty_stdin() {
    assert_main_eq("e1_file", StdinKind::File(b""), b"0\n");
    assert_main_eq("e1_pipe", StdinKind::Pipe(b""), b"0\n");
    assert_main_eq("e1_devnull", StdinKind::DevNull, b"0\n");
}

// ---------------------------------------------------------------------------
// E2 — EOF for the second fgets: s2 stays "", result is strlen(s1)
// ---------------------------------------------------------------------------
#[test]
fn err_e2_single_line() {
    assert_main_eq("e2_nl", StdinKind::File(b"abcdef\n"), b"6\n");
    // no trailing newline: the chop eats a real byte -> 5
    assert_main_eq("e2_nonl", StdinKind::File(b"abcdef"), b"5\n");
    assert_main_eq("e2_pipe", StdinKind::Pipe(b"abcdef\n"), b"6\n");
}

// ---------------------------------------------------------------------------
// E3 — file descriptor 0 closed: fgets fails (EBADF), nothing is read
// ---------------------------------------------------------------------------
#[test]
fn err_e3_stdin_closed() {
    // through the .so
    assert_main_eq("e3_so", StdinKind::Closed, b"0\n");
    // and through the two executables
    let c = run_exe_kind(c_exe(), "e3_c", StdinKind::Closed);
    let r = run_exe_kind(rust_exe(), "e3_r", StdinKind::Closed);
    assert_eq!(
        c.observable(),
        r.observable(),
        "closed-stdin divergence\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(c.stdout, b"0\n");
}

// ---------------------------------------------------------------------------
// E4 — stdin is a directory: read() fails with EISDIR
// ---------------------------------------------------------------------------
#[test]
fn err_e4_stdin_is_dir() {
    assert_main_eq("e4_so", StdinKind::Directory, b"0\n");
    let c = run_exe_kind(c_exe(), "e4_c", StdinKind::Directory);
    let r = run_exe_kind(rust_exe(), "e4_r", StdinKind::Directory);
    assert_eq!(
        c.observable(),
        r.observable(),
        "directory-stdin divergence\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(c.stdout, b"0\n");
}

// ---------------------------------------------------------------------------
// E5 / E6 — `s[strlen(s)-1] = '\0'` with strlen == 0: out-of-bounds write at
// index (size_t)-1. Must not crash and must not change the visible result.
// ---------------------------------------------------------------------------
#[test]
fn err_e5_oob_write_s1() {
    // s1 empty because stdin is empty
    assert_main_eq("e5_a", StdinKind::File(b""), b"0\n");
    // s1 empty because its first byte is NUL, s2 non-empty
    assert_main_eq("e5_b", StdinKind::File(b"\0abc\nxyz\n"), b"0\n");
    // s1 empty, s2 = "xyz" (chop of "xyz\n")
    assert_main_eq("e5_c", StdinKind::File(b"\0\nxyz\n"), b"0\n");
}

#[test]
fn err_e6_oob_write_s2() {
    // s2 empty because there is no second line
    assert_main_eq("e6_a", StdinKind::File(b"abc\n"), b"3\n");
    // s2 empty because its first byte is NUL
    assert_main_eq("e6_b", StdinKind::File(b"abc\n\0xyz\n"), b"3\n");
    // both empty
    assert_main_eq("e6_c", StdinKind::File(b"\0\n\0\n"), b"0\n");
    // s1 filled to the brim (99 bytes, no newline) so that the s2 OOB write
    // lands next to a full buffer
    let mut input = vec![b'a'; 99];
    input.push(b'\n');
    assert_main_eq("e6_d", StdinKind::File(&input), b"98\n");
}

// ---------------------------------------------------------------------------
// E7 — line longer than sizeof(s1)-1 == 99: fgets truncation boundary
// ---------------------------------------------------------------------------
#[test]
fn err_e7_line_over_99() {
    // 100 'a' + newline: s1 = 99 'a' (no \n) -> chop -> 98 'a';
    // s2 = "a\n" -> chop -> "a"  => strcspn = 0
    let mut input = vec![b'a'; 100];
    input.push(b'\n');
    assert_main_eq("e7_a", StdinKind::File(&input), b"0\n");

    // 99 'a' then 51 'b' then newline: s1 = 99 'a' -> 98 'a';
    // s2 = 51 'b' -> 51 'b' minus the chopped byte? (no \n in s2's 99-byte
    // window: "bbb...b\n" *does* contain the newline) => s2 = 51 'b'
    let mut input = vec![b'a'; 99];
    input.extend_from_slice(&vec![b'b'; 51]);
    input.push(b'\n');
    assert_main_eq("e7_b", StdinKind::File(&input), b"98\n");

    // exactly 99 bytes + newline: the newline does not fit into the first
    // fgets, so it becomes the whole second line
    let mut input = vec![b'x'; 99];
    input.push(b'\n');
    assert_main_eq("e7_c", StdinKind::File(&input), b"98\n");

    // one byte below the boundary: 98 bytes + newline fits completely
    let mut input = vec![b'x'; 98];
    input.push(b'\n');
    assert_main_eq("e7_d", StdinKind::File(&input), b"98\n");
}

// ---------------------------------------------------------------------------
// E8 — no trailing newline: the chop deletes real data
// ---------------------------------------------------------------------------
#[test]
fn err_e8_no_trailing_newline() {
    assert_main_eq("e8_a", StdinKind::File(b"abcdef\ncd"), b"2\n");
    assert_main_eq("e8_b", StdinKind::File(b"abcdef\nc"), b"6\n"); // s2 = "" after chop
    assert_main_eq("e8_c", StdinKind::File(b"a"), b"0\n"); // s1 = "" after chop
}

// ---------------------------------------------------------------------------
// E9 — embedded NUL byte: strlen stops early
// ---------------------------------------------------------------------------
#[test]
fn err_e9_embedded_nul() {
    // "ab\0cd" -> strlen 2 -> chop -> "a"; s2 = "cb" => strcspn("a","cb") = 1
    assert_main_eq("e9_a", StdinKind::File(b"ab\0cd\ncb\n"), b"1\n");
    // NUL in line 2: s2 = "x\0c" -> strlen 1 -> chop -> "" => strlen(s1) = 6
    assert_main_eq("e9_b", StdinKind::File(b"abcdef\nx\0c\n"), b"6\n");
    // NUL right before the newline
    assert_main_eq("e9_c", StdinKind::File(b"abc\0\nc\n"), b"2\n");
}

// ---------------------------------------------------------------------------
// E10 — leading NUL: strlen == 0 -> the E5/E6 OOB path
// ---------------------------------------------------------------------------
#[test]
fn err_e10_leading_nul() {
    assert_main_eq("e10_a", StdinKind::File(b"\0abc\nxyz\n"), b"0\n");
    assert_main_eq("e10_b", StdinKind::File(b"abc\n\0xyz\n"), b"3\n");
    assert_main_eq("e10_c", StdinKind::File(b"\0\n\0\n"), b"0\n");
    assert_main_eq("e10_d", StdinKind::File(b"\0"), b"0\n");
}

// ---------------------------------------------------------------------------
// E11/E12/E13 — NULL pointers through the FFI boundary: strcspn faults
// ---------------------------------------------------------------------------
fn assert_null_parity(which: &str, other: &[u8]) {
    let c = run_driver_null(c_so(), which, other);
    let r = run_driver_null(rust_so(), which, other);
    assert_eq!(
        c.observable(),
        r.observable(),
        "NULL {which} divergence\n  C   : {c:?}\n  Rust: {r:?}"
    );
    assert_eq!(
        c.signal,
        Some(11),
        "expected the C reference to die from SIGSEGV: {c:?}"
    );
    assert!(c.stdout.is_empty() && r.stdout.is_empty());
}

#[test]
fn err_e11_null_s1() {
    assert_null_parity("s1", b"abc");
    assert_null_parity("s1", b"");
}

#[test]
fn err_e12_null_s2() {
    assert_null_parity("s2", b"abc");
    assert_null_parity("s2", b"");
}

#[test]
fn err_e13_null_both() {
    assert_null_parity("both", b"");
}

/// Generic boundary: a non-NULL but invalid pointer (address 1).
#[test]
fn err_e13b_bogus_pointer() {
    for which in ["s1", "s2", "both"] {
        let c = run_driver_bogus(c_so(), which, b"abc");
        let r = run_driver_bogus(rust_so(), which, b"abc");
        assert_eq!(
            c.observable(),
            r.observable(),
            "bogus-pointer {which} divergence\n  C   : {c:?}\n  Rust: {r:?}"
        );
        assert_eq!(c.signal, Some(11), "expected SIGSEGV from C: {c:?}");
    }
}

// ---------------------------------------------------------------------------
// E14/E15 — zero-length operands
// ---------------------------------------------------------------------------
#[test]
fn err_e14_empty_s1() {
    assert_driver_eq("e14_a", b"", b"", b"0\n");
    assert_driver_eq("e14_b", b"", b"abc", b"0\n");
    assert_driver_eq("e14_c", b"", b"\xff\x80", b"0\n");
}

#[test]
fn err_e15_empty_s2() {
    assert_driver_eq("e15_a", b"abc", b"", b"3\n");
    assert_driver_eq("e15_b", b"a", b"", b"1\n");
    let long = vec![b'q'; 300];
    assert_driver_eq("e15_c", &long, b"", b"300\n");
}

// ---------------------------------------------------------------------------
// E16 — oversized operands (way past the program's own 100-byte buffers)
// ---------------------------------------------------------------------------
#[test]
fn err_e16_oversized() {
    for size in [4096usize, 65536] {
        let s1 = vec![b'a'; size];
        assert_driver_eq(
            &format!("e16_{size}_nomatch"),
            &s1,
            b"Z",
            format!("{size}\n").as_bytes(),
        );
        let mut s1b = s1.clone();
        s1b[size - 1] = b'Z';
        assert_driver_eq(
            &format!("e16_{size}_last"),
            &s1b,
            b"Z",
            format!("{}\n", size - 1).as_bytes(),
        );
        // oversized reject set as well
        let s2 = vec![b'b'; size];
        assert_driver_eq(&format!("e16_{size}_bigs2"), &s1, &s2, format!("{size}\n").as_bytes());
    }
}

// ---------------------------------------------------------------------------
// E17 — bytes >= 0x80 (signed `char` hazard)
// ---------------------------------------------------------------------------
#[test]
fn err_e17_high_bytes() {
    assert_driver_eq("e17_a", b"\xff", b"\xff", b"0\n");
    assert_driver_eq("e17_b", b"\x80\x81\x82", b"\x82", b"2\n");
    assert_driver_eq("e17_c", b"abc\xff", b"\xff", b"3\n");
    assert_driver_eq("e17_d", b"\xff\xfe", b"\x7f", b"2\n");
    // every high byte, matching itself
    for b in 0x80u16..=0xff {
        let b = b as u8;
        assert_driver_eq(&format!("e17_{b}"), &[b'a', b'b', b], &[b], b"2\n");
    }
}

// ---------------------------------------------------------------------------
// E18 — interior NUL in the operands passed straight to `driver`
// ---------------------------------------------------------------------------
#[test]
fn err_e18_interior_nul() {
    assert_driver_eq("e18_a", b"abc\0def", b"d", b"3\n");
    assert_driver_eq("e18_b", b"abc\0def", b"c", b"2\n");
    assert_driver_eq("e18_c", b"abc", b"d\0c", b"3\n");
    assert_driver_eq("e18_d", b"\0abc", b"a", b"0\n");
    assert_driver_eq("e18_e", b"abc", b"\0", b"3\n");
}

// ---------------------------------------------------------------------------
// E19 — surplus input lines are silently ignored
// ---------------------------------------------------------------------------
#[test]
fn err_e19_surplus_lines() {
    assert_main_eq("e19_a", StdinKind::File(b"abc\ndef\nghi\n"), b"3\n");
    assert_main_eq("e19_b", StdinKind::File(b"abc\ncd\nghi\njkl\n"), b"2\n");
    assert_main_eq("e19_c", StdinKind::Pipe(b"abc\ndef\nghi\n"), b"3\n");
}

// ---------------------------------------------------------------------------
// E20 — there is no failure exit path: the status is always 0
// ---------------------------------------------------------------------------
#[test]
fn err_e20_exit_code_always_zero() {
    let inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"abc".to_vec(),
        b"abc\n".to_vec(),
        b"abc\ndef\n".to_vec(),
        b"\0\n\0\n".to_vec(),
        vec![b'a'; 500],
    ];
    for (i, input) in inputs.iter().enumerate() {
        let c = run_main(c_so(), &format!("e20_{i}.c"), StdinKind::File(input));
        let r = run_main(rust_so(), &format!("e20_{i}.rs"), StdinKind::File(input));
        assert_eq!(c.code, Some(0), "C exit code for case {i}: {c:?}");
        assert_eq!(r.code, Some(0), "Rust exit code for case {i}: {r:?}");
        assert_eq!(c.observable(), r.observable(), "case {i}: {c:?} vs {r:?}");
    }
    // the same through the executables
    for (i, input) in inputs.iter().enumerate() {
        let c = run_exe(c_exe(), &format!("e20x_{i}.c"), input);
        let r = run_exe(rust_exe(), &format!("e20x_{i}.rs"), input);
        assert_eq!(c.code, Some(0), "C exe exit code for case {i}: {c:?}");
        assert_eq!(r.code, Some(0), "Rust exe exit code for case {i}: {r:?}");
        assert_eq!(c.observable(), r.observable(), "exe case {i}: {c:?} vs {r:?}");
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries that every C API has
// ---------------------------------------------------------------------------

/// A pointer to a buffer whose only byte is the terminator (zero length) and a
/// pointer one past the end of a string (empty string) — both are valid C
/// strings and must be accepted identically.
#[test]
fn err_generic_zero_length_pointers() {
    assert_driver_eq("g_zero_a", b"", b"", b"0\n");
    assert_driver_eq("g_zero_b", b"\0", b"\0", b"0\n"); // interior NUL first byte
}

/// Values one step past the interesting ranges: byte 0x00 (terminator), 0x01
/// (lowest legal byte), 0x7f/0x80 (sign boundary), 0xff (highest byte).
#[test]
fn err_generic_byte_range_edges() {
    for b in [0x01u8, 0x7f, 0x80, 0xff] {
        assert_driver_eq(&format!("g_edge_{b}"), &[b], &[b], b"0\n");
        assert_driver_eq(&format!("g_edge_n_{b}"), &[b], &[b.wrapping_add(1) | 1], b"1\n");
    }
    // a NUL byte inside the reject set terminates it: nothing is rejected
    assert_driver_eq("g_edge_nul_reject", b"abc", b"\0abc", b"3\n");
}

/// The `main` entry point does not take arguments, so there is no enum or
/// integer parameter that could be out of range; the pointer-shaped inputs of
/// `driver` are covered by the NULL/bogus-pointer tests above. This test pins
/// the remaining boundary: a stdin payload exactly at, one below and one above
/// every internal size constant (100, 99, 98).
#[test]
fn err_generic_size_constants() {
    for len in [0usize, 1, 97, 98, 99, 100, 101, 197, 198, 199, 200, 201] {
        let mut input = vec![b'a'; len];
        input.push(b'\n');
        input.extend_from_slice(b"a\n");
        let label = format!("g_size_{len}");
        let c = run_main(c_so(), &format!("{label}.c"), StdinKind::File(&input));
        let r = run_main(rust_so(), &format!("{label}.rs"), StdinKind::File(&input));
        assert_eq!(
            c.observable(),
            r.observable(),
            "[{label}] divergence\n  C   : {c:?}\n  Rust: {r:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// E21 — stdout is a pipe with no reader: `printf` gets EPIPE and the process is
// killed by SIGPIPE (the C default disposition). A Rust binary starts with
// SIGPIPE ignored, so the translation must restore the C default.
// ---------------------------------------------------------------------------
#[test]
fn err_e21_broken_stdout_pipe_executables() {
    for input in [&b"abcdef\ncd\n"[..], b"", b"abc"] {
        let c = run_exe_stdout(c_exe(), "e21_c", StdinKind::File(input), StdoutKind::BrokenPipe);
        let r = run_exe_stdout(rust_exe(), "e21_r", StdinKind::File(input), StdoutKind::BrokenPipe);
        assert_eq!(
            c.observable(),
            r.observable(),
            "broken-stdout-pipe divergence for {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(input)
        );
        assert_eq!(c.signal, Some(13), "expected the C exe to die from SIGPIPE: {c:?}");
    }
}

/// The same at the `.so` level: with the default SIGPIPE disposition restored by
/// the host, both libraries must be killed by SIGPIPE; with SIGPIPE ignored
/// (a plain Rust host) both must survive and exit 0.
#[test]
fn err_e21b_broken_stdout_pipe_shared_objects() {
    for reset in [true, false] {
        let c = run_main_stdout(
            c_so(),
            "e21b_c",
            StdinKind::File(b"abcdef\ncd\n"),
            StdoutKind::BrokenPipe,
            reset,
        );
        let r = run_main_stdout(
            rust_so(),
            "e21b_r",
            StdinKind::File(b"abcdef\ncd\n"),
            StdoutKind::BrokenPipe,
            reset,
        );
        assert_eq!(
            c.observable(),
            r.observable(),
            "broken-pipe .so divergence (reset_sigpipe={reset})\n  C   : {c:?}\n  Rust: {r:?}"
        );
        if reset {
            assert_eq!(c.signal, Some(13), "expected SIGPIPE from the C .so: {c:?}");
        } else {
            assert_eq!(c.code, Some(0), "expected a clean exit from the C .so: {c:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// E22 — stdout is /dev/full: the write fails with ENOSPC. The C code ignores
// the printf return value, so it exits 0 with no output.
// ---------------------------------------------------------------------------
#[test]
fn err_e22_stdout_enospc() {
    for input in [&b"abcdef\ncd\n"[..], b""] {
        let c = run_exe_stdout(c_exe(), "e22_c", StdinKind::File(input), StdoutKind::Full);
        let r = run_exe_stdout(rust_exe(), "e22_r", StdinKind::File(input), StdoutKind::Full);
        assert_eq!(
            c.observable(),
            r.observable(),
            "/dev/full divergence for {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(input)
        );
        assert_eq!(c.code, Some(0), "C exit code on ENOSPC: {c:?}");
        assert!(r.stderr.is_empty(), "Rust stderr on ENOSPC: {r:?}");
    }
    let c = run_main_stdout(c_so(), "e22b_c", StdinKind::File(b"abc\n"), StdoutKind::Full, false);
    let r = run_main_stdout(rust_so(), "e22b_r", StdinKind::File(b"abc\n"), StdoutKind::Full, false);
    assert_eq!(c.observable(), r.observable(), "C: {c:?} Rust: {r:?}");
}

// ---------------------------------------------------------------------------
// E23 — file descriptor 1 closed: the write fails with EBADF.
// ---------------------------------------------------------------------------
#[test]
fn err_e23_stdout_closed() {
    for input in [&b"abcdef\ncd\n"[..], b""] {
        let c = run_exe_stdout(c_exe(), "e23_c", StdinKind::File(input), StdoutKind::Closed);
        let r = run_exe_stdout(rust_exe(), "e23_r", StdinKind::File(input), StdoutKind::Closed);
        assert_eq!(
            c.observable(),
            r.observable(),
            "closed-stdout divergence for {:?}\n  C   : {c:?}\n  Rust: {r:?}",
            hex(input)
        );
        assert_eq!(c.code, Some(0), "C exit code with fd 1 closed: {c:?}");
    }
    let c = run_main_stdout(c_so(), "e23b_c", StdinKind::File(b"abc\n"), StdoutKind::Closed, false);
    let r = run_main_stdout(rust_so(), "e23b_r", StdinKind::File(b"abc\n"), StdoutKind::Closed, false);
    assert_eq!(c.observable(), r.observable(), "C: {c:?} Rust: {r:?}");
}
