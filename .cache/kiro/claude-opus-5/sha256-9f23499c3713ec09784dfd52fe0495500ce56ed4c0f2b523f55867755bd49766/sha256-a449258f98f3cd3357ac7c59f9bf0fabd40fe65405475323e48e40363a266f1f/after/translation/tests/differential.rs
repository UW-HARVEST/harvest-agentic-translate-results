//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical stdin and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links the Rust crate as a library — both programs are driven
//! exactly the way a shell would drive them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, provided by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake once per test process and return the C binary path.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }
        std::fs::create_dir_all(&build).expect("create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake must be installed to run the differential tests");
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
            .expect("cmake --build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );
        assert!(exe.is_file(), "C binary missing after build: {}", exe.display());
        exe
    })
    .as_path()
}

/// Run `program` with `args`, feeding `stdin_data` to its standard input.
fn run(program: &Path, args: &[&str], stdin_data: &[u8]) -> Output {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("piped stdin");
        let data = stdin_data.to_vec();
        // Write on a helper thread so a large payload cannot deadlock against
        // the child's own output.
        let writer = std::thread::spawn(move || {
            let _ = sink.write_all(&data);
            let _ = sink.flush();
            // dropping `sink` closes the pipe, signalling EOF
        });
        writer.join().expect("stdin writer thread");
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Core assertion: identical stdout, stderr and exit status.
fn assert_same_with_args(label: &str, args: &[&str], stdin_data: &[u8]) {
    let c = run(c_bin(), args, stdin_data);
    let r = run(Path::new(RUST_BIN), args, stdin_data);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for case `{label}`\n  stdin: \"{}\"\n  C  stdout: \"{}\"\n  Rust stdout: \"{}\"",
        show(stdin_data),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for case `{label}`\n  stdin: \"{}\"\n  C  stderr: \"{}\"\n  Rust stderr: \"{}\"",
        show(stdin_data),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit code differs for case `{label}`\n  stdin: \"{}\"\n  C: {:?}  Rust: {:?}",
        show(stdin_data),
        c.status,
        r.status
    );
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "exit status success differs for case `{label}` (stdin \"{}\")",
        show(stdin_data)
    );
}

fn assert_same(label: &str, stdin_data: &[u8]) {
    assert_same_with_args(label, &[], stdin_data);
}

fn assert_all(cases: &[(&str, &[u8])]) {
    for (label, data) in cases {
        assert_same(label, data);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), &[], b"1 2");
    let r = run(Path::new(RUST_BIN), &[], b"1 2");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
    assert_eq!(c.stdout, r.stdout);
    // The C program prints `x | ~y` followed by puts("") -> a single newline.
    // For `1 2`: 1 | ~2 == 1 | -3 == -3.
    assert_eq!(c.stdout, b"-3\n".to_vec(), "unexpected reference output for `1 2`");
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C actually branches on.
// ---------------------------------------------------------------------------

/// Both `scanf` calls fail: `x` and `y` keep their initialisers (0, 0).
#[test]
fn no_input_at_all() {
    assert_all(&[
        ("empty", b""),
        ("single_space", b" "),
        ("spaces_only", b"        "),
        ("newlines_only", b"\n\n\n\n"),
        ("tabs_only", b"\t\t\t"),
        ("mixed_ws_only", b" \t\n\x0b\x0c\r "),
        ("cr_only", b"\r"),
    ]);
}

/// First `scanf` succeeds, second hits EOF: `y` stays 0.
#[test]
fn single_item_only() {
    assert_all(&[
        ("one_zero", b"0"),
        ("one_positive", b"5"),
        ("one_negative", b"-5"),
        ("one_with_newline", b"42\n"),
        ("one_with_trailing_ws", b"42   \n\t "),
        ("one_intmax", b"2147483647"),
        ("one_intmin", b"-2147483648"),
    ]);
}

/// The ordinary two-integer path, across every separator `isspace` accepts.
#[test]
fn two_items_separators() {
    assert_all(&[
        ("space", b"5 7"),
        ("many_spaces", b"5        7"),
        ("newline", b"5\n7"),
        ("newline_scanf_crosses_it", b"5\n\n\n7\n"),
        ("tab", b"5\t7"),
        ("crlf", b"5\r\n7\r\n"),
        ("vtab_formfeed", b"\x0b5\x0c7"),
        ("leading_ws", b"   \n\t  12   \n 34  "),
        ("no_trailing_newline", b"5 7"),
    ]);
}

/// Sign handling and leading zeros.
#[test]
fn signs_and_leading_zeros() {
    assert_all(&[
        ("both_negative", b"-5 -7"),
        ("mixed_signs", b"-5 7"),
        ("mixed_signs2", b"5 -7"),
        ("explicit_plus", b"+5 +7"),
        ("plus_and_minus", b"+5 -7"),
        ("negative_zero", b"-0 -0"),
        ("plus_zero", b"+0 +0"),
        ("leading_zeros", b"00000000000000000000005 0000007"),
        ("sign_then_space_then_digits", b"- 5"),
        ("zero_zero", b"0 0"),
    ]);
}

/// Exhaustive-ish sweep of `x | ~y` over interesting bit patterns.
#[test]
fn bitwise_patterns() {
    let vals: [i64; 17] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -8,
        255,
        -256,
        65535,
        -65536,
        1_431_655_765,  // 0x55555555
        -1_431_655_766, // 0xAAAAAAAA as i32
        2_147_483_647,  // INT_MAX
        -2_147_483_648, // INT_MIN
    ];
    for a in vals {
        for b in vals {
            let input = format!("{a} {b}");
            assert_same(&format!("bits_{a}_{b}"), input.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Phase B/C: integer overflow, truncation and signedness exactly as C does it.
// ---------------------------------------------------------------------------

/// Values outside `int` but inside `long`: glibc converts with `strtol`, then
/// the `%d` store narrows to `int`.
#[test]
fn out_of_int_range_truncates() {
    assert_all(&[
        ("x_int_overflow", b"2147483648 0"),
        ("x_int_overflow_y", b"2147483648 1"),
        ("x_int_underflow", b"-2147483649 0"),
        ("y_int_overflow", b"1234 2147483648"),
        ("y_int_underflow", b"1234 -2147483649"),
        ("2_32", b"5 4294967296"),
        ("2_32_plus", b"5 4294967301"),
        ("neg_2_32", b"5 -4294967296"),
        ("2_32_minus_1", b"4294967295 4294967295"),
        ("2_33", b"8589934592 8589934592"),
        ("mixed_wide", b"-8589934593 8589934591"),
    ]);
}

/// Values at and beyond the `long` limits: `strtol` saturates at LONG_MAX /
/// LONG_MIN, and the saturated value is then narrowed to `int`.
#[test]
fn long_range_saturation() {
    assert_all(&[
        ("long_max", b"5 9223372036854775807"),
        ("long_min", b"5 -9223372036854775808"),
        ("long_max_plus_1", b"5 9223372036854775808"),
        ("long_min_minus_1", b"5 -9223372036854775809"),
        ("long_max_both", b"9223372036854775807 9223372036854775807"),
        ("long_min_both", b"-9223372036854775808 -9223372036854775808"),
        ("twenty_nines", b"5 99999999999999999999"),
        ("twenty_nines_neg", b"5 -99999999999999999999"),
        ("twenty_nines_x", b"99999999999999999999 3"),
        ("twenty_nines_x_neg", b"-99999999999999999999 3"),
        ("forty_digits", b"1111111111111111111111111111111111111111 7"),
        ("forty_digits_neg", b"-1111111111111111111111111111111111111111 7"),
        ("huge_both", b"10000000000000000000000000 -10000000000000000000000000"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: matching-failure paths. `scanf` leaves the variable untouched, so
// the initialiser (0) survives; the return value is discarded by the C.
// ---------------------------------------------------------------------------

/// First conversion fails outright.
#[test]
fn first_conversion_fails() {
    assert_all(&[
        ("letters", b"abc"),
        ("letters_then_numbers", b"abc 5 7"),
        ("punctuation", b"#*"),
        ("comma_first", b",5"),
        ("dot_first", b".5"),
        ("minus_only", b"-"),
        ("plus_only", b"+"),
        ("double_minus", b"--5 7"),
        ("minus_plus", b"-+5 7"),
        ("plus_minus", b"+-5 7"),
        ("hex_prefix_x_only", b"x10 5"),
    ]);
}

/// Second conversion fails after the first succeeded.
#[test]
fn second_conversion_fails() {
    assert_all(&[
        ("num_then_letters", b"5 abc"),
        ("num_then_minus", b"5 -"),
        ("num_then_plus", b"5 +"),
        ("num_then_dot", b"5 ."),
        ("num_then_comma", b"5 ,"),
        ("num_then_double_minus", b"5 --7"),
        ("num_then_punct", b"5 #"),
    ]);
}

/// Partial matches: `%d` stops at the first non-digit and leaves it queued,
/// so the following `scanf` starts on that byte.
#[test]
fn partial_matches_and_queued_bytes() {
    assert_all(&[
        ("digits_then_letters", b"12abc 34"),
        ("digits_then_letters_no_space", b"12abc34"),
        ("hex_literal", b"0x10 5"),
        ("hex_literal_upper", b"0X10 5"),
        ("float", b"1.5 2.5"),
        ("float_no_int_part", b"5 .5"),
        ("comma_separated", b"1,2"),
        ("comma_separated_ws", b"1 , 2"),
        ("exponent", b"1e5 2"),
        ("exponent_upper", b"1E5 2"),
        ("digits_then_dash", b"1-2"),
        ("digits_then_plus", b"1+2"),
        ("underscore", b"1_000 2"),
    ]);
}

/// Only the first two conversions happen; the rest of the stream is ignored.
#[test]
fn extra_input_is_ignored() {
    assert_all(&[
        ("three_items", b"5 7 9"),
        ("four_items", b"5 7 9 11"),
        ("trailing_garbage", b"5 7 garbage here"),
        ("many_items", b"1 2 3 4 5 6 7 8 9 10\n"),
    ]);
}

/// Non-textual and non-ASCII bytes.
#[test]
fn binary_and_non_ascii_bytes() {
    assert_all(&[
        ("nul_only", b"\x00"),
        ("nul_then_digits", b"\x005 7"),
        ("digits_then_nul", b"5\x007"),
        ("high_bytes", &[0xff, 0xfe, 0x80]),
        ("utf8_accented", "é 5".as_bytes()),
        ("utf8_between", "5 é 7".as_bytes()),
        ("del_byte", b"\x7f 5"),
        ("nbsp_is_not_c_space", &[0xc2, 0xa0, b'5', b' ', b'7']),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: buffering boundaries. The Rust reader refills a fixed-size buffer,
// so tokens must survive being split across a refill.
// ---------------------------------------------------------------------------

#[test]
fn buffer_boundary_cases() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // A digit run that crosses the 4096-byte refill boundary.
    let mut v = vec![b'0'; 4095];
    v.extend_from_slice(b"7 9");
    cases.push(("digits_span_4096".to_string(), v));

    // Whitespace filling exactly one buffer, then the real tokens.
    let mut v = vec![b'\n'; 4096];
    v.extend_from_slice(b"-5 -9");
    cases.push(("ws_fills_4096".to_string(), v));

    // A number whose digits straddle the boundary.
    let mut v = vec![b' '; 4094];
    v.extend_from_slice(b"123456 654321");
    cases.push(("number_straddles_boundary".to_string(), v));

    // The sign lands on the last byte of the first buffer.
    let mut v = vec![b' '; 4095];
    v.extend_from_slice(b"-42 -42");
    cases.push(("sign_on_boundary".to_string(), v));

    // Very long digit runs (well past any buffer).
    let mut v = vec![b'9'; 10_000];
    v.push(b' ');
    v.extend(std::iter::repeat(b'8').take(10_000));
    cases.push(("ten_thousand_digits_each".to_string(), v));

    // Long run of zeros only: converts to 0, second scanf hits EOF.
    cases.push(("ten_thousand_zeros".to_string(), vec![b'0'; 10_000]));

    // Leading zeros then a significant tail, longer than a buffer.
    let mut v = vec![b'0'; 8192];
    v.extend_from_slice(b"12345 ");
    v.extend(std::iter::repeat(b'0').take(8192));
    v.extend_from_slice(b"6789");
    cases.push(("long_leading_zeros".to_string(), v));

    // Large stream where only the first two tokens matter.
    cases.push(("fifty_thousand_tokens".to_string(), b"7 ".repeat(50_000)));

    for (label, data) in &cases {
        assert_same(label, data);
    }
}

// ---------------------------------------------------------------------------
// Phase C: process-level behaviour.
// ---------------------------------------------------------------------------

/// `main()` takes no parameters, so argv must have no effect.
#[test]
fn argv_is_ignored() {
    assert_same_with_args("argv_one", &["foo"], b"3 4");
    assert_same_with_args("argv_many", &["-h", "--version", "1", "2"], b"3 4");
    assert_same_with_args("argv_empty_input", &["whatever"], b"");
}

/// Immediate EOF on stdin (pipe closed with nothing written).
#[test]
fn stdin_closed_immediately() {
    assert_same("stdin_eof", b"");
}

// ---------------------------------------------------------------------------
// Pseudo-random differential sweep, seeded for reproducibility.
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

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn randomized_integer_pairs() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for i in 0..300 {
        let a = rng.next() as i64;
        let b = rng.next() as i64;
        // Mix widths: sometimes narrow to i32, sometimes keep the full i64.
        let (a, b) = if i % 3 == 0 {
            (i64::from(a as i32), i64::from(b as i32))
        } else {
            (a, b)
        };
        let input = format!("{a} {b}");
        assert_same(&format!("rand_pair_{i}"), input.as_bytes());
    }
}

#[test]
fn randomized_garbage_streams() {
    const ALPHABET: &[u8] = b"0123456789 \t\n+-abcxeE.,\r\x0b\x0c\x00#*/";
    let mut rng = Rng(0x0bad_c0de_dead_beef);
    for i in 0..600 {
        let len = rng.below(14) as usize;
        let data: Vec<u8> = (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("rand_garbage_{i}"), &data);
    }
}

#[test]
fn randomized_long_digit_strings() {
    let mut rng = Rng(0xfeed_face_cafe_1357);
    for i in 0..120 {
        let len = 1 + rng.below(30) as usize;
        let mut data: Vec<u8> = Vec::new();
        if rng.below(3) == 0 {
            data.push(b'-');
        }
        for _ in 0..len {
            data.push(b'0' + rng.below(10) as u8);
        }
        data.push(b' ');
        if rng.below(3) == 0 {
            data.push(b'-');
        }
        let len2 = 1 + rng.below(30) as usize;
        for _ in 0..len2 {
            data.push(b'0' + rng.below(10) as u8);
        }
        assert_same(&format!("rand_long_digits_{i}"), &data);
    }
}
