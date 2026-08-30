//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses with identical stdin and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell drives them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Once;

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root (parent of the `translation` crate directory).
fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_src_dir() -> PathBuf {
    repo_root().join("c_src")
}

fn c_bin_path() -> PathBuf {
    c_src_dir().join("build").join("driver")
}

static BUILD_C: Once = Once::new();

/// Ensures the C reference binary exists, building it with CMake if needed.
/// `c_src/` is only ever read from / built in place; its sources are never
/// modified.
fn c_bin() -> PathBuf {
    BUILD_C.call_once(|| {
        if c_bin_path().exists() {
            return;
        }
        let src = c_src_dir();
        let build = src.join("build");
        std::fs::create_dir_all(&build).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake` -- is CMake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
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
            String::from_utf8_lossy(&compile.stderr)
        );
    });

    let p = c_bin_path();
    assert!(
        p.exists(),
        "C reference binary missing at {}; build it with \
         `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`",
        p.display()
    );
    p
}

/// Runs `bin` with `input` on stdin, capturing stdout, stderr and status.
fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The child may stop reading early; a broken pipe is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", bin.display()))
}

/// Asserts the C and Rust programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, input: &[u8]) {
    let c = run(&c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Preview(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit status differs\n  input:  {:?}\n  C:      {:?}\n  Rust:   {:?}",
        Preview(input),
        c.status,
        r.status,
    );
}

/// Truncating debug wrapper so failure messages stay readable.
struct Preview<'a>(&'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8_lossy(self.0);
        if s.len() <= 160 {
            write!(f, "{s:?}")
        } else {
            // Truncate on a char boundary so this helper can never panic.
            let head: String = s.chars().take(160).collect();
            write!(f, "{head:?}... ({} bytes)", self.0.len())
        }
    }
}

fn check(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// call_fma: len == 0 early return
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // scanf fails immediately -> i == 0 -> call_fma returns 0.
    check("empty", "");
}

#[test]
fn whitespace_only_input() {
    // Leading whitespace is consumed, then EOF: still an input failure.
    check("spaces_only", "   ");
    check("newlines_only", "\n\n\n");
    check("all_c_space", " \t\n\x0b\x0c\r");
}

#[test]
fn first_token_not_a_number() {
    // Matching failure on the very first scanf -> i == 0 -> prints 0.
    check("alpha", "abc");
    check("alpha_nl", "abc\n");
    check("punct", ",");
    check("semicolon", ";7 8");
    check("dot_five", ".5");
    check("lone_minus", "-");
    check("lone_plus", "+");
    check("double_minus", "--5");
    check("minus_plus", "-+5");
    check("minus_then_alpha", "-a");
    check("plus_then_eof_nl", "+\n");
}

// ---------------------------------------------------------------------------
// call_fma: the normal path (result is the last value read)
// ---------------------------------------------------------------------------

#[test]
fn single_item() {
    check("single", "5");
    check("single_nl", "5\n");
    check("single_trailing_spaces", "5   ");
    check("single_zero", "0");
    check("single_negative", "-7");
    check("single_plus", "+8");
    check("single_leading_zeros", "000000000000000000000000005");
}

#[test]
fn several_items() {
    check("three", "1 2 3");
    check("three_nl", "1\n2\n3\n");
    // scanf reads across newlines, unlike fgets.
    check("across_newlines", "1\n2 3\n\n  4\t5\r\n6");
    check("mixed_signs", "-1 +2 -3 +4 -5");
    check("last_is_zero", "9 9 9 0");
    check("last_is_negative", "1 2 -99");
}

#[test]
fn indentation_and_odd_whitespace() {
    check("leading_ws", "   \n  42  \n");
    check("vt_ff_cr", "\t\x0b\x0c\r 9");
    check("tabs_between", "1\t\t\t2");
}

// ---------------------------------------------------------------------------
// Early break out of the read loop: garbage after valid numbers
// ---------------------------------------------------------------------------

#[test]
fn garbage_stops_the_loop() {
    check("garbage_mid", "1 2 x 3");
    check("garbage_mid2", "10 20 zzz 30 40");
    // "%d" does not accept a 0x prefix: "0" converts, then 'x' fails.
    check("hex_prefix", "0x10");
    check("hex_prefix_upper", "0X1F 5");
    // "%d" stops at 'e': "1" converts, then 'e' fails.
    check("exponent", "1e5");
    check("float", "3.14");
    check("float_then_int", "3.14 7");
    check("comma_separated", "1,2,3");
    check("nul_byte_first", "\x00 7");
    check("nul_byte_mid", "1 2\x003 4");
    check("trailing_minus", "1 2 -");
    check("trailing_plus", "1 2 +");
    check("underscore", "1 2 _3");
}

// ---------------------------------------------------------------------------
// int / long boundaries: glibc converts via strtol, clamps, then truncates
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    check("int_max", "2147483647");
    check("int_min", "-2147483648");
    check("int_max_plus_1", "2147483648");
    check("int_min_minus_1", "-2147483649");
    check("u32_max", "4294967295");
    check("u32_max_plus_1", "4294967296");
    check("truncating_value", "4294967297");
}

#[test]
fn long_boundaries_and_overflow() {
    check("long_max", "9223372036854775807");
    check("long_min", "-9223372036854775808");
    check("long_max_plus_1", "9223372036854775808");
    check("long_min_minus_1", "-9223372036854775809");
    check("way_over", "99999999999999999999");
    check("way_under", "-99999999999999999999");
    check("absurdly_long", &format!("{}", "9".repeat(400)));
    check("absurdly_long_neg", &format!("-{}", "9".repeat(400)));
    check("padded_overflow", &format!("{}{}", "0".repeat(50), "9".repeat(30)));
}

#[test]
fn overflow_is_not_the_last_value() {
    // The overflowing value is read successfully, so the loop continues.
    check("overflow_then_ok", "99999999999999999999 42");
    check("ok_then_overflow", "42 99999999999999999999");
}

// ---------------------------------------------------------------------------
// The 100-element bound on `data`
// ---------------------------------------------------------------------------

fn seq(n: usize) -> String {
    (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join("\n") + "\n"
}

#[test]
fn exactly_below_at_and_above_the_maximum() {
    check("n_1", &seq(1));
    check("n_2", &seq(2));
    check("n_99", &seq(99));
    // Exactly the maximum the code handles.
    check("n_100", &seq(100));
    // The loop condition stops at 100; the 101st token is never read.
    check("n_101", &seq(101));
    check("n_150", &seq(150));
    check("n_500", &seq(500));
}

#[test]
fn maximum_on_one_line_without_trailing_newline() {
    let s = (1..=100).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    check("oneline_100_no_nl", &s);
}

#[test]
fn maximum_followed_by_garbage() {
    // The 100 limit is reached before the garbage would be seen.
    let mut s = (1..=101).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    s.push_str(" zzz");
    check("100_then_garbage", &s);
}

#[test]
fn garbage_at_element_100() {
    // 99 good values then garbage: loop breaks with i == 99.
    let mut s = (1..=99).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    s.push_str(" q 1000");
    check("garbage_at_100", &s);
}

// ---------------------------------------------------------------------------
// Larger inputs: exercise stdio buffer refills, including a number that
// straddles the Rust reader's internal chunk boundary.
// ---------------------------------------------------------------------------

#[test]
fn number_straddling_the_read_buffer_boundary() {
    for off in [8188usize, 8189, 8190, 8191, 8192, 8193, 16383, 16384] {
        let pad: String = "1 ".repeat(off / 2 + 1).chars().take(off).collect();
        let input = format!("{pad}123456789 42");
        check(&format!("straddle_{off}"), &input);
    }
}

#[test]
fn very_large_input_stops_at_100() {
    let big = (1..=20_000).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    check("20000_tokens", &big);
}

// ---------------------------------------------------------------------------
// Non-UTF8 / arbitrary bytes
// ---------------------------------------------------------------------------

#[test]
fn arbitrary_binary_bytes() {
    assert_same("all_bytes", &(0u8..=255).collect::<Vec<u8>>());
    assert_same("high_bytes_then_num", b"\xff\xfe 5");
    assert_same("num_then_high_bytes", b"5 \xff\xfe");
    assert_same("invalid_utf8_mid", b"1 2 \xc3\x28 3");
}

// ---------------------------------------------------------------------------
// Deterministic pseudo-random sweep over the input alphabet the parser
// branches on. Uses a fixed seed so failures are reproducible.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn fuzz_over_parser_alphabet() {
    const ALPHA: &[u8] = b"0123456789 \t\n\r\x0b\x0c+-abcxXeE.,;#\x00";
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for case in 0..400 {
        let n = rng.below(48);
        let input: Vec<u8> = (0..n).map(|_| ALPHA[rng.below(ALPHA.len())]).collect();
        assert_same(&format!("fuzz_alpha_{case}"), &input);
    }
}

#[test]
fn fuzz_over_numeric_tokens() {
    let mut rng = Rng(0xdead_beef_cafe_1234);
    let widths = [1usize, 2, 5, 9, 10, 11, 18, 19, 20, 25, 40];
    let seps = [" ", "\n", "\t", "  \n ", "\r\n"];
    for case in 0..200 {
        let count = rng.below(130);
        let mut toks = Vec::with_capacity(count);
        for _ in 0..count {
            let w = widths[rng.below(widths.len())];
            let digits: String = (0..w)
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            let tok = match rng.below(5) {
                0 => format!("-{digits}"),
                1 => format!("+{digits}"),
                _ => digits,
            };
            toks.push(tok);
        }
        let sep = seps[rng.below(seps.len())];
        assert_same(&format!("fuzz_num_{case}"), toks.join(sep).as_bytes());
    }
}
