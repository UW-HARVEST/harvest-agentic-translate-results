//! Differential tests: run the C binary and the Rust binary as subprocesses on
//! identical stdin and require byte-identical stdout, byte-identical stderr and
//! an identical exit status.
//!
//! The Rust code is never linked as a library here — both programs are driven
//! exactly the way a shell would drive them.
//!
//! # Input domain
//!
//! One family of inputs makes the C program invoke undefined behaviour: `main`
//! declares `uint8_t buffer[256]`, but `compact_runs` can *grow* the logical
//! length (a threshold of 1 rewrites every singleton run as a `value, count`
//! pair) and writes the grown data through the same pointer, past the end of the
//! array. Growth happens only when the threshold is 1, i.e. only when
//! `param1 == 1` with the compact flag set, and the grown length is bounded by
//! `2 * length`. See `ERRORS.md` for the full analysis and the measured
//! boundaries. `c_clobbers_live_locals` identifies that family so the generated
//! sweeps stay inside the well-defined domain; the boundary itself is pinned by
//! `compact_growth_overflow_boundary_matches`.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Once;

/// What a run of one program produced.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit code {:?}, signal {:?}\n      stdout: {:?}\n      stderr: {:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
        )
    }
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the compiled C reference, building it first if it is absent.
///
/// This only runs the build commands documented for the C tree; nothing under
/// `c_src/` is modified.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let c_src = repo_root().join("c_src");
    let exe = c_src.join("build").join("driver");

    BUILD.call_once(|| {
        if exe.exists() {
            return;
        }
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .status();
        let compiled = match configure {
            Ok(s) if s.success() => Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build_dir)
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
            _ => false,
        };
        assert!(
            compiled,
            "could not build the C reference. Build it manually with:\n  \
             cd c_src && mkdir -p build && cd build && cmake .. && cmake --build ."
        );
    });

    assert!(
        exe.exists(),
        "missing C reference binary at {}",
        exe.display()
    );
    exe
}

fn run_program(exe: &PathBuf, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(input)
        // A program that exits before draining stdin (every error path does)
        // makes this write fail with EPIPE; that is expected, not a failure.
        .ok();
    drop(child.stdin.take());

    let out = child.wait_with_output().expect("wait failed");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(input: &str) {
    let c = run_program(&c_bin(), input.as_bytes());
    let r = run_program(&rust_bin(), input.as_bytes());

    if c != r {
        panic!(
            "divergence for input {:?}\n  C   -> {:?}\n  Rust-> {:?}",
            input, c, r
        );
    }
}

#[track_caller]
fn assert_same_all(inputs: &[&str]) {
    for i in inputs {
        assert_same(i);
    }
}

/// Same as [`assert_same`] for inputs that are not valid UTF-8.
#[track_caller]
fn assert_same_bytes(input: &[u8]) {
    let c = run_program(&c_bin(), input);
    let r = run_program(&rust_bin(), input);

    if c != r {
        panic!(
            "divergence for byte input {:?}\n  C   -> {:?}\n  Rust-> {:?}",
            String::from_utf8_lossy(input),
            c,
            r
        );
    }
}

/// True when the C program would write past `buffer[280]` — i.e. past the
/// already-consumed locals that follow `buffer` in `main`'s frame — and so
/// produce output governed by undefined behaviour rather than by its own logic.
///
/// `compact_runs` grows the length only when the threshold is 1 (`param1 == 1`),
/// and the grown length never exceeds `2 * length`, so a length of at most 140
/// is always safe.
fn c_clobbers_live_locals(flags: u32, param1: i32, length: usize) -> bool {
    (flags & 0x02) != 0 && param1 == 1 && length > 140
}

fn encode(flags: u32, param1: i32, param2: i32, values: &[u8]) -> String {
    let mut s = format!("{} {} {} {}", flags, param1, param2, values.len());
    for v in values {
        s.push_str(&format!(" {}", v));
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// main.c: reading and validating the four scalars
// ---------------------------------------------------------------------------

/// `scanf("%u", &flags)` returning != 1: EOF and matching failures.
#[test]
fn error_reading_flags() {
    assert_same_all(&[
        "",                 // immediate EOF
        "\n",               // whitespace then EOF
        "   \t\n\r\n  ",    // only whitespace
        "abc",              // matching failure on first char
        "abc 1 2 3",        // matching failure with more input available
        "-",                // sign with no digits
        "+",                //
        "-\n",              //
        "+ 1 2 3",          //
        ".5 1 2 3",         //
        "x1 2 3 4",         //
    ]);
}

/// `scanf("%d", &param1)` returning != 1.
#[test]
fn error_reading_param1() {
    assert_same_all(&[
        "1",             // EOF right after flags
        "1 ",            //
        "1\n",           //
        "1 abc",         // matching failure
        "1 -",           //
        "1 +",           //
        "1 zz 3 4",      //
    ]);
}

/// `scanf("%d", &param2)` returning != 1.
#[test]
fn error_reading_param2() {
    assert_same_all(&["1 2", "1 2 ", "1 2\n", "1 2 abc", "1 2 -", "1 2 ** 4"]);
}

/// `scanf("%zu", &length)` returning != 1.
#[test]
fn error_reading_length() {
    assert_same_all(&["1 2 3", "1 2 3 ", "1 2 3\n", "1 2 3 abc", "1 2 3 -", "1 2 3 x 5"]);
}

/// `length > 256` -> "Error: length %zu exceeds maximum 256".
#[test]
fn error_length_exceeds_maximum() {
    assert_same_all(&[
        "0 0 0 257",
        "0 0 0 258",
        "0 0 0 1000",
        "0 0 0 65535",
        // `%zu` uses strtoul semantics, so a negative literal wraps to a huge
        // size_t and reports that huge value in the message.
        "0 0 0 -1",
        "0 0 0 -257",
        "0 0 0 18446744073709551615",
        // strtoul saturates at ULONG_MAX on overflow.
        "0 0 0 99999999999999999999999999",
        "0 0 0 18446744073709551616",
    ]);
}

/// 256 is accepted; 257 is not.
#[test]
fn length_at_and_above_the_maximum() {
    let values: Vec<u8> = (0..=255u8).collect();
    assert_same(&encode(0, 0, 0, &values));
    assert_same("0 0 0 257");
}

/// `scanf("%u", &byte)` returning != 1 -> "Error reading byte %zu".
#[test]
fn error_reading_byte() {
    assert_same_all(&[
        "0 0 0 1",            // fails at byte 0 (EOF)
        "0 0 0 1 abc",        // fails at byte 0 (matching failure)
        "0 0 0 2 5",          // fails at byte 1
        "0 0 0 5 1 2 3",      // fails at byte 3
        "0 0 0 5 1 2 3 4",    // fails at byte 4
        "0 0 0 3 1 2 -",      // sign with no digits
        "0 0 0 3 1 2 q",      //
    ]);

    // Fails at the very last byte of a maximum-length buffer.
    let mut s = String::from("0 0 0 256");
    for i in 0..255 {
        s.push_str(&format!(" {}", i % 256));
    }
    s.push('\n');
    assert_same(&s);
}

/// `length == 0` short-circuits `process_buffer`, which returns 0.
#[test]
fn zero_length_buffer() {
    // Every flag combination must still print just "0".
    for flags in 0..32u32 {
        assert_same(&encode(flags, 3, 1, &[]));
    }
    assert_same("0 0 0 0");
    assert_same("31 1 1 0");
}

/// A single item, for every flag combination: exercises the `len <= 1` and
/// `new_len < 2` / `new_len < 4` early exits.
#[test]
fn single_item_buffer() {
    for flags in 0..32u32 {
        for param1 in [-3, 0, 1, 2, 3, 4] {
            for param2 in [0, 1] {
                assert_same(&encode(flags, param1, param2, &[42]));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// main.c: scanf's lexical behaviour
// ---------------------------------------------------------------------------

/// `scanf` numeric conversions skip *any* whitespace, newlines included, so the
/// same tokens are accepted in any layout.
#[test]
fn scanf_reads_across_whitespace_and_newlines() {
    assert_same_all(&[
        "3 2 1 4 10 20 30 40\n",
        "3\n2\n1\n4\n10\n20\n30\n40\n",
        "3\t2\t1\t4\t10\t20\t30\t40\n",
        "  3   2   1   4   10   20   30   40  \n",
        "\n\n\n3\n\n2\n\n1\n\n4\n\n10 20\n30 40\n",
        "3 2 1 4 10 20 30 40",          // no trailing newline
        "3 2 1 4 10 20 30 40\r\n",      // CRLF
        "3\r\n2\r\n1\r\n4\r\n10 20 30 40\r\n",
        "\u{b}\u{c}3 2 1 4 10 20 30 40\n", // vertical tab / form feed
    ]);
}

/// Input past the last needed byte is simply never read.
#[test]
fn trailing_input_is_ignored() {
    assert_same_all(&[
        "0 0 0 1 42 99 88\n",
        "0 0 0 1 42 garbage here\n",
        "0 0 0 0 1 2 3 4 5\n",
    ]);
}

/// Signs and width truncation, as `strtoul`/`strtol` plus assignment perform them.
#[test]
fn numeric_conversion_edges() {
    assert_same_all(&[
        "+3 +2 +1 4 10 20 30 40\n",
        "3 -2 -1 4 10 20 30 40\n",
        // %u truncates to unsigned int
        "4294967296 0 0 2 1 2\n",
        "4294967295 0 0 2 1 2\n",
        "4294967297 0 0 2 1 2\n",
        // strtoul saturation then truncation
        "99999999999999999999999 0 0 2 1 2\n",
        // %d truncates to int
        "0 2147483647 0 2 1 2\n",
        "0 -2147483648 0 2 1 2\n",
        "0 2147483648 0 2 1 2\n",
        "0 4294967296 0 2 1 2\n",
        "0 99999999999999999999 0 2 1 2\n",
        "0 -99999999999999999999 0 2 1 2\n",
        // byte values above 255 truncate to uint8_t
        "0 0 0 4 256 257 511 65535\n",
        "0 0 0 4 -1 -2 -256 -257\n",
        // leading zeros are decimal, and "0x10" parses as 0 then fails on 'x'
        "007 003 001 4 010 020 030 040\n",
        "0x10 0 0 2 1 2\n",
        "0 0 0 2 0x10 2\n",
    ]);
}

// ---------------------------------------------------------------------------
// lib.c: one operation at a time
// ---------------------------------------------------------------------------

/// flags bit 0: rotate. Covers `offset == 0` (both the caller's check and
/// `rotate_buffer`'s), the small-offset branch, the large-offset branch,
/// negative offsets and `len <= 1`.
#[test]
fn rotate_only() {
    let ten: Vec<u8> = (1..=10u8).collect();
    let nine: Vec<u8> = (1..=9u8).collect();

    for param1 in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 20, -1, -2, -3, -4, -5, -9, -10, -11, -20, 100,
        -100, 255, 256, i32::MIN, i32::MAX,
    ] {
        assert_same(&encode(1, param1, 0, &ten));
        assert_same(&encode(1, param1, 0, &nine));
        assert_same(&encode(1, param1, 0, &[7]));
        assert_same(&encode(1, param1, 0, &[7, 8]));
        assert_same(&encode(1, param1, 0, &[7, 8, 9]));
    }

    // Maximum length, both rotation branches.
    let big: Vec<u8> = (0..=255u8).collect();
    for param1 in [1, 2, 100, 127, 128, 129, 200, 255, -1, -128, -255] {
        assert_same(&encode(1, param1, 0, &big));
    }
}

/// flags bit 1: compact runs. Covers the threshold default (`param1` outside
/// 1..=255), thresholds larger than every run, the growth case (threshold 1)
/// and the 255 run-length cap.
#[test]
fn compact_runs_only() {
    let patterns: Vec<Vec<u8>> = vec![
        vec![],
        vec![5],
        vec![5, 5],
        vec![5, 5, 5],
        vec![5, 5, 5, 5],
        vec![1, 2, 3, 4, 5],
        vec![1, 1, 2, 2, 3, 3],
        vec![1, 1, 1, 2, 2, 2, 3, 3, 3],
        vec![9, 9, 9, 9, 9, 1, 2, 3],
        vec![1, 2, 3, 9, 9, 9, 9, 9],
        vec![0, 0, 255, 255, 255, 0, 0, 0, 7],
        vec![4; 20],
        vec![1, 1, 2, 3, 3, 3, 4, 5, 5, 5, 5, 6],
    ];

    for p in &patterns {
        for param1 in [-5, -1, 0, 1, 2, 3, 4, 5, 6, 20, 255, 256, 257, 1000] {
            if c_clobbers_live_locals(0x02, param1, p.len()) {
                continue;
            }
            assert_same(&encode(2, param1, 0, p));
        }
    }

    // A run of 256 identical bytes hits `if (run_len > 255) run_len = 255`.
    let same = vec![7u8; 256];
    for param1 in [1, 2, 3, 200, 255, 256] {
        if c_clobbers_live_locals(0x02, param1, same.len()) {
            continue;
        }
        assert_same(&encode(2, param1, 0, &same));
    }

    // threshold 255 with a run of exactly 255, and one of 254.
    let mut r255 = vec![3u8; 255];
    r255.push(9);
    assert_same(&encode(2, 255, 0, &r255));
    let mut r254 = vec![3u8; 254];
    r254.extend_from_slice(&[9, 9]);
    assert_same(&encode(2, 255, 0, &r254));
}

/// flags bit 2: remove duplicates. `param2 == 0` takes the swap-to-front branch,
/// `param2 != 0` the order-preserving branch; `len <= 1` returns early.
#[test]
fn remove_duplicates_only() {
    let patterns: Vec<Vec<u8>> = vec![
        vec![],
        vec![5],
        vec![5, 5],
        vec![5, 6],
        vec![1, 2, 3, 4],
        vec![1, 1, 1, 1],
        vec![1, 2, 1, 2, 1, 2],
        vec![3, 1, 2, 3, 1, 2, 9],
        vec![0, 255, 0, 255, 128],
        vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
        (0..=255u8).collect(),
        vec![7u8; 64],
    ];
    for p in &patterns {
        for param2 in [0, 1, -1, 2, 12345] {
            assert_same(&encode(4, 0, param2, p));
        }
    }
}

/// flags bit 3: interleave halves. Requires `new_len >= 2`; odd lengths take the
/// `buf[len - 1] = buf[half]` tail, which reads an already-overwritten byte.
#[test]
fn interleave_only() {
    for len in 0..=40usize {
        let vals: Vec<u8> = (0..len).map(|i| (i + 1) as u8).collect();
        assert_same(&encode(8, 0, 0, &vals));
    }
    let big: Vec<u8> = (0..=255u8).collect();
    assert_same(&encode(8, 0, 0, &big));
    let odd: Vec<u8> = (0..255u8).collect();
    assert_same(&encode(8, 0, 0, &odd));
}

/// flags bit 4: reverse segments. Requires `new_len >= 4`; `seg_size` is
/// `param1` when positive and 4 otherwise; `seg_size <= new_len` gates the call
/// and `seg_size <= 1` returns immediately. Also covers remainder 0, 1 and >1.
#[test]
fn reverse_segments_only() {
    for len in 0..=40usize {
        let vals: Vec<u8> = (0..len).map(|i| (i + 1) as u8).collect();
        for param1 in [-3, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 16, 39, 40, 41, 100] {
            assert_same(&encode(16, param1, 0, &vals));
        }
    }
    let big: Vec<u8> = (0..=255u8).collect();
    for param1 in [2, 3, 4, 5, 7, 16, 100, 128, 255, 256, 257] {
        assert_same(&encode(16, param1, 0, &big));
    }
}

// ---------------------------------------------------------------------------
// lib.c: operations in combination
// ---------------------------------------------------------------------------

/// Every one of the 32 meaningful flag combinations, over several data shapes.
#[test]
fn all_flag_combinations() {
    let patterns: Vec<Vec<u8>> = vec![
        vec![],
        vec![42],
        vec![1, 2],
        vec![1, 2, 3],
        vec![1, 2, 3, 4],
        vec![1, 2, 3, 4, 5],
        vec![5, 5, 5, 5, 5, 5],
        vec![1, 1, 2, 2, 3, 3, 4, 4],
        vec![7, 7, 7, 1, 2, 2, 9, 9, 9, 9],
        vec![0, 255, 0, 255, 0, 255, 0],
        (0..17u8).collect(),
        vec![3u8; 33],
    ];

    for flags in 0..32u32 {
        for p in &patterns {
            for param1 in [-4, -1, 0, 1, 2, 3, 4, 5, 8, 255, 256] {
                if c_clobbers_live_locals(flags, param1, p.len()) {
                    continue;
                }
                for param2 in [0, 1] {
                    assert_same(&encode(flags, param1, param2, p));
                }
            }
        }
    }
}

/// Only the low five bits of `flags` are tested by the C code; the rest are
/// ignored, including the sign bit.
#[test]
fn high_flag_bits_are_ignored() {
    let vals: Vec<u8> = vec![4, 4, 4, 1, 2, 3, 3, 9];
    for flags in [
        0xFFFF_FFE0u32,
        0xFFFF_FFFF,
        0x8000_0000,
        0xDEAD_BEEF,
        0x0000_0020,
        0x0000_003F,
        1 << 31,
    ] {
        for param1 in [0, 2, 3, 4] {
            assert_same(&encode(flags, param1, 1, &vals));
        }
    }
}

/// Maximum-length buffers through every flag combination. `param1` avoids 1 so
/// `compact_runs` cannot grow past the C array (see the module docs).
#[test]
fn maximum_length_through_all_flags() {
    let ramp: Vec<u8> = (0..=255u8).collect();
    let same: Vec<u8> = vec![11u8; 256];
    let alt: Vec<u8> = (0..256).map(|i| if i % 2 == 0 { 0 } else { 255 }).collect();
    let modulo: Vec<u8> = (0..256).map(|i| (i % 7) as u8).collect();

    for flags in 0..32u32 {
        for p in [&ramp, &same, &alt, &modulo] {
            for param1 in [0, 2, 3, 4, 5, 128, 255, -7] {
                for param2 in [0, 1] {
                    assert!(!c_clobbers_live_locals(flags, param1, p.len()));
                    assert_same(&encode(flags, param1, param2, p));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generated sweep
// ---------------------------------------------------------------------------

/// Deterministic pseudo-random sweep over the well-defined input domain.
#[test]
fn randomized_sweep() {
    // xorshift64* — deterministic, no external dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    for _ in 0..1500 {
        let flags = (next() % 32) as u32;
        let param1 = match next() % 5 {
            0 => (next() % 17) as i32 - 8,
            1 => (next() % 601) as i32 - 300,
            2 => 1,
            3 => (next() % 256) as i32,
            _ => next() as i32,
        };
        let param2 = [0i32, 1, -1, 12345][(next() % 4) as usize];
        let length = match next() % 3 {
            0 => (next() % 17) as usize,
            1 => (next() % 65) as usize,
            _ => (next() % 141) as usize, // <= 140: growth can never leave the defined domain
        };
        let mode = next() % 5;
        let values: Vec<u8> = (0..length)
            .map(|i| match mode {
                0 => (next() % 256) as u8,
                1 => (next() % 2) as u8,
                2 => {
                    if next() % 8 == 0 {
                        6
                    } else {
                        5
                    }
                }
                3 => (i % 3) as u8,
                _ => {
                    if i % 2 == 0 {
                        0
                    } else {
                        255
                    }
                }
            })
            .collect();

        if c_clobbers_live_locals(flags, param1, length) {
            continue;
        }
        assert_same(&encode(flags, param1, param2, &values));
    }
}

// ---------------------------------------------------------------------------
// Byte-level and lexical oddities
// ---------------------------------------------------------------------------

/// Inputs that are not valid UTF-8, or that contain characters `scanf` treats as
/// neither whitespace nor digits (a NUL byte is a matching failure, not a
/// terminator, because the program reads a stream and not a C string).
#[test]
fn exotic_byte_level_inputs() {
    let long_digits: Vec<u8> = std::iter::repeat(b'9').take(400).collect();
    let mut neg_long = vec![b'-'];
    neg_long.extend_from_slice(&long_digits);

    let mut plus_256 = b"0 0 0 +256".to_vec();
    for _ in 0..256 {
        plus_256.extend_from_slice(b" 1");
    }

    let mut long_ws = vec![b' '; 5000];
    long_ws.extend_from_slice(b"3 2 1 4 10 20 30 40");

    let mut p1_long = b"0 ".to_vec();
    p1_long.extend_from_slice(&long_digits);
    p1_long.extend_from_slice(b" 0 2 1 2");

    let mut p1_neg_long = b"0 ".to_vec();
    p1_neg_long.extend_from_slice(&neg_long);
    p1_neg_long.extend_from_slice(b" 0 2 1 2");

    let mut len_long = b"0 0 0 ".to_vec();
    len_long.extend_from_slice(&long_digits);

    let cases: Vec<Vec<u8>> = vec![
        b"\x00 1 2 3".to_vec(),                  // NUL where flags belong
        b"3 2 1 4 10 \x00 20 30 40".to_vec(),    // NUL mid-stream
        (1u8..40).collect(),                     // binary garbage
        vec![0xff, 0xfe, 0xfd],                  // invalid UTF-8
        b"0 0 0 -0".to_vec(),                    // strtoul("-0") == 0
        b"-0 -0 -0 2 1 2".to_vec(),
        plus_256,
        b"00000000000000000003 0000002 001 4 0010 020 030 040".to_vec(),
        len_long,                                // strtoul saturation
        p1_long,                                 // strtol saturation, positive
        p1_neg_long,                             // strtol saturation, negative
        b"\t\x0b\x0c3\x0b2\x0c1\t4 10 20 30 40".to_vec(),
        b"0 0 0 3 4294967296 4294967297 18446744073709551616".to_vec(),
        b"0 0 0 256".to_vec(),                   // max length then immediate EOF
        b"3 2 1 4 10 20 30".to_vec(),            // EOF one byte short
        b"0 0 0 +".to_vec(),                     // sign only
        b"++3 2 1 0".to_vec(),                   // double sign
        b"- 3 2 1 0".to_vec(),                   // space between sign and digits
        b"1e5 0 0 0".to_vec(),                   // 'e' is not consumed
        b"3.5 2 1 0".to_vec(),                   // '.' is not consumed
        long_ws,
        b"3\r2\r1\r4\r10\r20\r30\r40".to_vec(),  // bare CR separators
        b"0 0 0 1 42".to_vec(),                  // no trailing newline
    ];

    for c in &cases {
        assert_same_bytes(c);
    }
}

// ---------------------------------------------------------------------------
// The undefined-behaviour boundary
// ---------------------------------------------------------------------------

/// Pins the edge of the well-defined domain.
///
/// With `flags = 2` and `param1 = 1` every singleton run becomes a `value,
/// count` pair, so a length of `n` distinct bytes yields a length of `2n` that
/// the C writes through a 256-byte array. The bytes past the array land first on
/// locals `main` has already consumed (`length`, `param2`, `param1`, `flags`,
/// which occupy `buffer[256..280]`), so the two programs still agree; from
/// `buffer[280]` onward the C overwrites `new_length` and the print-loop
/// counter, and past the return address it crashes.
///
/// This test asserts agreement across the whole defined region, up to and
/// including the last length that stays inside it. Behaviour beyond it is
/// undefined in the C and is documented in `ERRORS.md` rather than asserted.
#[test]
fn compact_growth_overflow_boundary_matches() {
    for n in 1..=140usize {
        let values: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
        assert!(!c_clobbers_live_locals(0x02, 1, n));
        assert_same(&encode(2, 1, 0, &values));
    }
}
