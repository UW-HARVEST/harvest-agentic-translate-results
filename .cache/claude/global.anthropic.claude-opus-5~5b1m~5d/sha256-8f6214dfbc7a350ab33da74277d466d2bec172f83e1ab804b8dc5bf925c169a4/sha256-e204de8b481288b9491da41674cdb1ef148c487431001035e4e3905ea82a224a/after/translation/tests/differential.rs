//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on the same stdin and require byte-identical stdout, byte
//! identical stderr and the identical exit status (including death by signal).
//!
//! The Rust program is never used as a library here; it is only ever driven
//! through its compiled binary, exactly like a shell would.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating / building the two programs
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Path of the C executable, built with the documented
/// `cmake .. && cmake --build .` recipe on first use.
fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cmake = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cmake.status.success(),
                "cmake failed:\n{}\n{}",
                String::from_utf8_lossy(&cmake.stdout),
                String::from_utf8_lossy(&cmake.stderr)
            );
            let make = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                make.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&make.stdout),
                String::from_utf8_lossy(&make.stderr)
            );
        }
        assert!(exe.exists(), "C executable missing at {}", exe.display());
        exe
    })
    .as_path()
}

/// Path of the Rust executable under test (built by cargo for this test).
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// Running them
// ---------------------------------------------------------------------------

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

fn run(exe: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(input)
        .or_else(|e| {
            // The C program can die (SIGSEGV/SIGFPE) before draining stdin.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("write stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait for child");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Assert that both programs behave identically on `input`.
fn check(input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input {:?}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input {:?}\n  C: {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "exit status differs for input {:?}\n  C: {c:?}\n  Rust: {r:?}",
        String::from_utf8_lossy(input)
    );
}

fn check_all(cases: &[(&str, &str)]) {
    for (name, input) in cases {
        let c = run(c_binary(), input.as_bytes());
        let r = run(rust_binary(), input.as_bytes());
        assert_eq!(c, r, "case `{name}` differs for input {input:?}");
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` returns early on a matching/input failure, leaving the remaining
/// arguments at their initialisers; every prefix length is an input class.
#[test]
fn input_shapes_and_scanf_failures() {
    check_all(&[
        ("empty", ""),
        ("only_newline", "\n"),
        ("only_whitespace", "   \t\n  "),
        ("one_int", "3"),
        ("one_int_newline", "3\n"),
        ("two_fields", "0 1.5"),
        ("three_fields", "1 1.5 2.5"),
        ("four_fields", "1 1.5 2.5 3.5"),
        ("five_fields", "1 1.5 2.5 3.5 4"),
        ("six_fields", "1 1.5 2.5 3.5 4 8"),
        ("seven_fields", "1 1.5 2.5 3.5 4 8 16"),
        ("eight_fields", "1 1.5 2.5 3.5 4 8 16 200"),
        ("nine_fields", "2 1 2 3 4 5 6 7 2.0"),
        ("ten_fields", "2 1 2 3 4 5 6 7 2.0 0.5"),
        ("eleven_fields", "2 1 2 3 4 5 6 7 2.0 0.5 1.0"),
        ("twelve_fields", "0 0.25 0.5 0.75 0 0 0 0 2 0.5 1 6"),
        ("trailing_junk", "0 0.25 0.5 0.75 0 0 0 0 2 0.5 1 6 extra 99"),
        (
            "newline_separated",
            "0\n0.25\n0.5\n0.75\n0\n0\n0\n0\n2\n0.5\n1\n6\n",
        ),
        ("tabs_and_crs", "0\t0.25\r\n0.5 0.75 0 0 0 0 2 0.5 1 6"),
        ("vertical_tab_formfeed", "5\t\t\t3.5\u{b}4.5\u{c}5.5 3 5 7 9 0 0 0 0"),
        ("leading_whitespace", "\n\n\t   1 0.5 0.5 0.5 0 0 0 7 0 0 0 0"),
        ("trailing_newlines", "0 1 2 3 0 0 0 0 0 0 0 0\n\n\n"),
        (
            "crlf_separated",
            "\r\n5\r\n1\r\n2\r\n3\r\n1\r\n1\r\n1\r\n0\r\n0\r\n0\r\n0",
        ),
        // %d matching failures
        ("first_not_a_number", "abc 1 2 3"),
        ("first_plus_only", "+ 1 2 3"),
        ("first_minus_only", "- 1 2 3"),
        ("first_dot", ". 1 2 3"),
        ("lone_minus", "-"),
        ("lone_plus", "+"),
        ("lone_dot", "."),
        ("int_plus_sign", "+3 0.5 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("int_minus_zero", "-0 0.5 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("int_leading_zeros", "003 0.5 0.5 0.5 0 0 0 0 2 0.5 1 3"),
        ("int_hex_like_stops_at_x", "0x3 0.5 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("int_float_like_stops_at_dot", "1.5 0.5 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("int_double_minus", "--3 0.5 0.5 0.5 0 0 0 0 0 0 0 0"),
        // %d ranges
        ("int_max", "0 0.5 0.5 0.5 2147483647 0 0 0 0 0 0 0"),
        ("int_min", "0 0.5 0.5 0.5 -2147483648 0 0 0 0 0 0 0"),
        ("int_wraps_2p31", "0 0.5 0.5 0.5 2147483648 0 0 0 0 0 0 0"),
        ("int_wraps_2p32", "0 1 2 3 4294967296 4294967297 4294967298 0 0 0 0 0"),
        (
            "int_saturates_long",
            "0 0.5 0.5 0.5 9223372036854775808 0 0 0 0 0 0 0",
        ),
        (
            "int_saturates_huge",
            "0 0.5 0.5 0.5 99999999999999999999999 0 0 0 0 0 0 0",
        ),
        (
            "int_saturates_huge_negative",
            "0 0.5 0.5 0.5 -99999999999999999999999 0 0 0 0 0 0 0",
        ),
        ("which_from_wrapped_int", "4294967296 0.5 0.5 0.5 0 0 0 0 2 0.5 1 3"),
        ("which_huge_positive", "999999999999999999999999 1 2 3 0 0 0 0 0 0 0 0"),
        ("which_huge_negative", "-999999999999999999999999 1 2 3 0 0 0 0 0 0 0 0"),
    ]);
}

/// `%f` conversions: glibc's accepted spellings, its failure cases, and how
/// much input each of them consumes.
#[test]
fn float_conversions() {
    check_all(&[
        ("stops_at_letter", "0 1.5x 2 3 0 0 0 0 0 0 0 0"),
        ("bare_dot", "0 . 2 3 0 0 0 0 0 0 0 0"),
        ("leading_dot", "0 .5 -.25 +.75 0 0 0 0 0 0 0 0"),
        ("trailing_dot", "0 1. 2. 3. 0 0 0 0 0 0 0 0"),
        ("zero_dot_zero", "0 00.0 2 3 0 0 0 0 0 0 0 0"),
        ("exp_without_digits", "0 1e 2 3 0 0 0 0 0 0 0 0"),
        ("exp_sign_without_digits", "0 1e+ 2 3 0 0 0 0 0 0 0 0"),
        ("exp_minus_without_digits", "0 1e- 2 3 0 0 0 0 0 0 0 0"),
        ("exp_ok", "0 1e2 2e-1 3E+0 0 0 0 0 0 0 0 0"),
        ("exp_at_eof", "0 1.5e"),
        ("exp_sign_at_eof", "0 1.5e-"),
        ("dot_after_exp", "0 5.e3 2 3 0 0 0 0 0 0 0 0"),
        ("hex_float", "0 0x1.8p1 0x2p0 0x0.8p+2 0 0 0 0 0 0 0 0"),
        ("hex_no_digits", "0 0x 2 3 0 0 0 0 0 0 0 0"),
        ("hex_no_digits_at_eof", "0 0x"),
        ("hex_upper", "0 0X1P4 2 3 0 0 0 0 0 0 0 0"),
        ("hex_no_exp_digits", "0 0x1p 2 3 0 0 0 0 0 0 0 0"),
        ("hex_exp_sign_no_digits", "0 0x1p+ 2 3 0 0 0 0 0 0 0 0"),
        ("hex_fraction_only", "0 0x.8p1 2 3 0 0 0 0 0 0 0 0"),
        ("hex_dot_p_no_digits", "0 0x.p 2 3 0 0 0 0 0 0 0 0"),
        ("hex_big_exponent", "0 0x1p200 2 3 0 0 0 0 0 0 0 0"),
        ("hex_small_exponent", "0 0x1p-200 2 3 0 0 0 0 0 0 0 0"),
        ("inf", "0 inf 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("neg_inf", "0 -inf 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("infinity", "0 infinity 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("infinity_mixed_case", "0 iNfInItY 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("infi_partial", "0 infi 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("infin_partial", "0 infin 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("infinit_partial", "0 infinit 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("inf_at_eof", "0 inf"),
        ("in_partial", "0 in"),
        ("nan", "0 nan 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("neg_nan", "0 -nan 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("nan_uppercase", "0 NAN 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("nan_payload", "0 nan(x) 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("nan_trailing_letter", "0 nanq 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("na_partial", "0 na 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("double_overflow", "0 1e400 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("double_overflow_negative", "0 -1e400 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("double_underflow", "0 1e-400 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("float_overflow", "0 1e39 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("float_max", "0 3.4028235e38 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("float_denormal", "0 1e-45 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("float_smallest_normal", "0 1.1754944e-38 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("float_rounds_to_even", "0 1.00000001 2 3 0 0 0 0 0 0 0 0"),
        ("float_eps_boundary", "0 0.000000059604645 0.5 0.5 0 0 0 0 0 0 0 0"),
    ]);
}

/// The `switch (which)` in `inner`, including the `default: return NAN;` arm.
#[test]
fn which_dispatch() {
    check_all(&[
        ("which0_noise3", "0 3.25 -1.75 0.125 0 0 0 0 0 0 0 0"),
        ("which1_noise3_seed", "1 3.25 -1.75 0.125 0 0 0 200 0 0 0 0"),
        ("which2_ridge", "2 3.25 -1.75 0.125 0 0 0 0 2 0.5 1 6"),
        ("which3_fbm", "3 3.25 -1.75 0.125 0 0 0 0 2 0.5 0 6"),
        ("which4_turbulence", "4 3.25 -1.75 0.125 0 0 0 0 2 0.5 0 6"),
        ("which5_nonpow2", "5 3.25 -1.75 0.125 0 0 0 0 0 0 0 0"),
        ("which6_default_nan", "6 3.25 -1.75 0.125 0 0 0 0 2 0.5 1 6"),
        ("which7_default_nan", "7 1 2 3 0 0 0 0 0 0 0 0"),
        ("which_negative_default", "-1 3.25 -1.75 0.125 0 0 0 0 2 0.5 1 6"),
        ("which_big_default", "1000000 1 2 3 0 0 0 0 0 0 0 0"),
        ("which_int_min_default", "-2147483648 1 2 3 0 0 0 0 0 0 0 0"),
        ("which_int_max_default", "2147483647 1 2 3 0 0 0 0 0 0 0 0"),
    ]);
}

/// `stb_perlin_noise3` / `..._seed`: the wrap masks, the `fastfloor` branch and
/// the seed truncation to `unsigned char`.
#[test]
fn noise3_and_seeded_noise3() {
    check_all(&[
        ("integer_coords", "0 1 2 3 0 0 0 0 0 0 0 0"),
        ("fractional_coords", "0 0.25 0.5 0.75 0 0 0 0 0 0 0 0"),
        ("negative_coords", "0 -1.5 -2.5 -3.5 0 0 0 0 0 0 0 0"),
        ("negative_integer_coords", "0 -1 -2 -3 0 0 0 0 0 0 0 0"),
        ("zero_coords", "0 0 0 0 0 0 0 0 0 0 0 0"),
        ("negative_zero_coords", "0 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0"),
        ("wrap_pow2", "0 3.5 4.5 5.5 4 8 16 0 0 0 0 0"),
        ("wrap_one", "0 3.5 4.5 5.5 1 1 1 0 0 0 0 0"),
        ("wrap_two", "0 3.5 4.5 5.5 2 2 2 0 0 0 0 0"),
        ("wrap_256", "0 3.5 4.5 5.5 256 256 256 0 0 0 0 0"),
        ("wrap_512", "0 3.5 4.5 5.5 512 512 512 0 0 0 0 0"),
        ("wrap_nonpow2", "0 3.5 4.5 5.5 3 5 7 0 0 0 0 0"),
        ("wrap_negative", "0 3.5 4.5 5.5 -1 -2 -3 0 0 0 0 0"),
        ("wrap_int_min", "0 3.5 4.5 5.5 -2147483648 -2147483648 -2147483648 0 0 0 0 0"),
        ("wrap_int_max", "0 3.5 4.5 5.5 2147483647 2147483647 2147483647 0 0 0 0 0"),
        ("seed_0", "1 3.5 4.5 5.5 0 0 0 0 0 0 0 0"),
        ("seed_1", "1 3.5 4.5 5.5 0 0 0 1 0 0 0 0"),
        ("seed_255", "1 3.5 4.5 5.5 0 0 0 255 0 0 0 0"),
        ("seed_256_truncates", "1 3.5 4.5 5.5 0 0 0 256 0 0 0 0"),
        ("seed_257_truncates", "1 3.5 4.5 5.5 0 0 0 257 0 0 0 0"),
        ("seed_negative_truncates", "1 3.5 4.5 5.5 0 0 0 -1 0 0 0 0"),
        ("seed_int_min_truncates", "1 3.5 4.5 5.5 0 0 0 -2147483648 0 0 0 0"),
        ("seed_and_wrap", "1 300.5 -400.25 500.75 16 32 64 137 0 0 0 0"),
        // fastfloor: (int) cast is out of range / NaN => INT_MIN on x86-64
        ("coord_nan", "0 nan 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("coord_nan_all", "0 nan nan nan 0 0 0 0 0 0 0 0"),
        ("coord_neg_nan", "0 -nan 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("coord_inf", "0 inf 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("coord_inf_all", "0 inf inf inf 0 0 0 0 0 0 0 0"),
        ("coord_neg_inf_all", "0 -inf -inf -inf 0 0 0 0 0 0 0 0"),
        ("coord_huge", "0 1e30 -1e30 1e30 0 0 0 0 0 0 0 0"),
        ("coord_2p31", "0 2147483648 -2147483648 2147483647 0 0 0 0 0 0 0 0"),
        ("coord_below_2p31", "0 2147483520 0.5 0.5 0 0 0 0 0 0 0 0"),
        ("coord_2p24", "0 16777216 16777217 -16777216 0 0 0 0 0 0 0 0"),
        ("coord_denormal", "0 1e-45 -1e-45 1e-45 0 0 0 0 0 0 0 0"),
        // results that exercise printf("%.9g") corner cases
        ("prints_negative_zero", "1 -0.0 -0.0 0 0 0 0 3 0 0 0 0"),
        ("prints_negative_zero2", "1 -0.0 -0.0 -0.0 0 0 0 14 0 0 0 0"),
        ("prints_zero", "0 1 2 3 0 0 0 0 0 0 0 0"),
    ]);
}

/// The three fractal loops: `octaves <= 0` skips the loop entirely, and the
/// `(unsigned char) i` seed wraps every 256 iterations.
#[test]
fn fractal_noise_loops() {
    check_all(&[
        ("ridge_octaves_0", "2 1.5 2.5 3.5 2 0.5 1 0"),
        ("ridge_octaves_1", "2 1.5 2.5 3.5 2 0.5 1 1"),
        ("ridge_octaves_2", "2 1.5 2.5 3.5 2 0.5 1 2"),
        ("ridge_octaves_6", "2 1.5 2.5 3.5 2 0.5 1 6"),
        ("ridge_octaves_negative", "2 1.5 2.5 3.5 2 0.5 1 -3"),
        ("ridge_octaves_255", "2 1.5 2.5 3.5 2 0.5 1 255"),
        ("ridge_octaves_256_seed_wraps", "2 1.5 2.5 3.5 2 0.5 1 256"),
        ("ridge_octaves_257_seed_wraps", "2 1.5 2.5 3.5 2 0.5 1 257"),
        ("ridge_lacunarity_zero", "2 1.5 2.5 3.5 0 0.5 1 6"),
        ("ridge_lacunarity_negative", "2 1.5 2.5 3.5 -2 0.5 1 6"),
        ("ridge_gain_zero", "2 1.5 2.5 3.5 2 0 1 6"),
        ("ridge_offset_zero", "2 1.5 2.5 3.5 2 0.5 0 6"),
        ("ridge_offset_negative", "2 1.5 2.5 3.5 2 0.5 -1 6"),
        ("ridge_nan_lacunarity", "2 1.5 2.5 3.5 nan 0.5 1 6"),
        ("ridge_inf_lacunarity", "2 1.5 2.5 3.5 inf 0.5 1 6"),
        ("ridge_inf_gain", "2 1.5 2.5 3.5 2 inf 1 6"),
        ("ridge_inf_offset", "2 1.5 2.5 3.5 2 0.5 inf 6"),
        ("ridge_overflowing_gain", "2 1.5 2.5 3.5 2 1e20 1 20"),
        ("ridge_denormal_params", "2 0.5 0.5 0.5 1e-40 1e-40 1e-40 5"),
        ("ridge_missing_params", "2 1.5 2.5 3.5"),
        ("ridge_nan_coord", "2 nan 2.5 3.5 2 0.5 1 6"),
        ("fbm_octaves_0", "3 1.5 2.5 3.5 2 0.5 0 0"),
        ("fbm_octaves_1", "3 1.5 2.5 3.5 2 0.5 0 1"),
        ("fbm_octaves_negative", "3 1.5 2.5 3.5 2 0.5 0 -1"),
        ("fbm_octaves_64", "3 1.5 2.5 3.5 2 0.5 0 64"),
        ("fbm_octaves_260", "3 1.5 2.5 3.5 2 0.5 0 260"),
        ("fbm_gain_gt_one", "3 1.5 2.5 3.5 2 4 0 40"),
        ("fbm_gain_inf", "3 1.5 2.5 3.5 2 inf 0 6"),
        ("fbm_lacunarity_nan", "3 1.5 2.5 3.5 nan 0.5 0 6"),
        ("fbm_lacunarity_zero", "3 1.5 2.5 3.5 0 0.5 0 6"),
        ("fbm_lacunarity_negative", "3 1.5 2.5 3.5 -2 0.5 0 6"),
        ("fbm_huge_frequency", "3 1 1 1 1e20 1e20 0 5"),
        ("fbm_octaves_100", "3 0.5 0.5 0.5 2 0.5 0 100"),
        ("turb_octaves_0", "4 1.5 2.5 3.5 2 0.5 0 0"),
        ("turb_octaves_1", "4 1.5 2.5 3.5 2 0.5 0 1"),
        ("turb_octaves_negative", "4 1.5 2.5 3.5 2 0.5 0 -7"),
        ("turb_octaves_300", "4 1.5 2.5 3.5 2 0.5 0 300"),
        ("turb_gain_gt_one", "4 1.5 2.5 3.5 2 8 0 40"),
        ("turb_nan_coord", "4 nan 2.5 3.5 2 0.5 0 6"),
        ("turb_inf_coord", "4 inf 2.5 3.5 2 0.5 0 6"),
        ("turb_denormals", "4 1e-40 1e-40 1e-40 1e-40 1e-40 0 5"),
    ]);
}

/// `stb_perlin_noise3_wrap_nonpow2`: the `wrap ? wrap : 256` fallbacks, the
/// `x0 < 0` fixups, and the out-of-bounds table indices its `%` can produce.
#[test]
fn nonpow2_wrap() {
    check_all(&[
        ("default_wraps", "5 3.5 4.5 5.5 0 0 0 0 0 0 0 0"),
        ("wrap_1", "5 3.5 4.5 5.5 1 1 1 0 0 0 0 0"),
        ("wrap_3_5_7", "5 3.5 4.5 5.5 3 5 7 0 0 0 0 0"),
        ("wrap_255", "5 300.5 400.5 500.5 255 255 255 0 0 0 0 0"),
        ("wrap_256", "5 3.5 4.5 5.5 256 256 256 0 0 0 0 0"),
        ("negative_coords", "5 -3.5 -4.5 -5.5 3 5 7 0 0 0 0 0"),
        ("negative_coords_default_wrap", "5 -3.5 -4.5 -5.5 0 0 0 0 0 0 0 0"),
        ("negative_zero_coords", "5 -0.0 -0.0 -0.0 0 0 0 0 0 0 0 0"),
        ("seed_255", "5 3.5 4.5 5.5 3 5 7 255 0 0 0 0"),
        ("seed_256_truncates", "5 3.5 4.5 5.5 3 5 7 256 0 0 0 0"),
        ("seed_negative_truncates", "5 3.5 4.5 5.5 3 5 7 -1 0 0 0 0"),
        ("wrap_1000", "5 300.5 0.5 0.5 1000 1000 1000 0 0 0 0 0"),
        ("wrap_512_big_coords", "5 600.5 700.5 800.5 512 512 512 0 0 0 0 0"),
        ("wrap_neg1", "5 3.5 4.5 5.5 -1 -1 -1 0 0 0 0 0"),
        ("wrap_neg2", "5 3.5 4.5 5.5 -2 -2 -2 0 0 0 0 0"),
        ("wrap_int_max", "5 3.5 4.5 5.5 2147483647 2147483647 2147483647 0 0 0 0 0"),
        ("wrap_int_min", "5 3.5 4.5 5.5 -2147483648 -2147483648 -2147483648 0 0 0 0 0"),
        // reads past the end of the tables, still inside the mapped pages
        ("oob_above_x_600", "5 600.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("oob_above_x_1000", "5 1000.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("oob_above_x_2000", "5 2000.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("oob_above_x_4030", "5 4030.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("oob_above_y_600", "5 0.5 600.5 0.5 1 2147483647 1 0 0 0 0 0"),
        ("oob_above_z_600", "5 0.5 0.5 600.5 1 1 2147483647 0 0 0 0 0"),
        // reads before the tables, still inside the mapped pages
        ("oob_below_x_small", "5 -1.5 0.5 0.5 -3 1 1 0 0 0 0 0"),
        ("oob_below_x_20", "5 -20.5 0.5 0.5 -30 1 1 0 0 0 0 0"),
        ("oob_below_x_deep", "5 -5000.5 0.5 0.5 -9000 1 1 0 0 0 0 0"),
        // the first read that leaves the mapped pages: SIGSEGV
        ("segv_above_x_4031", "5 4031.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("segv_above_x_4032", "5 4032.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("segv_above_x_far", "5 100000.5 0.5 0.5 2147483647 1 1 0 0 0 0 0"),
        ("segv_below_x_20000", "5 -19000.5 0.5 0.5 -20000 1 1 0 0 0 0 0"),
        ("segv_below_x_22000", "5 -21000.5 0.5 0.5 -22000 1 1 0 0 0 0 0"),
        // INT_MIN % -1 overflows: SIGFPE
        ("sigfpe_x", "5 -1e30 0.5 0.5 -1 3 7 5 0 0 0 0"),
        ("sigfpe_y", "5 0.5 -1e30 0.5 3 -1 7 5 0 0 0 0"),
        ("sigfpe_z", "5 0.5 0.5 -1e30 3 7 -1 5 0 0 0 0"),
        ("sigfpe_nan_coord", "5 nan 0.5 0.5 -1 3 7 5 0 0 0 0"),
        ("sigfpe_inf_coord", "5 inf 0.5 0.5 -1 3 7 5 0 0 0 0"),
    ]);
}

/// Inputs that are not valid text: NUL bytes and non-UTF-8 bytes.
#[test]
fn non_text_input() {
    for input in [
        &b"\x00"[..],
        &b"0\x00 1 2"[..],
        &b"0 \x001 2"[..],
        &b"\xff\xfe"[..],
        &b"0 1.5\xff 2 3 0 0 0 0 0 0 0 0"[..],
        &b"0 1 2 3 0 0 0 0 0 0 0 0\x00garbage"[..],
        &b"\x1b[0m0 1 2 3"[..],
    ] {
        check(input);
    }
}

// ---------------------------------------------------------------------------
// A deterministic sweep over the parameter space
// ---------------------------------------------------------------------------

/// Small xorshift PRNG so the generated corpus is identical on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

/// Sweeps coordinates, wraps, seeds and fractal parameters. The wrap values
/// stay in `1..=256` (or 0) so that `which == 5` keeps its table indices in
/// bounds: reads outside the tables land on bytes that depend on the address
/// space layout, which ASLR randomises, so the C program itself is not
/// deterministic there and no test can pin it down. Those paths are covered by
/// the explicit `nonpow2_wrap` cases instead, which stay inside the region that
/// is stable across runs.
#[test]
fn generated_sweep() {
    const COORDS: [&str; 24] = [
        "0", "-0.0", "1", "-1", "0.5", "-0.5", "3.25", "-3.25", "7", "-7.5", "255.5", "-255.5",
        "256", "-256.25", "1000.125", "-1000.125", "65535.5", "-65536.5", "1e6", "-1e6", "1e30",
        "nan", "inf", "-inf",
    ];
    const WRAPS: [&str; 10] = ["0", "1", "2", "4", "8", "16", "32", "64", "128", "256"];
    const SEEDS: [&str; 8] = ["0", "1", "7", "31", "128", "255", "256", "-1"];
    const FLOATS: [&str; 12] = [
        "0", "-0.0", "0.5", "1", "2", "-2", "4.5", "0.25", "1e10", "-1e10", "nan", "inf",
    ];
    const OCTAVES: [&str; 10] = ["0", "1", "2", "3", "5", "6", "8", "-1", "-4", "12"];
    const SEPS: [&str; 5] = [" ", "\n", "\t", "  ", " \n "];

    let mut rng = Rng(0x243F_6A88_85A3_08D3);
    for _ in 0..260 {
        let which = rng.below(8) as i64 - 1;
        let fields = [
            which.to_string(),
            rng.pick(&COORDS).to_string(),
            rng.pick(&COORDS).to_string(),
            rng.pick(&COORDS).to_string(),
            rng.pick(&WRAPS).to_string(),
            rng.pick(&WRAPS).to_string(),
            rng.pick(&WRAPS).to_string(),
            rng.pick(&SEEDS).to_string(),
            rng.pick(&FLOATS).to_string(),
            rng.pick(&FLOATS).to_string(),
            rng.pick(&FLOATS).to_string(),
            rng.pick(&OCTAVES).to_string(),
        ];
        let n = if rng.below(6) == 0 {
            1 + rng.below(12) as usize
        } else {
            12
        };
        let input = fields[..n].join(rng.pick(&SEPS));
        check(input.as_bytes());
    }
}
