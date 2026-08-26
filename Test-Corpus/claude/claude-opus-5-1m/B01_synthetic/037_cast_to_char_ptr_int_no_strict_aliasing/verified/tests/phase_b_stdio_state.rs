//! Phase B — differential tests for the *stdio state* the implementations
//! leave behind, which is part of the observable behaviour of a program whose
//! whole job is reading stdin.  Rows 31-35 of CONFIGS.md.
//!
//! Three distinct axes are checked here:
//!   * how many bytes of stdin the implementation consumes from the descriptor
//!     (glibc reads a whole `st_blksize` block at a time),
//!   * the exit-time `_IO_cleanup` seek-back on a seekable descriptor,
//!   * the process-wide stdin buffer (repeated `main()` calls continue where the
//!     previous conversion stopped, and EOF is sticky).

mod common;

use common::*;

const CASES: &[&[u8]] = &[
    b"",
    b" ",
    b"\n",
    b"42",
    b"42 ",
    b"42\n",
    b"42 rest",
    b"42\n7\n9\n",
    b"  \t 42XY",
    b"abc",
    b"-",
    b"+",
    b"-x5",
    b"--5",
    b"- 5",
    b"0x10",
    b"\0 5",
    b"99999999999999999999999zz",
    b"  -2147483648  trailing text",
    b"+0000000000000000000000000000005 tail",
];

/// CONFIGS row 31 — bytes consumed from fd 0 by the exported `main`, seekable
/// stdin (no exit-time cleanup: the `.so`'s `main` just returns).
#[test]
fn cfg_31_so_main_stdin_consumption_file() {
    for input in CASES {
        assert_main_drain_eq(input, Stdin::File);
    }
    // inputs longer than one stdio block (4096 on this fs) and around it
    for len in [4095usize, 4096, 4097, 8192, 20002] {
        let mut input = b"42 ".to_vec();
        input.resize(len, b'A');
        assert_main_drain_eq(&input, Stdin::File);
    }
    let mut rng = Rng::new();
    for _ in 0..40 {
        let v = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
        let tail_len = rng.below(200) as usize;
        let mut input = format!("{v}").into_bytes();
        input.extend((0..tail_len).map(|_| *rng.pick(b"ABC \n0123-+")));
        assert_main_drain_eq(&input, Stdin::File);
    }
}

/// CONFIGS row 32 — same, but for a non-seekable descriptor (pipe): the whole
/// buffered block stays consumed.
#[test]
fn cfg_32_so_main_stdin_consumption_pipe() {
    for input in CASES {
        assert_main_drain_eq(input, Stdin::Pipe);
    }
    assert_main_drain_eq(b"", Stdin::DevNull);
    assert_main_drain_eq(b"1", Stdin::DevNull);
}

/// CONFIGS row 33 — repeated `main()` calls in one process share the stdin
/// buffer: the second conversion continues after the first one's pushback.
#[test]
fn cfg_33_so_main_repeated_same_process() {
    for n in [1usize, 2, 3, 5] {
        assert_main_n_eq(b"", Stdin::File, n);
        assert_main_n_eq(b"42", Stdin::File, n);
        assert_main_n_eq(b"1 2 3 4 5 6\n", Stdin::File, n);
        assert_main_n_eq(b"  7\t-8\n+9  10  ", Stdin::File, n);
        assert_main_n_eq(b"abc def", Stdin::File, n);
        assert_main_n_eq(b"1 abc 2", Stdin::File, n);
        assert_main_n_eq(b"2147483648 -2147483649 0", Stdin::File, n);
        assert_main_n_eq(
            b"99999999999999999999 -99999999999999999999",
            Stdin::File,
            n,
        );
        assert_main_n_eq(b"1 2 3", Stdin::Pipe, n);
        assert_main_n_eq(b"", Stdin::Pipe, n);
        assert_main_n_eq(b"", Stdin::DevNull, n);
        assert_main_n_eq(b"5", Stdin::Closed, n);
    }
    let mut rng = Rng::new();
    for _ in 0..40 {
        let count = 1 + rng.below(6) as usize;
        let mut input = String::new();
        for _ in 0..count {
            let v = rng.range_i64(i64::MIN, i64::MAX);
            input.push_str(&format!("{v}"));
            input.push(char::from(*rng.pick(b" \t\n")));
        }
        assert_main_n_eq(input.as_bytes(), Stdin::File, 1 + rng.below(5) as usize);
    }
}

/// CONFIGS row 34 — the whole program with a seekable stdin: stdout **and** the
/// file offset it leaves behind (libc's exit-time seek-back).
#[test]
fn cfg_34_exe_file_stdin_leftover() {
    for input in CASES {
        assert_exe_file_stdin_eq(input);
    }
    for len in [4095usize, 4096, 4097, 20002] {
        let mut input = b"42 ".to_vec();
        input.resize(len, b'A');
        assert_exe_file_stdin_eq(&input);
    }
    let mut rng = Rng::new();
    for _ in 0..60 {
        let n = rng.below(40) as usize;
        let input: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"0123456789+- \t\n\r\x0b\x0cabz.\0"))
            .collect();
        assert_exe_file_stdin_eq(&input);
    }
}

/// CONFIGS row 35 — the whole program with a pre-filled pipe as stdin: stdout
/// plus the bytes left unread in the pipe (no seek-back possible).
#[test]
fn cfg_35_exe_pipe_stdin_leftover() {
    for input in CASES {
        assert_exe_pipe_stdin_eq(input);
    }
    for len in [4095usize, 4096, 4097, 20002, 50000] {
        let mut input = b"42 ".to_vec();
        input.resize(len, b'A');
        assert_exe_pipe_stdin_eq(&input);
    }
    let mut rng = Rng::new();
    for _ in 0..60 {
        let n = rng.below(40) as usize;
        let input: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"0123456789+- \t\n\r\x0b\x0cabz.\0"))
            .collect();
        assert_exe_pipe_stdin_eq(&input);
    }
}
