// Differential tests: run the ORIGINAL C binary and the Rust binary as
// subprocesses on identical stdin, and require byte-identical stdout,
// byte-identical stderr and an identical exit status.
//
// Nothing here links the Rust code as a library: both programs are driven
// exactly the way a shell would drive them, because that is how they are
// compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------- plumbing

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust program under test. Cargo hands integration tests the path of the
/// binary target it just built, so this always matches the current profile.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C program: ground truth. Built with cmake on first use if missing.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let root = manifest_dir()
            .parent()
            .expect("translation/ must have a parent")
            .to_path_buf();
        let src = root.join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

/// Scratch directory for the stdin files.
fn scratch() -> PathBuf {
    let d = manifest_dir().join("target").join("difftest-stdin");
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// Run `exe` with `input` on stdin.
///
/// stdin is supplied from a real file rather than a pipe we feed ourselves:
/// the largest cases push ~100 KB in and ~100 KB out, which would deadlock a
/// naive "write all of stdin, then read stdout" loop once the 64 KB pipe
/// buffer fills. `output()` drains stdout and stderr concurrently.
fn run(exe: &Path, tag: &str, input: &[u8]) -> Outcome {
    let path = scratch().join(format!("{tag}.stdin"));
    {
        let mut f = std::fs::File::create(&path).expect("create stdin file");
        f.write_all(input).expect("write stdin file");
        f.sync_all().ok();
    }
    let f = std::fs::File::open(&path).expect("open stdin file");
    let out = Command::new(exe)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) if s.len() <= 600 => s.to_string(),
        Ok(s) => format!("{}... [{} bytes total]", &s[..600], b.len()),
        Err(_) => format!("{:?}", &b[..b.len().min(600)]),
    }
}

/// The single assertion used by every case: stdout, stderr and exit status
/// must all agree.
fn assert_same(name: &str, input: &[u8]) {
    let tag: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let c = run(c_bin(), &format!("c_{tag}"), input);
    let r = run(&rust_bin(), &format!("r_{tag}"), input);

    let mut problems = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!(
            "STDOUT differs\n  C   : {}\n  Rust: {}",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "STDERR differs\n  C   : {}\n  Rust: {}",
            show(&c.stderr),
            show(&r.stderr)
        ));
    }
    if c.status != r.status {
        problems.push(format!(
            "EXIT STATUS differs\n  C   : {:?}\n  Rust: {:?}",
            c.status, r.status
        ));
    }
    assert!(
        problems.is_empty(),
        "case `{name}` diverged\ninput: {}\n{}",
        show(input),
        problems.join("\n")
    );
}

fn same(name: &str, input: &str) {
    assert_same(name, input.as_bytes());
}

// -------------------------------------------------------------- generators

fn seq_bytes(n: usize) -> String {
    (0..n)
        .map(|i| (i % 256).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `count` buffers, each of `len` bytes 0..len-1.
fn many_buffers(op: i32, count: usize, len: usize, trailing: Option<i32>) -> String {
    let mut s = format!("{op} {count}\n");
    for _ in 0..count {
        s.push_str(&format!("{len} {}\n", seq_bytes(len)));
    }
    if let Some(t) = trailing {
        s.push_str(&format!("{t}\n"));
    }
    s
}

// =====================================================================
// Phase B / C: every input class the C source branches on.
// =====================================================================

// ---- main(): reading the operation ----

#[test]
fn op_read_failures() {
    same("empty_input", "");
    same("whitespace_only", "   \n\t\n  ");
    same("newlines_only", "\n\n\n");
    same("non_numeric_op", "abc\n");
    same("lone_minus", "-");
    same("lone_plus", "+");
    same("minus_then_letter", "-x 1\n");
    same("op_is_dot", ".5 1\n");
}

// ---- main(): reading the buffer count ----

#[test]
fn count_read_failures() {
    same("op_only", "0");
    same("op_only_nl", "0\n");
    same("op_then_junk", "0 xyz\n");
    same("op_then_eof_after_ws", "6   \n");
}

#[test]
fn count_range_validation() {
    same("count_zero", "0 0\n");
    same("count_negative", "0 -5\n");
    same("count_neg_one", "3 -1\n");
    same("count_101", "0 101\n");
    same("count_100_ok", &many_buffers(6, 100, 1, None));
    same("count_1_ok", &many_buffers(6, 1, 1, None));
    // int truncation of a value that fits in long but not int
    same("count_5e9_truncates", "0 5000000000\n");
    // strtol saturation then truncation to int -> -1
    same("count_saturating", "0 99999999999999999999\n");
    same("count_int_min", "0 -2147483648\n");
    same("count_int_max", "0 2147483647\n");
}

// ---- read_buffer() ----

#[test]
fn buffer_length_validation() {
    same("len_missing_eof", "6 1");
    same("len_negative", "6 1\n-1\n");
    same("len_257", "6 1\n257 1\n");
    same("len_zero", "6 1\n0\n");
    same("len_256_max", &format!("6 1\n256 {}\n", seq_bytes(256)));
    same("len_255", &format!("6 1\n255 {}\n", seq_bytes(255)));
    same("len_int_max", "6 1\n2147483647\n");
    same("len_2147483648_truncates", "6 1\n2147483648\n");
    same("len_saturating", "6 1\n99999999999999999999\n");
    same("len_non_numeric", "6 1\nzz\n");
    // second buffer's length is the one that fails
    same("len_bad_on_second", "6 2\n1 5\n-3\n");
}

#[test]
fn buffer_byte_read_failures() {
    same("byte_missing_first", "6 1\n3\n");
    same("byte_missing_middle", "6 1\n3 1 2\n");
    same("byte_non_numeric", "6 1\n2 5 zz\n");
    same("byte_hex_prefix_stops", "6 1\n2 0x10 5\n");
    same("byte_lone_minus_mid", "6 1\n2 - 5 7\n");
    same("byte_fail_on_second_buffer", "6 2\n1 5\n4 1 2\n");
    same("byte_fail_index_reported", "6 1\n5 1 2 3 4\n");
}

#[test]
fn byte_truncation_to_uint8() {
    // (uint8_t)byte truncation, including negatives
    same("bytes_over_255", "6 1\n3 256 300 -1\n");
    same("bytes_int_max", "6 1\n2 2147483647 2147483648\n");
    same("bytes_negative", "6 1\n4 -1 -128 -255 -256\n");
    same("bytes_saturating", "6 1\n2 99999999999999999999 -99999999999999999999\n");
    same("bytes_exactly_255_256", "6 1\n2 255 256\n");
    // visible through the data, not just the checksum
    same("reverse_truncated_bytes", "1 1\n4 256 511 -1 130\n");
}

// ---- OP_COPY (0) ----

#[test]
fn op_copy() {
    same("copy_needs_two_count1", "0 1\n3 1 2 3\n");
    same("copy_ok", "0 2\n3 1 2 3\n2 9 9\n");
    same("copy_src_empty", "0 2\n0\n1 1\n");
    same(
        "copy_src_max",
        &format!("0 2\n256 {}\n1 1\n", seq_bytes(256)),
    );
    // only buffers[0] is copied; the rest merely have to parse
    same("copy_three_buffers", "0 3\n2 7 8\n1 1\n0\n");
}

// ---- OP_REVERSE (1) ----

#[test]
fn op_reverse() {
    same("reverse_single", "1 1\n3 1 2 3\n");
    same("reverse_empty_only", "1 1\n0\n");
    same("reverse_len1", "1 1\n1 42\n");
    same("reverse_even", "1 1\n4 1 2 3 4\n");
    same("reverse_mixed_lengths", "1 3\n3 1 2 3\n0\n1 9\n");
    same("reverse_max", &format!("1 1\n256 {}\n", seq_bytes(256)));
    same("reverse_100x256_stress", &many_buffers(1, 100, 256, None));
    same("reverse_100x0", &many_buffers(1, 100, 0, None));
}

// ---- OP_MERGE (2) ----

#[test]
fn op_merge() {
    same("merge_needs_two_count1", "2 1\n3 1 2 3\n");
    same("merge_ok", "2 2\n3 1 2 3\n2 9 9\n");
    same("merge_both_empty", "2 2\n0\n0\n");
    same("merge_first_empty", "2 2\n0\n3 1 2 3\n");
    same("merge_second_empty", "2 2\n3 1 2 3\n0\n");
    same(
        "merge_exactly_256",
        &format!("2 2\n128 {}\n128 {}\n", seq_bytes(128), seq_bytes(128)),
    );
    same(
        "merge_257_overflow",
        &format!("2 2\n256 {}\n1 7\n", seq_bytes(256)),
    );
    same(
        "merge_512_overflow",
        &format!("2 2\n256 {}\n256 {}\n", seq_bytes(256), seq_bytes(256)),
    );
    // only the first two are merged
    same("merge_three_buffers", "2 3\n1 1\n1 2\n1 3\n");
}

// ---- OP_SPLIT (3) ----

#[test]
fn op_split() {
    same("split_pos_missing", "3 1\n2 1 2\n");
    same("split_pos_non_numeric", "3 1\n2 1 2\nzz\n");
    same("split_pos_zero", "3 1\n3 1 2 3\n0\n");
    same("split_pos_middle", "3 1\n4 1 2 3 4\n2\n");
    same("split_pos_eq_len", "3 1\n3 1 2 3\n3\n");
    same("split_pos_gt_len", "3 1\n3 1 2 3\n9\n");
    // int -> size_t sign extension makes negatives enormous
    same("split_pos_neg_one", "3 1\n4 1 2 3 4\n-1\n");
    same("split_pos_int_min", "3 1\n4 1 2 3 4\n-2147483648\n");
    same("split_pos_saturating", "3 1\n4 1 2 3 4\n99999999999999999999\n");
    same("split_empty_src_pos_zero", "3 1\n0\n0\n");
    same("split_empty_src_pos_one", "3 1\n0\n1\n");
    same(
        "split_max_buffer",
        &format!("3 1\n256 {}\n128\n", seq_bytes(256)),
    );
    same("split_ignores_later_buffers", "3 2\n2 1 2\n2 3 4\n1\n");
}

// ---- OP_INTERLEAVE (4) ----

#[test]
fn op_interleave() {
    same("interleave_needs_two_count1", "4 1\n3 1 2 3\n");
    same("interleave_uneven", "4 2\n3 1 2 3\n2 9 9\n");
    same("interleave_equal", "4 2\n3 1 2 3\n3 7 8 9\n");
    same("interleave_second_longer", "4 2\n1 1\n4 5 6 7 8\n");
    same("interleave_both_empty", "4 2\n0\n0\n");
    same("interleave_first_empty", "4 2\n0\n3 1 2 3\n");
    same("interleave_second_empty", "4 2\n3 1 2 3\n0\n");
    same(
        "interleave_exactly_256",
        &format!("4 2\n128 {}\n128 {}\n", seq_bytes(128), seq_bytes(128)),
    );
    same(
        "interleave_257_overflow",
        &format!("4 2\n200 {}\n57 {}\n", seq_bytes(200), seq_bytes(57)),
    );
    same(
        "interleave_512_overflow",
        &format!("4 2\n256 {}\n256 {}\n", seq_bytes(256), seq_bytes(256)),
    );
}

// ---- OP_ROTATE (5) ----

#[test]
fn op_rotate() {
    same("rotate_amount_missing", "5 1\n2 1 2\n");
    same("rotate_amount_non_numeric", "5 1\n2 1 2\nzz\n");
    same("rotate_zero", "5 1\n4 1 2 3 4\n0\n");
    same("rotate_one", "5 1\n4 1 2 3 4\n1\n");
    same("rotate_eq_len", "5 1\n4 1 2 3 4\n4\n");
    same("rotate_gt_len", "5 1\n4 1 2 3 4\n7\n");
    same("rotate_negative", "5 1\n5 1 2 3 4 5\n-2\n");
    same("rotate_negative_gt_len", "5 1\n3 1 2 3\n-7\n");
    same("rotate_len_zero", "5 1\n0\n5\n");
    same("rotate_len_zero_amount_zero", "5 1\n0\n0\n");
    same("rotate_len_one", "5 1\n1 42\n7\n");
    same("rotate_int_min", "5 1\n3 1 2 3\n-2147483648\n");
    same("rotate_int_max", "5 1\n3 1 2 3\n2147483647\n");
    same("rotate_saturating", "5 1\n3 1 2 3\n99999999999999999999\n");
    same("rotate_neg_saturating", "5 1\n3 1 2 3\n-99999999999999999999\n");
    same("rotate_mixed_lengths", "5 3\n4 1 2 3 4\n0\n1 9\n3\n");
    same("rotate_max_buffer", &format!("5 1\n256 {}\n77\n", seq_bytes(256)));
    same("rotate_100x256_stress", &many_buffers(5, 100, 256, Some(77)));
    same("rotate_100x0", &many_buffers(5, 100, 0, Some(3)));
}

// ---- OP_CHECKSUM (6) ----

#[test]
fn op_checksum() {
    same("checksum_single", "6 1\n3 1 2 3\n");
    same("checksum_empty", "6 1\n0\n");
    same("checksum_multi", "6 3\n3 1 2 3\n0\n1 255\n");
    same("checksum_max_buffer", &format!("6 1\n256 {}\n", seq_bytes(256)));
    same("checksum_100x256", &many_buffers(6, 100, 256, None));
    // exercises the (sum << 3) wraparound over many bytes
    same("checksum_high_bytes", "6 1\n12 255 255 255 255 255 255 255 255 255 255 255 255\n");
}

// ---- default: unknown operation ----

#[test]
fn unknown_operations() {
    same("op_7", "7 1\n1 5\n");
    same("op_8", "8 1\n1 5\n");
    same("op_negative", "-1 1\n1 5\n");
    same("op_int_min", "-2147483648 1\n1 5\n");
    same("op_int_max", "2147483647 1\n1 5\n");
    same("op_saturating_becomes_neg1", "99999999999999999999 1\n1 5\n");
    same("op_5e9_truncates", "5000000000 1\n1 5\n");
    // the buffers are still fully read (and can still fail) before the
    // unknown-operation check is reached
    same("op_unknown_but_bad_buffer", "7 1\n3 1 2\n");
    same("op_unknown_but_bad_count", "7 0\n");
}

// ---- scanf tokenisation details ----

#[test]
fn scanf_whitespace_and_sign_handling() {
    // scanf("%d") reads straight across newlines
    same("all_on_one_line", "6 2 3 1 2 3 0\n");
    same("one_token_per_line", "6\n2\n3\n1\n2\n3\n0\n");
    same("explicit_plus_signs", "+6 +1\n+2 +5 +7\n");
    same("leading_zeros", "0006 0001\n0002 0005 0007\n");
    same("tabs_and_crlf", "6\t1\r\n2\t\t5\r\n7\n");
    same("vtab_and_formfeed", "6\u{b}1\u{c}2 5 7");
    same("leading_whitespace", "\n\n\t  6 1\n1 5\n");
    same("no_trailing_newline", "6 1\n2 5 7");
    same("extra_trailing_tokens", "6 1\n2 5 7\n99 99 99\n");
    same("huge_leading_whitespace", &format!("{}6 1\n1 5\n", " ".repeat(9000)));
    same("many_leading_newlines", &format!("{}6 1\n1 5\n", "\n".repeat(5000)));
    same("trailing_garbage_after_success", &format!("6 1\n1 5\n{}", "x".repeat(9000)));
    same("very_long_digit_run", &format!("6 1\n1 {}\n", "9".repeat(400)));
    same("very_long_negative_digit_run", &format!("6 1\n1 -{}\n", "9".repeat(400)));
    same("padded_zeros_op", &format!("6 {}1\n1 5\n", "0".repeat(50)));
}

/// glibc's `%d` converts via `strtol` (saturating at LONG_MAX / LONG_MIN) and
/// then *stores* the result truncated to `int`. These inputs sit exactly on
/// that two-step boundary, where naive parsing gives a different answer.
#[test]
fn scanf_long_boundary_saturation() {
    same("op_long_max", "9223372036854775807 1\n1 5\n");
    same("op_long_max_plus_1", "9223372036854775808 1\n1 5\n");
    // LONG_MIN truncates to int 0, i.e. OP_COPY
    same("op_long_min", "-9223372036854775808 1\n1 5\n");
    same("op_long_min_minus_1", "-9223372036854775809 1\n1 5\n");
    same("op_ulong_max", "18446744073709551615 1\n1 5\n");
    same("op_two_pow_64", "18446744073709551616 1\n1 5\n");
    same("op_two_pow_32", "4294967296 1\n1 5\n");
    same("op_400_zeros", &format!("{} 1\n1 5\n", "0".repeat(400)));
    // digits then a '-' is a token break, not a sign
    same("zeros_then_minus_digit", "000000000-9 1\n1 5\n");
    same("count_4294967290", "6 4294967290\n");
    same("len_long_max", "6 1\n9223372036854775807\n");
    same("byte_long_min", "6 1\n1 -9223372036854775808\n");
    same("split_long_max", "3 1\n2 1 2\n9223372036854775807\n");
    // truncates to 0, so this is a valid split at position 0
    same("split_long_min", "3 1\n2 1 2\n-9223372036854775808\n");
    same("rotate_long_max", "5 1\n3 1 2 3\n9223372036854775807\n");
    same("rotate_long_min", "5 1\n3 1 2 3\n-9223372036854775808\n");
}

#[test]
fn non_utf8_and_nul_input() {
    assert_same("nul_bytes_leading", b"\x00\x01\x02");
    assert_same("nul_inside_number", b"6 1\n2 5\x00 7");
    assert_same("high_bytes", b"\xff\xfe\xfd");
    assert_same("nul_between_tokens", b"6\x001\n1 5");
    assert_same("utf8_sequence_as_byte", b"6 1\n1 \xc3\xa9");
    assert_same("op_then_high_byte", b"6 1\n1 \x80\n");
}

// =====================================================================
// Deterministic pseudo-random sweep: fixed seed, so it is reproducible.
// =====================================================================

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
}

const ODD_TOKENS: &[&str] = &[
    "0", "1", "-1", "2", "3", "4", "5", "6", "7", "8", "-2147483648", "2147483647",
    "2147483648", "4294967296", "99999999999999999999", "-99999999999999999999", "256",
    "257", "255", "100", "101", "-5", "+3", "0007", "abc", "x", "0x10", "-", "+", "1e5",
    "999999999999", "",
];

const SPACERS: &[&str] = &[" ", "\n", "\t", "  ", "\n\n", "\r\n", " \t "];

/// Structured-but-varied inputs: well-shaped often enough to reach the
/// operation bodies, malformed often enough to reach the error paths.
#[test]
fn sweep_structured_inputs() {
    let mut rng = Lcg(0x5eed_1234_abcd_0001);
    for case in 0..900u32 {
        let op = rng.pick(&[0i64, 1, 2, 3, 4, 5, 6, 7, -1, 8, 2, 3, 5]);
        let count = rng.pick(&[1i64, 2, 2, 3, 4, 1, 5]);
        let mut s = format!("{op}{}{count}", rng.pick(SPACERS));
        for _ in 0..count {
            let len = rng.pick(&[0i64, 1, 2, 3, 4, 128, 129, 256, 7, 2, 3]);
            s.push_str(rng.pick(SPACERS));
            s.push_str(&len.to_string());
            for _ in 0..len {
                s.push_str(rng.pick(SPACERS));
                let b = rng.pick(&[0i64, 1, 255, 256, -1, 42, 128, 300, -255, 7]);
                s.push_str(&b.to_string());
            }
        }
        if rng.below(10) < 9 {
            s.push_str(rng.pick(SPACERS));
            let t = rng.pick(&[
                0i64,
                1,
                -1,
                2,
                3,
                128,
                256,
                -256,
                2147483647,
                -2147483648,
                77,
            ]);
            s.push_str(&t.to_string());
        }
        s.push('\n');
        assert_same(&format!("sweep_structured_{case}"), s.as_bytes());
    }
}

/// Token soup: mostly reaches the parse/validation error paths.
#[test]
fn sweep_token_soup() {
    let mut rng = Lcg(0xfeed_face_0000_0007);
    for case in 0..900u32 {
        let n = rng.below(14);
        let mut s = String::new();
        for _ in 0..n {
            if rng.below(4) == 0 {
                s.push_str(rng.pick(ODD_TOKENS));
            } else {
                let v = rng.below(600) as i64 - 300;
                s.push_str(&v.to_string());
            }
            s.push_str(rng.pick(SPACERS));
        }
        assert_same(&format!("sweep_soup_{case}"), s.as_bytes());
    }
}

/// Raw random bytes: nothing here should be parsed as UTF-8 by either side.
#[test]
fn sweep_random_bytes() {
    let mut rng = Lcg(0x0bad_c0de_1111_2222);
    for case in 0..400u32 {
        let n = rng.below(70) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            // bias towards characters that can start a number
            v.push(if rng.below(3) == 0 {
                rng.below(256) as u8
            } else {
                rng.pick(&b"0123456789 \n\t-+"[..])
            });
        }
        assert_same(&format!("sweep_bytes_{case}"), &v);
    }
}
