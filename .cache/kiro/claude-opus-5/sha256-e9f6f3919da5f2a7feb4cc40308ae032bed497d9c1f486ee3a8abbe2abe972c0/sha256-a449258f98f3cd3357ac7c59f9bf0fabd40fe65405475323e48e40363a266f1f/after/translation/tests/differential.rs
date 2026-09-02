//! Differential test against the original C program.
//!
//! This is an executable, so it is compared by *running* it: both binaries are
//! spawned as subprocesses with the same bytes on stdin, and stdout, stderr and
//! the exit status (including a terminating signal) must be identical. Nothing
//! here calls the translated code as a library.
//!
//! The corpus below is one input per branch class of `c_src`, enumerated from
//! the source: every `case` of `inner`'s switch plus its `default`, every point
//! at which the single `scanf` can stop, every `%d`/`%f` matching failure and
//! overflow, both branches of `stb__perlin_fastfloor`, the wrap masks of
//! `stb_perlin_noise3`, the `(unsigned char)` seed truncations, the three
//! fractal loops with zero/negative/huge octave counts, and every branch of
//! `stb_perlin_noise3_wrap_nonpow2` — including the inputs that make it trap
//! with SIGFPE or SIGSEGV.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path of the binary cargo built from `src/main.rs`.
const RUST_EXE: &str = env!("CARGO_BIN_EXE_driver");

/// Locates the C reference binary, building it out-of-source if needed.
/// `c_src/` itself is never written to.
fn c_exe() -> &'static Path {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_src = manifest.parent().expect("workspace root").join("c_src");

        let prebuilt = c_src.join("build").join("driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let build_dir = manifest.join("target").join("c_ref");
        let exe = build_dir.join("driver");
        if exe.is_file() {
            return exe;
        }
        std::fs::create_dir_all(&build_dir).expect("create the C build directory");

        let configured = Command::new("cmake")
            .arg("-S")
            .arg(&c_src)
            .arg("-B")
            .arg(&build_dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .arg("--build")
                .arg(&build_dir)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

        if !configured {
            // cmake with no CMAKE_BUILD_TYPE passes no optimisation flags, so a
            // bare `cc` invocation is an equivalent fallback.
            let ok = Command::new("cc")
                .arg("-o")
                .arg(&exe)
                .arg(c_src.join("src").join("main.c"))
                .arg("-lm")
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "could not build the C reference program");
        }
        assert!(exe.is_file(), "C reference binary missing at {}", exe.display());
        exe
    })
}

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
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

fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    // The program may die (SIGFPE/SIGSEGV) before draining stdin, which turns
    // the write into EPIPE; that is not a test failure.
    let _ = child.stdin.as_mut().expect("stdin").write_all(input);
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait for the child");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// One input per branch class of `c_src`; see the module comment.
const CASES: &[&str] = &[
    // inner switch: every `which` case, incl. the NAN default
    "0 0.5 0.25 0.125 0 0 0 0 0 0 0 0",
    "1 0.5 0.25 0.125 0 0 0 7 0 0 0 0",
    "2 0.5 0.5 0.5 0 0 0 0 2 0.5 1 6",
    "3 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
    "4 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
    "5 0.5 0.5 0.5 0 0 0 0 0 0 0 0",
    "-1 1 2 3 0 0 0 0 0 0 0 0",
    "-2 1 2 3 0 0 0 0 0 0 0 0",
    "6 1 2 3 0 0 0 0 0 0 0 0",
    "7 1 2 3 0 0 0 0 0 0 0 0",
    "100 1 2 3 0 0 0 0 0 0 0 0",
    "2147483647 1 2 3 0 0 0 0 0 0 0 0",
    "-2147483648 1 2 3 0 0 0 0 0 0 0 0",
    // scanf stops after 0 conversion(s)
    "",
    // scanf stops after 1 conversion(s)
    "3",
    // scanf stops after 2 conversion(s)
    "3 1.5",
    // scanf stops after 3 conversion(s)
    "3 1.5 2.5",
    // scanf stops after 4 conversion(s)
    "3 1.5 2.5 3.5",
    // scanf stops after 5 conversion(s)
    "3 1.5 2.5 3.5 8",
    // scanf stops after 6 conversion(s)
    "3 1.5 2.5 3.5 8 4",
    // scanf stops after 7 conversion(s)
    "3 1.5 2.5 3.5 8 4 2",
    // scanf stops after 8 conversion(s)
    "3 1.5 2.5 3.5 8 4 2 9",
    // scanf stops after 9 conversion(s)
    "3 1.5 2.5 3.5 8 4 2 9 2",
    // scanf stops after 10 conversion(s)
    "3 1.5 2.5 3.5 8 4 2 9 2 0.5",
    // scanf stops after 11 conversion(s)
    "3 1.5 2.5 3.5 8 4 2 9 2 0.5 1",
    // scanf stops after 12 conversion(s)
    "3 1.5 2.5 3.5 8 4 2 9 2 0.5 1 6",
    // scanf: empty and whitespace-only input
    "",
    " ",
    "\n",
    "\t",
    "\r\n",
    " \t\n\x0b\x0c\r ",
    // scanf: more fields than directives
    "3 1.5 2.5 3.5 8 4 2 9 2 0.5 1 6 7 8 9",
    // scanf: trailing whitespace
    "3 1.5 2.5 3.5 8 4 2 9 2 0.5 1 6\n\n",
    // scanf reads across newlines
    "3\n1.5\n2.5\n3.5\n8\n4\n2\n9\n2\n0.5\n1\n6",
    "3\r\n1.5\r\n2.5\r\n3.5\r\n8\r\n4\r\n2\r\n9\r\n2\r\n0.5\r\n1\r\n6",
    "3 \t\n 1.5 \t\n 2.5 \t\n 3.5 \t\n 8 \t\n 4 \t\n 2 \t\n 9 \t\n 2 \t\n 0.5 \t\n 1 \t\n 6",
    // %d matching failure on `which`
    "abc",
    "-",
    "+",
    ".",
    "x1",
    "--1",
    "+-1",
    "e1",
    // %d overflow is stored truncated to int
    "99999999999999999999 1 2 3 0 0 0 0 0 0 0 0",
    "-99999999999999999999 1 2 3 0 0 0 0 0 0 0 0",
    "2147483648 1 2 3 0 0 0 0 0 0 0 0",
    "-2147483649 1 2 3 0 0 0 0 0 0 0 0",
    "4294967296 1 2 3 0 0 0 0 0 0 0 0",
    "9223372036854775807 1 2 3 0 0 0 0 0 0 0 0",
    "9223372036854775808 1 2 3 0 0 0 0 0 0 0 0",
    "-9223372036854775808 1 2 3 0 0 0 0 0 0 0 0",
    "0 1 2 3 99999999999999999999 0 0 0 0 0 0 0",
    "0 1 2 3 0 0 0 99999999999999999999 0 0 0 0",
    "3 1 2 3 0 0 0 0 2 0.5 0 99999999999999999999",
    // %d with a leading plus / redundant zeros
    "+0 +1 +2 +3 +0 +0 +0 +0 +0 +0 +0 +0",
    "0 1 2 3 0 0 0 0000000000000000000255 0 0 0 0",
    // %f: decimal forms
    "0 .5 1. -.25 0 0 0 5 2 .5 1 4",
    "0 0. 0.0 -0.0 0 0 0 0 0 0 0 0",
    "0 1e5 1e-5 1e+5 0 0 0 0 0 0 0 0",
    "0 1E5 1E-5 1E+5 0 0 0 0 0 0 0 0",
    // %f: a dangling exponent marker is consumed, then fails
    "0 1e 2 3 0 0 0 5 2 .5 1 4",
    "0 1e- 2 3 0 0 0 5 2 .5 1 4",
    "0 1e+ 2 3 0 0 0 5 2 .5 1 4",
    // %f: matching failure with no digits
    "0 . 1 1 0 0 0 0 0 0 0 0",
    "0 -. 1 1 0 0 0 0 0 0 0 0",
    "0 e5 1 1 0 0 0 0 0 0 0 0",
    "0 - 1 1 0 0 0 0 0 0 0 0",
    // %f: hexadecimal forms
    "0 0x10 0x1p-1 1 0 0 0 0 0 0 0 0",
    "0 0X10 0X1P-1 1 0 0 0 0 0 0 0 0",
    "0 0x1.8p1 0x0 1 0 0 0 0 0 0 0 0",
    "0 0x.8 0x8. 1 0 0 0 0 0 0 0 0",
    "0 0x1p 1 1 0 0 0 0 0 0 0 0",
    "0 0x1p+ 1 1 0 0 0 0 0 0 0 0",
    "0 0x 1 1 0 0 0 0 0 0 0 0",
    "0 0X 1 1 0 0 0 0 0 0 0 0",
    "0 0x. 1 1 0 0 0 0 0 0 0 0",
    "0 0xg 1 1 0 0 0 0 0 0 0 0",
    // %f: inf / infinity, and the partial spellings that fail
    "0 inf 1 1 0 0 0 0 0 0 0 0",
    "0 INF 1 1 0 0 0 0 0 0 0 0",
    "0 Inf 1 1 0 0 0 0 0 0 0 0",
    "0 -inf 1 1 0 0 0 0 0 0 0 0",
    "0 +inf 1 1 0 0 0 0 0 0 0 0",
    "0 infinity 1 1 0 0 0 0 0 0 0 0",
    "0 INFINITY 1 1 0 0 0 0 0 0 0 0",
    "0 -infinity 1 1 0 0 0 0 0 0 0 0",
    "0 i 1 1 0 0 0 0 0 0 0 0",
    "0 in 1 1 0 0 0 0 0 0 0 0",
    "0 infi 1 1 0 0 0 0 0 0 0 0",
    "0 infin 1 1 0 0 0 0 0 0 0 0",
    "0 infinit 1 1 0 0 0 0 0 0 0 0",
    "0 inf3 1 1 0 0 0 0 0 0 0 0",
    // %f: nan, and that `nan(...)` never eats the parentheses
    "0 nan 0 0 0 0 0 0 0 0 0 0",
    "0 NAN 0 0 0 0 0 0 0 0 0 0",
    "0 NaN 0 0 0 0 0 0 0 0 0 0",
    "0 -nan 0 0 0 0 0 0 0 0 0 0",
    "0 nan(x) 1 1 0 0 0 0 0 0 0 0",
    "0 nan(123) 1 1 0 0 0 0 0 0 0 0",
    "0 nan() 1 1 0 0 0 0 0 0 0 0",
    "0 n 1 1 0 0 0 0 0 0 0 0",
    "0 na 1 1 0 0 0 0 0 0 0 0",
    "0 nans 1 1 0 0 0 0 0 0 0 0",
    // %f: rounding at the edges of the float range
    "0 3.4028235e38 -3.4028235e38 1 0 0 0 0 0 0 0 0",
    "0 3.4028236e38 1 1 0 0 0 0 0 0 0 0",
    "0 1e39 -1e39 1 0 0 0 0 0 0 0 0",
    "0 1e-45 7e-46 2.8e-45 0 0 0 0 0 0 0 0",
    "0 1e-46 1 1 0 0 0 0 0 0 0 0",
    "0 1.17549435e-38 1.17549421e-38 1 0 0 0 0 0 0 0 0",
    "0 16777217 8388609 1.0000001 0 0 0 0 0 0 0 0",
    "0 1e999 1e-999 1 0 0 0 0 0 0 0 0",
    "0 0.99999999999999999 1 1 0 0 0 0 0 0 0 0",
    "0 000000000000000000001 1 1 0 0 0 0 0 0 0 0",
    // fastfloor edge: 0
    "0 0 0 0 0 0 0 0 0 0 0 0",
    // fastfloor edge: -0.0
    "0 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0",
    // fastfloor edge: 1
    "0 1 1 1 0 0 0 0 0 0 0 0",
    // fastfloor edge: -1
    "0 -1 -1 -1 0 0 0 0 0 0 0 0",
    // fastfloor edge: 0.5
    "0 0.5 0.5 0.5 0 0 0 0 0 0 0 0",
    // fastfloor edge: -0.5
    "0 -0.5 -0.5 -0.5 0 0 0 0 0 0 0 0",
    // fastfloor edge: 255
    "0 255 255 255 0 0 0 0 0 0 0 0",
    // fastfloor edge: 256
    "0 256 256 256 0 0 0 0 0 0 0 0",
    // fastfloor edge: -256
    "0 -256 -256 -256 0 0 0 0 0 0 0 0",
    // fastfloor edge: 8388608
    "0 8388608 8388608 8388608 0 0 0 0 0 0 0 0",
    // fastfloor edge: 16777216
    "0 16777216 16777216 16777216 0 0 0 0 0 0 0 0",
    // fastfloor edge: 2147483520
    "0 2147483520 2147483520 2147483520 0 0 0 0 0 0 0 0",
    // fastfloor edge: 2147483647
    "0 2147483647 2147483647 2147483647 0 0 0 0 0 0 0 0",
    // fastfloor edge: 2147483648
    "0 2147483648 2147483648 2147483648 0 0 0 0 0 0 0 0",
    // fastfloor edge: -2147483648
    "0 -2147483648 -2147483648 -2147483648 0 0 0 0 0 0 0 0",
    // fastfloor edge: -2147483649
    "0 -2147483649 -2147483649 -2147483649 0 0 0 0 0 0 0 0",
    // fastfloor edge: 1e30
    "0 1e30 1e30 1e30 0 0 0 0 0 0 0 0",
    // fastfloor edge: -1e30
    "0 -1e30 -1e30 -1e30 0 0 0 0 0 0 0 0",
    // fastfloor edge: 1e-45
    "0 1e-45 1e-45 1e-45 0 0 0 0 0 0 0 0",
    // fastfloor edge: -1e-45
    "0 -1e-45 -1e-45 -1e-45 0 0 0 0 0 0 0 0",
    // fastfloor edge: inf
    "0 inf inf inf 0 0 0 0 0 0 0 0",
    // fastfloor edge: -inf
    "0 -inf -inf -inf 0 0 0 0 0 0 0 0",
    // fastfloor edge: nan
    "0 nan nan nan 0 0 0 0 0 0 0 0",
    // noise3 wrap mask: 0
    "0 12.75 -3.5 8.125 0 0 0 0 0 0 0 0",
    // noise3 wrap mask: 1
    "0 12.75 -3.5 8.125 1 1 1 0 0 0 0 0",
    // noise3 wrap mask: 2
    "0 12.75 -3.5 8.125 2 2 2 0 0 0 0 0",
    // noise3 wrap mask: 4
    "0 12.75 -3.5 8.125 4 4 4 0 0 0 0 0",
    // noise3 wrap mask: 16
    "0 12.75 -3.5 8.125 16 16 16 0 0 0 0 0",
    // noise3 wrap mask: 256
    "0 12.75 -3.5 8.125 256 256 256 0 0 0 0 0",
    // noise3 wrap mask: 512
    "0 12.75 -3.5 8.125 512 512 512 0 0 0 0 0",
    // noise3 wrap mask: 1024
    "0 12.75 -3.5 8.125 1024 1024 1024 0 0 0 0 0",
    // noise3 wrap mask: 3
    "0 12.75 -3.5 8.125 3 3 3 0 0 0 0 0",
    // noise3 wrap mask: 5
    "0 12.75 -3.5 8.125 5 5 5 0 0 0 0 0",
    // noise3 wrap mask: -1
    "0 12.75 -3.5 8.125 -1 -1 -1 0 0 0 0 0",
    // noise3 wrap mask: -4
    "0 12.75 -3.5 8.125 -4 -4 -4 0 0 0 0 0",
    // noise3 wrap mask: 2147483647
    "0 12.75 -3.5 8.125 2147483647 2147483647 2147483647 0 0 0 0 0",
    // noise3 wrap mask: -2147483648
    "0 12.75 -3.5 8.125 -2147483648 -2147483648 -2147483648 0 0 0 0 0",
    // noise3 mixed wrap masks
    "0 12.75 -3.5 8.125 16 4 2 0 0 0 0 0",
    "0 12.75 -3.5 8.125 0 16 0 0 0 0 0 0",
    "0 -20.5 33.25 -0.75 8 0 512 0 0 0 0 0",
    // noise3_seed seed truncation: 0
    "1 -20.5 33.25 -0.75 8 4 2 0 0 0 0 0",
    // noise3_seed seed truncation: 1
    "1 -20.5 33.25 -0.75 8 4 2 1 0 0 0 0",
    // noise3_seed seed truncation: 7
    "1 -20.5 33.25 -0.75 8 4 2 7 0 0 0 0",
    // noise3_seed seed truncation: 127
    "1 -20.5 33.25 -0.75 8 4 2 127 0 0 0 0",
    // noise3_seed seed truncation: 128
    "1 -20.5 33.25 -0.75 8 4 2 128 0 0 0 0",
    // noise3_seed seed truncation: 200
    "1 -20.5 33.25 -0.75 8 4 2 200 0 0 0 0",
    // noise3_seed seed truncation: 255
    "1 -20.5 33.25 -0.75 8 4 2 255 0 0 0 0",
    // noise3_seed seed truncation: 256
    "1 -20.5 33.25 -0.75 8 4 2 256 0 0 0 0",
    // noise3_seed seed truncation: 257
    "1 -20.5 33.25 -0.75 8 4 2 257 0 0 0 0",
    // noise3_seed seed truncation: 511
    "1 -20.5 33.25 -0.75 8 4 2 511 0 0 0 0",
    // noise3_seed seed truncation: -1
    "1 -20.5 33.25 -0.75 8 4 2 -1 0 0 0 0",
    // noise3_seed seed truncation: -255
    "1 -20.5 33.25 -0.75 8 4 2 -255 0 0 0 0",
    // noise3_seed seed truncation: -256
    "1 -20.5 33.25 -0.75 8 4 2 -256 0 0 0 0",
    // noise3_seed seed truncation: 65535
    "1 -20.5 33.25 -0.75 8 4 2 65535 0 0 0 0",
    // noise3_seed seed truncation: 2147483647
    "1 -20.5 33.25 -0.75 8 4 2 2147483647 0 0 0 0",
    // noise3_seed seed truncation: -2147483648
    "1 -20.5 33.25 -0.75 8 4 2 -2147483648 0 0 0 0",
    // ridge octaves=0 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 0",
    // ridge octaves=-1 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -1",
    // ridge octaves=-100 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -100",
    // ridge octaves=1 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 1",
    // ridge octaves=2 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 2",
    // ridge octaves=6 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 6",
    // ridge octaves=10 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 10",
    // ridge octaves=255 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 255",
    // ridge octaves=256 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 256",
    // ridge octaves=257 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 257",
    // ridge octaves=300 (i is cast to unsigned char per octave)
    "2 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 300",
    // ridge with degenerate lacunarity/gain/offset
    "2 0.5 0.5 0.5 0 0 0 0 0 0 0 6",
    "2 0.5 0.5 0.5 0 0 0 0 1 1 1 6",
    "2 0.5 0.5 0.5 0 0 0 0 -2 -0.5 -1 8",
    "2 0.5 0.5 0.5 0 0 0 0 1e20 1e20 1 6",
    "2 0.5 0.5 0.5 0 0 0 0 inf 0.5 1 4",
    "2 0.5 0.5 0.5 0 0 0 0 nan 0.5 1 4",
    "2 0.5 0.5 0.5 0 0 0 0 2 nan 1 4",
    "2 0.5 0.5 0.5 0 0 0 0 2 0.5 nan 4",
    "2 0.5 0.5 0.5 0 0 0 0 2 0.5 inf 4",
    "2 nan 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "2 inf 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "2 1e30 1 1 0 0 0 0 2 0 0 4",
    "2 1e20 1 1 0 0 0 0 2 0 0 4",
    "2 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8",
    "2 0.001 0.002 0.003 1.0000001 0.9999999 1 20",
    // fbm octaves=0 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 0",
    // fbm octaves=-1 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -1",
    // fbm octaves=-100 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -100",
    // fbm octaves=1 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 1",
    // fbm octaves=2 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 2",
    // fbm octaves=6 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 6",
    // fbm octaves=10 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 10",
    // fbm octaves=255 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 255",
    // fbm octaves=256 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 256",
    // fbm octaves=257 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 257",
    // fbm octaves=300 (i is cast to unsigned char per octave)
    "3 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 300",
    // fbm with degenerate lacunarity/gain/offset
    "3 0.5 0.5 0.5 0 0 0 0 0 0 0 6",
    "3 0.5 0.5 0.5 0 0 0 0 1 1 1 6",
    "3 0.5 0.5 0.5 0 0 0 0 -2 -0.5 -1 8",
    "3 0.5 0.5 0.5 0 0 0 0 1e20 1e20 1 6",
    "3 0.5 0.5 0.5 0 0 0 0 inf 0.5 1 4",
    "3 0.5 0.5 0.5 0 0 0 0 nan 0.5 1 4",
    "3 0.5 0.5 0.5 0 0 0 0 2 nan 1 4",
    "3 0.5 0.5 0.5 0 0 0 0 2 0.5 nan 4",
    "3 0.5 0.5 0.5 0 0 0 0 2 0.5 inf 4",
    "3 nan 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "3 inf 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "3 1e30 1 1 0 0 0 0 2 0 0 4",
    "3 1e20 1 1 0 0 0 0 2 0 0 4",
    "3 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8",
    "3 0.001 0.002 0.003 1.0000001 0.9999999 1 20",
    // turbulence octaves=0 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 0",
    // turbulence octaves=-1 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -1",
    // turbulence octaves=-100 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 -100",
    // turbulence octaves=1 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 1",
    // turbulence octaves=2 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 2",
    // turbulence octaves=6 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 6",
    // turbulence octaves=10 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 10",
    // turbulence octaves=255 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 255",
    // turbulence octaves=256 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 256",
    // turbulence octaves=257 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 257",
    // turbulence octaves=300 (i is cast to unsigned char per octave)
    "4 1.5 -2.25 3.75 0 0 0 0 1.9 0.55 1.25 300",
    // turbulence with degenerate lacunarity/gain/offset
    "4 0.5 0.5 0.5 0 0 0 0 0 0 0 6",
    "4 0.5 0.5 0.5 0 0 0 0 1 1 1 6",
    "4 0.5 0.5 0.5 0 0 0 0 -2 -0.5 -1 8",
    "4 0.5 0.5 0.5 0 0 0 0 1e20 1e20 1 6",
    "4 0.5 0.5 0.5 0 0 0 0 inf 0.5 1 4",
    "4 0.5 0.5 0.5 0 0 0 0 nan 0.5 1 4",
    "4 0.5 0.5 0.5 0 0 0 0 2 nan 1 4",
    "4 0.5 0.5 0.5 0 0 0 0 2 0.5 nan 4",
    "4 0.5 0.5 0.5 0 0 0 0 2 0.5 inf 4",
    "4 nan 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "4 inf 0.5 0.5 0 0 0 0 2 0.5 1 4",
    "4 1e30 1 1 0 0 0 0 2 0 0 4",
    "4 1e20 1 1 0 0 0 0 2 0 0 4",
    "4 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8",
    "4 0.001 0.002 0.003 1.0000001 0.9999999 1 20",
    // nonpow2 `wrap ? wrap : 256` combination 0/0/0
    "5 -12.25 7.5 -3.125 0 0 0 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 0/0/5
    "5 -12.25 7.5 -3.125 0 0 5 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 0/3/0
    "5 -12.25 7.5 -3.125 0 3 0 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 0/3/5
    "5 -12.25 7.5 -3.125 0 3 5 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 7/0/0
    "5 -12.25 7.5 -3.125 7 0 0 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 7/0/5
    "5 -12.25 7.5 -3.125 7 0 5 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 7/3/0
    "5 -12.25 7.5 -3.125 7 3 0 200 0 0 0 0",
    // nonpow2 `wrap ? wrap : 256` combination 7/3/5
    "5 -12.25 7.5 -3.125 7 3 5 200 0 0 0 0",
    // nonpow2 the `x0 < 0` fixups
    "5 -1.5 -1.5 -1.5 7 3 5 0 0 0 0 0",
    "5 -1.5 2.5 -1.5 7 3 5 0 0 0 0 0",
    "5 -0.5 -0.5 -0.5 256 256 256 0 0 0 0 0",
    "5 -257.5 -257.5 -257.5 0 0 0 0 0 0 0 0",
    // nonpow2 in-bounds and out-of-bounds table indices
    "5 0.5 0.5 0.5 512 512 512 0 0 0 0 0",
    "5 511 0 0 512 1 1 0 0 0 0 0",
    "5 600.5 0 0 1000 1 1 0 0 0 0 0",
    "5 0 600.5 0 1 1000 1 0 0 0 0 0",
    "5 0 0 600.5 1 1 1000 0 0 0 0 0",
    "5 1200 0 0 1300 1 1 0 0 0 0 0",
    "5 0 0 1200 1 1 1300 0 0 0 0 0",
    "5 -4 0 0 -3 1 1 0 0 0 0 0",
    "5 -300 0 0 -299 1 1 0 0 0 0 0",
    "5 -300 -300 -300 -299 -299 -299 0 0 0 0 0",
    "5 100.5 -100.5 50.25 3 5 7 9 0 0 0 0",
    "5 -12.25 7.5 -3.125 6 10 14 200 0 0 0 0",
    // nonpow2 seed truncation
    "5 0.5 0.5 0.5 7 3 5 255 0 0 0 0",
    "5 0.5 0.5 0.5 7 3 5 256 0 0 0 0",
    "5 0.5 0.5 0.5 7 3 5 -1 0 0 0 0",
    "5 0.5 0.5 0.5 7 3 5 65535 0 0 0 0",
    // nonpow2 SIGFPE: `INT_MIN % -1` traps in idivl
    "5 -2147483648 0 0 -1 0 0 0 0 0 0 0",
    "5 0 -2147483648 0 0 -1 0 0 0 0 0 0",
    "5 0 0 -2147483648 0 0 -1 0 0 0 0 0",
    "5 nan 0 0 -1 0 0 0 0 0 0 0",
    "5 inf 0 0 -1 0 0 0 0 0 0 0",
    "5 1e30 0 0 -1 0 0 0 0 0 0 0",
    "5 2147483648 0 0 -1 0 0 0 0 0 0 0",
    // nonpow2 no trap for wrap == -1 with an in-range px
    "5 0 0 0 -1 -1 -1 0 0 0 0 0",
    "5 -2147483647 0 0 -1 0 0 0 0 0 0 0",
    "5 2147483647 0 0 -1 0 0 0 0 0 0 0",
    "5 -2147483648 0 0 -2 0 0 0 0 0 0 0",
    "5 -2147483648 0 0 1 0 0 0 0 0 0 0",
    "5 -2147483648 0 0 -2147483648 0 0 0 0 0 0 0",
    // nonpow2 SIGSEGV: the table index leaves the mapped pages
    "5 1e9 0 0 2000000000 0 0 0 0 0 0 0",
    "5 0 1e9 0 0 2000000000 0 0 0 0 0 0",
    "5 0 0 1e9 0 0 2000000000 0 0 0 0 0",
    "5 100000 0 0 2147483647 1 1 0 0 0 0 0",
    "5 -100000 0 0 -99999 1 1 0 0 0 0 0",
    "5 0 0 -100000 1 1 -99999 0 0 0 0 0",
    // NaN sign propagation through the SSE operand order
    "2 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8",
    "3 1e20 1 1 0 0 0 0 2 0 0 4",
    "4 1e20 1 1 0 0 0 0 2 0 0 4",
    "0 inf 1 1 0 0 0 0 0 0 0 0",
    "0 -inf 1 1 0 0 0 0 0 0 0 0",
    "0 1 inf 1 0 0 0 0 0 0 0 0",
    "0 1 1 inf 0 0 0 0 0 0 0 0",
    "0 nan 1 1 0 0 0 0 0 0 0 0",
    "0 -nan 1 1 0 0 0 0 0 0 0 0",
    "5 1e30 1e30 1e30 0 0 0 0 0 0 0 0",
    "5 inf inf inf 0 0 0 0 0 0 0 0",
    "5 nan nan nan 0 0 0 0 0 0 0 0",
    // printf %.9g: %f style, %e style and the trailing-zero trimming
    "3 -7.125 4.5 0.875 0 0 0 0 2.5 0.4 0 10",
    "4 9.25 -1.5 6.75 0 0 0 0 1.75 0.6 0 5",
    "0 0.5 0.5 0.5 0 0 0 0 0 0 0 0",
    "3 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
    "4 0.5 0.5 0.5 0 0 0 0 2 0.5 0 6",
    "3 1e-8 2e-8 3e-8 0 0 0 0 2 0.5 0 1",
    "3 1e-20 2e-20 3e-20 0 0 0 0 2 0.5 0 1",
    "4 1e-30 1e-30 1e-30 0 0 0 0 2 1e-20 0 3",
    "2 0.5 0.5 0.5 0 0 0 0 2 0.5 1e20 6",
    "3 1 1 1 0 0 0 0 2 1e10 0 20",
];

#[test]
fn stdout_stderr_and_exit_status_match_the_c_program() {
    let c = c_exe();
    let rust = Path::new(RUST_EXE);
    assert!(rust.is_file(), "missing {RUST_EXE}");

    let mut failures = Vec::new();
    for input in CASES {
        let expected = run(c, input.as_bytes());
        let got = run(rust, input.as_bytes());
        if expected != got {
            failures.push(format!(
                "input {input:?}\n     C: {expected:?}\n  rust: {got:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} inputs differ:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}

/// The corpus has to actually exercise the interesting exit paths, otherwise a
/// regression there would go unnoticed.
#[test]
fn the_corpus_reaches_the_signalling_paths() {
    let c = c_exe();
    let mut normal = 0;
    let mut sigfpe = 0;
    let mut sigsegv = 0;
    for input in CASES {
        match run(c, input.as_bytes()).signal() {
            None => normal += 1,
            Some(8) => sigfpe += 1,
            Some(11) => sigsegv += 1,
            Some(s) => panic!("unexpected signal {s} for {input:?}"),
        }
    }
    assert!(normal > 250, "only {normal} inputs exit normally");
    assert!(sigfpe >= 5, "only {sigfpe} inputs reach the idivl trap");
    assert!(sigsegv >= 5, "only {sigsegv} inputs reach the unmapped read");
}

impl Outcome {
    fn signal(&self) -> Option<i32> {
        self.signal
    }
}
