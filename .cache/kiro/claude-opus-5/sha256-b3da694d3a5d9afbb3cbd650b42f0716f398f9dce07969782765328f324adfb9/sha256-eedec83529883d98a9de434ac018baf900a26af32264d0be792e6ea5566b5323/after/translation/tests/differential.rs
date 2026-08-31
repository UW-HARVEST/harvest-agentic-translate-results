//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links against the Rust crate as a library; both programs are
//! driven exactly the way a shell drives them.
//!
//! The C program under test is:
//!
//! ```c
//! int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
//! ```
//!
//! `driver` memcpy's the `int` into a `char[4]` and prints the four bytes as
//! lowercase `%02x`, followed by a newline. So the only thing that can differ
//! between the two programs is the `int` value that `scanf("%d", ...)` leaves
//! in `x` (or leaves untouched, at 0, on a matching/input failure) plus the
//! process exit status.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

/// Path to the Rust binary produced by Cargo for this test run.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // translation/ -> ..
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with CMake on first use if necessary.
///
/// This mirrors the documented build:
///   cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` -- is CMake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr),
        );

        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr),
        );

        assert!(
            exe.is_file(),
            "C build reported success but {} does not exist",
            exe.display()
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// Running a program the way a shell would
// ---------------------------------------------------------------------------

fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sin = child.stdin.take().expect("piped stdin");
        // A short write can legitimately fail with EPIPE if the child stops
        // reading (it only ever performs one conversion), so ignore the error
        // and just close the pipe.
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:x?}"),
    }
}

/// Assert the C and Rust programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(case: &str, stdin_bytes: &[u8]) -> Vec<u8> {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    let preview: Vec<u8> = stdin_bytes.iter().copied().take(64).collect();

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for case `{case}` (input len {}, first bytes {})\n  C   : {}\n  Rust: {}",
        stdin_bytes.len(),
        show(&preview),
        show(&c.stdout),
        show(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for case `{case}` (input len {}, first bytes {})\n  C   : {}\n  Rust: {}",
        stdin_bytes.len(),
        show(&preview),
        show(&c.stderr),
        show(&r.stderr),
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status mismatch for case `{case}` (input len {}, first bytes {}): C {:?} vs Rust {:?}",
        stdin_bytes.len(),
        show(&preview),
        c.status,
        r.status,
    );

    c.stdout
}

/// Same as [`assert_same`], and additionally pin the exact expected bytes so
/// the test still means something if both programs were to drift together.
#[track_caller]
fn assert_same_and_eq(case: &str, stdin_bytes: &[u8], expected_stdout: &str) {
    let out = assert_same(case, stdin_bytes);
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected_stdout,
        "unexpected agreed-upon stdout for case `{case}`"
    );
}

// ---------------------------------------------------------------------------
// Phase A: the binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert!(c.status.success(), "C program failed: {:?}", c.status);
    assert!(r.status.success(), "Rust program failed: {:?}", r.status);
    assert_eq!(c.stdout, b"01000000\n".to_vec());
    assert_eq!(r.stdout, c.stdout);
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF without storing anything: `x` keeps its initial 0.
#[test]
fn empty_input_leaves_x_at_zero() {
    assert_same_and_eq("empty", b"", "00000000\n");
}

/// Only whitespace: the skip loop consumes everything and hits EOF.
#[test]
fn whitespace_only_is_an_input_failure() {
    for (name, input) in [
        ("spaces", &b"   "[..]),
        ("newline", &b"\n"[..]),
        ("newlines", &b"\n\n\n"[..]),
        ("tab", &b"\t"[..]),
        ("cr", &b"\r"[..]),
        ("vt_ff", &b"\x0b\x0c"[..]),
        ("all_ws", &b" \t\n\r\x0b\x0c"[..]),
    ] {
        assert_same_and_eq(name, input, "00000000\n");
    }
}

/// A single well-formed item, with and without a trailing newline.
#[test]
fn single_item() {
    assert_same_and_eq("42", b"42", "2a000000\n");
    assert_same_and_eq("42_nl", b"42\n", "2a000000\n");
    assert_same_and_eq("zero", b"0", "00000000\n");
    assert_same_and_eq("one", b"1", "01000000\n");
    assert_same_and_eq("255", b"255", "ff000000\n");
    assert_same_and_eq("256", b"256", "00010000\n");
    assert_same_and_eq("65535", b"65535", "ffff0000\n");
    assert_same_and_eq("16777216", b"16777216", "00000001\n");
}

/// Explicit sign handling, including `-0`.
#[test]
fn signs() {
    assert_same_and_eq("plus5", b"+5", "05000000\n");
    assert_same_and_eq("minus1", b"-1", "ffffffff\n");
    assert_same_and_eq("minus_zero", b"-0", "00000000\n");
    assert_same_and_eq("plus_zero", b"+0", "00000000\n");
    assert_same_and_eq("minus7", b"-7", "f9ffffff\n");
}

/// `scanf`'s leading-whitespace skip crosses newlines (unlike `fgets`).
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    assert_same_and_eq("nl_then_7", b"\n\n\n7", "07000000\n");
    assert_same_and_eq("mixed_ws_then_7", b" \t\n\r\x0b\x0c 7", "07000000\n");
    assert_same_and_eq("ws_then_neg", b"     -7  ", "f9ffffff\n");
    assert_same_and_eq("tab_then_5", b"\t5", "05000000\n");
}

/// Only the first conversion happens; the rest of stdin is never read.
#[test]
fn only_the_first_item_is_converted() {
    assert_same_and_eq("two_ints", b"3 4", "03000000\n");
    assert_same_and_eq("two_lines", b"3\n4\n", "03000000\n");
    assert_same_and_eq("many_ints", b"12 34 56 78\n", "0c000000\n");
    assert_same_and_eq("first_then_garbage", b"12abc", "0c000000\n");
    assert_same_and_eq("first_then_dot", b"3.9", "03000000\n");
    assert_same_and_eq("comma_group", b"1,000", "01000000\n");
    assert_same_and_eq("underscore", b"1_0", "01000000\n");
    assert_same_and_eq("e_notation", b"1e5", "01000000\n");
    // %d is base 10, so conversion stops at the 'x'.
    assert_same_and_eq("hex_literal", b"0x10", "00000000\n");
}

/// Matching failure: nothing is stored, `x` stays 0.
#[test]
fn matching_failures_leave_x_at_zero() {
    for (name, input) in [
        ("alpha", &b"abc"[..]),
        ("dot_first", &b".5"[..]),
        ("sign_only_minus", &b"-"[..]),
        ("sign_only_plus", &b"+"[..]),
        ("double_minus", &b"--5"[..]),
        ("double_plus", &b"++5"[..]),
        ("plus_minus", &b"+-5"[..]),
        ("minus_plus", &b"-+5"[..]),
        ("space_after_sign", &b"- 5"[..]),
        ("space_after_plus", &b"+ 5"[..]),
        ("sign_then_nl", &b"-\n5"[..]),
        ("punct", &b"!?#"[..]),
        ("nul_first", &b"\x00\x005"[..]),
        ("nul_only", &b"\x00"[..]),
        ("high_bytes", &b"\xff\xfe5"[..]),
        ("ws_then_alpha", &b"   xyz"[..]),
    ] {
        assert_same_and_eq(name, input, "00000000\n");
    }
}

/// The int boundaries, and the truncation of `strtol`'s `long` result into an
/// `int` that glibc performs for `%d`.
#[test]
fn int_boundaries_and_truncation() {
    assert_same_and_eq("int_max", b"2147483647", "ffffff7f\n");
    assert_same_and_eq("int_min", b"-2147483648", "00000080\n");
    // Fits in a long, then truncated to int: wraps to INT_MIN.
    assert_same_and_eq("int_max_plus_1", b"2147483648", "00000080\n");
    // ... and to INT_MAX.
    assert_same_and_eq("int_min_minus_1", b"-2147483649", "ffffff7f\n");
    // 2^32 truncates to 0.
    assert_same_and_eq("two_pow_32", b"4294967296", "00000000\n");
    assert_same_and_eq("two_pow_32_plus_1", b"4294967297", "01000000\n");
    assert_same_and_eq("neg_two_pow_32", b"-4294967296", "00000000\n");
}

/// glibc converts `%d` with `strtol`, which *saturates* at LONG_MAX / LONG_MIN
/// on overflow; the saturated `long` is then truncated to `int`. So a huge
/// positive number becomes -1 and a huge negative one becomes 0.
#[test]
fn long_overflow_saturates_then_truncates() {
    assert_same_and_eq("long_max", b"9223372036854775807", "ffffffff\n");
    assert_same_and_eq("long_max_plus_1", b"9223372036854775808", "ffffffff\n");
    assert_same_and_eq("long_min", b"-9223372036854775808", "00000000\n");
    assert_same_and_eq("long_min_minus_1", b"-9223372036854775809", "00000000\n");
    assert_same_and_eq("twenty_nines", b"99999999999999999999", "ffffffff\n");
    assert_same_and_eq("twenty_nines_neg", b"-99999999999999999999", "00000000\n");
    assert_same_and_eq("two_pow_64", b"18446744073709551616", "ffffffff\n");
    assert_same_and_eq("ten_pow_30", b"1000000000000000000000000000000", "ffffffff\n");
}

/// Leading zeros are not an overflow, however many there are.
#[test]
fn leading_zeros() {
    let mut input = vec![b'0'; 30];
    input.extend_from_slice(b"42");
    assert_same_and_eq("leading_zeros_42", &input, "2a000000\n");

    let mut neg = vec![b'-'];
    neg.extend(std::iter::repeat(b'0').take(30));
    neg.push(b'1');
    assert_same_and_eq("leading_zeros_neg1", &neg, "ffffffff\n");

    let zeros = vec![b'0'; 100_000];
    assert_same_and_eq("100k_zeros", &zeros, "00000000\n");
}

// ---------------------------------------------------------------------------
// Phase C: the paths nothing above reaches
// ---------------------------------------------------------------------------

/// Digit runs long enough to cross the Rust reader's internal 4 KiB buffer and
/// glibc's own `scanf` work buffer.
#[test]
fn very_long_inputs_and_buffer_boundaries() {
    let nines = vec![b'9'; 100_000];
    assert_same_and_eq("100k_nines", &nines, "ffffffff\n");

    let mut neg_nines = vec![b'-'];
    neg_nines.extend(std::iter::repeat(b'9').take(100_000));
    assert_same_and_eq("100k_nines_neg", &neg_nines, "00000000\n");

    // Whitespace runs that straddle the 4096-byte read boundary, so that the
    // digits land at, just before and just after the refill point.
    for pad in [4094usize, 4095, 4096, 4097, 8191, 8192, 8193] {
        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"123456");
        assert_same_and_eq(&format!("ws_pad_{pad}"), &input, "40e20100\n");
    }

    // A sign and digits split across the boundary.
    for pad in [4095usize, 4096] {
        let mut input = vec![b'\n'; pad];
        input.extend_from_slice(b"-2");
        assert_same_and_eq(&format!("nl_pad_{pad}_neg2"), &input, "feffffff\n");
    }

    let mut big_ws = vec![b' '; 10_000];
    big_ws.extend_from_slice(b"-13");
    assert_same_and_eq("10k_spaces_neg13", &big_ws, "f3ffffff\n");
}

/// Arbitrary binary stdin, including every byte value, must not diverge.
#[test]
fn arbitrary_binary_input() {
    let all: Vec<u8> = (0u8..=255).collect();
    assert_same("all_256_bytes", &all);

    // Every single byte on its own: exercises "is this byte whitespace, a
    // sign, a digit, or a matching failure?" for the whole domain.
    for b in 0u8..=255 {
        assert_same(&format!("single_byte_{b:#04x}"), &[b]);
    }

    // Every byte value directly after a digit (does the conversion stop
    // there?) and directly after a sign (matching failure or not?).
    for b in 0u8..=255 {
        assert_same(&format!("digit_then_{b:#04x}"), &[b'7', b]);
        assert_same(&format!("sign_then_{b:#04x}"), &[b'-', b]);
    }
}

/// Every value of the low byte, plus a spread of bit patterns, to pin the
/// `%02x` formatting (zero padding, lowercase, no separators, one trailing
/// newline) across the whole hex alphabet.
#[test]
fn hex_formatting_covers_every_nibble() {
    for v in [
        0i32, 1, 9, 10, 15, 16, 127, 128, 171, 205, 239, 255, 256, 4095, 4096, 0x0f0f_0f0f,
        0x1234_5678, 0x7fff_ffff, -1, -2, -16, -256, -65536, -16_777_216, i32::MIN, i32::MAX,
        0x00ab_cdef, 0x0000_00a0,
    ] {
        let expected = {
            let bytes = v.to_ne_bytes();
            let mut s = String::new();
            for b in bytes {
                s.push_str(&format!("{b:02x}"));
            }
            s.push('\n');
            s
        };
        assert_same_and_eq(&format!("value_{v}"), v.to_string().as_bytes(), &expected);
    }
}

/// Command-line arguments are ignored by both programs (`main` takes none).
#[test]
fn argv_is_ignored() {
    for args in [vec!["foo"], vec!["-1"], vec!["--help"], vec!["a", "b", "c"]] {
        let mut c = Command::new(c_bin());
        let mut r = Command::new(rust_bin());
        let co = spawn_with_args(&mut c, &args, b"5\n");
        let ro = spawn_with_args(&mut r, &args, b"5\n");
        assert_eq!(co.stdout, ro.stdout, "stdout differs with args {args:?}");
        assert_eq!(co.stderr, ro.stderr, "stderr differs with args {args:?}");
        assert_eq!(co.status, ro.status, "status differs with args {args:?}");
    }
}

fn spawn_with_args(cmd: &mut Command, args: &[&str], stdin_bytes: &[u8]) -> Output {
    let mut child = cmd
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut sin = child.stdin.take().expect("piped stdin");
        let _ = sin.write_all(stdin_bytes);
    }
    child.wait_with_output().expect("wait")
}

/// stdin closed outright (not just empty): `/dev/null` and an immediately
/// closed pipe are both input failures.
#[test]
fn stdin_closed_immediately() {
    let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(devnull))
        .output()
        .expect("run C");
    let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(devnull))
        .output()
        .expect("run Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status, r.status);
    assert_eq!(c.stdout, b"00000000\n".to_vec());
}

/// stdin is a regular file rather than a pipe (different buffering path).
#[test]
fn stdin_from_a_regular_file() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("driver_stdin_{}.txt", std::process::id()));
    std::fs::write(&path, b"  -12345\nignored\n").expect("write temp stdin file");

    let mut outs = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let f = std::fs::File::open(&path).expect("open temp stdin file");
        outs.push(
            Command::new(prog)
                .stdin(Stdio::from(f))
                .output()
                .expect("run"),
        );
    }
    let _ = std::fs::remove_file(&path);

    assert_eq!(outs[0].stdout, outs[1].stdout);
    assert_eq!(outs[0].stderr, outs[1].stderr);
    assert_eq!(outs[0].status, outs[1].status);
    assert_eq!(outs[0].stdout, b"c7cfffff\n".to_vec());
}

/// stdout is a pipe with no reader. The C program is killed by SIGPIPE; the
/// Rust program must be too (the Rust runtime ignores SIGPIPE by default, so
/// `main` restores the default disposition).
///
/// The child blocks reading stdin until this test writes to it, so closing the
/// read end of its stdout first is deterministic rather than racy.
#[test]
#[cfg(unix)]
fn stdout_pipe_with_no_reader() {
    use std::os::unix::process::ExitStatusExt;

    let mut statuses = Vec::new();
    for prog in [c_bin(), rust_bin()] {
        let mut child = Command::new(prog)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");

        // Close the read end of the child's stdout before it can write.
        drop(child.stdout.take());

        {
            let mut sin = child.stdin.take().expect("piped stdin");
            let _ = sin.write_all(b"5\n");
        }

        let status = child.wait().expect("wait");
        statuses.push(status);
    }

    assert_eq!(
        (statuses[0].code(), statuses[0].signal()),
        (statuses[1].code(), statuses[1].signal()),
        "C {:?} vs Rust {:?} when stdout has no reader",
        statuses[0],
        statuses[1],
    );
}

/// A deterministic sweep of numeric strings around every interesting decimal
/// magnitude and boundary, driven through both programs.
#[test]
fn numeric_sweep() {
    let mut cases: Vec<String> = Vec::new();

    for e in 0..=22u32 {
        let base = 10u128.pow(e);
        for delta in [-1i128, 0, 1] {
            let v = base as i128 + delta;
            cases.push(v.to_string());
            cases.push(format!("-{v}"));
            cases.push(format!("+{v}"));
        }
    }
    for b in [
        i32::MAX as i128,
        i32::MIN as i128,
        u32::MAX as i128,
        1i128 << 31,
        1i128 << 32,
        1i128 << 63,
        1i128 << 64,
        i64::MAX as i128,
        i64::MIN as i128,
    ] {
        for delta in [-2i128, -1, 0, 1, 2] {
            cases.push((b + delta).to_string());
        }
    }
    for n in -300i32..=300 {
        cases.push(n.to_string());
    }

    for c in &cases {
        assert_same(c, c.as_bytes());
        assert_same(&format!("{c}\\n"), format!("{c}\n").as_bytes());
        assert_same(&format!("ws {c}"), format!("  \n\t{c} rest\n").as_bytes());
    }
}

/// Pseudo-random differential fuzzing over structured and unstructured stdin.
/// Deterministic (fixed seed) so a failure is always reproducible.
#[test]
fn randomized_differential_fuzz() {
    let mut rng = Lcg::new(0x2545_F491_4F6C_DD1D);

    for i in 0..1500 {
        let input = gen_input(&mut rng);
        assert_same(&format!("fuzz_{i}"), &input);
    }
}

/// A small xorshift* PRNG: no dev-dependencies needed, and fully reproducible.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
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

fn gen_input(rng: &mut Lcg) -> Vec<u8> {
    const WS_AND_NUMERIC: &[u8] = b" \t\n\r\x0b\x0c+-0123456789.,eExX";
    let kind = rng.below(9);
    match kind {
        // Random bytes from the whole domain.
        0 => (0..rng.below(24)).map(|_| rng.next_u64() as u8).collect(),
        // Random bytes drawn from the alphabet scanf actually branches on.
        1 => (0..rng.below(16))
            .map(|_| *rng.pick(WS_AND_NUMERIC))
            .collect(),
        // A random i32, formatted.
        2 => (rng.next_u64() as i32).to_string().into_bytes(),
        // A random i64, formatted (exercises truncation).
        3 => (rng.next_u64() as i64).to_string().into_bytes(),
        // A random digit string of random length (exercises saturation).
        4 => {
            let len = 1 + rng.below(40) as usize;
            let mut v: Vec<u8> = (0..len).map(|_| b'0' + rng.below(10) as u8).collect();
            if rng.below(2) == 0 {
                v.insert(0, if rng.below(2) == 0 { b'-' } else { b'+' });
            }
            v
        }
        // Leading zeros then a value.
        5 => {
            let mut v = vec![b'0'; rng.below(40) as usize];
            v.extend((rng.next_u64() as u32).to_string().into_bytes());
            v
        }
        // Leading whitespace then a value then trailing junk.
        6 => {
            let mut v: Vec<u8> = (0..rng.below(10))
                .map(|_| *rng.pick(b" \t\n\r\x0b\x0c"))
                .collect();
            v.extend((rng.next_u64() as i32).to_string().into_bytes());
            v.extend((0..rng.below(8)).map(|_| rng.next_u64() as u8));
            v
        }
        // Sign soup.
        7 => {
            let mut v: Vec<u8> = (0..1 + rng.below(4))
                .map(|_| *rng.pick(b"+- \t"))
                .collect();
            v.extend((rng.next_u64() as u16).to_string().into_bytes());
            v
        }
        // Long whitespace padding near the reader's refill boundary.
        _ => {
            let pad = 4090 + rng.below(12) as usize;
            let mut v = vec![*rng.pick(b" \n\t"); pad];
            v.extend((rng.next_u64() as i32).to_string().into_bytes());
            v
        }
    }
}
