//! Executable level differential tests of the *real* two binaries (no ptrace
//! tricks): the C program built from `c_src/` versus the Rust binary, compared on
//! stdout, stderr and exit status.
//!
//! This covers `c_src/src/main.c` - rows 17-24 of `ERRORS.md` (the driver's own
//! rejections) and rows 40-41 of `CONFIGS.md` (the stdin pipeline and the
//! `scanf` conversions).
//!
//! The inputs here are chosen so that the result does not depend on the
//! uninitialised part of the C stack frame (every buffer is NUL terminated
//! inside the bytes read from stdin, and `operation 4` with the case sensitive
//! flag never gets a pattern longer than its text). The overread cases are
//! covered by `tests/exe_frame.rs`, which controls those bytes explicitly.

mod common;

use common::*;

/// NUL terminated payload of `len` bytes (so no read runs past it).
fn terminated(len: usize, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; len];
    if let Some(last) = v.last_mut() {
        *last = 0;
    }
    v
}

// ---------------------------------------------------------------------------
// CONFIGS row 40: the whole pipeline, all operations and flag combinations
// ---------------------------------------------------------------------------

#[test]
fn exe_random_all_ops() {
    let rng = Rng::new(4040);
    let words: [&[u8]; 12] = [
        b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK", b"NONE",
        b"EMPTY", b"abc", b"",
    ];
    let mut done = 0usize;
    while done < 300 {
        let op = rng.pick(&[0i64, 1, 2, 3, 4, 0, 1, 2, 3, 4, 5, -1]);
        let flags = rng.pick(&[0u32, 1, 2, 3, 4, 0xFFFF_FFFF]);
        let make = || -> Vec<u8> {
            let mut v = match rng.below(4) {
                0 => rng.pick(&words).to_vec(),
                1 => (0..rng.below(20)).map(|_| rng.byte() | 1).collect(),
                2 => {
                    let n = rng.pick(&[0usize, 1, 2, 3, 7, 8, 15, 16, 33, 64]);
                    vec![rng.pick(&[b'A', b'a', b'z', 1u8, 255, b' ', b':', b'|']); n]
                }
                _ => {
                    let mut v = rng.pick(&words).to_vec();
                    v.extend((0..rng.below(4)).map(|_| rng.byte() | 1));
                    v
                }
            };
            v.push(0); // always NUL terminated: no overread
            v
        };
        let a = make();
        let b = make();
        // `match_pattern` with case sensitivity underflows its loop bound when
        // the pattern is longer than the text; that walks up the stack and is
        // covered by tests/exe_frame.rs instead.
        if op == 4 && flags & 0x02 != 0 {
            let tlen = a.iter().position(|&c| c == 0).unwrap_or(a.len());
            let plen = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            if tlen < plen {
                continue;
            }
        }
        diff_exe(&exe_case(op, flags, &a, &b), "row40 random pipeline");
        done += 1;
    }
}

#[test]
fn exe_boundary_lengths() {
    for len in [1usize, 2, 1023, 1024] {
        for op in [0i64, 1, 2, 3, 4] {
            for flags in [0u32, 1, 2, 3] {
                let a = terminated(len, b'A');
                let b = terminated(len, b'a');
                diff_exe(&exe_case(op, flags, &a, &b), "row40 boundary lengths");
                let b2 = terminated(len, b'A');
                diff_exe(&exe_case(op, flags, &a, &b2), "row40 boundary, identical");
            }
        }
    }
}

#[test]
fn exe_literal_results() {
    // every documented return value of the library, driven through the binary
    for (input, op, flags, reference, expect) in [
        (b"START\0".as_slice(), 1i64, 0u32, b"\0".as_slice(), "0\n"),
        (b"STOP\0", 1, 0, b"\0", "1\n"),
        (b"PAUSE\0", 1, 0, b"\0", "2\n"),
        (b"RESUME\0", 1, 0, b"\0", "3\n"),
        (b"RESET\0", 1, 0, b"\0", "4\n"),
        (b"ADMIN\0", 1, 0, b"\0", "99\n"),
        (b"nope\0", 1, 0, b"\0", "-1\n"),
        (b"VALID\0", 0, 0, b"zz\0", "1\n"),
        (b"OK\0", 0, 0, b"zz\0", "1\n"),
        (b"same\0", 0, 0, b"same\0", "1\n"),
        (b"diff\0", 0, 0, b"other\0", "0\n"),
        (b"prefix_v1\0", 2, 1, b"prefix\0", "2\n"),
        (b"prefix_v2\0", 2, 1, b"prefix\0", "3\n"),
        (b"prefix_old\0", 2, 1, b"prefix\0", "4\n"),
        (b"prefix_new\0", 2, 1, b"prefix\0", "5\n"),
        (b"prefix_tmp\0", 2, 1, b"prefix\0", "6\n"),
        (b"prefixed\0", 2, 0, b"prefix\0", "1\n"),
        (b"prefixed\0", 2, 1, b"prefix\0", "0\n"),
        // ref_len == 0 -> the delimiter defaults to ':'
        (b"a:b\0", 3, 0, b"", "1\n"),
        // ref_len == 1 with reference[0] == '\0' -> the delimiter *is* NUL
        (b"a:b\0", 3, 0, b"\0", "3\n"),
        (b"ab|c\0", 3, 0, b"|\0", "2\n"),
        (b"NONE\0", 3, 0, b"|\0", "-2\n"),
        (b"EMPTY\0", 3, 0, b":\0", "-3\n"),
        (b"xyz\0", 3, 0, b":\0", "-1\n"),
        (b"*pat*\0", 4, 2, b"pat\0", "2\n"),
        (b"pat*\0", 4, 2, b"pat\0", "3\n"),
        (b"*pat\0", 4, 2, b"pat\0", "4\n"),
        (b"xxpat\0", 4, 2, b"pat\0", "12\n"),
        (b"PAT\0", 4, 0, b"pat\0", "6\n"),
        (b"patxx\0", 4, 0, b"pat\0", "5\n"),
        (b"same\0", 4, 0, b"same\0", "1\n"),
        (b"anything\0", 9, 0, b"x\0", "-3\n"),
    ] {
        let r = diff_exe(
            &exe_case(op, flags, input, reference),
            "row40 documented results",
        );
        assert_eq!(
            String::from_utf8_lossy(&r.stdout),
            expect,
            "input {input:?} op {op} flags {flags} reference {reference:?}"
        );
        assert_eq!(r.code, 0);
    }
}

#[test]
fn exe_segfault_parity() {
    // strlen(text) < strlen(pattern) with case sensitive matching: the loop
    // bound underflows and both programs die walking up their stack. The pattern
    // is high entropy so that it cannot be found anywhere above the frame.
    let input = exe_case(4, 2, b"AB\0", b"\xf7\x3b\xa9\xd1\x5c\xe2\x84\x91\0");
    let r = diff_exe(&input, "row40 unbounded loop");
    assert_eq!(r.code, 128 + 11, "expected SIGSEGV: {r:?}");
    assert!(r.stdout.is_empty());
}

// ---------------------------------------------------------------------------
// CONFIGS row 41: scanf tokenisation shapes
// ---------------------------------------------------------------------------

#[test]
fn exe_scanf_shapes() {
    for input in [
        "0 0 3 79 75 0 3 79 75 0\n",
        "0\n0\n3\n79\n75\n0\n3\n79\n75\n0\n",
        "  0   0\t3\r\n79 75 0\n\n3 79 75 0\n",
        "+0 +0 +3 79 75 0 +3 79 75 0\n",
        "-1 0 1 0 1 0\n",
        "0 -1 2 65 0 2 65 0\n",
        "1 0 6 83 84 79 80 0 0 0\n",
        "1 0 6 83 84 79 80 0 0 0 extra tokens are ignored\n",
        "0 0 2 4294967295 4294967296 2 255 0\n",
        "0 0 2 -1 -2 2 255 254\n",
        "3 0 4 97 58 98 0 1 58\n",
        "0 0 2 65 0 2 65 0 ",
        "0 0 2 65 0 2 65 0",
        "4 2 3 65 66 0 3 65 66 0\n",
        "2 1 4 97 98 99 0 4 97 98 99 0\n",
        "0x10 0 0 0\n",
        "0 0x1 0 0\n",
        "99999999999999999999 0 1 0 1 0\n",
        "0 99999999999999999999 1 0 1 0\n",
        "2147483647 0 1 0 1 0\n",
        "-2147483648 0 1 0 1 0\n",
        "2147483648 0 1 0 1 0\n",
        "0 4294967295 1 0 1 0\n",
        "0 4294967296 1 0 1 0\n",
        "1 0 1024 1 0\n",
        "0 0 0 1 0\n",
        "\t\n 2 \n 1 \n 4 \n 97 98 99 0 \n 4 \n 97 98 99 0 \n",
        // strtol/strtoul saturation followed by truncation to the target type
        "-99999999999999999999 0 1 0 1 0\n",
        "0 -18446744073709551615 1 0 1 0\n",
        "0 -18446744073709551616 1 0 1 0\n",
        "0 0 0000000000000000000000002 65 0 1 0\n",
        "0 0 +2 65 0 +1 0\n",
        "00000 0 2 65 0 2 65 0\n",
        "0 0 2 000000000000065 0 2 65 0\n",
        "-0 -0 1 0 1 0\n",
        "0 0 1 -0 1 -0\n",
        "0 0 1 +0 1 +0\n",
        "9999999999 0 1 0 1 0\n",
        "0 4294967297 1 0 1 0\n",
        "0 0 2 4294967297 0 2 1 0\n",
        "0 0 1 18446744073709551616 1 0\n",
        "2 4294967295 4 97 98 99 0 4 97 98 99 0\n",
        "0 0 1 0\n",
        "0 0 1 0 1\n",
        "0 0 1 0 1 0 1 0\n",
        "0\t0\t1\t0\t1\t0\n",
    ] {
        diff_exe(input, "row41 scanf shapes");
    }
}

/// The same tokenisation shapes whose *result* depends on the uninitialised
/// frame (buffers that are not NUL terminated), compared with a controlled frame.
#[test]
fn exe_scanf_shapes_with_overread() {
    for input in [
        "\n\n\n0 0 0 0\n",
        "0 -1 1 65 1 65\n",
        "1 0 5 83 84 65 82 84 0\n",
        "1 0 5 83 84 65 82 84 0 extra\n",
        "0 0 2 4294967295 4294967296 0\n",
        "0 0 2 -1 -2 0\n",
        "0 0 1 65 1 65 ",
        "2 0 0 0\n",
        "2 1 0 0\n",
        "3 0 1 65 0\n",
        "4 0 0 0\n",
        "0 0 1 0 0\n",
        "0 0 0 1 65\n",
        "1 0 4 83 84 79 80 0\n",
    ] {
        diff_exe_injected(input, "row41 scanf shapes with overread");
    }
}

// ---------------------------------------------------------------------------
// ERRORS rows 17-24: the driver's own rejections
// ---------------------------------------------------------------------------

#[test]
fn exe_err_reading_operation() {
    for input in [
        "",
        "\n",
        "   ",
        "abc\n",
        "x 0 0 0\n",
        "-\n",
        "+\n",
        ".5 0 0 0\n",
        "\0",
    ] {
        let r = diff_exe(input, "row17 error reading operation");
        assert_eq!(r.code, 1);
        assert_eq!(r.stderr, b"Error reading operation\n");
        assert!(r.stdout.is_empty());
    }
}

#[test]
fn exe_err_reading_flags() {
    for input in ["0\n", "0", "0 \n", "0 abc\n", "0 -\n", "0 +x\n"] {
        let r = diff_exe(input, "row18 error reading flags");
        assert_eq!(r.code, 1);
        assert_eq!(r.stderr, b"Error reading flags\n");
    }
}

#[test]
fn exe_err_reading_input_length() {
    for input in ["0 0\n", "0 0 ", "0 0 abc\n", "0 0 -\n", "0 0 z\n"] {
        let r = diff_exe(input, "row19 error reading input length");
        assert_eq!(r.code, 1);
        assert_eq!(r.stderr, b"Error reading input length\n");
    }
}

#[test]
fn exe_err_input_length_too_big() {
    for (input, shown) in [
        ("0 0 1025\n", "1025"),
        ("0 0 2000 1 2 3\n", "2000"),
        ("0 0 -1\n", "18446744073709551615"),
        ("0 0 18446744073709551615\n", "18446744073709551615"),
        ("0 0 99999999999999999999999\n", "18446744073709551615"),
        ("0 0 1099511627776\n", "1099511627776"),
        ("0 0 -1024\n", "18446744073709550592"),
    ] {
        let r = diff_exe(input, "row20 input length exceeds maximum");
        assert_eq!(r.code, 1);
        assert_eq!(
            r.stderr,
            format!("Error: input length {shown} exceeds maximum 1024\n").into_bytes()
        );
    }
}

#[test]
fn exe_err_reading_input_byte() {
    for (input, idx) in [
        ("0 0 1\n", 0usize),
        ("0 0 3 65 66\n", 2),
        ("0 0 5 65 66 67 68 abc\n", 4),
        ("0 0 2 65 -\n", 1),
        ("0 0 1024 1 2 3\n", 3),
    ] {
        let r = diff_exe(input, "row21 error reading input byte");
        assert_eq!(r.code, 1);
        assert_eq!(
            r.stderr,
            format!("Error reading input byte {idx}\n").into_bytes()
        );
    }
}

#[test]
fn exe_err_reading_ref_length() {
    for input in ["0 0 0\n", "0 0 2 65 66\n", "0 0 1 65 abc\n", "0 0 0 -\n"] {
        let r = diff_exe(input, "row22 error reading reference length");
        assert_eq!(r.code, 1);
        assert_eq!(r.stderr, b"Error reading reference length\n");
    }
}

#[test]
fn exe_err_ref_length_too_big() {
    for (input, shown) in [
        ("0 0 0 1025\n", "1025"),
        ("0 0 1 65 4096 1 2\n", "4096"),
        ("0 0 0 -1\n", "18446744073709551615"),
        ("0 0 0 18446744073709551616\n", "18446744073709551615"),
    ] {
        let r = diff_exe(input, "row23 reference length exceeds maximum");
        assert_eq!(r.code, 1);
        assert_eq!(
            r.stderr,
            format!("Error: reference length {shown} exceeds maximum 1024\n").into_bytes()
        );
    }
}

#[test]
fn exe_err_reading_ref_byte() {
    for (input, idx) in [
        ("0 0 0 1\n", 0usize),
        ("0 0 1 65 3 66 67\n", 2),
        ("0 0 0 4 1 2 3 zzz\n", 3),
        ("0 0 0 1024 1\n", 1),
    ] {
        let r = diff_exe(input, "row24 error reading reference byte");
        assert_eq!(r.code, 1);
        assert_eq!(
            r.stderr,
            format!("Error reading reference byte {idx}\n").into_bytes()
        );
    }
}

#[test]
fn exe_accepts_maximum_lengths() {
    // exactly MAX_BUFFER_SIZE is accepted for both buffers
    let a = terminated(1024, b'A');
    let b = terminated(1024, b'B');
    let r = diff_exe(&exe_case(2, 0, &a, &b), "boundary: 1024/1024 accepted");
    assert_eq!(r.code, 0);
    let r = diff_exe(
        &exe_case(2, 0, &a[..1023], &b[..1023]),
        "boundary: 1023/1023 accepted",
    );
    assert_eq!(r.code, 0);
}
