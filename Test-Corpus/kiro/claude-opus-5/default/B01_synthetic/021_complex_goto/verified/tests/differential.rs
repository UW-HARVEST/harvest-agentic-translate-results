//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin and require byte-identical stdout, stderr and
//! exit status.
//!
//! Nothing here loads the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them, because that is how they are
//! compared.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Workspace root: the directory holding both `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the compiled C program, building it with CMake on first use if the
/// binary is not present yet. `c_src/` sources are never modified.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = workspace_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run `cmake ..` in c_src/build");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run `cmake --build .` in c_src/build");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(bin.exists(), "expected C binary at {}", bin.display());
        bin
    })
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

/// Spawns `bin` with `input` on stdin and both output streams piped.
fn spawn(bin: &Path, input: &[u8]) -> Child {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    let mut stdin = child.stdin.take().expect("stdin pipe");
    let input = input.to_vec();
    // Write on a helper thread: the child may exit without draining stdin.
    std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
        let _ = stdin.flush();
    });
    child
}

/// Runs a program to completion and captures everything it produced.
///
/// The programs under test terminate immediately for every input used with this
/// helper; the timeout only exists so that a regression shows up as a failure
/// instead of a hung test run.
fn run_to_completion(bin: &Path, input: &[u8]) -> Run {
    let mut child = spawn(bin, input);
    let mut stdout_pipe = child.stdout.take().expect("stdout pipe");
    let mut stderr_pipe = child.stderr.take().expect("stderr pipe");

    // Drain both pipes concurrently so a full pipe buffer cannot deadlock us.
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => break status,
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "{} did not terminate within 20s on input {:?}",
                        bin.display(),
                        String::from_utf8_lossy(input)
                    );
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    };

    Run {
        stdout: stdout_reader.join().expect("stdout reader"),
        stderr: stderr_reader.join().expect("stderr reader"),
        code: status.code(),
    }
}

/// Reads at most `limit` bytes of stdout, then kills the process.
///
/// Used for the inputs where the C program never terminates (see
/// `NON_TERMINATING`): the observable behaviour there is the byte stream itself,
/// so the two programs are compared on a bounded prefix of it.
fn run_bounded(bin: &Path, input: &[u8], limit: u64) -> Vec<u8> {
    let mut child = spawn(bin, input);
    let stdout_pipe = child.stdout.take().expect("stdout pipe");
    let mut buf = Vec::new();
    let _ = stdout_pipe.take(limit).read_to_end(&mut buf);
    let _ = child.kill();
    let _ = child.wait();
    buf
}

/// Asserts stdout, stderr and exit status all match for one input.
fn assert_same(input: &[u8]) {
    let c = run_to_completion(c_bin(), input);
    let r = run_to_completion(rust_bin(), input);

    let shown = String::from_utf8_lossy(input).into_owned();
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {shown:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {shown:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit status differs for input {shown:?}: C {:?} vs Rust {:?}",
        c.code, r.code
    );
}

fn assert_same_str(input: &str) {
    assert_same(input.as_bytes());
}

/// Asserts a bounded prefix of stdout matches, for the inputs on which the C
/// program loops forever.
fn assert_same_prefix(input: &str, limit: u64) {
    let c = run_bounded(c_bin(), input.as_bytes(), limit);
    let r = run_bounded(rust_bin(), input.as_bytes(), limit);
    assert_eq!(
        c.len(),
        limit as usize,
        "expected the C program to keep producing output for {input:?}"
    );
    assert_eq!(
        c, r,
        "stdout prefix differs for input {input:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
}

// ---------------------------------------------------------------------------
// Input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` never assigns anything, so `x` and `y` keep their initialisers of 0
/// and `foo` prints nothing.
#[test]
fn no_conversion_at_all() {
    for input in [
        "",            // empty input: EOF before the first conversion
        " ",           // whitespace only
        "   \t\n  ",   //
        "\n",          //
        "\u{b}",       // lone vertical tab (isspace, but not ASCII-whitespace in Rust)
        "\r\n",        //
        "abc",         // matching failure on the first %d
        "-",           // sign with no digits
        "+",           //
        "- -",         //
        "+x 3",        //
        ".5 2",        //
        ",",           //
        "\0\0",        // NUL bytes
        "\u{feff}5 6", // leading non-ASCII bytes
    ] {
        assert_same_str(input);
    }
}

/// Only the first conversion succeeds, so `y` stays 0.
#[test]
fn single_item_only() {
    for input in [
        "5",       // EOF after the first number
        "5 ",      //
        "5\n",     //
        "0",       //
        "-3",      //
        "5 abc",   // second %d hits a matching failure
        "5 -",     //
        "5 +q",    //
        "7 .5",    //
        "4\t",     //
        "12 x 34", //
    ] {
        assert_same_str(input);
    }
}

/// Both zero: the `while` condition is false on the first evaluation.
#[test]
fn loop_never_entered() {
    for input in [
        "0 0", "-0 -0", "-1 -1", "-5 -7", "0 -1", "-1 0", "0 -2147483648", "-2147483648 0",
        "-2147483648 -2147483648",
    ] {
        assert_same_str(input);
    }
}

/// `x == 1 && y == 4` is the only input that takes the `goto label2` edge on the
/// first iteration, skipping the `x` decrement.
#[test]
fn goto_label2_special_case() {
    assert_same_str("1 4");
}

/// Inputs that reach `if (y == 0) continue;`.
#[test]
fn y_zero_continue_path() {
    for input in ["1 0", "2 0", "3 0", "4 0", "10 0"] {
        assert_same_str(input);
    }
}

/// Only `y` drives the loop; the `if (x > 0)` body at `label1` is never taken.
#[test]
fn x_non_positive_with_positive_y() {
    for input in ["0 1", "0 2", "0 7", "-1 1", "-4 9", "-2147483648 3"] {
        assert_same_str(input);
    }
}

/// The `if (x < 3) goto label1;` back edge, on both sides of the boundary.
#[test]
fn x_less_than_three_back_edge() {
    for input in [
        "1 1", "2 1", "2 2", "3 1", "3 2", "3 3", "4 1", "4 3", "4 4", "5 5", "6 2", "9 9",
    ] {
        assert_same_str(input);
    }
}

/// Exhaustive sweep over every terminating `(x, y)` pair in a window that covers
/// all the constants the C code tests against (0, 1, 3, 4).
#[test]
fn exhaustive_small_grid() {
    for x in -4..=14 {
        for y in -4..=14 {
            // x > 0 && y < 0 never terminates; covered by `non_terminating_inputs`.
            if x > 0 && y < 0 {
                continue;
            }
            assert_same_str(&format!("{x} {y}"));
        }
    }
}

/// `x > 0` together with `y < 0` makes the C program loop forever: `y != 0`
/// keeps printing, and `x < 3` jumps back to `label1` even once `x` is 0.
/// Compared on a bounded prefix of stdout.
#[test]
fn non_terminating_inputs() {
    for input in ["1 -1", "2 -1", "5 -3", "1 -100", "3 -2", "1 -2147483648"] {
        assert_same_prefix(input, 64 * 1024);
    }
}

/// The largest magnitudes the program can be handed. Full runs would take
/// billions of iterations, so these are compared on a bounded stdout prefix.
#[test]
fn extreme_magnitudes_bounded() {
    for input in [
        "2147483647 0",           // INT_MAX
        "2147483647 2147483647",  //
        "2147483646 1",           //
        "-2147483649 5",          // wraps to INT_MAX after truncation to int
        "5 9223372036854775808",  // y truncates to -1 -> non-terminating
        "2147483647 -2147483648", //
    ] {
        assert_same_prefix(input, 64 * 1024);
    }
}

/// `%d` skips arbitrary leading whitespace, including newlines, and the literal
/// space in `"%d %d"` does the same between the two numbers.
#[test]
fn whitespace_handling() {
    for input in [
        "  3   4  ",
        "\n\n3\n4\n",
        "\t3\t4",
        "\r\n3\r\n4",
        "\u{b}3\u{b}4",   // vertical tab
        "\u{c}3\u{c}4",   // form feed
        "\u{b}\u{b}2 5",  //
        " \t\n\u{b}\u{c}\r2 3",
        "3    \n\t  4",
    ] {
        assert_same_str(input);
    }
}

/// Signs and redundant leading zeros.
#[test]
fn signs_and_leading_zeros() {
    for input in [
        "+2 +3",
        "+0 +0",
        "-0 5",
        "5 -0",
        "007 004",
        "0000000000000000000000004 5",
        "4 0000000000000000000000004",
        "+++4 5", // second '+' is a matching failure
        "4 ++5",
        "--4 5",
    ] {
        assert_same_str(input);
    }
}

/// Values outside `int`, converted with a `long` and then truncated, exactly as
/// glibc's `%d` does. `2147483648` becomes `INT_MIN`, values past `LONG_MAX`
/// saturate to `LONG_MAX` and truncate to `-1`.
#[test]
fn integer_overflow_and_truncation() {
    for input in [
        "2147483648 5",                             // INT_MAX + 1 -> INT_MIN
        "4294967296 5",                             // 2^32 -> 0
        "4294967297 5",                             // 2^32 + 1 -> 1
        "8589934593 5",                             // 2^33 + 1 -> 1
        "9223372036854775807 5",                    // LONG_MAX -> -1
        "9223372036854775808 5",                    // above LONG_MAX -> -1
        "99999999999999999999999 5",                //
        "1234567890123456789012345678901234567890 2",
        "-2147483648 5",                            // INT_MIN
        "-9223372036854775808 5",                   // LONG_MIN -> 0
        "-9223372036854775809 5",                   // below LONG_MIN -> 0
        "-99999999999999999999999 5",               //
        "5 4294967296",
        "5 4294967297",
        "5 -9223372036854775809",
    ] {
        assert_same_str(input);
    }
}

/// Truncation results that land in the non-terminating class (`x > 0 && y < 0`),
/// so they are compared on a bounded stdout prefix.
#[test]
fn integer_truncation_non_terminating() {
    for input in [
        "5 9223372036854775807", // LONG_MAX -> y == -1
        "5 4294967295",          // 2^32 - 1 -> y == -1
        "5 2147483648",          // INT_MAX + 1 -> y == INT_MIN
        "3 -4",
    ] {
        assert_same_prefix(input, 64 * 1024);
    }
}

/// Input that stops the conversion partway through, and trailing input that is
/// never read at all.
#[test]
fn partial_and_trailing_input() {
    for input in [
        "0x5 3",   // reads 0, then 'x' fails the second %d
        "5e3 3",   // reads 5, then 'e' fails
        "5.9 2.1", // reads 5, then '.' fails
        "5,6",     //
        "5 6 7 8", // extra numbers are left in the stream
        "1 4 ignored",
        "3 3\nmore text\n",
    ] {
        assert_same_str(input);
    }
}

/// Bytes that are not valid UTF-8 must not change anything: both programs read
/// raw bytes.
#[test]
fn non_utf8_input() {
    for input in [
        b"\xff\xfe 5 6".as_slice(),
        b"5 6\xff\xfe".as_slice(),
        b"\x80\x815 6".as_slice(),
        b"1 4\x00\x00".as_slice(),
    ] {
        assert_same(input);
    }
}

/// Both programs exit 0 and print nothing when stdin is closed immediately.
#[test]
fn closed_stdin() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C program");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust program");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}
