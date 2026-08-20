// Differential test harness: runs the ORIGINAL C binary and the TRANSLATED
// Rust binary against identical stdin and requires byte-identical results.
//
// The C reference (`c_src/CMakeLists.txt`) is an `add_executable`, and
// `nm -D` on it shows ZERO defined dynamic symbols (see SYMBOLS.md). There is
// no C `.so` and no exported function to resolve, so the complete observable
// surface of this program is the process boundary:
//
//     stdin bytes -> (stdout bytes, stderr bytes, exit status / signal)
//
// Every assertion below therefore drives both implementations as external
// processes, exactly as a real consumer invokes them, and compares the full
// outcome triple. `libloading` is present in [dev-dependencies] as required but
// has nothing to open here; loading an executable that exports no symbols would
// measure the harness rather than the translation.

use std::fs;
use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------- outcome ---

#[derive(PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?} stdout={:?} stderr={:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

// ------------------------------------------------------------ binary paths ---

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Builds the C reference with CMake if it is not already present.
fn c_bin() -> PathBuf {
    let root = manifest_dir().join("c_src");
    let build = root.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }
    fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed: {}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let out = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake build");
    assert!(
        out.status.success(),
        "cmake build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(bin.exists(), "C binary missing after build");
    bin
}

// -------------------------------------------------------------- temp files ---

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(dir).join(format!("diff_{}_{}_{}", tag, std::process::id(), n))
}

fn write_temp(input: &[u8]) -> PathBuf {
    let p = temp_path("in");
    let mut f = fs::File::create(&p).expect("create temp stdin");
    f.write_all(input).expect("write temp stdin");
    f.sync_all().ok();
    p
}

// ------------------------------------------------------------ run variants ---

/// Normal invocation: stdin is a regular file containing `input`.
fn run(exe: &Path, input: &[u8]) -> Outcome {
    let path = write_temp(input);
    let stdin = fs::File::open(&path).expect("open temp stdin");
    let out = Command::new(exe)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn driver");
    let _ = fs::remove_file(&path);
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Invocation with a file descriptor closed in the child before `exec`.
fn run_with_closed_fd(exe: &Path, input: &[u8], fd: i32) -> Outcome {
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    let path = write_temp(input);
    let stdin = fs::File::open(&path).expect("open temp stdin");
    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::from(stdin));
    if fd == 1 {
        cmd.stdout(Stdio::null());
    } else {
        cmd.stdout(Stdio::piped());
    }
    cmd.stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(move || {
            close(fd);
            Ok(())
        });
    }
    let out = cmd.output().expect("spawn driver");
    let _ = fs::remove_file(&path);
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Invocation whose stdout is a pipe with **no reader at all**, deterministically.
///
/// The read end is closed in the parent *before* the child is spawned, so the
/// write end has zero readers from the outset and no other concurrently-forked
/// process can ever inherit a reader. (Closing it *after* `spawn` is racy: an
/// unrelated test's child, forked in the window before its own `exec`, briefly
/// holds a copy and suppresses the signal.) The child's first `write` to fd 1
/// therefore always raises `SIGPIPE`. The C reference dies (`signal = 13`); Rust
/// must do the same, which requires undoing the runtime's `SIG_IGN` default.
fn run_with_readerless_stdout(exe: &Path, input: &[u8]) -> Outcome {
    extern "C" {
        fn pipe2(fds: *mut i32, flags: i32) -> i32;
        fn close(fd: i32) -> i32;
    }
    const O_CLOEXEC: i32 = 0o2000000;

    let path = write_temp(input);
    let stdin = fs::File::open(&path).expect("open temp stdin");

    let mut fds = [-1i32; 2];
    let rc = unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) };
    assert_eq!(rc, 0, "pipe2 failed");
    let (read_fd, write_fd) = (fds[0], fds[1]);

    let write_end = unsafe {
        use std::os::fd::FromRawFd;
        fs::File::from_raw_fd(write_fd)
    };

    // No reader exists from this point on: fully deterministic SIGPIPE.
    unsafe { close(read_fd) };

    let child = Command::new(exe)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(write_end)) // dup2 -> fd 1, CLOEXEC cleared
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn driver");

    let out = child.wait_with_output().expect("wait driver");
    let _ = fs::remove_file(&path);
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

// -------------------------------------------------------------- assertions ---

fn assert_same_labeled(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);
    assert_eq!(
        c,
        r,
        "\nDIVERGENCE [{}]\n  stdin  : {:?}{}\n  C   -> {:?}\n  Rust-> {:?}\n",
        label,
        String::from_utf8_lossy(&input[..input.len().min(60)]),
        if input.len() > 60 {
            format!(" (+{} more bytes)", input.len() - 60)
        } else {
            String::new()
        },
        c,
        r
    );
}

fn assert_same(input: &[u8]) {
    assert_same_labeled("case", input)
}

fn assert_same_all(label: &str, inputs: &[&[u8]]) {
    for i in inputs {
        assert_same_labeled(label, i);
    }
}

/// Confirms both agree AND pins the expected C observable, so the test would
/// catch "both changed together" as well as a pure divergence.
fn assert_same_and_stdout(label: &str, input: &[u8], expected_stdout: &str) {
    assert_same_labeled(label, input);
    let c = run(&c_bin(), input);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        expected_stdout,
        "C reference stdout changed for [{}]",
        label
    );
    assert_eq!(c.code, Some(0), "C reference exit code for [{}]", label);
}

// ----------------------------------------------------------------- seeded RNG ---

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

// ===========================================================================
// Phase B — CONFIGS.md rows 1-23 (valid-path differential coverage)
// ===========================================================================

/// CONFIGS row 1 — x == 0 via plain "0" -> bad()
#[test]
fn cfg_zero_plain() {
    assert_same_and_stdout("zero plain", b"0", "0\n");
}

/// CONFIGS row 2 — leading zeros (incl. 500) still parse to 0
#[test]
fn cfg_zero_many_leading() {
    assert_same_and_stdout("0000", b"0000", "0\n");
    let z = vec![b'0'; 500];
    assert_same_and_stdout("500 zeros", &z, "0\n");
    let z = vec![b'0'; 4096];
    assert_same_and_stdout("4096 zeros", &z, "0\n");
}

/// CONFIGS row 3 — signed zero, both signs
#[test]
fn cfg_signed_zero() {
    assert_same_and_stdout("+0", b"+0", "0\n");
    assert_same_and_stdout("-0", b"-0", "0\n");
    assert_same_and_stdout("-00000", b"-00000", "0\n");
}

/// CONFIGS row 4 — every C-locale whitespace byte as a leading skip
#[test]
fn cfg_zero_leading_whitespace() {
    for ws in [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        let mut v = vec![ws; 3];
        v.push(b'0');
        assert_same_and_stdout("ws+0", &v, "0\n");
        let mut v = vec![ws; 3];
        v.push(b'7');
        assert_same_and_stdout("ws+7", &v, "5\n");
    }
    assert_same_and_stdout("mixed ws", b" \t\n\x0b\x0c\r 0", "0\n");
    assert_same_and_stdout("mixed ws nz", b" \t\n\x0b\x0c\r 42", "5\n");
}

/// CONFIGS row 5 — trailing bytes after the conversion are left unconsumed
#[test]
fn cfg_zero_trailing_junk() {
    assert_same_all(
        "trailing junk",
        &[b"0abc", b"0 ", b"0\n9", b"0\t", b"0.5", b"0-1", b"5xyz", b"5 5"],
    );
}

/// CONFIGS row 6 — smallest nonzero -> good()
#[test]
fn cfg_nonzero_one() {
    assert_same_and_stdout("one", b"1", "5\n");
}

/// CONFIGS row 7 — negative values -> good()
#[test]
fn cfg_nonzero_negative() {
    assert_same_and_stdout("-1", b"-1", "5\n");
    assert_same_and_stdout("-3", b"-3", "5\n");
}

/// CONFIGS row 8 — explicit plus sign
#[test]
fn cfg_nonzero_plus_sign() {
    assert_same_and_stdout("+9", b"+9", "5\n");
    assert_same_and_stdout("+00009", b"+00009", "5\n");
}

/// CONFIGS row 9 — INT_MAX / INT_MIN exactly
#[test]
fn cfg_int_boundaries() {
    assert_same_and_stdout("INT_MAX", b"2147483647", "5\n");
    assert_same_and_stdout("INT_MIN", b"-2147483648", "5\n");
}

/// CONFIGS row 10 — 2^31 truncates to INT_MIN (nonzero) -> good()
#[test]
fn cfg_trunc_int_min() {
    assert_same_and_stdout("2^31", b"2147483648", "5\n");
}

/// CONFIGS row 11 — values whose low 32 bits are zero flip the branch to bad()
#[test]
fn cfg_trunc_low32_zero() {
    assert_same_and_stdout("2^32", b"4294967296", "0\n");
    assert_same_and_stdout("2^33", b"8589934592", "0\n");
    for m in 1u64..40 {
        let v = m * (1u64 << 32);
        assert_same_and_stdout("m*2^32", v.to_string().as_bytes(), "0\n");
        let s = format!("-{}", v);
        assert_same_and_stdout("-m*2^32", s.as_bytes(), "0\n");
    }
}

/// CONFIGS row 12 — positive saturation to LONG_MAX (low32 = 0xFFFFFFFF)
#[test]
fn cfg_saturate_positive() {
    assert_same_and_stdout("LONG_MAX", b"9223372036854775807", "5\n");
    assert_same_and_stdout("LONG_MAX+1", b"9223372036854775808", "5\n");
    assert_same_and_stdout("2^64", b"18446744073709551616", "5\n");
    assert_same_and_stdout("30 nines", b"999999999999999999999999999999", "5\n");
}

/// CONFIGS row 13 — negative saturation to LONG_MIN (low32 = 0) -> bad()
#[test]
fn cfg_saturate_negative() {
    assert_same_and_stdout("LONG_MIN", b"-9223372036854775808", "0\n");
    assert_same_and_stdout("LONG_MIN-1", b"-9223372036854775809", "0\n");
    assert_same_and_stdout("-2^64", b"-18446744073709551616", "0\n");
    assert_same_and_stdout("-30 nines", b"-999999999999999999999999999999", "0\n");
}

/// CONFIGS row 14 — matching failure on leading non-numeric bytes
#[test]
fn cfg_matching_failure() {
    assert_same_all(
        "matching failure",
        &[b"abc", b".", b"x", b"@", b"/", b":", b"`", b"{", b"~", b"e5", b"X1"],
    );
}

/// CONFIGS row 15 — input failure: empty stdin
#[test]
fn cfg_input_failure_eof() {
    assert_same_and_stdout("empty", b"", "0\n");
}

/// CONFIGS row 16 — sign followed by a non-digit is a matching failure
#[test]
fn cfg_sign_then_nondigit() {
    assert_same_all(
        "sign then non-digit",
        &[b"+", b"-", b"- 5", b"+ 5", b"--5", b"++5", b"+a", b"-z", b"+-1", b"-.5"],
    );
}

/// CONFIGS row 17 — bytes that resemble blanks but are not C-locale whitespace
/// must NOT be skipped (they cause a matching failure instead).
#[test]
fn cfg_non_locale_blank_bytes() {
    for b in [0x85u8, 0xa0, 0x00, 0x1c, 0x1d, 0x1e, 0x1f, 0x7f, 0xff] {
        assert_same_and_stdout("blank-ish byte + 5", &[b, b'5'], "0\n");
    }
}

/// CONFIGS row 18 — oversized digit runs, signed and unsigned
#[test]
fn cfg_oversized_digit_runs() {
    for n in [100usize, 500, 1000, 4096, 20000] {
        let nines = vec![b'9'; n];
        assert_same_labeled("nines run", &nines);
        let mut neg = vec![b'-'];
        neg.extend_from_slice(&nines);
        assert_same_labeled("negative nines run", &neg);
        let mut padded = vec![b'0'; n];
        padded.extend_from_slice(b"4294967296");
        assert_same_and_stdout("zero-padded 2^32", &padded, "0\n");
    }
}

/// CONFIGS row 19 — oversized whitespace run
#[test]
fn cfg_oversized_whitespace() {
    let mut v = vec![b' '; 10000];
    assert_same_and_stdout("10k spaces then EOF", &v, "0\n");
    v.push(b'7');
    assert_same_and_stdout("10k spaces then 7", &v, "5\n");
    let mut v = vec![b'\n'; 10000];
    v.push(b'0');
    assert_same_and_stdout("10k newlines then 0", &v, "0\n");
}

/// CONFIGS row 20 — every power of two 2^0..2^67, both signs
#[test]
fn cfg_all_powers_of_two() {
    for e in 0..68u32 {
        let v: u128 = 1u128 << e;
        assert_same_labeled("2^e", v.to_string().as_bytes());
        assert_same_labeled("-2^e", format!("-{}", v).as_bytes());
        // one step either side of the boundary
        assert_same_labeled("2^e-1", (v - 1).to_string().as_bytes());
        assert_same_labeled("2^e+1", (v + 1).to_string().as_bytes());
    }
}

/// CONFIGS row 21 — stdin file descriptor closed outright
#[test]
fn cfg_stdin_closed() {
    let c = run_with_closed_fd(&c_bin(), b"0", 0);
    let r = run_with_closed_fd(&rust_bin(), b"0", 0);
    assert_eq!(c, r, "\nDIVERGENCE [stdin closed]\n C-> {:?}\n R-> {:?}\n", c, r);
}

/// CONFIGS row 22a — stdout file descriptor closed outright
#[test]
fn cfg_stdout_closed() {
    for input in [&b"0"[..], &b"1"[..]] {
        let c = run_with_closed_fd(&c_bin(), input, 1);
        let r = run_with_closed_fd(&rust_bin(), input, 1);
        assert_eq!(
            c, r,
            "\nDIVERGENCE [stdout closed, stdin={:?}]\n C-> {:?}\n R-> {:?}\n",
            String::from_utf8_lossy(input), c, r
        );
    }
}

/// CONFIGS row 22b / ERRORS row 12 — stdout is a pipe with no reader.
///
/// This is the regression test for the SIGPIPE divergence: the Rust runtime
/// installs SIG_IGN for SIGPIPE before main, so without an explicit reset the
/// Rust build exits 0 while the C reference is killed by signal 13.
#[test]
fn cfg_stdout_epipe_sigpipe_parity() {
    for input in [&b"0"[..], &b"1"[..], &b"abc"[..]] {
        let c = run_with_readerless_stdout(&c_bin(), input);
        let r = run_with_readerless_stdout(&rust_bin(), input);
        assert_eq!(
            c, r,
            "\nDIVERGENCE [readerless stdout, stdin={:?}]\n C-> {:?}\n R-> {:?}\n",
            String::from_utf8_lossy(input), c, r
        );
        assert_eq!(
            c.signal,
            Some(13),
            "expected the C reference to die from SIGPIPE, got {:?}",
            c
        );
    }
}

/// CONFIGS row 23 — non-decimal numeric prefixes; %d is base 10 only
#[test]
fn cfg_non_decimal_prefix() {
    assert_same_and_stdout("0x10", b"0x10", "0\n");
    assert_same_and_stdout("0b1", b"0b1", "0\n");
    assert_same_and_stdout("1e5", b"1e5", "5\n");
    assert_same_and_stdout("1,5", b"1,5", "5\n");
    assert_same_and_stdout("1_0", b"1_0", "5\n");
    assert_same_and_stdout("010", b"010", "5\n");
    assert_same_and_stdout("0X0", b"0X0", "0\n");
}

/// CONFIGS row 25 — exhaustive sweep of every small integer -1000..=1000
#[test]
fn cfg_exhaustive_small_ints() {
    for v in -1000i32..=1000 {
        let s = v.to_string();
        let expected = if v == 0 { "0\n" } else { "5\n" };
        assert_same_and_stdout("small int", s.as_bytes(), expected);
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows 1-12 (error/rejection-path differential coverage)
//
// The C program has no explicit error handling: `scanf`'s return value is
// discarded and `main` always returns 0, so every rejection is absorbed into
// the `x == 0` path and must yield exactly `stdout == "0\n"`, exit 0. Each test
// asserts BOTH that C and Rust agree AND that the specific sentinel (that exact
// stdout + exit code) is produced -- not merely that "both failed somehow".
// ===========================================================================

/// ERRORS row 1 — matching failure, leading non-numeric byte
#[test]
fn err_matching_failure_alpha() {
    for inp in [&b"abc"[..], b".", b"x", b"@", b"/", b":", b"["] {
        assert_same_and_stdout("matching failure", inp, "0\n");
    }
}

/// ERRORS row 2 — input failure, empty stdin / immediate EOF
#[test]
fn err_input_failure_empty() {
    assert_same_and_stdout("empty stdin", b"", "0\n");
}

/// ERRORS row 3 — whitespace-only input then EOF
#[test]
fn err_whitespace_only_eof() {
    assert_same_and_stdout("spaces only", b"   ", "0\n");
    assert_same_and_stdout("newlines only", b"\n\n", "0\n");
    assert_same_and_stdout("all ws bytes", b" \t\n\x0b\x0c\r", "0\n");
    assert_same_and_stdout("10k spaces", &vec![b' '; 10000], "0\n");
}

/// ERRORS row 4 — sign with no following digits
#[test]
fn err_sign_without_digits() {
    for inp in [&b"+"[..], b"-", b"+ ", b"- ", b"--5", b"- 5", b"+a"] {
        assert_same_and_stdout("sign without digits", inp, "0\n");
    }
}

/// ERRORS row 5 — stdin closed: read error becomes an input failure
#[test]
fn err_stdin_closed() {
    let c = run_with_closed_fd(&c_bin(), b"5", 0);
    let r = run_with_closed_fd(&rust_bin(), b"5", 0);
    assert_eq!(c, r, "\nDIVERGENCE [stdin closed]\n C-> {:?}\n R-> {:?}\n", c, r);
    assert_eq!(c.stdout, b"0\n", "expected the bad() path sentinel");
    assert_eq!(c.code, Some(0));
}

/// ERRORS row 6 — bytes that are not C-locale whitespace are not skipped
#[test]
fn err_non_locale_whitespace() {
    for b in [0x85u8, 0xa0, 0x00, 0x1c, 0x1f] {
        assert_same_and_stdout("non-locale blank", &[b, b'5'], "0\n");
    }
}

/// ERRORS row 7 — non-decimal prefixes under %d
#[test]
fn err_non_decimal_prefix() {
    assert_same_and_stdout("0x10", b"0x10", "0\n");
    assert_same_and_stdout("0b1", b"0b1", "0\n");
    assert_same_and_stdout("1e5", b"1e5", "5\n");
}

/// ERRORS row 8 — positive overflow: strtol saturates to LONG_MAX, ERANGE ignored
#[test]
fn err_overflow_positive() {
    assert_same_and_stdout("LONG_MAX+1", b"9223372036854775808", "5\n");
    assert_same_and_stdout("400 nines", &vec![b'9'; 400], "5\n");
}

/// ERRORS row 9 — negative overflow: saturates to LONG_MIN, low 32 bits are 0
#[test]
fn err_overflow_negative() {
    assert_same_and_stdout("LONG_MIN-1", b"-9223372036854775809", "0\n");
    let mut v = vec![b'-'];
    v.extend_from_slice(&vec![b'9'; 400]);
    assert_same_and_stdout("-400 nines", &v, "0\n");
}

/// ERRORS row 10 — in-range value truncating to zero flips the branch
#[test]
fn err_truncation_to_zero() {
    assert_same_and_stdout("2^32", b"4294967296", "0\n");
    assert_same_and_stdout("-2^32", b"-4294967296", "0\n");
}

/// ERRORS row 11 — the uninitialised-pointer dereference in bad() is stable.
///
/// `bad()` reads a stale stack slot left behind by `main`'s `scanf` frame and
/// dereferences it (UB). The reference -O0 build lands on a zeroed libc slot and
/// prints "0" without crashing. This pins that observable across every distinct
/// route into the x == 0 branch, since each performs a different amount of
/// `scanf` work and therefore leaves different stack/libc state behind.
#[test]
fn err_uninitialised_ptr_deref_stable() {
    let long_zeros = vec![b'0'; 1000];
    let long_ws = vec![b' '; 5000];
    let routes: Vec<&[u8]> = vec![
        b"0", b"0000", b"+0", b"-0", b"   0", b"\n\n0", b"0abc",
        b"abc", b"+", b"-", b".", b"   ", b"", b"\x00",
        b"4294967296", b"8589934592", b"-9223372036854775809",
        &long_zeros, &long_ws,
    ];
    for inp in routes {
        assert_same_and_stdout("bad() route", inp, "0\n");
        let c = run(&c_bin(), inp);
        assert_eq!(c.signal, None, "C reference crashed on {:?}", c);
    }
}

/// ERRORS row 12 — stdout unwritable. `printf`'s return value is ignored by the
/// C code, so a closed fd is silent; a readerless pipe instead raises SIGPIPE,
/// which the C does not mask and Rust must not mask either.
#[test]
fn err_stdout_closed() {
    let c = run_with_closed_fd(&c_bin(), b"0", 1);
    let r = run_with_closed_fd(&rust_bin(), b"0", 1);
    assert_eq!(c, r, "\nDIVERGENCE [stdout closed]\n C-> {:?}\n R-> {:?}\n", c, r);
}

#[test]
fn err_stdout_epipe() {
    let c = run_with_readerless_stdout(&c_bin(), b"0");
    let r = run_with_readerless_stdout(&rust_bin(), b"0");
    assert_eq!(c, r, "\nDIVERGENCE [stdout EPIPE]\n C-> {:?}\n R-> {:?}\n", c, r);
    assert_eq!(c.signal, Some(13), "expected SIGPIPE from the C reference");
}

/// Generic boundary sweep: one step past every documented range edge.
#[test]
fn boundary_one_past_range() {
    let edges: [i128; 8] = [
        i32::MAX as i128,
        i32::MIN as i128,
        u32::MAX as i128,
        1i128 << 32,
        i64::MAX as i128,
        i64::MIN as i128,
        u64::MAX as i128,
        1i128 << 64,
    ];
    for e in edges {
        for delta in -2i128..=2 {
            let v = e + delta;
            assert_same_labeled("one past range", v.to_string().as_bytes());
        }
    }
}

// ===========================================================================
// CONFIGS row 24 — randomised property sweep (fixed seed, reproducible)
// ===========================================================================

#[test]
fn prop_random_differential() {
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    const WS: &[u8] = b" \t\n\x0b\x0c\r";
    const DIGITS: &[u8] = b"0123456789";
    const SYM: &[u8] = b"+-.eExX,'_/:@[`{~\x00\x80\xff";

    let iterations = 1200;
    for i in 0..iterations {
        let mut buf: Vec<u8> = Vec::new();
        match i % 8 {
            // pure digit run
            0 => {
                let n = 1 + rng.below(24);
                for _ in 0..n {
                    buf.push(*rng.pick(DIGITS));
                }
            }
            // tiny degenerate inputs
            1 => {
                let choices: [&[u8]; 5] = [b"", b"+", b"-", b" ", b"\n"];
                buf.extend_from_slice(*rng.pick(&choices));
            }
            // ws* sign? digit*
            2 => {
                for _ in 0..rng.below(4) {
                    buf.push(*rng.pick(WS));
                }
                match rng.below(3) {
                    0 => buf.push(b'+'),
                    1 => buf.push(b'-'),
                    _ => {}
                }
                for _ in 0..rng.below(22) {
                    buf.push(*rng.pick(DIGITS));
                }
            }
            // soup of ws/digits/symbols
            3 => {
                let n = rng.below(12);
                for _ in 0..n {
                    let pool = [WS, DIGITS, SYM];
                    let p = rng.pick(&pool);
                    buf.push(*rng.pick(p));
                }
            }
            // wide-range signed integers
            4 => {
                let v = (rng.next_u64() as i128) - (1i128 << 63);
                buf.extend_from_slice(v.to_string().as_bytes());
            }
            // near powers of two (branch-flipping region)
            5 => {
                let e = *rng.pick(&[31u32, 32, 33, 63, 64, 65, 67]);
                let d = (rng.below(7) as i128) - 3;
                let sign = if rng.below(2) == 0 { -1i128 } else { 1 };
                let v = sign * ((1i128 << e) + d);
                buf.extend_from_slice(v.to_string().as_bytes());
            }
            // leading zeros then digits
            6 => {
                for _ in 0..1 + rng.below(40) {
                    buf.push(b'0');
                }
                for _ in 0..rng.below(15) {
                    buf.push(*rng.pick(DIGITS));
                }
            }
            // arbitrary bytes
            _ => {
                let n = rng.below(10);
                for _ in 0..n {
                    buf.push((rng.next_u64() & 0xff) as u8);
                }
            }
        }
        assert_same(&buf);
    }
}

/// CONFIGS row 18 — long digit runs as a property, incl. the debug-profile
/// overflow question (the u128 accumulator must not panic on huge inputs).
#[test]
fn prop_long_digit_runs() {
    let mut rng = Rng::new(0x5EED_0BAD_F00D_1234);
    for _ in 0..40 {
        let n = 50 + rng.below(3000) as usize;
        let mut buf = Vec::with_capacity(n + 1);
        if rng.below(2) == 0 {
            buf.push(if rng.below(2) == 0 { b'-' } else { b'+' });
        }
        for _ in 0..n {
            buf.push(*rng.pick(b"0123456789"));
        }
        assert_same(&buf);
    }
}
