//! Differential tests: run the C driver and the Rust driver as *subprocesses*
//! on the same stdin and require identical stdout, stderr and wait status.
//!
//! The C program is the ground truth. Nothing here links against the Rust
//! crate as a library — the binary is driven exactly the way a shell drives it.
//!
//! # What the C program branches on
//!
//! `main` performs a single
//! `scanf("%d%f%f%f%d%d%d%d%f%f%f%d", ...)` and then switches on `which`:
//!
//! * `scanf` stops at the first matching failure or at end of input, leaving
//!   every later variable at its zero initialiser, so "how many fields were
//!   supplied" and "which field was malformed" are 13 distinct input classes.
//! * `which` selects one of six noise functions, `default` returns `NAN`.
//! * `stb_perlin_noise3_internal` masks its coordinates with
//!   `(wrap - 1) & 255`, so `wrap` values of 0, 1, a power of two, a non-power
//!   of two and a negative number take different paths.
//! * `stb_perlin_{ridge,fbm,turbulence}_noise3` loop `octaves` times, so
//!   `octaves <= 0` skips the loop entirely.
//! * `stb_perlin_noise3_wrap_nonpow2` uses `%` and unchecked table indexing;
//!   `wrap == 0` becomes 256, a negative remainder is corrected by
//!   `+= wrap`, and out-of-range indices read past the tables — far enough and
//!   the process dies of `SIGSEGV`, while `INT_MIN % -1` raises `SIGFPE`.
//! * `printf("%.9g\n", res)` must reproduce glibc's formatting, NaN sign
//!   included.
//!
//! # Inputs deliberately not asserted on
//!
//! A few `which == 5` inputs make the C program read the four pointer slots
//! the dynamic linker fills in (`DT_DEBUG`, the `__libc_start_main` GOT entry,
//! `.got.plt[1..2]` and the resolved `__isoc99_scanf` slot). Those bytes carry
//! ASLR entropy, so the C program prints a *different answer on every run* and
//! sometimes crashes instead. No translation can match a program that does not
//! agree with itself; see `ERRORS.md`. Everything else is asserted.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating and running the two programs
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path of the built C driver, building it with cmake on first use.
///
/// Tests run in parallel inside one binary, so the build is done exactly once
/// behind a `OnceLock` — concurrent `cmake` invocations in the same build
/// directory race and corrupt each other's cache.
fn c_driver() -> &'static Path {
    static C_DRIVER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    C_DRIVER.get_or_init(build_c_driver).as_path()
}

fn build_c_driver() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");
    let exe = build.join("driver");
    if exe.exists() {
        return exe;
    }
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is it installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}",
        String::from_utf8_lossy(&cfg.stderr)
    );
    let out = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        out.status.success(),
        "cmake --build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(exe.exists(), "cmake did not produce {}", exe.display());
    exe
}

fn rust_driver() -> &'static Path {
    static RUST_DRIVER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    RUST_DRIVER
        .get_or_init(|| PathBuf::from(env!("CARGO_BIN_EXE_driver")))
        .as_path()
}

/// Everything about a run that the C program can be compared on.
#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some` when the process exited normally.
    code: Option<i32>,
    /// `Some` when the process was killed by a signal.
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

/// `prctl(PR_SET_DUMPABLE, 0)` in the child.
///
/// Several inputs make both programs die of `SIGSEGV`. This host pipes core
/// dumps to `systemd-coredump`, which costs about a second per crash; turning
/// the child undumpable skips that. It is applied to *both* programs, and the
/// comparison ignores the "core dumped" bit, so it cannot mask a difference.
#[cfg(all(unix, target_arch = "x86_64"))]
unsafe fn suppress_core_dump() -> std::io::Result<()> {
    const SYS_PRCTL: i64 = 157;
    const PR_SET_DUMPABLE: i64 = 4;
    let mut _ret: i64;
    std::arch::asm!(
        "syscall",
        inlateout("rax") SYS_PRCTL => _ret,
        in("rdi") PR_SET_DUMPABLE,
        in("rsi") 0i64,
        in("rdx") 0i64,
        in("r10") 0i64,
        in("r8") 0i64,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
    );
    Ok(())
}

#[cfg(not(all(unix, target_arch = "x86_64")))]
unsafe fn suppress_core_dump() -> std::io::Result<()> {
    Ok(())
}

fn run(program: &Path, input: &str) -> Outcome {
    use std::os::unix::process::{CommandExt, ExitStatusExt};

    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| suppress_core_dump());
    }
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes())
        // The C program can die before draining stdin; a broken pipe is not a
        // test failure.
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert that both programs behave identically on `input`.
#[track_caller]
fn same(input: &str) {
    let c = run(c_driver(), input);
    let r = run(rust_driver(), input);
    assert_eq!(
        c, r,
        "\ninput  : {input:?}\nC      : {c:?}\nRust   : {r:?}\n"
    );
}

#[track_caller]
fn same_all(inputs: &[&str]) {
    for i in inputs {
        same(i);
    }
}

// ---------------------------------------------------------------------------
// Phase A — both programs exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_programs_build_and_run() {
    let c = run(c_driver(), "0 0.5 0.5 0.5 0 0 0 0 0 0 0 0\n");
    let r = run(rust_driver(), "0 0.5 0.5 0.5 0 0 0 0 0 0 0 0\n");
    assert_eq!(c.code, Some(0), "C driver did not exit 0: {c:?}");
    assert_eq!(c.stdout, b"-0.5\n", "unexpected C baseline output: {c:?}");
    assert_eq!(c, r);
}

// ---------------------------------------------------------------------------
// Phase B — the `which` switch, including the `default: return NAN` arm
// ---------------------------------------------------------------------------

#[test]
fn which_selects_each_noise_function() {
    same_all(&[
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 42 0 0 0 0",
        "2 0.5 0.25 0.125 0 0 0 0 2.0 0.5 1.0 6",
        "3 0.5 0.25 0.125 0 0 0 0 2.0 0.5 1.0 6",
        "4 0.5 0.25 0.125 0 0 0 0 2.0 0.5 1.0 6",
        "5 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn which_out_of_range_returns_nan() {
    same_all(&[
        "6 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "7 0 0 0 0 0 0 0 0 0 0 0",
        "-1 0 0 0 0 0 0 0 0 0 0 0",
        "2147483647 0 0 0 0 0 0 0 0 0 0 0",
        "-2147483648 0 0 0 0 0 0 0 0 0 0 0",
    ]);
}

// ---------------------------------------------------------------------------
// Phase B — scanf: how far the single call gets
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input() {
    // `scanf` returns EOF and leaves all twelve variables at zero, so `which`
    // stays 0 and `stb_perlin_noise3(0,0,0,0,0,0)` is printed.
    same_all(&["", " ", "   ", "\n", "\n\n\n", "\t\t", " \t\r\n\x0b\x0c"]);
}

#[test]
fn input_truncated_after_each_field() {
    // One case per prefix length: every later argument keeps its initialiser.
    same_all(&[
        "0",
        "0 1",
        "0 1 2",
        "0 1 2 3",
        "0 1 2 3 4",
        "0 1 2 3 4 5",
        "0 1 2 3 4 5 6",
        "0 1 2 3 4 5 6 7",
        "2 1 2 3 4 5 6 7 8",
        "2 1 2 3 4 5 6 7 8 9",
        "2 1 2 3 4 5 6 7 8 9 10",
        "2 1 2 3 4 5 6 7 8 9 10 11",
    ]);
}

#[test]
fn matching_failure_at_the_first_field() {
    same_all(&["x", "+", "-", "abc", ".", "e", "+x", "-.", "0x"]);
}

#[test]
fn matching_failure_mid_input() {
    same_all(&[
        "0 x",
        "0 +",
        "0 -",
        "0 .",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1",
        "0 0.5 0.25 x 0 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 x 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 x",
    ]);
}

#[test]
fn scanf_reads_across_newlines_and_arbitrary_whitespace() {
    // `%d`/`%f` skip leading whitespace of any kind, so a single scanf call
    // consumes fields spread over many lines.
    same_all(&[
        "2\n0.5\n0.25\n0.125\n0\n0\n0\n0\n2\n0.5\n1\n4",
        "\t\t 3 \n 0.5\t0.25 \n\n 0.125 0 0 0 0 2 0.5 1 5\n",
        "  0\r\n0.5\r\n0.25\r\n0.125\r\n0 0 0 0 0 0 0 0",
        "0\x0b0.5\x0c0.25 0.125 0 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn trailing_input_is_ignored() {
    same_all(&[
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0 trailing junk",
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0\n\n\n",
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0 13 14 15",
    ]);
}

#[test]
fn scanf_float_spellings() {
    same_all(&[
        "0 5. .5 . 0 0 0 0 0 0 0 0",
        "0 1.2.3 4 5 0 0 0 0 0 0 0 0",
        "0 1e 2 3 0 0 0 0 0 0 0 0",
        "0 1e+ 2 3 0 0 0 0 0 0 0 0",
        "0 1e5 2 3 0 0 0 0 0 0 0 0",
        "0 1E-5 2 3 0 0 0 0 0 0 0 0",
        "0 0x 2 3 0 0 0 0 0 0 0 0",
        "0 0xg 2 3 0 0 0 0 0 0 0 0",
        "0 0x1p 2 3 0 0 0 0 0 0 0 0",
        "0 0x1p4 2 3 0 0 0 0 0 0 0 0",
        "0 0X1.8P-3 2 3 0 0 0 0 0 0 0 0",
        "0 0x.8p1 2 3 0 0 0 0 0 0 0 0",
        "0 +0.5 -0.25 +.125 0 0 0 0 0 0 0 0",
        "0 000000.5 0.25 0.125 0 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn scanf_infinity_and_nan_spellings() {
    same_all(&[
        "0 na 2 3 0 0 0 0 0 0 0 0",
        "0 nan 2 3 0 0 0 0 0 0 0 0",
        "0 -nan 2 3 0 0 0 0 0 0 0 0",
        "0 NAN 2 3 0 0 0 0 0 0 0 0",
        "0 nan(1) 2 3 0 0 0 0 0 0 0 0",
        "0 i 2 3 0 0 0 0 0 0 0 0",
        "0 infi 2 3 0 0 0 0 0 0 0 0",
        "0 infinit 2 3 0 0 0 0 0 0 0 0",
        "0 inf 2 3 0 0 0 0 0 0 0 0",
        "0 -inf 2 3 0 0 0 0 0 0 0 0",
        "0 INFINITY 2 3 0 0 0 0 0 0 0 0",
        "0 infinity 2 3 0 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn scanf_integer_overflow_and_forms() {
    // glibc's `%d` is `(int) strtol(...)`, so an out-of-range literal
    // saturates to LONG_MAX/LONG_MIN and is then truncated to `int`.
    same_all(&[
        "12345678901234567890 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "-12345678901234567890 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 2147483648 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 -2147483649 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 4294967296 0 0 0 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 0x10 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 007 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 +42 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 -0 0 0 0 0",
    ]);
}

// ---------------------------------------------------------------------------
// Phase B — stb_perlin_noise3 / _seed: the `& 255` wrap masks
// ---------------------------------------------------------------------------

#[test]
fn noise3_wrap_masks() {
    same_all(&[
        "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "0 0.5 0.25 0.125 1 1 1 0 0 0 0 0",
        "0 0.5 0.25 0.125 2 4 8 0 0 0 0 0",
        "0 0.5 0.25 0.125 3 5 7 0 0 0 0 0",
        "0 0.5 0.25 0.125 256 256 256 0 0 0 0 0",
        "0 0.5 0.25 0.125 65536 65536 65536 0 0 0 0 0",
        "0 0.5 0.25 0.125 -1 -1 -1 0 0 0 0 0",
        "0 0.5 0.25 0.125 2147483647 -2147483648 1 0 0 0 0 0",
    ]);
}

#[test]
fn noise3_seed_is_truncated_to_unsigned_char() {
    same_all(&[
        "1 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 -1 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 255 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 256 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 511 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 -256 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 2147483647 0 0 0 0",
        "1 0.5 0.25 0.125 0 0 0 -2147483648 0 0 0 0",
    ]);
}

#[test]
fn fastfloor_edges() {
    // `(int) a` is `cvttss2si`: NaN and anything out of `int` range give
    // INT_MIN, and the `a < ai` correction then wraps to INT_MAX.
    same_all(&[
        "0 0 0 0 0 0 0 0 0 0 0 0",
        "0 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0",
        "0 1 -1 0.5 0 0 0 0 0 0 0 0",
        "0 -0.5 -0.25 -0.125 16 16 16 0 0 0 0 0",
        "0 255.5 255.25 255.125 256 256 256 0 0 0 0 0",
        "0 -256.5 -256.25 -256.125 256 256 256 0 0 0 0 0",
        "0 2147483648 2147483648 2147483648 0 0 0 0 0 0 0 0",
        "0 -2147483648 -2147483648 -2147483648 0 0 0 0 0 0 0 0",
        "0 2147483647 -2147483649 1 0 0 0 0 0 0 0 0",
        "0 1e30 1e30 1e30 0 0 0 0 0 0 0 0",
        "0 -1e30 -1e30 -1e30 0 0 0 0 0 0 0 0",
        "0 inf inf inf 0 0 0 0 0 0 0 0",
        "0 -inf -inf -inf 0 0 0 0 0 0 0 0",
        "0 nan nan nan 0 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn nan_sign_propagation() {
    // The sign of a propagated NaN is visible through `printf("%.9g")`, and
    // which operand's NaN survives depends on the SSE operand order gcc picks.
    same_all(&[
        "0 -nan 0.5 0.25 0 0 0 0 0 0 0 0",
        "0 0.5 -nan 0.25 0 0 0 0 0 0 0 0",
        "0 0.5 0.25 -nan 0 0 0 0 0 0 0 0",
        "0 nan -nan nan 0 0 0 0 0 0 0 0",
        "0 -nan -nan -nan 0 0 0 0 0 0 0 0",
        "0 inf 0.5 0.25 0 0 0 0 0 0 0 0",
        "0 -inf 0.5 0.25 0 0 0 0 0 0 0 0",
        "1 -nan 128 nan 2147483647 227 -42 447246631 0 0 0 0",
        "2 0.5 0.25 0.125 0 0 0 0 nan nan nan 5",
        "2 0.5 0.25 0.125 0 0 0 0 -nan 0.5 1 4",
        "3 0.5 0.25 0.125 0 0 0 0 nan 0.5 0 4",
        "3 0.5 0.25 0.125 0 0 0 0 -nan -nan 0 4",
        "4 0.5 0.25 0.125 0 0 0 0 -nan 0.5 0 4",
        "4 0.5 0.25 0.125 0 0 0 0 nan nan 0 4",
        // `(float) fabs(r)` is compiled to a sign-bit clear, so a NaN comes
        // back positive.
        "4 nan 0.25 0.125 0 0 0 0 2 0.5 0 1",
        "2 nan 0.25 0.125 0 0 0 0 2 0.5 1 1",
    ]);
}

// ---------------------------------------------------------------------------
// Phase B — the three fractal loops
// ---------------------------------------------------------------------------

#[test]
fn ridge_octave_counts() {
    same_all(&[
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 0",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 -1",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 -2147483648",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 1",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 2",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 6",
        // `(unsigned char) i` wraps, so octave 256 reuses seed 0.
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 256",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 257",
        "2 0.5 0.25 0.125 0 0 0 0 0 0 0 6",
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 -1 6",
        "2 0.5 0.25 0.125 0 0 0 0 1e20 1e20 1e20 8",
        "2 0.5 0.25 0.125 0 0 0 0 inf 0.5 1 4",
    ]);
}

#[test]
fn fbm_octave_counts() {
    same_all(&[
        "3 0.5 0.25 0.125 0 0 0 0 2 0.5 0 0",
        "3 0.5 0.25 0.125 0 0 0 0 2 0.5 0 -5",
        "3 0.5 0.25 0.125 0 0 0 0 2 0.5 0 1",
        "3 0.5 0.25 0.125 0 0 0 0 2 0.5 0 6",
        "3 0.5 0.25 0.125 0 0 0 0 2 0.5 0 300",
        "3 0.5 0.25 0.125 0 0 0 0 2 2 0 40",
        "3 0.5 0.25 0.125 0 0 0 0 inf inf 0 4",
        "3 0.5 0.25 0.125 0 0 0 0 -2 -0.5 0 6",
    ]);
}

#[test]
fn turbulence_octave_counts() {
    same_all(&[
        "4 0.5 0.25 0.125 0 0 0 0 2 0.5 0 0",
        "4 0.5 0.25 0.125 0 0 0 0 2 0.5 0 -1",
        "4 0.5 0.25 0.125 0 0 0 0 2 0.5 0 1",
        "4 0.5 0.25 0.125 0 0 0 0 2 0.5 0 6",
        "4 0.5 0.25 0.125 0 0 0 0 2 0.5 0 256",
        "4 0.5 0.25 0.125 0 0 0 0 2 2 0 40",
        "4 0.5 0.25 0.125 0 0 0 0 -2 -0.5 0 6",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — stb_perlin_noise3_wrap_nonpow2
// ---------------------------------------------------------------------------

#[test]
fn nonpow2_wrap_defaults_and_small_moduli() {
    same_all(&[
        // wrap == 0 becomes 256
        "5 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
        "5 0.5 0.25 0.125 1 1 1 0 0 0 0 0",
        "5 0.5 0.25 0.125 2 3 4 0 0 0 0 0",
        "5 0.5 0.25 0.125 3 5 7 0 0 0 0 0",
        "5 0.5 0.25 0.125 255 255 255 0 0 0 0 0",
        "5 0.5 0.25 0.125 256 256 256 0 0 0 0 0",
        "5 0.5 0.25 0.125 257 257 257 0 0 0 0 0",
        "5 0.5 0.25 0.125 0 0 0 255 0 0 0 0",
        "5 0.5 0.25 0.125 0 0 0 256 0 0 0 0",
        "5 0.5 0.25 0.125 0 0 0 -1 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_negative_remainder_correction() {
    // `x0 = px % w; if (x0 < 0) x0 += w;` — reached only for negative
    // coordinates, and with a negative `w` the "correction" makes it *more*
    // negative, which is what drives the reads below the tables.
    same_all(&[
        "5 -0.5 -0.25 -0.125 0 0 0 0 0 0 0 0",
        "5 -0.5 -0.25 -0.125 3 5 7 200 0 0 0 0",
        "5 -1.5 -2.5 -3.5 256 256 256 0 0 0 0 0",
        "5 -1 -1 -1 -2 -3 -4 0 0 0 0 0",
        "5 0.5 0.25 0.125 -3 -5 -7 0 0 0 0 0",
        "5 -300.5 -1.25 -1.125 -256 1 1 0 0 0 0 0",
        "5 -1 0.25 0.125 -300 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_reads_past_the_tables() {
    // Indices well outside `stb__perlin_randtab[512]` that still land on
    // mapped, load-time-constant bytes.
    same_all(&[
        "5 900.5 900.25 900.125 1000 1000 1000 0 0 0 0 0",
        "5 0.5 0.25 0.125 1000 1000 1000 0 0 0 0 0",
        "5 0.5 0.25 0.125 2000000000 2000000000 2000000000 0 0 0 0 0",
        "5 600 0.25 0.125 601 0 0 0 0 0 0 0",
        "5 1500 0.25 0.125 1501 0 0 0 0 0 0 0",
        "5 3000 0.25 0.125 3001 0 0 0 0 0 0 0",
        "5 4030 0.25 0.125 4031 0 0 0 0 0 0 0",
        "5 -1 0.25 0.125 -1000 0 0 0 0 0 0 0",
        "5 -1 0.25 0.125 -5000 0 0 0 0 0 0 0",
        "5 -1 0.25 0.125 -20543 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_index_at_the_edges_of_the_mapping() {
    // randtab index 4031 is the last readable byte and -20544 the first;
    // one step further and the C process is killed by SIGSEGV.
    same_all(&[
        "5 4031 0.25 0.125 4032 0 0 0 0 0 0 0",
        "5 -1 0.25 0.125 -20544 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_unmapped_index_dies_of_sigsegv() {
    // Kept short: every case here costs a real crash in both programs.
    same_all(&[
        "5 4032 0.25 0.125 4033 0 0 0 0 0 0 0",
        "5 100000 0.25 0.125 1000000 0 0 0 0 0 0 0",
        "5 -1 0.25 0.125 -30000 0 0 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_int_min_modulo_minus_one_dies_of_sigfpe() {
    // `idiv` raises #DE because the *quotient* -(INT_MIN) is unrepresentable,
    // even though the remainder would be 0.
    same_all(&[
        "5 -2147483648 0.5 0.5 -1 1 1 0 0 0 0 0",
        "5 0.5 -2147483648 0.5 1 -1 1 0 0 0 0 0",
        "5 0.5 0.5 -2147483648 1 1 -1 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_int_min_modulo_other_divisors_is_fine() {
    same_all(&[
        "5 -2147483648 0.5 0.5 -2 1 1 0 0 0 0 0",
        "5 -2147483648 0.5 0.5 1 1 1 0 0 0 0 0",
        "5 2147483647 0.5 0.5 -1 1 1 0 0 0 0 0",
        "5 -1e30 0.25 0.125 -1 1 1 0 0 0 0 0",
    ]);
}

#[test]
fn nonpow2_non_finite_coordinates() {
    same_all(&[
        "5 nan 0.25 0.125 0 0 0 0 0 0 0 0",
        "5 inf 0.25 0.125 0 0 0 0 0 0 0 0",
        "5 -inf 0.25 0.125 0 0 0 0 0 0 0 0",
        "5 0.25 nan 0.125 0 0 0 0 0 0 0 0",
        "5 0.25 0.125 -nan 0 0 0 0 0 0 0 0",
        "5 inf -inf nan 3 5 7 9 0 0 0 0",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — printf("%.9g")
// ---------------------------------------------------------------------------

#[test]
fn printf_g_output_shapes() {
    // Exercise `%e` style, `%f` style, trailing-zero removal, the two-digit
    // exponent field, subnormals, zero, infinities and NaN signs — all through
    // results the noise functions actually produce.
    same_all(&[
        // exactly representable small values and plain zero
        "2 0.5 0.25 0.125 0 0 0 0 2 0.5 1 0",
        "0 0.5 0.5 0.5 0 0 0 0 0 0 0 0",
        // tiny magnitudes -> exponent form with a negative exponent
        "3 0.5 0.25 0.125 0 0 0 0 2 1e-20 0 3",
        "3 0.5 0.25 0.125 0 0 0 0 2 1e-30 0 4",
        "2 0.5 0.25 0.125 0 0 0 0 2 1e-30 1e-20 3",
        // subnormal results
        "3 0.5 0.25 0.125 0 0 0 0 2 1e-38 0 4",
        "3 0.5 0.25 0.125 0 0 0 0 2 1e-44 0 3",
        // huge magnitudes -> exponent form, then inf
        "3 0.5 0.25 0.125 0 0 0 0 2 1e20 0 4",
        "2 0.5 0.25 0.125 0 0 0 0 2 1e30 1e30 4",
        "3 0.5 0.25 0.125 0 0 0 0 2 1e38 0 6",
        // nine significant digits and rounding
        "1 0.3 0.7 0.9 0 0 0 17 0 0 0 0",
        "1 12.3456 78.9012 34.5678 0 0 0 200 0 0 0 0",
        "4 0.123456 0.654321 0.111111 0 0 0 0 1.97 0.53 0 7",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C — a broad sweep, to catch anything the named cases miss
// ---------------------------------------------------------------------------

#[test]
fn deterministic_sweep_over_many_inputs() {
    // A fixed, deterministic pseudo-random corpus.
    //
    // For `which == 5` the wrap values are restricted to `0..=512`. With a
    // non-negative modulus every `x0`/`y0`/`z0` ends up in `0..wrap`, so every
    // table index stays non-negative, and the negative indices — the only ones
    // that can reach the ASLR-dependent pointer slots described at the top of
    // this file — are unreachable. Negative wraps are covered by the curated
    // cases above, each of which was checked to be stable across many runs.
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |m: u64| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s % m
    };
    let floats = [
        "0", "-0", "0.5", "-0.5", "1", "-1", "0.125", "63.75", "-63.75", "255.5", "-255.5",
        "1e-20", "1e20", "3.4028235e38", "1e-45", "inf", "-inf", "nan", "-nan", "0x1p4",
        "0x1.8p-3", "123.456", "-987.654", "1e-6",
    ];
    let ints = [
        "0", "1", "2", "3", "4", "7", "8", "16", "31", "32", "64", "127", "128", "255", "256",
        "257", "511", "-1", "-2", "-3", "-7", "-16", "-64", "-255", "-256", "-511",
    ];
    let nonneg_ints = [
        "0", "1", "2", "3", "4", "7", "8", "16", "31", "32", "64", "127", "128", "255", "256",
        "257", "384", "511", "512",
    ];
    let mut cases = Vec::new();
    for _ in 0..320 {
        let which = next(8) as i64 - 1; // -1 ..= 6
        let pool: &[&str] = if which == 5 { &nonneg_ints } else { &ints };
        let x = floats[next(floats.len() as u64) as usize];
        let y = floats[next(floats.len() as u64) as usize];
        let z = floats[next(floats.len() as u64) as usize];
        let xw = pool[next(pool.len() as u64) as usize];
        let yw = pool[next(pool.len() as u64) as usize];
        let zw = pool[next(pool.len() as u64) as usize];
        let seed = pool[next(pool.len() as u64) as usize];
        let lac = floats[next(floats.len() as u64) as usize];
        let gain = floats[next(floats.len() as u64) as usize];
        let off = floats[next(floats.len() as u64) as usize];
        let oct = ["0", "-1", "1", "2", "3", "6", "7"][next(7) as usize];
        cases.push(format!(
            "{which} {x} {y} {z} {xw} {yw} {zw} {seed} {lac} {gain} {off} {oct}"
        ));
    }
    for c in &cases {
        same(c);
    }
}
