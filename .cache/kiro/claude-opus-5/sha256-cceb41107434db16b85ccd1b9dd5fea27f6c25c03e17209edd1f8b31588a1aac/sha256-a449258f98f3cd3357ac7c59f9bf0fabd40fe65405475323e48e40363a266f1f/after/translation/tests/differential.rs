//! Differential integration tests: run the C binary and the Rust binary as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ==================== locating / building the two binaries ====================

/// `translation/` — the directory holding this crate's Cargo.toml.
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory holding both `c_src/` and `translation/`.
fn work_dir() -> PathBuf {
    crate_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test. Built on demand so `cargo test` works
/// from a clean checkout.
fn rust_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // `CARGO_BIN_EXE_<name>` is set by cargo for integration tests and
        // already points at a freshly built binary.
        let p = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
        assert!(
            p.is_file(),
            "rust binary missing at {} (run `cargo build`)",
            p.display()
        );
        p
    })
    .as_path()
}

/// Path to the C binary under test, building it with cmake if needed.
fn c_bin() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let c_src = work_dir().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("cmake must be installed to build the C reference");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}",
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(
            exe.is_file(),
            "C binary missing at {}; build c_src first",
            exe.display()
        );
        exe
    })
    .as_path()
}

// ==================== running and comparing ====================

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    /// Raw wait status rendering, used only for failure messages.
    status: String,
}

fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        // The child may exit before consuming all of stdin (every error path
        // does exactly that), so a broken pipe here is expected, not a failure.
        let _ = sin.write_all(input);
        let _ = sin.flush();
    }

    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        status: format!("{}", out.status),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.len() <= 2000 => s.to_string(),
        Ok(s) => format!("{}…<{} bytes total>", &s[..2000], bytes.len()),
        Err(_) => format!("{:?}", bytes),
    }
}

/// The whole point of the suite: all three observable channels must match.
#[track_caller]
fn assert_same(name: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs\n--- input ---\n{}\n--- C stdout ---\n{}\n--- Rust stdout ---\n{}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs\n--- input ---\n{}\n--- C stderr ---\n{}\n--- Rust stderr ---\n{}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "[{name}] exit status differs: C {} vs Rust {}\n--- input ---\n{}",
        c.status,
        r.status,
        show(input)
    );
}

#[track_caller]
fn same(name: &str, input: &str) {
    assert_same(name, input.as_bytes());
}

// Convenience builders for wide inputs.
fn buf_line(len: usize, f: impl Fn(usize) -> i32) -> String {
    let mut s = len.to_string();
    for i in 0..len {
        s.push(' ');
        s.push_str(&f(i).to_string());
    }
    s
}

// ==================== Phase A: the binaries run at all ====================

#[test]
fn both_binaries_exist_and_run() {
    // Establishes the invocation used by every other test: no arguments, input
    // on stdin, output on stdout/stderr.
    let c = run(c_bin(), b"6 1\n1 5\n");
    let r = run(rust_bin(), b"6 1\n1 5\n");
    assert_eq!(c.code, Some(0), "C reference should exit 0 on a valid input");
    assert_eq!(r.code, Some(0));
    assert_eq!(c.stdout, r.stdout);
}

#[test]
fn argv_is_ignored_by_both() {
    // main() takes argc/argv but never reads them.
    let input = b"6 1\n1 5\n";
    let mut c = Command::new(c_bin());
    let mut r = Command::new(rust_bin());
    for cmd in [&mut c, &mut r] {
        cmd.args(["--nonsense", "extra", "-1"]);
    }
    let cr = {
        let mut ch = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        ch.stdin.take().unwrap().write_all(input).unwrap();
        ch.wait_with_output().unwrap()
    };
    let rr = {
        let mut ch = r
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        ch.stdin.take().unwrap().write_all(input).unwrap();
        ch.wait_with_output().unwrap()
    };
    assert_eq!(cr.stdout, rr.stdout);
    assert_eq!(cr.stderr, rr.stderr);
    assert_eq!(cr.status.code(), rr.status.code());
}

// ==================== Phase B: header parsing branches ====================

#[test]
fn empty_and_whitespace_only_input() {
    // `scanf("%d", &operation) != 1` -> "Failed to read operation", exit 1.
    same("empty", "");
    same("only_newline", "\n");
    same("only_spaces", "   ");
    same("only_mixed_ws", " \t\r\n\u{0b}\u{0c} ");
}

#[test]
fn operation_read_failures() {
    same("garbage_op", "abc");
    same("garbage_op_word", "x 1");
    same("sign_only_op", "-");
    same("plus_only_op", "+");
    same("double_sign_op", "--3");
    same("dot_op", ".5");
}

#[test]
fn buffer_count_read_failures() {
    // operation parsed, then EOF or a non-numeric token.
    same("count_eof", "0");
    same("count_eof_nl", "0\n");
    same("count_garbage", "0 abc");
    same("count_sign_only", "0 -");
}

#[test]
fn buffer_count_range_branches() {
    // if (buffer_count <= 0 || buffer_count > 100)
    same("count_zero", "0 0");
    same("count_negative", "0 -1");
    same("count_negative_big", "0 -5");
    same("count_101", "0 101");
    same("count_100_then_eof", "0 100");
    same("count_1_boundary_ok", "6 1\n1 1\n");
    // 100 is the maximum accepted; supply all 100 buffers.
    let mut s = String::from("6 100\n");
    for i in 0..100 {
        s.push_str(&buf_line(1, |_| i));
        s.push('\n');
    }
    same("count_100_full", &s);
}

// ==================== Phase B: read_buffer branches ====================

#[test]
fn buffer_length_branches() {
    // if (length < 0 || length > 256)
    same("len_negative", "1 1\n-1\n");
    same("len_257", "1 1\n257\n");
    same("len_huge", "1 1\n100000\n");
    same("len_zero", "1 1\n0\n");
    same("len_one", "1 1\n1 42\n");
    // 256 is the maximum accepted length.
    same("len_256", &format!("1 1\n{}\n", buf_line(256, |i| i as i32)));
    // 256 declared but no bytes follow: fails on byte 0.
    same("len_256_no_bytes", "1 1\n256\n");
}

#[test]
fn byte_read_failures_report_the_index() {
    // "Error: Failed to read byte %zu" for the first missing byte.
    same("byte0_eof", "1 1\n3\n");
    same("byte1_eof", "1 1\n3 7\n");
    same("byte2_eof", "1 1\n3 7 8\n");
    same("byte_garbage", "1 1\n3 7 q 9\n");
    same("byte_garbage_first", "1 1\n2 z 9\n");
    // failure while reading the *second* buffer
    same("second_buffer_byte_fail", "2 2\n1 1\n2 5\n");
}

#[test]
fn byte_values_are_truncated_to_u8() {
    // buf->data[i] = (uint8_t)byte;
    same("byte_trunc_over", "1 1\n4 256 257 300 511\n");
    same("byte_trunc_negative", "1 1\n4 -1 -2 -256 -257\n");
    same("byte_trunc_wide", "1 1\n3 4294967296 4294967297 -4294967295\n");
    same("byte_edges", "6 1\n4 0 127 128 255\n");
}

#[test]
fn scanf_crosses_newlines_and_accepts_all_whitespace() {
    // %d skips whitespace, so line structure is irrelevant.
    same("one_token_per_line", "1\n1\n3\n1\n2\n3\n");
    same("all_on_one_line", "1 1 3 1 2 3\n");
    same("tabs_and_crlf", "1\t1\r\n3\t1\r\n2 3\r\n");
    same("vt_ff", "1\u{0b}1\u{0c}3 1 2 3\n");
    same("leading_ws", "\n\n\t 1 1 3 1 2 3\n");
    same("no_trailing_newline", "6 1\n1 5");
    same("trailing_space", "6 1\n1 5 ");
    same("trailing_garbage_ignored", "6 1\n1 5\nGARBAGE HERE\n");
}

#[test]
fn scanf_number_syntax_quirks() {
    same("leading_zeros", "0006 1\n0001 0005\n");
    same("explicit_plus", "+6 +1\n+1 +5\n");
    same("negative_zero", "6 1\n1 -0\n");
    // %d stops at 'x': "0x1f" parses as 0, leaving "x1f" to fail the next read.
    same("hex_looking_op", "0x10 1\n1 1\n");
    same("hex_looking_byte", "6 1\n1 0x41\n");
    // digits then a sign: "5-3" parses 5, then "-3" is the next token.
    same("digits_then_sign", "6 1\n2 5-3\n");
}

#[test]
fn scanf_integer_overflow_matches_glibc() {
    // glibc converts to `long` (saturating) and stores the low 32 bits.
    same("op_int_max_plus_1", "2147483648 1\n1 1\n");
    same("op_2_pow_32", "4294967296 1\n1 1\n");
    same("op_int_min", "-2147483648 1\n1 1\n");
    same("op_long_overflow", "99999999999999999999999 1\n1 1\n");
    same("op_long_overflow_neg", "-99999999999999999999999 1\n1 1\n");
    // 4294967298 truncates to 2, a valid buffer count.
    same("count_wraps_to_2", "6 4294967298\n1 1\n1 2\n");
    // 4294967297 truncates to 1, a valid length.
    same("len_wraps_to_1", "6 1\n4294967297 5\n");
    // 4294967296 truncates to 0 -> operation 0 (copy).
    same("op_wraps_to_copy", "4294967296 2\n1 1\n1 2\n");
}

// ==================== Phase B/C: the operation switch ====================

#[test]
fn op_copy() {
    // OP_COPY = 0: copies buffers[0] into an uninitialized local and prints it.
    same("copy_two", "0 2\n3 1 2 3\n3 4 5 6\n");
    same("copy_needs_two", "0 1\n3 1 2 3\n");
    same("copy_empty_source", "0 2\n0\n0\n");
    same("copy_max_len", &format!("0 2\n{}\n0\n", buf_line(256, |i| i as i32)));
    same("copy_many", "0 5\n2 1 2\n1 3\n0\n1 4\n1 5\n");
}

#[test]
fn op_reverse() {
    // OP_REVERSE = 1: reverses and prints every buffer.
    same("reverse_one", "1 1\n3 1 2 3\n");
    same("reverse_many", "1 3\n3 1 2 3\n1 9\n0\n");
    same("reverse_empty_only", "1 1\n0\n");
    same("reverse_single_byte", "1 1\n1 7\n");
    same("reverse_max", &format!("1 1\n{}\n", buf_line(256, |i| i as i32)));
    same("reverse_all_empty", "1 3\n0\n0\n0\n");
    // even and odd lengths take different paths through the index arithmetic
    same("reverse_even", "1 1\n4 1 2 3 4\n");
    same("reverse_odd", "1 1\n5 1 2 3 4 5\n");
}

#[test]
fn op_merge() {
    // OP_MERGE = 2: merges buffers[0] and buffers[1].
    same("merge_basic", "2 2\n2 1 2\n2 3 4\n");
    same("merge_needs_two", "2 1\n1 1\n");
    same("merge_empty_both", "2 2\n0\n0\n");
    same("merge_empty_first", "2 2\n0\n2 1 2\n");
    same("merge_empty_second", "2 2\n2 1 2\n0\n");
    // sum exactly 256: allowed
    same(
        "merge_sum_256",
        &format!(
            "2 2\n{}\n{}\n",
            buf_line(128, |i| i as i32),
            buf_line(128, |i| (i * 7) as i32)
        ),
    );
    // sum 257: "Error: Merged length 257 exceeds maximum"
    same(
        "merge_sum_257",
        &format!(
            "2 2\n{}\n{}\n",
            buf_line(129, |i| i as i32),
            buf_line(128, |i| i as i32)
        ),
    );
    // sum 512: the largest reportable overflow
    same(
        "merge_sum_512",
        &format!(
            "2 2\n{}\n{}\n",
            buf_line(256, |i| i as i32),
            buf_line(256, |i| 255 - i as i32)
        ),
    );
    // extra buffers beyond the first two are read but unused
    same("merge_extra_buffers", "2 4\n1 1\n1 2\n1 3\n1 4\n");
}

#[test]
fn op_split() {
    // OP_SPLIT = 3: reads one extra int after all buffers.
    same("split_middle", "3 1\n5 1 2 3 4 5\n2\n");
    same("split_at_zero", "3 1\n3 1 2 3\n0\n");
    same("split_at_length", "3 1\n3 1 2 3\n3\n");
    same("split_past_length", "3 1\n3 1 2 3\n4\n");
    same("split_empty_buffer_at_0", "3 1\n0\n0\n");
    same("split_empty_buffer_at_1", "3 1\n0\n1\n");
    // missing split position
    same("split_pos_eof", "3 1\n3 1 2 3\n");
    same("split_pos_garbage", "3 1\n3 1 2 3\nzz\n");
    // A negative int becomes a huge size_t before the bounds check, and the
    // error message prints that huge value.
    same("split_pos_neg1", "3 1\n3 1 2 3\n-1\n");
    same("split_pos_neg_big", "3 1\n3 1 2 3\n-2147483648\n");
    same("split_pos_int_max", "3 1\n3 1 2 3\n2147483647\n");
    // full-length buffer, split at both extremes
    let full = buf_line(256, |i| i as i32);
    same("split_max_at_256", &format!("3 1\n{full}\n256\n"));
    same("split_max_at_0", &format!("3 1\n{full}\n0\n"));
    same("split_max_at_128", &format!("3 1\n{full}\n128\n"));
    // additional buffers are read but only buffers[0] is split
    same("split_extra_buffers", "3 3\n2 1 2\n1 9\n1 8\n1\n");
}

#[test]
fn op_interleave() {
    // OP_INTERLEAVE = 4.
    same("interleave_equal", "4 2\n3 1 2 3\n3 4 5 6\n");
    same("interleave_first_longer", "4 2\n3 1 2 3\n1 9\n");
    same("interleave_second_longer", "4 2\n1 9\n3 1 2 3\n");
    same("interleave_needs_two", "4 1\n1 1\n");
    same("interleave_both_empty", "4 2\n0\n0\n");
    same("interleave_one_empty", "4 2\n0\n3 1 2 3\n");
    // sum exactly 256: allowed
    same(
        "interleave_sum_256",
        &format!(
            "4 2\n{}\n{}\n",
            buf_line(200, |i| i as i32),
            buf_line(56, |i| i as i32)
        ),
    );
    // sum 257: "Error: Interleaved length exceeds maximum"
    same(
        "interleave_sum_257",
        &format!(
            "4 2\n{}\n{}\n",
            buf_line(201, |i| i as i32),
            buf_line(56, |i| i as i32)
        ),
    );
    same(
        "interleave_sum_512",
        &format!(
            "4 2\n{}\n{}\n",
            buf_line(256, |i| i as i32),
            buf_line(256, |i| i as i32)
        ),
    );
}

#[test]
fn op_rotate() {
    // OP_ROTATE = 5: reads one extra int, then rotates and prints every buffer.
    same("rotate_by_1", "5 1\n4 1 2 3 4\n1\n");
    same("rotate_by_0", "5 1\n4 1 2 3 4\n0\n");
    same("rotate_by_len", "5 1\n4 1 2 3 4\n4\n");
    same("rotate_over_len", "5 1\n4 1 2 3 4\n6\n");
    same("rotate_negative", "5 1\n4 1 2 3 4\n-1\n");
    same("rotate_negative_len", "5 1\n4 1 2 3 4\n-4\n");
    same("rotate_negative_over", "5 1\n4 1 2 3 4\n-6\n");
    same("rotate_large", "5 1\n2 1 2\n1000000\n");
    same("rotate_int_max", "5 1\n3 1 2 3\n2147483647\n");
    same("rotate_int_min", "5 1\n3 1 2 3\n-2147483648\n");
    same("rotate_int_min_len1", "5 1\n1 9\n-2147483648\n");
    // length 0 short-circuits before the modulo, so no division by zero
    same("rotate_empty_buffer", "5 1\n0\n5\n");
    same("rotate_empty_then_nonempty", "5 2\n0\n3 1 2 3\n2\n");
    // missing rotation amount
    same("rotate_amount_eof", "5 1\n3 1 2 3\n");
    same("rotate_amount_garbage", "5 1\n3 1 2 3\nxyz\n");
    same("rotate_many", "5 3\n3 1 2 3\n0\n5 1 2 3 4 5\n2\n");
    same(
        "rotate_max_len",
        &format!("5 1\n{}\n77\n", buf_line(256, |i| i as i32)),
    );
}

#[test]
fn op_checksum() {
    // OP_CHECKSUM = 6: prints the stored checksum of each buffer as %u.
    same("checksum_one", "6 1\n3 1 2 3\n");
    same("checksum_empty", "6 1\n0\n");
    same("checksum_many", "6 3\n1 7\n0\n2 1 2\n");
    // `sum = (sum << 3) ^ data[i]` overflows uint32 once length > ~11, which is
    // where the wrapping shift is observable.
    same("checksum_overflow", &format!("6 1\n{}\n", buf_line(64, |i| (i * 13) as i32 % 256)));
    same("checksum_max_len", &format!("6 1\n{}\n", buf_line(256, |i| i as i32)));
    same("checksum_all_ff", &format!("6 1\n{}\n", buf_line(256, |_| 255)));
    same("checksum_high_bit", "6 1\n12 255 255 255 255 255 255 255 255 255 255 255 255\n");
}

#[test]
fn unknown_operations() {
    // default: "Error: Unknown operation %d"
    same("op_7", "7 1\n1 1\n");
    same("op_8", "8 1\n1 1\n");
    same("op_negative", "-1 1\n1 1\n");
    same("op_negative_3", "-3 1\n1 1\n");
    same("op_99", "99 1\n1 1\n");
    same("op_int_max", "2147483647 1\n1 1\n");
    // buffers are still read (and can still fail) before the switch is reached
    same("op_unknown_but_bad_buffer", "99 1\n-1\n");
    same("op_unknown_but_short_buffer", "99 1\n3 1 2\n");
}

// ==================== Phase C: scale and stress ====================

#[test]
fn maximum_scale_inputs() {
    // 100 buffers of 256 bytes each: the largest input the program accepts,
    // and ~100 KiB of stdout, which forces multiple stdio flushes in the C.
    let mut s = String::from("1 100\n");
    for b in 0..100 {
        s.push_str(&buf_line(256, |i| ((i + b) % 256) as i32));
        s.push('\n');
    }
    same("reverse_100x256", &s);

    let mut s = String::from("6 100\n");
    for b in 0..100 {
        s.push_str(&buf_line(256, |i| ((i * 3 + b) % 256) as i32));
        s.push('\n');
    }
    same("checksum_100x256", &s);

    let mut s = String::from("5 100\n");
    for b in 0..100 {
        s.push_str(&buf_line(if b % 3 == 0 { 0 } else { 256 }, |i| i as i32));
        s.push('\n');
    }
    s.push_str("-13\n");
    same("rotate_100_mixed", &s);
}

#[test]
fn tokens_straddling_internal_read_boundaries() {
    // Long zero-padded tokens push conversions across the stdio buffer
    // boundary, exercising the refill path in the Rust scanner.
    let mut s = String::from("6 1\n256");
    for i in 0..256 {
        s.push(' ');
        s.push_str(&format!("{:0>97}", i));
    }
    s.push('\n');
    same("padded_tokens_25kb", &s);

    // A single token longer than any internal buffer.
    same("very_long_token", &format!("{}6 1\n1 5\n", "0".repeat(9000)));
}

#[test]
fn non_utf8_and_control_bytes_in_stream() {
    // The C reads bytes, not text; a stray non-ASCII byte is a matching failure.
    assert_same("invalid_utf8_op", b"\xff\xfe 1\n");
    assert_same("invalid_utf8_byte", b"6 1\n2 5 \xc3\x28\n");
    assert_same("nul_byte", b"6 1\n1 \x005\n");
    assert_same("nul_after_valid", b"6 1\n1 5\x00\n");
}

// ==================== Phase C: broad randomized differential sweep ====================

/// Tiny deterministic PRNG so the sweep is reproducible without a dev-dependency.
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u32) as usize]
    }
}

#[test]
fn randomized_structured_sweep() {
    let ops = [0i64, 1, 2, 3, 4, 5, 6, 7, -1, 99, 2147483648, 4294967296];
    let counts = [1i64, 2, 3, 5, 100, 101, 0, -1, 4294967298];
    let lens = [0i64, 1, 2, 3, 127, 128, 129, 200, 255, 256, 257, -1];
    let bytes = [
        0i64,
        1,
        127,
        128,
        255,
        256,
        -1,
        300,
        4294967296,
        -2147483648,
    ];
    let extras = [
        0i64,
        1,
        2,
        3,
        255,
        256,
        -1,
        -4,
        2147483647,
        -2147483648,
        100000000000000000000i128 as i64,
    ];

    let mut rng = Rng(0x1234_5678_9abc_def1);
    for case in 0..600 {
        let op = *rng.pick(&ops);
        let count = *rng.pick(&counts);
        let mut parts = vec![op.to_string(), count.to_string()];
        let nb = if (1..=100).contains(&count) {
            count
        } else {
            rng.below(3) as i64
        };
        for _ in 0..nb {
            let l = *rng.pick(&lens);
            parts.push(l.to_string());
            if l > 0 {
                // Keep the generated inputs small enough to stay fast.
                let n = l.min(300);
                for _ in 0..n {
                    parts.push(rng.pick(&bytes).to_string());
                }
            }
        }
        if op == 3 || op == 5 || rng.below(5) == 0 {
            parts.push(rng.pick(&extras).to_string());
        }
        let sep = if rng.below(2) == 0 { " " } else { "\n" };
        let input = format!("{}\n", parts.join(sep));
        assert_same(&format!("sweep_{case}"), input.as_bytes());
    }
}

#[test]
fn randomized_token_soup() {
    // Free-form token streams, most of which land on an error path early.
    let toks = [
        "0", "1", "2", "3", "4", "5", "6", "7", "-1", "99", "100", "101", "255", "256", "257",
        "-256", "2147483647", "-2147483648", "4294967296", "99999999999999999999", "abc", "x",
        "0x1f", "+", "-", "--3", "+7", ".", "00", "-0",
    ];
    let seps = [" ", "\n", "\t", "  ", "\r\n", "\u{0b}", "\u{0c}", "\n\n"];

    let mut rng = Rng(0x0bad_c0de_dead_beef);
    for case in 0..600 {
        let n = rng.below(14);
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push_str(rng.pick(&seps));
            }
            s.push_str(rng.pick(&toks));
        }
        s.push_str(rng.pick(&["", "\n", " "]));
        assert_same(&format!("soup_{case}"), s.as_bytes());
    }
}

// ==================== Phase C: process-level behavior ====================

#[test]
fn closed_stdout_kills_both_the_same_way() {
    // The C has stdout block-buffered to a pipe; when the reader goes away it
    // dies from SIGPIPE. Rust's runtime ignores SIGPIPE by default, so the
    // translation must restore the platform default to match the wait status.
    let mut input = String::from("1 100\n");
    for _ in 0..100 {
        input.push_str(&buf_line(256, |i| i as i32));
        input.push('\n');
    }

    let outcome = |exe: &Path| -> (Option<i32>, String) {
        // The reader below takes one byte and then drops the pipe.
        let mut writer = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut sin = writer.stdin.take().unwrap();
        let stdout = writer.stdout.take().unwrap();
        let data = input.clone();
        let handle = std::thread::spawn(move || {
            let _ = sin.write_all(data.as_bytes());
        });
        // Read a single byte, then drop the pipe.
        {
            use std::io::Read;
            let mut r = stdout;
            let mut one = [0u8; 1];
            let _ = r.read(&mut one);
        }
        let st = writer.wait().unwrap();
        let _ = handle.join();
        (st.code(), format!("{st}"))
    };

    let (cc, cs) = outcome(c_bin());
    let (rc, rs) = outcome(rust_bin());
    assert_eq!(
        cc, rc,
        "exit status on a closed stdout differs: C {cs} vs Rust {rs}"
    );
}

#[test]
fn stdin_from_dev_null() {
    // Equivalent to immediate EOF.
    let go = |exe: &Path| {
        Command::new(exe)
            .stdin(Stdio::from(std::fs::File::open("/dev/null").unwrap()))
            .output()
            .unwrap()
    };
    let c = go(c_bin());
    let r = go(rust_bin());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn stdin_delivered_in_tiny_chunks() {
    // Forces short reads so a conversion can span two refills.
    let mut input = String::from("5 3\n");
    for b in 0..3 {
        input.push_str(&buf_line(200, |i| ((i * 5 + b) % 256) as i32));
        input.push('\n');
    }
    input.push_str("-77\n");
    let bytes = input.into_bytes();

    let go = |exe: &Path| {
        let mut ch = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut sin = ch.stdin.take().unwrap();
        let data = bytes.clone();
        let h = std::thread::spawn(move || {
            for chunk in data.chunks(7) {
                if sin.write_all(chunk).is_err() {
                    return;
                }
                let _ = sin.flush();
            }
        });
        let out = ch.wait_with_output().unwrap();
        let _ = h.join();
        out
    };

    let c = go(c_bin());
    let r = go(rust_bin());
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn output_formatting_is_byte_exact() {
    // Pin the literal bytes so a formatting regression cannot hide behind a
    // matching-but-wrong pair of programs: "%zu" then " %u" per byte, "\n".
    let out = run(rust_bin(), b"1 2\n3 1 2 3\n0\n");
    assert_eq!(out.stdout, b"3 3 2 1\n0\n");
    assert_eq!(out.stderr, b"");
    assert_eq!(out.code, Some(0));

    let out = run(rust_bin(), b"6 2\n1 255\n0\n");
    assert_eq!(out.stdout, b"255\n0\n");

    let bad = run(rust_bin(), b"1 1\n257\n");
    assert_eq!(bad.stdout, b"");
    assert_eq!(bad.stderr, b"Error: Invalid buffer length 257\n");
    assert_eq!(bad.code, Some(1));

    // And confirm the C agrees with those exact bytes.
    same("format_pin_reverse", "1 2\n3 1 2 3\n0\n");
    same("format_pin_checksum", "6 2\n1 255\n0\n");
    same("format_pin_error", "1 1\n257\n");
}
