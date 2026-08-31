// Differential tests: run the C binary and the Rust binary as subprocesses,
// feed both the same bytes on stdin, and require that stdout, stderr and the
// exit status match exactly.
//
// The Rust code is never called as a library here. `main.rs` is driven only
// through its compiled executable, because that is how it is compared against
// the C program.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

/// Path to the Rust executable, supplied by Cargo for the `driver` bin target.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // tests/ live in translation/, so the sibling c_src/ is one level up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C executable, building it with CMake on first use if needed.
///
/// Only files under `c_src/build/` are produced; nothing in `c_src/src/` or the
/// `CMakeLists.txt` is touched.
fn c_bin() -> PathBuf {
    static BUILD: Once = Once::new();
    let c_src = repo_root().join("c_src");
    let bin = c_src.join("build").join("driver");

    BUILD.call_once(|| {
        if bin.exists() {
            return;
        }
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let conf = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            conf.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&conf.stderr)
        );
        let built = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("run cmake --build");
        assert!(
            built.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&built.stderr)
        );
    });

    assert!(bin.exists(), "C binary missing at {}", bin.display());
    bin
}

/// What a run of either program produced.
#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} status={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            match self.status {
                Ok(c) => format!("exit {c}"),
                Err(s) => format!("signal {s}"),
            }
        )
    }
}

/// Run `bin` with `input` on stdin, capturing both output streams.
fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // The child may exit without draining stdin (scanf stops early), so a
    // failed write is expected and must not fail the test.
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: match out.status.code() {
            Some(c) => Ok(c),
            None => Err(out.status.signal().expect("exited by signal")),
        },
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label} (input {:?})\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label} (input {:?})\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status differs for {label} (input {:?})\n  C: {:?}\n  R: {:?}",
        String::from_utf8_lossy(input),
        c.status,
        r.status
    );
}

/// Assert agreement and additionally pin the exact bytes the C program emits, so
/// the pair cannot drift together into being wrong.
fn assert_same_and_stdout(label: &str, input: &[u8], expected_stdout: &str) {
    assert_same(label, input);
    let c = run(&c_bin(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_stdout,
        "C stdout for {label} is not what the test expected"
    );
    assert!(c.stderr.is_empty(), "unexpected stderr for {label}");
    assert_eq!(c.status, Ok(0), "unexpected status for {label}");
}

// ---------------------------------------------------------------------------
// Phase A: both programs are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = run(&c_bin(), b"0");
    let r = run(&rust_bin(), b"0");
    assert_eq!(c.status, Ok(0));
    assert_eq!(r.status, Ok(0));
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// Phase B: the branches scanf("%d") and print_hex actually take.
// ---------------------------------------------------------------------------

// --- input failure: EOF reached while skipping leading whitespace ---
// `x` keeps its initializer of 0, so print_hex emits four zero bytes.

#[test]
fn empty_input_leaves_x_at_zero() {
    assert_same_and_stdout("empty input", b"", "00000000\n");
}

#[test]
fn whitespace_only_inputs_leave_x_at_zero() {
    // Every character isspace() accepts, alone and combined. Each reaches the
    // "skip whitespace then hit EOF" path.
    for (label, input) in [
        ("space", " " as &str),
        ("tab", "\t"),
        ("newline", "\n"),
        ("vertical tab", "\u{0b}"),
        ("form feed", "\u{0c}"),
        ("carriage return", "\r"),
        ("crlf", "\r\n"),
        ("all whitespace", " \t\n\u{0b}\u{0c}\r "),
        ("many newlines", "\n\n\n\n\n"),
    ] {
        assert_same_and_stdout(label, input.as_bytes(), "00000000\n");
    }
}

// --- successful conversion: the ordinary path ---

#[test]
fn single_item_zero() {
    assert_same_and_stdout("zero", b"0", "00000000\n");
}

#[test]
fn single_item_one() {
    assert_same_and_stdout("one", b"1", "01000000\n");
}

#[test]
fn small_positive_values() {
    assert_same_and_stdout("42", b"42", "2a000000\n");
    assert_same_and_stdout("255", b"255", "ff000000\n");
    assert_same_and_stdout("256", b"256", "00010000\n");
    assert_same_and_stdout("65535", b"65535", "ffff0000\n");
    assert_same_and_stdout("65536", b"65536", "00000100\n");
    assert_same_and_stdout("16777216", b"16777216", "00000001\n");
}

#[test]
fn every_byte_position_is_distinct() {
    // 0x04030201 exercises all four indices of the print_hex loop with distinct
    // values, so a wrong byte order or a wrong index cannot pass by accident.
    assert_same_and_stdout("0x04030201", b"67305985", "01020304\n");
}

#[test]
fn negative_values() {
    assert_same_and_stdout("-1", b"-1", "ffffffff\n");
    assert_same_and_stdout("-2", b"-2", "feffffff\n");
    assert_same_and_stdout("-256", b"-256", "00ffffff\n");
    assert_same_and_stdout("negative zero", b"-0", "00000000\n");
}

#[test]
fn int_limits() {
    assert_same_and_stdout("INT_MAX", b"2147483647", "ffffff7f\n");
    assert_same_and_stdout("INT_MIN", b"-2147483648", "00000080\n");
}

// --- the optional sign branch ---

#[test]
fn explicit_plus_sign_is_accepted() {
    assert_same_and_stdout("+5", b"+5", "05000000\n");
    assert_same_and_stdout("+0", b"+0", "00000000\n");
    assert_same_and_stdout("+2147483647", b"+2147483647", "ffffff7f\n");
}

#[test]
fn sign_then_eof_is_a_matching_failure() {
    // scanf consumes the sign, finds no digit, and reports a matching failure;
    // `x` is never written, so it stays 0.
    assert_same_and_stdout("lone minus", b"-", "00000000\n");
    assert_same_and_stdout("lone plus", b"+", "00000000\n");
}

#[test]
fn sign_then_non_digit_is_a_matching_failure() {
    assert_same_and_stdout("minus space digit", b"-  5", "00000000\n");
    assert_same_and_stdout("minus newline digit", b"-\n5", "00000000\n");
    assert_same_and_stdout("double minus", b"--5", "00000000\n");
    assert_same_and_stdout("minus plus", b"-+5", "00000000\n");
    assert_same_and_stdout("plus then letter", b"+a", "00000000\n");
    assert_same_and_stdout("space plus space digit", b"  +  1", "00000000\n");
}

// --- the "no digits at all" matching failure ---

#[test]
fn non_numeric_input_is_a_matching_failure() {
    for (label, input) in [
        ("letters", &b"abc"[..]),
        ("leading dot", b".5"),
        ("comma", b","),
        ("hex prefix has no digits after skip", b"x10"),
        ("underscore", b"_1"),
        ("hash", b"#"),
        ("slash", b"/"), // the byte just below '0'
        ("colon", b":"), // the byte just above '9'
        ("nul byte", b"\x00"),
        ("nul then digit", b"\x005"),
        ("high byte", b"\xff"),
        ("utf8 e-acute", b"\xc3\xa9"),
        ("newline then letter", b"\nz"),
    ] {
        assert_same_and_stdout(label, input, "00000000\n");
    }
}

// --- leading whitespace is skipped across newlines (scanf, not fgets) ---

#[test]
fn leading_whitespace_including_newlines_is_skipped() {
    assert_same_and_stdout("newline then 42", b"\n   42", "2a000000\n");
    assert_same_and_stdout("multiline then 7", b"\n\n\n7", "07000000\n");
    assert_same_and_stdout("crlf then 3", b"\r\n3", "03000000\n");
    assert_same_and_stdout("mixed ws then 8", b"\x0b\x0c\t 8", "08000000\n");
}

// --- the digit loop stops at the first non-digit, and only the first item is read ---

#[test]
fn conversion_stops_at_first_non_digit() {
    assert_same_and_stdout("trailing letters", b"42abc", "2a000000\n");
    assert_same_and_stdout("trailing newline", b"42\n", "2a000000\n");
    assert_same_and_stdout("trailing junk lines", b"42\nignored\n", "2a000000\n");
    assert_same_and_stdout("digit then nul", b"5\x00", "05000000\n");
    assert_same_and_stdout("decimal point", b"3.9", "03000000\n");
    assert_same_and_stdout("hex literal reads 0", b"0x10", "00000000\n");
}

#[test]
fn only_the_first_of_several_integers_is_read() {
    assert_same_and_stdout("two ints", b"7 9", "07000000\n");
    assert_same_and_stdout("three lines", b"1\n2\n3\n", "01000000\n");
}

#[test]
fn leading_zeros_are_not_octal() {
    assert_same_and_stdout("010", b"010", "0a000000\n");
    assert_same_and_stdout("many leading zeros", b"0000000000000000042", "2a000000\n");
    assert_same_and_stdout("negative leading zeros", b"-000000001", "ffffffff\n");
}

// --- out-of-range: strtol saturates at the long limits, then the store to an
//     int truncates. Both effects must be reproduced. ---

#[test]
fn values_above_int_max_truncate() {
    assert_same_and_stdout("INT_MAX+1", b"2147483648", "00000080\n");
    assert_same_and_stdout("INT_MAX+2", b"2147483649", "01000080\n");
    assert_same_and_stdout("2^32", b"4294967296", "00000000\n");
    assert_same_and_stdout("2^32+1", b"4294967297", "01000000\n");
    assert_same_and_stdout("2^32+42", b"4294967338", "2a000000\n");
}

#[test]
fn values_below_int_min_truncate() {
    assert_same_and_stdout("INT_MIN-1", b"-2147483649", "ffffff7f\n");
    assert_same_and_stdout("-2^32", b"-4294967296", "00000000\n");
    assert_same_and_stdout("-2^32-1", b"-4294967297", "ffffffff\n");
}

#[test]
fn long_limits_saturate_then_truncate() {
    // LONG_MAX = 0x7fff_ffff_ffff_ffff, low 32 bits are all ones.
    assert_same_and_stdout("LONG_MAX", b"9223372036854775807", "ffffffff\n");
    // LONG_MIN = 0x8000_0000_0000_0000, low 32 bits are all zero.
    assert_same_and_stdout("LONG_MIN", b"-9223372036854775808", "00000000\n");
    // Just past the limits: strtol clamps, so the result is unchanged.
    assert_same_and_stdout("LONG_MAX+1", b"9223372036854775808", "ffffffff\n");
    assert_same_and_stdout("LONG_MIN-1", b"-9223372036854775809", "00000000\n");
}

#[test]
fn wildly_out_of_range_values_saturate() {
    assert_same_and_stdout("20 nines", b"99999999999999999999", "ffffffff\n");
    assert_same_and_stdout("20 nines negative", b"-99999999999999999999", "00000000\n");
}

#[test]
fn very_long_digit_runs() {
    // Far longer than any internal scanf buffer, to be sure the saturation
    // decision does not depend on how the digits are chunked.
    for n in [64usize, 100, 1000, 5000] {
        let nines = "9".repeat(n);
        assert_same_and_stdout(&format!("{n} nines"), nines.as_bytes(), "ffffffff\n");
        assert_same_and_stdout(
            &format!("{n} nines negative"),
            format!("-{nines}").as_bytes(),
            "00000000\n",
        );
        // Leading zeros do not contribute magnitude, so no saturation occurs
        // however many there are.
        let padded = format!("{}7", "0".repeat(n));
        assert_same_and_stdout(&format!("{n} zeros then 7"), padded.as_bytes(), "07000000\n");
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths not covered above.
// ---------------------------------------------------------------------------

#[test]
fn stdin_at_immediate_eof_from_dev_null() {
    // Not a pipe: /dev/null returns EOF on the very first read.
    let mut outs = Vec::new();
    for bin in [c_bin(), rust_bin()] {
        let dev_null = std::fs::File::open("/dev/null").expect("open /dev/null");
        let out = Command::new(&bin)
            .stdin(Stdio::from(dev_null))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with /dev/null stdin");
        outs.push((out.stdout, out.stderr, out.status.code()));
    }
    assert_eq!(outs[0], outs[1], "C and Rust differ with /dev/null on stdin");
    assert_eq!(outs[0].0, b"00000000\n", "unexpected stdout on empty stdin");
    assert_eq!(outs[0].2, Some(0));
}

#[test]
fn broken_stdout_pipe_kills_both_the_same_way() {
    // A C program dies from SIGPIPE when it writes to a pipe with no reader.
    // The Rust runtime masks SIGPIPE by default, which would make the Rust
    // program exit 0 here; main.rs restores the default disposition so the two
    // agree. Compared as a raw wait status, which encodes the signal.
    fn status_with_dead_stdout(bin: &Path) -> Result<i32, i32> {
        // `head -c 0` exits immediately, leaving the pipe's read end closed.
        let reader = Command::new("head")
            .args(["-c", "0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn head");
        let pipe = reader.stdin.expect("head stdin");
        // Give `head` a moment to exit so the read end is genuinely gone.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(pipe))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn subject");
        let _ = child.stdin.take().expect("subject stdin").write_all(b"1");
        let st = child.wait().expect("wait");
        match st.code() {
            Some(c) => Ok(c),
            None => Err(st.signal().expect("signal")),
        }
    }

    let c = status_with_dead_stdout(&c_bin());
    let r = status_with_dead_stdout(&rust_bin());
    assert_eq!(
        c, r,
        "broken-pipe exit status differs: C {c:?} vs Rust {r:?}"
    );
}

#[test]
fn full_stdout_device_is_handled_the_same() {
    // Writes to /dev/full fail with ENOSPC. The C program never checks printf's
    // return value, so it still exits 0; the Rust program must too.
    fn status_to_dev_full(bin: &Path) -> Option<i32> {
        let full = match std::fs::OpenOptions::new().write(true).open("/dev/full") {
            Ok(f) => f,
            Err(_) => return None, // /dev/full absent: nothing to compare
        };
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(full))
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let _ = child.stdin.take().expect("stdin").write_all(b"1");
        child.wait().expect("wait").code()
    }
    let c = status_to_dev_full(&c_bin());
    let r = status_to_dev_full(&rust_bin());
    assert_eq!(c, r, "exit status to /dev/full differs");
}

#[test]
fn command_line_arguments_are_ignored() {
    // main() takes no parameters, so extra argv entries must change nothing.
    for args in [vec!["--help"], vec!["99"], vec!["-x", "-y"]] {
        let mut outs = Vec::new();
        for bin in [c_bin(), rust_bin()] {
            let mut child = Command::new(&bin)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn with args");
            let _ = child.stdin.take().expect("stdin").write_all(b"5");
            let out = child.wait_with_output().expect("wait");
            outs.push((out.stdout, out.stderr, out.status.code()));
        }
        assert_eq!(outs[0], outs[1], "differ with args {args:?}");
        assert_eq!(outs[0].0, b"05000000\n");
    }
}

#[test]
fn output_is_exactly_nine_bytes_with_one_trailing_newline() {
    // print_hex writes 2 hex digits per byte of an int plus a single '\n'.
    for input in [&b""[..], b"0", b"-1", b"2147483647", b"abc"] {
        let c = run(&c_bin(), input);
        let r = run(&rust_bin(), input);
        assert_eq!(c.stdout.len(), 9, "C stdout length for {input:?}");
        assert_eq!(r.stdout.len(), 9, "Rust stdout length for {input:?}");
        assert_eq!(*c.stdout.last().unwrap(), b'\n');
        assert!(
            c.stdout[..8]
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
            "C hex digits must be lowercase for {input:?}"
        );
        assert_eq!(c.stdout, r.stdout);
    }
}

#[test]
fn neither_program_writes_to_stderr() {
    for input in [&b""[..], b"abc", b"-", b"99999999999999999999", b"7"] {
        let c = run(&c_bin(), input);
        let r = run(&rust_bin(), input);
        assert!(c.stderr.is_empty(), "C wrote stderr for {input:?}");
        assert!(r.stderr.is_empty(), "Rust wrote stderr for {input:?}");
    }
}

// ---------------------------------------------------------------------------
// Broad sweeps: exhaustive over a dense region, plus deterministic fuzzing.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_small_integers() {
    for v in -600i64..=600 {
        assert_same(&format!("value {v}"), v.to_string().as_bytes());
    }
}

#[test]
fn boundary_neighbourhoods() {
    // A few units either side of every limit the conversion can hit.
    let anchors: [i128; 6] = [
        0,
        i32::MAX as i128,
        i32::MIN as i128,
        1i128 << 32,
        i64::MAX as i128,
        i64::MIN as i128,
    ];
    for a in anchors {
        for d in -3i128..=3 {
            let v = a + d;
            assert_same(&format!("boundary {v}"), v.to_string().as_bytes());
        }
    }
}

#[test]
fn powers_of_two_and_their_neighbours() {
    for bit in 0..40u32 {
        let p = 1i128 << bit;
        for v in [p - 1, p, p + 1, -p - 1, -p, -p + 1] {
            assert_same(&format!("pow2 {v}"), v.to_string().as_bytes());
        }
    }
}

#[test]
fn deterministic_fuzz_over_scanf_relevant_bytes() {
    // A small xorshift keeps this reproducible without adding a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    // Bytes that steer the conversion: digits, signs, every whitespace
    // character, and representative rejects.
    const ALPHABET: &[u8] = b"0123456789+-      \t\n\r\x0b\x0cabcxX.,_\x00\xff";

    for case in 0..1500 {
        let len = (next() % 15) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("fuzz case {case}"), &input);
    }
}

#[test]
fn deterministic_fuzz_over_long_digit_strings() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for case in 0..300 {
        let digits = (next() % 40) as usize + 1;
        let mut s = String::new();
        if next() % 2 == 0 {
            s.push(if next() % 2 == 0 { '-' } else { '+' });
        }
        for _ in 0..digits {
            s.push((b'0' + (next() % 10) as u8) as char);
        }
        assert_same(&format!("long-digit fuzz case {case}"), s.as_bytes());
    }
}
