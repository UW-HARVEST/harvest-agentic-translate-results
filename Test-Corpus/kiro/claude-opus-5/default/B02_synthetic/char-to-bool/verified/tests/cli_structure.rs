//! Input handling in `main`: the three `fgets` reads, the read-failure exit
//! paths, newline stripping, and the 1024-byte buffer boundary.

mod harness;
use harness::{c_binary, case, compare_all, rust_binary};

/// Phase A sanity check: both executables exist and run.
#[test]
fn both_programs_are_runnable() {
    let c = c_binary();
    let rust = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(rust.is_file(), "Rust binary missing at {}", rust.display());

    let out = harness::run(c, b"0\n0\nyyy\n");
    assert_eq!(out.stdout, b"107\n", "C program produced {out:?}");
    let out = harness::run(rust, b"0\n0\nyyy\n");
    assert_eq!(out.stdout, b"107\n", "Rust program produced {out:?}");
}

/// Each of the three `fgets` calls can fail, and each has its own message
/// on stderr and an exit status of 1.
#[test]
fn read_failure_paths() {
    let inputs = vec![
        // First fgets returns NULL: nothing at all on stdin.
        b"".to_vec(),
        // First fgets succeeds on a bare newline, second returns NULL.
        b"\n".to_vec(),
        b"0".to_vec(),
        b"0\n".to_vec(),
        b"abc".to_vec(),
        b"abc\n".to_vec(),
        // Third fgets returns NULL.
        b"0\n0".to_vec(),
        b"0\n0\n".to_vec(),
        b"\n\n".to_vec(),
        b"2\n3".to_vec(),
        b"2\n3\n".to_vec(),
        // All three succeed.
        b"\n\n\n".to_vec(),
        b"\n\n\n\n".to_vec(),
        b"0\n0\nyyy".to_vec(),
        b"0\n0\nyyy\n".to_vec(),
        // Extra input after the third line is never read.
        b"0\n0\nyyy\nyyy\nyyy\n".to_vec(),
        b"0\n0\nyyy\nzzz".to_vec(),
    ];
    compare_all("read_failure_paths", inputs);
}

/// The decision line with and without a trailing newline, and the case
/// where stripping the newline leaves a zero-length string (which makes
/// `process_decisions` take its `length == 0` path and return -1).
#[test]
fn newline_stripping_and_empty_decision_string() {
    let mut inputs = Vec::new();
    for op in ["0", "1", "2", "3", "4", "-1"] {
        for param in ["0", "1", "2", "3"] {
            // Empty decision line: len becomes 0 after the newline is removed.
            inputs.push(format!("{op}\n{param}\n\n").into_bytes());
            // Decision line at EOF with no newline to strip.
            inputs.push(format!("{op}\n{param}\ny").into_bytes());
            inputs.push(format!("{op}\n{param}\nynn").into_bytes());
            inputs.push(format!("{op}\n{param}\nynnyn").into_bytes());
            // With a newline to strip.
            inputs.push(case(op, param, "y"));
            inputs.push(case(op, param, "ynn"));
            inputs.push(case(op, param, "ynnyn"));
        }
    }
    compare_all("newline_stripping_and_empty_decision_string", inputs);
}

/// `fgets` keeps `\r`, so a CRLF file leaves a carriage return as the last
/// character of the decision string. It is not a `y`, so `parse_bool` maps
/// it to false, and it still counts towards the length.
#[test]
fn carriage_returns_are_part_of_the_data() {
    let mut inputs = Vec::new();
    for op in ["0", "1", "2", "3"] {
        for param in ["0", "1", "2", "3"] {
            for s in ["y", "yn", "ynn", "yyn", "yynn", "ynynyn", "yyy", "nnn"] {
                inputs.push(format!("{op}\r\n{param}\r\n{s}\r\n").into_bytes());
                // Bare carriage returns: no newline at all, so the whole
                // stream is one `fgets` line.
                inputs.push(format!("{op}\r{param}\r{s}\r").into_bytes());
                // A stray CR in the middle of the decision string.
                inputs.push(format!("{op}\n{param}\ny\rn\n").into_bytes());
            }
        }
    }
    compare_all("carriage_returns_are_part_of_the_data", inputs);
}

/// `MAX_INPUT_SIZE` is 1024, so `fgets` stores at most 1023 bytes and the
/// rest of an over-long line is left on stdin to be picked up by the next
/// read. Exercise lines just below, at, and above that boundary in each of
/// the three positions.
#[test]
fn buffer_boundary() {
    let mut inputs = Vec::new();
    for n in [1, 2, 1021, 1022, 1023, 1024, 1025, 1026, 2047, 2048, 2049, 3000] {
        for op in ["0", "1", "2", "3", "7"] {
            // Over-long decision string: truncated at 1023 bytes.
            inputs.push(format!("{op}\n0\n{}\n", "y".repeat(n)).into_bytes());
            inputs.push(format!("{op}\n2\n{}\n", "yn".repeat(n / 2 + 1)).into_bytes());
            inputs.push(format!("{op}\n0\n{}\n", "n".repeat(n)).into_bytes());
            // Over-long operation line: the tail becomes the parameter line.
            inputs.push(format!("{}\n{op}\nyny\n", "y".repeat(n)).into_bytes());
            inputs.push(format!("{}{op}\n0\nyny\n", "0".repeat(n)).into_bytes());
            // Over-long parameter line: the tail becomes the decision line.
            inputs.push(format!("3\n{}\nyny\n", "3".repeat(n)).into_bytes());
        }
        // No trailing newline on a maximal line.
        inputs.push(format!("2\n0\n{}", "y".repeat(n)).into_bytes());
    }
    compare_all("buffer_boundary", inputs);
}

/// Bytes that are neither `y` nor `n`, including embedded NULs (which
/// shorten the string as far as `strlen` is concerned) and bytes above
/// 127 (where C's `char` is signed but the comparisons still fail).
#[test]
fn non_yn_bytes() {
    let mut inputs = Vec::new();
    for op in ["0", "1", "2", "3"] {
        for param in ["0", "1", "2", "3"] {
            for s in [
                "x", "xyz", "q", "Y", "N", "YN", "YyNn", "y n", "y\tn", "0", "1", "  ", "..",
                "YYY", "NNN", "yNy", "Ynn", "yy?", "?yy",
            ] {
                inputs.push(case(op, param, s));
            }
            // Embedded NUL at each position of an 8 character string.
            for i in 0..8 {
                let mut bytes = b"ynynynyn".to_vec();
                bytes[i] = 0;
                let mut input = format!("{op}\n{param}\n").into_bytes();
                input.extend_from_slice(&bytes);
                input.push(b'\n');
                inputs.push(input);
            }
            // High bytes: signed `char` in C, `u8` in Rust.
            for b in [0x80u8, 0xf9, 0xff, 0xd9] {
                let mut input = format!("{op}\n{param}\n").into_bytes();
                input.extend_from_slice(&[b'y', b, b'n', b, b'n']);
                input.push(b'\n');
                inputs.push(input);
            }
            // A NUL as the very first byte of each line.
            inputs.push(format!("\0{op}\n{param}\nyny\n").into_bytes());
            inputs.push(format!("{op}\n\0{param}\nyny\n").into_bytes());
            inputs.push(format!("{op}\n{param}\n\0yny\n").into_bytes());
        }
    }
    compare_all("non_yn_bytes", inputs);
}
